# DF-index-not-an-inbound-citation: an index entry is navigation, not use

**Status:** Accepted
**Date:** 2026-08-25

## 1. Context

An index row is an ordinary recognized citation. So the moment a kind's index is required to name every declaration in its folder ([§FS-check.4.6](../../functional-spec/FS-check.md#46-declaration-missing-from-its-kinds-index)), every one of those declarations acquires an inbound citation — by construction, from a file whose whole job is to name them.

The unused-declaration warning ([§FS-check.4.1](../../functional-spec/FS-check.md#41-unused-declaration)) and `grund list --unused` ([§FS-list.1](../../functional-spec/FS-list.md#1-inputs)) both answer "who points at this?" by counting inbound citations. Left alone, they would answer "the index does" for every ID in every indexed folder, which is the same as answering nothing. That signal is the one [§RM-gap-report](../../roadmap.md#rm-gap-report-orphan-and-uncovered-id-reports) is being built to sharpen, and this feature would have quietly emptied it.

Measured on this repository at v0.11.0 the effect was latent rather than live: across `FS`, `AR`, `REQ`, `DF` and `DA`, no ID was cited *only* by its index. `DISC` was the exception — two of the eleven proposals were named nowhere but `docs/discussions/README.md`, which is exactly the state this decision makes visible.

## 2. Decision

### 2.1 A kind's own index entry does not count as an inbound citation

For the unused-declaration warning and for `grund list --unused` only. `grund refs` is unchanged and still lists the index entry: it is a real citation, and a reader asking who points at an ID wants to be told that its index does.

### 2.2 The exclusion is the entry, not the file

A citation in an index file of an ID whose home lies *outside* that folder is ordinarily a reference and counts like any other. `docs/architecture/README.md` cites [§GOAL-fast-feedback](../../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible) in the row for the benchmark harness; that is a reference to a goal, made in prose the author chose to write, and nothing about the file it sits in makes it navigation. The exception is the exact canonical bare-ID link that enrolls an external inline declaration ([§DF-index-entry-form.2.7](DF-index-entry-form.md#27-a-canonical-bare-id-link-enrolls-an-external-inline-declaration)): that one site is the entry, while another citation of the same ID on the same page remains ordinary use.

This is the narrow reading of the same fact [§FS-check.4.6](../../functional-spec/FS-check.md#46-declaration-missing-from-its-kinds-index) states in the other direction: location alone never turns a citation into navigation. Folder membership or the external enrollment form decides it, and the latter is tracked by exact site rather than by ID so it does not swallow surrounding references.

### 2.3 `refs` in `grund list --format json` stays the total

It is documented as the count of recognized citations across the scanned tree ([§FS-list.3.2](../../functional-spec/FS-list.md#32---format-json)), and `grund refs` lists exactly that set; two counts of "citations of this ID" that differ by which command printed them would be worse than one count that needs a sentence of explanation. The consequence — a declaration selected by `--unused` may carry a non-zero `refs`, namely its index entry — is stated where the field is defined.

## 3. Alternatives considered

**Exclude the whole index file.** Simpler to implement and wrong in the same way "any folder README is an index" is wrong: it would silently discount a citation an author wrote as a reference, because of where they wrote it.

**A designated region of the index — a managed block whose citations do not count.** That is the rendered-index design ([§DF-index-entry-form.3](DF-index-entry-form.md#3-alternatives-considered)) wearing a different hat, and it presumes the generator this check deliberately does not depend on. When the renderer lands, the region it owns is a natural refinement of §2.2, not a replacement for it.

**Leave the accounting alone and accept the noise.** The unused signal is not decoration: it is how a tree tells its maintainer that a spec, a decision, or a discussion has fallen out of use. Trading it for an index check would have been a net loss even if the check were free.

## 4. Consequences

Two `DISC` proposals in this repository stopped being cited the moment the exclusion landed — `DISC-external-ticket-resolvers` and `DISC-markup-format-declarations` — and were given real citations from the spec points they bear on rather than being left as warnings. That is the signal working on its first run.
