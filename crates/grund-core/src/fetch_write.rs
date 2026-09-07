// Snapshot-home discovery, declaration ownership, and atomic installation for
// the explicit materializer (§FS-fetch.4, §FS-fetch.5, §REQ-no-data-loss.2).

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
    let mut headings = Vec::<(usize, Option<Id>)>::new();
    let mut offset = 0usize;
    let mut fence = None;
    for line in text.split_inclusive('\n') {
        let without_newline = line.strip_suffix('\n').unwrap_or(line);
        let content = without_newline.strip_suffix('\r').unwrap_or(without_newline);
        let trimmed = content.trim_start();
        if markdown_fence_delimiter(&mut fence, content) {
            offset += line.len();
            continue;
        }
        if fence.is_some() {
            offset += line.len();
            continue;
        }
        let hashes = trimmed.bytes().take_while(|byte| *byte == b'#').count();
        if markdown_heading_level(trimmed) == Some(depth) {
            let start = offset + content.len() - trimmed.len();
            let tail = trimmed[hashes..].trim_start();
            let id = match tail.split_once(':') {
                Some((raw, title)) => match parse_id_arg(raw.trim(), grammar) {
                    Ok((id, None)) if !title.trim().is_empty() => Some(id),
                    Ok((_, None)) => {
                        return Err(fetch_operational(format!(
                            "snapshot home has malformed declaration heading `{}`",
                            tail.trim()
                        )));
                    }
                    _ if near_miss_heading(grammar, trimmed, false, true).is_some() => {
                        return Err(fetch_operational(format!(
                            "snapshot home has malformed declaration heading `{}`",
                            tail.trim()
                        )));
                    }
                    _ => None,
                },
                None if parse_longest_id_prefix(tail, grammar).is_some() => {
                    return Err(fetch_operational(format!(
                        "snapshot home has malformed declaration heading `{}`",
                        tail.trim()
                    )));
                }
                None => None,
            };
            headings.push((start, id));
        }
        offset += line.len();
    }
    Ok(headings
        .iter()
        .enumerate()
        .filter_map(|(index, (start, id))| {
            let id = id.as_ref()?;
            // The blank separator before the next sibling is outside the
            // declaration-local replacement and therefore remains byte-for-byte.
            let end = headings.get(index + 1).map_or(bytes.len(), |next| {
                let end = next.0;
                if bytes[..end].ends_with(b"\r\n\r\n") {
                    end - 2
                } else if bytes[..end].ends_with(b"\n\n") {
                    end - 1
                } else {
                    end
                }
            });
            Some(SnapshotDeclaration {
                id: id.clone(),
                start: *start,
                end,
                path: None,
            })
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
        return Err(fetch_operational(
            "snapshot home has multiple declarations for requested ID",
        ));
    }
    let (start, end) = if let Some(existing) = matching.first() {
        (existing.start, existing.end)
    } else {
        let requested = grammar.render(requested, 3);
        let start = declarations
            .iter()
            .find(|declaration| grammar.render(&declaration.id, 3) > requested)
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
    config: &Config,
    requested: &Id,
    local: &str,
    snapshot: &[u8],
) -> std::result::Result<(), FetchFailure> {
    let mut matches = Vec::new();
    if folder.exists() {
        if !folder.is_dir() {
            return Err(fetch_operational(format!(
                "snapshot folder {} is not a directory",
                folder.display()
            )));
        }
        // §FS-fetch.5: discovery uses the scanner's recursive folder traversal,
        // including its ignore, exclusion, hidden-file, and symlink semantics.
        let walked = walk_scannable_files_reporting(config, Some(folder), true).map_err(|err| {
            fetch_operational(format!(
                "cannot read snapshot folder {}: {err:#}",
                folder.display()
            ))
        })?;
        if let Some((path, message)) = walked.errors.first() {
            return Err(fetch_operational(format!(
                "cannot read snapshot folder {} at {}: {message}",
                folder.display(),
                path.display()
            )));
        }
        for path in walked.files {
            if path.extension().and_then(|extension| extension.to_str()) != Some("md") {
                continue;
            }
            let bytes = fs::read(&path)
                .map_err(|err| fetch_operational(format!("cannot read {}: {err}", path.display())))?;
            let declarations = declarations_at_depth(&bytes, &config.grammar, 1)?;
            let contains_requested = declarations
                .iter()
                .any(|declaration| declaration.id == *requested);
            if contains_requested {
                let declaration = declarations
                    .iter()
                    .find(|declaration| declaration.id == *requested)
                    .expect("contains requested declaration");
                let has_content_before = bytes[..declaration.start]
                    .iter()
                    .any(|byte| !byte.is_ascii_whitespace());
                let has_content_after = bytes[declaration.end..]
                    .iter()
                    .any(|byte| !byte.is_ascii_whitespace());
                if declarations.len() != 1 || has_content_before || has_content_after {
                    return Err(fetch_operational(format!(
                        "snapshot file {} contains the requested ID beside other content",
                        path.display()
                    )));
                }
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
        return Err(fetch_operational(
            "snapshot folder has multiple declarations for requested ID",
        ));
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
    let existing_permissions = match fs::metadata(path) {
        Ok(metadata) => Some(metadata.permissions()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
        Err(err) => {
            return Err(fetch_operational(format!(
                "cannot read metadata for {}: {err}",
                path.display()
            )));
        }
    };
    let parent = path
        .parent()
        .ok_or_else(|| fetch_operational(format!("cannot write {}", path.display())))?;
    let created_directories = create_parent_directories(parent, path)?;
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
                rollback_created_directories(&created_directories);
                return Err(fetch_operational(format!(
                    "cannot write {}: {err}",
                    path.display()
                )));
            }
        }
    }
    let Some((temporary_path, mut file)) = temporary else {
        rollback_created_directories(&created_directories);
        return Err(fetch_operational(format!(
            "cannot create temporary file for {}",
            path.display()
        )));
    };
    use std::io::Write;
    let installed = file.write_all(bytes).and_then(|_| {
        if let Some(permissions) = existing_permissions {
            file.set_permissions(permissions)?;
        }
        file.sync_all()
    });
    if let Err(err) = installed {
        drop(file);
        let _ = fs::remove_file(&temporary_path);
        rollback_created_directories(&created_directories);
        return Err(fetch_operational(format!(
            "cannot write {}: {err}",
            path.display()
        )));
    }
    drop(file);
    if let Err(err) = fs::rename(&temporary_path, path) {
        let _ = fs::remove_file(&temporary_path);
        rollback_created_directories(&created_directories);
        return Err(fetch_operational(format!(
            "cannot atomically replace {}: {err}",
            path.display()
        )));
    }
    Ok(())
}

/// Create only the missing parent chain and remember exactly what this fetch
/// added, so every later failure can restore the prior tree (§FS-fetch.4,
/// §FS-fetch.5, §REQ-no-data-loss.2).
fn create_parent_directories(
    parent: &Path,
    target: &Path,
) -> std::result::Result<Vec<PathBuf>, FetchFailure> {
    let mut missing = Vec::new();
    let mut cursor = parent;
    loop {
        match fs::metadata(cursor) {
            Ok(metadata) if metadata.is_dir() => break,
            Ok(_) => {
                return Err(fetch_operational(format!(
                    "cannot write {}: {} is not a directory",
                    target.display(),
                    cursor.display()
                )));
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                missing.push(cursor.to_path_buf());
                cursor = cursor.parent().ok_or_else(|| {
                    fetch_operational(format!("cannot write {}: {err}", target.display()))
                })?;
            }
            Err(err) => {
                return Err(fetch_operational(format!(
                    "cannot write {}: {err}",
                    target.display()
                )));
            }
        }
    }

    let mut created = Vec::new();
    for directory in missing.into_iter().rev() {
        if let Err(err) = fs::create_dir(&directory) {
            rollback_created_directories(&created);
            return Err(fetch_operational(format!(
                "cannot write {}: {err}",
                target.display()
            )));
        }
        created.push(directory);
    }
    Ok(created)
}

fn rollback_created_directories(created: &[PathBuf]) {
    for directory in created.iter().rev() {
        let _ = fs::remove_dir(directory);
    }
}
