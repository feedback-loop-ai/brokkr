Status: proposed
Date: 2026-09-24
Change: `2026-09-24-codex-0156-boxed-seat`

## Context

Adopt the existing change at specification commit `4b446a8d`, following the
commissioned base `aa58d07a`. Motivation and acceptance requirements remain in
[proposal.md](proposal.md) and the
[boxed-hands-discovery delta](specs/boxed-hands-discovery/spec.md). Neither
requires replacement. This visit has no `returned_from` finding. It writes
only the design artifact declared by the rendered OpenSpec instructions;
tasks, the numbered semantic decision and implementation follow in their
own phases.

The host's 2026-09-24 Codex 0.156.0 observations are supplied measurements,
not probes repeated by this council. The current source already appends a
generic paragraph naming `mcp__brokkr__workspace`; what it lacks is deferred
discovery guidance. Boxed Claude already receives that generic paragraph.
Its continued presence is compatible with receiving no new notice.

The implementation has three relevant boundaries:

- `agents/load.rs::parse_adapter` accepts closed adapter data and hashes
  the complete file. `agents.rs::resolve_report` retains provider candidates
  and pins every consulted adapter, including fallback-only providers.
- `bundle.rs` reconstructs candidates after command expansion and maintains
  canonical `SiteFacts`. Inline built-in model drivers already consult and
  witness adapter data through `load_pin_adapters` / `collect_unpinned`.
  Boxed inline model sites are admitted; they are not no-hands sites.
- `engine.rs::execute` verifies the requested-input digest before selecting
  attempt-specific delivery facts. Singles, panel members and sequence
  steps assemble inputs separately. `brokkr-protocol::adapters::render_prompt`
  appends the mandatory result contract, including `hands_paragraph`, and
  omits model-facing hands prose for exec.

The design applies decisions 0004, 0005, 0009, 0016, 0041 ruling 8, 0042,
0043, 0046 and 0063. Relevant history inspected includes `39b6b47d` and
`9da5ff92` for the measured 0.154.0 qualification, and `314e8786` for adapter
and identity movement. Those records do not qualify 0.156.0 resume.

## Goals / Non-Goals

**Goals:** make discovery metadata a small, validated provider fact; preserve
it through compilation; apply it to the actual executing site's provider
on every attempt; and render a fixed, independently testable paragraph.
Cover both agent-resolved sites and already admitted inline built-in model
sites without introducing another authority for hands or provider selection.
Keep the declaration bound to the existing adapter-byte identity.

**Non-goals:** guaranteeing a model follows the instruction; eager tool
loading, live provider probes or a discovery state machine; changing launches,
permissions, fallback eligibility, sandbox admission or result delivery;
charter/recipe controls; extending resume qualification; new journal or
manifest schemas; an engine version bump; capability-grant work from 0065;
release preparation, publication or push. Production remains Rust under
`crates/`. Frozen contracts, policy, schemas, fixtures, reference and
extensions remain untouched. Supported hosts are Linux and macOS; named but
unbuilt boundaries remain refused at runtime.

## Decisions

### D1. Divide ownership between adapter, engine and renderer

The adapter owns the two provider-specific tool names. The engine owns
applicability and selection. The protocol renderer owns the explanatory
wording inside the mandatory result contract. The office charter owns none
of these facts. This combines both council positions and follows 0016's
adapter-data rule and 0041's portable-charter rule.

Reject a Codex-name or model-name branch: another adapter can declare the
same requirement, and a Codex adapter without the declaration must get no
implicit notice. Reject an arbitrary adapter prose string: it creates a
configuration-to-prompt channel wider than the two facts required. Reject a
charter instruction, recipe opt-in or suppression key: an author cannot know
which fallback provider serves a future attempt, and the guidance must not
depend on authors remembering it. No new authority layer is needed.

### D2. A closed optional declaration and one small type

Admit this sibling of `hands.workspace` in `adapters/codex.json`:

```json
"notice": {
  "workspace_tool": "mcp__brokkr__workspace",
  "discovery_tool": "tool_search"
}
```

Use one dedicated `HandsNotice` value for these strings, distinct from the
existing optional-capability `Notice`. Put the shared value and narrow
validation in the existing protocol adapters module, which runtime already
depends on; do not add a crate or dependency. Runtime's loader supplies the
provider and field context when reporting errors. The private driver-input
serialization carries exactly the two members; it is not a public recipe or
manifest field.

The object must contain exactly both string members. Each is 1–128 ASCII
bytes, starts with an ASCII letter or underscore, and continues with ASCII
letters, digits or underscores. Use a small byte check, not a regex package.
Require supported, nonempty `hands.workspace`. Absence yields `None`;
explicit null, false, strings, arrays, missing/extra members and invalid
identifiers refuse with an exact, attributable diagnostic. Keep legacy empty
workspace fragments valid when no notice is present. The loader must inspect
presence before decoding, so malformed presence cannot collapse to absence.

Reject `enabled`, version predicates, transport tags, templates, prose and
step arrays. Presence is enough; the instruction is conditional on the tool
not being listed. No syntax or validation is added to frozen schemas.

### D3. Preserve provider facts through compilation, including inline sites

Store `Option<HandsNotice>` on `Adapter` and on each resolved `Candidate`.
Read the adapter directly in `resolve_report`, alongside its resume
assessment. Carry the value through the production candidate reconstruction
in `bundle.rs` after `expand_command`; the temporary policy-only projection
can use `None` because it neither executes nor renders. Do not enlarge
`ChainEntry` or the human-facing resolution report merely to relay the value.
At runtime there is no reopening of adapter files.

For inline built-in model sites, reuse `built_in_model_driver` and the
existing compile-time adapter lookup. Collect the notice into an optional
`inline_hands_notice` member of canonical `SiteFacts`. Witness the consumed
adapter with the existing `pin_drivers`/`drivers` digest mechanism, including
when it has no resume qualification. Keep discovery separate from
`inline_resume`: reading a notice must not create a resume assessment or
qualified coordinate. Relocation of a site's whole `SiteFacts` must carry
this member too. Unknown/custom raw drivers get no guessed association.

Change the optional inline adapter read to an error-preserving result:
no inline built-in consumer or an absent optional adapter directory means
no association; a valid library missing this provider or notice means no
notice; a present adapter library that cannot be loaded is a compile refusal.
Use normal loader diagnostics, including the exact notice validation errors.
Do not use `.ok()` to erase errors or match diagnostic strings to classify
them. Keep the existing mandatory adapter-loading requirements for agents,
gates, secrets and dialects intact. This deliberately ends silent disregard
of malformed *present* adapter data in the optional inline path; otherwise a
malformed notice would violate the specification while appearing compatible.
Valid older adapters and absent optional directories remain compatible.

**Scenario: an admitted inline boxed Codex site has discovery guidance.**
Given a built-in inline Codex work site with canonical workspace hands, a
boxed boundary and a valid associated Codex declaration, compile and dispatch
it through the production paths. Its contract has the same exact paragraph
as an agent-resolved site; the adapter digest is witnessed. Moving that site
under the dialect verify wrapper preserves both facts at the new label.

**Scenario: optional is not malformed.** An inline work fixture that otherwise
needs no adapter library keeps compiling with the directory absent. A valid
associated adapter with no notice gives no discovery paragraph. The same
fixture with a present malformed notice fails compilation with its exact
provider/field diagnostic. A custom unassociated driver acquires no inferred
Codex notice. These distinguish absence from invalidity without introducing
runtime argv heuristics.

Reject candidate-only coverage: the compiler already admits the inline case,
and the specification says every executing model site. Reject a second
resolver, runtime command guessing or suppressing the inline load error:
they either duplicate authority or leave the universal promise unproved.

### D4. Derive applicability per executing site and selected attempt

Add one engine helper beside `mark_hands` and `mark_delivery`. It first
removes any existing private `hands_notice` carrier, then inserts a notice
only when canonical `bundle.sites[label].hands` is `HandsState::Hands`,
`Boundary::is_boxed()` is true, and the provider source for this invocation
contains the declaration. For an agent-backed invocation that source is
`Selection`'s current candidate. Only an inline invocation without a
candidate uses its own `SiteFacts::inline_hands_notice`. A selected candidate
whose notice is absent must never fall through to stale inline metadata.

Use the actual selected body's label for a single, each member's label in
`member_runs`, and each single step's label in sequence execution. Nested
panel steps use `member_runs`; dialect exec steps gain no model guidance.
Containers and unselected bodies own no executing provider and supply none.
Clear-on-entry covers reused input, no-hands, unknown/unregistered sites,
unboxed sites and notice-free provider transitions. The renderer separately
excludes exec. Do not introduce a parallel mutable map of hands facts.

Stamp the notice after the requested-input digest check, alongside the
existing per-attempt delivery marking, and after ordinary context assembly.
Do not stamp it into `seat_input`'s durable requested effect. Adapter bytes
and journaled selection already bind its derivation; a new effect field,
event or resume coordinate is unnecessary.

**Scenario: fallback changes the instruction, not the requested effect.**
A failure-to-start transition from Codex to Claude clears the notice; a
separate Claude-to-Codex transition adds it. Both retain the same requested
input digest and avoid the different-effect refusal. A started model that
ignores discovery retains existing retry/fallback rules; its failure is not
a new failure-to-start category.

**Scenario: site isolation survives topology.** A selected body, mixed panel,
sequence and nested panel each render the paragraph of their own selected
provider or none. A forged prior carrier, sibling or parent cannot supply it.
No-hands and unknown facts clear it. Applicability under namespace, seatbelt
and container is tested as data; current runtime refusals for unbuilt boxes
remain exact and are not described as executed boundary proof.

Reject placing this value in requested input, using chain index zero, or
inheriting a parent marker. Each conflicts with existing selection or digest
semantics visible in `execute` and the independently assembled member inputs.

### D5. Render one fixed paragraph and one consistent workspace name

For a model invocation with the engine-supplied notice and boxed-hands
markers, append exactly this paragraph after the existing hands paragraph,
inside the mandatory result contract:

> Your workspace tool is `mcp__brokkr__workspace`. If it is not listed, use `tool_search` to load it before doing workspace work. Native shell and apply_patch writes are refused by design; this is not a blocker. Use the workspace tool for all workspace writes, including the result file.

Substitute only the validated identifiers. The renderer consumes the private
carrier, not nested context, evidence or a serialized adapter. An absent or
invalid carrier supplies no discovery prose; malformed *adapter* data has
already refused at loading. Exec emits none even if a synthetic input carries
it. Preserve the result path, JSON object shape and harness last-message door.

The generic boxed paragraph takes its workspace name from the same validated
applicable notice when present, retaining `mcp__brokkr__workspace` when absent.
This is a narrow substitution, not a rewrite. Shipped Codex's generic paragraph
stays byte-identical; boxed Claude's entire baseline hands guidance stays
byte-identical. Harness/open and no-hands behavior stays unchanged.

**Scenario: a custom declaration cannot name two different workspaces.**
A temporary provider declaring `fixture_workspace` and `fixture_search`
produces a complete engine contract whose generic and discovery paragraphs
both name `fixture_workspace`. The legacy workspace literal is absent from
that contract. Removing the declaration restores the generic baseline and
removes the new paragraph. This implements the delta's provider-independent
scenario without contradictory instructions.

Adopt robustness's coherence correction and qualify simplicity's request to
preserve the generic literal everywhere: preserving shipped baseline bytes
is required; hard-coding an inconsistent name for a custom declaration is
not. Repeating the name in two consistent paragraphs is an acceptable prompt
cost. Reject claims that the text confines native reads or guarantees MCP
availability: 0043 limits Codex's confinement claim to writes, and the new
paragraph explains native write refusal rather than promising tool success.

### D6. Engine ownership is a structural guarantee

Do not add `hands.notice` or `hands_notice` to recipe site keys, agent
overrides or the closed evaluator-input vocabulary. Existing recipe ingestion
and `HandsSpec` parsing reject structural attempts to author them. Legal
charter, house, feature and prior-result text remains ordinary text, even
when it quotes the entire notice or says to omit it. The engine overwrites
or removes the carrier after assembling that context, and the renderer reads
only that carrier for discovery facts.

**Scenario: text cannot opt in or out.** Compile and dispatch actual recipes
with a silent charter, an omission request, and quotes of the complete notice
and its result-contract heading. Applicable Codex still receives the same
engine-generated contract; notice-free Claude and unboxed/no-hands cases
still receive none. Structural false/null/forged declarations are refused
with exact parser diagnostics. A closed input declaration cannot promote
`hands_notice` from seat results into driver authority.

Tests must compare the independent literal contract suffix or the entire
expected prompt, and inspect the production-assembled carrier. Do not infer
authority from tool-name occurrence counts across authored text or split at
the first author-reproducible heading. Put distinct sentinels into unrelated
valid adapter evidence and launch configuration and require their absence
from the complete prompt.

Reject a content censor or a promise of prompt-injection immunity. Recipes
can quote instructions, and reviewed adapter data can be wrong. The bounded
guarantee is that recipe data cannot become, replace or suppress the
engine's structured discovery facts. This adopts both positions' distinction
between data-flow authority and model obedience.

### D7. Record discovery evidence and preserve qualification

Append short dated entries to Codex's existing limitations list, each within
the existing 400-character bound. Label them operator-supplied host
hands-discovery measurements from 2026-09-24. Preserve old evidence verbatim.
Cover installed 0.156.0 and the adapter-equivalent attachment; the two
no-tool-call probes; the reported `removed, true` feature state; both
ineffective false toggles; no measured per-server loading switch; and the
reported wager outcomes. Attribute them, including four searches and 225
workspace calls by gpt-5.6-sol, and claim no outcome for unreported arms.
The existing specification records the full evidence obligation.

Keep `resume.work-site.identity` exactly
`{"version":"0.154.0","applies_to":"0.154.0"}`. Preserve status, classes,
boundaries, hands coordinate, qualified argv and eligibility behavior.
Explicitly state that these added limitations do not qualify 0.156.0 resume
and that task 11.1 still awaits its operator ruling. Do not insert discovery
observations under a resume evidence axis or relax its text validator.

Reject moving the version because a newer CLI is installed: version
observation is not restriction/root/accounting qualification. No live provider
call is necessary for this feature's deterministic proof.

### D8. Reuse byte identities and measure all affected pins

The declaration and limitations both change Codex adapter bytes. Keep the
existing whole-file hash, consulted-adapter resolution digest and inline
driver witnesses; add no hash exclusion or new manifest member. Compile all
existing witness and compose cases before/after, recording which pins move
and why in the implementation proof record. Include a Claude-served chain
that consults Codex only as fallback, and an inline Codex consumer. A typed
field retained only in memory is still bound by the adapter bytes it came
from; copying it cannot substitute for preserving the witness.

Update only measured changed expectations in `tests/witness_digests.rs` and
`bundle/compose_tests.rs`, plus any other demonstrated consumers discovered
by those suites. Do not guess replacement hashes or mass-update unaffected
pins. Tests must also show a notice/evidence byte change moves a consumer's
identity. Existing old-run manifest mismatch refusal remains in force.
There is no engine version bump and no historical release-reference rewrite.

### D9. Reconcile both positions explicitly and keep the phase boundary

Both `.forge/design/positions/robustness.md` and `simplicity.md` were read in
full. The disposition of their substantive claims is:

| Claim | Disposition and evidence |
| --- | --- |
| Two typed adapter facts, fixed engine wording, no charter controls | Adopt both, D1–D2; existing adapter and prompt ownership already separates these responsibilities. |
| Preserve metadata through candidate expansion and selected fallback | Adopt robustness's explicit reconstruction warning and simplicity's direct adapter lookup, D3–D4; `bundle.rs` copies candidates. |
| Apply after digest verification and clear stale carriers per site | Adopt both, D4; `execute`, `member_runs` and sequence inputs have separate assembly paths. |
| Cover admitted inline sites and preserve their adapter witness | Adopt both, D3; existing inline admission and `SiteFacts` support this without a second resolver. |
| Do not swallow malformed inline notice data | Adopt both, D3; replace optional-load error erasure with explicit absence versus refusal. |
| Preserve generic guidance literally versus custom-name coherence | Combine with a bounded correction, D5: shipped bytes stay fixed; a declared custom workspace replaces only that identifier. An unconditional legacy literal is rejected because it contradicts the custom-provider scenario. |
| Strong ownership/disclosure proof without claiming model immunity | Adopt both, D6 and D10; production input provenance and independent contract expectations are the oracle. |
| Exhaustive semantic protection versus avoiding a test cross-product | Combine, D10: exercise each distinct data-flow path and predicate with mutations, without multiplying every model, OS and topology. |
| Dated limitations, measured pins, no resume requalification | Adopt both, D7–D8; 0.154.0 history and whole-file hashes answer different identity questions. |
| No new services, registries, public fields or launch changes | Adopt both; the existing compiler, site table and renderer suffice. |
| Chief should also write a numbered decision/index and adapter guide now | Defer those artifacts to implementation as the adopted proposal states. The rendered dialect declares only `design.md` for this visit. D11 retains them as implementation obligations, not omitted work. |

No earlier artifact is at fault. Inline coverage instantiates its universal
site requirement; the custom-name choice keeps its existing generic guidance
coherent. The scenarios above record those answers under the dialect's
`## Decisions` place. No upstream return or specification rewrite is needed.

### D10. Deterministic proof follows the production data flow

Extend existing Rust loader, bundle, engine and protocol suites. Temporary
fixture directories sit outside frozen `fixtures/`. Use actual adapter
loading, recipe ingestion, compilation, selection and dispatch assembly,
with an existing controlled driver seam capturing input and invoking the
production renderer. Direct renderer tests additionally pin exact prose;
they cannot be the only evidence for ownership or propagation. No provider
process is required. All errors have exact reason-bearing assertions,
never `is_err()`.

| Proof boundary | Required assertion and compiling mutation |
| --- | --- |
| Declaration | Exact retained strings and errors for every invalid shape, type and grammar boundary; mutate validation to admit a malformed value. Cover absent, explicit null, empty workspace, 1/128/129-byte identifiers. |
| Compiler propagation | Loaded notice reaches final dispatch; mutate candidate reconstruction to discard it. Inline malformed data refuses; mutate optional loading to erase the error. |
| Positive wording | Independent literal contract comparison includes both names, conditional discovery, native-refusal/not-a-blocker explanation and result-writing direction; mutate each claimed element or remove the paragraph. |
| Applicability | Zero added engine paragraphs for boxed Claude, harness Codex, open Codex, no-hands, unknown/unregistered and exec; mutate each relevant guard. Preserve generic Claude and last-message instructions. |
| Actual selection and effect identity | Both failure-to-start directions change only the selected instruction; mutate lookup to first candidate or move attempt marking before the request digest check. Observe the pertinent assertion fail. |
| Site isolation | Selected body, mixed panel, single sequence step and nested panel use their own facts; mutate a label/selection to parent or sibling, or retain stale carrier data. |
| Inline sites and relocation | Inline boxed Codex gets its notice, adapter witness and relocated facts; mutate inline collection, relocation or witness retention. Absence/custom unknown cases remain compatible. |
| Data-driven names | Temporary independently named provider and Codex without notice produce their declared/absent guidance; mutate to hard-coded Codex policy. Entire custom contract uses one workspace name; mutate only the generic paragraph back to the legacy literal. |
| Recipe authority | Real rejected fields/inputs and legal quoted or hostile charter/context cannot author or suppress metadata; mutate construction to accept an injected value, honor an opt-out or require charter guidance. |
| Narrow disclosure | Unrelated adapter/configuration sentinels are absent from the whole prompt; mutate rendering to append such data. |
| Identity and qualification | Measured adapter-byte changes move only actual consumers; the historical identity/qualification fields and semantics remain equal. Preserve existing exact refusal tests and bind new preservation claims to a targeted compiling mutation. |

Record mutations under `.forge/tasks/` with mutation/diff, targeted command,
successful compilation, relevant test and failed assertion, restoration,
and passing rerun. A compile error, unrelated failure, changed expectation
or predicted failure is not evidence. If one mutation covers several cases,
run them so every claimed failure is observed and named; a test that aborts
on its first assertion does not prove its remaining rows. Restore all
mutants before final checks. This matrix specifies future proof, not results
already obtained by the design seat.

### D11. Implementation completion retains the house gates

Implementation adds the numbered semantic decision with `Status: proposed`,
alternatives and named enforcement bindings, its index entry, and a focused
addition to `docs/guides/provider-adapters.md`. Choose the next available
decision number when authoring; only the operator accepts it. The following
tasks artifact must order parser/type, compiler propagation, per-site
assembly, rendering, adapter/evidence, measured pins, documentation and proof
so each delivered step can be checked. This design creates no tasks artifact.

Before implementation reports success, restore mutations and run:

```sh
cargo test --workspace --all-features --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo run --locked -p brokkr-cli -- compile --bundle bundles/self
openspec validate 2026-09-24-codex-0156-boxed-seat --strict --no-interactive
bash scripts/coverage-exact.sh
```

Use the unchanged pinned toolchain and exact threshold. The box cannot
supply external namespace proof: host/CI coverage stays pending until its
actual result exists. A toolchain failure is likewise pending, not a clean
gate. Tests of unbuilt boundary values are data-only and cannot replace that
proof. No new Windows matrix, live provider test, workflow-runner invocation,
release operation or push is introduced.

## Risks / Trade-offs

- **A model may ignore correct guidance** → guarantee the delivered prompt;
  do not claim behavioral success without a separate live measurement.
- **Optional old adapters omit the fact** → absence stays compatible; ship
  the declaration for Codex and document how another adapter opts in.
- **A syntactically valid declared name may be wrong** → keep reviewed,
  byte-pinned adapter authority; network discovery is outside this slice.
- **Inline optional loading previously hid invalid libraries** → explicitly
  distinguish missing from present-invalid data and pin exact refusal tests;
  do not preserve silent malformed notices as compatibility.
- **Attempt/site metadata can leak or be dropped** → clear on every assembly,
  preserve candidate copies and whole-site relocation, and mutate those paths.
- **The two paragraphs repeat the workspace name** → accept the small prompt
  cost to preserve shipped generic guidance; use one name consistently.
- **A discovery limitation sits beside resume limitations** → date and label
  it as supplied hands evidence, preserve historical text and qualification,
  and assert the 0.154.0 boundary explicitly.
- **Exact coverage cannot be established in this seat** → carry the actual
  tooling/host limitation as pending; never reduce or substitute the gate.

## Migration Plan

1. Implement the optional loader/type and propagation before adding the
   shipped field. Old valid adapters continue loading without a notice.
   Add deterministic tests and the proposed decision with the implementation.
2. Add Codex's declaration and bounded dated limitations without changing
   launch fragments or qualification. No recipe, charter, journal or schema
   migration is needed. Distribute the adapter with the engine that can read
   it: an older closed-vocabulary loader will refuse the new `notice` key.
3. Compile the witness/compose set and record before/after identities, update
   only measured changed pins, complete restored mutations and required
   validation. Changed adapter bytes intentionally invalidate old run pins;
   preserve the existing mismatch handling instead of rebasing a journal.
4. Roll back code, adapter changes and their measured dependent pin updates
   together. A rolled-back run follows the same existing identity checks;
   rollback does not grant resume across different adapter bytes.

## Open Questions

None that change the approach, scope or task breakdown. Codex 0.156.0 resume
qualification remains a separate operator-controlled task 11.1, not an open
question this feature must answer. Future live discovery measurements may
assess model obedience without changing this prompt-delivery contract.

## Design validation — 2026-09-24

`openspec instructions design --change 2026-09-24-codex-0156-boxed-seat`
was read through workspace hands, followed by a fresh read of its proposal
dependency. No workflow runner was invoked. Strict OpenSpec validation
passed. `openspec status --change ... --json` reports proposal, specs and
design done, with tasks ready; overall implementation is not complete.

The following checks were attempted through the same hands on this visit:

| Check | Observed result |
| --- | --- |
| `cargo test --workspace --all-features --locked` | Exit 127: `cargo: command not found`. |
| `cargo fmt --all -- --check` | Exit 127: `cargo: command not found`. |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | Exit 127: `cargo: command not found`. |
| `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self` | Exit 127: `cargo: command not found`. |
| `bash scripts/coverage-exact.sh` | Exit 127 at line 33: `cargo: command not found`; no coverage measurement was produced. |

Cargo and rustup are absent from the box's PATH. These Rust checks remain
pending in a toolchain-equipped environment; exact coverage additionally
requires the house's host/CI boundary proof. No test, threshold or sandbox
was altered. This is a design draft, with its proof obligations specified;
no implementation, semantic mutation or new manifest digest is claimed.
Only this design artifact is included in the design commit. The required
run-local chief result is written separately through workspace hands.
