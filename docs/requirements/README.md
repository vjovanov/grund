# Requirements

What `grund` must never break — the hard invariants a release is blocked on — plus the contracts the repository's own surfaces keep. Each requirement lives in its own file; the H1 declares a `REQ-<slug>` ID.

## The tool

- [§REQ-backwards-compatibility](REQ-backwards-compatibility.md#req-backwards-compatibility-an-upgrade-never-changes-a-verdict-quietly) — a passing tree keeps passing across upgrades
- [§REQ-no-missed-citation](REQ-no-missed-citation.md#req-no-missed-citation-every-citation-the-run-reads-is-checked) — every citation is seen; false negatives are bugs
- [§REQ-no-wrong-citation](REQ-no-wrong-citation.md#req-no-wrong-citation-a-citation-never-resolves-to-a-guess) — no guessed resolution, no false alarms
- [§REQ-no-data-loss](REQ-no-data-loss.md#req-no-data-loss-grund-never-eats-user-content) — read commands never write; write commands touch only what they own
- [§REQ-deterministic-output](REQ-deterministic-output.md#req-deterministic-output-same-input-same-bytes) — byte-identical output across runs, platforms, and bindings
- [§REQ-never-crashes](REQ-never-crashes.md#req-never-crashes-garbage-in-diagnostic-out) — located errors and truthful exit codes, never a panic

## This repository

- [§REQ-readme](REQ-readme.md#req-readme-the-readme-is-the-grounded-shop-window) — the README: real examples, checked citations
- [§REQ-agents-md](REQ-agents-md.md#req-agents-md-the-agent-entrypoint-stays-managed-and-grounded) — the agent entrypoint: managed block, grounded rules

This index is navigational only. Citations should target the requirement ID directly, never this file.
