# gate-boundary-policy

## Purpose

Which boundaries may hold a gate: decision 0021's gate law under the
boundary axis, the adapters' `hands.harness`, the bundle-pinned-script
reading for exec gates, and the run-time argv per boundary (decision
0046 ruling 4; decision 0043 rulings 2 and 3; decision 0021 rulings 2
and 7).

## ADDED Requirements

### Requirement: An adapter declares how its harness stands under the harness boundary
The adapter loader SHALL admit, beside `hands.workspace`, an optional
`hands.harness` object with two members, `gate` and `work`, each either
an argv fragment (an array of strings) or `{"unsupported": "<measured
reason>"}`; an absent member reads unsupported without a reason,
fail-closed, on the three-shape convention `tool_permissions` uses.
`gate` is the read-only fragment decision 0046 ruling 4 names — the
harness's own sandbox with reads only and one write door, the result
path — under which a model may judge; `work` is the harness's own
writable sandbox, under which a work-class site with hands writes the
worktree as its charter requires. `adapters/codex.json` SHALL declare
`gate` as `["--sandbox", "read-only"]` plus whatever the measured result
door needs, and `work` as `["--sandbox", "workspace-write"]`;
`adapters/claude.json` SHALL declare both as fragments measured against
the installed claude 2.1.x and never guessed; `adapters/dsh.json` and
`adapters/lanetally.json` SHALL declare no `hands.harness`. The loader
SHALL refuse any other key under `hands.harness`. An empty fragment is
a legal declaration: it says the adapter's driver argv already stands
in that mode — claude's driver carries `--permission-mode acceptEdits`,
the candidate `work` answer — and it is a measured declaration like any
other, recorded in the guide with what the mode denies and allows.

#### Scenario: codex declares both fragments
- **WHEN** `adapters/codex.json` is loaded
- **THEN** `hands.harness.gate` begins `--sandbox read-only` and `hands.harness.work` is `--sandbox workspace-write`

#### Scenario: claude's fragments are measured, not guessed
- **WHEN** `adapters/claude.json` is loaded
- **THEN** `hands.harness.gate` and `hands.harness.work` are each an argv fragment or `unsupported` with a measured reason, and the provider-adapters guide records the claude version they were measured against and what each denies and allows

#### Scenario: dsh and lanetally declare none
- **WHEN** `adapters/dsh.json` and `adapters/lanetally.json` are loaded
- **THEN** neither carries `hands.harness`, and each is refused at a `harness` gate exactly as it is refused at a boxed gate today

#### Scenario: An unknown member is refused
- **WHEN** an adapter declares `hands.harness.judge`
- **THEN** loading is refused naming the key and the two members the vocabulary admits

### Requirement: The gate law reads the boundary for sites that declare hands
`enforce_model_policy` SHALL apply decision 0021's gate refusals as
today under every boundary and, for a gate-class site that declares
hands, SHALL additionally rule by the boundary the bundle compiles
under: under `namespace`, `seatbelt` and `container` a model gate is
admitted as today and a boxed exec gate as decision 0043 ruling 3
reads; under `harness` a model gate is admitted only when every link of
its resolved chain declares `hands.harness.gate` as a fragment, and
refused otherwise naming the link, the provider and the missing
declaration; under `open` a model gate is refused naming decision 0046
ruling 4. A work-class site with hands under `harness` SHALL be refused
as a capability gap when a link declares no `hands.harness.work`
fragment. A gate-class site without hands has no box whose boundary
could be named and SHALL compile as it does today under every boundary
(decision 0046 ruling 4; decision 0021 rulings 2 and 7; decision 0041
ruling 3).

#### Scenario: A harness gate on a provider that declares the fragment is admitted
- **WHEN** a gate-class agent site with hands resolves to a trusted judging provider whose adapter declares `hands.harness.gate`, and the bundle compiles under `harness`
- **THEN** compilation succeeds

#### Scenario: A harness gate on a provider that declares none is refused
- **WHEN** the same site resolves to a provider whose adapter declares no `hands.harness`, and the bundle compiles under `harness`
- **THEN** compilation is refused naming the provider, `hands.harness.gate` and decision 0046 ruling 4

#### Scenario: A fallback link without the fragment refuses the chain
- **WHEN** a gate's chain has a first link whose adapter declares `hands.harness.gate` and a second whose adapter does not, under `harness`
- **THEN** compilation is refused naming the second link

#### Scenario: An open model gate is refused
- **WHEN** a gate-class agent site with hands compiles under `open`, whatever its adapter declares
- **THEN** compilation is refused naming decision 0046 ruling 4

#### Scenario: A harness work seat without a work fragment is refused
- **WHEN** a work-class agent site with hands resolves to a provider whose adapter declares no `hands.harness.work`, under `harness`
- **THEN** compilation is refused as a capability gap naming the provider and `hands.harness.work`

#### Scenario: A gate without hands is untouched
- **WHEN** an inline trusted model gate with a tool list and no hands compiles under `open`
- **THEN** it is admitted exactly as under `namespace`

#### Scenario: namespace is exactly today
- **WHEN** every shipped bundle compiles under `namespace`
- **THEN** every refusal and admission is what it was before this change

### Requirement: An exec gate under harness or open holds only for pinned bytes
An exec gate that declares hands, compiled under `harness` or `open`,
SHALL be admitted only when its command is the bundle's own pinned
script: after the `--` that ends the `{brokkr} driver exec` dispatch,
the first token that is a `./`-relative path resolves inside the
bundle's own root — or an ancestor root under composition — by a
comparison that canonicalises both sides and compares path components,
so a macOS `/private/var` spelling and a Windows `\` spelling compare
equal to their other spellings, and every token before it is a bare
interpreter name without a path separator; the tokens after it are
its arguments and are not judged, which is how the shipped ship gate
hands `{brokkr}` to its own script. A command with no such
script — a bare program, an absolute path, a `{brokkr}` verb, or a
`../` that escapes the root — SHALL be refused naming decision 0046
ruling 4 and decision 0021. A dialect validate or check step, whose
argv is the dialect's own and pinned by the dialect's content digest in
the run manifest beside the tool's declared name and version, SHALL be
admitted on the same cleared-environment and network terms, the run
marked unboxed all the same; the tool's binary is not pinned by digest,
which makes this the weaker of the two readings, recorded as such in
the proposal's D6 for the operator to confirm or refuse (decision 0046
ruling 4; decision 0042 rulings 1 and 4).

#### Scenario: The shipped verifier under open is admitted
- **WHEN** a bundle whose verify seat is `["{brokkr}","driver","exec","--","bash","./scripts/verify-seat.sh","{prompt_file}"]` with hands compiles under `open`
- **THEN** it is admitted

#### Scenario: A brokkr-external command under open is refused
- **WHEN** an exec gate with hands whose command is `["{brokkr}","driver","exec","--","true"]` compiles under `open`
- **THEN** compilation is refused naming decision 0046 ruling 4

#### Scenario: An escaping or absolute script is refused
- **WHEN** the script token is `./../outside.sh` or `/usr/bin/true` under `harness`
- **THEN** compilation is refused naming the token

#### Scenario: Platform spellings compare equal
- **WHEN** the comparison is given a root spelled `/private/var/b` and a script resolved as `/var/b/scripts/s.sh` on a host where `/var` links to `/private/var`, or a root `C:\b` and a script `C:/b/scripts/s.sh`
- **THEN** both are judged inside the root, and a script under a sibling root is not

#### Scenario: The shipped ship gate under harness is admitted
- **WHEN** the shipped ship seat, `["{brokkr}","driver","exec","--","bash","./scripts/ship-seat.sh","{prompt_file}","{brokkr}"]` with hands, compiles under `harness`
- **THEN** it is admitted, because the tokens after the script are its arguments and the `{brokkr}` among them names no command

#### Scenario: A dialect step under harness is admitted
- **WHEN** a fixture bundle with an artifact phase, its chief seated on a fixture provider that declares both `hands.harness` members as fragments, compiles under `harness` in a realm that declares a dialect
- **THEN** its synthetic validate and check steps are admitted and the bundle compiles

### Requirement: Every shipped bundle compiles under harness once the fragments are measured
Decision 0046 ruling 6 promises that after this slice a macOS operator
runs every shipped bundle, the review offices under their harness's own
sandbox. The work offices that declare hands — `chief-architect` and
`intake-sdd` — chain claude and codex, so the promise holds exactly when
`adapters/codex.json` and `adapters/claude.json` declare
`hands.harness.gate` and `hands.harness.work` as fragments. Every bundle
under `recipes/` and `bundles/` SHALL compile under `harness` on that
condition. If the measurement finds a claude mode that cannot be
declared as a fragment, the refusal SHALL name the adapter, the member
and the site, and the implementation SHALL report ruling 6's promise as
unmet for the operator to rule on — never widen the rule, seat another
provider or declare an unmeasured fragment to make the shipped bundles
compile (decision 0046 rulings 4 and 6; decision 0042's addendum,
ruling 1: a decision is amended only by a decision).

#### Scenario: The shipped bundles compile under harness
- **GIVEN** the codex and claude adapters declare both `hands.harness` members as fragments
- **WHEN** every bundle under `recipes/` and `bundles/` compiles under `harness` in a realm that declares the openspec dialect
- **THEN** each compiles, and each hands site's manifest `boundary` entry reads `harness`

#### Scenario: A measured gap is reported, not papered over
- **WHEN** the measurement declares claude's `work` member unsupported
- **THEN** `recipes/sdd`, `recipes/triage` and every bundle hiring the chief or the sdd intake refuse under `harness` naming `claude`, `hands.harness.work` and the site, and the implementation reports the unmet promise instead of amending the adapter, the roster or the rule

### Requirement: The argv of a site with hands follows the boundary and the class
At run time the engine SHALL compose the argv of every site with hands
from the boundary the bundle was compiled under, which is the realms map
the run was started with: under `namespace` exactly today's path — the
adapter's `hands.workspace` fragment with `{hands_mcp_json}`,
`{hands_args_toml}` and `{brokkr}` expanded, and `brokkr hands exec`
around an exec dispatch; under `harness` the adapter's
`hands.harness.gate` fragment for a gate-class site and
`hands.harness.work` for a work-class site, no workspace tool served and
no box built; under `open` no fragment of Brokkr's at all. Under
`harness` and `open` the site's `hands.network` and `hands.binds` stay
pinned in the manifest as declared and are enforced by nothing of
Brokkr's: the harness's own sandbox decides what the hands may reach,
which is the fact the *unboxed* rendering states. Under `harness` and
`open` an exec dispatch SHALL run with the environment
cleared to the box's own allow-list, its `./` script at its real path in
the bundle root, and, on Linux, inside a new network namespace through
`unshare`'s unprivileged form when `unshare` is on PATH and the kernel
permits it, otherwise with the network on; the record marks the run
unboxed all the same. An inline model site with hands under `harness`
or `open` SHALL be refused at compile naming the repair, because its
argv is the author's and carries the box's own tokens. `seatbelt` and
`container` are refused by `refuse_unboxable` before the engine
composes anything (decision 0046 rulings 1 and 4; decision 0043
rulings 1 and 3).

#### Scenario: namespace is byte-identical to today
- **WHEN** a boxed model site and a boxed exec site are composed under `namespace`
- **THEN** their argv equal what `hands_command` produced before this change, token for token

#### Scenario: A harness gate takes the read-only fragment
- **WHEN** a gate-class agent site with hands on codex is composed under `harness`
- **THEN** its argv carries `--sandbox read-only`, no `mcp_servers.brokkr` entry, and no `{hands_mcp_json}` expansion

#### Scenario: A harness work seat takes the work fragment
- **WHEN** a work-class agent site with hands on codex is composed under `harness`
- **THEN** its argv carries `--sandbox workspace-write` and no MCP server

#### Scenario: An open site takes nothing
- **WHEN** a work-class agent site with hands is composed under `open`
- **THEN** its argv is the adapter's base driver argv with the model and effort pins and nothing else

#### Scenario: The hands policy is declared, not enforced, under harness
- **WHEN** a site declaring `"hands": {"kind": "workspace", "network": false, "binds": []}` is composed under `harness`
- **THEN** its manifest `hands` entry still says `network` false, its argv carries no network switch of Brokkr's, and the run is rendered *unboxed*

#### Scenario: An exec script under harness runs unboxed with the environment cleared
- **WHEN** a boxed exec site is composed under `harness` on Linux with `unshare` on PATH
- **THEN** its argv does not begin with `brokkr hands exec`, names the script at its real path, clears the environment to the box's allow-list, and wraps the command in `unshare`'s unprivileged network namespace; with `unshare` absent the wrapper is skipped and the command still runs

#### Scenario: An inline model site with hands under harness is refused
- **WHEN** an inline seat whose command is a `{brokkr} driver claude` dispatch declares `hands` and compiles under `harness`
- **THEN** compilation is refused naming the seat and the repair

### Requirement: A judge under harness still delivers its result file
Under `harness` a gate-class model site SHALL be able to write exactly
its result file: the adapter's `gate` fragment, as measured, leaves that
one write door open — for claude through a permission rule scoped to the
result path or an equivalent measured mechanism, for codex through the
harness's own last-message capture into the result path or an
equivalent measured mechanism — and the prompt paragraph under `harness`
tells the seat how the file reaches the engine. An adapter whose
measured read-only mode leaves no such door SHALL declare
`hands.harness.gate` as `{"unsupported": "<measured reason>"}` and is
refused at a `harness` gate. The result file stays the only channel the
engine reads (decision 0046 ruling 4; decision 0043 as amended by the
boxed-marker fix).

#### Scenario: The measured door is recorded
- **WHEN** an adapter declares `hands.harness.gate` as a fragment
- **THEN** the provider-adapters guide records the measurement that showed a seat under it writing its result file and nothing else

#### Scenario: No door means unsupported
- **WHEN** a harness's read-only mode is measured to leave no way to write the result file
- **THEN** its adapter declares `hands.harness.gate` unsupported with that reason, and a `harness` gate on it is refused at compile
