/// Test module: `[workspace] optional_members` — the member a repository has
/// declared may be legitimately absent (§FS-workspace.2.2, §FS-workspace.2.2.1,
/// §FS-workspace.2.2.2), and the announcement an absent one earns
/// (§FS-check.4.9).
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

    /// §FS-workspace.2.2, §FS-check.4.9: the default does not move. A member
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

    /// §FS-workspace.2.2: the same refusal in the checkout that does *not* have the
    /// directory. Behind the `is_dir` test it could only fire where the member was
    /// present, and the other checkout was told to list the entry in
    /// `optional_members` — where the author had already put it. What is wrong is
    /// the pair of lists, and the pair reads the same in every checkout.
    #[test]
    fn an_entry_in_both_lists_is_refused_with_the_member_absent_too() {
        let root = test_root("an_entry_in_both_lists_is_refused_with_the_member_absent_too");
        root_config(
            &root,
            "[workspace]\nmembers = [\"vendored\"]\noptional_members = [\"vendored\"]",
        );

        let Err(err) = expand(&root) else {
            panic!("an entry in both lists must be refused whether or not it is there");
        };

        assert_eq!(
            format!("{err:#}"),
            "grund.toml:6: `vendored` is listed in both [workspace] members and optional_members",
            "the same sentence at the same line as the checkout that has the member"
        );
    }

    /// §FS-workspace.3: an absent entry's alias is unique among its siblings like
    /// any other. Left out of the check, this config both reports a dangling
    /// reference *in* namespace `vendored` and announces that `vendored` was not
    /// checked — and the announcement is worth nothing if it can be false.
    #[test]
    fn an_absent_entry_may_not_take_a_present_siblings_alias() {
        let root = test_root("an_absent_entry_may_not_take_a_present_siblings_alias");
        root_config(
            &root,
            "[workspace]\nmembers = [\"present\"]\noptional_members = [\"gone/vendored\"]",
        );
        member_config(&root, "present", Some("vendored"));

        let Err(err) = expand(&root) else {
            panic!("an absent entry may not claim an alias a present member already has");
        };

        assert_eq!(
            format!("{err:#}"),
            "grund.toml:5: duplicate workspace project alias `vendored` (workspace members \
             `gone/vendored` and `present`)"
        );
    }

    /// §FS-workspace.3: the root project and the top-level members share one level,
    /// so an absent entry may not name the root's own namespace either — a run that
    /// announced `acme` unverified while checking `acme` would contradict itself.
    #[test]
    fn an_absent_entry_may_not_take_the_root_alias() {
        let root = test_root("an_absent_entry_may_not_take_the_root_alias");
        root_config(&root, "[workspace]\noptional_members = [\"acme\"]");

        let Err(err) = expand(&root) else {
            panic!("an absent entry may not claim the root's alias");
        };

        assert_eq!(
            format!("{err:#}"),
            "grund.toml:5: duplicate workspace project alias `acme` (workspace root and \
             workspace member `acme`)"
        );
    }

    /// §FS-workspace.3: and not another absent entry's, which is the shape with no
    /// project on either side — two entries, two announcements, one namespace.
    #[test]
    fn two_absent_entries_may_not_share_an_alias() {
        let root = test_root("two_absent_entries_may_not_share_an_alias");
        root_config(
            &root,
            "[workspace]\noptional_members = [\"a/vendored\", \"b/vendored\"]",
        );

        let Err(err) = expand(&root) else {
            panic!("two absent entries may not claim one alias");
        };

        assert_eq!(
            format!("{err:#}"),
            "grund.toml:5: duplicate workspace project alias `vendored` (workspace members \
             `a/vendored` and `b/vendored`)"
        );
    }

    /// §FS-workspace.2.2, §FS-workspace.6.1: one entry written twice is one member
    /// in *either* checkout. Folding it only where the directory is present would
    /// have made the same config a launch-time error for CI and a green run for the
    /// developer holding the member — the checkout-dependent verdict this key
    /// exists to remove.
    #[test]
    fn one_optional_entry_listed_twice_is_one_member_in_either_checkout() {
        let root = test_root("one_optional_entry_listed_twice_is_one_member");
        root_config(
            &root,
            "[workspace]\noptional_members = [\"vendored\", \"vendored\"]",
        );

        let mut absent = load_config(&root).expect("the config must load with the member absent");
        let aliases: Vec<String> = expand_workspace_tree(&mut absent)
            .expect("a repeated absent entry must not fail the load")
            .into_iter()
            .map(|entry| entry.alias)
            .collect();

        assert_eq!(aliases, vec!["acme".to_string()], "only the root is a project here");
        assert_eq!(
            absent
                .workspace_absent_optional
                .iter()
                .map(|namespace| namespace.written.clone())
                .collect::<Vec<_>>(),
            vec!["vendored".to_string()],
            "one directory is one namespace, announced once"
        );

        member_config(&root, "vendored", Some("vendored"));

        assert_eq!(
            expand(&root).expect("a repeated present entry must load"),
            vec!["acme".to_string(), "vendored".to_string()],
            "and the checkout that has the member reaches the same verdict"
        );
    }

    /// §FS-errors.4: the collision is reported at the line of the list the second
    /// claimant was written on. A *present* optional member reported at the
    /// `members` line sends the reader to the other list — and a block that lists
    /// no plain members at all has no such line, so the sentence lost its location
    /// entirely.
    #[test]
    fn a_present_optional_claimant_reports_at_the_optional_members_line() {
        let root = test_root("a_present_optional_claimant_reports_at_the_optional_members_line");
        root_config(
            &root,
            "[workspace]\nmembers = [\"m\"]\noptional_members = [\"x/vendored\", \"y/vendored\"]",
        );
        member_config(&root, "m", None);
        member_config(&root, "x/vendored", None);
        member_config(&root, "y/vendored", None);

        let Err(err) = expand(&root) else {
            panic!("two present optional members may not claim one alias");
        };

        assert_eq!(
            format!("{err:#}"),
            "grund.toml:6: duplicate workspace project alias `vendored` (workspace members \
             `x/vendored` and `y/vendored`)"
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
