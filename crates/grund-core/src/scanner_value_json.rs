/// Discovery and one-read enrollment of home-derived JSON value sources
/// (§FS-values.2.2, §AR-scanner.2.1, §AR-scanner.3).

fn scan_value_json_sources(
    config: &Config,
    overlays: &TextOverlays,
    findings: &mut Findings,
    errors: &mut Vec<ScanError>,
) {
    let sources = match value_json_sources(config, overlays) {
        Ok(sources) => sources,
        Err((path, message)) => {
            errors.push((path, message));
            return;
        }
    };
    let mut grouped = BTreeMap::<PathBuf, (Vec<PathBuf>, Vec<&KindConfig>)>::new();
    for (path, kind) in sources {
        let (paths, owners) = grouped.entry(physical_path_key(&path)).or_default();
        if !paths.contains(&path) {
            paths.push(path);
        }
        if !owners.iter().any(|owner| owner.kind == kind.kind) {
            owners.push(kind);
        }
    }
    let mut grouped = grouped.into_values().collect::<Vec<_>>();
    for (paths, owners) in &mut grouped {
        paths.sort_by_key(|path| sort_path_key(path));
        owners.sort_by(|left, right| left.kind.cmp(&right.kind));
    }
    grouped.sort_by_key(|(paths, _)| sort_path_key(&paths[0]));
    for (paths, owners) in grouped {
        let path = &paths[0];
        let text = match paths.iter().find_map(|candidate| overlay_text(overlays, candidate)) {
            Some(text) => text.to_string(),
            None => match fs::read_to_string(path) {
                Ok(text) => text,
                Err(error) => {
                    errors.push((path.clone(), format!("could not read value JSON source: {error}")));
                    continue;
                }
            },
        };
        match JsonReader::parse(&text) {
            Ok(root) => enroll_json_root(config, path, &owners, &text, root, findings),
            Err(message) => errors.push((path.clone(), format!("invalid JSON: {message}"))),
        }
        if owners.len() > 1 {
            // A source owned by two homes is ambiguous even after one read;
            // duplicate each site so the ordinary duplicate rule stays the
            // single winner (§FS-values.2.3, §FS-check.3.3).
            let duplicated = findings
                .declarations
                .iter()
                .flat_map(|(id, declarations)| {
                    declarations
                        .iter()
                        .filter(|declaration| {
                            matches!(declaration.source, DeclarationSource::Json { .. })
                                && paths_same_location(&declaration.file, path)
                        })
                        .cloned()
                        .map(|declaration| (id.clone(), declaration))
                })
                .collect::<Vec<_>>();
            for (id, declaration) in duplicated {
                findings.declarations.entry(id).or_default().push(declaration);
            }
        }
    }
}

fn value_json_sources<'a>(
    config: &'a Config,
    overlays: &TextOverlays,
) -> std::result::Result<Vec<(PathBuf, &'a KindConfig)>, (PathBuf, String)> {
    let mut sources = Vec::new();
    for kind in config.kinds.iter().filter(|kind| kind.values) {
        if let Some(file) = &kind.file {
            let path = normalize_path_lexically(&config.root.join(file));
            if path.extension().and_then(|extension| extension.to_str()) == Some("json") {
                sources.push((path, kind));
            }
            continue;
        }
        let Some(folder) = &kind.folder else { continue };
        let folder = normalize_path_lexically(&config.root.join(folder));
        let entries = fs::read_dir(&folder)
            .map_err(|error| (folder.clone(), format!("read value home: {error}")))?;
        for entry in entries {
            let entry =
                entry.map_err(|error| (folder.clone(), format!("read value home: {error}")))?;
            let path = normalize_path_lexically(&entry.path());
            if path.extension().and_then(|extension| extension.to_str()) == Some("json")
                && path.is_file()
            {
                sources.push((path, kind));
            }
        }
        for path in overlays.keys() {
            if path.parent().is_some_and(|parent| paths_same_location(parent, &folder))
                && path.extension().and_then(|extension| extension.to_str()) == Some("json")
            {
                sources.push((path.clone(), kind));
            }
        }
    }
    sources.sort_by(|(left, left_kind), (right, right_kind)| {
        (sort_path_key(left), &left_kind.kind).cmp(&(sort_path_key(right), &right_kind.kind))
    });
    sources.dedup_by(|left, right| left.0 == right.0 && left.1.kind == right.1.kind);
    Ok(sources)
}

fn enroll_json_root(
    config: &Config,
    path: &Path,
    owners: &[&KindConfig],
    text: &str,
    root: JsonNode,
    findings: &mut Findings,
) {
    let JsonNode::Object(members, span) = root else {
        let span = root.span();
        push_json_invalid(findings, None, path, text, span, "value JSON root must be an object");
        return;
    };
    if members.is_empty() {
        push_json_invalid(findings, None, path, text, span, "value JSON object must not be empty");
    }
    for member in members {
        enroll_json_member(config, path, owners, text, member, findings);
    }
}
