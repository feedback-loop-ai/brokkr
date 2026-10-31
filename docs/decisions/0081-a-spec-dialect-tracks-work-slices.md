# 0081 — A spec dialect tracks work slices; the decisions are the law

Status: accepted (operator ruled in chat, 2026-10-10, on #367)
Date: 2026-10-10

Built: built
Supersedes in part: 0042
Keeps the minting of: contracts/dialect.v1.schema.json
Keeps the minting of: contracts/dialect.v2.schema.json
Keeps the minting of: contracts/dialect.v3.schema.json

## Context

Decision 0042 made the spec-driven dialect the realm's and gave this
repository OpenSpec. Its addendum of 2026-09-04, ruling 2, went further:
the fold runs one way, the decision record is a stream of deltas, and
`openspec/specs/` is the truth that stream accumulates into, seeded from
the accepted decisions and held by a test that refuses a capability
specification citing no decision. Its addendum of 2026-09-06 read a
capability spec as standing truth. Neither reading was built.

Issue #367 (epic #330) measured what the tree held on 2026-09-25:

- `openspec/` was 97 files and 67,232 Markdown lines, 66% of all tracked
  Markdown. One change's task ledger was 14,751 lines for 152 tasks.
- No page said what `openspec/` is: not the repository layout,
  `ARCHITECTURE.md`, `README.md` or `CONTRIBUTING.md`.
- A shipped change, `2026-09-17-bound-transcript-projector` (#298), was
  never archived, and its added requirement stood in no capability spec.
- The capability specs covered 15 capabilities, all written from
  2026-09-06 on. No decision before 0046 was folded in: the journal, the
  fold, policy, recipes, agents, secrets, the anchor and the seat record
  have no spec. Several specs rest on decisions still proposed, the
  citation test the addendum promised does not exist, and the specs still
  required Windows after decision 0063.

The issue offered two readings. (a) `openspec/specs/` is the behaviour
contract: backfill a spec for every earlier decision, make each
`ARCHITECTURE.md` section cite its spec, and add the citation and fold
checks. (b) OpenSpec only tracks slices of work: say so, and stop calling
it the truth. Reading (a) restates every ruling a second time, in a second
place, which decision 0071 ruling 5 forbids and which drifts: the specs
had already drifted from the decisions and the code they cite. The
operator ruled (b), for both dialects.

## Rulings

1. **A spec dialect tracks work slices only.** OpenSpec and spec-kit
   track slices of work. Neither dialect's artifacts are a realm's living
   truth: not `openspec/specs/` and not an OpenSpec change, and not
   spec-kit's `specs/<feature>/` spec, plan and tasks. A change is the
   record of one slice: what it proposed, how it was designed, what it
   tasked and what it found. The capability tree an archive folds into
   is the tool's record of the requirements archived slices left, and is
   no contract.

   **Enforcement binding:** judgment guidance for prose, stated as such.
   The product says it in its own words: the OpenSpec archive instruction
   (`dialects/openspec/archive.md`), the SDD smith's charter
   (`agents/charters/implementer-sdd.md`) and spec-kit's `truth` reason in
   `dialects/speckit.json`. `status_pages::is_record` names all of
   `openspec/` a record, so the living-doc checks of #365 hold the living
   stack and leave a slice's records their history.

2. **Where the truth lives.** The decisions are the law, and their index
   is the ledger (#363). `ARCHITECTURE.md` is the as-built map (#364).
   The code and its tests are the executable contract, and the living
   docs are held true to them (#365). Where a dialect's artifact disagrees
   with a decision or the code, the decision and the code win, and the
   artifact is a record of what a slice once said.

   **Enforcement binding:** `crates/brokkr-cli/tests/decisions_index.rs`
   for the ledger and `crates/brokkr-cli/tests/doc_truth.rs` for the living
   docs; both exist.

3. **A slice ends at its archive.** A change is archived when its work is
   done, every task in its task list ticked, by the dialect's archive
   step, which stays the slice's last task: OpenSpec's archive operation
   and its provenance line. An evidence ledger belongs in `docs/evidence/`
   or a run's `refs/forge` anchor, not in a change directory.

   **Enforcement binding:** `crates/brokkr-cli/tests/openspec_retention.rs`
   refuses a change under `openspec/changes/` outside the archive whose
   `tasks.md` has every task ticked, and refuses one whose task list it
   cannot read. Where an evidence ledger lives is judgment guidance; the
   ledgers the live changes already carry stay where they are until each
   change is archived.

4. **What this supersedes, and what stands.** This decision supersedes,
   in part, decision 0042: its addendum of 2026-09-04, ruling 2 (the
   one-way fold into `openspec/specs/` as the truth, the truth seeded from
   the accepted decisions, and the test refusing a specification that
   cites no decision, never built), and its addendum of 2026-09-06 in its
   reading of a capability spec as standing truth. What stands: decision
   0042's rulings, with ruling 7's OpenSpec as this repository's
   work-slice format; the 2026-09-04 addendum's ruling 1, that the
   decision is the authority, no spec artifact amends one, and only a
   superseding decision changes a ruling; its ruling 3, the dialect's
   installer; and the 2026-09-06 addendum's rulings 1 and 2, the
   provenance line the archive step appends, now read as a capability's
   history rather than its truth.

## Consequences

- No decision is backfilled into a spec, and no `ARCHITECTURE.md` section
  cites one.
- `docs/guides/repository-layout.md` and `ARCHITECTURE.md` say what
  `openspec/` is and give the retention rule.
- `2026-09-17-bound-transcript-projector` is archived and its deltas are
  folded, its added requirement among them. The four live changes stay:
  slice two of decision 0065 is another lane's live work, slice one's
  archive is the operator's call, `boundary-seatbelt-slice-ii` waits on
  #269, and #226's session resumption is unfinished.
- The capability specs drop their Windows requirements (#356) and the
  retired `/api/run/<id>` read surface (#351), so the record agrees with
  the code as it is archived from here on.
- The dialect contract's `truth` field keeps its name: it names the tree
  an archive folds into, and a contract changes only by a new version.
