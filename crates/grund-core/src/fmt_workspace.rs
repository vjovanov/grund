/// Run one formatter pass across every project in a workspace-root scope.
///
/// A write-capable caller first invokes this with `perform_writes = false`.
/// That lets every project discover whether it needs the whole declaration set
/// before any earlier project can be changed. If any project refuses, scan
/// errors from both strict and ordinary project walks become one strict abort
/// in the root-then-members order established by `WorkspaceContext`
/// (§FS-fmt.3).
fn fmt_workspace_projects(
    context: &WorkspaceContext,
    render: &Config,
    add_marker: bool,
    explicit_cross_refs: bool,
    requested_write: bool,
    perform_writes: bool,
) -> Result<FmtTreeOutcome> {
    let mut outcome = FmtTreeOutcome {
        changes: Vec::new(),
        scan_errors: Vec::new(),
        refused_writes: Vec::new(),
    };
    let mut preflight_scan_errors = Vec::new();
    let mut strict_abort = false;

    for project in &context.projects {
        let auto_cross_refs = auto_cross_refs_for_scope(
            &project.config,
            Some(&project.config.root),
            true,
            requested_write,
        )?;
        let run_opts = FmtRunOpts {
            add_marker,
            cross_refs: explicit_cross_refs || auto_cross_refs,
            write: perform_writes,
            render,
            workspace: Some(context),
            precomputed_findings: usable_findings(project),
            // §FS-fmt.6.1: the index is linkified whatever the toggle says.
            index_cross_refs: requested_write || explicit_cross_refs,
        };
        match fmt_tree(
            &project.config,
            Some(&project.config.root),
            true,
            &run_opts,
        ) {
            Ok(mut walked) => {
                preflight_scan_errors.extend(walked.scan_errors.iter().cloned());
                outcome.changes.append(&mut walked.changes);
                outcome.scan_errors.append(&mut walked.scan_errors);
                outcome.refused_writes.append(&mut walked.refused_writes);
            }
            Err(error) => {
                if let Some(abort) = error.downcast_ref::<FmtScanAbort>() {
                    strict_abort = true;
                    preflight_scan_errors.extend(abort.scan_errors.iter().cloned());
                } else {
                    return Err(error);
                }
            }
        }
    }

    if strict_abort {
        return Err(FmtScanAbort {
            scan_errors: preflight_scan_errors,
        }
        .into());
    }
    Ok(outcome)
}
