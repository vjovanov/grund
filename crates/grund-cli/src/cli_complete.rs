/// `grund complete` / `grund completions <shell>`: dynamic ID completion and
/// the static shell completion scripts (§FS-completions).
fn command_complete(args: &[String]) -> ExitCode {
    match args.first().map(|arg| arg.as_str()) {
        Some("ids") => command_complete_ids(&args[1..]),
        _ => {
            eprintln!("error: expected `complete ids`");
            ExitCode::from(2)
        }
    }
}

fn command_complete_ids(args: &[String]) -> ExitCode {
    let mut path = PathBuf::from(".");
    let mut path_provided = false;
    let mut prefix = String::new();
    let mut force_sections = false;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--prefix" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --prefix requires a value");
                    return ExitCode::from(2);
                }
                prefix = args[idx].clone();
            }
            other if other.starts_with("--prefix=") => {
                prefix = other.trim_start_matches("--prefix=").to_string();
            }
            "--sections" => force_sections = true,
            "--path" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --path requires a value");
                    return ExitCode::from(2);
                }
                path = PathBuf::from(&args[idx]);
                path_provided = true;
            }
            other if other.starts_with('-') => {
                eprintln!("error: unknown flag `{other}`");
                return ExitCode::from(2);
            }
            other => {
                path = PathBuf::from(other);
                path_provided = true;
            }
        }
        idx += 1;
    }
    let candidates = match complete_ids(CompleteIdsOpts {
        path,
        path_provided,
        prefix,
        sections: force_sections,
    }) {
        Ok(candidates) => candidates,
        Err(_) => return ExitCode::SUCCESS,
    };
    for candidate in candidates {
        println!("{candidate}");
    }
    ExitCode::SUCCESS
}

fn command_completions(args: &[String]) -> ExitCode {
    if args.is_empty() {
        eprintln!("error: completions requires <bash|zsh|fish>");
        return ExitCode::from(2);
    }
    if args.len() > 1 {
        eprintln!("error: completions takes exactly one shell argument");
        return ExitCode::from(2);
    }
    match args[0].as_str() {
        "bash" => {
            print_bash_completion();
            ExitCode::SUCCESS
        }
        "zsh" => {
            print_zsh_completion();
            ExitCode::SUCCESS
        }
        "fish" => {
            print_fish_completion();
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("error: unsupported shell `{other}`");
            eprintln!("known shells: bash, zsh, fish");
            ExitCode::from(2)
        }
    }
}

fn print_bash_completion() {
    print!(
        r#"# bash completion for grund
_grund_complete_ids() {{
    local cur="${{COMP_WORDS[COMP_CWORD]}}"
    mapfile -t COMPREPLY < <(grund complete ids --prefix "$cur" 2>/dev/null)
    for candidate in "${{COMPREPLY[@]}}"; do
        if [[ "$candidate" == */ ]]; then
            compopt -o nospace 2>/dev/null
            break
        fi
    done
}}

_grund() {{
    local cur="${{COMP_WORDS[COMP_CWORD]}}"
    local sub="${{COMP_WORDS[1]}}"
    COMPREPLY=()

    if [[ $COMP_CWORD -eq 1 ]]; then
        local commands=($(compgen -W "check show list refs cover fmt id init config agent-setup-instructions completions" -- "$cur"))
        # IDs start with an uppercase kind (FS-…, GOAL-…), but workspace aliases
        # are lowercase (`api/FS-login`). Once the user has typed a non-flag
        # prefix, ask the helper for matching IDs and aliases.
        if [[ -n "$cur" && "$cur" != -* ]]; then
            _grund_complete_ids
        fi
        COMPREPLY=("${{commands[@]}}" "${{COMPREPLY[@]}}")
        return 0
    fi

    case "$sub" in
        show|refs)
            _grund_complete_ids
            return 0
            ;;
    esac
}}

complete -F _grund grund
"#
    );
}

fn print_zsh_completion() {
    println!(
        r#"#compdef grund

_grund_ids() {{
  local -a raw bare aliases
  raw=("${{(@f)$(grund complete ids --prefix "$words[CURRENT]" 2>/dev/null)}}")
  for candidate in $raw; do
    if [[ -z "$candidate" ]]; then
      continue
    fi
    if [[ "$candidate" == */ ]]; then
      aliases+=("$candidate")
    else
      bare+=("$candidate")
    fi
  done
  if (( ${{#aliases}} > 0 )); then
    compadd -S '' -a aliases
  fi
  if (( ${{#bare}} > 0 )); then
    _describe 'grund ids' bare
  fi
}}

_grund() {{
  local -a commands
  commands=(
    'check:validate every reference in a repo'
    'show:print one declaration body by ID'
    'list:list declared IDs'
    'refs:list citations of an ID'
    'cover:group citations by file'
    'fmt:normalize citation syntax'
    'id:emit the next conflict-free ID'
    'init:scaffold AGENTS.md and config'
    'config:inspect the effective config'
    'agent-setup-instructions:print the guided setup instructions for AI agents'
    'completions:print shell completion script'
  )

  if (( CURRENT == 2 )); then
    _describe 'grund command' commands
    # IDs start with an uppercase kind, but workspace aliases are lowercase.
    # Once a non-flag prefix exists, ask the helper for matching IDs/aliases.
    if [[ -n "$words[CURRENT]" && "$words[CURRENT]" != -* ]]; then
      _grund_ids
    fi
    return
  fi

  case "$words[2]" in
    show|refs) _grund_ids ;;
    *) _files ;;
  esac
}}

_grund "$@"
"#
    );
}

fn print_fish_completion() {
    println!(
        r#"# fish completion for grund
function __grund_complete_ids
    set -l token (commandline -ct)
    grund complete ids --prefix "$token" 2>/dev/null
end

complete -c grund -f -n "__fish_use_subcommand" -a "check show list refs cover fmt id init config agent-setup-instructions completions"
# IDs start with an uppercase kind, but workspace aliases are lowercase. Once a
# non-flag prefix exists, ask the helper for matching IDs/aliases.
complete -c grund -f -k -n "__fish_use_subcommand; and test -n (commandline -ct); and not string match -qr '^-' -- (commandline -ct)" -a "(__grund_complete_ids)"
complete -c grund -f -k -n "__fish_seen_subcommand_from show refs" -a "(__grund_complete_ids)"
"#
    );
}
