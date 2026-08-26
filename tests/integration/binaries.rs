//! §AR-bindings.1 — the two binaries the integration tests drive. Cargo names
//! a package's own binaries to its tests and nobody else's, so a test that
//! needs both `grund` and `grund-lsp` finds them beside itself in the profile
//! directory it was built into, and builds one on demand when a partial
//! invocation (`cargo test -p grund-integration-tests` on a fresh tree) has not
//! produced it yet. `cargo test --workspace --all-targets` always has.
#![allow(dead_code)]

use std::path::PathBuf;
use std::process::Command;

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `target/<profile>/`, read off the running test binary's own location.
fn profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current test binary");
    let deps = exe.parent().expect("deps dir");
    assert_eq!(
        deps.file_name().and_then(|name| name.to_str()),
        Some("deps"),
        "unexpected test binary location {}",
        exe.display()
    );
    deps.parent().expect("profile dir").to_path_buf()
}

fn binary(package: &str, name: &str) -> PathBuf {
    let dir = profile_dir();
    let path = dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    if !path.is_file() {
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let mut build = Command::new(cargo);
        build
            .args(["build", "-p", package, "--locked"])
            .current_dir(repo_root());
        if dir.file_name().and_then(|name| name.to_str()) == Some("release") {
            build.arg("--release");
        }
        let status = build
            .status()
            .unwrap_or_else(|err| panic!("run cargo build -p {package}: {err}"));
        assert!(status.success(), "cargo build -p {package} failed");
    }
    assert!(path.is_file(), "no {name} binary at {}", path.display());
    path
}

pub fn grund() -> PathBuf {
    binary("grund", "grund")
}

pub fn grund_lsp() -> PathBuf {
    binary("grund-lsp", "grund-lsp")
}
