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
//! something `fmt` skips, which answers `rewrote 0 references` and leaves a
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
    let dir = manifest_dir().join("target/index-round-trip").join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("docs/specs")).expect("create specs dir");
    fs::write(
        dir.join("grund.toml"),
        format!(
            "grund_config_version = 1\nproject_name = \"round-trip\"\n\n\
             [reference]\nmarker = \"§\"\nstrict = {strict}\n\n\
             [[kinds]]\nprefix = \"FS\"\nfolder = \"docs/specs\"\n\n\
             [scan]\ninclude = [\"docs\"]\n\n\
             [fmt.cross_refs]\nenabled = true\nanchor_format = \"github\"\n"
        ),
    )
    .expect("write grund.toml");
    fs::write(
        dir.join("docs/specs/FS-001-login.md"),
        "# FS-001-login: a user logs in\n\nLead.\n\n## 1. Behaviour\n\nText.\n",
    )
    .expect("write FS-001-login.md");
    fs::write(dir.join("docs/specs/README.md"), index_body).expect("write README.md");
    dir
}

fn has_unlinked_entry(output: &Output) -> bool {
    stdout(output).contains("is not a link")
}

/// The round trip, for an index entry `fmt` does rewrite: `check` names the
/// error, the named command clears it, and the second `check` is silent about
/// it. The `fmt` run must also report that it wrote something — a pass that
/// answers `rewrote 0 references` while `check` stays red is exactly the state
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
        !stdout(&fmt).contains("rewrote 0 references"),
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
/// finding here would name a command that answers `rewrote 0 references`.
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
        stdout(&fmt).contains("rewrote 0 references"),
        "the fixture is only meaningful while `fmt` leaves this line alone: {}",
        stdout(&fmt)
    );
}
