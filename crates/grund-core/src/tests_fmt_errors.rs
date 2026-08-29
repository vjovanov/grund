/// Test module: strict formatter scan errors across the public API (§FS-fmt.3).
#[cfg(all(test, unix))]
mod tests_fmt_errors {
    use super::tests_support::*;
    use super::*;

    /// A whole-declaration-set rewrite keeps its fatal scan result structured
    /// for embedding callers, with every path in scan order.
    #[test]
    fn public_fmt_api_preserves_every_strict_scan_error() {
        let root = test_root("public_fmt_api_preserves_every_strict_scan_error");
        write(
            &root.join(".agents/grund.toml"),
            "grund_config_version = 1\n\n[scan]\ninclude = [\"docs\"]\n",
        );
        write(
            &root.join("docs/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n\nCites §FS-001-alpha.\n",
        );
        std::os::unix::fs::symlink("nowhere.md", root.join("docs/FS-002-gone.md"))
            .expect("create first broken symlink");
        std::os::unix::fs::symlink("nowhere-either.md", root.join("docs/FS-003-also-gone.md"))
            .expect("create second broken symlink");

        let error = format_references(FmtOpts {
            path: root,
            path_provided: true,
            cross_refs: true,
            ..FmtOpts::default()
        })
        .expect_err("strict formatter must reject an incomplete declaration set");
        let abort = error
            .downcast_ref::<FmtScanAbort>()
            .expect("strict errors remain structured across the public API");

        assert_eq!(
            abort.scan_errors,
            vec![
                ApiScanError {
                    path: "docs/FS-002-gone.md".to_string(),
                    message: "broken symlink: the target does not exist".to_string(),
                },
                ApiScanError {
                    path: "docs/FS-003-also-gone.md".to_string(),
                    message: "broken symlink: the target does not exist".to_string(),
                },
            ]
        );
        assert_eq!(
            abort.to_string(),
            concat!(
                "nothing was rewritten: docs/FS-002-gone.md: broken symlink: the target does not exist\n",
                "nothing was rewritten: docs/FS-003-also-gone.md: broken symlink: the target does not exist",
            )
        );
    }
}
