## Context

Issue #307 changes one shipped work office, `implementer-engine`, from a
Claude-only `fable -> opus` chain with `tools.allow = ["cargo", "git"]` to
the operator-ruled `astra -> fable` chain under the workspace-hands policy
already accepted by decisions 0043 and 0046. The specification has already
settled the two questions that could otherwise change the architecture:

- the restriction is the existing empty-root filesystem and network boundary,
  not a command allow-list; and
- `hands.workspace` under a namespace boundary and
  `hands.harness.work` under a harness boundary are separate alternatives,
  not two fragments of one launch.

The current code already implements those rules:

- `agents.rs::compose` checks `agent.hands` before `agent.allow`. Under a
  boxed boundary it requires and appends the adapter's workspace fragment;
  it does not consult the tool list. With no hands it still enters the
  `tool_permissions` branch and emits the existing capability refusal.
- `agents.rs::report_under` passes `boundary.is_boxed()` into that composition,
  and `agents.rs::resolve_report` refuses a capability gap on every mapped
  chain entry, not only the chosen entry.
- `bundle.rs::resolve_reference` preserves the seat label around resolver
  diagnostics. `bundle.rs::enforce_hands_boundary` returns immediately for a
  boxed boundary, but under `harness` looks up the class-selected adapter
  `HarnessHands` for every candidate link. The same declaration is carried in
  `Candidate::harness` for the engine. Its work refusal already
  names the seat, link, provider, optional measured reason, and decisions 0046
  rulings 1 and 4.
- `engine.rs::compose_site` sends namespace sites to `hands_command`; under
  `harness` it appends only `candidate.harness.work` for a work seat; under
  `open` it appends neither. `hands_command` expands the workspace fragment
  with the current executable, workdir, and serialized hands policy.
- `brokkr_protocol::hands::HandsSpec` already represents exactly the policy
  needed here: `network` plus extra binds. `hands::serve_args` carries the
  writable workdir separately from the policy, so the agent must not repeat
  the checkout as an `rw` bind.
- `adapters/codex.json` already has the complete workspace fragment for Astra:
  native `--sandbox read-only`, the Brokkr MCP command and arguments, and the
  approval setting. Its `hands.harness.work` is separately
  `--sandbox workspace-write`.
- `adapters/claude.json` already has the complete workspace fragment for
  Fable: built-in tools disabled, strict MCP configuration, the expanded MCP
  server, and exactly one `--allowedTools mcp__brokkr__workspace` grant. It
  has no `hands.harness.work` declaration.

The shipped `agents/implementer-engine.json` is therefore the inconsistency:
it still names `fable -> opus`, carries the now-inapplicable Cargo/Git tool
list, and has no hands. `recipes/triage` is the only shipped source that hires
that office. `recipes/night-shift` and `recipes/gpt-flash` extend triage but
replace the complete `implement` seat, so neither resolved leaf uses
`implementer-engine`. Composition ancestor digests hash recipe-layer files and
explicitly exclude leaf agent resolution; changing the agent therefore does
not move their `@compose/0000/triage` entries.

This is an enactment of accepted semantics, not a new semantic rule. It does
not require a new decision document, contract version, protocol field, adapter
capability, dependency, or Rust production branch. Decision 0043 receives a
dated application note, retaining status `accepted`; no proposed decision is
created. Decision 0009 keeps all implementation and tests in Rust. Decision
0063 limits host obligations to Linux and macOS; there is no native-Windows
branch, fixture, or evidence obligation.

## Goals / Non-Goals

### Goals

- Make the shipped engine smith resolve, in order, to Astra on Codex and Fable
  on Claude, both at high effort.
- Give that office the same network-off Cargo-overlay/Rustup-read-only hands
  policy already used by boxed Rust work, without duplicating the automatic
  writable workspace mount.
- Preserve the exact no-hands/tool-list refusal for a provider that cannot
  express the list.
- Prove that hands replace a retained fixture tool list only when the selected
  boundary and provider declaration support the corresponding route.
- Prove complete, independently expected namespace launches for both shipped
  candidates, including Claude's workspace MCP grant and the absence of
  retired Cargo/Git grants.
- Prove the harness path independently, including the shipped chain's refusal
  at Fable/Claude link 2.
- Re-measure only the compiled identities that actually move and explain the
  movement in both pin histories.
- Document the confinement actually supplied and preserve an honest boundary
  between deterministic composition evidence and the controller's first live
  Astra-smith measurement.

### Non-Goals

- No command parser or `cargo,git` allow-list is added to workspace hands.
- No Codex per-tool permission contract is invented, and
  `tool_permissions.unsupported` is not weakened or remapped to a sandbox
  class.
- No adapter JSON is changed. In particular, Claude does not acquire a guessed
  `hands.harness.work` fragment, and its workspace MCP grant is not removed.
- No change is made to `HandsSpec`, `serve_args`, the MCP transport, namespace
  construction, harness composition, open-boundary behavior, or provider
  resume declarations.
- No live provider call is made or claimed. Compilation and argv inspection
  prove expressibility, not live Astra behavior.
- No release version, dependency, frozen contract, policy schema,
  `policy/phase-machine.json`, fixture corpus, or `reference/` byte changes.
- No Windows-specific code or test is added.

## Decisions

### D1 — Change only the engine-smith declaration; preserve its office

`agents/implementer-engine.json` will retain its description, charter and
limits byte-for-byte in meaning, and will make these exact data changes:

```json
{
  "models": ["astra", "fable"],
  "efforts": {"astra": "high", "fable": "high"},
  "hands": {
    "kind": "workspace",
    "network": false,
    "binds": [
      {
        "path": "~/.cargo",
        "mode": "overlay",
        "mask": ["credentials.toml", "credentials"]
      },
      {
        "path": "~/.rustup",
        "mode": "ro"
      }
    ]
  }
}
```

The existing `tools` member is removed. No `boundary` member is added: the
realm owns that choice. No `rw` checkout bind is added: `serve_args` carries
the canonical workdir and the hands server mounts it read-write. No network or
machine-specific path is added. The Cargo overlay remains writable only in its
seat-private upper layer and masks both credential spellings; Rustup is
read-only.

`crates/brokkr-runtime/tests/library_data.rs` gains
`the_shipped_engine_smith_hires_astra_then_fable_through_workspace_hands`.
It loads the shipped data and asserts the complete ordered model and effort
maps, concrete provider/model candidates, `allow == None`, empty MCP needs,
the exact `HandsSpec`, unchanged charter, unchanged limits, no inputs, and the
absence of raw `tools` and `boundary` keys. It also asserts that the only bind
paths are `~/.cargo` and `~/.rustup`; this is the machine-path and duplicate
workspace-bind guard.

`crates/brokkr-runtime/tests/adoption.rs::triage_cases_resolve_to_the_roster`
will add `implement:engine` to `TRIAGE` with concrete model `gpt-6-astra` and
the existing implementer charter digest. `expected_argv` will classify only
that implement site as hands-bearing and expect the complete current Codex
workspace fragment rather than a Cargo/Git allow-list. The other triage cases,
including the GPT/Flash recipe's separately scoped engine smith, remain
unchanged.

`crates/brokkr-runtime/tests/roster.rs::is_house_tool_grant` will remove the
now-retired `implementer-engine` Cargo/Git exemption. The existing
`tool_grants_keep_house_tools_explicit_and_effort_never_rises_on_fallback`
test already refuses `tools` beside hands; the new shipped-data test prevents
the office from evading that rule by losing hands and regaining its old list.

Accepted trade-off: the active declaration no longer preserves Cargo/Git as
descriptive metadata. Keeping it would be misleading because
`agents.rs::compose` deliberately ignores it when hands exist. Historical
commits, the pin-history note, and the dated decision note preserve why it was
removed.

### D2 — Keep resolver behavior unchanged and prove both sides of the branch

No production line in `crates/brokkr-runtime/src/agents.rs` is designed to
change. The order in `compose` is the required behavior:

1. When `agent.hands.is_some()` and the boundary is boxed, require
   `adapter.hands`, append that complete fragment, and do not consult
   `agent.allow`.
2. When hands are absent and `agent.allow` is present, require
   `adapter.tool_permissions` and map every name.
3. Under an unboxed boundary, an agent with hands receives neither the
   workspace fragment nor the old allow-list; the bundle law decides whether
   that boundary admits the site.

New temporary-fixture tests belong in
`crates/brokkr-runtime/src/bundle/model_policy_tests.rs`, because compilation
there preserves both the seat name from `resolve_reference` and the provider
capability text from `ResolveError::Capability`.
Every temporary root is canonicalized before it is passed to the compiler or
launch composer, covering Linux and macOS path aliases without introducing a
native-Windows case.

`a_tool_list_without_hands_still_refuses_unsupported_provider_verbatim` uses
a work-class seat named `work`, agent `listed-smith`, first model `astra`, a
Codex fixture with `tool_permissions: "unsupported"`, and a later capable
Claude candidate. Namespace compilation must fail on the first link rather
than skip it. The complete expected diagnostic is:

```text
seat 'work': agent 'listed-smith' cannot be served by provider 'codex' on model 'astra': the provider declares tool_permissions unsupported, so the agent's restriction to ["cargo", "git"] cannot be expressed and the agent would run with MORE power than it declares. A capability the provider cannot express fails compilation here rather than degrading silently at run time
```

The same test repeats with an object-form unsupported declaration and compares
the complete message including the literal measured reason `codex fixture
restricts by sandbox class, not tool name` in parentheses after
`tool_permissions unsupported`. This pins the reason-bearing shape rather
than merely checking `is_err()`.

`namespace_hands_replace_the_tool_list_and_require_workspace_support` keeps
the fixture's `tools.allow` and adds the exact smith hands. With Codex and
Claude fixture workspace fragments, namespace compilation must succeed and
assert the candidate order, providers, concrete models, efforts, manifest
boundary, recorded hands, and complete fragments. The Claude-shaped fixture
deliberately uses `--allowedTools` both for native tool-list mapping and inside
`hands.workspace`; the resolved argv must contain the flag exactly once,
followed by `mcp__brokkr__workspace`, and contain neither `Bash(cargo:*)` nor
`Bash(git:*)`. This makes source, not flag spelling, the tested distinction.

The test then uses otherwise valid adapters with workspace hands absent and
with `{"unsupported":"codex fixture has no workspace MCP configuration"}`.
Each compile must compare the
complete capability diagnostic, including:

```text
the provider declares hands unsupported, so the agent's hands cannot be put in the box and the agent would run with the harness's own tools
```

and, in the measured case, the supplied reason in parentheses after
`hands unsupported`: `codex fixture has no workspace MCP configuration`.
It must still name seat `work`, agent `listed-smith`, provider `codex`,
and model `astra`.

Accepted trade-off: exact diagnostic tests are intentionally sensitive to
wording. Here the diagnostic is part of the requirement and operator-facing
repair path, so that brittleness is the protection rather than test noise.

### D3 — Keep namespace and harness launches mutually exclusive

The two paths remain:

```text
namespace
  report_under(boxed=true)
    -> Candidate.argv + adapter hands.workspace
    -> compose_site(Namespace)
    -> hands_command
    -> provider native read-only fragment + Brokkr MCP hands server

harness
  report_under(boxed=false)
    -> Candidate.argv without hands.workspace
    -> enforce_hands_boundary checks every link's hands.harness.work
    -> compose_site(Harness)
    -> base argv + only that link's writable harness fragment
```

`harness_work_is_separate_and_requires_every_chain_link` in
`bundle/model_policy_tests.rs` uses the same one-link Codex fixture with both
workspace support and `hands.harness.work = ["--sandbox",
"workspace-write"]`. Under `harness`, compilation must record the hands and
boundary while the candidate has an empty `hands_fragment`. Calling the
production `engine::compose_site` as a work seat must yield the base argv plus
exactly `--sandbox workspace-write`, with no workspace MCP server and no
tool-list flag. Removing or declaring the work member unsupported must compare
the existing reason-bearing refusal, including:

```text
seat 'work' link 1 resolves to provider 'codex', which declares no `hands.harness.work` fragment: a capability gap — under the `harness` boundary a work seat with hands writes the tree only under the harness's own writable sandbox as the adapter addresses it (decision 0046 rulings 1 and 4)
```

The measured variant must include its supplied reason immediately after
`fragment` and before `: a capability gap`. That fixture reason is the
literal `codex fixture has no writable harness class`, so the assertion
cannot pass by finding an unrelated parenthetical.

`the_shipped_engine_smith_refuses_harness_at_the_fable_fallback` compiles a
minimal temporary work seat using the shipped agent and shipped adapters under
`harness`. It must refuse on `link 2`, provider `claude`, name
`hands.harness.work`, and compare the same writable-sandbox reason through
`(decision 0046 rulings 1 and 4)`. It also asserts that the refusal does not
name link 1 as missing: Codex's existing work fragment is sufficient only for
its own link. No Claude fragment is guessed to make the bundle pass.

The existing
`the_gate_law_reads_the_boundary_for_sites_that_declare_hands` and
`compose_site_follows_the_boundary_and_the_class` tests remain the regression
pins for `open`, `seatbelt`, `container`, gate-class harness composition,
and sites without hands. No production edit is expected in
`bundle.rs::enforce_hands_boundary`, `engine.rs::compose_site`, or
`engine.rs::hands_command`.

Accepted trade-off: a harness realm cannot run the shipped engine-smith chain
today even though Astra/Codex alone has a writable harness fragment. The
whole-chain refusal is safer than silently deleting Fable or treating
workspace MCP support as a harness sandbox. On macOS this remains an explicit
unboxed-path limitation until Claude's work fragment is measured; this change
does not turn that missing measurement into a declaration.

### D4 — Inspect the actual shipped namespace launches, not helper echoes

`crates/brokkr-runtime/src/engine/boundary_tests.rs` gains
`the_shipped_engine_smith_namespace_launches_are_exact`. It loads the actual
agent and both actual adapters, resolves under `Boundary::Namespace`, uses a
canonicalized `tempfile` directory as the workdir, gives each candidate its
own result path, and calls production `compose_site` with
`BuiltBoundary::Namespace` and `SeatClass::Work`.

The expectations are independently written literals plus decoded data; the
test must not build its expected argv by calling `hands_command`,
`hands::serve_args`, or `hands::mcp_config`.

For Astra/Codex it asserts:

- abstract model `astra`, concrete model `gpt-6-astra`, provider `codex`, and
  effort `high`;
- the complete argv prefix and workspace fragment in order, including native
  `--sandbox read-only`, and all three MCP `-c` entries, including
  `mcp_servers.brokkr.default_tools_approval_mode="approve"`;
- the current engine executable as `mcp_servers.brokkr.command`;
- the JSON-decoded `mcp_servers.brokkr.args` array is exactly `hands`,
  `serve`, `--workdir`, the canonical temporary workdir, `--spec`, and one
  JSON hands object;
- that decoded hands object is exactly network false, the Cargo overlay with
  both masks, and read-only Rustup;
- no `workspace-write`, `--allowedTools`, Cargo/Git grant, harness token, or
  unexpanded `{hands_*}` placeholder occurs.

For Fable/Claude it asserts:

- abstract model `fable`, concrete model `claude-fable-5-1`, provider
  `claude`, and effort `high`;
- the complete existing workspace tail, in order:
  `--tools`, empty string, `--strict-mcp-config`, `--mcp-config`, expanded
  MCP JSON, `--allowedTools`, `mcp__brokkr__workspace`;
- the decoded MCP JSON has exactly the `brokkr` server, the current engine
  executable, the canonical workdir, and the same complete decoded hands
  object;
- `--allowedTools` occurs exactly once and grants only the workspace MCP
  tool; no argument contains `Bash(cargo:*)` or `Bash(git:*)`;
- no `workspace-write`, harness fragment, or unexpanded placeholder occurs.

The workdir check proves where writable project access comes from. The bind
check proves only the two extra toolchain paths are declared. Together with
the raw-agent assertion in D1, this excludes a checkout-specific path or a
second writable host bind without executing a provider or nesting a
namespace.

Accepted trade-off: the test pins provider CLI argv closely. That is necessary
because losing Claude's MCP grant while retaining the same flag name elsewhere
is precisely the subtle regression at issue. Adapter interface changes must
update this test with new measurement rather than silently relaxing it.

### D5 — Re-pin only triage, and explain both copies of the pin

Changing the agent JSON changes its canonical `agent_digest`, its resolved
chain and adapter digest set, the chosen provider/model, and the manifest's
hands/boundary facts for `implement:engine`. Therefore the following two
stored digests move to the same newly measured value:

- `crates/brokkr-runtime/tests/witness_digests.rs`, the `recipes/triage`
  member of `WITNESSES`; and
- `crates/brokkr-runtime/src/bundle/compose_tests.rs`, the explicit
  `triage.manifest_digest()` expectation in
  `a_composed_bundles_manifest_is_pinned`.

Both history blocks will append issue #307's reason: the engine case changes
from Fable/Opus plus a Cargo/Git list to Astra/Fable at high effort plus the
network-off workspace hands and toolchain binds. Old historical reasons stay
in place.

No other witness or compose digest is expected to move:

- `recipes/night-shift` replaces the whole `implement` seat with its inline
  DSH seat;
- `recipes/gpt-flash` replaces the whole `implement` seat with its scoped
  GPT/Flash office;
- an ancestor digest in `bundle::compose::resolve` covers layer files and
  ancestor digests, not the leaf compilation's agent records; and
- no other shipped bundle source references `implementer-engine`.

Implementation must first leave the old two triage pins in place and capture
their reported actual value from real compilation, then replace both with that
identical measured value. It must not precompute or guess a hash. All other
pins remain unchanged, so `pinned_bundles_keep_their_recorded_digest`,
`recipes_that_opted_into_nothing_keep_their_digests`, and the full runtime
suite are negative evidence against broad re-pinning.

The existing `every_shipped_panel_seats_at_least_two_providers` test proves
panel diversity remains intact. The complete `gpt_flash_shape` integration
test proves the separate scoped engine smith and the GPT/Flash roster are not
substituted. `every_shipped_agent_resolves_at_compile_time` and
`triage_cases_resolve_to_the_roster` prove the changed standard office without
weakening the dead-tools rule.

### D6 — Documentation records existing semantics and an evidence boundary

`docs/guides/provider-adapters.md`, in its existing Hands section, will add a
focused engine-smith example. It will state all of the following together:

- a provider with no native per-tool list can serve this seat because the
  seat declares workspace hands;
- under `namespace`, writable commands go through the MCP workspace server,
  whose filesystem/network policy is the restriction;
- this is not a `cargo,git` command allow-list;
- Codex's native shell remains present and read-only outside the box, so
  host-read secrecy is not promised, and provider traffic remains outside the
  box in the harness;
- Claude's `--allowedTools mcp__brokkr__workspace` is the existing grant that
  reaches the boxed tool, not a resurrection of the retired agent tool list;
  and
- `hands.harness.work` is the separate unboxed route and is required on every
  work-chain link.

`docs/decisions/0043-the-hands-are-one-tool.md` will append, not rewrite, a
dated `2026-09-21` issue #307 operator note. The top-level status remains
`accepted`. The note records that the earlier commission's command-list
enforcement premise is withdrawn, this smith applies decisions 0043 and 0046
as they already stand, the workspace and harness routes remain separate, and
no new command parsing, per-tool enforcement, transport semantics, or native
tool bypass contract is accepted.

`crates/brokkr-runtime/tests/roster.rs` gains
`the_engine_smith_docs_name_the_existing_confinement_and_evidence_limit`.
It reads both shipped documents and asserts the accepted status, dated issue
note, filesystem/network-not-command-list statement, Codex native-shell and
provider-traffic limitations, the distinct `hands.harness.work` route, and the
pending controller measurement. Removing any of those required clauses must
fail a named assertion rather than leave documentation coverage to a visual
skim.

The implementation result notes are the delivery record for evidence that is
not repository state. They must distinguish:

- deterministic compile/composition results obtained here; from
- the still-pending controller observation of Astra performing Cargo and Git
  through workspace hands, producing a real commit, and passing verify.

No deterministic test, mock, or argv inspection may be described as that live
measurement. Missing commands or external results are recorded as pending,
never converted to pass instructions.

### D7 — Production Rust and frozen surfaces remain unchanged

The source inspection above settles that no production repair is necessary.
The following are read by this design but must not change in implementation:

- `agents.rs::compose`, `report_under`, `resolve_report`, `Candidate`, and
  `Resolution`;
- `bundle.rs::resolve_reference`, `enforce_model_policy`,
  `enforce_hands_boundary`, `record_hands`, and manifest construction;
- `engine.rs::compose`, `compose_site`, and `hands_command`;
- `hands.rs::HandsSpec`, `mcp_config`, and `serve_args`;
- `adapters/codex.json` and `adapters/claude.json`;
- all contracts, frozen fixtures, `policy/phase-machine.json`,
  `policy/schemas/`, and `reference/`.

If a specified regression cannot pass against these existing seams, that is a
design discrepancy to return, not authority to add a per-tool mechanism,
merge the two boundary paths, or alter a frozen contract. A smallest Rust
repair is permissible only after the design is revised to name the concrete
gap; none is designed now.

Rejected alternatives:

- Mapping Codex `workspace-write` to `tool_permissions` was rejected because a
  sandbox class does not express the declared tool names and would invalidate
  the exact no-hands refusal.
- Keeping `tools` beside hands was rejected because it is inactive data and
  the shipped roster explicitly refuses dead grants.
- Removing Claude's `--allowedTools` flag was rejected because that occurrence
  grants the sole MCP workspace tool and is part of its existing workspace
  fragment.
- Adding `hands.harness.work` to Claude was rejected because no measured
  declaration supports it and whole-chain admission must fail closed.
- Appending both workspace and harness fragments was rejected because it
  combines a Brokkr namespace with an unboxed provider sandbox and contradicts
  decision 0046's boundary axis.
- Adding an Astra/engine-smith match arm was rejected because providers,
  models, efforts and fragments are data in this repository.
- Extending `HandsSpec` with commands was rejected because it creates the new
  enforcement contract the operator explicitly declined.
- Re-pinning triage descendants was rejected because their complete implement
  overrides remove this agent from the resolved leaf, and ancestor composition
  identity does not include leaf agent resolution.

## Requirement Proof and Removal Evidence

The implementation will use valid, compiling mutants of production code or
shipped data. For every row, it will run the named targeted test against the
mutant, record the relevant assertion failure, restore the mutation, and rerun
the same test successfully. A malformed source edit, unrelated compile error,
fixture-only change, or comment is not removal evidence. The mutation ledger
belongs in the implementation result notes; no extra committed evidence file
is introduced by this change.

| Requirement / scenario | Passing proof and asserted facts | Removal proof |
|---|---|---|
| Engine smith resolves both hires | `the_shipped_engine_smith_hires_astra_then_fable_through_workspace_hands`; `triage_cases_resolve_to_the_roster` | R1 removes or reorders a shipped model/effort; the exact chain or Astra argv assertion fails. Restore and pass. |
| Writable project access uses the existing mount | `the_shipped_engine_smith_namespace_launches_are_exact` decodes the canonical workdir separately from the two exact extra binds; the shipped-data test excludes `boundary`, checkout paths, extra `rw`, and network | R2 changes the forwarded workdir or adds an `rw` checkout bind; the decoded workdir/spec or raw-source assertion fails. |
| Inactive tools removed, workspace grant retained | Shipped-data and adoption tests assert no `tools`; launch test asserts no Codex tool flag and exactly one Claude MCP grant with no Cargo/Git grant | R3 restores shipped `tools`, removes Claude's MCP grant, or adds a Cargo/Git grant; the corresponding presence/absence assertion fails. |
| Changed hires/hands are witnessed | `pinned_bundles_keep_their_recorded_digest` and `a_composed_bundles_manifest_is_pinned` agree on the measured triage digest and both history comments name #307 | R4 restores either old triage pin after the data edit; that exact digest assertion reports actual versus expected. |
| Independent rosters retain guarantees | `every_shipped_panel_seats_at_least_two_providers`, the entire `gpt_flash_shape` target, `every_shipped_agent_resolves_at_compile_time`, `tool_grants_keep_house_tools_explicit_and_effort_never_rises_on_fallback`, and the runtime suite | R5 temporarily points the scoped GPT/Flash engine case at the standard office or restores dead tools; the scoped-roster or dead-tools assertion fails. |
| Allow-list escalation is answered | `the_engine_smith_docs_name_the_existing_confinement_and_evidence_limit` asserts filesystem/network boundary, “not a command allow-list”, and no new per-tool contract | R6 removes each sentence independently; its named documentation assertion fails. |
| Namespace/harness composition is not combined | The same documentation test plus both boundary tests asserts namespace MCP/read-only versus harness writable/no-MCP | R7 makes `compose_site` append the harness work fragment under namespace or the workspace fragment under harness; exact argv and forbidden-token assertions fail. |
| Decision note preserves accepted semantics | Documentation test asserts the `accepted` status, dated #307 note, decisions 0043/0046 application, and absence of a newly accepted rule | R8 changes the status or removes the application statement; its exact documentation assertion fails. |
| Deterministic evidence is not live proof | Documentation test asserts the pending Cargo/Git/commit/verify controller measurement; implementation result notes label it pending | R9 removes the pending-measurement clause; documentation test fails. Result review refuses notes that claim a live run without controller evidence. |
| Required quality evidence | The validation set below, with unavailable external checks explicitly pending | No protection is removed for this bookkeeping scenario; a nonzero or absent result remains non-passing and is recorded verbatim. |

The provider-admission and launch scenarios are proved separately so one
passing branch cannot mask another:

| Requirement / scenario | Passing proof and asserted reason/facts | Removal proof |
|---|---|---|
| Tool-listed no-hands refusal | `a_tool_list_without_hands_still_refuses_unsupported_provider_verbatim` compares the complete bare and measured diagnostics, including seat, agent, provider, model, exact restriction text, and capability suffix | R10 changes `compose` to admit an absent `tool_permissions`; the test fails because compilation succeeds instead of producing the exact reason. |
| Same fixture gains namespace hands | `namespace_hands_replace_the_tool_list_and_require_workspace_support` asserts both concrete candidates, hands, namespace and complete fragments | R11 makes `compose` continue into tool-list mapping after hands; Codex refuses or the capable fixture gains Cargo/Git grants, failing the concrete success/absence assertions. |
| Missing workspace support refuses | The same test compares bare and measured hands refusals word-for-word | R12 treats absent `adapter.hands` as an empty fragment; the test fails because compilation succeeds rather than returning the named reason. |
| MCP grant differs from retired grants | The capable-provider arm counts one `--allowedTools`, checks the following MCP value, and excludes both Bash grants | R11 consults the retired list and fails the Bash absence; R13 removes the workspace fragment's MCP grant and fails its presence/count. |
| Harness admission uses work fragment | `harness_work_is_separate_and_requires_every_chain_link` asserts compiled boundary/hands and exact `--sandbox workspace-write` launch with no MCP/tool-list tokens | R14 removes the `compose_site` work-fragment append; the exact launch tail fails while compilation still succeeds. |
| Missing harness work refuses | The same test compares link-1 bare and measured existing writable-sandbox reasons | R15 bypasses the work-member arm in `enforce_hands_boundary`; compilation unexpectedly succeeds. |
| Shipped Fable fallback keeps the chain honest | `the_shipped_engine_smith_refuses_harness_at_the_fable_fallback` compares link 2, `claude`, `hands.harness.work`, and the complete 0046 reason | R15 makes the minimal shipped seat compile; the expected refusal fails. Removing Fable instead fails the shipped chain test R1. |
| Complete Codex namespace launch | `the_shipped_engine_smith_namespace_launches_are_exact` asserts full argv, read-only native sandbox, executable, decoded workdir/spec and approval setting, plus forbidden tokens | R16 independently removes MCP command registration, changes workdir, changes a bind/network value, replaces read-only, or injects workspace-write/tool-list text; each mutation reaches a specific equality or absence failure. |
| Complete Claude namespace launch | The same test asserts the full ordered workspace fragment, decoded MCP server and spec, one MCP grant, and no retired/harness tokens | R17 independently removes each fragment token (including the MCP grant), alters forwarded policy, or adds a Cargo/Git grant; the exact vector, decoded data, count, or absence assertion fails. |
| Compile protections have removal evidence | R10–R15, each with targeted failure, restoration, and passing rerun recorded | Review rejects a ledger entry whose mutant did not compile, changed only a fixture, or failed for another refusal. |
| Launch protections have removal evidence | R2, R3, R7, R13, R14, R16 and R17, each independently restored and rerun | Review rejects helper-to-itself comparisons and any entry described as live-provider enforcement. |

The two exact diagnostics deliberately remain owned by their current functions:
the no-hands reason by `agents.rs::compose`, wrapped with the seat by
`bundle.rs::resolve_reference`; and the harness-work reason by
`bundle.rs::enforce_hands_boundary`. Tests must not reconstruct a substitute
error in a new helper.

## Risks / Trade-offs

- Codex keeps a native read-only shell outside the Brokkr namespace. The MCP
  path confines writes and its own reads, but this change does not promise
  host-read secrecy or prevent provider egress. The guide and decision note
  make that residual explicit.
- Workspace hands permit arbitrary shell commands inside the declared
  filesystem/network boundary. This is deliberately broader in command names
  and narrower in reachable state than the retired Cargo/Git list.
- The Claude fallback is namespace-capable but not harness-work-capable as
  shipped. Whole-chain compilation under `harness` therefore refuses. This is
  an accepted availability cost of fail-closed fallback admission.
- Namespace execution with the Cargo overlay still requires bubblewrap 0.10+
  on Linux. macOS deterministic tests can compile and inspect the launch, but
  this slice neither builds Seatbelt nor invents a Claude harness declaration.
- Tight argv and documentation assertions require deliberate updates when a
  measured provider interface or wording changes. That maintenance cost buys
  protection against losing one token from a security-relevant fragment.
- The triage manifest digest changes, so an in-flight run pinned to the old
  engine-case identity cannot silently continue under the new hire. It must be
  recompiled/restarted through the existing manifest-drift behavior.
- Deterministic proof can establish composition but not that a live Astra
  invocation chooses the MCP tool for Cargo and Git. That remains controller
  evidence after landing.

## Migration Plan

1. Change only `agents/implementer-engine.json` as D1 specifies. Add the
   shipped-data/adoption assertions while leaving digest pins old so the
   identity movement is visible.
2. Add the temporary compile fixtures in `bundle/model_policy_tests.rs` and
   compare every rejected case to its reason-bearing diagnostic. No frozen
   fixture is used or regenerated.
3. Add the actual shipped launch regression in `engine/boundary_tests.rs`,
   canonicalizing temporary roots on both supported hosts and decoding the
   server arguments independently.
4. Amend the guide and append the dated decision 0043 note; keep the status
   and historical text intact. Add the focused documentation regression.
5. Compile all shipped witnesses, confirm that only `recipes/triage` reports a
   movement, place its measured value in both pin locations, and append the
   same #307 explanation to both history blocks. Any additional moved pin is
   investigated rather than bulk-updated.
6. Execute R1–R17 as valid temporary mutants, one protection at a time. Record
   the targeted assertion failure, restore the tree, rerun the target, and
   finish with a clean diff containing no mutant.
7. Run the implementation validation set:

   - `cargo fmt --all -- --check`
   - `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
   - separately, `cargo test --locked --all-features -p` each of
     `brokkr-core`, `brokkr-store`, `brokkr-protocol`, `brokkr-runtime`,
     `brokkr-view`, `brokkr-bridge`, and `brokkr-cli`
   - `cargo test --workspace --all-features --locked`
   - `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self`
   - `cargo run --locked -p brokkr-cli -- compile --bundle bundles/verify`
   - `cargo build --release --locked -p brokkr-cli`
   - `openspec validate 2026-09-21-307-astra-engine-smith --strict --no-interactive`
   - `openspec validate --all --strict --no-interactive`
   - `bash scripts/coverage-exact.sh` on the CI/host environment that can
     create the required namespace, retaining literal 100% coverage

   The implementation record names the host for Linux/macOS runs. It records
   namespace skips, unavailable commands, external exact coverage, final-head
   remote CI, publication, channels and live-profile verification as pending
   until their actual results exist; none is inferred from a local pass.
8. Confirm that `.github/workflows/ci.yml`,
   `.github/workflows/release.yml`, and `scripts/coverage-exact.sh` all
   consume `rust-nightly-version.txt`; no workflow/compiler-pin edit is
   expected.
9. Compare the implementation range against `contracts/`, `fixtures/`,
   `policy/phase-machine.json`, `policy/schemas/`, and `reference/`; the
   expected diff is empty. No package version or release note is part of this
   change, so witness movement comes only from the agent declaration, not an
   engine-version bump.

There is no data migration. New compiles receive the new manifest identity;
old journal records and historical “live from” versions remain historical
facts and are not rewritten.

## Open Questions

None. The operator has settled the chain, restriction shape, workspace-only
boxed route, separate harness route, unchanged no-hands refusal, and supported
hosts. The controller's first live Astra Cargo/Git/commit/verify observation,
external exact coverage, and final-head remote CI are pending evidence, not
design ambiguities.
