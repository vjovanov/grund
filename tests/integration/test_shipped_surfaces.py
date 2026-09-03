"""§REQ-shipped-surfaces.3 — no ID declared in this repository reaches a byte
that leaves it: not an asset the binary embeds, not its copy in the tree, not an
end-to-end output golden, and not a string literal a frontend prints. The filter
is the catalog of *declared* IDs rather than the ID shape, so the illustrative
IDs the scaffold and the skill teach (`FS-login`, `FS-014-user-login`) pass by
construction; an ID inside a public `blob/main` URL passes too, because that is
the address §REQ-shipped-surfaces.1 asks a shipped sentence to carry. Covers
what `grund init` writes (§FS-init.2.1), what `grund agent-setup-instructions`
prints (§FS-init.5), and what `grund integrations --write` installs
(§FS-integrations.4)."""

import json
import re
import subprocess
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]

# Directories whose whole content ships: what the binary embeds, and the tree
# copies `test_asset_sync.py` keeps byte-identical to it.
SHIPPED_TREES = ["crates/grund-core/assets", "templates", "skills"]

# An ID-shaped token under this repository's `[id] format` — matched loosely and
# then looked up in the catalog, so `FS-check-full` is one token and not a hit
# on `FS-check`.
ID_SHAPE = re.compile(r"\b[A-Z][A-Z0-9]*-[a-z][a-z0-9-]*")

# The address a shipped sentence is allowed to carry: this repository's own
# public blob URL, which names the same declaration and opens for any reader.
PUBLIC_URL = re.compile(r"https://github\.com/vjovanov/grund/blob/[^\s)\]\"'`]+")

RAW_STRING_OPEN = re.compile(r'r(#*)"')
CHAR_LITERAL = re.compile(r"'(?:\\.|[^\\'])'")


def _declared_ids():
    """This repository's declaration catalog, as `grund list` reports it."""
    listing = subprocess.run(
        ["cargo", "run", "--quiet", "--locked", "--", "list", "--format", "json"],
        cwd=REPO_ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return {json.loads(line)["id"] for line in listing.splitlines() if line.strip()}


def offending_ids(text, catalog):
    """The declared IDs `text` names, ignoring the ones inside a public URL."""
    return sorted({m.group(0) for m in ID_SHAPE.finditer(PUBLIC_URL.sub(" ", text))} & catalog)


def string_literals(source):
    """The content of every string literal in a Rust source file.

    Hand-rolled rather than a regex: comments, char literals and raw strings all
    carry quotes a regex would read as a literal's edge, and a comment is
    exactly where a citation is supposed to live.
    """
    literals = []
    i, n = 0, len(source)
    while i < n:
        rest = source[i:]
        if rest.startswith("//"):
            end = source.find("\n", i)
            i = n if end == -1 else end + 1
        elif rest.startswith("/*"):
            depth, i = 1, i + 2
            while i < n and depth:
                if source.startswith("/*", i):
                    depth, i = depth + 1, i + 2
                elif source.startswith("*/", i):
                    depth, i = depth - 1, i + 2
                else:
                    i += 1
        elif source[i] == "r" and _starts_token(source, i) and RAW_STRING_OPEN.match(source, i):
            opening = RAW_STRING_OPEN.match(source, i)
            closing = '"' + opening.group(1)
            end = source.find(closing, opening.end())
            if end == -1:
                break
            literals.append(source[opening.end() : end])
            i = end + len(closing)
        elif source[i] == '"':
            j = i + 1
            while j < n and source[j] != '"':
                j += 2 if source[j] == "\\" else 1
            literals.append(source[i + 1 : j])
            i = j + 1
        elif source[i] == "'":
            char = CHAR_LITERAL.match(source, i)
            i = char.end() if char else i + 1
        else:
            i += 1
    return literals


def _starts_token(source, i):
    return i == 0 or not (source[i - 1].isalnum() or source[i - 1] == "_")


def _printing_sources():
    """Every Rust source that is part of a shipped binary rather than a test.

    Test code cites the spec it pins and prints to nobody, so it keeps its
    citations; `tests/` directories and `tests*.rs` modules are that code.
    """
    for path in sorted((REPO_ROOT / "crates").rglob("*.rs")):
        parts = path.relative_to(REPO_ROOT).parts
        if "tests" in parts or parts[-1].startswith("tests"):
            continue
        yield path


def _shipped_files():
    for tree in SHIPPED_TREES:
        for path in sorted((REPO_ROOT / tree).rglob("*")):
            if path.is_file():
                yield path
    for path in sorted((REPO_ROOT / "tests" / "e2e").rglob("expected.std*")):
        yield path


class ShippedSurfaceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.catalog = _declared_ids()

    def test_the_catalog_is_the_repositorys_own(self):
        self.assertGreaterEqual(len(self.catalog), 100, "grund list reported no catalog to match")

    def test_no_shipped_file_names_a_declared_id(self):
        for path in _shipped_files():
            with self.subTest(path=str(path.relative_to(REPO_ROOT))):
                named = offending_ids(path.read_text(encoding="utf-8", errors="replace"), self.catalog)
                self.assertEqual(
                    [],
                    named,
                    f"{path.relative_to(REPO_ROOT)} ships an ID of this repository: {', '.join(named)}. "
                    "Drop it, or point the sentence at the declaration's public URL.",
                )

    def test_no_printed_string_names_a_declared_id(self):
        for path in _printing_sources():
            source = path.read_text(encoding="utf-8")
            named = sorted({i for text in string_literals(source) for i in offending_ids(text, self.catalog)})
            with self.subTest(path=str(path.relative_to(REPO_ROOT))):
                self.assertEqual(
                    [],
                    named,
                    f"{path.relative_to(REPO_ROOT)} prints an ID of this repository: {', '.join(named)}. "
                    "Ground the line in the comment beside it instead.",
                )

    def test_the_scan_catches_what_shipped_and_lets_the_illustrations_through(self):
        """The regression this guard exists for, and the three things it must not flag."""
        shipped = "# Citation directions (FS-config.3.9): encode which kinds may cite which"
        self.assertEqual(["FS-config"], offending_ids(shipped, self.catalog))
        for allowed in (
            "an ID like FS-login or FS-014-user-login teaches the grammar",
            "see [the config spec](https://github.com/vjovanov/grund/blob/main/"
            "docs/functional-spec/FS-config.md#39-citations--citation-direction-rules)",
            "no ID at all in this sentence",
        ):
            with self.subTest(allowed=allowed):
                self.assertEqual([], offending_ids(allowed, self.catalog))

    def test_the_literal_scan_reads_code_and_not_the_comments_around_it(self):
        source = '\n'.join(
            [
                '// §FS-cli.6 — grounding lives in the comment.',
                '/* §FS-check.1.3 in a block comment, and a "quote" inside it. */',
                'let url = "https://example.invalid/a//b";',
                'let sep = \'"\';',
                'let help = r#"There is no --config override."#;',
            ]
        )
        # The char literal is skipped whole: were its quote read as a string's
        # edge, every literal after it would be inverted.
        self.assertEqual(
            ["https://example.invalid/a//b", "There is no --config override."],
            string_literals(source),
        )


if __name__ == "__main__":
    unittest.main()
