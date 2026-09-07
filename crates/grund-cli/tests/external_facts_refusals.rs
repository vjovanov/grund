//! Writer and refusal contract for [§FS-fetch.3](../../docs/functional-spec/FS-fetch.md#3-accepted-declaration) through [§FS-fetch.7](../../docs/functional-spec/FS-fetch.md#7-output-and-exits).

#[path = "support/external_facts.rs"]
mod support;

use std::fs;
use support::*;

#[cfg(unix)]
#[test]
fn external_facts_file_home_is_stably_inserted_replaced_and_idempotent() {
    let root = root("file-writes");
    write(
        &root,
        "grund.toml",
        &file_config("docs/tickets.md", None, Some("scripts/fetch-ticket")),
    );
    write(
        &root,
        "docs/tickets.md",
        "# Tickets\n\nPreamble.\n\n## TICKET-1000: Keep A\n\nA.\n\n## TICKET-1500: Old\n\nOld.\n\n## TICKET-2000: Keep B\n\nB.\n",
    );
    executable(
        &root,
        "scripts/fetch-ticket",
        "#!/bin/sh\ncase \"$1\" in\n  TICKET-1500) printf '## TICKET-1500: New\\n\\nReplacement.\\n' ;;\n  TICKET-1750) printf '## TICKET-1750: Inserted\\n\\nBetween.\\n' ;;\n  *) exit 9 ;;\nesac\n",
    );

    assert_code(&run(&root, &["fetch", "TICKET-1500"]), 0, "replace");
    assert_code(&run(&root, &["fetch", "TICKET-1750"]), 0, "insert");
    let expected = concat!(
        "# Tickets\n\nPreamble.\n\n",
        "## TICKET-1000: Keep A\n\nA.\n\n",
        "## TICKET-1500: New\n\nReplacement.\n\n",
        "## TICKET-1750: Inserted\n\nBetween.\n\n",
        "## TICKET-2000: Keep B\n\nB.\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("docs/tickets.md")).unwrap(),
        expected
    );
    let before = tree(&root);
    assert_code(&run(&root, &["fetch", "TICKET-1750"]), 0, "idempotent");
    assert_eq!(tree(&root), before);
}

#[cfg(unix)]
#[test]
fn external_facts_folder_home_preserves_verbatim_output_environment_and_argv() {
    let root = root("folder-write");
    write(
        &root,
        "grund.toml",
        concat!(
            "grund_config_version = 1\n[id]\nformat = \"{kind}-{slug}\"\n",
            "[[kinds]]\nkind = \"TICKET\"\nfolder = \"snapshots\"\nindex = false\n",
            "format = \"{kind}-{number}\"\nfetch = \"scripts/fetch-ticket\"\n",
            "[scan]\ninclude = [\"docs\"]\nrespect_gitignore = false\n"
        ),
    );
    executable(
        &root,
        "scripts/fetch-ticket",
        "#!/bin/sh\n[ \"$#\" -eq 1 ] || exit 31\n[ \"$1\" = TICKET-77 ] || exit 32\n[ \"$GRUND_FETCH_PROBE\" = inherited ] || exit 33\nprintf '# TICKET-77: Folder fact\\n\\n  spacing stays  \\n'\n",
    );
    let output = run_env(
        &root,
        &["fetch", "TICKET-77"],
        "GRUND_FETCH_PROBE",
        "inherited",
    );
    assert_code(&output, 0, "folder fetch");
    assert_eq!(
        fs::read(root.join("snapshots/TICKET-77.md")).unwrap(),
        b"# TICKET-77: Folder fact\n\n  spacing stays  \n"
    );
}

#[test]
fn external_facts_config_validation_pins_coupling_enum_homes_and_default() {
    let valid = root("config-default");
    write(
        &valid,
        "grund.toml",
        &file_config("docs/tickets.md", None, Some("scripts/fetch-ticket")),
    );
    let shown = run(&valid, &["config", "show", "."]);
    assert_code(&shown, 0, "fetch defaults to must");
    assert!(stdout(&shown).contains("format = \"{kind}-{number}\""));
    assert!(stdout(&shown).contains("resolve = \"must\""));
    assert!(stdout(&shown).contains("fetch = \"scripts/fetch-ticket\""));

    for (name, row, expected) in [
        (
            "resolve-without-fetch",
            "file = \"docs/tickets.md\"\nresolve = \"should\"",
            "requires `fetch`",
        ),
        (
            "resolve-may",
            "file = \"docs/tickets.md\"\nresolve = \"may\"\nfetch = \"scripts/f\"",
            "must or should",
        ),
        ("fetch-no-home", "fetch = \"scripts/f\"", "exactly one"),
        (
            "fetch-both-homes",
            "file = \"docs/tickets.md\"\nfolder = \"tickets\"\nfetch = \"scripts/f\"",
            "exactly one",
        ),
        (
            "invalid-kind-format",
            "file = \"docs/tickets.md\"\nformat = \"{kind}\"",
            "at least one",
        ),
    ] {
        let root = root(name);
        let format = if name == "invalid-kind-format" {
            ""
        } else {
            "format = \"{kind}-{number}\"\n"
        };
        write(
            &root,
            "grund.toml",
            &format!(
                "grund_config_version = 1\n[id]\nformat = \"{{kind}}-{{slug}}\"\n[[kinds]]\nkind = \"TICKET\"\n{format}{row}\n"
            ),
        );
        let output = run(&root, &["config", "validate", "."]);
        assert_code(&output, 1, name);
        assert!(
            stderr(&output).contains(expected),
            "{name}: {}",
            stderr(&output)
        );
    }

    let legacy = root("config-absent-keys");
    write(
        &legacy,
        "grund.toml",
        concat!(
            "grund_config_version = 1\n[reference]\nstrict = true\n",
            "[id]\nformat = \"{kind}-{slug}\"\n",
            "[[kinds]]\nkind = \"FS\"\nfolder = \"docs/specs\"\nindex = false\n",
            "[scan]\ninclude = [\"docs\"]\nrespect_gitignore = false\n"
        ),
    );
    write(
        &legacy,
        "docs/specs/FS-alpha.md",
        "# FS-alpha: Alpha\n\nFact.\n",
    );
    write(&legacy, "docs/guide.md", "# Guide\n\nSee \u{a7}FS-alpha.\n");
    let output = run(&legacy, &["check", "."]);
    assert_code(&output, 0, "absent keys retain schema-v1 behavior");
    assert_eq!(stdout(&output), "success\n");
}

#[cfg(unix)]
#[test]
fn external_facts_rejected_outputs_and_failures_never_mutate_the_tree() {
    for (name, script, home, expected) in [
        (
            "malformed",
            "#!/bin/sh\nprintf 'not a declaration\\n'\n",
            "docs/tickets.md",
            "declaration",
        ),
        (
            "wrong-id",
            "#!/bin/sh\nprintf '## TICKET-9: Wrong\\n\\nBody.\\n'\n",
            "docs/tickets.md",
            "TICKET-8",
        ),
        (
            "duplicate-output",
            "#!/bin/sh\nprintf '## TICKET-8: One\\n\\n## TICKET-8: Two\\n'\n",
            "docs/tickets.md",
            "exactly one",
        ),
        (
            "nonzero",
            "#!/bin/sh\nprintf 'provider failed\\n' >&2\nexit 7\n",
            "docs/tickets.md",
            "7",
        ),
        (
            "blocked-write",
            "#!/bin/sh\nprintf '## TICKET-8: Valid\\n\\nBody.\\n'\n",
            "blocked/tickets.md",
            "write",
        ),
    ] {
        let root = root(name);
        write(
            &root,
            "grund.toml",
            &file_config(home, Some("must"), Some("scripts/fetch-ticket")),
        );
        write(&root, "docs/guide.md", "# Guide\n\nSee \u{a7}TICKET-8.\n");
        if name == "blocked-write" {
            write(&root, "blocked", "not a directory\n");
        }
        executable(&root, "scripts/fetch-ticket", script);
        let before = tree(&root);
        let output = run(&root, &["fetch", "TICKET-8"]);
        assert_code(&output, 2, name);
        assert!(
            stderr(&output).contains(expected),
            "{name}: {}",
            stderr(&output)
        );
        assert_eq!(tree(&root), before, "{name} mutated the tree");
    }
}

#[cfg(unix)]
#[test]
fn external_facts_missing_fetch_bad_id_and_shell_text_are_refused_without_execution() {
    let no_fetch = root("no-fetch");
    write(
        &no_fetch,
        "grund.toml",
        &file_config("docs/tickets.md", None, None),
    );
    let output = run(&no_fetch, &["fetch", "TICKET-8"]);
    assert_code(&output, 2, "kind without fetch");
    assert!(stderr(&output).contains("no fetch"));

    let bad_id = root("bad-id");
    write(
        &bad_id,
        "grund.toml",
        &file_config("docs/tickets.md", None, Some("scripts/fetch-ticket")),
    );
    executable(
        &bad_id,
        "scripts/fetch-ticket",
        "#!/bin/sh\ntouch executed\n",
    );
    let output = run(&bad_id, &["fetch", "not-an-id"]);
    assert_code(&output, 1, "unparseable ID");
    assert!(!bad_id.join("executed").exists());

    let no_shell = root("no-shell");
    write(
        &no_shell,
        "grund.toml",
        &file_config(
            "docs/tickets.md",
            None,
            Some("scripts/fetch-ticket;touch-pwned"),
        ),
    );
    executable(&no_shell, "scripts/fetch-ticket", "#!/bin/sh\nexit 0\n");
    let before = tree(&no_shell);
    let output = run(&no_shell, &["fetch", "TICKET-8"]);
    assert_code(&output, 2, "shell text is an executable path");
    assert!(stderr(&output).contains("scripts/fetch-ticket;touch-pwned"));
    assert!(!no_shell.join("touch-pwned").exists());
    assert_eq!(tree(&no_shell), before);
}

#[cfg(unix)]
#[test]
fn external_facts_fetched_body_citations_are_live() {
    let root = root("live-body-citation");
    write(
        &root,
        "grund.toml",
        &file_config("docs/tickets.md", None, Some("scripts/fetch-ticket")),
    );
    write(&root, "docs/guide.md", "# Guide\n\nSee \u{a7}TICKET-8.\n");
    executable(
        &root,
        "scripts/fetch-ticket",
        "#!/bin/sh\nprintf '## TICKET-8: Fact\\n\\nDepends on \u{a7}TICKET-9.\\n'\n",
    );
    assert_code(&run(&root, &["fetch", "TICKET-8"]), 0, "fetch live body");
    let check = run(&root, &["check", "."]);
    assert_code(&check, 1, "body citation is checked");
    assert_eq!(
        stdout(&check),
        "docs/tickets.md:3: unknown reference TICKET-9; no snapshot in docs/tickets.md — run grund fetch TICKET-9\n"
    );
}
