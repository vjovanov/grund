//! Which discovered project answers for a document, and what the session does
//! when one of the editor's folders cannot be turned into a project at all
//! (§FS-lsp.2.2, §AR-lsp.2).
//!
//! Separate from `workspace_folders.rs`, which covers discovery and the folder
//! lifecycle: these cases fail together for a different reason — the mapping
//! from document to snapshot, not the mapping from folder to project.

mod support;

use serde_json::{Value, json};
use std::fs;
use std::io::Read;
#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin};
use std::sync::mpsc;
use support::*;

/// A project whose spec declares `FS-001-example` and whose source cites it.
fn write_project(root: &Path, citation_name: &str) -> (PathBuf, PathBuf) {
    fs::write(
        root.join("grund.toml"),
        "grund_config_version = 1\n[scan]\ninclude = [\"docs\", \"src\"]\n\
         extensions = [\"md\", \"java\"]\n",
    )
    .expect("write config");
    let spec = root.join("docs/FS-001-example.md");
    let source = root.join(format!("src/{citation_name}.java"));
    write(&spec, "# FS-001-example: Example\n\nLead.\n");
    write(
        &source,
        "/// Implements §FS-001-example.\nfinal class Example {}\n",
    );
    (spec, source)
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
    fs::write(path, text).expect("write fixture");
}

/// Hover over the `§FS-001-example` citation on the third line of a spec body.
fn hover_citation(
    child: &mut Child,
    stdin: &mut ChildStdin,
    receiver: &mpsc::Receiver<Value>,
    id: i64,
    path: &Path,
) -> Value {
    hover_result(stdin, receiver, child, id, &file_uri(path), 2, 6)
}

fn references(
    child: &mut Child,
    stdin: &mut ChildStdin,
    receiver: &mpsc::Receiver<Value>,
    id: i64,
    spec: &Path,
) -> Vec<Value> {
    send_message(
        stdin,
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "textDocument/references",
            "params": {
                "textDocument": { "uri": file_uri(spec) },
                "position": { "line": 0, "character": 8 },
                "context": { "includeDeclaration": false }
            }
        }),
    );
    recv_response_or_panic(receiver, child, id)["result"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

fn cites(locations: &[Value], path: &Path) -> bool {
    let uri = file_uri(path);
    locations
        .iter()
        .any(|location| location["uri"].as_str() == Some(uri.as_str()))
}

/// Every `publishDiagnostics` push naming `uri_needle`, newest last. Unlike
/// `recv_diagnostics` this keeps empty pushes and does not stop at the first
/// hit, because the subject here is how *many* verdicts a file collects.
fn diagnostic_pushes(receiver: &mpsc::Receiver<Value>, uri_needle: &str) -> Vec<Vec<Value>> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut pushes = Vec::new();
    while std::time::Instant::now() < deadline {
        let Ok(message) = receiver.recv_timeout(std::time::Duration::from_millis(300)) else {
            continue;
        };
        if message.get("method").and_then(Value::as_str) == Some("textDocument/publishDiagnostics")
            && message["params"]["uri"]
                .as_str()
                .is_some_and(|uri| uri.contains(uri_needle))
        {
            pushes.push(
                message["params"]["diagnostics"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default(),
            );
        }
    }
    pushes
}

fn diagnostic_codes(diagnostics: &[Value]) -> Vec<&str> {
    diagnostics
        .iter()
        .map(|diagnostic| diagnostic["code"].as_str().unwrap_or("?"))
        .collect()
}

fn stop_server(
    child: &mut Child,
    stdin: &mut ChildStdin,
    receiver: &mpsc::Receiver<Value>,
    id: i64,
) {
    send_message(
        stdin,
        json!({ "jsonrpc": "2.0", "id": id, "method": "shutdown", "params": null }),
    );
    recv_response_or_panic(receiver, child, id);
    send_message(stdin, json!({ "jsonrpc": "2.0", "method": "exit" }));
}

fn folder_list(folders: &[(&Path, &str)]) -> Value {
    folders
        .iter()
        .map(|(path, name)| json!({ "uri": file_uri(path), "name": name }))
        .collect()
}

#[test]
#[cfg(unix)]
fn a_symlinked_include_root_outside_the_project_still_answers() {
    // `[scan] include` is a scan scope, not a fence: a symlinked include
    // resolves outside the project root, so a root prefix alone cannot decide
    // which project owns a document (§FS-lsp.2.2).
    let base = std::env::temp_dir().join(format!("grund-lsp-symlink-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    let root = base.join("repo");
    let outside = base.join("outside-docs");
    fs::write(
        root.join("grund.toml"),
        "grund_config_version = 1\n[scan]\ninclude = [\"docs\"]\nextensions = [\"md\"]\n",
    )
    .expect("write config");
    write(
        &outside.join("FS-001-example.md"),
        "# FS-001-example: Example\n\nLead.\n",
    );
    write(
        &outside.join("FS-002-user.md"),
        "# FS-002-user: User\n\nSee §FS-001-example, and §FS-404-gone.\n",
    );
    symlink(&outside, root.join("docs")).expect("symlink the include root");

    let (mut child, mut stdin, receiver) = start_server_with_workspace_folders(&root, &[&root]);
    // Diagnostics first: a request read discards the notifications queued
    // behind it. The owning project publishes for this file even though it is
    // not under the project root — the ownership filter must not drop it.
    let diagnostics = recv_diagnostics(&receiver, &mut child, "FS-002-user");
    assert!(
        diagnostic_codes(&diagnostics).contains(&"dangling"),
        "the dangling citation is still reported: {diagnostics:?}"
    );
    let hover = hover_citation(
        &mut child,
        &mut stdin,
        &receiver,
        2,
        &root.join("docs/FS-002-user.md"),
    );
    assert!(
        hover["contents"]["value"]
            .as_str()
            .is_some_and(|body| body.contains("FS-001-example")),
        "a citation in a symlinked include root must still resolve: {hover}"
    );

    stop_server(&mut child, &mut stdin, &receiver, 3);
    drop(stdin);
    wait_for_exit(&mut child);
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn a_parent_relative_include_root_still_answers() {
    let base = std::env::temp_dir().join(format!("grund-lsp-parent-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    let root = base.join("repo");
    let shared = base.join("shared");
    fs::write(
        root.join("grund.toml"),
        "grund_config_version = 1\n[scan]\ninclude = [\"docs\", \"../shared\"]\nextensions = [\"md\"]\n",
    )
    .expect("write config");
    write(
        &root.join("docs/FS-001-example.md"),
        "# FS-001-example: Example\n\nLead.\n",
    );
    write(
        &shared.join("FS-002-user.md"),
        "# FS-002-user: User\n\nSee §FS-001-example, and §FS-404-gone.\n",
    );

    let (mut child, mut stdin, receiver) = start_server_with_workspace_folders(&root, &[&root]);
    let diagnostics = recv_diagnostics(&receiver, &mut child, "FS-002-user");
    assert!(
        diagnostic_codes(&diagnostics).contains(&"dangling"),
        "the dangling citation is still reported: {diagnostics:?}"
    );
    let hover = hover_citation(
        &mut child,
        &mut stdin,
        &receiver,
        2,
        &shared.join("FS-002-user.md"),
    );
    assert!(
        hover["contents"]["value"]
            .as_str()
            .is_some_and(|body| body.contains("FS-001-example")),
        "a citation under a parent-relative include root must still resolve: {hover}"
    );

    stop_server(&mut child, &mut stdin, &receiver, 3);
    drop(stdin);
    wait_for_exit(&mut child);
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn a_file_two_projects_scan_collects_one_verdict() {
    // A workspace root and one of its members, both opened as editor folders,
    // scan the member's files. The member owns them, so its verdict is the one
    // published — never both snapshots' findings merged (§FS-lsp.2.2).
    let root = std::env::temp_dir().join(format!("grund-lsp-overlap-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    write(
        &root.join("grund.toml"),
        "grund_config_version = 1\n[scan]\ninclude = [\"docs\"]\nextensions = [\"md\"]\n\
         [workspace]\nmembers = [\"packages/app\"]\ninclude_root = true\n",
    );
    write(
        &root.join("docs/FS-001-root.md"),
        "# FS-001-root: Root\n\nLead.\n",
    );
    let member = root.join("packages/app");
    write(
        &member.join("grund.toml"),
        "grund_config_version = 1\n[scan]\ninclude = [\"docs\"]\nextensions = [\"md\"]\n",
    );
    write(
        &member.join("docs/FS-002-app.md"),
        "# FS-002-app: App\n\nLead.\n",
    );

    let (mut child, mut stdin, receiver) =
        start_server_with_workspace_folders(&root, &[&root, &member]);
    let pushes = diagnostic_pushes(&receiver, "FS-002-app");
    let latest = pushes.last().cloned().unwrap_or_default();
    assert_eq!(
        diagnostic_codes(&latest),
        vec!["unused"],
        "the owning project states the verdict once: {latest:?}"
    );

    stop_server(&mut child, &mut stdin, &receiver, 3);
    drop(stdin);
    wait_for_exit(&mut child);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_virtual_workspace_folder_is_skipped_not_fatal() {
    // VS Code multi-root windows mix local folders with virtual ones. A folder
    // grund cannot read must not take the local folders beside it down
    // (§FS-lsp.2.2, §REQ-never-crashes).
    let root = std::env::temp_dir().join(format!("grund-lsp-virtual-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::write(
        root.join("grund.toml"),
        "grund_config_version = 1\n[scan]\ninclude = [\"docs\"]\nextensions = [\"md\"]\n",
    )
    .expect("write config");
    write(
        &root.join("docs/FS-001-example.md"),
        "# FS-001-example: Example\n\nLead.\n",
    );
    write(
        &root.join("docs/FS-002-user.md"),
        "# FS-002-user: User\n\nSee §FS-001-example.\n",
    );

    let (mut child, mut stdin, receiver) = start_server_with_initialize(
        &root,
        json!({
            "processId": std::process::id(),
            "workspaceFolders": [
                { "uri": "vscode-vfs://github/acme/repo", "name": "virtual" },
                { "uri": file_uri(&root), "name": "local" }
            ],
            "capabilities": { "workspace": { "workspaceFolders": true } }
        }),
    );
    let hover = hover_citation(
        &mut child,
        &mut stdin,
        &receiver,
        2,
        &root.join("docs/FS-002-user.md"),
    );
    assert!(
        hover["contents"]["value"]
            .as_str()
            .is_some_and(|body| body.contains("FS-001-example")),
        "the local folder keeps working beside a virtual one: {hover}"
    );

    stop_server(&mut child, &mut stdin, &receiver, 3);
    drop(stdin);
    wait_for_exit(&mut child);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_folder_whose_config_will_not_load_leaves_the_others_serving() {
    // One half-typed `grund.toml` used to abort the whole batch, so the folder
    // beside it lost every feature and the failure only reached stderr
    // (§FS-lsp.2.2, §REQ-never-crashes).
    let root = std::env::temp_dir().join(format!("grund-lsp-broken-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let good = root.join("good");
    let bad = root.join("bad");
    fs::write(
        good.join("grund.toml"),
        "grund_config_version = 1\n[scan]\ninclude = [\"docs\"]\nextensions = [\"md\"]\n",
    )
    .expect("write config");
    write(
        &good.join("docs/FS-001-example.md"),
        "# FS-001-example: Example\n\nLead.\n",
    );
    write(
        &good.join("docs/FS-002-user.md"),
        "# FS-002-user: User\n\nSee §FS-001-example.\n",
    );
    write(&bad.join("grund.toml"), "grund_config_version = 1\n[scan\n");

    let (mut child, mut stdin, receiver) = start_server_with_initialize(
        &root,
        json!({
            "processId": std::process::id(),
            "workspaceFolders": folder_list(&[(&good, "good"), (&bad, "bad")]),
            "capabilities": { "workspace": { "workspaceFolders": true } }
        }),
    );
    let hover = hover_citation(
        &mut child,
        &mut stdin,
        &receiver,
        2,
        &good.join("docs/FS-002-user.md"),
    );
    assert!(
        hover["contents"]["value"]
            .as_str()
            .is_some_and(|body| body.contains("FS-001-example")),
        "the loadable folder keeps working: {hover}"
    );

    stop_server(&mut child, &mut stdin, &receiver, 3);
    drop(stdin);
    wait_for_exit(&mut child);
    let mut stderr = String::new();
    if let Some(child_stderr) = child.stderr.as_mut() {
        let _ = child_stderr.read_to_string(&mut stderr);
    }
    assert!(
        stderr.contains("no usable config"),
        "the skipped folder is reported rather than swallowed: {stderr}"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn an_edit_in_one_project_leaves_the_others_answering() {
    // Only the projects that can see the edited document are rebuilt, so this
    // pins that the edited one *does* update and the untouched one does not go
    // stale or vanish (§AR-lsp.2).
    let root = std::env::temp_dir().join(format!("grund-lsp-scoped-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let first_root = root.join("first");
    let second_root = root.join("second");
    let (first_spec, first_source) = write_project(&first_root, "FirstUse");
    let (second_spec, second_source) = write_project(&second_root, "SecondUse");

    let (mut child, mut stdin, receiver) =
        start_server_with_workspace_folders(&root, &[&first_root, &second_root]);
    assert!(cites(
        &references(&mut child, &mut stdin, &receiver, 2, &first_spec),
        &first_source
    ));

    // Drop the citation from the first project's source, in the buffer only.
    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": file_uri(&first_source), "version": 2 },
                "contentChanges": [{ "text": "final class Example {}\n" }]
            }
        }),
    );

    assert!(
        !cites(
            &references(&mut child, &mut stdin, &receiver, 3, &first_spec),
            &first_source
        ),
        "the edited document's own project is rebuilt"
    );
    assert!(
        cites(
            &references(&mut child, &mut stdin, &receiver, 4, &second_spec),
            &second_source
        ),
        "a project that cannot see the edited document keeps answering"
    );

    stop_server(&mut child, &mut stdin, &receiver, 5);
    drop(stdin);
    wait_for_exit(&mut child);
    let _ = fs::remove_dir_all(&root);
}
