## Purpose

Make realms.json the sole source of capability grants, preserving old realm
loading while removing implicit native authority from every realm.

## ADDED Requirements

### Requirement: Realms v6 adds grants without changing frozen versions

The system SHALL add contracts/realms.v6.schema.json with schema
forge.realms/v6, preserving v5 fields and adding an optional per-realm
capabilities map. Each entry SHALL select a tool dialect, with optional
tools subset, offices scope and dialect-defined restriction keys. Existing
versions v1–v5 SHALL keep loading and grant no capabilities. Missing map,
missing capabilities and an empty capabilities map SHALL all grant nothing.
The repository's own realms.json SHALL remain without grants. Abstract
capabilities/ definitions SHALL be separate operator data under the lookup
rules in tool-dialect-contract; their presence SHALL NOT constitute a realm
grant or require changing a legacy map's frozen schema.

#### Scenario: Every older version denies by absence

- **WHEN** each valid forge.realms/v1 through forge.realms/v5 map is loaded and a Codex seat compiles against its realm
- **THEN** the map loads with no capability grants and the seat's web-search is composed OFF if a valid denial plan exists; otherwise seating Codex refuses with the complete native-denial cause
- **AND** no migration or historical use grants it implicitly

#### Scenario: Absence and an explicit empty map are equivalent

- **WHEN** the same seat compiles without a realms map, in a v6 realm omitting capabilities, or in a v6 realm with capabilities: {}
- **THEN** each holds no requested capability and enforces the same native denial policy

#### Scenario: New fields cannot hide under an older version

- **WHEN** a v1–v5 realm supplies capabilities even as an empty map
- **THEN** loading refuses naming the realm, capabilities and forge.realms/v6 as the version that admits the field
- **AND** existing realm contracts retain their frozen bytes

#### Scenario: This repository authorizes no search

- **WHEN** the shipped realms.json is loaded after implementation
- **THEN** every realm in it grants no capabilities
- **AND** upgrading its schema, if done, does not add any grant

### Requirement: Only an operator's realm selects concrete implementations

Recipes, agents, inline seats, panel members, sequence steps and selected
case bodies SHALL declare abstract requests only, never a grant catalogue,
tool dialect selection, native provider key, server definition or restriction
that widens authority. Tool dialect serves SHALL equal the realm capability
key. A missing, invalid, escaping or mismatched dialect reference SHALL be a
named compile refusal. Recipe composition SHALL NOT manufacture realm data.

#### Scenario: The office remains portable

- **WHEN** an office requests web-search and two independently configured realms select different valid native dialects serving it
- **THEN** the office declaration and charter need no server, provider or tool-name edit
- **AND** each realm alone determines the concrete binding

#### Scenario: A recipe cannot supply the missing grant

- **WHEN** an agent or executable site attempts to put dialect, server, tools or grant restrictions in its capability request
- **THEN** loading or compilation refuses naming the site and explaining that requests use requires/wants and grants belong to realms.json
- **AND** a recipe inherited through composition cannot evade that refusal

#### Scenario: A dialect cannot serve a different abstraction accidentally

- **WHEN** realm private grants web-search using a dialect whose serves is web-fetch
- **THEN** compilation refuses naming private, web-search, the dialect and its actual serves value

### Requirement: Authored provider configuration cannot supply capability authority

Compilation SHALL refuse every authored capability-bearing option for Claude,
Codex, DSH and LaneTally's Claude path, in driver commands and seat settings,
including every primary/fallback, inline/agent-backed, nested and inherited
site. The refusal SHALL be independent of grants, value, polarity and apparent
agreement with the plan. The operator's exact rule is “Nothing is merged.”
Tools SHALL come only from typed agent/seat declarations and applicable realm
grants, composed by the engine. Engine-owned hands and typed local permissions
SHALL retain their provenance; copying their bytes SHALL NOT confer it.

This catalogue is exhaustive for the supported known-harness surface. Each
value-taking long name includes split `--name VALUE` and joined `--name=VALUE`,
every listed alias, every repeat and every variadic value. Every value-taking
short alias includes `-x VALUE`, `-x=VALUE` and attached `-xVALUE`. Empty,
wildcard, default, malformed and apparently restrictive values are refused
without parsing authored tool-list contents into contributions. Unsupported
forms and future/unknown options SHALL also refuse under the closed grammar;
this catalogue SHALL NOT become a permissive spelling scanner.

| Harness | Authored options/configuration refused |
| --- | --- |
| Claude | Tool lists `--tools`, `--allowedTools`, `--allowed-tools`, `--disallowedTools`, `--disallowed-tools`; MCP `--mcp-config`, `--strict-mcp-config`; plugin loading `--plugin-dir`; permission controls `--permission-mode`, `--dangerously-skip-permissions`, `--allow-dangerously-skip-permissions`, `--permission-prompt-tool`, `--add-dir` (additional filesystem permission); opaque settings/agent loading `--settings`, `--setting-sources`, `--agents`, `--agent`. WebSearch/WebFetch, MCP and wildcards in any list are already refused by the option, as are unsupported `--web`, `--web-search`, `--web-fetch`, `--search` forms. |
| LaneTally (Claude child) | Every Claude entry above, in the forwarded command or wrapper settings; the wrapper cannot launder a child capability option. Settings-source controls are refused, including `--settings` and `--setting-sources`. Wrapper-owned session settings keep engine provenance and undergo final validation. |
| Codex | `-c` / `--config` assignments to `mcp_servers` and every descendant/table form; `web_search`, `web_search_mode`, `tools.web_search`, `features.web_search_request`, `features.web_search_cached`; permission/sandbox keys `approval_policy`, `sandbox_mode`, `sandbox_workspace_write` and descendants; tool/plugin/feature/MCP tables and descendants whose effects carry capabilities. `--search`, `--include-plan-tool`; `--add-dir` (additional filesystem permission); `--enable` / `--disable` capability features (including web_search_request/web_search_cached); `--sandbox` / `-s`, `--ask-for-approval` / `-a`, `--full-auto`, `--approve-for-me`, `--ignore-rules`, `--dangerously-bypass-approvals-and-sandbox` / `--yolo`; profile loading `--profile` / `-p`. Unclassified config keys, feature names and malformed assignments refuse, never pass through. |
| DSH | Capability-bearing `--patch` contents or seat settings: tool/MCP configuration, plugin loading, permissions or web/search enablement; arbitrary `--profile`, `web` and `plugin` launch subcommands; unbound/additional patches and opaque configuration. Unsupported tool-list/MCP/plugin/permission/web/search flags, including the names above, refuse rather than being forwarded. No short capability aliases are admitted by the supported DSH grammar; attempted aliases/attached forms refuse. The sole non-capability patch exception is the existing bound, contained, digest-checked route-only overlay. |

Codex config forms SHALL include all five `-c KEY=VALUE`, `-c=KEY=VALUE`,
`-cKEY=VALUE`, `--config KEY=VALUE`, `--config=KEY=VALUE`, with quoted/dotted
keys, whole tables and descendants. Every occurrence is classified before
reduction; a later harmless assignment cannot erase a forbidden one. Known
non-capability config such as model reasoning effort may be admitted only
with bounded typed meaning. No arbitrary settings document is inert data.

A refusal SHALL name a normalized option, provider, source/site and bounded
cause, never its value (including joined/attached tokens or secret-bearing
paths/config payloads). The option label SHALL come from the grammar or a
bounded sanitized key-free label for unknown syntax, not the raw token. The
option/cause portion SHALL be at most 512 Unicode scalar values; truncating
a raw token is not redaction. Known short aliases name their canonical long
option; unknown short-attached input uses a fixed unknown-option label.

#### Scenario: Every spelling refuses independently of authority

- **WHEN** each catalogue option is authored in each split, equals, short-attached and alias form applicable to its harness, with grants absent and then present
- **THEN** compilation refuses before any provider/plugin/server launch with the complete bounded option-naming reason, without echoing the value
- **AND** all primary/fallback, inline/agent-backed, work/gate, panel/sequence and inherited sites obey that same refusal

#### Scenario: Additional permissions and tool switches are authored authority

- **WHEN** Claude, LaneTally's Claude child or Codex authors `--add-dir PATH` or `--add-dir=PATH`, including repeated or supported variadic forms, or Codex authors `--include-plan-tool`
- **THEN** compilation refuses the normalized option without disclosing the directory or accepting the current grammar's inert/switch classification as authority
- **AND** bare switches with attempted equals or attached values refuse as unsupported grammar, rather than becoming a forwarding exception

#### Scenario: Short permission and profile aliases have no spelling gap

- **WHEN** Codex authors each of `-s VALUE`, `-s=VALUE`, `-sVALUE`, `-a VALUE`, `-a=VALUE`, `-aVALUE`, `-p VALUE`, `-p=VALUE`, `-pVALUE`, and the split/equal long forms of sandbox, ask-for-approval and profile
- **THEN** each refuses its canonical capability-bearing option regardless of a compatible realm grant or a restrictive value
- **AND** `--yolo` and the other bare permission aliases refuse too; unsupported combinations and unknown aliases never pass through

#### Scenario: Local lists and deny lists are not exceptions

- **WHEN** Claude or LaneTally authors `--tools Read`, `--tools=`, `--allowedTools Read`, `--disallowedTools mcp__*`, `--disallowedTools Read(` or an alias, repeated or multi-value variant
- **THEN** each refuses for the authored option even if it only narrows access or agrees with a managed control; none is folded into the plan
- **AND** engine-owned typed local tool declarations remain the migration path

#### Scenario: Codex assignments cannot hide behind spelling or order

- **WHEN** an authored MCP, web/search, plugin/tool or permission assignment uses any of the five config forms, quoted/table/descendant keys, or precedes a harmless assignment
- **THEN** compilation refuses the normalized `--config` option without printing the assignment or its value
- **AND** malformed and unclassified assignments refuse as unbounded configuration

#### Scenario: DSH distinguishes route data from capability options

- **WHEN** a DSH invocation supplies a bound route-only overlay and then independently a profile, a web/plugin subcommand, a tool-bearing patch, an unknown flag or an unsupported attached alias
- **THEN** only the validated contained digest-matched route-only case can compile; every other case refuses before staging
- **AND** the route overlay grants no capability and its pre-staging drift check remains mandatory

#### Scenario: Engine provenance cannot be copied

- **WHEN** engine-owned hands and typed permissions are composed under empty grants, then their concrete argv is copied into an authored command
- **THEN** the first retains its allowed typed authority and native OFF while the copied command refuses, regardless of matching server names or bytes

### Requirement: Known provider commands have a closed argument grammar

Every known-harness authored command SHALL parse fully into typed options and
values before admission. Unknown syntax, malformed assignments, ambiguous
boundaries, unplaced positionals and forbidden duplicates SHALL refuse.
Aliases, split/equals/attached forms, arity and repeatability SHALL have one
meaning shared by admission, selectors, extraction, resume and final parsing.
An inert value SHALL NOT become an option through a later token scan.

#### Scenario: Payloads never become diagnostics or controls

- **WHEN** an unknown joined option carries `REVIEW_SENTINEL`, a short-attached config carries it, or a rejected tool list contains it
- **THEN** the complete bounded diagnostic names the option and position but contains no sentinel or raw value
- **AND** an admitted inert value such as the word `resume` in `--image resume` remains a value through eligibility and final parsing

#### Scenario: Diagnostics stay bounded through their outer consumers

- **WHEN** joined options, attached short forms, config assignments, rejected positionals or list values contain long, newline-bearing or secret-path sentinels
- **THEN** complete compile, launch and doctor diagnostics contain no payload sentinel and their option/cause portion remains within 512 Unicode scalar values
- **AND** a fixed positional or unknown-option label supplies the bounded identity when the grammar cannot safely name an option

#### Scenario: A closed grammar has no opaque remainder

- **WHEN** a known harness receives a new option, malformed duplicate, dangling value or misplaced `--`
- **THEN** it refuses instead of forwarding the remainder, even when its adapter has an unmeasured inventory or another name

### Requirement: Subtractive tool lists never grant a capability

Typed subtraction SHALL narrow an office request. Authored harness deny lists
SHALL nevertheless refuse under ruling 1; subtraction is not an exception to
engine ownership. Managed denials SHALL remain effective without weakening
hands or required holdings; an unrepresentable managed conflict SHALL refuse.

#### Scenario: Refusal supersedes the earlier subtractive merge

- **WHEN** an authored Claude or LaneTally `--disallowedTools mcp__*` would have been compatible with a prior plan, or would conflict with workspace hands
- **THEN** both compile-refuse the authored option before any reconciliation
- **AND** typed request subtraction still removes the ask and composes native OFF

### Requirement: Office scopes and tool subsets only narrow a grant

Omitted offices SHALL make a grant available to all requesting offices;
an explicit list SHALL make it available only to the named offices; an empty
list SHALL make it available to none. Omitted tools SHALL admit the dialect's
named tools; an explicit list SHALL be a subset, never an expansion. Empty
tools SHALL admit no tool and SHALL NOT produce a usable holding: a remaining
requires refuses and a wants is dropped with a reason. Malformed scope/tool
lists and unknown tool names SHALL be rejected.

The office key SHALL be the referenced agent name for an agent-backed site,
and the stable executable site label for an inline site. Provider choice
SHALL NOT change the office. Diagnostics and manifests SHALL also retain the
site label so several seats hiring one office remain distinguishable.

#### Scenario: Scope does not bleed between offices

- **WHEN** private grants web-search to researcher and seats research and implement hire researcher and implementer respectively
- **THEN** only research has an applicable grant
- **AND** implement remains denied even if it uses the same provider or model

#### Scenario: Unused grants do not enable tools

- **WHEN** an office is in the grant's scope but does not request its capability, or explicitly subtracts it
- **THEN** the seat holds nothing from that entry and any controllable native equivalent remains OFF

#### Scenario: Tool narrowing is exact

- **WHEN** a test dialect serves a capability with tools lookup and search and its realm admits only lookup
- **THEN** the holding, composed native controls and manifest admit only lookup
- **AND** a grant adding a tool not declared by the dialect refuses rather than widening the set

#### Scenario: Empty lists do not mean unrestricted

- **WHEN** a grant has offices: [] or tools: []
- **THEN** the first authorizes no office and the second admits no tool
- **AND** a remaining requires refuses with the specific scope or empty-tool reason while a wants records that reason as a drop

### Requirement: Restrictions are validated, carried and pinned without engine interpretation

The realm's restriction values SHALL be validated against its chosen dialect's
schema and preserved unchanged as structured data in the pinned realm grant.
For a held capability the binding's configuration and capability record SHALL
carry those values unchanged. Reserved keys SHALL retain their engine-defined
meaning. The compiler SHALL distinguish grant validity from the serving
provider's ability to enforce a valid grant; it SHALL NOT drop a restriction
while retaining the capability.

All selected-realm grants SHALL pass structural, reference, class-consistency,
tool-subset and restriction-schema validation before seat compatibility is
resolved. Invalid restrictions SHALL refuse compilation irrespective of asks,
strength or office scope. The MCP slice-two refusal and reserved-hands refusal
also SHALL apply independently of demand. These realm-wide refusals SHALL NOT
be converted to optional drops.

After valid native grants are established, inability to express a restriction
SHALL make the binding unusable for that provider candidate. A remaining
requires SHALL refuse; a wants SHALL be dropped with its full compatibility
reason in manifest notices and the native capability SHALL remain OFF. A
grant unused after asks, subtraction and office scope SHALL stay pinned but
inactive; native ON/restriction composition SHALL not be required for that
unused binding. Both adapter-declared argv halves still parse at load; this
inactive state is not a syntax-validation exemption. Known native OFF checks SHALL still run independently for
every serving candidate: an unsupported or unmeasured OFF for a known native
capability, or missing required denial authority, SHALL refuse even where a
wants could otherwise drop or a grant is unused. Successful cases SHALL
never launch an unrestricted version of the rejected binding.

#### Scenario: A valid restriction survives each boundary

- **WHEN** a test native binding supports a dialect-defined restriction object and the realm provides a valid value
- **THEN** resolution, the binding's configuration and the manifest carry that value unchanged
- **AND** changing the restriction changes the manifest digest

#### Scenario: CQ1 a required inexpressible restriction refuses

- **GIVEN** private grants web-search to researcher through native dialect search-native with schema-valid allow.hosts and the test-native adapter cannot express allow.hosts but can switch web-search OFF
- **WHEN** seat research requires web-search
- **THEN** compilation refuses "seat 'research' (office 'researcher') in realm 'private': requires capability 'web-search' through dialect 'search-native', but provider 'test-native' cannot express restriction 'allow.hosts'; the capability cannot be held under this grant"
- **AND** no seat launches and the restriction is not removed to make the grant usable

#### Scenario: CQ1 an optional inexpressible restriction drops with OFF

- **GIVEN** exactly the same valid grant, dialect, provider and supported OFF control as the preceding scenario
- **WHEN** research instead wants web-search
- **THEN** compilation succeeds without that holding and records "seat 'research' (office 'researcher') in realm 'private': dropped wanted capability 'web-search' through dialect 'search-native' because provider 'test-native' cannot express restriction 'allow.hosts'; native capability remains OFF"
- **AND** the final argv contains the adapter's OFF disposition, never an unrestricted ON disposition
- **AND** the manifest retains the grant's original restriction as realm context, and the prompt explains the lost capability without claiming the restriction was enforced on an enabled tool

#### Scenario: CQ1 an unused inexpressible grant stays inactive

- **GIVEN** exactly the same valid grant, dialect, provider and supported OFF control as the preceding scenarios
- **WHEN** no seat asks for web-search or every asking seat subtracts it
- **THEN** compilation succeeds without a holding or native ON/restriction configuration for that entry
- **AND** all serving test-native seats receive OFF and the unchanged restriction remains pinned in realm context
- **AND** no optional restriction-compatibility notice is invented for a nonexistent or subtracted ask
- **AND** if a remaining ask is instead excluded by office scope, requires refuses for scope and wants records its scope-drop notice before restriction compatibility is relevant

#### Scenario: CQ1 invalid restrictions refuse before optionality

- **WHEN** allow.hosts in any of those three cases has a value forbidden by search-native's schema
- **THEN** compilation refuses naming private, web-search, search-native and the complete allow.hosts schema violation even for wants or an unused grant
- **AND** no compatibility drop substitutes for the invalid-grant refusal

#### Scenario: CQ1 impossible OFF overrides an optional drop or unused grant

- **WHEN** the optional or unused case instead has web-search OFF declared unsupported with a measured reason
- **THEN** compilation refuses with the complete native-denial reason naming research, private, web-search, test-native and its measured OFF limitation
- **AND** an existing realm grant cannot authorize a capability that the seat does not effectively hold

### Requirement: Realm context is resolved before capability authorization

Compile, run and rerun SHALL authorize against the operated repository's
realm, not a neighboring realm or the recipe's home. Resume SHALL use the
run's pinned realm and capability identity; it SHALL NOT borrow newly added
workspace grants or restore the old default-ON behavior from a legacy run.
Any inability to reproduce an authorized identity SHALL use the existing
manifest-mismatch refusal, with the capability difference named. Starting a
run SHALL refuse a world whose operated realm has different capability
authority from the context under which the bundle compiled, before creating
the run or spawning any seat.

#### Scenario: Neighboring realms do not share a grant

- **WHEN** a world contains public with web-search and private without it and the operated repository belongs to private
- **THEN** compiling or rerunning the same recipe denies search in private despite public's grant

#### Scenario: Resume cannot acquire a grant from an edited map

- **WHEN** a run pinned no web-search grant and the workspace map now grants it
- **THEN** with the original capability-capable inputs unchanged, resume reproduces the pinned no-grant context and composes OFF despite the edited map
- **AND** a supplied bundle that changes that pinned capability identity is refused by manifest comparison rather than silently enabling search

#### Scenario: A pre-capability manifest gets no grandfathering

- **WHEN** an older run manifest lacks capability authorization and resume is attempted after this slice
- **THEN** compilation adds the explicit denied capability identity required by v11 and the old manifest comparison refuses that identity change
- **AND** no pre-capability privilege is restored and historical records are not rewritten


#### Scenario: A start cannot substitute a different grant context

- **WHEN** a bundle compiled for one capability grant context is started with an operated realm whose capability grants differ
- **THEN** start refuses the inconsistent capability authority before creating a run or spawning a seat
- **AND** a matching context starts under its compiled holdings

## Decisions

Ruling 1 replaces both earlier H2 admission reconciliation and second M1's
subtractive-list exception. A deny list need not be a grant to be forbidden:
its author does not own the harness capability controls. No value-dependent
allowance remains. Closed grammar and explicit provenance survive because
engine controls still need a parse and authors cannot counterfeit ownership.
CQ1 remains: validate every grant first; unsupported valid restrictions refuse
requires, drop wants with OFF, and leave unused grants inactive and pinned.
Discarding a restriction or authoring an equivalent flag is not a remedy.
