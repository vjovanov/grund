#!/usr/bin/env python3
"""Generate the large conformant repo used by the instruction benchmark."""

from __future__ import annotations

import argparse
import shutil
from pathlib import Path


DEFAULT_FILE_COUNT = 10_000
DEFAULT_COMPONENT_COUNT = 100


def id_for(index: int) -> str:
    return f"FS-{index:05d}-feature-{index:05d}"


def fixture_config(citations: bool = False) -> str:
    base = """grund_config_version = 1

[reference]
strict = true

[scan]
include = ["docs"]
exclude = ["target", "node_modules", ".git", "dist", "build", ".venv"]
extensions = ["md"]
respect_gitignore = false
"""
    if not citations:
        return base
    # A direction ruleset that exercises the classify + obligation + prohibition
    # passes without producing errors (each generated FS cites the next FS, so
    # the FS->FS obligation is met; no AR exists, so the prohibition never fires)
    # — `grund check` still exits 0 (§FS-config.3.9).
    return base + """
[citations]
default = "may"

[citations.FS]
should = ["FS"]
must-not = ["AR"]
"""


def declaration_body(index: int, file_count: int, component_count: int) -> str:
    ident = id_for(index)
    next_ident = id_for(index + 1 if index < file_count else 1)
    component = (index - 1) % component_count
    return (
        f"# {ident}: Feature {index:05d}\n\n"
        f"Feature {index:05d} belongs to synthetic component {component:03d} "
        f"and cites §{next_ident} so every declaration is used.\n"
    )


def generate_fixture(
    root: Path, file_count: int, component_count: int, citations: bool = False
) -> None:
    if file_count < 1:
        raise ValueError("--files must be at least 1")
    if component_count < 1:
        raise ValueError("--components must be at least 1")

    if root.exists():
        shutil.rmtree(root)

    root.mkdir(parents=True)
    (root / "grund.toml").write_text(fixture_config(citations), encoding="utf-8")

    # §FS-check.3.18: the FS kind's default index requires every declaration to
    # be listed as a link, or `grund check` errors on this fixture instead of
    # measuring it — so every generated declaration gets a matching entry here.
    index_lines = ["# Functional spec\n\n"]

    for index in range(1, file_count + 1):
        component = (index - 1) % component_count
        directory = root / "docs" / "functional-spec" / f"component-{component:03d}"
        directory.mkdir(parents=True, exist_ok=True)
        ident = id_for(index)
        path = directory / f"{ident}.md"
        path.write_text(
            declaration_body(index, file_count, component_count),
            encoding="utf-8",
        )
        index_lines.append(f"- [§{ident}](component-{component:03d}/{ident}.md)\n")

    index_path = root / "docs" / "functional-spec" / "README.md"
    index_path.write_text("".join(index_lines), encoding="utf-8")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        type=Path,
        default=Path("target/bench-fixtures/large-conformant-repo"),
        help="fixture root to replace and regenerate",
    )
    parser.add_argument(
        "--files",
        type=int,
        default=DEFAULT_FILE_COUNT,
        help="number of Markdown declaration files to generate",
    )
    parser.add_argument(
        "--components",
        type=int,
        default=DEFAULT_COMPONENT_COUNT,
        help="number of component directories to spread files across",
    )
    parser.add_argument(
        "--citations",
        action="store_true",
        help="emit a [citations] direction ruleset so the fixture exercises the "
        "citation-direction checks",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    generate_fixture(args.root, args.files, args.components, args.citations)
    print(f"generated {args.files} files under {args.root}")


if __name__ == "__main__":
    main()
