//! §FS-check.3.17 — the one invariant the index-entry rule rests on: every
//! `unlinked-index-entry` error is cleared by the command its own message
//! names. `check` reports the finding only for an occurrence the next
//! `grund fmt --write` turns into a link (§FS-fmt.6.2), so running that command
//! must take the tree from the error to a clean run, in one pass, with no
//! remaining finding of that code.
//!
//! Every other test of this rule asserts the *negative* half — that a citation
//! `fmt` will not wrap is not reported (`tests_kind_index_entry_form.rs`). That
//! half cannot catch the failure this file exists for: a finding that fires on
//! something `fmt` skips, which answers `rewrote 0 lines` and leaves a
//! repository permanently red. Two such cases shipped and were caught by hand
//! (an unmarked token off strict mode, and a citation naming a section no
//! declaration declares); this asserts the round trip instead of enumerating
//! the ways it can break.
//!
//! `--write` is the whole point, so these run the real binary against a real
//! tree rather than the in-process checker.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn run_grund(args: &[&str], cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_grund"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("spawn grund")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// A project whose `FS` kind is a folder with the default `README.md` index
/// (§FS-config.3.4), holding one declaration with one citable section. `index`
/// is left at its default, so the index rule applies.
///
/// `strict` picks the citation form the repository recognizes
/// (§FS-config.3.1): off strict mode a bare token is a citation, which is where
/// `check` and `fmt` have the most room to disagree.
fn build_fixture(name: &str, strict: bool, index_body: &str) -> PathBuf {
    build_fixture_with_home(name, strict, index_body, "FS-001-login.md")
}

/// Same as [`build_fixture`], but the declaration's home file name is given
/// explicitly rather than assumed to match the ID — the grund#131 shape needs a
/// name that *extends* the ID (`FS-001-login-a.md`) so the link `fmt` writes
/// puts an ID-shaped token in its own destination.
fn build_fixture_with_home(name: &str, strict: bool, index_body: &str, home: &str) -> PathBuf {
    let dir = manifest_dir().join("target/index-round-trip").join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("docs/specs")).expect("create specs dir");
    fs::write(
        dir.join("grund.toml"),
        format!(
            "grund_config_version = 1\nproject_name = \"round-trip\"\n\n\
             [reference]\nmarker = \"§\"\nstrict = {strict}\n\n\
             [[kinds]]\nkind = \"FS\"\nfolder = \"docs/specs\"\n\n\
             [scan]\ninclude = [\"docs\"]\n\n\
             [fmt.cross_refs]\nenabled = true\nanchor_format = \"github\"\n"
        ),
    )
    .expect("write grund.toml");
    fs::write(
        dir.join("docs/specs").join(home),
        "# FS-001-login: a user logs in\n\nLead.\n\n## 1. Behaviour\n\nText.\n",
    )
    .expect("write declaration");
    fs::write(dir.join("docs/specs/README.md"), index_body).expect("write README.md");
    dir
}

/// §FS-check.4.6 / §FS-list.2 / §FS-show.2.3: the issue #133 shape. The index
/// directly enrolls the source declaration, with no Markdown stub to become a
/// second scanner record or a query home.
fn build_external_inline_fixture() -> PathBuf {
    let dir = manifest_dir()
        .join("target/index-round-trip")
        .join("external-inline-enrollment");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("docs/architecture")).expect("create architecture dir");
    fs::create_dir_all(dir.join("src")).expect("create source dir");
    fs::write(
        dir.join("grund.toml"),
        "grund_config_version = 1\nproject_name = \"external-inline\"\n\n\
         [[kinds]]\nkind = \"AR\"\nfolder = \"docs/architecture\"\n\n\
         [scan]\ninclude = [\"docs\", \"src\"]\n",
    )
    .expect("write grund.toml");
    fs::write(
        dir.join("src/bus.rs"),
        "/// AR-001-bus: The in-process event bus\n\
         ///\n\
         /// Broadcasts to subscribers in registration order.\n\
         fn bus() {}\n",
    )
    .expect("write source declaration");
    fs::write(
        dir.join("docs/architecture/README.md"),
        "# Architecture\n\n- [§AR-001-bus](../../src/bus.rs)\n",
    )
    .expect("write index");
    dir
}

fn has_unlinked_entry(output: &Output) -> bool {
    stdout(output).contains("is not a link")
}

/// The round trip, for an index entry `fmt` does rewrite: `check` names the
/// error, the named command clears it, and the second `check` is silent about
/// it. The `fmt` run must also report that it wrote something — a pass that
/// answers `rewrote 0 lines` while `check` stays red is exactly the state
/// §FS-check.3.17's licence under §REQ-backwards-compatibility.3 forbids.
fn assert_round_trip(dir: &Path, case: &str) {
    let before = run_grund(&["check", "."], dir);
    assert!(
        has_unlinked_entry(&before),
        "{case}: expected an unlinked-index-entry error to start from, got: {}",
        stdout(&before)
    );

    let fmt = run_grund(&["fmt", "--write", "."], dir);
    assert_eq!(fmt.status.code(), Some(0), "{case}: fmt failed");
    assert!(
        !stdout(&fmt).contains("rewrote 0 lines"),
        "{case}: `check` named `grund fmt --write` as the fix and it wrote nothing: {}",
        stdout(&fmt)
    );

    let after = run_grund(&["check", "."], dir);
    assert!(
        !has_unlinked_entry(&after),
        "{case}: the fix command ran and the error survived it: {}",
        stdout(&after)
    );
}

/// A marker-prefixed bare ID — the ordinary case the error is written for.
#[test]
fn fmt_write_clears_a_bare_entry() {
    let dir = build_fixture(
        "bare_entry",
        true,
        "# Specs\n\n- §FS-001-login — a user logs in\n",
    );
    assert_round_trip(&dir, "bare entry");
}

/// The same, off strict mode. The bare form is recognized here whether or not
/// it carries the marker, so the predicate has to keep the two apart: this one
/// is marked and must round-trip.
#[test]
fn fmt_write_clears_a_marked_bare_entry_off_strict_mode() {
    let dir = build_fixture(
        "bare_entry_loose",
        false,
        "# Specs\n\n- §FS-001-login — a user logs in\n",
    );
    assert_round_trip(&dir, "marked bare entry, strict = false");
}

/// grund#131: a declaration whose home file name extends its own ID
/// (`FS-001-login` homed in `FS-001-login-a.md`). Off strict mode, the token
/// `fmt --cross-refs` writes into the link destination (`FS-001-login-a`) must
/// not become a citation of its own — the round trip has to clear the unlinked
/// entry without leaving behind a new `unknown reference` (§FS-check.1.1).
#[test]
fn fmt_write_clears_a_bare_entry_whose_home_extends_the_id() {
    let dir = build_fixture_with_home(
        "bare_entry_home_extends_id",
        false,
        "# Specs\n\n- §FS-001-login — a user logs in\n",
        "FS-001-login-a.md",
    );

    let before = run_grund(&["check", "."], &dir);
    assert!(
        has_unlinked_entry(&before),
        "expected an unlinked-index-entry error to start from, got: {}",
        stdout(&before)
    );

    let fmt = run_grund(&["fmt", "--write", "."], &dir);
    assert_eq!(fmt.status.code(), Some(0), "fmt failed");
    assert!(
        !stdout(&fmt).contains("rewrote 0 lines"),
        "`check` named `grund fmt --write` as the fix and it wrote nothing: {}",
        stdout(&fmt)
    );

    let after = run_grund(&["check", "."], &dir);
    assert_eq!(
        after.status.code(),
        Some(0),
        "the token `fmt` just wrote into the link destination must not dangle: {}",
        stdout(&after)
    );
    assert!(
        !stdout(&after).contains("unknown reference"),
        "the token `fmt` just wrote into the link destination must not dangle: {}",
        stdout(&after)
    );
}

/// A marked citation of a section that exists. `fmt` computes the section's
/// anchor (§FS-fmt.6.2), so this is an entry and must round-trip — the
/// counterpart to `a_dangling_section_is_not_an_entry` below, which is the same
/// shape with a section that does not.
#[test]
fn fmt_write_clears_a_bare_entry_naming_a_section() {
    let dir = build_fixture(
        "bare_entry_section",
        true,
        "# Specs\n\n- §FS-001-login.1 — how a user logs in\n",
    );
    assert_round_trip(&dir, "bare entry with a section");
}

/// The negative half, asserted through the same door: a citation naming a
/// section no declaration declares has no link target, so `fmt` skips the line
/// (§FS-fmt.6.2). `check` must not name it under §FS-check.3.17 — the tree is
/// already red for the missing section itself (§FS-check.3.2), and a second
/// finding here would name a command that answers `rewrote 0 lines`.
#[test]
fn a_dangling_section_is_not_an_entry() {
    let dir = build_fixture(
        "dangling_section",
        false,
        "# Specs\n\n- §FS-001-login.9 — a section that is not there\n",
    );

    let before = run_grund(&["check", "."], &dir);
    assert!(
        stdout(&before).contains("missing section"),
        "expected the missing-section error: {}",
        stdout(&before)
    );
    assert!(
        !has_unlinked_entry(&before),
        "`grund fmt --write` cannot wrap a citation with no link target, so \
         §FS-check.3.17 must not name it: {}",
        stdout(&before)
    );

    let fmt = run_grund(&["fmt", "--write", "."], &dir);
    assert!(
        stdout(&fmt).contains("rewrote 0 lines"),
        "the fixture is only meaningful while `fmt` leaves this line alone: {}",
        stdout(&fmt)
    );
}

/// The canonical external link is navigation rather than use, while declaration
/// consumers still have exactly one home — the source file. This runs the real
/// binary so the result cannot be an artifact of the checker's internal model.
#[test]
fn an_external_inline_enrollment_keeps_the_source_as_the_cli_home() {
    let dir = build_external_inline_fixture();

    let check = run_grund(&["check", "."], &dir);
    let check_stdout = stdout(&check);
    assert_eq!(check.status.code(), Some(0));
    assert!(
        check_stdout.contains("declared but never cited: AR-001-bus"),
        "the enrollment is navigation, not use: {check_stdout}"
    );
    assert!(
        !check_stdout.contains("not listed"),
        "the canonical link enrolls and lists the declaration: {check_stdout}"
    );

    let list = run_grund(&["list", "."], &dir);
    let list_stdout = stdout(&list);
    assert_eq!(list.status.code(), Some(0));
    assert!(
        list_stdout.contains("AR-001-bus") && list_stdout.contains("src/bus.rs:1"),
        "list must name the source declaration: {list_stdout}"
    );

    let unused = run_grund(&["list", ".", "--unused"], &dir);
    assert!(
        stdout(&unused).contains("src/bus.rs:1"),
        "list --unused applies the same navigation carve-out as check: {}",
        stdout(&unused)
    );

    let show = run_grund(&["show", "AR-001-bus", "."], &dir);
    assert_eq!(show.status.code(), Some(0));
    assert!(
        stdout(&show).contains("Broadcasts to subscribers in registration order."),
        "show must read the source doc-comment: {}",
        stdout(&show)
    );
}
