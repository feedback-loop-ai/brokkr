# 0055 — Read every retained transcript kind

Status: proposed
Date: 2026-09-09

## Context

Decision 0032 records and retains the operator's transcript but
the local pane reads Claude alone. Issue #222 extends that local read to
Codex and DSH and adds a scriptable command. The OpenSpec change
`read-every-transcript-kind` supplies the detailed requirements and scenarios.
This proposal supplements ownership/retention/privacy and proposes to replace
only 0032 ruling 4's Claude-only command-construction enforcement binding.
Session resumption and sandbox re-imposition remain governed by 0030 and the
separate #226 work; hints execute nothing.

**Rulings.**

1. **The recorded reference is authority, independently of validity.** The
   latest common reference wins even when empty or refused; local JSON echoes
   its three strings unchanged and marks it nonlegacy. Only an absent common
   reference with eligible Claude/LaneTally/absent legacy provenance and a
   valid id can synthesize a Claude reference. Codex uses its existing
   1-128 ASCII alphanumeric/dash language with alphanumeric first character;
   Claude uses leading hexadecimal and 1-64 hexadecimal/dash characters;
   DSH uses a validated relative component path and absolute recorded home.
   The built-in 80-character recording clamp is unchanged; a reader never
   repairs a clipped locator, stitches attempts or borrows another seat.
   **Enforcement binding:** pure reference constructors, per-kind boundary
   tables, latest-reference/legacy and rejected-reference serialization tests.

2. **An owned local source is unique, safely opened and bounded.** Claude
   searches immediate project directories for the exact filename; Codex uses
   the whole filename-token predicate in its sessions tree through depth six,
   without a payload/header identity gate. DSH searches only its recorded
   root's project/session layout, admitting first-record `session` ownership
   with absent or unsigned-zero depth. Invalid depth is not coerced. A lookup
   examines at most 10,000 entries and at most 65,536 first-header bytes per
   DSH candidate. Below the canonical home, directory and leaf opens refuse
   symlinks/reparse points and nonregular sources, retain checked handles and
   report paths losslessly. Source input is at most 32 MiB plus one overflow
   probe byte. Pure view derivation has no I/O. A narrow local-reader
   production-dependency exception permits target-specific `rustix` fs and
   `windows-sys` filesystem bindings already in the lockfile; no provider SDK,
   shipped JavaScript runtime or broad filesystem framework follows. Ruling 4
   separately permits one pinned dev-only test engine for exact client proof.
   Directory I/O, or the bounded DSH opening-header I/O/UTF-8 check, can
   return discovery-stage `unreadable` when it prevents a safe unique answer;
   that refusal has no confirmed source path and closes browser presentation
   admission. An I/O/UTF-8 failure after safe unique discovery is instead a
   body-stage `unreadable` and does not retroactively erase admission.
   **Enforcement binding:** handle-based resolver, entry/header/source-bound
   tests, competing-failure/uniqueness tests, Unix/Windows ancestor/leaf race
   and nonregular-source tests, plus browser traces for both unreadable stages;
   frozen-file and dependency/license gates.

3. **Projection is a bounded audit interpretation, with explicit admission.**
   Preserve Claude's closed content/omission table. Decode Codex responses
   and recognized content-bearing events, including completed items; prefer
   a response only for proved content identity, preserving unassociated
   records without text or timestamp heuristics. This does not claim that
   id-less legacy mirrors can always be suppressed. Decode DSH numeric
   on-disk version zero only, after unique ownership and usable bounded
   UTF-8 acquisition. Missing, mistyped or foreign versions refuse as
   `unsupported-format`, with confirmed path/hint, no turns, zero counts and
   source-only truncation. Version cannot select among owned roots, a later
   header cannot repair the opening one, and unused replay-header metadata
   is not an audit admission gate. Omitted depth remains legacy zero;
   omitted version does not.

   Under admitted DSH version zero, packed rows decode into ordered logical
   events only after whole-row validation. Physical rows govern source bytes
   and diagnostics; logical events govern turns, timestamps and display
   bytes. Signed safe epoch-millisecond time is rendered as decimal; ordinary
   invalid time is absent, invalid packed storage refuses. Assembly suppresses
   only cited, unique, earlier same-turn/step chunks. Inclusive citation ranges
   match observed identities without expansion. Valid overlapping/duplicate/
   unordered citations are sets, deliberately differing from DSH's replay
   ordering validator; invalid encodings/self/future citations refuse.
   `surfaceOp` never rewrites audit history. Unknown DSH envelopes are counted
   omissions only with top-level boolean `ignorable: true`; required unknowns
   and invalid packed/citation rows refuse all prose while preserving complete
   usable-prefix counts and source-only truncation. I/O/UTF-8 failure precedes
   both format refusals and keeps zero counts.

   In every admitted projection, classify the complete bounded source and
   resolve associations before the 4,000,000-byte block-text budget. Retain
   complete turns until first overflow. Source/member order outranks time;
   partial appends and cap fragments supply no fabricated events. Missing
   partners and uncited fragments remain visible. **Enforcement binding:**
   pure projector tests for provider mappings, proved/unproved counterparts,
   DSH header/depth/version matrices, packed/plain equivalence, citation sets,
   all refusal stages, diagnostic units and both exact cap boundaries.

4. **Every local surface consumes one result.** `brokkr transcript` selects
   one run/participant and optionally a positive u64 one-based displayed turn
   after projection. Its `brokkr.transcript/v1` fields, closed reasons, stdout/
   stderr/exit rules, reference echo and metadata retention follow the command
   delta; future published structural changes require another schema version.
   It does not widen inspect/seats/watch or journal models. Shared notices
   have truncation/malformed/unrecognized ordering and exact spelling, with
   no Claude suffix. TUI snapshot replacement invalidates changed subjects,
   prior indices and overlays; active, final and manual reads recheck sources.
   Empty success and refusal remain different door states.

   The proposed replacement for 0032 ruling 4's command binding is: the shared
   local derivation constructs informational full-session lines by validated
   kind. Claude names `claude --resume <id>`; Codex names the confirmed rollout
   or explicit rollout unavailability, `codex exec resume <id>` and recorded
   home; DSH names only a confirmed session file. Rejected references have no
   hint. Path/home fragments use the reading delta's JSON-string-literal
   quoting. TUI, CLI and browser participant presentation consume the exact
   shared value; no other kind borrows a command and no display executes it.
   Browser presentation stays separate from body/journal models and performs
   selection, validation and discovery but no body projection. Its
   kind-agnostic source admission and the browser's Claude-kind/local-home
   drill eligibility remain distinct; every session label, id-only body
   request and growth watch requires both, so Codex, DSH and foreign-home
   sources never enter a Claude route. Active client work is keyed by run,
   participant and complete effective reference and guarded by a generation.
   Discovery-stage `unreadable` is its shared presentation refusal and closes
   admission; body-stage `unreadable` after a safe unique Claude discovery
   remains on the id-only route and follows the refusal floor below.
   Admission or eligibility loss closes the exact watch and clears prose
   before repaint, and stale responses cannot restore it.

   The selected participant receives a recurring presentation re-check at
   least as often as the runs poll, including after conclusion. Each interval
   permits at most one automatic watch opening, including one opened by the
   re-check. A refused body silences further body/watch work until the next
   interval; an equivalent result repairs only a missing body and then a
   missing working-seat watch. A successful body, including HTTP 200 with zero
   turns, is received until an explicit clear or refusal. Every explicit
   operator selection starts a new generation and interval even when it
   reselects the identical subject: it closes the prior owned watch, clears
   body/prose, pending work and the refusal floor, restores one opening budget
   and fetches fresh presentation. Late callbacks from the prior generation
   remain inert. Presentation/body responses bypass HTTP caching. Existing
   id-only Claude HTTP routes retain successful envelopes and the specified
   404/SSE-loss behavior; Codex/DSH body routes remain absent.

   The served page isolates this policy in one dependency-injected controller
   block. Tests extract and execute those exact `PAGE` bytes using the exact
   `boa_engine` 0.21.1 CLI dev-dependency with default features disabled,
   controlled effect adapters and drained Promise jobs. Boa is not linked into
   the release binary and adds no Node or browser service, but its lockfile,
   MSRV, license, audit and all-platform Cargo results are admission evidence.
   **Enforcement binding:** CLI selector/text/JSON tests, shared-hint/notice
   conformance, headless TUI navigation and atomic-refresh tests, existing HTTP
   endpoint and thin-adapter source tests, and Boa-executed exact served-client
   transition traces for admission/eligibility loss, stale generations,
   discovery-stage unreadability, admitted non-Claude and foreign-home
   selections, body-stage unreadability, zero-turn success, refusal floors,
   both watch opening orders, recurring recovery, no-store refetch and
   identical-subject operator reselection.

5. **Reading retains private evidence and remains inert.** Only explicit
   local transcript reads expose requested prose; journal, checkpoints,
   exports, dossiers and result telemetry retain their existing path/id-only
   boundaries. No persistent body cache, provider process, repair, media fetch,
   deletion or transcript mutation occurs. Terminal output uses `Safe`, JSON
   preserves escaped strings and browser output uses text nodes.
   **Enforcement binding:** test-owned homes and synthetic content, retained
   byte/existence and journal count/hash comparisons, sentinel privacy tests,
   provider-launch fail sentinels and the unchanged exact coverage gate.

## Consequences

This is an additive local document and reader, with no
stored-data migration or frozen contract/schema/corpus edit. It deliberately
breaks Claude's leading-hyphen id acceptance, first-match/unbounded/symlink
lookup and displayed-text-only input limit. The operator may inspect the
original independently, resolve duplicate placement, fit the recorded scope
within discovery bounds or use actual owned entries instead of below-home
symlinks. The reader offers no home override and reorganizes nothing.
DSH's invalid-depth refusal is deliberately stricter than the shipped
adapter; its version/citation policy is an audit policy, not execution replay.
Unproved Codex associations can remain visible twice, preserving evidence.
Exact browser-controller proof adds the pinned default-feature-free
`boa_engine` 0.21.1 development dependency and its lockfile graph; it does not
enter the released binary or permit Node, a browser service or an additional
production language. Only the operator accepts this proposal. Filing it does
not certify dependency admission, live resumption, implementation tests or
controller host/remote gates.
