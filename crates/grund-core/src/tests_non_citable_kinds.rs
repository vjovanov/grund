/// Test module: kinds that declare no IDs (§FS-config.3.4.1) — the `citable`
/// key, the rules a non-citable home is subject to instead of the declaration
/// rules, and the two selectors that refuse one. The `kind` / `prefix` rename
/// (§FS-config.3.4.6) lives here too: it is the same change seen from the
/// config file, and a reader chasing either one wants both.
#[cfg(test)]
mod tests_non_citable_kinds {
    use super::tests_support::*;
    use super::*;

    /// A repo whose `skills/` is a non-citable home, plus the `FS` its rules
    /// point at. `[scan] include` deliberately does **not** name `skills` — the
    /// home is what puts it in scope (§FS-config.3.5).
    fn skills_repo(name: &str, citations: &str, skill_body: &str) -> PathBuf {
        let root = test_root(name);
        write(
            &root.join("grund.toml"),
            &format!(
                "grund_config_version = 1\n\n\
                 [[kinds]]\nkind = \"FS\"\nfolder = \"docs/specs\"\nindex = false\n\n\
                 [[kinds]]\nkind = \"AR\"\nfolder = \"docs/architecture\"\nindex = false\n\n\
                 [[kinds]]\nkind = \"skill\"\nfolder = \"skills\"\ncitable = false\n\
                 title = \"Agent skills\"\n\n\
                 [scan]\ninclude = [\"docs\"]\n\n{citations}"
            ),
        );
        write(
            &root.join("docs/specs/FS-001-login.md"),
            "# FS-001-login: A user logs in\n\nBody.\n",
        );
        write(
            &root.join("docs/architecture/AR-001-bus.md"),
            "# AR-001-bus: The bus\n\nBody.\n",
        );
        write(&root.join("skills/review/SKILL.md"), skill_body);
        root
    }

    /// §FS-config.3.5: a configured home is walked whether or not `include`
    /// names it — otherwise the citations in it would be *invisible* rather than
    /// dangling, which is silence where a finding belongs.
    #[test]
    fn a_kind_home_outside_include_is_still_walked() {
        let run = check_run(
            &skills_repo(
                "a_kind_home_outside_include_is_still_walked",
                "",
                "# Review skill\n\nSee §FS-999-ghost.\n",
            ),
            false,
        );
        assert!(
            codes(&run).contains(&"dangling".to_string()),
            "the home's own citations are checked: {:?}",
            codes(&run)
        );
    }

    /// §FS-check.3.7: the home admits no declaration, and the finding names the
    /// place rather than a kind the author could have written instead.
    #[test]
    fn a_declaration_in_a_non_citable_home_is_misplaced() {
        let run = check_run(
            &skills_repo(
                "a_declaration_in_a_non_citable_home_is_misplaced",
                "",
                "# FS-002-review: Review\n\nSee §FS-001-login.\n",
            ),
            false,
        );
        assert_eq!(
            only(&run, "misplaced-declaration").message,
            "FS-002-review must not be declared in skills/ (not a citable home)",
        );
    }

    /// §FS-check.3.11: the obligation unit is a *file* in the home — Markdown
    /// included, which is where `code`'s per-file rule stops — and the finding
    /// names the home, because the unit has no ID to print.
    #[test]
    fn an_obligation_fires_per_file_on_markdown_in_the_home() {
        let run = check_run(
            &skills_repo(
                "an_obligation_fires_per_file_on_markdown_in_the_home",
                "[citations]\n[citations.skill]\nmust = [\"FS\"]\n",
                "# Review skill\n\nSee §AR-001-bus.\n",
            ),
            false,
        );
        let finding = only(&run, "missing-citation");
        assert_eq!(finding.message, "skills/ must cite FS (citation direction)");
        assert!(
            finding
                .path
                .as_ref()
                .is_some_and(|path| path.ends_with("skills/review/SKILL.md")),
            "anchored at the file that is the unit: {:?}",
            finding.path
        );
    }

    /// §FS-check.3.11: the same rule is satisfied by one citation in the file —
    /// there is no declaration to put it in.
    #[test]
    fn an_obligation_is_satisfied_by_a_citation_anywhere_in_the_file() {
        let run = check_run(
            &skills_repo(
                "an_obligation_is_satisfied_by_a_citation_anywhere_in_the_file",
                "[citations]\n[citations.skill]\nmust = [\"FS\"]\n",
                "# Review skill\n\nSee §FS-001-login.\n",
            ),
            false,
        );
        assert!(
            !codes(&run).contains(&"missing-citation".to_string()),
            "a cited FS satisfies it: {:?}",
            codes(&run)
        );
    }

    /// §FS-check.3.12: a prohibition names the place too.
    #[test]
    fn a_prohibition_names_the_home() {
        let root = skills_repo(
            "a_prohibition_names_the_home",
            "[citations]\n[citations.skill]\nmust-not = [\"FS\"]\n",
            "# Review skill\n\nSee §FS-001-login.\n",
        );
        let run = check_run(&root, false);
        assert_eq!(
            only(&run, "forbidden-citation").message,
            "skills/ must not cite FS (citation direction)",
        );
    }

    /// §FS-check.3.6: `require_grounding` reaches Markdown inside a non-citable
    /// home, and only there — the exemption is about documents, and this home is
    /// one the maintainer declared matters.
    #[test]
    fn require_grounding_reaches_markdown_in_a_non_citable_home() {
        let root = skills_repo(
            "require_grounding_reaches_markdown_in_a_non_citable_home",
            "",
            "# Review skill\n\nNo citation at all.\n",
        );
        let config_path = root.join("grund.toml");
        let config = std::fs::read_to_string(&config_path).expect("read config");
        write(
            &config_path,
            &config.replace(
                "grund_config_version = 1\n",
                "grund_config_version = 1\n\n[reference]\nrequire_grounding = true\n",
            ),
        );
        let run = check_run(&root, false);
        let finding = only(&run, "ungrounded");
        assert_eq!(
            finding.message,
            "ungrounded file in kind home skills/: no § citation to a declared ID",
        );
        assert!(
            !run.report.errors.iter().any(|diagnostic| {
                diagnostic.code == "ungrounded"
                    && diagnostic
                        .path
                        .as_ref()
                        .is_some_and(|path| path.ends_with("docs/specs/FS-001-login.md"))
            }),
            "Markdown outside such a home keeps its exemption: {:?}",
            codes(&run)
        );
    }

    /// §FS-config.3.4.1 / §FS-init.2.3.4.4: the generated block names the kind by
    /// its place, and leaves it out of the ID vocabulary.
    #[test]
    fn the_generated_block_names_a_non_citable_kind_by_place() {
        let root = skills_repo(
            "the_generated_block_names_a_non_citable_kind_by_place",
            "[citations]\n[citations.skill]\nmust = [\"FS\"]\n",
            "# Review skill\n\nSee §FS-001-login.\n",
        );
        let config = load_config(&root).expect("load config");
        let block = render_agents_append_block("demo", &config, &root, true, ConversationSurface::Plain);
        assert!(
            block.contains("- [skills/](skills): Agent skills"),
            "map row by place: {block}"
        );
        assert!(
            block.contains("- **skills/** must cite FS."),
            "directions row by place: {block}"
        );
        assert!(
            block.contains("KIND ∈ {FS, AR}"),
            "the vocabulary line lists citable kinds only: {block}"
        );
    }

    /// §FS-config.3.4.5: prefix-freedom is about tokenization, so it stops where
    /// tokenization does. A name that never appears in an ID has no prefix.
    #[test]
    fn a_non_citable_name_may_prefix_a_citable_one() {
        let root = test_root("a_non_citable_name_may_prefix_a_citable_one");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n\
             [[kinds]]\nkind = \"SKI\"\nfolder = \"docs/ski\"\nindex = false\n\n\
             [[kinds]]\nkind = \"SKILL\"\nfolder = \"skills\"\ncitable = false\n",
        );
        assert!(
            load_config(&root).is_ok(),
            "a non-citable name never tokenizes, so it collides with nothing"
        );
    }

    /// §FS-config.3.4.5: two citable names still collide, and every name is still
    /// unique.
    #[test]
    fn citable_names_still_collide_and_names_are_unique() {
        let root = test_root("citable_names_still_collide_and_names_are_unique");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n\
             [[kinds]]\nkind = \"DA\"\nfolder = \"docs/da\"\n\n\
             [[kinds]]\nkind = \"DAT\"\nfolder = \"docs/dat\"\n",
        );
        assert!(
            config_error(&root).contains("collide"),
            "two citable prefixes are still ambiguous"
        );
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n\
             [[kinds]]\nkind = \"FS\"\nfolder = \"docs/a\"\n\n\
             [[kinds]]\nkind = \"FS\"\nfolder = \"docs/b\"\n",
        );
        assert!(
            config_error(&root)
                .contains("kind `FS` is declared twice"),
            "a name is the handle [citations.*] keys on, so it answers for one row"
        );
    }

    /// §FS-config.3.4.1: the keys a non-citable kind may not combine — an index
    /// lists declarations it will never have, and a place with no home is `code`.
    #[test]
    fn a_non_citable_kind_needs_a_home_and_takes_no_index() {
        let root = test_root("a_non_citable_kind_needs_a_home_and_takes_no_index");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n\
             [[kinds]]\nkind = \"skill\"\nfolder = \"skills\"\ncitable = false\nindex = \"README.md\"\n",
        );
        assert!(
            config_error(&root)
                .contains("a non-citable kind declares nothing to index"),
            "the key is a statement about a set that can never be non-empty"
        );
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n[[kinds]]\nkind = \"skill\"\ncitable = false\n",
        );
        assert!(
            config_error(&root)
                .contains("the homeless one is `code`"),
            "a non-citable kind is a place, and the placeless one already has a name"
        );
    }

    /// §FS-config.3.9.5: a non-citable kind cites and is never cited, and the
    /// message says which of the two mistakes it is.
    #[test]
    fn a_non_citable_kind_is_not_a_citation_target() {
        let root = test_root("a_non_citable_kind_is_not_a_citation_target");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n\
             [[kinds]]\nkind = \"FS\"\nfolder = \"docs/specs\"\nindex = false\n\n\
             [[kinds]]\nkind = \"skill\"\nfolder = \"skills\"\ncitable = false\n\n\
             [citations]\n[citations.FS]\nmust = [\"skill\"]\n",
        );
        assert!(
            config_error(&root)
                .contains("names a non-citable target kind `skill`"),
            "a configured kind with no IDs is not an unknown kind"
        );
    }

    /// §FS-config.3.4.6: the old spelling loads, and the run says where it is and
    /// when it stops working.
    #[test]
    fn the_deprecated_prefix_key_loads_and_warns() {
        let root = test_root("the_deprecated_prefix_key_loads_and_warns");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n\
             [[kinds]]\nprefix = \"FS\"\nfolder = \"docs/specs\"\nindex = false\n\n\
             [[kinds]]\nprefix = \"AR\"\nfolder = \"docs/architecture\"\nindex = false\n",
        );
        let config = load_config(&root).expect("the old spelling still loads");
        assert_eq!(config.kinds[0].kind, "FS");
        let warnings = config_warnings(&config);
        assert_eq!(
            warnings,
            vec![format!(
                "grund.toml:4: [[kinds]] `prefix` is deprecated — rename it to `kind`; \
                 `prefix` stops loading in grund {KIND_PREFIX_KEY_REMOVAL_RELEASE}"
            )],
            "one warning per config, anchored at the first entry that uses it"
        );
    }

    /// §FS-config.3.4.6: they name the same thing, so a file that spells both has
    /// no answer if the two disagree.
    #[test]
    fn kind_and_prefix_together_are_a_config_error() {
        let root = test_root("kind_and_prefix_together_are_a_config_error");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n[[kinds]]\nkind = \"FS\"\nprefix = \"FS\"\nfolder = \"docs\"\n",
        );
        assert!(
            config_error(&root)
                .contains("sets both `kind` and `prefix`"),
            "one entry, one name"
        );
    }

    /// §REQ-backwards-compatibility.2: the deprecation names a release, and a
    /// named release that has already passed is a promise grund broke. Held
    /// ahead of the running version so the window cannot expire unnoticed.
    #[test]
    fn the_prefix_deprecation_release_is_still_ahead() {
        let parse = |version: &str| {
            version
                .split('.')
                .map(|part| part.parse::<u32>().unwrap_or(0))
                .collect::<Vec<_>>()
        };
        assert!(
            parse(env!("CARGO_PKG_VERSION")) < parse(KIND_PREFIX_KEY_REMOVAL_RELEASE),
            "this tree is {}, which has reached the release §FS-config.3.4.6 promised \
             `prefix` would stop loading in ({KIND_PREFIX_KEY_REMOVAL_RELEASE}). Remove the \
             key rather than moving the date.",
            env!("CARGO_PKG_VERSION")
        );
    }

    /// The message a config this repo cannot load fails with. `Config` carries no
    /// `Debug`, so the error is unwrapped by matching rather than by `expect_err`.
    fn config_error(root: &Path) -> String {
        match load_config(root) {
            Ok(_) => panic!("expected the config to be rejected"),
            Err(error) => format!("{error:#}"),
        }
    }

    /// §FS-list.1 / §FS-id.1: both selectors take a citable kind, and a
    /// configured non-citable one is refused with the reason rather than as a
    /// typo — it would select nothing, every time.
    #[test]
    fn the_kind_selectors_refuse_a_non_citable_kind() {
        let root = skills_repo(
            "the_kind_selectors_refuse_a_non_citable_kind",
            "",
            "# Review skill\n\nSee §FS-001-login.\n",
        );
        let outcome = propose_id(
            "skill",
            "Review",
            IdOpts {
                path: root.clone(),
                ..IdOpts::default()
            },
        )
        .expect("propose");
        match outcome {
            IdProposalOutcome::UnknownKind { headline, known } => {
                assert_eq!(
                    headline,
                    "kind `skill` declares no IDs — skills/ is not a citable home"
                );
                assert_eq!(known, vec!["FS", "AR"], "the citable kinds, and only those");
            }
            other => panic!("expected a refusal, got {other:?}"),
        }
    }
}
