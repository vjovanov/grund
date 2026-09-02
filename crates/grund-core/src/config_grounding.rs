/// The grounding half of the config (§FS-config.3.4.8), beside `config_kinds.rs`
/// (§AR-core-module-layout.1): the two keys that say *whether* a place's files
/// must cite a declared ID and *how finely* that is asked, their `[reference]`
/// defaults, the rules that reject a combination they cannot describe, and the
/// per-row resolution every reader of them goes through.
///
/// They live here rather than with the rest of `[[kinds]]` because they are one
/// contract spanning two sections: each key is written either on a row or in
/// `[reference]` as the default for every row, and a validation rule reaches
/// across both (`[reference] grounding_level` is an error when no row turns
/// grounding on). Keeping the pair in one file is what stops that rule from
/// being written twice.

/// Where a `[[kinds]]` row wrote each grounding key. The *values* live on the
/// row's `KindConfig`, where every reader wants them; only the lines are held
/// aside, so a rejection anchors at the offending key rather than at the block
/// header (§FS-config.4.3).
#[derive(Default)]
struct ParsedGrounding {
    require_line: Option<usize>,
    level_line: Option<usize>,
}

/// Read one `[[kinds]]` grounding key into the row being parsed
/// (§FS-config.3.4.8). Both keys are booleans-and-integers with no defaulting of
/// their own: an absent key stays `None` and inherits `[reference]` later.
fn parse_kind_grounding_key(
    path: &Path,
    line_no: usize,
    key: &str,
    value: &str,
    current_kind: &mut Option<ParsedKind>,
) -> Result<()> {
    let Some(slot) = current_kind.as_mut() else {
        bail_config(path, line_no, format!("`{key}` outside of [[kinds]] block"))?;
        unreachable!();
    };
    if key == "require_grounding" {
        slot.config.require_grounding = Some(parse_bool(path, line_no, value)?);
        slot.grounding.require_line = Some(line_no);
    } else {
        let level = parse_usize(path, line_no, value)?;
        check_grounding_level(path, line_no, level)?;
        slot.config.grounding_level = Some(level);
        slot.grounding.level_line = Some(line_no);
    }
    Ok(())
}

/// §FS-config.3.4.8: a level outside `1..=6` names no heading Markdown can have,
/// wherever it is written.
fn check_grounding_level(path: &Path, line_no: usize, level: usize) -> Result<()> {
    if !GROUNDING_LEVELS.contains(&level) {
        bail_config(
            path,
            line_no,
            format!(
                "`grounding_level` must be a Markdown heading level {}..{} (`{level}` is not)",
                GROUNDING_LEVELS.start(),
                GROUNDING_LEVELS.end()
            ),
        )?;
    }
    Ok(())
}

/// Every rule the two row keys have to satisfy (§FS-config.3.4.8), each closing a
/// state they cannot describe. Run over the parsed entries once the rest of the
/// `[[kinds]]` validation has passed, so a row that is already malformed is
/// reported as that rather than as a grounding error.
fn validate_kind_grounding(path: &Path, parsed: &[ParsedKind]) -> Result<()> {
    for entry in parsed {
        let kind = &entry.config;
        // §FS-config.3.4.7: nothing in an unwalked home is read, so the rule
        // could never fire — the reasoning that already refuses a
        // `[citations.<kind>]` rule on an unwalked citing kind.
        if kind.require_grounding == Some(true)
            && !kind.scan
            && let Some(line) = entry.grounding.require_line
        {
            bail_config(
                path,
                line,
                format!(
                    "kind `{}` sets `require_grounding = true` and `scan = false` (no file in an unwalked home is read, so the rule could never fire)",
                    kind.kind
                ),
            )?;
        }
        // §FS-config.3.4.8: a single-file kind is one Markdown document and
        // §FS-check.3.6 never reaches it, so neither key has anything to mean —
        // as `index` has nothing to mean on a file kind (§FS-config.3.4.2).
        if kind.file.is_some()
            && let Some((key, line)) = grounding_key_site(&entry.grounding)
        {
            bail_config(
                path,
                line,
                format!(
                    "kind `{}` sets `{key}` with `file` (a single-file kind is one document, which the grounding rule never reaches)",
                    kind.kind
                ),
            )?;
        }
        // §FS-config.3.4.8: a level beside an explicit `false` on the same row is
        // a unit for a rule this row just switched off.
        if kind.require_grounding == Some(false)
            && let Some(line) = entry.grounding.level_line
        {
            bail_config(
                path,
                line,
                format!(
                    "kind `{}` sets `grounding_level` and `require_grounding = false` (the level could never fire)",
                    kind.kind
                ),
            )?;
        }
    }
    Ok(())
}

/// Which of the two keys a row wrote first, for the `file =` rejection above —
/// by line, so the message names the key the reader can go and delete.
fn grounding_key_site(grounding: &ParsedGrounding) -> Option<(&'static str, usize)> {
    let require = grounding.require_line.map(|line| ("require_grounding", line));
    let level = grounding.level_line.map(|line| ("grounding_level", line));
    match (require, level) {
        (Some(a), Some(b)) if b.1 < a.1 => Some(b),
        (Some(a), _) => Some(a),
        (None, other) => other,
    }
}

/// §FS-config.3.4.8: `[reference] grounding_level` with the global boolean off
/// and no row turning grounding on is a unit for a rule nothing switched on —
/// the row-scoped rejection above, one scope up. Run after `[[kinds]]` is final,
/// because "no row turns it on" is a question about the resolved table.
fn validate_global_grounding(path: &Path, config: &Config, line: Option<usize>) -> Result<()> {
    let Some(line) = line else {
        return Ok(());
    };
    if config.require_grounding
        || config
            .kinds
            .iter()
            .any(|kind| kind.require_grounding == Some(true))
    {
        return Ok(());
    }
    bail_config(
        path,
        line,
        "[reference] `grounding_level` is set and nothing turns grounding on (set `require_grounding` here or on a [[kinds]] row)".to_string(),
    )
}

impl Config {
    /// The effective grounding pair for one `[[kinds]]` row (§FS-config.3.4.8):
    /// the row's word where it has one, else the `[reference]` default — which is
    /// also what `grund check --require-grounding` sets, so an explicit row
    /// `false` wins over the flag.
    fn kind_grounding(&self, kind: &KindConfig) -> (bool, usize) {
        (
            kind.require_grounding.unwrap_or(self.require_grounding),
            kind.grounding_level.unwrap_or(self.grounding_level),
        )
    }

    /// The effective pair for the homeless kind (§FS-config.3.9.2) — its declared
    /// row when the table has one, else the `[reference]` defaults, since an
    /// undeclared complement has no row to write them on.
    fn homeless_grounding(&self) -> (bool, usize) {
        declared_homeless_kind(&self.kinds)
            .map(|kind| self.kind_grounding(kind))
            .unwrap_or((self.require_grounding, self.grounding_level))
    }

    /// Whether any place is grounded at all — the early out that keeps the whole
    /// pass off a tree that asked for none (§FS-check.3.6).
    pub fn grounding_enabled(&self) -> bool {
        self.require_grounding
            || self
                .kinds
                .iter()
                .any(|kind| kind.require_grounding == Some(true))
    }

    /// The `[[kinds]]` grounding lines `grund config show` prints for one row
    /// (§FS-config.4.2): each key only where the row's effective value differs
    /// from the effective global, which is printed under `[reference]`. A row
    /// that inherits both prints neither, and the shown config loads back to the
    /// same effective values — printing a key on every row would be noise, and
    /// printing a level under a row that turned grounding off would not load at
    /// all (§FS-config.3.4.8).
    pub fn kind_grounding_toml_lines(&self, kind: &KindConfig) -> Vec<String> {
        let (require, level) = self.kind_grounding(kind);
        let mut lines = Vec::new();
        if require != self.require_grounding {
            lines.push(format!("require_grounding = {require}"));
        }
        if level != self.grounding_level {
            lines.push(format!("grounding_level = {level}"));
        }
        lines
    }

    /// Whether any place asks for a unit finer than the file, which is what the
    /// scanner records per-file structure for (§AR-scanner.2.7). Derived once, on
    /// load, so the question is a field read per file rather than a walk of the
    /// kind table.
    fn recompute_grounding_units(&mut self) {
        let homeless = self.homeless_grounding().1;
        self.grounding_units = homeless > DEFAULT_GROUNDING_LEVEL
            || self
                .kinds
                .iter()
                .any(|kind| self.kind_grounding(kind).1 > DEFAULT_GROUNDING_LEVEL);
    }
}
