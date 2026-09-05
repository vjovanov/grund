//! §FS-lsp.4 — the parity sweep that used to be called future hardening: for
//! every e2e case that is a plain `check` of a fixture carrying its own config,
//! the diagnostics `grund-lsp` publishes on `initialized` are exactly the
//! located findings `grund check --format json` prints for the same tree —
//! path, line, code, severity and message — so the server is provably the same
//! engine behind a different transport (§AR-lsp) and not a second one that can
//! drift. Cases the CLI refuses (exit `2`) and fixtures with no config at their
//! root are skipped and counted, never silently, and a floor on the number of
//! compared cases keeps the sweep from shrinking unnoticed. A CLI-level
//! `warning:` or `error:` line (§FS-errors.2.2) is stepped over rather than
//! compared: it is settled before a report exists, so neither surface can carry
//! it as a diagnostic.

#[path = "binaries.rs"]
mod binaries;
#[path = "corpus.rs"]
mod corpus;
#[path = "../../crates/grund-lsp/tests/support/mod.rs"]
mod support;

use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};
use support::{send_message, start_server, wait_for_exit};

/// Fewer compared cases than this is a broken sweep, not a smaller corpus: the
/// corpus compares well over a hundred, and no rewrite of a handful of cases
/// into other commands should trip it.
const MIN_COMPARED_CASES: usize = 80;

/// The two CLI-level message prefixes of §FS-errors.2.2. A launch-time
/// diagnostic keeps its raw text on stderr under `--format json` as well
/// (§FS-errors.5), so the reduction below has to step over one — and over
/// nothing else: any other line on either stream is output this sweep does not
/// understand, and a harness that swallowed it would stop being the guard it is
/// here to be.
const CLI_LEVEL_PREFIXES: [&str; 2] = ["error: ", "warning: "];

/// One located finding, in the shape both surfaces can be reduced to.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Finding {
    path: String,
    line: u64,
    severity: &'static str,
    code: String,
    message: String,
}

/// `grund check . --format json` from the fixture root: every located finding
/// on either stream (§FS-output-shapes.7), or `None` when the run is refused.
fn cli_findings(grund: &Path, root: &Path) -> Option<BTreeSet<Finding>> {
    let output = Command::new(grund)
        .args(["check", ".", "--format", "json"])
        .current_dir(root)
        .output()
        .expect("run grund check");
    if output.status.code() == Some(2) {
        return None;
    }
    let mut findings = BTreeSet::new();
    for stream in [&output.stdout, &output.stderr] {
        for line in String::from_utf8_lossy(stream).lines() {
            let Ok(value) = serde_json::from_str::<Value>(line) else {
                // §FS-errors.2.2: a CLI-level message is settled before a report
                // exists, so neither surface carries it as a finding — §FS-check.4.8's
                // and §FS-workspace.6.1's warnings both arrive here. Nothing else may.
                if CLI_LEVEL_PREFIXES
                    .iter()
                    .any(|prefix| line.starts_with(prefix))
                {
                    continue;
                }
                panic!(
                    "non-JSON line from grund check --format json in {}: {line}",
                    root.display()
                );
            };
            let Some(path) = value["path"].as_str() else {
                continue;
            };
            let severity = match value["severity"].as_str() {
                Some("error") => "error",
                Some("warning") => "warning",
                other => panic!("unexpected severity {other:?} in {line}"),
            };
            findings.insert(Finding {
                path: path.replace('\\', "/"),
                // The server anchors a line-less located finding on line 1.
                line: value["line"].as_u64().unwrap_or(1),
                severity,
                code: value["code"].as_str().unwrap_or_default().to_string(),
                message: value["message"].as_str().unwrap_or_default().to_string(),
            });
        }
    }
    Some(findings)
}

/// Everything the server publishes between `initialized` and the `shutdown`
/// response: messages are handled in order, so the diagnostics the handshake
/// pushed are all on the wire before the response to the request sent after it.
fn lsp_findings(root: &Path) -> BTreeSet<Finding> {
    let (mut child, mut stdin, receiver) = start_server(root);
    send_message(
        &mut stdin,
        json!({ "jsonrpc": "2.0", "id": 2, "method": "shutdown", "params": null }),
    );
    let canonical_root = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut findings = BTreeSet::new();
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let message = receiver.recv_timeout(remaining).unwrap_or_else(|_| {
            panic!("no shutdown response from grund-lsp for {}", root.display())
        });
        if message.get("id").and_then(Value::as_i64) == Some(2) {
            break;
        }
        if message.get("method").and_then(Value::as_str) != Some("textDocument/publishDiagnostics")
        {
            continue;
        }
        let uri = message["params"]["uri"].as_str().expect("diagnostic uri");
        let path = url::Url::parse(uri)
            .ok()
            .and_then(|url| url.to_file_path().ok())
            .unwrap_or_else(|| panic!("diagnostic uri is not a file path: {uri}"));
        // Canonical on both sides: Windows spells a canonical path with a
        // `\\?\` prefix that a URI-derived one lacks.
        let path = fs::canonicalize(&path).unwrap_or(path);
        let relative = path
            .strip_prefix(&canonical_root)
            .or_else(|_| path.strip_prefix(root))
            .unwrap_or_else(|_| {
                panic!(
                    "diagnostic path {} is outside {}",
                    path.display(),
                    root.display()
                )
            });
        for diagnostic in message["params"]["diagnostics"]
            .as_array()
            .expect("diagnostics array")
        {
            let severity = match diagnostic["severity"].as_u64() {
                Some(1) => "error",
                Some(2) => "warning",
                other => panic!("unexpected diagnostic severity {other:?}"),
            };
            findings.insert(Finding {
                path: relative.to_string_lossy().replace('\\', "/"),
                line: diagnostic["range"]["start"]["line"]
                    .as_u64()
                    .expect("start line")
                    + 1,
                severity,
                code: diagnostic["code"].as_str().unwrap_or_default().to_string(),
                message: diagnostic["message"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }
    }
    send_message(
        &mut stdin,
        json!({ "jsonrpc": "2.0", "method": "exit", "params": null }),
    );
    wait_for_exit(&mut child);
    findings
}

#[test]
fn lsp_diagnostics_are_the_cli_findings_for_every_plain_check_case() {
    let repo = binaries::repo_root();
    let grund = binaries::grund();
    let _ = support::SERVER_BINARY.set(binaries::grund_lsp());
    let corpus::Selection {
        cases,
        other_commands,
        no_config,
    } = corpus::plain_check_cases(&repo);
    let mut compared = 0;
    let mut refused = Vec::new();
    let mut mismatches = Vec::new();
    for case in &cases {
        let Some(cli) = cli_findings(&grund, &case.root) else {
            refused.push(case.name.clone());
            continue;
        };
        let lsp = lsp_findings(&case.root);
        compared += 1;
        if cli != lsp {
            let only_cli = cli.difference(&lsp).collect::<Vec<_>>();
            let only_lsp = lsp.difference(&cli).collect::<Vec<_>>();
            mismatches.push(format!(
                "{}:\n  only the CLI reports: {only_cli:#?}\n  only the LSP reports: {only_lsp:#?}",
                case.name
            ));
        }
    }
    eprintln!(
        "lsp/cli parity: {compared} case(s) compared, {} refused by the CLI (exit 2), \
         {} with no config at the fixture root, {other_commands} not a plain check",
        refused.len(),
        no_config.len()
    );
    assert!(
        mismatches.is_empty(),
        "{} case(s) where grund-lsp and grund check disagree:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
    assert!(
        compared >= MIN_COMPARED_CASES,
        "only {compared} case(s) compared; the sweep expects at least {MIN_COMPARED_CASES} \
         (refused: {refused:?}; no config: {no_config:?})"
    );
}
