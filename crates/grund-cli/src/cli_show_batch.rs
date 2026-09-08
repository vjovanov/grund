#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct BatchInputQuery {
    id: String,
    #[serde(default)]
    section: Option<String>,
}

/// Validate the complete NDJSON stream before entering the workspace loader;
/// empty lines are not records and physical line numbers stay reportable
/// (§FS-show.1, §FS-show.2.6).
fn read_batch_queries() -> Result<Vec<BatchShowQuery>, String> {
    use std::io::Read;

    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|error| format!("read batch input: {error}"))?;
    input
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| {
            serde_json::from_str::<BatchInputQuery>(line)
                .map(|query| BatchShowQuery {
                    id: query.id,
                    section: query.section,
                })
                .map_err(|error| {
                    format!(
                        "batch input line {}: {}",
                        index + 1,
                        concise_batch_input_error(&error.to_string())
                    )
                })
        })
        .collect()
}

fn concise_batch_input_error(message: &str) -> String {
    if let Some(rest) = message.strip_prefix("unknown field `")
        && let Some((field, _)) = rest.split_once('`')
    {
        return format!("unknown field `{field}`");
    }
    message.to_string()
}

/// Emit fixed ordered batch envelopes and map their aggregate verdict without
/// moving any per-query failure to stderr (§FS-output-shapes.4.1, §FS-cli.5).
fn command_show_batch(path: PathBuf, path_provided: bool, mode: ShowMode, all: bool) -> ExitCode {
    let queries = if all {
        None
    } else {
        match read_batch_queries() {
            Ok(queries) => Some(queries),
            Err(message) => {
                eprintln!("error: {message}");
                return ExitCode::from(2);
            }
        }
    };
    let records = match show_batch_with_scope(
        queries,
        ShowOpts {
            path,
            section: None,
            mode,
            format: ShowFormat::Json,
        },
        path_provided,
    ) {
        Ok(records) => records,
        Err(error) => {
            eprintln!("error: {error:#}");
            return ExitCode::from(2);
        }
    };
    let mut failed = false;
    for record in records {
        let section = record
            .query
            .section
            .as_deref()
            .map(|section| format!("\"{}\"", json_escape(section)))
            .unwrap_or_else(|| "null".to_string());
        let query = format!(
            "{{\"id\":\"{}\",\"section\":{section}}}",
            json_escape(&record.query.id)
        );
        match record.result {
            Ok(result) => println!(
                "{{\"query\":{query},\"ok\":true,\"result\":{result},\"error\":null}}"
            ),
            Err(error) => {
                failed = true;
                println!(
                    "{{\"query\":{query},\"ok\":false,\"result\":null,\"error\":{{\"severity\":\"error\",\"path\":null,\"line\":null,\"code\":\"{}\",\"message\":\"{}\",\"sites\":{}}}}}",
                    error.code,
                    json_escape(&error.message),
                    render_finding_sites_json(&error.sites)
                );
            }
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
