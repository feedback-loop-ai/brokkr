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

## Second-hold amendment — proposed, 2026-09-22

The second council reviewed `3b31c5de` after `f97b7e77` and held again.
Its checked ruling is `.forge/tasks/council-ruling-25d222e6.md`; the first
ruling remains `council-ruling-3c72a18a.md`. Adopt the subsequent specification
commit `49fda5e6` and the second design synthesis without rewriting that history.

Second H1/H2 exposed attached Codex config, Claude plugin loading and later
variadic admission values; H3 exposed a prompt value swallowing OFF; H4 exposed
dropped explicit include restrictions. H5/H6 exposed filesystem-resolved input
escapes and unenforced library charter pins at dispatch. M1 found subtraction
treated as admission; M2 found provider-blind doctor promises; M3 found
restriction removal proved only at intermediate composition. V1 measured a
fresh exact-coverage failure: **34897/35073 source lines, 5730/5744 branches,
3443/3453 logical functions**, exit 1. L1 rejects workflow instructions and
residual-floor claims embedded in panel prose.

Rulings 3–6 below replace their earlier insufficient mechanisms. Rulings
1/2/7/8/9 retain their first-repair meaning. The six HIGHs and three behavioral
MEDIUMs, MEDIUM validation failure and LOW integrity finding remain open.
H5/H6 preserve `spec_defect=true`; `has_security_residual=true` stays true.
This amendment is design, not implementation evidence or operator acceptance.

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

3. **A control the compiler accepts reaches an effective option position
   in the final argv, or compilation refuses its representation.** One closed
   parser and fallible composer in `brokkr-protocol::native_controls` own the
   supported commands of Codex, Claude, LaneTally and DSH. Their option grammar
   declares identities/aliases, split/equals/attached forms, typed values,
   arity/empty policy, variadic boundaries, repetition, subcommands and
   positionals. Unknown or unplaceable tokens and unclassified configuration
   effects refuse with source/provider/token position and cause. Concrete native
   switches remain adapter data; naming a flag cannot extend the grammar.

   Compile admission, final composition, duplicate/restriction checks,
   selectors, model/effort extraction and resume eligibility consume the same
   parsed option/value structure. Parse again with the same implementation at
   the process boundary; no new persisted AST or frozen schema is needed.
   Managed argv and restriction transports parse too; there is no non-list
   verbatim escape hatch. An ambiguous split prompt value such as
   `--append-system-prompt --disallowedTools hello` refuses; admitted inert or
   explicitly delimited text stays data through rendering. Each origin is
   complete before merging, so a value or terminator cannot consume another
   fragment's mandatory control.

   Explicit `--tools` is a restriction, distinct from additive selection and
   the engine hands baseline: absent, present empty and present nonempty remain
   distinct. Preserve the specified Read and empty OFF forms in successful
   unboxed cold and eligible-resume commands with independent WebFetch denial.
   Compose by narrowing, never union away a hard limit; unrepresentable
   conflicts refuse. Denial lists are subtraction, including `mcp__*`, and
   merge with native denial. A real hands/required-holding contradiction is
   diagnosed as a conflict, not a grant. Required holdings cannot be silently
   lost; optional losses retain whole-drop notices and effective native OFF.

   Codex consumes supported argv and refuses selection; unconsumed DSH native
   representations refuse. Claude/LaneTally consume only modeled representations
   with their own evidence limits. Final engine output/session/stdin controls
   participate in structural validation. Actual eligible resume and rejected
   rejoin cold replacement preserve the selected validated plan and existing
   eligibility. Complete compiled final-command literals and independent
   removals on cold and actual resume are required for accepted nonempty
   restrictions; intermediate Controls/composer output is insufficient.
   A synthetic grant fixture must use a supported production transport;
   fictitious flags or test-only grammar exceptions cannot close that proof.

4. **A recipe's authored command is data, not capability authority.**
   Preserve the first repair's `Candidate`/`SiteSpawn`/`launch_arguments`
   origin recorded where the hands fragment is appended, including its checked
   length-based split. Do not recover it by text matching or server name.
   Missing/mismatched provenance and parts that do not reassemble actual argv
   refuse; the by-hand interface cannot mint engine provenance.

   Admission judges parsed effects and every value/occurrence, independently
   of inventory omissions. All supported Codex `-c`/`--config` spellings,
   including attached `-cVALUE`, share config-key semantics; whole tables,
   descendants, quoted keys and repeats cannot hide server/native authority.
   Claude/LaneTally plugin loading, MCP/settings channels and include/allow
   lists cannot admit unauthorized tools, including later variadic MCP entries
   and wildcards. Subtractive lists do not enter that admission branch.
   Opaque configuration with unbounded authority refuses. A native holding
   does not authorize an arbitrary plugin/server; MCP dialect grants still
   refuse in slice one.

   Engine hands retain their own admission origin, grammar and strict
   configuration; an authored server named `brokkr` remains authored.
   DSH retains its one bound, contained, digest-matched route-only patch,
   including validation of content before staging. Unknown/residual forms
   refuse. LaneTally parses its supported wrapper/forwarded grammar and
   inherits no Claude native inventory or live evidence. Renaming an adapter
   cannot hide a recognized harness.

   *Migration:* unsupported authored forms must be removed or modeled with
   evidence; no permissive passthrough mode. Inline boxed seats cannot
   counterfeit hands: use the existing agent/adapter-owned fragment.

5. **Active identity follows the filesystem and the bytes consumed
   (corrects D7 and task 6.3; second H5/H6 keep `spec_defect=true`).**
   Canonicalize each original layer-owned reference before containment:
   never cancel `..` across a symlink first. A canonical target outside its
   declaring layer refuses, standalone and inherited, before active bytes are
   consumed. Retain both authored-path and canonical-target excluded-tree
   refusals. Missing, unreadable, unresolvable, non-regular or unpinned inputs
   are errors, never absence of drift. Every layer's policy and role uses this
   check; policy parses the same buffer whose digest matches its file map.

   Keep the `capabilities/` exclusion and consulted-definition pins; do not
   pin unconsulted definitions. Ordinary inputs use existing layer file maps.
   Independently owned library charters and dialect instructions retain their
   own contained roots/pins. Carry the selected charter's owner, original
   reference, compiled canonical target and existing expected digest; choose
   Layer or Library at compilation, not by a later longest-prefix guess.

   At common dispatch enforce a layer file pin or the selected agent's existing
   library `charter_digest`, even for an external library or missing layer-file
   entry. Re-resolve/check owner, target and digest; never bless changed bytes
   by recomputing the expected pin. Missing applicable pins, changed/retargeted
   sources and read/encoding failures refuse before provider work.
   Hash/read once and pass that verified text through engine-private input
   after authored merging; managed prompt rendering consumes the same buffer,
   without reopening the role path or converting failure to empty instructions.
   Explicit charterless exec is a separate state. Standalone/inherited,
   panel/sequence and primary/fallback sites keep their selected binding.

   *Migration:* relocate escaping/excluded layer inputs into pinned paths;
   recompile after an intentional library charter edit. No second inventory,
   snapshot store, public protocol or manifest version is needed. Recompile-only
   checking is not dispatch enforcement.

6. **Doctor says only what provider-aware composition establishes.**
   OFF-first ordering stays, but an Argv/Selection/Default variant is not a
   delivery result. Share the same fallible control lowering, grammar and
   composability assessment with compile/launch, using the actual harness,
   native inventory and selection mapping. Assess interacting OFF controls
   together; do not fabricate an incomplete plan to bypass known obligations.

   Distinguish composable declared OFF, measured unsupported, unmeasured and
   full grammar/provider/composition refusal. A Codex selection OFF reports the
   same cause as compilation. Both the native line and restriction/drop
   paragraph assess before describing grants: scoped, empty, unused, absent
   and subtracted grants cannot cure an uncomposable OFF. Without a resolved
   seat, state adapter-level scope rather than certify unassessed authored
   options or restrictions. This is static composition evidence, not a live
   provider denial measurement.

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
- *Six more scanner patterns or generic opaque config values:* another spelling
  can still carry the same authority without classification.
- *Treating explicit restrictions as additive or empty as absent:* this restores
  provider defaults; subtractive MCP patterns also cannot be labeled grants.
- *Lexical containment or recompile-only library pinning:* neither checks the
  file and bytes dispatch consumes.
- *Intermediate restriction removals or historical coverage as final proof:*
  they do not observe the commissioned boundary or candidate.

## Enforcement bindings

The first repair's floor/strict-decoding bindings remain in runtime
`bundle::load_pin_adapters`, `capabilities::native_plan` and protocol
`native_controls::managed`; strict source parsing remains in
`compose::read_layers` and `agents/load`; loaded-library lint and consulted
pins remain in `Bundle::assemble`/`Authority`. Their recorded first-repair
tests and optional-notice removals remain historical evidence, not closure of
the second findings.

| Second finding | Proposed enforcement | Required consuming proof |
| --- | --- | --- |
| H1/H2 | Protocol closed provider grammar and typed authored admission in shared composer | Native-controls/adapters and runtime compiled final-launch refusal tests; engine-hands/DSH positives |
| H3/H4/M1 | Parsed consumers, explicit constraints, subtractive merge and final serialization | Exact prompt refusal; whole cold/eligible-resume commands for Read, empty and MCP-deny forms, with independent removals |
| H5 | Fallible active-input resolution from `parse_role` and every `compose::own_table` | Bundle/compose four escape cases plus contained-target identity/consumption and independent role/policy removals |
| H6 | Existing library pin bound to selected site, common `spawn_site` verification, verified-text `render_prompt` | Engine dispatch without recompilation, standalone/inherited/external libraries and all serving paths; no changed prompt/provider work |
| M2 | Provider-aware assessment shared by runtime and both doctor paths | Whole compile/report-line equality for each grant shape and tag-only-removal failure |
| M3 | Accepted restriction through production compilation and `claude_command`/`claude_launch` | Full cold/actual-resume literals and independent final-delivery removals/restoration |
| V1 | Unchanged exact gate on final repaired source head | Fresh nonzero whole-workspace covered/total equality for lines, branches and logical functions; no stale pass |
| L1 | Artifact/evidence provenance audit | Rejected note instructions, unchanged true residual/specification-defect facts; no phase choice |

These second-hold bindings are design obligations, not claims that code or
proofs have landed. Tasks/evidence reopen the affected completion claims;
task 12.1 remains open. The first-head equal coverage figures are historical;
V1 stays failed until a fresh final-head exact pass exists.

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
the refusal names the file and the repair. The first repair's provenance
carriage stayed in private driver input rather than changing manifest identity. This amendment likewise reuses existing
identity records; any actual witness movement from repaired input handling
must be measured, explained and pinned from final compiles. No identity
stability or implementation completion is inferred from this design.
