// The `symlinks` manifest half of the case runner: reading a case's link
// declarations, deciding whether the platform can build them, and creating them
// in the fixture copy. Split out of `case_runner.rs` along the seam that was
// already there — that file runs cases and compares goldens, this one builds the
// one part of a fixture git cannot carry. Included into the same module, so the
// two halves still share `case_name` and the manifest readers.

/// Why a case with a `symlinks` manifest may not run. One reason, named once, so
/// the summary and the skip cannot drift.
const SYMLINK_SKIP: &str = "the platform cannot create a directory symlink under target/e2e-work";

/// The `symlinks` manifest of a case: one `<link> -> <target>` per line, both
/// relative to the fixture repo. Symlinks are built into the *copy* at run time
/// rather than committed, because git on Windows checks a committed symlink out as
/// a text file holding its target unless developer mode is on — the fixture would
/// then be a different tree, and the golden would fail for a reason the case is not
/// about. §FS-workspace.6.1's containment rule can only be reached through one, so
/// the corpus needs the affordance.
fn case_symlinks(case: &Path) -> Vec<(String, String)> {
    let manifest = case.join("symlinks");
    if !manifest.is_file() {
        return Vec::new();
    }
    let name = case_name(case);
    let mut links = Vec::new();
    for (index, line) in read_to_string(&manifest).lines().enumerate() {
        // Every rejection below points at the line that has to change, the way a
        // `grund` diagnostic does: the manifest is a contract, and a contract needs
        // a location (§FS-errors.2.1).
        let at = format!("{name}: symlinks:{}", index + 1);
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let mut halves = line.split("->");
        let link = halves.next().unwrap_or_default().trim().to_string();
        let target = halves
            .next()
            .unwrap_or_else(|| panic!("{at}: expected `<link> -> <target>`, got `{line}`"))
            .trim()
            .to_string();
        // `split_once` took the first arrow and swallowed the rest into the target,
        // so `self -> . -> extra` created a link named `. -> extra` and the case
        // failed later as a golden mismatch instead of here as the malformed line
        // it is.
        assert!(
            halves.next().is_none(),
            "{at}: more than one `->` in `{line}` — one link per line"
        );
        assert!(
            link_stays_in_the_copy(&link),
            "{at}: the link path `{link}` must stay inside the fixture copy — relative, \
             `/`-separated, and no `..`"
        );
        assert!(!target.is_empty(), "{at}: the link target is empty");
        links.push((link, target));
    }
    // A manifest with no links is not a case without symlinks: it is a case that
    // *says* it needs one. It also switched the platform skip off, so such a case
    // was green on every platform while testing nothing the manifest describes.
    assert!(
        !links.is_empty(),
        "{name}: the `symlinks` manifest declares no links — delete the file or fill it in"
    );
    links
}

/// Whether a manifest's **link** path is one the harness may write.
///
/// The *target* is deliberately free to leave the copy — `link -> ..` is the whole
/// point of one of the cases — but the link is where the harness creates a file,
/// and `PathBuf::join` discards its base for an absolute path, so an unchecked link
/// wrote symlinks anywhere the test process could reach: an absolute line landed
/// outside the tree, `../../../../x` landed in the repository root, and nothing
/// cleaned either up. Every component has to be a plain name, which rejects the
/// absolute forms, `..`, and a Windows drive prefix in one test; `\` is rejected
/// separately, because it is a separator on one platform and a filename character
/// on another and the manifest is documented as `/`-separated.
fn link_stays_in_the_copy(link: &str) -> bool {
    !link.is_empty()
        && !link.contains('\\')
        && Path::new(link)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

/// Whether this platform lets the test process create a directory symlink **where
/// the cases create theirs** — `target/e2e-work/`, the fixture-copy root. On
/// Windows that needs developer mode or elevation, so the answer is a probe rather
/// than a `cfg`; probing `std::env::temp_dir()` instead answered a question no case
/// asks, and a `TMPDIR` that cannot hold a symlink silently deleted the coverage on
/// Linux and macOS.
///
/// Probed once per process (`OnceLock`), which is also what makes it safe from the
/// threads libtest runs the passes on: the old pid-keyed probe raced with itself,
/// and its `remove_file` cleanup fails on Windows, where a directory symlink is a
/// directory — so the first call left the probe behind and every later call
/// reported `false`.
fn symlinks_supported(manifest_dir: &Path) -> bool {
    static SUPPORTED: OnceLock<bool> = OnceLock::new();
    *SUPPORTED.get_or_init(|| {
        let work = manifest_dir.join("target/e2e-work");
        if fs::create_dir_all(&work).is_err() {
            return false;
        }
        // The probe link must be the *kind of link the cases write*, not the
        // easiest one to write. A `.` target has no separator and no `..`, so it
        // answered `true` on a platform where every real manifest target — all of
        // them multi-segment and `/`-separated — produced a link resolving to
        // nothing, and the cases then failed one at a time as golden mismatches.
        // This one crosses a directory and comes back, which is the shape of
        // `docs/shared -> ../shared-docs`.
        let probe_root = work.join("symlink-support-probe");
        let _ = fs::remove_dir_all(&probe_root);
        if fs::create_dir_all(probe_root.join("target-dir/inner")).is_err()
            || fs::create_dir_all(probe_root.join("nest")).is_err()
        {
            return false;
        }
        let probe = probe_root.join("nest/link");
        let made = create_symlink(Path::new("../target-dir/inner"), &probe, true).is_ok()
            // Creation succeeding is not the question — resolving is. Windows
            // reports success for a link it will never follow.
            && probe.exists();
        remove_symlink(&probe);
        let _ = fs::remove_dir_all(&probe_root);
        made
    })
}

/// Remove a path that may be a *directory* symlink. `remove_file` is enough on
/// Unix and fails on Windows, where such a link is a directory to `fs`.
fn remove_symlink(path: &Path) {
    if fs::remove_file(path).is_err() {
        let _ = fs::remove_dir(path);
    }
}

/// Create one link of the kind the target calls for. Unix has a single kind and
/// ignores the flag; **Windows stores the kind in the link itself**, and a link
/// made with the wrong one does not resolve — a `symlink_dir` onto a file is a
/// broken link to every reader, so `grund` reported the fixture's own file links
/// as unreadable paths and the case exited `2` where its golden says `1`. The
/// manifest does not spell the kind out, because the tree already knows it: the
/// target is resolved against the link's own directory and asked whether it is a
/// directory. A target that does not exist is a file link, which is right for the
/// deliberately-dangling cases and indistinguishable for the rest.
fn create_symlink(target: &Path, link: &Path, target_is_dir: bool) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let _ = target_is_dir;
        std::os::unix::fs::symlink(target, link)
    }
    #[cfg(windows)]
    {
        // A manifest target is `/`-separated (`e2e/README.md`), and Windows stores
        // the target *string* in the reparse point rather than resolving it at
        // creation: `symlink_dir` accepts `../shared-docs` and produces a link that
        // resolves to nothing. The separator is the whole of the difference, which
        // is why the slash-free targets (`.`, `..`) this harness carried before
        // multi-segment ones arrived worked here for years.
        let target = PathBuf::from(target.to_string_lossy().replace('/', "\\"));
        if target_is_dir {
            std::os::windows::fs::symlink_dir(&target, link)
        } else {
            std::os::windows::fs::symlink_file(&target, link)
        }
    }
}

fn create_case_symlinks(case: &Path, repo_copy: &Path, name: &str) {
    let copy_root = fs::canonicalize(repo_copy)
        .unwrap_or_else(|err| panic!("{name}: canonicalize {}: {err}", repo_copy.display()));
    for (link, target) in case_symlinks(case) {
        let link_path = repo_copy.join(&link);
        // The lexical check in `case_symlinks` rules out a link path that *reads*
        // like an escape; this one rules out the tree answering differently — an
        // earlier line in the same manifest may have made a parent directory a
        // symlink out of the copy. Both are cheap, and only both together mean "the
        // harness writes inside the fixture copy".
        let parent = link_path.parent().unwrap_or(repo_copy);
        let landing = fs::canonicalize(parent).unwrap_or_else(|err| {
            panic!("{name}: link {link}: resolve {}: {err}", parent.display())
        });
        assert!(
            landing.starts_with(&copy_root),
            "{name}: link {link} would be created outside the fixture copy, at {}",
            landing.display()
        );
        // Resolved against the link's own directory, which is what a relative
        // symlink target is resolved against — not against the fixture root.
        let target_path = landing.join(&target);
        let target_existed = target_path.exists();
        create_symlink(Path::new(&target), &link_path, target_path.is_dir()).unwrap_or_else(
            |err| {
                panic!("{name}: link {link} -> {target}: {err}");
            },
        );
        // A link whose target was there and still does not resolve was built with
        // the wrong kind, and the case would fail later as a golden mismatch about
        // `grund` rather than here as the fixture problem it is.
        assert!(
            !target_existed || link_path.exists(),
            "{name}: link {link} -> {target} does not resolve after creation — \
             the fixture was built wrong, not the golden"
        );
    }
}
