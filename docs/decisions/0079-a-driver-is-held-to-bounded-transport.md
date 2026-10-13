# 0079 — A driver is held to a bounded transport

Status: accepted — operator ruled 2026-10-10 (#433). The same day the operator ruled that the host configuration configures the limits (ruling 6).
Date: 2026-10-10

Built: partial (#433) — the host configuration does not carry the limits yet; they are constants

## Context

The driver transport (`crates/brokkr-protocol/src/process.rs`) read each
stdout line with an unbounded `read_line`, drained stderr whole with
`read_to_end`, and kept every checkpoint until the attempt finished
(#433). A runaway or buggy driver, a line with no newline, a stderr flood
or a checkpoint loop, could exhaust the engine's memory. The operator's
2026-09-26 threat model asks a gate to catch realistic accidental misuse
and fail closed on what it cannot read. A refusal is journaled, and the
journal is append-only, so a refusal that quoted the driver's bytes
could copy a secret there for good (decision 0012).

## Decision

1. **Four limits, as named constants with their units** in
   `process/limits.rs`. A stdout protocol line is at most 16 MiB before
   its newline (`FRAME_BYTES`); a result is one line, so this is its
   limit too. The report keeps at most 1 MiB of stderr (`STDERR_BYTES`).
   An attempt journals and keeps at most 100,000 checkpoints
   (`CHECKPOINTS`) and 64 MiB of checkpoint lines (`CHECKPOINT_BYTES`).
   Each is a count compared without overflow: zero admits nothing, and
   no value means unlimited.
2. **A line is refused before it is held whole.** It is refused at the
   first byte past the frame limit, before it is decoded as UTF-8 or
   parsed as JSON, so an endless line and invalid UTF-8 are refused
   bounded. A checkpoint past either checkpoint limit is neither
   journaled nor kept.
3. **A driver past a frame, result or checkpoint limit ends its attempt
   indeterminate.** It may already have had effects, so the attempt is
   never reported succeeded, nor failed in a way a retry may safely
   repeat (decision 0006), as #394's held-bytes overflow ends one. Its
   process tree is ended through #403's bounded teardown.
4. **Stderr past its limit is no failure.** The pipe is drained to its
   end so the driver never blocks on it, and the report keeps the first
   and last halves of the limit around a marker counting the bytes
   dropped.
5. **Refusal evidence carries no content.** A refused line is named by
   its length, the class of its first byte and the first 16 hex digits
   of its SHA-256, never by its bytes. A stdout line that is not a
   protocol message is named the same way, with the kind and place of
   its JSON error, whose own text can quote the input and is never used.
   A checkpoint refusal names the counts held: checkpoints, their bytes,
   and the bytes of the one refused.
6. **The host configures the limits.** Ruled by the operator on
   2026-10-10: the host configuration (`forge.host/v1`, the machine-scoped
   file #430 introduces) declares them, since a transport's memory bound
   is a fact about the machine, not about a realm, an adapter or a
   recipe. Until that file carries them they are constants, injectable
   only through the transport's private constructor for tests.

## Consequences

New runs journal the content-free text for an unreadable driver message;
existing journals and fixtures are unchanged, and no contract moves. The
loopback UI's limits, spill files and their cleanup, and the budget across
the contention hold, the arrivals channel and the report remain with
#433's later slices and #464.

## Addendum — 2026-10-11, accepted 2026-10-13: the loopback UI is held to the same rules

Status: accepted — operator ruled in chat 2026-10-13 (#433 slice 2,
built in #635). The rulings
above stand; this carries them to the one other surface that reads bytes
it does not control, the `brokkr ui` loopback server
(`crates/brokkr-cli/src/ui.rs`). It read the request line and headers
with an unbounded `read_line`, set no read or write timeout, and started
a thread for every connection it accepted. The UI binds 127.0.0.1 only,
so the case is the operator's 2026-09-26 one: a stuck or runaway local
client, a hung tab, a script that never closes, a request with no end of
headers, exhausting threads or memory. No remote exploit is claimed.

- **Five limits, as named constants with their units** in
  `ui/limits.rs`. The request line is at most 8 KiB, its line ending
  included (`REQUEST_LINE_BYTES`); the header section at most 32 KiB,
  every header line and the blank line ending them included
  (`HEADER_BYTES`). The request must arrive within 10 seconds of its
  accept, each read waiting only for the time left (`READ_TIMEOUT`); a
  write waits at most 10 seconds for the client to read
  (`WRITE_TIMEOUT`). At most 64 connections are served at once
  (`CONNECTIONS`). Each is compared without overflow: zero admits
  nothing, and no value means unlimited. A zero write timeout is refused
  by the platform, so that connection closes unanswered.
- **A line is refused before it is held whole**, at the first byte past
  its limit, as ruling 2 refuses a driver's line.
- **Excess work is refused before a thread exists.** The acceptor takes a
  slot before it starts a connection's thread, and the thread gives it
  back when it ends, on every return and on unwind. A connection past the
  cap is answered at once from the acceptor with a write that does not
  block, and closed: it is never queued and never given a thread.
- **A server-sent stream counts against the cap** for as long as it is
  open. It keeps its one-second heartbeat, so a client that has gone is
  found by the heartbeat's failing write and its slot returns within two
  polls; a client that stops reading is ended by the write timeout once
  the kernel's send buffer is full.
- **A refusal carries no content.** An over-long request line answers
  414, an over-large header section 431, a request that did not arrive in
  time 408, and a connection past the cap 503, each with a fixed body
  naming the limit and its unit, never the client's bytes (ruling 5).
- **The host configures these too**, by ruling 6's reasoning: a
  surface's memory and thread bound is a fact about the machine. Until
  `forge.host/v1` carries them they are constants, injectable only through
  the private `Limits` the server's functions take, for tests.
