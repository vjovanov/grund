"""§AR-goal-measurement.1 says architecture pages own harness shape, and a
harness described by a path that is not there is not described: every
backticked repository path an architecture page names — a crate file, a
script, a test, a template, a workflow — exists on disk. Illustrative paths
belong in fenced blocks, which this test does not read."""

import re
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
ARCHITECTURE = REPO_ROOT / "docs" / "architecture"
PATH_TOKEN = re.compile(
    r"`((?:crates|scripts|tests|docs|templates|skills|editor|examples|\.github|\.agents)/[^`\s]+)`"
)
FENCE = re.compile(r"^(?: {0,3})(`{3,}|~{3,}).*?^(?: {0,3})\1[ \t]*$", re.M | re.S)


def _named_paths(text):
    for token in PATH_TOKEN.findall(FENCE.sub("", text)):
        path = token.split("#", 1)[0].rstrip("/")
        if any(glob in path for glob in "*{}<>") or path.endswith((".", ",")):
            continue
        yield token, path


class ArchitecturePagePathTests(unittest.TestCase):
    def test_every_named_repository_path_exists(self):
        missing, seen = [], 0
        for page in sorted(ARCHITECTURE.glob("*.md")):
            for token, path in _named_paths(page.read_text(encoding="utf-8")):
                seen += 1
                if not (REPO_ROOT / path).exists():
                    missing.append(f"{page.name}: `{token}`")
        self.assertGreaterEqual(seen, 30, "the pages name fewer paths than expected; parser broken?")
        self.assertEqual([], missing, "\n".join(missing))


if __name__ == "__main__":
    unittest.main()
