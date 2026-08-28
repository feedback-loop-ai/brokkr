# Change Proposal: sealed-secret-bindings

## Why

The checkpoint-target ruling bans commands, URLs, and prose from the
journal because they routinely embed inline credentials, and the
append-only, hash-chained journal can never scrub a secret once
recorded. The ban treats the symptom and costs telemetry density. The
disease is that a secret value can appear inside a command at all.
Decision 0012 (`docs/decisions/0012-sealed-secret-bindings.md`,
accepted 2026-08-28) rules the cure: secrets are referenced by name,
never written by value, anywhere a seat or bundle can reach — after
which exec steps can run credentialed commands (`gh`, `curl` with
tokens) with no value in the bundle, journal, telemetry, or UI, and
unresolved command *templates* become safe to journal as targets.

## What Changes

Six layers, each independently testable, plus one amendment:

1. **Reference + declaration**: exec templates may contain
   `{{secret:NAME}}` (`[A-Z][A-Z0-9_]*`); charters declare
   `"secrets": [names]` (bundle-format key only); compile refuses
   undeclared or malformed references and denylisted names
   (`PATH`/`IFS`/`LD_*`/`FORGE_*`) in the 0004/0007
   load-time-refusal shape. Injection is declaration-driven —
   declared-but-unreferenced names still inject, so env-reading tools
   like `gh` work.
2. **Operator store**: `forge secrets set|list|remove` over an
   env-format file outside VCS (default `.forge/secrets.env`, 0600 on
   create, refuse broader on read, atomic writes, `--secrets-file`
   override, value via stdin, no `get`); digests carry names only, so
   rotation never changes a digest — proven end to end.
3. **Injection discipline**: values reach the child only via its
   environment, resolved at spawn inside the exec driver arm; never
   argv; `{{secret:NAME}}` resolves in template text to `$NAME`;
   missing names refuse before spawn; the engine threads names and
   store path only.
4. **`Secret` type**: no `Display`, `Debug` → `Secret(REDACTED)`,
   best-effort zeroization on drop, plaintext egress through exactly
   one method with exactly one production call site (CI-grep
   enforced); std-only, zero new dependencies.
5. **Known-plaintext masking**: one byte-level choke point in the exec
   arm masks captured stdout, stderr, and the child-written result
   payload — before the stderr re-emit, checkpoints, journal, and UI —
   replacing every bound value and its listed encodings (base64
   std/URL-safe × padded/unpadded, hex both cases, percent both
   cases) with `[secret:NAME]`, from one needle constant shared with
   the layer-6 proof.
6. **Journal invariant**: a machine proof runs a scripted child that
   leaks the value in every listed encoding via stdout, stderr, and
   result notes, then byte-scans every journal envelope for every
   needle — zero hits or fail.

**Amendment**: an unresolved command template (journaled here in its
pre-substitution `{{secret:NAME}}` spelling) MAY be recorded as a
checkpoint target within the existing 80-char clamp; resolved command
lines, URLs, and prose remain banned; model-authored Bash remains
unjournaled.

Design artifacts:

- [specs/sealed-secret-bindings/spec.md](../../../specs/sealed-secret-bindings/spec.md)
  — what and why: the confinement statement, all six layers with
  their fail-closed edges, the amendment, and the
  `## Acceptance Criteria`.
- [specs/sealed-secret-bindings/plan.md](../../../specs/sealed-secret-bindings/plan.md)
  — how: the panel-position reconciliation (eleven explicit rulings),
  files touched in dependency order, risks with mitigations.
- [specs/sealed-secret-bindings/tasks.md](../../../specs/sealed-secret-bindings/tasks.md)
  — twelve ordered tasks, each paired with the test that proves it.

## Impact

- **New**: `crates/forge-protocol/src/secret.rs` — the plaintext trust
  boundary (type, store, scanner, masker, needle + denylist
  constants).
- **Edited**: `crates/forge-runtime/src/bundle.rs` (charter `secrets`
  key, compile lints, store-in-bundle refusal),
  `crates/forge-runtime/src/engine.rs` (names + store path threaded
  into the exec driver input; no store read in forge-runtime),
  `crates/forge-protocol/src/adapters.rs` (spawn-time resolution, env
  injection, the masking choke point, the amendment's template
  target), `crates/forge-cli/src/main.rs` (`secrets` subcommand,
  `--secrets-file`), plus `forge-protocol/src/lib.rs` module
  registration.
- **Tests**: unit tests beside every layer; machine proofs in
  `crates/forge-cli/tests/machine_proof.rs` (journal invariant for
  succeeding and failing children, digest stability across rotation,
  amendment clauses); a CI grep test pinning the single plaintext
  call site.
- **Untouched**: `crates/forge-core`, `ui.rs`/`ui.html` (the UI reads
  only journal envelopes, which are masked upstream),
  `policy/phase-machine.json`, `reference/`, `fixtures/` (frozen
  corpus), `contracts/` v1 files, all recipes. No new dependencies.
- **Operational**: operators seed values once via
  `forge secrets set NAME` (stdin); charters declare names; rotation
  is a `set` that changes no digest; a leak-shaped bug surfaces as a
  CI proof failure, never as a journaled secret.
