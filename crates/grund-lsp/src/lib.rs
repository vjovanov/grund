//! LSP transport over stdio. §AR-lsp.4

use anyhow::{Context, Result, anyhow};
use grund_core::{
    DeclaredId, Finding, LspCitation, LspDeclaration, LspSnapshot, LspSnapshotOpts, LspStub,
    LspUsage, ShowFormat, ShowMode, ShowOpts, canonical_snapshot_path, citation_under_title,
    effective_config, lsp_snapshot, lsp_title_hover_body, on_type_line_edits, show_with_overlays,
};
use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::{
    Diagnostic, DiagnosticSeverity, DocumentHighlight, DocumentHighlightKind, DocumentLink,
    DocumentLinkOptions, DocumentOnTypeFormattingOptions, DocumentOnTypeFormattingParams,
    GotoDefinitionResponse, Hover, HoverContents, InitializeParams, Location, LocationLink,
    MarkupContent, MarkupKind, OneOf, Position, PublishDiagnosticsParams, Range, ReferenceParams,
    ServerCapabilities, TextDocumentContentChangeEvent, TextDocumentPositionParams,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions, TextEdit, Url,
    WorkspaceFolder, WorkspaceFoldersServerCapabilities, WorkspaceServerCapabilities,
};
use serde_json::Value;
use serde_json::json;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub fn run() -> Result<()> {
    let (connection, io_threads) = Connection::stdio();
    let (initialize_id, initialize_value) = connection.initialize_start()?;
    let initialize_params: InitializeParams = serde_json::from_value(initialize_value)?;
    let folders = initialize_folders(&initialize_params)?;
    let definition_link_support = client_supports_definition_links(&initialize_params);
    let mut server = Server::new(connection, folders, definition_link_support)?;
    let initialize_result = json!({
        "capabilities": server_capabilities(server.trigger()),
        "serverInfo": {
            "name": "grund-lsp",
            "version": env!("CARGO_PKG_VERSION"),
        },
    });
    server
        .connection
        .initialize_finish(initialize_id, initialize_result)?;
    server.publish_diagnostics()?;
    server.event_loop()?;
    drop(server);
    io_threads.join()?;
    Ok(())
}

fn server_capabilities(trigger: &str) -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::FULL),
                save: Some(lsp_types::TextDocumentSyncSaveOptions::Supported(true)),
                ..TextDocumentSyncOptions::default()
            },
        )),
        hover_provider: Some(lsp_types::HoverProviderCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
        references_provider: Some(OneOf::Left(true)),
        document_highlight_provider: Some(OneOf::Left(true)),
        document_link_provider: Some(DocumentLinkOptions {
            resolve_provider: Some(false),
            work_done_progress_options: Default::default(),
        }),
        document_on_type_formatting_provider: Some(DocumentOnTypeFormattingOptions {
            first_trigger_character: trigger
                .chars()
                .next()
                .map(|ch| ch.to_string())
                .unwrap_or_else(|| "$".to_string()),
            more_trigger_character: Some(on_type_trigger_characters(trigger)),
        }),
        workspace: Some(WorkspaceServerCapabilities {
            workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                supported: Some(true),
                change_notifications: Some(OneOf::Left(true)),
            }),
            file_operations: None,
        }),
        ..ServerCapabilities::default()
    }
}

fn on_type_trigger_characters(trigger: &str) -> Vec<String> {
    let mut chars = BTreeSet::new();
    for ch in '!'..='~' {
        chars.insert(ch.to_string());
    }
    // §FS-lsp.1.4: the shorthand expands on the keystroke that *ends* the token,
    // and a space is the commonest way to end one. The printable range above
    // starts at `!`, so without this the ordinary `see §FS-042 and …` never fires.
    chars.insert(" ".to_string());
    for ch in trigger.chars() {
        chars.insert(ch.to_string());
    }
    chars.into_iter().collect()
}

/// Whether the client can receive `LocationLink` definition results
/// (`textDocument.definition.linkSupport`). Plain `Location` results carry no
/// origin span, so editors fall back to underlining the word at the cursor;
/// `LocationLink` lets us hand back the whole token span as one navigable unit.
/// Some clients omit the flag even though they understand the union member, so
/// only an explicit `false` opts out (§FS-lsp.1.3).
fn client_supports_definition_links(params: &InitializeParams) -> bool {
    params
        .capabilities
        .text_document
        .as_ref()
        .and_then(|text_document| text_document.definition.as_ref())
        .and_then(|definition| definition.link_support)
        .unwrap_or(true)
}

fn initialize_folders(params: &InitializeParams) -> Result<BTreeSet<PathBuf>> {
    if let Some(folders) = params
        .workspace_folders
        .as_ref()
        .filter(|folders| !folders.is_empty())
    {
        return folders.iter().map(workspace_folder_path).collect();
    }
    #[allow(deprecated)]
    if let Some(uri) = &params.root_uri {
        let root = uri
            .to_file_path()
            .map_err(|_| anyhow!("initialize rootUri is not a file URI: {uri}"))?;
        return Ok(BTreeSet::from([canonical_snapshot_path(&root)]));
    }
    Ok(BTreeSet::from([canonical_snapshot_path(
        &std::env::current_dir().context("resolve current directory")?,
    )]))
}

fn workspace_folder_path(folder: &WorkspaceFolder) -> Result<PathBuf> {
    folder
        .uri
        .to_file_path()
        .map(|path| canonical_snapshot_path(&path))
        .map_err(|_| anyhow!("workspace folder URI is not a file URI: {}", folder.uri))
}

/// Resolve an editor folder to the scan root it names. A discovered config owns
/// the scan, including sibling `[scan] include` roots; only a zero-config folder
/// remains its own boundary (§FS-lsp.2.2).
fn project_root(folder: &Path) -> Result<PathBuf> {
    let config = effective_config(folder)?;
    if config.config_file.is_some() {
        Ok(canonical_snapshot_path(&config.root))
    } else {
        Ok(canonical_snapshot_path(folder))
    }
}

struct ProjectSnapshot {
    root: PathBuf,
    snapshot: LspSnapshot,
}

struct Server {
    connection: Connection,
    workspace_folders: BTreeSet<PathBuf>,
    projects: Vec<ProjectSnapshot>,
    open_docs: BTreeMap<Url, String>,
    diagnostic_uris: BTreeSet<Url>,
    definition_link_support: bool,
    // Per-snapshot line cache so a file is read and split at most once between
    // refreshes, rather than once per token: `document_links` and `references`
    // walk every citation on a file and each used to re-read the whole file
    // (§AR-lsp.5). Cleared in `refresh` whenever the snapshot is rebuilt.
    line_cache: RefCell<BTreeMap<PathBuf, Vec<String>>>,
}

impl Server {
    fn new(
        connection: Connection,
        workspace_folders: BTreeSet<PathBuf>,
        definition_link_support: bool,
    ) -> Result<Self> {
        let mut server = Self {
            connection,
            workspace_folders,
            projects: Vec::new(),
            open_docs: BTreeMap::new(),
            diagnostic_uris: BTreeSet::new(),
            definition_link_support,
            line_cache: RefCell::new(BTreeMap::new()),
        };
        server.refresh()?;
        Ok(server)
    }

    fn trigger(&self) -> &str {
        self.projects
            .first()
            .map(|project| project.snapshot.trigger.as_str())
            .unwrap_or("$$")
    }

    fn event_loop(&mut self) -> Result<()> {
        while let Ok(message) = self.connection.receiver.recv() {
            match message {
                Message::Request(request) => {
                    if self.connection.handle_shutdown(&request)? {
                        return Ok(());
                    }
                    // A request that fails (bad params, resolution error) is
                    // answered with an error response, never fatal to the
                    // session (§FS-lsp.1).
                    self.handle_request(request)?;
                }
                Message::Notification(notification) => {
                    let method = notification.method.clone();
                    // A malformed or failing notification — e.g. params that do
                    // not deserialize — is logged to stderr and skipped, so one
                    // bad message cannot tear the server down mid-session
                    // (§FS-lsp.1).
                    if let Err(err) = self.handle_notification(notification) {
                        eprintln!("grund-lsp: notification `{method}` failed: {err:#}");
                    }
                }
                Message::Response(_) => {}
            }
        }
        Ok(())
    }

    fn refresh(&mut self) -> Result<()> {
        // Each edit rebuilds the whole snapshot — a full workspace scan (and,
        // for projects with `[citations]`, the direction classification pass).
        // That is acceptable at the current scale; an incremental rescan keyed
        // off the changed document is the natural optimization for large repos,
        // left as future work.
        let roots = self
            .workspace_folders
            .iter()
            .map(|folder| project_root(folder))
            .collect::<Result<BTreeSet<_>>>()?;
        let overlays = self.open_document_overlays();
        self.projects = roots
            .into_iter()
            .map(|root| {
                let snapshot = lsp_snapshot(LspSnapshotOpts {
                    path: root.clone(),
                    // A discovered project root is the whole project, so its
                    // configured include paths govern just as for a root CLI run.
                    path_provided: true,
                    open_documents: overlays.clone(),
                })?;
                Ok(ProjectSnapshot { root, snapshot })
            })
            .collect::<Result<Vec<_>>>()?;
        // The rebuilt snapshot reflects the new document contents, so any cached
        // lines from before the edit are stale.
        self.line_cache.borrow_mut().clear();
        Ok(())
    }

    fn handle_request(&mut self, request: Request) -> Result<()> {
        let id = request.id.clone();
        let method = request.method.clone();
        let outcome = match method.as_str() {
            "textDocument/hover" => self.hover(request.params).and_then(to_value),
            "textDocument/definition" => self.definition(request.params).and_then(to_value),
            "textDocument/references" => self.references(request.params).and_then(to_value),
            "textDocument/documentHighlight" => {
                self.document_highlights(request.params).and_then(to_value)
            }
            "textDocument/documentLink" => self.document_links(request.params).and_then(to_value),
            "textDocument/onTypeFormatting" => {
                self.on_type_formatting(request.params).and_then(to_value)
            }
            _ => {
                let response = Response::new_err(
                    id,
                    lsp_server::ErrorCode::MethodNotFound as i32,
                    format!("unsupported request `{method}`"),
                );
                return self.send_response(response);
            }
        };
        let response = match outcome {
            Ok(value) => Response::new_ok(id, value),
            Err(err) => {
                eprintln!("grund-lsp: request `{method}` failed: {err:#}");
                Response::new_err(
                    id,
                    lsp_server::ErrorCode::InternalError as i32,
                    err.to_string(),
                )
            }
        };
        self.send_response(response)
    }

    fn send_response(&self, response: Response) -> Result<()> {
        self.connection.sender.send(Message::Response(response))?;
        Ok(())
    }

    fn handle_notification(&mut self, notification: Notification) -> Result<()> {
        match notification.method.as_str() {
            "initialized" => {
                self.refresh()?;
                self.publish_diagnostics()?;
            }
            "textDocument/didOpen" => {
                let params: lsp_types::DidOpenTextDocumentParams =
                    serde_json::from_value(notification.params)?;
                self.open_docs
                    .insert(params.text_document.uri, params.text_document.text);
                self.refresh()?;
                self.publish_diagnostics()?;
            }
            "textDocument/didChange" => {
                let params: lsp_types::DidChangeTextDocumentParams =
                    serde_json::from_value(notification.params)?;
                if let Some(text) = full_change_text(params.content_changes) {
                    self.open_docs.insert(params.text_document.uri, text);
                }
                self.refresh()?;
                self.publish_diagnostics()?;
            }
            "textDocument/didSave" => {
                let params: lsp_types::DidSaveTextDocumentParams =
                    serde_json::from_value(notification.params)?;
                if let Some(text) = params.text {
                    self.open_docs.insert(params.text_document.uri, text);
                } else {
                    self.open_docs.remove(&params.text_document.uri);
                }
                self.refresh()?;
                self.publish_diagnostics()?;
            }
            "textDocument/didClose" => {
                let params: lsp_types::DidCloseTextDocumentParams =
                    serde_json::from_value(notification.params)?;
                self.open_docs.remove(&params.text_document.uri);
                self.refresh()?;
                self.publish_diagnostics()?;
            }
            "workspace/didChangeWatchedFiles" => {
                self.refresh()?;
                self.publish_diagnostics()?;
            }
            "workspace/didChangeWorkspaceFolders" => {
                let params: lsp_types::DidChangeWorkspaceFoldersParams =
                    serde_json::from_value(notification.params)?;
                let removed = params
                    .event
                    .removed
                    .iter()
                    .map(workspace_folder_path)
                    .collect::<Result<Vec<_>>>()?;
                let added = params
                    .event
                    .added
                    .iter()
                    .map(workspace_folder_path)
                    .collect::<Result<Vec<_>>>()?;
                for folder in removed {
                    self.workspace_folders.remove(&folder);
                }
                self.workspace_folders.extend(added);
                self.refresh()?;
                self.publish_diagnostics()?;
            }
            _ => {}
        }
        Ok(())
    }

    fn hover(&self, params: Value) -> Result<Option<Hover>> {
        let params: TextDocumentPositionParams = serde_json::from_value(params)?;
        let Some((snapshot, token)) = self.token_at(&params.text_document.uri, params.position)
        else {
            return Ok(None);
        };
        let citation = match token {
            Token::Citation(citation) => citation,
            // A declaration-side title has no body to preview — the cursor is
            // already in it — so the hover carries what is not on screen: how
            // many sites cite this title, across how many files (§FS-lsp.1.2).
            Token::Declaration(decl) => {
                let usage = snapshot.title_usage(&decl.query_id, &decl.section_separator);
                return Ok(Some(title_hover(
                    declaration_range(decl, self),
                    &decl.text,
                    usage,
                )));
            }
            Token::Stub(stub) => {
                let usage = snapshot.title_usage(&stub.query_id, &stub.section_separator);
                return Ok(Some(title_hover(stub_range(stub, self), &stub.text, usage)));
            }
        };
        // A citation that does not resolve has no preview body. Its diagnostic
        // already carries the nearest-ID hint through publishDiagnostics;
        // returning that text from hover too would double it
        // in editors that render diagnostics inside the hover popup (§FS-lsp.1.2).
        let body = match show_with_overlays(
            &citation.query_id,
            ShowOpts {
                path: snapshot.root.clone(),
                section: None,
                mode: ShowMode::Toc,
                format: ShowFormat::Markdown,
            },
            self.open_document_overlays(),
        ) {
            Ok(output) => linkify_hover_body(&output.body, &snapshot.citations),
            Err(_) => return Ok(None),
        };
        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: body,
            }),
            range: Some(citation_range(citation, self)),
        }))
    }

    fn definition(&self, params: Value) -> Result<Option<GotoDefinitionResponse>> {
        let params: TextDocumentPositionParams = serde_json::from_value(params)?;
        let Some((snapshot, token)) = self.token_at(&params.text_document.uri, params.position)
        else {
            return Ok(None);
        };
        // The origin span is the whole token (citation, declaration title, or
        // stub title) under the cursor, so editors underline it as one unit
        // rather than the bare word at the click position (§FS-lsp.1.3).
        let origin = token.range(self);
        match token {
            Token::Citation(citation) => Ok(self
                .citation_location(snapshot, citation)
                .map(|location| self.scalar_definition(origin, location))),
            Token::Declaration(decl) => {
                let locations = self.citation_locations_for_declaration(snapshot, decl);
                if locations.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(self.array_definition(origin, locations)))
                }
            }
            Token::Stub(stub) => Ok(self
                .stub_target_location(snapshot, stub)
                .map(|location| self.scalar_definition(origin, location))),
        }
    }

    /// A single-target definition result, carried as a `LocationLink` (with the
    /// origin span) when the client opted into definition links, and as a plain
    /// `Location` otherwise.
    fn scalar_definition(&self, origin: Range, location: Location) -> GotoDefinitionResponse {
        if self.definition_link_support {
            GotoDefinitionResponse::Link(vec![location_link(origin, location)])
        } else {
            GotoDefinitionResponse::Scalar(location)
        }
    }

    /// A multi-target definition result (a declaration's usages), carried as
    /// `LocationLink`s sharing the declaration-title origin span when the client
    /// opted in, and as a plain `Location` array otherwise.
    fn array_definition(&self, origin: Range, locations: Vec<Location>) -> GotoDefinitionResponse {
        if self.definition_link_support {
            GotoDefinitionResponse::Link(
                locations
                    .into_iter()
                    .map(|location| location_link(origin, location))
                    .collect(),
            )
        } else {
            GotoDefinitionResponse::Array(locations)
        }
    }

    fn references(&self, params: Value) -> Result<Option<Vec<Location>>> {
        let params: ReferenceParams = serde_json::from_value(params)?;
        let Some((snapshot, token)) = self.token_at(
            &params.text_document_position.text_document.uri,
            params.text_document_position.position,
        ) else {
            return Ok(None);
        };
        let include_decl = params.context.include_declaration;
        let mut locations = Vec::new();
        match token {
            Token::Declaration(decl) => {
                if include_decl && let Some(location) = self.declaration_location(decl) {
                    locations.push(location);
                }
                locations.extend(self.citation_locations_for_declaration(snapshot, decl));
            }
            Token::Citation(source) => {
                if include_decl {
                    for decl in &snapshot.declarations {
                        if decl.query_id == source.declaration_query_id
                            && let Some(location) = self.declaration_location(decl)
                        {
                            locations.push(location);
                        }
                    }
                }
                for citation in &snapshot.citations {
                    if citation_under_title(
                        &source.declaration_query_id,
                        &citation.query_id,
                        &source.section_separator,
                    ) && let Some(uri) = path_uri(&citation.path)
                    {
                        locations.push(Location {
                            uri,
                            range: citation_range(citation, self),
                        });
                    }
                }
            }
            Token::Stub(stub) => {
                if include_decl && let Some(location) = self.stub_target_location(snapshot, stub) {
                    locations.push(location);
                }
                for citation in &snapshot.citations {
                    if citation_under_title(
                        &stub.query_id,
                        &citation.query_id,
                        &stub.section_separator,
                    ) && let Some(uri) = path_uri(&citation.path)
                    {
                        locations.push(Location {
                            uri,
                            range: citation_range(citation, self),
                        });
                    }
                }
            }
        }
        Ok(Some(locations))
    }

    fn citation_locations_for_declaration(
        &self,
        snapshot: &LspSnapshot,
        decl: &LspDeclaration,
    ) -> Vec<Location> {
        snapshot
            .title_citations(&decl.query_id, &decl.section_separator)
            .into_iter()
            .filter_map(|citation| {
                Some(Location {
                    uri: path_uri(&citation.path)?,
                    range: citation_range(citation, self),
                })
            })
            .collect()
    }

    /// Mark the citation, declaration, section, or stub token under the cursor
    /// as one span, plus every same-ID occurrence in the same document, so an
    /// editor that would otherwise fall back to its word pattern boxes the whole
    /// `§<ID>` token instead of a single sub-word (§FS-lsp.1.3.3).
    fn document_highlights(&self, params: Value) -> Result<Option<Vec<DocumentHighlight>>> {
        let params: TextDocumentPositionParams = serde_json::from_value(params)?;
        let uri = &params.text_document.uri;
        let Some(path) = uri.to_file_path().ok().map(normalize_path) else {
            return Ok(None);
        };
        let Some((snapshot, token)) = self.token_at(uri, params.position) else {
            return Ok(None);
        };
        // The token under the cursor is always highlighted; the sibling pass
        // below may re-add its range, which the dedup at the end collapses.
        let mut ranges = vec![token.range(self)];
        match token {
            Token::Citation(source) => {
                for citation in &snapshot.citations {
                    if same_path(&citation.path, &path)
                        && citation_under_title(
                            &source.declaration_query_id,
                            &citation.query_id,
                            &source.section_separator,
                        )
                    {
                        ranges.push(citation_range(citation, self));
                    }
                }
                for decl in &snapshot.declarations {
                    if same_path(&decl.path, &path) && decl.query_id == source.declaration_query_id
                    {
                        ranges.push(declaration_range(decl, self));
                    }
                }
            }
            Token::Declaration(decl) => {
                for citation in &snapshot.citations {
                    if same_path(&citation.path, &path)
                        && citation_under_title(
                            &decl.query_id,
                            &citation.query_id,
                            &decl.section_separator,
                        )
                    {
                        ranges.push(citation_range(citation, self));
                    }
                }
            }
            Token::Stub(stub) => {
                for citation in &snapshot.citations {
                    if same_path(&citation.path, &path)
                        && citation_under_title(
                            &stub.query_id,
                            &citation.query_id,
                            &stub.section_separator,
                        )
                    {
                        ranges.push(citation_range(citation, self));
                    }
                }
            }
        }
        ranges.sort_by_key(|range| {
            (
                range.start.line,
                range.start.character,
                range.end.line,
                range.end.character,
            )
        });
        ranges.dedup();
        Ok(Some(
            ranges
                .into_iter()
                .map(|range| DocumentHighlight {
                    range,
                    kind: Some(DocumentHighlightKind::TEXT),
                })
                .collect(),
        ))
    }

    fn document_links(&self, params: Value) -> Result<Option<Vec<DocumentLink>>> {
        let params: lsp_types::DocumentLinkParams = serde_json::from_value(params)?;
        let Some(path) = params
            .text_document
            .uri
            .to_file_path()
            .ok()
            .map(normalize_path)
        else {
            return Ok(Some(Vec::new()));
        };
        let Some(snapshot) = self.snapshot_for_path(&path) else {
            return Ok(Some(Vec::new()));
        };
        let mut links = snapshot
            .citations
            .iter()
            .filter(|citation| normalize_path(&citation.path) == path)
            .filter_map(|citation| {
                let location = self.citation_location(snapshot, citation)?;
                Some(DocumentLink {
                    range: citation_range(citation, self),
                    target: document_link_target(citation).or(Some(location.uri)),
                    tooltip: Some(format!("Open {}", citation.query_id)),
                    data: None,
                })
            })
            .collect::<Vec<_>>();
        // Ordinary Markdown declaration titles are deliberately not document
        // links: a self-pointing link would shadow the editor's Ctrl-click and
        // navigate the title onto its own line, hiding the declaration's usages.
        // The title stays navigable through go-to-definition, which returns the
        // citation sites (§FS-lsp.1.3.2). Stub titles below still link, because
        // they point at the source declaration, not at themselves.
        links.extend(
            snapshot
                .stubs
                .iter()
                .filter(|stub| normalize_path(&stub.path) == path)
                .map(|stub| DocumentLink {
                    range: stub_range(stub, self),
                    target: stub_document_link_target(stub),
                    tooltip: Some(format!("Open {}", stub.query_id)),
                    data: None,
                }),
        );
        Ok(Some(links))
    }

    fn on_type_formatting(&self, params: Value) -> Result<Option<Vec<TextEdit>>> {
        let params: DocumentOnTypeFormattingParams = serde_json::from_value(params)?;
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        // The whole document, not just the edited line: the shorthand rewrite has
        // to know whether the line sits inside a fenced block, exactly as
        // `grund fmt` does (§FS-lsp.1.4).
        let Some(text) = self.document_text(&uri) else {
            return Ok(Some(Vec::new()));
        };
        let Some(path) = uri.to_file_path().ok() else {
            return Ok(Some(Vec::new()));
        };
        let declared_ids = self
            .snapshot_for_path(&path)
            .map(Self::declared_ids)
            .unwrap_or_default();
        on_type_replacement_for_line(&path, &text, position, &declared_ids)
    }

    /// The edited document's full text — the open-buffer copy when the client has
    /// one, else the file on disk, the same fallback `line_text` uses.
    fn document_text(&self, uri: &Url) -> Option<String> {
        if let Some(text) = self.open_docs.get(uri) {
            return Some(text.clone());
        }
        fs::read_to_string(uri.to_file_path().ok()?).ok()
    }

    /// The declarations in the session snapshot, for shorthand expansion in the
    /// live transform (§FS-lsp.1.4). Section titles are excluded — only whole-ID
    /// declarations can be what a shorthand names.
    ///
    /// `query_id` carries an `<alias>/` prefix in workspace mode, which no `[id]
    /// format` can parse; the bare ID is what a shorthand is matched against, and
    /// the declaration's path is what scopes it to the edited file's own member.
    /// Borrowed, not cloned: this runs on every keystroke.
    fn declared_ids(snapshot: &LspSnapshot) -> Vec<DeclaredId<'_>> {
        snapshot
            .declarations
            .iter()
            .map(|decl| DeclaredId {
                path: decl.path.as_path(),
                id: decl
                    .project
                    .as_deref()
                    .and_then(|alias| decl.query_id.strip_prefix(alias)?.strip_prefix('/'))
                    .unwrap_or(&decl.query_id),
            })
            .collect()
    }

    fn publish_diagnostics(&mut self) -> Result<()> {
        let mut by_uri: BTreeMap<Url, Vec<Diagnostic>> = BTreeMap::new();
        for project in &self.projects {
            for finding in project.snapshot.report.errors.clone() {
                if let Some((uri, diagnostic)) = self.diagnostic_for_finding(
                    &project.snapshot,
                    finding,
                    DiagnosticSeverity::ERROR,
                ) {
                    by_uri.entry(uri).or_default().push(diagnostic);
                }
            }
            for finding in project.snapshot.report.warnings.clone() {
                if let Some((uri, diagnostic)) = self.diagnostic_for_finding(
                    &project.snapshot,
                    finding,
                    DiagnosticSeverity::WARNING,
                ) {
                    by_uri.entry(uri).or_default().push(diagnostic);
                }
            }
        }
        let next_diagnostic_uris: BTreeSet<Url> = by_uri.keys().cloned().collect();
        for uri in self.diagnostic_uris.difference(&next_diagnostic_uris) {
            self.connection
                .sender
                .send(Message::Notification(Notification::new(
                    "textDocument/publishDiagnostics".to_string(),
                    PublishDiagnosticsParams {
                        uri: uri.clone(),
                        diagnostics: Vec::new(),
                        version: None,
                    },
                )))?;
        }
        self.diagnostic_uris = next_diagnostic_uris;
        for (uri, diagnostics) in by_uri {
            self.connection
                .sender
                .send(Message::Notification(Notification::new(
                    "textDocument/publishDiagnostics".to_string(),
                    PublishDiagnosticsParams {
                        uri,
                        diagnostics,
                        version: None,
                    },
                )))?;
        }
        Ok(())
    }

    fn diagnostic_for_finding(
        &self,
        snapshot: &LspSnapshot,
        finding: Finding,
        severity: DiagnosticSeverity,
    ) -> Option<(Url, Diagnostic)> {
        let path = finding.path.as_deref()?;
        let path = if Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            snapshot.root.join(path)
        };
        let uri = path_uri(&path)?;
        let line = finding.line.unwrap_or(1).saturating_sub(1) as u32;
        let range = self
            .range_for_finding(snapshot, &path, &finding)
            .unwrap_or(Range {
                start: Position { line, character: 0 },
                end: Position { line, character: 1 },
            });
        Some((
            uri,
            Diagnostic {
                range,
                severity: Some(severity),
                code: Some(lsp_types::NumberOrString::String(finding.code.to_string())),
                source: Some("grund".to_string()),
                message: finding.message,
                ..Diagnostic::default()
            },
        ))
    }

    fn range_for_finding(
        &self,
        snapshot: &LspSnapshot,
        path: &Path,
        finding: &Finding,
    ) -> Option<Range> {
        let line = finding.line?;
        // When the finding carries the offending citation's column, anchor on
        // that exact token rather than the first citation on the line — a single
        // comment can carry several citations (§FS-lsp.1.1).
        if let Some(column) = finding.column
            && let Some(citation) = snapshot.citations.iter().find(|citation| {
                same_path(&citation.path, path)
                    && citation.line == line
                    && citation.column == column
            })
        {
            return Some(citation_range(citation, self));
        }
        // Line-anchored diagnostics must not borrow the first citation on their
        // line. In VSCode, diagnostic hovers include every diagnostic whose
        // range overlaps the cursor; mapping an ungrounded-file error to a
        // dangling citation would make the citation hover show two messages
        // even though only one diagnostic belongs to that token (§FS-lsp.1.1).
        snapshot
            .declarations
            .iter()
            .find(|decl| same_path(&decl.path, path) && decl.line == line)
            .map(|decl| declaration_range(decl, self))
            .or_else(|| {
                snapshot
                    .sections
                    .iter()
                    .find(|section| same_path(&section.path, path) && section.line == line)
                    .map(|section| declaration_range(section, self))
            })
            .or_else(|| {
                snapshot
                    .stubs
                    .iter()
                    .find(|stub| same_path(&stub.path, path) && stub.line == line)
                    .map(|stub| stub_range(stub, self))
            })
    }

    fn token_at(&self, uri: &Url, position: Position) -> Option<(&LspSnapshot, Token<'_>)> {
        let path = uri.to_file_path().ok().map(normalize_path)?;
        let snapshot = self.snapshot_for_path(&path)?;
        let token = snapshot
            .citations
            .iter()
            .find(|citation| {
                same_path(&citation.path, &path)
                    && contains(citation_range(citation, self), position)
            })
            .map(Token::Citation)
            .or_else(|| {
                snapshot
                    .declarations
                    .iter()
                    .find(|decl| {
                        same_path(&decl.path, &path)
                            && contains(declaration_range(decl, self), position)
                    })
                    .map(Token::Declaration)
            })
            .or_else(|| {
                // A numbered section heading is a declaration-side title too, so
                // definition and references resolve to its section citations
                // (§FS-lsp.1.3.1).
                snapshot
                    .sections
                    .iter()
                    .find(|decl| {
                        same_path(&decl.path, &path)
                            && contains(declaration_range(decl, self), position)
                    })
                    .map(Token::Declaration)
            })
            .or_else(|| {
                snapshot
                    .stubs
                    .iter()
                    .find(|stub| {
                        same_path(&stub.path, &path) && contains(stub_range(stub, self), position)
                    })
                    .map(Token::Stub)
            })?;
        Some((snapshot, token))
    }

    fn citation_location(
        &self,
        snapshot: &LspSnapshot,
        citation: &LspCitation,
    ) -> Option<Location> {
        let target_path = citation.target_path.as_ref()?;
        let target_line = citation.target_line?;
        Some(Location {
            uri: path_uri(target_path)?,
            range: self.definition_target_range(
                snapshot,
                target_path,
                target_line,
                citation.query_id.len().max(1),
            ),
        })
    }

    fn declaration_location(&self, decl: &LspDeclaration) -> Option<Location> {
        Some(Location {
            uri: path_uri(&decl.path)?,
            range: declaration_range(decl, self),
        })
    }

    fn stub_target_location(&self, snapshot: &LspSnapshot, stub: &LspStub) -> Option<Location> {
        Some(Location {
            uri: path_uri(&stub.target_path)?,
            range: self.definition_target_range(
                snapshot,
                &stub.target_path,
                stub.target_line,
                stub.query_id.len().max(1),
            ),
        })
    }

    fn definition_target_range(
        &self,
        snapshot: &LspSnapshot,
        path: &Path,
        line: usize,
        fallback_width: usize,
    ) -> Range {
        snapshot
            .declarations
            .iter()
            .find(|decl| same_path(&decl.path, path) && decl.line == line)
            .map(|decl| declaration_range(decl, self))
            .or_else(|| {
                snapshot
                    .sections
                    .iter()
                    .find(|section| same_path(&section.path, path) && section.line == line)
                    .map(|section| declaration_range(section, self))
            })
            .unwrap_or_else(|| single_line_range(path, line, 1, fallback_width))
    }

    fn snapshot_for_path(&self, path: &Path) -> Option<&LspSnapshot> {
        let path = canonical_snapshot_path(path);
        self.projects
            .iter()
            .filter(|project| path.starts_with(&project.root))
            .max_by_key(|project| project.root.components().count())
            .map(|project| &project.snapshot)
    }

    fn open_document_overlays(&self) -> BTreeMap<PathBuf, String> {
        self.open_docs
            .iter()
            .filter_map(|(uri, text)| uri.to_file_path().ok().map(|path| (path, text.clone())))
            .collect()
    }

    /// The full text of `path` — the live editor buffer if the document is open,
    /// otherwise the on-disk contents. Matches `line_text`'s lookup so cached and
    /// uncached reads agree.
    fn file_text(&self, path: &Path) -> Option<String> {
        if let Some(uri) = path_uri(path)
            && let Some(text) = self.open_docs.get(&uri)
        {
            return Some(text.clone());
        }
        fs::read_to_string(path).ok()
    }

    /// One line of `path` (1-based), reading and splitting the file at most once
    /// per snapshot via `line_cache` so token-range computations over many
    /// citations on one file do not re-read it each time (§AR-lsp.5).
    fn cached_line(&self, path: &Path, line: usize) -> Option<String> {
        let key = normalize_path(path);
        let mut cache = self.line_cache.borrow_mut();
        if !cache.contains_key(&key) {
            let lines = self
                .file_text(path)
                .map(|text| text.lines().map(str::to_string).collect())
                .unwrap_or_default();
            cache.insert(key.clone(), lines);
        }
        cache
            .get(&key)
            .and_then(|lines| lines.get(line.saturating_sub(1)).cloned())
    }
}

enum Token<'a> {
    Citation(&'a LspCitation),
    Declaration(&'a LspDeclaration),
    Stub(&'a LspStub),
}

impl<'a> Token<'a> {
    fn range(&self, server: &Server) -> Range {
        match self {
            Token::Citation(citation) => citation_range(citation, server),
            Token::Declaration(decl) => declaration_range(decl, server),
            Token::Stub(stub) => stub_range(stub, server),
        }
    }
}

fn full_change_text(changes: Vec<TextDocumentContentChangeEvent>) -> Option<String> {
    changes.into_iter().last().map(|change| change.text)
}

fn to_value<T: serde::Serialize>(value: T) -> Result<Value> {
    Ok(serde_json::to_value(value)?)
}

fn location_link(origin: Range, location: Location) -> LocationLink {
    LocationLink {
        origin_selection_range: Some(origin),
        target_uri: location.uri,
        target_range: location.range,
        target_selection_range: location.range,
    }
}

fn title_hover(range: Range, text: &str, usage: LspUsage) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: lsp_title_hover_body(text, usage),
        }),
        range: Some(range),
    }
}

fn citation_range(citation: &LspCitation, server: &Server) -> Range {
    token_range(
        server,
        &citation.path,
        citation.line,
        citation.column,
        &citation.text,
    )
}

fn declaration_range(decl: &LspDeclaration, server: &Server) -> Range {
    token_range(server, &decl.path, decl.line, decl.column, &decl.text)
}

fn stub_range(stub: &LspStub, server: &Server) -> Range {
    token_range(server, &stub.path, stub.line, stub.column, &stub.text)
}

fn token_range(server: &Server, path: &Path, line: usize, column: usize, text: &str) -> Range {
    let zero_line = line.saturating_sub(1) as u32;
    let line_text = server.cached_line(path, line);
    if let Some(line_text) = line_text {
        let start_byte = column.saturating_sub(1).min(line_text.len());
        let end_byte = start_byte.saturating_add(text.len()).min(line_text.len());
        return Range {
            start: Position {
                line: zero_line,
                character: byte_to_utf16(&line_text, start_byte),
            },
            end: Position {
                line: zero_line,
                character: byte_to_utf16(&line_text, end_byte),
            },
        };
    }
    Range {
        start: Position {
            line: zero_line,
            character: column.saturating_sub(1) as u32,
        },
        end: Position {
            line: zero_line,
            character: column.saturating_sub(1).saturating_add(text.len()) as u32,
        },
    }
}

fn single_line_range(path: &Path, line: usize, column: usize, width: usize) -> Range {
    let zero_line = line.saturating_sub(1) as u32;
    let line_text = fs::read_to_string(path)
        .ok()
        .and_then(|body| body.lines().nth(line.saturating_sub(1)).map(str::to_string));
    if let Some(line_text) = line_text {
        let start_byte = column.saturating_sub(1).min(line_text.len());
        let end_byte = start_byte.saturating_add(width).min(line_text.len());
        return Range {
            start: Position {
                line: zero_line,
                character: byte_to_utf16(&line_text, start_byte),
            },
            end: Position {
                line: zero_line,
                character: byte_to_utf16(&line_text, end_byte),
            },
        };
    }
    Range {
        start: Position {
            line: zero_line,
            character: column.saturating_sub(1) as u32,
        },
        end: Position {
            line: zero_line,
            character: column.saturating_sub(1).saturating_add(width) as u32,
        },
    }
}

fn contains(range: Range, position: Position) -> bool {
    if position.line != range.start.line || position.line != range.end.line {
        return false;
    }
    position.character >= range.start.character && position.character < range.end.character
}

fn byte_to_utf16(line: &str, byte_idx: usize) -> u32 {
    line[..byte_idx.min(line.len())]
        .chars()
        .map(|ch| ch.len_utf16() as u32)
        .sum()
}

fn utf16_to_byte(line: &str, utf16_idx: u32) -> usize {
    let mut units = 0;
    for (idx, ch) in line.char_indices() {
        if units >= utf16_idx {
            return idx;
        }
        units += ch.len_utf16() as u32;
    }
    line.len()
}

fn same_path(left: &Path, right: &Path) -> bool {
    normalize_path(left) == normalize_path(right)
}

fn normalize_path(path: impl AsRef<Path>) -> PathBuf {
    // Match an incoming request URI against snapshot paths using the exact
    // absolutization `grund-core` applied when it built the snapshot, so the
    // two cannot drift (§AR-lsp.5).
    canonical_snapshot_path(path.as_ref())
}

fn path_uri(path: &Path) -> Option<Url> {
    Url::from_file_path(path).ok()
}

fn document_link_target(citation: &LspCitation) -> Option<Url> {
    let mut uri = path_uri(citation.target_path.as_ref()?)?;
    uri.set_fragment(Some(&format!("L{}", citation.target_line?)));
    Some(uri)
}

fn stub_document_link_target(stub: &LspStub) -> Option<Url> {
    let mut uri = path_uri(&stub.target_path)?;
    uri.set_fragment(Some(&format!("L{}", stub.target_line)));
    Some(uri)
}

fn file_uri_with_line(path: &Path, line: usize) -> Option<String> {
    let mut uri = path_uri(path)?;
    uri.set_fragment(Some(&format!("L{line}")));
    Some(uri.to_string())
}

fn linkify_hover_body(body: &str, citations: &[LspCitation]) -> String {
    let mut links = Vec::new();
    for citation in citations {
        if let (Some(path), Some(line)) = (&citation.target_path, citation.target_line)
            && let Some(uri) = file_uri_with_line(path, line)
        {
            links.push((citation.text.clone(), uri));
        }
    }
    links.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then_with(|| a.0.cmp(&b.0)));
    links.dedup_by(|a, b| a.0 == b.0);
    let mut out = body.to_string();
    for (token, uri) in links {
        out = replace_unlinked_token(&out, &token, &uri);
    }
    out
}

fn replace_unlinked_token(body: &str, token: &str, uri: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(idx) = rest.find(token) {
        let before = &rest[..idx];
        out.push_str(before);
        let prev = before.chars().last().or_else(|| out.chars().last());
        if prev == Some('[') {
            out.push_str(token);
        } else {
            out.push('[');
            out.push_str(token);
            out.push_str("](");
            out.push_str(uri);
            out.push(')');
        }
        rest = &rest[idx + token.len()..];
    }
    out.push_str(rest);
    out
}

fn on_type_replacement_for_line(
    path: &Path,
    text: &str,
    position: Position,
    declarations: &[DeclaredId<'_>],
) -> Result<Option<Vec<TextEdit>>> {
    let line_index = position.line as usize;
    let Some(line) = text.lines().nth(line_index) else {
        return Ok(Some(Vec::new()));
    };
    let cursor = utf16_to_byte(line, position.character);
    // One config resolution per keystroke: the core helper resolves the edited
    // file's marker/trigger and the fmt-context exclusions together (§FS-lsp.1.4).
    // It returns the whole edit set already ordered, so the two rewrites it can
    // produce — trigger→marker and shorthand→canonical — need no reconciling here.
    let at = |offset| Position {
        line: position.line,
        character: byte_to_utf16(line, offset),
    };
    Ok(Some(
        on_type_line_edits(path, text, line_index, cursor, declarations)?
            .into_iter()
            .map(|edit| TextEdit {
                range: Range {
                    start: at(edit.start),
                    end: at(edit.end),
                },
                new_text: edit.text,
            })
            .collect(),
    ))
}

#[cfg(test)]
mod tests;
