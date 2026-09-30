## MODIFIED Requirements

### Requirement: The engine smith hires the ruled chain through workspace hands

The shipped `implementer-engine` agent SHALL declare models exactly
`["sol", "fable"]` and efforts exactly `{"sol":"medium","fable":"high"}`
(decision 0045's addendum of 2026-09-30: Sol stands where Astra stood, one
effort step below Astra's). It SHALL declare workspace hands with `network: false`, a `~/.cargo` overlay
masking `credentials.toml` and `credentials`, and `~/.rustup` read-only.
The existing workspace hands mount SHALL supply the writable workdir; extra
binds SHALL NOT duplicate it or name a machine-specific checkout. The agent
SHALL omit `tools`, retain its charter and limits, and declare no boundary:
the realm selects that axis. Both providers SHALL receive the same hands
policy under namespace. Each adapter's existing `hands.workspace` fragment
SHALL be preserved, including Claude's MCP workspace grant. Neither provider
SHALL receive arguments generated from the retired agent `tools.allow` list.

#### Scenario: The shipped engine smith resolves both ruled hires
- **WHEN** an otherwise valid work seat names `implementer-engine` under a namespace realm using the shipped adapters
- **THEN** it compiles with Sol on Codex first at medium effort and Fable on Claude second at high effort
- **AND** its recorded hands contain network false and exactly the two toolchain binds above; its boundary is namespace

#### Scenario: Writable project access uses the existing workspace mount
- **WHEN** the smith's workspace hands are composed for a canonical temporary workdir
- **THEN** the hands server receives that workdir as its writable workspace and the declared Cargo overlay, masks and read-only Rust toolchain as additional binds
- **AND** the agent contains no checkout-specific path, additional writable host bind, network grant or `boundary` field

#### Scenario: The inactive tools declaration is removed while the workspace grant remains (A1)
- **WHEN** the shipped engine-smith declaration is inspected and resolved for both chain links under namespace
- **THEN** it has no `tools` object and neither candidate receives arguments generated from the retired `tools.allow` list
- **AND** Sol/Codex receives no per-tool list flag, while Fable/Claude retains exactly the existing `--allowedTools mcp__brokkr__workspace` grant as part of its complete `hands.workspace` fragment
- **AND** neither candidate receives `Bash(cargo:*)` or `Bash(git:*)` anywhere in its arguments; granting the workspace MCP tool does not impose a `cargo,git` command restriction inside the box

### Requirement: Deterministic evidence does not stand in for the first live smith

The delivery record SHALL distinguish expressibility from live proof. This
boxed seat has no network and SHALL NOT run or claim a live Sol smith.
The first live Sol implementation SHALL remain the controller's measurement
after landing: Cargo and Git through Codex's workspace hands, a real commit,
and verify passing. Compilation, launch inspection, mocks and removal tests
SHALL NOT discharge that measurement. The result notes SHALL describe evidence
and residuals, never instruct a gate to pass.

#### Scenario: A deterministic composition passes before any live measurement
- **WHEN** the compiler and composition tests pass without a controller observation of Sol implementing
- **THEN** the record says the ruling is expressible, not proved by a live smith, and names the controller's Cargo/Git/commit/verify measurement as pending

#### Scenario: The required quality evidence is available or explicitly pending
- **WHEN** implementation completion is assessed
- **THEN** the evidence covers formatting, clippy with warnings denied, every crate's suite separately, `cargo test --workspace`, compilation of `bundles/self`, strict OpenSpec validation, and literal 100% exact coverage including every added production line
- **AND** unavailable execution, namespace skips or a missing external exact-coverage result are recorded as pending evidence, never as a pass or a lower threshold
- **AND** tests use canonical temporary roots on Linux and macOS, add no Windows obligations under decision 0063, and write no frozen fixture or contract bytes
