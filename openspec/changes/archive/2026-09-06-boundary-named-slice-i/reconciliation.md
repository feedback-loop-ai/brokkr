# Reconciliation of the inherited implementation

Initial audit, 2026-09-06. Read the entire inherited tracked diff (including
its staged deletion), every new untracked implementation/contract/test file,
the proposal, design, tasks, eight delta specs, and cited decisions. This is
a snapshot of the inherited tree; `tasks.md` is the current completion record.
No inherited implementation was discarded wholesale.

Baseline `cargo test --workspace --no-fail-fast`: six failures across four
targets: two delivery runs exit 3, the CLI realms test still treats v4 as an
unknown version, two compose pin assertions and one witness pin assertion.
Formatting and clippy passed. Coverage stopped at the delivery failures.

A checked task below has its supporting tests present and passing in that
baseline. Partial tasks remain unchecked until their full named proof passes.
Claude measurements are explicitly operator-owned and remain honest absences;
the codex capture measurement is documented as pending, per task 8.9.

| Task | Inherited state | Evidence or remaining work |
|---|---|---|
| 1.1 | Satisfied | bundle/tests.rs retirement/parser/source tests; bundle/agent_tests.rs; machine_proof.rs |
| 1.2 | Satisfied | bundle/tests.rs retirement/parser/source tests; bundle/agent_tests.rs; machine_proof.rs |
| 1.3 | Satisfied | bundle/tests.rs retirement/parser/source tests; bundle/agent_tests.rs; machine_proof.rs |
| 1.4 | Satisfied | bundle/tests.rs retirement/parser/source tests; bundle/agent_tests.rs; machine_proof.rs |
| 1.5 | Partial | Partial: source scan exists; explicit unchanged seat/member/step argv coverage needs strengthening. |
| 2.1 | Satisfied | core/realms/tests.rs vocabulary and v4 loader tests; runtime/frozen_contracts.rs |
| 2.2 | Untouched | Untouched: word/sentinel functions exist, but no Option<Boundary> serde helper. |
| 2.3 | Satisfied | core/realms/tests.rs vocabulary and v4 loader tests; runtime/frozen_contracts.rs |
| 2.4 | Satisfied | core/realms/tests.rs vocabulary and v4 loader tests; runtime/frozen_contracts.rs |
| 2.5 | Satisfied | core/realms/tests.rs vocabulary and v4 loader tests; runtime/frozen_contracts.rs |
| 2.6 | Satisfied | core/realms/tests.rs vocabulary and v4 loader tests; runtime/frozen_contracts.rs |
| 2.7 | Partial | Partial: file/title and predecessor pins exist; v4 schema rejection tests missing. |
| 3.1 | Satisfied | cli/boundary_verbs.rs; runtime/bundle/{tests,agent_tests,model_policy_tests}.rs; engine/boundary_tests.rs |
| 3.2 | Satisfied | cli/boundary_verbs.rs; runtime/bundle/{tests,agent_tests,model_policy_tests}.rs; engine/boundary_tests.rs |
| 3.3 | Satisfied | cli/boundary_verbs.rs; runtime/bundle/{tests,agent_tests,model_policy_tests}.rs; engine/boundary_tests.rs |
| 3.4 | Partial | Partial: harness run/rerun and unbuilt compile tested; pinned namespace resume after map change and witness gate outstanding. |
| 3.5 | Satisfied | cli/boundary_verbs.rs; runtime/bundle/{tests,agent_tests,model_policy_tests}.rs; engine/boundary_tests.rs |
| 3.6 | Satisfied | cli/boundary_verbs.rs; runtime/bundle/{tests,agent_tests,model_policy_tests}.rs; engine/boundary_tests.rs |
| 3.7 | Partial | Partial: binary self printout tested; plain binary printout lacks the asserted absent keys. |
| 4.1 | Satisfied | runtime/bundle/model_policy_tests.rs manifest validation; dispatch_seam.rs; engine/boundary_tests.rs resume |
| 4.2 | Partial | Partial: both maps exist, but two separate loops contradict the required single loop. |
| 4.3 | Partial | Partial: new schema and boundary identities tested; fixed witness suite is red. |
| 4.4 | Partial | Partial: v9 validation passes; predecessor coverage needs extending across every frozen version. |
| 4.5 | Satisfied | runtime/bundle/model_policy_tests.rs manifest validation; dispatch_seam.rs; engine/boundary_tests.rs resume |
| 4.6 | Satisfied | runtime/bundle/model_policy_tests.rs manifest validation; dispatch_seam.rs; engine/boundary_tests.rs resume |
| 4.7 | Satisfied | runtime/bundle/model_policy_tests.rs manifest validation; dispatch_seam.rs; engine/boundary_tests.rs resume |
| 5.1 | Satisfied | cli/boundary/tests.rs; doctor/tests.rs; boundary_verbs.rs; engine/boundary_tests.rs entry fences |
| 5.2 | Satisfied | cli/boundary/tests.rs; doctor/tests.rs; boundary_verbs.rs; engine/boundary_tests.rs entry fences |
| 5.3 | Satisfied | cli/boundary/tests.rs; doctor/tests.rs; boundary_verbs.rs; engine/boundary_tests.rs entry fences |
| 5.4 | Satisfied | cli/boundary/tests.rs; doctor/tests.rs; boundary_verbs.rs; engine/boundary_tests.rs entry fences |
| 5.5 | Satisfied | cli/boundary/tests.rs; doctor/tests.rs; boundary_verbs.rs; engine/boundary_tests.rs entry fences |
| 5.6 | Satisfied | cli/boundary/tests.rs; doctor/tests.rs; boundary_verbs.rs; engine/boundary_tests.rs entry fences |
| 5.7 | Satisfied | cli/boundary/tests.rs; doctor/tests.rs; boundary_verbs.rs; engine/boundary_tests.rs entry fences |
| 5.8 | Satisfied | cli/boundary/tests.rs; doctor/tests.rs; boundary_verbs.rs; engine/boundary_tests.rs entry fences |
| 5.9 | Satisfied | cli/boundary/tests.rs; doctor/tests.rs; boundary_verbs.rs; engine/boundary_tests.rs entry fences |
| 5.10 | Satisfied | cli/boundary/tests.rs; doctor/tests.rs; boundary_verbs.rs; engine/boundary_tests.rs entry fences |
| 5.11 | Partial | Partial: doctor carries boundary but compiles without the discovered realm dialect. |
| 5.12 | Partial | Partial: boundary_verbs integration covers harness/container; required init_doctor matrix and dialect compilation still missing. |
| 5.13 | Satisfied | cli/boundary/tests.rs; doctor/tests.rs; boundary_verbs.rs; engine/boundary_tests.rs entry fences |
| 6.1 | Satisfied | runtime/bundle/model_policy_tests.rs grammar/policy/walk matrix; compose_tests.rs ancestor walk |
| 6.2 | Satisfied | runtime/bundle/model_policy_tests.rs grammar/policy/walk matrix; compose_tests.rs ancestor walk |
| 6.3 | Satisfied | runtime/bundle/model_policy_tests.rs grammar/policy/walk matrix; compose_tests.rs ancestor walk |
| 6.4 | Satisfied | runtime/bundle/model_policy_tests.rs grammar/policy/walk matrix; compose_tests.rs ancestor walk |
| 6.5 | Partial | Partial: hands law precedes tier in policy, but agent composition can refuse first; D33 test contradicts its task. |
| 6.6 | Satisfied | runtime/bundle/model_policy_tests.rs grammar/policy/walk matrix; compose_tests.rs ancestor walk |
| 6.7 | Satisfied | runtime/bundle/model_policy_tests.rs grammar/policy/walk matrix; compose_tests.rs ancestor walk |
| 6.8 | Satisfied | runtime/bundle/model_policy_tests.rs grammar/policy/walk matrix; compose_tests.rs ancestor walk |
| 6.9 | Satisfied | runtime/bundle/model_policy_tests.rs grammar/policy/walk matrix; compose_tests.rs ancestor walk |
| 6.10 | Satisfied | runtime/bundle/model_policy_tests.rs grammar/policy/walk matrix; compose_tests.rs ancestor walk |
| 6.11 | Satisfied | runtime/bundle/model_policy_tests.rs grammar/policy/walk matrix; compose_tests.rs ancestor walk |
| 7.1 | Partial | Partial: SpawnEnv implemented; explicit Inherit/Exactly child environment tests missing. |
| 7.2 | Satisfied | engine/boundary_tests.rs composition/probe/walk; protocol/hands/tests.rs environment and prefix |
| 7.3 | Satisfied | engine/boundary_tests.rs composition/probe/walk; protocol/hands/tests.rs environment and prefix |
| 7.4 | Satisfied | engine/boundary_tests.rs composition/probe/walk; protocol/hands/tests.rs environment and prefix |
| 7.5 | Satisfied | engine/boundary_tests.rs composition/probe/walk; protocol/hands/tests.rs environment and prefix |
| 7.6 | Satisfied | engine/boundary_tests.rs composition/probe/walk; protocol/hands/tests.rs environment and prefix |
| 7.7 | Partial | Partial: single/sequence spawns re-walk; panel spawns bypass it. |
| 7.8 | Satisfied | engine/boundary_tests.rs composition/probe/walk; protocol/hands/tests.rs environment and prefix |
| 7.9 | Satisfied | engine/boundary_tests.rs composition/probe/walk; protocol/hands/tests.rs environment and prefix |
| 7.10 | Partial | Partial: pure table tested; planted SSH child-environment proof and full Windows table test missing. |
| 7.11 | Partial | Partial: direct single re-walk tests exist; actual successful spawn, panel, and inherited spawn proof incomplete. |
| 8.1 | Satisfied | runtime/agents/tests.rs harness loader/data; model_policy_tests.rs; engine/boundary_tests.rs; protocol/adapters/tests.rs |
| 8.2 | Satisfied | runtime/agents/tests.rs harness loader/data; model_policy_tests.rs; engine/boundary_tests.rs; protocol/adapters/tests.rs |
| 8.3 | Satisfied | runtime/agents/tests.rs harness loader/data; model_policy_tests.rs; engine/boundary_tests.rs; protocol/adapters/tests.rs |
| 8.4 | Satisfied | runtime/agents/tests.rs harness loader/data; model_policy_tests.rs; engine/boundary_tests.rs; protocol/adapters/tests.rs |
| 8.5 | Operator measurement deferred by D21/task 8.5 | Operator measurement deferred by D21/task 8.5: Claude fragments honestly absent; guide has candidates and recipe. |
| 8.6 | Conditional on operator measurement | Conditional on operator measurement: no unsupported reason invented; loader tests cover measured declarations. |
| 8.7 | Partial | Partial: adapters unchanged, but inherited test asserts the wrong refusal under harness. |
| 8.8 | Satisfied | runtime/agents/tests.rs harness loader/data; model_policy_tests.rs; engine/boundary_tests.rs; protocol/adapters/tests.rs |
| 8.9 | Satisfied | runtime/agents/tests.rs harness loader/data; model_policy_tests.rs; engine/boundary_tests.rs; protocol/adapters/tests.rs |
| 8.10 | Satisfied | runtime/agents/tests.rs harness loader/data; model_policy_tests.rs; engine/boundary_tests.rs; protocol/adapters/tests.rs |
| 8.11 | Partial | Partial: argv/input/prompt units present; file-door and malformed capture path need a joined proof. |
| 8.12 | Partial, contradicts D33 | Partial, contradicts D33: inherited test explicitly expects generic resolver hands refusal instead of gate fragment/tier ordering. |
| 9.1 | Satisfied | engine/boundary_tests.rs entry/stamp/input; store/seat_record.rs version tests; protocol/adapters/tests.rs prompts |
| 9.2 | Partial | Partial: extension schema exists and is listed; emitted entries not yet validated against it. |
| 9.3 | Partial | Partial: entry shapes/fold tested; no-hands byte equality needs explicit emitted-payload assertion. |
| 9.4 | Satisfied | engine/boundary_tests.rs entry/stamp/input; store/seat_record.rs version tests; protocol/adapters/tests.rs prompts |
| 9.5 | Satisfied | engine/boundary_tests.rs entry/stamp/input; store/seat_record.rs version tests; protocol/adapters/tests.rs prompts |
| 9.6 | Satisfied | engine/boundary_tests.rs entry/stamp/input; store/seat_record.rs version tests; protocol/adapters/tests.rs prompts |
| 9.7 | Partial | Partial: stamps/version tests pass; actual boxed export test is red and invalid-boundary append failure proof missing. |
| 9.8 | Satisfied | engine/boundary_tests.rs entry/stamp/input; store/seat_record.rs version tests; protocol/adapters/tests.rs prompts |
| 9.9 | Satisfied | engine/boundary_tests.rs entry/stamp/input; store/seat_record.rs version tests; protocol/adapters/tests.rs prompts |
| 9.10 | Partial | Partial: prompt/input units pass; required shipped self verifier input/prompt matrix missing. |
| 10.1 | Satisfied | view/tests.rs; cli/{render,tui,ui,compare}/tests.rs; boundary_readouts.rs |
| 10.2 | Partial | Partial: last attempt word used for old rows and stale words retained after a retry without an entry. |
| 10.3 | Partial | Partial: fallback exists, but stamp precedence on rows and retry reset need correction. |
| 10.4 | Satisfied | view/tests.rs; cli/{render,tui,ui,compare}/tests.rs; boundary_readouts.rs |
| 10.5 | Partial | Partial: single-attempt coverage passes; retries and row-local attempt semantics unproved. |
| 10.6 | Partial | Partial: four headers implemented; full four-state rendering matrix missing. |
| 10.7 | Satisfied | view/tests.rs; cli/{render,tui,ui,compare}/tests.rs; boundary_readouts.rs |
| 10.8 | Satisfied | view/tests.rs; cli/{render,tui,ui,compare}/tests.rs; boundary_readouts.rs |
| 10.9 | Satisfied | view/tests.rs; cli/{render,tui,ui,compare}/tests.rs; boundary_readouts.rs |
| 10.10 | Satisfied | view/tests.rs; cli/{render,tui,ui,compare}/tests.rs; boundary_readouts.rs |
| 10.11 | Satisfied | view/tests.rs; cli/{render,tui,ui,compare}/tests.rs; boundary_readouts.rs |
| 10.12 | Satisfied | view/tests.rs; cli/{render,tui,ui,compare}/tests.rs; boundary_readouts.rs |
| 10.13 | Satisfied | view/tests.rs; cli/{render,tui,ui,compare}/tests.rs; boundary_readouts.rs |
| 10.14 | Partial | Partial: two delivery integration tests fail (run exits 3). |
| 11.1 | Untouched | Untouched: witness and compose pins still contain pre-change digests; three tests fail. |
| 11.2 | Untouched | Untouched: pin comments have not recorded decision 0046 as reason for new digests. |
| 11.3 | Partial | Partial: baseline fixed points reached before failing fast witness; full pin gate outstanding. |
| 11.4 | Satisfied | runtime/bundle/model_policy_tests.rs shipped 13-bundle positive/refusal and measured-gap matrices |
| 11.5 | Satisfied | runtime/bundle/model_policy_tests.rs shipped 13-bundle positive/refusal and measured-gap matrices |
| 11.6 | Untouched | Untouched: completion note remains to write with measurements, four refusals, and narrowed extension reading. |
| 11.7 | Satisfied | runtime/bundle/model_policy_tests.rs shipped 13-bundle positive/refusal and measured-gap matrices |
| 12.1 | Satisfied | cli/tests/contributing.rs boundary prose assertions; inherited guide diff and exact decision erratum |
| 12.2 | Satisfied | cli/tests/contributing.rs boundary prose assertions; inherited guide diff and exact decision erratum |
| 12.3 | Satisfied | cli/tests/contributing.rs boundary prose assertions; inherited guide diff and exact decision erratum |
| 12.4 | Satisfied | cli/tests/contributing.rs boundary prose assertions; inherited guide diff and exact decision erratum |
| 12.5 | Satisfied | cli/tests/contributing.rs boundary prose assertions; inherited guide diff and exact decision erratum |
| 12.6 | Satisfied | cli/tests/contributing.rs boundary prose assertions; inherited guide diff and exact decision erratum |
| 12.7 | Satisfied | cli/tests/contributing.rs boundary prose assertions; inherited guide diff and exact decision erratum |
| 12.8 | Satisfied | cli/tests/contributing.rs boundary prose assertions; inherited guide diff and exact decision erratum |
| 12.9 | Satisfied | cli/tests/contributing.rs boundary prose assertions; inherited guide diff and exact decision erratum |
| 12.10 | Satisfied | cli/tests/contributing.rs boundary prose assertions; inherited guide diff and exact decision erratum |
| 12.11 | Partial | Partial: test checks only sampled headings, omits read-surfaces; full retained-section pin missing. |
| 12.12 | Satisfied | cli/tests/contributing.rs boundary prose assertions; inherited guide diff and exact decision erratum |
| 13.1 | Satisfied | Local baseline fmt and clippy logs: /tmp/brokkr-boundary-{fmt,clippy}.log |
| 13.2 | Satisfied | Local baseline fmt and clippy logs: /tmp/brokkr-boundary-{fmt,clippy}.log |
| 13.3 | Pending | Pending: baseline workspace run failed six tests across four targets. |
| 13.4 | Pending | Pending: package not run. |
| 13.5 | Pending | Pending: standalone bundle compile gates not run. |
| 13.6 | Pending | Pending: exact coverage stopped at failing delivery tests. |
| 13.7 | Untouched | Untouched: archive and archived validation not run. |
| 13.8 | Untouched | Untouched: inherited work was uncommitted; no commit made during reconciliation. |

## Continuation corrections

The inherited D33 pin expected the resolver's generic hands gap, contrary to
the operator's refusal ordering. The hands law now runs before composition gaps;
namespace keeps the provider-tier refusal and harness names its missing fragment.
Unboxed work agents no longer need the namespace workspace fragment.

The inherited panel spawn bypassed the re-walk; every spawn now shares its
fence. The declaring layer follows the script token before later arguments.
Attempt-local boundary entries now win over contradictory result stamps, and
a retry cannot inherit a prior attempt's word. Doctor compiles in the discovered
realm, including its dialect. The manifest maps now share one loop.

The initial audit overstated two proofs: task 7.4 still needed a three-arm built
boundary type, now implemented, and task 6.9 needed the explicit isolated self
verifier proof, now added. Conversely, the inherited environment test already
spawned a child to prove SSH_AUTH_SOCK absent; task 7.10 needed the full Windows
bootstrap matrix, not a second SSH proof. USERPROFILE is now consulted only on
Windows when deriving the home, matching the table's platform rule.

Delivery fixtures exclude engine scratch and instrumentation profile files from
source dirtiness. Witness and compose pins were taken from successive failing
tests' left/right pairs, with decision 0046 named in both pin-file doc comments.
The frozen evaluator corpus and existing contract versions remain unchanged.
The provider and recipe guides also lose two stale claims: dsh/lanetally boxed
hands capability, and doctor dialect compilation being future work.
