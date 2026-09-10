//! Binary-level compatibility contract for unmarked Markdown headings, their
//! 0.15.0 deadline, and managed-block v10 repair (§FS-check.4.14,
//! §FS-init.2.3.4.5, §RM-unmarked-heading-error).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/issue-132-unmarked-headings")
        .join(name);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create fixture root");
    root
}

fn run(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_grund"))
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap_or_else(|error| panic!("run grund {args:?}: {error}"))
}

fn version(text: &str) -> Vec<u32> {
    text.trim_end_matches("-dev")
        .split('.')
        .map(|part| part.parse::<u32>().expect("numeric version"))
        .collect()
}

#[test]
fn unmarked_heading_warning_deadline_is_ahead_of_the_running_version() {
    assert!(
        version(env!("CARGO_PKG_VERSION")) < version("0.15.0"),
        "this tree reached 0.15.0; land §RM-unmarked-heading-error instead of \
         shipping the warning past its deadline"
    );
}

#[test]
fn unmarked_heading_guidance_moves_v9_to_v10_without_a_config_bump() {
    let root = root("managed-v10");
    fs::write(
        root.join("grund.toml"),
        "grund_config_version = 1\nproject_name = \"fixture\"\n",
    )
    .expect("write config");
    fs::write(
        root.join("AGENTS.md"),
        concat!(
            "before\n",
            "<!-- BEGIN GRUND MANAGED BLOCK -->\n",
            "## Grounding with grund (v9)\n\n",
            "Legacy guidance.\n",
            "<!-- END GRUND MANAGED BLOCK -->\n",
            "after\n",
        ),
    )
    .expect("write v9 block");

    let initialized = run(&root, &["init", ".", "--agents-md"]);
    assert_eq!(
        initialized.status.code(),
        Some(0),
        "stdout:\n{}stderr:\n{}",
        String::from_utf8_lossy(&initialized.stdout),
        String::from_utf8_lossy(&initialized.stderr)
    );

    let agents = fs::read_to_string(root.join("AGENTS.md")).expect("read AGENTS.md");
    assert!(agents.starts_with("before\n"), "{agents}");
    assert!(agents.ends_with("after\n"), "{agents}");
    assert!(agents.contains("## Grounding with grund (v10)"), "{agents}");
    assert!(
        agents.contains("Every non-declaration heading inside a Markdown declaration body"),
        "{agents}"
    );
    assert!(agents.contains("bold labels remain"), "{agents}");

    let config = fs::read_to_string(root.join("grund.toml")).expect("read config");
    assert!(config.contains("grund_config_version = 1"), "{config}");
    assert!(!config.contains("unmarked_headings"), "{config}");
}
