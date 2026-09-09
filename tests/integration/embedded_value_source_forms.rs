//! Every source-comment form that can declare an embedded value must also carry
//! that authority through the CLI checker. Missed recognition cannot pass: each
//! form deliberately mismatches (§AR-scanner.2.2, §AR-checker.2.18).

#[path = "binaries.rs"]
mod binaries;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn write(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create fixture directory");
    }
    fs::write(path, text).expect("write fixture");
}

fn source_form_root() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "grund-embedded-source-forms-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create source-form root");
    write(
        &root.join("grund.toml"),
        "grund_config_version = 1\n\n\
         [reference]\nstrict = true\n\n\
         [[kinds]]\nkind = \"FS\"\nfolder = \"src\"\nindex = false\n\n\
         [scan]\ninclude = [\"src\"]\n",
    );
    root
}

fn line_form(id: &str, prefix: &str) -> String {
    format!(
        "{prefix} {id}: Value\n\
         {prefix} ## 1. Price <!-- grund:value -->\n\
         {prefix} ### 1.1. 1200\n\
         {prefix} ## 2. Use\n\
         {prefix} Bound: `999` (§{id}.1.1)\n"
    )
}

#[test]
fn every_promised_source_form_reaches_value_comparison() {
    let root = source_form_root();
    let line_forms = [
        ("FS-001-slash", "slash.rs", "//"),
        ("FS-002-rustdoc", "rustdoc.rs", "///"),
        ("FS-003-module-doc", "module.rs", "//!"),
        ("FS-004-hash", "hash.py", "#"),
        ("FS-005-semicolon", "semicolon.clj", ";"),
        ("FS-006-double-dash", "dash.sql", "--"),
    ];
    for (id, file, prefix) in line_forms {
        write(&root.join("src").join(file), &line_form(id, prefix));
    }
    for (id, file) in [
        ("FS-007-javadoc", "javadoc.java"),
        ("FS-008-jsdoc", "jsdoc.js"),
    ] {
        write(
            &root.join("src").join(file),
            &format!(
                "/**\n\
                 * {id}: Value\n\
                 * ## 1. Price <!-- grund:value -->\n\
                 * ### 1.1. 1200\n\
                 * ## 2. Use\n\
                 * Bound: `999` (§{id}.1.1)\n\
                 */\n"
            ),
        );
    }
    for (id, file, quote) in [
        ("FS-009-double-docstring", "double.py", "\"\"\""),
        ("FS-010-single-docstring", "single.py", "'''"),
    ] {
        write(
            &root.join("src").join(file),
            &format!(
                "{quote}{id}: Value\n\
                 ## 1. Price <!-- grund:value -->\n\
                 ### 1.1. 1200\n\
                 ## 2. Use\n\
                 Bound: `999` (§{id}.1.1)\n\
                 {quote}\n"
            ),
        );
    }

    let output = Command::new(binaries::grund())
        .args(["check"])
        .arg(&root)
        .output()
        .expect("run grund check");
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 stderr");
    assert_eq!(
        output.status.code(),
        Some(1),
        "stdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert_eq!(stdout.matches("value mismatch").count(), 10, "{stdout}");
    assert!(!stdout.contains("invalid value"), "{stdout}");
    for file in [
        "slash.rs",
        "rustdoc.rs",
        "module.rs",
        "hash.py",
        "semicolon.clj",
        "dash.sql",
        "javadoc.java",
        "jsdoc.js",
        "double.py",
        "single.py",
    ] {
        assert!(stdout.contains(file), "no mismatch from {file}: {stdout}");
    }
    let _ = fs::remove_dir_all(root);
}
