// The stderr-conciseness half of the case runner: judging a non-zero case's
// `expected.stderr` against the shape the JSON format promises rather than
// against its serialized length. Split out of `case_runner.rs` along the same
// seam as `case_symlinks.rs` — included into the same module, so both halves
// share `case_name` and the manifest readers.

/// Whether a case's command selects `--format json`. Decided from the
/// expanded argument vector rather than the raw `command.args` text, because
/// the corpus spells the flag both ways: `--format=json` in one argument, or
/// `--format` and `json` as two.
fn command_selects_json(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--format=json")
        || args
            .windows(2)
            .any(|pair| pair[0] == "--format" && pair[1] == "json")
}

/// A non-zero case's `expected.stderr` must stay a concise diagnostic
/// (`tests/e2e/README.md`, "Error output is part of the contract"). Reads the
/// case's `expected.exit` — a clean case has nothing to judge — then hands the
/// judgement itself to [`assert_stderr_is_concise`], which touches no file and
/// is what the self-tests at the bottom of this file drive directly.
fn assert_expected_errors_are_concise(case: &Path, name: &str, args: &[String], stderr: &str) {
    if read_to_string(case.join("expected.exit")).trim() == "0" {
        return;
    }
    assert_stderr_is_concise(name, command_selects_json(args), stderr);
}

/// The judgement, isolated from the filesystem. A `--format json` case's
/// stderr is not uniformly JSON (§FS-errors.5): a launch-time `error: …` line
/// and a run-level `warning:` / `hint:` stay text and keep the plain 180-byte
/// cap. Only a line that opens a JSON object is parsed and judged by its
/// `message` field instead of its serialized length — the scaffolding around
/// it (§FS-distribution.3.0's `severity`, `path`, `line`, `code`, `sites`) is
/// fixed cost the conciseness policy was never about.
fn assert_stderr_is_concise(name: &str, json_case: bool, stderr: &str) {
    assert!(
        !stderr.contains("error(s)") && !stderr.contains("warning(s)"),
        "{name}: stderr should not include aggregate summaries"
    );
    for line in stderr.lines().filter(|line| !line.trim().is_empty()) {
        if json_case && line.starts_with('{') {
            assert_json_diagnostic_is_concise(name, line);
        } else {
            assert!(
                line.len() <= 180,
                "{name}: stderr line is too long for a concise diagnostic: {line}"
            );
        }
    }
}

/// Parse one `{`-line as the diagnostic object §FS-errors.5 / §FS-distribution.3.0
/// promise, assert its shape, and cap its `message`. Every rejection names the
/// case and the whole line, exactly as the plain-text cap does.
fn assert_json_diagnostic_is_concise(name: &str, line: &str) {
    let value: serde_json::Value = serde_json::from_str(line)
        .unwrap_or_else(|err| panic!("{name}: stderr line is not valid JSON ({err}): {line}"));
    let object = value
        .as_object()
        .unwrap_or_else(|| panic!("{name}: stderr JSON line is not an object: {line}"));

    let mut actual_keys: Vec<&str> = object.keys().map(String::as_str).collect();
    actual_keys.sort_unstable();
    let mut expected_keys = ["severity", "path", "line", "code", "message", "sites"];
    expected_keys.sort_unstable();
    assert_eq!(
        actual_keys, expected_keys,
        "{name}: stderr JSON line has the wrong key set: {line}"
    );

    let severity = object.get("severity").and_then(|v| v.as_str());
    assert!(
        matches!(severity, Some("error") | Some("warning")),
        "{name}: stderr JSON `severity` must be \"error\" or \"warning\": {line}"
    );
    assert!(
        object
            .get("code")
            .and_then(|v| v.as_str())
            .is_some_and(|code| !code.is_empty()),
        "{name}: stderr JSON `code` must be a non-empty string: {line}"
    );
    let message = object
        .get("message")
        .and_then(|v| v.as_str())
        .filter(|message| !message.is_empty())
        .unwrap_or_else(|| {
            panic!("{name}: stderr JSON `message` must be a non-empty string: {line}")
        });
    assert!(
        message.len() <= 180,
        "{name}: stderr JSON `message` is too long for a concise diagnostic ({} bytes): {line}",
        message.len()
    );
    assert!(
        matches!(
            object.get("path"),
            Some(serde_json::Value::Null) | Some(serde_json::Value::String(_))
        ),
        "{name}: stderr JSON `path` must be null or a string: {line}"
    );
    assert!(
        matches!(object.get("line"), Some(v) if v.is_null() || v.is_number()),
        "{name}: stderr JSON `line` must be null or a number: {line}"
    );
    assert_json_sites_are_well_formed(name, line, object.get("sites"));
}

/// `sites` is `null`, or a list of `{ path, line }` locating every site of a
/// multi-site finding (§FS-distribution.3.0) — unlike the diagnostic's own
/// `path` / `line`, a site always has both: it exists to say where.
fn assert_json_sites_are_well_formed(name: &str, line: &str, sites: Option<&serde_json::Value>) {
    match sites {
        Some(serde_json::Value::Null) => {}
        Some(serde_json::Value::Array(entries)) => {
            for site in entries {
                let site = site.as_object().unwrap_or_else(|| {
                    panic!("{name}: stderr JSON `sites` entry is not an object: {line}")
                });
                let mut keys: Vec<&str> = site.keys().map(String::as_str).collect();
                keys.sort_unstable();
                assert_eq!(
                    keys,
                    ["line", "path"],
                    "{name}: stderr JSON `sites` entry has the wrong key set: {line}"
                );
                assert!(
                    matches!(site.get("path"), Some(serde_json::Value::String(_))),
                    "{name}: stderr JSON `sites[].path` must be a string: {line}"
                );
                assert!(
                    matches!(site.get("line"), Some(v) if v.is_number()),
                    "{name}: stderr JSON `sites[].line` must be a number: {line}"
                );
            }
        }
        _ => panic!("{name}: stderr JSON `sites` must be null or an array: {line}"),
    }
}

#[cfg(test)]
mod stderr_concise_tests {
    use super::assert_stderr_is_concise;
    use std::panic::{self, AssertUnwindSafe};

    /// Runs the judgement and returns the panic message, or panics itself if
    /// the stderr under test was wrongly accepted.
    fn rejects(json_case: bool, stderr: &str) -> String {
        match panic::catch_unwind(AssertUnwindSafe(|| {
            assert_stderr_is_concise("case", json_case, stderr)
        })) {
            Ok(()) => panic!("expected stderr to be rejected, but it passed: {stderr}"),
            Err(payload) => payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_string()))
                .unwrap_or_default(),
        }
    }

    #[test]
    fn json_case_accepts_a_representative_diagnostic_object() {
        let message = "ambiguous section: FS-001-login.1 (declared at \
                        docs/functional-spec/FS-001-login.md:5, \
                        docs/functional-spec/FS-001-login.md:9)";
        assert!(message.len() <= 180, "fixture message grew past the cap");
        let line = format!(
            "{{\"severity\":\"error\",\"path\":null,\"line\":null,\
              \"code\":\"ambiguous-section\",\"message\":\"{message}\",\"sites\":null}}"
        );
        assert!(line.len() > 180, "fixture line no longer exceeds the cap");
        assert_stderr_is_concise("case", true, &line);
    }

    #[test]
    fn json_case_rejects_a_message_over_the_cap() {
        let message = "x".repeat(181);
        let line = format!(
            "{{\"severity\":\"error\",\"path\":null,\"line\":null,\
              \"code\":\"c\",\"message\":\"{message}\",\"sites\":null}}"
        );
        let panic_message = rejects(true, &line);
        assert!(panic_message.contains("message"), "{panic_message}");
    }

    #[test]
    fn json_case_rejects_a_line_with_an_extra_key() {
        let line = "{\"severity\":\"error\",\"path\":null,\"line\":null,\"code\":\"c\",\
                     \"message\":\"m\",\"sites\":null,\"extra\":1}";
        let panic_message = rejects(true, line);
        assert!(panic_message.contains("key set"), "{panic_message}");
    }

    #[test]
    fn json_case_rejects_a_line_with_a_missing_key() {
        let line = "{\"severity\":\"error\",\"path\":null,\"code\":\"c\",\
                     \"message\":\"m\",\"sites\":null}";
        let panic_message = rejects(true, line);
        assert!(panic_message.contains("key set"), "{panic_message}");
    }

    #[test]
    fn json_case_rejects_malformed_json() {
        let panic_message = rejects(true, "{\"severity\":\"error\"");
        assert!(panic_message.contains("not valid JSON"), "{panic_message}");
    }

    #[test]
    fn json_case_still_caps_a_text_line() {
        let line = format!("error: {}", "x".repeat(180));
        assert!(line.len() > 180, "fixture line no longer exceeds the cap");
        let panic_message = rejects(true, &line);
        assert!(panic_message.contains("too long"), "{panic_message}");
    }

    #[test]
    fn text_case_rejects_a_line_over_the_cap() {
        let line = "x".repeat(223);
        let panic_message = rejects(false, &line);
        assert!(panic_message.contains("too long"), "{panic_message}");
    }
}
