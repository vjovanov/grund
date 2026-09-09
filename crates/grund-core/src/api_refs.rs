/// Data-producing implementation behind [`refs_outcome`]: scan once, select the target
/// grammar, and return either citation data or the typed resolver rejection
/// (§AR-bindings.2, §FS-refs.2, §FS-refs.4).
fn refs_impl(opts: RefsOpts) -> Result<RefsOutcome> {
    let context = load_workspace_context(&opts.path, opts.path_provided)?;
    let current_config = context
        .current_project()
        .map(|project| &project.config)
        .unwrap_or_else(|| context.render_config());
    let (alias, raw_id) = split_qualified_id_arg(&opts.id).map_err(|err| {
        anyhow!(
            "{err:#}\nhint: this repo's [id] format is `{}` (run `grund config show`); `grund list` shows the IDs that exist",
            current_config.id_format
        )
    })?;
    let target_project = match alias.as_deref() {
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
    let target_alias = target_project.alias.as_str();
    let render_config = &target_project.config;
    let scan_errors = context
        .projects
        .iter()
        .flat_map(|project| {
            project
                .scan_errors
                .iter()
                .map(|(file, message)| api_scan_error(context.render_config(), file, message))
        })
        .collect::<Vec<_>>();
    // §FS-refs.4: the `[id] format` hint is for an argument that does not match
    // it; an ambiguous shorthand did match and lists its candidates instead.
    let (id, inline_section) = match resolve_id_arg(
        raw_id,
        render_config,
        &target_project.findings,
    ) {
        Ok(resolved) => resolved,
        Err(error) => {
            return Ok(RefsOutcome {
                output: RefsOutput {
                    output_format: render_config.output_format.clone(),
                    workspace: context.workspace_loaded,
                    hits: Vec::new(),
                    note: None,
                    scan_errors,
                },
                query_failure: Some(RefsQueryFailure::from_resolver_error(
                    &error,
                    &render_config.id_format,
                )),
            });
        }
    };
    if opts.section.is_some() && inline_section.is_some() {
        return Err(anyhow!(
            "--section cannot be combined with an inline section"
        ));
    }
    let section = opts.section.or(inline_section);

    struct Hit<'a> {
        project: &'a WorkspaceProject,
        citation: &'a Citation,
    }
    let mut hits = Vec::new();
    for project in &context.projects {
        let is_target = project.alias == target_alias;
        for citation in &project.findings.citations {
            let local_match = citation.namespace.is_none() && is_target;
            let qualified_match = citation
                .namespace
                .as_deref()
                .map(|ns| ns == target_alias)
                .unwrap_or(false);
            if !(local_match || qualified_match) || citation.id != id {
                continue;
            }
            if let Some(expected) = section.as_deref()
                && citation.section.as_deref() != Some(expected)
            {
                continue;
            }
            hits.push(Hit { project, citation });
        }
    }
    hits.sort_by(|a, b| {
        (sort_path_key(&a.citation.file), a.citation.line, a.citation.column).cmp(&(
            sort_path_key(&b.citation.file),
            b.citation.line,
            b.citation.column,
        ))
    });
    let render_path = |project: &WorkspaceProject, path: &Path| -> String {
        if context.workspace_loaded {
            display_path(context.render_config(), path)
        } else {
            display_path(&project.config, path)
        }
    };
    let public_hits = hits
        .iter()
        .map(|hit| RefHit {
            project: context.workspace_loaded.then(|| hit.project.alias.clone()),
            path: render_path(hit.project, &hit.citation.file),
            line: hit.citation.line,
            column: hit.citation.column,
            id: render_id(render_config, &hit.citation.id),
            section: hit.citation.section.clone(),
            marker: hit.citation.has_marker,
            text: hit.citation.text.clone(),
        })
        .collect::<Vec<_>>();
    let note = if public_hits.is_empty() && !target_project.findings.declarations.contains_key(&id)
    {
        if context.workspace_loaded && alias.is_some() {
            Some(format!(
                "{}/{} is neither declared nor cited — run `grund list --project {}` to see {}'s declared IDs",
                target_alias,
                render_id(render_config, &id),
                target_alias,
                target_alias
            ))
        } else {
            Some(format!(
                "{} is neither declared nor cited — run `grund list` to see every declared ID",
                render_id(render_config, &id)
            ))
        }
    } else {
        None
    };
    Ok(RefsOutcome {
        output: RefsOutput {
            output_format: render_config.output_format.clone(),
            workspace: context.workspace_loaded,
            hits: public_hits,
            note,
            scan_errors,
        },
        query_failure: None,
    })
}
