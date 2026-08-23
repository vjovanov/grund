/// Test module: the caution a run earns when it read files and matched nothing
/// in them (§FS-check.4.5, §DF-nothing-recognized). The empty scan next door
/// (§FS-check.2.2) is about a scope that found no files; every case here is
/// about a scope that found them and recognized none of their content.
#[cfg(test)]
mod tests_nothing_recognized {
    use super::*;
    use super::tests_support::*;

    const DEFAULT_CONFIG: &str =
        "grund_config_version = 1\nproject_name = \"acme\"\n\n[scan]\ninclude = [\"docs\"]\n";

    fn caution(run: &CheckRun) -> Option<&Diagnostic> {
        run.report
            .warnings
            .iter()
            .find(|diagnostic| diagnostic.code == "nothing-recognized")
    }

    /// §FS-check.4.5, the defect: a docs tree written slug-only under the default
    /// `{kind}-{number}-{slug}` declares nothing, cites nothing, and used to print
    /// the same word as a tree that checked clean.
    #[test]
    fn a_tree_written_for_another_id_format_stops_reporting_success() {
        let root = test_root("a_tree_written_for_another_id_format_stops_reporting_success");
        write(&root.join("grund.toml"), DEFAULT_CONFIG);
        write(&root.join("docs/FS-alpha.md"), "# FS-alpha: The alpha spec\n\nBody.\n");
        write(&root.join("docs/FS-beta.md"), "# FS-beta: The beta spec\n\nBody.\n");

        let run = check_run(&root, false);
        let caution = caution(&run).expect("a tree that recognized nothing earns the caution");
        assert!(
            run.report.errors.is_empty(),
            "§DF-nothing-recognized.2.2: the exit code is untouched — what the caution takes away is the `success` marker"
        );
        assert_eq!(caution.path, None, "§FS-check.2.1.1: a CLI-level message names no site");
        assert_eq!(caution.line, None);
        assert!(
            caution.message.contains("grund read 2 files"),
            "the count is what separates this from the empty scan: {}",
            caution.message
        );
        assert!(
            caution.message.contains("`# <KIND>-<NNN>-<slug>: <title>`")
                && caution.message.contains("`§<KIND>-<NNN>-<slug>`"),
            "§DF-nothing-recognized.2.3: both shapes come from the configured template: {}",
            caution.message
        );
        assert!(
            caution.message.contains("[id] format = \"{kind}-{number}-{slug}\""),
            "the message names the format the headings failed: {}",
            caution.message
        );
    }

    /// §DF-nothing-recognized.2.3: never an example ID assembled from
    /// `[id] number_pattern` / `[id] slug_pattern`, and never a corrected
    /// spelling for a heading the tree actually holds.
    #[test]
    fn the_caution_proposes_no_id() {
        let root = test_root("the_caution_proposes_no_id");
        write(&root.join("grund.toml"), DEFAULT_CONFIG);
        write(&root.join("docs/FS-alpha.md"), "# FS-alpha: The alpha spec\n");

        let message = caution(&check_run(&root, false))
            .expect("caution")
            .message
            .clone();
        assert!(
            !message.contains("FS-001"),
            "an ID built from the patterns would be a guess at what they accept: {message}"
        );
        assert!(
            !message.contains("FS-alpha"),
            "the caution is a fact about the run, not a proposal for a line: {message}"
        );
    }

    /// §FS-check.4.5: the shapes and the marker are the project's own, not the
    /// defaults — a repository that configures neither still gets a message it
    /// can act on.
    #[test]
    fn the_caution_speaks_the_configured_grammar() {
        let root = test_root("the_caution_speaks_the_configured_grammar");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\nproject_name = \"acme\"\n\n[reference]\nmarker = \"@\"\n\n\
             [id]\nformat = \"{kind}-{slug}\"\n\n[[kinds]]\nprefix = \"SPEC\"\nfolder = \"docs\"\n\n\
             [scan]\ninclude = [\"docs\"]\n",
        );
        write(&root.join("docs/note.md"), "# SPEC 1: Not a declaration\n");

        let message = caution(&check_run(&root, false))
            .expect("caution")
            .message
            .clone();
        assert!(
            message.contains("`# <KIND>-<slug>: <title>`") && message.contains("`@<KIND>-<slug>`"),
            "the shape reduces with the template and the citation carries the configured marker: {message}"
        );
        assert!(
            message.contains("with <KIND> one of {SPEC}"),
            "the configured kinds are the other half of the fix: {message}"
        );
    }

    /// §FS-check.4.5: one recognized declaration proves the grammar and the tree
    /// agree, which is the whole question.
    #[test]
    fn a_recognized_declaration_silences_the_caution() {
        let root = test_root("a_recognized_declaration_silences_the_caution");
        write(&root.join("grund.toml"), DEFAULT_CONFIG);
        write(&root.join("docs/FS-alpha.md"), "# FS-alpha: Not a declaration\n");
        write(
            &root.join("docs/FS-001-beta.md"),
            "# FS-001-beta: The beta spec\n\nBody.\n",
        );

        let run = check_run(&root, false);
        assert!(
            caution(&run).is_none(),
            "a tree that declares something is not a tree grund recognized nothing in"
        );
    }

    /// §FS-check.2.2 owns the walk that read no files: the two cautions answer different
    /// questions and a run earns at most one of them.
    #[test]
    fn a_walk_that_read_no_files_keeps_the_empty_scan_caution() {
        let root = test_root("a_walk_that_read_no_files_keeps_the_empty_scan_caution");
        write(&root.join("grund.toml"), DEFAULT_CONFIG);
        std::fs::create_dir_all(root.join("docs")).expect("create an empty docs tree");

        let run = check_run(&root, false);
        assert!(caution(&run).is_none(), "no file was read, so nothing was there to recognize");
        assert!(
            run.report
                .warnings
                .iter()
                .any(|diagnostic| diagnostic.code == "empty-scan"),
            "§FS-check.2.2 is the caution that names the scope"
        );
    }

    /// §FS-check.4.5: the claim is about a whole project, so a run that read part
    /// of one does not make it.
    #[test]
    fn a_narrowed_run_makes_no_claim_about_the_project() {
        let root = test_root("a_narrowed_run_makes_no_claim_about_the_project");
        write(&root.join("grund.toml"), DEFAULT_CONFIG);
        write(
            &root.join("docs/FS-001-alpha.md"),
            "# FS-001-alpha: The alpha spec\n\nBody.\n",
        );
        write(&root.join("notes/scratch.md"), "Nothing grund recognizes.\n");

        let run = check_run(&root.join("notes"), false);
        assert!(
            caution(&run).is_none(),
            "a slice the caller chose, holding nothing, is an answer rather than a misconfiguration"
        );
    }

    /// A workspace root over two members: `specs` declares, `app` only cites it
    /// across the namespace boundary (§FS-workspace.1).
    fn citing_member_workspace(name: &str) -> PathBuf {
        let root = test_root(name);
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\nproject_name = \"root\"\n\n\
             [workspace]\nmembers = [\"specs\", \"app\"]\ninclude_root = false\n",
        );
        write(
            &root.join("specs/grund.toml"),
            "grund_config_version = 1\nproject_name = \"specs\"\n\n[scan]\ninclude = [\"docs\"]\n",
        );
        write(
            &root.join("specs/docs/FS-001-alpha.md"),
            "# FS-001-alpha: The alpha spec\n\nBody.\n",
        );
        write(
            &root.join("app/grund.toml"),
            "grund_config_version = 1\nproject_name = \"app\"\n\n[scan]\ninclude = [\"docs\"]\n",
        );
        root
    }

    /// §FS-check.4.5 asks *recognized*, not *declared*: a member whose whole job
    /// is to point at another project's specs declares nothing and is working
    /// exactly as intended.
    #[test]
    fn a_member_that_only_cites_another_project_earns_no_caution() {
        let root = citing_member_workspace("a_member_that_only_cites_another_project_earns_no_caution");
        write(
            &root.join("app/docs/notes.md"),
            "Built against \u{a7}specs/FS-001-alpha.\n",
        );

        let run = check_run(&root, false);
        assert!(
            run.report.errors.is_empty(),
            "the cross-project citation resolves: {:?}",
            located_diagnostics(&run.config, &run.report.errors)
        );
        assert!(
            caution(&run).is_none(),
            "one citation anywhere in the project answers the question the caution asks"
        );
    }

    /// §FS-check.4.5: per project, like §FS-check.2.2 — the member that
    /// recognized nothing earns the caution, and the member beside it that did
    /// does not.
    #[test]
    fn a_member_that_recognized_nothing_is_judged_alone() {
        let root = citing_member_workspace("a_member_that_recognized_nothing_is_judged_alone");
        write(&root.join("app/docs/notes.md"), "Nothing grund recognizes.\n");

        let run = check_run(&root, false);
        let cautions: Vec<&Diagnostic> = run
            .report
            .warnings
            .iter()
            .filter(|diagnostic| diagnostic.code == "nothing-recognized")
            .collect();
        assert_eq!(
            cautions.len(),
            1,
            "the declaring member is not swept up with the one that recognized nothing: {:?}",
            run.report
                .warnings
                .iter()
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>()
        );
        assert!(
            cautions[0].message.contains("grund read 1 file"),
            "the count is that member's own: {}",
            cautions[0].message
        );
    }
}
