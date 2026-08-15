// The declaration-side title hover (§FS-lsp.1.2): which citations belong to a
// title, how many sites and files that is, and the exact Markdown body the LSP
// hands the editor.
//
// Split out of `api.rs` for the reason `on_type.rs` was: the public items below
// are part of the embedding contract §AR-core-module-layout.2 keeps there, but
// what they carry is a behavior with its own invariant — one definition of "is
// cited by this title", shared by the hover count and the reference list so the
// two can never disagree (§FS-lsp.1.3.1) — and that invariant is what a reader
// comes here for.
//
// File-level prose, so `//` rather than `///` — see the note in `shorthand.rs`.

/// How much of the tree leans on one declaration-side title: citation sites and
/// the distinct files those sites live in (§FS-lsp.1.2).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LspUsage {
    pub sites: usize,
    pub files: usize,
}

/// Whether a citation belongs to the declaration-side title `title_query_id`:
/// the title's own ID, or one of its deeper sections (§FS-lsp.1.3.1).
///
/// Both query IDs are already namespace-qualified by the snapshot, so a
/// member's own `§<ID>` and a sibling's `§<alias>/<ID>` compare equal here for
/// the same declaration — which is what makes the count agree with `grund refs`
/// across a workspace (§FS-lsp.1.2, §FS-workspace.8.2).
pub fn citation_under_title(
    title_query_id: &str,
    citation_query_id: &str,
    section_separator: &str,
) -> bool {
    citation_query_id == title_query_id
        || citation_query_id
            .strip_prefix(title_query_id)
            .is_some_and(|tail| tail.starts_with(section_separator))
}

impl LspSnapshot {
    /// Every citation site that belongs to the declaration-side title
    /// `query_id`, in snapshot order — the set `textDocument/references`
    /// returns for that title (§FS-lsp.1.3.1), which on a whole-ID title is by
    /// definition the set `grund refs <ID>` reports (§FS-refs.2).
    pub fn title_citations(&self, query_id: &str, section_separator: &str) -> Vec<&LspCitation> {
        self.citations
            .iter()
            .filter(|citation| {
                citation_under_title(query_id, &citation.query_id, section_separator)
            })
            .collect()
    }

    /// The counts §FS-lsp.1.2 shows on a declaration-side title, over exactly
    /// the sites `title_citations` lists. Read from the snapshot rather than
    /// from a fresh `refs` query, which would re-scan the tree per hover
    /// (§AR-lsp.5).
    pub fn title_usage(&self, query_id: &str, section_separator: &str) -> LspUsage {
        let sites = self.title_citations(query_id, section_separator);
        LspUsage {
            sites: sites.len(),
            files: sites
                .iter()
                .map(|citation| citation.path.as_path())
                .collect::<BTreeSet<&Path>>()
                .len(),
        }
    }
}

/// The declaration-title hover body (§FS-lsp.1.2): the whole title as inline
/// code, then the usage clause. One line, and the same bytes for the same
/// snapshot — the wording lives here rather than in the transport so a second
/// frontend cannot re-word it (§FS-lsp.4).
pub fn lsp_title_hover_body(title: &str, usage: LspUsage) -> String {
    format!("`{}` — {}", title.replace('`', "\\`"), usage_clause(usage))
}

/// `cited at <n> site(s) across <m> file(s)`, or `not cited` at zero
/// (§FS-lsp.1.2). Only the two nouns inflect: the preposition is `across` at
/// every count, so a skimmed hover changes only in its digits.
fn usage_clause(usage: LspUsage) -> String {
    if usage.sites == 0 {
        return "not cited".to_string();
    }
    format!(
        "cited at {} site{} across {} file{}",
        usage.sites,
        plural_s(usage.sites),
        usage.files,
        plural_s(usage.files)
    )
}

fn plural_s(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}
