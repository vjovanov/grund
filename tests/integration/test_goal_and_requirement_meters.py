"""§AR-goal-measurement.1 — an unmeasured requirement is a wish: every goal
declared in `docs/goals.md` and every requirement under `docs/requirements/`
has a row in the meters tables, and every row names a goal or requirement
that still exists."""

import re
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
METERS = REPO_ROOT / "docs" / "architecture" / "AR-goal-measurement.md"
GOALS = REPO_ROOT / "docs" / "goals.md"
REQUIREMENTS = REPO_ROOT / "docs" / "requirements"
ROW_ID = re.compile(r"^\|\s*\[§((?:GOAL|REQ)-[a-z0-9-]+)\]")


def _declared():
    goals = set(re.findall(r"^## (GOAL-[a-z0-9-]+):", GOALS.read_text(encoding="utf-8"), re.M))
    requirements = set()
    for page in REQUIREMENTS.glob("REQ-*.md"):
        first = page.read_text(encoding="utf-8").splitlines()[0]
        match = re.match(r"# (REQ-[a-z0-9-]+):", first)
        if match:
            requirements.add(match.group(1))
    return goals, requirements


def _metered():
    ids = set()
    for line in METERS.read_text(encoding="utf-8").splitlines():
        match = ROW_ID.match(line)
        if match:
            ids.add(match.group(1))
    return ids


class MeterTests(unittest.TestCase):
    def test_every_goal_and_requirement_has_a_meter(self):
        goals, requirements = _declared()
        self.assertGreaterEqual(len(goals), 5)
        self.assertGreaterEqual(len(requirements), 5)
        metered = _metered()
        self.assertEqual(set(), goals - metered, "goals with no meter row")
        self.assertEqual(set(), requirements - metered, "requirements with no meter row")

    def test_every_meter_row_names_something_declared(self):
        goals, requirements = _declared()
        self.assertEqual(set(), _metered() - goals - requirements, "meter rows for nothing")


if __name__ == "__main__":
    unittest.main()
