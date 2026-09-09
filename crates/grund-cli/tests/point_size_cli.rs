//! CLI grammar and launch-failure contract for point sizes (§FS-list.1,
//! §FS-list.3.4, §FS-list.4).

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/point-size-cli")
        .join(std::process::id().to_string());
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create point-size fixture");
    root
}

fn write(root: &Path, relative: &str, body: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create fixture parent");
    }
    fs::write(path, body).expect("write fixture");
}

fn run(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_grund"))
        .args(args)
        .current_dir(root)
        .output()
        .unwrap_or_else(|error| panic!("run grund {args:?}: {error}"))
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn assert_error(root: &Path, args: &[&str], expected: &str) {
    let output = run(root, args);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(text(&output.stdout), "");
    assert_eq!(text(&output.stderr), expected);
}

#[test]
fn size_and_top_grammar_rejects_every_invalid_or_ambiguous_form_before_scanning() {
    let root = root();
    let missing = root.join("missing");
    let missing = missing.to_str().unwrap();
    let cases: &[(&[&str], &str)] = &[
        (
            &["list", missing, "--size="],
            "error: --size requires one or more units\n",
        ),
        (
            &["list", missing, "--size=words,"],
            "error: --size requires one or more units\n",
        ),
        (
            &["list", missing, "--size=tokens"],
            "error: unknown size unit `tokens` (expected lines, words, or bytes)\n",
        ),
        (
            &["list", missing, "--size=Words"],
            "error: unknown size unit `Words` (expected lines, words, or bytes)\n",
        ),
        (
            &["list", missing, "--size", "--size=words"],
            "error: --size may only appear once\n",
        ),
        (
            &["list", missing, "--top=1"],
            "error: --top requires --size\n",
        ),
        (
            &["list", missing, "--size", "--top=0"],
            "error: --top requires a positive integer\n",
        ),
        (
            &["list", missing, "--size", "--top=-1"],
            "error: --top requires a positive integer\n",
        ),
        (
            &["list", missing, "--size", "--top=x"],
            "error: --top requires a positive integer\n",
        ),
        (
            &["list", missing, "--size", "--top"],
            "error: --top requires a positive integer\n",
        ),
        (
            &["list", missing, "--size", "--top=1", "--top", "2"],
            "error: --top may only appear once\n",
        ),
        (
            &["list", missing, "--size", "--summary"],
            "error: --size cannot be combined with --summary\n",
        ),
        (
            &["list", missing, "--top=1", "--summary"],
            "error: --top cannot be combined with --summary\n",
        ),
    ];
    for (args, expected) in cases {
        assert_error(&root, args, expected);
    }

    let scan_failure = run(&root, &["list", missing, "--size"]);
    assert_eq!(scan_failure.status.code(), Some(2));
    assert_eq!(text(&scan_failure.stdout), "");
    assert!(
        text(&scan_failure.stderr).contains(missing),
        "valid size query did not reach scan: {}",
        text(&scan_failure.stderr)
    );

    write(
        &root,
        "words/grund.toml",
        "grund_config_version = 1\n\
         [reference]\nstrict = true\n\
         [id]\nformat = \"{kind}-{slug}\"\n\
         [[kinds]]\nkind = \"FS\"\nfolder = \"docs\"\nindex = false\n\
         [scan]\ninclude = [\"docs\"]\n",
    );
    write(
        &root,
        "words/docs/FS-path.md",
        "# FS-path: Path\n\nPath body.\n",
    );
    let separated = run(&root, &["list", "--size", "words", "--format=json"]);
    assert_eq!(separated.status.code(), Some(0));
    let row: Value = serde_json::from_slice(&separated.stdout).expect("size row JSON");
    assert_eq!(row["id"], "FS-path");

    let deduplicated = run(
        &root.join("words"),
        &["list", "--size=bytes,words,bytes", "--format=json"],
    );
    assert_eq!(deduplicated.status.code(), Some(0));
    let raw = text(&deduplicated.stdout);
    assert_eq!(raw.matches("lead_bytes").count(), 1);
    assert!(raw.find("lead_bytes").unwrap() < raw.find("lead_words").unwrap());

    let _ = fs::remove_dir_all(root);
}
