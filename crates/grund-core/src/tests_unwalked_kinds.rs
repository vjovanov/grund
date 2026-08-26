/// Test module: a place that is listed but not walked (§FS-config.3.4.7) — the
/// `scan` key, what an unwalked kind keeps and loses, and the three
/// combinations the config refuses.
#[cfg(test)]
mod tests_unwalked_kinds {
    use super::tests_support::*;
    use super::*;

    /// A repo whose `templates/` is an unwalked non-citable home beside an `FS`
    /// home, under `require_grounding`. The home holds one citation-free
    /// Markdown file and one with a dangling citation, so "not walked" and
    /// "walked and clean" are told apart by what the run reports.
    fn templates_repo(name: &str, extra: &str) -> PathBuf {
        let root = test_root(name);
        write(
            &root.join("grund.toml"),
            &format!(
                "grund_config_version = 1\n\n\
                 [reference]\nrequire_grounding = true\n\n\
                 [[kinds]]\nkind = \"FS\"\nfolder = \"docs/specs\"\nindex = false\n\n\
                 [[kinds]]\nkind = \"template\"\nfolder = \"templates\"\ncitable = false\n\
                 scan = false\ntitle = \"Scaffold templates\"\n\n\
                 [scan]\ninclude = [\"docs\"]\n\n{extra}"
            ),
        );
        write(
            &root.join("docs/specs/FS-001-login.md"),
            "# FS-001-login: A user logs in\n\nBody.\n",
        );
        write(
            &root.join("templates/README.md"),
            "# Scaffold\n\nNo citation here, on purpose.\n",
        );
        write(
            &root.join("templates/agents.md"),
            "# Agents\n\nSee §FS-999-ghost.\n",
        );
        root
    }

    /// §FS-config.3.4.7: nothing in the home is read, so neither the grounding
    /// rule of §FS-check.3.6 nor the dangling citation is a finding.
    #[test]
    fn an_unwalked_home_is_neither_grounded_nor_checked() {
        let run = check_run(
            &templates_repo("an_unwalked_home_is_neither_grounded_nor_checked", ""),
            false,
        );
        let codes = codes(&run);
        assert!(
            !codes
                .iter()
                .any(|code| code.contains("ungrounded") || code.contains("dangling")),
            "listed, not walked: {codes:?}"
        );
    }

    /// §FS-config.3.4.7 / §FS-check.1.3: `--full` walks the whole root and
    /// reports resolution failures only — the dangling citation surfaces, the
    /// citation-free file earns no grounding finding.
    #[test]
    fn full_reaches_an_unwalked_home_as_out_of_scope_territory() {
        let run = check_run(
            &templates_repo("full_reaches_an_unwalked_home_as_out_of_scope_territory", ""),
            true,
        );
        let codes = codes(&run);
        assert!(
            codes.iter().any(|code| code.contains("dangling")),
            "the wider walk still reads it: {codes:?}"
        );
        assert!(
            !codes.iter().any(|code| code.contains("ungrounded")),
            "no convention it did not adopt: {codes:?}"
        );
    }

    /// §FS-init.2.3.4.4 / §FS-init.2.3.5: the row is the point of configuring
    /// the kind; a directions bullet would promise a check nothing performs.
    #[test]
    fn the_generated_block_lists_an_unwalked_kind_without_a_directions_bullet() {
        let root = templates_repo(
            "the_generated_block_lists_an_unwalked_kind_without_a_directions_bullet",
            "[citations]\n[citations.FS]\nshould = [\"FS\"]\n",
        );
        let config = load_config(&root).expect("load config");
        let block = render_agents_append_block("demo", &config, &root, true, ConversationSurface::Plain);
        assert!(
            block.contains("- [templates/](templates): Scaffold templates"),
            "map row by place: {block}"
        );
        assert!(
            !block.contains("**templates/**"),
            "no directions bullet: {block}"
        );
    }

    /// §FS-config.3.4.7: a citable kind is always walked — unwalked, its
    /// declarations would be invisible rather than declared.
    #[test]
    fn scan_false_is_refused_on_a_citable_kind() {
        let root = test_root("scan_false_is_refused_on_a_citable_kind");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n\
             [[kinds]]\nkind = \"FS\"\nfolder = \"docs/specs\"\nindex = false\nscan = false\n",
        );
        assert!(
            config_error(&root).contains("sets `scan = false` and declares IDs"),
            "a kind that declares IDs is always walked"
        );
    }

    /// §FS-config.3.4.7: the homeless kind has no home to leave unwalked.
    #[test]
    fn scan_false_is_refused_without_a_home() {
        let root = test_root("scan_false_is_refused_without_a_home");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n\
             [[kinds]]\nkind = \"FS\"\nfolder = \"docs/specs\"\nindex = false\n\n\
             [[kinds]]\nkind = \"misc\"\ncitable = false\nscan = false\n",
        );
        assert!(
            config_error(&root).contains("sets `scan = false` without a home"),
            "what of the complement is walked is `[scan] include`'s to say"
        );
    }

    /// §FS-config.3.4.7: a rule on a kind none of whose files is scanned could
    /// never fire, so it is refused where it makes the promise.
    #[test]
    fn a_citation_rule_on_an_unwalked_kind_is_refused() {
        let root = templates_repo(
            "a_citation_rule_on_an_unwalked_kind_is_refused",
            "[citations]\n[citations.template]\nmust = [\"FS\"]\n",
        );
        assert!(
            config_error(&root).contains("names an unwalked kind `template`"),
            "the rule would pass vacuously"
        );
    }

    fn config_error(root: &Path) -> String {
        match load_config(root) {
            Ok(_) => panic!("expected the config to be rejected"),
            Err(error) => format!("{error:#}"),
        }
    }
}
