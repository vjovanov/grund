//! LSP binary entry point. §AR-lsp.4

fn main() -> anyhow::Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print!(
            "grund-lsp {}\n\n\
Language Server Protocol server for grund citations.\n\n\
USAGE:\n    grund-lsp\n\n\
The server speaks LSP over stdio and is normally spawned by an editor.\n\
Configure your editor's LSP client to run this binary in the workspace root.\n\n\
OPTIONS:\n    -h, --help       Print this help text\n    -V, --version    Print version\n",
            env!("CARGO_PKG_VERSION")
        );
        return Ok(());
    }
    if args.iter().any(|arg| arg == "-V" || arg == "--version") {
        println!("grund-lsp {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if let Some(arg) = args.first() {
        anyhow::bail!("unexpected argument `{arg}`; use --help for usage");
    }
    grund_lsp::run()
}
