// What one `grund init` run reports about the entrypoint layout it found
// (§FS-init.2.2): the `note:` lines, which change nothing and never touch the
// exit code. Both are things a caller would otherwise have to notice for
// itself — an agent reading the same block twice, or reading the wrong form of
// it — and both are only visible from the run that just wrote to those files.
//
// Both are also written in the conditional under `--dry-run` (§FS-init.2.2):
// the run wrote nothing, so a note in the present tense describes a tree that
// does not exist, and an instruction that assumes the write already happened —
// "delete the symlink" — costs the reader the only entrypoint they have.

/// The `note:` for a repository whose committed `link` opinion cannot reach
/// Claude, because a Claude entrypoint is a symlink to the canonical file
/// (§FS-init.2.3.4.17). Silence here reads as the opinion not working, on the
/// one agent it was mostly written for.
///
/// `claude_companions` is this run's plan for the files Claude reads: its first
/// entry is the real entrypoint the run gives the repository, when it gives it
/// one (§FS-init.2.1.1). The note names the fix
/// still open, and which fix that is depends on what the tree leaves possible:
/// write the entrypoint where a path is free, delete the symlink where this run
/// just wrote one, and — where symlinks have taken every path Claude reads —
/// delete one of them, because there is nowhere left for `--claude` to write.
/// Advising a command that has just run and can do no more would be a note that
/// never retires.
pub(crate) fn shadowed_claude_entrypoint_note(
    root: &Path,
    claude_companions: &[String],
    dry_run: bool,
) -> Option<String> {
    let shadowed = claude_entrypoints_shadowed_by_symlink(root);
    if shadowed.is_empty() {
        return None;
    }
    let claude_entrypoint = claude_companions.first().map(String::as_str);
    let subject = format_list(
        &shadowed.iter().map(String::as_str).collect::<Vec<_>>(),
        "and",
    );
    let (is_are, symlinks) = if shadowed.len() == 1 {
        ("is", "a symlink")
    } else {
        ("are", "symlinks")
    };
    let reads = if dry_run { "would read" } else { "reads" };
    let head = format!("{subject} {is_are} {symlinks} to {CANONICAL_AGENT_ENTRYPOINT}, so Claude {reads} the plain-location form");
    Some(match claude_entrypoint {
        Some(entrypoint) if dry_run => format!(
            "{head} from there as well as the linked form from {entrypoint}; once {entrypoint} exists, delete the symlink to leave it as Claude's only entrypoint"
        ),
        Some(entrypoint) => format!(
            "{head} from there as well as the linked form from {entrypoint}; delete the symlink to leave {entrypoint} as Claude's only entrypoint"
        ),
        None if claude_entrypoint_path_is_free(root) => format!(
            "{head}; run `grund init --claude` to write a real Claude entrypoint that teaches the linked form"
        ),
        None => format!(
            "{head}, and the symlinks have taken every path a real Claude entrypoint could go; delete one of them and re-run `grund init --claude`"
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
    claude_entrypoint_rows()
        .filter(|entrypoint| {
            let path = root.join(entrypoint.rel);
            is_file_or_symlink(&path) && is_symlink_to(&path, &canonical).unwrap_or(false)
        })
        .map(|entrypoint| entrypoint.rel.to_string())
        .collect()
}

/// Whether any path Claude reads is still free for `--claude` to write
/// (§FS-init.2.1.1) — what decides whether writing the entrypoint is a fix the
/// user can still reach, or whether a symlink has to go first.
fn claude_entrypoint_path_is_free(root: &Path) -> bool {
    claude_entrypoint_rows()
        .any(|entrypoint| path_missing_without_following_symlinks(&root.join(entrypoint.rel)))
}

fn claude_entrypoint_rows() -> impl Iterator<Item = &'static CompanionAgentEntrypoint> {
    COMPANION_AGENT_ENTRYPOINTS
        .iter()
        .filter(|entrypoint| entrypoint.agent == Some(AgentEntrypoint::Claude))
}

/// One `note:` line per agent this run puts the managed block in front of more
/// than once (§FS-init.2.1.1). `init` no longer creates that second file, but a
/// repository that already carries two keeps feeding the same block to one
/// agent twice, and the run that just wrote to each of them is the only place
/// that is visible.
///
/// `canonical_selected` says whether this run also writes `AGENTS.md`, which a
/// companion symlink resolves to: that symlink carries whatever the canonical
/// file carries, so it is one of the agent's copies of the block even though it
/// is never in the companion plan (§FS-init.2.1).
///
/// `linked_conversation` withholds that reading for the Claude symlink alone,
/// because there the shadowed-entrypoint note above names the same two files
/// and a *better* fix (§FS-init.2.3.4.17). "Delete the one you do not want" is
/// the wrong sentence for a repository that committed the `link` opinion and
/// just asked for the second file: one of the two is a symlink to the entrypoint
/// every other agent reads, and only the specific note says which.
pub(crate) fn duplicate_agent_entrypoint_notes(
    target: &Path,
    entrypoints: &[InitCompanionAgentEntrypoint],
    canonical_selected: bool,
    linked_conversation: bool,
    dry_run: bool,
) -> Vec<String> {
    let canonical = target.join(CANONICAL_AGENT_ENTRYPOINT);
    let written: Vec<&CompanionAgentEntrypoint> = COMPANION_AGENT_ENTRYPOINTS
        .iter()
        .filter(|entrypoint| {
            let path = target.join(entrypoint.rel);
            let spoken_for =
                linked_conversation && entrypoint.agent == Some(AgentEntrypoint::Claude);
            entrypoints.iter().any(|planned| planned.path() == path)
                || (canonical_selected
                    && !spoken_for
                    && is_symlink_to(&path, &canonical).unwrap_or(false))
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
