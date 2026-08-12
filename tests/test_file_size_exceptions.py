"""Consistency rules for the two fissile file-size exception registries (§AR-ci.9).

`fissile check` validates that each registry parses and that the sizes it
records are still accurate. It cannot see the rules this repository puts on
*what an entry may say* — that a `deferred` entry names the boundary that
retires it rather than a bigger number, and that a file recorded in both
registries carries one argument rather than two that can disagree. Those are
checked here so a registry edit fails the same commit that made it.
"""

from pathlib import Path
import re
import tomllib
import unittest


REPO_ROOT = Path(__file__).resolve().parents[1]
HARD_REGISTRY = REPO_ROOT / "docs" / "file-size-human-exceptions.toml"
SOFT_REGISTRY = REPO_ROOT / "docs" / "file-size-agent-exceptions.toml"

# The schema both registries are written against. fissile refuses a registry
# declaring any other version, so pinning it here turns "upgrade fissile without
# migrating" into a named failure rather than a parse error at commit time.
REGISTRY_VERSION = 2

# An `until` says what retires the entry. A size threshold is not that: it is a
# larger budget wearing the word "until", and it retires nothing — the file sits
# there until someone raises the number again.
LINE_COUNT_TRIGGER = re.compile(
    r"\b(passes|exceeds|reaches|hits|grows past|goes over)\s+\d+", re.IGNORECASE
)


def load(registry: Path) -> list[dict]:
    document = tomllib.loads(registry.read_text())
    version = document.get("fissile_exceptions_version")
    if version != REGISTRY_VERSION:
        raise AssertionError(
            f"{registry.name} declares fissile_exceptions_version = {version!r}, "
            f"but fissile reads {REGISTRY_VERSION}"
        )
    return document["exceptions"]


def where(registry: Path, entry: dict) -> str:
    """Locate an entry the way fissile does: registry file, then path.

    Version 2 dropped the `id` field, because the registry an entry lives in
    and what it accepts already identify it, and a second name is the one that
    can be wrong (§AR-ci.9). Diagnostics here name the same pair, which is also
    the line a reader has to edit.
    """
    return f"{registry.name}: {entry['path']}"


class FileSizeExceptionTests(unittest.TestCase):
    def setUp(self):
        self.hard = load(HARD_REGISTRY)
        self.soft = load(SOFT_REGISTRY)
        self.all = [(HARD_REGISTRY, e) for e in self.hard]
        self.all += [(SOFT_REGISTRY, e) for e in self.soft]

    def test_every_entry_is_complete(self):
        for registry, entry in self.all:
            for field in ("path", "kind", "reason", "until", "max_accepted"):
                self.assertIn(
                    field, entry, f"{where(registry, entry)} is missing `{field}`"
                )

    def test_no_entry_carries_a_removed_field(self):
        """Version 2 removed `id` and `replaces`. fissile rejects them as unknown
        keys; catching them here names the field rather than the parse."""
        for registry, entry in self.all:
            for field in ("id", "replaces"):
                self.assertNotIn(
                    field,
                    entry,
                    f"{where(registry, entry)} still carries `{field}`, which "
                    f"fissile_exceptions_version = 2 removed — delete the line",
                )

    def test_one_entry_per_path_per_registry(self):
        """What identifies an entry is its registry and what it accepts, so two
        entries accepting one path at one severity are two rationales for one
        fact — and fissile reports the overlap as a schema error."""
        for registry, entries in ((HARD_REGISTRY, self.hard), (SOFT_REGISTRY, self.soft)):
            paths = [entry["path"] for entry in entries]
            self.assertCountEqual(
                paths, set(paths), f"{registry.name} accepts a path twice"
            )

    def test_kind_agrees_with_until(self):
        for registry, entry in self.all:
            kind, until = entry["kind"], entry["until"]
            self.assertIn(kind, ("structural", "deferred"), where(registry, entry))
            if kind == "structural":
                self.assertEqual(
                    until,
                    "indefinite",
                    f"{where(registry, entry)}: a structural exception never expires",
                )
            else:
                self.assertNotEqual(
                    until,
                    "indefinite",
                    f"{where(registry, entry)}: a deferred exception must name "
                    f"what retires it",
                )

    def test_until_names_an_event_not_a_line_count(self):
        for registry, entry in self.all:
            self.assertIsNone(
                LINE_COUNT_TRIGGER.search(entry["until"]),
                f"{where(registry, entry)}: `until` is a size threshold, not a "
                f"boundary — name what has to exist before the split can happen",
            )

    def test_paths_still_exist(self):
        for registry, entry in self.all:
            if entry.get("match", "exact") != "exact":
                continue
            self.assertTrue(
                (REPO_ROOT / entry["path"]).exists(),
                f"{where(registry, entry)} accepts a file that no longer exists — "
                f"drop the entry or point it at the file's new home",
            )

    def test_deferred_hard_entries_have_a_soft_twin(self):
        """A file past the hard budget is past the soft one too, and a `deferred`
        hard entry deliberately leaves the soft finding standing, so the hard
        entry alone prints a warning nobody can act on today on every unrelated
        commit (§AR-ci.9). A `structural` hard entry silences both tiers itself,
        so it wants no twin."""
        soft_paths = {entry["path"] for entry in self.soft}
        for entry in self.hard:
            if entry["kind"] == "structural":
                self.assertNotIn(
                    entry["path"],
                    soft_paths,
                    f"{where(HARD_REGISTRY, entry)} is structural, which silences "
                    f"the soft finding too — the soft entry silences nothing and "
                    f"should be dropped",
                )
                continue
            self.assertIn(
                entry["path"],
                soft_paths,
                f"{where(HARD_REGISTRY, entry)} silences the hard finding but "
                f"leaves the soft one printing forever — add the soft twin",
            )

    def test_soft_twins_point_at_the_hard_entry_instead_of_copying_it(self):
        """One file, one argument. The twin records the same ceiling and locates
        the entry that carries the reasoning — by registry, since an entry has no
        name of its own — so the two cannot drift apart."""
        hard_by_path = {entry["path"]: entry for entry in self.hard}
        for twin in self.soft:
            hard = hard_by_path.get(twin["path"])
            if hard is None:
                continue
            here = where(SOFT_REGISTRY, twin)
            self.assertEqual(
                twin["max_accepted"],
                hard["max_accepted"],
                f"{here} and its hard entry accept the same file at different "
                f"sizes — the ratchet must be one number",
            )
            self.assertIn(
                HARD_REGISTRY.name,
                twin["reason"],
                f"{here} must point at {HARD_REGISTRY.name} rather than restate "
                f"the argument it keeps",
            )
            self.assertIn(
                HARD_REGISTRY.name,
                twin["until"],
                f"{here} retires when its entry in {HARD_REGISTRY.name} does — "
                f"say so in `until`",
            )


if __name__ == "__main__":
    unittest.main()
