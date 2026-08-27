//! `textDocument/hover` cases driven against a real server process: the usage
//! counts a declaration-side title carries, the citation preview they must
//! leave alone, and the citation that resolves to nothing and defers to its
//! diagnostic (§FS-lsp.1.2).

mod support;

use serde_json::{Value, json};
use std::fs;
use support::*;

/// A tree with one declaration cited twice from two files, a section cited
/// once, a stub-and-inline-source pair cited once, and an uncited declaration —
/// the four count shapes §FS-lsp.1.2 words differently, in one workspace.
fn hover_fixture(name: &str) -> std::path::PathBuf {
    let root = test_root(name);
    fs::write(
        root.join(".agents/grund.toml"),
        "grund_config_version = 1\n[scan]\ninclude = [\"docs\", \"src\"]\n\
         extensions = [\"md\", \"rs\"]\n",
    )
    .expect("write config");
    fs::create_dir_all(root.join("docs/functional-spec")).expect("create specs");
    fs::create_dir_all(root.join("docs/architecture")).expect("create architecture");
    fs::create_dir_all(root.join("src")).expect("create source");
    fs::write(
        root.join("docs/functional-spec/FS-001-alpha.md"),
        "# FS-001-alpha: Alpha\n\nLead.\n\n## 1. Detail\nMore.\n",
    )
    .expect("write spec");
    fs::write(
        root.join("docs/functional-spec/FS-002-beta.md"),
        "# FS-002-beta: Beta\n\nLead.\n",
    )
    .expect("write uncited spec");
    fs::write(
        root.join("docs/architecture/AR-001-router.md"),
        "# AR-001-router: [src/router.rs](../../src/router.rs)\n",
    )
    .expect("write stub");
    fs::write(
        root.join("src/router.rs"),
        "/// AR-001-router: Router\n/// Uses §FS-001-alpha.1.\npub fn router() {}\n",
    )
    .expect("write source");
    fs::write(
        root.join("src/caller.rs"),
        "//! §FS-001-alpha\n//! §AR-001-router\npub fn caller() {}\n",
    )
    .expect("write caller");
    root
}

fn hover_body(result: &Value) -> String {
    result["contents"]["value"]
        .as_str()
        .unwrap_or_else(|| panic!("hover body in {result:?}"))
        .to_string()
}

/// §FS-lsp.1.2: every declaration-side title — Markdown heading, numbered
/// section heading, and inline-spec stub title — hovers with the sites and
/// files that cite it, and keeps the whole-title hover range.
#[test]
fn title_hover_reports_usage_counts_for_every_title_kind() {
    let root = hover_fixture("hover-usage-counts");
    let spec = root.join("docs/functional-spec/FS-001-alpha.md");
    let beta = root.join("docs/functional-spec/FS-002-beta.md");
    let stub = root.join("docs/architecture/AR-001-router.md");
    let source = root.join("src/router.rs");
    let (mut child, mut stdin, receiver) = start_server(&root);

    // Two sites in two files: `§FS-001-alpha.1` in src/router.rs and
    // `§FS-001-alpha` in src/caller.rs.
    let declaration = hover_result(&mut stdin, &receiver, &mut child, 2, &file_uri(&spec), 0, 5);
    assert_eq!(
        hover_body(&declaration),
        "`FS-001-alpha: Alpha` — cited at 2 sites across 2 files"
    );
    assert!(
        declaration["range"]["start"]["character"].as_i64() == Some(2)
            && declaration["range"]["end"]["character"].as_i64()
                == Some("# FS-001-alpha: Alpha".len() as i64),
        "the whole-title hover range survives the usage counts (§FS-lsp.1.2): {declaration:?}"
    );

    // The section-scoped set (§FS-lsp.1.3.1), and the singular wording.
    let section = hover_result(&mut stdin, &receiver, &mut child, 3, &file_uri(&spec), 4, 4);
    assert_eq!(
        hover_body(&section),
        "`1. Detail` — cited at 1 site across 1 file"
    );
    assert!(
        section["range"]["start"]["line"].as_i64() == Some(4)
            && section["range"]["start"]["character"].as_i64() == Some(3)
            && section["range"]["end"]["character"].as_i64() == Some("## 1. Detail".len() as i64),
        "a section heading hovers with its whole title range (§FS-lsp.1.2): {section:?}"
    );

    // A stub title is a whole-ID title: it counts the citations of the
    // declaration it points at, in the file that cites it.
    let stub_hover = hover_result(&mut stdin, &receiver, &mut child, 4, &file_uri(&stub), 0, 5);
    let stub_heading = "# AR-001-router: [src/router.rs](../../src/router.rs)";
    assert_eq!(
        hover_body(&stub_hover),
        "`AR-001-router: [src/router.rs](../../src/router.rs)` — cited at 1 site across 1 file"
    );
    assert!(
        stub_hover["range"]["start"]["character"].as_i64() == Some(2)
            && stub_hover["range"]["end"]["character"].as_i64() == Some(stub_heading.len() as i64),
        "a stub title hovers with its whole title range (§FS-lsp.1.2): {stub_hover:?}"
    );

    // And the inline source declaration the stub points at answers the same.
    let inline = hover_result(
        &mut stdin,
        &receiver,
        &mut child,
        5,
        &file_uri(&source),
        0,
        8,
    );
    assert_eq!(
        hover_body(&inline),
        "`AR-001-router: Router` — cited at 1 site across 1 file"
    );

    // Zero replaces the whole clause rather than suppressing the hover.
    let uncited = hover_result(&mut stdin, &receiver, &mut child, 6, &file_uri(&beta), 0, 5);
    assert_eq!(hover_body(&uncited), "`FS-002-beta: Beta` — not cited");

    // §FS-lsp.4: the same request against an unchanged tree answers with the
    // same bytes.
    let repeated = hover_result(&mut stdin, &receiver, &mut child, 7, &file_uri(&spec), 0, 5);
    assert_eq!(hover_body(&repeated), hover_body(&declaration));

    send_message(
        &mut stdin,
        json!({"jsonrpc": "2.0", "id": 8, "method": "shutdown", "params": null}),
    );
    recv_response_or_panic(&receiver, &mut child, 8);
    send_message(
        &mut stdin,
        json!({"jsonrpc": "2.0", "method": "exit", "params": null}),
    );
    drop(stdin);

    wait_for_exit(&mut child);
    let _ = fs::remove_dir_all(root);
}

/// §FS-lsp.1.2: hovering a *citation* is unchanged — it still previews the
/// declaration body `grund <ID> --toc` prints, with no usage clause. The counts
/// belong to the declaration side, where there is no body to show.
#[test]
fn citation_hover_still_previews_the_declaration_body() {
    let root = hover_fixture("hover-citation-preview");
    let source = root.join("src/router.rs");
    let (mut child, mut stdin, receiver) = start_server(&root);

    let citation = hover_result(
        &mut stdin,
        &receiver,
        &mut child,
        2,
        &file_uri(&source),
        1,
        14,
    );
    let body = hover_body(&citation);
    assert!(
        body.contains("Detail"),
        "citation hover keeps the `--toc` body of the cited section: {body:?}"
    );
    assert!(
        !body.contains("cited at") && !body.contains("not cited"),
        "usage counts belong to declaration-side titles only (§FS-lsp.1.2): {body:?}"
    );

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

/// A citation that cannot resolve has no hover body. Its diagnostic already
/// carries the nearest-ID hint, so returning it from hover too would double
/// the text in editors that render diagnostics in the hover popup; hover
/// returns nothing and the diagnostic stands alone (§FS-lsp.1.2).
#[test]
fn hover_on_dangling_citation_defers_to_diagnostic() {
    let root = test_root("hover-dangling-defers");
    fs::write(
        root.join(".agents/grund.toml"),
        "grund_config_version = 1\n[scan]\ninclude = [\"docs\"]\nextensions = [\"md\"]\n\
         [id]\nformat = \"{kind}-{slug}\"\nslug_pattern = \"[a-z][a-z0-9-]*\"\n",
    )
    .expect("write config");
    fs::create_dir_all(root.join("docs/functional-spec")).expect("create specs");
    fs::write(
        root.join("docs/functional-spec/FS-check.md"),
        "# FS-check: Check\n\nLead.\n",
    )
    .expect("write spec");
    let marker = '§';
    let line = format!("Uses {marker}FS-chek.");
    let uses = root.join("docs/uses.md");
    fs::write(&uses, format!("{line}\n")).expect("write uses");

    let (mut child, mut stdin, receiver) = start_server(&root);

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": file_uri(&uses) },
                "position": { "line": 0, "character": 8 }
            }
        }),
    );
    let hover = recv_response_or_panic(&receiver, &mut child, 2);
    assert!(
        hover["result"].is_null(),
        "hover on a dangling citation returns nothing so the diagnostic is not \
         echoed a second time in the hover popup (§FS-lsp.1.2): {hover:?}"
    );

    send_message(
        &mut stdin,
        json!({ "jsonrpc": "2.0", "id": 3, "method": "shutdown", "params": null }),
    );
    recv_response_or_panic(&receiver, &mut child, 3);
    send_message(&mut stdin, json!({ "jsonrpc": "2.0", "method": "exit" }));
    drop(stdin);

    wait_for_exit(&mut child);
    let _ = fs::remove_dir_all(root);
}
