/// The complete `grund check --help` discovery surface: all inputs, selected
/// report semantics, the public code catalog, and recovery examples (§FS-cli.3).
fn print_check_help() {
    println!("grund check — validate every ID citation across the repo.");
    println!();
    println!(
        "Usage:  grund check [PATH] [--full] [--require-grounding] [--suggestions] [--only CODE] [--ignore CODE] [--format text|json]"
    );
    println!();
    println!(
        "PATH defaults to `.`; config is discovered by walking up from it — the root `grund.toml` is the home, `.agents/grund.toml` a deprecated fallback."
    );
    println!(
        "With no config, grund scans `docs/`, `e2e/`, and `src/`; set `[scan] include` to widen it."
    );
    println!("Pointing grund at an explicit PATH scans exactly that file or directory.");
    println!("Path validation is explicit; `grund PATH` is parsed as an ID query.");
    println!();
    println!("Options:");
    println!(
        "  --format text|json   text (default) prints `success` or `path:line: message`; json emits NDJSON."
    );
    println!(
        "  --full               also walk past [scan] include and report the references that resolve to nothing out there."
    );
    println!(
        "  --require-grounding  also require every source file to cite a declared ID ([reference] require_grounding; a [[kinds]] row that sets it false stays exempt)."
    );
    println!(
        "  --suggestions        also surface should/should-not citation-direction findings ([citations])."
    );
    println!(
        "  --only <code>        retain one exact finding code; repeat for a union (`--only=<code>` also works)."
    );
    println!(
        "  --ignore <code>      remove one exact finding code; repeat for a union; ignore wins over only (`--ignore=<code>` also works)."
    );
    println!();
    println!(
        "Findings go to stdout (the linter convention) — `grund check | …` and `grund check"
    );
    println!("--format json | jq` need no redirect. Only run-level `error:` / `warning:` lines");
    println!("(unreadable path, empty scan) go to stderr; a clean text run prints `success`.");
    println!();
    // §FS-check.2: state both the post-scan boundary and selected exit meaning.
    println!("Selection happens after the complete check. It filters errors, warnings, and enabled");
    println!("suggestions by exact code; operational failures remain visible and exit 2. Exit 0 means");
    println!("the selected report has no errors, not that the unselected repository is clean.");
    println!();
    println!("Supported check finding codes:");
    for code in CHECK_FINDING_CODES {
        println!("  {code}");
    }
    println!();
    println!(
        "Exit:  0 clean · 1 dangling / duplicate / unknown-section / ungrounded findings · 2 unreadable tree or CLI error."
    );
    println!();
    println!("Examples:");
    println!("  grund check              # check the whole repo");
    println!("  grund check docs/        # check one subtree");
    println!("  grund check --full       # plus dangling citations outside [scan] include");
    println!(
        "  grund check --ignore agents-init # ask whether the selected content report has errors"
    );
    println!("  grund check --format json | jq # machine-readable diagnostics for CI");
}
