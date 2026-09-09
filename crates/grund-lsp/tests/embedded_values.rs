//! Embedded value roots keep ordinary section identity in editor ranges,
//! navigation, raw previews, and CLI-equal diagnostics (§FS-lsp.1.2,
//! §FS-lsp.1.3, §FS-lsp.4, §FS-values.7).

mod support;

use serde_json::json;
use std::fs;
use support::*;

#[test]
fn embedded_value_ranges_navigation_and_diagnostics_match_the_cli() {
    let root = test_root("embedded-values");
    fs::write(
        root.join("grund.toml"),
        "grund_config_version = 1\n\n\
         [reference]\nstrict = true\n\n\
         [[kinds]]\nkind = \"FS\"\nfolder = \"docs\"\nindex = false\n\n\
         [scan]\ninclude = [\"docs\"]\n",
    )
    .expect("write config");
    let document = root.join("docs/pricing.md");
    let root_heading = "## 1. Price <!-- grund:value -->";
    fs::write(
        &document,
        format!(
            "# FS-001-pricing: Pricing\n\n\
             {root_heading}\n\
             ### 1.1. 1200\n\
             ## 2. Use\n\
             Bound: `999` (§FS-001-pricing.1.1)\n\
             See §FS-001-pricing.1.\n"
        ),
    )
    .expect("write document");

    let cli = grund_core::check(&root).expect("CLI engine report");
    let cli_mismatch = cli
        .errors
        .iter()
        .find(|error| error.code == "value-mismatch")
        .expect("CLI mismatch");

    let (mut child, mut stdin, receiver) = start_server_with_capabilities(
        &root,
        json!({ "textDocument": { "definition": { "linkSupport": true } } }),
    );
    let diagnostics = recv_diagnostics(&receiver, &mut child, "pricing.md");
    let mismatch = diagnostics
        .iter()
        .find(|diagnostic| diagnostic["code"].as_str() == Some("value-mismatch"))
        .expect("LSP mismatch");
    assert_eq!(
        mismatch["message"].as_str(),
        Some(cli_mismatch.message.as_str())
    );
    assert_eq!(
        mismatch["range"]["start"]["line"].as_u64(),
        cli_mismatch.line.map(|line| line as u64 - 1)
    );

    let uri = file_uri(&document);
    let title = hover_result(&mut stdin, &receiver, &mut child, 2, &uri, 2, 6);
    assert_eq!(title["range"]["start"]["character"].as_u64(), Some(3));
    assert_eq!(
        title["range"]["end"]["character"].as_u64(),
        Some("## 1. Price".len() as u64)
    );
    assert!(
        !title["contents"]["value"]
            .as_str()
            .unwrap_or_default()
            .contains("grund:value"),
        "the semantic title range and hover omit marker bytes: {title:?}"
    );

    let preview = hover_result(&mut stdin, &receiver, &mut child, 3, &uri, 6, 8);
    assert!(
        preview["contents"]["value"]
            .as_str()
            .unwrap_or_default()
            .contains("<!-- grund:value -->"),
        "citation preview keeps the raw section slice: {preview:?}"
    );

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "textDocument/definition",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 5, "character": 20 }
            }
        }),
    );
    let definition = recv_response_or_panic(&receiver, &mut child, 4);
    let links = definition["result"].as_array().expect("definition links");
    assert!(
        links.iter().any(|link| {
            link["targetUri"].as_str() == Some(file_uri(&document).as_str())
                && link["targetSelectionRange"]["start"]["line"].as_u64() == Some(3)
                && link["targetSelectionRange"]["start"]["character"].as_u64() == Some(4)
        }),
        "binding navigation must land on the existing component section: {links:?}"
    );

    send_message(
        &mut stdin,
        json!({ "jsonrpc": "2.0", "id": 5, "method": "shutdown", "params": null }),
    );
    recv_response_or_panic(&receiver, &mut child, 5);
    send_message(
        &mut stdin,
        json!({ "jsonrpc": "2.0", "method": "exit", "params": null }),
    );
    drop(stdin);
    wait_for_exit(&mut child);
    let _ = fs::remove_dir_all(root);
}
