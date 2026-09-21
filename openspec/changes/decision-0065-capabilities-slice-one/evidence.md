# Decision 0065, slice one — delivery evidence

Recorded 2026-09-21 by the implement seat of run
`build-decision-0065-slice-one-of-773a4e83`. This file describes what was
observed. It instructs no gate, waives none, and claims no live provider
result: every argv assertion named here is composition evidence.

The two implementation visits below are historical observations. The repair
design addendum at the end supersedes their completion/permission conclusions;
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
