// Content-and-contract tests for `grund init`. The e2e harness covers the CLI surface
// (exit code + stderr listing); these tests cover what the *bytes on disk* look like
// after init runs: every emitted file exists, the `grund.toml` location matches the spec,
// the config validates, and `grund check` is clean against the freshly-scaffolded tree.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn workdir(suffix: &str) -> PathBuf {
    let dir = manifest_dir().join("target/init-tests").join(suffix);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create workdir");
    dir
}

fn run_grund<P: AsRef<Path>>(args: &[&str], cwd: P) -> Output {
    Command::new(env!("CARGO_BIN_EXE_grund"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("spawn grund")
}

#[test]
fn init_default_writes_canonical_pair_and_passes_check() {
    // §FS-init.2.1 (default form) + §FS-config.1 (.agents/grund.toml location).
    let target = workdir("init_default_writes_canonical_pair_and_passes_check");
    let output = run_grund(&["init", target.to_str().unwrap()], manifest_dir());
    assert!(
        output.status.success(),
        "init failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        target.join("AGENTS.md").is_file(),
        "AGENTS.md was not written"
    );
    assert!(
        target.join(".agents/grund.toml").is_file(),
        ".agents/grund.toml was not written; init must place grund.toml under .agents/"
    );
    assert!(
        !target.join("grund.toml").exists(),
        "init must NOT write grund.toml at the repo root — it lives under .agents/"
    );

    let validate = run_grund(
        &["config", "validate", target.to_str().unwrap()],
        manifest_dir(),
    );
    assert!(
        validate.status.success(),
        "init's grund.toml does not validate:\n{}",
        String::from_utf8_lossy(&validate.stderr)
    );
}

#[test]
fn init_docs_form_emits_full_scaffold_and_check_is_clean() {
    // §FS-init.2.1 (--docs form). The scaffolded tree must satisfy `grund check` —
    // i.e. the canonical AGENTS.md + grund.toml + docs skeleton is internally consistent.
    let target = workdir("init_docs_form_emits_full_scaffold_and_check_is_clean");
    let output = run_grund(
        &[
            "init",
            target.to_str().unwrap(),
            "--docs",
            "--name",
            "DemoProject",
        ],
        manifest_dir(),
    );
    assert!(
        output.status.success(),
        "init --docs failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let expected = [
        "AGENTS.md",
        ".agents/grund.toml",
        "docs/grund.md",
        "docs/goals.md",
        "docs/roadmap.md",
        "docs/changelog.md",
        "requirements.md",
        "docs/architecture/README.md",
        "docs/decisions/architectural/.gitkeep",
        "docs/decisions/functional/.gitkeep",
        "e2e/README.md",
        "e2e/cases/.gitkeep",
    ];
    for rel in expected {
        assert!(target.join(rel).exists(), "init --docs did not write {rel}");
    }

    let agents = fs::read_to_string(target.join("AGENTS.md")).expect("read AGENTS.md");
    assert!(
        agents.contains("DemoProject"),
        "AGENTS.md must interpolate the --name into the H1 / opening sentence"
    );

    let grund_toml =
        fs::read_to_string(target.join(".agents/grund.toml")).expect("read .agents/grund.toml");
    assert!(
        grund_toml.contains("project_name = \"DemoProject\""),
        ".agents/grund.toml must carry project_name from --name"
    );

    let check = run_grund(&["check", target.to_str().unwrap()], manifest_dir());
    assert!(
        check.status.success(),
        "freshly init'd tree should be grund-clean but produced:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}

#[test]
fn init_docs_default_requirements_file_is_scanned() {
    // §FS-init.2.1 / §FS-config.3.5: the generated FS home is in the generated
    // scan roots, so declarations added where `init` points users are resolvable.
    let target = workdir("init_docs_default_requirements_file_is_scanned");
    let output = run_grund(
        &["init", target.to_str().unwrap(), "--docs"],
        manifest_dir(),
    );
    assert!(
        output.status.success(),
        "init --docs failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    fs::write(
        target.join("requirements.md"),
        "# Requirements\n\n## FS-001-session: Sessions can be created\n",
    )
    .expect("write requirements.md");
    fs::create_dir_all(target.join("src")).expect("create src");
    fs::write(
        target.join("src/lib.rs"),
        "/// Creates a user session. §FS-001-session\npub fn create_session() {}\n",
    )
    .expect("write src/lib.rs");

    let check = run_grund(&["check", target.to_str().unwrap()], manifest_dir());
    assert!(
        check.status.success(),
        "FS declaration in generated requirements.md should resolve:\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
}

#[test]
fn init_description_flag_writes_config_key() {
    // §FS-init.1 + §FS-init.2.4: `--description` replaces the commented
    // teaching line in the generated config with the real key.
    let target = workdir("init_description_flag_writes_config_key");
    let output = run_grund(
        &[
            "init",
            target.to_str().unwrap(),
            "--name",
            "api",
            "--description",
            "Payment API service",
        ],
        manifest_dir(),
    );
    assert!(
        output.status.success(),
        "init --description failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let grund_toml =
        fs::read_to_string(target.join(".agents/grund.toml")).expect("read .agents/grund.toml");
    assert!(
        grund_toml.contains("project_description = \"Payment API service\""),
        ".agents/grund.toml must carry project_description from --description"
    );
    assert!(
        !grund_toml.contains("# project_description ="),
        "the teaching comment must be replaced by the real key"
    );

    let validate = run_grund(
        &["config", "validate", target.to_str().unwrap()],
        manifest_dir(),
    );
    assert!(
        validate.status.success(),
        "generated config with project_description must validate: {}",
        String::from_utf8_lossy(&validate.stderr)
    );
}

#[test]
fn init_description_flag_rejects_multiline_value() {
    // §FS-init.1: `--description` mirrors the config-side single-line rule.
    let target = workdir("init_description_flag_rejects_multiline_value");
    let output = run_grund(
        &[
            "init",
            target.to_str().unwrap(),
            "--description",
            "first line\nsecond line",
        ],
        manifest_dir(),
    );
    assert_eq!(
        output.status.code(),
        Some(2),
        "multi-line --description must be a CLI error"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--description must be a single line"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !target.join(".agents/grund.toml").exists(),
        "no file may be written on a rejected --description"
    );
}

#[test]
fn init_failed_docs_write_reports_prior_progress() {
    // §FS-init.2.2 / §FS-init.4: init reports touched paths as it goes. If a
    // later scaffold write fails, the user still needs the transcript for the
    // files that were already created.
    let target = workdir("init_failed_docs_write_reports_prior_progress");
    fs::write(target.join("docs"), "not a directory").expect("write docs file");

    let output = run_grund(
        &["init", target.to_str().unwrap(), "--docs"],
        manifest_dir(),
    );
    assert!(
        !output.status.success(),
        "init should fail when docs/ is a file"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("wrote AGENTS.md"),
        "stderr should include prior AGENTS.md write, got:\n{stderr}"
    );
    assert!(
        stderr.contains("wrote .agents/grund.toml"),
        "stderr should include prior config write, got:\n{stderr}"
    );
    assert!(
        stderr.contains("error: "),
        "stderr should include final error, got:\n{stderr}"
    );
    assert!(target.join("AGENTS.md").is_file());
    assert!(target.join(".agents/grund.toml").is_file());
}

#[test]
fn init_generated_config_comments_list_constrained_values() {
    // §FS-init.2.4: the generated config is a teaching surface, so non-boolean
    // constrained keys carry inline comments listing their accepted values.
    let target = workdir("init_generated_config_comments_list_constrained_values");
    let output = run_grund(&["init", target.to_str().unwrap()], manifest_dir());
    assert!(
        output.status.success(),
        "init failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let grund_toml =
        fs::read_to_string(target.join(".agents/grund.toml")).expect("read .agents/grund.toml");
    for expected in [
        "inline_style = \"citation-with-note\" # citation-with-note | citation-only",
        "section_heading_levels = \"strict\" # strict | warn | loose",
        "format = \"text\" # text | json",
        "anchor_format = \"github\" # github | gitlab | mkdocs | pandoc | none",
    ] {
        assert!(
            grund_toml.contains(expected),
            "generated config should include `{expected}`, got:\n{grund_toml}"
        );
    }
    assert!(
        !grund_toml.contains("# true | false"),
        "generated config should not enumerate boolean values, got:\n{grund_toml}"
    );

    let validate = run_grund(
        &["config", "validate", target.to_str().unwrap()],
        manifest_dir(),
    );
    assert!(
        validate.status.success(),
        "commented generated config should validate:\n{}",
        String::from_utf8_lossy(&validate.stderr)
    );
}

#[test]
fn init_agents_guidance_uses_existing_configured_artifact_homes() {
    let target = workdir("init_agents_guidance_uses_existing_configured_artifact_homes");
    fs::create_dir_all(target.join(".agents")).expect("create .agents");
    fs::write(
        target.join(".agents/grund.toml"),
        r#"grund_config_version = 1

[scan]
include = ["specs", "records", "crates"]

[[kinds]]
prefix = "FS"
folder = "specs"
title = "Product spec"

[[kinds]]
prefix = "ADR"
folder = "records/adr"
title = "Architecture decision"
"#,
    )
    .expect("write custom grund.toml");

    let output = run_grund(
        &["init", target.to_str().unwrap(), "--name", "Configured"],
        manifest_dir(),
    );
    assert!(
        output.status.success(),
        "init failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let agents = fs::read_to_string(target.join("AGENTS.md")).expect("read AGENTS.md");
    assert!(
        agents.contains("- [FS](specs): Product spec"),
        "AGENTS.md should describe configured spec homes:\n{agents}"
    );
    assert!(
        agents.contains("- [ADR](records/adr): Architecture decision"),
        "AGENTS.md should describe configured decision homes:\n{agents}"
    );
    assert!(
        !agents.contains("docs/architecture/") && !agents.contains("docs/decisions/"),
        "AGENTS.md must not introduce canonical docs folders when specs are configured elsewhere"
    );
    assert!(
        !agents.contains("`grund` scans:"),
        "AGENTS.md must not surface scan scope (§FS-init.2.3.4.4):\n{agents}"
    );
}

#[test]
fn init_docs_existing_implicit_legacy_config_uses_legacy_fs_home() {
    // §FS-config.2 / §FS-init.2.1: existing configs without explicit kind homes
    // keep the legacy FS folder, and `init --docs` must scaffold that effective home.
    let target = workdir("init_docs_existing_implicit_legacy_config_uses_legacy_fs_home");
    fs::create_dir_all(target.join(".agents")).expect("create .agents");
    fs::write(
        target.join(".agents/grund.toml"),
        "grund_config_version = 1\n",
    )
    .expect("write legacy implicit config");

    let output = run_grund(
        &["init", target.to_str().unwrap(), "--docs"],
        manifest_dir(),
    );
    assert!(
        output.status.success(),
        "init --docs failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("wrote docs/functional-spec/README.md"),
        "legacy implicit config should scaffold legacy FS folder, got:\n{stderr}"
    );
    assert!(
        stderr.contains("then add it under docs/functional-spec"),
        "next guidance should target the effective legacy FS folder, got:\n{stderr}"
    );
    assert!(
        !target.join("requirements.md").exists(),
        "legacy implicit config must not scaffold the new FS file"
    );
}

#[test]
fn init_updates_existing_agent_entrypoint_without_creating_agents_md() {
    // §FS-init.2.1 / §FS-init.2.3: automatic mode preserves an existing repo's
    // agent-entrypoint choice instead of adding canonical AGENTS.md.
    let target = workdir("init_updates_existing_agent_entrypoint_without_creating_agents_md");
    fs::write(target.join("CLAUDE.md"), "# Claude notes\n").expect("write CLAUDE.md");

    let output = run_grund(&["init", target.to_str().unwrap()], manifest_dir());
    assert!(
        output.status.success(),
        "init failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("appended CLAUDE.md"),
        "init should append to existing CLAUDE.md, got:\n{stderr}"
    );
    assert!(
        !stderr.contains("AGENTS.md") && !target.join("AGENTS.md").exists(),
        "init should not create AGENTS.md when an existing agent entrypoint is present, got:\n{stderr}"
    );
    assert!(
        stderr.contains("see CLAUDE.md for the full workflow."),
        "next block should point at the selected entrypoint, got:\n{stderr}"
    );

    let claude = fs::read_to_string(target.join("CLAUDE.md")).expect("read CLAUDE.md");
    assert!(
        claude.starts_with("# Claude notes\n\n## Grounding with grund (v5)\n"),
        "CLAUDE.md should keep existing notes and append the managed block:\n{claude}"
    );
}

#[test]
fn init_agent_flags_create_requested_entrypoints() {
    // §FS-init.1 / §FS-init.2.1: explicit agent flags create exactly the requested
    // entrypoint families and do not add the automatic AGENTS.md fallback.
    let target = workdir("init_agent_flags_create_requested_entrypoints");

    let output = run_grund(
        &[
            "init",
            target.to_str().unwrap(),
            "--claude",
            "--gemini",
            "--copilot",
        ],
        manifest_dir(),
    );
    assert!(
        output.status.success(),
        "init failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    for rel in [
        "CLAUDE.md",
        ".claude/CLAUDE.md",
        "GEMINI.md",
        ".github/copilot-instructions.md",
    ] {
        assert!(
            target.join(rel).is_file(),
            "explicit init should create {rel}"
        );
        assert!(
            stderr.contains(&format!("wrote {rel}")),
            "stderr should report writing {rel}, got:\n{stderr}"
        );
    }
    assert!(
        !target.join("AGENTS.md").exists(),
        "explicit companion-agent init should not add AGENTS.md"
    );
}

#[test]
fn init_workspace_companion_only_marks_missing_self_agents_and_uses_marker() {
    // §FS-init.2.3.4.15: a companion-only workspace init must not point the self
    // row at a missing AGENTS.md, and its grammar hint must use the local marker.
    let root = workdir("init_workspace_companion_only_marks_missing_self_agents_and_uses_marker");
    fs::create_dir_all(root.join(".agents")).expect("create root config dir");
    fs::write(
        root.join(".agents/grund.toml"),
        "project_name = \"root\"\n\n[workspace]\nmembers = [\"apps/api\"]\n",
    )
    .expect("write workspace config");
    let api = root.join("apps/api");
    fs::create_dir_all(api.join(".agents")).expect("create api config dir");
    fs::write(
        api.join(".agents/grund.toml"),
        "[reference]\nmarker = \"@\"\nstrict = true\n",
    )
    .expect("write api config");

    let output = run_grund(&["init", api.to_str().unwrap(), "--claude"], manifest_dir());
    assert!(
        output.status.success(),
        "init failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !api.join("AGENTS.md").exists(),
        "explicit companion-only init should not create AGENTS.md"
    );

    let claude = fs::read_to_string(api.join("CLAUDE.md")).expect("read CLAUDE.md");
    assert!(
        claude.contains("Cross-project citations use @alias/<ID>."),
        "workspace citation hint should use the configured marker:\n{claude}"
    );
    assert!(
        claude.contains("- [`api`](./) *(not yet initialized)*"),
        "self row should link to the project directory until AGENTS.md exists:\n{claude}"
    );
    assert!(
        !claude.contains("- [`api`](AGENTS.md)"),
        "companion-only init must not link to a missing AGENTS.md:\n{claude}"
    );
}

#[test]
fn init_cursor_flag_updates_existing_legacy_cursorrules() {
    // §FS-init.2.1 / §FS-init.2.3: explicit --cursor creates/updates the modern
    // Cursor rule file and also updates legacy .cursorrules when it already
    // exists, without creating the legacy file for new adopters.
    let target = workdir("init_cursor_flag_updates_existing_legacy_cursorrules");
    fs::write(target.join(".cursorrules"), "# Legacy Cursor notes\n").expect("write .cursorrules");

    let output = run_grund(
        &["init", target.to_str().unwrap(), "--cursor"],
        manifest_dir(),
    );
    assert!(
        output.status.success(),
        "init failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("wrote .cursor/rules/grund.mdc"),
        "--cursor should create the modern Cursor rules file, got:\n{stderr}"
    );
    assert!(
        stderr.contains("appended .cursorrules"),
        "--cursor should update existing legacy .cursorrules, got:\n{stderr}"
    );
    assert!(
        !target.join("AGENTS.md").exists(),
        "explicit Cursor init should not add AGENTS.md"
    );

    let legacy = fs::read_to_string(target.join(".cursorrules")).expect("read .cursorrules");
    assert!(
        legacy.starts_with("# Legacy Cursor notes\n\n## Grounding with grund (v5)\n"),
        ".cursorrules should keep existing notes and append the managed block:\n{legacy}"
    );

    let target2 = workdir("init_cursor_flag_does_not_create_legacy_cursorrules");
    let output2 = run_grund(
        &["init", target2.to_str().unwrap(), "--cursor"],
        manifest_dir(),
    );
    assert!(
        output2.status.success(),
        "init failed: stderr={}",
        String::from_utf8_lossy(&output2.stderr)
    );
    assert!(
        !target2.join(".cursorrules").exists(),
        "--cursor must not create legacy .cursorrules"
    );
}

#[cfg(unix)]
#[test]
fn init_agent_flag_updates_canonical_target_for_symlinked_entrypoint() {
    // §FS-init.2.1 / §FS-init.2.3: a requested companion symlink to AGENTS.md is
    // covered by updating the canonical target, even when --agents-md was not
    // passed explicitly.
    let target = workdir("init_agent_flag_updates_canonical_target_for_symlinked_entrypoint");
    std::os::unix::fs::symlink("AGENTS.md", target.join("CLAUDE.md"))
        .expect("create CLAUDE.md symlink");

    let output = run_grund(
        &["init", target.to_str().unwrap(), "--claude"],
        manifest_dir(),
    );
    assert!(
        output.status.success(),
        "init failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("wrote AGENTS.md"),
        "init should update the symlink target, got:\n{stderr}"
    );
    assert!(
        stderr.contains("wrote .claude/CLAUDE.md"),
        "explicit --claude should still create the non-symlink Claude entrypoint, got:\n{stderr}"
    );
    assert!(
        !stderr.contains("wrote CLAUDE.md") && !stderr.contains("appended CLAUDE.md"),
        "init should not write through the CLAUDE.md symlink separately, got:\n{stderr}"
    );
    let agents = fs::read_to_string(target.join("AGENTS.md")).expect("read AGENTS.md");
    assert!(
        agents.contains("## Grounding with grund (v5)"),
        "AGENTS.md should receive the managed block:\n{agents}"
    );
}

#[cfg(unix)]
#[test]
fn init_workspace_symlinked_alias_writes_canonical_target() {
    // §FS-init.2.1 / §FS-init.2.3: a workspace-selected companion symlink to
    // AGENTS.md is covered by updating the canonical target, even before the
    // target exists, rather than writing a companion-only block through it.
    let target = workdir("init_workspace_symlinked_alias_writes_canonical_target");
    fs::create_dir_all(target.join(".claude")).expect("create .claude");
    std::os::unix::fs::symlink("AGENTS.md", target.join("CLAUDE.md"))
        .expect("create CLAUDE.md symlink");

    let output = run_grund(&["init", target.to_str().unwrap()], manifest_dir());
    assert!(
        output.status.success(),
        "init failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("wrote AGENTS.md"),
        "init should update the canonical symlink target, got:\n{stderr}"
    );
    assert!(
        stderr.contains("wrote .claude/CLAUDE.md"),
        "init should still create the missing workspace alias, got:\n{stderr}"
    );
    assert!(
        !stderr.contains("wrote CLAUDE.md") && !stderr.contains("appended CLAUDE.md"),
        "init should not write through the CLAUDE.md symlink separately, got:\n{stderr}"
    );

    let claude_metadata =
        fs::symlink_metadata(target.join("CLAUDE.md")).expect("inspect CLAUDE.md");
    assert!(
        claude_metadata.file_type().is_symlink(),
        "CLAUDE.md should remain a symlink"
    );
    let agents = fs::read_to_string(target.join("AGENTS.md")).expect("read AGENTS.md");
    assert!(
        agents.starts_with("# init_workspace_symlinked_alias_writes_canonical_target"),
        "AGENTS.md should be the full canonical entrypoint with an H1, got:\n{agents}"
    );
    let claude_scoped =
        fs::read_to_string(target.join(".claude/CLAUDE.md")).expect("read .claude/CLAUDE.md");
    assert!(
        claude_scoped.starts_with("## Grounding with grund (v5)\n"),
        ".claude/CLAUDE.md should be a thin managed-block alias, got:\n{claude_scoped}"
    );
}

#[test]
fn init_creates_agent_aliases_when_agent_workspaces_exist() {
    // §FS-init.2.1 / §FS-init.2.3: missing neutral companion aliases are created
    // only when their owning agent-specific workspace already exists.
    let target = workdir("init_creates_agent_aliases_when_agent_workspaces_exist");
    fs::create_dir_all(target.join(".claude")).expect("create .claude");
    fs::create_dir_all(target.join(".gemini")).expect("create .gemini");
    fs::create_dir_all(target.join(".github/workflows")).expect("create github metadata");

    let output = run_grund(&["init", target.to_str().unwrap()], manifest_dir());
    assert!(
        output.status.success(),
        "init failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);

    for rel in ["CLAUDE.md", ".claude/CLAUDE.md", "GEMINI.md"] {
        assert!(
            stderr.contains(&format!("wrote {rel}")),
            "init should report writing {rel}, got:\n{stderr}"
        );
        let contents = fs::read_to_string(target.join(rel)).expect("read companion alias");
        assert!(
            contents.starts_with("## Grounding with grund (v5)\n"),
            "{rel} should be a thin managed-block alias, got:\n{contents}"
        );
    }
    assert!(
        !target.join("AGENTS.md").exists(),
        "workspace-triggered aliases should prevent the AGENTS.md fallback"
    );
    assert!(
        !target.join("AGENTS.override.md").exists(),
        "AGENTS.override.md is an override file and should not be created as an alias"
    );
    assert!(
        !target.join(".github/copilot-instructions.md").exists(),
        ".github is generic GitHub metadata and should not create Copilot instructions"
    );

    let second = run_grund(&["init", target.to_str().unwrap()], manifest_dir());
    assert!(second.status.success());
    let second_stderr = String::from_utf8_lossy(&second.stderr);
    assert!(
        second_stderr.contains("exists CLAUDE.md")
            && second_stderr.contains("exists .claude/CLAUDE.md")
            && second_stderr.contains("exists GEMINI.md"),
        "second init should leave workspace-created aliases unchanged, got:\n{second_stderr}"
    );
    assert!(
        !second_stderr.contains(".github/copilot-instructions.md"),
        "second init should not mention absent Copilot instructions, got:\n{second_stderr}"
    );
}

#[test]
fn init_rerun_on_current_repo_writes_nothing_and_reports_exists() {
    // §FS-init.2.2 / §FS-init.2.3: re-running `grund init` on a repo whose managed
    // AGENTS.md block already matches the current render rewrites nothing — the
    // file's bytes are untouched and it is reported with `exists `, not `updated `.
    let target = workdir("init_rerun_on_current_repo_writes_nothing_and_reports_exists");
    let first = run_grund(&["init", target.to_str().unwrap()], manifest_dir());
    assert!(first.status.success());

    let agents_before = fs::read(target.join("AGENTS.md")).unwrap();
    let toml_before = fs::read(target.join(".agents/grund.toml")).unwrap();

    let second = run_grund(&["init", target.to_str().unwrap()], manifest_dir());
    assert!(second.status.success());
    let stderr = String::from_utf8_lossy(&second.stderr);
    assert!(
        stderr.contains("exists AGENTS.md"),
        "second `grund init` should report `exists AGENTS.md`, got:\n{stderr}"
    );
    assert!(
        !stderr.contains("updated AGENTS.md") && !stderr.contains("wrote AGENTS.md"),
        "second `grund init` must not rewrite an already-current AGENTS.md, got:\n{stderr}"
    );
    assert!(
        stderr.contains("exists .agents/grund.toml"),
        "second `grund init` should report `exists .agents/grund.toml`, got:\n{stderr}"
    );

    assert_eq!(
        fs::read(target.join("AGENTS.md")).unwrap(),
        agents_before,
        "AGENTS.md bytes changed on a no-op re-init"
    );
    assert_eq!(
        fs::read(target.join(".agents/grund.toml")).unwrap(),
        toml_before,
        ".agents/grund.toml bytes changed on a no-op re-init"
    );
}

#[test]
fn init_force_never_overwrites_an_existing_config() {
    // §FS-init.2.4 / §FS-init.3: `.agents/grund.toml` is the repo's config, not a
    // scaffold artifact — `grund init --force` regenerates AGENTS.md but leaves an
    // existing config byte-for-byte intact and reports it with `exists `, never
    // `wrote `. (Overwriting it with the canonical template was a footgun.)
    let target = workdir("init_force_never_overwrites_an_existing_config");
    fs::create_dir_all(target.join(".agents")).expect("create .agents");
    let custom_config = "grund_config_version = 1\n\
        project_name = \"Custom\"\n\n\
        [reference]\nstrict = true\n\n\
        [[kinds]]\nprefix = \"SPEC\"\nfolder = \"specs\"\ntitle = \"Spec\"\n";
    fs::write(target.join(".agents/grund.toml"), custom_config).expect("write custom config");

    let output = run_grund(
        &["init", target.to_str().unwrap(), "--force"],
        manifest_dir(),
    );
    assert!(
        output.status.success(),
        "init --force failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("exists .agents/grund.toml"),
        "`grund init --force` must report `exists .agents/grund.toml`, got:\n{stderr}"
    );
    assert!(
        !stderr.contains("wrote .agents/grund.toml"),
        "`grund init --force` must not overwrite an existing config, got:\n{stderr}"
    );
    assert_eq!(
        fs::read_to_string(target.join(".agents/grund.toml")).unwrap(),
        custom_config,
        "`grund init --force` left .agents/grund.toml byte-for-byte? it did not"
    );
}

#[test]
fn init_is_byte_deterministic() {
    // §FS-non-goals.13: same input → byte-identical output.
    let a = workdir("init_is_byte_deterministic_a");
    let b = workdir("init_is_byte_deterministic_b");
    for target in [&a, &b] {
        let out = run_grund(
            &["init", target.to_str().unwrap(), "--name", "Same"],
            manifest_dir(),
        );
        assert!(out.status.success());
    }
    let agents_a = fs::read(a.join("AGENTS.md")).unwrap();
    let agents_b = fs::read(b.join("AGENTS.md")).unwrap();
    assert_eq!(agents_a, agents_b, "AGENTS.md must be byte-identical");
    let toml_a = fs::read(a.join(".agents/grund.toml")).unwrap();
    let toml_b = fs::read(b.join(".agents/grund.toml")).unwrap();
    assert_eq!(toml_a, toml_b, ".agents/grund.toml must be byte-identical");
}

#[test]
fn init_dry_run_writes_no_files_and_reports_would_prefixes() {
    // §FS-init.1 / §FS-init.2.2: --dry-run reports what a real run would do
    // (would-write / would-append / would-update) and leaves the working tree
    // untouched. Re-running without --dry-run then produces the same on-disk
    // outcome as a single non-dry-run would.
    let target = workdir("init_dry_run_writes_no_files_and_reports_would_prefixes");
    let dry = run_grund(
        &["init", target.to_str().unwrap(), "--dry-run"],
        manifest_dir(),
    );
    assert!(
        dry.status.success(),
        "init --dry-run failed: stderr={}",
        String::from_utf8_lossy(&dry.stderr)
    );
    let stderr = String::from_utf8_lossy(&dry.stderr);
    assert!(
        stderr.contains("would-write AGENTS.md")
            && stderr.contains("would-write .agents/grund.toml"),
        "dry-run should report `would-write …` for new files, got:\n{stderr}"
    );
    assert!(
        !stderr.contains("\nwrote ") && !stderr.contains("\nappended "),
        "dry-run must not use the real-run verbs, got:\n{stderr}"
    );
    assert!(
        !target.join("AGENTS.md").exists() && !target.join(".agents/grund.toml").exists(),
        "dry-run must not write anything to disk"
    );

    // Real run on the same target should now write the files cleanly.
    let real = run_grund(&["init", target.to_str().unwrap()], manifest_dir());
    assert!(real.status.success());
    assert!(target.join("AGENTS.md").is_file());
    assert!(target.join(".agents/grund.toml").is_file());
}

#[test]
fn init_dry_run_on_current_repo_suppresses_next_block() {
    // §FS-init.2.2: when every reported path is `exists ` (and no would-… lines
    // were emitted), the `next:` guidance block is suppressed — the user has
    // a complete setup, so there is nothing to teach. This holds for both
    // real runs and dry-runs.
    let target = workdir("init_dry_run_on_current_repo_suppresses_next_block");
    let first = run_grund(&["init", target.to_str().unwrap()], manifest_dir());
    assert!(first.status.success());

    let second = run_grund(&["init", target.to_str().unwrap()], manifest_dir());
    assert!(second.status.success());
    let stderr = String::from_utf8_lossy(&second.stderr);
    assert!(
        stderr.contains("exists AGENTS.md") && stderr.contains("exists .agents/grund.toml"),
        "second init should report `exists` for both managed paths, got:\n{stderr}"
    );
    assert!(
        !stderr.contains("\nnext:") && !stderr.contains("see "),
        "all-exists run must suppress the `next:` block, got:\n{stderr}"
    );

    let dry = run_grund(
        &["init", target.to_str().unwrap(), "--dry-run"],
        manifest_dir(),
    );
    assert!(dry.status.success());
    let dry_stderr = String::from_utf8_lossy(&dry.stderr);
    assert!(
        !dry_stderr.contains("\nnext:") && !dry_stderr.contains("see "),
        "all-exists dry-run must also suppress the `next:` block, got:\n{dry_stderr}"
    );
}

#[test]
fn init_dry_run_with_docs_previews_scaffold_without_writing() {
    // §FS-init.1 / §FS-init.2.2: --dry-run composes with --docs — every docs
    // scaffold path is reported as `would-write` and no file lands on disk.
    let target = workdir("init_dry_run_with_docs_previews_scaffold_without_writing");
    let dry = run_grund(
        &["init", target.to_str().unwrap(), "--docs", "--dry-run"],
        manifest_dir(),
    );
    assert!(
        dry.status.success(),
        "init --docs --dry-run failed: stderr={}",
        String::from_utf8_lossy(&dry.stderr)
    );
    let stderr = String::from_utf8_lossy(&dry.stderr);
    for rel in [
        "AGENTS.md",
        ".agents/grund.toml",
        "docs/grund.md",
        "docs/goals.md",
        "docs/roadmap.md",
        "docs/changelog.md",
        "requirements.md",
        "docs/architecture/README.md",
        "docs/decisions/architectural/.gitkeep",
        "docs/decisions/functional/.gitkeep",
        "e2e/README.md",
        "e2e/cases/.gitkeep",
    ] {
        assert!(
            stderr.contains(&format!("would-write {rel}")),
            "dry-run --docs should preview `would-write {rel}`, got:\n{stderr}"
        );
        assert!(
            !target.join(rel).exists(),
            "dry-run --docs must not write {rel} to disk"
        );
    }
    assert!(
        !stderr.contains("\nwrote "),
        "dry-run --docs must not use the real-run verb, got:\n{stderr}"
    );
}

#[test]
fn init_force_dry_run_previews_canonical_rewrite() {
    // §FS-init.1 / §FS-init.2.2: --force --dry-run takes the rewrite path
    // (instead of update-in-place) and previews `would-write AGENTS.md`
    // without changing the file's bytes on disk. .agents/grund.toml is the
    // exception: --force never overwrites it, so dry-run reports `exists`.
    let target = workdir("init_force_dry_run_previews_canonical_rewrite");
    let first = run_grund(&["init", target.to_str().unwrap()], manifest_dir());
    assert!(first.status.success());

    let agents_before = fs::read(target.join("AGENTS.md")).unwrap();
    let toml_before = fs::read(target.join(".agents/grund.toml")).unwrap();

    let dry = run_grund(
        &["init", target.to_str().unwrap(), "--force", "--dry-run"],
        manifest_dir(),
    );
    assert!(
        dry.status.success(),
        "init --force --dry-run failed: stderr={}",
        String::from_utf8_lossy(&dry.stderr)
    );
    let stderr = String::from_utf8_lossy(&dry.stderr);
    assert!(
        stderr.contains("would-write AGENTS.md"),
        "--force --dry-run should preview the canonical rewrite, got:\n{stderr}"
    );
    assert!(
        stderr.contains("exists .agents/grund.toml"),
        "--force never overwrites the config, even under dry-run, got:\n{stderr}"
    );
    assert!(
        !stderr.contains("\nwrote AGENTS.md"),
        "dry-run must not use the real-run verb, got:\n{stderr}"
    );
    assert_eq!(
        fs::read(target.join("AGENTS.md")).unwrap(),
        agents_before,
        "--force --dry-run must not modify AGENTS.md"
    );
    assert_eq!(
        fs::read(target.join(".agents/grund.toml")).unwrap(),
        toml_before,
        "--force --dry-run must not modify the config"
    );
}

#[test]
fn init_cursor_workspace_creates_cursor_rules_alias() {
    // §FS-init.2.1 / §FS-init.2.3: a present `.cursor/` workspace triggers
    // creation of `.cursor/rules/grund.mdc` in automatic mode — the same
    // pattern that `.claude/` and `.gemini/` use. The legacy `.cursorrules`
    // is never auto-created.
    let target = workdir("init_cursor_workspace_creates_cursor_rules_alias");
    fs::create_dir_all(target.join(".cursor")).expect("create .cursor");

    let output = run_grund(&["init", target.to_str().unwrap()], manifest_dir());
    assert!(
        output.status.success(),
        "init failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("wrote .cursor/rules/grund.mdc"),
        "init should create `.cursor/rules/grund.mdc` when `.cursor/` exists, got:\n{stderr}"
    );
    assert!(
        target.join(".cursor/rules/grund.mdc").is_file(),
        ".cursor/rules/grund.mdc was not written"
    );
    assert!(
        !target.join(".cursorrules").exists(),
        "init must not auto-create legacy .cursorrules; modern path is preferred"
    );
    assert!(
        !target.join("AGENTS.md").exists(),
        "workspace-triggered Cursor alias should prevent the AGENTS.md fallback"
    );
}

#[test]
fn init_zed_rules_is_only_workspace_or_flag_gated() {
    // §FS-init.2.1 / §FS-init.2.3: `.rules` is too generic a filename to
    // attribute to Zed by existence alone — automatic mode must NOT pick it
    // up. Only an explicit `--zed` flag, or a `.zed/` workspace directory,
    // creates or updates `.rules`.
    let target = workdir("init_zed_rules_is_only_workspace_or_flag_gated");
    // Pre-existing `.rules` with no `.zed/` workspace: must be left strictly
    // alone, and the AGENTS.md fallback kicks in instead.
    fs::write(target.join(".rules"), "# Build rules, not Zed\n").expect("write .rules");

    let output = run_grund(&["init", target.to_str().unwrap()], manifest_dir());
    assert!(
        output.status.success(),
        "init failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains(".rules"),
        "automatic mode must not mention `.rules`, got:\n{stderr}"
    );
    assert_eq!(
        fs::read_to_string(target.join(".rules")).unwrap(),
        "# Build rules, not Zed\n",
        "init must not touch a generic .rules file in automatic mode"
    );
    assert!(
        target.join("AGENTS.md").is_file(),
        "no Zed workspace → AGENTS.md is the fallback"
    );

    // Explicit `--zed` opts in.
    let target2 = workdir("init_zed_rules_is_only_workspace_or_flag_gated_explicit");
    let zed_output = run_grund(
        &["init", target2.to_str().unwrap(), "--zed"],
        manifest_dir(),
    );
    assert!(zed_output.status.success());
    let zed_stderr = String::from_utf8_lossy(&zed_output.stderr);
    assert!(
        zed_stderr.contains("wrote .rules"),
        "--zed should create .rules, got:\n{zed_stderr}"
    );
    assert!(target2.join(".rules").is_file());

    // A `.zed/` workspace owns `.rules`, so a second automatic run must keep
    // selecting the existing alias instead of falling back to AGENTS.md.
    let target3 = workdir("init_zed_rules_is_workspace_idempotent");
    fs::create_dir_all(target3.join(".zed")).expect("create .zed");
    let first = run_grund(&["init", target3.to_str().unwrap()], manifest_dir());
    assert!(
        first.status.success(),
        "init failed: stderr={}",
        String::from_utf8_lossy(&first.stderr)
    );
    let first_stderr = String::from_utf8_lossy(&first.stderr);
    assert!(
        first_stderr.contains("wrote .rules"),
        "Zed workspace should create .rules, got:\n{first_stderr}"
    );

    let second = run_grund(&["init", target3.to_str().unwrap()], manifest_dir());
    assert!(
        second.status.success(),
        "second init failed: stderr={}",
        String::from_utf8_lossy(&second.stderr)
    );
    let second_stderr = String::from_utf8_lossy(&second.stderr);
    assert!(
        second_stderr.contains("exists .rules"),
        "second init should select existing .rules, got:\n{second_stderr}"
    );
    assert!(
        !second_stderr.contains("AGENTS.md") && !target3.join("AGENTS.md").exists(),
        "second init must not fall back to AGENTS.md, got:\n{second_stderr}"
    );
}

#[test]
fn init_preserves_lone_override_entrypoint_without_creating_agents_md() {
    // §FS-init.2.1 / §FS-init.2.3: AGENTS.override.md is the "automatic
    // existing-file-only" override channel. When it is the only known agent
    // entrypoint present, automatic mode treats it as the existing repo's
    // choice — its managed block is appended/updated and no canonical
    // AGENTS.md is created. This locks in the behavior of the
    // existing-companion branch in `selected_init_agent_entrypoints` so a
    // future refactor cannot silently regress an adopter who is running
    // `init` against a Codex-style override-only layout.
    let target = workdir("init_preserves_lone_override_entrypoint_without_creating_agents_md");
    fs::write(target.join("AGENTS.override.md"), "# Local override\n")
        .expect("write AGENTS.override.md");

    let output = run_grund(&["init", target.to_str().unwrap()], manifest_dir());
    assert!(
        output.status.success(),
        "init failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("appended AGENTS.override.md"),
        "init should append managed block to lone AGENTS.override.md, got:\n{stderr}"
    );
    assert!(
        !target.join("AGENTS.md").exists(),
        "init must not create canonical AGENTS.md when only the override file is present"
    );
    assert!(
        !stderr.contains("wrote AGENTS.md") && !stderr.contains("appended AGENTS.md"),
        "stderr should not mention canonical AGENTS.md, got:\n{stderr}"
    );
    assert!(
        stderr.contains("see AGENTS.override.md for the full workflow."),
        "next block should point at the selected entrypoint, got:\n{stderr}"
    );

    let override_contents =
        fs::read_to_string(target.join("AGENTS.override.md")).expect("read override file");
    assert!(
        override_contents.starts_with("# Local override\n\n## Grounding with grund (v5)\n"),
        "AGENTS.override.md should keep existing notes and append the managed block:\n{override_contents}"
    );
}

#[test]
fn init_output_passes_check_when_agents_md_is_scanned() {
    // §FS-init.2.3: generated output must pass `grund check` unmodified, even
    // when the entrypoint itself is inside the scan scope of a strict repo —
    // the worked citation example is `<§>`-escaped, not a live dangling
    // reference (the grund init → grund check → grund init wedge of the
    // pre-v4 template).
    let target = workdir("init_output_passes_check_when_agents_md_is_scanned");
    fs::create_dir_all(target.join(".agents")).expect("create .agents");
    fs::write(
        target.join(".agents/grund.toml"),
        r#"grund_config_version = 1

[reference]
strict = true

[scan]
include = ["AGENTS.md", "docs"]

[[kinds]]
prefix = "FS"
folder = "docs/functional-spec"
title = "Spec"
"#,
    )
    .expect("write config");
    fs::create_dir_all(target.join("docs")).expect("create docs");
    fs::write(target.join("docs/.keep"), "").expect("write docs/.keep");

    let output = run_grund(&["init", target.to_str().unwrap()], manifest_dir());
    assert!(
        output.status.success(),
        "init failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let check = run_grund(&["check", target.to_str().unwrap()], manifest_dir());
    assert!(
        check.status.success(),
        "freshly generated AGENTS.md must pass check unmodified:\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );

    // Re-running init must also be a no-op, not a refresh loop.
    let second = run_grund(&["init", target.to_str().unwrap()], manifest_dir());
    assert!(second.status.success());
    let stderr = String::from_utf8_lossy(&second.stderr);
    assert!(
        stderr.contains("exists AGENTS.md"),
        "second init should be a no-op, got:\n{stderr}"
    );
}
