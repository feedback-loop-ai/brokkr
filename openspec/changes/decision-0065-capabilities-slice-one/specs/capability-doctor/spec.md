## Purpose

Show operators, per realm, which capabilities they granted and which native
provider powers are denied, unsupported or still unmeasured.

## ADDED Requirements

### Requirement: Doctor reports grants for every realm

brokkr doctor SHALL report each realm's granted capability names, selected
tool dialects and office scope. It SHALL distinguish unrestricted requesting
office scope from an explicit office list or empty scope, and say when a realm
grants nothing. It SHALL show narrowed tools and restrictions without exposing
secret values. Reporting SHALL use that realm's own data, not the realm of
the current working directory for all rows. Abstract class definitions SHALL
be resolved from the same explicit operator definition context as compilation.
A definition SHALL not be reported as a grant; a missing or conflicting
required definition SHALL be reported as invalid metadata with its source and
capability, rather than a provider-availability or ordinary no-grant result.
Independent realm and native-inventory reporting SHALL continue where possible.

#### Scenario: Neighboring realms keep different grants

- **WHEN** a world has private with no capabilities and public granting web-search through codex-native-search only to researcher
- **THEN** doctor names private as granting none
- **AND** its public report names web-search, codex-native-search and researcher without implying all public offices hold search

#### Scenario: Empty and unrestricted scope are distinguishable

- **WHEN** otherwise identical valid grants omit offices, name two offices, or specify offices: []
- **THEN** doctor respectively reports all requesting offices, the two named offices, and no offices
- **AND** the empty list cannot render as all offices

#### Scenario: No realm map is still a visible denial default

- **WHEN** doctor runs in a repository without a realms map
- **THEN** it reports that no capability grants are declared and native capabilities are governed by the no-grant default

#### Scenario: CQ2 definitions do not become grants in doctor

- **WHEN** operator-library-docs has a valid reads/egress operator definition and private grants nothing
- **THEN** private still reports no grants, without an invented dialect or holding
- **AND** a library request missing that definition or a selected dialect asserting conflicting classes instead reports the named metadata defect while preserving independent native-denial and unmeasured-inventory lines

#### Scenario: CQ1 a declared restriction is not a claim of usable authority

- **WHEN** a valid native grant contains a restriction its binding cannot express
- **THEN** doctor shows the grant, dialect, scope and restriction as declared, and names the binding's restriction incompatibility without claiming it can run unrestricted
- **AND** if reporting a resolved seat, it reports the requires refusal, wants drop with OFF, or unused inactive grant according to CQ1 rather than converting that grant to a holding

### Requirement: Installed native capabilities absent from grants are explicit

For every installed harness, doctor SHALL compare its declared native
capabilities against each realm's grants and name those not granted there.
A same-name grant through another provider's dialect or a restricted office
scope SHALL NOT be described as universal native authorization. Doctor SHALL
distinguish an available declared OFF control from a measured impossible OFF
control and include the unsupported reason and compile consequence. It SHALL
not equate a missing provider binary with an installed denied capability.
A matching grant SHALL NOT bypass OFF assessment for seats without an
effective holding, including empty/scoped grants, empty tools, unused grants
and subtracted requests. Unsupported OFF SHALL report the compile refusal
with its measured reason; unmeasured OFF SHALL retain its reason and claim
no denial. For a known unheld native capability, an unmeasured or unavailable
mandatory OFF plan SHALL report the same refusal consequence as compilation.
Only a valid composable OFF disposition SHALL justify saying an unheld seat
is launched with the capability switched off, within its evidence scope.
Doctor SHALL use the same provider-aware grammar, control representation and
composition admission as compilation and final launch. An argv, selection or
default tag alone SHALL NOT be assessed as delivered. A representation the
provider cannot consume, a malformed control or an incompatible composition
SHALL be reported as a compile refusal with its complete cause, distinct from
measured unsupported and unmeasured controls. A report without a resolved seat
SHALL state the scope of its assessment rather than assert universal delivery
for unassessed authored arguments or restrictions.

Doctor SHALL submit the complete resolved plan to the same composer and final
validation used by launch: all known native dispositions, typed permissions,
hands, restrictions, authored inert arguments, serving candidate and command
shape together. It SHALL report that admission or refusal, never certify
capabilities one at a time with fabricated answers for the rest. Without a
resolved seat, only an explicitly labeled complete adapter-level hypothetical
plan may be assessed; no universal seat-launch promise is allowed.

#### Scenario: Individually valid controls can conflict together

- **WHEN** Claude declares separate OFF argv for WebFetch and WebSearch that duplicate an option or admit a tool the other denies
- **THEN** doctor submits both together and reports exactly the whole-plan composer's admission or refusal; two per-capability Delivered tags are insufficient
- **AND** matching compiled fixtures independently assert the same full outcome for absent, scoped, empty, unused and subtracted grants

#### Scenario: Doctor observes final assembly and authored refusal

- **WHEN** a plan has an authored forbidden option, a cross-origin duplicate or a final parse/state mismatch
- **THEN** doctor reports the same bounded cause as launch admission and promises no delivery
- **AND** an adapter-only assessment names its scope and leaves unassessed seat arguments unclaimed

#### Scenario: A complete admitted plan is reported at its actual scope

- **WHEN** a resolved seat has compatible local permissions, required hands and all native ON/OFF and restriction dispositions
- **THEN** both doctor reporting paths submit that entire candidate and command shape to launch's final composer and report its statically admitted result
- **AND** independent full-line expectations name the assessed seat and evidence scope; they are not generated from the composer under test and make no live-enforcement promise

#### Scenario: Codex default search is made visible

- **WHEN** Codex is installed and private grants nothing
- **THEN** doctor names private, codex and ungranted web-search, and reports its declared disabled control with the cold-0.154.0 evidence scope
- **AND** it does not infer denial from workspace network false

#### Scenario: A partial grant does not authorize every office or provider

- **WHEN** private grants web-search to researcher through a different provider's native dialect
- **THEN** doctor reports that binding and scope accurately
- **AND** it does not present Codex's native search or other offices as covered by that entry

#### Scenario: Impossible denial predicts the refusal

- **WHEN** an installed test provider declares a native capability's OFF control unsupported with a measured reason and private has no grant for it
- **THEN** doctor names the provider, capability and private, prints that reason, and says seating it without the grant will refuse compilation

#### Scenario: An absent provider is not measured as installed

- **WHEN** the same provider is absent from the supplied availability facts
- **THEN** doctor reports provider absence using the existing availability behavior
- **AND** it does not claim to have observed its native capability state on this host

#### Scenario: M1 a matching scoped grant cannot imply denial elsewhere

- **WHEN** a valid grant matches an installed provider/key but names only researcher, leaving another seat without a holding
- **THEN** doctor names the grant and office scope and independently reports supported OFF, unsupported OFF with compile refusal, or unmeasured OFF with its reason and no denial claim
- **AND** the unsupported and unmeasured cases never print that every other seat is switched off

#### Scenario: M1 empty and unused grants retain the OFF consequence

- **WHEN** the matching grant has `offices: []`, has `tools: []`, is not requested, or has been subtracted by the seat
- **THEN** each case reports the correct OFF disposition for the unheld capability, including compile refusal whenever a known power cannot receive valid denial
- **AND** tests independently assert each complete report line for supported, unsupported and unmeasured OFF, not merely the grant description

#### Scenario: M1 reporting cannot survive removal of its disposition check

- **WHEN** a matching grant is made to bypass the OFF assessment in an isolated removal experiment
- **THEN** the scoped, empty and unused unsupported/unmeasured report assertions fail at their exact expected lines
- **AND** restoration passes while the supported control still reports declared denial within its actual evidence scope

#### Scenario: Second M2 a Codex selection OFF reports refusal rather than denial

- **GIVEN** installed Codex declares web-search OFF as a tool selection that its provider composition cannot consume
- **WHEN** doctor reports a realm with no grant, a matching scoped grant, empty offices, empty tools, an unused grant or a subtracted request
- **THEN** every unheld case reports the same complete provider/representation refusal cause as compilation and never says the seat is launched with search switched off
- **AND** independent whole-line assertions verify every grant shape and the complete compile refusal; no live provider is invoked
- **AND** valid Codex argv OFF remains a positive declared-composition result within the cold-only measurement scope

#### Scenario: Second M2 provider composition determines every supported claim

- **WHEN** otherwise valid native OFF declarations use malformed managed argv, an unsupported selection mapping, an unconsumed DSH representation or a conflicting explicit restriction
- **THEN** doctor and compile agree on the full grammar/composition refusal instead of treating the representation tag as proof
- **AND** supported argv/selection/default controls, measured unsupported controls and unmeasured controls each retain their distinct report and complete reason
- **AND** authored deny lists refuse, and accepted engine-owned empty restrictive lists are not reported as absent controls

#### Scenario: Second M2 removing provider assessment breaks the report proof

- **WHEN** only doctor's provider-aware assessment is replaced by declaring every argv/selection OFF delivered
- **THEN** the separate scoped, empty and unused Codex-selection whole-line assertions fail for the false promise while compilation still refuses
- **AND** restoring the shared assessment restores the exact lines and pass; a required-case failure or grant-description assertion cannot substitute

### Requirement: Unknown inventories and live-control gaps remain unmeasured

Doctor SHALL surface DSH and LaneTally native assessments as unmeasured with
their adapter reasons. It SHALL preserve the distinction between unknown
inventory, known unsupported OFF and declared but not live-measured controls.
Claude's existing empty native-tool list and strict MCP declaration SHALL be
identified as adapter data. Codex's cold measurement SHALL NOT become a
resumed-session measurement in a doctor line.

#### Scenario: DSH remains an open measurement

- **WHEN** installed DSH declares native capabilities unmeasured because mcp/tool_permissions unsupported do not prove absence of native egress
- **THEN** doctor prints unmeasured with that reason beside DSH in the relevant per-realm report
- **AND** it reports neither an empty verified inventory nor verified denial

#### Scenario: LaneTally is judged through its own wrapper

- **WHEN** installed LaneTally has unverified native controls
- **THEN** doctor prints its unmeasured reason even if Claude is also installed and declares controls
- **AND** it does not inherit Claude's evidence

#### Scenario: Known adapter configuration is not a live result

- **WHEN** doctor reports Claude's declared controls and Codex's measured cold OFF fragment without additional live evidence
- **THEN** it labels Claude enforcement and Codex resumed enforcement as unmeasured
- **AND** it does not upgrade either based on successful argv unit tests

### Requirement: Doctor explains unbuilt bindings without running capabilities

Doctor SHALL diagnose a realm selecting an MCP dialect as not implemented
until decision 0065 slice two, including the capability and dialect, rather
than offering it as usable. Doctor SHALL continue reporting independent
realms and measurement gaps where possible. It SHALL execute no model request,
search, fetch or MCP capability server to produce this report; normal bounded
provider availability/version inspection SHALL not count as capability proof.

#### Scenario: An MCP declaration is not reported as a working server

- **WHEN** a realm's library-docs grant selects docs-mcp
- **THEN** doctor names that grant and says its broker support is absent until slice two
- **AND** a deterministic test observes no capability server launch

#### Scenario: Reports can be proven without live providers

- **WHEN** tests supply installed/absent provider facts and two realm maps with known, unsupported and unmeasured native declarations
- **THEN** assertions verify the complete relevant lines, scope and reasons for each realm
- **AND** no live provider response is required or claimed by the test

## Decisions

Ruling 4 combines third S4/C5/R7 with earlier M1/M2: OFF-first ordering and
provider-aware per-capability checks are insufficient. The whole actual plan
must reach the launch composer. Reject a doctor-only resolver, synthetic
incomplete Controls and expected test causes obtained from the production
function under test. Adapter-level scope and static versus live evidence
remain distinct even when complete composition succeeds.
