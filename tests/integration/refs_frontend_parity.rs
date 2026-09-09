//! §AR-bindings.2: the published CLI and deprecated core process adapter keep
//! §FS-refs.4 launch-error precedence byte-identical.

mod binaries;

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

fn fixture() -> PathBuf {
    let root = binaries::repo_root().join("target/refs-frontend-parity");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("docs")).expect("create fixture docs");
    fs::write(
        root.join("grund.toml"),
        "grund_config_version = 1\n\
         [reference]\nstrict = false\n\n\
         [id]\nformat = \"{kind}-{number}-{slug}\"\n\n\
         [scan]\ninclude = [\"docs\"]\n",
    )
    .expect("write fixture config");
    root
}

fn run(binary: PathBuf, root: &PathBuf) -> Output {
    Command::new(binary)
        .args(["refs", "FS-bar", "--format", "yaml"])
        .arg(root)
        .output()
        .expect("run refs adapter")
}

#[test]
fn unsupported_format_wins_over_resolver_rejection_in_both_process_adapters() {
    let root = fixture();
    let public = run(binaries::grund(), &root);
    let compat = run(binaries::grund_core_compat(), &root);

    assert_eq!(public.status.code(), Some(2));
    assert_eq!(public.stdout, b"");
    assert_eq!(public.stderr, b"error: unsupported refs format `yaml`\n");
    assert_eq!(compat.status.code(), public.status.code());
    assert_eq!(compat.stdout, public.stdout);
    assert_eq!(compat.stderr, public.stderr);
}
