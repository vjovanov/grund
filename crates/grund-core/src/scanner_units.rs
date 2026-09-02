/// The per-file structure a grounding unit is cut out of (§AR-scanner.2.7),
/// beside `scanner.rs` in the scanner category (§AR-core-module-layout.1).
///
/// This is a *description* of a file, not a list of units: the level that turns
/// headings and doc-comment blocks into units belongs to the `[[kinds]]` row that
/// governs the file (§FS-config.3.4.8), and the cut is made in
/// `checker_grounding.rs` so the level rule is written once. It runs per file,
/// only where that file's own row asks for a unit finer than the file, so one
/// fine-grained place does not describe the whole tree and a project at level `1`
/// — every configuration written before the keys existed — records nothing and
/// pays nothing (§GOAL-fast-feedback).

/// One Markdown heading outside a fence (§AR-scanner.2.7): its line, its level,
/// and its text without the leading `#`s, which is what a section finding quotes
/// back (§FS-check.3.6.3).
pub struct FileHeading {
    pub line: usize,
    pub level: usize,
    pub text: String,
}

/// One doc-comment block in a source file (§AR-scanner.2.7): its 1-indexed
/// inclusive line span, and whether its first line starts at column 0.
/// Indentation is the parse-free stand-in for "top-level item" that
/// §FS-check.3.6.2 reads at level 2 — it holds across Rust, Python, Java, Go, and
/// Kotlin without knowing any of them (§FS-non-goals.3).
pub struct DocCommentBlock {
    pub start: usize,
    pub end: usize,
    pub indented: bool,
}

/// One file's grounding structure (§AR-scanner.2.7). A Markdown file fills
/// `headings` and a source file `doc_comments`; `total_lines` closes the last
/// subtree or block, which would otherwise have no end.
#[derive(Default)]
pub struct FileStructure {
    pub headings: Vec<FileHeading>,
    pub doc_comments: Vec<DocCommentBlock>,
    pub total_lines: usize,
}

/// Record `path`'s grounding structure into `findings`, or do nothing when the
/// row this file belongs to asks for no unit finer than the file
/// (§AR-scanner.2.7). Called once per file from the per-file scan, with the text
/// it already read.
fn record_file_structure(path: &Path, text: &str, config: &Config, findings: &mut Findings) {
    // The project-wide answer first: one field read exempts every file of a
    // level-1 tree — every configuration written before the keys existed — from
    // the per-file lookup below (§GOAL-fast-feedback).
    if !config.grounding_units || file_grounding_level(path, config) <= DEFAULT_GROUNDING_LEVEL {
        return;
    }
    let extension = path.extension().and_then(|ext| ext.to_str());
    let structure = if extension == Some("md") {
        markdown_structure(text)
    } else {
        source_structure(path, text, extension == Some("py"), config)
    };
    findings.file_structure.insert(path.to_path_buf(), structure);
}

/// The effective `grounding_level` of the row `path` belongs to (§AR-scanner.2.7)
/// — its home kind's, or the homeless kind's where no single home claims it
/// (§AR-scanner.2.4). That lookup is the one §FS-check.3.6.1 defers to for which
/// row governs a file, and the level it feeds is the checker's own, so what is
/// recorded here and what is cut out of it later are one rule.
///
/// A Markdown document in a *citable* home is recorded when its row asks for a
/// finer unit, though §FS-check.3.6 will not ask for its units: the row's level
/// is a statement about the place, and erring toward having the structure costs
/// one description, while a second home rule here could disagree with that one.
fn file_grounding_level(path: &Path, config: &Config) -> usize {
    let home = file_home_kind(path, config);
    grounding_level_for_kind(config, home.as_deref().unwrap_or(config.homeless_kind()))
}

/// Every heading outside a fenced block, with its text (§AR-scanner.2.7). The
/// fence state is the one §AR-scanner.2.3 keeps for citations, for the same
/// reason: a `##` inside a fence is an example of a document, not a section of
/// this one.
fn markdown_structure(text: &str) -> FileStructure {
    let mut structure = FileStructure::default();
    let mut fence = None;
    for (index, line) in text.lines().enumerate() {
        structure.total_lines = index + 1;
        if markdown_fence_delimiter(&mut fence, line) || fence.is_some() {
            continue;
        }
        let trimmed = line.trim_start();
        if let Some(level) = markdown_heading_level(trimmed) {
            structure.headings.push(FileHeading {
                line: index + 1,
                level,
                text: heading_text(trimmed, level),
            });
        }
    }
    structure
}

/// A heading's text: the line without its opening `#`s and without the optional
/// closing run Markdown allows (`## Steps ##`), so a finding quotes the title
/// rather than the syntax (§FS-check.3.6.3).
fn heading_text(trimmed: &str, level: usize) -> String {
    trimmed[level..]
        .trim()
        .trim_end_matches('#')
        .trim_end()
        .to_string()
}

/// Every doc-comment block in a source file, with its indentation
/// (§AR-scanner.2.7). Which blocks are documentation is the per-language rule of
/// §FS-inline-citation-style.1.1, read once per file from the extension and
/// applied to each block with one comparison — the same call the inline-site pass
/// makes, so a block cannot be a doc comment for one rule and a note for the
/// other.
fn source_structure(path: &Path, text: &str, is_py: bool, config: &Config) -> FileStructure {
    let lines = text.lines().collect::<Vec<_>>();
    let doc_rule = doc_comment_rule(path);
    let leading_limit = match doc_rule {
        DocCommentRule::Position(_) => first_content_line(&lines),
        _ => 0,
    };
    let mut structure = FileStructure {
        total_lines: lines.len(),
        ..FileStructure::default()
    };
    for (start, end, kind) in comment_blocks(&lines, is_py, config) {
        let block = &lines[start..=end];
        if block_is_doc_comment(
            doc_rule,
            &kind,
            block,
            lines.get(end + 1).copied(),
            start <= leading_limit,
        ) {
            structure.doc_comments.push(DocCommentBlock {
                start: start + 1,
                end: end + 1,
                indented: lines[start].starts_with(char::is_whitespace),
            });
        }
    }
    structure
}
