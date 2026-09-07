/// Focused first-class-value scanner cases: Markdown title boundaries and the
/// exact attempted-binding delimiter and source-context contracts
/// (§FS-values.2.1, §FS-values.3.1, §FS-values.3.2).
#[cfg(test)]
mod tests_values {
    use super::*;
    use crate::tests_support::{check_run, codes, test_root, write};

    #[test]
    fn markdown_component_excludes_the_complete_numeric_coordinate_delimiter() {
        let config = Config::default_for(PathBuf::from("."));

        assert_eq!(markdown_component("## 1. 1200", &config), Some(("1200", 7)));
        assert_eq!(markdown_component("## 1 USD", &config), Some(("USD", 6)));
        assert_eq!(
            markdown_component("### 1.2. low / base / high", &config),
            Some(("low / base / high", 10))
        );
    }

    fn value_repo(name: &str, binding: &str) -> PathBuf {
        let root = test_root(name);
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n\
             [reference]\nstrict = true\n\n\
             [id]\nformat = \"{kind}-{slug}\"\nslug_pattern = \"[a-z][a-z0-9-]*\"\n\n\
             [[kinds]]\nkind = \"CONST\"\nfolder = \"values\"\nindex = false\nvalues = true\n\n\
             [scan]\ninclude = [\"docs\"]\nextensions = [\"md\"]\n",
        );
        write(
            &root.join("values/field-price.md"),
            "# CONST-field-price: Reference field price\n## 1. 1200\n",
        );
        write(&root.join("docs/offer.md"), binding);
        root
    }

    fn source_value_repo(name: &str, source: &str) -> PathBuf {
        let root = test_root(name);
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n\
             [reference]\nstrict = true\n\n\
             [id]\nformat = \"{kind}-{slug}\"\nslug_pattern = \"[a-z][a-z0-9-]*\"\n\n\
             [[kinds]]\nkind = \"CONST\"\nfolder = \"values\"\nindex = false\nvalues = true\n\n\
             [scan]\ninclude = [\"src\"]\nextensions = [\"md\", \"rs\"]\n",
        );
        write(
            &root.join("values/field-price.md"),
            "# CONST-field-price: Reference field price\n## 1. 1.2e3\n",
        );
        write(&root.join("src/lib.rs"), source);
        root
    }

    #[test]
    fn source_bindings_follow_complete_comment_spans() {
        let dereference = source_value_repo(
            "value_binding_rust_dereference_string",
            "pub fn replace(ptr: &mut &'static str) {\n\
             *ptr = \"`999` (§CONST-field-price.1)\";\n\
             }\n",
        );
        let dereference_errors = check_run(&dereference, false).report.errors;
        let dereference_messages = dereference_errors
            .iter()
            .map(|error| format!("{}: {}", error.code, error.message))
            .collect::<Vec<_>>();
        assert!(
            dereference_errors.is_empty(),
            "binding-shaped text in a dereference expression's host string is unchecked: {dereference_messages:?}"
        );

        let block = source_value_repo(
            "value_binding_rust_block_interior",
            "pub fn documented() {\n\
             /*\n\
             Checked documentation: `999` (§CONST-field-price.1)\n\
             */ let trailing = \"`998` (§CONST-field-price.1)\";\n\
             let _ = trailing;\n\
             }\n",
        );
        let mismatches = check_run(&block, false)
            .report
            .errors
            .into_iter()
            .filter(|error| error.code == "value-mismatch")
            .map(|error| error.line)
            .collect::<Vec<_>>();
        assert_eq!(
            mismatches,
            vec![Some(3)],
            "the block interior binds, while host bytes after `*/` do not"
        );
    }

    #[test]
    fn unterminated_and_multiline_value_bindings_are_located_errors() {
        for (name, binding, line) in [
            (
                "value_binding_unterminated",
                "`1200 (§CONST-field-price.1)\n",
                1,
            ),
            (
                "value_binding_multiline",
                "`1200\n` (§CONST-field-price.1)\n",
                2,
            ),
        ] {
            let run = check_run(&value_repo(name, binding), false);
            assert!(
                run.report.errors.iter().any(|error| {
                    error.code == "invalid-value-binding" && error.line == Some(line)
                }),
                "{name} should report a located invalid binding"
            );
        }
    }

    #[test]
    fn malformed_delimiters_are_errors_but_unbackticked_adjacency_is_prose() {
        for (name, binding) in [
            ("value_binding_missing_space", "`1200`(§CONST-field-price.1)\n"),
            ("value_binding_missing_parens", "`1200` §CONST-field-price.1\n"),
        ] {
            assert_eq!(
                codes(&check_run(&value_repo(name, binding), false)),
                vec!["invalid-value-binding"],
                "{name} should be an invalid attempted binding"
            );
        }

        assert!(
            check_run(
                &value_repo(
                    "value_binding_unbackticked_prose",
                    "1200 (§CONST-field-price.1)\n",
                ),
                false,
            )
            .report
            .errors
            .is_empty(),
            "unbackticked adjacency remains ordinary prose"
        );
    }
}
