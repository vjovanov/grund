/// Options for programmatic declaration reads through [`show`]
/// (§FS-distribution.3.0, §FS-distribution.3.1).
#[derive(Clone)]
pub struct ShowOpts {
    pub path: PathBuf,
    pub section: Option<String>,
    pub mode: ShowMode,
    pub format: ShowFormat,
}

impl Default for ShowOpts {
    fn default() -> Self {
        Self {
            path: PathBuf::from("."),
            section: None,
            mode: ShowMode::Lead,
            format: ShowFormat::Text,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ShowMode {
    Brief,
    Lead,
    Toc,
    Full,
}

impl ShowMode {
    fn render_mode(self) -> ShowRenderMode {
        match self {
            ShowMode::Brief => ShowRenderMode::Brief,
            ShowMode::Lead => ShowRenderMode::Default,
            ShowMode::Toc => ShowRenderMode::Toc,
            ShowMode::Full => ShowRenderMode::Full,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ShowFormat {
    Text,
    Markdown,
    Json,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FindingSite {
    pub path: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finding {
    pub severity: &'static str,
    pub code: &'static str,
    pub path: Option<String>,
    pub line: Option<usize>,
    /// 1-based start column of the offending citation when the finding concerns
    /// a specific token, so an LSP can anchor on that token rather than the first
    /// citation on the line (§FS-lsp.1.1). `None` for line-anchored findings.
    pub column: Option<usize>,
    pub message: String,
    pub sites: Vec<FindingSite>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Report {
    pub errors: Vec<Finding>,
    pub warnings: Vec<Finding>,
    /// The non-severity advisory channel (§FS-check.2.3): citation-direction
    /// `should` / `should-not` findings, populated only when
    /// [`CheckOpts::include_suggestions`] is set. Never affects the exit code.
    pub suggestions: Vec<Finding>,
}

#[derive(Clone)]
pub struct CheckOpts {
    pub path: PathBuf,
    pub path_provided: bool,
    pub require_grounding: bool,
    /// Surface the citation-direction suggestions channel (§FS-check.2.3) —
    /// the `grund check --suggestions` flag at the library level.
    pub include_suggestions: bool,
    /// Walk the whole config root, past `[scan] include`, and add the
    /// out-of-scope reference tier (§FS-check.1.3, §FS-check.3.14) — the
    /// `grund check --full` flag at the library level. Purely additive: the
    /// findings inside the configured scope are unchanged by it.
    pub full: bool,
}

impl Default for CheckOpts {
    fn default() -> Self {
        Self {
            path: PathBuf::from("."),
            path_provided: false,
            require_grounding: false,
            include_suggestions: false,
            full: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckOutput {
    pub output_format: String,
    pub report: Report,
    pub had_scan_errors: bool,
}

/// Scan one project tree and return the raw scanner findings. This is the
/// embedding surface later frontends share instead of re-reading files.
pub fn scan(path: &Path) -> Result<Findings> {
    let config = resolve_workspace_config(path)?;
    scan_tree_strict(&config, Some(path), true)
}

/// Programmatic `check`: load config, scan, and return structured findings
/// without CLI argument parsing, stdout/stderr rendering, or exit-code mapping
/// (§FS-distribution.3.1, §RM-core-cli-split.3).
pub fn check(path: &Path) -> Result<Report> {
    Ok(check_with_opts(CheckOpts {
        path: path.to_path_buf(),
        path_provided: true,
        require_grounding: false,
        include_suggestions: false,
        full: false,
    })?
    .report)
}

/// Programmatic `check` with the same scope and grounding options as the CLI,
/// returning data instead of printing a report or mapping a process exit.
pub fn check_with_opts(opts: CheckOpts) -> Result<CheckOutput> {
    let run = run_check(&opts.path, opts.path_provided, opts.require_grounding, opts.full)?;
    Ok(CheckOutput {
        output_format: run.config.output_format.clone(),
        report: public_report(&run.config, run.report, opts.include_suggestions),
        had_scan_errors: run.had_scan_errors,
    })
}

fn public_report(config: &Config, report: CheckReport, include_suggestions: bool) -> Report {
    Report {
        errors: report
            .errors
            .into_iter()
            .map(|diagnostic| public_finding(config, "error", diagnostic))
            .collect(),
        warnings: report
            .warnings
            .into_iter()
            .map(|diagnostic| public_finding(config, "warning", diagnostic))
            .collect(),
        // §FS-check.2.3: suggestions are surfaced only on demand. The public
        // severity tag stays `"suggestion"` so a consumer can tell them apart.
        suggestions: if include_suggestions {
            report
                .suggestions
                .into_iter()
                .map(|diagnostic| public_finding(config, "suggestion", diagnostic))
                .collect()
        } else {
            Vec::new()
        },
    }
}

fn public_finding(config: &Config, severity: &'static str, diagnostic: Diagnostic) -> Finding {
    Finding {
        severity,
        code: diagnostic.code,
        path: diagnostic.path.map(|path| public_path(config, &path)),
        line: diagnostic.line,
        column: diagnostic.column,
        message: diagnostic.message,
        sites: diagnostic
            .sites
            .into_iter()
            .map(|site| FindingSite {
                path: public_path(config, &site.path),
                line: site.line,
            })
            .collect(),
    }
}

fn public_path(config: &Config, path: &Path) -> String {
    display_path(config, path)
}

/// Programmatic declaration read. This mirrors `grund show` resolution but
/// returns the structured body instead of printing it.
pub fn show(id_arg: &str, opts: ShowOpts) -> Result<ShowOutput> {
    show_with_scope(id_arg, opts, true)
}

pub fn show_with_overlays(
    id_arg: &str,
    opts: ShowOpts,
    open_documents: BTreeMap<PathBuf, String>,
) -> Result<ShowOutput> {
    show_with_scope_and_overlays(id_arg, opts, true, &normalized_overlays(open_documents))
}

#[doc(hidden)]
pub fn show_with_scope(id_arg: &str, opts: ShowOpts, path_provided: bool) -> Result<ShowOutput> {
    show_with_scope_and_overlays(id_arg, opts, path_provided, &TextOverlays::new())
}

fn show_with_scope_and_overlays(
    id_arg: &str,
    opts: ShowOpts,
    path_provided: bool,
    overlays: &TextOverlays,
) -> Result<ShowOutput> {
    let context = load_workspace_context_with_overlays(&opts.path, path_provided, overlays, false)?;
    let (alias, raw_id) = split_qualified_id_arg(id_arg)?;
    let project = match alias.as_deref() {
        Some(name) => context
            .project_by_alias(name)
            .ok_or_else(|| {
                if !context.workspace_loaded {
                    anyhow!(
                        "unknown project alias `{name}`\nnote: workspace aliases are defined in the root grund.toml under [workspace]"
                    )
                } else {
                    anyhow!(
                        "unknown project alias `{name}`\nknown aliases: {}",
                        context.aliases().join(", ")
                    )
                }
            })?,
        None => context.current_project().ok_or_else(|| {
            let known = context.aliases().join(", ");
            if known.is_empty() {
                anyhow!("unqualified ID requires a project alias when include_root = false")
            } else {
                anyhow!(
                    "unqualified ID requires a project alias when include_root = false\nknown aliases: {known}"
                )
            }
        })?,
    };
    if let Some((file, message)) = project.scan_errors.first() {
        return Err(anyhow!(
            "{}: {}",
            display_path(&project.config, file),
            message
        ));
    }
    let config = &project.config;
    let (id, inline_section) =
        resolve_id_arg(raw_id, config, &project.findings).map_err(|err| anyhow!("{err}"))?;
    if opts.section.is_some() && inline_section.is_some() {
        return Err(anyhow!("--section cannot be combined with an inline section"));
    }
    let section = opts.section.or(inline_section);
    let mut output = show_declaration_with_overlays(
        config,
        context.render_config(),
        &project.findings,
        &id,
        section.as_deref(),
        opts.mode.render_mode(),
        opts.format == ShowFormat::Markdown,
        overlays,
    )?;
    if opts.format != ShowFormat::Markdown {
        output.body = flatten_cross_ref_links(&output.body, config);
    }
    if opts.format == ShowFormat::Json {
        let json = render_show_output_json(
            config,
            context.render_config(),
            &id,
            section.as_deref(),
            opts.mode.render_mode(),
            &output,
        );
        output.json = Some(json);
    }
    Ok(output)
}

#[derive(Clone)]
pub struct CompleteIdsOpts {
    pub path: PathBuf,
    pub path_provided: bool,
    pub prefix: String,
    pub sections: bool,
}

impl Default for CompleteIdsOpts {
    fn default() -> Self {
        Self {
            path: PathBuf::from("."),
            path_provided: false,
            prefix: String::new(),
            sections: false,
        }
    }
}

/// Dynamic ID completion candidates for shell frontends. Config or scan errors
/// are returned to the caller so command frontends can decide whether to hide
/// them during tab completion.
pub fn complete_ids(opts: CompleteIdsOpts) -> Result<Vec<String>> {
    let context = load_workspace_context(&opts.path, opts.path_provided)?;
    let current_config = context
        .current_project()
        .map(|project| &project.config)
        .unwrap_or_else(|| context.render_config());
    let mut candidates = BTreeSet::new();
    // §FS-workspace.8.4: an alias path may itself carry slashes
    // (§FS-workspace.6.1), so the split is at the *last* `/` — the left names a
    // project, the right is its ID-prefix. A prefix that lands mid-path
    // completes both: the deeper alias paths still to be typed, and the IDs of
    // the project already named.
    if let Some((alias_prefix, id_prefix)) = opts.prefix.rsplit_once('/') {
        if !context.workspace_loaded {
            return Ok(Vec::new());
        }
        for alias in context.aliases() {
            let continuation = format!("{alias}/");
            // The exact prefix is withheld: re-offering what is already on the
            // line stalls the shell instead of advancing it.
            if continuation != opts.prefix && continuation.starts_with(&opts.prefix) {
                candidates.insert(continuation);
            }
        }
        if let Some(project) = context.project_by_alias(alias_prefix) {
            let complete_sections =
                opts.sections || id_prefix.contains(&project.config.section_separator);
            add_complete_id_candidates(
                &mut candidates,
                Some(alias_prefix),
                &project.config,
                &project.findings,
                complete_sections,
            );
        }
    } else {
        let complete_sections = opts.sections || opts.prefix.contains(&current_config.section_separator);
        if let Some(current_project) = context.current_project() {
            add_complete_id_candidates(
                &mut candidates,
                None,
                current_config,
                &current_project.findings,
                complete_sections,
            );
        }
        if context.workspace_loaded {
            for alias in context.aliases() {
                candidates.insert(format!("{alias}/"));
            }
        }
    }
    Ok(candidates
        .into_iter()
        .filter(|candidate| candidate.starts_with(&opts.prefix))
        .collect())
}

fn add_complete_id_candidates(
    candidates: &mut BTreeSet<String>,
    alias: Option<&str>,
    config: &Config,
    findings: &Findings,
    include_sections: bool,
) {
    let qualifier = alias.map(|alias| format!("{alias}/")).unwrap_or_default();
    for (id, decls) in &findings.declarations {
        let rendered = render_id(config, id);
        if include_sections {
            for decl in decls {
                for section in decl.sections.keys() {
                    candidates.insert(format!(
                        "{}{}{}{}",
                        qualifier, rendered, config.section_separator, section
                    ));
                }
            }
        } else {
            candidates.insert(format!("{qualifier}{rendered}"));
        }
    }
}

#[derive(Clone)]
pub struct IdOpts {
    pub path: PathBuf,
    pub path_provided: bool,
    pub width: usize,
}

impl Default for IdOpts {
    fn default() -> Self {
        Self {
            path: PathBuf::from("."),
            path_provided: false,
            width: 3,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdProposal {
    pub id: String,
    pub kind: String,
    pub number: Option<u32>,
    pub slug: String,
    pub folder: Option<String>,
    pub file: Option<String>,
    pub e2e_case_dir: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IdProposalOutcome {
    Proposed(IdProposal),
    UnknownKind { kind: String, known: Vec<String> },
    Rejected { message: String },
}

/// Programmatic `id`: compute the next conflict-free declaration ID without
/// parsing CLI flags or printing the text/JSON report (§RM-core-cli-split).
pub fn propose_id(kind: &str, title: &str, opts: IdOpts) -> Result<IdProposalOutcome> {
    let config = resolve_workspace_config(&opts.path)?;
    let Some(kind_config) = config
        .kinds
        .iter()
        .find(|candidate| candidate.prefix == kind)
    else {
        return Ok(IdProposalOutcome::UnknownKind {
            kind: kind.to_string(),
            known: kind_prefixes(&config.kinds),
        });
    };
    let slug = slugify_title(title, &config.slug_pattern);
    if slug.is_empty() {
        return Ok(IdProposalOutcome::Rejected {
            message: format!("title produces empty slug after normalization: \"{title}\""),
        });
    }
    let findings = scan_tree_strict(&config, Some(&opts.path), opts.path_provided)?;
    let uses_number = config.id_format.contains("{number}");
    let number = if uses_number {
        let max = findings
            .declarations
            .keys()
            .filter(|id| id.kind == kind)
            .filter_map(|id| id.num)
            .max()
            .unwrap_or(0);
        Some(max + 1)
    } else {
        None
    };
    let id = Id {
        kind: kind.to_string(),
        num: number,
        slug: if config.id_format.contains("{slug}") {
            Some(slug.clone())
        } else {
            None
        },
    };
    let rendered = format_id(&id, &config, opts.width);
    if let Some(decls) = findings.declarations.get(&id)
        && let Some(decl) = decls.first()
    {
        return Ok(IdProposalOutcome::Rejected {
            message: format!(
                "proposed ID `{}` already declared at {}:{}",
                rendered,
                display_path(&config, &decl.file),
                decl.line
            ),
        });
    }
    Ok(IdProposalOutcome::Proposed(IdProposal {
        e2e_case_dir: (kind == "E2E").then(|| e2e_case_dir_name(&config, &rendered)),
        id: rendered,
        kind: kind.to_string(),
        number,
        slug,
        folder: kind_config.folder.clone(),
        file: kind_config.file.clone(),
    }))
}

/// Load the effective config for a path without rendering it as CLI TOML.
pub fn effective_config(path: &Path) -> Result<Config> {
    load_config(path)
}

/// Validate config discovery/parsing for a path without printing CLI output
/// (§FS-config.4.1). Discovery *is* the validation — a config that loads is a
/// config that is valid — so this is [`effective_config`] under the name of the
/// question `grund config validate` asks, and delegates rather than repeating
/// it. Returns the loaded config so a caller can also report the non-fatal
/// findings [`config_warnings`] carries.
pub fn validate_config(path: &Path) -> Result<Config> {
    effective_config(path)
}

/// The CLI-level `warning:` texts a loaded config carries — today only the
/// redundant discovery pair (§FS-config.1.1, §FS-check.4.3). Message text only,
/// so `grund config validate` and `grund config show` print the same sentence
/// `grund check` does without depending on the checker's report type.
pub fn config_warnings(config: &Config) -> Vec<String> {
    redundant_config_warning(config)
        .map(|diagnostic| diagnostic.message)
        .into_iter()
        .collect()
}

/// Reference marker and typing trigger resolved for one path. §FS-lsp.1.4
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceStyle {
    pub marker: String,
    pub trigger: String,
}

/// Resolve the marker/trigger pair for a specific document. Workspace member
/// files use the member config, matching `grund fmt` and `grund check`
/// (§FS-lsp.1.4, §FS-workspace.5).
pub fn reference_style(path: &Path) -> Result<ReferenceStyle> {
    let config = resolve_workspace_config(path)?;
    Ok(ReferenceStyle {
        marker: config.marker,
        trigger: config.trigger,
    })
}

#[derive(Clone)]
pub struct LspSnapshotOpts {
    pub path: PathBuf,
    pub path_provided: bool,
    pub open_documents: BTreeMap<PathBuf, String>,
}

impl Default for LspSnapshotOpts {
    fn default() -> Self {
        Self {
            path: PathBuf::from("."),
            path_provided: false,
            open_documents: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LspSnapshot {
    pub root: PathBuf,
    pub marker: String,
    pub trigger: String,
    pub workspace: bool,
    pub report: Report,
    pub declarations: Vec<LspDeclaration>,
    /// Numbered section headings (`<ID>.<section>`) inside declaration bodies,
    /// each a declaration-side title editors can navigate to its section
    /// citations (§FS-lsp.1.3.1). Kept separate from `declarations` so the
    /// whole-ID home set stays the bare-ID declarations.
    pub sections: Vec<LspDeclaration>,
    pub stubs: Vec<LspStub>,
    pub citations: Vec<LspCitation>,
    pub scan_errors: Vec<ApiScanError>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LspDeclaration {
    pub project: Option<String>,
    pub path: PathBuf,
    pub display_path: String,
    pub line: usize,
    pub column: usize,
    pub text: String,
    pub query_id: String,
    pub section_separator: String,
}

/// LSP token for an inline-spec stub title whose definition follows to the
/// source doc-comment declaration. §FS-lsp.1.3
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LspStub {
    pub project: Option<String>,
    pub path: PathBuf,
    pub display_path: String,
    pub line: usize,
    pub column: usize,
    pub text: String,
    pub query_id: String,
    pub section_separator: String,
    pub target_path: PathBuf,
    pub target_line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LspCitation {
    pub project: Option<String>,
    pub path: PathBuf,
    pub display_path: String,
    pub line: usize,
    pub column: usize,
    pub text: String,
    pub query_id: String,
    pub declaration_query_id: String,
    pub section_separator: String,
    pub target_path: Option<PathBuf>,
    pub target_line: Option<usize>,
}

/// Programmatic snapshot for `grund-lsp`: all scanner-derived declaration and
/// citation ranges plus their resolved navigation targets. This keeps the LSP
/// transport from re-implementing the reference grammar (§AR-lsp.1).
pub fn lsp_snapshot(opts: LspSnapshotOpts) -> Result<LspSnapshot> {
    let overlays = normalized_overlays(opts.open_documents);
    // §FS-lsp.1.1: classify citing sides so the citation-direction checks
    // (`missing-citation` / `forbidden-citation`) run and surface as editor
    // diagnostics, the same errors `grund check` reports.
    let context =
        load_workspace_context_with_overlays(&opts.path, opts.path_provided, &overlays, true)?;
    let render_config = context.render_config().clone();
    let report = public_report(&render_config, check_workspace_context(&context, false), false);
    let mut declarations = Vec::new();
    let mut sections = Vec::new();
    let mut stubs = Vec::new();
    let mut citations = Vec::new();
    let mut scan_errors = Vec::new();

    for project in &context.projects {
        scan_errors.extend(
            project
                .scan_errors
                .iter()
                .map(|(file, message)| api_scan_error(&project.config, file, message)),
        );
        for (id, decls) in &project.findings.declarations {
            let rendered = render_id(&project.config, id);
            let query_id = lsp_query_id(&context, project, &rendered, None);
            let mut homes: Vec<&Declaration> = decls
                .iter()
                .filter(|decl| !is_stub_for_inline_decl(&project.config.root, decl, decls))
                .collect();
            homes.sort_by(|a, b| {
                (sort_path_key(&a.file), a.line).cmp(&(sort_path_key(&b.file), b.line))
            });
            for home in homes {
                let display = if context.workspace_loaded {
                    display_path(context.render_config(), &home.file)
                } else {
                    display_path(&project.config, &home.file)
                };
                let (column, text) = declaration_range_parts(home, &rendered, &overlays);
                declarations.push(LspDeclaration {
                    project: context.workspace_loaded.then(|| project.alias.clone()),
                    path: absolutize_path(&home.file),
                    display_path: display.clone(),
                    line: home.line,
                    column,
                    text,
                    query_id: query_id.clone(),
                    section_separator: project.config.section_separator.clone(),
                });
                // Each numbered section heading is its own declaration-side
                // title: editors navigate `<ID>.<section>` to that section's
                // citations, the same way the whole-ID title does (§FS-lsp.1.3.1).
                for (section, info) in &home.sections {
                    let (column, text) =
                        heading_span_parts(&home.file, info.line, section, &overlays);
                    sections.push(LspDeclaration {
                        project: context.workspace_loaded.then(|| project.alias.clone()),
                        path: absolutize_path(&home.file),
                        display_path: display.clone(),
                        line: info.line,
                        column,
                        text,
                        query_id: lsp_query_id(&context, project, &rendered, Some(section)),
                        section_separator: project.config.section_separator.clone(),
                    });
                }
            }
            for stub in decls
                .iter()
                .filter(|decl| is_stub_for_inline_decl(&project.config.root, decl, decls))
            {
                let target = lsp_target_for_stub(project, stub, decls);
                if let Some((target_path, target_line)) = target {
                    let (column, text) = declaration_range_parts(stub, &rendered, &overlays);
                    stubs.push(LspStub {
                        project: context.workspace_loaded.then(|| project.alias.clone()),
                        path: absolutize_path(&stub.file),
                        display_path: if context.workspace_loaded {
                            display_path(context.render_config(), &stub.file)
                        } else {
                            display_path(&project.config, &stub.file)
                        },
                        line: stub.line,
                        column,
                        text,
                        query_id: query_id.clone(),
                        section_separator: project.config.section_separator.clone(),
                        target_path: absolutize_path(&target_path),
                        target_line,
                    });
                }
            }
        }
        for citation in &project.findings.citations {
            let target_project = match citation.namespace.as_deref() {
                Some(alias) => context.project_by_alias(alias),
                None => Some(project),
            };
            let rendered_id = target_project
                .map(|target| render_id(&target.config, &citation.id))
                .unwrap_or_else(|| {
                    citation
                        .text
                        .trim_start_matches(&render_config.marker)
                        .to_string()
                });
            let query_id = target_project
                .map(|target| {
                    lsp_query_id(
                        &context,
                        target,
                        &rendered_id,
                        citation.section.as_deref(),
                    )
                })
                .unwrap_or_else(|| rendered_id.clone());
            let declaration_query_id = target_project
                .map(|target| lsp_query_id(&context, target, &rendered_id, None))
                .unwrap_or_else(|| rendered_id.clone());
            let section_separator = target_project
                .map(|target| target.config.section_separator.clone())
                .unwrap_or_else(|| render_config.section_separator.clone());
            let target = target_project.and_then(|target| lsp_target_for_citation(target, citation));
            citations.push(LspCitation {
                project: context.workspace_loaded.then(|| project.alias.clone()),
                path: absolutize_path(&citation.file),
                display_path: if context.workspace_loaded {
                    display_path(context.render_config(), &citation.file)
                } else {
                    display_path(&project.config, &citation.file)
                },
                line: citation.line,
                column: citation.column,
                text: citation.text.clone(),
                query_id,
                declaration_query_id,
                section_separator,
                target_path: target.as_ref().map(|(path, _)| absolutize_path(path)),
                target_line: target.map(|(_, line)| line),
            });
        }
    }

    let declaration_sort = |a: &LspDeclaration, b: &LspDeclaration| {
        (sort_path_key(&a.path), a.line, a.column, &a.text).cmp(&(
            sort_path_key(&b.path),
            b.line,
            b.column,
            &b.text,
        ))
    };
    declarations.sort_by(declaration_sort);
    sections.sort_by(declaration_sort);
    stubs.sort_by(|a, b| {
        (sort_path_key(&a.path), a.line, a.column, &a.text).cmp(&(
            sort_path_key(&b.path),
            b.line,
            b.column,
            &b.text,
        ))
    });
    citations.sort_by(|a, b| {
        (sort_path_key(&a.path), a.line, a.column, &a.text).cmp(&(
            sort_path_key(&b.path),
            b.line,
            b.column,
            &b.text,
        ))
    });

    Ok(LspSnapshot {
        root: absolutize_path(&render_config.root),
        marker: render_config.marker,
        trigger: render_config.trigger,
        workspace: context.workspace_loaded,
        report,
        declarations,
        sections,
        stubs,
        citations,
        scan_errors,
    })
}

fn normalized_overlays(overlays: BTreeMap<PathBuf, String>) -> TextOverlays {
    overlays
        .into_iter()
        .map(|(path, text)| (absolutize_path(&path), text))
        .collect()
}

fn check_workspace_context(context: &WorkspaceContext, force_require_grounding: bool) -> CheckReport {
    let workspace = context
        .projects
        .iter()
        .map(|project| {
            (
                project.alias.clone(),
                WorkspaceCheckTarget {
                    findings: &project.findings,
                    config: &project.config,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut report = CheckReport::default();
    for project in &context.projects {
        let mut config = project.config.clone();
        if force_require_grounding {
            config.require_grounding = true;
        }
        let mut project_report = if context.workspace_loaded {
            check_with_workspace(
                &project.findings,
                &config,
                // §FS-workspace.8.1: paths inside a message are spelled from the
                // render root, like the anchors beside them.
                context.render_config(),
                Some(&project.alias),
                &workspace,
            )
        } else {
            check_findings(&project.findings, &config)
        };
        let project_has_findings =
            !project_report.errors.is_empty() || !project_report.warnings.is_empty();
        report.errors.append(&mut project_report.errors);
        report.warnings.append(&mut project_report.warnings);
        append_lsp_scan_errors(&mut report, project.scan_errors.iter().cloned());
        // §FS-lsp.4: the same decision `grund check` makes, from the same
        // function — an editor and a terminal over one tree report one set of
        // diagnostics (§FS-check.2.2, §FS-check.4.5).
        report.warnings.extend(scan_scope_caution(
            &config,
            &project.findings,
            &config.root,
            true,
            project.scan_errors.is_empty() && !project_has_findings,
        ));
    }
    sort_diagnostics(&mut report.errors);
    sort_diagnostics(&mut report.warnings);
    report
}

fn append_lsp_scan_errors(
    report: &mut CheckReport,
    scan_errors: impl IntoIterator<Item = (PathBuf, String)>,
) {
    for (file, message) in scan_errors {
        report.errors.push(Diagnostic {
            code: "io",
            path: Some(file),
            line: None,
            column: None,
            message,
            sites: Vec::new(),
        });
    }
}

fn lsp_query_id(
    context: &WorkspaceContext,
    project: &WorkspaceProject,
    rendered_id: &str,
    section: Option<&str>,
) -> String {
    let mut query = if context.workspace_loaded {
        format!("{}/{}", project.alias, rendered_id)
    } else {
        rendered_id.to_string()
    };
    if let Some(section) = section {
        query.push_str(&project.config.section_separator);
        query.push_str(section);
    }
    query
}

fn lsp_target_for_citation(
    target_project: &WorkspaceProject,
    citation: &Citation,
) -> Option<(PathBuf, usize)> {
    let decls = target_project.findings.declarations.get(&citation.id)?;
    if let Some(section) = citation.section.as_deref()
        && let Some(decl) = decls
            .iter()
            .find(|decl| decl.sections.contains_key(section))
        && let Some(info) = decl.sections.get(section)
    {
        return Some((decl.file.clone(), info.line));
    }
    let mut homes: Vec<&Declaration> = decls
        .iter()
        .filter(|decl| !is_stub_for_inline_decl(&target_project.config.root, decl, decls))
        .collect();
    homes.sort_by(|a, b| {
        (sort_path_key(&a.file), a.line).cmp(&(sort_path_key(&b.file), b.line))
    });
    let home = homes.first().copied().or_else(|| decls.first())?;
    Some((home.file.clone(), home.line))
}

fn lsp_target_for_stub(
    project: &WorkspaceProject,
    stub: &Declaration,
    decls: &[Declaration],
) -> Option<(PathBuf, usize)> {
    let target = stub.defined_in.as_ref()?;
    let resolved = resolve_stub_target(&project.config.root, &stub.file, target);
    let inline = decls
        .iter()
        .find(|decl| paths_same_location(&decl.file, &resolved) && decl.file != stub.file)?;
    Some((inline.file.clone(), inline.line))
}

fn declaration_range_parts(
    decl: &Declaration,
    rendered_id: &str,
    overlays: &TextOverlays,
) -> (usize, String) {
    heading_span_parts(&decl.file, decl.line, rendered_id, overlays)
}

/// The 1-based start column and title text of a heading-line token: the span
/// from `needle` (the rendered ID for a declaration, the section number for a
/// section heading) to the end of the trimmed line. Falls back to column 1 and
/// the bare `needle` when the line cannot be read.
fn heading_span_parts(
    file: &Path,
    line: usize,
    needle: &str,
    overlays: &TextOverlays,
) -> (usize, String) {
    overlay_text(overlays, file)
        .map(str::to_string)
        .or_else(|| fs::read_to_string(file).ok())
        .and_then(|text| text.lines().nth(line.saturating_sub(1)).map(str::to_string))
        .and_then(|line| {
            let start = line.find(needle)?;
            let end = line.trim_end().len();
            let text = line
                .get(start..end)
                .filter(|text| !text.is_empty())
                .unwrap_or(needle)
                .to_string();
            Some((start + 1, text))
        })
        .unwrap_or_else(|| (1, needle.to_string()))
}

fn absolutize_path(path: &Path) -> PathBuf {
    canonicalize_existing_prefix(path)
}

/// Canonicalize `path` to the same normalized form `LspSnapshot` paths carry,
/// so an LSP client's request URI matches the snapshot's declaration, stub,
/// and citation paths (§AR-lsp.5). Existing files resolve through
/// `fs::canonicalize`; a not-yet-saved overlay file resolves its existing
/// prefix and appends the missing tail — the same absolutization the snapshot
/// applies — so `grund-lsp` does not need a second, drift-prone copy of this
/// logic.
pub fn canonical_snapshot_path(path: &Path) -> PathBuf {
    absolutize_path(path)
}

#[derive(Clone)]
pub struct CoverOpts {
    pub path: PathBuf,
    pub path_provided: bool,
}

impl Default for CoverOpts {
    fn default() -> Self {
        Self {
            path: PathBuf::from("."),
            path_provided: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoverCitation {
    /// The alias of the project whose tree contains this citation site — the
    /// *citing* project, as in `refs` (§FS-workspace.8.2). `None` outside
    /// workspace mode, where the JSON field is omitted entirely so a
    /// single-project repo's bytes are unchanged (§DF-cover-workspace-scope.2.3).
    pub project: Option<String>,
    pub path: String,
    pub line: usize,
    pub column: usize,
    pub id: String,
    pub section: Option<String>,
    pub marker: bool,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoverEntry {
    /// The alias of the project this scanned file belongs to. A file belongs to
    /// exactly one project by the boundary rule (§FS-workspace.6), so this is
    /// unambiguous. `None` outside workspace mode (§FS-workspace.8.6).
    pub project: Option<String>,
    pub path: String,
    pub citations: Vec<CoverCitation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApiScanError {
    pub path: String,
    pub message: String,
}

fn api_scan_error(config: &Config, path: &Path, message: &str) -> ApiScanError {
    ApiScanError {
        path: display_path(config, path),
        message: message.to_string(),
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CoverOutput {
    pub output_format: String,
    pub entries: Vec<CoverEntry>,
    pub scan_errors: Vec<ApiScanError>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoverTextCitation {
    pub line: usize,
    pub column: usize,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoverTextEntry {
    pub path: String,
    pub citations: Vec<CoverTextCitation>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CoverTextOutput {
    pub output_format: String,
    pub entries: Vec<CoverTextEntry>,
    pub scan_errors: Vec<ApiScanError>,
}

/// One scanned file's row in the cover index, with the project that owns it
/// (§FS-workspace.8.6). Borrowed from the [`WorkspaceContext`] the caller holds
/// so both `cover` and `cover_text` build the index once, the same way.
struct CoverRow<'a> {
    /// `None` outside workspace mode — see [`CoverEntry::project`].
    alias: Option<&'a str>,
    /// Rendered against the workspace root in workspace mode, so a member's
    /// file is spelled the way `[workspace] members` spells it
    /// (§FS-workspace.8.6).
    path: String,
    citations: Vec<CoverCitationRow<'a>>,
}

struct CoverCitationRow<'a> {
    citation: &'a Citation,
    /// The config the ID renders under: the **target** project's, matching
    /// `refs` (§FS-workspace.8.2). Falls back to the citing project's config
    /// when the alias names no loaded project — `cover` reports the graph and
    /// leaves the unknown-alias verdict to `check` (§FS-workspace.8.1).
    target_config: &'a Config,
}

impl CoverCitationRow<'_> {
    /// The canonical spelling of the token the file wrote: alias-qualified only
    /// when the citation itself was, never qualified on the file's behalf
    /// (§FS-workspace.8.6).
    fn rendered_id(&self) -> String {
        let id = render_id(self.target_config, &self.citation.id);
        match &self.citation.namespace {
            Some(alias) => format!("{alias}/{id}"),
            None => id,
        }
    }
}

/// Group every scanned file in the run — every project the loader returned —
/// with every citation in it, qualified or not (§FS-workspace.8.6,
/// §DF-cover-workspace-scope). No consumer-end filter: `cover` is keyed by
/// file, so a dropped row and a file with nothing to say print the same, which
/// is the silent skip §REQ-no-missed-citation.1 forbids.
fn cover_rows(context: &WorkspaceContext) -> Vec<CoverRow<'_>> {
    // A file lives in exactly one project (§FS-workspace.6), so the index needs
    // no merge across projects; keying by absolute path is enough to keep the
    // per-file grouping and the owning alias in step.
    let mut by_file: BTreeMap<&PathBuf, (usize, Vec<CoverCitationRow>)> = BTreeMap::new();
    for (index, project) in context.projects.iter().enumerate() {
        for file in &project.findings.scanned_files {
            by_file.entry(file).or_insert_with(|| (index, Vec::new()));
        }
        for citation in &project.findings.citations {
            let target_config = citation
                .namespace
                .as_deref()
                .and_then(|alias| context.project_by_alias(alias))
                .map(|target| &target.config)
                .unwrap_or(&project.config);
            by_file
                .entry(&citation.file)
                .or_insert_with(|| (index, Vec::new()))
                .1
                .push(CoverCitationRow {
                    citation,
                    target_config,
                });
        }
    }

    let mut rows = by_file
        .into_iter()
        .map(|(file, (index, mut citations))| {
            citations.sort_by_key(|row| (row.citation.line, row.citation.column));
            let project = &context.projects[index];
            let path = if context.workspace_loaded {
                display_path(context.render_config(), file)
            } else {
                display_path(&project.config, file)
            };
            CoverRow {
                alias: context.workspace_loaded.then_some(project.alias.as_str()),
                path,
                citations,
            }
        })
        .collect::<Vec<_>>();
    // §FS-cover.2: sorted by the *rendered* path, so the order a caller reads
    // is the order it can diff against (§FS-errors.4).
    rows.sort_by(|left, right| left.path.cmp(&right.path));
    rows
}

/// Load the projects `cover` indexes and their scan errors, rendered against
/// the same base as the rows (§FS-workspace.8.6).
///
/// The narrowable loader, not the plain one: `cover`'s `<path>` bounds the walk
/// (§FS-cover.1), so a scope inside the workspace root stays one narrowed scan
/// the way `grund check <dir>` does.
///
/// §AR-scanner.2.4: `cover` groups citations by file and never reads
/// citing-side classification — skip the scan post-pass (§AR-benchmarks).
fn cover_context(opts: &CoverOpts) -> Result<WorkspaceContext> {
    load_narrowable_workspace_context(&opts.path, opts.path_provided)
}

/// Every loaded project's scan errors, in project order (§FS-workspace.8.7):
/// a member's unreadable file fails the run at the workspace root, because the
/// index the run just printed is incomplete for the tree it claimed.
fn cover_scan_errors(context: &WorkspaceContext) -> Vec<ApiScanError> {
    // The same base the rows render against — the workspace root in workspace
    // mode, the only project otherwise, which is what `render_config` already
    // holds. A member's path spelled against the member names a file that does
    // not exist from where the run was launched (§FS-errors.4).
    let config = context.render_config();
    context
        .projects
        .iter()
        .flat_map(|project| {
            project
                .scan_errors
                .iter()
                .map(|(path, message)| api_scan_error(config, path, message))
        })
        .collect()
}

/// Programmatic `cover`: group every citation by scanned file, across every
/// project the run loaded, without choosing a CLI output format or process exit
/// code (§RM-core-cli-split, §FS-workspace.8.6).
pub fn cover(opts: CoverOpts) -> Result<CoverOutput> {
    let context = cover_context(&opts)?;
    let entries = cover_rows(&context)
        .into_iter()
        .map(|row| CoverEntry {
            project: row.alias.map(str::to_string),
            citations: row
                .citations
                .iter()
                .map(|citation_row| CoverCitation {
                    project: row.alias.map(str::to_string),
                    // The nested object repeats the row's path so `cover` and
                    // `refs` JSON stay comparable field for field (§FS-cover.3.2).
                    path: row.path.clone(),
                    line: citation_row.citation.line,
                    column: citation_row.citation.column,
                    id: citation_row.rendered_id(),
                    section: citation_row.citation.section.clone(),
                    marker: citation_row.citation.has_marker,
                    text: citation_row.citation.text.clone(),
                })
                .collect(),
            path: row.path,
        })
        .collect();
    Ok(CoverOutput {
        output_format: context.render_config().output_format.clone(),
        entries,
        scan_errors: cover_scan_errors(&context),
    })
}

/// Programmatic text-oriented `cover`: return only the citation fields needed
/// for the default human-readable cover view while still leaving rendering to
/// frontends (§RM-core-cli-split). Same index as [`cover`] (§FS-workspace.8.6);
/// the text view carries no alias because the path already renders from the
/// workspace root and the token is printed verbatim (§FS-cover.3.1).
pub fn cover_text(opts: CoverOpts) -> Result<CoverTextOutput> {
    let context = cover_context(&opts)?;
    let entries = cover_rows(&context)
        .into_iter()
        .map(|row| CoverTextEntry {
            path: row.path,
            citations: row
                .citations
                .iter()
                .map(|citation_row| CoverTextCitation {
                    line: citation_row.citation.line,
                    column: citation_row.citation.column,
                    text: citation_row.citation.text.clone(),
                })
                .collect(),
        })
        .collect();
    Ok(CoverTextOutput {
        output_format: context.render_config().output_format.clone(),
        entries,
        scan_errors: cover_scan_errors(&context),
    })
}

#[derive(Clone)]
pub struct FmtOpts {
    pub path: PathBuf,
    pub path_provided: bool,
    pub write: bool,
    pub add_marker: bool,
    pub cross_refs: bool,
}

impl Default for FmtOpts {
    fn default() -> Self {
        Self {
            path: PathBuf::from("."),
            path_provided: false,
            write: false,
            add_marker: false,
            cross_refs: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FmtChange {
    pub path: String,
    pub line: usize,
    /// The rewrite class, and — for a line that expanded a number-only shorthand
    /// — the text it wrote (§FS-fmt.3). A `String` rather than one of four fixed
    /// labels because that detail is per-site.
    pub label: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FmtOutput {
    pub changes: Vec<FmtChange>,
    /// The paths the walk could not read (§FS-fmt.3) — the CLI prints these and
    /// exits `2`. Non-empty means the rewrite ran over less than the whole tree.
    pub scan_errors: Vec<ApiScanError>,
    /// The files read but not rewritten, because a link reaches them from outside
    /// the config root (§FS-fmt.2.3.2). The CLI names each one on stderr; the
    /// exit code is untouched, because the refusal is the intended behavior.
    pub refused_writes: Vec<String>,
}

/// Programmatic `fmt`: run the normalizer and return the changed locations
/// without printing the CLI report or mapping the exit code (§RM-core-cli-split).
pub fn format_references(opts: FmtOpts) -> Result<FmtOutput> {
    let context = load_workspace_context(&opts.path, opts.path_provided)?;
    let config = context.render_config().clone();
    let explicit_cross_refs = opts.cross_refs;
    let workspace_for_wrap = if context.workspace_loaded {
        Some(&context)
    } else {
        None
    };
    let mut changes: Vec<(PathBuf, usize, String)> = Vec::new();
    let mut scan_errors: Vec<ApiScanError> = Vec::new();
    let mut refused_writes: Vec<String> = Vec::new();
    let walk_all_projects = context.workspace_loaded
        && (!opts.path_provided
            || fs::canonicalize(&opts.path)
                .map(|canonical| canonical == config.root)
                .unwrap_or(false));
    if walk_all_projects {
        for project in &context.projects {
            let auto_cross_refs = auto_cross_refs_for_scope(
                &project.config,
                Some(&project.config.root),
                true,
                opts.write,
            )?;
            let run_opts = FmtRunOpts {
                add_marker: opts.add_marker,
                cross_refs: explicit_cross_refs || auto_cross_refs,
                write: opts.write,
                render: &config,
                workspace: workspace_for_wrap,
                precomputed_findings: Some(&project.findings),
            };
            let mut walked = fmt_tree(
                &project.config,
                Some(&project.config.root),
                true,
                &run_opts,
            )?;
            changes.append(&mut walked.changes);
            scan_errors.append(&mut walked.scan_errors);
            refused_writes.append(&mut walked.refused_writes);
        }
    } else {
        let reusable_findings = (!opts.path_provided)
            .then(|| context.current_project().map(|project| &project.findings))
            .flatten();
        let auto_cross_refs =
            auto_cross_refs_for_scope(&config, Some(&opts.path), opts.path_provided, opts.write)?;
        let run_opts = FmtRunOpts {
            add_marker: opts.add_marker,
            cross_refs: explicit_cross_refs || auto_cross_refs,
            write: opts.write,
            render: &config,
            workspace: workspace_for_wrap,
            precomputed_findings: reusable_findings,
        };
        let walked = fmt_tree(&config, Some(&opts.path), opts.path_provided, &run_opts)?;
        changes = walked.changes;
        scan_errors = walked.scan_errors;
        refused_writes = walked.refused_writes;
    }

    Ok(FmtOutput {
        changes: changes
            .into_iter()
            .map(|(path, line, label)| FmtChange {
                path: display_path(&config, &path),
                line,
                label,
            })
            .collect(),
        scan_errors,
        refused_writes,
    })
}

#[derive(Clone)]
pub struct ListOpts {
    pub path: PathBuf,
    pub path_provided: bool,
    pub kind_filter: BTreeSet<String>,
    pub project_filter: BTreeSet<String>,
    pub unused_only: bool,
}

impl Default for ListOpts {
    fn default() -> Self {
        Self {
            path: PathBuf::from("."),
            path_provided: false,
            kind_filter: BTreeSet::new(),
            project_filter: BTreeSet::new(),
            unused_only: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListEntry {
    pub project: Option<String>,
    pub id: String,
    pub kind: String,
    pub path: String,
    pub line: usize,
    pub title: Option<String>,
    pub stub: bool,
    pub defines: Option<String>,
    pub refs: usize,
    pub duplicate: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListSummary {
    pub project: Option<String>,
    pub kind: String,
    pub title: String,
    pub home: String,
    pub count: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ListOutput {
    pub output_format: String,
    pub workspace: bool,
    pub entries: Vec<ListEntry>,
    pub summaries: Vec<ListSummary>,
    pub scan_errors: Vec<ApiScanError>,
}

fn list_summary_home(kind: &KindConfig) -> String {
    kind.file
        .as_deref()
        .or(kind.folder.as_deref())
        .unwrap_or_default()
        .to_string()
}

/// Programmatic `list`: return the catalog and per-kind summary rows without
/// selecting text/JSON rendering or an exit code (§RM-core-cli-split).
pub fn list(opts: ListOpts) -> Result<ListOutput> {
    let context = load_workspace_context(&opts.path, opts.path_provided)?;
    if !opts.project_filter.is_empty() && !context.workspace_loaded {
        return Err(anyhow!(
            "--project requires workspace mode (no [workspace] block discovered)"
        ));
    }
    for alias in &opts.project_filter {
        if context.project_by_alias(alias).is_none() {
            let known = context.aliases().join(", ");
            return if known.is_empty() {
                Err(anyhow!("unknown project alias `{alias}`"))
            } else {
                Err(anyhow!("unknown project alias `{alias}`\nknown aliases: {known}"))
            };
        }
    }
    for kind in &opts.kind_filter {
        let exists = context
            .projects
            .iter()
            .filter(|project| {
                opts.project_filter.is_empty() || opts.project_filter.contains(&project.alias)
            })
            .any(|project| {
                project
                    .config
                    .kinds
                    .iter()
                    .any(|candidate| &candidate.prefix == kind)
            });
        if !exists {
            let mut known: Vec<String> = Vec::new();
            let mut seen: BTreeSet<String> = BTreeSet::new();
            for project in &context.projects {
                for k in &project.config.kinds {
                    if seen.insert(k.prefix.clone()) {
                        known.push(k.prefix.clone());
                    }
                }
            }
            return Err(anyhow!(
                "unknown kind `{kind}`\nknown kinds: {}",
                known.join(", ")
            ));
        }
    }

    struct Entry<'a> {
        project_alias: &'a str,
        project_config: &'a Config,
        id: &'a Id,
        home: &'a Declaration,
        duplicate: bool,
        refs: usize,
    }

    let mut ref_counts_by_alias: BTreeMap<&str, BTreeMap<&Id, usize>> = BTreeMap::new();
    for source in &context.projects {
        for citation in &source.findings.citations {
            let target_alias: &str = match &citation.namespace {
                Some(ns) => ns.as_str(),
                None => source.alias.as_str(),
            };
            *ref_counts_by_alias
                .entry(target_alias)
                .or_default()
                .entry(&citation.id)
                .or_insert(0) += 1;
        }
    }
    let empty_ref_counts: BTreeMap<&Id, usize> = BTreeMap::new();
    let mut entries: Vec<Entry<'_>> = Vec::new();
    let mut scan_errors = Vec::new();
    for project in &context.projects {
        if !opts.project_filter.is_empty() && !opts.project_filter.contains(&project.alias) {
            continue;
        }
        scan_errors.extend(
            project
                .scan_errors
                .iter()
                .map(|(file, message)| api_scan_error(&project.config, file, message)),
        );
        let ref_counts: &BTreeMap<&Id, usize> = ref_counts_by_alias
            .get(project.alias.as_str())
            .unwrap_or(&empty_ref_counts);
        for (id, decls) in &project.findings.declarations {
            if !opts.kind_filter.is_empty() && !opts.kind_filter.contains(&id.kind) {
                continue;
            }
            let refs = ref_counts.get(id).copied().unwrap_or(0);
            if opts.unused_only && refs > 0 {
                continue;
            }
            if opts.unused_only && id.kind == "E2E" && !opts.kind_filter.contains("E2E") {
                continue;
            }
            let mut homes: Vec<&Declaration> = decls
                .iter()
                .filter(|decl| !is_stub_for_inline_decl(&project.config.root, decl, decls))
                .collect();
            homes.sort_by(|a, b| {
                (sort_path_key(&a.file), a.line).cmp(&(sort_path_key(&b.file), b.line))
            });
            let duplicate = homes.len() > 1;
            for home in homes {
                entries.push(Entry {
                    project_alias: project.alias.as_str(),
                    project_config: &project.config,
                    id,
                    home,
                    duplicate,
                    refs,
                });
            }
        }
    }
    if context.workspace_loaded {
        entries.sort_by(|a, b| {
            (a.project_alias, a.id, sort_path_key(&a.home.file), a.home.line).cmp(&(
                b.project_alias,
                b.id,
                sort_path_key(&b.home.file),
                b.home.line,
            ))
        });
    }

    let render_qualified = |entry: &Entry<'_>| -> String {
        if context.workspace_loaded {
            format!(
                "{}/{}",
                entry.project_alias,
                render_id(entry.project_config, entry.id)
            )
        } else {
            render_id(entry.project_config, entry.id)
        }
    };
    let render_config = context.render_config();
    let public_entries = entries
        .iter()
        .map(|entry| ListEntry {
            project: context
                .workspace_loaded
                .then(|| entry.project_alias.to_string()),
            id: render_qualified(entry),
            kind: entry.id.kind.clone(),
            path: display_path(render_config, &entry.home.file),
            line: entry.home.line,
            title: entry.home.title.clone(),
            stub: entry.home.is_stub,
            defines: entry.home.defined_in.as_ref().map(|target| format_path(target)),
            refs: entry.refs,
            duplicate: entry.duplicate,
        })
        .collect::<Vec<_>>();

    let mut summaries = Vec::new();
    if context.workspace_loaded {
        let mut counts: BTreeMap<(String, String), usize> = BTreeMap::new();
        for entry in &entries {
            *counts
                .entry((entry.project_alias.to_string(), entry.id.kind.clone()))
                .or_insert(0) += 1;
        }
        for project in &context.projects {
            if !opts.project_filter.is_empty() && !opts.project_filter.contains(&project.alias) {
                continue;
            }
            for kind in &project.config.kinds {
                let count = counts
                    .get(&(project.alias.clone(), kind.prefix.clone()))
                    .copied()
                    .unwrap_or(0);
                if count == 0 {
                    continue;
                }
                summaries.push(ListSummary {
                    project: Some(project.alias.clone()),
                    kind: kind.prefix.clone(),
                    title: kind.title.clone().unwrap_or_else(|| "Declaration".to_string()),
                    home: list_summary_home(kind),
                    count,
                });
            }
        }
    } else {
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for entry in &entries {
            *counts.entry(&entry.id.kind).or_insert(0) += 1;
        }
        for kind in &render_config.kinds {
            let count = counts.get(kind.prefix.as_str()).copied().unwrap_or(0);
            if count == 0 {
                continue;
            }
            summaries.push(ListSummary {
                project: None,
                kind: kind.prefix.clone(),
                title: kind.title.clone().unwrap_or_else(|| "Declaration".to_string()),
                home: list_summary_home(kind),
                count,
            });
        }
    }

    Ok(ListOutput {
        output_format: render_config.output_format.clone(),
        workspace: context.workspace_loaded,
        entries: public_entries,
        summaries,
        scan_errors,
    })
}

#[derive(Clone)]
pub struct RefsOpts {
    pub path: PathBuf,
    pub path_provided: bool,
    pub id: String,
    pub section: Option<String>,
}

impl Default for RefsOpts {
    fn default() -> Self {
        Self {
            path: PathBuf::from("."),
            path_provided: false,
            id: String::new(),
            section: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefHit {
    pub project: Option<String>,
    pub path: String,
    pub line: usize,
    pub column: usize,
    pub id: String,
    pub section: Option<String>,
    pub marker: bool,
    pub text: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RefsOutput {
    pub output_format: String,
    pub workspace: bool,
    pub hits: Vec<RefHit>,
    pub note: Option<String>,
    pub scan_errors: Vec<ApiScanError>,
}

/// Programmatic `refs`: resolve an ID query and return all citation sites
/// without selecting text/summary/JSON rendering (§RM-core-cli-split).
pub fn refs(opts: RefsOpts) -> Result<RefsOutput> {
    let context = load_workspace_context(&opts.path, opts.path_provided)?;
    let current_config = context
        .current_project()
        .map(|project| &project.config)
        .unwrap_or_else(|| context.render_config());
    let (alias, raw_id) = split_qualified_id_arg(&opts.id).map_err(|err| {
        anyhow!(
            "{err:#}\nhint: this repo's [id] format is `{}` (run `grund config show`); `grund list` shows the IDs that exist",
            current_config.id_format
        )
    })?;
    let target_project = match alias.as_deref() {
        Some(name) => context.project_by_alias(name).ok_or_else(|| {
            if !context.workspace_loaded {
                anyhow!(
                    "unknown project alias `{name}`\nnote: workspace aliases are defined in the root grund.toml under [workspace]"
                )
            } else {
                anyhow!(
                    "unknown project alias `{name}`\nknown aliases: {}",
                    context.aliases().join(", ")
                )
            }
        })?,
        None => context.current_project().ok_or_else(|| {
            let known = context.aliases().join(", ");
            if known.is_empty() {
                anyhow!("unqualified ID requires a project alias when include_root = false")
            } else {
                anyhow!(
                    "unqualified ID requires a project alias when include_root = false\nknown aliases: {known}"
                )
            }
        })?,
    };
    let target_alias = target_project.alias.as_str();
    let render_config = &target_project.config;
    // §FS-refs.4: the `[id] format` hint is for an argument that does not match
    // it; an ambiguous shorthand did match and lists its candidates instead.
    let (id, inline_section) = resolve_id_arg(raw_id, render_config, &target_project.findings)
        .map_err(|err| {
            if err.wants_format_hint() {
                anyhow!(
                    "{err}\nhint: this repo's [id] format is `{}` (run `grund config show`); `grund list` shows the IDs that exist",
                    render_config.id_format
                )
            } else {
                anyhow!("{err}")
            }
        })?;
    if opts.section.is_some() && inline_section.is_some() {
        return Err(anyhow!(
            "--section cannot be combined with an inline section"
        ));
    }
    let section = opts.section.or(inline_section);

    struct Hit<'a> {
        project: &'a WorkspaceProject,
        citation: &'a Citation,
    }
    let mut hits = Vec::new();
    let mut scan_errors = Vec::new();
    for project in &context.projects {
        scan_errors.extend(
            project
                .scan_errors
                .iter()
                .map(|(file, message)| api_scan_error(&project.config, file, message)),
        );
        let is_target = project.alias == target_alias;
        for citation in &project.findings.citations {
            let local_match = citation.namespace.is_none() && is_target;
            let qualified_match = citation
                .namespace
                .as_deref()
                .map(|ns| ns == target_alias)
                .unwrap_or(false);
            if !(local_match || qualified_match) || citation.id != id {
                continue;
            }
            if let Some(expected) = section.as_deref()
                && citation.section.as_deref() != Some(expected)
            {
                continue;
            }
            hits.push(Hit { project, citation });
        }
    }
    hits.sort_by(|a, b| {
        (sort_path_key(&a.citation.file), a.citation.line, a.citation.column).cmp(&(
            sort_path_key(&b.citation.file),
            b.citation.line,
            b.citation.column,
        ))
    });
    let render_path = |project: &WorkspaceProject, path: &Path| -> String {
        if context.workspace_loaded {
            display_path(context.render_config(), path)
        } else {
            display_path(&project.config, path)
        }
    };
    let public_hits = hits
        .iter()
        .map(|hit| RefHit {
            project: context.workspace_loaded.then(|| hit.project.alias.clone()),
            path: render_path(hit.project, &hit.citation.file),
            line: hit.citation.line,
            column: hit.citation.column,
            id: render_id(render_config, &hit.citation.id),
            section: hit.citation.section.clone(),
            marker: hit.citation.has_marker,
            text: hit.citation.text.clone(),
        })
        .collect::<Vec<_>>();
    let note = if public_hits.is_empty() && !target_project.findings.declarations.contains_key(&id)
    {
        if context.workspace_loaded && alias.is_some() {
            Some(format!(
                "{}/{} is neither declared nor cited — run `grund list --project {}` to see {}'s declared IDs",
                target_alias,
                render_id(render_config, &id),
                target_alias,
                target_alias
            ))
        } else {
            Some(format!(
                "{} is neither declared nor cited — run `grund list` to see every declared ID",
                render_id(render_config, &id)
            ))
        }
    } else {
        None
    };
    Ok(RefsOutput {
        output_format: render_config.output_format.clone(),
        workspace: context.workspace_loaded,
        hits: public_hits,
        note,
        scan_errors,
    })
}
