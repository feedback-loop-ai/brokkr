# boundary-availability Specification

## Purpose
A boundary the machine cannot build refuses at start, a boundary this
engine does not yet build refuses the same way, and `doctor` says which
boundaries a run can start under here (decision 0046 ruling 2,
generalising decision 0043 ruling 7; decision 0046 ruling 6 for the two
boundaries later slices build).

## Requirements

### Requirement: A boundary the machine cannot build refuses at start
`refuse_unboxable` SHALL judge the compiled bundle's boundary against
the search path it is given, and only when the bundle declares at least
one hands site: `namespace` needs a `bwrap` on that path and, for a spec
with overlay binds, one reporting 0.10 or newer, exactly as today;
`seatbelt` needs `sandbox-exec` on that path; `container` needs `docker`
or `podman` on that path; `harness` and `open` are always offered. The
refusal SHALL name the boundary, what it needs and what was found, the
seats that declare hands, and decision 0046 ruling 2; it SHALL fire in
`run`, `resume` and `rerun` before any journal row is written or a seat
spawned (`resume` has already opened the journal to read the run's
pinned manifest, and writes nothing to it); and the boundary is never
simulated — nothing is degraded to run anyway (decision 0046 ruling 2;
decision 0043 ruling 7).

#### Scenario: namespace on an empty PATH refuses
- **WHEN** a bundle with a boxed seat `work` compiles under `namespace` and `refuse_unboxable` is given an empty search path
- **THEN** it refuses naming `namespace`, bubblewrap, the seat `work`, and decision 0046 ruling 2

#### Scenario: seatbelt on an empty PATH refuses
- **WHEN** the same bundle compiles under `seatbelt` and the search path is empty
- **THEN** it refuses naming `seatbelt`, `sandbox-exec`, and the seat

#### Scenario: container on an empty PATH refuses
- **WHEN** the same bundle compiles under `container` and the search path is empty
- **THEN** it refuses naming `container`, `docker` or `podman`, and the seat

#### Scenario: harness and open pass on an empty PATH
- **WHEN** the same bundle compiles under `harness`, and again under `open`, and the search path is empty
- **THEN** both pass

#### Scenario: namespace passes with its tool
- **WHEN** the search path holds a `bwrap` and the bundle binds no overlay
- **THEN** `namespace` passes, exactly as today

#### Scenario: An overlay bind still asks 0.10 of bubblewrap
- **WHEN** a `namespace` bundle binds an overlay and the `bwrap` found reports an older version
- **THEN** it refuses naming the seat and `0.10 or newer`, as today

#### Scenario: A plain bundle passes everywhere
- **WHEN** a bundle with no hands site compiles under any boundary and the search path is empty
- **THEN** `refuse_unboxable` passes

#### Scenario: The three verbs refuse before the journal
- **WHEN** `brokkr run`, `brokkr resume` or `brokkr rerun` is given a boxed bundle whose boundary this machine cannot build
- **THEN** the command fails with the refusal and no journal row is written and no seat is spawned

### Requirement: A boundary this engine does not build refuses at start until its slice lands
This slice builds `namespace`, `harness` and `open`. `seatbelt` and
`container` are named by decision 0046 ruling 1, pinned by the manifest
and admitted by the gate law, and are built by ruling 6's slices (ii)
and (iii): no composition for either exists in this engine, and the
boundary is never simulated. `refuse_unboxable` SHALL therefore refuse
a bundle with hands sites compiled under `seatbelt` or `container` on
every machine, after the tool check of the requirement above, with a
message naming the boundary, the tool and where it was found, the seats
that declare hands, the slice of decision 0046 ruling 6 that builds the
boundary, and `harness` as the boundary a realm may declare today. The
engine SHALL refuse the same bundle at its own entry — `run`, `resume`
and `rerun` alike — before `run/started` or any other row is written,
so a library caller reaches no composition that does not exist; the
engine's composition SHALL be written over the three boundaries it
builds and never carry an arm for a word it cannot. When slice (ii) or
(iii) lands, its refusal is deleted and the tool check becomes the whole
gate; nothing else moves (decision 0046 rulings 2 and 6; decision 0043
ruling 1: the boundary is never simulated).

#### Scenario: seatbelt with sandbox-exec present still refuses
- **WHEN** a bundle with a boxed seat `work` compiles under `seatbelt` and the search path holds a `sandbox-exec`
- **THEN** `refuse_unboxable` refuses naming `seatbelt`, the `sandbox-exec` it found, the seat `work`, slice (ii) of decision 0046 ruling 6, and `harness` as the boundary open today

#### Scenario: container with an engine present still refuses
- **WHEN** the same bundle compiles under `container` and the search path holds only a `docker`, and again only a `podman`
- **THEN** each refuses naming `container`, the engine it found, the seat, and slice (iii) of decision 0046 ruling 6

#### Scenario: The engine's own fence
- **WHEN** a library caller starts, resumes or reruns a run over a bundle with hands sites compiled under `seatbelt` or `container`
- **THEN** the engine refuses naming the boundary and its slice, and no `run/started` or other row is written

#### Scenario: A plain bundle under seatbelt starts
- **WHEN** a bundle with no hands site compiles under `seatbelt` and a run starts
- **THEN** nothing refuses, because no box is asked for

### Requirement: doctor names the boundaries this machine offers
`brokkr doctor` SHALL print exactly one line labelled `boundaries` that
names the boundaries a run can start under on this machine with this
engine — `namespace` with the bubblewrap version found, `harness` and
`open` always — and, for each boundary it does not offer, why:
`namespace` without `bwrap` names the missing tool; `seatbelt` and
`container` name the slice of decision 0046 ruling 6 that builds them
and, as a readiness fact, whether `sandbox-exec`, or `docker` or
`podman`, is on PATH. When `--bundle` is given, doctor SHALL compile the
bundle in the discovered realm and the existing `hands` line SHALL
judge the bundle's hands sites against the realm's boundary rather than
against bubblewrap alone: healthy under `namespace` with bubblewrap and
under `harness` or `open` always, a warning under `namespace` without
bubblewrap, and a warning under `seatbelt` or `container` naming the
unbuilt slice (decision 0046 ruling 2; decision 0043's consequence that
doctor reports what a boxed bundle needs).

#### Scenario: A Linux box with bubblewrap and docker
- **WHEN** `bwrap` reporting `0.11.0` and `docker` are on PATH and `sandbox-exec` is not
- **THEN** the `boundaries` line offers `namespace (bubblewrap 0.11.0)`, `harness` and `open`; says `seatbelt` is built by slice (ii) and `sandbox-exec` is not on PATH; and says `container` is built by slice (iii) and `docker` was found

#### Scenario: An empty PATH
- **WHEN** no tool is on PATH
- **THEN** the `boundaries` line offers `harness` and `open`, names `bwrap` as what `namespace` is missing, and names the missing tool and the building slice for `seatbelt` and `container`

#### Scenario: A bundle under harness needs no bubblewrap
- **WHEN** `doctor --bundle` compiles a boxed bundle in a realm declaring `harness` on a machine without `bwrap`
- **THEN** the `hands` line says the seats declare hands and can run under `harness`, and the report stays healthy on that account

#### Scenario: A bundle under seatbelt is warned about the slice
- **WHEN** `doctor --bundle` compiles a boxed bundle in a realm declaring `seatbelt`, with or without `sandbox-exec` on PATH
- **THEN** the `hands` line warns that the seats declare hands and that `seatbelt` is built by slice (ii) of decision 0046, not by this engine

### Requirement: init's warning speaks the vocabulary
`brokkr init`'s warning when bubblewrap is absent SHALL say that the
scaffolded seats declare hands and run under the realm's boundary, that
`namespace` — the default — needs bubblewrap on PATH, and that a realm
may declare `harness` instead (decision 0046), rather than that the
shipped gates require Linux.

#### Scenario: init without bubblewrap
- **WHEN** `brokkr init` runs on a machine without `bwrap` on PATH
- **THEN** its warning names the scaffolded seats, `namespace`, bubblewrap, and `harness` as the road a realm may declare, and cites decision 0046
