# boundary-availability

## Purpose

A boundary the machine cannot build refuses at start, and `doctor` says
which it can (decision 0046 ruling 2, generalising decision 0043
ruling 7).

## ADDED Requirements

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
pinned manifest, and writes nothing to it);
and the boundary is never simulated — nothing is degraded to run anyway
(decision 0046 ruling 2; decision 0043 ruling 7).

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

#### Scenario: Each boxed boundary passes with its tool
- **WHEN** the search path holds only a `bwrap`, only a `sandbox-exec`, only a `docker`, or only a `podman`
- **THEN** `namespace`, `seatbelt`, `container` and `container` respectively pass, and each of the other boxed boundaries still refuses

#### Scenario: An overlay bind still asks 0.10 of bubblewrap
- **WHEN** a `namespace` bundle binds an overlay and the `bwrap` found reports an older version
- **THEN** it refuses naming the seat and `0.10 or newer`, as today

#### Scenario: A plain bundle passes everywhere
- **WHEN** a bundle with no hands site compiles under any boundary and the search path is empty
- **THEN** `refuse_unboxable` passes

#### Scenario: The three verbs refuse before the journal
- **WHEN** `brokkr run`, `brokkr resume` or `brokkr rerun` is given a boxed bundle whose boundary this machine cannot build
- **THEN** the command fails with the refusal and no journal row is written and no seat is spawned

### Requirement: doctor names the boundaries this machine offers
`brokkr doctor` SHALL print exactly one line labelled `boundaries` that
names the boundaries this machine offers — `namespace` with the
bubblewrap version found, `seatbelt` when `sandbox-exec` is on PATH,
`container` with the engine found, `harness` and `open` always — and,
for each boxed boundary it does not offer, what is missing. When
`--bundle` is given, doctor SHALL compile the bundle in the discovered
realm and the existing `hands` line SHALL judge the bundle's hands sites
against the realm's boundary rather than against bubblewrap alone
(decision 0046 ruling 2; decision 0043's consequence that doctor reports
what a boxed bundle needs).

#### Scenario: A Linux box with bubblewrap and docker
- **WHEN** `bwrap` reporting `0.11.0` and `docker` are on PATH and `sandbox-exec` is not
- **THEN** the `boundaries` line names `namespace (bubblewrap 0.11.0)`, `container (docker)`, `harness` and `open` as offered, and `seatbelt` as not offered because `sandbox-exec` is not on PATH

#### Scenario: An empty PATH
- **WHEN** no tool is on PATH
- **THEN** the `boundaries` line offers `harness` and `open` and names the missing tool for each of the three boxed boundaries

#### Scenario: A bundle under harness needs no bubblewrap
- **WHEN** `doctor --bundle` compiles a boxed bundle in a realm declaring `harness` on a machine without `bwrap`
- **THEN** the `hands` line says the seats declare hands and can run under `harness`, and the report stays healthy on that account

### Requirement: init's warning speaks the vocabulary
`brokkr init`'s warning when bubblewrap is absent SHALL say that the
scaffolded seats declare hands and run under the realm's boundary, that
`namespace` — the default — needs bubblewrap on PATH, and that a realm
may declare `harness` instead (decision 0046), rather than that the
shipped gates require Linux.

#### Scenario: init without bubblewrap
- **WHEN** `brokkr init` runs on a machine without `bwrap` on PATH
- **THEN** its warning names the scaffolded seats, `namespace`, bubblewrap, and `harness` as the road a realm may declare, and cites decision 0046
