# U1f: independent strict intent (tasks 7.1–7.2)

U1f was built in run `0065-slice-two-unit-u1f-see-the--83dfea52` on branch
`s2/U1f`, cut from main at `907eab8e`, where U1e had landed, and repaired in
run `0065-slice-two-unit-u1f-see-the--deb358ab` (the last section below).

## Current state

The final change touches all three of the row's production files:
`crates/brokkr-runtime/src/agents.rs`, `crates/brokkr-runtime/src/bundle.rs`
and the new `crates/brokkr-runtime/src/bundle/mcp.rs`. A candidate's intended
set is `Composition.mcp`, recorded in `agents::compose` from its typed hands
and the boundary, and `McpIntent::of_candidate` reads it back. Required hands
under a boxing boundary intend `Hands`. Under `harness` or `open`, or with no
hands, the set is `Empty`, never `Hands`. An inline site's set is
`SiteFacts::inline_mcp`, from its resolved hands and dispatched driver. A
dialect step records `NoModelSurface`. The first typed capability refusal
still propagates, and authored bytes equal to an engine fragment gain no
origin. The child suite `bundle/agent_tests/mcp_tests.rs` holds six new
tests and one moved test. `engine::notice_tests` holds the dispatch-access
test. The removal controls are M1–M9, R1–R4 and S1–S4 below. No contract,
policy, fixture, reference, extension, adapter, recipe or grant moved. The
MCP compile fence is untouched. The sections that follow are the record of
each visit in order. Where a later visit reversed an earlier claim, the
earlier section says so.

## First visit (historical): what moved and what is carried

The capability pass's per-serving composition, `site_capabilities` in
bundle.rs, moved into `bundle/mcp.rs` and was split along its one branch:
`candidate_capabilities` composes each candidate of an agent-backed site's
chain (now also choosing each candidate's `hands.harness.*` fragment for the
seat's class, which `record_capabilities` used to compute beside it), and
`inline_capabilities` composes an inline site's one serving from an
`InlineServing` parameter object. Both resolve through one `resolved` helper.
The nine-parameter function and its `too_many_arguments` expectation are
gone, and `record_capabilities` shrank from 173 to 136 lines.

`McpIntent` is the server set the engine intends one serving to launch with:
`Empty`, `Hands`, or `NoModelSurface` for exec. With no MCP holding
admissible before U9b, those are the only sets.

| Serving | Where the intent is read | Value |
| --- | --- | --- |
| A candidate (primary or fallback) | `McpIntent::of_candidate`, from `Composition.mcp`, recorded in `agents::compose` from the office's typed hands and the boundary (this row reflects the second return, below) | `Hands` where the office declares hands under a boxing boundary (`namespace`, `seatbelt`, `container`); `Empty` under `harness` or `open`, or with no hands; `None` for a refused or unmapped entry |
| An inline site dispatching claude, lanetally, codex or dsh, or an opaque command | `SiteFacts::inline_mcp`, recorded in `record_capabilities` from the hands the site resolved and the driver its command dispatches | `Hands` or `Empty` by its resolved hands |
| An inline exec site or a dialect step | the same field, by the exec driver kind | `NoModelSurface`, whatever its hands |
| An agent-backed site, a container | none (`inline_mcp` is `None`) | its candidates carry theirs |

The intent has a production consumer in this unit: a plan types a fragment
as the box's hands (`Provenance::hands`) only where the serving's intent holds
the hands server, or the serving is exec's, whose box is served by no MCP
surface. An empty or unrecorded intent types none of a fragment, whatever its
bytes. For every composition the compiler builds today the two agree, so no
shipped plan's count moves; the gate makes a fragment whose bytes equal the
engine's, beside an intent without hands, count for nothing.

Three readings are assumptions, stated for review. The second return
reversed the first two; they are kept here as the record of what was
reviewed.

1. **(Superseded by the second return.) A candidate's intent is read from
   its composition, not stored twice.**
   Each candidate already carries its own typed hands intent in its
   `Lowering` (`Composition.intent.hands`, or `Refused(intent)`), set where
   the composition is made and kept through `expand_lowering`. A second
   `mcp` field derived from it would restate one fact (decision 0071 ruling
   5) and would have grown four test files that sit at their baselines.
   So `agents.rs` is unchanged, and `bundle/mcp.rs` reads that intent.
2. **(Superseded by the second return.) Hands under the `harness` boundary
   still intend the hands set.** The
   protocol's `isolated` (U1c) compares the intent's server set with whether
   the sealed serving inputs carry a hands spec, and the engine seals the
   office's spec under every boundary. The intent follows the same typed
   fact, never whether a fragment was emitted.
3. **An opaque inline command intends a set by its hands.** No adapter
   answers for it and nothing is measured, so admission (U1g) decides what
   becomes of it. Only exec is `NoModelSurface`, because its kind is the one
   the adapter loader admits as having no model MCP surface.

## First visit (historical): tests

The first visit's five new tests are in the new child module
`crates/brokkr-runtime/src/bundle/agent_tests/mcp_tests.rs`, registered in
`agent_tests.rs`. The second return added a sixth and revised three of them.

| Test | What it pins exactly |
| --- | --- |
| `each_candidate_intends_its_own_set_from_its_typed_hands_never_its_emitted_fragment` | Primary Claude and fallback Codex, in order, each `Hands` with 3 and 8 emitted hands tokens under `namespace`. The hands-less office is `Empty` on both links. A boxed office under `harness` is `Hands` with 0 tokens emitted. |
| `a_fallback_types_its_own_hands_and_equal_bytes_never_supply_their_origin` | A fallback whose composition intends no hands, with a fragment byte-equal to its primary's, types `[8, 0]`. Inline, `Hands` and `NoModelSurface` type 8, and `Empty` and an unrecorded intent type 0. |
| `each_execution_site_records_its_own_intended_set_at_its_own_label` | Across a panel member beside an agent, a sequence step, and a select whose default and one case are inherited from a base layer while the leaf rewrites the other case, each label holds exactly its own intent. |
| `a_dialect_step_has_no_model_surface_and_a_relocated_verify_keeps_its_intent` | A dialect step and the dialect's verify step are `NoModelSurface`. The wrapped verify's agent, relocated to `verify:checks`, keeps its candidate's `Empty`, and the two opaque inline sites are `Empty`. |
| `authored_bytes_equal_to_an_engine_fragment_stay_authored_and_refused` | The empty set's strict flag and document after a Claude command, and the hands table after a Codex command, each refuse with slice one's exact authored-option text. |

Test moves: `an_inline_sites_composition_carries_each_serving_input_as_its_adapter_declares_it`
moved unchanged from `agent_tests.rs` to `agent_tests/mcp_tests.rs`, beside
the intent it neighbours, so that the over-baseline suite shrank (6,788 to
6,676 lines) rather than growing by the registration. `gate_tests.rs`'s
`hire_judge` and `wrapped_verify` became `pub(super)` so the new module
shares them. They gained no line.

## First visit (historical): removal controls

Each mutation compiled, was run against the module's tests, failed the
named assertion, and was restored. After all of them, the module's six tests
passed again.

| # | Mutation | Failing test: assertion |
| --- | --- | --- |
| M1 | `of_candidate` reads the intent from whether `hands_fragment` is empty | candidates: row `harnessed, harness` read `Some(Empty)` for `Some(Hands)`; fallback: `[8, 8]` for `[8, 0]` |
| M2 | `McpIntent::of` maps no hands to `Hands` | candidates: row `bare, namespace`; execution sites: `review:chore`; dialect: `design:draft`, `review`, `verify:checks`; fallback: `[8, 8]` |
| M3 | every candidate's count reads `chain[0]`'s intent | fallback: `[8, 8]` for `[8, 0]` |
| M4 | `typed_hands` counts the whole fragment for `Empty` and `None` | fallback: `[8, 8]` for `[8, 0]` |
| M5 | `inline_capabilities` counts the inline fragment whatever its intent | fallback: rows `Some(Empty)` and `None` read `[8]` for `[0]` |
| M6 | exec dispatch is treated as a model driver | execution sites: `review:default` `Hands` and `work:second` `Empty` for `NoModelSurface`; dialect: `design:validate`, `verify:dialect-verify` |
| M7 | the inline branch records no intent | execution sites: every inline label `None`; dialect: `design:draft`, `review`, `verify:dialect-verify` |
| M8 | the dialect-step branch records no intent | dialect: `design:validate` `None` |
| M9 | protocol `native_controls.rs` skips its authored capability-option refusal (`Ok(true) => continue`) | authored bytes: both rows compiled instead of refusing |

## First visit (historical): gates and measurements

| Check | Observed on the first visit's tree (`907eab8e` plus that change) |
| --- | --- |
| Formatting, `git diff --check`, `typos --hidden` | clean |
| Workspace clippy, all targets and features, `-D warnings` | clean |
| `cargo test -p brokkr-runtime` | lib 841, `capability_launch` 75, `it` 120, `operated_repo` 1, `queued_launch` 3; all passed |
| `cargo test -p brokkr-cli --no-fail-fast` | all nine binaries passed (lib 627, `it` 501) after the suppressions baseline was lowered |
| protocol, core, view, store and bridge tests | 19 binaries, all passed |
| `compile --bundle bundles/self` | compiled |
| `bash quality/ratchet.sh files` and `clones` | file size holds; duplication holds |
| `openspec validate --all --strict` | 20 passed, 0 failed |

`quality/suppressions.txt` lowered production `too_many_arguments` from 29 to
28: the brokkr-cli suppressions test first failed on exactly that count, and
passed once it was lowered. `quality/file-lines.txt` records bundle.rs at
7,578 (was 7,734), agent_tests.rs at 6,676 (was 6,788), and the two new files
at 283 and 429. `quality/too-many-lines.txt` was re-measured with measure.sh
step 5's forced clippy lint: `record_capabilities` is 136 lines (was 173),
and the shifted bundle.rs and agent_tests.rs rows carry their new lines. No
new function exceeds 100 lines. No witness or compose input moved:
`witness_digests::pinned_bundles_keep_their_recorded_digest` and
`budgets::every_seat_prompt_stays_within_its_committed_byte_budget` passed
unchanged. The public API is unchanged, since every new item is `pub(crate)`
or narrower.

## First return from review (historical): typed access for dispatch

The review returned one medium finding. U1g, which may change neither bundle
file nor `agents.rs`, could not name the intent this unit records, because
`bundle::mcp` is private and `of_candidate` was `pub(super)`. The return
answers it inside the row's two production files:

| Access | Change | Production consumer |
| --- | --- | --- |
| The type | `pub(crate) use mcp::McpIntent;` in bundle.rs, so `crate::bundle::McpIntent` names it and the `pub(crate)` field `SiteFacts::inline_mcp` is readable by type | bundle.rs itself: the field's type and both writes in `record_capabilities` go through the re-export |
| A candidate's set | `McpIntent::of_candidate` is `pub(crate)` | `candidate_capabilities`, as before |

Nothing new derives the intent. Dispatch reads what the capability pass
composed with. `record_capabilities` lost three lines (136 to 133) because
the shorter path let one statement fit on a line.

The binding test sits outside the bundle subtree, in the engine module that
U1g makes the consumer:
`engine::notice_tests::dispatch_reads_each_servings_intended_mcp_set_by_type`.
It compiles a panel of a boxed `codex-then-claude` office and a hands-less
`codex-bare` office, an inline Codex review seat with hands, and the
fixture's opaque triage seat. It then asserts the exact list of
(label, inline intent, per-candidate intents) over every site with
capabilities: `review` holds `Some(Hands)`, `triage` holds `Some(Empty)`,
`work:bare` holds `None` with `[Some(Empty)]`, and `work:boxed` holds `None`
with `[Some(Hands), Some(Hands)]`, so primary and fallback are each read.
notice_tests.rs grows from 1,962 to 1,997 lines, which stays under the
2,000-line ceiling.

Removal controls for the return. Each was run against that test and then
restored:

| # | Mutation | Result |
| --- | --- | --- |
| R1 | the re-export narrowed to a private `use mcp::McpIntent;` | does not compile: `E0603 enum import McpIntent is private` at notice_tests.rs |
| R2 | `of_candidate` back to `pub(super)` | does not compile: `E0624 associated function of_candidate is private` at notice_tests.rs |
| R3 | `of_candidate` answers `Hands` for every composed or refused candidate (compiles) | fails: `work:bare` read `[Some(Hands)]` for `[Some(Empty)]` |
| R4 | `McpIntent::inline` maps no hands to `Required` (compiles) | fails: `triage` read `Some(Hands)` for `Some(Empty)` |

R3 and R4 were re-run on the test's final form, after a clippy
`type_complexity` fix that only removed the test's explicit type annotation.

Gates on the return's tree: formatting check, workspace clippy with all
targets and features under `-D warnings`, `typos --hidden` and
`git diff --check` all passed clean. Every brokkr-runtime test target passed:
lib 842, `capability_launch` 75, `it` 120, `operated_repo` 1 and
`queued_launch` 3. Doc tests had none to run. brokkr-cli's `suppressions::`
and `ratchets::` passed (19 passed, 1 ignored). `bundles/self` compiled.
`openspec validate --all --strict` reported 20 passed. The files and clones
ratchets held. `quality/file-lines.txt` records bundle.rs at 7,576,
bundle/mcp.rs at 285 and engine/notice_tests.rs at 1,997.
`quality/too-many-lines.txt` was re-measured with measure.sh step 5's forced
lint: `record_capabilities` is 133 lines, and the shifted bundle.rs and
notice_tests.rs rows carry their new line numbers. No suppression moved.

The review's low finding, that `Hands` is intended under the `harness`
boundary where no hands server is emitted, followed assumption 2 above. The
second return below fixes it.

## Second return from review (historical): the boundary decides the set, and the refusal's type is pinned

The second review returned R1 (medium) and R2 (low). This visit answers both.

**R2, the boundary.** SI2 asks for exactly the authorized server set. Only a
boxing boundary serves an office's hands through the engine's hands server.
Under `harness` the harness's own sandbox carries them, and under `open`
nothing does, so no hands server is launched in either case. A candidate's
set is now a typed fact recorded where its boundary is read:
`Composition.mcp` (a `pub(crate)` field in `agents.rs`) is set in
`agents::compose` by `McpIntent::composed(intent.hands, boundary)`. That
function gives `Hands` only for required hands under a boundary that
`Boundary::is_boxed` boxes, the same fact `compose` uses to decide whether
it appends the workspace fragment, and gives `Empty` otherwise.
`of_candidate` reads that field. A refused entry composed nothing and
launches nothing, so it now reads `None`, as an unmapped one does, instead
of a set derived from its hands alone. `Intent.hands` is unchanged and
still seals the office's hands spec under every boundary. `expand_lowering`
keeps the field through its `..composition.clone()`. An inline site keeps
`McpIntent::inline`, because the hands law refuses an inline model harness's
hands unboxed (decision 0046 ruling 4), so its hands are always boxed.

Assumption 1 is reversed. The set is no longer one fact restated: it is the
hands intent and the boundary together, and only `compose` holds both. This
return's production change stays inside the row: `agents.rs` and
`bundle/mcp.rs`. `bundle.rs` kept the first visit's change and did not move
again in this return. `agents.rs` stays at its 1,671-line baseline,
because `compose` lost its one-use `boxed` binding and a three-line comment
that said nothing MCP-related is composed (no longer true) became one line.
`compose` stays at 122 counted lines.

Seven hand-built `Composition` fixtures gained the field. The over-ceiling
test files stayed at or under their baselines:

| File | Lines (was) | How |
| --- | --- | --- |
| `agents/tests.rs` | 5,838 (5,838) | Two doc comments were tightened. `type Lowered` moved to module scope and is now shared by the two unit 3 tests. Three function-local `use brokkr_core::realms::Boundary;` lines became one module import. |
| `engine/boundary_tests.rs` | 4,616 (4,616) | A two-line comment and a four-line test doc were each shortened by one line. |
| `engine/capability_tests.rs` | 2,226 (2,231) | A local `strings` closure that shadowed the module's identical `fn strings` was removed. |
| `engine/resume_tests.rs` | 4,081 (4,085) | The `words` closure became one line. |
| `engine/tests.rs` | 6,652 (6,651; baseline 6,681) | +1 line. |

**R1, the variant.** `authored_bytes_equal_to_an_engine_fragment_stay_authored_and_refused`
now matches `Err(error @ CompileError::Capability(_))` before it pins the
text. Any other outcome renders as `not a capability refusal: …`.

Tests changed or added, all in `bundle/agent_tests/mcp_tests.rs` unless
named:

| Test | What it pins exactly |
| --- | --- |
| `each_candidate_intends_its_own_set_from_its_typed_hands_never_its_emitted_fragment` | Each link's (provider, set, typed hands, emitted tokens). Under `namespace`, boxed is `(claude, Hands, Required, 3)` then `(codex, Hands, Required, 8)`, and bare is `Empty`/`None`/0 on both links. Under `harness`, the boxed office is `(codex, Empty, Required, 0)`: the hands stay required, but no server is intended. |
| `only_a_boxing_boundary_turns_required_hands_into_the_hands_server` (new) | For each of the five boundaries, a literal table gives (required, none): `open` and `harness` are `(Empty, Empty)`; `namespace`, `seatbelt` and `container` are `(Hands, Empty)`. |
| `a_fallback_types_its_own_hands_and_equal_bytes_never_supply_their_origin` | The fallback's recorded set (`mcp`, formerly `intent.hands`) is set to `Empty` with a fragment byte-equal to its primary's: `[8, 0]`. |
| `authored_bytes_equal_to_an_engine_fragment_stay_authored_and_refused` | Variant `CompileError::Capability`, then the exact text. |
| `agents::tests::unit3_primitives_cannot_bypass_the_delivery_handoff` | Its expected compositions now carry `mcp`: `Hands` on the three `namespace` rows and `Empty` on the row for unboxed hands under `harness`. |
| `agents::tests::unit3_lowering_retains_absence_empty_and_sandbox_intent` | Its hands-less compositions carry `Empty`. |

Removal controls for this return. Each mutation compiled and was run. The
named tests failed, and then the mutation was restored:

| # | Mutation | Failing test: observed |
| --- | --- | --- |
| S1 | `resolved` maps a refusal to `CompileError::Invalid` (same text) | authored bytes: both rows read `not a capability refusal: bundle: seat 'review' …` |
| S2 | `composed` ignores the boundary (`boundary.is_boxed() \|\| true`) | boundary table: rows `open` and `harness` read `(Hands, Empty)`; candidates: `harnessed, harness` read `Some(Hands)`; `unit3_primitives…`: the unboxed row read `mcp: Hands` |
| S3 | `compose` records `composed(HandsIntent::Required, boundary)` | `unit3_lowering…` (four rows), candidates `bare, namespace`, `a_dialect_step…`, `engine::notice_tests::dispatch_reads_each_servings_intended_mcp_set_by_type` |
| S4 | `of_candidate` derives the set from `intent.hands` as if boxed (the first return's behaviour) | candidates: `harnessed, harness` read `Some(Hands)`; fallback: `[8, 8]` for `[8, 0]` |

Gates on this tree:

| Check | Observed |
| --- | --- |
| `cargo fmt --all -- --check`; `typos --hidden`; `git diff --check` | clean |
| Workspace clippy, all targets and features, `-D warnings` | clean |
| brokkr-runtime | lib 843, `capability_launch` 75, `it` 120 (witness digests included), `operated_repo` 1, `queued_launch` 3: all passed |
| brokkr-cli `it` `suppressions::` and `ratchets::` | 19 passed, 1 ignored |
| `compile --bundle bundles/self` | compiled |
| `openspec validate --all --strict` | 20 passed, 0 failed |
| `quality/ratchet.sh files` and `clones` | **not run**: this launch's sandbox refused both `quality/ratchet.sh` and a direct `jscpd` call. File sizes were checked by hand with `wc -l` against `quality/file-lines.txt`, as tabled above. The clone scan is pending. |

`quality/file-lines.txt` now records bundle/mcp.rs at 290, mcp_tests.rs at
474, capability_tests.rs at 2,226, resume_tests.rs at 4,081 and
engine/tests.rs at 6,652. `quality/too-many-lines.txt` was re-measured with
measure.sh step 5's forced lint, scoped to brokkr-runtime. Only line numbers
moved, and no length rose: `compose` is 122 at 1102, the unit 3 tests are 121
and 131, and the capability_tests test that lost its closure is 111 (was
117). engine/tests.rs's rows were already stale before this change, and they
now carry the measured lines. No suppression moved.

Follow-up for U1g, not fixed here: the protocol's `isolated` (U1c) refuses
`NotSealed` unless the intent's server set equals whether the sealed inputs
carry a hands spec. The engine seals the spec under every boundary, so a
`harness`-boundary hands launch that U1g wires with this `Empty` set will
refuse. That is fail-closed. Reconciling the sealed fact with the boundary
(the protocol file) is outside this row.

Pending at that return: exact coverage (`scripts/coverage-exact.sh` on a
capable host), the clones ratchet, the release-binary size budget, remote CI,
and macOS. The repair below ran the clones ratchet.

## Repair visit: one fixture builder, reconciled records, the field doc

Run `0065-slice-two-unit-u1f-see-the--deb358ab` received the whole attempt
uncommitted on `907eab8e`. At the start of the visit,
`bash quality/ratchet.sh clones` refused with two new test clones. One was
6 lines, at `engine/capability_tests.rs:405` and `engine/resume_tests.rs:308`.
The other was 8 lines, at `engine/resume_tests.rs:312` and
`engine/tests.rs:37`. Both were hand-built `Composition` fixtures that had
gained the `mcp` field.

**R1, one builder.** `engine/tests.rs` now holds the suite's one fixture
composition, `composition(segments, intent, application)`. It reads the
template from the first segment, pins no effort and uses default serving
inputs. It derives the set as `McpIntent::composed(intent.hands,
Boundary::Namespace)`. Beside it, `unlimited(hands)` is an intent with no
local limits. Four fixtures now build through it, and each keeps the values
it spelled before:

| Fixture | Before | Through the builder |
| --- | --- | --- |
| `engine::tests::templated` | template segment only, the candidate's effort, `Empty`, no hands, unrestricted | `Composition { effort, ..composition(…, unlimited(None), Unrestricted) }` gives `Empty` |
| `engine::resume_tests::dsh_link` | template and authored segments, no effort, `Empty`, no hands, unrestricted | `composition(…, unlimited(None), Unrestricted)` gives `Empty` |
| `engine::capability_tests::composed_link` | `McpIntent::composed(intent.hands, Namespace)`, no effort | `composition(segments, intent, application)`, the same derivation |
| `engine::boundary_tests::candidate` | no hands gives `Empty`, a hands fragment gives `Hands`; effort `high` | `Composition { effort, ..composition(…, unlimited(hands), Unrestricted) }`; `composed` under `namespace` gives the same two values |

No clone baseline was edited. After the change, `bash quality/ratchet.sh
clones` printed `ratchet: duplication holds`. This visit added no test, so it
has no new removal control. The fixtures' values did not change, and the
suites that use them pass as before (gates below).

**R2, records.** The current-state summary above, and tasks 7.1 and 7.2, now
describe the final implementation: all three production files, the recorded
`Composition.mcp`, `Empty` under `harness`, six new tests and one moved in the
child suite, and the dispatch-access test. Earlier sections are headed as
historical. Each one keeps what that visit observed.

**R3, the field doc.** `SiteFacts::inline_mcp`'s doc no longer says it is
`None` at every site not visited as an inline one. It now says a dialect
step, which exec serves, records `NoModelSurface`, and it names where the
field is `None`: agent-backed sites, a panel, sequence or select, and an
unsupported check. The doc still takes five lines.

Measurements after the repair, by `wc -l` and measure.sh step 5's forced
`too_many_lines` lint, scoped to brokkr-runtime:

| File | Lines (attempt as received) | Main's baseline |
| --- | --- | --- |
| `engine/tests.rs` | 6,676 (6,652) | 6,681 |
| `engine/resume_tests.rs` | 4,073 (4,081) | 4,085 |
| `engine/capability_tests.rs` | 2,218 (2,226) | 2,231 |
| `engine/boundary_tests.rs` | 4,610 (4,616) | 4,616 |

`quality/file-lines.txt` records those four counts. In
`quality/too-many-lines.txt`, no function's length moved. Only line offsets
did: boundary_tests rows moved up by 6, capability_tests rows below the
fixture by 8, resume_tests rows by 8 and engine/tests.rs rows down by 24.
Every brokkr-runtime row matches the lint's output. No suppression moved.

Gates on the repaired tree:

| Check | Observed |
| --- | --- |
| `cargo +1.88 check --workspace --all-targets --all-features` | finished, no error |
| Workspace clippy, all targets and features, `-D warnings` | finished, no warning |
| `cargo test -p brokkr-runtime --tests --all-features --locked` | lib 843, `capability_launch` 75, `it` 120, `operated_repo` 1, `queued_launch` 3: all passed |
| brokkr-cli `it` `suppressions::`, `ratchets::` and `layering::` | 52 passed, 1 ignored |
| `quality/ratchet.sh` `api`, `files`, `clones` | public API holds; file size holds; duplication holds |
| `quality/ratchet.sh baselines origin/main` (`89deef2c`) | no baseline raised |
| `compile --bundle bundles/self` | compiled |

Pending: exact coverage (`scripts/coverage-exact.sh` outside the box), the
release-binary size budget, remote CI and macOS. U1g still owns two things:
production dispatch's consumption of `McpIntent`, and the fail-closed
`NotSealed` refusal for an `Empty` set beside sealed hands.

This re-vouch (run 0065-slice-two-unit-u1f-see-the--8be53fc7) carries the controller's coverage test `a_candidate_that_composed_nothing_intends_no_set` for the refused-or-unavailable arm of `McpIntent::of_candidate`, which CI's exact gate found unreached at 49584/49585; answering `Some(McpIntent::Empty)` there failed it at its `Refused` assertion (`left: Some(Empty)`, `right: None`), and restored, the brokkr-runtime lib ran 844 passed with the 1.88 check, clippy, and the api, files, clones and baselines ratchets against `89deef2c` all holding.
