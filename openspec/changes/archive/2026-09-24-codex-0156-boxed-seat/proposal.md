Status: proposed

## Why

The operator's 2026-09-24 host measurement found that Codex 0.156.0 defers
MCP hands behind `tool_search`; several boxed wager seats stopped at intentional
native-write refusals without finding their writable workspace tool. The
engine must explain how to discover the selected provider's hands so that a
native sandbox refusal does not become a false delivery blocker.

## What Changes

- Add an optional, narrowly structured discovery declaration to a provider's
  `hands`: the workspace tool name and the tool used to discover it. Codex
  declares `mcp__brokkr__workspace` and `tool_search`; existing declarations
  without this metadata keep their behavior.
- Have the engine supply the selected provider's discovery facts only for a
  model seat whose workspace hands are actually boxed. Render fixed guidance
  naming both tools, explaining that native writes are refused by design and
  are not a blocker, and directing workspace and result-file writes through
  the hands tool. Render no other adapter or launch configuration.
- Make the guidance engine-owned across every executing site and provider
  fallback. Recipes, their input declarations and charter text cannot author,
  override or suppress this structured notice. Keep the existing generic
  hands paragraph and result-delivery contract.
- Append the supplied, dated 0.156.0 hands-discovery observation to Codex's
  adapter evidence or limitations. Preserve `resume.work-site.identity.version`
  and `applies_to` at `0.154.0`, all resume qualifications, and task 11.1's
  separate operator dependency.
- Measure affected bundle identities after the adapter edit and re-pin only
  witnesses and compose assertions whose actual compiles change, including
  chains that consult Codex as a fallback.
- Require deterministic Rust proofs for the rendered guidance, its absence
  on other seats, its ownership and its narrow data boundary. Bind each claim
  to a compiling mutation, a relevant assertion failure, restoration and a
  passing rerun; no live provider call is required.

## Capabilities

### New Capabilities

- `boxed-hands-discovery`: provider-declared discovery facts, engine-owned
  conditional prompt guidance, dated evidence without resume requalification,
  measured identity movement and mutation-bound proof.

### Modified Capabilities

None. Existing `boxed-work-provider-admission` and realm-boundary requirements
remain intact; this adds guidance after their existing admission and selection.

## Impact

The implementation touches Codex adapter data, Rust adapter loading and
candidate propagation in `brokkr-runtime`, execution-site input assembly and
prompt rendering in `brokkr-protocol`, their proving suites, and measured
witness/compose pins. Narrow provider-adapter documentation and a new numbered
house decision with `Status: proposed` and enforcement bindings accompany the
semantic implementation. Only the operator can accept that decision. No new
crate, dependency, provider call, workflow runner or release is required.

This specify visit authors only this proposal, its capability delta and the
OpenSpec-created change metadata. Council design, tasks, the numbered house
decision, production edits and mutation evidence belong to subsequent phases.
The choices below are specification constraints for that design to reconcile
under `## Decisions`, not a claim of implementation or operator acceptance.

## Decisions

1. **Adapter facts; engine selection and rendering.** Decision 0016 makes
   provider differences adapter data. Declare two tool identifiers, with a
   closed, bounded shape; the engine supplies the fixed explanatory wording.
   The engine knows the actual execution site's hands, boundary and selected
   provider, including fallback, so it owns applicability. The owning delta's
   scenarios make these answers executable acceptance criteria.
2. **No charter or recipe workaround.** Decision 0041 ruling 8 keeps offices
   portable and engine facts in engine-rendered text. A charter would drift
   across offices and could not reliably describe the provider serving a
   fallback. Reject recipe opt-ins, suppression flags and provider-name
   matching as the source of the new notice; a declaration is authoritative.
   Preserve decision 0043's existing generic paragraph, which already names
   the workspace tool but does not explain deferred discovery.
3. **Move byte identities, preserve qualification identities.** Adapter files
   are hashed as bytes and resolution pins every consulted provider. The
   declaration and dated evidence therefore require measured digest updates.
   Neither change establishes resume safety on 0.156.0: changing the qualified
   version or attempting task 11.1 is refused as outside this commission.
4. **Preserve the boundary.** Reject a native-write workaround or a switch to
   disable deferral: the supplied measurements found no working switch, and
   widening writes would defeat the commissioned box. Guidance grants no
   capability and changes no MCP registration, permission or sandbox flag.

## Evidence and scope

Read at `aa58d07a`: `README.md`, decisions 0004, 0005, 0009, 0016, 0041,
0043, 0046 and 0063; `.forge/tasks/codex-0156-boxed-seat.md`; the Codex and
Claude adapters; `adapters.rs::hands_paragraph` and `render_prompt` with their
existing tests; `engine.rs::mark_hands` and candidate selection; the closed
hands loader and the consulted-adapter digest in agent resolution. Relevant
history includes `39b6b47d` (0.154.0 task 11.1 assertions), `9da5ff92`
(qualification evidence) and `314e8786` (measured roster/pin movement).

The 2026-09-24 provider observations are supplied host evidence, not new probes
by this seat. The delta records their scope and the exact version boundary.
No council positions or `returned_from` findings were supplied on this visit.

Frozen `contracts/`, `policy/phase-machine.json`, `policy/schemas/`,
`fixtures/`, `reference/` and `extensions/` do not move. Production remains
Rust under `crates/`; supported hosts are Linux and macOS only. This slice
adds no resume coordinate, capability-grant work from 0065, model/roster
change, live wager, release bump, publication or push. Implementation must
run the house checks and report exact coverage honestly: host/CI namespace
validation remains pending until measured, never silently waived.

## Specification validation — 2026-09-24

`openspec validate 2026-09-24-codex-0156-boxed-seat --strict --no-interactive`
passed. The dialect's status command reports proposal and specs done, design
ready, and tasks waiting for design. This is a specification draft, not an
implementation-complete claim; no runtime test or mutation result is claimed.

The box exposes neither Cargo nor rustup on PATH. Attempts at workspace tests
(`--all-features --locked`), formatting, clippy and the self-bundle compile all
returned exit 127, `cargo: command not found`. The unchanged exact-coverage
script likewise stopped at its Cargo invocation (line 33, exit 127), before
producing coverage evidence. These checks remain pending in a seat with the
Rust toolchain; boundary coverage additionally needs the host/CI execution
already required by the house. No threshold, test or boundary was altered to
turn this tooling absence into a passing result.
