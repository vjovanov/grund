/// The `note:` for a repository whose committed `link` opinion cannot reach
/// Claude, because a Claude entrypoint is a symlink to the canonical file
/// (§FS-init.2.3.4.17). Silence here reads as the opinion not working, on the
/// one agent it was mostly written for.
///
/// The fix the note names is the one the *tree* leaves open, not the one this
/// run happens to be about: whether Claude has an entrypoint of its own is a
/// fact about the repository, and a run that selected some other agent has not
/// changed it. So `planned` — this run's plan for the files Claude reads
/// (§FS-init.2.1.1) — only decides *which* entrypoint to name and whether it is
/// current by the time the run ends; an entrypoint already on disk answers the
/// question just as well. Where Claude has none: write it where a path is free,
/// and — where symlinks have taken every path Claude reads — delete one first,
/// because there is nowhere left for `--claude` to write. Advising a command
/// that has just run and can do no more would be a note that never retires.
///
/// What this file holds: what one `grund init` run reports about the entrypoint
/// layout it found (§FS-init.2.2) — the `note:` lines, which change nothing and
/// never touch the exit code. Both are things a caller would otherwise have to
/// notice for itself — an agent reading the same block twice, or reading the
/// wrong form of it — and both are only visible from the run that just wrote to
/// those files.
///
/// Both are also written in the conditional under `--dry-run` (§FS-init.2.2):
/// the run wrote nothing, so a note in the present tense describes a tree that
/// does not exist, and an instruction that assumes the write already happened —
/// "delete the symlink" — costs the reader the only entrypoint they have.
///
/// The duplicate note is built from the run's plan and touches no disk; the
/// shadowed note asks the tree, because its subject is an agent the run may not
/// have selected at all. Neither guesses from the other's evidence: a note that
/// reads the plan for a fact only the tree has is how `--gemini` came to print a
/// diagnosis of Claude's files that a flagless run on the same tree contradicts.
pub(crate) fn shadowed_claude_entrypoint_note(
    root: &Path,
    planned: &[String],
    dry_run: bool,
) -> Result<Option<String>, (PathBuf, String)> {
    let canonical = root.join(CANONICAL_AGENT_ENTRYPOINT);
    let mut shadowed = Vec::new();
    let mut on_disk = Vec::new();
    let mut free = false;
    for entrypoint in claude_entrypoint_rows() {
        let path = root.join(entrypoint.rel);
        if !is_file_or_symlink(&path) {
            free |= path_missing_without_following_symlinks(&path);
            continue;
        }
        match is_symlink_to(&path, &canonical) {
            Ok(true) => shadowed.push(entrypoint.rel),
            Ok(false) => on_disk.push(entrypoint.rel.to_string()),
            Err(err) => return Err((path, format!("{err:#}"))),
        }
    }
    if shadowed.is_empty() {
        return Ok(None);
    }
    let subject = format_list(&shadowed, "and");
    let (is_are, symlinks, one_of_them) = if shadowed.len() == 1 {
        ("is", "a symlink", "delete it")
    } else {
        ("are", "symlinks", "delete one of them")
    };
    let reads = if dry_run { "would read" } else { "reads" };
    let head = format!(
        "{subject} {is_are} {symlinks} to {CANONICAL_AGENT_ENTRYPOINT}, so Claude {reads} the plain-location form"
    );
    // The symlink may go before the entrypoint only once that entrypoint really
    // carries the block: this run wrote to it, and the run was not a preview.
    // Otherwise the deletion is still the fix, and the note says what has to be
    // true first rather than sending the reader to delete their only Claude
    // entrypoint on the strength of a file that exists but says nothing.
    let carries_the_block = !dry_run && !planned.is_empty();
    let delete_symlinks = if shadowed.len() == 1 {
        "delete the symlink"
    } else {
        "delete the symlinks"
    };
    Ok(Some(
        match planned.first().map(String::as_str).or(on_disk.first().map(String::as_str)) {
            Some(entrypoint) if carries_the_block => format!(
                "{head} from there as well as the linked form from {entrypoint}; {delete_symlinks} to leave {entrypoint} as Claude's only entrypoint"
            ),
            Some(entrypoint) => format!(
                "{head} from there as well as the linked form from {entrypoint}; once {entrypoint} carries the block, {delete_symlinks} to leave it as Claude's only entrypoint"
            ),
            None if free => format!(
                "{head}; run `grund init --claude` to write a real Claude entrypoint that teaches the linked form"
            ),
            None => format!(
                "{head}, and the symlinks have taken every path a real Claude entrypoint could go; {one_of_them} and re-run `grund init --claude`"
            ),
        },
    ))
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
/// Everything this reads is already in the plan. `plan.canonical_symlinks` are
/// the companions that resolve to the canonical file: never in the companion
/// plan, because the canonical write is the one that reaches them, but copies of
/// the block for their agent all the same whenever this run also writes
/// `AGENTS.md` (§FS-init.2.1).
///
/// `reach` withholds that reading for a symlink whose agent the canonical render
/// cannot speak for, because there the shadowed-entrypoint note above names the
/// same two files and a *better* fix (§FS-init.2.3.4.17). "Delete the one you do
/// not want" is the wrong sentence for a repository that committed the `link`
/// opinion and just asked for the second file: one of the two is a symlink to
/// the entrypoint every other agent reads, and only the specific note says which.
pub(crate) fn duplicate_agent_entrypoint_notes(
    target: &Path,
    plan: &SelectedInitAgentEntrypoints,
    reach: CanonicalSurfaceReach,
    dry_run: bool,
) -> Vec<String> {
    let written: Vec<&CompanionAgentEntrypoint> = COMPANION_AGENT_ENTRYPOINTS
        .iter()
        .filter(|entrypoint| {
            let path = target.join(entrypoint.rel);
            plan.companions
                .iter()
                .any(|planned| planned.path() == path)
                || (plan.canonical
                    && !reach.leaves_uncovered(&path)
                    && plan.canonical_symlinks.contains(&path))
        })
        .collect();
    let mut notes = Vec::new();
    let mut considered = Vec::new();
    for entrypoint in &written {
        let Some(agent) = entrypoint.agent.filter(|agent| !considered.contains(agent)) else {
            continue;
        };
        considered.push(agent);
        let rels: Vec<&str> = written
            .iter()
            .filter(|other| other.agent == Some(agent))
            .map(|other| other.rel)
            .collect();
        if rels.len() < 2 {
            continue;
        }
        // Two is the shape every agent in the table has today; the wording is
        // derived rather than written for it, so a third path added to one agent
        // reports itself instead of claiming "both".
        let (all, times, spare) = if rels.len() == 2 {
            ("both", "twice".to_string(), "the one you do not want")
        } else {
            ("all", format!("{} times", rels.len()), "the ones you do not want")
        };
        let (carry, reads) = if dry_run {
            (format!("would {all} carry"), "would read")
        } else {
            (format!("{all} carry"), "reads")
        };
        notes.push(format!(
            "{} {carry} the managed block, so {} {reads} it {times}; delete {spare} — `grund init` creates only one",
            format_list(&rels, "and"),
            agent.name(),
        ));
    }
    notes
}
