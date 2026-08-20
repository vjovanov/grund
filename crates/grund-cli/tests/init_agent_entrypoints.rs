//! §FS-init.2.1 / §FS-init.2.1.1 — which agent entrypoint files one `grund init`
//! run selects: the existing ones it updates, the one it creates for an agent
//! that has none, and the companion symlink that stands for the canonical file
//! instead of the agent's own.
//!
//! Split out of `init.rs`, which keeps the scaffold-and-report cases; both build
//! their targets with the shared fixture in `support/init_fixture.rs`.

use std::fs;

#[path = "support/init_fixture.rs"]
mod init_fixture;

use init_fixture::{manifest_dir, run_grund, workdir};

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
        claude.starts_with(
            "# Claude notes\n\n<!-- BEGIN GRUND MANAGED BLOCK -->\n## Grounding with grund (v7)\n"
        ),
        "CLAUDE.md should keep existing notes and append the managed block:\n{claude}"
    );
}

#[test]
fn init_agent_flags_create_requested_entrypoints() {
    // §FS-init.1 / §FS-init.2.1: explicit agent flags create exactly the requested
    // entrypoint families and do not add the automatic AGENTS.md fallback.
    // §FS-init.2.1.1: one entrypoint per agent — Claude reads two files and gets
    // the root-visible one, not both.
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
    for rel in ["CLAUDE.md", "GEMINI.md", ".github/copilot-instructions.md"] {
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
        !target.join(".claude").exists(),
        "--claude should write one Claude entrypoint, not a second under .claude/"
    );
    assert!(
        !stderr.contains(".claude/CLAUDE.md"),
        "stderr should not mention a second Claude entrypoint, got:\n{stderr}"
    );
    assert!(
        !target.join("AGENTS.md").exists(),
        "explicit companion-agent init should not add AGENTS.md"
    );
}

#[test]
fn init_claude_flag_updates_the_entrypoint_the_repo_already_has() {
    // §FS-init.2.1.1: the block is the same bytes in both Claude entrypoints, so
    // a repo that has one keeps one — `--claude` updates it and creates nothing
    // beside it, on the first run and on every re-run.
    let target = workdir("init_claude_flag_updates_the_entrypoint_the_repo_already_has");
    fs::write(target.join("CLAUDE.md"), "# Claude notes\n").expect("write CLAUDE.md");

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
        stderr.contains("appended CLAUDE.md"),
        "--claude should append to the existing Claude entrypoint, got:\n{stderr}"
    );
    assert!(
        !target.join(".claude/CLAUDE.md").exists() && !stderr.contains(".claude/CLAUDE.md"),
        "--claude must not create a second Claude entrypoint, got:\n{stderr}"
    );

    let second = run_grund(
        &["init", target.to_str().unwrap(), "--claude", "--dry-run"],
        manifest_dir(),
    );
    assert!(second.status.success());
    let second_stderr = String::from_utf8_lossy(&second.stderr);
    assert!(
        second_stderr.contains("exists CLAUDE.md") && !second_stderr.contains(".claude/CLAUDE.md"),
        "a --dry-run re-run should preview one current entrypoint, got:\n{second_stderr}"
    );
}

#[test]
fn init_claude_flag_reports_a_repo_that_carries_both_entrypoints() {
    // §FS-init.2.1.1: `init` maintains what it finds, so a repo that already has
    // both Claude entrypoints keeps both — and hears that the block reaches
    // Claude twice, since this run is the only place that is visible.
    let target = workdir("init_claude_flag_reports_a_repo_that_carries_both_entrypoints");
    fs::write(target.join("CLAUDE.md"), "# Claude notes\n").expect("write CLAUDE.md");
    fs::create_dir_all(target.join(".claude")).expect("create .claude");
    fs::write(target.join(".claude/CLAUDE.md"), "# Claude project notes\n")
        .expect("write .claude/CLAUDE.md");

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
        stderr.contains("appended CLAUDE.md") && stderr.contains("appended .claude/CLAUDE.md"),
        "both existing Claude entrypoints should be updated, got:\n{stderr}"
    );
    assert!(
        stderr.contains(
            "note: CLAUDE.md and .claude/CLAUDE.md both carry the managed block, so Claude reads it twice;"
        ),
        "the duplicated entrypoint should be reported, got:\n{stderr}"
    );
}

#[test]
fn init_cursor_flag_updates_existing_legacy_cursorrules() {
    // §FS-init.2.1 / §FS-init.2.3: explicit --cursor updates legacy .cursorrules
    // when it already exists, and never creates the legacy file for new adopters.
    // §FS-init.2.1.1: Cursor reads both rule surfaces, so the repo's existing one
    // is updated rather than a second one added beside it.
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
        !target.join(".cursor").exists() && !stderr.contains(".cursor/rules/grund.mdc"),
        "--cursor should not add a second Cursor entrypoint beside .cursorrules, got:\n{stderr}"
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
        legacy.starts_with("# Legacy Cursor notes\n\n<!-- BEGIN GRUND MANAGED BLOCK -->\n## Grounding with grund (v7)\n"),
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
        agents.contains("## Grounding with grund (v7)"),
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
        claude_scoped
            .starts_with("<!-- BEGIN GRUND MANAGED BLOCK -->\n## Grounding with grund (v7)\n"),
        ".claude/CLAUDE.md should be a thin managed-block alias, got:\n{claude_scoped}"
    );
}

#[test]
fn init_creates_agent_aliases_when_agent_workspaces_exist() {
    // §FS-init.2.1 / §FS-init.2.3: missing neutral companion aliases are created
    // only when their owning agent-specific workspace already exists — one per
    // agent (§FS-init.2.1.1), so `.claude/` yields `CLAUDE.md` alone.
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

    for rel in ["CLAUDE.md", "GEMINI.md"] {
        assert!(
            stderr.contains(&format!("wrote {rel}")),
            "init should report writing {rel}, got:\n{stderr}"
        );
        let contents = fs::read_to_string(target.join(rel)).expect("read companion alias");
        assert!(
            contents
                .starts_with("<!-- BEGIN GRUND MANAGED BLOCK -->\n## Grounding with grund (v7)\n"),
            "{rel} should be a thin managed-block alias, got:\n{contents}"
        );
    }
    assert!(
        !target.join(".claude/CLAUDE.md").exists(),
        ".claude/ proves Claude is in use once, and CLAUDE.md is the alias it creates"
    );
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
        second_stderr.contains("exists CLAUDE.md") && second_stderr.contains("exists GEMINI.md"),
        "second init should leave workspace-created aliases unchanged, got:\n{second_stderr}"
    );
    assert!(
        !second_stderr.contains(".claude/CLAUDE.md"),
        "second init should not grow the alias it did not create, got:\n{second_stderr}"
    );
    assert!(
        !second_stderr.contains(".github/copilot-instructions.md"),
        "second init should not mention absent Copilot instructions, got:\n{second_stderr}"
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
        override_contents.starts_with(
            "# Local override\n\n<!-- BEGIN GRUND MANAGED BLOCK -->\n## Grounding with grund (v7)\n"
        ),
        "AGENTS.override.md should keep existing notes and append the managed block:\n{override_contents}"
    );
}
