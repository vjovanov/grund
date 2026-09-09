/// Everything one `grund fmt` walk needs to expand a shorthand: this project's
/// declaration index, plus one per workspace alias for the qualified form
/// (§FS-fmt.2.4, §FS-workspace.8.5).
///
/// Built once per walk rather than per line. `fmt` visits every marker of every
/// scanned file, so resolving each against a linear scan of the declaration set
/// is quadratic on exactly the tree this rewrite exists to clean up
/// (§GOAL-fast-feedback).
struct ShorthandTargets<'a> {
    /// `None` until the walk has a declaration set — §FS-fmt.2.4 defers that scan
    /// until a shorthand is actually met, so a repo without one never pays for it.
    local: Option<ShorthandIndex<'a>>,
    by_alias: BTreeMap<&'a str, ShorthandAliasTarget<'a>>,
}

/// One aliased project's half of `ShorthandTargets`: its declarations, and the
/// config the canonical ID renders under (a workspace may mix `[id] format`s).
struct ShorthandAliasTarget<'a> {
    config: &'a Config,
    index: ShorthandIndex<'a>,
}

impl<'a> ShorthandTargets<'a> {
    fn new(
        config: &Config,
        findings: Option<&'a Findings>,
        workspace: Option<&'a WorkspaceContext>,
    ) -> Self {
        Self {
            local: findings
                .map(|found| ShorthandIndex::build(config, found.declarations.keys())),
            by_alias: workspace
                .map(|workspace| {
                    workspace
                        .projects
                        .iter()
                        .map(|project| {
                            (
                                project.alias.as_str(),
                                ShorthandAliasTarget {
                                    config: &project.config,
                                    index: ShorthandIndex::build(
                                        &project.config,
                                        project.findings.declarations.keys(),
                                    ),
                                },
                            )
                        })
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}
