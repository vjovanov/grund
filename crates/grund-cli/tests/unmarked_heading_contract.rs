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

fn heading_project(name: &str) -> PathBuf {
    let root = root(name);
    fs::create_dir_all(root.join("docs")).expect("create docs home");
    fs::write(
        root.join("grund.toml"),
        concat!(
            "grund_config_version = 1\n\n",
            "[reference]\nstrict = true\n\n",
            "[id]\nformat = \"{kind}-{slug}\"\n\n",
            "[[kinds]]\nkind = \"FS\"\nfolder = \"docs\"\nindex = false\n\n",
            "[scan]\ninclude = [\"docs\"]\nextensions = [\"md\"]\n",
        ),
    )
    .expect("write config");
    root
}

fn write_doc(root: &Path, name: &str, body: &str) {
    fs::write(root.join("docs").join(name), body).expect("write Markdown fixture");
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
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

#[test]
fn unbounded_sibling_numbers_get_strictly_larger_unused_suggestions() {
    let root = heading_project("unbounded-suggestions");
    write_doc(
        &root,
        "FS-overflow-u32.md",
        concat!(
            "# FS-overflow-u32: u32 boundary\n\n",
            "Fixture prose.\n\n",
            "## 4294967295. Existing maximum\n\n",
            "## Missing sibling\n",
        ),
    );
    write_doc(
        &root,
        "FS-overflow-larger.md",
        concat!(
            "# FS-overflow-larger: Larger boundary\n\n",
            "Fixture prose.\n\n",
            "## 18446744073709551615. Existing maximum\n\n",
            "## Missing sibling\n",
        ),
    );

    let checked = run(&root, &["check", "--only", "unmarked-heading"]);
    assert_eq!(checked.status.code(), Some(0), "{}", stderr(&checked));
    assert_eq!(
        stdout(&checked),
        concat!(
            "docs/FS-overflow-larger.md:7: warning: unmarked heading inside FS-overflow-larger; number it (## 18446744073709551616. Missing sibling) as FS-overflow-larger.18446744073709551616, declare an ID, or use a bold label; this warning becomes an error in grund 0.15.0\n",
            "docs/FS-overflow-u32.md:7: warning: unmarked heading inside FS-overflow-u32; number it (## 4294967296. Missing sibling) as FS-overflow-u32.4294967296, declare an ID, or use a bold label; this warning becomes an error in grund 0.15.0\n",
        )
    );
}

#[test]
fn suggested_titles_preserve_hash_text_and_only_remove_atx_closers() {
    let root = heading_project("hash-titles");
    write_doc(
        &root,
        "FS-titles.md",
        concat!(
            "# FS-titles: Hash titles\n\n",
            "Fixture prose.\n\n",
            "## C#\n\n",
            "## C# ###\n",
        ),
    );

    let checked = run(&root, &["check", "--only", "unmarked-heading"]);
    assert_eq!(checked.status.code(), Some(0), "{}", stderr(&checked));
    assert_eq!(
        stdout(&checked),
        concat!(
            "docs/FS-titles.md:5: warning: unmarked heading inside FS-titles; number it (## 1. C#) as FS-titles.1, declare an ID, or use a bold label; this warning becomes an error in grund 0.15.0\n",
            "docs/FS-titles.md:7: warning: unmarked heading inside FS-titles; number it (## 2. C#) as FS-titles.2, declare an ID, or use a bold label; this warning becomes an error in grund 0.15.0\n",
        )
    );

    write_doc(
        &root,
        "FS-titles.md",
        concat!(
            "# FS-titles: Hash titles\n\n",
            "Fixture prose.\n\n",
            "## 1. C#\n\n",
            "## 2. C# ###\n",
        ),
    );
    let repaired = run(&root, &["check", "--only", "unmarked-heading"]);
    assert_eq!(repaired.status.code(), Some(0), "{}", stderr(&repaired));
    assert_eq!(stdout(&repaired), "success\n");
}

#[test]
fn titleless_heading_gets_a_self_valid_suggestion() {
    let root = heading_project("titleless-suggestion");
    write_doc(
        &root,
        "FS-titleless.md",
        concat!(
            "# FS-titleless: Titleless heading\n\n",
            "Fixture prose.\n\n",
            "##\n",
        ),
    );

    let checked = run(&root, &["check", "--only", "unmarked-heading"]);
    assert_eq!(checked.status.code(), Some(0), "{}", stderr(&checked));
    assert_eq!(
        stdout(&checked),
        "docs/FS-titleless.md:5: warning: unmarked heading inside FS-titleless; number it (## 1. Untitled) as FS-titleless.1, declare an ID, or use a bold label; this warning becomes an error in grund 0.15.0\n"
    );

    write_doc(
        &root,
        "FS-titleless.md",
        concat!(
            "# FS-titleless: Titleless heading\n\n",
            "Fixture prose.\n\n",
            "## 1. Untitled\n",
        ),
    );
    let repaired = run(&root, &["check", "--only", "unmarked-heading"]);
    assert_eq!(repaired.status.code(), Some(0), "{}", stderr(&repaired));
    assert_eq!(stdout(&repaired), "success\n");
}

#[test]
fn unmarked_heading_scan_does_not_shorten_show_or_list_slices() {
    let root = heading_project("read-only-slices");
    write_doc(
        &root,
        "FS-show-edge.md",
        concat!(
            "# FS-show-edge: Read-only slices\n\n",
            "Fixture prose.\n\n",
            "## Missing coordinate\n\n",
            "More prose.\n\n",
            "# Plain chapter\n\n",
            "Trailing prose.\n",
        ),
    );

    let shown = run(&root, &["show", "FS-show-edge", "--full"]);
    assert_eq!(shown.status.code(), Some(0), "{}", stderr(&shown));
    assert!(stdout(&shown).contains("# Plain chapter\n\nTrailing prose."));

    let listed = run(&root, &["list", "--size=lines"]);
    assert_eq!(listed.status.code(), Some(0), "{}", stderr(&listed));
    assert!(stdout(&listed).contains("FS-show-edge  docs/FS-show-edge.md:1  lines=5/5"));
}

#[test]
fn only_markdown_atx_headings_get_unmarked_heading_warnings() {
    let root = heading_project("atx-boundaries");
    write_doc(
        &root,
        "FS-atx.md",
        concat!(
            "# FS-atx: ATX boundaries\n",
            "## Zero-space indent\n",
            " ## One-space indent\n",
            "  ## Two-space indent\n",
            "   ## Three-space indent\n",
            "###### Six hashes\n",
            "##\tASCII tab separator\n",
            "    ## Four-space indent is code\n",
            "\t## Leading tab indent is code\n",
            "####### Seven hashes is not ATX\n",
            "######## Eight hashes is not ATX\n",
            "##No separator\n",
            "##\u{a0}Non-ASCII whitespace separator\n",
        ),
    );

    let checked = run(&root, &["check", "--only", "unmarked-heading"]);
    assert_eq!(checked.status.code(), Some(0), "{}", stderr(&checked));
    let output = stdout(&checked);
    let warned_lines = output
        .lines()
        .map(|line| {
            line.strip_prefix("docs/FS-atx.md:")
                .and_then(|tail| tail.split(':').next())
                .and_then(|line| line.parse::<usize>().ok())
                .unwrap_or_else(|| panic!("unexpected warning: {line}"))
        })
        .collect::<Vec<_>>();
    assert_eq!(warned_lines, vec![2, 3, 4, 5, 6, 7], "{output}");
}

#[test]
fn duplicate_declaration_bodies_allocate_suggestions_independently() {
    let root = heading_project("duplicate-owner-suggestions");
    write_doc(
        &root,
        "FS-duplicate-owner.md",
        concat!(
            "# FS-duplicate-owner: First site\n\n",
            "## Missing in first body\n\n",
            "# FS-duplicate-owner: Second site\n\n",
            "## Missing in second body\n",
        ),
    );

    let checked = run(&root, &["check", "--only", "unmarked-heading"]);
    assert_eq!(checked.status.code(), Some(0), "{}", stderr(&checked));
    assert_eq!(
        stdout(&checked),
        concat!(
            "docs/FS-duplicate-owner.md:3: warning: unmarked heading inside FS-duplicate-owner; number it (## 1. Missing in first body) as FS-duplicate-owner.1, declare an ID, or use a bold label; this warning becomes an error in grund 0.15.0\n",
            "docs/FS-duplicate-owner.md:7: warning: unmarked heading inside FS-duplicate-owner; number it (## 1. Missing in second body) as FS-duplicate-owner.1, declare an ID, or use a bold label; this warning becomes an error in grund 0.15.0\n",
        )
    );
}
