# REQ-shipped-surfaces: what grund ships or prints resolves where it lands

`grund` writes files into other people's repositories and dotfiles, and prints text into terminals that are not this one. Those bytes leave this tree and are read where none of this tree's declarations exist, so a citation of `FS-config.3.9` in one of them points at nothing the reader has — or, worse, at an unrelated document of their own that happens to share a slug. That is the failure [§GOAL-no-dangling-refs](../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) exists to prevent, committed by the tool that checks for it, and it reaches the user at the least forgiving moment: the first file `grund init` asks them to edit, and the error message they get when it is wrong ([§GOAL-friendliness-first](../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible)).

## 1. No shipped or printed byte names a declaration of this repository

An ID declared in this tree may not appear in anything that leaves it. Where a sentence needs an address, it carries one the reader can open — the public `https://github.com/vjovanov/grund/blob/main/…` URL of the document — and where it does not, the parenthetical goes: a sentence that says what a section does has already said it.

The surfaces this covers are every byte a user receives without cloning this repository:

- the scaffold `grund init` writes ([§FS-init.2.1](../functional-spec/FS-init.md#21-files-written-updated-or-left-in-place)) and the entrypoint it generates ([§FS-init.2.3](../functional-spec/FS-init.md#23-generated-agent-entrypoints));
- the setup skill `grund agent-setup-instructions` prints byte-for-byte ([§FS-init.5](../functional-spec/FS-init.md#5-agent-setup-instructions)), and the canonical pages copied into it;
- the integration payloads `grund integrations --write` installs into a user's dotfiles ([§FS-integrations.4](../functional-spec/FS-integrations.md#4-managed-writes---write)), which are idempotent and therefore permanent;
- every string the frontends print — help text, reports, and diagnostics alike.

Illustrative IDs are not covered and never were: `FS-login`, `FS-014-user-login` and their kin teach the grammar and resolve to nothing here, so they name nothing they could be confused with.

## 2. The grounding moves, it is not deleted

Removing a citation from a payload is only half the fix. The sentence still has a reason, and that reason is still this repository's to record, so the `§` citation moves to the file that *owns* the shipped bytes — the module holding the `include_str!`, the spec point the behaviour belongs to — where it is checked like every other citation. A payload edit that drops a citation without rehoming it trades a wrong address for no address, which is the loss [§GRUND-understanding](../grund.md#grund-understanding-the-why-stays-known) is about.

This is also why a home whose files ship verbatim is configured `scan = false` ([§FS-config.3.4.7](../functional-spec/FS-config.md#347-scan--a-place-that-is-listed-not-walked)): the key exists for exactly this content, and it says in the config what this requirement says in prose — nothing in that directory is this repository's to ground.

## 3. Checked, not remembered

The rule holds because a test enforces it, not because an author remembers it. `tests/integration/test_shipped_surfaces.py` reads this repository's own declaration catalog from `grund list` ([§FS-list](../functional-spec/FS-list.md#fs-list-grund-lists-every-declared-id)) and fails if any ID in it appears in an embedded asset, a tree copy of one, an end-to-end output golden, or a string literal the binaries print. Matching against the catalog rather than the ID *shape* is what lets the illustrative IDs of §1 through: they are not declared, so they are not IDs.

A guard on the shape of the bytes is the only kind that works here. The adopting repository's own `grund check` catches a `§`-marked foreign citation the day it lands ([issue #56](https://github.com/vjovanov/grund/issues/56)), but an unmarked one is plain prose there and invisible here — the hole [issue #156](https://github.com/vjovanov/grund/issues/156) came in through.
