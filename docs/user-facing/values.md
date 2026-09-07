# First-class values

First-class values make one numbered declaration component authoritative and
let `grund check` compare intentional uses against it. The complete contract is
[§FS-values](../functional-spec/FS-values.md#fs-values-opted-in-kinds-bind-authored-components-to-one-declared-value); this guide focuses on authoring and operation.

## Opt a kind in

Add `values = true` to a citable `[[kinds]]` row that has exactly one existing
`file` or `folder` home inside the project root:

```toml
[[kinds]]
kind = "CONST"
folder = "values"
index = false
values = true
```

The key is absent and false by default. There is no `value_sources` key, flag,
or new command. Repositories without an enabled row keep their previous reads,
diagnostics, output, and exit status.

## Declare values in Markdown

An ordinary declaration in the kind home becomes a value declaration. Its
immediate citable child headings must be one contiguous run from `.1` through
`.N`:

```markdown
# CONST-field-price: Reference field price
## 1. 1200
## 2. USD
```

Each component is the complete nonempty heading title after its coordinate. It
must stay on one line and may not have edge whitespace, a backtick, or a control
character. Gaps, duplicates, zero or leading-zero coordinates, named or nested
citable sections, and incorrect heading depth are
`invalid-value-declaration` errors. Lead prose, bodies, and plain non-citable
headings are ignored.

A Markdown component that completely matches JSON number grammar is numeric.
Every other valid component is a string.

## Declare values in JSON

JSON is discovered only at the enabled kind's existing home:

- A kind whose `file` ends in `.json` reads that file.
- A folder kind reads each direct `.json` child in normalized bytewise path
  order.
- Nested JSON, JSON outside the home, and a non-JSON `file` are not value
  sources.

Generic scan extensions, include/exclude and ignore filters, explicit command
paths, and `--full` cannot add or suppress these inputs. A home JSON file is
catalog input only and never contributes citations.

The root is a nonempty object. Every key is a full, unqualified ID of the owning
kind, and every value is a nonempty array of JSON numbers or strings:

```json
{
  "CONST-field-price": [1200, "USD"],
  "CONST-discount": [0.25, "%"]
}
```

Array element zero declares `.1`, element one declares `.2`, and so on. String
components follow the same nonempty, edge-whitespace, backtick, and control
character boundary as Markdown. Duplicate keys are preserved and reported as
ambiguous declarations; declarations duplicated across files or across JSON
and Markdown are ambiguous too. Read, UTF-8, or JSON syntax failures make the
scan incomplete and exit `2`; readable schema violations use
`invalid-value-declaration` and exit `1`.

Application code can read this JSON file directly. `grund` does not generate a
module or maintain a second artifact.

## Bind an authored component

The only compared form is a nonempty single-backtick literal, one ASCII space,
then a parenthesized, marker-prefixed citation with one explicit positive
numeric field, all on one physical line:

```markdown
The field price is `1200.0` (§CONST-field-price.1).
```

It is recognized in Markdown outside fences and wholly within scanned source
comment or doc-comment lines:

```python
# Model default: `1200` (§CONST-field-price.1)
field_price = values["CONST-field-price"][0]
```

The citation remains an ordinary citation for `refs`, `cover`, direction and
unused checks. It resolves locally or through the normal workspace alias path.
Unknown aliases, dangling IDs, duplicates, invalid declarations, missing
fields, and noncanonical shorthand are reported first and suppress comparison
at that site.

An unbackticked adjacent token and a bare citation are deliberately ordinary
prose/citations, not binding near-misses:

```text
1200 (§CONST-field-price.1)       # no backticks
§CONST-field-price.1             # bare citation
```

Once a backtick-delimited literal is adjacent to a value reference, malformed
spacing, punctuation, marker, or field syntax is an attempted binding and
produces `invalid-value-binding`:

```text
`1200` §CONST-field-price.1      # no parentheses
`1200` (CONST-field-price.1)     # no marker
```

The marker remains mandatory even when `[reference] strict = false`.

## Equality and diagnostics

When both components are numbers, comparison uses exact arbitrary-precision
decimal value: `1200`, `1200.0`, and `1.2e3` agree, and negative zero equals
zero. There is no floating-point conversion or exponent expansion.

Otherwise both components must be strings and decoded Unicode scalar sequences
must match exactly. Comparison does not trim, normalize case or Unicode, strip
separators, convert units, or coerce JSON strings to numbers. Consequently
`1,200`, `+1200`, and `1_200` are strings.

A mismatch is an exit-`1` `value-mismatch` at the binding and names the
declaration site. Text and NDJSON carry the same message and declaration site.
The other fixed exit-`1` codes are `invalid-value-declaration` and
`invalid-value-binding`; none is controlled by `strict` or `--suggestions`.

## Existing commands and editors

JSON values share the ordinary declaration and section catalog. They appear in
`list`, completion, collision checks, unused checks, indexes, and navigation;
binding citations appear in `refs` and `cover`. `id` notices JSON collisions but
never writes JSON. `show` returns the exact JSON member slice for an ID or exact
element slice for a field in every read mode rather than synthesizing Markdown.

The LSP uses the same report, hover slice, and definition spans as the CLI.
Go-to-definition lands on the Markdown component heading or exact JSON key or
element. `fmt --cross-refs` leaves the citation bytes inside a recognized
binding unchanged so it cannot destroy the authored form.

## Deliberate v1 boundary

V1 has no surrounding-prose inference, interpolation or rendered substitution,
bare-literal lint, range or unit semantics, arithmetic, generated code,
fingerprints or history, `reconcile` or `stale` verb, excluded-path hint,
value-aware search, generated-file policy, or orphan relief. Ranges and units
can be represented as successive components, but their meaning belongs to the
application rather than to `grund`.
