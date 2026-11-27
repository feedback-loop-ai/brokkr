# 0075 — Harness onboarding in a day: a harness is a spec, a probe measures it, a new seat starts provisional, and evidence promotes it

Status: accepted — operator ruled in chat 2026-10-12
Date: 2026-09-28
Built: partial (#485) — the conformance probe (#500) and the provisional tier (#526) are built; the epic tracks the rest
Amended by: 0107

## Context

The agent space moves weekly. A vendor ships a new CLI (Gemini's, Grok's,
the next one), or a much better model lands on a provider Brokkr already
reaches, and the operator wants it seated the same week, ideally the same
day. The operator raised the question on 2026-09-28: "it could be gemini,
could be grok, it is a dynamic space". What would it take to add one?

Today the answer is about a week, spread over three separate costs.

**The code.** #347 measured about 20 edits across 6 crates to add a
harness:
- an `AdapterKind` variant, whose `supports()` default claims resume nobody
  decided;
- a launch builder and argument splitters;
- an invoke arm, an event folder and a refusal classifier;
- a `BROKKR_<H>_BIN` override;
- transcript kind and home resolution;
- the runtime's `built_in_model_driver` match, without which the new
  harness's model pins go unpoliced;
- the CLI's known list, `doctor` and the UI's discovery;
- the view's transcript projector;
- and a **new seat-record contract version**, because `transcript.kind` is
  a closed enum.

Only the model catalogue and the permission flags are data. #264 is the
same failure class: harness facts that live as Rust constants.

**The facts.** Every adapter field is a dated, measured claim. For a new
CLI they are found by a hand spike, and review rounds then find the ones
that were guessed:
- how it reports usage (#402 found Claude's counts double-counted per
  stream event);
- whether it can switch off its own tools and take an MCP server;
- what native egress it carries (#462 found dsh's web tools on in every
  realm);
- whether it inherits the operator's personal configuration (#467 found
  every unboxed Claude seat and every Codex seat starting the operator's
  MCP servers, with their credentials).

**The seat.** A new harness enters the roster the way Astra did: a
measurement, a decision (0045), and a recipe edit. It is promoted, or not,
on impression.

**The fast lane.** For a new *model*, rather than a new harness, it exists
already: a model reachable through OpenRouter or an OpenAI-compatible
endpoint becomes a dsh route in hours. But dsh declares
`hands: unsupported`, so such a seat records the `not applicable`
boundary and cannot hold a boxed office (#290).

**Alternatives weighed:**
- *Keep adding harnesses by hand.* This is proven, and it costs a week and
  a review arms race each time. It scales with the space's pace only if
  the operator stops wanting new things.
- *Route every new model through one multi-provider harness only.* This is
  the fastest, but it forgoes a vendor CLI that is materially better than
  the generic loop, and the space's best agents have so far been vendor
  CLIs.
- *Make a harness data, measure it with a probe, and seat it provisionally
  until evidence promotes it.* This is the one ruled here. It costs a
  refactor on the critical path (#226 → #348 → #347) and one new tool (the
  probe, #484), and it turns each later harness into a day's work.

Since it was drafted (refreshed 2026-10-12):
- The conformance probe is built: `brokkr probe harness` measures an
  agent CLI and writes the facts its adapter must declare (#500, which
  closed #484 on 2026-10-06).
- Ruling 5's provisional tier is built (#526, 2026-10-04, ported onto
  decision 0065). An adapter's model declares its tier, `forge.realms/v7`
  lists the offices a provisional model may hold, the compile refuses one
  anywhere else and never seats one as a gate, and `brokkr doctor` names
  each provisional model.
- Decision 0077, accepted on 2026-10-03, makes the capability broker the
  harness's child and measures a harness to exclude ambient MCP
  configuration. That is the mechanism for #467's isolation.
- Decision 0072, accepted on 2026-10-12, lets a harness hold full access
  inside a seat box: an engine-owned boundary around the harness process,
  with its credential outside. Whether a harness can stand in one is an
  adapter fact (`hands.seat`), measured like the rest.
- The contract lifecycle #361 asks for is decision 0082 (#643).
  Ruling 2's mint goes through it.
- The fast lane is proven. New models have entered as dsh routes within
  hours: a locally served GLM model, DeepSeek Flash, and Muse through
  OpenRouter. dsh's hands (#290) are still open.
- The critical path for rulings 1 and 2 is unchanged and still open:
  #226, #348, #347.

## Rulings

1. **A harness is a declarative spec plus at most one module.** The adapter
   file declares everything today's code restates per harness: the launch
   grammar, the mapping from the CLI's events to the journal, its refusal
   classes, effort, the tool-restriction and MCP mechanism, the transcript
   location and kind, and the resume shape. Code is allowed only where data
   cannot express the harness, and then in one module under the harness's
   name, with no edit elsewhere. A default that claims a capability (resume,
   hands, effort) is refused: every capability is declared.
   *Binding:* #347's acceptance. A test enumerates every `AdapterKind` and
   refuses a harness fact kept as a Rust constant outside its module (#264's
   class). The adapter loader refuses a harness missing a spec field.

2. **The transcript kind is open.** A seat record names its transcript kind
   from the harness's adapter, validated against the adapters loaded, not
   against a closed enum in a frozen contract. So a new harness needs no
   contract version. The change is minted through the contract lifecycle
   (#361, which decision 0082 rules), once.
   *Binding:* the seat-record schema accepts a declared kind; a test seats a
   fixture harness unknown to the schema's author and its record verifies.

3. **Every adapter fact is measured by the conformance probe.** The probe
   (#484) runs a CLI against a scratch repository and records each fact the
   spec declares, with the CLI's version, the host, the date and the
   evidence. A fact the probe cannot measure is `unmeasured`, as dsh's
   native capabilities are today, never assumed. The probe is proven against
   the three shipped harnesses, reproducing their adapters' measured fields,
   before it is trusted on a new one. Live probing needs real credentials,
   bound through decision 0012. It is an operator host step, never CI, and
   the no-real-providers gate still holds for the repository. The probe
   itself is built (`brokkr probe harness`, #500); what remains of this
   ruling is tracked under #485.
   *Binding:* a test that every adapter field carries a measurement reference
   to a committed probe report. Re-running the probe on a new CLI version
   reports drift against that report.

4. **Eligibility is derived, not argued.** From the probe's measured facts:
   - a harness that can stand in a seat box (`hands.seat` measured, decision
     0072) may hold full access inside it, at work sites, as that decision
     rules, and only after its own slice is measured and merged;
   - a harness whose own tools can be switched off and which takes an MCP
     server may hold boxed offices (decisions 0043 and 0046);
   - one that cannot may hold unboxed or tool-less offices only;
   - one whose native egress has no measured off switch is refused in every
     realm that has not granted that capability (decision 0065 ruling 4,
     restated here for new harnesses);
   - one whose user-scope configuration cannot be isolated (#467) is
     refused. Decision 0077 measures a harness to exclude ambient MCP
     configuration.
   *Binding:* the existing boundary and capability compile refusals consume
   the adapter's measured fields; a fixture harness of each kind is refused
   or admitted exactly as stated.

5. **A new harness, or a new model on an existing one, starts provisional.**
   A provisional seat may hold only the offices the operator lists for
   provisional seats, for example review-panel positions, research, and
   wager arms. It never holds a gate (a review chief) or the sole implementer
   of main-bound work until promoted. Its per-token price rows exist before
   its first seat, so its cost is visible from the first run.
   *Binding:* an adapter or route field `tier: provisional | promoted`; a
   compile refusal for a provisional seat in a gate or sole-implementer
   office; LaneTally's unpriced audit shows no provisional route unpriced.

6. **Promotion is on evidence, by the operator.** A provisional seat is
   promoted after a wager (#237) on real stories. The wager compares it with
   the incumbent on cost per accepted change (#224) and on review holds and
   rounds (#436). The promotion is a roster addendum, not a new decision per
   harness.
   *Judgment-guidance:* the operator's ruling, recorded in the roster
   addendum with the wager's run ids. No mechanism promotes automatically.

7. **The fast lane stays fast, and gains hands.** A new model reachable
   through an existing multi-provider harness is added as a route:
   - its alias and effort table;
   - one `BROKKR_BLESS=1` re-pin (#358);
   - its price rows;
   - a probe run;
   - and the provisional tier.

   The target is hours. dsh gains hands (#290), so that this lane can reach
   boxed offices.
   *Binding:* #290's acceptance, and ruling 4's eligibility applied to dsh's
   measured facts.

## Consequences

- **Targets:**
  - A new model on an existing harness is seatable, provisionally, in
    **hours**.
  - A new CLI whose shape matches one already supported is seatable,
    provisionally, in **one day**. The probe writes its facts, and no crate
    changes beyond its one module.
  - A new shape takes longer, and pays for that shape once.
- **Critical path:** #226 (safe resumption across harnesses) → #348 (split
  brokkr-protocol) → #347 (a harness is one module). Rulings 1 and 2 wait on
  it. Rulings 3 to 7 do not: the probe (#484, built by #500), config isolation (#467),
  native-capability declarations (#319, #462), dsh's hands (#290) and the
  provisional tier can start now.
- **What it costs:**
  - one new compile concept (the tier);
  - a probe that needs live credentials on the operator's host;
  - a contract version for the open transcript kind, minted once.
- **What it changes for existing seats:** nothing until each ruling lands.
  claude, codex and dsh are promoted today by their current rosters.
- **Tracking:** epic #485.
- **Related decisions:** 0012 (secret bindings), 0036 (egress is a property
  of the route), 0043 and 0046 (hands and the boundary), 0045 (the Astra
  roster, whose pattern this decision generalises), 0065 (capabilities are
  the realm's to grant), 0071 (the architectural principles), 0072 (full
  access inside a seat box), 0077 (the broker as the harness's child), and
  0082 (the contract lifecycle).

## Addendum — 2026-11-21, operator ruled: a provisional harness's stream confirms its served model

Accepted by the operator on 2026-11-21 (#1178, ruling "1").

A provisional seat is priced and judged on the model it was pinned to,
so its stream must prove that model served it. Qoder CLI is the first
case (#1178; `docs/evidence/adapters/qoder-cli-1-1.md`). Four rounds of
review found a value escaping a mask or a rendering in a new place each
time, because the pin and the served model ran as free text through
masks, `Debug` renderings and error words. These rulings close that
class:

1. **The pin is a plain id.** It is 1 to 80 characters. The first is an
   ASCII letter or a digit, so no pin reads as a flag, and each other is
   an ASCII letter, a digit, `.`, `_` or `-`. It is checked once, at launch,
   before spawn, and a pin that fails is refused in fixed words naming
   no value. A plain id has no escaped form. It also holds no character
   of the mask's replacement text, so no mask can turn another model
   into the pin.
2. **Exactly one identity row names it.** The driver's own rules read
   each row as the harness wrote it. The shared fold reads only a copy
   the seat's bindings masked, so its classifier, the session record and
   every row emitted read masked text alone. One identity row
   naming exactly the pin confirms the model. One naming another model,
   a second identity row, or a turn before any confirmation refuses the
   seat.
3. **Any shape no measured stream carries refuses.** For Qoder, that
   is a stdout line that is not UTF-8 or not a JSON object, an empty
   line included. No line ends the read early, and none is skipped.
4. **A fallback notice is a substitution wherever the stream stops.**
   The harness's own notice that it served another model refuses the
   seat as substituted, whatever the identity row said. A refusal
   before the seat's first turn and before any delivery (item 8) is a
   refusal to start. After a turn, or after delivery, the seat ends with
   no result, and its error keeps the refusal's words.
5. **Refusal words name only plain ids.** Words name an id by
   `Display`, in single quotes. They name the served model only when it
   is a plain id, and otherwise say "a model that is not a plain id". No
   other value from the stream, its stderr, the environment or a path
   appears in them. Every word the driver returns passes the seat's
   bindings once, last. The shared driver's own pass over every
   adapter's refusal stays.

Amended on 2026-11-21 under the same ruling, and still accepted:

6. **A classified provider refusal is named by its class.** A refusal
   the shared classifier reads is named in fixed words for its class.
   Today there are two classes: a rate limit, and any other refusal,
   an API error or an authentication failure included. The words carry
   none of the provider's text and no other value from the row. The
   HTTP status and the retry-after stay as the refusal's
   fields. The driver's own words pass no clipping step, so an id of 80
   characters is named whole.
7. **A fallback notice wins every stop.** Where the harness's fallback
   notice is on the stderr captured by then, every stop refuses as
   substituted, in the reported-on-stderr words. That includes a second
   identity row and a line no measured stream carries. Otherwise each
   stop keeps its own words.
8. **A stop after work or delivery is lost (decision 0006).** Delivery
   is read from the harness's real exit: it exited 0 on its own and its
   result file exists. After a turn, or after delivery, a stop or an
   identity refusal ends the attempt with no result of its own. The
   driver prints its words for the operator and sends no wire result, so
   no provider chain advances past delivered work.

The order of masking and folding, for every adapter, is tracked as #1242.

## Addendum — 2026-11-21, operator ruled: seat-record v8 opens the transcript kind and the root session kind

The operator ruled on
[#1227](https://github.com/feedback-loop-ai/brokkr/issues/1227) that day:
"v8 in its own story, and yes open root_session.kind". Seat-record v8
opens the transcript kind as ruling 2 ordered, and the same version
opens `root_session.kind` on the same rule, amending the closed `kind`
of decision 0056 ruling 3 and nothing else of that object.

Each kind is one lowercase word of letters, digits and single inner
hyphens, at most 40 characters, declared by the harness's adapter in its
optional `record_kinds` member. The built-in drivers declare the kinds
they write. `none` stays a transcript's empty locator and is never a
root session kind. The engine refuses at append, wherever a record
names a kind, one the seat's own harness does not declare — its
adapter's, or the kind of the built-in driver its own command runs —
naming the kind and the seat; no other adapter's declaration counts,
and an adapter adds kinds and a resume, never a driver. A seat answers
to its selected candidate's adapter, or, inline, to the adapter the
compile pins for its model-bearing built-in driver; an adapter that
declares no kinds grants only its driver's and `none`. A site that
answers to no adapter, an inline raw command, an inline exec site or a
site no plan covers, keeps v7's three kinds and `none`: no adapter
exists to hold a declaration for it, and v8 neither opens nor narrows
what it may write. A root of a kind no built-in driver writes is
offered for a rejoin only where the seat's adapter declares it and the
one shape its `record_kinds.resume_shape` names is `supported` and
covers the site: class `work`, the site's boundary among its
`boundaries`, and its `hands` the site's, the words the engine writes
into the driver's input, so the engine offers no wider than the
driver's resume gate, asked about that shape, rejoins (decision 0056
ruling 5: an unmeasured shape is `unsupported-resume` there too). A
harness that declares its kinds is never offered decision 0030's flat
session id. An adapter declaring a root session kind must declare
`resume` and name in `resume_shape` one of its shapes, and declares no
`resume_shape` without a root session kind. Wherever a record names a
kind, anything there but a declared kind is refused, on every road,
whether or not a schema judges it after, an unresolved cleanup's
evidence and a panel member's result included, and the refusal never
echoes a value outside the grammar. A record refused for its kind is a
failed attempt and is never bound as a word.

The rule this protects is one: nothing acts on a kind that the harness
which wrote it did not declare. It is enforced from both sides:

1. What a driver hands the engine is judged before any of it is
   appended: a checkpoint, a result, a panel member's result, an
   unresolved cleanup's received outcome. What it refuses is appended
   nowhere, in no field. Decision 0109's refusals keep their precedence:
   a checkpoint that claims a record only the engine authors, or a work
   site's result that reports findings, is refused by 0109 in 0109's
   words before its kinds are judged, and the kind check runs before
   anything else is appended.
2. What the engine reads back from its own journal is already journaled.
   Copying it into a later event (decide's `raw_result`, a poisoned
   aggregate's `notes.members`, a refused carry's evidence) adds no
   record and no kind place: the value stands in the journal either way,
   and refusing the copy would remove nothing. Such a copy is evidence,
   and no reader acts on a kind inside evidence. A partial panel retry's
   carried member is more than a copy, since the carry makes it a
   contribution to a new verdict: a carried result whose kind place
   names an undeclared or out-of-grammar kind of the seat that wrote it
   is not carried, and one that is not an object has no kind place and
   carries for the join to poison.
3. A reader acts on a kind only at a seat-record place, the three v8
   defines (a checkpoint's `transcript` and `root_session`, and a
   result's `transcript`, standing alone or nested in an engine-authored
   checkpoint), and only on a kind that was judged. A verifier judges a
   record by the schema alone, so an exported journal verifies without
   the adapters that wrote it; acting on a kind is a reader's, not a
   verifier's.
   A reader that takes a kind from an earlier event to act on it, where
   an earlier engine may have written that event, judges the kind itself
   before acting, as the resume offer does. The resume offer forwards a
   transcript's locator and home only where a write of that row at that
   seat would pass the writer's kind check, a built-in kind included,
   which changes no offer a real journal can produce, since no built-in
   driver writes another driver's transcript kind and a site with no
   adapter admits all three. A reader meeting a kind it
   does not know reads the record as holding no transcript or no root of
   a kind it can use: it never guesses, never falls back to another
   kind, and never fails the read of the rest.

v8 is minted under this decision, which is accepted,
through decision 0082's lifecycle, and stands at v7's engine line, so
dispatch never chooses v7.

**Enforcement binding:** `contracts/seat-record.v8.schema.json` and its
embedded copy; `seat_record::v8_tests` in brokkr-store; the writer's
check in `crates/brokkr-runtime/src/agents/kinds.rs` and
`engine::kinds_tests`, whose fixture harness no code names seats,
records, verifies, exports and renders, and whose undeclared kind is
refused at append.

## Addendum — 2026-11-24, Antigravity CLI level 2 kept transcripts

Addendum status: proposed

Keeping a turn's transcript file in the operator's state directory after an
Antigravity CLI seat ends is a new effect on disk (`#1227`, decision 0075
level 2).

- **What is kept and where.** When a seat's loaded adapter declares
  `record_kinds.transcript: "agy-conversation"` (carried from the engine in
  `resume_context.declared_transcript_kind` through one shared type in
  `brokkr-protocol`, `DeclaredTranscriptKind`, using the one declared-kind
  grammar in `brokkr-core` shared with the adapter loader and runtime and
  refusing `"none"`, over-long words and invalid spellings; an unreadable or
  refused value refuses the launch naming
  `resume_context.declared_transcript_kind` rather than recording `none`),
  `brokkr driver agy` copies one file —
  `<engine HOME>/.gemini/antigravity-cli/brain/<id>/.system_generated/logs/transcript.jsonl`
  — out of the engine's temporary HOME after `agy` exits and before the engine
  HOME is removed, into
  `<operator HOME>/.local/state/brokkr/agy-conversations/<id>/transcript.jsonl`
  (beside `~/.local/state/brokkr/dsh-homes/`). The conversation database,
  `transcript_full.jsonl`, `chunks/` and `steps/` are not kept.
- **Modes, bounds and untrusted-source checks.** `<id>` must be a single
  exact 36-byte lowercase UUID (`8-4-4-4-12`, one shared helper in
  `brokkr-core`) read beside the fold without changing the fold's `init` type
  (absent, non-string, repeated, or non-UUID values yield `InvalidId` and
  `none` while leaving model confirmation and the turn's verdict unchanged).
  Before publishing, the driver validates the reference it will record against
  the same home and path rule the run view uses (`is_valid_transcript_home`,
  `is_native_transcript_home`, and `is_clean_transcript_path` in
  `brokkr-core`), so a relative operator `HOME`, a non-native drive or UNC
  spelling, or one carrying a control character such as a newline keeps
  nothing with a typed notice (`KeepNotice::InvalidReference`). Every component under
  the engine HOME is opened with `O_NOFOLLOW`, the leaf must be a regular
  file, and the copy is bounded at the shared 32 MiB source cap
  (`TRANSCRIPT_SOURCE_CAP`). Ancestors above `agy-conversations`
  (`~/.local/state/brokkr` and up) are traversed and, if missing, created with
  `create_dir_all`, asking nothing of an existing ancestor that the operating
  system does not ask to traverse it and create an entry in it (an existing
  ancestor of mode 0300 succeeds). No mode is changed by pathname:
  `agy-conversations` and each conversation directory that the keeper creates
  is created with `mkdirat` at mode 0700, opened with `O_NOFOLLOW`, verified by
  `(st_dev, st_ino)` against the `statat` taken right after `mkdirat`, and set
  to mode 0700 through that verified handle with `fchmod`, while an existing
  directory's mode is never changed. When the umask makes a directory the
  keeper has just created impossible for its owner to open, no transcript is
  kept and that directory is removed through `unlink_if_same` so nothing of the
  attempt remains (a umask that leaves a newly created directory openable, such
  as 0100 leaving mode 0600, is opened, set to 0700 through the verified
  handle, and keeps the transcript); a replaced entry is left untouched
  (`KeepNotice::ReplacedDirectory`) with nothing kept. The `agy-conversations` parent is checked to be a real directory
  (never a symlink) owned by the effective UID with no group or other access
  (`mode & 0o077 == 0`); if an existing owner-only parent is not writable by
  the owner (for example mode 0500), creating the conversation directory fails
  with a typed keep notice (`KeepNotice::Io`, recording `none`) and leaves the
  turn unchanged. The kept file is set to mode 0600 through `fchmod`, tightening agy's
  measured 0644 mode.
- **Only copy and kept-file rule.** The kept `transcript.jsonl` is the only
  copy that survives removal of the engine HOME. The driver and the engine are
  two processes, so the driver can only keep nothing in the cases it can see.
  Those are: a probe (unchanged from main: nothing kept, no notice, answer
  untouched); a refusal to start (judged by the one pure `refusal_to_start`
  predicate over a single observation of `result_path` made in `agy::concluded`
  immediately after `agy` exits and passed along on `Invocation` to
  `run_seat_with`); no `init` with a conversation; an id that is not a UUID; a
  source that is not a regular file reached without a symbolic link; a file
  over the bound; a reference the reader's rule rejects; a row the turn's
  masking would change (checked by building the exact `Emitted::transcript`
  row and passing it through the shared path's `probe::seat_row`, keeping
  nothing with a typed notice naming no value if any byte differs from the
  unmasked row); and an undeclared kind. What the driver cannot see is the
  engine refusing the row after the driver has returned. Then a kept file with
  no record can remain; it is readable only by the operator, and the reaper of
  issue `#1323` removes kept files that no journal names.
- **Same-user directory limit.** The stated limit is the class of processes
  running as the operator's own user that create, rename, move, replace or
  remove entries under the operator's `agy-conversations` directory while a
  keep is in progress (operator ruling of 2026-11-25 on `#1227`). The keeper
  runs as that same user: everything such a process can make the keeper do —
  write a file into a directory it controls, set a directory it owns to mode
  0700, remove an entry it owns — that process can already do itself with the
  same system calls, crossing no privilege boundary and reaching nothing of
  another user's. Against that class the keeper promises no outcome (for
  example, not where the transcript ends up if that process moves the
  directory, not which entry is removed if that process swaps one in between
  `statat` and `unlinkat`, and not what the record's home names if that
  process renames a parent). What the code holds are four testable mechanisms:
  (m1) every open for the kept place is an `openat` relative to a held handle
  with `O_NOFOLLOW` on the final component, so it never traverses a symbolic
  link placed at a name it opens; (m2) no system call changes a mode by
  pathname, only `fchmod` on a handle; (m3) the transcript file is created
  with `O_CREAT | O_EXCL | O_NOFOLLOW` relative to the conversation
  directory's handle, so an existing entry at that name is never opened or
  truncated; (m4) every `unlinkat` is preceded by a `statat` through the same
  parent handle whose `(st_dev, st_ino)` equals the recorded identity, and is
  skipped when they differ. When no such process acts on the tree during a
  keep, the kept place, modes, refusals and rollback guarantees above hold.
- **Reaping.** Nothing reaps `~/.local/state/brokkr/agy-conversations/` in
  this story, as nothing reaps `~/.local/state/brokkr/dsh-homes/`; a shared
  reaper is tracked as `#1323`.
- **At-rest secrecy and masking.** As measured in
  `l2b-printed-value.transcript.ndjson`, a bound secret printed by a tool lands
  in `transcript.jsonl` in clear. Like Claude Code's session files, the kept
  file at rest is **not** masked; only the operator's UID can read it (private
  owner-only parent, 0700 conversation directory, 0600 file). Every run-view
  surface (`brokkr runs transcript`, the TUI and the browser) reads the kept
  file through the shared `read_local` and `mask_secrets` path against
  `secrets.env` beside the journal; `mask_secrets` visits `Turn.ts` as well as
  every block's text, for every transcript kind. The agy reader never re-parses
  and never re-serializes what a row holds as a string: each member of
  `tool_calls[].args` is rendered as `name=value` with string values verbatim
  (already JSON-encoded once by agy, matching claude and codex) and non-string
  values as compact JSON, so `mask_secrets` covers every raw or once-escaped
  bound value including bound JSON text. `created_at` admits only exact
  20-byte `YYYY-MM-DDTHH:MM:SSZ` timestamps; any other present value
  (including JSON `null`) becomes an empty timestamp and is counted in a
  notice while keeping the row shown, while only an absent member is not
  counted.
- **Follow-ups.** Rejoin (`--conversation`, serialized per conversation) is the
  next story; applied effort from per-row model slugs is `#1324`; the shared
  reaper is `#1323`.

## Addendum — 2026-11-26, Antigravity CLI level 2 kept rejoin state and root session

Addendum status: proposed

Keeping the conversation files a later Antigravity CLI turn needs to rejoin
and recording the turn's `root_session` extend the 2026-11-24 kept-transcript
addendum (`#1227`, decision 0075 level 2, slice 1 of 2; launch is slice 2).

- **One kept place, one rule.** After a work turn whose loaded adapter declares
  `record_kinds.root_session: "agy-conversation"` (carried from the engine for
  work seats alone in `resume_context.declared_root_session_kind` through the
  shared `DeclaredTranscriptKind` carrier and validated under the same
  `brokkr-core` kind grammar), `brokkr driver agy` keeps the two conversation
  parts measured as sufficient for a rejoin in a fresh HOME
  (`l2-07-rejoin-moved-files`, `l2-10-box-cold`, `l2-11-box-rejoin`,
  `l2b-01-iso-cold` through `l2b-03-iso-rejoin-same-home-empty-set` on
  Antigravity CLI 1.5.14):
  `<engine HOME>/.gemini/antigravity-cli/conversations/<id>.db` as
  `<operator HOME>/.local/state/brokkr/agy-conversations/<id>/rejoin/conversation.db`
  and `<engine HOME>/.gemini/antigravity-cli/brain/<id>/` as
  `<operator HOME>/.local/state/brokkr/agy-conversations/<id>/rejoin/brain/`.
  The engine HOME is still removed when the turn ends. The keeper's existing
  guarantees and four testable mechanisms (m1)–(m4) from the 2026-11-24
  addendum apply unchanged by reference, with no second kept place and no
  second set of guarantees.
- **Same-user limit on both sides (operator rulings 2026-11-25 and
  2026-11-26).** Ruling "A" of 2026-11-25 holds for the source engine HOME as
  it does for the kept tree: against a process of the operator's own user that
  changes the engine's HOME for the turn, or the kept tree, during a keep, the
  keeper promises no outcome. No privilege boundary is crossed: the engine HOME
  is a mode `0700` directory the driver made, owned by the operator; the turn's
  `agy` process has exited when the keep begins; and a boxed seat's tools ran
  in the box and cannot reach it. Examples of that stated limit on the source
  side include a source directory detached by a replaced parent before the
  final check, a database or subtree changed after its own final check and
  before publication, and a same-length in-place rewrite that restores the
  recorded modification time (changing bytes while leaving every compared stat
  field equal), as well as any change undone before the final check. The
  keeper's promises are against accidents: a turn still writing when it was
  killed, a partial tree, a symbolic link, something that is not a regular file
  or a directory, a tree over its bounds, a bound value in a file or a name, or
  a turn whose model pin was not confirmed. Its mechanisms (no following of
  links, reading from the handle that was classified, the final check at the
  end of the walk, name screening, pin confirmation, bounded descriptors, and
  (m1)–(m4) on the kept side) make accidents visible; they are not claimed to
  defeat a same-user process or to prove the kept state unchanged.
- **Operator-only modes.** Every directory created under `rejoin/` is mode
  `0700` and every file is mode `0600` through `fchmod`, tightening the `0644`
  mode agy writes on brain files.
- **Bound values exclude rejoin.** As measured in `l2b-09-printed-value-allowed`,
  values printed by tools land in clear in both `conversations/<id>.db` and
  `brain/<id>/`. Because the SQLite database cannot be masked and still be read
  by `agy`, a conversation in which any bound secret value appears (checked
  across every copied byte and every file and directory name under `brain/<id>/`
  using the shared projected masker encodings across chunk boundaries, and on
  the emitted `root_session` row through `probe::seat_row`) refuses rejoin
  persistence (`persistent: true` is never recorded) and rollback attempts to
  remove what it staged while the kept transcript's own rule stays unaffected;
  an unlink I/O failure during removal can leave a partial `rejoin/` directory
  on disk even as persistence is refused.
- **Bounded, with attempted rollback.** Rejoin keeping is bounded at 128 MiB total
  bytes (4× `TRANSCRIPT_SOURCE_CAP`, fitting `transcript.jsonl`,
  `transcript_full.jsonl`, `chunks/`, `steps/`, and `conversations/<id>.db`
  while capping per-turn disk growth well above the 384–672 KiB database and
  3–48 KiB brain sizes measured across 113 short lab turns), 4,096 total
  entries (charging `conversation.db`, the `brain/<id>/` root, and each brain
  entry as it is met during traversal), and 16 directory levels under
  `brain/<id>/` (4× the measured lab depth of 4). During the copy walk, every
  source path component is opened once from its parent handle with `O_NOFOLLOW`
  and classified by `fstat` on that handle rather than by name; only
  directories and regular files are copied, each regular file is read from the
  handle that was classified, and after the last entry is copied (before
  publishing) one final check re-stats the database and reopens the brain root
  and its subdirectories to compare every directory's membership and every
  entry's type, size, `(st_dev, st_ino)`, and modification time to the depth
  walked. Over any bound, or if the brain is
  empty or any part is missing, unreadable, not a regular file, or fails that
  check, persistence is refused (`persistent: true` is never recorded) and
  rollback attempts to remove every created rejoin file and directory in
  reverse order through `unlink_if_same`, leaving any kept `transcript.jsonl`
  untouched. Because `unlink_if_same` ignores unlink errors, an I/O failure
  during removal can leave a partial `rejoin/` directory on disk even as
  persistence is refused (as can a same-user process under the stated limit
  above).
- **Root session and `persistent`.** For a declaring work turn that did not
  refuse to start, whose `init` row confirmed the seat's pinned model, and that
  named a lowercase UUID conversation id, the driver asks
  the launched binary for its version via `agy --version` under the engine HOME
  and `AGY_CLI_DISABLE_AUTO_UPDATE=true` (decision 0124), parsing stdout with
  the same version-token rule the other built-in drivers share. When a
  recordable version is observed and masking does not rewrite the row, the
  driver publishes a `harness-started` checkpoint with `harness: "agy"`,
  `launch: "cold"`, and `root_session: {"kind": "agy-conversation", "id":
  "<uuid>", "harness_version": "<observed>", "persistent": <bool>}`, where
  `persistent` is `true` only when rejoin state was kept whole (carrying the
  kept `transcript` coordinates on the same row when `transcript.jsonl` was
  also kept) and `false` on every rejoin keep failure. When `agy --version`
  yields no recordable version or masking would rewrite the row, no
  `root_session` is recorded and no rejoin state is kept, matching the other
  built-in drivers' rule.
- **Unchanged non-keeping seats and still no rejoin offer.** Probes, gate
  seats, adapterless or undeclared seats, turns whose `init` row did not
  confirm the seat's pinned model, turns with no conversation id, and turns with
  a non-UUID id keep no rejoin state and emit no `root_session`,
  recording what they recorded before. `docs/examples/adapters/agy.json`
  declares `record_kinds.root_session: "agy-conversation"` and
  `resume_shape: "stream-work-site"` while keeping `stream-work-site` at
  `status: "unsupported"`, so the engine offers an agy seat no session and an
  offered session is still refused before any process starts.
- **Not built in this slice.** The `--conversation` launch, restoring kept
  state into an engine HOME, confirming the returned conversation id and
  handling an unknown id, serializing concurrent turns on one conversation, an
  MCP isolation row for rejoin, flipping `stream-work-site` to `supported`, and
  the shared reaper of `#1323` (which now also has `rejoin/` beside
  `transcript.jsonl` to reap).
- **Not measured.** Real work-seat database and brain sizes (the 113 measured
  conversations were short lab turns); macOS; and any Antigravity CLI version
  other than 1.5.14 for rejoin.
