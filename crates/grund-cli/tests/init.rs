// Content-and-contract tests for `grund init`. The e2e harness covers the CLI surface
// (exit code + stderr listing); these tests cover what the *bytes on disk* look like
// after init runs: every emitted file exists, the `grund.toml` location matches the spec,
// the config validates, and `grund check` is clean against the freshly-scaffolded tree.
// Which entrypoint files a run selects is the sibling suite, `init_agent_entrypoints.rs`.

use std::fs;

#[path = "support/init_fixture.rs"]
mod init_fixture;

use init_fixture::{manifest_dir, run_grund, workdir};

const CITATION_DIRECTIONS_URL: &str =
    "https://github.com/vjovanov/grund/blob/main/docs/user-facing/citation-directions.md";

#[test]
fn init_default_writes_canonical_pair_and_passes_check() {
    // §FS-init.2.1 (default form) + §FS-config.1 (config file location) +
    // §FS-init.2.3.4.10 (reachable static citation-direction guidance).
    let target = workdir("init_default_writes_canonical_pair_and_passes_check");
    let output = run_grund(
        &["init", target.to_str().unwrap(), "--agents-md"],
        manifest_dir(),
    );
    assert!(
        output.status.success(),
        "init failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        target.join("AGENTS.md").is_file(),
        "AGENTS.md was not written"
    );
    assert!(
        target.join("grund.toml").is_file(),
        "grund.toml was not written; init generates the bare root form (§DF-config-file-location.2.3)"
    );
    assert!(
        !target.join(".agents/grund.toml").exists(),
        "init must NOT write .agents/grund.toml — that form is discovered, not generated"
    );

    let agents = fs::read_to_string(target.join("AGENTS.md")).expect("read AGENTS.md");
    let grund_toml = fs::read_to_string(target.join("grund.toml")).expect("read grund.toml");
    for (surface, contents) in [("AGENTS.md", agents), ("grund.toml", grund_toml)] {
        assert!(
            contents.contains(CITATION_DIRECTIONS_URL),
            "fresh {surface} should link to the reachable canonical citation directions page"
        );
        assert!(
            !contents.contains("See docs/user-facing/citation-directions.md"),
            "fresh {surface} must not point at an absent repo-local page"
        );
    }

    let validate = run_grund(
        &["config", "validate", target.to_str().unwrap()],
        manifest_dir(),
    );
    assert!(
        validate.status.success(),
        "init's grund.toml does not validate:\n{}",
        String::from_utf8_lossy(&validate.stderr)
    );
}

#[test]
fn init_docs_form_emits_full_scaffold_and_check_is_clean() {
    // §FS-init.2.1 (--docs form) + §FS-init.2.3.4.10 (reachable static
    // citation-direction guidance). The scaffolded tree must satisfy `grund check` —
    // i.e. the canonical AGENTS.md + grund.toml + docs skeleton is internally consistent.
    let target = workdir("init_docs_form_emits_full_scaffold_and_check_is_clean");
    let output = run_grund(
        &[
            "init",
            target.to_str().unwrap(),
            "--docs",
            "--agents-md",
            "--name",
            "DemoProject",
        ],
        manifest_dir(),
    );
    assert!(
        output.status.success(),
        "init --docs failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let expected = [
        "AGENTS.md",
        "grund.toml",
        "docs/grund.md",
        "docs/goals.md",
        "docs/roadmap.md",
        "docs/changelog.md",
        "requirements.md",
        "docs/architecture/README.md",
        "docs/decisions/architectural/README.md",
        "docs/decisions/functional/README.md",
        "tests/e2e/README.md",
        "tests/integration/.gitkeep",
    ];
    for rel in expected {
        assert!(target.join(rel).exists(), "init --docs did not write {rel}");
    }

    let agents = fs::read_to_string(target.join("AGENTS.md")).expect("read AGENTS.md");
    assert!(
        agents.contains("DemoProject"),
        "AGENTS.md must interpolate the --name into the H1 / opening sentence"
    );

    let grund_toml = fs::read_to_string(target.join("grund.toml")).expect("read grund.toml");
    assert!(
        grund_toml.contains("project_name = \"DemoProject\""),
        "grund.toml must carry project_name from --name"
    );
    assert!(
        agents.contains(CITATION_DIRECTIONS_URL),
        "--docs AGENTS.md should link to the reachable canonical citation directions page"
    );
    assert!(
        grund_toml.contains(CITATION_DIRECTIONS_URL),
        "--docs grund.toml should link to the reachable canonical citation directions page"
    );
    assert!(
        !agents.contains("See docs/user-facing/citation-directions.md")
            && !grund_toml.contains("See docs/user-facing/citation-directions.md"),
        "--docs output must not point at an absent repo-local page"
    );

    let check = run_grund(&["check", target.to_str().unwrap()], manifest_dir());
    assert!(
        check.status.success(),
        "freshly init'd tree should be grund-clean but produced:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}

#[test]
fn init_docs_default_requirements_file_is_scanned() {
    // §FS-init.2.1 / §FS-config.3.5: the generated FS home is in the generated
    // scan roots, so declarations added where `init` points users are resolvable.
    let target = workdir("init_docs_default_requirements_file_is_scanned");
    let output = run_grund(
        &["init", target.to_str().unwrap(), "--docs"],
        manifest_dir(),
    );
    assert!(
        output.status.success(),
        "init --docs failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    fs::write(
        target.join("requirements.md"),
        "# Requirements\n\n## FS-001-session: Sessions can be created\n",
    )
    .expect("write requirements.md");
    fs::create_dir_all(target.join("src")).expect("create src");
    fs::write(
        target.join("src/lib.rs"),
        "/// Creates a user session. §FS-001-session\npub fn create_session() {}\n",
    )
    .expect("write src/lib.rs");

    let check = run_grund(&["check", target.to_str().unwrap()], manifest_dir());
    assert!(
        check.status.success(),
        "FS declaration in generated requirements.md should resolve:\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
}

#[test]
fn init_description_flag_writes_config_key() {
    // §FS-init.1 + §FS-init.2.4: `--description` replaces the commented
    // teaching line in the generated config with the real key.
    let target = workdir("init_description_flag_writes_config_key");
    let output = run_grund(
        &[
            "init",
            target.to_str().unwrap(),
            "--name",
            "api",
            "--description",
            "Payment API service",
        ],
        manifest_dir(),
    );
    assert!(
        output.status.success(),
        "init --description failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let grund_toml = fs::read_to_string(target.join("grund.toml")).expect("read grund.toml");
    assert!(
        grund_toml.contains("project_description = \"Payment API service\""),
        "grund.toml must carry project_description from --description"
    );
    assert!(
        !grund_toml.contains("# project_description ="),
        "the teaching comment must be replaced by the real key"
    );

    let validate = run_grund(
        &["config", "validate", target.to_str().unwrap()],
        manifest_dir(),
    );
    assert!(
        validate.status.success(),
        "generated config with project_description must validate: {}",
        String::from_utf8_lossy(&validate.stderr)
    );
}

#[test]
fn init_description_flag_rejects_multiline_value() {
    // §FS-init.1: `--description` mirrors the config-side single-line rule.
    let target = workdir("init_description_flag_rejects_multiline_value");
    let output = run_grund(
        &[
            "init",
            target.to_str().unwrap(),
            "--description",
            "first line\nsecond line",
        ],
        manifest_dir(),
    );
    assert_eq!(
        output.status.code(),
        Some(2),
        "multi-line --description must be a CLI error"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--description must be a single line"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !target.join("grund.toml").exists(),
        "no file may be written on a rejected --description"
    );
}

#[test]
fn init_failed_docs_write_reports_prior_progress() {
    // §FS-init.2.2 / §FS-init.4: init reports touched paths as it goes. If a
    // later scaffold write fails, the user still needs the transcript for the
    // files that were already created.
    let target = workdir("init_failed_docs_write_reports_prior_progress");
    fs::write(target.join("docs"), "not a directory").expect("write docs file");

    let output = run_grund(
        &["init", target.to_str().unwrap(), "--docs"],
        manifest_dir(),
    );
    assert!(
        !output.status.success(),
        "init should fail when docs/ is a file"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("wrote AGENTS.md"),
        "stderr should include prior AGENTS.md write, got:\n{stderr}"
    );
    assert!(
        stderr.contains("wrote grund.toml"),
        "stderr should include prior config write, got:\n{stderr}"
    );
    assert!(
        stderr.contains("error: "),
        "stderr should include final error, got:\n{stderr}"
    );
    assert!(target.join("AGENTS.md").is_file());
    assert!(target.join("grund.toml").is_file());
}

#[test]
fn init_generated_config_comments_list_constrained_values() {
    // §FS-init.2.4: the generated config is a teaching surface, so non-boolean
    // constrained keys carry inline comments listing their accepted values.
    let target = workdir("init_generated_config_comments_list_constrained_values");
    let output = run_grund(&["init", target.to_str().unwrap()], manifest_dir());
    assert!(
        output.status.success(),
        "init failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let grund_toml = fs::read_to_string(target.join("grund.toml")).expect("read grund.toml");
    for expected in [
        "inline_style = \"citation-with-note\" # citation-with-note | citation-only",
        "inline_note_layout = \"any\" # any | citation-first-colon",
        "inline_note_layout_check = \"off\" # off | warn | error",
        "section_heading_levels = \"strict\" # strict | warn | loose",
        "format = \"text\" # text | json",
        "anchor_format = \"github\" # github | gitlab | mkdocs | pandoc | none",
    ] {
        assert!(
            grund_toml.contains(expected),
            "generated config should include `{expected}`, got:\n{grund_toml}"
        );
    }
    assert!(
        !grund_toml.contains("# true | false"),
        "generated config should not enumerate boolean values, got:\n{grund_toml}"
    );

    let validate = run_grund(
        &["config", "validate", target.to_str().unwrap()],
        manifest_dir(),
    );
    assert!(
        validate.status.success(),
        "commented generated config should validate:\n{}",
        String::from_utf8_lossy(&validate.stderr)
    );
}

/// §FS-inline-citation-style.5: the rendered house style closes with the
/// doc-comment sentence at every `inline_style`, so the agent reading the
/// block knows the budgets stop where documentation starts
/// (§FS-inline-citation-style.1.1).
#[test]
fn init_agents_block_closes_the_house_style_with_the_doc_comment_sentence() {
    let target = workdir("init_agents_block_closes_the_house_style_with_the_doc_comment_sentence");
    let output = run_grund(&["init", target.to_str().unwrap()], manifest_dir());
    assert!(
        output.status.success(),
        "init failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let agents = fs::read_to_string(target.join("AGENTS.md")).expect("read AGENTS.md");
    let expected = "- **Inline citation style.** Inline notes: ≤ 1 line preferred, hard cap 3 lines; ≤ 100 columns. A note is one comment block: a blank line splits it, an empty comment line does not. Doc-comments (`///`, `//!`, `/** */`, a docstring, a comment right above a definition) are documentation, not notes: they are never measured, so cite in-sentence there.";
    assert!(
        agents.contains(expected),
        "AGENTS.md should carry the rendered house style `{expected}`, got:\n{agents}"
    );
}

#[test]
fn init_agents_guidance_uses_existing_configured_artifact_homes() {
    let target = workdir("init_agents_guidance_uses_existing_configured_artifact_homes");
    fs::create_dir_all(target.join(".agents")).expect("create .agents");
    fs::write(
        target.join(".agents/grund.toml"),
        r#"grund_config_version = 1

[scan]
include = ["specs", "records", "crates"]

[[kinds]]
kind = "FS"
folder = "specs"
title = "Product spec"

[[kinds]]
kind = "ADR"
folder = "records/adr"
title = "Architecture decision"
"#,
    )
    .expect("write custom grund.toml");

    let output = run_grund(
        &["init", target.to_str().unwrap(), "--name", "Configured"],
        manifest_dir(),
    );
    assert!(
        output.status.success(),
        "init failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let agents = fs::read_to_string(target.join("AGENTS.md")).expect("read AGENTS.md");
    assert!(
        agents.contains("- [FS](specs): Product spec"),
        "AGENTS.md should describe configured spec homes:\n{agents}"
    );
    assert!(
        agents.contains("- [ADR](records/adr): Architecture decision"),
        "AGENTS.md should describe configured decision homes:\n{agents}"
    );
    assert!(
        !agents.contains("docs/architecture/") && !agents.contains("docs/decisions/"),
        "AGENTS.md must not introduce canonical docs folders when specs are configured elsewhere"
    );
    assert!(
        !agents.contains("`grund` scans:"),
        "AGENTS.md must not surface scan scope (§FS-init.2.3.4.4):\n{agents}"
    );
}

#[test]
fn init_docs_existing_implicit_legacy_config_uses_legacy_fs_home() {
    // §FS-config.2 / §FS-init.2.1: existing configs without explicit kind homes
    // keep the legacy FS folder, and `init --docs` must scaffold that effective home.
    let target = workdir("init_docs_existing_implicit_legacy_config_uses_legacy_fs_home");
    fs::create_dir_all(target.join(".agents")).expect("create .agents");
    fs::write(
        target.join(".agents/grund.toml"),
        "grund_config_version = 1\n",
    )
    .expect("write legacy implicit config");

    let output = run_grund(
        &["init", target.to_str().unwrap(), "--docs"],
        manifest_dir(),
    );
    assert!(
        output.status.success(),
        "init --docs failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("wrote docs/functional-spec/README.md"),
        "legacy implicit config should scaffold legacy FS folder, got:\n{stderr}"
    );
    assert!(
        stderr.contains("then add it under docs/functional-spec"),
        "next guidance should target the effective legacy FS folder, got:\n{stderr}"
    );
    assert!(
        !target.join("requirements.md").exists(),
        "legacy implicit config must not scaffold the new FS file"
    );
}

#[test]
fn init_workspace_companion_only_omits_self_and_uses_marker() {
    // §FS-init.2.3.4.15: a companion-only workspace init omits self just like
    // canonical init, retains foreign rows, and uses the local citation marker.
    let root = workdir("init_workspace_companion_only_omits_self_and_uses_marker");
    fs::create_dir_all(root.join(".agents")).expect("create root config dir");
    fs::write(
        root.join(".agents/grund.toml"),
        "project_name = \"root\"\n\n[workspace]\nmembers = [\"apps/api\"]\n",
    )
    .expect("write workspace config");
    let api = root.join("apps/api");
    fs::create_dir_all(api.join(".agents")).expect("create api config dir");
    fs::write(
        api.join(".agents/grund.toml"),
        "[reference]\nmarker = \"@\"\nstrict = true\n",
    )
    .expect("write api config");

    let output = run_grund(&["init", api.to_str().unwrap(), "--claude"], manifest_dir());
    assert!(
        output.status.success(),
        "init failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !api.join("AGENTS.md").exists(),
        "explicit companion-only init should not create AGENTS.md"
    );

    let claude = fs::read_to_string(api.join("CLAUDE.md")).expect("read CLAUDE.md");
    assert!(
        claude.contains("Cross-project citations use @alias/<ID>."),
        "workspace citation hint should use the configured marker:\n{claude}"
    );
    assert!(
        !claude.contains("`api`"),
        "companion-only init must omit its canonical self project:\n{claude}"
    );
    assert!(
        claude.contains("- [`root`](../../) *(not yet initialized)*"),
        "the foreign root row should retain its relative link and annotation:\n{claude}"
    );
}

#[test]
fn init_rerun_on_current_repo_writes_nothing_and_reports_exists() {
    // §FS-init.2.2 / §FS-init.2.3: re-running `grund init` on a repo whose managed
    // AGENTS.md block already matches the current render rewrites nothing — the
    // file's bytes are untouched and it is reported with `exists `, not `updated `.
    let target = workdir("init_rerun_on_current_repo_writes_nothing_and_reports_exists");
    let first = run_grund(&["init", target.to_str().unwrap()], manifest_dir());
    assert!(first.status.success());

    let agents_before = fs::read(target.join("AGENTS.md")).unwrap();
    let toml_before = fs::read(target.join("grund.toml")).unwrap();

    let second = run_grund(&["init", target.to_str().unwrap()], manifest_dir());
    assert!(second.status.success());
    let stderr = String::from_utf8_lossy(&second.stderr);
    assert!(
        stderr.contains("exists AGENTS.md"),
        "second `grund init` should report `exists AGENTS.md`, got:\n{stderr}"
    );
    assert!(
        !stderr.contains("updated AGENTS.md") && !stderr.contains("wrote AGENTS.md"),
        "second `grund init` must not rewrite an already-current AGENTS.md, got:\n{stderr}"
    );
    assert!(
        stderr.contains("exists grund.toml"),
        "second `grund init` should report `exists grund.toml`, got:\n{stderr}"
    );

    assert_eq!(
        fs::read(target.join("AGENTS.md")).unwrap(),
        agents_before,
        "AGENTS.md bytes changed on a no-op re-init"
    );
    assert_eq!(
        fs::read(target.join("grund.toml")).unwrap(),
        toml_before,
        "grund.toml bytes changed on a no-op re-init"
    );
}

/// §FS-init.2.4 / §FS-init.3: `.agents/grund.toml` is the repo's config, not a
/// scaffold artifact — `grund init --force` regenerates AGENTS.md but leaves an
/// existing config byte-for-byte intact and reports it with `exists `, never
/// `wrote `. (Overwriting it with the canonical template was a footgun.)
#[test]
fn init_force_never_overwrites_an_existing_config() {
    let target = workdir("init_force_never_overwrites_an_existing_config");
    fs::create_dir_all(target.join(".agents")).expect("create .agents");
    let custom_config = "grund_config_version = 1\n\
        project_name = \"Custom\"\n\n\
        [reference]\nstrict = true\n\n\
        [[kinds]]\nkind = \"SPEC\"\nfolder = \"specs\"\ntitle = \"Spec\"\n";
    fs::write(target.join(".agents/grund.toml"), custom_config).expect("write custom config");

    let output = run_grund(
        &["init", target.to_str().unwrap(), "--force"],
        manifest_dir(),
    );
    assert!(
        output.status.success(),
        "init --force failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("exists .agents/grund.toml"),
        "`grund init --force` must report `exists .agents/grund.toml`, got:\n{stderr}"
    );
    assert!(
        !stderr.contains("wrote .agents/grund.toml"),
        "`grund init --force` must not overwrite an existing config, got:\n{stderr}"
    );
    assert_eq!(
        fs::read_to_string(target.join(".agents/grund.toml")).unwrap(),
        custom_config,
        "`grund init --force` left .agents/grund.toml byte-for-byte? it did not"
    );
}

#[test]
fn init_is_byte_deterministic() {
    // §FS-non-goals.13: same input → byte-identical output.
    let a = workdir("init_is_byte_deterministic_a");
    let b = workdir("init_is_byte_deterministic_b");
    for target in [&a, &b] {
        let out = run_grund(
            &["init", target.to_str().unwrap(), "--name", "Same"],
            manifest_dir(),
        );
        assert!(out.status.success());
    }
    let agents_a = fs::read(a.join("AGENTS.md")).unwrap();
    let agents_b = fs::read(b.join("AGENTS.md")).unwrap();
    assert_eq!(agents_a, agents_b, "AGENTS.md must be byte-identical");
    let toml_a = fs::read(a.join("grund.toml")).unwrap();
    let toml_b = fs::read(b.join("grund.toml")).unwrap();
    assert_eq!(toml_a, toml_b, "grund.toml must be byte-identical");
}

/// §FS-init.1 / §FS-init.2.2: --dry-run reports what a real run would do
/// (would-write / would-append / would-update) and leaves the working tree
/// untouched. Re-running without --dry-run then produces the same on-disk
/// outcome as a single non-dry-run would.
#[test]
fn init_dry_run_writes_no_files_and_reports_would_prefixes() {
    let target = workdir("init_dry_run_writes_no_files_and_reports_would_prefixes");
    let dry = run_grund(
        &["init", target.to_str().unwrap(), "--dry-run"],
        manifest_dir(),
    );
    assert!(
        dry.status.success(),
        "init --dry-run failed: stderr={}",
        String::from_utf8_lossy(&dry.stderr)
    );
    let stderr = String::from_utf8_lossy(&dry.stderr);
    assert!(
        stderr.contains("would-write AGENTS.md") && stderr.contains("would-write grund.toml"),
        "dry-run should report `would-write …` for new files, got:\n{stderr}"
    );
    assert!(
        !stderr.contains("\nwrote ") && !stderr.contains("\nappended "),
        "dry-run must not use the real-run verbs, got:\n{stderr}"
    );
    assert!(
        !target.join("AGENTS.md").exists() && !target.join("grund.toml").exists(),
        "dry-run must not write anything to disk"
    );

    // Real run on the same target should now write the files cleanly.
    let real = run_grund(&["init", target.to_str().unwrap()], manifest_dir());
    assert!(real.status.success());
    assert!(target.join("AGENTS.md").is_file());
    assert!(target.join("grund.toml").is_file());
}

/// §FS-init.2.2: when every reported path is `exists ` (and no would-… lines
/// were emitted), the `next:` guidance block is suppressed — the user has
/// a complete setup, so there is nothing to teach. This holds for both
/// real runs and dry-runs.
#[test]
fn init_dry_run_on_current_repo_suppresses_next_block() {
    let target = workdir("init_dry_run_on_current_repo_suppresses_next_block");
    let first = run_grund(&["init", target.to_str().unwrap()], manifest_dir());
    assert!(first.status.success());

    let second = run_grund(&["init", target.to_str().unwrap()], manifest_dir());
    assert!(second.status.success());
    let stderr = String::from_utf8_lossy(&second.stderr);
    assert!(
        stderr.contains("exists AGENTS.md") && stderr.contains("exists grund.toml"),
        "second init should report `exists` for both managed paths, got:\n{stderr}"
    );
    assert!(
        !stderr.contains("\nnext:") && !stderr.contains("see "),
        "all-exists run must suppress the `next:` block, got:\n{stderr}"
    );

    let dry = run_grund(
        &["init", target.to_str().unwrap(), "--dry-run"],
        manifest_dir(),
    );
    assert!(dry.status.success());
    let dry_stderr = String::from_utf8_lossy(&dry.stderr);
    assert!(
        !dry_stderr.contains("\nnext:") && !dry_stderr.contains("see "),
        "all-exists dry-run must also suppress the `next:` block, got:\n{dry_stderr}"
    );
}

#[test]
fn init_dry_run_with_docs_previews_scaffold_without_writing() {
    // §FS-init.1 / §FS-init.2.2: --dry-run composes with --docs — every docs
    // scaffold path is reported as `would-write` and no file lands on disk.
    let target = workdir("init_dry_run_with_docs_previews_scaffold_without_writing");
    let dry = run_grund(
        &["init", target.to_str().unwrap(), "--docs", "--dry-run"],
        manifest_dir(),
    );
    assert!(
        dry.status.success(),
        "init --docs --dry-run failed: stderr={}",
        String::from_utf8_lossy(&dry.stderr)
    );
    let stderr = String::from_utf8_lossy(&dry.stderr);
    for rel in [
        "AGENTS.md",
        "grund.toml",
        "docs/grund.md",
        "docs/goals.md",
        "docs/roadmap.md",
        "docs/changelog.md",
        "requirements.md",
        "docs/architecture/README.md",
        "docs/decisions/architectural/README.md",
        "docs/decisions/functional/README.md",
        "tests/e2e/README.md",
        "tests/integration/.gitkeep",
    ] {
        assert!(
            stderr.contains(&format!("would-write {rel}")),
            "dry-run --docs should preview `would-write {rel}`, got:\n{stderr}"
        );
        assert!(
            !target.join(rel).exists(),
            "dry-run --docs must not write {rel} to disk"
        );
    }
    assert!(
        !stderr.contains("\nwrote "),
        "dry-run --docs must not use the real-run verb, got:\n{stderr}"
    );
}

/// §FS-init.1 / §FS-init.2.2: --force --dry-run takes the rewrite path
/// (instead of update-in-place) and previews `would-write AGENTS.md`
/// without changing the file's bytes on disk. The config is the exception:
/// --force never overwrites it, so dry-run reports `exists`.
#[test]
fn init_force_dry_run_previews_canonical_rewrite() {
    let target = workdir("init_force_dry_run_previews_canonical_rewrite");
    let first = run_grund(&["init", target.to_str().unwrap()], manifest_dir());
    assert!(first.status.success());

    let agents_before = fs::read(target.join("AGENTS.md")).unwrap();
    let toml_before = fs::read(target.join("grund.toml")).unwrap();

    let dry = run_grund(
        &["init", target.to_str().unwrap(), "--force", "--dry-run"],
        manifest_dir(),
    );
    assert!(
        dry.status.success(),
        "init --force --dry-run failed: stderr={}",
        String::from_utf8_lossy(&dry.stderr)
    );
    let stderr = String::from_utf8_lossy(&dry.stderr);
    assert!(
        stderr.contains("would-write AGENTS.md"),
        "--force --dry-run should preview the canonical rewrite, got:\n{stderr}"
    );
    assert!(
        stderr.contains("exists grund.toml"),
        "--force never overwrites the config, even under dry-run, got:\n{stderr}"
    );
    assert!(
        !stderr.contains("\nwrote AGENTS.md"),
        "dry-run must not use the real-run verb, got:\n{stderr}"
    );
    assert_eq!(
        fs::read(target.join("AGENTS.md")).unwrap(),
        agents_before,
        "--force --dry-run must not modify AGENTS.md"
    );
    assert_eq!(
        fs::read(target.join("grund.toml")).unwrap(),
        toml_before,
        "--force --dry-run must not modify the config"
    );
}

/// §FS-init.2.3: generated output must pass `grund check` unmodified, even
/// when the entrypoint itself is inside the scan scope of a strict repo —
/// the worked citation example is `<§>`-escaped, not a live dangling
/// reference (the grund init → grund check → grund init wedge of the
/// pre-v4 template).
#[test]
fn init_output_passes_check_when_agents_md_is_scanned() {
    let target = workdir("init_output_passes_check_when_agents_md_is_scanned");
    fs::create_dir_all(target.join(".agents")).expect("create .agents");
    fs::write(
        target.join(".agents/grund.toml"),
        r#"grund_config_version = 1

[reference]
strict = true

[scan]
include = ["AGENTS.md", "docs"]

[[kinds]]
kind = "FS"
folder = "docs/functional-spec"
title = "Spec"
"#,
    )
    .expect("write config");
    fs::create_dir_all(target.join("docs")).expect("create docs");
    fs::write(target.join("docs/.keep"), "").expect("write docs/.keep");

    let output = run_grund(&["init", target.to_str().unwrap()], manifest_dir());
    assert!(
        output.status.success(),
        "init failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let check = run_grund(&["check", target.to_str().unwrap()], manifest_dir());
    assert!(
        check.status.success(),
        "freshly generated AGENTS.md must pass check unmodified:\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );

    // Re-running init must also be a no-op, not a refresh loop.
    let second = run_grund(&["init", target.to_str().unwrap()], manifest_dir());
    assert!(second.status.success());
    let stderr = String::from_utf8_lossy(&second.stderr);
    assert!(
        stderr.contains("exists AGENTS.md"),
        "second init should be a no-op, got:\n{stderr}"
    );
}
