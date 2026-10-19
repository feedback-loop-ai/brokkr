# Change: the roads back are square, nested and counted (#568)

Status: proposed. Asked for by the operator on 2026-10-19, after previewing `2026-10-19-568-run-view-returns` (`f70780cc`) on a run with two returns into implement (VERIFY-FAIL, then REVIEW-REFORGE).
Builds on: `2026-10-19-568-run-view-returns`.
Parent story: [#568](https://github.com/feedback-loop-ai/brokkr/issues/568).

## Why

Both roads landed on one arrowhead under implement, and the outer road cut through the inner road's label. The operator kept the single rail with roads beneath, and asked for them tidied and counted.

## What Changes

- **Each road is counted.** A phase's `returns` becomes a list of roads, `{from, taken, rules}`: how many times the journal ruled that road, and each rule that ruled it once, the most recent last. A ruling with no rule adds to `taken` alone. The wire stays at 24, which has not been released.
- **Roads run square and nested.** Each road leaves from its own slot along the phase that sent the run back and lands on its own slot along the phase it was sent to, so no two share an arrowhead. Along a phase, departures stand left of landings; the longest departure is rightmost and the longest landing leftmost. A road runs down, across its lane and up, with rounded corners. Its lane is below every road it encloses, so the longest runs outermost and no road crosses another's legs.
- **Each label stays clear.** A label hangs under its own lane, centred on its span, and widens only into space no deeper road's leg and no road beside it takes. It says as much as fits: the whole sentence ("sent back to implement by verify · VERIFY-FAIL ×1"), then who sent it back with the rules and count, then the rules and count. The whole sentence is its title. The rail is sized to fit.
- **The rail is as wide as its last phase.** The last phase's name and NOW box count in the rail's width, so neither is clipped at the rail's right edge. (Spreading the rail to fill a wide pane was tried on 2026-10-19 and withdrawn: the operator chose the fixed, compact spacing.)
- The TUI and `brokkr ui` read a road's `from`; what they draw is unchanged.

## Impact

- `crates/brokkr-view`: `Road`, `Phase.returns: Vec<Road>`; tests. The public-API snapshot gains `Road`.
- `crates/brokkr-cli`: `tui/layout.rs` and `ui.html` read `road.from`; their tests.
- `extensions/dsh/brokkr-console/`: `lib/host/readouts.js` (roads), `lib/client.js` (`roadsOf`, `railOf`, `Rail`); tests check that no label meets any road and no two roads share an arrowhead.
- Docs: `docs/guides/dsh-console.md`, `docs/guides/read-surfaces.md`.
- Specs: `operator-console-read`, one requirement modified.
