//! §AR-bindings.2: the published CLI and deprecated core process adapter keep
//! §FS-errors.4 all-channel text ordering byte-identical.

mod binaries;

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn run(binary: impl AsRef<Path>, root: &Path) -> Output {
    Command::new(binary.as_ref())
        .arg("check")
        .arg(root)
        .arg("--suggestions")
        .output()
        .expect("run check adapter")
}

#[test]
fn all_channel_text_is_identical_in_both_process_adapters() {
    let case = binaries::repo_root().join("tests/e2e/cases/check-text-severity-groups");
    let expected = fs::read(case.join("expected.stdout")).expect("read expected stdout");
    let public = run(binaries::grund(), &case.join("repo"));
    let compat = run(binaries::grund_core_compat(), &case.join("repo"));

    assert_eq!(public.status.code(), Some(1));
    assert_eq!(public.stdout, expected);
    assert_eq!(public.stderr, b"");
    assert_eq!(compat.status.code(), public.status.code());
    assert_eq!(compat.stdout, public.stdout);
    assert_eq!(compat.stderr, public.stderr);
}
