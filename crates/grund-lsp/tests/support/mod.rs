//! Shared stdio harness for the `grund-lsp` integration suites: spawn the
//! server, frame JSON-RPC over stdin/stdout, and wait for a response, a
//! diagnostics push, or exit (§AR-lsp.4).
//!
//! Extracted so cases group by the request under test instead of by the file
//! the first case happened to land in; each suite compiles this module into its
//! own test binary, so helpers one suite does not reach for are not dead code.
#![allow(dead_code)]

use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub fn test_root(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "grund-lsp-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(".agents")).expect("create config dir");
    fs::create_dir_all(dir.join("docs")).expect("create docs dir");
    fs::write(
        dir.join(".agents/grund.toml"),
        "grund_config_version = 1\n[scan]\ninclude = [\"docs\"]\nextensions = [\"md\"]\n",
    )
    .expect("write config");
    dir
}

pub fn file_uri(path: &Path) -> String {
    let path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    url::Url::from_file_path(path)
        .expect("file uri")
        .to_string()
}

pub fn send_message(stdin: &mut impl Write, message: Value) {
    let body = serde_json::to_vec(&message).expect("serialize message");
    write!(stdin, "Content-Length: {}\r\n\r\n", body.len()).expect("write header");
    stdin.write_all(&body).expect("write body");
    stdin.flush().expect("flush message");
}

pub fn read_messages(stdout: impl Read + Send + 'static) -> mpsc::Receiver<Value> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut stdout = BufReader::new(stdout);
        loop {
            let mut content_length = None;
            loop {
                let mut line = String::new();
                let Ok(bytes) = stdout.read_line(&mut line) else {
                    return;
                };
                if bytes == 0 {
                    return;
                }
                let line = line.trim_end_matches(['\r', '\n']);
                if line.is_empty() {
                    break;
                }
                if let Some(length) = line.strip_prefix("Content-Length: ") {
                    content_length = length.parse::<usize>().ok();
                }
            }
            let Some(content_length) = content_length else {
                return;
            };
            let mut body = vec![0; content_length];
            if stdout.read_exact(&mut body).is_err() {
                return;
            }
            let Ok(message) = serde_json::from_slice(&body) else {
                return;
            };
            let _ = sender.send(message);
        }
    });
    receiver
}

pub fn recv_response(receiver: &mpsc::Receiver<Value>, id: i64) -> Result<Value, String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(format!("timed out waiting for LSP response {id}"));
        }
        let message = receiver
            .recv_timeout(remaining)
            .map_err(|err| format!("receive LSP message: {err}"))?;
        if message.get("id").and_then(Value::as_i64) == Some(id) {
            return Ok(message);
        }
    }
}

pub fn recv_response_or_panic(
    receiver: &mpsc::Receiver<Value>,
    child: &mut Child,
    id: i64,
) -> Value {
    match recv_response(receiver, id) {
        Ok(message) => message,
        Err(err) => {
            if child.try_wait().expect("poll child").is_none() {
                let _ = child.kill();
                let _ = child.wait();
            }
            let mut stderr = String::new();
            if let Some(child_stderr) = child.stderr.as_mut() {
                let _ = child_stderr.read_to_string(&mut stderr);
            }
            panic!("{err}; server stderr: {stderr}");
        }
    }
}

pub fn recv_diagnostics(
    receiver: &mpsc::Receiver<Value>,
    child: &mut Child,
    uri_suffix: &str,
) -> Vec<Value> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let outcome = if remaining.is_zero() {
            Err(mpsc::RecvTimeoutError::Timeout)
        } else {
            receiver.recv_timeout(remaining)
        };
        match outcome {
            Ok(message) => {
                if message.get("method").and_then(Value::as_str)
                    == Some("textDocument/publishDiagnostics")
                    && message["params"]["uri"]
                        .as_str()
                        .is_some_and(|uri| uri.contains(uri_suffix))
                {
                    let diagnostics = message["params"]["diagnostics"]
                        .as_array()
                        .cloned()
                        .unwrap_or_default();
                    if !diagnostics.is_empty() {
                        return diagnostics;
                    }
                }
            }
            Err(_) => {
                if child.try_wait().expect("poll child").is_none() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
                let mut stderr = String::new();
                if let Some(child_stderr) = child.stderr.as_mut() {
                    let _ = child_stderr.read_to_string(&mut stderr);
                }
                panic!(
                    "no diagnostics matching {uri_suffix} before timeout; server stderr: {stderr}"
                );
            }
        }
    }
}

pub fn wait_for_exit(child: &mut Child) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = child.try_wait().expect("poll child") {
            assert!(status.success(), "grund-lsp exited with {status}");
            return;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("grund-lsp did not exit after shutdown/exit");
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

/// Spawn a server rooted at `root` and complete the `initialize` /
/// `initialized` handshake (§FS-lsp.2.2) as a client that advertises nothing,
/// leaving request id `1` spent — a case starts its own requests at `2`.
pub fn start_server(root: &Path) -> (Child, ChildStdin, mpsc::Receiver<Value>) {
    start_server_with_capabilities(root, json!({}))
}

/// The same handshake for a case whose subject is what the server does with a
/// declared client capability (`textDocument.definition.linkSupport`, say):
/// `capabilities` is sent verbatim as the `initialize` client capabilities.
pub fn start_server_with_capabilities(
    root: &Path,
    capabilities: Value,
) -> (Child, ChildStdin, mpsc::Receiver<Value>) {
    start_server_with_initialize(
        root,
        json!({
            "processId": std::process::id(),
            "rootUri": file_uri(root),
            "capabilities": capabilities
        }),
    )
}

/// Start a server whose client supplies the given LSP workspace folders. The
/// process cwd is independent so cases can prove the initialize payload, not
/// an ambient directory, determines project discovery (§FS-lsp.2.2).
pub fn start_server_with_workspace_folders(
    current_dir: &Path,
    folders: &[&Path],
) -> (Child, ChildStdin, mpsc::Receiver<Value>) {
    let workspace_folders = folders
        .iter()
        .map(|folder| {
            json!({
                "uri": file_uri(folder),
                "name": folder.file_name().and_then(|name| name.to_str()).unwrap_or("root")
            })
        })
        .collect::<Vec<_>>();
    start_server_with_initialize(
        current_dir,
        json!({
            "processId": std::process::id(),
            "workspaceFolders": workspace_folders,
            "capabilities": { "workspace": { "workspaceFolders": true } }
        }),
    )
}

fn start_server_with_initialize(
    current_dir: &Path,
    initialize_params: Value,
) -> (Child, ChildStdin, mpsc::Receiver<Value>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_grund-lsp"))
        .current_dir(current_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn grund-lsp");
    let mut stdin = child.stdin.take().expect("child stdin");
    let receiver = read_messages(child.stdout.take().expect("child stdout"));
    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": initialize_params
        }),
    );
    recv_response_or_panic(&receiver, &mut child, 1);
    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        }),
    );
    (child, stdin, receiver)
}

/// The `textDocument/hover` result for one position, as the server returns it.
pub fn hover_result(
    stdin: &mut ChildStdin,
    receiver: &mpsc::Receiver<Value>,
    child: &mut Child,
    id: i64,
    uri: &str,
    line: i64,
    character: i64,
) -> Value {
    send_message(
        stdin,
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }
        }),
    );
    recv_response_or_panic(receiver, child, id)["result"].clone()
}
