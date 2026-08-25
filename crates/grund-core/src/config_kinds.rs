// The `[[kinds]]` half of the config parser (§FS-config.3.4), in a file of its
// own beside `config_discovery.rs` and `config_cmd.rs` (§AR-core-module-layout.1):
// the per-key reader that fills one `[[kinds]]` entry, and the whole-list
// validation that runs once the file is read. Both are pure functions over the
// parsed entries — the discovery, the section walk, and every other section's
// keys stay in `config.rs`.

/// Read one `key = value` line inside a `[[kinds]]` block into `slot`
/// (§FS-config.3.4). Returns `false` for a key this section does not define, so
/// the caller reports it as an unknown config key (§FS-config.4.3).
fn parse_kinds_key(
    path: &Path,
    line_no: usize,
    key: &str,
    value: &str,
    current_kind: &mut Option<KindConfig>,
) -> Result<bool> {
    match key {
        "prefix" => {
            let prefix = parse_string(path, line_no, value)?;
            // §FS-config.3.2: a kind prefix is the leading component of every ID
            // in its kind, so a `/` here lands in the ID as surely as one in
            // `slug_pattern` does.
            if let Some(message) =
                id_grammar_literal_slash_error(&format!("[[kinds]] prefix `{prefix}`"), &prefix)
            {
                bail_config(path, line_no, message)?;
            }
            if let Some(slot) = current_kind.as_mut() {
                slot.prefix = prefix;
            } else {
                bail_config(
                    path,
                    line_no,
                    "`prefix` outside of [[kinds]] block".to_string(),
                )?;
            }
        }
        "folder" => {
            let folder = parse_string(path, line_no, value)?;
            if let Some(slot) = current_kind.as_mut() {
                slot.folder = Some(folder);
            } else {
                bail_config(
                    path,
                    line_no,
                    "`folder` outside of [[kinds]] block".to_string(),
                )?;
            }
        }
        "file" => {
            let file = parse_string(path, line_no, value)?;
            if let Some(slot) = current_kind.as_mut() {
                slot.file = Some(file);
            } else {
                bail_config(
                    path,
                    line_no,
                    "`file` outside of [[kinds]] block".to_string(),
                )?;
            }
        }
        // §FS-config.3.4: `index` names the file under `folder` that must list
        // the folder's declarations (§FS-check.4.6). A file name or `false`;
        // `true` names no file, so it is rejected rather than read as "the
        // default", which the key's own absence already spells.
        "index" => {
            let index = if value == "false" {
                KindIndex::Disabled
            } else if value == "true" {
                bail_config(
                    path,
                    line_no,
                    "[[kinds]] `index` takes a file name or `false` (omit the key for the default `README.md`)"
                        .to_string(),
                )?;
                unreachable!();
            } else {
                KindIndex::Named(parse_string(path, line_no, value)?)
            };
            if let Some(slot) = current_kind.as_mut() {
                slot.index = index;
            } else {
                bail_config(
                    path,
                    line_no,
                    "`index` outside of [[kinds]] block".to_string(),
                )?;
            }
        }
        "title" => {
            let title = parse_string(path, line_no, value)?;
            if let Some(slot) = current_kind.as_mut() {
                slot.title = Some(title);
            } else {
                bail_config(
                    path,
                    line_no,
                    "`title` outside of [[kinds]] block".to_string(),
                )?;
            }
        }
        _ => return Ok(false),
    }
    Ok(true)
}

/// Every whole-list rule a `[[kinds]]` block has to satisfy (§FS-config.3.4):
/// each entry declares a prefix, `folder` and `file` are exclusive, `index`
/// needs a folder, `code` is reserved, and no prefix is a prefix of another.
/// Applied only when the file declared the block at all — `[[kinds]]` replaces
/// the defaults entirely rather than merging into them.
fn apply_parsed_kinds(path: &Path, parsed_kinds: Vec<KindConfig>, config: &mut Config) -> Result<()> {
    // [[kinds]] replaces defaults entirely, per §FS-config.3.4.
    if parsed_kinds.iter().any(|p| p.prefix.is_empty()) {
        return Err(anyhow!(
            "{}: every [[kinds]] entry must declare a `prefix`",
            format_path(path)
        ));
    }
    if parsed_kinds.is_empty() {
        return Err(anyhow!(
            "{}: at least one [[kinds]] entry must declare a `prefix`",
            format_path(path)
        ));
    }
    // Reject kinds that set both `folder` and `file` — they're mutually
    // exclusive (§FS-config.3.4). A kind is either multi-file (folder) or
    // single-file (file); the schema models the "can always be broken up"
    // transition as swapping one key for the other, not setting both.
    for k in &parsed_kinds {
        if k.folder.is_some() && k.file.is_some() {
            return Err(anyhow!(
                "{}: kind `{}` sets both `folder` and `file` (use one)",
                format_path(path),
                k.prefix
            ));
        }
        // §FS-config.3.4: `index` names a file inside `folder`, so a kind
        // with no folder — a single-file `file` kind, or one with no home at
        // all — has nothing to index and the key is a config error.
        if k.index != KindIndex::Default && k.folder.is_none() {
            return Err(anyhow!(
                "{}: kind `{}` sets `index` without `folder` (only a folder kind has an index)",
                format_path(path),
                k.prefix
            ));
        }
        // §FS-config.3.9.2: `code` is the reserved citing pseudo-kind; it can
        // never be a real declaration kind, so reject it as a `[[kinds]]`
        // prefix to keep that non-collision an invariant.
        if k.prefix == CODE_SOURCE_KIND {
            return Err(anyhow!(
                "{}: `{}` is reserved as the citation-direction pseudo-kind and cannot be a [[kinds]] prefix",
                format_path(path),
                CODE_SOURCE_KIND
            ));
        }
    }
    // Reject kinds whose prefix is itself a prefix of another kind's prefix
    // (§FS-config.3.4 — would make tokenization ambiguous).
    for (i, a) in parsed_kinds.iter().enumerate() {
        for (j, b) in parsed_kinds.iter().enumerate() {
            if i != j
                && a.prefix.len() <= b.prefix.len()
                && b.prefix.starts_with(a.prefix.as_str())
            {
                return Err(anyhow!(
                    "{}: kinds `{}` and `{}` collide (one is a prefix of the other)",
                    format_path(path),
                    a.prefix,
                    b.prefix
                ));
            }
        }
    }
    config.kinds = parsed_kinds;
    Ok(())
}
