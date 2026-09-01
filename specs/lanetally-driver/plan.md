# Implementation Plan: LaneTally driver

**Feature slug**: `lanetally-driver`
**Spec**: [spec.md](spec.md)

## Position reconciliation (how this plan was synthesized)

The panel agreed on the load-bearing shape: the adapter is the claude
adapter with a different binary and one constant; `capture` attaches in
`run_seat`, kind-guarded, never at fold time; `fold_stream_event` is
untouched; no new module, trait, or file in `brokkr-protocol`; no
spawn-time fallback to plain `claude`; no wrapper metadata; no
`brokkr costs`/UI changes; no configurability beyond
`FORGE_LANETALLY_BIN`; doctor is advisory with the four existing
warning strings byte-identical; docs get exactly the framed
cost-provenance sentence. The chief's rulings on the divergences:

1. **Factoring shape** — *robustness adopted*: a binary-parameterized
   helper (`invoke_stream_json`), not simplicity's match-arm
   fallthrough. The framing itself says "shared code parameterized by
   binary", and the helper makes "the only difference is the binary"
   structurally true — no `kind` reaches the shared body, so
   lanetally-only drift is unrepresentable, where a fallthrough arm is
   exactly where the next contributor hangs an inner conditional.
   *Simplicity's floor adopted*: the helper is a private `fn` in
   `adapters.rs`, moved verbatim — no `stream_json.rs` module, no
   trait, no config struct. Simplicity's diff-reviewability cost is
   real and is paid deliberately: the compensating control is that the
   claude conformance assertions stay literally unedited and must pass.
2. **The buffering trap** — *robustness adopted as a named
   anti-design*: the helper preserves live folding, the stderr-drain
   thread, and decision-0001 noise pass-through. Convergence on
   `run_cli`/`wait_with_output` is rejected in the spec by name because
   it would pass every current assertion while killing live telemetry.
3. **`driver_name()`** — *robustness adopted*: `"claude-lanetally"`.
   Simplicity's "shortest string" argument falls to an existing
   precedent check: the parse token and driver name already diverge
   (`"claude"` → `"claude-code"`, adapters.rs:45), so `"lanetally"`
   buys no uniformity; `"claude-lanetally"` is provenance-honest (the
   harness is Claude Code; LaneTally is the capture channel), matches
   the binary and doctor label, and leaves namespace room for a future
   `codex-lanetally`. Step vocabulary in the append-only journal is
   chosen once. The spec states robustness's companion rule: the ledger
   discriminator is the `capture` field, never the step name
   (`FORGE_EXEC_NAME` already makes step names forgeable).
4. **`capture` attachment order** — *unanimous site, robustness
   ordering adopted*: inserted in `run_seat` AFTER the `session_meta`
   extend, so last-write-wins guarantees no stream key can shadow the
   constant even if the fold later widens. Pinned by the adversarial
   `"capture":"evil"` shim plus the claude no-`capture` negative
   assertion.
5. **Conformance shims** — *both adopted, scoped*: the obedient battery
   leg reuses `CLAUDE_STREAM_SHIM` verbatim (simplicity — the reuse IS
   the argv-compatibility proof; a lanetally-specific shim would weaken
   it). One new shim exists (robustness — over simplicity's "no new
   shims"): the adversarial leg needs `"capture":"evil"` in the result
   event and argv recording, and neither invariant is testable without
   it; the two checks share the one shim.
6. **The two silent-wrongness traps** — *robustness adopted*:
   (a) `drive()` pins `FORGE_LANETALLY_BIN` unconditionally, or tests
   spawn the real wallet-touching wrapper on LaneTally-equipped
   machines while staying green elsewhere; (b) the masking leg bakes
   the store's plaintext into the shim's result notes — simplicity's
   reuse of `NOTES_LEAK_SHIM` (which echoes `$API_TOKEN`) is vacuous
   because the claude/lanetally arm injects no secret env
   (`bindings` feed only the exec arm's `run_cli`), so the shim would
   write "leaked " and the assertion would exercise zero masking code.
   The test asserts masked-token-present AND plaintext-absent.
   *Simplicity's mechanism adopted*: `drive_exec_with_secrets`
   generalizes to take driver args rather than growing a parallel
   helper.
7. **Cost-flow regression** — *robustness adopted*: a two-line
   `seat_costs` test that a `capture`-carrying checkpoint still sums
   `total_cost_usd`, pinning that the framing's "zero costs/UI changes"
   stays true (`compare.rs` reads only `num_turns`/`total_cost_usd`).
8. **Doctor detail** — *reconciled*: the probe tuple grows an optional
   hint suffix so the lanetally warning names both
   `~/.local/bin/claude-lanetally` and the `FORGE_LANETALLY_BIN`
   override (robustness — `~/.local/bin` is routinely off PATH in
   non-interactive contexts) while the four existing warning strings
   stay byte-identical (both positions). No second probe loop.
9. **Masking boundary sentence** — *robustness adopted*: the spec
   records that forge masks its journal, not LaneTally's capture files,
   so a future env-injection extension to this arm cannot silently leak
   into a second data store. Committed docs keep exactly the framed
   one-sentence cost-provenance scope.
10. **Cut list** — *simplicity adopted wholesale*: no spawn
    probe/fallback, no wrapper metadata, no costs/UI changes, no
    configurability beyond the env var, no decision doc, no protocol
    bump, thin spec artifacts.

## Approach

Six edits in dependency order; no new production files.

1. **`crates/brokkr-protocol/src/adapters.rs`**:
   - `AdapterKind::Lanetally`; `parse("lanetally")`;
     `driver_name() = "claude-lanetally"`; module doc comment env list
     gains `FORGE_LANETALLY_BIN`.
   - Extract the `AdapterKind::Claude` arm of `invoke_with_stager`
     (:348-401) verbatim into private
     `fn invoke_stream_json(bin, extra, prompt, workdir, emit)`;
     both arms become one-liners
     (`adapter_binary("FORGE_CLAUDE_BIN", "claude")` /
     `adapter_binary("FORGE_LANETALLY_BIN", "claude-lanetally")`).
     Live fold, stderr-drain thread, and noise pass-through move
     unmodified; `fold_stream_event` untouched.
   - In `run_seat` (:633-639), after
     `checkpoint.extend(invocation.session_meta)`: kind-guarded insert
     of `capture: "lanetally"`.
   - Unit tests beside the code: parse/name rows, capture-after-extend
     ordering.
2. **`crates/brokkr-cli/src/main.rs`**: `:616` unknown-driver error and
   the `Driver` doc/help text (`:226,230`) gain `lanetally`.
3. **`crates/brokkr-cli/src/doctor.rs`**: probe tuple grows an optional
   hint; `("claude-lanetally", "claude-lanetally", hint)` row whose
   warning names the install path and the env override; existing four
   warnings byte-identical. `doctor/tests.rs`: missing→warning-with-path,
   present→ok, existing-strings-unchanged.
4. **`crates/brokkr-cli/tests/driver_conformance.rs`**:
   - `drive()` gains `.env("FORGE_LANETALLY_BIN", shim)` beside its
     three siblings (unconditional).
   - `all_adapters()` gains `("lanetally", vec!["lanetally"])`;
     lanetally gets its own explicit label branch (no
     `starts_with("claude")` cleverness) asserting the claude obedient
     shape plus `step == "claude-lanetally-session-finished"`,
     `capture == "lanetally"`, `total_cost_usd == 0.125`; the claude
     branch gains `finished.get("capture").is_none()`; claude's
     existing assertions otherwise unedited.
   - `adapters_name_themselves…` gains
     `("lanetally", "claude-lanetally")`.
   - New `LANETALLY_ADVERSARIAL_SHIM`: records `"$@"` to a file, emits
     the claude stream shape with `"capture":"evil"` injected into the
     `result` event; test asserts argv
     (`-p --output-format stream-json --verbose`) and
     `capture == "lanetally"` on the finished checkpoint.
   - Masking leg: generalize `drive_exec_with_secrets` to accept the
     driver args; new leak shim baking the store's known plaintext
     literally into result notes; assert `[secret:API_TOKEN]` present
     and plaintext absent in the full result message.
   - `seat_costs` test in `compare.rs`'s test module: checkpoint with
     `capture` still sums `total_cost_usd`.
5. **Docs**: `ARCHITECTURE.md` adapter paragraph (~:104) and
   `docs/extension-model.md` `driver` row (~:54) each gain lanetally
   plus the one cost-provenance sentence.
6. **Sweep**: `cargo test --workspace`; `git diff --stat` empty for
   `reference/`, `fixtures/`, `contracts/`, `policy/`.

Untouched: `fold_stream_event`, the codex/dsh/exec arms, secrets
machinery (`secret.rs`), `brokkr costs` aggregation, `ui.rs`/`ui.html`,
`crates/brokkr-core`, frozen contracts v1, the differential corpus, all
recipes. No new dependencies.

## Risks and mitigations

- **The factoring silently changes claude behavior.** The extraction is
  a verbatim code move reviewed as such, and the claude conformance
  assertions — literally unedited — are the regression pin. Any drift
  in message order, stream folding, or error text fails the battery.
- **The helper is "simplified" into buffering later.** The spec names
  `run_cli`/`wait_with_output` convergence as a rejected design; the
  live-fold property is documented at the helper. Residual risk
  accepted: timing is not directly asserted (the current battery checks
  order, not liveness), same epistemic position as today's claude arm.
- **Conformance reaches a real wrapper.** `drive()` pins
  `FORGE_LANETALLY_BIN` in the same commit that adds the first
  lanetally test; AC-6 makes the pin itself an acceptance criterion.
- **The masking assertion goes vacuous.** The leak shim bakes plaintext
  (never `$API_TOKEN` env expansion); the test asserts both the masked
  token's presence and the plaintext's absence — a shim regression to
  echoing an unset var fails the presence half.
- **A stream key shadows `capture`.** Insert-after-extend plus the
  adversarial shim; the claude negative assertion keeps the guard
  kind-scoped.
- **Missing wrapper at spawn is confusing.** Accepted: the generic
  "could not invoke the agent CLI" failure is shared with claude and
  stays shared; doctor is the surface that names the install path and
  the env override. A runtime fallback to plain `claude` is rejected —
  it would silently un-capture sessions.
- **Step-name greppers mis-bucket lanetally sessions.** The spec pins
  the rule (discriminator is `capture`, step names are cosmetic and
  forgeable); `brokkr costs`/UI key off checkpoint data, not step
  strings.
- **LaneTally's wrapper drifts from argv compatibility.** The argv
  assertion in the adversarial leg names the break determinately
  instead of a seat mysteriously hanging. A real-wrapper integration
  test is rejected: the fleet must work on machines without LaneTally.
- **Doctor string churn breaks operator muscle memory.** The optional
  hint suffix keeps the four existing warnings byte-identical, asserted
  by test.
