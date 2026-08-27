/// Test module: the number-only shorthand as the *consumer* surfaces see it —
/// ID rendering, `grund <ID>` and its ambiguity listing, the LSP snapshot, and
/// the terminal/editor client matchers (§FS-check.1.2,
/// §DF-number-only-citation-shorthand). Split out of `tests_shorthand.rs`,
/// which pins recognition, resolution, and the check error: these cases fail
/// when a surface stops accepting the shorthand, not when the scanner changes
/// its mind about one (§AR-core-module-layout.1).
#[cfg(test)]
mod tests_shorthand_surfaces {
    use super::tests_support::*;
    use super::*;

    // §AR-scanner.2.6: `render_id` reduces a partial `Id` by the same rule the
    // shorthand pattern is derived from, so an unresolved shorthand prints as
    // `FS-042` rather than leaking the raw `{slug}` placeholder into a report.
    #[test]
    fn render_id_prints_a_slugless_id_as_the_shorthand() {
        let config = numbered_config(test_root("render_id_prints_a_slugless_id_as_the_shorthand"));
        let shorthand = Id {
            kind: "FS".into(),
            num: Some(42),
            slug: None,
        };
        assert_eq!(render_id(&config, &shorthand), "FS-042");
        assert_eq!(
            render_id(
                &config,
                &Id {
                    kind: "FS".into(),
                    num: Some(42),
                    slug: Some("user-login".into()),
                }
            ),
            "FS-042-user-login"
        );
    }

    // §FS-check.1.2 / §FS-show.1: a query persists nothing, so the shorthand is
    // simply expanded at the CLI boundary. This is also what makes a clicked
    // `§FS-042` open (§FS-integrations.3.1).
    #[test]
    fn shorthand_resolves_as_a_query_argument() {
        let root = test_root("shorthand_resolves_as_a_query_argument");
        write(
            &root.join("docs/functional-spec/FS-042-user-login.md"),
            "# FS-042-user-login: User login\n\nLead.\n",
        );
        let config = numbered_config(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan");

        let (id, section) = resolve_id_arg("FS-042", &config, &findings).expect("resolve");
        assert_eq!(render_id(&config, &id), "FS-042-user-login");
        assert_eq!(section, None);

        let (id, section) = resolve_id_arg("FS-042.1", &config, &findings).expect("resolve");
        assert_eq!(render_id(&config, &id), "FS-042-user-login");
        assert_eq!(section.as_deref(), Some("1"));

        // A full ID is unaffected, and an unknown shorthand keeps its written
        // form so the caller's own "not found" path names what was asked for.
        let (id, _) = resolve_id_arg("FS-999", &config, &findings).expect("resolve");
        assert_eq!(render_id(&config, &id), "FS-999");
    }

    // §FS-show.2.2.1: an ambiguous shorthand argument is a query failure that
    // lists every candidate rather than picking one.
    #[test]
    fn ambiguous_shorthand_argument_lists_every_candidate() {
        let root = test_root("ambiguous_shorthand_argument_lists_every_candidate");
        write(
            &root.join("docs/functional-spec/FS-042-user-login.md"),
            "# FS-042-user-login: User login\n\nLead.\n",
        );
        write(
            &root.join("docs/functional-spec/FS-042-user-logout.md"),
            "# FS-042-user-logout: User logout\n\nLead.\n",
        );
        let config = numbered_config(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan");

        let err = resolve_id_arg("FS-042", &config, &findings).expect_err("ambiguous");
        assert_eq!(
            format!("{err:#}"),
            "ambiguous ID: FS-042 (matches FS-042-user-login, FS-042-user-logout)"
        );
    }

    /// §FS-lsp.1.4: a shorthand already in the document navigates like any other
    /// citation. The snapshot carries the canonical target while the range stays
    /// the written token, which is what makes hover, go-to-definition,
    /// references, document links, and highlight all work without any of them
    /// knowing the shorthand exists.
    #[test]
    fn lsp_snapshot_navigates_a_shorthand_citation() {
        let root = test_root("lsp_snapshot_navigates_a_shorthand_citation");
        write(
            &root.join(".agents/grund.toml"),
            "grund_config_version = 1\n[id]\nformat = \"{kind}-{number}-{slug}\"\n",
        );
        write(
            &root.join("docs/functional-spec/FS-042-user-login.md"),
            "# FS-042-user-login: User login\n\nLead.\n\n## 1. Inputs\n\nStuff.\n",
        );
        write(&root.join("src/lib.rs"), "//! §FS-042.1\n");
        let snapshot = lsp_snapshot(LspSnapshotOpts {
            path: root.clone(),
            path_provided: true,
            open_documents: BTreeMap::new(),
        })
        .expect("lsp snapshot");

        let citation = snapshot
            .citations
            .iter()
            .find(|citation| citation.display_path == "src/lib.rs")
            .expect("shorthand citation");
        assert_eq!(citation.text, "\u{a7}FS-042.1", "range covers what was typed");
        assert_eq!(citation.query_id, "FS-042-user-login.1");
        assert_eq!(citation.declaration_query_id, "FS-042-user-login");
        assert_eq!(
            citation.target_path.as_deref().map(canonical_test_path),
            Some(canonical_test_path(
                &root.join("docs/functional-spec/FS-042-user-login.md")
            ))
        );
        assert_eq!(citation.target_line, Some(5), "jumps to the cited section");

        // The §FS-check.3.13 finding reaches the editor as a diagnostic.
        assert!(
            snapshot
                .report
                .errors
                .iter()
                .any(|finding| finding.code == "shorthand-citation"),
            "{:?}",
            snapshot.report.errors
        );
    }

    /// §FS-integrations.3.1: the clients need no shorthand matcher of their own —
    /// the shared citation shape already accepts one, in every form. This pins
    /// that, because the spec claims it and six hand-written regexes would
    /// otherwise be free to drift from the engine.
    #[test]
    fn client_matchers_already_accept_the_shorthand() {
        let citation_shape = Regex::new(
            r"[^\w\s]{1,3}(?:[a-z][a-z0-9-]*/)?[A-Z][A-Z0-9]*-[a-z0-9][a-z0-9-]*(?:\.[0-9]+)*",
        )
        .expect("client citation shape");
        for (text, expected) in [
            ("see \u{a7}FS-042 here", "\u{a7}FS-042"),
            ("see \u{a7}FS-042.1 here", "\u{a7}FS-042.1"),
            ("see \u{a7}api/FS-042 here", "\u{a7}api/FS-042"),
            ("see \u{a7}FS-042-user-login here", "\u{a7}FS-042-user-login"),
        ] {
            assert_eq!(
                citation_shape.find(text).map(|found| found.as_str()),
                Some(expected),
                "client matcher must claim {text:?}"
            );
        }
        // The same shape is what every client artifact embeds.
        for artifact in [
            WEZTERM_SNIPPET,
            KITTY_SNIPPET,
            ITERM2_SNIPPET,
            VSCODE_EXTENSION_JS,
        ] {
            assert!(
                artifact.contains("[A-Z][A-Z0-9]*-[a-z0-9][a-z0-9-]*"),
                "client artifact lost the shared citation shape"
            );
        }
    }
}
