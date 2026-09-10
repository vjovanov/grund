/// Focused scanner boundaries for unmarked Markdown ATX headings
/// (§FS-check.4.14).
#[cfg(test)]
mod tests_unmarked_headings {
    use super::markdown_heading_level;

    #[test]
    fn atx_recognition_accepts_only_markdown_indentation_hash_and_separator_bounds() {
        for (line, expected) in [
            ("# One hash", Some(1)),
            ("###### Six hashes", Some(6)),
            ("#", Some(1)),
            ("######", Some(6)),
            ("##\tASCII tab separator", Some(2)),
            ("## Zero spaces", Some(2)),
            (" ## One space", Some(2)),
            ("  ## Two spaces", Some(2)),
            ("   ## Three spaces", Some(2)),
            ("    ## Four spaces is code", None),
            ("\t## Leading tab is code", None),
            ("####### Seven hashes", None),
            ("######## Eight hashes", None),
            ("##No separator", None),
            ("##\u{a0}Non-ASCII whitespace", None),
        ] {
            assert_eq!(markdown_heading_level(line), expected, "line: {line:?}");
        }
    }
}
