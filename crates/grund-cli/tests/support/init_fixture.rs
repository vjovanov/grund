//! The fixture every `grund init` integration suite builds on: a scratch target
//! directory under `target/init-tests/` and one call into the built binary.
//!
//! Shared rather than copied because the suites that use it are split by
//! scenario, not by fixture — `init.rs` covers the scaffold and its report,
//! `init_agent_entrypoints.rs` covers which entrypoint files a run selects
//! (§FS-init.2.1) — and a second copy of the builder would let the two drift
//! into testing subtly different trees. The refused-target cases keep their own
//! fixture: those build outside this repository, which is the condition under
//! test (§FS-init.1.2).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The repository root, used as the child's working directory so a relative
/// path in a message resolves the way a user's run would.
pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A fresh, empty target directory named after the case, inside this
/// repository's own tree — which satisfies the version-control rule
/// (§FS-init.1.2) without any fixture having to fake a marker.
pub fn workdir(suffix: &str) -> PathBuf {
    let dir = manifest_dir().join("target/init-tests").join(suffix);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create workdir");
    dir
}

pub fn run_grund<P: AsRef<Path>>(args: &[&str], cwd: P) -> Output {
    Command::new(env!("CARGO_BIN_EXE_grund"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("spawn grund")
}
