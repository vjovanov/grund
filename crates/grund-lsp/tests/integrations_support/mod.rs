//! Black-box helpers for the installed editor-template contract (§FS-lsp.2.4).

#![allow(dead_code)]

use serde_json::{Value, json};
use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

const EXECUTABLE_ENV: &str = "GRUND_LSP_LSP4IJ_EXECUTABLE";
const CMD_QUOTE_ENV: &str = "GRUND_LSP_LSP4IJ_QUOTE";

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
        Self::with_binary_dir(
            source,
            label,
            "installed binary β & '() $;! %PATH% ^ with spaces",
        )
    }

    pub fn with_binary_dir(source: &Path, label: &str, binary_dir: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "grund-lsp-integrations-{label}-{}-{nonce}",
            std::process::id()
        ));
        let binary_dir = root.join(binary_dir);
        fs::create_dir_all(&binary_dir).expect("create isolated binary directory");
        let binary = binary_dir.join(format!("grund-lsp{}", std::env::consts::EXE_SUFFIX));
        crate::support::copy_executable(
            source,
            &binary,
            "copy grund-lsp into isolated installation",
        );
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
    let mut command = Command::new(binary);
    command
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = crate::support::spawn_child(&mut command, "run isolated grund-lsp");
    child
        .wait_with_output()
        .expect("wait for isolated grund-lsp")
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

    assert_program_arg_boundaries(
        template,
        r"C:\repo%PATH%\project Ω & '() $;! ^ with spaces",
        executable,
    );
    let executable = executable.to_string_lossy();
    let args = template["programArgs"]
        .as_object()
        .expect("programArgs object");
    assert_eq!(args.len(), 2, "only host command forms are emitted");
    assert_eq!(template["workingDir"], "$PROJECT_DIR$");
    assert_eq!(
        template["env"]["variables"][EXECUTABLE_ENV],
        executable.as_ref()
    );
    for key in ["default", "windows"] {
        let command = args[key].as_str().expect("command string");
        assert!(
            !command.contains("$PROJECT_DIR$") && !command.contains(executable.as_ref()),
            "{key} exposes path bytes to command tokenization: {command}"
        );
        assert!(command.contains(EXECUTABLE_ENV));
        assert!(!command.contains("cargo run"), "{key} depends on Cargo");
    }
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

/// Mirror LSP4IJ `CommandUtils.createCommands`: only double quotes group text,
/// and those grouping quotes are removed from the resulting argument.
pub fn lsp4ij_commands(command_line: &str) -> Vec<String> {
    let mut commands = Vec::new();
    let mut command_part = String::new();
    let mut in_string = false;
    for ch in command_line.chars() {
        match ch {
            '"' => in_string = !in_string,
            ' ' if !in_string => {
                if !command_part.trim().is_empty() {
                    commands.push(command_part.trim().to_owned());
                }
                command_part.clear();
            }
            _ => command_part.push(ch),
        }
    }
    if !command_part.trim().is_empty() {
        commands.push(command_part.trim().to_owned());
    }
    commands
}

fn assert_program_arg_boundaries(template: &Value, project: &str, executable: &Path) {
    let args = template["programArgs"]
        .as_object()
        .expect("programArgs object");
    let executable = executable.to_string_lossy();

    let default = args["default"].as_str().expect("default command");
    assert_eq!(
        lsp4ij_commands(&default),
        vec![
            "sh".to_owned(),
            "-c".to_owned(),
            format!("set -f; IFS=; exec ${EXECUTABLE_ENV}"),
        ],
        "default command does not survive LSP4IJ tokenization"
    );

    assert_eq!(template["env"]["includeSystemEnvironmentVariables"], true);
    assert_eq!(template["env"]["variables"][CMD_QUOTE_ENV], "\"");
    assert_eq!(
        template["env"]["variables"][EXECUTABLE_ENV],
        executable.as_ref()
    );
    let windows = args["windows"].as_str().expect("windows command");
    let windows = lsp4ij_commands(&windows);
    assert_eq!(
        &windows[..5],
        ["cmd", "/D", "/V:OFF", "/S", "/C"],
        "windows shell arguments changed"
    );
    assert_eq!(windows.len(), 6, "windows script split during tokenization");
    let expanded_script = expand_cmd_variables_once(
        &windows[5],
        &[(CMD_QUOTE_ENV, "\""), (EXECUTABLE_ENV, &executable)],
    );
    assert_eq!(
        expanded_script,
        format!("\"{executable}\""),
        "windows executable is not quoted after one cmd expansion"
    );
    assert_eq!(
        template["workingDir"]
            .as_str()
            .expect("working directory")
            .replace("$PROJECT_DIR$", project),
        project,
        "Windows project macro changed before reaching the process working directory"
    );
}

/// Model cmd's single percent-variable expansion pass. Percent-delimited text
/// introduced by a variable's value is data and is not expanded recursively.
fn expand_cmd_variables_once(script: &str, variables: &[(&str, &str)]) -> String {
    let mut expanded = String::new();
    let mut rest = script;
    while let Some(start) = rest.find('%') {
        expanded.push_str(&rest[..start]);
        let after_start = &rest[start + 1..];
        let Some(end) = after_start.find('%') else {
            expanded.push_str(&rest[start..]);
            return expanded;
        };
        let name = &after_start[..end];
        if let Some((_, value)) = variables.iter().find(|(candidate, _)| *candidate == name) {
            expanded.push_str(value);
        }
        rest = &after_start[end + 1..];
    }
    expanded.push_str(rest);
    expanded
}

/// Tokenize exactly as LSP4IJ does, substitute its project macro, and prove
/// the host-selected command reaches the copied stdio server from that root.
pub fn assert_host_command_launches(template: &Value, project: &Path) {
    let key = if cfg!(windows) { "windows" } else { "default" };
    let project_text = project.to_str().expect("UTF-8 project path");
    let command_line = template["programArgs"][key].as_str().expect("host command");
    let commands = lsp4ij_commands(&command_line);
    let working_dir = template["workingDir"]
        .as_str()
        .expect("host working directory")
        .replace("$PROJECT_DIR$", project_text);
    let mut command = Command::new(&commands[0]);
    command
        .args(&commands[1..])
        .current_dir(&working_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (name, value) in template["env"]["variables"]
        .as_object()
        .expect("host environment")
    {
        command.env(name, value.as_str().expect("environment value"));
    }
    let mut child = crate::support::spawn_child(&mut command, "launch generated host command");
    let mut stdin = child.stdin.take().expect("host command stdin");
    let receiver = crate::support::read_messages(child.stdout.take().expect("host command stdout"));
    crate::support::send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "capabilities": {}
            }
        }),
    );
    let initialized = crate::support::recv_response_or_panic(&receiver, &mut child, 1);
    assert_eq!(
        initialized["result"]["capabilities"]["documentOnTypeFormattingProvider"]["firstTriggerCharacter"],
        "%",
        "generated command did not start the server from the project root"
    );
    crate::support::send_message(
        &mut stdin,
        json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }),
    );
    crate::support::send_message(
        &mut stdin,
        json!({ "jsonrpc": "2.0", "id": 2, "method": "shutdown", "params": null }),
    );
    crate::support::recv_response_or_panic(&receiver, &mut child, 2);
    crate::support::send_message(&mut stdin, json!({ "jsonrpc": "2.0", "method": "exit" }));
    stdin.flush().expect("flush exit notification");
    drop(stdin);
    crate::support::wait_for_exit(&mut child);
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
