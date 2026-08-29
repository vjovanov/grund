/// A whole-declaration-set rewrite refused because its completed scan found
/// unreadable paths (§FS-fmt.3). Kept structured so API callers can inspect the
/// complete set and each CLI adapter can add its own `error:` prefix.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FmtScanAbort {
    pub scan_errors: Vec<ApiScanError>,
}

impl std::fmt::Display for FmtScanAbort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.scan_errors.is_empty() {
            return f.write_str("nothing was rewritten");
        }
        for (index, error) in self.scan_errors.iter().enumerate() {
            if index > 0 {
                f.write_str("\n")?;
            }
            write!(
                f,
                "nothing was rewritten: {}: {}",
                error.path, error.message
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for FmtScanAbort {}
