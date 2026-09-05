/// The literal a §FS-workspace.8.1.1 candidate clause opens with. The builder
/// below writes it and [`names_member_id_candidate`] reads it back, so the two
/// shapes an `ID not found:` refusal now has cannot be told apart wrongly.
const MEMBER_CANDIDATE_CLAUSE: &str = "; did you mean ";

/// Whether an `ID not found:` refusal already names a project that declares the
/// ID (§FS-workspace.8.1.1). That is the one case where the `grund list` hint of
/// §FS-show.3 is withheld: the catalogue it points at is what this line has
/// already searched, and its other half proposes minting an ID that exists.
pub fn names_member_id_candidate(message: &str) -> bool {
    message.contains(MEMBER_CANDIDATE_CLAUSE)
}

/// §FS-workspace.8.1.1: name the projects that declare an unqualified ID the
/// current one does not, by appending the candidate clause to the `ID not
/// found:` refusal that would otherwise be a dead end.
///
/// Applied at the seam where the whole run is in hand — every `show` frontend
/// resolves through a [`WorkspaceContext`] before it renders — so the shipped
/// CLI and the deprecated `main_entry()` mirror (§AR-bindings.2) print the same
/// bytes from one builder, and the context that already holds every member's
/// findings is not loaded a second time to answer (§GOAL-fast-feedback).
///
/// Everything else is left exactly as raised: a qualified lookup already named
/// its project, an ID the current grammar rejects fails earlier as `invalid ID`,
/// and the `ID not found:` prefix stays the first token because it is what
/// selects the `not-found` code (§FS-errors.5).
fn with_member_id_candidates(
    err: anyhow::Error,
    context: &WorkspaceContext,
    qualified_alias: Option<&str>,
    raw_id: &str,
) -> anyhow::Error {
    if qualified_alias.is_some() {
        return err;
    }
    let message = format!("{err:#}");
    if !message.starts_with("ID not found:") {
        return err;
    }
    match member_id_candidate_clause(context, raw_id) {
        Some(clause) => anyhow!("{message}{clause}"),
        None => err,
    }
}

/// The clause itself, or `None` when no other project in the run declares the
/// ID — where there is nothing to name, the refusal is printed unchanged.
fn member_id_candidate_clause(context: &WorkspaceContext, raw_id: &str) -> Option<String> {
    let candidates = member_id_candidates(context, raw_id);
    (!candidates.is_empty())
        .then(|| format!("{MEMBER_CANDIDATE_CLAUSE}{}?", join_alternatives(&candidates)))
}

/// Every project other than the one the lookup ran against that declares the ID
/// as written, qualified with its alias — sorted and cut at three, joined the
/// way §FS-check.3.8 joins its own candidates. Several are listed and none is
/// chosen: two projects declaring one ID are two declarations in two namespaces
/// (§FS-workspace.8.1), and picking one would be a guess
/// (§REQ-no-wrong-citation.1).
///
/// A narrowed run offers nothing (§FS-workspace.8.1.1). Member-local runs,
/// standalone repositories, and a `<path>` pinned inside a member all load with
/// `workspace_loaded == false`; a run that cannot see the whole tree must not
/// guess at what the rest of it declares.
fn member_id_candidates(context: &WorkspaceContext, raw_id: &str) -> Vec<String> {
    if !context.workspace_loaded {
        return Vec::new();
    }
    let mut candidates: Vec<String> = context
        .projects
        .iter()
        .enumerate()
        .filter(|(index, _)| Some(*index) != context.current)
        .filter_map(|(_, project)| qualified_declared_id(project, raw_id))
        .collect();
    candidates.sort();
    // Three is the §FS-check.3.8 cut: `grund list` is the catalogue, a
    // diagnostic is not.
    candidates.truncate(3);
    candidates
}

/// The `<alias>/<ID>` spelling of the written text in one project, when that
/// project declares it.
///
/// §FS-workspace.1: the text is re-parsed with *this* project's `[id]` grammar
/// and rendered back with it, exactly as a qualified citation is. In a
/// mixed-format workspace one text is a different `Id` in each project —
/// `SPEC-007-shipping` is slug `007-shipping` under `{kind}-{slug}` and number
/// `007` under `{kind}-{number}-{slug}` — so carrying the current project's
/// parse across the boundary would find nothing and name no candidate at all.
///
/// A section written after the ID is dropped: this run never looked for that
/// coordinate here, so suggesting it would be the one guessed part of the line
/// (§FS-workspace.8.1.1). A text this project's own grammar rejects, or a
/// shorthand several of its declarations answer to, names no candidate here —
/// the clause offers a spelling that resolves, or it offers nothing.
fn qualified_declared_id(project: &WorkspaceProject, raw_id: &str) -> Option<String> {
    let (id, _section) = resolve_id_arg(raw_id, &project.config, &project.findings).ok()?;
    project
        .findings
        .declarations
        .contains_key(&id)
        .then(|| format!("{}/{}", project.alias, render_id(&project.config, &id)))
}
