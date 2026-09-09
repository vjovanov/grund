/// Negative embedded-value boundaries: wrapper normalization, host-language
/// enrollment, and overlapping root authority (§FS-values.2.4, §FS-values.5.1,
/// §FS-values.9).
#[cfg(test)]
mod tests_embedded_value_boundaries {
    use super::tests_support::{
        embedded_value_config, scan_embedded_value, test_root, write,
    };
    use super::*;

    #[test]
    fn block_close_bytes_are_wrapper_syntax_only_in_block_comments() {
        let (config, findings) = scan_embedded_value(
            "embedded_markdown_block_close_bytes",
            "# FS-001-alpha: Alpha\n\n\
             ## 1. Inert <!-- grund:value -->*/\n\
             ### 1.1. ignored\n\
             ## 2. Price <!-- grund:value -->\n\
             ### 2.1. 1200*/\n\
             ## 3. Use\n\
             Bound: `1200` (§FS-001-alpha.2.1)\n",
        );
        let declaration = findings.declarations.values().next().unwrap().first().unwrap();
        assert!(
            declaration.sections["1"].value_root.is_none(),
            "a Markdown marker followed by authored bytes is inert"
        );
        assert_eq!(
            declaration.sections["2.1"].value.as_ref().unwrap().decoded,
            "1200*/",
            "Markdown component bytes are never source-wrapper syntax"
        );
        let value_codes = check_findings(&findings, &config)
            .errors
            .into_iter()
            .filter(|error| {
                matches!(
                    error.code,
                    "invalid-value-declaration" | "invalid-value-binding" | "value-mismatch"
                )
            })
            .map(|error| error.code)
            .collect::<Vec<_>>();
        assert_eq!(value_codes, vec!["value-mismatch"]);

        let root = test_root("embedded_line_comment_block_close_bytes");
        let path = root.join("src/value.rs");
        write(
            &path,
            "/// FS-001-alpha: Alpha\n\
             /// ## 1. Price <!-- grund:value -->\n\
             /// ### 1.1. 1200*/\n\
             /// ## 2. Use\n\
             /// Bound: `1200` (§FS-001-alpha.1.1)\n",
        );
        let config = embedded_value_config(root);
        let (findings, errors) = scan_tree(&config, Some(&path), true).expect("scan line comment");
        assert!(errors.is_empty());
        let declaration = findings.declarations.values().next().unwrap().first().unwrap();
        assert_eq!(
            declaration.sections["1.1"].value.as_ref().unwrap().decoded,
            "1200*/",
            "line-comment content retains authored block-close bytes"
        );
        assert!(
            check_findings(&findings, &config)
                .errors
                .iter()
                .any(|error| error.code == "value-mismatch")
        );

        let root = test_root("embedded_block_comment_close_is_wrapper");
        let path = root.join("src/value.rs");
        let id = "FS-001-alpha";
        write(
            &path,
            &format!(
                "/**\n\
             * {id}: Alpha\n\
             * ## 1. Price <!-- grund:value -->\n\
             * ### 1.1. 1200 */\n"
            ),
        );
        let config = embedded_value_config(root);
        let (findings, errors) = scan_tree(&config, Some(&path), true).expect("scan block comment");
        assert!(errors.is_empty());
        let declaration = findings.declarations.values().next().unwrap().first().unwrap();
        assert_eq!(
            declaration.sections["1.1"].value.as_ref().unwrap().decoded,
            "1200",
            "a recognized block-comment close is wrapper syntax"
        );
        assert!(findings.invalid_value_declarations.is_empty());
    }

    #[test]
    fn non_comment_source_markers_are_inert() {
        let root = test_root("embedded_host_strings_are_inert");
        let path = root.join("src/value.rs");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n\
             [[kinds]]\nkind = \"FS\"\nfolder = \"src\"\nindex = false\n\n\
             [scan]\ninclude = [\"src\"]\nextensions = [\"rs\"]\ncomment_prefixes = [\"//\"]\n",
        );
        write(
            &path,
            "/// FS-001-alpha: Alpha\n\
             const RAW: &str = r#\"\n\
             ## 1. Raw <!-- grund:value -->\n\
             ### 1.1. 1200\n\
             \"#;\n\
             const ORDINARY: &str = \"\n\
             ## 2. Ordinary <!-- grund:value -->\n\
             ### 2.1. 50\n\
             \";\n",
        );
        let mut config = embedded_value_config(root);
        config.comment_prefixes = vec!["//".to_string()];
        config.rebuild_grammar().expect("rebuild Rust grammar");
        let (findings, errors) = scan_tree(&config, Some(&path), true).expect("scan host strings");
        assert!(errors.is_empty());
        let declaration = findings.declarations.values().next().unwrap().first().unwrap();
        assert!(
            declaration
                .sections
                .values()
                .all(|section| section.value_root.is_none()),
            "host-language strings must not add value-root catalog metadata"
        );
        assert!(
            findings.invalid_value_declarations.is_empty(),
            "host-language strings must not produce value diagnostics: {:?}",
            findings
                .invalid_value_declarations
                .iter()
                .map(|site| (site.line, site.message.as_str()))
                .collect::<Vec<_>>()
        );
        let catalog = list(ListOpts {
            path: config.root.clone(),
            path_provided: true,
            ..ListOpts::default()
        })
        .expect("list host-string fixture");
        assert_eq!(catalog.entries.len(), 1);
        assert!(catalog.entries[0].value_roots.is_empty());
    }

    #[test]
    fn reversed_and_duplicate_root_claims_invalidate_catalog_authority() {
        let (config, findings) = scan_embedded_value(
            "embedded_reversed_root_overlap",
            "# FS-001-alpha: Alpha\n\n\
             ### 1.1. Inner <!-- grund:value -->\n\
             #### 1.1.1. 1200\n\
             ## 1. Outer <!-- grund:value -->\n\
             ## 2. Use\n\
             Bound: `999` (§FS-001-alpha.1.1.1)\n",
        );
        let declaration = findings.declarations.values().next().unwrap().first().unwrap();
        assert!(
            ["1", "1.1"].into_iter().all(|path| !declaration.sections[path]
                .value_root
                .as_ref()
                .unwrap()
                .valid),
            "both overlapping catalog roots are invalid independent of source order"
        );
        let report = check_findings(&findings, &config);
        assert!(report.errors.iter().any(|error| {
            error.code == "invalid-value-declaration" && error.line == Some(3)
        }));
        assert!(report.errors.iter().all(|error| error.code != "value-mismatch"));
        write(
            &config.root.join("grund.toml"),
            "grund_config_version = 1\n\n\
             [[kinds]]\nkind = \"FS\"\nfolder = \"docs\"\nindex = false\n\n\
             [scan]\ninclude = [\"docs\"]\n",
        );
        let catalog = list(ListOpts {
            path: config.root.clone(),
            path_provided: true,
            ..ListOpts::default()
        })
        .expect("list reversed roots");
        assert_eq!(
            catalog.entries[0]
                .value_roots
                .iter()
                .map(|root| (root.id.as_str(), root.valid))
                .collect::<Vec<_>>(),
            vec![("FS-001-alpha.1", false), ("FS-001-alpha.1.1", false)]
        );

        let (config, findings) = scan_embedded_value(
            "embedded_duplicate_root_claim",
            "# FS-001-alpha: Alpha\n\n\
             ## 1. Price <!-- grund:value -->\n\
             ### 1.1. 1200\n\
             ## 1. Duplicate <!-- grund:value -->\n\
             ## 2. Use\n\
             Bound: `999` (§FS-001-alpha.1.1)\n",
        );
        let declaration = findings.declarations.values().next().unwrap().first().unwrap();
        assert!(
            !declaration.sections["1"].value_root.as_ref().unwrap().valid,
            "a duplicate marked path invalidates its primary catalog authority"
        );
        let report = check_findings(&findings, &config);
        assert!(report.errors.iter().any(|error| {
            error.code == "invalid-value-declaration" && error.line == Some(5)
        }));
        assert!(report.errors.iter().any(|error| error.code == "duplicate-section"));
        assert!(report.errors.iter().all(|error| error.code != "value-mismatch"));
        write(
            &config.root.join("grund.toml"),
            "grund_config_version = 1\n\n\
             [[kinds]]\nkind = \"FS\"\nfolder = \"docs\"\nindex = false\n\n\
             [scan]\ninclude = [\"docs\"]\n",
        );
        let catalog = list(ListOpts {
            path: config.root.clone(),
            path_provided: true,
            ..ListOpts::default()
        })
        .expect("list duplicate root");
        assert_eq!(catalog.entries[0].value_roots.len(), 1);
        assert!(!catalog.entries[0].value_roots[0].valid);
    }
}
