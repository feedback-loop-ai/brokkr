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


## Validation audit — 2026-09-09

Candidate inspected: `c966ef3a948f823e74b8dd63e34ff8de985562f1`. Host: Linux. This audit validates current
refusal behavior and specification structure, not native Seatbelt enforcement.

| Check | Observed result |
| --- | --- |
| `openspec validate boundary-seatbelt-slice-ii --strict --no-interactive` | PASS, exit 0. Structural validation does not resolve semantic contradictions. |
| `git diff --check` | PASS, exit 0. |
| `cargo test --locked -p brokkr-runtime --lib engine::boundary_tests:: -- --nocapture` | PASS, exit 0: 23 passed, 0 failed, 0 ignored, 349 filtered out. Includes unbuilt-boundary refusal before journal writes. |
| `cargo test --locked -p brokkr-cli --test boundary_verbs run_resume_and_rerun_refuse_an_unbuilt_boundary_before_the_journal -- --exact` | PASS, exit 0: 1 passed, 0 failed, 0 ignored, 7 filtered out. Proves run/resume/rerun refusal before journal writes or seat spawn. |
| `gh pr list --state all --head fire/0046-seatbelt --json number,state,title,url` | No matching PR returned. |
| `gh run list --branch fire/0046-seatbelt --limit 5 --json databaseId,headSha,conclusion,status,url` | No matching CI run returned. No remote native evidence found by this branch query. |

Source inspection confirms `built_boundary` in
`crates/brokkr-runtime/src/engine.rs` returns `Err("ii")` for Seatbelt;
`crates/brokkr-protocol/src/hands.rs` has no Seatbelt launcher implementation.
There is no native enforcement suite to execute for these guarantees in this
candidate. A bundle with no hands site is deliberately outside the unbuilt
box refusal: its execution is not evidence of a working Seatbelt boundary.

### Additional residual: SEATBELT-SPEC-RECONCILIATION

OPEN. The accepted addendum and proposal disposition are recorded, but the
capability deltas still require readable-empty masks, reject relocated
snapshots, and call R1–R4 upstream/unruled. Examples are in
`specs/seatbelt-execution/spec.md` under the mask, overlay and peer-gate
requirements. This was identified as preparation work in the ruling commit;
the audit confirms it remains outstanding. Reconcile these scenarios with
0046 before implementation. OpenSpec's structural PASS is not semantic
conformance. No design or tasks artifacts exist in this change yet.

**Verdict:** existing boundary refusal is demonstrated locally. R1–R4 native
guarantees are not demonstrated, remain OPEN and block Seatbelt activation
and completion. The full workspace suite and exact coverage were not run in
this focused audit, and no native macOS test was run or claimed.
