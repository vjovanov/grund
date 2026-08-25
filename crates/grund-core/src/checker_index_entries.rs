// The membership half of the kind-index invariant (§AR-checker.2.16), split
// from `checker_index.rs` so adding external enrollment does not push that rule
// family past the core-source file budget (§AR-core-module-layout.3).

/// The IDs each kind index owes an entry for, keyed by the index's
/// config-root-relative path (§FS-check.4.6). `folder_owed` preserves the
/// original folder-membership rule; `owed` additionally holds external inline
/// declarations enrolled by a canonical link. Their exact sites stay separate
/// so another citation of the same external ID on the page remains real use
/// (§DF-index-not-an-inbound-citation.2.2).
struct KindIndexEntries {
    configured_root: PathBuf,
    physical_root: PathBuf,
    owed: BTreeMap<PathBuf, BTreeSet<Id>>,
    folder_owed: BTreeMap<PathBuf, BTreeSet<Id>>,
    external_sites: BTreeMap<PathBuf, BTreeSet<(usize, usize)>>,
}

impl KindIndexEntries {
    fn new(findings: &Findings, config: &Config) -> Self {
        let configured_root = scanned_path_key(&config.root);
        let physical_root = physical_path_key(&config.root);
        let targets = kind_index_targets(config);
        let mut folder_owed: BTreeMap<PathBuf, BTreeSet<Id>> = BTreeMap::new();
        for target in &targets {
            for (id, decls) in &findings.declarations {
                if id.kind == target.kind
                    && declarations_under_folder(
                        decls,
                        &target.folder_key,
                        &configured_root,
                        &physical_root,
                    )
                {
                    folder_owed
                        .entry(target.index_key.clone())
                        .or_default()
                        .insert(id.clone());
                }
            }
        }
        let mut owed = folder_owed.clone();
        let external_sites = enroll_external_inline_declarations(
            findings,
            config,
            &targets,
            &configured_root,
            &physical_root,
            &mut owed,
        );
        Self {
            configured_root,
            physical_root,
            owed,
            folder_owed,
            external_sites,
        }
    }

    /// The IDs this index owes an entry for, or `None` when `path` is not a
    /// configured index (§FS-fmt.6.1). What the always-linkify carve-out wraps:
    /// declarations under the folder plus an external inline ID only after its
    /// canonical enrollment link exists (§FS-check.4.6).
    fn entries_in(&self, path: &Path) -> Option<&BTreeSet<Id>> {
        if self.owed.is_empty() {
            return None;
        }
        let relative =
            scanned_decl_relative_path(path, &self.configured_root, &self.physical_root)?;
        self.owed.get(relative.as_ref())
    }

    /// §FS-check.4.1 / §DF-index-not-an-inbound-citation.2.2: folder-owned IDs
    /// keep PR #134's accounting. For an external ID only the canonical link site
    /// is navigation; a second same-ID citation in the index is ordinary use.
    fn is_index_entry(&self, citation: &Citation) -> bool {
        if citation.namespace.is_some() || self.owed.is_empty() {
            return false;
        }
        let Some(relative) = scanned_decl_relative_path(
            &citation.file,
            &self.configured_root,
            &self.physical_root,
        ) else {
            return false;
        };
        self.folder_owed
            .get(relative.as_ref())
            .is_some_and(|ids| ids.contains(&citation.id))
            || self
                .external_sites
                .get(relative.as_ref())
                .is_some_and(|sites| sites.contains(&(citation.line, citation.column)))
    }
}

/// Add every canonical external-inline enrollment to `owed`, returning the
/// exact citation sites that are navigation rather than use (§FS-check.4.6).
/// Citations are already scanner records; index text is read once per target
/// solely to inspect the persisted Markdown wrapper and destination.
fn enroll_external_inline_declarations(
    findings: &Findings,
    config: &Config,
    targets: &[KindIndexTarget<'_>],
    configured_root: &Path,
    physical_root: &Path,
    owed: &mut BTreeMap<PathBuf, BTreeSet<Id>>,
) -> BTreeMap<PathBuf, BTreeSet<(usize, usize)>> {
    let targets_by_index: BTreeMap<&Path, &KindIndexTarget<'_>> = targets
        .iter()
        .map(|target| (target.index_key.as_path(), target))
        .collect();
    let lines_by_index: BTreeMap<&Path, Vec<String>> = targets
        .iter()
        .filter_map(|target| {
            let text = fs::read_to_string(&target.index_file).ok()?;
            Some((
                target.index_key.as_path(),
                text.lines().map(str::to_string).collect(),
            ))
        })
        .collect();
    let mut sites: BTreeMap<PathBuf, BTreeSet<(usize, usize)>> = BTreeMap::new();

    for citation in &findings.citations {
        if citation.namespace.is_some()
            || citation.section.is_some()
            || !citation.has_marker
        {
            continue;
        }
        let Some(relative) = scanned_decl_relative_path(
            &citation.file,
            configured_root,
            physical_root,
        ) else {
            continue;
        };
        let Some(target) = targets_by_index.get(relative.as_ref()).copied() else {
            continue;
        };
        if citation.id.kind != target.kind {
            continue;
        }
        let Some(decls) = findings.declarations.get(&citation.id) else {
            continue;
        };
        if external_inline_home(
            config,
            decls,
            &target.folder_key,
            configured_root,
            physical_root,
        )
        .is_none()
        {
            continue;
        }
        let Some(line) = lines_by_index
            .get(target.index_key.as_path())
            .and_then(|lines| lines.get(citation.line.saturating_sub(1)))
        else {
            continue;
        };
        let Some(written_target) = link_target_at_citation(line, citation) else {
            continue;
        };
        let Some(canonical_target) = markdown_link_target(
            &target.index_file,
            &citation.id,
            None,
            config,
            findings,
        ) else {
            continue;
        };
        if written_target != canonical_target {
            continue;
        }
        owed
            .entry(target.index_key.clone())
            .or_default()
            .insert(citation.id.clone());
        sites
            .entry(target.index_key.clone())
            .or_default()
            .insert((citation.line, citation.column));
    }
    sites
}

/// The one logical, non-Markdown home an enrollment candidate would add. A
/// declaration already under the folder is covered by the ordinary rule, and
/// multiple independent homes remain the duplicate error rather than becoming
/// an index membership `grund` guessed (§REQ-no-wrong-citation.1).
fn external_inline_home<'a>(
    config: &Config,
    decls: &'a [Declaration],
    folder_key: &Path,
    configured_root: &Path,
    physical_root: &Path,
) -> Option<&'a Declaration> {
    if declarations_under_folder(decls, folder_key, configured_root, physical_root) {
        return None;
    }
    let mut homes = decls
        .iter()
        .filter(|decl| !is_stub_for_inline_decl(&config.root, decl, decls));
    let home = homes.next()?;
    if homes.next().is_some()
        || home.is_stub
        || home.e2e_case.is_some()
        || home.file.extension().and_then(|ext| ext.to_str()) == Some("md")
    {
        return None;
    }
    Some(home)
}

/// Return the destination of the Markdown wrapper at this scanner-recorded
/// citation site. Unlike `index_citation_form`, enrollment must inspect this
/// exact occurrence: another linked occurrence of the same text on the line
/// cannot lend it structural meaning (§DF-index-entry-form.2.7).
fn link_target_at_citation<'a>(line: &'a str, citation: &Citation) -> Option<&'a str> {
    let start = citation.column.checked_sub(1)?;
    let open = start.checked_sub(1)?;
    if line.as_bytes().get(open) != Some(&b'[') || is_inside_inline_code(line, open) {
        return None;
    }
    let end = start.checked_add(citation.text.len())?;
    if line.get(start..end)? != citation.text {
        return None;
    }
    let rest = line.get(end..)?.strip_prefix("](")?;
    let close = rest.find(')')?;
    (close > 0).then_some(&rest[..close])
}
