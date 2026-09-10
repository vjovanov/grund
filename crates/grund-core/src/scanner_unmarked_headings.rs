/// Retain only Markdown headings owned by a declaration body, choose the
/// nearest nested owner, and assign deterministic unused section paths
/// (§AR-scanner.2.2, §AR-scanner.2.4, §FS-check.4.14).
fn assign_unmarked_heading_owners(
    findings: &mut Findings,
    mut candidates: Vec<UnmarkedHeadingCandidate>,
    md_headings: &[(usize, usize)],
    total_lines: usize,
) {
    #[derive(Clone)]
    struct KnownPath {
        line: usize,
        path: String,
    }

    let bodies = findings
        .declarations
        .values()
        .flatten()
        .map(|decl| {
            (
                decl.line,
                markdown_declaration_body_end(decl, md_headings, total_lines),
                decl.heading_level,
                decl.id.clone(),
                decl.sections
                    .iter()
                    .map(|(path, info)| KnownPath {
                        line: info.line,
                        path: path.clone(),
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|candidate| candidate.line);
    let mut suggested_by_owner: BTreeMap<Id, Vec<KnownPath>> = BTreeMap::new();

    for candidate in candidates {
        let Some((_, _, declaration_level, owner, existing)) = bodies
            .iter()
            .filter(|(start, end, level, _, _)| {
                *start <= candidate.line
                    && candidate.line <= *end
                    && candidate.heading_level > *level
            })
            .max_by_key(|(start, _, _, _, _)| *start)
        else {
            continue;
        };
        let target_depth = candidate.heading_level - declaration_level;
        let prior_suggestions = suggested_by_owner.entry(owner.clone()).or_default();
        let mut paths = existing.clone();
        paths.extend(prior_suggestions.iter().cloned());
        let suggested_path = suggested_section_path(&paths, candidate.line, target_depth);
        prior_suggestions.push(KnownPath {
            line: candidate.line,
            path: suggested_path.clone(),
        });
        findings.unmarked_headings.push(UnmarkedHeading {
            file: candidate.file,
            line: candidate.line,
            column: candidate.column,
            heading: candidate.heading,
            heading_level: candidate.heading_level,
            title: candidate.title,
            owner: owner.clone(),
            suggested_path,
        });
    }

    fn suggested_section_path(paths: &[KnownPath], line: usize, target_depth: usize) -> String {
        let parent = paths
            .iter()
            .filter(|known| known.line < line && path_depth(&known.path) < target_depth)
            .max_by_key(|known| known.line)
            .map(|known| known.path.clone());

        let mut parts = parent
            .as_deref()
            .map(|path| path.split('.').map(str::to_string).collect::<Vec<_>>())
            .unwrap_or_default();
        if parts.is_empty() {
            parts.push(next_numeric_child(paths, &[]));
        }
        while parts.len() + 1 < target_depth {
            parts.push("1".to_string());
        }
        if parts.len() < target_depth {
            let next = next_numeric_child(paths, &parts);
            parts.push(next);
        }
        parts.join(".")
    }

    fn path_depth(path: &str) -> usize {
        path.split('.').count()
    }

    fn next_numeric_child(paths: &[KnownPath], parent: &[String]) -> String {
        paths
            .iter()
            .filter_map(|known| {
                let parts = known.path.split('.').collect::<Vec<_>>();
                if parts.len() != parent.len() + 1
                    || !parts[..parent.len()]
                        .iter()
                        .zip(parent)
                        .all(|(left, right)| *left == right)
                {
                    return None;
                }
                let child = *parts.last()?;
                child.bytes().all(|byte| byte.is_ascii_digit()).then_some(child)
            })
            .max_by(|left, right| compare_decimal(left, right))
            .map(increment_decimal)
            .unwrap_or_else(|| "1".to_string())
    }

    // Section coordinates accept an unbounded digit run. Compare and increment
    // those runs as decimal text so repair guidance stays above every authored
    // sibling without imposing an integer-width ceiling (§FS-check.4.14).
    fn compare_decimal(left: &str, right: &str) -> std::cmp::Ordering {
        let left = normalize_decimal(left);
        let right = normalize_decimal(right);
        left.len()
            .cmp(&right.len())
            .then_with(|| left.cmp(right))
    }

    fn normalize_decimal(value: &str) -> &str {
        let normalized = value.trim_start_matches('0');
        if normalized.is_empty() { "0" } else { normalized }
    }

    fn increment_decimal(value: &str) -> String {
        let normalized = normalize_decimal(value);
        let mut digits = normalized.as_bytes().to_vec();
        for digit in digits.iter_mut().rev() {
            if *digit == b'9' {
                *digit = b'0';
            } else {
                *digit += 1;
                return String::from_utf8(digits).expect("decimal digits stay UTF-8");
            }
        }
        format!("1{}", "0".repeat(digits.len()))
    }
}

/// Compute an unmarked-heading ownership boundary without publishing it on the
/// declaration model. Read-only `show` and `list --size` scans intentionally
/// keep their lazy whole-declaration slicing (§FS-check.4.14, §FS-show.2.1.3).
fn markdown_declaration_body_end(
    decl: &Declaration,
    md_headings: &[(usize, usize)],
    total_lines: usize,
) -> usize {
    if decl.is_stub {
        return decl.line;
    }
    md_headings
        .iter()
        .filter(|(line, level)| *line > decl.line && *level <= decl.heading_level)
        .map(|(line, _)| line - 1)
        .min()
        .unwrap_or(total_lines)
        .max(decl.line)
}
