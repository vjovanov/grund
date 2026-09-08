//! Installed editor-template behavior exercised through the real binary.
//! §FS-lsp.2.4 is deliberately black-box: a copied or packaged `grund-lsp`
//! must carry the complete artifact without the repository `editor/` tree.

mod integrations_support;
mod support;

use integrations_support::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn help_list_and_errors_dispatch_without_starting_lsp() {
    let sandbox = Sandbox::with_binary(&workspace_binary(), "dispatch");
    let project = sandbox.project("default project Ω");

    for args in [&["--help"][..], &["integrations", "--help"][..]] {
        let output = run(&sandbox.binary, &project, args);
        assert_success(&output, "help");
        let help = stdout(&output);
        assert!(
            help.contains("integrations"),
            "help omits integrations:\n{help}"
        );
        assert!(help.to_lowercase().contains("editor"));
        assert!(help.contains("grund integrations"));
        let help_lower = help.to_ascii_lowercase();
        assert!(help_lower.contains("exits:"), "help omits exit statuses");
        for status in ["0", "2"] {
            assert!(
                help.lines()
                    .any(|line| line.split_whitespace().next() == Some(status)),
                "help omits exit status {status}:\n{help}"
            );
        }
    }

    let listed = run(&sandbox.binary, &project, ["integrations"]);
    assert_success(&listed, "list integrations");
    let listing = stdout(&listed);
    assert_eq!(listing.lines().count(), 1, "catalog is one line per entry");
    assert!(listing.contains("lsp4ij"));
    assert!(listing.to_lowercase().contains("template"));
    assert!(
        fs::read_dir(&project)
            .expect("read project after list")
            .next()
            .is_none(),
        "help or list wrote into the project"
    );

    assert_error_2(
        &run(&sandbox.binary, &project, ["integrations", "unknown"]),
        "unknown template",
    );
    assert_error_2(
        &run(
            &sandbox.binary,
            &project,
            ["integrations", "lsp4ij", "--write"],
        ),
        "missing write directory",
    );
}

#[test]
fn invalid_config_is_a_batch_error() {
    let sandbox = Sandbox::with_binary(&workspace_binary(), "invalid-config");
    let project = sandbox.project("invalid project");
    fs::write(project.join("grund.toml"), "not valid TOML\n").expect("write invalid config");
    assert_error_2(
        &run(&sandbox.binary, &project, ["integrations", "lsp4ij"]),
        "invalid config",
    );
}

#[test]
fn preview_snapshots_effective_extensions_and_quoted_commands() {
    let sandbox = Sandbox::with_binary(&workspace_binary(), "preview");
    let default_project = sandbox.project("default project Ω & '() $;! with spaces");

    let preview = run(
        &sandbox.binary,
        &default_project,
        ["integrations", "lsp4ij"],
    );
    assert_success(&preview, "default preview");
    assert!(
        fs::read_dir(&default_project)
            .expect("read default project")
            .next()
            .is_none(),
        "preview wrote into the project"
    );
    let preview_text = stdout(&preview);
    assert_import_steps(&preview_text);
    let default_template = template_from(&preview_text);
    assert_template(&default_template, DEFAULT_EXTENSIONS, &sandbox.binary);
    fs::write(
        default_project.join("grund.toml"),
        "grund_config_version = 1\n[reference]\ntrigger = \"%%\"\n",
    )
    .expect("write launch cwd config");
    assert_host_command_launches(&default_template, &default_project, &sandbox.root);

    let parent = sandbox.project("custom project Δ");
    let nested = parent.join("nested/working directory");
    fs::create_dir_all(&nested).expect("create nested project directory");
    fs::write(
        parent.join("grund.toml"),
        "grund_config_version = 1\n[scan]\nextensions = [\"md\", \"rs\", \"mystery\"]\n",
    )
    .expect("write custom config");
    let custom = run(&sandbox.binary, &nested, ["integrations", "lsp4ij"]);
    assert_success(&custom, "custom preview");
    assert_template(
        &template_from(&stdout(&custom)),
        &["md", "rs", "mystery"],
        &sandbox.binary,
    );
}

#[test]
fn write_is_exact_idempotent_and_preserves_every_conflict() {
    let sandbox = Sandbox::with_binary(&workspace_binary(), "write");
    let project = sandbox.project("project λ");
    let output_root = sandbox.root.join("output template π with spaces");

    let first = write(&sandbox.binary, &project, &output_root);
    assert_success(&first, "first write");
    assert!(stdout(&first).starts_with(&format!("created {}", output_root.display())));
    assert_import_steps(&stdout(&first));
    assert_eq!(entries(&output_root), ["README.md", "template.json"]);
    let template = fs::read(output_root.join("template.json")).expect("read template");
    let readme = fs::read(output_root.join("README.md")).expect("read README");
    assert_import_steps(&String::from_utf8(readme.clone()).expect("README is UTF-8"));
    assert_template(
        &serde_json::from_slice(&template).expect("template is JSON"),
        DEFAULT_EXTENSIONS,
        &sandbox.binary,
    );

    let repeated = write(&sandbox.binary, &project, &output_root);
    assert_success(&repeated, "identical write");
    assert!(stdout(&repeated).starts_with(&format!("unchanged {}", output_root.display())));
    assert_eq!(
        fs::read(output_root.join("template.json")).unwrap(),
        template
    );
    assert_eq!(fs::read(output_root.join("README.md")).unwrap(), readme);

    for conflict in [Conflict::Modified, Conflict::Missing, Conflict::Extra] {
        let root = sandbox.root.join(format!("conflict-{conflict:?}"));
        assert_success(&write(&sandbox.binary, &project, &root), "seed conflict");
        conflict.apply(&root);
        let before = snapshot_tree(&root);
        let refused = write(&sandbox.binary, &project, &root);
        assert_error_2(&refused, "conflicting write");
        assert!(stderr(&refused).contains("move") || stderr(&refused).contains("remove"));
        assert_eq!(
            snapshot_tree(&root),
            before,
            "conflict mutated existing bytes"
        );
    }
}

#[test]
fn packaged_crate_runs_without_the_repository_editor_tree() {
    let packaged_binary = build_packaged_binary();
    let sandbox = Sandbox::with_binary(&packaged_binary, "package");
    fs::remove_file(&packaged_binary).expect("remove retained packaged binary");
    let project = sandbox.project("packaged project");
    assert!(
        !sandbox.root.join("editor").exists(),
        "isolated package unexpectedly has repository editor assets"
    );
    let preview = run(&sandbox.binary, &project, ["integrations", "lsp4ij"]);
    assert_success(&preview, "packaged preview");
    assert_template(
        &template_from(&stdout(&preview)),
        DEFAULT_EXTENSIONS,
        &sandbox.binary,
    );
}

fn assert_import_steps(text: &str) {
    assert!(text.contains("Settings"));
    assert!(text.contains("Languages & Frameworks"));
    assert!(text.contains("Language Servers"));
    assert!(text.contains("New Language Server"));
    assert!(text.contains("Import from custom template"));
}

fn write(binary: &Path, project: &Path, output: &Path) -> std::process::Output {
    let output = output.to_str().expect("UTF-8 output path");
    run(
        binary,
        project,
        ["integrations", "lsp4ij", "--write", output],
    )
}

#[derive(Debug)]
enum Conflict {
    Modified,
    Missing,
    Extra,
}

impl Conflict {
    fn apply(&self, root: &Path) {
        match self {
            Self::Modified => fs::write(root.join("template.json"), "user modification\n")
                .expect("modify template"),
            Self::Missing => fs::remove_file(root.join("README.md")).expect("remove README"),
            Self::Extra => fs::write(root.join("notes.txt"), "user file\n").expect("add file"),
        }
    }
}

fn snapshot_tree(root: &Path) -> Vec<(String, Vec<u8>)> {
    entries(root)
        .into_iter()
        .map(|name| {
            let bytes = fs::read(root.join(&name)).expect("read snapshot entry");
            (name, bytes)
        })
        .collect()
}

/// Package both local crates and patch the unpacked `grund-lsp` to the unpacked
/// `grund-core`. The released version is intentionally absent from crates.io
/// during development; this keeps the proof about package contents without
/// weakening it to a workspace build. §FS-lsp.2.1 §FS-lsp.2.4
fn build_packaged_binary() -> PathBuf {
    let fixture = Sandbox::with_binary(&workspace_binary(), "package-build");
    let target = fixture.root.join("package-target");
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let status = Command::new(&cargo)
        .args([
            "package",
            "-p",
            "grund-core",
            "-p",
            "grund-lsp",
            "--locked",
            "--offline",
            "--allow-dirty",
            "--no-verify",
            "--target-dir",
        ])
        .arg(&target)
        .current_dir(&repo)
        .status()
        .expect("run cargo package");
    assert!(status.success(), "cargo package failed");

    let version = env!("CARGO_PKG_VERSION");
    let unpacked = fixture.root.join("unpacked");
    fs::create_dir_all(&unpacked).expect("create package extraction root");
    for package in ["grund-core", "grund-lsp"] {
        let archive = target
            .join("package")
            .join(format!("{package}-{version}.crate"));
        let status = Command::new("tar")
            .args(["-xzf"])
            .arg(&archive)
            .arg("-C")
            .arg(&unpacked)
            .status()
            .expect("extract crate archive");
        assert!(status.success(), "extract {}", archive.display());
    }
    assert!(
        !unpacked
            .join(format!("grund-lsp-{version}/editor"))
            .exists(),
        "packaged crate unexpectedly depends on the repository editor tree"
    );
    fs::create_dir_all(unpacked.join(".cargo")).expect("create cargo config directory");
    fs::write(
        unpacked.join(".cargo/config.toml"),
        format!(
            "[patch.crates-io]\ngrund-core = {{ path = {:?} }}\n",
            unpacked.join(format!("grund-core-{version}"))
        ),
    )
    .expect("write package fixture patch");
    let build_target = fixture.root.join("build-target");
    let status = Command::new(cargo)
        .args(["build", "--offline", "--manifest-path"])
        .arg(unpacked.join(format!("grund-lsp-{version}/Cargo.toml")))
        .arg("--target-dir")
        .arg(&build_target)
        .current_dir(&unpacked)
        .status()
        .expect("build unpacked grund-lsp");
    assert!(status.success(), "build unpacked grund-lsp failed");
    let built = build_target
        .join("debug")
        .join(format!("grund-lsp{}", std::env::consts::EXE_SUFFIX));
    let retained = std::env::temp_dir().join(format!(
        "grund-lsp-packaged-binary-{}{}",
        std::process::id(),
        std::env::consts::EXE_SUFFIX
    ));
    fs::copy(&built, &retained).expect("retain packaged binary after fixture cleanup");
    retained
}
