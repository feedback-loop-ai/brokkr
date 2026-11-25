# 0123 — One workspace call may run an hour

Status: accepted — operator ruled 2026-11-25, accepting it with its three sibling engine patches from feedback-loop-ai/swarmail (decisions 0120 to 0123, pull requests #1291 to #1294) and landing each by hand
Date: 2026-11-23

Built: built

## Context

The `workspace` tool of decision
[0043](0043-the-hands-are-one-tool.md) takes a command and a
`timeoutMs`. Its ruling 1 bounds time by the call and names no number;
the code defaults a call to 30 seconds and refuses one that asks for
more than 600,000 milliseconds, ten minutes
(`crates/brokkr-protocol/src/hands.rs`, `MAX_TIMEOUT_MS`). The tool's
schema advertises the same maximum to the harness.

Ten minutes is shorter than the work a verify or implement seat on a
Rust workspace must do in one command: a cold `cargo build` of a
workspace this size, then its full test suite, routinely outlasts it on
a shared host. A seat whose only tool is `workspace` then cannot run its
own gate in one call; it must split a build that does not split, or
background it and poll, which the box (a private `/tmp` and pid
namespace per call) does not carry from one call to the next.

The adopter `feedback-loop-ai/swarmail` has carried a patch raising the
bound to an hour on brokkr 0.12.0 since 2026-10-29
(`tooling-patches/0004-raise-hands-command-timeout.patch`, release
`swarmail-engine-a9893b4-p4`), and asks for it upstream so it can pin a
stock release (feedback-loop-ai/swarmail#405).

## Rulings

1. **A workspace call may ask for up to an hour.** `timeoutMs` admits an
   integer from 100 through 3,600,000; the default stays 30,000, so a
   call that names no timeout is bounded as before. The bound the call
   check holds is the one the tool's schema and description advertise.
   A seat's own `timeout_seconds` still bounds the whole session, and a
   call that outlives it ends with the seat's attempt, as it always has.

   **Enforcement binding:** `MAX_TIMEOUT_MS` in
   `crates/brokkr-protocol/src/hands.rs`; tests in
   `crates/brokkr-protocol/src/hands/call_refusal/tests.rs`: 600,001 and
   3,600,000 are admitted and 3,600,001 refused; the refusal's text names
   the new bound; the advertised schema's `maximum` and the description
   name the bound the check holds.

## Consequences

- A seat that asks for more than ten minutes in one call is admitted
  where it was refused. No shipped recipe or charter asks for a timeout,
  so no shipped run changes unless its model chooses a longer one.
- One call can now hold its box, and the CPU its command uses, for up to
  an hour inside the seat's own deadline. The seat's `timeout_seconds`
  (3,600 by default) remains the outer bound, and decision 0104's
  deadline floor for shared hardware is unchanged.
- A harness may bound an MCP tool call by its own client-side timeout;
  this decision raises Brokkr's bound only, and does not measure or
  configure a harness's.
- No contract, fixture or witness digest changes: the tool definition
  is generated, and no digest covers it.

## Alternatives

- **A per-site bound in the hands spec.** It would let a recipe choose
  ten minutes or an hour per seat, at the cost of a new `hands` key, its
  validation and its docs, for a bound no shipped recipe has asked to
  lower. Left for when one does.
- **No bound but the seat's deadline.** Rejected: a call with no
  ceiling of its own leaves a mistyped timeout to the outer watchdog,
  and the refusal is the operator's signal that a model asked for
  something unreasonable.
