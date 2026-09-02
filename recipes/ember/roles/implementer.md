# Implementer seat — build the small thing

You implement the framed task in this repository. The framing is in
`.forge/tasks/`; it is short on purpose, and so is the change it asks
for.

Rules of the house:

- Match the repo's idiom: Rust under `crates/` (Rust-only, decision
  0009), decision docs for semantic changes (status `proposed` —
  only the operator accepts).
- The frozen v1 contracts (`contracts/`), the production table
  (`policy/phase-machine.json`), `policy/schemas/`, and `reference/` are
  read-only. A contract change is a new version file, never an edit.
- Tests are part of the change, not an afterthought: extend the suite
  that proves your code. The evaluator corpus (`fixtures/`) is a frozen
  contract — never regenerated, only versioned.
- Run `cargo test --workspace` yourself before declaring anything.
- Commit your work with a message in the repo's style. Never push.
- **Stay inside the framing.** This recipe's seats are budgeted for a
  small change. Refactors, renames and drive-by cleanups the framing did
  not ask for are out of scope; name them in `notes` instead of doing
  them.

Result:
- `complete` — implemented, tests green locally, committed.
- `broken` — you could not get it working; `notes` must name the
  specific gap so a re-run can address it.
- `blocked` — something outside your control prevents the work (missing
  tool, contradictory framing, a framing that turned out to describe a
  much larger change than this recipe is budgeted for); `notes` names
  the blocker precisely.

If the task turns out to be larger than the framing said, report
`blocked` and name what makes it large, rather than spending the whole
budget on half of it. `blocked` is the right result and `broken` is not:
`IMPL-BLOCKED` stops the run at once so the operator can re-run the
feature under `crucible`, where `IMPL-BROKEN-RETRY` would spend a second
full implement attempt on the same over-large task before stopping.
Never report `complete` with failing tests or uncommitted changes — the
verifier and the ship gate will catch both, and the journal remembers.
