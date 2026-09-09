/// Focused embedded-section value proofs: exact markers, every supported
/// source wrapper, strict physical shape, binding ownership, and formatter
/// stability (§FS-values.2.4, §FS-values.5.1, §FS-values.8).
#[cfg(test)]
mod tests_embedded_values {
    use super::tests_support::{
        embedded_value_config as embedded_config, scan_embedded_value as scan_embedded, test_root,
        write,
    };
    use super::*;

    fn invalid_lines(name: &str, body: &str, named: bool) -> Vec<usize> {
        let root = test_root(name);
        let path = root.join("docs/value.md");
        write(
            &path,
            &format!(
                "# FS-001-alpha: Alpha\n\n## 1. Price <!-- grund:value -->\n{body}"
            ),
        );
        let mut config = embedded_config(root);
        config.named_sections = named;
        config.rebuild_grammar().expect("rebuild named grammar");
        let (findings, errors) = scan_tree(&config, Some(&path), true).expect("scan invalid shape");
        assert!(errors.is_empty(), "fixture should be readable: {errors:?}");
        findings
            .invalid_value_declarations
            .iter()
            .map(|site| site.line)
            .collect()
    }

    #[test]
    fn exact_marker_probe_is_total_for_utf8_and_rejects_lookalikes() {
        assert_eq!(exact_embedded_value_marker("`1200` §FS-001-alpha.1"), None);
        assert_eq!(exact_embedded_value_marker("## 1. Café"), None);
        assert_eq!(
            exact_embedded_value_marker("## 1. Café <!-- grund:value -->"),
            Some("## 1. Café ".len())
        );
        for lookalike in [
            "## 1. Price<!-- grund:value -->",
            "## 1. Price  <!-- grund:value -->",
            "## 1. Price <!-- GRUND:value -->",
            "## 1. Price <!-- grund:value --",
            "## 1. Price <!-- grund:value -->*/",
        ] {
            assert_eq!(
                exact_embedded_value_marker(lookalike),
                None,
                "lookalike must stay inert: {lookalike}"
            );
        }
    }

    #[test]
    fn exact_marker_requires_a_citable_numeric_heading() {
        let (_, findings) = scan_embedded(
            "embedded_marker_heading_boundary",
            "# FS-001-alpha: Alpha\n\n## Notes <!-- grund:value -->\n\
             ## 1. Price <!-- Grund:value -->\n§FS-001-alpha\n",
        );
        assert_eq!(
            findings
                .invalid_value_declarations
                .iter()
                .map(|site| site.line)
                .collect::<Vec<_>>(),
            vec![3]
        );
    }

    #[test]
    fn every_source_wrapper_and_python_docstring_compares_values() {
        let line_forms = [
            ("slash", "rs", "//"),
            ("rustdoc", "rs", "///"),
            ("rust-module-doc", "rs", "//!"),
            ("hash", "py", "#"),
            ("semicolon", "clj", ";"),
            ("double-dash", "sql", "--"),
        ];
        for (name, extension, prefix) in line_forms {
            let root = test_root(&format!("embedded_wrapper_{name}"));
            let path = root.join(format!("src/value.{extension}"));
            write(
                &path,
                &format!(
                    "{prefix} FS-001-alpha: Alpha\n\
                     {prefix} ## 1. Price <!-- grund:value -->\n\
                     {prefix} ### 1.1. 1200\n\
                     {prefix} ## 2. Use\n\
                     {prefix} Bound: `999` (§FS-001-alpha.1.1)\n"
                ),
            );
            assert_single_mismatch(&root, &path, name);
        }

        for (name, extension) in [("javadoc", "java"), ("jsdoc", "js")] {
            let root = test_root(&format!("embedded_wrapper_{name}"));
            let path = root.join(format!("src/value.{extension}"));
            let id = "FS-001-alpha";
            write(
                &path,
                &format!(
                    "/**\n\
                 * {id}: Alpha\n\
                 * ## 1. Price <!-- grund:value -->\n\
                 * ### 1.1. 1200\n\
                 * ## 2. Use\n\
                 * Bound: `999` (§{id}.1.1)\n\
                 */\n"
                ),
            );
            assert_single_mismatch(&root, &path, name);
        }

        for (name, quote) in [("double-docstring", "\"\"\""), ("single-docstring", "'''")] {
            let root = test_root(&format!("embedded_wrapper_{name}"));
            let path = root.join("src/value.py");
            write(
                &path,
                &format!(
                    "{quote}FS-001-alpha: Alpha\n\
                     ## 1. Price <!-- grund:value -->\n\
                     ### 1.1. 1200\n\
                     ## 2. Use\n\
                     Bound: `999` (§FS-001-alpha.1.1)\n\
                     {quote}\n"
                ),
            );
            assert_single_mismatch(&root, &path, name);
        }
    }

    fn assert_single_mismatch(root: &Path, path: &Path, name: &str) {
        let config = embedded_config(root.to_path_buf());
        let (findings, errors) = scan_tree(&config, Some(path), true).expect("scan source form");
        assert!(errors.is_empty(), "{name}: readable source form");
        let report = check_findings(&findings, &config);
        let value_codes = report
            .errors
            .iter()
            .filter(|error| {
                matches!(
                    error.code,
                    "invalid-value-declaration" | "invalid-value-binding" | "value-mismatch"
                )
            })
            .map(|error| error.code)
            .collect::<Vec<_>>();
        assert_eq!(value_codes, vec!["value-mismatch"], "{name}: compare outcome");
    }

    #[test]
    fn bad_component_text_does_not_cascade_to_following_coordinates() {
        assert_eq!(
            invalid_lines(
                "embedded_invalid_text_no_cascade",
                "### 1.1. `bad`\n### 1.2. good\n",
                false,
            ),
            vec![4]
        );
    }

    #[test]
    fn every_out_of_order_physical_position_is_located() {
        assert_eq!(
            invalid_lines(
                "embedded_out_of_order_both_sites",
                "### 1.2. second\n### 1.1. first\n",
                false,
            ),
            vec![4, 5]
        );
    }

    #[test]
    fn strict_shape_branches_report_the_offending_lines() {
        let cases = [
            ("zero", "", false, vec![3]),
            (
                "gap",
                "### 1.1. first\n### 1.3. third\n",
                false,
                vec![5],
            ),
            ("named", "### field: USD\n", true, vec![4, 3]),
            (
                "grandchild",
                "### 1.1. first\n#### 1.1.1. nested\n",
                false,
                vec![5],
            ),
            ("root-prose", "Prose is forbidden.\n### 1.1. first\n", false, vec![4]),
            ("component-body", "### 1.1. first\nBody is forbidden.\n", false, vec![5]),
            ("empty-title", "### 1.1.\n", false, vec![4]),
            ("wrong-depth", "#### 1.1. first\n", false, vec![4]),
        ];
        for (name, body, named, expected) in cases {
            assert_eq!(
                invalid_lines(&format!("embedded_shape_{name}"), body, named),
                expected,
                "strict branch {name}"
            );
        }

        let duplicate = invalid_lines(
            "embedded_shape_duplicate",
            "### 1.1. first\n### 1.1. duplicate\n",
            false,
        );
        assert!(duplicate.iter().all(|line| *line == 5));
        assert!(duplicate.len() >= 2, "duplicate owns coordinate and duplicate findings");
    }

    #[test]
    fn longest_invalid_root_suppresses_its_component_binding_only() {
        let (config, findings) = scan_embedded(
            "embedded_nested_binding_ownership",
            "# FS-001-alpha: Alpha\n\n\
             ## 1. Outer <!-- grund:value -->\n\
             ### 1.1. Inner <!-- grund:value -->\n\
             #### 1.1.1. 1200\n\
             ## 2. Use\n\
             `1200` (§FS-001-alpha.1.1.1)\n\
             `1200` (§FS-001-alpha.1.1)\n\
             `1200` (§FS-001-alpha.1.1.1.1)\n",
        );
        let report = check_findings(&findings, &config);
        let binding_lines = report
            .errors
            .iter()
            .filter(|error| error.code == "invalid-value-binding")
            .map(|error| error.line.unwrap())
            .collect::<Vec<_>>();
        assert_eq!(binding_lines, vec![8, 9]);
    }

    fn write_local_format_fixture(name: &str) -> (PathBuf, PathBuf) {
        let root = test_root(name);
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n\
             [[kinds]]\nkind = \"FS\"\nfolder = \"docs\"\nindex = false\n\n\
             [scan]\ninclude = [\"docs\"]\n",
        );
        let path = root.join("docs/pricing.md");
        write(
            &path,
            "# FS-001-pricing: Pricing\n\n\
             ## 1. Group\n\
             ### 1.1. Price <!-- grund:value -->\n\
             #### 1.1.1. 1200\n\
             ## 2. Use\n\
             Bound: `999` (§FS-001-pricing.1.1.1)\n",
        );
        (root, path)
    }

    #[test]
    fn formatter_preserves_local_embedded_binding_across_two_writes() {
        let (root, path) = write_local_format_fixture("embedded_fmt_local");
        assert_formatter_and_comparison_stable(&root, &path);
    }

    #[test]
    fn formatter_preserves_qualified_embedded_binding_across_two_writes() {
        let root = test_root("embedded_fmt_qualified");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\nproject_name = \"root\"\n\n\
             [workspace]\nmembers = [\"pricing\", \"shop\"]\ninclude_root = false\n",
        );
        for member in ["pricing", "shop"] {
            write(
                &root.join(member).join("grund.toml"),
                &format!(
                    "grund_config_version = 1\nproject_name = \"{member}\"\n\n\
                     [[kinds]]\nkind = \"FS\"\nfolder = \"docs\"\nindex = false\n\n\
                     [scan]\ninclude = [\"docs\"]\n"
                ),
            );
        }
        write(
            &root.join("pricing/docs/pricing.md"),
            "# FS-001-pricing: Pricing\n\n\
             ## 1. Price <!-- grund:value -->\n\
             ### 1.1. 1200\n",
        );
        let path = root.join("shop/docs/offer.md");
        write(
            &path,
            "# FS-002-offer: Offer\n\nBound: `999` (§pricing/FS-001-pricing.1.1)\n",
        );
        assert_formatter_and_comparison_stable(&root, &path);
    }

    fn assert_formatter_and_comparison_stable(root: &Path, path: &Path) {
        let before = fs::read_to_string(path).expect("read authored binding");
        for _ in 0..2 {
            let output = format_references(FmtOpts {
                path: root.to_path_buf(),
                path_provided: true,
                write: true,
                cross_refs: true,
                ..FmtOpts::default()
            })
            .expect("format embedded binding");
            assert!(output.changes.is_empty(), "binding must not be wrapped");
            assert_eq!(fs::read_to_string(path).unwrap(), before);
        }
        let report = check(root).expect("check formatted tree");
        assert!(
            report.errors.iter().any(|error| error.code == "value-mismatch"),
            "the second formatter pass must leave comparison active"
        );
    }
}
