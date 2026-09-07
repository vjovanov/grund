/// Focused first-class-value scanner cases: Markdown title boundaries and the
/// exact attempted-binding delimiter contract (§FS-values.2.1, §FS-values.3.1).
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
