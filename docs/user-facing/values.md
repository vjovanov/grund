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
format = "{kind}-{slug}"
values = true
```

The per-kind `format` override ([§FS-config.3.4.10](../functional-spec/FS-config.md#3410-format-resolve-and-fetch--external-snapshot-kinds)) keeps these slug-only IDs valid.

The key is absent and false by default. There is no `value_sources` key, flag,
or new command. Repositories without an enabled row keep their previous reads,
diagnostics, output, and exit status.

An explicit `[[kinds]]` list replaces the implicit default kinds
([§FS-config.3.4.4](../functional-spec/FS-config.md#344-the-default-kinds)). If the project
has no kinds table yet, first copy the effective default rows from that section
before adding `CONST`; otherwise existing declarations disappear from `list`
and `check` remains green because those kinds no longer exist.

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

## Mark a value inside any declaration

When the surrounding declaration should stay prose, end one citable numeric
section heading with one ASCII space and the exact lowercase
`<!-- grund:value -->` marker. This independently authorizes that section; no
`values = true` kind setting is needed ([§FS-values.1](../functional-spec/FS-values.md#1-per-kind-opt-in-and-identity), [§FS-values.2.4](../functional-spec/FS-values.md#24-embedded-section-value-roots)):

```markdown
# FS-pricing: Pricing rules

## 2. Regional floor <!-- grund:value -->
### 2.1. 1200
### 2.2. USD

## 3. Use

The floor is `1200.0` (§FS-pricing.2.1).
```

The marked root may sit at any numbered depth. Relative to it, the shape is
strict: exactly one nonempty, physically ordered level `.1` through `.N`, with
no gaps, duplicate coordinates, named or plain children, grandchildren, lead
prose, or component bodies. Each component uses the same one-line title grammar
as a whole-declaration Markdown value. Blank lines are harmless. A nested mark
invalidates both overlapping roots; a mark inside an opted-in whole value is
invalid because the whole declaration already owns its fields.

The same authored headings work inside configured source comments and enabled
Python docstrings. The source wrapper is outside the heading depth:

```python
# FS-python: Python defaults
# ## 1. Retry limit <!-- grund:value -->
# ### 1.1. 3
# ## 2. Use
# The configured default is `3` (§FS-python.1.1).
```

`//`, `///`, `//!`, `;`, `--`, `/* ... */` / `*` Javadoc or JSDoc lines, and
both Python triple-quote delimiters follow the same rule. Lookalike marker
spellings are inert prose. An exact marker on a nonnumeric heading is a located
`invalid-value-declaration` instead of guessed authority.

The root and components keep their existing dotted section identities.
`show` returns the raw section slice (including the marker), `refs` and `cover`
count the binding citation once, completion offers the same section paths, and
the LSP navigates to the component heading. `list` keeps one enclosing row and
adds `[value roots: FS-pricing.2]` in text or an optional `value_roots` member in
NDJSON ([§FS-values.6](../functional-spec/FS-values.md#6-shared-catalog-consumers), [§FS-values.7](../functional-spec/FS-values.md#7-workspaces-and-editor-consumers)). `fmt --cross-refs` preserves marker and binding bytes so another pass cannot disable comparison ([§FS-values.8](../functional-spec/FS-values.md#8-formatting-stability)).

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
then a parenthesized, marker-prefixed citation with a positive numeric component
path, all on one physical line:

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

For an embedded value, the citation names the marked root path plus its one
immediate component, such as `<§>FS-pricing.2.1`. A binding aimed at the root
itself or below a component is invalid; the same delimited shape aimed at an
ordinary unmarked dotted section remains ordinary prose and a citation.

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

Whole and embedded values share the ordinary declaration and section catalog.
JSON declarations appear in `list`, completion, collision checks, unused checks,
indexes, and navigation; embedded roots add metadata to the enclosing `list`
row. Binding citations appear in `refs` and `cover`. `id` notices JSON collisions
but never writes JSON. `show` returns the exact source slice rather than
synthesizing Markdown.

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
