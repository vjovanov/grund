//! Real-server coverage for the named-section records shared with the CLI:
//! diagnostics, hover, navigation, references, and highlights (§FS-lsp.1).

mod support;

use serde_json::json;
use std::fs;
use support::*;

#[test]
fn named_sections_have_cli_parity_across_editor_surfaces() {
    let root = test_root("named-sections");
    fs::write(
        root.join("grund.toml"),
        "grund_config_version = 1\n\n[reference]\nstrict = true\n\n\
         [id]\nformat = \"{kind}-{slug}\"\nnamed_sections = true\n\n\
         [scan]\ninclude = [\"docs\"]\nextensions = [\"md\"]\n",
    )
    .expect("write named config");
    let spec = root.join("docs/FS-doc.md");
    fs::write(
        &spec,
        "# FS-doc: Document\n\nLead.\n\n\
         ## goals: Scope\n\nGoals lead.\n\n\
         ### goals.performance: Performance\n\nNested target.\n\n\
         Uses \u{a7}FS-doc.goals.performance twice: \u{a7}FS-doc.goals.performance.\n\
         Missing \u{a7}FS-doc.absent.\n",
    )
    .expect("write named spec");

    let (mut child, mut stdin, receiver) = start_server(&root);

    // §FS-lsp.1.1: the core's named missing-section finding is transported at
    // the citation line with the same message and whole-token range.
    let diagnostics = recv_diagnostics(&receiver, &mut child, "FS-doc.md");
    let missing = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic["message"]
                .as_str()
                .is_some_and(|message| message.contains("FS-doc.absent"))
        })
        .unwrap_or_else(|| panic!("named missing-section diagnostic: {diagnostics:?}"));
    assert!(missing["message"].as_str().unwrap().contains("<§>"));
    assert_eq!(missing["range"]["start"]["line"].as_i64(), Some(13));

    // §FS-lsp.1.2: citation hover is the named section's `show --toc` body.
    let hover = hover_result(
        &mut stdin,
        &receiver,
        &mut child,
        2,
        &file_uri(&spec),
        12,
        12,
    );
    let hover_body = hover["contents"]["value"]
        .as_str()
        .unwrap_or_else(|| panic!("named hover body: {hover:?}"));
    assert!(hover_body.contains("goals.performance: Performance"));
    assert!(hover_body.contains("Nested target"));

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "textDocument/definition",
            "params": {
                "textDocument": { "uri": file_uri(&spec) },
                "position": { "line": 12, "character": 12 }
            }
        }),
    );
    let definition = recv_response_or_panic(&receiver, &mut child, 3);
    let links = definition["result"]
        .as_array()
        .unwrap_or_else(|| panic!("named definition links: {definition:?}"));
    assert!(links.iter().any(|link| {
        link["targetSelectionRange"]["start"]["line"].as_i64() == Some(8)
            && link["originSelectionRange"]["start"]["character"].as_i64() == Some(5)
    }));

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "textDocument/references",
            "params": {
                "textDocument": { "uri": file_uri(&spec) },
                "position": { "line": 8, "character": 8 },
                "context": { "includeDeclaration": true }
            }
        }),
    );
    let references = recv_response_or_panic(&receiver, &mut child, 4);
    let references = references["result"]
        .as_array()
        .unwrap_or_else(|| panic!("named references: {references:?}"));
    assert_eq!(
        references
            .iter()
            .filter(|location| location["range"]["start"]["line"].as_i64() == Some(12))
            .count(),
        2
    );

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "textDocument/documentHighlight",
            "params": {
                "textDocument": { "uri": file_uri(&spec) },
                "position": { "line": 12, "character": 12 }
            }
        }),
    );
    let highlights = recv_response_or_panic(&receiver, &mut child, 5);
    let highlights = highlights["result"]
        .as_array()
        .unwrap_or_else(|| panic!("named highlights: {highlights:?}"));
    assert_eq!(
        highlights.len(),
        3,
        "heading plus two citations: {highlights:?}"
    );
    assert!(highlights.iter().all(|highlight| {
        let range = &highlight["range"];
        range["end"]["character"].as_i64() > range["start"]["character"].as_i64()
    }));

    send_message(
        &mut stdin,
        json!({"jsonrpc": "2.0", "id": 6, "method": "shutdown", "params": null}),
    );
    recv_response_or_panic(&receiver, &mut child, 6);
    send_message(
        &mut stdin,
        json!({"jsonrpc": "2.0", "method": "exit", "params": null}),
    );
    drop(stdin);
    wait_for_exit(&mut child);
    let _ = fs::remove_dir_all(root);
}
