# wager-harness — the elevation instrument

Decision 0021 ruling 3 says a driver's trust tier is **earned by
evidence**, and ruling 7 says a newcomer's first outings are *wagers*:
the same feature run under rival crews, compared by artifacts. This
recipe is that procedure, written down and made compilable.

It is two things at once, and both matter:

1. **A worked example.** `bundle.json` here is a real, listable,
   digest-pinned recipe: `fast` with the implement seat's driver swapped
   to `codex`. Compile it, run it, `brokkr compare` it against a `fast`
   run of the same feature. It reproduces exactly what the deleted
   `recipes/fast-codex` was during the wager
   ([the-wager.md](../../docs/essays/the-wager.md),
   [PR #85](https://github.com/feedback-loop-ai/brokkr/pull/85)),
   updated for the current schema.
2. **A pattern to copy.** The parity checklist below is the reusable
   part. Swap `codex` for whichever challenger you are weighing, keep
   everything else, and you have a lawful wager.

## The whole diff

```json
{
  "name": "wager-harness",
  "extends": "fast",
  "override": { "seats": ["implement"] },
  "seats": {
    "implement": {
      "role": "roles/implementer.md",
      "results": ["complete", "broken", "blocked"],
      "class": "work",
      "limits": { "max_attempts": 2, "timeout_seconds": 5400 },
      "driver": {
        "command": ["{brokkr}", "driver", "codex", "--",
                    "--sandbox", "danger-full-access"]
      }
    }
  }
}
```

Thirty-four lines as the file is actually formatted, against `fast`'s
109, and **one** of them is the experiment: the driver name in
`driver.command`. Everything else — the phase table, the reforging
ladder, the protected review gate, the verifier, the reviewer, the
shipper, the limits, the charter — is `fast`'s, inherited. That is the
property this whole repository exists to make possible, and most agent
stacks cannot express it at all: the challenger and the incumbent differ
by a name, under one digest each, both recorded.

Two things changed against the historical `recipes/fast-codex`
(`git show 3718061c5591:recipes/fast-codex/bundle.json`), and both are
schema drift rather than design:

- `"class": "work"` on the implement seat. The historical bundle
  predates decision 0021, which now requires every driver-bearing site
  to declare whether it works or judges.
- `{brokkr}` rather than `{forge}`. The old token still expands and
  warns once on stderr (decision 0019); a new recipe should not ship
  using a deprecated spelling.

## Why the challenger may hold this seat at all

`implement` is `class: "work"`. Decision 0021 ruling 7: a driver with no
journaled track record here holds work seats freely, whatever its
vendor. `adapters/codex.json` declares `trust_tier: "untrusted"` and
`binding_grant: false`, and neither blocks this seat — it judges nothing
and binds no secrets.

What the challenger may **not** hold is a gate. `verify`, `review` and
`ship` are inherited from `fast` unchanged, on the incumbent, and the
compiler refuses at compile time any attempt to move a gate onto an
untrusted driver. **This is a feature of the wager, not a limitation of
it:** the judge is the same in both arms, so a comparison measures the
crews and not the referee.

## The parity checklist

Round one of the original wager was rigged, and the referee said so:
the Codex sessions ran in a tighter sandbox than the Claude ones —
write access denied where the incumbent had it freely. The challenger
blocked mid-implement against walls the incumbent never saw. Not
malice; a default nobody had questioned, which is what bias usually is.
The rematch ran under parity, and only the rematch counted.

That confession is encoded here as procedure. **Run through this list
before you trust a comparison, not after.**

1. **Same repository base.** Both arms start from the same commit. Use
   `brokkr rerun --run <id> --recipe <challenger>` rather than launching
   a fresh run by hand: it carries the base forward for you, and the
   journal records that it did.
2. **Same sandbox.** The challenger's `driver.command` must grant the
   same access the incumbent's does. `fast`'s implement seat runs
   `--permission-mode acceptEdits`; this recipe's runs `--sandbox
   danger-full-access`. Those are two providers' spellings of *the same
   grant*, and matching them is a judgement call you must make
   deliberately and record. **A tighter cage on one arm is the exact
   defect that voided round one.**
3. **Same tools.** Every command one arm may run, the other may run.
   Where a provider's adapter declares `tool_permissions:
   "unsupported"` — as `codex` and `dsh` both do — the arms are not
   symmetric by construction. Say so in the comparison rather than
   pretending the asymmetry away.
4. **Same charter, byte for byte.** `roles/implementer.md` in this
   directory is a byte-identical copy of
   `recipes/fast/roles/implementer.md`. Not "equivalent", not "adapted
   for the challenger" — identical, and
   `crates/brokkr-runtime/tests/wager_parity.rs` fails if the two files
   ever diverge. A charter rewritten "to suit" the challenger is a
   second variable, and two variables measure nothing.
5. **Same limits.** `max_attempts: 2`, `timeout_seconds: 5400` — copied
   from `fast`'s implement seat, not tuned. A challenger given more
   attempts, or less time, is not being compared.
6. **Same gates, same judge.** Inherited, not restated. If you find
   yourself overriding `verify`, `review` or `ship`, you have left the
   wager and started building a different recipe.
7. **One seat overridden.** `override: { "seats": ["implement"] }` names
   exactly one. If the list has two entries, the experiment has two
   variables.
8. **Both digests recorded before either runs.** `brokkr recipes show
   fast` and `brokkr recipes show wager-harness`. Composition resolves
   at compile time and the chain rides in each run manifest, so a
   comparison can state what it compared. A recipe edited between the
   two arms is a third variable that leaves no trace unless you pinned
   the digests.
9. **Rigging leaves a record.** When you discover you got one of the
   above wrong — and you will — the flawed run stays in the journal,
   labelled for what it was, and the rematch is what counts. A fair test
   is not one where the referee is unbiased; no such referee exists,
   human or model. A fair test is one where the rigging leaves a record.

## Running the wager

```
brokkr run    --recipe fast --repo . --feature "<the commission>"   # incumbent
brokkr rerun  --run <id> --recipe wager-harness                     # challenger
brokkr compare <run-a> <run-b>                                      # trails, divergence, per-seat costs
```

Then judge the **artifacts** — the code, the tests, the residual
lists — with the vendor names mattering not at all. In the original
wager neither crew's artifact was shippable alone; each was wrong
somewhere the other was right, and what landed was a co-credited
synthesis. Expect that outcome to be normal rather than surprising.

A promotion or demotion that follows is an **operator ruling recorded
where rulings live** (decision 0021 ruling 3), citing this comparison as
its evidence. This recipe produces the evidence; it never produces the
ruling.

## Cost expectations

**Structural, not measured.** Running a wager costs, by construction,
*two full deliveries of the same feature* — the incumbent's and the
challenger's — plus whatever the synthesis takes afterwards. The saving
is not in the run; it is in not choosing a model by benchmark
screenshots.

The one wager this repository has actually run is documented in
[the-wager.md](../../docs/essays/the-wager.md) with its journal
citations; **this recipe itself has not been run.** It compiles under
the shipped adapters, its charter parity against `recipes/fast` is
tested, and its manifest digest is pinned in
`crates/brokkr-runtime/tests/witness_digests.rs`. That is the claim, and
the only one.

## Where the example lives

At `recipes/wager-harness/bundle.json` — this directory's own bundle,
rather than a sub-path the README points at. The choice was open; this
one was taken so the example is a real library member: `brokkr recipes
list` shows it, `brokkr rerun --recipe wager-harness` runs it, the
tree-wide compile test covers it, and its digest is pinned like any
other recipe's. An example nobody can run is a snippet, and snippets
rot.
