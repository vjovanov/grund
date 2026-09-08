/// Ownership and canonical-root boundaries shared by the scanner's reporting
/// walk and its read-any-file probe (§AR-scanner.1, §FS-workspace.6).

/// Whether a canonical path belongs to a project of this run that is **not** the
/// one doing the walking (§FS-workspace.6, §AR-workspace.6). The owner is the
/// innermost project root containing it, since a nested member's root sits inside
/// the block that listed it. A path no loaded project owns does not cross this
/// ownership boundary; the canonical project-root fence is answered separately
/// by the caller (§FS-config.3.5.1). Empty list — every run that loaded no
/// workspace — answers `false` without a comparison.
fn owned_by_another_project(config: &Config, own_root: &Path, canonical: &Path) -> bool {
    config
        .workspace_project_roots
        .iter()
        .filter(|root| canonical.starts_with(root))
        .max_by_key(|root| root.components().count())
        .is_some_and(|owner| owner.as_path() != own_root)
}

/// Whether directory-link traversal carried a scan root outside the canonical
/// project root (§FS-config.3.5.1, §AR-scanner.1). For an in-project spelling,
/// every component below the project root is checked: the named root may be a
/// descendant of the link rather than the link itself. An external spelling is
/// rejected only when the named root itself is a link, so a plain parent-relative
/// external root remains intentional scope. Comparing the resolved roots first
/// keeps an aliased config root and in-root links readable.
fn outward_directory_link_root(
    scan_root: &Path,
    canonical_scan_root: &Path,
    project_root: &Path,
    physical_root: &Path,
) -> bool {
    if canonical_scan_root.starts_with(physical_root) {
        return false;
    }
    let Ok(relative) = scan_root.strip_prefix(project_root) else {
        return is_directory_symlink(scan_root);
    };
    let mut component_path = project_root.to_path_buf();
    relative.components().any(|component| {
        component_path.push(component);
        is_directory_symlink(&component_path)
    })
}

fn is_directory_symlink(path: &Path) -> bool {
    path.is_dir()
        && fs::symlink_metadata(path)
            .is_ok_and(|metadata| metadata.file_type().is_symlink())
}
