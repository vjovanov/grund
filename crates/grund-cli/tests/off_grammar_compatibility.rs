//! End-to-end contract for catalog-backed reads of persisted off-grammar
//! declarations across CLI surfaces (§FS-config.3.2, §FS-check.1.1,
//! §FS-show.1, §FS-refs.1, §FS-list.2, §FS-completions.2, §FS-cover.2,
//! §FS-fmt.6, and §FS-workspace.8).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn test_root(name: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("grund-off-grammar-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create test root");
    root
}

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create fixture parent");
    }
    fs::write(path, text).expect("write fixture");
}

fn run(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_grund"))
        .args(args)
        .current_dir(root)
        .output()
        .unwrap_or_else(|err| panic!("run grund {args:?}: {err}"))
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("UTF-8 stdout")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("UTF-8 stderr")
}

fn expect_code(failures: &mut Vec<String>, name: &str, output: &Output, expected: i32) {
    if output.status.code() != Some(expected) {
        failures.push(format!(
            "{name}: expected exit {expected}, got {:?}\nstdout:\n{}stderr:\n{}",
            output.status.code(),
            stdout(output),
            stderr(output)
        ));
    }
}

fn expect_contains(failures: &mut Vec<String>, name: &str, actual: &str, needle: &str) {
    if !actual.contains(needle) {
        failures.push(format!(
            "{name}: expected {needle:?} in output\nactual:\n{actual}"
        ));
    }
}

fn numbered_fixture(root: &Path) {
    write(
        root,
        "grund.toml",
        "grund_config_version = 1\n\
         project_name = \"fixture\"\n\n\
         [reference]\nstrict = true\n\n\
         [id]\nformat = \"{kind}-{number}-{slug}\"\n\n\
         [[kinds]]\nkind = \"FS\"\nfolder = \"docs/functional-spec\"\nindex = false\n\n\
         [scan]\ninclude = [\"docs\", \"src\"]\nextensions = [\"md\", \"rs\"]\nrespect_gitignore = false\n",
    );
    write(
        root,
        "docs/functional-spec/FS-security-providers.md",
        "# FS-security-providers: Security providers\n\n\
         The project defines security providers.\n\n\
         ## 1. Contract\n\n\
         The provider contract is stable.\n",
    );
    write(
        root,
        "src/security.rs",
        "// The implementation follows \u{a7}FS-security-providers and \u{a7}FS-security-providers.1.\n\
         // Unmarked FS-security-providers is prose.\n\
         // Marked but unbacked \u{a7}FS-not-declared is not promoted by compatibility.\n\n\
         pub fn provider_name() -> &'static str { \"fixture\" }\n",
    );
    write(
        root,
        "docs/consumer.md",
        "The implementation follows \u{a7}FS-security-providers and \u{a7}FS-security-providers.1.\n\
         Marked but unbacked \u{a7}FS-not-declared is not promoted by compatibility.\n",
    );
}

/// The triage reproducer, expanded across every CLI consumer that reads the
/// shared scanner/catalog (§FS-config.3.2). This test deliberately gathers all
/// mismatches so a scanner regression cannot be hidden by `show` failing first.
#[test]
fn persisted_off_grammar_declaration_is_read_consistently_across_cli_surfaces() {
    let root = test_root("cli-matrix");
    numbered_fixture(&root);
    let mut failures = Vec::new();

    for (name, args, needle) in [
        (
            "show brief",
            vec!["show", "FS-security-providers", "--brief"],
            "Security providers",
        ),
        (
            "show lead",
            vec!["show", "FS-security-providers"],
            "project defines security providers",
        ),
        (
            "show toc",
            vec!["show", "FS-security-providers", "--toc"],
            "## 1. Contract",
        ),
        (
            "show full",
            vec!["show", "FS-security-providers", "--full"],
            "provider contract is stable",
        ),
        (
            "show inline section",
            vec!["show", "FS-security-providers.1", "--toc"],
            "1. Contract",
        ),
        (
            "show section flag",
            vec!["show", "FS-security-providers", "--section", "1", "--brief"],
            "1. Contract",
        ),
    ] {
        let output = run(&root, &args);
        expect_code(&mut failures, name, &output, 0);
        expect_contains(&mut failures, name, &stdout(&output), needle);
    }

    let refs = run(&root, &["refs", "FS-security-providers"]);
    expect_code(&mut failures, "refs", &refs, 0);
    expect_contains(&mut failures, "refs", &stdout(&refs), "src/security.rs:1:");
    expect_contains(
        &mut failures,
        "refs",
        &stdout(&refs),
        "\u{a7}FS-security-providers.1",
    );

    let section_refs = run(&root, &["refs", "FS-security-providers", "--section", "1"]);
    expect_code(&mut failures, "section refs", &section_refs, 0);
    expect_contains(
        &mut failures,
        "section refs",
        &stdout(&section_refs),
        "\u{a7}FS-security-providers.1",
    );

    for (name, args) in [
        ("list text", vec!["list"]),
        ("list json", vec!["list", "--format", "json"]),
        (
            "completion",
            vec!["complete", "ids", "--prefix", "FS-security"],
        ),
        (
            "completion sections",
            vec![
                "complete",
                "ids",
                "--prefix",
                "FS-security-providers.",
                "--sections",
            ],
        ),
        ("cover", vec!["cover"]),
    ] {
        let output = run(&root, &args);
        expect_code(&mut failures, name, &output, 0);
        expect_contains(
            &mut failures,
            name,
            &stdout(&output),
            "FS-security-providers",
        );
    }

    let check = run(&root, &["check"]);
    expect_code(&mut failures, "check text", &check, 0);
    let check_text = stdout(&check);
    for needle in [
        "docs/functional-spec/FS-security-providers.md:1:",
        "resolves for compatibility",
        "[id] format = \"{kind}-{number}-{slug}\"",
        "this warning becomes an error in grund 0.15.0",
    ] {
        expect_contains(&mut failures, "check text", &check_text, needle);
    }
    if check_text.contains("success\n") {
        failures.push("check text: a warning-bearing run printed `success`".into());
    }
    if check_text.contains("FS-not-declared") {
        failures.push("check text: an unbacked malformed candidate became a citation".into());
    }

    let check_json = run(&root, &["check", "--format", "json"]);
    expect_code(&mut failures, "check json", &check_json, 0);
    let json_text = stdout(&check_json);
    for needle in [
        "\"severity\":\"warning\"",
        "\"code\":\"declaration-near-miss\"",
        "\"path\":\"docs/functional-spec/FS-security-providers.md\"",
        "\"line\":1",
    ] {
        expect_contains(&mut failures, "check json", &json_text, needle);
    }

    let invalid = run(&root, &["show", "FS-not-declared"]);
    expect_code(&mut failures, "unbacked query", &invalid, 1);
    expect_contains(
        &mut failures,
        "unbacked query",
        &stderr(&invalid),
        "invalid ID",
    );

    let source_before =
        fs::read_to_string(root.join("src/security.rs")).expect("read source before rewrite");
    let fmt = run(&root, &["fmt", "--cross-refs", "--write"]);
    expect_code(&mut failures, "fmt cross refs", &fmt, 0);
    let rewritten = fs::read_to_string(root.join("docs/consumer.md")).expect("read rewrite");
    expect_contains(
        &mut failures,
        "fmt cross refs",
        &rewritten,
        "[\u{a7}FS-security-providers](functional-spec/FS-security-providers.md#",
    );
    if rewritten.contains("[\u{a7}FS-not-declared]") {
        failures.push("fmt cross refs: an unbacked malformed candidate was wrapped".into());
    }
    let source_after =
        fs::read_to_string(root.join("src/security.rs")).expect("read source after rewrite");
    if source_after != source_before {
        failures.push("fmt cross refs: a source file received Markdown link syntax".into());
    }

    assert!(
        failures.is_empty(),
        "off-grammar CLI compatibility mismatches:\n\n{}",
        failures.join("\n\n")
    );
}

/// Qualified reads use the target member's catalog and per-kind grammar, while
/// local and qualified citations resolve to one target (§FS-workspace.4,
/// §FS-workspace.8).
#[test]
fn off_grammar_workspace_qualified_reads_use_the_target_per_kind_catalog() {
    let root = test_root("workspace");
    write(
        &root,
        "grund.toml",
        "grund_config_version = 1\nproject_name = \"root\"\n\n\
         [workspace]\nmembers = [\"api\", \"client\"]\ninclude_root = false\n",
    );
    write(
        &root,
        "api/grund.toml",
        "grund_config_version = 1\nproject_name = \"api\"\n\n\
         [id]\nformat = \"{kind}-{number}-{slug}\"\n\n\
         [[kinds]]\nkind = \"FS\"\nfolder = \"docs/specs\"\nformat = \"{kind}_{number}_{slug}\"\nindex = false\n\n\
         [scan]\ninclude = [\"docs\", \"src\"]\n",
    );
    write(
        &root,
        "api/docs/specs/security.md",
        "# FS-security-providers: Security providers\n\nMember lead.\n\n## 1. Contract\n\nMember contract.\n",
    );
    write(
        &root,
        "api/src/security.rs",
        "// Local \u{a7}FS-security-providers.1\n",
    );
    write(
        &root,
        "client/grund.toml",
        "grund_config_version = 1\nproject_name = \"client\"\n\n\
         [id]\nformat = \"{kind}-{slug}\"\n\n\
         [scan]\ninclude = [\"docs\"]\n",
    );
    write(
        &root,
        "client/docs/notes.md",
        "The client follows \u{a7}api/FS-security-providers.\n",
    );

    let mut failures = Vec::new();
    for (name, args, needle) in [
        (
            "qualified show",
            vec!["show", "api/FS-security-providers.1"],
            "Member contract",
        ),
        (
            "qualified refs",
            vec!["refs", "api/FS-security-providers"],
            "client/docs/notes.md",
        ),
        (
            "workspace list",
            vec!["list", "--project", "api"],
            "api/FS-security-providers",
        ),
        (
            "workspace completion",
            vec!["complete", "ids", "--prefix", "api/FS-security"],
            "api/FS-security-providers",
        ),
    ] {
        let output = run(&root, &args);
        expect_code(&mut failures, name, &output, 0);
        expect_contains(&mut failures, name, &stdout(&output), needle);
    }
    let check = run(&root, &["check"]);
    expect_code(&mut failures, "workspace check", &check, 0);
    let check_text = stdout(&check);
    expect_contains(
        &mut failures,
        "workspace check",
        &check_text,
        "api/docs/specs/security.md:1:",
    );
    expect_contains(
        &mut failures,
        "workspace check",
        &check_text,
        "format = \"{kind}_{number}_{slug}\"",
    );
    if check_text.contains("unknown reference") {
        failures.push(format!(
            "workspace check: declaration-backed local/qualified citation was dangling:\n{check_text}"
        ));
    }

    assert!(
        failures.is_empty(),
        "off-grammar workspace compatibility mismatches:\n\n{}",
        failures.join("\n\n")
    );
}

/// Exact persisted IDs do not license guesses: shorthand collisions stay
/// ambiguous, while a whole-token declaration wins before section splitting
/// (§FS-config.3.2, §FS-show.1).
#[test]
fn off_grammar_exact_id_section_and_shorthand_precedence_fail_rather_than_guess() {
    let root = test_root("ambiguity");
    numbered_fixture(&root);
    write(
        &root,
        "docs/functional-spec/FS-042.md",
        "# FS-042: Persisted exact shorthand spelling\n\nExact legacy target.\n",
    );
    write(
        &root,
        "docs/functional-spec/FS-042-security.md",
        "# FS-042-security: Conforming shorthand target\n\nCanonical target.\n",
    );
    write(
        &root,
        "docs/functional-spec/FS-security-providers.1.md",
        "# FS-security-providers.1: Whole-token declaration\n\nWhole-token target.\n",
    );

    let ambiguous = run(&root, &["show", "FS-042"]);
    assert_eq!(
        ambiguous.status.code(),
        Some(1),
        "shorthand collision must fail:\n{}",
        stdout(&ambiguous)
    );
    let ambiguous_text = stderr(&ambiguous);
    assert!(
        ambiguous_text.contains("ambiguous ID")
            && ambiguous_text.contains("FS-042")
            && ambiguous_text.contains("FS-042-security"),
        "collision must name both targets, got:\n{ambiguous_text}"
    );

    let exact = run(&root, &["show", "FS-security-providers.1"]);
    assert_eq!(
        exact.status.code(),
        Some(0),
        "whole-token exact declaration must win:\n{}",
        stderr(&exact)
    );
    assert!(stdout(&exact).contains("Whole-token target."));
}

/// Duplicate persisted spellings retain the ordinary ambiguous-ID refusal and
/// deterministic declaration sites (§FS-config.3.2, §FS-list.2).
#[test]
fn duplicate_off_grammar_declarations_are_ambiguous_with_sorted_sites() {
    let root = test_root("duplicate");
    numbered_fixture(&root);
    write(
        &root,
        "docs/functional-spec/duplicate.md",
        "# FS-security-providers: Duplicate security providers\n\nDuplicate.\n",
    );

    let output = run(&root, &["show", "FS-security-providers"]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "duplicate query must fail:\n{}",
        stdout(&output)
    );
    let actual = stderr(&output);
    let first = "docs/functional-spec/FS-security-providers.md:1";
    let second = "docs/functional-spec/duplicate.md:1";
    let first_offset = actual.find(first);
    let second_offset = actual.find(second);
    assert!(
        actual.contains("ambiguous ID")
            && first_offset.is_some()
            && second_offset.is_some()
            && first_offset < second_offset,
        "duplicate sites must be complete and sorted, got:\n{actual}"
    );
}
