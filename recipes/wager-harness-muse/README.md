# wager-harness-muse — the Muse Spark arm

[`wager-harness`](../wager-harness/README.md)'s pattern, copied as it
asks to be: `fast` with the implement seat's driver swapped, and
nothing else. The one line of experiment here is
`{brokkr} driver dsh -- --model meta-contributor/meta/muse-spark-1.3-contributor --effort xhigh`.
The charter is a byte-identical copy of `recipes/fast/roles/implementer.md`;
limits, results, class, the phase table, and every gate are `fast`'s,
inherited.

The lane is Meta Model API's Muse Spark 1.3 under its **contributor**
id — the terms under which Meta may use prompts and completions to
improve its products, at roughly a twelfth of the standard id's price
— reached through the `dsh` adapter's `meta-contributor/` route
(decision 0036: the route is the prefix of the id, and here the id is
the terms). The material this arm sends is this public repository and
`fast`'s own charter. Egress class is `uncontracted`, the floor, and
dsh's adapter clearance is `untrusted`, so the arm holds a work seat
and no gate — the same standing as the deepseek arm.

## Parity, judged and recorded (2026-09-05)

- **Same tools.** As for [`wager-harness-dsh`](../wager-harness-dsh/README.md):
  `adapters/dsh.json` declares `tool_permissions: "unsupported"`, so
  the challenger runs with whatever dsh's headless profile permits in
  the seat's workdir, while the incumbent runs seven named `Bash`
  prefixes under `acceptEdits`. Not narrower than the incumbent as far
  as is known; not equal; not verified beyond that.
- **Same model class?** No, and that is the point: a cheap untrusted
  lane on a third vendor against the incumbent's fable, on the same
  commission, judged by the same gates on the incumbent.
- **Effort.** The `--effort xhigh` pin reaches the wire: the dsh
  driver writes it into a settings document of the seat's own, dsh
  reads that over the route's `reasoning:` default, and the request
  header echoes `reasoningEffort: xhigh` into the record (decision
  0035 addendum, 2026-09-05). The incumbent runs `high`. Effort is not
  a parity item — the wager measures the hire, and the hire includes
  its effort — but the comparison must name both.
- **Reasoning is encrypted.** Meta returns its reasoning as an
  encrypted block (`reasoning.encrypted`, format `meta-responses-v1`);
  dsh records the signature and an empty text block. The challenger's
  reasoning is therefore not readable in the transcript, where the
  incumbent's is. A judging seat reading trails sees less of this arm.
- **Reasoning across turns.** This arm reaches Meta over chat
  completions, which Meta documents as not carrying reasoning across
  turns; Meta points agentic work at its Responses API. If the arm
  loses the thread on long tool loops, that is a finding about the
  route, not the model, and the next arm is the `openai-responses`
  shape on the same dsh route.
- **Metering.** dsh reports token usage in its session record on this
  route (verified 2026-09-05), which the driver reads into the
  checkpoint, so tokens reach `brokkr costs`. LaneTally carries both
  ids' list rates under the `openrouter` provider key
  (feedback-loop-ai/lanetally#96), so the arm's tokens price the way
  the incumbent's do; the spend is also visible in OpenRouter's
  dashboard. The incumbent's spend is journaled per seat as usual.
- **Key.** `OPENROUTER_API_KEY` is exported into the challenger engine's
  launching environment only, never into argv, the recipe, or the
  journal (decision 0012). `brokkr doctor` warns, by route, that it is
  ambient; that warning is the design.
- **Served model.** Decision 0031's served `model` on this arm is what
  dsh's thread record says Meta returned — `muse-spark-1.3-contributor`
  — and is the proof the contributor terms, not the standard ones,
  were the ones billed.

Run as the harness README says: `brokkr run --recipe fast` for the
incumbent, `brokkr rerun --run <id> --recipe wager-harness-muse` for
this arm, `brokkr compare` for the trails, then judge the artifacts.

The harness inherits `fast`'s boxed verifier and shipper by construction.
Cargo verification runs offline from the bound registry cache; an
uncached dependency fails closed and its decisive line is quoted.
