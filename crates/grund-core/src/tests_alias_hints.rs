/// Test module: the candidate tiers behind `unknown project alias` (§FS-check.3.8).
///
/// Its own module because these cases fail together for one reason: which
/// project a written alias path is allowed to be read as. The e2e corpus pins the
/// rendered diagnostics (`workspace-nested-alias-hint-worked-examples`, the
/// narrowed-run cases); these pin the rules a message shape cannot show — that
/// the tiers never mix, that a narrowed run reaches none of them, that the list
/// is sorted and cut at three, and how it is joined.
#[cfg(test)]
mod tests_alias_hints {
    use super::*;

    /// §FS-check.3.8: the first tier is a proper prefix of slash-separated
    /// segments. Exact paths and byte prefixes without a segment boundary do
    /// not join it, nor does a path that merely ends with the written segments.
    #[test]
    fn proper_prefix_tier_is_segment_aware() {
        assert_eq!(
            nearest_project_aliases(
                "group",
                ["group", "group/alpha", "grouped/alpha", "other/group"].into_iter()
            ),
            vec!["group/alpha".to_string()]
        );
        assert_eq!(
            nearest_project_aliases(
                "group/alpha",
                [
                    "group/alpha",
                    "group/alpha/beta",
                    "grouped/alpha/beta",
                    "other/group/alpha",
                ]
                .into_iter()
            ),
            vec!["group/alpha/beta".to_string()]
        );
    }

    /// §FS-check.3.8: a proper-prefix winner suppresses every lower tier.
    #[test]
    fn proper_prefix_tier_outranks_suffix_leaf_and_typo_candidates() {
        let known = ["x/group", "wrong/group", "grouq", "group/alpha"];
        assert_eq!(
            nearest_project_aliases("group", known.into_iter()),
            vec!["group/alpha".to_string()]
        );
    }

    /// §FS-check.3.8 / §REQ-deterministic-output: deeper aliases are byte-sorted
    /// before the same three-candidate cap as every other tier.
    #[test]
    fn proper_prefix_candidates_are_sorted_and_truncated_to_three() {
        let known = ["group/zeta", "group/beta", "group/alpha", "group/mid"];
        assert_eq!(
            nearest_project_aliases("group", known.into_iter()),
            vec![
                "group/alpha".to_string(),
                "group/beta".to_string(),
                "group/mid".to_string()
            ]
        );
    }

    /// §FS-errors.3: the proper-prefix tier uses the frozen one-, two-, and
    /// three-candidate message forms without changing the base error.
    #[test]
    fn proper_prefix_messages_use_the_exact_candidate_phrasing() {
        assert_eq!(
            unknown_project_message("group", ["group/alpha"].into_iter(), ""),
            "unknown project alias group; did you mean group/alpha?"
        );
        assert_eq!(
            unknown_project_message(
                "group",
                ["group/beta", "group/alpha"].into_iter(),
                ""
            ),
            "unknown project alias group; did you mean group/alpha or group/beta?"
        );
        assert_eq!(
            unknown_project_message(
                "group",
                ["group/gamma", "group/alpha", "group/beta"].into_iter(),
                ""
            ),
            "unknown project alias group; did you mean group/alpha, group/beta or group/gamma?"
        );
    }

    /// §FS-check.3.8: narrowed runs still suppress the new tier. When `--full`
    /// finds the same error outside `include`, its scope clause remains first.
    #[test]
    fn proper_prefix_hint_preserves_scope_decorations() {
        let known = ["group/alpha"];
        assert_eq!(
            unknown_project_message("group", known.into_iter(), "left"),
            "unknown project alias group; only the left subtree is in scope here — check from the workspace root for a path outside it"
        );
        let diagnostic = Diagnostic {
            code: "unknown-project",
            path: None,
            line: None,
            column: None,
            message: unknown_project_message("group", known.into_iter(), ""),
            sites: Vec::new(),
        };
        let diagnostic = tag_out_of_scope(diagnostic);
        assert_eq!(diagnostic.code, "out-of-scope-unknown-project");
        assert_eq!(
            diagnostic.message,
            "outside [scan] include: unknown project alias group; did you mean group/alpha?"
        );
    }

    /// §FS-check.3.8: the dropped-prefix tier — a project whose path *ends with*
    /// what was written. The mistake whole alias paths invite (§FS-workspace.6.1).
    #[test]
    fn dropped_prefix_tier_offers_the_longer_path() {
        let known = ["root", "hardware", "hardware/sprayer"];
        assert_eq!(
            nearest_project_aliases("sprayer", known.into_iter()),
            vec!["hardware/sprayer".to_string()]
        );
    }

    /// §FS-check.3.8: the dropped-prefix tier is a *tier*, not a filter that the
    /// last-segment tier would have applied anyway. A written path that is a
    /// proper suffix of one project and shares a last segment with another offers
    /// the suffix match **alone** — deleting the tier hands the reader two
    /// candidates, one of them a different project.
    #[test]
    fn dropped_prefix_tier_outranks_a_same_leaf_candidate() {
        let known = ["mid/inner/leaf", "other/leaf", "root"];
        assert_eq!(
            nearest_project_aliases("inner/leaf", known.into_iter()),
            vec!["mid/inner/leaf".to_string()],
            "§FS-check.3.8: `other/leaf` shares the leaf but is not what the author dropped a prefix from"
        );
    }

    /// §FS-check.3.8: a narrowed run offers **no candidate at all**, not even the
    /// dropped-prefix tier. This is the case that shipped wrong: `left` holds a
    /// nested `api`, the citation names the top-level `api` the run cannot see,
    /// and the hint re-pointed a citation the workspace-root run accepts at a
    /// different declaration — green before, green after.
    #[test]
    fn a_narrowed_run_offers_no_candidate_even_for_a_dropped_prefix() {
        assert_eq!(
            unknown_project_message("api", ["left", "left/api"].into_iter(), "left"),
            "unknown project alias api; only the left subtree is in scope here — check from the workspace root for a path outside it"
        );
        assert_eq!(
            unknown_project_message("lef", ["left", "left/api"].into_iter(), "left"),
            "unknown project alias lef; only the left subtree is in scope here — check from the workspace root for a path outside it",
            "§FS-check.3.8: the typo tier is off too — one rule for the whole narrowed run"
        );
    }

    /// §FS-check.3.8: the outermost root is where the tiers live, and the scope
    /// sentence is not printed there — a path with no candidate reports bare.
    #[test]
    fn the_outermost_root_still_hints_and_never_names_a_scope() {
        assert_eq!(
            unknown_project_message("sprayer", ["hardware/sprayer", "root"].into_iter(), ""),
            "unknown project alias sprayer; did you mean hardware/sprayer?"
        );
        assert_eq!(
            unknown_project_message("payments/refunds", ["hardware/sprayer"].into_iter(), ""),
            "unknown project alias payments/refunds"
        );
    }

    /// §FS-check.3.8: the last-segment tier — the written path is the right
    /// length but names the wrong parent, so no suffix match exists and the
    /// project sharing the leaf is offered instead.
    #[test]
    fn last_segment_tier_offers_the_same_leaf_under_another_parent() {
        let known = ["root", "left/api", "left"];
        assert_eq!(
            nearest_project_aliases("wrong/api", known.into_iter()),
            vec!["left/api".to_string()]
        );
    }

    /// §FS-check.3.8: the typo tier — no suffix and no shared leaf, so the
    /// near-match rule §3.1 uses decides.
    #[test]
    fn typo_tier_offers_a_project_one_edit_away() {
        let known = ["root", "hardware", "left/api"];
        assert_eq!(
            nearest_project_aliases("hardwar", known.into_iter()),
            vec!["hardware".to_string()]
        );
    }

    /// §FS-check.3.8: "tiers do not mix". A dropped-prefix match is near-certain,
    /// so a typo-distance candidate present in the same tree is withheld rather
    /// than listed beside it — the good hint stays the whole hint.
    #[test]
    fn a_suffix_match_suppresses_the_lower_tiers() {
        let known = ["left/api", "apj", "root"];
        assert_eq!(
            nearest_project_aliases("api", known.into_iter()),
            vec!["left/api".to_string()],
            "§FS-check.3.8: the suffix tier fires alone, `apj` is one edit away"
        );
    }

    /// §FS-check.3.8: candidates are sorted and cut at three — `grund list` is
    /// the catalogue, a finding is not. The fourth match is dropped by sort
    /// order, not by discovery order, so the diagnostic is deterministic.
    #[test]
    fn candidates_are_sorted_and_truncated_to_three() {
        let known = ["zeta/api", "beta/api", "alpha/api", "mid/api"];
        assert_eq!(
            nearest_project_aliases("api", known.into_iter()),
            vec![
                "alpha/api".to_string(),
                "beta/api".to_string(),
                "mid/api".to_string()
            ]
        );
    }

    /// §FS-check.3.8: the candidate list reads as prose — `a`, `a or b`,
    /// `a, b or c` — which is what makes `did you mean …?` a sentence.
    #[test]
    fn alternatives_join_as_prose() {
        let of = |items: &[&str]| {
            join_alternatives(&items.iter().map(|s| s.to_string()).collect::<Vec<_>>())
        };

        assert_eq!(of(&["left/api"]), "left/api");
        assert_eq!(of(&["left/api", "right/api"]), "left/api or right/api");
        assert_eq!(
            of(&["a/api", "b/api", "c/api"]),
            "a/api, b/api or c/api"
        );
    }

    /// §FS-check.3.8: a path with nothing to offer reports on its own — an empty
    /// candidate list, never a bare `did you mean ?`.
    #[test]
    fn an_unrelated_path_offers_nothing() {
        let known = ["root", "hardware/sprayer"];
        assert!(nearest_project_aliases("payments/refunds", known.into_iter()).is_empty());
    }
}
