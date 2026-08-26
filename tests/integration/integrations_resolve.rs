//! §FS-integrations.4 — the headless half of `scripts/try-integrations.sh`,
//! run as a gate: install the resolver into a sandbox HOME from the binary
//! under test and resolve every citation form the clients hand it against this
//! repository — plain, sectioned, bare, punctuation-swept, workspace-qualified,
//! unknown, and `path:line` locations (§FS-integrations.3.1). The script is a
//! manual testbed for the clickable clients; its resolver checks need no
//! terminal, so nothing excuses them from CI. Unix only: the script is `bash`
//! and the resolver it installs is the Unix shell integration.
#![cfg(unix)]

#[path = "binaries.rs"]
mod binaries;

use std::fs;
use std::process::Command;

#[cfg(unix)]
#[test]
fn the_resolver_resolves_every_citation_form_headlessly() {
    let repo = binaries::repo_root();
    let sandbox = repo.join("target/integration-work/integrations-sandbox");
    let _ = fs::remove_dir_all(&sandbox);
    let output = Command::new("bash")
        .arg(repo.join("scripts/try-integrations.sh"))
        .arg("resolve")
        .arg("--binary")
        .arg(binaries::grund())
        .arg("--sandbox")
        .arg(&sandbox)
        .current_dir(&repo)
        .output()
        .expect("run scripts/try-integrations.sh");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "try-integrations.sh resolve exited with {}\nstdout:\n{stdout}\nstderr:\n{stderr}",
        output.status
    );
    let failed = stdout
        .lines()
        .filter(|line| line.trim_start().starts_with("FAIL"))
        .collect::<Vec<_>>();
    assert!(
        failed.is_empty(),
        "resolver checks failed:\n{}\n\nfull output:\n{stdout}",
        failed.join("\n")
    );
    let passed = stdout
        .lines()
        .filter(|line| line.trim_start().starts_with("PASS"))
        .count();
    assert!(
        passed >= 8,
        "expected the resolver checks to run; saw {passed} PASS line(s):\n{stdout}"
    );
}
