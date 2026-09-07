/// The explicit external-snapshot materializer (§FS-fetch). This module owns
/// integration execution and declaration-local atomic writes; every scanner and
/// query remains a reader under §REQ-runs-offline.

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
        fetch_operational(format!("fetch integration `{fetch}` for {raw} emitted non-UTF-8 output"))
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
            write_folder_home(&path, &selected.grammar, &id, local, snapshot.as_bytes())
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
                    return Err(fetch_operational("fetch output declaration needs `: <title>`"));
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
                    return Err(fetch_operational("fetch output declaration title is empty"));
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

#[derive(Clone)]
struct SnapshotDeclaration {
    id: Id,
    start: usize,
    end: usize,
    path: Option<PathBuf>,
}

fn declarations_at_depth(
    bytes: &[u8],
    grammar: &Grammar,
    depth: usize,
) -> std::result::Result<Vec<SnapshotDeclaration>, FetchFailure> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| fetch_operational("snapshot home is not UTF-8 Markdown"))?;
    let prefix = format!("{} ", "#".repeat(depth));
    let mut starts = Vec::<(usize, Id)>::new();
    let mut offset = 0usize;
    let mut fence: Option<String> = None;
    for line in text.split_inclusive('\n') {
        let content = line.strip_suffix('\n').unwrap_or(line).strip_suffix('\r').unwrap_or(line.strip_suffix('\n').unwrap_or(line));
        let trimmed = content.trim_start();
        if let Some(open) = &fence {
            if trimmed.starts_with(open) {
                fence = None;
            }
        } else if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fence = Some(trimmed.chars().take(3).collect());
        } else if let Some(tail) = trimmed.strip_prefix(&prefix)
            && let Some((raw, _)) = tail.split_once(':')
            && let Ok((id, None)) = parse_id_arg(raw.trim(), grammar)
        {
            starts.push((offset + content.len() - trimmed.len(), id));
        }
        offset += line.len();
    }
    Ok(starts
        .iter()
        .enumerate()
        .map(|(index, (start, id))| SnapshotDeclaration {
            id: id.clone(),
            start: *start,
            // The blank separator before the next sibling is outside the
            // declaration-local replacement and therefore remains byte-for-byte.
            end: starts.get(index + 1).map_or(bytes.len(), |next| {
                let end = next.0;
                if bytes[..end].ends_with(b"\r\n\r\n") {
                    end - 2
                } else if bytes[..end].ends_with(b"\n\n") {
                    end - 1
                } else {
                    end
                }
            }),
            path: None,
        })
        .collect())
}

/// §FS-fetch.4: replace only the named H2 or insert it in ID order.
fn write_file_home(
    path: &Path,
    grammar: &Grammar,
    requested: &Id,
    snapshot: &[u8],
) -> std::result::Result<(), FetchFailure> {
    let original = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(err) => return Err(fetch_operational(format!("cannot read {}: {err}", path.display()))),
    };
    let declarations = declarations_at_depth(&original, grammar, 2)?;
    let matching = declarations
        .iter()
        .filter(|declaration| &declaration.id == requested)
        .collect::<Vec<_>>();
    if matching.len() > 1 {
        return Err(fetch_operational("snapshot home has multiple declarations for requested ID"));
    }
    let (start, end) = if let Some(existing) = matching.first() {
        (existing.start, existing.end)
    } else {
        let start = declarations
            .iter()
            .find(|declaration| declaration.id > *requested)
            .map_or(original.len(), |declaration| declaration.start);
        (start, start)
    };
    let mut replacement = Vec::with_capacity(original.len() + snapshot.len());
    let line_ending: &[u8] = if original.windows(2).any(|pair| pair == b"\r\n") {
        b"\r\n"
    } else {
        b"\n"
    };
    replacement.extend_from_slice(&original[..start]);
    if !replacement.is_empty()
        && !replacement.ends_with(b"\n\n")
        && !replacement.ends_with(b"\r\n\r\n")
    {
        replacement.extend_from_slice(line_ending);
    }
    replacement.extend_from_slice(snapshot);
    if end < original.len()
        && !original[end..].starts_with(b"\n")
        && !original[end..].starts_with(b"\r\n")
        && !replacement.ends_with(b"\n\n")
        && !replacement.ends_with(b"\r\n\r\n")
    {
        replacement.extend_from_slice(line_ending);
    }
    replacement.extend_from_slice(&original[end..]);
    atomic_install(path, &replacement)
}

/// §FS-fetch.5: replace the unique declaring file or create `<ID>.md`.
fn write_folder_home(
    folder: &Path,
    grammar: &Grammar,
    requested: &Id,
    local: &str,
    snapshot: &[u8],
) -> std::result::Result<(), FetchFailure> {
    let mut matches = Vec::new();
    if folder.exists() {
        let entries = fs::read_dir(folder)
            .map_err(|err| fetch_operational(format!("cannot read {}: {err}", folder.display())))?;
        for entry in entries {
            let path = entry
                .map_err(|err| fetch_operational(format!("cannot read {}: {err}", folder.display())))?
                .path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("md") {
                continue;
            }
            let bytes = fs::read(&path)
                .map_err(|err| fetch_operational(format!("cannot read {}: {err}", path.display())))?;
            let declarations = declarations_at_depth(&bytes, grammar, 1)?;
            let contains_requested = declarations
                .iter()
                .any(|declaration| declaration.id == *requested);
            if contains_requested && declarations.len() != 1 {
                return Err(fetch_operational(format!(
                    "snapshot file {} contains the requested ID beside another declaration",
                    path.display()
                )));
            }
            for mut declaration in declarations {
                if declaration.id == *requested {
                    declaration.path = Some(path.clone());
                    matches.push(declaration);
                }
            }
        }
    }
    if matches.len() > 1 {
        return Err(fetch_operational("snapshot folder has multiple declarations for requested ID"));
    }
    let target = matches
        .first()
        .and_then(|declaration| declaration.path.clone())
        .unwrap_or_else(|| folder.join(format!("{local}.md")));
    if matches.is_empty() && target.exists() {
        return Err(fetch_operational(format!(
            "snapshot file {} already exists for a different ID",
            target.display()
        )));
    }
    atomic_install(&target, snapshot)
}

/// §FS-fetch.4 / §FS-fetch.5: install complete bytes by same-directory rename,
/// leaving an unchanged target untouched.
fn atomic_install(path: &Path, bytes: &[u8]) -> std::result::Result<(), FetchFailure> {
    if fs::read(path).ok().as_deref() == Some(bytes) {
        return Ok(());
    }
    let parent = path
        .parent()
        .ok_or_else(|| fetch_operational(format!("cannot write {}", path.display())))?;
    fs::create_dir_all(parent)
        .map_err(|err| fetch_operational(format!("cannot write {}: {err}", path.display())))?;
    let mut temporary = None;
    for attempt in 0..100u32 {
        let candidate = parent.join(format!(
            ".grund-fetch-{}-{attempt}.tmp",
            std::process::id()
        ));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => {
                temporary = Some((candidate, file));
                break;
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => {
                return Err(fetch_operational(format!("cannot write {}: {err}", path.display())));
            }
        }
    }
    let (temporary_path, mut file) = temporary
        .ok_or_else(|| fetch_operational(format!("cannot create temporary file for {}", path.display())))?;
    use std::io::Write;
    if let Err(err) = file.write_all(bytes).and_then(|_| file.sync_all()) {
        let _ = fs::remove_file(&temporary_path);
        return Err(fetch_operational(format!("cannot write {}: {err}", path.display())));
    }
    drop(file);
    if let Err(err) = fs::rename(&temporary_path, path) {
        let _ = fs::remove_file(&temporary_path);
        return Err(fetch_operational(format!("cannot atomically replace {}: {err}", path.display())));
    }
    Ok(())
}
