/// Test module: `[workspace] optional_members` — the member a repository has
/// declared may be legitimately absent (§FS-workspace.2.2, §FS-workspace.2.2.1,
/// §FS-workspace.2.2.2), and the announcement an absent one earns
/// (§FS-check.4.10).
///
/// What lives here is what member-list expansion decides on its own: whether an
/// entry is accepted at all, what alias it carries, and whether an absent one
/// stops the run. The behaviour of a whole run — the warning's text, the stream
/// it lands on, and the citations it does *not* report — is pinned end to end
/// under `tests/e2e/cases/workspace-optional-member-*`, because it is a property
/// of a report rather than of one function.
#[cfg(test)]
mod tests_workspace_optional_members {
    use super::*;
    use super::tests_support::*;

    /// The root config every case here starts from: a workspace block written by
    /// the caller, and an ID grammar the fixtures' `SPEC-NNN-slug` headings match.
    fn root_config(root: &std::path::Path, workspace_block: &str) {
        write(
            &root.join("grund.toml"),
            &format!(
                "grund_config_version = 1\nproject_name = \"acme\"\n\n{workspace_block}\n\
                 \n[id]\nformat = \"{{kind}}-{{number}}-{{slug}}\"\n"
            ),
        );
    }

    fn member_config(root: &std::path::Path, member: &str, project_name: Option<&str>) {
        let name = project_name
            .map(|name| format!("project_name = \"{name}\"\n"))
            .unwrap_or_default();
        write(
            &root.join(member).join("grund.toml"),
            &format!("grund_config_version = 1\n{name}"),
        );
    }

    fn expand(root: &std::path::Path) -> Result<Vec<String>> {
        let mut config = load_config(root)?;
        Ok(expand_workspace_tree(&mut config)?
            .into_iter()
            .map(|entry| entry.alias)
            .collect())
    }

    /// §FS-workspace.2.2: the ticket's own case. A member the config declares may
    /// be absent, and is, loads instead of failing — the whole point of the key,
    /// and the one behaviour a softened `members` would have given for free and
    /// wrongly.
    #[test]
    fn an_absent_optional_member_does_not_fail_expansion() {
        let root = test_root("an_absent_optional_member_does_not_fail_expansion");
        root_config(&root, "[workspace]\noptional_members = [\"vendored\"]");

        let aliases = expand(&root).expect("an absent optional member must not fail the load");

        assert_eq!(aliases, vec!["acme".to_string()], "only the root is a project here");
    }

    /// §FS-workspace.2.2: present, an optional member is an ordinary member —
    /// scanned under its own config and carrying its own alias.
    #[test]
    fn a_present_optional_member_is_an_ordinary_member() {
        let root = test_root("a_present_optional_member_is_an_ordinary_member");
        root_config(&root, "[workspace]\noptional_members = [\"vendored\"]");
        member_config(&root, "vendored", Some("vendored"));

        let aliases = expand(&root).expect("a present optional member must load");

        assert_eq!(aliases, vec!["acme".to_string(), "vendored".to_string()]);
    }

    /// §FS-workspace.2.2, §FS-check.4.10: the default does not move. A member
    /// listed in `members` and missing is the same fatal config error it has
    /// always been, at the same line — and the message now names the key that
    /// would have made the absence legal, so a CI author is not left guessing
    /// that an escape hatch exists.
    #[test]
    fn a_missing_plain_member_still_fails_and_names_the_key() {
        let root = test_root("a_missing_plain_member_still_fails_and_names_the_key");
        root_config(&root, "[workspace]\nmembers = [\"vendored\"]");

        let Err(err) = expand(&root) else {
            panic!("a missing member listed in `members` must still fail the load");
        };

        assert_eq!(
            format!("{err:#}"),
            "grund.toml:5: workspace member does not exist: vendored — list it in \
             [workspace] optional_members if it may be legitimately absent"
        );
    }

    /// §FS-workspace.2.2.2: the alias of an optional member is the entry's last
    /// path segment, whatever the entry's depth — so the same citation text names
    /// the same namespace in a full checkout and a partial one.
    #[test]
    fn a_multi_segment_optional_entry_takes_its_last_segment_as_the_alias() {
        let root = test_root("a_multi_segment_optional_entry_takes_its_last_segment_as_the_alias");
        root_config(&root, "[workspace]\noptional_members = [\"hardware/sprayer\"]");
        member_config(&root, "hardware/sprayer", None);

        let aliases = expand(&root).expect("a present multi-segment optional member must load");

        assert_eq!(aliases, vec!["acme".to_string(), "sprayer".to_string()]);
    }

    /// §FS-workspace.2.2.2: the half the author called the important one. A
    /// *present* optional member whose `project_name` disagrees with the entry's
    /// last segment is refused, naming both names — otherwise citation text that
    /// resolves in a full checkout would quietly name nothing in a partial one,
    /// which is the trap only the partial checkout can spring.
    #[test]
    fn a_present_optional_member_must_agree_with_its_entry_segment() {
        let root = test_root("a_present_optional_member_must_agree_with_its_entry_segment");
        root_config(&root, "[workspace]\noptional_members = [\"vendored\"]");
        member_config(&root, "vendored", Some("warehouse"));

        let Err(err) = expand(&root) else {
            panic!("a present optional member whose project_name disagrees must fail");
        };

        assert_eq!(
            format!("{err:#}"),
            "grund.toml:5: optional workspace member `vendored` declares project_name \
             `warehouse`, but its alias is the entry's last segment `vendored` — \
             make the two agree"
        );
    }

    /// §FS-workspace.2.2: a trailing glob may not be optional. An absent parent
    /// directory names no namespaces, so the key would appear to work and do
    /// nothing — and the refusal has to say what to write instead, because a user
    /// meeting it needs the form that works.
    #[test]
    fn an_optional_member_glob_is_refused_and_names_the_shape_to_write() {
        let root = test_root("an_optional_member_glob_is_refused_and_names_the_shape_to_write");
        root_config(&root, "[workspace]\noptional_members = [\"hardware/*\"]");
        member_config(&root, "hardware/sprayer", None);

        let Err(err) = expand(&root) else {
            panic!("a glob in optional_members must be refused");
        };

        assert_eq!(
            format!("{err:#}"),
            "grund.toml:5: [workspace] optional_members may not use a glob: `hardware/*` — \
             an absent parent names no namespaces; list one concrete entry per namespace instead"
        );
    }

    /// §FS-workspace.2.2: one entry belongs to one list. The two lists state
    /// opposite intents about one directory, and resolving that in either
    /// direction would silently discard half of what the author wrote.
    #[test]
    fn an_entry_may_not_be_in_both_member_lists() {
        let root = test_root("an_entry_may_not_be_in_both_member_lists");
        root_config(
            &root,
            "[workspace]\nmembers = [\"vendored\"]\noptional_members = [\"vendored\"]",
        );
        member_config(&root, "vendored", None);

        let Err(err) = expand(&root) else {
            panic!("an entry in both lists must be refused");
        };

        assert_eq!(
            format!("{err:#}"),
            "grund.toml:6: `vendored` is listed in both [workspace] members and optional_members"
        );
    }

    /// §FS-workspace.2.2.2: the segment has to be a valid alias in its own right.
    /// An entry whose last segment is not a lowercase slug can name a namespace in
    /// neither checkout, so it is refused before any directory is looked for —
    /// here with no `Vendored` on disk at all.
    #[test]
    fn an_optional_entry_whose_segment_is_not_a_slug_is_refused() {
        let root = test_root("an_optional_entry_whose_segment_is_not_a_slug_is_refused");
        root_config(&root, "[workspace]\noptional_members = [\"Vendored\"]");

        let Err(err) = expand(&root) else {
            panic!("an optional entry with an unusable alias must be refused");
        };

        assert_eq!(
            format!("{err:#}"),
            "grund.toml:5: invalid workspace project alias `Vendored` (expected \
             [a-z][a-z0-9-]*) for workspace member `Vendored`"
        );
    }

    /// §FS-workspace.2.2, §FS-workspace.6.1: every `[workspace]` block reads the
    /// key. A nested block's absent optional member is skipped exactly as the
    /// outermost root's is — expansion re-enters the same rules at every depth, so
    /// a fix that held only at the top would be a hole one directory down.
    #[test]
    fn a_nested_block_may_declare_an_optional_member() {
        let root = test_root("a_nested_block_may_declare_an_optional_member");
        root_config(&root, "[workspace]\nmembers = [\"sub\"]");
        write(
            &root.join("sub/grund.toml"),
            "grund_config_version = 1\nproject_name = \"sub\"\n\n\
             [workspace]\noptional_members = [\"vendored\"]\n",
        );

        let aliases = expand(&root).expect("a nested absent optional member must not fail");

        assert_eq!(aliases, vec!["acme".to_string(), "sub".to_string()]);
    }

    /// §FS-workspace.2.2: a block whose last project goes missing is not the
    /// empty block §FS-workspace.6.1 refuses. That test is read from the config
    /// text — a non-empty `optional_members` list names members — so whether they
    /// are present is a fact about the checkout, and failing on it would be the
    /// verdict the key exists to remove.
    #[test]
    fn include_root_false_with_only_absent_optional_members_is_not_an_empty_block() {
        let root = test_root("include_root_false_with_only_absent_optional_members_is_not_empty");
        root_config(
            &root,
            "[workspace]\ninclude_root = false\noptional_members = [\"vendored\"]",
        );

        let aliases = expand(&root)
            .expect("a block whose only members may be absent is not an empty block");

        assert!(aliases.is_empty(), "no project is in scope, and that is not an error");
    }
}
