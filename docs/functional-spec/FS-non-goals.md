# FS-non-goals: what grund will deliberately not do

This spec exists to prevent feature creep. Every entry below is a thing `grund` could plausibly grow into and is **deliberately out of scope**. When a contributor proposes one of these, the answer is "no" — not because it would be hard, but because the answer was decided.

A non-goal is not the same as "we'll do it later." Non-goals are commitments. To turn one into a goal requires a decision record under `docs/decisions/architectural/` overturning this spec, with a clear rationale.

## 1. Markdown link validation

`grund` does **not** validate `[text](url)` links, anchor `#section` references inside markdown, or HTTP URLs. Use [`lychee`](https://github.com/lycheeverse/lychee) for those — it is fast, focused, and well-maintained. Reasoning: there is no token-cheap reason to merge two lints into one binary.

## 2. Spelling, grammar, prose quality

`grund` reads spec text as opaque content between IDs. It does not lint English. Use any general-purpose linter — `vale`, `ltex-ls`, or a thousand others — alongside `grund`.

## 3. Code AST parsing

`grund` does not parse code. It does line-oriented regex over comments and doc-comments (per [AR-scanner](../architecture/AR-scanner.md#ar-scanner-how-grund-discovers-declarations-and-citations)). It does not understand classes, methods, types, scopes, or call graphs. The stub-heading link is a file path, not a symbol reference. Reasoning: [§GOAL-fast-feedback](../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible) rules out per-language parsers, and IDs are syntactic by design.

## 4. Cross-workspace ID renaming

`grund` does not provide a "rename ID" refactoring. The reference scheme says IDs are forever; renaming an ID is a deliberate edit (`Supersedes:` chain), not an automated operation. The optional LSP server ([§FS-lsp](FS-lsp.md#fs-lsp-grund-ships-an-optional-lsp-server)) intentionally omits this affordance, and no first-party editor wrapper would add it ([§FS-non-goals.12.1](FS-non-goals.md#121-plugins-or-scripting-hooks-inside-the-engine)).

## 5. Documentation generation

`grund` does not generate HTML, PDF, or any rendered documentation from specs. It reads, validates, and slices spec content; it does not publish. Static-site generators are the right tool for that downstream — `grund <ID>` is a building block they can use.

## 6. Decision database, audit log, history tracking

`grund` does not store decisions in a database, render decision graphs over time, track who changed what when, or visualize the ID graph. Git is the audit log; `git log` is the time machine. The reverse-lookup the project does ship — `grund refs <ID>` ([§FS-refs](FS-refs.md#fs-refs-grund-lists-every-citation-of-an-id)) — answers "who cites this ID *now*" from a single scan; it is a query over the current tree, not a stored graph or a history view.

## 7. Inter-agent messaging or workflow

`grund` is a checker and a retriever. It does not coordinate agents, queue work, route messages, or implement any kind of state machine. Tools like `rhei` exist for that.

## 8. Generalization beyond the ID scheme

`grund` does not validate other kinds of references — RFC numbers in random codebases, bug tracker IDs, etc. — outside the configured `[[kinds]]`. If a project does not adopt the ID scheme, `grund` has nothing to offer. Reasoning: see [§GRUND-grund](../grund.md#grund-grund-agents-stay-grounded-in-the-spec) — generalization dilutes the promise without expanding the audience.

## 9. Severity, exit code, or report-ordering customization

Per [§GOAL-friendliness-first.2](../goals.md#2-what-this-rules-out) and [§FS-config.6](FS-config.md#6-what-is-not-configured-here), the severity model (`error`/`warning`), the exit-code mapping (`0`/`1`/`2`), and the deterministic report ordering are **not** configurable. Reasoning: two correctly-configured `grund` installs must agree on whether a repo passes. Letting any of these vary by project breaks that contract.

## 10. Interactive mode

`grund` does not have a TUI, an interactive prompt, or a confirmation step. Every subcommand is non-interactive and CI-friendly. Reasoning: [§GOAL-friendliness-first](../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible) — interactive flows block CI and complicate scripting.

## 11. Network access during a check

`grund check` performs no network I/O. There is no "fetch this URL," no "validate against a remote schema," no telemetry. The only filesystem access is reading the scanned tree. Reasoning: `grund` runs in CI, in pre-commit hooks, and on laptops offline; correctness must not depend on the network.

## 12. Surfaces outside `grund-core` and the LSP transport

`grund` ships exactly two kinds of surface over the engine: the bindings enumerated in [§FS-distribution](FS-distribution.md#fs-distribution-grund-distribution-targets) (cargo CLI, Node API, Python API, LSP server) and nothing else. Anything that would add a third — in-engine scripting, per-editor wrappers, marketplace plugins — is out of scope.

### 12.1 Plugins or scripting hooks inside the engine

`grund-core` is a library. Bindings ([§FS-distribution](FS-distribution.md#fs-distribution-grund-distribution-targets)) are first-party. There is no plugin system, no Lua hook, no JavaScript user script that runs during a check. Custom behavior is achieved by calling the API from your own code; the core stays small. Reasoning: a plugin surface multiplies attack surface, breaks reproducibility, and undermines "two installs agree."

### 12.2 First-party per-editor plugins

`grund` does not ship and does not maintain VSCode extensions, IntelliJ plugins, Vim/Neovim plugins, Emacs packages, or any other editor-specific wrapper. The optional LSP server ([§FS-lsp](FS-lsp.md#fs-lsp-grund-ships-an-optional-lsp-server)) is the only first-party editor surface; configuring an editor to talk to it is the user's one-time work, with example snippets in the README. Reasoning: per-editor plugins multiply maintenance surface across release cadences we do not control (marketplace review, extension manifests, native UI APIs), and the LSP protocol already gives every modern editor the four capabilities `grund-lsp` exposes ([§FS-lsp.1](FS-lsp.md#1-capabilities)) for free. Decided in [§DA-lsp-optional](../decisions/architectural/DA-lsp-optional.md#da-lsp-optional-lsp-server-ships-as-a-separate-optional-binary). Reconsidering this entry would require an architectural decision record overturning [§DA-lsp-optional](../decisions/architectural/DA-lsp-optional.md#da-lsp-optional-lsp-server-ships-as-a-separate-optional-binary).

## 13. Anything that would let two `grund` installs disagree

Above all: any feature that could cause two correctly-configured `grund` installs — same version, same `grund.toml` — to disagree on whether a given repo is well-formed is permanently out of scope. This is the bright line.

## 14. Generated summaries; token-saving inside `check`

The token-cheap read modes — the default `grund <ID>` lead slice ([§FS-show.2.1](FS-show.md#21-whole-declaration-default)), `grund <ID> --brief` ([§FS-show.2.1.1](FS-show.md#211-brief---brief)), `grund <ID> --toc` ([§FS-show.2.1.2](FS-show.md#212-section-map---toc)), `grund refs <ID> --summary` ([§FS-refs.3.3](FS-refs.md#33---summary)), `grund list --summary` ([§FS-list.3.3](FS-list.md#33---summary)) — are *structural slices* of the heading tree and the citation graph: heading lines, lead prose, per-file counts. There is no model-generated summary, no paraphrase, no "TL;DR" — a slice is always a verbatim subset of bytes `grund` already parsed, so it stays byte-deterministic on `(tree, config)` like every other output (§13, [§FS-errors.4](FS-errors.md#4-determinism)). And the token economy applies only to the read/query surface: `grund check` diagnostics are never abridged — every dangling reference, every warning, in full located form ([§FS-errors.2.1](FS-errors.md#21-located-finding)) — because a check that hides findings to save tokens would defeat the point. Reasoning: `grund`'s value is that the same input gives the same answer everywhere; a generative summarizer is the opposite of that, and a quiet checker is worse than a verbose one. Decided in [§DF-show-default-token-cheap](../decisions/functional/DF-show-default-token-cheap.md#df-show-default-token-cheap-grund-show-defaults-to-the-cheap-read-the-full-body-is-opt-in), carrying forward the structural-slices rule from [§DF-show-token-cheap-reads](../decisions/functional/DF-show-token-cheap-reads.md#df-show-token-cheap-reads-grund-show-keeps-the-full-body-default-token-cheap-slices-are-opt-in).
