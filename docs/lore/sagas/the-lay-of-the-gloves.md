# The Lay of the Gloves

*A saga of this forge, for release v0.13.0 "Járngreipr". Law three of
decision 0019: where a saga retells a real event, it cites its evidence.
This one does.*

---

The Edda gives the thunderer's gloves one line. Of his three treasures,
the third "is his iron gloves; these he cannot do without when he lays
hold of the hammer's haft." It does not say why. This shop has its own
reading, and it is not a flattering one. The hammer came out of Brokkr's
forge with a handle an inch too short, because a fly bit the
bellows-hand once. A hammer you cannot hold by its haft must be held
near its head, and the head is hot. Hence the gloves.

The forge's last release was named for a binding: Gleipnir, thin as a
ribbon, which nothing breaks. In it the realm learned to grant tools.
Every power was off until the map named it, and every grant was written
into the run's own record. It was a good binding. But a granted tool
that no hand can safely hold is a hammer left on the anvil, admired.

So the forge set to making gloves. Ninety-five pieces went into them,
unit by unit:
- a broker of Brokkr's own, standing between every seat and every tool
  server;
- a box around each server;
- a ledger that writes down what was asked and what came back;
- a refusal for anything it could not read.

Every piece passed its trials, and the fixtures said the gloves fit.

The operator ruled that the fixtures would not have the last word.
Before the gloves were called finished, a real hand would wear them.

**The first time the hand reached,** the gloves would not let it near.
On an ordinary host, with its system files linked many ways over, the
box's identity check turned the seat away at the door. The forge taught
the driver to lay down its privileges for good, setting no_new_privs,
before it handed a brokered harness over. Then the hand came closer.

**The second time,** the hand greeted the hammer in a newer tongue than
the gloves knew. It asked for protocol version 2025-11-25, and the
gloves refused the greeting. So the seat never saw the tool it had been
granted. Every fixture had spoken the older tongue, and none had
noticed. The forge taught the broker to answer in its own version, as
the protocol's negotiation says, and now the hand could see the hammer.

**The third time,** the hand saw the hammer and was forbidden to close
on it. The harness's own permission list named only the workspace tool,
so the brokered one was denied, and a headless seat has no one to ask.
The forge taught the launch to name each broker it had sealed, and
nothing more. The hand closed, the call went through, and the answer
was kept.

**The fourth time,** the hand let go, and the gloves fell off before
they could write down the letting-go. The harness ends its tool servers
with an interrupt, and the broker had no ear for one. It died before it
wrote the session closed, and every run parked on a ledger it could not
read. The forge taught the broker to take the interrupt as the
session's end, the way a closed hand ends a grip.

Then the real hand reached, was greeted, closed, called, and let go.
The run completed.

---

**What the lay teaches.** A real hand found every burn in this lay, and
every one of them had passed its fixtures. A fixture is a hand the smith
carved for the glove: it fits because it was made to fit. Gloves are
not done when they fit the carving. They are done when the hand that
will use them can approach, greet, grip and release. Those are four
separate motions, and each one burned once.

The release is named for the gloves because that is what this forge
made this month. It is not a new power. It is a way to hold the powers
the realm grants without being burned by them.

**Evidence.**
- **The verse:** Snorri Sturluson, *Gylfaginning* 21, in Rasmus B.
  Anderson's translation (1879; Project Gutenberg #18947): "His third
  treasure that is of so great value is his iron gloves; these he cannot
  do without when he lays hold of the hammer's haft."
- **The short handle:** decision 0019 and docs/lore/README.md. Reading
  the gloves as its consequence is this shop's own commentary. The Edda
  gives no reason.
- **Gleipnir:** the v0.12.0 release notes, and decision 0065 slice one
  (#319).
- **The ninety-five pieces:** decision 0065 slice two, whose openspec
  change is `decision-0065-capabilities-slice-two`. That is the count of
  commits on main between v0.12.0 and this release whose subjects name
  it.
- **The real hand:** ruling F1 of unit U9b1b asked for a real Claude and
  a real Codex harness through a brokered launch before the unit landed.
  #940's ruling names "the real-harness check that ruling F1 of U9b1b
  asked for".
- **The four motions:**
  - approach: #933, where `brokkr driver` sets no_new_privs (F1: an MCP
    seat refused by MB3's identity cause on a host with multiply-linked
    system files);
  - greet: #940, the 2025-06-18 answer to a 2025-11-25 `initialize`
    (R1);
  - grip: #952, each sealed broker named in the launch's allowance (R3);
  - release: #955 (merged 2026-11-01 23:35Z as 3bb0e566c), SIGINT or
    SIGTERM taken as the session's end, which closes the ledger (R5).

  Each ruling quotes the real-harness finding it answers.
- **The completed run:** the real-harness check on #955's tree, on the
  code that merged as 3bb0e566c.
