/// Test module: an index canonically enrolls an external inline declaration
/// without a stub (§FS-check.4.6, §DF-index-entry-form.2.7). These cases pin the
/// discriminator and the exact-site unused accounting; the CLI test pins the
/// `show` / `list` surface.
#[cfg(test)]
mod tests_kind_index_enrollment {
    use super::tests_support::*;
    use super::*;

    fn external_index_repo(name: &str, index: &str) -> PathBuf {
        let root = test_root(name);
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n\
             [[kinds]]\nprefix = \"AR\"\nfolder = \"docs/architecture\"\n\n\
             [scan]\ninclude = [\"docs\", \"src\"]\n",
        );
        write(
            &root.join("src/bus.rs"),
            "/// AR-001-bus: The in-process event bus\n\
             ///\n\
             /// Broadcasts in registration order.\n\
             ///\n\
             /// ## 1. Ordering\n\
             ///\n\
             /// Subscribers are called in order.\n\
             fn bus() {}\n",
        );
        write(&root.join("docs/architecture/README.md"), index);
        root
    }

    fn bus_id() -> Id {
        Id {
            kind: "AR".to_string(),
            num: Some(1),
            slug: Some("bus".to_string()),
        }
    }

    fn scanned_index(root: &Path) -> (Config, Findings, KindIndexEntries) {
        let config = resolve_workspace_config(root).expect("load config");
        let findings = scan_findings(&config, root);
        let entries = KindIndexEntries::new(&findings, &config);
        (config, findings, entries)
    }

    /// §FS-check.4.6: the canonical bare-ID source link is membership and entry
    /// at once. With no other citation, §FS-check.4.1 still reports the source
    /// declaration unused — proof that the link is navigation rather than use.
    #[test]
    fn a_canonical_source_link_enrolls_without_a_stub() {
        let root = external_index_repo(
            "a_canonical_source_link_enrolls_without_a_stub",
            "# Architecture\n\n- [§AR-001-bus](../../src/bus.rs)\n",
        );

        let run = check_run(&root, false);
        assert!(
            !codes(&run).contains(&"missing-index-entry".to_string())
                && !codes(&run).contains(&"unlinked-index-entry".to_string()),
            "the canonical link is both enrollment and entry: {:?}",
            findings(&run)
        );
        assert!(
            codes(&run).contains(&"unused".to_string()),
            "the enrollment site is navigation, not use: {:?}",
            findings(&run)
        );

        let (config, findings, entries) = scanned_index(&root);
        let owed = entries
            .entries_in(&root.join("docs/architecture/README.md"))
            .expect("configured index obligations");
        assert!(owed.contains(&bus_id()), "the index owns the external ID");
        let citation = findings.citations.first().expect("index citation");
        assert!(entries.is_index_entry(citation), "the exact link is the entry");

        let shown = show_declaration(
            &config,
            &config,
            &findings,
            &bus_id(),
            None,
            ShowRenderMode::Default,
            false,
        )
        .expect("show external declaration");
        assert_eq!(
            canonical_test_path(&shown.path),
            canonical_test_path(&root.join("src/bus.rs")),
            "enrollment creates no synthetic home"
        );
    }

    /// §DF-index-not-an-inbound-citation.2.2: only the canonical bare-ID link
    /// enrolls. A section citation on the next line remains ordinary use even
    /// though a source home's canonical link target is the same bare file path.
    #[test]
    fn a_second_same_id_reference_on_the_page_still_counts_as_use() {
        let root = external_index_repo(
            "a_second_same_id_reference_on_the_page_still_counts_as_use",
            "# Architecture\n\n\
             - [§AR-001-bus](../../src/bus.rs)\n\n\
             Ordering is explained by [§AR-001-bus.1](../../src/bus.rs).\n",
        );

        let run = check_run(&root, false);
        assert!(
            !codes(&run).contains(&"unused".to_string()),
            "the section citation is real use: {:?}",
            findings(&run)
        );

        let (_, findings, entries) = scanned_index(&root);
        assert_eq!(findings.citations.len(), 2);
        assert!(entries.is_index_entry(&findings.citations[0]));
        assert!(
            !entries.is_index_entry(&findings.citations[1]),
            "membership is tracked by exact site, not by ID"
        );
    }

    /// §DF-index-entry-form.2.7: a full same-kind link with a custom destination
    /// is an ordinary reference. Shape alone cannot enroll an external home.
    #[test]
    fn a_noncanonical_link_is_an_ordinary_reference() {
        let root = external_index_repo(
            "a_noncanonical_link_is_an_ordinary_reference",
            "# Architecture\n\nSee [§AR-001-bus](../../src/not-the-bus.rs).\n",
        );

        let run = check_run(&root, false);
        assert!(
            !codes(&run).contains(&"unused".to_string())
                && !codes(&run).contains(&"missing-index-entry".to_string()),
            "the link is use, not a membership claim: {:?}",
            findings(&run)
        );
        let (_, findings, entries) = scanned_index(&root);
        assert!(
            entries
                .entries_in(&root.join("docs/architecture/README.md"))
                .is_none(),
            "an empty folder and a noncanonical link create no obligation"
        );
        assert!(!entries.is_index_entry(&findings.citations[0]));
    }

    /// §FS-check.4.6: the exception is for inline source declarations only. A
    /// canonical link to a Markdown declaration outside the configured folder is
    /// still ordinary prose and does not redefine the kind home.
    #[test]
    fn an_external_markdown_declaration_is_not_enrolled() {
        let root = test_root("an_external_markdown_declaration_is_not_enrolled");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n\
             [[kinds]]\nprefix = \"AR\"\nfolder = \"docs/architecture\"\n\n\
             [scan]\ninclude = [\"docs\", \"notes\"]\n",
        );
        write(
            &root.join("notes/AR-001-bus.md"),
            "# AR-001-bus: The bus\n\nBody.\n",
        );
        write(
            &root.join("docs/architecture/README.md"),
            "# Architecture\n\nSee [§AR-001-bus](../../notes/AR-001-bus.md#ar-001-bus-the-bus).\n",
        );

        let (_, findings, entries) = scanned_index(&root);
        assert!(
            entries
                .entries_in(&root.join("docs/architecture/README.md"))
                .is_none()
        );
        assert!(!entries.is_index_entry(&findings.citations[0]));
    }

    /// §FS-fmt.6.1 / §DF-index-entry-form.2.7: under the default cross-reference
    /// mode an author may write the bare marked ID and let `fmt` persist the
    /// canonical form that carries enrollment meaning.
    #[test]
    fn fmt_can_write_the_canonical_enrollment_form() {
        let root = external_index_repo(
            "fmt_can_write_the_canonical_enrollment_form",
            "# Architecture\n\n- §AR-001-bus\n",
        );

        format_references(FmtOpts {
            path: root.clone(),
            path_provided: true,
            write: true,
            ..FmtOpts::default()
        })
        .expect("fmt");

        let index = fs::read_to_string(root.join("docs/architecture/README.md"))
            .expect("read formatted index");
        assert!(
            index.contains("[§AR-001-bus](../../src/bus.rs)"),
            "fmt writes the enrollment discriminator: {index}"
        );
        assert!(
            codes(&check_run(&root, false)).contains(&"unused".to_string()),
            "the newly canonical site is now navigation"
        );
    }

    /// §FS-fmt.6.1: the entries-only carve-out cannot infer intent from a bare
    /// external citation. With global link generation disabled it stays ordinary
    /// prose until the author explicitly writes or requests the canonical link.
    #[test]
    fn entries_only_fmt_does_not_turn_external_prose_into_membership() {
        let root = external_index_repo(
            "entries_only_fmt_does_not_turn_external_prose_into_membership",
            "# Architecture\n\nSee §AR-001-bus.\n",
        );
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n\
             [[kinds]]\nprefix = \"AR\"\nfolder = \"docs/architecture\"\n\n\
             [scan]\ninclude = [\"docs\", \"src\"]\n\n\
             [fmt.cross_refs]\nenabled = false\n",
        );

        format_references(FmtOpts {
            path: root.clone(),
            path_provided: true,
            write: true,
            ..FmtOpts::default()
        })
        .expect("fmt");

        let index = fs::read_to_string(root.join("docs/architecture/README.md"))
            .expect("read formatted index");
        assert!(index.contains("See §AR-001-bus."), "ordinary prose stays bare: {index}");
        assert!(
            !codes(&check_run(&root, false)).contains(&"unused".to_string()),
            "the ordinary citation still counts as use"
        );
    }
}
