/// Embedding contracts for external facts (§FS-id.1, §FS-id.2.1,
/// §AR-bindings.2).
#[cfg(test)]
mod tests_external_facts {
    use super::tests_support::*;
    use super::*;

    #[test]
    fn public_id_api_honors_width_for_a_per_kind_numeric_format() {
        let root = test_root("public_id_api_honors_width_for_a_per_kind_numeric_format");
        write(
            &root.join("grund.toml"),
            concat!(
                "grund_config_version = 1\n[id]\nformat = \"{kind}-{slug}\"\n",
                "[[kinds]]\nkind = \"TICKET\"\nfile = \"docs/tickets.md\"\n",
                "format = \"{kind}-{number}\"\n",
                "[scan]\ninclude = [\"docs\"]\nrespect_gitignore = false\n"
            ),
        );
        write(
            &root.join("docs/tickets.md"),
            "# Tickets\n\n## TICKET-1234: Existing\n",
        );

        let proposed = propose_id(
            "TICKET",
            "Another ticket",
            IdOpts {
                path: root,
                path_provided: true,
                width: 6,
            },
        )
        .expect("public id api");
        let IdProposalOutcome::Proposed(proposed) = proposed else {
            panic!("expected a proposed ticket ID");
        };
        assert_eq!(proposed.id, "TICKET-001235");
        assert_eq!(proposed.number, Some(1235));
    }
}
