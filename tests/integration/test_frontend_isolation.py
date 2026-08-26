"""§AR-bindings.1 — the frontends depend on the engine and on nothing of each
other: `grund`'s dependency tree carries no LSP transport, `grund-lsp`'s carries
no CLI, and `grund-core` depends on neither, which is the property
§DA-lsp-optional rests on. Read from `cargo metadata`, so it is the resolved
graph that is checked and not the manifests' intent."""

import json
import subprocess
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
ENGINE = "grund-core"
FRONTENDS = {"grund", "grund-lsp"}
LSP_TRANSPORT = {"lsp-server", "lsp-types"}


class Graph:
    """The resolved dependency graph, normal dependencies only."""

    def __init__(self, metadata):
        self.name_of = {package["id"]: package["name"] for package in metadata["packages"]}
        self.member_id = {self.name_of[pkg_id]: pkg_id for pkg_id in metadata["workspace_members"]}
        self.nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}

    def _normal_deps(self, pkg_id):
        return [
            dep["pkg"]
            for dep in self.nodes[pkg_id]["deps"]
            if any(kind.get("kind") in (None, "normal") for kind in dep["dep_kinds"])
        ]

    def direct(self, member):
        return {self.name_of[pkg_id] for pkg_id in self._normal_deps(self.member_id[member])}

    def closure(self, member):
        seen, stack = set(), [self.member_id[member]]
        while stack:
            for dep in self._normal_deps(stack.pop()):
                if dep not in seen:
                    seen.add(dep)
                    stack.append(dep)
        return {self.name_of[pkg_id] for pkg_id in seen}


def _graph():
    metadata = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked"],
        cwd=REPO_ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return Graph(json.loads(metadata))


class FrontendIsolationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.graph = _graph()

    def test_the_cli_carries_no_lsp_transport(self):
        self.assertEqual(
            set(),
            self.graph.closure("grund") & (LSP_TRANSPORT | {"grund-lsp"}),
            "grund's dependency tree must contain no JSON-RPC machinery and no LSP types",
        )

    def test_the_lsp_server_carries_no_cli(self):
        self.assertNotIn("grund", self.graph.closure("grund-lsp"))

    def test_the_engine_depends_on_no_frontend(self):
        self.assertEqual(
            set(),
            self.graph.closure(ENGINE) & (FRONTENDS | LSP_TRANSPORT),
            "grund-core is the engine every frontend depends on, and depends on none of them",
        )

    def test_every_frontend_depends_directly_on_the_engine_and_on_no_other_member(self):
        members = set(self.graph.member_id) - {ENGINE, "grund-integration-tests"}
        self.assertEqual(FRONTENDS, members, "a new frontend crate joins the isolation contract")
        for frontend in sorted(members):
            with self.subTest(frontend=frontend):
                self.assertIn(ENGINE, self.graph.direct(frontend))
                self.assertEqual(
                    set(),
                    (self.graph.closure(frontend) & members) - {frontend},
                    f"{frontend} must not depend on another frontend",
                )

    def test_the_integration_tests_are_not_a_dependency_of_anything(self):
        for member in self.graph.member_id:
            if member != "grund-integration-tests":
                self.assertNotIn("grund-integration-tests", self.graph.closure(member))


if __name__ == "__main__":
    unittest.main()
