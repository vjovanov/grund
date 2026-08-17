/// Test module: the `[id]` grammar contract — what a config may build an ID out of
/// (§FS-config.3.2).
///
/// Its own module because these cases fail together for one reason: a component
/// that admits a character the citation grammar has already spent. The
/// alias-path boundary (§FS-workspace.1) is the first such character, and it is
/// asked of a *pattern* as a question about what the pattern matches — both
/// directions are pinned below, because a substring test got both wrong.
#[cfg(test)]
mod tests_id_grammar {
    use super::*;
    use super::tests_support::*;

    /// §FS-config.3.2: a `/` in any ID component is rejected at the line that
    /// wrote it. Without this, `slug_pattern = "[a-z][a-z0-9/-]*"` declared and
    /// resolved `FS-a/b` while `grund m/FS-a/b` split on the last `/` and read
    /// `m/FS-a` as the alias path (§FS-workspace.1) — an ID grund emitted and
    /// then refused as a query.
    #[test]
    fn id_grammar_rejects_a_slash_in_every_component() {
        let root = test_root("id_grammar_rejects_a_slash_in_every_component");
        let cases = [
            (
                "[id]\nformat = \"{kind}/{slug}\"\n",
                "[id].format must not contain `/`",
                2,
            ),
            (
                "[id]\nnumber_pattern = \"[0-9/]+\"\n",
                "[id].number_pattern must not match `/`",
                2,
            ),
            (
                "[id]\nslug_pattern = \"[a-z][a-z0-9/-]*\"\n",
                "[id].slug_pattern must not match `/`",
                2,
            ),
            (
                "[[kinds]]\nprefix = \"F/S\"\nfolder = \"docs\"\n",
                "[[kinds]] prefix `F/S` must not contain `/`",
                2,
            ),
        ];

        for (body, expected, line) in cases {
            write(&root.join(".agents/grund.toml"), body);
            let err = match load_config(&root) {
                Ok(_) => panic!("a `/` in the ID grammar should fail to load: {body}"),
                Err(err) => format!("{err:#}"),
            };
            assert!(
                err.contains(expected),
                "unexpected error for {body}: {err}"
            );
            assert!(
                err.contains(&format!(".agents/grund.toml:{line}:")),
                "error should locate the offending line for {body}: {err}"
            );
        }
    }

    /// §FS-config.3.2: the rule is about what a pattern **matches**, so a pattern
    /// with no `/` in its text is rejected when it can produce one. This is the
    /// case a substring test missed: `slug_pattern = "[^.[:space:]]+"` loaded,
    /// declared `FS-a/b`, listed it — and `grund FS-a/b` then refused the ID grund
    /// had just emitted, because the CLI splits an `<alias>/<ID>` argument on the
    /// last `/` (§FS-workspace.1).
    #[test]
    fn id_grammar_rejects_a_pattern_that_matches_a_slash_without_containing_one() {
        let root = test_root("id_grammar_rejects_a_pattern_that_matches_a_slash_without_containing_one");
        for (pattern, key) in [
            ("[^.[:space:]]+", "slug_pattern"),
            (".+", "slug_pattern"),
            ("[!-9]+", "slug_pattern"),
            ("[^[:space:]]+", "slug_pattern"),
            ("[0-9]+|[/a-z]+", "number_pattern"),
        ] {
            write(
                &root.join(".agents/grund.toml"),
                &format!("grund_config_version = 1\n\n[id]\n{key} = \"{pattern}\"\n"),
            );
            let err = match load_config(&root) {
                Ok(_) => panic!("a pattern matching `/` should fail to load: {pattern}"),
                Err(err) => format!("{err:#}"),
            };
            assert!(
                err.contains(&format!(".agents/grund.toml:4: [id].{key} must not match `/`")),
                "unexpected error for {pattern}: {err}"
            );
        }
    }

    /// §FS-config.3.2, the other direction: a pattern that **forbids** `/` names
    /// the character in its text and must load. Rejecting it broke configs that
    /// had always worked, and told their authors to delete the very exclusion the
    /// rule asks for.
    #[test]
    fn id_grammar_accepts_a_pattern_that_forbids_a_slash() {
        let root = test_root("id_grammar_accepts_a_pattern_that_forbids_a_slash");
        for pattern in ["[^/. ]+", "[^/.]+", "(?:a|b)[^/.]*", "[a-z][a-z0-9-]*"] {
            write(
                &root.join(".agents/grund.toml"),
                &format!("grund_config_version = 1\n\n[id]\nslug_pattern = \"{pattern}\"\n"),
            );
            let config = load_config(&root)
                .unwrap_or_else(|err| panic!("`{pattern}` cannot produce a `/` and must load: {err:#}"));
            assert_eq!(config.slug_pattern, pattern);
        }

        // `[^/]+` excludes only the `/`, so this rule has nothing to say about it.
        // The rule that does is the section-separator collision — a different
        // conflict, named as itself.
        write(
            &root.join(".agents/grund.toml"),
            "grund_config_version = 1\n\n[id]\nslug_pattern = \"[^/]+\"\n",
        );
        let err = match load_config(&root) {
            Ok(_) => panic!("`[^/]+` also matches the section separator"),
            Err(err) => format!("{err:#}"),
        };
        assert!(
            !err.contains("must not match `/`"),
            "the `/` rule must not be what rejects a pattern that forbids `/`: {err}"
        );
        assert!(
            err.contains("[id].section_separator `.` is matched by [id].slug_pattern"),
            "unexpected error: {err}"
        );
    }

    /// §FS-config.3.2: the same invariant holds for a `Config` assembled in code —
    /// `Grammar::build` is the backstop under every located check above, so no
    /// caller can route around the rule the namespace grammar depends on.
    #[test]
    fn rebuilding_the_grammar_rejects_a_slash_in_the_slug_pattern() {
        let mut config = Config::default_for(test_root(
            "rebuilding_the_grammar_rejects_a_slash_in_the_slug_pattern",
        ));
        config.slug_pattern = "[a-z][a-z0-9/-]*".into();

        let err = config
            .rebuild_grammar()
            .expect_err("a slug pattern admitting `/` is rejected at build");

        assert!(
            format!("{err:#}").contains("[id].slug_pattern must not match `/`"),
            "unexpected error: {err:#}"
        );
    }
}
