//! §FS-fmt.7.1 — `grund fmt` with no `<path>` and `grund fmt .` name the same
//! scope, so they are one run written two ways: same stdout, same stderr, same
//! exit code, same tree afterwards, in every mode.
//!
//! This file used to pin that for one tree and one mode — the shape issue #105
//! had, where the no-path form reused a project's already-computed `Findings`
//! without checking whether the scan that produced them met an error, and so
//! rewrote a tree the explicit-path form correctly refused. The property is the
//! same; what changed is that it is now asserted over every shape in the corpus
//! (`support/fmt_shapes.rs`) and every mode, because a rule that only covers the
//! tree the last defect had does not cover the next code path
//! (§DF-fmt-one-model.2.2).
//!
//! Unix only: the corpus needs real broken symlinks.

#![cfg(unix)]

#[path = "support/fmt_shapes.rs"]
mod fmt_shapes;

use fmt_shapes::{run_grund, shapes, snapshot, stderr, stdout};

/// Every mode §FS-fmt.7.1 names: the implicit dry run, the explicit one, the
/// write, and each of the two rewrite knobs on a write and a dry run.
const MODES: &[&[&str]] = &[
    &["fmt"],
    &["fmt", "--check"],
    &["fmt", "--write"],
    &["fmt", "--marker", "--write"],
    &["fmt", "--check", "--cross-refs"],
    &["fmt", "--write", "--cross-refs"],
];

#[test]
fn the_default_scope_and_its_explicit_path_are_one_run() {
    for shape in shapes() {
        for mode in MODES {
            let slot = format!("scope-{}", mode.join("-"));
            let omitted = shape.materialize(&format!("{slot}-omitted"));
            let explicit = shape.materialize(&format!("{slot}-explicit"));

            let mut with_path = mode.to_vec();
            with_path.push(".");
            let without = run_grund(mode, &omitted);
            let with = run_grund(&with_path, &explicit);

            let form = format!("{} / `grund {}`", shape.name, mode.join(" "));
            assert_eq!(
                without.status.code(),
                with.status.code(),
                "{form}: exit code differs by how the scope was named\n  no path: {}\n  `.`: {}",
                stderr(&without),
                stderr(&with),
            );
            assert_eq!(
                stdout(&without),
                stdout(&with),
                "{form}: report differs by how the scope was named",
            );
            assert_eq!(
                stderr(&without),
                stderr(&with),
                "{form}: diagnostics differ by how the scope was named",
            );
            assert_eq!(
                snapshot(&omitted),
                snapshot(&explicit),
                "{form}: the two forms left different trees on disk",
            );
        }
    }
}

#[test]
/// The equality above is satisfied by two runs that are wrong together, so the
/// shape #105 had keeps its absolute claim: on a tree the completeness check
/// refuses, *neither* form writes anything, and both say so (§FS-fmt.3).
fn neither_form_rewrites_a_tree_the_completeness_check_refuses() {
    let refusing = ["strict-abort", "two-scopes", "workspace-member-abort"];
    for shape in shapes().into_iter().filter(|s| refusing.contains(&s.name)) {
        let omitted = shape.materialize("refusal-omitted");
        let explicit = shape.materialize("refusal-explicit");
        let before = snapshot(&omitted);

        let without = run_grund(&["fmt", "--write"], &omitted);
        let with = run_grund(&["fmt", "--write", "."], &explicit);

        for (form, output, root) in [("no path", &without, &omitted), ("`.`", &with, &explicit)] {
            assert_eq!(
                output.status.code(),
                Some(2),
                "{}: {form}: expected the strict refusal, stderr was: {}",
                shape.name,
                stderr(output),
            );
            assert_eq!(
                stdout(output),
                "",
                "{}: {form}: an aborted run emitted a rewrite report",
                shape.name,
            );
            assert!(
                stderr(output).contains("nothing was rewritten: "),
                "{}: {form}: the refusal did not say the tree was untouched: {}",
                shape.name,
                stderr(output),
            );
            assert_eq!(
                snapshot(root),
                before,
                "{}: {form}: a refused run rewrote the tree",
                shape.name,
            );
        }
    }
}
