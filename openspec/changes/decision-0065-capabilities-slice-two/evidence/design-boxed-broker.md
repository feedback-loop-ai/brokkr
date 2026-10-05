# Boxed broker design visit — 2026-10-05

Status: specify and council design drafted; dependent tasks reconciled and
document consistency audited. Phase outcomes remain the engine's; no
implementation proof or operator acceptance is claimed.

The original specify record below is historical and preserved. The council
design visit and its validation follow it in this same commissioned file.
Change: `decision-0065-capabilities-slice-two`.
Run: `amend-decision-0065-slice-two-s--a9a103e3`.
Starting HEAD: `19bca5ff` (U4b, #547); tracked worktree initially clean.

## Scope and artifact order

The current seat is specify. Read the rendered instructions and
[dialect definition](../../../../dialects/openspec.json) through workspace
hands: specify owns proposal then capability deltas; council owns design,
the tasks seat owns tasks, and analysis judges their coherence. Adopted the
existing change and wrote the proposal first, then its affected deltas.
No workflow runner was invoked, no phase was selected by this seat, and no
council position was supplied at this specify visit. Future council positions
must each be reconciled in design Decisions; this record claims no council
consensus.

Also appended the commissioned operator addendum verbatim and added the
permitted one-line pointer to accepted decision 0077 without rewriting its
status or rulings. This file owns this amendment's visit record. Historical
`evidence.md`, completed tasks, the task preamble, frozen contracts, fixtures,
policy and reference bytes remain untouched. No production, test, recipe,
release or living-spec file changed. No live harness/namespace experiment or
secret lookup was performed. The documents remain proposed implementation
detail beneath the supplied operator ruling.

## Source evidence

Read the existing proposal, complete design and tasks, all seven deltas and
operator record; README and decisions 0004/0005; decisions 0009, 0012, 0043,
0046, 0063, 0065, 0071 and 0077 as applicable. The commission and the boxed
triage framing provide the two held-run identifiers and seven HIGH findings.
The held runs are `0065-slice-two-unit-u6c-see-the--99c50c0b` and
`0065-slice-two-unit-u6c-see-the--f48203e8`. Their artifacts are not present in
this seat's reachable worktree; no journal/history capability is granted.
They were not independently opened or reproduced. Their supplied findings
are design inputs, and their code is neither adopted nor an implementation
base. U6c restarts from current main.

Source observations at the starting HEAD:

| Source | Observation and implication |
| --- | --- |
| `crates/brokkr-protocol/src/hands.rs:384` | `HOST_TOOLCHAIN_BINDS` is the existing single system-source list; reuse it in the shared builder (0071 ruling 5). |
| `crates/brokkr-protocol/src/hands.rs:420` | `box_argv` always supplies workspace/Git/declared binds, host-backed HOME/TMPDIR and `--new-session`. It cannot safely be used as-is for the server. A typed profile/extraction with an existing hands consumer is needed (rulings 2, 4, 5, 10). |
| `crates/brokkr-protocol/src/hands.rs:940` and `:1066` | Workspace calls and boxed exec consume the same builder today. Preserve these consumers and their parity proofs during extraction. |
| `crates/brokkr-protocol/src/hands/overlay.rs` | Session overlays and RAM exec overlays are already distinct. The new server needs neither the seat's overlays nor their writable sources; HOME/TMPDIR are fresh tmpfs. |
| `crates/brokkr-protocol/src/hands/session.rs` | `Session` honors host TMPDIR, owns a lock, removes scratch and has a dead-session reaper. It supplies lifetime, not protected ancestry or durable evidence. |
| `crates/brokkr-runtime/src/engine.rs:251`, `:4217`, `:4447` | `BuiltBoundary::Namespace` and `hands_command` preserve the harness-spawned hands channel. A broker stays that harness's child; boxing its server does not move harness ownership. |
| `crates/brokkr-protocol/src/secret.rs:122` | U6a's `bind_environment` already owns the sole `expose_for_spawn` invocation, and is `pub(crate)`. The consuming protocol-side handoff or visibility change must be budgeted, not assumed callable from CLI. |
| `crates/brokkr-cli/tests/machine_proof.rs:3142` | The machine proof includes secret.rs and pins `bind_environment` as the single call site. New transport must preserve it (0012 and 0071 ruling 5). |
| `crates/brokkr-cli/src/broker.rs:82` and `crates/brokkr-cli/tests/capability_broker.rs` | U6b has bounded plan/digest CLI parsing and unconditionally refuses PlanUnbound. Its real-binary tests pin that refusal and the realm-wide compile fence. It is not a delivered session. |
| `contracts/tool-dialect.v1.schema.json` | Frozen v1 has argv/URL, names/version/retained/egress but no package-root key. The amended conservative installed-entry rule requires no schema change and permits safe failure for other layouts. |

Local git history confirms U6b (`d0af931c`, #545), U5a2 (`e225066e`, #537),
U5f (`79addef6`, #541), U4a2 (`37e9250a`, #546) and U4b (`19bca5ff`, #547)
on this base. No branch from a held U6c run was checked out or merged.

## Specification decisions and finding disposition

[MB3/MB4 and their Decisions](../specs/mcp-capability-broker/spec.md#decisions)
own the complete H1–H7 claim-by-claim reconciliation and exact causes; no
parallel refusal vocabulary is defined here. H2/H3/H7's language-loading
proofs are replaced by the admitted empty-root box. H4 is confined only with
clean host launcher environment, actual readiness and the private handoff.
H1 and H5 retain whole-chain host identity checks; child private paths are
fresh tmpfs. H6 still needs positive store-identity exclusion, including bind
aliases, before lookup. Canonical path and hard-link checks are necessary
but do not alone identify a bind alias. None is declared a repaired runtime
HIGH finding by these documents (0071 rulings 3, 8, 9).

The proposed package-root choice is intentionally conservative: canonical
installed executable parent, or one level above an immediate bin/sbin;
system-contained entries use only the fixed system set. An installation can
place its launcher under its package/bin to bind sibling modules. No argv,
loader, shebang, package-manager or error-driven widening is admitted. An
unavailable dependency fails inside the box. Clarification and council must
assess this explicit compatibility cost, not silently add v1 fields.

MB4 defines eight fixed environment entries and preserves all shared name
refusals. In particular `_JAVA_OPTIONS` is not a valid decision-0012 name;
otherwise-valid loading names are allowed only for the final boxed exec.
Readiness precedes value lookup; a private anonymous handoff avoids exposing
those values to bubblewrap's host loader or argv. Box admission refusals
have zero lookups and dialect-server starts. Execution or handoff failure
after admission is explicitly a later failure, with zero invented calls.

## Requirement and proof handoff

All 26 existing requirement IDs are retained. The amendment changes MB1–MB5,
SC1, SD1–SD3, CR2–CR4 and SI1's explanatory boundary; unchanged requirements
retain their independent authority. Answers are scenarios and rejected
alternatives are in the owning Decisions sections. The exact refusal table
and its scenarios cover every new pre-secret box cause, store exclusion,
confined handoff failure and boxed-exec failure.

| Obligation | Owning requirements | Future proof owners to retain in the unit plan |
| --- | --- | --- |
| Program/system binds, package layout, all reach modes, hard links, aliases and whole ancestry | MB3, SC1 | CLI capability_broker; protocol hands tests; runtime capability_broker_launch and boundary tests |
| Store exclusion, fixed environment, valid loading variables, anonymous handoff, egress and binding minimum | MB2–MB4, SI1 | CLI capability_broker and machine_proof; U5a2 capability/bundle tests; runtime capability_broker_launch |
| Supervised box/stdio, private tmpfs lifetime, pre-secret cancellation and cleanup | MB5, CR3–CR4 | CLI capability_broker; runtime cleanup and capability_ledger suites; real Linux namespace evidence |
| Box failure overrides harness success without synthetic calls; evidence outside server mounts | CR2–CR4, CC2 | Runtime capability_ledger and capability_broker_launch; shared helpers plus integrated U9b path |
| Shared builder/injector consumers, at most three production files, inert merges and final activation | SD2–SD4 | Hands parity, broker CLI fence, machine_proof and integrated U9b tests; final count/inventory audit |

Each future behavioral test needs exact values/variants and a compiling
removal that fails its intended assertion, restored and recorded. No tests
or mutations are added or run by this documentary specify visit. New test
children must obey the owning suite and file-size rules; their file budgets
are separate from production. External namespace, Linux/macOS, exact coverage
and final-head CI results remain implementation obligations, not doc evidence.

## Dependent artifact work still owed in this run

The existing design and tasks were read and preserved for their assigned
seats. They still contain the old arbitrary-startup contract and cannot yet
be called coherent with this specification. This is pending downstream
amendment work, not an excuse to implement the old plan or report analyze
consistent. The supplied ruling resolves the earlier specification fault;
this specify result does not claim the whole design run complete.

Council must amend D5 and reconcile D3, D6, D9, D10, goals/non-goals, risks
and migration; place the detailed alternatives under design Decisions and
reconcile every received council position. Re-plan U6c onward with explicit
shared-builder and secret-handoff consumers, registration and all file paths.
Count the current 47-row design base, add each necessary split, then update
its sole merge order, details and Hot files consistently. Include affected
U7 reach/box intent, U8 failure/evidence settlement, and U9 integrated proof.
Keep U9b the sole compile-fence removal; direct broker serving stays refused
until its full protections exist.

Tasks must then rescope groups 28–32, affected 33–38/38a and 39–47, plus
final audit ownership as needed; preserve unchanged IDs/completed facts and
add new IDs for boxing. Do not edit the preamble's historical 45 PR / 105 task
counts. The design count and unit inventory, not that historical sentence,
are the amended order. Analysis must check proposal/deltas/design/tasks as a
whole and return any earliest-owner fault instead of hiding it downstream.

## Document validation and delivery

Observed during specify:

- `git diff --check` and `git diff --cached --check`: passed on the complete
  nine-document candidate; no whitespace errors.
- `openspec validate --all --strict`: passed, 20 items, zero failures. The
  final repeat also passed all 20 items. The command emitted informational long-requirement notices and unrelated
  pre-existing archive notices for adapter-resume-safety and
  sdd-progress-markers; no archive was attempted.
- `openspec validate decision-0065-capabilities-slice-two --strict --no-interactive`:
  passed for the adopted change.
- A read-only document audit confirmed the exact docs-only changed-path set,
  unchanged design/tasks/evidence.md bytes, all 26 requirement IDs with
  scenarios, the once-appended verbatim ruling and existing local link targets.
  This is a document audit, not a behavioral test.
- `typos --hidden`: unavailable (`/bin/bash: typos: command not found`).
  Spelling validation is pending outside this box; no pass is claimed.
- `git config --get commit.gpgsign`: `false`, the boxed hands policy from
  0043 ruling 6. This seat's document commit is unsigned as authorized; the
  controller signs the squash. No push is authorized or performed.

Cargo tests, bundle compiles, format/clippy and exact coverage are not claimed
from document validation. They remain the future implementation rows' gates;
this commission authorizes no production or test change. Result is written
through workspace hands with `inputs.change`, without a next-phase field.

## Council design visit

Seat: chief, design phase only; starting HEAD `35593f8c`, clean tracked tree.
The journal supplied a clear clarify result and two completed positions.
Adopted `decision-0065-capabilities-slice-two`; did not create another change
or invoke the dialect workflow runner. Read the dialect definition and all
rendered phase instructions through workspace hands. Revisited the proposal
and owning scenarios first, amended D5 and dependent design sections, then
reconciled tasks. This is the requested document draft; it does not choose a
phase-machine route or claim to have run another office.

Read the commissioned proposal, whole design, tasks, MB1–MB5, SC1–SC5 and
SD1–SD4, retention and affected isolation requirements; README and decisions
0004/0005, 0012, 0043, 0046, 0063, 0065, 0071 and 0077. Read both complete
council positions from `.forge/design/positions/robustness.md` and
`.forge/design/positions/simplicity.md`. Read hands.rs, hands/overlay.rs and
hands/session.rs, the runtime Namespace/hands_command seams, U6a's injector
and store reader, U6b's command and CLI tests, machine_proof's single-accessor
check and #403's documented residuals. These reads support design, not a
claim that the proposed mechanisms already exist.

Unlike the earlier specify seat, this visit could inspect held implementation
evidence through local git objects: `65c879c5` for run
`0065-slice-two-unit-u6c-see-the--99c50c0b`, and `08171090` for run
`0065-slice-two-unit-u6c-see-the--f48203e8`, each at the change's
`evidence/U6c.md`. No held branch was checked out, merged or used as code.
The second record acknowledges unqualified ELF/RUNPATH, fixed PATH,
bind-alias, sticky-ancestor and bare-runtime cases. Its argument/shebang
walk and growing loading-name denylist are superseded, not repaired here.
The seven HIGH findings remain commission-supplied findings; this visit did
not reproduce them or close them as runtime security results.

Rechecked source facts that affect the cuts:

- `native_controls.rs` declares `mod mcp` privately. The final plan uses a
  consumed `protocol/broker.rs`, registered in `protocol/lib.rs` in U6c,
  rather than adding a fourth visibility file or exporting Transport twice.
  U6d can then register its reachable ledger child within its three files.
- `hands.rs` is 1,267 lines, `secret.rs` 706 and `cli_args.rs` 699 at this
  base. The shared builder, observer, bootstrap and store reader each have
  named extractions/registrations and consumers; no ceiling exception is
  inferred. The gate must measure future code, not this line budget.
- `read_store` checks pathname metadata then reads that path; it does not
  bind lookup to admitted identity. `resolve_bindings` skips empty names.
  MB4 now explicitly requires a descriptor-bound read and an existing
  protected store even for a secret-free server; an empty store is allowed.
- `bwrap --version` reported `bubblewrap 0.11.0`; `bwrap --help` advertises
  `--ro-bind-fd`, `--ro-bind-data`, namespace flags and `--die-with-parent`.
  This is interface evidence only. No namespace was launched and no
  control-FD carriage, mount, readiness or cleanup guarantee was measured.
- Existing #403 code names the Linux table-read and read-to-fork residuals
  pending #472. Omitting `--new-session` preserves attempt ownership; it is
  not a cgroup guarantee. macOS remains a refusal platform for this feature.

## Council reconciliation

[Design D12](../design.md#d12-council-reconciliation-for-the-boxed-server-amendment)
contains the explicit claim dispositions in the dialect's decisions place.
Adopted both positions' shared builder, conservative installation convention,
fixed environment, confined handoff, independent native-read qualification,
all-writer evidence protection and existing cleanup. Adopted robustness's
bounded source observer, whole-chain/mount-alias checks, descriptor-bound
store read and separate bootstrap. Accepted both positions' compatibility
cost: a layout outside this bounded support refuses or fails inside, never
widens a mount or retries on the host.

Rejected simplicity's proposed 51-row cut, with evidence rather than a
compromise count: it did not budget the private shared-type visibility,
store-reader space and accumulated bootstrap responsibilities adequately.
The final 54-row order adds seven cuts, U6c2–U6c8, with at most three
production files each. It retains the minimal-service intent: one box
builder, one injector, a private command in the existing binary, no daemon,
installer, loader analyzer or new public contract version. The shared type
home avoids another visibility split when ledger types arrive. Every later
U6/U7/U8/U9 consumer, its task IDs and Hot files follow that single inventory.

H1–H7 each have a construction/remaining-check disposition in D5 and MB3's
Decisions. H1/H5/H6 keep host-source/ancestry/store checks; H2/H3/H7 lose
language-startup analysis; H4 requires clean host launchers and confined
post-readiness bindings as well as the existing shared name refusals. No
whole-harness sandbox or root-owned-only installation rule replaces those
checks. Typed safe failure, masking, plan binding and evidence settlement
remain independent duties under 0071 rulings 1–3 and 7–10.

## Dependent artifact consistency audit

The council repaired the owning scenarios before dependent design/tasks:
bounded observation/readiness/handoff, system loading aliases, a missing
store with no bindings, same-descriptor reading and exact refusal timing.
D5 removes the old no-server-sandbox non-goal and startup analyzer; D3/D6/
D7/D9/D10, risks, migration, open questions and U6 onward now agree. The
operator's addendum and 0077's already-landed one-line pointer are preserved
without another copy or a change to decision acceptance. Historical
`evidence.md`, frozen files and completed implementation facts are unchanged.

Read-only audit over the candidate confirmed 54 unique unit rows, all
production budgets at or below three, every dependency earlier in the one
order, one detailed section per row, and Hot files agreeing with the table.
It confirmed 123 unique task IDs and all 32 completed task lines unchanged;
the **entire** preamble, including its historical counts, is byte-identical
to this visit's starting HEAD. All 26 requirements retain scenario and task
coverage; all six new MB3 box causes have explicit scenarios, as do store
exclusion, handoff and confined exec failure. U9b alone removes the realm-wide
compile fence; incomplete direct serving still refuses through U6f and
retained plans through U8b.

The first audit pass incorrectly counted an existing parenthetical list of
transitive prerequisites as extra direct edges. After correcting the audit,
it caught an accidental U6c2 → U6c8 task dependency introduced while changing
U6d; that task was corrected to U6c. A manual file-consumer review then moved
the proposed shared plan/ledger type home into the explicitly registered
protocol/broker module to avoid U6d's otherwise hidden visibility edit. The
repeated audit passed. These were document corrections, not runtime findings.

This visit leaves no unresolved documentary upstream fault. Future proofs
must still establish source identity, real namespace readiness, confined
injection and cleanup; a failing implementation must refuse or return to its
owning design rather than weaken any check. No checkbox is closed by planning.

## Council design validation

Observed through workspace hands on this document candidate:

- `git diff --check` and `git diff --cached --check`: passed.
- `openspec validate --all --strict`: 20 passed, zero failed. Informational
  long-requirement and unrelated pre-existing archive notices are not an
  archive operation; no workflow runner or archive was invoked.
- `python3 .forge/design/audit_boxed_docs.py`: passed the read-only inventory,
  dependency, task, scenario, H1–H7, exact-path and unchanged-ruling checks
  above. This ignored run-local script is a document audit, not a new test or
  a production artifact.
- Read-only local-link audit: 710 file-link targets checked, zero missing;
  the supplied ruling matches verbatim exactly once and 0077 retains its pointer.
- `typos --hidden`: unavailable, `/bin/bash: typos: command not found`.
  Spelling remains pending outside this box.
- `git config --get commit.gpgsign`: `false`. Decision 0043 ruling 6 keeps
  signing credentials outside the hands box; the authorized document commit
  is unsigned and the controller signs the squash. No push is performed.

Cargo tests, bundle compilation, formatting/clippy, behavior-removal
mutations, namespace evidence, Linux/macOS and exact coverage are future
implementation gates and were not run or claimed in this docs-only visit.
No production, test, fixture, policy, reference, contract, recipe or release
file is changed. The phase result is written through workspace hands to the
commissioned result path with `result: drafted` and `inputs.change`.
