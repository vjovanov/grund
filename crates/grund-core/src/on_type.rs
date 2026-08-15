// The LSP live on-type transform (§FS-lsp.1.4) — the keystroke-time counterpart
// to `grund fmt`'s bulk passes (§FS-fmt.2.1, §FS-fmt.2.4).
//
// Split out of `api.rs`, which §AR-core-module-layout.2 keeps as the published
// embedding contract: the public types below are part of that contract, but the
// rule deciding *which* keystroke produces *which* edit is a behavior with its
// own invariant — the trigger converts eagerly and the shorthand expands only at
// a token boundary — and that invariant is what a reader comes here for.
//
// File-level prose, so `//` rather than `///` — see the note in `shorthand.rs`.

/// Check the same context exclusions as `grund fmt` before an LSP on-type
/// `$$` rewrite (§FS-fmt.2.3, §FS-lsp.1.4).
pub fn can_replace_trigger_at(
    path: &Path,
    line: &str,
    trigger_start: usize,
    token: &str,
) -> Result<bool> {
    let config = resolve_workspace_config(path)?;
    Ok(can_replace_trigger_with_config(
        &config,
        path,
        line,
        trigger_start,
        token,
    ))
}

fn can_replace_trigger_with_config(
    config: &Config,
    path: &Path,
    line: &str,
    trigger_start: usize,
    token: &str,
) -> bool {
    let after = trigger_start + config.trigger.len();
    let token_end = after + token.len();
    // §FS-lsp.1.4: the same "is a real ID here" test `grund fmt` uses, so the
    // live transform and the bulk pass consume the same triggers — including the
    // number-only shorthand where the repo has one (§FS-check.1.2).
    if id_token_end_at(line, after, &config.grammar) != Some(token_end) {
        return false;
    }
    let is_md = path.extension().and_then(|ext| ext.to_str()) == Some("md");
    if is_md {
        !is_inside_inline_code(line, trigger_start)
            && !is_inside_markdown_link_destination(line, trigger_start)
    } else {
        !is_inside_string_literal(line, trigger_start)
    }
}

/// One replacement on the edited line: the byte span to replace, and the text to
/// put there (§FS-lsp.1.4).
pub struct LineEdit {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// The edits one on-type keystroke produces on `line` with the cursor at
/// `cursor_byte`, in ascending non-overlapping order — the shape an LSP
/// `TextEdit[]` response needs. Empty when the keystroke changes nothing.
///
/// The file's config is resolved exactly once, so the hot per-keystroke path does
/// a single config walk rather than one per check (§FS-lsp.1.4).
///
/// Two independent rewrites live here, and they fire on *different* keystrokes:
///
/// - **Trigger → marker** (§FS-fmt.2.1), the moment the text after `$$` first
///   reads as an ID. This is eager on purpose: it only replaces the `$$`, so the
///   author keeps typing straight through it.
/// - **Shorthand → canonical** (§FS-fmt.2.4), when the keystroke *ends* the token
///   — a character that cannot continue an ID (`id_token_continues_with`).
///
/// The split is what makes the expansion correct. Under the default format the
/// trigger becomes rewritable at the first *digit*, because `FS-0` is already a
/// well-formed shorthand; expanding there would rewrite the token to whatever
/// `FS-0` happens to name and leave the rest of the number trailing behind it
/// (`$$FS-12` → `§FS-001-login2`). Waiting for the terminator is the only point at
/// which the typed number is known to be finished.
///
/// `declarations` are the declarations already known to the caller's session
/// snapshot, so an expansion costs a list scan and never a fresh tree walk
/// (§GOAL-fast-feedback). Pass an empty slice to get trigger conversion alone.
pub fn on_type_line_edits(
    path: &Path,
    text: &str,
    line_index: usize,
    cursor_byte: usize,
    declarations: &[DeclaredId<'_>],
) -> Result<Vec<LineEdit>> {
    let config = resolve_workspace_config(path)?;
    let Some(line) = text.lines().nth(line_index) else {
        return Ok(Vec::new());
    };
    let cursor = cursor_byte.min(line.len());
    if let Some(edit) = trigger_marker_edit(&config, path, line, cursor) {
        return Ok(vec![edit]);
    }
    Ok(
        shorthand_expansion_edit(&config, path, text, line_index, line, cursor, declarations)
            .into_iter()
            .collect(),
    )
}

/// Whether `grund fmt` would rewrite anything on this line at all — the two
/// whole-line skips `rewrite_file` applies before it looks at any citation: a
/// fenced code block, and a declaration heading (§FS-fmt.2.3).
///
/// Both need the lines *above* the cursor, which is why the on-type entry point
/// takes the document rather than one line. Only the shorthand rewrite consults
/// this: it is the one that edits text the author did not just type, so a live
/// transform that ignored these would silently rewrite an illustration inside a
/// fence, or a citation in the title of a declaration.
fn line_is_rewritable(config: &Config, text: &str, line_index: usize, is_md: bool) -> bool {
    let mut in_fence = false;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if is_md && (trimmed.starts_with("```") || trimmed.starts_with("~~~")) {
            if index == line_index {
                return false;
            }
            in_fence = !in_fence;
            continue;
        }
        if index == line_index {
            return !in_fence
                && declaration_captures(&config.grammar, line, false, is_md).is_none();
        }
    }
    false
}

/// One declaration the caller already knows about: the file it lives in, and its
/// **unqualified** rendered ID (§FS-lsp.1.4).
///
/// The path is what scopes a shorthand to the right namespace. In a workspace the
/// snapshot holds every member's declarations, and `§FS-042` typed in `web` means
/// `web`'s `FS-042-…` — never `api`'s. Filtering by the edited file's config root
/// is what keeps a sibling member from either stealing the expansion or creating a
/// false ambiguity that suppresses it (§FS-workspace.5).
pub struct DeclaredId<'a> {
    pub path: &'a Path,
    pub id: &'a str,
}

/// The IDs of the declarations that live under `root` (§FS-lsp.1.4).
///
/// A plain prefix test is not enough, because the two sides can reach the same
/// directory by different spellings: the editor's URI and the discovered config
/// root need not normalize alike, and on macOS `/var` and `/private/var` name one
/// directory while Windows adds `\\?\` verbatim prefixes. Getting this wrong is
/// silent — every candidate is filtered out and the expansion simply never fires.
///
/// So the raw comparison runs first, and only if it finds nothing does the
/// normalized one run, through the same `canonical_snapshot_path` the LSP snapshot
/// itself is built with (§AR-lsp.5) rather than a second, drift-prone rule about
/// path shapes. On a tree whose paths already agree that costs no I/O at all.
fn declarations_under_root<'a>(declarations: &[DeclaredId<'a>], root: &Path) -> Vec<&'a str> {
    let direct: Vec<&str> = declarations
        .iter()
        .filter(|declared| declared.path.starts_with(root))
        .map(|declared| declared.id)
        .collect();
    if !direct.is_empty() || declarations.is_empty() {
        return direct;
    }
    let canonical_root = canonical_snapshot_path(root);
    declarations
        .iter()
        .filter(|declared| canonical_snapshot_path(declared.path).starts_with(&canonical_root))
        .map(|declared| declared.id)
        .collect()
}

/// The `$$` → `§` conversion for the trigger immediately before `cursor`, when the
/// text between it and the cursor is a whole ID-shaped token (§FS-fmt.2.1).
fn trigger_marker_edit(
    config: &Config,
    path: &Path,
    line: &str,
    cursor: usize,
) -> Option<LineEdit> {
    let trigger_start = line[..cursor].rfind(&config.trigger)?;
    let token_start = trigger_start + config.trigger.len();
    let token = &line[token_start..cursor];
    if token.is_empty() || !can_replace_trigger_with_config(config, path, line, trigger_start, token)
    {
        return None;
    }
    Some(LineEdit {
        start: trigger_start,
        end: token_start,
        text: config.marker.clone(),
    })
}

/// The canonical form of a number-only shorthand the just-typed character has
/// just terminated (§FS-lsp.1.4, §FS-check.1.2).
///
/// Returns `None` for every case that must not stall typing: no marker, a full ID,
/// a shorthand naming zero or several declarations, or a keystroke that could
/// still be extending the token. The resulting `§FS-042` then earns the
/// §FS-check.3.13 diagnostic, which names the problem in the editor instead.
#[allow(clippy::too_many_arguments)]
fn shorthand_expansion_edit(
    config: &Config,
    path: &Path,
    text: &str,
    line_index: usize,
    line: &str,
    cursor: usize,
    declarations: &[DeclaredId<'_>],
) -> Option<LineEdit> {
    if !config.grammar.has_shorthand() || config.marker.is_empty() {
        return None;
    }
    // The keystroke that fires this is the one that ended the token; anything
    // that could still be part of an ID means the author is mid-word.
    let typed = line[..cursor].chars().next_back()?;
    if config.grammar.id_token_continues_with(typed) {
        return None;
    }
    let token_end = cursor - typed.len_utf8();
    let marker_start = line[..token_end].rfind(&config.marker)?;
    let token_start = marker_start + config.marker.len();
    // §FS-fmt.2.3: the contexts the bulk pass refuses are the contexts the live
    // transform refuses, so a citation illustrated in inline code stays as typed.
    let is_md = path.extension().and_then(|ext| ext.to_str()) == Some("md");
    if never_rewrite_context(line, is_md, marker_start)
        || !line_is_rewritable(config, text, line_index, is_md)
    {
        return None;
    }
    // Scoped to the edited file's own project — see `DeclaredId`. Collected only
    // now, after every cheap gate above has passed, so an ordinary keystroke never
    // walks the declaration list at all (§GOAL-fast-feedback).
    let in_project = declarations_under_root(declarations, &config.root);
    let text = shorthand_token_expansion(config, &line[token_start..token_end], &in_project)?;
    Some(LineEdit {
        start: token_start,
        end: token_end,
        text,
    })
}
