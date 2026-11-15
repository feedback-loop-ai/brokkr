# 0109 — A gate's re-visit reads what changed since its last verdict, and the findings that still stand, unless a declared trigger asks for a whole read

Status: proposed
Date: 2026-11-15

Built: unbuilt (#1088)

This decision is a document only. It proposes the shapes and rules a later build
will enforce; it mints no contract version and adds no code, test, guide or
recipe. It acts on 0034's successful-result record and relies on 0071's rulings 2,
5 and 9, on 0082's minting rule, and on 0022's and 0006's residual and attempt
bounds, which stand unchanged. The replacement sentence for 0034 is in
[What 0034 says after acceptance](#what-0034-says-after-acceptance); the header
pointer and back-pointer are absent until acceptance, because this story may not
edit an existing decision. One rule stands in front of every choice: the engine
decides the read and the transition from typed facts, never from a seat's prose;
no model chooses a whole read, a delta read or a routing.

## Context

Issue #1088 is the story; #1094 is the epic. The adopter measured the problem
over 695 runs on brokkr 0.12.0, 2026-11-05 to 2026-11-15: spec 55%, code review
29%, build 10% and live verification 2% of reported spend. The spec design seat
made 302 attempts in 84 runs, 218 of them revisions after a later gate sent the
plan back; the review panel averaged 98 turns per attempt. Those figures are the
adopter's, quoted from #1088 and #1094, not measurements of this tree.

The loop is real in this tree's charters. A first review reads the whole branch;
every later review reads it again, even when the implementer changed one file. A
fresh stochastic full read also tends to add new low findings, so the loop does
not converge on its own. #1048 names the other half: a residual loop with no
progress, and a residual no writer can close.

The two issues are adjacent, not the same. #1048(a) needs a stable key per finding
so the engine can compare a visit's set with the same phase's previous set. This
decision supplies that key, the covered identity a verdict records, and the read
plan that follows. #1048(b)'s `awaiting_disposition` and its per-run cost budget
are **not** decided here; they are consumers of the same key. #1089's grounding
split is a recipe-pack story waiting on #1035's pack; it would read this
decision's `covered` and `findings` for its `ground` phase, and this decision does
not design it.

## What the tree carries today

### The result file and what the engine journals

The prompt renders a seat's result as `result`, `inputs` and `notes` and says no
other top-level key is admitted (`crates/brokkr-protocol/src/adapters.rs:476-491`).
The successful result is sealed under the seat-record contract, whose `result`
definition is closed and admits exactly those three report fields beside the
accounting fields (`contracts/seat-record.v6.schema.json:557-575`; 0034 ruling 2
at `docs/decisions/0034-the-seat-record.md:45-49`). The engine's pre-decision
check is narrower: a declared `result` string, and a declared `change` input when
the seat declares one (`crates/brokkr-runtime/src/engine/decide.rs:239-261`). The
contract fence is the store append (0034 ruling 6 at
`docs/decisions/0034-the-seat-record.md:88-100`).

`effect/succeeded` carries the whole result under `result`
(`crates/brokkr-runtime/src/engine.rs:1619-1627`). `transition/decided` carries
`from`, `result`, the matched `rule_id`, `next`, `severity`, the declared and
engine-overlaid `inputs`, and a `problem`
(`crates/brokkr-runtime/src/engine/decide.rs:152-215`). Only declared inputs
survive; undeclared keys are dropped and engine-owned keys are never accepted from
the seat (`:76-98`). `notes` is not journaled on a normal ruling; a rule-driven
park may use it as the park's `problem` (`:226-234`).

The one verdict fact recorded for a later visit is the protected phase's head. At
decide time the engine writes `reviewed_heads` as `{realm: head}` from HEAD
(`crates/brokkr-runtime/src/engine/decide.rs:120-126`), the fold keeps the latest
(`crates/brokkr-core/src/fold.rs:731-732`), and the ship gate reads it for drift
(`crates/brokkr-runtime/src/engine/decide.rs:278-290`). No other gate records a
covered head, the findings that stood, or which kind of read it took.

The recipes with at least one gate-class seat are the command's output at HEAD:
brokkr-fast, fast, gpt-flash, landing, node, panel-review, preflight, release,
research, review-first, self, standby, triage and verify — 14 files. Self's review
is one gate seat (`recipes/self/bundle.json:10-19`); preflight's review is one gate
seat with the two residual inputs (`recipes/preflight/bundle.json:39-50`);
panel-review's review is a gate panel with correctness and security members,
aggregated by `review-panel` (`recipes/panel-review/bundle.json:44-54`); triage's
analyze ends a sequence in a gate `analyst` (`recipes/triage/bundle.json:133-141`).
The production review rules read `max_residual_severity` and
`has_security_residual` (`policy/phase-machine.json:207-238`); triage's review
rules return `spec_defect` to `specify` (`recipes/triage/policy.json:42-45`).

### What a re-visit reads today

The engine does not compose a diff. It composes the seat input from the journal,
the pinned bundle and the feature (`crates/brokkr-runtime/src/engine/input.rs:52-108`),
and on a return adds `returned_from` with the result that sent the seat back
(`:144-169`). A single body receives the absolute `workdir` and no delta
(`:185-194`); a panel's members and a sequence's steps receive the same
(`:195-222`). The commission is assembled in `Engine::seat_input` and requested
in `Engine::request_or_finish` (`crates/brokkr-runtime/src/engine.rs:973-1010`).

The charters say the whole branch. The reviewer's charter says "everything on the
branch since its merge-base with the default branch (`git log` / `git diff` ...)"
(`agents/charters/reviewer.md:1-5`); the security member repeats it
(`agents/charters/review-security.md:3-9`); the preflight reviewer says "The
change is `git diff main...HEAD` and `git log main..HEAD`"
(`recipes/preflight/roles/reviewer.md:5-9`). Whether a seat also opened whole files
is **unmeasured**: no journal was opened. At HEAD the tree is 2,426 tracked files
and 1,004,496 lines (`git ls-files`); the newest commit's own diff was 10 files,
755 insertions and 68 deletions (`git show --numstat HEAD`). That is not a run's
read; it shows the two reads are different orders of magnitude.

### The residual finding today

There is no per-finding record. `brokkr-core` derives a residual from the
structured rule inputs only — `max_residual_severity`, `has_security_residual`
and `high_risk_uncovered` (`crates/brokkr-core/src/residual.rs:52-106`) — over
`transition/decided` events from `verify` and `review` (`:23-40`, `:142-170`).
Prose in `notes` is never re-read as a finding, because deriving structure from
prose is repair (0001). A panel's aggregate keeps each member's `notes` as
evidence and joins only the three aggregate inputs
(`crates/brokkr-runtime/src/bundle/aggregate.rs:188-216`); a member's individual
findings are not typed facts. That is enough for a threshold and a boolean, not
for #1048(a)'s set comparison.

### The spec gates

The shipped SDD table has `specify`, `clarify`, `design`, `tasks` and `analyze`
(`recipes/triage/policy.json:3-4`). The analyze gate is the last step of a
sequence and its result is the effect's typed result
(`recipes/triage/bundle.json:133-141`); review can return `spec_defect` to
`specify` (`recipes/triage/policy.json:42-45`). The dialect owns what a section
is: `contracts/dialect.v3.schema.json:33` defines a `decisionLocation` as an
`artifact` and a `section`, and the shipped dialects fill it
(`dialects/speckit.json:61-64`, `dialects/openspec.json:60-63`). The engine can
name a declared artifact and its bytes; it cannot name an arbitrary Markdown
heading, and it does not parse one.

## The rulings

Each ruling gives the options, what each costs and refuses, and the recommended
option. The operator may accept all or pick.

### 1. Where a verdict records its findings and its covered identity

**Recommended: (a) three optional fields on a gate's successful result —
`findings`, `follow_ups` and `covered` — under a new seat-record v7; v6 stays
read.**

- **(a) The result.** It keeps `result`, `inputs` and `notes`; a gate gains the
  seat's `findings`, the engine's non-blocking `follow_ups`, and `covered`. Cost:
  one contract version, one prompt sentence, one v6 fallback. It refuses a
  finding in `notes`, where prose is never re-read.
- **(b) Inside `inputs`.** No new seat-record version, but `inputs` is the
  transition's declared facts (0007), filtered to declared names
  (`crates/brokkr-runtime/src/engine/decide.rs:76-82`); every gate would have to
  declare `findings` or lose it, and findings would become a policy input they
  are not. Refused.
- **(c) A new `gate/verdict` event.** A new `event-envelope` version, every
  manifest and witness digest, and a new fold arm for a fact `effect/succeeded`
  already carries. Refused, as 0106 refused a new park event.

`covered` and `follow_ups` are engine-written, for the reason `boundary` is: the
engine knows the head, the artifact bytes and the delta. A result carrying either
is refused, not repaired (0001). `findings` is the seat's report; the engine
completes its `key` under ruling 2.

**Binding:** v7 closes the result with `additionalProperties: false` and `$defs`
for one finding and one covered record; result admission refuses a seat-supplied
`covered` or `follow_ups`; the 0034 ruling 6 fence validates v7 at append; tests
pin the variants and refusal text.

### 2. The stable finding key

**Recommended: (a) `key = (site, id)` — the seat writes a stable `id`, the engine
stamps the site digest from the compiled `SiteKey`, and the pair is the key.
`file`, `line`, `summary`, `severity` and `security` are the record, not the
identity.**

- **(a) Site plus id.** `id` is a lowercase identifier
  (`^[a-z0-9][a-z0-9._-]*$`, at most 64 bytes). The site is the 64-hex digest the
  compiler already computes for one structural site
  (`crates/brokkr-runtime/src/bundle/site_key.rs:163-171`), so panel members
  cannot collide and a recipe cannot borrow another site's identity. The seat
  writes no digest it cannot compute; the engine guesses no id. A moved line is
  the same key; a reworded summary with the same id is the same finding, with a
  new locator.
- **(b) `file:line` plus a summary.** A moved line closes and reopens the finding
  by accident. Refused: line is a locator.
- **(c) An engine hash over the summary.** A reworded finding is new, and the
  engine would invent identity from prose. Refused.

The engine refuses a duplicate key in one verdict, an id outside the pattern, and
a key whose site did not produce the finding.

**Binding:** a typed `FindingKey` with its own serde, `deny_unknown_fields`, the
pattern and the site digest; the contract repeats the pattern; tests pin a moved
line, a reworded summary, a duplicate id and a foreign site.

### 3. How two visits' sets are compared

**Recommended: (a) exact key matching, no fuzzy match — standing, new, closed,
and "unchanged or only grew" is `closed.is_empty()`.**

For the same phase, the previous standing set is `P`, the current is `C`:
standing `P ∩ C`, new `C - P`, closed `P - C`. "Only grew" means `C - P` is
non-empty and `P - C` is empty; "unchanged" means both are empty; progress means
`P - C` is non-empty. A moved line is standing because line is not in the key. A
finding that moves file under the same id and site is standing; the file is a
locator. A finding the seat drops is closed; if it was not fixed, the review says
so, not a fuzzy engine guess. #1048(a) consumes `closed.is_empty()` as
`residuals_unchanged_since_last_visit`; its condition, table wiring and routing
belong to #1048.

**Binding:** a pure `compare(previous, current)` in `brokkr-core` returning the
three typed sets; tests pin equal, moved, reworded, closed and only-grew cases,
and pin that `line` is ignored.

### 4. Older results with no key

**Recommended: (a) a whole read, never a guessed key.**

A previous verdict with no `covered`, or with `covered` but no `findings`, is
`unkeyed_result`; a phase never visited is `first_visit`. Both are whole reads. A
v6 journal takes the same road. It refuses to synthesize a finding from `notes`,
to treat an empty set as clean, or to reuse an old aggregate severity as a key.

**Binding:** the trigger enum has `first_visit` and `unkeyed_result`; a test hands
the planner a v6-shaped result and asserts the full read and the exact trigger;
no path builds a `FindingKey` without a site and an id.

### 5. What the verdict records as covered

**Recommended: (a) an engine-stamped `covered`, typed by the gate's kind: `head`
for a code gate, `artifacts` for a spec gate.**

- **`head`:** the git commit the gate read, the base it compared against when one
  resolves, and whether the worktree was dirty at the start. A dirty worktree
  forces a whole read, because a commit-range delta would miss the uncommitted
  bytes the gate read.
- **`artifacts`:** the declared artifact files and each file's SHA-256, taken
  before the gate runs. A missing declared artifact refuses (0001).

The engine already holds the head: `arms_effect_gate_head` arms the stable-head
check and `finish_gate_head_check` refuses a gate that moved it
(`crates/brokkr-runtime/src/engine.rs:947-968`, `:3219-3227`). The covered head is
that start head, not a seat report. `base` is the merge-base resolved at
commission time; when none resolves, `base` is `null` and a delta read is refused
(`base_unknown`). `reviewed_heads` and ship drift stay as they are; this is one
more consumer of the same head.

**Binding:** `covered` is a tagged enum, each arm with required fields and
`additionalProperties: false`; the engine stamps it before append and refuses a
seat-supplied one; tests pin both arms, the dirty refusal and the unresolved-base
refusal.

### 6. Where the read plan and the verdict live

**Recommended: (a) the read plan in the `effect/requested` payload, the verdict
in the successful result, and one `GateVisit` per phase in the fold.**

The request gains a typed `read`: `kind` (`full` or `delta`), the triggers that
fired, `since` and `base`, the changed file count, the diff digest and the
standing count. It is the one home for "which kind of read was this, and why". A
proposed gate-read v1 extension schema types it, as 0106 did for `park-cause`; the
event-envelope v1 payload stays open and no event type moves. The verdict's
findings and covered live in the result under ruling 1. The fold reads the result
it already keeps as `last_result` (`crates/brokkr-core/src/fold.rs:659-665`) and
records one `GateVisit` per phase when the ruling closes the effect. `inspect` and
`analytics` read the same events; no engine decision reads a view.

**Binding:** a typed `GateRead` in `brokkr-core` (no `Value` past the edge), the
extension schema, and a fold test that the `GateVisit` rebuilds from the journal
alone; a view test that `inspect` reads the same plan. A new event is refused: it
needs an `event-envelope` version and moves every digest; an input-only plan is
refused because the input is digested, not journaled.

### 7. What the delta commission carries

**Recommended: (a) the engine computes the range since the covered head, the
paths that still differ from the base, the unified diff for those paths, and the
standing findings.**

The engine composes: (1) `since` is the previous covered revision; if it is not an
ancestor of HEAD, the read is full (`ancestor_lost`). (2) `base` is the current
merge-base of HEAD and the default branch; if none resolves, the read is full
(`base_unknown`). (3) The changed paths are `git diff --name-only since..HEAD`,
kept only where `git diff --name-only base...HEAD` also names the path; a path the
base touched and the branch still differs from is read whole, which over-reads a
mixed file rather than under-reading it, and a clean merge of the base leaves no
path and owes no re-read. (4) The diff is `git diff since..HEAD --` over those
paths, capped by `delta_files_gt` and `delta_bytes_gt`; over either cap is a whole
read. (5) The commission carries the `read` object, the diff text, and the
standing findings with their keys, locators and severities. A security-class
position in a mixed panel takes the whole read instead (ruling 10).

No model computes a delta or chooses which paths to omit. A dirty worktree is a
whole read before step 1. Whether a visit with an empty delta is skipped is
#1048's no-progress question: the gate still visits and its commission says no
files changed.

**Binding:** a pure delta/range planner over a small repository port, with the git
calls at the runtime edge; the plan is part of the digested input, so recovery
rebuilds it or parks; tests pin the exact path set for an ordinary fix, a base
merge, an empty delta, a rewritten history and both caps. A seat-run `git diff` is
refused: the engine cannot prove what the seat read.

### 8. A finding outside the delta, and the security refusal

**Recommended: (a) a finding outside the delta is not re-opened as a block; it is
carried as a follow-up the PR names. A standing security finding always blocks,
and a position that stands it takes a whole read.**

The verdict has two arrays of one finding shape: `findings` (blocking) and
`follow_ups` (carried, non-blocking). The engine, not the seat, carries them. A
previous finding inside the delta and absent is closed. A previous finding outside
the delta and absent is carried into `follow_ups`, so it is not closed and
#1048(a) does not read false progress. A previous **security** finding is never a
follow-up. If it still stands, it is blocking, its position takes a whole read
(`standing_security`), and `has_security_residual` must be `true`; if the full read
closes it, it is closed like any finding. If its site no longer exists or is not
security-class, the whole phase takes a whole read. A finding the seat reports is
blocking; the seat cannot demote one.

The engine checks the declared aggregate against the blocking findings: a
`max_residual_severity` lower than the highest blocking finding, or
`has_security_residual` false while a blocking finding has `security: true`,
refuses the result. `spec_defect` stays the seat's flag; no finding derives it.
The PR names the follow-ups (ruling 12); the review office is read-only, and every
finding is a test case with a name and a location. A delta read can still miss a
finding no one ever reported; that is the accepted cost of trusting a clean prior
verdict, and why the security-class position and a standing security finding never
take the delta.

**Binding:** `findings` and `follow_ups` share one definition and the key is unique
across both; the engine derives the follow-ups from the previous `GateVisit` and
the delta; the aggregate check is a typed refusal; tests pin the carried
follow-up, the security block, the understated-severity refusal and the
false-security refusal. Re-opening every standing finding as a block is refused
because the loop never converges; dropping the outside findings is refused because
a standing security finding could disappear.

### 9. The whole-read triggers and their home

**Recommended: (a) a top-level `reads` map in a `forge.phase-machine/v3` table,
one entry per gate phase, with a closed trigger list; an absent entry is a whole
read, and a v2 table may not carry `reads`.**

| Trigger | Fires when | Refuses |
|---|---|---|
| `first_visit` | the phase has no previous verdict in this run | a delta read for a first read |
| `unkeyed_result` | the previous verdict has no covered identity or no keyed findings | a guessed key |
| `ancestor_lost` | the covered head is not an ancestor of the current head | a delta across rewritten history |
| `base_unknown` | no base resolves for the comparison | a delta with no base |
| `worktree_dirty` | the worktree is dirty at commission time | a commit-only delta over uncommitted bytes |
| `standing_security` | a previous finding with `security: true` still stands | carrying a security finding as a follow-up |
| `delta_files_gt` | the changed path count is over the declared N | an unbounded diff |
| `delta_bytes_gt` | the diff bytes are over the declared N | an unbounded input |
| `delta_touches` | any changed path matches a declared glob | a delta read over a protected surface |
| `operator_whole_read` | a journaled one-shot operator command names the phase | a recipe turning a whole read off |

The entry names the phase's sites, each with its read policy; a site the entry
does not name is a whole read, so a new gate site cannot accidentally inherit a
delta. The journal records every trigger that fired, in a canonical order; a
delta read records none. The glob grammar is closed: `*` matches any run except `/`, `**`
matches any run including `/`, `?` matches one character except `/`, every other
character is literal, matching is case-sensitive against a repository-relative
path, and a pattern containing `..` or an absolute path is refused. A pattern
that matches nothing is not an error.

The home is a top-level `reads` map because the read is decided **before** the
effect, while `when` is evaluated **after** the result, at the transition. Putting
the read policy in `when` would give one fact two homes and ask a post-effect
condition to decide a pre-effect read. The map sits beside the `rules` array whose
`when` vocabulary it mirrors, and is matched with the same closed-enum rule: an
unknown trigger word refuses at load, and no wildcard arm decides. A per-rule
`read` beside `when` is refused because it repeats the plan on every result arm
and lets two arms disagree; a recipe-bundle field is refused because it is not the
phase table.

The operator's `whole_read` is a journaled `operator/commanded` command scoped to
a run and a phase, consumed by exactly the next visit; it is not a recipe flag and
not a model choice. A v3 table without a `reads` entry for a gate phase is a whole
read, so a recipe opts in; a v2 table is read as it is.

**Binding:** a phase-machine v3 schema that admits `reads` and refuses it under
v2; a closed `Trigger` enum with serde and the loader's refusal; the glob matcher
as a pure function in `brokkr-core`; tests for every trigger, an unknown word, a
v2 table carrying `reads`, and the operator one-shot.

### 10. Which positions are security-class

**Recommended: (a) a position is security-class exactly when its compiled result
declares `has_security_residual` and its phase's read policy names that site as
security-class. A phase all of whose positions are security-class is a
security-class gate. A security-class position always takes a whole read.**

The policy may name more positions than the floor requires, never fewer. The
loader refuses a delta mark on a site whose member key is `security` or whose
compiled office is `review-security`; that is a refusal, not a role grant, and a
recipe cannot turn it off. The office remains the agent's name (0107's
distinction); the class is a property of the read policy and the declared security
result, not a new role. A mixed review panel whole-reads its security member and
may delta-read its correctness and simplicity members; a dedicated security gate
whole-reads. An absent read policy is a whole read for the phase.

**Binding:** the compiler derives each site's `security_class` from its
office/member key and the declared `has_security_residual`, writes it into the
compiled read policy, and refuses a policy that marks it delta; a compile test
pins a mixed panel and a dedicated security gate. Deriving the class from the agent
file name alone is refused (0107 refused name inference for a gate role); the
operator naming every position every run is refused as the only mechanism,
because a forgotten name silently full-reads and a run cannot start without the
operator.

### 11. Spec review, grounding and analysis

**Recommended: (a) the same facts, with the section owned by the dialect and the
file owned by the engine.**

The engine can know an artifact **file** and its bytes, and a **section** only
where the dialect declares one: `contracts/dialect.v3.schema.json:33` defines a
decision location as an artifact plus a section, and the shipped dialects fill it
(`dialects/speckit.json:61-64`, `dialects/openspec.json:60-63`). Without such a
declaration the engine knows no section, so a changed artifact file is read whole;
it never parses arbitrary headings to guess one.

A spec review, grounding or analysis pass reads the same `read` plan: changed
artifact files since its covered digests, the amended declared sections where the
dialect names them, and the standing findings that sent the revision back. The
`analyze` gate (`recipes/triage/policy.json:36-40`) gets the same plan as review;
the recipe's dialect decides whether a section can be named. What stays the
recipe's: section markers, the claims a section makes, and the split of grounding
results into `unquoted`, `noted` and `contradicted`. #1089 is that split, in
#1035's pack; when it lands it reads this decision's `covered` and `findings` for
its `ground` phase. This decision does not design it.

**Binding:** the covered `artifacts` arm records each file's digest; the read plan
may carry a declared section only when the dialect's decision location names it;
tests pin a file-level delta, a declared-section delta and the whole-file fallback.
Parsing Markdown headings is refused because a heading is not a typed section; the
seat naming sections is refused because the engine could not compare two visits
deterministically.

### 12. What the readouts show

**Recommended: (a) `analytics` separates full reads from delta reads and
`inspect` names each visit's read kind and triggers, both derived once in
`brokkr-view`.**

`analytics` gains a `reads` figure under `Rework`: full count, delta count and
trigger counts (the readout already has `sent_back` and `second_try_*` at
`crates/brokkr-view/src/analytics.rs:328-340`). `inspect` gains a read cell on the
participant row, beside the visit number it already carries
(`crates/brokkr-view/src/lib.rs:387-399`); the renderer paints it
(`crates/brokkr-cli/src/render.rs:412-503`). A journal with no `read` record reads
`unrecorded`, never zero, so an older run is not counted as a delta read. The view
is the one derivation (0071 ruling 7); the CLI, the TUI and `ui.html` only paint
it, and no engine decision admits anything from the view.

**Binding:** a view test pins the full/delta/trigger counts and the `unrecorded`
case; a render test pins the `inspect` line; a public-API snapshot moves only for
the added view fields. `analytics`-only is refused because the operator cannot tell
one visit's read from another's; no readout is refused because the saving is
invisible.

## What 0034 says after acceptance

The current sentence in 0034 ruling 2 reads
(`docs/decisions/0034-the-seat-record.md:45-49`):

> The phase machine's separately governed `result`, `inputs`, and `notes` report
> remains inside a successful result; those fields do not widen the adjacent
> seat-record vocabulary.

After acceptance it reads:

> The phase machine's separately governed `result`, `inputs`, and `notes` report
> remains inside a successful result, and a gate's result may also carry the
> separately governed `findings`, `follow_ups` and `covered` report that decision
> 0109 rules; those fields do not widen the adjacent seat-record vocabulary, and
> only seat-record v7 admits the three gate fields.

The replacement takes effect with acceptance; 0034 gains the `Amended by: 0109`
back-pointer then, not in this story. No other decision's text changes. 0022,
0006 and 0082 stand as written.

## What stays out

- **No model decides a routing or a read.** The engine computes the trigger set
  and the delta; the policy table routes the typed result. `security-hold`,
  `has_security_residual` and every rule that reads them are unchanged.
- **No refusal is weakened.** The trust tier, egress class, boundary, schema
  refusal, attempt cap, residual park and `security-hold` stop stand.
- **No delta before a full read.** A phase with no full verdict takes
  `first_visit` or `unkeyed_result`; there is no fallback that guesses.
- **No Windows code.** Decision 0063 keeps Linux and macOS as the hosts.
- **#1048(b) and #1048(c) are out.** `awaiting_disposition` and the per-run cost
  budget may consume the key and the read facts; their outcome, fields and routing
  are #1048's.
- **#1089's recipe split is out.** It waits on #1035's pack and reads this
  decision's facts; the grounding split is not designed here.
- **No contract is minted and no code, test, guide, recipe or existing decision is
  edited.** The slices are the commission; the first moves the marker from
  `unbuilt`.

## The slices

Smallest first. The recommended first slice records the key and the covered
identity, and reads them back, with no routing change; the next field sweep can
then measure what a delta read would have saved before any visit is shortened.

| Slice | What it builds | Files touched (rough) | Size | Contract changes | Tests it adds |
|---|---|---|---|---|---|
| 0109a | The finding key, the covered identity, and the read-back; no routing change. | contracts/seat-record.v7.schema.json (new); crates/brokkr-core/src/residual.rs or a new findings module; crates/brokkr-core/src/fold.rs; crates/brokkr-runtime/src/engine.rs; crates/brokkr-runtime/src/engine/decide.rs; crates/brokkr-runtime/src/bundle/aggregate.rs; crates/brokkr-protocol/src/adapters.rs; crates/brokkr-view/src/residual_findings.rs; crates/brokkr-cli/src/render.rs | about 900 lines with tests | seat-record v7 (proposed, minted on acceptance) | v7 result validation; engine-stamped covered on both arms; a moved line and a reworded summary; a panel's member findings joined; a v6 result reads unkeyed; `inspect` prints the findings and covered |
| 0109b | The shadow delta: compute the range, paths, diff digest and standing set on every re-visit, journal it as the plan it would use, but still commission a full read. | crates/brokkr-core (a pure delta/set module); crates/brokkr-runtime/src/engine/input.rs; crates/brokkr-runtime/src/engine.rs; crates/brokkr-view/src/analytics.rs; the recipe policies that will later opt in | about 450 lines with tests | gate-read v1 (proposed, typed in the effect/requested payload) | a first visit has no shadow; a second visit names the exact paths; a base merge removes base-only paths; an empty delta; a rewritten history; the potential saving in analytics |
| 0109c | The read policy and the triggers: a phase-machine v3 `reads` map, the closed trigger enum, the glob matcher, and a delta commission for one non-security gate (the analyze gate first). | contracts/phase-machine.v3.schema.json (new); crates/brokkr-core/src/policy.rs and its condition module; crates/brokkr-runtime/src/engine/input.rs; crates/brokkr-runtime/src/engine.rs; recipes/triage/policy.json (the analyze gate's read policy) | about 700 lines with tests | phase-machine v3 (proposed) | every trigger; an unknown trigger refused; a v2 table carrying `reads` refused; an absent `reads` is full; the analyze gate's delta and its whole-read fallbacks |
| 0109d | Review panels, security-class positions, standing security and follow-ups: the engine carries outside-delta findings, refuses an understated aggregate, and whole-reads a security position. | crates/brokkr-runtime/src/bundle/aggregate.rs; crates/brokkr-runtime/src/bundle (the security-class derivation); crates/brokkr-runtime/src/engine/decide.rs; contracts/seat-record.v7.schema.json; recipes/panel-review, recipes/preflight, recipes/self and the other review recipes' read policies | about 650 lines with tests | none new; v7's `follow_ups` and the aggregate rule | a carried follow-up is not closed; a standing security finding blocks and forces full; a false security flag refuses; a mixed panel's security member whole-reads; a panel's follow-ups are named |
| 0109e | The readouts and the operator one-shot: `analytics` full/delta and trigger counts, `inspect`'s read cell, `operator whole-read`. | crates/brokkr-view/src/analytics.rs; crates/brokkr-view/src/lib.rs; crates/brokkr-cli/src/render.rs; crates/brokkr-cli/src/verbs (the operator verb and its help); the operator command's reader | about 400 lines with tests | none new | full/delta/trigger counts; `unrecorded` for a journal with no read record; the `inspect` line; the operator command is one-shot and phase-scoped; no operator command can widen a security refusal |

The order is buildable: a gives b the key and the covered identity, b gives c the
measurement and the planner, c gives d the policy and the plan, and d gives e the
verdict facts it paints. Each slice moves the decision's `Built:` marker and the
index cell only when its tests pass. The first slice is recommended because it
changes no routing: the next field sweep can compare the shadow delta's size with
the full read it still commissioned, the number the operator needs before opting a
recipe in.

## Consequences

The saving is the adopter's figure, not a promise: if a review re-visit reads only
the fix diff and the standing findings, the 29% code-review share and the 55% spec
share have room to fall where the changes are small. The cost is a contract
version, a new read plan in every gate's request, a delta computation that runs
git in the engine, and triggers a recipe author must choose. The refusal that
keeps the saving honest is that a security-class position and a standing security
finding never take the delta.

The residual risks are named rather than hidden. A delta read trusts the prior
full verdict: a finding no one reported is not in the standing set, and re-reading
only the delta will not find it; the whole-read triggers are the operator's bound
on that cost. A seat can reword a finding and give it a new id: the old key closes
and the new opens, so #1048(a) reads progress; the standing findings and the
readout's per-key diff make it visible, but the engine does not guess a fuzzy
match. A dirty worktree or an unresolved base forces a whole read, so some
re-visits save nothing: a delta that cannot prove its range is not a delta. The
readout counts only journals that carry the `read` record; older runs read
`unrecorded`, never zero.

## Evidence

The inventory above was read at HEAD `94ada796d`; no journal was opened. The
commands and their output that the claims rest on:

```text
git ls-files | wc -l
2426
git ls-files -z | xargs -0 wc -l | tail -1
1004496 total
git show --numstat --format= HEAD | awk '{a+=$1; d+=$2} END {print a " insertions, " d " deletions"}'
755 insertions, 68 deletions
grep -rln '"class": "gate"' recipes/*/bundle.json
brokkr-fast fast gpt-flash landing node panel-review preflight release
research review-first self standby triage verify
gh pr list --search 0109 --state open
(no output)
```

The result-file shape, the engine's journaling, the reviewed head, the residual
derivation, the panel aggregate, the policy vocabulary, the dialect section and
the readouts are cited by file and line above. Everything not observed is marked
unmeasured: in particular, how many files a particular gate seat actually opened
on a particular re-visit, and the saving a delta would have produced in any real
run. Those are the first slice's measurement, not this document's claim.
