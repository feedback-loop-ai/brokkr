## Context

Adopt change `2026-09-21-307-astra-engine-smith` on
`slice-307-astra-smith`, at specify revision `5ef97115`. See
[proposal.md](proposal.md#why) for motivation and the
[smith](specs/astra-engine-smith/spec.md) and
[admission](specs/boxed-work-provider-admission/spec.md) deltas for requirements.
The existing proposal and deltas already encode both operator answers and
returned clarification A1. They need no amendment for this design. There is
no new `returned_from` finding in this visit.

The chief read both complete council positions, `robustness.md` and
`simplicity.md` in `.forge/design/positions/`, the framing, README, decisions
0004, 0005, 0009, 0043, 0046 and 0063, and the relevant source and history.
OpenSpec's `instructions design --change
2026-09-21-307-astra-engine-smith --json` assigns this artifact. This phase
writes only `design.md`; the task ledger follows in its assigned phase.
No workflow runner is involved.

Current source supports applying the existing rules without production Rust
changes:

- `agents/implementer-engine.json` hires Fable then Opus, both high, and
  declares Cargo/Git tools without hands. Ordinary boxed work offices
  `intake-sdd` and `chief-architect` declare network false;
  `review-correctness` supplies the existing Rust toolchain bind pattern.
- `agents.rs::compose` checks hands before `tools.allow`. For boxed hands it
  requires and appends the adapter workspace fragment. Without hands it
  retains the unsupported-tool-permissions refusal. `resolve_report` checks
  every mapped entry for capability gaps before selecting candidates.
- `bundle.rs::enforce_hands_boundary` returns for boxed boundaries; under
  harness it separately requires `hands.harness.work` on every work link.
  `engine.rs::compose_site` expands namespace hands or appends the harness
  fragment according to the boundary. It does not combine those paths.
- Codex declares workspace hands with a read-only native sandbox and MCP
  server, and harness work with a writable native sandbox. Claude declares
  workspace hands, including its MCP permission grant, and no harness work.
- `brokkr_protocol::hands::HandsSpec` contains network and binds;
  `execute_in` runs `/bin/bash -lc`. `box_argv` supplies the writable workdir
  and existing Git metadata protections. There is no command allow-list.

These findings agree with the hands enactment (`0e3df556`) and named-boundary
slice (`7b53e929`). They are source evidence, not executed proof of the new
roster or a live Astra implementation.

## Goals / Non-Goals

**Goals:** Use the smallest declaration change that expresses the ruled hire;
prove admission and refusal at their owning layers; independently inspect the
actual shipped candidates' composed launches; and make every resulting
identity movement traceable to successful compilation.

**Non-Goals:** No command filtering, shell parser, transport contract,
native-tool bypass prevention, new adapter capability, boundary, trust tier,
resume qualification, dependency or production abstraction. No widening of
network or host-write access, fallback expansion, release work or live probe.
Existing open, Seatbelt and container behavior remains with its owning rules.
Linux and macOS are the hosts; this slice adds no Windows handling and does
not activate another boundary on macOS.

Protected surfaces remain untouched: `contracts/`,
`policy/phase-machine.json`, `policy/schemas/`, `fixtures/`, `reference/`,
`extensions/`, and the issue #226 task ledger. Temporary Rust test trees are
not additions to the frozen evaluator corpus.

## Decisions

### D1 — Reconcile both positions against the implementation

| Council claim | Disposition and reason |
| --- | --- |
| Robustness R1 and simplicity 1: one hands declaration for both hires, remove tools | Adopt. `compose` ignores tools whenever hands exist, including on Claude; the roster rejects dead tools beside hands. D2 fixes the exact declaration. |
| Robustness R2 and simplicity 2: preserve admission and the separate harness path | Adopt. The current resolver and boundary policy already implement the operator's answer. D3 tests those paths without a Codex-specific exception. |
| Robustness R2: distinguish missing workspace from malformed adapter JSON | Adopt as a necessary refinement of simplicity's compact matrix. The loader requires a workspace array in a capability object; D3 separates loadable compile fixtures from the internal resolver control. No specification change is needed. |
| Robustness R3 and simplicity 3: use shipped composition and independent expectations | Adopt. The existing generic boundary test compares against `hands_command` itself and cannot be the sole oracle for this change. D4 covers both actual providers and A1. |
| Robustness R4 and simplicity 3: observe each removal failure | Combine. Keep robustness's separate protection targets and simplicity's existing test seams; use temporary mutations and a compact evidence table, without a mutation framework or duplicated policy matrix. D5 specifies the evidence. |
| Robustness R5 and simplicity 4: measure identities and amend existing documentation | Adopt. Hands, resolved agents, consulted adapters and composed ancestors participate in identity. D6 covers all measured movements and preserves historical facts. |
| Robustness R6 and simplicity's evidence limits | Adopt. Gate results, external coverage and the controller's live measurement are separate evidence. D7 records that boundary without waiving a gate. |
| Simplicity's scope cuts and both positions' rejected alternatives | Adopt. No resolver refactor, adapter edit, new test crate, command enforcement system, extra hire or platform campaign is justified by this source. A compiler repair is conditional on a required failing regression. |

The synthesis preserves the specific regression obligations rather than
reducing them to a successful roster compile. It also rejects extra production
machinery: the observed gap is the declaration and its evidence. Both
positions' references to later design/tasks are interpreted through the
rendered phase assignment: this visit authors the design only.

### D2 — Replace the inactive tool policy with the existing workspace policy

Set the shipped agent's models to `["astra", "fable"]` and efforts to
`{"astra":"high","fable":"high"}`. Retain its description, charter and
limits. Remove the entire `tools` object, including the empty MCP list, and
use exactly:

```json
"hands": {
  "kind": "workspace",
  "network": false,
  "binds": [
    {
      "path": "~/.cargo",
      "mode": "overlay",
      "mask": ["credentials.toml", "credentials"]
    },
    {"path": "~/.rustup", "mode": "ro"}
  ]
}
```

Workspace hands already bind the workdir read-write. These are additional
build-tool binds, not a replacement workspace mount. The existing Git
handling supports linked worktrees without granting the checkout parent or
host home. Boundary stays a realm fact; no agent boundary or machine-specific
checkout path is added.

Keeping tools for capable providers is rejected: the same precedence bypasses
that list on Claude, and the existing roster invariants forbid the dead
policy. A test-only dual declaration will prove the precedence instead.
A writable Cargo host bind is rejected because build scripts could replace
host executables. The overlay and both masks preserve the shipped precedent;
insufficient overlay support stays a refusal. The release manager's network
and resolver grants are specific to its office and are not copied here.

This records the operator's 2026-09-21 answer: the restriction is decision
0043's existing filesystem/network confinement. The earlier claim that the
hands box enforces `["cargo", "git"]` is withdrawn. No per-tool contract is
being designed, and that answered escalation is not reopened.

### D3 — Test the existing admission rules before considering a compiler edit

Place the full compile matrix in
`crates/brokkr-runtime/src/bundle/model_policy_tests.rs`, using its
`Fixture::compile_roots` / `Bundle::compile_under` seams and a minimal,
explicitly work-class seat. Use valid charters, results, mappings, effort
pins and unrelated capabilities so each case reaches its intended rule.
Canonicalize temporary roots before building bundle or workdir expectations;
keep the `TempDir` alive. Compilation is pure and uses unspecified provider
availability, never an installed-binary probe.

| Case | Declaration and boundary | Required observation |
| --- | --- | --- |
| N0 | Fixture Codex, `tools.allow: ["cargo", "git"]`, no agent hands, namespace | Exact current capability refusal with seat, agent, provider and model. A valid later candidate cannot rescue the chain. |
| N1 | Same fixture retaining tools, add agent hands, namespace, workspace fragment present | Compiles with Astra/Codex and high effort, hands and namespace recorded, complete workspace fragment, no generated tool-list arguments. |
| N2 | Same hands agent, namespace, adapter hands absent or unsupported | Existing unsupported-hands refusal with seat/agent/provider/model and any measured gap reason. |
| H1 | Same hands agent, Codex-only, harness work present, harness | Compiles. Composition from this unboxed resolution emits the writable harness fragment without workspace MCP or tool-list arguments; hands remain recorded, unenforced by Brokkr. |
| H2 | H1 with harness work absent or measured unsupported, workspace still present | Refuses with seat, link, provider, `hands.harness.work`, the writable-sandbox explanation, and decision 0046 rulings 1 and 4; preserves a measured reason. |
| H3 | Minimal work seat, actual shipped Astra/Fable chain, harness | Refuses at link 2 on Claude's missing `hands.harness.work`; unrelated review or dialect failures cannot substitute. |
| P1 | Capable Claude-shaped adapter, agent hands plus Cargo/Git tools, namespace | Preserves the complete workspace fragment and its single MCP grant while omitting the Cargo/Git grants, as detailed in D4. |

N0 compares the complete stable diagnostic, including the unchanged reason:

```text
the provider declares tool_permissions unsupported, so the agent's restriction to ["cargo", "git"] cannot be expressed and the agent would run with MORE power than it declares
```

The surrounding existing capability-error explanation is also pinned. Cover
bare and measured unsupported tool declarations. For N2, cover absent, bare
unsupported and measured unsupported hands declarations and preserve the
existing explanation that the hands cannot be put in the box and the agent
would run with the harness's own tools. Use `expect_err(...).to_string()` or
an explicit match and diagnostic equality/assertions, never `is_err()`.
Positive cases assert resolved facts with explanatory expectations; there is
no error reason on success.

**Missing-workspace fixture choice.** `agents/load.rs` requires `workspace`
as an array when `hands` is a capability object. Deleting that member while
leaving `hands.harness` produces a loader error, not N2's capability error.
N2 therefore uses the valid absent/unsupported whole-capability shapes.
Separately, in `agents/tests.rs`, load and clone a valid adapter, set its
parsed workspace capability (`Adapter.hands`) to `None` while retaining
`harness.work`, and call the existing boxed composition path. Assert the
unsupported-hands reason: harness support cannot rescue boxed resolution.
Label this an internal resolver control, not a loadable JSON fixture. Do not
change loader grammar or treat `workspace: []` as absence; empty and absent
are different in the existing contract.

Use `agents/tests.rs` for P1 as a focused precedence regression, retaining
native permission mappings on the capable provider. Keep the compile matrix
in one suite rather than repeating it at every layer. The H1 composition
control belongs with D4 and must start from a harness compilation, not a
hand-built namespace candidate routed through another branch.

The namespace rule is not a universal rejection of hands under harness or
open. Keep the existing boundary policy unchanged. A production edit to
`agents.rs` or `bundle.rs` is justified only if these required tests expose
a gap in the existing decisions; use the smallest repair and keep N0's text
verbatim. A new semantic rule would require a separate proposed decision
and an upstream return, not a downstream workaround in this slice.

### D4 — Inspect the actual composed launch, preserving the A1 distinction

Extend `crates/brokkr-runtime/src/engine/boundary_tests.rs`. Compile a minimal
namespace work seat against the real agent library and adapters; obtain both
engine-smith candidates and the recorded hands, then pass them through
production `compose_site`. Use a canonical temporary workdir and a result
path inside it. The principal regression must not fabricate a candidate or
copy a synthetic hands policy into the launch in place of the resolved one.
No provider process, network call or nested namespace is needed.

Use an independent expected argument structure with dynamic executable and
canonical path values supplied by the test. Decode MCP JSON/TOML arguments
and compare them to literal policy values from D2. Do not produce the expected
answer with `hands_command`, `serve_args`, `mcp_config` or another call to the
serializer being tested. Comparing the full relevant ordered argv makes a
second conflicting sandbox option or configuration override observable.

For Astra/Codex, establish all of these facts together:

- The concrete model is `gpt-6-astra`, effort high, with exactly the shipped
  `--sandbox read-only` selection and no competing writable sandbox setting.
- `mcp_servers.brokkr.command` is this engine executable; its decoded argument
  array is `hands serve --workdir <canonical-workdir> --spec <policy>`.
  The literal policy is network false with exactly the Cargo overlay, both
  credential masks and read-only Rustup bind. No workdir or bind is omitted.
- The shipped `mcp_servers.brokkr.default_tools_approval_mode="approve"`
  configuration remains present. The provider guide records the otherwise
  failing noninteractive workspace calls.
- No harness work fragment, `workspace-write`, per-tool list flag, retired
  Cargo/Git grant or unexpanded hands placeholder occurs anywhere in argv.

For Fable/Claude, pin `claude-fable-5-1` and high effort and preserve the
complete existing workspace fragment in order:

```text
--tools "" --strict-mcp-config --mcp-config <expanded-MCP-JSON>
--allowedTools mcp__brokkr__workspace
```

The empty argument is an actual empty argv element, not literal quote bytes.
Decode its MCP server and apply the same independent executable, command,
workdir and complete-policy expectations. Assert exactly one `--allowedTools`
with only the workspace grant, no `Bash(cargo:*)` or `Bash(git:*)`, no harness
fragment, and no unexpanded hands tokens.

This carries forward the answer to returned A1. P1 deliberately maps the
retired list through the same `--allowedTools` spelling used by the workspace
fragment: the proof distinguishes the source and value of the grant. A blanket
flag ban on Claude is rejected because it removes the required MCP permission;
a blanket exemption is rejected because it could hide restored Cargo/Git
grants. Codex retains the commission's literal absence-of-tool-list assertion.
Neither adapter needs editing to satisfy these expectations.

The H1 positive control compiles and composes under harness with exactly the
writable fragment and no namespace MCP registration. This demonstrates the
existing separate route and does not claim Brokkr enforces the hands policy
there. No guessed Claude harness fragment or mixed launch is introduced.

### D5 — Require independent removal evidence without new infrastructure

For each required compile and composition protection, record the targeted
test, exact temporary mutation, observed relevant assertion failure,
restoration and passing rerun. Keep that compact table with implementation
completion evidence, outside frozen fixtures. Each mutation is isolated;
restore it before another mutation, digest measurement or final gates. A
broken Rust build, unrelated refusal, zero selected tests, changed expected
answer or changed test fixture alone is not removal proof.

| Protection | Mutation whose effect the regression must detect |
| --- | --- |
| N0 no-hands refusal | Bypass its capability refusal so the otherwise valid seat compiles; separately changing the diagnostic breaks its exact comparison. |
| N1 / P1 hands precedence | Suppress workspace emission or consult the retired list. N1 detects the lost admission/fragment; P1 detects Cargo/Git grants on the capable provider. |
| N2 boxed capability requirement | Bypass the workspace guard. Missing support must become an unintended admission caught by the refusal test, including the resolver control retaining harness work. |
| H2 / H3 harness capability and whole-chain checks | Bypass the work requirement or check only the first link. Each applicable refusal test detects unintended admission. |
| H1 harness composition | Remove the work fragment; the positive launch assertion fails. |
| Namespace MCP and forwarded policy | Independently remove registration or corrupt the executable/serve arguments, workdir, network, bind membership, bind mode or either Cargo mask; the corresponding literal assertion fails. |
| Codex native sandbox and MCP approval | Remove/replace the read-only selection or approval configuration; the corresponding assertion fails. |
| Codex path separation and tool-list absence | Inject the harness work fragment or a tool-list flag; the absence/full-argv assertion fails. |
| Claude's complete workspace fragment and A1 | Remove/alter its required fragment, especially the MCP grant, or inject a retired Cargo/Git grant. Preservation and absence assertions fail independently; P1 also detects precedence loss. |

Use the existing Rust suites and temporary source/declaration mutations, not
new production machinery or permanent mutant files. Roster assertions over
the actual shipped agent complement these tests: removing its hands, changing
the chain/effort or restoring tools must break its declaration pin.

### D6 — Measure identity and document the applied ruling

Add a focused assertion in `tests/roster.rs` for the exact engine-smith chain,
efforts, hands and absence of tools, while retaining the existing declaration
and provider-diversity rules. Keep `tests/library_data.rs` resolution checks,
`every_shipped_panel_seats_at_least_two_providers`, `gpt_flash_shape`, and the
whole runtime suite green. The separate `gpt-flash-implementer-engine` office
is not the target. No charter change means no invented charter-digest update.

After all mutations are restored, compile the shipped bundles and recipes
under their existing namespace configuration. Re-measure every witness and
compose pin represented in `tests/witness_digests.rs` and
`src/bundle/compose_tests.rs`, including inherited uses and affected ancestor
identities. Use actual successful compile output/test-reported values;
unchanged pins stay unchanged. Both history blocks gain an issue #307 entry
attributing the movement to the Astra/Fable hire, effort pins and hands
replacing tools. Preserve earlier history rather than rewriting it. If a
whole-bundle harness pin encounters a different first refusal after the
roster change, record that actual site/reason while keeping focused H3 intact.

Update the existing Hands discussion in `docs/guides/provider-adapters.md`
to describe this restricted work seat on a provider without native per-tool
flags: namespace requires declared hands and workspace support; writes use
MCP hands while Codex's native sandbox is read-only. Explain the empty-root,
declared-bind and network boundary and explicitly deny command filtering.
Retain the limitation that Codex's native shell can read outside the box and
provider traffic is outside it. Preserve Claude's workspace grant and explain
the distinct unboxed harness requirement. Reconcile nearby current-tense
roster claims such as every hands agent chaining Opus when they conflict with
the new Fable fallback, without rewriting dated provider measurements.

Append a dated 2026-09-21 note to
`docs/decisions/0043-the-hands-are-one-tool.md`, recording issue #307's operator
answer and withdrawing the earlier allow-list-enforcement claim. Record that
namespace workspace hands and harness work remain separate and no composition
change or new per-tool contract is authorized. Preserve the decision's status
and historical rulings. This applies accepted semantics; no new semantic
decision is authored or accepted here.

### D7 — Keep required checks and live evidence distinct

Implementation evidence retains these gates:

- `cargo fmt --all -- --check`.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`.
- `cargo test -p <crate> --all-features --locked`, separately for
  `brokkr-core`, `brokkr-store`, `brokkr-protocol`, `brokkr-runtime`,
  `brokkr-bridge`, `brokkr-view` and `brokkr-cli`, including D6's roster checks.
- `cargo test --workspace --all-features --locked` and
  `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self`, plus
  the actual shipped compiles supporting D6's pins.
- `bash scripts/coverage-exact.sh`, retaining its literal 100% gate and pinned
  compiler with every added production line covered. No exclusion, rounding
  or lower threshold is introduced. Test-only source remains subject to the
  script's existing reporting convention, not a new coverage rule.
- `openspec validate --all --strict` and a clean `git diff --check`.

Namespace-dependent coverage requires host/CI execution outside this box;
unavailable execution or a skip remains pending evidence. Notes describe
observed results and residuals; they never instruct a gate to pass.

This authoring seat has no network and cannot run a live Astra smith. The
controller owns the first live measurement after landing: Cargo and Git
through Codex's boxed hands, a real commit, and verify passing. Deterministic
compile, composition and removal evidence establish that the ruling is
expressible, not proved by that live measurement. Nothing here upgrades the
adapter's resume qualification or asserts new installed-provider behavior.

## Risks / Trade-offs

- Offline dependency or toolchain gaps → Preserve network false and the
  existing Cargo overlay/Rustup grants; record failure rather than granting
  network or writable host caches. Overlay support below the existing
  minimum remains a refusal.
- The box runs arbitrary shell commands against writable project bytes →
  Describe filesystem/network scope honestly. The withdrawn command-list
  premise supplies no additional restriction to enforce.
- Codex retains native host reads and provider egress → Keep decision 0043's
  limitation visible in the guide and evidence; do not claim host-read
  secrecy or reframe argv checks as native-tool bypass prevention.
- Fable prevents this chain compiling under harness → Preserve the exact
  missing-work capability refusal and H3. Do not silently drop the fallback
  or invent a writable Claude fragment.
- Shared helpers or malformed fixtures could make the proof misleading →
  Use literal composition expectations, canonical paths, valid N2 fixtures,
  a separate internal resolver control, and observed removal failures.
- Pure compilation on macOS can succeed without a runnable namespace →
  Distinguish compiler/composition evidence from host availability and live
  enforcement. Existing boundary-start refusals remain authoritative.
- Missed transitive identity changes → Compile the complete shipped witness
  and compose sets after restoration; update measured pins and history only.

## Migration Plan

1. Add the reason-bearing compiler and composition regressions at D3/D4's
   existing seams and pin the actual shipped declaration. Apply D2's agent
   edit; change the compiler only against a demonstrated required failure.
2. Collect D5's independent removal evidence and restore each mutation.
   Recompile the affected shipped set and update both measured pin histories.
3. Apply D6's guide correction and dated decision note. Record D7's check
   results and any pending external coverage without manufacturing a pass.
4. Commit completed implementation artifacts in repository style; do not
   push. The controller's post-landing Cargo/Git/commit/verify measurement
   remains separate from preparation and deterministic expressibility.

There is no data, contract, schema, dependency or version migration. New
compiled identities flow through normal manifest drift checks; existing
in-flight runs are not silently repinned. Rollback is a coherent revert of
the declaration and corresponding measured pins/documentation, followed by
validation, not a model-only reversal that leaves mismatched identity or a
new bypass for an old run. Historical evidence remains historical.

## Open Questions

No unresolved operator or architectural question remains. The two triage
escalations and A1 are answered above and in the existing scenarios. Deferred
items are measurements only: actual moved digests, regression/removal results,
full local and external gate evidence, and the controller's first live smith.
They do not require changing this approach or the specification.

## Design validation — 2026-09-21

This phase adds only this Markdown design; it implements no roster, Rust,
adapter, guide or digest change. The production and removal tests
above are implementation obligations, not claimed executions in this visit.

The required Rust checks were attempted through workspace hands: format,
all-target/all-feature locked clippy with warnings denied, all seven crate
suites separately, the all-feature locked workspace suite, and compilation
of `bundles/self`. Each exited 127 because `cargo` is unavailable in this
seat's box. `bash scripts/coverage-exact.sh` also exited 127 at its first
Cargo command (line 33). These checks are unverified; no coverage, successful
compile, removal run or live Astra measurement is claimed by this design.

`openspec validate --all --strict --no-interactive` passed all 16 items with
zero failures. Existing informational archive/length diagnostics remain
outside this design's scope. OpenSpec reports proposal, specs and design done,
with tasks ready for their assigned phase. Whitespace validation is clean.
