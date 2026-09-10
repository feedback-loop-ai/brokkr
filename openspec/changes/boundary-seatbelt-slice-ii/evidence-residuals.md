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

## Preparation record — 2026-09-10

Recorded on the Linux controller. This is implementation and preparation
status, not native enforcement evidence. It does not modify the historical
audit above, whose inspected revision predates this record.

- **Capability reconciliation: DONE.** The five capability deltas were
  reconciled to the accepted 2026-09-09 addendum at `7e79b43` and verified
  again for this record. They require private replacement-locator overlay
  snapshots, denied-read masks and conditional full-peer hooks; the earlier
  readable-empty/same-path/unruled text is gone. `openspec validate
  boundary-seatbelt-slice-ii --strict --no-interactive` passes. This closes
  the old SEATBELT-SPEC-RECONCILIATION preparation item; it closes no
  enforcement residual.
- **Design and tasks: DONE.** `design.md` and `tasks.md` exist and encode the
  pre-implementation R3 native-proof gate (design D1–D14, tasks 1.1–1.5).
- **Bounded R3 probe: REPAIRED, NATIVE RUN PENDING.** The probe lives in
  `crates/brokkr-protocol/tests/seatbelt_lifetime_probe.rs` and its
  `seatbelt_probe` module. The interpreted payload has been replaced by one
  committed Rust test-support executable
  (`tests/seatbelt_probe/helper.rs`, the `seatbelt-probe-helper` bin) serving
  payload, descendant, guard, supervisor and startup roles. The shared model
  now implements the four-cell S0–S3 startup matrix and gates the lifetime
  matrix on a passing startup verdict; the measurement findings (real
  per-case triggers, attack-before-observation, guard liveness before
  unregister, a real original-process-group `SIGKILL`, public start
  identities, preserved failure reports, role-separated state and the
  forged-marker/harness-cleanup bans) are encoded and injected tests fail each
  forged fact. The macOS launchd lease-pair adapter is type-checked on Linux
  while its required tests run only on macOS. CI selects the startup gate
  first, uploads the durable startup and lifetime reports even on failure, and
  runs the lifetime gate only after startup succeeds.
- **Native lifetime feasibility evidence: STILL REQUIRED.** No macOS host was
  available to this controller, so SEATBELT-R3 has no adversarial
  measurement. The controller-dispatched native probe run, with the candidate
  commit, macOS version/architecture, both job labels, descendant identities,
  triggers and durable log, remains the concrete missing prerequisite.
- **Fence: unchanged.** Seatbelt stays unbuilt (slice ii) and container
  unbuilt (slice iii). The existing start refusal is untouched and no
  dependent production Seatbelt implementation was added.

## Native startup measurement — 2026-09-10

Recorded on the Linux controller from the controller-supplied native CI log.
This is a native observation of a failed startup, not a lifetime result. It
does not replace or rewrite the historical audit below.

- **Candidate:** `6a19a6f4ab9bd30b47537de1a649949cd1099d01`, native CI run
  `34433461814`, `macos-26-arm64` runner image, macOS 26.6.2 (build 25G83),
  arm64.
- **Observed:** direct `sandbox-exec -f policy.sb /usr/bin/python3 ...` ended
  on signal 6 (`SIGABRT`) with empty stdout and stderr. The equivalent
  transient launchd payload job registered, showed `runs = 1` and
  `successive crashes = 1`, and never advanced the payload heartbeat. The
  direct and launchd failures were identical for every case.
- **Conclusion:** payload startup was not established. The startup gate did
  not pass, no lifetime case ran, and none of SEATBELT-R1–R4 is answered. The
  `SIGABRT` cause is not established and is deliberately not named; the
  interpreted Python payload is recorded only as the payload that aborted.
- **Not the failure:** the already-repaired Git-metadata and `/usr/include`
  runner prerequisites; registration plus one crashed run is not quiescence.
- **Next action:** dispatch the committed four-cell S0–S3 startup matrix on
  the existing macOS CI host. This row is `SEATBELT-R3-STARTUP` failure
  evidence; it closes no residual.


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
