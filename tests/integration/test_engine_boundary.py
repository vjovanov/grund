"""§AR-bindings.2 — the engine returns data and the frontends render it: the
embedding API (`api*.rs`) writes to no stream and exits no process; terminal
rendering inside `grund-core` exists only on the deprecated
`grund_core::main_entry()` path, in a closed set of modules that may shrink
and never grow unnoticed; and the published CLI and the LSP server import
none of it — `grund` calls its own `main_entry`, never the engine's."""

import re
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
CORE = REPO_ROOT / "crates" / "grund-core" / "src"
FRONTENDS = (REPO_ROOT / "crates" / "grund-cli" / "src", REPO_ROOT / "crates" / "grund-lsp" / "src")
STREAM_OR_EXIT = re.compile(r"\b(e?println!|e?print!\(|io::stdout|io::stderr|process::exit)")
# The modules the deprecated `main_entry()` path still renders through. A new
# name here is a decision to widen the deprecation boundary, not a slip: the
# embedding API and every module not listed stay data-only.
COMPAT_RENDERERS = {
    "checker_cmd.rs",
    "compat_cli.rs",
    "completions.rs",
    "config_cmd.rs",
    "cover.rs",
    "fmt_cmd.rs",
    "id.rs",
    "init_cmd.rs",
    "integrations.rs",
    "list.rs",
    "output.rs",
    "refs.rs",
    "show.rs",
    "workspace_members.rs",
}
ENGINE_ONLY_SYMBOLS = ("grund_core::main_entry", "compat_cli", "grund_core::command_")


def _implementation_files():
    return sorted(
        path for path in CORE.glob("*.rs") if not path.name.startswith("tests")
    )


def _writes_to_a_stream(path):
    return bool(STREAM_OR_EXIT.search(path.read_text(encoding="utf-8")))


class EngineBoundaryTests(unittest.TestCase):
    def test_the_embedding_api_writes_nothing(self):
        for path in sorted(CORE.glob("api*.rs")):
            with self.subTest(file=path.name):
                self.assertFalse(_writes_to_a_stream(path), f"{path.name} renders or exits")

    def test_rendering_in_the_engine_stays_inside_the_compat_modules(self):
        writers = {path.name for path in _implementation_files() if _writes_to_a_stream(path)}
        self.assertEqual(
            set(),
            writers - COMPAT_RENDERERS,
            "engine modules that write to a stream or exit outside the deprecated "
            "main_entry() rendering set",
        )

    def test_the_compat_set_names_only_modules_that_still_render(self):
        writers = {path.name for path in _implementation_files() if _writes_to_a_stream(path)}
        self.assertEqual(set(), COMPAT_RENDERERS - writers, "stale entries: shrink the set")

    def test_the_frontends_import_no_engine_rendering(self):
        for frontend in FRONTENDS:
            for path in sorted(frontend.glob("**/*.rs")):
                text = path.read_text(encoding="utf-8")
                for symbol in ENGINE_ONLY_SYMBOLS:
                    with self.subTest(file=path.relative_to(REPO_ROOT).as_posix(), symbol=symbol):
                        self.assertNotIn(symbol, text)

    def test_the_cli_owns_its_own_main_entry(self):
        cli = REPO_ROOT / "crates" / "grund-cli" / "src"
        self.assertIn("grund::main_entry()", (cli / "main.rs").read_text(encoding="utf-8"))
        self.assertTrue(
            any("pub fn main_entry()" in path.read_text(encoding="utf-8") for path in cli.glob("*.rs")),
            "grund-cli defines the main_entry the binary calls",
        )


if __name__ == "__main__":
    unittest.main()
