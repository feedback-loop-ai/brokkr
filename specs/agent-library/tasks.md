# Tasks: The agent library

**Feature slug**: `agent-library`
**Spec**: [spec.md](spec.md) · **Plan**: [plan.md](plan.md)

Ordered; each task names the test that proves it. `AC-n` refers to
spec.md's `## Acceptance Criteria`. Every task lands with its tests in
the same commit — `scripts/coverage-exact.sh` refuses `coverage(off)`
outright, so untested new code cannot ship at the end.

The ordering is deliberate: the witnesses are pinned **first**, the
library and resolver land **before** anything references them, and
adoption is **last** so that the slice's riskiest edit (`bundles/self`,
which the self-forge loop runs) is one revertible commit.

## Movement 0 — pin the witnesses before anything moves

- [x] **T1 — Golden `manifest_digest()` for the non-adopters.** Record
  the current digests of `recipes/fast` and `bundles/verify` as
  constants, asserting also that neither manifest has an `agents` key.
  Landed before any production edit, so the byte-identity claim is
  measured across the change rather than asserted after it.
  *Proven by*: AC-4.

- [x] **T2 — Golden journal for a non-adopting run.** A fixture run over
  a non-adopting recipe whose serialised events are compared
  byte-for-byte, so a stray payload field fails loudly.
  *Proven by*: AC-13.

## Movement 1 — the data formats

- [x] **T3 — `agents/` library: 16 definitions, 14 charters.** Charters
  `git mv`'d out of `bundles/self`, `recipes/panel-review` and
  `recipes/sdd` per plan.md's roster table (zero content change;
  `verifier`/`verifier-speckit` and `shipper`/`shipper-speckit` share a
  file). Definitions carry description, charter, ordered `models`,
  `tools`, `limits`; none carries `inputs`.
  *Proven by*: a test digesting every charter against the pre-move bytes
  — the move is provably byte-preserving.

- [x] **T4 — `adapters/`: five provider files.** `claude`, `lanetally`,
  `codex`, `dsh`, `exec`. Each declares binary, driver prefix, models,
  and for `model_flag` / `tool_permissions` / `mcp` either a mapping or
  the explicit string `"unsupported"`. Every value is derived from
  `crates/forge-protocol/src/adapters.rs` and the provider CLI's
  documented flags; where the truth is not established, the file says
  `"unsupported"` rather than guessing.
  *Proven by*: AC-9's fixture test proving the loader is data-driven,
  plus a test that `exec` declares all three unsupported.

- [x] **T5 — Loaders with strict parsing.** `Library::load`,
  `Adapters::load`. Unknown keys rejected; names match
  `^[a-z][a-z0-9-]*$` and are unique case-insensitively; charter paths
  canonicalised and contained within the library root; a model name
  mapped by two adapters is an error naming both files; `"allow": []`
  rejected as ambiguous; `secrets.env` in either tree refused.
  *Proven by*: AC-20, AC-11, AC-1's duplicate-mapping case — one test
  per rejection, each asserting the message names the file and key.

## Movement 2 — the pure resolver

- [x] **T6 — `resolve()` and `Availability`.** The signature of
  plan.md; `Presence` tri-state; `unspecified()` performs no
  availability filtering. First mapped candidate wins; `unavailable`
  skips and records `skipped[]`; the composed argv keeps `{forge}`
  unexpanded.
  *Proven by*: AC-1 — two processes, byte-identical output including key
  order, and an anti-drift test asserting `agents.rs` contains no
  `std::fs`, `std::env`, `std::process`, `Command` or clock reference.

- [x] **T7 — Capability checking, over every chain entry.** Grants
  (`tools.mcp`) versus restrictions (`tools.allow`); per named item, not
  per class; `optional` unrepresentable on a restriction; a gap on a
  *non-chosen* entry fails exactly as loudly as on the chosen one; a
  missing `tools.allow` records `tool_restriction: none`.
  *Proven by*: AC-2 (message names agent, provider, capability, item),
  AC-3 (optional gap produces a notice, not an error), and a test that
  the operator's `fable → qwen-3.8-max → gpt-5.6-sol` chain fails for a
  tool-restricted agent with a message a reader can act on.

- [x] **T8 — The resolution record.** The manifest record of spec Q1:
  digests, full chain, `chosen_index`, closed-vocabulary `skipped`,
  `notices`. Names and digests only; no argv.
  *Proven by*: a golden record for one agent, plus a test that a
  charter's byte change moves `charter_digest` and an adapter's byte
  change moves `adapter_digest`.

## Movement 3 — compile-time wiring

- [x] **T9 — `agent:` at seat, panel member and sequence step.**
  Extends the existing exactly-one-of check as a fourth alternative.
  `role`, `driver`, `limits`, `inputs` forbidden alongside; `results`,
  `secrets`, `confine` legal. Resolution runs **before** the existing
  lints and produces an ordinary body.
  *Proven by*: AC-21 (each conflicting key named), AC-22 (0007
  provenance, 0012 secret-reference, results-covered-by-a-rule and
  protected-phase reachability each fire on an agent-resolved seat),
  AC-5 (resolved seat equals the equivalent inline seat, element for
  element).

- [x] **T10 — `Bundle::compile_with`, and the manifest key.**
  `compile(dir)` keeps its signature and delegates with the `agents` /
  `adapters` defaults; the library is read only when a seat references
  an agent. `manifest_for` gains `agents`, absent otherwise;
  `manifest_diff` names an `agents` difference.
  *Proven by*: AC-4 (T1's goldens still pass), AC-9 (a fixture library +
  adapter dir compiles a bundle against a brand-new provider and model
  with no Rust edit in the test's diff).

- [x] **T11 — `contracts/run-manifest.v3.schema.json`.** v1's bytes plus
  one optional `agents` property; v1 and v2 unedited.
  `contracts/README.md` documents the two lineages.
  *Proven by*: a test asserting the frozen contract files' digests are
  unchanged by this slice.

- [x] **T12 — The v2 lineage refuses rather than truncates.**
  `build_run_manifest_v2` errors when the bundle manifest carries
  `agents`, naming the limitation and the follow-up.
  *Proven by*: AC-19 — the refusal, plus non-adopting dispatch unchanged.

## Movement 4 — the journal and bounded fallback

- [ ] **T13 — `AttemptReport.accepted`.** Surfaced from the local
  `run_attempt` already keeps.
  *Proven by*: a process-layer test for both values, and the existing
  driver-protocol suite staying green.

- [ ] **T14 — Provenance events.** `effect/started.provenance` as a list
  over invocation sites, absent when no site is agent-resolved;
  `effect/failed.start_failure` plus member tag when the structural
  predicate holds; `contracts/effect-provenance.v1.schema.json`
  published; `contracts/README.md`'s prose amended in this commit with
  its reason.
  *Proven by*: AC-13 (T2's golden — a non-adopting journal gains no
  field; `fold` yields an identical `RunState` for an adopting run),
  AC-16 (a two-provider panel produces two records; sdd's design
  sequence reports `claude` and `exec` separately).

- [ ] **T15 — Bounded fallback.** Candidates on `Single`, `StepBody` and
  `PanelMember`, empty for inline seats; per-site index derived by
  scanning the effect's events for prior `start_failure`s, clamped to
  the last candidate; `max_attempts` untouched.
  *Proven by*: AC-6 (absent binary → next candidate, journaled, inside
  the bound; exhaustion parks with the last error), AC-7 and AC-14
  (accepted-then-failed does not fall back; `effect/indeterminate` still
  never auto-retries), AC-15 (restart between attempts selects the same
  candidate).

## Movement 5 — the readouts

- [ ] **T16 — One derivation.** `forge-view` gains `Provenance` on
  `Participant`, derived at the `EffectStarted` arm; run-level notices
  read from the already-journaled `run/started.payload.manifest.agents`;
  `VIEW_VERSION` → 2.
  *Proven by*: the existing view goldens moving once, plus a derivation
  test for a fallback run (`chain_index > 0` ⇒ `fallback: true`).

- [ ] **T17 — Four surfaces render it.** `render.rs` (`inspect`, `runs`,
  `watch`), `tui.rs`, `ui.html`, and a run-level notice line for
  `chosen_index > 0` and optional-capability gaps.
  *Proven by*: AC-8, AC-17, and the anti-drift test that no surface
  formats provenance itself.

- [ ] **T18 — `forge compare` resolution divergence.** A first-class
  `resolution_divergence` per seat and member, computed by calling the
  `forge-view` derivation for each run — what actually ran, not what was
  pinned — reported unconditionally, including when `same_recipe` is
  `true`.
  *Proven by*: AC-18, including the case where the digests agree and the
  models do not.

## Movement 6 — the CLI

- [ ] **T19 — `forge agents list|show`.** `list` prints
  `name ⇥ chain ⇥ description`, warning without aborting on a broken
  file; `show` prints the definition plus a `resolution` block from the
  same `resolve`; unknown name errors naming the known set.
  `--agents-dir` / `--adapters-dir` default to `agents` / `adapters`.
  *Proven by*: AC-10.

- [ ] **T20 — `forge doctor` reads the adapter files.** The hardcoded
  five-tuple is replaced by a loop over `adapters/`, reporting per
  provider its binary, its probe result and its declared models, and per
  agent which model would be chosen **here** — by calling `resolve` with
  the probed `Availability`, which is the non-`unknown` arms' real
  consumer. The injected `probe` fn is unchanged.
  *Proven by*: AC-10 — a fixture adapter dir with a sixth provider shows
  up in `doctor` with no rebuild.

## Movement 7 — adoption, last

- [ ] **T21 — `recipes/panel-review` and `recipes/sdd` reference
  agents.** Every seat, panel member and sequence step except
  `design > speckit-check`, which stays inline.
  *Proven by*: AC-5 per seat (resolved equals the pre-adoption inline
  body except for the added `--model`), and both recipes still
  compiling and passing every bundle lint.

- [ ] **T22 — `bundles/self` references agents.** Landed as its own
  commit, revertible alone, because the self-forge loop runs it.
  *Proven by*: T21's per-seat equality plus the `recipes` listing test.

- [ ] **T23 — Docs.** `README.md` and `ARCHITECTURE.md` gain the
  library, the adapters and the honesty rules — including the named
  limits, stated as limits: Looper-dispatched runs cannot adopt agents,
  provenance does not cross the bridge, and "no `Accepted` ever arrives"
  parks rather than falling back.
  *Proven by*: review — and by the spec, which says the same thing in
  the same words.

- [ ] **T24 — The gates.** `cargo fmt --check`, clippy warning-free
  across all targets and features, the 97-case differential corpus and
  the machine-proof suite unmodified and green,
  `scripts/coverage-exact.sh` at its exact nonzero 100%.
  *Proven by*: AC-12.
