## Purpose

Seat Astra as the engine smith with Fable as fallback under decision 0043's
existing workspace confinement, with declared identity and evidence limits
that an operator can inspect.

## ADDED Requirements

### Requirement: The engine smith hires the ruled chain through workspace hands

The shipped `implementer-engine` agent SHALL declare models exactly
`["astra", "fable"]` and efforts exactly `{"astra":"high","fable":"high"}`.
It SHALL declare workspace hands with `network: false`, a `~/.cargo` overlay
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
- **THEN** it compiles with Astra on Codex first and Fable on Claude second, both at high effort
- **AND** its recorded hands contain network false and exactly the two toolchain binds above; its boundary is namespace

#### Scenario: Writable project access uses the existing workspace mount
- **WHEN** the smith's workspace hands are composed for a canonical temporary workdir
- **THEN** the hands server receives that workdir as its writable workspace and the declared Cargo overlay, masks and read-only Rust toolchain as additional binds
- **AND** the agent contains no checkout-specific path, additional writable host bind, network grant or `boundary` field

#### Scenario: The inactive tools declaration is removed while the workspace grant remains (A1)
- **WHEN** the shipped engine-smith declaration is inspected and resolved for both chain links under namespace
- **THEN** it has no `tools` object and neither candidate receives arguments generated from the retired `tools.allow` list
- **AND** Astra/Codex receives no per-tool list flag, while Fable/Claude retains exactly the existing `--allowedTools mcp__brokkr__workspace` grant as part of its complete `hands.workspace` fragment
- **AND** neither candidate receives `Bash(cargo:*)` or `Bash(git:*)` anywhere in its arguments; granting the workspace MCP tool does not impose a `cargo,git` command restriction inside the box

### Requirement: Changed hires and hands are witnessed by actual compiled identities

Every witness and compose digest moved by this declaration SHALL match an
actual compile of its bundle, including inherited uses and affected ancestors.
Both pin collections SHALL explain the move in their history blocks: the
operator's Astra/Fable hire, effort pins and hands replace the earlier tool
list and therefore change recorded agent and hands identity. Unaffected pins
SHALL remain unchanged. Shipped panel diversity, GPT/Flash's independent
roster and existing runtime invariants SHALL remain satisfied.

#### Scenario: A hiring bundle changes its identity for the declared reason
- **WHEN** all shipped bundles and recipes are compiled after the engine-smith declaration changes
- **THEN** every changed digest is measured and pinned in `tests/witness_digests.rs` and `bundle/compose_tests.rs` wherever represented
- **AND** their histories attribute this change to issue #307 rather than replacing old history or guessing a hash

#### Scenario: The independent rosters retain their guarantees
- **WHEN** `every_shipped_panel_seats_at_least_two_providers`, the `gpt_flash_shape` suite and the entire `brokkr-runtime` crate suite execute
- **THEN** they pass without weakening panel diversity, substituting the GPT/Flash engine smith, or making an exception for dead tool declarations beside hands

### Requirement: The guide and decision record describe the confinement actually declared

`docs/guides/provider-adapters.md` SHALL explain that a provider without native
per-tool flags serves this restricted work seat through declared hands. Under
namespace it requires `hands.workspace`: writable work occurs through MCP
hands and the native Codex sandbox remains read-only. The confinement SHALL
be described as the existing filesystem and network boundary, never a command
allow-list. The guide SHALL preserve 0043's limits: Codex's native shell is
still available read-only outside that box, host-read secrecy is not promised,
and provider traffic belongs to the harness outside the box. The distinct
`hands.harness.work` route SHALL retain its unboxed meaning and existing
admission requirements.

Decision 0043 SHALL gain a dated note recording the operator's 2026-09-21
issue #307 ruling without changing its accepted status or rewriting its
historical text. The note SHALL withdraw the earlier allow-list-enforcement
claim and record that no new per-tool contract or boundary composition is
being designed.

#### Scenario: The allow-list escalation is answered by the recorded ruling
- **WHEN** a reader asks whether the boxed smith is limited to commands named `cargo` and `git`
- **THEN** the guide and dated note say it is not: workspace hands execute shell commands subject to the existing filesystem and network boundary, and the earlier commission's enforcement claim is withdrawn
- **AND** neither artifact adds command parsing, transport semantics or native-tool bypass prevention as an obligation of this slice

#### Scenario: The composition escalation is answered without combining paths
- **WHEN** a reader compares namespace work with harness work
- **THEN** the guide says namespace uses the MCP server plus Codex's read-only native sandbox, while harness work uses its declared writable harness fragment without a Brokkr box
- **AND** the note records the operator's existing-semantics ruling rather than treating the two paths as parts of one launch or re-escalating that settled question

#### Scenario: The note does not create or accept a new semantic rule
- **WHEN** the 2026-09-21 note is added to decision 0043
- **THEN** its status remains accepted, the historical rulings remain intact, and the note describes application of existing decisions 0043 and 0046

### Requirement: Deterministic evidence does not stand in for the first live smith

The delivery record SHALL distinguish expressibility from live proof. This
boxed seat has no network and SHALL NOT run or claim a live Astra smith.
The first live Astra implementation SHALL remain the controller's measurement
after landing: Cargo and Git through Codex's workspace hands, a real commit,
and verify passing. Compilation, launch inspection, mocks and removal tests
SHALL NOT discharge that measurement. The result notes SHALL describe evidence
and residuals, never instruct a gate to pass.

#### Scenario: A deterministic composition passes before any live measurement
- **WHEN** the compiler and composition tests pass without a controller observation of Astra implementing
- **THEN** the record says the ruling is expressible, not proved by a live smith, and names the controller's Cargo/Git/commit/verify measurement as pending

#### Scenario: The required quality evidence is available or explicitly pending
- **WHEN** implementation completion is assessed
- **THEN** the evidence covers formatting, clippy with warnings denied, every crate's suite separately, `cargo test --workspace`, compilation of `bundles/self`, strict OpenSpec validation, and literal 100% exact coverage including every added production line
- **AND** unavailable execution, namespace skips or a missing external exact-coverage result are recorded as pending evidence, never as a pass or a lower threshold
- **AND** tests use canonical temporary roots on Linux and macOS, add no Windows obligations under decision 0063, and write no frozen fixture or contract bytes
