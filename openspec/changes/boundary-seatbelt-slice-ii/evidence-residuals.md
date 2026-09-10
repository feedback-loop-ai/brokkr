# Seatbelt acceptance evidence and open residuals

Recorded 2026-09-09 after the operator accepted the 0046 Seatbelt addendum.
This is a documentation inventory, not a mutation of the run journal or a
claim that native tests ran on the recording host (Linux). The dated audits
below track subsequently supplied macOS evidence. All four guarantees remain
OPEN; the corrected native process-group baseline is complete and records survivors.

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


## Feasibility preparation validated — 2026-09-10

Source candidate: `71522bde7d6ad012d2d4620aae4c72a9b17f7065`.
The five deltas now follow the accepted 0046 observables; design and executable
tasks exist. SEATBELT-SPEC-RECONCILIATION is resolved in the source candidate.
This documentation inventory does not mutate historical journal findings.

The operator's section A baseline is verified from the archive and all five
logs at evidence commit `9137783a0f8f31ad3c05ecbecac9d1b26f568833`: macOS
26.6.2 arm64, source `bb0bc39520f064ff7abe62ec517f52922bab6b53`, launcher
readiness and both exact refusal tests passed. The SHA-256 matches the #253
comment. Intel and all native containment guarantees remain unmeasured.

The new standalone Rust experiment and `scripts/validate-seatbelt-macos.sh
--lifetime` are ready for native measurement. The allow-default profile tests
the original-process-group candidate only; it is not an implemented boundary
or full MCP/exec acceptance suite. See the experiment README for scope and
exit codes. Its evidence format cannot authorize activation or close R3.

Local validation on the isolated source candidate:

- Portable matrix: all 12 candidate cases ran, 3 observed cleanup and 9
  recorded survivors; the separate no-cleanup negative control was detected.
  All 13 fixtures had verified cleanup after their verdict. Native=false.
- Standalone unit tests: 2 passed. Rust 1.88 metadata compilation and stable
  warning-free compilation passed. Native runner wrong-host refusal returned
  2 and retained a complete error log, exit code and non-success manifest.
- Workspace all-features locked tests: 1,290 passed across 64 result groups,
  zero failures. Formatting, clippy with warnings denied, self/verify bundle
  compilation, OpenSpec strict validation and diff whitespace checks passed.
- Exact coverage passed: 21,335/21,335 lines; 3,328/3,328 branches;
  2,025/2,025 logical functions. The standalone nonproduction experiment is
  outside the Cargo workspace; these figures cover existing production code,
  not native macOS bindings or the experiment itself.

The final workspace/coverage commands used command-local Git configuration
`commit.gpgsign=false` for disposable test commits. Coverage used `TMPDIR=/var/tmp`
to avoid RAM-backed /tmp capacity limits and temporary fixtures beneath a
repository/home. The earlier resource-constrained/inconclusive runs were not
credited. No global configuration, gate threshold or coverage exclusion changed.
The historical erratum test now bounds its assertion to that section, allowing
the accepted addendum without weakening its one-line/content checks.

[Portable measurement details](../../../docs/evidence/issue-253/portable-lifetime-2026-09-10/measurement.json)
record source/binary-independent fixture facts and verified cleanup. The native
operator must now run the pinned candidate and attach its archive to #253.
R1–R4 remain OPEN; production Seatbelt remains unavailable.


## Native process-group attempt reviewed — 2026-09-10

The operator's [updated evidence](https://github.com/feedback-loop-ai/brokkr/issues/253#issuecomment-5614518335)
is pinned at `3421ae9676feb266864c6cdbf7aae81de9ca8dc3`, directory
`docs/evidence/issue-253/native-process-group-2026-09-10-arm64/`.
Source was clean `7822fdccb044d11e8a0e3c1a6431feeb669262d6`, macOS
26.6.2 (25G83) arm64, stable Rust 1.98.1. Native archive SHA-256
`d277193203ad9de2a41775e7ad2cfe735be7513f12e4708dfdbd2369316f1904`
was verified, along with all 166 archived files against committed Git blobs.
The earlier failed-attempt archive SHA-256
`2528a9ce832fbb5593b6283080e7d5ea44e0fbbd158dd403c57324cc52349572`
and all five archived files were also verified.

The native run returned **2**: 12 attempts, 3 no-survivor observations,
7 survivor observations, and 2 missing observations (`setsid-parent-exit`,
`double-fork-parent-exit`). Negative control detected; all 13 fixture cleanup
markers present. Both missing observations timed out waiting for `action`
after the supervisor reported EPERM. This is incomplete evidence, not closure.

The supervisor returned before publishing `action` when group signalling
failed. Apple's published [XNU signal implementation](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/kern_sig.c)
excludes zombies from the group iterator and can return EPERM when no eligible
member remains. A zombie-only original group with an escaped descendant is
consistent with these logs; the exact installed kernel was not inspected.
The corrected experiment records the raw signal errno before publishing the
action marker and allows the outside observer to measure the remaining leaf.
A signal failure is a candidate residual, never a successful signal.

Two runner defects were also corrected: Homebrew rustc shadowing rustup's
proxy (use explicit `rustup run stable rustc`), and Bash 3.2's unwaitable
process-substitution PID (use a direct tee child with a FIFO). Only a completed,
valid measurement returning 1 may yield runner exit 1; a prerequisite returning
1 now yields 2. Original evidence is preserved unchanged.

Corrected-source Linux validation: 3 standalone unit tests, warning-free
compilation, full portable matrix (12 cases, 3 clean, 9 residual, negative
control detected, 13 verified fixture cleanups), shell syntax, formatting and
whitespace checks passed. A shell regression with a shadow compiler and a
rustup prerequisite returning 1 verified runner exit 2, explicit rustup selection,
complete transcript/archive capture and FIFO removal. These are harness tests;
macOS Bash 3.2 and native errno observations require the operator rerun.
Production code and its previous workspace/coverage results are unchanged.

The original-process-group experiment is separate from draft #259's launchd
lease-pair candidate. Neither this attempted run nor its runner fixes establish
R1/R2/R4, integrated MCP/exec lifetime, arbitrary descendants or activation.
R3 remains OPEN and a complete native rerun is the next evidence step.


## Corrected native process-group rerun reviewed — 2026-09-10

Reviewed [operator rerun](https://github.com/feedback-loop-ai/brokkr/issues/253#issuecomment-5614915948)
at evidence commit `030e8888b7297768df907442978ec43139306a0b`, directory
`docs/evidence/issue-253/native-process-group-8b4a390-2026-09-10-arm64/`.
Exact source: `8b4a3906d3daa033e0aa9b7f2d55697391803db1`; host log records a
clean checkout, macOS 26.6.2 (25G83) arm64 and Rust stable 1.98.1. Command:
`bash scripts/validate-seatbelt-macos.sh --lifetime`.

Archive SHA-256 verified:
`5c4c72b0ce529148d0f7b90c36084400c1746e1f3e485a18b8b783c2317ad3d0`.
All **180 committed expanded text files** match archive contents and Git blobs.
The archive contains **181 non-AppleDouble regular files**: those 180 plus the
Mach-O arm64 executable, whose recorded SHA-256 also matches. Both recorded
source hashes match the exact candidate's Git blobs. Archive metadata was not
credited as measurement evidence; no archived executable or script was run.

Original host, transcript, summary, all 13 case results, identities/groups,
signal/action records and cleanup records were inspected. Measurement and
runner exit records both report **1**. Summary: `native:true`,
`valid_measurement:true`, `cases:12`, `residual_cases:9`, `errors:[]`,
`r3_closed:false`, `seatbelt_activation_authorized:false`.

| Cases | Native observation |
| --- | --- |
| Ordinary timeout, cancellation, parent exit | 3 no-survivor observations; no heartbeat advance. |
| setsid and double-fork, all four triggers | 8 surviving leaves with advancing heartbeats. |
| Ordinary supervisor death | Root and leaf survive with advancing leaf heartbeat. |
| Separate no-cleanup negative control | Root and leaf survive; control detected. |

Both detached parent-exit cases now retain attempted group SIGKILL with
`candidate_signal_errno:1` (EPERM), followed by complete survivor observations.
All other attempted group signals record zero errno; supervisor-death and
control cases correctly record no candidate signal. Positive liveness and
requested-topology checks gate case results in the pinned source. All 13
identity/group records agree with their case results; all 13 fixtures have
post-verdict cleanup verification, with no expiry markers. Every fixture's
stdout/stderr is empty; the complete runner transcript has no previous wait
error. The available logs do not separately record the Bash version or outer
shell exit, so those are not independently attested beyond the operator report
and runner exit file.

**Verdict:** the corrected baseline/falsification matrix is complete on this
Mac. The original-process-group candidate fails R3 in nine measured cases;
fixture cooperation and self-expiry are cleanup safeguards, not containment.
The earlier incomplete attempt remains preserved above. No further rerun of
this unchanged baseline is required to establish its observed failure.

Next requirement is a separately pinned replacement lifetime mechanism with
its own native evidence, including detached/double-fork payloads, retained
pipes, hostile lifetime races and both MCP/exec paths. Draft #259's launchd
lease-pair mechanism is not evaluated by this evidence. Intel, R1/R2/R4 and
full integrated native acceptance remain unproven; R1–R4 remain OPEN and
Seatbelt activation, full peer status and story completion remain blocked.
This audit changes documentation only; validation is evidence consistency
and `git diff --check`, with no broad production suite rerun.
