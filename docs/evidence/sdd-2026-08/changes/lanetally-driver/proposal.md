# Change Proposal: lanetally-driver

## Why

Seats launched through the `claude` driver report only the harness's
list-price `total_cost_usd`; nothing prices a session at real marginal
cost. LaneTally's session-capture wrapper (`claude-lanetally`, argv-
compatible with `claude` including `--output-format stream-json`)
records every session in LaneTally's cost ledger, where
subscription-vs-API accounting makes it priceable. A fifth built-in
forge-driver/v1 adapter, `brokkr driver lanetally`, runs the SAME Claude
Code harness through that wrapper — a decision-0008 fleet extension
inside delivered scope (no new decision doc) — so seat sessions become
ledger-priceable, and one constant checkpoint field lets `brokkr costs`
and the UI tell captured sessions apart.

## What Changes

- **Adapter**: `AdapterKind::Lanetally`, parsed from `"lanetally"`,
  `driver_name() = "claude-lanetally"` (parse token and driver name
  already diverge for claude; the ledger discriminator is the `capture`
  field, never the forgeable step name). Binary from
  `FORGE_LANETALLY_BIN`, default `claude-lanetally`. The claude arm of
  `invoke_with_stager` is factored verbatim into a private
  binary-parameterized helper — no `kind` in the shared body, live
  folding, stderr-drain thread, and decision-0001 noise pass-through
  preserved; `fold_stream_event` untouched; claude behavior
  byte-for-byte unchanged.
- **Capture constant**: the session-finished checkpoint gains
  `capture: "lanetally"` — a source-literal, kind-guarded insert in
  `run_seat` AFTER the `session_meta` extend, so no stream-derived key
  can shadow it (pinned by an adversarial `"capture":"evil"` shim and
  a claude no-`capture` negative assertion). Rides checkpoint `data`;
  frozen contracts v1 untouched. All checkpoint disciplines hold
  (file_path-only targets, 80-char clamps, decision-0012 masking via
  the shared `run_seat` choke point).
- **CLI + doctor**: unknown-driver error and `Driver` help list gain
  `lanetally`; doctor probes `claude-lanetally` present-or-advisory —
  a missing binary warns, naming `~/.local/bin/claude-lanetally` and
  the `FORGE_LANETALLY_BIN` override, never a hard failure; the four
  existing warning strings stay byte-identical.
- **Conformance**: lanetally joins the battery via the existing
  fake-harness technique (`FORGE_LANETALLY_BIN` pinned unconditionally
  in `drive()` so no test can reach a real wrapper); the obedient leg
  reuses `CLAUDE_STREAM_SHIM` verbatim as the argv-compatibility
  proof, asserting `capture:"lanetally"` and `total_cost_usd`
  flow-through; claude's finished checkpoint asserts no `capture`; a
  masking leg bakes store plaintext into result notes and asserts
  masked-token-present AND plaintext-absent (assertion only — zero new
  masking code); a `seat_costs` regression pins that `brokkr costs`
  needs zero changes.
- **Docs**: driver lists in `ARCHITECTURE.md` and
  `docs/extension-model.md` gain lanetally with one honest sentence:
  `total_cost_usd` stays harness-reported list price; LaneTally capture
  makes the session priceable in the LaneTally ledger; the per-session
  actual-cost join is deferred until readplane exposes a session query.

Design artifacts:

- [specs/lanetally-driver/spec.md](../../../specs/lanetally-driver/spec.md)
  — what and why: naming, the sharing invariants, the capture
  constant's integrity, masking boundary honesty, and the
  `## Acceptance Criteria`.
- [specs/lanetally-driver/plan.md](../../../specs/lanetally-driver/plan.md)
  — how: the panel-position reconciliation (ten explicit rulings),
  files touched in dependency order, risks with mitigations.
- [specs/lanetally-driver/tasks.md](../../../specs/lanetally-driver/tasks.md)
  — thirteen ordered tasks, each paired with the test that proves it.

## Impact

- **Edited**: `crates/brokkr-protocol/src/adapters.rs` (variant, helper
  factoring, capture insert, doc comment; + its tests),
  `crates/brokkr-cli/src/main.rs` (two strings),
  `crates/brokkr-cli/src/doctor.rs` + `doctor/tests.rs` (probe row with
  hint), `crates/brokkr-cli/tests/driver_conformance.rs` (battery
  membership, adversarial + masking legs), `crates/brokkr-cli/src/compare.rs`
  tests only (seat_costs regression), `ARCHITECTURE.md`,
  `docs/extension-model.md`.
- **New**: no production files; one conformance shim constant; the spec
  artifacts above.
- **Untouched**: `fold_stream_event`, the codex/dsh/exec arms and all
  drivers' observable behavior, secrets machinery (decision 0012 —
  demonstrated by assertion, not changed), `brokkr costs` aggregation,
  `ui.rs`/`ui.html`, `crates/brokkr-core`, frozen contracts v1,
  `policy/phase-machine.json`, `reference/`, the differential corpus,
  all recipes. No new dependencies, no HTTP client.
- **Operational**: on machines with LaneTally installed,
  `brokkr driver lanetally` seats land in the cost ledger automatically;
  on machines without it, doctor advises and the fleet works unchanged.
  `brokkr costs` output is unchanged except that captured sessions are
  distinguishable by `capture:"lanetally"`.
