## Why

The operator ruled on 2026-09-20 that the engine smith hires Astra, then Fable,
at high effort; changing only the roster currently fails because Codex cannot
express its tool list. Issue #307's 2026-09-21 rulings resolve both triage
escalations: declare decision 0043's existing workspace hands and preserve
0046's separate namespace and harness paths.

## What Changes

- Seat `agents/implementer-engine.json` on `["astra", "fable"]`, with both
  efforts `high`, and declare workspace hands with network disabled. The
  existing workspace mount supplies writable project access; the established
  masked Cargo overlay and read-only Rust toolchain supply its build tools.
  Remove the now-inactive `tools` object; keep its charter and limits.
- Prove the existing compile rules using temporary Codex fixtures: unchanged
  tool-list refusal without hands, namespace admission with hands, refusal
  without `hands.workspace`, and the separately resolved harness outcomes.
  Source inspection indicates no compiler change is needed. Change production
  code only if the tests expose a gap, using the smallest repair consistent
  with decisions 0043 and 0046 and preserving the refusal text verbatim.
- Prove the engine smith's actual namespace launch composition: the MCP hands
  server receives its workdir and declared binds, Codex's native sandbox is
  read-only, and neither `hands.harness.work` nor a tool-list flag travels.
  Every required compile and composition assertion carries removal evidence.
- Re-measure affected manifests and update every moved witness and compose
  digest, with an explanatory history entry. Preserve panel diversity,
  GPT/Flash's separate crew and the whole runtime suite.
- Clarify provider-adapters guidance and append a dated 2026-09-21 operator
  note to decision 0043 without changing its status. Record expressibility
  and the controller's still-pending first live Astra implementation as
  different evidence.

## Capabilities

### New Capabilities

- `astra-engine-smith`: The engine smith's ordered hire, declared confinement,
  manifest identity, documentation and bounded evidence claims.
- `boxed-work-provider-admission`: Regression and removal proof of existing
  provider capability refusals and mutually exclusive namespace/harness
  composition for this work seat.

### Modified Capabilities

None. Existing `realm-boundary` and `gate-boundary-policy` requirements remain
in force. These deltas specify the new roster and its evidence obligations;
they do not amend those capabilities' boundary or admission semantics.

## Impact

Implementation touches the engine-smith agent, runtime tests (agent resolution,
bundle policy, engine boundary composition, roster, witness and compose pins),
`docs/guides/provider-adapters.md`, and the dated note in
`docs/decisions/0043-the-hands-are-one-tool.md`. No adapter capability, protocol,
dependency or boundary vocabulary changes are planned. The default namespace
bundles hiring the smith regain compilability with the ruled hire. A harness
realm still needs `hands.harness.work` on every link: the shipped Claude
fallback lacks it and must refuse, not acquire a guessed fragment.

This specify visit authors only this proposal, its two capability deltas and
OpenSpec's generated change metadata. Design, task breakdown, implementation,
tests and production documentation belong to their subsequent phases.
`contracts/`, `policy/phase-machine.json`, `policy/schemas/`, `fixtures/`,
`reference/`, `extensions/` and the issue #226 task ledger are outside the
change. Hosts are Linux and macOS under decision 0063; temporary test roots
must be canonicalized. No workflow runner, live smith or publication is part
of specification.

## Decisions

### D1 — Adopt the operator's restriction ruling; reject the withdrawn premise

The 2026-09-21 issue ruling is the answer, not an unresolved question. The
restriction is the existing empty-root filesystem and network boundary, not
`["cargo", "git"]`. `agents.rs::compose` ignores `allow` when hands exist;
`HandsSpec` holds network and binds; `hands::execute_in` runs `bash -lc`.
No command allow-list, shell-command policy, transport contract or new
native-tool bypass prevention is authorized. The deltas encode this answer
as scenarios. The unchanged no-hands refusal remains necessary.

### D2 — Remove the inactive tool declaration and use existing boxed binds

The boxed work agents `analyst`, `clarifier`, `chief-architect` and `intake-sdd`
declare `network: false`. The release manager's explicit network grant serves
release work and is not this smith's precedent. The reviewer and release
manager already declare `~/.cargo` as an overlay masking `credentials.toml`
and `credentials`, and `~/.rustup` read-only. Use those build-tool binds; the
workspace itself is automatically bound read-write by workspace hands, so no
absolute checkout path or invented bind token belongs in the agent.

Remove `tools` rather than retain a misleading Claude-only grant: 0043 ruling
2 ignores it on both providers, and
`tool_grants_keep_house_tools_explicit_and_effort_never_rises_on_fallback`
already refuses dead tools beside shipped hands. A test-only agent with both
fields proves replacement without changing that shipped roster rule.

### D3 — Preserve the two existing launch paths and their refusals

`agents.rs::compose` requires `hands.workspace` for boxed hands;
`bundle.rs::enforce_hands_boundary` requires `hands.harness.work` on every
harness work link. `engine.rs::compose_site` selects namespace hands or the
harness fragment by boundary. These source facts support tests before any
compiler edit. Namespace admission must not borrow `workspace-write` from
the harness path; harness admission must not imply a Brokkr box. Preserve
existing open/other-boundary behavior without widening this slice.

### D4 — State the evidence boundary honestly

The guide must retain decision 0043's Codex limitation: the native shell
remains available read-only outside the hands box, and provider traffic is
outside it. No host-read secrecy or command filtering is established here.
Deterministic compilation/composition can show the ruling is expressible,
not proved in a live Astra smith. This seat has no network; the first live
Astra implementation is the controller's measurement after landing: Cargo
and Git through Codex's boxed hands, a real commit, and verify passing.
This enacts existing accepted semantics, so no new semantic decision is
asserted or accepted; 0043's status stays unchanged.

## Specification validation — 2026-09-21

The specify artifacts pass
`openspec validate 2026-09-21-307-astra-engine-smith --strict --no-interactive`
and `openspec validate --all --strict --no-interactive` (16 items passed,
zero failed). OpenSpec reports proposal and specs done; design and tasks are
not authored in this phase. Existing informational archive diagnostics on
other changes do not fail strict validation and are outside this slice.

The following checks were attempted through the workspace tool and could not
start because `cargo` is unavailable in this seat's box (exit 127):
`cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
`cargo test --workspace`, `cargo test -p <crate> --all-features --locked` for
each of the seven workspace crates, and
`cargo run --locked -p brokkr-cli -- compile --bundle bundles/self`.
`bash scripts/coverage-exact.sh` likewise stopped at its first Cargo command.
These are unverified, not passing results. No Rust production or test lines
were added during specify. Implementation and its removal proofs are future
work; exact coverage still requires the established host/CI measurement
outside the namespace box with the literal threshold unchanged. No live
Astra smith was attempted or proved.
