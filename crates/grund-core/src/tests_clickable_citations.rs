/// Test module: the clickable-citations guidance section (§FS-integrations)
#[cfg(test)]
mod tests_clickable_citations {
    use super::*;
    use super::tests_support::*;

    // §FS-init.2.3.6: without the `conversation` opinion, repositories get only
    // the fixed web-surface convention.
    #[test]
    fn clickable_citations_section_is_fixed_without_opinion() {
        let root = test_root("clickable_citations_section_is_fixed_without_opinion");
        write(&root.join(".agents/grund.toml"), "grund_config_version = 1\n");
        let config = load_config(&root).expect("load config");
        let rendered = clickable_citations_section(&config, ConversationSurface::Plain);
        assert_eq!(
            rendered,
            "### Clickable citations\n\nOn repository web surfaces, link `§<ID>` to the PR branch in PR bodies, the reviewed commit in reviews, an exact commit for permalinks, and the default branch otherwise; fall back to plain when unsure."
        );
    }

    // §FS-init.2.3.4.17, §DF-conversation-link-target: the committed `link`
    // opinion adds the config-derived local-conversation sentence, in the form
    // the entrypoint's own agent is verified to render.
    #[test]
    fn clickable_citations_section_renders_conversation_opinion() {
        let root = test_root("clickable_citations_section_renders_conversation_opinion");
        write(
            &root.join(".agents/grund.toml"),
            "grund_config_version = 1\n[reference]\nconversation = \"link\"\n",
        );
        let config = load_config(&root).expect("load config");
        let deference = "If a user-level grund block states a local-conversation rendering, follow that instead: that machine knows what its surface can open.";

        // The gated fallback (§DF-conversation-link-target.2.4): the location
        // travels as plain text where no click-test says more.
        let plain = clickable_citations_section(&config, ConversationSurface::Plain);
        assert!(plain.starts_with("### Clickable citations\n\nOn repository web surfaces,"));
        assert!(plain.contains(
            "In local conversations, follow `§<ID>` with its declaration location as plain `path:line` text; fall back to the bare citation when unsure."
        ));
        assert!(plain.contains(deference));
        // No trailing newline, so the template's placeholder keeps init idempotent.
        assert!(!plain.ends_with('\n'));

        // The Claude entrypoints: a Markdown link over the machine-independent
        // `file` target, the citation itself as the visible text.
        let linked = clickable_citations_section(&config, ConversationSurface::Linked);
        assert!(linked.contains(
            "In local conversations, render `§<ID>` as a Markdown link whose visible text is the citation itself and whose target is `file://<absolute path>#L<line>` for its declaration; fall back to the bare citation when unsure."
        ));
        assert!(linked.contains(deference));
        assert!(!linked.ends_with('\n'));
    }

    // §FS-init.2.3.6: the wording is fixed, the marker is the repository's. A
    // hardcoded `§` in a repo configured with another marker would teach agents
    // a token that repo does not treat as a citation — `grund check` ignores it
    // under strict, so the grounded claim is silently never verified.
    #[test]
    fn clickable_citations_section_renders_the_configured_marker() {
        let root = test_root("clickable_citations_section_renders_the_configured_marker");
        write(
            &root.join(".agents/grund.toml"),
            "grund_config_version = 1\n[reference]\nmarker = \"@\"\nconversation = \"link\"\n",
        );
        let config = load_config(&root).expect("load config");
        let rendered = clickable_citations_section(&config, ConversationSurface::Plain);
        // Both sentences: the always-present web rule and the config-derived one.
        assert!(rendered.contains("On repository web surfaces, link `@<ID>` to the PR branch"));
        assert!(rendered.contains("In local conversations, follow `@<ID>` with its declaration"));
        // The linked form renders the marker too — it is the citation's own.
        let linked = clickable_citations_section(&config, ConversationSurface::Linked);
        assert!(linked.contains("In local conversations, render `@<ID>` as a Markdown link"));
        assert!(
            !linked.contains('\u{a7}'),
            "no hardcoded § may survive in a custom-marker repo: {linked}"
        );
        assert!(
            !rendered.contains('\u{a7}'),
            "no hardcoded § may survive in a custom-marker repo: {rendered}"
        );
    }

    // §FS-config.3.1: the closed enum admits only `link` — `plain` is machine
    // state and stays user-scoped (§DF-repo-conversation-opinion.2.2).
    #[test]
    fn repository_config_rejects_non_link_conversation() {
        let root = test_root("repository_config_rejects_non_link_conversation");
        write(
            &root.join(".agents/grund.toml"),
            "grund_config_version = 1\n[reference]\nconversation = \"plain\"\n",
        );
        let error = match load_config(&root) {
            Ok(_) => panic!("conversation = \"plain\" must be rejected"),
            Err(error) => error,
        };
        assert!(error
            .to_string()
            .contains("unknown [reference] conversation `plain` (expected link)"));
    }

    // §FS-config.3.1: `[render.links]` is the retired spelling of the user-level
    // preference; the two scopes now share the `[reference] conversation` name.
    // It must stay an unknown section rather than come back as a silent alias.
    #[test]
    fn repository_config_rejects_the_retired_render_links_section() {
        let root = test_root("repository_config_rejects_the_retired_render_links_section");
        write(
            &root.join(".agents/grund.toml"),
            "grund_config_version = 1\n[render.links]\nconversation = \"plain\"\n",
        );
        let error = match load_config(&root) {
            Ok(_) => panic!("repo render.links must be rejected"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("unknown config section"));
    }
}
