"""§AR-workspace.9 — the test-contracts table names things that exist: every
`tests/e2e/cases/<case>` cell is a case directory and every backticked test name
is a `fn` in the file the cell names; and every e2e case any architecture page
mentions is on disk. A contract that names a test nobody can run is a wish, and a
page that points at a case that moved is a broken map."""

import re
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
ARCHITECTURE = REPO_ROOT / "docs" / "architecture"
CASES = REPO_ROOT / "tests" / "e2e" / "cases"
TABLE_PAGE = ARCHITECTURE / "AR-workspace.md"
TABLE_HEADING = "## 9. Test contracts"
CASE_RE = re.compile(r"`(tests/e2e/cases/[A-Za-z0-9._-]+)`")
TOKEN_RE = re.compile(r"`([^`]+)`")
NAME_RE = re.compile(r"[a-z][a-z0-9_]*")


def _table_cells(text):
    section = text.split(TABLE_HEADING, 1)[1].split("\n## ", 1)[0]
    rows = [line for line in section.splitlines() if line.startswith("|")]
    cells = []
    for row in rows[2:]:  # header and rule
        columns = [column.strip() for column in row.strip("|").split("|")]
        cells.append((columns[0], columns[-1]))
    return cells


def _test_files():
    files = list((REPO_ROOT / "crates").glob("*/src/tests*.rs"))
    files += list((REPO_ROOT / "crates").glob("*/tests/**/*.rs"))
    files += list((REPO_ROOT / "tests" / "integration").glob("*.rs"))
    return files


class CoverageTableTests(unittest.TestCase):
    def test_every_contract_names_a_test_or_fixture_that_exists(self):
        cells = _table_cells(TABLE_PAGE.read_text(encoding="utf-8"))
        self.assertGreaterEqual(len(cells), 40, "the test-contracts table was not found")
        all_tests = {path: path.read_text(encoding="utf-8") for path in _test_files()}
        problems = []
        for invariant, cell in cells:
            for item in cell.split(";"):
                tokens = TOKEN_RE.findall(item)
                files = [token for token in tokens if token.endswith(".rs")]
                cases = [token for token in tokens if token.startswith("tests/e2e/cases/")]
                names = [
                    token
                    for token in tokens
                    if NAME_RE.fullmatch(token) and token not in files and token not in cases
                ]
                for case in cases:
                    if not (REPO_ROOT / case).is_dir():
                        problems.append(f"{invariant!r}: no case directory {case}")
                sources = {REPO_ROOT / file: all_tests.get(REPO_ROOT / file) for file in files}
                for file, text in sources.items():
                    if text is None:
                        problems.append(f"{invariant!r}: no test file {file.relative_to(REPO_ROOT)}")
                haystack = [text for text in sources.values() if text] or list(all_tests.values())
                for name in names:
                    if not any(re.search(rf"\bfn {re.escape(name)}\b", text) for text in haystack):
                        where = ", ".join(files) or "any test file"
                        problems.append(f"{invariant!r}: no `fn {name}` in {where}")
        self.assertEqual([], problems, "\n".join(problems))

    def test_every_case_an_architecture_page_mentions_is_on_disk(self):
        missing = []
        for page in sorted(ARCHITECTURE.glob("*.md")):
            for case in CASE_RE.findall(page.read_text(encoding="utf-8")):
                if not (REPO_ROOT / case).is_dir():
                    missing.append(f"{page.name}: {case}")
        self.assertEqual([], missing, "\n".join(missing))

    def test_the_table_is_where_the_page_says_it_is(self):
        self.assertIn(TABLE_HEADING, TABLE_PAGE.read_text(encoding="utf-8"))
        self.assertTrue(CASES.is_dir())


if __name__ == "__main__":
    unittest.main()
