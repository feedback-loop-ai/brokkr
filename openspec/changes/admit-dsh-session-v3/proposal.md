# Change: Admit DSH session format version 3 on measured evidence (#279)

## Why

PR #278 (merged `d330765`) taught discovery the filename the installed
DSH core writes: `session.v3.jsonl` is now found beside `session.jsonl`
and its path is confirmed. The reader then refuses the content with
`unsupported-format`, because the living `transcript-reading` requirement
admits only an opening header whose `version` is numerically zero and
says in terms that there is no legacy default version and no automatic
migration. That refusal is correct and specified. Issue #279 asks for one
of two outcomes: replace the refusal with a measured admission of version
3, or state precisely what must be built first.

This change delivers the second outcome, and records why. Version 3
exists because something changed in the writer. The commission is explicit
that the evidence for admitting it must come from reading the installed
`@deepseek-ai/dsh` 0.1.5-rc.1 writer, not from sampling more session files,
and that an unmeasured part stays refused rather than guessed. Version zero
was admitted exactly that way: the #222 council held a controller capture of
the 0.1.2-rc.1 session and persistence-JSONL packages, 42 source texts with
SHA-256 digests, and the version-zero rules (event vocabulary, packed rows,
citations, header admission) cite those files by name and line. The seat
that authored this change reaches no such evidence for 0.1.5-rc.1: its
hands are boxed to this worktree plus read-only system paths, and every
probe for the writer came back empty (S1). The six questions in #279
therefore cannot be answered from primary evidence here, and the honest
specification is one that names the admitted versions, says why version 3
is not among them, records what each question needs, and pins everything
that *is* measurable from the repository's own evidence.

## What Changes

- The `transcript-reading` requirement "Discovery identifies one owned
  local file" names the admitted DSH version set explicitly. It is closed
  and is exactly `{0}`. The requirement states the evidence behind zero
  (the captured 0.1.2-rc.1 writer) and the reason 3 is refused (the
  0.1.5-rc.1 writer has not been read). It states how the set grows: a
  version joins only through a change that records, from the writer's own
  source, its package identity and captured files with digests and answers
  the six questions of #279. A versioned filename, a header number and the
  resemblance of sampled rows to admitted shapes are named as non-evidence.
- The requirement states that the filename and the header version are
  independent facts: `session.v3.jsonl` with a version-zero header is
  readable, `session.jsonl` with version 3 is refused. Neither infers the
  other.
- `isSeeded` joins the list of header fields that neither qualify nor veto
  the read. It does not establish or remove ownership, does not alter the
  delegation-depth rule and does not supply admission. Its meaning is
  unmeasured and no reader claim rests on it.
- Four scenarios pin what is measurable without the writer: a version-3
  file is located and refused with its confirmed path and zero counts, and
  no later row gains a disposition by being present; filename and version
  are independent; a seeded header is inert for ownership and admission;
  and under version zero the three types the v3 sample surfaced already
  have rulings from the 0.1.2-rc.1 catalogue, so `todo/write` and
  `turn/end` are recognized quiet and `system/message` is a required
  unknown. That last pin is the baseline against which version 3's
  dispositions will be ruled once its writer is read.
- No projector production change is made. The reader already behaves as
  the amended requirement states; the change adds the specification and
  the host-independent tests that prove it, so that the future admission
  change starts from pinned ground rather than from behaviour that happens
  to hold. No numbered decision is proposed by this change (S5).
- The precise prerequisite for admitting version 3 is recorded (S4): a
  controller-supplied capture of the installed 0.1.5-rc.1 packages in the
  form the #222 run used, or a seat whose box mounts the global install
  read-only. When that evidence exists, this change is returned to
  specification with the finding and the six answers are recorded in the
  requirement; nothing is admitted downstream by inference.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `transcript-reading`: the requirement "Discovery identifies one owned
  local file" names the admitted version set, its evidence and its growth
  rule; adds `isSeeded` to the inert header fields; and gains the four
  scenarios above.

## Impact

- `openspec/specs/transcript-reading/spec.md` on fold.
- `crates/brokkr-view/src/transcript/tests.rs`: header matrix and
  disposition tests for the four scenarios (version 3, `isSeeded`, the
  three sampled types under version zero).
- `crates/brokkr-cli/src/ui/tests.rs` and
  `crates/brokkr-cli/tests/transcript_command.rs`: the located-and-refused
  `session.v3.jsonl` read through discovery, CLI text/JSON and the TUI
  refusal state, with synthetic test-owned homes.
- No change to `crates/brokkr-view/src/transcript.rs`,
  `crates/brokkr-cli/src/ui.rs`, `crates/brokkr-protocol`, frozen
  `contracts/`, `policy/`, `reference/` or `fixtures/`. No change to the
  `transcript-command` or `transcript-tui` capabilities: their observable
  behaviour for a refused header is already specified and unchanged.
- The exact-coverage gate is unchanged; test-only additions cannot lower
  it.

## Decisions

### S1 — Evidence and its limits: the writer is not reachable from this seat

The commission names the primary source: the `@deepseek-ai/dsh` 0.1.5-rc.1
writer as installed on this host. This seat's hands are boxed to the
worktree at `brokkr-fire-279`, the shared `.git` of the main checkout, and
read-only system paths (`/usr/bin`, `/usr/lib`, `/usr/local`, `/bin`,
`/lib`). The following probes were made on 2026-09-13 and every one came
back empty:

| Probe | Result |
|---|---|
| `which dsh`, `which npm` | not found; `node` v22.23.2 is present |
| `/usr/local/lib/node_modules` | holds only `@fission-ai` (OpenSpec 1.12.0) |
| `find / -iname '*deepseek*'`, `-name dsh` | nothing outside the worktree |
| `~/.dsh` under the box's home and under `/home/vyanakiev` | not mounted |
| `.forge/` in this worktree | only the triage framing and its result; no capture like the #222 run's `.forge/controller-dsh-transcript-storage-interface.json` |
| network (`api.github.com`, `registry.npmjs.org`) | DNS resolution fails |
| `git grep` for `isSeeded` and `known-event-types` across every local branch head | only the archived #222 design; no branch carries the writer or a capture |
| `git log --all -S'controller-dsh-transcript-storage-interface'` | four #222 commits that *cite* the capture; the capture itself was run-local and never committed |

The 17 version-3 and 146 version-zero sessions under
`~/.dsh/sessions/brokkr` are equally out of reach. Their counts and the
row-type observations below are therefore cited as reported in issue #279
and relayed by the commission, not as measurements of this sitting. The
commission ranks them as corroboration only, and this change treats them
that way.

What this seat *can* read is the repository's own record of the
version-zero writer: the archived #222 design (D6 and its evidence table)
and proposal (S11), which cite the 0.1.2-rc.1 capture by file and digest;
the living requirement; the projector at
`crates/brokkr-view/src/transcript.rs`; and the #278 change. Every claim in
S2 and S3 rests on one of those, and each says which.

### S2 — The admitted version set is closed, named, and version 3 is outside it

The living requirement said "a JSON number equal to zero" and "no legacy
default version or automatic migration" without saying why zero and not
some other number. The amended requirement says why: version zero is
admitted because its writer was read. The #222 capture's session
`lib/index.js` line 57 declares `SESSION_FORMAT_VERSION = 0`; the
persistence-JSONL `lib/index.js` (SHA-256
`dfd6cde28928996f2f44220d013359563c8d9bf6c9c904972e77256767eec385`) writes
that header and refuses a foreign numeric version at lines 214–236 before
decoding; `lib/types/known-event-types.js` (SHA-256
`e7aee13d1dd119fa2c2ef6818eada27e547b2f20bccdbd4f1dadde0012a8c4f7`) is the
closed vocabulary behind the quiet list; `lib/types/chunk-rows.js` and
`lib/types/seq-ranges.js` are the packed-row and citation rules. Those are
the archived proposal's S11 and design's D6 facts, restated so the
requirement carries its own reason.

Version 3 is refused for the mirror-image reason: nobody in this repository
has read the 0.1.5-rc.1 writer. The alternative, widening the numeric gate
to `{0, 3}` because seventeen sampled files look like version-zero files
with a few extra types, is rejected on the commission's own terms and on
decision 0004's: absent evidence is no claim. It is also rejected on the
merits. The sample already shows that `assistant/chunk` is absent (S3, Q2).
If version 3 persists whole messages only, then the living rule that each
fragment is its own logical event and is never concatenated into an
invented assembled message changes *behaviour* without changing text: an
interrupted step would leave no retained text at all, and the scenario "An
interrupted DSH step retains its chunks" would be vacuous for version 3. A
reader that admitted 3 today would display such a file as if nothing were
missing. That is exactly the invented completeness the reader refuses
everywhere else.

The set is therefore closed and named, and the requirement states the only
way it grows: a change recording the writer's package identity and captured
files with digests, plus the six answers. This mirrors #278, whose delta
closed the filename set "so that adding a name is a decision with evidence
behind it".

### S3 — The six questions, answered as far as the evidence reaches

Each row states what the repository's own evidence establishes, what the
reported sample suggests, why the sample cannot decide it, and which writer
fact decides it. Unless a row says "measured", the question stays open and
the affected behaviour stays refused with the version.

| # | Question | What is established here | What the reported sample suggests | What must be read in the writer |
|---|---|---|---|---|
| 1 | The record types and shapes the v3 writer emits | Only the version-zero vocabulary is measured: five content kinds and 46 quiet kinds (51 in the 0.1.2-rc.1 catalogue), three packed storage rows, and the header shape. Nothing about v3's vocabulary is measured. | At least `session` (header), `system/message`, `todo/write`, `turn/end`, `user/message`, `assistant/message`, `tool/call`, `tool/result`. A sample lists what seventeen runs happened to emit, a lower bound, never the vocabulary. | The 0.1.5-rc.1 successor of `known-event-types`, the persistence sink's row kinds (packed rows included), the header constructor, and the file-naming code that chose `session.v3.jsonl` and the header value `3`. |
| 2 | `assistant/chunk` is absent from the v3 sample | Under version zero fragments persist two ways: ordinary `assistant/chunk` rows, and `text-chunks` / `reasoning-chunks` / `tool-call-chunks` packed rows decoded by `chunk-rows.js`. The reader treats each fragment as one logical event and never joins fragments. | Ordinary chunk rows are gone. Four hypotheses are open: (a) fragments are not persisted at all, whole messages only; (b) renamed; (c) always packed, so the census lists them under the storage names; (d) conditional, for example only on interruption or only outside seeded prefixes. | The sink's handling of chunk events: whether the bus still emits them to persistence, whether the sink filters, packs or drops them, and whether an interrupt or shutdown path flushes anything. Only (c) leaves the fragment and citation rules' behaviour unchanged; (a) makes the interrupted-step scenario vacuous and must be stated in the requirement; (b) and (d) need a mapping. |
| 3 | `isSeeded` is new in the header | Version zero headers carry `seedLength`, `parentSession` and `origin`, which the reader ignores; `session/end-seed` is a recognized quiet event marking the end of a seeded prefix. Ownership is `type: session` plus depth zero, and nothing else. **Measured here:** under the existing "other header fields" rule the reader already ignores `isSeeded`; this change names it and pins it (scenario "A seeded header changes neither ownership nor admission"). | A boolean, plausibly derived from a nonzero seed. | The header constructor and the seed-splicing code: whether seeded prefix events keep their originating `seq`, `turn` and `step` (which bears on citation uniqueness and on "earlier logical event"), whether `isSeeded` ever coincides with a positive `delegationDepth`, and whether a seeded root is still the seat's own file. Until then the reader claims nothing about provenance from it. |
| 4 | `surfaceOp` and `sourceEventSeqs` rise from under 1% to about 21% of rows | Under version zero `sourceEventSeqs` on `assistant/message` cites the chunks the assembly replaces; on `user/message` and `tool/result` it records a surface replacement that the reader deliberately does not replay, because the read is an audit of retained events (0055 ruling 3: `surfaceOp` never rewrites audit history). Suppression is limited to readable assemblies citing unique earlier same-turn/step chunks. Invalid citation grammar refuses the whole read. | The rise is consistent with Q2(a) or (c): every assembled message now cites the fragments it was built from, whether or not those fragments were persisted. It is equally consistent with a new use. The sample cannot tell these apart. | The v3 `surfaceOp` value set, the grammar of `sourceEventSeqs` (individual and `[start, end]` only, or new shapes), and which events carry them. Two consequences hinge on this: a new citation shape would refuse every v3 read under the current grammar, and a `surfaceOp` that removes content from the model surface must still leave the audit order intact, which is reader policy and does not change. |
| 5 | `system/message`, `todo/write`, `turn/end` appear in v3 and not in version zero | **Measured here, for version zero:** `todo/write` and `turn/end` are in the 0.1.2-rc.1 catalogue and are recognized quiet (`dsh_quiet_event`, design D6's list). Their absence from version-zero samples is a sampling fact, not a version difference. `system/message` is *not* in that catalogue; under version zero it is a required unknown that refuses the read unless the row carries top-level `ignorable: true`. Scenario "Version zero already rules the three sampled types" pins both. | All three occur in v3 files. Whether their payloads changed is unknown. | The v3 type definitions for the three. `system/message` needs its payload read before it can be ruled: if it carries the request's system prompt it belongs with `request/context` as quiet; if it carries an operator-facing notice it may be a displayed turn, which would introduce a `system` role that the command and TUI capabilities do not yet render, a design decision in its own right. `todo/write` and `turn/end` keep their quiet ruling only if their v3 shapes are still operational records. |
| 6 | Is version 3 a superset of version zero? | Not measured. Note that a superset of the *vocabulary* is not a superset of *meaning*: Q2(a) alone changes the fragment rule's behaviour without adding a type. | The sample resembles version zero plus three types and minus one. Resemblance is what the commission forbids as a basis. | A file-by-file comparison of the five principal captured 0.1.2-rc.1 sources against their 0.1.5-rc.1 successors. A per-version projection is needed if any of these moved: header and ownership fields, `seq`/`turn`/`step` semantics (seeded prefixes), packed-row shapes, citation grammar, or the persistence of fragments. Otherwise admitting `3` beside `0` with an extended vocabulary suffices, and the requirement should say so as a measurement. |

Two of the six therefore have measured answers *for version zero* (Q3's
inertness of the field, Q5's dispositions), and those are pinned here.
None has a measured answer for version 3.

### S4 — What must be supplied before version 3 can be admitted

Exactly one of the following, in the operator's hands:

1. **A controller capture** of the installed 0.1.5-rc.1 packages placed as
   run-local evidence under `.forge/`, in the form the #222 run used
   (`.forge/controller-dsh-transcript-storage-interface.json`: per file, its
   package-relative `path`, `sha256` and `text`, with each package's `name`
   and `version`). It must cover at least the session package's
   `package.json`, `lib/index.js` (the `SESSION_FORMAT_VERSION` declaration
   and header construction), `lib/types/types.d.ts`,
   `lib/types/known-event-types.js`, `lib/types/chunk-rows.js` and
   `lib/types/seq-ranges.js`; the persistence-JSONL package's `package.json`
   and `lib/index.js` (the file naming, header write and version check);
   and whichever modules contain the literals `isSeeded`, `system/message`,
   `surfaceOp` and `sourceEventSeqs`, found by searching each package's
   `lib/`. Those are the 0.1.2-rc.1 file names; the capture records the
   actual 0.1.5-rc.1 layout where it differs. Private transcripts, profiles
   and credentials are not part of it.
2. **A seat whose box mounts the global install read-only**, so that the
   same files can be read and digested from the seat, with the ledger
   written into the change's design.

With either in hand, this change is returned to specification with the
finding, the six answers are recorded in the requirement with their file
and line evidence, the admitted set is widened only as far as those answers
reach, and every new type's disposition is pinned by host-independent
tests. Real host sessions may then corroborate, never lead. Nothing in the
design, tasks or implementation phases of this change admits version 3
without that return.

### S5 — No numbered decision is proposed by this change

Decision 0055 ruling 3 already rules "Decode DSH numeric on-disk version
zero only … Missing, mistyped or foreign versions refuse as
`unsupported-format`". This change does not alter that semantics; it names
its evidence and its growth rule in the specification and pins behaviour
that already holds. #278's closed filename set carried no decision for the
same reason. A numbered proposal supplementing 0055 ruling 3 becomes
necessary the moment a second version is admitted, because that is a
semantic change to what the reader decodes; it belongs to the returned
visit of S4, where it can cite the writer evidence it rests on. Decision
0032's ownership, retention and privacy constraints are untouched: no real
session is read, copied or committed, and the tests use synthetic rows in
test-owned homes.

### S6 — Validation of this sitting

Strict OpenSpec validation was run over the whole tree after authoring.
Cargo is absent from this seat's box, so no format, clippy, test or bundle
gate ran here; they belong to the implementation seat before its activation
commit, and the exact-coverage gate to host validation outside the box. This
sitting commits only the proposal and the capability delta, unsigned, and
pushes nothing.
