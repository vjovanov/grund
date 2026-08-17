/// Test module: the `[id]` grammar contract — what a config may build an ID out of
/// (§FS-config.3.2).
///
/// Its own module because these cases fail together for one reason: a component
/// that admits a character the citation grammar has already spent. The
/// alias-path boundary (§FS-workspace.1) is the first such character.
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
                "[id] format must not contain `/`",
                2,
            ),
            (
                "[id]\nnumber_pattern = \"[0-9/]+\"\n",
                "[id] number_pattern must not contain `/`",
                2,
            ),
            (
                "[id]\nslug_pattern = \"[a-z][a-z0-9/-]*\"\n",
                "[id] slug_pattern must not contain `/`",
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
            format!("{err:#}").contains("[id] slug_pattern must not contain `/`"),
            "unexpected error: {err:#}"
        );
    }
}
