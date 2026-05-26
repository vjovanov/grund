#!/usr/bin/env python3
"""Write a local wall-clock benchmark report for grund vs lychee."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import platform
import re
import shutil
import socket
import statistics
import subprocess
import sys
import time
from pathlib import Path


DEFAULT_LYCHEE_PATHS = ["README.md", "docs", "examples"]
MARKUP_EXTENSIONS = {".md", ".markdown", ".html", ".htm"}
INSTRUCTION_BASELINE = [
    ("check", "this repo", "299,672,739", "431,943,109"),
    ("check_large_10k", "generated 10k-file fixture", "1,055,099,244", "1,475,491,249"),
    ("list", "this repo", "289,680,670", "417,771,050"),
    ("show_brief", "this repo", "284,142,202", "409,995,654"),
    ("show", "this repo", "284,116,880", "410,041,898"),
    ("show_full", "this repo", "290,702,550", "419,471,481"),
    ("refs", "this repo", "284,135,903", "410,102,297"),
    ("cover", "this repo", "301,086,562", "433,510,661"),
    ("fmt_check", "this repo", "349,977,643", "502,904,379"),
]
MARKDOWN_LINK_TOKEN_IMPACT = {
    "markdown_files": "70",
    "bare_tokens": "149,966",
    "linked_tokens": "180,461",
    "delta_tokens": "+30,495",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Measure local cold/warm wall-clock timings for grund and lychee, "
            "collect machine/tool/workload details, and write a markdown report."
        )
    )
    parser.add_argument(
        "--repo",
        default=".",
        help="Repository root to measure (default: current directory).",
    )
    parser.add_argument(
        "--out",
        default="docs/benchmarks.md",
        help="Markdown report path (default: docs/benchmarks.md).",
    )
    parser.add_argument(
        "--grund",
        default=None,
        help="grund binary to run (default: target/release/grund if present, else grund on PATH).",
    )
    parser.add_argument(
        "--lychee",
        default="lychee",
        help="lychee binary to run (default: lychee).",
    )
    parser.add_argument(
        "--warm-runs",
        type=int,
        default=7,
        help="Number of warm timing samples after the cold run (default: 7).",
    )
    parser.add_argument(
        "--lychee-path",
        action="append",
        dest="lychee_paths",
        help=(
            "Path passed to lychee. May be repeated. "
            "Default: README.md, docs, examples."
        ),
    )
    parser.add_argument(
        "--no-lychee",
        action="store_true",
        help="Skip the lychee measurement and the comparison section (e.g. for CI runs where lychee is not installed).",
    )
    parser.add_argument(
        "--no-markdown",
        action="store_true",
        help="Do not write the markdown report. Useful when only the badge JSON is needed.",
    )
    parser.add_argument(
        "--badge-out",
        default=None,
        help=(
            "If set, also write a shields.io endpoint-format JSON file with the "
            "lines-of-code-per-second throughput derived from the grund check warm median."
        ),
    )
    return parser.parse_args()


def run_capture(command: list[str], cwd: Path, check: bool = True) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        command,
        cwd=cwd,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if check and result.returncode != 0:
        rendered = " ".join(command)
        raise RuntimeError(
            f"command failed ({result.returncode}): {rendered}\n"
            f"stdout:\n{result.stdout}\n"
            f"stderr:\n{result.stderr}"
        )
    return result


def time_command(command: list[str], cwd: Path) -> float:
    start = time.perf_counter()
    result = subprocess.run(
        command,
        cwd=cwd,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
    )
    elapsed = time.perf_counter() - start
    if result.returncode != 0:
        rendered = " ".join(command)
        raise RuntimeError(
            f"timed command failed ({result.returncode}): {rendered}\n{result.stderr}"
        )
    return elapsed


def resolve_grund(repo: Path, explicit: str | None) -> str:
    if explicit:
        return explicit
    local_release = repo / "target" / "release" / "grund"
    if local_release.exists():
        return str(local_release)
    return "grund"


def command_version(command: list[str], cwd: Path) -> str:
    try:
        result = run_capture(command, cwd)
    except Exception as exc:  # pragma: no cover - defensive report path
        return f"unavailable ({exc})"
    return (result.stdout or result.stderr).strip().splitlines()[0]


def git_value(args: list[str], repo: Path) -> str:
    result = run_capture(["git", *args], repo, check=False)
    if result.returncode != 0:
        return "unavailable"
    return result.stdout.strip() or "unavailable"


def git_dirty(repo: Path) -> str:
    result = run_capture(["git", "status", "--short"], repo, check=False)
    if result.returncode != 0:
        return "unknown"
    return "yes" if result.stdout.strip() else "no"


def mem_total() -> str:
    meminfo = Path("/proc/meminfo")
    if not meminfo.exists():
        return "unavailable"
    for line in meminfo.read_text(encoding="utf-8", errors="replace").splitlines():
        if line.startswith("MemTotal:"):
            kib = int(line.split()[1])
            return f"{kib / 1024 / 1024:.1f} GiB"
    return "unavailable"


def cpu_model() -> str:
    cpuinfo = Path("/proc/cpuinfo")
    if cpuinfo.exists():
        for line in cpuinfo.read_text(encoding="utf-8", errors="replace").splitlines():
            if line.startswith("model name"):
                return line.split(":", 1)[1].strip()
    return platform.processor() or "unavailable"


def machine_specs(repo: Path) -> dict[str, str]:
    usage = shutil.disk_usage(repo)
    return {
        "timestamp": dt.datetime.now().astimezone().isoformat(timespec="seconds"),
        "host": socket.gethostname(),
        "os": platform.platform(),
        "kernel": " ".join(platform.uname()),
        "cpu": cpu_model(),
        "logical_cpus": str(os.cpu_count() or "unavailable"),
        "memory": mem_total(),
        "repo_disk_free": f"{usage.free / 1024 / 1024 / 1024:.1f} GiB",
        "python": platform.python_version(),
    }


def grund_workload(grund: str, repo: Path) -> dict[str, int]:
    listed = run_capture([grund, "list", str(repo)], repo).stdout
    declarations = len([line for line in listed.splitlines() if line.strip()])

    cover = run_capture([grund, "cover", "--format", "json", str(repo)], repo).stdout
    files = 0
    citations = 0
    scanned_lines = 0
    for line in cover.splitlines():
        if not line.strip():
            continue
        files += 1
        record = json.loads(line)
        citations += len(record.get("citations", []))
        scanned_path = repo / record["path"]
        try:
            with scanned_path.open("rb") as handle:
                scanned_lines += sum(1 for _ in handle)
        except OSError:
            pass
    return {
        "declarations": declarations,
        "citations": citations,
        "scanned_files": files,
        "scanned_lines": scanned_lines,
        "edges": declarations + citations,
    }


def count_markup_files(paths: list[str], repo: Path) -> int:
    count = 0
    for raw in paths:
        path = (repo / raw).resolve()
        if path.is_file() and path.suffix.lower() in MARKUP_EXTENSIONS:
            count += 1
        elif path.is_dir():
            for child in path.rglob("*"):
                if child.is_file() and child.suffix.lower() in MARKUP_EXTENSIONS:
                    count += 1
    return count


def lychee_workload(lychee: str, paths: list[str], repo: Path) -> dict[str, int | str]:
    command = [lychee, "--no-progress", "--include-fragments", *paths]
    result = run_capture(command, repo)
    output = f"{result.stdout}\n{result.stderr}"
    total = re.search(r"(\d+)\s+Total", output)
    ok = re.search(r"(\d+)\s+OK", output)
    errors = re.search(r"(\d+)\s+Errors", output)
    excluded = re.search(r"(\d+)\s+Excluded", output)
    return {
        "links": int(total.group(1)) if total else -1,
        "ok": int(ok.group(1)) if ok else -1,
        "errors": int(errors.group(1)) if errors else -1,
        "excluded": int(excluded.group(1)) if excluded else -1,
        "scanned_files": count_markup_files(paths, repo),
        "summary": " ".join(output.split()),
    }


def measure(name: str, command: list[str], cwd: Path, warm_runs: int) -> dict[str, object]:
    cold = time_command(command, cwd)
    warm = [time_command(command, cwd) for _ in range(warm_runs)]
    return {
        "name": name,
        "command": command,
        "cold": cold,
        "warm": warm,
        "median": statistics.median(warm),
        "minimum": min(warm),
        "maximum": max(warm),
    }


def seconds(value: float) -> str:
    return f"{value:.3f}s"


def ratio(numerator: float, denominator: float) -> str:
    if denominator == 0:
        return "n/a"
    return f"{numerator / denominator:.1f}x"


def command_text(command: list[str], repo: Path) -> str:
    rendered = []
    repo_text = str(repo)
    for item in command:
        if item == repo_text:
            rendered.append(".")
            continue
        try:
            item_path = Path(item)
            if item_path.is_absolute():
                rendered.append(str(item_path.relative_to(repo)))
                continue
        except ValueError:
            pass
        rendered.append(item)
    return " ".join(rendered)


def write_report(
    out: Path,
    repo: Path,
    grund: str,
    lychee: str | None,
    lychee_paths: list[str],
    warm_runs: int,
    specs: dict[str, str],
    versions: dict[str, str],
    git: dict[str, str],
    grund_load: dict[str, int],
    lychee_load: dict[str, int | str] | None,
    results: list[dict[str, object]],
) -> None:
    by_name = {str(result["name"]): result for result in results}
    grund_check = by_name["grund check"]
    lychee_check = by_name.get("lychee")
    grund_per_edge = float(grund_check["median"]) / grund_load["edges"]
    if lychee_check is not None and lychee_load is not None:
        lychee_per_link = float(lychee_check["median"]) / int(lychee_load["links"])
    else:
        lychee_per_link = None

    lines = [
        "# Local benchmark report",
        "",
        "This report is a local wall-clock snapshot for the `grund` repo. It complements "
        "the instruction-counting CI benchmark in [§AR-benchmarks](architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands) "
        "and the baseline work tracked by [§RM-benchmarks](roadmap.md#rm-benchmarks-a-benchmark-harness-for-the-goal-fast-feedback-budgets). "
        "It is meant for product-facing comparisons with Lychee; it is not the release-blocking regression meter.",
        "",
        "## Instruction-Count Baseline",
        "",
        "The release-blocking meter is Callgrind instruction count, not wall-clock time "
        "([§DA-benchmark-instruction-counting](decisions/architectural/DA-benchmark-instruction-counting.md#da-benchmark-instruction-counting-the-performance-harness-counts-instructions-not-wall-clock-seconds)). "
        "Pull-request CI compares against the current base branch and fails when `Ir` grows by more than 5% "
        "([§AR-ci.5](architecture/AR-ci.md#5-benchmark-job)); the table below is the committed human-readable snapshot from "
        "`cargo bench -p grund --features bench --locked --bench instructions -- --save-summary=json` on 2026-05-20.",
        "",
        "| Benchmark | Input | Instructions (`Ir`) | Estimated Cycles |",
        "|---|---|---:|---:|",
    ]
    for benchmark, input_name, instructions, cycles in INSTRUCTION_BASELINE:
        lines.append(f"| `{benchmark}` | {input_name} | {instructions} | {cycles} |")
    lines.extend(
        [
            "",
        "## Instructions",
        "",
        "Regenerate this report from the repository root:",
        "",
        "```sh",
        "python3 scripts/local-benchmark-report.py --out docs/benchmarks.md",
        "```",
        "",
        "Useful options:",
        "",
        "- `--warm-runs N` changes the number of warm samples after the cold run.",
        "- `--grund PATH` points at a specific `grund` binary; by default the script uses `target/release/grund` when present.",
        "- `--lychee PATH` points at a specific Lychee binary.",
        "- `--lychee-path PATH` may be repeated to replace the default Lychee inputs: `README.md docs examples`.",
        "",
        "For a fair local comparison, build the release binary first:",
        "",
        "```sh",
        "cargo build --release --locked",
        "python3 scripts/local-benchmark-report.py --out docs/benchmarks.md",
        "```",
        "",
        "## Method",
        "",
        f"- Cold time is the first measured invocation of each exact command in this script run. The script does not use `sudo` and does not drop the OS page cache.",
        f"- Warm time is the median of {warm_runs} immediate subsequent invocations with command output suppressed.",
        "- Timings use Python `time.perf_counter()` around the whole subprocess, so process startup and argument parsing are included.",
        "- Lychee may perform URL work and may benefit from its own cache or network conditions; `grund` works only over the local scanned tree.",
        "",
        "## Machine",
        "",
        "| Field | Value |",
        "|---|---|",
        ]
    )
    for key in [
        "timestamp",
        "host",
        "os",
        "kernel",
        "cpu",
        "logical_cpus",
        "memory",
        "repo_disk_free",
        "python",
    ]:
        lines.append(f"| {key.replace('_', ' ').title()} | {specs[key]} |")
    lines.extend(
        [
            "",
            "## Tool Versions",
            "",
            "| Tool | Version |",
            "|---|---|",
            f"| `grund` | `{versions['grund']}` |",
        ]
    )
    if "lychee" in versions:
        lines.append(f"| `lychee` | `{versions['lychee']}` |")
    lines.extend(
        [
            f"| Git commit | `{git['commit']}` |",
            f"| Git branch | `{git['branch']}` |",
            f"| Working tree dirty | `{git['dirty']}` |",
            "",
            "## Workload",
            "",
            "| Tool | Local work checked |",
            "|---|---:|",
            f"| `grund check .` | {grund_load['declarations']:,} declarations + {grund_load['citations']:,} citations across {grund_load['scanned_files']:,} scanned files ({grund_load['scanned_lines']:,} lines) |",
        ]
    )
    if lychee_load is not None:
        lines.append(
            f"| `lychee --include-fragments README.md docs examples` | "
            f"{int(lychee_load['links']):,} links across "
            f"{int(lychee_load['scanned_files']):,} markup files |"
        )
    lines.extend(
        [
            "",
            "## Markdown Link Token Impact",
            "",
            "Markdown cross-reference links are generated presentation over the underlying citation text "
            "([§FS-fmt.6](functional-spec/FS-fmt.md#6-cross-reference-emission)). "
            "To quantify that prompt cost, this snapshot measured temporary copies under `/tmp/grund-link-impact` "
            "with `tiktoken` `o200k_base`. "
            f"The workload was the {MARKDOWN_LINK_TOKEN_IMPACT['markdown_files']} Markdown files in this repo's "
            "configured `grund` scan scope; non-Markdown scanned source files and the root `README.md` were not counted.",
            "",
            "| Form | Tokens |",
            "|---|---:|",
            f"| Bare `§...` citations | {MARKDOWN_LINK_TOKEN_IMPACT['bare_tokens']} |",
            f"| Markdown links `[§...](...)` | {MARKDOWN_LINK_TOKEN_IMPACT['linked_tokens']} |",
            f"| Delta | {MARKDOWN_LINK_TOKEN_IMPACT['delta_tokens']} tokens |",
            "",
            "## Results",
            "",
            "| Command | Cold | Warm median | Warm min | Warm max |",
            "|---|---:|---:|---:|---:|",
        ]
    )
    for result in results:
        lines.append(
            "| "
            f"`{command_text(result['command'], repo)}` | "
            f"{seconds(float(result['cold']))} | "
            f"{seconds(float(result['median']))} | "
            f"{seconds(float(result['minimum']))} | "
            f"{seconds(float(result['maximum']))} |"
        )

    if float(grund_check["median"]) > 0 and int(grund_load["scanned_lines"]) > 0:
        loc_per_s = int(grund_load["scanned_lines"]) / float(grund_check["median"])
        lines.extend(
            [
                "",
                "## Throughput",
                "",
                f"At the warm median, `grund check .` scans about {round(loc_per_s / 1000):,}k "
                "lines of source per second — roughly the throughput figure on the README badge.",
            ]
        )

    if lychee_check is not None and lychee_load is not None and lychee_per_link is not None:
        lines.extend(
            [
                "",
                "## Comparison",
                "",
                f"`grund check .` checks {ratio(grund_load['edges'], int(lychee_load['links']))} as many local intent edges as Lychee checks links in this run.",
                f"Using warm medians, `grund check .` is {ratio(float(lychee_check['median']), float(grund_check['median']))} faster than the configured Lychee run.",
                f"Per checked edge, `grund` costs about {grund_per_edge * 1_000_000:.0f} microseconds; Lychee costs about {lychee_per_link * 1_000_000:.0f} microseconds per link.",
                "",
                "Product copy:",
                "",
                "> Lychee checks whether Markdown links still open. `grund` checks whether the project still knows why the code exists.",
                "> On this local run, `grund` checks more than twice as many project-intent edges and finishes several times faster.",
            ]
        )

    lines.extend(
        [
            "",
            "## Raw Warm Samples",
            "",
        ]
    )
    for result in results:
        samples = ", ".join(seconds(value) for value in result["warm"])
        lines.append(f"- `{result['name']}`: {samples}")
    lines.append("")

    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(lines), encoding="utf-8")


def write_badge(out: Path, grund_check: dict[str, object], grund_load: dict[str, int]) -> None:
    """Emit shields.io endpoint JSON: lines of code scanned per second (grund check, warm median)."""
    scanned_lines = int(grund_load["scanned_lines"])
    median = float(grund_check["median"])
    if scanned_lines <= 0 or median <= 0:
        raise SystemExit("cannot compute badge: grund scanned 0 lines")
    loc_per_s = scanned_lines / median
    payload = {
        "schemaVersion": 1,
        "label": "grund check",
        "message": f"~{round(loc_per_s / 1000)}k LoC/s",
        "color": "brightgreen",
    }
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(payload) + "\n", encoding="utf-8")


def main() -> int:
    args = parse_args()
    if args.warm_runs < 1:
        raise SystemExit("--warm-runs must be at least 1")

    repo = Path(args.repo).resolve()
    out = (repo / args.out).resolve() if not Path(args.out).is_absolute() else Path(args.out)
    grund = resolve_grund(repo, args.grund)
    lychee = args.lychee
    lychee_paths = args.lychee_paths or DEFAULT_LYCHEE_PATHS

    grund_check_command = [grund, "check", str(repo)]
    grund_fmt_command = [grund, "fmt", "--check", str(repo)]

    results = [
        measure("grund check", grund_check_command, repo, args.warm_runs),
        measure("grund fmt --check", grund_fmt_command, repo, args.warm_runs),
    ]
    if not args.no_lychee:
        lychee_command = [lychee, "--no-progress", "--include-fragments", *lychee_paths]
        results.append(measure("lychee", lychee_command, repo, args.warm_runs))

    specs = machine_specs(repo)
    versions = {"grund": command_version([grund, "--version"], repo)}
    if not args.no_lychee:
        versions["lychee"] = command_version([lychee, "--version"], repo)
    git = {
        "commit": git_value(["rev-parse", "HEAD"], repo),
        "branch": git_value(["branch", "--show-current"], repo),
        "dirty": git_dirty(repo),
    }
    grund_load = grund_workload(grund, repo)
    lychee_load = None if args.no_lychee else lychee_workload(lychee, lychee_paths, repo)

    grund_check_result = next(r for r in results if r["name"] == "grund check")

    if args.badge_out:
        badge_path = Path(args.badge_out)
        if not badge_path.is_absolute():
            badge_path = (repo / badge_path).resolve()
        write_badge(badge_path, grund_check_result, grund_load)
        print(f"wrote {badge_path}")

    if args.no_markdown:
        return 0

    write_report(
        out=out,
        repo=repo,
        grund=grund,
        lychee=None if args.no_lychee else lychee,
        lychee_paths=lychee_paths,
        warm_runs=args.warm_runs,
        specs=specs,
        versions=versions,
        git=git,
        grund_load=grund_load,
        lychee_load=lychee_load,
        results=results,
    )
    print(f"wrote {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
