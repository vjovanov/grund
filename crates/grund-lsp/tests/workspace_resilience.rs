//! Snapshot-retention and external-scan edge cases for multi-root sessions
//! (§FS-lsp.2.2, §AR-lsp.2).

mod support;

use serde_json::{Value, json};
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin};
use std::sync::mpsc;
use support::*;

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
    fs::write(path, text).expect("write fixture");
}

fn write_project(root: &Path, source_name: &str) -> (PathBuf, PathBuf) {
    write(
        &root.join("grund.toml"),
        "grund_config_version = 1\n[scan]\ninclude = [\"docs\", \"src\"]\n\
         extensions = [\"md\", \"java\"]\n",
    );
    let spec = root.join("docs/FS-001-example.md");
    let source = root.join(format!("src/{source_name}.java"));
    write(&spec, "# FS-001-example: Example\n\nLead.\n");
    write(&source, "/// Uses §FS-001-example.\nfinal class Use {}\n");
    (spec, source)
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

#[test]
fn a_temporarily_invalid_config_keeps_its_snapshot_while_other_projects_refresh() {
    let base = std::env::temp_dir().join(format!("grund-lsp-retain-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    let first = base.join("first");
    let second = base.join("second");
    let (first_spec, first_source) = write_project(&first, "FirstUse");
    let (second_spec, second_source) = write_project(&second, "SecondUse");

    let (mut child, mut stdin, receiver) =
        start_server_with_workspace_folders(&base, &[&first, &second]);
    assert!(cites(
        &references(&mut child, &mut stdin, &receiver, 2, &first_spec),
        &first_source
    ));

    write(
        &first.join("grund.toml"),
        "grund_config_version = 1\n[scan\n",
    );
    write(&second_source, "final class SecondUse {}\n");
    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "workspace/didChangeWatchedFiles",
            "params": { "changes": [] }
        }),
    );

    assert!(
        cites(
            &references(&mut child, &mut stdin, &receiver, 3, &first_spec),
            &first_source
        ),
        "the invalid project's last good snapshot stays available"
    );
    assert!(
        !cites(
            &references(&mut child, &mut stdin, &receiver, 4, &second_spec),
            &second_source
        ),
        "a neighboring healthy project still refreshes"
    );

    stop_server(&mut child, &mut stdin, &receiver, 5);
    drop(stdin);
    wait_for_exit(&mut child);
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn a_scanned_sibling_does_not_give_another_project_ownership() {
    let base = std::env::temp_dir().join(format!("grund-lsp-exact-owner-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    let first = base.join("first");
    let second = base.join("nested/second");
    let shared = base.join("shared");
    write(
        &first.join("grund.toml"),
        "grund_config_version = 1\n[scan]\ninclude = [\"docs\", \"../shared\"]\n\
         extensions = [\"md\"]\n",
    );
    write(
        &first.join("docs/FS-001-example.md"),
        "# FS-001-example: Example\n\nLead.\n",
    );
    let user = shared.join("user.md");
    write(&user, "See §FS-001-example.\n");
    write(
        &second.join("grund.toml"),
        "grund_config_version = 1\n[scan]\ninclude = [\"../../shared\"]\n\
         extensions = [\"java\"]\n",
    );
    write(&shared.join("Noise.java"), "final class Noise {}\n");

    let (mut child, mut stdin, receiver) =
        start_server_with_workspace_folders(&base, &[&first, &second]);
    let hover = hover_result(&mut stdin, &receiver, &mut child, 2, &file_uri(&user), 0, 6);
    assert!(
        hover["contents"]["value"]
            .as_str()
            .is_some_and(|body| body.contains("FS-001-example")),
        "only the project that actually scanned the document may own it: {hover}"
    );

    stop_server(&mut child, &mut stdin, &receiver, 3);
    drop(stdin);
    wait_for_exit(&mut child);
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn two_external_scan_claims_are_rejected_as_ambiguous() {
    let base =
        std::env::temp_dir().join(format!("grund-lsp-ambiguous-owner-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    let first = base.join("first");
    let second = base.join("nested/second");
    let shared = base.join("shared");
    write(
        &first.join("grund.toml"),
        "grund_config_version = 1\n[scan]\ninclude = [\"docs\", \"../shared\"]\n\
         extensions = [\"md\"]\n",
    );
    write(
        &first.join("docs/FS-001-example.md"),
        "# FS-001-example: First\n\nFirst project body.\n",
    );
    write(
        &second.join("grund.toml"),
        "grund_config_version = 1\n[scan]\ninclude = [\"docs\", \"../../shared\"]\n\
         extensions = [\"md\"]\n",
    );
    write(
        &second.join("docs/FS-001-example.md"),
        "# FS-001-example: Second\n\nSecond project body.\n",
    );
    let user = shared.join("FS-002-user.md");
    write(&user, "# FS-002-user: User\n\nSee §FS-001-example.\n");

    let (mut child, mut stdin, receiver) =
        start_server_with_workspace_folders(&base, &[&first, &second]);
    let hover = hover_result(&mut stdin, &receiver, &mut child, 2, &file_uri(&user), 2, 6);
    assert!(
        hover.is_null(),
        "an external file scanned by two independent namespaces has no arbitrary owner: {hover}"
    );

    stop_server(&mut child, &mut stdin, &receiver, 3);
    drop(stdin);
    wait_for_exit(&mut child);
    let _ = fs::remove_dir_all(&base);
}

#[test]
#[cfg(unix)]
fn an_external_unreadable_file_still_publishes_its_scan_error() {
    let base = std::env::temp_dir().join(format!("grund-lsp-external-io-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    let root = base.join("repo");
    let shared = base.join("shared");
    write(
        &root.join("grund.toml"),
        "grund_config_version = 1\n[scan]\ninclude = [\"../shared\"]\nextensions = [\"md\"]\n",
    );
    fs::create_dir_all(&shared).expect("create shared include");
    symlink("missing.md", shared.join("broken.md")).expect("create broken symlink");

    let (mut child, mut stdin, receiver) = start_server_with_workspace_folders(&root, &[&root]);
    let diagnostics = recv_diagnostics(&receiver, &mut child, "broken.md");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic["code"].as_str() == Some("io")),
        "the external scan error must not be discarded for lacking a scanned-file owner: {diagnostics:?}"
    );

    stop_server(&mut child, &mut stdin, &receiver, 2);
    drop(stdin);
    wait_for_exit(&mut child);
    let _ = fs::remove_dir_all(&base);
}
