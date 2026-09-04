//! §FS-fmt.7.2 — the unreadable paths `fmt` reports are the ones `check`
//! reports over the same walk. `fmt` is the model `check` verifies plus a write
//! step, so the one command that edits files in place may not have a private
//! account of the files it never saw.
//!
//! Equality is of the `<path>: <reason>` pairs and their order, not of the bytes
//! of the line: a strict abort spells its lines `error: nothing was rewritten:
//! …` on purpose (§FS-fmt.3), because the two exit `2`s mean opposite things.
//! `scan_error_pairs` removes that one licensed prefix and nothing else.
//!
//! The comparison is per walk rather than per argv, which is the part a scoped
//! run makes visible. A `fmt` that needs the whole declaration set scans the
//! project even under a narrowed path (§FS-fmt.2.4), so `grund fmt --check docs`
//! on a tree whose unreadable file sits in `src/` names that file while `grund
//! check docs` does not — and is compared against `grund check .`, which walks
//! what it walked. A partial run scans only its scope and is compared against
//! `check` over that scope.
//!
//! Unix only: the corpus needs real broken symlinks.

#![cfg(unix)]

#[path = "support/fmt_shapes.rs"]
mod fmt_shapes;

use fmt_shapes::{aborted_strictly, run_grund, scan_error_pairs, shapes, stderr};

/// Each `fmt` form §FS-fmt.7.2 names, over the default scope: the dry run, the
/// write, and the explicitly strict pass.
const DEFAULT_SCOPE_FORMS: &[&[&str]] = &[
    &["fmt", "--check"],
    &["fmt", "--write"],
    &["fmt", "--check", "--cross-refs"],
];

#[test]
fn fmt_and_check_give_the_same_account_of_the_default_scope() {
    for shape in shapes() {
        let checker = shape.materialize("reader-check");
        let expected = scan_error_pairs(&run_grund(&["check", "."], &checker));

        for form in DEFAULT_SCOPE_FORMS {
            let root = shape.materialize(&format!("reader-{}", form.join("-")));
            let output = run_grund(form, &root);
            assert_eq!(
                scan_error_pairs(&output),
                expected,
                "{} / `grund {}`: fmt's account of what it could not read differs \
                 from check's over the same tree\n  fmt stderr: {}",
                shape.name,
                form.join(" "),
                stderr(&output),
            );
        }
    }
}

#[test]
/// The scoped form. Which `check` run a narrowed `fmt` is comparable to is
/// decided by the walk `fmt` performed, and the run says which that was: a
/// strict abort scanned the project, an ordinary run scanned its scope.
fn a_scoped_fmt_gives_the_same_account_as_check_over_the_tree_it_walked() {
    for shape in shapes() {
        let root = shape.materialize("reader-scoped");
        let checker = shape.materialize("reader-scoped-check");

        let output = run_grund(&["fmt", "--check", shape.inner_scope], &root);
        let comparable = if aborted_strictly(&output) {
            "."
        } else {
            shape.inner_scope
        };
        let expected = scan_error_pairs(&run_grund(&["check", comparable], &checker));

        assert_eq!(
            scan_error_pairs(&output),
            expected,
            "{}: `grund fmt --check {}` disagrees with `grund check {comparable}`, \
             which walked the same tree\n  fmt stderr: {}",
            shape.name,
            shape.inner_scope,
            stderr(&output),
        );
    }
}

#[test]
/// The licensed difference is licensed in one direction only: the strict prefix
/// says the tree was not touched, so a run that carries it must have written
/// nothing, and a run that rewrote must not carry it (§FS-fmt.3).
fn the_strict_prefix_appears_exactly_where_nothing_was_rewritten() {
    for shape in shapes() {
        let root = shape.materialize("reader-prefix");
        let output = run_grund(&["fmt", "--write"], &root);
        let report = String::from_utf8_lossy(&output.stdout).to_string();

        if aborted_strictly(&output) {
            assert_eq!(
                report, "",
                "{}: a run claiming nothing was rewritten still reported rewrites",
                shape.name,
            );
        } else {
            assert!(
                !stderr(&output).contains("nothing was rewritten: "),
                "{}: a run that rewrote spelled its errors as a refusal: {}",
                shape.name,
                stderr(&output),
            );
        }
    }
}

#[test]
/// An equivalence between two runs that both do nothing is satisfied by
/// anything, so the corpus declares which half of it each shape exercises and
/// this test holds it to that. A fixture that stops meeting an unreadable path
/// fails here rather than quietly making three rules vacuous.
fn every_shape_exercises_the_half_of_the_corpus_it_claims() {
    for shape in shapes() {
        let root = shape.materialize("reader-coverage");
        let output = run_grund(&["fmt", "--check"], &root);
        assert_eq!(
            !scan_error_pairs(&output).is_empty(),
            shape.reports_unreadable,
            "{}: the shape's `reports_unreadable` no longer describes it: {}",
            shape.name,
            stderr(&output),
        );
    }
}
