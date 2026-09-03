# 0040 — The model's hands are one tool, and the tool runs in an empty root

Status: accepted (operator ruled in chat, 2026-09-03)
Date: 2026-09-03

## Context

Decision 0016 rules that a restriction a provider cannot express is a
compile failure: the agent would run with more power than it declares.
On 2026-09-03 that rule refused the operator's own ruling for the review
gates — `fable@high → opus@xhigh → sol@xhigh` — at the third link,
because Codex restricts by sandbox class, not by tool name, and every
review agent declares a tool allow-list:

```
seat 'review:correctness': agent 'review-correctness' cannot be served by provider 'codex' on
model 'sol': the provider declares tool_permissions unsupported … the agent's restriction to
["cargo","git","python3",…] cannot be expressed and the agent would run with MORE power than it declares.
```

The rule was right and the restriction was the wrong shape. A tool
allow-list bounds what the model may *run*; what the operator wants
bounded is what running anything can *touch*. Those are different
statements, and the second is stronger: a model that can run any command
inside a box holding only the worktree has less power than a model that
can run seven named commands against the whole host.

The same day the operator asked what the verifier and shipper seats do,
and the honest answer was: run two commands and write one file. Both are
gate class, so decision 0021 requires the trusted tier of them, and the
`exec` adapter is untrusted — not because anyone measured a script and
found it unreliable, but because ruling 7 starts every newcomer there.
The tier measures trust to *judge*, a stochastic axis. A pinned script has
no stochastic axis. What it has is a blast radius, and that is the same
thing the review gates needed bounded.

One design answers both, and it is not new: the boundary goes around the
model's tool calls, not around the harness. The harness keeps its
credential and its network to the provider outside the box, as trusted
control-plane code. What the model asks to run goes through one tool,
and every call executes inside a bubblewrap namespace built from an
empty root. Nothing of the host is bound but the worktree and the
read-only toolchain; there is a private home and a private tmp per call;
pid, ipc, uts and (by default) net are unshared; every capability is
dropped; the environment is cleared. A tool name list has nothing left to
protect.

Alternatives weighed:

- **Put the whole harness in a container.** Rejected: the harness needs
  the provider credential and the network, so the box would hold the
  key beside the model's commands, and a command could read it.
- **Let codex express the list as its `workspace-write` sandbox class.**
  Rejected as a one-provider patch: dsh cannot express a list either,
  and the coarsening would live in an adapter rather than in a boundary.
- **Drop the tool grants from the review agents.** Rejected: it widens
  every provider's grant to remove one provider's refusal.
- **Rule `exec` trusted.** Rejected: it would be a tier granted without
  the evidence ruling 3 asks for, to a driver that can run anything the
  operator can. The box is the thing that makes a script safe to seat,
  not a tier.

## Rulings

1. **Brokkr serves one workspace tool.** `brokkr hands serve` speaks MCP
   over stdio and offers exactly one tool, `workspace`, taking a command
   and a timeout. Each call runs `bash -lc <command>` inside a namespace
   built from an empty root: the worktree bound read-write at its own
   path; the host toolchain (`/usr/bin`, `/usr/lib`, `/bin`, `/lib` and
   their siblings) read-only where present; a generated identity and a
   files-only resolver; a private `HOME` and `/tmp` per call; pid, ipc,
   uts and cgroup unshared; every capability dropped; the environment
   cleared to `PATH`, `HOME`, `TMPDIR` and a locale. Network is unshared
   unless the spec grants it. Output and time are bounded. The
   boundary is never simulated: no `bwrap`, no tool.

   **Enforcement binding:** `brokkr-protocol::hands`; unit tests over the
   namespace argv and the JSON-RPC loop, and an integration test on Linux
   that reads inside the box, fails to read outside it, writes the
   worktree, and is cut off by the timeout.

2. **A site declares `hands` instead of a tool list.** An agent or an
   inline seat, panel member or sequence step may declare
   `"hands": "workspace"` or `{"kind": "workspace", "network": bool,
   "binds": [{path, mode, mask}]}`. Binds add host paths to the box —
   a toolchain cache, read-write, with its credentials file masked behind
   `/dev/null`. When a site has hands, its tool allow-list is not
   consulted: the box expresses the restriction. Each adapter says how it
   replaces the harness's own tools with the one boxed tool — Claude
   Code disables its built-in tools and loads only this MCP server; Codex
   sets its own sandbox read-only and adds the server — or declares
   `hands` `unsupported` with the measured reason, exactly as
   `tool_permissions` may. The site vocabulary stays closed; `hands`
   joins it.

   **Enforcement binding:** `hands` in the agent and adapter loaders,
   `compose` in the resolver, and the site parsers; a seat that names
   an agent cannot amend the agent's hands; a site with hands and secret
   bindings is refused, because the box clears the environment.

3. **A boxed `exec` command may hold a gate.** A deterministic command
   has no stochastic axis for a trust tier to measure; its blast radius
   is the box. An `exec` site declaring `hands` passes the gate-tier
   check; the engine runs the whole dispatch through `brokkr hands exec`,
   which builds the same namespace with this binary bound read-only and
   passes the driver's stdio straight through. An unboxed `exec` gate
   stays refused as 0021 reads.

   **Enforcement binding:** the gate refusal in `enforce_model_policy`;
   `hands_command` in the engine; `brokkr hands exec`.

4. **The box is part of the bundle's identity.** The run manifest gains
   `hands`, a map from site to box spec, published as
   `run-manifest.v6` beside v5 and absent when no site declares hands,
   so a bundle that boxes nothing keeps its exact identity. A bind added
   to a reviewer's box moves the digest of every bundle that hires it.

   **Enforcement binding:** `contracts/run-manifest.v6.schema.json`,
   `manifest_for`, the frozen-contracts test.

5. **The review gates hire as the operator ruled.** `reviewer`,
   `review-correctness`, `review-security`, `review-security-speckit`
   and `review-spec-compliance` chain `fable@high → opus@xhigh →
   sol@xhigh`, keep their tool grants for the record, and declare hands
   with the Rust toolchain bound and its credentials masked. The bundles
   that hire them move, and both digest pins say why.

   **Enforcement binding:** the five agent files; `bundles/self`,
   `recipes/panel-review`, `recipes/sdd` and `recipes/sdd-paranoid`
   compile with sol as the third link; the witness and compose pins.

## Consequences

- What is *not* bounded by the box is egress: a boxed reviewer on codex
  still sends the worktree's contents to the provider, and what may be
  serialized toward a destination remains decision 0036's axis.
- The dsh adapter declares hands unsupported, honestly: its tool surface
  is replaced only through a profile plugin, and that plugin is the next
  slice. Until then dsh seats keep the tool-list path. The lanetally
  wrapper is unsupported until its pass-through is measured.
- A boxed seat cannot receive a secret binding; the two are refused
  together. A future ruling may thread a binding into the box as an
  environment entry named in the spec; this one does not.
- The verifier and shipper become boxed `exec` scripts in a following
  slice: the verifier as the checks it already lists, the shipper's
  ledger as a `brokkr` verb rendered from the journal. This decision
  makes both seatable; it does not yet seat them.
- `brokkr doctor` reports whether `bwrap` is present, since a bundle with
  hands cannot run without it.
