## Why

Decision 0045's addendum of 2026-09-30 records three operator rulings:
`sol` is Sol 6.1 (`gpt-6.1-sol`); Sol takes every seat Astra held, with
Astra left only as the last link of a chief's chain at `max`; and Sol's
effort is capped at `high`, one step down its old scale. The roster
moved with that ruling, and three capability specs still stated the
roster from before it.

## What Changes

- `astra-engine-smith`: the engine smith's ruled chain is `sol` at
  `medium` then `fable` at `high`, and the pending live measurement is
  Sol's. The capability keeps its name; its purpose says why.
- `boxed-work-provider-admission`: the shipped smith's Codex link is
  Sol's, launched at `medium`.
- `gate-boundary-policy`: the chief architect chains fable, sol, opus,
  astra, and `bundles/self`'s reviewer chains sol, fable, opus.

The archived changes and the unarchived change records keep what they
said when they were written. They are dated evidence.

## Impact

- No code moves with this fold. The roster, the recipes and the binding
  test `roster.rs::sol_is_capped_and_astra_is_a_chiefs_last_fallback`
  landed with the ruling.
- No contract, fixture, policy table or reference file moves.
