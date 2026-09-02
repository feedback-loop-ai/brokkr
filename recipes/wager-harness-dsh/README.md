# wager-harness-dsh — the deepseek arm

[`wager-harness`](../wager-harness/README.md)'s pattern, copied as it
asks to be: `fast` with the implement seat's driver swapped, and
nothing else. The one line of experiment here is
`{brokkr} driver dsh -- --model deepseek/deepseek-v4-flash`. The charter is a
byte-identical copy of `recipes/fast/roles/implementer.md`; limits,
results, class, the phase table, and every gate are `fast`'s, inherited.

## Parity, judged and recorded (2026-09-02)

- **Same tools.** `adapters/dsh.json` declares `tool_permissions:
  "unsupported"`: the headless launcher has no allowed-tools flag. The
  challenger runs with whatever the harness permits — dsh's own
  `fs-sandbox` and code runtime in the seat's workdir — while the
  incumbent runs seven named `Bash` prefixes under `acceptEdits`. Not
  narrower than the incumbent as far as is known; not equal; not
  verified beyond that. The comparison must say so.
- **Same model class?** No, and that is the point: the wager measures
  a cheap untrusted lane against the incumbent's opus, on the same
  commission, judged by the same gates on the incumbent.
- **Metering.** The dsh seat reports no usage to the driver, and this
  arm reaches `api.deepseek.com` directly, so its spend is visible in
  the DeepSeek console and nowhere in `brokkr costs` or LaneTally. The
  incumbent's spend is journaled per seat as usual. An asymmetry of
  evidence, recorded here before the run.
- **Key.** `DEEPSEEK_API_KEY` is exported into the challenger engine's
  launching environment only, never into argv, the recipe, or the
  journal (decision 0012).

Run as the harness README says: `brokkr run --recipe fast` for the
incumbent, `brokkr rerun --run <id> --recipe wager-harness-dsh` for
this arm, `brokkr compare` for the trails, then judge the artifacts.
