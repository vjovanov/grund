"""§AR-goal-measurement.3 — the backward-compatibility meter proves that each
verdict route is explicit, bounded, and exercised by its repository record."""

import re
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
REQUIREMENT = REPO_ROOT / "docs" / "requirements" / "REQ-backwards-compatibility.md"
REQUIREMENTS = REPO_ROOT / "docs" / "requirements"
DECISIONS = REPO_ROOT / "docs" / "decisions"
COVER_DECISION = DECISIONS / "functional" / "DF-cover-workspace-scope.md"
RELEASE = REPO_ROOT / "docs" / "changelog" / "0.10.1.md"
CORRECTION_ROUTE = "§REQ-backwards-compatibility.5"
CONFLICT_PROOF = "§REQ-no-missed-citation.1"
REQUIREMENT_SECTION_RE = re.compile(r"§(REQ-[a-z0-9-]+)\.(\d+(?:\.\d+)*)")


def _section(text, number):
    match = re.search(
        rf"^## {re.escape(str(number))}\. .+$(.*?)(?=^## \d+\.|\Z)",
        text,
        re.MULTILINE | re.DOTALL,
    )
    if not match:
        raise AssertionError(f"section {number} is missing")
    return match.group(0)


def _requirement_catalog():
    catalog = {}
    for path in REQUIREMENTS.glob("REQ-*.md"):
        text = path.read_text(encoding="utf-8")
        declaration = re.match(r"# (REQ-[a-z0-9-]+):", text)
        if declaration:
            catalog[declaration.group(1)] = set(
                re.findall(r"^## (\d+(?:\.\d+)*)\.", text, re.M)
            )
    return catalog


def _release_entry(text, *markers):
    entries = [
        line
        for line in text.splitlines()
        if line.startswith("- ") and all(marker in line for marker in markers)
    ]
    if len(entries) != 1:
        raise AssertionError(
            f"expected one release entry containing {markers!r}, found {len(entries)}"
        )
    return entries[0]


class RequirementRouteTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.text = REQUIREMENT.read_text(encoding="utf-8")

    def test_verdict_routes_are_exhaustive_and_the_exemption_is_unchanged(self):
        covered = _section(self.text, 1)
        self.assertEqual({"2", "3", "5"}, set(re.findall(r"§([235])\b", covered)))

        exemption = _section(self.text, 4)
        self.assertIn("no defined meaning", exemption)
        self.assertIn("produced no output", exemption)
        self.assertIn("before `1.0`", exemption)
        self.assertIn("not a general escape", exemption)

    def test_correction_route_keeps_all_five_gates_conjunctive(self):
        route = _section(self.text, 5)
        self.assertIn("only when all five conditions hold", route)
        conditions = dict(
            re.findall(r"^\d+\. \*\*([^*]+)\.\*\* (.+)$", route, re.MULTILINE)
        )
        self.assertEqual(
            {
                "Prior prohibition",
                "Accepted proof",
                "Named release",
                "Actionable findings",
                "No new licence",
            },
            set(conditions),
        )
        self.assertRegex(
            conditions["Prior prohibition"],
            r"separately declared hard requirement.*already applied.*cited numbered section",
        )
        self.assertRegex(
            conditions["Accepted proof"],
            r"accepted decision record.*proves both the conflict.*neither the §2 .* nor the §3",
        )
        self.assertRegex(
            conditions["Named release"], r"release names the verdict change"
        )
        self.assertRegex(
            conditions["Actionable findings"],
            r"[Ee]very finding.*names its location.*action the maintainer can take",
        )
        self.assertRegex(
            conditions["No new licence"],
            r"cannot justify ordinary policy tightening, feature removal, or a prohibition invented",
        )


class DecisionRouteTests(unittest.TestCase):
    def test_every_correction_decision_cites_another_hard_requirement_section(self):
        catalog = _requirement_catalog()
        for path in sorted(DECISIONS.rglob("*.md")):
            text = path.read_text(encoding="utf-8")
            if CORRECTION_ROUTE not in text:
                continue
            with self.subTest(path=path.relative_to(REPO_ROOT)):
                self.assertIn("**Status:** Accepted", text)
                cited_prohibitions = [
                    (requirement, section)
                    for requirement, section in REQUIREMENT_SECTION_RE.findall(text)
                    if requirement != "REQ-backwards-compatibility"
                    and section in catalog.get(requirement, set())
                ]
                self.assertTrue(
                    cited_prohibitions,
                    "a decision invoking the correction route must cite a numbered "
                    "section of a different, declared hard requirement",
                )

    def test_cover_decision_is_the_accepted_worked_case(self):
        consequences = _section(COVER_DECISION.read_text(encoding="utf-8"), 4)
        self.assertTrue(
            CORRECTION_ROUTE in consequences,
            "DF-cover-workspace-scope.4 does not invoke the correction route",
        )
        self.assertIn(CONFLICT_PROOF, consequences)
        self.assertRegex(
            consequences,
            r"workspace whose `\[workspace\]` block cannot be expanded now fails `cover`",
        )
        self.assertIn("duplicate workspace project alias", consequences)
        self.assertIn("broken symlink names the path that cannot be read", consequences)
        self.assertIn("no old form to carry beside a new one", consequences)
        self.assertIn(
            "no command the tool ships that completes the change", consequences
        )


class ReleaseRouteTests(unittest.TestCase):
    def test_0_10_1_names_the_worked_correction_and_its_remedies(self):
        entry = _release_entry(
            RELEASE.read_text(encoding="utf-8"), "PR #114", "§DF-cover-workspace-scope"
        )
        self.assertIn("§DF-cover-workspace-scope", entry)
        self.assertTrue(
            CORRECTION_ROUTE in entry,
            "the 0.10.1 PR #114 entry does not invoke the correction route",
        )
        self.assertIn(CONFLICT_PROOF, entry)
        self.assertIn("`grund cover` indexes every project the run loaded", entry)
        self.assertIn("Verdicts move `0` → `2` in workspaces", entry)
        self.assertIn("names a location and a fix", entry)


if __name__ == "__main__":
    unittest.main()
