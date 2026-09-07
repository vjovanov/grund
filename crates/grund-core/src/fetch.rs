/// The explicit external-snapshot materializer (§FS-fetch). This module owns
/// integration execution and output validation; every scanner and query remains
/// a reader under §REQ-runs-offline.

/// Which public CLI exit class a fetch refusal belongs to (§FS-fetch.7).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FetchFailureKind {
    Query,
    Operational,
}

/// A located fetch refusal with its fixed exit class (§FS-fetch.7).
#[derive(Debug)]
pub struct FetchFailure {
    pub kind: FetchFailureKind,
    pub message: String,
}

impl std::fmt::Display for FetchFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for FetchFailure {}

fn fetch_query(message: impl Into<String>) -> FetchFailure {
    FetchFailure {
        kind: FetchFailureKind::Query,
        message: message.into(),
    }
}

fn fetch_operational(message: impl Into<String>) -> FetchFailure {
    FetchFailure {
        kind: FetchFailureKind::Operational,
        message: message.into(),
    }
}

/// Materialize exactly one local or qualified external ID (§FS-fetch.1).
pub fn fetch_snapshot(raw: &str, path: &Path) -> std::result::Result<(), FetchFailure> {
    let mut root_config = resolve_workspace_config(path)
        .map_err(|err| fetch_operational(format!("{err:#}")))?;
    let (namespace, local) = raw.rsplit_once('/').map_or((None, raw), |(ns, id)| {
        (Some(ns), id)
    });
    let selected = if let Some(namespace) = namespace {
        if !root_config.workspace_declared {
            return Err(fetch_operational(format!(
                "unknown project alias `{namespace}` for fetch"
            )));
        }
        expand_workspace_tree(&mut root_config)
            .map_err(|err| fetch_operational(format!("{err:#}")))?
            .into_iter()
            .find(|project| project.alias == namespace)
            .map(|project| project.config)
            .ok_or_else(|| {
                fetch_operational(format!("unknown project alias `{namespace}` for fetch"))
            })?
    } else {
        root_config
    };

    let (id, section) = parse_id_arg(local, &selected.grammar)
        .map_err(|err| fetch_query(format!("{err:#}")))?;
    if section.is_some() {
        return Err(fetch_query(format!("invalid ID `{raw}`")));
    }
    let kind = selected
        .kinds
        .iter()
        .find(|kind| kind.kind == id.kind)
        .ok_or_else(|| fetch_query(format!("invalid ID `{raw}`")))?;
    let fetch = kind.fetch.as_deref().ok_or_else(|| {
        fetch_operational(format!("kind `{}` has no fetch integration", kind.kind))
    })?;
    let integration = {
        let configured = Path::new(fetch);
        if configured.is_absolute() {
            configured.to_path_buf()
        } else {
            selected.root.join(configured)
        }
    };

    // §FS-fetch.2: direct argv invocation, inherited environment, no shell and
    // no implicit stdin. Stdout is data and is never forwarded to the caller.
    let output = std::process::Command::new(&integration)
        .arg(local)
        .current_dir(&selected.root)
        .output()
        .map_err(|err| {
            fetch_operational(format!(
                "cannot run fetch integration `{fetch}` for {raw}: {err}"
            ))
        })?;
    if !output.status.success() {
        let status = output
            .status
            .code()
            .map_or_else(|| "signal".to_string(), |code| code.to_string());
        return Err(fetch_operational(format!(
            "fetch integration `{fetch}` for {raw} exited {status}"
        )));
    }
    let snapshot = std::str::from_utf8(&output.stdout).map_err(|_| {
        fetch_operational(format!(
            "fetch integration `{fetch}` for {raw} emitted non-UTF-8 output"
        ))
    })?;

    let (home, depth) = match (&kind.file, &kind.folder) {
        (Some(file), None) => (FetchHome::File(selected.root.join(file)), 2),
        (None, Some(folder)) => (FetchHome::Folder(selected.root.join(folder)), 1),
        _ => {
            return Err(fetch_operational(format!(
                "kind `{}` cannot fetch without exactly one snapshot home",
                kind.kind
            )));
        }
    };
    validate_fetched_declaration(snapshot, local, depth)?;
    match home {
        FetchHome::File(path) => write_file_home(&path, &selected.grammar, &id, snapshot.as_bytes()),
        FetchHome::Folder(path) => {
            write_folder_home(&path, &selected, &id, local, snapshot.as_bytes())
        }
    }
}

enum FetchHome {
    File(PathBuf),
    Folder(PathBuf),
}

/// §FS-fetch.3: validate the complete stdout before any mutation.
fn validate_fetched_declaration(
    text: &str,
    requested: &str,
    native_depth: usize,
) -> std::result::Result<(), FetchFailure> {
    let mut fence: Option<String> = None;
    let mut declaration_count = 0usize;
    let mut first_content = true;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if let Some(open) = &fence {
            if trimmed.starts_with(open) {
                fence = None;
            }
            first_content = false;
            continue;
        }
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fence = Some(trimmed.chars().take(3).collect());
            first_content = false;
            continue;
        }
        let hashes = trimmed.bytes().take_while(|byte| *byte == b'#').count();
        if hashes > 0 && trimmed.as_bytes().get(hashes) == Some(&b' ') {
            if hashes == native_depth {
                let tail = &trimmed[hashes + 1..];
                let Some((id, title)) = tail.split_once(':') else {
                    return Err(fetch_operational(
                        "fetch output declaration needs `: <title>`",
                    ));
                };
                declaration_count += 1;
                if declaration_count != 1 || !first_content {
                    return Err(fetch_operational(
                        "fetch output must contain exactly one declaration and no leading content",
                    ));
                }
                if id.trim() != requested {
                    return Err(fetch_operational(format!(
                        "fetch output declares `{}` instead of requested `{requested}`",
                        id.trim()
                    )));
                }
                if title.trim().is_empty() {
                    return Err(fetch_operational(
                        "fetch output declaration title is empty",
                    ));
                }
            } else if declaration_count == 0 || hashes != native_depth + 1 {
                return Err(fetch_operational(format!(
                    "fetch output heading depth {hashes} is invalid for a depth-{native_depth} declaration"
                )));
            }
        } else if first_content && !line.trim().is_empty() {
            return Err(fetch_operational(
                "fetch output must begin with one Markdown declaration",
            ));
        }
        if !line.trim().is_empty() {
            first_content = false;
        }
    }
    if declaration_count != 1 {
        return Err(fetch_operational(
            "fetch output must contain exactly one Markdown declaration",
        ));
    }
    Ok(())
}
