/// The exact public code vocabulary accepted by `grund check --only` and
/// `--ignore`, kept sorted for deterministic help output (§FS-errors.5).
#[doc(hidden)]
pub const CHECK_FINDING_CODES: &[&str] = &[
    "agents-init",
    "broken-stub",
    "dangling",
    "declaration-near-miss",
    "deprecated-config-location",
    "discouraged-citation",
    "duplicate",
    "duplicate-section",
    "empty-citation-obligation",
    "empty-scan",
    "escaped-citation-resolves",
    "forbidden-citation",
    "full-scope-ignored",
    "inline-citation-style",
    "invalid-value-binding",
    "invalid-value-declaration",
    "io",
    "misplaced-declaration",
    "missing-citation",
    "missing-index-entry",
    "missing-section",
    "missing-snapshot",
    "nothing-recognized",
    "optional-member-absent",
    "orphan-section",
    "out-of-scope-dangling",
    "out-of-scope-missing-section",
    "out-of-scope-shorthand-citation",
    "out-of-scope-unknown-project",
    "oversized-lead",
    "redundant-config",
    "section-heading-level",
    "section-outside-declaration",
    "shorthand-citation",
    "shorthand-numeric-run",
    "suggested-citation",
    "ungrounded",
    "unknown-project",
    "unlinked-index-entry",
    "unlisted-workspace-block",
    "unmarked-heading",
    "unused",
    "value-mismatch",
];

/// Repeatable exact-code selection for the two CLI check adapters. The
/// structured Rust API deliberately does not carry this presentation query, so
/// API and LSP callers continue to receive the complete report (§FS-check.1).
#[doc(hidden)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CheckFindingSelection {
    only: BTreeSet<String>,
    ignore: BTreeSet<String>,
}

impl CheckFindingSelection {
    /// Add one selector value after applying the pre-scan CLI validation and
    /// exact error vocabulary from §FS-cli.4.
    pub fn add_only(&mut self, value: &str) -> Result<()> {
        validate_check_finding_code("--only", value)?;
        self.only.insert(value.to_string());
        Ok(())
    }

    /// Add one ignored code; repetitions collapse into the specified set
    /// semantics (§FS-check.1).
    pub fn add_ignore(&mut self, value: &str) -> Result<()> {
        validate_check_finding_code("--ignore", value)?;
        self.ignore.insert(value.to_string());
        Ok(())
    }

    /// Decide whether a completed check's diagnostic enters the selected
    /// report. Ignore wins over only, while `io` cannot be hidden because it
    /// marks an incomplete scan (§FS-check.2).
    pub fn retains(&self, code: &str) -> bool {
        code == "io"
            || (self.only.is_empty() || self.only.contains(code)) && !self.ignore.contains(code)
    }
}

fn validate_check_finding_code(flag: &str, value: &str) -> Result<()> {
    if value.is_empty() {
        return Err(anyhow!("{flag} requires a finding code"));
    }
    let valid_shape = value
        .split('-')
        .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit()));
    if !valid_shape {
        return Err(anyhow!(
            "invalid finding code \"{value}\" (expected lowercase kebab-case)"
        ));
    }
    if CHECK_FINDING_CODES.binary_search(&value).is_err() {
        return Err(anyhow!(
            "unknown check finding code \"{value}\"; run `grund check --help` for supported codes"
        ));
    }
    Ok(())
}
