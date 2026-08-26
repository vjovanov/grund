"""§FS-distribution.4 — the release step that moves the changelog: the
`## Unreleased` section becomes the inline latest release, the previous latest is
archived one-per-file, and the release notes are read back from it."""

import importlib.util
import tempfile
import unittest
from pathlib import Path


SCRIPT_PATH = Path(__file__).resolve().parents[2] / "scripts" / "prepare_changelog_release.py"
SPEC = importlib.util.spec_from_file_location("prepare_changelog_release", SCRIPT_PATH)
prepare_changelog_release = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(prepare_changelog_release)


SAMPLE_CHANGELOG = """# Changelog

Intro.

## Unreleased

### Fixed

- [§FS-distribution.4](functional-spec/FS-distribution.md#4-release-process): rotate release notes automatically.

## 2. [0.2.0] — 2026-05-17

Workspace and agent-entrypoint release. The main user-visible change is workspace aliases.

### Added

- [§FS-workspace](functional-spec/FS-workspace.md#fs-workspace-grund-validates-cross-project-citations-in-a-workspace): validate aliases.

## 3. Older releases

- [0.1.0](changelog/0.1.0.md) — 2026-05-14: first published release and baseline CLI surface.
"""


class PrepareChangelogReleaseTests(unittest.TestCase):
    def write_changelog(self, text: str = SAMPLE_CHANGELOG) -> Path:
        root = Path(self.tempdir.name)
        changelog = root / "docs" / "changelog.md"
        changelog.parent.mkdir(parents=True)
        changelog.write_text(text, encoding="utf-8")
        return changelog

    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()

    def tearDown(self) -> None:
        self.tempdir.cleanup()

    def test_prepare_promotes_unreleased_and_archives_previous_latest(self) -> None:
        changelog = self.write_changelog()

        prepare_changelog_release.prepare_release(changelog, "0.2.1", "2026-05-18")

        updated = changelog.read_text(encoding="utf-8")
        self.assertIn("## Unreleased\n\n## 2. [0.2.1] — 2026-05-18", updated)
        self.assertIn("rotate release notes automatically.", updated)
        self.assertIn(
            "- [0.2.0](changelog/0.2.0.md) — 2026-05-17: Workspace and agent-entrypoint release.",
            updated,
        )
        self.assertIn(
            "- [0.1.0](changelog/0.1.0.md) — 2026-05-14: first published release and baseline CLI surface.",
            updated,
        )

        archived = changelog.parent / "changelog" / "0.2.0.md"
        self.assertEqual(
            archived.read_text(encoding="utf-8"),
            """# 0.2.0 — 2026-05-17

Workspace and agent-entrypoint release. The main user-visible change is workspace aliases.

### Added

- [§FS-workspace](../functional-spec/FS-workspace.md#fs-workspace-grund-validates-cross-project-citations-in-a-workspace): validate aliases.

""",
        )

    def test_archived_links_that_climb_above_docs_gain_another_level(self) -> None:
        """A destination is rewritten by how far the file moved, not by its shape.

        `docs/changelog.md` sits in `docs/`, so a link to a repository-root path is
        written `../crates/...`. Archiving moves the body one directory deeper, to
        `docs/changelog/<version>.md`, where that same destination has to climb twice.
        Leaving an already-climbing link alone is how v0.10.0 shipped a link resolving
        to `docs/crates/...` and turned the tree's own link check red.
        """
        changelog = self.write_changelog(
            """# Changelog

## Unreleased

### Added

- New thing.

## 2. [0.2.0] — 2026-05-17

Previous release.

### Added

- [§AR-checker.2.12](../crates/grund-core/src/checker.rs): an inline declaration.
- [§FS-workspace](functional-spec/FS-workspace.md#fs-workspace): a sibling under docs.
- [an anchor](#goal-x) and [an absolute one](/README.md) and [a url](https://example.com/a).

## 3. Older releases
"""
        )

        prepare_changelog_release.prepare_release(changelog, "0.3.0", "2026-05-18")
        archived = (changelog.parent / "changelog" / "0.2.0.md").read_text(encoding="utf-8")

        # Climbed once against `docs/`, so it climbs twice from `docs/changelog/`.
        self.assertIn("](../../crates/grund-core/src/checker.rs)", archived)
        # A sibling under `docs/` gains exactly one level.
        self.assertIn("](../functional-spec/FS-workspace.md#fs-workspace)", archived)
        # An anchor, an absolute path, and a URL mean the same thing at any depth.
        self.assertIn("](#goal-x)", archived)
        self.assertIn("](/README.md)", archived)
        self.assertIn("](https://example.com/a)", archived)

    def test_prepare_fails_when_unreleased_has_no_bullets(self) -> None:
        changelog = self.write_changelog(
            """# Changelog

## Unreleased

## 2. [0.2.0] — 2026-05-17

Previous release.

## 3. Older releases
"""
        )

        with self.assertRaisesRegex(prepare_changelog_release.ChangelogError, "no bullet entries"):
            prepare_changelog_release.prepare_release(changelog, "0.2.1", "2026-05-18")

    def test_extract_notes_writes_inline_release_body(self) -> None:
        changelog = self.write_changelog()
        output = changelog.parent / "release-notes.md"

        prepare_changelog_release.extract_notes(changelog, "0.2.0", output)

        notes = output.read_text(encoding="utf-8")
        self.assertIn("Workspace and agent-entrypoint release.", notes)
        self.assertIn("### Added", notes)
        self.assertNotIn("Older releases", notes)


if __name__ == "__main__":
    unittest.main()
