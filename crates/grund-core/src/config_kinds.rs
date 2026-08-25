// The `[[kinds]]` half of the config parser (§FS-config.3.4), in a file of its
// own beside `config_discovery.rs` and `config_cmd.rs` (§AR-core-module-layout.1):
// the built-in kind table and its per-name defaults, the per-key reader that
// fills one `[[kinds]]` entry, and the whole-list validation that runs once the
// file is read. The last two are pure functions over the parsed entries — the
// discovery, the section walk, and every other section's keys stay in
// `config.rs`. The defaults live here rather than in `model.rs` because they are
// config defaults (§AR-core-module-layout.1) and because this is the file that
// resolves them onto a declared block.

/// The canonical kind set (§FS-config.3.4). `e2e` and `integration` are
/// *non-citable*: a test proves a claim someone else wrote, so it cites and is
/// never cited, and lowercase names them as places rather than ID prefixes.
const DEFAULT_KINDS: &[&str] = &[
    "GRUND",
    "GOAL",
    "FS",
    "AR",
    "DF",
    "DA",
    "e2e",
    "integration",
    "RM",
];

/// §FS-config.3.4: `E2E` is the canonical `index = false` kind — its home holds
/// case directories rather than a navigable document set, and the `e2e/README.md`
/// one level up describes the case layout in English instead of naming `E2E-` IDs.
/// Every other default folder kind takes the `README.md` default.
///
/// `E2E` is no longer one of the [`DEFAULT_KINDS`], but the default stays keyed
/// on the name: it exists for the configs that declare `E2E` themselves, which
/// is every config `grund init` wrote before the kind left the default set.
fn default_kind_index(kind: &str) -> KindIndex {
    match kind {
        "E2E" => KindIndex::Disabled,
        _ => KindIndex::Default,
    }
}

/// Whether a built-in kind declares IDs (§FS-config.3.4). The two test kinds do
/// not: a test is evidence for a claim declared elsewhere, so it has a home and
/// citation directions but no ID namespace.
fn default_kind_citable(kind: &str) -> bool {
    !matches!(kind, "e2e" | "integration")
}

/// Default home folder for each built-in kind — the directory `grund id` proposes
/// a path under and `grund check` expects the declaration to live in (§FS-config.3.4).
fn default_kind_folder(kind: &str) -> Option<&'static str> {
    match kind {
        "AR" => Some("docs/architecture"),
        "DA" => Some("docs/decisions/architectural"),
        "DF" => Some("docs/decisions/functional"),
        "E2E" => Some("e2e/cases"),
        "e2e" => Some("tests/e2e"),
        "integration" => Some("tests/integration"),
        // GRUND, GOAL, RM are single-file kinds — see `default_kind_file`. A
        // kind can always be broken up later by swapping `file = "…"` for
        // `folder = "…"` and moving the document into the folder.
        _ => None,
    }
}

/// Default single-file home for kinds whose declarations all live in one
/// document — `GRUND` in `docs/grund.md`, `GOAL` in `docs/goals.md`, `FS` in
/// `requirements.md`, and `RM` in `docs/roadmap.md` (§FS-config.3.4). Other
/// built-in kinds have no `file` (each declaration is its own file).
fn default_kind_file(kind: &str) -> Option<&'static str> {
    match kind {
        "GRUND" => Some("docs/grund.md"),
        "GOAL" => Some("docs/goals.md"),
        "FS" => Some("requirements.md"),
        "RM" => Some("docs/roadmap.md"),
        _ => None,
    }
}

/// Default human title for each built-in kind, printed by `grund id` (§FS-config.3.4,
/// §FS-id.2).
fn default_kind_title(kind: &str) -> Option<&'static str> {
    match kind {
        "GRUND" => Some("Why: project motivation"),
        "GOAL" => Some("Where: project direction and outcomes"),
        "FS" => Some("What: behavior, requirements, and constraints"),
        "AR" => Some("How: high-level implementation, structure, and design"),
        "DA" => Some("Architecture decisions and tradeoffs"),
        "DF" => Some("Product behavior decisions and tradeoffs"),
        "E2E" => Some("Executable user scenarios"),
        "e2e" => Some("User scenarios: black-box proof of the spec"),
        "integration" => Some("Integration tests: proof that the parts fit as designed"),
        "RM" => Some("Planned milestones and sequencing"),
        _ => None,
    }
}

/// The release the deprecated `[[kinds]] prefix` spelling stops loading in
/// (§FS-config.3.4, §REQ-backwards-compatibility.2). Named in the warning, and
/// held ahead of the running version by a unit test, so the window cannot expire
/// unnoticed the way an undated deprecation does.
const KIND_PREFIX_KEY_REMOVAL_RELEASE: &str = "0.13.0";

/// One `[[kinds]]` entry as the parser has it so far: the entry itself, the line
/// its `[[kinds]]` header sat on (what an entry-level error anchors at), and
/// which key spelled its name — `kind`, or the deprecated `prefix`
/// (§FS-config.3.4), which the warning in §FS-config.4.1 names.
struct ParsedKind {
    config: KindConfig,
    header_line: usize,
    name_key: Option<(&'static str, usize)>,
}

impl ParsedKind {
    fn new(header_line: usize) -> Self {
        Self {
            config: KindConfig {
                kind: String::new(),
                folder: None,
                file: None,
                title: None,
                index: KindIndex::Default,
                citable: true,
            },
            header_line,
            name_key: None,
        }
    }
}

/// Read one `key = value` line inside a `[[kinds]]` block into `slot`
/// (§FS-config.3.4). Returns `false` for a key this section does not define, so
/// the caller reports it as an unknown config key (§FS-config.4.3).
fn parse_kinds_key(
    path: &Path,
    line_no: usize,
    key: &str,
    value: &str,
    current_kind: &mut Option<ParsedKind>,
) -> Result<bool> {
    match key {
        // §FS-config.3.4: `kind` is the name. `prefix` is the same key under the
        // name it carried before non-citable kinds made "prefix" wrong for half
        // the table, accepted through the deprecation window of
        // §REQ-backwards-compatibility.2.
        "kind" | "prefix" => {
            let name = parse_string(path, line_no, value)?;
            // §FS-config.3.2: a citable kind's name is the leading component of
            // every ID in it, so a `/` here lands in the ID as surely as one in
            // `slug_pattern` does.
            if let Some(message) =
                id_grammar_literal_slash_error(&format!("[[kinds]] {key} `{name}`"), &name)
            {
                bail_config(path, line_no, message)?;
            }
            let Some(slot) = current_kind.as_mut() else {
                bail_config(path, line_no, format!("`{key}` outside of [[kinds]] block"))?;
                unreachable!();
            };
            if let Some((seen, _)) = slot.name_key {
                let message = if seen == key {
                    format!("[[kinds]] sets `{key}` twice")
                } else {
                    "[[kinds]] sets both `kind` and `prefix`, which name the same thing (keep `kind`)"
                        .to_string()
                };
                bail_config(path, line_no, message)?;
            }
            slot.name_key = Some((if key == "kind" { "kind" } else { "prefix" }, line_no));
            slot.config.kind = name;
        }
        // §FS-config.3.4: `citable = false` makes the entry a place rather than
        // an ID namespace — scanned and directed, never declared in.
        "citable" => {
            let citable = parse_bool(path, line_no, value)?;
            if let Some(slot) = current_kind.as_mut() {
                slot.config.citable = citable;
            } else {
                bail_config(
                    path,
                    line_no,
                    "`citable` outside of [[kinds]] block".to_string(),
                )?;
            }
        }
        "folder" => {
            let folder = parse_string(path, line_no, value)?;
            if let Some(slot) = current_kind.as_mut() {
                slot.config.folder = Some(folder);
            } else {
                bail_config(
                    path,
                    line_no,
                    "`folder` outside of [[kinds]] block".to_string(),
                )?;
            }
        }
        "file" => {
            let file = parse_string(path, line_no, value)?;
            if let Some(slot) = current_kind.as_mut() {
                slot.config.file = Some(file);
            } else {
                bail_config(
                    path,
                    line_no,
                    "`file` outside of [[kinds]] block".to_string(),
                )?;
            }
        }
        // §FS-config.3.4: `index` names the file under `folder` that must list
        // the folder's declarations (§FS-check.4.6). A file name or `false`;
        // `true` names no file, so it is rejected rather than read as "the
        // default", which the key's own absence already spells.
        "index" => {
            let index = if value == "false" {
                KindIndex::Disabled
            } else if value == "true" {
                bail_config(
                    path,
                    line_no,
                    "[[kinds]] `index` takes a file name or `false` (omit the key for the default `README.md`)"
                        .to_string(),
                )?;
                unreachable!();
            } else {
                let name = parse_string(path, line_no, value)?;
                if let Some(message) = kind_index_name_error(&name) {
                    bail_config(path, line_no, message)?;
                }
                KindIndex::Named(name)
            };
            if let Some(slot) = current_kind.as_mut() {
                slot.config.index = index;
            } else {
                bail_config(
                    path,
                    line_no,
                    "`index` outside of [[kinds]] block".to_string(),
                )?;
            }
        }
        "title" => {
            let title = parse_string(path, line_no, value)?;
            if let Some(slot) = current_kind.as_mut() {
                slot.config.title = Some(title);
            } else {
                bail_config(
                    path,
                    line_no,
                    "`title` outside of [[kinds]] block".to_string(),
                )?;
            }
        }
        _ => return Ok(false),
    }
    Ok(true)
}

/// Why `index = "<name>"` is not a usable index file name, or `None`
/// (§FS-config.3.4). Two rules, each closing a state the rules built on this key
/// cannot describe:
///
/// * **It names a file inside `folder`.** The value is joined onto `folder`, and
///   an absolute path or one that climbs out with `..` silently replaces the
///   folder instead of naming a file in it — `grund check` would then read, and
///   `grund fmt --write` would then rewrite, a file outside the tree the config
///   describes (§FS-non-goals.11). `.` is rejected with them: it names the same
///   file by a path no message should have to print.
/// * **It names a Markdown file.** `--cross-refs` runs on `.md` files only
///   (§FS-fmt.6.1), so an index with any other extension is one the formatter can
///   never linkify — and §FS-check.3.17, whose whole licence is that
///   `grund fmt --write` fixes it, would be an error no command could clear
///   (§DF-index-entry-form.2.3).
fn kind_index_name_error(name: &str) -> Option<String> {
    if name.is_empty() {
        return Some("[[kinds]] `index` must name a file (use `false` to opt out)".to_string());
    }
    let inside_folder = !name.contains('\\')
        && Path::new(name)
            .components()
            .all(|component| matches!(component, Component::Normal(_)));
    if !inside_folder {
        return Some(format!(
            "[[kinds]] `index` must be a relative path inside `folder` (`{name}` is not)"
        ));
    }
    if Path::new(name).extension().and_then(|ext| ext.to_str()) != Some("md") {
        return Some(format!(
            "[[kinds]] `index` must name a Markdown file (`{name}` is not a `.md` file)"
        ));
    }
    None
}

/// Every whole-list rule a `[[kinds]]` block has to satisfy (§FS-config.3.4):
/// each entry names a kind, `folder` and `file` are exclusive, `index` needs a
/// folder and a citable kind, a non-citable kind needs a home, `code` is
/// reserved, names are unique, and no *citable* kind's name is a prefix of
/// another's. Applied only when the file declared the block at all — `[[kinds]]`
/// replaces the defaults entirely rather than merging into them.
///
/// It also resolves the per-name `index` defaults, so a declared kind and a
/// built-in one of the same name agree about what `index` is when the key is
/// absent (§FS-config.3.4), and records where the deprecated `prefix` spelling
/// was used, so the run can say so (§FS-config.4.1).
fn apply_parsed_kinds(path: &Path, parsed: Vec<ParsedKind>, config: &mut Config) -> Result<()> {
    // [[kinds]] replaces defaults entirely, per §FS-config.3.4.
    if let Some(nameless) = parsed.iter().find(|entry| entry.config.kind.is_empty()) {
        return Err(anyhow!(
            "{}:{}: every [[kinds]] entry must declare a `kind`",
            format_path(path),
            nameless.header_line
        ));
    }
    if parsed.is_empty() {
        return Err(anyhow!(
            "{}: at least one [[kinds]] entry must declare a `kind`",
            format_path(path)
        ));
    }
    for entry in &parsed {
        let k = &entry.config;
        // Reject kinds that set both `folder` and `file` — they're mutually
        // exclusive (§FS-config.3.4). A kind is either multi-file (folder) or
        // single-file (file); the schema models the "can always be broken up"
        // transition as swapping one key for the other, not setting both.
        if k.folder.is_some() && k.file.is_some() {
            return Err(anyhow!(
                "{}: kind `{}` sets both `folder` and `file` (use one)",
                format_path(path),
                k.kind
            ));
        }
        // §FS-config.3.4: `index` names a file inside `folder`, so a kind
        // with no folder — a single-file `file` kind, or one with no home at
        // all — has nothing to index and the key is a config error.
        if k.index != KindIndex::Default && k.folder.is_none() {
            return Err(anyhow!(
                "{}: kind `{}` sets `index` without `folder` (only a folder kind has an index)",
                format_path(path),
                k.kind
            ));
        }
        // §FS-config.3.4: an index lists the folder's declarations
        // (§FS-check.4.6), and a non-citable kind has none — so the key is not a
        // no-op here, it is a statement about a set that can never be non-empty.
        if k.index != KindIndex::Default && !k.citable {
            return Err(anyhow!(
                "{}: kind `{}` sets `index` and `citable = false` (a non-citable kind declares nothing to index)",
                format_path(path),
                k.kind
            ));
        }
        // §FS-config.3.9.2: `code` is the *default* name of the homeless kind,
        // and a name a project may take only by declaring that kind — the
        // complement of every home, which is what "no `folder`, no `file`,
        // `citable = false`" spells. Any other row wearing it would collide with
        // the fallback every citation outside a home resolves to.
        if k.kind == CODE_SOURCE_KIND && !(!k.citable && k.folder.is_none() && k.file.is_none()) {
            return Err(anyhow!(
                "{}: `{CODE_SOURCE_KIND}` names the homeless kind — a [[kinds]] entry may take it only with `citable = false` and no `folder` or `file` (§FS-config.3.9.2)",
                format_path(path)
            ));
        }
    }
    // §FS-config.3.4: the `index` default is keyed on the name, and this is
    // where a *declared* kind picks it up. `[[kinds]]` replaces the built-in list
    // rather than merging into it, so without this line the same block that
    // `grund init` writes would mean one thing when the file omits it and
    // another when the file spells it out — and every repository whose config
    // predates this key would inherit an obligation the built-in default
    // deliberately declines. Runs after the validation above, which reads
    // `index` as the file wrote it.
    let mut kinds: Vec<KindConfig> = parsed.iter().map(|entry| entry.config.clone()).collect();
    for kind in &mut kinds {
        if kind.index == KindIndex::Default && kind.folder.is_some() && kind.citable {
            kind.index = default_kind_index(&kind.kind);
        }
    }
    // §FS-config.3.9.2: the homeless kind is the complement of every configured
    // home, and a complement is one place. Two rows claiming it would leave the
    // fallback with no single answer, so the second is refused where it is
    // written rather than resolved by order.
    let homeless: Vec<&KindConfig> = kinds
        .iter()
        .filter(|kind| !kind.citable && kind.folder.is_none() && kind.file.is_none())
        .collect();
    if let [first, second, ..] = homeless.as_slice() {
        return Err(anyhow!(
            "{}: kinds `{}` and `{}` both declare the homeless kind (no `folder`, no `file`) — there is one complement of every home",
            format_path(path),
            first.kind,
            second.kind
        ));
    }
    // §FS-config.3.4: names are unique across the whole table — `[citations.*]`
    // and `grund list --kind` key on one, so two rows wearing one name is a
    // config with no answer to "which".
    for (i, a) in kinds.iter().enumerate() {
        if kinds[..i].iter().any(|b| b.kind == a.kind) {
            return Err(anyhow!(
                "{}: kind `{}` is declared twice",
                format_path(path),
                a.kind
            ));
        }
    }
    // Reject kinds whose name is itself a prefix of another kind's name
    // (§FS-config.3.4 — would make tokenization ambiguous). Scoped to *citable*
    // kinds: the rule exists because `DAT-foo` parses as either `DA` or `DAT`,
    // and a name that never appears in an ID never tokenizes, so it has no
    // prefix to be ambiguous with.
    for (i, a) in kinds.iter().enumerate() {
        for (j, b) in kinds.iter().enumerate() {
            if i != j
                && a.citable
                && b.citable
                && a.kind.len() <= b.kind.len()
                && b.kind.starts_with(a.kind.as_str())
            {
                return Err(anyhow!(
                    "{}: kinds `{}` and `{}` collide (one is a prefix of the other)",
                    format_path(path),
                    a.kind,
                    b.kind
                ));
            }
        }
    }
    // §FS-config.4.1 / §REQ-backwards-compatibility.2: the old spelling still
    // loads, and the run says where it is and when it stops working. Anchored at
    // the first entry that uses it — one warning per config, not one per row.
    config.deprecated_kind_prefix = parsed
        .iter()
        .filter_map(|entry| match entry.name_key {
            Some(("prefix", line)) => Some(line),
            _ => None,
        })
        .min()
        .map(|line| ConfigLocation {
            path: path.to_path_buf(),
            line,
        });
    config.kinds = kinds;
    Ok(())
}
