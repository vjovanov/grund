/// `grund complete <subcommand>` — the namespace for internal completion helpers
/// the generated shell scripts call (§FS-completions.2).
fn command_complete(args: &[String]) -> ExitCode {
    match args.first().map(|arg| arg.as_str()) {
        Some("ids") => command_complete_ids(&args[1..]),
        _ => {
            eprintln!("error: expected `complete ids`");
            ExitCode::from(2)
        }
    }
}

/// `grund complete ids [--prefix P] [--sections] [path]` — the dynamic helper a
/// shell completion calls on every tab press (§FS-completions.2): emit declared
/// IDs (or `ID.section` candidates) matching the prefix, one per line. Scan/config
/// failures exit `0` silently so a broken repo never smears diagnostics across the
/// prompt; output is deterministic (§FS-completions.3).
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

    // Completion is called on every tab press. Config or scan failures must not
    // smear diagnostics across the prompt; explicit flag misuse above is still a
    // normal CLI error because it is a bug in the installed completion script.
    let context = match load_workspace_context(&path, path_provided) {
        Ok(context) => context,
        Err(_) => return ExitCode::SUCCESS,
    };
    // §FS-workspace.8.4: split the prefix on the first `/` — the left is an
    // alias, the right is the alias's project ID-prefix. A prefix without a
    // slash completes the current project's IDs and, in workspace mode,
    // also emits one trailing-slash candidate per known alias so the shell
    // can advance from `api` → `api/`.
    let current_config = context
        .current_project()
        .map(|project| &project.config)
        .unwrap_or_else(|| context.render_config());
    if let Some(slash) = prefix.find('/') {
        let (alias_prefix, id_prefix) = prefix.split_at(slash);
        let id_prefix = &id_prefix[1..];
        if !context.workspace_loaded {
            return ExitCode::SUCCESS;
        }
        let Some(project) = context.project_by_alias(alias_prefix) else {
            return ExitCode::SUCCESS;
        };
        let complete_sections =
            force_sections || id_prefix.contains(&project.config.section_separator);
        let mut candidates = BTreeSet::new();
        for (id, decls) in &project.findings.declarations {
            let rendered = render_id(&project.config, id);
            if complete_sections {
                for decl in decls {
                    for section in decl.sections.keys() {
                        candidates.insert(format!(
                            "{}/{}{}{}",
                            alias_prefix, rendered, project.config.section_separator, section
                        ));
                    }
                }
            } else {
                candidates.insert(format!("{}/{}", alias_prefix, rendered));
            }
        }
        for candidate in candidates {
            if candidate.starts_with(&prefix) {
                println!("{candidate}");
            }
        }
        return ExitCode::SUCCESS;
    }

    let complete_sections = force_sections || prefix.contains(&current_config.section_separator);
    let mut candidates = BTreeSet::new();
    if let Some(current_project) = context.current_project() {
        for (id, decls) in &current_project.findings.declarations {
            let rendered = render_id(current_config, id);
            if complete_sections {
                for decl in decls {
                    for section in decl.sections.keys() {
                        candidates.insert(format!(
                            "{}{}{}",
                            rendered, current_config.section_separator, section
                        ));
                    }
                }
            } else {
                candidates.insert(rendered);
            }
        }
    }
    if context.workspace_loaded {
        // §FS-workspace.8.4: alias-as-candidate, trailing slash signals the
        // shell to keep going rather than insert a space. The current
        // project's own alias is emitted too — typing `root/` reaches the
        // same IDs as bare prefixes do, and a script that wants the
        // qualified form gets it the same way every alias is reached.
        for alias in context.aliases() {
            candidates.insert(format!("{alias}/"));
        }
    }

    for candidate in candidates {
        if candidate.starts_with(&prefix) {
            println!("{candidate}");
        }
    }
    ExitCode::SUCCESS
}

/// `grund completions <bash|zsh|fish>` — print the completion script for one shell
/// to stdout, ready to `source` (§FS-completions.1, §FS-completions.4). The scripts
/// call back into `grund complete ids` for the dynamic ID list.
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

/// The bash completion script: subcommand + flag completion, with ID queries,
/// explicit `show`, and `refs` wired to `grund complete ids`
/// (§FS-completions.1, §FS-completions.2).
fn print_bash_completion() {
    // §FS-workspace.8.4: when any candidate ends in `/` (a workspace alias
    // continuation), call `compopt -o nospace` so a Tab from `api` advances
    // to `api/` without inserting a trailing space.
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
        local commands=($(compgen -W "check show list refs cover fmt id init config agent-setup-instructions completions integrations" -- "$cur"))
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

/// The zsh completion script — the zsh counterpart of `print_bash_completion`
/// (§FS-completions.1, §FS-completions.2).
fn print_zsh_completion() {
    // §FS-workspace.8.4: partition slash-suffixed candidates (workspace
    // alias continuations) from bare-ID candidates so alias completions
    // do not append a trailing space — `_describe` is reserved for the
    // bare-ID batch; aliases land via `compadd -S ''` so `api` advances
    // to `api/` on the next Tab.
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
    'integrations:print or install clickable-citation integrations'
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

/// The fish completion script — `complete -c grund …` lines, ID arguments wired to
/// `grund complete ids` (§FS-completions.1, §FS-completions.2).
fn print_fish_completion() {
    // §FS-workspace.8.4: fish's `complete -k` keeps candidates verbatim
    // and (with `-f` to skip file completion) does not auto-append a
    // space after a `/`-terminated candidate — perfect for workspace
    // alias continuations.
    println!(
        r#"# fish completion for grund
function __grund_complete_ids
    set -l token (commandline -ct)
    grund complete ids --prefix "$token" 2>/dev/null
end

complete -c grund -f -n "__fish_use_subcommand" -a "check show list refs cover fmt id init config agent-setup-instructions completions integrations"
# IDs start with an uppercase kind, but workspace aliases are lowercase. Once a
# non-flag prefix exists, ask the helper for matching IDs/aliases.
complete -c grund -f -k -n "__fish_use_subcommand; and test -n (commandline -ct); and not string match -qr '^-' -- (commandline -ct)" -a "(__grund_complete_ids)"
complete -c grund -f -k -n "__fish_seen_subcommand_from show refs" -a "(__grund_complete_ids)"
"#
    );
}
