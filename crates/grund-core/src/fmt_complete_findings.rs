/// The one place a declaration set is proven complete (§FS-fmt.7.4).
///
/// A rewrite that expands a shorthand (§FS-fmt.2.4) or wraps a cross-reference
/// (§FS-fmt.6) resolves an ID against the project's declarations, so a set short
/// of the whole project can name the wrong declaration — and a wrong rewrite is
/// a well-formed citation of a real declaration that no later pass can see
/// (§REQ-no-wrong-citation.3). Completeness is therefore a precondition of the
/// consumer rather than a habit of each producer: `FmtRunOpts` takes the proof,
/// not the findings, so a call site that has a `Findings` at hand and skips the
/// check does not compile (issue #105).
///
/// That guarantee is why this is a real `mod` and not a newtype beside the rest.
/// `grund-core` is one flat module assembled by `include!()`, so a private field
/// written in any of those files is still constructible by every sibling file,
/// and the rule would be a convention wearing a type's clothes. Here the field
/// is reachable only from inside this module, which makes the two constructors
/// below the only ways a proof comes into being:
///
/// - `CompleteScan::of_tree_or_abort` — scan the whole project and refuse the
///   run when the scan met an error;
/// - `WorkspaceProject::complete_findings` — reuse a scan already made, after
///   checking the same field a fresh scan would have refused on.
///
/// Neither takes an error list from its caller. A constructor that did could be
/// satisfied with an empty one, which is the bypass again with more ceremony.
mod complete_findings {
    use super::{
        ApiScanError, Config, Findings, FmtScanAbort, Result, WorkspaceProject, api_scan_error,
        scan_tree,
    };

    /// A whole-project scan that met no unreadable path, owning its findings
    /// (§FS-fmt.7.4). `fmt_tree` holds one when it had to scan for itself.
    pub(crate) struct CompleteScan {
        findings: Findings,
    }

    impl CompleteScan {
        /// Scan the whole project for the rewrites that need every declaration —
        /// a cross-reference wrap (§FS-fmt.6.3) or a shorthand expansion
        /// (§FS-fmt.2.4) — and refuse the whole run when any path could not be
        /// read, rather than resolving against what was legible (§FS-fmt.3).
        ///
        /// The scan is unscoped and non-strict about the scope on purpose: the
        /// declaration a shorthand names routinely lives outside the files being
        /// rewritten, so a scope-narrow set is not the set this question is
        /// asked of (§FS-fmt.2.4).
        ///
        /// Why a refusal and not a partial result: this is the one command that
        /// edits files, and the errors it aborts on are the same ones every
        /// other command reports, made fatal up front and preserved for
        /// reporting (§FS-fmt.3).
        ///
        /// Every refusal line names itself. Both `fmt` failures exit `2`, but the
        /// partial-scan one means every readable file was rewritten and this one
        /// means nothing was, and `--write` reaches this path in the ordinary
        /// case rather than the exceptional one — it turns `--cross-refs` on by
        /// itself wherever the scope holds Markdown (§FS-fmt.6.6). Paths are
        /// rendered against the run's config, like every other path `fmt` prints.
        pub(crate) fn of_tree_or_abort(config: &Config, render: &Config) -> Result<Self> {
            let (findings, errors) = scan_tree(config, None, false)?;
            if !errors.is_empty() {
                return Err(FmtScanAbort {
                    scan_errors: errors
                        .into_iter()
                        .map(|(path, message)| api_scan_error(render, &path, &message))
                        .collect::<Vec<ApiScanError>>(),
                }
                .into());
            }
            Ok(Self { findings })
        }

        /// The proven set. Only this direction exists: a `&Findings` never
        /// becomes a proof, which is the whole of §FS-fmt.7.4.
        pub(crate) fn findings(&self) -> &Findings {
            &self.findings
        }
    }

    /// A borrowed declaration set carrying the proof that the scan which
    /// produced it met no error (§FS-fmt.7.4) — what `FmtRunOpts` accepts in
    /// place of a bare `&Findings`.
    #[derive(Clone, Copy)]
    pub(crate) struct CompleteFindings<'a> {
        findings: &'a Findings,
    }

    impl<'a> CompleteFindings<'a> {
        /// The proven set, in the one direction that is sound. See
        /// `CompleteScan::findings`.
        pub(crate) fn findings(self) -> &'a Findings {
            self.findings
        }
    }

    impl WorkspaceProject {
        /// This project's already-computed findings from `load_workspace_context`,
        /// reusable by `fmt_tree` only when the scan that produced them met no
        /// error (§FS-fmt.3, §FS-fmt.7.1). A caller that reuses a scan instead of
        /// running one has to answer the question a fresh scan would have failed
        /// on, not just borrow the `Findings` beside it — reuse is an
        /// optimization of one computation and never a second one.
        ///
        /// It lives here, beside the field it proves, because this is the only
        /// code that reads both halves of the pair: no call site is trusted to
        /// remember to check.
        pub(crate) fn complete_findings(&self) -> Option<CompleteFindings<'_>> {
            self.scan_errors
                .is_empty()
                .then_some(CompleteFindings {
                    findings: &self.findings,
                })
        }
    }
}

use complete_findings::{CompleteFindings, CompleteScan};
