Status: proposed

## Why

Issue #352 (epic #330) found the browser's transcript body Claude-only and
looked up by bare session id (`/api/session/<id>`), while the TUI and
`brokkr transcript` read Claude, Codex and DSH by participant. The v0.11.0
notes already claim all three kinds are readable in the browser. The
implementation (commit on `story-352-one-transcript-path`) retired the
id-only routes and serves every kind by participant; this delta brings the
living `transcript-reading` spec to what the code does, as proposed
decision 0072 rules.

## What Changes

- **BREAKING:** `/api/session/<id>` and `/sse/session/<id>` are retired,
  with their three-field Claude envelope. No route looks a transcript up by
  a bare id.
- `/api/transcript/<run>/<key>` serves the participant's `brokkr transcript
  --json` document byte for byte (the command adds a trailing newline):
  HTTP 200 when readable, HTTP 404 carrying the refused document otherwise.
  `/sse/transcript/<run>/<key>` watches the same source grow.
- Codex and DSH references become drill-eligible. A Claude reference whose
  recorded home is not the local projects home stays ineligible, and the
  server now refuses it on both participant routes (HTTP 404,
  `{"error":"transcript not found"}`, before any transcript read), not only
  the page.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `transcript-reading`: the browser requirement, and the local-lookup,
  full-session, Claude-content and partial-records requirements whose text
  or scenarios named the retired routes, are restated for the participant
  routes. No reader, discovery, projection or cap rule changes.
- `transcript-tui`: one scenario's clause that leaned on the id-only lookup
  is restated.

### Not folded here

Two requirements, "One transcript derivation serves the local readers" and
"Discovery identifies one owned local file", still name the id-only routes
in four passages (the browser-drill sentence, the discovery-bounds sentence,
the id-only API and Claude SSE paragraphs, and the scenario "Claude browser
lookup refusals have fixed HTTP responses"). The still-active change
`2026-09-17-bound-transcript-projector` restates both requirements in full,
so editing them here and there makes new data clones that
`quality/jscpd-baseline-data.json` must admit, and that baseline could not be
regenerated from the implementing seat. Decision 0072 governs those
passages until they are folded, together with the same edits to that
change's copies.

## Impact

`crates/brokkr-cli/src/ui.rs`, `crates/brokkr-cli/src/ui.html` and their
tests; `docs/guides/read-surfaces.md`. Decision 0055 ruling 4's route
sentences are superseded by decision 0072; nothing else in 0055 moves.
