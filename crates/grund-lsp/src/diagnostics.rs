// Diagnostic publication and source-range transport (§FS-lsp.1.1).

impl Server {
    fn publish_diagnostics(&mut self) -> Result<()> {
        let mut by_uri: BTreeMap<Url, Vec<Diagnostic>> = BTreeMap::new();
        for project in &self.projects {
            for finding in project.snapshot.report.errors.clone() {
                if let Some((uri, diagnostic)) =
                    self.diagnostic_for_finding(project, finding, DiagnosticSeverity::ERROR)
                {
                    by_uri.entry(uri).or_default().push(diagnostic);
                }
            }
            for finding in project.snapshot.report.warnings.clone() {
                if let Some((uri, diagnostic)) =
                    self.diagnostic_for_finding(project, finding, DiagnosticSeverity::WARNING)
                {
                    by_uri.entry(uri).or_default().push(diagnostic);
                }
            }
        }
        let next_diagnostic_uris: BTreeSet<Url> = by_uri.keys().cloned().collect();
        for uri in self.diagnostic_uris.difference(&next_diagnostic_uris) {
            self.connection
                .sender
                .send(Message::Notification(Notification::new(
                    "textDocument/publishDiagnostics".to_string(),
                    PublishDiagnosticsParams {
                        uri: uri.clone(),
                        diagnostics: Vec::new(),
                        version: None,
                    },
                )))?;
        }
        self.diagnostic_uris = next_diagnostic_uris;
        for (uri, diagnostics) in by_uri {
            self.connection
                .sender
                .send(Message::Notification(Notification::new(
                    "textDocument/publishDiagnostics".to_string(),
                    PublishDiagnosticsParams {
                        uri,
                        diagnostics,
                        version: None,
                    },
                )))?;
        }
        Ok(())
    }

    /// Turn one finding into the diagnostic the editor shows, or `None` when
    /// this project is not the one that answers for the file.
    ///
    /// Why exactly one project may state a verdict: overlapping editor folders —
    /// an enclosing workspace and a member opened as its own folder, say — both
    /// scan the shared file, and without an owner the editor would show two
    /// merged diagnostic sets, with the two projects' differing views of the
    /// same citation side by side.
    fn diagnostic_for_finding(
        &self,
        project: &ProjectSnapshot,
        finding: Finding,
        severity: DiagnosticSeverity,
    ) -> Option<(Url, Diagnostic)> {
        let snapshot = &project.snapshot;
        let path = absolute_finding_path(snapshot, &finding)?;
        // Overlapping editor folders mean two projects can scan one file; only
        // the project that answers requests for it states its verdict
        // (§FS-lsp.2.2).
        let owner = self.project_for_diagnostic_path(&path)?;
        if owner.root != project.root {
            return None;
        }
        let uri = path_uri(&path)?;
        let line = finding.line.unwrap_or(1).saturating_sub(1) as u32;
        let range = self
            .range_for_finding(snapshot, &path, &finding)
            .unwrap_or(Range {
                start: Position { line, character: 0 },
                end: Position { line, character: 1 },
            });
        Some((
            uri,
            Diagnostic {
                range,
                severity: Some(severity),
                code: Some(lsp_types::NumberOrString::String(finding.code.to_string())),
                source: Some("grund".to_string()),
                message: finding.message,
                ..Diagnostic::default()
            },
        ))
    }

    /// The range a finding's diagnostic is anchored on.
    ///
    /// Why a line-anchored finding does not reuse a citation's range: in VSCode
    /// a diagnostic hover includes every diagnostic whose range overlaps the
    /// cursor, so mapping an ungrounded-file error onto a dangling citation
    /// would make the citation hover show two messages even though only one
    /// diagnostic belongs to that token.
    fn range_for_finding(
        &self,
        snapshot: &LspSnapshot,
        path: &Path,
        finding: &Finding,
    ) -> Option<Range> {
        let line = finding.line?;
        // When the finding carries the offending citation's column, anchor on
        // that exact token rather than the first citation on the line — a single
        // comment can carry several citations (§FS-lsp.1.1).
        if let Some(column) = finding.column
            && let Some(citation) = snapshot.citations.iter().find(|citation| {
                same_path(&citation.path, path)
                    && citation.line == line
                    && citation.column == column
            })
        {
            return Some(citation_range(citation, self));
        }
        // Rejected section headings retain their exact title range solely for
        // diagnostics; they remain absent from every navigation collection
        // (§FS-check.3.23, §FS-lsp.1.1).
        if let Some(range) = snapshot.finding_ranges.iter().find(|range| {
            range.code == finding.code && same_path(&range.path, path) && range.line == line
        }) {
            return Some(token_range(
                self,
                &range.path,
                range.line,
                range.column,
                &range.text,
            ));
        }
        // Line-anchored diagnostics must not borrow the first citation on their
        // line (§FS-lsp.1.1).
        snapshot
            .declarations
            .iter()
            .find(|decl| same_path(&decl.path, path) && decl.line == line)
            .map(|decl| declaration_range(decl, self))
            .or_else(|| {
                snapshot
                    .sections
                    .iter()
                    .find(|section| same_path(&section.path, path) && section.line == line)
                    .map(|section| declaration_range(section, self))
            })
            .or_else(|| {
                snapshot
                    .stubs
                    .iter()
                    .find(|stub| same_path(&stub.path, path) && stub.line == line)
                    .map(|stub| stub_range(stub, self))
            })
    }
}

fn absolute_finding_path(snapshot: &LspSnapshot, finding: &Finding) -> Option<PathBuf> {
    let path = Path::new(finding.path.as_deref()?);
    Some(if path.is_absolute() {
        path.to_path_buf()
    } else {
        snapshot.root.join(path)
    })
}
