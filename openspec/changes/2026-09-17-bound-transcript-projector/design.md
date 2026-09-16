## Context

Status: proposed. Change: `2026-09-17-bound-transcript-projector`.

Adopt the proposal and three capability deltas committed at `b82d2479`, on
production base `5ef4a842`; see [proposal.md](proposal.md) for motivation.
This visit authors only this design. There is no `returned_from` finding in
the supplied context. The council inputs are the complete run-local
`.forge/design/positions/simplicity.md` and
`.forge/design/positions/robustness.md`; their claims are reconciled below.
They are evidence, not additional artifacts to commit.

The current `project_dsh` first builds `Vec<DshEvent>`, resolves citations
and dedicated tools, builds turns, then calls `display_cap`.
`dsh_packed` constructs a payload event for each nonempty member.
`display_cap` counts only block text. Those collection lifetimes, not the
TUI's refresh implementation, are the allocation defect. The existing
source snapshot is already bounded and resident; it can be parsed again
without reopening a file or admitting newer bytes.

Two ordering facts constrain the repair. A later readable assembly can
suppress an earlier chunk; a later dedicated tool event can remove an
embedded block from an earlier message. The first candidate's final cost
therefore cannot generally be decided at first encounter. The current
citation sweep also tests assembly readability *before* tool association;
that order must survive the refactor, including when association eventually
empties the assembly.

Grounding includes README, decisions 0004, 0005, 0009, 0055 and 0061; the
living transcript specs and their adopted deltas; `transcript.rs`, its
unit tests, `ui.rs::dsh_header`, CLI/TUI transcript tests; and the history
of `b809e02` and `5aee618`. Line numbers in the commission predate this tree.
No frozen material or scope-fenced implementation has been changed.

Validation on this visit: the adopted change passes strict OpenSpec
validation, and `openspec validate --all --strict --no-interactive` reports
15 passed, zero failed (with informational notices for other changes and
long requirements). The design section/decision inventory also passes a
local structural check. Formatting, clippy, workspace tests and both bundle
compile commands were attempted; each could not start because `cargo` is
absent from this boxed seat. There are no new Rust, coverage, allocation or
removal-proof results. Host/remote gates remain pending, not inferred from
the green production base.

## Goals / Non-Goals

**Goals:** make DSH payload collection obey the adopted display budget
throughout projection; coalesce before per-token events exist; keep complete
snapshot classification and association; share the charging rule across
all providers; and make each protection independently falsifiable by a
removal test. The two accepted lows remain separate, small repairs.

**Non-Goals:** a new reader API, document schema, dependency, production
module, source format, persistent cache, cross-row merge, TUI polling change,
or provider/journal behavior change. This slice does not promise that RSS,
JSON trees, arbitrary block arrays, or all providers' intermediate storage
fit inside four million bytes. These residuals must be measured and named.

The code design is complete, but D0 identifies an upstream scope conflict
that must be resolved before its implementation can satisfy the commission.
It is not a deferrable implementation choice.

## Decisions

### D0. Adopt the change; return the missing dependent-test scope upstream

The operator's merge ruling and the adopted reading/command deltas agree.
The proposal's Impact section, however, confines implementation to
`crates/brokkr-view/` and the named `ui.rs` header repair, without admitting
an indispensable dependent CLI test edit outside that fence.

Concrete counterexample: the existing
`crates/brokkr-cli/tests/transcript_command.rs::packed_dsh_members_have_separate_selectable_turns`
(at line 1093 in this tree) supplies an uncited packed `["a","b","c"]` row
followed by a user message. It asserts four turns, selects `b` at index two
with stamp `999`, selects the user at index four, and equates packed and
ordinary index two. Under the commissioned rule there are two turns:
`abc` stamped `1000`, then the user. The test must change for
`cargo test --workspace` to pass. No change confined to the permitted
projector and header code can satisfy both assertions honestly.

The upstream correction is to the proposal's scope/dependency inventory,
with the commission's fence explicitly reconciled: admit test-only changes
in `crates/brokkr-cli/tests/transcript_command.rs` for the new projection and
selection behavior. Also name the directly dependent surface proof in
`crates/brokkr-cli/tests/transcript_surfaces.rs` and
`crates/brokkr-cli/src/tui/tests.rs`, plus `ui/tests.rs` for the authorized
header repair. This grants no additional CLI/TUI production changes and
no exception to the explicitly forbidden files. The controller/operator
owns any necessary clarification of that fence; this design does not
silently grant it.

The TUI test `six_packed_members_are_readable_through_both_doors` currently
supplies `read_of(turns_of(6), false)` directly. It does not parse a packed
row, so it is not an independent proof of either old or new packed
semantics. Preserve its generic navigation coverage and add or adapt the
real projected-source surface case after the test scope is reconciled.

Rejected alternatives: retain old CLI indices only in tests; make CLI undo
coalescing; skip or weaken the contradictory integration test; or declare
workspace success based only on the view crate. Each defeats either the
one-projection contract or the required verification. No earlier artifact
is rewritten by this design-only commit. The phase result is `upstream`
for this concrete dependency fault, with the design below retained for
adoption on return.

### D1. Collect complete-prefix facts, then project a bounded prefix

Adopt robustness R1/R4 and simplicity S4's requirement to decide final
costs after association. Choose two parses of the same admitted bytes over
a retained `TurnRef` for every prospective turn. A reference containing
role, timestamp, block lengths and packed member boundaries still scales
with all candidate content and risks becoming the disguised event list the
added retention requirement forbids. Re-parsing trades CPU for fewer live
payloads and a smaller lifetime argument.

1. Preserve `first_physical_row`/`dsh_header` admission before parsing later
   rows. Header refusal retains zero row counts and only source truncation.
2. **Fact pass:** visit every complete physical row using the existing
   version-first classification and recorded-token rules. Count malformed
   and unrecognized rows once, validate all packed members and citations,
   and accumulate refusal state. Retain identities, eligible citation
   intervals and dedicated-tool keys as D2 describes. An ordinary decoder
   may construct one temporary event to obtain its facts, then releases it;
   it never appends payloads to a complete-prefix event vector. Packed
   validation produces a borrowed row description, never member events.
3. Any event/storage refusal returns `unsupported-format`, no turns,
   complete-prefix counts and only the already established source truncation. No display truncation has
   been established on this path. Otherwise finalize the fact indexes.
4. **Projection pass:** parse the identical snapshot, resolve each ordinary
   candidate against the final facts, and visit packed surviving runs one
   at a time. Retain only final content-bearing candidates that fit D4.
   At first overflow, seal the displayed prefix permanently. Later content
   is not admitted to spare space. This pass need not collect diagnostics
   again or finish projecting the suffix: the fact pass already did so.
5. Move the bounded retained events into `Turn`s, dropping empty events
   defensively as the current final conversion does. No text clone is
   needed. The existing final cap can remain as a bounded, idempotent
   application of the same rule; it is no longer the DSH retention bound.

Keep classification/packed validation shared between the passes; do not
maintain two provider disposition tables. Source/member order determines
output; neither hash order nor timestamp order can decide it. Do not reopen
or reread the source from disk between passes.

Rejected: cap during the fact scan, associate only the retained prefix,
collect all events then coalesce, or suppress a provisional concatenated
string after losing its member geometry. In particular, an oversized
fallback that a later assembly wholly suppresses must never cause display
truncation or exclude that assembly.

### D2. Identity facts describe source geometry, not projected payloads

Combine simplicity S1's preservation of member identity with robustness
R3/R5's avoidance of per-token maps on the ordinary streaming case.

For a validated packed row, observed sequences form one inclusive span
`seq0..seq0 + member_count - 1`. Store that span, not `Vec<i64>` plus a hash
entry per member. Ordinary quiet, omitted, unknown and blockless rows
contribute their existing observed singleton identities. Invalid packed
rows contribute no partial facts. Preserve exactly the current observation
rules, including identity from valid quiet tool-argument members and empty
text members.

After the fact pass, sequence multiplicity can be represented by sorted
interval endpoints and disjoint spans. Keep the exact active count during
the endpoint sweep; only the resulting query spans collapse counts to zero,
one, or multiple, so ending one of several overlaps cannot invent uniqueness.
Use endpoint arithmetic wide enough for the accepted
sequence domain, including ordinary signed identities; never increment an
`i64::MAX` endpoint in `i64`. This representation must count overlaps
between packed spans and ordinary singletons without expanding any span.
Only uniqueness is queried; a duplicate anywhere in the bounded snapshot
keeps that sequence ambiguous, including a duplicate later than an assembly.

For suppression, retain each eligible assembly's `(turn, step)`, physical
row ordinal and cited intervals. Eligibility means readable non-omitted
blocks before dedicated-tool association, matching the existing sweep.
Normalize overlapping intervals with their greatest citing source ordinal
(or an equivalent interval index). A readable candidate is suppressed only
if its sequence is globally unique and the matching turn/step interval has
a citing row strictly later than the candidate. This replaces the current
per-event `chunk_index` without needing a stored payload or record for
every packed member. Out-of-order sequences must not substitute for physical
source order. An interval lookup must not scan or allocate across a numeric
gap, and repeated overlapping citations must not become a quadratic
per-member scan.

If there is no eligible citing assembly, omit the multiplicity/suppression
indexes and release collected observation spans. Under version three no
admitted chunk exists, so do not build an unused suppression index.
Validation of citations still runs where the version's row rules require
it. A late citation is handled because facts are collected before this
consumer decision, not because an early scan guessed there would be none.
This answers robustness R5 without introducing a speculative early exit or
an additional discovery parse.

For dedicated tools, keep the existing `(id, direction, turn, step)`
uniqueness rule: count a key once per dedicated event, preserve collisions,
and never deduplicate dedicated events against each other. Collect the keys
from the whole prefix, even beyond the eventual display stop. Association
runs under version zero and under version three only when `isSeeded` is
exactly false. A version-three user message still supplies no positions.
Empty or omitted-only messages cannot become suppressing assemblies merely
because metadata identifies them.

The fact collections scale with actual source rows, identity bytes and
citation endpoints. They contain no copied text, role, stamp, block vector,
or per-member run descriptor. Their memory is a separate measured cost,
not charged display content. Small private types in `transcript.rs` are
permitted; simplicity's proposed limit of exactly one new type is rejected
as an implementation count rather than a correctness property.

### D3. Coalesce in packed projection, after resolving member suppression

Use one validated borrowed description of the current packed row: kind,
positions, initial sequence/time and references to its decoded text/gap
arrays. Finish the current exact envelope, member-type, sequence and
cumulative-time checks before that row contributes any fact or payload.
Do not retain an additional per-member gaps or boundary vector.

In the projection pass, walk members in stored order, using D2 to test each
readable member. Track a run's first visible member, byte length and stamp;
concatenate its text only as that run's one candidate, preferably measuring
length before allocating the final string. A suppressed readable member
ends the run. Empty members affect identities and time reconstruction but
neither start nor split a run. The emitted candidate goes immediately to
the bounded collector; do not return a vector of every run in the row.
Whole-row validation has already happened even if the first run is too big.

The normative rule remains solely in the reading delta's "DSH sessions
expose assembled or provisional content once" requirement. This algorithm
implements it; it adds no cross-row or completed-message inference.
`tool-call-chunks` remains quiet. No per-token `DshEvent`, `Block`, role
string or timestamp string is allocated in either pass.

Reject simplicity S1's pre-concatenation followed by splitting a retained
run after suppression: it holds text that is not yet eligible for the
budget and may copy it again. Adopt its local packed allocation-site repair,
row fence and unchanged public types instead.

### D4. One named charge for all returned projections; bounded DSH collection

Define `DISPLAY_EVENT_COST = 512` beside `DISPLAY_CAP = 4_000_000`, and
replace their text-only documentation with **display accounting bytes**.
The operator selected both merge and a constant. The value 512 rounds above
the issue's approximately 330-byte event audit; that audit is not our
measurement and the constant is not an allocator layout guarantee.

One shared cost helper consumes final emitted block text lengths: an empty
block list costs zero and supplies no turn; any nonempty list costs 512
plus the sum of UTF-8 bytes. Use checked accumulation and comparison before
subtraction so overflow refuses admission. No per-block or per-member
structural charge is added. Test empty-text blocks, omitted-only turns,
multiple blocks and all roles as well as ordinary prose.

`display_cap` uses this helper for Claude, Codex and DSH. DSH also uses it
at the collection boundary after suppression/tool association, before
pushing an event into the accumulated prefix. Equality fits. One current
candidate may exceed the budget and must be discarded whole. The retained
prefix has charged cost at most 4,000,000 and at most 7,812 content-bearing
turns; the candidate is additional, not another accumulated prefix.
Keep the charge after an association that removes a final block, so an
emptied event spends nothing. Retained DSH events move into final turns;
conversion never duplicates their owned strings.

Keep a distinct blockless retention guard after fact observation and
association. The final empty-event filter remains the defensive
conversion rule. Removing the collection guard must expose retained empty
slots to the observation test even though the final filter still produces
the same visible turns. This implements shape (c) independently of the
semantic changes. Merely observing no final empty turns is not its proof.

Adopt simplicity S2/S3 and robustness R2's single charging formula. Reject
lowering `DISPLAY_CAP`, scattered literal `+ 512` expressions, skipping an
overflow to keep a smaller successor, and changing TUI clone/poll behavior
as a substitute for bounding its input.

### D5. State the residuals instead of extending the commission implicitly

Resolve the robustness seat's four questions as follows:

| Question | Decision and reason |
|---|---|
| Does intermediate retention extend to Claude/Codex? | No new complete-prefix retention algorithm for them in this slice. All returned turns pay D4; the added intermediate-retention requirement explicitly names DSH. Their existing pre-cap block-bearing records/turns remain a source-sized residual and need measurement. This adopts simplicity's scope restraint and answers robustness F10 explicitly. |
| Is packed JSON deserialization replaced? | No. Retain one `serde_json::Value` per physical row and release it before parsing the next, in each pass. Its member strings and array storage can substantially exceed encoded bytes. Measure a single-large-row case; borrowed/streaming JSON deserialization is a separate future change, with escaping, duplicate-key and token-exactness consequences. |
| Is the charge extended per block? | No. The adopted formula charges once per turn; a turn with many empty blocks, or long metadata strings, is not an exact memory bound. Record block counts, retained heap/RSS proxy and metadata costs separately. Any new term needs a proposed decision and spec revision. |
| Is the second parse accepted? | Yes. It preserves complete-prefix semantics without retaining content descriptors for the full source. Record elapsed time beside memory so repeated TUI reads' CPU cost is visible. |

These answer robustness F6-F10, not declarations that residual memory is
small or safe because input is finite. The source cap bounds the amount
of input decoded, while parser/container/metadata amplification remains
measured separately. Neither a four-MB RSS claim nor a whole-console
allocation ceiling is authorized by this accounting formula.

No new production file, dependency, public iterator API, serialized field,
per-block term, or cross-row merge is needed. This adopts simplicity's
cuts 1-10 except its rejection of any shared budget logic: a small shared
cost helper and an incremental DSH collector are necessary to enforce the
same rule twice without drift; a public budget abstraction is unnecessary.

### D6. Observe Codex sharing while both association paths hold keys

Keep `CodexId = Rc<str>` and the existing before/after block-sharing checks.
Add a test-only observation seam at both key-construction sites inside
`associate_codex`: construction for the `seen`/canonical/fallback maps,
and the lookup during fallback retention. The observer borrows the source
id and the actual key; it must not clone an `Rc`, allocate an id, or change
the strong count being asserted.

Assert pointer equality between the actual live key and the block's id,
and the expected strong counts at that precise lifetime, accounting for
keys still in `seen` and the maps. Include both canonical and fallback
records; the existing canonical-only test cannot exercise the second key
site. After map teardown the current before/after invariants still hold.

For each key site independently, replace `Rc::clone(id)` with a fresh
`Rc::<str>::from(id.as_ref())`, retaining row-level sharing and compiling
successfully. The *during-pass* assertion must fail at that site; restore
and rerun it. This is the isolated byte-copying behavior of `5aee618`, not a
whole-commit revert that also breaks block construction. Retain a separate
control that the existing block-sharing assertion catches per-block copies.
This combines simplicity S5 with robustness F12.

### D7. Enforce the header's byte length, including true EOF

Keep `read_bounded(DSH_HEADER_CAP + 1)` so a delimiter just after an
exact-limit header is visible. Measure the slice before the first newline,
or the entire buffer if no newline exists, against `DSH_HEADER_CAP`.
A slice longer than the cap returns `HeaderCheck::TooLarge`; an overflow
with no delimiter also refuses. Do not use the read's overflow flag as the
only header-length predicate. Update the comment to state those units.

Use otherwise valid headers at 65,536 and 65,537 UTF-8 bytes, each with and
without newline, under both admitted versions. Check the public result is
exactly `DiscoveryLimit`/`discovery-limit` for refusal, with the discovery
stage's null path/hint and zero counts, and successful ownership at equality.
The newline is never an admitted header byte. Reintroduce the EOF bug and
observe the cap-plus-one refusal assertion fail for the intended reason;
restore and pass. This adopts both seats' header diagnosis without changing
ownership, version admission or `safe_fs`.

### D8. Separate deterministic bound proofs from memory measurements

Adopt both seats' demand for independent removal proofs and robustness R7's
logical lifetime observation. Do not call a logical counter an allocator.
Use test-only thread-local observation at actual event construction,
collection and release/transfer boundaries. Record payload constructions,
live/peak retained event slots (including blockless slots), retained text
bytes, charged bytes and the current candidate separately. Observe all
payload collections, including temporary packed expansion; a counter only
on successful budget admission would miss the defect it claims to prove.
Compute expected limits independently in the tests, rather than asking the
mutated charge helper whether its own result is within budget.

The proof inventory is mandatory:

| Protection | Independent witness and compiled removal |
|---|---|
| Packed coalescing | A many-member run whose combined text fits returns one exact chunk and never constructs per-token payloads. Restore member-event construction, including a variant that coalesces that expanded vector later; the construction/peak observation fails even if the returned turns still match. |
| Fixed structural charge | 10,000 absent-data ordinary tool calls retain 7,797 one-byte turns with truncation. End at 7,797 for the no-truncation control. Remove only the constant; the independent count assertion fails. No packed rows are present. |
| Intermediate budget | Tiny ordinary calls keep charged retained storage within cap plus one candidate throughout both passes. Restore uncapped accumulation while leaving the final `display_cap` intact; the lifetime assertion fails despite unchanged returned prefix. Include oversized ordinary text to prove bytes as well as count. |
| Blockless guard | Thousands of absent-data user/results between two visible events retain no empty slots. Remove only the collection guard; the in-pass slot assertion fails while the final empty filter preserves displayed turns. Separate duplicate-sequence and diagnostic cases prove neutrality. |
| Codex key sharing | D6's two independent fresh-id mutations fail their live-key pointer/count assertions while row block sharing remains green. |
| Header EOF predicate | D7's old overflow-only predicate fails the exact cap-plus-one `discovery-limit` assertion at true EOF. |

Also cover the new index's overlap/multiplicity boundaries, singleton vs
ranged citations, duplicate quiet/empty/tool-fragment sequences, late
assemblies, late dedicated keys, and a pre-association-readable assembly
emptied by association. Preserve every version, refusal, invalid late packed
member, source fragment, recorded time, UTF-8 and diagnostic precedence
matrix. Exercise exact budget equality, first overflow, no later smaller
turn, and source-only truncation on semantic refusal.

For reproducible before/after memory evidence, choose a fresh-process Linux
peak-RSS measurement using `/usr/bin/time -v` around the release-built test
executable directly, not around Cargo. One small ignored measurement test in
the Rust test tree calls the public projector once on supplied deterministic
bytes; it creates no provider process and does no TUI work. This is a
**defensible process-memory proxy**, not allocator accounting. It avoids a
new benchmark dependency or an unsafe custom global allocator. This is the
reasoned replacement for both seats' preferred counting allocator.

Use identical harness/input bytes on production base `5ef4a842` and the
completed candidate, in isolated baseline/candidate builds. The baseline
projector remains unchanged; only the test harness is added for measurement.
Generate outside frozen `fixtures/`: a version-zero header and sixteen
text packed rows, 16,384 one-character token members per row, consecutive
nonoverlapping sequences, valid positions, and one-millisecond gaps, with
no assemblies or dedicated ids. Also run the same total 262,144 members
in one packed row to expose JSON-row transient costs. Record generator
parameters, exact encoded sizes and digests, physical-row/member counts,
compiler/profile/target, exact commands and candidate identities. This
fixed moderate source permits a repeatable baseline without deliberately
recreating this host's multi-gigabyte incident.

Run each case in at least three fresh processes and record each peak RSS
and elapsed time, with units; report the maximum observed peak as well as
the retained turn count, block count, UTF-8 bytes and accounting bytes.
Fixture generation happens before the measured process. File loading, the
resident source bytes, Rust test runtime and parsing are inside the process
peak; allocator fragmentation and resident-page behavior are included.
This cannot attribute exact bytes to individual allocations or detect every
unresident allocation. Use separate input-only and parse-only controls and
D8's lifetime observations to explain contributions; do not subtract
unrelated process maxima and label the difference exact projector memory.

Add separate tiny-call and blockless-row measurements, quiet packed tool
arguments, a block-dense message, and representative Claude/Codex tiny-row
cases for D5's residuals. No fixed before/after number is asserted here:
none has been measured in this design seat. Implementation evidence must
contain actual observations, not the issue's arithmetic, this test size,
or a final vector length presented as a peak. For every removal, record
the mutation, target command, relevant failed assertion, restored pass and
clean source state. Compile errors and unrelated refusals prove nothing.

### D9. Council reconciliation and artifact authority

All positions were read in full. The operative choices are D1-D8; this table
closes claims that otherwise risk being mistaken for accepted instructions.

| Position claims | Disposition |
|---|---|
| Simplicity S1-S4 | Combine source-local coalescing, one charge and the blockless guard with final-facts-first projection. Reject retaining all `TurnRef`s and pre-suppression payload strings (D1-D4). |
| Simplicity S5 and measurement/cuts | Adopt the two targeted lows, no API/dependency/TUI production expansion, and mutation evidence; replace its allocator choice with the explicitly limited RSS proxy (D5-D8). |
| Robustness R1-R5; F1-F5/F9 | Adopt the complete-prefix ordering, single cost, member geometry and unused-index avoidance. Use source spans and citation intervals instead of per-member maps; preserve assembly readability before tool association (D1-D4). |
| Robustness R6-R7; F11-F12 | Adopt header-length reasoning, independent lifetime observations, selection-after-cap and borrowing key observations; use the same process proxy rather than a custom allocator (D6-D8). |
| Robustness F6-F10 and alternatives A1-A5 | Reject early capping and full payload/ref accumulation. Accept parser, block-count and non-DSH intermediate costs as explicit measured residuals, not a silent exemption or a four-MB RSS claim (D5). |
| Robustness migration inventory | Adopt the view test and decision/living-spec dependencies in the Migration Plan. Add the overlooked, actually contradictory CLI test as D0's upstream finding. |
| Robustness proposed scenario retitles | No semantic correction is needed: the command scenario's retained singleton turns are explicitly separated by suppression, and the TUI's individual turns are the two coalesced chunks in its body. The specify seat preserved existing scenario names as dialect identities and added general coalescing scenarios. Reject treating these headings as permission to restore member-per-turn behavior or as an independent upstream fault. |

Only the operator accepts house decisions. The later semantic implementation
must add a `proposed` decision (next available number, currently 0062) and
its index entry. It supplements 0055 ruling 3's logical-event/turn mapping,
block-text-only budget and general packed/plain equivalence, while leaving
0061's version/seeded/token admission intact. Do not edit historical decision
0055 to pretend the new semantics were its original wording.

## Risks / Trade-offs

- [Two parses add CPU to a frequently refreshed path] -> Record elapsed time
  on the same baseline/candidate fixtures, use no unused per-token indexes,
  and share validation rather than creating divergent decoders.
- [An interval optimization can lose source order or ambiguity] -> Keep
  source ordinals separate from numeric sequences, test interval endpoints,
  late duplicates, empty members, overlapping spans and huge sparse ranges;
  never expand a citation's numeric width.
- [Readable-before-association is accidentally evaluated afterward] -> Pin
  an assembly that suppresses a chunk but loses its own last embedded block
  to a later dedicated event; final charging must retain neither fallback.
- [One candidate, one JSON row, metadata or many blocks exceed accounting]
  -> State the separate lifetimes and measured peaks under D5/D8. Never
  report accounting bytes as actual RSS or charge the candidate twice.
- [A green final-output test misses transient allocation] -> Observe actual
  construction/retention and prove each observation with its own compiled
  regression mutation; preserve the final cap during the accumulation test.
- [Scope expansion is hidden as test maintenance] -> D0 returns the concrete
  upstream dependency now. Do not edit the forbidden files or silently
  broaden CLI/TUI production work.
- [Exact coverage is confused with an in-box percentage] -> Report literal
  hit/total lines, branches and functions when measured. Namespace skips
  under #286 are reported, not excluded; host equality remains controller
  evidence. Every new production branch still needs its exercising case.

## Migration Plan

1. Resolve D0 in the upstream scope/commission record and adopt this design
   on return. Author tasks afterward in dialect dependency order. No
   `tasks.md`, house decision, code, test or measurement artifact is claimed
   complete by this design visit.
2. Add the proposed house decision and index entry with the explicit 0055
   supplement described in D9. Implement and test the shared charge, fact
   indexes, packed visitor and bounded collector in `transcript.rs` and its
   test tree, plus the single header fix and tests. Preserve every stated
   frozen directory and the in-flight resumption files byte-for-byte.
3. Revise the existing view tests for packed sequence/time, packed/ordinary
   equivalence, cap-between-members, reasoning, empty members and citation
   matrices. Keep single-member equivalence and add multi-member divergence,
   suppression-gap and indivisible-chunk cases. Update the D0-authorized CLI
   selection test and use an actual projected source for surface agreement:
   whole/selected JSON, both TUI doors, capped index 7,797/7,798 and refresh
   from `abcd` to `a`, `c`, assembly.
4. Apply this change's three deltas to the living specs in the normal dialect
   synchronization step before the implementation landing; the reading spec
   owns the merge rule once, command/TUI refer to it. This design visit does
   not prematurely synchronize or archive the change. There is no stored
   source, journal or document-field migration; displayed chunk counts and
   indices change intentionally and refresh already invalidates changed
   selections.
5. Record the reproducible measurements and all removal/restoration proofs.
   Run formatting, clippy across all targets/features, workspace tests, both
   self/verify bundle compiles and strict OpenSpec validation. Prefix git and
   test processes that invoke git with
   `env -u GIT_CONFIG_COUNT -u GIT_CONFIG_VALUE_0`. Use a writable disk-backed
   `TMPDIR` outside the repository; stage executable fixtures beside their
   targets and rename before execution.
6. Run/report in-box coverage where the toolchain is available using the
   pinned compiler from `rust-nightly-version.txt`. Leave controller host
   exact coverage and remote CI on the final head pending until observed.
   Never lower the gate, push, merge, publish or launch another Brokkr run.

Rollback is a coherent revert of the implementation and its corresponding
spec/decision updates, not an attempt to preserve old CLI indices on a new
projector. It restores the known availability defect, so the controller must
weigh that operational cost. No source files or journals need conversion.

## Open Questions

No unresolved design choice is delegated to implementation. D0 is an explicit
upstream scope finding, not an optional question. The measurement outcomes,
remaining coverage regions and controller host/remote results are pending
evidence; they cannot be replaced by predictions or declared passed here.
