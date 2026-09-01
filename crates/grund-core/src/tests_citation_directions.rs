/// Test module: citation direction rules and managed-block drift (§FS-config.3.9)
#[cfg(test)]
mod tests_citation_directions {
    use super::*;
    use super::tests_support::*;

    // §AR-scanner.2.4: a Markdown declaration's body runs until the next
    // same-or-higher heading; an enclosed citation is classified by the
    // declaration's kind.
    #[test]
    fn scanner_markdown_body_and_source_kind() {
        let root = test_root("scanner_markdown_body_and_source_kind");
        write(
            &root.join("docs/goals.md"),
            "# Goals\n\n## GOAL-001-first: First\n\nGrounds in §GRUND-001-why.\n\n### 1. Detail\n\nMore.\n\n## GOAL-002-second: Second\n\nNothing cited.\n",
        );
        // Classification runs only under `[citations]` (§AR-benchmarks), which
        // is what these tests exercise.
        let mut config = Config::default_for(root.clone());
        config.citations.declared = true;
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan");

        let first = Id {
            kind: "GOAL".into(),
            num: Some(1),
            slug: Some("first".into()),
        };
        let decls = findings.declarations.get(&first).expect("GOAL-first");
        let decl = &decls[0];
        // Body runs from the `## GOAL-first` line up to (not including) the next
        // H2 `## GOAL-second`.
        assert_eq!(decl.body_start, 3);
        assert_eq!(decl.body_end, 10);

        let cite = findings
            .citations
            .iter()
            .find(|c| c.id.slug.as_deref() == Some("why"))
            .expect("GRUND-why citation");
        assert_eq!(cite.source_kind, "GOAL");
        assert_eq!(cite.enclosing_declaration.as_ref(), Some(&first));
    }

    // §AR-scanner.2.4: a citation in a source file outside any inline
    // declaration falls through to the reserved `code` pseudo-kind; one inside
    // an inline declaration's comment block takes that declaration's kind.
    #[test]
    fn scanner_code_source_kind_and_inline_block() {
        let root = test_root("scanner_code_source_kind_and_inline_block");
        write(
            &root.join("src/app.rs"),
            "/// AR-001-router: Router\n/// Implements §FS-001-cli.\n\nfn main() {\n    // see §FS-002-check\n}\n",
        );
        let mut config = Config::default_for(root.clone());
        config.citations.declared = true;
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan");

        let inline = findings
            .citations
            .iter()
            .find(|c| c.id.slug.as_deref() == Some("cli"))
            .expect("FS-cli citation");
        assert_eq!(inline.source_kind, "AR");
        assert_eq!(inline.enclosing_declaration.as_ref().map(|id| id.kind.as_str()), Some("AR"));

        let loose = findings
            .citations
            .iter()
            .find(|c| c.id.slug.as_deref() == Some("check"))
            .expect("FS-check citation");
        assert_eq!(loose.source_kind, "code");
        assert!(loose.enclosing_declaration.is_none());
    }

    // §AR-scanner.2.4 step 2: a citation in a Markdown file under a kind home
    // but outside any declaration body takes the file's home kind.
    #[test]
    fn scanner_file_home_source_kind() {
        let root = test_root("scanner_file_home_source_kind");
        write(
            &root.join("docs/architecture/README.md"),
            "Overview prose citing §FS-001-cli before any declaration.\n",
        );
        let mut config = Config::default_for(root.clone());
        config.citations.declared = true;
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan");

        let cite = findings
            .citations
            .iter()
            .find(|c| c.id.slug.as_deref() == Some("cli"))
            .expect("FS-cli citation");
        assert_eq!(cite.source_kind, "AR");
        assert!(cite.enclosing_declaration.is_none());
    }

    // §FS-check.3.11 / §FS-check.3.12 / §FS-check.2.3: the [citations] obligation
    // and prohibition passes and the suggestions channel.
    #[test]
    fn citation_directions_obligations_and_prohibitions() {
        let root = test_root("citation_directions_obligations_and_prohibitions");
        // Numbered IDs deliberately do not parse under this repo's own
        // `{kind}-{slug}` grammar, so these fixture tokens stay inert when the
        // grund tree self-scans `tests.rs`.
        write(
            &root.join(".agents/grund.toml"),
            r#"project_name = "scratch"
[[kinds]]
kind = "GOAL"
file = "docs/goals.md"
[[kinds]]
kind = "FS"
folder = "docs/functional-spec"
[[kinds]]
kind = "AR"
folder = "docs/architecture"
[scan]
include = ["docs"]
[citations]
default = "may"
[citations.FS]
should = ["GOAL"]
must-not = ["AR"]
[citations.AR]
must = ["FS"]
"#,
        );
        write(&root.join("docs/goals.md"), "# Goals\n\n## GOAL-001-fast: Fast\n\nBe fast.\n");
        write(
            &root.join("docs/functional-spec/FS-001-login.md"),
            "# FS-001-login: Login\n\nImplements via §AR-001-router.\n",
        );
        write(
            &root.join("docs/architecture/AR-001-router.md"),
            "# AR-001-router: Router\n\nRoutes.\n",
        );

        let config = load_config(&root).expect("load config");
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan");
        let report = check_findings(&findings, &config);

        // must: AR-001-router must cite FS → missing-citation error.
        assert!(
            report.errors.iter().any(|d| d.code == "missing-citation"
                && d.message.contains("AR-001-router must cite FS")),
            "expected missing-citation, got {:?}",
            report.errors.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
        // must-not: FS-001-login cites AR → forbidden-citation error at the site.
        assert!(
            report
                .errors
                .iter()
                .any(|d| d.code == "forbidden-citation" && d.line == Some(3)),
            "expected forbidden-citation at line 3"
        );
        // should: FS-001-login cites no GOAL → suggested-citation on the channel.
        assert!(
            report.suggestions.iter().any(|d| d.code == "suggested-citation"
                && d.message.contains("FS-001-login should cite GOAL")),
            "expected suggested-citation, got {:?}",
            report.suggestions.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
        // should/should-not never leak into the gating channels.
        assert!(
            !report.errors.iter().any(|d| d.code == "suggested-citation")
                && !report.warnings.iter().any(|d| d.code == "suggested-citation"),
            "suggestions must not appear among errors or warnings"
        );
    }

    // §FS-config.3.9.2: Markdown files outside kind homes are still checked for
    // prohibited citation directions, but they are not `code` obligation units.
    #[test]
    fn citation_directions_code_obligations_exempt_markdown() {
        let root = test_root("citation_directions_code_obligations_exempt_markdown");
        write(
            &root.join(".agents/grund.toml"),
            r#"project_name = "scratch"
[scan]
include = ["docs", "README.md"]
[citations]
default = "may"
[citations.code]
must = ["FS"]
"#,
        );
        write(&root.join("docs/goals.md"), "# GOAL-001-fast: Fast\n\nBe fast.\n");
        write(
            &root.join("docs/functional-spec/FS-001-cli.md"),
            "# FS-001-cli: CLI\n\nShip the interface.\n",
        );
        write(&root.join("README.md"), "# Scratch\n\nSee §GOAL-001-fast.\n");

        let config = load_config(&root).expect("load config");
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan");
        let report = check_findings(&findings, &config);

        assert!(
            !report.errors.iter().any(|d| d.code == "missing-citation"
                && d.path.as_deref() == Some(root.join("README.md").as_path())),
            "root README.md must not be a code obligation unit: {:?}",
            report.errors.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    // §FS-config.3.9 / §FS-check.3.11: an E2E case with no scanned citations is
    // still an obligation unit, so `[citations.E2E] must = ["FS"]` is a hard gate
    // in a normal root check that skips direct fixture trees.
    #[test]
    fn citation_directions_e2e_must_is_not_vacuous_without_scanned_files() {
        let root = test_root("citation_directions_e2e_must_is_not_vacuous_without_scanned_files");
        write(
            &root.join(".agents/grund.toml"),
            r#"project_name = "scratch"
[[kinds]]
kind = "FS"
folder = "docs/functional-spec"
[[kinds]]
kind = "E2E"
folder = "e2e/cases"
index = false
[scan]
include = ["e2e"]
[citations]
[citations.E2E]
must = ["FS"]
"#,
        );
        write(&root.join("e2e/cases/001-login/expected.exit"), "0\n");
        write(
            &root.join("e2e/cases/001-login/repo/docs/functional-spec/FS-001-login.md"),
            "# FS-001-login: Login\n\nFixture-only citation target.\n",
        );

        let config = load_config(&root).expect("load config");
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan");
        let report = check_findings(&findings, &config);

        assert!(
            report.errors.iter().any(|d| d.code == "missing-citation"
                && d.path.as_ref().is_some_and(|p| p.ends_with("e2e/cases/001-login"))
                && d.message.contains("E2E-001-login must cite FS")),
            "expected E2E missing-citation, got {:?}",
            report.errors.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    // §FS-config.3.9: E2E `spec.refs` entries count as case-level evidence for
    // citation-direction obligations without entering the ordinary citation stream.
    #[test]
    fn citation_directions_e2e_spec_refs_satisfy_must() {
        let root = test_root("citation_directions_e2e_spec_refs_satisfy_must");
        write(
            &root.join(".agents/grund.toml"),
            r#"project_name = "scratch"
[[kinds]]
kind = "FS"
folder = "docs/functional-spec"
[[kinds]]
kind = "E2E"
folder = "e2e/cases"
index = false
[scan]
include = ["e2e"]
[citations]
[citations.E2E]
must = ["FS"]
"#,
        );
        write(&root.join("e2e/cases/001-login/expected.exit"), "0\n");
        write(&root.join("e2e/cases/001-login/spec.refs"), "FS-001-login.1\n");

        let config = load_config(&root).expect("load config");
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan");
        let report = check_findings(&findings, &config);

        assert!(
            !report.errors.iter().any(|d| d.code == "missing-citation"
                && d.path.as_deref() == Some(root.join("e2e/cases/001-login").as_path())),
            "spec.refs should satisfy E2E must; got {:?}",
            report.errors.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
        assert!(
            findings.citations.is_empty(),
            "spec.refs must not become ordinary citations"
        );
    }

    // §FS-config.3.9: an absent [citations] section runs no direction checks.
    #[test]
    fn citation_directions_absent_section_is_inert() {
        let root = test_root("citation_directions_absent_section_is_inert");
        write(
            &root.join("docs/architecture/AR-001-router.md"),
            "# AR-001-router: Router\n\nNo upward citation.\n",
        );
        let config = Config::default_for(root.clone());
        assert!(!config.citations.declared);
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan");
        let report = check_findings(&findings, &config);
        assert!(
            !report
                .errors
                .iter()
                .any(|d| d.code == "missing-citation" || d.code == "forbidden-citation"),
            "no direction findings without a [citations] section"
        );
    }

    /// §FS-check.2.2.1: a citable folder with scanned content beyond each of its
    /// effective entry files earns one warning per empty obligation kind. This
    /// covers the default README index, a named index, and `index = false`, plus
    /// the `should` wording and the once-per-kind boundary.
    #[test]
    fn empty_folder_obligations_warn_once_with_each_entry_rule() {
        let root = test_root("empty_folder_obligations_warn_once_with_each_entry_rule");
        write(
            &root.join(".agents/grund.toml"),
            r#"grund_config_version = 1

[[kinds]]
kind = "FS"
folder = "docs/spec"
index = false

[[kinds]]
kind = "DEFAULT"
folder = "docs/default"

[[kinds]]
kind = "CUSTOM"
folder = "docs/custom"
index = "INDEX.md"

[[kinds]]
kind = "NOIDX"
folder = "docs/no-index"
index = false

[citations.DEFAULT]
must = ["FS"]
[citations.CUSTOM]
should = ["FS"]
[citations.NOIDX]
must = ["FS"]
"#,
        );
        write(&root.join("docs/default/README.md"), "# Default index\n");
        write(&root.join("docs/default/one.md"), "# First\n");
        write(&root.join("docs/default/two.md"), "# Second\n");
        write(&root.join("docs/custom/INDEX.md"), "# Custom index\n");
        write(&root.join("docs/custom/one.md"), "# First\n");
        write(&root.join("docs/no-index/README.md"), "# Content\n");

        let config = load_config(&root).expect("load config");
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan");
        let report = check_findings(&findings, &config);
        let warnings = report
            .warnings
            .iter()
            .filter(|warning| warning.code == "empty-citation-obligation")
            .collect::<Vec<_>>();

        assert_eq!(warnings.len(), 3, "one warning per qualifying kind");
        assert!(warnings.iter().all(|warning| {
            warning.path.is_none() && warning.line.is_none() && warning.sites.is_empty()
        }));
        assert!(warnings.iter().any(|warning| {
            warning.message
                == "[citations.DEFAULT] must applies to nothing — docs/default/ declares no DEFAULT ID; did you mean `citable = false`?"
        }));
        assert!(warnings.iter().any(|warning| {
            warning.message
                == "[citations.CUSTOM] should applies to nothing — docs/custom/ declares no CUSTOM ID; did you mean `citable = false`?"
        }));
        assert!(warnings.iter().any(|warning| {
            warning.message
                == "[citations.NOIDX] must applies to nothing — docs/no-index/ declares no NOIDX ID; did you mean `citable = false`?"
        }));
        assert!(report.suggestions.is_empty(), "zero units have no should miss");
    }

    /// §FS-check.2.2.1: entry-only folders, file homes, prohibition-only tables,
    /// the homeless kind, and unwalked kinds stay silent even when a full walk
    /// makes the unwalked home visible.
    #[test]
    fn empty_folder_obligation_warning_negative_matrix() {
        let root = test_root("empty_folder_obligation_warning_negative_matrix");
        write(
            &root.join(".agents/grund.toml"),
            r#"grund_config_version = 1

[[kinds]]
kind = "FS"
folder = "docs/spec"
index = false

[[kinds]]
kind = "ENTRY"
folder = "entry"

[[kinds]]
kind = "SINGLE"
file = "docs/single.md"

[[kinds]]
kind = "PROHIBIT"
folder = "prohibit"
index = false

[[kinds]]
kind = "src"
citable = false

[[kinds]]
kind = "UNWALK"
folder = "unwalk"
citable = false
scan = false

[citations.ENTRY]
must = ["FS"]
[citations.SINGLE]
must = ["FS"]
[citations.PROHIBIT]
must-not = ["FS"]
[citations.src]
must = ["FS"]
"#,
        );
        write(&root.join("entry/README.md"), "# Entry-only scaffold\n");
        write(&root.join("docs/single.md"), "# Single-file scaffold\n");
        write(&root.join("prohibit/extra.md"), "# Prohibition-only content\n");
        write(&root.join("src/main.rs"), "fn main() {}\n");
        write(&root.join("unwalk/extra.md"), "# Unwalked content\n");

        let mut config = load_config(&root).expect("load config");
        // Config validation rejects an obligation on an unwalked kind because it
        // can never fire. Inject it after loading to pin the checker's defensive
        // exclusion independently of that load-time rule.
        let unwalk_rule = config
            .citations
            .per_kind
            .get("ENTRY")
            .cloned()
            .expect("ENTRY rule");
        config
            .citations
            .per_kind
            .insert("UNWALK".to_string(), unwalk_rule);
        config.scan_full = true;
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("full scan");
        assert!(
            findings
                .scanned_files
                .iter()
                .any(|file| file.ends_with("unwalk/extra.md")),
            "the full walk must expose the unwalked negative case"
        );
        let report = check_findings(&findings, &config);

        assert!(!report
            .warnings
            .iter()
            .any(|warning| warning.code == "empty-citation-obligation"));
    }

    /// §FS-check.2.2.1: the run-level warning is independent of unrelated graph
    /// errors, including a dangling citation in another scanned file.
    #[test]
    fn empty_folder_obligation_warning_coexists_with_other_findings() {
        let root = test_root("empty_folder_obligation_warning_coexists_with_other_findings");
        write(
            &root.join(".agents/grund.toml"),
            r#"grund_config_version = 1

[[kinds]]
kind = "FS"
folder = "docs/spec"
index = false

[[kinds]]
kind = "SKILL"
folder = "skills"
index = false

[citations.SKILL]
must = ["FS"]
"#,
        );
        write(&root.join("skills/README.md"), "# Skills\n");
        write(&root.join("skills/review.md"), "# Review\n");
        write(&root.join("src/app.rs"), "// see §FS-001-missing\n");

        let config = load_config(&root).expect("load config");
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan");
        let report = check_findings(&findings, &config);

        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.code == "empty-citation-obligation"));
        assert!(report.errors.iter().any(|error| error.code == "dangling"));
    }

    /// §FS-check.2.2.1 / §FS-workspace.5: empty-obligation warnings are evaluated
    /// per workspace member, so an entry-only member does not inherit a sibling's
    /// warning.
    #[test]
    fn empty_folder_obligation_warning_is_isolated_per_workspace_member() {
        let root = test_root("empty_folder_obligation_warning_is_isolated_per_workspace_member");
        write(
            &root.join(".agents/grund.toml"),
            r#"grund_config_version = 1
project_name = "root"

[[kinds]]
kind = "FS"
folder = "docs/spec"
index = false

[[kinds]]
kind = "SKILL"
folder = "skills"

[citations.SKILL]
must = ["FS"]

[workspace]
members = ["member"]
"#,
        );
        write(
            &root.join("member/grund.toml"),
            r#"grund_config_version = 1
project_name = "member"

[[kinds]]
kind = "FS"
folder = "docs/spec"
index = false

[[kinds]]
kind = "SKILL"
folder = "skills"

[citations.SKILL]
must = ["FS"]
"#,
        );
        write(&root.join("skills/README.md"), "# Skills\n");
        write(&root.join("skills/review.md"), "# Review\n");
        write(&root.join("member/skills/README.md"), "# Member skills\n");

        let run = check_run(&root, false);
        let warnings = run
            .report
            .warnings
            .iter()
            .filter(|warning| warning.code == "empty-citation-obligation")
            .collect::<Vec<_>>();

        assert_eq!(warnings.len(), 1, "only the root member has non-entry content");
        assert!(warnings[0].message.contains("skills/"));
    }

    // §FS-init.2.3.5: the generated Citation directions section renders the
    // canonical bullets in `[[kinds]]` order with `code` last.
    #[test]
    fn citation_directions_section_renders_canonical_bullets() {
        let root = test_root("citation_directions_section_renders_canonical_bullets");
        write(
            &root.join(".agents/grund.toml"),
            r#"[citations]
[citations.FS]
should = ["GOAL|FS"]
must-not = ["AR"]
[citations.e2e]
must = ["FS"]
[citations.code]
should = ["FS|AR"]
"#,
        );
        let config = load_config(&root).expect("load config");
        let section = citation_directions_section(&config);
        assert!(section.contains("- **FS** should cite GOAL or FS; never cite AR."));
        // §FS-init.2.3.5: a non-citable kind renders by place, not by name.
        assert!(section.contains("- **tests/e2e/** must cite FS."));
        assert!(section.contains("- **code** (any file outside a kind home) should cite FS or AR."));
        assert!(section.trim_end().ends_with("Unlisted kinds and pairs are fine."));
        // No trailing newline, so the template's placeholder keeps init idempotent.
        assert!(!section.ends_with('\n'));
    }

    // §FS-init.2.3.5: closed-world configs that use `default` plus `may` render
    // both rules and do not leave the open-world fallback sentence in place.
    #[test]
    fn citation_directions_section_renders_default_and_may_rules() {
        let root = test_root("citation_directions_section_renders_default_and_may_rules");
        write(
            &root.join(".agents/grund.toml"),
            r#"[citations]
default = "must-not"
[citations.FS]
may = ["GOAL"]
default = "must-not"
"#,
        );
        let config = load_config(&root).expect("load config");
        let section = citation_directions_section(&config);

        assert!(section.contains("By default, unlisted citation pairs are forbidden."));
        assert!(section.contains("- **FS** may cite GOAL; unlisted citations are forbidden."));
        assert!(section.contains("Unlisted kinds and pairs follow their configured defaults."));
        assert!(!section.contains("Unlisted kinds and pairs are fine."));
    }

    // §FS-config.3.9.5: a `should`/`must-not` pair whose namespaces overlap
    // (`*/AR` covers a bare local `AR`) is rejected; disjoint namespaces are not.
    #[test]
    fn citation_validation_rejects_overlapping_namespace_polarities() {
        let root = test_root("citation_validation_rejects_overlapping_namespace_polarities");
        let cfg = |body: &str| {
            format!(
                "[[kinds]]\nkind = \"FS\"\nfolder = \"docs/functional-spec\"\n[[kinds]]\nkind = \"AR\"\nfolder = \"docs/architecture\"\n[citations.FS]\n{body}"
            )
        };

        // `*/AR` (any) overlaps a bare local `AR` at the opposing level → error.
        write(&root.join(".agents/grund.toml"), &cfg("should = [\"AR\"]\nmust-not = [\"*/AR\"]\n"));
        match load_config(&root) {
            Ok(_) => panic!("overlapping namespace polarities must be rejected"),
            Err(err) => assert!(
                err.to_string().contains("overlap"),
                "expected an overlap error, got: {err}"
            ),
        }

        // A local `AR` permitted while a pinned `root/AR` is forbidden is fine —
        // the matchers are disjoint.
        write(&root.join(".agents/grund.toml"), &cfg("may = [\"AR\"]\nmust-not = [\"root/AR\"]\n"));
        load_config(&root).expect("disjoint namespaces must load");
    }

    // §FS-config.3.9.3 / §FS-workspace.1: namespace-qualified citation targets
    // must use the same alias grammar the scanner can actually produce.
    #[test]
    fn citation_validation_rejects_malformed_namespace_qualifiers() {
        let root = test_root("citation_validation_rejects_malformed_namespace_qualifiers");
        let cfg = |target: &str| {
            format!(
                "[[kinds]]\nkind = \"FS\"\nfolder = \"docs/functional-spec\"\n[[kinds]]\nkind = \"AR\"\nfolder = \"docs/architecture\"\n[citations.FS]\nmust-not = [\"{target}\"]\n"
            )
        };

        // §FS-config.3.9.3: config diagnostics use citation-target vocabulary,
        // name the kind and identify the first invalid qualifier segment.
        let diagnostic = |target: &str| {
            write(&root.join(".agents/grund.toml"), &cfg(target));
            let err = match load_config(&root) {
                Ok(_) => panic!("malformed citation namespace qualifier must fail"),
                Err(err) => format!("{err:#}"),
            };
            let start = err
                .find("citation target ")
                .expect("config error has a citation-target message");
            err[start..].to_string()
        };

        assert_eq!(
            diagnostic("/AR"),
            "citation target `/AR`: namespace qualifier before kind `AR` is empty (expected [a-z][a-z0-9-]*, one segment per workspace level)"
        );
        assert_eq!(
            diagnostic("Root/AR"),
            "citation target `Root/AR`: invalid namespace qualifier segment `Root` in `Root` before kind `AR` (expected [a-z][a-z0-9-]*, one segment per workspace level)"
        );
        assert_eq!(
            diagnostic("group/Api/AR"),
            "citation target `group/Api/AR`: invalid namespace qualifier segment `Api` in `group/Api` before kind `AR` (expected [a-z][a-z0-9-]*, one segment per workspace level)"
        );
        assert_eq!(
            diagnostic("group//AR"),
            "citation target `group//AR`: invalid namespace qualifier segment (empty) in `group/` before kind `AR` (expected [a-z][a-z0-9-]*, one segment per workspace level)"
        );
        assert_eq!(
            diagnostic("group/*/FS"),
            "citation target `group/*/FS`: invalid namespace qualifier segment `*` in `group/*` before kind `FS` (expected [a-z][a-z0-9-]*, one segment per workspace level); `*` may only be the whole qualifier"
        );

        write(&root.join(".agents/grund.toml"), &cfg("*/AR"));
        load_config(&root).expect("whole-qualifier wildcard must load");
        write(&root.join(".agents/grund.toml"), &cfg("root/AR"));
        load_config(&root).expect("valid namespace qualifier must load");

        // §FS-workspace.6.1: the kind is the last segment, so a nested member is
        // pinned by its whole alias path — the same spelling a citation uses.
        write(&root.join(".agents/grund.toml"), &cfg("group/api/AR"));
        let config = load_config(&root).expect("a nested alias qualifier must load");
        let target = &config.citations.per_kind["FS"].must_not[0].targets[0];
        assert_eq!(target.kind, "AR");
        assert!(
            matches!(&target.namespace, NamespaceMatch::Alias(alias) if alias == "group/api"),
            "the whole alias path is the qualifier, not just its last segment"
        );
    }

    #[cfg(unix)]
    #[test]
    fn claude_symlink_to_agents_is_not_a_companion_entrypoint() {
        let root = test_root("claude_symlink_to_agents_is_not_a_companion_entrypoint");
        write(&root.join("AGENTS.md"), &current_block());
        std::os::unix::fs::symlink("AGENTS.md", root.join("CLAUDE.md"))
            .expect("create CLAUDE.md symlink");

        let companions = companion_agent_entrypoints(&root).expect("discover companions");

        assert!(
            companions.is_empty(),
            "CLAUDE.md symlinked to AGENTS.md should be covered by AGENTS.md"
        );
    }
}
