# REQ-no-wrong-citation: a citation never resolves to a guess

A citation resolves to exactly the declaration its ID names, or it is reported — never a silent best-effort pick, and never a false alarm.

## 1. No wrong resolution

Ambiguity is an error, not a choice: a number-only shorthand matching more than one declaration is rejected with the candidate list ([§FS-check.1.2](../functional-spec/FS-check.md#12-the-number-only-shorthand)), duplicate declarations are reported rather than ranked ([§FS-check.3.3](../functional-spec/FS-check.md#33-duplicate-declaration)), and a section coordinate resolves to the declaration's recorded heading or fails ([§FS-check.3.2](../functional-spec/FS-check.md#32-missing-section)). The resolver never substitutes a near miss — a "did you mean" hint is message text, and the citation still dangles and still fails the run.

Where a lookup *can* be satisfied two ways, the rule that picks must be written down. Silently preferring the first of two identical section paths is the shape of a guess even when it is deterministic.

## 2. No false alarms

A citation that resolves and is written in canonical form is never reported as broken. A checker that cries wolf trains its users to ignore it, which unwinds the whole loop ([§GOAL-agent-grounding](../goals.md#goal-agent-grounding-agents-stay-cited-as-they-work)), so a false positive is treated with the same severity as a false negative.

Three classes are deliberately *not* false alarms, because each reports something true about a citation that resolves. A **non-canonical form** is flagged for what it is, not for failing to resolve: a shorthand that resolves uniquely is still an error naming the form to write ([§FS-check.3.13](../functional-spec/FS-check.md#313-number-only-shorthand-citation)). A **policy** finding — the citation-direction rules a repository opted into — judges whether a citation should have been made, not whether it points at anything ([§FS-check.3.12](../functional-spec/FS-check.md#312-forbidden-citation)). And a finding the spec **knowingly scopes**, like the unused-declaration warning that `--full` leaves standing for a declaration cited only from outside `[scan] include` ([§FS-check.1.3](../functional-spec/FS-check.md#13-the-full-tree-scope---full)), is a stated cost of additivity rather than a mistake. Each must stay legible as such in its message; a true statement filed under the wrong name is how a report loses trust.

## 3. No wrong write

A persisted rewrite **is** a resolution, so §1 binds the writer exactly as it binds the reader. `grund fmt --write` expanding a shorthand does not report which declaration the token names — it *makes* the token name it, and the result is a well-formed citation of a real declaration that no later pass can tell apart from one the author wrote ([§FS-fmt.2.4](../functional-spec/FS-fmt.md#24-shorthand-to-canonical), [§DF-shorthand-numeric-run.2.7](../decisions/functional/DF-shorthand-numeric-run.md#27-invention-is-reported-in-full-whatever-the-rule-decides)). A guess the reader makes is a wrong answer to one question; a guess the writer makes is a wrong answer persisted into the tree, where it reads as ground truth and every subsequent run agrees with it.

So `fmt` may not write a rewrite that changes what an ID token says unless the declaration set it resolved against is proven complete. Where the proof is missing — a path in the walked tree that could not be read — nothing is rewritten and the run says so, rather than resolving against whatever it happened to see ([§FS-fmt.3](../functional-spec/FS-fmt.md#3-outputs), [§FS-fmt.7.4](../functional-spec/FS-fmt.md#74-no-write-without-a-complete-model)). Completeness is a precondition of the code that consumes the declaration set, not a promise each caller is trusted to keep: this rule has been stated in prose twice and broken by code written after each statement.

§2 binds the writer too, on the other side. A dry run listing a rewrite that `--write` will never perform is the write-side false alarm — a finding no edit can clear, on a tree whose gate can therefore never go green ([§FS-fmt.7.3](../functional-spec/FS-fmt.md#73-preview-equivalence)).
