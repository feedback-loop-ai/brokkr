# 0059 — Muninn reads the crossings the map draws: the dossier is journal facts plus one already-computed crossing report

Status: proposed
Date: 2026-09-10

## Context

Decision 0020 is accepted, and two of its rulings bound Muninn's evidence
tighter than Phase 2 slice (vi) now needs:

- Ruling 1 says Muninn "reads only what the journal derives": its
  worldview is the `forge-view` models, it opens the store read-only, it
  reads no repository tree, receives no secrets and is handed no working
  directory to change.
- Ruling 3 says every proposal cites the journal facts it was derived
  from — run ids and sequence numbers.

Decision 0057 (proposed, 2026-09-09) ruled the crossing vocabulary only.
Its ruling 5 says in as many words that nothing is read, resolved or
verified in that slice, and its Phase 2 close-out addendum records what
the later slices built while changing no ruling above. Phase 2 slice (vi)
then built the readouts. `brokkr muninn run` now carries the world's one
crossing report into the fleet dossier — per mapped realm, what it
publishes (each with its declared repository-relative path) and what it
consumes (each with its publishing realm and its pin state `matching`,
`moved` or `unchecked`) — with a moved pin raised as a finding of the
CONSUMING realm and citable by that realm and the crossing's name. A
world whose mapped realms have not run yet still yields a crossing
dossier, because the report comes off the map. A matching or unchecked
pin is stated and is not a finding, and the crossing entries are
snapshotted into Muninn's append-only record so the evidence survives a
later map change.

That behaviour is commissioned and correct, and it stays. But it widens
0020's evidence source (map- and filesystem-derived crossing observations
beside journal-derived models) and its citation rule (a `(realm,
crossing)` pair beside a `(run_id, seq)` pair). Decision 0042 ruling 1,
and this directory's own law, is that only a superseding numbered
decision changes what an accepted ruling means: a capability
specification, an implementation account or a close-out addendum cannot.
Decision 0057 says in terms that it rules the vocabulary only. So the
delta has no ruling behind it until this decision is proposed.

## Decision

1. **The journal stays first, and the world loader's one crossing report
   is admitted beside it.** Muninn's worldview is the journal-derived
   `forge-view` models **plus** the crossing report the world loader has
   already computed for the map by reading each publication in the
   publishing realm's own tree: per realm, the files it publishes — each
   with its declared repository-relative path — and the crossings it
   consumes — each with its publishing realm and its pin state. This
   amends decision 0020 ruling 1's "reads only what the journal derives".

   The order of the two is deliberate. The journal remains the source of
   run facts; the crossing report is a single map-derived fact set, and
   Muninn consumes the report the loader built rather than resolving,
   hashing or comparing a pin a second time. The seat is still handed no
   repository tree, no host location behind a declared path, no secret
   and no working directory but its own empty scratch directory; the file
   reads and digest comparisons happen in `brokkr-runtime` when the
   command opens the world, exactly as they do for `brokkr doctor` and
   `brokkr realms`.

2. **A crossing fact is a citable fact.** A proposal's durable citations
   are, beside a run's `(run_id, seq)` pair, a crossing's `(realm,
   crossing)` pair, where the realm is the CONSUMER whose run would
   refuse and the crossing is the name the dossier states as a moved
   finding. This amends decision 0020 ruling 3's "run ids and sequence
   numbers".

   A crossing citation is valid only when the dossier states that
   crossing as a finding charged to that realm. An invented crossing, or
   one charged to the wrong realm, is refused and nothing is recorded,
   the same way an invented run citation is. A crossing has no run id and
   no sequence number and none is borrowed, because a reader who followed
   a borrowed run citation would reach a ruling that never mentioned a
   contract. The crossing evidence the proposal stood on — declared
   paths, publishing realms and pin states — is snapshotted into
   Muninn's append-only record, so a later reader can still check the
   proposal against what it saw after the map changes.

3. **A world with no readable journal may still yield a dossier.** Ruling
   1's worldview assumed a journal to derive from. Where the map draws a
   crossing, the dossier can be written with no journal at all: each
   absent mapped journal is said out loud, and only a world with neither
   a readable journal nor a crossing is refused. The invocation still
   rules nothing and executes nothing; it is not tenure and it is not a
   new action.

4. **Nothing else in 0020 moves.** Ruling 2 (Muninn proposes, never rules
   and never works), ruling 4 (a bounded seat, not a daemon with tenure),
   ruling 5 (the machine's mouth stays plain) and ruling 6 (delegation
   only by a future, explicit, recorded grant) stand unchanged. This
   amendment grants no operator command, no run, no file edit, no secret
   and no repository tree. It is cited where the crossing half of the
   dossier and its record are enforced.

   **Enforcement binding:** the crossing half of `Dossier`, its closed
   crossing-citation set, and the `crossing_citations` and snapshotted
   `crossings` of a record line in `crates/brokkr-cli/src/muninn.rs`; the
   append-only file in `crates/brokkr-cli/src/muninn/record.rs` that
   keeps those records durable; and the tests
   `a_moved_crossing_is_a_finding_under_the_consuming_realm`,
   `a_crossing_citation_is_checked_against_the_consuming_realm`,
   `a_crossing_citation_is_recorded_and_rendered`,
   `the_record_keeps_the_crossing_evidence_after_the_source_moves` and
   `a_world_with_crossings_reports_them_before_any_journal_exists`. The
   read-only and no-journal-write guarantees stay held by
   `no_run_journal_gains_an_event_and_the_store_is_opened_read_only`.

## Consequences

- The overseer's evidence now includes what the world says about the
  contracts between its realms, and the seat still never sees the
  filesystem itself. `brokkr realms`, `brokkr doctor` and
  `brokkr muninn run` read one already-computed report, so no surface can
  word a crossing a second time.
- A world mapped before its first run is no longer nothing to report on
  when it draws a crossing; that world gets a dossier, and its absent
  journals are named rather than swallowed.
- 0020's own text is not edited into a different meaning. This decision
  takes its own number and names the rulings it amends, as 0042 ruling 1
  requires and this directory's "How a decision is made" repeats. Only
  the operator accepts it; until then the crossing half of Muninn's
  dossier rests on a proposed amendment, which is stated here so the
  choice is visible rather than assumed.
