# 0012 — Sealed secret bindings: seats reference secrets, only the runner resolves them

Status: accepted (operator ruling in chat, 2026-08-28)

Built: built

## Context

Checkpoint targets journal file paths only. Commands, URLs, and prose are
banned from the journal because they routinely embed inline credentials,
and the journal is append-only and hash-chained — a secret journaled once
can never be scrubbed. The cost of that ruling is telemetry density: most
seat turns are Bash, so most drill-down rows carry no target.

The ban treats the symptom. The disease is that a secret value can appear
inside a command at all. If commands structurally cannot contain secret
values, command *templates* become safe to record.

Prior art: prepared statements (code/data separation), 1Password `op run`,
`sops exec-env`, systemd-creds, GitHub Actions secret masking.

## Decision

Secrets are **referenced by name, never written by value**, anywhere a
seat or bundle can reach. Six layers, each independently testable:

1. **Reference syntax + declaration.** Exec-driver command templates may
   contain `{{secret:NAME}}`. A seat's charter declares the secret names
   it binds (`secrets: ["NAME", …]`), exactly parallel to decision 0007
   input provenance: undeclared names never resolve. Bundle compile fails
   on a template referencing an undeclared name (constitutional-lint
   shape). `NAME` matches `[A-Z][A-Z0-9_]*`.
2. **Operator-side store.** `forge secrets set|list|remove NAME` manages
   an env-format file outside the bundle and outside version control
   (default `.forge/secrets.env`, created `0600`; overridable via
   `--secrets-file`). Bundles and their digests carry names only —
   rotation never changes a digest. `list` prints names, never values.
3. **Injection discipline.** Values reach the child **only via the child
   environment**, resolved at spawn time by the runner — never via argv
   (argv is world-readable in `/proc/*/cmdline`), never via the template
   substitution itself. `{{secret:NAME}}` in a template resolves to the
   literal shell-safe env reference (`$NAME`), not the value.
4. **A `Secret` type that cannot leak accidentally.** No `Display`;
   `Debug` prints `Secret(REDACTED)`; best-effort zeroization on drop;
   the plaintext is reachable through exactly one method with exactly
   one production call site (the spawn injector). No new dependencies
   unless the implementing seat justifies one in its result notes.
5. **Known-plaintext masking on captured streams.** Before any child
   stdout/stderr reaches the journal, checkpoints, logs, or the UI, it
   is scanned for every bound value and its common encodings (base64,
   hex, URL-escaped) and replaced with `[secret:NAME]`. This is exact
   matching against known literals, not blocklist guessing.
6. **Journal invariant.** A machine proof runs an effect with a bound
   secret whose child prints the value in several encodings, then
   byte-scans every journal envelope for the value and all listed
   encodings. Zero hits or the proof fails.

### Amendment to the checkpoint-target ruling

Unchanged: resolved command lines, URLs, and prose are never journaled.
Amended: a command **template whose secret references are unresolved**
(`{{secret:NAME}}` / `$NAME` spelling) is not a secret-bearing value and
MAY be journaled where a target is otherwise recorded, subject to the
existing 80-char clamp. Seat-authored (model-authored) Bash commands
remain unjournaled: the model can also embed non-credential sensitive
prose, and this decision does not relax that.

## What this does not promise

A child process that deliberately transforms a secret into an unlisted
shape before printing defeats masking. The guarantee is layered upstream:
the model never holds the value (a seat cannot paste what it never saw),
argv never holds it, and literal-or-common-encoding leaks are caught. An
actively adversarial child binary is the confinement boundary's problem
(`driver.confine`), not this mechanism's.

## Consequences

- Exec steps can use credentialed commands (gh, curl with tokens) without
  the bundle, journal, or telemetry ever holding a value.
- The Secret type, resolver, lint, masker, and store round-trip are unit
  tested; the journal invariant is a machine proof; the single-call-site
  property of layer 4 is enforceable by grep in CI.
- Future relaxations of telemetry (journaling Bash activity classes,
  transcript deep-links) build on this floor and get their own decisions.

## Addendum — 2026-11-20, operator ruled: the journal names a driver's stderr only by content-free evidence

Status: accepted (operator, 2026-11-20, #1199 "(a)")

Until this addendum a failed or indeterminate attempt's journaled error
or reason carried the last 2000 characters of its driver's stderr as a
"stderr tail". A built-in driver masks that stderr by the seat's own
bindings only, so a credential a harness printed that the seat does not
bind reached the append-only journal, where it can never be scrubbed.

The rule: **the journal names a driver's stderr only by content-free
evidence.** A failed or indeterminate attempt's error or reason ends
`; driver stderr: <evidence>`, the evidence being the retained stderr's
length, the class of its first byte and a prefix of its SHA-256, as a
refused driver message is named (#433); an empty stderr reads
`; driver stderr: empty`. One type, `brokkr_protocol::process::DriverStderr`,
derives it. A spawn failure has no stderr and names none, as before.

The masked tail the reason used to carry goes to the engine's own
stderr, each line prefixed by the attempt id, and is never journaled:
the driver log under `brokkr dispatch`, the terminal under `brokkr run`
and `resume`.

The journal is append-only, so events written before this addendum keep
their tails and every reader still reads them. Nothing migrates, and no
contract or event schema changes: `error` and `reason` stay strings.

**2026-11-21, round two (#1199).** The review of the first build found
two paths already on `main` that still carried a harness's or a
validator's stderr into the journal, and the controller ruled them closed
under the operator's ruling "(a)":

- A dialect check's `notes` joined its validator's stdout and stderr, and
  the engine journals those notes as a step checkpoint or as the attempt's
  result. Its notes are now its validator's trimmed stdout alone; where
  the validator wrote to stderr, one last line names it by the same
  evidence, `validator stderr: <evidence>`, counted and digested over the
  masked bytes captured, never a lossy reading of them, and an empty
  stderr adds nothing. The verdict is unchanged. A realm that wants its validator's
  diagnostics in the notes redirects them to stdout inside its own command.
- A Codex model read from the `model:` header on stderr, when the stream
  and the rollout named none, went into the `codex-session-finished`
  checkpoint. It now counts only when it equals the seat's pinned model,
  the `--model` or `-m` the engine resolved into the seat's passthrough,
  read in every spelling the compiler's pin reader admits (`-m VALUE`,
  `-m=VALUE`, `-mVALUE`); any other header, or a seat with no pin, leaves
  the model `not reported`.

And the engine tells a failed attempt's masked tail only after it has
appended the attempt's terminal event (the failure, the indeterminate
park, a sequence step's failure or indeterminate, and the failure a
seat-record fence makes of a step's refused result) and any park that
ending takes, through a writer it holds, best-effort: forming the reason
does no I/O, and a stalled or closed stderr can no longer keep an attempt
from settling or hold the run between its failure and its park. A
dialect step keeps its driver's stderr for that tell, so a validator a
signal kills still leaves its tail on the engine's stderr.

**2026-11-21, round three (#1199, proposed).** The Codex stderr header is checked
against the pin in the seat's own validated passthrough. The driver uses
`--model` and Codex's declared `-m` there. Other built-in adapters use
`--model` alone for refusal attribution. The engine queues the masked tail
while it appends the terminal event and any park due on that ending, then
tells it after those appends. A terminal event held for journal contention
keeps its tail until the event lands and its turn's appends finish.

**2026-11-21, pointer (#1242; accepted 2026-11-23).** How a built-in fold reads a stream row that layer 5 masks, its structure raw and its text masked, is decided in [0114](0114-a-fold-reads-a-rows-structure-as-written-and-its-text-masked.md).

## Addendum — 2026-11-21, accepted under the operator's standing ruling of 2026-11-04: the union of every occurrence is masked (#1213)

Layer 5 replaced each match as a left-to-right scan found it, longest
needle first, and skipped the bytes it replaced, so a needle that started
inside an earlier match was never tried at its own start and the rest of
it was written in clear: `prefix!Z` bound beside a value starting `Z`
masked the first and left the second's other bytes readable.

1. **The union is masked.** Every byte any occurrence of any needle
   covers is masked: each bound value raw and in every listed encoding,
   and, where a projected transcript is masked, each value's JSON string
   escaping too, all in one union. Occurrences that overlap or touch are
   one run. Each needle is found in one linear pass, so the cost stays
   linear in the text times the needles, with no new dependency.
2. **One marker per run.** A run reads one `[secret:NAME]`, its form
   unchanged, naming the binding whose occurrence starts first; of those
   starting at one byte, the longest needle's, as before. A run that
   holds one occurrence reads exactly as it did. A union can make two
   keys of one JSON object one text, as `abcd` and `abcdefg` do with
   `abcd` and `defg` bound. `mask_json` keeps one member for them, as it
   did before. Where that matters, the caller detects it: the broker's
   output edge refuses it as the server's protocol fault. What an
   adapter's stream rows read as structure is #1242's.
3. **The guard sees what the masker would.** The final guard refuses
   bytes holding a bound value whose start an earlier match covers, as it
   refuses any other occurrence.
