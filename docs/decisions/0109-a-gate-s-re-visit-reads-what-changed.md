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
other top-level key is admitted (`crates/brokkr-protocol/src/adapters.rs:478-491`).
The successful result is sealed under the seat-record contract, whose `result`
definition is closed and admits four report fields beside the accounting fields:
`result`, `inputs`, `notes` and the dialect step's `state`
(`contracts/seat-record.v6.schema.json:557-575`; `state` at `:573-574`;
0034 ruling 2 at `docs/decisions/0034-the-seat-record.md:45-50`). A v7 built from
three fields would refuse a dialect step's `state`, so v7 keeps it. The engine's
pre-decision check is narrower: a declared `result` string, and a declared
`change` input when the seat declares one
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
evidence and joins up to four inputs — `fixes_applied`, `spec_defect`,
`max_residual_severity` and `has_security_residual`; today it writes the two
residual inputs only when a member reports a residual or a rank above none
(`crates/brokkr-runtime/src/bundle/aggregate.rs:188-216`), and 0109a makes it
write both always, `none` and `false` on an all-clean panel. A member's
individual findings are not typed facts. That is enough for a threshold and a
boolean, not for #1048(a)'s set comparison.

A seat resolves its inputs from its hired agent first, then its own `inputs`, then
the phase's referenced set (`crates/brokkr-runtime/src/bundle/assemble.rs:303-347`);
`inputs` beside `agent` is refused (`crates/brokkr-runtime/src/bundle/site_form.rs:251-273`).
The fast review table references `max_residual_severity` but **not**
`has_security_residual` (`recipes/fast/policy.json:103-186`), yet the agents the
fast family hires already declare both: `agents/fast-reviewer.json:3`,
`agents/brokkr-fast-reviewer.json:5` and `agents/node-reviewer.json:4`. Fast,
brokkr-fast and node therefore compile both inputs. Verify's review hires
`verify-reviewer`, which declares both (`agents/verify-reviewer.json:6`), and
verify's own table references both (`recipes/verify/policy.json:45,54`). Self and
release hire `brokkr-reviewer`, which extends `reviewer` and declares no `inputs`
(`agents/brokkr-reviewer.json:1-10`; `agents/reviewer.json:1-17`); their inherited
fast table references `max_residual_severity` only, so those two seats compile
`max_residual_severity` only. Slice 0109d adds both inputs to
`agents/brokkr-reviewer.json` alone, one home (0071 ruling 5); self and release
inherit them. Review-first, standby, preflight, panel-review, triage and
gpt-flash already declare both at the seat or its agent.

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

### 2. A verdict's word is bound to its findings and its declared aggregate

**Recommended: (a) refuse a malformed verdict — never rewrite the seat's word.**

A gate result is judged against two independent bodies of evidence: the findings
it reports and the aggregate it declares. Either can refuse a passing word;
neither may rewrite it. A work-class seat's result is not a verdict and is
outside this check, except for a member of a review panel: a review member's word
is joined into the aggregate that becomes the gate's word or the gate's input, and
can lift or hide it, so a review-panel member is bound whatever its class and
whatever the panel's class. Any other work result is outside.

**The passing word.** The engine holds one closed enum of non-passing words:
`residual`, `drift`, `ambiguous`, `clean-unverified`, `fail`, `security-hold`,
`upstream` and `escalate`. Every other result word is passing. The enum is engine
data, not a reading of the phase table, so a recipe cannot add a `when` that drops
the aggregate check, and a word the enum does not name is checked as passing
rather than exempted. In the shipped tables `clean`, `pass`, `consistent`,
`clear`, `ready` and `shipped` are passing. The check applies to a gate-class
seat, to the engine-written panel aggregate, and to every member of a review
panel whatever its class.

An operator's own non-passing word is recipe data the engine cannot know: it is
checked as passing. A blocking finding or a declared residual still refuses it;
a word with neither is admitted as today. An operator who needs a new non-passing
word adds it to this enum by a decision amendment; the cost is that until then
the word is only as strong as its findings and its aggregate. `upstream` and `escalate` are shipped
non-passing words and are in the enum for that reason. `upstream` is the
**work-class author's** step result in the specify, design and tasks phases
(`recipes/triage/bundle.json:77`, `:114`, `:129`), not a gate's word; the enum
holds it so that a gate that ever returns it is not read as passing. `escalate`
is triage's.

**The findings arm.** A gate result whose `findings` array is non-empty is bound
as follows. A finding is blocking when its `severity` is above `low`, or when its
`security` is true.

- any finding with `security: true` requires the word `security-hold`;
- any other blocking finding refuses a passing word; `residual` and
  `clean-unverified` are admitted, and `security-hold` is admitted as a stricter
  word;
- `follow_ups` are non-blocking and do not change the word.

A gate whose declared result set does not include `security-hold` and whose seat
reports a `security: true` finding has no lawful word: the result is refused as
malformed and the run parks. The engine does not substitute the phase's security
position or rewrite the finding. The member charters that forbid `security-hold`
(the correctness and spec-compliance members) therefore report a security smell
in `notes` at its honest severity; the security-class member owns the typed hold.

**The aggregate arm.** A passing word is refused when the declared aggregate says
a residual stands:

- `max_residual_severity` is above `none`;
- `has_security_residual` is true;
- a **residual** input is absent from the result although the seat's compiled
  input set includes it (a seat that declares both must write both; a seat that
  declares one must write that one). The arm covers only the residual pair,
  `max_residual_severity` and `has_security_residual`, the two "both" names. The
  seat's other inputs (`spec_defect`, `fixes_applied`, `change`) are not
  aggregate inputs: an absent one reads as today and is the chief's or the
  aggregate's to write, never a refusal of this arm.

The severity is the closed `none < info < low < medium < high < critical`
vocabulary; an absent, null or unknown value where the seat declares the input
refuses rather than reads as `none`.

The check runs on both a member's result and the seat's result: the seat is a
single seat or the engine-written panel aggregate, and each member of a review
panel is checked before its word is joined. A member declares no input set of its
own (`crates/brokkr-runtime/src/bundle/site_form.rs:425-428`), so a member is held
to **the residual pair only**, whichever of the two the owning seat's compiled
input set includes, and the seat's other inputs are the chief's: triage's review
seat compiles `spec_defect` too (`recipes/triage/bundle.json:34-35`), and a
member that writes only the pair, as every member charter orders
(`agents/charters/review-correctness.md:16-18`, `review-security.md:16-18`,
`review-adversarial.md:19-21`), is not refused for omitting it. The aggregate is
held to the same pair, and the chief's `spec_defect` is written or absent as it
is today. 0109a amends those three member charters so a clean writes
`max_residual_severity: none` and `has_security_residual: false`
(`review-spec-compliance.md:15-19` already says a clean uses `none`), and
`agents/charters/review-chief.md` needs the same `none`/`false` line at `:102`
and no `spec_defect` amendment, because it already carries `spec_defect`
(`review-chief.md:18-19`, `:98`) and `spec_defect` is outside the pair. 0109a makes the engine's panel aggregate write
`max_residual_severity` and `has_security_residual` on every result, `none` and
`false` when no member reports a residual (`crates/brokkr-runtime/src/bundle/aggregate.rs:188-216`
writes them today only when a member does), so an all-clean panel and a
charter-shaped clean are admitted and the absent-input clause refuses only a
result that omits a residual input its own compiled set declares. A member's refusal parks the
panel before its aggregate rules; the member's word never reaches the aggregate.

The aggregate arm is beside the findings arm: either refuses the word. A seat
that declares no residual inputs is not refused by this arm; its word stands as
today, and if it is security-class, ruling 11 already keeps it on a whole read.
The check lives in the engine's verdict check beside the result-schema check
(`crates/brokkr-runtime/src/engine/decide.rs:31-46`, `:239-261`), not in a phase
table's `when`, so no recipe can drop it: a missing `when` on `REVIEW-CLEAN`
(`recipes/fast/policy.json:180-184`; `policy/phase-machine.json:285-289`) cannot
let a clean word over a declared residual ship.

The engine refuses the result as malformed before the transition: it journals the
same `__schema-invalid__` `transition/decided` the existing schema check writes
(`crates/brokkr-runtime/src/engine/decide.rs:31-46`) and the run parks. The result
never reaches a rule. The engine does not derive the word from the findings or the
aggregate: the word is the seat's typed verdict, and rewriting it would make the
engine a second author, hide a seat that claims clean over its own evidence, and
leave no journaled refusal to repair. A refusal is the fail-closed cost.

- **(b) Derive the word.** A security finding would force `security-hold` and a
  non-security residual a lower word. Refused for the reason above: it silently
  replaces a seat's claim and makes the engine's rewrite the record.

**Binding:** the check sits with the result schema check
(`crates/brokkr-runtime/src/engine/decide.rs:239-261`) and is a typed function over
the parsed findings and the compiled input set, not prose. Tests: `clean` plus a
declared high residual and no finding is refused; `clean` plus
`has_security_residual: true` is refused; `clean` that omits an aggregate input
the seat declares is refused; `clean` from a seat that declares no residual inputs
is admitted; `clean` plus one high security finding is refused, never shipped;
`clean` plus a medium non-security finding is refused; `security-hold` plus a high
security finding is admitted; `residual` plus a medium finding is admitted;
`clean` plus an engine-carried `follow_ups` entry is admitted; `consistent` plus a
blocking finding is refused; a correctness member's `security: true` finding is
refused and parks because the member has no `security-hold`; an all-clean panel is
admitted; a charter-shaped clean (`clean` with `none` and `false`) is admitted; a
**triage-shaped all-clean panel**, built from the member charters' own cleans
(correctness, security, adversarial and spec-compliance members each writing only
the residual pair, over a seat that also compiles `spec_defect`), is admitted; a
member of a review panel that writes `clean` over a declared high residual is
refused whatever its class; a member that omits a residual input is refused, and
one that omits only `spec_defect` is not; the panel aggregate writes both inputs
on an all-clean panel.

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
an id outside the pattern, and a seat-supplied `site`.

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
  closes a standing finding whose hunk was not read and makes #1048(a) read false
  progress. Refused.

- **Inside.** A finding is inside the delta when its `file` is a changed path and
  its `line` falls in a changed hunk of the `since..HEAD` diff for that path,
  widened by the diff's context (the plan uses `-U3`, so three lines). The engine
  records the hunks it read; it does not guess.
- **Outside.** A finding outside the changed hunks of a changed file — or in a
  file the delta does not touch — is outside. It is carried, never closed.
- **Unplaceable.** A finding with no `file`, or whose `file` the delta cannot
  place (a deleted path, a path outside the repository, a hand-built older
  record), is a third case: neither inside nor outside. It is blocking and fails
  closed: it stays standing and is not closed by an inside-delta read's omission;
  only a whole read closes it. A new finding must carry `file`; the fail-closed
  arm is for a historical `GateVisit` that lacks one.
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
  return threshold is the whole-read trigger `outside_threshold` (ruling 10). The
  threshold is the lowest severity at which any rule from the phase with a
  `max_residual_severity_above` condition returns to an earlier phase (not the
  same phase, not a forward phase, not `stop` or a park), over the closed order
  `none < info < low < medium < high < critical`: an `above: low` return puts the
  threshold at `medium`. If no such return rule exists, no threshold derives, and
  every blocking outside-delta finding — one above `low`, or one with
  `security: true` — forces a whole read (`outside_threshold`). An outside finding
  that is not blocking is carried as a follow-up and the PR names it; the threshold
  is read from the phase's own rules, not a hard-coded severity. This arm is built in
  the slice that first commissions a delta (0109c), not later with the panels: a
  delta must arrive with the carry rule or a standing finding outside it would be
  neither carried nor closed.
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
stays standing and an inside-delta read's omission does not close it; an outside
finding at or above the derived threshold forces a whole read, with
`outside_threshold` journaled; where no threshold derives, a blocking outside
finding forces a whole read; a non-empty `findings` with the aggregate input
absent is refused; a seat-supplied `follow_ups` is refused; a seat cannot declare a closure; a
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
  resolves, and whether the worktree was dirty at the start. The arm has two field
  sets. A gate with its own head record writes
  `{kind: "head", head: <commit id>, base: <id|null>, dirty: bool}`. The bundle's
  protected phase writes `{kind: "head", head_digest: <sha256 of the head
  string>, base: <id|null>, dirty: bool}`: its `covered.head` is a digest-pinned
  projection of the decide-time head, not a third head authority. `head` and
  `head_digest` are mutually exclusive and exactly one is required. The engine
  reads the protected phase's head value from the decide-time `reviewed_heads`
  (`crates/brokkr-runtime/src/engine/decide.rs:120-126`);
  the decide-time head is the authority. The two reads are equal by construction
  under the gate's stable-head check: the effect's start and end heads must agree
  (`crates/brokkr-runtime/src/engine.rs:942-968`, `:3219-3222`), the decide-time
  read is the end read, and the append-time projection is pinned to that same end
  read by its digest; a moved head parks before a verdict, and a projection whose
  digest does not match the decide-time head refuses at decide. The `phase/entered`
  head (`crates/brokkr-runtime/src/engine.rs:3086-3106`) is the start of the same
  interval, so the phase's own commits lie between the two. The test asserts
  `reviewed_heads` equals the projected head and that the `phase/entered` head is
  an ancestor. A gate in any other phase has no such record, so the engine stamps
  `covered.head` on its result at the end of the effect, and the stable-head check
  already refuses a single or all-gate panel that moved it
  (`crates/brokkr-runtime/src/engine.rs:942-968`, `:3219-3222`); a sequence's
  steps carry their own checks. The shipped gates that need the new record are
  every gate whose phase is not the protected one. The list is illustrative, not
  a table: the `verify` and `ship` gates in the fast family, triage's and
  gpt-flash's `triage` gates, landing's `classify` gate, and the dialect gates
  (`specify`, `clarify`, `design`, `tasks`, `analyze`) in recipes whose protected
  phase is `review`. The compiler derives the set from
  the compiled sites; a bundle that makes `verify` protected (research) derives
  its verify head instead.
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
- **`dirty`** is `true` unless the tree check (ruling 9) proved the worktree
  clean at commission time; a difference, a failure and a timeout are `true`.
  `worktree_dirty` fires on that **or** on the previous visit's recorded dirty
  flag, because a commit-range delta would miss the uncommitted bytes the gate
  read.

**No alternative offered for the protected phase:** the decide-time
`reviewed_heads` is the head the gate judged, and `covered` is its digest-pinned
projection, so the protected phase gains a pin and no second head authority.
`reviewed_heads` and ship drift stay as they are; this is one more reader of the
same head, not a new head fact for the protected phase.

**Binding:** `covered` is a tagged enum, each arm with required fields and
`additionalProperties: false`; the engine stamps it before append and refuses a
seat-supplied one; tests pin both arms and both `head` field sets, the derived
kind, the dirty refusal, the unresolved-base refusal, the
`phase/entered`/`reviewed_heads`/`covered` agreement for the protected phase, the
digest-mismatch refusal at decide (0109a), and the new record for a non-protected
gate.

### 8. Where the read plan lives

**Recommended: (a) the journal is the home: the plan is in the
`effect/requested` payload, and the digested seat input carries the same plan as a
projection pinned by the plan's digest.**

The plan cannot live only in the input: the seat input is a pure function of
(journal, pinned bundle, feature) so recovery rebuilds it
(`crates/brokkr-runtime/src/engine/input.rs:49-57`), and a plan computed from the
repository cannot be rebuilt from a digest alone; recomputing it on recovery
would compare a moved repository and park or shorten a read that was never
decided. The payload is journaled and is the one durable home. At request time
the engine computes the plan, appends it under `read` with
`read_digest = sha256(plan)` and `input_digest`, and builds the seat input's `read`
from the same plan. At append it checks both the payload and the input against
`read_digest`; a mismatch refuses. The seat reads the input's copy; the view reads
the payload; the engine decides from the payload. There is one home and one
derived, digest-pinned projection.

The diff text is the per-path patches of ruling 9 concatenated in the changed
set's byte order, and the digest is over that text. It follows the taken kind. A **taken delta** journals the capped diff
text in the `read` payload and carries it in the seat input, so recovery reads the
bytes that were decided, never a fresh `git diff`. A **shadow** read (`taken:
full`) journals the shadow plan and the diff's digest and byte count, not its
text: the measurement needs the size, and the text would grow the journal on every
re-visit with no cap while no seat reads it. The shadow's cost is therefore a plan
and a digest, and the cap `delta_bytes_gt` is enforced on the text only when a
delta is taken (0109c).

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
fold test rebuilds `GateVisit` from the journal alone. A shadow journal carries
the plan and the diff digest but no diff text; a taken delta's text is journaled,
capped, and recovery reads it rather than a fresh `git diff`. A new event is
refused: it needs an `event-envelope` version and moves every digest.

### 9. What the delta commission carries, and how the engine runs git

**Recommended: (a) the engine computes the range since the covered head, the
raw set of paths that still differ from the base, one patch per path, and the
standing findings; the read is complete or it is whole; and nothing the read
uses can be written by a seat or named by the repository.**

Three rules hold this ruling, each stated once below and each an ask of its own:
the **isolation rule** (what the read runs in), the **completeness rule** (what
the read must carry) and the **tree-check rule** (how the engine knows the
worktree is the head it read). The class they close is not one more vector: it
is "an input the read uses was written by a seat or the repository", and "the
read came back short without failing". Rounds r2 to r4 closed vectors one flag
at a time; each rule below is one sentence that holds by construction, plus a
check that fails closed.

The engine composes: (1) `since` is the previous covered revision; if it is not an
ancestor of HEAD, the read is full (`ancestor_lost`). (2) `base` is the current
merge-base of HEAD and the default branch; if none resolves, the read is full
(`base_unknown`). (3) The changed set is the `since..HEAD` raw set, read with
`-z` and kept as raw bytes with each entry's two modes, kept only where the
`base...HEAD` raw set names the same bytes; a path the base touched and the
branch still differs from is read whole — the since..HEAD patch of that path —
which over-reads a mixed file rather than under-reading it, and a clean merge of
the base leaves no path and owes no re-read. (4) The patch is read **one path at
a time**: one `diff-tree -p` per path in the set, with `-U3`, each naming exactly
that one path as a literal pathspec. `delta_files_gt` is checked against the
set's size before any patch runs, and `delta_bytes_gt` against the bytes read,
which are bounded: a command whose output passes the remaining budget is stopped
and the read is whole. (5) The completeness check (below) runs over the set and
the patches. (6) The commission carries the `read` plan, the diff text, and the
standing findings with their keys, locators and severities. A security-class site
takes the whole read instead (ruling 11). A spec gate follows ruling 12's
file-level plan instead of the git range.

A path is a byte string, never a `String`: a path with a newline, a quote or a
byte outside UTF-8 is an ordinary path. The plan carries each path as bytes in
the gate-read schema (an array of lowercase-hex byte strings), the view derives a
display form and nothing reads that form back (0071 ruling 3).

**The isolation rule.** *No seat and no repository can write the scratch, and the
engine reads it only as it made it.* Every git command of the delta read and of
the tree check runs in a scratch git directory the engine makes at the read, from
an environment the engine constructs, reaching the repository's objects only
through `objects/info/alternates`, given commit ids rather than refs, and a
missing object fails the read closed. The sentence is the build's test, and it
is true by construction rather than by a list of flags:

- **Where it lives.** Not under the run-local tree, and not in the worktree. The
  run-local directory is `<workdir>/.forge` or the folder's `state/`
  (`crates/brokkr-runtime/src/engine.rs:1021-1025`), and its `scratch/` is the
  **seat's own** place, which seats write by design (`SEAT_SCRATCH`,
  `engine.rs:3167-3169`, "each attempt's seat scratch tree"; `engine.rs:1021-1022`
  places the run's results, scratch and secrets there under 0083 ruling 3); r4 put the scratch there, and a seat that can
  write `.forge/results/` can plant `.forge/scratch/<name>/config` first. The
  scratch lives under the engine's own state root on the host,
  `<host HOME>/.local/state/brokkr/reads/`, a sibling of the protected root the
  capability broker already keeps at `.local/state/brokkr/capabilities`
  (`crates/brokkr-runtime/src/engine/broker.rs:95`). It is reached by the same
  protected walk, which judges every ancestor owner-only and opens each name
  through the directory handle before it (`broker.rs:680-705`;
  `broker/root.rs:1-8`), so a scratch is never opened by a path a seat can swap.
  A host with no resolvable HOME has no such root, and every read is whole
  (`read_failed`).
- **When it is made.** At request time, **after the work seat's attempt is proven
  over**: the attempt settled, its process group gone, its descendants killed and
  reaped, nothing of it holding the driver's pipes; an attempt that cannot show
  that parks as `Unsettled` and never reaches a request
  (`crates/brokkr-protocol/src/process/tree.rs:205-233`;
  `crates/brokkr-protocol/src/process/attempts.rs:535-575`). The scratch does not
  exist while any seat of the run runs: the read makes it, uses it and removes it
  before this gate's own seats launch.
- **How it is made.** A fresh directory named from 128 bits of the operating
  system's random source (never the read digest: that is a function of facts the journal
  records, so a name made from it could be predicted), made with `mkdirat` mode 0700 exclusively, so a name that
  already exists is refused and never reused. A boxed seat cannot reach it: the
  namespace box's writable set is the workspace and the box's own tmp and home,
  and the rest of the host is read-only to it (dsh's profile must begin
  `--ro-bind / /`: `crates/brokkr-protocol/src/dsh_sandbox.rs:149-152`; the
  claude and codex boxes' profile is **unmeasured** here), so a write to the state
  root fails, and a read of it shows a path to objects the seat can already read.
  An unboxed seat has no fence, being a process of the same user, and is kept out
  by time and by name instead: it is over before the directory exists, and a name
  drawn from the random source at the read is not a name it could have planted.
- **What is in it.** Two directories made together under one random parent, each
  holding only what the engine writes: the **read scratch** holds `HEAD`, the
  constant `ref: refs/heads/main`, which names no commit (the operated
  repository's HEAD is never copied into it); an **empty `refs/` directory**,
  because git refuses a directory without one (observed below); and
  `objects/info/alternates`, one line naming the operated repository's object
  directory, resolved from the worktree's `.git` (a directory, or a file naming
  the git dir) and that git dir's `commondir` when it names one: a path read,
  never a config read. A planted `.git` or `commondir` can only point at another
  object directory, which supplies or withholds objects by the ids the engine
  names, never a program. The **tree scratch** (the tree-check rule) has the same
  three and gains one `index`, written by the engine's own `read-tree`; the index
  is never in the read scratch, because git falls back to the index for
  attributes (observed: the operated `.git` with an unrelated directory as cwd
  still printed "Binary files" for a `*.txt -diff` its index held), and a covered
  head's committed `.gitattributes` would then apply to the patch. Neither has a `config`, a `hooks/`, an `info/attributes`, a
  `.gitattributes`, a `packed-refs`, a `shallow`, a `refs/replace` or a graft file,
  so no local configuration, no hook, no attribute source, no replacement and no
  shallow boundary of the operated repository is in view;
  `GIT_CONFIG_NOSYSTEM=1`, `GIT_CONFIG_GLOBAL=/dev/null` and `GIT_CONFIG_COUNT=0`
  leave no system or global configuration.
- **The engine reads it only as it made it.** Before any git command opens a
  scratch the engine lists it through its directory handle, without following a
  link, and compares the listing with what it wrote: the entry names, their kinds,
  and the bytes of `HEAD` and `alternates`. Any other entry (a `config`, an
  `info/attributes`, a `.gitattributes`, a `hooks/`, a symlink, a second
  `objects/` entry) is the whole-read trigger `scratch_foreign`, and no git
  command starts in it. The same check runs on the tree scratch with its `index`
  absent until the engine's own `read-tree` makes it.
- **The cwd.** Every child's working directory is its scratch, never the operated
  worktree and never a path a seat can name. Git takes the working directory as
  the worktree when the directory it is given is not bare; a child run with the
  operated worktree as its cwd reads that worktree's `.gitattributes`, and a
  committed `*.txt -diff` turned the patch into "Binary files ... differ"
  (observed below).
- **What it costs.** Two directories, five small files and one listing per read;
  both are removed by a guard after the read, on success or failure. A crash
  leaves the random parent under the engine's own root, where nothing reads it
  again, since a name is never reused; sweeping leftovers belongs to whatever the
  engine already sweeps at start (unmeasured here, so a follow-up).
- **What it does not defend.** A concurrent process of the same user could write
  the scratch between the listing and the read, or the operated repository's
  objects, or the journal. That is outside this decision's threat: the same
  process could already write the operated worktree, and every transition rests
  on the worktree and the journal as they stand. The rule closes the seat (which
  is over before the scratch exists, or cannot write the host) and the repository
  (which names programs only through a config, an attribute, a hook or a remote,
  none of which the scratch holds).
- **The command.** The engine has a new `git_diff` helper at the runtime edge
  (`git_query`, below, is the other). It runs plumbing, not porcelain, with the ids, and the child's working
  directory is the read scratch. The raw set is read with no pathspec, so the
  pathless form is the one intended; the patch is read once per path, never from
  a list:

  ```text
  git --git-dir=<scratch> -c core.fsmonitor=false -c core.attributesFile=/dev/null \
      --no-pager diff-tree -r -z --raw --no-renames --no-commit-id <since> <head>
  git --git-dir=<scratch> -c core.fsmonitor=false -c core.attributesFile=/dev/null \
      --no-pager diff-tree -r -z --raw --no-renames --no-commit-id <base> <head>
  git --git-dir=<scratch> -c core.fsmonitor=false -c core.attributesFile=/dev/null \
      --no-pager diff-tree -r -p -U3 --no-renames --no-ext-diff --no-textconv \
      --ignore-submodules=all --no-commit-id <since> <head> -- <one path>
  ```

  The second line is step 3's `base...HEAD` side: `<base>` is the merge-base id
  `git_query` printed, and the comparison is a two-id `diff-tree`, not a
  three-dot range. The child environment is constructed, not inherited:
  `GIT_DIR` is the scratch, `PATH` is the engine's, then
  `GIT_CONFIG_NOSYSTEM=1`, `GIT_CONFIG_GLOBAL=/dev/null`, `GIT_CONFIG_COUNT=0`,
  `GIT_ATTR_NOSYSTEM=1`, `GIT_NO_LAZY_FETCH=1` and `GIT_LITERAL_PATHSPECS=1`. No
  other `GIT_*` variable reaches it, so `GIT_EXTERNAL_DIFF`, `GIT_CONFIG_KEY_*`,
  `GIT_SSH_COMMAND` and `GIT_ASKPASS` cannot be planted from outside.
  `GIT_LITERAL_PATHSPECS` (git 1.8.2) turns pathspec magic off, so a path named
  `:(exclude)*`, `:lead.txt` or `:(top)x` is a file name and nothing else; `--`
  ends options and does not end pathspec magic. `--no-renames` makes a rename a
  delete and an add, each in the set and each patched. `--no-ext-diff`,
  `--no-textconv`, `--ignore-submodules=all`, `-c core.fsmonitor=false`,
  `-c core.attributesFile=/dev/null` and `GIT_NO_LAZY_FETCH=1` are belts over the
  construction, not the construction itself: the lazy-fetch variable and the
  `--no-lazy-fetch` option are version-dependent and nothing rests on them.
  **No attribute source.** The patch reads no `.gitattributes`: the cwd is the
  scratch, which holds none; the scratch has no index and no `info/attributes`;
  `core.attributesFile` and the system file name nothing; and its `HEAD` names no
  commit, so neither `attr.tree` nor a bare repository's default tree is a source.
  `--attr-source` (git 2.40) is not used on the read: it names an empty tree where
  it exists and is an unknown option, a failure, where it does not, and a read
  that cannot run on an older supported git is no delta, not a safer one. The
  tree check, whose input is the operated worktree's own `.gitattributes`, does
  need it, and an older git there fails closed (the tree-check rule).
- **The object reads.** The new scrubbed `git_query` helper runs `merge-base` and
  `cat-file -e` in the same scratch with the same environment and ids. A missing
  object makes the command fail; the read is whole and nothing is fetched. The
  engine resolves `HEAD` and the default-branch tip to ids before the read:
  `HEAD` through the effect's existing stable-head check
  (`crates/brokkr-runtime/src/engine.rs:942-968`, `:3219-3222`), the default branch
  through the bundle's default-branch name. That ref read is a ref-to-id
  observation, not part of the delta read; it is not widened here, and a moved
  head parks before a verdict.
- **No refs.** Every revision argument is a full lowercase hex object id from the
  engine's own read; the helper refuses anything that is not one before it starts
  git, so a ref name and an option-shaped revision are data it never passes. A
  path travels as one literal pathspec after `--`, by the environment above. The
  scratch's constant `HEAD` is the only ref file and names nothing, so no ref of
  the operated repository, no `refs/replace/` and no `attr.tree` is resolved in
  the read.
- **The alternates chain.** A repository may itself carry
  `objects/info/alternates`; the scratch follows that chain to more objects and
  nothing else. A chain target's config, attributes and hooks are not read, and a
  chain that cannot supply an object fails the read closed.

**The completeness rule.** *A delta read carries the text of every changed path,
or the gate reads whole.* The patch is read for each path of the raw set, and the
read is a delta only if all of these hold; otherwise it is whole, with the
trigger `delta_incomplete` (or `delta_files_gt`/`delta_bytes_gt` for a cap):

1. every path of the set has its own patch, and none was skipped;
2. each patch holds exactly one `diff --git ` header line and at least one
   `@@ ` hunk line, and no `Binary files ` and no `GIT binary patch` line (a
   content line starts with a space, `+` or `-`, so these markers at column 0 are
   git's own, whatever the file holds);
3. the headers across all patches number exactly the set: a patch that names a
   path outside the set, or two paths for one, fails the check, and a literal
   pathspec naming a path that is a file on one side and a directory on the other
   is such a case;
4. no entry of the set has the gitlink mode `160000` on either side: a submodule
   pointer is a commit id in another repository, and the commission cannot
   carry its text.

The check does not parse a path out of a patch header (git quotes it), and the
set is never reconstructed from patch text: the set is the `-z` bytes, the patch
is attributed to its path by the one literal pathspec that produced it. A path
with no hunk fails the check by rule 2, which therefore also sends these to a
whole read, none of them a bug: a mode-only change, an empty added file, a
binary file (a NUL in the blob, git's size threshold, or an attribute), a
gitlink. A deleted file and a symlink have a hunk (the deleted lines, the link
target) and pass. The cost is a whole read where a delta is possible; that is the
fail-closed side, and a recipe that wants a mode change read does not need the
delta. `delta_incomplete` and its cause are journaled with the plan.

**An empty set runs no patch.** The set is empty after step 3 (a clean merge of
the base, or an empty `since..HEAD`). The loop of step 4 then has no iteration:
no `diff-tree -p` starts, because `diff-tree -p <since> <head> --` with no paths
reads the whole range, not an empty delta (observed below). The helper takes one
path, never a list, so an empty list cannot be written. The plan is a delta of
zero paths with an empty diff text and a recorded digest of the empty text, and
the commission says no files changed. Whether a visit with an empty delta is
skipped is #1048's no-progress question; the gate still visits.

**The tree-check rule.** *The engine knows the worktree is the head it read by
comparing bytes in a scratch it made, never by asking the repository.* The
observation that decides `worktree_dirty` is not `git status`
(`crates/brokkr-runtime/src/engine/worktree.rs:17-28`, `:45-47`), which runs in
the operated repository with that repository's configuration and attributes: a
`*.txt filter=evil` in `.gitattributes` and a `filter.evil.clean` that prints the
index blob made `git status --porcelain` print nothing for a content-dirty
worktree, and a `core.fsmonitor` hook that answers "nothing changed" does the
same (both observed below). A dirty trigger that can be silenced by the
repository is no trigger. The new `tree_proven_clean` (beside `read_tree`; the
0093 carry and ship decisions keep `read_tree` and `git_dirty` as they are) runs
in the **tree scratch**, with the operated worktree named only as `--work-tree`
and the child's cwd the scratch:

```text
git --git-dir=<tree scratch> --work-tree=<worktree> read-tree <head>
git --git-dir=<tree scratch> --work-tree=<worktree> --attr-source=<empty tree> \
    -c core.fsmonitor=false -c core.excludesFile=/dev/null \
    update-index --ignore-submodules --refresh
git ... --attr-source=<empty tree> diff-files --ignore-submodules=all --raw -z
git ... --attr-source=<empty tree> ls-files -o --exclude-standard -z
```

The index is built from the head the read is about, so the scratch's own
configuration is empty, its attribute source is the empty tree (`--attr-source`,
git 2.40: the worktree's `.gitattributes` is then not the source, observed for the
patch and for the `filter=` case below; whether `text`, `eol`, `ident` and
`working-tree-encoding` are ignored by `update-index` too is **unmeasured**, and
0109b's test plants each), no filter or fsmonitor is defined, and a file's bytes
are meant to be compared to the committed blob's, unconverted. The worktree is
proven clean only if `update-index --refresh` exits 0, `diff-files` prints
nothing and `ls-files -o` prints nothing. **Anything else is dirty:** a
difference, a non-zero exit, a timeout, a git that cannot start, a git older than
2.40 (which refuses `--attr-source`), a SHA-256 repository (the scratch has no
`extensions.objectformat`), an unreadable worktree. Only the proof is clean. That
differs from `git_dirty` today, where a refused `git status` with no output reads
as not dirty (`worktree.rs:23-26`, `:45-47`); this ruling does not change
`git_dirty`.

*What it costs.* It hashes every tracked file once: the scratch index is new, so
there is no stat cache to skip a file by. On this tree (2,427 tracked files) the
`update-index --refresh` took 0.229 s wall on git 2.51.0 with a warm cache; a
cold cache or a far larger tree is **unmeasured**, and the cost grows with the
tracked bytes. *What it can get wrong, and the direction.* It can only say dirty
where `git status` says clean, never the reverse, because it is built to apply no
conversion (the test that plants `ident`, `eol` and a lying filter holds it to
that): a legitimate clean filter (an LFS pointer in the blob and the large
file in the worktree), a CRLF checkout, or an `ident` expansion compares unequal
and the worktree reads dirty. An LFS or CRLF adopter therefore takes a whole read
every time, which is no delta and not a wrong one. Untracked files are read as
`git status` reads them, by the worktree's own `.gitignore`; an ignore rule can
hide an untracked file from the check exactly as it hides it from `git status`
today, and an untracked file is not a byte of any commit, so no commit-range
delta could have carried it. A gitlink entry is skipped (`--ignore-submodules`):
its worktree is another repository, whose own configuration would run in a child
process, which is r4's submodule case; a changed gitlink is a whole read by
rule 4 above.

No model computes a delta or chooses which paths to omit. A dirty worktree is a
whole read before step 1. A seat may run git for its own checks, but the engine
never reads the seat's diff as the delta; the commission's diff is the engine's.
The read's other inputs are the engine's own: `since` is the journal's `covered`,
`head` is the stable-head check's read, `base` is a `merge-base` over those ids.
The journal is what every transition rests on, and a writer that could alter it
could already forge a verdict, so it is outside this ruling's threat as it is
outside 0034's.

**What was observed** (git 2.51.0, scratch repositories under `/tmp`, this round;
each result is quoted as run, and the rest of the Evidence names the commands):

- the scratch, in the stated form with an **empty `refs/` directory**, runs; with
  `HEAD` and `objects/info/alternates` alone, `git --git-dir=<it> diff-tree ...`
  exits 128, `fatal: not a git repository`;
- a file named `:(exclude)*` is listed by `-z --raw` and `--name-only`; the patch
  with `-- ':(exclude)*'` is **0 bytes**, and the same with
  `--literal-pathspecs` is the text hunk; `-- ':lead.txt'` is 0 bytes (the
  leading colon is read as magic and dropped); a file named `line\nbreak.txt`
  is printed by `--name-only` as `"line\nbreak.txt"`, and that quoted string as a
  pathspec matches nothing (0 bytes), while `-z --raw` prints the bytes;
- with the operated worktree as cwd and its committed `*.txt -diff`, the stated
  patch is `Binary files a/a.txt and b/a.txt differ`; with the scratch as cwd it
  is the text hunk; with the operated worktree as cwd and `--attr-source` naming
  the empty tree it is also the text hunk, and with `--text` it is too;
- a scratch whose `HEAD` names the covered commit (detached), whose tree carries
  `*.txt -diff`, prints the text hunk both with `-c core.bare=true` and without:
  on 2.51.0 a bare repository's default tree is not read by `diff-tree`; with
  `-c attr.tree=<covered commit>` the same read prints "Binary files": a planted
  scratch config can blank the text through `attr.tree`;
- `diff-tree -p <since> <head> --` with no path prints every changed path;
- a planted scratch `config` with `extensions.partialClone`,
  `remote.origin.promisor`, `remote.origin.url` and
  `remote.origin.uploadpack = <script>`, and one blob missing, ran the script
  (sentinel present) under the stated read without `--no-lazy-fetch`, and did not
  with it (exit 128, "could not fetch ... from promisor remote"); a scratch the
  engine made (no config) exits 128, "unable to read <blob>", and the script did
  not run;
- `git status --porcelain` in an operated repository with `*.txt filter=evil` and
  `filter.evil.clean` printing the index blob, over a content-dirty file, printed
  nothing and the filter ran (sentinel present); the same with a
  `core.fsmonitor` hook that answers "nothing changed" printed nothing while
  `-c core.fsmonitor=false status --porcelain` printed ` M a.txt`; the tree check
  in the tree scratch reported `a.txt: needs update` (exit 1) and listed `a.txt`
  in `diff-files --raw`, with neither the filter nor the hook started.

**No alternative offered:** the engine must own the range and the diff, because a
seat-run `git diff` cannot prove what the seat read. A flag list leaves the
sources in place and asks each new flag to close the last path found open; the
scratch is the simpler form because it removes the sources rather than filtering
them, and a scratch under a tree a seat writes is not removal (r4's placement).
The completeness check is not a flag either: it is what makes a short read
impossible whichever vector shortened it. Cost: two git helpers and one tree
check, two scratch directories per read, one patch process per changed path (at
most `delta_files_gt`, 50 by default), one plan, and the planted-sentinel matrix.

**Binding:** a pure delta/range planner over a small repository port, with the
git calls at the runtime edge; the plan is journaled and projected as ruling 8
says, so recovery reads it; paths are bytes end to end. Tests, in 0109b's column
(each read for `git_diff` and `git_query` alike, each plant paired with a control
that runs the command that would fire it without the rule, so every sentinel is
shown live):

- *The scratch is the engine's.* A scratch path planted **before** the read with
  a promisor `config` (`extensions.partialClone`, `remote.origin.promisor`,
  `uploadpack` naming a script), a `diff.<driver>.command` and an
  `info/attributes`: the read is whole (`scratch_foreign`), no git command starts
  in it, and no sentinel fires; the control runs the stated `diff-tree` in the
  planted scratch with the exactness check removed, and the uploadpack sentinel
  fires. A name that already exists is refused (`mkdirat` fails). The scratch is
  never under the run-local tree or the worktree. Its shape is pinned: no
  `config`, `HEAD` names no commit, an empty `refs/` directory, one `alternates`
  line; a ref-shaped revision is refused; a positive control prints the changed
  path and its text.
- *The read is complete or whole.* A file named `:(exclude)*`, one named with a
  newline, one with a double quote, one with a leading `:`, and a committed
  `-diff` attribute and a `binary` attribute, **each with the cwd inside and
  outside the operated worktree**: in every case the changed file's text is in
  the delta, or the read is whole. A positive control shows each case blanks or
  shortens the patch without the rule (no literal pathspecs; the name parsed from
  `--name-only`; the cwd the operated worktree; the attribute source left in). A
  mode-only change, an empty added file, a binary file, a gitlink, a symlink, a
  deleted file and a rename each give the exact verdict above; an empty changed
  set runs no `diff-tree -p` (a spy on the helper) and yields a delta of zero
  paths; a path the patch names outside the set, and a path with no hunk, give
  `delta_incomplete`; both caps with explicit `ReadLimits` values, and a patch
  over the byte budget stops the process and reads whole.
- *The repository names no program.* For `git_diff` and `git_query` alike, each
  planted sentinel stays absent: a `diff=` driver, a textconv, a
  `diff.<driver>.command`, the submodule `diff.submodule=diff` case with the
  submodule's own `diff.external`, the promisor case with `extensions.partialClone`
  and `remote.<n>.uploadpack` (fail-closed: non-zero, the object still missing,
  a whole read), `filter.<driver>.clean` and `.process`, `core.fsmonitor`,
  `core.hooksPath`, an `info/attributes` driver, an environment-carried
  `GIT_EXTERNAL_DIFF` and `GIT_CONFIG_*`, and an alternates chain whose target
  names a driver. The controls are the unhardened diff, `git add`, `git status`,
  `git commit` and the operated-repo `cat-file`.
- *The tree check is not the repository's.* The lying clean filter (a
  `filter.evil.clean` that prints the index blob, over a content-dirty file) and
  an fsmonitor hook that reports nothing changed: each makes the read whole
  (`worktree_dirty`) with the filter and the hook never started, and a control
  shows `git status --porcelain` printing nothing for each. A clean worktree is
  proven clean (positive control); `ident`, `eol` and a CRLF checkout read dirty;
  a git that refuses `--attr-source`, a timeout and a refused git each read dirty.

### 10. The whole-read triggers and their home

**Recommended: (a) a top-level `reads` map in a `forge.phase-machine/v3` table,
one entry per gate phase, with a closed trigger list; an absent entry is a whole
read, a v2 table may not carry `reads`, and a v1 table never opts in.**

| Trigger | Fires when | Refuses |
|---|---|---|
| `first_visit` | the phase has no previous verdict in this run | a delta read for a first read |
| `unkeyed_result` | the previous verdict has no covered identity | a guessed key |
| `clean_prior` | the previous verdict's findings and follow-ups are both empty | a delta on a clean word's evidence |
| `ancestor_lost` | the covered head is not an ancestor of the current head | a delta across rewritten history |
| `base_unknown` | no base resolves for the comparison | a delta with no base |
| `worktree_dirty` | the tree check (ruling 9) does not prove the worktree clean at commission time (a difference, a failure or a timeout) or the previous visit recorded it dirty | a commit-only delta over uncommitted bytes |
| `scratch_foreign` | a scratch holds anything the engine did not write | a read in a directory a seat or the repository shaped |
| `read_failed` | a git command failed, timed out or found an object missing, no HOME resolves, or an id is not a commit id | a delta from a read that did not complete |
| `delta_incomplete` | a changed path has no text hunk, a patch names a path outside the set, or a changed path is a gitlink | a delta that is shorter than the change |
| `artifact_missing` | a declared artifact glob matches no file, or a covered file is gone | a file delta over an absent file |
| `standing_security` | a previous finding with `security: true` still stands | carrying a security finding as a follow-up |
| `outside_threshold` | a carried outside-delta finding is at or above the phase's return threshold, or is blocking where no threshold derives | carrying a return-worthy finding as a follow-up |
| `delta_files_gt` | the changed path count is over the declared N | an unbounded diff |
| `delta_bytes_gt` | the diff bytes are over the declared N | an unbounded input |
| `delta_touches` | any changed path matches a declared glob | a delta read over a protected surface |
| `operator_whole_read` | a journaled one-shot operator command names the phase | a recipe turning a whole read off |

`ancestor_lost` and `base_unknown` are head-gate triggers; an artifact gate
compares the covered digests and never asks for a base, with `artifact_missing`
as its counterpart. `outside_threshold` reads ruling 5's derivation from the
phase's own return rules; it is not a hard-coded severity. The entry names the
phase's sites, each with its read policy, and, for a spec gate, the artifact
globs it covers. A site is named by its compiled structural tag. An unselected
body uses the seat name for a single site, `<member>` for a panel member,
`<step>` or `<step>:<member>` for a sequence. A body under a `select` case
prefixes the case, because the same member name repeats in several cases:
`<case>` for a single, `<case>:<member>` for a panel member, `<case>:<step>` or
`<case>:<step>:<member>` for a sequence. The compiler resolves the tag within the
phase to exactly one `SiteKey`; a tag that matches no site or more than one
refuses at load. The tag names a site, not a role (0107). A site the entry does
not name is a whole read, so a new gate site cannot accidentally inherit a delta.
The journal records every trigger that fired, in a canonical order; a delta read
records none. The glob grammar is closed: `*` matches any run except `/`, `**`
matches any run including `/`, `?` matches one character except `/`, every other
character is literal, matching is case-sensitive against a repository-relative
path, and a pattern containing `..` or an absolute path is refused. A pattern
that matches nothing is not a load error; for an artifact glob it is
`artifact_missing`, and for `delta_touches` it simply matches no path.

**The entry's shape.** A `reads` entry is closed data:

```json
"reads": {
  "analyze": {
    "sites": {
      "judge": {
        "read": "delta",
        "delta_files_gt": 50,
        "delta_bytes_gt": 262144,
        "delta_touches": ["contracts/**", "policy/**"],
        "artifacts": ["specs/**"]
      }
    }
  }
}
```

The example names `judge`, the analyst, the delta-eligible site of triage's
analyze sequence (`recipes/triage/bundle.json:133-141`: the dialect `check`
script, then the `judge`). The script `check` is **not named**, so by the
absent-site rule it is a whole read; it reads no diff either way.

`reads` maps a phase name to an entry; an entry has only `sites`, a map from the
tag grammar above to a site policy; both maps refuse an unknown key and an unknown
phase or tag. A site policy has `read`, required and exactly `delta` or `whole`.
A `whole` site policy carries no other key. A `delta` site policy may carry
`delta_files_gt` and `delta_bytes_gt` (positive integers), `delta_touches` (globs)
and, only on a spec gate, `artifacts` (globs); an unknown key refuses. The planner
takes a `ReadLimits { files, bytes }` parameter whose defaults are 50 files and
262144 bytes; the entry overrides them, and 0109b tests the planner with explicit
values before 0109c exposes them in the schema.

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
a whole read on every gate. Each opting recipe's `policy.json` moves to v3, and a
recipe whose own table composes over an opted-in table moves its table to v3 in
the same slice: `merge_table` refuses layers with different schemas
(`crates/brokkr-runtime/src/bundle/compose.rs:817-838`), so a v3 layer over a v2
base, or the reverse, refuses at load. A table without a `reads` entry for a gate
phase is a whole read, so a recipe opts in one phase at a time. The spec gate's
plan and its re-arm rule are ruling 12's; they are not repeated here.

The operator's `whole_read` is a journaled `operator/commanded` command scoped to
a run and a phase, consumed by exactly the next visit; it is not a recipe flag and
not a model choice. `OperatorCommand` is a closed three-variant enum today
(`crates/brokkr-core/src/fold.rs:303-313`); the build adds a fourth variant,
`WholeRead`, with a phase argument, and the operator reader names both.

**Binding:** a phase-machine v3 schema under `contracts/` (proposed) that admits
`reads` and refuses it under v1 and v2; a closed `Trigger` enum with serde and the
loader's refusal; the glob matcher as a pure function in `brokkr-core`; tests for
every trigger, including `outside_threshold` and its derived threshold; an
unknown word; a v1 table read as it is; a v2 table carrying `reads` refused; an
absent `reads` is full; the analyze gate's delta and its whole-read fallbacks;
the spec re-arm rule; a `select`-case tag resolves to exactly one site and a bare
member tag that matches more than one refuses; and the operator one-shot.

### 11. Which positions are security-class

**Recommended: (a) one derivation, from typed facts the compiler reads once.**

A site is security-class when at least one of these holds:

1. **Its hired agent declares the security office as typed data.** `"security": true`
   in the agent definition, read once into the compiled site. The field is a new
   typed boolean the agent loader admits. An overlay inherits it and may set it,
   never unset it: `security` joins `OVERLAY_KEYS`
   (`crates/brokkr-runtime/src/agents/load/overlay.rs:54-57`), an overlay that
   omits it inherits the base value, an overlay that writes `false` over a `true`
   base is refused at load, and an overlay may add `true` where the base has none.
   The agent's file name, charter path, panel member key, step name and office
   string are never read for this class (0107 at
   `docs/decisions/0107-a-scoped-gate-grant.md:68` and `:111`).
2. **Its declared result set permits `security-hold`.** Clause 2 reads an
   executing site's own `results` or, for a panel member, its hired agent's typed
   `results` list. A single seat, or a sequence step with its own `agent`,
   executes; a panel, sequence or select container does not, and its `results` are
   the aggregate's or the case's, so clause 2 is never read on a container. That
   keeps triage's and gpt-flash's `positions` steps — panel containers whose
   results include `security-hold` — from whole-reading the very members ruling 2
   binds; their members are classified one by one. A panel member declares no
   result set of its own (`crates/brokkr-runtime/src/bundle/site_form.rs:425-444`),
   so its agent declares the permission as a typed `results` list in its
   definition, and the compiler reads that list. The charter is prose and is never
   parsed. `agents/review-adversarial.json` gains `results: ["clean", "residual",
   "security-hold"]` because its charter permits the hold
   (`agents/charters/review-adversarial.md:19-22`);
   `agents/review-security.json` gains the same three words, because its charter
   permits the hold (`agents/charters/review-security.md:16-20`) and
   `security: true` alone makes it security-class but does not make its own hold a
   lawful result. `gpt-flash-review-adversarial` and
   `gpt-flash-review-security` inherit both fields through `extends`.
   The list binds. A panel member's compiled result set is its agent's list, and
   the engine refuses a member result outside that set as malformed, so the
   security member's `security-hold` is lawful and its high finding is admitted.
   **Every member agent declares its list in 0109c**, because a list that binds
   must exist for every member it binds: `agents/review-correctness.json` and
   `agents/review-spec-compliance.json`, which declare none today, gain
   `results: ["clean", "residual"]` (their charters forbid a hold:
   `agents/charters/review-correctness.md:19-20`, `review-spec-compliance.md:11-12`);
   the security and adversarial agents' lists are the three words above.
   **A member whose agent declares no list is held to `["clean", "residual"]`**,
   the narrowest set that still lets it pass or return, never to the owning seat's
   set (which in triage and gpt-flash includes `security-hold`) and never to
   "unchanged": a future member agent cannot fall through to a wider permission,
   and a hold from a list-less member is refused, so the panel parks loudly
   instead of shipping a hold nothing declared. The cost is that an operator's
   own list-less security member loses its lawful hold until it declares the list;
   the panel rule below then whole-reads that panel, which is the fail-closed side.
   A single seat compiles its own bundle `results`; the loader refuses a seat
   result its hired agent's list does not admit, so the agent list is a ceiling,
   never a floor, and a seat that hires an agent declaring no list is unchanged.
   The `results` list is inherited through `extends` and is not an overlay key, so
   an overlay cannot remove a hold from it; an overlay that needs to add the class
   writes `security: true`. The member's sites are
   `recipes/triage/bundle.json:61` (class `gate`, so the delta-eligible adversarial
   member) and `recipes/gpt-flash/bundle.json:187-188` (class `work`: only
   gpt-flash's `chief` step is a gate, so that member takes no delta; its word is
   still bound as a panel member, below). This is the clause that holds a member
   whose charter lets it hold but whose definition does not claim the security
   office.
3. **It is the sole REVIEW position of its selected body.** The role is
   structural, the one 0107 derives from the `SiteKey`
   (`crates/brokkr-runtime/src/bundle/site_key.rs:75-207`): the selected body has
   exactly one gate-class site, and that site is a `review` position — a single
   `review` seat, a one-member `review` panel, or a first-step gate site of a
   `review` sequence with no earlier step. A multi-member panel is not one site, so
   its members are not class by this clause. A sole gate of a spec, analyze or
   clarify phase is **not** security-class by this clause. A `verify` or `ship` gate
   is not a review position and is not class by this clause. The analyze judge is
   not a review position; in any event the dialect `check` step compiles as a gate
   site (`crates/brokkr-runtime/src/bundle/sequence.rs:290-291`), so the judge is
   not the body's only gate site. It remains delta-eligible.

A security-class site always takes a whole read, and the compile refuses a `reads`
entry that marks it `delta`. The class is never narrowed by a recipe or overlay: a
recipe may widen the floor, never narrow it, and no overlay can clear `security` or
remove `security-hold` from a declared result set.

A panel whose phase can end in `security-hold` must hold at least one
security-class member, or every member whole-reads. "Can end in" is declared data:
the gate seat's `results` include `security-hold` (a panel step's `results` are
its aggregate and are read here, not by clause 2), or the panel's aggregate result
set does. This closes the undeclared security slot: a panel with no
security-class member does not compile a delta; every member whole-reads. A
security member hiring an agent that declares neither `security: true` nor
`security-hold`, with no other class member in the panel, is therefore whole-read.
The operator fixes the data; the engine never infers from the member key, the file
name or the charter prose.

A work-class site is not a verdict and takes no delta of its own. A work-class
member of a review panel is the exception ruling 2 states: its word is joined into
the review aggregate, so it is bound by ruling 2 whatever its class, but it is
not a gate site and cannot be named in a `reads` entry. A `reads` tag that
resolves to a work site refuses at load: there is no verdict of its own for a
delta to shorten.

The shipped effect is illustrative, because the compiler derives it. The single
review seat of fast, brokkr-fast, node, preflight, release, review-first, self,
standby, triage's `chore` case and verify declares `security-hold`, so it is
whole-read; their overlays inherit the seat. In a mixed panel the correctness and
spec-compliance members remain eligible for a delta; the security member is held
by its agent's `security: true` (`review-security` gains it; the
`gpt-flash-review-security` agent inherits through `extends`), and the
adversarial member by its agent's declared `results`. The final `chief` step of
triage's and gpt-flash's design and engine cases carries the seat's
`security-hold` result set and is whole-read. gpt-flash's review `positions`
members are all class `work`; only its `chief` step is a gate, so those members
take no delta of their own, and their words are bound as panel members by ruling
2. Triage's `positions` step is a panel container and is not class as a
container; its security and adversarial members are class, and its correctness
and spec-compliance members are not. The `verify` and `ship` exec gates are not
class by clause 3, and no `reads` entry opts them in, so **the absent-entry rule
(ruling 10) is why they stay whole-read**. Were they opted in, an exec gate
reports no findings, so ruling 6 would make every re-visit `clean_prior` and the
delta would still not arrive; that is a second reason, not the first, and
`clean_prior` is the shadow plan's trigger for them, never the reason a
commission is whole. Defining an exec gate's findings and its delta was the alternative, and is refused:
the gate's cost is the script it runs, not a read of the tree, and making every
script emit a typed findings array would add a contract for a read nothing
shortens. Self and release are whole-read on their single review seat by clauses 2 and 3.
Only their agent, `agents/brokkr-reviewer.json`, lacks both residual inputs today;
slice 0109d adds them there alone, one home (0071 ruling 5), and self and release
inherit them. The fast, brokkr-fast, node and verify review agents already declare
both (`agents/fast-reviewer.json:3`, `agents/brokkr-fast-reviewer.json:5`,
`agents/node-reviewer.json:4`, `agents/verify-reviewer.json:6`).

**No alternative offered:** the three typed facts are the only ones the compiler
has; naming every position in policy every run makes a forgotten name silently
full-read, and a string match on a member key or a file name is the rule 0107
refused. The policy may widen the floor, never narrow it.

What an operator's own security agent must declare: `"security": true` in its
agent definition, or a declared result set that includes `security-hold`, or it
must be the sole review position of its phase. A security slot hiring an agent
with none of these is not held; a panel in that state whole-reads, and the engine
does not infer from the key or the file name.

**Binding:** the compiler derives `security_class` from the three typed facts and
writes it into the compiled read policy; a compile test pins a mixed panel and a
dedicated security gate; a compile test refuses a delta mark on each of the three
shapes; a test pins that a member key or an agent file name does not decide; a
test pins the shipped whole-read set; a panel whose phase can end in
`security-hold` with an undeclared security slot and no other class member
whole-reads; the adversarial
member is security-class through its declared results; the security member's
`security-hold` plus a high security finding is admitted and its
`security: true` alone would not admit it; **compile and run: a correctness
member's and a spec-compliance member's `clean` is admitted, and a member whose
agent declares no list is held to `clean` and `residual` (its `security-hold` is
refused)**; a member result outside its agent's
declared `results` is refused, and a single seat result its agent does not admit
is refused; an overlay cannot clear `security` and an overlay can set it; the
analyze judge is delta-eligible; triage's design and engine `positions` steps are
not class as containers, while their security and adversarial members are class
and their correctness and spec-compliance members are not; gpt-flash compiles its
work-class review members under ruling 2's word binding and names no delta for
them.

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
`noted` and `contradicted`. The analyze judge's charter
(`agents/charters/analyst.md:3-4`) orders a whole read today, though analyze is
the first gate 0109c makes delta-eligible: 0109c amends it to name the read kind
and the delta, and binds the text. Clarify is a spec gate, but no `reads` entry
opts it in in 0109c, so `agents/charters/clarifier.md` keeps its whole read until
a later slice names the clarify gate; the slice that opts it in amends that
charter with it. `#1089` is that split, in `#1035`'s pack; when it
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
  and the delta; the policy table routes the typed result. The `security-hold`
  stop and the residual rules stand; the new verdict check only adds refusals.
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
identity, adds the word refusal, and reads them back, with no delta routing
change; the next field sweep can then measure what a delta read would have saved
before any visit is shortened. Every test a ruling's binding names is in a slice's
test column.

| Slice | What it builds | Files touched (rough) | Size | Contract changes | Tests it adds |
|---|---|---|---|---|---|
| 0109a | The finding key, the covered head, the v7 result, the word binding to findings and the declared aggregate, and the read-back; no routing change. | a new seat-record v7 schema under `contracts/` (a new version file beside v6, never an edit to v6); a new findings module under `crates/brokkr-core/src/`; `crates/brokkr-core/src/fold.rs`; `crates/brokkr-runtime/src/engine.rs`; `crates/brokkr-runtime/src/engine/decide.rs`; `crates/brokkr-runtime/src/bundle/aggregate.rs`; `crates/brokkr-protocol/src/adapters.rs`; a new `crates/brokkr-view/src/residual_findings.rs`; `crates/brokkr-cli/src/render.rs`; `agents/charters/reviewer.md`; `agents/charters/verify-reviewer.md`; `agents/charters/node-reviewer.md`; `agents/charters/review-chief.md`; `agents/charters/review-correctness.md`; `agents/charters/review-security.md`; `agents/charters/review-adversarial.md`; `recipes/preflight/roles/reviewer.md`; `recipes/review-first/roles/reviewer.md`; `recipes/standby/roles/reviewer.md` (each clean names `none` and `false`; 0109c edits the correctness charter again, for the read kind, on top of this slice) | about 1,150 lines with tests | seat-record v7 (proposed, minted on acceptance) | v7 closes the result and the finding/covered records with `additionalProperties: false` and `$defs`, and admits `result`, `inputs`, `notes`, `state`, the accounting fields and the three gate report fields; a v6 record with a v7 field refused; v7 variants and the refusal text; a seat-supplied `covered`/`follow_ups` refused; engine-stamped `covered` on the head arm and both `head` field sets; the protected `phase/entered`/`reviewed_heads`/`covered` digest agreement and the digest-mismatch refusal at decide; the new record for a non-protected gate; a moved line, a reworded summary and a moved file; a duplicate id; an id outside the pattern; a seat-supplied `site`; a panel's member findings joined with each member's site; a v6 result reads unkeyed; no path builds a `FindingKey` without a site and an id; the closed non-passing enum marks `residual`, `drift`, `ambiguous`, `clean-unverified`, `fail`, `security-hold`, `upstream` and `escalate` non-passing, `clean`, `pass`, `consistent`, `clear`, `ready` and `shipped` passing, and an unnamed word (an operator's own) passing while its findings still refuse it; `clean` plus a declared high residual and no finding refused; `clean` plus `has_security_residual: true` refused; `clean` omitting a residual input its seat declares refused, and omitting only `spec_defect` admitted; an absent or unknown aggregate severity refused; `clean` from a seat that declares no residual inputs admitted; `clean` plus one high security finding refused, never shipped; `clean` plus a non-security blocking finding refused; `security-hold` plus a high security finding admitted; `residual` plus a blocking finding admitted; `clean` plus an engine-carried `follow_ups` admitted; `consistent` plus a blocking finding refused; a correctness member's `security: true` finding refused and parks; the panel aggregate writes both inputs on an all-clean panel; an all-clean panel admitted; a charter-shaped clean (`clean`, `none`, `false`) admitted; a triage-shaped all-clean panel, built from the member charters' own cleans over a seat that also compiles `spec_defect`, admitted; a work-class panel member's `clean` over a declared high residual refused; a member that omits a residual input refused and one that omits only `spec_defect` admitted; `inspect` prints the findings and covered |
| 0109b | The shadow delta: compute the range, paths, diff digest, diff size and standing set on every re-visit, journal the plan and digest it would use, but still commission a full read; the engine-made scratch under the engine's state root, the completeness check, the tree check and the planted matrix; the per-visit record and `compare()`. | a new delta/set module under `crates/brokkr-core/src/`; a new `crates/brokkr-runtime/src/engine/delta_git.rs` (the `git_diff`/`git_query` helpers and the completeness check); a new `crates/brokkr-runtime/src/engine/delta_scratch.rs` (make, list, compare and remove the scratch); `crates/brokkr-runtime/src/engine/broker.rs` (the protected walk takes the reads root's names beside `PROTECTED_ROOT`); `crates/brokkr-core/src/fold.rs`; `crates/brokkr-runtime/src/engine/input.rs`; `crates/brokkr-runtime/src/engine.rs`; `crates/brokkr-runtime/src/engine/worktree.rs`; `crates/brokkr-view/src/analytics.rs`; a new gate-read v1 schema under `contracts/` (paths as hex byte strings) | about 1,100 lines with tests | gate-read v1 (proposed) | a first visit has no shadow and journals `first_visit`; a v6-shaped previous result forces a full read with `unkeyed_result`; a clean previous verdict forces `clean_prior`; a second visit names the exact paths; a base merge removes base-only paths; an empty delta; a rewritten history; both caps with explicit `ReadLimits` values; ruling 9's four test groups, each plant with its control and each for `git_diff` and `git_query` alike: **the scratch is the engine's** (a scratch path planted before the read with a promisor config, a `diff.<driver>.command` and an `info/attributes` makes the read whole with `scratch_foreign` and no sentinel; the control runs the planted uploadpack without the exactness check; an existing name is refused; the scratch is never under the run-local tree or the worktree; its shape is no config, `HEAD` names no commit, an empty `refs/`, one `alternates` line, a ref-shaped revision refused, a positive control printing the changed path and text); **complete or whole** (a `:(exclude)*` file, a name with a newline, one with a double quote, one with a leading `:`, a committed `-diff` and a `binary` attribute, each with the cwd inside and outside the operated worktree, the text in the delta or the read whole, and a control showing each blanks or shortens the patch without the rule; a mode-only change, an empty added file, a binary file, a gitlink, a symlink, a deleted file and a rename; an empty set runs no `diff-tree -p`; a path with no hunk and a path outside the set give `delta_incomplete`; the byte budget stops a patch); **the repository names no program** (a `diff=` driver, a textconv, a `diff.<driver>.command`, the submodule `diff.submodule=diff` case with the submodule's own `diff.external`, the promisor case with `extensions.partialClone`/`remote.<n>.promisor`/`uploadpack` failing closed, `filter.<driver>.clean` and `.process`, `core.fsmonitor`, `core.hooksPath`, an `info/attributes` driver, an environment-carried `GIT_EXTERNAL_DIFF`/`GIT_CONFIG_*`, and an alternates chain whose target names a driver); **the tree check is not the repository's** (the lying clean filter and the fsmonitor that reports nothing changed each make the read whole with the filter and the hook never started, the control showing `git status --porcelain` print nothing; a clean tree is proven clean; `ident`, `eol` and a CRLF checkout read dirty; a refused `--attr-source`, a timeout and a refused git each read dirty); the shadow journal carries the plan and the diff digest but no diff text; the payload plan and the input projection cannot disagree; a mutated payload fails `read_digest`; a recovery rebuilt from the journal yields the same input; a new event is refused; a shadow-era journal never counts a full read as a delta; `GateVisit` records `findings` and `follow_ups`; `compare()` over the union pins equal, moved, reworded, closed and only-grew; a fold test rebuilds `GateVisit` from the journal with both arrays; a follow-up carried in the first visit is still carried or closed by a covering read in the second; the potential saving in `analytics` |
| 0109c | The read policy and the triggers: a phase-machine v3 `reads` map, the closed trigger enum, the glob matcher, the security-class derivation and its refusal, the spec file-level plan, the artifacts arm and derived kind, the carry rule, and a delta commission for one non-security gate (the analyze gate first); the v3 move of fast and every table that composes over it; the delta-eligible charters and the prompt text that names the read kind and the delta. | a new phase-machine v3 schema under `contracts/`; `crates/brokkr-core/src/policy.rs` and its condition module; a new glob module under `crates/brokkr-core/src/`; `crates/brokkr-runtime/src/engine/input.rs`; `crates/brokkr-runtime/src/engine.rs`; `crates/brokkr-runtime/src/bundle/` (the security-class derivation, the artifacts arm and the agent loader's `security` and `results` fields); `crates/brokkr-runtime/src/agents.rs`; `crates/brokkr-runtime/src/agents/load/`; `agents/review-security.json` (`security: true` and `results`); `agents/review-adversarial.json` (`results` with `security-hold`); `agents/review-correctness.json` and `agents/review-spec-compliance.json` (`results: ["clean", "residual"]`); `agents/charters/review-correctness.md`; `agents/charters/review-spec-compliance.md`; `agents/charters/analyst.md` (names the read kind and the delta); `crates/brokkr-protocol/src/adapters.rs`; `recipes/fast/policy.json` (moves to v3 with no `reads` entry yet); `recipes/triage/policy.json` (v3 with the analyze `reads` entry); `recipes/node/policy.json`, `recipes/review-first/policy.json`, `recipes/landing/policy.json`, `recipes/panel-review/policy.json` (their own v2 tables move to v3 with fast, because one composition shares one schema) | about 1,000 lines with tests | phase-machine v3 (proposed) | every trigger, including `outside_threshold` and its derived threshold; an unknown trigger refused; a v1 table read as it is and never opted in; a v2 table carrying `reads` refused; an absent `reads` is full; the analyze gate's delta and its whole-read fallbacks; the analyze judge is delta-eligible; the spec file-level delta (a changed artifact file read whole, an unchanged one not re-read, a new match read whole, a missing file a whole read); the spec re-arm rule; the artifacts arm records each matched file's digest and the read plan carries the glob set; the derived kind; the dirty refusal and the unresolved-base refusal; the carry rule: an outside-hunk finding is carried, not closed, and a blocking outside finding where no threshold derives forces a whole read with `outside_threshold`; a fileless finding stays standing; the taken delta's diff text is journaled, capped, and recovery reads it rather than a fresh `git diff`; the security-class derivation for the declared result set, the sole review position and the typed agent office; a delta mark refused on each; a member key or agent file name does not decide; the shipped whole-read set; a panel whose phase can end in `security-hold` with an undeclared security slot and no other class member whole-reads; the adversarial member is security-class; the security member's `security-hold` plus a high security finding is admitted; a correctness member's and a spec-compliance member's `clean` is admitted (compile and run); a member whose agent declares no list is held to `clean` and `residual`; a member result outside its agent's declared `results` is refused and a single-seat result its agent does not admit is refused; the analyze `reads` entry names `judge`, and the unnamed `check` reads whole; an overlay cannot clear `security` and an overlay can set it; the glob matcher; a `select`-case tag resolves to one site and an ambiguous bare tag refuses; triage's design and engine `positions` steps are not class as containers; gpt-flash's work-class review members are bound by ruling 2 and name no delta; the correctness, spec-compliance and analyst charters bind the read kind and the delta; a security-class charter keeps its whole read; `recipes/triage`, `recipes/fast`, `recipes/node`, `recipes/review-first`, `recipes/landing` and `recipes/panel-review` compose after the v3 move |
| 0109d | Review panels, standing security and follow-ups: the engine rejects an understated or false aggregate, whole-reads a security position, closes a follow-up only by a covering read, gives the one single-review agent both residual inputs, and names the panel and preflight read policies. | `crates/brokkr-runtime/src/bundle/aggregate.rs`; `crates/brokkr-runtime/src/engine/decide.rs`; `crates/brokkr-runtime/src/engine/input.rs`; `crates/brokkr-core/src/fold.rs`; `agents/brokkr-reviewer.json` (both residual inputs, one home; self and release inherit); `recipes/panel-review/policy.json`; `recipes/preflight/policy.json` (each v3, naming its read policy; panel-review's table moved in 0109c) | about 650 lines with tests | none new; v7 admits `follow_ups` from 0109a | a fileless finding stays standing and an inside-delta read's omission does not close it; a blocking outside-delta finding with a derived threshold forces a whole read with `outside_threshold`; a seat cannot declare a closure and a seat-supplied `follow_ups` is refused; a follow-up closes only by a covering read; a carried follow-up is not closed; a standing security finding blocks and forces full; a non-empty `findings` with the aggregate input absent refused; the understated-severity refusal; the false-security refusal; a mixed panel's security member whole-reads and its correctness and spec-compliance members delta-read; a panel's follow-ups are named; self and release compile with both residual inputs; fast's verify and ship gates are unopted (no `reads` entry), so the absent-entry rule keeps them whole-read, and the commission carries no delta; two consecutive delta visits, with a finding carried in the first still carried or closed by a covering read in the second |
| 0109e | The readouts and the operator one-shot: `analytics` full/delta/trigger counts, `inspect`'s read cell and visit number, `operator whole-read`. | `crates/brokkr-view/src/analytics.rs`; `crates/brokkr-view/src/lib.rs`; `crates/brokkr-cli/src/render.rs`; the operator verb under `crates/brokkr-cli/src/verbs/`; `crates/brokkr-core/src/fold.rs` (the fourth `OperatorCommand` variant and the phase argument) | about 450 lines with tests | none new | full/delta/trigger counts; `unrecorded` for a journal with no read record; the `inspect` line with the visit number and the read cell; the public-API snapshot moves only for the added view fields; the operator command is one-shot and phase-scoped; no operator command can widen a security refusal |

The order is buildable: a gives b the key and the covered identity, b gives c the
measurement and the planner, c gives d the policy and the plan, and d gives e the
verdict facts it paints. c moves fast and every table that composes over it in one
step, because `merge_table` refuses mixed schemas; d then adds the panel and
preflight read policies over the v3 base. Each slice moves the decision's `Built:` marker and the
index cell only when its tests pass. The first slice is recommended because it
commissions no delta: its only new outcome is a malformed verdict parking, and the
next field sweep can compare the shadow delta's size with the full read it still
commissioned, the number the operator needs before opting a recipe in.

**Contention:** `crates/brokkr-protocol/src/adapters.rs` is touched by 0109a (the
v7 result shape) and 0109c (the read kind and the delta in the prompt). The
slices are sequential, so the repair is one branch at a time; the second slice
builds its prompt edit on the first and the text test pins both sentences.
`crates/brokkr-runtime/src/engine/broker.rs` (the protected walk gains the reads
root's names) and `crates/brokkr-runtime/src/engine/worktree.rs` (the tree check)
are touched by 0109b only; a sibling branch that edits either is serialized with
it, one branch at a time, as the capability broker's slices already are.
`agents/charters/review-correctness.md` is edited by 0109a (the clean's
`none`/`false`) and again by 0109c (the read kind): the second builds on the
first.

## Consequences

The saving is the adopter's figure, not a promise: if a review re-visit reads only
the fix diff and the standing findings, the 29% code-review share and the 55% spec
share have room to fall where the changes are small. The cost is a contract
version, a new read plan in every gate's request, a delta computation that runs
git in the engine, two scratch git directories per read under the engine's own
state root, one patch process per changed path, a tree check that hashes every
tracked file once, and triggers a recipe author must choose. The refusals that
keep the saving honest are that a security-class position and a standing security
finding never take the delta, a clean verdict never licenses one, a delta that is
shorter than the change is a whole read, and nothing a seat can write is an input
to the read. The price of that last refusal is a whole read wherever the check
cannot prove the tree (an LFS or CRLF checkout, a git older than 2.40, a SHA-256
repository, a mode-only or binary change).

The effect is uneven by design. Every single review seat that declares
`security-hold` whole-reads, and a mixed panel's security and adversarial members
whole-read too; the `verify` and `ship` exec gates are not review positions, and
they stay whole-read because no `reads` entry opts them in (the absent-entry
rule). The saving lands in the spec and analyze gates and in the
correctness and spec-compliance members of a mixed panel, not in the security
member and not in the exec gates. That is the fail-closed shape the decision
keeps: the floor misses no seat that can hold for security, and a panel with no
security-class member whole-reads.

The residual risks are named rather than hidden. A delta read trusts the prior
verdict: a finding no one reported is not in the standing set, and re-reading only
the delta will not find it; the whole-read triggers are the operator's bound on
that cost. A seat can reword a finding and give it a new id: the old key closes and
the new opens, so #1048(a) reads progress; the `GateVisit` standing sets and
`compare()` make it visible, but the engine does not guess a fuzzy match. A dirty
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
139:        "max_residual_severity_above": "none",
140:        "max_residual_severity_at_most": "low"
151:        "max_residual_severity_above": "low"
162:        "max_residual_severity_above": "none",
163:        "max_residual_severity_at_most": "low"
174:        "max_residual_severity_at_most": "none"
(no has_security_residual line in recipes/fast/policy.json)
grep -n '"inputs"' agents/{fast-reviewer,brokkr-fast-reviewer,node-reviewer,verify-reviewer,brokkr-reviewer,reviewer}.json
agents/fast-reviewer.json:3:  "inputs": ["has_security_residual", "max_residual_severity"]
agents/brokkr-fast-reviewer.json:5:  "inputs": ["has_security_residual", "max_residual_severity"],
agents/node-reviewer.json:4:  "inputs": ["has_security_residual", "max_residual_severity"]
agents/verify-reviewer.json:6:  "inputs": ["has_security_residual", "max_residual_severity"],
(no inputs line in agents/brokkr-reviewer.json or agents/reviewer.json)
grep -n '"inputs"' recipes/{brokkr-fast,fast,node,self,release,verify}/bundle.json
(no output; these seats hire the agents above and cannot write `inputs` beside `agent`)
grep -n 'const OVERLAY_KEYS' -A2 crates/brokkr-runtime/src/agents/load/overlay.rs
54:const OVERLAY_KEYS: [&str; 8] = [
55-    "extends", "models", "efforts", "charter", "inputs", "tools", "hands", "replaces",
56-];
grep -n 'security-hold' agents/charters/{review-adversarial,review-correctness,review-spec-compliance}.md
agents/charters/review-adversarial.md:22:`security-hold` for any unresolved high/critical finding — one hold from
agents/charters/review-correctness.md:19:finding in `notes` · never `security-hold` (that verdict belongs to the
grep -n '"select"' recipes/{triage,gpt-flash}/bundle.json
recipes/triage/bundle.json:23:      "select": {
recipes/triage/bundle.json:37:      "select": {
recipes/gpt-flash/bundle.json:48:      "select": {
recipes/gpt-flash/bundle.json:85:      "select": {
```

The base tree is 2,426 files and 1,004,496 lines; at the design commit the same
commands print 2,427 files and 1,005,065 lines, because the design story adds the
decision and the index row. The design branch's diff is the 2-file/569-insertion
read above; the fix commit `baed1b168` is 2 files, 669 insertions and 306
deletions; a final branch diff after this revision's own commit is **unmeasured**,
because this document changes it. The recipe count is 14 declarations plus five
inheriting overlays; the compiled count is unmeasured. The fast table references
only `max_residual_severity`, but `agents/fast-reviewer.json`,
`agents/brokkr-fast-reviewer.json` and `agents/node-reviewer.json` already declare
both inputs, so fast, brokkr-fast and node compile both; verify's agent declares
both and its table references both. Only `agents/brokkr-reviewer.json`, hired by
self and release, declares neither, so those two compile `max_residual_severity`
only; slice 0109d adds both to that one agent, and self and release inherit. The
result-file shape, the engine's journaling, the two head records, the residual
derivation, the four panel inputs, the policy vocabulary, the dialect's artifact
names and section string, and the readouts are cited by file and line above.
Everything not observed is marked unmeasured: in particular, how many files a
particular gate seat actually opened on a particular re-visit, and the saving a
delta would have produced in any real run. Those are the first slice's
measurement, not this document's claim.

### Checks run for the 1088r5 round

Every result in ruling 9's "What was observed" list was run on git 2.51.0 in
scratch repositories under `/tmp`, not in this worktree. The implementer's shell
refuses scripts, `mkdir` and environment prefixes outside the workspace, so each
repository was built with git alone (`git fast-import` from a stream kept under
`.forge/`, `git clone --bare --shared`, then `git config --remove-section core`
and `remote.origin`, `git symbolic-ref HEAD refs/heads/none` and
`git update-ref -d` to leave the shape the engine will write: an empty config, an
unborn `HEAD`, no refs, one `objects/info/alternates` line). The hand-made form,
a directory holding `HEAD`, an empty `refs/` and `objects/info/alternates`, was
run too. Each sentinel is a script a repository names, which appends to a file
under `/tmp`; "present" and "absent" are `git hash-object <file>` finding it or
not. The commands are quoted without paths; `<scratch>` is the config-less form.

```text
diff-tree -r -z --raw --no-renames --no-commit-id c1 c2      (operated repo)
  lists .gitattributes, ":(exclude)*", ":lead.txt", a.txt, bin.dat, a path with
  a newline, link, mode.sh, a path with a double quote; the newline path is raw
diff-tree -r -p -U3 --no-commit-id c1 c2 -- ':(exclude)*'           0 bytes
git --literal-pathspecs ... same command                            the hunk
diff-tree ... -- ':lead.txt'                                        0 bytes
diff-tree ... -- '"line\nbreak.txt"'                                0 bytes
diff-tree -p c1 c2 --                                               every path
operated worktree as cwd, committed "*.txt -diff":
  "Binary files a/a.txt and b/a.txt differ" (also bin.dat, :lead.txt, the
  newline path and the quote path); mode.sh prints "old mode / new mode" and no @@
operated .git as the git dir, an unrelated empty directory as cwd (no
  .gitattributes there, the operated index holds one):         Binary files ... differ
<scratch>, operated worktree as cwd:      Binary files ... differ
<scratch>, <scratch> as cwd:               the text hunk
<scratch>, operated worktree as cwd, --attr-source=4b825dc...:  the text hunk
<scratch>, operated worktree as cwd, --text:                    the text hunk
<scratch> with HEAD detached at c2, core.bare=true or not:       the text hunk
<scratch> with HEAD detached at c2, -c attr.tree=c2:             Binary files ...
<scratch>, whole range, cwd the scratch:  eight text hunks and mode.sh with none
directory with HEAD + objects/info/alternates, no refs/:  exit 128, not a git repository
directory with HEAD + empty refs/ + objects/info/alternates:     the text hunk
partial clone, one blob missing, clean <scratch>:   exit 128 "unable to read <blob>"; sentinel absent
same, planted scratch config (partialClone, promisor, url, uploadpack = script):
  with git --no-lazy-fetch:   exit 128 "could not fetch ... from promisor remote"; absent
  without it:                 the hunk is printed; sentinel PRESENT
operated repo, *.txt filter=evil, filter.evil.clean = prints the index blob:
  git status --porcelain:     empty output over a content-dirty a.txt; sentinel PRESENT
operated repo, core.fsmonitor = hook answering "nothing changed", external write:
  git status --porcelain:     empty;  git -c core.fsmonitor=false status: " M a.txt"
tree scratch (read-tree c, update-index --refresh, diff-files --raw), same worktrees:
  filter case:     exit 1 "a.txt: needs update"; diff-files lists a.txt; sentinel absent
  fsmonitor case:  exit 1 "a.txt: needs update"; diff-files lists a.txt; hook absent
  with --attr-source=4b825dc..., --ignore-submodules:   the same, exit 1
  clean tree (this worktree, 2,427 tracked files):  exit 0, no output, 0.229 s wall
diff.<driver>.command + textconv planted in an operated repo:
  git log -p --ext-diff --textconv:  both sentinels PRESENT
  <scratch> diff-tree as ruling 9 states it, operated worktree as cwd:  the hunks, both absent
```

**Not re-run by the implementer:** 1088r4's matrix entries for the submodule child
diff, `core.hooksPath`, `info/attributes`, `GIT_EXTERNAL_DIFF` and `GIT_CONFIG_*`
in the environment, because the shell refuses scripts and environment prefixes.
Nothing in this round's statement changes what those depend on (an empty
configuration, no attribute source, ids only, a constructed environment), and the
1088r4 panel's results for them, on the same git, stand as theirs. The `ident`,
`eol` and `working-tree-encoding` attributes under `update-index --refresh`,
the NUL-content binary case and the file-to-directory type change are
**unmeasured**; 0109b's tests plant each.
