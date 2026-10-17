# The Library's boards

These are the design boards the operator agreed on 2026-10-17. They are Design Component pages (`.dc.html`): plain HTML with a small script. Their figures are swarmail's journal for recipe `self` since 10 October, read on 17 October. They are aggregates only, with no session content.

- `L1Graph.dc.html` is **the Graph view**: the recipe's state machine, with each rule's count on its edge, dormant rules dashed, `stop` beneath, the pulse on a live run, and a side panel for the picked state.
  - **In scope:** everything except the red escape markings (the dashed red rings and arrows, the "Escapes" figure and the "slipped past" boxes).
  - **Out of scope:** the bench actions and the "repair edge" suggestions, which are drawn as hints only. Escapes need a check-owns-class map that Brokkr does not record yet.
- `L1Pareto.dc.html` is **the Graph Pareto frontier**: panel 1, the four classified quadrants with the operator's targets and the figure table beneath.
  - **Out of scope:** panels 2 and 3 of this board (the ranking by escapes and the isolated experiment). They belong to later slices, which need escapes and the bench's work-order capture.

Copy rules carried into the console:
- human names, never hashes;
- "done", "stop", "stopped by you";
- cost only where it was reported, with the silent providers named.
