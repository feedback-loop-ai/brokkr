## MODIFIED Requirements

### Requirement: A run's boundary is the realm's, resolved at compile
Bundle compilation SHALL take the boundary of the realm it compiles in —
`namespace` when it compiles in no realm — and SHALL expose it on the
compiled bundle beside the manifest. `brokkr run` and `brokkr compile`
SHALL compile against the boundary of the operated repository's realm
in the discovered or named map; `brokkr resume` SHALL compile against
the realm embedded in the run's pinned world and never against the
workspace's map as it stands today, as it already does for the dialect;
`brokkr rerun` SHALL discover the workspace map and compile against
the operated repository's realm exactly as `brokkr run` does, so a
rerun stands under the realm's word and its manifest carries the realms
pin (design DD6). Compilation consults no machine and no engine
capability: a realm declaring `seatbelt` or `container` compiles and
pins the word, and any refusal that stops a run comes at start and is
boundary-availability's: Seatbelt needs its policy/peer activation and
implemented macOS boundary, retaining the unbuilt slice (ii) refusal until
then, while container remains unbuilt until slice (iii) (decision 0046
ruling 1; the resume rule of decision 0042 ruling 1's enactment).

#### Scenario: Run resolves the operated realm's boundary
- **WHEN** a `forge.realms/v4` map declares the operated repository's realm with `"boundary": "harness"` and `brokkr run` compiles a bundle in it
- **THEN** the compiled bundle's boundary is `harness` and its manifest's `boundary` map says so for every hands site

#### Scenario: Resume reads the pinned world, not the file
- **GIVEN** a run started under a map that declared no boundary
- **WHEN** the workspace's `realms.json` now declares `harness` and `brokkr resume` compiles the run's bundle
- **THEN** the bundle compiles under `namespace`, the pinned manifest matches, and the resume proceeds under the boundary the run was started with

#### Scenario: Rerun resolves the discovered realm
- **WHEN** the workspace's `realms.json` declares the operated repository's realm with `"boundary": "harness"` under `forge.realms/v4` and `brokkr rerun` re-runs a past run's feature
- **THEN** the new run's bundle compiles under `harness`, its manifest carries the realms pin as a `run`'s does, and `refuse_unboxable` judges `harness`

#### Scenario: Compile without a map is namespace
- **WHEN** a bundle compiles through `Bundle::compile_with` with no realm context
- **THEN** its boundary is `namespace` and its manifest is what this tree's witness table pins

#### Scenario: A seatbelt realm compiles and pins the word
- **WHEN** a `forge.realms/v4` map declares the operated repository's realm with `"boundary": "seatbelt"` and `brokkr compile` runs over a boxed bundle on a machine without `sandbox-exec`
- **THEN** the bundle compiles and its manifest's `boundary` map says `seatbelt` for every hands site; compilation consults no host capability; starting that bundle retains the slice (ii) unbuilt refusal while activation prerequisites are unresolved; after activation it refuses a wrong host or missing/unusable trusted system launcher with that host/tool cause

#### Scenario: Seatbelt resume and rerun preserve their different realm sources
- **GIVEN** a run pinned under `seatbelt` and a current workspace realm changed to `harness`
- **WHEN** the controller resumes the run on a capable Mac after Seatbelt activation and separately reruns the feature
- **THEN** resume compiles the pinned Seatbelt world and applies Seatbelt availability, while rerun compiles the discovered harness world and receives that distinct identity; a resume on Linux refuses instead of substituting harness

#### Scenario: A pinned realm does not override an unresolved activation prerequisite
- **WHEN** a run/resume/rerun compiles under the Seatbelt realm word while R1–R4's policy or peer prerequisite remains unresolved
- **THEN** its manifest still pins Seatbelt and its runtime entry refuses under the unbuilt fence; neither a current harness realm nor a compile-only identity changes that verdict
