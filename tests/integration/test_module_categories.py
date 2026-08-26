"""§AR-core-module-layout.1 — the source layout matches the category
boundaries the page names: every implementation file under
`crates/grund-core/src/` belongs to exactly one category by its file-name
prefix, every prefix the page lists owns at least one file, and `lib.rs` is
the one file outside the categories, as the crate entrypoint."""

import re
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
PAGE = REPO_ROOT / "docs" / "architecture" / "AR-core-module-layout.md"
CORE = REPO_ROOT / "crates" / "grund-core" / "src"
ROW = re.compile(r"^\|\s*\*\*([a-z]+)\*\*\s*\|\s*(.*?)\s*\|\s*$")
PREFIX = re.compile(r"`([a-z][a-z0-9_]*)`")


def _categories():
    table = {}
    for line in PAGE.read_text(encoding="utf-8").splitlines():
        match = ROW.match(line)
        if match:
            table[match.group(1)] = PREFIX.findall(match.group(2))
    return table


def _implementation_stems():
    return sorted(
        path.stem
        for path in CORE.glob("*.rs")
        if not path.name.startswith("tests") and path.name != "lib.rs"
    )


def _category_of(stem, table):
    owners = {
        category
        for category, prefixes in table.items()
        if any(stem == prefix or stem.startswith(prefix + "_") for prefix in prefixes)
    }
    return owners


class ModuleCategoryTests(unittest.TestCase):
    def test_the_page_carries_the_category_table(self):
        self.assertGreaterEqual(len(_categories()), 14, "category table not found on the page")

    def test_every_implementation_file_has_exactly_one_category(self):
        table = _categories()
        problems = []
        for stem in _implementation_stems():
            owners = _category_of(stem, table)
            if len(owners) != 1:
                problems.append(f"{stem}.rs -> {sorted(owners) or 'no category'}")
        self.assertEqual([], problems, "\n".join(problems))

    def test_every_listed_prefix_owns_a_file(self):
        stems = _implementation_stems()
        stale = []
        for category, prefixes in _categories().items():
            for prefix in prefixes:
                if not any(stem == prefix or stem.startswith(prefix + "_") for stem in stems):
                    stale.append(f"{category}: `{prefix}`")
        self.assertEqual([], stale, "\n".join(stale))

    def test_lib_rs_is_the_entrypoint_outside_the_categories(self):
        self.assertTrue((CORE / "lib.rs").is_file())
        self.assertNotIn("lib", {p for ps in _categories().values() for p in ps})


if __name__ == "__main__":
    unittest.main()
