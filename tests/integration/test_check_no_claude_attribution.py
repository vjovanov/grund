"""§AR-ci.8 — the attribution gate in each of its modes: no assistant
attribution boilerplate lands in this repository's files or commit messages,
whether the script is handed files, a message file, or a commit range."""

import importlib.util
import subprocess
import tempfile
import unittest
from pathlib import Path

SCRIPT_PATH = Path(__file__).resolve().parents[2] / "scripts" / "check_no_claude_attribution.py"
SPEC = importlib.util.spec_from_file_location("check_no_claude_attribution", SCRIPT_PATH)
check_no_claude_attribution = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(check_no_claude_attribution)

# Assembled at runtime so this test file does not itself contain the literals the
# gate rejects — the staged-file hook scans it like any other tracked file.
CO_AUTHOR = "Co-Authored-By:" + " Claude Fable 5 <noreply@" + "anthropic.com>"
SESSION = "Claude-Session:" + " https://claude.ai/code/session_01ABC"
GENERATED = "\N{ROBOT FACE} Generated with " + "[Claude Code](https://claude.com/claude-code)"


class FindAttributionTests(unittest.TestCase):
    def test_flags_the_co_author_trailer(self) -> None:
        findings = check_no_claude_attribution.find_attribution(f"subject\n\n{CO_AUTHOR}\n", "m")
        self.assertEqual([f.label for f in findings], ["co-author trailer"])
        self.assertEqual(findings[0].line_number, 3)

    def test_flags_session_and_generated_markers(self) -> None:
        for text, label in ((SESSION, "session trailer"), (GENERATED, "generated-with marker")):
            with self.subTest(label=label):
                findings = check_no_claude_attribution.find_attribution(text, "m")
                self.assertEqual([f.label for f in findings], [label])

    def test_reports_each_offending_line_once(self) -> None:
        findings = check_no_claude_attribution.find_attribution(f"{CO_AUTHOR}\n{SESSION}\n", "m")
        self.assertEqual(len(findings), 2)

    def test_allows_prose_that_mentions_claude(self) -> None:
        prose = "Claude Code cannot open the citation, so the fallback is plain text.\n"
        self.assertEqual(check_no_claude_attribution.find_attribution(prose, "m"), [])

    def test_allows_a_human_co_author_trailer(self) -> None:
        trailer = "Co-Authored-By:" + " Ada Lovelace <ada@example.com>\n"
        self.assertEqual(check_no_claude_attribution.find_attribution(trailer, "m"), [])


class ScanFilesTests(unittest.TestCase):
    def test_skips_a_path_staged_for_deletion(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            missing = Path(tmp) / "gone.md"
            self.assertEqual(check_no_claude_attribution.scan_files([missing]), [])

    def test_flags_file_content(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "notes.md"
            path.write_text(f"notes\n{CO_AUTHOR}\n", encoding="utf-8")
            findings = check_no_claude_attribution.scan_files([path])
            self.assertEqual([f.label for f in findings], ["co-author trailer"])


class ScanCommitsTests(unittest.TestCase):
    def make_repo(self, tmp: str) -> Path:
        root = Path(tmp)
        for command in (
            ["git", "init", "--quiet", "--initial-branch", "main"],
            ["git", "config", "user.email", "test@example.com"],
            ["git", "config", "user.name", "Test"],
        ):
            subprocess.run(command, cwd=root, check=True, capture_output=True)
        return root

    def commit(self, root: Path, message: str) -> str:
        (root / "file.txt").write_text(message, encoding="utf-8")
        subprocess.run(["git", "add", "-A"], cwd=root, check=True, capture_output=True)
        subprocess.run(
            ["git", "commit", "--quiet", "--no-verify", "-m", message],
            cwd=root,
            check=True,
            capture_output=True,
        )
        result = subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=root, check=True, capture_output=True, text=True
        )
        return result.stdout.strip()

    def test_scans_only_the_requested_range(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = self.make_repo(tmp)
            import os

            cwd = os.getcwd()
            os.chdir(root)
            try:
                base = self.commit(root, f"first\n\n{CO_AUTHOR}")
                self.commit(root, "second, clean")
                # The offending commit is the range base, so it is excluded.
                self.assertEqual(check_no_claude_attribution.scan_commits([f"{base}..HEAD"]), [])
                # ...and reachable when the range includes it.
                findings = check_no_claude_attribution.scan_commits(["HEAD"])
                self.assertEqual([f.label for f in findings], ["co-author trailer"])
            finally:
                os.chdir(cwd)

    def test_reports_how_many_messages_it_read(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = self.make_repo(tmp)
            import io
            import os
            from contextlib import redirect_stdout

            cwd = os.getcwd()
            os.chdir(root)
            try:
                self.commit(root, "first")
                self.commit(root, "second")
                out = io.StringIO()
                with redirect_stdout(out):
                    check_no_claude_attribution.scan_commits(["HEAD"])
                self.assertIn("scanned 2 commit message(s)", out.getvalue())
            finally:
                os.chdir(cwd)

    def test_unreachable_base_narrows_to_the_tip(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = self.make_repo(tmp)
            import os

            cwd = os.getcwd()
            os.chdir(root)
            try:
                self.commit(root, "only commit")
                self.assertEqual(check_no_claude_attribution.resolve_range("0" * 40 + "..HEAD"), ["-1", "HEAD"])
                self.assertEqual(check_no_claude_attribution.resolve_range("f" * 40 + "..HEAD"), ["-1", "HEAD"])
            finally:
                os.chdir(cwd)


if __name__ == "__main__":
    unittest.main()
