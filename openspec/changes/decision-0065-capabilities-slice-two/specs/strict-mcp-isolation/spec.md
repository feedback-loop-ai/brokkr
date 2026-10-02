## Purpose

Make 0065 ruling 6's "A harness's own MCP configuration is never inherited"
true at every model serving site, without claiming an unmeasured mechanism.

## ADDED Requirements

### Requirement: SI1 isolation is measured before support is declared

U0 SHALL record dated adapter evidence before U1 selects an isolation
mechanism. Every result SHALL name harness binary/version, host, invocation
shape, configuration sources, candidate, exact command/environment changes,
planted sentinel server, positive control, observed server start/call log,
tool listing/discovery, outcome and limitations. Supported hosts are Linux
and macOS; namespace activation remains Linux only. Missing evidence SHALL
be unmeasured, never unsupported-by-analogy or a successful negative.

Candidates SHALL include Claude's strict MCP flag with an explicit engine
configuration, the same through LaneTally's actual child, Codex engine-only
configuration/home isolation versus replacement of the complete MCP table,
and dsh's measured profile/plugin loading and possible engine-only profile.
These are experiments, not established flags or permission to add a plugin.
Codex trials SHALL independently measure authentication, normal model/effort
configuration and session behavior under isolation. They SHALL inspect
mcp_tool_call server/tool/call-identity fields and tool_search discovery.
DSH trials SHALL establish both ambient exclusion and engine MCP loading;
one does not prove the other.

#### Scenario: A sentinel distinguishes replacement from merging

- **WHEN** U0 plants an operator-scope MCP server and project-scope server in a disposable configuration, observes both on the positive control, then launches each candidate configuration
- **THEN** a supported result requires neither ambient server to start, be listed, be discovered or accept a call, while the engine's sentinel server remains usable
- **AND** the same experiment covers cold and each supported resumed shape, with and without workspace hands; failure or inability to observe a stream stays explicit

#### Scenario: Codex adding a server is not isolation

- **WHEN** an invocation adds only mcp_servers.brokkr to otherwise inherited configuration
- **THEN** U0 does not infer strictness from the added server, sandbox read-only, a successful command, or absence of a voluntary model call
- **AND** table replacement and private configuration candidates are measured independently, including configuration precedence and deferred discovery
- **AND** the selected evidence records real mcp_tool_call field names and repeat/start/completion behavior without fabricating them in this specification

#### Scenario: A wrapper or dsh needs its own evidence

- **WHEN** Claude passes a strictness trial but LaneTally or dsh has not passed its own trial
- **THEN** neither inherits Claude's support; its assessment remains unmeasured with its own cause
- **AND** no new harness, plugin, resume qualification or boundary is enabled merely to obtain a positive

### Requirement: SI2 every model serving path excludes ambient MCP

The engine SHALL compose exactly its authorized MCP server set on all model
seats, even with no hands, requests or grants. Without hands or holdings the
set is empty. Isolation SHALL apply to ordinary, inline, agent-backed,
selected/inherited, panel and sequence sites, each fallback and every actual
cold/resumed/replacement invocation. Exec scripts have no model MCP surface;
they SHALL not be misreported as a measured harness.

Compilation SHALL refuse a model candidate lacking a measured applicable
strict mechanism, independently of requires/wants. The final serving check
SHALL also refuse a missing or changed isolation mechanism before spawn.
Adapter mcp support SHALL have a real consumer for capability carriage;
the legacy server map SHALL supply no authority. Authored MCP/config/profile
options retain slice one's unconditional refusal.

Exact new site causes use the existing bounded site/office/realm prefix and
are: "provider '<provider>' cannot exclude ambient MCP configuration
(<measured reason>)" for measured unsupported isolation, or "provider
'<provider>' has no measured strict MCP configuration for '<shape>'" for
unmeasured/missing shape evidence. The reason comes from validated adapter
evidence, bounded and masked, never ambient config contents.

#### Scenario: An empty realm still needs strictness

- **WHEN** work seat implement (office implementer) in private requests nothing and its serving test provider has no measured cold isolation
- **THEN** compilation refuses "seat 'implement' (office 'implementer') in realm 'private': provider 'test-provider' has no measured strict MCP configuration for 'cold'"
- **AND** granting a wanted capability or omitting hands does not bypass that refusal

#### Scenario: A measured limitation is a distinct cause

- **WHEN** the same provider records unsupported strictness with reason "project MCP configuration cannot be excluded"
- **THEN** compilation refuses "seat 'implement' (office 'implementer') in realm 'private': provider 'test-provider' cannot exclude ambient MCP configuration (project MCP configuration cannot be excluded)"
- **AND** no optional MCP drop substitutes for protection against ambient servers

#### Scenario: Final configuration cannot reintroduce ambient servers

- **WHEN** final assembly removes strict isolation, appends another MCP config source, or changes the isolated configuration after compile
- **THEN** launch refuses "provider '<provider>' final MCP configuration is not the engine's sealed server set" under the owning site prefix before any server or model starts
- **AND** valid empty, hands-only and hands-plus-brokers configurations remain distinct exact positives

#### Scenario: Fallback and resume repeat the check

- **WHEN** a primary fails to start or an eligible resume is selected
- **THEN** the new candidate/shape must pass its own strictness and final server-set check
- **AND** a resume without measured isolation refuses that shape or follows the existing cold replacement policy; it cannot silently use an unisolated resume
- **AND** replacing it cold is recorded as cold, not measured resume support

## Decisions

R3 requires measurement, so neither private Codex home nor whole-table
replacement is selected here. Design names and bounds both experiments;
U0 supplies the answer before adapter support changes. The requirement is
exclusion, not a particular configuration flag. Strictness applies before
MCP enablement and to native-only seats as a correction of #467.

A model's statement "no tools" alone is rejected as proof: the sentinel's
positive control and server lifecycle observations must distinguish absence
from a model choosing not to call. The checked Codex adapter explicitly
records ambient isolation as unproven; its hands fragment cannot certify it.
