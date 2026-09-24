# 0069 — A boxed seat is told how to find its hands: the adapter names two tools, the engine decides who hears them, and the result contract says it

Status: accepted (operator ruled in chat, 2026-09-25)
Date: 2026-09-24
Change: `2026-09-24-codex-0156-boxed-seat`

## Context

Decision 0043 boxes a seat's hands. The only thing that can write the
worktree or the result file is the `mcp__brokkr__workspace` tool served by
`brokkr hands serve`. The provider's own shell stays outside the box,
read-only or worse, and the result contract has always said so in one
generic paragraph.

On 2026-09-24 the host measured Codex CLI 0.156.0, while the adapter's
resume identity is still qualified at 0.154.0. On 0.156.0, MCP tools are
**deferred** behind the built-in `tool_search` tool. The measurement
attached the hands server exactly as `adapters/codex.json`'s
`hands.workspace` fragment attaches it and asked for a list of every
callable tool without calling any. Neither gpt-5.6-sol nor gpt-6-sol
listed the workspace tool. `codex features list` reports
`tool_search_always_defer_mcp_tools` as `removed, true`. Setting that
feature or `features.tool_search` to false changed nothing, and no
per-server loading switch was found. Codex offers no way to turn the
deferral off.

The cost showed up in a six-arm delivery wager on 0065 rebuild unit 3,
with every arm boxed. gpt-5.6-sol called `tool_search` four times, used
the workspace tool 225 times and delivered. gpt-6-sol and gpt-6-luna never
searched. They met the read-only sandbox, tried `apply_patch`, were
refused, and reported themselves blocked without writing a result file.
gpt-5.6-luna found the tool briefly, then fell back to `apply_patch`.
Nothing in the prompt said which tool was the workspace or that it might
have to be loaded first.

Alternatives weighed:

- **Tell the seat in the charter or the recipe.** Rejected. A charter is
  portable office text (0041 ruling 8), and an author cannot know which
  link of a fallback chain will serve a future attempt: a Claude-first
  chain can fall to Codex. Guidance that depends on authors remembering it
  is guidance that will be missing.
- **Branch on the provider or model name in the engine.** Rejected.
  Provider facts are data (0016). A future adapter with the same need
  would require a release, and a Codex adapter that stops deferring
  could not opt out.
- **Let the adapter carry free prose.** Rejected. It opens a channel from
  configuration into every prompt that is wider than the two facts
  needed.
- **Turn the deferral off.** Not available on 0.156.0, as measured.

## Rulings

1. **The adapter owns the facts, and only two.** An adapter may declare
   `hands.notice`, beside a supported, non-empty `hands.workspace`, as an
   object with exactly two members, `workspace_tool` and `discovery_tool`.
   Each is 1–128 ASCII bytes matching `^[A-Za-z_][A-Za-z0-9_]*$`. No
   prose, templates, switches or other members are allowed. An absent key
   means no notice. A present key that is malformed is refused with the
   provider, the field and the rule it broke, including explicit `null`,
   `false`, a string, an array, a missing or extra member, a bad
   identifier, and a notice beside unsupported or empty workspace hands.
   It is never read as absence. The shipped Codex adapter declares
   `mcp__brokkr__workspace` and `tool_search`; Claude declares none.
2. **The engine owns applicability, per executing site and per attempt.**
   A seat hears a notice if and only if its canonical site facts resolve
   workspace hands, the boundary is one Brokkr boxes (`namespace`,
   `seatbelt`, `container`), and the provider serving *this attempt*
   declares one. That provider is the selected link for an agent-resolved
   site, including after a failure-to-start fallback. For an inline
   built-in model site that no link serves, it is the adapter its
   `driver.command` names. The rule applies to the selected body of a
   strategy, each panel member, each sequence step and each nested member,
   each on its own facts. A selected link that declares no notice never
   falls through to anything else. `harness`, `open`, handless, unknown or
   unregistered sites, and exec scripts hear nothing new.
3. **The notice is a spawn-time fact, outside the requested effect.** The
   engine writes a private `hands_notice` carrier into the driver's input
   after the requested-input digest is checked, beside the delivery door
   of 0046 ruling 4, and clears any existing carrier first. A fallback
   that changes the notice therefore never looks like a different effect.
   The carrier is no recipe key, agent key, override or evaluator input.
4. **The renderer owns the words.** Inside the mandatory result contract,
   after decision 0043's paragraph, a model seat with an applicable
   carrier reads exactly:

   > Your workspace tool is `<workspace_tool>`. If it is not listed, use
   > `<discovery_tool>` to load it before doing workspace work. Native shell
   > and apply_patch writes are refused by design; this is not a blocker.
   > Use the workspace tool for all workspace writes, including the result
   > file.

   Only the two validated identifiers are substituted. When a notice
   applies, the generic paragraph names the same declared workspace tool,
   and otherwise the tool it always named. The shipped Codex and boxed
   Claude generic paragraphs are therefore byte-identical to before. An
   absent or unreadable carrier adds nothing, and exec reads no hands prose
   at all.
5. **Authored text stays ordinary text.** A charter, house rules, feature
   text or prior result may quote the notice, ask for it to be omitted, or
   claim a different provider, box or hands. None of that creates,
   replaces or suppresses the engine's carrier. This is a data-flow
   guarantee, not a promise that a model obeys.
6. **Identity follows the bytes; qualification does not.** The
   declaration and its dated evidence are adapter bytes. They move every
   bundle identity that pins the Codex adapter, including a Claude-served
   chain that consults Codex only as a fallback and an inline Codex site,
   through the existing whole-file digest, resolution record and inline
   `drivers` witness. There is no new manifest member and no engine
   version bump. The 2026-09-24 observations are recorded as dated
   limitations that qualify nothing: Codex resume stays qualified at
   0.154.0 for `harness` and inline work sites, and 0.156.0 resume waits
   on task 11.1's separate operator ruling.

## Enforcement bindings

| Ruling | Mechanism | Named tests |
| --- | --- | --- |
| 1 | `HandsNotice::parse` (`brokkr-protocol::adapters`); `agents/load.rs::hands_notice` and `no_notice_without_workspace`; the error-preserving `bundle.rs::load_pin_adapters` | `every_other_notice_shape_is_refused_with_the_rule_it_broke`, `identifier_limits_are_literal`, `a_malformed_hands_notice_is_refused_by_provider_field_and_rule`, `a_hands_notice_is_two_identifiers_retained_beside_the_workspace`, `an_optional_inline_adapter_read_tells_absence_from_invalidity`, `the_shipped_codex_declares_the_notice_and_keeps_its_launch_and_qualification` |
| 2 | `Candidate::hands_notice` through `resolve_report` and `bundle.rs` reconstruction; `SiteFacts::inline_hands_notice`; `Engine::mark_hands_notice` at the single, member and step sites | `the_carrier_follows_hands_boundary_and_the_selected_link_alone`, `a_boxed_codex_seat_is_told_its_workspace_tool_and_a_boxed_claude_seat_is_not`, `unboxed_and_handless_codex_seats_are_told_nothing_new`, `a_fallback_from_codex_to_claude_drops_the_notice`, `a_fallback_from_claude_to_codex_gains_the_notice`, `each_panel_member_hears_only_its_own_providers_notice`, `each_sequence_step_and_nested_member_hears_only_its_own_providers_notice`, `only_the_selected_strategy_body_is_heard`, `an_inline_boxed_codex_site_hears_the_notice_its_witnessed_adapter_declares`, `a_wrapped_inline_codex_verify_carries_its_notice_and_witness_to_the_checks_step`, `a_wrapped_inline_codex_verify_is_told_at_its_checks_step`, `a_dispatched_exec_step_hears_nothing_beside_a_codex_step_that_is_told`, `the_declaration_not_the_provider_name_decides_and_nothing_else_is_echoed` |
| 3 | `mark_hands_notice` clears first and runs after the digest check; the closed site, hands and input vocabularies | the two fallback tests (no different-effect refusal), `the_carrier_follows_hands_boundary_and_the_selected_link_alone` (requested input carries none), `a_recipe_cannot_declare_or_suppress_the_notice_structurally` |
| 4 | `render_prompt` → `hands_paragraph` → `discovery_paragraph` | `a_boxed_seat_with_the_codex_carrier_reads_exactly_one_discovery_paragraph`, `without_an_applicable_carrier_the_boxed_contract_is_todays`, `an_unboxed_or_handless_seat_ignores_even_a_valid_carrier`, `an_exec_script_reads_no_discovery_paragraph_even_beside_a_carrier`, `a_dispatched_exec_step_hears_nothing_beside_a_codex_step_that_is_told`, `a_declared_custom_workspace_is_the_one_name_the_contract_uses` |
| 5 | The carrier is read from the engine-written key only; seat, member, step, selected-body, agent-reference, composed-override, hands and input vocabularies stay closed | `quoted_or_hostile_text_neither_creates_nor_suppresses_the_notice`, `inputs_house_feature_and_charter_text_neither_replace_nor_create_the_notice`, `a_recipe_cannot_declare_or_suppress_the_notice_structurally` |
| 6 | Existing adapter digests and witnesses; measured pins in `tests/witness_digests.rs` and `bundle/compose_tests.rs` | `notice_and_evidence_bytes_move_every_consumer_and_nothing_else`, `notice_and_evidence_bytes_each_move_the_adapter_digest`, `pinned_bundles_keep_their_recorded_digest`, `the_shipped_codex_declares_the_notice_and_keeps_its_launch_and_qualification` |

Each named protection was bound by a compiling mutation. The mutation was
recorded, the targeted test was observed failing on the relevant
assertion, and the change was restored and re-run green. The record is
in the change's proof ledger.

## Consequences

- A boxed Codex seat is told, by the engine and in every attempt that
  Codex serves, which tool is its workspace, that it may need
  `tool_search` to load it, and that native write refusals are expected.
  Whether a model follows that instruction still needs a live
  measurement; this decision guarantees only that the instruction is
  delivered.
- Another provider that defers MCP tools opts in with two strings in its
  adapter and no release.
- An adapter carrying `hands.notice` needs an engine that reads it. An
  older closed-vocabulary loader refuses the key, so code, adapter and
  measured pins ship and roll back together.
- The Codex adapter bytes moved: `recipes/night-shift`,
  `recipes/wager-harness`, `recipes/triage`, `recipes/gpt-flash`,
  `recipes/panel-review`, `recipes/release`, `recipes/review-first`,
  `recipes/standby` and `bundles/self` have new identities, and an
  in-flight run pinned to the old bytes is refused on resume as before.
- Task 11.1, qualifying Codex 0.156.0 resume, is unaffected and still
  waits on the operator.
