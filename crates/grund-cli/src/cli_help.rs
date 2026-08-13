/// `grund --help` / `grund help` — the top-level usage text: the subcommand list and
/// global flags (§FS-cli.2). `grund help <cmd>` defers to `print_subcommand_help`.
fn print_help() {
    println!("grund — ground your agents in the spec.");
    println!("Checks ID-based citations (§<ID>.<section>) across Markdown docs and source-code doc-comments, so every reader — human or AI — points at the same facts.");
    println!();
    println!("Usage:");
    println!(
        "  grund <ID>[.<section>] [OPTIONS]    print one declaration body"
    );
    println!(
        "  grund check [PATH] [OPTIONS]        validate a repo or subtree"
    );
    println!(
        "  grund <COMMAND> [ARGS] [OPTIONS]    run `grund <COMMAND> --help` for that command's options"
    );
    println!();
    println!("Commands:");
    println!("  show     Print one declaration body for agent context (default).  e.g. grund FS-login.3");
    println!("  check    Validate every reference in a repo.                      e.g. grund check .");
    println!(
        "  list     The ID catalog: every declared ID, path:line, title.     e.g. grund list --kind FS"
    );
    println!(
        "  refs     List every citation of an ID, as path:line.              e.g. grund refs FS-login"
    );
    println!(
        "  cover    Group the citation graph by scanned file.                e.g. grund cover --format json"
    );
    println!(
        "  fmt      Rewrite `$$` triggers to `§`; --marker upgrades cites.   e.g. grund fmt --check"
    );
    println!(
        "  id       Next conflict-free ID for a new declaration.             e.g. grund id FS \"user login\""
    );
    println!(
        "  init     Scaffold agent instructions + grund.toml.                e.g. grund init --docs"
    );
    println!(
        "  config   Validate or show the effective grund.toml.               e.g. grund config show"
    );
    println!(
        "  agent-setup-instructions  Print AI setup guide.                   e.g. grund agent-setup-instructions"
    );
    println!(
        "  completions  Print shell completion scripts.                      e.g. grund completions bash"
    );
    println!(
        "  integrations  Print/install clickable-citation integrations.      e.g. grund integrations wezterm"
    );
    println!();
    println!(
        "Options:  --format text|json   per-command (place after the subcommand); text is the default."
    );
    println!(
        "          --version, -V        print version.   --help, -h   show this screen.   Docs: docs/functional-spec/"
    );
}

/// Per-subcommand `--help` / `help <subcommand>` page (§FS-cli.2, §FS-cli.3): what
/// it takes, every flag with a one-line example, the exit codes, and the common
/// recovery path. Goes to stdout, exit 0 — help is never an error.
fn print_subcommand_help(cmd: &str) {
    match cmd {
        "check" => {
            println!(
                "grund check — validate every ID citation across the repo."
            );
            println!();
            println!("Usage:  grund check [PATH] [--require-grounding] [--suggestions] [--format text|json]");
            println!();
            println!(
                "PATH defaults to `.`; config (`grund.toml`, root or `.agents/`) is discovered by walking up from it."
            );
            println!(
                "With no config, grund scans `docs/`, `e2e/`, and `src/`; set `[scan] include` to widen it."
            );
            println!("Pointing grund at an explicit PATH scans exactly that file or directory.");
            println!(
                "Path validation is explicit; `grund PATH` is parsed as an ID query."
            );
            println!();
            println!("Options:");
            println!(
                "  --format text|json   text (default) prints `success` or `path:line: message`; json emits NDJSON."
            );
            println!(
                "  --require-grounding  also require every source file to cite a declared ID ([reference] require_grounding)."
            );
            println!(
                "  --suggestions        also surface should/should-not citation-direction findings ([citations])."
            );
            println!();
            println!(
                "Findings go to stdout (the linter convention) — `grund check | …` and `grund check"
            );
            println!(
                "--format json | jq` need no redirect. Only run-level `error:` / `warning:` lines"
            );
            println!(
                "(unreadable path, empty scan) go to stderr; a clean text run prints `success`."
            );
            println!();
            println!(
                "Exit:  0 clean · 1 dangling / duplicate / unknown-section / ungrounded findings · 2 unreadable tree or CLI error."
            );
            println!();
            println!("Examples:");
            println!("  grund check              # check the whole repo");
            println!("  grund check docs/        # check one subtree");
            println!("  grund check --format json | jq # machine-readable diagnostics for CI");
        }
        "show" => {
            println!(
                "grund show — print one declaration's body by ID, so an agent pulls a single fact"
            );
            println!("into context without loading the whole document. `show` is the default command.");
            println!();
            println!(
                "Usage:  grund [show] <ID>[.<section>] [PATH] [--section S] [--brief|--toc|--full] [--format text|md|json] [--path PATH]"
            );
            println!();
            println!("Modes form an ordered ladder (each adds to the previous):");
            println!("  --brief                heading + first paragraph    e.g. grund --brief FS-login");
            println!("  (default)              + the rest of the lead, cut at the first child section");
            println!("  --toc                  + the nested section map     e.g. grund --toc FS-login");
            println!("  --full                 + every subsection body      e.g. grund --full FS-login");
            println!();
            println!("Other options:");
            println!("  --section S            show only that section path, e.g. --section 3.1");
            println!(
                "  --format text|md|json  text (default) is the body; md keeps the heading; json wraps it"
            );
            println!("  --path PATH            repo or subtree to resolve the ID in (default `.`)");
            println!();
            println!(
                "Exit:  0 printed · 1 ID not found / ambiguous / broken stub / unknown section · 2 CLI error."
            );
            println!();
            println!("Examples:");
            println!("  grund FS-login                   # the lead — the cheap default");
            println!("  grund FS-login --toc             # lead + section map");
            println!("  grund FS-login.3.1               # the lead of that nested section");
            println!("  grund FS-login --full            # the whole declaration body");
            println!();
            println!(
                "ID not found? `grund list` shows every declared ID; `grund id <KIND> \"…\"` proposes a new one."
            );
        }
        "list" => {
            println!("grund list — the ID catalog: every declared ID in the repo, with where it's");
            println!(
                "declared and its one-line title. The complement of `grund refs` (which lists"
            );
            println!(
                "the citations of one ID) — `list` is the index of what you can read with `grund <ID>`."
            );
            println!();
            println!(
                "Usage:  grund list [PATH] [--kind KIND[,KIND]...] [--unused] [--summary] [--format text|json]"
            );
            println!();
            println!(
                "Output is one line per declared ID, `<ID>  <path>:<line>  <title>`, sorted by ID."
            );
            println!(
                "Stub-and-inline pairs collapse to one line; a duplicate-declared ID gets a line per home."
            );
            println!();
            println!("Options:");
            println!(
                "  --kind KIND[,KIND]  only selected kinds; repeatable       e.g. grund list --kind FS,AR"
            );
            println!(
                "  --unused            only declarations nothing cites yet (skips E2E unless E2E is selected)"
            );
            println!(
                "  --summary           one row per kind with count and home  e.g. grund list --summary"
            );
            println!(
                "  --format text|json   text (default) is the table on stdout; json emits NDJSON (adds `refs` count)."
            );
            println!();
            println!(
                "Exit:  0 scan succeeded (an empty catalog prints nothing) · 2 unreadable tree, or an unknown --kind."
            );
            println!();
            println!("Examples:");
            println!("  grund list                      # the whole catalog");
            println!("  grund list --kind FS,AR docs/   # specs and architecture IDs under docs/");
            println!("  grund list --summary            # counts by kind");
            println!(
                "  grund list --unused             # uncited declarations (specs, decisions, …) — E2E cases excluded"
            );
            println!("  grund list --unused --kind E2E  # uncited e2e cases only, for inventory");
        }
        "refs" => {
            println!(
                "grund refs — list every citation of an ID, as `path:line`, so you can see who"
            );
            println!("depends on a declaration before you change it.");
            println!();
            println!(
                "Usage:  grund refs <ID>[.<section>] [PATH] [--section S] [--summary] [--format text|json]"
            );
            println!();
            println!(
                "PATH defaults to `.`. With a `.<section>` (or --section), only citations of that"
            );
            println!(
                "exact section are listed. An ID with no citations prints nothing and exits 0."
            );
            println!();
            println!("Options:");
            println!(
                "  --section S          list only citations of that section path   e.g. grund refs FS-login --section 3"
            );
            println!(
                "  --summary            group citations by citing file             e.g. grund refs FS-login --summary"
            );
            println!(
                "  --format text|json   text (default) prints `path:line: <citation>`; json emits NDJSON."
            );
            println!();
            println!(
                "The citation list is the result, so it goes to stdout (text and json alike) —"
            );
            println!(
                "`grund refs <ID> | …` works like `grund list`. Only the typo `note:` goes to stderr."
            );
            println!();
            println!(
                "Exit:  0 scan succeeded (with or without hits) · 2 unreadable tree or CLI error."
            );
            println!();
            println!("Examples:");
            println!("  grund refs FS-login             # every citation of FS-login");
            println!("  grund refs FS-login --summary   # one row per citing file");
            println!("  grund refs FS-login.3           # only citations of section 3");
        }
        "cover" => {
            println!("grund cover — group the citation graph by scanned file.");
            println!();
            println!("Usage:  grund cover [PATH] [--format text|json]");
            println!();
            println!("PATH defaults to `.`. The command runs the same scan as `check` and `refs`,");
            println!("then prints one file record with the citations found in that file.");
            println!();
            println!("Options:");
            println!(
                "  --format text|json   text (default) groups citations by file; json emits one record per file."
            );
            println!();
            println!("Exit:  0 scan succeeded · 2 unreadable tree, incomplete scan, or CLI error.");
            println!();
            println!("Examples:");
            println!("  grund cover src/                # source files and their spec citations");
            println!("  grund cover --format json       # machine-readable coverage index");
        }
        "fmt" => {
            println!(
                "grund fmt — normalize citation syntax: rewrite the `$$` trigger to the `§` marker,"
            );
            println!(
                "optionally upgrade bare ID tokens, and optionally emit Markdown cross-reference links."
            );
            println!();
            println!("Usage:  grund fmt [PATH] [--check | --write] [--marker] [--cross-refs]");
            println!();
            println!("Options:");
            println!(
                "  --check        report pending rewrites, exit 1 if any exist         e.g. grund fmt --check"
            );
            println!(
                "  --write        apply the changes in place                           e.g. grund fmt --write"
            );
            println!(
                "  --marker       also prefix bare `<ID>` tokens with the marker        e.g. grund fmt --write --marker"
            );
            println!(
                "  --cross-refs   wrap citations as Markdown links to targets          e.g. grund fmt --write --cross-refs"
            );
            println!(
                "                 also runs by default on --write; set [fmt.cross_refs].enabled = false to opt out"
            );
            println!();
            println!(
                "With neither --check nor --write, fmt prints the would-be changes and exits 1 if any (a dry run)."
            );
            println!(
                "--write prints `rewrote N references:` then one `  <path> (count)` line per file touched."
            );
            println!(
                "The report goes to stdout (like `grund check`); CLI-level `error:` lines go to stderr."
            );
            println!();
            println!(
                "Exit:  0 nothing to do, or --write succeeded · 1 changes pending (dry run / --check) · 2 unreadable tree or CLI error."
            );
        }
        "id" => {
            println!("grund id — emit the next conflict-free ID for a new declaration of a kind.");
            println!();
            println!(
                "Usage:  grund id <KIND> \"<title>\" [PATH] [--width N] [--explain] [--format text|json]"
            );
            println!();
            println!(
                "KIND is one of the configured `[[kinds]]` prefixes — defaults G, FS, AR, DF, DA, E2E, RM;"
            );
            println!(
                "`grund config show` lists this repo's. The title is slugified deterministically; the number"
            );
            println!("is `max(existing) + 1` (holes are never filled).");
            println!();
            println!("Options:");
            println!(
                "  --width N      minimum digit width for the number (default 3)   e.g. grund id FS \"x\" --width 4"
            );
            println!(
                "  --explain      also print where to put the declaration file     e.g. grund id FS \"x\" --explain"
            );
            println!(
                "  --format text|json   text (default) is the bare ID on stdout; json adds kind/number/slug/folder."
            );
            println!();
            println!(
                "Exit:  0 ID emitted · 1 empty slug / collision · 2 unknown kind, scan, or CLI error."
            );
            println!();
            println!("Examples:");
            println!(
                "  grund id FS \"User can log in\"          # -> FS-007-user-can-log-in (or FS-user-can-log-in)"
            );
            println!(
                "  ID=$(grund id FS \"User can log in\"); $EDITOR \"docs/functional-spec/$ID.md\""
            );
        }
        "init" => {
            println!(
                "grund init — scaffold agent instructions + `grund.toml` (and, with --docs, the docs/ and e2e/ layout)."
            );
            println!(
                "Idempotent: re-running updates the managed agent-instructions block in place and leaves your edits alone."
            );
            println!();
            println!(
                "Usage:  grund init [PATH] [--docs] [--name NAME] [--description TEXT] [--force] [--dry-run] [--agents-md] [--claude] [--gemini] [--pi] [--copilot] [--cursor] [--windsurf] [--zed]"
            );
            println!();
            println!("Options:");
            println!(
                "  --docs         also write docs/ (grund, goals, roadmap, changelog, spec READMEs) and e2e/"
            );
            println!(
                "  --name NAME    project name to interpolate (default: derived from the directory)"
            );
            println!(
                "  --description TEXT  one-line project description written to grund.toml (shown next to this project in workspace member lists)"
            );
            println!(
                "  --force        rewrite the canonical AGENTS.md and --docs scaffolds; grund.toml is never overwritten"
            );
            println!(
                "  --dry-run      report what would be written/appended/updated without touching any file"
            );
            println!(
                "  --agents-md    create/update canonical AGENTS.md even when another entrypoint exists"
            );
            println!("  --claude       create/update CLAUDE.md and .claude/CLAUDE.md");
            println!("  --gemini       create/update GEMINI.md");
            println!("  --pi           create/update .pi/AGENTS.md");
            println!("  --copilot      create/update .github/copilot-instructions.md");
            println!(
                "  --cursor       create/update .cursor/rules/grund.mdc (legacy .cursorrules is updated only if present)"
            );
            println!("  --windsurf     create/update .windsurfrules");
            println!("  --zed          create/update .rules");
            println!();
            println!(
                "Exit:  0 written / updated / already current · 2 missing target, unknown flag, or unsupported newer block."
            );
            println!();
            println!("Examples:");
            println!("  grund init --docs                  # full first-time scaffold");
            println!("  grund init --dry-run               # preview without writing");
            println!(
                "  grund init --name \"My Service\"      # auto-detect entrypoint, else AGENTS.md"
            );
            println!("  grund init --claude --gemini        # create/update both agent entrypoints");
        }
        "config" => {
            println!(
                "grund config — inspect the effective `grund.toml` discovered from a path."
            );
            println!();
            println!("Usage:  grund config <show | validate> [PATH]");
            println!();
            println!(
                "  show       print the effective config as TOML (defaults filled in for keys you didn't set)."
            );
            println!(
                "  validate   parse the discovered config and report the first error; exit 0 if it's well-formed."
            );
            println!();
            println!("PATH defaults to `.`; config is discovered by walking up from that path.");
            println!(
                "There is no `--config <file>` override — config is discovered, not pointed at (FS-cli.6)."
            );
            println!();
            println!(
                "Exit:  0 well-formed / printed · 1 `validate` found an error · 2 no subcommand, or `show` couldn't read the config."
            );
        }
        "completions" => {
            println!("grund completions — print a shell completion script for grund.");
            println!();
            println!("Usage:  grund completions <bash|zsh|fish>");
            println!();
            println!("The generated scripts complete subcommands and complete declared IDs for");
            println!("`grund <ID>`, explicit `show`, and `grund refs <ID>` by calling the hidden helper:");
            println!("`grund complete ids --prefix <word>`.");
            println!();
            println!("Install examples:");
            println!("  source <(grund completions bash)");
            println!("  grund completions zsh > ~/.zfunc/_grund");
            println!("  grund completions fish > ~/.config/fish/completions/grund.fish");
            println!();
            println!("Exit:  0 script printed · 2 unsupported shell.");
        }
        "integrations" => {
            println!(
                "grund integrations — print or install the clickable-citation terminal/editor integrations."
            );
            println!();
            println!(
                "Usage:  grund integrations [<client>] [--write] [--conversation plain|link]"
            );
            println!(
                "                          [--conversation-target file|path|web|vscode|vscodium|cursor]"
            );
            println!(
                "                          [--agent codex|claude|gemini|copilot|zed|pi] [--format text|json]"
            );
            println!();
            println!("With no client, detects the terminal/editor from the environment and prints");
            println!("what applies. With a client — codium, iterm2, kitty, tmux, vscode, or");
            println!("wezterm — prints that integration's snippet and the grund-open resolver;");
            println!("--write installs it as a managed, idempotent block instead of printing. A");
            println!("write also records the local conversation preference and updates global agent");
            println!("instructions. With no client, --write with either conversation flag changes");
            println!("only that preference. --conversation-target picks how a linked citation");
            println!("addresses its declaration; --agent scopes that choice to one agent instead");
            println!("of the machine. --format json emits a plan.");
            println!();
            println!("Preview and install examples:");
            println!("  grund integrations                    # detect and list what applies");
            println!("  grund integrations wezterm            # print the snippet + resolver");
            println!("  grund integrations wezterm --write    # install it, idempotently");
            println!("  grund integrations --write --conversation link  # preference only");
            println!("  grund integrations --write --conversation-target vscodium  # open in the editor");
            println!("  grund integrations --write --agent codex --conversation-target web  # that agent only");
            println!();
            println!("Exit:  0 printed or installed · 2 invalid options, missing write target, or a newer block.");
        }
        "agent-setup-instructions" => {
            println!(
                "grund agent-setup-instructions — print the guided setup instructions for AI agents."
            );
            println!();
            println!("Usage:  grund agent-setup-instructions");
            println!();
            println!(
                "The output is the same Markdown source shipped as `skills/grund-init/SKILL.md`,"
            );
            println!("embedded in the binary so installed agents can discover the setup workflow");
            println!("without access to the source tree.");
            println!();
            println!("Exit:  0 instructions printed · 2 unexpected arguments.");
        }
        _ => print_help(),
    }
}
