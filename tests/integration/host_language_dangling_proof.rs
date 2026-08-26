//! §REQ-no-missed-citation.3 — every doc-comment form the scanner claims to
//! support is proven by a dangling citation planted in that form, and
//! §FS-config.3.5 makes the claim concrete: every default extension and every
//! default comment prefix. Read from the corpus goldens — an `unknown
//! reference` line names the file and the line, the fixture shows the form —
//! so an extension nobody plants a dangling citation in, a comment prefix no
//! reported line opens with, or a golden that points at a line not citing
//! the ID it reports, fails here. The defaults come from the binary, not from a copy.

#[path = "binaries.rs"]
mod binaries;
#[path = "corpus.rs"]
mod corpus;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// `extensions` and `comment_prefixes` as `grund config show` prints them for
/// a directory with no config of its own — outside this repository, so the
/// walk up finds none.
fn defaults(grund: &Path) -> (Vec<String>, Vec<String>) {
    let dir = std::env::temp_dir().join(format!("grund-defaults-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create empty dir");
    let output = Command::new(grund)
        .args(["config", "show"])
        .arg(&dir)
        .output()
        .expect("run grund config show");
    let _ = fs::remove_dir_all(&dir);
    let text = String::from_utf8_lossy(&output.stdout);
    let list = |key: &str| -> Vec<String> {
        let line = text
            .lines()
            .find(|line| line.starts_with(&format!("{key} = [")))
            .unwrap_or_else(|| panic!("no `{key}` line in config show output:\n{text}"));
        line.split('[')
            .nth(1)
            .unwrap()
            .trim_end_matches(']')
            .split(',')
            .map(|item| item.trim().trim_matches('"').to_string())
            .filter(|item| !item.is_empty())
            .collect()
    };
    (list("extensions"), list("comment_prefixes"))
}

fn find_under(root: &Path, relative: &str) -> Option<PathBuf> {
    let direct = root.join(relative);
    if direct.is_file() {
        return Some(direct);
    }
    // A narrowed or workspace run reports relative to another root: search.
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).ok()?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.ends_with(relative) {
                return Some(path);
            }
        }
    }
    None
}

#[test]
fn every_default_extension_and_comment_prefix_has_a_dangling_proof() {
    let repo = binaries::repo_root();
    let grund = binaries::grund();
    let (extensions, prefixes) = defaults(&grund);
    assert!(
        extensions.len() >= 20 && prefixes.len() >= 5,
        "defaults look wrong: {extensions:?} {prefixes:?}"
    );
    let mut by_extension: BTreeMap<String, usize> = BTreeMap::new();
    let mut opening_tokens: BTreeSet<String> = BTreeSet::new();
    let mut problems = Vec::new();
    for case in corpus::case_dirs(&repo) {
        // A `symlinks` manifest names files the harness creates at run time, so
        // the committed fixture cannot show their form.
        if case.join("symlinks").is_file() {
            continue;
        }
        let Ok(stdout) = fs::read_to_string(case.join("expected.stdout")) else {
            continue;
        };
        let root = case.join("repo");
        for line in stdout.lines() {
            let Some((location, rest)) = line.split_once(": unknown reference ") else {
                continue;
            };
            let unresolved = rest
                .trim_start()
                .trim_end_matches(|c: char| !(c.is_alphanumeric() || "-./_".contains(c)))
                .split(|c: char| c.is_whitespace() || ";,".contains(c))
                .next()
                .unwrap_or("");
            let Some((path, line_no)) = location.rsplit_once(':') else {
                continue;
            };
            let Ok(line_no) = line_no.parse::<usize>() else {
                continue;
            };
            let Some(file) = find_under(&root, path) else {
                problems.push(format!(
                    "{}: golden names {path}, not in the fixture",
                    case.display()
                ));
                continue;
            };
            let text = fs::read_to_string(&file).unwrap_or_default();
            let Some(cited) = text.lines().nth(line_no - 1) else {
                problems.push(format!(
                    "{}: {path}:{line_no} is past the end of the file",
                    case.display()
                ));
                continue;
            };
            // A non-strict fixture cites bare, so the proof that the golden points
            // at the citation is the unresolved ID itself, not the marker.
            if !cited.contains(unresolved) {
                problems.push(format!(
                    "{}: {path}:{line_no} does not cite {unresolved}: {cited:?}",
                    case.display()
                ));
                continue;
            }
            let extension = file
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("")
                .to_string();
            *by_extension.entry(extension).or_default() += 1;
            let token = cited
                .trim_start()
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string();
            opening_tokens.insert(token);
        }
    }
    let unproven_extensions = extensions
        .iter()
        .filter(|ext| !by_extension.contains_key(*ext))
        .cloned()
        .collect::<Vec<_>>();
    let unproven_prefixes = prefixes
        .iter()
        .filter(|prefix| {
            !opening_tokens
                .iter()
                .any(|token| token.starts_with(prefix.as_str()))
        })
        .cloned()
        .collect::<Vec<_>>();
    eprintln!("dangling proof per extension: {by_extension:?}");
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    assert!(
        unproven_extensions.is_empty(),
        "default extensions with no dangling citation anywhere in the corpus: {unproven_extensions:?}"
    );
    assert!(
        unproven_prefixes.is_empty(),
        "default comment prefixes no reported dangling line opens with: {unproven_prefixes:?}"
    );
}
