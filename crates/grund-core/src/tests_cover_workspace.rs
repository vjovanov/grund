/// Test module: `grund cover` indexes every project the run loaded, and counts
/// a cross-project citation toward the file that wrote it (§FS-workspace.8.6,
/// §DF-cover-workspace-scope). These are the two halves of the same defect —
/// a coverage index that omits whole projects, and one that reports a fully
/// grounded file as citing nothing — and both print as absence, which is why
/// they need a test that names what should be there.
#[cfg(test)]
mod tests_cover_workspace {
    use super::*;
    use super::tests_support::*;

    const ROOT_CONFIG: &str = "grund_config_version = 1\n\
        project_name = \"root\"\n\n\
        [id]\n\
        format = \"{kind}-{slug}\"\n\
        slug_pattern = \"[a-z][a-z0-9-]*\"\n\n\
        [[kinds]]\n\
        prefix = \"FS\"\n\
        folder = \"docs\"\n\n\
        [scan]\n\
        include = [\"docs\"]\n\
        extensions = [\"md\"]\n\n\
        [workspace]\n\
        members = [\"packages/sub\"]\n";

    /// The member drops the `[workspace]` block and keeps everything else, so a
    /// case that wants the two projects to differ says which key it changed.
    fn member_config(extra: &str) -> String {
        format!(
            "grund_config_version = 1\n\
             project_name = \"sub\"\n\n\
             [id]\n\
             {extra}\n\
             [[kinds]]\n\
             prefix = \"FS\"\n\
             folder = \"docs\"\n\n\
             [scan]\n\
             include = [\"docs\"]\n\
             extensions = [\"md\"]\n"
        )
    }

    const SLUG_ID: &str = "format = \"{kind}-{slug}\"\nslug_pattern = \"[a-z][a-z0-9-]*\"\n";

    /// A workspace root with one member, both scoped to `docs`. `root_doc` and
    /// `member_doc` are the bodies the case is about; the member's file is named
    /// for the ID it declares, since the two `[id]` grammars here spell it
    /// differently.
    fn workspace(
        name: &str,
        member_id: &str,
        member_file: &str,
        root_doc: &str,
        member_doc: &str,
    ) -> PathBuf {
        let root = test_root(name);
        write(&root.join("grund.toml"), ROOT_CONFIG);
        write(&root.join("packages/sub/grund.toml"), &member_config(member_id));
        write(&root.join("docs/FS-root-thing.md"), root_doc);
        write(&root.join("packages/sub/docs").join(member_file), member_doc);
        root
    }

    fn cover_at(path: &Path) -> CoverOutput {
        cover(CoverOpts {
            path: path.to_path_buf(),
            path_provided: true,
        })
        .expect("cover")
    }

    /// One row per scanned file as `<project>|<path>|<id>@<line>:<column>,…`,
    /// which is every field the JSON renderer reads (§FS-cover.3.2) in the order
    /// it prints them.
    fn rows(output: &CoverOutput) -> Vec<String> {
        output
            .entries
            .iter()
            .map(|entry| {
                let citations = entry
                    .citations
                    .iter()
                    .map(|citation| {
                        format!("{}@{}:{}", citation.id, citation.line, citation.column)
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                format!(
                    "{}|{}|{}",
                    entry.project.as_deref().unwrap_or("-"),
                    entry.path,
                    citations
                )
            })
            .collect()
    }

    /// §FS-workspace.8.6: the member's files are in the index, spelled from the
    /// workspace root, and carrying their own alias. Before this, a run at the
    /// root saw only `docs/` and exited `0` over a tree it had not read
    /// (§REQ-no-missed-citation.1).
    #[test]
    fn cover_at_a_workspace_root_indexes_every_member() {
        let root = workspace(
            "cover_at_a_workspace_root_indexes_every_member",
            SLUG_ID,
            "FS-sub-thing.md",
            "# FS-root-thing: Root\n\nRoot body.\n",
            "# FS-sub-thing: Sub\n\nSub leans on \u{a7}FS-sub-thing itself.\n",
        );
        assert_eq!(
            rows(&cover_at(&root)),
            vec![
                "root|docs/FS-root-thing.md|".to_string(),
                "sub|packages/sub/docs/FS-sub-thing.md|FS-sub-thing@3:14".to_string(),
            ]
        );
    }

    /// §DF-cover-workspace-scope.2.2: a `§<alias>/<ID>` is one of the citing
    /// file's citations. A file whose citations are all qualified used to print
    /// as `(no citations)` — the report a reader takes for "ungrounded".
    #[test]
    fn cover_counts_a_qualified_citation_toward_the_citing_file() {
        let root = workspace(
            "cover_counts_a_qualified_citation_toward_the_citing_file",
            SLUG_ID,
            "FS-sub-thing.md",
            "# FS-root-thing: Root\n\nRoot leans on \u{a7}sub/FS-sub-thing.\n",
            "# FS-sub-thing: Sub\n\nSub body.\n",
        );
        assert_eq!(
            rows(&cover_at(&root)),
            vec![
                "root|docs/FS-root-thing.md|sub/FS-sub-thing@3:15".to_string(),
                "sub|packages/sub/docs/FS-sub-thing.md|".to_string(),
            ]
        );
    }

    /// §FS-workspace.8.6: the ID renders under the **target** project's `[id]`
    /// config, matching `refs` (§FS-workspace.8.2). The member numbers its IDs
    /// and the root does not, so rendering under the citing project's config
    /// would spell the same citation `sub/FS-sub-thing`.
    #[test]
    fn a_qualified_id_renders_under_the_target_projects_config() {
        let root = workspace(
            "a_qualified_id_renders_under_the_target_projects_config",
            "format = \"{kind}-{number}-{slug}\"\nslug_pattern = \"[a-z][a-z0-9-]*\"\n",
            "FS-001-sub-thing.md",
            "# FS-root-thing: Root\n\nRoot leans on \u{a7}sub/FS-001-sub-thing.\n",
            "# FS-001-sub-thing: Sub\n\nSub body.\n",
        );
        let output = cover_at(&root);
        assert_eq!(
            rows(&output),
            vec![
                "root|docs/FS-root-thing.md|sub/FS-001-sub-thing@3:15".to_string(),
                "sub|packages/sub/docs/FS-001-sub-thing.md|".to_string(),
            ]
        );
    }

    /// §FS-workspace.8 intro: a `<path>` inside a member is member-scoped. The
    /// alias is `None` there — no workspace is loaded, so there is no namespace
    /// to name — and the JSON field is omitted rather than emitted empty
    /// (§DF-cover-workspace-scope.2.3).
    #[test]
    fn cover_under_a_member_path_stays_member_local() {
        let root = workspace(
            "cover_under_a_member_path_stays_member_local",
            SLUG_ID,
            "FS-sub-thing.md",
            "# FS-root-thing: Root\n\nRoot leans on \u{a7}sub/FS-sub-thing.\n",
            "# FS-sub-thing: Sub\n\nSub body.\n",
        );
        assert_eq!(
            rows(&cover_at(&root.join("packages/sub"))),
            vec!["-|docs/FS-sub-thing.md|".to_string()]
        );
    }

    /// §FS-workspace.8.7: a member's unreadable file fails the run launched at
    /// the workspace root, named from that root. Rendered against the member it
    /// would name a file that does not exist from where the run started
    /// (§FS-errors.4).
    #[cfg(unix)]
    #[test]
    fn a_members_scan_error_is_reported_from_the_workspace_root() {
        let root = workspace(
            "a_members_scan_error_is_reported_from_the_workspace_root",
            SLUG_ID,
            "FS-sub-thing.md",
            "# FS-root-thing: Root\n\nRoot body.\n",
            "# FS-sub-thing: Sub\n\nSub body.\n",
        );
        symlink("FS-gone-target.md", &root.join("packages/sub/docs/FS-gone.md"));
        let output = cover_at(&root);
        assert_eq!(
            output
                .scan_errors
                .iter()
                .map(|error| error.path.clone())
                .collect::<Vec<_>>(),
            vec!["packages/sub/docs/FS-gone.md".to_string()]
        );
    }

    /// The two API surfaces answer with the same index: `cover_text` drops the
    /// fields the human view does not print and nothing else (§FS-cover.3.1).
    #[test]
    fn cover_text_indexes_the_same_files_as_cover() {
        let root = workspace(
            "cover_text_indexes_the_same_files_as_cover",
            SLUG_ID,
            "FS-sub-thing.md",
            "# FS-root-thing: Root\n\nRoot leans on \u{a7}sub/FS-sub-thing.\n",
            "# FS-sub-thing: Sub\n\nSub body.\n",
        );
        let json = cover_at(&root);
        let text = cover_text(CoverOpts {
            path: root.clone(),
            path_provided: true,
        })
        .expect("cover_text");
        assert_eq!(
            text.entries
                .iter()
                .map(|entry| (entry.path.clone(), entry.citations.len()))
                .collect::<Vec<_>>(),
            json.entries
                .iter()
                .map(|entry| (entry.path.clone(), entry.citations.len()))
                .collect::<Vec<_>>()
        );
    }
}
