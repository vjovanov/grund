/// Test module: the candidate tiers behind `unknown project alias` (§FS-check.3.8).
///
/// Its own module because these cases fail together for one reason: which
/// project a written alias path is allowed to be read as. The e2e corpus pins the
/// rendered diagnostics (`workspace-nested-alias-hint-worked-examples`, the two
/// narrowed-run cases); these pin the tier rules a message shape cannot show —
/// that the tiers never mix, that the list is sorted and cut at three, and how it
/// is joined.
#[cfg(test)]
mod tests_alias_hints {
    use super::*;

    /// §FS-check.3.8: the dropped-prefix tier — a project whose path *ends with*
    /// what was written. The mistake whole alias paths invite (§FS-workspace.6.1).
    #[test]
    fn dropped_prefix_tier_offers_the_longer_path() {
        let known = ["root", "hardware", "hardware/sprayer"];
        assert_eq!(
            nearest_project_aliases("sprayer", known.into_iter(), false),
            vec!["hardware/sprayer".to_string()]
        );
    }

    /// §FS-check.3.8: the last-segment tier — the written path is the right
    /// length but names the wrong parent, so no suffix match exists and the
    /// project sharing the leaf is offered instead.
    #[test]
    fn last_segment_tier_offers_the_same_leaf_under_another_parent() {
        let known = ["root", "left/api", "left"];
        assert_eq!(
            nearest_project_aliases("wrong/api", known.into_iter(), false),
            vec!["left/api".to_string()]
        );
    }

    /// §FS-check.3.8: the typo tier — no suffix and no shared leaf, so the
    /// near-match rule §3.1 uses decides.
    #[test]
    fn typo_tier_offers_a_project_one_edit_away() {
        let known = ["root", "hardware", "left/api"];
        assert_eq!(
            nearest_project_aliases("hardwar", known.into_iter(), false),
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
            nearest_project_aliases("api", known.into_iter(), false),
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
            nearest_project_aliases("api", known.into_iter(), false),
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
        assert!(nearest_project_aliases("payments/refunds", known.into_iter(), false).is_empty());
    }
}
