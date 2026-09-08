//! Black-box helpers for the installed editor-template contract (§FS-lsp.2.4).

#![allow(dead_code)]

use serde_json::{Value, json};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

pub const DEFAULT_EXTENSIONS: &[&str] = &[
    "md", "rs", "go", "java", "kt", "ts", "tsx", "js", "py", "c", "cpp", "swift", "scala", "rb",
    "php", "cs", "lisp", "scm", "clj", "sql", "hs", "lhs", "lua", "ada", "adb", "ads",
];

pub struct Sandbox {
    pub root: PathBuf,
    pub binary: PathBuf,
}

impl Sandbox {
    pub fn with_binary(source: &Path, label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "grund-lsp-integrations-{label}-{}-{nonce}",
            std::process::id()
        ));
        let binary_dir = root.join("installed binary β with spaces");
        fs::create_dir_all(&binary_dir).expect("create isolated binary directory");
        let binary = binary_dir.join(format!("grund-lsp{}", std::env::consts::EXE_SUFFIX));
        fs::copy(source, &binary).expect("copy grund-lsp into isolated installation");
        Self { root, binary }
    }

    pub fn project(&self, name: &str) -> PathBuf {
        let path = self.root.join(name);
        fs::create_dir_all(&path).expect("create isolated project");
        path
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub fn workspace_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_grund-lsp"))
}

pub fn run<I, S>(binary: &Path, cwd: &Path, args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new(binary)
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("run isolated grund-lsp")
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8")
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is UTF-8")
}

pub fn assert_success(output: &Output, context: &str) {
    assert!(
        output.status.success(),
        "{context} failed with {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        stdout(output),
        stderr(output)
    );
    assert!(stderr(output).is_empty(), "{context} wrote stderr");
}

pub fn template_from(text: &str) -> Value {
    for (offset, ch) in text.char_indices() {
        if ch != '{' {
            continue;
        }
        let mut values = serde_json::Deserializer::from_str(&text[offset..]).into_iter::<Value>();
        if let Some(Ok(value)) = values.next()
            && value.get("programArgs").is_some()
            && value.get("fileTypeMappings").is_some()
        {
            return value;
        }
    }
    panic!("output does not contain an LSP4IJ template object:\n{text}");
}

pub fn assert_template(template: &Value, extensions: &[&str], executable: &Path) {
    assert_eq!(template["id"], "grund-lsp");
    assert_eq!(template["name"], "grund LSP");

    let executable = executable.to_string_lossy();
    let args = template["programArgs"]
        .as_object()
        .expect("programArgs object");
    assert_eq!(args.len(), 2, "only host command forms are emitted");
    for key in ["default", "windows"] {
        let command = args[key].as_str().expect("command string");
        assert!(command.contains("$PROJECT_DIR$"), "{key} does not set cwd");
        assert!(
            command.contains(executable.as_ref()),
            "{key} does not use the running executable: {command}"
        );
        assert!(!command.contains("cargo run"), "{key} depends on Cargo");
        assert!(
            command.contains(&format!("\"{executable}\""))
                || command.contains(&format!("'{executable}'")),
            "{key} does not quote the executable path: {command}"
        );
    }
    assert!(args["default"].as_str().unwrap().contains("sh"));
    assert!(args["windows"].as_str().unwrap().contains("cmd"));

    let expected = extensions
        .iter()
        .map(|extension| {
            json!({
                "fileType": { "patterns": [format!("*.{extension}")] },
                "languageId": language_id(extension),
            })
        })
        .collect::<Vec<_>>();
    let actual = template["fileTypeMappings"]
        .as_array()
        .expect("fileTypeMappings array")
        .iter()
        .map(|mapping| {
            json!({
                "fileType": { "patterns": mapping["fileType"]["patterns"].clone() },
                "languageId": mapping["languageId"].clone(),
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(actual, expected, "generated mapping snapshot changed");
}

fn language_id(extension: &str) -> &str {
    match extension {
        "md" => "markdown",
        "rs" => "rust",
        "kt" => "kotlin",
        "ts" => "typescript",
        "tsx" => "typescriptreact",
        "js" => "javascript",
        "py" => "python",
        "rb" => "ruby",
        "cs" => "csharp",
        "scm" => "scheme",
        "clj" => "clojure",
        "hs" | "lhs" => "haskell",
        "adb" | "ads" => "ada",
        other => other,
    }
}

pub fn entries(path: &Path) -> Vec<String> {
    let mut entries = fs::read_dir(path)
        .expect("read generated directory")
        .map(|entry| {
            entry
                .expect("read generated entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    entries.sort();
    entries
}

pub fn assert_error_2(output: &Output, context: &str) {
    assert_eq!(
        output.status.code(),
        Some(2),
        "{context} did not exit 2\nstdout:\n{}\nstderr:\n{}",
        stdout(output),
        stderr(output)
    );
    assert!(output.stdout.is_empty(), "{context} wrote stdout");
    let error = stderr(output);
    assert!(error.starts_with("error: "), "{context}: {error:?}");
    assert_eq!(
        error.lines().count(),
        1,
        "{context} wrote multiple diagnostics"
    );
}
