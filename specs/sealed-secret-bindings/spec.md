# Feature Specification: Sealed secret bindings (decision 0012)

**Feature slug**: `sealed-secret-bindings`
**Run**: `implement-decision-0012-sealed-s-e426698e`
**Status**: Committed (design phase ruling)
**Ruling**: `docs/decisions/0012-sealed-secret-bindings.md` (accepted
2026-08-28) — the decision text is the skeleton; this spec refines it
into testable contracts and does not re-litigate it.
**Input positions**: `.forge/design/positions/simplicity.md`,
`.forge/design/positions/robustness.md` (run-local, uncommitted)

## Why

Checkpoint targets journal file paths only, because commands, URLs, and
prose routinely embed inline credentials and the journal is append-only
and hash-chained — a secret journaled once can never be scrubbed. That
ban treats the symptom; the disease is that a secret value can appear
inside a command at all. If commands structurally cannot contain secret
values, command *templates* become safe to record, and exec steps can
use credentialed commands (`gh`, `curl` with tokens) without the
bundle, journal, telemetry, or UI ever holding a value.

Secrets are therefore **referenced by name, never written by value**,
anywhere a seat or bundle can reach. Six layers, each independently
testable, plus one amendment to the checkpoint-target ruling.

## The confinement statement (the spec in one paragraph)

**One process holds plaintext: the exec driver. One module holds
plaintext: `crates/forge-protocol/src/secret.rs`. One choke point
masks: the exec arm, on the raw captured bytes, before anything else
touches them.** The engine passes *names* and the *store path* through
the driver `start` input (both journal-safe); the exec arm opens the
store at spawn time, injects values into the child environment, masks
everything captured, and drops the plaintext. Values never cross the
NDJSON protocol, never enter the engine process during a run, and never
reach model seats — a claude/codex seat in the same bundle cannot
`echo $NAME` a value it was never given.

## Layer 1 — Reference syntax and declaration

- Exec-driver command templates may contain `{{secret:NAME}}`, `NAME`
  matching `[A-Z][A-Z0-9_]*`.
- A seat charter declares its bound names: `"secrets": ["NAME", …]` —
  exactly parallel to decision 0007 input provenance: declared or
  dropped; undeclared names never resolve. The key is a bundle-format
  addition only; no frozen contract changes.
- Bundle compile (`parse_command` / charter parsing in
  `crates/forge-runtime/src/bundle.rs`) fails, in the existing
  `CompileError::Invalid` load-time-refusal shape (decisions 0004/0007),
  on any of:
  1. **Undeclared reference** — a well-formed `{{secret:NAME}}` whose
     `NAME` is not in the charter's `secrets` list.
  2. **Malformed reference** — any occurrence of the opening `{{secret:`
     in a template part that does not parse as a well-formed, declared
     reference: lowercase or empty names, interior whitespace
     (`{{ secret:NAME }}`), unclosed `{{secret:NAME`. Typos fail
     closed at compile; they never ride into argv as literal text.
  3. **Ill-formed declaration** — a declared name that fails the name
     grammar.
  4. **Denylisted name** — declared or referenced names in the exact
     set `{PATH, IFS, LD_PRELOAD, LD_LIBRARY_PATH}` or carrying the
     harness-owned prefix `FORGE_`. All match the name grammar; all
     would turn the injector into an env-hijack or harness-spoofing
     primitive. One shared constant, also enforced by
     `forge secrets set` (layer 2).
- The lint is **one-directional**: referenced ⇒ declared.
  Declared-but-unreferenced is legal and injected (layer 3) — the
  headline consumers (`gh` reading `GH_TOKEN`) take secrets from the
  environment with no argv reference at all.

## Layer 2 — Operator-side store

- `forge secrets set|list|remove NAME` manages an env-format
  (`NAME=value`, line-oriented, `#` comments and blank lines ignored)
  file outside the bundle and outside version control. Default
  `.forge/secrets.env` (`.forge/` is already gitignored); overridable
  via `--secrets-file <path>` on the secrets subcommands and on run
  entry points.
- **Create**: mode `0600` set before content is written.
- **Write**: atomic — temp file in the same directory, `0600` before
  content, then rename over. An existing file's mode is never widened.
  No file locking: the store is operator-local and single-writer;
  a lost race manifests as a failed resolve (missing name → determinate
  refusal), never as a leak.
- **Read**: refuse (determinate error naming the path, never the
  contents) a store whose permissions are broader than `0600` —
  ssh's posture; a silent read of a world-readable file would make the
  create-time test meaningless.
- **`set`** reads the value from **stdin**, never argv — the CLI obeys
  its own injection discipline. Value constraints, enforced at `set`:
  non-empty, valid UTF-8, no newline or NUL (the format is
  line-oriented; a multi-line value would silently truncate into a
  wrong secret plus garbage lines), length ≥ 4 bytes (refused), warn
  under 8 (masking a 2-byte value turns the journal into
  `[secret:X]` confetti and destroys the evidence trail). `set` also
  enforces the layer-1 name grammar and denylist.
- **`list`** prints names, never values. There is **no
  `forge secrets get`** — a value-printing verb is the one thing this
  decision exists to prevent.
- **Digest stability**: bundles and their digests carry names only, so
  rotation never changes a digest. This holds *only because* the store
  lives outside the bundle dir (`manifest_for` hashes every file under
  it — a store inside the bundle would both break the promise and embed
  a SHA-256 of the secret file, an offline-guessing oracle for
  low-entropy secrets). Compile therefore refuses a bundle dir
  containing a file named `secrets.env`, and the stability property is
  proven end to end: set → compile → rotate → compile → digests
  byte-equal.

## Layer 3 — Injection discipline

- Values reach the child **only via the child environment**
  (`Command::env`), resolved at spawn time inside the exec arm of
  `crates/forge-protocol/src/adapters.rs`. Never via argv
  (`/proc/*/cmdline` is world-readable), never via template
  substitution.
- **Injection is driven by declaration, not by template reference**:
  every name in the charter's `secrets` list is resolved and injected.
  Template references are the optional argv-side spelling.
- `{{secret:NAME}}` in template text resolves to the literal
  shell-safe env reference `$NAME` — not the value — alongside the
  existing `{workdir}`/`{prompt_file}` substitutions.
- **Shell semantics, stated honestly**: `run_cli` performs a direct
  `Command::new(program).args(args)` exec with no shell. `$NAME` in
  argv therefore expands only when the template itself invokes a shell
  (`bash -c '…'`). Env injection is the mechanism that always works;
  the `$NAME` spelling is not silently "fixed" by wrapping commands in
  `sh -c` — that would change quoting semantics for every existing exec
  bundle and open injection through the other substitutions. (`$NAME`
  is POSIX spelling; Windows `cmd` uses `%NAME%` — recorded as a
  portability caveat, not solved here.)
- **Missing name fails before spawn**: a declared name absent from the
  store (or a store that cannot be read/parsed) refuses the attempt
  determinately, naming the secret name and the store path — never the
  file contents, never an empty-string injection that turns into a
  downstream 401.
- If a declared name collides with a pre-existing child env entry, the
  declared secret wins (documented; no `FORGE_SECRET_*` namespacing —
  the ruling pins the `$NAME` spelling).
- The engine (`crates/forge-runtime/src/engine.rs`) threads exactly two
  facts into the driver `start` input: the declared names and the store
  path. Both are journal-safe. Resolution never happens engine-side —
  grep-able: no secret-store read exists in `forge-runtime`.

## Layer 4 — The `Secret` type

Lives in the new module `crates/forge-protocol/src/secret.rs`, which is
the plaintext trust boundary, not just a type:

- No `Display` (pinned by a compile-fail test); hand-implemented
  `Debug` prints `Secret(REDACTED)`; no `Clone`, no `Serialize`.
- Best-effort zeroization on drop: `std::ptr::write_volatile` loop plus
  `std::sync::atomic::compiler_fence`. Construction avoids intermediate
  copies (the store file is read into one buffer and split in place —
  no `format!`/`to_string` on values). The doc comment says
  "best-effort" and means it; the spec claims no more than the
  mechanism delivers.
- Plaintext is reachable through exactly one method
  (`expose_for_spawn`) with exactly one production call site: the
  env-injection line in the exec-arm spawn path. Enforced by a CI grep
  test asserting the method name appears exactly once outside
  `secret.rs`.
- The layer-5 masker inherently derives needles from the raw bytes; it
  is therefore constructed **inside** the module from private field
  access and does not use (or count against) `expose_for_spawn`.
- **Zero new dependencies.** Std-only throughout: hand-rolled
  encode-only base64 and hex (~25 lines against fixed test vectors),
  hand-rolled env-format parse, hand-scanned `{{secret:NAME}}`
  references (one literal prefix, one character class, one suffix — a
  `regex` dependency fails the decision-0009 posture for no gain).
  Layer 4's justify-any-dependency clause is satisfied by having
  nothing to justify; forge-protocol's `Cargo.toml` is untouched.

## Layer 5 — Known-plaintext masking

- **The needle list is one canonical constant** in `secret.rs`, shared
  verbatim by the masker and the layer-6 proof (drift between them
  would let the proof pass while a listed encoding leaks). For each
  bound value: raw bytes; base64 standard padded and unpadded; base64
  URL-safe padded and unpadded; hex lowercase and uppercase;
  percent-encoding of all non-unreserved bytes, `%XX` upper- and
  lower-hex. Encodings of encodings are explicitly out — single pass,
  listed shapes only, matching 0012's "what this does not promise".
- Masking operates on **raw captured bytes first**, lossy-converting to
  string second — never the reverse (UTF-8 replacement characters must
  not split needles). Replacement is longest-needle-first so
  overlapping needles from multiple secrets resolve deterministically.
  Matches are replaced with `[secret:NAME]`.
- **One choke point, three surfaces.** In the exec arm, immediately
  after the child is captured and before anything else touches the
  bytes, masking covers:
  1. captured **stdout**;
  2. captured **stderr** — before the driver's own stderr re-emit at
     `adapters.rs:490`, whose tail `conclude_single`
     (`engine.rs:593`) journals into failed/indeterminate outcomes —
     the path that fires exactly when a credentialed command fails and
     prints the offending header;
  3. the child-written **result payload** (`.forge/results/<id>.json`
     as read by the driver) before it becomes `Body::Result` — a child
     that echoes `$TOKEN` into its result `notes` must not put
     plaintext into the append-only journal via `EffectSucceeded`.
  Every byte of child output that can reach a checkpoint, the journal,
  logs, or the UI provably passes through this point, so no
  engine-side or UI-side re-masking exists; the layer-6 proof, which
  scans the journal rather than the masker, is the guard against any
  future path that routes around it.
- **Buffered-only invariant, written down**: today the exec arm
  captures complete buffers (`wait_with_output`), so whole-buffer
  masking is sufficient. Any future streaming capture must carry an
  overlap window ≥ the longest needle or it silently reopens the leak;
  this invariant is recorded as a doc comment on the masker and in this
  spec so a one-line "optimize to streaming" PR cannot drop it
  unnoticed.
- Exact matching against known literals only — no entropy or blocklist
  guessing. UI (`forge ui`) reads only journal envelopes via
  `store.load`, so journal-side masking covers it with zero UI code
  (verified by the panel; `ui.rs`/`ui.html` untouched).

## Layer 6 — Journal invariant (machine proof)

A machine proof in `crates/forge-cli/tests/machine_proof.rs`
(scripted-child pattern per `driver_conformance.rs`): bind a secret,
run an exec effect whose child prints the value in **every** listed
encoding to **stdout and stderr**, and also writes it into its **result
notes** — the proof must cover the result-file path or it proves the
wrong thing. Then byte-scan every journal envelope for the value and
every needle from the shared layer-5 constant (iterated from that same
constant, not a hand-copied list). Zero hits or the proof fails. A
companion proof does the same for a *failing* child (exercising the
stderr-tail journal path).

## Checkpoint-target amendment (exactly as 0012 states)

- **Unchanged**: resolved command lines, URLs, and prose are never
  journaled.
- **Amended**: a command template whose secret references are
  unresolved MAY be journaled where a target is otherwise recorded,
  subject to the existing 80-char clamp. This design journals the
  **pre-substitution charter template string** (`{{secret:NAME}}`
  spelling, parts joined with spaces, clamped): it is the exact
  artifact the compile lint proved value-free, whereas post-substitution
  text embeds `{workdir}`/`{prompt_file}` expansions that are not the
  recorded contract. "Journal the thing linted at compile time" is a
  property a test can hold forever.
- **Unchanged**: the claude fold's file-path-only discipline
  (`adapters.rs:135–176`) is untouched; seat-authored (model-authored)
  Bash commands remain unjournaled.

## Non-goals

- No journaling of resolved command lines, URLs, or prose; no
  relaxation for model-authored Bash telemetry.
- No blocklist/entropy secret detection; no encryption at rest (an
  encrypted store needs a key, which needs a store — that regress is
  Vault's job); no remote/managed backends and no backend trait for a
  single implementation.
- No `forge secrets get`; no per-step scoping, TTLs, audit log, or
  rotation history; no new protocol message kinds; no streaming
  masker; no `sh -c` wrapping.
- No changes to `policy/phase-machine.json`, `reference/`, `fixtures/`
  (frozen corpus), or frozen v1 contract files — the `secrets` key is
  bundle-format only.
- An actively adversarial child that re-encodes a secret into an
  unlisted shape defeats masking by design; that is the confinement
  boundary's problem (`driver.confine`), per 0012's own text.

## Acceptance Criteria

Each criterion is testable; unit tests live beside the code they prove,
machine proofs in `crates/forge-cli/tests/machine_proof.rs`.

1. **Name grammar and reference scanning**: `[A-Z][A-Z0-9_]*`
   accept/reject vectors; the scanner finds well-formed references and
   classifies malformed ones (lowercase, empty, spaced, unclosed).
2. **Compile lint fails closed**: undeclared reference, malformed
   `{{secret:`, ill-formed declared name, denylisted name
   (`PATH`, `FORGE_X`), and a `secrets.env` file inside the bundle dir
   each refuse compile with the offending name/path in the error;
   a declared-and-referenced template compiles;
   declared-but-unreferenced compiles.
3. **Store round-trip and hygiene**: set/list/remove round-trip;
   `0600` on create; atomic replace preserves mode; read of a
   broader-than-`0600` file refuses; `--secrets-file` override works;
   `list` output contains names and no values; `set` refuses empty,
   multi-line, NUL, <4-byte, denylisted, and grammar-violating input;
   value arrives via stdin.
4. **Digest stability end to end**: set → compile → rotate the value →
   compile → `manifest_digest` byte-equal.
5. **Injection discipline**: post-resolution template text contains
   `$NAME` and never the value; spawned child argv never contains the
   value; child env contains it; every declared name is injected even
   when unreferenced; a missing name refuses determinately before
   spawn, naming the name; declared name overrides pre-existing env.
6. **Secret type**: `Debug` renders `Secret(REDACTED)`; no `Display`
   (compile-fail test); drop zeroizes where observable; the CI grep
   test finds exactly one `expose_for_spawn` call site outside
   `secret.rs`.
7. **Masker**: every needle in the shared constant (raw, base64
   std/URL-safe × padded/unpadded, hex lower/upper, percent
   upper/lower) replaced with `[secret:NAME]`; multiple secrets with
   overlapping needles resolve longest-first; text containing no needle
   passes through byte-identical; masking applied to bytes (invalid
   UTF-8 around a needle still masks).
8. **Journal invariant (machine proof)**: scripted child leaks the
   value in all listed encodings via stdout, stderr, and result notes;
   byte-scan of every journal envelope, iterating the shared needle
   constant, finds zero hits — for both a succeeding and a failing
   child.
9. **Amendment**: an exec effect journals the unresolved charter
   template as its checkpoint target within the 80-char clamp; the
   resolved command line appears in no envelope; claude-fold
   checkpoints remain file-path-only; model-authored Bash remains
   target-less.
10. **Workspace green**: `cargo test` across all crates passes; the
    frozen corpus and contracts v1 files are byte-untouched
    (`git diff --stat` empty for `reference/`, `fixtures/`,
    `contracts/`, `policy/`).
