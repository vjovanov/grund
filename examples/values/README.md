# Values: one declaration, checked uses

This runnable mini-repository demonstrates [§FS-values](../../docs/functional-spec/FS-values.md#fs-values-opted-in-kinds-bind-authored-components-to-one-declared-value): a `CONST` kind with Markdown and JSON declarations, exact numeric equality, prose and Python-comment bindings, direct application reads of the JSON source, the deliberate unbackticked non-binding, and one caught mismatch.

Run it from the repository root:

```bash
grund examples/values/repo
echo $?    # 1: the intentional discount mismatch is caught
```

The field-price bindings use `1200.0` and `1.2e3` for a declaration written as
`1200`; exact decimal comparison accepts both. The discount binding deliberately
writes `0.30` against JSON `0.25`, producing the golden `value-mismatch` below.
Change it to `0.25` and the repository prints `success`.

The nearby unbackticked `1200 (§CONST-field-price.1)` is an ordinary citation,
not inferred value syntax. This keeps intent explicit and avoids guessing which
number in a sentence belongs to a citation.
