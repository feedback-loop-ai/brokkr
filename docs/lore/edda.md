# The Edda of Brokkr

What was built, what it is named for, and the lesson each name carries.
Governed by the five laws of decision 0019: myth here is commentary, never
specification — every rule in this file lives, in plain words, in a decision
doc or a contract, and each entry earns its place by teaching a true
constraint. Delete any entry; if nothing is lost, it should not have been
here.

Real myths below are from the Prose and Poetic Eddas, told straight. The
product's own stories live in [sagas/](sagas/), labeled as ours.

---

## Brokkr — the self-forge loop

When Brokkr and Sindri forged the gifts of the gods, Brokkr's entire task
was the bellows: pump steadily, no matter what. Loki became a fly and bit
his hand, his neck, and at the last his eyelid; Brokkr flinched once, and
Mjölnir came out with a handle too short. The gods judged it the finest
thing ever made anyway.

**The lesson:** steadiness is the whole job, one lapse ships a flaw, and a
flaw documented at judgment is not a scandal — it is a handle. The
self-forge loop is Brokkr at the bellows: the product delivers itself, run
after run, under the same gates it applies to everything else.

*Plain form: the self-forge loop (operational since 2026-08-23); two-step
ship, decision 0005.*

## Muninn — the standing overseer (reserved)

Odin keeps two ravens, Huginn and Muninn — Thought and Memory. Each dawn
they fly over the whole world; each evening they return and speak into his
ear. Odin says he fears for Huginn, but more for Muninn: losing memory is
the greater loss.

**The lesson:** the overseer's value is memory across runs — the one thing
no fresh seat can hold — and its whole authority is to *report*. Muninn
flies, remembers, and speaks to the operator. It decides nothing.

*Plain form: name reserved by decision 0019; the agent's authority model is
a future decision.*

## Loki — the verify seats

Loki wagered his head that the dwarves could not out-forge the sons of
Ivaldi, then bit the bellows-hand to win the bet. The gods took the
artifacts anyway — and kept Loki around, because nothing finds a flaw like
the one who profits from finding it.

**The lesson:** keep the adversary in the shop, on purpose, with a bounded
wager. The verify seats are prompted to refute, and their findings stop
ships.

*Evidence:* run `verify-two-delivered-slices-slic-ffc01c67`, ruling
`REVIEW-RESIDUAL-SECURITY`, review → stop — a stored XSS found in
agent-authored UI code before it shipped. See
[sagas/the-lay-of-the-fly.md](sagas/the-lay-of-the-fly.md).

## The chief — Mímir's head

Mímir's well holds wisdom; Odin gave an eye to drink from it. After Mímir
was slain, Odin kept the head, preserved it, and consulted it at need —
counsel taken fresh, then set down.

**The lesson:** the chief rules on the council's written positions and
clashes in a fresh session, blind to who authored what. Judgment is
protected by *removing* context — the exact opposite trade from Muninn,
whose value is accumulating it. Both prohibitions are deliberate.

*Plain form: the design phase's positions-then-chief sequence,
`recipes/sdd`; heritage protocol in `reference/handoff-protocol.md`.*

## The Norns — the journal

At the well of Urðr sit the Norns — Was, Becoming, Shall-be — carving what
happens into the tree. What is carved is carved; fate is append-only.

**The lesson:** the journal is the product. Events are appended, never
edited; state is a fold over what became; deleting a cache cannot discard a
security hold, because the record, not the cache, is the authority.

*Plain form: the append-only, hash-chained store, `forge-store`; decision
0001.*

## Gleipnir — the chain of evidence

Two fetters of iron failed to hold the wolf. The third, Gleipnir, was thin
as a silk ribbon, forged from six impossible things — the sound of a cat's
footfall, the roots of a mountain, the breath of a fish — and it held,
precisely because it could not be broken the way chains break.

**The lesson:** tamper-evidence over bulk. The journal's hash chain and its
anchors are ribbon-thin and hold because a quiet rewrite is what they make
impossible; an attacker must break them *loudly*.

**Also the lesson, honestly:** the gods only bound the wolf by giving Týr's
hand as surety. Evidence costs something up front. Pay it.

*Plain form: hash-chained events, head anchoring; heritage protocol in
`reference/handoff-protocol.md`.*

## Skíðblaðnir — the pinned bundle

The best of ships: room for all the gods and their gear, fair wind wherever
it sails — and it folds up to fit in a pouch.

**The lesson:** a whole delivery, folded into a digest. A bundle's pinned
manifest is the ship in the pouch: identity preserved through compression,
and the same ship unfolds every time.

*Plain form: bundle compilation and manifest digests; a bundle's identity
covers the engine version.*

## Andvari's ring — sealed secrets

Loki took the dwarf Andvari's gold, and Andvari cursed the last ring,
Andvaranaut: ruin to whoever held it bare. Every hand it passed to
unguarded, it destroyed.

**The lesson:** secrets pass hands only under seal. Seats receive bindings
they declared, from a store the operator controls, and nothing else;
treasure grabbed bare curses the run that grabs it.

*Plain form: sealed secret bindings, decision 0012.*

## Heimdallr — the watch

Heimdallr needs less sleep than a bird, sees a hundred leagues by night,
and hears wool grow on sheep. He holds one horn, Gjallarhorn, and it sounds
for one reason only.

**The lesson:** watch everything; alarm rarely. The read surfaces see every
event; the horn — a park, a stop, a finding — sounds only when an operator
must act.

*Plain form: `watch`, `tui`, `ui` — read-only projections of one
derivation, decision 0013.*

## Draupnir — attested releases

From Draupnir, the ring Brokkr and Sindri forged, eight rings of equal
weight drip every ninth night. Copies, provably true to the original, on a
cadence.

**The lesson:** a release is a reproduction you can prove. The tag admits
only a version that matches the workspace, the suite, and the coverage
gate; the artifacts ship attested, with their checksums beside them.

*Plain form: `.github/workflows/release.yml` — admission, exact release
coverage, attestation, SHA256SUMS.*

## Bifröst — forge-bridge

The burning rainbow bridge between the worlds of gods and men.

**The lesson:** none needed — the crate was named `forge-bridge` before the
Edda existed. Sometimes the myth was already there, waiting.

*Plain form: the `forge-bridge` crate.*

## Sindri and the sons of Ivaldi — the implement seat and the driver fleet

Sindri laid the work in the hearth, told his brother to blow steadily no
matter what, and shaped the gifts with his own hands. But his was not the
only anvil: the sons of Ivaldi had already forged Gungnir and Skíðblaðnir
under the same commission, and the gods judged the crews by their
artifacts alone.

**The lesson:** the smith never judges his own work, and the anvil does
not care which crew holds the hammer. The implement seat shapes; the loop
keeps its heat; judgment happens elsewhere. And a second crew at the same
commission is not redundancy — it is how the gods learned which hammer to
trust.

*Plain form: the implement seat; the driver fleet behind one conformance
suite, decision 0008; `rerun` and `compare`, decision 0010 — the wager's
mechanic, two crews judged side by side by outcomes. Sindri stays prose
only (see the names left in the ground); the seat keeps its plain name
per 0019 ruling 3.*

## The judging of the gifts — two-step ship

Nothing the dwarves made was Mjölnir until the gods had weighed it against
every other gift and ruled. The making and the judging were never the same
act.

**The lesson:** `ready` is not `shipped`. Done has a sole door, and the
ruling that opens it is recorded like everything else.

*Plain form: ship taxonomy, decision 0005 — `shipped` is the sole entry
into `done`.*

## The operator — no name

The operator has no myth name and will not get one — and the operator is
not the smith. The seats hammer; Brokkr keeps the fires; the operator
commissions the work, judges it, and wields what survives judgment, which
in the wager story is the seat the gods hold. Their strike is the ruling,
not the hammer-blow. The authority model is the product, and it is not a
costume. (Corrected by the operator, 2026-08-31.)

*Plain form: the operator-ruling culture; rulings are the operator's,
recorded in the journal and in these decision docs.*

---

## Names left in the ground

Good ore we do not mine, and why — recorded so the question is not
re-litigated at every naming.

- **Sindri**, the brother who did the smithing: a commercial
  zero-knowledge-proof toolchain holds this name, adjacent to our own
  evidence story. Prose only.
- **Mímir**: an observability company owns the developer mindshare and a
  mark in a neighboring category. Prose only (see the chief, above).
- **Odin, Thor**: the gods judge and wield; they are the users of forged
  things, not parts of the forge. Also, a certain film studio.
- **Runic scripts and bound symbols**: some Norse iconography carries
  modern baggage the myths do not deserve. Names and stories, never
  glyphs — law five.
