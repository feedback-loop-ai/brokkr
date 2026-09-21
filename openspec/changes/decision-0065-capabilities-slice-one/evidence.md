# Decision 0065, slice one — delivery evidence

Recorded 2026-09-21 by the implement seat of run
`build-decision-0065-slice-one-of-773a4e83`. This file describes what was
observed. It instructs no gate, waives none, and claims no live provider
result: every argv assertion named here is composition evidence.

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

## Not finished (for the next visit; see tasks.md for the unticked rows)

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
