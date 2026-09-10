//! Real-server regression for rejected section headings: diagnostics retain the
//! complete title range without making the heading navigable (§FS-check.3.23,
//! §FS-lsp.1.1).

mod support;

use serde_json::json;
use std::fs;
use support::*;

#[test]
fn rejected_section_has_an_exact_diagnostic_range_but_no_definition() {
    let root = test_root("section-outside-declaration");
    fs::write(
        root.join("grund.toml"),
        "grund_config_version = 1\n\n[reference]\nstrict = false\n\n\
         [[kinds]]\nkind = \"FS\"\nfolder = \"docs/functional-spec\"\nindex = false\n",
    )
    .expect("write config");
    fs::create_dir_all(root.join("docs/functional-spec")).expect("create spec dir");
    let spec = root.join("docs/functional-spec/FS-001-body-scope.md");
    fs::write(
        &spec,
        "## FS-001-body-scope: Body scope\n\n\
         The declaration body ends at the next heading of the same level.\n\n\
         ## Plain chapter\n\n\
         This plain chapter is outside the declaration body.\n\n\
         ### 1. Outside body\n\n\
         This numbered heading follows the body-closing heading.\n",
    )
    .expect("write ticket fixture");
    let (mut child, mut stdin, receiver) = start_server(&root);

    let diagnostics = recv_diagnostics(&receiver, &mut child, "FS-001-body-scope.md");
    let rejected = diagnostics
        .iter()
        .find(|diagnostic| diagnostic["code"].as_str() == Some("section-outside-declaration"))
        .unwrap_or_else(|| panic!("outside-section diagnostic: {diagnostics:?}"));
    assert_eq!(
        rejected["range"],
        json!({
            "start": { "line": 8, "character": 4 },
            "end": { "line": 8, "character": 19 }
        })
    );

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/definition",
            "params": {
                "textDocument": { "uri": file_uri(&spec) },
                "position": { "line": 8, "character": 6 }
            }
        }),
    );
    let definition = recv_response_or_panic(&receiver, &mut child, 2);
    assert_eq!(definition["result"], json!(null));

    send_message(
        &mut stdin,
        json!({"jsonrpc": "2.0", "id": 3, "method": "shutdown", "params": null}),
    );
    recv_response_or_panic(&receiver, &mut child, 3);
    send_message(
        &mut stdin,
        json!({"jsonrpc": "2.0", "method": "exit", "params": null}),
    );
    drop(stdin);
    wait_for_exit(&mut child);
    let _ = fs::remove_dir_all(root);
}
