//! Workspace-folder discovery and lifecycle cases driven against a real server
//! process (§FS-lsp.2.2, §AR-lsp.2).

mod support;

use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin};
use std::sync::mpsc;
use support::*;

fn write_project(root: &Path, citation_name: &str) -> (PathBuf, PathBuf) {
    fs::create_dir_all(root.join(".agents")).expect("create config dir");
    fs::create_dir_all(root.join("docs")).expect("create docs dir");
    fs::create_dir_all(root.join("src")).expect("create source dir");
    fs::write(
        root.join(".agents/grund.toml"),
        "grund_config_version = 1\n[scan]\ninclude = [\"docs\", \"src\"]\n\
         extensions = [\"md\", \"java\"]\n",
    )
    .expect("write config");
    let spec = root.join("docs/FS-001-example.md");
    let source = root.join(format!("src/{citation_name}.java"));
    fs::write(&spec, "# FS-001-example: Example\n\nLead.\n").expect("write spec");
    fs::write(
        &source,
        "/// Implements §FS-001-example.\nfinal class Example {}\n",
    )
    .expect("write source");
    (spec, source)
}

fn references(
    child: &mut Child,
    stdin: &mut ChildStdin,
    receiver: &mpsc::Receiver<Value>,
    id: i64,
    spec: &Path,
) -> Value {
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
    recv_response_or_panic(receiver, child, id)["result"].clone()
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
fn nested_editor_folder_scans_the_discovered_project_root() {
    // IntelliJ commonly supplies a module/project base below the checkout root.
    // That folder discovers the enclosing config; it must not hide sibling code
    // included by that config (§FS-lsp.2.2).
    let root = test_root("nested-workspace-folder");
    let (spec, source) = write_project(&root, "NestedUse");
    let nested = root.join("ee/vm-enterprise");
    fs::create_dir_all(&nested).expect("create nested editor folder");

    let (mut child, mut stdin, receiver) = start_server_with_workspace_folders(&nested, &[&nested]);
    let result = references(&mut child, &mut stdin, &receiver, 2, &spec);
    let locations = result.as_array().expect("references result");
    assert!(
        locations
            .iter()
            .any(|location| location["uri"].as_str() == Some(file_uri(&source).as_str())),
        "nested editor folder should retain code citations from the enclosing project: {locations:?}"
    );

    stop_server(&mut child, &mut stdin, &receiver, 3);
    drop(stdin);
    wait_for_exit(&mut child);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn every_initial_workspace_folder_gets_an_independent_snapshot() {
    let root = test_root("multiple-workspace-folders");
    let first_root = root.join("first");
    let second_root = root.join("second");
    let (first_spec, first_source) = write_project(&first_root, "FirstUse");
    let (second_spec, second_source) = write_project(&second_root, "SecondUse");

    let (mut child, mut stdin, receiver) =
        start_server_with_workspace_folders(&root, &[&first_root, &second_root]);
    let first = references(&mut child, &mut stdin, &receiver, 2, &first_spec);
    let first = first.as_array().expect("first references");
    assert!(
        first
            .iter()
            .any(|location| { location["uri"].as_str() == Some(file_uri(&first_source).as_str()) })
    );
    assert!(
        !first.iter().any(|location| {
            location["uri"].as_str() == Some(file_uri(&second_source).as_str())
        })
    );

    let second = references(&mut child, &mut stdin, &receiver, 3, &second_spec);
    let second = second.as_array().expect("second references");
    assert!(
        second.iter().any(|location| {
            location["uri"].as_str() == Some(file_uri(&second_source).as_str())
        })
    );

    stop_server(&mut child, &mut stdin, &receiver, 4);
    drop(stdin);
    wait_for_exit(&mut child);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn workspace_folder_changes_rebuild_and_remove_project_snapshots() {
    let root = test_root("changed-workspace-folders");
    let first_root = root.join("first");
    let second_root = root.join("second");
    let (_first_spec, _first_source) = write_project(&first_root, "FirstUse");
    let (second_spec, second_source) = write_project(&second_root, "SecondUse");

    let (mut child, mut stdin, receiver) =
        start_server_with_workspace_folders(&root, &[&first_root]);
    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "workspace/didChangeWorkspaceFolders",
            "params": {
                "event": {
                    "added": [{ "uri": file_uri(&second_root), "name": "second" }],
                    "removed": []
                }
            }
        }),
    );
    let added = references(&mut child, &mut stdin, &receiver, 2, &second_spec);
    assert!(
        added
            .as_array()
            .expect("added references")
            .iter()
            .any(|location| {
                location["uri"].as_str() == Some(file_uri(&second_source).as_str())
            })
    );

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "workspace/didChangeWorkspaceFolders",
            "params": {
                "event": {
                    "added": [],
                    "removed": [{ "uri": file_uri(&second_root), "name": "second" }]
                }
            }
        }),
    );
    assert!(
        references(&mut child, &mut stdin, &receiver, 3, &second_spec).is_null(),
        "a removed independent project must stop answering requests"
    );

    stop_server(&mut child, &mut stdin, &receiver, 4);
    drop(stdin);
    wait_for_exit(&mut child);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn removing_one_of_two_folders_keeps_the_shared_project_snapshot() {
    let root = test_root("shared-project-workspace-folders");
    let (spec, source) = write_project(&root, "SharedUse");
    let docs = root.join("docs");
    let src = root.join("src");
    let (mut child, mut stdin, receiver) =
        start_server_with_workspace_folders(&root, &[&docs, &src]);

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "workspace/didChangeWorkspaceFolders",
            "params": {
                "event": {
                    "added": [],
                    "removed": [{ "uri": file_uri(&docs), "name": "docs" }]
                }
            }
        }),
    );
    let remaining = references(&mut child, &mut stdin, &receiver, 2, &spec);
    assert!(
        remaining
            .as_array()
            .expect("remaining references")
            .iter()
            .any(|location| location["uri"].as_str() == Some(file_uri(&source).as_str())),
        "the remaining folder anchor should keep its enclosing project active"
    );

    stop_server(&mut child, &mut stdin, &receiver, 3);
    drop(stdin);
    wait_for_exit(&mut child);
    let _ = fs::remove_dir_all(root);
}
