//! §FS-init.1 / §FS-init.2.1.1 — the explicit agent flags: `--claude`,
//! `--cursor`, and their siblings create or update exactly the entrypoints they
//! name, one per agent, and never a second file for an agent that already has
//! one. The cases that need a symlinked companion live here too: an explicit
//! request is the only thing that writes past one, and only where the committed
//! `conversation = "link"` opinion makes the two files differ (§FS-init.2.3.4.17).
//!
//! The automatic-mode half is `init_agent_entrypoints.rs`; both build their
//! targets with the shared fixture in `support/init_fixture.rs`.

use std::fs;

#[path = "support/init_fixture.rs"]
mod init_fixture;

use init_fixture::{manifest_dir, run_grund, workdir};

/// §FS-init.1 / §FS-init.2.1: explicit agent flags create exactly the requested
/// entrypoint families and do not add the automatic AGENTS.md fallback.
/// §FS-init.2.1.1: one entrypoint per agent — Claude reads two files and gets
/// the root-visible one, not both.
#[test]
fn init_agent_flags_create_requested_entrypoints() {
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

/// §FS-init.2.1 / §FS-init.2.3: explicit --cursor updates legacy .cursorrules
/// when it already exists, and never creates the legacy file for new adopters.
/// §FS-init.2.1.1: Cursor reads both rule surfaces, so the repo's existing one
/// is updated rather than a second one added beside it.
#[test]
fn init_cursor_flag_updates_existing_legacy_cursorrules() {
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
        legacy.starts_with("# Legacy Cursor notes\n\n<!-- BEGIN GRUND MANAGED BLOCK -->\n## Grounding with grund (v8)\n"),
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

/// §FS-init.2.1 / §FS-init.2.3: a requested companion symlink to AGENTS.md is
/// covered by updating the canonical target, even when --agents-md was not
/// passed explicitly — and §FS-init.2.1.1: covered means covered, so nothing
/// is created beside it while the block both files would carry is the same.
#[cfg(unix)]
#[test]
fn init_agent_flag_updates_canonical_target_for_symlinked_entrypoint() {
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
        !target.join(".claude").exists(),
        "the symlink already feeds Claude this block; a second file would be the same bytes twice"
    );
    assert!(
        !stderr.contains("wrote CLAUDE.md") && !stderr.contains("appended CLAUDE.md"),
        "init should not write through the CLAUDE.md symlink separately, got:\n{stderr}"
    );
    let agents = fs::read_to_string(target.join("AGENTS.md")).expect("read AGENTS.md");
    assert!(
        agents.contains("## Grounding with grund (v8)"),
        "AGENTS.md should receive the managed block:\n{agents}"
    );
}

/// §FS-init.2.1.1 / §FS-init.2.3.4.17: the one case where a symlinked
/// companion leaves its agent uncovered. With `conversation = "link"` the
/// canonical file carries the plain form — the one Claude is not meant to
/// read — so `--claude` writes the Claude entrypoint the symlink left free,
/// and that file is the only one carrying the linked sentence.
#[cfg(unix)]
#[test]
fn init_claude_flag_writes_the_real_entrypoint_a_link_repo_symlinked_away() {
    let target = workdir("init_claude_flag_writes_the_real_entrypoint_a_link_repo_symlinked_away");
    fs::write(
        target.join("grund.toml"),
        "grund_config_version = 1\nproject_name = \"demo\"\n\n[reference]\nconversation = \"link\"\n",
    )
    .expect("write config");
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
        stderr.contains("wrote .claude/CLAUDE.md"),
        "--claude should write the Claude entrypoint the symlink left free, got:\n{stderr}"
    );
    assert!(
        stderr.contains(
            "note: CLAUDE.md is a symlink to AGENTS.md, so Claude reads the plain-location form from there as well as the linked form from .claude/CLAUDE.md; delete the symlink to leave .claude/CLAUDE.md as Claude's only entrypoint"
        ),
        "the note should name the fix left open once the entrypoint exists, got:\n{stderr}"
    );

    let scoped =
        fs::read_to_string(target.join(".claude/CLAUDE.md")).expect("read .claude/CLAUDE.md");
    assert!(
        scoped.contains("as a Markdown link whose visible text is the citation itself"),
        ".claude/CLAUDE.md should carry the linked form:\n{scoped}"
    );
    let agents = fs::read_to_string(target.join("AGENTS.md")).expect("read AGENTS.md");
    assert!(
        !agents.contains("as a Markdown link whose visible text is the citation itself"),
        "the canonical file keeps the plain form, which is why the second file exists:\n{agents}"
    );
}

/// §FS-init.2.1.1 / §FS-init.2.3.4.17: both of Claude's paths symlinked to
/// AGENTS.md under the `link` opinion. There is no free path, so `--claude`
/// writes nothing for Claude — and the note has to name the fix that is
/// actually left rather than the command the user just ran.
#[cfg(unix)]
#[test]
fn init_reports_a_symlink_pair_that_leaves_claude_nowhere_to_write() {
    let target = workdir("init_reports_a_symlink_pair_that_leaves_claude_nowhere_to_write");
    fs::write(
        target.join("grund.toml"),
        "grund_config_version = 1\nproject_name = \"demo\"\n\n[reference]\nconversation = \"link\"\n",
    )
    .expect("write config");
    fs::create_dir_all(target.join(".claude")).expect("create .claude");
    std::os::unix::fs::symlink("AGENTS.md", target.join("CLAUDE.md")).expect("symlink CLAUDE.md");
    std::os::unix::fs::symlink("../AGENTS.md", target.join(".claude/CLAUDE.md"))
        .expect("symlink .claude/CLAUDE.md");

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
        stderr.contains(
            "note: CLAUDE.md and .claude/CLAUDE.md are symlinks to AGENTS.md, so Claude reads the plain-location form, and the symlinks have taken every path a real Claude entrypoint could go; delete one of them and re-run `grund init --claude`"
        ),
        "the note should name the symlinks and the only fix left, got:\n{stderr}"
    );
    assert_eq!(
        stderr.matches("note:").count(),
        1,
        "one situation, one note — the generic duplication line would advise the wrong deletion here, got:\n{stderr}"
    );
}

#[test]
fn init_dry_run_reports_a_duplicated_entrypoint_in_the_conditional() {
    // §FS-init.2.2: a preview writes nothing, so its note describes what the run
    // would do — the files it names may not carry a block at all yet.
    let target = workdir("init_dry_run_reports_a_duplicated_entrypoint_in_the_conditional");
    fs::write(target.join("CLAUDE.md"), "# root notes\n").expect("write CLAUDE.md");
    fs::create_dir_all(target.join(".claude")).expect("create .claude");
    fs::write(target.join(".claude/CLAUDE.md"), "# project notes\n")
        .expect("write .claude/CLAUDE.md");

    let output = run_grund(
        &["init", target.to_str().unwrap(), "--claude", "--dry-run"],
        manifest_dir(),
    );
    assert!(
        output.status.success(),
        "init failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "note: CLAUDE.md and .claude/CLAUDE.md would both carry the managed block, so Claude would read it twice;"
        ),
        "the preview's note should be in the conditional, got:\n{stderr}"
    );
}

/// §FS-init.2.1.1 / §FS-init.2.3.4.17: whether Claude has an entrypoint of
/// its own is a fact about the tree, not about which flag this run carries.
/// A run that selected some other agent has not changed it, so it must reach
/// the same diagnosis a flagless run does — naming the real entrypoint and
/// the symlink to delete, never claiming there is nowhere left to write.
#[cfg(unix)]
#[test]
fn init_symlink_note_names_the_entrypoint_claude_already_has() {
    let target = workdir("init_symlink_note_names_the_entrypoint_claude_already_has");
    fs::write(
        target.join("grund.toml"),
        "grund_config_version = 1\nproject_name = \"demo\"\n\n[reference]\nconversation = \"link\"\n",
    )
    .expect("write config");
    fs::write(target.join("AGENTS.md"), "# demo\n").expect("write AGENTS.md");
    fs::create_dir_all(target.join(".claude")).expect("create .claude");
    fs::write(target.join(".claude/CLAUDE.md"), "# project notes\n")
        .expect("write .claude/CLAUDE.md");
    std::os::unix::fs::symlink("AGENTS.md", target.join("CLAUDE.md")).expect("symlink CLAUDE.md");

    for flags in [&["--gemini"][..], &["--agents-md"][..]] {
        let mut args = vec!["init", target.to_str().unwrap()];
        args.extend_from_slice(flags);
        let output = run_grund(&args, manifest_dir());
        assert!(
            output.status.success(),
            "init {flags:?} failed: stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("the linked form from .claude/CLAUDE.md"),
            "{flags:?}: the note must name the entrypoint Claude already has, got:\n{stderr}"
        );
        assert!(
            !stderr.contains("taken every path"),
            "{flags:?}: Claude has a real entrypoint, so no path is exhausted, got:\n{stderr}"
        );
        assert!(
            !stderr.contains("run `grund init --claude` to write"),
            "{flags:?}: the entrypoint exists — writing it is not the open fix, got:\n{stderr}"
        );
    }
}

/// §FS-init.2.2: a preview writes nothing, so the symlink may only go once
/// the entrypoint *carries the block*. Keying the conditional on the file
/// existing is the same sentence with a precondition the reader can already
/// satisfy — they check, see the file, delete their only Claude entrypoint,
/// and are left with one that says nothing.
#[cfg(unix)]
#[test]
fn init_dry_run_symlink_note_waits_for_the_block_not_the_file() {
    let target = workdir("init_dry_run_symlink_note_waits_for_the_block_not_the_file");
    fs::write(
        target.join("grund.toml"),
        "grund_config_version = 1\nproject_name = \"demo\"\n\n[reference]\nconversation = \"link\"\n",
    )
    .expect("write config");
    fs::write(target.join("AGENTS.md"), "# demo\n").expect("write AGENTS.md");
    fs::create_dir_all(target.join(".claude")).expect("create .claude");
    fs::write(target.join(".claude/CLAUDE.md"), "# project notes\n")
        .expect("write .claude/CLAUDE.md");
    std::os::unix::fs::symlink("AGENTS.md", target.join("CLAUDE.md")).expect("symlink CLAUDE.md");

    let output = run_grund(
        &["init", target.to_str().unwrap(), "--claude", "--dry-run"],
        manifest_dir(),
    );
    assert!(
        output.status.success(),
        "init failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("would-append .claude/CLAUDE.md"),
        "the fixture needs an entrypoint the run appends to, not creates, got:\n{stderr}"
    );
    assert!(
        stderr.contains(
            "once .claude/CLAUDE.md carries the block, delete the symlink to leave it as Claude's only entrypoint"
        ),
        "the preview's condition is the block, not the file, got:\n{stderr}"
    );
    assert!(
        !stderr.contains("once .claude/CLAUDE.md exists"),
        "the file already exists — that condition is satisfied on sight, got:\n{stderr}"
    );
}
