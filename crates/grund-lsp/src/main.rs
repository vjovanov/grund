//! LSP binary entry point and pre-transport batch dispatch. §AR-lsp.4 §FS-lsp.2.4

mod integrations;

fn main() -> anyhow::Result<()> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        return grund_lsp::run();
    }
    if let Err(error) = integrations::dispatch(&args) {
        let message = format!("{error:#}").replace(['\r', '\n'], " ");
        eprintln!("error: {message}");
        std::process::exit(2);
    }
    Ok(())
}
