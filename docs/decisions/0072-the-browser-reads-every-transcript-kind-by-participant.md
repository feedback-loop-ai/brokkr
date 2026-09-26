# 0072 — The browser reads every transcript kind by participant

Status: proposed (implementer, 2026-09-27)
Date: 2026-09-27

## Context

Decision 0055 ruling 4 kept the browser's transcript body Claude-only: the
id-only `/api/session/<id>` and `/sse/session/<id>` routes looked a session
up by bare id under the server's local `HOME/.claude/projects`, a drill was
eligible only for a Claude reference recorded under that home, and "Codex/DSH
body routes remain absent". The TUI and `brokkr transcript` read all three
kinds by participant, and the v0.11.0 notes claim the browser does too.
Issue #352 (epic #330) asks for one participant-keyed route returning the
`brokkr transcript --json` document and a watch keyed the same way, and for
`/api/session` and `claude_source*` to retire.

The review of that change found the foreign-home Claude rule enforced by the
page alone: the presentation answered `drill_eligible: false` while the new
routes served the prose and opened the stream. On `main` no browser route
could serve that file, because the id-only route read the local home only.

## Decision

1. **One route per purpose, keyed by participant.**
   `/api/transcript/<run>/<key>` serves the participant's `brokkr transcript
   --json` document byte for byte (the command adds a trailing newline),
   read through the one local read and masked against the store beside the
   journal: HTTP 200 when readable, HTTP 404 carrying the refused document
   otherwise. `/sse/transcript/<run>/<key>` watches the same source. An
   unparsable route or unknown participant is refused before any read.
2. **The id-only routes retire.** `/api/session/<id>`, `/sse/session/<id>`
   and their three-field Claude envelope are removed. No browser route looks
   a transcript up by a bare id. This supersedes 0055 ruling 4's sentences
   "Existing id-only Claude HTTP routes retain successful envelopes and the
   specified 404/SSE-loss behavior; Codex/DSH body routes remain absent" and
   its requirement that "every session label, id-only body request and
   growth watch" require a Claude kind. Nothing else in 0055 moves.
3. **Eligibility: every kind, less a foreign Claude home.** A valid Codex or
   DSH reference is drill-eligible. A Claude reference is eligible only when
   its recorded home is canonically the local projects home, as before.
4. **The server enforces eligibility, not only the page.** The presentation,
   the body route and the watch answer by one function. An ineligible
   reference gets HTTP 404 with `{"error":"transcript not found"}` from both
   routes, before any transcript file is opened and before any event-stream
   header is written.

## Consequences

- The browser shows Codex and DSH transcripts. A foreign-home Claude
  reference drills nothing on any surface of the browser, while `brokkr
  transcript` and the TUI still read it from its recorded home. For that one
  case the route's bytes differ from the command's by design.
- The OpenSpec change
  `openspec/changes/archive/2026-09-27-352-browser-reads-by-participant/`
  restates `transcript-reading` for these routes and is folded into the
  living spec, except four passages in two requirements that an active
  change also restates; its proposal names them, and this decision
  governs them until they are folded.

## For the operator to rule

Ruling 4 keeps a refusal whose original reason, the id-only route
synthesizing the local home, is gone with ruling 2. The alternative is to
retire the foreign-home rule on every surface, so the browser reads what the
command reads. This decision takes the fail-closed choice and leaves the
retirement to a ruling.
