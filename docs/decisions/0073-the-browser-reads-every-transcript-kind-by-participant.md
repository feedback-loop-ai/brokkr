# 0073 — The browser reads every transcript kind by participant

Status: accepted — operator ruled 2026-09-27
Date: 2026-09-27

Built: built
Supersedes in part: 0055

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
The rule's original reason, the id-only route synthesizing the local home,
is gone with ruling 2, and the operator ruled on 2026-09-27 that the browser
reads what the command reads.

## Decision

1. **One route per purpose, keyed by participant.**
   `/api/transcript/<run>/<key>` serves the participant's `brokkr transcript
   --json` document byte for byte (the command adds a trailing newline),
   read through the one local read and masked against the store beside the
   journal: HTTP 200 when readable, HTTP 404 carrying the refused document
   otherwise. `/sse/transcript/<run>/<key>` watches the same source. The
   watch and the participant presentation admit that source by its
   reference and discovery and read no body, as the living spec's
   admission rule records: a body-stage refusal of an admitted source
   (`unreadable`, `unsupported-format`) is the body route's alone. An
   unparsable route or unknown participant is refused before any read. A
   run whose journal loads and does not fold is refused by the body route,
   the watch and the participant presentation in the fold's own words, as
   `brokkr transcript` refuses it, and no transcript is read.
2. **The id-only routes retire.** `/api/session/<id>`, `/sse/session/<id>`
   and their three-field Claude envelope are removed. No browser route looks
   a transcript up by a bare id. This supersedes 0055 ruling 4's sentences
   "Existing id-only Claude HTTP routes retain successful envelopes and the
   specified 404/SSE-loss behavior; Codex/DSH body routes remain absent" and
   its requirement that "every session label, id-only body request and
   growth watch" require a Claude kind.
3. **Eligibility: every valid reference of every kind.** A Claude, Codex or
   DSH reference that validates is drill-eligible, whatever home it was
   recorded under. A Claude reference under a home that is not the local
   projects home, and a legacy flat id under a local projects home that does
   not exist, are read as the command reads them.
4. **One function decides it.** The server computes eligibility by one
   function, which the presentation reports and the watch opens by; the page
   drills on that answer and keeps no rule of its own. The body route
   serves the command's bytes for every reference, readable or refused,
   behind the loopback Host guard.

## Consequences

- The browser shows Claude, Codex and DSH transcripts, and for every
  participant its body route answers what `brokkr transcript --json`
  prints. No surface refuses a reference for the home it was recorded
  under.
- 0055 ruling 4's eligibility sentence ("a drill was eligible only for a
  Claude reference recorded under that home") is superseded. Nothing else in
  0055 moves.
- The OpenSpec change
  `openspec/changes/archive/2026-09-27-352-browser-reads-by-participant/`
  restates `transcript-reading` for these routes and is folded into the
  living spec.
- `/api/view/<id>`, `/api/run/<id>` and the TUI pane still tolerate an
  unfoldable journal; bringing them to ruling 1's refusal is a follow-up.
