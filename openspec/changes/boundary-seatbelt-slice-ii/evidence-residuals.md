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
- **Bounded R3 probe: REPAIRED AGAIN, NATIVE RUN PENDING.** The probe lives in
  `crates/brokkr-protocol/tests/seatbelt_lifetime_probe.rs` and its
  `seatbelt_probe` module. The interpreted payload has been replaced by one
  committed Rust test-support executable
  (`tests/seatbelt_probe/helper.rs`, the `seatbelt-probe-helper` bin) serving
  payload, descendant, guard, supervisor and startup roles. The shared model
  implements the four-cell S0–S3 startup matrix and gates the lifetime matrix
  on a passing startup verdict; the measurement findings (real per-case
  triggers, attack-before-observation, guard liveness before unregister, a real
  original-process-group `SIGKILL`, public start identities, preserved failure
  reports, role-separated state and the forged-marker/harness-cleanup bans) are
  encoded and injected tests fail each forged fact. After native CI
  `34441725835` the adapter was repaired again: the helper's Unix ABI calls are
  target-gated so the Windows workspace test links; the two destructive native
  tests are `#[ignore]`d and selected only by the macOS step (`--ignored
  --exact`) so the generic parallel suite runs only the host-independent model;
  every launchd label and private root is unique per host instance and
  invocation, with absence proven before bootstrap and after bounded bootout;
  the private root is canonicalized after creation so the profile matches the
  resolved `/private/var` path; `launchctl print` facts are parsed or the cell
  fails, and no missing counter is synthesized into an exit code; the
  structural helper/argv comparison treats the cell root as a typed variable;
  and a bounded one-authority-at-a-time differential plus the labelled `allow
  default` control is recorded as non-passing diagnosis on an exact-profile
  failure. The macOS launchd lease-pair adapter is type-checked on Linux while
  its required tests run only on macOS. CI selects the startup gate first,
  uploads the durable startup and lifetime reports even on failure, and runs
  the lifetime gate only after startup succeeds.
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


## Native startup measurement — candidate `8c53dce` (CI `34441725835`)

Recorded on the Linux controller from the controller-supplied native CI log.
This is a native observation of a failed startup, not a lifetime result. It
does not replace or rewrite the historical audit below.

- **Candidate:** `8c53dcecaef414938b3abfb8911a77d9ec958f23`, native CI run
  `34441725835`, GitHub `macos-latest` arm64 runner. The supplied log does not
  contain `sw_vers` output, so no macOS version or build is recorded here; only
  the runner label and architecture are facts.
- **S0 direct unboxed — PASS:** the committed Rust helper digest
  `e925083d55a9c8b0` reached nonce-authenticated `READY`, identified an
  ordinary child and exited `0`.
- **S1 direct exact profile — FAIL:** the identical helper and argv aborted
  with signal 6 (`SIGABRT`) before `READY`, with empty stdout and stderr.
- **`allow default` diagnostic — NON-PASSING:** the same helper and argv under
  a labelled broad diagnostic exited `0`. This localizes the refusal to
  authority withheld by the exact profile; it identifies no missing authority
  and authorizes no allowance. It cannot satisfy a cell.
- **S2/S3 launchd cells — NO COHERENT CONTROL:** across the two Gate A
  executions the cells alternated. Once S2 reached `READY` and an ordinary
  child but reported `runs=None`, `crashes=None` and a synthesized `exit 1`;
  once S2 failed `Bootstrap failed: 5: Input/output error`. S3 likewise
  alternated between bootstrap error 5 and a non-ready `exit 1` with no
  run/crash facts. Registration, a run count or a missing field is not
  startup evidence.
- **Gate B lifetime — NOT RUN.** Startup did not pass, so no lifetime,
  survivor or quiescence fact exists. SEATBELT-R1–R4 remain open.
- **Windows — link FAIL:** the workspace test binary failed to link with
  unresolved external symbols `getuid` and `getpgid` from the helper. That is
  a portability defect, not native security evidence.
- **Measurement defects isolated:** (1) both native tests ran in parallel in
  one process and generated the *same* launchd labels, so one test could
  bootstrap or bootout the other's job and produce order-dependent
  `Bootstrap failed: 5`; (2) a missing `launchctl print` counter was
  converted into a synthesized `exit 1`; (3) the probe root failed to
  canonicalize because it did not exist yet, so the profile granted the
  `.../var/...` spelling while the kernel resolved `.../private/var/...`.
- **Not the failure:** the already-repaired Git-metadata and `/usr/include`
  runner prerequisites; registration plus one crashed run is not quiescence;
  the `SIGABRT` cause is not established.
- **Next action:** dispatch the repaired exact-head probe. The destructive
  adapter is now explicitly selected, labels/roots are never reused, the root
  is canonicalized, launchd facts are parsed or the cell fails without a
  synthesized exit, and a bounded one-authority-at-a-time diagnostic records
  which named allowance (if any) restores `READY`. This row is
  `SEATBELT-R3-STARTUP` failure evidence; it closes no residual.


## Native startup measurement — candidate `9f4c2c9` (CI `34449331270`)

Recorded on the Linux controller from the controller-supplied native CI log.
This is a native observation of a failed startup, not a lifetime result. It
does not replace or rewrite the historical audit below or the earlier rows.

- **Candidate:** `9f4c2c944cac217ccb8dc055971cc62614313ed4`, native CI run
  `34449331270`, GitHub `macos-latest` arm64. The generic workspace job passed
  on macOS, Linux and Windows, and Gate A failed. Windows no longer shows the
  unresolved `getuid`/`getpgid` link.
- **S0 direct unboxed — PASS:** the helper reached the exact bounded stages
  `["entry", "payload-dir", "executable", "child", "ready", "return-clean"]`,
  spawned and identified an ordinary child and exited `0`.
- **S1 direct exact profile — FAIL:** the identical helper and argv aborted with
  signal 6 before the first stage. All seven one-authority-at-a-time
  differentials (resolved temp read, global read-metadata, `/Users`+`/opt`
  read, `mach-lookup`, `network*`, `system-socket`, `iokit-open`) also aborted
  before the first stage. The labelled `allow default` control reached `READY`,
  but it is diagnostic only and cannot authorize a profile.
- **S2 launchd unboxed — OBSERVATION REFUSAL:** the job was bootstrapped, then
  the cell failed with "never produced a parseable not-running state". No raw
  `launchctl print` sample was preserved, so this is an observation defect, not
  a payload-execution result; it is not treated as either a pass or a proof
  that the payload did not run.
- **S3 launchd exact profile — FAIL:** parsed `state = "not running"` with
  `runs = 1`, `successive crashes = 1`; no `READY` or stage. Its profile digest
  differed from S1 only because each cell's profile embeds its own private root,
  which the model wrongly compared as authority drift.
- **Denial controls — UNOBSERVED:** credential-read, host-write and network-bind
  were not observed and denied because the payload never started, so no
  boundary proof exists.
- **Gate B lifetime — NOT RUN.** Startup did not pass; SEATBELT-R1–R4 remain
  open. No non-starting payload was counted as an enforcement result.
- **Next action:** the repaired probe in tasks 1.13–1.17. This row is
  `SEATBELT-R3-STARTUP` failure evidence; it closes no residual.

## Named startup cause and probe repair — 2026-09-10

Recorded on the Linux controller as preparation, not native evidence. The
native rerun on the exact repaired head remains pending.

- **Named cause of the pre-stage `SIGABRT`.** The exact candidate profile
  granted `file-read*` on `/usr`, `/bin`, `/sbin`, `/System`, `/Library`, the
  temp areas, the case root and specific `/dev` literals, but never the
  filesystem-root inode `/`. macOS `dyld` reads that inode while initialising a
  dynamically linked process; Seatbelt denies the read and fails closed with
  `SIGABRT` before the payload can record its first stage. The same defect is
  documented publicly for other Seatbelt profiles — a profile missing
  `(allow file-read* (literal "/"))` aborts a dynamically linked binary at
  `dyld` init, while the broad `allow default` control starts it (the
  `allow default` control in CI `34449331270` is exactly that contrast; public
  corroboration: `astrid-runtime` commit `6ba24cf`, "stop silently disabling the
  macOS sandbox on macOS 15+"). The seven one-class diagnostics did not restore
  startup because none of them supplied a `file-read-data` grant on the root
  inode: `file-read-metadata` covers only metadata, not the root directory
  read.
- **Repair, minimal aperture.** The candidate profile now carries
  `(allow file-read* (literal "/"))`. `(literal "/")` grants a read of the root
  inode only and never the recursive `(subpath "/")` access; the profile still
  denies `default`, still grants no `mach-lookup`, and still writes only the
  payload directory.
- **Removal proof, not an assumption.** `STARTUP_NEGATIVE_ALLOWANCES` replays
  the exact candidate with that one rule stripped and requires the same payload
  to fail closed. A passing Seatbelt startup cell now must show the removal
  observed and blocking; a stripped profile that still starts fails the cell.
  This is what makes the rule load-bearing rather than a widening that rides
  along, and it keeps the broad `allow default` control non-admitting.
- **Lossless launchd observation.** Every launchd startup cell now retains a
  bounded sequence of distinct raw `launchctl print` samples with exit status,
  stdout and stderr, reports the last parseable state when no terminal state is
  reached, and includes the last raw sample in the refusal. A reaped label is
  reported as a reaped label; a missing field is never synthesized into an exit
  code.
- **Structured profile identity.** The cross-cell profile digest now replaces
  the typed private cell root with the structural `ROOT_TOKEN` before hashing,
  so isolated cells compare their policy rather than their private path, while
  a real rule difference still changes the digest.
- **Fence: unchanged.** Seatbelt stays unbuilt (slice ii) and container
  unbuilt (slice iii). This section records a diagnosis and a bounded probe
  repair; it does not claim native enforcement, and Gate B remains not run.


## Native startup measurement — candidate `fa7ece5` (CI `34457208029`)

Recorded on the Linux controller from the controller-supplied native CI log
(`controller-native-fa7-findings.json`, `controller-native-fa7-report.txt`,
`controller-native-fa7-startup.log`). This is a native observation of a
failed startup with measured progress, not a lifetime result. It does not
replace or rewrite the historical audit below or the earlier rows.

- **Candidate:** `fa7ece587178a46baa66a7310e0546bfb87a0857`, native CI run
  `34457208029`, GitHub `macos-latest` arm64 runner. As with `8c53dce`, the
  supplied log carries no `sw_vers` output, so no macOS version or build is
  recorded here; only the runner label and architecture are facts.
- **S0 direct unboxed — PASS:** the helper (digest `10d37a1aa412dc7a`) reached
  the exact bounded stages `["entry", "payload-dir", "executable", "child",
  "ready", "return-clean"]`, identified an ordinary child and exited `0`.
- **S1 direct exact profile — FAIL, past the prior pre-stage abort:** with the
  root-inode read granted, the identical helper and argv (profile digest
  `8deef6241f098ca4`) now reached `entry`, `payload-dir` and `executable`,
  then the ordinary-child spawn failed with `EPERM` (`os error 1`) and the
  helper exited `2` without `READY` or an identified child. Every one-class
  diagnostic (`tmp-realpath-read`, `ancestor-read-metadata`,
  `helper-parent-read`, `mach-lookup`, `network`, `system-socket`,
  `iokit-open`) failed identically at the same child-spawn point; none of
  them granted any `file-write*`. The labelled `allow-default` diagnostic
  reached `READY` and exited `0`; it authorizes nothing and identifies no
  operation or target.
- **Denial controls — OBSERVED DENIED on this candidate:** credential-read
  (`/etc/passwd`, `EACCES`/`EPERM`), host-write
  (`/private/tmp/brokkr-probe-denial-write`, `EPERM`) and network-bind
  (loopback `127.0.0.1:0`, `EPERM`) were each observed and denied. These
  describe only this failing candidate and close none of SEATBELT-R1–R4.
- **S2 launchd unboxed — READY, ordinary child observed:** the launchd-owned
  job reached the same stages, `READY` and an identified ordinary child. The
  raw `launchctl print` sample shows `state = not running`, `runs = 1`,
  `last exit code = 0`, with no `successive crashes` line. The committed
  parser rejected the whole print on that absent field and the report erased
  the printed facts instead of keeping them as independently parsed and
  unknown; that is a measurement defect in the parser, not a launchd verdict,
  and task 1.26 owns its repair.
- **S3 launchd exact profile — FAIL, matches S1:** the job reached `entry`,
  `payload-dir` and `executable`, then the same `EPERM` child-spawn failure
  as S1, exiting `2` without `READY`. The raw print shows `runs = 1`,
  `last exit code = 2`, again with no `successive crashes` line and the same
  parser defect.
- **Gate B lifetime — NOT RUN.** No Seatbelt cell reached `READY`, so no
  removal control was due, the root-inode read is not yet proven
  load-bearing by removal on this candidate, and SEATBELT-R1–R4 remain open.
- **Not the failure:** the already-repaired Git-metadata and `/usr/include`
  runner prerequisites; the S0 pass and the S2 launchd `READY` are not
  Seatbelt enforcement evidence; the `EPERM` operation and target are not yet
  named.
- **Next action:** the audited rule ledger repair in tasks 1.18–1.27 —
  `HOST_TOOLCHAIN_BINDS`, the typed ledger and pure check, the repaired
  observer, the child-spawn sub-stages and four discriminating replay cells
  that localize this `EPERM`, the withdrawal/narrowing restoration
  diagnostics, the bounded `log show` denial events and the top-level
  launchd brace-depth scanner — followed by a controller-dispatched Gate A
  run on that exact head (task 1.28). This row is `SEATBELT-R3-STARTUP`
  failure evidence with measured progress; it closes no residual and no fa7
  stage progress, denial result or removal verdict carries over to the
  ledger candidate.

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

### SEATBELT-R3-STARTUP, fourth measurement and the named cause (CI 34694562248, head `0692477`)

The fourth native run finally discriminated the pre-stage abort. On the macOS
runner with `/usr/bin/sandbox-exec` present, S0 and S2 (both unboxed) reached
a nonce-authenticated `READY` and every stage. S1 and S3 (both under the
candidate profile) aborted at `stages=[]` with signal 6. The staged helper
digest equalled the committed build digest (`814ac8535776fff9`) in all four
cells, so instantiation, staging and identity were never the fault.

The payload's own stderr names the operation:

```
failed to allocate a guard page: Invalid argument (os error 22)
fatal runtime error: initialization or cleanup bug, aborting
```

The bounded `log show` collection recorded the denial sequence that precedes
it, and the last event before the abort is `deny(1) sysctl-read
hw.pagesize_compat`. Read with the differentials, the cause is settled:

| Cell or differential | Added authority | Result |
| --- | --- | --- |
| `allow-default` | every operation | reached `READY`, all seven stages |
| `restore-all-fa7` | the withdrawn and narrowed fa7 units | `entry`, `payload-dir`, `executable` |
| `tmp-realpath-read`, `ancestor-read-metadata`, `helper-parent-read` | file classes | aborted at `stages=[]` |
| `mach-lookup`, `network`, `system-socket`, `iokit-open` | mach, network, socket, IOKit | aborted at `stages=[]` |

No differential in the set admitted `sysctl-read`. That was the one class the
seven one-at-a-time allowances never varied, which is why three earlier runs
recorded the abort without naming it. The failure is `EINVAL`, not `EPERM`,
because the denial makes the runtime's page-size query return no usable value
and the stack-guard `mprotect` is then called with a length the kernel
refuses. It is not an operation the ledger was missing on a filesystem path.

**The repair (task 1.34), with minimal aperture.** The candidate admits
`(allow sysctl-read (sysctl-name "hw.pagesize_compat"))` as a
diagnosis-admitted unit: one parameter by name, never the `sysctl-read` class,
matching the rule the root-inode read already follows. Its removal control
`page-size-sysctl-read` joins the bounded removal set, so a startup cell that
reaches `READY` must also observe the stripped replay failing closed before it
may pass. The previously untested `sysctl-read` class enters the diagnostic
set as an eighth labelled non-passing allowance, so the one-class record is
complete for any future abort. Host-independent tests falsify the named
aperture and require the removal set and the diagnosis-admitted half to remain
the same set in both directions.

Earlier measurements are not rewritten. The seven one-class differentials that
still aborted before the first stage stand as recorded, and they are what
makes this reading the only one left.

### SEATBELT-R3-STARTUP, fifth measurement: the child-spawn attribution (CI 34694964853, head `20b1a78`)

With the page-size sysctl admitted, both Seatbelt cells started. S1 and S3
advanced from `stages=[]` and signal 6 to `["entry", "payload-dir",
"executable"]` and a clean `exit 2`: the payload now runs and refuses at a
named step instead of dying before its first. The guard-page reading is
therefore confirmed by construction, not only by inference.

The cell named its own failing sub-stage: `child streams: stdout /dev/null:
Operation not permitted (os error 1)`, and the denial collection recorded
`deny(1) file-write-data /dev/null` throughout. The discriminating cells the
specification requires then separated the stdio hypothesis from an exec-side
refusal, and they agree:

| Discriminating cell | Added authority | Result |
| --- | --- | --- |
| `child-probe-open` (opens the null device write-only, no spawn) | none | stopped at `executable` |
| `child-probe-null` (spawns with null stdio) | none | stopped at `executable` |
| `child-probe-inherit` (spawns with inherited stdio) | none | reached `child-observed` |
| `dev-null-write` (candidate plus exactly one literal) | `(allow file-write-data (literal "/dev/null"))` | reached `READY`, all seven stages, clean exit |

No exec-side refusal appears: nothing in the collection names a `process-exec`
or a read of the resolved helper path. The attribution is one operation,
`file-write-data`, on one target, the literal `/dev/null`, which is exactly
the shape `A named child-spawn predicate is literal-scoped and removable`
requires. The eighth diagnostic, the previously untested `sysctl-read` class,
also ran and stopped at `executable`, confirming it carries nothing further.

**The repair (task 1.35).** `(allow file-write-data (literal "/dev/null"))`
enters as the child-spawn attribution's own diagnosis-admitted unit with the
removal control `child-spawn-dev-null-write`. No `/dev` subpath, no
unfiltered `file-write*`, and no wider process, Mach, IPC, service or network
grant enters with it. The device-set baseline still reads and never writes:
the attribution owns this write, as spec.md:907-909 already provided. A cell
that reaches `READY` now owes three observed removals rather than one.

Host-independent tests were retargeted rather than deleted. The test that
required the candidate to carry no null-device write recorded the state before
any attribution existed; it now requires the write to be present, to be
diagnosis-admitted and never a device-set baseline, and it keeps its proof
that the same unit recorded as a device-set baseline fails the operation
anchor. The removal-entry law is now proved against the committed entry by
dropping its removal control, instead of against a duplicate pushed onto the
ledger.

### SEATBELT-R3-STARTUP closes: Gate A passes (CI 34695285134, head `c3daedf`)

**Gate A startup verdict: PASS.** All four cells — both unboxed controls and
both Seatbelt cells — reached a nonce-authenticated `READY` through the exact
required stage sequence with a clean exit, on the same staged helper digest
`814ac8535776fff9`.

The pass is proved, not asserted. Every removal control was observed blocking
in both Seatbelt cells, six observations across the three admitted rules:
strip the root-inode read, the page-size sysctl or the child's null-device
write and the identical payload fails closed. No admitted rule rides along.
The denial controls held in the same run: `credential-read`,
`data-volume-credential-read` and `host-write` were each observed and denied,
so the boundary refuses what it must while admitting what the payload needs.
The bounded `log show` collection for the passing cells is empty: no Sandbox
denial event names the helper at all.

The candidate's authority remains narrow. Three diagnosis-admitted units, each
one operation on one named target, each with its own removal control; no
`/dev` subpath, no unfiltered `file-write*`, no blanket `sysctl-read`, no
wider process, Mach, IPC, service or network grant, and every device-set
baseline unit still a read. `SEATBELT-R3-STARTUP` is closed.

### SEATBELT-R3-LIFETIME, first measurement (same run)

Gate A passing let Gate B execute for the first time. It fails, as the plan
expects: tasks 1.29 through 1.33 own the lease-pair lifetime model and all
five are unticked.

Four cells pass: `ordinary-child`, `guard-interference`, `peer-bootout` and
`escape-job` each end with zero survivors, the transient labels gone and the
guard alive. An authenticated attacker booted out neither the guard nor a
peer, and registered no escape job.

Four cells fail identically, which makes them one defect: `cancellation`,
`supervisor-death`, `ignored-signals` and `retained-pipes` each record the
lifecycle `[Prepared, GuardRegistered, PayloadStarted, Terminating]` where
`[... Terminating, PayloadQuiescent, PrivateStateRemoved, GuardUnregistered]`
is required. One payload descendant outlives the teardown bound, its heartbeat
moves during the one-second quiet window, the transient label survives, and
private state is removed before quiescence rather than after. The ordering
inversion is the first thing to repair, because removing state under a live
payload is what makes the other three unobservable.

Four cells never armed: `timeout`, `double-fork` and `parent-exit` skipped
because the payload heartbeat had not advanced before the trigger, and
`group-kill-negative-control` skipped for the helper's heartbeat. They measure
nothing yet, which is exactly what task 1.30's requirement that every B1 fact
be observational rather than assigned is for.

Recorded as issue #269. `SEATBELT-R3` stays open, Seatbelt stays unbuilt, and
the lifetime step reports rather than gates until that issue closes.
