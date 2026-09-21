## Purpose

Make the engine smith's use of existing provider capability and boundary rules
checkable through reason-bearing compile tests and deterministic launch
composition, without changing decisions 0043 or 0046.

## ADDED Requirements

### Requirement: A tool-listed work seat without hands retains its exact refusal

For an otherwise valid work-class agent declaring `tools.allow` and no hands,
a provider declaring `tool_permissions` unsupported SHALL still refuse
compilation with the existing diagnostic, word for word. A work class,
namespace realm or available later candidate SHALL NOT bypass that refusal.
The regression SHALL use a temporary fixture Codex adapter, explicitly name
the seat, agent, provider and model, and compare the reason text rather than
using `is_err()` or a generic failure assertion. The fixture SHALL NOT depend
on a live provider or mutate the frozen `fixtures/` tree.

#### Scenario: Codex cannot express a cargo and git list without hands
- **GIVEN** a work agent listing `cargo` and `git`, no hands, and an otherwise valid Codex fixture with `tool_permissions: "unsupported"`
- **WHEN** it compiles in a namespace realm
- **THEN** the refusal identifies the seat, agent, provider `codex` and model, and preserves this reason exactly: `the provider declares tool_permissions unsupported, so the agent's restriction to ["cargo", "git"] cannot be expressed and the agent would run with MORE power than it declares`
- **AND** the existing capability-error explanation remains intact; a measured unsupported declaration retains its supplied reason in the existing diagnostic shape

### Requirement: Namespace hands replace the tool list only through declared workspace support

For the otherwise valid fixture above under namespace, the restriction SHALL
be expressible if and only if the agent declares hands and the provider
declares `hands.workspace`. Adding hands SHALL replace the allow-list on both
Codex and providers capable of expressing that list. Replacement SHALL
suppress arguments generated from `tools.allow`, not permission flags already
declared by `hands.workspace` to grant access to the workspace MCP tool.
The complete workspace fragment SHALL remain intact after placeholder
expansion. A provider lacking workspace hands SHALL refuse rather than
dropping the hands policy or borrowing `hands.harness.work`. Existing resolution that satisfies this rule SHALL remain
unchanged; any necessary repair SHALL preserve existing boundary semantics
and the no-hands diagnostic.

#### Scenario: The same tool-listed fixture gains hands and compiles
- **GIVEN** the same Codex fixture and work agent as the no-hands refusal, retaining its tool list
- **WHEN** the agent gains workspace hands and the adapter declares `hands.workspace`, with namespace still selected
- **THEN** compilation succeeds and records those hands and namespace, with the Codex workspace fragment present and no tool-list flag
- **AND** the test asserts the resulting candidate and hands facts with an explanatory success expectation, not just an absence of an error

#### Scenario: Declared hands cannot use an unsupported workspace adapter
- **GIVEN** the hands-declaring fixture agent, namespace boundary and an otherwise valid Codex adapter with hands absent or declared unsupported
- **WHEN** the work seat compiles
- **THEN** it refuses naming the seat, agent, provider and model, with `the provider declares hands unsupported` and `so the agent's hands cannot be put in the box and the agent would run with the harness's own tools` in the existing reason text
- **AND** a supplied measured hands-gap reason is preserved; this is a capability refusal, not a malformed-fixture or unrelated tier error

#### Scenario: A fixture distinguishes the MCP grant from retired tool-list grants (A1)
- **GIVEN** a valid test-only agent carrying the same hands and `tools.allow: ["cargo", "git"]`, and a Claude-shaped provider fixture whose native permissions map that list to `--allowedTools Bash(cargo:*),Bash(git:*)` while its workspace fragment includes `--allowedTools mcp__brokkr__workspace`
- **WHEN** it resolves under namespace
- **THEN** the entire workspace fragment is preserved and `--allowedTools` occurs exactly once with the argument `mcp__brokkr__workspace`
- **AND** no argument contains `Bash(cargo:*)` or `Bash(git:*)`; flag spelling alone does not distinguish the two sources, and hands precedence does not remove the MCP grant

### Requirement: Harness work is a separate case governed by the existing whole-chain rule

A hands-declaring work agent under a harness realm SHALL require
`hands.harness.work` on every chain link under existing 0046 rules. The
namespace workspace fragment SHALL NOT be composed there, and admission SHALL
NOT imply that Brokkr enforces the declared binds or network policy. Conversely,
namespace composition SHALL NOT append the writable harness fragment. This
slice SHALL preserve the existing open and other-boundary behavior.

#### Scenario: The same Codex-only hands agent is admitted by its harness work fragment
- **GIVEN** the test-only hands agent and Codex adapter declaring both workspace hands and `hands.harness.work: ["--sandbox", "workspace-write"]`
- **WHEN** the otherwise identical work seat compiles under harness and its launch is composed
- **THEN** it compiles under the existing rule and its launch carries `--sandbox workspace-write`, no workspace MCP server and no per-tool allow-list flag
- **AND** its boundary remains harness and its hands policy stays recorded but unenforced by Brokkr

#### Scenario: Missing harness work remains a reason-bearing refusal
- **GIVEN** the same fixture with its workspace fragment intact but `hands.harness.work` absent or explicitly unsupported
- **WHEN** the hands-declaring work seat compiles under harness
- **THEN** it refuses naming the seat, chain link, provider, `hands.harness.work`, and the existing reason that work with hands writes only under the harness's own writable sandbox, citing decision 0046 rulings 1 and 4
- **AND** any measured unsupported reason appears in that diagnostic; workspace support does not satisfy it

#### Scenario: The shipped Fable fallback keeps the whole chain honest
- **GIVEN** a minimal work seat using the shipped engine smith and adapters, avoiding unrelated earlier bundle refusals
- **WHEN** it compiles under harness with the current Claude declaration lacking `hands.harness.work`
- **THEN** it refuses on link 2, provider `claude`, naming `hands.harness.work` and the existing writable-sandbox reason even though the Astra/Codex link can compile alone
- **AND** no guessed Claude fragment, fallback omission or combined launch is introduced to make it pass

### Requirement: The shipped engine smith's namespace launch is tested as composed

Deterministic regressions SHALL load the actual shipped engine-smith agent
and Codex and Claude adapters, resolve the work seat under namespace and
inspect each candidate's launch produced by the production composition path.
Independent expectations SHALL establish the complete relevant argv and
server policy rather than compare a helper to itself. The proof SHALL be runnable without a provider,
network or nested namespace and SHALL canonicalize temporary workdirs.

#### Scenario: The complete Codex namespace launch exposes exactly the declared hands route
- **WHEN** the shipped engine smith's Astra candidate is composed for a canonical temporary workdir and its own result path under namespace
- **THEN** the model and effort are Astra's concrete Codex model and high, and the launch registers `mcp_servers.brokkr` with the engine executable and `hands serve` arguments
- **AND** the decoded server arguments carry the canonical workdir and the complete declared hands spec, including network false, both toolchain binds and both Cargo masks
- **AND** the native sandbox is exactly `--sandbox read-only` from `hands.workspace`, and the shipped MCP approval configuration remains present
- **AND** no `workspace-write`, harness work fragment, per-tool list flag or unexpanded hands placeholder appears anywhere in that launch

#### Scenario: The shipped Claude launch preserves its complete workspace fragment (A1)
- **WHEN** the shipped engine smith's Fable candidate is composed for a canonical temporary workdir and its own result path under namespace
- **THEN** the model and effort are Fable's concrete Claude model and high, and the complete existing workspace fragment appears in order: `--tools`, an empty argument, `--strict-mcp-config`, `--mcp-config`, the expanded hands MCP JSON, `--allowedTools`, `mcp__brokkr__workspace`
- **AND** the decoded MCP JSON registers the Brokkr server with the engine executable and `hands serve` arguments carrying the canonical workdir and complete declared hands policy, including network false, both toolchain binds and both Cargo masks
- **AND** `--allowedTools` occurs exactly once and grants only `mcp__brokkr__workspace`; no argument contains `Bash(cargo:*)` or `Bash(git:*)`
- **AND** no harness work fragment or unexpanded hands placeholder appears; preserving this shipped fragment requires no adapter edit

### Requirement: Every claimed compile and composition protection has removal evidence

Each required regression SHALL have evidence that removing its particular
protection makes its assertion fail for the claimed reason, followed by a
restored passing run. A compile failure from a broken mutation, an unrelated
refusal, a changed fixture alone or a comment saying a test would fail SHALL
NOT count. Rejected inputs SHALL assert their diagnostic reason, never merely
`is_err()`. Positive tests SHALL assert concrete compiled or composed facts.
No production rule SHALL be weakened to improve coverage or admit the roster.

#### Scenario: Removal proves each compile case independently
- **WHEN** the evidence for the no-hands refusal, namespace admission, missing-workspace refusal and both harness admission/refusal outcomes is reviewed
- **THEN** each names the removed protection, targeted test, observed relevant assertion failure, restoration and passing rerun
- **AND** it covers the whole-chain missing-fallback refusal and the hands-over-tools precedence, including the capable-provider fixture: consulting the retired list introduces Cargo/Git grants and fails their absence assertions, while losing the existing workspace MCP grant fails its presence assertion

#### Scenario: Removal proves the launch claims independently
- **WHEN** the Codex and Claude namespace launch evidence is reviewed
- **THEN** removing either MCP registration or losing or altering its forwarded workdir/binds/network policy breaks the corresponding assertion, as does replacing Codex's read-only native sandbox
- **AND** introducing a harness work fragment or any tool-list flag on Codex, or a retired Cargo/Git grant on Claude, breaks the corresponding absence assertion
- **AND** removing or altering any part of Claude's required workspace fragment, including its `--allowedTools mcp__brokkr__workspace` grant, breaks its preservation assertion; every mutation is restored and the targeted test passes again
- **AND** these observations are described as composition evidence, not proof of live Codex behavior or a new native-tool enforcement contract
