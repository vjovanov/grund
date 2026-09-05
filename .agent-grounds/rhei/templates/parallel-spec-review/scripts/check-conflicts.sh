#!/usr/bin/env bash
set -euo pipefail

summary_path="${1:-}"

if [[ -z "$summary_path" || ! -f "$summary_path" ]]; then
  exit 1
fi

if grep -Eiq '^[[:space:]]*Conflict:[[:space:]]*yes[[:space:]]*$' "$summary_path"; then
  exit 2
fi

if grep -Eiq '^[[:space:]]*Conflict:[[:space:]]*no[[:space:]]*$' "$summary_path"; then
  exit 0
fi

exit 1
