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

# An `until` says what retires the entry. A size threshold is not that: it is a
# larger budget wearing the word "until", and it retires nothing — the file sits
# there until someone raises the number again.
LINE_COUNT_TRIGGER = re.compile(
    r"\b(passes|exceeds|reaches|hits|grows past|goes over)\s+\d+", re.IGNORECASE
)


def load(registry: Path) -> list[dict]:
    return tomllib.loads(registry.read_text())["exceptions"]


class FileSizeExceptionTests(unittest.TestCase):
    def setUp(self):
        self.hard = load(HARD_REGISTRY)
        self.soft = load(SOFT_REGISTRY)
        self.all = self.hard + self.soft

    def test_every_entry_is_complete(self):
        for entry in self.all:
            for field in ("id", "path", "kind", "reason", "until", "max_accepted"):
                self.assertIn(field, entry, f"{entry.get('id')} is missing `{field}`")

    def test_ids_are_unique_across_both_registries(self):
        ids = [entry["id"] for entry in self.all]
        self.assertCountEqual(ids, set(ids), "exception IDs must be unique")

    def test_kind_agrees_with_until(self):
        for entry in self.all:
            kind, until = entry["kind"], entry["until"]
            self.assertIn(kind, ("structural", "deferred"), entry["id"])
            if kind == "structural":
                self.assertEqual(
                    until,
                    "indefinite",
                    f"{entry['id']}: a structural exception never expires",
                )
            else:
                self.assertNotEqual(
                    until,
                    "indefinite",
                    f"{entry['id']}: a deferred exception must name what retires it",
                )

    def test_until_names_an_event_not_a_line_count(self):
        for entry in self.all:
            self.assertIsNone(
                LINE_COUNT_TRIGGER.search(entry["until"]),
                f"{entry['id']}: `until` is a size threshold, not a boundary — "
                f"name what has to exist before the split can happen",
            )

    def test_paths_still_exist(self):
        for entry in self.all:
            if entry.get("match", "exact") != "exact":
                continue
            self.assertTrue(
                (REPO_ROOT / entry["path"]).exists(),
                f"{entry['id']} accepts {entry['path']}, which no longer exists — "
                f"drop the entry or point it at the file's new home",
            )

    def test_hard_entries_have_a_soft_twin(self):
        """A file past the hard budget is past the soft one too, and fissile
        reports both tiers, so the hard entry alone leaves a warning nobody can
        act on printed on every unrelated commit (§AR-ci.9)."""
        soft_by_path = {entry["path"]: entry for entry in self.soft}
        for entry in self.hard:
            self.assertIn(
                entry["path"],
                soft_by_path,
                f"{entry['id']} silences the hard finding for {entry['path']} but "
                f"leaves the soft one printing forever — add the soft twin",
            )

    def test_soft_twins_point_at_the_hard_entry_instead_of_copying_it(self):
        """One file, one argument. The twin records the same ceiling and names
        the entry that carries the reasoning, so the two cannot drift apart."""
        hard_by_path = {entry["path"]: entry for entry in self.hard}
        for twin in self.soft:
            hard = hard_by_path.get(twin["path"])
            if hard is None:
                continue
            self.assertEqual(
                twin["max_accepted"],
                hard["max_accepted"],
                f"{twin['id']} and {hard['id']} accept the same file at different "
                f"sizes — the ratchet must be one number",
            )
            self.assertIn(
                hard["id"],
                twin["reason"],
                f"{twin['id']} must point at {hard['id']} rather than restate it",
            )
            self.assertIn(
                hard["id"],
                twin["until"],
                f"{twin['id']} retires when {hard['id']} does — say so in `until`",
            )


if __name__ == "__main__":
    unittest.main()
