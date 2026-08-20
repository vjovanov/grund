// What `init`'s `<path>` argument *is*, and whether it may be scaffolded at
// all. `init` is the only subcommand that creates adoption scaffolding, and
// every path it writes is `<path>`-relative — which makes that one argument
// load-bearing in a way no other command's is, and worth interpreting in one
// place (§FS-init.1, §FS-init.1.2).

/// §FS-init.1.2: the version-control markers whose presence in the target or
/// any ancestor says it is inside a working tree. Presence is what is tested,
/// not type — a linked worktree and a submodule both write `.git` as a file.
/// Nothing here reads any of them, so §FS-non-goals.6 is untouched: the
/// marker's existence is a fact about the tree in the way a `grund.toml`'s is.
const INIT_VCS_MARKERS: [&str; 4] = [".git", ".hg", ".jj", ".svn"];

/// Everything that can disqualify a target before `init` writes anything: it
/// must exist and be a directory (§FS-init.1), and it must not be one of the
/// refused targets (§FS-init.1.2). Returns the message naming the rule that
/// declined and, where one exists, the flag that proceeds. `no_vcs` lifts the
/// version-control rule and only that one.
///
/// A refusal is total: `init` writes nothing at all, not even the files that
/// would have been unobjectionable, so this answers for the whole run rather
/// than per path.
fn refuse_init_target(target: &Path, no_vcs: bool) -> Option<String> {
    if !target.exists() {
        return Some(format!(
            "target directory does not exist: {}",
            target.display()
        ));
    }
    if !target.is_dir() {
        return Some(format!("target is not a directory: {}", target.display()));
    }
    let resolved = resolve_for_target_compare(target);
    // The home directory, unconditionally. `<path>/.claude/CLAUDE.md` with
    // `<path>` at `$HOME` *is* `~/.claude/CLAUDE.md`, the machine-global file
    // every agent session in every project loads (§FS-integrations.4.3) — and
    // `.claude/` existing is the same signal automatic mode reads as "this
    // project uses Claude" (§FS-init.2.1). Both readings are right in their own
    // scope; in `$HOME` they name one file. No flag lifts this: nobody targets
    // `$HOME` on purpose, so there is no case to keep working.
    // `std::env::home_dir` rather than `$HOME` directly: the variable is the
    // Unix spelling, and a Windows runner that never sets it would silently
    // lose the rule that stops the accident this exists for.
    if let Some(home) = std::env::home_dir()
        && resolve_for_target_compare(&home) == resolved
    {
        return Some(format!(
            "refusing to scaffold into the home directory {} — init writes repository paths, and .claude/CLAUDE.md here is the machine-global agent instruction file",
            resolved.display()
        ));
    }
    if no_vcs {
        return None;
    }
    if init_vcs_tree_covers(&resolved) {
        return None;
    }
    // Unlike the rule above this one has a legitimate other side — a directory
    // scaffolded before `git init`, or a project under no VCS at all — so it is
    // a default, not a law, and the message names the flag that proceeds.
    Some(format!(
        "{} is not inside a version-controlled tree — no {} here or above; pass --no-vcs to scaffold anyway",
        resolved.display(),
        format_alternatives(&INIT_VCS_MARKERS),
    ))
}

/// §FS-init.1.2 — decline when a planned entrypoint is one of the file-backed
/// user-global agent instruction targets of §FS-integrations.4.3. Today
/// `~/.claude/CLAUDE.md` is the only path the repository entrypoints and that
/// table share, so the home-directory rule above already covers every case this
/// catches; they are two checks because they are two different facts, and only
/// this one stays true if either table grows an entry. The division is the one
/// §FS-integrations.4.3 already states: the user-global files carry machine-wide
/// policy and are `grund integrations --write`'s to manage, the repository
/// entrypoint carries this project's syntax and is `init`'s.
fn refuse_init_global_instruction_paths(
    entrypoints: &[InitCompanionAgentEntrypoint],
) -> Option<String> {
    entrypoints.iter().find_map(|entrypoint| {
        let resolved = resolve_for_target_compare(entrypoint.path());
        let owned = GLOBAL_AGENT_INSTRUCTION_TARGETS.iter().any(|target| {
            expand_target(target.file)
                .is_some_and(|global| resolve_for_target_compare(&global) == resolved)
        });
        owned.then(|| {
            format!(
                "refusing to write {} — that is the machine-global agent instruction file, managed by `grund integrations --write`, not a repository entrypoint",
                resolved.display()
            )
        })
    })
}

/// The default project name when `--name` is omitted: the basename of `<path>`
/// resolved to an absolute path (§FS-init.1).
fn derive_default_name(target: &Path) -> Result<String> {
    let absolute =
        fs::canonicalize(target).with_context(|| format!("resolve {}", target.display()))?;
    absolute
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_string())
        .ok_or_else(|| anyhow!("cannot derive project name from {}", absolute.display()))
}

/// Walk from the target to the filesystem root looking for a marker.
fn init_vcs_tree_covers(target: &Path) -> bool {
    let mut current = Some(target);
    while let Some(dir) = current {
        if INIT_VCS_MARKERS
            .iter()
            .any(|marker| dir.join(marker).exists())
        {
            return true;
        }
        current = dir.parent();
    }
    false
}

/// An absolute, symlink-resolved form of `path` for comparing two paths that
/// name the same file. A path that does not exist yet still has to compare
/// equal to the same path written literally, so a failed canonicalization falls
/// back to resolving the parent and re-joining the final component; when even
/// that fails the path is returned as given, which compares equal to itself and
/// to nothing else.
fn resolve_for_target_compare(path: &Path) -> PathBuf {
    if let Ok(resolved) = path.canonicalize() {
        return resolved;
    }
    if let (Some(parent), Some(name)) = (path.parent(), path.file_name())
        && let Ok(resolved) = parent.canonicalize()
    {
        return resolved.join(name);
    }
    path.to_path_buf()
}

/// `a`, `b`, `c`, or `d` — the marker list spelled the way the message reads.
fn format_alternatives(items: &[&str]) -> String {
    match items {
        [] => String::new(),
        [only] => (*only).to_string(),
        [first, second] => format!("{first} or {second}"),
        [rest @ .., last] => format!("{}, or {last}", rest.join(", ")),
    }
}
