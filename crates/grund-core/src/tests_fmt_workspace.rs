/// Workspace-wide strict formatter preflight through the public API and the
/// deprecated compatibility adapter (§FS-fmt.3).
#[cfg(all(test, unix))]
mod tests_fmt_workspace {
    use super::tests_support::*;
    use super::*;

    const ROOT_DOCUMENT: &str =
        "# FS-root-thing: Root concern\n\nRoot leans on §sub/FS-sub-thing.\n";

    fn build_workspace_fixture(name: &str, root_scan_error: bool) -> PathBuf {
        let root = test_root(name);
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\nproject_name = \"root\"\n\n\
             [reference]\nmarker = \"§\"\nstrict = true\n\n\
             [id]\nformat = \"{kind}-{slug}\"\n\n\
             [scan]\ninclude = [\"docs\"]\n\n\
             [fmt.cross_refs]\nenabled = true\n\n\
             [workspace]\nmembers = [\"packages/sub\"]\n",
        );
        write(&root.join("docs/FS-root-thing.md"), ROOT_DOCUMENT);
        write(
            &root.join("packages/sub/grund.toml"),
            "grund_config_version = 1\nproject_name = \"sub\"\n\n\
             [reference]\nmarker = \"§\"\nstrict = true\n\n\
             [id]\nformat = \"{kind}-{slug}\"\n\n\
             [scan]\ninclude = [\"docs\"]\n\n\
             [fmt.cross_refs]\nenabled = true\n",
        );
        write(
            &root.join("packages/sub/docs/FS-sub-thing.md"),
            "# FS-sub-thing: Sub concern\n\nSub leans on §root/FS-root-thing.\n",
        );
        if root_scan_error {
            std::os::unix::fs::symlink("nowhere.md", root.join("docs/FS-gone.md"))
                .expect("create root broken symlink");
        }
        std::os::unix::fs::symlink(
            "nowhere.md",
            root.join("packages/sub/docs/FS-gone.md"),
        )
        .expect("create member broken symlink");
        root
    }

    #[test]
    fn public_fmt_api_aggregates_strict_workspace_scan_errors() {
        let root = build_workspace_fixture("fmt_workspace_api_errors", true);

        let error = format_references(FmtOpts {
            path: root,
            path_provided: true,
            cross_refs: true,
            ..FmtOpts::default()
        })
        .expect_err("strict workspace formatter must reject every incomplete project");
        let abort = error
            .downcast_ref::<FmtScanAbort>()
            .expect("workspace scan failures remain one structured abort");

        assert_eq!(
            abort.scan_errors,
            vec![
                ApiScanError {
                    path: "docs/FS-gone.md".to_string(),
                    message: "broken symlink: the target does not exist".to_string(),
                },
                ApiScanError {
                    path: "packages/sub/docs/FS-gone.md".to_string(),
                    message: "broken symlink: the target does not exist".to_string(),
                },
            ]
        );
    }

    #[test]
    fn public_fmt_api_preflights_later_member_before_workspace_write() {
        let root = build_workspace_fixture("fmt_workspace_api_write", false);

        let error = format_references(FmtOpts {
            path: root.clone(),
            path_provided: true,
            write: true,
            ..FmtOpts::default()
        })
        .expect_err("a later member scan error must abort the workspace write");

        assert!(error.downcast_ref::<FmtScanAbort>().is_some());
        assert_eq!(
            fs::read_to_string(root.join("docs/FS-root-thing.md"))
                .expect("read root document after public fmt"),
            ROOT_DOCUMENT
        );
    }

    #[test]
    fn compatibility_fmt_preflights_later_member_before_workspace_write() {
        let root = build_workspace_fixture("fmt_workspace_compat_write", false);
        let args = vec!["--write".to_string(), root.to_string_lossy().into_owned()];

        assert_eq!(command_fmt(&args), ExitCode::from(2));
        assert_eq!(
            fs::read_to_string(root.join("docs/FS-root-thing.md"))
                .expect("read root document after compatibility fmt"),
            ROOT_DOCUMENT
        );
    }
}
