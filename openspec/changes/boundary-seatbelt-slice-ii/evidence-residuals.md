# Seatbelt acceptance evidence and open residuals

Recorded 2026-09-09 after the operator accepted the 0046 Seatbelt addendum.
This is a documentation inventory, not a mutation of the run journal or a
claim that native tests ran. All four guarantees remain unmeasured here.
The recording host is Linux; no macOS enforcement evidence was supplied.

| ID | Required demonstration | Current status | Next action |
| --- | --- | --- | --- |
| SEATBELT-R1 | Arbitrary overlay locator correctness; cross-call persistence; isolation across seats/attempts; host-source immutability and later-change invisibility; alias/mask handling; shipped Cargo/npm users | OPEN — native evidence missing | Reconcile locator specification, then exercise real overlays through MCP and exec on macOS. |
| SEATBELT-R2 | Masked secrets unreadable through direct/alias/overlap access; host bytes and entries unchanged after mutation attempts; ordinary neighbors usable | OPEN — native evidence missing | Encode accepted denied-read outcomes and run native adversaries with positive controls on both paths. |
| SEATBELT-R3 | No payload survives timeout, cancellation or supervisor death, including setsid/double-fork descendants; scratch cleanup follows process termination | OPEN — mechanism and native evidence missing; feasibility blocker | Identify and demonstrate a native lifetime mechanism first; record any failure as a residual. |
| SEATBELT-R4 | Ordinary Git works with empty private hooks; host hooks/config/routing resist direct writes and bypass attempts in primary and linked worktrees | OPEN — native evidence missing | Run native hook/config adversaries and verify protected host bytes after execution. |

## Closure requirements

For each row attach the candidate commit, macOS version/architecture, exact
command, exit status, named scenarios, positive-control outcomes and durable
logs or CI links. Both hands entry points must be covered. Report failures,
skips and unavailable facilities explicitly; none counts as a pass. Keep
implementation state separate from observed enforcement.

These residuals block boundary activation, full peer status and slice
completion. The operator accepted the semantics, not the evidence gaps as
shippable debt. Linux exact coverage and existing regression checks are
additional obligations, not substitutes for native enforcement. Preserve the
remaining slice requirements and their evidence. Recorded journal findings
must be closed through the existing 0047 lifecycle; this file does not close them.

## Preparation still required

- Reconcile the five capability deltas with the accepted 0046 addendum; the
  earlier readable-empty/same-path requirements and unruled hooks language
  record the previous upstream return, not the current operator ruling.
- Supply native lifetime feasibility evidence before full implementation.
- Keep the existing unbuilt refusal until all activation requirements pass.
