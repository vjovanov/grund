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
            if kind.kind == "FS" {
                kind.folder = Some("docs/functional-spec".to_string());
                kind.file = None;
            }
            // §FS-config.3.4: the suites built on this helper are about the
            // scanner, the shorthand, and the walk — not about the index a folder
            // kind keeps. Opting every folder kind out keeps their fixtures from
            // carrying a README they never assert on (the rule's own cases live in
            // `tests_kind_index.rs`).
            if kind.folder.is_some() {
                kind.index = KindIndex::Disabled;
            }
        }
        config
    }

    /// The default `grund init` config: `{kind}-{number}-{slug}`, the only shape
    /// that has a shorthand at all (§FS-check.1.2). Shared by the four shorthand
    /// suites, which each had a byte-identical copy — including the assertion,
    /// which is the point of the helper: a change to the default format must
    /// fail here rather than quietly leave those suites testing no shorthand.
    pub(crate) fn numbered_config(root: PathBuf) -> Config {
        let config = legacy_fs_folder_config(root);
        assert_eq!(config.id_format, "{kind}-{number}-{slug}");
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
            // §FS-config.3.4: `index = false` on both folder kinds — these cases
            // are about which *files* the walk reads under a link, and an index
            // README in the fixture would only add findings none of them assert on.
            // The `E2E` kind is restated because `[[kinds]]` replaces the defaults
            // entirely, and one case here is about an e2e case directory.
            "grund_config_version = 1\n\n\
             [[kinds]]\nkind = \"FS\"\nfolder = \"docs/functional-spec\"\nindex = false\n\n\
             [[kinds]]\nkind = \"E2E\"\nfolder = \"e2e/cases\"\nindex = false\n\n\
             [scan]\ninclude = [\"docs\"]\n",
        );
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n",
        );
        root
    }

    /// A repo whose `FS` kind is a folder with the default `README.md` index
    /// (§FS-config.3.4), holding one declaration — the fixture the kind-index
    /// cases share across their three modules.
    pub(crate) fn kind_index_repo(name: &str) -> PathBuf {
        kind_index_repo_with(name, "")
    }

    /// The same fixture off strict mode, where a bare ID-shaped token is a
    /// recognized citation (§FS-config.3.1) — the configuration the entry-form
    /// cases need, because that is where `fmt` and `check` can disagree.
    pub(crate) fn kind_index_repo_loose(name: &str) -> PathBuf {
        kind_index_repo_with(name, "[reference]\nstrict = false\n\n")
    }

    fn kind_index_repo_with(name: &str, reference: &str) -> PathBuf {
        let root = test_root(name);
        write(
            &root.join("grund.toml"),
            &format!(
                "grund_config_version = 1\n\n{reference}\
                 [[kinds]]\nkind = \"FS\"\nfolder = \"docs/specs\"\n\n\
                 [scan]\ninclude = [\"docs\"]\n"
            ),
        );
        write(
            &root.join("docs/specs/FS-001-login.md"),
            "# FS-001-login: A user logs in\n\nBody.\n",
        );
        root
    }

    /// Every finding's code, errors then warnings — what a case asserts on when
    /// it cares which rules fired and not what they said.
    pub(crate) fn codes(run: &CheckRun) -> Vec<String> {
        run.report
            .errors
            .iter()
            .chain(run.report.warnings.iter())
            .map(|diagnostic| diagnostic.code.to_string())
            .collect()
    }

    /// The first finding carrying `code`, panicking with the codes that did fire
    /// when there is none — so a case that expected one rule and got another
    /// fails naming both.
    pub(crate) fn only<'a>(run: &'a CheckRun, code: &str) -> &'a Diagnostic {
        run.report
            .errors
            .iter()
            .chain(run.report.warnings.iter())
            .find(|diagnostic| diagnostic.code == code)
            .unwrap_or_else(|| panic!("expected a {code} finding, got {:?}", codes(run)))
    }

    /// The graph findings — everything the report prints on stdout as
    /// `path:line: message` (§FS-check.2.1).
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

    /// One source file's inline-citation-style findings as `(line, message)`,
    /// ascending — the harness the two doc-comment suites share
    /// (§FS-inline-citation-style.1.1). The tree always declares `FS-001-login`,
    /// so a citation resolves and only the style rule can speak, and the caps
    /// are the defaults (3 lines, 100 columns), so a four-line comment block is
    /// over them.
    pub(crate) fn inline_style_findings(
        name: &str,
        file: &str,
        source: &str,
    ) -> Vec<(usize, String)> {
        inline_style_findings_with(name, file, source, |_| {})
    }

    /// The same harness with one turn of the config first: an extension outside
    /// the default `[scan] extensions`, a different `inline_style`, or the note
    /// layout keys.
    pub(crate) fn inline_style_findings_with(
        name: &str,
        file: &str,
        source: &str,
        configure: impl FnOnce(&mut Config),
    ) -> Vec<(usize, String)> {
        let root = test_root(name);
        write(
            &root.join("docs/functional-spec/FS-001-login.md"),
            "# FS-001-login: Login\n",
        );
        write(&root.join(file), source);

        let mut config = legacy_fs_folder_config(root.clone());
        configure(&mut config);
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);
        let mut rows = report
            .errors
            .iter()
            .chain(report.warnings.iter())
            .filter(|finding| finding.code == "inline-citation-style")
            .map(|finding| (finding.line.unwrap_or(0), finding.message.clone()))
            .collect::<Vec<_>>();
        rows.sort();
        rows
    }

    /// The one finding a block of more than three lines earns under the default
    /// caps, anchored at the line the block opens on.
    pub(crate) fn over_the_line_cap(line: usize) -> Vec<(usize, String)> {
        vec![(line, "inline note exceeds 3-line maximum".to_string())]
    }

    /// The one finding a line wider than a hundred characters earns.
    pub(crate) fn over_the_column_cap(line: usize) -> Vec<(usize, String)> {
        vec![(line, "inline note exceeds 100-column maximum".to_string())]
    }
}
