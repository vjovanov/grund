# Local benchmark report

This report is a local wall-clock snapshot for the `grund` repo. It complements the instruction-counting CI benchmark in [§AR-benchmarks](architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands) and the baseline work tracked by [§RM-benchmarks](roadmap.md#rm-benchmarks-a-benchmark-harness-for-the-goal-fast-feedback-budgets). It is meant for product-facing comparisons with Lychee; it is not the release-blocking regression meter.

## Instruction-Count Baseline

The release-blocking meter is Callgrind instruction count, not wall-clock time ([§DA-benchmark-instruction-counting](decisions/architectural/DA-benchmark-instruction-counting.md#da-benchmark-instruction-counting-the-performance-harness-counts-instructions-not-wall-clock-seconds)). Pull-request CI compares against the current base branch and fails when `Ir` grows by more than 5% ([§AR-ci.5](architecture/AR-ci.md#5-benchmark-job)); the table below is the committed human-readable snapshot from `cargo bench -p grund --features bench --locked --bench instructions -- --save-summary=json` on 2026-05-20.

| Benchmark | Input | Instructions (`Ir`) | Estimated Cycles |
|---|---|---:|---:|
| `check` | this repo | 299,672,739 | 431,943,109 |
| `check_large_10k` | generated 10k-file fixture | 1,055,099,244 | 1,475,491,249 |
| `list` | this repo | 289,680,670 | 417,771,050 |
| `show_brief` | this repo | 284,142,202 | 409,995,654 |
| `show` | this repo | 284,116,880 | 410,041,898 |
| `show_full` | this repo | 290,702,550 | 419,471,481 |
| `refs` | this repo | 284,135,903 | 410,102,297 |
| `cover` | this repo | 301,086,562 | 433,510,661 |
| `fmt_check` | this repo | 349,977,643 | 502,904,379 |

## Instructions

Regenerate this report from the repository root:

```sh
python3 scripts/local-benchmark-report.py --out docs/benchmarks.md
```

Useful options:

- `--warm-runs N` changes the number of warm samples after the cold run.
- `--grund PATH` points at a specific `grund` binary; by default the script uses `target/release/grund` when present.
- `--lychee PATH` points at a specific Lychee binary.
- `--lychee-path PATH` may be repeated to replace the default Lychee inputs: `README.md docs examples`.

For a fair local comparison, build the release binary first:

```sh
cargo build --release --locked
python3 scripts/local-benchmark-report.py --out docs/benchmarks.md
```

## Method

- Cold time is the first measured invocation of each exact command in this script run. The script does not use `sudo` and does not drop the OS page cache.
- Warm time is the median of 7 immediate subsequent invocations with command output suppressed.
- Timings use Python `time.perf_counter()` around the whole subprocess, so process startup and argument parsing are included.
- Lychee may perform URL work and may benefit from its own cache or network conditions; `grund` works only over the local scanned tree.

## Machine

| Field | Value |
|---|---|
| Timestamp | 2026-05-20T03:51:55+02:00 |
| Host | kung-fu-workstation |
| Os | Linux-6.17.0-29-generic-x86_64-with-glibc2.39 |
| Kernel | Linux kung-fu-workstation 6.17.0-29-generic #29~24.04.1-Ubuntu SMP PREEMPT_DYNAMIC Mon May 11 10:30:58 UTC 2 x86_64 x86_64 |
| Cpu | Intel(R) Core(TM) Ultra 9 285K |
| Logical Cpus | 24 |
| Memory | 93.7 GiB |
| Repo Disk Free | 13.2 GiB |
| Python | 3.11.5 |

## Tool Versions

| Tool | Version |
|---|---|
| `grund` | `grund 0.4.0` |
| `lychee` | `lychee 0.23.0` |
| Git commit | `2cd13fff1ee00a7decde6d8768531fdb3eb3a894` |
| Git branch | `rm-parallel-scan` |
| Working tree dirty | `no` |

## Workload

| Tool | Local work checked |
|---|---:|
| `grund check .` | 335 declarations + 2,184 citations across 98 scanned files (21,458 lines) |
| `lychee --include-fragments README.md docs examples` | 1,083 links across 80 markup files |

## Markdown Link Token Impact

Markdown cross-reference links are generated presentation over the underlying citation text ([§FS-fmt.6](functional-spec/FS-fmt.md#6-cross-reference-emission)). To quantify that prompt cost, this snapshot measured temporary copies under `/tmp/grund-link-impact` with `tiktoken` `o200k_base`. The workload was the 69 Markdown files in this repo's configured `grund` scan scope; non-Markdown scanned source files and the root `README.md` were not counted.

| Form | Tokens |
|---|---:|
| Bare `§...` citations | 154,594 |
| Markdown links `[§...](...)` | 185,370 |
| Delta | +30,776 tokens |

## Results

| Command | Cold | Warm median | Warm min | Warm max |
|---|---:|---:|---:|---:|
| `target/release/grund check .` | 0.032s | 0.030s | 0.029s | 0.034s |
| `target/release/grund fmt --check .` | 0.040s | 0.037s | 0.036s | 0.038s |
| `lychee --no-progress --include-fragments README.md docs examples` | 1.170s | 0.535s | 0.504s | 0.563s |

## Throughput

At the warm median, `grund check .` scans about 722k lines of source per second — roughly the throughput figure on the README badge.

## Comparison

`grund check .` checks 2.3x as many local intent edges as Lychee checks links in this run.
Using warm medians, `grund check .` is 18.0x faster than the configured Lychee run.
Per checked edge, `grund` costs about 12 microseconds; Lychee costs about 494 microseconds per link.

Product copy:

> Lychee checks whether Markdown links still open. `grund` checks whether the project still knows why the code exists.
> On this local run, `grund` checks more than twice as many project-intent edges and finishes several times faster.

## Raw Warm Samples

- `grund check`: 0.031s, 0.032s, 0.034s, 0.030s, 0.029s, 0.029s, 0.029s
- `grund fmt --check`: 0.037s, 0.038s, 0.038s, 0.036s, 0.037s, 0.036s, 0.036s
- `lychee`: 0.559s, 0.504s, 0.525s, 0.535s, 0.563s, 0.529s, 0.559s
