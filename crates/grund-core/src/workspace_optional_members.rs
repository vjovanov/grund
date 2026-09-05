/// One `[workspace] optional_members` entry whose directory this checkout does
/// not have, and therefore one namespace this run did not read
/// (§FS-workspace.2.2, §FS-workspace.2.2.1).
///
/// This file is the `optional_members` rule set: the entry shapes the key adds to
/// the ones `members` already has, the alias an optional entry carries, and the
/// announcement an absent one earns. It rides on this first item rather than a
/// `//!` module doc because the crate is assembled by `include!`
/// (§AR-core-module-layout.2).
///
/// Split out of `workspace_members.rs` rather than added to it: that file is one
/// list's expansion and the invariants that list has to satisfy, and this is a
/// second list with a grammar rule of its own (no glob), an alias rule of its own
/// (the entry's last segment, not the member's `project_name`), and an outcome the
/// other list has no shape for — a member that is simply not there, and a report
/// that has to say so (§AR-core-module-layout.1, §FS-check.4.10).
#[derive(Clone)]
pub struct AbsentOptionalNamespace {
    /// The entry **as the config wrote it** — the string an author can edit
    /// (§FS-errors.4).
    pub written: String,
    /// The whole alias path this run spells the namespace with: one segment per
    /// workspace level, so an entry one `[workspace]` block down is `sub/vendored`
    /// while the entry itself stays `vendored` (§FS-check.4.10). Expansion sets it
    /// to the bare segment; the walk that knows the enclosing path composes the
    /// rest ([`qualify_absent_optional`]).
    pub alias_path: String,
    /// The `optional_members` line of the block that holds the entry.
    pub source: ConfigLocation,
}

/// §FS-workspace.2.2 / §FS-workspace.2.2.2: the three refusals `optional_members`
/// adds to the shape rules `members` already carries. All three are properties of
/// the entry text alone, which is why they are taken at config load — before any
/// directory is looked for, so the same config is rejected in the checkout that
/// has the member and in the checkout that does not.
fn validate_optional_workspace_member(path: &Path, line: usize, member: &str) -> Result<()> {
    validate_workspace_member(path, line, member)?;
    // §FS-workspace.2.2: an absent parent directory names no namespaces, so a
    // glob here would appear to work and do nothing. The message names the shape
    // that works, because a user who has just been refused needs the form to write.
    if member.contains('*') {
        return Err(anyhow!(
            "{}:{line}: [workspace] optional_members may not use a glob: `{member}` — an absent \
             parent names no namespaces; list one concrete entry per namespace instead",
            format_path(path),
        ));
    }
    let alias = optional_member_alias_segment(member);
    if !is_valid_project_alias(alias) {
        return Err(anyhow!(
            "{}:{line}: {} for workspace member `{member}`",
            format_path(path),
            invalid_project_alias_message(alias),
        ));
    }
    Ok(())
}

/// §FS-workspace.2.2.2: the alias of an optional member is the entry's last path
/// segment. An absent member has no config to read `project_name` from and no
/// directory to take a basename from, so the entry text is the only name the
/// block that lists it can recover — and taking it in both checkouts is what makes
/// one citation text mean one thing in a full tree and a partial one.
fn optional_member_alias_segment(member: &str) -> &str {
    member.rsplit('/').next().unwrap_or(member)
}

/// The `invalid workspace project alias` sentence, shared with [`derive_alias`] so
/// an entry refused for its segment and a member refused for its `project_name`
/// read as one rule (§AR-workspace.5.3).
fn invalid_project_alias_message(alias: &str) -> String {
    format!("invalid workspace project alias `{alias}` (expected [a-z][a-z0-9-]*)")
}

fn workspace_optional_members_error(config: &Config, message: String) -> anyhow::Error {
    config_location_error(config.workspace_optional_members_source.as_ref(), message)
}

/// §FS-workspace.2.2: fold this block's `optional_members` into the expanded
/// member list `members` already holds. A present entry becomes an ordinary
/// member — every §FS-workspace.2 invariant below applies to it unchanged; an
/// absent one is returned for the report to announce (§FS-check.4.10) and costs
/// the run nothing else.
fn expand_optional_members(
    config: &Config,
    members: &mut Vec<WorkspaceMember>,
) -> Result<Vec<AbsentOptionalNamespace>> {
    let Some(source) = config.workspace_optional_members_source.clone() else {
        return Ok(Vec::new());
    };
    // §FS-workspace.2.2 "one entry belongs to one list": the roots `members`
    // already expanded to, read before the two lists are merged. Canonical, the
    // way §FS-workspace.6.1 compares roots, so a glob in `members` that expands
    // onto an optional entry is the same collision — and read *now*, because the
    // dedup at the end of expansion would otherwise resolve the contradiction
    // silently by dropping one of the two entries the author wrote.
    let plain: Vec<PathBuf> = members.iter().map(|member| member.root.clone()).collect();
    let mut absent = Vec::new();
    for entry in &config.workspace_optional_members {
        let lexical = config.root.join(entry);
        // §FS-workspace.2.2.1: absent is `is_dir` and nothing more. The empty
        // directory git leaves for an uninitialized submodule is *present*, and
        // deliberately so — widening the test would put a typo'd directory on the
        // unverified path and move the blind spot's edge somewhere no line of the
        // repository records.
        if !lexical.is_dir() {
            absent.push(AbsentOptionalNamespace {
                written: entry.clone(),
                alias_path: optional_member_alias_segment(entry).to_string(),
                source: source.clone(),
            });
            continue;
        }
        let root = workspace_member_root(config, Some(&source), entry, &lexical)?;
        if plain.contains(&root) {
            return Err(workspace_optional_members_error(
                config,
                format!("`{entry}` is listed in both [workspace] members and optional_members"),
            ));
        }
        members.push(WorkspaceMember {
            written: entry.clone(),
            root,
            optional: true,
        });
    }
    Ok(absent)
}

/// §FS-workspace.2.2.2: the alias of a **present** optional member. Still the
/// entry's last path segment — and a `project_name` that disagrees with it is a
/// config error at the `optional_members` line rather than a second name.
///
/// Resolving the disagreement in grund's favour either way is what the rule
/// exists to refuse: `<§>hardware/sprayer/FS-nozzle` would resolve in the tree that
/// has the member and quietly name nothing in the tree that does not, which is a
/// trap only the checkout least equipped to notice can spring. Either name may be
/// the one the existing citations already write, so the repository picks.
fn optional_member_alias(member_config: &Config, block: &Config, written: &str) -> Result<String> {
    let segment = optional_member_alias_segment(written);
    match &member_config.project_name {
        Some(name) if name != segment => Err(workspace_optional_members_error(
            block,
            format!(
                "optional workspace member `{written}` declares project_name `{name}`, but its \
                 alias is the entry's last segment `{segment}` — make the two agree"
            ),
        )),
        _ => Ok(segment.to_string()),
    }
}

/// §FS-workspace.2.2, §FS-workspace.6.1: the `optional_members` entry of `block`
/// that names `child_root`, if any.
///
/// The ancestor climb needs this and nothing pins it: an optional entry claims the
/// directory below it like a `members` entry does, but the alias it contributes is
/// its own segment rather than the member's `project_name`. A run started *inside*
/// a present optional member reads its alias path out of that claim, so without
/// this it would spell the subtree one way and the workspace root another —
/// exactly the disagreement §FS-workspace.6.1 exists to prevent.
fn optional_entry_naming<'a>(block: &'a Config, child_root: &Path) -> Option<&'a String> {
    block
        .workspace_optional_members
        .iter()
        .find(|entry| canonical_workspace_path(&block.root.join(entry)) == child_root)
}

/// §FS-check.4.10: the alias path a namespace is announced by, composed one level
/// at a time exactly as a project's alias is (§FS-workspace.6.1) — `vendored` at
/// the outermost root, `sub/vendored` for the same entry one block down.
fn qualify_absent_optional(
    absent: Vec<AbsentOptionalNamespace>,
    prefix: &str,
) -> Vec<AbsentOptionalNamespace> {
    absent
        .into_iter()
        .map(|namespace| AbsentOptionalNamespace {
            alias_path: qualify_alias(prefix, &namespace.alias_path),
            ..namespace
        })
        .collect()
}

/// §FS-check.4.10: one warning per absent optional entry, in the order the list
/// writes them, anchored at the `optional_members` line of the block that holds it.
///
/// A **located** finding on stdout, where its two nearest neighbours (§FS-check.4.8,
/// §FS-check.4.9) are CLI-level `warning:` lines on stderr. Those two report a
/// misconfiguration that makes every command in the tree wrong; nothing is
/// misconfigured here — the repository declared this may happen and it happened —
/// and what is at stake is only the coverage of `check`'s own report. Exit `2` is
/// grund's one way to say "this report is incomplete" and this is the single case
/// that is deliberately incomplete and still exits `0`, so the exit code carries
/// nothing and stdout has to. `success` is withheld all the same, because it is
/// withheld for every warning (§FS-check.2.1).
///
/// It names no remedy and no release: the only thing that would "fix" it is a
/// checkout with the member in it, which is not grund's to ask for, and the skip
/// is the standing price of the opt-out rather than a state to migrate off.
fn absent_optional_member_warnings(config: &Config) -> Vec<Diagnostic> {
    config
        .workspace_absent_optional
        .iter()
        .map(|absent| Diagnostic {
            code: "optional-member-absent",
            path: Some(absent.source.path.clone()),
            line: Some(absent.source.line),
            column: None,
            message: absent_optional_member_message(&absent.written, &absent.alias_path),
            sites: Vec::new(),
        })
        .collect()
}

/// The sentence [`absent_optional_member_warnings`] carries, built apart from the
/// diagnostic so a test can read it: the entry as the config wrote it, the
/// namespace by the whole alias path a citation has to write, and what the run
/// therefore does not cover (§FS-check.4.10, §FS-errors.4).
fn absent_optional_member_message(written: &str, alias_path: &str) -> String {
    format!(
        "optional workspace member `{written}` is absent — citations into namespace \
         `{alias_path}` were not checked, so this run does not cover it"
    )
}

/// §FS-workspace.4: whether an alias path names, or descends into, a namespace
/// this run did not read. Such a citation is **unverified** — the third state
/// beside resolved and unknown — so nothing is reported at its site: the citation
/// may be perfect and the checkout merely partial, and a tree that cites an absent
/// namespace widely would pay thousands of lines to be told one fact it is told
/// once, at the entry that made the skip legal (§FS-check.4.10).
///
/// Descending counts because an absent member may itself have declared
/// `[workspace]` and the run cannot know how many levels it had or what they were
/// called (§FS-workspace.2.2.2) — `hardware/AR-bus` and `hardware/sprayer/FS-nozzle`
/// alike when `hardware` is the absent entry.
fn namespace_is_unverified(config: &Config, namespace: &str) -> bool {
    config.workspace_absent_optional.iter().any(|absent| {
        namespace == absent.alias_path
            || namespace
                .strip_prefix(&absent.alias_path)
                .is_some_and(|rest| rest.starts_with('/'))
    })
}
