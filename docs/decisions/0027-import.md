# 0027 — Import: journals never merge, runs relocate

Status: proposed
Date: 2026-09-01

## Context

`brokkr export` has always been half a verb. It writes a run's canonical
NDJSON and its pinned manifest — portable, hash-chained, verifiable
offline by a stranger — and then there was nowhere to put it back. A run
driven in one journal stayed in that journal forever, or stopped being
evidence.

The pressure is concrete. Five wave-1 fire journals sit in sibling
`forge-w1-*` worktrees, each holding runs that belong in the canonical
journal. The available answers were all bad: copy the `.db` file and lose
whatever the destination held; open both databases and `INSERT` across
them by hand, with no verification and no record that it happened; or
re-drive the work, which forges new evidence and throws away the old.

Two things had to be settled before code could be written, and they pull
against each other. **Adoption must be verified** — a journal that
accepts unverified events is not a journal. And **adoption must be
invisible** — an adopted run has to be a run, not a second-class citizen
with a footnote, or every readout and every gate downstream grows a
special case.

The second requirement is the sharp one, because the obvious way to
record "this run came from somewhere else" is to write an event saying
so, and that is precisely what cannot be done. `state = fold(events)`
(README, determinism law 2) means every byte inside the chain is state
the phase machine can read. An `run/imported` event, or an arrival key in
the run manifest that rides inside `run/started`, would make "was this
run adopted?" a foldable fact — and then a policy table could branch on
it, and an adopted run would stop being the same kind of thing as a
native one.

## Decision

1. **Import is single-run relocation, never merge.** Two journals are
   never combined. One run moves from a canonical export into a
   destination journal's `runs` and `events` tables, byte-identically.
   `brokkr import --from <run>.ndjson` is the whole verb; the
   `<run>.manifest.json` sidecar is read from beside it and is
   **required**, because the sidecar is where an export declares itself
   redacted and an import that shrugged at a missing declaration would
   accept exactly the pair whose declaration went missing.

2. **Every gate passes before a single event lands.** In order, and all
   of them refusing the import WHOLE:

   - **A redacted derivative is refused by name**, before its content is
     read at all — the `.redacted.` filename `export --redact` writes, or
     a `"redacted": true` sidecar manifest. Redaction rewrites payload
     bytes and leaves the recorded hashes behind, so a redacted export's
     chain can never verify. There is no `--force`: the only import of a
     redacted export is an adoption of unverifiable content, which is
     the opposite of what the gate is for. Two marks, either sufficient,
     so a copy somebody renamed back is still caught.
   - **The chain verifies whole.** One broken link refuses the import,
     never a good prefix of it. There is no partial adoption.
   - **The events fold**, and a `FoldError` refuses with the fold's own
     citation — the same words a quarantined run shows anywhere else.
   - **A run_id collision refuses outright.** No rename, no overwrite, no
     dedup, no quiet no-op. The run_id is hashed into every envelope of
     its chain, so renaming it on import would invalidate every hash:
     collision is structurally not a rename-and-retry situation. It is
     the operator's problem to rule on, and an importer that papered
     over it would be ruling.

3. **Adoption is not idempotent — it is once.** A second import of the
   same export refuses exactly as a genuinely different run sharing a
   run_id does. This is ruled explicitly, and named as its own test, so
   "import is idempotent" is never assumed by a caller or a future
   retry loop.

4. **The adopted events are written verbatim.** Same bytes, same hashes,
   same seqs, same `recorded_at`, same run_id. `Store::append_next` is
   therefore the wrong primitive by construction — it seals a *fresh*
   envelope from the destination's head — so import gets its own
   low-level append of pre-sealed envelopes, all of them in one
   transaction or none of them.

5. **Arrival is recorded BESIDE the chain, never inside it.** Not an
   `EventType`, not an event, not a key in a new
   `contracts/run-manifest.v*` — that manifest rides inside
   `run/started`'s payload, which is inside the chain. Two additive
   columns on the `runs` table, `imported_at` and `imported_from`, which
   is already store bookkeeping beside `events` and already how
   `list_runs` and `manifest` answer "what runs exist." NULL means
   native. `brokkr_core::fold` cannot observe any of it, so law 2 holds
   identically for a native run and an adopted one — and if arrival ever
   *needs* to affect a decision, that is a future ruling, not a
   consequence of this one.

   This deliberately forgoes the additive-contract-version escape hatch
   that `run-manifest.v5` and decision 0021 used. An additive contract
   version would have been the easy move and the wrong one: it would
   have put the fact inside the chain, where fold can see it.

6. **`DATABASE_SCHEMA` does not move, and the columns migrate on open.**
   The version guards *compatibility*, and columns nobody selects break
   nothing in either direction: an older binary reads a migrated journal
   exactly as before, and this binary migrates an older journal the
   moment it opens it read-write. Bumping the version would refuse both
   — an old binary against a new journal, and this binary against the
   five wave-1 journals it exists to adopt from — and buy nothing.

7. **The `runs` row is derived from the verified chain, not the
   sidecar.** The run manifest rides inside `run/started`'s payload, so
   feature and manifest come from bytes a hash covers; `created_at` is
   the first event's `recorded_at`, so the adopted run sorts into the
   fleet where it actually ran. The sidecar manifest — which no hash
   covers — is consulted for exactly one thing, its redaction marker,
   where trusting an uncovered file can only ever cause a refusal. An
   export whose `run/started` carries no `feature` or no `manifest` is
   refused rather than filled in from the sidecar: a readout that shows
   this run must not show a blank where the chain said nothing.

8. **Provenance is queryable, not surfaced.** `Store::arrival(run_id)`
   answers "how did this get here" without reading an event, and
   `brokkr import` prints what it recorded. `brokkr runs`, `brokkr tui`
   and `brokkr inspect` are unchanged and show no difference — the
   `RunEntry` they build from has no field an arrival column can reach.
   A dedicated provenance readout is deferred until an operator wants
   one; the columns being queryable is what this decision rules.

9. **A verified chain does not vouch for the name it was sealed under.**
   Event hashes are unkeyed sha256 over the envelope's own content, so
   `verify_chain` proves that no byte of an export was altered since it
   was sealed — never that whoever sealed it was entitled to the run_id
   it carries. Anyone can seal a self-consistent chain under any name at
   all. Import is the first path by which an externally authored run_id
   reaches the `runs` table: engine ids are a feature slug and eight hex
   characters, and dispatch ids ride an envelope this journal already
   verified. And the name does not stay in the database — `brokkr
   export` composes `<out>/<run_id>.ndjson` from it, and every readout
   prints it. So adoption gates the id to what this journal would itself
   have minted: 1 to 128 characters of ASCII letters, digits, `-` and
   `_`. That excludes `.` and so `..`, the path separators, and every
   control and bidi character. The gate sits after verification (so it
   judges an id the chain agrees on) and before anything is derived
   from, stored under, or printed with the name — which is also what
   makes `Collision`'s message and the CLI's success line safe to print:
   by the time either can name a run_id, the gate has passed it.

## Consequences

The five wave-1 fire journals can relocate, each run verified twice on
the way — once as an export, once as an adoption — and the canonical
journal gains them without losing anything it held. A collision stops
being a silent overwrite and becomes a refusal an operator reads.

Import is a store-and-CLI feature built entirely on the existing
verification primitives. `EventEnvelope`, `EventType`, `verify_chain`
and `fold` are untouched, and `contracts/event-envelope.v1.schema.json`
stays frozen: the verb needed no new journal content, which is the
strongest evidence that arrival never belonged in the chain.

What this does not do: merge journals, rename a colliding run, or import
a redacted export under any flag. Those are not deferrals — they are
ruled out.
