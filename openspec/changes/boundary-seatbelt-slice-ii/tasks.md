## 1. Accepted specification and feasibility preparation

- [x] 1.1 Reconcile all five capability deltas with accepted 0046 R1–R4 observables and evidence gates.
- [x] 1.2 Record the verified #253 macOS baseline and separate it from containment evidence.
- [x] 1.3 Build a standalone Rust process-group lifetime experiment with ordinary/setsid/double-fork cases and timeout/cancel/parent-exit/supervisor-death triggers.
- [x] 1.4 Add independent observation, a mandatory negative control, bounded fixture cleanup, explicit residual results and a native evidence runner.
- [x] 1.5 Complete Linux harness verification, OpenSpec validation and repository gates; record all failures or unavailable checks.
- [ ] 1.6 Deliver a reachable candidate SHA and exact native invocation to #253 through the controller.
- [ ] 1.7 Run on the operator's Mac and retain evidence; do not mark R3 closed from a baseline or controlled experiment.

## 2. Lifetime feasibility gate

- [ ] 2.1 Evaluate native measurements and retain every survivor/control/observation failure as a named residual.
- [ ] 2.2 Identify and demonstrate an enforceable native lifetime mechanism satisfying R3 if the process-group candidate fails; otherwise return the unresolved guarantee upstream.
- [ ] 2.3 Extend proof to arbitrary detachment/fork races, retained pipes and both workspace MCP and boxed exec, including supervisor death, before enabling dependent enforcement.

## 3. Dependent full Seatbelt implementation — blocked on lifetime feasibility

- [ ] 3.1 Complete overlay locator transport/identity and alias-safe snapshot design for arbitrary and shipped binds.
- [ ] 3.2 Implement filesystem, environment, masks, network, protected inputs and Git metadata protection through both hands paths.
- [ ] 3.3 Demonstrate R1/R2/R4 with native positive controls, including primary and linked worktrees.
- [ ] 3.4 Add required macOS CI acceptance and final candidate evidence, plus Linux regression/exact coverage; mark untested architectures pending.
- [ ] 3.5 Reconcile records/readouts/guides and activate only after all required guarantees pass; use the established finding lifecycle to close residuals by evidence.
