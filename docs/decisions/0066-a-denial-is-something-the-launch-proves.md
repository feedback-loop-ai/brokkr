# 0066 — A denial is something the launch proves: repairing the council's security hold on decision 0065's first slice

Status: proposed
Date: 2026-09-22

## Context

Decision 0065 ruled three things that are not negotiable: every
capability is OFF BY DEFAULT with no grandfathering, only the REALM
grants one, and TOOLS ARE AN ABSTRACTION a dialect makes concrete. Its
first slice was built to 40 of 43 tasks on `slice-0065-capabilities`,
and a full council — adversarial, correctness, security,
spec-compliance and a chief — judged the found head `f0264a9b`. The
verdict was VERIFY PASS, REVIEW SECURITY-HOLD: four HIGH and four MEDIUM
findings, each confirmed by the chief against source, none fixed. The
chief's ruling is the finding source for everything below.

All four HIGH findings are ways the delivered code was FAIL-OPEN against
0065:

- **H1.** `load_pin_adapters` swallowed an adapter load error with
  `.ok()`. The loader was written for an effort exemption, where "no
  adapters" safely means "no exemption"; the capability pass then read
  the same absence as "inventory unmeasured" and compiled a Codex seat
  with no search OFF. An unrelated malformed adapter file was enough.
- **H2.** A recipe's authored driver argv could carry Codex
  `mcp_servers.*` configuration, or Claude `--mcp-config` with an
  `mcp__*` allowed tool, under `grants: {}`. The native guards cover
  declared native controls only; nothing judged a concrete server.
- **H3.** The compiler accepted a native control expressed as argv for
  Claude, and the Claude launch consumed only the tool selection. An OFF
  written `--disallowedTools WebSearch` was recorded and never delivered.
- **H4.** A specification defect. Design D7 excluded the top-level
  `capabilities/` directory from the file walk at every recipe layer,
  while `parse_role` accepts a charter there and `compose::own_table`
  accepts a policy there. A changed charter or policy under that tree
  left the bundle digest unchanged.

The four MEDIUM findings: the doctor promised a native denial whenever a
matching grant existed (M1); a duplicate request key could weaken
`requires` to `wants` before validation saw it (M2); a loaded agent no
seat referenced escaped the definition lint at compile (M3); and the
optional-notice half of two compatibility proofs had never failed on its
own removal (M4). One new test also glued a temporary path where macOS
reports `/private/var`.

## Rulings

1. **A denial is something the launch proves, never something absence
   implies.** A provider known to carry a native power — the floor is
   Codex `web-search`, Claude `web-search` and `web-fetch`, plus whatever
   its adapter declares — is launched only with a valid, delivered
   control for every such power, or it is refused. Absent adapter roots,
   absent providers, legacy or omitted or empty `native_capabilities`, an
   omitted floor power, an unreadable or malformed adapter file
   (unrelated ones included) and an unmeasured OFF of a known unheld
   power each refuse compilation with the site, office, realm, provider,
   capability and the original cause named. The floor grants nothing,
   supplies no switch and is no evidence; concrete controls stay adapter
   data. DSH, LaneTally and opaque custom drivers keep their own declared
   uncertainty and inherit nobody's inventory.

2. **The same law holds at the driver.** The engine's plan is decoded
   fallibly: a wrong type, a malformed guard, an incomplete selection
   mapping, an unmeasured inventory for a floor provider or a floor power
   in neither the ON nor the OFF set refuses before provider work. No
   error path degrades to an empty plan; the cold replacement of a
   rejected Codex rejoin carries the controls the launch already
   validated rather than decoding again with `.ok()`.

3. **A control the compiler accepts reaches the final argv, or the
   compile refuses that representation.** One fallible composer in
   `brokkr-protocol::native_controls` owns what each provider consumes.
   Codex consumes argv. Claude and LaneTally consume argv and selection:
   managed list argv is normalised into the same include/allow/deny
   lists as a selection and each list flag is emitted once; a non-list
   argv (a restriction transport) is appended verbatim. A form a provider
   does not consume — a selection for Codex, anything for DSH — refuses
   at compile and again at launch, naming the provider and the form.
   The compiler and the launch call the same function.

4. **A recipe's authored driver command is recipe data, so it carries no
   capability.** The engine keeps what was authored apart from what it
   owns: the adapter's `hands.workspace` fragment under the box, its
   `hands.harness` fragment under `harness`. Provenance is a fact carried
   from `agents::compose` through `Candidate`, `SiteSpawn` and the
   driver's private `launch_arguments`; it is never recovered by matching
   text, and a server named `brokkr` proves nothing. In the authored part
   the engine refuses Codex `mcp_servers` tables and descendants under
   `-c`/`--config` in split and joined spellings, and Claude/LaneTally
   `--mcp-config` plus any allowed-tool list that admits `mcp__*` or a
   wildcard. DSH keeps its one bound, contained, digest-matched
   route-only `--patch`; a row outside that closed grammar refuses. An
   engine launch whose provenance is missing, or whose two parts do not
   reassemble the argv it was handed, refuses. The by-hand driver
   interface is unchanged and cannot mint provenance.

   *Migration:* an inline model seat under a boxed boundary used to author
   the box's tokens itself. It is now refused with the way out: seat the
   office through an agent, whose adapter owns the hands fragment. No
   shipped recipe does this.

5. **Nothing that can change what a seat is told, or how a run is ruled,
   sits outside the bundle's identity (corrects design D7 and task 6.1;
   `spec_defect=true`).** The `capabilities/` exclusion stays — consulted
   definitions are pinned by name and digest, and an unconsulted one must
   not move identity — but only with an enforced refusal: a role or a
   policy whose lexical path or canonical target falls under a top-level
   name the walk skips is refused at the layer that declares it, ancestor
   layers included, before the bytes are read. Relocation to an ordinary
   pinned path is the migration. No second identity inventory and no new
   manifest version.

6. **The doctor never claims a denial the launch does not deliver.** The
   OFF disposition is judged before any grant is described. Supported OFF
   reports the declared denial within its evidence limits; unsupported
   OFF reports the compile refusal and its reason; unmeasured OFF keeps
   its reason, claims nothing and reports ruling 1's refusal.

7. **Request sources are read strictly.** `compose::read_layers` and
   `agents/load::read_json` parse their original bytes with
   `parse_strict` before any map conversion: a repeated capability name
   or a repeated `capabilities` field refuses, in either strength order,
   equal repetitions and a later `{}` included, at every layer and every
   nesting. Other duplicate keys in those same documents refuse too.

8. **Compilation lints every loaded agent's requests.** Where a library
   is loaded at all, `Definitions::lint` runs over the whole of it before
   any seat is resolved, and every definition a loaded agent names is
   pinned as consulted, seated or not.

9. **A removal proof proves the assertion it is named for.** Required,
   optional and unused cases of provider compatibility and restriction
   compatibility are separately runnable, and each optional test's first
   substantive assertion is whole-vector notice equality.

## Rejected

- *An embedded emergency OFF catalogue* for a missing adapter: it would
  put a concrete switch in the engine and claim a measurement nobody made.
- *Pruning a broken provider from a chain* instead of refusing: a new
  fallback policy, and a silent one.
- *Recognising engine-owned configuration by its bytes or server name:*
  exactly what an author can counterfeit.
- *Banning every DSH patch:* the bound route overlay is legitimate and
  carries no server or tool.
- *Pinning the whole `capabilities/` tree:* an unconsulted definition
  would become a second source of identity.
- *A doctor-only resolver:* a second calculation drifts from launch.

## Enforcement bindings

| Finding | Enforcement | Proof |
| --- | --- | --- |
| H1 | `bundle.rs` preserved adapter `Result`, `capabilities.rs` known-power floor and unmeasured-OFF refusal, `native_controls::managed` and `compose_for_provider` readiness | `bundle/tests.rs`, `capabilities/tests.rs`, `native_controls/tests.rs`, `tests/capability_launch.rs` |
| H2 | `native_controls::authored_server_conflict`, `launch_arguments` provenance, compile admission in `site_capabilities` | `native_controls/tests.rs`, `adapters/tests.rs`, `tests/capability_launch.rs` |
| H3 | `native_controls::compose_for_provider`, consumed by `claude_launch` and compile admission | `native_controls/tests.rs`, `adapters/tests.rs`, `tests/capability_launch.rs` |
| H4 | `bundle.rs::refuse_unpinned_active_input` from `parse_role` and `compose::own_table` | `bundle/tests.rs`, `bundle/compose_tests.rs` |
| M1 | `doctor.rs` OFF-first native lines | `doctor/capability_tests.rs` |
| M2 | `compose::read_layers`, `agents/load::read_json` strict parse | `bundle/compose_tests.rs`, `agents/tests.rs` |
| M3 | `Bundle::assemble` library lint, `Authority` consulted set | `bundle/agent_tests.rs` |
| M4 | split tests in `capabilities/tests.rs` | the change's `evidence.md` |
| macOS | `agents/tests::Tree::new` canonical root | `agents/tests.rs` |

## What this decision does not do

It does not change accepted 0065, add a grant to this repository's
realm, build the slice-two broker, lift the compile refusal of an `mcp`
dialect, or touch hands and boundaries (0043, 0046). It does not claim
live provider enforcement: a final argv is composition evidence, and the
limits 0065's slice recorded — resumed Codex OFF/ON, other CLI versions,
ambient MCP servers in an operator's Codex profile, Claude's live
controls, DSH and LaneTally inventories — remain unmeasured. It does not
clear the security hold; the council re-judges.

## Consequences

A realm whose adapter data predates 0065, or whose adapters directory
holds one broken file, no longer compiles an inline Codex or Claude seat;
the refusal names the file and the repair. Every bundle digest that
contains an inline or agent-backed model site is unaffected by the
provenance carriage, which rides the driver input and not the manifest.
