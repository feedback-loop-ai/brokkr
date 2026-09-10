# Design: The crossing, enacted — decision 0057, Phase 2

This change folds a mechanism already built and proved into the living
specs; it rules no new behavior. The design records the three readings
that shaped the fold, each already encoded in the code and in decision
0057's own consequences, so the next reader meets the reason beside the
requirement.

## DD1 — One reading of the disk, two questions answered

`World::load` and `World::read` share one `resolve_crossings`. It
resolves the whole world's publications first and compares the pins
second, so a published file that is gone is the PUBLISHER's failure and
is never reported as a consumer having pinned the wrong digest of a file
that is not there at all. The same reading is carried as
`CrossingReport` per realm: `World::load` refuses on the first failure
([`first_crossing_failure`]), while doctor, `realms` and `muninn` read
the reports and refuse nothing. One wording, read out of
`CrossingFailure::error`, serves every surface.

Consequence for the fold: the requirements state the resolve-then-compare
order as a correctness property, not an implementation detail.

## DD2 — The fault direction follows the run-time consequence

A **moved** pin is charged to the CONSUMER: its declaration and the
publisher's bytes disagree, and it is the consumer's run that
`World::load` stops. An **unchecked** pin — a publication that could not
be read — is the PUBLISHER's failure, carried on the publisher's line,
and the consumer is warned without being charged; the pin is counted so
that `consumed` is never read as "this many matched". `brokkr doctor`
prints both directions distinctly, and `brokkr muninn run` raises only
the moved pin as a finding, under the consuming realm.

Consequence for the fold: the manifest requirement keeps observation
(`crossings`) apart from declaration (the map), and the readout
requirements keep the two fault directions apart rather than collapsing
them into one "unverified" state.

## DD3 — A crossing is not a run

A moved pin is a fact about the MAP, not about any run: it has no run id
and no sequence number. The dossier therefore cites it as
`(consuming realm, crossing)` — its own citation shape, checked against
the dossier's own closed set — rather than borrowing a run citation a
reader would follow to a ruling that never mentioned a contract. An
invented crossing, or one charged to the wrong realm, is refused and
recorded nowhere (decision 0001; decision 0007's provenance discipline).

Consequence for the fold: the `muninn` requirement names the crossing
citation explicitly so a later change cannot quietly re-point it at a
run.

## DD4 — Declared paths in the refusal and the evidence, resolved paths in the manifest

A crossing is named repository-relative on the map, and the loader also
resolves it against the publishing realm's worktree to read the bytes.
The two paths answer different questions, and the surfaces keep them
apart: a moved refusal names the publisher's own DECLARED path, so the
refusal text and the crossing evidence Muninn hands a seat carry no host
location, while the run manifest records the RESOLVED `source` the loader
actually read beside the digest — a run testifies to what it stood on,
and that testimony is workspace data, never bundle data.

The dossier states entry for entry what each realm publishes and consumes
rather than bare counts, and Muninn snapshots those entries into its own
append-only record, so a later reader can still check a proposal against
what it saw after the map moves. Because the report comes off the map, a
mapped world whose realms have not run yet still yields a crossing
dossier; only a world with neither a readable journal nor a crossing is
refused.

Consequence for the fold: the `muninn` requirement states the
entry-level evidence, the snapshot and the no-journal case, and the
loader requirement names the declared path in a refusal.

The dossier's move from journal-derived facts alone to the loader's
map-derived crossing report is a widening of accepted decision 0020's
evidence-source and citation rules. It is not this change's to rule, and
it is not left resting on the fold: the proposed decision 0059 takes its
own number, names the two 0020 rulings it amends, and is cited where the
`muninn` requirement and the crossing half of the record are enforced.
