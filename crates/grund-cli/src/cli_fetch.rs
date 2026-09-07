/// `grund fetch <ID>`: the sole explicit external integration entry point
/// (§FS-fetch.1, §FS-fetch.7).
fn command_fetch(args: &[String]) -> ExitCode {
    if args.len() != 1 {
        eprintln!("error: fetch requires exactly one <ID>");
        return ExitCode::from(2);
    }
    match fetch_snapshot(&args[0], Path::new(".")) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => match err.kind {
            FetchFailureKind::Query => {
                eprintln!("error: {}", err.message);
                ExitCode::FAILURE
            }
            FetchFailureKind::Operational => {
                eprintln!("error: {}", err.message);
                ExitCode::from(2)
            }
        },
    }
}
