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
- **The graph fills its pane.** The operator then found the graph using about 40% of a 32" screen's pane, its labels shortened. The rail is laid out against the pane's width, measured and followed as it resizes: a wider pane spreads the phases, each arrow between phases growing up to 560 pixels, so a four-phase run fills a 1280 to 2500 pixel pane; road slots widen with it, and the labels then say their whole sentence. A narrower pane keeps the least spacing and scrolls to the current phase. The last phase's name and NOW box count in the rail's width.
- The TUI and `brokkr ui` read a road's `from`; what they draw is unchanged.

## Impact

- `crates/brokkr-view`: `Road`, `Phase.returns: Vec<Road>`; tests. The public-API snapshot gains `Road`.
- `crates/brokkr-cli`: `tui/layout.rs` and `ui.html` read `road.from`; their tests.
- `extensions/dsh/brokkr-console/`: `lib/host/readouts.js` (roads), `lib/client.js` (`roadsOf`, `railOf`, `usePaneWidth`, `Rail`); tests check that no label meets any road and no two roads share an arrowhead at 700, 1366 and 2560 pixels, and that a wide pane is filled up to the cap.
- Docs: `docs/guides/dsh-console.md`, `docs/guides/read-surfaces.md`.
- Specs: `operator-console-read`, one requirement modified.
