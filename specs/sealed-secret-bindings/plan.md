# Implementation Plan: Sealed secret bindings

**Feature slug**: `sealed-secret-bindings`
**Spec**: [spec.md](spec.md)

## Position reconciliation (how this plan was synthesized)

The panel agreed on the load-bearing shape: one new module
(`crates/forge-protocol/src/secret.rs`) holding type + store + masker +
scanner; zero new dependencies; resolution and injection confined to
the exec arm so values never enter the engine process or cross the
NDJSON protocol; a single masking choke point in the adapter; no
`forge secrets get`; no backend trait; no streaming masker; UI
untouched (journal envelopes are its only source). The chief's rulings
on the genuine divergences:

1. **Injection basis** — *robustness adopted*. All declared names are
   injected; template references are optional argv-side spelling;
   the lint is one-directional (referenced ⇒ declared). A
   reference-driven injector silently breaks the headline use case:
   `gh` reads `GH_TOKEN` from the environment with no argv reference
   at all.
2. **Masking scope** — *robustness adopted, and it is the decisive
   ruling of this design*. Simplicity masked `Invocation.stderr` only;
   robustness identified two bypasses that put plaintext into the
   append-only journal: the stderr re-emit at `adapters.rs:490`
   (journaled as the stderr tail by `engine.rs:593` on failure —
   exactly when credentialed commands print offending headers) and the
   child-written **result payload** (journaled in `EffectSucceeded`).
   The choke point therefore masks stdout, stderr, and the serialized
   result payload, on raw bytes before lossy string conversion, before
   the re-emit and before `Body::Result`. Simplicity's
   *single*-choke-point stance survives: no engine-side or UI-side
   re-masking — the layer-6 proof scans the journal, not the masker,
   and guards against future bypass paths.
3. **Needle list** — *robustness adopted*. "base64, hex, URL-escaped"
   is four-way ambiguous for base64 alone; the spec enumerates the
   exact needle set once, as a constant in `secret.rs` shared verbatim
   by masker and proof, with longest-needle-first replacement.
   Simplicity's narrower list (standard base64 only) rejected —
   `base64url` is the JWT alphabet, squarely a "common encoding".
4. **Store hygiene** — *robustness largely adopted*:
   refuse-broader-than-0600 on read, atomic temp+rename write that
   never widens an existing mode, stdin-only `set`, value constraints
   (non-empty, single-line UTF-8, no NUL, ≥4 bytes refused / <8
   warned). *Simplicity adopted* on file locking (none: operator-local
   single-writer; a lost race is a failed resolve, never a leak) and
   on encryption at rest (none: the key-for-the-keystore regress is
   Vault's job, a declared non-goal).
5. **Denylist** — *robustness adopted*. Exact set
   `{PATH, IFS, LD_PRELOAD, LD_LIBRARY_PATH}` plus the `FORGE_`
   prefix, one shared constant, enforced at bundle compile and at
   `forge secrets set`. All are grammar-legal names that would turn
   the injector into a code-loading or harness-spoofing primitive.
6. **Digest-stability hazard** — *robustness adopted with one cut*.
   Adopted: the end-to-end rotation test (set → compile → rotate →
   compile → digests byte-equal) and a compile refusal of a file named
   `secrets.env` inside the bundle dir (exact-name check; a store
   inside the bundle both breaks digest stability and embeds a
   guessable hash in the manifest). Rejected: `forge secrets set`
   detecting "a target inside a bundle dir it can recognize" — the CLI
   has no reliable bundle-dir oracle, and the compile lint catches the
   harm where it manifests.
7. **The masker/expose tension in layer 4** — *robustness adopted*.
   The masker derives needles from the raw bytes, so it is constructed
   inside `secret.rs` from private field access; `expose_for_spawn`
   remains the single public egress with its single call site in the
   exec-arm injector, CI-grep enforced. Left implicit, an implementer
   either adds a second accessor (breaking the grep) or routes the
   masker through `expose` (breaking the count).
8. **Amendment spelling** — *robustness adopted*. The journaled target
   is the pre-substitution charter template (`{{secret:NAME}}`
   spelling), not simplicity's post-`$NAME` text: the charter string
   is the exact artifact the compile lint proved value-free, while
   post-substitution text embeds `{workdir}`/`{prompt_file}`
   expansions that are not the recorded contract. Decision 0012
   permits either spelling; this one is the provable one. The claude
   fold and model-Bash postures are untouched (unanimous).
9. **Dependency and abstraction floor** — *simplicity adopted
   throughout*: zero new crates (hand-rolled encode-only base64 and
   hex — hex *is* in the workspace but only forge-core depends on it,
   and ~10 lines of encode beats a Cargo.toml edge for a
   trust-boundary module), no regex, no dotenv, no zeroize crate
   (std `write_volatile` + `compiler_fence`), no secrets-manager
   trait, no entropy detection, no per-step scoping/TTL/audit, no new
   protocol message kinds. Robustness's "backends one function away"
   is satisfied by the store being plain functions — an interface
   extracted when a second backend exists, not before.
10. **Streaming** — *both adopted*: no streaming masker (every masked
    surface is a complete captured buffer today), but robustness's
    invariant is written down — masking operates on complete buffers;
    any future streaming capture needs an overlap window ≥ the longest
    needle — as a doc comment on the masker and in the spec, so the
    regression cannot arrive silently.
11. **Missing/malformed handling** — *robustness adopted*: missing
    name refuses before spawn naming the name (never an empty-string
    injection); any `{{secret:` occurrence that does not parse as a
    well-formed declared reference is a compile error.

## Approach

One new module, five edited files, in dependency order:

1. **`crates/forge-protocol/src/secret.rs`** (new; registered in
   `lib.rs`) — the whole plaintext trust boundary:
   - `Secret`: `Vec<u8>` newtype; no `Display`/`Clone`/`Serialize`;
     `Debug` → `Secret(REDACTED)`; `Drop` zeroizes via
     `write_volatile` + `compiler_fence`; `expose_for_spawn(&self) ->
     &[u8]` is the only public egress.
   - Store: env-format read/parse (single buffer, split in place) and
     atomic write (`OpenOptions` + `PermissionsExt` 0600, temp +
     rename); mode check on read; value validation; the denylist
     constant.
   - Reference scanner: hand-scanner for `{{secret:NAME}}` returning
     well-formed refs and malformed-occurrence errors.
   - Masker: needle-list constant (raw; base64 std/URL-safe ×
     padded/unpadded; hex lower/upper; percent upper/lower per value),
     built in-module from private bytes; byte-level,
     longest-needle-first replacement with `[secret:NAME]`.
2. **`crates/forge-runtime/src/bundle.rs`** — parse `"secrets"` next
   to the 0007 `inputs` handling; validate names (grammar + denylist);
   in `parse_command` (~:548) scan raw template parts and refuse
   undeclared/malformed references with the existing
   `CompileError::Invalid` shape; refuse a `secrets.env` file in the
   bundle dir. Names ride `manifest_for` (~:639) via the charter as
   today — digest stability across rotation is free by construction
   and asserted by test.
3. **`crates/forge-runtime/src/engine.rs`** — thread declared names +
   store path into the exec driver `start` input (names-only,
   journal-safe). That is the entire engine change; no store read
   exists in `forge-runtime`.
4. **`crates/forge-protocol/src/adapters.rs`** — the exec arm (~:406):
   resolve `{{secret:NAME}}` → `$NAME` in template text alongside
   `{workdir}`; open the store, resolve all declared names (refusing
   determinately on a missing one), pass env pairs into `run_cli`
   (~:101), which grows an `envs` parameter carrying the single
   `expose_for_spawn` call site; mask captured stdout/stderr bytes and
   the child-written result payload before the stderr re-emit (:490),
   before checkpoints, before `Body::Result`; journal the
   pre-substitution template (80-char clamp, reusing the clamp
   discipline at :135–176) as the checkpoint target.
5. **`crates/forge-cli/src/main.rs`** — `Secrets` subcommand
   (`Set`/`List`/`Remove`), `--secrets-file` flag (also plumbed to run
   entry points, default `.forge/secrets.env`), `set` reading the
   value from stdin.
6. **Tests** — unit tests beside each piece; machine proofs appended
   to `crates/forge-cli/tests/machine_proof.rs` (scripted-child
   pattern per `driver_conformance.rs`); the CI grep test for the
   single call site.

Untouched: `crates/forge-core/*`, `crates/forge-cli/src/ui.rs`,
`ui.html`, `policy/phase-machine.json`, `reference/`, `fixtures/`,
`contracts/`, all recipes.

## Risks and mitigations

- **A future output path routes around the choke point.** The layer-6
  proof byte-scans the journal itself, iterating the shared needle
  constant — a new unmasked path fails the proof in CI, not in
  production. This is the accepted alternative to belt-and-braces
  masking at every append.
- **Hand-rolled base64/hex/percent encoders are wrong.** Encode-only,
  ~40 lines total, unit-tested against fixed vectors; and the layer-6
  proof independently exercises every encoding end to end (the child
  encodes with real tools, the scan uses our needles — a broken
  encoder shows up as a proof failure, not a silent gap).
- **Masker and proof drift apart.** Structurally prevented: one
  constant, both consumers iterate it; the proof has no hand-copied
  encoding list.
- **`$NAME` argv spelling surprises non-shell templates.** Documented
  honestly in spec and charter docs: env injection always works; argv
  expansion requires a shell in the template; no `sh -c` wrapping.
  Residual confusion is a docs problem, not a leak — the value is in
  the env either way and never in argv.
- **Declared-secret env override clobbers an operator variable.**
  Documented: declared wins. The declaration is in the reviewed
  charter; a collision is visible at review time.
- **Masking short/low-entropy values shreds the journal's evidence
  value.** Mitigated at `set`: <4 bytes refused, <8 warned.
- **Zeroization is best-effort, not a guarantee.** Stated as such in
  the doc comment and spec; the test asserts overwrite where
  observable and the spec claims nothing stronger (kernel buffers and
  pre-drop copies are out of scope by decision text).
- **A child re-encodes creatively (rot13, gzip, split).** Out of
  scope by 0012's own "what this does not promise": the layered
  guarantee is model-never-holds-it, argv-never-holds-it,
  listed-encodings-caught; adversarial children belong to
  `driver.confine`.
- **TOCTOU between store read and spawn.** Accepted: single-writer
  operator-local file; the failure mode is a determinate refusal or a
  stale value, never a leak.
- **Windows.** `$NAME` spelling and 0600 modes are POSIX; the
  fake-driver state files were just made Windows-safe (#27), but the
  secrets store's permission checks are Unix-gated
  (`#[cfg(unix)]`), with the caveat recorded rather than solved —
  matching the ruling's scope.
