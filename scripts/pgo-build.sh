#!/usr/bin/env bash
set -euo pipefail

# Profile-guided-optimization build of the `grund` release binary (§DA-pgo-release).
# This is a release/benchmarking tool, not part of the normal development build
# or push/PR CI loop.
#
# Three phases: build an instrumented binary (`-Cprofile-generate`), run the
# §AR-benchmarks self-repo workload against this repo's own conformant tree to
# produce `.profraw` profiles, merge them, then rebuild the release binary with
# `-Cprofile-use`. The training workload is deliberately the same hot command
# list `crates/grund-cli/benches/instructions.rs` benchmarks. PGO trains on this
# repo (a representative real workload); the benchmarks run the same commands
# against generated fixtures (a stable regression-gate input). The generated
# `check_large_10k` benchmark is a CI budget input, not release-profile training
# data.
#
# Output: target/release/grund, optimized against the merged profile.
# Requires: the `llvm-tools-preview` rustup component (`llvm-profdata`).
#
# `cargo install grund` from source does not run this — a plain `cargo build
# --release` has no profile to use. The release pipeline (§RM-distribution) runs
# this script to produce the distributed binary; benchmarking can also run it
# when comparing the optimized release artifact. The napi-rs / PyO3 builds get
# the same treatment when they land.

cd "$(dirname "$0")/.."
repo="$PWD"
pgo_dir="$repo/target/pgo-data"
profdata="$pgo_dir/merged.profdata"
host="$(rustc -vV | awk '/^host:/ { print $2 }')"

rustc_path() {
  local path="$1"
  if [[ "$host" == *windows* ]] && command -v cygpath >/dev/null 2>&1; then
    cygpath -m "$path"
  else
    printf '%s\n' "$path"
  fi
}

# llvm-profdata ships in the active toolchain's llvm-tools-preview component.
llvm_profdata="$(find "$(rustc --print sysroot)" -type f -name 'llvm-profdata*' | head -n1)"
if [ -z "$llvm_profdata" ]; then
  echo "error: llvm-profdata not found — run: rustup component add llvm-tools-preview" >&2
  exit 1
fi

rm -rf "$pgo_dir"
mkdir -p "$pgo_dir"

pgo_dir_rustc="$(rustc_path "$pgo_dir")"
profdata_rustc="$(rustc_path "$profdata")"

echo "==> 1/3  build instrumented binary (-Cprofile-generate)"
RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-Cprofile-generate=$pgo_dir_rustc" cargo build --release --locked

exe_suffix=""
case "$host" in
  *windows*) exe_suffix=".exe" ;;
esac
grund="$repo/target/release/grund$exe_suffix"

echo "==> 2/3  training run — the §AR-benchmarks workload"
# Keep this self-repo hot command list in sync with crates/grund-cli/benches/instructions.rs.
# Exit codes are irrelevant here (a non-canonical tree makes `fmt --check` exit
# 1); we only want the code paths exercised.
set +e
for _ in 1 2 3; do
  "$grund" check "$repo"                   >/dev/null 2>&1
  "$grund" list "$repo"                    >/dev/null 2>&1
  "$grund" show FS-check --brief "$repo"   >/dev/null 2>&1
  "$grund" show FS-check "$repo"           >/dev/null 2>&1
  "$grund" show FS-check --full "$repo"    >/dev/null 2>&1
  "$grund" refs GOAL-fast-feedback "$repo" >/dev/null 2>&1
  "$grund" cover "$repo"                   >/dev/null 2>&1
  "$grund" fmt --check "$repo"             >/dev/null 2>&1
done
set -e

# Fail loudly if the training loop produced no profiles — every command above
# was wrapped in `set +e`, so a totally-broken `$grund` invocation would
# otherwise be hidden until `llvm-profdata` errored on an empty input.
shopt -s nullglob
profraws=("$pgo_dir"/*.profraw)
if [ ${#profraws[@]} -eq 0 ]; then
  echo "error: PGO training produced no .profraw files in $pgo_dir" >&2
  echo "       (the instrumented '$grund' did not run successfully)" >&2
  exit 1
fi

"$llvm_profdata" merge -o "$profdata" "${profraws[@]}"

echo "==> 3/3  rebuild optimized (-Cprofile-use)"
RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-Cprofile-use=$profdata_rustc -Cllvm-args=-pgo-warn-missing-function" cargo build --release --locked

echo "==> done: $grund"
"$grund" --version
