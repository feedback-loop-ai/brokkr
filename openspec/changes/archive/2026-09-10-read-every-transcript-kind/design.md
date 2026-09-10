## Context

Adopt **read-every-transcript-kind** at entry checkpoint `d76a310`, including
preserved implementation checkpoint `48739c6`, commissioned checkpoint
`73797a6`, task repairs `c03aafc`/`c4ceb78`, the implementation repairs
through `bde819a`, and all preceding specification/design repairs, for issue
#222 on shipped main
`5bc8cf305aaef9af269866cbf83f094939691399`.
See [proposal.md](proposal.md) for motivation and the three capability deltas
for requirements. This sitting belongs to run
`current-successor-prior-run-curr-e4e66eb1`; clarification is `clear`.

An earlier successor commission supplied findings F1-F3. F1 was valid at
its earliest owner: R20 had assigned every `unreadable` outcome to a body route even though
shared discovery can establish `unreadable` before admission and DSH has no
browser body route. Commit `d9ac098` repairs proposal S15, the owning admission
requirement/scenario and R24; `3d7d4ae` carries the distinction through D2,
D9-D11 and proposed 0055. F2 and F3 were valid task-owned defects: `c03aafc`
scopes the mutation prohibition to the reader/system under test and real
operator evidence, and makes the Rust 1.88 gate compile the Boa-bearing test
target; `c4ceb78` keeps post-commit controller evidence outside the tracked
checkboxes. These repairs are landed and are not reopened here.

The inherited design's earlier `returned_from` was the second analyze visit and
reported two planning defects. First, D11 requires executable traces of the embedded browser controller but
selects no seam or runtime that `cargo test --workspace` can exercise. Second,
the task plan omits the specified operator reselection transition when the
operator selects the already active subject. Both findings are valid and are
answered here: D9/D11 select exact served-controller execution through a
pinned dev-only Rust ECMAScript engine, D10 carries that proof and dependency
into proposed 0055, and D9 makes every operator selection an unconditional
new generation and re-check interval. Their dependent task repairs are already
present in the adopted `tasks.md`.

The historical design at `6ece1ea` returned U1/U2 to specification. Those
findings are **answered**, not outstanding: reading R14/R15, command C6/C7
and TUI T5/T6 govern required unknown DSH events, packed rows and citations.
R16/C8/T7 additionally settle opening-header version admission. S10 preserves
citation-set membership despite the provider replay decoder's stricter
ordering. R17-R21 settle browser invalidation, recovery, recurrence, admission
and watch budgeting; R22 requires drill eligibility beside admission on every
browser action, and R23 treats a successful zero-turn body as received rather
than perpetually missing. R24 distinguishes discovery-stage from body-stage
`unreadable`; R25 replaces unsafe ordinary JSON quoting and semicolon framing
with one exact portable display literal and comma-separated descriptive fields.
This revision preserves all of those answers and all earlier reference,
compatibility, fallback and diagnostic answers. No requirement or scenario is
removed.

The current tree now contains the intended shared projector, handle-oriented
filesystem helper, transcript command, TUI refresh, prose-free browser
presentation and exact served-controller harness. The eight implementation
faults recorded by the previous design sitting were repaired through
`bde819a`; this sitting does not reopen them without new evidence. The current
robustness seat instead confirms one concrete R25 propagation gap: the shared
helper, hint framing, decision, checked tasks, implementation and surface tests
still encode ordinary JSON quoting and semicolon-separated Codex fields. D1,
D7 and D10-D11 replace that design rule without changing the architecture.
The decision, task, implementation and proof repairs remain dependency-ordered
downstream work; a checked box or an earlier green result cannot prove the
new rule.

The earlier analyze sequence 706 carried three valid artifact defects. F1's
earliest owner was this design: the premature archive
already folded the three `openspec/specs/transcript-*` capabilities, so those
living specifications are accumulated truth and the required base for every
active `MODIFIED` delta, not stale copies to delete. Their existing provenance
lines are append-only under decision 0042. Preserve the three specifications
through implementation and review, reconcile the final deltas into them with
one normal archive fold under the already-recorded destination
`2026-09-10-read-every-transcript-kind`, and neither rewrite nor duplicate the
existing pointer. F2 and F3 begin in `tasks.md`: the closing gates omit the
distinct locked release-profile build, and task 8.9 conflates four
contract-admitted CLI refusal documents with the future-kind journal fence and
the pure reader's defensive `unsupported-kind` arm. D1, D11 and the Migration
Plan record the design constraints; the next task office must repair those
task-owned lines. At entry the ledger has 77 checked and zero unchecked tasks. R25 invalidates
the evidence behind the affected implementation, surface, guide, gate and fold
items, so the next task office must reopen them before implementation resumes.

Decisions 0004/0005, 0009, 0013/0014, 0030, 0032, 0034 and 0042 bind this
work: explicit refusals, Rust production, shared derivation, read-only
surfaces, retained ownership, prose-free journals and separate phase
judgments. Proposed 0055 and its number-ordered registry row are filed at the
adopted checkpoint and remain unaccepted; the controller's 0054/0056 sibling
reservations stand and no sibling integration is assumed.

### Evidence and its limits

Both current positions were read completely:
`.forge/design/positions/robustness.md` (363 lines) and
`.forge/design/positions/simplicity.md` (234 lines), each against `d76a310`.
They are run-local evidence, not artifacts to commit. D1 reconciles every
material claim; the earlier sitting's rejected scope cuts remain recorded in
proposal S9 and git history.

The workspace tool exposes OpenSpec and Git but no Cargo, rustup, Codex, DSH or
Claude executable in this sitting. The simplicity seat reports current-head format,
clippy, both bundle compilations and all workspace tests except the one
namespace-dependent machine proof as passing from its execution boundary. Its
results, and the older 108/108 `brokkr-view` result including 23 transcript
tests, are useful council evidence but are not chief reruns or completion.
Host exact coverage is explicitly absent: the recorded controller run exits
one, and native Windows/macOS, Rust 1.88 Boa, license/audit, remote CI and
publication evidence remain pending. The robustness claims above were checked
directly against current source; they outweigh green suites that do not cover
the missing distinctions.
Provider help reported by councils remains council-reported evidence. Accepted
0030 independently records the Codex resume spelling. No hint is live
resumption, credential, session-ownership or sandbox-reimposition proof for
#226.

The robustness position's test-engine facts were independently checked against
Boa's tagged primary metadata. `boa_engine` 0.21.1 declares Rust 1.88.0 and
`Unlicense OR MIT`; its only default features are `float16` and `xsum`. The
design therefore pins `boa_engine = "=0.21.1"` with default features disabled
as a CLI dev-dependency. Its documented context can evaluate script source and
drain queued jobs. This is versioned source evidence, not proof that the
unmodified workspace currently resolves or passes that new dependency: Cargo
is absent here, so implementation must obtain lockfile, MSRV, license, audit
and all-platform evidence before treating the seam as viable.

The chief rechecked `.forge/controller-dsh-transcript-storage-interface.json`:
all 42 captured source texts match their SHA-256 values. It records installed
public packages at 2026-09-09T08:10:13.444450+00:00, not operator transcripts.
The session and persistence packages identify 0.1.2-rc.1. Principal bindings:

| Captured source, relative to its package | SHA-256 / fact used |
|---|---|
| session `lib/types/types.d.ts` | `8e33c2a629ed12b2456ca9c827e131a3a15f452d640696fa0df55a6a5f4a2f6e`; event envelopes, data nesting, depth and citation ownership. |
| session `lib/types/known-event-types.js` | `e7aee13d1dd119fa2c2ef6818eada27e547b2f20bccdbd4f1dadde0012a8c4f7`; closed recognized event vocabulary. |
| session `lib/types/chunk-rows.js` | `5724c4f798ed07e77406ab13a75685622a3e08868f257cbd142949099f0ea4c2`; packed members, sequence and time reconstruction. |
| session `lib/types/seq-ranges.js` | `68a127c76affa98edeeb50e302eb43f154f4d24f7d04cb95ba8b323e88f3d09e`; inclusive ranges; its replay ordering restriction is deliberately not adopted. |
| persistence-jsonl `lib/index.js` | `dfd6cde28928996f2f44220d013359563c8d9bf6c9c904972e77256767eec385`; opening version check and stored JSONL representation. |

Public primary sources were additionally inspected through the browsing tool
on 2026-09-09. The links below pin source tags; they do not attest to an
installed binary's exact build or to a live retained file:

- Codex `rust-v0.153.4`: [persistence policy](https://github.com/openai/codex/blob/rust-v0.153.4/codex-rs/rollout/src/policy.rs),
  [response models](https://github.com/openai/codex/blob/rust-v0.153.4/codex-rs/protocol/src/models.rs),
  [completed item types](https://github.com/openai/codex/blob/rust-v0.153.4/codex-rs/protocol/src/items.rs),
  [dynamic tool wire types](https://github.com/openai/codex/blob/rust-v0.153.4/codex-rs/protocol/src/dynamic_tools.rs)
  and [event protocol](https://github.com/openai/codex/blob/rust-v0.153.4/codex-rs/protocol/src/protocol.rs).
  They establish the payload mappings in D5, including persisted completed
  items and summary-only reasoning.
- Codex [event mapping](https://github.com/openai/codex/blob/rust-v0.153.4/codex-rs/core/src/event_mapping.rs),
  [legacy conversion](https://github.com/openai/codex/blob/rust-v0.153.4/codex-rs/protocol/src/legacy_events.rs),
  [producer sequencing](https://github.com/openai/codex/blob/rust-v0.153.4/codex-rs/core/src/stream_events_utils.rs)
  and [session persistence](https://github.com/openai/codex/blob/rust-v0.153.4/codex-rs/core/src/session/mod.rs)
  constrain association, including where identity is absent; D5 states the
  measured relationships and the limits of positional inference.
- DSH `dsh-v0.1.2-rc.1`: [message types](https://github.com/deepseek-ai/deepseek-harness/blob/dsh-v0.1.2-rc.1/packages/llm/llm/src/message.ts)
  and [blocks and streaming chunks](https://github.com/deepseek-ai/deepseek-harness/blob/dsh-v0.1.2-rc.1/packages/llm/llm/src/types.ts)
  complete the data nesting behind the controller's session types (D6).

S5's bounds are proposed reader policy, not a measured distribution of
provider homes, header lengths or performance. None of these observations
requires a paid model experiment, transcript copying or global settings.

## Goals / Non-Goals

**Goals:** give every commissioned local reader the same typed answer;
separate authority, safely opened bytes and interpretation; retain interrupted
content; make every failure and refresh transition testable; execute the exact
served browser controller under the workspace's ordinary Cargo tests; preserve
the existing Claude content and successful HTTP envelope.

**Non-Goals:** a provider plugin API, a new crate, replaying provider execution,
joining seats or attempts, a persistent body cache, a file watcher, Codex/DSH
browser body routes, media fetching, accounting changes, or #226's launch
and resumption work. The controller harness is not full DOM, rendering or
browser-SSE conformance and adds no Node/browser service or shipped JavaScript
runtime. Frozen contracts, policy, reference and fixtures stay unchanged.
No provider SDK or shipped JavaScript runtime is added; the only production
dependency change is the narrow target-specific filesystem binding already
selected in D3 and present in the lock graph.

## Decisions

### D1 — Reconcile the current council by claim

| Position / claim | Resolution and evidence |
|---|---|
| Robustness: staged authority, safe open, bounded snapshot, pure projection and typed result. | **Adopt.** Reference rejection, DSH's two format-refusal states and empty success cannot be represented by the old `Option<(Vec<Turn>, bool)>`. |
| Simplicity: keep the production surface to one pure view module and existing CLI orchestration. | **Adopt with the current factoring.** The new production files are `brokkr-view/src/transcript.rs` and CLI-private `ui/safe_fs.rs`; moving 486 lines of target-specific handle code into `ui.rs` would hide rather than remove a boundary. No third module, crate or public framework follows. Run/world resolution stays in `lib.rs`, rendering in `render.rs`. |
| Both: closed enum dispatch; reuse `Transcript`, `Turn` and `Block`. | **Combine.** Reuse the existing reference, move the content shape, and add a closed outcome. Private source positions do not become a second public event model. |
| Robustness: provider-specific validated states and private source identity. | **Adopt within ordinary structs/enums.** Pure constructors guard invariants; no service hierarchy, exception framework or plugin trait is needed. |
| Simplicity: reject replay graphs, body caches, watchers and generalized browser transport. | **Adopt.** None serves an additional settled scenario; each adds independent state or exposure. |
| Both: retain all repaired fallbacks, diagnostics, Claude restrictions and refresh behavior. | **Adopt.** The current simplicity position explicitly withdraws the earlier scope cuts. S9's refusals remain valid; this sitting does not attribute those obsolete cuts to the current position. |
| Robustness: checked directory/file handles across discovery and read. | **Adopt.** Shipped `is_file` plus path reopening can follow replaced ancestors. D3 chooses a bounded platform helper rather than merely promising later research. |
| Simplicity: keep that helper local, with narrowly featured OS dependencies if needed. | **Adopt.** D3 confines it to the private `ui/safe_fs.rs` child module; proposed 0055 states the narrow dependency exception. No reusable sandbox library or provider SDK follows. |
| Both: response preference plus content-bearing Codex completed events. | **Adopt historical D9.** Persistence policy stores completed items; calling them all lifecycle metadata would recreate unreadability. The existing R1 preference applies only to proved counterparts. |
| Both: unknown Codex association preserves content, not guessed suppression. | **Adopt with an explicit evidence limit.** D5 pins the measured id relationships and declines unproved legacy suppression. This does not claim universal duplicate-free legacy support. |
| Robustness: DSH ownership, then version admission, then event/storage classification. | **Adopt R14-R16 completely.** The current requirements already answer U1/U2; current `project_dsh` classifies later rows before checking version, so D4 makes the format-first transition and early-return allocation boundary explicit for downstream repair. |
| Robustness: lexical validity is not native-path admission, and source identity must be lossless on every target. | **Adopt.** Pure validation continues to recognize both supported path languages for portable documents; D3 adds a target-native absolute-path gate before filesystem access and a checked, lossless identity representation. Reject the current Unix reinterpretation of Windows syntax and the platform-dependent `u64` device assumption. |
| Robustness: a rollout-shaped directory must still be traversed. | **Adopt.** Opened child type precedes candidate-name classification: regular files are tested as candidates and directories continue within the depth bound regardless of their name. The current match-name-first `continue` is a downstream bug, not a reason to narrow discovery. |
| Robustness: public audit text must retain call identities and complete measured provider context. | **Adopt.** D5/D6 keep association metadata private but require deterministic displayed text to carry the recorded id plus server/tool/arguments/results needed to distinguish facts. The current dynamic-response, MCP-begin and completed-MCP omissions are implementation defects. Growing the public `Block` schema is rejected because the required identity can remain centralized text. |
| Robustness: DSH citation work needs a computational bound in addition to the byte cap. | **Adopt.** D6 selects an indexed interval lookup with near `O((events + ranges) log events)` work and an adversarial proof. Repeatedly scanning every same-turn/step chunk for every assembly is rejected as potentially quadratic. |
| Robustness: the command must use world-aware, read-only run selection. | **Adopt.** D7 requires the existing multi-hearth selection rule, including ambiguity and `latest`; the current `journal_of` single-store shortcut is a downstream defect. Opening even the sole hearth read-write merely because it was named is rejected for this read-only command. |
| Robustness: browser output must paint the authoritative common reference and every real selection path must trigger the explicit reset event. | **Adopt.** D9 keeps reference, hint and body distinct and binds both row and graph click handlers to `operator_select`. A test that calls the controller directly cannot excuse a shipped graph path that calls `sync`. |
| Simplicity: five grouped decision rulings instead of copying the scenario catalogue. | **Adopt.** D10 authors the proposed ruling text and enforcement bindings; the capability scenarios remain the detailed acceptance contract. |
| Robustness: separate selected-reference, presentation and body result types. | **Combine the invariant with simplicity's smaller public surface.** Keep one public `TranscriptRead`; use a CLI-private, prose-free browser presentation payload whose constructor accepts only selection, validation and discovery facts. This makes a body or read-level reason unavailable at the transport boundary without adding a second public view-crate result. |
| Robustness: model browser recovery as explicit states; simplicity: keep five private facts rather than a public state framework. | **Combine.** D9 defines the transition machine and its invariants, while implementation stores the key, generation, body-success/refusal state, exact watch handle and re-check budget as private client fields. No new crate, public taxonomy or generalized browser transport follows. |
| Robustness: isolate a dependency-injected controller and execute the exact served JavaScript with pinned, default-feature-free Boa. | **Adopt.** Promise ordering, generation capture and exact `EventSource` ownership live in the JavaScript applier, so executing only a policy description cannot prove the shipped controller. Boa 0.21.1 is Rust, matches the workspace MSRV and has a compatible license; D9/D11 confine it to a dev-only harness over exact `PAGE` bytes. |
| Earlier simplicity alternative: ship a Rust-owned JSON transition table, execute it with Rust and leave a generic JavaScript applier unexecuted. | **Reject with reason; the current simplicity position withdraws it.** It reduces the dependency graph but proves neither the applier's first-match behavior nor its async generation/handle capture. Executing a second Rust applier duplicates semantics; not executing the JavaScript applier leaves the returned finding open. The source-string-only fallback is rejected for the same reason. |
| Both: same-subject operator reselection is an event, not an equivalence no-op. | **Adopt.** Every explicit selection starts a fresh generation and interval even when the full subject key is equal; D9 fixes the reset order and D11 requires the adversarial stale-callback trace. |
| Both: admission and drill eligibility are independent, and every label/body/watch needs both. | **Adopt R22.** Admission remains the shared kind-agnostic discovery result; drill eligibility remains the Claude-kind/local-home client gate. Widening either fact into the other would make Codex, DSH or foreign-home sources call a Claude-only route. |
| Both: a successful zero-turn body is received. | **Adopt R23.** Body state is independent of `turns.length`; HTTP 200 with `turns: []` ends the deferred repair until an explicit clear or refusal. |
| Robustness: make R24's discovery/body `unreadable` distinction structural; simplicity: add no new public outcome, route, state or token. | **Combine.** Discovery can yield the existing `unreadable` token before admission; only an admitted arm can proceed to a body-stage failure. Private constructors enforce that split, while `TranscriptRead` remains the only public result and the browser presentation remains prose-free. D2, D7 and D9 state the legal construction and D11 proves both traces. |
| Robustness: R25 invalidates ordinary JSON quoting and semicolon framing; simplicity: preserve the current small architecture and treat remaining work as proof and ledger repair. | **Combine, with the specification as authority.** D7 replaces the shared helper with one pure `portable_display_literal`, exact comma framing and complete-line proof. CLI, TUI and browser continue to consume one shared value, so no renderer, route, crate or public model is added. Calling this proof-only is rejected because the current design and implementation construct a different observable string from the repaired requirement. |
| Both current positions: living capability specifications are the base for the active `MODIFIED` deltas, not disposable generated output. | **Adopt.** Proposal S16 and decision 0042 are controlling evidence: the previous fold accumulated these requirements into living truth, and every current delta modifies that truth. Pre-archive deletion, conversion back to `ADDED`, a fabricated archive directory and provenance exemptions are rejected. Preserve the specifications and their existing append-only pointers; the final archive operation reconciles the deltas once under `2026-09-10-read-every-transcript-kind`, making the existing pointers resolvable without rewriting or duplicating them. |
| Both current positions: production dependency inspection and MSRV all-target compilation do not prove the release artifact builds. | **Adopt as a distinct downstream gate.** D11 requires `cargo build --release --locked -p brokkr-cli` with the house's job bound in addition to, and not instead of, the Boa production-graph assertion and Rust 1.88 all-targets check. This is task-owned F2; no new dependency or architecture follows. |
| Both current positions: current CLI refusal documents and defensive unknown-kind handling are different reachability classes. | **Combine into three proofs.** Command tests cover the four contract-admitted rejected common references and preserve their documents; a journal-boundary test proves `future-session` is refused before selection with no transcript document or stdout; a pure-view test retains direct `unsupported-kind` handling. Widening frozen seat-record kinds for test convenience and deleting the defensive arm are both rejected. This is task-owned F3 and changes no requirement. |
| Both: scope the no-mutation convention to the system under test and real operator evidence while allowing synthetic fixture creation and mutation. | **Adopt; landed in `tasks.md` at `c03aafc`.** The reader/hints/renderers never create or mutate retained evidence; test setup may create and mutate only its own synthetic homes/files between reader invocations. This preserves the production read-only boundary and makes the required append, shrink, disappearance, replacement and same-length rewrite tests possible. |
| Robustness: use Rust 1.88 `cargo test --no-run`; simplicity: add `--all-targets` to the existing MSRV check. | **Combine on the smaller sufficient gate; landed in task 13.6 at `c03aafc`.** Run `cargo check --workspace --all-targets --all-features --locked` under the CI-installed Rust 1.88 toolchain, locally as `cargo +1.88.0 ...` when available. `--all-targets` compiles the CLI test target and its dev-only Boa dependency; `--all-features` matches the workspace admission surface. The existing remote MSRV job receives the same arguments, so no second job is needed. A production-only check is rejected because it never compiles the promised harness. |
| Simplicity: current-head local gates show the prior architecture is implemented; robustness: green local suites do not close R25 or absent host proof. | **Combine by evidence class.** The simplicity seat's format/clippy/test/bundle runs are retained as historical evidence from its boundary. Direct inspection of the shared helper, framing, decision and tasks keeps R25 open, while the controller's failed exact-coverage record keeps the host gate open. Neither council report is promoted into chief-executed proof for the repaired head. |
| Simplicity: trim unused projector-stage items to `pub(crate)`. | **Reject as unrelated cleanup for this change.** It is optional, does not repair R25 or a specified compatibility boundary, and would add production API churn while the exact semantic repair and proof remain open. A later evidence-backed API cleanup may narrow those items separately. |
| Robustness: defeat both JavaScript and HTTP stale caches; simplicity: keep this as a route/fetch detail. | **Adopt the behavior at the narrow boundary.** Presentation and body responses use `Cache-Control: no-store` (and fetches request equivalent freshness); no persistent cache or new contract field is introduced. |
| Simplicity: resolve factual drift without adding architecture; robustness: reopen dishonest completion state. | **Combine.** Refresh this design and preserve the small public shape. The next task office must reopen the R25-invalidated helper/framing, surface, guide, gate, archive and final-commit items; a checked box cannot substitute for missing evidence. The living-spec fold is preserved as accumulated truth and updated only by one final archive operation after review, never deleted or hand-maintained beside the active deltas. |
| Simplicity: keep `docs/guides/read-surfaces.md` rather than the intake's stale filename. | **Adopt.** That is the implemented and designed guide. Changing `journal-and-verification.md` would create unrelated overlap, not repair #222. |
| Both: preserve filed proposed 0055 and avoid changing its status. | **Adopt with an owning-artifact repair.** Checkpoint `48739c6` contains the decision and registry row, both still `proposed`. R25 supersedes one ruling but does not change that status. This sitting edits only the dialect-declared `design.md`; D10 supplies the exact replacement for the later decision-owning phase, and only the operator may accept it. |

Decision filing is no longer pending. Its R25 wording repair is pending in the
owning artifact. It remains an architectural prerequisite and an unaccepted
proposal, not proof of implementation or operator approval.
Decisions 0054/0056 remain the controller's.

A Node/headless-browser job is rejected because the normal Cargo and release
paths promise no Node or service and #222 does not own a new CI runtime. V8,
`deno_core` and QuickJS bindings add larger native/runtime surfaces without
more acceptance coverage than the isolated controller needs. Source-token
assertions alone are rejected because they cannot order promises or watch
callbacks. The chosen Boa seam is the smallest option presented that executes
the shipped controller itself under `cargo test --workspace`; its dependency
cost is explicit rather than hidden.

### D2 — One pure model, one existing I/O home

Add two production source files:
`crates/brokkr-view/src/transcript.rs` for the pure model/projectors and
`crates/brokkr-cli/src/ui/safe_fs.rs` as a private child of the existing UI
orchestrator for target-specific held-handle I/O. Reuse
`brokkr_view::Transcript { kind, locator, home }`. Move the serializable
`Turn { role, ts, blocks }` and `Block { kind, text }` shape into the view
module, with equality for cross-surface tests. Keep the five block kinds
closed internally while serializing the required strings.

Use `TranscriptKind`, `Unavailable` and a local `TranscriptRead` with
constructors for readable and refused results. Its selected reference and
legacy flag are distinct from lookup admission. A private `DiscoveryOutcome`
has only a refused arm carrying discovery-stage reasons and an admitted arm
carrying the safely opened source facts; body acquisition accepts only the
admitted arm. Thus the same public `unreadable` token cannot erase which stage
established it. `TranscriptRead` owns confirmed path, turns, source/display
truncation facts, counts, ordered notices, refusal and explanation, and
`full_session`. CLI serialization adds run id, exact seat key and requested
index without serializing private state. The thirteen reason tokens are
exactly the command delta's vocabulary.

Pure functions select/validate a supplied reference, admit supplied snapshot
facts, classify rows, associate content, cap turns and construct hints and
notices. They access neither environment nor filesystem. `lib.rs` supplies
read-only journal/world/participant facts; `ui.rs` orchestrates legacy-home,
discovery and byte-snapshot facts through private `ui/safe_fs.rs`.
`render.rs`, `tui.rs` and `ui.html` paint
those facts. Browser presentation is a CLI-private transport struct built by
the same pure selection, validation, discovery and hint helpers, but its
constructor is not given body bytes, turns, blocks or body-stage outcomes. It
can receive discovery-stage `unreadable` only from the discovery refusal arm
and serializes only the narrow fields in D9. This compile-time separation adopts
the robustness invariant without promoting a second result into
`brokkr-view`'s public model. `RunView` gains neither bodies nor local
presentation fields.

Private candidates need only physical row/member position, optional
association keys, block positions and the projected turn. Suppression may
remove an echoed block while retaining the rest of a composite event. An
empty block list removes that turn. Do not retain a second complete JSON
object graph for the whole file. Store a private source stamp and bounded
source/member identities beside the current TUI result, not in its JSON.

Alternatives rejected: a trait registry obscures the closed kind law; moving
I/O into `brokkr-view` breaks decision 0013; separate surface parsers repeat
the defect; a public replay/event ontology exceeds the `Turn` contract.

### D3 — Reference selection and discovery yield a safely opened source

Select the latest common reference before validating it, preserving its
three strings even on refusal. A present common reference defeats every
legacy fallback. Only the specified absent/Claude/LaneTally legacy
provenance can synthesize a valid Claude reference from supplied local HOME.
An eligible nonempty invalid legacy id is `invalid-reference` with no
synthesis. Exact participant key wins over label; an otherwise nonunique
exact label reports all matching keys. A parent never borrows a child.

Implement the reading delta's complete per-kind languages: Claude's leading
hexadecimal and 1-64 characters; Codex's leading ASCII alphanumeric and
1-128 characters; DSH's relative forward-slashed components and absolute
recorded home. Never trim, case-fold, reclamp or expand a locator. The
protocol's separate 80-character recording clamp stays unchanged.

Lexical validity is portable document validation, not permission for native
I/O. Before canonicalization or any filesystem call, require the recorded home
to use the current target's native absolute-path syntax: Unix accepts its
rooted syntax and Windows accepts its drive-absolute or UNC syntax. A
foreign-platform spelling is `invalid-reference` at the I/O admission boundary
and remains echoed byte-for-byte in JSON; it is never reinterpreted as a
host-relative path.

Canonicalize the recorded home once (the home itself may be a symlink),
then open that directory as the traversal root. All descendants are opened
one component at a time relative to held directory handles, without following
symlinks/reparse points. Enumerate through those handles, not a reconstructed
absolute pathname. Keep the candidate's verified handle for the body read;
never validate one file and reopen its name later.

Choose a small private `ui/safe_fs.rs` helper with
`cfg(unix)` / `cfg(windows)` implementations:

- Unix: use target-specific `rustix` 1.1.4 with `fs` (already in Cargo.lock),
  descriptor-relative directory iteration, `openat` with `NOFOLLOW`,
  `DIRECTORY` for ancestors and `NONBLOCK` for the leaf, then `fstat` regular
  file admission. Hold owned descriptors and compare device/inode identity
  through a representation that losslessly widens every supported target's
  signed or unsigned native values; checked conversion failure refuses the
  source rather than wrapping or narrowing it.
- Windows: use the locked `windows-sys` 0.61.2 bindings with only the needed
  filesystem/foundation/WDK features. Open the canonical root, then use
  handle-relative `NtCreateFile` with a single component, `FILE_OPEN` and
  reparse-point opening, rejecting reparse attributes; enumerate directories
  and obtain stable file identity through handles. A leaf must be a regular
  disk file before reading. Use owned handles and read/share flags that do
  not block the provider's normal appends. Microsoft documents the
  [existing-file and reparse options](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/nf-ntifs-ntcreatefile).

These target dependencies extend direct edges to existing locked packages;
implementation must not upgrade registry versions. They are restricted to
this local reader. A pathname-only Windows fallback is not admitted. The
platform tests must prove error mapping, handle lifetime and ancestor/leaf
replacement; unmeasured compilation or Windows execution is not claimed
here. An implementation unable to express this selected approach returns a
concrete design finding instead of silently weakening ownership.

Stable source identity is platform-neutral and lossless. Signed `i128`
fields are sufficient for the currently selected Unix device/inode and Windows
volume/file identifiers when populated by checked widening; a convenient
Linux-only `u64` representation is rejected. The identity belongs to the held
leaf and is rechecked without reopening its display path.

The scoped walks are closed matches, not recursive provider discovery:

| Kind | Eligible scope and identity |
|---|---|
| Claude | Exact `<id>.jsonl` in immediate project directories. |
| Codex | `rollout-*.jsonl` under `<home>/sessions`, containing the entire case-sensitive id with non-ASCII-alphanumeric/end token boundaries; files at directory depths 0-6. No content/header identity gate. |
| DSH | `<home>/<locator>/<project>/<session>/session.jsonl`; the first complete physical row is a `session` object with absent depth or unsigned-integer zero. No skipping to a later header, recursive/newest-file search or delegated substitute. |

Count every examined entry against 10,000, including irrelevant entries and
entries in every visited directory. A candidate is provisional until the
bounded scope is exhausted. At most 65,536 first-row bytes may be read from
each safely opened DSH candidate; EOF can complete a valid non-newline header.
No Claude/Codex content is read during discovery. For Codex, open and
classify each child before applying the filename predicate: a regular file can
be a candidate, while an opened directory remains traversable within depth six
even if its name itself resembles `rollout-*.jsonl`. Detect a longer DSH header
without allocating it. Retain at most the first qualifying handle and a
candidate count: other handles can close once inspected; two is already
ambiguous, but discovery must still account for failures/limits.

Collect discovery facts, then resolve the declared outcomes; do not let
iterator order choose the explanation. Limit exhaustion defeats a
provisional match, I/O preventing a unique answer is `unreadable`, multiple
safe qualifying files are `ambiguous-source`, and unsafe entries cannot
supply content. When no safe unique source exists, retain `unsafe-path`
evidence as specified. Invalid-depth-only DSH lookup uses its fixed
`not-found` explanation. Version never changes the root candidate count.
Test competing facts and both directory orders, not just isolated failures.

Before confirming the path, require a lossless Unicode spelling and verify
that the opened source still corresponds to the traversed candidate. Recheck
held ancestry/candidate identity at the read boundary; never follow a swapped
name. If a DSH opening ownership header changes during acquisition, do not
use the earlier header to establish a unique current source. Fail closed on
that acquisition rather than loop without a bound. A non-Unicode path is an
explicit output-identity `unreadable` failure before confirmation, not a
`to_string_lossy` path. No directory creation, copying or automatic repair.

### D4 — A bounded byte snapshot precedes every interpretation

Read at most 33,554,432 bytes plus one probe byte from the retained handle.
The probe is only an overflow fact. Pass the bounded bytes and EOF/probe
facts to the pure reader. A valid final JSON value at true EOF is complete
without newline; an invalid final non-newline append is provisional. Above
the source cap, only newline-terminated rows in the prefix participate.
Never parse a cap-cut row or use its members for association.

Validate consumed UTF-8 before semantic admission. A code point cut solely
by the source cap is a boundary fragment, not malformed data; invalid bytes
elsewhere are `unreadable`. Do not use replacement characters or expose a
partially scanned result. Source failure keeps only a confirmed path/hint
and independently established source truncation, with zero counts.

For DSH, validate opening-header ownership against the acquired snapshot,
then apply R16's numeric-zero version admission. The implementation transition
is `Snapshot -> FormatAdmitted -> Projected`: no later physical row is decoded,
counted or retained, and no provider-event/citation/association allocation is
created, until the opening row admits version zero. Missing/mistyped/foreign
versions return `unsupported-format` with zero counts and source-only
truncation. Neither subsequent JSON classification nor the display budget
runs. Depth deliberately uses the stricter unsigned-integer representation;
version accepts every numeric zero spelling, including negative zero.
Unused header metadata is not replayed or interpreted. Decode-then-reset is
rejected because foreign-version bytes must not exercise current-version
allocation or semantics.

For an admitted snapshot, classify all complete physical rows, even after
would-be display exhaustion or a DSH event/storage refusal. Malformed JSON
increments `skipped_lines`; unsupported valid rows increment
`unrecognized_records` at most once each. DSH event/storage refusal clears
all prose and preserves these complete-prefix counts and source-only
truncation. Header refusal and event refusal therefore share a token but
have distinct, intentionally constructed states.

Resolve all proved associations within the complete bounded prefix before
spending the 4,000,000-byte sum of final block texts. Retain whole turns
through equality; stop before the first overflow, never skip forward to a
smaller turn. A capped canonical event does not resurrect its fallback.
Apply `--turn` only after this result. Counters never depend on selection.

Process one physical row at a time, release its temporary JSON tree, and
retain only bounded candidate/association facts. For DSH, validate an entire
packed row before yielding members. Use checked safe-number arithmetic;
preserve negative-zero information where the sequence rules distinguish it.
Do not expand citation ranges or allocate by a sequence gap. Source order
is `(physical row, member)`, with block order preserved inside each turn.
Timestamps and hash-map order cannot reorder the transcript.

Alternatives rejected: whole-file `read_to_string`, selection before caps,
byte-to-text repair, fragment concatenation, and early display cutoff before
diagnostics or association. Each violates an explicit scenario.

### D5 — Codex mapping and association have separate evidence

Decode the retained `{timestamp, type, payload}` envelope; do not use the
adapter's separate `codex exec --json` stdout vocabulary. Keep the enclosing
recorded timestamp, or an empty string. The source tag is a support baseline,
not a required file header or a guessed installed-version field.

The response-model mapping is direct: `message.content` uses ordered
`input_text`/`output_text` text and inert image/audio omissions; `reasoning`
uses `summary[].text` only for `summary_text` members. Function/custom calls
retain `name`, `call_id` and raw `arguments`/`input`; their outputs retain
`call_id` and `output`, including structured text/media output arrays. Actual
JSON payloads are displayed as deterministic JSON, strings as recorded.
`local_shell_call.action`, `web_search_call.action` and
`tool_search_call.arguments` stay recorded structured tool data;
`tool_search_output.tools` stays recorded JSON output. Never reconstruct a
shell command. Raw/encrypted reasoning is omitted. Recognized response
context types `additional_tools`, `compaction`/`compaction_summary`,
`context_compaction` and `compaction_trigger` are quiet. These pointers follow
the tagged response models linked above.

Completed-event mapping follows the tagged item types, whose discriminants
are case-sensitive PascalCase, not the response types' snake_case:

| `event_msg.payload` family | Projection |
|---|---|
| `item_completed`, item `UserMessage` / `AgentMessage` | Ordered `item.content` text/media; agent `Text.text` and user input text retain their roles. |
| `item_completed`, item `Reasoning` | `item.summary_text[]` as reasoning; never `raw_content`. |
| `item_completed`, item `FunctionCallOutput` | Recorded name/id and output as a tool result. |
| `item_completed`, item `CommandExecution` | `id`, `command`; output uses recorded `stdout`/`stderr`, otherwise `aggregated_output`, otherwise `formatted_output`. |
| `item_completed`, item `DynamicToolCall` | `id`, `tool`, `arguments`; ordered `content_items` and `error`. |
| `item_completed`, item `McpToolCall` | `id`, `server`, `tool`, `arguments`; `result.content` and `error.message`. |
| `item_completed`, item `Plan` | Recorded `text` as assistant text. |
| Legacy `user_message` / `agent_message` / `agent_reasoning` | `message` / `message` / `text`; media fields yield inert omissions. |
| `exec_command_begin` / `exec_command_end` | `call_id`, `command`; end supplies `stdout`/`stderr`, `aggregated_output`, `formatted_output` in the same preference as completed commands. |
| `mcp_tool_call_begin` / `mcp_tool_call_end` | `call_id`, `invocation.server`/`tool`/`arguments`; end's `result.Ok.content` or `result.Err`. |
| `dynamic_tool_call_request` / `dynamic_tool_call_response` | Request `callId`, response `call_id`; recorded `tool`, `arguments`; response's `content_items` and `error`. |

For command output, use stdout/stderr when either contains text (including
whitespace); otherwise use a nonempty aggregate, then formatted output.
Omit the alternate representations, and keep a recorded empty result when
all are empty. Dynamic output types are `inputText` (`text`), `inputImage` and `inputAudio`;
MCP content uses its tagged text/media types. Project supported text and
inert media omissions, not encoded media bodies. Dynamic request fields
use camelCase while the response event uses snake_case; do not guess aliases. A begin
contributes its call and an end its result; metadata on the end does not
invent another call or require a begin to exist. Completed composite items
can contribute both blocks within their single logical-event turn. These
protocol-defined event fallbacks are independently readable when retained.

Decode only the declared field variants; an unknown completed item is a
counted omission, not generic lifecycle metadata. The tagged persistence
policy proves both response and completed-item storage and its legacy mode
split; it does not prove that every transient event is written by this
version. Supporting a retained execution event from its protocol definition
is not a claim of observing it in a live rollout.

Enumerate quiet top-level metadata as `session_meta`, `turn_context`,
`compacted`, `token_usage_record`, `world_state`, `security_risk_score`.
For event envelopes, the quiet list is `token_count`, `thread_goal_updated`,
`thread_rolled_back`, `turn_aborted`, `task_started`, `turn_started`,
`task_complete`, `turn_complete`, `thread_settings_applied`,
`session_configured`, `context_compacted`, `item_started`,
`agent_reasoning_raw_content`, `agent_reasoning_section_break` and
`reasoning_raw_content_delta`. A lifecycle name's nested last-message or
settings fields do not become prose. Unknown names are counted; no prefix
match extends this list. Recognized encrypted-only reasoning stays quiet.

Association is a distinct pass with these measured boundaries:

| Family | Positive evidence / consequence |
|---|---|
| Assistant response message and completed `AgentMessage` | `event_mapping::parse_agent_message` preserves a supplied response id; match the nonempty id and compatible content family. If absent, the provider generates another id, so no identity is inferred. |
| Response reasoning and completed `Reasoning` | `parse_turn_item` carries the response id into the completed item. Match a nonempty id, never an empty default. |
| Calls/results and their execution events | `legacy_events` carries completed command/tool ids into call ids. Match recorded identity plus direction and compatible family; a call and its output are two facts, not duplicates of each other. |
| Legacy agent/reasoning messages | Conversion emits one event per content/summary member without the item's id. The session persists `event.msg`, not the outer event correlation id. These sources do not establish a general lossless association key. |
| User prompt response and user event | The producer records the response then constructs a user item separately; client id alone is not proved equal to the response id. Do not equate them. |

The non-tool producer emits completion before recording its response;
the user path records response before completion. That is measured ordering,
not permission to match arbitrary adjacent records. Await points and separate
persistence operations do not establish an atomic, uniquely identified
pair in every retained history. No positional matcher is admitted without
an additional producer-backed proof of its exact applicable sequence.

Association identity may remain private, but the final deterministic
`Block.text` for every call/result must retain the nonempty recorded identifier
and its required context. In particular, MCP calls retain server, tool and
arguments; completed MCP items retain server as well as tool; dynamic responses
render recorded `content_items` and error rather than request arguments. Two
facts with different recorded ids must remain distinguishable on CLI, TUI and
browser body output. Centralize this formatting in the projector; do not grow
the public `Block` schema or let each renderer invent labels.

For a proved counterpart, prefer the response at its own source position;
remove only the blocks it actually covers. A composite completed tool event
whose output has no recognized response counterpart retains that output.
Require unique compatible identities; collisions keep content. Canonical
records never deduplicate each other. For absent/unproved identities, keep
both records, as R1 already requires, including repeated equal words.

This closes the design choice without fabricating S3's missing universal
legacy association proof. **Universal duplicate-free legacy support is not
claimed.** Additional controller-provided redacted pairs or producer evidence
may justify a narrower positional matcher; if the desired acceptance instead
requires deleting actual unassociable mirrors, that is an upstream R1 choice,
not an implementation heuristic. The reader still supports those records and
both surfaces now; lack of an association never hides an event-only ruling.
Tests must distinguish measured id pairs from deliberately unassociated
legacy data rather than add imaginary ids to provider structs.

### D6 — DSH decodes storage before projecting audit content

Apply the hash-verified captured version-zero envelope and the tagged message
and block definitions. Ordinary `user/message` stores a message directly in
`data`; `assistant/message` stores it in `data.message`. `tool/call` has
`data.name`, `callId`, `arguments`, `turn`, `step`. `tool/result` stores a
message in `data.message`, whose tool-result block carries `toolCallId` and
nested content. That provider message has user role for model input, but
Brokkr's standalone result turn has the specified `tool` role. Do not guess
an `output` property on the DSH event.

Project ordered `text.text`, `reasoning.text`, complete `tool-call` and
`tool-result` blocks, with inert `image` omissions. Supported nested result
text remains tool-result content, including recorded errors; private replay,
usage and tool metadata do not become prose. The recursive block traversal
must stay within the bounded parsed row; unknown nested blocks count the row
once while keeping supported siblings.

For `assistant/chunk`, only `text-delta` and `reasoning-delta` yield readable
fragments, with whitespace preserved and empty strings omitted. The exact
quiet chunk list is `tool-call-delta`, `block-start`, `block-end`, `usage`,
`finish`; argument deltas and block-end copies do not invent completed calls.
Unknown chunk types are counted nested-content omissions.

Decode `text-chunks`, `reasoning-chunks`, `tool-call-chunks` using R15's
exact-key tables, member arrays, safe numbers and checked reconstruction.
Validate all members before admitting any. Ordinary and packed logical
events enter the same projector. DSH timestamps use signed epoch-millisecond
decimal strings; invalid ordinary time is empty, packed invalid time refuses.
A physical packed row can yield multiple turn positions, but one diagnostic
row at most. Its incomplete tail yields no members.

Build a bounded sequence index over all observed logical events so duplicate
sequence identities remain ambiguous even when one is quiet. Match inclusive
citation ranges against those observed identities; never expand intervals.
Validate the entire `sourceEventSeqs` field on each surface event as R15
requires. Only readable assistant assemblies suppress uniquely identified,
earlier, cited chunks in the same recorded turn/step. Missing, empty,
partial, cross-step and ambiguous citations preserve uncited content. User
or result citations and `surfaceOp` never erase earlier audit history.
Dedicated call/result events suppress matching embedded blocks only with
proved call identity and turn/step, leaving unrelated message blocks intact.
Build interval-searchable citation indexes per `(turn, step)` and process each
observed chunk/range with near `O((events + citation ranges) log events)` work
and linear retained storage. Never rescan every same-turn/step chunk for every
assembly and never expand an integer range. A maximum-source adversarial matrix
with many assemblies, chunks and ranges is part of admission evidence; the
32 MiB byte cap alone is not a CPU bound.

The recognized quiet event vocabulary is the captured catalog minus the five
content kinds above, explicitly:

`agent-preset/selected`, `agent/inbox/spliced`, `approval/asked`,
`approval/decided`, `approval/policy`, `command/done`, `command/run`,
`compaction/end`, `compaction/prune`, `compaction/start`, `compaction/summary`,
`feedback/record`, `goal/change`, `hook/invoked`, `hook/result`, `llm/retry`,
`llm/retry-started`, `model/selection`, `permission/preset`, `plan/mode`,
`request/context`, `request/header`, `sandbox/mode`, `schedule/change`,
`session-log-deepseek/delivery-accepted`, `session/end-seed`, `session/title`,
`session/title-llm-request`, `step/end`, `step/start`, `subagent/descriptor`,
`subagent/model-selection-policy`, `team/member`, `team/message/delivered`,
`team/message/queued`, `team/task`, `todo/write`, `tool-workflow/agent-end`,
`tool-workflow/agent-start`, `tool-workflow/run-end`, `tool-workflow/run-start`,
`tool/code-dispatch`, `tool/code-dispatch-start`, `turn/end`, `turn/start`,
`web/deepseek-search-llm-request`.

These are recognized operational/context records outside the selected
conversation projection, not a wildcard that all future events are safe.
The admitted opening header is quiet; later `session` rows are unknown
events. An unknown event requires top-level boolean `ignorable: true` to be
an omitted success. Otherwise R14 refuses the entire projection. Invalid
packed rows/citations refuse regardless of that marker. Continue counting
the usable prefix after refusal; return no turns. This is U1/U2's implemented
design answer, not an imported DSH replay engine.

### D7 — One result determines output, hints and failures

Preserve Claude's exact R9 classification and shipped fixture `(1, 0)` counts,
string-content empty/whitespace behavior, and `Read · <file-path>`/tool-name
markers. Do not expand Claude thinking, argument or result prose. Both its
HTTP body and the new local readers call that same pure projector.

Use typed constructors to enforce the failure-stage matrix:

| Stage / result | Path / hint | Counts / truncation / turns |
|---|---|---|
| Rejected or absent reference | Null / null; common strings still echoed | Zero / false / none. |
| Discovery refusal | Null / valid Claude or unresolved Codex hint; DSH null | Zero / false / none. |
| Source I/O or UTF-8 failure after confirmation | Confirmed path / shared hint | Zero / established source overflow only / none. |
| DSH rejected opening version | Confirmed path / DSH hint | Zero / source overflow only / none. |
| DSH admitted-header event/storage refusal | Confirmed path / DSH hint | Whole usable-prefix counts / source overflow only / none. |
| Readable projection | Confirmed path / shared hint | Whole-prefix counts / either cap / retained turns, possibly zero. |
| `turn-not-retained` after readable projection | Preserve whole-read metadata | Preserve counts/caps/notices / none. |

Construct every hint once in `brokkr-view` after kind-specific identifier
validation. A single pure `portable_display_literal(value)` helper emits a
double-quoted valid JSON string literal. Inside its quotes it emits only ASCII
letters, digits, `/`, `.`, `_`, `-` and `:` directly. Every other Unicode
scalar is encoded with lowercase hexadecimal: one exact four-digit `\uXXXX`
escape for a basic-multilingual-plane scalar, or the two exact four-digit JSON
UTF-16 surrogate escapes for a non-BMP scalar. It never uses JSON short escapes,
deletes, normalizes or replaces a scalar; JSON decoding must recover the exact
Rust string.

Build the complete shared value with this fixed framing:

| Kind / resolution | Exact shared value |
|---|---|
| Claude, with or without a confirmed path | `full session: claude --resume <id>` |
| Codex, confirmed path | `full session: path <display-path>, codex exec resume <id>, home <display-home>` |
| Codex, no confirmed path | `full session: rollout unavailable, codex exec resume <id>, home <display-home>` |
| DSH, confirmed path | `full session: path <display-path>` |
| DSH without a path, or rejected/absent reference | null |

Commas are descriptive separators; fixed framing introduces no semicolon,
pipe, ampersand or redirection operator. Claude and Codex ids can remain raw
only because their closed validators have already accepted the respective
ASCII alphanumeric/dash languages and rejected a leading dash. DSH contributes
no raw locator. CLI text, CLI JSON, TUI and browser participant presentation
consume the completed shared string verbatim; they neither reconstruct nor
re-encode a fragment. Serializing `brokkr.transcript/v1` escapes the complete
string a second time as an ordinary JSON member.

This is portable display data, not POSIX, PowerShell or `cmd.exe` quoting and
not a pasteable command. Its narrower guarantee is that an agent-controlled
path or home cannot contribute a raw substitution, quote, command separator,
redirection or line boundary, so submitting the complete displayed hint to a
shell cannot evaluate or split a path/home-supplied fragment. Brokkr itself
never submits the value anywhere. POSIX single quotes are rejected because the
same field is shown on Windows; ordinary `serde_json::to_string` is rejected
because valid JSON may leave `$()`, backticks and printable operators raw;
terminal sanitization is rejected because those characters are not controls.
No ambient-home override or launch action is added.

Generate notices once, in this order when applicable:
`transcript truncated (size cap)`, `malformed transcript lines skipped: <n>`,
`unrecognized transcript records: <n>`. The old Claude suffix disappears.
A successful zero-turn read explains empty versus capped; a refusal never
becomes a readable empty result just because its counts are positive.

Add clap arguments in `cli_args.rs` and dispatch in `lib.rs`. Use the
existing adopted-world resolver rather than selecting one convenient journal:
explicit `--db` wins; otherwise consult every distinct existing hearth
read-only, refuse exact/prefix ambiguity across journals, and resolve `latest`
by the recorded ordering rule. The transcript command never uses a read-write
`Store::open`, even for a sole named hearth, because a read must not create WAL
sidecars, migrate or repair a journal. Parse positive u64 turn indices before
creating a document. JSON serializes the command delta's exact fields directly;
text renders through `render.rs`. Usage/run/seat errors leave stdout empty.
Post-selection unavailability exits one with safe stderr; only JSON mode also
emits the unavailable document. Whole readable empty/truncated reads exit zero.

Apply terminal `Safe` at every display edge, including references, paths,
roles, timestamps, hints and explanations. JSON retains escaped originals;
do not sanitize the model and destroy the machine document. No body is
attached to inspect/seats/watch/export/dossier or result telemetry.

### D8 — Refresh replaces the entire selected snapshot

Replace the Claude-only ask with a selected subject: realm/journal identity,
full run, participant key and complete effective reference. Only this
participant's transcript is read. At every existing refresh opportunity
while working, re-resolve safely and rederive even with unchanged journal
head or file length. Make the final read when it concludes, then stop
automatic polling; manual refresh still resolves it again.

An in-memory source stamp includes stable opened-file identity and bounded
source/member identities. To preserve selection, require the same authority
and source and an unchanged prefix of both projected turns and their source
members. Comparing rendered words alone, mtime alone or length alone is
insufficient. Retain bounded bytes/spans for this comparison if needed;
there is no persistent cache or fleet-wide body cache.

Publish each refreshed result atomically. On removal, replacement, reorder,
shrink, changed authority, ambiguity, ownership loss or format refusal,
clear the cursor and close the transcript overlay before showing new
indices. Notice-only changes can retain the cursor. A pure append with
uncited chunks unchanged retains navigation. Same-size version replacement
must refuse with zero counts and close both doors; later admission starts
fresh without restoring the old cursor. A second foreign-version root
instead removes the confirmed path through ambiguity.

Keep all existing keys and scrolling. Pane previews can clip; neither door
may drop a retained block. Number turns from one. Both doors carry shared
notices, and the whole door carries the full hint. A readable zero-turn
result opens its explanation; an unavailable result has no active door.

### D9 — Browser presentation is local metadata, with Claude compatibility

Add one GET participant-presentation endpoint in existing `ui.rs`, keyed by
full run id and an encoded participant-key component. `ui.html` uses
`encodeURIComponent`; the server decodes each component exactly once,
rejects malformed or extra components, and resolves the exact participant in
its read-only journal. It accepts no provider path or home override. Keep the
existing loopback, Host and method guard.

Use a CLI-private presentation payload with selected reference, legacy,
admission, lookup unavailability reason/explanation, shared hint, and eligible
Claude drill id/home-equality facts. Construct it from reference
selection/validation and bounded safe discovery only; it receives no selected
body snapshot and never runs a content projector. DSH's bounded opening-header
ownership check remains part of discovery and exposes no event prose. A
discovery-stage `unreadable`—directory I/O, or bounded DSH opening-header
I/O/UTF-8 failure that prevents a safe unique answer—is therefore the
presentation's own unavailability reason and closes admission. Only an
`unreadable` established after safe unique discovery, `unsupported-format`,
or a readable-empty outcome belongs to body-stage derivation and cannot become
the presentation reason. The shared token is stage-typed by construction: the
presentation constructor accepts a discovery refusal but no admitted-body
outcome. Do not widen `RunView` or the three-field Claude body response. Render the
authoritative common `kind`, `locator` and recorded `home` as a distinct local
fact, separate from the derived hint and body state, and send every string to
the DOM through `textContent`.

A drill is offered only for an already admitted Claude reference whose
canonical home equals the server's local projects home. On unproved/different
home, use the specified home explanation and checkpoint fallback. Codex/DSH
keep their shared hints and fallback without a Claude body request. A stale
flat id cannot override any common reference or explicit non-Claude legacy
provenance. The client Claude guard moves with the Rust leading-hex rule.
Admission is the kind-agnostic selection/validation/discovery state;
`drill_eligible` is the independent Claude-kind plus canonical-home-equality
gate. Every `· session <id>` label, id-only body request and growth watch
requires both. An admitted Codex, DSH or foreign-home Claude presentation
therefore remains displayable and recurrently rechecked but creates zero
Claude body requests and zero watches.

Treat the page as a small private transition machine, implemented with
ordinary client fields rather than a public framework:

- the active key is full run id, participant key and complete effective
  reference, never session id alone;
- a monotonic generation guards every presentation/body response and every
  watch callback;
- body state distinguishes missing, pending, succeeded (including an empty
  `turns` array) and refused in the current re-check interval;
- the exact active `EventSource` handle is owned separately; and
- a re-check epoch carries the one-automatic-watch-opening budget.

Isolate those facts and transitions in one start/end-delimited block inside the
served `ui.html`. The block exposes one private
`createTranscriptController(effects)` factory. Presentation/body requests,
watch open/close, timer scheduling/cancellation, prose clearing and painting
are injected effects; production supplies thin adapters around `fetch`,
`EventSource`, timers and DOM text nodes. The controller reaches no ambient
browser global internally. D11 executes that exact block from `PAGE`; there is
no copied JavaScript fixture, Rust transition twin or JSON policy interpreter.

Changing the active key, admission or drill eligibility bumps the generation,
closes the exact old watch and clears its body/prose before painting the new
presentation. A callback may mutate display or cache only when its key,
generation and owned handle are still current; an old callback cannot close a
new watch. Presentation and body HTTP responses use
`Cache-Control: no-store`, and the corresponding fetches request equivalent
freshness, so clearing client state cannot restore an HTTP-cached body.

Keep the journal-independent `/api/session/<id>` and `/sse/session/<id>`
selectors. Use shared safe discovery and projection, unchanged successful
body/size-event shapes, and the exact specified 404 envelopes. Every SSE poll
rechecks unique safe discovery; lost admission closes the stream before
further old sizes are sent. The page closes the exact `EventSource` before
handling its error and never delegates reconnection to the browser.

On closure or body refusal, clear body/prose, bump the generation and request
fresh presentation. If that result is not admitted or not drill-eligible,
render its shared refusal/hint/fallback and do no body or watch work. If it
still admits the same eligible drill and the refusal floor does not silence
it, fetch one ordinary body, paint it on success, and open a watch only when
the participant is working and that interval's opening remains. A refused
body—either 404 envelope, another non-success status, transport failure or
unparseable response—keeps the existing unqualified body-failure prose,
because the envelopes do not identify a reason, and silences body/watch work
for that key until the next recurring re-check.

An explicit `operator_select(subject)` is edge-triggered, not an equality
check. Every actual participant-selection entry point, including both the seat
row and graph node, must call this transition before rendering; no click path
may fall through to background `sync`. It always increments the generation,
closes the exact watch owned by the
prior generation, clears body/prose and pending work, clears the refusal floor,
begins a new re-check interval with one automatic-opening budget, and requests
a fresh no-store presentation. This sequence applies when `subject` equals the
active full key as well as when it differs; presentation equivalence may
suppress a background repaint but may never erase an operator action. Any late
body, presentation or watch callback from the preceding selection remains
inert under the generation/owned-handle guards.

Run one recurring presentation timer tied to the active selection, at least as
often as the existing runs poll, including for concluded participants.
Re-rendering must not accumulate timers. Each tick starts one new interval and
restores one automatic watch opening. An equivalent presentation—same
reference, admission, reason, hint and drill eligibility—does not repaint. It
may only repair a missing body and then, after body success, a missing watch
for a working participant. A body is missing only when none has succeeded
since turns were last discarded and none is outstanding. HTTP 200 marks it
succeeded even when `turns` is empty; equivalent ticks do not fetch it again.
A concluded participant continues presentation checks but never opens a
watch. Every automatic opening, including one performed by the tick itself,
spends the interval's single budget; a second immediate closure may refresh a
body but leaves the watch closed until the next tick. Every operator selection,
including same-subject reselection, establishes new state rather than consuming
the prior generation's refusal floor or recovery budget.

This design preserves the existing Claude routes without adding a Codex/DSH
body route, durable cache, watcher service or general transcript transport.

### D10 — Proposed decision 0055 is filed and remains unaccepted

This phase's rendered output is `design.md` only. Checkpoint `48739c6`
materialized proposed 0055 and its number-ordered registry row, but R25 now
supersedes the filed ruling's ordinary-JSON sentence. The complete text below
is the corrected design target for the later decision-owning phase; this
sitting does not edit an undeclared artifact, amend an accepted decision or
claim acceptance. The exact display encoder/framing, test-only dependency and
controller-harness bindings must all be present before the task prerequisite
can be treated as current.

<!-- proposed-decision-0055:start -->

#### 0055 — Read every retained transcript kind

Status: proposed
Date: 2026-09-09

**Context.** Decision 0032 records and retains the operator's transcript but
the local pane reads Claude alone. Issue #222 extends that local read to
Codex and DSH and adds a scriptable command. The OpenSpec change
`read-every-transcript-kind` supplies the detailed requirements and scenarios.
This proposal supplements ownership/retention/privacy and proposes to replace
only 0032 ruling 4's Claude-only command-construction enforcement binding.
Session resumption and sandbox re-imposition remain governed by 0030 and the
separate #226 work; hints execute nothing.

**Rulings.**

1. **The recorded reference is authority, independently of validity.** The
   latest common reference wins even when empty or refused; local JSON echoes
   its three strings unchanged and marks it nonlegacy. Only an absent common
   reference with eligible Claude/LaneTally/absent legacy provenance and a
   valid id can synthesize a Claude reference. Codex uses its existing
   1-128 ASCII alphanumeric/dash language with alphanumeric first character;
   Claude uses leading hexadecimal and 1-64 hexadecimal/dash characters;
   DSH uses a validated relative component path and absolute recorded home.
   The built-in 80-character recording clamp is unchanged; a reader never
   repairs a clipped locator, stitches attempts or borrows another seat.
   **Enforcement binding:** pure reference constructors, per-kind boundary
   tables, latest-reference/legacy and rejected-reference serialization tests.

2. **An owned local source is unique, safely opened and bounded.** Claude
   searches immediate project directories for the exact filename; Codex uses
   the whole filename-token predicate in its sessions tree through depth six,
   without a payload/header identity gate. DSH searches only its recorded
   root's project/session layout, admitting first-record `session` ownership
   with absent or unsigned-zero depth. Invalid depth is not coerced. A lookup
   examines at most 10,000 entries and at most 65,536 first-header bytes per
   DSH candidate. Below the canonical home, directory and leaf opens refuse
   symlinks/reparse points and nonregular sources, retain checked handles and
   report paths losslessly. Source input is at most 32 MiB plus one overflow
   probe byte. Pure view derivation has no I/O. A narrow local-reader
   production-dependency exception permits target-specific `rustix` fs and
   `windows-sys` filesystem bindings already in the lockfile; no provider SDK,
   shipped JavaScript runtime or broad filesystem framework follows. Ruling 4
   separately permits one pinned dev-only test engine for exact client proof.
   Directory I/O, or the bounded DSH opening-header I/O/UTF-8 check, can
   return discovery-stage `unreadable` when it prevents a safe unique answer;
   that refusal has no confirmed source path and closes browser presentation
   admission. An I/O/UTF-8 failure after safe unique discovery is instead a
   body-stage `unreadable` and does not retroactively erase admission.
   **Enforcement binding:** handle-based resolver, entry/header/source-bound
   tests, competing-failure/uniqueness tests, Unix/Windows ancestor/leaf race
   and nonregular-source tests, plus browser traces for both unreadable stages;
   frozen-file and dependency/license gates.

3. **Projection is a bounded audit interpretation, with explicit admission.**
   Preserve Claude's closed content/omission table. Decode Codex responses
   and recognized content-bearing events, including completed items; prefer
   a response only for proved content identity, preserving unassociated
   records without text or timestamp heuristics. This does not claim that
   id-less legacy mirrors can always be suppressed. Decode DSH numeric
   on-disk version zero only, after unique ownership and usable bounded
   UTF-8 acquisition. Missing, mistyped or foreign versions refuse as
   `unsupported-format`, with confirmed path/hint, no turns, zero counts and
   source-only truncation. Version cannot select among owned roots, a later
   header cannot repair the opening one, and unused replay-header metadata
   is not an audit admission gate. Omitted depth remains legacy zero;
   omitted version does not.

   Under admitted DSH version zero, packed rows decode into ordered logical
   events only after whole-row validation. Physical rows govern source bytes
   and diagnostics; logical events govern turns, timestamps and display
   bytes. Signed safe epoch-millisecond time is rendered as decimal; ordinary
   invalid time is absent, invalid packed storage refuses. Assembly suppresses
   only cited, unique, earlier same-turn/step chunks. Inclusive citation ranges
   match observed identities without expansion. Valid overlapping/duplicate/
   unordered citations are sets, deliberately differing from DSH's replay
   ordering validator; invalid encodings/self/future citations refuse.
   `surfaceOp` never rewrites audit history. Unknown DSH envelopes are counted
   omissions only with top-level boolean `ignorable: true`; required unknowns
   and invalid packed/citation rows refuse all prose while preserving complete
   usable-prefix counts and source-only truncation. I/O/UTF-8 failure precedes
   both format refusals and keeps zero counts.

   In every admitted projection, classify the complete bounded source and
   resolve associations before the 4,000,000-byte block-text budget. Retain
   complete turns until first overflow. Source/member order outranks time;
   partial appends and cap fragments supply no fabricated events. Missing
   partners and uncited fragments remain visible. **Enforcement binding:**
   pure projector tests for provider mappings, proved/unproved counterparts,
   DSH header/depth/version matrices, packed/plain equivalence, citation sets,
   all refusal stages, diagnostic units and both exact cap boundaries.

4. **Every local surface consumes one result.** `brokkr transcript` selects
   one run/participant and optionally a positive u64 one-based displayed turn
   after projection. Its `brokkr.transcript/v1` fields, closed reasons, stdout/
   stderr/exit rules, reference echo and metadata retention follow the command
   delta; future published structural changes require another schema version.
   It does not widen inspect/seats/watch or journal models. Shared notices
   have truncation/malformed/unrecognized ordering and exact spelling, with
   no Claude suffix. TUI snapshot replacement invalidates changed subjects,
   prior indices and overlays; active, final and manual reads recheck sources.
   Empty success and refusal remain different door states.

   The proposed replacement for 0032 ruling 4's command binding is: the shared
   local derivation constructs informational full-session lines by validated
   kind. Claude uses exactly
   `full session: claude --resume <id>`. Confirmed Codex uses exactly
   `full session: path <display-path>, codex exec resume <id>, home <display-home>`.
   Unresolved Codex uses exactly
   `full session: rollout unavailable, codex exec resume <id>, home <display-home>`.
   Confirmed DSH uses exactly `full session: path <display-path>`.
   Rejected/absent references and unresolved DSH references have no hint.

   One pure shared portable-display encoder produces every `<display-path>`
   and `<display-home>` as a valid double-quoted JSON string literal. It emits
   only ASCII letters, digits, `/`, `.`, `_`, `-` and `:` directly;
   every other Unicode scalar uses lowercase, four-digit JSON `\u` escapes,
   including a surrogate pair for a non-BMP scalar, with no short escapes.
   JSON decoding recovers the exact string. The comma framing contributes no
   semicolon, pipe, ampersand or redirection operator. The complete line is
   portable display data, not shell syntax or a pasteable command, and outer
   JSON serialization escapes that shared line a second time. TUI, CLI and
   browser participant presentation consume the exact shared value without
   fragment reconstruction; no other kind borrows a command and no display
   executes it. Browser presentation stays separate from body/journal models and performs
   selection, validation and discovery but no body projection. Its
   kind-agnostic source admission and the browser's Claude-kind/local-home
   drill eligibility remain distinct; every session label, id-only body
   request and growth watch requires both, so Codex, DSH and foreign-home
   sources never enter a Claude route. Active client work is keyed by run,
   participant and complete effective reference and guarded by a generation.
   Discovery-stage `unreadable` is its shared presentation refusal and closes
   admission; body-stage `unreadable` after a safe unique Claude discovery
   remains on the id-only route and follows the refusal floor below.
   Admission or eligibility loss closes the exact watch and clears prose
   before repaint, and stale responses cannot restore it.

   The selected participant receives a recurring presentation re-check at
   least as often as the runs poll, including after conclusion. Each interval
   permits at most one automatic watch opening, including one opened by the
   re-check. A refused body silences further body/watch work until the next
   interval; an equivalent result repairs only a missing body and then a
   missing working-seat watch. A successful body, including HTTP 200 with zero
   turns, is received until an explicit clear or refusal. Every explicit
   operator selection starts a new generation and interval even when it
   reselects the identical subject: it closes the prior owned watch, clears
   body/prose, pending work and the refusal floor, restores one opening budget
   and fetches fresh presentation. Late callbacks from the prior generation
   remain inert. Presentation/body responses bypass HTTP caching. Existing
   id-only Claude HTTP routes retain successful envelopes and the specified
   404/SSE-loss behavior; Codex/DSH body routes remain absent.

   The served page isolates this policy in one dependency-injected controller
   block. Tests extract and execute those exact `PAGE` bytes using the exact
   `boa_engine` 0.21.1 CLI dev-dependency with default features disabled,
   controlled effect adapters and drained Promise jobs. Boa is not linked into
   the release binary and adds no Node or browser service, but its lockfile,
   MSRV, license, audit and all-platform Cargo results are admission evidence.
   **Enforcement binding:** pure portable-display and exact-framing tables,
   adversarial path/home round trips and complete-line forbidden-character
   checks, CLI selector/text/JSON tests including the outer JSON escaping layer,
   shared-hint/notice cross-surface equality, headless TUI navigation and
   atomic-refresh tests, browser text-node identity, existing HTTP endpoint and
   thin-adapter source tests, and Boa-executed exact served-client transition
   traces for admission/eligibility loss, stale generations,
   discovery-stage unreadability, admitted non-Claude and foreign-home
   selections, body-stage unreadability, zero-turn success, refusal floors,
   both watch opening orders, recurring recovery, no-store refetch and
   identical-subject operator reselection.

5. **Reading retains private evidence and remains inert.** Only explicit
   local transcript reads expose requested prose; journal, checkpoints,
   exports, dossiers and result telemetry retain their existing path/id-only
   boundaries. No persistent body cache, provider process, repair, media fetch,
   deletion or transcript mutation occurs. Terminal output uses `Safe`, JSON
   preserves escaped strings and browser output uses text nodes.
   **Enforcement binding:** test-owned homes and synthetic content, retained
   byte/existence and journal count/hash comparisons, sentinel privacy tests,
   provider-launch fail sentinels and the unchanged exact coverage gate.

**Consequences.** This is an additive local document and reader, with no
stored-data migration or frozen contract/schema/corpus edit. It deliberately
breaks Claude's leading-hyphen id acceptance, first-match/unbounded/symlink
lookup and displayed-text-only input limit. The operator may inspect the
original independently, resolve duplicate placement, fit the recorded scope
within discovery bounds or use actual owned entries instead of below-home
symlinks. The reader offers no home override and reorganizes nothing.
DSH's invalid-depth refusal is deliberately stricter than the shipped
adapter; its version/citation policy is an audit policy, not execution replay.
Unproved Codex associations can remain visible twice, preserving evidence.
Exact browser-controller proof adds the pinned default-feature-free
`boa_engine` 0.21.1 development dependency and its lockfile graph; it does not
enter the released binary or permit Node, a browser service or an additional
production language. Only the operator accepts this proposal. Filing it does
not certify dependency admission, live resumption, implementation tests or
controller host/remote gates.

<!-- proposed-decision-0055:end -->

The filed decision changes the decision heading above to level one and uses
the registry's ordinary Context and Consequences section headings without
changing the rulings. Its number-ordered registry row is:

```text
| [0055](0055-read-every-transcript-kind.md) | Read every retained transcript kind | One bounded local projection of Claude, Codex and DSH transcripts, with recorded-reference authority, safe discovery, explicit format refusals and shared CLI/TUI/browser presentation; retained prose stays out of the journal. | proposed |
```

The decision file and registry row are present at the adopted checkpoint and
remain `proposed`. The file's R25 wording is not yet exact; the next owning
phase must apply the corrected ruling above and prove it before implementation
completion. This design commit does not modify either undeclared artifact.

### D11 — Verification proves boundaries through the shared implementation

Use existing crate suites and synthetic test-owned homes. Do not copy a live
operator session or edit the frozen evaluator corpus. Add table cases for
every scenario, grouping related cases rather than creating 179 bespoke test
functions. No parser-shaped mock in each renderer can replace cross-surface
comparison of the same serialized `Turn` and metadata.

| Requirement group | Design binding / proving suite |
|---|---|
| One derivation; recorded reference; full-session information | D2/D3/D7; view reference/result and exact-framing tables, portable-display round trips, and all-kind CLI/TUI/browser equality. |
| Browser eligibility; owned discovery; path ownership | D3/D9; CLI local reader and HTTP tests, native-vs-foreign home cases, rollout-shaped directory traversal, custom-home/legacy/refusal cases, lossless cross-target identity and platform safe-open/race matrices. |
| Claude, Codex and DSH content | D5-D7; existing view tests extended with source-grounded synthetic families and their unknown variants. |
| Partial records; source/display caps | D4/D6; below/at/above bounds, newline/UTF-8 cuts, DSH header-vs-event refusal collisions and physical/member counts. |
| Local inert prose | D7/D9; journal hash/count and retained-file before/after assertions; readout/export/dossier sentinels and no-provider-launch assertions. |
| Command run/seat selection; turn selection; JSON; text/errors | D3/D7; argument, read-only multi-hearth world resolver, cross-journal ambiguity/latest, ambiguous-label, parent/leaf, all reason states and whole/selected stdout/stderr tests. |
| TUI pane/doors; hints; notices; live refresh | D8; headless keys, scrolling and buffers, late appearance, pure append, assembly replacement, same-size rewrite, refusal/recovery, final/manual reads. |
| Browser presentation, eligibility and recovery | D9; HTTP/adapter tests plus Boa execution of the exact marker-delimited controller bytes extracted from `PAGE`, with the common reference painted separately, injected presentation/body promises, discovery-stage and body-stage unreadability, watch callbacks, timer ticks, both real click paths, selections and out-of-order responses. |

Command refusal proof must respect actual boundary reachability. Exercise JSON
and text documents for the four rejected common references admitted by the
frozen seat-record vocabulary: `none`, `unannounced`, `missing-home` and
`invalid-reference`. Separately prove that a `future-session` row is refused at
journal append or verification before command selection, producing neither a
`brokkr.transcript/v1` document nor stdout. Keep a direct pure-view case for an
unknown supplied kind returning `unsupported-kind` with all three strings
preserved. No one test is counted across those three boundaries.

Provider tests must include event-only Codex, completed-item-only Codex,
response/counterpart id pairs, id-less legacy records, repeated words and
capped/incomplete counterparts, exact wire casing, audit-visible distinct ids,
MCP server/tool/arguments, dynamic `content_items`/error and aggregate-only
command output. DSH tests prove that foreign header versions return before
later-row decoding/allocation and include all header version/depth
combinations in both candidate orders, packed/unpacked content equality,
negative time gaps, safe-number edges, huge citation ranges, partial/absent/
empty/overlapping citations, required/ignorable events and unknown nested
blocks, plus a bounded adversarial same-turn/step case that detects quadratic
citation scans. Browser tests preserve successful envelopes and verify exact
refusal responses and stale-request/watch cancellation. Platform race tests replace
ancestors and leaves, not just filenames before a preliminary metadata check.

The R25 matrix includes spaces, quote, reverse solidus, newline and other
controls; `$()` and backticks; `;`, `&`, `|`, `<`, `>`; `%` and `!`;
one BMP non-ASCII scalar; and one non-BMP scalar. Pure tests assert each exact
lowercase escape, four-digit or surrogate width, absence of short escapes and
JSON round-trip. Complete-value tests assert the fixed framing contains no
command separator and that every hostile path/home character is absent raw.
The same completed `full_session` string must compare equal across the view
result, CLI text, decoded CLI JSON, TUI pane and whole overlay, and browser
presentation; the JSON assertion separately proves the enclosing document's
second escaping layer. A harmless isolated shell probe may supplement this
matrix where available, but the direct alphabet and fixed framing are the
portable proof and no shell's availability is required.

Browser proof is behavioral, not a set of `PAGE.contains` assertions. Mark
one controller block exactly once in `ui.html`, extract those exact served bytes
from the existing `PAGE = include_str!("ui.html")`, and evaluate them with
`boa_engine` 0.21.1 from `brokkr-cli`'s dev-dependencies using default features
disabled. The evaluated block must expose only
`createTranscriptController(effects)`. Deterministic fake effects resolve or
refuse presentation/body promises, open and close identity-bearing fake
watches, deliver callbacks in adversarial order, tick the recurring timer and
record clear/paint/request/open/close effects. Drain Boa's Promise jobs after
each delivered event and assert both the effect trace and the controller's
small state snapshot before advancing. There is no copied `.js` fixture and no
Rust implementation of the controller transitions.

The production effect adapter remains in those same served bytes but outside
the extracted controller block. Focused source assertions prove it constructs
the controller exactly once, uses `encodeURIComponent`, requests no-store
presentation/body reads, owns `EventSource` and timer handles, and paints
untrusted strings through `textContent`; Rust HTTP tests prove the actual route
and SSE envelopes. This division executes the concurrent policy that can
restore stale prose while avoiding a false claim that Boa supplies a DOM,
network stack, layout engine or browser-level SSE conformance.

The executable traces pin zero id-only requests and watches for admitted
Codex/DSH/foreign-home presentations across re-checks; one total body request
for a concluded successful zero-turn Claude body; one refused body request per
interval for a persistently unreadable admitted Claude source; one automatic
watch opening for both re-check-first and closure-first intervals; a second
immediate closure repainting from a fresh body without opening another watch;
admission loss with unchanged journal/reference clearing prose and rejecting
an old response; recovery after admission returns; a no-store refetch; and
identity or eligibility change resetting only the new key's state. A separate
trace first exhausts the refusal floor and watch budget, then explicitly
reselects the identical subject and proves a fresh generation, presentation,
body and eligible opening while late body/watch callbacks from the prior
selection remain inert. Cross-surface tests compare the metadata subset only,
and prove the presentation carries no turns, blocks or transcript prose.

A separate discovery-refusal trace supplies an otherwise valid selected DSH
reference whose bounded opening-header I/O/UTF-8 check cannot establish unique
ownership. It must paint shared `unreadable`, its explanation, the null DSH
hint and checkpoint fallback with closed admission, null path/session label, zero
id-only body requests and zero watches; every recurring tick performs only a
fresh bounded presentation discovery. Table cases apply the same result to
directory I/O that defeats Claude or Codex uniqueness. This is distinct from
the admitted-Claude body-unreadable trace above.

The current `tasks.md` retains the settled R17-R24/A5-A7 repairs, scoped
synthetic-fixture rule, discovery-refusal trace, Rust 1.88 all-targets Boa gate,
release build and preserved-living-spec fold. Its 77 checked items predate R25
and therefore overstate completion. The next task office must rewrite and
reopen at least the shared helper/framing and adversarial table (2.5/2.7),
CLI exact-value proof (8.5/8.6/8.8/8.11 as applicable), TUI consumption
(9.4/9.5), browser transport/rendering (10.2/10.3/10.5), cross-surface and
privacy proof (11.1/11.2), affected guide wording (12.1/12.2), and every
downstream gate/fold/commit item whose evidence predates the repair (13.1-13.10).
It must also amend proposed 0055 before production edits. Controller-only
post-commit evidence stays outside tracked checkboxes. These corrections change
no additional product requirement or architecture; this sitting preserves
`tasks.md` because the rendered artifact is `design.md` only.

Implementation still owes, with `CARGO_BUILD_JOBS=2` and `RUST_TEST_THREADS=2`:
`cargo fmt --all -- --check`, clippy for all workspace targets/features with
`--locked -- -D warnings`, workspace tests with `--all-features --locked`,
locked compilation of both `bundles/self` and `bundles/verify`, and the distinct
house release build `cargo build --release --locked -p brokkr-cli`. The release
build does not replace Rust 1.88 all-target compilation of the Boa-bearing test
target or the proof that Boa is absent from the production graph.
Run the unchanged `scripts/coverage-exact.sh`; host proof requires
`TMPDIR=/var/tmp` and `BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1`, consuming the same
`rust-nightly-version.txt` pin as CI/release admission. A skipped nested
boundary test is not evidence for that gate. File these checks as pending
until their actual results exist; controller integration owns final host
proof, PR/remote CI, publication, merging and issue closure.

## Risks / Trade-offs

- **[Provider drift or id-less mirrors]** → Closed measured mappings,
  counted omissions and DSH refusal prevent false quiet success. Preserve
  unassociated Codex content; require new proof before suppression or a
  duplicate-free claim. Installed/live support remains separately measured.
- **[Filesystem portability and replacement]** → One narrowly scoped OS
  helper with native absolute-path admission, lossless target identities, held
  handles and deterministic race tests. Do not replace the guarantee with
  pathname checks or wrapping casts when a platform implementation is
  difficult.
- **[Bounded rescans can still be expensive]** → Read only the selected
  participant, retain lightweight candidates and use the indexed citation
  algorithm in D6. Entry, byte, event and range bounds plus the stated
  asymptotic ceiling bound work; no latency measurement is claimed.
- **[Prose escapes through a convenient model]** → Keep the local result out
  of journal-derived models; test serialization boundaries and inert displays.
- **[Ordinary JSON quoting leaves executable-looking fragments]** → Construct
  the path/home literal once with D7's restricted direct alphabet, use only the
  exact comma framing, and compare the complete shared value across every
  surface. Treat the outer JSON member escaping as a separate serialization
  layer; no renderer reconstructs a fragment.
- **[Claude compatibility costs]** → Preserve the explicit breaking list in
  proposed 0055 and the read-surfaces guide, with operator-owned remediation.
- **[A hint is mistaken for launch evidence]** → Show recorded-home and
  unavailable facts verbatim; no clickable execution or provider launch action is added.
- **[Filed decision drifts or is treated as accepted]** → D10 records the
  R25-corrected target and the current mismatch; the decision-owning phase must
  amend ruling 4 while preserving `Status: proposed` until the operator rules.
- **[The test engine expands the dependency and platform risk]** → Pin
  `boa_engine` exactly at 0.21.1, disable default features and keep it in
  `brokkr-cli` dev-dependencies only. Require the changed lockfile to pass Rust
  1.88 MSRV, license, audit and all-three-OS Cargo gates; if it cannot, return to
  design for another exact-code mechanism rather than weakening D11.
- **[The harness is mistaken for full browser conformance]** → Execute only the
  exact dependency-injected controller bytes and deterministic async effects;
  keep DOM text-node, URL, no-store and HTTP/SSE envelope checks at their real
  adapter/route boundaries and claim no layout or browser networking proof.
- **[Browser recovery restores stale or wrong-provider prose]** → Keep
  admission and drill eligibility separate, key state by full subject, guard
  callbacks by generation and owned handle, treat same-subject selection as a
  reset event, mark zero-turn success independently of array length, bypass
  HTTP caching and prove each transition from the exact served controller.
- **[One `unreadable` token collapses two stages]** → Construct presentation
  refusals only from selection/validation/discovery and body outcomes only
  after an admitted safe source; prove DSH/directory discovery refusal
  separately from admitted Claude body failure.
- **[Test safety wording forbids its own fixtures]** → Scope immutability to
  the system under test and real operator evidence; test setup owns and may
  mutate only synthetic fixtures between invocations.
- **[MSRV passes without compiling the controller harness]** → Add
  `--all-targets --all-features` to the Rust 1.88 workspace check locally and
  in CI so the dev-only Boa graph and controller test source are compiled.
- **[Premature archive cleanup destroys accumulated truth]** → Retain the three
  living capability specs as the base for the active `MODIFIED` deltas. Fold
  once into that state under the existing provenance destination; never delete
  the base, rewrite the pointer or convert the deltas back to `ADDED`.
- **[Dependency admission is mistaken for a release build]** → Keep the
  production-graph/MSRV checks and separately build the locked release-profile
  `brokkr-cli` target before final archive and commit.

## Migration Plan

1. Proposed 0055, its registry row and the settled F1-F3/R1-R25 repairs
   are present, but ruling 4 still carries R25's superseded ordinary-JSON
   sentence. The decision-owning phase applies D10's corrected wording without
   changing `Status: proposed`; the task office then rewrites and reopens every
   item whose proof predates R25 or lacks host evidence.
2. Preserve the staged reader and replace only the shared hint construction:
   implement D7's pure portable display literal and exact comma framing, then
   update CLI, TUI and browser consumers, adversarial cross-surface/privacy
   proofs and `docs/guides/read-surfaces.md`. No renderer receives a second
   encoder. Re-run every downstream gate on that repaired head.
3. No retained file or journal migration runs. Operators control any external
   placement repair; this reader changes neither paths nor bytes. A rollback
   restores the earlier reader without altering retained evidence; Codex/DSH
   unreadability and the earlier Claude lookup behavior would return.
4. Repair the existing browser participant drill and prose-free
   presentation route atomically, retaining the marker-delimited
   dependency-injected controller, thin production adapters and exact pinned
   Boa dev-dependency. Require exact served-byte traces—including same-subject
   reselection through both click paths—before enabling recurring presentation
   checks or disabling native EventSource reconnection. Existing successful
   Claude body/SSE envelopes remain compatible. Rollback restores the old
   browser client and routes together and removes the test-only dependency/lock
   delta, so neither side observes half of the recovery protocol.
5. Preserve the three existing living capability specifications and their
   append-only provenance lines throughout implementation and review. After all
   repaired implementation, proofs, the distinct locked release build and local
   checks are complete, archive the final `MODIFIED` deltas exactly once into
   that existing state under the recorded destination
   `2026-09-10-read-every-transcript-kind`; do not seed an absent tree, rewrite
   or duplicate the existing provenance pointer. Run the final workspace and
   archived-change validations against the folded head before the candidate
   commit. Controller host proof validates that fixed candidate without
   changing tracked checkboxes. The controller resolves shared-file overlap
   with #226; this work does not modify or import that sibling's worktree.
   Remote CI, publication, PR/merge and issue closure remain handoff actions.

## Open Questions

No unresolved behavioral choice is deferred to implementation. S3's remaining
legacy positional-association proof is not presumed: D5 selects the existing
preserve-unassociated behavior and states exactly what evidence would permit
additional suppression. A live provider census and latency measurement remain
unmeasured validation evidence, not authority to narrow requirements or run
models. A contradictory future measurement must return to its owning
requirement; neither unknown provider facts nor difficult platform code may
be disguised as downstream success.

## Validation of this sitting

Read the dialect and return instructions through
`openspec instructions design --change read-every-transcript-kind --json`;
they declare only `design.md`, and no workflow runner was invoked. Adopted
checkpoint `d76a310`, the clear clarification result, proposal S17 and the
owning full-session requirement/scenarios. Read both current council positions
completely and reconciled every material claim in D1: keep simplicity's small
architecture; adopt robustness's R25 correction because the current design,
decision, tasks and implementation demonstrably encode the superseded value;
reject unrelated public-vocabulary cleanup.

`openspec validate read-every-transcript-kind --strict --no-interactive`
passes with 20 requirements and 179 scenarios, and status reports all four
planning artifacts present. The frozen set remains byte-identical to shipped
main. At entry the task ledger is 77 checked / zero unchecked, proposed 0055
remains `proposed`, and the expected archive destination is absent. Those are
current-state facts, not completion: D10-D11 and the Migration Plan make the
decision/task/implementation/proof/fold repairs explicit instead of hiding
them downstream.

This workspace exposes OpenSpec and Git but no Cargo, rustup or provider
executable. The simplicity seat's prior local gate report and the historical
108/108 view result remain partial historical evidence only. Exact coverage,
repaired all-target/platform tests, Rust 1.88 Boa compilation, license/audit,
remote CI, publication and live-profile proof remain pending until obtained
for the exact repaired candidate. This sitting changes and commits only the
dialect-declared `design.md`; proposal, capability deltas, tasks, proposed
decision 0055, accepted decisions and frozen bytes remain untouched. Drafting
claims neither implementation completion nor operator acceptance. The phase
result carries `inputs.change: read-every-transcript-kind`.
