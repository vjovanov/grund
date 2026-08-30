//! §FS-config.3.6 / §FS-workspace.8.6: with `relative_paths = false`, one
//! workspace-wide report keeps the cwd-derived base. A sibling member uses the
//! minimum bounded `..` spelling instead of leaking its canonical absolute path.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn build_fixture() -> PathBuf {
    let root = manifest_dir().join("target/workspace-relative-paths-test");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("docs")).expect("create root docs dir");
    fs::create_dir_all(root.join("hw/docs")).expect("create member docs dir");
    fs::write(
        root.join("grund.toml"),
        "project_name = \"root\"\n\n\
         [output]\nrelative_paths = false\n\n\
         [id]\nformat = \"{kind}-{slug}\"\nslug_pattern = \"[a-z][a-z0-9-]*\"\n\n\
         [scan]\ninclude = [\"docs\"]\nrespect_gitignore = false\n\n\
         [workspace]\nmembers = [\"hw\"]\n",
    )
    .expect("write root config");
    fs::write(
        root.join("docs/FS-root.md"),
        "# FS-root: Root\n\nThe root cites §hw/FS-nozzle.\n",
    )
    .expect("write root declaration");
    fs::write(
        root.join("hw/grund.toml"),
        "project_name = \"hw\"\n\n\
         [id]\nformat = \"{kind}-{slug}\"\nslug_pattern = \"[a-z][a-z0-9-]*\"\n\n\
         [scan]\ninclude = [\"docs\"]\nrespect_gitignore = false\n",
    )
    .expect("write member config");
    fs::write(
        root.join("hw/docs/FS-nozzle.md"),
        "# FS-nozzle: Nozzle\n\nThe member declaration.\n",
    )
    .expect("write member declaration");
    root
}

fn run_grund(args: &[&str], cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_grund"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("spawn grund")
}

fn assert_success(output: &Output, command: &str) {
    assert!(
        output.status.success(),
        "`grund {command}` failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
}

#[test]
fn relative_paths_false_from_subdirectory_reaches_workspace_member() {
    let root = build_fixture();
    let cwd = root.join("docs");

    let cover = run_grund(&["cover"], &cwd);
    assert_success(&cover, "cover");
    assert_eq!(
        String::from_utf8_lossy(&cover.stdout),
        concat!(
            "../hw/docs/FS-nozzle.md:\n",
            "  (no citations)\n",
            "FS-root.md:\n",
            "  3:16 §hw/FS-nozzle\n",
        )
    );

    let list = run_grund(&["list"], &cwd);
    assert_success(&list, "list");
    assert_eq!(
        String::from_utf8_lossy(&list.stdout),
        concat!(
            "hw/FS-nozzle  ../hw/docs/FS-nozzle.md:1  Nozzle\n",
            "root/FS-root  FS-root.md:1  Root\n",
        )
    );

    let cover_json = run_grund(&["cover", "--format", "json"], &cwd);
    assert_success(&cover_json, "cover --format json");
    assert_eq!(
        String::from_utf8_lossy(&cover_json.stdout),
        concat!(
            "{\"project\":\"hw\",\"path\":\"../hw/docs/FS-nozzle.md\",\"citations\":[]}\n",
            "{\"project\":\"root\",\"path\":\"FS-root.md\",\"citations\":[",
            "{\"project\":\"root\",\"path\":\"FS-root.md\",\"line\":3,\"column\":16,",
            "\"id\":\"hw/FS-nozzle\",\"section\":null,\"marker\":true,",
            "\"text\":\"§hw/FS-nozzle\"}]}\n",
        )
    );

    #[cfg(unix)]
    {
        // §FS-config.3.5.2 / §FS-config.3.6: an explicit in-tree symlink keeps its
        // bounded lexical report path even when its physical target is outside.
        let external = root
            .parent()
            .expect("fixture has a parent")
            .join("workspace-relative-paths-external.md");
        fs::write(
            &external,
            "# FS-external: External\n\nThis cites \u{a7}FS-missing.\n",
        )
        .expect("write external symlink target");
        let link = root.join("docs/external-link.md");
        std::os::unix::fs::symlink(&external, &link).expect("create external file symlink");

        let check = run_grund(&["check", "docs/external-link.md"], &root);
        assert_eq!(check.status.code(), Some(1));
        assert_eq!(
            String::from_utf8_lossy(&check.stdout),
            concat!(
                "external-link.md:1: declared but never cited: FS-external\n",
                "external-link.md:3: unknown reference FS-missing\n",
            )
        );
        assert_eq!(String::from_utf8_lossy(&check.stderr), "");

        let full_check = run_grund(&["check", "docs/external-link.md", "--full"], &root);
        assert_eq!(full_check.status.code(), Some(1));
        assert_eq!(
            String::from_utf8_lossy(&full_check.stdout),
            String::from_utf8_lossy(&check.stdout)
        );
        assert_eq!(
            String::from_utf8_lossy(&full_check.stderr),
            "warning: --full has no effect with an explicit PATH — it cancels [scan] include, and external-link.md already bypasses it\n"
        );

        let full_check_json = run_grund(
            &[
                "check",
                "docs/external-link.md",
                "--full",
                "--format",
                "json",
            ],
            &root,
        );
        assert_eq!(full_check_json.status.code(), Some(1));
        assert_eq!(
            String::from_utf8_lossy(&full_check_json.stderr),
            concat!(
                "{\"severity\":\"warning\",\"path\":null,\"line\":null,",
                "\"code\":\"full-scope-ignored\",",
                "\"message\":\"--full has no effect with an explicit PATH — it cancels ",
                "[scan] include, and external-link.md already bypasses it\",",
                "\"sites\":null}\n",
            )
        );

        let linked_cover = run_grund(
            &["cover", "docs/external-link.md", "--format", "json"],
            &root,
        );
        assert_success(&linked_cover, "cover docs/external-link.md --format json");
        assert_eq!(
            String::from_utf8_lossy(&linked_cover.stdout),
            concat!(
                "{\"path\":\"external-link.md\",\"citations\":[",
                "{\"path\":\"external-link.md\",\"line\":3,\"column\":12,",
                "\"id\":\"FS-missing\",\"section\":null,\"marker\":true,",
                "\"text\":\"\u{a7}FS-missing\"}]}\n",
            )
        );
        let physical = external.to_string_lossy();
        let machine_prefix = manifest_dir().to_string_lossy().into_owned();
        assert!(!String::from_utf8_lossy(&check.stdout).contains(physical.as_ref()));
        assert!(!String::from_utf8_lossy(&linked_cover.stdout).contains(physical.as_ref()));
        assert!(!String::from_utf8_lossy(&full_check.stderr).contains(physical.as_ref()));
        assert!(!String::from_utf8_lossy(&full_check_json.stderr).contains(physical.as_ref()));
        assert!(!String::from_utf8_lossy(&full_check.stderr).contains(&machine_prefix));
        assert!(!String::from_utf8_lossy(&full_check_json.stderr).contains(&machine_prefix));
    }
}
