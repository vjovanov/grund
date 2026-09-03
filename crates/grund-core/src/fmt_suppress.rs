// The two scopes a repository takes out of `fmt`'s reach (§FS-fmt.2.5): the
// `[fmt] exclude` list, and the `grund:fmt` regions written in the files. Beside
// the walk rather than in it, per §AR-core-module-layout.3's file budget.

/// The fixed text of a suppression directive (§FS-fmt.2.5.2). Not configurable,
/// for the same reason the fence syntax is not: a marker that reads differently
/// per repository is one nobody can recognize on sight
/// (§DF-fmt-suppression.2.2).
const FMT_DIRECTIVE: &str = "grund:fmt";

/// The files this project's `[fmt] exclude` takes out of every rewrite
/// (§FS-fmt.2.5.1). Empty — and free — for the repositories that set no key,
/// which is why the matcher is an `Option` rather than an empty `Gitignore`:
/// `matched_path_or_any_parents` walks a path's ancestors, and a run with no
/// patterns should not pay for that on every file (§GOAL-fast-feedback).
struct FmtExcluded {
    root: PathBuf,
    matcher: Option<Gitignore>,
}

impl FmtExcluded {
    /// The matcher for one project's config. Patterns were already validated at
    /// load (§FS-config.3.10), so a failure here is a grund bug rather than a
    /// user error — it is still reported rather than swallowed, because the
    /// alternative is a `--write` that silently rewrites a protected file.
    fn new(config: &Config) -> Result<Self> {
        let matcher = if config.fmt_exclude.is_empty() {
            None
        } else {
            Some(
                build_fmt_exclude_matcher(&config.fmt_exclude)
                    .map_err(|message| anyhow!("[fmt] exclude: {message}"))?,
            )
        };
        Ok(Self {
            root: config.root.clone(),
            matcher,
        })
    }

    /// Whether `path` is excluded. The patterns are config-root-relative
    /// (§FS-config.3.10), so the walk's path is rebased before matching and a
    /// path that is not under the root — nothing the walk produces today — is
    /// simply not excluded rather than guessed about.
    fn contains(&self, path: &Path) -> bool {
        let Some(matcher) = &self.matcher else {
            return false;
        };
        let Ok(relative) = path.strip_prefix(&self.root) else {
            return false;
        };
        matcher
            .matched_path_or_any_parents(relative, false)
            .is_ignore()
    }
}

/// Compile `[fmt] exclude` into a matcher over config-root-relative paths
/// (§FS-config.3.10). The root is left empty on purpose: every caller rebases
/// the path itself, so the matcher never has to guess how much of an absolute
/// path is the project.
fn build_fmt_exclude_matcher(patterns: &[String]) -> std::result::Result<Gitignore, String> {
    let mut builder = GitignoreBuilder::new("");
    for pattern in patterns {
        builder.add_line(None, pattern).map_err(|err| err.to_string())?;
    }
    builder.build().map_err(|err| err.to_string())
}

/// §FS-config.3.10: reject a malformed glob at the line that wrote it, rather
/// than at the first `grund fmt` in a repository that has forgotten about it
/// (§FS-config.4.3).
fn validate_fmt_exclude(patterns: &[String]) -> std::result::Result<(), String> {
    build_fmt_exclude_matcher(patterns)
        .map(|_| ())
        .map_err(|message| format!("[fmt] exclude: {message}"))
}

/// The `grund:fmt off` / `grund:fmt on` region state for one file
/// (§FS-fmt.2.5.2): whether the rewrite is on at the line about to be read, and
/// how a directive is spelled in this file's syntax. Every file starts with the
/// rewrite on — nothing carries across files.
struct FmtDirectives<'a> {
    rewriting: bool,
    /// `None` in Markdown, where the directive is an HTML comment. In a source
    /// file, the comment prefixes to strip — built once per file rather than
    /// once per line, because `comment_strip_prefixes` allocates and sorts
    /// (§GOAL-fast-feedback).
    prefixes: Option<Vec<&'a str>>,
}

impl<'a> FmtDirectives<'a> {
    fn new(config: &'a Config, is_md: bool) -> Self {
        Self {
            rewriting: true,
            prefixes: (!is_md).then(|| comment_strip_prefixes(config)),
        }
    }

    /// Take `line` when it is a directive, returning whether it was one. A
    /// directive line is never rewritten, whichever state it leaves behind
    /// (§FS-fmt.2.5.2) — so the caller passes it through on `true`.
    fn consume(&mut self, line: &str, docstring: DocstringContent<'_>) -> bool {
        match self.directive(line, docstring) {
            Some(rewriting) => {
                // A redundant directive is a no-op: assigning the state it
                // already holds is exactly that (§FS-fmt.2.5.2).
                self.rewriting = rewriting;
                true
            }
            None => false,
        }
    }

    /// Whether the rewrite is on for the line about to be read.
    fn rewriting(&self) -> bool {
        self.rewriting
    }

    /// The state this line asks for, if it is a directive at all. Only an exact
    /// content match counts (§FS-fmt.2.5.2): `grund:fmt-off` and
    /// `grund:fmt off please` are ordinary comments.
    fn directive(&self, line: &str, docstring: DocstringContent<'_>) -> Option<bool> {
        // The cheap gate first — `fmt` asks this of every line of every scanned
        // file, and almost none of them carry the text (§GOAL-fast-feedback).
        if !line.contains(FMT_DIRECTIVE) {
            return None;
        }
        let content = match &self.prefixes {
            None => line.trim().strip_prefix("<!--")?.strip_suffix("-->")?.trim(),
            Some(prefixes) => {
                let text = docstring.text_of(line);
                // A docstring line is documentation and carries no prefix of its
                // own (§FS-fmt.2.3.1); every other source line must actually be
                // a comment, or a string holding this text would toggle a region.
                if !docstring.is_docstring()
                    && !prefixes
                        .iter()
                        .any(|prefix| text.trim_start().starts_with(prefix))
                {
                    return None;
                }
                strip_comment_tokens(text, prefixes)
            }
        };
        match content.strip_prefix(FMT_DIRECTIVE)?.trim() {
            "off" => Some(false),
            "on" => Some(true),
            _ => None,
        }
    }
}
