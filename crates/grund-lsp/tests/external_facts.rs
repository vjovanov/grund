//! [§FS-lsp.1.1](../../../docs/functional-spec/FS-lsp.md#11-diagnostics) / [§REQ-runs-offline](../../../docs/requirements/REQ-runs-offline.md#req-runs-offline-verification-never-depends-on-an-external-service): missing-snapshot diagnostics cross the existing LSP transport without executing the fetcher.
#![cfg(unix)]

mod support;

use serde_json::json;
use std::fs;
use support::*;

#[test]
fn external_facts_lsp_publishes_the_warning_without_fetch_or_code_action() {
    use std::os::unix::fs::PermissionsExt;

    let root = test_root("external-facts-offline");
    fs::create_dir_all(root.join("scripts")).unwrap();
    fs::write(
        root.join("grund.toml"),
        concat!(
            "grund_config_version = 1\n[reference]\nstrict = true\n",
            "[id]\nformat = \"{kind}-{slug}\"\nslug_pattern = \"[a-z][a-z-]*\"\n",
            "[[kinds]]\nkind = \"TICKET\"\nfile = \"docs/tickets.md\"\n",
            "format = \"{kind}-{number}\"\nresolve = \"should\"\n",
            "fetch = \"scripts/fetch-ticket\"\n",
            "[scan]\ninclude = [\"docs\"]\nrespect_gitignore = false\n"
        ),
    )
    .unwrap();
    fs::write(
        root.join("docs/guide.md"),
        "# Guide\n\nSee \u{a7}TICKET-1234.\n",
    )
    .unwrap();
    let fetcher = root.join("scripts/fetch-ticket");
    fs::write(&fetcher, "#!/bin/sh\ntouch fetcher-ran\nexit 99\n").unwrap();
    let mut permissions = fs::metadata(&fetcher).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&fetcher, permissions).unwrap();

    let (mut child, mut stdin, receiver) = start_server(&root);
    let diagnostics = recv_diagnostics(&receiver, &mut child, "guide.md");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0]["severity"], 2);
    assert_eq!(diagnostics[0]["code"], "missing-snapshot");
    assert_eq!(
        diagnostics[0]["message"],
        "no snapshot for TICKET-1234 in docs/tickets.md — run grund fetch TICKET-1234"
    );
    assert!(!root.join("fetcher-ran").exists());

    send_message(
        &mut stdin,
        json!({"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}),
    );
    recv_response_or_panic(&receiver, &mut child, 2);
    send_message(
        &mut stdin,
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    );
    drop(stdin);
    wait_for_exit(&mut child);
}
