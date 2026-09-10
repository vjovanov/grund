//! Real-server parity for the body-owned Markdown warning (§FS-check.4.14,
//! §FS-lsp.1.1).

mod support;

use serde_json::json;
use std::fs;
use support::*;

#[test]
fn unmarked_heading_is_the_same_warning_over_lsp() {
    let root = test_root("unmarked-heading");
    fs::write(
        root.join("grund.toml"),
        "grund_config_version = 1\n\n[reference]\nstrict = true\n\n\
         [id]\nformat = \"{kind}-{slug}\"\n\n\
         [[kinds]]\nkind = \"FS\"\nfolder = \"docs\"\nindex = false\n",
    )
    .expect("write config");
    let spec = root.join("docs/FS-policy.md");
    fs::write(
        &spec,
        "# FS-policy: Policy\n\nThe declaration cites \u{a7}FS-policy.\n\n## Missing coordinate\n",
    )
    .expect("write spec");
    let (mut child, mut stdin, receiver) = start_server(&root);

    let diagnostics = recv_diagnostics(&receiver, &mut child, "FS-policy.md");
    let warning = diagnostics
        .iter()
        .find(|diagnostic| diagnostic["code"].as_str() == Some("unmarked-heading"))
        .unwrap_or_else(|| panic!("unmarked-heading diagnostic: {diagnostics:?}"));
    assert_eq!(warning["severity"], json!(2));
    assert_eq!(
        warning["message"],
        json!(
            "unmarked heading inside FS-policy; number it (## 1. Missing coordinate) as \
             FS-policy.1, declare an ID, or use a bold label; this warning becomes an error \
             in grund 0.15.0"
        )
    );
    assert_eq!(
        warning["range"],
        json!({
            "start": { "line": 4, "character": 0 },
            "end": { "line": 4, "character": 21 }
        })
    );

    send_message(
        &mut stdin,
        json!({"jsonrpc": "2.0", "id": 2, "method": "shutdown", "params": null}),
    );
    recv_response_or_panic(&receiver, &mut child, 2);
    send_message(
        &mut stdin,
        json!({"jsonrpc": "2.0", "method": "exit", "params": null}),
    );
    drop(stdin);
    wait_for_exit(&mut child);
    let _ = fs::remove_dir_all(root);
}
