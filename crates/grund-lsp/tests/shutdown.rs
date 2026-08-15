//! Lifecycle, navigation, and diagnostics cases driven against a real server
//! process over stdio (§FS-lsp.1, §FS-lsp.2.2).

mod support;

use serde_json::{Value, json};
use std::fs;
use std::process::{Command, Stdio};
use support::*;

#[test]
fn shutdown_exit_terminates_stdio_server() {
    // The editor owns the lifecycle and talks to grund-lsp over stdio only.
    // §FS-lsp.2.2 §AR-lsp.4
    let root = test_root("shutdown");
    let mut child = Command::new(env!("CARGO_BIN_EXE_grund-lsp"))
        .current_dir(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn grund-lsp");
    let mut stdin = child.stdin.take().expect("child stdin");
    let receiver = read_messages(child.stdout.take().expect("child stdout"));

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "rootUri": file_uri(&root),
                "capabilities": {}
            }
        }),
    );
    recv_response_or_panic(&receiver, &mut child, 1);
    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        }),
    );

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "shutdown",
            "params": null
        }),
    );
    recv_response_or_panic(&receiver, &mut child, 2);
    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "exit"
        }),
    );
    drop(stdin);

    wait_for_exit(&mut child);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn navigation_covers_source_comment_citations_and_stub_titles() {
    // Stub-title definition follows the inline source home (§FS-lsp.1.3);
    // declaration-side titles have no hover and expose usages through
    // definition/references (§FS-lsp.1.2 §FS-lsp.1.3.1), while citations expose
    // document links (§FS-lsp.1.3.2).
    let root = test_root("navigation");
    fs::write(
        root.join(".agents/grund.toml"),
        "grund_config_version = 1\n[scan]\ninclude = [\"docs\", \"src\"]\n\
         extensions = [\"md\", \"rs\"]\n",
    )
    .expect("write config");
    fs::create_dir_all(root.join("docs/functional-spec")).expect("create specs");
    fs::create_dir_all(root.join("docs/architecture")).expect("create architecture");
    fs::create_dir_all(root.join("src")).expect("create source");
    let spec = root.join("docs/functional-spec/FS-001-alpha.md");
    let stub = root.join("docs/architecture/AR-001-router.md");
    let source = root.join("src/router.rs");
    let spec_heading = "# FS-001-alpha: Alpha";
    fs::write(
        &spec,
        format!("{spec_heading}\n\nLead.\n\n## 1. Detail\nMore.\n"),
    )
    .expect("write spec");
    let stub_heading = "# AR-001-router: [src/router.rs](../../src/router.rs)";
    fs::write(&stub, format!("{stub_heading}\n")).expect("write stub");
    fs::write(
        &source,
        "/// AR-001-router: Router\n/// Uses §FS-001-alpha.1.\npub fn router() {}\n",
    )
    .expect("write source");

    let mut child = Command::new(env!("CARGO_BIN_EXE_grund-lsp"))
        .current_dir(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn grund-lsp");
    let mut stdin = child.stdin.take().expect("child stdin");
    let receiver = read_messages(child.stdout.take().expect("child stdout"));

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "rootUri": file_uri(&root),
                "capabilities": {}
            }
        }),
    );
    recv_response_or_panic(&receiver, &mut child, 1);
    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        }),
    );

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/definition",
            "params": {
                "textDocument": { "uri": file_uri(&stub) },
                "position": { "line": 0, "character": 22 }
            }
        }),
    );
    let stub_definition = recv_response_or_panic(&receiver, &mut child, 2);
    let stub_definition = stub_definition["result"]
        .as_array()
        .expect("stub definition links");
    assert!(
        stub_definition.iter().any(|link| {
            link["targetUri"].as_str() == Some(file_uri(&source).as_str())
                && link["originSelectionRange"]["start"]["character"].as_i64() == Some(2)
                && link["originSelectionRange"]["end"]["character"].as_i64()
                    == Some(stub_heading.len() as i64)
                && link["targetSelectionRange"]["start"]["line"].as_i64() == Some(0)
                && link["targetSelectionRange"]["start"]["character"].as_i64() == Some(4)
                && link["targetSelectionRange"]["end"]["character"].as_i64()
                    == Some("/// AR-001-router: Router".len() as i64)
        }),
        "stub definition should select the whole stub title and inline source title: {stub_definition:?}"
    );

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "textDocument/documentLink",
            "params": {
                "textDocument": { "uri": file_uri(&stub) }
            }
        }),
    );
    let stub_links_response = recv_response_or_panic(&receiver, &mut child, 3);
    let stub_links = stub_links_response["result"]
        .as_array()
        .expect("stub document links");
    assert!(
        stub_links.iter().any(|link| {
            link["range"]["start"]["character"].as_i64() == Some(2)
                && link["range"]["end"]["character"].as_i64() == Some(stub_heading.len() as i64)
                && link["target"]
                    .as_str()
                    .is_some_and(|target| target.contains("src/router.rs#L1"))
        }),
        "stub title should be one whole-title document link: {stub_links:?}"
    );

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "textDocument/documentLink",
            "params": {
                "textDocument": { "uri": file_uri(&spec) }
            }
        }),
    );
    let spec_links_response = recv_response_or_panic(&receiver, &mut child, 4);
    let spec_links = spec_links_response["result"]
        .as_array()
        .expect("spec document links");
    assert!(
        !spec_links.iter().any(|link| {
            link["range"]["start"]["line"].as_i64() == Some(0)
                && link["target"]
                    .as_str()
                    .is_some_and(|target| target.contains("FS-001-alpha.md"))
        }),
        "ordinary Markdown declaration title must not be a self-pointing document link, \
         so the click resolves to go-to-definition usages (§FS-lsp.1.3.2): {spec_links:?}"
    );

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "textDocument/documentLink",
            "params": {
                "textDocument": { "uri": file_uri(&source) }
            }
        }),
    );
    let links = recv_response_or_panic(&receiver, &mut child, 5);
    let links = links["result"].as_array().expect("document links");
    assert!(
        links.iter().any(|link| {
            link["range"]["start"]["line"].as_i64() == Some(1)
                && link["range"]["start"]["character"].as_i64() == Some(9)
                && link["target"]
                    .as_str()
                    .is_some_and(|target| target.contains("FS-001-alpha.md#L5"))
        }),
        "source-comment citation should be a document link: {links:?}"
    );

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": file_uri(&spec) },
                "position": { "line": 0, "character": 17 }
            }
        }),
    );
    let declaration_hover_response = recv_response_or_panic(&receiver, &mut child, 6);
    assert!(
        declaration_hover_response["result"]["range"]["start"]["character"].as_i64() == Some(2)
            && declaration_hover_response["result"]["range"]["end"]["character"].as_i64()
                == Some(spec_heading.len() as i64)
            && declaration_hover_response["result"]["contents"]["value"]
                .as_str()
                .is_some_and(|value| value.contains("FS-001-alpha")),
        "declaration-title hover should carry the whole title range for editor hover affordances (§FS-lsp.1.2): {declaration_hover_response:?}"
    );

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "textDocument/references",
            "params": {
                "textDocument": { "uri": file_uri(&source) },
                "position": { "line": 1, "character": 10 },
                "context": { "includeDeclaration": true }
            }
        }),
    );
    let references = recv_response_or_panic(&receiver, &mut child, 7);
    let references = references["result"].as_array().expect("references");
    assert!(
        references.iter().any(|location| {
            location["uri"].as_str() == Some(file_uri(&spec).as_str())
                && location["range"]["start"]["line"].as_i64() == Some(0)
        }),
        "references should include the declaration: {references:?}"
    );
    assert!(
        references.iter().any(|location| {
            location["uri"].as_str() == Some(file_uri(&source).as_str())
                && location["range"]["start"]["line"].as_i64() == Some(1)
        }),
        "references should include the source-comment citation: {references:?}"
    );

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "textDocument/references",
            "params": {
                "textDocument": { "uri": file_uri(&spec) },
                "position": { "line": 0, "character": 17 },
                "context": { "includeDeclaration": true }
            }
        }),
    );
    let markdown_references = recv_response_or_panic(&receiver, &mut child, 8);
    let markdown_references = markdown_references["result"]
        .as_array()
        .expect("markdown references");
    assert!(
        markdown_references.iter().any(|location| {
            location["uri"].as_str() == Some(file_uri(&source).as_str())
                && location["range"]["start"]["line"].as_i64() == Some(1)
        }),
        "markdown title references should include source-comment citations: {markdown_references:?}"
    );

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "textDocument/definition",
            "params": {
                "textDocument": { "uri": file_uri(&spec) },
                "position": { "line": 0, "character": 17 }
            }
        }),
    );
    let markdown_definition = recv_response_or_panic(&receiver, &mut child, 9);
    let markdown_definition = markdown_definition["result"]
        .as_array()
        .expect("markdown definition links");
    assert!(
        markdown_definition.iter().any(|link| {
            link["targetUri"].as_str() == Some(file_uri(&source).as_str())
                && link["originSelectionRange"]["start"]["character"].as_i64() == Some(2)
                && link["originSelectionRange"]["end"]["character"].as_i64()
                    == Some(spec_heading.len() as i64)
                && link["targetSelectionRange"]["start"]["line"].as_i64() == Some(1)
        }),
        "markdown title definition navigation should include source-comment citations: {markdown_definition:?}"
    );

    // `## 1. Detail` is line 5 (0-based 4); the section number `1` starts at
    // character 3. Definition and references on the section heading resolve to
    // the `§FS-001-alpha.1` citation, not the whole-ID usages (§FS-lsp.1.3.1).
    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 11,
            "method": "textDocument/definition",
            "params": {
                "textDocument": { "uri": file_uri(&spec) },
                "position": { "line": 4, "character": 6 }
            }
        }),
    );
    let section_definition = recv_response_or_panic(&receiver, &mut child, 11);
    let section_definition = section_definition["result"]
        .as_array()
        .expect("section definition links");
    assert!(
        section_definition.iter().any(|link| {
            link["targetUri"].as_str() == Some(file_uri(&source).as_str())
                && link["originSelectionRange"]["start"]["line"].as_i64() == Some(4)
                && link["originSelectionRange"]["start"]["character"].as_i64() == Some(3)
                && link["originSelectionRange"]["end"]["character"].as_i64()
                    == Some("## 1. Detail".len() as i64)
                && link["targetSelectionRange"]["start"]["line"].as_i64() == Some(1)
        }),
        "section-heading definition should navigate to the section citation: {section_definition:?}"
    );

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 13,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": file_uri(&spec) },
                "position": { "line": 4, "character": 6 }
            }
        }),
    );
    let section_hover_response = recv_response_or_panic(&receiver, &mut child, 13);
    assert!(
        section_hover_response["result"]["range"]["start"]["line"].as_i64() == Some(4)
            && section_hover_response["result"]["range"]["start"]["character"].as_i64() == Some(3)
            && section_hover_response["result"]["range"]["end"]["character"].as_i64()
                == Some("## 1. Detail".len() as i64),
        "section-heading hover should carry the whole section title range: {section_hover_response:?}"
    );

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 14,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": file_uri(&stub) },
                "position": { "line": 0, "character": 22 }
            }
        }),
    );
    let stub_hover_response = recv_response_or_panic(&receiver, &mut child, 14);
    assert!(
        stub_hover_response["result"]["range"]["start"]["character"].as_i64() == Some(2)
            && stub_hover_response["result"]["range"]["end"]["character"].as_i64()
                == Some(stub_heading.len() as i64),
        "stub-title hover should carry the whole stub title range: {stub_hover_response:?}"
    );

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 12,
            "method": "textDocument/references",
            "params": {
                "textDocument": { "uri": file_uri(&spec) },
                "position": { "line": 4, "character": 6 },
                "context": { "includeDeclaration": true }
            }
        }),
    );
    let section_references = recv_response_or_panic(&receiver, &mut child, 12);
    let section_references = section_references["result"]
        .as_array()
        .expect("section references");
    assert!(
        section_references.iter().any(|location| {
            location["uri"].as_str() == Some(file_uri(&source).as_str())
                && location["range"]["start"]["line"].as_i64() == Some(1)
        }),
        "section-heading references should include the section citation: {section_references:?}"
    );
    assert!(
        section_references.iter().any(|location| {
            location["uri"].as_str() == Some(file_uri(&spec).as_str())
                && location["range"]["start"]["line"].as_i64() == Some(4)
        }),
        "section-heading references should include the section heading itself: {section_references:?}"
    );

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "shutdown",
            "params": null
        }),
    );
    recv_response_or_panic(&receiver, &mut child, 10);
    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "exit"
        }),
    );
    drop(stdin);

    wait_for_exit(&mut child);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn diagnostic_anchors_on_offending_citation_token() {
    // A single line can carry several citations, so an unknown-reference
    // diagnostic anchors on the offending token's start column — not merely the
    // first citation on the line (§FS-lsp.1.1).
    let root = test_root("diagnostic-anchor");
    fs::create_dir_all(root.join("docs/functional-spec")).expect("create specs");
    fs::write(
        root.join("docs/functional-spec/FS-001-alpha.md"),
        "# FS-001-alpha: Alpha\n\nLead.\n",
    )
    .expect("write spec");
    // The first citation resolves; the second is a mistyped unknown reference.
    let line = "Refs §FS-001-alpha and §FS-404-ghost.";
    let uses = root.join("docs/uses.md");
    fs::write(&uses, format!("{line}\n")).expect("write uses");

    let mut child = Command::new(env!("CARGO_BIN_EXE_grund-lsp"))
        .current_dir(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn grund-lsp");
    let mut stdin = child.stdin.take().expect("child stdin");
    let receiver = read_messages(child.stdout.take().expect("child stdout"));

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "rootUri": file_uri(&root),
                "capabilities": {}
            }
        }),
    );
    recv_response_or_panic(&receiver, &mut child, 1);
    send_message(
        &mut stdin,
        json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }),
    );

    let diagnostics = recv_diagnostics(&receiver, &mut child, "uses.md");

    // UTF-16 columns: the resolving §FS-001-alpha starts before the unknown
    // §FS-404-ghost, so the two anchors must differ.
    let first_marker_col = line[..line.find('§').expect("first marker")]
        .encode_utf16()
        .count() as i64;
    let ghost_marker_byte = line.match_indices('§').nth(1).expect("second marker").0;
    let ghost_marker_col = line[..ghost_marker_byte].encode_utf16().count() as i64;

    let dangling: Vec<&Value> = diagnostics
        .iter()
        .filter(|d| d["code"].as_str() == Some("dangling"))
        .collect();
    assert_eq!(
        dangling.len(),
        1,
        "exactly one unknown-reference diagnostic expected: {diagnostics:?}"
    );
    let dangling = dangling[0];
    assert!(
        dangling["message"]
            .as_str()
            .is_some_and(|message| message.contains("FS-404-ghost")),
        "diagnostic should name the offending reference: {dangling:?}"
    );
    assert_eq!(
        dangling["range"]["start"]["line"].as_i64(),
        Some(0),
        "diagnostic is on the citation line: {dangling:?}"
    );
    assert_eq!(
        dangling["range"]["start"]["character"].as_i64(),
        Some(ghost_marker_col),
        "diagnostic must anchor on the offending §FS-404-ghost token, not the \
         resolving first citation at column {first_marker_col} (§FS-lsp.1.1): {dangling:?}"
    );

    send_message(
        &mut stdin,
        json!({ "jsonrpc": "2.0", "id": 2, "method": "shutdown", "params": null }),
    );
    recv_response_or_panic(&receiver, &mut child, 2);
    send_message(&mut stdin, json!({ "jsonrpc": "2.0", "method": "exit" }));
    drop(stdin);

    wait_for_exit(&mut child);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn line_anchored_diagnostic_does_not_overlap_dangling_citation() {
    // A dangling citation in an ungrounded source file gets two diagnostics from
    // the checker (§FS-check.3.6), but only the dangling diagnostic belongs to
    // the citation token. The line-level ungrounded diagnostic must not borrow
    // the citation range, because VSCode renders overlapping diagnostics in the
    // same hover popup (§FS-lsp.1.1).
    let root = test_root("diagnostic-line-anchor-no-citation-overlap");
    fs::write(
        root.join(".agents/grund.toml"),
        "grund_config_version = 1\n[scan]\ninclude = [\"docs\", \"src\"]\n\
         extensions = [\"md\", \"rs\"]\n[reference]\nrequire_grounding = true\n\
         [id]\nformat = \"{kind}-{slug}\"\nslug_pattern = \"[a-z][a-z0-9-]*\"\n",
    )
    .expect("write config");
    fs::create_dir_all(root.join("docs/functional-spec")).expect("create specs");
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("docs/functional-spec/FS-check.md"),
        "# FS-check: Check\n\nLead.\n",
    )
    .expect("write spec");
    let marker = '§';
    let line = format!("//! {marker}FS-chek");
    let source = root.join("src/lib.rs");
    fs::write(&source, format!("{line}\n")).expect("write source");

    let mut child = Command::new(env!("CARGO_BIN_EXE_grund-lsp"))
        .current_dir(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn grund-lsp");
    let mut stdin = child.stdin.take().expect("child stdin");
    let receiver = read_messages(child.stdout.take().expect("child stdout"));

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "rootUri": file_uri(&root),
                "capabilities": {}
            }
        }),
    );
    recv_response_or_panic(&receiver, &mut child, 1);
    send_message(
        &mut stdin,
        json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }),
    );

    let diagnostics = recv_diagnostics(&receiver, &mut child, "src/lib.rs");
    let dangling = diagnostics
        .iter()
        .find(|diagnostic| diagnostic["code"].as_str() == Some("dangling"))
        .unwrap_or_else(|| panic!("dangling diagnostic missing: {diagnostics:?}"));
    let ungrounded = diagnostics
        .iter()
        .find(|diagnostic| diagnostic["code"].as_str() == Some("ungrounded"))
        .unwrap_or_else(|| panic!("ungrounded diagnostic missing: {diagnostics:?}"));
    let citation_col = line[..line.find(marker).expect("marker")]
        .encode_utf16()
        .count() as i64;

    assert_eq!(
        dangling["range"]["start"]["character"].as_i64(),
        Some(citation_col),
        "dangling diagnostic should cover the citation token: {dangling:?}"
    );
    assert_eq!(
        ungrounded["range"]["start"]["character"].as_i64(),
        Some(0),
        "line-level ungrounded diagnostic should stay at line start: {ungrounded:?}"
    );
    assert_eq!(
        ungrounded["range"]["end"]["character"].as_i64(),
        Some(1),
        "line-level ungrounded diagnostic must not overlap the citation token at \
         column {citation_col}: {ungrounded:?}"
    );

    send_message(
        &mut stdin,
        json!({ "jsonrpc": "2.0", "id": 2, "method": "shutdown", "params": null }),
    );
    recv_response_or_panic(&receiver, &mut child, 2);
    send_message(&mut stdin, json!({ "jsonrpc": "2.0", "method": "exit" }));
    drop(stdin);

    wait_for_exit(&mut child);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn hover_on_dangling_citation_defers_to_diagnostic() {
    // A citation that cannot resolve has no hover body. Its diagnostic already
    // carries the nearest-ID hint, so returning it from hover too would double
    // the text in editors that render diagnostics in the hover popup; hover
    // returns nothing and the diagnostic stands alone (§FS-lsp.1.2).
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

    let mut child = Command::new(env!("CARGO_BIN_EXE_grund-lsp"))
        .current_dir(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn grund-lsp");
    let mut stdin = child.stdin.take().expect("child stdin");
    let receiver = read_messages(child.stdout.take().expect("child stdout"));

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "rootUri": file_uri(&root),
                "capabilities": {}
            }
        }),
    );
    recv_response_or_panic(&receiver, &mut child, 1);
    send_message(
        &mut stdin,
        json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }),
    );

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

#[test]
fn document_links_cover_python_docstring_citation_columns() {
    // Python docstring content is normalized for scanning, but LSP links must
    // still cover the original editor columns (§AR-scanner.4 §FS-lsp.1.3.2).
    let root = test_root("python-docstring-links");
    fs::write(
        root.join(".agents/grund.toml"),
        "grund_config_version = 1\n[scan]\ninclude = [\"docs\", \"src\"]\n\
         extensions = [\"md\", \"py\"]\n",
    )
    .expect("write config");
    fs::create_dir_all(root.join("docs/functional-spec")).expect("create specs");
    fs::create_dir_all(root.join("src")).expect("create source");
    fs::write(
        root.join("docs/functional-spec/FS-001-alpha.md"),
        "# FS-001-alpha: Alpha\n\nLead.\n",
    )
    .expect("write spec");
    let source = root.join("src/service.py");
    fs::write(
        &source,
        "class Service:\n    \"\"\"\n    Uses §FS-001-alpha.\n    \"\"\"\n\n\
         def inline():\n    \"\"\"Inline §FS-001-alpha.\"\"\"\n",
    )
    .expect("write source");

    let mut child = Command::new(env!("CARGO_BIN_EXE_grund-lsp"))
        .current_dir(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn grund-lsp");
    let mut stdin = child.stdin.take().expect("child stdin");
    let receiver = read_messages(child.stdout.take().expect("child stdout"));

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "rootUri": file_uri(&root),
                "capabilities": {}
            }
        }),
    );
    recv_response_or_panic(&receiver, &mut child, 1);
    send_message(
        &mut stdin,
        json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }),
    );

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/documentLink",
            "params": {
                "textDocument": { "uri": file_uri(&source) }
            }
        }),
    );
    let links = recv_response_or_panic(&receiver, &mut child, 2);
    let links = links["result"].as_array().expect("document links");
    assert!(
        links.iter().any(|link| {
            link["range"]["start"]["line"].as_i64() == Some(2)
                && link["range"]["start"]["character"].as_i64() == Some(9)
                && link["range"]["end"]["character"].as_i64() == Some(22)
                && link["target"]
                    .as_str()
                    .is_some_and(|target| target.contains("FS-001-alpha.md#L1"))
        }),
        "indented Python docstring citation should link at its source column: {links:?}"
    );
    assert!(
        links.iter().any(|link| {
            link["range"]["start"]["line"].as_i64() == Some(6)
                && link["range"]["start"]["character"].as_i64() == Some(14)
                && link["range"]["end"]["character"].as_i64() == Some(27)
                && link["target"]
                    .as_str()
                    .is_some_and(|target| target.contains("FS-001-alpha.md#L1"))
        }),
        "same-line Python docstring citation should link after the opening delimiter: {links:?}"
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

#[test]
fn definition_links_carry_whole_token_origin_span() {
    // Clients that advertise `textDocument.definition.linkSupport` get
    // `LocationLink`s whose `originSelectionRange` is the whole citation or
    // declaration-title span, so editors underline one navigable unit instead
    // of the bare word at the cursor (§FS-lsp.1.3).
    let root = test_root("definition-links");
    fs::create_dir_all(root.join("docs/functional-spec")).expect("create specs");
    fs::create_dir_all(root.join("docs/architecture")).expect("create architecture");
    let spec = root.join("docs/functional-spec/FS-001-alpha.md");
    let user = root.join("docs/architecture/AR-002-user.md");
    let spec_heading = "# FS-001-alpha: Alpha";
    fs::write(
        &spec,
        format!("{spec_heading}\n\nLead.\n\n## 1. Detail\nMore.\n"),
    )
    .expect("write spec");
    let citation_line = "Uses §FS-001-alpha.";
    fs::write(&user, format!("# AR-002-user: User\n\n{citation_line}\n")).expect("write user");

    let mut child = Command::new(env!("CARGO_BIN_EXE_grund-lsp"))
        .current_dir(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn grund-lsp");
    let mut stdin = child.stdin.take().expect("child stdin");
    let receiver = read_messages(child.stdout.take().expect("child stdout"));

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "rootUri": file_uri(&root),
                "capabilities": {
                    "textDocument": { "definition": { "linkSupport": true } }
                }
            }
        }),
    );
    recv_response_or_panic(&receiver, &mut child, 1);
    send_message(
        &mut stdin,
        json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }),
    );

    // Definition on the Markdown declaration title returns its usages as links
    // whose origin span covers the whole title, not just one word.
    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/definition",
            "params": {
                "textDocument": { "uri": file_uri(&spec) },
                "position": { "line": 0, "character": 17 }
            }
        }),
    );
    let title_definition = recv_response_or_panic(&receiver, &mut child, 2);
    let title_links = title_definition["result"]
        .as_array()
        .expect("declaration definition links");
    assert!(
        title_links.iter().any(|link| {
            link["originSelectionRange"]["start"]["line"].as_i64() == Some(0)
                && link["originSelectionRange"]["start"]["character"].as_i64() == Some(2)
                && link["originSelectionRange"]["end"]["character"].as_i64()
                    == Some(spec_heading.len() as i64)
                && link["targetUri"]
                    .as_str()
                    .is_some_and(|target| target.contains("AR-002-user.md"))
        }),
        "declaration-title definition link origin should span the whole title: {title_links:?}"
    );

    // Definition on a citation returns one link whose origin span covers the
    // whole `§<ID>` token rather than the word the cursor happens to be on.
    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "textDocument/definition",
            "params": {
                "textDocument": { "uri": file_uri(&user) },
                "position": { "line": 2, "character": 8 }
            }
        }),
    );
    let citation_definition = recv_response_or_panic(&receiver, &mut child, 3);
    let citation_links = citation_definition["result"]
        .as_array()
        .expect("citation definition links");
    let marker_index = citation_line.find('§').expect("citation marker") as i64;
    assert!(
        citation_links.iter().any(|link| {
            link["originSelectionRange"]["start"]["line"].as_i64() == Some(2)
                && link["originSelectionRange"]["start"]["character"].as_i64() == Some(marker_index)
                && link["originSelectionRange"]["end"]["character"].as_i64()
                    > Some(marker_index + 1)
                && link["targetSelectionRange"]["start"]["line"].as_i64() == Some(0)
                && link["targetSelectionRange"]["start"]["character"].as_i64() == Some(2)
                && link["targetSelectionRange"]["end"]["character"].as_i64()
                    == Some(spec_heading.len() as i64)
                && link["targetUri"]
                    .as_str()
                    .is_some_and(|target| target.contains("FS-001-alpha.md"))
        }),
        "citation definition link should span the source token and target title: {citation_links:?}"
    );

    send_message(
        &mut stdin,
        json!({ "jsonrpc": "2.0", "id": 4, "method": "shutdown", "params": null }),
    );
    recv_response_or_panic(&receiver, &mut child, 4);
    send_message(&mut stdin, json!({ "jsonrpc": "2.0", "method": "exit" }));
    drop(stdin);

    wait_for_exit(&mut child);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn document_highlight_marks_whole_citation_token() {
    // With no highlight provider an editor falls back to its word pattern and
    // boxes only one sub-word of `§FS-001-alpha`. The server marks the whole
    // token under the cursor as one span, plus the sibling citation of the same
    // ID in the same file (§FS-lsp.1.3.3).
    let root = test_root("document-highlight");
    fs::create_dir_all(root.join("docs/functional-spec")).expect("create specs");
    fs::create_dir_all(root.join("docs/architecture")).expect("create architecture");
    let spec = root.join("docs/functional-spec/FS-001-alpha.md");
    let user = root.join("docs/architecture/AR-002-user.md");
    fs::write(&spec, "# FS-001-alpha: Alpha\n\nLead.\n").expect("write spec");
    let citation_line = "Uses §FS-001-alpha and again §FS-001-alpha.";
    fs::write(&user, format!("# AR-002-user: User\n\n{citation_line}\n")).expect("write user");

    let mut child = Command::new(env!("CARGO_BIN_EXE_grund-lsp"))
        .current_dir(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn grund-lsp");
    let mut stdin = child.stdin.take().expect("child stdin");
    let receiver = read_messages(child.stdout.take().expect("child stdout"));

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "rootUri": file_uri(&root),
                "capabilities": {}
            }
        }),
    );
    recv_response_or_panic(&receiver, &mut child, 1);
    send_message(
        &mut stdin,
        json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }),
    );

    // Byte and UTF-16 offsets of the first citation, then a cursor parked on a
    // sub-word inside it (a few units past the marker).
    let first_marker_byte = citation_line.find('§').expect("first marker");
    let first_marker_col = citation_line[..first_marker_byte].encode_utf16().count();
    let token_units = "§FS-001-alpha".encode_utf16().count();
    let cursor = first_marker_col + 7;

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/documentHighlight",
            "params": {
                "textDocument": { "uri": file_uri(&user) },
                "position": { "line": 2, "character": cursor }
            }
        }),
    );
    let response = recv_response_or_panic(&receiver, &mut child, 2);
    let highlights = response["result"].as_array().expect("document highlights");

    assert!(
        highlights.iter().any(|highlight| {
            highlight["range"]["start"]["line"].as_i64() == Some(2)
                && highlight["range"]["start"]["character"].as_i64()
                    == Some(first_marker_col as i64)
                && highlight["range"]["end"]["character"].as_i64()
                    == Some((first_marker_col + token_units) as i64)
        }),
        "highlight should cover the whole §FS-001-alpha token under the cursor, not a sub-word: {highlights:?}"
    );

    let second_marker_byte = citation_line
        .match_indices('§')
        .nth(1)
        .expect("second marker")
        .0;
    let second_marker_col = citation_line[..second_marker_byte].encode_utf16().count();
    assert!(
        highlights.iter().any(|highlight| {
            highlight["range"]["start"]["character"].as_i64() == Some(second_marker_col as i64)
        }),
        "the sibling citation of the same ID in this file should also be highlighted: {highlights:?}"
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

#[test]
fn forbidden_citation_surfaces_as_diagnostic() {
    // §FS-lsp.1.1: a citation that violates a `[citations]` `must-not` rule is a
    // `forbidden-citation` error in `grund check`, so the LSP must surface the
    // same diagnostic. The snapshot classifies citing sides for this to work.
    let root = test_root("forbidden-citation");
    fs::write(
        root.join(".agents/grund.toml"),
        "grund_config_version = 1\n\
         [scan]\ninclude = [\"docs\"]\nextensions = [\"md\"]\n\n\
         [citations]\ndefault = \"may\"\n\n\
         [citations.FS]\nmust-not = [\"AR\"]\n",
    )
    .expect("write config");
    fs::create_dir_all(root.join("docs/architecture")).expect("create arch dir");
    fs::create_dir_all(root.join("docs/functional-spec")).expect("create spec dir");
    // A real AR home, so the offending citation resolves — the finding is a
    // direction violation, not a dangling reference.
    fs::write(
        root.join("docs/architecture/AR-001-arch.md"),
        "# AR-001-arch: Arch\n\nLead.\n",
    )
    .expect("write arch");
    // The FS body cites AR, which its `must-not` rule forbids.
    let spec = root.join("docs/functional-spec/FS-001-alpha.md");
    fs::write(
        &spec,
        "# FS-001-alpha: Alpha\n\nThe behavior leans on §AR-001-arch.\n",
    )
    .expect("write spec");

    let mut child = Command::new(env!("CARGO_BIN_EXE_grund-lsp"))
        .current_dir(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn grund-lsp");
    let mut stdin = child.stdin.take().expect("child stdin");
    let receiver = read_messages(child.stdout.take().expect("child stdout"));

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "rootUri": file_uri(&root),
                "capabilities": {}
            }
        }),
    );
    recv_response_or_panic(&receiver, &mut child, 1);
    send_message(
        &mut stdin,
        json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }),
    );

    let diagnostics = recv_diagnostics(&receiver, &mut child, "FS-001-alpha.md");
    let forbidden: Vec<&Value> = diagnostics
        .iter()
        .filter(|d| d["code"].as_str() == Some("forbidden-citation"))
        .collect();
    assert_eq!(
        forbidden.len(),
        1,
        "exactly one forbidden-citation diagnostic expected: {diagnostics:?}"
    );
    assert!(
        forbidden[0]["message"]
            .as_str()
            .is_some_and(|message| message.contains("AR")),
        "diagnostic should name the forbidden target kind: {:?}",
        forbidden[0]
    );

    send_message(
        &mut stdin,
        json!({ "jsonrpc": "2.0", "id": 4, "method": "shutdown", "params": null }),
    );
    recv_response_or_panic(&receiver, &mut child, 4);
    send_message(&mut stdin, json!({ "jsonrpc": "2.0", "method": "exit" }));
    drop(stdin);

    wait_for_exit(&mut child);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn malformed_message_does_not_crash_server() {
    // A notification whose params do not deserialize, and a request that fails,
    // must not tear the session down: the server logs and keeps serving, and a
    // following request still gets a response (§FS-lsp.1).
    let root = test_root("malformed-message");
    fs::create_dir_all(root.join("docs/functional-spec")).expect("create specs");
    fs::write(
        root.join("docs/functional-spec/FS-001-alpha.md"),
        "# FS-001-alpha: Alpha\n\nLead.\n",
    )
    .expect("write spec");

    let mut child = Command::new(env!("CARGO_BIN_EXE_grund-lsp"))
        .current_dir(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn grund-lsp");
    let mut stdin = child.stdin.take().expect("child stdin");
    let receiver = read_messages(child.stdout.take().expect("child stdout"));

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "rootUri": file_uri(&root),
                "capabilities": {}
            }
        }),
    );
    recv_response_or_panic(&receiver, &mut child, 1);
    send_message(
        &mut stdin,
        json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }),
    );

    // A `didChange` whose params are missing the required fields fails to
    // deserialize inside the notification handler.
    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": { "unexpected": true }
        }),
    );
    // A request whose params are likewise malformed fails inside the handler.
    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/hover",
            "params": { "unexpected": true }
        }),
    );
    // The failed request still gets a response rather than silence …
    let bad_response = recv_response_or_panic(&receiver, &mut child, 2);
    assert!(
        bad_response.get("error").is_some(),
        "a malformed request should get an error response, not crash the server: {bad_response}"
    );

    // … and a well-formed request afterwards is still served, proving the loop
    // survived both failures.
    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": file_uri(&root.join("docs/functional-spec/FS-001-alpha.md")) },
                "position": { "line": 0, "character": 0 }
            }
        }),
    );
    let ok_response = recv_response_or_panic(&receiver, &mut child, 3);
    assert!(
        ok_response.get("result").is_some() && ok_response.get("error").is_none(),
        "the server must keep answering after a malformed message: {ok_response}"
    );

    send_message(
        &mut stdin,
        json!({ "jsonrpc": "2.0", "id": 4, "method": "shutdown", "params": null }),
    );
    recv_response_or_panic(&receiver, &mut child, 4);
    send_message(&mut stdin, json!({ "jsonrpc": "2.0", "method": "exit" }));
    drop(stdin);

    wait_for_exit(&mut child);
    let _ = fs::remove_dir_all(root);
}
