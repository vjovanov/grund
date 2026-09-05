#!/usr/bin/env python3
"""Refuse a release the tree's own messages contradict. §FS-distribution.4.2

A ramp is a promise written into a message. A warning names the release it
becomes an error in, and once the ramp lands the error names the release the
change was made in. Both halves are claims about a version, and a release that
disagrees with one publishes a message the binary it ships in is not.

Given the version about to be cut, this reads the release each message names out
of the tree's own message text and exits 1 when any of them disagrees.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from typing import Iterable, NamedTuple, Sequence


REPO_ROOT = Path(__file__).resolve().parents[1]

# The message text: what a user is shown, and the goldens that pin those bytes.
SOURCES = (("crates", "*.rs"), ("tests/e2e/cases", "expected.stdout"), ("tests/e2e/cases", "expected.stderr"))

PENDING = "pending"
LANDED = "landed"

# The clause vocabulary is closed (§FS-distribution.4.2): a ramp is written in
# it or this gate does not see it. Each entry is the wording the messages
# already use, and the direction the release named in it constrains.
CLAUSES = (
    ("becomes an error in", PENDING),
    ("became an error in", LANDED),
    ("was removed in", LANDED),
    ("stopped loading in", LANDED),
    ("unchecked in", LANDED),
)

PATTERNS = tuple(
    (re.compile(re.escape(clause) + r"\s+(?:grund\s+)?\*{0,2}(\d+\.\d+\.\d+)"), clause, direction)
    for clause, direction in CLAUSES
)

RELEASE_RE = re.compile(r"^\d+\.\d+\.\d+$")


class Claim(NamedTuple):
    path: str
    line: int
    clause: str
    release: str
    direction: str

    def quoted(self) -> str:
        return f"{self.path}:{self.line}: `{self.clause} {self.release}`"


def version(text: str) -> tuple[int, ...]:
    return tuple(int(part) for part in text.split("."))


def scan_text(path: str, text: str) -> Iterable[Claim]:
    for number, line in enumerate(text.splitlines(), start=1):
        for pattern, clause, direction in PATTERNS:
            for match in pattern.finditer(line):
                yield Claim(path, number, clause, match.group(1), direction)


def scan_tree(root: Path) -> list[Claim]:
    claims: list[Claim] = []
    for directory, glob in SOURCES:
        base = root / directory
        if not base.is_dir():
            continue
        for file in sorted(base.rglob(glob)):
            text = file.read_text(encoding="utf-8", errors="replace")
            claims.extend(scan_text(file.relative_to(root).as_posix(), text))
    return claims


def release_window(claims: Sequence[Claim]) -> tuple[str | None, str | None]:
    """The releases this tree may be cut as: at or above the floor, below the ceiling."""
    landed = [claim.release for claim in claims if claim.direction == LANDED]
    pending = [claim.release for claim in claims if claim.direction == PENDING]
    floor = max(landed, key=version) if landed else None
    ceiling = min(pending, key=version) if pending else None
    return floor, ceiling


def violations(claims: Sequence[Claim], release: str) -> tuple[list[Claim], list[Claim]]:
    cut = version(release)
    behind = [c for c in claims if c.direction == LANDED and cut < version(c.release)]
    ahead = [c for c in claims if c.direction == PENDING and cut >= version(c.release)]
    return behind, ahead


def report(claims: Sequence[Claim], release: str) -> list[str]:
    behind, ahead = violations(claims, release)
    if not behind and not ahead:
        return []

    lines = [f"error: this tree cannot be released as {release}"]
    if behind:
        lines.append("")
        lines.append("  these lines say a change has already been made in a later release,")
        lines.append("  so cutting this version would ship it under a version that denies it:")
        lines.extend(f"    {claim.quoted()}" for claim in behind)
    if ahead:
        lines.append("")
        lines.append("  these lines promise a change at a release this version has reached,")
        lines.append("  so cutting this version would break the promise rather than keep it:")
        lines.extend(f"    {claim.quoted()}" for claim in ahead)

    floor, ceiling = release_window(claims)
    lines.append("")
    if floor and ceiling and version(floor) >= version(ceiling):
        lines.append(
            f"  this tree can be cut as no release at all: at or above {floor} for the changes it "
            f"has already made, and below {ceiling} for the ones it still only promises —"
        )
        held = next(c for c in claims if c.direction == PENDING and c.release == ceiling)
        lines.append(f"    {held.quoted()}")
        lines.append(f"  land that ramp too, and every other one naming {ceiling}, before cutting anything.")
    else:
        window = f"at or above {floor}" if floor else "any release"
        if ceiling:
            window += f" and below {ceiling}"
        lines.append(f"  this tree can be cut as {window}.")
        lines.append("  land or revert the ramps above rather than moving the version.")
    return lines


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("release", help="the version about to be cut, without the leading v")
    parser.add_argument("--root", type=Path, default=REPO_ROOT, help="repository root to scan")
    args = parser.parse_args(argv)

    if not RELEASE_RE.match(args.release):
        print(f"error: release must look like 0.13.0, got '{args.release}'", file=sys.stderr)
        return 2

    lines = report(scan_tree(args.root), args.release)
    if lines:
        print("\n".join(lines), file=sys.stderr)
        return 1
    print(f"no message in this tree contradicts a release of {args.release}.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
