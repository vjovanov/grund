//! Black-box contract for the point-size catalog (§FS-list.1, §FS-list.2,
//! §FS-list.3.4, §FS-output-shapes.5, §FS-workspace.8.3).

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_ROOT: AtomicUsize = AtomicUsize::new(0);

struct Repo(PathBuf);

impl Repo {
    fn new(name: &str) -> Self {
        let serial = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/point-size-list")
            .join(format!("{name}-{}-{serial}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create point-size fixture");
        Self(root)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn write(&self, relative: &str, body: &str) {
        let path = self.0.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create fixture parent");
        }
        fs::write(path, body).expect("write fixture");
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_grund"))
        .args(args)
        .current_dir(root)
        .output()
        .unwrap_or_else(|error| panic!("run grund {args:?}: {error}"))
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn assert_run(output: &Output, exit: i32, expected_stdout: &str, expected_stderr: &str) {
    assert_eq!(
        output.status.code(),
        Some(exit),
        "stdout:\n{}stderr:\n{}",
        stdout(output),
        stderr(output)
    );
    assert_eq!(stdout(output), expected_stdout);
    assert_eq!(stderr(output), expected_stderr);
}

fn records(output: &Output) -> Vec<Value> {
    assert_eq!(
        output.status.code(),
        Some(0),
        "size query failed:\n{}",
        stderr(output)
    );
    stdout(output)
        .lines()
        .map(|line| serde_json::from_str(line).expect("size row is JSON"))
        .collect()
}

fn point_config(extra: &str) -> String {
    format!(
        "grund_config_version = 1\nproject_name = \"fixture\"\n\n\
         [reference]\nstrict = true\n\n\
         [id]\nformat = \"{{kind}}-{{slug}}\"\nnamed_sections = true\n\n\
         [[kinds]]\nkind = \"FS\"\nfolder = \"docs\"\nindex = false\n\n\
         [[kinds]]\nkind = \"AR\"\nfolder = \"docs/ar\"\nindex = false\n\n\
         [[kinds]]\nkind = \"E2E\"\nfolder = \"e2e/cases\"\nindex = false\n\n\
         [scan]\ninclude = [\"docs\", \"src\", \"e2e\"]\nextensions = [\"md\", \"rs\"]\n{extra}"
    )
}

fn ascii_words(text: &str) -> usize {
    text.as_bytes()
        .split(|byte| matches!(byte, b'\t'..=b'\r' | b' '))
        .filter(|word| !word.is_empty())
        .count()
}

fn nonblank_lines(text: &str) -> usize {
    text.split('\n')
        .filter(|line| {
            line.as_bytes()
                .iter()
                .any(|byte| !matches!(byte, b'\t'..=b'\r' | b' '))
        })
        .count()
}

#[test]
fn size_rows_cover_every_source_form_and_measure_the_show_slices() {
    let repo = Repo::new("forms");
    repo.write(
        "grund.toml",
        &point_config("\n[[kinds]]\nkind = \"CONST\"\nfile = \"values.json\"\nvalues = true\n"),
    );
    repo.write(
        "docs/FS-alpha.md",
        "# FS-alpha: Alpha\n\nCafé one.\n\n\
         ## 1. Parent\n\nTwo\u{2003}words.\n\n\
         ### 1.1 Child\n\nlast\n\n\
         ## notes: Named\n\nNamed body.\n\n\
         ## Plain heading\n\n```md\n## 9. Fenced\n```\n",
    );
    repo.write(
        "src/lib.rs",
        "/// AR-inline: Inline declaration\n///\n/// Doc-comment lead.\npub struct Inline;\n",
    );
    repo.write(
        "docs/ar/AR-inline.md",
        "# AR-inline: [src/lib.rs](../../src/lib.rs)\n",
    );
    repo.write("values.json", "{\n  \"CONST-price\": [1200, \"USD\"]\n}\n");
    repo.write("e2e/cases/login/expected.exit", "0\n");
    repo.write("e2e/cases/login/command.args", "check\n");

    let all = run(
        repo.path(),
        &["list", "--size=words,lines,bytes", "--format=json"],
    );
    let rows = records(&all);
    let raw = stdout(&all);
    assert!(raw.lines().next().unwrap().starts_with(
        "{\"id\":\"AR-inline\",\"section\":null,\"kind\":\"AR\",\"path\":\"src/lib.rs\",\"line\":1,\"stub\":false,\"defines\":null,\"duplicate\":false,\"lead_words\":"
    ));
    for line in raw.lines() {
        assert!(line.find("lead_words").unwrap() < line.find("full_words").unwrap());
        assert!(line.find("full_words").unwrap() < line.find("lead_lines").unwrap());
        assert!(line.find("full_lines").unwrap() < line.find("lead_bytes").unwrap());
        assert!(line.find("lead_bytes").unwrap() < line.find("full_bytes").unwrap());
        assert!(!line.contains("\"title\"") && !line.contains("\"refs\""));
    }
    let coordinates = rows
        .iter()
        .map(|row| match row["section"].as_str() {
            Some(section) => format!("{}.{}", row["id"].as_str().unwrap(), section),
            None => row["id"].as_str().unwrap().to_string(),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        coordinates,
        [
            "AR-inline",
            "CONST-price",
            "CONST-price.1",
            "CONST-price.2",
            "E2E-login",
            "FS-alpha",
            "FS-alpha.1",
            "FS-alpha.1.1",
            "FS-alpha.notes",
        ]
    );
    assert!(!stdout(&all).contains("FS-alpha.9"));

    for row in rows.iter().filter(|row| row["id"] != "E2E-login") {
        let coordinate = match row["section"].as_str() {
            Some(section) => format!("{}.{}", row["id"].as_str().unwrap(), section),
            None => row["id"].as_str().unwrap().to_string(),
        };
        let mut args = vec!["show", coordinate.as_str(), "--format=json"];
        let lead = run(repo.path(), &args);
        assert_eq!(
            lead.status.code(),
            Some(0),
            "{coordinate}: {}",
            stderr(&lead)
        );
        let lead: Value = serde_json::from_slice(&lead.stdout).expect("show lead JSON");
        args.push("--full");
        let full = run(repo.path(), &args);
        assert_eq!(
            full.status.code(),
            Some(0),
            "{coordinate}: {}",
            stderr(&full)
        );
        let full: Value = serde_json::from_slice(&full.stdout).expect("show full JSON");
        let lead = lead["body"].as_str().unwrap();
        let full = full["body"].as_str().unwrap();
        assert_eq!(row["lead_words"], ascii_words(lead));
        assert_eq!(row["full_words"], ascii_words(full));
        assert_eq!(row["lead_lines"], nonblank_lines(lead));
        assert_eq!(row["full_lines"], nonblank_lines(full));
        assert_eq!(row["lead_bytes"], lead.len());
        assert_eq!(row["full_bytes"], full.len());
    }

    let e2e = rows
        .iter()
        .find(|row| row["id"] == "E2E-login")
        .expect("E2E point has a size row");
    let e2e_manifest =
        "grund check\nexpected exit: 0\nfixtures:\n- command.args\n- expected.exit\n";
    assert_eq!(e2e["lead_words"], ascii_words(e2e_manifest));
    assert_eq!(e2e["full_words"], ascii_words(e2e_manifest));
    assert_eq!(e2e["lead_lines"], nonblank_lines(e2e_manifest));
    assert_eq!(e2e["full_lines"], nonblank_lines(e2e_manifest));
    assert_eq!(e2e["lead_bytes"], e2e_manifest.len());
    assert_eq!(e2e["full_bytes"], e2e_manifest.len());

    let text = run(repo.path(), &["list", "--size=bytes,words", "--kind=FS"]);
    assert_eq!(text.status.code(), Some(0), "{}", stderr(&text));
    let text = stdout(&text);
    assert!(text.lines().all(|line| line.starts_with("FS-alpha")));
    assert!(
        text.lines()
            .all(|line| line.contains("bytes=") && line.contains(" words="))
    );
    assert!(!text.contains("lines="));

    let ordinary = run(repo.path(), &["list", "--kind=FS", "--format=json"]);
    assert_eq!(ordinary.status.code(), Some(0));
    assert!(stdout(&ordinary).contains("\"title\":\"Alpha\""));
    assert!(!stdout(&ordinary).contains("\"section\""));

    assert_run(
        &run(repo.path(), &["list", "--size", "--kind=GOAL"]),
        2,
        "",
        "error: unknown kind `GOAL`\nknown kinds: FS, AR, E2E, CONST\n",
    );
}

#[test]
fn list_help_documents_the_complete_size_surface_without_a_token_unit() {
    let repo = Repo::new("help");
    let help = run(repo.path(), &["help", "list"]);
    assert_eq!(help.status.code(), Some(0));
    let text = stdout(&help);
    assert!(text.contains("--size[=lines,words,bytes]"));
    assert!(text.contains("--top N"));
    assert!(text.contains("lead/full"));
    assert!(!text.contains("tokens"));
}

#[test]
fn top_filtering_workspace_qualification_and_ties_are_deterministic() {
    let repo = Repo::new("order");
    repo.write(
        "grund.toml",
        &point_config("\n[workspace]\nmembers = [\"member\"]\n")
            .replace("project_name = \"fixture\"", "project_name = \"root\""),
    );
    repo.write(
        "member/grund.toml",
        &point_config("").replace("project_name = \"fixture\"", "project_name = \"member\""),
    );
    repo.write("docs/FS-zulu.md", "# FS-zulu: Zulu\n\none two three\n");
    repo.write("docs/use.md", "Uses \u{a7}FS-zulu.\n");
    repo.write(
        "member/docs/FS-alpha.md",
        "# FS-alpha: Alpha\n\none two three\n",
    );
    repo.write(
        "member/docs/FS-beta.md",
        "# FS-beta: Beta\n\none two\n\n## 1. Child\n\none two three four\n",
    );

    let top = records(&run(
        repo.path(),
        &["list", "--size=words", "--top=3", "--format=json"],
    ));
    let ids = top
        .iter()
        .map(|row| {
            (
                row["project"].as_str().unwrap(),
                row["id"].as_str().unwrap(),
                row["section"].clone(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        [
            ("member", "member/FS-beta", Value::String("1".into())),
            ("member", "member/FS-alpha", Value::Null),
            ("root", "root/FS-zulu", Value::Null),
        ]
    );

    let member = records(&run(
        repo.path(),
        &[
            "list",
            "--project=member",
            "--kind=FS",
            "--size=words",
            "--format=json",
        ],
    ));
    assert!(member.iter().all(|row| row["project"] == "member"));
    assert!(
        member
            .iter()
            .all(|row| row["path"].as_str().unwrap().starts_with("member/"))
    );

    let unused = records(&run(
        repo.path(),
        &["list", "--unused", "--size=words", "--format=json"],
    ));
    assert!(!unused.iter().any(|row| row["id"] == "root/FS-zulu"));
    assert!(
        unused
            .iter()
            .any(|row| { row["id"] == "member/FS-beta" && row["section"] == "1" })
    );

    assert_run(
        &run(
            repo.path(),
            &["list", "--project=member", "--kind=AR", "--size"],
        ),
        0,
        "",
        "",
    );
}

#[test]
fn top_uses_the_first_requested_unit_before_the_normal_order_tiebreak() {
    let repo = Repo::new("first-unit");
    repo.write("grund.toml", &point_config(""));
    repo.write("docs/FS-bytes.md", "# FS-bytes: Bytes\n\néééé\n");
    repo.write("docs/FS-words.md", "# FS-words: Words\n\na b c\n");

    let by_bytes = records(&run(
        repo.path(),
        &["list", "--size=bytes,words", "--top=1", "--format=json"],
    ));
    assert_eq!(by_bytes[0]["id"], "FS-bytes");
    let by_words = records(&run(
        repo.path(),
        &["list", "--size=words,bytes", "--top=1", "--format=json"],
    ));
    assert_eq!(by_words[0]["id"], "FS-words");
}

#[test]
fn duplicate_and_stub_rows_are_site_local_without_changing_show_ambiguity() {
    let repo = Repo::new("duplicates");
    repo.write("grund.toml", &point_config(""));
    repo.write("docs/a.md", "# FS-dupe: First\n\nshort\n");
    repo.write(
        "docs/b.md",
        "# FS-dupe: Second\n\na much longer duplicate lead\n",
    );
    repo.write(
        "docs/FS-sections.md",
        "# FS-sections: Sections\n\n## 1. First\n\none\n\n## 1. Second\n\ntwo three\n",
    );
    repo.write(
        "docs/ar/AR-inline.md",
        "# AR-inline: [src/lib.rs](../../src/lib.rs)\n",
    );
    repo.write(
        "src/lib.rs",
        "/// AR-inline: Inline\n///\n/// source body\npub struct Inline;\n",
    );
    repo.write(
        "docs/ar/AR-broken.md",
        "# AR-broken: [src/missing.rs](../../src/missing.rs)\n",
    );

    let rows = records(&run(
        repo.path(),
        &["list", "--size=words", "--format=json"],
    ));
    let dupes = rows
        .iter()
        .filter(|row| row["id"] == "FS-dupe")
        .collect::<Vec<_>>();
    assert_eq!(dupes.len(), 2);
    assert!(dupes.iter().all(|row| row["duplicate"] == true));
    assert_ne!(dupes[0]["lead_words"], dupes[1]["lead_words"]);

    let sections = rows
        .iter()
        .filter(|row| row["id"] == "FS-sections" && row["section"] == "1")
        .collect::<Vec<_>>();
    assert_eq!(sections.len(), 2);
    assert!(sections.iter().all(|row| row["duplicate"] == true));
    assert_ne!(sections[0]["lead_words"], sections[1]["lead_words"]);

    assert_eq!(
        rows.iter().filter(|row| row["id"] == "AR-inline").count(),
        1
    );
    let broken = rows.iter().find(|row| row["id"] == "AR-broken").unwrap();
    assert_eq!(broken["stub"], true);
    assert_eq!(broken["lead_words"], Value::Null);
    assert_eq!(broken["full_words"], Value::Null);
    let empty = rows
        .iter()
        .find(|row| row["id"] == "FS-sections" && row["section"].is_null())
        .unwrap();
    assert_eq!(empty["lead_words"], 0);

    let text = stdout(&run(repo.path(), &["list", "--size=words"]));
    assert_eq!(
        text.lines()
            .filter(|line| line.starts_with("FS-dupe"))
            .filter(|line| line.ends_with("(duplicate — site-local)"))
            .count(),
        2
    );
    let broken_target = broken["defines"].as_str().unwrap();
    assert!(text.lines().any(|line| {
        line.starts_with("AR-broken")
            && line.ends_with(&format!("words=-/-  (broken stub → {broken_target})"))
    }));

    for coordinate in ["FS-dupe", "FS-sections.1"] {
        let shown = run(repo.path(), &["show", coordinate]);
        assert_eq!(shown.status.code(), Some(1));
        assert!(stderr(&shown).contains("ambiguous"));
    }
}
