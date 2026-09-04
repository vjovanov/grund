#!/usr/bin/env bash
# §FS-fmt.7.4 — the acceptance check for the one rule of §FS-fmt.7 that is not a
# behavior: the code that consumes `fmt`'s declaration set must accept only a set
# carrying the proof that the scan producing it met no error, so handing it a
# project's raw findings is a program that does not build.
#
# There is nothing to run for that, only something to compile. So this script
# writes the mistake issue #105 actually was — the workspace formatter passing
# `Some(&project.findings)` straight through, skipping the completeness check —
# compiles the crate, restores the tree, and fails when the compile succeeded.
#
# It is deliberately not a pre-commit hook: it edits tracked source, and a step
# that can leave the working tree modified when interrupted is the wrong thing to
# run before every commit (§DF-fmt-one-model.2.6). Run it by hand, and in review.
#
# Exit 0 — the bypass did not compile: the rule holds.
# Exit 1 — the bypass compiled: the invariant is a convention again.
# Exit 2 — the check could not be run, and says why.

set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
target="$repo_root/crates/grund-core/src/fmt_workspace.rs"
backup_dir="$repo_root/target/check-fmt-complete-findings"
backup="$backup_dir/fmt_workspace.rs.orig"
bypass='precomputed_findings: Some(&project.findings),'

if [ ! -f "$target" ]; then
  echo "error: $target does not exist; the consumer this rule is about has moved" >&2
  exit 2
fi

if ! grep -q 'precomputed_findings:' "$target"; then
  echo "error: no \`precomputed_findings:\` line in $target — the anchor this check" >&2
  echo "       substitutes into has been renamed. Re-point the script at the field" >&2
  echo "       the declaration set is now passed through, rather than deleting it." >&2
  exit 2
fi

mkdir -p "$backup_dir"
cp "$target" "$backup"
restore() { cp "$backup" "$target"; }
trap restore EXIT INT TERM

# The whole substitution: whatever guarded the reuse, hand the consumer the
# project's findings with nothing checked about the scan that produced them.
perl -pi -e 's{^(\s*)precomputed_findings: .*$}{$1'"$bypass"'}' "$target"

if ! grep -qF "$bypass" "$target"; then
  echo "error: the bypass was not written into $target; nothing was compiled" >&2
  exit 2
fi

echo "checking that the #105 bypass does not compile:"
echo "  $bypass"
if cargo check --quiet --locked -p grund-core 2>&1 | sed 's/^/  /'; then
  echo
  echo "FAIL: \`cargo check -p grund-core\` accepted the bypass." >&2
  echo "      §FS-fmt.7.4 asks for a declaration set that carries its own proof of" >&2
  echo "      completeness; a consumer taking a bare findings collection leaves the" >&2
  echo "      invariant as a guard each new call site re-decides, which is what" >&2
  echo "      issue #105 was." >&2
  exit 1
fi

echo
echo "ok: the bypass does not compile — §FS-fmt.7.4 holds."
