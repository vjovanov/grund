/// Slugify a heading into a fragment anchor, dispatching on the configured
/// `[fmt.cross_refs] anchor_format` profile (github / gitlab / mkdocs / pandoc) —
/// §FS-fmt.6.7, §DF-md-link-anchor-strategy.
fn anchor_slug(text: &str, profile: &str) -> String {
    match profile {
        "pandoc" => anchor_slug_pandoc(text),
        "mkdocs" => anchor_slug_mkdocs(text),
        "gitlab" => anchor_slug_gitlab(text),
        _ => anchor_slug_github(text),
    }
}

/// Reproduce GitHub's `github-slugger` byte-for-byte: lowercase the text, delete
/// every character that is not a letter, digit, `_`, or `-` (each deletion in
/// place, so the neighbours close up), then turn each remaining space into one
/// `-`. It does **not** collapse runs of `-` and does **not** trim trailing ones —
/// `## A — B` → `#a--b`, `` ## 6. Watch mode (`--watch`) `` → `#6-watch-mode---watch`.
/// Matching that exactly is the whole point of the `github` profile: the emitted
/// `#fragment` navigates only if it is the slug GitHub itself renders
/// (§DF-github-anchor-fidelity, correcting the "collapse consecutive `-`" wording
/// in §DF-md-link-anchor-strategy.2.3).
fn anchor_slug_github(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars().flat_map(char::to_lowercase) {
        if ch.is_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch);
        } else if ch == ' ' {
            out.push('-');
        }
        // anything else (`.`, brackets, backticks, em dash, tabs, …) is dropped
    }
    out
}

fn anchor_slug_gitlab(text: &str) -> String {
    // "Similar to GitHub with minor Unicode-handling differences"
    // (§DF-md-link-anchor-strategy.2.3); identical for the ASCII headings grund's own
    // specs use, so it rides the github slugger (§DF-github-anchor-fidelity).
    anchor_slug_github(text)
}

// Python-Markdown's TOC slugger: lowercase, drop everything that isn't a word
// char, whitespace, or `-`, then collapse each run of whitespace-and-`-` to one
// `-` (`re.sub(r'[-\s]+', sep, value)`). The keep-set includes `-`, unlike a naive
// "alnum + `_`" filter — `# FS-1-x: Y` slugs to `#fs-1-x-y`, not `#fs1x-y`.
fn anchor_slug_mkdocs(text: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in text.nfkd() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() || lower == '_' {
            out.push(lower);
            last_dash = false;
        } else if (lower.is_ascii_whitespace() || lower == '-') && !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

fn anchor_slug_pandoc(text: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in text.nfkd() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() || lower == '_' || lower == '-' || lower == '.' {
            out.push(lower);
            last_dash = lower == '-';
        } else if lower.is_ascii_whitespace() && !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}
