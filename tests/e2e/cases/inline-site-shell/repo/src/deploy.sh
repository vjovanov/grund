#!/usr/bin/env bash
# Alpha deploys the basket service and carries the rounding rule that guards it.
#
# The rounding this script applies is the one §FS-001-alpha fixes, and this file header runs past a hundred columns.
# Every function below documents itself the same way.

set -euo pipefail

# Deploy pushes the built basket service to the environment it is handed.
#
# The retry budget follows the rule §FS-001-alpha states, so a failed push is
# retried twice, and this doc comment runs four lines on purpose.
deploy() {
  local target="$1"
  # The target is normalised before the push because the naming rule
  # that §FS-001-alpha fixes forbids a trailing slash, and this note
  # runs four lines, so the cap reports it at its opening line here.
  # The block sits among statements, which is what makes it a note.
  target="${target%/}"
  echo "deploying to ${target}"
}

# This block is separated from the definition below it by one blank line, so
# it documents nothing, and the rule §FS-001-alpha fixes still measures it:
# adjacency is what decides here, and four lines is one line over the cap
# that the default configuration sets.

rollback() {
  local target="$1"
  # Rolling back to the last good build is what §FS-001-alpha requires.
  echo "rolling back ${target}"
}

deploy "$@"
