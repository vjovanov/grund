//! Regression coverage for exact persisted spellings, promoted inline citation
//! sites, and formatter-wrapper reads (§FS-config.3.2, §FS-show.3.2,
//! §FS-inline-citation-style.3.1, §FS-workspace.8).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn test_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "grund-off-grammar-fix-{name}-{}",
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

/// A narrowed component regex cannot claim a shorter configured prefix before
/// the complete colon-delimited spelling reaches the catalog (§FS-config.3.2,
/// §FS-show.1, §FS-list.2, §FS-refs.1, §FS-check.4.6).
#[test]
fn off_grammar_narrowed_slug_pattern_retains_the_exact_written_token() {
    let root = test_root("narrowed-pattern");
    write(
        &root,
        "grund.toml",
        "grund_config_version = 1\nproject_name = \"fixture\"\n\n\
         [reference]\nstrict = true\n\n\
         [id]\nformat = \"{kind}-{slug}\"\nslug_pattern = \"[a-z]+\"\n\n\
         [[kinds]]\nkind = \"FS\"\nfolder = \"docs\"\nindex = false\n\n\
         [scan]\ninclude = [\"docs\"]\n",
    );
    write(
        &root,
        "docs/specs.md",
        "# FS-legacy-2: Persisted under the former slug pattern\n\nExact legacy body.\n",
    );
    write(&root, "docs/use.md", "Uses \u{a7}FS-legacy-2.\n");

    let show = run(&root, &["show", "FS-legacy-2", "--full"]);
    assert_code(&show, 0, "show");
    assert!(stdout(&show).contains("Exact legacy body."));

    let list = run(&root, &["list"]);
    assert_code(&list, 0, "list");
    let listed = stdout(&list);
    assert!(listed.contains("FS-legacy-2  docs/specs.md:1"), "{listed}");
    assert!(
        !listed.lines().any(|line| line.starts_with("FS-legacy  ")),
        "{listed}"
    );

    let refs = run(&root, &["refs", "FS-legacy-2"]);
    assert_code(&refs, 0, "refs");
    let referenced = stdout(&refs);
    assert!(referenced.contains("docs/use.md:1:"), "{referenced}");
    assert!(referenced.contains("\u{a7}FS-legacy-2"), "{referenced}");

    let check = run(&root, &["check"]);
    assert_code(&check, 0, "check");
    let checked = stdout(&check);
    assert!(
        checked.contains("`FS-legacy-2` resolves for compatibility"),
        "{checked}"
    );
    assert!(
        checked.contains("[id] format = \"{kind}-{slug}\""),
        "{checked}"
    );
    assert!(!checked.contains("unknown reference"), "{checked}");
    assert!(!checked.contains("success\n"), "{checked}");
}

fn local_inline_config(style: &str, layout: bool) -> String {
    let layout = if layout {
        "inline_note_layout = \"citation-first-colon\"\ninline_note_layout_check = \"error\"\n"
    } else {
        ""
    };
    format!(
        "grund_config_version = 1\nproject_name = \"fixture\"\n\n\
         [reference]\nstrict = true\ninline_style = \"{style}\"\n{layout}\n\
         [id]\nformat = \"{{kind}}-{{number}}-{{slug}}\"\n\n\
         [[kinds]]\nkind = \"FS\"\nfolder = \"docs\"\nindex = false\n\n\
         [scan]\ninclude = [\"docs\", \"src\"]\nextensions = [\"md\", \"rs\"]\n"
    )
}

/// Local promotion supplies the final token range to both citation-only and
/// layout classification (§FS-inline-citation-style.3.1, §FS-config.3.2).
#[test]
fn off_grammar_local_promotion_drives_inline_style_and_layout() {
    let root = test_root("local-inline");
    write(
        &root,
        "grund.toml",
        &local_inline_config("citation-only", false),
    );
    write(
        &root,
        "docs/legacy.md",
        "# FS-security-providers: Security providers\n\nStable.\n",
    );
    write(&root, "src/lib.rs", "// \u{a7}FS-security-providers\n");

    let pure = run(&root, &["check"]);
    assert_code(&pure, 0, "local citation-only");
    assert!(!stdout(&pure).contains("inline citation must carry no prose"));

    write(
        &root,
        "grund.toml",
        &local_inline_config("citation-with-note", true),
    );
    write(
        &root,
        "src/lib.rs",
        "// Rationale before \u{a7}FS-security-providers\n",
    );
    let laid_out = run(&root, &["check"]);
    assert_code(&laid_out, 1, "local layout");
    assert!(
        stdout(&laid_out).contains("inline note must open with its citations and a colon"),
        "{}",
        stdout(&laid_out)
    );
}

fn workspace_inline_fixture(root: &Path, style: &str, layout: bool, source: &str) {
    write(
        root,
        "grund.toml",
        "grund_config_version = 1\nproject_name = \"root\"\n\n\
         [workspace]\nmembers = [\"api\", \"client\"]\ninclude_root = false\n",
    );
    write(
        root,
        "api/grund.toml",
        "grund_config_version = 1\nproject_name = \"api\"\n\n\
         [id]\nformat = \"{kind}-{number}-{slug}\"\n\n\
         [[kinds]]\nkind = \"FS\"\nfolder = \"docs\"\nindex = false\n\n\
         [scan]\ninclude = [\"docs\"]\n",
    );
    write(
        root,
        "api/docs/legacy.md",
        "# FS-security-providers: Security providers\n\nStable.\n",
    );
    let layout = if layout {
        "inline_note_layout = \"citation-first-colon\"\ninline_note_layout_check = \"error\"\n"
    } else {
        ""
    };
    write(
        root,
        "client/grund.toml",
        &format!(
            "grund_config_version = 1\nproject_name = \"client\"\n\n\
             [reference]\nstrict = true\ninline_style = \"{style}\"\n{layout}\n\
             [id]\nformat = \"{{kind}}-{{slug}}\"\n\n\
             [[kinds]]\nkind = \"FS\"\nfolder = \"docs\"\nindex = false\n\n\
             [scan]\ninclude = [\"docs\", \"src\"]\nextensions = [\"md\", \"rs\"]\n"
        ),
    );
    write(root, "client/src/lib.rs", source);
}

/// Workspace-qualified promotion uses the citing project's final site verdicts,
/// not the rejected pre-promotion tokenization (§FS-workspace.8,
/// §FS-inline-citation-style.3.1).
#[test]
fn off_grammar_qualified_promotion_drives_inline_style_and_layout() {
    let root = test_root("qualified-inline");
    workspace_inline_fixture(
        &root,
        "citation-only",
        false,
        "// \u{a7}api/FS-security-providers\n",
    );
    let pure = run(&root, &["check"]);
    assert_code(&pure, 0, "qualified citation-only");
    assert!(!stdout(&pure).contains("inline citation must carry no prose"));

    workspace_inline_fixture(
        &root,
        "citation-with-note",
        true,
        "// Rationale before \u{a7}api/FS-security-providers\n",
    );
    let laid_out = run(&root, &["check"]);
    assert_code(&laid_out, 1, "qualified layout");
    assert!(
        stdout(&laid_out).contains("inline note must open with its citations and a colon"),
        "{}",
        stdout(&laid_out)
    );
}

/// Text and JSON reads flatten local and qualified wrappers emitted by the
/// Markdown cross-reference pass while inline-code illustrations remain exact
/// (§FS-show.3.2, §FS-fmt.6.1, §FS-fmt.6.4).
#[test]
fn off_grammar_show_flattens_local_and_qualified_formatter_wrappers() {
    let root = test_root("show-flatten");
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
         [[kinds]]\nkind = \"FS\"\nfolder = \"docs/specs\"\nindex = false\n\n\
         [scan]\ninclude = [\"docs\"]\n",
    );
    write(
        &root,
        "api/docs/specs/security.md",
        "# FS-security-providers: Security providers\n\nStable.\n",
    );
    write(
        &root,
        "client/grund.toml",
        "grund_config_version = 1\nproject_name = \"client\"\n\n\
         [id]\nformat = \"{kind}-{number}-{slug}\"\n\n\
         [[kinds]]\nkind = \"FS\"\nfolder = \"docs/specs\"\nindex = false\n\n\
         [scan]\ninclude = [\"docs\"]\n",
    );
    write(
        &root,
        "client/docs/specs/legacy.md",
        "# FS-client-legacy: Client legacy\n\nStable.\n",
    );
    write(
        &root,
        "client/docs/specs/consumer.md",
        "# FS-001-consumer: Consumer\n\n\
         Follows \u{a7}FS-client-legacy and \u{a7}api/FS-security-providers.\n\n\
         Keeps the ordinary [\u{a7}not-a-citation](manual.md) link and [documentation](manual.md).\n\n\
         Keeps `[\u{a7}FS-client-legacy](manual.md)` illustrative.\n",
    );

    let fmt = run(&root, &["fmt", "--cross-refs", "--write"]);
    assert_code(&fmt, 0, "fmt cross refs");
    let markdown =
        fs::read_to_string(root.join("client/docs/specs/consumer.md")).expect("read consumer");
    assert!(markdown.contains("[\u{a7}FS-client-legacy]("), "{markdown}");
    assert!(
        markdown.contains("[\u{a7}api/FS-security-providers]("),
        "{markdown}"
    );
    assert!(
        markdown.contains("`[\u{a7}FS-client-legacy](manual.md)`"),
        "{markdown}"
    );

    for (label, args) in [
        ("text", vec!["show", "client/FS-001-consumer", "--full"]),
        (
            "json",
            vec![
                "show",
                "client/FS-001-consumer",
                "--full",
                "--format",
                "json",
            ],
        ),
    ] {
        let shown = run(&root, &args);
        assert_code(&shown, 0, label);
        let body = stdout(&shown);
        assert!(
            body.contains("Follows \u{a7}FS-client-legacy and \u{a7}api/FS-security-providers."),
            "{label}: {body}"
        );
        assert!(!body.contains("Follows [\u{a7}"), "{label}: {body}");
        assert!(
            body.contains(
                "Keeps the ordinary [\u{a7}not-a-citation](manual.md) link and \
                 [documentation](manual.md)."
            ),
            "{label}: {body}"
        );
        assert!(
            body.contains("`[\u{a7}FS-client-legacy](manual.md)`"),
            "{label}: {body}"
        );
    }
}
