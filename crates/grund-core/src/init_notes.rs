// What one `grund init` run reports about the entrypoint layout it found
// (§FS-init.2.2): the `note:` lines, which change nothing and never touch the
// exit code. Both are things a caller would otherwise have to notice for
// itself — an agent reading the same block twice, or reading the wrong form of
// it — and both are only visible from the run that just wrote to those files.

/// The `note:` for a repository whose committed `link` opinion cannot reach
/// Claude, because a Claude entrypoint is a symlink to the canonical file
/// (§FS-init.2.3.4.17). Silence here reads as the opinion not working, on the
/// one agent it was mostly written for.
///
/// `claude_entrypoint` is the real Claude entrypoint this run gives the
/// repository, when it gives it one (§FS-init.2.1.1). The note names the fix,
/// and which fix it is depends on that: with no real entrypoint the run that
/// writes one is the remedy; with one, the remedy is already applied and what
/// is left to remove is the symlink still feeding Claude the plain form beside
/// it. Advising a command that has just run and can do no more would be a note
/// that never retires.
pub(crate) fn shadowed_claude_entrypoint_note(
    root: &Path,
    claude_entrypoint: Option<&str>,
) -> Option<String> {
    let shadowed = claude_entrypoints_shadowed_by_symlink(root);
    if shadowed.is_empty() {
        return None;
    }
    let subject = format_list(
        &shadowed.iter().map(String::as_str).collect::<Vec<_>>(),
        "and",
    );
    let is_are = if shadowed.len() == 1 { "is" } else { "are" };
    Some(match claude_entrypoint {
        None => format!(
            "{subject} {is_are} a symlink to {CANONICAL_AGENT_ENTRYPOINT}, so Claude reads the plain-location form; run `grund init --claude` to write a real Claude entrypoint that teaches the linked form"
        ),
        Some(entrypoint) => format!(
            "{subject} {is_are} a symlink to {CANONICAL_AGENT_ENTRYPOINT}, so Claude reads the plain-location form from there as well as the linked form from {entrypoint}; delete the symlink to leave {entrypoint} as Claude's only entrypoint"
        ),
    })
}

/// Claude entrypoints that are symlinks to the canonical `AGENTS.md`
/// (§FS-init.2.3.4.17). A symlink resolves to the canonical target, so one file
/// carries the block for every agent — and that file is the one Codex reads,
/// where the linked form is recorded as erasing the citation. The committed
/// `link` opinion therefore cannot reach Claude through a symlinked entrypoint.
fn claude_entrypoints_shadowed_by_symlink(root: &Path) -> Vec<String> {
    let canonical = root.join(CANONICAL_AGENT_ENTRYPOINT);
    COMPANION_AGENT_ENTRYPOINTS
        .iter()
        .filter(|entrypoint| entrypoint.agent == Some(AgentEntrypoint::Claude))
        .filter(|entrypoint| {
            let path = root.join(entrypoint.rel);
            is_file_or_symlink(&path) && is_symlink_to(&path, &canonical).unwrap_or(false)
        })
        .map(|entrypoint| entrypoint.rel.to_string())
        .collect()
}

/// One `note:` line per agent whose entrypoints this run wrote the managed
/// block into more than once (§FS-init.2.1.1). `init` no longer creates that
/// second file, but a repository that already carries both keeps feeding the
/// same block to one agent twice, and the run that just wrote to each of them
/// is the only place that is visible. Under `--dry-run` the sentence is in the
/// conditional, like every other line of that report (§FS-init.2.2) — the run
/// wrote nothing, and the files it names may not carry a block yet.
pub(crate) fn duplicate_agent_entrypoint_notes(
    target: &Path,
    entrypoints: &[InitCompanionAgentEntrypoint],
    dry_run: bool,
) -> Vec<String> {
    let written: Vec<&CompanionAgentEntrypoint> = COMPANION_AGENT_ENTRYPOINTS
        .iter()
        .filter(|entrypoint| {
            let path = target.join(entrypoint.rel);
            entrypoints.iter().any(|planned| planned.path() == path)
        })
        .collect();
    let mut notes = Vec::new();
    let mut reported = Vec::new();
    for entrypoint in &written {
        let Some(agent) = entrypoint.agent.filter(|agent| !reported.contains(agent)) else {
            continue;
        };
        let rels: Vec<&str> = written
            .iter()
            .filter(|other| other.agent == Some(agent))
            .map(|other| other.rel)
            .collect();
        if rels.len() < 2 {
            continue;
        }
        reported.push(agent);
        let (carry, reads) = if dry_run {
            ("would both carry", "would read")
        } else {
            ("both carry", "reads")
        };
        notes.push(format!(
            "{} {carry} the managed block, so {} {reads} it twice; delete the one you do not want — `grund init` creates only one",
            format_list(&rels, "and"),
            agent.name(),
        ));
    }
    notes
}
