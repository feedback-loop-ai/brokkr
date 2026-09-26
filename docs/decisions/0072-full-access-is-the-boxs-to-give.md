# 0072 — Full access is the box's to give: a harness drops its own sandbox only when the whole seat stands in an engine-owned boundary, and its credentials stand outside it

Status: proposed
Date: 2026-09-25

## Context

On 2026-09-25 the operator ruled "narrow" for decision 0065's inline Codex
sandbox classes. Work seats run `workspace-write`, gates run `read-only`,
and `danger-full-access` is admitted nowhere. Rebuild unit 7 enacted it.
`recipes/standby`'s Codex smith and `recipes/wager-harness`'s Codex
implementer had both run under `danger-full-access`. `standby`'s README
gave the reason: "a smith that cannot write is not a smith".

The operator then asked the question this decision answers. Isn't
sandboxing meant to give the model full permission *inside* the sandbox?
It is. The ruling refuses `danger-full-access` because, at every site
Brokkr can build today, nothing of Brokkr's stands around the harness
process. Two facts make that so.

1. **Decision 0043 boxes the tool, not the harness.** `brokkr hands serve`
   runs each `workspace` call in an empty-root namespace: the worktree
   read-write, the toolchain read-only, a private `HOME` and `/tmp`, the
   environment cleared and the network unshared. The harness process
   runs on the host. For Codex, 0043 ruling 2 sets the harness's own
   sandbox read-only and adds the server. The model can therefore write
   only through the box. `danger-full-access` at a hands site would give
   the model a second shell with the operator's full reach, beside the
   box it was meant to use.
2. **The harness holds a credential and needs the network.** It
   authenticates to its provider with login state or a key, and it must
   reach the provider's endpoint. Suppose Brokkr moved the whole harness
   process into the box and dropped the harness's own sandbox. The
   model's shell would then share a box with the credential, and with a
   network path out. A box that holds the key and the way out confines
   nothing that matters.

Decision 0046 names what can stand around a site: `namespace`,
`seatbelt`, `container`, `harness` and `open`. None of them, as enacted,
puts the harness process itself inside. `container` (0008's `confine`,
re-homed) comes closest, and it is unbuilt. At `harness`, the harness's
sandbox is the boundary, so dropping it leaves `open`.

The narrow ruling is therefore the correct floor for slice one. It is
not the end state the operator described. This decision names the end
state and what it takes.

## Rulings

1. **Full access is a class only a seat box may emit.** Codex's
   `danger-full-access`, and any harness mode that removes the harness's
   own confinement, is emitted only by the engine and only at a site
   whose harness process stands inside an engine-owned boundary: a
   *seat box*. Claude Code's `bypassPermissions` is such a mode, and a
   future harness declares its equivalent in adapter data. Everywhere
   else the 2026-09-25 narrow ruling stands unchanged: `workspace-write`
   at work, `read-only` at gates, full access nowhere. It is refused as
   decision 0065 refuses every uncompiled capability: never
   reconciled, never clamped.

   **Enforcement binding:** 0065's typed `tools.sandbox` lowering admits
   the full-access class only when the site's resolved hands kind is
   `seat` (ruling 2). The launch judgment of rebuild units 5d–5d-fix-c2
   refuses the class in the final composed command at any other site.
   Regressions cover a hands site, a `harness` site and an `open` site.

2. **A seat box is a hands kind, and the realm's boundary builds it.** A
   site may declare `"hands": {"kind": "seat", "binds": [...]}`. The
   harness process is spawned inside the realm's boundary, and so is
   every command it runs. The contents are 0043's box: the worktree
   read-write at its own path, the toolchain read-only, binds in 0043's
   three modes, a private `HOME` and `/tmp`, and the environment cleared.
   Only `namespace`, `seatbelt` and `container` can build a seat box.
   Under `harness` or `open` it is refused at compile and at start,
   naming the realm (0046 rulings 1–2). The first slice admits a seat box
   at work sites only. A gate needs no full access, so gates keep
   0043/0046 as they are.

   **Enforcement binding:** the hands parser and the adapter loaders
   (`hands.seat` in adapter data, or `unsupported` with a measured
   reason); `refuse_unboxable`; `run-manifest` pins `kind: seat` beside
   the boundary, so a seat-boxed run is its own identity (0043 ruling
   4).

3. **No credential enters a seat box.** No provider key, login file or
   token is bound, copied or passed into the box's environment. The
   harness reaches its provider only through an engine-owned egress
   relay that stands outside the box. The relay is the box's only
   network path; the network is otherwise unshared. It forwards to the
   hosts the adapter declares for that provider, and it attaches the
   credential on the way out. A harness that cannot authenticate this
   way records `hands.seat: {"unsupported": "<measured reason>"}` and
   stays at the narrow ceiling. The relay is never simulated. If the
   relay cannot stand, the seat refuses at start; it never falls back to
   a credential inside the box.

   **Enforcement binding:** the seat-box spawn path. An integration test
   on Linux shows each of these inside the box: the harness's login
   state is absent; an arbitrary host is unreachable; the provider is
   reachable through the relay alone; and a command with full access
   still cannot read the relay's credential.

4. **Wider network is a capability, not a side effect.** Anything beyond
   the relay is a decision 0065 capability the realm grants: a package
   registry, web fetch, a second provider. Full access inside the box
   never widens the network, because the box, not the harness, owns
   the network.

5. **The record says a seat stood in a seat box.** The seat record, the
   run manifest and every readout that names a seat's boundary name
   `seat` as its hands kind. A run is summarised as boxed only if every
   gate site was boxed, as 0046 ruling 3 already reads. Full access is
   shown as the class it is, never hidden behind "boxed".

6. **Enactment, measured first.** Each harness is measured before any
   shipped recipe names a seat box. The measurements are:
   - whether it runs with its home directory private and its login state
     absent;
   - whether it accepts an egress relay (a configurable endpoint or
     proxy) for authentication;
   - what its own tools do under full access inside the box;
   - how resume behaves.

   The slices, in order:
   - (i) `namespace` and Codex, with `recipes/standby` and
     `recipes/wager-harness` restored to full access inside a seat box;
   - (ii) Claude Code;
   - (iii) `seatbelt`;
   - (iv) `container`, together with 0046 ruling 5's retirement of
     `confine`.

   Until a harness's slice is measured and merged, that harness stays at
   the narrow ceiling.

## Consequences

- The operator's model becomes the shipped one where it is safe: the
  box confines, and the harness inside it may do anything the box
  allows.
- A seat box is a heavier boundary than a hands box. It adds a relay per
  seat, a per-harness measurement, and a new manifest identity. That is
  why it is a kind a site opts into, and why hands boxes remain the
  default for seats that only need the workspace tool.
- `recipes/standby`'s smith runs under `workspace-write` until slice (i)
  lands. Anything it needs outside the worktree, or from the network (a
  new crate), fails there today. That is recorded here as the known
  cost of the floor.
- Decision 0065 slice one ships unchanged. This decision starts after it
  merges and reopens none of its rulings.

## Alternatives weighed

- **Admit `danger-full-access` at hands sites.** Rejected: the harness
  process is on the host, so the model gains a host shell beside the box
  (fact 1).
- **Admit it under the `harness` boundary.** Rejected: there the
  harness's own sandbox *is* the boundary, so dropping it is `open`.
- **Put the whole harness in the box with its credential.** Rejected:
  the model's full-access shell shares the box with the key and the way
  out (fact 2).
- **Wait for `container` alone.** Rejected as the only path. A container
  meets the rulings, but it is the least built boundary. A Linux
  namespace seat box reuses 0043's empty root, which is measured and
  shipped. `container` remains slice (iv) under the same rulings.
- **Keep full access nowhere, forever.** Rejected: it gives up the
  simplest confinement model, which is one box with everything inside.
  It also keeps smiths from work that needs full access, even where a
  box could safely give it.

## Open questions for the operator

- Is the relay's host list per provider adapter data, as proposed, or a
  realm grant under 0065?
- Should slice (i) restore `standby` and `wager-harness` to full access,
  or first prove the seat box on a new recipe and leave those two
  narrowed?
