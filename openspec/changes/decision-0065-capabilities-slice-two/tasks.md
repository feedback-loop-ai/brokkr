# Decision 0065 slice two — implementation tasks

Status: proposed; all implementation and measurement tasks remain open.
The documents have been authored in this visit. That does not complete U0,
qualify a harness, accept decision 0077 or close any checkbox below.

Follow design.md's final **Slice two units** section: one PR per row, from
main, signed and through the merge queue after the operator rules. Each group
below belongs to exactly one row, with its exact production/test/evidence
inventory and three-production-file ceiling. Additional files require a
documented split before editing. U1–U4 are independent of each other; U1/U4
need U0 measurements, U2/U3 do not. U9b alone enables compilation of MCP grants.

For every behavioral group, completion includes exact values/typed variants,
one exact text pin per module, a compiling behavior-removal mutation caught
by each new test, restoration and recorded pass. Use shared builders and
unwind-safe environment guards. Record actual revision, command and evidence,
not an invented baseline failure. All groups owe SD2's file-budget audit and
SD4's applicable gates; the verification task in each group names this duty.

Implementation gates: cargo test --workspace; cargo run --locked -p
brokkr-cli -- compile --bundle bundles/self; cargo fmt --all -- --check;
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings;
unchanged exact coverage on a capable external host. Final U10b adds locked
all-feature workspace tests, bundles/verify, strict OpenSpec, diff check,
supported-host/remote evidence and pinned-toolchain agreement. Measure changed
witness/compose identities in their owning rows. External gates stay pending
until results exist. No test or production edits are authorized in this
document visit.

## 1. U0 — Measure isolation and telemetry

Dependencies: none; independent objective. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 1.1 (U0; [SI1](specs/strict-mcp-isolation/spec.md), [SD1](specs/slice-two-delivery/spec.md)) Execute D2's complete positive/sentinel matrix; record raw redacted observations and adapter-specific verdicts, including native call identifiers. This PR changes evidence only. Verify: A reviewer can reproduce every supported row; missing rows are unmeasured and cannot license U1/U4.
- [ ] 1.2 (U0; [SI1](specs/strict-mcp-isolation/spec.md), [SD1](specs/slice-two-delivery/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) Review the listed documents/evidence against the named requirements and retain explicit unmeasured/pending outcomes. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 2. U1a — Extract existing MCP transport checks

Dependencies: U0. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 2.1 (U1a; [SI2](specs/strict-mcp-isolation/spec.md), [MB1](specs/mcp-capability-broker/spec.md), [SD2](specs/slice-two-delivery/spec.md)) Move the current single-server transport/parser checks into the named module, retaining production callers and exact behavior; create room under the existing file baseline. Verify: Unchanged hands-only exact-state and authored-option tests, with no added server acceptance.
- [ ] 2.2 (U1a; [SI2](specs/strict-mcp-isolation/spec.md), [MB1](specs/mcp-capability-broker/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 2.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 3. U1b — Type adapter MCP facts

Dependencies: U1a. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 3.1 (U1b; [SI1](specs/strict-mcp-isolation/spec.md), [SI2](specs/strict-mcp-isolation/spec.md)) Extract McpSupport and its loader into agents/mcp.rs; represent measured, unsupported and unmeasured isolation by invocation shape and carriage separately. Consume the type at load; legacy servers never grant authority. Verify: Strict closed decoding, old unsupported data and independent wrapper evidence have exact variants.
- [ ] 3.2 (U1b; [SI1](specs/strict-mcp-isolation/spec.md), [SI2](specs/strict-mcp-isolation/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 3.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 4. U1c — Build isolated serving configurations

Dependencies: U1b. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 4.1 (U1c; [SI2](specs/strict-mcp-isolation/spec.md), [MB1](specs/mcp-capability-broker/spec.md)) Factor existing config assembly into adapters/mcp.rs and implement only U0-qualified isolation shapes. Typed engine input crosses the private serving edge; the current no-broker plan is empty or hands-only. Verify: Exact cold/resume/replacement configuration, auth/session controls and missing evidence refusals; the module is used by existing launch builders.
- [ ] 4.2 (U1c; [SI2](specs/strict-mcp-isolation/spec.md), [MB1](specs/mcp-capability-broker/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 4.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 5. U1d — Record Claude, Codex and LaneTally declarations

Dependencies: U1c. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 5.1 (U1d; [SI1](specs/strict-mcp-isolation/spec.md), [SI2](specs/strict-mcp-isolation/spec.md)) Populate declarations from U0 with exact evidence scope; unsupported/unmeasured is a valid outcome, never guessed support. Verify: Adapter-load and whole-file identity tests pin only observed facts.
- [ ] 5.2 (U1d; [SI1](specs/strict-mcp-isolation/spec.md), [SI2](specs/strict-mcp-isolation/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 5.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 6. U1e — Record dsh and exec declarations

Dependencies: U1d. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 6.1 (U1e; [SI1](specs/strict-mcp-isolation/spec.md), [SI2](specs/strict-mcp-isolation/spec.md)) Record dsh's separate exclusion/carriage verdict and exec's inapplicable model surface. No plugin or harness is added. Verify: No inheritance from Claude, no fabricated dsh hands support, and exec remains a script path.
- [ ] 6.2 (U1e; [SI1](specs/strict-mcp-isolation/spec.md), [SI2](specs/strict-mcp-isolation/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 6.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 7. U1f — Thread independent strict intent

Dependencies: U1e. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 7.1 (U1f; [SI2](specs/strict-mcp-isolation/spec.md), [MB1](specs/mcp-capability-broker/spec.md)) Extract capability-relevant candidate composition from bundle into bundle/mcp.rs. Carry strict empty/hands intent through candidates and inline site facts independently of emitted config. Verify: Primary/fallback and nested sites retain distinct intended sets; equality of authored bytes never supplies origin.
- [ ] 7.2 (U1f; [SI2](specs/strict-mcp-isolation/spec.md), [MB1](specs/mcp-capability-broker/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 7.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 8. U1g — Seal and enforce every launch

Dependencies: U1f. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 8.1 (U1g; [SI2](specs/strict-mcp-isolation/spec.md), [SD2](specs/slice-two-delivery/spec.md)) Bind U1f facts at dispatch and consume the final checked isolated configuration at all serving builders. Mandatory strict admission is activated with this complete path, including no-ask sites. Shrink engine composition by using existing extracted helpers. Verify: All SI2 shapes and final isolation removal fail exactly; every shipped compile either passes measured support or reports its exact unmeasured refusal, never a filename exemption.
- [ ] 8.2 (U1g; [SI2](specs/strict-mcp-isolation/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 8.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 9. U2 — Remove both native-binding panics

Dependencies: none; independent objective. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 9.1 (U2; [SC5](specs/slice-two-contracts/spec.md), [SD2](specs/slice-two-delivery/spec.md)) Extract kind-specific binding projection and replace the indexed native binding and doctor expect with exhaustive typed outcomes. Native behavior and the public MCP fence stay unchanged. Verify: Direct non-native seam fixtures prove both panic sites fixed; native controls and the old compile refusal are exact.
- [ ] 9.2 (U2; [SC5](specs/slice-two-contracts/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 9.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 10. U3a — Apply gate classes

Dependencies: none; independent objective. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 10.1 (U3a; [GP1](specs/gate-capability-policy/spec.md)) Thread canonical executable SeatClass and stable office once; extract the common pure class check. Writes precedes egress; D4 scope and independent native OFF remain. Verify: Exact native reads/writes/egress requires/wants, all nested sites, subtraction and fallback tests; helper-level MCP cases do not bypass compile.
- [ ] 10.2 (U3a; [GP1](specs/gate-capability-policy/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 10.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 11. U3b — Check loaded office charters

Dependencies: U3a. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 11.1 (U3b; [GP2](specs/gate-capability-policy/spec.md)) Factor the existing charter-byte read through its verified binding and check every loaded requesting office, including unseated/subtracted asks. Use the bounded paragraph scanner. Verify: Existing researcher prose passes; absent, nearby, repeated, fenced and substring-only names refuse with exact causes.
- [ ] 11.2 (U3b; [GP2](specs/gate-capability-policy/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 11.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 12. U3c — Check inline requester charters

Dependencies: U3b. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 12.1 (U3c; [GP2](specs/gate-capability-policy/spec.md)) Extract inline verified-charter consumption into bundle/charters.rs and use the same scanner for every executable inline body; no new reopening of unchecked paths. Verify: Inline, selected/inherited/member/step cases, preserved dispatch pins and fixed DATA reminder; no duplicated scanner.
- [ ] 12.2 (U3c; [GP2](specs/gate-capability-policy/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 12.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 13. U4a — Make room for additive record validation

Dependencies: U0. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 13.1 (U4a; [CC3](specs/capability-call-checkpoints/spec.md), [SC4](specs/slice-two-contracts/spec.md)) Extract existing validation functions into a consumed child module; preserve dispatch and export/verify behavior. Verify: Historical version and exact refusal tests stay green; the oversized parent shrinks.
- [ ] 13.2 (U4a; [CC3](specs/capability-call-checkpoints/spec.md), [SC4](specs/slice-two-contracts/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 13.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 14. U4b — Publish and consume seat-record v6

Dependencies: U4a. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 14.1 (U4b; [CC3](specs/capability-call-checkpoints/spec.md), [SC4](specs/slice-two-contracts/spec.md)) Add the schema and its exact embedded copy, select the engine-line boundary on then-current main, and use v6 at append/export/verify. Older valid rows remain valid. Verify: Frozen-byte pins, embedded equality and each field/state/dependency boundary are exact; no package version bump solely for dispatch.
- [ ] 14.2 (U4b; [CC3](specs/capability-call-checkpoints/spec.md), [SC4](specs/slice-two-contracts/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 14.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 15. U4c — Normalize harness call observations

Dependencies: U4b. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 15.1 (U4c; [CC1](specs/capability-call-checkpoints/spec.md), [CC2](specs/capability-call-checkpoints/spec.md)) Extract telemetry normalization at the harness edge; parse measured call identifiers/server/tool fields and deduplicate native start/completion without carrying raw Value state. Carry typed observations through the existing opaque driver data edge. Verify: Claude/Codex/dsh measured format fixtures, duplicate and missing identity controls; unrelated ordinary checkpoints unchanged.
- [ ] 15.2 (U4c; [CC1](specs/capability-call-checkpoints/spec.md), [CC2](specs/capability-call-checkpoints/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 15.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 16. U4d — Bind attribution to compiled holdings

Dependencies: U4c. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 16.1 (U4d; [CC1](specs/capability-call-checkpoints/spec.md), [SC4](specs/slice-two-contracts/spec.md)) Build a typed reverse attribution index from each selected native holding and adapter inventory; compile-refuse ambiguous or unrepresentable names. Extract existing projection logic to keep parents below baseline. Verify: Exact selected dialect/tool, long-name and ambiguous-map cases, no substring matching or inventory-only grant.
- [ ] 16.2 (U4d; [CC1](specs/capability-call-checkpoints/spec.md), [SC4](specs/slice-two-contracts/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 16.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 17. U4e — Stamp single and panel calls

Dependencies: U4d. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 17.1 (U4e; [CC1](specs/capability-call-checkpoints/spec.md), [CC3](specs/capability-call-checkpoints/spec.md)) Extract common checkpoint attribution beside current boundary/site stamps. Erase untrusted authority fields, assign attempt-owned native IDs and keep local calls unattributed; fail known-unheld observations. Verify: Native compile-to-journal proofs for ordinary, inline, fallback and panel calls; spoofed stamps cannot survive.
- [ ] 17.2 (U4e; [CC1](specs/capability-call-checkpoints/spec.md), [CC3](specs/capability-call-checkpoints/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 17.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 18. U4f — Bind sequence and resumed observations

Dependencies: U4e. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 18.1 (U4f; [CC1](specs/capability-call-checkpoints/spec.md), [CC2](specs/capability-call-checkpoints/spec.md)) Route step and nested-member observations through the same stamp, preserve session eligibility and ensure cold replacement has a new attempt identity. Verify: Separate actual resumed and cold replacement tests, sibling ownership and repeated-call IDs.
- [ ] 18.2 (U4f; [CC1](specs/capability-call-checkpoints/spec.md), [CC2](specs/capability-call-checkpoints/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 18.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 19. U4g — Derive call evidence once

Dependencies: U4f. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 19.1 (U4g; [CC2](specs/capability-call-checkpoints/spec.md), [CC3](specs/capability-call-checkpoints/spec.md), [CR5](specs/capability-response-retention/spec.md)) Extract checkpoint-derived call facts into a pure view module, consumed by the existing view. One call ID counts once; historical absence is unrecorded. Verify: Literal view values for native observed and synthetic broker stages, no I/O or grant admission in view.
- [ ] 19.2 (U4g; [CC2](specs/capability-call-checkpoints/spec.md), [CC3](specs/capability-call-checkpoints/spec.md), [CR5](specs/capability-response-retention/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 19.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 20. U5a — Extract dialect loading

Dependencies: U2. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 20.1 (U5a; [SC1](specs/slice-two-contracts/spec.md), [SD2](specs/slice-two-delivery/spec.md)) Move the current dialect edge loader and contained read use to a consumed module without changing v1 acceptance or the MCP fence. Verify: Existing schema/duplicate/containment and refusal tests prove extraction parity.
- [ ] 20.2 (U5a; [SC1](specs/slice-two-contracts/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 20.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 21. U5b — Retain typed MCP connection and policy

Dependencies: U5a. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 21.1 (U5b; [SC1](specs/slice-two-contracts/spec.md), [MB4](specs/mcp-capability-broker/spec.md)) Carry typed v1 connection, version, names, retained, egress and sends. Use exhaustive kind and retention variants; reject runtime use of references/URL only through the specified compatibility causes after enablement. Verify: Exact field retention/digests, no process/store read, old native data and pre-U9 refusal intact.
- [ ] 21.2 (U5b; [SC1](specs/slice-two-contracts/spec.md), [MB4](specs/mcp-capability-broker/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 21.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 22. U5c — Extract version-aware realm grants

Dependencies: none; independent objective. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 22.1 (U5c; [SC2](specs/slice-two-contracts/spec.md)) Extract current grant parsing/serialization and reserved-key selection, retaining its v6 behavior and any landed v7 fields. Typed errors keep old text. Verify: Legacy realm round trips, empty/omitted lists and restriction identity remain exact.
- [ ] 22.2 (U5c; [SC2](specs/slice-two-contracts/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 22.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 23. U5d — Mint the retention-veto realm version

Dependencies: U5c. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 23.1 (U5d; [SC2](specs/slice-two-contracts/spec.md), [CR1](specs/capability-response-retention/spec.md)) N denotes the next realms version after v7, allocated against main at this PR; preserve #487's v7 fields. Decode retain false as Veto only in that new version; prior spellings stay restrictions. Verify: New/old round trips, bad veto values and provisional-office compatibility; frozen pins do not move.
- [ ] 23.2 (U5d; [SC2](specs/slice-two-contracts/spec.md), [CR1](specs/capability-response-retention/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 23.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 24. U5e — Bind reservation and effective retention

Dependencies: U5b, U5d. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 24.1 (U5e; [SC2](specs/slice-two-contracts/spec.md), [CR1](specs/capability-response-retention/spec.md), [MB2](specs/mcp-capability-broker/spec.md)) Make dialect restriction-reservation checks use the grant's version, carry inherit/veto into the typed holding and preserve D11. Bound identifiers and egress minimum without granting secret clearance. Verify: Four retention outcomes, legacy retain as restriction, reserved collisions through refs/composition and unchanged inactive grants.
- [ ] 24.2 (U5e; [SC2](specs/slice-two-contracts/spec.md), [CR1](specs/capability-response-retention/spec.md), [MB2](specs/mcp-capability-broker/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 24.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 25. U5f — Publish manifest v12 with its live native consumer

Dependencies: U5e. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 25.1 (U5f; [SC3](specs/slice-two-contracts/spec.md), [CR1](specs/capability-response-retention/spec.md)) Extract manifest projections and emit v12 native implementation/retention plus the typed MCP shape while MCP still refuses compilation. Keep every consulted and inactive grant identity fact. Verify: Real native compiles validate v12; internal MCP projection is typed; independent identity changes and old manifest reads are exact.
- [ ] 25.2 (U5f; [SC3](specs/slice-two-contracts/spec.md), [CR1](specs/capability-response-retention/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 25.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 26. U6a — Share the one secret injector

Dependencies: none; independent objective. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 26.1 (U6a; [MB4](specs/mcp-capability-broker/spec.md)) Move bind_environment to the named shared module and keep the single expose_for_spawn production call there; existing harness spawns consume it immediately. New errors are typed. Verify: Existing secret machine proof is updated to the one new location, not weakened; no second accessor or environment fallback.
- [ ] 26.2 (U6a; [MB4](specs/mcp-capability-broker/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 26.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 27. U6b — Introduce the broker command as a closed handler

Dependencies: U5f, U6a. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 27.1 (U6b; [MB3](specs/mcp-capability-broker/spec.md), [MB4](specs/mcp-capability-broker/spec.md)) Add Cmd plus handler for broker serve with bounded engine-plan locator/digest arguments. An unbound manual invocation refuses; no raw server argv, grants or secret values are CLI options. Move dispatch code out of the oversized CLI parent. Verify: Exact CLI parsing, bound-plan refusal and unchanged commands; compile still refuses MCP.
- [ ] 27.2 (U6b; [MB3](specs/mcp-capability-broker/spec.md), [MB4](specs/mcp-capability-broker/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 27.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 28. U6c — Define and consume the bound plan

Dependencies: U6b, U1a. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 28.1 (U6c; [MB3](specs/mcp-capability-broker/spec.md), [MB4](specs/mcp-capability-broker/spec.md), [MB5](specs/mcp-capability-broker/spec.md)) Define the shared typed BrokerPlan at the private protocol edge and implement owner-rooted reads and direct child spawn in session.rs. Only engine inventory-bound plans pass, with sanitized base environment and shared injector. Verify: Fake child version/init, egress clearance, missing/denied/store-only names and no-argv leaks; process group is inherited.
- [ ] 28.2 (U6c; [MB3](specs/mcp-capability-broker/spec.md), [MB4](specs/mcp-capability-broker/spec.md), [MB5](specs/mcp-capability-broker/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 28.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 29. U6d — Establish durable ledger records before calls

Dependencies: U6c. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 29.1 (U6d; [CR3](specs/capability-response-retention/spec.md), [CC2](specs/capability-call-checkpoints/spec.md)) Define the shared closed ledger vocabulary and bounded writer; session consumes it for Opened/Closed lifecycle. This moves minimal ledger durability earlier than U8 because MB3 cannot forward an unrecorded call. Verify: Owner binding, exclusive open, durable opened/closed and write/sync failure controls; no journal writer.
- [ ] 29.2 (U6d; [CR3](specs/capability-response-retention/spec.md), [CC2](specs/capability-call-checkpoints/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 29.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 30. U6e — Serve the filtered protocol

Dependencies: U6d. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 30.1 (U6e; [MB3](specs/mcp-capability-broker/spec.md), [CC2](specs/capability-call-checkpoints/spec.md), [CR3](specs/capability-response-retention/spec.md)) Implement MB3's protocol subset and bounds, complete filtered catalog and per-call Started/Terminal logging through the shared ledger. No unsafe passthrough, shell injection or background daemon. Verify: Fake-server allow/deny/pagination/version/method/cancellation/limit matrix; record failure precedes external action.
- [ ] 30.2 (U6e; [MB3](specs/mcp-capability-broker/spec.md), [CC2](specs/capability-call-checkpoints/spec.md), [CR3](specs/capability-response-retention/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 30.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 31. U6f — Mask and canonicalize all output

Dependencies: U6e. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 31.1 (U6f; [MB4](specs/mcp-capability-broker/spec.md), [CR2](specs/capability-response-retention/spec.md)) Factor bounded response/stderr masking and canonical serialization, including textual keys and common encodings. Retained staging remains refused until U8 storage is wired; no retained response can be forwarded without its durable evidence. Verify: Literal/encoded/split-chunk leak scan across every output, invalid-after-masking protocol identities and exact masked data bytes.
- [ ] 31.2 (U6f; [MB4](specs/mcp-capability-broker/spec.md), [CR2](specs/capability-response-retention/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 31.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 32. U6g — Prove attempt cleanup

Dependencies: U6f. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 32.1 (U6g; [MB5](specs/mcp-capability-broker/spec.md), [SD4](specs/slice-two-delivery/spec.md)) Exercise existing #403 process ownership with fake servers that block, fail init and die mid-call. Make no production tree rewrite. Verify: Linux real-process positive and cancellation/timeout tests; report existing subreaper/cgroup residuals without claiming them fixed.
- [ ] 32.2 (U6g; [MB5](specs/mcp-capability-broker/spec.md), [SD4](specs/slice-two-delivery/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 32.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 33. U7a — Represent the complete server set

Dependencies: U1g, U5f, U6g. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 33.1 (U7a; [MB1](specs/mcp-capability-broker/spec.md), [SI2](specs/strict-mcp-isolation/spec.md)) Move hands config helpers into the common typed MCP builder and replace singleton transport intent with exact named server intent. Parse final config independently; preserve authored provenance and native OFF. Verify: Zero/hands/three-server exact positives and independent missing/extra/changed/counterfeit negatives.
- [ ] 33.2 (U7a; [MB1](specs/mcp-capability-broker/spec.md), [SI2](specs/strict-mcp-isolation/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 33.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 34. U7b — Consume adapter carriage and selected holdings

Dependencies: U7a. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 34.1 (U7b; [MB1](specs/mcp-capability-broker/spec.md), [MB2](specs/mcp-capability-broker/spec.md), [SI2](specs/strict-mcp-isolation/spec.md)) Read the formerly dead mcp support at each selected candidate and materialize the engine-owned empty/hands/broker contributions from typed holdings; unsupported carriage keeps requires/wants semantics. Verify: No union of fallbacks, dead server maps give no authority, unrepresentable cap names refuse.
- [ ] 34.2 (U7b; [MB1](specs/mcp-capability-broker/spec.md), [MB2](specs/mcp-capability-broker/spec.md), [SI2](specs/strict-mcp-isolation/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 34.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 35. U7c — Provision protected per-attempt plans

Dependencies: U7b. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 35.1 (U7c; [MB1](specs/mcp-capability-broker/spec.md), [MB3](specs/mcp-capability-broker/spec.md), [CR3](specs/capability-response-retention/spec.md)) Extract broker provisioning into engine/broker.rs. Write durable inventory and per-capability plan/ledger roots outside seat-writable reach, then seal their digest/owner in selected spawn inputs. Verify: Plan substitution, missing inventory, pre-start failure and every site/attempt identity are exact; no live compile escape.
- [ ] 35.2 (U7c; [MB1](specs/mcp-capability-broker/spec.md), [MB3](specs/mcp-capability-broker/spec.md), [CR3](specs/capability-response-retention/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 35.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 36. U7d — Deliver checked multi-server configurations

Dependencies: U7c. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 36.1 (U7d; [MB1](specs/mcp-capability-broker/spec.md), [SI2](specs/strict-mcp-isolation/spec.md)) Wire the expanded server set and dialect-secret environment removals into each U0-supported builder and final consumption point; unsupported measured carriers refuse. No modifications after checked command creation. Verify: Actual composed cold/resume/replacement commands match independent literal server intent, and mutations are refused at the final serving boundary.
- [ ] 36.2 (U7d; [MB1](specs/mcp-capability-broker/spec.md), [SI2](specs/strict-mcp-isolation/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 36.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 37. U7e — Render selected capability discovery

Dependencies: U7d. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 37.1 (U7e; [SD3](specs/slice-two-delivery/spec.md), [CC2](specs/capability-call-checkpoints/spec.md)) Reuse the measured discovery identifier from adapter hands.notice, since eligible MCP sites already have hands; render fixed capability/tool guidance per selected attempt outside the requested digest. Verify: Codex deferred tool notice, fallback clearing, optional drop and hostile prose tests; hands notice stays byte-coherent.
- [ ] 37.2 (U7e; [SD3](specs/slice-two-delivery/spec.md), [CC2](specs/capability-call-checkpoints/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 37.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 38. U8a — Protect artifact paths in workspace hands

Dependencies: U7e. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 38.1 (U8a; [CR2](specs/capability-response-retention/spec.md), [MB3](specs/mcp-capability-broker/spec.md)) Carry engine-only protected storage facts, overlay the artifacts directory read-only after writable worktree binds, and reject every conflicting writable/overlay alias by canonical owner identity. No new authored HandsSpec key. Verify: Real namespace writes/ancestor renames/aliases/overlap controls cannot change evidence; mount failure refuses, never omits the protection.
- [ ] 38.2 (U8a; [CR2](specs/capability-response-retention/spec.md), [MB3](specs/mcp-capability-broker/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 38.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 39. U8b — Stage retained responses before delivery

Dependencies: U8a. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 39.1 (U8b; [CR1](specs/capability-response-retention/spec.md), [CR2](specs/capability-response-retention/spec.md), [CR3](specs/capability-response-retention/spec.md)) Stage only opted-in masked canonical bytes in the protected attempt root, fsync/rename before durable terminal metadata and delivery; metadata-only veto writes no body. Verify: Four retention outcomes and failure injection at stage/sync/terminal/delivery; error bodies retained, local failures have no digest.
- [ ] 39.2 (U8b; [CR1](specs/capability-response-retention/spec.md), [CR2](specs/capability-response-retention/spec.md), [CR3](specs/capability-response-retention/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 39.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 40. U8c — Publish verified content-addressed artifacts

Dependencies: U8b. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 40.1 (U8c; [CR2](specs/capability-response-retention/spec.md), [CR5](specs/capability-response-retention/spec.md)) Use owner-rooted no-follow reads, verify staged bytes and atomically publish immutable digest paths under the operated repository; retain handles across checks. Verify: Existing-good reuse, collision/mismatch, nonregular/symlink/hardlink and concurrent replacement controls, exact missing/corrupt causes.
- [ ] 40.2 (U8c; [CR2](specs/capability-response-retention/spec.md), [CR5](specs/capability-response-retention/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 40.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 41. U8d — Fold ledger stages through the append fence

Dependencies: U8c, U4g. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 41.1 (U8d; [CC2](specs/capability-call-checkpoints/spec.md), [CR3](specs/capability-response-retention/spec.md), [CR4](specs/capability-response-retention/spec.md)) Consume shared typed ledger records, compare sealed ownership/tools, publish before digest checkpoints and deduplicate stages from committed journal facts. Keep capability lifecycle rows out of lossy progress coalescing. Verify: Started/terminal/refused/failed/interrupted, duplicate/conflict/gap/partial/unexpected/missing cases and one-use view counts.
- [ ] 41.2 (U8d; [CC2](specs/capability-call-checkpoints/spec.md), [CR3](specs/capability-response-retention/spec.md), [CR4](specs/capability-response-retention/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 41.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 42. U8e — Settle and recover every attempt's evidence

Dependencies: U8d. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 42.1 (U8e; [CR3](specs/capability-response-retention/spec.md), [CR4](specs/capability-response-retention/spec.md), [MB5](specs/mcp-capability-broker/spec.md)) Fold after owned processes settle and before any successful ordinary/panel/step result is admitted, including stop/timeout/engine restart paths. Failed folds use existing failed/indeterminate transitions. Verify: Crash at each durability window, panel/sequence/fallback ownership, no automatic child replay, restart idempotence from journal.
- [ ] 42.2 (U8e; [CR3](specs/capability-response-retention/spec.md), [CR4](specs/capability-response-retention/spec.md), [MB5](specs/mcp-capability-broker/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 42.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 43. U8f — Expose retained evidence in the pure view

Dependencies: U8e. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 43.1 (U8f; [CC2](specs/capability-call-checkpoints/spec.md), [CR5](specs/capability-response-retention/spec.md)) Extend the already consumed call projection with digest/provenance and terminal selection, without reading paths or deriving grants. Verify: Unrecorded/veto-unknown/recorded distinctions, duplicate stages and shared display data values.
- [ ] 43.2 (U8f; [CC2](specs/capability-call-checkpoints/spec.md), [CR5](specs/capability-response-retention/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 43.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 44. U8g — Open a cited artifact through inspect

Dependencies: U8f. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 44.1 (U8g; [CR5](specs/capability-response-retention/spec.md)) Add inspect --capability-call <call_id>; read the selected run's derived call, use shared runtime artifact reader and emit verified bytes/provenance. Register the handler as a submodule of readouts.rs so no fourth file is needed. Verify: Complete exact bytes and missing/no-digest/corrupt/path/foreign-run refusal tests; no refetch and no view I/O.
- [ ] 44.2 (U8g; [CR5](specs/capability-response-retention/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 44.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 45. U9a — Prepare whole-plan MCP doctor reporting

Dependencies: U8g, U2, U3c. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 45.1 (U9a; [SC5](specs/slice-two-contracts/spec.md), [SD3](specs/slice-two-delivery/spec.md)) Extract capability reporting into the named module and use the shared complete planner for native/MCP metadata; preserve pre-U9 public compile refusal until U9b. Verify: Grant versus holding and retention facts with static scope, native denial and exact no-spawn assertions.
- [ ] 45.2 (U9a; [SC5](specs/slice-two-contracts/spec.md), [SD3](specs/slice-two-delivery/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 45.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 46. U9b — Enable the proved namespace path

Dependencies: U9a. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 46.1 (U9b; [MB2](specs/mcp-capability-broker/spec.md), [SD3](specs/slice-two-delivery/spec.md)) Lift only SD3's realm-wide unbuilt-kind fence; activate namespace/hands/network, class, strictness, carriage, connection, secret-clearance and D11 rules in D3 order. Every prior unit must be accepted and proved. Verify: Real compile/launch through engine, deterministic harness fixture, actual broker and fake dialect; independent denial, secret, retention, tamper, cold/fallback/member/step controls. No hand-built holding substitutes.
- [ ] 46.2 (U9b; [MB2](specs/mcp-capability-broker/spec.md), [SD3](specs/slice-two-delivery/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 46.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 47. U9c — Publish implemented scope and contract guidance

Dependencies: U9b. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 47.1 (U9c; [SD3](specs/slice-two-delivery/spec.md), [SC1](specs/slice-two-contracts/spec.md), [SC2](specs/slice-two-contracts/spec.md), [SC3](specs/slice-two-contracts/spec.md), [SC4](specs/slice-two-contracts/spec.md)) Describe operator grant/veto migration, strictness refusals, secret-store-only binding, namespace/stdio/empty restrictions and checkpoint/artifact inspection with measured limitations. Verify: Examples agree with real compile/report evidence; no MCP server or realm grant is shipped.
- [ ] 47.2 (U9c; [SD3](specs/slice-two-delivery/spec.md), [SC1](specs/slice-two-contracts/spec.md), [SC2](specs/slice-two-contracts/spec.md), [SC3](specs/slice-two-contracts/spec.md), [SC4](specs/slice-two-contracts/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) Review the listed documents/evidence against the named requirements and retain explicit unmeasured/pending outcomes. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 48. U10a — Audit every removal proof and scope

Dependencies: U9c. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 48.1 (U10a; [SD4](specs/slice-two-delivery/spec.md), [SD2](specs/slice-two-delivery/spec.md)) Audit requirement/task/test mapping, each compiling removal/restored pass, no new suppression/clone/unused API and frozen bytes against each unit's main. Repair a missing proof in its assigned suite, not by declaring it proved. Verify: All 26 requirement IDs covered; typed refusal text pins once per module; no frozen fixture regeneration or unrecorded windows.
- [ ] 48.2 (U10a; [SD4](specs/slice-two-delivery/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 48.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## 49. U10b — Run final candidate gates and hand off

Dependencies: U10a. Files and scope: [design.md](design.md#slice-two-units).

- [ ] 49.1 (U10b; [SD4](specs/slice-two-delivery/spec.md), [SD1](specs/slice-two-delivery/spec.md)) Run the complete D10 validation set on the restored final candidate, obtain external exact coverage and Linux/macOS/remote results naming its head; keep missing results pending. Verify: Literal nonzero covered/total equality for lines/branches/functions, pinned compiler agreement, self/verify compiles, measured identities and signed merge-queue delivery; never push from a seat.
- [ ] 49.2 (U10b; [SD4](specs/slice-two-delivery/spec.md), [SD1](specs/slice-two-delivery/spec.md), [SD2](specs/slice-two-delivery/spec.md), [SD4](specs/slice-two-delivery/spec.md)) In the owning suites listed for this row, record independent exact assertions, compiling removal failures and restored passes for the scenarios in task 49.1. Verify the row's production inventory and size/clone/consumer constraints, measure affected pins, run applicable gates, and record their tested head. Do not close unavailable external checks or remove the MCP fence before U9b.

## Requirement coverage audit

This table maps every delta requirement to its owning groups; a shared proof
never erases the individual unit ownership above.

| Requirement | Task groups / PR rows |
| --- | --- |
| [CC1](specs/capability-call-checkpoints/spec.md) | 15 (U4c), 16 (U4d), 17 (U4e), 18 (U4f) |
| [CC2](specs/capability-call-checkpoints/spec.md) | 15 (U4c), 18 (U4f), 19 (U4g), 29 (U6d), 30 (U6e), 37 (U7e), 41 (U8d), 43 (U8f) |
| [CC3](specs/capability-call-checkpoints/spec.md) | 13 (U4a), 14 (U4b), 17 (U4e), 19 (U4g) |
| [CR1](specs/capability-response-retention/spec.md) | 23 (U5d), 24 (U5e), 25 (U5f), 39 (U8b) |
| [CR2](specs/capability-response-retention/spec.md) | 31 (U6f), 38 (U8a), 39 (U8b), 40 (U8c) |
| [CR3](specs/capability-response-retention/spec.md) | 29 (U6d), 30 (U6e), 35 (U7c), 39 (U8b), 41 (U8d), 42 (U8e) |
| [CR4](specs/capability-response-retention/spec.md) | 41 (U8d), 42 (U8e) |
| [CR5](specs/capability-response-retention/spec.md) | 19 (U4g), 40 (U8c), 43 (U8f), 44 (U8g) |
| [GP1](specs/gate-capability-policy/spec.md) | 10 (U3a) |
| [GP2](specs/gate-capability-policy/spec.md) | 11 (U3b), 12 (U3c) |
| [MB1](specs/mcp-capability-broker/spec.md) | 2 (U1a), 4 (U1c), 7 (U1f), 33 (U7a), 34 (U7b), 35 (U7c), 36 (U7d) |
| [MB2](specs/mcp-capability-broker/spec.md) | 24 (U5e), 34 (U7b), 46 (U9b) |
| [MB3](specs/mcp-capability-broker/spec.md) | 27 (U6b), 28 (U6c), 30 (U6e), 35 (U7c), 38 (U8a) |
| [MB4](specs/mcp-capability-broker/spec.md) | 21 (U5b), 26 (U6a), 27 (U6b), 28 (U6c), 31 (U6f) |
| [MB5](specs/mcp-capability-broker/spec.md) | 28 (U6c), 32 (U6g), 42 (U8e) |
| [SC1](specs/slice-two-contracts/spec.md) | 20 (U5a), 21 (U5b), 47 (U9c) |
| [SC2](specs/slice-two-contracts/spec.md) | 22 (U5c), 23 (U5d), 24 (U5e), 47 (U9c) |
| [SC3](specs/slice-two-contracts/spec.md) | 25 (U5f), 47 (U9c) |
| [SC4](specs/slice-two-contracts/spec.md) | 13 (U4a), 14 (U4b), 16 (U4d), 47 (U9c) |
| [SC5](specs/slice-two-contracts/spec.md) | 9 (U2), 45 (U9a) |
| [SD1](specs/slice-two-delivery/spec.md) | 1 (U0), 49 (U10b) |
| [SD2](specs/slice-two-delivery/spec.md) | 2 (U1a), 8 (U1g), 9 (U2), 20 (U5a), 48 (U10a) |
| [SD3](specs/slice-two-delivery/spec.md) | 37 (U7e), 45 (U9a), 46 (U9b), 47 (U9c) |
| [SD4](specs/slice-two-delivery/spec.md) | 32 (U6g), 48 (U10a), 49 (U10b) |
| [SI1](specs/strict-mcp-isolation/spec.md) | 1 (U0), 3 (U1b), 5 (U1d), 6 (U1e) |
| [SI2](specs/strict-mcp-isolation/spec.md) | 2 (U1a), 3 (U1b), 4 (U1c), 5 (U1d), 6 (U1e), 7 (U1f), 8 (U1g), 33 (U7a), 34 (U7b), 36 (U7d) |

SD1 additionally owns this visit's document-order/commit/result contract.
SD2 and SD4 apply to every row through its second task, even where not repeated
in the table. No numbered task belongs to two PRs. No code mutation, U0 live
measurement, signed implementation PR, external gate or operator acceptance
is marked complete by this planning artifact.
