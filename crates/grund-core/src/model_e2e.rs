/// Scanner/catalog records for executable scenario declarations
/// (§AR-scanner.6, §AR-core-module-layout.1).

/// An `e2e/cases/<name>/` directory treated as an `E2E-<name>` declaration
/// (§AR-scanner.6) — its `command.args`, `expected.exit`, and fixture file list
/// are what `grund E2E-<name>` renders (§FS-show.2.4).
#[derive(Debug, Clone)]
pub struct E2eCase {
    pub dir: PathBuf,
    pub args: Vec<String>,
    pub expected_exit: i32,
    pub fixtures: Vec<PathBuf>,
    pub spec_refs: Vec<E2eSpecRef>,
}

/// A non-empty `spec.refs` manifest line from an E2E case (§AR-scanner.6).
/// It is evidence for E2E citation-direction obligations (§FS-config.3.9), not a
/// normal citation site, so it does not produce dangling-ref findings: an E2E
/// case grounds in the *layer* a `spec.refs` entry names, and entries are
/// deliberately allowed to use idealized, not-locally-resolvable IDs, so only
/// `kind` (plus `namespace`) is retained.
#[derive(Debug, Clone)]
pub struct E2eSpecRef {
    pub namespace: Option<String>,
    pub kind: String,
}
