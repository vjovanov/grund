//! Writer regressions for rendered ordering and replacement metadata under
//! [§FS-fetch.4](../../docs/functional-spec/FS-fetch.md#4-file-home-write) and
//! [§FS-fetch.6](../../docs/functional-spec/FS-fetch.md#6-stability-and-ownership).
#![cfg(unix)]

#[path = "support/external_facts.rs"]
mod support;

use std::fs;
use support::*;

#[test]
fn external_facts_file_home_orders_by_the_effective_rendered_id() {
    let root = root("file-rendered-order");
    write(
        &root,
        "grund.toml",
        concat!(
            "grund_config_version = 1\n[id]\nformat = \"{kind}-{slug}\"\n",
            "[[kinds]]\nkind = \"TICKET\"\nfile = \"docs/tickets.md\"\n",
            "format = \"{kind}-{slug}-{number}\"\nfetch = \"scripts/fetch-ticket\"\n",
            "[scan]\ninclude = [\"docs\"]\nrespect_gitignore = false\n"
        ),
    );
    write(
        &root,
        "docs/tickets.md",
        "# Tickets\n\n## TICKET-zeta-1: Existing\n\nBody.\n",
    );
    executable(
        &root,
        "scripts/fetch-ticket",
        "#!/bin/sh\nprintf '## TICKET-alpha-9: Inserted\\n\\nBody.\\n'\n",
    );

    assert_code(&run(&root, &["fetch", "TICKET-alpha-9"]), 0, "insert");
    assert_eq!(
        fs::read_to_string(root.join("docs/tickets.md")).unwrap(),
        concat!(
            "# Tickets\n\n",
            "## TICKET-alpha-9: Inserted\n\nBody.\n\n",
            "## TICKET-zeta-1: Existing\n\nBody.\n"
        )
    );
}

#[test]
fn external_facts_replacement_preserves_file_and_folder_home_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let file = root("file-mode-preservation");
    write(
        &file,
        "grund.toml",
        &file_config("docs/tickets.md", None, Some("scripts/fetch-ticket")),
    );
    write(
        &file,
        "docs/tickets.md",
        "# Tickets\n\n## TICKET-8: Old\n\nOld.\n",
    );
    fs::set_permissions(
        file.join("docs/tickets.md"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    executable(
        &file,
        "scripts/fetch-ticket",
        "#!/bin/sh\nprintf '## TICKET-8: New\\n\\nNew.\\n'\n",
    );
    assert_code(&run(&file, &["fetch", "TICKET-8"]), 0, "file replace");
    assert_eq!(
        fs::metadata(file.join("docs/tickets.md"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );

    let folder = root("folder-mode-preservation");
    write(
        &folder,
        "grund.toml",
        concat!(
            "grund_config_version = 1\n[id]\nformat = \"{kind}-{slug}\"\n",
            "[[kinds]]\nkind = \"TICKET\"\nfolder = \"snapshots\"\nindex = false\n",
            "format = \"{kind}-{number}\"\nfetch = \"scripts/fetch-ticket\"\n",
            "[scan]\ninclude = [\"docs\"]\nrespect_gitignore = false\n"
        ),
    );
    write(
        &folder,
        "snapshots/provider.md",
        "# TICKET-8: Old\n\nOld.\n",
    );
    fs::set_permissions(
        folder.join("snapshots/provider.md"),
        fs::Permissions::from_mode(0o640),
    )
    .unwrap();
    executable(
        &folder,
        "scripts/fetch-ticket",
        "#!/bin/sh\nprintf '# TICKET-8: New\\n\\nNew.\\n'\n",
    );
    assert_code(&run(&folder, &["fetch", "TICKET-8"]), 0, "folder replace");
    assert_eq!(
        fs::metadata(folder.join("snapshots/provider.md"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o640
    );
}
