use super::*;
use std::fs;

fn test_root(name: &str) -> PathBuf {
    let unique = format!(
        "{}-{}-{:?}",
        name,
        std::process::id(),
        std::thread::current().id()
    );
    let dir = std::env::temp_dir().join("grund-lsp-tests").join(unique);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create test root");
    dir
}

fn write(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, text).expect("write fixture");
}

#[test]
fn hover_linkifier_preserves_existing_markdown_links() {
    let linked = replace_unlinked_token(
        "See §FS-lsp and [§FS-lsp](already).",
        "§FS-lsp",
        "file:///tmp/FS-lsp.md#L1",
    );
    assert_eq!(
        linked,
        "See [§FS-lsp](file:///tmp/FS-lsp.md#L1) and [§FS-lsp](already)."
    );
}

#[test]
fn hover_linkifier_uses_displayed_citation_text_for_workspace_links() {
    let root = test_root("hover_linkifier_uses_displayed_citation_text_for_workspace_links");
    let target = root.join("docs/functional-spec/FS-002-b.md");
    let citation = LspCitation {
        project: Some("root".to_string()),
        path: root.join("docs/functional-spec/FS-001-a.md"),
        display_path: "docs/functional-spec/FS-001-a.md".to_string(),
        line: 3,
        column: 6,
        text: "§FS-002-b".to_string(),
        query_id: "root/FS-002-b".to_string(),
        declaration_query_id: "root/FS-002-b".to_string(),
        section_separator: ".".to_string(),
        target_path: Some(target),
        target_line: Some(1),
    };

    let linked = linkify_hover_body("See §FS-002-b.", &[citation]);

    assert!(
        linked.contains("[§FS-002-b]("),
        "hover linkification must use the displayed citation text, not the workspace-qualified query ID (§FS-lsp.1.2): {linked}"
    );
    assert!(
        !linked.contains("§root/FS-002-b"),
        "workspace query IDs are not present in local hover prose: {linked}"
    );
}

#[test]
fn utf16_position_conversion_handles_non_ascii() {
    let line = "a§𐐀b";
    let byte = utf16_to_byte(line, 2);
    assert_eq!(&line[..byte], "a§");
    assert_eq!(byte_to_utf16(line, byte), 2);
}

#[test]
fn on_type_capabilities_cover_configured_trigger_punctuation() {
    let capabilities = server_capabilities("%%");
    let Some(DocumentOnTypeFormattingOptions {
        first_trigger_character,
        more_trigger_character,
    }) = capabilities.document_on_type_formatting_provider
    else {
        panic!("on-type formatting capability");
    };
    assert_eq!(first_trigger_character, "%");
    let more = more_trigger_character.expect("more trigger characters");
    assert!(more.iter().any(|ch| ch == "%"));
    assert!(more.iter().any(|ch| ch == ":"));
    let workspace_folders = capabilities
        .workspace
        .and_then(|workspace| workspace.workspace_folders)
        .expect("workspace-folder capabilities");
    assert_eq!(workspace_folders.supported, Some(true));
    assert_eq!(
        workspace_folders.change_notifications,
        Some(OneOf::Left(true))
    );
}

#[test]
fn omitted_definition_link_support_still_uses_location_links() {
    let omitted: InitializeParams = serde_json::from_value(json!({
        "processId": std::process::id(),
        "rootUri": "file:///tmp",
        "capabilities": {}
    }))
    .expect("initialize params");
    assert!(client_supports_definition_links(&omitted));

    let explicit_false: InitializeParams = serde_json::from_value(json!({
        "processId": std::process::id(),
        "rootUri": "file:///tmp",
        "capabilities": {
            "textDocument": { "definition": { "linkSupport": false } }
        }
    }))
    .expect("initialize params");
    assert!(!client_supports_definition_links(&explicit_false));
}

#[test]
fn nonempty_virtual_workspace_folders_do_not_fall_back_to_root_uri() {
    let params: InitializeParams = serde_json::from_value(json!({
        "processId": std::process::id(),
        "rootUri": "file:///tmp/unrelated-local-root",
        "workspaceFolders": [
            { "uri": "vscode-vfs://github/acme/repo", "name": "virtual" }
        ],
        "capabilities": { "workspace": { "workspaceFolders": true } }
    }))
    .expect("initialize params");

    assert!(
        initialize_folders(&params)
            .expect("initialize folders")
            .is_empty(),
        "a present workspaceFolders list is authoritative even when every URI is virtual"
    );
}

#[test]
fn on_type_formatting_accepts_configured_id_punctuation() {
    let root = test_root("on_type_formatting_accepts_configured_id_punctuation");
    write(
        &root.join("grund.toml"),
        "grund_config_version = 1\n[id]\nformat = \"{kind}:{slug}\"\n",
    );
    let path = root.join("src/lib.rs");
    write(&path, "//! $$FS:login\n");
    let edits = on_type_replacement_for_line(
        &path,
        "//! $$FS:login",
        Position {
            line: 0,
            character: "//! $$FS:login".len() as u32,
        },
        &[],
    )
    .expect("formatting check");
    assert_eq!(
        edits
            .expect("formatting response")
            .first()
            .map(|edit| edit.new_text.as_str()),
        Some("§")
    );
}

#[test]
fn on_type_formatting_uses_member_trigger_and_marker() {
    let root = test_root("on_type_formatting_uses_member_trigger_and_marker");
    write(
        &root.join("grund.toml"),
        "grund_config_version = 1\n[reference]\ntrigger = \"$$\"\nmarker = \"§\"\n\
         [workspace]\nmembers = [\"packages/app\"]\n",
    );
    write(
        &root.join("packages/app/grund.toml"),
        "grund_config_version = 1\n[reference]\ntrigger = \"%%\"\nmarker = \"@\"\n",
    );
    let path = root.join("packages/app/src/lib.rs");
    write(&path, "//! %%FS-001-login\n");
    let edits = on_type_replacement_for_line(
        &path,
        "//! %%FS-001-login",
        Position {
            line: 0,
            character: "//! %%FS-001-login".len() as u32,
        },
        &[],
    )
    .expect("formatting check");
    assert_eq!(
        edits
            .expect("formatting response")
            .first()
            .map(|edit| edit.new_text.as_str()),
        Some("@")
    );

    let root_trigger_edits = on_type_replacement_for_line(
        &path,
        "//! $$FS-001-login",
        Position {
            line: 0,
            character: "//! $$FS-001-login".len() as u32,
        },
        &[],
    )
    .expect("formatting check");
    assert!(
        root_trigger_edits.expect("formatting response").is_empty(),
        "member file should not use the root trigger"
    );
}

#[test]
fn document_link_targets_include_line_fragment() {
    let root = test_root("document_link_targets_include_line_fragment");
    let path = root.join("docs/functional-spec/FS-login.md");
    write(&path, "# FS-login: Login\n");
    let marker = "\u{00a7}";
    let citation = LspCitation {
        project: None,
        path: root.join("src/lib.rs"),
        display_path: "src/lib.rs".to_string(),
        line: 1,
        column: 5,
        text: format!("{marker}FS-login"),
        query_id: "FS-login".to_string(),
        declaration_query_id: "FS-login".to_string(),
        section_separator: ".".to_string(),
        target_path: Some(path),
        target_line: Some(7),
    };
    assert_eq!(
        document_link_target(&citation)
            .expect("document link target")
            .fragment(),
        Some("L7")
    );
}
