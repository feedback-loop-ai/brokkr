# wager-harness-dsh — the GLM-flash arm

`fast` with one seat changed, as
[`wager-harness`](../wager-harness/README.md) asks a wager to be built:
the implement seat's driver is swapped and nothing else is. The
experiment is one office, `agents/wager-glm-flash-implementer.json`, an
overlay of `implementer` that resolves to
`{brokkr} driver dsh -- --model spark-glm/GLM-5.3-Flash-EXL3`, the dsh
adapter's `glm-flash` alias on the `spark-glm` route (the operator's
ruling of 2026-10-04,
[#532](https://github.com/feedback-loop-ai/brokkr/issues/532)). The
route is effortless: dsh 0.1.5-rc.1 refuses a reasoning effort on it
(measured 2026-09-16), so the arm pins no `--effort`, and its record
reads effort `not applicable`, as the adapter's `effortless_routes`
entry for `spark-glm` says. Its chain holds that one model and no
fallback, and the recipe declares `forced_crew` (decision 0041's addendum
of 2026-10-07): a wager forces the arm it measures. The charter is
fast's own, the library's `charters/implementer-intakeless.md`; limits,
results, class, the phase table, and every gate are `fast`'s, inherited.

## Parity, judged and recorded (2026-09-02)

- **Same tools.** `adapters/dsh.json` declares `tool_permissions:
  "unsupported"`: the headless launcher has no allowed-tools flag. The
  challenger runs with whatever the harness permits — dsh's own
  `fs-sandbox` and code runtime in the seat's workdir — while the
  incumbent runs under `acceptEdits` with five `Bash` prefixes
  pre-approved through `--allowedTools`, which removes no tool, so
  neither arm is held to a named list: each is bounded by its own
  harness and the operator's settings. Not equal; not verified beyond
  that. The comparison must say so.
- **Same model class?** No, and that is the point: the wager measures
  a local untrusted lane against the incumbent's opus, on the same
  commission, judged by the same gates on the incumbent.
- **Metering.** The dsh seat reports no usage to the driver, and this
  arm's model calls go to the host's own `spark-glm` server, a `local`
  route, so no provider bills them and neither `brokkr costs` nor
  LaneTally prices them. The incumbent's spend is journaled per seat as
  usual. An asymmetry of evidence, recorded here before the run.
- **Key.** The route names `SPARK_API_KEY`. Its server checks no key,
  but the route requires one to be named. It is exported into the
  challenger engine's launching environment only, never into argv, the
  recipe, or the journal (decision 0012).

**Host requirement: the `spark-glm` route.** This arm runs only where
the host's dsh profile serves that route; no CI runner does. Anywhere
else the implement attempt fails at launch, and its journaled failure
names the route: the dsh driver ends a failed seat's stderr tail with
`dsh driver: dsh exited <code> with the pinned model
spark-glm/GLM-5.3-Flash-EXL3 on route spark-glm`. How dsh's own refusal
is worded there has not been measured.

Run as the harness README says: `brokkr run --recipe fast` for the
incumbent, `brokkr rerun --run <id> --recipe wager-harness-dsh` for
this arm, `brokkr compare` for the trails, then judge the artifacts.

The harness inherits `fast`'s verifier and shipper by construction, boxed
only where the realm's boundary is `namespace`.
Cargo verification runs offline from the bound registry cache; an
uncached dependency fails closed and its decisive line is quoted.
