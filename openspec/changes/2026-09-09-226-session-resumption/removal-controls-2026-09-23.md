# Removal controls — 2026-09-23

The dated evidence file the acceptance ledger's entry 18 prescribes, and
which entries 19–21 share. This revision holds **entry 21(a)** alone: the
controls of THE PROOFS' tests for which B5's inventory (ledger §4, B5,
(i)–(vii)) found no record. Entry 17 has no ruling, so entries 18, 19, 20
and 21(b) are not performed here.

- Candidate: `be1ecf77624a16399ba133442db98a790f41d671` on `slice-dsh-8810`,
  clean before the first control and after the last.
- Host: Linux 6.17.0-41-generic x86_64, cargo 1.98.0 (797e8a9bc 2026-08-05).
- Run: `issue-226-acceptance-ledger-entr-4b36a6f8`, implement seat.
- Method: one control at a time. Establish the named case green, apply
  one compiling mutation (plus, where stated, a recorded narrowing or
  backstop removal), run the case alone with `--exact`, and capture the
  failure. Then restore by hand, confirm `git status --porcelain` is
  empty, and rerun green. Every run executed exactly one test. The
  counts are in the `test result` lines quoted below. Line numbers are
  `be1ecf77`'s. A narrowed file shifts the panic line, and each record
  maps it back.

Command forms, `<name>` being the test named under each control:

```text
cargo test --locked -p brokkr-cli --all-features --test driver_conformance <name> -- --exact --nocapture
cargo test --locked -p brokkr-runtime --all-features --lib bundle::tests::<name> -- --exact --nocapture
cargo test --locked -p brokkr-runtime --all-features --lib engine::tests::<name> -- --exact --nocapture
```

Baseline, before any mutation:
`cargo test --locked -p brokkr-cli --all-features --test driver_conformance the_compiled_`
gave `test result: ok. 2 passed; 0 failed; … 22 filtered out`.

## Summary

| # | Ledger bullet | Test | Mutation | Parted at | Verdict |
|---|---|---|---|---|---|
| 1 | no-hands marker, unwrapped | `driver_conformance.rs::the_compiled_live_inline_codex_shapes_rejoin_their_provider_confirmed_root` (`:3194`), unwrapped `Single` and `NoHandsMember` | `mark_hands`'s `NoHands` arm emptied | `:3223` (cold root recorded) | parted at the live decision's first assertion. **Flagged for the grader** (see 1) |
| 2 | hands→no-hands, unwrapped | `::the_compiled_hands_inline_codex_shapes_refuse_unavailable_confinement` (`:3297`), unwrapped `HandsMember` | `Hands` arm publishes `not applicable`/`none` | `:3352` (`resume_refusal`) | parted at the named assertion |
| 3 | second verify diagnostic | `bundle/tests.rs::a_dialect_wrapped_verify_select_reaches_the_single_or_panel_refusal` (`:220`) | `bundle.rs:1529` message replaced | `:241` | parted at the named assertion |
| 4 | empty-agent propagation | `::a_selected_agent_case_keeps_its_empty_reference_cause` (`:254`) | `parse_selected_body`'s `resolve_reference` cause replaced (`bundle.rs:3050`) | `:270` | parted at the named assertion |
| 5 | census, both invocations | `::a_literal_phase_that_aliases_a_selected_case_is_refused_globally` (`:1271`) | `owner_index`'s existing-label refusal disabled (`bundle.rs:1782`) | `:1291` via `error()` (`:7`) | refusal gone, alias compiles; rename-only construction valid under the mutation |
| 6 | authoring census | `::a_raw_phase_that_aliases_a_wrapped_panel_member_is_refused` (`:1349`) | first census's refusal dropped (`bundle.rs:1475`) | `:1371` via `error()` (`:7`) | refusal gone, alias compiles; no backstop masked it |
| 7 | injected-validator claim | `::a_literal_phase_that_aliases_the_injected_validator_is_refused` (`:1387`) | claim removed (`bundle.rs:1550–1555`) **and** masking final walk removed (`:1607`) | `:1408` via `error()` (`:7`) | the final walk masked the claim's removal on its own, as recorded; with both removed the alias compiles |
| 8 | prefix sweep | `::a_wrapped_verify_panel_leaves_an_unrelated_literal_phase_untouched` (`:1425`) | `relocate_verify_facts` sweeps `verify:*` | `:1448` | parted at the named assertion, not a census refusal or the `:1447` unwrap |
| 9 | insert-as-removed | `::a_wrapped_panel_drains_overlapping_member_addresses_without_overwrite` (`:1469`) | each destination inserted as its source is removed | `:1491` | parted at the named assertion |
| 10 | dispatch re-mark | `engine/tests.rs::a_selected_single_publishes_its_own_confinement_at_dispatch` (`:188`) | `self.mark_hands(&site_name, &mut input)` removed (`engine.rs:1111`) | `:266` | parted at the named assertion |

Controls 5, 6 and 7 fail inside the test's own `error()` helper
(`bundle/tests.rs:5–10`, `Ok(_) => panic!("expected compilation to fail")`),
which each test calls to demand the refusal before it reads the message.
That is the collision assertion's first half: under each mutation the
aliasing bundle **compiles**. No other refusal, no map panic and no
compile error stands in its place. The `message.contains(…)` line is not
reached because no message exists.

## 1. The no-hands marker, unwrapped shapes

**Narrowing** (`crates/brokkr-cli/tests/driver_conformance.rs`, restored).
The only change was the iteration list, and no assertion moved:

```diff
-    for (shape, wrapped) in [
-        (ProofShape::Single, true),
-        (ProofShape::Single, false),
-        (ProofShape::NoHandsMember, true),
-        (ProofShape::NoHandsMember, false),
-    ] {
+    for (shape, wrapped) in [(ProofShape::Single, false)] {
```

A second run used `[(ProofShape::NoHandsMember, false)]` in the same place.

**Mutation** (`crates/brokkr-runtime/src/engine.rs`, `mark_hands`):

```diff
-            Some(HandsState::NoHands) => {
-                input["boundary"] = json!("not applicable");
-                input["hands"] = json!("none");
-            }
+            Some(HandsState::NoHands) => {}
```

**Intended:** the unwrapped shape's live decision fails, "Enabled" in
`tasks.md` 9454–9459. **Observed**, exit non-zero, for the unwrapped single:

```text
thread 'the_compiled_live_inline_codex_shapes_rejoin_their_provider_confirmed_root' (3949380) panicked at crates/brokkr-cli/tests/driver_conformance.rs:3218:9:
assertion `left == right` failed: Single wrapped=false: the provider-confirmed root is recorded
  left: Null
 right: "0199aaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 23 filtered out; finished in 0.03s
```

and for the unwrapped no-hands member:

```text
thread 'the_compiled_live_inline_codex_shapes_rejoin_their_provider_confirmed_root' (3959442) panicked at crates/brokkr-cli/tests/driver_conformance.rs:3218:9:
assertion `left == right` failed: NoHandsMember wrapped=false: the provider-confirmed root is recorded
  left: Null
 right: "0199aaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 23 filtered out; finished in 0.04s
```

The narrowing removed five lines, so narrowed `:3218` is `be1ecf77`'s
`:3223–3226`, the cold row's
`cold_row["root_session"]["id"] == PROOF_OFFER` ("the provider-confirmed
root is recorded"). The cold-marker assertion `:3227–3230`
(`boundary == "not applicable"`) comes after it and was not what parted.
No fixture or compile failure preceded it.

**Why it parts there, and what the grader must rule.** The gate decides at
both invocations. On the cold one, with no offer, `codex_launch` calls
`qualify(&gate, …)` (`adapters.rs:2571–2573`). A `Disabled` gate returns
`observed: None` and runs no version probe (`adapters.rs:1058–1065`), and
a root is recorded only beside an observed version (`adapters.rs:809–818`).
With the marker dropped, the gate is `Disabled("restrictions-unavailable")`
(`adapters.rs:974–979`). So the decision's first observable effect is the
unrecorded cold root. `tasks.md` 9461 lists that effect first among the
live-row assertions: "assert the durable cold root equals the engine
offer". It follows that **no marker mutation can reach the retry's
`launch: resumed` assertion (`:3238–3243`)**. With no root recorded, the
engine makes no offer. This record therefore reads the cold-root
assertion as the live decision parting at its first observable point. If
the grader reads "its decision assertion" as the retry assertion alone,
this control is a finding, not a pass. The reason is that the test
cannot express that reading under any marker mutation, not that the
mutation fails to bind. The test was not changed to force it.

**Restoration.** The arm was restored by hand. The narrowed single shape
reran green (`1 passed; … 23 filtered out; finished in 0.09s`). Then the
iteration list was restored, `git status --porcelain` was empty, and the
whole named test reran green with all four shapes:
`test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 23 filtered out; finished in 0.35s`.

## 2. Hands falsely marked no-hands, unwrapped `HandsMember`

**Narrowing** (restored):

```diff
-    for (shape, wrapped) in [
-        (ProofShape::HandsMember, true),
-        (ProofShape::HandsMember, false),
-    ] {
+    for (shape, wrapped) in [(ProofShape::HandsMember, false)] {
```

**Mutation** (`engine.rs`, `mark_hands`'s `Hands` arm):

```diff
             Some(HandsState::Hands(_)) => {
-                input["boundary"] = json!(self.boundary.word());
-                input["hands"] = json!(if self.boundary.is_boxed() {
-                    "boxed"
-                } else {
-                    "none"
-                });
+                input["boundary"] = json!("not applicable");
+                input["hands"] = json!("none");
             }
```

**Intended:** `refused_row["data"]["resume_refusal"] == "restrictions-unavailable"`
(`:3352–3355`). **Observed**, exit non-zero. Narrowed `:3349` is
`be1ecf77`'s `:3352`, and the exchange dump is abridged at `…`:

```text
thread 'the_compiled_hands_inline_codex_shapes_refuse_unavailable_confinement' (3963762) panicked at crates/brokkr-cli/tests/driver_conformance.rs:3349:9:
assertion `left == right` failed: HandsMember wrapped=false: the gate's own token: [… Object {"attempt_id": String("a1"), "data": Object {"effort": String("not reported"), "harness": String("codex"), "launch": String("resumed"), "model": String("not reported"), "root_session": Object {"harness_version": String("0.154.0"), "id": String("0199aaaa-bbbb-cccc-dddd-eeeeeeeeeeee"), "kind": String("codex-thread"), "persistent": Bool(true)}, "sandbox": String("danger-full-access"), "step": String("harness-started")}, … "type": String("checkpoint")}, …]
  left: Null
 right: "restrictions-unavailable"
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 23 filtered out; finished in 0.04s
```

The falsely marked boxed member **rejoined** (`launch: resumed`). The
direct token assertion is what caught it, ahead of the supplemental
marker assertions (`:3364–3371`), which never ran.

**Restoration.** The arm was restored. The narrowed shape reran green
(`1 passed; … finished in 0.04s`). The list was restored, the tree was
clean, and the whole test reran green with both shapes:
`test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 23 filtered out; finished in 0.10s`.

## 3. `assemble`'s second verify diagnostic

```diff
                             _ => return Err(CompileError::Invalid(
-                                "dialect verify currently requires a single or panel verify seat"
+                                "mutated: second verify diagnostic"
                                     .into(),
```

**Intended:** `:241–244`, `message.contains("dialect verify currently requires a single or panel verify seat")`.
**Observed:**

```text
thread 'bundle::tests::a_dialect_wrapped_verify_select_reaches_the_single_or_panel_refusal' (3969945) panicked at crates/brokkr-runtime/src/bundle/tests.rs:241:5:
bundle: mutated: second verify diagnostic
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 463 filtered out; finished in 0.00s
```

The panic message is the refusal the mutated second site produced. The
first guard (`bundle.rs:1495–1498`) kept its text, so the case does
reach the outer-body match. **Restored**, clean, rerun:
`test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 463 filtered out; finished in 0.00s`.

## 4. The selected-agent empty-reference cause

The propagation from `parse_selected_body`'s agent arm was mutated. The
diagnostic's origin, `resolve_reference` (`bundle.rs:2069`), was left
alone because every agent site shares it:

```diff
             Site::Seat,
             boundary,
-        )?;
+        )
+        .map_err(|_| CompileError::Invalid(format!("seat '{what}' mutated: cause replaced")))?;
```

**Intended:** `:270–273`, `message.contains("seat 'work:engine' agent must be a non-empty string")`.
**Observed:**

```text
thread 'bundle::tests::a_selected_agent_case_keeps_its_empty_reference_cause' (3973475) panicked at crates/brokkr-runtime/src/bundle/tests.rs:270:5:
bundle: seat 'work:engine' mutated: cause replaced
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 463 filtered out; finished in 0.00s
```

**Restored**, clean, rerun: `test result: ok. 1 passed; … 463 filtered out; finished in 0.01s`.

## 5. `owner_index`'s existing-label refusal, both census invocations

One edit inside `owner_index` disables the refusal at both of its call
sites, the authoring census (`bundle.rs:1475`) and the final walk
(`refuse_global_aliasing`, `:1806`):

```diff
-                if let Some(first) = seen.get(&label) {
+                if let Some(first) = seen.get(&label).filter(|_| false) {
```

**Intended:** the collision assertion at `:1291–1295` (`error(…)`, then
`message.contains("addresses two different sites as 'work:chore'")`).
**Observed:**

```text
thread 'bundle::tests::a_literal_phase_that_aliases_a_selected_case_is_refused_globally' (3980253) panicked at crates/brokkr-runtime/src/bundle/tests.rs:7:18:
expected compilation to fail
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 463 filtered out; finished in 0.00s
```

The literal `work:chore` beside the selected case `work:chore` compiled.

**The rename-only construction stays valid under the mutation.** The
test's own control (`:1296–1307`) sits after the panic. So, with the
mutation still applied, a temporary test was added beside the original
and later deleted. It copied the construction verbatim: the same policy
and config, the `work:chore` → `work:other` rename, the rule-2 `from`
edit, and `assert!(fixture.compile(&control, &control_policy).is_ok())`.
The original test was not touched:

```text
#[test]
fn e21a_temporary_rename_only_isolation() {
    <lines 1272–1290 of be1ecf77, verbatim>
    <lines 1298–1307 of be1ecf77, verbatim>
}
```

It passed:
`test bundle::tests::e21a_temporary_rename_only_isolation ... ok`,
`test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 464 filtered out`.
So the mutation drops the collision refusal and nothing else the
construction relies on.

**Restored.** The temporary test was removed with
`git checkout -- crates/brokkr-runtime/src/bundle/tests.rs`, and the
refusal by hand. The tree was clean. Rerun:
`test result: ok. 1 passed; … 463 filtered out; finished in 0.00s`.

## 6. The authoring census before the wrapper renames an address

Only the first invocation's refusal was dropped. The final walk
(`:1607`/`:1806`) was left intact, to see whether it would mask the
removal:

```diff
         refuse_aliasing_sites(&seats)?;
-        let census = owner_index(&seats)?;
+        let census = owner_index(&seats).unwrap_or_default();
```

(A refused census yields an empty map here, so the wrapper's claims find
no occupant either. No claim bears on this fixture, whose destinations
`verify:checks:alpha`/`:beta` and `verify:dialect-verify` no other owner
takes.)

**Intended:** `:1371–1380`, the refusal naming
`addresses two different sites as 'verify:alpha'`. **Observed:**

```text
thread 'bundle::tests::a_raw_phase_that_aliases_a_wrapped_panel_member_is_refused' (3988452) panicked at crates/brokkr-runtime/src/bundle/tests.rs:7:18:
expected compilation to fail
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 463 filtered out; finished in 0.00s
```

No backstop masked it. Once wrapping renamed the member to
`verify:checks:alpha`, the final walk found no collision, which is the
disguise the test's doc names. **Restored**, clean, rerun:
`test result: ok. 1 passed; … 463 filtered out; finished in 0.00s`.

## 7. The claim on the injected validator's address

**Step 1, the claim alone removed** (`bundle.rs:1550–1555`):

```diff
-            claim_address(
-                &remaining,
-                "verify",
-                "verify:dialect-verify",
-                "the injected dialect validator",
-            )?;
             let dialect_site = "verify:dialect-verify";
```

The test **stayed green** (`1 passed; … 463 filtered out`). The removal
was masked.

**Step 2, the masking backstop found and disabled.** The final walk was
removed as well:

```diff
-        refuse_global_aliasing(&seats)?;
-
         let select_records: Map<String, Value> = seats
```

**Observed**, exit non-zero:

```text
warning: function `refuse_global_aliasing` is never used
thread 'bundle::tests::a_literal_phase_that_aliases_the_injected_validator_is_refused' (3993341) panicked at crates/brokkr-runtime/src/bundle/tests.rs:7:18:
expected compilation to fail
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 463 filtered out; finished in 0.00s
```

The intended assertion was `:1408–1417`, naming
`addresses two different sites as 'verify:dialect-verify'`.

**Step 3, the claim's own effect.** The claim was restored and the final
walk kept removed. The test passed (`1 passed; … 463 filtered out`), so
the claim refuses by itself and the final walk is only its backstop.

**Restored** both, clean, rerun:
`test result: ok. 1 passed; … 463 filtered out; finished in 0.00s`.

## 8. A prefix sweep in place of the exact relocation pairs

```diff
-    let staged: Vec<(String, SiteFacts)> = moved
-        .iter()
-        .filter_map(|(source, destination, _)| {
-            sites
-                .remove(source)
-                .map(|facts| (destination.clone(), facts))
+    let _ = moved;
+    let swept: Vec<String> = sites
+        .keys()
+        .filter(|label| {
+            (label.as_str() == "verify" || label.starts_with("verify:"))
+                && label.as_str() != "verify:dialect-verify"
+        })
+        .cloned()
+        .collect();
+    let staged: Vec<(String, SiteFacts)> = swept
+        .into_iter()
+        .filter_map(|source| {
+            let destination = match source.strip_prefix("verify:") {
+                Some(tag) => format!("verify:checks:{tag}"),
+                None => "verify:checks".to_string(),
+            };
+            sites.remove(&source).map(|facts| (destination, facts))
         })
         .collect();
```

The sweep leaves out `verify:dialect-verify`. The wrapper writes that
label just before relocating (`bundle.rs:1574`), and it is not an
authoring label. Every other label under the `verify:` prefix moves.

**Intended:** `:1448` or `:1455`. **Observed:**

```text
thread 'bundle::tests::a_wrapped_verify_panel_leaves_an_unrelated_literal_phase_untouched' (3999669) panicked at crates/brokkr-runtime/src/bundle/tests.rs:1448:5:
the unrelated literal phase keeps its own hands
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 463 filtered out; finished in 0.00s
```

The compile at `:1447` succeeded, so this is neither a census refusal nor
the `unwrap` panic. **Restored**, clean, rerun:
`test result: ok. 1 passed; … 463 filtered out; finished in 0.00s`.

## 9. Insert each destination as its source is removed

```diff
-    let staged: Vec<(String, SiteFacts)> = moved
-        .iter()
-        .filter_map(|(source, destination, _)| {
-            sites
-                .remove(source)
-                .map(|facts| (destination.clone(), facts))
-        })
-        .collect();
-    for (destination, facts) in staged {
-        sites.insert(destination, facts);
+    for (source, destination, _) in moved {
+        if let Some(facts) = sites.remove(source) {
+            sites.insert(destination.clone(), facts);
+        }
     }
```

**Intended:** `:1491` or `:1501`. **Observed:**

```text
thread 'bundle::tests::a_wrapped_panel_drains_overlapping_member_addresses_without_overwrite' (4002878) panicked at crates/brokkr-runtime/src/bundle/tests.rs:1491:9:
member `a` keeps its own network-enabled hands
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 463 filtered out; finished in 0.00s
```

**Restored**, clean, rerun:
`test result: ok. 1 passed; … 463 filtered out; finished in 0.00s`.

## 10. The dispatch-time re-mark

```diff
-        self.mark_hands(&site_name, &mut input);
         self.mark_delivery(&site_name, gate, selection.get(&None), &mut input);
```

**Intended:** `:266–269`, `start["input"]["boundary"] == "namespace"`.
**Observed** (the Start dump abridged at `…`):

```text
thread 'engine::tests::a_selected_single_publishes_its_own_confinement_at_dispatch' (4006487) panicked at crates/brokkr-runtime/src/engine/tests.rs:266:5:
assertion `left == right` failed: the selected single publishes its own boundary: {"attempt_id":"3353e00e-…","effect_id":"e20e06e3-…","input":{"allowed_results":["complete"],"boundary":null,…
  left: Null
 right: "namespace"
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 463 filtered out; finished in 0.05s
```

The phase label `work` owns no facts, so `seat_input`'s marking published
`boundary: null`. Only the removed re-mark of `work:engine` could publish
`namespace`. **Restored**, clean, rerun:
`test result: ok. 1 passed; … 463 filtered out; finished in 0.02s`.

## Gates after restoration

Run on `be1ecf77`'s bytes with the tree clean. The only difference was
this file, which was not yet written when the cargo gates ran:

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | pass |
| `cargo test -p brokkr-runtime --all-features --locked` | pass. lib 464, integration 94 over 22 binaries |
| `cargo test -p brokkr-cli --all-features --locked` | pass. lib 468, integration 318 over 29 binaries, `driver_conformance` 24 |
| `git diff --check` | pass |

Not run here, as the commission directs: the exact-coverage script, the
bundle compiles and entry 14's ordered list (entry 22 records those),
and every checkbox.
