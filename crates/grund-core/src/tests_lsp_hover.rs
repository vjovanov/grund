/// Test module: the declaration-side title hover — usage counts and the exact
/// body bytes an editor shows (§FS-lsp.1.2)
#[cfg(test)]
mod tests_lsp_hover {
    use super::*;
    use super::tests_support::*;

    /// A snapshot of `root`, with nothing open in the editor.
    fn snapshot_of(root: &Path) -> LspSnapshot {
        lsp_snapshot(LspSnapshotOpts {
            path: root.to_path_buf(),
            path_provided: true,
            open_documents: BTreeMap::new(),
        })
        .expect("lsp snapshot")
    }

    /// The hover body §FS-lsp.1.2 specifies for the declaration heading or
    /// numbered section heading whose query ID is `query_id`.
    fn title_hover_body(snapshot: &LspSnapshot, query_id: &str) -> String {
        let decl = snapshot
            .declarations
            .iter()
            .chain(snapshot.sections.iter())
            .find(|decl| decl.query_id == query_id)
            .expect("declaration or section title");
        let usage = snapshot.title_usage(&decl.query_id, &decl.section_separator);
        lsp_title_hover_body(&decl.text, usage)
    }

    /// The same, for the inline-spec stub title that points at `query_id`'s
    /// source home — a separate lookup because the stub and the inline
    /// declaration it points at are two titles carrying one ID.
    fn stub_hover_body(snapshot: &LspSnapshot, query_id: &str) -> String {
        let stub = snapshot
            .stubs
            .iter()
            .find(|stub| stub.query_id == query_id)
            .expect("stub title");
        let usage = snapshot.title_usage(&stub.query_id, &stub.section_separator);
        lsp_title_hover_body(&stub.text, usage)
    }

    /// The `grund refs <ID>` answer for the same tree, as (sites, files) — the
    /// CLI side of the parity §FS-lsp.1.2 claims for a whole-ID title.
    fn refs_counts(root: &Path, id: &str, section: Option<&str>) -> (usize, usize) {
        let output = refs(RefsOpts {
            path: root.to_path_buf(),
            path_provided: true,
            id: id.to_string(),
            section: section.map(str::to_string),
        })
        .expect("public refs api");
        let files = output
            .hits
            .iter()
            .map(|hit| hit.path.clone())
            .collect::<BTreeSet<String>>();
        (output.hits.len(), files.len())
    }

    fn plural_fixture(root: &Path) {
        write(&root.join(".agents/grund.toml"), "grund_config_version = 1\n");
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n\nLead.\n\n## 1. Detail\nMore.\n\n### 1.1 Deeper\nDeeper.\n",
        );
        write(
            &root.join("docs/functional-spec/FS-002-beta.md"),
            "# FS-002-beta: Beta\n\nLead.\n",
        );
        write(
            &root.join("docs/functional-spec/FS-003-gamma.md"),
            "# FS-003-gamma: Gamma\n\nLead.\n",
        );
        // Three sites across two files for alpha, one site for beta, none for
        // gamma — the three count shapes §FS-lsp.1.2 words differently.
        write(
            &root.join("src/first.rs"),
            "//! §FS-001-alpha\n/// §FS-001-alpha.1\npub fn first() {}\n",
        );
        write(
            &root.join("src/second.rs"),
            "//! §FS-001-alpha.1.1\n/// §FS-002-beta\npub fn second() {}\n",
        );
    }

    /// §FS-lsp.1.3.1: a declaration-side title claims its own ID and its
    /// sections, under whichever `[id] section_separator` the project
    /// configures — and never a longer ID that merely starts the same way.
    /// Moved here with the matcher, which used to live in `grund-lsp`.
    #[test]
    fn a_title_claims_its_sections_and_no_longer_id() {
        assert!(citation_under_title("FS-lsp", "FS-lsp", "."));
        assert!(citation_under_title("FS-lsp", "FS-lsp.1", "."));
        assert!(citation_under_title("FS-lsp", "FS-lsp/1", "/"));
        assert!(!citation_under_title("FS-lsp", "FS-lsp-extra.1", "."));
        assert!(!citation_under_title("FS-lsp.1", "FS-lsp.11", "."));
    }

    /// §FS-lsp.1.2: the body is the title as inline code, an em dash, and the
    /// usage clause — plural nouns above one, `across` at every count.
    #[test]
    fn declaration_title_hover_counts_sites_and_files() {
        let root = test_root("declaration_title_hover_counts_sites_and_files");
        plural_fixture(&root);
        let snapshot = snapshot_of(&root);

        assert_eq!(
            title_hover_body(&snapshot, "FS-001-alpha"),
            "`FS-001-alpha: Alpha` — cited at 3 sites across 2 files"
        );
    }

    /// §FS-lsp.1.2: at one, both nouns lose the `s` and the preposition does
    /// not change — `cited at 1 site across 1 file`.
    #[test]
    fn declaration_title_hover_uses_singular_nouns_at_one() {
        let root = test_root("declaration_title_hover_uses_singular_nouns_at_one");
        plural_fixture(&root);
        let snapshot = snapshot_of(&root);

        assert_eq!(
            title_hover_body(&snapshot, "FS-002-beta"),
            "`FS-002-beta: Beta` — cited at 1 site across 1 file"
        );
    }

    /// §FS-lsp.1.2: an uncited title reads `not cited` — the whole clause is
    /// replaced, and the hover is not suppressed, so the title keeps its
    /// whole-title hover range and the answer is never mistaken for silence.
    #[test]
    fn uncited_declaration_title_hover_reads_not_cited() {
        let root = test_root("uncited_declaration_title_hover_reads_not_cited");
        plural_fixture(&root);
        let snapshot = snapshot_of(&root);

        assert_eq!(
            title_hover_body(&snapshot, "FS-003-gamma"),
            "`FS-003-gamma: Gamma` — not cited"
        );
        // And the unused-declaration warning still carries its own sentence, so
        // the hover restates nothing the diagnostic already says (§FS-check.4.1).
        assert!(
            snapshot
                .report
                .warnings
                .iter()
                .any(|warning| warning.code == "unused"
                    && warning.message == "declared but never cited: FS-003-gamma"),
            "the uncited declaration keeps its own diagnostic: {:?}",
            snapshot.report.warnings
        );
    }

    /// §FS-lsp.1.2: the title goes into the code span verbatim, so a title
    /// carrying backticks is fenced with a run one longer than the longest run
    /// inside it — a backslash does not escape a backtick inside a code span,
    /// and the raw one would close the span early — and padded with one space
    /// at each end when it starts or ends with a backtick.
    #[test]
    fn title_hover_body_fences_a_title_that_carries_backticks() {
        let usage = LspUsage { sites: 1, files: 1 };
        let clause = "cited at 1 site across 1 file";

        // No backtick in the title: the plain single-backtick span, unchanged.
        assert_eq!(
            lsp_title_hover_body("FS-001-alpha: Alpha", usage),
            format!("`FS-001-alpha: Alpha` — {clause}")
        );
        // One run of one, as most section headings that name a flag carry.
        assert_eq!(
            lsp_title_hover_body("2.1.2 Section map (`--toc`)", usage),
            format!("``2.1.2 Section map (`--toc`)`` — {clause}")
        );
        // A run of two needs three, not two: the fence must not appear inside.
        assert_eq!(
            lsp_title_hover_body("1. Spans written ``like this``", usage),
            format!("``` 1. Spans written ``like this`` ``` — {clause}")
        );
        // Starting and ending with a backtick: one space at each end, which
        // CommonMark strips back off when it renders the span.
        assert_eq!(
            lsp_title_hover_body("`--toc` and `--full`", usage),
            format!("`` `--toc` and `--full` `` — {clause}")
        );
    }

    /// §FS-lsp.1.2: the same rule end to end — a declaration heading and a
    /// section heading whose titles carry backticks hover with the title intact.
    #[test]
    fn title_hover_fences_backticks_read_off_a_real_tree() {
        let root = test_root("title_hover_fences_backticks_read_off_a_real_tree");
        write(&root.join(".agents/grund.toml"), "grund_config_version = 1\n");
        write(
            &root.join("docs/functional-spec/FS-004-delta.md"),
            "# FS-004-delta: The `--toc` flag\n\nLead.\n\n## 1. Section map (`--toc`)\nMore.\n",
        );
        write(&root.join("src/user.rs"), "//! §FS-004-delta.1\n");
        let snapshot = snapshot_of(&root);

        assert_eq!(
            title_hover_body(&snapshot, "FS-004-delta"),
            "``FS-004-delta: The `--toc` flag`` — cited at 1 site across 1 file"
        );
        assert_eq!(
            title_hover_body(&snapshot, "FS-004-delta.1"),
            "``1. Section map (`--toc`)`` — cited at 1 site across 1 file"
        );
    }

    /// §FS-lsp.1.2: a numbered section heading counts the section-scoped set
    /// §FS-lsp.1.3.1 defines — `§<ID>.<section>` and deeper — which is wider
    /// than the exact-coordinate filter `grund refs <ID> --section <s>` applies.
    #[test]
    fn section_title_hover_counts_the_section_subtree() {
        let root = test_root("section_title_hover_counts_the_section_subtree");
        plural_fixture(&root);
        let snapshot = snapshot_of(&root);

        assert_eq!(
            title_hover_body(&snapshot, "FS-001-alpha.1"),
            "`1. Detail` — cited at 2 sites across 2 files"
        );
        assert_eq!(
            title_hover_body(&snapshot, "FS-001-alpha.1.1"),
            "`1.1 Deeper` — cited at 1 site across 1 file"
        );
        // The deliberate divergence, pinned so it cannot drift silently: the
        // CLI flag keeps only citations whose coordinate is exactly `1`.
        assert_eq!(refs_counts(&root, "FS-001-alpha", Some("1")), (1, 1));
    }

    /// §FS-lsp.1.2: an inline-spec stub title is a whole-ID title, so it counts
    /// what the declaration it points at is cited by, not what the stub file is.
    #[test]
    fn stub_title_hover_counts_the_inline_declarations_citations() {
        let root = test_root("stub_title_hover_counts_the_inline_declarations_citations");
        write(&root.join(".agents/grund.toml"), "grund_config_version = 1\n");
        write(
            &root.join("docs/architecture/AR-001-router.md"),
            "# AR-001-router: [src/router.rs](../../src/router.rs)\n",
        );
        write(
            &root.join("src/router.rs"),
            "/// AR-001-router: Router\npub fn router() {}\n",
        );
        write(&root.join("src/caller.rs"), "//! §AR-001-router\n");
        let snapshot = snapshot_of(&root);

        assert_eq!(
            stub_hover_body(&snapshot, "AR-001-router"),
            "`AR-001-router: [src/router.rs](../../src/router.rs)` — cited at 1 site across 1 file"
        );
        // The inline declaration the stub points at is the same ID, so the
        // source-side title answers with the same counts.
        assert_eq!(
            title_hover_body(&snapshot, "AR-001-router"),
            "`AR-001-router: Router` — cited at 1 site across 1 file"
        );
    }

    /// §FS-lsp.1.2 / §FS-lsp.4: on a whole-ID title the two numbers *are* the
    /// `grund refs <ID>` answer. Held by comparison rather than by claim, since
    /// the LSP counts from the session snapshot and never re-runs the query.
    #[test]
    fn whole_id_title_hover_counts_match_the_refs_query() {
        let root = test_root("whole_id_title_hover_counts_match_the_refs_query");
        plural_fixture(&root);
        let snapshot = snapshot_of(&root);

        for id in ["FS-001-alpha", "FS-002-beta", "FS-003-gamma"] {
            let decl = snapshot
                .declarations
                .iter()
                .find(|decl| decl.query_id == id)
                .expect("declaration");
            let usage = snapshot.title_usage(&decl.query_id, &decl.section_separator);
            assert_eq!(
                (usage.sites, usage.files),
                refs_counts(&root, id, None),
                "hover counts must equal `grund refs {id}`"
            );
        }
    }

    /// §FS-lsp.1.2: in a workspace the count follows `refs` across namespaces —
    /// a member's own `§<ID>` and a sibling's `§<alias>/<ID>` both count for the
    /// member's declaration title.
    #[test]
    fn workspace_title_hover_counts_cross_namespace_citations() {
        let root = test_root("workspace_title_hover_counts_cross_namespace_citations");
        write(
            &root.join(".agents/grund.toml"),
            "grund_config_version = 1\n\n[workspace]\nmembers = [\"apps/api\"]\n",
        );
        write(
            &root.join("apps/api/.agents/grund.toml"),
            "grund_config_version = 1\n",
        );
        write(
            &root.join("apps/api/docs/functional-spec/FS-001-session.md"),
            "# FS-001-session: Session\n\nLead.\n",
        );
        // One local citation inside the member, one qualified from the root.
        write(&root.join("apps/api/src/session.rs"), "//! §FS-001-session\n");
        write(&root.join("src/root.rs"), "//! §api/FS-001-session\n");
        let snapshot = snapshot_of(&root);

        assert_eq!(
            title_hover_body(&snapshot, "api/FS-001-session"),
            "`FS-001-session: Session` — cited at 2 sites across 2 files"
        );
        assert_eq!(refs_counts(&root, "api/FS-001-session", None), (2, 2));
    }

    /// §FS-lsp.4: same tree, same config, same bytes. Two snapshots of one
    /// unchanged tree render every title identically.
    #[test]
    fn title_hover_bodies_are_byte_identical_across_snapshots() {
        let root = test_root("title_hover_bodies_are_byte_identical_across_snapshots");
        plural_fixture(&root);
        let first = snapshot_of(&root);
        let second = snapshot_of(&root);

        for query_id in [
            "FS-001-alpha",
            "FS-001-alpha.1",
            "FS-001-alpha.1.1",
            "FS-002-beta",
            "FS-003-gamma",
        ] {
            assert_eq!(
                title_hover_body(&first, query_id),
                title_hover_body(&second, query_id),
                "hover body for {query_id} must be byte-identical between scans"
            );
        }
    }
}
