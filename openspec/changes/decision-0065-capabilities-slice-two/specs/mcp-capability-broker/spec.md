## Purpose

Serve a realm-held MCP capability through Brokkr's own harness child outside
the seat's namespace, preserving the box and the realm's exclusive authority.

## ADDED Requirements

### Requirement: MB1 the harness launches exactly the engine's brokers

As R1 says, "a granted mcp capability is served by `brokkr broker serve`,
listed in that same engine-written MCP config beside the hands server, one
server per held capability." The engine SHALL derive the set from the selected
site and candidate's sealed holdings. The harness SHALL start the broker;
the broker SHALL start the dialect's real server with stdio. Neither SHALL
detach from the attempt's process group. The engine SHALL not start an
independent unsupervised sibling or ship a third-party capability server.

Each server SHALL be named exactly `cap-<capability>`, with no truncation,
normalization or suffix inferred from model text. Hands remains `brokkr`.
Unrepresentable or colliding identifiers SHALL refuse before launch.
The final check SHALL parse the actual complete command/configuration and
compare executable, arguments, server names, permitted tools and isolation
against independently sealed typed intent, not against a recomposition of
the same output. An authored equal-byte server definition SHALL still refuse.

#### Scenario: Two holdings produce three servers

- **WHEN** a namespace seat holds library-docs and issue-tracker through two admitted MCP dialects
- **THEN** the engine supplies exactly brokkr, cap-library-docs and cap-issue-tracker; each cap entry invokes the current brokkr executable with broker serve and only its own sealed plan
- **AND** dropping either holding removes that server; an unused grant adds no server

#### Scenario: A counterfeit or missing server refuses

- **WHEN** final assembly adds, removes, duplicates or changes any engine server, including a server with the right name and wrong executable or plan
- **THEN** launch refuses "provider '<provider>' final MCP configuration is not the engine's sealed server set" under the owning site prefix
- **AND** copying valid server argv into an authored command still receives slice one's authored-option refusal, never engine provenance

#### Scenario: A server name cannot be shortened into another holding

- **WHEN** cap plus the exact capability identifier is not representable under the measured adapter's server-name grammar
- **THEN** compilation refuses with cause "MCP server name 'cap-<capability>' is not representable by provider '<provider>'"
- **AND** no truncation, alias or hash-based rename is substituted

### Requirement: MB2 only an actual namespace hands site may hold MCP

R2 says "an mcp grant is admitted only at a site inside a Brokkr box
(boundary `namespace`); every other site refuses it with an exact reason."
After structural grant validity, request/subtraction and office scope,
an otherwise applicable MCP request SHALL refuse outside namespace even
when wanted. Within namespace it SHALL require workspace hands and
network false, a model harness with measured strictness, and supported
adapter MCP carriage. Exec and opaque custom drivers SHALL not gain a
capability channel from being boxed.

Boundary/hands/network eligibility is a hard site refusal under R2.
Native-tool write containment is also mandatory: inability to protect the
plan/ledger/artifacts refuses with "MCP capability '<capability>' cannot
protect broker evidence from this provider's native tools". Provider carriage is ruling 5 compatibility: requires refuses, wants drops
with its exact reason, provided SI2 and native OFF can still hold.
The former realm-wide unbuilt-kind refusal SHALL remain until U9.
After U9, valid unused grants stay pinned and inactive; they need no site
or server, consistent with D4. There is no implicit grant or auto-fallback
to another dialect.

After scope and nonempty tools, MB2's hard site checks run before GP1's
class compatibility; GP1 then runs before ordinary provider/connection
compatibility. SI2's independent ambient-isolation check still applies when
a holding drops. New hard-refusal causes, in order, SHALL be:
"MCP capability '<capability>' requires boundary 'namespace'; this site uses
'<boundary>'"; "MCP capability '<capability>' requires workspace hands at
this site"; "MCP capability '<capability>' requires workspace hands with
network false"; "MCP capability '<capability>' requires a model harness with
an engine-owned MCP channel". The evidence-protection check follows those four hard checks. For a dialect
with secrets, a separate measured read-isolation check SHALL then prove that
workspace hands and model-native tools cannot read the bound store or the
child's secret-bearing process state. Failure SHALL hard-refuse with
"MCP capability '<capability>' cannot protect its secret bindings from this provider's native tools". Read-only write confinement and file mode 0600 SHALL
NOT count as read isolation. Secret-free holdings need no secret-read proof. Existing start-time namespace availability
refusals remain in force on both supported hosts.

#### Scenario: Every other boundary refuses by name

- **WHEN** research (office researcher) in private has an applicable library-docs MCP request under harness, open, seatbelt or container
- **THEN** each refuses "seat 'research' (office 'researcher') in realm 'private': MCP capability 'library-docs' requires boundary 'namespace'; this site uses '<boundary>'", substituting the actual boundary
- **AND** both requires and wants obey this refusal; boundary widening is a later decision

#### Scenario: A namespace label alone is insufficient

- **WHEN** that namespace site has no workspace hands, hands with network true, or a boxed exec rather than model harness
- **THEN** it refuses the corresponding complete cause above under the same site prefix, in the stated order
- **AND** a valid model/hands/network-false control remains eligible subject to strictness, carriage and every independent grant rule

#### Scenario: Carriage honors request strength after safety checks

- **WHEN** the otherwise eligible strict provider test-provider declares MCP carriage unsupported with reason "engine MCP loading is unavailable"
- **THEN** requires refuses "seat 'research' (office 'researcher') in realm 'private': requires capability 'library-docs' through dialect 'docs-mcp', but provider 'test-provider' cannot carry MCP (engine MCP loading is unavailable)"
- **AND** wants records "seat 'research' (office 'researcher') in realm 'private': dropped wanted capability 'library-docs' through dialect 'docs-mcp' because provider 'test-provider' cannot carry MCP (engine MCP loading is unavailable)"
- **AND** the dropped candidate starts no broker and independently proves native denial and strict empty/hands-only MCP

#### Scenario: Old exclusion and subtraction still win before site eligibility

- **WHEN** the realm grants only another office, grants empty tools, or the seat subtracts its ask
- **THEN** D4's exact scope/empty-tool/subtraction outcomes remain; no MCP holding or broker is invented
- **AND** invalid grant data remains a realm-wide refusal, even unused; ungranted native equivalents stay OFF

#### Scenario: Nonempty restrictions remain deferred for MCP

- **WHEN** a structurally valid MCP grant has nonempty restrictions and an otherwise eligible site requests it after U9
- **THEN** requires refuses and wants drops with "MCP nonempty restrictions are deferred to the restriction-transport slice", in GP1's full diagnostic forms
- **AND** an unused valid grant remains pinned/inactive; a child claiming support never bypasses D11

#### Scenario: Native tools cannot forge broker evidence

- **WHEN** an otherwise eligible namespace provider has no proved write confinement for its native tools
- **THEN** the hard refusal is "MCP capability 'library-docs' cannot protect broker evidence from this provider's native tools"
- **AND** neither wants nor a permission prompt replaces the confinement proof

#### Scenario: Read-only native tools cannot qualify a secret-bearing holding

- **WHEN** U0's canary control proves a native tool can read the store or child process environment despite read-only workspace settings
- **THEN** the secret-bearing holding hard-refuses with MB2's secret-protection cause for both requires and wants
- **AND** a secret-free control remains eligible under the other rules; hands and every supported serving shape have independent read-isolation evidence

### Requirement: MB3 the broker offers only granted tools and records refusals

The broker SHALL load an engine-created, identity-bound private plan naming
one capability, dialect identity, connection, exact tools, empty restrictions,
retention policy and attempt ownership. It SHALL not read fresh realm grants
or accept authority from tool arguments, server output, recipe files or an
unbound path in the workspace. Plans and ledgers SHALL be outside writable
seat reach; a changed, missing or forged plan SHALL refuse before contacting
the child, with "broker plan is not bound to this attempt".

Before any secret lookup or child spawn, the broker SHALL select a protected
working directory and resolve the executable and startup inputs outside
seat-writable reach, including aliases and replaceable ancestors. An executable
inside that reach SHALL refuse "MCP server launch resolves inside seat-writable reach"; unprotected scripts, configuration, package/plugin loading or an
unprovable startup arrangement SHALL refuse "MCP server startup inputs are not protected from seat writes". Direct argv and a server-reported version alone
are insufficient. No heuristic interpreter-flag parser can grant an exception.

Only MCP initialization and the measured protocol operations necessary for
listing/calling granted tools SHALL be proxied. Tool listing SHALL expose
exactly the allowed subset, preserving schemas as data; pagination SHALL not
leak excluded tools. A call outside that set SHALL be refused before forwarding
with "tool '<tool>' is not granted to capability '<capability>'". Dynamic
catalog changes SHALL not widen it. A granted tool missing from the child's
complete listing SHALL refuse "MCP server does not offer granted tool
'<tool>'"; no partial substitute toolset is advertised as the full holding. Unrelated resource/prompt operations or
server requests for sampling, roots, credentials or additional model actions
SHALL not create another authority channel; unadmitted methods refuse
"MCP method is not admitted by this broker". The admitted protocol is MCP 2025-06-18 over newline-delimited JSON-RPC 2.0.
Initialize, initialized notification, ping, tools/list, tools/call and local
cancellation handling are the only admitted operations; no server-initiated
request is forwarded to the harness. Unknown protocol versions refuse "MCP
protocol version is not supported by this broker". Bounds are 1 MiB per
request, 8 MiB per response/canonical artifact, JSON depth 64, 1,024 catalog
tools over at most 32 pages, one forwarded call at a time per broker, 30 s
initialize/list and 120 s per call (all also bounded by the attempt deadline).
Cancellation grants 2 s for child settlement before existing attempt cleanup.
An over-limit input returns "MCP request exceeds the broker limit"; catalog
count/page/cursor failures return "MCP tool catalog exceeds the broker limit";
a second concurrent call returns "MCP broker already has an active call".
Responses exceeding bounds return the response-limit cause below; no truncated
response is delivered as complete. These are slice limits, not configurable
new grant keys. Responses SHALL match the exact typed JSON-RPC request ID,
method and session phase; string and numeric IDs are distinct. Unknown, wrong,
duplicate or late responses SHALL refuse "MCP server protocol is invalid" with a
typed correlation failure. Timeouts are absolute deadlines unaffected by ping,
progress or notification traffic. Cancellation/timeout ends an uncertain child
session; its external action SHALL NOT be replayed. Non-response frames share
a fixed 1 MiB cumulative byte budget per initialization/list or call operation;
exhaustion SHALL return "MCP server response exceeds the broker limit".

#### Scenario: Filtering is enforced at both list and call

- **WHEN** the fake child offers lookup and admin but the realm permits only lookup
- **THEN** listing returns exactly lookup and a direct admin call returns "tool 'admin' is not granted to capability 'library-docs'"
- **AND** the child's call log contains no admin invocation, while the broker ledger records the refused attempt without marking admin granted

#### Scenario: A result cannot grant a tool or a method

- **WHEN** child output advertises a new tool or requests sampling, or a model supplies a replacement dialect/plan in a call
- **THEN** no authority changes; an ungranted call or unadmitted method returns the corresponding exact refusal
- **AND** the original admitted call still has its checkpoint and the returned content remains DATA

#### Scenario: Version and protocol errors do not degrade to passthrough

- **WHEN** the child initialize response reports a server version different from the dialect's version
- **THEN** the broker refuses "MCP server version does not match the dialect's pinned version" before offering a tool
- **AND** malformed protocol, output-limit and timeout failures return respectively "MCP server protocol is invalid", "MCP server response exceeds the broker limit" and "MCP server call timed out", with typed ledger outcomes and no raw stderr echoed
- **AND** the literal protocol/limit contract above is tested at each boundary; no fallback to unbounded proxying is admitted

#### Scenario: Protocol and resource bounds refuse exactly

- **WHEN** a caller selects another protocol version, exceeds the request/depth bound, or calls concurrently while one call is active
- **THEN** it receives respectively the unsupported-version, request-limit or active-call cause above, without forwarding that call
- **AND** catalog overflow, more than 32 pages or a repeated cursor returns the catalog-limit cause; exact-limit positive controls still work

#### Scenario: Startup cannot execute a seat replacement with secrets

- **WHEN** a seat substitutes a repository-relative executable, interpreter script, startup config, plugin, package or ancestor before launch
- **THEN** launch refuses the corresponding executable or startup-input cause before reading or injecting a secret
- **AND** a protected installed fake-server control starts with a private working directory and the permitted binding; serverInfo.version alone never admits the replacement

#### Scenario: Correlation and deadlines survive hostile protocol traffic

- **WHEN** a child sends a wrong-type ID, duplicate/late response or response for the wrong phase
- **THEN** the broker ends that session with the exact protocol cause, records the unresolved call without a digest, and forwards no mismatched result
- **AND** continuous pings or progress cannot extend the fixed deadline; notification-budget exhaustion has the exact response-limit cause, and an uncertain cancelled action is never retried

### Requirement: MB4 only the real server receives broker secret bindings

Per R1, "never in argv, never in the harness's environment": the broker SHALL
resolve only dialect-declared names from the operator's decision 0012 store
at child spawn. Dialect argv SHALL execute directly, without interpolation
or a shell added by Brokkr. The actual child environment alone receives
values through the existing single plaintext injector, refactored for reuse
if needed without adding a second accessor call. The harness, hands box,
manifest, broker plan and argv SHALL contain no resolved value. Dialect secret
names SHALL be removed from the inherited harness and broker environment;
they are resolved afresh only for the real child. A measured harness auth
variable colliding with such a name SHALL refuse the holding with cause
"MCP secret binding '<name>' collides with harness authentication", using
GP1's required/optional form. A secret-store path inside workspace hands'
readable reach SHALL refuse launch with "MCP secret store is reachable by
workspace hands"; owner-bound alias checks SHALL not trust lexical paths.

Missing, disallowed, unreadable or invalid bindings SHALL refuse child spawn
using decision 0012's typed causes and safe name-only diagnostics. Dialect
egress SHALL meet the operator's existing binding minimum under 0036;
a capability class or gate office does not grant secret clearance. The
compatibility cause is exactly "MCP dialect '<dialect>' has egress '<class>'
below binding minimum '<minimum>'", in GP1's required/optional forms.
The child SHALL receive a bounded base environment plus declared bindings,
not inherited harness credentials. Known literals and common encodings SHALL
be masked before any response, stderr, ledger, artifact or diagnostic leaves
the broker. Masking SHALL preserve protocol structure and work across stream
chunk boundaries. A masked identifier that can no longer be matched safely
SHALL refuse protocol handling rather than use raw bytes.

#### Scenario: Secret values stop at the child environment

- **WHEN** the fake MCP server needs DOCS_TOKEN and the operator store supplies it
- **THEN** the child receives DOCS_TOKEN; captured harness/broker argv, harness environment, plans, journal and artifacts contain none of its raw or decision-0012 encoded values
- **AND** an applicable argv secret-reference connection refuses under SC1 before spawning, with "MCP connection argv cannot contain secret references; declare environment bindings in secrets"

#### Scenario: Undeclared or missing secrets do not fall back to ambient values

- **WHEN** DOCS_TOKEN is absent from the store but exists in the harness's ambient environment
- **THEN** child spawn receives the existing missing-binding refusal; no ambient value satisfies it
- **AND** a dialect declaring a denied injection name receives decision 0012's exact denied-name refusal before any child starts

#### Scenario: Dialect secrets meet their own route clearance

- **WHEN** a secret-bearing MCP dialect has uncontracted egress and the operator's binding minimum is contracted
- **THEN** requires refuses and wants drops with "MCP dialect 'docs-mcp' has egress 'uncontracted' below binding minimum 'contracted'"
- **AND** the harness's higher route clearance does not authorize the dialect's bindings

#### Scenario: Ambient credentials and exposed stores are not broker channels

- **WHEN** DOCS_TOKEN is exported by the operator as well as stored, and a broker holding declares that name
- **THEN** the harness and broker inherit no DOCS_TOKEN; only the real child receives the store's binding
- **AND** a measured auth-name collision receives the exact collision compatibility cause, while a store inside hands reach receives the exact store-path launch refusal above

#### Scenario: Known encodings are masked before persistence and delivery

- **WHEN** the fake child returns a bound literal, its common encodings or a split-stream occurrence in result data or stderr
- **THEN** every delivered/persisted surface contains the declared redaction instead, and the retained digest is over the masked bytes
- **AND** a real compiling removal of masking makes the full leak-scan test fail; an arbitrary transformed secret remains outside 0012's claimed guarantee

### Requirement: MB5 attempt cleanup covers broker and child

The broker and child SHALL remain in the harness attempt's process tree,
with cancellation, timeout, normal shutdown and failed initialization using
the existing #403 ownership and cleanup machinery. No broker daemon,
detached server or separate untracked engine group SHALL be introduced.
Each new child failure SHALL leave an attributable ledger outcome even when
its result cannot be delivered. Existing #403 residuals SHALL remain named;
a process-group assertion SHALL not claim cgroup guarantees.

#### Scenario: Cancellation leaves no owned child running

- **WHEN** a fake server blocks and the attempt is cancelled or times out
- **THEN** the broker and its owned fake child terminate under the same attempt cleanup, and engine folding records the interrupted call
- **AND** a failed initialization cleans up the child without exposing a tool; real Linux evidence is required for namespace enablement

## Decisions

R1's harness child is adopted because engine.rs:4434 and hands.rs:1119 show
that ownership today; an engine-spawned sibling would require new supervision
and transport. Proposed 0077 records the difference from 0065's literal
"launched by the engine".

R2's explicit refusal takes precedence over a wants drop for a site that is
not safely boxed; ordinary unsupported carriage still follows ruling 5.
The box here is 0043's tool-call box, not the proposed whole-harness seat box.
No 0072 boundary or full-access behavior is smuggled into this change.

Robustness A/B/E are adopted at MB2–MB4, their earliest owners: native
read-only access is not secrecy (hands.rs:1–13, decision 0043), process.rs:173–178
otherwise inherits the workdir, and a method result needs typed request
correlation (0071 rulings 3, 8, 9). U0 may prove a harness unsupported; this
proposal adds no whole-harness box, executable installer or general attestation
service. The protected operator installation remains trusted code.

Nonempty restrictions remain inexpressible under D11, even if an MCP child
claims to understand them. The realm's empty restriction is still the sole
restriction authority. URL transport, native response retention and wider
boundaries require later commissioned work, not speculative runtime code.
