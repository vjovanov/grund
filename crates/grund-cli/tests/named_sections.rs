//! Black-box contract for opt-in named section coordinates. The fixture is the
//! validated issue-178 reproducer expanded across every CLI consumer
//! (§FS-config.3.2, §FS-config.3.3).

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
            .join("../../target/named-sections-tests")
            .join(format!("{name}-{}-{serial}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("docs")).expect("create named-section fixture");
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
        fs::write(path, body).expect("write fixture file");
    }

    fn enabled(&self, strict: bool) {
        self.write(
            "grund.toml",
            &format!(
                "grund_config_version = 1\n\n[reference]\nstrict = {strict}\n\n\
                 [id]\nformat = \"{{kind}}-{{slug}}\"\nnamed_sections = true\n\n\
                 [scan]\ninclude = [\"docs\"]\n"
            ),
        );
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn grund(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_grund"))
        .args(args)
        .current_dir(root)
        .output()
        .expect("run grund")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn expect_success(output: &Output, context: &str) {
    assert!(
        output.status.success(),
        "{context}: expected success, got {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        stdout(output),
        stderr(output)
    );
}

fn doc() -> &'static str {
    "# FS-doc: Document\n\nDeclaration lead.\n\n\
     ## goals: Scope\n\nGoals lead.\n\n\
     ### goals.performance: Performance\n\nThis nested named section is the decisive target.\n\n\
     #### goals.performance.latency: Latency\n\nNested child.\n\n\
     ### goals.3: Ordered child\n\nThe legal numeric child.\n\n\
     ## 1. Numeric\n\nExisting numeric section.\n"
}

#[test]
fn named_sections_drive_show_refs_json_and_completion() {
    let repo = Repo::new("readers");
    repo.enabled(true);
    repo.write("docs/FS-doc.md", doc());
    repo.write(
        "docs/FS-reader.md",
        "# FS-reader: Reader\n\nUses \u{a7}FS-doc.goals.performance and \u{a7}FS-doc.goals.3.\n",
    );

    let direct = grund(repo.path(), &["show", "FS-doc.goals.performance"]);
    expect_success(&direct, "nested name query");
    let direct_text = stdout(&direct);
    assert!(direct_text.contains("### goals.performance: Performance"));
    assert!(direct_text.contains("decisive target"));
    assert!(
        !direct_text.contains("Nested child"),
        "default cuts at child"
    );

    let declaration = grund(repo.path(), &["show", "FS-doc"]);
    expect_success(&declaration, "named heading cuts declaration lead");
    assert_eq!(stdout(&declaration), "Declaration lead.\n");

    for args in [
        vec!["show", "FS-doc.goals.performance", "--brief"],
        vec!["show", "FS-doc.goals.performance", "--toc"],
        vec!["show", "FS-doc.goals.performance", "--full"],
        vec!["show", "FS-doc", "--section", "goals.performance"],
        vec!["show", "FS-doc.goals.3"],
    ] {
        let output = grund(repo.path(), &args);
        expect_success(&output, &format!("named read mode: {args:?}"));
        if args.contains(&"--full") {
            assert!(stdout(&output).contains("Nested child"));
        }
    }

    let markdown = grund(
        repo.path(),
        &["show", "FS-doc.goals.performance", "--format", "md"],
    );
    expect_success(&markdown, "named Markdown query");
    assert!(stdout(&markdown).starts_with("### goals.performance: Performance"));

    let toc = grund(repo.path(), &["show", "FS-doc", "--toc"]);
    expect_success(&toc, "whole named TOC");
    let toc = stdout(&toc);
    for heading in [
        "## goals: Scope",
        "### goals.performance: Performance",
        "#### goals.performance.latency: Latency",
        "### goals.3: Ordered child",
        "## 1. Numeric",
    ] {
        assert!(toc.contains(heading), "TOC omitted {heading:?}: {toc}");
    }

    let json = grund(
        repo.path(),
        &["show", "FS-doc.goals.performance", "--format", "json"],
    );
    expect_success(&json, "named JSON query");
    let json: Value = serde_json::from_slice(&json.stdout).expect("show JSON");
    assert_eq!(json["section"], "goals.performance");
    assert!(json["body"].as_str().unwrap().contains("decisive target"));
    let json_toc = grund(
        repo.path(),
        &["show", "FS-doc", "--toc", "--format", "json"],
    );
    expect_success(&json_toc, "named JSON TOC");
    let json_toc: Value = serde_json::from_slice(&json_toc.stdout).expect("TOC JSON");
    let paths = json_toc["sections"]
        .as_array()
        .expect("section array")
        .iter()
        .map(|section| section["path"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(paths.contains(&"goals.performance"));
    assert!(paths.contains(&"goals.3"));

    let refs = grund(
        repo.path(),
        &["refs", "FS-doc", "--section", "goals.performance"],
    );
    expect_success(&refs, "named refs filter");
    let refs = stdout(&refs);
    assert!(refs.contains("FS-reader.md"));
    assert!(refs.contains("\u{a7}FS-doc.goals.performance"));
    assert!(!refs.contains("\u{a7}FS-doc.goals.3"));

    let complete = grund(
        repo.path(),
        &["complete", "ids", "--prefix", "FS-doc.goals", "--sections"],
    );
    expect_success(&complete, "named completion");
    let complete = stdout(&complete);
    for candidate in [
        "FS-doc.goals",
        "FS-doc.goals.performance",
        "FS-doc.goals.performance.latency",
        "FS-doc.goals.3",
    ] {
        assert!(complete.lines().any(|line| line == candidate));
    }
    assert!(!complete.contains("FS-doc.1.goals"));

    let shown = grund(repo.path(), &["config", "show"]);
    expect_success(&shown, "enabled config inspection");
    assert!(stdout(&shown).contains("named_sections = true"));
}

#[test]
fn named_sections_report_missing_reserved_orphan_depth_and_duplicates() {
    let repo = Repo::new("diagnostics");
    repo.enabled(false);
    repo.write(
        "docs/FS-doc.md",
        "# FS-doc: Document\n\n\
         ## goals: First\n\nFirst.\n\n\
         ## goals: Second\n\nSecond.\n\n\
         ## missing.performance: Wrong depth and orphan\n\nOrphan.\n\n\
         ## 1. Numeric\n\nNumeric.\n",
    );
    repo.write(
        "docs/FS-reader.md",
        "# FS-reader: Reader\n\n\
         Marked missing \u{a7}FS-doc.absent.\n\
         Unmarked FS-doc.absent must stay prose.\n\
         Reserved \u{a7}FS-doc.1.goals must not truncate.\n",
    );

    let check = grund(repo.path(), &["check"]);
    assert_eq!(check.status.code(), Some(1), "stderr: {}", stderr(&check));
    let findings = stdout(&check);
    assert!(findings.contains("section not found") && findings.contains("FS-doc.absent"));
    assert!(
        findings.contains("<§>"),
        "missing-name escape hint: {findings}"
    );
    assert!(findings.contains("duplicate section FS-doc.goals"));
    assert!(findings.contains("orphan") && findings.contains("missing.performance"));
    assert!(
        findings.contains("FS-doc.md:11"),
        "orphan heading line: {findings}"
    );
    assert!(findings.contains("section heading level mismatch"));

    let ambiguous = grund(repo.path(), &["show", "FS-doc.goals"]);
    assert_eq!(ambiguous.status.code(), Some(1));
    assert!(stderr(&ambiguous).contains("ambiguous section: FS-doc.goals"));

    let toc = grund(repo.path(), &["show", "FS-doc", "--toc"]);
    expect_success(&toc, "whole-declaration TOC despite duplicate");
    assert_eq!(stdout(&toc).matches("## goals:").count(), 2);

    let reserved = grund(repo.path(), &["show", "FS-doc.1.goals"]);
    assert!(!reserved.status.success());
    assert!(!stdout(&reserved).contains("## 1. Numeric"));

    let refs = grund(repo.path(), &["refs", "FS-doc"]);
    expect_success(&refs, "refs with suppressed prose tokens");
    let refs = stdout(&refs);
    assert!(refs.contains("\u{a7}FS-doc.absent"));
    assert!(!refs.contains("Unmarked FS-doc.absent"));
    assert!(!refs.contains("\u{a7}FS-doc.1.goals"));
}

#[test]
fn named_sections_preserve_full_ids_before_shorthand_and_findings_compose() {
    let repo = Repo::new("shorthand");
    repo.write(
        "grund.toml",
        "grund_config_version = 1\n\n[reference]\nstrict = true\n\n\
         [id]\nformat = \"{kind}-{number}-{slug}\"\nnamed_sections = true\n\n\
         [scan]\ninclude = [\"docs\"]\n",
    );
    repo.write(
        "docs/FS-042-doc.md",
        "# FS-042-doc: Document\n\n## goals: Goals\n\nBody.\n",
    );
    repo.write(
        "docs/FS-043-reader.md",
        "# FS-043-reader: Reader\n\n\u{a7}FS-042.missing\n\u{a7}FS-042-doc.missing\n",
    );

    let check = grund(repo.path(), &["check"]);
    assert_eq!(check.status.code(), Some(1), "stderr: {}", stderr(&check));
    let findings = stdout(&check);
    assert!(findings.contains("FS-042-doc.missing"));
    assert!(
        findings.contains("shorthand"),
        "canonical-form finding: {findings}"
    );
    assert_eq!(findings.matches("section not found").count(), 2);
}

#[test]
fn named_section_formatting_uses_rendered_anchor_and_never_changes_handles() {
    let repo = Repo::new("formatting");
    repo.enabled(true);
    repo.write("docs/FS-doc.md", doc());
    repo.write(
        "docs/FS-reader.md",
        "# FS-reader: Reader\n\n\u{a7}FS-doc.goals and unmarked FS-doc.goals.\n",
    );

    let first = grund(repo.path(), &["fmt", "--cross-refs", "--write"]);
    expect_success(&first, "wrap named citation");
    let reader = fs::read_to_string(repo.path().join("docs/FS-reader.md")).unwrap();
    assert!(reader.contains("[\u{a7}FS-doc.goals](FS-doc.md#goals-scope)"));
    assert!(reader.contains("unmarked FS-doc.goals"));
    let named = fs::read_to_string(repo.path().join("docs/FS-doc.md")).unwrap();
    assert!(named.contains("## goals: Scope"));

    repo.write(
        "docs/FS-doc.md",
        &doc().replace("## goals: Scope", "## goals: Intent"),
    );
    let second = grund(repo.path(), &["fmt", "--cross-refs", "--write"]);
    expect_success(&second, "refresh named anchor after retitle");
    let reader = fs::read_to_string(repo.path().join("docs/FS-reader.md")).unwrap();
    assert!(reader.contains("[\u{a7}FS-doc.goals](FS-doc.md#goals-intent)"));
    assert!(reader.contains("\u{a7}FS-doc.goals"));
}

#[test]
fn init_teaches_false_and_the_disabled_gate_is_an_operational_guard() {
    let initialized = Repo::new("init");
    let init = grund(initialized.path(), &["init", "--no-vcs"]);
    expect_success(&init, "initialize named-section teaching default");
    let generated = fs::read_to_string(initialized.path().join("grund.toml")).unwrap();
    assert!(generated.contains("named_sections = false"));
    let default_guidance = fs::read_to_string(initialized.path().join("AGENTS.md")).unwrap();
    assert!(!default_guidance.contains("goals.performance"));

    let absent = Repo::new("absent");
    absent.write(
        "grund.toml",
        "grund_config_version = 1\n[id]\nformat = \"{kind}-{slug}\"\n[scan]\ninclude = [\"docs\"]\n",
    );
    absent.write(
        "docs/FS-doc.md",
        "# FS-doc: Document\n\n## 1. Numeric\n\nBody.\n",
    );
    let explicit_false = Repo::new("false");
    explicit_false.write(
        "grund.toml",
        "grund_config_version = 1\n[id]\nformat = \"{kind}-{slug}\"\n\
         named_sections = false\n[scan]\ninclude = [\"docs\"]\n",
    );
    explicit_false.write(
        "docs/FS-doc.md",
        "# FS-doc: Document\n\n## 1. Numeric\n\nBody.\n",
    );
    for args in [
        vec!["check"],
        vec!["show", "FS-doc.1", "--full"],
        vec!["refs", "FS-doc", "--section", "1"],
        vec!["complete", "ids", "--sections"],
        vec!["fmt", "--check"],
        vec!["config", "show"],
    ] {
        let left = grund(absent.path(), &args);
        let right = grund(explicit_false.path(), &args);
        assert_eq!(
            left.stdout, right.stdout,
            "absent and false stdout differ for {args:?}"
        );
        assert_eq!(
            left.stderr, right.stderr,
            "absent and false stderr differ for {args:?}"
        );
        assert_eq!(
            left.status.code(),
            right.status.code(),
            "absent and false exit differ for {args:?}"
        );
    }

    let enabled = Repo::new("enabled-inspection");
    enabled.enabled(true);
    let validate = grund(enabled.path(), &["config", "validate"]);
    expect_success(&validate, "enabled config must be accepted");
    let init_enabled = grund(enabled.path(), &["init", "--no-vcs"]);
    expect_success(&init_enabled, "render enabled named-section guidance");
    let enabled_guidance = fs::read_to_string(enabled.path().join("AGENTS.md")).unwrap();
    assert!(enabled_guidance.contains("goals.performance"));
    assert!(enabled_guidance.contains("goals.3"));
    assert!(enabled_guidance.contains("number.name"));
}
