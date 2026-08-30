# 0019 — Brokkr: the name, the verb, and the lore layer

Status: accepted — operator ruled 2026-08-31
Date: 2026-08-30

## Context

The product has been `the-forge`. Two facts make that name untenable as a
public identity. First, "forge" is the most collided word in software —
SourceForge, Laravel Forge, Autodesk Forge, MinecraftForge, ForgeRock, and
every code host generically called a forge — so the name was never findable.
Second, a widely followed author released **SwarmForge**, a multi-agent
coordination tool, into the same niche. In a naming collision the bigger
megaphone defines what the shared word means; staying "the other forge" means
being evaluated inside that product's frame (packs, cockpits, prompts) where
this product's distinguishing properties — the journal, bounded seats,
attested releases, the self-forge loop — do not appear.

The renaming question opened a second one: this product's components map onto
the Norse forging myths with structural, not decorative, precision — the
dwarves who made the artifacts, the wager judged by a council, the hammer
shipped with a documented flaw. A disciplined lore layer is a positioning
asset. An undisciplined one is a theme park.

## Decision

1. **The product is named Brokkr.** In the myth, Brokkr's whole task was to
   pump the bellows and not stop — Loki, as a biting fly, made him flinch
   once, and Mjölnir's handle came out short. Steadiness under distraction,
   and the cost of one lapse, is this product's core loop (the self-forge
   loop) told as a story a thousand years old. The name is short, spellable
   without diacritics, and unclaimed in this space.

2. **"Forge" survives as the verb.** Slices are forged; runs are forged;
   Brokkr forges. The proper noun is retired from the marquee, not from the
   vocabulary.

3. **Mechanisms keep plain names.** Journal, trail, rail, seats, panel,
   chief, park, ship, intake — untouched. A new operator must be able to
   guess what a command does with no glossary. Myth names attach only to
   *personas* and *releases*, where a name with a story outperforms a common
   noun.

4. **Persona names are allocated, not yet all built.**
   - **Muninn** is reserved for the standing overseer agent (Odin's raven,
     Memory: flies the whole fleet, remembers across runs, reports to the
     operator, decides nothing). Its authority model is its own future
     decision; this decision only holds the name.
   - **Loki** is the documented flavor of the verify role — the adversary
     kept on purpose. Flavor only; the role keeps its plain name in every
     mechanism and command.
   - **The chief keeps its plain name.** Mímir — the head consulted fresh
     for one ruling, then set down — may appear in prose about the chief,
     but never on a marquee: an adjacent company holds that name's mindshare
     and a mark in a neighboring category.
   - **The operator has no myth name, deliberately.** The smith is the
     human. Costuming the operator in a god-name would misstate the
     authority model, which is the product.

5. **Releases carry artifact names.** Version numbers stay the identity;
   each release may take a forged-artifact name (Gungnir, Skíðblaðnir,
   Draupnir…) as its epithet. Pure delight, zero load-bearing weight.

6. **The lore layer exists under `docs/lore/`** — an Edda (the canonical
   mapping of what was built to what it means) and Sagas (original short
   myths retelling real journal events). It is governed by five laws:

   1. *Myth is commentary, never specification.* No rule may live only in a
      myth. If the lore burned, the constitution must be whole.
   2. *Every myth is deletable-testable.* Delete the reference; if nothing
      is lost, it never belonged. No quota — a feature with no myth gets no
      myth.
   3. *Never fake antiquity.* Real references cite the Eddas honestly;
      original sagas are labeled as this forge's own. A product about
      provenance does not launder its lore. Where a saga retells a real
      event, it cites its evidence: a run id, a ruling id, a PR.
   4. *The lore stays out of the machine's mouth.* Errors, park reasons,
      CLI output: plain, always. Sanctioned exception: release epithets.
   5. *Visual restraint.* Names and stories, yes; runic scripts and
      appropriated symbols, never. The `∙ ∙ ⏺` mark is the register.

7. **SwarmForge is acknowledged, and nothing of it is copied.** The
   standing-overseer concept reached this product by way of SwarmForge's
   lieutenant, and the README will say so plainly. SwarmForge carries no
   license, which means all rights reserved: no code, scripts, prompts, or
   prose from it may enter this tree. Ideas are not expression; Muninn is an
   independent design with an inverted authority model (reads the journal,
   remembers across runs, proposes to the operator, rules nothing).

8. **Positioning names the category difference, not the competitor's
   frame.** Coordination tools help agents work together; Brokkr proves what
   they did. Evidence first, lore behind it — the same order the product
   enforces.

9. **Migration sequence** (each step its own slice, after this decision is
   accepted): repository rename (the host redirects old URLs), binary rename
   with a `forge` shim for one release, the TUI brand mark, README
   repositioning with the SwarmForge acknowledgment, crate renames lagging
   by a release.

## Consequences

The product gains a findable, ownable name whose story encodes its thesis,
and a docs identity no adjacent product can copy — because the sagas cite a
journal, and only this product has one. The cost is a one-time rename (taken
at the cheapest possible moment, pre-adoption) and a standing editorial duty:
the five laws are load-bearing, and the first violation — a rule that lives
only in a myth, a saga passed off as ancient — converts the asset into a
liability.
