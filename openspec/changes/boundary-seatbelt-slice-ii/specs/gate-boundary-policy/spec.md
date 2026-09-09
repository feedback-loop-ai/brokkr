## MODIFIED Requirements

### Requirement: The gate law reads the boundary for sites that declare hands
`enforce_model_policy` SHALL apply decision 0021's gate refusals as
today under every boundary and, for a site that declares hands — as
one hands law that runs before its class read, so no work-class early
return bypasses it and no adapter is read for an inline site (design
DD22) — SHALL additionally rule by the boundary the bundle compiles
under: under `namespace`, `seatbelt` and `container` a model gate is
admitted as today and a boxed exec gate as decision 0043 ruling 3
reads — the compile defines the identity now, and a run under
`seatbelt` requires the complete policy and peer-status activation defined
by boundary-availability, with the unbuilt slice (ii) fence retained until
then, and a run under `container` still refuses until slice (iii),
availability being boundary-availability's rule and not this one's; under
`harness` a model gate is admitted only when every link of its resolved chain
declares `hands.harness.gate` as a fragment, and refused otherwise
naming the link, the provider and the missing declaration; under `open`
a model gate is refused naming decision 0046 ruling 4. A work-class site
with hands under `harness` SHALL be refused as a capability gap when a
link declares no `hands.harness.work` fragment. A gate-class site
without hands has no box whose boundary could be named and SHALL compile
as it does today under every boundary. Ruling 4's own binding —
`model_policy_tests.rs`, "a `harness` gate on codex admitted, on dsh
refused" — SHALL be pinned in that file against the shipped adapter
library, the files under `adapters/` as they stand, and not against a
fixture provider shaped like either; the arms of the chain rule stay on
fixture providers, as that file's charter requires (decision 0046
ruling 4; decision 0021 rulings 2 and 7; decision 0041 ruling 3).

#### Scenario: A harness gate on a provider that declares the fragment is admitted
- **WHEN** a gate-class agent site with hands resolves to a trusted judging provider whose adapter declares `hands.harness.gate`, and the bundle compiles under `harness`
- **THEN** compilation succeeds

#### Scenario: A harness gate on a provider that declares none is refused
- **WHEN** the same site resolves to a provider whose adapter declares no `hands.harness`, and the bundle compiles under `harness`
- **THEN** compilation is refused naming the provider, `hands.harness.gate` and decision 0046 ruling 4

#### Scenario: A harness gate on the shipped codex adapter is admitted
- **WHEN** a fixture gate-class agent whose agent file declares hands and whose chain is `astra` alone compiles under `harness` against the shipped adapter library — `adapters/` as it stands in the tree — in `model_policy_tests.rs`, beside the pins that compile the shipped codex adapter at a boxed gate
- **THEN** compilation succeeds and the manifest's agents entry pins the shipped codex adapter's digest: decision 0046 ruling 4's "a `harness` gate on codex admitted", read against the file it names

#### Scenario: A harness gate on the shipped dsh or lanetally adapter is refused as at a boxed gate
- **WHEN** a fixture gate-class agent with hands whose chain is one dsh model alone, and again one whose chain is one lanetally model alone, compiles against the shipped adapter library under `namespace` and then under `harness`, in the same file
- **THEN** each is refused under both: under `namespace` by decision 0021 ruling 2's trust-tier refusal as today, and under `harness` by the hands law, which is the first statement of the gate policy (design DD22), naming the link, the provider and the missing `hands.harness.gate` — the two texts differ, and that is the order the record rules (proposal D33)

#### Scenario: A fallback link without the fragment refuses the chain
- **WHEN** a gate's chain has a first link whose adapter declares `hands.harness.gate` and a second whose adapter does not, under `harness`
- **THEN** compilation is refused naming the second link

#### Scenario: An open model gate is refused
- **WHEN** a gate-class agent site with hands compiles under `open`, whatever its adapter declares
- **THEN** compilation is refused naming decision 0046 ruling 4

#### Scenario: A seatbelt gate is admitted at compile
- **WHEN** a gate-class agent site with hands compiles under `seatbelt`, and again under `container`
- **THEN** compilation succeeds exactly as under `namespace` and the manifest pins the word, whatever the compiling machine holds

#### Scenario: Compile admission is not a peer ruling
- **WHEN** a Seatbelt gate compiles while overlay, mask, hooks-view/peer or lifetime prerequisites are unresolved, including a candidate passing only an unsigned-commit smoke test
- **THEN** compilation pins the realm word as before, but start retains the unbuilt slice (ii) refusal before any journal row or seat; compile admission does not authorize a weaker runtime gate

#### Scenario: A harness work seat without a work fragment is refused
- **WHEN** a work-class agent site with hands resolves to a provider whose adapter declares no `hands.harness.work`, under `harness`, in a bundle that seats no gate and binds no secret, so nothing but the hands law stands before the gate law's work-class early return (design DD22)
- **THEN** compilation is refused as a capability gap naming the provider and `hands.harness.work`

#### Scenario: A gate without hands is untouched
- **WHEN** an inline trusted model gate with a tool list and no hands compiles under `open`
- **THEN** it is admitted exactly as under `namespace`

#### Scenario: namespace is exactly today
- **WHEN** every shipped bundle compiles under `namespace`
- **THEN** every refusal and admission is what it was before this change

### Requirement: An exec site with hands under harness or open is admitted only for pinned bytes
An exec site that declares hands — gate or work, the class deciding
only whether the site may hold a gate (proposal D32) — compiled under
`harness` or `open`, SHALL be admitted only when its command is the
bundle's own pinned script, checked by construction on the raw command, before
`expand_command` erases the `./` spelling: after the `--` that ends the
`{brokkr} driver exec` dispatch, zero or more bare interpreter names —
no path separator, no leading `-`, so `bash -c '…'` is refused — then
exactly one script token, then arguments. The script token is
`./`-relative, every component after `./` a plain name — no `..`, no
`.`, no empty component, no `\`, no drive or UNC prefix — and, joined to
the directory of the layer that declared the seat (the bundle's own
directory, or the ancestor's that wrote the seat under composition), a
regular file at compile by `metadata`, following a symlink as the
manifest walk does, and not a key the walk skips (`realms.json`,
`dialects/…`), the exclusion being one function shared with the walk.
That directory is the one the compiler already expands `./` against and
the one the manifest walk digests, so a token that passes is pinned by
the manifest of the layer that declared it, and no path is canonicalised
and no two spellings are compared (design DD9).

This rule SHALL always be stated as an integrity check over the bundle's
own bytes. It is an execution guarantee only under a boundary that
supplies the filesystem and the `PATH`: `namespace`, `seatbelt`,
`container`. Under `harness` and `open` the interpreter is unpinned and
resolved through an inherited `PATH`; here the rule is careless-bundle
defence only, not a defence against a hostile seat, and the execution
guarantee does not hold (decision 0049 ruling 3; decision 0046 ruling 4).
Closing a list of interpreter names would not pin their identities:
anything earlier on `PATH` can answer to a name. Refusing option tokens
such as `-c` does not exclude shell-source interpretation by every
interpreter: `powershell.exe` takes its first positional argument as a
command string without any option token to refuse (decision 0048's
corrected security ruling). Decision 0049 names Linux first-class,
macOS supported and Windows best-effort; where operating-system behaviour
defeats a stated guarantee, the guarantee SHALL be named as not holding
there rather than pursued. Decision 0046's addendum still refuses
`container` at start until slice (iii), and Seatbelt until its complete
policy and peer prerequisites are resolved and implemented. Availability
is a runtime requirement, never a compile-time tool probe.

The compiler SHALL additionally refuse the measured startup-character set
`*`, `?`, `[`, `]`, `{`, `}`, `(`, `)`, `'`, `"`, `\`, `~`, CR (`\r`,
U+000D), LF (`\n`, U+000A) and backtick (U+0060) in every
script component after `./`, in this same grammar walk before lookup or
expansion, on every platform (decision 0048, operator security ruling
2026-09-07; decision 0046 ruling 4). Wildcards and brackets select glob
matches; braces expand alternatives; parentheses also trigger MSYS
`globify`; quotes and backslash change quoting or escaping; tilde
introduces home expansion; CR and LF split startup arguments without
triggering Rust's automatic quoting. The backtick closes the one route the
review traced through `powershell.exe` to a mutable sibling, a small
engineering choice with no guarantee attached (decision 0049 ruling 3).
The set is closed against the interpreters that have been measured only;
a different interpreter may reinterpret other bytes. The execution
guarantee rests on the boundary that supplies the filesystem and `PATH`,
not on this set. No survey of further interpreters, additional refusals
beyond the backtick, or parent-process attempt to defeat shell startup
handling belongs to this correction (0049 ruling 3).
Each character SHALL refuse even unmatched
or where its position would suppress expansion. The refusal SHALL name
the first offending component and its first refused character in token
order, decision 0048 and decision 0046 ruling 4. The verdict SHALL depend
on the bundle token's bytes, never the host, interpreter, environment or
presence of a matching sibling. This narrows admission only: manifest
keys and the canonical directory pin SHALL retain exact filename bytes,
and other bundle files and later unjudged arguments keep their existing
meaning. Other bytes, including spaces and tabs, remain admissible
without a claim that every interpreter preserves them literally;
argument encoding alone proves no interpreter's parsing.

The verdict is a compile fact, and it is re-derived where it matters:
at every unboxed exec dispatch spawn the engine SHALL re-walk the script's containing
directory and its descendants with the walk the compiler already
performs, against the declaring layer's compiled file map — the leaf's
`files` map, or the map hashed into an ancestor's compose digest — and
SHALL refuse the dispatch, spawning nothing and
journaling the attempt's failure naming the layer and the first key
that differs, when any pinned file moved, went missing or appeared.
This narrows design DD9 as proposed decision 0048 records: the script's
directory includes its helpers, while an `init .` realm's journal,
results and implementation files outside that directory may change.
A root-level script selects the whole layer. Filename components retain
their exact UTF-8 bytes, with `/` only between components; filenames
that cannot be represented without loss are refused. Helpers outside
the selected directory and the interval between the re-walk and the
`exec` are outside the check, which the guide states. The namespace and Seatbelt boxes keep no
unboxed re-walk: a boxed gate is admitted by its walls, an unboxed one by its
bytes, with only the integrity scope stated above (decision 0043 ruling 3;
decision 0046 ruling 4; decision 0049 ruling 3). The tokens
after the script are its arguments and are not judged, which is how the
shipped ship gate hands `{brokkr}` to its own script. A command with no
such script — a bare program, a `{brokkr}` verb, an absolute path, a
`\`-spelled or `/private/var`-spelled token, a `../` that escapes, or a
`./` token naming no file — SHALL be refused naming decision 0046
ruling 4 and decision 0021 and, for a spelling, the spelling. A dialect
validate or check step holds its gate today boxed — decision 0042
ruling 4's own words, and the compiler's synthetic boxed exec gate
passed through the gate law — and its argv, the dialect's own, is not
the bundle's pinned script; under `harness` and `open` it SHALL be
refused at compile naming the step, decision 0046 ruling 4, decision
0042 ruling 4 and a boxed boundary as the road open today, until a
decision admits the realm's pinned dialect declaration on the
pinned-script terms — the proposal's D6 and the design's DD8 record the
amendment for the operator, and no design note may widen a ruling
(decision 0046 ruling 4; decision 0042 rulings 1 and 4; decision 0042's
addendum, ruling 1).

#### Scenario: The shipped verifier under open is admitted
- **WHEN** `bundles/self`, whose verify seat is `["{brokkr}","driver","exec","--","bash","./scripts/verify-seat.sh","{prompt_file}"]` with hands, compiles under `open`
- **THEN** it is admitted, because `bundles/self/scripts/verify-seat.sh` is a file the bundle's own manifest walk pins

#### Scenario: A work-class exec site with hands is judged on the gate's ground
- **WHEN** an exec site with hands and `class: work` whose command is `["{brokkr}","driver","exec","--","bash","./scripts/lint.sh"]` naming a file the bundle's walk pins compiles under `open`, and a sibling whose command is `["{brokkr}","driver","exec","--","true"]` compiles beside it
- **THEN** the first is admitted, spawned in the fixed environment behind the network prefix and re-walked at spawn exactly as the shipped verify gate is, and the second is refused naming decision 0046 ruling 4 and decision 0021, the class changing nothing

#### Scenario: The exec law reads no adapter
- **WHEN** the same two-site bundle — both sites `class: work`, naming no agent, seating no gate and binding no secret, so the compiler opens no adapter library for it — compiles under `open` with no `adapters/` directory reachable
- **THEN** the first site is admitted and the second refused exactly as before, because the pinned-bytes law reads the raw command and the declaring layer's directory and nothing of the adapters, and the compile neither opens nor needs them (design DD22)

#### Scenario: A brokkr-external command under open is refused
- **WHEN** an exec gate with hands whose command is `["{brokkr}","driver","exec","--","true"]` compiles under `open`
- **THEN** compilation is refused naming decision 0046 ruling 4

#### Scenario: An escaping, absolute or platform-spelled script is refused
- **WHEN** the script token is `./../outside.sh`, `/usr/bin/true`, `.\scripts\s.sh` or `/private/var/b/scripts/s.sh` under `harness`
- **THEN** compilation is refused naming the token, and no path is compared to judge it

#### Scenario: A pinned-looking token that names no file is refused
- **WHEN** the script token is `./scripts/missing.sh` and no such file exists under the declaring layer's directory
- **THEN** compilation is refused naming the token and the directory searched

#### Scenario: Startup metacharacters refuse at compile on every host
- **WHEN** an exec site with hands, work or gate, compiles under `harness` or `open` with any refused character in a directory or filename component of its pinned script
- **THEN** compilation SHALL refuse before lookup or expansion, naming the offending component, character, decision 0048 and decision 0046 ruling 4, including for an inherited declaration and on hosts whose filesystems cannot create that name
- **AND** an ordinary script SHALL still compile with exact filename keys and unchanged later arguments, including when other files in the layer carry metacharacters

#### Scenario: Measured spellings refuse at compile with a mutable matching sibling
- **GIVEN** a real `scripts[1]/gate.sh` and an independently mutable matching sibling `scripts1/gate.sh` outside the former directory's re-walk
- **WHEN** the gate names `./scripts[1]/gate.sh`, before and after the sibling is created or edited
- **THEN** the bundle SHALL refuse at compile on every host; the real CLI's `compile` and `run` SHALL refuse before a journal or result file exists
- **AND** compile regressions SHALL cover brace and apostrophe directory spellings and the backtick route reported for `powershell.exe` too, before and after the matching sibling is created or edited, and preserve their exact manifest keys when compiling an ordinary script
- **AND** the tests' names and comments SHALL describe compile refusal and byte integrity, with no execution guarantee through an unpinned interpreter; their comments SHALL state that they start no interpreter and that Unix execution does not reproduce Windows native command-line encoding, MSYS startup parsing or `powershell.exe` command parsing (0049 ruling 3)

#### Scenario: An option before the script is refused
- **WHEN** the command is `["{brokkr}","driver","exec","--","bash","-c","./scripts/s.sh"]` under `harness`
- **THEN** compilation is refused naming `-c` as an option token before the script

#### Scenario: A file the walk skips is refused as unpinned
- **WHEN** the script token is `./dialects/run.sh` and such a file exists under the declaring layer's directory
- **THEN** compilation is refused naming the token as a path the manifest walk does not pin

#### Scenario: An inherited seat resolves against the layer that wrote it
- **WHEN** `recipes/wager-harness`, which inherits its verify seat from `recipes/fast`, compiles under `harness`
- **THEN** the verify seat is admitted, the file checked being `recipes/fast/scripts/verify-seat.sh`

#### Scenario: The shipped ship gate under harness is admitted
- **WHEN** the shipped ship seat, `["{brokkr}","driver","exec","--","bash","./scripts/ship-seat.sh","{prompt_file}","{brokkr}"]` with hands, compiles under `harness`
- **THEN** it is admitted, because the tokens after the script are its arguments and the `{brokkr}` among them names no command

#### Scenario: A pinned script edited after compile refuses at spawn
- **WHEN** an exec gate with hands compiles under `harness`, its verify script is edited after the compile, and the dispatch reaches its spawn
- **THEN** the engine refuses the dispatch naming the layer and the script's key, spawns nothing, and journals the attempt's failure

#### Scenario: A sibling the script sources is covered
- **WHEN** the script is untouched and a sibling file under the same layer is edited after the compile
- **THEN** the dispatch is refused the same way, naming the sibling's key

#### Scenario: An untouched layer spawns
- **WHEN** no pinned file of the declaring layer changed since the compile
- **THEN** the re-walk names no key and the dispatch spawns

#### Scenario: An inherited seat's ancestor is re-derived
- **WHEN** `recipes/wager-harness`'s inherited verify seat reaches its spawn under `harness` after `recipes/fast/scripts/verify-seat.sh` was edited
- **THEN** the ancestor's compose digest is re-derived and differs, and the dispatch is refused naming `fast` as the layer

#### Scenario: A dialect step under harness is refused until a decision admits it
- **WHEN** a fixture bundle with an artifact phase, its chief seated on a fixture provider that declares both `hands.harness` members as fragments, compiles under `harness` in a realm that declares a dialect
- **THEN** compilation is refused naming the synthetic validate step, decision 0046 ruling 4, decision 0042 ruling 4 and a boxed boundary as the road; and the same bundle compiles under `namespace` exactly as today

### Requirement: The argv of a site with hands follows the boundary and the class
At run time the engine SHALL compose the argv of every site with hands
from the boundary the bundle was compiled under, which is the realms map
the run was started with: under `namespace` exactly today's path — the
adapter's `hands.workspace` fragment with `{hands_mcp_json}`,
`{hands_args_toml}` and `{brokkr}` expanded, and `brokkr hands exec`
around an exec dispatch; under `seatbelt` the same adapter workspace
fragment for a model site with an explicit Seatbelt server invocation,
and `brokkr hands exec` explicitly selecting Seatbelt around the whole
exec dispatch. The Seatbelt command carries the site's policy and the
correct declaring bundle inputs at paths its implementation actually
exposes, as `seatbelt-execution` requires; it SHALL not reuse a fictional
`/runtime/bundle` mount or enter the unboxed exec branch. Under
`harness` the adapter's
`hands.harness.gate` fragment for a gate-class site and
`hands.harness.work` for a work-class site, with `{result_path}`
expanded to the seat's own result path and `{brokkr}` to this binary,
no workspace tool served and no box built; under `open` no fragment of
Brokkr's at all. Under `harness` and `open` the site's `hands.network`
and `hands.binds` stay pinned in the manifest as declared and are
enforced by nothing of Brokkr's: the harness's own sandbox decides what
the hands may reach, which is the fact the *unboxed* rendering states.
Under `harness` and `open` alike an exec dispatch — an admitted exec
site's, work or gate (proposal D32) — SHALL be the compiled command with the
script argument spelled for its interpreter as below, spawned by
the engine through `DriverProcess::spawn` with no verb of Brokkr's
around it: `{brokkr}` and `./` were expanded at compile by
`expand_command` against the declaring layer's directory, and
`{prompt_file}` stays literal for the exec driver to expand when it
stages the prompt. It SHALL start in the environment the next
requirement lists and, on Linux only, behind a network prefix when a
probe at spawn passes — `unshare --map-root-user --net -- sh -c 'ip
link set lo up && exec unshare --map-user=<uid> --map-group=<gid> --
"$@"' sh`, `<uid>` and `<gid>` being the engine's own ids: a user
namespace with the engine mapped to root, so the exec'd `sh` keeps the
capability to bring the loopback up, then a second user namespace
mapping root back to the operator, so the dispatch runs as the
operator with its capabilities dropped on exec and the network
namespace inherited. Every layer replaces itself by exec, so the PID
the engine holds is the driver's and the deadline kill reaches it as
today. The probe SHALL be that prefix around `true`, run once per engine
process at the first unboxed exec dispatch, in that dispatch's
environment against its search path, its answer remembered for every
later dispatch of the run (design DD15): with no `unshare` on the path
nothing is spawned and the answer is no; with a non-zero exit the
prefix is skipped and the dispatch runs with the network on. The
prefixed argv SHALL be one pure function of the dispatch, the probe's
answer and the ids, which the argv tests read directly; the probe's
answer is not journaled, and the record marks the run unboxed all the
same. A model site's process inherits the engine's environment under
every boundary, because its harness needs the operator's keys. An
inline model site with hands under `harness` or `open` SHALL be
refused at compile naming the repair, because its argv is the author's
and carries the box's own tokens.
`container` never reaches composition: the engine refuses it at its
entry before any journal row (boundary-availability). Seatbelt reaches
composition only after boundary-availability's policy/peer activation and
host/tool checks; no partial or unruled implementation reaches a production
seat. All model, exec and synthetic dialect sites use that same boundary
transport, including panel members, sequence steps and selected cases. A
dialect step under `harness` or `open` is refused at compile and never
reaches composition either (decision 0046 rulings 1 and 4; decision
0043 rulings 1 and 3).

#### Scenario: namespace is byte-identical to today
- **WHEN** a boxed model site and a boxed exec site are composed under `namespace`
- **THEN** their argv equal what `hands_command` produced before this change, token for token

#### Scenario: A harness gate takes the read-only fragment and its door
- **WHEN** a gate-class agent site with hands on codex, whose result path is `<workdir>/.forge/results/<effect>.json`, is composed under `harness`
- **THEN** its argv carries `--sandbox read-only --output-last-message <workdir>/.forge/results/<effect>.json`, no `mcp_servers.brokkr` entry, no `{hands_mcp_json}` expansion, and no literal `{result_path}`

#### Scenario: A harness work seat takes the work fragment
- **WHEN** a work-class agent site with hands on codex is composed under `harness`
- **THEN** its argv carries `--sandbox workspace-write` and no MCP server

#### Scenario: An open site takes nothing
- **WHEN** a work-class agent site with hands is composed under `open`
- **THEN** its argv is the adapter's base driver argv with the model and effort pins and nothing else

#### Scenario: The hands policy is declared, not enforced, under harness
- **WHEN** a site declaring `"hands": {"kind": "workspace", "network": false, "binds": []}` is composed under `harness`
- **THEN** its manifest `hands` entry still says `network` false, its argv carries no network switch of Brokkr's, and the run is rendered *unboxed*

#### Scenario: The shipped verify seat under harness on Linux with the probe passing
- **WHEN** `bundles/self`'s verify seat, compiled in this repository so that `{brokkr}` is `<brokkr>` and `./scripts/verify-seat.sh` is `<repo>/bundles/self/scripts/verify-seat.sh`, is composed under `harness` on Linux with the probe passing and the engine's ids `<uid>` and `<gid>`
- **THEN** its argv is exactly, token by token: `unshare`, `--map-root-user`, `--net`, `--`, `sh`, `-c`, `ip link set lo up && exec unshare --map-user=<uid> --map-group=<gid> -- "$@"`, `sh`, `<brokkr>`, `driver`, `exec`, `--`, `bash`, `<repo>/bundles/self/scripts/verify-seat.sh`, `{prompt_file}` — no `hands` verb, no `/runtime/bundle` path, and the literal `{prompt_file}` left for the exec driver

#### Scenario: The script argument and the spawn pin have separate spellings
- **WHEN** an exec script is composed on Windows from a canonical verbatim drive or UNC layer root
- **THEN** the script-directory pin SHALL retain the original canonical components, while only the script argv becomes an ordinary `C:/...` or `//server/share/...` path below 260 UTF-16 units; a path requiring the verbatim prefix because of length, namespace or filename components SHALL refuse before spawn (proposed decision 0048, Windows script argument repair)
- **AND** the engine token and subsequent unjudged arguments SHALL stay unchanged; Unix argv SHALL retain its exact bytes, including literal backslashes; plain exec sites without hands SHALL use the same conversion without adding a re-walk
- **AND** a pure composition test SHALL exercise both platform policies on Linux as well as on native hosts, asserting the complete child argv separately from the canonical directory pin, including an inherited script followed by an argument naming another layer
- **AND** its comment SHALL state that it starts no interpreter and cannot prove native Windows startup or command parsing on Unix; argv and pin assertions SHALL claim composition only, with no execution guarantee under `harness` or `open` (decision 0049 ruling 3)

#### Scenario: The same seat with the probe failing, and off Linux
- **WHEN** the same seat is composed under `harness` on Linux with the probe failing, and again on macOS and on Windows
- **THEN** its argv is exactly `<brokkr>`, `driver`, `exec`, `--`, `bash`, `<repo>/bundles/self/scripts/verify-seat.sh`, `{prompt_file}`, with the Windows script path spelled for its interpreter as above, spawned in the fixed environment with the network on — and the same holds under `open`

#### Scenario: The probe is the prefix around true
- **WHEN** the probe runs
- **THEN** the command it spawns is the eight-token prefix followed by `true`, in the dispatch's environment, nothing it learns is journaled, and a second unboxed exec dispatch of the same engine process spawns no second probe

#### Scenario: The probe's arms on a planted search path
- **WHEN** the probe is given a search path with no `unshare`, then one whose `unshare` is a planted executable exiting non-zero, then one exiting zero
- **THEN** it answers no without spawning anything, no, and yes, in that order; and on macOS and Windows it is never consulted

#### Scenario: A model site keeps the engine's environment
- **WHEN** a gate-class agent site with hands on codex is composed under `harness`
- **THEN** it is spawned with the engine's own environment, exactly as under `namespace`

#### Scenario: An inline model site with hands under harness is refused
- **WHEN** an inline seat whose command is a `{brokkr} driver claude` dispatch declares `hands` and compiles under `harness`
- **THEN** compilation is refused naming the seat and the repair

#### Scenario: A Seatbelt model site serves the workspace tool
- **WHEN** a work or gate model site with hands on an adapter supporting workspace hands is composed under `seatbelt`
- **THEN** its adapter workspace fragment carries an MCP server invocation selecting Seatbelt, the site's policy, worktree and engine executable, with both JSON and TOML forms preserving path quoting; its provider harness keeps its control-plane environment and its commands use the cleared hands environment

#### Scenario: A Seatbelt exec site never takes the unboxed branch
- **WHEN** a boxed work or gate exec site is composed under `seatbelt`
- **THEN** the whole driver dispatch is inside `hands exec` selecting Seatbelt, without the unboxed inherited-PATH environment or best-effort network prefix, and its script/helper paths and result-file delivery follow `seatbelt-execution`

#### Scenario: All invocation shapes and dialect gates receive Seatbelt
- **WHEN** a single site, panel member, sequence step, step-panel member, selected case and synthetic dialect validate/check step with hands are composed under `seatbelt`
- **THEN** each actual invocation receives Seatbelt and its own hands policy, with its existing member/step identity preserved; a dialect gate uses the real boxed exec path without inventing admission under harness/open

#### Scenario: Namespace and unboxed behavior remain covered
- **WHEN** the existing namespace, harness and open composition and gate-policy suites run after adding Seatbelt
- **THEN** their admissions, refusals, identity and prompt delivery remain unchanged, including decision 0048's script-directory checks and decision 0049's unboxed interpreter limitation

## Decisions

- **R4 — conditional full peer, no weaker fallback.** Compilation remains
  machine-independent. The accepted 0046 addendum settles the permitted
  hooks observable; native independent protection evidence is still required.
  The runtime fence prevents an incomplete or unproven Seatbelt mechanism
  from holding a gate. A reduced grade would require a new operator ruling
  and coherent gate/record/readout changes; none is authorized here.
