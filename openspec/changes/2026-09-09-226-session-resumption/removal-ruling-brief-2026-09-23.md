# Removal ruling brief — acceptance-ledger entry 17

Issue #226, change `2026-09-09-226-session-resumption`, 2026-09-23. This brief
prepares the operator's ruling for ledger entry 17
(`acceptance-ledger-8.8-8.10.md` §6). **It does not take the ruling.** It ticks
nothing, regrades nothing, and changes no test or production byte. The ruling
is recorded in `tasks.md` once the operator gives it.

## The question

Verbatim from entry 17:

> *a compiling mutation run by a named seat on a named revision, recorded with
> the assertion that parted and the restored pass but leaving no artefact in the
> tree — is that acceptance for a clause that asks for an "observed" removal?*

The operator may answer per row, and a ruling covers only the rows it names.
It does not cover a control with no record (those were entry 21(a)'s, now
closed) or a control never performed (N11, entry 16). This brief does not rely
on `tasks.md:1248`'s "Adopt resolver removals without replay". That sentence
governs 8.8.1.1's resolver alone (ledger F5).

## How to read the table

**Line numbers.** `tasks.md` lines are **current** (`0af3a3c9`, and
unchanged since). The
ledger cites `origin/main`'s numbering. Entry 13 inserted 14 lines after main's
1252 and 23 after main's 1287. So a ledger citation at or below 1252 is
unchanged, 1253–1287 is +14, and 1288 onward is +37. For example, the ledger's
1665–1673 are now 1702–1710, its 12366–12383 are now 12403–12420, and its
10230–10233 are now 10267–10270.

**Source and log.** Source lines are current, and are the bytes entry 14's
gate ran on (below). Log references are to the host's retained logs in
`.forge/tasks/entry14-be1ecf77/`:

| Short | File |
|---|---|
| PL | `timeout_2400_cargo_test__p_brokkr_protocol___all_features___.out` |
| RT | `timeout_2400_cargo_test__p_brokkr_runtime___all_features___l.out` |
| CL | `timeout_2400_cargo_test__p_brokkr_cli___all_features___locke.out` |

`PL:252` means that log's line 252 reads `test … ... ok`.

**Source files.**

| Short | Path under `crates/` |
|---|---|
| A | `brokkr-protocol/src/adapters/tests.rs` |
| C | `brokkr-protocol/src/adapters/composite/tests.rs` |
| D | `brokkr-cli/src/doctor/tests.rs` |
| B | `brokkr-cli/tests/doctor_dsh_selection.rs` |
| DC | `brokkr-cli/tests/driver_conformance.rs` |
| BT | `brokkr-runtime/src/engine/boundary_tests.rs` |

**The records**, cited by short name in the table. Each is located in the
ledger section below.

| Short | Seat and revision | Record |
|---|---|---|
| **PC** | Pass C implement seat, run `dsh-launch-planner-issue-226-tas-bff4c1e2`, candidate `f88535ac` | `tasks.md` 451–489 |
| **RR** | R1–R4 implement seat, run `dsh-composite-identity-issue-226-e291e076`, on `336598b9` | 1695–1710 |
| **DR1** | returned implement of run `…-124cca78`, which adopted `d120dd91` and landed as `3a1df8d3` | 1947–1963 |
| **DR2** | the same run's second sitting, which adopted `3a1df8d3` | 2084–2099 |
| **HM** | first-hold implement seat, run `…-4437331e`, HEAD above `d54a9f7b` | 12387–12420 |
| **CX** | the operator-ruling implementation record on adopted head `21f4ac2`, which names no run | 9111–9151 |

## The class: twenty-nine rows

Recounted from ledger F5 and entry 17: nine N rows, one A row, fifteen S rows
and four B rows.

- N: N2, N4, N5d, N6a, N6b, N7, N8, N9, N10.
- A: A61.
- S: S1, S3, S4, S5, S6, S7, S8a, S8b, S8c, S9a, S9b, S9c, S9d, S9e, S10.
  S2 is not in the class.
- B: B5, B16, B20, B77.

**29 rows. 8.10 needs 19 of them named:** B5, B16, B20 and B77, and the fifteen
S rows through B2. The N rows and A61 are 8.8's.

**PC's shared facts** apply to every PC row below:

- *Command.* Each mutation ran `cargo test -p brokkr-protocol --all-features
  --locked --lib -- <the ten new test names>` (452–453).
- *What "false confirmation" means* (456–460). The named terminal-refusal
  assertion in `assert_refused` (A:5905–5910, `"{label}: the named terminal
  refusal"`) parted. On the clean exit its left was `seat wrote no result
  file…`. On the delivered result it was `None`.
- *Timeouts.* "No removal timed out except M10" (460). M10 is recorded at 479.
- *Restoration.* Each mutation was restored before the next. The restored file
  is SHA-256 `252cb0f7…029ef5`, byte-identical to the committed one, and the
  focused ten rerun `10 passed` (461–463, 493).

In the table, "PL:n" follows each targeted test. Every targeted test exists at
the cited source line and asserts what its record claims, unless a note under
**Observations** says otherwise.

| Row | Clause (current `tasks.md`) | Recorded mutation(s): the assertion that parted and the restored pass | Targeted case(s) now, and entry 14's pass | 8.10 names it? | Ruling |
|---|---|---|---|---|---|
| **N2** | 8.8.1.2, 1267–1283. Removals at 1277–1280: "Independently remove selected-env qualification, then independently restore canonical execution" | **RR M5** (1706): selected-env qualification removed. Protocol failed "expected a refusal", and the built doctor's line lacked the pre-probe cause. **RR M6** (1707): the canonical path substituted as invocation. Protocol failed on `DshInvocation` inequality, and the built doctor's version line printed `lib/launcher.sh`. Restored byte-identical by `cmp`, one at a time (1695–1698). Also entry 12's two mutations on the "no second search" cell, which are B5's reach (ledger 1651–1656) | `C::the_selected_invocation_is_not_replaced_by_its_canonical_target` C:3053. M5's refusal is at C:3090, M6's invocation assertion at C:3160–3166, PL:159. `B::a_dsh_alias_of_env_is_refused_and_an_admitted_alias_runs_as_selected` B:2771. M5's assertion is at B:2838–2843 and M6's at B:2890–2894, CL:620. See observation 3 | no (8.8) | yes / no |
| **N4** | 8.8.2.2, 1325–1343. Removal at 1333–1335: "Independently restore blank-tail `Ok(None)` admission: selection and callback tests fail" | **RR M4** (1705): blank tail `Ok(None)` restored. The classifier failed "expected a refusal" for the bare-env row, and nothing was spawned. The callback-test half is entry 12's: blank-tail `Ok(None)` parted D:2836 with nothing spawned (landed `36b16922`, ledger 1661–1662) | `C::the_candidate_classifier_stops_where_the_child_stops_and_refuses_the_unprovable` C:5621. The bare-env row is C:5796 and `refused` is C:5810, PL:166. `D::an_env_launcher_without_a_program_reaches_neither_doctor_callback` D:2827, with its probe `panic!` at D:2836, CL:76 | no (8.8) | yes / no |
| **N5d** | 8.8.3.1(4), 1392–1401: one-space consumption, the raw-span guard removed, the guard moved after trimming, admission bypassed before DSH and before Node, and retained input reopened | **RR M1** (1702): producer `{node:  *missing}` "was accepted"; built doctor markers `["dsh","node"]` ≠ `[]`. **M2** (1703): "expected a refusal" at the 1,025 row, and doctor markers ≠ `[]`. **M3** (1704): the same two tests, at the padded-over-limit row. **M7** (1708): doctor unit call counts `(1, 0)` ≠ `(0, 0)`, and built-doctor markers `["dsh"]` ≠ `[]`. **M8** (1709): Node probe count `1` ≠ `0`. **M9** (1710): retain-and-reuse composite inequality. Restored as for N2 | `C::missing_pnpm_field_separation_and_unsupported_flow_syntax_refuse_by_reason` C:4310, rows at C:4632–4645 and C:4703–4728, PL:150. `B::ignored_pnpm_values_are_admitted_as_syntax_through_the_built_doctor` B:2256, probes at B:2603–2607, CL:628. `D::a_refused_pnpm_lock_stops_the_line_before_either_probe` D:2920, `(0,0)` at D:2969–2973, CL:36. `C::the_pnpm_lock_is_admitted_before_any_probe_and_composed_as_retained` C:3229, with M8 at C:3329 and M9 at C:3266–3269, PL:98 | no (8.8) | yes / no |
| **N6a** | 8.8.4.1, 1413–1423. Removal at 1420–1423: "Remove the heading-set rejection and observe those named assertions fail; restore it and rerun positive and negative controls" | **HM M8** (12414): `seen_packages.insert` no longer refuses a repeat. The test failed `"identical" was accepted`. Every restored rerun is green in HM's final gates table (12395–12397, 12433–) | `C::duplicate_decoded_pnpm_package_keys_refuse_before_triple_normalization` C:5470, `refused_vector` at C:5548, PL:39 | no (8.8) | yes / no |
| **N6b** | 8.8.4.1's two extensions, 1428–1438: "proved by its own removal (D1–D3)" and "proved by removals D4–D5" | **DR1 D1–D3** (1957–1959): the flow-map, block-key and package-child guards were each disabled. Protocol failed `… was accepted`, and doctor reported the control's composite `f742ba0e…`. **DR2 D4–D5** (2094–2095): the typed-key refusal disabled, then the separator trim removed, with the same two failures. Each was restored exactly, 1 passed each (1947–1952, 2084–2089) | Protocol in C:4310, rows C:4515–4575, PL:150. Doctor in B:2256, rows B:2429–2476, CL:628. See observation 4 | no (8.8) | yes / no |
| **N7** | 8.8.5.1, 1442–1453: "Alter production path/line ordering in a compiling mutation and observe the pinned expectation fail" | **HM M9** (12415): `component_digest` iterates `digests.iter().rev()`. The test failed at "the producer's recorded output over the synthetic six-file set", `f193cd95…` against `8894f23e…`. Also D2's first row (809): the same reversal parted the worked plugin vector's pinned component, and was replayed on D2's finished candidate (827–831) | `C::the_plugin_component_is_bytewise_path_order_and_fails_closed` C:806, assertion C:824–827, PL:94. `C::the_worked_plugin_vector_pins_the_bytewise_path_order_of_the_component` C:10801, assertions C:10833–10842, PL:104 | no (8.8) | yes / no |
| **N8** | 8.8.5.2, 1455–1463: "Restore that exact test oracle in a compiling mutation: the conformance assertion must fail even if runtime digest equality still passes" | **HM M10** (12416), a test mutation: the inherited NUL oracle restored. The component test passed. `no_test_reassembles_the_component_stream` failed "tests.rs:686 pushes a NUL separator…". Also the D3 returned review's guard extension (1058–1066): run against `859e5b07`'s source, it failed naming `tests.rs:11282` | `C::no_test_reassembles_the_component_stream` C:4248, assertions C:4263–4267 and C:4279–4291, PL:123 | no (8.8) | yes / no |
| **N9** | 8.8.6.1, 1470–1478: "Remove safe rendering in a compiling control and fail the actual output assertion" | **HM M11** (12417–12418): `Safe` dropped from both interpolations. The built binary failed "a raw escape byte reached stdout", and the unit test's raw newline split the line | `B::a_nonexistent_override_cannot_inject_terminal_control_bytes_through_doctor` B:2004, assertion B:2015, CL:617. `D::the_unavailable_line_escapes_the_binary_and_the_selection_cause` D:3121, assertion D:3150–3153, CL:114 | no (8.8) | yes / no |
| **N10** | 8.8.7.1, 1485–1495: "Remove filename context in a compiling mutation, observe the producer-facing reason assertion fail" | **HM M12** (12419–12420): the exhausted lookup reads `bundle '<name>' does not resolve`. The producer failed `left: "…does not resolve"` against `right: "…: no package.json found"`, and the built binary failed "the bundle and the file" | `C::removing_only_the_plugin_manifest_names_the_drifted_file` C:5579, assertion C:5587–5591, PL:66. The built binary's test of the same name is B:2106, assertion B:2142, CL:618 | no (8.8) | yes / no |
| **A61** | 8.8 prose, 5067–5069: "Verify the six headline child proofs, completed-observation witnesses and all D7 rule controls/removals through the real terminal body" | All fourteen **PC** rows (465–480): M1a, M1b, M2, M3, M4, M5, M5b, M5+M7, M6, M6′, M8, M9, M10 and M12, with the shared facts above | The ten PC tests and four inherited ones. Every one is `ok`: PL:252, 244, 248, 230, 295, 236, 250, 241, 251, 237, 174, 218, 262 and 435. Names are in the ledger section below | no (8.8) | yes / no |
| **S1** | 8.8.9.1, 135–143. Removal at 142–143: "Prove the controls' publication/terminal assertions by applicable removal and restored rerun" | **PC M10** (479): consistent-but-incomplete evidence refuses. The positive "pre-init noise, a sibling and delayed activity" failed accepted, and the unreadable-line case lost its `[Pending, Refused, Refused]` order. One child waited out its 30 s bound and failed `every awaited observation was really completed`, which is recorded as a timeout and not read as a refusal. **PC M12** (480): `unread` allowed to refuse a cold launch. The cold control (`rooted`) and `a_qualified_stream_json_launch_skips_a_malformed_line_and_still_confirms` both failed | `A::dsh_a_consistent_pending_rejoin_still_confirms_through_the_terminal_body` A:6590, case at A:6593, PL:251. `A::dsh_an_unreadable_stream_line_refuses_a_pending_rejoin_before_the_stream_ends` A:6390, PL:244. `A::dsh_cold_noise_and_a_root_mismatch_keep_their_terminal_behaviour` A:6756, PL:237. `A::a_qualified_stream_json_launch_skips_a_malformed_line_and_still_confirms` A:7158, PL:174. The M10 timeout's assertion is A:5950–5953 | yes, through B2 | yes / no |
| **S3** | 8.8.10.1, 156–163. Removal at 161–163: "baseline-repair controls with applicable compiling removals" | **PC M6** (475): the refusal in `new` removed. All five baseline cases, both endings, lost `ended refused` (`left: Some(Pending)`). The record discloses that the terminal reason does not change. **PC M6′** (476): the baseline header resolved to its first match. "ambiguous baseline, the first header removed" was falsely confirmed on both endings | `A::dsh_a_rejoin_without_its_pre_spawn_baseline_is_refused_before_the_child_runs` A:6429, cases A:6440–6472, PL:241. The watcher-state assertion is in `refused_on_both_endings`, A:6081–6085 | yes, through B2 | yes / no |
| **S4** | 8.8.10.2, 164–174. Removal at 170–172: "…controls with applicable removals/restoration" | **PC M5** (472): id-set comparison. R3, alias and replaced-at-new-address were falsely confirmed. "Offered header moves" stayed refused. **M5b** (473): pair SET. The alias alone was falsely confirmed. **M5+M7** (474): adds "offered header moves", with the overlap disclosed. **M9** (478): whole-store equality. The positive "an unrelated baseline sibling is gone" failed `a confirmed rejoin is accepted` | `A::dsh_census_identity_counts_occurrences_and_addresses_not_distinct_ids` A:6524, cases A:6527–6546, PL:250. `A::dsh_a_fresh_entry_reusing_a_sibling_id_at_a_new_address_refuses_the_rejoin` A:6211, PL:230. `A::dsh_a_consistent_pending_rejoin_still_confirms_through_the_terminal_body` A:6590, case A:6609, PL:251. M9's parted assertion, `"{label}: a confirmed rejoin is accepted"`, is A:6630–6634. The case's own containment assertion ("containment, not whole-store equality") is A:6729–6735, which the record does not name | yes, through B2 | yes / no |
| **S5** | 8.8.10.3, 175–184. Removal at 181–182: "Verify each former early return … with focused controls/removals" | **PC M3** (470): a post-init refusal becomes a pending return. R2 and five post-init cases were falsely confirmed on both endings. **PC M4** (471): containment skipped before init. "fresh sibling before init" was falsely confirmed on both endings | `A::dsh_contradictions_after_the_init_event_are_never_restored_away` A:6258, cases A:6261–6300, PL:295. `A::dsh_an_observed_fresh_sibling_refuses_the_rejoin_after_it_disappears` A:6161, PL:248. `A::dsh_a_fresh_sibling_observed_before_the_init_event_refuses_the_rejoin_for_good` A:6357, PL:236 | yes, through B2 | yes / no |
| **S6** | 8.8.11.1, 188–196. Removal at 194–196: "each removal must fail the intended terminal assertion, **not time out**" | **PC M1a** (467): R1 and "store unmoved", clean and delivered, falsely confirmed. **M1b** (468): "store unmoved" falsely confirmed. R1 stayed refused and lost only the `census: None` witness, so R1 is jointly protected and M1a is its removal. **M2** (469): "unreadable line while pending" falsely confirmed. The record says no removal timed out except M10 (460), which is S1's. **No elapsed time is recorded.** The clause asks for a property *of* the failure, and the record supplies it only as that sentence | `A::dsh_malformed_output_before_the_init_event_refuses_the_rejoin_for_good` A:6112, cases A:6116 and A:6124, PL:252. `A::dsh_an_unreadable_stream_line_refuses_a_pending_rejoin_before_the_stream_ends` A:6390, PL:244. The intended assertion is A:5905–5910 | yes, through B2 | yes / no |
| **S7** | 8.8.11.2, 197–204. Removal at 201–203: "Remove only the observation-before-skip protection while keeping witness acknowledgment active; require the terminal assertion to fail" | **PC M8** (477): no store reading behind a malformed post-init line, with acknowledgment still firing. "sibling read on a malformed line" was falsely confirmed on both endings, and the positive "malformed noise after init" lost `confirmed on Malformed`. PC M3 (470) also covers the same case | `A::dsh_contradictions_after_the_init_event_are_never_restored_away` A:6258, case A:6300, PL:295. `A::dsh_a_consistent_pending_rejoin_still_confirms_through_the_terminal_body`, case A:6617 and assertion A:6708, PL:251 | yes, through B2 | yes / no |
| **S8a** | 8.8.12.1, 208–216. Removal at 212–214: "Remove malformed pre-init refusal and observe each terminal/publication assertion fail" | **PC M1a** (467), as for S6 | `A::dsh_malformed_output_before_the_init_event_refuses_the_rejoin_for_good` A:6112, "R1 malformed pre-init work" at A:6116, PL:252 | yes, through B2 | yes / no |
| **S8b** | 8.8.12.2, 217–226. Removal at 223–226: "Remove only the permanent contradiction transition…" | **PC M3** (470): R2, clean and delivered, falsely confirmed after the acknowledged repair | `A::dsh_an_observed_fresh_sibling_refuses_the_rejoin_after_it_disappears` A:6161, case A:6164, PL:248 | yes, through B2 | yes / no |
| **S8c** | 8.8.12.3, 227–236. Removal at 232–235: "Mutate counted containment to the former ID-only comparison" | **PC M5** (472): R3, clean and delivered, falsely confirmed | `A::dsh_a_fresh_entry_reusing_a_sibling_id_at_a_new_address_refuses_the_rejoin` A:6211, case A:6214, PL:230 | yes, through B2 | yes / no |
| **S9a** | 8.8.13.1, 240–248. Disclosure at 245–247: "isolate/disclose overlapping cardinality/identity protection in removals" | **PC M3** (470): offered header missing and ambiguous after init, falsely confirmed on both endings. The record discloses at 482–486: "Not separately removable, and said so: the CURRENT offered-header cardinality check after init… M3 proves its permanence, not its independence" | `A::dsh_contradictions_after_the_init_event_are_never_restored_away` A:6258, cases A:6261 and A:6272, PL:295 | yes, through B2 | yes / no |
| **S9b** | 8.8.13.2, 249–258. Removal at 255–257: "applicable guard removals that permit later false confirmation" | **PC M3** (470): census fails, and the sequence was unreadable after init. **PC M6** (475) and **M6′** (476): the baseline cases, as for S3 | `A::dsh_contradictions_after_the_init_event_are_never_restored_away`, cases A:6281 and A:6290, PL:295. `A::dsh_a_rejoin_without_its_pre_spawn_baseline_is_refused_before_the_child_runs` A:6429, PL:241 | yes, through B2 | yes / no |
| **S9c** | 8.8.13.3, 259–266. Removal at 262–263: "remove the pre-init contradiction protection to expose later false confirmation" | **PC M4** (471): "fresh sibling before init" falsely confirmed on both endings | `A::dsh_a_fresh_sibling_observed_before_the_init_event_refuses_the_rejoin_for_good` A:6357, case A:6360, PL:236 | yes, through B2 | yes / no |
| **S9d** | 8.8.13.4, 267–276. Removal at 272–274: "applicable pair-set/occurrence-reuse and equal-total/identity mutations, disclosing overlapping offered-address protection" | **PC M5** (472), **M5b** (473) and **M5+M7** (474), with the overlap disclosed in 474: "counted containment alone also refuses a moved offer, so the address check is only separable jointly" | `A::dsh_census_identity_counts_occurrences_and_addresses_not_distinct_ids` A:6524, cases A:6527, A:6536 and A:6546, PL:250 | yes, through B2 | yes / no |
| **S9e** | 8.8.13.5, 277–287. Removal at 286–287: "run applicable removals on new assertions to exclude unconditional refusal or whole-census equality" | **PC M9** (478), **M10** (479) and **M12** (480), as for S4 and S1 | `A::dsh_a_consistent_pending_rejoin_still_confirms_through_the_terminal_body` A:6590, PL:251. `A::dsh_cold_noise_and_a_root_mismatch_keep_their_terminal_behaviour` A:6756, PL:237. `A::a_qualified_stream_json_launch_skips_a_malformed_line_and_still_confirms` A:7158, PL:174 | yes, through B2 | yes / no |
| **S10** | 8.8.14.1, 291–299: the "task -> scenario -> actual test/ending -> mutation -> failed assertion -> restored pass ledger", and "Inspect the diff to ensure no mutation… survives" | The whole **PC** ledger, 451–489, with its fourteen rows and the disclosure paragraph at 482–489. The diff-inspection half is re-confirmed by this brief (below) | All fourteen PC-targeted tests, as for A61 | yes, through B2 | yes / no |
| **B5** | 8.10, 5135–5136: "Every new test needs an observed compiling mutation failure at its claimed assertion and a restored pass" | Every recorded ledger below: PC's fourteen; RR's M1–M9, DR1/DR2's D1–D5 and HM's M1–M12, the R1–R4 group's records, which entry 17's list names and the ledger's B5 cell does not (observation 9); Pass D's D1, D2, D3 and returned-review records; entries 1–10's own records; entries 11, 12 and 13-fix's records, which entry 17's list omits but B5 reaches (observation 8); `4a3854ca`'s three Codex removals, which **name no assertion**; and task 10.5's M8, M8a, M9 and M9a. The last four each failed `tests.rs:2030:43 <case>: must not enable` and were restored to "1 passed" (10267–10270). Not reached by a ruling: the ten 21(a) controls, closed under the operator's F1–F4 ruling | Every targeted case exists and reads `ok` (the ledger section below). 10.5's target is `A::a_supported_assessment_without_both_affirmative_markers_declines` A:1987, whose decision arm is at A:2086–2089, PL:200. `4a3854ca`'s targets are `DC::the_compiled_live_inline_codex_shapes_rejoin_their_provider_confirmed_root` DC:3194, CL:663, and `DC::the_compiled_hands_inline_codex_shapes_refuse_unavailable_confinement` DC:3297, CL:664. See observation 6 | **yes** | yes / no |
| **B16** | 8.10, 5156–5162: "…with separately observed disabled-status, boxed-hands, harness-fragment and boundary-mark mutation failures" | **CX** (9134–9138), with the same four restated in 11.1's prose (5979–5985). The shipped status reverted to `unmeasured` failed the retry with `resume_refusal: unsupported-resume`. Declared hands reverted to `boxed` failed it cold with `restrictions-unavailable`. Suppressing `compose_site`'s harness fragment, and separately `mark_hands`'s boundary write, "each failed the bridge". "Every mutation was restored and rerun green." **The record names no run.** For the two bridge controls it names no assertion | `BT::the_shipped_codex_harness_work_seat_composes_the_preserved_rejoin` BT:1695, RT:275. Its fragment assertion is BT:1758–1763 and its boundary assertion BT:1780, but the record does not say which parted. `DC::the_shipped_codex_harness_work_seat_rejoins_its_retry` DC:2250, CL:656: `launch_row(&resumed, "resumed")` at DC:2374 asserts at DC:2783. `DC::the_shipped_inline_codex_work_seat_rejoins_its_retry` DC:2434, CL:655 | **yes** | yes / no |
| **B20** | 8.10, 5167–5172: "pair its selector mutation with the existing protocol adapter test `a_refused_resume_is_a_cold_spawn_with_the_refusal_journaled`, verifying two children, no selector and the retained sandbox in the replacement argv, and one cold launch row" | **CX Low 3** (9150–9151): "The cold-selector test failed (guard returned `None`) with the selector guard bypassed. Restored." The commission is at 9017–9035. **The record does not say the paired test was run beside the mutation** | `A::a_codex_seat_argv_that_selects_a_session_is_refused_on_the_cold_path_too` A:1827. Its guard assertion `codex_selector_conflict(&extra) == Some(part)` is at A:1845, PL:180. The paired `A::a_refused_resume_is_a_cold_spawn_with_the_refusal_journaled` A:2768 asserts two children (A:2801), no selector (A:2803), the retained sandbox (A:2804) and one cold launch row (A:2810–2816), PL:231 | **yes** | yes / no |
| **B77** | 8.10, 5395–5400: "…and each regression/ending's compiling removal/restoration" | All fourteen **PC** rows, as for A61 | As for A61 | **yes** | yes / no |

## The mutation ledgers, located

Current `tasks.md` lines unless a file is named. Where the ledger's count
disagrees with the table it counts, both are given.

| Ledger | Where | Seat, revision | Restored pass |
|---|---|---|---|
| Pass C, **fourteen** rows (M1a, M1b, M2, M3, M4, M5, M5b, M5+M7, M6, M6′, M8, M9, M10, M12), and the overlap disclosure | 465–480 (method 451–463, disclosure 482–489) | run `dsh-launch-planner-issue-226-tas-bff4c1e2`, `f88535ac` | 461–463, 491–493 |
| Pass D D1's two (byte-form pin) | 616–623 | run `dsh-pass-d-part-one-of-three-the-0357f090`, D1 head `4d6b15f3` | "reverted"; gates 639–649 |
| Pass D D2's "seven" — **the table has eight rows** | 803–816 | run `dsh-pass-d-part-two-of-three-the-65b37a39`, D2 head `8fbe325d` | "reverted"; two replayed on the finished candidate (827–831) |
| Pass D D3's "six plus one discarded" — **the table has seven rows**: six, plus the fold that replaced the discarded attempt. The discarded `profile_patch` reading of `cordis.yml` is described at 994–998 and has no row | 992–1008 | run `dsh-pass-d-part-three-of-three-d-144c7c79`, D3 head `859e5b07` | 992–994; gates 1010–1018 |
| D3's returned review, two | 1058–1066 (guard extension), 1067–1074 (sorting mutation) | the D3 returned-review seat, answered at `1d1d9f17` | 1073–1074 |
| R1–R4 M1–M9 (ledger: 1665–1673) | 1700–1710 | run `…-e291e076`, `336598b9` before rustfmt and `aa1dbbb5` (1695–1698) | restored byte-identical by `cmp`; gates 1712–1720 |
| D1–D3 with E5 (ledger: 1920–1922) | 1954–1959 | run `…-124cca78`, adopted `d120dd91` | "1 passed each" (1947–1952) |
| D4–D5 with E6 (ledger: 2057–2058) | 2091–2095 | the same run, second sitting, adopted `3a1df8d3` | the same (2084–2089) |
| earlier M1–M12 (ledger: 12366–12383) | 12401–12420 | run `…-4437331e`, a scratch worktree of HEAD above `d54a9f7b` | 12392–12397; logs run-local under `.forge/implement-4437331e/`, not in this worktree and not opened |
| entries 1–10's own records | ledger §6, entries 1–10 (945–1545, returns and adoption notes included) | each entry's implement seat. Most name no run; entries 8, 9 and 10 name their commission or stop runs | each record's "run red and reverted" plus its gates line |
| entries 11, 12 and 13-fix's records (not in entry 17's list; B5 reaches them, observation 8) | ledger §6: entry 11 at 1596–1604 and 1618–1625, entry 12 at 1651–1662, 13-fix at 1718–1722 and 1742–1750 | entry 11 at `6e1e7066` and `471bc740`, entry 12 at `36b16922`, 13-fix at `1d2763cf` and `4ce6eba2` (answering run `…-bda73e1f`'s review) | each record's "run red and reverted" plus its gates line |
| `4a3854ca`'s three Codex removals: whole relocation disabled, the no-hands marker dropped, a hands member falsely marked no-hands | `git show 4a3854ca`, commit message only | commit of 2026-09-16; **names no run and no assertion**, only "each broke the named test" | "observed and restored" |
| task 10.5's M8, M8a, M9 and M9a (ledger: 10230–10233) | 10267–10270 (method 10248–10258) | the returned-implement correction on review of `8a5a1675`, landed in `9da5ff92` | "1 passed" per row |
| B16's four and B20's one | 9134–9138 and 9150–9151 (B16 also 5979–5985) | the 2026-09-15 implementation record on `21f4ac2`, names no run | "restored and rerun green"; B20 "Restored." |

## What was confirmed on the tree

**No mutation survives.** `git diff origin/main` names thirteen paths: eight
code and test files, `tasks.md`, and four documents in this change's
directory, one of them this brief. At `0af3a3c9`, before this brief, it named
twelve. Its production hunks are all recorded landings:

- `adapters.rs:1007–1014`: entry 9's strict UTF-8 decode, `6a5f1bc9`;
- `route_overlay.rs:111–116`: entry 4's pin filter, `289d9c5b`;
- `composite.rs`: `step`'s `operation` argument and `denied: operation !=
  "metadata"`, which is 13-fix, `1d2763cf`. The rest of that diff is comments
  (13-fix, and entry 13's `fcb91ad2`).

None restores a recorded mutation. A grep of `crates/` for the recorded
mutation spellings finds none: `&& false`, `|_| false`, `let key = key;`,
`false && `, `digests.iter().rev()`, and `from_utf8_lossy` in
`observed_version`. It also finds no `REMOVAL` or `MUTATION` marker. `git
status` is clean, and `git diff --check origin/main` is clean.

**Every targeted case exists and asserts what is claimed.** The cases were
opened for every ledger above, with the exceptions listed under
*Observations*. None of those exceptions is a missing test or a missing
assertion.

**Every targeted case passed in entry 14's gate run.** The host's `gates.log`
records head `be1ecf77624a16399ba133442db98a790f41d671`, `rustc 1.98.0` and
`openspec 1.12.0`, and exit 0 for each command in order:

1. `cargo fmt`;
2. workspace clippy;
3. the seven crate suites, each `--all-features --locked`: protocol 430 lib +
   99 (2 ignored) + 1 doc, at PL:437, PL:544 and PL:551; runtime 464 lib, at
   RT:475, and every integration binary;
4. `openspec validate --all --strict`;
5. both bundle compiles.

Five tests were ignored across the seven suites. Two are the native macOS
seatbelt probes in protocol (PL:442–443). Three are the measurement tests
`transcript::tests::measure_input_only`, `measure_parse_only` and
`measure_projection_peak` in `brokkr-view` (its log, lines 205, 206 and 208;
that suite reads `243 passed; 0 failed; 3 ignored` at line 253). No targeted
case is among the five. Each targeted case's `ok` line is cited in the
table and ledger section.

Since `be1ecf77`, only documentation has moved. `git diff --name-status
be1ecf77 HEAD` lists exactly three documents: the acceptance ledger and
`removal-controls-2026-09-23.md`, from entry 21(a)'s performance and close,
and this brief. At `0af3a3c9`, before this brief, it listed the first two.

Entry 14's own delivery record is entry 22's to write. This brief cites the
host log and does not record entry 14 as closed.

## Observations for the ruling

These do not decide the question. Each is something a "yes" would accept as it
stands.

1. **Records are on older bytes, and their line numbers have moved.** Almost
   every file:line a record quotes has shifted, because later entries added
   code above it. The assertions are still present at the lines cited in the
   table.
2. **HM M1 and M2 were recorded against absent-PATH behaviour that
   production has since replaced.** They are historical evidence, kept apart
   from what the tree asserts today.
   - *Then.* HM's scratch worktree was HEAD above `d54a9f7b`. There,
     `resolve_executable_in` refused an absent `PATH` unconditionally, as
     `'{command}': PATH is absent` (`git show
     d54a9f7b:crates/brokkr-protocol/src/adapters/composite.rs`, line 1508).
     M1 replaced that refusal with `path.unwrap_or_default()`. Its protocol
     failure was `left: "…'dsh' is not on PATH"` against `right: "…'dsh':
     PATH is absent"`, and its built-binary failure was
     `doctor_dsh_selection.rs:156`. M2 handed a failed selection on to the
     probe, and failed at `doctor/tests.rs:2687` and at the built binary's
     `:161`, "the refusal names the absent PATH" (12403–12406).
   - *The change.* `f030cce5` (2026-09-20, "resolve a program the
     platform's way") removed the unconditional refusal. With no `PATH`, a
     name was then searched on a default search path: glibc's and Apple's
     read through `confstr(_CS_PATH)`, and musl's own literal (`git show
     f030cce5:crates/brokkr-protocol/src/adapters/composite.rs`,
     1532–1559). That commit reports nine removals of its own "recorded in
     the tasks account". Entry 17 does not list them, and this brief did not
     open them.
   - *Current production* no longer asks `confstr` everywhere. It selects
     the search per target through `default_search_of`
     (`composite.rs:3221–3231`): musl's literal (3188), Apple's
     `_PATH_DEFPATH` literal `/usr/bin:/bin` (3205), FreeBSD's own literal
     (3213), and the `confstr(_CS_PATH)` query for Linux/glibc alone
     (3258–3280). Any other target refuses as unestablished (3261–3263).
     The Apple literal replaced the query in the #311 squash `e78c1da1`
     (its comment cites review 2026-09-20, F2). A refusal names the search
     it ran (`Search::capture`, `composite.rs:2994`, message at 3095). The
     code M1 mutated no longer exists in that form.
   - *Now.* The current assertions are these, each passing in entry 14's
     run.
     `C::an_absent_path_is_a_named_refusal_and_never_the_working_directory`
     C:3479 expects "`… is not on the default search path {default} (PATH
     is absent) (the search ended at …)`" (C:3505–3513), PL:67.
     `B::absent_path_refuses_before_doctor_can_execute_a_cwd_sentinel` B:234
     asserts "the refusal names the unsuccessful native default search, PATH
     absence as context" (B:267–271), CL:622.
     `D::a_failed_selection_probes_nothing_and_carries_its_cause` D:2765
     panics on any probe (D:2769), CL:40.
   A yes on B5 would accept M1 and M2 as records against `d54a9f7b`'s
   refusal. It would not make them removal evidence for today's
   default-search arm. M1 and M2 serve no row of this class directly. They
   are B5's reach, and after a no no entry replays them (observation 9).
3. **N2's M6 on current bytes.** Two assertions now run before the
   invocation assertion M6 names (C:3160–3166). They are C:3119, direct env's
   `DshInvocation`, and C:3155, "the invocation runs without a search", which
   entry 12 added. Under M6 today the first failure may be one of them.
4. **N6b's doctor half on current bytes.** RR added the probes assertion
   B:2603–2607 ("no DSH or Node probe for a lock that fails admission"). It
   now runs before the digest assertion D1–D5 recorded (B:2618–2621, "never
   reports the valid control's composite"). Under D1–D5 today, the doctor
   would part at B:2603 first. The ruling on F1–F4 accepted the first
   dependent assertion for those four 21(a) controls only. It does not extend
   here.
5. **Two counts in Pass D differ from their tables.** D2 says seven and
   tabulates eight. D3 says six plus one discarded and tabulates seven, the
   seventh being the replacement fold. The ledger repeats both counts.
6. **Gaps in the Codex records, kept as found.**
   - `4a3854ca`'s three name "the named test" but no assertion. The loops run
     the wrapped shape first (DC:3196–3198, DC:3299–3301), so the record
     establishes at most the wrapped shape's failure. Entry 21(a) performed
     the unwrapped controls.
   - B16's two bridge controls name no assertion.
   - B20's record does not say the paired test ran beside the mutation.
   - None of these records names a run.
7. **S6's "not time out" rests on one sentence.** The record says "No removal
   timed out except M10" (460). It gives no elapsed time for M1a, M1b or M2.
8. **New tests outside entry 17's list.** Entries 11 (`6e1e7066`,
   `471bc740`), 12 (`36b16922`) and 13-fix (`1d2763cf`, `4ce6eba2`) landed
   new tests, each with recorded compiling mutations (ledger §6, entries 11,
   12 and 13-fix). Every targeted case exists and passes: RT:460; C:3053 at
   PL:159 and D:2827 at CL:76; and C:2371 at PL:49, C:7240 at PL:91, C:8169
   at PL:83 and `native_matrix.rs` at PL:433.

   B5's "every new test" (`tasks.md` 5135–5136) reaches them, so B5 is not
   met unless their recorded controls are covered too. Entry 17's list names
   entries 1–10 only, and entry 21(b) replays "the compiling mutations
   entries 1–10 recorded". So:
   - a yes that names B5 meets it only if it also names entries 11, 12 and
     13-fix's records (ledger 1596–1604, 1618–1625, 1651–1662, 1718–1722
     and 1742–1750);
   - after a no on those records, **no entry owns their replay**. Entries
     18, 19 and 21(b) do not reach them, so B5, and with it entry 22's
     removal prerequisite, stays unmet until the operator assigns a replay.
     This brief discloses the gap and does not extend any entry to close
     it.

   Two smaller points:
   - 13-fix's third parted location (`:8234`) names no test. The case is most
     plausibly `each_pinned_errno_ends_in_the_same_refusal_on_every_librarys_arm`
     (C:8169), but that is inferred.
   - Its Apple rows remain pending the macOS leg.
9. **The R1–R4 group's records and B5: the ledger reads two ways.** RR's
   M1–M9, DR1/DR2's D1–D5 and HM's M1–M12 are compiling mutations against
   new tests of this change. Every targeted case exists and passed in entry
   14's run: the N rows' cases as cited in the table; HM M1 and M2's as in
   observation 2; HM M3 and M4's
   `B::an_obstructed_path_search_takes_the_explicit_safe_refusal` B:685,
   with its interpreter assertion at B:713 and B's sentinel assertion at
   B:737, CL:625, and the classifier C:5621, PL:166; HM M5 and M6's C:4310,
   PL:150; and HM M7's
   `C::pnpm_identity_strings_preserve_the_distinction_from_typed_scalars`
   C:5282, whose `refused_vector` is at C:5330, PL:96.

   The ledger does not say whether B5 reaches them:
   - its B5 scope, "every suite this change added tests to" (ledger 467),
     reaches them, and entry 17's list names all three ledgers (ledger
     1937–1939);
   - its B5 cell does not list them, and both that cell and entry 20 say
     "Unit 20 is 8.8's alone, so B5 does not wait on it" (ledger 467, 1998).

   Entry 20 replays only the mutations that serve the N rows: RR M1–M9,
   DR1/DR2 D1–D5 and HM M8–M12. HM M1–M7 serve the first hold's findings
   S1, 2 and 3, not a row of this class, and no entry lists them.

   So:
   - a yes that names B5 meets it only if it also names these three
     records, unless the operator rules that B5's "every new test" does not
     reach the R1–R4 group's tests;
   - after a no, if B5 reaches them, B5 waits on entry 20 and on a replay
     of HM M1–M7 that **no entry owns**. That contradicts the ledger's "B5
     does not wait on unit 20". If B5 does not reach them, entry 20 stays
     8.8's alone, as the ledger says.

   This brief discloses the conflict. It does not extend entry 20 or 21(b),
   and it does not assign the HM M1–M7 replay.

## Closing: what a "yes" must name, and what a "no" leaves

**For 8.10.** Entry 22's removal-ruling prerequisite is met by a single "yes"
that names exactly these **nineteen rows**:

> **B5, B16, B20, B77, S1, S3, S4, S5, S6, S7, S8a, S8b, S8c, S9a, S9b, S9c,
> S9d, S9e, S10**

The fifteen S rows are reached through B2. For B5, the yes must cover the
recorded controls in every suite. All eight groups are required, unless the
operator rules the eighth outside B5 (observation 9):

- the Pass C terminal body;
- the Codex bridge and conformance suites;
- task 10.5's four;
- `4a3854ca`'s three;
- Pass D's records;
- entries 1–10's records;
- entries 11, 12 and 13-fix's records (observation 8). B5's "every new
  test" reaches their tests, so a yes that leaves them out does not meet
  B5;
- the R1–R4 group's records: RR M1–M9, DR1/DR2 D1–D5 and HM M1–M12
  (observation 9). Naming N2–N10 for 8.8 names all but HM M1–M7; B5 needs
  all three records named in its own right.

That meets this one prerequisite. Entry 22 still owns the rest of its list and
the regrade and tick. The rest of the list is:

- entry 14 recorded green. The host's log above exists, and the record is
  entry 22's to write;
- entry 11, which has landed;
- entry 21(a), which has landed;
- every 8.10 row reading discharged.

**For 8.8.** The ten other rows also need naming, but 8.10 does not wait on
them: N2, N4, N5d, N6a, N6b, N7, N8, N9, N10 and A61. 8.8 also waits on
entries 13, 15, 16 and 23.

**For any row answered no**, the work falls to these entries:

| Entry | Covers | On the final candidate |
|---|---|---|
| **18** | S1, S3–S7, S8a–c, S9a–e, S10, A61 and B77, and B5's share for the Pass C tests | Replay Pass C's fourteen (465–480) one at a time, capturing each failing assertion verbatim. For S6, record that the failure is the terminal assertion and not a timeout, with elapsed time. Restore and rerun green, into a dated evidence file |
| **19** | B16's four (disabled status, boxed hands, harness fragment, boundary mark) and B20's selector with its paired `a_refused_resume_is_a_cold_spawn_with_the_refusal_journaled`, and B5's share for those suites | Same method as entry 18 |
| **21(b)** | B5's remainder | D1's two, D2's (tabulated eight), D3's (tabulated seven) and the returned review's two; entries 1–10's recorded mutations; `4a3854ca`'s three, each against a named assertion; and task 10.5's M8, M8a, M9 and M9a against A:2086–2089. Entries 11, 12 and 13-fix are not in its list (observation 8) |
| **none** | Entries 11, 12 and 13-fix's records, and HM M1–M7 if B5 reaches the R1–R4 group (observation 9): B5's last share | No entry owns this replay. If the operator does not name these records in a yes, B5 stays unmet until a replay is assigned. This brief does not assign one |
| **20** | N2 (M5, M6), N4 (M4, with entry 12's callback half), N5d (M1–M3, M7–M9), N6a (HM M8), N6b (D1–D5), N7 (HM M9), N8 (HM M10), N9 (HM M11) and N10 (HM M12) | The ledger says 8.10 does not wait on this entry (467, 1998). If B5 reaches the R1–R4 group's tests, these replays are also B5's share, and 8.10 would wait on it. The brief discloses this and does not rule it (observation 9) |

After a no, 8.10 waits on entries 18, 19 and 21(b), all of them. Entry 22's
list accepts those three as the alternative to a yes. But they do not reach
entries 11, 12 and 13-fix, or the R1–R4 group's records. So B5 would still
not read discharged, and entry 22's last condition would stay unmet, until
those records are ruled or replayed, or the operator rules the R1–R4 group
outside B5. The operator's
F1–F4 ruling covers exactly entry 21(a)'s four controls. In entries 18–21(b), a
control that does not part its named assertion is a finding. Observations 3
and 4 name two places where that may happen on current bytes.
