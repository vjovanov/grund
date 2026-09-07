//! Same-marker precedence regressions for persisted off-grammar declarations
//! (§FS-config.3.2, §FS-check.1.1, §FS-refs.1, §FS-fmt.6,
//! §FS-workspace.8).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn test_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "grund-off-grammar-precedence-{name}-{}",
        std::process::id()
    ));
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

fn assert_code(output: &Output, expected: i32, label: &str) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{label}: expected exit {expected}\nstdout:\n{}stderr:\n{}",
        stdout(output),
        stderr(output)
    );
}

fn dotted_config(project: &str) -> String {
    format!(
        "grund_config_version = 1\nproject_name = \"{project}\"\n\n\
         [reference]\nstrict = true\n\n\
         [id]\nformat = \"{{kind}}.{{number}}.{{slug}}\"\nsection_separator = \"#\"\n\n\
         [[kinds]]\nkind = \"FS\"\nfolder = \"docs\"\nindex = false\n\n\
         [scan]\ninclude = [\"docs\"]\n"
    )
}

fn numbered_config(project: &str) -> String {
    format!(
        "grund_config_version = 1\nproject_name = \"{project}\"\n\n\
         [reference]\nstrict = true\n\n\
         [id]\nformat = \"{{kind}}-{{number}}-{{slug}}\"\n\n\
         [[kinds]]\nkind = \"FS\"\nfolder = \"docs\"\nindex = false\n\n\
         [scan]\ninclude = [\"docs\"]\n"
    )
}

fn write_precedence_declarations(root: &Path) {
    write(
        root,
        "docs/specs.md",
        &[
            "# FS.1: Persisted shorter spelling\n\nLegacy target.\n\n",
            "# FS.1.canonical: Configured full spelling\n\nCanonical target.\n",
        ]
        .concat(),
    );
}

/// A configured full citation survives the shorter exact-legacy interpretation
/// at the same local marker (§FS-config.3.2, §FS-refs.1).
#[test]
fn off_grammar_local_configured_full_citation_keeps_precedence_and_accounting() {
    let root = test_root("local-full");
    write(&root, "grund.toml", &dotted_config("fixture"));
    write_precedence_declarations(&root);
    write(&root, "docs/use.md", "Uses \u{a7}FS.1.canonical.\n");

    let refs = run(&root, &["refs", "FS.001.canonical"]);
    assert_code(&refs, 0, "configured refs");
    assert!(
        stdout(&refs).contains("docs/use.md:1:"),
        "{}",
        stdout(&refs)
    );
    let check = run(&root, &["check"]);
    assert_code(&check, 0, "check");
    let checked = stdout(&check);
    assert!(
        checked.contains("declared but never cited: FS.1"),
        "{checked}"
    );
    assert!(
        !checked.contains("declared but never cited: FS.001.canonical"),
        "{checked}"
    );
}

fn workspace_root(root: &Path) {
    write(
        root,
        "grund.toml",
        "grund_config_version = 1\nproject_name = \"root\"\n\n\
         [workspace]\nmembers = [\"api\", \"web\"]\ninclude_root = false\n",
    );
}

/// Workspace-qualified configured IDs keep the same precedence and graph
/// accounting as the local form (§FS-config.3.2, §FS-workspace.8).
#[test]
fn off_grammar_qualified_configured_full_citation_keeps_precedence_and_accounting() {
    let root = test_root("qualified-full");
    workspace_root(&root);
    write(&root, "api/grund.toml", &dotted_config("api"));
    write_precedence_declarations(&root.join("api"));
    write(&root, "web/grund.toml", &numbered_config("web"));
    write(&root, "web/docs/use.md", "Uses \u{a7}api/FS.1.canonical.\n");

    let refs = run(&root, &["refs", "api/FS.001.canonical"]);
    assert_code(&refs, 0, "qualified configured refs");
    assert!(
        stdout(&refs).contains("web/docs/use.md:1:"),
        "{}",
        stdout(&refs)
    );
    let check = run(&root, &["check"]);
    assert_code(&check, 0, "workspace check");
    let checked = stdout(&check);
    assert!(
        checked.contains("declared but never cited: FS.1"),
        "{checked}"
    );
    assert!(
        !checked.contains("declared but never cited: FS.001.canonical"),
        "{checked}"
    );
}

fn write_ambiguous_declarations(root: &Path) {
    write(
        root,
        "docs/specs.md",
        &[
            "# FS-042: Persisted exact shorthand spelling\n\nLegacy target.\n\n",
            "# FS-042-canonical: Configured shorthand target\n\nCanonical target.\n",
        ]
        .concat(),
    );
}

fn assert_ambiguous_check(output: &Output, written: &str) {
    assert_code(output, 1, "ambiguous check");
    let checked = stdout(output);
    assert!(
        checked.contains(&format!(
            "shorthand citation {written} is ambiguous: FS-042, FS-042-canonical"
        )),
        "{checked}"
    );
    assert!(
        checked.contains("declared but never cited: FS-042"),
        "{checked}"
    );
    assert!(
        checked.contains("declared but never cited: FS-042-canonical"),
        "{checked}"
    );
}

/// A local exact-legacy/shorthand collision remains one unresolved citation
/// site, so neither candidate acquires a guessed edge (§FS-config.3.2,
/// §FS-check.1.1, §FS-fmt.6).
#[test]
fn off_grammar_local_exact_legacy_and_shorthand_are_ambiguous_without_graph_edge() {
    let root = test_root("local-ambiguity");
    write(&root, "grund.toml", &numbered_config("fixture"));
    write_ambiguous_declarations(&root);
    write(&root, "docs/use.md", "Uses \u{a7}FS-042.\n");

    assert_ambiguous_check(&run(&root, &["check"]), "\u{a7}FS-042");
    let query = run(&root, &["refs", "FS-042"]);
    assert_code(&query, 2, "ambiguous refs query");
    assert!(
        stderr(&query).contains("ambiguous ID: FS-042 (matches FS-042, FS-042-canonical)"),
        "{}",
        stderr(&query)
    );
    let canonical_refs = run(&root, &["refs", "FS-042-canonical"]);
    assert_code(&canonical_refs, 0, "canonical refs");
    assert!(
        stdout(&canonical_refs).is_empty(),
        "{}",
        stdout(&canonical_refs)
    );

    let before = fs::read_to_string(root.join("docs/use.md")).expect("read before fmt");
    assert_code(&run(&root, &["fmt", "--write"]), 0, "fmt");
    let after = fs::read_to_string(root.join("docs/use.md")).expect("read after fmt");
    assert_eq!(
        after, before,
        "ambiguous citation must remain byte-identical"
    );
}

/// The qualified form reports the same sorted combined target set and remains
/// absent from both candidates' graph counts (§FS-config.3.2,
/// §FS-workspace.8).
#[test]
fn off_grammar_qualified_exact_legacy_and_shorthand_are_ambiguous_without_graph_edge() {
    let root = test_root("qualified-ambiguity");
    workspace_root(&root);
    write(&root, "api/grund.toml", &numbered_config("api"));
    write_ambiguous_declarations(&root.join("api"));
    write(&root, "web/grund.toml", &numbered_config("web"));
    write(&root, "web/docs/use.md", "Uses \u{a7}api/FS-042.\n");

    assert_ambiguous_check(&run(&root, &["check"]), "\u{a7}api/FS-042");
    let query = run(&root, &["refs", "api/FS-042"]);
    assert_code(&query, 2, "qualified ambiguous refs query");
    assert!(
        stderr(&query).contains("ambiguous ID: FS-042 (matches FS-042, FS-042-canonical)"),
        "{}",
        stderr(&query)
    );
    let canonical_refs = run(&root, &["refs", "api/FS-042-canonical"]);
    assert_code(&canonical_refs, 0, "qualified canonical refs");
    assert!(
        stdout(&canonical_refs).is_empty(),
        "{}",
        stdout(&canonical_refs)
    );

    let before = fs::read_to_string(root.join("web/docs/use.md")).expect("read before fmt");
    assert_code(&run(&root, &["fmt", "--write"]), 0, "workspace fmt");
    let after = fs::read_to_string(root.join("web/docs/use.md")).expect("read after fmt");
    assert_eq!(
        after, before,
        "ambiguous citation must remain byte-identical"
    );
}
