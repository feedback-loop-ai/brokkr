# sdd-paranoid — SDD, with a different review panel

`extends: "sdd"`. Everything the SDD recipe rules — the intake seat, the
design council sequence, the implementer, the verifier, the shipper, and
the whole phase machine including the unavoidable review gate — is
inherited. This recipe states one difference: the review panel is
`adversarial + security` instead of `spec-compliance + security`, and it
says so with an explicit `override: { "seats": ["review"] }` so the
collision is deliberate rather than accidental (decision 0017).

## How small a difference is

| File | Lines |
|---|---|
| `recipes/sdd/bundle.json` | 227 |
| `recipes/sdd-paranoid/bundle.json` | 60 |

Sixty lines, of which fifty-three are the one seat being replaced. Before
decision 0017 the same change meant copying `recipes/sdd/` whole — 227
lines of `bundle.json`, 185 of `policy.json`, a `roles/` tree and a
`drivers/` tree — and the copy started drifting from its origin the
moment either changed. This recipe cannot drift: it has no policy table,
no intake role and no shipper role of its own, because it does not
redefine any of them.

## What the engine sees

Nothing of the above. Composition is resolved at compile time into one
flat bundle; there is no inheritance at run time and no dynamic lookup.
`brokkr compile --bundle recipes/sdd-paranoid` prints the resolved
result, and a `composed_from` chain naming `sdd` and its digest — the
same chain that rides in the run manifest under `@compose/`, so a run
states not just what it ran but what it was composed from. Change
`recipes/sdd`, and this recipe's digest moves: it is a different
strategy.

## Running it

```
forge run --recipe sdd-paranoid --feature "<the task>"
```

The panel is read-only, like SDD's. One `security-hold` from either
member stops the run; the aggregate takes the worst member verdict,
maxes severities and ORs the security flags.
