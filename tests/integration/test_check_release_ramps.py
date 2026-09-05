"""§FS-distribution.4.2 — the release gate reads the release each message names
out of the tree's own message text and refuses a version that contradicts one.
The synthetic trees below pin the two directions and the closed clause
vocabulary; the last two run the gate against this repository, because a gate
wired to text nobody writes any more would pass everything in silence."""

import importlib.util
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "scripts" / "check_release_ramps.py"

_spec = importlib.util.spec_from_file_location("check_release_ramps", SCRIPT)
ramps = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(ramps)


def claims(text, path="crates/grund-core/src/x.rs"):
    return list(ramps.scan_text(path, text))


def refused(text, release):
    return ramps.report(claims(text), release)


class ClauseReadingTests(unittest.TestCase):
    def test_a_pending_clause_names_its_release(self):
        (claim,) = claims('"… becomes an error in grund 0.14.0"')
        self.assertEqual((claim.release, claim.direction), ("0.14.0", ramps.PENDING))

    def test_a_landed_clause_names_its_release(self):
        (claim,) = claims('"`prefix` was removed in grund 0.13.0 — rename it"')
        self.assertEqual((claim.release, claim.direction), ("0.13.0", ramps.LANDED))

    def test_grund_is_optional_and_bold_is_tolerated(self):
        for text in ("stopped loading in 0.13.0", "stopped loading in grund **0.13.0**"):
            (claim,) = claims(text)
            self.assertEqual(claim.release, "0.13.0", text)

    def test_the_clause_vocabulary_is_closed(self):
        self.assertEqual(claims("this will be removed in 0.15.0 one day"), [])

    def test_a_release_is_never_read_across_a_line_break(self):
        self.assertEqual(claims("… becomes an error in\ngrund 0.14.0."), [])

    def test_every_line_of_a_golden_is_read(self):
        found = claims("was removed in 0.13.0\nx\nbecomes an error in 0.14.0")
        self.assertEqual([(c.line, c.release) for c in found], [(1, "0.13.0"), (3, "0.14.0")])


class VerdictTests(unittest.TestCase):
    LANDED = '"`prefix` was removed in grund 0.13.0"'
    PENDING = '"an index entry becomes an error in grund 0.13.0"'

    def test_a_landed_change_may_not_ship_below_the_release_it_names(self):
        report = refused(self.LANDED, "0.12.4")
        self.assertTrue(report)
        self.assertIn("cannot be released as 0.12.4", report[0])
        self.assertTrue(any("was removed in 0.13.0" in line for line in report))

    def test_a_landed_change_ships_at_the_release_it_names(self):
        self.assertEqual(refused(self.LANDED, "0.13.0"), [])

    def test_a_landed_change_ships_above_the_release_it_names(self):
        self.assertEqual(refused(self.LANDED, "0.14.0"), [])

    def test_a_pending_promise_may_not_ship_at_the_release_it_names(self):
        report = refused(self.PENDING, "0.13.0")
        self.assertTrue(any("becomes an error in 0.13.0" in line for line in report))

    def test_a_pending_promise_ships_below_the_release_it_names(self):
        self.assertEqual(refused(self.PENDING, "0.12.4"), [])

    def test_a_tree_that_landed_and_still_promises_one_release_can_cut_nothing(self):
        report = refused(f"{self.LANDED}\n{self.PENDING}", "0.12.4")
        self.assertTrue(any("no release at all" in line for line in report))

    def test_the_window_is_the_highest_landed_and_the_lowest_promised(self):
        found = claims('"was removed in 0.11.0"\n"was removed in 0.13.0"\n"becomes an error in 0.14.0"')
        self.assertEqual(ramps.release_window(found), ("0.13.0", "0.14.0"))

    def test_a_tree_naming_no_release_refuses_nothing(self):
        self.assertEqual(refused('"ordinary message"', "0.12.4"), [])

    def test_a_version_that_is_not_a_release_is_a_usage_error(self):
        self.assertEqual(ramps.main(["0.12.4-dev"]), 2)


class ThisRepositoryTests(unittest.TestCase):
    """The gate is only worth its step if it still sees this tree's own ramps."""

    @classmethod
    def setUpClass(cls):
        cls.claims = ramps.scan_tree(REPO_ROOT)

    def test_the_scan_reaches_both_a_rust_source_and_an_e2e_golden(self):
        homes = {claim.path.split("/")[0] for claim in self.claims}
        self.assertIn("crates", homes)
        self.assertIn("tests", homes)

    def test_the_removal_this_tree_landed_holds_the_floor_at_0_13_0(self):
        floor, _ = ramps.release_window(self.claims)
        self.assertEqual(floor, "0.13.0")
        report = ramps.report(self.claims, "0.12.4")
        self.assertTrue(
            any("config_kinds.rs" in line for line in report),
            "the `prefix` removal must be what refuses a 0.12.x release of this tree",
        )


if __name__ == "__main__":
    unittest.main()
