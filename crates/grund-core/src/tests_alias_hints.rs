/// Test module: the candidate tiers behind `unknown project alias` (§FS-check.3.8).
///
/// Its own module because these cases fail together for one reason: which
/// project a written alias path is allowed to be read as. The e2e corpus pins the
/// rendered diagnostics (`workspace-nested-alias-hint-worked-examples`, the
/// narrowed-run cases); these pin the rules a message shape cannot show — that
/// the tiers never mix, when a narrowed run may reach them, that the list is
/// sorted and cut at three, and how it is joined.
#[cfg(test)]
mod tests_alias_hints {
    use super::*;

    const SCOPE_CLARIFICATION_SUFFIX: &str =
        " — here, the {scope} subtree means the {scope} project and its descendants; this wording changes in grund 0.14.0";

    fn legacy_scope_only_message(namespace: &str, scope: &str) -> String {
        format!(
            "unknown project alias {namespace}; only the {scope} subtree is in scope here — check from the workspace root for a path outside it"
        )
    }

    fn migrating_scope_only_message(namespace: &str, scope: &str) -> String {
        let legacy = legacy_scope_only_message(namespace, scope);
        format!(
            "{legacy}{}",
            SCOPE_CLARIFICATION_SUFFIX.replace("{scope}", scope)
        )
    }

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

    /// §FS-errors.3: 0.13.2 preserves the complete narrowed scope-only
    /// diagnostic as a contiguous prefix and appends the exact migration suffix.
    #[test]
    fn narrowed_scope_only_message_has_the_0132_compatibility_form() {
        let actual = unknown_project_message("alpha", ["group/alpha"].into_iter(), "group");
        let legacy = legacy_scope_only_message("alpha", "group");
        assert_eq!(
            actual.get(..legacy.len()),
            Some(legacy.as_str()),
            "the complete legacy diagnostic must remain a contiguous prefix"
        );
        assert_eq!(
            actual.get(legacy.len()..),
            Some(
                SCOPE_CLARIFICATION_SUFFIX
                    .replace("{scope}", "group")
                    .as_str()
            ),
            "the compatibility suffix must be byte-exact"
        );
        assert_eq!(actual, migrating_scope_only_message("alpha", "group"));
    }

    /// §FS-check.3.8: narrowed runs still suppress the new tier. When `--full`
    /// finds the same error outside `include`, its scope clause remains first.
    #[test]
    fn proper_prefix_hint_preserves_scope_decorations() {
        let known = ["group/alpha"];
        assert_eq!(
            unknown_project_message("group", known.into_iter(), "left"),
            migrating_scope_only_message("group", "left")
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

    /// §FS-check.3.8.1: only a strict, segment-wise extension is eligible in a
    /// narrowed run. Shorter, equal, outside-prefix, and lexical-prefix paths all
    /// keep the same scope-only diagnostic.
    #[test]
    fn a_narrowed_alias_run_rejects_paths_that_are_not_strict_segment_extensions() {
        let cases = [
            ("group", "group/alpha"),
            ("group/alpha", "group/alpha"),
            ("outside/alpha", "group"),
            ("grouped/alpha", "group"),
        ];
        for (namespace, scope) in cases {
            assert_eq!(
                unknown_project_message(
                    namespace,
                    ["group", "group/alpha", "group/alpha/beta"].into_iter(),
                    scope,
                ),
                migrating_scope_only_message(namespace, scope),
                "§FS-check.3.8.1: {namespace:?} must not be treated as inside {scope:?}"
            );
        }
    }

    /// §FS-check.3.8.1: the admission check is segment-wise at every depth. A
    /// typo below a two-segment scope may use the aliases loaded below it.
    #[test]
    fn a_narrowed_alias_run_hints_for_a_strict_multi_segment_extension() {
        assert_eq!(
            unknown_project_message(
                "group/alpha/bet",
                ["group/alpha", "group/alpha/beta"].into_iter(),
                "group/alpha",
            ),
            "unknown project alias group/alpha/bet; did you mean group/alpha/beta?"
        );
    }

    /// §FS-check.3.8.1: admission permits a candidate search; it does not invent
    /// a candidate. An eligible path with no match uses the established bare form.
    #[test]
    fn a_narrowed_alias_eligible_path_without_a_candidate_uses_the_bare_message() {
        assert_eq!(
            unknown_project_message(
                "group/payments/refunds",
                ["group", "group/hardware/sprayer"].into_iter(),
                "group",
            ),
            "unknown project alias group/payments/refunds"
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
