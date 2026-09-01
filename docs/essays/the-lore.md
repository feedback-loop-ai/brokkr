# The Lore

We named the bellows Brokkr. The journal is carved by Norns. A raven
named Muninn flies over the fleet each day and speaks into the
operator's ear, and when a review seat catches a flaw before it ships,
the saga that retells it is called *The Lay of the Fly*.

I want to defend this — not as branding, but as an engineering
decision with a design, a failure mode, and a set of laws that keep it
from rotting. Because the objection writes itself: whimsy is how
codebases accumulate glossary tax. Every team has met the service
named after an in-joke nobody remembers, the cute noun that new hires
must reverse-engineer, the wiki page of "what we call things" that is
itself out of date. Undisciplined lore is a liability, and most lore
is undisciplined.

So before the first myth entered the tree, we ruled five laws over it
([decision 0019](https://github.com/feedback-loop-ai/brokkr/blob/main/docs/decisions/0019-brokkr.md)):

1. **Myth is commentary, never specification.** No rule may live only
   in a myth. If the lore burned, the constitution must be whole.
2. **Every myth is deletable-testable.** Delete the reference; if
   nothing is lost, it never belonged. No quota — a feature with no
   myth gets no myth.
3. **Never fake antiquity.** Real references cite the Eddas honestly;
   original sagas are labeled as this forge's own, and where a saga
   retells a real event, it cites its evidence: a run id, a ruling, a
   PR.
4. **The lore stays out of the machine's mouth.** Errors, park
   reasons, CLI output: plain, always. One sanctioned exception:
   release epithets.
5. **Visual restraint.** Names and stories, yes; runic scripts and
   appropriated symbols, never.

Notice what these are: the same discipline the product applies to
everything else, applied to its own storytelling. Commentary never
specification is our first determinism law wearing a different coat.
Cite-your-evidence is the journal's rule. The lore is governed the way
the machine is governed, which is the only reason it works.

## Why myth, though

Here is the honest mechanism: **a name that carries its lesson is
documentation with a recall rate.**

When Brokkr and Sindri forged the gifts of the gods, Brokkr's entire
task was the bellows — pump steadily, no matter what. Loki, as a fly,
bit his hand, his neck, his eyelid; Brokkr flinched *once*, and
Mjölnir came out with a handle too short. And the gods judged it the
finest thing ever made anyway.

That is not decoration. That is a compressed spec of the self-forge
loop: steadiness is the whole job; one lapse ships a flaw; and a flaw
documented at judgment is not a scandal — it is a handle. I could
write those three constraints in a design doc, and I did. But the
design doc has a recall rate near zero and the story has one near
perfect, because the story is a thousand years old and survived on
memorability alone. Myths are pre-tested mnemonics for failure modes.
The Eddas are what oral tradition kept after everything forgettable
fell away — which makes them, oddly, the most battle-hardened
documentation format we have.

So each entry in [our Edda](https://github.com/feedback-loop-ai/brokkr/blob/main/docs/lore/edda.md)
earns its place only by teaching a true constraint:

- **Muninn** — Odin fears more for Memory than for Thought. The
  standing overseer's whole value is memory across runs, and its whole
  authority is to *report*. It decides nothing.
- **Loki** — who bit the bellows-hand to win a wager, and was kept
  around anyway. Keep the adversary in the shop, on purpose, bounded.
  Our verify seats are prompted to refute, and their findings stop
  ships. The saga citing the stored XSS a review seat caught carries
  the run id, because law 3 says a saga without evidence is a fake.
- **Mímir's head** — counsel consulted fresh, then set down. The chief
  rules on written positions in a fresh session, blind to authorship.
  Judgment protected by *removing* context — the exact opposite trade
  from Muninn, whose value is accumulating it. The pairing teaches
  both prohibitions at once.
- **The Norns** — what is carved is carved. The journal is
  append-only; fate does not get updated in place.
- **The operator — no name.** The one entry that is an absence, and
  the most load-bearing: the human's authority is not a character in
  the story. The machine has lore; the law above it does not.

## Summon by myth, read in plain

The rule that keeps the whole thing from becoming glossary tax came
from an operator ruling, and it is one line: **you may summon by myth;
you must read in plain.**

Names live in tiers. The product and its releases may be mythic —
that is marketing, and the shelf is clean. Mechanisms — commands,
phases, seat vocabulary — are plain and guessable with no glossary:
`run`, `verify`, `park`, `conclude`. Pipeline agents whose names
surface on evidence are plain on every evidence surface, and may carry
a mythic byname only where reading is leisurely. But a standing agent
you invoke and address by name — Muninn — is mythic *on purpose*,
because invocation is where a name with a story earns its keep.

The asymmetry is the design. When you are debugging at 2am, every
surface the machine shows you is plain words. When you are deciding
whether to trust a standing agent with your fleet's memory, the name
tells you exactly what it is and — more important — what it is not.
Nobody who knows the raven asks why Muninn cannot retry a run.

## Lore under errata

The strongest evidence that the lore is governed rather than
decorative: it has been *wrong*, and the corrections are on the
record.

Early lore implied the operator was the smith. An operator ruling
corrected it — the implement seat wields the hammer; the operator is
not a character in the forge at all — and the erratum is dated and
attributed in the tree. A later ruling corrected the naming
reasoning itself (public-domain myth carries no trademark caution
except at the marquee tier), and the correction is recorded *inside
decision 0019* as an addendum. Storytelling with an amendment record
is not something marketing departments produce. It is what you get
when the same constitution governs the code and the tales about it.

And the releases carry the sanctioned exception: Gungnir, the spear
that flies true; Skíðblaðnir, the ship that folds; Bilskirnir, the
hall of five hundred and forty rooms — the many-hearths release, named
the day per-realm journals landed. An epithet is one word of myth on
an otherwise attested artifact: signed tag, checksums, provenance.
Flavor riding on evidence, never instead of it.

## The moat nobody can copy

Here is the positioning fact, stated plainly because our own rules
demand it: any product can adopt Norse names tomorrow. The names are
free — that is rather the point of a thousand-year-old commons.

What cannot be copied is the citation. Our sagas retell journal
events, and end with run ids you can export and verify — the chain
recomputes on your machine or the saga is false. A competitor can
name their scheduler Odin; they cannot cite the hash-chained record
of the day their review agent stopped a ship, because they do not
have one. Lore with provenance is just the product's thesis again:
the story is only as good as the evidence line under it.

## The honest limits

Taste does not compile. The five laws bound the lore, but they cannot
make it good — that stays editorial judgment, which is why law 2
exists as a standing prune: delete the reference; if nothing is lost,
it never belonged. There is no quota, and the pressure runs toward
fewer entries, not more. A feature with no myth gets no myth, and most
features have none.

And the tax is real, just bounded. A newcomer meets perhaps six mythic
names, each with a two-paragraph story that *is* the explanation, and
every operational surface in plain words. We pay a small, fixed
reading cost for a large, compounding recall benefit. Undisciplined
lore inverts that trade — which is why the laws came first and the
raven second.

Engineers love fun. That is not a weakness to manage; it is a
retention mechanism for the constraints that matter — if, and only
if, the fun is under law. Ours is. Delete any entry; if nothing is
lost, it should not have been here.

---

*The Edda is public:
[docs/lore/edda.md](https://github.com/feedback-loop-ai/brokkr/blob/main/docs/lore/edda.md)
— every entry under the five laws of
[decision 0019](https://github.com/feedback-loop-ai/brokkr/blob/main/docs/decisions/0019-brokkr.md),
every saga citing the
[journal](https://github.com/feedback-loop-ai/brokkr/tree/main/docs/evidence)
it retells.*
