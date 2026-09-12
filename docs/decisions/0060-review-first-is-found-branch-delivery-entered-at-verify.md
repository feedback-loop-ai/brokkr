# 0060 — Review-first is found-branch delivery entered at verify, with a forced Muse/Astra crew

Status: proposed
Date: 2026-09-12

## Context

The branch already exists. On this firing it is `a3799b6` (the tree
re-pinned to the installed `dsh 0.1.5-rc.1`) plus `281a0c9` (the
promotion-refusal flake hardening): code, written, needing judgment,
proof, and shipment — not a commission to implement from nothing. The
general case is the same shape: a branch written by hand, by a stopped
run, or by a wager arm, where what remains is verification, judgment,
remediation on findings, and ship.

Decision 0051 already gives hand-authored work a road: `recipes/landing`
is `fast` entered at a classify gate, routing prose to review and code
to verify, seating `fast`'s library crew. Two things do not fit that
road on this firing:

- The firing names the crew. The task block seats Astra (codex, xhigh)
  to judge and Muse Spark 1.3-contributor (dsh, xhigh) to remediate on
  return. A library chain would silently undo that at its first
  fallback — the fallback is the feature everywhere else and the defect
  here, exactly ruling 7's reason in decision 0041.
- The branch is code. Classification would always answer `code`, paying
  a gate for no routing, before verifying exactly as a delivery would.

Decision 0041 ruling 2 makes the library the roster and ruling 7 leaves
inline model sites only where a recipe must force its crew: the wager
harnesses (the original exception) and `recipes/standby` (the addendum
of 2026-09-06, whose whole purpose is the vendor it does not use).
Decision 0058 adds the other forced-crew mechanism: scoped `gpt-flash-*`
library offices, each pinning one model with no fallback, for a
fifteen-seat strategy table where an inline restatement would multiply
the table beyond the resolver's checks. `review-first` has two model
seats, like `standby`'s two — not fifteen — so the inline form is
proportionate and keeps the model assignment where the roster walk
already looks for it.

The table is `fast`'s constitution with one word changed: `initial` is
`verify` instead of `implement`. The suite verifies the branch as found,
Astra judges verified code, findings return to Muse through the same
bounded edges `fast` bounds, and a clean verified branch ships through
the same boxed seats. Review stays unavoidable on every path to ship.

Alternatives weighed:

- **Commission `fast` with "change nothing".** Rejected for 0051's
  reason: an implement seat is spent reading, the journal records an
  implementation that did not happen, and the library crew substitutes
  the named crew at its first fallback.
- **Land it through `recipes/landing`.** Rejected for this shape. Landing
  remains the road for shop work on the library crew with a docs/code
  fork. A code-only found branch with a named crew would pay classify
  for a constant answer and still seat the wrong smith and judge.
- **Preflight it.** Rejected: `preflight` verifies and judges but ends
  at `review`; it has no seat to answer a finding and no ship to anchor
  the head, so a residual could never be remediated nor vouched.
- **Pin the crew as scoped offices (0058).** Rejected as disproportionate
  here. Scoped offices compose a whole strategy table across triage
  classes; two inline seats state the same force where the roster walk
  already asserts it, which is why `standby` is inline and `gpt-flash`
  is scoped. A later forced crew that spans a strategy table should be
  scoped; this one should not.
- **Let any recipe prefer a vendor inline.** Rejected: the exception is
  named, never generalized. A recipe that can take a fallback belongs in
  the library; the test names `review-first` by name rather than by a
  pattern, as it names `standby` and the wagers.

## Rulings

1. **`recipes/review-first` is `fast`'s constitution entered at
   `verify`.** The phases, rules, bounds, results, inputs, limits, and
   boxed verify/ship seats are `fast`'s verbatim; the only table
   differences are `initial` (`verify`) and the description. The run
   verifies the branch as found (`VERIFY-PASS` to `review`,
   `VERIFY-FAIL` to `implement` carrying the failing output), reviews
   verified code (`REVIEW-CLEAN`/`REVIEW-RESIDUAL-OK` to `ship`,
   `REVIEW-REFORGE` to `implement` while the bounded heats remain, with
   0041's exhaustion ladder unchanged), and ships through the same
   drift- and dirtiness-gated close-out. `implement` is the remediation
   seat only, entered by `VERIFY-FAIL` or `REVIEW-REFORGE` and bounded as
   `fast` bounds it. Review is unavoidable on every path to ship.

2. **The crew is forced inline, for 0041 ruling 7's reason.** The
   `implement` seat pins `dsh` with `--model
   meta-contributor/meta/muse-spark-1.3-contributor` at `xhigh`; the
   `review` seat pins `codex` with `--model gpt-6-astra` at `xhigh`
   under `--sandbox workspace-write`. The firing names those models, and
   a library chain would silently hire another vendor at its first
   fallback. `astra` holds the review gate as a declared codex judge
   (decision 0045; `adapters/codex.json` judges `astra` and `sol`);
   `implement` is work and needs no judges declaration (0041 ruling 3:
   dsh declares none). Both efforts are `xhigh`: the last judge before
   ship is hired at `xhigh` (0041 ruling 2), and a returned heat carries
   work the default crew would otherwise have done. Verify and ship stay
   boxed exec scripts with no model, as in `fast`.

3. **The exemption is keyed by the recipe name and asserted, never
   inferred.** The roster walk names `review-first` beside the wagers,
   `fast`, `standby`, `node`, `preflight`, and the `night-shift` /
   `research-dsh` lane exceptions, with this decision as the reason. A
   future inline site that is not part of a ruled forced crew is not
   admitted by a wildcard; it must be named, and naming it is the
   decision. The standard roster, its assignments, and its chains are
   untouched.

4. **Landing stays the road for shop work; review-first is the road for
   a found code branch with a named crew.** Prose-only found branches
   use landing's docs arm (`classify` to `review`, judged not built).
   Found code branches on the library crew use landing's code arm. Found
   branches — code, or needing proof before judgment — whose firing
   names the Muse/Astra pair use review-first's verify entry. Nothing in
   0051 moves; this decision names the complement it left open.

**Enforcement binding:** `recipes/review-first` — `bundle.json`,
`policy.json`, `README.md`, `roles/implementer.md`,
`roles/reviewer.md`, `scripts/verify-seat.sh`, `scripts/ship-seat.sh`;
`crates/brokkr-runtime/tests/roster.rs` names `review-first` in the
inline exception with this decision as the reason;
`crates/brokkr-runtime/tests/table_lints.rs` pins `review-first` at
`(46, 4)` — `fast`'s valuations and unruled, because the constitution
is `fast`'s;
`crates/brokkr-runtime/src/bundle/model_policy_tests.rs` counts eighteen
shipped bundles compiling under harness-namespace with fifteen
compiling under harness once the fragments are measured;
`crates/brokkr-cli/tests/contributing.rs` pins the `review-first` row
of the `CONTRIBUTING.md` recipe table against `brokkr recipes list`.
A determinable ruling with no named mechanism is judgment-guidance and
says so: ruling 4's routing guidance is judgment-guidance; rulings 1–3
are refused at compile time or by the walks above.

## Consequences

- **What moves.** One recipe (seven files), one decision, one row in
  `CONTRIBUTING.md`, three test pins that count it. No engine change,
  no contract change, no adapter change, no witness re-pin: the recipe
  duplicates `fast`'s constitution with one word changed, the sweep
  reads identically, and the witnesses do not name it.
- **What it costs.** A code found-branch delivery is a verify's minutes
  and one review seat at `xhigh` on each model seat. Remediation is
  spent only on a finding, never more than twice, as in `fast`.
- **What stays.** Decision 0041 rulings 2–3 and 5 (the roster, the judge
  vocabulary, every finding's edge and bound); decision 0045 (Astra
  judges); decision 0051 (landing, the classify fork, the label as
  residue). `preflight` stays the optional check that does not ship.
- **Named, not ruled.** Whether `review-first` later composes `fast` by
  `extends` (as `standby` does) instead of duplicating its table; whether
  a future forced crew with a strategy-spanning table should be scoped
  offices under 0058 rather than inline; whether a classify fork ever
  belongs in front of verify-entry. The operator may extend or retire
  the pattern later; this decision writes no general naming law.
