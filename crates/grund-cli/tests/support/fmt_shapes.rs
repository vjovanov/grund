//! §FS-fmt.7 — the corpus of tree shapes the three equivalence properties of
//! §FS-fmt.7.1, §FS-fmt.7.2 and §FS-fmt.7.3 are each asserted over, plus the
//! comparisons they are equalities between.
//!
//! Shared rather than copied because the three rules are three questions about
//! one command over one corpus: a shape added for one of them is a shape the
//! other two owe an answer for. A case per tree that the defect of the day
//! happened to have is what this repository already had when all three rules
//! held and none of them was a rule (§DF-fmt-one-model.2.2), so the shapes live
//! in one place and each suite iterates all of them.
//!
//! The six shapes are the forms §FS-fmt.7.2 names. `clean-markdown` rewrites and
//! meets no error. `strict-abort` is the whole-declaration-set path refusing up
//! front (§FS-fmt.3). `partial-source` is a source-only scope, where no rewrite
//! needs the whole set, so the run reports its rewrites *and* names the path it
//! could not read. `two-scopes` narrows the rewrite to one directory while the
//! unreadable path sits in the other, which is the shape where `fmt` and `check`
//! walk different trees on purpose. The last two are a workspace, clean and with
//! a member that refuses.
//!
//! Unix only: every shape but the first needs a real broken symlink, and the
//! helpers exist only to serve suites that are themselves `#![cfg(unix)]`.

#![cfg(unix)]
#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The repository root; every fixture is built under its `target/` so nothing
/// lands in the system temp directory.
pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// One tree in the corpus: a builder, and the sub-scope a narrowed run uses.
pub struct Shape {
    pub name: &'static str,
    /// A path inside the fixture that a scoped run narrows to. Never the whole
    /// scan set for the shapes where narrowing is the point.
    pub inner_scope: &'static str,
    /// Whether this shape holds a path the walk cannot read. Declared here and
    /// asserted by the suites, so a fixture that quietly stops exercising its
    /// half of the corpus fails rather than making an equivalence vacuously
    /// true — two runs that both do nothing agree about everything.
    pub reports_unreadable: bool,
    /// Whether a plain `--write` over this shape changes any line, for the same
    /// reason.
    pub rewrites: bool,
    build: fn(&Path),
}

impl Shape {
    /// Build this shape under `target/fmt-equivalence/<slot>/<name>`, replacing
    /// whatever was there. `slot` separates the copies one property needs to
    /// compare, and separates concurrently running suites from each other.
    pub fn materialize(&self, slot: &str) -> PathBuf {
        let dir = manifest_dir()
            .join("target/fmt-equivalence")
            .join(slot)
            .join(self.name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create fixture root");
        (self.build)(&dir);
        dir
    }
}

pub fn shapes() -> Vec<Shape> {
    vec![
        Shape {
            name: "clean-markdown",
            reports_unreadable: false,
            rewrites: true,
            inner_scope: "docs",
            build: build_clean_markdown,
        },
        Shape {
            name: "strict-abort",
            reports_unreadable: true,
            rewrites: false,
            inner_scope: "docs",
            build: build_strict_abort,
        },
        Shape {
            name: "partial-source",
            reports_unreadable: true,
            rewrites: true,
            inner_scope: "src",
            build: build_partial_source,
        },
        Shape {
            name: "two-scopes",
            reports_unreadable: true,
            rewrites: false,
            inner_scope: "docs",
            build: build_two_scopes,
        },
        Shape {
            name: "workspace-clean",
            reports_unreadable: false,
            rewrites: true,
            inner_scope: "docs",
            build: build_workspace_clean,
        },
        Shape {
            name: "workspace-member-abort",
            reports_unreadable: true,
            rewrites: false,
            inner_scope: "docs",
            build: build_workspace_member_abort,
        },
    ]
}

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create fixture directory");
    }
    fs::write(&path, contents).expect("write fixture file");
}

fn broken_symlink(root: &Path, relative: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create fixture directory");
    }
    std::os::unix::fs::symlink("nowhere-at-all", &path).expect("create broken symlink");
}

const MARKDOWN_SCOPE: &str = "grund_config_version = 1\n\n[scan]\ninclude = [\"docs\"]\n\n[fmt.cross_refs]\nenabled = true\n";
const BOTH_SCOPES: &str = "grund_config_version = 1\n\n[scan]\ninclude = [\"docs\", \"src\"]\n\
     extensions = [\"md\", \"rs\"]\n\n[fmt.cross_refs]\nenabled = true\n";
const SOURCE_SCOPE: &str = "grund_config_version = 1\n\n[scan]\ninclude = [\"src\"]\n\
     extensions = [\"md\", \"rs\"]\n\n[fmt.cross_refs]\nenabled = true\n";

const DECLARATION: &str = "# FS-001-alpha: Alpha\n\nAlpha is the first thing.\n";
/// A typed trigger, a bare token `--marker` would mark, and a shorthand whose
/// expansion needs the whole declaration set — three of the four rewrite classes
/// on three lines, so a preview has something to be a preview of.
const CITING_NOTES: &str = "# Notes\n\nSee $$FS-001-alpha for the rest.\n\n\
     Bare here: FS-001-alpha.\n\nShorthand here: $$FS-001.\n";
const CITING_SOURCE: &str = "//! FS-001-alpha: Alpha\n//!\n//! Alpha is the first thing.\n\n\
     // Cites $$FS-001-alpha from a comment.\n";

fn build_clean_markdown(root: &Path) {
    write(root, "grund.toml", MARKDOWN_SCOPE);
    write(root, "docs/FS-001-alpha.md", DECLARATION);
    write(root, "docs/notes.md", CITING_NOTES);
}

fn build_strict_abort(root: &Path) {
    build_clean_markdown(root);
    broken_symlink(root, "docs/FS-002-gone.md");
    broken_symlink(root, "docs/FS-003-also-gone.md");
}

fn build_partial_source(root: &Path) {
    // No Markdown in scope, so the cross-reference pass does not turn itself on
    // (§FS-fmt.6.6) and nothing here needs the whole declaration set: the run
    // rewrites what it can and still owes an account of what it could not read.
    write(root, "grund.toml", SOURCE_SCOPE);
    write(root, "src/lib.rs", CITING_SOURCE);
    write(root, "src/util.rs", "// Also cites $$FS-001-alpha.\n");
    broken_symlink(root, "src/gone.rs");
}

fn build_two_scopes(root: &Path) {
    write(root, "grund.toml", BOTH_SCOPES);
    write(root, "docs/FS-001-alpha.md", DECLARATION);
    write(root, "docs/notes.md", CITING_NOTES);
    write(
        root,
        "src/lib.rs",
        "// Cites $$FS-001-alpha from a comment.\n",
    );
    broken_symlink(root, "src/gone.rs");
}

fn build_workspace_clean(root: &Path) {
    write(
        root,
        "grund.toml",
        "grund_config_version = 1\nproject_name = \"root\"\n\n\
         [scan]\ninclude = [\"docs\"]\n\n\
         [fmt.cross_refs]\nenabled = true\n\n\
         [workspace]\nmembers = [\"packages/sub\"]\n",
    );
    write(
        root,
        "docs/FS-001-root.md",
        "# FS-001-root: Root concern\n\nRoot leans on $$sub/FS-001-sub.\n",
    );
    write(root, "packages/sub/grund.toml", MARKDOWN_SCOPE);
    write(
        root,
        "packages/sub/docs/FS-001-sub.md",
        "# FS-001-sub: A thing sub provides\n\nSub leans on $$root/FS-001-root.\n",
    );
}

fn build_workspace_member_abort(root: &Path) {
    build_workspace_clean(root);
    broken_symlink(root, "packages/sub/docs/FS-002-gone.md");
}

pub fn run_grund(args: &[&str], cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_grund"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("spawn grund")
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

/// The `<path>: <reason>` pairs a run reported, in the order it printed them,
/// with §FS-fmt.7.2's one licensed difference removed: a strict abort spells its
/// lines `error: nothing was rewritten: …` on purpose (§FS-fmt.3), because the
/// two exit `2`s mean opposite things. Everything after that prefix is the part
/// two readers of one tree must agree on.
pub fn scan_error_pairs(output: &Output) -> Vec<String> {
    stderr(output)
        .lines()
        .filter_map(|line| line.strip_prefix("error: "))
        .map(|rest| {
            rest.strip_prefix("nothing was rewritten: ")
                .unwrap_or(rest)
                .to_string()
        })
        .collect()
}

/// Whether a run took the strict whole-declaration-set path, read off the one
/// thing that says so. A strict `fmt` scans the project rather than the scope
/// (§FS-fmt.3, §FS-fmt.2.4), so the run it is comparable to is `check` over the
/// project — the distinction §FS-fmt.7.2 draws per walk rather than per argv.
pub fn aborted_strictly(output: &Output) -> bool {
    stderr(output).contains("error: nothing was rewritten: ")
}

/// The `(path, line)` sites a dry-run report named — the left side of
/// §FS-fmt.7.3.
pub fn reported_sites(output: &Output) -> BTreeSet<(String, usize)> {
    stdout(output)
        .lines()
        .filter_map(|line| {
            let (path, rest) = line.split_once(':')?;
            let (number, _) = rest.split_once(':')?;
            Some((path.to_string(), number.trim().parse().ok()?))
        })
        .collect()
}

/// Every readable regular file under `root`, as lines, keyed by its path
/// relative to `root`. Symlinks are skipped whole: a fixture's broken links are
/// the input, not the result.
pub fn snapshot(root: &Path) -> BTreeMap<String, Vec<String>> {
    let mut files = BTreeMap::new();
    collect(root, root, &mut files);
    files
}

fn collect(root: &Path, dir: &Path, files: &mut BTreeMap<String, Vec<String>>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(metadata) = path.symlink_metadata() else {
            continue;
        };
        if metadata.is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            collect(root, &path, files);
        } else if let Ok(contents) = fs::read_to_string(&path) {
            let relative = path
                .strip_prefix(root)
                .expect("fixture path under its root")
                .to_string_lossy()
                .replace('\\', "/");
            files.insert(relative, contents.lines().map(str::to_string).collect());
        }
    }
}

/// The `(path, line)` sites that actually differ between two snapshots of one
/// tree — the right side of §FS-fmt.7.3. Every `fmt` rewrite edits a line in
/// place, so the comparison is positional; a file that gained or lost lines
/// would show as a run of differences and is reported as such rather than
/// hidden.
pub fn changed_sites(
    before: &BTreeMap<String, Vec<String>>,
    after: &BTreeMap<String, Vec<String>>,
) -> BTreeSet<(String, usize)> {
    let mut sites = BTreeSet::new();
    for (path, original) in before {
        let written = match after.get(path) {
            Some(lines) => lines,
            None => continue,
        };
        for index in 0..original.len().max(written.len()) {
            if original.get(index) != written.get(index) {
                sites.insert((path.clone(), index + 1));
            }
        }
    }
    sites
}
