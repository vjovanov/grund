/// Test module: a path baked *into* a message is spelled from the report root,
/// not from the project the finding came out of (§FS-workspace.8.1,
/// §FS-config.3.6). The printer already renders `Diagnostic.path` that way; a
/// site named inside the message text has to agree, or one workspace line has
/// two halves relative to two different roots and an editor can follow only the
/// first of them.
#[cfg(test)]
mod tests_workspace_message_paths {
    use super::*;
    use super::tests_support::*;

    fn alpha() -> Id {
        Id {
            kind: "FS".to_string(),
            num: Some(1),
            slug: Some("alpha".to_string()),
        }
    }

    fn router() -> Id {
        Id {
            kind: "AR".to_string(),
            num: Some(1),
            slug: Some("router".to_string()),
        }
    }

    /// A workspace whose one member is `apps/api`, holding `files`. Returns the
    /// root config the report renders against and the member config the project
    /// is checked with — the pair every case here needs to keep apart.
    fn member_workspace(name: &str, files: &[(&str, &str)]) -> (Config, Config, Findings) {
        let root = test_root(name);
        let member = root.join("apps/api");
        for (path, text) in files {
            write(&member.join(path), text);
        }
        let mut root_config = legacy_fs_folder_config(root);
        root_config.workspace_boundary_roots = vec![canonical_test_path(&member)];
        let api_config = legacy_fs_folder_config(member);
        let (api_findings, _) =
            scan_tree(&api_config, Some(&api_config.root), true).expect("scan member");
        (root_config, api_config, api_findings)
    }

    const TWO_HOMES: [(&str, &str); 2] = [
        (
            "docs/functional-spec/FS-001-alpha.md",
            "# FS-001-alpha: Alpha\n\nLead.\n",
        ),
        (
            "docs/functional-spec/FS-001-alpha-again.md",
            "# FS-001-alpha: Alpha again\n\nLead.\n",
        ),
    ];

    /// §FS-check.3.3: the duplicate-declaration finding anchors at the first site
    /// and names the rest in its message. Both halves belong to one report, so
    /// both are spelled from the root that report is rendered against.
    #[test]
    fn duplicate_declaration_names_the_other_home_from_the_workspace_root() {
        let (root_config, api_config, api_findings) = member_workspace(
            "workspace_message_paths_duplicate_declaration",
            &TWO_HOMES,
        );
        let workspace = BTreeMap::from([(
            "api".to_string(),
            WorkspaceCheckTarget {
                findings: &api_findings,
                config: &api_config,
            },
        )]);

        let report =
            check_with_workspace(&api_findings, &api_config, &root_config, Some("api"), &workspace);

        assert_eq!(
            located_diagnostics(
                &root_config,
                report.errors.iter().filter(|error| error.code == "duplicate")
            ),
            vec![
                "apps/api/docs/functional-spec/FS-001-alpha-again.md:1: duplicate declaration of \
                 FS-001-alpha (also declared at \
                 apps/api/docs/functional-spec/FS-001-alpha.md:1)"
            ],
            "§FS-workspace.8.1: the anchor and the site it names come from one root"
        );
    }

    /// §FS-show.2.2.1: the same rule for `show`'s refusal. Its sites are the only
    /// paths a failed query prints, and a member-relative one sends the reader to
    /// a file the workspace root does not have.
    #[test]
    fn ambiguous_id_lists_its_sites_from_the_workspace_root() {
        let (root_config, api_config, api_findings) =
            member_workspace("workspace_message_paths_ambiguous_id", &TWO_HOMES);

        let Err(err) = show_declaration(
            &api_config,
            &root_config,
            &api_findings,
            &alpha(),
            None,
            ShowRenderMode::Default,
            false,
        ) else {
            panic!("§FS-show.2.2.1: two homes must refuse rather than pick one");
        };

        assert_eq!(
            err.to_string(),
            "ambiguous ID: FS-001-alpha (declared at \
             apps/api/docs/functional-spec/FS-001-alpha-again.md:1, \
             apps/api/docs/functional-spec/FS-001-alpha.md:1)",
            "§FS-workspace.8.1: `show` spells its sites from the report root"
        );
    }

    /// §FS-errors.5: the typed carrier's `sites` are the same `path:line` pairs
    /// the prose above just pinned, spelled from the same report root — no
    /// consumer of the JSON diagnostic has to re-derive them from `message`.
    #[test]
    fn ambiguous_id_carries_its_sites_from_the_workspace_root() {
        let (root_config, api_config, api_findings) =
            member_workspace("workspace_message_paths_ambiguous_id_sites", &TWO_HOMES);

        let Err(err) = show_declaration(
            &api_config,
            &root_config,
            &api_findings,
            &alpha(),
            None,
            ShowRenderMode::Default,
            false,
        ) else {
            panic!("§FS-show.2.2.1: two homes must refuse rather than pick one");
        };

        let carrier = err
            .downcast_ref::<ShowQueryError>()
            .expect("§FS-errors.5: the refusal carries a typed error with sites");
        assert_eq!(
            carrier.sites,
            vec![
                FindingSite {
                    path: "apps/api/docs/functional-spec/FS-001-alpha-again.md".to_string(),
                    line: 1,
                },
                FindingSite {
                    path: "apps/api/docs/functional-spec/FS-001-alpha.md".to_string(),
                    line: 1,
                },
            ],
            "§FS-workspace.8.1: sites are spelled from the report root, in the message's order"
        );
    }

    /// The third message the same function raises: a stub whose target is missing
    /// prints where that stub is. The target itself stays verbatim — that text is
    /// the user's own link quoted back, not a path `grund` resolved.
    #[test]
    fn broken_stub_names_the_stub_from_the_workspace_root() {
        let (root_config, api_config, api_findings) = member_workspace(
            "workspace_message_paths_broken_stub",
            &[(
                "docs/architecture/AR-001-router.md",
                "# AR-001-router: [router](src/router.rs)\n",
            )],
        );

        let Err(err) = show_declaration(
            &api_config,
            &root_config,
            &api_findings,
            &router(),
            None,
            ShowRenderMode::Default,
            false,
        ) else {
            panic!("a stub pointing at nothing must refuse");
        };

        assert_eq!(
            err.to_string(),
            "broken stub: AR-001-router (stub at \
             apps/api/docs/architecture/AR-001-router.md:1 points at src/router.rs, \
             which does not exist)",
            "§FS-workspace.8.1: the stub's own path is a report path"
        );
    }
}
