## Purpose

Declare native provider powers and compose their controls from realm-authorized
holdings, keeping measured enforcement separate from unverified provider claims.

## ADDED Requirements

### Requirement: Native capability controls are adapter-owned evidence-bearing data

Adapters SHALL declare their native capability inventory and, per known
capability, how to enable it and disable it. Supported controls SHALL be
adapter data, following the existing tool_permissions pattern; impossible
controls SHALL use unsupported with a nonempty measured reason. Unknown
inventory or unverified controls SHALL be represented explicitly as unmeasured
with a nonempty reason, not as an empty supported map, unsupported-by-analogy
or a fabricated switch. Provider-native tool dialects SHALL reference these
adapter keys and SHALL NOT themselves confer authorization.

Both ON and OFF argv for every declared capability SHALL parse at adapter
load, even if the selected realm never uses that half. Invalid syntax,
unclassified effects, malformed values, unsupported separators and invalid
selection mappings SHALL refuse the load. A measured default is an explicit
state with its evidence scope, never an empty accidental argv.

#### Scenario: Both halves are validated before selection

- **WHEN** any known harness declares an ON or OFF argv containing a dangling `-c`, unknown option, misplaced `--`, synthetic positional prompt or malformed value
- **THEN** adapter loading refuses that declaration, including an unused ON or OFF half, before planning a seat
- **AND** a valid pair still loads; declaring a default retains its explicit evidence scope

#### Scenario: Selection serialization cannot alter its meaning

- **WHEN** a Claude selection would serialize WebFetch and WebSearch with `:` or another separator that parses them as one pattern
- **THEN** loading refuses the mapping rather than recording two denials
- **AND** malformed or ambiguous managed tool patterns refuse instead of relying on a different provider splitter

#### Scenario: Missing halves and reasons are rejected

- **WHEN** a native declaration asserts support without an ON or OFF disposition, or asserts unsupported/unmeasured without its reason
- **THEN** loading refuses naming the adapter, capability or inventory, missing field and complete cause
- **AND** no missing control becomes an empty success by default

#### Scenario: A measured default can be stated as the ON mechanism

- **WHEN** the Codex declaration identifies the measured cold-exec default as its ON mechanism and the disabled configuration as its OFF mechanism
- **THEN** the declaration explicitly records both dispositions and the evidence scope
- **AND** the engine does not invent a measured explicit ON configuration value

#### Scenario: Missing inventory is not a verified empty inventory

- **WHEN** an older third-party adapter has no native capability assessment
- **THEN** its native inventory is reported as unmeasured because the declaration is absent
- **AND** absence neither grants a capability nor establishes verified native denial
- **AND** a recognized provider already known to have an unheld native capability refuses compilation unless a valid denial plan can still be established and delivered

#### Scenario: An adapter declaration is not a grant

- **WHEN** an adapter lists supported web-search and a dialect binds it but the realm grants nothing
- **THEN** the seat receives its OFF control rather than its ON control

#### Scenario: CQ1 dropping a restricted want composes denial

- **WHEN** a valid native grant has a schema-valid restriction its selected provider binding cannot express and the seat only wants the capability
- **THEN** the capability is dropped with the complete CQ1 compatibility notice and the final argv uses its supported native OFF control
- **AND** neither its ON control nor an unrestricted replacement is composed; a measured unsupported OFF still refuses independently
- **AND** a grant unused after asks, subtraction or scope likewise composes only native OFF, without activating an unused ON/restriction configuration; load validation still parses both declared halves

### Requirement: Known native powers require a valid delivered denial or refusal

For every recognized provider known to have a native capability, compilation
SHALL establish a valid control plan for each unheld capability or refuse
before launch. Missing computed authority, absent or legacy assessments,
unreadable/malformed adapter data, unmeasured OFF and errors erased on any
security-relevant load path SHALL NOT become a successful no-control plan.
An adapter's omission SHALL NOT erase an already known native power. Refusals
SHALL name the seat, realm, provider, known capability and the complete cause,
including the failing source and load reason when applicable. No requests,
empty grants, an unused grant or legacy realm data SHALL bypass this check.
The final launch boundary SHALL refuse missing or undeliverable managed
controls rather than accept a plan that merely calls denial unmeasured.

#### Scenario: H1 missing Codex adapter data cannot restore default search

- **GIVEN** an inline Codex work seat requests nothing in a realm granting nothing
- **WHEN** its adapter root is absent, the root contains no Codex declaration, the declaration predates native metadata, or it omits the known web-search power from its inventory
- **THEN** each case either produces the exact final `-c`, `web_search="disabled"` pair from a valid engine plan or refuses with the complete missing-denial cause naming the seat, realm, codex and web-search
- **AND** no case launches with only an unmeasured record or no native control
- **AND** absence of an operator definition directory does not relax denial

#### Scenario: H1 a load failure remains a failure of denial authority

- **WHEN** the same no-ask seat encounters unreadable or malformed Codex adapter data, or an unrelated malformed adapter makes loading the adapter library fail
- **THEN** compilation refuses with the full source/load cause and the affected seat, realm, codex and web-search denial context unless valid independently loaded denial authority remains available
- **AND** an ignored load error cannot produce a launch without OFF

#### Scenario: H1 legacy realms and unmeasured OFF have no exemption

- **WHEN** the absent, legacy and malformed adapter cases are repeated without a realm map and with each supported v1–v5 realm, or a known native power explicitly declares OFF unmeasured
- **THEN** each unheld known power still requires delivered OFF or the complete named refusal
- **AND** an empty holding or a manifest notice alone cannot make the seat launchable

### Requirement: Codex web-search OFF uses the controller's measured fragment

adapters/codex.json SHALL declare web-search with the OFF argv pair "-c",
"web_search=\"disabled\"". Its evidence SHALL cite
.forge/tasks/controller-codex-web-search-switch-2026-09-21.json and limit
the live claim to cold codex exec on codex-cli 0.154.0. A seat without an
effective web-search holding SHALL receive that OFF pair whether it is
agent-backed or inline, boxed or unboxed, and whether it requests nothing,
loses a wants, is out of scope, or subtracts the capability.

#### Scenario: Cold denial and admission cover both boundaries

- **WHEN** otherwise valid Codex seats are composed with boxed workspace hands and without boxed hands, each first without and then with an effective web-search holding
- **THEN** both denied final cold argv contain the exact measured OFF pair
- **AND** both held cases use the adapter's declared ON mechanism without a conflicting OFF pair
- **AND** ordinary sandbox, model, effort, hands and result controls remain intact

#### Scenario: A realm-wide entry alone does not enable the seat

- **WHEN** the realm lists web-search but the seat does not ask for it, subtracts it, or falls outside its offices scope
- **THEN** each final Codex argv contains the OFF pair

#### Scenario: Box networking does not replace the switch

- **WHEN** workspace hands already has network false
- **THEN** an unheld web-search still receives the explicit native OFF pair because provider-server search is not confined by the workspace network

### Requirement: Eligible Codex resumes reimpose the capability control

An actual eligible Codex resume SHALL carry the current pinned capability
control in its final exec resume argv, including the OFF pair when web-search
is not held. Integration SHALL preserve the existing session, version,
sandbox, effort, accounting and boundary eligibility checks. The engine-owned
native control SHALL NOT by itself trigger incompatible-argv and turn every
eligible resume into a cold launch. Arbitrary -c passthrough SHALL NOT become
eligible merely to accommodate this control.

#### Scenario: Resume denial is proved on a resume rather than a fallback

- **GIVEN** a valid currently supported unboxed Codex resume offer, matching measured harness identity, sandbox class and all other eligibility facts
- **WHEN** the seat holds no web-search
- **THEN** the final plan actually invokes exec resume with the offered session and stdin prompt positional
- **AND** its argv includes "-c", "web_search=\"disabled\"" alongside the reimposed sandbox and effort controls

#### Scenario: A granted resume takes the declared ON disposition

- **GIVEN** the same eligible resume setup
- **WHEN** web-search is requested and held through an applicable realm grant
- **THEN** the actual resume argv uses the adapter's ON disposition with no conflicting OFF pair
- **AND** this deterministic composition does not claim live resumed enablement was measured

#### Scenario: Boxed resume eligibility is not widened

- **WHEN** a boxed Codex site whose existing resume shape is unsupported receives a resume offer
- **THEN** its existing eligibility refusal and cold fallback remain in force, and the resulting denied cold argv still contains OFF
- **AND** that cold plan is not counted as the proof of an actual resume
- **AND** this slice does not activate boxed resume to fill a test matrix

#### Scenario: Arbitrary config cannot ride the native-control exception

- **WHEN** an otherwise eligible resume includes a caller-authored arbitrary -c override
- **THEN** the existing unsafe-passthrough refusal remains
- **AND** only the engine-owned, resolved capability control receives the narrowly scoped treatment needed for composition

### Requirement: Neither inline arguments nor fallback can override native denial

All serving Codex paths SHALL enforce the same capability decision, including
inline driver declarations and executable model fallbacks. Every authored capability-bearing option SHALL be refused with its source
and option named, whether it enables or disables and whether granted; native ON/OFF fragment ordering SHALL
NOT accidentally decide authorization. Existing strict MCP configuration and
hands replacement SHALL remain intact.

#### Scenario: A concrete enable flag cannot override the realm

- **WHEN** an inline seat without a web-search holding supplies an argument or config setting that requests native search enablement
- **THEN** it refuses with a complete bounded reason naming the seat and authored option, without echoing its value
- **AND** it does not execute with two competing ON/OFF settings

#### Scenario: A fallback Codex candidate is also denied

- **WHEN** an authorized fallback selects Codex for a seat that holds no web-search
- **THEN** its cold or eligible resume command contains the OFF pair just as the primary Codex case does

### Requirement: Other provider declarations preserve the commissioned uncertainty

Claude's native WebSearch and WebFetch controls SHALL be declared through its
adapter and native dialects with their abstract capability names. Its existing
empty native tool list under strict MCP configuration SHALL be described as
adapter evidence, not a live enforcement measurement. Enabling a held native
tool SHALL retain the hands tool and strict MCP policy and SHALL not restore
all built-in tools.

DSH SHALL declare its native inventory/controls unmeasured because unsupported
mcp and tool_permissions do not establish absence of native egress.
LaneTally SHALL declare native inventory/controls unmeasured because they have
not been verified through that wrapper. Unmeasured declarations SHALL NOT
authorize a holding or claim verified denial; they SHALL remain distinct from
a known capability with a measured unsupported OFF control. Neither
unmeasured state SHALL permit a provider already known to have an unheld
native capability to launch without a valid denial plan; the refusal above
applies independently of whether the uncertainty was explicitly declared.

#### Scenario: Claude admits only held native tools beside hands

- **WHEN** a test Claude seat holds web-search but not web-fetch
- **THEN** the composed native tool selection admits only WebSearch for these capabilities, retains workspace MCP when applicable, and keeps strict MCP configuration
- **AND** with neither held the declared native search/fetch controls disable both
- **AND** deterministic argv assertions are labeled composition evidence, not live Claude proof

#### Scenario: DSH's unsupported fields do not prove a negative

- **WHEN** DSH's adapter is read with mcp and tool_permissions unsupported
- **THEN** its native capability assessment remains unmeasured with the commissioned reason
- **AND** neither compiler output nor doctor describes it as having no native egress or verified native denial

#### Scenario: LaneTally does not inherit Claude's evidence

- **WHEN** the LaneTally adapter is read beside Claude's supported native-control declarations
- **THEN** the wrapper's native assessment remains unmeasured and retains its own reason
- **AND** neither Claude's flags nor evidence of wrapper argv forwarding is counted as measured native enforcement

#### Scenario: An unmeasured control cannot satisfy a request

- **WHEN** a seat requires a capability whose selected native binding has unmeasured controls
- **THEN** compilation refuses naming the seat, realm, capability, provider and unmeasured reason
- **AND** if the request is wants it is dropped with that reason only when independent native denial remains valid; a known unheld native capability with unmeasured OFF refuses compilation instead
- **AND** no successful optional drop claims that an unknown inventory has been proven free of native egress

### Requirement: Every accepted native control reaches the final command

Every accepted native argv disposition, selection contribution and restriction
transport SHALL be represented in the final serving command after hands,
local permissions and boundary composition. A representation that cannot be
safely composed for that provider SHALL refuse during compilation, naming the
seat, realm, capability, provider and unsupported representation or conflict.
Successful compilation SHALL NOT record OFF, ON or an enforced restriction
that the launch drops. Controls SHALL compose once from the same typed
option/value structure used for compile admission, preserving current restriction arity, duplicate checks,
strict MCP and engine-owned hands. Managed argv and restriction transports
SHALL also parse in that provider grammar; a non-list token SHALL NOT be
forwarded verbatim without a classified position. Final validation SHALL judge
that structure, not search raw values for apparent option names.

The complete serialized command SHALL be parsed back immediately before
launch, after engine prefixes, wrapper options, model/effort, hands, local
permissions, restrictions, expansions and session/stdin positionals. Its
actual capability state SHALL equal the sealed plan: every held and denied
power, exact admitted tool subset, restrictions and required hands. Missing,
extra, contradictory or uninterpretable state SHALL refuse launch. Comparing
only an intermediate composer or finding a flag substring SHALL NOT suffice.
Compile admission SHALL preflight the complete shape available at compilation;
launch SHALL repeat the check after actual expansion and session selection.
The expected state SHALL come from sealed typed inputs, not from parsing the
output being checked. Private origins SHALL survive selected-candidate
projection, boundary composition, placeholder expansion and runtime input
assembly. Missing, malformed or non-reassembling origin records SHALL refuse;
model-authored input SHALL NOT replace them. A successful final check SHALL
produce a private checked command consumed without further argv mutation at
the serving spawn. Each replacement or fallback command needs its own check.
This applies to known-harness serving commands; availability probes and opaque
custom drivers SHALL NOT acquire capability guarantees from it.

Before flattening, the private plan SHALL distinguish contributions supplied
by authored commands, adapter templates, typed local permissions, engine hands
and realm-derived native controls. Ordered segments SHALL retain their source
kind and exact argv, including empty values and repeated equal bytes; engine
ownership SHALL NOT be recovered by matching values, flags or server names.
The independent expected state SHALL retain provider/harness identity, known
or unmeasured inventory, held and denied powers, admitted tools, structured
restrictions, local limits and required hands as applicable. A measured default
ON with no emitted argv SHALL still retain its expected holding; unmeasured
inventory SHALL NOT become a known empty one.

The new private decoder/reassembler SHALL be fallible. It SHALL reject missing
mandatory state, malformed shapes/types, unknown origin kinds and segments
whose ordered concatenation differs from the supplied argv. It SHALL NOT
repair them into empty or recover the richer origins from the old two-array
record. Reassembly proves byte correspondence, not permission or authentic
engine authorship: unit 4 supplies the protected runtime handoff, while later
units judge grammar, authored refusal and the complete final command. Unit 3
SHALL define and test the new primitives without activating that later refusal
or changing the existing serving consumers to require an unwired record.
No versioned public contract or manifest field is added for this private data.

#### Scenario: Unit 3 equal bytes retain different supplying origins

- **GIVEN** private records containing byte-identical argument contributions supplied independently as authored, template, local, hands and native segments
- **WHEN** the new primitive decodes and reassembles each record, including a record with repeated identical contributions
- **THEN** the flattened bytes match exactly while the decoded origin sequence remains different and preserves every occurrence and its order
- **AND** an authored copy remains authored; equal bytes neither grant authority nor prove that an untrusted record was sealed by the engine

#### Scenario: Unit 3 private decoding never defaults missing authority

- **WHEN** a record or mandatory expected-state field is absent or null, a segment/argv has the wrong type, an argv element is not a string, or an origin tag is unknown
- **THEN** decoding returns the complete bounded owning cause, distinct from a valid explicitly empty contribution or known empty state
- **AND** tests assert exact reasons without echoing argument payloads; the old authored/managed pair is not sufficient input to the richer decoder

#### Scenario: Unit 3 reassembly checks every argument in order

- **GIVEN** a well-formed private record and its supplied argument vector
- **WHEN** an argument is independently dropped, added, changed, or distinct segments are reordered so their concatenation differs
- **THEN** reassembly refuses with its exact full mismatch reason
- **AND** the valid record round-trips exact segments and argv, including empty strings; exchanging byte-identical contributions is distinguished by recorded origin values, not by a claim that byte equality detects their exchange

#### Scenario: Unit 3 expected state is independent of emission

- **GIVEN** sealed typed inputs for a particular candidate's held/denied native powers, admitted tools and restrictions, local limits and required hands
- **WHEN** the primitives produce contributions, including a measured default ON with no argv or an explicitly unmeasured inventory
- **THEN** the expected state equals an independent literal derived from those inputs and preserves the default holding or unmeasured reason exactly
- **AND** changing only emitted argv cannot rewrite that expected state; this is a primitive-state proof, while refusal of a semantically mismatched final serving command remains with units 13–15

#### Scenario: Final serialization is checked rather than trusted

- **WHEN** a valid plan loses OFF after a terminator, gains an extra tool, loses an empty restriction, changes a list separator, or duplicates an engine output-format option during final assembly
- **THEN** the final parse or exact-state comparison refuses before spawning, on cold, actual eligible resume and cold replacement
- **AND** valid complete commands parse back to exactly the planned ON/OFF, tools, restrictions and hands state

#### Scenario: Runtime preserves origin through candidate and boundary selection

- **WHEN** a production-compiled primary or fallback carries adapter templates, typed local permissions, hands and native controls through dispatch and expansion
- **THEN** their distinct private origins reassemble the actual arguments and the final check compares against the selected candidate's sealed state
- **AND** identical bytes supplied as authored input, missing origin segments, changed ordering or an authored override of the private record refuse before provider work

#### Scenario: A checked command cannot be changed before serving

- **WHEN** cold, eligible-resume, wrapper-child, DSH or rejected-rejoin replacement assembly adds its final engine and session arguments
- **THEN** each serving command obtains its own checked value after those additions and the spawn consumes that value without another argv edit
- **AND** any requested later mutation invalidates the check and requires recomposition and validation; an earlier cold check cannot certify a different resume or replacement

#### Scenario: Managed effects and expected state remain independent

- **WHEN** a managed deny pattern overlaps required hands, an include restriction contradicts a holding, or a setting has syntax but no bounded restriction meaning
- **THEN** compilation refuses an unrepresentable plan, without deriving a replacement expected state from the emitted command
- **AND** empty, absent and nonempty includes remain distinct; unknown DSH/LaneTally inventory is never converted into a verified empty inventory

#### Scenario: Codex managed argv has no unchecked path

- **WHEN** Codex managed ON or OFF contains `--`, `hello`, `--unknown-off=synthetic` or a dangling `-c`
- **THEN** its adapter load refuses, and a corrupted private plan independently refuses at final launch
- **AND** neither engine prefixes nor a named OFF record can make that command a denial

#### Scenario: H3 Claude argv denial is executable denial

- **GIVEN** Claude web-search OFF is declared as argv `["--disallowedTools", "WebSearch"]` instead of a selection contribution
- **WHEN** an unheld seat compiles and its actual cold or eligible resumed command is assembled
- **THEN** the final command denies WebSearch with that disposition composed into the authoritative tool lists, or compilation has already refused that representation with the complete reason
- **AND** accepting the declaration and emitting only selection is a test failure
- **AND** existing hands and local restrictions remain present without conflicting duplicate flags

#### Scenario: H3 a held restriction survives final composition

- **GIVEN** a synthetic supported native grant has a schema-valid nonempty restriction and a declared argv transport, with a supported ON disposition
- **WHEN** a Claude seat compiles with that holding through production admission and reaches actual cold and eligible-resume final command construction
- **THEN** both complete ordered commands match independent literal expectations containing the accepted ON control and the exact nonempty encoded restriction value
- **AND** the original structured restriction remains pinned, and the native capability cannot launch unrestricted
- **AND** the resumed case verifies the offered session and resumed shape, never a cold replacement
- **AND** rejecting this supported test representation is a failure of the positive proof; separate unsupported-form cases assert their full compile refusal

#### Scenario: H1 through H3 cover every serving path

- **WHEN** denial/admission, authored-configuration and accepted-control regressions exercise inline and agent-backed work/gate seats, primary and fallback links, panel members, sequence steps and selected/inherited bodies, boxed and unboxed, cold and eligible resumed
- **THEN** every supported path compiles its fixture through production admission and asserts the complete ordered final command against a literal and its effective ON/OFF/restriction disposition, while an unsupported path asserts its complete refusal
- **AND** the actual resume assertions verify the offered session and resumed command; boxed ineligible cold fallback stays a separate case
- **AND** an always-OFF mutation fails authorized ON assertions, and removing a delivered argv or restriction fragment fails its own final-command assertion
- **AND** this matrix tests the current gate launch paths without implementing slice-two gate capability policy

### Requirement: Prompt values cannot absorb a composed control

Option values SHALL retain their typed positions through composition and final
validation. In authored split syntax, an option-looking value whose boundary
is ambiguous SHALL refuse at compilation; an explicitly delimited value may
be accepted only if the provider grammar can preserve it without consuming a
managed option. Native OFF SHALL always occupy an effective option position.
The same rule SHALL apply to cold, resume and rejected-rejoin replacement
commands and to every admitted control transport.

#### Scenario: Second H3 the prompt-value reproduction refuses

- **WHEN** an empty-holding Claude command includes `--append-system-prompt --disallowedTools hello`
- **THEN** compilation refuses with the complete site/provider/argument-grammar reason identifying the ambiguous value for `--append-system-prompt` and the token `--disallowedTools`
- **AND** no command is emitted with tail `["--append-system-prompt", "--disallowedTools", "hello,WebFetch,WebSearch"]`, and no missing denial is recorded as delivered
- **AND** the shared LaneTally path obeys its declared grammar and gives the same semantic refusal

#### Scenario: Explicit prompt text is data rather than a tool-list option

- **WHEN** a supported prompt option has an unambiguous inert text value containing `--disallowedTools`, or an explicitly joined option-looking value admitted by that provider grammar
- **THEN** its complete final command preserves that text as one prompt value and emits real managed denial options in their own grammatical positions
- **AND** final duplicate, arity and capability checks do not mistake that value for a control; genuinely malformed forms assert their full compile refusal instead

### Requirement: Explicit restrictive tool lists retain their meaning

An engine-owned explicit native-tool include list SHALL remain a restriction, distinct from
an additive selection contribution. Absence, a present empty list and a present
nonempty list SHALL be distinct states through compilation and final command
composition. An accepted empty list SHALL remain effective as no built-in
tools; it SHALL NOT disappear into provider defaults. Explicit restrictions
SHALL NOT be widened by an additive native contribution or by argument order.
Compatible restrictions and native denial SHALL compose with engine-owned
hands once; an incompatible required holding or representation SHALL refuse
at compilation with its full provider/capability/restriction cause.

#### Scenario: Second H4 restrictive Read OFF survives cold and resume

- **GIVEN** only the shipped Claude web-search OFF declaration is changed to `["--tools", "Read"]` or independently `["--tools=Read"]`
- **WHEN** an otherwise valid empty-holding unboxed seat compiles and reaches cold and actual eligible-resume final construction
- **THEN** each complete command matches a literal retaining the effective `Read` include restriction and the independent WebFetch denial
- **AND** WebSearch is excluded by the include restriction; a final command containing only `--disallowedTools WebFetch` fails the proof
- **AND** the independent `["--disallowedTools", "WebSearch"]` OFF positive control still produces both managed native denials

#### Scenario: Second H4 explicit empty differs from no tools option

- **GIVEN** only Claude web-search OFF is changed to `["--tools="]`, or the equivalent supported split explicit empty value
- **WHEN** the same cold and eligible-resume commands are composed
- **THEN** each complete literal expectation contains an effective empty native-tool list plus the independent WebFetch denial; omitting the tools option fails
- **AND** a separate fixture without an explicit include restriction still receives its independently declared effective native OFF controls
- **AND** boxed engine-owned hands remain permitted independently of the empty built-in list; an incompatible restriction refuses rather than being weakened

#### Scenario: Admission and restriction cannot erase each other

- **WHEN** an explicit restrictive list conflicts with another managed control or a required effective holding
- **THEN** compilation refuses the full conflict with provider, capability and restriction named instead of unioning away the restriction or dropping mandatory denial
- **AND** a compatible held-ON case still reaches its literal final command, so always-OFF cannot pass as grant support
- **AND** every supported alias, variadic and repeated form is either parsed with the same restriction meaning or refused for its precise grammar/duplicate cause

### Requirement: Denial and admission have removal proofs and bounded live claims

Deterministic tests SHALL inspect final composed argv for inline and
agent-backed Codex seats, boxed and unboxed, with and without an effective
holding, plus actual eligible resumed denial/admission. Tests SHALL remove
the relevant OFF composition and observe the denial assertion fail, restore
it and pass; admission tests SHALL independently detect an always-OFF
implementation. Existing ineligible resume cases SHALL keep their reasons.
For the operator ruling and retained final-restriction obligations, the compiled launch matrix SHALL cover every
applicable cold/eligible-resume, boxed/unboxed, gate/work, primary/fallback,
ordinary/panel/sequence, inline/agent-backed and nested/inherited shape. Every
matrix row SHALL name an actual whole-command or whole-refusal assertion;
coverage of one dimension SHALL NOT imply an untested serving path. Expected
commands SHALL be independent ordered literals with canonical fixture values,
never sorted, deduplicated or obtained from the production composer under test.
These tests SHALL NOT substitute for a live provider measurement.

#### Scenario: Removing OFF is caught where it protects the launch

- **WHEN** each cold and eligible resume OFF composition is removed in turn
- **THEN** the corresponding final-argv denial assertion fails for the missing exact pair
- **AND** restoring the control restores the pass without changing the assertion or unrelated eligibility

#### Scenario: Always disabling is not a grant implementation

- **WHEN** the ON path is replaced with unconditional OFF
- **THEN** the held-capability cold and eligible resume assertions fail
- **AND** restoring the declared grant behavior restores them

#### Scenario: Second M3 restriction removal fails at final launch

- **GIVEN** the held supported nonempty restriction case has passed both compiled cold and actual eligible-resume literal final-command assertions
- **WHEN** restriction delivery is removed independently at each serving path while compile admission, holding, ON control and fixture remain valid
- **THEN** each corresponding whole-command equality fails for the missing restriction at the production final launch boundary
- **AND** restoring delivery restores each pass, with mutation, revision, test, intended assertion, observed failure and restored result recorded
- **AND** manual construction of a control plan, resolver argv or intermediate composer output cannot substitute for either final-launch failure

#### Scenario: Second H3 H4 and M1 removals detect lost semantics

- **WHEN** positional parsing, explicit nonempty restriction retention, explicit empty retention or authored capability-option refusal is independently removed
- **THEN** its exact compile-refusal or whole cold/eligible-resume command assertion fails for that lost protection and passes after restoration
- **AND** a build error, unrelated refusal or earlier failed test is not the required observation

#### Scenario: Delivery carries every measurement gap forward

- **WHEN** deterministic controls pass but no new controller measurements are supplied
- **THEN** delivery notes still list Codex live resumed denial and enablement, explicit ON values and other versions, Claude live controls, DSH native inventory/controls and LaneTally native inventory/controls as unmeasured and owed to the controller
- **AND** no failed, absent or unrun live check is described as passed

### Requirement: A DSH launch refuses authority before it reads its boundary

A DSH launch SHALL compose its plan before it checks its argument boundaries.
A site with no engine-computed authority, a plan carrying a native control DSH
does not consume, and an authored argv the DSH grammar cannot parse SHALL each
refuse at composition, with that refusal's reason, before the boundary check
reads any argument. The boundary check SHALL then inspect the composed argv,
the command that will actually launch, never the argv as handed over in its
place. DSH folds no control in today, so the two are the same bytes; a later
composition that adds controls stays covered by the check.

#### Scenario: The authority refusal wins over a boundary fault

- **GIVEN** a DSH seat argv that the boundary check alone refuses, such as `--model -` (the grammar reads the lone `-` as the model's value) or `--model --effort`
- **WHEN** the launch carries managed arguments, a tool selection, or no computed authority
- **THEN** it refuses with the exact composition reason (managed arguments or a tool selection not consumed, the missing authority, or the grammar's authored-parse refusal for `--model --effort`), and never the boundary's `--model needs a model id after it`
- **AND** the same `--model -` under the plan the engine writes for DSH, and either argv launched by hand, refuses with the boundary's own reason
- **AND** a well-formed `--model <id> --effort <level>` under that plan passes both guards and launches
- **AND** moving the boundary check ahead of composition fails this scenario's native-control assertion

## Decisions

Ruling 2 makes decision 0066 total: declared halves parse at load and the
complete final command parses back and matches the whole plan before launch.
Third S1/S2/C1/R1 are addressed by ruling 1 refusal, not a better authored-list
merge. C2 requires validating serialization as well as declarations. S3/C4/R4/R8
reject the unchecked Codex and engine-prefix paths. Managed Read/empty
restrictions still need compiled cold and actual-resume proofs; no authored
list is allowed to compete with them. C6/R10 retain the obligation to compile a
real held nonempty restriction and prove independent final-delivery removals.
Static command equality never becomes live provider enforcement evidence.
The DSH guard order is the operator's addendum of 2026-09-23 (rebuild unit
1b): the authority refusal wins, and the boundary check reads the composed
argv under ruling 2.

Unit 3 specification clarification (2026-09-23, based on b4839426): D5 and
rulings 1–2 require explicit origins before flattening and expectations derived
from typed inputs. The current hands suffix remains historical evidence only.
Reject string matching, saturating a missing boundary into an empty segment,
inferring the richer record from two arrays, and deriving expected state from
the command being checked. Equal-byte reassembly is necessary but cannot
authenticate a model-supplied origin label; the protected handoff is unit 4's
obligation, and final counterfeit refusal remains in 15.2. Define the fallible
primitive now without requiring an unwired format at today's serving doors.
The Rust interface may cross runtime/protocol internally; this is no new public
wire or manifest contract. No authored-refusal activation, unused-half load
validation, grammar campaign or final checked-command implementation is moved
into unit 3. Decision 0066 remains proposed.
