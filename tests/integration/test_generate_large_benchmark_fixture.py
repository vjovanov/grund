"""§AR-benchmarks.1 — the generated large conformant fixture the
instruction-counting benches read: what the generator writes is conformant, and
the same arguments write the same tree."""

import importlib.util
import tempfile
import unittest
from pathlib import Path


SCRIPT_PATH = (
    Path(__file__).resolve().parents[2] / "scripts" / "generate_large_benchmark_fixture.py"
)
SPEC = importlib.util.spec_from_file_location("generate_large_benchmark_fixture", SCRIPT_PATH)
generate_large_benchmark_fixture = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(generate_large_benchmark_fixture)


class GenerateLargeBenchmarkFixtureTests(unittest.TestCase):
    def test_generates_requested_file_count_and_config(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "fixture"
            generate_large_benchmark_fixture.generate_fixture(root, 12, 3)

            markdown_files = sorted((root / "docs").rglob("*.md"))
            self.assertEqual(len(markdown_files), 12)
            self.assertTrue((root / "grund.toml").exists())
            self.assertEqual(
                markdown_files[0].relative_to(root).as_posix(),
                "docs/functional-spec/component-000/FS-00001-feature-00001.md",
            )
            self.assertIn(
                "§FS-00002-feature-00002",
                markdown_files[0].read_text(encoding="utf-8"),
            )
            self.assertIn(
                "§FS-00001-feature-00001",
                markdown_files[-1].read_text(encoding="utf-8"),
            )

    def test_replaces_existing_fixture_deterministically(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "fixture"
            generate_large_benchmark_fixture.generate_fixture(root, 4, 2)
            first = {
                path.relative_to(root).as_posix(): path.read_text(encoding="utf-8")
                for path in sorted(root.rglob("*"))
                if path.is_file()
            }

            (root / "stale.txt").write_text("stale", encoding="utf-8")
            generate_large_benchmark_fixture.generate_fixture(root, 4, 2)
            second = {
                path.relative_to(root).as_posix(): path.read_text(encoding="utf-8")
                for path in sorted(root.rglob("*"))
                if path.is_file()
            }

            self.assertEqual(first, second)
            self.assertNotIn("stale.txt", second)

    def test_rejects_empty_fixture(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaisesRegex(ValueError, "--files"):
                generate_large_benchmark_fixture.generate_fixture(Path(tmp) / "fixture", 0, 1)


if __name__ == "__main__":
    unittest.main()
