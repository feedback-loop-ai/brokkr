# Tasks: LaneTally driver

**Feature slug**: `lanetally-driver`
**Spec**: [spec.md](spec.md) · **Plan**: [plan.md](plan.md)

Ordered; each task names the test that proves it. Acceptance-criteria
numbers (AC-n) refer to spec.md's `## Acceptance Criteria`.

- [ ] **T1 — `AdapterKind::Lanetally` in
  `crates/forge-protocol/src/adapters.rs`.** Variant; `parse("lanetally")`;
  `driver_name() = "claude-lanetally"`; module doc comment env list
  gains `FORGE_LANETALLY_BIN`.
  *Proven by*: AC-1 unit tests beside the code (parse row, name row)
  and the `adapters_name_themselves…` row (T6).
- [ ] **T2 — Factor `invoke_stream_json`.** Move the
  `AdapterKind::Claude` arm of `invoke_with_stager` (:348-401) verbatim
  into a private binary-parameterized helper (live fold, stderr-drain
  thread, decision-0001 noise pass-through preserved;
  `fold_stream_event` untouched; no `kind` parameter in the shared
  body); the claude arm becomes a one-liner.
  *Proven by*: AC-3 — the existing claude conformance assertions pass
  literally unedited (the regression guard on the factoring).
- [ ] **T3 — The lanetally arm.** One-liner calling the helper with
  `adapter_binary("FORGE_LANETALLY_BIN", "claude-lanetally")`.
  *Proven by*: AC-2 and AC-4 — the obedient battery leg (T6) drives it
  through `FORGE_LANETALLY_BIN` pointed at the scripted stand-in.
- [ ] **T4 — The `capture` constant in `run_seat`.** Kind-guarded
  insert of `capture: "lanetally"` immediately AFTER
  `checkpoint.extend(invocation.session_meta)` (:633-639) — a source
  literal, one site, last-write-wins over any stream key.
  *Proven by*: AC-5 — the adversarial `"capture":"evil"` shim leg (T7)
  and the claude `finished.get("capture").is_none()` negative
  assertion; plus a unit test on the insert-after-extend ordering.
- [ ] **T5 — Pin `FORGE_LANETALLY_BIN` in `drive()`.** Unconditional
  `.env("FORGE_LANETALLY_BIN", shim)` beside the three siblings
  (`driver_conformance.rs:92-94`), in the same commit as the first
  lanetally test.
  *Proven by*: AC-6 — no conformance test can spawn a real
  `claude-lanetally`; verifiable by inspection plus the battery running
  green with no wrapper installed.
- [ ] **T6 — Battery membership.** `all_adapters()` gains
  `("lanetally", vec!["lanetally"])` with its own explicit label branch:
  obedient (`CLAUDE_STREAM_SHIM` verbatim — the argv-compatibility
  proof) asserts the claude shape plus
  `step == "claude-lanetally-session-finished"`,
  `capture == "lanetally"`, `total_cost_usd == 0.125`, `session_id`,
  exit_code 0, succeeded; silent asserts failed with "no result file";
  `adapters_name_themselves…` gains `("lanetally", "claude-lanetally")`.
  *Proven by*: AC-1, AC-4 — the battery itself.
- [ ] **T7 — Adversarial shim leg.** New `LANETALLY_ADVERSARIAL_SHIM`
  records its argv and emits the stream shape with `"capture":"evil"`
  in the `result` event; test asserts
  `-p --output-format stream-json --verbose` arrived (prompt on stdin)
  and the finished checkpoint still says `capture:"lanetally"`.
  *Proven by*: AC-3 (argv shape) and AC-5 (shadowing) — this task IS
  those assertions.
- [ ] **T8 — Masking through the shared choke point.** Generalize
  `drive_exec_with_secrets` to accept driver args; new leak shim baking
  the store's known plaintext literally into its result notes (never
  `$API_TOKEN` — the claude/lanetally arm injects no secret env);
  assert `[secret:API_TOKEN]` present AND plaintext absent in the full
  result message. Zero new masking code.
  *Proven by*: AC-7 — this task IS the assertion.
- [ ] **T9 — Cost-flow regression.** Test that `seat_costs`
  (`compare.rs`) sums `total_cost_usd` from a session-finished
  checkpoint carrying `capture`, with zero production changes to
  `compare.rs`, `forge costs`, or the UI.
  *Proven by*: AC-8 — the test, plus an empty diff on those surfaces.
- [ ] **T10 — CLI surface in `crates/forge-cli/src/main.rs`.**
  Unknown-driver error (:616) and `Driver` help text (:226,230) gain
  `lanetally`.
  *Proven by*: AC-9 — a CLI test asserting both strings list it.
- [ ] **T11 — Doctor row in `crates/forge-cli/src/doctor.rs`.** Probe
  tuple grows an optional hint; the `claude-lanetally` warning names
  `~/.local/bin/claude-lanetally` and the `FORGE_LANETALLY_BIN`
  override; the four existing warning strings stay byte-identical.
  *Proven by*: AC-10 — `doctor/tests.rs`: missing→warning-with-path,
  present→ok, existing-warnings-unchanged.
- [ ] **T12 — Docs.** `ARCHITECTURE.md` (~:104) and
  `docs/extension-model.md` (~:54) list lanetally with the one honest
  cost-provenance sentence (list price stays; ledger-priceable; join
  deferred until readplane exposes a session query).
  *Proven by*: AC-11 — review against the framed sentence.
- [ ] **T13 — Workspace regression sweep.** `cargo test --workspace`;
  frozen surfaces byte-untouched.
  *Proven by*: AC-12 — full suite green plus empty `git diff --stat`
  for `reference/`, `fixtures/`, `contracts/`, `policy/`.
