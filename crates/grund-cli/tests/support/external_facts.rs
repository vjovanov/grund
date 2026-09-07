//! Test-only repositories and process helpers for [§FS-fetch](../../../docs/functional-spec/FS-fetch.md#fs-fetch-grund-materializes-one-external-fact-snapshot).

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub fn root(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("target/external-facts")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create fixture root");
    dir
}

pub fn write(root: &Path, relative: &str, body: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create fixture parent");
    }
    fs::write(path, body).expect("write fixture file");
}

#[cfg(unix)]
pub fn executable(root: &Path, relative: &str, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    write(root, relative, body);
    let path = root.join(relative);
    let mut permissions = fs::metadata(&path).expect("fetcher metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("make fetcher executable");
}

#[cfg(not(unix))]
pub fn executable(root: &Path, relative: &str, body: &str) {
    write(root, relative, body);
}

pub fn run(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_grund"))
        .args(args)
        .current_dir(root)
        .output()
        .expect("run grund")
}

pub fn run_env(root: &Path, args: &[&str], key: &str, value: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_grund"))
        .args(args)
        .env(key, value)
        .current_dir(root)
        .output()
        .expect("run grund with environment")
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

pub fn file_config(home: &str, resolve: Option<&str>, fetch: Option<&str>) -> String {
    file_config_for_project("facts", home, resolve, fetch)
}

pub fn file_config_for_project(
    project: &str,
    home: &str,
    resolve: Option<&str>,
    fetch: Option<&str>,
) -> String {
    let mut row = format!(
        "grund_config_version = 1\nproject_name = \"{project}\"\n\n\
         [reference]\nstrict = true\n\n\
         [id]\nformat = \"{{kind}}-{{slug}}\"\nslug_pattern = \"[a-z][a-z-]*\"\n\n\
         [[kinds]]\nkind = \"TICKET\"\nfile = \"{home}\"\nformat = \"{{kind}}-{{number}}\"\n"
    );
    if let Some(resolve) = resolve {
        row.push_str(&format!("resolve = \"{resolve}\"\n"));
    }
    if let Some(fetch) = fetch {
        row.push_str(&format!("fetch = \"{fetch}\"\n"));
    }
    row.push_str("\n[scan]\ninclude = [\"docs\"]\nrespect_gitignore = false\n");
    row
}

pub fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn collect(base: &Path, at: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
        let mut entries = fs::read_dir(at)
            .expect("read fixture tree")
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                collect(base, &path, files);
            } else {
                let relative = path
                    .strip_prefix(base)
                    .expect("fixture-relative path")
                    .to_string_lossy()
                    .replace('\\', "/");
                files.insert(relative, fs::read(path).expect("read fixture file"));
            }
        }
    }

    let mut files = BTreeMap::new();
    collect(root, root, &mut files);
    files
}

pub fn tree_with_directories(root: &Path) -> BTreeMap<String, Option<Vec<u8>>> {
    fn collect(base: &Path, at: &Path, entries: &mut BTreeMap<String, Option<Vec<u8>>>) {
        let mut children = fs::read_dir(at)
            .expect("read fixture tree")
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        children.sort_by_key(|entry| entry.file_name());
        for entry in children {
            let path = entry.path();
            let relative = path
                .strip_prefix(base)
                .expect("fixture-relative path")
                .to_string_lossy()
                .replace('\\', "/");
            if path.is_dir() {
                entries.insert(format!("{relative}/"), None);
                collect(base, &path, entries);
            } else {
                entries.insert(relative, Some(fs::read(path).expect("read fixture file")));
            }
        }
    }

    let mut entries = BTreeMap::new();
    collect(root, root, &mut entries);
    entries
}

pub fn assert_code(output: &Output, code: i32, context: &str) {
    assert_eq!(
        output.status.code(),
        Some(code),
        "{context}: stdout={:?}, stderr={:?}",
        stdout(output),
        stderr(output)
    );
}
