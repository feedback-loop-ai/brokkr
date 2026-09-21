# Astra as the boxed engine smith

## Context

This design adopts `2026-09-21-307-astra-engine-smith` at `5ef97115` for issue
#307. The proposal and both capability deltas are authoritative and unchanged.
The design and return instructions were read from `dialects/openspec/design.md`
and `dialects/openspec/return.md`; `dialects/openspec.json` orders this artifact
before tasks. This is the sole architect's design. No workflow runner, build,
test, provider or nested namespace is needed to write it. The implementation
and evidence below are obligations, not executed results.

The source reading included README, decisions 0004, 0005, 0009, 0043, 0046 and
0063, the named agent, adapters, resolver, bundle compiler, engine and hands
protocol, their tests, recipe inheritance, both digest collections and the
provider guide. Relevant history includes the roster's introduction in
`f0d5e504`, strategy selection in `881c4e44`, runtime plumbing in `5ef4a842`,
and this change's A1 clarification in `5ef97115`.

The restriction is decision 0043's filesystem and network confinement, not a
`cargo,git` command restriction. `hands::execute_in` executes `/bin/bash -lc
<command>`; `HandsSpec` contains only `network` and `binds`. Codex retains a
native read-only shell outside the box; provider traffic also remains outside
it. Host-read secrecy and removal of that native shell are not established.

The current smith hires Fable then Opus, both high, with a Cargo/Git tool list.
Changing only the models to Astra/Fable is correctly refused. Existing code
already supplies the required alternative:

| Source | Existing behavior retained |
| --- | --- |
| `agents.rs::compose` | With hands and a boxed boundary, requires and appends the whole workspace fragment. Hands bypass `Agent.allow`. Without hands, an unsupported tool restriction is a capability error. |
| `report_under`, `entry_for`, `resolve_report` in that file | Resolves by `boundary.is_boxed()`, checks every chain entry, and carries `Candidate.argv`, `hands_fragment`, `harness`, effort and `Resolution.hands`. Pins the agent, charter and all consulted adapters. |
| `bundle.rs::resolve_reference` | Applies the boundary law to mapped hands chains before resolving capability errors, expands driver tokens and records each invocation site. |
| `enforce_model_policy`, `enforce_hands_boundary` | Boxed hands follow 0043; harness work requires `hands.harness.work` on every link. Work class does not waive resolver errors. Open, gates and inline exec retain their separate rules. |
| `record_hands`, `manifest_for` | Records hands in site facts, `Bundle.hands` and manifest hands/boundary maps. The realm owns the boundary. |
| `engine.rs::Engine::compose`, `compose_site`, `hands_command` | Namespace expands workspace tokens. Harness appends its class-specific fragment to an unboxed resolution. The engine delegates to the pure `compose_site` entry point. |
| `hands.rs::mcp_config`, `serve_args`, `HandsSpec::to_value`, `box_argv` | Registers this executable; forwards `hands serve --workdir <path> --spec <JSON>`; automatically binds the workdir read-write. Serialization includes `mask: []` on an unmasked bind. |

## Goals and non-goals

Seat Astra/Codex then Fable/Claude at high effort under namespace with the
same declared hands. Pin actual identity changes; preserve exact refusals;
prove both launch paths independently of a provider. Give each specification
requirement assertions, removal evidence and an honest evidence boundary.

No new per-tool contract, command parser, MCP transport, boundary, provider
capability, resume qualification, trust/egress grant, dependency, manifest
version or release. Preserve charter, limits, panels and GPT/Flash's offices.
Linux and macOS are the test hosts under 0063; pure namespace composition on
macOS does not establish namespace execution there. Seatbelt/container
activation remains outside scope. Frozen contracts, policy/schema/reference
and corpus bytes, `extensions/`, and issue #226's ledger remain untouched.

## Decisions

### D1 — Change the declaration and preserve the production rules

Implementation changes these fields in `agents/implementer-engine.json`:

```json
{
  "models": ["astra", "fable"],
  "efforts": {"astra": "high", "fable": "high"},
  "hands": {
    "kind": "workspace",
    "network": false,
    "binds": [
      {"path": "~/.cargo", "mode": "overlay", "mask": ["credentials.toml", "credentials"]},
      {"path": "~/.rustup", "mode": "ro"}
    ]
  }
}
```

This is a field-level excerpt. Remove `tools`, including its empty MCP list.
Preserve description, `charters/implementer.md`, `max_attempts: 2`,
`timeout_seconds: 7200`, and absent inputs/boundary overrides. No extra bind
names the workdir, a checkout path or another writable host directory. Use
the reviewer's established build-tool binds, not the release manager's
network and resolver/certificate grants.

No production Rust function or type needs a planned edit. Keep all functions
in the context table and `Agent.allow`/`hands`, `Adapter.hands`/`hands_gap`,
`HarnessHands`, `Candidate`, `Resolution`, `SiteFacts`, `HandsState` and
`HandsSpec` unchanged. Add tests in their existing modules. If tests expose a
gap, revise this design to name the smallest repair in the owning resolver,
admission or composition function before implementing it. It must restore
these assertions without altering 0043/0046 or the no-hands diagnostic. A
need for new semantics requires an upstream return and a proposed decision
for the operator; it is not an incidental repair.

Rejected: a Codex/work-class exception (waives a real capability gap), a fake
native tool map (unmeasured capability), deleting restriction without hands
(no declared box), and dead `tools` beside shipped hands (misleading and
forbidden by the roster test). Only temporary test agents carry both fields
to prove precedence. Accepted trade-off: commands are unrestricted inside the
existing box; offline caches and the existing overlay prerequisites are needed.

### D2 — Preserve complete workspace fragments and the source of each grant

Keep both shipped adapters byte-for-byte unchanged. Concrete hires are
`gpt-6-astra` and `claude-fable-5-1`. Codex retains `--sandbox read-only` and
all three MCP settings: command, args and
`default_tools_approval_mode="approve"`. Claude retains in order:

```text
--tools  <empty argument>  --strict-mcp-config  --mcp-config  <expanded JSON>
--allowedTools  mcp__brokkr__workspace
```

The empty argument is an argv element. Claude has exactly one
`--allowedTools`, granting only the workspace MCP tool. Neither provider gets
`Bash(cargo:*)` or `Bash(git:*)` anywhere in argv; Codex has no per-tool flag.
This is the settled A1 distinction between arguments generated from the
retired list and arguments supplied by the workspace declaration.

Reject filtering by flag spelling: Claude uses the same flag for both sources.
Reject testing only unsupported Codex: that would miss accidental native
grants on a capable provider. Accept separate capable-provider and shipped
Claude assertions, with full-fragment preservation and grant count/value checks.

### D3 — Compile and compose the boundary cases separately

Use `Bundle::compile_under` with namespace or harness explicitly. Never make
a harness candidate by stripping a namespace candidate's argv. Under harness,
`report_under` adds no workspace fragment, `enforce_hands_boundary` checks all
links, and `compose_site` appends `Candidate.harness.work`. Hands stay recorded
but their binds/network policy are unenforced by Brokkr. The model spawn
inherits the engine environment so its harness can reach the provider.

A Codex-only temporary agent is admitted under harness with `--sandbox
workspace-write`. The shipped smith refuses on link 2, Claude, which declares
no work fragment. Prove that with a minimal valid work seat: triage itself
can refuse first at an unrelated dialect step. Retain the existing open,
other-boundary and all-shipped harness tests as regression controls.

Reject borrowing harness work for namespace, combining fragments, skipping
Fable, inventing a Claude fragment or silently changing the realm. Accepted
trade-off: this two-provider smith is currently namespace-admissible and
harness-inadmissible. Pure tests run on both supported hosts without spawning
a provider; they establish neither a macOS box nor live harness behavior.

### D4 — Extend existing test seams with independent expectations

Planned implementation files beyond the declaration and documentation:

| File under `crates/brokkr-runtime/` | Change |
| --- | --- |
| `src/agents/tests.rs` | T4, capable-provider hands precedence. |
| `src/bundle/agent_tests.rs` | T1–T3, exact refusals and namespace admission; test-only extensions to `AgentFixture`. |
| `src/bundle/model_policy_tests.rs` | T5–T7 through existing `Fixture::compile_roots`/`compile_bounded`. |
| `src/engine/boundary_tests.rs` | T8–T10 through actual compiled candidates and `compose_site`. |
| `tests/roster.rs` | R1, exact shipped declaration; retain all existing invariants. |
| `tests/witness_digests.rs`, `src/bundle/compose_tests.rs` | D5's measured pins, appended history and affected-site/unchanged-ancestor assertions. |

No new public helper or production abstraction is needed. Fixtures use the
existing valid two-seat pattern with explicit `class: work` on `work` and an
unrelated valid inline work-class control seat. New Codex data declares
provider/binary `codex`, driver `{brokkr} driver codex --`, Astra mapped to
`gpt-6-astra`, valid model/effort flags and high effort, unsupported native
tools and MCP, and no requested MCP needs or secrets. Agent `worker` declares
Astra high, a valid charter and `tools.allow: ["cargo", "git"]`. T2 adds D1's
hands while retaining that list. Independently specify the full workspace
fragment and harness work fragment in the fixture. Loaders must succeed before
asserting a capability refusal; vary only the capability under test.

Loader details constrain the fixtures: `agents/load.rs` requires a `workspace`
array when `hands` is a supported object, so a JSON object containing only
`harness.work` is malformed, not a valid missing-workspace capability case.
Do not change that loader or count that error as T3 evidence. Top-level hands
may be absent, the string `"unsupported"`, or a measured unsupported object.
By contrast `harness_hands` accepts a work array, absent work, or an object
`{"unsupported":"<reason>"}`; the bare string `"unsupported"` is not a legal
work member. T6 uses the absent and object forms. T2/T8 prove namespace never
borrows the writable work fragment even when the valid adapter declares both;
T5/T10 prove harness never composes the simultaneously declared workspace.

Use temporary directories, never frozen `fixtures/`. Canonicalize each created
root before deriving paths; canonicalize the result parent then append a new
filename, not a nonexistent result file. Pass explicit roots and boundaries;
do not change process cwd, PATH or HOME. No provider/network/nested namespace,
installed-adapter mutation or Windows-specific work is needed.

T8/T9 compile a minimal seat referencing the actual shipped smith against
actual shipped agents/adapters under namespace. Extract both candidates from
`SeatBody::Single`, assert their order, and call `compose_site` with
`BuiltBoundary::Namespace`, `SeatClass::Work`, that candidate's argv and
reference, `bundle.hands.get("work")`, a canonical temporary workdir,
`bundle.roots`, the seat's own result path and no unboxed-exec preparation.
This is the production composition path used by `Engine::compose`.

Do not use synthetic `candidate()`/`CODEX_FRAGMENT` helpers or derive expected
output from `hands_command`, `serve_args`, `mcp_config`, the actual adapter
fragment or `HandsSpec::to_value`. The existing
`compose_site_follows_the_boundary_and_the_class` helper-to-helper comparison
stays, but is insufficient for these new claims.

Use `current_exe()` as the independent expected engine path. Assert full argv
length/order with serialized config tokens checked separately after decoding.
For these temporary paths, the emitted TOML basic-string array is also valid
JSON, so `serde_json::from_str::<Vec<String>>` decodes Codex's server args
without adding a dependency. Decode the inner spec JSON independently. Decode
Claude's MCP JSON and require exactly one Brokkr server, this executable, and
exactly `hands serve --workdir <canonical workdir> --spec <policy>`.
The literal expected policy is:

```json
{"kind":"workspace","network":false,"binds":[{"path":"~/.cargo","mode":"overlay","mask":["credentials.toml","credentials"]},{"path":"~/.rustup","mode":"ro","mask":[]}]}
```

Compare complete structured JSON, including array order and no extra keys.
The forwarded tilde paths remain literal; runtime `box_argv` expands them.
No separate result-path argument belongs in `hands serve`; namespace uses
the existing driver result contract, not the harness last-message door.

Codex's expected prefix is executable, `driver codex -- --model gpt-6-astra
--effort high`, then D2's exact workspace tokens. Claude's is executable,
`driver claude -- --permission-mode acceptEdits --model claude-fable-5-1
--effort high`, then its exact fragment. Assert `SpawnEnv::Inherit` and no
exec rewalk. Search every argument for unexpanded `{brokkr}`, workspace and
result placeholders, retired grants and foreign boundary tokens. Codex has
exactly one read-only sandbox pair, no workspace-write/native tool-list flag.
Claude has exactly one MCP grant. Full-vector comparison detects extra flags.

### D5 — Measure identities; do not infer movement from ancestry alone

`resolve_report` changes the smith's `agent_digest`, chain, primary model and
provider, and aggregate consulted-adapter digest (now Codex as well as Claude).
The charter and individual adapter digests, limits, notices and chosen index
zero stay unchanged. High efforts are part of agent identity; do not invent
an effort field in the resolution record. `record_hands` and `manifest_for`
add hands and boundary for `recipes/triage`'s `implement:engine` site.

Expected inventory at this revision:

| Pin or manifest | Expected result |
| --- | --- |
| `WITNESSES["recipes/triage"]` in `tests/witness_digests.rs` | Moves from `d95b41d920e0ca5db3012a4eae51449d16505733c4530a4eb83445dff36f336f` because the smith resolution and its hands/boundary identity change. |
| Triage literal in `bundle/compose_tests.rs::a_composed_bundles_manifest_is_pinned` | Moves from the same old hash to the same measured new hash. Append issue #307 history beside this pin. |
| Triage's `@compose/0000/fast` | Unchanged: no fast recipe bytes or engine version change. |
| `recipes/night-shift` and its triage/fast ancestors | Unchanged: it replaces the entire implement seat with inline dsh. |
| `recipes/gpt-flash` and its triage/fast ancestors | Unchanged: it replaces the entire implement seat with scoped offices, including `gpt-flash-implementer-engine`. |
| Other nine `WITNESSES` entries and all four `UNCOMPOSED` entries | Unchanged, including `bundles/self` (hires `implementer`, not this smith) and `bundles/verify`. |

`bundle/compose.rs::resolve` builds ancestor manifests with no agent records,
no driver records and empty hands. They hash the ancestor's recipe files and
its own ancestors, not a standalone compilation of the ancestor's resolved
roster. An agent edit outside the recipe therefore does not move a descendant
that replaces every use. No affected ancestor digest is predicted here.
Editing triage's recipe bytes would be different and is not planned.

Implementation enumerates and compiles all 18 current bundle/recipe roots,
as `every_bundle_in_the_tree_compiles` does, recording before/after manifests
and digests with identical roots, dialect and namespace boundary. Capture the
baseline before the declaration edit and the result after all planned edits.
A measurement pass must report all roots, not stop at the first pin mismatch.
Update only actual changed hashes wherever represented in both collections.
No guessed new hash belongs in this design. Unexpected movement requires a
manifest diff and resolution of its cause before repinning; newly added
inherited users, if any, must be accounted for from their actual compiles.

Append histories in both collections naming issue #307, Astra/Fable, both high
efforts, and hands replacing the tool list; keep old history. Strengthen the
compose test to assert triage's engine chain/hands/namespace and that
night-shift/GPT-Flash do not resolve the ordinary smith. Check their unchanged
witness hashes and ancestor entries; an ancestor hash is not triage's
standalone manifest hash. No version bump, schema change or blanket refresh.
Accepted trade-off: active hires get new identity; old runs retain their
pinned strategy and cannot silently adopt it.

### D6 — Document accepted semantics and bounded evidence

Implementation updates `docs/guides/provider-adapters.md`'s Hands and harness
discussion and appends a `2026-09-21 — issue #307 operator ruling` note to
`docs/decisions/0043-the-hands-are-one-tool.md`. Keep accepted status and all
historical text. This applies 0043/0046; it asserts no new semantic decision.

Explain native permissions versus the workspace grant, the actual smith
policy, namespace's read-only native Codex sandbox/MCP route, and the separate
unboxed harness work route with its whole-chain rule. Keep Codex's native
shell/host-read limitation, provider traffic outside the box and the shipped
MCP approval setting. Explicitly withdraw the earlier premise of a Cargo/Git
command restriction. Correct current guide explanations that all hands agents
chain Opus or that this smith's grant forbids other commands. Preserve dated
provider measurements and unrelated resume evidence as historical facts.

The dated note records the operator's existing-semantics ruling and withdrawal
of the earlier enforcement claim; no command parsing, transport semantics or
native-tool bypass prevention is owed. Reject a new ADR solely for this data
change. A discovered need for new semantics follows D1's upstream/proposed
ADR rule rather than changing the accepted decision's status.

The delivery record states actual compiles, inspected launches, observed and
restored removals and residuals; it never instructs a gate to pass. First live
Astra implementation remains the controller's post-landing measurement: Cargo
and Git through Codex workspace hands, a real commit and verify passing.
Mocks, parser acceptance, composition and removal tests cannot discharge it.

## Requirement-to-proof plan

The following are intended new test names, not existing or executed results.
Negative cases assert `CompileError::Invalid` and its reason-bearing display,
never just `is_err()`. Positive cases use an explanatory `expect` and compare
concrete compiled facts. The prefix R means roster, T means a new regression,
I denotes retained suites and E denotes evidence review.

| ID / test and owner | Requirement/scenarios and assertions |
| --- | --- |
| R1 `engine_smith_declares_astra_fable_with_workspace_hands` (`tests/roster.rs`) | Astra spec's ruled chain, project access declaration and tools removal. Exact D1 fields; no extra permission, bind or boundary; preserved description, charter and limits. |
| T1 `codex_work_tool_list_without_hands_keeps_exact_refusal` (`bundle/agent_tests.rs`) | Admission spec's no-hands refusal. Namespace Codex worker alone and with valid capable Fable fallback; full exact diagnostic below, including measured native-gap variant. Work class and a later candidate cannot excuse it. |
| T2 `codex_namespace_hands_replace_the_same_tool_list` (same module) | Adding hands admits the same tool-listed fixture. Assert provider/model/high, full workspace fragment, no native list flag, `bundle.hands["work"]` and manifest hands/namespace. |
| T3 `codex_namespace_requires_declared_workspace_hands` (same module) | Missing-workspace refusal. Loader-valid variants: absent hands, bare unsupported and measured unsupported object. Assert exact capability diagnostic and supplied reason; no capability is inferred from the work class or boundary. |
| T4 `capable_provider_hands_keep_only_the_workspace_grant` (`agents/tests.rs`) | Both specs' A1 precedence. Claude-shaped fixture maps Cargo/Git natively with `--allowedTools`, while its full workspace fragment uses that same flag for MCP. Resolve a hands-plus-tools agent; compare full fragment and `Candidate.hands_fragment`, one MCP-valued flag and no retired grant substring. Also call `compose_site` and independently check the complete expanded fragment/server policy. |
| T5 `codex_harness_work_admits_its_declared_fragment` (`bundle/model_policy_tests.rs`) | Codex-only harness admission. Same hands/tools and adapter policy as T2, compiled under harness. Assert base candidate argv, empty workspace fragment, retained hands and manifest harness. T10 proves its launch. |
| T6 `codex_harness_work_gap_preserves_its_reason` (same module) | Workspace support remains but work is absent or a measured unsupported object. Assert full harness-work diagnostic, link 1 Codex and supplied reason. |
| T7 `shipped_engine_smith_harness_refuses_fable_link_two` (same module) | Compile a minimal seat using actual shipped smith/adapters under harness. Assert exact link 2 Claude work-gap refusal. Temporary Codex-only agent against shipped adapters is the positive control; no unrelated dialect or gate refusal counts. |
| T8 `shipped_engine_smith_codex_namespace_launch_is_complete` (`engine/boundary_tests.rs`) | Both specs' shipped hire/workdir and complete Codex launch. Actual two-link smith, Astra model/high, command/args/approval settings, canonical workdir, exact binds/masks/network, read-only sandbox, no harness work/list flag/placeholders; all D4 assertions. |
| T9 `shipped_engine_smith_claude_namespace_launch_is_complete` (same module) | Both specs' A1 and complete Fable launch. Exact prefix and whole ordered workspace fragment, decoded server and policy, one MCP grant, no retired grants or foreign/unexpanded tokens; all D4 assertions. |
| T10 `codex_harness_work_launch_excludes_workspace_hands` (same module) | Successfully harness-compiled Codex-only fixture, not a stripped namespace candidate. Exact executable/driver/model/high prefix then `--sandbox workspace-write`; no MCP registration, workspace token, read-only sandbox, list flag or last-message gate flag. Retained hands/harness manifest. |
| I1 existing `pinned_bundles_keep_their_recorded_digest`, `every_witness_manifest_satisfies_the_v9_contract_it_claims`, `every_bundle_in_the_tree_compiles`, `a_composed_bundles_manifest_is_pinned`, `recipes_that_opted_into_nothing_keep_their_digests` | Astra spec's actual identities and inherited/independent uses. D5 supplies measured inventory and affected-site/ancestor assertions. |
| I2 existing `every_shipped_panel_seats_at_least_two_providers`, `tool_grants_keep_house_tools_explicit_and_effort_never_rises_on_fallback`, `a_codex_lane_is_chained_only_into_boxed_or_toolless_offices`, `library_data`, all `gpt_flash_shape`, entire runtime suite | Astra spec's independent rosters and runtime invariants. No dead-tools exception, diversity reduction or GPT/Flash substitution. Existing all-shipped harness and open/other-boundary tests stay. |
| I3 existing protocol `the_namespace_is_built_from_an_empty_root_and_binds_what_the_spec_names` and real namespace/Git tests | Astra spec's automatic writable project mount: no duplicate bind needed. T8/T9 establish the forwarded workdir/policy; protocol tests retain responsibility for actual confinement. |
| Documentation diff review | All three Astra guide/decision scenarios: D6 claims match code; dated note is append-only; accepted status and historical rulings remain. No new prose-string test is needed. |
| E1 implementation evidence record | Both live-evidence scenarios and every removal scenario: actual quality results, observed assertion failures/restorations and pending controller measurement. |

R1 checks exact raw declaration fields. The existing
`library_data::the_library_holds_the_decision_0041_roster` also preserves the
shared implementer charter and its digest. T8/T9 explicitly assert both
candidate facts and compiled hands/boundary before inspecting argv. Neither
an empty success nor a mere count of binds proves the declared policy.

The exact T1 bare-unsupported display is one line:

```text
bundle: seat 'work': agent 'worker' cannot be served by provider 'codex' on model 'astra': the provider declares tool_permissions unsupported, so the agent's restriction to ["cargo", "git"] cannot be expressed and the agent would run with MORE power than it declares. A capability the provider cannot express fails compilation here rather than degrading silently at run time
```

The measured T1 reason is `fixture Codex has sandbox classes but no per-tool
flag`. Its sole display change is parentheses immediately after
`tool_permissions unsupported`. Preserve the trailing capability explanation
byte-for-byte. The Fable fallback fixture must map its model and high effort
validly, avoiding another refusal.

T3 uses that same seat/agent/provider/model prefix and trailing explanation,
with this capability reason:

```text
the provider declares hands unsupported, so the agent's hands cannot be put in the box and the agent would run with the harness's own tools
```

The measured variant inserts ` (fixture Codex has no workspace bridge)` after
`hands unsupported`. Compare full reason/envelope, not loose words that a
malformed-fixture error could also contain.

T6's absent-work display is exactly:

```text
bundle: seat 'work' link 1 resolves to provider 'codex', which declares no `hands.harness.work` fragment: a capability gap — under the `harness` boundary a work seat with hands writes the tree only under the harness's own writable sandbox as the adapter addresses it (decision 0046 rulings 1 and 4)
```

The measured variant inserts ` (fixture Codex exposes no writable sandbox)`
immediately after `fragment`. T7 changes link 1 to link 2 and `codex` to
`claude`; all other absent-fragment wording stays. Do not invent agent/model
wording in the harness error: the existing law names seat, link and provider.

## Removal experiments and evidence protocol

In implementation, first run each targeted test passing. Apply one temporary
mutation to the production protection or shipped production declaration, keep
the fixtures and assertions unchanged, run the test and observe an assertion
failure caused by that mutation. Restore exact bytes and rerun passing. Name
parameterized subcases and record each relevant variant. A compile error,
unrelated capability refusal, changed fixture alone, changed expectation,
zero-test run or comment saying a test would fail is not removal evidence.
Do not commit mutants or weaken admission/coverage rules to make tests pass.

Append the implementation evidence to this owning design, recording for each
row/variant: revision, precise mutation/diff, command and executed test count,
exit status, relevant assertion excerpt, restoration check and passing rerun.
An unavailable experiment stays pending. The experiments below are separate;
a broad mutation cannot substitute for unrelated assertions.

| Removal | Target and expected relevant failure |
| --- | --- |
| M1: `agents::compose` skips the native allow-list branch when the native permission map is absent | T1 wrongly compiles, alone and with fallback; the exact-refusal expectations fail. |
| M2: remove hands-over-tools precedence in `compose` so the original native list is consulted on Codex | T2's success expectation fails with the specific native tool-permission refusal hands should replace. |
| M3: replace `compose`'s missing workspace error with an accepted empty fragment | Each T3 variant wrongly compiles: absent, bare unsupported and measured unsupported. |
| M4: suppress supplied measured reasons in resolver error formatting; separately change the capability-error explanation | T1/T3 measured full-message comparisons fail on lost reasons; independent envelope comparisons fail on changed explanation. One field at a time. |
| M5: also generate native permissions after the hands fragment on capable adapters | T4 still resolves but gains Cargo/Git grants and a second `--allowedTools`; its count/value/absence assertions fail. Unsupported Codex cannot prove this case. |
| M6: filter the workspace `--allowedTools` pair from the fragment appended by `compose` | T4's full-fragment/MCP-presence assertions fail even though no retired grant appears. Restore separately from M5. |
| M7: make `enforce_hands_boundary` refuse work even when its fragment is declared | T5's admission expectation fails with the harness-work reason; this proves positive admission, separately from fragment forwarding. |
| M8: bypass the missing-work guard in `enforce_hands_boundary` | All T6 variants wrongly compile. Separately suppress `measured(&harness.work_gap)` to break the measured-reason comparison while refusal remains. |
| M9: check only the first candidate in the harness work-chain law | T7 wrongly compiles although Claude lacks work support. Its link 2 refusal assertion fails; the Codex-only control stays passing. |
| M10: drop `harness.work` forwarding in `compose_site` | T10 compiles but lacks workspace-write; exact argv fails. Separately force boxed resolution under harness in `report_under`: T10 gets workspace argv and fails its exclusion assertions. |
| M11: remove each shipped MCP registration element separately | Remove Codex's production workspace command setting, then args setting: T8 fails exact registration/argv. Remove Claude's production MCP-config pair: T9 fails. These are production-declaration mutations, not changed temporary fixtures. |
| M12: mutate `hands::serve_args` forwarding one coordinate at a time | Both T8 and T9 independently fail when workdir is replaced, network becomes true, either bind is dropped, a mode changes, either Cargo mask is lost or an extra bind is added. Change only forwarded policy; leave the actual agent valid. Also alter the `hands`/`serve` and workdir/spec option tokens to prove the server argv assertions. |
| M13: replace Codex's production workspace read-only with workspace-write; separately remove its approval setting | T8 fails its exact sandbox assertion, then its approval assertion. Restore each independently. |
| M14: append `Candidate.harness.work` during namespace `compose_site` | T8 detects the extra writable sandbox. Claude's shipped work fragment is absent: separately append a foreign `--sandbox workspace-write` pair in namespace composition to make T9's full-vector/exclusion check fail. No guessed fragment enters the final adapter. |
| M15: append a native tool-list flag/grant to namespace composition | Append `--allowedTools` plus a Cargo/Git grant temporarily: T8 fails its no-list assertion. Independently append each retired grant to Claude output: T9 fails no-retired-grant/one-MCP-grant checks. Prove Cargo and Git absence separately. |
| M16: remove or alter each element of Claude's shipped workspace fragment, one at a time | T9 fails independently for missing/changed `--tools`, empty argument, strict flag, MCP option/config, permission flag or workspace grant. MCP payload contents also have M11/M12. Removing only the MCP grant must fail presence, not merely leave no Cargo/Git grants. |
| M17: suppress each workspace placeholder expansion in `hands_command`, independently | T8/T9 fail the corresponding no-placeholder or decoded-config assertion. The independent registration and policy comparisons remain required. |
| M18: omit `Resolution.hands`; separately omit compiler hands recording or manifest boundary recording | T2 and shipped compilation assertions in T8/T9 fail explicit hands/namespace facts. T5 separately fails retained-hands/harness facts. |

R1 has production-declaration controls too: undo the chain/efforts edit,
reintroduce tools, drop a bind/mask, grant network or change a retained limit,
and observe the corresponding assertion fail. These controls supplement
M1–M18. For identity proof, leave the old triage hash in each collection once
after the actual agent edit and observe both pin assertions report the new
measured digest; restore the correct pin and rerun. Unchanged witnesses are
the control against blanket repinning. Existing real protocol namespace/Git
tests retain their enforcement responsibility. None is live-provider proof.

## Risks and accepted trade-offs

- Offline Cargo depends on toolchains/dependencies already available through
  permitted read-only/overlay inputs. A missing cache is a runtime limitation,
  not authority to grant network or writable host Cargo access.
- Codex's native shell can read host material. Network false applies to hands,
  not provider traffic. Keep that accepted 0043 limit explicit in evidence.
- The Fable fallback currently makes the shipped chain inadmissible under
  harness; preserve refusal until a separately measured fragment exists.
- Exact argv tests intentionally detect adapter drift. Independent literals
  duplicate the relevant contract; structured JSON comparison avoids coupling
  to insignificant serialization formatting.
- Removal experiments cost effort but discriminate workspace grants from
  retired grants and demonstrate the claimed protections. Keep helpers small
  and test-only; do not add production abstractions for test convenience.
- Digest predictions are source reasoning until measured. Resolve unexpected
  movement before changing pins; retain history and unaffected hashes.
- No resume qualification follows. Existing Codex declarations exclude boxed
  workspace hands; namespace admission does not qualify that coordinate.

## Migration and verification

No persisted schema or database migration. During implementation land the
agent, tests and measured pins coherently, with guide and append-only note.
Existing runs keep pinned identity; a newly compiled bundle/run selects the
new hire. Do not rewrite historical manifests, adapter measurements, journal
examples or release/channel facts. Rollback reverses declaration, new-hire
assertions, matching pins and current explanatory prose coherently, not by
waiving the previous tool-list refusal.

After implementation and mutant restoration, collect this quality evidence
on the exact candidate. These commands are for that phase, not this design:

- `cargo fmt --all -- --check` and
  `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`.
- Each crate separately with `cargo test -p <crate> --all-features --locked`:
  `brokkr-core`, `brokkr-store`, `brokkr-protocol`, `brokkr-runtime`,
  `brokkr-view`, `brokkr-bridge`, `brokkr-cli`; then `cargo test --workspace`
  and `cargo test --workspace --all-features --locked`.
- `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self`, the
  all-shipped compile/digest inventory, and the suites/removals above.
- Direct strict OpenSpec validation for this change and all changes, plus
  a diff check for frozen-byte or unrelated edits. Do not invoke the runner.
- `bash scripts/coverage-exact.sh` on host/CI outside the workspace box, using
  `rust-nightly-version.txt` and required real boundary evidence. Preserve
  literal nonzero 100% source-line, branch and function equality, including
  any necessary new production line. Namespace skips and missing external
  results are pending evidence, never passes or a reduced threshold.

Run the new deterministic tests on Linux and macOS with canonical roots.
Real namespace enforcement runs on a capable Linux host; macOS composition
cannot substitute. Keep all runtime regressions for other boundaries, gates,
secrets, fallback and resume. No release build, tag, publication, profile
change or remote CI claim is part of this design. Any later release handoff
must keep final-head remote results and publication pending until observed.

## Open questions and evidence state

No semantic ambiguity remains: chain, confinement, path separation and A1's
grant distinction are settled and implementable against inspected code. No
upstream specification defect was identified.

Pending measurements are the new digest/inventory, all implementation tests,
removals and quality checks (including external exact coverage), and the
controller's first live Astra Cargo/Git/commit/verify implementation after
landing. Their record must state actual status and evidence. Deterministic
success establishes expressibility only. This phase commits only this design;
the mandatory engine result is written separately to its assigned path.

Design validation on 2026-09-21: direct
`openspec validate 2026-09-21-307-astra-engine-smith --strict --no-interactive`
passed; `openspec validate --all --strict --no-interactive` passed all 16
items with zero failures. Existing informational archive diagnostics for
other changes are outside this slice. No Cargo command, build, Rust test,
removal experiment or live provider was run in this design seat, as instructed.
