"""§AR-ci.7 — the pull-request changelog gate, exercised the way CI runs it: a
pull request must be named in the `## Unreleased` section of `docs/changelog.md`
(§FS-distribution.4), and the script reads the PR number the way the event
supplies it."""

import importlib.util
import json
import tempfile
import unittest
from subprocess import CompletedProcess
from unittest.mock import patch
from pathlib import Path

SCRIPT_PATH = Path(__file__).resolve().parents[2] / "scripts" / "check_changelog_pr_entry.py"
SPEC = importlib.util.spec_from_file_location("check_changelog_pr_entry", SCRIPT_PATH)
check_changelog_pr_entry = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(check_changelog_pr_entry)


class CheckChangelogPrEntryTests(unittest.TestCase):
    def write_changelog(self, root: Path, unreleased: str) -> Path:
        changelog = root / "docs" / "changelog.md"
        changelog.parent.mkdir(parents=True)
        changelog.write_text(
            f"# Changelog\n\n## Unreleased\n\n{unreleased}\n\n"
            "## 2. [0.3.0] — 2026-05-18\n\nPrevious release.\n",
            encoding="utf-8",
        )
        return changelog

    def test_accepts_pr_number_in_unreleased_entry(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            changelog = self.write_changelog(
                Path(tmp),
                "### Changed\n\n- §FS-distribution.4: add the changelog PR gate. PR #15",
            )
            check_changelog_pr_entry.check_changelog_pr_entry(changelog, 15)

    def test_accepts_pull_url_in_unreleased_entry(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            changelog = self.write_changelog(
                Path(tmp),
                "### Fixed\n\n- §FS-distribution.4: fix release notes (https://github.com/vjovanov/grund/pull/15).",
            )
            check_changelog_pr_entry.check_changelog_pr_entry(changelog, 15)

    def test_rejects_missing_pr_number(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            changelog = self.write_changelog(
                Path(tmp),
                "### Changed\n\n- §FS-distribution.4: add the changelog PR gate.",
            )
            with self.assertRaisesRegex(check_changelog_pr_entry.ChangelogPrError, "PR #15"):
                check_changelog_pr_entry.check_changelog_pr_entry(changelog, 15)

    def test_reads_pull_request_number_from_event_file(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            event_path = root / "event.json"
            event_path.write_text(json.dumps({"pull_request": {"number": 15}}), encoding="utf-8")
            self.assertEqual(check_changelog_pr_entry.pr_number_from_event(event_path), 15)

    def test_non_pull_request_event_skips(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            event_path = root / "event.json"
            event_path.write_text(json.dumps({"ref": "refs/heads/main"}), encoding="utf-8")
            self.assertIsNone(check_changelog_pr_entry.pr_number_from_event(event_path))

    def test_reads_pull_request_number_from_gh_current_branch(self) -> None:
        with patch.object(
            check_changelog_pr_entry.subprocess,
            "run",
            return_value=CompletedProcess(args=[], returncode=0, stdout="18\n", stderr=""),
        ):
            self.assertEqual(check_changelog_pr_entry.pr_number_from_current_branch(), 18)

    def test_missing_gh_current_branch_pr_skips(self) -> None:
        with patch.object(
            check_changelog_pr_entry.subprocess,
            "run",
            return_value=CompletedProcess(args=[], returncode=1, stdout="", stderr="no pull requests found"),
        ):
            self.assertIsNone(check_changelog_pr_entry.pr_number_from_current_branch())


if __name__ == "__main__":
    unittest.main()
