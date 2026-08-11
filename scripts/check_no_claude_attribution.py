#!/usr/bin/env python3
"""Block AI-tool attribution boilerplate from files and commit messages. §AR-ci.8

Three surfaces, one pattern set:

* ``--files PATH...``      staged file contents (pre-commit stage)
* ``--message-file PATH``  a single commit message (commit-msg stage)
* ``--range A..B``         every commit message in a range (CI)

The patterns are narrow on purpose. Prose that mentions Claude is fine; the
machine-generated trailers and "generated with" markers are not, because they
are what a tool appends without being asked.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path
from typing import Iterable, Sequence


class AttributionError(Exception):
    pass


# (label, pattern). The label names the offending form in the failure report so
# the fix is obvious without reading this file.
PATTERNS: tuple[tuple[str, re.Pattern[str]], ...] = (
    ("co-author trailer", re.compile(r"(?i)co-authored-by:\s*claude")),
    ("session trailer", re.compile(r"(?i)^\s*claude-session:\s*\S")),
    ("generated-with marker", re.compile(r"(?i)generated with \[?claude code\]?")),
    ("robot generated-with marker", re.compile(r"🤖 Generated with")),
    ("Anthropic no-reply address", re.compile(r"(?i)noreply@anthropic\.com")),
)


class Finding:
    def __init__(self, origin: str, line_number: int, line: str, label: str) -> None:
        self.origin = origin
        self.line_number = line_number
        self.line = line
        self.label = label

    def __str__(self) -> str:
        return f"{self.origin}:{self.line_number}: {self.label}: {self.line.strip()}"


def find_attribution(text: str, origin: str) -> list[Finding]:
    findings = []
    for line_number, line in enumerate(text.splitlines(), start=1):
        for label, pattern in PATTERNS:
            if pattern.search(line):
                findings.append(Finding(origin, line_number, line, label))
                break
    return findings


def scan_files(paths: Iterable[Path]) -> list[Finding]:
    findings = []
    for path in paths:
        try:
            text = path.read_text(encoding="utf-8")
        except FileNotFoundError:
            # A path staged for deletion is not a path we can scan.
            continue
        except (UnicodeDecodeError, OSError):
            # Binary or unreadable: nothing to match, same as `grep -I`.
            continue
        findings.extend(find_attribution(text, str(path)))
    return findings


def scan_commits(log_args: Sequence[str]) -> list[Finding]:
    """Scan every commit message ``git log`` selects with ``log_args``.

    Records are delimited with ASCII unit/record separators so a commit message
    containing blank lines or the delimiter's printable neighbours still parses.
    """
    result = subprocess.run(
        ["git", "log", "--format=%H%x1f%B%x1e", *log_args],
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise AttributionError(f"git log {' '.join(log_args)} failed: {result.stderr.strip()}")

    findings = []
    scanned = 0
    for record in result.stdout.split("\x1e"):
        record = record.strip("\n")
        if not record:
            continue
        sha, _, message = record.partition("\x1f")
        scanned += 1
        findings.extend(find_attribution(message, f"commit {sha[:10]}"))
    # Say how much was actually read. A mis-built range selects nothing and
    # passes, which is indistinguishable from a real pass unless the count is
    # on the record.
    print(f"scanned {scanned} commit message(s) in {' '.join(log_args)}")
    return findings


def resolve_range(rev_range: str) -> list[str]:
    """Turn a requested range into `git log` arguments.

    A force-push reports a ``before`` SHA the remote no longer has, and the
    first push of a branch reports the all-zero SHA. Neither is resolvable, and
    neither should turn the gate red: fall back to scanning the tip commit.
    """
    base, sep, _ = rev_range.partition("..")
    if not sep:
        return [rev_range]
    if not base or set(base) == {"0"}:
        return ["-1", "HEAD"]
    probe = subprocess.run(
        ["git", "cat-file", "-e", f"{base}^{{commit}}"],
        check=False,
        capture_output=True,
    )
    if probe.returncode != 0:
        print(f"note: {base} is unreachable; scanning HEAD only", file=sys.stderr)
        return ["-1", "HEAD"]
    return [rev_range]


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Reject Claude attribution boilerplate.")
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--files", nargs="*", type=Path, help="file contents to scan")
    source.add_argument("--message-file", type=Path, help="a commit message file to scan")
    source.add_argument("--range", dest="rev_range", help="a git revision range of commit messages")
    args = parser.parse_args(argv)

    try:
        if args.rev_range is not None:
            findings = scan_commits(resolve_range(args.rev_range))
        elif args.message_file is not None:
            findings = scan_files([args.message_file])
        else:
            findings = scan_files(args.files or [])
    except AttributionError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    if findings:
        print("Claude attribution boilerplate found:", file=sys.stderr)
        for finding in findings:
            print(f"  {finding}", file=sys.stderr)
        print(
            "\nRemove the line(s) above. In a commit message, amend with "
            "`git commit --amend` and drop the trailer.",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
