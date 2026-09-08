/// The single and batch show usage page (§FS-cli.2, §FS-show.1).
fn print_show_help() {
    println!(
        "grund show — print one declaration's body by ID, so an agent pulls a single fact"
    );
    println!("into context without loading the whole document. `show` is the default command.");
    println!();
    println!(
        "Usage:  grund [show] <ID>[.<section>] [PATH] [--section S] [--brief|--toc|--full] [--format text|md|json] [--path PATH]"
    );
    println!(
        "        grund show --batch [--all] [PATH] --format=json [--brief|--toc|--full] [--path PATH]"
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
    println!(
        "  --batch                 read ordered {{\"id\":…,\"section\":…}} NDJSON queries from stdin"
    );
    println!("  --all                   with --batch, query every declaration and section");
    println!();
    println!(
        "Exit:  0 printed · 1 ID not found / ambiguous / broken stub / unknown section · 2 unknown project alias, or CLI error."
    );
    println!(
        "Batch: 0 all queries succeeded · 1 any query failed · 2 invocation, input, or scan error."
    );
    println!();
    println!("Examples:");
    println!("  grund FS-login                   # the lead — the cheap default");
    println!("  grund FS-login --toc             # lead + section map");
    println!("  grund FS-login.3.1               # the lead of that nested section");
    println!("  grund FS-login --full            # the whole declaration body");
    println!(
        "  printf '%s\\n' '{{\"id\":\"FS-login\"}}' | grund show --batch --format=json"
    );
    println!("  grund show --batch --all --format=json # every coordinate, one scan");
    println!();
    println!(
        "ID not found? `grund list` shows every declared ID; `grund id <KIND> \"…\"` proposes a new one."
    );
}
