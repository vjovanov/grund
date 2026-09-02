/// The grounding pass (§FS-check.3.6), in a file of its own beside
/// `checker_citations.rs` and `checker_sections.rs` (§AR-core-module-layout.1):
/// which `[[kinds]]` row governs each scanned file, what unit that row's
/// `grounding_level` cuts the file into, and which of those units carry no
/// citation to a declared ID.
///
/// It is the sixth rule to leave `check_with_workspace` as a named function
/// rather than an inline block, and it leaves because it stopped being one: the
/// per-file test became a per-unit one over structure the scanner records
/// (§AR-scanner.2.7), with a row lookup in front of it.

/// One thing a row's `grounding_level` asks for a citation (§FS-check.3.6.2):
/// where it starts, where it ends, the line a finding anchors at, and how the
/// message names it.
struct GroundingUnit {
    start: usize,
    end: usize,
    subject: GroundingSubject,
}

/// How a unit is named in its finding (§FS-check.3.6.3). The home, when the unit
/// sits in a non-citable one, is added by the caller — one place decides it, so
/// every shape reads the same way.
enum GroundingSubject {
    File,
    Section(String),
    DocComment,
}

/// §FS-check.3.6 / §DF-require-grounding: every unit of every governed file must
/// carry a citation to a declared ID — or, in a source file outside a
/// non-citable home, declare one inline (a spec home is grounded in the spec it
/// *is*).
fn check_grounding(
    findings: &Findings,
    config: &Config,
    kind_homes: &KindHomeIndex<'_>,
    workspace: &BTreeMap<String, WorkspaceCheckTarget<'_>>,
    report: &mut CheckReport,
) {
    if !config.grounding_enabled() {
        return;
    }
    // Two linear passes — citations, then declarations — so the per-unit test
    // below is a lookup in a small per-file list, never a re-scan
    // (§GOAL-fast-feedback).
    let mut cited: BTreeMap<&Path, Vec<usize>> = BTreeMap::new();
    for cite in &findings.citations {
        if citation_resolves(cite, findings, config, workspace) {
            cited.entry(cite.file.as_path()).or_default().push(cite.line);
        }
    }
    let mut declared: BTreeMap<&Path, Vec<usize>> = BTreeMap::new();
    for decl in findings.declarations.values().flatten() {
        if !decl.is_stub && decl.e2e_case.is_none() {
            declared
                .entry(decl.file.as_path())
                .or_default()
                .push(decl.line);
        }
    }

    for file in &findings.scanned_files {
        let home = kind_homes.unique_decl_home_for_file(file);
        let Some((require, level)) = governing_grounding(config, home.as_ref(), file) else {
            continue;
        };
        if !require {
            continue;
        }
        let place = home.as_ref().filter(|home| !home.citable).map(|home| home.place());
        let empty = Vec::new();
        let mut grounding: Vec<usize> = cited.get(file.as_path()).unwrap_or(&empty).clone();
        // §FS-check.3.6.2: no inline-declaration escape in a non-citable home —
        // a declaration there is already misplaced (§FS-check.3.7), so the only
        // way to ground a unit is to cite one.
        if place.is_none() {
            grounding.extend(declared.get(file.as_path()).unwrap_or(&empty));
        }
        for unit in grounding_units(findings.file_structure.get(file), level) {
            if grounding
                .iter()
                .any(|line| unit.start <= *line && *line <= unit.end)
            {
                continue;
            }
            report.errors.push(Diagnostic {
                code: "ungrounded",
                path: Some(file.clone()),
                line: Some(unit.start),
                column: None,
                message: format!(
                    "{}: no {} citation to a declared ID",
                    ungrounded_subject(&unit, place.as_deref()),
                    config.marker
                ),
                sites: Vec::new(),
            });
        }
    }
}

/// The effective `(require_grounding, grounding_level)` for the row that governs
/// `file`, or `None` when no row does (§FS-check.3.6.1). Three predicates, which
/// are §FS-check.3.6's own: a non-citable home governs every scanned file in it,
/// a citable folder home governs the source files in it, and the homeless kind
/// governs the source files no single home claims.
fn governing_grounding(
    config: &Config,
    home: Option<&DeclarationHome<'_>>,
    file: &Path,
) -> Option<(bool, usize)> {
    let is_markdown = file.extension().and_then(|ext| ext.to_str()) == Some("md");
    match home {
        Some(home) if !home.citable => config
            .kinds
            .iter()
            .find(|kind| kind.kind == home.kind)
            .map(|kind| config.kind_grounding(kind)),
        // A Markdown document is not implementation, so a citable home and the
        // homeless complement both leave it alone; the exception above is a home
        // rather than an extension.
        _ if is_markdown => None,
        Some(home) => config
            .kinds
            .iter()
            .find(|kind| kind.kind == home.kind)
            .map(|kind| config.kind_grounding(kind)),
        None => Some(config.homeless_grounding()),
    }
}

/// The units one file offers at `level` (§FS-check.3.6.2). Each level contains
/// the one below it, so the file is always a unit and nothing passes vacuously
/// for lacking structure; above level 1 the recorded structure adds the heading
/// subtrees of a Markdown file, or the doc-comment blocks of a source file.
fn grounding_units(structure: Option<&FileStructure>, level: usize) -> Vec<GroundingUnit> {
    let end = structure.map_or(usize::MAX, |structure| structure.total_lines.max(1));
    let mut units = vec![GroundingUnit {
        start: 1,
        end,
        subject: GroundingSubject::File,
    }];
    let Some(structure) = structure.filter(|_| level > DEFAULT_GROUNDING_LEVEL) else {
        return units;
    };
    for (index, heading) in structure.headings.iter().enumerate() {
        if heading.level < 2 || heading.level > level {
            continue;
        }
        // The subtree runs to the line before the next heading at the same or a
        // higher level, which is what makes a parent satisfied by any descendant
        // and a leaf answerable for itself.
        let next = structure.headings[index + 1..]
            .iter()
            .find(|later| later.level <= heading.level)
            .map(|later| later.line - 1)
            .unwrap_or(structure.total_lines);
        units.push(GroundingUnit {
            start: heading.line,
            end: next.max(heading.line),
            subject: GroundingSubject::Section(format!(
                "{} {}",
                "#".repeat(heading.level),
                heading.text
            )),
        });
    }
    for block in &structure.doc_comments {
        // §FS-check.3.6.2: level 2 reaches the unindented blocks — the parse-free
        // stand-in for a top-level item — and any higher level reaches them all.
        if block.indented && level == 2 {
            continue;
        }
        units.push(GroundingUnit {
            start: block.start,
            end: block.end,
            subject: GroundingSubject::DocComment,
        });
    }
    units
}

/// The line spans one file offers as units at `level` (§FS-check.3.11) — the
/// same cut §FS-check.3.6.2 makes for grounding, because *whether* a place's
/// files must cite and *what* they must cite are asked of the same thing.
fn grounding_unit_spans(structure: Option<&FileStructure>, level: usize) -> Vec<(usize, usize)> {
    grounding_units(structure, level)
        .into_iter()
        .map(|unit| (unit.start, unit.end))
        .collect()
}

/// The effective grounding level of the row named `kind` (§FS-config.3.4.8) —
/// including the homeless kind, whose row a config need not have declared.
fn grounding_level_for_kind(config: &Config, kind: &str) -> usize {
    config
        .kinds
        .iter()
        .find(|configured| configured.kind == kind)
        .map(|configured| config.kind_grounding(configured).1)
        .unwrap_or_else(|| config.homeless_grounding().1)
}

/// The obligation units of one citing kind that has no declarations to attach
/// an obligation to — the homeless kind and every non-citable one
/// (§FS-check.3.11). Their unit is the file, cut by the row's
/// `grounding_level` exactly as the grounding pass cuts it, so *whether* a
/// place's files must cite and *what* they must cite are asked of one thing. A
/// unit carrying no citation is not a unit: obligations constrain what a file
/// cites, never whether it cites at all. At level 1 — every configuration
/// written before the key existed — this is one unit per file at line 1.
fn file_obligation_units<'a>(
    citing_kind: &str,
    config: &Config,
    findings: &'a Findings,
    by_file: &BTreeMap<(&'a str, &'a Path), Vec<&'a Citation>>,
) -> Vec<ObligationUnit<'a>> {
    let place = config
        .kinds
        .iter()
        .find(|kind| kind.kind == citing_kind)
        .and_then(KindConfig::place_label);
    let level = grounding_level_for_kind(config, citing_kind);
    by_file
        .iter()
        .filter(|((kind, _), _)| *kind == citing_kind)
        .flat_map(|((_, file), citations)| {
            grounding_unit_spans(findings.file_structure.get(*file), level)
                .into_iter()
                .filter_map(|(start, end)| {
                    let inside: Vec<&Citation> = citations
                        .iter()
                        .copied()
                        .filter(|cite| start <= cite.line && cite.line <= end)
                        .collect();
                    (!inside.is_empty()).then(|| ObligationUnit {
                        id: None,
                        place: place.clone(),
                        path: file.to_path_buf(),
                        line: start,
                        citations: inside,
                        e2e_spec_refs: Vec::new(),
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// How an ungrounded unit is named (§FS-check.3.6.3): the unit, then the
/// non-citable home it sits in when it sits in one. A section unit only ever
/// arises inside such a home, since that is the only place Markdown is governed.
fn ungrounded_subject(unit: &GroundingUnit, place: Option<&str>) -> String {
    let subject = match (&unit.subject, place) {
        (GroundingSubject::File, None) => "ungrounded source file".to_string(),
        (GroundingSubject::File, Some(_)) => "ungrounded file".to_string(),
        (GroundingSubject::Section(heading), _) => format!("ungrounded section `{heading}`"),
        (GroundingSubject::DocComment, _) => "ungrounded doc-comment".to_string(),
    };
    match place {
        Some(place) => format!("{subject} in kind home {place}"),
        None => subject,
    }
}
