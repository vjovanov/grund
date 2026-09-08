const AGENTS_INIT_COMPATIBILITY_TAIL: &str =
    " — repo maintenance; citation checks still ran; wording changes in grund 0.14.0";

/// Preserve the legacy diagnostic as a contiguous prefix while giving readers
/// the maintenance classification during the two-release migration (§FS-errors.3).
fn agents_init_compatibility_message(legacy: String) -> String {
    format!("{legacy}{AGENTS_INIT_COMPATIBILITY_TAIL}")
}

/// Validate the managed agent-entrypoint blocks (§FS-check.3.5): the begin/end
/// marker pair must be present and intact, and the `vN` version must match this
/// binary — an older `vN` is "run `grund init`" (§FS-init.2.3), a newer one is
/// fatal. `AGENTS.md` is canonical; known companion entrypoints are checked when
/// present and not symlinked to `AGENTS.md`.
fn check_agents_block_version(config: &Config, report: &mut CheckReport) {
    let root = &config.root;
    let canonical = root.join("AGENTS.md");
    let canonical_exists = canonical.exists();
    if canonical_exists {
        check_agent_block_path(config, &canonical, report, true);
    }
    match companion_agent_entrypoints(root) {
        Ok(companions) => {
            for companion in companions {
                check_agent_block_path(config, &companion, report, canonical_exists);
            }
        }
        Err((path, message)) => {
            report.errors.push(Diagnostic {
                code: "io",
                path: Some(path),
                line: Some(1),
                column: None,
                message,
                sites: Vec::new(),
            });
        }
    }
}

/// Checks one agent-entrypoint file's managed `grund init` block: present when
/// required, version supported, and its generated sections still matching config.
///
/// Why the generated sections are compared by re-rendering: rendering is
/// deterministic, so a fresh render is the hash. `\r` is stripped from the block
/// first, because the managed `AGENTS.md` is not pinned to LF in `.gitattributes`,
/// so a Windows checkout has CRLF and would read as drift against the LF render.
///
/// Why the local-conversation sentence is re-rendered per file: flipping
/// `[reference] conversation` without re-running `grund init` must surface as
/// drift, and the sentence also varies by entrypoint — so the comparison derives
/// the surface from the path, the same way `init` chose it.
fn check_agent_block_path(
    config: &Config,
    path: &Path,
    report: &mut CheckReport,
    require_block: bool,
) {
    if !path.exists() {
        return;
    }
    let Ok(text) = fs::read_to_string(path) else {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("agent entrypoint");
        report.errors.push(Diagnostic {
            code: "io",
            path: Some(path.to_path_buf()),
            line: Some(1),
            column: None,
            message: format!("cannot read {file_name}"),
            sites: Vec::new(),
        });
        return;
    };
    let block = match find_agents_block(&text) {
        AgentsBlockLookup::Malformed { message, at } => {
            // §FS-check.3.5 / §FS-init.2.3: broken delimiters are diagnosed at
            // the offending line and never rewritten — `grund init` refuses
            // them too.
            report.errors.push(Diagnostic {
                code: "agents-init",
                path: Some(path.to_path_buf()),
                line: Some(line_for_byte_index(&text, at)),
                column: None,
                message: agents_init_compatibility_message(format!(
                    "malformed grund managed block: {message}"
                )),
                sites: Vec::new(),
            });
            return;
        }
        AgentsBlockLookup::Found(block) => Some(block),
        AgentsBlockLookup::Absent => None,
    };
    if let Some(block) = block {
        let line = line_for_byte_index(&text, block.start);
        if block.version < AGENTS_BLOCK_VERSION {
            report.errors.push(Diagnostic {
                code: "agents-init",
                path: Some(path.to_path_buf()),
                line: Some(line),
                column: None,
                message: agents_init_compatibility_message(format!(
                    "outdated grund init block v{} (run `grund init` to update to v{})",
                    block.version, AGENTS_BLOCK_VERSION
                )),
                sites: Vec::new(),
            });
        } else if block.version > AGENTS_BLOCK_VERSION {
            report.errors.push(Diagnostic {
                code: "agents-init",
                path: Some(path.to_path_buf()),
                line: Some(line),
                column: None,
                message: agents_init_compatibility_message(format!(
                    "unsupported grund init block v{} (this grund supports v{})",
                    block.version, AGENTS_BLOCK_VERSION
                )),
                sites: Vec::new(),
            });
        } else {
            // §FS-check.3.5 / §FS-init.2.3.5: citation directions are generated
            // from `[citations]`, so the version marker alone cannot catch a
            // config edit that left the block stale. Re-render and byte-compare.
            let block_text = text[block.start..block.end].replace('\r', "");
            let generated_sections = [
                (
                    "### Citation directions",
                    citation_directions_section(config),
                    "citation directions",
                ),
                // §FS-init.2.3.6: the local-conversation sentence derives from
                // `[reference] conversation` and varies by entrypoint
                // (§FS-init.2.3.4.17), so drift re-renders for *this* file's surface.
                (
                    "### Clickable citations",
                    clickable_citations_section(config, ConversationSurface::for_entrypoint(path)),
                    "clickable citations",
                ),
            ];
            for (heading, expected, noun) in generated_sections {
                if section_in_block(&block_text, heading) != Some(expected.trim_end()) {
                    report.errors.push(Diagnostic {
                        code: "agents-init",
                        path: Some(path.to_path_buf()),
                        line: Some(line),
                        column: None,
                        message: agents_init_compatibility_message(format!(
                            "stale grund init block: {noun} differ from grund.toml (run `grund init` to refresh)"
                        )),
                        sites: Vec::new(),
                    });
                }
            }
        }
        return;
    }
    if !require_block {
        return;
    }
    report.errors.push(Diagnostic {
        code: "agents-init",
        path: Some(path.to_path_buf()),
        line: Some(1),
        column: None,
        message: agents_init_compatibility_message(format!(
            "missing grund init block v{}",
            AGENTS_BLOCK_VERSION
        )),
        sites: Vec::new(),
    });
}

/// The text of a `heading`-led section inside the managed block, from the heading
/// line to the next heading of any level (or block end), trailing blank lines
/// trimmed. Used to byte-compare config-derived sections against a fresh render
/// (§FS-check.3.5). The boundary is *any* following heading, not just H1/H2, so
/// two adjacent config-derived `###` sections (Citation directions, Clickable
/// citations — §FS-init.2.3.5/2.3.6) do not bleed into each other; neither
/// rendered section contains a `#`-led line, so this cannot cut one short.
fn section_in_block<'a>(block_text: &'a str, heading: &str) -> Option<&'a str> {
    let start = block_text.match_indices(heading).find_map(|(index, _)| {
        let at_line_start = index == 0 || block_text.as_bytes().get(index - 1) == Some(&b'\n');
        let after = index + heading.len();
        let line_ends =
            after == block_text.len() || block_text.as_bytes().get(after) == Some(&b'\n');
        (at_line_start && line_ends).then_some(index)
    })?;
    let section_body_start = start + heading.len();
    // A block-final section is bounded by the `<!-- END GRUND MANAGED BLOCK -->`
    // line, not the block end: the delimiter is the block's frame, never part
    // of a rendered section's body.
    let end = [
        next_heading_offset(block_text, section_body_start),
        AGENTS_BLOCK_END
            .find_at(block_text, section_body_start)
            .map(|m| m.start()),
    ]
    .into_iter()
    .flatten()
    .min()
    .unwrap_or(block_text.len());
    Some(block_text[start..end].trim_end())
}

/// Offset of the next heading line (any `#`-led line) at or after `from`, scanning
/// line by line so a `#` mid-line never counts.
fn next_heading_offset(text: &str, from: usize) -> Option<usize> {
    let mut offset = from;
    for line in text[from..].split_inclusive('\n') {
        if line.trim_start().starts_with('#') {
            return Some(offset);
        }
        offset += line.len();
    }
    None
}

fn line_for_byte_index(text: &str, byte_index: usize) -> usize {
    text[..byte_index]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}
