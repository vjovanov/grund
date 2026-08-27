/// What `init` did to an existing `AGENTS.md`'s managed block — `appended ` (no
/// block before), `updated ` (a supported block whose bytes changed: an older
/// block upgraded, or a same-version block re-rendered against a changed
/// template or config), or `unchanged` (a supported block already byte-identical
/// to the current render — `init` rewrites nothing, §FS-init.2.2/§FS-init.2.3,
/// and reports it with the `exists ` prefix like any other untouched file).
///
/// What this file holds: reading and rewriting the managed agent-instructions
/// block inside a document that is otherwise the user's (§FS-init.2.3).
/// Locating it, versioning it, migrating the legacy heading-bounded form to the
/// delimited one, and splicing the current render over exactly its bytes are one
/// job with one failure mode — eating content it does not own — which is why it
/// lives apart from the scaffold writing in `init.rs`
/// (§AR-core-module-layout.1). `grund check` reads the same block through
/// `find_agents_block` (§FS-check.3.5), so the locator the checker trusts and
/// the one the writer splices with are the same function.
#[derive(Debug, Eq, PartialEq)]
enum AgentsUpdateResult {
    Appended,
    Updated,
    Unchanged,
}

/// Returns the file event for the canonical entrypoint, or an already formatted
/// I/O message for the caller to surface.
fn write_or_update_canonical_agent_entrypoint(
    target: &Path,
    rel: &str,
    contents: &str,
    block: &str,
    force: bool,
    dry_run: bool,
) -> Result<InitEvent, String> {
    let dest = target.join(rel);
    if !force && dest.exists() {
        match update_agents_block(&dest, block, rel, dry_run) {
            Ok(AgentsUpdateResult::Appended) => Ok(InitEvent {
                verb: verb_appended(dry_run),
                path: rel.to_string(),
            }),
            Ok(AgentsUpdateResult::Updated) => Ok(InitEvent {
                verb: verb_updated(dry_run),
                path: rel.to_string(),
            }),
            Ok(AgentsUpdateResult::Unchanged) => Ok(InitEvent {
                verb: "exists",
                path: rel.to_string(),
            }),
            // Forward slashes on every platform, like report paths
            // (§FS-errors.2.2) — Windows must not leak backslashes.
            Err(err) => Err(format!("update {}: {err}", format_path(&dest))),
        }
    } else {
        if !dry_run
            && let Some(parent) = dest.parent()
            && let Err(err) = fs::create_dir_all(parent)
        {
            return Err(format!("create {}: {err}", parent.display()));
        }
        if !dry_run
            && let Err(err) = fs::write(&dest, contents)
        {
            return Err(format!("write {}: {err}", dest.display()));
        }
        Ok(InitEvent {
            verb: verb_wrote(dry_run),
            path: rel.to_string(),
        })
    }
}

/// Append or update the managed block in an existing agent entrypoint on disk
/// (§FS-init.2.3). A supported block is re-rendered from the current
/// template/config even when the schema version already matches — but when that
/// re-render is byte-identical to what is on disk the file is left untouched
/// (`Unchanged`, reported as `exists `), so re-running `grund init` on an
/// up-to-date repo writes nothing (§FS-init.2.2). Under `--dry-run`, the
/// computed result is returned without writing.
fn update_agents_block(
    dest: &Path,
    block: &str,
    label: &str,
    dry_run: bool,
) -> Result<AgentsUpdateResult> {
    let existing = fs::read_to_string(dest)?;
    let (updated, result) = update_agents_text(&existing, block, label)?;
    if !dry_run && result != AgentsUpdateResult::Unchanged {
        fs::write(dest, updated)?;
    }
    Ok(result)
}

/// The pure string transform behind `update_agents_block`: splice the current
/// managed block into `existing`, preserving everything outside it byte-for-byte
/// — including the block's position and any CRLF endings (§FS-init.2.3.1,
/// §FS-init.2.3.2). Returns `Unchanged` when the splice would reproduce
/// `existing` exactly. A newer-than-supported block is an error.
fn update_agents_text(
    existing: &str,
    block: &str,
    label: &str,
) -> Result<(String, AgentsUpdateResult)> {
    match find_agents_block(existing) {
        // §FS-init.2.3: splicing against broken delimiters risks eating user
        // content, so a malformed block is a hard error and the file is left
        // untouched.
        AgentsBlockLookup::Malformed { message, .. } => {
            Err(anyhow!("malformed grund managed block: {message}"))
        }
        AgentsBlockLookup::Found(existing_block) => {
            if existing_block.version > AGENTS_BLOCK_VERSION {
                return Err(anyhow!(
                    "{label} contains newer grund init block v{}; this binary supports v{}",
                    existing_block.version,
                    AGENTS_BLOCK_VERSION
                ));
            }
            // A legacy H2-bounded block is migrated to the delimited form by
            // this same splice (§FS-init.2.3): the replacement `block` carries
            // the delimiters, and the legacy span is what gets replaced.
            let mut updated = String::with_capacity(existing.len() + block.len());
            updated.push_str(&existing[..existing_block.start]);
            updated.push_str(block);
            updated.push_str(&existing[existing_block.end..]);
            let result = if updated == existing {
                AgentsUpdateResult::Unchanged
            } else {
                AgentsUpdateResult::Updated
            };
            Ok((updated, result))
        }
        AgentsBlockLookup::Absent => {
            let separator = if existing.is_empty() || existing.ends_with("\n\n") {
                ""
            } else if existing.ends_with('\n') {
                "\n"
            } else {
                "\n\n"
            };
            let mut updated =
                String::with_capacity(existing.len() + separator.len() + block.len());
            updated.push_str(existing);
            updated.push_str(separator);
            updated.push_str(block);
            Ok((updated, AgentsUpdateResult::Appended))
        }
    }
}

/// The byte span and `vN` version of the managed block inside an `AGENTS.md`
/// (§FS-init.2.3) — what both `grund init`'s update and `grund check`'s validation
/// (§FS-check.3.5) key off.
struct AgentsBlock {
    start: usize,
    end: usize,
    version: u32,
}

/// What locating the managed block found (§FS-init.2.3): a well-formed block
/// (delimited or legacy), no block at all, or delimiters that are present but
/// broken — which neither `init` nor `check` may splice over.
enum AgentsBlockLookup {
    Found(AgentsBlock),
    Absent,
    /// `message` names the specific defect; `at` is the byte offset of the
    /// offending delimiter line, for line-anchored diagnostics.
    Malformed { message: String, at: usize },
}

/// Locate the managed block in an agent entrypoint (§FS-init.2.3). From v4 the
/// block is bounded by explicit `<!-- BEGIN/END GRUND MANAGED BLOCK -->`
/// delimiter lines; a legacy v3-and-earlier block has no delimiters — its H2
/// marker line (`## Grounding with grund (vN)`) opens it and it runs until the
/// next H1 or H2 (or EOF). Broken delimiters are reported as `Malformed`
/// rather than guessed around (§FS-check.3.5).
fn find_agents_block(text: &str) -> AgentsBlockLookup {
    let begins: Vec<regex::Match<'_>> = AGENTS_BLOCK_BEGIN.find_iter(text).collect();
    let ends: Vec<regex::Match<'_>> = AGENTS_BLOCK_END.find_iter(text).collect();
    if begins.is_empty() && ends.is_empty() {
        return find_legacy_agents_block(text);
    }
    let Some(begin) = begins.first() else {
        return AgentsBlockLookup::Malformed {
            message: "`<!-- END GRUND MANAGED BLOCK -->` without a begin delimiter".to_string(),
            at: ends[0].start(),
        };
    };
    if begins.len() > 1 {
        return AgentsBlockLookup::Malformed {
            message: "duplicate `<!-- BEGIN GRUND MANAGED BLOCK -->`".to_string(),
            at: begins[1].start(),
        };
    }
    if let Some(stray) = ends.iter().find(|end| end.start() < begin.start()) {
        return AgentsBlockLookup::Malformed {
            message: "`<!-- END GRUND MANAGED BLOCK -->` before the begin delimiter".to_string(),
            at: stray.start(),
        };
    }
    let Some(end) = ends.first() else {
        return AgentsBlockLookup::Malformed {
            message: "missing `<!-- END GRUND MANAGED BLOCK -->`".to_string(),
            at: begin.start(),
        };
    };
    if ends.len() > 1 {
        return AgentsBlockLookup::Malformed {
            message: "duplicate `<!-- END GRUND MANAGED BLOCK -->`".to_string(),
            at: ends[1].start(),
        };
    }
    let region = &text[begin.start()..end.end()];
    let Some(version) = AGENTS_BLOCK_H2
        .captures(region)
        .and_then(|caps| caps.name("version")?.as_str().parse::<u32>().ok())
    else {
        return AgentsBlockLookup::Malformed {
            message: "no `## Grounding with grund (vN)` heading between the delimiters"
                .to_string(),
            at: begin.start(),
        };
    };
    // The span owns the END delimiter's line ending, so splicing a freshly
    // rendered block (which ends `… -->\n`) over an on-disk block reproduces
    // the file byte-for-byte and re-runs stay `exists ` (§FS-init.2.3.1).
    let mut span_end = end.end();
    if text.as_bytes().get(span_end) == Some(&b'\n') {
        span_end += 1;
    }
    AgentsBlockLookup::Found(AgentsBlock {
        start: begin.start(),
        end: span_end,
        version,
    })
}

/// The pre-v4 lookup: the H2 marker line opens the block and the next H1/H2 (or
/// EOF) closes it (§FS-init.2.3).
fn find_legacy_agents_block(text: &str) -> AgentsBlockLookup {
    let Some(caps) = AGENTS_BLOCK_H2.captures(text) else {
        return AgentsBlockLookup::Absent;
    };
    let (Some(begin_match), Some(version)) = (
        caps.get(0),
        caps.name("version")
            .and_then(|version| version.as_str().parse::<u32>().ok()),
    ) else {
        return AgentsBlockLookup::Absent;
    };
    let after = begin_match.end();
    let section_end = AGENTS_SECTION_BOUNDARY
        .find_at(text, after)
        .map(|m| m.start())
        .unwrap_or(text.len());
    // Trailing blank lines before the next section are inter-section spacing,
    // not part of the managed body. Trim them back so a re-render of the same
    // content is a no-op (`exists `, §FS-init.2.3.1).
    let mut end = section_end;
    while end > after && text[..end].ends_with("\n\n") {
        end -= 1;
    }
    AgentsBlockLookup::Found(AgentsBlock {
        start: begin_match.start(),
        end,
        version,
    })
}
