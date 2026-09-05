# Parallel Spec Review Template

This template creates a directory workspace that reviews one specification with
multiple configurable reviewer agents in parallel. A single configurable
summary agent, Claude by default, synthesizes the review notes into attributed
weighted findings. A deterministic conflict check gates unresolved conflicts to
a human before a single configurable fix agent updates the spec. The loop is
bounded by the configured iteration count.

## Inputs

| Name | Type | Default | Description |
| --- | --- | --- | --- |
| `plan_title` | string | `Parallel Spec Review` | Title for the rendered workspace. |
| `spec` | path | required | Specification file to review and update. |
| `review_focus` | string | empty | Optional extra review focus or acceptance criteria. |
| `iterations` | number | `2` | Maximum review-summary-fix cycles. |
| `review_agents` | array<object> | Claude, Codex, Gemini | Parallel reviewer targets. Each entry has `id`, `label`, and `selector`. |
| `summary_agent` | string | `claude-code[yolo]:anthropic:claude-opus-4-7` | Single target that writes the weighted synthesis. |
| `fix_agent` | string | `codex[yolo]:openai:gpt-5-codex` | Single target that applies fixes from the synthesis. |
| `review_output_dir` | string | `runtime/reviews` | Directory for per-agent review notes. |
| `summary_output_dir` | string | `runtime/summaries` | Directory for synthesized summaries. |
| `fix_output_dir` | string | `runtime/fixes` | Directory for fix notes. |

## Task Kinds

| Task kind | State path | Notes |
| --- | --- | --- |
| `review-spec` | `review -> summarize -> conflict-check -> fix` | The normal path when the summary says `Conflict: no`. |
| `review-spec` with conflicts | `review -> summarize -> conflict-check -> human-conflict -> fix` | `human-conflict` is a gating state; agents stop until a human edits the latest summary and transitions the task. |
| repeated review | `fix -> review` | The loop continues while `visitCount < visits`. |
| complete | `fix -> completed` | The task finishes when `visitCount >= visits`. |

See [states.yaml](states.yaml) for the full state diagram, including the
`all_targets` reviewer fan-out.

## Flow

1. The `review` state fans out across every configured `review_agents[].selector`
   and writes one weighted review note per target.
2. The `summarize` state runs only `summary_agent` and writes a synthesis that
   preserves who mentioned each issue and the weight each reviewer assigned.
3. The `conflict-check` program reads the summary's exact `Conflict: yes|no`
   marker.
4. If the marker is `yes` or missing, `human-conflict` pauses execution until a
   human resolves the summary and transitions the task.
5. The `fix` state runs only `fix_agent`, updates the spec from the fix
   directives, and either starts another iteration or completes.

## Instantiate

```bash
rhei instantiate parallel-spec-review \
  --set spec=docs/functional-spec/FS-check.md \
  --set review_focus="Check consistency with CLI error handling and examples." \
  --set iterations=2 \
  --output examples/parallel-spec-review-example
```

Run the rendered workspace with enough parallel slots for the reviewer fan-out:

```bash
rhei run examples/parallel-spec-review-example --parallel 3
```

For custom reviewers, use a values file:

```yaml
review_agents:
  - id: claude
    label: Claude review
    selector: claude-code[yolo]:anthropic:claude-opus-4-7
  - id: codex
    label: Codex review
    selector: codex[yolo]:openai:gpt-5-codex

summary_agent: claude-code[yolo]:anthropic:claude-opus-4-7
fix_agent: codex[yolo]:openai:gpt-5-codex
iterations: 3
```

Checked-in example: [examples/parallel-spec-review-example](../../../../examples/parallel-spec-review-example).
