//! Black-box contract for opt-in oversized-lead warnings and v10 guidance
//! (§FS-config.3.1, §FS-check.1, §FS-check.4.13, §FS-errors.5,
//! §FS-init.2.3, §FS-init.2.3.4.3).

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_ROOT: AtomicUsize = AtomicUsize::new(0);

struct Repo(PathBuf);

impl Repo {
    fn new(name: &str) -> Self {
        let serial = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/point-size-check")
            .join(format!("{name}-{}-{serial}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("docs")).expect("create point-size fixture");
        Self(root)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn write(&self, relative: &str, body: &str) {
        let path = self.0.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create fixture parent");
        }
        fs::write(path, body).expect("write fixture");
    }

    fn config(&self, warning: Option<(u64, &str)>, extra: &str) {
        self.config_named("fixture", warning, extra);
    }

    fn config_named(&self, name: &str, warning: Option<(u64, &str)>, extra: &str) {
        let warning = warning
            .map(|(max, unit)| {
                format!("lead_size_warning = {{ max = {max}, unit = \"{unit}\" }}\n")
            })
            .unwrap_or_default();
        self.write(
            "grund.toml",
            &format!(
                "grund_config_version = 1\nproject_name = \"{name}\"\n\n\
                 [reference]\nstrict = true\n{warning}\n\
                 [id]\nformat = \"{{kind}}-{{slug}}\"\nnamed_sections = true\n\n\
                 [[kinds]]\nkind = \"FS\"\nfolder = \"docs\"\nindex = false\n\n\
                 [[kinds]]\nkind = \"AR\"\nfolder = \"docs/ar\"\nindex = false\n\n\
                 [scan]\ninclude = [\"docs/in\", \"src\"]\nextensions = [\"md\", \"rs\"]\n{extra}"
            ),
        );
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_grund"))
        .args(args)
        .current_dir(root)
        .output()
        .unwrap_or_else(|error| panic!("run grund {args:?}: {error}"))
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn assert_success(output: &Output) {
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout:\n{}stderr:\n{}",
        stdout(output),
        stderr(output)
    );
}

#[test]
fn absent_config_is_byte_stable_and_word_threshold_is_strict_with_exact_message() {
    let repo = Repo::new("word-threshold");
    repo.write(
        "docs/in/FS-equal.md",
        "# FS-equal: Equal\n\none two \u{a7}FS-equal\n",
    );
    repo.write(
        "docs/in/FS-over.md",
        "# FS-over: Over\n\none two three \u{a7}FS-over\n",
    );
    repo.config(None, "");
    let before = run(repo.path(), &["check"]);
    assert_success(&before);
    assert_eq!(stdout(&before), "success\n");
    assert_eq!(stderr(&before), "");

    repo.config(Some((3, "words")), "");
    let checked = run(repo.path(), &["check", "--only=oversized-lead"]);
    assert_success(&checked);
    assert_eq!(
        stdout(&checked),
        concat!(
            "docs/in/FS-over.md:1: warning: FS-over lead is 4 words, over the configured maximum of 3; ",
            "move detail into citable child sections, or promote a child section to its own ID ",
            "after running grund refs FS-over --summary\n"
        )
    );
    assert_eq!(stderr(&checked), "");

    let ignored = run(repo.path(), &["check", "--ignore", "oversized-lead"]);
    assert_success(&ignored);
    assert_eq!(stdout(&ignored), "success\n");

    let json = run(
        repo.path(),
        &["check", "--only", "oversized-lead", "--format=json"],
    );
    assert_success(&json);
    let warning: Value = serde_json::from_slice(&json.stdout).expect("warning JSON");
    assert_eq!(warning["severity"], "warning");
    assert_eq!(warning["path"], "docs/in/FS-over.md");
    assert_eq!(warning["line"], 1);
    assert_eq!(warning["code"], "oversized-lead");
    assert_eq!(warning["sites"], Value::Null);
    assert_eq!(
        warning["message"],
        "FS-over lead is 4 words, over the configured maximum of 3; move detail into citable child sections, or promote a child section to its own ID after running grund refs FS-over --summary"
    );
}

#[test]
fn equality_and_overage_are_byte_defined_for_all_three_units_and_sections() {
    for (unit, max, equal, over, actual) in [
        ("lines", 2, "first\nsecond", "first\nsecond\nthird", 3),
        ("words", 2, "one two", "one\u{2003}two three four", 3),
        ("bytes", 3, "é", "éx", 4),
    ] {
        let repo = Repo::new(&format!("{unit}-boundary"));
        repo.write(
            "docs/in/FS-equal.md",
            &format!("# FS-equal: Equal\n\n{equal}\n"),
        );
        repo.write(
            "docs/in/FS-over.md",
            &format!("# FS-over: Over\n\n{over}\n"),
        );
        repo.write(
            "docs/in/FS-section.md",
            &format!("# FS-section: Section\n\n## 1. Child\n\n{over}\n"),
        );
        repo.config(Some((max, unit)), "");
        let checked = run(repo.path(), &["check", "--only=oversized-lead"]);
        assert_success(&checked);
        let output = stdout(&checked);
        assert!(!output.contains("FS-equal lead is"), "{unit}: {output}");
        assert!(
            output.contains("docs/in/FS-over.md:1: warning: FS-over lead is "),
            "{unit}: {output}"
        );
        assert!(
            output.contains(&format!(
                "lead is {actual} {unit}, over the configured maximum of {max}"
            )),
            "{unit}: {output}"
        );
        assert!(
            output.contains("docs/in/FS-section.md:3: warning: FS-section.1 lead is "),
            "section heading/location missing for {unit}: {output}"
        );
    }
}

#[test]
fn duplicate_sites_warn_locally_broken_stubs_stay_silent_and_errors_still_win() {
    let repo = Repo::new("duplicate-stub");
    repo.config(Some((0, "words")), "");
    repo.write("docs/in/a.md", "# FS-dupe: First\n\none\n");
    repo.write("docs/in/b.md", "# FS-dupe: Second\n\none two\n");
    repo.write(
        "docs/in/FS-sections.md",
        "# FS-sections: Sections\n\n## 1. First\n\none\n\n## 1. Second\n\none two\n",
    );
    repo.write(
        "docs/ar/AR-broken.md",
        "# AR-broken: [src/missing.rs](../../src/missing.rs)\n",
    );

    let complete = run(repo.path(), &["check"]);
    assert_eq!(complete.status.code(), Some(1));
    assert!(stdout(&complete).contains("duplicate declaration"));
    assert!(stdout(&complete).contains("duplicate section"));
    assert!(stdout(&complete).contains("stub link target missing: ../../src/missing.rs"));
    assert_eq!(stdout(&complete).matches("FS-dupe lead is").count(), 2);
    assert_eq!(
        stdout(&complete).matches("FS-sections.1 lead is").count(),
        2
    );
    assert!(!stdout(&complete).contains("AR-broken lead is"));

    let selected = run(repo.path(), &["check", "--only=oversized-lead"]);
    assert_success(&selected);
    assert_eq!(stdout(&selected).matches("FS-dupe lead is").count(), 2);
    assert_eq!(
        stdout(&selected).matches("FS-sections.1 lead is").count(),
        2
    );
}

#[test]
fn configured_scope_is_per_project_explicit_path_and_not_widened_by_full() {
    let repo = Repo::new("scope");
    repo.write(
        "grund.toml",
        "grund_config_version = 1\nproject_name = \"root\"\n\n\
         [workspace]\nmembers = [\"warned\", \"silent\"]\ninclude_root = false\n",
    );
    for (member, warning) in [("warned", true), ("silent", false)] {
        let nested = Repo(repo.path().join(member));
        nested.config_named(member, warning.then_some((0, "words")), "");
        nested.write(
            "docs/in/FS-in.md",
            &format!("# FS-in: In\n\n{member} \u{a7}FS-in\n"),
        );
        nested.write(
            "outside/FS-out.md",
            "# FS-out: Outside configured include\n\noutside \u{a7}FS-out\n",
        );
        std::mem::forget(nested);
    }

    let workspace = run(repo.path(), &["check", "--only=oversized-lead"]);
    assert_success(&workspace);
    assert!(stdout(&workspace).contains("warned/FS-in lead is"));
    assert!(!stdout(&workspace).contains("silent/FS-in"));

    let warned = repo.path().join("warned");
    let explicit = run(
        &warned,
        &["check", "docs/in/FS-in.md", "--only=oversized-lead"],
    );
    assert_success(&explicit);
    assert!(stdout(&explicit).contains("FS-in lead is"));

    let full = run(&warned, &["check", "--full", "--only=oversized-lead"]);
    assert_success(&full);
    assert!(stdout(&full).contains("FS-in lead is"));
    assert!(!stdout(&full).contains("FS-out lead is"));
}

#[test]
fn lead_size_config_validates_strictly_and_config_show_round_trips_it() {
    let repo = Repo::new("config");
    let invalid = [
        "lead_size_warning = { unit = \"words\" }",
        "lead_size_warning = { max = 1 }",
        "lead_size_warning = { max = -1, unit = \"words\" }",
        "lead_size_warning = { max = 1, unit = \"tokens\" }",
        "lead_size_warning = { max = 1, unit = \"Words\" }",
        "lead_size_warning = { max = 1, unit = \"words\", severity = \"error\" }",
        "lead_size_warning = { max = 1, max = 2, unit = \"words\" }",
    ];
    for (index, line) in invalid.into_iter().enumerate() {
        repo.write(
            "grund.toml",
            &format!("grund_config_version = 1\n\n[reference]\nstrict = true\n{line}\n"),
        );
        let output = run(repo.path(), &["config", "validate"]);
        assert_eq!(
            output.status.code(),
            Some(1),
            "invalid config {index} accepted: {line}\n{}",
            stdout(&output)
        );
        assert!(stderr(&output).contains("lead_size_warning"));
    }

    repo.config(Some((600, "words")), "");
    let shown = run(repo.path(), &["config", "show"]);
    assert_success(&shown);
    assert!(stdout(&shown).contains("lead_size_warning = { max = 600, unit = \"words\" }"));

    repo.config(None, "");
    let absent = run(repo.path(), &["config", "show"]);
    assert_success(&absent);
    assert!(!stdout(&absent).contains("lead_size_warning"));
}

#[test]
fn init_emits_v10_size_guidance_and_migrates_v8_in_one_command() {
    let repo = Repo::new("init-v10");
    let v8_fixture = repo.path().join("v8");
    fs::create_dir_all(&v8_fixture).expect("create v8 fixture");
    fs::write(
        v8_fixture.join("grund.toml"),
        "grund_config_version = 1\nproject_name = \"fixture\"\n",
    )
    .expect("write v8 config");
    let canonical_v8 = concat!(
        "<!-- BEGIN GRUND MANAGED BLOCK -->\n",
        "## Grounding with grund (v8)\n\n",
        "Legacy guidance.\n",
        "<!-- END GRUND MANAGED BLOCK -->\n",
    );
    fs::write(v8_fixture.join("AGENTS.md"), &canonical_v8).expect("write v8 block");
    let stale = run(&v8_fixture, &["check", "--only=agents-init"]);
    assert_eq!(stale.status.code(), Some(1));
    assert!(stdout(&stale).contains("outdated grund init block v8"));
    assert!(stdout(&stale).contains("run `grund init` to update to v10"));

    let fresh = repo.path().join("fresh");
    fs::create_dir_all(&fresh).expect("create init target");
    let initialized = run(
        repo.path(),
        &["init", fresh.to_str().unwrap(), "--agents-md"],
    );
    assert_success(&initialized);
    let agents = fs::read_to_string(fresh.join("AGENTS.md")).expect("read fresh AGENTS.md");
    assert!(agents.contains("## Grounding with grund (v10)"));
    assert!(agents.contains("grund list --size=words --top 10"));
    let config = fs::read_to_string(fresh.join("grund.toml")).expect("read fresh config");
    assert!(!config.contains("lead_size_warning"));
    assert!(!config.contains("tokens"));

    let migrated = repo.path().join("migrated");
    fs::create_dir_all(&migrated).expect("create migration target");
    fs::copy(v8_fixture.join("grund.toml"), migrated.join("grund.toml"))
        .expect("copy migration config");
    fs::write(
        migrated.join("AGENTS.md"),
        format!("before\n{canonical_v8}after\n"),
    )
    .expect("write v8 block");
    let updated = run(
        repo.path(),
        &["init", migrated.to_str().unwrap(), "--agents-md"],
    );
    assert_success(&updated);
    let agents = fs::read_to_string(migrated.join("AGENTS.md")).expect("read migrated AGENTS.md");
    assert!(agents.starts_with("before\n"));
    assert!(agents.ends_with("after\n"));
    assert!(agents.contains("## Grounding with grund (v10)"));
    assert!(agents.contains("grund list --size=words --top 10"));
}
