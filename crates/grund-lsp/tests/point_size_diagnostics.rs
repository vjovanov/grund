//! LSP parity for the configured lead-size warning (§FS-check.4.13,
//! §FS-lsp.1.2).

mod support;

use serde_json::json;
use std::fs;
use support::*;

#[test]
fn oversized_lead_uses_the_cli_code_message_location_and_warning_severity() {
    let root = test_root("oversized-lead");
    fs::write(
        root.join("grund.toml"),
        "grund_config_version = 1\n\
         [reference]\nstrict = true\nlead_size_warning = { max = 2, unit = \"words\" }\n\
         [id]\nformat = \"{kind}-{slug}\"\n\
         [[kinds]]\nkind = \"FS\"\nfolder = \"docs\"\nindex = false\n\
         [scan]\ninclude = [\"docs\"]\nextensions = [\"md\"]\n",
    )
    .expect("write config");
    fs::write(
        root.join("docs/FS-heavy.md"),
        "# FS-heavy: Heavy\n\none two three \u{a7}FS-heavy\n",
    )
    .expect("write declaration");

    let (mut child, mut stdin, receiver) = start_server(&root);
    let diagnostics = recv_diagnostics(&receiver, &mut child, "FS-heavy.md");
    let warning = diagnostics
        .iter()
        .find(|diagnostic| diagnostic["code"] == "oversized-lead")
        .unwrap_or_else(|| panic!("oversized-lead diagnostic missing: {diagnostics:?}"));
    assert_eq!(warning["severity"], 2);
    assert_eq!(warning["range"]["start"]["line"], 0);
    assert_eq!(warning["range"]["start"]["character"], 0);
    assert_eq!(
        warning["message"],
        "FS-heavy lead is 4 words, over the configured maximum of 2; move detail into citable child sections, or promote a child section to its own ID after running grund refs FS-heavy --summary"
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
