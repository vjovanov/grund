/// Shared fixtures for the `tests_*` modules: temp-repo setup, config
/// builders, finding signatures, and the resolver harness. Split out of the
/// former single `mod tests` so each category's cases live in their own file
/// (§AR-core-module-layout.3).
#[cfg(test)]
mod tests_support {
    use super::*;

    pub(crate) fn test_root(name: &str) -> PathBuf {
        let unique = format!(
            "{}-{}-{:?}",
            name,
            std::process::id(),
            std::thread::current().id()
        );
        let dir = std::env::temp_dir().join("grund-lib-tests").join(unique);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create test root");
        dir
    }

    /// A test root as the *operating system* reports it, for fixtures whose
    /// expectations are compared against paths a subprocess printed. On macOS
    /// `$TMPDIR` lives under `/var/folders/…`, which is a symlink to
    /// `/private/var/folders/…`; a shell resolving its own working directory
    /// reports the physical form, so a logical root used as an expected prefix
    /// matches only as a substring and leaves a `/private` stub on the front of
    /// the result. Canonicalizing is a no-op wherever the two agree.
    #[cfg(unix)]
    pub(crate) fn physical_test_root(name: &str) -> PathBuf {
        let root = test_root(name);
        std::fs::canonicalize(&root).expect("canonicalize test root")
    }

    pub(crate) fn write(path: &Path, text: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create parent");
        }
        std::fs::write(path, text).expect("write fixture");
    }

    pub(crate) fn legacy_fs_folder_config(root: PathBuf) -> Config {
        let mut config = Config::default_for(root);
        for kind in &mut config.kinds {
            if kind.prefix == "FS" {
                kind.folder = Some("docs/functional-spec".to_string());
                kind.file = None;
            }
        }
        config
    }

    /// A tree with a configured inline note layout, gate still `off`
    /// (§FS-inline-citation-style.3.3). Shared because the classifier suite and
    /// the check suite configure the same two keys from opposite ends.
    pub(crate) fn layout_config(root: PathBuf, layout: &str) -> Config {
        let mut config = legacy_fs_folder_config(root);
        config.inline_note_layout = layout.to_string();
        config
    }

    /// A layout the check actually reads: the classifier records nothing while
    /// `inline_note_layout_check` is `off` (§FS-inline-citation-style.4.4), so a
    /// test that asks it a question has to turn the gate on.
    pub(crate) fn checked_layout_config(root: PathBuf, layout: &str) -> Config {
        let mut config = layout_config(root, layout);
        config.inline_note_layout_check = "error".to_string();
        config
    }

    pub(crate) fn canonical_test_path(path: &Path) -> PathBuf {
        std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
    }

    pub(crate) fn findings_signature(config: &Config, findings: &Findings) -> Vec<String> {
        let mut rows = Vec::new();
        for (id, declarations) in &findings.declarations {
            for declaration in declarations {
                rows.push(format!(
                    "decl|{}|{}|{}|{}|{}|{}|{}",
                    render_id(config, id),
                    sort_path_key(&declaration.file),
                    declaration.line,
                    declaration.heading_level,
                    declaration.is_stub,
                    declaration.title.as_deref().unwrap_or(""),
                    declaration
                        .defined_in
                        .as_ref()
                        .map(|path| format_path(path))
                        .unwrap_or_default()
                ));
                for (section, info) in &declaration.sections {
                    rows.push(format!(
                        "section|{}|{}|{}|{}|{}",
                        render_id(config, id),
                        section,
                        info.title,
                        info.line,
                        info.heading_level
                    ));
                }
                // §AR-scanner.2.2: a later heading claiming a recorded path is
                // kept beside the map, so the signature has to carry it too — a
                // signature blind to a recorded field cannot see it change.
                for (section, info) in &declaration.duplicate_sections {
                    rows.push(format!(
                        "duplicate-section|{}|{}|{}|{}|{}",
                        render_id(config, id),
                        section,
                        info.title,
                        info.line,
                        info.heading_level
                    ));
                }
                if let Some(case) = &declaration.e2e_case {
                    rows.push(format!(
                        "e2e|{}|{}|{}|{}|{}",
                        render_id(config, id),
                        sort_path_key(&case.dir),
                        case.expected_exit,
                        case.args.join(" "),
                        case.fixtures
                            .iter()
                            .map(|path| format_path(path))
                            .collect::<Vec<_>>()
                            .join(",")
                    ));
                }
            }
        }
        for citation in &findings.citations {
            rows.push(format!(
                "cite|{}|{}|{}|{}|{}|{}|{}|{}",
                citation.namespace.as_deref().unwrap_or(""),
                render_id(config, &citation.id),
                citation.section.as_deref().unwrap_or(""),
                sort_path_key(&citation.file),
                citation.line,
                citation.column,
                citation.has_marker,
                citation.text
            ));
        }
        for file in &findings.scanned_files {
            rows.push(format!("file|{}", sort_path_key(file)));
        }
        rows
    }

    pub(crate) fn scan_errors_signature(errors: Vec<ScanError>) -> Vec<String> {
        errors
            .into_iter()
            .map(|(path, message)| format!("{}|{}", sort_path_key(&path), message))
            .collect()
    }

    pub(crate) fn current_block() -> String {
        render_agents_append_block(
            "demo",
            &Config::default_for(PathBuf::from(".")),
            Path::new("."),
            true,
            ConversationSurface::Plain,
        )
    }

    pub(crate) fn current_marker() -> &'static str {
        "## Grounding with grund (v7)"
    }

    /// Run a just-written script, waiting out a kernel that still calls it busy.
    /// These tests `fs::write` a script, mark it executable, and exec it — while
    /// the rest of the suite runs in parallel. Any other test that spawns a
    /// process in the window where this file's write descriptor is open leaks
    /// that descriptor into its child, and exec then fails with `ETXTBSY` until
    /// the child exits. Nothing here can close another test's fd, so the honest
    /// fix is to retry rather than to fail a test that has found no defect.
    #[cfg(unix)]
    pub(crate) trait OutputRetryingBusy {
        fn output_unbusy(&mut self) -> std::process::Output;
    }

    #[cfg(unix)]
    impl OutputRetryingBusy for std::process::Command {
        fn output_unbusy(&mut self) -> std::process::Output {
            const ETXTBSY: i32 = 26;
            for _ in 0..100 {
                match self.output() {
                    Ok(output) => return output,
                    Err(err) if err.raw_os_error() == Some(ETXTBSY) => {
                        std::thread::sleep(std::time::Duration::from_millis(20));
                    }
                    Err(err) => panic!("run script: {err}"),
                }
            }
            panic!("script stayed busy for 2s; a leaked write descriptor never closed")
        }
    }

    /// Drive the real embedded `grund-open` against a mock `grund` that echoes a
    /// fixed `show --format json` object. Returns what the resolver handed the
    /// editor. `cwd` is where the click lands; `marker_token` is the clicked text.
    #[cfg(unix)]
    pub(crate) fn run_resolver(name: &str, cwd_suffix: &str, token: &str, json: &str) -> (String, String) {
        use std::os::unix::fs::PermissionsExt;

        let root = physical_test_root(name);
        let bin = root.join("bin");
        std::fs::create_dir_all(&bin).expect("create mock bin");
        write(&root.join(".agents/grund.toml"), "[project]\n");
        let cwd = root.join(cwd_suffix);
        std::fs::create_dir_all(&cwd).expect("create cwd");
        let capture = root.join("opened-argument");
        let mock_grund = bin.join("grund");
        let opener = bin.join("opener");
        let resolver = root.join("grund-open");
        // Echo the resolution, and record the argv the resolver passed through so
        // the section suffix is provably preserved rather than stripped.
        write(
            &mock_grund,
            &format!(
                "#!/bin/sh\nprintf '%s\\n' \"$1\" > \"{}\"\nprintf '%s\\n' '{}'\n",
                root.join("grund-argv").display(),
                json
            ),
        );
        write(&opener, "#!/bin/sh\nprintf '%s\\n' \"$1\" > \"$CAPTURE\"\n");
        write(&resolver, GRUND_OPEN_RESOLVER);
        for path in [&mock_grund, &opener, &resolver] {
            let mut permissions = std::fs::metadata(path).unwrap().permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(path, permissions).unwrap();
        }
        let path = format!(
            "{}:{}",
            bin.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let output = std::process::Command::new(&resolver)
            .arg(token)
            .current_dir(&cwd)
            .env("PATH", &path)
            .env("GRUND_OPEN_CMD", &opener)
            .env("CAPTURE", &capture)
            .env_remove("EDITOR")
            .output_unbusy();
        assert!(
            output.status.success(),
            "resolver failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let opened = std::fs::read_to_string(&capture).unwrap().trim_end().to_string();
        let argv = std::fs::read_to_string(root.join("grund-argv"))
            .unwrap()
            .trim_end()
            .to_string();
        (opened.replace(&format!("{}/", root.display()), ""), argv)
    }

    /// A repo whose code moved out from under `[scan] include`: specs and `src/`
    /// are configured, `sim/` is not — the shape the issue behind
    /// §DF-check-full-scope reports.
    pub(crate) fn drifted_include_repo(name: &str) -> PathBuf {
        let root = test_root(name);
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n[scan]\ninclude = [\"docs\", \"src\"]\n",
        );
        write(
            &root.join("docs/functional-spec/FS-001-login.md"),
            "# FS-001-login: A user can log in\n\n## 1. Rules\n\nThe login behavior.\n",
        );
        write(&root.join("src/auth.rs"), "// Implements §FS-001-login.1\n");
        root
    }

    /// Every diagnostic in the `path:line: message` shape the text report
    /// prints (§FS-check.2.1), so a test can compare two runs as text.
    /// The whole tree scanned with `path_provided`, which is what a test means by
    /// "point grund at this fixture". Shared by every suite that asserts over
    /// `Findings` rather than over a rendered report.
    pub(crate) fn scan_findings(config: &Config, root: &Path) -> Findings {
        let (findings, _) = scan_tree(config, Some(root), true).expect("scan tree");
        findings
    }

    /// A report's errors as `code@line`. `Diagnostic` is not `Debug`, and a case
    /// asserting *which rules fired, and no others* wants exactly this much of it.
    pub(crate) fn error_codes(report: &CheckReport) -> Vec<String> {
        report
            .errors
            .iter()
            .map(|error| format!("{}@{}", error.code, error.line.unwrap_or(0)))
            .collect()
    }

    pub(crate) fn located_diagnostics<'a>(
        config: &Config,
        diagnostics: impl IntoIterator<Item = &'a Diagnostic>,
    ) -> Vec<String> {
        diagnostics
            .into_iter()
            .map(|diagnostic| {
                format!(
                    "{}:{}: {}",
                    diagnostic
                        .path
                        .as_ref()
                        .map(|path| display_path(config, path))
                        .unwrap_or_default(),
                    diagnostic.line.unwrap_or(0),
                    diagnostic.message
                )
            })
            .collect()
    }

    pub(crate) fn check_run(root: &Path, full: bool) -> CheckRun {
        run_check(root, true, false, full).expect("check run")
    }

    /// A symlink, for the cases that are about one. Unix only: creating one on
    /// Windows needs developer mode, and every caller is `#[cfg(unix)]` too.
    #[cfg(unix)]
    pub(crate) fn symlink(target: &str, link: &Path) {
        if let Some(parent) = link.parent() {
            std::fs::create_dir_all(parent).expect("create parent");
        }
        std::os::unix::fs::symlink(target, link).expect("create symlink");
    }

    /// A repo scoped to `docs`, with one declaration inside it. Every symlink
    /// case adds the link it is about.
    #[cfg(unix)]
    pub(crate) fn linked_repo(name: &str) -> PathBuf {
        let root = test_root(name);
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n[scan]\ninclude = [\"docs\"]\n",
        );
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n",
        );
        root
    }

    /// The graph findings — everything the report prints on stdout as
    /// `path:line: message` (§FS-check.2.1).
    /// Unix only: every caller is a symlink case and so `#[cfg(unix)]` too.
    #[cfg(unix)]
    pub(crate) fn findings(run: &CheckRun) -> Vec<String> {
        let mut diagnostics = run
            .report
            .errors
            .iter()
            .chain(run.report.warnings.iter())
            .filter(|diagnostic| diagnostic.code != "io")
            .collect::<Vec<_>>();
        // The order the report prints in (§FS-errors.4), so a case can read as
        // the lines a user would see.
        diagnostics.sort_by(|a, b| diagnostic_cmp(a, b));
        located_diagnostics(&run.config, diagnostics)
    }

    /// The files the walk handed to the scanner, in report spelling — what a case
    /// about *where the walk went* asserts on, independent of which of them
    /// happened to declare anything.
    /// Unix only: every caller is a symlink case and so `#[cfg(unix)]` too.
    #[cfg(unix)]
    pub(crate) fn scanned(config: &Config, findings: &Findings) -> Vec<String> {
        let mut files: Vec<String> = findings
            .scanned_files
            .iter()
            .map(|file| display_path(config, file))
            .collect();
        files.sort();
        files
    }

    /// The `error: <path>: <reason>` lines a file the scan could not read earns
    /// (§FS-check.2, §FS-errors.2.2).
    /// Unix only: every caller is a symlink case and so `#[cfg(unix)]` too.
    #[cfg(unix)]
    pub(crate) fn scan_errors(run: &CheckRun) -> Vec<String> {
        run.report
            .errors
            .iter()
            .filter(|diagnostic| diagnostic.code == "io")
            .map(|diagnostic| {
                format!(
                    "{}: {}",
                    diagnostic
                        .path
                        .as_ref()
                        .map(|path| display_path(&run.config, path))
                        .unwrap_or_default(),
                    diagnostic.message
                )
            })
            .collect()
    }
}
