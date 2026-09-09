/// Core contract for the typed `refs` query-failure carrier and the deprecated
/// compatibility adapter (§FS-refs.4, §FS-errors.2.3).
#[cfg(test)]
mod tests_refs_query_failures {
    use super::tests_support::*;
    use super::*;

    fn refs_repo(name: &str) -> PathBuf {
        let root = test_root(name);
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n\
             [reference]\n\
             strict = false\n\n\
             [id]\n\
             format = \"{kind}-{number}-{slug}\"\n\n\
             [scan]\n\
             include = [\"docs\"]\n",
        );
        write(
            &root.join("docs/FS-042-user-login.md"),
            "# FS-042-user-login: Login\n",
        );
        write(
            &root.join("docs/FS-042-user-logout.md"),
            "# FS-042-user-logout: Logout\n",
        );
        write(
            &root.join("docs/FS-100-empty.md"),
            "# FS-100-empty: Empty\n",
        );
        root
    }

    fn query(root: &Path, id: &str) -> Result<RefsOutput> {
        refs(RefsOpts {
            path: root.to_path_buf(),
            path_provided: true,
            id: id.to_string(),
            section: None,
        })
    }

    #[test]
    fn invalid_format_is_a_typed_query_failure_with_the_format_hint() {
        let root = refs_repo("refs_invalid_format_carrier");
        let output = query(&root, "FS-bar").expect("resolver rejection is data");
        let failure = output.query_failure.expect("typed query failure");
        assert_eq!(failure.kind, RefsQueryFailureKind::InvalidId);
        assert_eq!(failure.message, "invalid ID `FS-bar`");
        assert_eq!(
            failure.format_hint.as_deref(),
            Some("{kind}-{number}-{slug}")
        );
        assert!(output.hits.is_empty());
    }

    #[test]
    fn ambiguous_number_only_shorthand_is_typed_without_a_format_hint() {
        let root = refs_repo("refs_ambiguous_shorthand_carrier");
        let output = query(&root, "FS-042").expect("resolver rejection is data");
        let failure = output.query_failure.expect("typed query failure");
        assert_eq!(failure.kind, RefsQueryFailureKind::Ambiguous);
        assert_eq!(
            failure.message,
            "ambiguous ID: FS-042 (matches FS-042-user-login, FS-042-user-logout)"
        );
        assert_eq!(failure.format_hint, None);
        assert!(output.hits.is_empty());
    }

    #[test]
    fn successful_and_setup_outcomes_do_not_receive_the_query_failure_carrier() {
        let root = refs_repo("refs_query_failure_seams");
        let output = query(&root, "FS-100-empty").expect("valid empty result");
        assert_eq!(output.query_failure, None);
        assert!(output.hits.is_empty());

        let error = query(&root, "unknown/FS-100-empty").expect_err("unknown alias");
        assert!(error.to_string().contains("unknown project alias `unknown`"));
    }

    fn version(text: &str) -> (u64, u64, u64) {
        let mut parts = text.split('.').map(|part| {
            part.split(|ch: char| !ch.is_ascii_digit())
                .next()
                .unwrap_or("0")
                .parse::<u64>()
                .unwrap_or(0)
        });
        (
            parts.next().unwrap_or(0),
            parts.next().unwrap_or(0),
            parts.next().unwrap_or(0),
        )
    }

    #[test]
    fn deprecated_compat_refs_uses_the_same_release_mapping() {
        let root = refs_repo("compat_refs_query_failure_mapping");
        let args = vec!["FS-bar".to_string(), root.display().to_string()];
        let actual = command_refs(&args);
        let expected = if version(env!("CARGO_PKG_VERSION")) >= version("0.15.0") {
            ExitCode::from(1)
        } else {
            ExitCode::from(2)
        };
        assert_eq!(actual, expected);
    }
}
