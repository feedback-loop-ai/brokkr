# Feature Specification: LaneTally driver (fifth forge-driver/v1 adapter)

**Feature slug**: `lanetally-driver`
**Run**: `forge-the-lanetally-driver-a-fif-b6636673`
**Status**: Committed (design phase ruling)
**Scope**: decision 0008 fleet extension — inside 0008's delivered
scope, no new decision doc.
**Input positions**: `.forge/design/positions/simplicity.md`,
`.forge/design/positions/robustness.md` (run-local, uncommitted)

## Why

Seats launched through the `claude` driver report only the harness's
list-price `total_cost_usd`. LaneTally's session-capture wrapper
(`claude-lanetally`, which execs `rollout/session-wrapper.sh -- claude
"$@"`) records every session in LaneTally's cost ledger, where it is
priceable at real marginal cost (subscription-vs-API accounting).
`brokkr driver lanetally` runs the SAME Claude Code harness through that
wrapper, so seat sessions become ledger-priceable with zero change to
what the harness streams or what Brokkr journals — plus one constant
field that lets `brokkr costs` and the UI tell captured sessions apart.

Operator-established facts this design relies on and does not
re-derive: `claude-lanetally` is on PATH (`~/.local/bin/claude-lanetally`)
and is argv-compatible with `claude`, including
`--output-format stream-json`. The existing claude invocation shape
(`-p --output-format stream-json --verbose`, prompt on stdin) and the
existing stream-json fold therefore apply unchanged.

## The design in one paragraph

The LaneTally adapter is the claude adapter with a different binary and
one constant. `AdapterKind::Lanetally` parses from `"lanetally"`,
resolves its binary from `BROKKR_LANETALLY_BIN` (default
`claude-lanetally`), and invokes through a shared, binary-parameterized
helper factored verbatim from today's claude arm — the helper takes a
binary, never an `AdapterKind`, so lanetally-only drift is
unrepresentable in shared code. The session-finished checkpoint
additionally carries `capture: "lanetally"`, a source-literal constant
inserted at exactly one site in `run_seat`, after the `session_meta`
extend, so stream data can never shadow it. Everything else — seat-turn
checkpoints, 80-char clamps, file_path-only targets, decision-0001
noise pass-through, the decision-0012 masking choke point — is
inherited by construction because the code does not fork.

## Naming

- **Parse token**: `"lanetally"` (`brokkr driver lanetally`, the
  bundle's `driver` field).
- **`driver_name()`**: `"claude-lanetally"` — following the existing
  precedent that the parse token and driver name diverge
  (`"claude"` → `"claude-code"`, adapters.rs:45). It says what the
  session IS (the Claude Code harness, LaneTally-captured), matches the
  binary and doctor label, and leaves the bare name free for a future
  `codex-lanetally` wrapper. This yields the step
  `claude-lanetally-session-finished` and capabilities
  `driver: "claude-lanetally"`. Step-name vocabulary in the append-only
  journal is chosen once and never renamed.
- **The ledger discriminator is the `capture` field, never the step
  name.** Step names are already forgeable (`FORGE_EXEC_NAME=lanetally`
  would journal `lanetally-session-finished` today with no capture
  semantics, adapters.rs:48-50). Tools that grep step names will
  mis-bucket; tools that read `capture` cannot.

## Invocation sharing

Today's `AdapterKind::Claude` arm of `invoke_with_stager`
(adapters.rs:348-401) moves verbatim into a private helper in the same
file — no new module, no trait:

```rust
fn invoke_stream_json(bin: String, extra: &[String], prompt: &str,
    workdir: &str, emit: &mut impl FnMut(&Value)) -> Result<Invocation, String>
```

with both arms reduced to calls selecting the `BROKKR_CLAUDE_BIN`/`claude`
or `BROKKR_LANETALLY_BIN`/`claude-lanetally` pair through the shared
one-release legacy resolver.
Three properties of the current arm are invariants the helper MUST
preserve, each pinned by tests:

1. **Live folding, not buffering.** Stdout is folded line-by-line while
   the child runs; checkpoints are streamed telemetry. Convergence on
   `run_cli`/`wait_with_output` is a rejected design, not an open
   option — it would silently kill live seat-turn checkpoints for both
   drivers while every message-order assertion stays green.
2. **The stderr-drain thread** (adapters.rs:376-382). The wrapper is an
   extra layer that may chat on stderr; the drain thread is MORE
   load-bearing for lanetally than for claude.
3. **Noise pass-through** (decision 0001, adapters.rs:390-392):
   unparseable lines are dropped, never repaired. The wrapper may
   interleave non-JSON output; this line is what makes that safe.
   `fold_stream_event` is shared untouched — no `kind` parameter is
   added to it.

Claude behavior is byte-for-byte unchanged; the existing claude
conformance assertions, literally unedited, are the regression guard on
the factoring.

## The `capture` constant

Attached in `run_seat`'s session-finished assembly (adapters.rs:633-639),
guarded by kind, AFTER the `session_meta` extend:

```rust
checkpoint.extend(invocation.session_meta);
if kind == AdapterKind::Lanetally {
    checkpoint.insert("capture".into(), Value::String("lanetally".into()));
}
```

- Constant-valued **by construction**: a source literal in a function
  that never sees stream data — checkable by reading four lines, never
  by tracing `session_meta` provenance through the fold.
- The insertion order is an invariant, not style: `Map::insert` is
  last-write-wins, so no stream-derived key can ever shadow the
  constant — not today, and not after a future edit widens what the
  fold copies into `session_meta`. Pinned adversarially: a shim whose
  `result` event carries `"capture":"evil"` must still yield
  `capture:"lanetally"`, and claude's finished checkpoint must carry no
  `capture` key at all (the negative assertion keeps the guard from
  quietly becoming unconditional).
- `capture` rides checkpoint `data` (adapter-defined evidence): a
  constant-valued additive field, not a protocol change. Frozen
  contracts v1 untouched.
- `capture:"lanetally"` means "priceable in the LaneTally ledger", not
  "priced". `total_cost_usd` stays the harness-reported list price; the
  per-session actual-cost join is deferred until readplane exposes a
  session query.

## CLI surface, doctor

- Unknown-driver error (`main.rs:616`) and the `Driver` help text
  (`main.rs:226,230`) gain `lanetally`. `AdapterKind::parse` is the
  single dispatch point; nothing else changes.
- Doctor's optional-tool probe list gains `claude-lanetally`,
  present-or-advisory: missing is a WARNING naming the expected install
  path (`~/.local/bin/claude-lanetally`) and the `BROKKR_LANETALLY_BIN`
  absolute-path override (the default resolution relies on
  `~/.local/bin` being on PATH, routinely false under systemd/cron/CI)
  — never a hard failure; the fleet must work on machines without
  LaneTally. The four existing warning strings stay byte-identical.
- No spawn-time existence probe and NO fallback to plain `claude` when
  the wrapper is missing — a fallback would silently un-capture
  sessions, strictly worse than the existing honest "could not invoke
  the agent CLI" failure. Doctor is the advisory surface; the runtime
  stays dumb.

## Masking boundary (decision 0012, stated honestly)

The decision-0012 choke point applies to this adapter exactly as to
claude: `run_seat`'s result-payload masking on raw bytes
(adapters.rs:666-672) is shared code the lanetally path reaches by
construction, and a conformance assertion proves it (no new masking
code). Two honest boundary lines:

- The claude/lanetally arm injects no secret env into the child
  (`bindings` feed only the exec arm's `run_cli`); today's exposure via
  this adapter is nil by construction. The masking demonstration must
  therefore bake the store's known plaintext into the shim's result
  notes — a shim echoing `$API_TOKEN` would leak nothing and prove
  nothing.
- **Brokkr masks its journal, not LaneTally's files.** LaneTally's
  session capture happens inside the wrapper, upstream of Brokkr's choke
  points; whatever the harness streams is captured there unmasked by
  Brokkr. The guarantee is "Brokkr's journal is masked", and any future
  extension that injects secret env into this arm must revisit this
  boundary explicitly.

## Non-goals

- No per-session actual-cost join, no readplane/HTTP query, no reading
  LaneTally's ledger from Brokkr. No new dependencies, no HTTP
  client.
- No changes to `brokkr costs` aggregation or the UI: checkpoint `data`
  is open, the extra field rides free — verified by test, then zero
  changes made.
- No wrapper metadata in the checkpoint (session id, ledger row,
  wrapper version, timestamps): all data-derived (forbidden) or join
  territory (deferred). The constant is the whole payload.
- No configurability beyond `BROKKR_LANETALLY_BIN`; no capture on/off
  flag; `extra` after `--` already flows.
- No changes to the secrets machinery (decision 0012) — assertion only.
- No changes to claude/codex/dsh/exec observable behavior; frozen
  contracts v1, `policy/phase-machine.json`, `reference/`, and the
  differential corpus untouched.
- No deep LaneTally integration (default-deny per-seat credentials) —
  deferred by name in decision 0008.

## Acceptance Criteria

1. **Parse and naming**: `AdapterKind::parse("lanetally")` →
   `Lanetally`; `driver_name()` is `"claude-lanetally"`; capabilities
   report `driver: "claude-lanetally"`; the session-finished step is
   `claude-lanetally-session-finished`;
   `adapters_name_themselves…` includes the
   `("lanetally", "claude-lanetally")` row.
2. **Binary resolution**: the harness binary comes from
   `BROKKR_LANETALLY_BIN`, default `claude-lanetally`, via the existing
   `adapter_binary()` idiom; the module doc comment's env list names it.
3. **Shared invocation**: the factored helper takes a binary, not an
   `AdapterKind`; the lanetally shim records its argv and the test
   asserts `-p --output-format stream-json --verbose` arrived with the
   prompt on stdin; the existing claude conformance assertions pass
   literally unchanged (regression guard on the factoring).
4. **Conformance battery**: lanetally joins `all_adapters()` and,
   driven with `CLAUDE_STREAM_SHIM` verbatim (the argv-compatibility
   proof), produces the same obedient shape as claude — capabilities,
   accepted, 3 seat-turn checkpoints, session-finished with
   `capture:"lanetally"`, `total_cost_usd` 0.125, `session_id`,
   exit_code 0, result succeeded; the silent shim yields failed with
   "no result file".
5. **Capture integrity**: an adversarial shim emitting
   `"capture":"evil"` in its result event still yields
   `capture:"lanetally"` on the finished checkpoint; the claude
   finished checkpoint carries no `capture` key.
6. **No real-wrapper escape**: `drive()` sets `BROKKR_LANETALLY_BIN`
   unconditionally beside its three siblings, so no conformance test
   can ever spawn a real `claude-lanetally` on a LaneTally-equipped
   machine.
7. **Masking through the shared choke point**: a lanetally leak shim
   with the store's plaintext baked literally into its result notes
   yields a result message containing the masked token
   `[secret:API_TOKEN]` AND not containing the plaintext — zero new
   masking code.
8. **Cost flow regression**: `seat_costs` sums `total_cost_usd` from a
   session-finished checkpoint that carries `capture`, with zero
   production changes to `compare.rs`, `brokkr costs`, or the UI.
9. **CLI surface**: the unknown-driver error and the `Driver` help
   text both list `lanetally`.
10. **Doctor**: missing `claude-lanetally` → warning naming
    `~/.local/bin/claude-lanetally` and the `BROKKR_LANETALLY_BIN`
    override; present → ok; the four existing warning strings are
    byte-identical to before.
11. **Docs**: `ARCHITECTURE.md` and `docs/extension-model.md` list
    lanetally with the one honest cost-provenance sentence
    (list price stays; ledger-priceable; join deferred until readplane
    exposes a session query).
12. **Workspace green**: `cargo test --workspace` passes; frozen
    surfaces (`reference/`, `fixtures/`, `contracts/`, `policy/`)
    byte-untouched.
