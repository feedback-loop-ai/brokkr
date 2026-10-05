# Operator rulings, 2026-10-03

R1. THE BROKER IS BROKKR'S OWN AND THE HARNESS'S CHILD. Code shows the harness, not the engine, launches the hands server from the MCP config the engine writes, and the final launch check admits exactly one server. So a granted mcp capability is served by `brokkr broker serve`, listed in that same engine-written MCP config beside the hands server, one server per held capability. The broker reads its secrets from the operator's store itself (never in argv, never in the harness's environment), starts the dialect's real server with them, exposes only the granted tools, refuses an ungranted call, masks secrets in its own output, and records every call. It lives in the attempt's process group, so #403's cleanup covers it and its child. This meets ruling 6's intent but not its letter ("launched by the engine"): write a proposed decision amending 0065 ruling 6 — number 0077 unless claimed (check docs/decisions/README.md's gap table and open pull requests) — with Status: proposed, and Amends/Amended by pointers both ways.
R2. FIRST SCOPE: an mcp grant is admitted only at a site inside a Brokkr box (boundary `namespace`); every other site refuses it with an exact reason. Widening is a later decision.
R3. CODEX CONFIG ISOLATION is decided by measurement: the first unit (U0) measures how each harness excludes the operator's own MCP servers; the design names the candidates and the measurement, not a guess.
R4. RETENTION: a dialect declares `retained`; the realm may veto it with a new reserved grant key; retained responses are stored content-addressed at .forge/artifacts/sha256/<hex>; the engine stays the journal's only writer (the broker writes a per-attempt ledger file that the engine folds in).
R5. PROCESS: this run produces the openspec change; the operator rules on it; then each unit lands as its own pull request from main, through the merge queue, signed, inert until the enabling unit.

## Addendum, 2026-10-04: U5a is split

U5a's binding-minimum comparison (MB4) needs `crates/brokkr-runtime/src/bundle.rs`, where the minimum is parsed after the authority loads and every serving context is built — a fourth production file. Ruled: split, not admit. U5a lands with the typed dialect policy, reservation and retention binding; the new unit U5a2 (capabilities.rs, capabilities/binding.rs, bundle.rs) carries the minimum into resolution with MB4's exact below-minimum cause and closes tasks 24.3–24.4; doctor's matching comparison joins U9a, which now depends on U5a2. The MCP compile fence keeps the gap inert until U9b.

## Addendum, 2026-10-04: reserved keys are stripped, not searched for

Three U5a council rounds each found a new construct where a static walk of a dialect's restriction schema disagreed with the draft-07 validator (pointer decoding, `$id` rebasing, inactive conditionals). Ruled: SC2's guarantee is that a grant's reserved keys never reach restriction validation (the restrictions validated are the grant's keys minus its version's reserved keys), so no schema composition can decide one. The collision refusal covers direct claims only: a key of the root `properties` or `dependencies`, or an entry of the root `required` or a root `dependencies` array. Indirect claims are not searched and are inert.

## Addendum, 2026-10-05: U5f may drop one cfg(test) in binding.rs

Manifest v12 records each holding's effective retention, whose one owner is `Retention::effective` in `crates/brokkr-runtime/src/capabilities/binding.rs`, staged `#[cfg(test)]` by U5a. Ruled: a one-time exception admits that file to U5f only to drop the attribute and fix its doc comment, so the manifest reads the rule in production; copying the rule (decision 0071 ruling 5) stays forbidden. Test files beyond the owning suites (v11 validator swaps, a child test module beside an over-ceiling suite) are the controller's standing clarification, not an exception.

## Addendum, 2026-10-05: U4a2 erases driver attribution before v6 admits it

U4b's council held seat-record v6 because it would admit a complete capability-call attribution group supplied by a driver (a forged `response_sha256` sealed as journal evidence) while the engine's own attribution arrives only at U4e/U4f. Ruled: erasure first. A new unit U4a2 (engine.rs, engine/checkpoints.rs) removes every driver-supplied CC1 attribution field before append, then U4b activates v6 as designed.

## Addendum, 2026-10-05: each MCP server runs in its own box

"U6c's broker session hit two security holds trying to prove that an arbitrary MCP server's startup arrangement cannot be tampered with. The second round found seven HIGH gaps: the ELF loader and RUNPATH, Python sibling modules, ever more code-loading environment variables, shebang parsing, private-directory ancestry, and bind mounts hiding the secret store. Ruled: the broker spawns each MCP server inside its own Brokkr box (bubblewrap, the machinery of decision 0043): system directories and the server's program tree bound read-only, network per the dialect's egress class, private HOME and TMPDIR, the workspace and every path the seat reaches never mounted, and the cleared environment holding only the declared secrets and fixed variables. Tampering through the seat's reach is then impossible by construction, rather than proven case by case, for user-installed servers too. Linux only, as R2 already implies."
