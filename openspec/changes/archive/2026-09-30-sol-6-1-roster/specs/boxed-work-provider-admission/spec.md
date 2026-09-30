## MODIFIED Requirements

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
- **THEN** it refuses on link 2, provider `claude`, naming `hands.harness.work` and the existing writable-sandbox reason even though the Sol/Codex link can compile alone
- **AND** no guessed Claude fragment, fallback omission or combined launch is introduced to make it pass

### Requirement: The shipped engine smith's namespace launch is tested as composed

Deterministic regressions SHALL load the actual shipped engine-smith agent
and Codex and Claude adapters, resolve the work seat under namespace and
inspect each candidate's launch produced by the production composition path.
Independent expectations SHALL establish the complete relevant argv and
server policy rather than compare a helper to itself. The proof SHALL be runnable without a provider,
network or nested namespace and SHALL canonicalize temporary workdirs.

#### Scenario: The complete Codex namespace launch exposes exactly the declared hands route
- **WHEN** the shipped engine smith's Sol candidate is composed for a canonical temporary workdir and its own result path under namespace
- **THEN** the model and effort are Sol's concrete Codex model and medium, and the launch registers `mcp_servers.brokkr` with the engine executable and `hands serve` arguments
- **AND** the decoded server arguments carry the canonical workdir and the complete declared hands spec, including network false, both toolchain binds and both Cargo masks
- **AND** the native sandbox is exactly `--sandbox read-only` from `hands.workspace`, and the shipped MCP approval configuration remains present
- **AND** no `workspace-write`, harness work fragment, per-tool list flag or unexpanded hands placeholder appears anywhere in that launch

#### Scenario: The shipped Claude launch preserves its complete workspace fragment (A1)
- **WHEN** the shipped engine smith's Fable candidate is composed for a canonical temporary workdir and its own result path under namespace
- **THEN** the model and effort are Fable's concrete Claude model and high, and the complete existing workspace fragment appears in order: `--tools`, an empty argument, `--strict-mcp-config`, `--mcp-config`, the expanded hands MCP JSON, `--allowedTools`, `mcp__brokkr__workspace`
- **AND** the decoded MCP JSON registers the Brokkr server with the engine executable and `hands serve` arguments carrying the canonical workdir and complete declared hands policy, including network false, both toolchain binds and both Cargo masks
- **AND** `--allowedTools` occurs exactly once and grants only `mcp__brokkr__workspace`; no argument contains `Bash(cargo:*)` or `Bash(git:*)`
- **AND** no harness work fragment or unexpanded hands placeholder appears; preserving this shipped fragment requires no adapter edit
