# FS-examples: examples teach canonical user workflows

The `examples/` tree is a user-facing learning surface, not an incidental test-data dump. Its maintained examples explain the canonical ways users are expected to adopt and operate `grund`: choosing an ID scheme, declaring facts, citing them, resolving citations, checking a tree, and using reports during review. This serves [§GOAL-friendliness-first](../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible) by making the common path legible before a user reads the implementation or the e2e suite, and it serves [§GOAL-agent-grounding](../goals.md#goal-agent-grounding-agents-stay-cited-as-they-work) because agents can learn the intended workflow from maintained, runnable material.

## 1. Scope

An example is any maintained directory or document under `examples/` that is advertised from the README, from `examples/README.md`, or from a functional spec. Scratch fixtures, private experiments, and e2e-only test cases are not examples until they are linked from one of those user-facing surfaces.

## 2. Canonical use-cases

Every maintained example must map to at least one canonical `grund` use-case. The use-case must be named in the example's README or manifest in user terms, such as:

- choosing between supported ID schemes;
- setting up a conformant repository;
- declaring specs in Markdown or source doc-comments;
- citing specs from prose, code, or executable tests;
- resolving a citation with `grund <ID>`;
- finding a declaration's blast radius with `grund refs`;
- grouping citations by file with `grund cover`;
- normalizing citations with `grund fmt`;
- validating a clean or broken tree with `grund check`;
- validating cross-project citations in a workspace.

An example may cover multiple use-cases, but it must stay small enough that a new user can tell which workflow it is teaching without reading unrelated files.

## 3. Required explanation

Each maintained example must include a detailed explanation for users. At minimum it names the scenario, the intended audience, the files worth opening first, the commands to run from the repository root, the expected exit code and stream behavior, and the lesson the user should take from the output. If the example demonstrates a tradeoff, such as an ID scheme choice, the README must describe both the benefit and the cost in practical terms.

The explanation must be self-contained: a user should not need to inspect `tests/` or implementation code to understand why the example exists or how to run it. Links back to the relevant spec are allowed, but they supplement the explanation rather than replacing it.

## 4. Maintenance contract

The root `README.md` is the concise public guide to shipped behavior. Any functional change that affects users must update `README.md` alongside the most-specific functional spec point, so the advertised workflow and the grounded contract do not drift.

Runnable examples must stay executable and regression-tested. If an example has `expected.exit`, `expected.stdout`, and `expected.stderr`, those files are the golden contract for the documented command. A behavior change that affects a canonical workflow must update the corresponding example explanation and golden output in the same change.

When a new canonical workflow becomes part of the README or functional spec, the examples tree must either gain a maintained example for it or explicitly link to an existing example that already teaches it. Removing an example requires either removing the advertised workflow or replacing the example with an equivalent maintained path.

## 5. E2E reuse without duplication

Runnable examples must also be executable end-to-end tests. They may have a lighter manifest than `tests/e2e/cases/` when that keeps the user-facing directory readable, but their command invocation, expected exit code, stdout/stderr comparison, mutable-repo handling, and final-repo snapshot comparison must be run by the same test runner logic used for ordinary e2e cases.

The repo must not maintain a second, example-only implementation of the e2e contract. Adding a new e2e capability such as `command.args`, `{repo_copy}`, `expected.repo`, deterministic-output checks, or golden-output refresh must make that capability available to examples through shared code, not through a copied harness.
