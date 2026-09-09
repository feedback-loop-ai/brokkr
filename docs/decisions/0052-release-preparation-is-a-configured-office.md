# 0052 — Release preparation is a configured office

Status: proposed
Date: 2026-09-08

## Context

While commissioning v0.10.0, the operator requested a Brokkr release agent:
prepare the release, derive its description from commits, reconcile the
documentation, README and architecture, and update release references in the
organization README. The operator also required extensible configuration and
separated the common process and prompt from repository-specific instructions.

The existing agent library, realm house pin and recipe composition already
provide these boundaries. A new engine configuration parser or release-specific
transition would duplicate them. Organization profiles need a separate
publication step: a run's local journal cannot prove another repository changed.

## Rulings

1. **The release manager is a work office.** Its portable charter describes the
   preparation process, commit-derived claims, documentation reconciliation and
   handoff. Repository names, version sources, commands, documentation and profile
   paths live in the realm's house file under Release configuration. Models,
   effort and tool grants remain agent-definition data.
2. **Configuration extends through data.** Targets and checks are lists; named
   extensions supply additional instructions and required evidence. The existing
   house digest covers the inline configuration. This is prompt configuration,
   not an executable schema: additional deterministic checks require the normal
   recipe verifier override. An unsupported required item blocks preparation.
3. **The hire is opus then astra, both at medium effort.** The operator
   selected this chain during preparation. Both use the workspace tool; a native
   tool allow-list cannot be expressed by Codex. Workspace networking is enabled
   to read release and organization-profile evidence, with repository-specific
   toolchain, resolver and CA binds in the agent definition. Registry credentials
   are masked. This is a network-enabled work office, not a URL allow-list or a
   new native fetch grant; decision 0044's native fetch roster stays unchanged.
4. **Preparation retains independent review.** The release recipe inherits
   `fast`'s policy and boxed exec verify and ship seats, and seats the library
   reviewer separately from the release manager. Findings return through the
   existing bounded policy. No release feature changes the engine or evaluator.
5. **External updates carry their own evidence.** Prepare organization-profile
   patches against recorded base commits and include them in the reviewable
   handoff. Apply them only after the release is published, within the operator's
   authorization. A local run's completion does not claim publication, channel
   availability or application to another repository.

## Enforcement bindings

- `agents/release-manager.json` and its charter separate grants from the office.
- `docs/house-rules.md` is the configured example and `realms.json` names it;
  the existing realm loader pins its content in the manifest.
- `recipes/release/configuration.example.md` documents the extensible shape.
- `crates/brokkr-runtime/tests/release_shape.rs` compares the inherited delivery
  rules and deterministic gates and checks the configured documentation paths.
- `crates/brokkr-runtime/tests/release_shape.rs` also pins the medium-effort
  chain and the networked workspace on the work seat. Existing roster tests
  retain the researcher-only native fetch grant and the boxed Codex rule.
- Claim accuracy, extension completion and live external state require review
  and operator verification; the configuration does not pretend to prove them.

## Consequences

Another repository reuses the charter, configures its house and agent grants,
and replaces the verifier for its stack. It does not fork the release prompt.
The shipped recipe is a Rust delivery specialization, not a universal validator.
Tagging, publishing and applying cross-repository changes remain explicit actions.
