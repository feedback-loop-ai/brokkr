# 0058 — A forced crew may be scoped offices: the library pins one model per office and hires no fallback

Status: proposed
Date: 2026-09-10

## Context

Decision 0041 ruling 2 made the library the roster: "One hire per office,
and the library is the roster." A shipped recipe seats an office from
`agents/`, and the office owns the chain. Ruling 7 leaves inline model
sites only where a recipe "must force the crew": the wager harnesses are
the original exception because their parity law needs inline arms, and
the addendum of 2026-09-06 added `recipes/standby`, whose whole purpose is
the vendor it does *not* use — a library chain would silently undo that
at its first fallback, so the recipe pins its two model seats inline.

The operator now requires a second forced crew, `recipes/gpt-flash` (PR
#257), fixed in every triage class at once: GPT Sol rules triage,
specification, clarification, design synthesis, task planning and
analysis; DeepSeek Flash 4.1 — the `flash-experiment` alias,
`deepseek-v4.1-flash-expires-on-0910` — implements; a GPT/Flash panel
states positions; and one GPT Astra chief alone rules the protected
review phase. No Claude model is hired, and no fallback may silently
reach an older Flash. The recipe's own README states the crew and the
expiring endpoint; the framing makes both mandatory.

Neither existing mechanism expresses that. Seating the standard offices
would keep their cross-vendor chains (ruling 2's table), so a first-link
failure could substitute a model the requirement forbids: the fallback is
the feature everywhere else and the defect here, exactly as it is for the
hedge. An inline crew of the kind ruling 7 admits would force the models,
but `recipes/gpt-flash` extends `recipes/triage` and selects a seat body
per triage class; every seat would restate a full driver command inline,
duplicating the inherited table and moving the model assignment out of
`agents/`, where `resolve_agent`, the roster walk and the library tests
can see it. The same forced pair would also be unusable by any later
recipe that needs it.

The tree lands fifteen scoped offices instead: `agents/gpt-flash-*.json`.
Each is an ordinary library agent — it reuses a library charter unchanged,
resolves through the same path, and is held to every existing check
(0031's pin, 0035's effort, 0041 ruling 3's judge vocabulary, 0045's
vendor line, 0046's boundary). What differs is one property: it names
exactly one model in `models` and one effort in `efforts`, so it has no
chain and nothing to fall back to. `recipes/gpt-flash` overrides each
inherited seat to its scoped office. The standard roster, its assignments
and its chains are untouched.

Alternatives weighed:

- **Seat the standard office and let it fall back.** Rejected: the first
  fallback is precisely the model the requirement excludes. A chain here
  reintroduces the silent substitution the recipe exists to refuse.
- **Pin the crew inline, as `standby` does.** Rejected: it forces the
  models but not the structure. This recipe's seats are selected per
  strategy, so the inline form multiplies the table and puts the model
  assignment beyond the resolver's checks and the library's tests;
  `standby`'s inline arms are `fast`'s two seats, while this recipe's
  reach is a whole strategy table.
- **Give the recipe a private agent directory outside `agents/`.**
  Rejected: the loader walks one roster, and a second would duplicate the
  resolver, the charter accounting and every compile-time check.
- **Relax the fallback law globally.** Rejected: the standard roster's
  chains cross vendors for a reason (0041 ruling 2), and a silent fallback
  is the norm a hedge or a forced crew is the exception to. The exception
  is named, never generalized.
- **Override the model on a seated standard office.** Rejected: a seat
  names an office and the office owns the chain. Overriding the office's
  model in the recipe would make the recipe's text disagree with the
  resolved agent, which is what the manifest and witness exist to prevent.

## Rulings

1. **A recipe that must force its crew may add scoped offices to the
   library.** The offices are ordinary `agents/` files named for the
   recipe (`gpt-flash-*`), each reusing a library charter unchanged. They
   are additions to the roster, never replacements: the standard offices,
   their models, efforts and chains are unchanged, and every check the
   resolver performs on an office applies to a scoped office.
   `recipes/gpt-flash` seats them by overriding its inherited seats; no
   recipe-local roster is created.

2. **A scoped office pins one model, one effort, and no fallback.** Its
   `models` array holds exactly one name and its `efforts` object exactly
   that name. No chain is written and the resolver's fallback path is
   never reached. The absence is the ruling: the crew is forced because a
   fallback would silently hire the wrong vendor or an older generation,
   which is the one thing the recipe exists to refuse.

3. **The exemption is keyed by the scoped name and asserted, never
   inferred.** The library test names the scoped offices, exempts exactly
   those from ruling 2's "real fallback chain" assertion, and requires
   every other model-backed office to keep a chain of at least two
   candidates. `muninn`'s single-model office is unchanged. A future
   single-model office that is not part of a forced crew is not admitted
   by a wildcard; it must be named, and naming it is the decision.

4. **The forced crew is also a shape check, not only a library check.**
   Compiling `recipes/gpt-flash` pins implementation on Flash 4.1 through
   the `flash-experiment` alias and never an older Flash lane; the Sol
   offices on codex; a GPT/Flash review panel that states positions
   before one Astra chief gate; every strategy answered; no Claude
   provider; one candidate per site; and the inherited policy table and
   deterministic verify/ship/validate gates byte-for-byte from
   `recipes/triage`.

5. **The standard roster is preserved.** Rulings 1–3 of decision 0041
   continue to hold for every standard office: one hire per office, a
   chain whose first link is hired at no less effort than any later link,
   and panels that cross a vendor line at their first hires.

**Enforcement binding:** `crates/brokkr-runtime/tests/library_data.rs`
names the scoped offices in `SCOPED_OFFICES`, requires exactly one
candidate for each, and keeps the two-candidate chain assertion for every
standard office; `crates/brokkr-runtime/tests/gpt_flash_shape.rs`
compiles `recipes/gpt-flash` and pins the assignments, the panel
composition, the sole Astra chief, the scoped roster and the inherited
gates; `crates/brokkr-runtime/tests/roster.rs` continues to hold every
shipped panel to two vendors and every shipped model site to the library
outside the ruled inline exceptions; `recipes/gpt-flash/README.md` states
the crew.

## Consequences

- A forced crew becomes composable: a later recipe that needs the same
  GPT/Flash pair seats the same offices, and the assignment is checkable
  by the resolver before a run starts.
- The library grows fifteen offices whose charters already exist, so
  charter accounting is unchanged; the new offices share their charters
  with the standard roster.
- The recipe stays a `triage` extension: the inherited policy table, the
  deterministic verify/ship/validate gates and the retry and escalation
  bounds are not restated, so a change to `recipes/triage` reaches it
  unchanged.
- **Deliberately unruled.** Whether a scoped office may ever be seated by
  a standard recipe, and whether the `gpt-flash-*` prefix remains the only
  scoped family. The operator may extend or retire the pattern later; this
  decision writes no general naming law.
