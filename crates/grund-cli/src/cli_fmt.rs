/// `grund fmt [--check|--write] [--marker]`: normalize citations in bulk —
/// rewrite `$$` triggers to `§` and keep cross-reference links current
/// (§FS-fmt).
fn command_fmt(args: &[String]) -> ExitCode {
    let mut path = PathBuf::from(".");
    let mut path_provided = false;
    let mut write = false;
    let mut check_flag = false;
    let mut marker = false;
    let mut cross_refs = false;
    for arg in args {
        match arg.as_str() {
            "--check" => check_flag = true,
            "--write" => write = true,
            "--marker" => marker = true,
            "--cross-refs" => cross_refs = true,
            other if other.starts_with('-') => {
                eprintln!("error: unknown flag `{other}`");
                return ExitCode::from(2);
            }
            other => {
                if path_provided {
                    eprintln!("error: fmt takes at most one path argument");
                    return ExitCode::from(2);
                }
                path = PathBuf::from(other);
                path_provided = true;
            }
        }
    }
    if write && check_flag {
        eprintln!("error: --check and --write cannot be used together");
        return ExitCode::from(2);
    }
    let output = match format_references(FmtOpts {
        path,
        path_provided,
        write,
        add_marker: marker,
        cross_refs,
    }) {
        Ok(output) => output,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    if write {
        let mut files = output
            .changes
            .iter()
            .map(|change| change.path.clone())
            .collect::<Vec<_>>();
        files.sort();
        files.dedup();
        println!(
            "rewrote {} reference{}{}",
            output.changes.len(),
            if output.changes.len() == 1 { "" } else { "s" },
            if files.is_empty() { "" } else { ":" }
        );
        for path in &files {
            let count = output
                .changes
                .iter()
                .filter(|change| &change.path == path)
                .count();
            println!("  {path} ({count})");
        }
    } else {
        for change in &output.changes {
            println!("{}:{}: {}", change.path, change.line, change.label);
        }
    }
    if !output.scan_errors.is_empty() {
        // §FS-fmt.3: `fmt` walks the tree `check` walks and owes the same account
        // of the paths in it that could not be read (§FS-check.2). What it did
        // rewrite is real and, under `--write`, already on disk — the `2` says the
        // tree it ran over was not the whole tree.
        for error in &output.scan_errors {
            eprintln!("error: {}: {}", error.path, error.message);
        }
        return ExitCode::from(2);
    }
    if write || output.changes.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
