# Recipe authoring

A recipe is a delivery strategy written as reviewable data. Compile it before
running it; compilation resolves composition and agent references, enforces the
closed vocabularies, and prints the content-addressed manifest.

```console
$ brokkr recipes list
$ brokkr compile --bundle recipes/triage
$ brokkr recipes show triage
```

The shipped choices are `fast` for the on-screen quickstart, `triage` for
routed delivery, `night-shift` for the same routing with one-attempt limits and
a dsh implement seat, `panel-review`, `node`, `preflight`, and the two explicit
wager harnesses. The wager harnesses extend `fast` because a wager must force
the crew. Costs are relative bands, not provider quotes.

## Bundle shape

`bundle.json` names the recipe, its policy, and one seat per non-terminal
phase. A seat declares results, optional inputs/secrets/limits, and one body:

- a single `agent`, or inline `role` plus `driver`;
- a `panel` with a named `aggregate`;
- a named `sequence` of single or panel steps;
- a strategy `select` whose cases are complete bodies of one of those three
  shapes.

The selector vocabulary is deliberately closed:

```json
{
  "results": ["clean", "residual", "security-hold"],
  "select": {
    "on": "strategy",
    "cases": {
      "chore": {"agent": "reviewer", "class": "gate"},
      "feature": {
        "aggregate": "review-panel",
        "panel": {
          "correctness": {"agent": "review-correctness", "class": "gate"},
          "security": {"agent": "review-security", "class": "gate"}
        }
      },
      "design": {"sequence": ["… full named steps …"]},
      "engine": {"sequence": ["… full named steps …"]}
    }
  }
}
```

Only `strategy` is admitted by `on`. Every one of `chore`, `feature`,
`design`, and `engine` must resolve to a named case or `default`. The compiler
runs the same agent resolution, trust tier, pin, egress, hands, and judge checks
over every case. A run with no triage result takes `default`; without one it
parks rather than guessing. The selected case is recorded on `phase/entered`
and rebuilt from the journal on resume.

The manifest's optional `select` map shows every case and the agent(s) each
resolved to. It is absent when no site selects, preserving old identities.

## Panels and sequences

`unanimous-pass` emits `pass` only when every member passes. `review-panel`
uses worst-member-wins over clean, residual, and security-hold while joining
the typed review inputs. Panels may not mix work and gate members.

Sequence order is load-bearing. A non-final result becomes context for the
next step; only the final result reaches the policy table. This means
untrusted model prose is now input to the prompt of the seat that rules. A
chief must treat panel notes as data, never instructions, and must not lower
the panel's floor. The engine and design cases in `recipes/triage/bundle.json`
are the worked examples; the deterministic spec-kit check remains inline.

## Composition

`extends` resolves base-first into one pinned bundle. Redefinition is explicit:

```json
{
  "name": "derived",
  "extends": "triage",
  "override": {"cases": ["review:feature"]},
  "seats": {
    "review": {"select": {"cases": {"feature": {"agent": "reviewer", "class": "gate"}}}}
  }
}
```

Use `override.seats` for a whole seat, `override.cases` for one selector case,
and the existing `override.rules`, `override.table`, and `override.bundle`
markers for their named members. `override.limits` changes only a seat's
attempt/deadline object; `night-shift` is the shipped example. Stale markers
are refused because they would lie about what composition replaced.

Relative role and command paths resolve against the layer that wrote the seat
or selected case. Paths inside a hands box are always POSIX strings joined
with `/`, independent of the host.

## Policy and result provenance

Every declared result needs an outer rule. Every rule input must be either
engine-owned or declared by the producing seat. Engine-owned facts such as
`strategy`, visit counts, repository heads, and drift cannot be declared or
claimed by a seat. Gate sites hire only adapter-declared judges and are checked
for a stable repository head around the effect.

Run `cargo test --workspace`, formatting, clippy with warnings denied, and the
exact coverage gate after changing compiler or engine semantics. Version a
contract beside its predecessor; never edit frozen contracts, evaluator
fixtures, or the production phase table.
