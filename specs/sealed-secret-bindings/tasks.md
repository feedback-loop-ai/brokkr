# Tasks: Sealed secret bindings

**Feature slug**: `sealed-secret-bindings`
**Spec**: [spec.md](spec.md) · **Plan**: [plan.md](plan.md)

Ordered; each task names the test that proves it. Acceptance-criteria
numbers (AC-n) refer to spec.md's `## Acceptance Criteria`.

- [x] **T1 — `Secret` type in `crates/brokkr-protocol/src/secret.rs`.**
  `Vec<u8>` newtype: no `Display`/`Clone`/`Serialize`; `Debug` →
  `Secret(REDACTED)`; drop zeroization via `write_volatile` +
  `compiler_fence`; `expose_for_spawn` as sole public egress.
  *Proven by*: AC-6 — `Debug` format assertion, no-`Display`
  compile-fail test, drop-overwrite check, and the CI grep test
  asserting exactly one `expose_for_spawn` call site outside
  `secret.rs`.
- [x] **T2 — Store read/write in `secret.rs`.** Env-format parse
  (single buffer, in-place split), atomic 0600 write (temp + rename,
  never widening), broader-than-0600 read refusal, value validation
  (non-empty, single-line UTF-8, no NUL, ≥4 bytes), denylist constant
  (`PATH`, `IFS`, `LD_PRELOAD`, `LD_LIBRARY_PATH`, `FORGE_` prefix).
  *Proven by*: AC-3 unit tests — round-trip, mode-on-create,
  atomic-replace-preserves-mode, refuse-broad-read, every rejected
  value class.
- [x] **T3 — Reference scanner + masker in `secret.rs`.** Hand-scanner
  for `{{secret:NAME}}` (well-formed vs. malformed occurrence); needle
  constant (raw; base64 std/URL-safe × padded/unpadded; hex
  lower/upper; percent upper/lower); byte-level longest-needle-first
  replacement with `[secret:NAME]`; buffered-only invariant doc
  comment.
  *Proven by*: AC-1 grammar/scanner vectors; AC-7 masker tests —
  every encoding, multiple overlapping secrets, pass-through text,
  invalid-UTF-8 surroundings.
- [x] **T4 — Charter declaration + compile lint in
  `crates/brokkr-runtime/src/bundle.rs`.** Parse `"secrets"` beside the
  0007 `inputs` handling; refuse undeclared references, malformed
  `{{secret:` occurrences, ill-formed or denylisted names, and a
  `secrets.env` file inside the bundle dir — all in the
  `CompileError::Invalid` load-time-refusal shape.
  *Proven by*: AC-2 compile tests (each refusal class, plus
  declared-and-referenced and declared-but-unreferenced compiling).
- [x] **T5 — Digest stability across rotation.** Names-only in the
  manifest; store outside the bundle dir.
  *Proven by*: AC-4 — set → compile → rotate value → compile →
  `manifest_digest` byte-equal, end to end.
- [x] **T6 — Engine threading in
  `crates/brokkr-runtime/src/engine.rs`.** Declared names + store path
  ride the exec driver `start` input; no store read in
  `brokkr-runtime`.
  *Proven by*: a unit test on the driver-input shape and a grep-style
  test asserting no secret-store call sites in `brokkr-runtime`; AC-8
  exercises the wiring end to end.
- [x] **T7 — Spawn-time resolution + injection in
  `crates/brokkr-protocol/src/adapters.rs`.** Exec arm resolves
  `{{secret:NAME}}` → `$NAME` in template text; opens the store,
  resolves every declared name, refuses determinately (naming the
  name) on a missing one; `run_cli` grows an `envs` parameter carrying
  the single `expose_for_spawn` call site; declared name overrides
  pre-existing child env.
  *Proven by*: AC-5 — argv-never-contains-value,
  env-contains-value, unreferenced-name-still-injected,
  missing-name-pre-spawn-refusal, override tests.
- [x] **T8 — Masking choke point in the exec arm.** Captured stdout,
  stderr, and the child-written result payload masked on raw bytes
  before the stderr re-emit (`adapters.rs:490`), before checkpoints,
  before `Body::Result`.
  *Proven by*: unit tests on each masked surface; AC-8 machine proof
  covers all three paths through the journal.
- [x] **T9 — Checkpoint-target amendment.** Exec effects journal the
  pre-substitution charter template (80-char clamp) as target; claude
  fold and model-Bash postures untouched.
  *Proven by*: AC-9 — unresolved-template target within clamp,
  resolved-line-never-journaled, claude-fold file-path-only, model
  Bash target-less (one test per clause).
- [x] **T10 — `brokkr secrets` CLI in
  `crates/brokkr-cli/src/main.rs`.** `set` (value via stdin) / `list`
  (names only) / `remove`; `--secrets-file` on secrets subcommands and
  run entry points; no `get`.
  *Proven by*: AC-3 CLI-level tests — round-trip through the binary,
  `list` output contains no value bytes, override flag honored.
- [x] **T11 — Layer-6 machine proof in
  `crates/brokkr-cli/tests/machine_proof.rs`.** Scripted child leaks
  the bound value in every listed encoding via stdout, stderr, and
  result notes; byte-scan of every journal envelope iterating the
  shared needle constant; zero hits or fail — for a succeeding and a
  failing child.
  *Proven by*: AC-8 (this task IS the proof).
- [x] **T12 — Workspace regression sweep.** `cargo test` across all
  crates; frozen surfaces byte-untouched.
  *Proven by*: AC-10 — full suite green plus empty
  `git diff --stat` for `reference/`, `fixtures/`, `contracts/`,
  `policy/`, `crates/brokkr-core`.
