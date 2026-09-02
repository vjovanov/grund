/// Test module: the grounding pair as *config* (§FS-config.3.4.8) — which
/// combinations of `require_grounding` and `grounding_level` load, which are
/// rejected and at whose line, and what `grund config show` prints back
/// (§FS-config.4.2). The behaviour those keys buy is pinned next door, in
/// `tests_grounding_per_place.rs`.
#[cfg(test)]
mod tests_grounding_config {
    use super::tests_support::*;
    use super::*;

    /// The message a config this repo cannot load fails with. `Config` carries no
    /// `Debug`, so the error is unwrapped by matching rather than by `expect_err`.
    fn config_error(name: &str, body: &str) -> String {
        let root = test_root(name);
        write(&root.join("grund.toml"), body);
        match load_config(&root) {
            Ok(_) => panic!("expected the config to be rejected"),
            Err(error) => format!("{error:#}"),
        }
    }

    /// §FS-config.3.4.8: no file in an unwalked home is read, so the rule could
    /// never fire — rejected at the key's own line.
    #[test]
    fn grounding_on_an_unwalked_row_is_rejected() {
        let error = config_error(
            "grounding_on_an_unwalked_row_is_rejected",
            "grund_config_version = 1\n\n\
             [[kinds]]\nkind = \"FS\"\nfolder = \"docs\"\n\n\
             [[kinds]]\nkind = \"template\"\nfolder = \"templates\"\ncitable = false\n\
             scan = false\nrequire_grounding = true\n",
        );
        assert!(
            error.ends_with(
                "grund.toml:12: kind `template` sets `require_grounding = true` and `scan = false` (no file in an unwalked home is read, so the rule could never fire)"
            ),
            "{error}"
        );
    }

    /// §FS-config.3.4.8: a *citable* single-file kind is one declaration
    /// document, which §FS-check.3.6.1 leaves alone, so neither key has anything
    /// to mean on it — the rejection the non-citable row below no longer earns.
    #[test]
    fn a_grounding_key_on_a_citable_file_row_is_rejected() {
        let error = config_error(
            "a_grounding_key_on_a_citable_file_row_is_rejected",
            "grund_config_version = 1\n\n\
             [[kinds]]\nkind = \"FS\"\nfolder = \"docs\"\n\n\
             [[kinds]]\nkind = \"GOAL\"\nfile = \"docs/goals.md\"\nrequire_grounding = true\n",
        );
        assert!(
            error.ends_with(
                "grund.toml:10: kind `GOAL` sets `require_grounding` with `file` on a citable kind (a citable single-file kind is one declaration document, which the grounding rule leaves alone — a non-citable one takes both keys)"
            ),
            "{error}"
        );
    }

    /// §FS-config.3.4.8: there is no seventh Markdown heading level for a level
    /// of `7` to name.
    #[test]
    fn a_level_outside_the_heading_range_is_rejected() {
        let error = config_error(
            "a_level_outside_the_heading_range_is_rejected",
            "grund_config_version = 1\n\n\
             [reference]\nrequire_grounding = true\ngrounding_level = 7\n\n\
             [[kinds]]\nkind = \"FS\"\nfolder = \"docs\"\n",
        );
        assert!(
            error.ends_with(
                "grund.toml:5: `grounding_level` must be a Markdown heading level 1..6 (`7` is not)"
            ),
            "{error}"
        );
    }

    /// §FS-config.3.4.8: a unit for a rule the same row just switched off.
    #[test]
    fn a_level_beside_an_explicit_row_false_is_rejected() {
        let error = config_error(
            "a_level_beside_an_explicit_row_false_is_rejected",
            "grund_config_version = 1\n\n\
             [reference]\nrequire_grounding = true\n\n\
             [[kinds]]\nkind = \"FS\"\nfolder = \"docs\"\n\n\
             [[kinds]]\nkind = \"skill\"\nfolder = \"skills\"\ncitable = false\n\
             require_grounding = false\ngrounding_level = 2\n",
        );
        assert!(
            error.ends_with(
                "grund.toml:15: kind `skill` sets `grounding_level` and `require_grounding = false` (the level could never fire)"
            ),
            "{error}"
        );
    }

    /// §FS-config.3.4.8: the same rule one scope up — a `[reference]` level with
    /// nothing turning grounding on anywhere.
    #[test]
    fn a_global_level_with_grounding_off_is_rejected() {
        let error = config_error(
            "a_global_level_with_grounding_off_is_rejected",
            "grund_config_version = 1\n\n\
             [reference]\ngrounding_level = 2\n\n\
             [[kinds]]\nkind = \"FS\"\nfolder = \"docs\"\n",
        );
        assert!(
            error.ends_with(
                "grund.toml:4: [reference] `grounding_level` is set and nothing turns grounding on (set `require_grounding` here or on a [[kinds]] row)"
            ),
            "{error}"
        );
    }

    /// §FS-config.3.4.8: a level is dead config wherever its row's *effective*
    /// `require_grounding` is off, not only where the row wrote `false` — the
    /// row spelling of the `[reference]` rejection above.
    #[test]
    fn a_row_level_under_an_inherited_false_is_rejected() {
        let error = config_error(
            "a_row_level_under_an_inherited_false_is_rejected",
            "grund_config_version = 1\n\n\
             [[kinds]]\nkind = \"FS\"\nfolder = \"docs\"\n\n\
             [[kinds]]\nkind = \"skill\"\nfolder = \"skills\"\ncitable = false\n\
             grounding_level = 2\n",
        );
        assert!(
            error.ends_with(
                "grund.toml:11: kind `skill` sets `grounding_level` and nothing turns grounding on for it (set `require_grounding = true` here or in [reference])"
            ),
            "{error}"
        );
    }

    /// §FS-config.3.4.8 / §FS-config.4.2: a non-citable `file` home is governed
    /// like any other place (§FS-check.3.6.1), so both keys load on its row —
    /// and print back, since the row's effective values differ from the global.
    #[test]
    fn a_non_citable_file_row_takes_both_keys() {
        let root = test_root("a_non_citable_file_row_takes_both_keys");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n\
             [reference]\nrequire_grounding = true\n\n\
             [[kinds]]\nkind = \"FS\"\nfolder = \"docs\"\n\n\
             [[kinds]]\nkind = \"runbook\"\nfile = \"RUNBOOK.md\"\ncitable = false\n\
             grounding_level = 2\n\n\
             [[kinds]]\nkind = \"notes\"\nfile = \"NOTES.md\"\ncitable = false\n\
             require_grounding = false\n",
        );
        let config = load_config(&root).expect("a non-citable file row loads");
        let lines = |name: &str| {
            config.kind_grounding_toml_lines(
                config.kinds.iter().find(|kind| kind.kind == name).unwrap(),
            )
        };
        assert_eq!(lines("runbook"), vec!["grounding_level = 2".to_string()]);
        assert_eq!(lines("notes"), vec!["require_grounding = false".to_string()]);
        assert_eq!(lines("FS"), Vec::<String>::new(), "inherits both");
    }
}
