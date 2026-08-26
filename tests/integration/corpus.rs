//! §FS-config.3.4.4 — the e2e corpus as the integration sweeps read it: the
//! cases that are a plain `check` of a fixture carrying its own config, so a
//! CLI run from the fixture root and an editor rooted there name one tree, and
//! the counts of what was left out, which every sweep prints rather than hides.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

pub struct Case {
    pub name: String,
    pub root: PathBuf,
}

pub struct Selection {
    pub cases: Vec<Case>,
    pub other_commands: usize,
    pub no_config: Vec<String>,
}

pub fn case_dirs(repo: &Path) -> Vec<PathBuf> {
    let cases_dir = repo.join("tests/e2e/cases");
    let mut entries = fs::read_dir(&cases_dir)
        .unwrap_or_else(|err| panic!("read {}: {err}", cases_dir.display()))
        .map(|entry| entry.expect("case entry").path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    entries.sort();
    entries
}

pub fn plain_check_cases(repo: &Path) -> Selection {
    let mut cases = Vec::new();
    let mut other_commands = 0;
    let mut no_config = Vec::new();
    for dir in case_dirs(repo) {
        let name = dir.file_name().unwrap().to_string_lossy().into_owned();
        let command = fs::read_to_string(dir.join("command.args")).ok();
        let plain_check = match command.as_deref().map(str::trim) {
            None | Some("check {repo}") => true,
            Some(_) => false,
        };
        let root = dir.join("repo");
        if !plain_check || dir.join("symlinks").is_file() || !root.is_dir() {
            other_commands += 1;
            continue;
        }
        if !root.join("grund.toml").is_file() && !root.join(".agents/grund.toml").is_file() {
            no_config.push(name);
            continue;
        }
        cases.push(Case { name, root });
    }
    Selection {
        cases,
        other_commands,
        no_config,
    }
}
