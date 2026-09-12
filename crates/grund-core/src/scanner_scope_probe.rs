/// Whether the effective configured scope contains a file the scanner would read
/// (§FS-config.3.5, §FS-init.2.2). Root selection stays beside the walk so init
/// cannot grow a second, path-existence approximation of scanner policy. Both
/// levels use lazy `any`, stopping at the first readable file in the first root
/// that contains one (§GOAL-fast-feedback).
fn effective_scope_reads_any_file(config: &Config) -> bool {
    effective_scope_reads_any_file_with(config, |root| walk_reads_any_file(config, root))
}

/// Apply the effective root order lazily, separated only so the short-circuit
/// itself can be pinned without replacing scanner behavior (§FS-config.3.5).
fn effective_scope_reads_any_file_with(
    config: &Config,
    mut root_reads_any_file: impl FnMut(&Path) -> bool,
) -> bool {
    root_scope_roots(config, config.scan_full)
        .iter()
        .any(|root| root_reads_any_file(root))
}
