# REQ-no-wrong-citation: a citation never resolves to a guess

A citation resolves to exactly the declaration its ID names, or it is reported — never a silent best-effort pick, and never a false alarm.

## 1. No wrong resolution

Ambiguity is an error, not a choice: a number-only shorthand matching more than one declaration is rejected with the candidate list (§FS-check.1.2), and a section coordinate resolves to the declaration's recorded heading or fails. The resolver never substitutes a near miss.

## 2. No false alarms

A correct citation is never flagged. A checker that cries wolf trains its users to ignore it, which unwinds the whole loop (§GOAL-agent-grounding); a false positive is treated with the same severity as a false negative.
