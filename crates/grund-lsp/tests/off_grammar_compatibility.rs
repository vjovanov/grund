//! Real-server parity for catalog-backed off-grammar declarations: the LSP
//! transports the same warning as `check` and navigates the same declaration
//! and citations as CLI readers (§FS-lsp.4, §FS-config.3.2).

mod support;

use serde_json::json;
use std::fs;
use support::*;

#[test]
fn off_grammar_declaration_has_cli_parity_across_lsp_surfaces() {
    let root = test_root("off-grammar-compatibility");
    fs::write(
        root.join("grund.toml"),
        "grund_config_version = 1\n\n\
         [reference]\nstrict = true\n\n\
         [id]\nformat = \"{kind}-{number}-{slug}\"\n\n\
         [[kinds]]\nkind = \"FS\"\nfolder = \"docs\"\nindex = false\n\n\
         [scan]\ninclude = [\"docs\"]\nextensions = [\"md\"]\n",
    )
    .expect("write config");
    let spec = root.join("docs/FS-security-providers.md");
    fs::create_dir_all(spec.parent().unwrap()).expect("create docs");
    fs::write(
        &spec,
        "# FS-security-providers: Security providers\n\n\
         Lead.\n\n\
         ## 1. Contract\n\n\
         Stable contract.\n\n\
         Uses \u{a7}FS-security-providers.1.\n",
    )
    .expect("write spec");

    let (mut child, mut stdin, receiver) = start_server(&root);
    let diagnostics = recv_diagnostics(&receiver, &mut child, "FS-security-providers.md");
    let mut failures = Vec::new();
    let near_miss = diagnostics.iter().find(|diagnostic| {
        diagnostic["message"]
            .as_str()
            .is_some_and(|message| message.contains("FS-security-providers"))
    });
    match near_miss {
        Some(diagnostic) => {
            let message = diagnostic["message"].as_str().unwrap_or_default();
            if !message.contains("resolves for compatibility")
                || !message.contains("error in grund 0.15.0")
            {
                failures.push(format!("LSP diagnostic has stale message: {diagnostic:?}"));
            }
            if diagnostic["severity"].as_i64() != Some(2)
                || diagnostic["range"]["start"]["line"].as_i64() != Some(0)
            {
                failures.push(format!(
                    "LSP warning location/severity drifted: {diagnostic:?}"
                ));
            }
        }
        None => failures.push(format!(
            "LSP published no declaration-near-miss diagnostic: {diagnostics:?}"
        )),
    }

    let hover = hover_result(
        &mut stdin,
        &receiver,
        &mut child,
        2,
        &file_uri(&spec),
        8,
        10,
    );
    let hover_text = hover["contents"]["value"].as_str().unwrap_or_default();
    if !hover_text.contains("1. Contract") || !hover_text.contains("Stable contract") {
        failures.push(format!(
            "off-grammar citation hover did not resolve: {hover:?}"
        ));
    }

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "textDocument/definition",
            "params": {
                "textDocument": { "uri": file_uri(&spec) },
                "position": { "line": 8, "character": 10 }
            }
        }),
    );
    let definition = recv_response_or_panic(&receiver, &mut child, 3);
    let definition_hits_section = definition["result"].as_array().is_some_and(|links| {
        links
            .iter()
            .any(|link| link["targetSelectionRange"]["start"]["line"].as_i64() == Some(4))
    });
    if !definition_hits_section {
        failures.push(format!(
            "off-grammar section definition did not reach line 5: {definition:?}"
        ));
    }

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "textDocument/references",
            "params": {
                "textDocument": { "uri": file_uri(&spec) },
                "position": { "line": 4, "character": 5 },
                "context": { "includeDeclaration": true }
            }
        }),
    );
    let references = recv_response_or_panic(&receiver, &mut child, 4);
    let reference_lines = references["result"]
        .as_array()
        .map(|locations| {
            locations
                .iter()
                .filter_map(|location| location["range"]["start"]["line"].as_i64())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if !reference_lines.contains(&4) || !reference_lines.contains(&8) {
        failures.push(format!(
            "off-grammar section references lost heading or citation: {references:?}"
        ));
    }

    send_message(
        &mut stdin,
        json!({"jsonrpc": "2.0", "id": 5, "method": "shutdown", "params": null}),
    );
    recv_response_or_panic(&receiver, &mut child, 5);
    send_message(
        &mut stdin,
        json!({"jsonrpc": "2.0", "method": "exit", "params": null}),
    );
    drop(stdin);
    wait_for_exit(&mut child);

    assert!(
        failures.is_empty(),
        "off-grammar LSP compatibility mismatches:\n\n{}",
        failures.join("\n\n")
    );
}
