//! End-to-end contract for per-kind external snapshots: [§FS-config.3.2](../../docs/functional-spec/FS-config.md#32-id--id-grammar), [§FS-check.4.12](../../docs/functional-spec/FS-check.md#412-missing-snapshot), and [§FS-fetch](../../docs/functional-spec/FS-fetch.md#fs-fetch-grund-materializes-one-external-fact-snapshot).

#[path = "support/external_facts.rs"]
mod support;

use std::fs;
use support::*;

#[cfg(unix)]
#[test]
fn external_facts_citation_materializes_then_resolves_offline() {
    let root = root("materialize-resolve");
    write(
        &root,
        "grund.toml",
        &file_config(
            "docs/tickets.md",
            Some("should"),
            Some("scripts/fetch-ticket"),
        ),
    );
    write(
        &root,
        "docs/guide.md",
        "# Guide\n\nThe rollout follows \u{a7}TICKET-1234.\n",
    );
    executable(
        &root,
        "scripts/fetch-ticket",
        "#!/bin/sh\n[ \"$#\" -eq 1 ] || exit 41\n[ \"$1\" = TICKET-1234 ] || exit 42\nprintf '## TICKET-1234: External fact\\n\\nFetched body.\\n'\n",
    );

    let before = run(&root, &["check", "."]);
    assert_code(&before, 0, "missing should snapshot");
    assert_eq!(
        stdout(&before),
        "docs/guide.md:3: no snapshot for TICKET-1234 in docs/tickets.md — run grund fetch TICKET-1234\n"
    );

    let fetch = run(&root, &["fetch", "TICKET-1234"]);
    assert_code(&fetch, 0, "fetch snapshot");
    assert_eq!(stdout(&fetch), "");
    assert_eq!(stderr(&fetch), "");
    assert_eq!(
        fs::read_to_string(root.join("docs/tickets.md")).unwrap(),
        "## TICKET-1234: External fact\n\nFetched body.\n"
    );

    fs::rename(
        root.join("scripts/fetch-ticket"),
        root.join("scripts/fetch-ticket.disabled"),
    )
    .unwrap();
    let check = run(&root, &["check", "."]);
    assert_code(&check, 0, "offline check");
    assert_eq!(stdout(&check), "success\n");
    let show = run(&root, &["show", "TICKET-1234"]);
    assert_code(&show, 0, "offline show");
    assert!(stdout(&show).contains("Fetched body."));
    let refs = run(&root, &["refs", "TICKET-1234"]);
    assert_code(&refs, 0, "offline refs");
    assert!(stdout(&refs).contains("docs/guide.md:3:"));
}

#[test]
fn external_facts_missing_snapshot_text_and_json_have_fixed_identities() {
    for (resolve, code, text, exit) in [
        (
            "must",
            "dangling",
            "unknown reference TICKET-1234; no snapshot in docs/tickets.md — run grund fetch TICKET-1234",
            1,
        ),
        (
            "should",
            "missing-snapshot",
            "no snapshot for TICKET-1234 in docs/tickets.md — run grund fetch TICKET-1234",
            0,
        ),
    ] {
        let root = root(&format!("diagnostic-{resolve}"));
        write(
            &root,
            "grund.toml",
            &file_config(
                "docs/tickets.md",
                Some(resolve),
                Some("scripts/fetch-ticket"),
            ),
        );
        write(
            &root,
            "docs/guide.md",
            "# Guide\n\nSee \u{a7}TICKET-1234.\n",
        );

        let plain = run(&root, &["check", "."]);
        assert_code(&plain, exit, resolve);
        assert_eq!(stdout(&plain), format!("docs/guide.md:3: {text}\n"));
        assert_eq!(stderr(&plain), "");

        let json = run(&root, &["check", ".", "--format", "json"]);
        assert_code(&json, exit, &format!("{resolve} json"));
        let severity = if resolve == "must" {
            "error"
        } else {
            "warning"
        };
        assert_eq!(
            stdout(&json),
            format!(
                "{{\"severity\":\"{severity}\",\"path\":\"docs/guide.md\",\"line\":3,\"code\":\"{code}\",\"message\":\"{text}\",\"sites\":null}}\n"
            )
        );
    }
}

#[test]
fn external_facts_existing_hints_replace_only_the_fetch_action_at_both_levels() {
    for (resolve, base, exit) in [
        ("must", "unknown reference TICKET-1234; no snapshot in", 1),
        ("should", "no snapshot for TICKET-1234 in", 0),
    ] {
        let root = root(&format!("hint-precedence-{resolve}"));
        write(
            &root,
            "grund.toml",
            &file_config(
                "docs/tickets.md",
                Some(resolve),
                Some("scripts/fetch-ticket"),
            ),
        );
        write(
            &root,
            "docs/tickets.md",
            "# Tickets\n\n## TICKET-1235: Nearby\n\nExists.\n",
        );
        write(
            &root,
            "docs/guide.md",
            "# Guide\n\nSee \u{a7}TICKET-1235 and \u{a7}TICKET-1234.\nIllustration: `\u{a7}TICKET-9999`.\n",
        );
        let output = run(&root, &["check", "."]);
        assert_code(&output, exit, resolve);
        let lines = stdout(&output);
        assert!(
            lines.contains(&format!(
                "docs/guide.md:3: {base} docs/tickets.md; did you mean TICKET-1235?\n"
            )),
            "near-ID hint did not preserve the snapshot base: {lines}"
        );
        let inline_base = if resolve == "must" {
            "unknown reference TICKET-9999; no snapshot in"
        } else {
            "no snapshot for TICKET-9999 in"
        };
        assert!(
            lines.contains(&format!(
                "docs/guide.md:4: {inline_base} docs/tickets.md; write <§>TICKET-9999 if this is an illustration\n"
            )),
            "inline-code hint did not preserve the snapshot base: {lines}"
        );
        assert!(!lines.contains("grund fetch"));
    }
}

#[cfg(unix)]
#[test]
fn external_facts_per_kind_grammar_is_shared_by_every_cli_consumer() {
    let root = root("consumer-parity");
    write(
        &root,
        "grund.toml",
        concat!(
            "grund_config_version = 1\n[reference]\nstrict = true\n",
            "[id]\nformat = \"{kind}-{slug}\"\nslug_pattern = \"[a-z][a-z-]*\"\n",
            "[[kinds]]\nkind = \"FS\"\nfolder = \"docs/specs\"\nindex = false\n",
            "[[kinds]]\nkind = \"TICKET\"\nfile = \"docs/tickets.md\"\n",
            "format = \"{kind}-{number}\"\nfetch = \"scripts/fetch-ticket\"\nresolve = \"must\"\n",
            "[scan]\ninclude = [\"docs\"]\nrespect_gitignore = false\n"
        ),
    );
    write(
        &root,
        "docs/specs/FS-alpha.md",
        "# FS-alpha: Alpha\n\nLocal.\n",
    );
    write(
        &root,
        "docs/tickets.md",
        "# Tickets\n\n## TICKET-1234: Ticket\n\nExternal.\n",
    );
    write(
        &root,
        "docs/guide.md",
        "# Guide\n\nSee \u{a7}FS-alpha and \u{a7}TICKET-1234.\n",
    );
    executable(
        &root,
        "scripts/fetch-ticket",
        "#!/bin/sh\ntouch fetch-ran\nexit 99\n",
    );

    for args in [
        vec!["check", "."],
        vec!["show", "TICKET-1234"],
        vec!["refs", "TICKET-1234"],
        vec!["list", "."],
        vec!["cover", "."],
        vec!["fmt", "--check", "."],
        vec!["complete", "ids", ".", "--prefix", "TICKET-"],
    ] {
        let output = run(&root, &args);
        assert_code(&output, 0, &args.join(" "));
    }
    let allocate = run(&root, &["id", "TICKET", "Another ticket"]);
    assert_code(&allocate, 0, "id TICKET");
    assert_eq!(stdout(&allocate), "TICKET-1235\n");
    assert!(!root.join("fetch-ran").exists());
}

#[cfg(unix)]
#[test]
fn external_facts_qualified_fetch_uses_member_home_and_local_argument() {
    let root = root("workspace-qualified");
    write(
        &root,
        "grund.toml",
        "project_name = \"root\"\n[scan]\ninclude = [\"docs\"]\nrespect_gitignore = false\n[workspace]\nmembers = [\"api\"]\n",
    );
    write(
        &root,
        "docs/guide.md",
        "# Guide\n\nSee \u{a7}api/TICKET-1234.\n",
    );
    write(
        &root,
        "api/grund.toml",
        &file_config(
            "docs/tickets.md",
            Some("must"),
            Some("scripts/fetch-ticket"),
        ),
    );
    executable(
        &root,
        "api/scripts/fetch-ticket",
        "#!/bin/sh\nprintf '%s' \"$1\" > received-id\nprintf '## TICKET-1234: Member ticket\\n\\nMember body.\\n'\n",
    );

    let fetch = run(&root, &["fetch", "api/TICKET-1234"]);
    assert_code(&fetch, 0, "qualified fetch");
    assert_eq!(
        fs::read_to_string(root.join("api/received-id")).unwrap(),
        "TICKET-1234"
    );
    assert!(root.join("api/docs/tickets.md").exists());
    assert!(!root.join("docs/tickets.md").exists());
    let check = run(&root, &["check", "."]);
    assert_code(&check, 0, "workspace after fetch");
}
