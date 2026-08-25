/// Test module: what counts as an **entry** in a kind's index
/// (§DF-index-entry-form.2.3). One predicate decides it — a citation is a bare
/// entry exactly when the next `grund fmt --write` would wrap it — and every
/// case here is a place where the answer is no, so §FS-check.3.17 must stay
/// silent and §FS-check.4.6's warning is what the reader gets instead. Each was
/// a permanent, unclearable error before the predicate replaced a list of
/// exemptions. Where the findings land, and the two carve-outs around them, are
/// in `tests_kind_index.rs`.
#[cfg(test)]
mod tests_kind_index_entry_form {
    use super::tests_support::*;

    /// §FS-check.3.17 / §DF-index-entry-form.2.3: an unmarked token is a
    /// recognized citation off strict mode, and `fmt --cross-refs` still leaves
    /// it alone — without `--marker` a bare token stays bare (§FS-fmt.6.5). So it
    /// is not an entry `fmt` can repair, and the error that names `grund fmt
    /// --write` must not fire on it.
    #[test]
    fn an_unmarked_token_is_not_an_entry_and_is_not_an_error() {
        let root = kind_index_repo_loose("an_unmarked_token_is_not_an_entry_and_is_not_an_error");
        write(
            &root.join("docs/specs/README.md"),
            "# Specs\n\n- FS-001-login — a user logs in\n",
        );

        let run = check_run(&root, false);
        assert!(
            !codes(&run).contains(&"unlinked-index-entry".to_string()),
            "`grund fmt --write` rewrites nothing here, so `check` must not name it: {:?}",
            findings(&run)
        );
        assert!(
            codes(&run).contains(&"missing-index-entry".to_string()),
            "there is still no entry, and that finding's fix is a human edit: {:?}",
            findings(&run)
        );
    }

    /// §FS-check.3.17: a bare ID-shaped token inside a Markdown link destination
    /// is a citation off strict mode (that it is scanned at all is grund#131),
    /// and `](…)` is a zone `fmt` never writes in (§FS-fmt.2.3). Reporting it
    /// would be an error neither `fmt --write` nor `fmt --write --marker` could
    /// ever clear.
    #[test]
    fn an_id_shaped_link_destination_is_not_an_entry() {
        let root = kind_index_repo_loose("an_id_shaped_link_destination_is_not_an_entry");
        write(
            &root.join("docs/specs/README.md"),
            "# Specs\n\nSee the [login spec](FS-001-login.md) for details.\n",
        );

        let run = check_run(&root, false);
        assert!(
            !codes(&run).contains(&"unlinked-index-entry".to_string()),
            "a link destination is a never-rewrite zone: {:?}",
            findings(&run)
        );
        assert!(
            codes(&run).contains(&"missing-index-entry".to_string()),
            "the ID has no entry the rule recognizes: {:?}",
            findings(&run)
        );
    }

    /// §DF-index-entry-form.2.3: the predicate withholds the *error*; it never
    /// withholds credit for a link a reader can already follow. A hand-written
    /// wrap around an unmarked token is one, and `fmt` leaves it alone.
    #[test]
    fn a_hand_written_link_around_an_unmarked_token_is_an_entry() {
        let root = kind_index_repo_loose("a_hand_written_link_around_an_unmarked_token_is_an_entry");
        write(
            &root.join("docs/specs/README.md"),
            "# Specs\n\n- [FS-001-login](FS-001-login.md#fs-001-login-a-user-logs-in)\n",
        );

        let run = check_run(&root, false);
        assert!(
            !codes(&run).contains(&"unlinked-index-entry".to_string())
                && !codes(&run).contains(&"missing-index-entry".to_string()),
            "the entry is the link, whatever `fmt` would have written: {:?}",
            findings(&run)
        );
    }

    /// §FS-fmt.6.4: `fmt` passes a declaration heading through untouched, so a
    /// citation riding on one is no more repairable than one in an inline-code
    /// span.
    #[test]
    fn a_citation_on_a_declaration_heading_is_not_an_entry() {
        let root = kind_index_repo("a_citation_on_a_declaration_heading_is_not_an_entry");
        write(
            &root.join("docs/specs/README.md"),
            "# FS-002-index: The index of §FS-001-login\n\nProse.\n",
        );

        let run = check_run(&root, false);
        assert!(
            !codes(&run).contains(&"unlinked-index-entry".to_string()),
            "the heading is a line `fmt` refuses to rewrite: {:?}",
            findings(&run)
        );
        assert!(
            only(&run, "missing-index-entry")
                .message
                .contains("FS-001-login"),
            "the ID is still unlisted: {:?}",
            findings(&run)
        );
    }

    /// §DF-index-entry-form.2.3: a citation inside an inline-code span is neither
    /// an entry nor a finding — `fmt` never wraps one, so demanding it would leave
    /// the repository permanently red.
    #[test]
    fn an_inline_code_mention_is_not_an_entry_and_is_not_an_error() {
        let root = kind_index_repo("an_inline_code_mention_is_not_an_entry_and_is_not_an_error");
        write(
            &root.join("docs/specs/README.md"),
            "# Specs\n\nThe login flow is `§FS-001-login`, explained below.\n",
        );

        let run = check_run(&root, false);
        assert!(
            !codes(&run).contains(&"unlinked-index-entry".to_string()),
            "`fmt` declines to wrap it, so `check` must not demand it: {:?}",
            findings(&run)
        );
        assert!(
            codes(&run).contains(&"missing-index-entry".to_string()),
            "the ID still has no entry, and that is the finding with a fix"
        );
    }
}
