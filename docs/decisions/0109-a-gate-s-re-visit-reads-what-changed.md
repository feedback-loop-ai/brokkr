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

This is the second fix round for 0109. It answers the review panel's findings in
the rulings themselves, not in a separate errata list:

| Finding | Answered in |
|---|---|
| HIGH: a `clean` result can carry a blocking security finding and ship | ruling 2 |
| HIGH: the security-class floor misses the seats that can hold for security, and is defined three ways | ruling 11 |
| Security smell: the delta's `git diff` runs in the operated repo | ruling 9 |
| MEDIUM: what is "inside the delta" has no exact rule | ruling 5 |
| MEDIUM: a carried follow-up can vanish on the next visit | ruling 4 |
| MEDIUM: a spec gate's section delta is not computable from `covered` | ruling 12 |
| MEDIUM: the read plan has two homes | ruling 8 |
| MEDIUM: the covered head is a third record of a head already journaled | ruling 7 |
| MEDIUM: the charters still order a whole-branch read | slice 0109c |
| MEDIUM: the slices are not buildable as written | [The slices](#the-slices) |
| LOW/INFO: the v6 `state` field; the recipe count; the index row; `returned_from`; `notes`; the Evidence counts; the single-shape rulings; the gate kind; absent `findings`; `worktree_dirty`; the foreign site; `inspect`'s visit; the cite slips | the inventory below and rulings 3, 5, 6, 7, 10, 12 and 13 |

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
other top-level key is admitted (`crates/brokkr-protocol/src/adapters.rs:478-491`).
The successful result is sealed under the seat-record contract, whose `result`
definition is closed and admits four report fields beside the accounting fields:
`result`, `inputs`, `notes` and the dialect step's `state`
(`contracts/seat-record.v6.schema.json:557-575`; `state` at `:572-574`;
0034 ruling 2 at `docs/decisions/0034-the-seat-record.md:45-50`). The old draft
said three; a v7 built from three would refuse a dialect step's `state`. The
engine's pre-decision check is narrower: a declared `result` string, and a
declared `change` input when the seat declares one
(`crates/brokkr-runtime/src/engine/decide.rs:239-261`). The contract fence is the
store append (0034 ruling 6 at `docs/decisions/0034-the-seat-record.md:88-100`).

`effect/succeeded` carries the whole result under `result`
(`crates/brokkr-runtime/src/engine.rs:1619-1627`). `transition/decided` carries
`from`, `result`, the matched `rule_id`, `next`, `severity`, the declared and
engine-overlaid `inputs`, and a `problem`
(`crates/brokkr-runtime/src/engine/decide.rs:152-215`). Only declared inputs
survive; undeclared keys are dropped and engine-owned keys are never accepted from
the seat (`:76-98`). `notes` is carried in `effect/succeeded`'s `result`, not in
`transition/decided`'s payload; a rule-driven park may use it as the park's
`problem` (`:226-234`).

For the protected phase the tree already records the head twice. `phase/entered`
carries the head the phase was entered at, as an optional payload field
(`crates/brokkr-runtime/src/engine.rs:3086-3106`;
`contracts/phase-entered-head.v1.schema.json:1-12`), and at decide time the engine
writes `reviewed_heads` as `{realm: head}` from HEAD
(`crates/brokkr-runtime/src/engine/decide.rs:120-126`), the fold keeps the latest
(`crates/brokkr-core/src/fold.rs:731-732`), and the ship gate reads it for drift
(`crates/brokkr-runtime/src/engine/decide.rs:278-290`). A gate in any other phase
records no head: `phase/entered` writes one only for the protected phase and a
returning implement (`crates/brokkr-runtime/src/engine.rs:3096-3107`), and `reviewed_heads` is written only
for the protected phase (`crates/brokkr-runtime/src/engine/decide.rs:120-126`). No gate records the findings that
stood or which kind of read it took.

The recipes with at least one gate-class seat are the command's output at HEAD:
brokkr-fast, fast, gpt-flash, landing, node, panel-review, preflight, release,
research, review-first, self, standby, triage and verify — 14 bundles declare a
gate in their own `bundle.json`. Five more declare no gate and inherit one:
night-shift (extends triage), research-dsh (extends research), and wager-harness,
wager-harness-dsh and wager-harness-muse (extend brokkr-fast), so at least 19
recipes have a gate after inheritance. The compiled count is **unmeasured**; the
declaration and inheritance counts are the measured facts. Self's review is one
gate seat (`recipes/self/bundle.json:10-19`); release's is one
(`recipes/release/bundle.json:31-43`); preflight's is one with the two residual
inputs (`recipes/preflight/bundle.json:39-50`); panel-review's review is a gate
panel with correctness and security members, aggregated by `review-panel`
(`recipes/panel-review/bundle.json:30-54`); triage's analyze ends a sequence in a
gate `analyst` (`recipes/triage/bundle.json:133-141`). The production review rules
read `max_residual_severity` and `has_security_residual`
(`policy/phase-machine.json:207-238`); the clean rules have no `when` on either
(`:274-291`); triage's review rules return `spec_defect` to `specify`
(`recipes/triage/policy.json:42-45`).

### What a re-visit reads today

The engine does not compose a diff. It composes the seat input from the journal,
the pinned bundle and the feature, as a pure function of those three, so recovery
rebuilds it (`crates/brokkr-runtime/src/engine/input.rs:49-57`); on a re-visit it
adds `returned_from` with the result that sent the seat back, and it does so for
any second visit (`:157-170`), not only a policy return. A single body receives
the absolute `workdir` and no delta (`:185-194`); a panel's members and a
sequence's steps receive the same (`:195-222`). The commission is assembled in
`Engine::seat_input` and requested in `Engine::request_or_finish`
(`crates/brokkr-runtime/src/engine.rs:973-1010`).

The charters say the whole branch. The reviewer's charter says "everything on the
branch since its merge-base with the default branch (`git log` / `git diff` ...)"
(`agents/charters/reviewer.md:3-5`); the security member repeats it
(`agents/charters/review-security.md:3-9`); the preflight reviewer says "The
change is `git diff main...HEAD` and `git log main..HEAD`"
(`recipes/preflight/roles/reviewer.md:5-9`). The prompt renders the role text
(`crates/brokkr-protocol/src/adapters.rs:478-491`). Whether a seat also opened
whole files is **unmeasured**: no journal was read.

At the base `94ada796d` the tree is 2,426 tracked files and 1,004,496 lines
(`git ls-files`). The design story's branch diff at `a20fdaa13` is 2 files, 569
insertions and 0 deletions (`git diff --stat $(git merge-base a20fdaa13 origin/main)..a20fdaa13`);
that is the read a re-visit of this story would be handed, not a whole-tree read.
The base commit's own diff is 10 files, 755 insertions and 68 deletions. A final
branch diff after this revision's own commit is **unmeasured**, because the
document changes it.

### The residual finding today

There is no per-finding record. `brokkr-core` derives a residual from the
structured rule inputs only — `max_residual_severity`, `has_security_residual`
and `high_risk_uncovered` (`crates/brokkr-core/src/residual.rs:52-106`) — over
`transition/decided` events from `verify` and `review` (`:23-40`, `:136-170`).
Prose in `notes` is never re-read as a finding, because deriving structure from
prose is repair (0001). A panel's aggregate keeps each member's `notes` as
evidence and joins four inputs — `fixes_applied`, `spec_defect`,
`max_residual_severity` and `has_security_residual`
(`crates/brokkr-runtime/src/bundle/aggregate.rs:188-216`); a member's individual
findings are not typed facts. That is enough for a threshold and a boolean, not
for #1048(a)'s set comparison.

A seat with no explicit `inputs` compiles with the inputs its phase's rules
reference (`crates/brokkr-runtime/src/bundle/assemble.rs:303-313`). Self and
release have no `policy.json` of their own; they inherit the fast table through
panel-review and brokkr-fast. The fast review rules reference
`max_residual_severity` but **not** `has_security_residual`
(`recipes/fast/policy.json:103-186`; a grep for `has_security_residual` in that
file prints nothing). The five review seats that declare no `inputs` —
brokkr-fast, fast, node, self and release — therefore compile with
`max_residual_severity` only. The agent files declare no `inputs` either
(`agents/brokkr-reviewer.json:1-6`; `agents/reviewer.json:1-6`). A security
finding from any of those seats is invisible to a check that reads
`has_security_residual`, so ruling 5's aggregate check would refuse a non-empty
`findings` with the input absent (fail closed), and slice 0109d writes both
inputs into those five seats. Review-first, standby, preflight, panel-review,
triage and gpt-flash already declare both.

### The spec gates

The shipped SDD table has `specify`, `clarify`, `design`, `tasks` and `analyze`
(`recipes/triage/policy.json:3-4`). The analyze gate is the last step of a
sequence and its result is the effect's typed result
(`recipes/triage/bundle.json:133-141`); review can return `spec_defect` to
`specify` (`recipes/triage/policy.json:42-45`). The dialect owns what an artifact
**name** is: an artifact step lists names such as `spec`, `plan`, `tasks` and
`specs` (`contracts/dialect.v3.schema.json:30-33`), and `decisionLocation` is
where a loop phase records its decisions — an artifact name plus a section string
(`:33`), filled by the shipped dialects (`dialects/speckit.json:61-64`,
`dialects/openspec.json:60-63`). It is not a path map and not a section map. The
engine can name a declared artifact *file* only when a typed path or glob names
it; it knows a section only as the dialect's string, and it parses no heading.

## The rulings

Each ruling gives the options, what each costs and refuses, and the recommended
option. The operator may accept all or pick. Where a ruling offers one shape, it
says so.

### 1. Where a verdict records its findings, follow-ups and covered identity

**Recommended: (a) three optional fields on a gate's successful result —
`findings`, `follow_ups` and `covered` — under a new seat-record v7; v6 stays read
and `state` is kept.**

- **(a) The result.** It keeps `result`, `inputs`, `notes`, `state` and the
  accounting fields; a gate gains the seat's `findings`, the engine's non-blocking
  `follow_ups`, and `covered`. Cost: one contract version, one prompt sentence,
  one v6 fallback. It refuses a finding in `notes`, where prose is never re-read.
- **(b) Inside `inputs`.** No new seat-record version, but `inputs` is the
  transition's declared facts (0007), filtered to declared names
  (`crates/brokkr-runtime/src/engine/decide.rs:76-82`); every gate would have to
  declare `findings` or lose it, and findings would become a policy input they
  are not. Refused.
- **(c) A new `gate/verdict` event.** A new `event-envelope` version, every
  manifest and witness digest, and a new fold arm for a fact `effect/succeeded`
  already carries. Refused, as 0106 refused a new park event.

`covered` and `follow_ups` are engine-written, for the reason `boundary` is: the
engine knows the head, the artifact digests and the delta. A result carrying
either is refused, not repaired (0001). `findings` is the seat's report; the
engine completes its `key` under ruling 3.

**Binding:** v7 closes the result with `additionalProperties: false` and `$defs`
for one finding and one covered record; it keeps `state` and the accounting
fields; result admission refuses a seat-supplied `covered` or `follow_ups`; the
0034 ruling 6 fence validates v7 at append; tests pin the variants, the four
report fields and the refusal text.

### 2. A verdict's word is bound to its findings

**Recommended: (a) refuse a malformed verdict — never rewrite the seat's word.**

A gate result whose `findings` array is non-empty is bound as follows:

- any finding with `security: true` requires the word `security-hold`;
- any other blocking finding refuses `clean` and `pass`; `residual` and
  `clean-unverified` are admitted, and `security-hold` is admitted as a stricter
  word;
- `follow_ups` are non-blocking and do not change the word.

The engine refuses the result as malformed before the transition: it journals
the same `__schema-invalid__` `transition/decided` the existing schema check
writes (`crates/brokkr-runtime/src/engine/decide.rs:31-46`) and the run parks.
The result never reaches a rule. The refusal is what keeps `clean` from carrying a
high security finding past `REVIEW-CLEAN` (`policy/phase-machine.json:285-291`) and
`REVIEW-CLEAN-NO-FIXES` (`:274-283`), which have no `when` on the residual inputs.
The engine does not derive the word from the findings: the word is the seat's
typed verdict, and rewriting it would make the engine a second author, hide a seat
that claims clean over its own findings, and leave no journaled refusal to repair.
A refusal is the fail-closed cost.

- **(b) Derive the word.** A security finding would force `security-hold` and a
  non-security finding `residual`. Refused for the reason above: it silently
  replaces a seat's claim and makes the engine's rewrite the record.

**Binding:** the check sits with the result schema check
(`crates/brokkr-runtime/src/engine/decide.rs:239-261`) and is a typed function
over the parsed findings, not prose. Tests: `clean` plus one high security finding
is refused, never shipped; `clean` plus a medium non-security finding is refused;
`security-hold` plus a high security finding is admitted; `residual` plus a medium
finding is admitted; `clean` plus an engine-carried `follow_ups` entry is admitted.

### 3. The stable finding key

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
  new locator; a finding whose file moves under the same id and site is standing,
  because the file is a locator.
- **(b) `file:line` plus a summary.** A moved line closes and reopens the finding
  by accident. Refused: line is a locator.
- **(c) An engine hash over the summary.** A reworded finding is new, and the
  engine would invent identity from prose. Refused.

`file` is required and repository-relative; `line` is a positive integer locator;
`summary` is non-empty; `severity` is the house's closed vocabulary; `security` is
a boolean. The engine refuses a duplicate `(site, id)` in one verdict, an id
outside the pattern, and any finding that carries a `site` field: a seat-supplied
site is the only input that could carry a foreign site, and refusing it is the
whole foreign-site rule. The engine stamps the site of the site that reported the
finding, so a panel member's finding carries that member's digest and a copy of
another member's finding is a different key, not a borrowed one.

**Binding:** a typed `Finding` and `FindingKey` with their own serde,
`deny_unknown_fields`, the pattern and the site digest; the contract repeats the
pattern; tests pin a moved line, a reworded summary, a moved file, a duplicate id,
and a seat-supplied `site`.

### 4. How two visits' sets are compared, and what the visit records

**Recommended: (a) exact key matching, no fuzzy match, over the union of
`findings` and `follow_ups`; each visit's `GateVisit` records both.**

For the same phase, the previous standing set is `P`, the current is `C`, and
each is the union of the visit's `findings` and `follow_ups`: standing `P ∩ C`,
new `C - P`, closed `P - C`. "Only grew" means `C - P` is non-empty and `P - C`
is empty; "unchanged" means both are empty; progress means `P - C` is non-empty.
A moved line or a moved file is standing because neither is in the key. A finding
the seat drops is closed only by a covering read (ruling 5); if it was not fixed,
the review says so, not a fuzzy engine guess. `compare()` runs over the union, so
a finding carried as a follow-up in the first visit is in `P` in the second: it
stays standing until a covering read closes it, and it cannot vanish because the
next derivation looked only at `findings`.

The fold records one `GateVisit` per phase when the ruling closes the effect,
with `findings`, `follow_ups`, `covered`, the read kind and the triggers. The next
visit reads the latest `GateVisit` for that phase, not the untyped `last_result`
(`crates/brokkr-core/src/fold.rs:659-665`). `#1048(a)` consumes `closed.is_empty()`
as `residuals_unchanged_since_last_visit`; its condition, table wiring and routing
belong to #1048.

**No alternative offered:** a fuzzy match would invent identity from prose, and
comparing only `findings` is the vanishing-follow-up bug. Cost: one pure
`compare()` and one fold record per phase. **Binding:** a pure `compare(previous,
current)` in `brokkr-core` over the union; tests pin equal, moved, reworded,
closed, only-grew, and a follow-up carried into the second visit; a fold test
rebuilds `GateVisit` from the journal with both arrays; two consecutive delta
visits show a finding carried in the first is still carried or closed by a
covering read in the second.

### 5. What "inside the delta" means, and the follow-up

**Recommended: (a) a changed hunk, not a changed path.**

- **(a) The hunk.** The rule below. Cost: the engine keeps the hunk ranges the
  diff already carries, and a finding in an untouched part of a changed file is
  carried rather than closed.
- **(b) The changed path.** A finding in a changed file is inside. Simpler, but it
  closes a standing finding whose hunk was not read — the adversarial's case — and
  makes #1048(a) read false progress. Refused.

- **Inside.** A finding is inside the delta when its `file` is a changed path and
  its `line` falls in a changed hunk of the `since..HEAD` diff for that path,
  widened by the diff's context (the plan uses `-U3`, so three lines). The engine
  records the hunks it read; it does not guess.
- **Fileless or unplaceable.** A finding with no `file`, or whose `file` the delta
  cannot place (a deleted path, a path outside the repository, a hand-built
  older record), is **inside**, and fails closed: it stays standing until a
  covering read closes it. A new finding must carry `file`; the fail-closed arm
  is for a historical `GateVisit` that lacks one.
- **Outside.** A finding outside the changed hunks of a changed file — or in a
  file the delta does not touch — is outside. It is carried, never closed.
- **Seat answers.** The seat answers a handed standing finding in exactly one of
  two ways: it re-reports it with the same key and a locator, and it still
  stands; or it omits it, and the engine reads absence. The seat may not declare
  a finding closed. "Closed with a reason" is therefore not a seat input: the
  closure is the engine's act on a read that covers the finding, and the covering
  read (whole, or the hunk) is the reason the engine journals.
- **Closing a follow-up.** A follow-up is a non-blocking finding carried in
  `follow_ups`. It closes only when a later visit's read covers its locator and
  the seat omits it: a whole read covers every locator; an inside-delta read
  covers its hunk; a delta that does not cover it carries it again.
- **The threshold.** An outside-delta finding at or above the phase table's
  return threshold forces a whole read; the table is the authority on the
  threshold (a medium that would return to implement today cannot pass as a
  follow-up). A below-threshold outside finding is carried as a follow-up and the
  PR names it. The threshold is read from the phase's own rules, not a hard-coded
  severity.
- **Security.** A previous finding with `security: true` is never a follow-up. If
  it still stands it is blocking, its position takes a whole read
  (`standing_security`), and `has_security_residual` must be `true`; if the whole
  read closes it, it is closed like any finding. If its site no longer exists or
  is not security-class, the whole phase takes a whole read. A finding the seat
  reports is blocking; the seat cannot demote one, and a seat-supplied
  `follow_ups` is refused.

The engine checks the declared aggregate against the blocking findings: a
`max_residual_severity` lower than the highest blocking finding, or
`has_security_residual` false while a blocking finding has `security: true`,
refuses the result; a non-empty `findings` with the matching aggregate input
absent also refuses, so a seat cannot omit the input it cannot understate.
`spec_defect` stays the seat's flag; no finding derives it. The PR names the
follow-ups; the review office is read-only, and every finding is a
test case with a name and a location. A delta read can still miss a finding no
one ever reported; that is the accepted cost of trusting a prior verdict, and why
a security-class position and a standing security finding never take the delta.
The engine also trusts the seat's `security` flag, as it trusts
`has_security_residual` today; a seat that labels a security finding false is not
caught by a rule, only by the security-class whole read and the review office.

**Binding:** `findings` and `follow_ups` share one definition and the key is
unique across both; the engine derives the follow-ups from the previous
`GateVisit` and the delta; the aggregate check is a typed refusal. Tests: a
finding outside the changed hunk of a changed file is carried; a fileless finding
stays standing; an outside finding at or above the threshold forces a whole read;
a seat-supplied `follow_ups` is refused; a seat cannot declare a closure; a
follow-up closes only by a covering read; the understated-severity refusal; the
false-security refusal; the standing-security block.

### 6. Older results, absent findings, and a clean prior

**Recommended: (a) a whole read, never a guessed key.**

A phase never visited is `first_visit`. A previous verdict with no `covered`, or
a v6/no-key result, is `unkeyed_result`. A previous verdict whose `findings` and
`follow_ups` are both empty — including a clean verdict — is `clean_prior`. All
three are whole reads. Absent `findings` and `[]` are the same: both are an empty
set, and neither proves what the prior read covered, so a clean verdict does not
license a delta on the next visit. The decision refuses to synthesize a finding
from `notes`, to treat an empty set as `clean` evidence, or to reuse an old
aggregate severity as a key. A run that never had a full read never takes a delta.

**No alternative offered:** a guessed key is identity from prose, and a clean
prior is the weakest possible evidence for a shortened read. Cost: three
triggers and a whole read after every clean verdict. **Binding:** the trigger
enum has `first_visit`, `unkeyed_result` and `clean_prior`; a test hands the
planner a v6-shaped result and asserts the full read and the exact trigger; a
test after a clean verdict asserts `clean_prior`; no path builds a `FindingKey`
without a site and an id.

### 7. What the verdict records as covered

**Recommended: (a) an engine-stamped `covered`, typed by the gate's kind: `head`
for a code gate, `artifacts` for a spec gate.**

- **`head`:** the commit the gate read, the base it compared against when one
  resolves, and whether the worktree was dirty at the start. For the bundle's
  protected phase, `covered.head` is the head the decide records in
  `reviewed_heads` (`crates/brokkr-runtime/src/engine/decide.rs:120-126`): the
  engine stamps the same `git_head` at the result append, and the decide writes
  it, so there is no third head record. The `phase/entered` head
  (`crates/brokkr-runtime/src/engine.rs:3086-3106`) is the start of the same
  interval, so the phase's own commits lie between the two. The test asserts
  `reviewed_heads` equals `covered.head` and that the `phase/entered` head is an
  ancestor. A gate in any
  other phase has no such record, so the engine stamps `covered.head` on its
  result at the end of the effect, and the stable-head check already refuses a
  single or all-gate panel that moved it
  (`crates/brokkr-runtime/src/engine.rs:942-968`, `:3219-3222`); a sequence's
  steps carry their own checks. The shipped gates that need
  the new record are every gate whose phase is not the protected one: the
  `verify` gate in fast, brokkr-fast, node, panel-review, preflight, self,
  release, triage, verify and gpt-flash; `ship` in fast; triage's and gpt-flash's
  `triage` gates; landing's `classify` gate; and the dialect gates (`specify`,
  `clarify`, `design`, `tasks`, `analyze`) in recipes whose protected phase is
  `review`. A bundle that makes
  `verify` protected (research) derives its verify head instead.
- **`artifacts`:** the declared artifact files and each file's SHA-256, taken
  before the gate runs. A spec gate's `reads` entry names its artifact files as
  repository-relative globs under the closed grammar of ruling 10 (the dialect's
  artifact *names* are prose to the engine; the entry is the typed path map). The
  engine records every matched regular file. A declared glob that matches nothing
  is a whole read (`artifact_missing`); a covered file that disappears before the
  next visit is a whole read too. The `covered` record itself refuses a file it
  cannot digest (0001).
- **`kind` is the policy's:** `artifacts` when the gate's `reads` entry names
  artifact globs; `head` otherwise. A dialect gate whose entry names no globs is a
  whole read (`artifact_missing`), never a head delta over untyped prose. That
  makes the spec and analyze gates artifacts gates and verify, review and ship
  code gates.
- **`base`** is the merge-base of HEAD and the default branch at commission time;
  when none resolves, `base` is `null` and a delta read is refused
  (`base_unknown`).
- **`dirty`** is the worktree's state at commission time; `worktree_dirty` fires
  on dirt then **or** on the previous visit's recorded dirty flag, because a
  commit-range delta would miss the uncommitted bytes the gate read.

**No alternative offered for the protected phase:** a third head record would
repeat a head the tree already journals, so the only choice is which existing
record is the covered one, and the decide-time `reviewed_heads` is the head the
gate judged. `reviewed_heads` and ship drift stay as they are; this is one more
reader of the same head, not a new head fact for the protected phase.

**Binding:** `covered` is a tagged enum, each arm with required fields and
`additionalProperties: false`; the engine stamps it before append and refuses a
seat-supplied one; tests pin both arms, the derived kind, the dirty refusal, the
unresolved-base refusal, the `phase/entered`/`reviewed_heads`/`covered` agreement
for the protected phase, and the new record for a non-protected gate.

### 8. Where the read plan lives

**Recommended: (a) the journal is the home: the plan is in the
`effect/requested` payload, and the digested seat input carries the same plan as a
projection pinned by the plan's digest.**

The earlier draft put the plan in the input and only a digest in the payload. That
is not buildable: the seat input is a pure function of (journal, pinned bundle,
feature) so recovery rebuilds it (`crates/brokkr-runtime/src/engine/input.rs:49-57`),
and a plan computed from the repository cannot be rebuilt from a digest alone;
recomputing it on recovery would compare a moved repository and park or shorten a
read that was never decided. The payload is journaled and is the one durable home.
At request time the engine computes the plan, appends it under `read` with
`read_digest = sha256(plan)` and `input_digest`, and builds the seat input's
`read` from the same plan. At append it checks both the payload and the input
against `read_digest`; a mismatch refuses. The seat reads the input's copy; the
view reads the payload; the engine decides from the payload. There is one home and
one derived, digest-pinned projection.

The `read` object names the **taken** kind separately from the **shadow** plan:
`taken` is `full` or `delta`, and `shadow` is the plan a delta would have used,
present only while the shadow slice still commissions a full read. A journal with
`taken: full` is never counted as a delta, however complete its shadow. The
readout counts `taken`; the shadow is a measurement, not a verdict.

- **(b) The input as the home.** It would need the plan in the journal to rebuild,
  which is (a); without it, recovery either loses the plan or recomputes it
  against a changed tree. Refused.

**Binding:** a typed `GateRead` in `brokkr-core` (no `Value` past the edge), a
gate-read v1 extension schema under `contracts/` (proposed), and a test that the
payload plan and the input projection cannot disagree: a mutated payload fails
`read_digest`, and a recovery rebuilt from the journal yields the same input. A
fold test rebuilds `GateVisit` from the journal alone. A new event is refused: it
needs an `event-envelope` version and moves every digest.

### 9. What the delta commission carries, and how the engine runs git

**Recommended: (a) the engine computes the range since the covered head, the
paths that still differ from the base, the unified diff for those paths, and the
standing findings; every diff read is hardened against repo-controlled drivers.**

The engine composes: (1) `since` is the previous covered revision; if it is not an
ancestor of HEAD, the read is full (`ancestor_lost`). (2) `base` is the current
merge-base of HEAD and the default branch; if none resolves, the read is full
(`base_unknown`). (3) The changed paths are `git diff --name-only since..HEAD`,
kept only where `git diff --name-only base...HEAD` also names the path; a path the
base touched and the branch still differs from is read whole — the since..HEAD
diff of that path — which over-reads a mixed file rather than under-reading it,
and a clean merge of the base leaves no path and owes no re-read. (4) The diff is
`git diff since..HEAD --` over those paths, with `-U3`, capped by
`delta_files_gt` and `delta_bytes_gt`; over either cap is a whole read. (5) The
commission carries the `read` plan, the diff text, and the standing findings with
their keys, locators and severities. A security-class site takes the whole read
instead (ruling 11). A spec gate follows ruling 12's file-level plan instead of
the git range.

The diff and name-only reads never run a repository-controlled program. The
engine has one `git_diff` helper, in the form the head reads already use
(`crates/brokkr-runtime/src/engine/worktree.rs:33-42`;
`crates/brokkr-runtime/src/engine.rs:3186-3200`), and it always runs:

```text
git -c core.fsmonitor=false --no-pager diff --no-ext-diff --no-textconv -U3 ...
git -c core.fsmonitor=false --no-pager diff --no-ext-diff --no-textconv --name-only ...
```

with a scrubbed environment: `GIT_CONFIG_NOSYSTEM=1`, `GIT_CONFIG_GLOBAL=/dev/null`,
`GIT_CONFIG_COUNT=0`, `GIT_EXTERNAL_DIFF` removed, and `-c core.fsmonitor=false`.
`--no-ext-diff` disables every external diff driver, both `diff=<driver>` in
`.gitattributes` and `diff.<driver>.command` in config; `--no-textconv` disables
every textconv; `--no-pager` and `core.fsmonitor=false` mean no repo config names
a program on this path. A planted `diff=` driver, a textconv and a
`diff.<driver>.command` in `.git/config` run nothing: the test writes a sentinel
that neither driver may touch. The engine's non-diff reads (rev-parse, merge-base,
cat-file) use the same scrubbed `git_query` form without the diff flags.

No model computes a delta or chooses which paths to omit. A dirty worktree is a
whole read before step 1. Whether a visit with an empty delta is skipped is
#1048's no-progress question: the gate still visits and its commission says no
files changed. A seat may run git for its own checks, but the engine never reads
the seat's diff as the delta; the commission's diff is the engine's.

**No alternative offered:** the engine must own the range and the diff, because a
seat-run `git diff` cannot prove what the seat read. Cost: one git helper, one
plan and one hardening test. **Binding:** a pure delta/range planner over a small
repository port, with the git calls at the runtime edge; the plan is journaled and
projected as ruling 8 says, so recovery reads it; tests pin the exact path set for
an ordinary fix, a base merge, an empty delta, a rewritten history, both caps,
and the planted driver/textconv run nothing.

### 10. The whole-read triggers and their home

**Recommended: (a) a top-level `reads` map in a `forge.phase-machine/v3` table,
one entry per gate phase, with a closed trigger list; an absent entry is a whole
read, a v2 table may not carry `reads`, and a v1 table never opts in.**

| Trigger | Fires when | Refuses |
|---|---|---|
| `first_visit` | the phase has no previous verdict in this run | a delta read for a first read |
| `unkeyed_result` | the previous verdict has no covered identity or no keyed findings | a guessed key |
| `clean_prior` | the previous verdict's findings and follow-ups are both empty | a delta on a clean word's evidence |
| `ancestor_lost` | the covered head is not an ancestor of the current head | a delta across rewritten history |
| `base_unknown` | no base resolves for the comparison | a delta with no base |
| `worktree_dirty` | the worktree is dirty at commission time or the previous visit recorded it dirty | a commit-only delta over uncommitted bytes |
| `artifact_missing` | a declared artifact glob matches no file, or a covered file is gone | a file delta over an absent file |
| `standing_security` | a previous finding with `security: true` still stands | carrying a security finding as a follow-up |
| `delta_files_gt` | the changed path count is over the declared N | an unbounded diff |
| `delta_bytes_gt` | the diff bytes are over the declared N | an unbounded input |
| `delta_touches` | any changed path matches a declared glob | a delta read over a protected surface |
| `operator_whole_read` | a journaled one-shot operator command names the phase | a recipe turning a whole read off |

`ancestor_lost` and `base_unknown` are head-gate triggers; an artifact gate
compares the covered digests and never asks for a base, with `artifact_missing`
as its counterpart. The entry names the phase's sites, each with its read policy,
and, for a spec gate, the artifact globs it covers. A site is named by its compiled structural
tag — the seat name for a single, `<member>` for a panel member, `<step>` or
`<step>:<member>` for a sequence — which the compiler resolves to exactly one
`SiteKey`; a tag that matches no site or more than one refuses at load. The tag
names a site, not a role (0107). A site the entry does not name is a whole read,
so a new gate site cannot accidentally inherit a delta. The journal records
every trigger that fired, in a canonical order; a delta read records none. The
glob grammar is closed: `*` matches any run except `/`, `**` matches any run
including `/`, `?` matches one character except `/`, every other character is
literal, matching is case-sensitive against a repository-relative path, and a
pattern containing `..` or an absolute path is refused. A pattern that matches
nothing is not a load error; for an artifact glob it is `artifact_missing`, and
for `delta_touches` it simply matches no path.

The home is a top-level `reads` map because the read is decided **before** the
effect, while `when` is evaluated **after** the result, at the transition. Putting
the read policy in `when` would give one fact two homes and ask a post-effect
condition to decide a pre-effect read. The map sits beside the `rules` array whose
`when` vocabulary it mirrors, and is matched with the same closed-enum rule: an
unknown trigger word refuses at load, and no wildcard arm decides. A per-rule
`read` beside `when` is refused because it repeats the plan on every result arm
and lets two arms disagree; a recipe-bundle field is refused because it is not the
phase table.

**No alternative offered:** the read is decided before the effect and the phase
table is the only pre-effect policy home; `when`, a per-rule `read` and a
recipe-bundle field are the refused alternatives above. A `forge.phase-machine/v3`
table may carry `reads`; a v2 table carrying it is
refused at load, and a v1 table is read as it is and never opts in. The frozen
production table `policy/phase-machine.json` is `forge.phase-machine/v1` and stays
a whole read on every gate. Each opting recipe's `policy.json` moves to v3; a
table without a `reads` entry for a gate phase is a whole read, so a recipe opts
in. A spec change re-arms only the phases whose covered artifacts or head changed:
the plan is per phase, and a phase whose covered identity is unchanged takes a
delta unless another trigger fires.

The operator's `whole_read` is a journaled `operator/commanded` command scoped to
a run and a phase, consumed by exactly the next visit; it is not a recipe flag and
not a model choice. `OperatorCommand` is a closed three-variant enum today
(`crates/brokkr-core/src/fold.rs:303-313`); the build adds a fourth variant,
`WholeRead`, with a phase argument, and the operator reader names both.

**Binding:** a phase-machine v3 schema under `contracts/` (proposed) that admits
`reads` and refuses it under v1 and v2; a closed `Trigger` enum with serde and the
loader's refusal; the glob matcher as a pure function in `brokkr-core`; tests for
every trigger, an unknown word, a v1 table read as it is, a v2 table carrying
`reads` refused, an absent `reads` is full, the analyze gate's delta and its
whole-read fallbacks, the spec re-arm rule, and the operator one-shot.

### 11. Which positions are security-class

**Recommended: (a) one derivation, from typed facts the compiler reads once.**

A site is security-class when at least one of these holds:

1. the site's **declared** result set includes `security-hold` — a single seat's
   or a sequence step's `results`. A panel member declares no result set of its
   own (`crates/brokkr-runtime/src/bundle/site_form.rs:425-444`), so this clause
   does not mark it; the seat's aggregate result set is not the member's;
2. it is the only gate site in its phase — a sole review position, structural
   from the compiled `SiteKey` (`crates/brokkr-runtime/src/bundle/site_key.rs:75-207`);
3. its hired agent declares the security office as typed data: `"security": true`
   in the agent definition, read once into the compiled site. The field is a new
   typed boolean the agent loader admits and an overlay may replace
   (`crates/brokkr-runtime/src/agents.rs:92-135`;
   `crates/brokkr-runtime/src/agents/load/overlay.rs:54-57`). The agent's file
   name, charter path, panel member key, step name and office string are never
   read for this class (0107 at
   `docs/decisions/0107-a-scoped-gate-grant.md:68` and `:111`).

A security-class site always takes a whole read, and the compile refuses a `reads`
entry that marks it `delta`. The policy may name more positions than the floor
requires, never fewer; a recipe cannot narrow the class. A phase all of whose gate
sites are security-class is a security-class gate.

The shipped effect: the single review seat of brokkr-fast, fast, node, preflight,
release, review-first, self, standby and verify declares `security-hold`, so it
is whole-read; triage's and gpt-flash's single `chore` review case and their final
`chief` step carry the seat's results and are whole-read too; every sole gate in a
non-review phase (verify's exec gate in fast, brokkr-fast, self, release,
panel-review and preflight; research's verify) is whole-read. In a mixed panel the
correctness, simplicity and spec-compliance members remain eligible for a delta;
the security member is held by its agent's typed declaration — `review-security`
and `gpt-flash-review-security` gain `"security": true` — not by its member key.
Self and release are whole-read on their single review seat for the same reason as
every other single review seat. Their seats must gain `has_security_residual` for
ruling 5's aggregate check to see a security finding: the inherited fast table
references only `max_residual_severity`, so the compile default supplies only that
one. Slice 0109d writes both inputs into brokkr-fast, fast, node, self and release.

**No alternative offered:** the three typed facts are the only ones the compiler
has; naming every position in policy every run makes a forgotten name silently
full-read, and a string match on a member key or a file name is the rule 0107
refused. The policy may widen the floor, never narrow it.

What an operator's own security agent must declare: `"security": true` in its
agent definition (or its site's result set must include `security-hold`, or it
must be the sole gate in its phase). A security slot hiring an agent without the
declaration is not held; the operator fixes the data, and the engine does not
infer from the key or the file name.

**Binding:** the compiler derives `security_class` from the three typed facts and
writes it into the compiled read policy; a compile test pins a mixed panel and a
dedicated security gate; a compile test refuses a delta mark on each of the three
shapes; a test pins that a member key or an agent file name does not decide; a
test pins the shipped whole-read set.

### 12. Spec review, grounding and analysis are file-level

**Recommended: (a) a changed artifact file is read whole, and an unchanged one is
not re-read. No section delta is built.**

The engine knows an artifact *file* only through the typed artifact globs in the
gate's `reads` entry; it knows a section only as the dialect's string.
`decisionLocation` (`contracts/dialect.v3.schema.json:33`) is where a loop phase
records its decisions, not a section map; the engine never parses a Markdown
heading. A spec gate's plan compares the SHA-256 of every file matched by its
artifact globs with the previous `covered.artifacts`; each changed file is read
whole, each unchanged file is not re-read, a new match is read whole, and a
missing file is a whole read. `analyze` (`recipes/triage/policy.json:36-40`) takes
the same plan as a spec review; the recipe's dialect still owns section markers,
the claims a section makes, and the split of grounding results into `unquoted`,
`noted` and `contradicted`. `#1089` is that split, in `#1035`'s pack; when it
lands it reads this decision's `covered` and `findings` for its `ground` phase.
This decision does not design it.

Per-section digests, with a closed extraction rule and a dialect version that
declares it, are a later option, **not built**: the engine would need the bytes
or the section map, and it has neither. A dialect that later declares a section
map can add them as a new version; this decision does not mint it.

A spec change re-arms only the phases whose covered artifacts or head changed. A
changed artifact file re-arms every phase whose globs match it; a phase whose
covered artifacts and head are unchanged is not re-armed and takes a delta unless
another trigger fires. The plan is per phase; no global re-arm exists.

**Binding:** the `covered.artifacts` arm records each matched file's digest; the
read plan carries the artifact glob set; tests pin a file-level delta, an
unchanged file not re-read, a new match read whole, the whole-file fallback for a
missing file, and the re-arm-only-changed-phases rule. Parsing Markdown headings
is refused because a heading is not a typed section; a seat naming sections is
refused because the engine could not compare two visits deterministically.

### 13. What the readouts show

**Recommended: (a) `analytics` separates full reads from delta reads and
`inspect` names each visit's read kind and triggers, both derived once in
`brokkr-view`.**

`analytics` gains a `reads` figure under `Rework`: full count, delta count and
trigger counts (the readout already has `sent_back` and `second_try_*` at
`crates/brokkr-view/src/analytics.rs:328-340`). `inspect` gains a read cell on the
participant row. The view already carries the visit number
(`crates/brokkr-view/src/lib.rs:394-399`), but `seats_block`
(`crates/brokkr-cli/src/render.rs:412-503`) does not paint it; the build paints
the visit number and the read cell together. A journal with no `read` record reads
`unrecorded`, never zero, so an older run is not counted as a delta read. The view
is the one derivation (0071 ruling 7); the CLI, the TUI and `ui.html` only paint
it, and no engine decision admits anything from the view.

**No alternative offered:** an `analytics`-only readout leaves one visit's read
indistinguishable from another's, and a per-surface derivation repeats the fact.
Cost: one view field group, one render cell and one snapshot move. **Binding:** a
view test pins the full/delta/trigger counts and the `unrecorded` case; a render
test pins the `inspect` line with the visit number and the read cell; a public-API
snapshot moves only for the added view fields.

## What 0034 says after acceptance

The current sentence in 0034 ruling 2 reads
(`docs/decisions/0034-the-seat-record.md:45-50`):

> The phase machine's separately governed `result`, `inputs`, and `notes` report
> remains inside a successful result; those fields do not widen the adjacent
> seat-record vocabulary.

After acceptance it reads:

> The phase machine's separately governed `result`, `inputs`, `notes` and `state`
> report remains inside a successful result, and a gate's result may also carry
> the separately governed `findings`, `follow_ups` and `covered` report that
> decision 0109 rules; those fields do not widen the adjacent seat-record
> vocabulary, and only seat-record v7 admits the three gate fields. v7 keeps
> `state` and the accounting fields.

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
  `first_visit`, `unkeyed_result` or `clean_prior`; there is no fallback that
  guesses.
- **No Windows code.** Decision 0063 keeps Linux and macOS as the hosts.
- **#1048(b) and its cost budget are out.** `awaiting_disposition` and the
  per-run cost budget may consume the key and the read facts; their outcome,
  fields and routing are #1048's.
- **#1089's recipe split is out.** It waits on #1035's pack and reads this
  decision's facts; the grounding split is not designed here.
- **No contract is minted and no code, test, guide, recipe or existing decision is
  edited.** The slices are the commission; the first moves the marker from
  `unbuilt`.

## The slices

Smallest first. The recommended first slice records the key and the covered
identity, and reads them back, with no routing change; the next field sweep can
then measure what a delta read would have saved before any visit is shortened.
Every test a ruling's binding names is in a slice's test column.

| Slice | What it builds | Files touched (rough) | Size | Contract changes | Tests it adds |
|---|---|---|---|---|---|
| 0109a | The finding key, the covered identity, the v7 result, the word binding, and the read-back; no routing change. | a new seat-record v7 schema under `contracts/` (a new version file beside v6, never an edit to v6); a new findings module under `crates/brokkr-core/src/`; `crates/brokkr-core/src/fold.rs`; `crates/brokkr-runtime/src/engine.rs`; `crates/brokkr-runtime/src/engine/decide.rs`; `crates/brokkr-runtime/src/bundle/aggregate.rs`; `crates/brokkr-protocol/src/adapters.rs`; a new `crates/brokkr-view/src/residual_findings.rs`; `crates/brokkr-cli/src/render.rs` | about 1,000 lines with tests | seat-record v7 (proposed, minted on acceptance) | v7 validation of `result`, `inputs`, `notes`, `state` and the accounting fields; a v6 record with a v7 field refused; a seat-supplied `covered`/`follow_ups` refused; engine-stamped `covered` on the head and artifacts arms; the derived kind; the protected `phase/entered`/`reviewed_heads`/`covered` agreement and the new record for a non-protected gate; a moved line, a reworded summary and a moved file; a duplicate id; a seat-supplied `site`; a panel's member findings joined with each member's site; a v6 result reads unkeyed; `clean` plus one high security finding refused, never shipped; `clean` plus a non-security blocking finding refused; `residual` plus a blocking finding admitted; `clean` plus an engine-carried `follow_ups` admitted; `inspect` prints the findings and covered |
| 0109b | The shadow delta: compute the range, paths, diff digest and standing set on every re-visit, journal the plan it would use, but still commission a full read; the git hardening; the per-visit record and `compare()`. | a new delta/set module under `crates/brokkr-core/src/`; `crates/brokkr-core/src/fold.rs`; `crates/brokkr-runtime/src/engine/input.rs`; `crates/brokkr-runtime/src/engine.rs`; `crates/brokkr-runtime/src/engine/worktree.rs`; `crates/brokkr-view/src/analytics.rs`; a new gate-read v1 schema under `contracts/` | about 550 lines with tests | gate-read v1 (proposed) | a first visit has no shadow; a second visit names the exact paths; a base merge removes base-only paths; an empty delta; a rewritten history; both caps; a planted `diff=` driver, a textconv and a `diff.<driver>.command` in `.git/config`, and the delta read runs neither; the payload plan and the input projection cannot disagree; a shadow-era journal never counts a full read as a delta; `GateVisit` records `findings` and `follow_ups`; `compare()` over the union; the potential saving in `analytics` |
| 0109c | The read policy and the triggers: a phase-machine v3 `reads` map, the closed trigger enum, the glob matcher, the security-class derivation and its refusal, the spec file-level plan, and a delta commission for one non-security gate (the analyze gate first); the charter, role and prompt text that names the read kind and the delta. | a new phase-machine v3 schema under `contracts/`; `crates/brokkr-core/src/policy.rs` and its condition module; a new glob module under `crates/brokkr-core/src/`; `crates/brokkr-runtime/src/engine/input.rs`; `crates/brokkr-runtime/src/engine.rs`; `crates/brokkr-runtime/src/bundle/` (the security-class derivation and the agent loader's `security` field); `crates/brokkr-runtime/src/agents.rs`; `crates/brokkr-runtime/src/agents/load/`; `agents/review-security.json`; `agents/gpt-flash-review-security.json`; `agents/charters/reviewer.md`; `agents/charters/review-security.md`; `recipes/preflight/roles/reviewer.md`; `crates/brokkr-protocol/src/adapters.rs`; `recipes/triage/policy.json` (moves to v3 with the analyze `reads` entry) | about 800 lines with tests | phase-machine v3 (proposed) | every trigger; an unknown trigger refused; a v1 table read as it is and never opted in; a v2 table carrying `reads` refused; an absent `reads` is full; the analyze gate's delta and its whole-read fallbacks; the spec file-level delta (a changed artifact file read whole, an unchanged one not re-read); the security-class derivation for the declared result set, the sole position and the typed agent office; a delta mark refused on each; a member key or agent file name does not decide; the shipped whole-read set; the glob matcher; the spec re-arm rule; the charter, role and prompt text binds the read kind and the delta, and a security-class charter keeps its whole read |
| 0109d | Review panels, standing security and follow-ups: the engine carries outside-delta findings, refuses an understated aggregate, whole-reads a security position, and gives every fast-family review seat both residual inputs. | `crates/brokkr-runtime/src/bundle/aggregate.rs`; `crates/brokkr-runtime/src/engine/decide.rs`; `crates/brokkr-runtime/src/engine/input.rs`; `crates/brokkr-core/src/fold.rs`; `recipes/brokkr-fast/bundle.json`; `recipes/fast/bundle.json`; `recipes/node/bundle.json`; `recipes/self/bundle.json`; `recipes/release/bundle.json` (both residual inputs on each review seat); `recipes/panel-review/policy.json`; `recipes/triage/policy.json`; `recipes/preflight/policy.json` (each moving to v3 and naming its read policy) | about 650 lines with tests | none new; v7 admits `follow_ups` from 0109a | an outside-hunk finding carried; a fileless finding standing; an outside finding at or above the threshold forces a whole read; a seat cannot declare a closure and a seat-supplied `follow_ups` is refused; a follow-up closes only by a covering read; a carried follow-up is not closed; a standing security finding blocks and forces full; the understated-severity refusal; the false-security refusal; a mixed panel's security member whole-reads and its correctness member delta-reads; a panel's follow-ups are named; self and release compile with both residual inputs; two consecutive delta visits, with a finding carried in the first still carried or closed by a covering read in the second |
| 0109e | The readouts and the operator one-shot: `analytics` full/delta/trigger counts, `inspect`'s read cell and visit number, `operator whole-read`. | `crates/brokkr-view/src/analytics.rs`; `crates/brokkr-view/src/lib.rs`; `crates/brokkr-cli/src/render.rs`; the operator verb under `crates/brokkr-cli/src/verbs/`; `crates/brokkr-core/src/fold.rs` (the fourth `OperatorCommand` variant and the phase argument) | about 450 lines with tests | none new | full/delta/trigger counts; `unrecorded` for a journal with no read record; the `inspect` line with the visit number and the read cell; the operator command is one-shot and phase-scoped; no operator command can widen a security refusal |

The order is buildable: a gives b the key and the covered identity, b gives c the
measurement and the planner, c gives d the policy and the plan, and d gives e the
verdict facts it paints. Each slice moves the decision's `Built:` marker and the
index cell only when its tests pass. The first slice is recommended because it
changes no routing: the next field sweep can compare the shadow delta's size with
the full read it still commissioned, the number the operator needs before opting a
recipe in.

**Contention:** `crates/brokkr-protocol/src/adapters.rs` is touched by 0109a (the
v7 result shape) and 0109c (the read kind and the delta in the prompt). The
slices are sequential, so the repair is one branch at a time; the second slice
builds its prompt edit on the first and the text test pins both sentences.

## Consequences

The saving is the adopter's figure, not a promise: if a review re-visit reads only
the fix diff and the standing findings, the 29% code-review share and the 55% spec
share have room to fall where the changes are small. The cost is a contract
version, a new read plan in every gate's request, a delta computation that runs
git in the engine, and triggers a recipe author must choose. The refusal that
keeps the saving honest is that a security-class position and a standing security
finding never take the delta, and a clean verdict never licenses one.

The effect is uneven by design. Every single review seat declares `security-hold`,
so it whole-reads, and the sole-gate clause also holds the `verify` and `ship`
exec gates, which carry no security word; that over-hold is the fail-closed cost
of a structural rule. The saving lands in the spec and analyze gates and in the
correctness, simplicity and spec-compliance members of a mixed panel, not in the
security member. That is the fail-closed shape the findings asked for: the floor
misses no seat that can hold for security.

The residual risks are named rather than hidden. A delta read trusts the prior
verdict: a finding no one reported is not in the standing set, and re-reading only
the delta will not find it; the whole-read triggers are the operator's bound on
that cost. A seat can reword a finding and give it a new id: the old key closes and
the new opens, so #1048(a) reads progress; the standing sets and the readout's
per-key diff make it visible, but the engine does not guess a fuzzy match. A dirty
worktree or an unresolved base forces a whole read, so some re-visits save nothing:
a delta that cannot prove its range is not a delta. The readout counts only
journals that carry the `read` record; older runs read `unrecorded`, never zero.

## Evidence

The inventory above was read at `94ada796d` for the base and `a20fdaa13` for the
design story; no journal was opened. The source lines are unchanged by the
documentation commits. The commands and their output that the claims rest on:

```text
git -C <checkout-of-94ada796d> ls-files | wc -l
2426
git -C <checkout-of-94ada796d> ls-files -z | xargs -0 wc -l | tail -1
1004496 total
git show --numstat --format= 94ada796d | awk '{a+=$1; d+=$2; f++} END {print f " files, " a " insertions, " d " deletions"}'
10 files, 755 insertions, 68 deletions
git diff --stat $(git merge-base a20fdaa13 origin/main)..a20fdaa13
 docs/decisions/0109-a-gate-s-re-visit-reads-what-changed.md | 568 +++++++++++++++++++++
 docs/decisions/README.md                                   |   1 +
 2 files changed, 569 insertions(+)
grep -rln '"class": "gate"' recipes/*/bundle.json
recipes/brokkr-fast/bundle.json
recipes/fast/bundle.json
recipes/gpt-flash/bundle.json
recipes/landing/bundle.json
recipes/node/bundle.json
recipes/panel-review/bundle.json
recipes/preflight/bundle.json
recipes/release/bundle.json
recipes/research/bundle.json
recipes/review-first/bundle.json
recipes/self/bundle.json
recipes/standby/bundle.json
recipes/triage/bundle.json
recipes/verify/bundle.json
grep -o '"extends": "[^"]*"' recipes/{night-shift,research-dsh,wager-harness,wager-harness-dsh,wager-harness-muse}/bundle.json
"extends": "triage"
"extends": "research"
"extends": "brokkr-fast"
"extends": "brokkr-fast"
"extends": "brokkr-fast"
grep -n 'REVIEW-CLEAN' policy/phase-machine.json
265:      "id": "REVIEW-CLEAN-UNVERIFIED",
274:      "id": "REVIEW-CLEAN-NO-FIXES",
285:      "id": "REVIEW-CLEAN",
grep -n 'has_security_residual' policy/phase-machine.json
224:        "has_security_residual": true
257:        "has_security_residual": true
grep -n 'has_security_residual\|max_residual_severity' recipes/fast/policy.json
116:        "max_residual_severity_above": "medium"
128:        "max_residual_severity_above": "low"
139:        "max_residual_severity_above": "none"
140:        "max_residual_severity_at_most": "low"
151:        "max_residual_severity_above": "low"
162:        "max_residual_severity_above": "none"
163:        "max_residual_severity_at_most": "low"
174:        "max_residual_severity_at_most": "none"
(no has_security_residual line in recipes/fast/policy.json)
grep -n '"inputs"' recipes/{brokkr-fast,fast,node,self,release}/bundle.json
(no output)
```

The base tree is 2,426 files and 1,004,496 lines; at the design commit the same
commands print 2,427 files and 1,005,065 lines, because the design story adds the
decision and the index row. The design branch's diff is the 2-file/569-insertion
read above; a final branch diff after this revision's own commit is unmeasured.
The recipe count is 14 declarations plus five inheriting overlays; the compiled
count is unmeasured. The fast table references only `max_residual_severity`, so
brokkr-fast, fast, node, self and release compile without `has_security_residual`
and slice 0109d adds both inputs. The result-file shape, the engine's journaling,
the two head records, the residual derivation, the four panel inputs, the policy
vocabulary, the dialect's artifact names and section string, and the readouts are
cited by file and line above. Everything not observed is marked unmeasured: in
particular, how many files a particular gate seat actually opened on a particular
re-visit, and the saving a delta would have produced in any real run. Those are
the first slice's measurement, not this document's claim.
