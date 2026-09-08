/// One input coordinate for the CLI-only batch-show adapter
/// (§FS-show.1, §FS-show.2.6).
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchShowQuery {
    pub id: String,
    pub section: Option<String>,
}

/// A query failure carried inside a batch envelope so one bad coordinate does
/// not stop the rest (§FS-output-shapes.4.1).
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchShowFailure {
    pub code: &'static str,
    pub message: String,
    pub sites: Vec<FindingSite>,
}

/// The rendered single-show JSON object or its per-query failure, paired with
/// the spelling batch output must preserve (§FS-output-shapes.4.1).
#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct BatchShowRecord {
    pub query: BatchShowQuery,
    pub result: std::result::Result<String, BatchShowFailure>,
}

/// Run an explicit query list (`Some`) or exhaustive discovery (`None`) against
/// one shared workspace context. This is intentionally a CLI adapter rather
/// than a replacement for the stable one-query core API (§FS-show.2.6).
#[doc(hidden)]
pub fn show_batch_with_scope(
    queries: Option<Vec<BatchShowQuery>>,
    mut opts: ShowOpts,
    path_provided: bool,
) -> Result<Vec<BatchShowRecord>> {
    if queries.as_ref().is_some_and(Vec::is_empty) {
        return Ok(Vec::new());
    }

    // §AR-workspace.8: this is the batch's only loader entry. Exhaustive
    // discovery reads the returned catalog and never starts a preliminary scan.
    let context = load_workspace_context(&opts.path, path_provided)?;
    if let Some((file, message)) = context
        .projects
        .iter()
        .find_map(|project| project.scan_errors.first())
    {
        return Err(anyhow!(
            "{}: {}",
            display_path(context.render_config(), file),
            message
        ));
    }
    let queries = queries.unwrap_or_else(|| exhaustive_batch_queries(&context));
    opts.format = ShowFormat::Json;

    queries
        .into_iter()
        .map(|query| {
            let query_opts = ShowOpts {
                section: query.section.clone(),
                ..opts.clone()
            };
            let result = match show_batch_query_in_context(
                &context,
                &query.id,
                query_opts,
                &TextOverlays::new(),
            ) {
                Ok(output) => Ok(output.json.unwrap_or_default()),
                Err(error) => match batch_query_failure(&error) {
                    Some(failure) => Err(failure),
                    None => return Err(error),
                },
            };
            Ok(BatchShowRecord { query, result })
        })
        .collect()
}

/// Resolve one batch coordinate through the shared context and the same lower
/// level resolver, body extractor, flattening, and JSON renderer as single show
/// (§FS-show.2.6, §AR-workspace.8).
fn show_batch_query_in_context(
    context: &WorkspaceContext,
    id_arg: &str,
    opts: ShowOpts,
    overlays: &TextOverlays,
) -> Result<ShowOutput> {
    let (alias, raw_id) = split_qualified_id_arg(id_arg)?;
    let project = match alias.as_deref() {
        Some(name) => context.project_by_alias(name).ok_or_else(|| {
            if !context.workspace_loaded {
                anyhow!(
                    "unknown project alias `{name}`\nnote: workspace aliases are defined in the root grund.toml under [workspace]"
                )
            } else {
                anyhow!(
                    "unknown project alias `{name}`\nknown aliases: {}",
                    context.aliases().join(", ")
                )
            }
        })?,
        None => context.current_project().ok_or_else(|| {
            let known = context.aliases().join(", ");
            if known.is_empty() {
                anyhow!("unqualified ID requires a project alias when include_root = false")
            } else {
                anyhow!(
                    "unqualified ID requires a project alias when include_root = false\nknown aliases: {known}"
                )
            }
        })?,
    };
    let config = &project.config;
    let (id, inline_section) =
        resolve_id_arg(raw_id, config, &project.findings).map_err(|error| anyhow!("{error}"))?;
    if opts.section.is_some() && inline_section.is_some() {
        return Err(anyhow!(
            "--section cannot be combined with an inline section"
        ));
    }
    let section = opts.section.or(inline_section);
    let mut output = show_declaration_with_overlays(
        config,
        context.render_config(),
        &project.findings,
        &id,
        section.as_deref(),
        opts.mode.render_mode(),
        false,
        overlays,
    )
    .map_err(|error| with_member_id_candidates(error, context, alias.as_deref(), raw_id))?;
    output.body = flatten_cross_ref_links(&output.body, config);
    output.json = Some(render_show_output_json(
        config,
        context.render_config(),
        &id,
        section.as_deref(),
        opts.mode.render_mode(),
        &output,
    ));
    Ok(output)
}

/// Generate stable coordinates from the declarations and sections already in
/// the loaded context (§FS-show.2.6). BTree sets collapse duplicate claimants;
/// the final bytewise sort is over the public qualified spelling.
fn exhaustive_batch_queries(context: &WorkspaceContext) -> Vec<BatchShowQuery> {
    let mut queries = Vec::new();
    for (project_index, project) in context.projects.iter().enumerate() {
        for (id, declarations) in &project.findings.declarations {
            let local_id = render_id(&project.config, id);
            let qualified_id = if context.current == Some(project_index) {
                local_id
            } else {
                format!("{}/{}", project.alias, local_id)
            };
            queries.push(BatchShowQuery {
                id: qualified_id.clone(),
                section: None,
            });
            let sections: BTreeSet<&str> = declarations
                .iter()
                .flat_map(|declaration| {
                    declaration
                        .sections
                        .keys()
                        .map(String::as_str)
                        .chain(
                            declaration
                                .duplicate_sections
                                .iter()
                                .map(|(path, _)| path.as_str()),
                        )
                })
                .collect();
            queries.extend(sections.into_iter().map(|section| BatchShowQuery {
                id: qualified_id.clone(),
                section: Some(section.to_string()),
            }));
        }
    }
    queries.sort_by(|left, right| {
        left.id
            .as_bytes()
            .cmp(right.id.as_bytes())
            .then_with(|| left.section.as_deref().cmp(&right.section.as_deref()))
    });
    queries
}

/// Convert only coordinate-level refusals into envelopes. An unexpected body
/// read or other operational error remains a run-level abort (§FS-show.2.6).
fn batch_query_failure(error: &anyhow::Error) -> Option<BatchShowFailure> {
    if let Some(carrier) = error.downcast_ref::<ShowQueryError>() {
        return Some(BatchShowFailure {
            code: carrier.code,
            message: carrier.message.clone(),
            sites: carrier.sites.clone(),
        });
    }
    let message = format!("{error:#}");
    let code = if message.starts_with("ID not found:") {
        "not-found"
    } else if message.starts_with("ambiguous ID:") {
        "ambiguous"
    } else if message.starts_with("section not found:") {
        "missing-section"
    } else if message.starts_with("invalid ID") {
        "invalid-id"
    } else if message.starts_with("broken stub:") {
        "broken-stub"
    } else if message.starts_with("unknown project alias") {
        "unknown-project"
    } else if message.starts_with("invalid project alias")
        || message.starts_with("unqualified ID requires a project alias")
        || message == "--section cannot be combined with an inline section"
    {
        "query-failed"
    } else {
        return None;
    };
    Some(BatchShowFailure {
        code,
        message,
        sites: Vec::new(),
    })
}
