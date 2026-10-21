# Change: a run with returns reads plainly in the console's run view (#568)

Status: proposed. Asked for by the operator on 2026-10-19: the run view "is quite convoluted when there are a few returns".
Parent story: [#568](https://github.com/feedback-loop-ai/brokkr/issues/568).

## Why

A run went implement, verify, review, back to implement on REVIEW-REFORGE, forward again through verify and review, then ship twice (SHIP-READY, then SHIP-COMPLETE). That is one return. The run graph drew four roads back with their labels printed over one another, and the seats table listed eight rows with nothing saying which pass each belonged to.

- **Brokkr counted arrivals as returns.** `brokkr-view`'s phase rail took any transition into a phase already entered as a road back. So did the second pass's forward steps (implement to verify, verify to review) and ship ruled onto itself. Its own field doc says a return is "the road in, not the count of arrivals".
- **The roads shared one lane.** Each further road dropped 6 pixels below the last, so labels overlapped, and each arc rose through the phase-name row to the rail.
- **The seats had no pass.** Nothing in the view told a seat of the second pass from one of the first.

## What Changes

- **A road back is a ruling that lands strictly earlier on the rail** than the phase it leaves, the rail's order being first entry. A phase ruled onto itself is never one. `visits` still counts every arrival, and the dedupe and journal order stand.
- **Each participant carries its `visit`**: which visit to its phase it was asked in, from 1, counted from the journal's `phase/entered` events before its request; `null` for a seat naming no phase the journal had entered. `VIEW_VERSION` moves 23 → 24.
- **The console's graph** gives each road its own lane below the phase names, the shortest nearest, its label a line of its own and haloed so a crossing arc does not strike it. Arcs leave and land below the names, and the rail is sized to its last label. The picked arrow is unchanged.
- **The console's seats table** names a later pass: a seat with `visit` 2 or more reads "implement · visit 2"; a first pass reads as before. The reader's seat heading says the same.
- **The run's bar** says whose running count it shows: "2 running in lanetally".

## Impact

- `crates/brokkr-view`: `phase_rail`'s backward rule; `Participant.visit` from `participant::visits`; `VIEW_VERSION` 24; tests in `src/tests/returns.rs`. The public-API snapshot gains `Participant::visit`.
- `crates/brokkr-cli`: `ui/tests.rs` reads the second pass's forward legs as no return, and the wire at 24.
- `extensions/dsh/brokkr-console/`: `lib/host/readouts.js` (wire 24, `visit`), `lib/client.js` (`railOf`, `Rail`, `visitOf`, `Seats`, `SeatFacts`, the run's bar), `lib/console.css`; tests.
- Docs: `docs/guides/dsh-console.md`, `docs/guides/read-surfaces.md`.
- Specs: `operator-console-read`, two requirements modified.
