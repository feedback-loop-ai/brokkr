# Decision 0065, slice one — delivery evidence

Recorded 2026-09-21 by the implement seat of run
`build-decision-0065-slice-one-of-773a4e83`. This file describes what was
observed. It instructs no gate, waives none, and claims no live provider
result: every argv assertion named here is composition evidence.

The two implementation visits below are historical observations. The repair
design and tasks addenda below supersede their completion/permission conclusions;
it does not rewrite their measurements or turn masked assertions into passes.

## What this visit found and did

The branch already carried five `wip:` commits (≈8.7k lines) from an earlier
visit of this seat, with no task ticked. The tree compiled and most suites
were green, so this visit audited it against the ledger, task by task, before
adding anything. The audit found real defects behind the green:

| Found | Repair |
| --- | --- |
| A DSH model launch never read the engine's plan, so `native_controls: null` (no computed authority) launched instead of refusing. | `dsh_launch_with` refuses first, before any provider work; LaneTally already went through `claude_launch`. |
| A catch-all gave every site lacking an outcome an empty `exec` one. It was masking the dialect steps (`analyze:check`, `clarify:count`, `design:validate`) the walk never visited, and would equally have masked an authored Codex site, which would then launch with no OFF pair. | Dialect steps and the wrapper's generated validator take the same `record_capabilities` walk as authored sites; the catch-all is gone; a compiled site with no outcome is a loud `expect`. |
| A wanted-drop notice claimed "native capability remains OFF" whenever a binding named another provider — true for Claude beside a Codex grant, false for DSH, LaneTally or exec, whose inventory is unmeasured. | The tail is decided from what the candidate's own native plan switched off; the same tail now rides the seat's `not_held` reason, so the prompt says it too. |
| `brokkr doctor` under a malformed map built a `<unknown>` realm granting nothing and printed "NOT granted here … switched off" for installed Codex and Claude. One failing grant erased the realm's other grants; one malformed adapter erased every native line silently. | Unknown authority prints adapter assessments and claims neither grant nor denial; each grant is validated alone; unreadable adapters and definitions are failing lines of their own. |
| Semantic library lint did not exist: the CQ2 lint string was produced only by a unit test calling the helper with a hand-typed name. | `Definitions::lint`, wired into `brokkr agents list` (warning) and `show` (refusal) under the operator's root, never `--agents-dir`. |
| No refusal for a legacy `tools.allow` alias that maps to a native tool (`websearch` → `WebSearch`). | Refused at resolution with a migration reason naming agent, provider, entry, tool and capability. |
| The resume mismatch blamed "engine or contract version" for a moved grant. | `manifest_diff` names capabilities and the record that moved, and says a pre-0065 run is never rewritten. |
| The start fence compared grants only. | It compares the realm the bundle was resolved in as well. |
| An empty operator root (`Bundle::compile` beside a `capabilities/` directory) refused every definition as an escape, because `""` does not canonicalize. | The empty root is the caller's directory. |
| The final-invocation conflict refusal did not name the seat; Claude list folding ignored `--allowed-tools` and `=` spellings and then refused its own duplicate. | Seat named from the input's `seat`; folding reuses the launch's one alias reading and folds in place. |
| `crates/brokkr-cli/tests/recipes.rs` asserted a composed layer's digest equals its base's standalone digest, which the required `capabilities` section ends. | The test now states the relation exactly: the layer digest is the base manifest without that section. |

## Removal experiments (tasks 9.1–9.3)

Each mutation was applied alone, the named tests run, the failure read, and
the mutation reverted; `git status` and `git diff --stat` were empty after the
last one, and the runtime and protocol suites then ran green. A compile error
was never counted: two mutations that first produced one were rewritten until
they compiled. Line numbers are those of the tree at `dda1a3c7`.

| # | Mutation (production unless stated) | Intended assertion | Observed |
| --- | --- | --- | --- |
| M1 | `resolve`: a `requires` failure no longer returns the refusal | `a_missing_grant_refuses_a_requirement_and_records_a_dropped_want`, full-reason equality | FAILED at `capabilities/tests.rs:851` (the `unwrap_err()` of that equality); five sibling refusal tests failed with it |
| M2 | `holding`: office-scope check disabled | `office_scope_and_an_empty_tool_list_only_narrow` | FAILED at `:951`; and CQ1's "scope precedes transport" equality failed showing the WRONG reason (restriction transport) — the case full-text assertion exists for |
| M3 | `holding`: provider/binding mismatch check disabled | `provider_compatibility_cannot_expand_a_holding` | FAILED at `:1046`; `a_site_records_each_candidate_apart…` failed too (a fallback gained the holding) |
| M4 | `holding`: CQ1 restriction-transport check disabled | `cq1_an_inexpressible_restriction_…` | FAILED at `:1241` (requirement refusal). Its want-notice and OFF-argv assertions sit later in the same function and were masked by this one; M6 and M8 reach them independently |
| M5 | `native_plan`: unsupported OFF no longer refuses | `a_native_power_that_cannot_be_switched_off_refuses_the_seat_whatever_it_asks` | FAILED at `:1367`; `a_generated_validator_is_refused_…` FAILED at `bundle/tests.rs:192` |
| M6 | `resolve`: wanted-drop notice not recorded | exact notice equality | `a_missing_grant…` FAILED at `:859`; CQ1 FAILED at `:1249` — the exact CQ1 notice, independently of M4 |
| M7 | `Authority::load`: `mcp` kind no longer refused | `an_mcp_grant_refuses_until_slice_two_even_unused_and_a_hands_grant_is_reserved` | FAILED (only that test), at the load refusal it demands, including the unused and `offices: []` cases it carries |
| input | TEST input of the positive control: grant removed | `a_codex_seat_that_holds_search_is_launched_without_the_off_pair` | compile refused with the full missing-grant reason for seat `boxed` |
| input | TEST input of the positive control: grant scoped to `nobody` | same | compile refused with the full scope reason |
| M8 | `codex_cold`: managed fragment not appended | final-argv denial matrix | `a_codex_seat_that_does_not_hold_search_…` FAILED at `capability_launch.rs:393` (the exact pair); the CQ1 restricted-want OFF composition FAILED independently at `:794`; the rejoin proof stayed green |
| M9 | `codex_launch` resume path: managed fragment not appended | actual-resume denial | only `an_eligible_rejoin_of_a_compiled_codex_seat_…` FAILED, on an `exec resume` argv with the session and `-` present and the pair absent; the cold matrix stayed green |
| M10 | `native_plan`: ON replaced by unconditional OFF | cold and resume ADMISSION | `a_codex_seat_that_holds_search_…` FAILED (1 OFF pair, 0 expected); the held rejoin argv FAILED carrying the pair; a panel/sequence holding test failed too |
| M11 | `Authority::manifest`: grants emptied | "an unused grant is still authority" | FAILED at `capability_launch.rs:697` |
| M12 | definition digest held constant | "capabilities/web-search.json bytes moved nothing" | FAILED at `:707`, after the dialect-bytes turn of the same loop had passed |
| M13 | dialect digest held constant | "dialects/tools/codex-native-search.json bytes moved nothing" | FAILED at `:707` on the dialect turn |
| M14 | restriction keys dropped from the pinned grant | `a_restriction_value_moves_the_manifest_digest_even_where_it_is_inactive` | FAILED (only that test) at `:782` |

No mutation is committed and no mutation framework was added.

## Gates as run by this seat

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | clean |
| crate suites, crate-scoped (`core`, `store`, `protocol`, `runtime`, `view`, `bridge`, `cli`) | green |
| workspace, all features, locked (run instrumented, `--no-fail-fast`) | green |
| `cargo test --workspace --no-fail-fast` | green |
| `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self` and `bundles/verify` | both compile; `recipes/research` in this repository's realm shows `grants: {}` and both researcher drop notices |
| witness and compose pins | unchanged by this visit and passing against actual compiles |
| `openspec validate --all --strict` | **NOT RUN — the command is not permitted to this seat** (both spellings were declined). Owed. |
| `bash scripts/coverage-exact.sh` | **NOT RUN — not permitted to this seat.** Pending, as task 11.4 words it. |

Seat-side coverage DIAGNOSTIC, not the gate: `cargo +nightly-2026-09-05
llvm-cov --workspace --all-features --locked --branch --lcov`, read for unhit
`DA` and `BRDA` records the way the script's awk does. The inherited tree had
five unhit records (`doctor.rs:782`; `bundle.rs:1699`, `3173`, and branches at
`3132`, `3167`); each was removed by restructuring or reached by a test, never
excluded. The last run, over the final tree, reported 34,304 line records
with no unhit `DA` and no unhit `BRDA` record. It is not candidate-bound
evidence: it shares no unique target
directory and did not run the script's function check.

## Second visit — the returned finding (tree from `b2cb8430`)

The first visit returned `broken` with fifteen tasks open. This visit owned
that finding. Twelve of the fifteen were within a seat's control and are now
done and ticked; three are not, for the same reason as before, and stay open
(see the last section). Nothing below claims a live provider result.

What was built, as distinct from only tested:

| Task | Built |
| --- | --- |
| 2.2 | Without a map the operator's directory is the OPERATED repository (`--repo`), not the workspace, on `run`, `rerun` and an unmapped `resume`; `Bundle::compile_unmapped` names the root rather than inferring it from the library's parent. |
| 6.3 | `start_in_world` refuses, before `create_run`, a bundle whose pinned definition or dialect bytes are missing or changed (`EngineError::CapabilityInputMoved`), read beside the map or under the operated repository. `start` delegates to it; `start_with_dispatch` already refuses the `capabilities` key before any row. |
| 6.4 | A resume whose pinned authority cannot be reproduced — it fails INSIDE the compile, so no manifest exists to compare — leaves through the manifest-mismatch refusal with capabilities named. `CompileError::Capability` reads exactly as `Invalid` does and survives the composed-bundle chain note. |
| 2.4 | `embedded_schema_fault` judged reserved keys at every depth, so a dialect's own nested `allow.tools` was falsely refused, and it missed `dependencies`, `patternProperties`, local references and another draft's `$schema`. It now follows only what describes the grant's top level, refuses a dangling local reference, and ignores a `$ref` spelled inside data. An `mcp` launch may reference only the secrets its dialect declares — judged as text through the house `{{secret:NAME}}` scanner, no store opened, no argument echoed. A pattern is judged by the same validator that later reads it, so no regex dependency was added. |
| 4.5 | The impossible-OFF refusal names its evidence source and scope beside the reason. |

Found while proving, each a gap a green suite was hiding:

- The adapter loader's repeated-key refusal (`agents/load.rs`) had NO test:
  removing it failed none of 473. One same-line closure kept line coverage
  at 100% over it.
- Passing `None` at the sequence step's `mark_capabilities` handed a DSH
  fallback its Codex primary's plan, and failed none of 475.
- My own first verb test could not tell a resume that read the wrong root
  from one that read the right one, because a decoy definition lay in both;
  removing the decoy before the resume made mutation R2 fail where it should.

### Removal experiments of this visit

Same discipline as above: one mutation at a time, the named test run and the
failure READ, the mutation reverted; `git diff` on the mutated file was empty
afterwards and the suites re-ran green.

| # | Mutation (production) | Intended assertion | Observed |
| --- | --- | --- | --- |
| S1 | start fence looks at an empty section | `a_run_starts_only_over_the_definition_and_dialect_bytes_…` | FAILED at its first full-reason equality (`Ok(())` where "is missing" was demanded); the other three engine capability tests stayed green |
| R1 | unmapped root reverts to the workspace | `an_unmapped_run_reads_and_keeps_the_operated_repository_…` | FAILED at the run that must refuse; the other verb tests green |
| R2 | an unmapped RESUME reads the workspace | same test | FAILED at the resume, with the mismatch refusal — reachable only after the decoy was removed |
| R3 | resume reads today's map, not the run's pin | the map-only-edit test and the unmapped test | both FAILED — and even mutated the engine REFUSED (`pinned grants, dialects, sites no longer match`) rather than borrowing the grant: the manifest comparison is a second line behind the pinned world |
| R4 | the mismatch-door mapping dropped | `a_resume_that_cannot_reproduce_its_pinned_inputs_…` | FAILED at the dialect-gone case, showing the bare compile error the door replaces |
| C1 | reserved-key walk stops following composition and references | `a_restriction_schema_stays_inside_its_file_…` | FAILED (only that test) at the `allOf` case |
| C2 | declared-secret comparison admits every name | `an_mcp_launch_names_only_the_secrets_…` | FAILED (only that test) at `OTHER_KEY` |
| L1 | adapter loader's repeated-key refusal disabled | `a_native_declaration_with_a_repeated_key_…` | FAILED (only that test; 473 others green) |
| A1 | a written subset is ignored (the seat keeps its office's asks) | `an_office_is_inherited_subset_and_emptied_…` | FAILED at `judges:subset`: held `web-search`, **0 OFF pairs where 1 was demanded**; the cold denial matrix failed with it |
| E1 | evidence source and scope swapped | `a_native_power_that_cannot_be_switched_off_…` | FAILED (only that test), on distinct source and scope values |
| D1 | sequence step ignores the selected link | `every_nested_dispatch_hands_its_driver_…` | FAILED at the fallback: the Codex OFF plan where DSH's unmeasured plan was demanded |
| — | class lists compared as written, not as sets | (a check that an EXISTING claim was proved) | ten tests FAILED: class-order equality was already well exercised |

### Gates as run by this visit

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | clean |
| crate suites, crate-scoped, all seven | green — with one event stated plainly: the FIRST `brokkr-protocol` run of this visit reported 68 failures in `adapters::tests`; the crate is untouched by this visit, and two immediate reruns with no change between were 410/410 and then 410 + 99 + 1 green. The shape matches issue #255 (one ETXTBSY panic poisoning a shared mutex), but the first run's root panic was not captured, so its cause is not asserted here. |
| `cargo test --workspace --all-features --locked --no-fail-fast` | green: 77 test binaries `ok`, none otherwise (counted, because silence is also what a timeout prints) |
| `compile --bundle bundles/self` and `bundles/verify` | both compile, each carrying the `capabilities` section |
| witness and compose pins | unmoved: nothing this visit changed enters a manifest |
| `openspec validate --all --strict --no-interactive` | **NOT RUN — declined to this seat again**, piped and bare. Not retried under another spelling: the refusal is a permission boundary, not an obstacle. |
| `bash scripts/coverage-exact.sh` | **NOT RUN — declined to this seat again.** Pending, as task 11.4 words it. |

Seat-side coverage DIAGNOSTIC over the final tree, not the gate: `cargo
+nightly-2026-09-05 llvm-cov --workspace --all-features --locked --branch
--lcov`, read with whole-record matches: 34,437 `DA` records, none unhit;
5,670 `BRDA` records, none with a zero or `-` taken count. It did not run
the script's function check (source functions keyed by file and start line),
shares no candidate-bound target directory, and is not offered as 11.4's
evidence.

## Owed to the controller (unmeasured; nothing here discharges them)

- Codex 0.154.0: whether OFF holds, and whether a held seat has search ON, on
  a RESUMED session. The pair is composed on the actual `exec resume` argv and
  proved deterministically; the live check is owed.
- Codex: any `web_search` value other than `"disabled"`, the `--search` flag
  under `exec`, other CLI versions, profile / `config.toml` precedence over
  `-c`, an exhaustive native inventory, and whether the hands fragment's
  `mcp_servers.brokkr` excludes an ambient MCP server.
- Claude: `WebSearch` / `WebFetch` OFF and ON, boxed and unboxed, are adapter
  data and argv composition. The empty native tool list under
  `--strict-mcp-config` is a declaration, not a live measurement.
- DSH: declared `unmeasured`. Unsupported `mcp` / `tool_permissions` proves
  neither an inventory nor an OFF; its headless profile is recorded as
  shipping web fetch ON. Nothing is granted through it and no denial claimed.
- LaneTally: declared `unmeasured`; forwarding is not confinement and Claude's
  evidence is not inherited.
- `exec`: `unmeasured`; the engine cannot certify an arbitrary child program.
- A by-hand `brokkr driver <model>` run carries no `native_controls` key and
  composes nothing: only an engine-launched site is ruled. Deliberate and
  documented at the top of `native_controls.rs`; stated here so nobody reads
  a by-hand Codex run as denied.

## What the first visit left open (kept as written; each answered by the second)

Every bullet but the last is closed by the second visit's section above. One
is closed by a recorded refusal rather than by building it: 6.1's conditional
file-walk exclusion (design D7, final paragraphs). The `(provider, model)`
collision was answered by proof, not by re-keying: two links sharing a pair
carry equal outcomes, because authored argv can refuse a compile and cannot
alter an outcome.

- 1.4 / 6.4: the v11 `capabilities.realm` is a bare name with no relative
  operator source context, so resume re-reads definitions and dialects beside
  TODAY's pinned-map directory. Grants do come from the pinned world, not the
  current map, and a moved input refuses naming capabilities — but there is
  no CLI-level resume test for: a map-only edit cannot add a grant; an
  unmapped run keeps its operated root; a missing definition (today a compile
  error from `Authority::load`, not the manifest-mismatch door).
- 6.3: the start fence compares realm and grants, not definition / dialect /
  adapter input bytes between compile and start.
- 2.2: without a map the operator root is the workspace, not `--repo`; the
  convenience compile entry points take the root from the library root's
  parent; no CLI `run` / `rerun` test exercises a v6 grant.
- 2.4: no semantic check of undeclared MCP secret references; an unsupported
  `$schema` draft is not refused; `embedded_schema_fault` walks nested
  `properties` (a nested key named `tools` is falsely refused) and misses
  `patternProperties`, `propertyNames`, `dependencies`. Not a fail-open:
  grants are validated with the reserved keys already stripped.
- 3.2 / 5.1: subtraction is proved at unit level and for top-level seats;
  agent-backed omission / subset / `{}` inside panel, sequence, select bodies
  and inherited bundles has no compile-level test. Outcomes are matched to
  candidates by `(provider, model)`, so two links with the same pair collide.
- 3.3: no refusal test for a duplicate key inside `native_capabilities` or an
  invalid selection mapping; the declaration-parse assertions are fragments.
- 4.5: unsupported OFF is not exercised boxed vs unboxed at compile level nor
  for a subtracted ask; the diagnostic carries the reason, not the evidence
  source and scope.
- 5.2: no test drives an engine dispatch (primary, fallback, nested) to the
  driver input; `mark_capabilities` is tested directly.
- 6.1: the `capabilities/` file-walk exclusion applies to any bundle root by
  name; no test for "no absolute roots or expanded argv in the section".
- 11.3, 11.4, 12.1: blocked on `openspec` and the coverage script as above;
  the change is therefore NOT archived.

## Not finished after the second visit

Three tasks, all for one reason: the commands they consist of are declined to
this seat, on both visits. None is within a re-run's control unless the seat's
grants change; all three are one host session's work.

- **11.3** — its cargo half is done (both bundles compile; `git diff --check`
  clean). `openspec validate --all --strict --no-interactive` is NOT run.
  This visit edited `design.md` and `tasks.md` of the change, so that
  validation is owed over those edits in particular.
- **11.4** — `bash scripts/coverage-exact.sh`, candidate-bound, outside the
  box. NOT run. The diagnostic above is not it.
- **12.1** — `openspec archive decision-0065-capabilities-slice-one --yes`,
  the six spec folds and their `## Provenance` lines. NOT run; it is ordered
  after 11.3 and 11.4 and the change is NOT archived. The branch's commits
  are `wip:` checkpoints, left unsquashed for the same reason.

Two things a reviewer should weigh, stated rather than buried:

- 6.1 is ticked against a recorded deviation, not against its letter.
- The flake in the first `brokkr-protocol` run is unexplained by this visit.

## Security-hold repair design — 2026-09-22

Run `build-decision-0065-slice-one-re-25d222e6`, design seat. Adopted every
branch commit through specification repair `5c53a30f`, following the council's
review of `f0264a9b`. Read both current design positions, all four original
review positions and the chief's complete eight-finding reconciliation.
This addendum reports design work only; no repair regression, implementation,
mutation, provider observation or new manifest measurement occurred here.

The design now rejects D7's former exception for excluded active charter and
policy bytes. The owning specification had already chosen compile refusal;
D7 and reopened task 6.1 now agree, including standalone/inherited attribution,
normalized/canonical paths and permitted-input identity controls. H4 remains
HIGH with `spec_defect=true`. The old paragraph saying 6.1 was closed by a
recorded refusal is historical and is no longer a completion claim.

D3–D8/D10 now bind H1–H3 to fallible known-provider admission, explicit
argument provenance, complete final control composition and independently
validated active input identity. They bind M1–M3 to disposition-aware doctor
wording, strict source-byte request parsing and whole-loaded-library lint,
with consulted-definition pins. Both council positions were reconciled by
claim and evidence; no new subsystem or later-slice behavior was adopted.
The macOS fixture design stores one canonical temporary root and keeps full
diagnostic equality. Its code change and actual host results remain pending.

Task 9.1 is reopened. Historical removal M3 failed on a required refusal or a
fallback holding; historical M4 failed on a required refusal before reaching
its optional assertions. M6 removed notice recording and M8 removed cold OFF
composition, so they do not establish optional notice detection when either
compatibility check itself is removed. No independent optional compatibility
failure/restored-pass result has been observed in this design visit. The
revised design/tasks identify the two separate optional proofs still owed.
The earlier adapter-duplicate, selected sequence fallback and no-decoy unmapped
resume removal observations remain historical protections to retain.

Reopened tasks are pending repair and proof, not a restatement of the original
40-of-43 ledger. The separate proposed repair decision remains before code
implementation in task 0.1; accepted 0065 is unchanged. Original contract
additions are adopted and their bytes are now frozen for this repair. No
witness is re-pinned by inference from a design change.

The operator has authorized OpenSpec validation and the exact-coverage script;
earlier records of denied permission do not apply to this run. The operator
also explicitly withholds archive/fold pending council re-judgment: task 12.1
remains open for that reason, independent of local check results. The security
hold remains. Notes here describe status and provide no gate instruction or
exception.

Design validation observations: strict all-item noninteractive OpenSpec passed
16 items with zero failures; `git diff --check` was clean. The unchanged
informational long-requirement and issue-226 archive notices were not failures.
Formatting, strict all-target/all-feature locked clippy, all seven crate-scoped
suites, both workspace suites and self/verify compiles each stopped at missing
`cargo` with exit 127 through workspace hands. The now-authorized exact-coverage
script stopped at line 33 for the same reason, exit 127. Coverage counts were
not produced: **source lines unavailable, branches unavailable, functions
unavailable**. No Rust, bundle, coverage, macOS or remote-CI pass is claimed.
These observations describe this design return, not completion of the pending
code repairs. Final committed-head validation is also reported in this seat's
result record; no historical coverage diagnostic substitutes for it.


## Security-hold repair tasks — 2026-09-22

Run `build-decision-0065-capabilities-slice-one-re-25d222e6`, tasks seat.
Adopted the branch through `6d47c120`; read all nine 0065 rulings, the chief's
complete ruling, proposal, six deltas and revised design, and checked the
named implementation/test seams. D7 and the owning manifest scenarios already
choose refusal of excluded active inputs, so no upstream correction was needed
to write an honest breakdown. H4 remains HIGH with `spec_defect=true`.

The revised ledger contains **52 tasks: 15 adopted checked tasks and 37 open
tasks**. Its dependency order puts red reproductions and the proposed repair
decision before code, strict source readers and argument-origin capture before
shared composition/admission, and dispatch before final launch proofs. M3's
whole-loaded-library lint is now separately executable as 3.7; H2's argument
origin prerequisite is 3.8. Task 6.1 retains the corrected H4 refusal and four
standalone/inherited charter/policy cases. Task 4.4 separates the optional
compatibility tests; 9.1 requires the two intended notice failures under actual
compatibility removal. No historical required failure was promoted to that
missing evidence. The finding index and launch matrix bind every repair to
its owning proofs; 9.4 carries closure observations, and 11.5 carries committed
handoff. All three earlier removal-found regressions remain obligations.

A read-only ledger audit verified unique numerical ordering, unchanged adopted
checkmarks, requirement citations on every task and valid links/titles for all
**39 requirements**. Tasks 6.1, 9.1 and 12.1 remain unchecked. Strict all-item
OpenSpec validation passed **16 items, zero failures**; `git diff --check`
passed. OpenSpec reports all planning artifacts present, which is no claim
that implementation is complete. Existing informational long-requirement and
unrelated issue-226 archive notices remain unchanged.

Formatting, strict all-target/all-feature locked clippy, all seven crate-scoped
suites, both workspace suites and self/verify compiles were attempted through
workspace hands; every command stopped with missing `cargo`, exit 127.
The authorized exact-coverage script stopped at line 33 for the same reason.
The three coverage results are **source lines unavailable; branches unavailable;
logical functions unavailable**. No report, counts or percentage was produced.
Final committed-head attempts are recorded in this seat's result record.

This visit changes the breakdown and this evidence addendum only. It adds no
production code, repair regression, mutation observation, host/provider result,
grant, contract change or digest pin. Linux/macOS repair results, Rust gates
and final implementation coverage remain pending; they are not passes. The
security hold remains for council re-judgment. No archive/fold was attempted;
12.1 stays open under the operator's explicit ruling. These notes describe
observations and supply no gate instruction or exception.

## Security-hold repair implementation — 2026-09-22

Run `build-decision-0065-slice-one-re-25d222e6`, implement seat, on Linux.
The repair was built in three visits of this seat over the tasks ledger at
`e5408669`. The first left four `wip:` commits (`cc0a9c31`, `b34e9c9a`,
`575915c8`, `59c3aa9c`: the proposed repair decision, every production
repair, the launch-level proofs) and its scratch observations under
`.forge/repair/observations.md`, with no task ticked and no result
recorded. The second adopted those commits, audited them against the ledger,
added what was missing (the resume-door active-input proof of 6.4, the
guides of 10.1, the measured pins of 10.2, one fixture the whole-library
lint newly refuses), ran every authorized gate but one on the final tree and
folded both visits' observations here; its exact-coverage run was cut off
with the seat, leaving an orphaned instrumented tree under `/tmp` and no
counts. The third visit re-ran every gate, ran the coverage gate to its
verdict, repaired the four gaps it named (below, under "Gates"), re-ran
everything on the repaired tree and committed. Every red observation below
was taken on the delivered behaviour at `e5408669` before the repair that
answers it; every removal was applied alone, its named test run and the
failure READ, the mutation reverted and the suite re-run green. No mutation
is committed and no mutation framework exists: `git diff` over `crates/*/src`
was empty after the last one, and the tree carries no `if false &&`,
`take(0)` or `.filter(|_| false)` outside a test.

Nothing here claims a live provider result. Every final-argv assertion is
composition evidence; the limits recorded under "Owed to the controller"
stand unchanged.

### The proposed decision (0.1)

`docs/decisions/0066-a-denial-is-something-the-launch-proves.md`, status
`proposed`, indexed in `docs/decisions/README.md`. Nine rulings: the
known-power floor and refusal (1), the same law at the driver (2), one
composer so an accepted control reaches the argv or its form refuses (3),
authored driver argv carries no capability with provenance carried not
recovered (4), the H4 correction of design D7 (5), the doctor's OFF-first
judgment (6), strict request readers (7), whole-library lint at compile (8),
and independently runnable optional-notice proofs (9). Accepted 0065 is
unchanged.

### Red reproductions on the delivered head (0.2)

| Finding | Test (owning suite) | Observed at `e5408669` |
| --- | --- | --- |
| H1 | `a_known_native_power_with_no_valid_denial_refuses_the_seat` (`runtime/tests/capability_launch.rs`) | The inline no-ask, no-grant Codex work seat launched `["codex","exec","--json","-C","/w","-c","model_reasoning_effort=\"high\"","--model","gpt-6-astra","--sandbox","workspace-write"]` under legacy adapter data — no `web_search` OFF. An unrelated malformed adapter file produced the same launch. The pre-existing protocol assertion `assert_eq!(codex_managed(&missing), Vec::new())` and the runtime case "An OFF nobody measured is not a refusal and not a denial" ASSERTED the fail-open; both corrected. |
| H2 | `an_authored_capability_server_refuses_the_compile_and_the_engines_hands_still_launch` (`capability_launch.rs`) | The panel's `-c mcp_servers.ungranted.command="npx"`, `-c mcp_servers.ungranted.args=["fetch-mcp"]` reached the final Codex argv under `grants: {}` beside the composed OFF pair; Claude `--mcp-config` plus `--allowedTools mcp__ungranted__fetch` reached the final Claude argv. |
| H3 | `a_native_control_declared_as_argv_reaches_the_final_claude_command` (`capability_launch.rs`) | Search OFF declared as argv `["--disallowedTools","WebSearch"]` was recorded in the plan and the final boxed Claude deny list read `WebFetch`, not `WebFetch,WebSearch`. |
| H4 | `a_charter_under_a_tree_the_walk_skips_is_refused_where_it_is_declared`, `a_policy_under_a_tree_the_walk_skips_is_refused_where_it_is_declared`, `no_spelling_and_no_link_hides_an_active_input_under_a_skipped_tree` (`bundle/compose_tests.rs`) | Each compiled, standalone and inherited: `compiled to <digest>` where the full refusal was demanded, and a changed charter or policy under `capabilities/` left the digest unchanged. |
| M1 | `a_matching_grant_never_promises_a_denial_the_launch_does_not_deliver` (`cli doctor/capability_tests.rs`) | Under a grant scoped to `offices: ["researcher"]` on the unsupported-OFF fixture, doctor printed "… every other seat on stuck is launched with it switched off"; the pre-existing `installed_harnesses_are_told_apart…` asserted that promise. |
| M2 | `a_request_key_written_twice_in_an_agent_source_is_refused_from_its_bytes` (`agents/tests.rs`), `a_request_key_written_twice_in_a_recipe_layer_is_refused_from_its_bytes` (`compose_tests.rs`) | The agent library LOADED `{"web-search": Wants}` from `{"web-search": "requires", "web-search": "wants"}`; the recipe layer composed the same silently (four site forms × five repetitions × own/ancestor layer). |
| M3 | `an_unseated_loaded_agent_with_an_undefined_request_refuses_the_compile`, `a_definition_only_an_unseated_agent_names_is_pinned_and_an_unconsulted_one_is_not` (`bundle/agent_tests.rs`) | A valid seated worker hid the unseated researcher's undefined `library-docs`: "expected compilation to fail"; definitions pinned `[]` where `["web-search"]` was expected. |
| M4 | the six split tests in `capabilities/tests.rs` (4.4) | Not a behaviour defect: the missing deliverable was the independent removal proof, recorded under 9.1 below. |
| macOS | `a_native_declaration_with_a_repeated_key_or_an_uncomposable_selection_is_refused` (`agents/tests.rs`) | Reproduced on Linux by reaching the fixture through a symlink `alias -> real` with the root used lexically: left named `/tmp/.tmpXXXX/real/adapters/claude.json`, right `/tmp/.tmpXXXX/alias/…` — the `/private/var` shape exactly. |

### What was built, per finding

- **H1 (3.3, 4.5, 5.2, 7.1–7.2).** `bundle.rs::load_pin_adapters` keeps
  its `Result`: the effort exemption takes the `Ok`, the capability pass
  takes the error as the refusal's cause. `capabilities.rs::native_plan`
  refuses an absent root, an absent provider, legacy/omitted/empty
  `native_capabilities`, an omitted floor power, an unreadable or malformed
  adapter file (unrelated ones included) and any unmeasured OFF of an unheld
  declared power, naming site, office, realm, provider, capability and the
  original cause. The plan carries `provider`, `harness` (the driver KIND —
  a provider may run the codex harness under any name), `on` and `off`.
  `native_controls::managed` decodes fallibly; `known_powers(harness)` is
  the floor (codex: web-search; claude: web-search, web-fetch). The cold
  replacement of a refused Codex rejoin reuses the launch's validated cold
  argv (`codex_launch_and_cold`); `codex_managed`'s `.ok()` is gone.
- **H2 (3.8, 4.6, 5.1, 7.3).** `SiteSpawn.managed` is recorded where the
  hands fragment is appended; the engine writes the driver's private
  `launch_arguments` (`authored`, `managed`) LAST, after every input merge,
  and the driver refuses a launch whose two parts do not reassemble its
  argv. `native_controls::authored_server_conflict` refuses Codex
  `mcp_servers` tables and descendants under `-c`/`--config` in split and
  joined spellings however quoted, Claude/LaneTally `--mcp-config`,
  `--settings` and any tool list admitting `mcp__*` or a wildcard, with no
  exception for a server named `brokkr`. DSH: every residual argument was
  already refused and its one `--patch` is the bound, contained,
  digest-matched route-only overlay, re-read and validated against the
  closed grammar before staging (`adapters/route_overlay.rs::claim`); no
  new door was found there. *Recorded deviation:* `Candidate` carries no
  separate base vector; `Candidate::parts()` splits the argv at the LENGTH
  `agents::compose` recorded in `hands_fragment` — positional provenance
  from the one place the fragment is appended, never a text search.
  *Migration:* an inline model seat under a box that authored the hands
  tokens itself is now refused as a counterfeit; no shipped recipe does
  this.
- **H3 (4.3, 4.4, 7.4).** `native_controls::compose_for_provider` is the
  one composer, called by the compiler (`capabilities::admit`) on the
  unexpanded parts and by the codex, claude, lanetally and dsh launches
  (`adapters::composed_launch`) on the expanded ones. Claude/LaneTally fold
  managed list argv into the same include/allow/deny lists a selection
  feeds, emit each list flag once, refuse a tool both admitted and denied,
  and append a non-list argv (a restriction transport) verbatim; Codex
  refuses a selection; DSH and exec refuse any nonempty representation.
- **H4 (6.1–6.4).** `bundle.rs::unpinned_active_input` shares
  `unpinned_top_level` with the walk and judges the reference as written
  (`.`/`..` folded) and its canonical target; a reference written out of
  its layer is refused on the same terms. `parse_role` calls it with the
  declaring layer, `compose::own_table` for EVERY layer before its table is
  read, so an inherited refusal names the ancestor's `bundle.json`.
  `bundle.rs::charter_drift` at `engine.rs::spawn_site` refuses a dispatch
  whose role bytes or link target moved since the compile. The resume door
  is proved in `capability_verbs.rs::a_resume_reads_no_active_input_the_run_did_not_pin`
  (this visit): a changed pinned charter is `pins a different bundle:
  changed: roles/work.md`; a charter relocated under `capabilities/` is the
  full ruling-5 refusal for pinned and changed bytes alike, no event is
  appended, and the restored tree resumes. Three fixtures that read an
  inline role from OUTSIDE their bundle now carry it inside. *Limitation
  recorded:* an agent's charter is pinned by its library record and
  compared at recompile, not at this per-dispatch door.
- **M1 (8.3, 8.4).** `capabilities::Denial` + `NativeCapability::denial()`
  is the shared assessment; doctor judges it before describing a grant, in
  the native line and in the restriction paragraph.
- **M2 (3.1).** `agents/load.rs::read_request_source` and
  `compose.rs::read_layers` parse the original bytes with `parse_strict`.
- **M3 (3.7, 6.2).** `Bundle::assemble` lints the loaded library right
  after it loads (`CompileError::Capability`); every loaded agent's asked
  names join the manifest's consulted set. Knock-on: every bundle that
  loads the shipped library now pins `web-search` and `web-fetch`
  (10.2), and a map directory without those definitions refuses the
  compile — the fixture in `cli/src/tests.rs::resume_compilation_reads_the_dialect_from_the_pinned_world`
  now carries them beside its map, as `engine/tests.rs::carry_definitions`
  already did.
- **M4 (4.4).** `provider_compatibility_{refuses_a_requirement,
  drops_a_want_with_its_exact_notice, leaves_an_unused_grant_idle}` and
  `cq1_an_inexpressible_restriction_{refuses_a_requirement,
  drops_a_want_with_its_exact_notice, idles_an_unused_grant}`; each optional
  test's first substantive assertion is the whole notice vector.
- **macOS (3.6).** `agents/tests::Tree::new` keeps its `TempDir` guard and
  canonicalizes the root once; every write and expectation derives from it.
  The Explore sweep over all 23 slice test files found no other added
  assertion comparing a lexical fixture root against canonicalized output
  (helpers checked: `capability_launch::Operator`, the cli
  `agent_readouts`/`capability_verbs`/`realms`/`recipes` workspaces,
  `bundle/tests::Fixture`, doctor `workspace_with`, `init_stacks`
  `scaffold_from`, `capabilities/tests` `cq1_root`; `compose_tests::Library`
  was already canonical; the new resume test canonicalizes its bundle path
  for the same reason). The macOS host itself has not run this tree (11.2).

### Removal proofs (9.1–9.4)

| # | Mutation (production unless stated) | Intended assertion | Observed |
| --- | --- | --- | --- |
| A | provider compatibility itself removed (`if false && bound != provider`), notice recording and OFF composition intact | `provider_compatibility_drops_a_want_with_its_exact_notice`, whole-vector notice equality | FAILED at `capabilities/tests.rs:1171`, its FIRST substantive assertion: left `[]`, right the one `dropped … because provider 'claude' cannot carry a binding to provider 'test-native'; native capability remains OFF` notice. Restored. |
| B | restriction compatibility itself removed (`if false && !grant.restrictions.is_empty()`) | `cq1_an_inexpressible_restriction_drops_a_want_with_its_exact_notice` | FAILED at `:1459`, first substantive assertion: left `[]`, right the `cannot express restriction 'allow.hosts'` notice. Restored; 32 capabilities tests green; `git diff --stat capabilities.rs` empty. |
| R-H1a | floor removed at its single source (`known_powers` arms renamed) | `a_known_native_power_with_no_valid_denial_refuses_the_seat` | FAILED at `capability_launch.rs:1577`: left the launched argv with NO `web_search` OFF (the delivered H1 defect), right the full refusal with the load failure as cause. |
| R-H1b | the compiler's floor lookup alone removed, the driver's kept | same | FAILED at `:1577`: the shared composer still refuses, but with `declares the provider's inventory unmeasured` — the original cause is lost, and the exact-cause equality detects it. |
| R-H1c | malformed-plan rejection made lenient (required arrays) | `a_plan_is_read_whole_and_a_malformed_one_refuses_naming_its_fault` | FAILED at `native_controls/tests.rs:237` on `'on' is missing`: left `Ok(Some(Controls{held: [] …}))`, right the full refusal. |
| R-H2 | authored-server guard disabled (`.filter(\|_\| false)` in `compose_for_provider`) | `an_authored_capability_server_refuses_the_compile_and_the_engines_hands_still_launch`; `an_authored_capability_server_is_refused_by_provenance_and_never_by_its_bytes` | FAILED at `capability_launch.rs:1826`: left the launched argv carrying `-c mcp_servers.ungranted.command="npx" … -c web_search="disabled"` under `grants: {}` (the delivered H2 defect); protocol FAILED at `:853` (`Ok(Composed…)` where `Err(Refusal…)`). |
| R-H2b | provenance reassembly check disabled | `provenance_is_a_recorded_fact_that_must_reassemble_the_argv` | FAILED at `:1378`: left `Ok((["--sandbox","read-only"], []))`, right the full refusal. |
| R-H3a | Claude's managed-argv consumption disabled | `a_native_control_declared_as_argv_reaches_the_final_claude_command`; `every_control_representation_reaches_the_composed_command_or_refuses` | FAILED at `capability_launch.rs:1926` on the FINAL boxed Claude argv: `WebFetch` where `WebFetch,WebSearch` (the delivered H3 defect); protocol FAILED at `:1084`. |
| R-H3b | selection contribution dropped (empty `Selection`) | four protocol tests | `a_local_claude_permission_is_kept_under_every_spelling_of_its_list_flag`, `an_unboxed_claude_seat_holds_a_native_tool_without_gaining_a_tool_list`, `claude_admits_only_held_native_tools_beside_its_hands`, `every_control_representation_…` all FAILED. |
| R-H3c | restriction transport dropped (`verbatim.take(0)`) | `every_control_representation_…` | FAILED at `:1160`: left `[…"--disallowedTools","WebFetch"]`, right `[… "--search-policy","{\"allow\":{\"hosts\":[\"yaml.org\"]}}"]`. |
| R-cold | cold OFF composition removed (`codex_cold`) | `a_cold_codex_argv_carries_the_off_pair_exactly_when_search_is_not_held`; `a_codex_seat_that_does_not_hold_search_…` | Both FAILED on the whole boxed argv (`adapters/tests.rs:10631`). |
| R-resume | actual-resume OFF removed | `an_eligible_codex_resume_reimposes_the_capability_control`; `an_eligible_rejoin_of_a_compiled_codex_seat_carries_the_control_either_way_round` | Both FAILED on the whole `exec resume … <session> -` argv (`:10699`). |
| R-replace | cold replacement built without its control | `a_harness_refused_rejoin_is_replaced_by_a_cold_spawn_that_stays_denied`; `a_launch_with_no_computed_authority_…` | FAILED at `:10869` on what the shim RECEIVED (`exec --json -C <dir> --sandbox read-only`, no pair); `:11161`. |
| R-ON | authorized ON replaced by unconditional OFF (`native_plan`) | positive controls | four `capability_launch` tests FAILED, including `a_codex_seat_that_holds_search_is_launched_without_the_off_pair` (1 OFF pair where 0). |
| R-role | role fence only (`.filter(\|_\| false)` in `parse_role`) | `a_charter_under_a_tree…`, `no_spelling…` | FAILED at `compose_tests.rs:1501` and `:1606`; `a_policy_under_a_tree…` stayed green. |
| R-policy | policy fence only (same in `own_table`) | `a_policy_under_a_tree…`, `no_spelling…` | FAILED at `:1541` and `:1616` (policy half); the charter test stayed green. |
| R-resume-role | role fence only, this visit | `a_resume_reads_no_active_input_the_run_did_not_pin` | FAILED at `capability_verbs.rs:334`: the relocated charter COMPILED and the resume left through the mismatch door blaming `bundle.json` instead — a charter edited in place under `capabilities/` would have reached the seat. Restored; 5 verb tests green. |
| R-drift | dispatch door disabled | `a_charter_that_moved_since_the_compile_refuses_the_dispatch` | FAILED at `boundary_tests.rs:977`: left `Ok(())`, right `Err("dispatch refused: a charter of layer 'base' moved since the compile (changed: roles/work.md); …")`. |
| R-doctor | grant-first bypass restored (first arm `(Some(held_by), _, _)`) | `a_matching_grant_never_promises…`; `installed_harnesses_…` | FAILED at `capability_tests.rs:362` on the all-combinations equality, and on the complete-line equality. |
| R1 | `agents/load.rs` strict parse removed | `a_request_key_written_twice_in_an_agent_source…` | ONLY that test FAILED, at the equality (`tests.rs:400`): left `loaded {"web-search": Wants}`. |
| R2 | `compose.rs::read_layers` strict parse removed | `a_request_key_written_twice_in_a_recipe_layer…` | ONLY that test FAILED (`compose_tests.rs:1413`): left the composed map with both wants. |
| R3 | compile lint disabled, CLI lint untouched | `an_unseated_loaded_agent_with_an_undefined_request_refuses_the_compile` | FAILED at `agent_tests.rs:754`: left `compiled 'fixture'`. |
| R4 | consulted-unseated-definition contribution removed (`library_asks.take(0)`) | `a_definition_only_an_unseated_agent_names_is_pinned…` | FAILED at `:806`: left `[]`, right `["web-search"]`. |
| lexical | fixture root used lexically (the macOS habit) | `a_native_declaration_with_a_repeated_key…` | FAILED at its first full-equality assertion with the `real`/`alias` mismatch; canonical root restores it. |

The three earlier removal-found gaps stay discriminating and were re-run:
L1 (adapter duplicate-key parsing) fails only its own test; D1 (sequence
step fallback taking the selected link's own plan, Codex primary to DSH
fallback) fails at the fallback with the Codex plan where DSH's unmeasured
plan is demanded; R2 (unmapped resume reading the workspace) fails only
after the decoy is removed, as the test now removes it.

One observation beside a mutation: R-H2b's run also showed 65 `PoisonError`
siblings rooted in ONE `Text file busy (os error 26)` panic in
`a_class_that_cannot_travel…` — the known flake from main, issue #255 /
PR #313, not this mutation and not repaired here.

### Launch matrix (7.5), actual test names

| Dimension | Tests |
| --- | --- |
| Inline and agent-backed; work and gate | `a_known_native_power_with_no_valid_denial_refuses_the_seat`, `an_agent_backed_link_on_legacy_adapter_data_refuses_too`, `a_codex_seat_that_does_not_hold_search_is_launched_with_it_switched_off`, `every_site_of_every_shipped_bundle_holds_nothing_and_has_its_native_powers_denied` (`capability_launch.rs`); `the_shipped_inline_codex_work_seats_carry_the_preserved_assessment` (`inline_resume.rs`) |
| Ordinary, panel member, sequence step | `a_panel_member_and_a_sequence_step_resolve_under_their_own_labels`, `an_authored_capability_server_refuses_the_compile_and_the_engines_hands_still_launch` (panel); `every_nested_dispatch_hands_its_driver_the_selected_links_own_controls`, `a_driver_input_carries_the_serving_candidates_controls_or_a_refusing_null` (`engine/capability_tests.rs`, Codex-to-DSH fallback included) |
| Selected, inherited, nested/wrapped | `an_office_is_inherited_subset_and_emptied_the_same_way_in_every_body`, `an_ungranted_requirement_refuses_compilation_at_every_site_form`; `a_charter_that_moved_since_the_compile_refuses_the_dispatch` (standalone + inherited) |
| Boxed and unboxed | `a_codex_that_could_not_switch_search_off_is_unseatable_boxed_and_unboxed`, `a_native_control_declared_as_argv_reaches_the_final_claude_command` (boxed, hands and strict config retained), `an_unboxed_claude_seat_holds_a_native_tool_without_gaining_a_tool_list`, `claude_admits_only_held_native_tools_beside_its_hands` (`native_controls/tests.rs`, `adapters/tests.rs`) |
| Cold, eligible resume, cold replacement | `a_cold_codex_argv_carries_the_off_pair_exactly_when_search_is_not_held`, `an_eligible_codex_resume_reimposes_the_capability_control`, `a_harness_refused_rejoin_is_replaced_by_a_cold_spawn_that_stays_denied`, `a_launch_with_no_computed_authority_…` (`adapters/tests.rs`); `an_eligible_rejoin_of_a_compiled_codex_seat_carries_the_control_either_way_round`, `a_codex_seat_that_holds_search_is_launched_without_the_off_pair` |
| Control representation | `every_control_representation_reaches_the_composed_command_or_refuses`, `a_known_native_power_is_composed_only_under_a_plan_that_answers_for_it`, `a_plan_is_read_whole_and_a_malformed_one_refuses_naming_its_fault`, `a_selection_folds_into_the_seats_own_lists_and_emits_each_flag_once`, `provenance_is_a_recorded_fact_that_must_reassemble_the_argv` |

### Pins measured on the final tree (10.2)

Measured by compiling each bundle exactly as `witness_digests.rs` and
`compose_tests.rs` do (`Bundle::compile_with` over the workspace `agents/`
and `adapters/`, no-grant context). Five moved, all for ONE reason —
decision 0066 ruling 8: a compile that loads the shipped library resolves
every loaded agent's asks, so the researcher's `web-search` and `web-fetch`
definitions are now consulted and pinned by bundles that load the library
without seating it. No charter, table, adapter, grant or README moved.

| Bundle | Was | Is | Why |
| --- | --- | --- | --- |
| `recipes/night-shift` | `f868685b…` | `11bd9bec…` | loads the library, seats no researcher; definitions `[web-fetch, web-search]` newly pinned |
| `recipes/triage` | `357e1889…` | `e4f24ee6…` | same (both pins, witness and compose, agree) |
| `recipes/gpt-flash` | `9fa653f7…` | `d434415b…` | same |
| `recipes/panel-review` | `4cc63d62…` | `11cb41f0…` | same |
| `bundles/self` | `04520449…` | `596541a8…` | same |
| `recipes/fast`, `recipes/node`, `recipes/preflight`, `recipes/wager-harness`, `bundles/verify` | unchanged | unchanged | load no library |
| `recipes/research` | `cd9b978b…` | unchanged | seats the researcher, so it already pinned both definitions |
| `recipes/research-dsh` | `22d9f849…` | unchanged | loads no library (inline seat) |

The `fast` ancestor layer's digest under `triage` did not move, as the
compose test states: a layer holds no capability. The `brokkr compile` verb
prints a DIFFERENT digest for the same bundle in this repository — it
resolves the repository's realm `brokkr` from `realms.json`, and the realm
name is part of the pinned section — so the verb's output is not the tests'
pin and was not used for one.

### Scoped-diff review (10.3)

`git diff --stat e5408669 -- policy fixtures reference extensions contracts
realms.json agents adapters capabilities dialects recipes bundles` is
empty: no protected or identity-bearing path moved, the repository's map
stays `forge.realms/v3` granting nothing, the researcher still loses both
wants with the applicable native OFF, and the `mcp` dialect still refuses
at compile. The repair touches `crates/` (production and tests), the
proposed decision and its index row, four guides, and this change's
artifacts.

### Guides (10.1)

`docs/guides/provider-adapters.md` (the known-power floor and what refuses
under it; one composer; provenance and the authored-server refusals; the
counterfeit-hands migration; DSH's bound `--patch`),
`docs/guides/agent-library.md` (strict request bytes; whole-library lint and
what a map directory must define), `docs/guides/recipe-authoring.md`
(consequence 4: active inputs under a skipped top-level name or out of the
layer refuse; the dispatch door), `docs/guides/read-surfaces.md` (doctor
judges OFF before describing a grant). Each sentence describes an observed
refusal or a measured argv; none claims live enforcement.

### Gates on the final head (11.1–11.4)

Run over the final tree on Linux (kernel 6.17, cargo 1.98.0, the pinned
`nightly-2026-09-05` for coverage), after the last edit and before the
commit that carries them; the commit's revision is named in the result
record and in 11.5 below. The second visit's run of every gate but coverage
was green on the tree before the coverage repair; the counts below are the
third visit's, on the repaired tree.

**The first coverage verdict, and the four gaps it named.** The first
complete run of `bash scripts/coverage-exact.sh` on this tree refused:
lines 35074 of 35075, branches 5744 of 5744, functions 3451 of 3454.
Named from `target/coverage/lcov.info` and the JSON report, grouped as the
script groups them (file + start line, any hit instance covering):

| Gap | Where | What it was | Repair |
| --- | --- | --- | --- |
| line | `bundle.rs::folded`, the `Component::CurDir => {}` arm | Unreachable: `Path::components` already drops every `.` but a leading one, and every path folded here is absolute — a reference joined to its layer's canonical root, or the role path the compile wrote. | The arm is gone and the doc says why; `..` is the only component left to fold. No caller's behaviour changes: the four H4 refusal tests and the dispatch door pass unchanged. |
| function | `doctor.rs::report_capabilities`, the `\|\| workspace.to_path_buf()` fallback for a realms map with no parent directory | Unreachable: the map is a file, so the directory it stands in is always there. | The root is derived as `engine.rs` derives it, `.map(Path::to_path_buf).unwrap_or_default()`, with no closure. |
| function | `adapters.rs::composed_launch`, the `\|refusal\| refusal.at_launch(input)` mapping | Reachable and UNPROVED: no launch-level test ever handed a driver an authored capability server, so the composer's refusal in the driver's voice — decision 0066 ruling 2 at the H2 door — had no regression of its own; the compile refused first everywhere. | `adapters/tests.rs::an_authored_capability_server_is_refused_at_launch_in_the_drivers_own_voice`: Codex cold and rejoining, Claude cold, holding its powers or not, each of the council's spellings and the counterfeit hands, asserting the whole refusal; a by-hand launch keeps its argv. **Removal:** the composer's server guard disabled (`authored_server_conflict(..).filter(\|_\| false)`) — FAILED at `adapters/tests.rs:10977`, its first assertion: left `None`, right the full `carry '-c mcp_servers'` refusal for provider `codex`. Restored; `git diff` over `native_controls.rs` empty; green. |
| function | `bundle.rs::record_capabilities`, the `\|\| format!("step-{}", index + 1)` label for a nameless sequence step | Duplicated the label `collect_unpinned` already computes (and proves) for the same step. | One `step_label(index, step)` shared by the pin walk and the capability walk, so both record a step under one spelling. |

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean (no output) |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | clean (`Finished`, no warning) |
| `cargo test -p brokkr-core --all-features --locked` | green: 82 lib + 3 + 8 + 2 |
| `cargo test -p brokkr-store --all-features --locked` | green: 58 lib + 1 + 1 + 1 + 1 + 2 |
| `cargo test -p brokkr-protocol --all-features --locked` | green: 415 lib (the new launch-level regression included) + 99 (2 ignored) + 1 |
| `cargo test -p brokkr-runtime --all-features --locked` | green: 489 lib; every integration binary `ok`, `capability_launch` 17, `witness_digests` 4 over the re-measured pins |
| `cargo test -p brokkr-view --all-features --locked` | green: 243 (3 ignored) |
| `cargo test -p brokkr-bridge --all-features --locked` | green: 13 |
| `cargo test -p brokkr-cli --all-features --locked` | green: 479 lib; every integration binary `ok`, `capability_verbs` 5 |
| `cargo test --workspace --all-features --locked --no-fail-fast` | **77 binaries `ok`, none otherwise**, first run, on the third visit's two runs (before and after the coverage repair). The second visit's first run had ONE failure — `brokkr-protocol` lib `hands::tests::the_network_prefix_is_eight_tokens_and_the_probe_asks_the_dispatchs_path` at `hands/tests.rs:1199` (a planted `unshare` shell script the probe must run answered no); `hands` is untouched by this repair, the crate-scoped run was green, and the shape — a script written then executed under workspace-wide parallelism — is the class issue #255 / PR #313 records, not asserted here as its cause. |
| `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self` | compiles; verb digest `09ff39b7…` (realm `brokkr`, grants `{}`; not the tests' pin, see 10.2) |
| `cargo run --locked -p brokkr-cli -- compile --bundle bundles/verify` | compiles; verb digest `fcbfe0ec…`, unchanged from the chief's measurement |
| `openspec validate --all --strict --no-interactive` | **16 passed, 0 failed (16 items)**; the informational long-requirement notices are unchanged |
| `git diff --check` | clean |
| `bash scripts/coverage-exact.sh` | **passes, literal equality on all three:** source lines **35073 / 35073**, branches **5744 / 5744**, logical functions **3453 / 3453** (`target/coverage/coverage-summary.json`, pinned `nightly-2026-09-05`, `--branch`, its own fresh target directory). Every production line this repair added is in the numerator; nothing was excluded and no threshold moved. The count fell by two lines and one function from the refused run because the two unreachable fallbacks and the duplicated label are gone, and rose by nothing: the new regression is test code. |
| macOS | **not run by this seat** — no macOS host; the alias-root reproduction on Linux is the local proof and the pushed PR's `test (macos-latest)` job is the remote one, pending until the operator pushes |
