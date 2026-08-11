/// Test module: the grund-open resolver (§FS-integrations)
#[cfg(test)]
mod tests_resolver {
    use super::*;
    // The resolver is a POSIX shell script, so every case that builds a
    // fixture for it is `cfg(unix)`. On Windows only the pure matcher case
    // below is compiled, and it needs no fixtures.
    #[cfg(unix)]
    use super::tests_support::*;

    #[cfg(unix)]
    #[test]
    fn resolver_does_not_evaluate_repository_paths_as_shell_source() {
        use std::os::unix::fs::PermissionsExt;

        let root = test_root("resolver_does_not_evaluate_repository_paths_as_shell_source");
        let bin = root.join("bin");
        std::fs::create_dir_all(&bin).expect("create mock bin");
        // The resolver walks up for the config root before doing anything else
        // (§FS-integrations.3.1), so the fixture has to look like a grund repo.
        write(&root.join(".agents/grund.toml"), "[project]\n");
        let pwned = root.join("pwned");
        let capture = root.join("opened-argument");
        let mock_grund = bin.join("grund");
        let opener = bin.join("opener");
        let resolver = root.join("grund-open");
        write(
            &mock_grund,
            &format!(
                "#!/bin/sh\nprintf '%s\\n' '{{\"id\":\"FS-safe\",\"section\":null,\"body\":\"\",\"path\":\"docs/$(touch {})evil.md\",\"line\":7}}'\n",
                pwned.display()
            ),
        );
        write(&opener, "#!/bin/sh\nprintf '%s\\n' \"$1\" > \"$CAPTURE\"\n");
        write(&resolver, GRUND_OPEN_RESOLVER);
        for path in [&mock_grund, &opener, &resolver] {
            let mut permissions = std::fs::metadata(path).unwrap().permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(path, permissions).unwrap();
        }
        let path = format!(
            "{}:{}",
            bin.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let output = std::process::Command::new(&resolver)
            .arg(format!("{}FS-safe", '\u{a7}'))
            .current_dir(&root)
            .env("PATH", &path)
            .env("GRUND_OPEN_CMD", &opener)
            .env("CAPTURE", &capture)
            .env_remove("EDITOR")
            .output_unbusy();

        assert!(output.status.success(), "resolver failed: {}", String::from_utf8_lossy(&output.stderr));
        assert!(!pwned.exists(), "repository path was evaluated as shell source");
        assert!(std::fs::read_to_string(capture).unwrap().contains("$(touch"));


        let empty_command = std::process::Command::new(&resolver)
            .arg(format!("{}FS-safe", '\u{a7}'))
            .current_dir(&root)
            .env("PATH", &path)
            .env("GRUND_OPEN_CMD", "   ")
            .env_remove("EDITOR")
            .output_unbusy();
        assert_eq!(empty_command.status.code(), Some(2));
        assert!(
            String::from_utf8_lossy(&empty_command.stderr).contains("contains no command")
        );
    }

    // §FS-integrations.3.1: a clicked `§<ID>.<section>` must open the *section's*
    // line, not the declaration heading. The resolver therefore forwards the whole
    // citation to `grund` instead of truncating at the first `.`; truncating would
    // send every click on a subsection to line 1.
    #[cfg(unix)]
    #[test]
    fn resolver_opens_the_cited_section_line() {
        let (opened, argv) = run_resolver(
            "resolver_opens_the_cited_section_line",
            ".",
            &format!("{}FS-target.2.1", '\u{a7}'),
            "{\"id\":\"FS-target\",\"section\":\"2.1\",\"body\":\"\",\"path\":\"docs/target.md\",\"line\":12}",
        );
        assert_eq!(argv, "FS-target.2.1", "section suffix must reach `grund`");
        assert_eq!(opened, "docs/target.md:12");
    }

    // §FS-integrations.3.1: the click may arrive with the shell in a subdirectory.
    // `grund` reports paths relative to the config root (§FS-config.3.6), so the
    // resolver joins against the root it discovered — handing the editor a
    // repo-relative path would open nothing from anywhere but the root.
    #[cfg(unix)]
    #[test]
    fn resolver_opens_absolute_path_from_a_subdirectory() {
        let (opened, _) = run_resolver(
            "resolver_opens_absolute_path_from_a_subdirectory",
            "src/deep",
            &format!("{}FS-target", '\u{a7}'),
            "{\"id\":\"FS-target\",\"section\":null,\"body\":\"\",\"path\":\"docs/target.md\",\"line\":4}",
        );
        assert_eq!(
            opened, "docs/target.md:4",
            "path must be joined against the config root, not the cwd"
        );
    }

    // §FS-integrations.3.1: `[reference] marker` is per-repo while the resolver is
    // user-global, so it strips any leading punctuation rather than a literal `§`.
    // A workspace-qualified `<alias>/<ID>` survives that strip and is forwarded
    // whole, because the alias begins with an alphanumeric.
    #[cfg(unix)]
    #[test]
    fn resolver_strips_any_marker_and_keeps_the_workspace_alias() {
        let (opened, argv) = run_resolver(
            "resolver_strips_any_marker_and_keeps_the_workspace_alias",
            ".",
            "@@app/FS-target.2",
            "{\"id\":\"FS-target\",\"section\":\"2\",\"body\":\"\",\"path\":\"apps/app/docs/target.md\",\"line\":12}",
        );
        assert_eq!(argv, "app/FS-target.2", "alias must survive marker stripping");
        assert_eq!(opened, "apps/app/docs/target.md:12");
    }

    // §FS-integrations.3.1: the *location* an agent prints beside a citation —
    // `path:line[:col]` — opens too. The shapes are mechanically distinct (an
    // ID's section suffix is dotted, never coloned); the printed path is
    // config-root-relative while the click may land in a subdirectory, so the
    // resolver climbs to the nearest ancestor holding the file — consulting no
    // `grund` and no config, which is what lets a location click work in any
    // repository. The fixture has neither, so success proves location mode.
    #[cfg(unix)]
    #[test]
    fn resolver_opens_a_location_token_without_consulting_grund() {
        use std::os::unix::fs::PermissionsExt;

        let root = physical_test_root("resolver_opens_a_location_token_without_consulting_grund");
        let cwd = root.join("src/deep");
        std::fs::create_dir_all(&cwd).expect("create cwd");
        write(&root.join("docs/file.test.md"), "l1\nl2\nl3\n");
        let capture = root.join("opened-argument");
        let opener = root.join("opener");
        let resolver = root.join("grund-open");
        write(&opener, "#!/bin/sh\nprintf '%s\\n' \"$1\" > \"$CAPTURE\"\n");
        write(&resolver, GRUND_OPEN_RESOLVER);
        for path in [&opener, &resolver] {
            let mut permissions = std::fs::metadata(path).unwrap().permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(path, permissions).unwrap();
        }
        // A multi-dot filename stays whole, a column suffix is dropped, and
        // swept-in punctuation is tolerated exactly as for citations.
        for (token, expect) in [
            ("docs/file.test.md:2", "docs/file.test.md:2"),
            ("docs/file.test.md:2:9", "docs/file.test.md:2"),
            ("(docs/file.test.md:3", "docs/file.test.md:3"),
        ] {
            let output = std::process::Command::new(&resolver)
                .arg(token)
                .current_dir(&cwd)
                .env("GRUND_OPEN_CMD", &opener)
                .env("CAPTURE", &capture)
                .env_remove("EDITOR")
                .output_unbusy();
            assert!(
                output.status.success(),
                "{token}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let opened = std::fs::read_to_string(&capture).unwrap();
            assert_eq!(
                opened.trim_end(),
                format!("{}/{expect}", root.display()),
                "token {token}"
            );
        }
        let missing = std::process::Command::new(&resolver)
            .arg("no/such/file.md:3")
            .current_dir(&cwd)
            .env("GRUND_OPEN_CMD", &opener)
            .env_remove("EDITOR")
            .output_unbusy();
        assert_eq!(missing.status.code(), Some(1));
        assert!(
            String::from_utf8_lossy(&missing.stderr).contains("no file 'no/such/file.md'")
        );
    }

    // §FS-integrations.3.1: the location rule must be registered before the
    // citation rule — ordered the other way, the citation matcher's recorded
    // false positive claims an ID-shaped fragment *inside* a `:line`-suffixed
    // path, and the location can never become one link. kitty encodes the same
    // order as alternation inside a single regex.
    #[test]
    fn location_matcher_precedes_citation_matcher() {
        let apply = WEZTERM_SNIPPET
            .split("function grund_apply_hyperlink_rule")
            .nth(1)
            .expect("wezterm apply function");
        let location = apply.find("grund_location_pattern").expect("location rule");
        let citation = apply.find("grund_citation_pattern").expect("citation rule");
        assert!(location < citation, "wezterm must register the location rule first");
        assert!(
            WEZTERM_SNIPPET
                .contains("patterns = { grund_location_pattern, grund_citation_pattern }"),
            "quick-select must offer both shapes, location first"
        );
        // Both kitty gestures: location alternative, then `|`, then the marker
        // run that opens the citation alternative.
        assert_eq!(
            KITTY_SNIPPET
                .matches(":[0-9]+(?::[0-9]+)?|[^\\w\\s]{1,3}")
                .count(),
            2,
            "both kitty hints must match location-first alternation"
        );
    }

    // The `body` field carries arbitrary declaration prose. Field extraction is
    // anchored to the end of the object so prose containing `"path":` or `"line":`
    // cannot be read as the real field (§FS-integrations.3.1).
    #[cfg(unix)]
    #[test]
    fn resolver_ignores_field_shapes_inside_the_body() {
        let (opened, _) = run_resolver(
            "resolver_ignores_field_shapes_inside_the_body",
            ".",
            &format!("{}FS-target", '\u{a7}'),
            "{\"id\":\"FS-target\",\"section\":null,\"body\":\"see \\\"path\\\":\\\"decoy.md\\\",\\\"line\\\":999\",\"path\":\"docs/real.md\",\"line\":8}",
        );
        assert_eq!(opened, "docs/real.md:8", "body prose must not shadow the real fields");
    }

    #[cfg(unix)]
    #[test]
    fn set_executable_surfaces_io_errors() {
        let missing = test_root("set_executable_surfaces_io_errors").join("missing");
        assert!(set_executable(&missing).is_err());
    }
}
