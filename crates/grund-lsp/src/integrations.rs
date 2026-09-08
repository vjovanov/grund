//! Embedded editor-client integrations for the installed server. §FS-lsp.2.4

use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Map, Value, json};
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

const LSP4IJ_TEMPLATE: &str = include_str!("../assets/integrations/lsp4ij/template.json");
const LSP4IJ_README: &str = include_str!("../assets/integrations/lsp4ij/README.md");

pub fn dispatch(args: &[OsString]) -> Result<()> {
    match args {
        [arg] if arg == "-h" || arg == "--help" => print_top_help(),
        [arg] if arg == "-V" || arg == "--version" => {
            println!("grund-lsp {}", env!("CARGO_PKG_VERSION"));
        }
        [command] if command == "integrations" => print_catalog(),
        [command, arg] if command == "integrations" && (arg == "-h" || arg == "--help") => {
            print_integrations_help();
        }
        [command, template] if command == "integrations" && template == "lsp4ij" => {
            let rendered = render_lsp4ij()?;
            print!("{}\n{}", rendered.template, rendered.readme);
        }
        [command, template, option, directory]
            if command == "integrations" && template == "lsp4ij" && option == "--write" =>
        {
            let rendered = render_lsp4ij()?;
            let directory = Path::new(directory);
            let outcome = materialize(directory, &rendered)?;
            println!("{} {}", outcome.verb(), directory.display());
            print!("{}", rendered.readme);
        }
        [command, template, arg]
            if command == "integrations" && template == "lsp4ij" && arg == "--help" =>
        {
            print_integrations_help();
        }
        [command, template, option]
            if command == "integrations" && template == "lsp4ij" && option == "--write" =>
        {
            bail!("--write requires an output directory; use --help for usage");
        }
        [command, template, ..] if command == "integrations" && template != "lsp4ij" => {
            bail!(
                "unknown editor template `{}`; available template: lsp4ij",
                template.to_string_lossy()
            );
        }
        [arg, ..] => bail!(
            "unexpected argument `{}`; use --help for usage",
            arg.to_string_lossy()
        ),
        [] => unreachable!("the no-argument server path is dispatched by main"),
    }
    Ok(())
}

fn print_top_help() {
    print!(
        "grund-lsp {}\n\n\
Language Server Protocol server and editor-client configuration for grund citations.\n\n\
USAGE:\n    grund-lsp\n    grund-lsp integrations [lsp4ij [--write <directory>]]\n\n\
With no arguments, the server speaks LSP over stdio and is normally spawned by an editor.\n\
`grund-lsp integrations` generates editor-client configuration; it is distinct from\n\
`grund integrations`, which configures clickable-citation rendering clients.\n\n\
COMMANDS:\n    integrations    List, preview, or write an editor template\n\n\
OPTIONS:\n    -h, --help       Print this help text\n    -V, --version    Print version\n\n\
EXITS:\n    0                Success\n    2                Invalid input, configuration, rendering, or write conflict\n",
        env!("CARGO_PKG_VERSION")
    );
}

fn print_integrations_help() {
    print!(
        "grund-lsp integrations — editor-client templates carried by grund-lsp.\n\n\
This is distinct from `grund integrations`, which configures clickable-citation\n\
rendering clients. No editor plugin or editor-owned configuration is installed.\n\n\
USAGE:\n    grund-lsp integrations\n    grund-lsp integrations lsp4ij\n    grund-lsp integrations lsp4ij --write <directory>\n\n\
The lsp4ij preview is read-only. --write creates an import directory and refuses\n\
to overwrite any changed, missing, or extra entry.\n\n\
EXITS:\n    0                Success\n    2                Invalid input, configuration, rendering, or write conflict\n"
    );
}

fn print_catalog() {
    println!("lsp4ij  LSP4IJ custom template for IntelliJ-family editors");
}

struct RenderedIntegration {
    template: String,
    readme: &'static str,
}

/// Render the effective extension snapshot without editor or transport state.
/// Configuration discovery remains in `grund-core`. §AR-lsp.1 §FS-lsp.2.4
fn render_lsp4ij() -> Result<RenderedIntegration> {
    let cwd = std::env::current_dir().context("read the current working directory")?;
    let config = grund_core::effective_config(&cwd)?;
    let executable = std::env::current_exe().context("find the running grund-lsp executable")?;
    let executable = executable
        .to_str()
        .ok_or_else(|| anyhow!("the grund-lsp executable path is not valid UTF-8"))?;

    let mut template: Value =
        serde_json::from_str(LSP4IJ_TEMPLATE).context("parse the embedded LSP4IJ template")?;
    let object = template
        .as_object_mut()
        .ok_or_else(|| anyhow!("the embedded LSP4IJ template is not a JSON object"))?;
    object.insert("programArgs".into(), program_args(executable));
    object.insert(
        "fileTypeMappings".into(),
        Value::Array(
            config
                .extensions
                .iter()
                .map(|extension| {
                    json!({
                        "fileType": { "patterns": [format!("*.{extension}")] },
                        "languageId": language_id(extension),
                    })
                })
                .collect(),
        ),
    );
    let mut template =
        serde_json::to_string_pretty(&template).context("render the LSP4IJ template")?;
    template.push('\n');
    Ok(RenderedIntegration {
        template,
        readme: LSP4IJ_README,
    })
}

fn program_args(executable: &str) -> Value {
    let mut args = Map::new();
    args.insert(
        "default".into(),
        Value::String(format!(
            "sh -c 'cd \"$PROJECT_DIR$\" && exec {}'",
            quote_posix(executable)
        )),
    );
    args.insert(
        "windows".into(),
        Value::String(format!(
            "cmd /D /V:OFF /S /C \"cd /D \"\"$PROJECT_DIR$\"\" && \"\"{}\"\"\"",
            quote_cmd_contents(executable)
        )),
    );
    Value::Object(args)
}

fn quote_posix(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn quote_cmd_contents(value: &str) -> String {
    value
        .replace('^', "^^")
        .replace('%', "%%")
        .replace('"', "\"\"")
}

fn language_id(extension: &str) -> &str {
    match extension {
        "md" => "markdown",
        "rs" => "rust",
        "kt" => "kotlin",
        "ts" => "typescript",
        "tsx" => "typescriptreact",
        "js" => "javascript",
        "py" => "python",
        "rb" => "ruby",
        "cs" => "csharp",
        "scm" => "scheme",
        "clj" => "clojure",
        "hs" | "lhs" => "haskell",
        "adb" | "ads" => "ada",
        other => other,
    }
}

enum WriteOutcome {
    Created,
    Unchanged,
}

impl WriteOutcome {
    fn verb(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Unchanged => "unchanged",
        }
    }
}

/// Write only a new root, or accept the exact two-file output already present.
/// Every other pre-existing tree is preserved as a conflict. §FS-lsp.2.4
fn materialize(root: &Path, rendered: &RenderedIntegration) -> Result<WriteOutcome> {
    match fs::symlink_metadata(root) {
        Ok(metadata) => {
            if metadata.file_type().is_dir() && directory_matches(root, rendered)? {
                return Ok(WriteOutcome::Unchanged);
            }
            bail!(
                "{} conflicts with the generated template; move or remove it before retrying",
                root.display()
            );
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(err).with_context(|| format!("inspect {}", root.display())),
    }

    if let Some(parent) = root.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    match fs::create_dir(root) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
            if directory_matches(root, rendered)? {
                return Ok(WriteOutcome::Unchanged);
            }
            bail!(
                "{} conflicts with the generated template; move or remove it before retrying",
                root.display()
            );
        }
        Err(err) => return Err(err).with_context(|| format!("create {}", root.display())),
    }

    if let Err(err) = write_new_root(root, rendered) {
        let template = root.join("template.json");
        if fs::read(&template).ok().as_deref() == Some(rendered.template.as_bytes()) {
            let _ = fs::remove_file(template);
        }
        let _ = fs::remove_dir(root);
        return Err(err);
    }
    Ok(WriteOutcome::Created)
}

fn directory_matches(root: &Path, rendered: &RenderedIntegration) -> Result<bool> {
    let mut names = fs::read_dir(root)
        .with_context(|| format!("read {}", root.display()))?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<std::io::Result<Vec<_>>>()
        .with_context(|| format!("read {}", root.display()))?;
    names.sort();
    if names != [OsString::from("README.md"), OsString::from("template.json")] {
        return Ok(false);
    }
    Ok(
        fs::read(root.join("README.md")).ok().as_deref() == Some(rendered.readme.as_bytes())
            && fs::read(root.join("template.json")).ok().as_deref()
                == Some(rendered.template.as_bytes()),
    )
}

fn write_new_root(root: &Path, rendered: &RenderedIntegration) -> Result<()> {
    let template = root.join("template.json");
    write_new_file(&template, rendered.template.as_bytes())?;
    let readme = root.join("README.md");
    write_new_file(&readme, rendered.readme.as_bytes())?;
    Ok(())
}

fn write_new_file(path: &Path, contents: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("create {}", path.display()))?;
    file.write_all(contents)
        .with_context(|| format!("write {}", path.display()))
}
