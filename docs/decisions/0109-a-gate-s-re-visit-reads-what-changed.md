# 0109 — A gate's re-visit reads what changed since its last verdict, and the findings that still stand, unless a declared trigger asks for a whole read

Status: accepted — operator ruled in chat 2026-11-17, "accept 0109 as recommended" (https://github.com/feedback-loop-ai/brokkr/issues/1088#issuecomment-6960606249)
Date: 2026-11-15

Built: unbuilt (#1088)
Amends: 0034

This decision is a document only. It sets the shapes and rules a later build
will enforce; it mints no contract version and adds no code, test, guide or
recipe. It acts on 0034's successful-result record and relies on 0071's rulings 2,
5 and 9, on 0082's minting rule, and on 0022's and 0006's residual and attempt
bounds, which stand unchanged. The replacement sentence for 0034 is in
[What 0034 says after acceptance](#what-0034-says-after-acceptance); the header
pointer above and 0034's back-pointer were added at acceptance, 2026-11-17. One
rule stands in front of every choice: the engine decides the read and the
transition from typed facts, never from a seat's prose; no model chooses a whole
read, a delta read or a routing.

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
says so. The operator accepted every ruling as recommended on 2026-11-17 (#1088).
Each contract version this decision marks proposed is minted by the slice that
builds it, under 0082's minting rule.

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
  widened by the diff's context (the plan uses `-U3`, so three lines). A standing
  finding's `line` is a line of the covered revision (`since`), so it is read
  against each hunk's old-side range (`@@ -a,b`). The engine records the hunks it
  read; it does not guess. **A finding on a path the delta deletes is inside:**
  the deletion's one hunk covers every old line (observed: a deleted one-line
  file's patch is `@@ -1 +0,0 @@`), so a fix that deletes the offending file
  closes the finding on that read, and no loop forms.
- **A locator's revision, and how a carried one moves.** *Every locator a
  `GateVisit` records is a line of that visit's own `covered.head`.* A finding
  the seat reports is a line of the head it read (the tree check proved the
  worktree is that head). A finding the visit carries without the seat's answer
  (outside the delta, or a standing one the read did not cover) is translated by
  the engine from `since` to `head` before it is recorded or handed out, through
  the hunks of that visit's own patch: for a carried line `L` of `since` in a
  path of the set, outside every hunk's old-side range, the new line is
  `L + (c + d) − (a + b)` for the last hunk `@@ -a,b +c,d @@` that ends before
  `L`, and `L` itself if no hunk does; a line in a path outside the set is
  unchanged. This is exact, not a guess: the completeness rule gives every path
  of the set its whole patch, so every hunk of the path is known, and outside a
  hunk git's lines are the same lines shifted. A translation the engine cannot
  prove makes the finding unplaceable, below: a carried `L` past the end of the
  `since` blob, or an `L'` past the end of the `head` blob, both of which the
  engine reads from verified blobs. An unplaceable finding is recorded with its
  `file` and without a `line`, so no stale line is handed to a later seat. A
  whole read needs no translation: it covers every locator, and the seat either
  re-reports a finding at a line of the head it read or omits it. Observed
  (this round): a finding at line 10 of a 50-line `f`; visit 2 prepends 20 lines,
  its patch is `@@ -1,3 +1,23 @@`, the finding is outside and carried at
  `10 + 24 − 4 = 30`, where its text now is; visit 3 edits only the current
  line 10, its patch is `@@ -7,7 +7,7 @@`, the carried line 30 is outside and
  stays 30. Without the translation the stored 10 falls inside `-7,7`, and the
  seat's omission would close a finding whose text no read covered.
- **Outside.** A finding outside the changed hunks of a changed file — or in a
  file the delta does not touch — is outside. It is carried, never closed.
- **Unplaceable.** A finding with no `file`, or whose `file` and `line` the delta
  cannot place (a path outside the repository, a line past the covered file's
  end, a hand-built older record), is a third case: neither inside nor outside.
  It fails closed: it is not closed by an inside-delta read's omission. **A
  standing unplaceable finding that is blocking is the whole-read trigger
  `unplaceable_finding` (ruling 10)**, so the next visit reads whole, and that
  whole read covers it and can close it; it cannot stand on every delta visit. A
  non-blocking unplaceable finding is carried as a follow-up. A new finding must
  carry `file`; the fail-closed arm is for a historical `GateVisit` that lacks
  one.
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

The engine checks the declared aggregate against the blocking findings, **for
each residual input the seat's compiled input set declares**: a
`max_residual_severity` lower than the highest blocking finding, or
`has_security_residual` false while a blocking finding has `security: true`,
refuses the result; a non-empty `findings` with a residual input absent **that
the seat's compiled set declares** also refuses, so a seat cannot omit the input
it cannot understate. In ruling 2's words, a seat that declares no residual
inputs is not refused by this check: its findings are bound by ruling 2's
findings arm alone, so a blocking finding still refuses a passing word, and a
`security: true` finding still needs a declared `security-hold` or the result
is refused and the run parks. The analyze gate is such a seat: it declares only
`drift_in` (`recipes/triage/bundle.json:133-135`), `agents/analyst.json`
declares no inputs, and the engine keeps only declared names
(`crates/brokkr-runtime/src/engine/decide.rs:76-82`), so an analyst could not
write the pair even if its charter asked. A charter-shaped `drift` with
`drift_in` and a medium finding is therefore admitted (`drift` is non-passing)
and routes to its owning phase (`recipes/triage/policy.json:37-39`), and
`consistent` plus a blocking finding is refused by the findings arm. **Not
taken: giving analyze the residual pair in 0109c.** It would change a shipped
recipe's compiled input set and the analyst's charter, for no gain the text
needs: analyze routes on `drift_in`, never on a severity, and the findings arm
already refuses the one wrong word a blocking finding could hide behind.
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
finding outside the changed hunk of a changed file is carried; a carried locator
is translated through the visit's hunks (the three-visit insert-above, carry,
unrelated-edit test: line 10 carried at 30 by `@@ -1,3 +1,23 @@`, still 30 and
still standing after `@@ -7,7 +7,7 @@`, its control without the translation
closing it), and a translation past either blob's end is unplaceable and loses
its line; a fileless finding
stays standing and an inside-delta read's omission does not close it; a standing
blocking fileless finding forces the next visit whole with `unplaceable_finding`,
and that whole read's omission closes it; a finding on a path the delta deletes
is inside, and the deleting read's omission closes it; an outside
finding at or above the derived threshold forces a whole read, with
`outside_threshold` journaled; where no threshold derives, a blocking outside
finding forces a whole read; a non-empty `findings` with a residual input
absent that the seat declares is refused; a charter-shaped analyze `drift` with
a medium finding and no residual pair routes to its owning phase, and
`consistent` plus a blocking finding is refused; a seat-supplied `follow_ups` is refused; a seat cannot declare a closure; a
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

- **`head`:** the commit the gate read, and whether the worktree was dirty at the
  start. It records no base: the delta read takes none (ruling 9's base rule). The
  arm has two field sets. A gate with its own head record writes
  `{kind: "head", head: <commit id>, dirty: bool}`. The bundle's
  protected phase writes `{kind: "head", head_digest: <sha256 of the head
  string>, dirty: bool}`: its `covered.head` is a digest-pinned
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
kind, the dirty refusal, the absence of a `base` field, the
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
set of every path that changed in it, one patch per path, and the standing
findings; the read is complete or it is whole; every object it rests on is
verified by the engine's own hash; and nothing the read uses can be written by a
seat or named by the repository.**

Five rules hold this ruling, each stated once below and each an ask of its own:
the **isolation rule** (what the read runs in), the **completeness rule** (what
the read must carry), the **tree-check rule** (how the engine knows the worktree
is the head it read), the **object rule** (how the engine knows an object's
bytes are the ones its id names) and the **base rule** (why no ref the seat can
move decides the set). The classes they close are not one more vector each:
"the repository names a program" (r4), "the read came back short without
failing" and "the scratch is seat-writable" (r5), and "**the read trusts an
input the work seat wrote**" (this round): the work seat commits, so it writes
the object store and the refs, and it can write anything else in the operated
repository. Rounds r2 to r4 closed vectors one flag at a time; each rule below
is one sentence that holds by construction, plus a check that fails closed.
[What the read trusts](#what-the-read-trusts-and-what-the-work-seat-can-write)
then lists every input the work seat can write, with the rule or check that
covers it.

The engine composes: (1) `since` is the previous covered revision, read from the
journal, and `head` is the stable-head check's id. The engine reads both commits
through the object rule, and walks `head`'s parents in the verified commit
bytes; if the walk does not reach `since` within 1,000 commits, the read is full
(`ancestor_lost`). (2) The changed set is **every path that differs between
`since`'s tree and `head`'s tree**, computed by the engine over verified tree
objects, kept as raw bytes with each entry's two modes and two ids. No base and
no filter enters it (the base rule). (3) The patch is read **one path at a
time**: one `diff-tree -p` per path in the set, with `-U3`, each naming exactly
that one path as a literal pathspec, in a scratch that holds only verified
objects. `delta_files_gt` is checked against the set's size before any patch
runs, and `delta_bytes_gt` against the bytes read, which are bounded: a command
whose output passes the remaining budget is stopped and the read is whole. (4)
The completeness check (below) runs over the set and the patches. (5) The
commission carries the `read` plan, the diff text, and the standing findings with
their keys, locators and severities. A security-class site takes the whole read
instead (ruling 11). A spec gate follows ruling 12's file-level plan instead of
the git range.

**The object rule.** *Every commit, tree and blob whose bytes decide the changed
set, a patch or the tree check is hashed by the engine, and equals the id the
engine asked for, or the gate reads whole.* Git does not check a loose or packed
object's bytes against its id when it reads one: a head blob's loose object
replaced by other bytes under the same id printed the replacement as the patch
text at exit 0, a replaced `since` tree and a replaced `since` commit each
shortened the raw set at exit 0, and `cat-file -e` exited 0 on each (observed
below). So the engine does not let git read an object it has not verified:

- **How.** The engine reads each object it needs with one long-lived
  `cat-file --batch` in the **source scratch** (the isolation rule's scratch
  whose `alternates` names the operated object directory), feeding ids on stdin
  and reading `<id> <type> <size>` and the bytes. It hashes `<type> <size>\0`
  and the bytes with SHA-1 in collision-detecting form (the sha1dc algorithm git
  itself uses; a new `sha1collisiondetection` dependency, so a crafted colliding
  pair is a mismatch, not a match), and compares the result and the type with
  the id and type it asked for. A different hash or type is the whole-read
  trigger `object_mismatch`; a `missing` line is `read_failed`, and nothing is
  fetched.
- **Which objects.** The `since` and `head` commits and every commit the
  ancestry walk reads; both root trees, and every subtree whose id differs
  between the two sides along the way to a changed path (a subtree with the same
  id on both sides is one object, so nothing beneath it can differ, and it is not
  read); both blobs of every changed path (one for an add or a delete). For the
  tree check: every tree of `head`, and the blob of every symlink entry, because
  `update-index --refresh` compares a symlink by reading its blob. The engine
  parses commit and tree bytes itself (a commit's `tree` and `parent` lines; a
  tree's `<mode> <name>\0<20-byte id>` entries), as pure functions in
  `brokkr-core`, so parents come from the verified bytes and no commit-graph,
  graft, replace ref or shallow file is consulted for them.
- **Then git reads only what the engine verified.** The engine writes each
  verified object into the **patch scratch** (or, for the tree check, the
  **tree scratch**) as a loose object, zlib-deflated by the deflate crate the
  lockfile already holds (`Cargo.lock`, 1.1.10, today only a transitive
  dependency; it becomes a direct one), at the path its own hash names. Neither
  scratch has an `alternates` line: git in it can open no object the engine did
  not verify, and an object the engine refused is simply absent, so the command
  fails and the read is whole (observed: with the forged blob written under its
  true hash, the a.txt patch exits 128, "unable to read 7a1591ac…", while b.txt
  prints its hunk). As a second derivation of the set, git's own `diff-tree -r -z
  --raw` over the two verified root trees runs in the patch scratch, and its
  output must equal the engine's set byte for byte, or the read is
  `delta_incomplete`.
- **What it costs.** Per delta read: two commits plus the walk's commits, two
  root trees plus the differing subtrees on each side, and at most two blobs per
  changed path, each read once, hashed once and written once. Measured on this
  branch's last fix commit (`b486d3ffd..0af190c64`, one changed path): 10 objects,
  233,800 bytes (2 commits of 780 and 1,256 bytes; 6 trees of 1,454, 1,454, 432,
  432, 7,262 and 7,262; 2 blobs of 91,015 and 122,453). Per tree check: every
  tree of `head`; on this tree 693 trees, 130,435 bytes, and no symlink. The
  object bytes count against a third budget, `ReadLimits.objects` (default 16
  MiB, not a policy key), and a read past it is whole (`delta_bytes_gt`, with the
  budget named in the journaled cause). A blob-heavy change (a one-line edit to a
  large file) therefore costs both versions of that file.

**The base rule.** *No ref the work seat can move decides which paths the delta
carries.* The r5 statement filtered the `since..HEAD` set by the `base...HEAD`
set, with `base` the merge-base of HEAD and the default-branch tip, and that tip
is a ref of the operated repository (`origin/HEAD`, else a local `main` or
`master`: `crates/brokkr-runtime/src/anchor.rs:111-135` names the same lookup for
0038's measure). One `git update-ref refs/heads/main <E>`, with `E` a commit of
the branch, moved the merge-base to `E`, and the filter dropped every path `E`
changed; an `info/grafts` line moved it the same way without touching a ref
(both observed below). **Taken: the filter is dropped.** The set is the whole
`since..HEAD` difference, and no base is read, recorded or compared; `base` leaves
`covered` (ruling 7) and `base_unknown` leaves the trigger list (ruling 10).

- **(a) Require the base to equal the previous visit's journaled `covered.base`**
  (the commission's recommendation) **is refused, because it does not hold the
  sentence.** The first visit's `covered.base` is itself read from the same ref,
  and the implementer runs before the first review. Observed: the seat points
  `main` at `E` (adding `x.txt` as `EVIL backdoor`) before visit 1; visit 1 covers
  `S'` (`x.txt` made benign), with `covered.base = E`; the seat then commits `H'`
  restoring the evil `x.txt`. At visit 2 the merge-base is `E`, equal to
  `covered.base`, so the rule admits it, and the filter drops `x.txt` from
  `{x.txt, y.txt}` at exit 0. A base equal to a seat-chosen base is still a base
  the seat chose. And whenever the real default branch moves, the merge-base
  moves and (a) reads whole anyway, so the filter's only remaining effect is to
  drop a path whose head bytes equal the base's.
- **(b) Drop the filter. Recommended and taken.** Nothing the seat can write
  decides the set: it is the two verified trees' difference, complete whether or
  not `since` is an ancestor. The cost is the saving the filter bought: after a
  merge of the default branch into the branch, the merged paths are in the
  delta, which over-reads rather than under-reads, and a large merge passes
  `delta_files_gt` and reads whole. A rebase onto the default branch already
  read whole (`ancestor_lost`), so the filter only ever saved on a merge.
- **A later option, not built:** a base read from a source the seat cannot
  write (the forge's own record of the default-branch tip, fetched by the
  engine) could restore the filter. It needs a source this decision does not
  have.

A path is a byte string, never a `String`: a path with a newline, a quote or a
byte outside UTF-8 is an ordinary path. The plan carries each path as bytes in
the gate-read schema (an array of lowercase-hex byte strings), the view derives a
display form and nothing reads that form back (0071 ruling 3).

**The isolation rule.** *No seat and no repository can write the scratch, and the
engine reads it only as it made it.* Every git command of the delta read and of
the tree check runs in a scratch git directory the engine makes at the read, from
an environment the engine constructs, given object ids rather than refs. Only
the source scratch reaches the repository's objects, through
`objects/info/alternates`, and only to hand their bytes to the object rule; the
patch and tree scratches hold nothing but objects the engine verified. A missing
object fails the read closed. The sentence is the build's test, and it
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
- **What is in it.** Three directories made together under one random parent,
  each holding only what the engine writes, and each with `HEAD` holding the
  constant `ref: refs/heads/main`, which names no commit (the operated
  repository's HEAD is never copied into it), and an **empty `refs/` directory**,
  because git refuses a directory without one (observed below). The **source
  scratch** adds `objects/info/alternates`, one line naming the operated
  repository's object directory, resolved from the worktree's `.git` (a
  directory, or a file naming the git dir) and that git dir's `commondir` when it
  names one: a path read, never a config read. Only `cat-file --batch` runs in
  it, and everything it returns is hashed (the object rule), so a planted `.git`
  or `commondir` can point only at another object directory, which can supply an
  object the hash then judges, or withhold one, never a program. The **patch
  scratch** has no `alternates`; its `objects/` holds only the loose objects the
  engine wrote after verifying them. The **tree scratch** (the tree-check rule)
  is the same, holding the verified trees of `head` and its symlink blobs, and
  gains one `index`, written by the engine's own `read-tree`; the index is never
  in the patch scratch, because git falls back to the index for attributes
  (observed: the operated `.git` with an unrelated directory as cwd still printed
  "Binary files" for a `*.txt -diff` its index held), and a covered head's
  committed `.gitattributes` would then apply to the patch. None has a `config`,
  a `hooks/`, an `info/attributes`, an `info/exclude`, an `info/grafts`, a
  `.gitattributes`, a `packed-refs`, a `shallow`, a `refs/replace`, a
  `commit-graph`, a pack or an `.idx`, so no local configuration, no hook, no
  attribute source, no replacement, no graft, no commit-graph and no shallow
  boundary of the operated repository is in view; `GIT_CONFIG_NOSYSTEM=1`,
  `GIT_CONFIG_GLOBAL=/dev/null` and `GIT_CONFIG_COUNT=0` leave no system or
  global configuration.
- **The engine reads it only as it made it.** Before any git command opens a
  scratch the engine lists it through its directory handle, without following a
  link, and compares the listing with what it wrote: the entry names, their kinds,
  the bytes of `HEAD` and `alternates`, and in the patch and tree scratches every
  object file's name and bytes. Any other entry (a `config`, an
  `info/attributes`, a `.gitattributes`, a `hooks/`, a symlink, an object the
  engine did not write, a second `objects/` entry) is the whole-read trigger
  `scratch_foreign`, and no git command starts in it. The same check runs on the
  tree scratch with its `index` absent until the engine's own `read-tree` makes
  it.
- **The cwd.** Every child's working directory is its scratch, never the operated
  worktree and never a path a seat can name. Git takes the working directory as
  the worktree when the directory it is given is not bare; a child run with the
  operated worktree as its cwd reads that worktree's `.gitattributes`, and a
  committed `*.txt -diff` turned the patch into "Binary files ... differ"
  (observed below).
- **What it costs.** Three directories, four small files, one index, the verified objects
  (the object rule's cost) and one listing per read; all are removed by a guard
  after the read, on success or failure. A crash
  leaves the random parent under the engine's own root, where nothing reads it
  again, since a name is never reused; sweeping leftovers belongs to whatever the
  engine already sweeps at start (unmeasured here, so a follow-up).
- **What it does not defend** is listed once, with every other input, in
  [What the read trusts](#what-the-read-trusts-and-what-the-work-seat-can-write).
- **The command.** The engine has a new `git_diff` helper at the runtime edge
  (`git_query`, below, is the other). It runs plumbing, not porcelain, with the
  verified tree ids, and the child's working directory is the patch scratch. The
  raw set is read with no pathspec, so the pathless form is the one intended; the
  patch is read once per path, never from a list:

  ```text
  git --git-dir=<patch scratch> -c core.fsmonitor=false -c core.attributesFile=/dev/null \
      --no-pager diff-tree -r -z --raw --no-renames <since tree> <head tree>
  git --git-dir=<patch scratch> -c core.fsmonitor=false -c core.attributesFile=/dev/null \
      --no-pager diff-tree -r -p -U3 --no-renames --no-ext-diff --no-textconv \
      --ignore-submodules=all <since tree> <head tree> -- <one path>
  ```

  The first line is the second derivation the object rule compares with the
  engine's own set. The child environment is constructed, not inherited:
  `GIT_DIR` is the scratch, `PATH` is the engine's, then
  `GIT_CONFIG_NOSYSTEM=1`, `GIT_CONFIG_GLOBAL=/dev/null`, `GIT_CONFIG_COUNT=0`,
  `GIT_ATTR_NOSYSTEM=1`, `GIT_NO_LAZY_FETCH=1`, `GIT_NO_REPLACE_OBJECTS=1` and
  `GIT_LITERAL_PATHSPECS=1`. No
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
  patch scratch, which holds none; that scratch has no index and no
  `info/attributes`; `core.attributesFile` and the system file name nothing; and
  its `HEAD` names no commit, so neither `attr.tree` nor a bare repository's
  default tree is a source.
  `--attr-source` (git 2.40) is not used on the read: it names an empty tree where
  it exists and is an unknown option, a failure, where it does not, and a read
  that cannot run on an older supported git is no delta, not a safer one. The
  tree check, whose input is the operated worktree's own `.gitattributes`, does
  need it, and an older git there fails closed (the tree-check rule).
- **The object reads.** The new scrubbed `git_query` helper runs one
  `cat-file --batch` in the source scratch with the same environment, fed ids
  only; the object rule hashes what it returns. It runs no `merge-base` and no
  `cat-file -e`: ancestry is the engine's walk over verified commits, and an
  object's presence is proven by reading and hashing it. A missing object is
  `read_failed`; nothing is fetched. The one ref the delta read resolves is
  `HEAD`, through the effect's existing stable-head check
  (`crates/brokkr-runtime/src/engine.rs:942-968`, `:3219-3222`): the head is the
  commit under review, so the seat choosing it is the seat choosing what is
  reviewed, and a moved head parks before a verdict. The default-branch tip is
  not read (the base rule).
- **No refs.** Every revision argument is a full lowercase hex object id from the
  engine's own read; the helper refuses anything that is not one before it starts
  git, so a ref name and an option-shaped revision are data it never passes. A
  path travels as one literal pathspec after `--`, by the environment above. Each
  scratch's constant `HEAD` is its only ref file and names nothing, so no ref of
  the operated repository, no `refs/replace/` and no `attr.tree` is resolved in
  the read (observed: a replace ref planted in the operated repository shortened
  its own `diff-tree` to `b.txt`, while the source scratch over the same objects
  listed `a.txt` and `b.txt`).
- **The alternates chain.** A repository may itself carry
  `objects/info/alternates`; the source scratch follows that chain to more
  objects and nothing else. A chain target's config, attributes and hooks are not
  read, every object it supplies is hashed, and a chain that cannot supply an
  object fails the read closed.

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
set is never reconstructed from patch text: the set is the engine's walk over
verified trees, equal byte for byte to git's `-z` raw output, and the patch
is attributed to its path by the one literal pathspec that produced it. A path
with no hunk fails the check by rule 2, which therefore also sends these to a
whole read, none of them a bug: a mode-only change, an empty added file, a
binary file (a NUL in the blob, git's size threshold, or an attribute), a
gitlink. A deleted file and a symlink have a hunk (the deleted lines, the link
target) and pass. The cost is a whole read where a delta is possible; that is the
fail-closed side, and a recipe that wants a mode change read does not need the
delta. `delta_incomplete` and its cause are journaled with the plan.

**An empty set runs no patch.** The set is empty when `since` and `head` name
the same root tree. The loop of step 3 then has no iteration:
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
git --git-dir=<tree scratch> --work-tree=<worktree> read-tree <head tree>
git --git-dir=<tree scratch> --work-tree=<worktree> --attr-source=<empty tree> \
    -c core.fsmonitor=false -c core.excludesFile=/dev/null \
    update-index --ignore-submodules --refresh
git ... --attr-source=<empty tree> diff-files --ignore-submodules=all --raw -z
git ... -c core.excludesFile=/dev/null ls-files -o --directory -z
git ... -c core.excludesFile=/dev/null check-ignore -v -n -z --stdin   (fed ":(top)" + each entry)
```

The first four run in the constructed environment of the patch read. The fifth
runs in **its own** constructed environment, the same one without
`GIT_LITERAL_PATHSPECS`: `check-ignore` refuses literal pathspecs outright
(observed: exit 128, `fatal: sub/: pathspec magic not supported by this command:
'literal'`), so under the patch's environment every worktree with any untracked
entry would read dirty. Without that variable, an entry is parsed as a pathspec,
and a seat-named untracked directory `:(top)target/` or `:target/` is read as
magic for `target/` and reported ignored by the tracked `target/` rule (observed),
which would admit it. So the engine feeds each entry with the long-form prefix
`:(top)`: the magic is parsed once, and the rest is the entry's bytes, whatever
they hold (observed: `:(top):(top)target/` and `:(top):target/` match no rule; a
directory named `targe*` matches none either, because the entry is the path being
matched, not a pattern). The engine then reads the output as records of four
fields and requires one record per fed entry, in order, each record's path field
byte-equal to what it fed; anything else (a missing, extra or different record,
a non-zero exit) is dirty.

The tree scratch holds only `head`'s verified trees and symlink blobs (the
object rule), so `read-tree` builds the index from trees whose bytes the engine
hashed: a forged tree cannot make a dirty worktree match it. The index is built
from the head the read is about, so the scratch's own
configuration is empty, its attribute source is the empty tree (`--attr-source`,
git 2.40: the worktree's `.gitattributes` is then not the source, observed for the
patch and for the `filter=` case below; whether `text`, `eol`, `ident` and
`working-tree-encoding` are ignored by `update-index` too is **unmeasured**, and
0109b's test plants each), no filter or fsmonitor is defined, and a file's bytes
are meant to be compared to the committed blob's, unconverted. The worktree is
proven clean only if `update-index --refresh` exits 0, `diff-files` prints
nothing, and every entry of the untracked listing is admitted by the untracked
rule below. **Anything else is dirty:** a
difference, a non-zero exit, a timeout, a git that cannot start, a git older than
2.40 (which refuses `--attr-source`), a SHA-256 repository (the scratch has no
`extensions.objectformat`), an unreadable worktree. Only the proof is clean. That
differs from `git_dirty` today, where a refused `git status` with no output reads
as not dirty (`worktree.rs:23-26`, `:45-47`); this ruling does not change
`git_dirty`.

*What it costs.* It hashes every tracked file once: the scratch index is new, so
there is no stat cache to skip a file by. Before that it reads, hashes and writes
every tree of `head` once (on this tree 693 trees, 130,435 bytes), and it lists
the untracked paths twice over (one listing, one `check-ignore` over it). On this tree (2,427 tracked files) the
`update-index --refresh` took 0.229 s wall on git 2.51.0 with a warm cache; a
cold cache or a far larger tree is **unmeasured**, and the cost grows with the
tracked bytes. *What it can get wrong, and the direction.* It can only say dirty
where `git status` says clean, never the reverse, because it is built to apply no
conversion (the test that plants `ident`, `eol` and a lying filter holds it to
that): a legitimate clean filter (an LFS pointer in the blob and the large
file in the worktree), a CRLF checkout, or an `ident` expansion compares unequal
and the worktree reads dirty. An LFS or CRLF adopter therefore takes a whole read
every time, which is no delta and not a wrong one.

*The untracked rule: only a tracked ignore rule hides a path.* An untracked
`.gitignore` is written by the seat without a commit, and one holding `*` hides
itself and its neighbours: an untracked `sub/.gitignore` with `*` beside an
untracked `sub/new.rs` left `git status --porcelain` and the r5 tree check both
clean (observed below), and cargo discovers an untracked `tests/` or `src/bin/`
file by itself. So the check lists every untracked path with no ignore rule
applied (`ls-files -o --directory`, which collapses an untracked directory to one
entry), and asks `check-ignore -v` which rule, from which file, ignores each. An
entry is admitted only when it is ignored by a rule in a `.gitignore` that is a
tracked path of `head`, whose bytes `update-index --refresh` has just proven
equal to the verified tree, **and whose pattern field does not begin with `!`**.
A pattern beginning with `!` is a negation: the entry matched it and is **not**
ignored, yet `check-ignore -v -n` prints it as a record with its tracked source
at exit 0 (observed: a tracked root `.gitignore` of `target/`, `*.log` and
`!keep.log` and an untracked `keep.log` give the record
`.gitignore | 3 | !keep.log | :(top)keep.log` while `git status --porcelain`
prints `?? keep.log`), so a negation record is dirty whatever its source. A
pattern that ignores a path literally named with a leading `!` is written `\!`,
and `check-ignore` prints it with the backslash (observed: `\!lit.log`), so the
first byte of the pattern field decides, unambiguously. Any other entry is dirty: one no rule ignores (what
`git status` lists as `??`), one an untracked `.gitignore` ignores (observed:
`sub/.gitignore:1:*` named as the source for `sub/`), and so every untracked
`.gitignore` that hides anything. An untracked `.gitignore` that a tracked rule
ignores is itself admitted, and what it hides is not: its neighbour's source is
the untracked file (observed: tracked root rule `lib/.gitignore` admits the
untracked `lib/.gitignore`, and `lib/new.rs` names `lib/.gitignore:1:new.rs`,
dirty). A `.gitignore` inside a directory a tracked rule ignores is not
examined, because `ls-files -o --directory` collapses that directory to one
entry, and it need not be: git never descends into an excluded directory and
cannot re-include a path under one, so the inner file governs nothing the
check admits (observed: `target/.gitignore` holding `!keep.rs` and `!*` left
`target/keep.rs` ignored by the tracked `target/` rule). That inner file is the
residual named below, and no test rests on it. The scratch has no `info/exclude` and
`core.excludesFile` is `/dev/null`, so the operated repository's own exclude
files are not sources. **A second clause:** an admitted entry that lies under a
changed directory (a directory, other than the repository root, that holds a
path of the delta's set) is dirty too, and the read is whole; this check runs
after step 2, once the set is known. *The residual it names:* a path a tracked
rule ignores at the repository root or outside every changed directory (a
`target/` or a `node_modules/`) is not read, exactly as a whole read does not read
it, and neither is any `.gitignore` inside it; the rule that ignores it is in a
commit, and a rule the branch adds is a changed `.gitignore` whose text the delta
carries.

*A staged-only difference reads clean, correctly.* The check never reads the
operated repository's index. When that index holds bytes that are in neither the
worktree nor `head` (`git status` says `MM`), the worktree still equals `head`,
the tree check proves it (observed: refresh exit 0, `diff-files` empty), and the
gate reads what is on disk and in the commit. The staged bytes are in no commit
and no file a seat runs; if they are committed later, they are in a later head
and in that visit's delta.

A gitlink entry is skipped (`--ignore-submodules`):
its worktree is another repository, whose own configuration would run in a child
process, which is r4's submodule case; a changed gitlink is a whole read by
rule 4 above.

No model computes a delta or chooses which paths to omit. A dirty worktree is a
whole read before step 1 (the untracked rule's second clause, after step 2). A
seat may run git for its own checks, but the engine never reads the seat's diff
as the delta; the commission's diff is the engine's.

#### What the read trusts, and what the work seat can write

The work seat runs before the read, in the operated worktree, and commits; an
unboxed seat is also a process of the operator's user. This is every input the
read uses, and everything in the operated repository the work seat can write,
with the rule or check that covers it. An input the list does not name is a
finding against this decision.

| Input | The work seat can write it? | How the read meets it |
|---|---|---|
| Loose objects (`objects/xx/…`) | yes, by committing, or by replacing a file | the object rule: every object the read or the tree check uses is hashed by the engine; a mismatch is `object_mismatch`; git then reads only verified copies |
| Packs, their `.idx`, `.rev`, `.mtimes`, `.keep` and `.promisor` files, `multi-pack-index`, bitmaps | yes | they only locate objects for the source scratch's `cat-file --batch`; whatever they return is hashed; a `.promisor` file fetches nothing because the scratch has no promisor config; nothing is read through them in the patch or tree scratch |
| `objects/info/commit-graph` and `commit-graphs/` | yes | not consulted: parents and root trees come from the engine's parse of verified commit bytes, and `cat-file` returns object bytes, not graph data |
| `objects/info/alternates` of the operated repository, and its chain | yes | the source scratch follows it to objects only, all hashed; a chain that withholds an object is `read_failed` |
| The worktree's `.git` file, `gitdir` and `commondir` | yes | a path read to find the object directory; whatever directory it names supplies objects the hash judges, or withholds them |
| Refs, `packed-refs` and the operated `HEAD` | yes | no ref is read except `HEAD` through the stable-head check, and that head is the commit under review by definition; a moved head parks before a verdict |
| The default-branch ref (`origin/HEAD`, `main`, `master`) | yes (observed: one `update-ref`) | not read: the base rule drops the base |
| `refs/replace/` | yes | the scratches hold no refs, `GIT_NO_REPLACE_OBJECTS=1` is set, and a replacement's bytes would fail the hash anyway (observed: the planted replace ref shortened only the operated repository's own read) |
| `info/grafts` | yes | read only from a repository's own git dir, which no scratch holds; the engine's parents come from the commit bytes (observed: the graft moved the operated merge-base, the scratch's did not move) |
| `shallow` | yes | no scratch holds one; a shallow history's missing parent is `read_failed` in the walk, so the read is whole |
| The operated index | yes | never read; the tree check builds its own index from verified trees (a staged-only difference is above) |
| The reflog (`logs/`) | yes | never read: every revision is an id |
| Config, hooks, `info/attributes`, `info/exclude`, `info/sparse-checkout` | yes | never read: no git command runs with the operated git dir (r4's matrix; the isolation rule) |
| Worktree files, tracked | yes | the tree check compares their bytes to verified trees |
| Worktree files, untracked, and untracked `.gitignore` files | yes | the untracked rule: admitted only under a tracked ignore rule |
| Worktree `.gitattributes` | yes | not an attribute source of the patch (cwd is the scratch) or the tree check (`--attr-source` is the empty tree) |
| Submodule git dirs (`modules/`) and gitlinks | yes | a gitlink in the set is `delta_incomplete` (rule 4); the tree check skips gitlinks, and nothing runs in a submodule |
| The scratches and their root | no, for a boxed seat; an unboxed seat only by racing a random name made after it is over | the isolation rule and the listing check (`scratch_foreign`) |
| The environment the engine passes to git | no | constructed by the engine, not inherited |
| The journal: the covered head, the previous findings and visit | it can, where the journal lies under the operated root: the fallback journal, opened when neither `--db` nor a map names one, is `<root>/.forge/forge.db` for a library or `<root>/.brokkr/state/forge.db` for a folder (`crates/brokkr-runtime/src/places.rs:155-173`, `:526-535`; `crates/brokkr-cli/src/lib.rs:1237-1238`), and a realm's `journal` may name any path | outside this ruling's threat wherever the journal lies, as it is outside 0034's: a writer that can alter the journal can write the verdict itself, which no read rule can defend, and the read adds no trust in the journal beyond what every transition already places in it |
| The engine binary, `git` on the engine's `PATH`, the user's home | not for a boxed seat (the host is read-only to it); an unboxed seat is a process of the same user and could | outside this decision's threat, with the concurrent same-user process below |

**What it does not defend.** A concurrent process of the same user, or an
unboxed seat writing outside the worktree, could write the scratch between the
listing and the read, the engine's own binary or the `git` on its `PATH`, or the
journal. That is outside this decision's threat: such a process could already
write the operated worktree after the read, replace the engine, or forge a
verdict, and every transition rests on the worktree, the engine and the journal
as they stand. The rules close the work seat as the repository's writer (every
object hashed, no ref but the head under review, no seat-written ignore rule, no
repository configuration) and the repository as a program's author.

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
  `filter.evil.clean` printing the index blob, over a **same-size**
  content-dirty file, printed nothing at exit 0 with an empty stderr, and the
  filter ran (sentinel present); over a **different-size** content-dirty file it
  printed ` M b.txt` (r6): a size difference is caught before the filter is
  asked, so the control must plant same-size content and pin the exit code and
  stderr. The same with a
  `core.fsmonitor` hook that answers "nothing changed" printed nothing while
  `-c core.fsmonitor=false status --porcelain` printed ` M a.txt`; the tree check
  in the tree scratch reported `a.txt: needs update` (exit 1) and listed `a.txt`
  in `diff-files --raw`, with neither the filter nor the hook started (r6: the
  same-size plant gave `b.txt: needs update` and `diff-files` listed `b.txt`, the
  sentinel unchanged).

Observed in r6 (git 2.51.0; ids abbreviated; `since` is `af02c44`, `head` is
`c7dd97d`, and the honest set is `a.txt` and `b.txt`):

- **(a)** `head`'s blob `7a1591a` (`EVIL backdoor`) replaced on disk by the loose
  file of the blob `two fixed nicely`: the r5 per-path patch, in the r5 scratch,
  printed `+two fixed nicely` (one header, one hunk) at exit 0, and `cat-file -e`
  exited 0; `cat-file --batch-check` printed `7a1591a… blob 17`. Re-hashed as the
  object rule does, the bytes are `e3e81f9…`, not `7a1591a…`. Written into a
  patch scratch under the hash of its bytes, the blob is absent at `7a1591a`, and
  the a.txt patch exits 128, "unable to read 7a1591ac…"; b.txt prints its hunk;
- **(b)** `since`'s root tree `6640fb0` replaced by a tree naming the head's
  `a.txt`: the r5 raw set listed `b.txt` only, at exit 0. Re-hashed: `bdb3535…`;
- **(c)** the `since` commit `af02c44` replaced by a commit naming that tree: the
  r5 raw set listed `b.txt` only, at exit 0. Re-hashed: `900fc5f…`; the honest
  `head` commit re-hashes to its own id;
- **(d)** `since` then `E` (adds `x.txt`) then `F` (adds `y.txt`), with `main` at
  `since`: the merge-base of `F` and `main` is `since`. After one
  `update-ref refs/heads/main E`, it is `E`, the `E..F` raw set is `y.txt` alone,
  and the r5 filter keeps `y.txt` from `{x.txt, y.txt}`, every command at exit 0.
  With `main` at `E` before visit 1 (`S'` makes `x.txt` benign; `H'` restores it),
  the merge-base is `E` at both visits, so the refused option (a) of the base
  rule admits it, and the filter drops `x.txt` from `{x.txt, y.txt}`;
- **the replace ref:** `git replace 6640fb0 bdb3535` in the operated repository
  made its own `diff-tree since head` list `b.txt` only; a source scratch over the
  same objects (no refs) listed `a.txt` and `b.txt`;
- **the graft:** an `info/grafts` line giving the default-branch tip `E` as its
  parent moved the operated merge-base of `F` and that tip from `since` to `E`
  (git warned the file is deprecated, and honoured it); in the source scratch the
  merge-base was `since`, and the tip's commit bytes still name `since`;
- **L1:** with an untracked `sub/.gitignore` holding `*` and an untracked
  `sub/new.rs`, `git status --porcelain` printed nothing, and the r5 tree check
  read clean (refresh exit 0, `diff-files` and `ls-files -o --exclude-standard`
  empty); `ls-files -o --directory` listed `sub/` and `target/`, and
  `check-ignore -v` named `sub/.gitignore:1:*` for `sub/` (untracked: dirty) and
  `.gitignore:1:target/` for `target/` (tracked: admitted). That `check-ignore`
  ran without `GIT_LITERAL_PATHSPECS` and without the `:(top)` feed, which r7
  added; the r7 checks below re-ran it as now stated;
- **a staged-only difference:** `git status --porcelain` printed `MM a.txt`; the
  tree check gave refresh exit 0 and an empty `diff-files`;
- **a deleted path:** the patch of a one-line `a.txt` that the head deletes is
  `deleted file mode 100644`, `@@ -1 +0,0 @@`, `-one`.

**No alternative offered:** the engine must own the range and the diff, because a
seat-run `git diff` cannot prove what the seat read. A flag list leaves the
sources in place and asks each new flag to close the last path found open; the
scratch is the simpler form because it removes the sources rather than filtering
them, and a scratch under a tree a seat writes is not removal (r4's placement).
The completeness check is not a flag either: it is what makes a short read
impossible whichever vector shortened it. Hashing every object is not a flag
either: `git fsck` would find the forged loose object (the chief observed "hash-path
mismatch"), but it reads the whole store on every read and names a fault rather
than refusing a read; the engine hashes only the objects the read uses. Cost: two
git helpers and one tree check, three scratch directories per read, the verified
objects, one patch process per changed path (at most `delta_files_gt`, 50 by
default), one plan, and the planted-sentinel matrix.

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
  line in the source scratch and none in the patch and tree scratches, whose
  object files are exactly the ones the engine wrote; an object file planted in
  the patch scratch is `scratch_foreign`; a ref-shaped revision is refused; a
  positive control prints the changed path and its text.
- *Every object is verified.* Each plant runs the read as stated and reads whole
  with the named trigger, and its control runs the r5 statement (git reading the
  operated objects through `alternates`, unverified) and shows the forged text or
  the shortened set at exit 0: **(a)** `head`'s blob replaced on disk by other
  bytes under the same id (`object_mismatch`; control: the forged text);
  **(b)** `since`'s root tree replaced (`object_mismatch`; control: the set lists
  one path where two changed); **(c)** the `since` commit replaced by a forged
  commit naming another tree (`object_mismatch`; control: the shortened set);
  the same for a forged subtree, a forged commit on the ancestry walk, and a
  forged `.idx` that maps an id to another object's bytes; a forged tree under
  the tree check (`worktree_dirty` with no clean proof; control: the r5 tree check
  proves a dirty worktree clean against it); a missing object is `read_failed`
  with no fetch; the engine's set equals git's raw set on an honest repository
  and a planted difference is `delta_incomplete`; the object budget reads whole
  past `ReadLimits.objects`; a SHA-1 collision-attack pair from the sha1dc test
  vectors is a mismatch.
- *No ref the seat moves decides the set.* **(d)** the default-branch ref moved
  to a commit of the branch by `update-ref`: the set is unchanged and complete
  (control: the r5 merge-base filter drops the moved-over path at exit 0); the
  same ref moved before the first visit and left there (control: the refused
  `covered.base` option drops the reintroduced path); **a replace ref** under
  `refs/replace/` (control: the operated repository's own `diff-tree` lists the
  shortened set); **a graft** in `info/grafts` (control: the operated merge-base
  moves); a `shallow` file (the walk is `read_failed` or reaches `since`, never a
  shortened set); a merge of the default branch into the branch puts the merged
  paths in the delta, over-read, never dropped; no `base` is recorded.
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
  shows `git status --porcelain` printing nothing for each, at exit 0 with an
  empty stderr: the filter control plants **same-size** content, because a size
  difference makes `git status` print ` M` before the filter starts, and a
  second control with different-size content pins that it does. A clean worktree is
  proven clean (positive control); `ident`, `eol` and a CRLF checkout read dirty;
  a git that refuses `--attr-source`, a timeout and a refused git each read dirty.
  The untracked rule: an untracked `sub/.gitignore` holding `*` beside an untracked
  `sub/new.rs` reads dirty (control: `git status --porcelain` and the r5 check
  read clean); an untracked `lib/.gitignore` that a tracked rule ignores is
  admitted, and the `lib/new.rs` it hides reads dirty (its source is the
  untracked file); an untracked `keep.log` matched by a tracked negation
  `!keep.log` reads dirty (control: admitted when only the record's source field
  is read), while an untracked `x.log` under the tracked `*.log` is admitted; a
  `target/` ignored by a tracked root rule is admitted; an untracked file ignored by a tracked rule under a changed directory
  reads whole; the operated `info/exclude` is not a source. A staged-only
  difference reads clean.

### 10. The whole-read triggers and their home

**Recommended: (a) a top-level `reads` map in a `forge.phase-machine/v3` table,
one entry per gate phase, with a closed trigger list; an absent entry is a whole
read, a v2 table may not carry `reads`, and a v1 table never opts in.**

| Trigger | Fires when | Refuses |
|---|---|---|
| `first_visit` | the phase has no previous verdict in this run | a delta read for a first read |
| `unkeyed_result` | the previous verdict has no covered identity | a guessed key |
| `clean_prior` | the previous verdict's findings and follow-ups are both empty | a delta on a clean word's evidence |
| `ancestor_lost` | the engine's walk over verified commits does not reach the covered head from the current head within 1,000 commits | a delta across rewritten history |
| `worktree_dirty` | the tree check (ruling 9) does not prove the worktree clean at commission time (a difference, a failure, a timeout, or an untracked path no tracked rule ignores) or the previous visit recorded it dirty | a commit-only delta over uncommitted bytes |
| `scratch_foreign` | a scratch holds anything the engine did not write | a read in a directory a seat or the repository shaped |
| `object_mismatch` | an object's bytes do not hash to the id the engine asked for, or its type is not the one asked | a delta over objects the seat could have forged |
| `read_failed` | a git command failed, timed out or found an object missing, no HOME resolves, or an id is not a commit id | a delta from a read that did not complete |
| `delta_incomplete` | a changed path has no text hunk, a patch names a path outside the set, a changed path is a gitlink, or git's raw set differs from the engine's | a delta that is shorter than the change |
| `artifact_missing` | a declared artifact glob matches no file, or a covered file is gone | a file delta over an absent file |
| `standing_security` | a previous finding with `security: true` still stands | carrying a security finding as a follow-up |
| `unplaceable_finding` | a standing blocking finding has no `file`, or a `file` and `line` the delta cannot place | a finding that no delta can ever close |
| `outside_threshold` | a carried outside-delta finding is at or above the phase's return threshold, or is blocking where no threshold derives | carrying a return-worthy finding as a follow-up |
| `delta_files_gt` | the changed path count is over the declared N | an unbounded diff |
| `delta_bytes_gt` | the diff bytes are over the declared N, or the verified object bytes are over `ReadLimits.objects` (the journaled cause names which) | an unbounded input |
| `delta_touches` | any changed path matches a declared glob | a delta read over a protected surface |
| `operator_whole_read` | a journaled one-shot operator command names the phase | a recipe turning a whole read off |

`ancestor_lost` and `object_mismatch` are head-gate triggers; an artifact gate
compares the covered digests and walks no history, with `artifact_missing` as its
counterpart. No trigger names a base: the delta read takes none (ruling 9's base
rule), so the r5 list's `base_unknown` is gone. `outside_threshold` reads ruling 5's derivation from the
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
   "security-hold"]` (in 0109a) because its charter permits the hold
   (`agents/charters/review-adversarial.md:19-22`);
   `agents/review-security.json` gains the same three words (in 0109a; its
   `security: true` comes in 0109c), because its charter permits the hold (`agents/charters/review-security.md:16-20`) and
   `security: true` alone makes it security-class but does not make its own hold a
   lawful result. `gpt-flash-review-adversarial` and
   `gpt-flash-review-security` inherit both fields through `extends`.
   The list binds **members of a review-panel aggregate only**, as ruling 2 scopes
   its member arm. A review-panel member's compiled result set is its agent's
   list, and the engine refuses a member result outside that set as malformed, so
   the security member's `security-hold` is lawful and its high finding is
   admitted. A member of a `unanimous-pass` panel is outside this rule and keeps
   that aggregate's `pass` and `fail`
   (`crates/brokkr-runtime/src/bundle/aggregate.rs:79-84`), as today, whatever its
   agent declares: triage's design `positions` step
   (`recipes/triage/bundle.json:97-110`) and gpt-flash's
   (`recipes/gpt-flash/bundle.json:283-299`) hire `position-simplicity` and
   `position-robustness` (and their gpt-flash overlays), which declare no list,
   and their lawful word is `pass`, which reaches their chief.
   **Every member agent declares its list in 0109a**, the slice that first binds
   a member's word (ruling 2), because a list that binds must exist for every
   member it binds, and ruling 2's "a correctness member's `security: true`
   finding is refused" can only be built once the member has a result set. The
   agent loader must then admit an agent's `results` at a member site: today an
   agent that declares a member-level key there is refused
   (`crates/brokkr-runtime/src/bundle/site_form.rs:424-436`, `MEMBER_KEYS`), and
   0109a admits `results` alone, read from the hired agent, never from the
   member's own key. 0109c reads the same list for clause 2's class.
   `agents/review-correctness.json` and
   `agents/review-spec-compliance.json`, which declare none today, gain
   `results: ["clean", "residual"]` (their charters forbid a hold:
   `agents/charters/review-correctness.md:19-20`, `review-spec-compliance.md:11-12`);
   the security and adversarial agents' lists are the three words above.
   **A review-panel member whose agent declares no list is held to
   `["clean", "residual"]`**,
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
member's and a spec-compliance member's `clean` is admitted, and a review-panel
member whose agent declares no list is held to `clean` and `residual` (its
`security-hold` is refused); compile and run: both shipped design positions'
`pass`, in triage and in gpt-flash, is admitted and reaches their chief**; a
review-panel member result outside its agent's declared `results` is refused, and a single seat result its agent does not admit
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

The replacement took effect at acceptance (2026-11-17), when 0034 gained the
`Amended by: 0109` back-pointer. No other decision's text changes. 0022,
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
| 0109a | The finding key, the covered head, the v7 result, the word binding to findings and the declared aggregate, and the read-back; no routing change. | a new seat-record v7 schema under `contracts/` (a new version file beside v6, never an edit to v6); a new findings module under `crates/brokkr-core/src/`; `crates/brokkr-core/src/fold.rs`; `crates/brokkr-runtime/src/engine.rs`; `crates/brokkr-runtime/src/engine/decide.rs`; `crates/brokkr-runtime/src/bundle/aggregate.rs`; `crates/brokkr-protocol/src/adapters.rs`; a new `crates/brokkr-view/src/residual_findings.rs`; `crates/brokkr-cli/src/render.rs`; `agents/charters/reviewer.md`; `agents/charters/verify-reviewer.md`; `agents/charters/node-reviewer.md`; `agents/charters/review-chief.md`; `agents/charters/review-correctness.md`; `agents/charters/review-security.md`; `agents/charters/review-adversarial.md`; `recipes/preflight/roles/reviewer.md`; `recipes/review-first/roles/reviewer.md`; `recipes/standby/roles/reviewer.md` (each clean names `none` and `false`; 0109c edits the correctness charter again, for the read kind, on top of this slice); the member result sets (ruling 11): `agents/review-correctness.json` and `agents/review-spec-compliance.json` (`results: ["clean", "residual"]`), `agents/review-security.json` and `agents/review-adversarial.json` (`results` with `security-hold`), `crates/brokkr-runtime/src/agents.rs`, `crates/brokkr-runtime/src/agents/load/` and `crates/brokkr-runtime/src/bundle/site_form.rs` (an agent's `results` admitted at a member site) | about 1,300 lines with tests | seat-record v7 (proposed, minted on acceptance) | v7 closes the result and the finding/covered records with `additionalProperties: false` and `$defs`, and admits `result`, `inputs`, `notes`, `state`, the accounting fields and the three gate report fields; a v6 record with a v7 field refused; v7 variants and the refusal text; a seat-supplied `covered`/`follow_ups` refused; engine-stamped `covered` on the head arm and both `head` field sets; the protected `phase/entered`/`reviewed_heads`/`covered` digest agreement and the digest-mismatch refusal at decide; the new record for a non-protected gate; a moved line, a reworded summary and a moved file; a duplicate id; an id outside the pattern; a seat-supplied `site`; a panel's member findings joined with each member's site; a v6 result reads unkeyed; no path builds a `FindingKey` without a site and an id; the closed non-passing enum marks `residual`, `drift`, `ambiguous`, `clean-unverified`, `fail`, `security-hold`, `upstream` and `escalate` non-passing, `clean`, `pass`, `consistent`, `clear`, `ready` and `shipped` passing, and an unnamed word (an operator's own) passing while its findings still refuse it; `clean` plus a declared high residual and no finding refused; `clean` plus `has_security_residual: true` refused; `clean` omitting a residual input its seat declares refused, and omitting only `spec_defect` admitted; an absent or unknown aggregate severity refused; `clean` from a seat that declares no residual inputs admitted; `clean` plus one high security finding refused, never shipped; `clean` plus a non-security blocking finding refused; `security-hold` plus a high security finding admitted; `residual` plus a blocking finding admitted; `clean` plus an engine-carried `follow_ups` admitted; `consistent` plus a blocking finding refused; a correctness member's `security: true` finding refused and parks; the panel aggregate writes both inputs on an all-clean panel; an all-clean panel admitted; a charter-shaped clean (`clean`, `none`, `false`) admitted; a triage-shaped all-clean panel, built from the member charters' own cleans over a seat that also compiles `spec_defect`, admitted; a work-class panel member's `clean` over a declared high residual refused; a member that omits a residual input refused and one that omits only `spec_defect` admitted; an agent's `results` admitted at a member site and a member's own `results` key still refused; a correctness member's and a spec-compliance member's `clean` admitted (compile and run); the list and its default bind review-panel members only (ruling 11): a review-panel member whose agent declares no list held to `clean` and `residual` (its `security-hold` refused), and, compile and run, triage's and gpt-flash's design `positions` (`unanimous-pass`, agents with no list) each return `pass` from both members and reach their chief; a review-panel member result outside its agent's declared `results` refused, and a single-seat result its agent does not admit refused; the security member's `security-hold` plus a high security finding admitted; `inspect` prints the findings and covered |
| 0109b | The shadow delta: compute the range, paths, diff digest, diff size and standing set on every re-visit, journal the plan and digest it would use, but still commission a full read; the engine-made scratches under the engine's state root, the object rule (verified commits, trees and blobs, the engine's own tree diff and ancestry walk), the base rule (no base, no filter), the completeness check, the tree check with the untracked rule, and the planted matrix; the per-visit record and `compare()`. | a new delta/set module under `crates/brokkr-core/src/`, with the pure commit and tree parsers, the tree diff and the collision-detecting object hash; `Cargo.toml`, `Cargo.lock`, `crates/brokkr-core/Cargo.toml` and `crates/brokkr-runtime/Cargo.toml` (the new `sha1collisiondetection` dependency; the deflate crate at 1.1.10, in the lockfile today, as a direct dependency of the runtime); a new `crates/brokkr-runtime/src/engine/delta_git.rs` (the `git_diff`/`git_query` helpers, the verified object reader and writer, and the completeness check); a new `crates/brokkr-runtime/src/engine/delta_scratch.rs` (make, list, compare and remove the scratches); `crates/brokkr-runtime/src/engine/broker.rs` (the protected walk takes the reads root's names beside `PROTECTED_ROOT`); `crates/brokkr-core/src/fold.rs`; `crates/brokkr-runtime/src/engine/input.rs`; `crates/brokkr-runtime/src/engine.rs`; `crates/brokkr-runtime/src/engine/worktree.rs`; `crates/brokkr-view/src/analytics.rs`; a new gate-read v1 schema under `contracts/` (paths as hex byte strings) | about 1,500 lines with tests | gate-read v1 (proposed) | a first visit has no shadow and journals `first_visit`; a v6-shaped previous result forces a full read with `unkeyed_result`; a clean previous verdict forces `clean_prior`; a second visit names the exact paths; a merge of the default branch into the branch puts the merged paths in the set; an empty delta; a rewritten history (`ancestor_lost` from the verified walk); all three caps with explicit `ReadLimits` values; ruling 9's six test groups, each plant with its control and each for `git_diff` and `git_query` alike: **the scratch is the engine's** (a scratch path planted before the read with a promisor config, a `diff.<driver>.command` and an `info/attributes` makes the read whole with `scratch_foreign` and no sentinel; the control runs the planted uploadpack without the exactness check; an existing name is refused; the scratch is never under the run-local tree or the worktree; its shape is no config, `HEAD` names no commit, an empty `refs/`, one `alternates` line in the source scratch and none in the patch and tree scratches, an object file the engine did not write refused, a ref-shaped revision refused, a positive control printing the changed path and text); **every object is verified** (plants (a), (b) and (c), a forged subtree, a forged walk commit, a forged `.idx` and a forged tree under the tree check each read whole with `object_mismatch` or a dirty tree, each control showing the forged text or the shortened set at exit 0 under the r5 statement; a missing object is `read_failed` with no fetch; the engine's set equals git's raw set and a planted difference is `delta_incomplete`; the object budget; a sha1dc collision vector is a mismatch); **no ref the seat moves decides the set** (plant (d) by `update-ref`, the same ref moved before the first visit, a replace ref, a graft and a `shallow` file, each leaving the set complete, each control showing the r5 filter or the operated repository's own read shortened at exit 0; no `base` recorded); **complete or whole** (a `:(exclude)*` file, a name with a newline, one with a double quote, one with a leading `:`, a committed `-diff` and a `binary` attribute, each with the cwd inside and outside the operated worktree, the text in the delta or the read whole, and a control showing each blanks or shortens the patch without the rule; a mode-only change, an empty added file, a binary file, a gitlink, a symlink, a deleted file and a rename; an empty set runs no `diff-tree -p`; a path with no hunk and a path outside the set give `delta_incomplete`; the byte budget stops a patch); **the repository names no program** (a `diff=` driver, a textconv, a `diff.<driver>.command`, the submodule `diff.submodule=diff` case with the submodule's own `diff.external`, the promisor case with `extensions.partialClone`/`remote.<n>.promisor`/`uploadpack` failing closed, `filter.<driver>.clean` and `.process`, `core.fsmonitor`, `core.hooksPath`, an `info/attributes` driver, an environment-carried `GIT_EXTERNAL_DIFF`/`GIT_CONFIG_*`, and an alternates chain whose target names a driver); **the tree check is not the repository's** (the lying clean filter and the fsmonitor that reports nothing changed each make the read whole with the filter and the hook never started, the control showing `git status --porcelain` print nothing at exit 0 with an empty stderr over **same-size** content, and a different-size control printing ` M`; a clean tree is proven clean; `ident`, `eol` and a CRLF checkout read dirty; a refused `--attr-source`, a timeout and a refused git each read dirty; the untracked rule: an untracked `sub/.gitignore` holding `*` beside `sub/new.rs` reads dirty, its control reading clean, an untracked `lib/.gitignore` that a tracked rule ignores admitted while the `lib/new.rs` it hides reads dirty, a tracked root `.gitignore` of `target/`, `*.log` and `!keep.log` with an untracked `keep.log` reads dirty by the negation (its record's pattern field `!keep.log`; control: the same record admitted when only the source field is read, while `git status --porcelain` prints `?? keep.log`) and an untracked `x.log` beside it is admitted, a tracked `\!lit.log` rule admits an untracked `!lit.log`, a `target/` under a tracked root rule is admitted, each pinned under `check-ignore`'s own production environment (its control: the same feed under the patch's environment exits 128 and reads dirty by failure, not by the rule), an untracked `:(top)target/` and `:target/` read dirty (their control: fed without the `:(top)` prefix, each is reported ignored by `target/` at exit 0), a missing, extra or renamed `check-ignore` record reads dirty, a tracked-rule-ignored file under a changed directory reads whole, the operated `info/exclude` is no source; a staged-only difference reads clean); the shadow journal carries the plan and the diff digest but no diff text; the payload plan and the input projection cannot disagree; a mutated payload fails `read_digest`; a recovery rebuilt from the journal yields the same input; a new event is refused; a shadow-era journal never counts a full read as a delta; `GateVisit` records `findings` and `follow_ups`; `compare()` over the union pins equal, moved, reworded, closed and only-grew; a fold test rebuilds `GateVisit` from the journal with both arrays; a follow-up carried in the first visit is still carried or closed by a covering read in the second; the potential saving in `analytics` |
| 0109c | The read policy and the triggers: a phase-machine v3 `reads` map, the closed trigger enum, the glob matcher, the security-class derivation and its refusal, the spec file-level plan, the artifacts arm and derived kind, the carry rule and the locator translation, and a delta commission for one non-security gate (the analyze gate first); the v3 move of fast and every table that composes over it; the delta-eligible charters and the prompt text that names the read kind and the delta. | a new phase-machine v3 schema under `contracts/`; `crates/brokkr-core/src/policy.rs` and its condition module; 0109b's delta/set module under `crates/brokkr-core/src/` (the pure locator translation over the hunk ranges); a new glob module under `crates/brokkr-core/src/`; `crates/brokkr-runtime/src/engine/input.rs`; `crates/brokkr-runtime/src/engine.rs`; `crates/brokkr-runtime/src/bundle/` (the security-class derivation, reading 0109a's `results` lists for clause 2, and the artifacts arm); `crates/brokkr-runtime/src/agents.rs` and `crates/brokkr-runtime/src/agents/load/` (the agent loader's `security` field and `OVERLAY_KEYS`); `agents/review-security.json` (`security: true`; its `results` came in 0109a); `agents/charters/review-correctness.md`; `agents/charters/review-spec-compliance.md`; `agents/charters/analyst.md` (names the read kind and the delta); `crates/brokkr-protocol/src/adapters.rs`; `recipes/fast/policy.json` (moves to v3 with no `reads` entry yet); `recipes/triage/policy.json` (v3 with the analyze `reads` entry); `recipes/node/policy.json`, `recipes/review-first/policy.json`, `recipes/landing/policy.json`, `recipes/panel-review/policy.json` (their own v2 tables move to v3 with fast, because one composition shares one schema) | about 900 lines with tests | phase-machine v3 (proposed) | every trigger, including `outside_threshold` and its derived threshold; an unknown trigger refused; the analyze gate keeps its inputs (ruling 5): a charter-shaped `drift` with `drift_in: design` and a medium finding, and no residual pair, is admitted and routes to `design`, and `consistent` plus a blocking finding is refused; a v1 table read as it is and never opted in; a v2 table carrying `reads` refused; an absent `reads` is full; the analyze gate's delta and its whole-read fallbacks; the analyze judge is delta-eligible; the spec file-level delta (a changed artifact file read whole, an unchanged one not re-read, a new match read whole, a missing file a whole read); the spec re-arm rule; the artifacts arm records each matched file's digest and the read plan carries the glob set; the derived kind; the dirty refusal; the carry rule: an outside-hunk finding is carried, not closed; the locator translation (the three-visit insert-above, carry, unrelated-edit test of ruling 5, with its control, and a translation past either blob's end recorded unplaceable without a line); and a blocking outside finding where no threshold derives forces a whole read with `outside_threshold`; a fileless finding stays standing, and a standing blocking one forces the next visit whole with `unplaceable_finding`, whose omission then closes it; a finding on a path the delta deletes is inside (the old-side range of its one hunk) and the deleting read's omission closes it; the taken delta's diff text is journaled, capped, and recovery reads it rather than a fresh `git diff`; the security-class derivation for the declared result set, the sole review position and the typed agent office; a delta mark refused on each; a member key or agent file name does not decide; the shipped whole-read set; a panel whose phase can end in `security-hold` with an undeclared security slot and no other class member whole-reads; the adversarial member is security-class through 0109a's `results` list; the security member is security-class through `security: true`; the analyze `reads` entry names `judge`, and the unnamed `check` reads whole; an overlay cannot clear `security` and an overlay can set it; the glob matcher; a `select`-case tag resolves to one site and an ambiguous bare tag refuses; triage's design and engine `positions` steps are not class as containers; gpt-flash's work-class review members are bound by ruling 2 and name no delta; the correctness, spec-compliance and analyst charters bind the read kind and the delta; a security-class charter keeps its whole read; `recipes/triage`, `recipes/fast`, `recipes/node`, `recipes/review-first`, `recipes/landing` and `recipes/panel-review` compose after the v3 move |
| 0109d | Review panels, standing security and follow-ups: the engine rejects an understated or false aggregate, whole-reads a security position, closes a follow-up only by a covering read, gives the one single-review agent both residual inputs, and names the panel and preflight read policies. | `crates/brokkr-runtime/src/bundle/aggregate.rs`; `crates/brokkr-runtime/src/engine/decide.rs`; `crates/brokkr-runtime/src/engine/input.rs`; `crates/brokkr-core/src/fold.rs`; `agents/brokkr-reviewer.json` (both residual inputs, one home; self and release inherit); `recipes/panel-review/policy.json`; `recipes/preflight/policy.json` (each v3, naming its read policy; panel-review's table moved in 0109c) | about 650 lines with tests | none new; v7 admits `follow_ups` from 0109a | a fileless finding stays standing and an inside-delta read's omission does not close it; a blocking outside-delta finding with a derived threshold forces a whole read with `outside_threshold`; a seat cannot declare a closure and a seat-supplied `follow_ups` is refused; a follow-up closes only by a covering read; a carried follow-up is not closed; a standing security finding blocks and forces full; a non-empty `findings` with a residual input the seat declares absent refused; the understated-severity refusal; the false-security refusal; a mixed panel's security member whole-reads and its correctness and spec-compliance members delta-read; a panel's follow-ups are named; self and release compile with both residual inputs; fast's verify and ship gates are unopted (no `reads` entry), so the absent-entry rule keeps them whole-read, and the commission carries no delta; two consecutive delta visits, with a finding carried in the first still carried or closed by a covering read in the second |
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
first. The agent loader (`crates/brokkr-runtime/src/agents.rs`,
`agents/load/`) and `agents/review-security.json` are edited by 0109a (the
`results` lists) and again by 0109c (`security: true` and `OVERLAY_KEYS`), in
that order. `Cargo.toml` and `Cargo.lock` move in 0109b for one new dependency;
a sibling branch that moves the lockfile is rebased over it, one branch at a
time.

## Consequences

The saving is the adopter's figure, not a promise: if a review re-visit reads only
the fix diff and the standing findings, the 29% code-review share and the 55% spec
share have room to fall where the changes are small. The cost is a contract
version, a new read plan in every gate's request, a delta computation that runs
git in the engine, three scratch git directories per read under the engine's own
state root, every object the read rests on read and hashed by the engine (both
versions of each changed file, and every tree of the head for the tree check),
one patch process per changed path, a tree check that hashes every tracked file
once, one new dependency (collision-detecting SHA-1), and triggers a recipe
author must choose. The refusals that keep the saving honest are that a
security-class position and a standing security finding never take the delta, a
clean verdict never licenses one, a delta that is shorter than the change is a
whole read, no object is trusted without its hash, no ref the seat can move
decides the set, and nothing a seat can write is an input to the read without a
check. The price of those refusals is a whole read wherever the check cannot
prove the tree (an LFS or CRLF checkout, a git older than 2.40, a SHA-256
repository, a mode-only or binary change), and a re-read of every path a merge
of the default branch brought in, because the base filter is gone.

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
worktree, a forged or missing object, or a lost ancestor forces a whole read, so
some re-visits save nothing: a delta that cannot prove its range is not a delta. The readout counts only
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

### Checks run for the 1088r6 round

Run on git 2.51.0 in scratch repositories under `/tmp`, never in this worktree.
The implementer's shell admits git but no script, no `mkdir`, no `cp` and no
environment prefix, so every repository was built with git alone: content files
and `mktree` inputs written under `.forge/` (not committed), objects made with
`hash-object -w`, `mktree` and `commit-tree` (so every object is loose), and a
file planted at an arbitrary path by staging it in a carrier repository with
`update-index --cacheinfo` and writing it with
`git --git-dir=<carrier> --work-tree=<target> checkout-index -f`. A forged loose
object is the loose file of another object, carried that way to the path of the
id it forges. Scratches are `git clone --bare --shared` with `remote.origin` and
`core` removed and no refs. Sentinels append to files under `.forge/`, read back
afterwards. The commands ran in the session's own environment, not the
constructed one, which the shell cannot set; the read's configuration-free shape
is the scratch's, and the r4 panel ran the environment rows. The object rule's
hash is emulated as `git cat-file <type> <id> | git hash-object -t <type> --stdin`
(git's hash is sha1dc too), and its verified copy as `hash-object -w` into a
scratch with no `alternates`; the listing check and the engine's own tree diff
are code, not run here. Ids are abbreviated; `S` is `af02c44`, `H` is `c7dd97d`.

```text
(a) H's blob 7a1591a replaced by the loose file of e3e81f9 ("two fixed nicely"):
  r5 patch -- a.txt, r5 scratch:      "+two fixed nicely", exit 0
  cat-file -e 7a1591a:                exit 0;  --batch-check: "7a1591a... blob 17"
  rule: cat-file blob 7a1591a | hash-object --stdin:      e3e81f9...  (mismatch)
  rule: verified copy in a no-alternates scratch, patch -- a.txt:
                                      exit 128 "unable to read 7a1591ac..."
                                      patch -- b.txt: the "+two v2" hunk, exit 0
(b) S's root tree 6640fb0 replaced (forged tree names H's a.txt):
  r5 raw set S H:                     b.txt only, exit 0   (honest: a.txt, b.txt)
  rule: cat-file tree 6640fb0 | hash-object -t tree --stdin:   bdb3535... (mismatch)
(c) commit S replaced by a commit naming that forged tree:
  r5 raw set S H:                     b.txt only, exit 0
  rule: cat-file commit S | hash-object -t commit --stdin:     900fc5f... (mismatch)
        the same for H:               c7dd97d... (its own id)
(d) S -> E (adds x.txt) -> F (adds y.txt); main at S:
  merge-base F main:                  af02c44 (S)
  update-ref refs/heads/main E; merge-base F main:   b584b5c (E)
  raw S F: x.txt, y.txt;  raw E F: y.txt         -> the r5 filter keeps y.txt only
  main at E before visit 1; S' = E + x.txt benign; H' = S' + x.txt evil + y.txt:
  merge-base S' main = merge-base H' main = b584b5c (E), equal at both visits
  raw S' H': x.txt, y.txt;  raw E H': y.txt      -> option (a) drops x.txt
replace ref: replace -f 6640fb0 bdb3535 in the operated repository
  operated diff-tree S H:             b.txt only, exit 0
  source scratch over the same objects (no refs):  a.txt and b.txt
graft: M1 = S + nothing, main at M1; info/grafts "M1 E"
  operated merge-base F main:         b584b5c (E), with git's deprecation hint
  source scratch merge-base F M1:     af02c44 (S);  M1's bytes rehash to M1
L1: untracked sub/.gitignore "*", untracked sub/new.rs, tracked .gitignore "target/",
    untracked target/debug/foo.o
  git status --porcelain:             nothing;  --ignored: "!! sub/", "!! target/"
  r5 tree check:                      refresh exit 0; diff-files, ls-files -o --exclude-standard empty
  ls-files -o --directory:            sub/, target/
  check-ignore -v -n -z --stdin:      sub/.gitignore:1:*  sub/;  .gitignore:1:target/  target/
                                      (session environment, no literal pathspecs, no
                                      ":(top)" feed; the stated form is in the r7 checks)
staged-only: update-index --cacheinfo of another blob for a.txt
  git status --porcelain:             MM a.txt
  tree check:                         refresh exit 0; diff-files empty
deleted path: S -> D (deletes a.txt)
  patch -- a.txt:                     deleted file mode 100644; @@ -1 +0,0 @@; -one
H4, info/attributes "*.txt filter=evil", filter.evil.clean prints the index blob:
  different-size b.txt, git status --porcelain:  " M b.txt", exit 0
  same-size b.txt ("TWO V2" for "two v2"):       nothing, exit 0, empty stderr; filter ran
  tree check:                         "b.txt: needs update"; diff-files lists b.txt; filter not run
r4 matrix in one operated repository (info/attributes "a.txt diff=evil filter=evil",
  "b.txt diff=tv filter=evp"; diff.evil.command, diff.external, diff.tv.textconv,
  filter.evil.clean, filter.evp.process, core.fsmonitor, core.hooksPath):
  new statement (source scratch cat-file, verified patch scratch, patch with cwd
  the scratch for a.txt and the operated worktree for b.txt; tree check in a
  verified tree scratch):   both hunks, refresh exit 0; no sentinel file at all
  controls: status:          fsmonitor hook, clean, process
            log -p --ext-diff: diff.evil.command, process
            log -p --textconv: textconv
            commit:            post-commit hook
promisor: partial clone (blob:none), uploadpack = sentinel
  source scratch cat-file --batch-check 7a1591a:  "missing"; no sentinel
  operated cat-file -t 7a1591a:       blob; sentinel PRESENT (lazy fetch ran)
  H1, the same config planted in the source scratch: cat-file --batch-check fetched;
                                      sentinel PRESENT twice (the listing check refuses it)
H2/H3 in the verified patch scratch:
  ':(exclude)*' without --literal-pathspecs: 0 bytes; with: the hunk
  ':lead.txt' without: 0 bytes; with: one hunk;  'q"uote.txt' with: one hunk
  "*.txt -diff" in the operated worktree, cwd the worktree:  Binary files ... differ
                                       cwd the scratch:      the text hunk
cost, this tree:  ls-tree -r -d HEAD | cat-file --batch-check: 692 trees, 128,981 bytes,
                  plus the root tree, 1,454 bytes; no 120000 entry
                  b486d3ffd..0af190c64: 10 objects, 233,800 bytes (batch-check sizes)
```

**Not re-run in r6:** `diff.external` did not fire in the controls, because the
process filter aborted `log -p --ext-diff` after `diff.evil.command` fired; the
r4 panel's row stands as theirs. The submodule child diff, `GIT_EXTERNAL_DIFF`
and `GIT_CONFIG_*` were not run (the shell sets no environment); a gitlink is a
whole read by rule 4 before any child could run. The forged `.idx`, the forged
subtree, the forged walk commit and the sha1dc collision vector are 0109b's
plants, not run here. The plant with an ignored file under a changed directory
is 0109b's too. An ignored untracked `.gitignore` is admitted, and what it hides
is dirty (the next block's `lib/.gitignore` rows); a tracked negation reads dirty
(the last block, for fix round 1088r7's M1, M2, L1 and L2).

### Checks run for the 1088r7 round

Run on git 2.51.0 in scratch repositories under `/tmp`, never in this worktree,
built as in r6 (content files under `.forge/`, carried by a carrier git dir's
`checkout-index`; the shell runs git and no script). The tree scratch is a bare
git dir with its `core` keys removed and `head`'s two trees and three blobs
written into it by `cat-file | hash-object -w`; every command below ran with the
scratch as git's working directory (`-C`). The constructed environment is not
settable from this shell, so `GIT_LITERAL_PATHSPECS=1` was given as git's
equivalent option `--literal-pathspecs`; the other variables were not set.

```text
tree: tracked .gitignore "target/" (later "target/\nlib/.gitignore"), a.txt, lib/a.rs
untracked: sub/.gitignore "*", sub/new.rs, target/debug/foo.o, target/.gitignore,
           ":(top)target/evil.rs", ":target/x.rs", "targe*/y.rs", lib/.gitignore "new.rs", lib/new.rs
read-tree, update-index --refresh: exit 0;  diff-files --raw -z: empty
ls-files -o --directory -z:  :(top)target/ :target/ lib/.gitignore lib/new.rs sub/ targe*/ target/
check-ignore under the patch's environment (--literal-pathspecs), fed sub/ target/:
    fatal: sub/: pathspec magic not supported by this command: 'literal'  (exit 128)
check-ignore without it, fed the bare entries:
    .gitignore 1 target/   :(top)target/       <- a seat-named directory admitted
    .gitignore 1 target/   :target/            <- likewise
    sub/.gitignore 1 *     sub/
    .gitignore 1 target/   target/
    (empty)                targe*/
check-ignore without it, fed ":(top)" + each entry (the stated form):
    (empty)                        :(top):(top)target/   dirty: no rule
    (empty)                        :(top):target/        dirty: no rule
    .gitignore 2 lib/.gitignore    :(top)lib/.gitignore  admitted (tracked rule)
    lib/.gitignore 1 new.rs        :(top)lib/new.rs      dirty: untracked source
    sub/.gitignore 1 *             :(top)sub/            dirty: untracked source
    (empty)                        :(top)targe*/         dirty: no rule
    .gitignore 1 target/           :(top)target/         admitted (tracked rule)
    every record's path field byte-equal to the entry fed, in order
target/.gitignore "!keep.rs\n!*" with an untracked target/keep.rs:
    status --porcelain --ignored -uall: "!! target/keep.rs"; ls-files -o --directory: target/
locator: f = 50 lines, finding at line 10
    v1 -> v2 (20 lines prepended):     @@ -1,3 +1,23 @@     carried: 10 + 24 - 4 = 30
    v2 -> v3 (current line 10 edited): @@ -7,7 +7,7 @@      carried: 30 (outside), stays 30
    grep -n: v1 line 10, v2 line 30, v3 line 30;  the stale 10 lies inside -7,7
```

The hunk headers were read with `git diff --no-index --no-ext-diff -U3` over the
three files, not with the stated `diff-tree` in a scratch; the arithmetic is the
hunk header's, which both print alike. **Not re-run in r7:** 1088r4's program
matrix, r5's H1 to H4 and r6's (a) to (d), the replace ref, the graft and the
staged-only and deleted-path plants: this round changes the tree check's last
command and ruling 5's carried locator, and neither touches the object reads, the
patch, its environment or the refs those plants exercise. The r6 review panel ran
them under the r6 statement, which this round leaves as it was.

### Checks run for fix round 1088r7 (M1, M2, L1, L2)

M1 and M2 are text scopes, read from the shipped data, not run: the design
`positions` steps are `unanimous-pass` (`recipes/triage/bundle.json:97-110`,
`recipes/gpt-flash/bundle.json:283-299`), `agents/position-simplicity.json` and
`agents/position-robustness.json` declare no `results`, their gpt-flash agents
only `extends` them, and `Aggregate::UnanimousPass` requires `pass` and `fail`
(`crates/brokkr-runtime/src/bundle/aggregate.rs:79-84`); analyze declares only
`drift_in` (`recipes/triage/bundle.json:133-135`), `agents/analyst.json` declares
no inputs, and `decide.rs:82` keeps only declared names. L1 and the r6 plants ran
on git 2.51.0 in scratch repositories under `/tmp`, built as in r6, with
`commit.gpgsign=false` for the scratch commits. The tree scratch is a bare git
dir with `core` removed and `head`'s tree and two blobs written by
`cat-file | hash-object -w`; every command ran with the scratch as git's working
directory, `--literal-pathspecs` standing in for `GIT_LITERAL_PATHSPECS=1` on the
first four commands and absent on `check-ignore`.

```text
L1: tracked .gitignore "target/\n*.log\n!keep.log", a.txt; untracked keep.log
  git status --porcelain:            ?? keep.log
  read-tree; update-index --refresh: exit 0;  diff-files --raw -z: empty
  ls-files -o --directory -z:        keep.log
  check-ignore -v -n -z, fed ":(top)keep.log":
      .gitignore | 3 | !keep.log | :(top)keep.log      exit 0
  without the rule (source field only): tracked source -> admitted, reads clean
  with the rule (pattern begins "!"):   dirty, the read is whole
  then untracked x.log too; status --porcelain --ignored: "?? keep.log", "!! x.log"
      .gitignore | 3 | !keep.log | :(top)keep.log      dirty (negation)
      .gitignore | 2 | *.log     | :(top)x.log         admitted (tracked rule)
escaped: tracked .gitignore "\!lit.log"; untracked "!lit.log" (operated repo, cwd it)
  status --porcelain --ignored:      "!! !lit.log"
  check-ignore -v -n -z:             .gitignore | 1 | \!lit.log | :(top)!lit.log
r6 (a) to (d), S = bcca55e, H = eab5ce5 (a.txt and b.txt change); honest set a.txt, b.txt
  (a) H's blob 9c98f36 replaced by the loose file of 37b12ab:
      diff-tree -p S H -- a.txt:     "+benign looking", exit 0;  cat-file -e: exit 0
      rule: cat-file blob | hash-object --stdin:  37b12ab... (mismatch)
  (b) S's root tree 6640fb0 replaced by a forged tree naming H's a.txt:
      raw set S H:                   b.txt only, exit 0
      rule: rehash:                  a135d62... (mismatch)
  (c) commit S replaced by a commit naming that forged tree:
      raw set S H:                   b.txt only, exit 0
      rule: rehash S: 84200f7... (mismatch);  rehash H: eab5ce5... (its own id)
  (d) S -> E (adds x.txt) -> F (adds y.txt); main at S:
      merge-base F main:             bcca55e (S)
      update-ref refs/heads/main E;  merge-base F main: e2a888f (E)
      raw S F (the stated set, no filter): x.txt, y.txt
      raw E F (the r5 filter's base):      y.txt only, exit 0
```

The four still hold: each control shows the forged text or the shortened set at
exit 0, the object rule's rehash refuses (a) to (c), and the base rule's
unfiltered `since..HEAD` set keeps `x.txt` in (d). The escaped case ran in the
operated repository itself, not a tree scratch; it shows only how `check-ignore`
prints a `\!` pattern. **Not re-run in this round:** the replace ref, the graft,
the shallow file, 1088r4's program matrix and r5's H1 to H4; this round changes
no read, environment or ref rule, and the r6 review panel re-ran them.
