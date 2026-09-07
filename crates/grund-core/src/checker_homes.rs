/// Whether this stub heading is the one-line pointer to an inline declaration in
/// code (`# <ID>: [text](src/foo.rs)` whose target also declares `<ID>`) — such a
/// stub does not count as a second home, so it is not a duplicate (§AR-scanner.4,
/// §FS-show.2.3).
fn is_stub_for_inline_decl(root: &Path, decl: &Declaration, decls: &[Declaration]) -> bool {
    if !decl.is_stub {
        return false;
    }
    let Some(target) = &decl.defined_in else {
        return false;
    };
    let resolved = resolve_stub_target(root, &decl.file, target);
    decls
        .iter()
        .any(|other| paths_same_location(&other.file, &resolved) && other.file != decl.file)
}

struct DeclarationHome<'a> {
    kind: &'a str,
    path: &'a str,
    /// Whether the home's kind declares IDs (§FS-config.3.4). A non-citable home
    /// admits no declaration at all, so the finding it produces names the home
    /// rather than a kind the author was supposed to have written.
    citable: bool,
    /// Whether the home is one exact `file` rather than a `folder`, so the label
    /// below reads as the directory it is.
    exact: bool,
}

impl DeclarationHome<'_> {
    /// How a non-citable home is named in a finding (§FS-check.3.7,
    /// §FS-check.3.6) — the same `<folder>/` label the citation-direction
    /// findings and the generated block use, so one home reads one way
    /// everywhere.
    fn place(&self) -> String {
        if self.exact {
            self.path.to_string()
        } else {
            format!("{}/", self.path)
        }
    }
}

struct SingleFileHome<'a> {
    kind: &'a str,
    path: &'a str,
    physical_path: PathBuf,
}

struct ConfiguredHome<'a> {
    kind: &'a str,
    path: &'a str,
    key: PathBuf,
    exact: bool,
    citable: bool,
}

struct KindHomeIndex<'a> {
    configured_root: PathBuf,
    physical_root: PathBuf,
    single_files: Vec<SingleFileHome<'a>>,
    homes: Vec<ConfiguredHome<'a>>,
    overlapping_homes: bool,
}

impl<'a> KindHomeIndex<'a> {
    fn new(config: &'a Config) -> Self {
        let configured_root = scanned_path_key(&config.root);
        let physical_root = physical_path_key(&config.root);
        let mut single_files = Vec::new();
        let mut homes = Vec::new();

        for kind in &config.kinds {
            if let Some(file) = kind.file.as_deref() {
                // §FS-check.3.7: only a citable kind has declarations to keep in one
                // document, so the single-file rule says nothing about a non-citable
                // `file` home — the home-kind rule below reports what is there, once.
                if kind.citable {
                    single_files.push(SingleFileHome {
                        kind: kind.kind.as_str(),
                        path: file,
                        physical_path: physical_path_key(&config.root.join(file)),
                    });
                }
                homes.push(ConfiguredHome {
                    kind: kind.kind.as_str(),
                    path: file,
                    key: configured_home_path_key(file),
                    exact: true,
                    citable: kind.citable,
                });
            }

            if let Some(folder) = kind.folder.as_deref() {
                homes.push(ConfiguredHome {
                    kind: kind.kind.as_str(),
                    path: folder,
                    key: configured_home_path_key(folder),
                    exact: false,
                    citable: kind.citable,
                });
            }
        }

        let overlapping_homes = homes_have_overlap(&homes);
        if !overlapping_homes {
            homes.sort_by(|left, right| left.exact.cmp(&right.exact));
        }

        Self {
            configured_root,
            physical_root,
            overlapping_homes,
            single_files,
            homes,
        }
    }

    /// The `[[kinds]].file` setting for `kind`, if any — the single document every
    /// declaration of that kind must live in (§FS-config.3.4). Returns `None` for
    /// multi-file kinds (those configured with `folder` instead).
    fn single_file_for_kind(&self, kind: &str) -> Option<&SingleFileHome<'a>> {
        self.single_files.iter().find(|home| home.kind == kind)
    }

    /// The configured kind home that contains `path`, when exactly one
    /// `[[kinds]]` home matches it. `file` homes are exact; `folder` homes are
    /// path-prefix matches against the scanner-recorded path, not the symlink
    /// target (§FS-config.3.4, §FS-check.3.7).
    fn unique_decl_home_for_file(&self, path: &Path) -> Option<DeclarationHome<'a>> {
        let path = scanned_decl_relative_path(path, &self.configured_root, &self.physical_root)?;
        if !self.overlapping_homes {
            return self
                .homes
                .iter()
                .find(|home| home_contains_path(home, path.as_ref()))
                .map(|home| DeclarationHome {
                    kind: home.kind,
                    path: home.path,
                    citable: home.citable,
                    exact: home.exact,
                });
        }

        let mut matches = self.homes.iter().filter_map(|home| {
            home_contains_path(home, path.as_ref()).then_some(DeclarationHome {
                kind: home.kind,
                path: home.path,
                citable: home.citable,
                exact: home.exact,
            })
        });

        let first = matches.next()?;
        if matches.next().is_some() {
            return None;
        }
        Some(first)
    }
}

fn home_contains_path(home: &ConfiguredHome<'_>, path: &Path) -> bool {
    if home.exact {
        path == home.key
    } else {
        path.starts_with(&home.key)
    }
}

fn homes_have_overlap(homes: &[ConfiguredHome<'_>]) -> bool {
    homes.iter().enumerate().any(|(index, left)| {
        homes
            .iter()
            .skip(index + 1)
            .any(|right| homes_overlap(left, right))
    })
}

fn homes_overlap(left: &ConfiguredHome<'_>, right: &ConfiguredHome<'_>) -> bool {
    match (left.exact, right.exact) {
        (true, true) => left.key == right.key,
        (true, false) => left.key.starts_with(&right.key),
        (false, true) => right.key.starts_with(&left.key),
        (false, false) => left.key.starts_with(&right.key) || right.key.starts_with(&left.key),
    }
}

fn paths_same_location(left: &Path, right: &Path) -> bool {
    physical_path_key(left) == physical_path_key(right)
}

fn paths_same_location_key(left: &Path, right: &Path) -> bool {
    physical_path_key(left) == right
}

fn physical_path_key(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| normalize_path_lexically(path))
}

fn scanned_path_key(path: &Path) -> PathBuf {
    normalize_path_lexically(path)
}

fn configured_home_path_key(home: &str) -> PathBuf {
    scanned_path_key(Path::new(home))
}

fn scanned_decl_relative_path<'a>(
    path: &'a Path,
    configured_root: &Path,
    physical_root: &Path,
) -> Option<std::borrow::Cow<'a, Path>> {
    if let Ok(relative) = path.strip_prefix(physical_root) {
        return Some(std::borrow::Cow::Borrowed(relative));
    }
    if let Ok(relative) = path.strip_prefix(configured_root) {
        return Some(std::borrow::Cow::Owned(scanned_path_key(relative)));
    }

    let path = scanned_path_key(path);
    if let Ok(relative) = path.strip_prefix(physical_root) {
        return Some(std::borrow::Cow::Owned(scanned_path_key(relative)));
    }
    if let Ok(relative) = path.strip_prefix(configured_root) {
        return Some(std::borrow::Cow::Owned(scanned_path_key(relative)));
    }
    None
}

/// Whether `path` contains a real (non-stub) inline declaration of `id` —
/// the check that a stub's link target actually carries the inline home it claims
/// (§FS-check.3.4, §AR-checker.2.4, §AR-scanner.4).
fn file_declares_inline_home(path: &Path, id: &Id, config: &Config) -> Result<bool> {
    let text = fs::read_to_string(path)?;
    let is_md = path.extension().and_then(|e| e.to_str()) == Some("md");
    let is_py = path.extension().and_then(|e| e.to_str()) == Some("py");
    let mut py_docstring = PythonDocstringScanState::default();
    for line in text.lines() {
        let scan = source_scan_line(line, is_py, config.docstring_python, &mut py_docstring);
        let scan_line = scan.text;
        if let Some((found, token_end)) =
            declaration_id_on_line(&config.grammar, scan_line, scan.in_py_docstring, is_md)
            && &found == id
        {
            let tail = &scan_line[token_end..];
            if STUB_LINK_HEADING.is_match(tail) {
                continue;
            }
            return Ok(true);
        }
    }
    Ok(false)
}
