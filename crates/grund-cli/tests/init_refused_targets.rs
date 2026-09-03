//! §FS-init.1.2 — the targets `grund init` declines to scaffold into, and the
//! one flag that lifts the rule that has a legitimate other side.
//!
//! These cases live outside `tests/init.rs` and outside the `tests/e2e/cases` corpus
//! because both run inside this repository's own git tree, where the
//! version-control rule can never fire: the condition under test is the state
//! every other init fixture takes for granted. Each case here therefore builds
//! its target under the system temp directory and, for the home-directory rule,
//! points the child's home directory at it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The personal file from the issue report: three lines that have nothing to do
/// with any project, which a refused run must leave byte-for-byte alone.
const PERSONAL_INSTRUCTIONS: &str = "# Personal instructions\n\nNothing to do with any project.\n";

/// The version-control markers the rule under test walks for (§FS-init.1.2),
/// spelled here rather than imported because these cases exercise the binary.
const VCS_MARKERS: [&str; 4] = [".git", ".hg", ".jj", ".svn"];

/// The nearest marker in `dir` or any ancestor, or `None` when the walk reaches
/// the filesystem root without finding one — the same walk `init` performs, so
/// the fixture can tell whether it is actually outside version control rather
/// than only checking its own directory.
fn vcs_marker_at_or_above(dir: &Path) -> Option<PathBuf> {
    let mut current = Some(dir);
    while let Some(dir) = current {
        for marker in VCS_MARKERS {
            let candidate = dir.join(marker);
            if candidate.exists() {
                return Some(candidate);
            }
        }
        current = dir.parent();
    }
    None
}

/// The temp root these fixtures build under: one no marker covers, *including
/// through its ancestors*, since that walk-up is the rule under test. The
/// system temp directory is the first candidate and normally the only one, but
/// `TMPDIR` pointing inside a checkout is common enough on CI images and
/// developer machines that a second candidate is worth having — with it, the
/// version-control cases keep testing the rule instead of silently inverting
/// into failures about the fixture.
///
/// The path is per-process: every case here starts by removing its directory,
/// so a shared fixed path would let two concurrent runs on one machine delete
/// each other's targets mid-run.
fn fixture_root() -> PathBuf {
    let system = std::env::temp_dir();
    // The second candidate exists only where the platform has a well-known temp
    // root to fall back to; elsewhere the list is the system one alone.
    #[cfg(unix)]
    let fallbacks = [PathBuf::from("/tmp")];
    #[cfg(not(unix))]
    let fallbacks: [PathBuf; 0] = [];
    let candidates: Vec<PathBuf> = std::iter::once(system.clone())
        .chain(fallbacks.into_iter().filter(|path| *path != system))
        .collect();
    let reasons: Vec<String> = candidates
        .iter()
        .filter_map(|candidate| {
            vcs_marker_at_or_above(candidate)
                .map(|marker| format!("{} is covered by {}", candidate.display(), marker.display()))
        })
        .collect();
    let root = candidates
        .into_iter()
        .find(|candidate| vcs_marker_at_or_above(candidate).is_none())
        .unwrap_or_else(|| {
            panic!(
                "these cases need a temp root outside every version-controlled tree, because that \
                 is the condition under test; point TMPDIR at one ({})",
                reasons.join("; ")
            )
        });
    root.join(format!("grund-init-refused-targets-{}", std::process::id()))
}

/// A fresh directory outside every version-controlled tree — deliberately *not*
/// under `target/`, which is inside this repository's git tree and would satisfy
/// the version-control rule through the walk-up.
fn outside_repo_dir(suffix: &str) -> PathBuf {
    let dir = fixture_root().join(suffix);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create target outside the repo");
    assert!(
        vcs_marker_at_or_above(&dir).is_none(),
        "fixture root must not be version-controlled: {}",
        dir.display()
    );
    dir
}

/// Run `grund init`, with the child's home directory pointed at `home`. Both
/// spellings are set because `init` resolves the home directory the way the
/// platform reports it (§FS-init.1.2): Unix reads `$HOME`, Windows
/// `%USERPROFILE%`, and a case that set only one would pass on the platform it
/// was written on and quietly test nothing on the other.
fn run_init(args: &[&str], home: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_grund"));
    command.arg("init").args(args);
    if let Some(home) = home {
        command.env("HOME", home).env("USERPROFILE", home);
        // `~/.config/zed/AGENTS.md` in the user-global table resolves through `$XDG_CONFIG_HOME`
        // when it is set (§FS-integrations.4), so a machine that sets it would send that row
        // somewhere the fixture home is not; clearing it restores every row to the home just set.
        command.env_remove("XDG_CONFIG_HOME");
    }
    command.output().expect("spawn grund")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

#[test]
fn home_directory_target_is_refused_and_leaves_the_personal_file_alone() {
    // The reported accident: a shell slip leaves the user in `$HOME`, and
    // `--claude` appends the managed block to the machine-global instruction
    // file every session in every project loads (§FS-init.1.2).
    let home = outside_repo_dir("home_target");
    fs::create_dir_all(home.join(".claude")).expect("create .claude");
    fs::write(home.join(".claude/CLAUDE.md"), PERSONAL_INSTRUCTIONS).expect("write personal file");

    let output = run_init(&["--claude", home.to_str().unwrap()], Some(&home));

    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    let message = stderr(&output);
    assert!(
        message.contains("refusing to scaffold into the home directory"),
        "the home rule must be the one that answers, not the version-control rule \
         that also applies to this target: {message}"
    );
    assert_eq!(
        fs::read_to_string(home.join(".claude/CLAUDE.md")).expect("read personal file"),
        PERSONAL_INSTRUCTIONS,
        "a refused run must not append the managed block"
    );
    // A refusal is total — not even the files that would have been fine.
    assert!(!home.join("CLAUDE.md").exists(), "CLAUDE.md was written");
    assert!(!home.join("grund.toml").exists(), "grund.toml was written");
}

#[test]
fn home_directory_refusal_is_lifted_by_no_flag() {
    let home = outside_repo_dir("home_unconditional");
    for flags in [
        vec!["--no-vcs"],
        vec!["--force"],
        vec!["--force", "--no-vcs"],
        vec!["--dry-run", "--no-vcs"],
        vec!["--check", "--no-vcs"],
    ] {
        let mut args = flags.clone();
        args.push(home.to_str().unwrap());
        let output = run_init(&args, Some(&home));
        assert_eq!(
            output.status.code(),
            Some(2),
            "{flags:?} must not lift the home rule: {}",
            stderr(&output)
        );
        assert!(
            stderr(&output).contains("refusing to scaffold into the home directory"),
            "{flags:?}: {}",
            stderr(&output)
        );
    }
    assert!(!home.join("AGENTS.md").exists(), "AGENTS.md was written");
    assert!(!home.join("grund.toml").exists(), "grund.toml was written");
}

#[test]
fn target_outside_version_control_is_refused_until_no_vcs() {
    let target = outside_repo_dir("no_vcs");
    // Not $HOME — only the version-control rule applies here.
    let home = outside_repo_dir("no_vcs_home");

    let refused = run_init(&[target.to_str().unwrap()], Some(&home));
    assert_eq!(refused.status.code(), Some(2), "{}", stderr(&refused));
    assert!(
        stderr(&refused).contains("is not inside a version-controlled tree")
            && stderr(&refused).contains("pass --no-vcs to scaffold anyway"),
        "the refusal must name the flag that proceeds: {}",
        stderr(&refused)
    );
    assert!(
        !target.join("AGENTS.md").exists() && !target.join("grund.toml").exists(),
        "a refused run wrote files"
    );

    let allowed = run_init(&["--no-vcs", target.to_str().unwrap()], Some(&home));
    assert_eq!(allowed.status.code(), Some(0), "{}", stderr(&allowed));
    assert!(target.join("AGENTS.md").is_file(), "AGENTS.md not written");
    assert!(
        target.join("grund.toml").is_file(),
        "grund.toml not written"
    );
}

#[test]
fn dry_run_reports_the_refusal_rather_than_a_preview() {
    let target = outside_repo_dir("dry_run");
    let home = outside_repo_dir("dry_run_home");

    let output = run_init(&["--dry-run", target.to_str().unwrap()], Some(&home));

    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    let message = stderr(&output);
    assert!(
        message.contains("is not inside a version-controlled tree"),
        "{message}"
    );
    assert!(
        !message.contains("would-write"),
        "a preview of a run that would be refused is that refusal: {message}"
    );
}

#[test]
fn check_reports_the_refusal_and_keeps_its_exit_code() {
    // A refusal is not a finding: `2` still wins over the `1` `--check` earns
    // for a pending change (§FS-init.4).
    let target = outside_repo_dir("check_refused");
    let home = outside_repo_dir("check_refused_home");

    let output = run_init(&["--check", target.to_str().unwrap()], Some(&home));

    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    let message = stderr(&output);
    assert!(
        message.contains("is not inside a version-controlled tree"),
        "{message}"
    );
    assert!(
        !message.contains("would-write"),
        "a gate over a run that would be refused is that refusal: {message}"
    );
    assert!(
        !target.join("AGENTS.md").exists() && !target.join("grund.toml").exists(),
        "a refused run wrote files"
    );
}

#[test]
fn a_marker_anywhere_above_the_target_satisfies_the_rule() {
    let root = outside_repo_dir("marker_above");
    let home = outside_repo_dir("marker_above_home");
    // A linked worktree and a submodule both write `.git` as a *file*, so the
    // rule tests presence rather than type (§FS-init.1.2).
    fs::write(root.join(".git"), "gitdir: /elsewhere\n").expect("write .git file");
    let nested = root.join("packages/service");
    fs::create_dir_all(&nested).expect("create nested target");

    let output = run_init(&[nested.to_str().unwrap()], Some(&home));

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(nested.join("AGENTS.md").is_file(), "AGENTS.md not written");
}

#[test]
fn each_supported_marker_satisfies_the_rule() {
    let home = outside_repo_dir("markers_home");
    for marker in [".git", ".hg", ".jj", ".svn"] {
        let target = outside_repo_dir(&format!("marker{marker}"));
        fs::create_dir_all(target.join(marker)).expect("create marker");
        let output = run_init(&[target.to_str().unwrap()], Some(&home));
        assert_eq!(
            output.status.code(),
            Some(0),
            "{marker} should satisfy the rule: {}",
            stderr(&output)
        );
    }
}

/// §FS-init.1.2 — the user-global instruction rule, reached the way a user
/// reaches it. The home rule cannot answer here: the target is a directory
/// *inside* the home directory, not the home directory itself. Neither can the
/// version-control rule, because dotfiles under `git` are the usual state of a
/// machine that has these directories at all — so this rule is alone with the
/// decision, and the file it protects is the same machine-global one.
///
/// Unix only: that table is `~`-rooted and resolved through `$HOME`
/// (§FS-integrations.4), the spelling a Windows runner may not set.
///
/// Why one row of that table is missing below:
/// `~/.copilot/copilot-instructions.md` is absent because `init` writes that
/// basename under `.github/` only, so no `<path>` names it.
#[cfg(unix)]
#[test]
fn a_user_global_instruction_file_is_refused_though_the_target_is_not_home() {
    let home = outside_repo_dir("global_files");
    fs::create_dir_all(home.join(".git")).expect("create the dotfiles marker");
    // Every row of §FS-integrations.4.3 that an `init` target can produce. Four of
    // the five are the canonical `AGENTS.md`, the entrypoint `init` reaches for by
    // default, which carries no agent's name to warn anyone off.
    for (dir, file) in [
        (".codex", "AGENTS.md"),
        (".config/zed", "AGENTS.md"),
        (".pi/agent", "AGENTS.md"),
        (".gemini", "GEMINI.md"),
        (".claude", "CLAUDE.md"),
    ] {
        let target = home.join(dir);
        fs::create_dir_all(&target).expect("create the user-global directory");
        fs::write(target.join(file), PERSONAL_INSTRUCTIONS).expect("write personal file");

        let output = run_init(&[target.to_str().unwrap()], Some(&home));

        assert_eq!(
            output.status.code(),
            Some(2),
            "{dir}/{file}: {}",
            stderr(&output)
        );
        let message = stderr(&output);
        assert!(
            message.contains("machine-global agent instruction file")
                && message.contains("grund integrations --write"),
            "{dir}/{file} must be refused by the user-global rule, naming the command \
             that owns the file: {message}"
        );
        assert_eq!(
            fs::read_to_string(target.join(file)).expect("read personal file"),
            PERSONAL_INSTRUCTIONS,
            "{dir}/{file}: a refused run must not touch the machine-global file"
        );
        assert!(
            !target.join("grund.toml").exists(),
            "{dir}/{file}: a refused run wrote a config"
        );
    }
}

#[cfg(unix)]
#[test]
fn no_flag_lifts_the_user_global_rule_and_none_rewrites_the_file() {
    let home = outside_repo_dir("global_force");
    fs::create_dir_all(home.join(".git")).expect("create the dotfiles marker");
    let target = home.join(".codex");
    fs::create_dir_all(&target).expect("create the user-global directory");
    let global = target.join("AGENTS.md");
    fs::write(&global, PERSONAL_INSTRUCTIONS).expect("write personal file");

    // `--force` is the worst of these: canonical `AGENTS.md` is the one file
    // `init` *overwrites* rather than appends to (§FS-init.3), so a run that got
    // this far would leave nothing behind to hand-remove.
    for flags in [
        vec!["--agents-md"],
        vec!["--force"],
        vec!["--force", "--no-vcs"],
        vec!["--dry-run"],
        vec!["--no-vcs"],
    ] {
        let mut args = flags.clone();
        args.push(target.to_str().unwrap());
        let output = run_init(&args, Some(&home));
        assert_eq!(
            output.status.code(),
            Some(2),
            "{flags:?} must not lift the user-global rule: {}",
            stderr(&output)
        );
        assert!(
            stderr(&output).contains("machine-global agent instruction file"),
            "{flags:?}: {}",
            stderr(&output)
        );
        assert_eq!(
            fs::read_to_string(&global).expect("read personal file"),
            PERSONAL_INSTRUCTIONS,
            "{flags:?} rewrote the machine-global file"
        );
    }
}

#[cfg(unix)]
#[test]
fn the_user_global_rule_refuses_to_create_the_file_too() {
    // The rule is about the path, not about what is at it: a machine that has
    // `~/.pi/agent/` but has never written the instruction file must not get one
    // from `init` either (§FS-init.1.2).
    let home = outside_repo_dir("global_missing");
    fs::create_dir_all(home.join(".git")).expect("create the dotfiles marker");
    let target = home.join(".pi/agent");
    fs::create_dir_all(&target).expect("create the user-global directory");

    let output = run_init(&["--agents-md", target.to_str().unwrap()], Some(&home));

    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    assert!(
        stderr(&output).contains("machine-global agent instruction file"),
        "{}",
        stderr(&output)
    );
    assert!(
        !target.join("AGENTS.md").exists(),
        "a refused run created the machine-global file"
    );
}

/// §FS-init.1.2: the user-global rule checks the entrypoints the run *plans*,
/// and the plan depends on the effective configuration (§FS-init.2.1.1) — so a
/// configuration `init` cannot parse is reported before it. Both refuse the run
/// and write nothing; only which of the two problems the message names differs,
/// and locking the order here is what keeps a later refactor from quietly
/// swapping a config error for a rule the run never got far enough to apply.
///
/// Unix only, for the same reason as the rule it orders against: the table is
/// `~`-rooted through `$HOME` (§FS-integrations.4).
#[cfg(unix)]
#[test]
fn an_unparseable_config_is_reported_before_the_user_global_rule() {
    let home = outside_repo_dir("global_bad_config");
    fs::create_dir_all(home.join(".git")).expect("create the dotfiles marker");
    let target = home.join(".claude");
    fs::create_dir_all(&target).expect("create the user-global directory");
    let global = target.join("CLAUDE.md");
    fs::write(&global, PERSONAL_INSTRUCTIONS).expect("write personal file");
    fs::write(
        target.join("grund.toml"),
        "grund_config_version = 1\n[project]\nname = \"x\"\n",
    )
    .expect("write an unparseable config");

    let output = run_init(&[target.to_str().unwrap(), "--claude"], Some(&home));

    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    let message = stderr(&output);
    assert!(
        message.contains("unknown config section `project`"),
        "the unreadable config is the problem named first: {message}"
    );
    assert!(
        !message.contains("machine-global agent instruction file"),
        "the user-global rule cannot have been reached — its input is the plan \
         this config would have produced: {message}"
    );
    assert_eq!(
        fs::read_to_string(&global).expect("read personal file"),
        PERSONAL_INSTRUCTIONS,
        "a refused run must not touch the machine-global file"
    );
}
