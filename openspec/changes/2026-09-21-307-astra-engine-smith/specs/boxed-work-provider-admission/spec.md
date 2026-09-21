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
Codex and providers capable of expressing that list. A provider lacking
workspace hands SHALL refuse rather than dropping the hands policy or borrowing
`hands.harness.work`. Existing resolution that satisfies this rule SHALL remain
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

#### Scenario: A fixture keeps both fields to prove replacement on a capable provider
- **GIVEN** a valid test-only agent carrying the same hands and tool list, and a provider fixture with both workspace hands and native per-tool permissions
- **WHEN** it resolves under namespace
- **THEN** the workspace fragment is present and the per-tool permission flag is absent, proving the same precedence used by Codex without retaining dead tools in the shipped agent

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

A deterministic regression SHALL load the actual shipped engine-smith agent
and Codex adapter, resolve the work seat under namespace and inspect the
launch produced by the production composition path. Independent expectations
SHALL establish the complete relevant argv and server policy rather than
compare a helper to itself. The proof SHALL be runnable without a provider,
network or nested namespace and SHALL canonicalize temporary workdirs.

#### Scenario: The complete namespace launch exposes exactly the declared hands route
- **WHEN** the shipped engine smith's Astra candidate is composed for a canonical temporary workdir and its own result path under namespace
- **THEN** the model and effort are Astra's concrete Codex model and high, and the launch registers `mcp_servers.brokkr` with the engine executable and `hands serve` arguments
- **AND** the decoded server arguments carry the canonical workdir and the complete declared hands spec, including network false, both toolchain binds and both Cargo masks
- **AND** the native sandbox is exactly `--sandbox read-only` from `hands.workspace`, and the shipped MCP approval configuration remains present
- **AND** no `workspace-write`, harness work fragment, per-tool list flag or unexpanded hands placeholder appears anywhere in that launch

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
- **AND** it covers the whole-chain missing-fallback refusal and the hands-over-tools precedence, including the capable-provider fixture

#### Scenario: Removal proves the launch claims independently
- **WHEN** the namespace launch evidence is reviewed
- **THEN** removing the MCP registration, losing or altering the forwarded workdir/binds/network policy, or replacing the read-only native sandbox each breaks its corresponding assertion
- **AND** introducing the harness work fragment or a per-tool flag likewise breaks the asserted absence, with every mutation restored and the test passing again
- **AND** these observations are described as composition evidence, not proof of live Codex behavior or a new native-tool enforcement contract
