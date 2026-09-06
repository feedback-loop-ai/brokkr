# 0046 — The boundary is named: a box has a boundary, `none` is not one of them, and the record says which stood

Status: accepted (operator ruled in chat, 2026-09-05)
Date: 2026-09-05

## Context

Decision 0043 put the model's hands in one tool and ran that tool in
an empty root built by bubblewrap, and said in its title that this is
a Linux boundary. Its ruling 7 refuses a boxed bundle at run start on a
machine without bubblewrap, and its consequences record the price:
macOS and Windows compile every shipped bundle and refuse to run the
ones that box a seat — the review offices, and since the verifier and
shipper became boxed exec, the quickstart itself.

On 2026-09-05 the operator ruled that every seat must be able to run on
every operating system, and first proposed a flag — a `--sandbox` that
is only on Linux. Asked to challenge it, the drafting agent did, and
the operator accepted the shape below in its place. The argument
against the flag, kept here because it is the reason for the rulings:

- **A flag hides the fact the digest law exists to record.** 0043
  ruling 4 made the box part of the bundle's identity. The same digest
  running boxed on one machine and unboxed on another is exactly the
  "different bundle identity wearing the pinned digest" that decision
  0021 ruling 5 refuses.
- **A flag unlaws the exec gates.** 0043 ruling 3 lets a deterministic
  command hold a gate only because "its blast radius is the box". With
  the box off, the exec adapter is untrusted at a gate again and the
  compiler refuses it. A switch cannot switch a trust rule.
- **The box mostly guards the operator's machine from the repository,
  not from the model.** Inside it `cargo test` still runs the tree's
  own build scripts; what the box removes is the network, the
  credentials and the git hooks path. Unboxed, a stranger's pull request
  runs its build scripts with the operator's keys in the environment.
- **A vouch that cannot say whether it was boxed is a weaker witness.**
  Decision 0038's gate would read the same for both.

What the tree already holds, cited, because the naming has to
reconcile it:

- **Three vocabularies for one idea.** `driver.confine {image, network,
  mounts}` is decision 0008's container confinement, still parsed by
  the compiler (`Confine` in `crates/brokkr-runtime/src/bundle.rs`) and
  still wrapped as `docker run` by the engine, and declared by no
  shipped bundle. `hands` is 0043's box: `{"kind": "workspace",
  "network": bool, "binds": [...]}`, where `kind` names the tool the
  model is handed — today only `workspace`. And each harness has its
  own sandbox that the adapter fragment addresses: codex's
  `--sandbox read-only`, claude's permission mode and its own boxed
  Bash tool.
- **`kind` is taken.** In `hands` it names the tool's shape, not the
  wall around it. Reusing it for the wall would make one word carry
  two axes, and the second axis is the one that varies per machine.

## The word — a survey, because the operator asked for one

The question is what to call the axis "what stands between the hands
and the machine", and what to call its values, including the value
that means nothing stands there. Six systems that choose an isolation
mechanism were read for their exact names on 2026-09-05:

| System | The axis | Its values name | Nothing stands there |
|---|---|---|---|
| Bazel (`--spawn_strategy`, docs "Sandboxing") | strategy | the mechanism: `linux-sandbox`, `darwin-sandbox`, `processwrapper-sandbox`; `sandboxed` picks per OS | `local` (also `standalone`) |
| Kubernetes `RuntimeClass` | `handler`, "the name of the corresponding CRI configuration" | the runtime: runc, gvisor, kata | — (a class always names one) |
| gVisor `runsc --platform` | platform | the mechanism: `kvm`, `systrap`, `ptrace` | — |
| Nix `sandbox` setting | one boolean-ish setting | `true`, `false`, `relaxed` | `false` (the macOS default) |
| Codex `sandbox_mode` | a policy, not a mechanism | `read-only`, `workspace-write` | `danger-full-access` |
| Nomad task drivers | driver | `exec` (chroot, cgroups), `docker` | `raw_exec`, "without any isolation … used with extreme care and disabled by default" |
| Claude Code sandboxed Bash | `sandbox` settings block, `enabled` | Seatbelt on macOS, bubblewrap on Linux, chosen by the tool | `unsandboxed`; `dangerouslyDisableSandbox` names the danger |

Three things the survey settles:

1. **The axis is named for what varies, and never for the OS.** Every
   system names the mechanism (`linux-sandbox`, `kvm`, `runc`,
   `seatbelt`), and the one that lets the tool choose per OS gives
   that a word of its own (`sandboxed`). None puts "linux" or "macos"
   in the axis.
2. **The absence is a first-class value with a plain name or a warning
   name.** Half the field names the danger (`danger-full-access`,
   `raw_exec`, `dangerouslyDisable…`); half names the fact (`local`,
   `false`, `unsandboxed`). Brokkr's law is legibility in the record,
   not fear in the data, so the data carries the plain word and every
   readout carries the adjective: *unboxed*.
3. **Policy and mechanism are different axes.** Codex's
   `sandbox_mode` says what is allowed; the platform decides how. In
   Brokkr what is allowed is already `hands` (`network`, `binds`); the
   new axis is only how.

The candidates, and why each loses or wins:

- `kind` — taken by the tool's shape (above). Loses.
- `sandbox` — the industry word, but in this tree it already means the
  harness's own thing (`--sandbox read-only` is a codex flag, the
  boxed Bash tool is claude's), and it blurs policy with mechanism.
  Loses; stays in prose about harnesses.
- `runtime`, `handler`, `platform`, `strategy`, `driver`, `profile` —
  each is a good word elsewhere and a taken word here: the runtime is
  a crate, the strategy is what triage rules (0041), the driver is the
  harness, the profile is dsh's plugin and Seatbelt's file. Lose.
- `confine` — 0008's verb for the docker path. A verb names an act,
  not the thing chosen, and the path it names has never shipped.
  Loses, and retires into a value below.
- **`boundary`** — 0043's own word: it is in the title and in the
  sentence that states the design ("the boundary goes around the
  model's hands"; "the boundary is never simulated"). It names the
  thing chosen, not the act; `namespace` and `seatbelt` are boundaries
  the way `kvm` is a platform; and its absence reads as a fact rather
  than a contradiction. Wins.

## Ruling — 2026-09-05, operator: accepted as proposed

Accepted in chat the day it was proposed ("I like the design,
approved"), without amendment: `boundary` is the word, `open` is the
bare value, and the enactment runs in ruling 6's order. The three
questions the consequences leave unruled stay open.

## Rulings

1. **The boundary is a named axis, and `kind` stays the tool's.** A
   site's `hands` gains `boundary`, one of a closed vocabulary that
   names the mechanism and never the OS:

   | Boundary | What stands there | Where |
   |---|---|---|
   | `namespace` | 0043's empty-root user namespace built by bubblewrap | Linux, WSL2 |
   | `seatbelt` | the same box built by the system sandbox (`sandbox-exec`) | macOS |
   | `container` | a pinned image with the worktree mounted, network off unless granted — 0008's `confine`, re-homed | any host with a container engine |
   | `harness` | nothing of Brokkr's; the harness's own sandbox as its adapter fragment addresses it | any |
   | `open` | nothing at all | any |

   `boundary` is declared by the realm (`forge.realms/v4`, one optional
   field per realm beside `house` and `dialect`), because the machine a
   realm runs on is the realm's fact; a bundle never names it. Absent,
   it reads `namespace`, which is what every bundle meant until today.
   The manifest pins the resolved boundary per site as `run-manifest.v9`
   beside v8, so a run under `seatbelt` and a run under `namespace` are
   two identities, as 0043 ruling 4 requires; `kind` keeps naming the
   tool. The enumeration is frozen the way a contract is: a new
   boundary is a new decision.

   **Enforcement binding:** `contracts/realms.v4.schema.json` and
   `contracts/run-manifest.v9.schema.json` with the frozen-contracts
   test; the site parser in `crates/brokkr-runtime/src/bundle.rs`
   refuses `boundary` inside a bundle, naming the realm as its home;
   `brokkr compile` prints the boundary under each hands site.

2. **A boundary the machine cannot build refuses, and `doctor` says
   which it can.** 0043 ruling 7 generalised: `run`, `resume` and
   `rerun` refuse at start when the realm's boundary is `namespace`
   without a 0.10+ bubblewrap, `seatbelt` without `sandbox-exec`, or
   `container` without an engine, naming the seats that need it.
   `brokkr doctor` prints the boundaries this machine offers, one line,
   and `harness` and `open` are always offered.

   **Enforcement binding:** `refuse_unboxable` grows the vocabulary;
   the doctor line; a test per boundary that refuses on an empty PATH.

3. **The record says which boundary stood.** Every `effect/started`
   carries `boundary` beside its provenance, the seat record's
   finishing checkpoint carries it as `seat-record.v3`, and every
   readout that shows a seat's model shows its boundary in the same
   row. A run in which any gate site stood under `harness` or `open`
   is rendered *unboxed* wherever the run is summarised — the TUI, the
   web console, `brokkr seats`, and the delivery gate's own check
   summary on the pull request (decision 0038). The word in the data
   is the plain one; the word on the screen is the adjective.

   **Enforcement binding:** the engine's effect start path;
   `contracts/seat-record.v3.schema.json`; `brokkr-view`'s derivation;
   the gate script that prints the check summary; `roster.rs`-style
   pins that every readout names the boundary where it names the model.

4. **Which boundaries may hold a gate.** `namespace`, `seatbelt` and
   `container` hold any gate, as 0043 ruling 3 reads. `harness` holds a
   model gate only when the adapter declares a read-only fragment for
   it (`hands.harness` in the adapter file: codex's
   `--sandbox read-only`, claude's read-only permission mode; dsh and
   lanetally declare none and are refused at a `harness` gate as they
   are at a boxed one today). `open` never holds a model gate. An exec
   gate under `harness` or `open` is refused as 0021 reads, with one
   narrowed reading: a script that is the bundle's own pinned bytes,
   run with the environment cleared and the network off where the
   platform can say so, may hold the verify and ship gates, because
   what a pinned script does is the digest's fact and not a trust
   tier's — and the record marks the run unboxed all the same.

   **Enforcement binding:** `enforce_model_policy`; the adapter files
   gain `hands.harness`; `model_policy_tests.rs` — a `harness` gate on
   codex admitted, on dsh refused, an `open` model gate refused, an
   `open` exec gate admitted only for a bundle-pinned script and
   refused for a `{brokkr}`-external command.

5. **`driver.confine` retires into the `container` boundary.** 0008's
   field has no shipped user; its image, network and mounts become
   the `container` boundary's declaration in the realm, measured in a
   slice of its own before any shipped realm names it. Until then the
   field is refused by the compiler with a message naming this
   decision, so a bundle that still carries it fails loudly rather than
   running a wrapper nobody has exercised.

   **Enforcement binding:** the parser refusal and its test; the
   engine's `docker run` wrapper deleted with the field; the guide's
   `driver.confine` row rewritten to point here.

6. **Enactment, in order.** (i) The word and the pin: realms v4,
   manifest v9, seat-record v3, `harness` on codex and claude, the
   readouts, the gate summary — after this slice a macOS operator runs
   every shipped bundle, the review offices under their harness's own
   sandbox, and every readout says so. (ii) `seatbelt`, measured on a
   Mac, as the boundary macOS operators are expected to declare.
   (iii) `container`, absorbing 0008. Windows declares `harness` or
   `open`; WSL2 declares `namespace`.

## Consequences

- **What moves.** Every pinned bundle, once, because the manifest
  gains a key; the realms map of this repository, which names
  `namespace`; the guides' hands sections and the quickstart's platform
  line, which stops saying Linux only and starts saying what each
  boundary is.
- **What the operator sees.** A pull request judged on a Mac before
  slice (ii) lands reads *unboxed* in its check summary. That is the
  point: the fact is on the page instead of in a flag nobody reads.
- **What it does not do.** Nothing here changes what `hands` allows;
  `network` and `binds` are the policy and stay where they are. Nothing
  here promotes or demotes a driver's tier.
- **Deliberately unruled.** Whether a vouch made under `harness` may
  merge without the operator's `by-hand` label, which is 0038's to
  amend; how the `container` image is pinned, which slice (iii) must
  measure; whether the `seatbelt` box can mask the git hooks path the
  way the namespace box does (0043 ruling 6), which decides whether it
  is a full peer of `namespace` or a `harness`-grade boundary.

## Addendum — 2026-09-06, operator ruled: an unbuilt boundary refuses at start

The first enactment fire's clarifier (run
`decision-0046-enactment-slice-i--165601b4`) found ruling 2 and ruling 6
in contradiction for slice (i): doctor would offer `seatbelt` wherever
`sandbox-exec` exists and `container` wherever a container engine is on
`PATH`, the policy would admit gates under them, the manifest would pin
them — and the engine would reach a site with no composition rule,
because those boundaries are slices (ii) and (iii).

The operator ruled in chat ("refuse seatbelt and container at start
until their slices land"). Ruling 2 reads, for every boundary whose
slice has not landed: `run`, `resume` and `rerun` refuse at start,
naming the boundary and the slice that builds it, whatever the machine
offers; `doctor` lists such a boundary as *not yet built*, never as
offered. A realm may declare it, the map is valid, and the refusal is
the run's first line. When slice (ii) lands, `seatbelt` moves from the
refused set to the built set by that slice's own change to
`refuse_unboxable` and its test; likewise `container` at (iii). No
branch exists for a boundary that cannot be composed, so the exact
coverage gate has nothing unreachable to demand.

**Enforcement binding:** `refuse_unboxable` and the doctor line; a test
per unbuilt boundary that a realm declaring it is refused at start on a
machine that has its tool, naming the slice.

## Erratum

Rulings 3 and 6 name `seat-record.v3` for the boundary field; v3 already exists (landed by #202 under decision 0034 rulings 6 and 7, the dialect state), so the field lands as `seat-record.v4`, additive on v3, and nothing else is renumbered.
