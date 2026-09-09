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
RELEASES = REPO_ROOT / "docs" / "changelog"
CORRECTION_ROUTE = "§REQ-backwards-compatibility.5"
CONFLICT_PROOF = "§REQ-no-missed-citation.1"
REQUIREMENT_SECTION_RE = re.compile(r"§(REQ-[a-z0-9-]+)\.(\d+(?:\.\d+)*)")
DECISION_CITATION_RE = re.compile(
    r"§((?:DF|DA)-[a-z0-9-]+)(?:\.[a-z0-9-]+)*\b"
)


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


def _verdict_route_citations(text):
    section = _section(text, 1)
    clause = re.search(r"The \*\*verdict\*\*[^.\n]*\.", section)
    if not clause:
        raise AssertionError("section 1 has no verdict-route clause")
    return set(re.findall(r"§(\d+(?:\.\d+)*)\b", clause.group(0)))


def _release_entries():
    return [
        line
        for path in sorted(RELEASES.glob("*.md"))
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.startswith("- ")
    ]


def _correction_route_errors(text, release_entries, catalog):
    errors = []
    declaration = re.match(r"# ((?:DF|DA)-[a-z0-9-]+):", text)
    if not declaration:
        return ["the decision must declare a DF or DA ID"]
    if "**Status:** Accepted" not in text:
        errors.append("the decision must be accepted")

    route_sections = [
        match.group(0)
        for match in re.finditer(
            r"^## (\d+)\. .+$(.*?)(?=^## \d+\.|\Z)",
            text,
            re.MULTILINE | re.DOTALL,
        )
        if CORRECTION_ROUTE in match.group(0)
    ]
    if len(route_sections) != 1:
        return errors + ["the correction route must be invoked in one numbered section"]

    route = route_sections[0]
    cited_prohibitions = {
        (requirement, section)
        for requirement, section in REQUIREMENT_SECTION_RE.findall(route)
        if requirement != "REQ-backwards-compatibility"
        and section in catalog.get(requirement, set())
    }
    if not cited_prohibitions:
        errors.append(
            "the route must cite a numbered section of a different hard requirement"
        )
    proved_prohibitions = {
        (requirement, section)
        for requirement, section in cited_prohibitions
        if re.search(
            rf"(?:old|prior)[^.\n]*\b(?:violated|forbidden|prohibition)\b"
            rf"[^.\n]*§{re.escape(requirement)}\.{re.escape(section)}\b",
            route,
            re.IGNORECASE,
        )
    }
    if not proved_prohibitions:
        errors.append("the route must prove that the prior verdict violated the citation")
    if not re.search(r"already applied[^.\n]*when[^.\n]*shipped", route):
        errors.append("the route must prove that the prohibition already applied")

    for section, name in (("2", "deprecation"), ("3", "mechanical migration")):
        if not re.search(
            rf"§{section}[^.\n]*(?:cannot|does not|doesn't|has no|there is no)",
            route,
            re.IGNORECASE,
        ):
            errors.append(f"the route must explain why §{section} {name} does not fit")
    if not re.search(r"not a licence|cannot justify", route, re.IGNORECASE):
        errors.append("the route must deny a broader compatibility licence")

    decision_id = declaration.group(1)
    matching_releases = [
        entry
        for entry in release_entries
        if decision_id in DECISION_CITATION_RE.findall(entry)
        and CORRECTION_ROUTE in entry
    ]
    matching_releases = [
        entry
        for entry in matching_releases
        if any(
            f"§{requirement}.{section}" in entry
            for requirement, section in proved_prohibitions
        )
    ]
    if not matching_releases:
        errors.append("the decision must have a matching release record")
    elif not any(
        re.search(
            r"verdicts?[^.]{0,20}(?:change[ds]?|flip(?:s|ped)?|move[sd]?)",
            entry,
            re.IGNORECASE,
        )
        and re.search(r"every finding[^.]*location[^.]*(?:fix|action)", entry, re.I)
        for entry in matching_releases
    ):
        errors.append(
            "the matching release must name the verdict change and located remedies"
        )
    return errors


class RequirementRouteTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.text = REQUIREMENT.read_text(encoding="utf-8")

    def test_verdict_routes_are_exhaustive_and_the_exemption_is_unchanged(self):
        self.assertEqual({"2", "3", "5"}, _verdict_route_citations(self.text))

        exemption = _section(self.text, 4)
        self.assertIn("no defined meaning", exemption)
        self.assertIn("produced no output", exemption)
        self.assertIn("before `1.0`", exemption)
        self.assertIn("not a general escape", exemption)

    def test_unapproved_route_cannot_hide_from_the_exhaustive_check(self):
        mutated = self.text.replace(
            "moves only by §2, §3, or §5", "moves only by §2, §3, §5, or §6"
        )
        self.assertEqual({"2", "3", "5", "6"}, _verdict_route_citations(mutated))

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
    def test_every_correction_decision_proves_all_five_gates(self):
        catalog = _requirement_catalog()
        release_entries = _release_entries()
        for path in sorted(DECISIONS.rglob("*.md")):
            text = path.read_text(encoding="utf-8")
            if CORRECTION_ROUTE not in text:
                continue
            with self.subTest(path=path.relative_to(REPO_ROOT)):
                self.assertEqual(
                    [], _correction_route_errors(text, release_entries, catalog)
                )

    def test_unproved_synthetic_correction_is_rejected(self):
        synthetic = """# DF-synthetic: invalid correction

**Status:** Accepted

## 1. Consequences

§REQ-backwards-compatibility.5 is invoked. §REQ-never-crashes.1 is related,
but there is no conflict proof, ordinary-route analysis, or release record.
"""
        errors = _correction_route_errors(
            synthetic, _release_entries(), _requirement_catalog()
        )
        self.assertIn(
            "the route must prove that the prior verdict violated the citation", errors
        )
        self.assertIn("the route must explain why §2 deprecation does not fit", errors)
        self.assertIn(
            "the route must explain why §3 mechanical migration does not fit", errors
        )
        self.assertIn("the decision must have a matching release record", errors)

    def test_release_join_requires_the_complete_decision_citation_id(self):
        decision = COVER_DECISION.read_text(encoding="utf-8")
        release_entries = _release_entries()
        catalog = _requirement_catalog()
        self.assertEqual(
            [], _correction_route_errors(decision, release_entries, catalog)
        )

        prefix_collision = decision.replace(
            "# DF-cover-workspace-scope:", "# DF-cover-workspace:", 1
        )
        self.assertIn(
            "the decision must have a matching release record",
            _correction_route_errors(prefix_collision, release_entries, catalog),
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
