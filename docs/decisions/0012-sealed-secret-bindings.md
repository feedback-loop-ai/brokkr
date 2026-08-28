# 0012 — Sealed secret bindings: seats reference secrets, only the runner resolves them

Status: accepted (operator ruling in chat, 2026-08-28)

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
