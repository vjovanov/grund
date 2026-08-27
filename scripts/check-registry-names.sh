#!/usr/bin/env bash
# Pre-release package-name guard for §FS-distribution.4.
# Claimed future package names must be either available or already owned by this
# repository. The bare "grund" name on npm — a dormant low-use squat
# (§DA-rename-to-grund / §DA-pypi-uses-grund-as-the-package-name) — is reported
# as a notice so the release manager can revisit FS-distribution if it changes.

set -euo pipefail

ua="grund-release-name-check/0.1"
repo_pattern='github.com[/:]vjovanov/grund'
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

http_get() {
  local url="$1"
  local out="$2"
  curl -sS -L -A "$ua" -o "$out" -w '%{http_code}' "$url"
}

metadata_mentions_repo() {
  local file="$1"
  python3 - "$file" "$repo_pattern" <<'PY'
import json
import re
import sys

path, pattern = sys.argv[1], sys.argv[2]
with open(path, "r", encoding="utf-8") as fh:
    data = json.load(fh)

haystack = json.dumps(data, sort_keys=True).lower()
sys.exit(0 if re.search(pattern, haystack) else 1)
PY
}

check_claimed_json_name() {
  local registry="$1"
  local name="$2"
  local url="$3"
  local out="$tmpdir/${registry}-${name}.json"
  local code

  code="$(http_get "$url" "$out")"
  case "$code" in
    200)
      if metadata_mentions_repo "$out"; then
        echo "ok: $registry/$name is owned by this project"
      else
        echo "error: $registry/$name is already taken by another project" >&2
        echo "       $url" >&2
        return 1
      fi
      ;;
    404)
      echo "ok: $registry/$name is available"
      ;;
    *)
      echo "error: could not query $registry/$name (HTTP $code)" >&2
      echo "       $url" >&2
      return 1
      ;;
  esac
}

notice_external_json_name() {
  local registry="$1"
  local name="$2"
  local url="$3"
  local out="$tmpdir/${registry}-${name}-external.json"
  local code

  code="$(http_get "$url" "$out")"
  case "$code" in
    200)
      if metadata_mentions_repo "$out"; then
        echo "notice: $registry/$name is owned by this project; docs may no longer need the alternate-name rationale"
      else
        echo "notice: $registry/$name is occupied by an external package as documented"
      fi
      ;;
    404)
      echo "notice: $registry/$name appears available; revisit the alternate-name rationale before publishing"
      ;;
    *)
      echo "warning: could not query documented external collision $registry/$name (HTTP $code)" >&2
      ;;
  esac
}

check_claimed_json_name "crates.io" "grund-core" "https://crates.io/api/v1/crates/grund-core"
check_claimed_json_name "crates.io" "grund" "https://crates.io/api/v1/crates/grund"
check_claimed_json_name "crates.io" "grund-lsp" "https://crates.io/api/v1/crates/grund-lsp"
check_claimed_json_name "npm" "grund-cli" "https://registry.npmjs.org/grund-cli"
check_claimed_json_name "npm" "grund-lsp" "https://registry.npmjs.org/grund-lsp"
check_claimed_json_name "pypi" "grund" "https://pypi.org/pypi/grund/json"
check_claimed_json_name "pypi" "grund-lsp" "https://pypi.org/pypi/grund-lsp/json"

notice_external_json_name "npm" "grund" "https://registry.npmjs.org/grund"
