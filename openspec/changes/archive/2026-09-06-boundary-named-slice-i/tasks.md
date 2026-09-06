# Tasks: The boundary is named — decision 0046, enactment slice (i)

Groups are the commission's items; their order is the design's landing
order (DD16), which is the order another smith executes them in:
deletion first so `hands_command`'s call sites are touched once, the
word and the pin before anything reads them, availability before the
engine composes, the gate law before the unboxed composition and the
adapters' fragments after both (DD16's steps 4a to 4c), the record
before the readouts that render it, then one re-pin, then the prose. Every task
names the requirement it serves as `<capability> / <Requirement>`; the
closing gates of group 13 serve every requirement of the change and say
so, because a gate is not a requirement of its own.

Conventions binding on every task below, restated once rather than per
task: no frozen contract is edited (`run-manifest.v1`–`v8`,
`realms.v1`–`v3`, `seat-record.v1`–`v3`, `effect-provenance.v1`,
`policy/phase-machine.json`, `policy/schemas/`, `reference/`,
`fixtures/evaluator/corpus.ndjson`); tests are written with the code
they prove, in the crate that holds it; any test that drives a boxed
step end to end skips under `BROKKR_HANDS_BOX` as
`crates/brokkr-cli/tests/hands.rs` does; no path is canonicalised and
no two spellings are compared; a mark repeated at several call sites is
one helper, because the coverage gate is literal.

## 1. Retire `driver.confine` (ruling 5)

- [x] 1.1 Delete `confined_command` (`crates/brokkr-runtime/src/engine.rs`)
      and the `Confine` type, and drop the `confine` field from seat
      bodies, panel members, sequence steps and executable bodies in
      `crates/brokkr-runtime/src/bundle.rs`, removing the parameter from
      every `hands_command` call site — driver-confine-retirement / The
      docker wrapper is gone.
- [x] 1.2 Delete `crates/brokkr-runtime/tests/confine_test.rs` and the
      docker-gated machine-proof scenario in
      `crates/brokkr-cli/tests/machine_proof.rs` — driver-confine-retirement /
      The docker wrapper is gone.
- [x] 1.3 Add the parser refusal of `driver.confine` at any site, in a
      bundle and beside `agent:` alike, naming the site, decision 0008's
      retirement into the `container` boundary declared by the realm,
      slice (iii) as what measures it, and decision 0046 ruling 5; drop
      `confine` from the keys legal beside `agent` —
      driver-confine-retirement / driver.confine is refused by name.
- [x] 1.4 Tests in `crates/brokkr-runtime/src/bundle/tests.rs` (and
      `agent_tests.rs`): an inline seat with `confine` refused, an agent
      seat with `confine` refused the same way, every bundle under
      `recipes/` and `bundles/` walked and found to declare none and
      still compiling — driver-confine-retirement / driver.confine is
      refused by name.
- [x] 1.5 A source-scan test that `docker run`, `confined_command` and
      `Confine` appear nowhere in the runtime and cli sources outside
      prose explaining the retirement; and argv tests that a seat, a
      panel member and a sequence step that never declared the field
      compose exactly what they composed before the deletion —
      driver-confine-retirement / The docker wrapper is gone.

## 2. The word: the type and the realm map (ruling 1)

- [x] 2.1 Add `Boundary { Namespace, Seatbelt, Container, Harness, Open }`
      beside `Realm` in `crates/brokkr-core/src/realms.rs` with a
      `FromStr`/`Display` pair, no `Default` — absence resolves to
      `namespace` only in `Realm::boundary()` — and a refusal
      message that lists the five words and says a new boundary is a new
      decision, citing 0046 (DD1) — realm-boundary / The boundary
      vocabulary is closed and lives in one type.
- [x] 2.2 Add the `Option<Boundary>` serde helper that reads and writes
      the sentinel `not applicable`, so the realms enum never admits it
      and the record's sentinel has one spelling — realm-boundary / The
      boundary vocabulary is closed and lives in one type; boundary-record /
      The seat record carries the boundary as seat-record/v4.
- [x] 2.3 Tests in `crates/brokkr-core/src/realms/tests.rs`: each of the
      five words parses and displays itself; `chroot` is refused with the
      five words and the new-decision sentence; a reader of evidence
      carrying no word holds an absence, the type offering no default —
      realm-boundary / The
      boundary vocabulary is closed and lives in one type.
- [x] 2.4 Write `contracts/realms.v4.schema.json` as v3 plus the optional
      per-realm `boundary` (the five-word enum, description saying
      absence reads `namespace`), title `Forge realms map v4`, schema
      constant `forge.realms/v4` — realm-boundary / The realm map declares
      the boundary as forge.realms/v4.
- [x] 2.5 Teach the map loader `forge.realms/v4`: `Realm.boundary:
      Option<Boundary>`, `Realm::boundary()` resolving absence to
      `namespace`, a refusal naming the realm, the field and
      `forge.realms/v4` when `boundary` appears under v1, v2 or v3
      (as `house` and `dialect` are held to theirs), and a refusal
      naming the realm and the five words for an unknown value —
      realm-boundary / The realm map declares the boundary as
      forge.realms/v4.
- [x] 2.6 Loader tests: a v4 map declaring five realms with the five
      words loads and each reports its word; a v4 realm without the field,
      a v3 realm, and a repository with no map at all all resolve to
      `namespace`; the word under `forge.realms/v3` is refused; an
      unknown word in a v4 map is refused — realm-boundary / The realm map
      declares the boundary as forge.realms/v4.
- [x] 2.7 Add the `realms.v4` entry to
      `crates/brokkr-runtime/tests/frozen_contracts.rs` — the file exists
      with its title, the v1–v3 realms files keep their pinned bytes, the
      v4 schema refuses a `boundary` outside the enum and a map whose
      `schema` is not `forge.realms/v4` — realm-boundary / The realm map
      declares the boundary as forge.realms/v4.

## 3. Resolution at compile, the bundle's refusal, the printout (ruling 1)

- [x] 3.1 Give `Bundle` a `boundary` field set by
      `Bundle::compile_with_realm` from the realm's resolved word,
      `namespace` when it compiles in no realm; `compile_with` keeps
      today's behaviour (DD3) — realm-boundary / A run's boundary is the
      realm's, resolved at compile.
- [x] 3.2 Wire the verbs in `crates/brokkr-cli/src/lib.rs`: `run` and
      `compile` compile against the operated repository's realm in the
      discovered or named map; `resume` against the realm embedded in the
      run's pinned world, as it already does for the dialect —
      realm-boundary / A run's boundary is the realm's, resolved at compile.
- [x] 3.3 Change `Cmd::Rerun` to discover the workspace map, compile with
      `compile_in_realm` against the operated repository, call
      `refuse_unboxable`, and start with `Engine::start_in_world`, its
      `--db` handling unchanged (DD6, amending D13); update the rerun
      tests that pinned a manifest with no realms pin and no dialect —
      realm-boundary / A run's boundary is the realm's, resolved at compile.
- [x] 3.4 Tests: `run` under a v4 map declaring `harness` compiles a
      bundle whose boundary is `harness`; `resume` under a workspace map
      that has since changed compiles under the run's pinned
      `namespace` and proceeds; `rerun` compiles under the discovered
      realm's `harness`, its manifest carrying the realms pin, and
      `refuse_unboxable` judges `harness`; `compile_with` with no realm
      is `namespace` and its manifest is what the witness table pins; a
      `seatbelt` realm compiles and pins the word on a machine without
      `sandbox-exec` — realm-boundary / A run's boundary is the realm's,
      resolved at compile.
- [x] 3.5 Refuse a `boundary` key in the bundle parser at every site — a
      seat, a panel member, a sequence step, a selected case body — and
      inside any `hands` object, in a bundle and in an agent file alike,
      with a message naming the site, `realms.json` /
      `forge.realms/v4` as the field's home, and decision 0046 ruling 1;
      the message names the realm rather than reporting an unknown key —
      realm-boundary / A bundle never names the boundary.
- [x] 3.6 Tests in `bundle/tests.rs` and `bundle/agent_tests.rs`: a seat,
      a member, a step and a case body each refused; `boundary` inside
      `hands` refused as a misplaced field and not as an unknown `hands`
      key; an agent file's `hands.boundary` refused when the library
      loads — realm-boundary / A bundle never names the boundary.
- [x] 3.7 Confirm `brokkr compile` prints the manifest and nothing else
      for the boundary — no second copy of the map beside it — and test
      that `brokkr compile --bundle bundles/self` prints
      `manifest.boundary` with `namespace` for every key of
      `manifest.hands` and no other key, and that a bundle with no hands
      site prints neither key — realm-boundary / brokkr compile prints the
      boundary under each hands site.

## 4. The manifest pin (ruling 1)

- [x] 4.1 Write `contracts/run-manifest.v9.schema.json` as v8 plus
      `boundary` (object from site label to the five-word enum), title
      `Forge run manifest v9`, co-presence with `hands` stated by
      `dependencies` in both directions — boundary-manifest-pin / The
      manifest pins the boundary per site as run-manifest/v9.
- [x] 4.2 Write `boundary` in `manifest_for`
      (`crates/brokkr-runtime/src/bundle.rs`) inside the loop that writes
      `hands`, one entry per hands site, every value the bundle's word,
      and neither key for a bundle that boxes nothing, so the key sets
      are equal by construction (DD4) — boundary-manifest-pin / The
      manifest pins the boundary per site as run-manifest/v9.
- [x] 4.3 Tests: two boxed sites give two `boundary` keys equal to the
      `hands` keys and validate under v9; a plain bundle carries neither
      key, validates under v9 and keeps its pre-change digest; the same
      boxed bundle under `namespace` and under `harness` differs only in
      `boundary` and the digests differ; a half-pinned manifest —
      `hands` without `boundary`, `boundary` without `hands`, a value
      outside the enum — fails validation — boundary-manifest-pin / The
      manifest pins the boundary per site as run-manifest/v9.
- [x] 4.4 Add the `run-manifest.v9` entry to `frozen_contracts.rs`: the
      file exists with its title, v1–v8 keep their bytes, every witness
      manifest validates against v9 — boundary-manifest-pin / The manifest
      pins the boundary per site as run-manifest/v9.
- [x] 4.5 Carry `boundary` through `bundle_manifest_from_run`
      (`crates/brokkr-core/src/dispatch.rs`) unchanged, and make
      `manifest_diff` (`crates/brokkr-runtime/src/engine.rs`) name
      `boundary` when it is the non-file field that differs instead of
      blaming the engine or contract version —
      boundary-manifest-pin / The pinned boundary survives resume and is
      refused by the Looper lineage.
- [x] 4.6 Make `build_run_manifest_v2` refuse a bundle manifest carrying
      `boundary` exactly as it refuses every key beyond the six the v2
      round-trip carries — boundary-manifest-pin / The pinned boundary
      survives resume and is refused by the Looper lineage.
- [x] 4.7 Tests: a run started under `namespace` handed a bundle compiled
      under `harness` refuses to resume with a diff naming `boundary`; a
      manifest carrying `hands` and `boundary` offered to the v2 lineage
      is refused naming the keys, before any journal is created —
      boundary-manifest-pin / The pinned boundary survives resume and is
      refused by the Looper lineage.

## 5. Refusal, doctor, init and the engine's fence (ruling 2)

- [x] 5.1 Add `offered(path) -> BTreeMap<Boundary, Offer>` with
      `Offer::Offered(detail) | MissingTool(name) | Unbuilt { slice, tool }`
      in one module of `brokkr-cli` (DD17): `namespace` probes `bwrap`
      and, for a spec with overlay binds, 0.10 or newer as today;
      `seatbelt` probes `sandbox-exec` and reports `Unbuilt("ii")`;
      `container` probes `docker` then `podman` and reports
      `Unbuilt("iii")`; `harness` and `open` are offered — boundary-availability /
      A boundary the machine cannot build refuses at start.
- [x] 5.2 Rewrite `refuse_unboxable` (`crates/brokkr-cli/src/lib.rs`) over
      `offered`: `Ok` for a bundle with no hands site; otherwise judge the
      bundle's boundary and refuse naming the boundary, what it needs and
      what was found, the seats that declare hands, and decision 0046
      ruling 2 — boundary-availability / A boundary the machine cannot build
      refuses at start.
- [x] 5.3 Add the unbuilt refusal after the tool check: `seatbelt` and
      `container` refuse on every machine naming the boundary, the tool
      and where it was found, the seats, the slice of ruling 6 that builds
      it, and `harness` as the road open today (D21) — boundary-availability /
      A boundary this engine does not build refuses at start until its
      slice lands.
- [x] 5.4 Call `refuse_unboxable` from `run`, `resume` and `rerun` before
      any journal row is written or a seat spawned, `resume` having only
      read the pinned manifest — boundary-availability / A boundary the
      machine cannot build refuses at start.
- [x] 5.5 Add the engine's own entry fences: `Engine::start_in_world`,
      `resume` and `start_with_dispatch` refuse a bundle with hands sites
      compiled under `seatbelt` or `container` before `create_run` and
      before any row, naming the boundary and its slice (D21) —
      boundary-availability / A boundary this engine does not build refuses
      at start until its slice lands; and `start_in_world` refuses, before
      `create_run` and before any row, a world that resolves for the
      operated repository a boundary other than the bundle's — no world
      resolving `namespace` — with `EngineError::BoundaryMismatch` naming
      both words, `Engine.boundary` set once from the bundle and never
      re-read (DD3); `resume` is fenced by `manifest_diff` instead (task
      4.5) — realm-boundary / The engine starts a run only under the
      boundary its bundle was compiled under.
- [x] 5.6 Availability tests, one per boundary on an empty search path:
      `namespace` refuses naming bubblewrap and the seat `work`;
      `seatbelt` naming `sandbox-exec`; `container` naming `docker` or
      `podman`; `harness` and `open` pass; `namespace` passes with a
      planted `bwrap` and no overlay bind; an overlay bind with an older
      `bwrap` still refuses naming `0.10 or newer`; a bundle with no hands
      site passes under every boundary — boundary-availability / A boundary
      the machine cannot build refuses at start.
- [x] 5.7 Unbuilt tests: `seatbelt` with a planted `sandbox-exec` still
      refuses naming the tool found, the seat, slice (ii) and `harness`;
      `container` with only a `docker`, then only a `podman`, each refuses
      naming slice (iii); a library caller starting, resuming or rerunning
      such a bundle is refused by the engine with no `run/started` or any
      other row written; a plain bundle under `seatbelt` starts —
      boundary-availability / A boundary this engine does not build refuses
      at start until its slice lands.
- [x] 5.8 Fence tests in the engine's tests: a `harness` bundle started
      with a world whose realm declares no boundary is refused naming
      `harness` and `namespace` with no `run/started` or other row; the
      same bundle with no world is refused the same way, while a
      `namespace` bundle with no world starts as today; a `harness` bundle
      with a world declaring `harness` starts and its `run/started`
      manifest's `boundary` map says `harness` — realm-boundary / The
      engine starts a run only under the boundary its bundle was compiled
      under.
- [x] 5.9 Verb tests: `brokkr run`, `brokkr resume` and `brokkr rerun`
      each fail with the refusal, writing no journal row and spawning no
      seat — boundary-availability / A boundary the machine cannot build
      refuses at start.
- [x] 5.10 Add doctor's one `boundaries` line from the same `offered` map
      (`crates/brokkr-cli/src/doctor.rs`): the boundaries a run can start
      under here with the bubblewrap version found, and for each it does
      not offer, why — the missing tool for `namespace`, the building
      slice plus the tool as a readiness fact for `seatbelt` and
      `container` — boundary-availability / doctor names the boundaries this
      machine offers.
- [x] 5.11 Make `doctor --bundle` compile in the discovered realm and
      judge the existing `hands` line against the realm's boundary:
      healthy under `namespace` with bubblewrap and under `harness` or
      `open` always, a warning under `namespace` without bubblewrap, a
      warning under `seatbelt` or `container` naming the unbuilt slice —
      boundary-availability / doctor names the boundaries this machine
      offers.
- [x] 5.12 Doctor tests in `crates/brokkr-cli/tests/init_doctor.rs`: a
      Linux box with `bwrap 0.11.0` and `docker` and no `sandbox-exec`;
      an empty PATH; `--bundle` on a boxed bundle in a `harness` realm
      with no `bwrap` staying healthy; `--bundle` in a `seatbelt` realm
      warning about slice (ii) with and without the tool —
      boundary-availability / doctor names the boundaries this machine
      offers.
- [x] 5.13 Rewrite `brokkr init`'s bubblewrap warning
      (`crates/brokkr-cli/src/lib.rs`, under `Cmd::Init`, where `bwrap_on`
      is consulted) to name the scaffolded seats,
      `namespace` as the default needing bubblewrap, `harness` as what a
      realm may declare instead, and decision 0046 — with its test —
      instead of saying the shipped gates require Linux —
      boundary-availability / init's warning speaks the vocabulary.

## 6. The gate law and the pinned-script grammar (ruling 4, DD16 step 4a)

- [x] 6.1 Extract the manifest walk's skip rule (`realms.json`, `dialects/…`)
      from `manifest_for` into one function and share it with the
      pinned-script lookup, so the two cannot drift (DD9) —
      gate-boundary-policy / An exec site with hands under harness or open holds only
      for pinned bytes.
- [x] 6.2 Add the pinned-script check as a pure grammar and lookup over
      the raw command, before `expand_command`, exactly as
      gate-boundary-policy / An exec site with hands under harness or open holds only
      for pinned bytes states it — that requirement is the one copy of the
      grammar and the lookup; the walk's skip rule is 6.1's shared
      function; nothing is canonicalised and no two spellings are
      compared (DD9) — gate-boundary-policy / An exec site with hands under harness
      or open holds only for pinned bytes.
- [x] 6.3 Admit an exec site with hands — gate or work, the class unread
      (D32) — under `harness` and `open` only on that verdict, refusing
      otherwise naming decision 0046 ruling 4, decision 0021 and, for a
      spelling, the spelling, as the inline-exec arm of the hands law
      `enforce_hands_boundary`, the first statement of
      `enforce_model_policy` before its class early-return and its
      adapter destructure, reading no adapter on this arm (DD22); hand
      the law the declaring layer's directory every caller already holds
      — gate-boundary-policy / An exec site with hands under harness or
      open holds only for pinned bytes.
- [x] 6.4 Refuse a dialect validate or check step — the compiler's
      synthetic boxed exec gate — under `harness` and `open` at compile,
      naming the step, decision 0046 ruling 4, decision 0042 ruling 4 and
      a boxed boundary as the road open today; under `namespace`,
      `seatbelt` and `container` the step compiles exactly as today;
      the two sites that build the synthetic gate share one helper and
      the refusal lives in it, before the gate law and the hands law run
      (DD8, DD22) — gate-boundary-policy / An exec site with hands under harness or open
      holds only for pinned bytes.
- [x] 6.5 Give `enforce_model_policy`
      (`crates/brokkr-runtime/src/bundle.rs`) the boundary axis for sites
      that declare hands as the chain arm of the hands law
      `enforce_hands_boundary`, its first statement — before the work
      early-return and the adapter destructure, reading the adapters only
      on this arm, where the `agent` key opened them (DD22) — decision
      0021's refusals unchanged under every
      boundary: `namespace`, `seatbelt` and `container` admit a model gate
      as today; `harness` admits one only when every link of the resolved
      chain declares `hands.harness.gate` as a fragment — a member group 8
      teaches the loader to read, so until then no link declares one and
      the arm refuses, fail-closed — refusing otherwise by link, provider
      and missing declaration; `open` refuses a model gate naming ruling
      4 and admits a work-class chain site, asking no fragment of any
      link, to run at the harness's default (D11, DD22), which is the
      arm 6.10 covers; a gate-class site without hands compiles as today
      under every boundary (D1) — gate-boundary-policy / The gate law
      reads the boundary for sites that declare hands.
- [x] 6.6 Refuse a work-class site with hands under `harness` whose chain
      has a link declaring no `hands.harness.work` fragment, as the
      capability gap it is, naming the link, inside the hands law's chain
      arm before the work early-return (DD7, DD22) —
      gate-boundary-policy / The gate law reads the boundary for sites that
      declare hands.
- [x] 6.7 Refuse an inline model site with hands under `harness` or `open`
      at compile, as the hands law's inline-model arm (DD22), naming the
      seat and the repair — seat it through an
      agent, or run the realm under a boxed boundary (D10); with its test
      in `model_policy_tests.rs`: an inline seat whose command is a
      `{brokkr} driver claude` dispatch and which declares hands is
      refused under `harness` and under `open` naming the seat and the
      repair, and compiles under `namespace` as today —
      gate-boundary-policy / The argv of a site with hands follows the
      boundary and the class.
- [x] 6.8 Add the pure layer re-walk beside `manifest_for`:
      `layer_drift(layer_dir, pinned) -> Option<key>` re-derives the
      declaring layer's pinned identity with the same walk — the leaf's
      `files` map, or an ancestor's compose digest by the same
      `manifest_for` call over `Ancestor.dir` and its deeper chain — and
      names the first key whose bytes moved, went missing or appeared
      (DD9); the engine calls it at spawn (task 7.7) —
      gate-boundary-policy / An exec site with hands under harness or open holds only
      for pinned bytes.
- [x] 6.9 Pinned-script tests in
      `crates/brokkr-runtime/src/bundle/model_policy_tests.rs`:
      `bundles/self`'s verify seat admitted under `open`;
      `["{brokkr}","driver","exec","--","true"]` refused; `./../outside.sh`,
      `/usr/bin/true`, `.\scripts\s.sh` and `/private/var/b/scripts/s.sh`
      each refused naming the token with no path compared;
      `./scripts/missing.sh` refused naming the token and the directory
      searched; `bash -c ./scripts/s.sh` refused naming `-c` as an option
      token; `./dialects/run.sh` refused as a path the walk does not pin;
      `recipes/wager-harness`'s inherited verify seat admitted against
      `recipes/fast/scripts/verify-seat.sh`; the operator's work-class
      pair — a `class: work` exec site with hands whose command names
      `./scripts/lint.sh`, a file the walk pins, admitted under `open`,
      and its sibling `["{brokkr}","driver","exec","--","true"]`
      refused naming ruling 4 and 0021, the class changing nothing — and
      the same bundle, which names no agent, gate or secret, compiled
      with no `adapters/` directory reachable, admitted and refused the
      same (D32, DD22); the shipped ship seat
      admitted with `{brokkr}` among its unjudged arguments; a fixture
      bundle with an artifact phase, its chief on a fixture provider that
      declares both `hands.harness` members, refused under `harness`
      naming its synthetic validate step, ruling 4, 0042 ruling 4 and a
      boxed boundary, and compiling under `namespace` — gate-boundary-policy
      / An exec site with hands under harness or open holds only for pinned bytes.
- [x] 6.10 Model-policy tests in the same file: a `harness` gate on a
      provider declaring the fragment admitted; on one declaring none
      refused naming provider, `hands.harness.gate` and ruling 4; a chain
      whose second link declares none refused naming the second link; an
      `open` model gate refused and an `open` work-class chain site
      admitted with no link declaring a fragment, the law's last arm
      (D11, DD22); a `seatbelt` and a `container` gate
      admitted at compile with the word pinned; a `harness` work seat
      without a `work` fragment refused as a capability gap; an inline
      trusted model gate with a tool list and no hands admitted under
      `open` exactly as under `namespace`; and every shipped bundle's
      refusals and admissions under `namespace` unchanged — these arms
      on fixture providers, as the file's charter requires; the shipped
      codex, dsh and lanetally adapters are compiled at a `harness` gate
      in 8.12, once 8.4 has landed codex's fragment —
      gate-boundary-policy / The gate law reads the boundary for sites
      that declare hands.
- [x] 6.11 Re-walk tests as a pure function over a temporary layer: an
      untouched layer names no key; an edited script, an edited sibling
      the script sources, a deleted pinned file and an added file each
      name the first key that differs; an ancestor layer's digest is
      re-derived for an inherited seat and names the ancestor when its
      script moved (DD9) — gate-boundary-policy / An exec site with hands under
      harness or open holds only for pinned bytes.

## 7. Run-time composition of the unboxed exec dispatch (ruling 4, DD16 step 4b)

- [x] 7.1 Add `SpawnEnv { Inherit, Exactly(table) }` to
      `DriverProcess::spawn` (`crates/brokkr-protocol/src/process.rs`),
      `Inherit` being today's behaviour, and thread it through every
      spawn site (D25, DD18) — gate-boundary-policy / An unboxed exec
      dispatch runs in a fixed environment.
- [x] 7.2 Add the pure unboxed-environment function beside the box's own
      table in `crates/brokkr-protocol/src/hands.rs`, of the engine's
      environment, the engine's home, the site's spec, the identity and
      the two scratch paths, composing exactly the table
      gate-boundary-policy / An unboxed exec dispatch runs in a fixed
      environment lists — that requirement is the one copy of the table;
      DD10 carries the reason for each entry that differs from the box's,
      `USER` and `LOGNAME` included — gate-boundary-policy / An unboxed
      exec dispatch runs in a fixed environment.
- [x] 7.3 Add the network prefix and its probe in the same module, the
      prefix's tokens and the probe's arms exactly as gate-boundary-policy /
      The argv of a site with hands follows the boundary and the class
      spells them — that requirement is the one copy — the probe run once
      per engine process at the first unboxed exec dispatch, in that
      dispatch's environment against its search path, its answer
      remembered on the engine for every later dispatch, consulted only
      on Linux, never journaled (D12, DD15) — gate-boundary-policy / The argv of a site
      with hands follows the boundary and the class.
- [x] 7.4 Replace the `hands_command(confined_command(..))` nesting at the
      four call sites (single, panel member, sequence step, dialect step)
      with `compose_site(boundary, class, command, hands,
      harness_fragments, workdir, roots, result_path, prefix) -> SiteSpawn
      { argv, env }` (DD18), a three-arm match over the boundaries this
      engine builds so no unreachable arm exists (DD2); the dialect-step
      call site composes under a boxed boundary only, because 6.4 refuses
      the step elsewhere — gate-boundary-policy / The argv of a site with
      hands follows the boundary and the class.
- [x] 7.5 Compose per boundary: `namespace` today's argv token for token
      with `SpawnEnv::Inherit`; `harness` the adapter's `gate` fragment
      for a gate-class site and `work` for a work-class one, with
      `{result_path}` and `{brokkr}` expanded, no MCP server served and no
      box built (the fragments group 8 lands); `open` the adapter's base
      driver argv and nothing of Brokkr's. An exec dispatch under
      `harness` and `open` is the compiled command itself —
      `{prompt_file}` left literal for the exec driver — behind the prefix
      when the probe passes, with `SpawnEnv::Exactly(table)` and the
      worktree as its working directory. A model site keeps the engine's
      environment under every boundary — gate-boundary-policy / The argv
      of a site with hands follows the boundary and the class; An unboxed
      exec dispatch runs in a fixed environment.
- [x] 7.6 Keep `hands.network` and `hands.binds` pinned in the manifest as
      declared under `harness` and `open`, enforced by nothing of
      Brokkr's (D18) — gate-boundary-policy / The argv of a site with hands
      follows the boundary and the class.
- [x] 7.7 Before an unboxed exec dispatch spawns, call `layer_drift` (6.8)
      over the declaring layer against the identity the run manifest
      pins, and on a named key refuse the dispatch — nothing spawned, the
      attempt's failure journaled naming the layer and the key — so the
      bytes that run are the bytes the digest names, the residual being
      the interval between the re-walk and the `exec` (DD9) —
      gate-boundary-policy / An exec site with hands under harness or open holds only
      for pinned bytes.
- [x] 7.8 Argv tests as pure argv, no spawning: a boxed model site and a
      boxed exec site under `namespace` equal to what `hands_command`
      produced before this change, token for token; a codex gate under
      `harness` carrying `--sandbox read-only --output-last-message
      <result path>` with no `mcp_servers.brokkr`, no `{hands_mcp_json}`
      expansion and no literal `{result_path}`; a codex work site carrying
      `--sandbox workspace-write` and no MCP server; an `open` work site
      carrying the base driver argv with the model and effort pins alone;
      a site declaring `network: false` keeping its manifest entry with no
      network switch in its argv; and every model site — the codex gate
      under `harness`, the `open` work site — spawned with
      `SpawnEnv::Inherit`, the engine's own environment, exactly as under
      `namespace`, because its harness needs the operator's keys; and
      `compose_site` over one exec command with hands under `open` with
      the class flipped, `Work` and `Gate`, yielding equal argv and
      environment (D32, DD22) —
      gate-boundary-policy / The argv of a
      site with hands follows the boundary and the class.
- [x] 7.9 Pin the unboxed exec dispatch token for token:
      `bundles/self`'s verify seat under `harness` on Linux with the probe
      passing is exactly `unshare`, `--map-root-user`, `--net`, `--`,
      `sh`, `-c`, `ip link set lo up && exec unshare --map-user=<uid>
      --map-group=<gid> -- "$@"`, `sh`, `<brokkr>`, `driver`, `exec`,
      `--`, `bash`, `<repo>/bundles/self/scripts/verify-seat.sh`,
      `{prompt_file}`; with the probe failing, and on macOS and Windows,
      and under `open`, exactly the compiled command alone; the probe's
      command is the eight-token prefix around `true`; the probe answers
      no-without-spawning, no and yes on a search path with no `unshare`,
      a planted non-zero `unshare` and a planted zero one, and a second
      dispatch of the same engine spawning no second probe —
      gate-boundary-policy / The argv of a site with hands follows the
      boundary and the class.
- [x] 7.10 Environment tests as a pure function: the shipped verify seat on
      a rustup machine carries `PATH` verbatim, the private `HOME` and
      `TMPDIR`, `CARGO_HOME` and `RUSTUP_HOME` from the declared binds,
      the fixed entries and the git identity, and none of `GH_TOKEN`,
      `ANTHROPIC_API_KEY`, `SSH_AUTH_SOCK`, `NPM_CONFIG_CACHE` or
      `BROKKR_HANDS_BOX`; a planted `.ssh/id` under the engine's home is
      not reached by a spawned `sh -c 'cat "$HOME/.ssh/id"'` — a proof
      about the environment and not the filesystem, named as such —
      while `CARGO_HOME` names the planted `.cargo`; the locators follow the
      binds and not the engine's environment, `~/.npm` giving
      `NPM_CONFIG_CACHE`; the marker is inherited and never set; the
      Windows names are carried verbatim on Windows and not consulted
      elsewhere — gate-boundary-policy / An unboxed exec dispatch runs in a
      fixed environment.
- [x] 7.11 Re-walk tests in the engine's tests, skipping under
      `BROKKR_HANDS_BOX` where a real spawn is driven: an unboxed exec
      gate whose verify script was edited after compile fails the attempt
      naming the layer and the key with nothing spawned and the failure
      journaled; the same gate with an edited sibling script fails the
      same way; an untouched layer spawns; an inherited seat whose
      ancestor layer moved fails naming the ancestor; a `namespace` gate
      over the same edited layer is composed as today, because the box
      is its admission — gate-boundary-policy / An exec site with hands under harness
      or open holds only for pinned bytes.

## 8. The adapters' `hands.harness` and the judge's door (ruling 4, DD16 step 4c)

- [x] 8.1 Extend the adapter loader
      (`crates/brokkr-protocol/src/adapters.rs` and the runtime's
      resolver) with an optional `hands.harness` object of exactly three
      members: `gate` and `work`, each an argv fragment or
      `{"unsupported": "<measured reason>"}`, an absent member reading
      unsupported without a reason (fail-closed, the three-shape
      convention `tool_permissions` uses); and `result`, optional, one of
      `file` and `last-message`, absent reading `file`; the gate law's
      `harness` arms (6.5, 6.6) now read the members —
      gate-boundary-policy / An adapter declares how its harness stands
      under the harness boundary.
- [x] 8.2 Admit `{result_path}` and `{brokkr}` in `hands.harness`
      fragments and refuse `{hands_mcp_json}` and `{hands_args_toml}`
      there, saying no workspace tool is served under `harness` (D23) —
      gate-boundary-policy / An adapter declares how its harness stands
      under the harness boundary.
- [x] 8.3 Loader tests: an unknown member (`hands.harness.judge`) refused
      naming the three the vocabulary admits; `result` `stdout` refused
      naming `file` and `last-message`; a workspace token in either
      fragment refused; an empty fragment accepted as a legal measured
      declaration — gate-boundary-policy / An adapter declares how its
      harness stands under the harness boundary.
- [x] 8.4 Declare `adapters/codex.json`'s `hands.harness`: `gate`
      `["--sandbox","read-only","--output-last-message","{result_path}"]`
      with `result` `last-message`, `work` `["--sandbox",
      "workspace-write"]`, from codex's own documented sandbox classes and
      the capture flag `codex exec --help` documents — `--sandbox
      read-only` being ruling 4's own word for codex, so the declaration
      is from the decision and the tool's record, not a guess; the one
      fact still unmeasured, whether the capture lands under the
      read-only class, is task 8.9's —
      gate-boundary-policy / An adapter declares how its harness stands
      under the harness boundary.
- [x] 8.5 Measure claude's fragments against the installed claude 2.1.x
      and record them in `adapters/claude.json` — never guessed — only
      where a `claude` the seat may run is reachable: the implementing
      seat's tool grant is `cargo` and `git`, so this measurement is the
      operator's and the seat does not attempt it. For `gate`, a
      read-only mode that leaves exactly one write door at the expanded
      `{result_path}` (candidates: `--permission-mode dontAsk` with
      `--allowedTools` and an edit rule scoped to the path,
      `--permission-mode plan`, and the reported `--restricted` /
      `--permission-prompts none` on 2.1.263); for `work`, a writable
      mode that leaves the chief its `cargo` and `git` without prompting
      (candidates: `--permission-mode acceptEdits` with the shell
      allowed, or the harness's sandbox settings; the empty fragment only
      if the driver's own `acceptEdits` is measured to grant the shell).
      The recipe is the proposal's *Measurements* section. Until the
      measurement is recorded, `adapters/claude.json` declares no
      `hands.harness` member — absence is the loader's fail-closed
      reading — and nothing is reported blocked for want of it: every
      other task is finished and committed, and task 11.6 carries the
      report — gate-boundary-policy / An adapter declares how its harness
      stands under the harness boundary; Every shipped bundle compiles
      under harness once the fragments are measured.
> Task 8.5 close-out: the conditional unmeasured state specified above is
> satisfied. Both Claude fragments remain absent and the operator measurements
> are still pending; this check does not claim a measurement. See completion.md.

- [x] 8.6 Where a measured mode leaves no door for a judge, or no writable
      mode a work seat can stand in, declare that member
      `{"unsupported": "<measured reason>"}` — never an unmeasured
      fragment (DD20) — gate-boundary-policy / A judge under harness still
      delivers its result file.
- [x] 8.7 Leave `adapters/dsh.json` and `adapters/lanetally.json` without
      `hands.harness`; their refusal at a `harness` gate — the hands
      law's, naming the link, the provider and the missing
      `hands.harness.gate` (D33), not the trust-tier text a boxed gate
      earns today — is pinned in `model_policy_tests.rs` against the
      shipped files by 8.12 —
      gate-boundary-policy / An adapter declares how its harness stands
      under the harness boundary.
- [x] 8.8 Adapter data tests: codex's two fragments and its door load as
      declared; claude's `gate` and `work` are each undeclared, a
      fragment, or `unsupported` with a reason, a `gate` fragment naming
      `{result_path}` as its door — gate-boundary-policy / An adapter
      declares how its harness stands under the harness boundary.
- [x] 8.9 Record codex's door: `--output-last-message {result_path}`
      under `--sandbox read-only` is declared from the tool's record, and
      whether the capture lands under the read-only class is measured by
      the same recipe — one gate seat under the fragment delivering its
      result and nothing else — where a `codex` may be run, which for
      the implementing seat is nowhere; record the measurement in the
      guide when made (task 12.1) and, until then, that it is pending and
      the operator's, listed in task 11.6's note; if it fails, declare
      `gate` unsupported with the reason (task 8.6) —
      gate-boundary-policy / A judge under harness still delivers its
      result file.
- [x] 8.10 Deliver the judge's result under `harness`: the engine expands
      `{result_path}` at spawn to the seat's own result path and reads the
      result file as today under both doors; a final message that is not
      the bare object is a missing result exactly as a malformed file is —
      gate-boundary-policy / A judge under harness still delivers its
      result file.
- [x] 8.11 Door tests: a codex gate whose result path is `P` composes
      `--output-last-message P`, its input carrying `result_delivery:
      last-message` and its prompt naming `P` and the final-message
      contract; an adapter with `result` absent and a `gate` fragment
      carrying `{result_path}` expands the token, carries no
      `result_delivery` and keeps today's result contract; an adapter
      declaring `gate` unsupported is refused at a `harness` gate at
      compile — gate-boundary-policy / A judge under harness still delivers
      its result file.
- [x] 8.12 Pin ruling 4's own binding in
      `crates/brokkr-runtime/src/bundle/model_policy_tests.rs` against
      the shipped adapter library (`shipped_adapters()`), beside the pins
      that compile the shipped codex adapter at a boxed gate: a fixture
      gate-class agent whose agent file declares hands and whose chain is
      `astra` alone compiles under `harness` and is admitted, its manifest
      pinning the shipped codex adapter's digest; the same fixture chaining
      one dsh model alone (`flash`), and one chaining one lanetally model
      alone (`fable-tallied`), each compiled under `namespace` and then
      under `harness`, refused under both — under `namespace` by 0021
      ruling 2's trust-tier text, under `harness` by the hands law
      naming the link, the provider and `hands.harness.gate` (D33) —
      gate-boundary-policy / The gate law reads the
      boundary for sites that declare hands; An adapter declares how its
      harness stands under the harness boundary.

## 9. The record (ruling 3)

- [x] 9.1 Build `boundary_entries` beside `select_candidates` in
      `crates/brokkr-runtime/src/engine.rs` from the same
      `invocation_sites(body)` traversal that builds `provenance`: one
      `{member, boundary, gate}` entry per invocation site of the attempt,
      the realm's word for a site with hands and the sentinel for one
      without, `gate` the site's class read where the compiler keeps it —
      `seat.body.selected_is_gate(strategy, seat.has_gate)` for a single,
      a panel member or a selected case, `step.class` for a step and its
      panel's members, no new field (DD5); write it on `effect/started`
      beside `provenance` exactly when at least one site of the attempt
      declares hands (DD5) — boundary-record / effect/started carries the
      boundary beside provenance.
- [x] 9.2 Publish `contracts/effect-boundary.v1.schema.json`, title
      `Forge effect boundary v1`, and add its frozen-contracts entry:
      `effect-provenance.v1` unedited, every entry the engine writes
      validating, and the contracts README listing it among the extension
      schemas `fold` never reads — boundary-record / effect/started carries
      the boundary beside provenance.
- [x] 9.3 Effect tests: a gate-class boxed single seat under `harness`
      journals one entry with `member` null, `harness`, `gate` true; a
      sequence of a hands-less author step and a boxed dialect validate
      step lists the first `not applicable` and the second `namespace`
      with `gate` true; a bundle with no hands site journals no
      `boundary` key and payloads byte-identical to today's; folding a
      journal with the field equals folding it without —
      boundary-record / effect/started carries the boundary beside
      provenance.
- [x] 9.4 Write `contracts/seat-record.v4.schema.json` as v3 plus the
      optional `boundary` on the checkpoint and on the successful result
      (the five words or `not applicable`), title `Forge seat record v4`,
      and embed the byte-identical copy at
      `crates/brokkr-store/src/seat-record.v4.schema.json` —
      boundary-record / The seat record carries the boundary as
      seat-record/v4.
- [x] 9.5 Add `SeatRecordVersion::V4` dispatched by the 0.9 line (D9): an
      engine at or after `0.9.0` under v4, the `0.8` line under v3, an
      earlier or unparseable engine under v1; the store's append fence
      validates the stamped record under v4 —
      boundary-record / The seat record carries the boundary as
      seat-record/v4.
- [x] 9.6 Add the `stamp_boundary(record, Option<Boundary>)` helper with
      one rule — a record that names a `model` carries `boundary` beside
      it, a record that names none carries no `boundary`, a driver's
      value replaced or dropped — applied at the two pass-throughs
      through which every driver checkpoint and result reach the store
      (the `run_driver` closure and the panel receiver loop, beside
      `tag_member`) and at the engine's own `panel-member-finished` and
      `sequence-step-finished` markers, which name a model (DD19);
      drivers and their conformance field sets do not change; a panel's
      aggregate result, which names no model, carries no `boundary`, and
      a sequence's ending result — its ending step's driver result —
      carries that step's word — boundary-record / The seat record
      carries the boundary as seat-record/v4.
- [x] 9.7 Record tests: a boxed exec gate under `namespace` carries the
      word on both records and `brokkr export` and `verify-run` accept the
      journal; a hands-less inline exec seat and a tool-list model seat
      carry `not applicable`; a driver's `open` on a `namespace` seat is
      replaced, a per-turn checkpoint that names no model is appended
      without the driver's word and one that names a model carries the
      engine's; the engine's member- and step-finished markers carry the
      site's word; a panel's aggregate carries none and a sequence's
      ending result its step's word; a result carrying
      `chroot` is refused at append naming the schema path with nothing
      written and the attempt's failure journaled as 0034 ruling 6 reads;
      the 0.9/0.8/earlier dispatch, and a tagged-0.9.0 journal with no
      `boundary` still validating; the frozen-contracts and embedded-copy
      entries with v1–v3 keeping their bytes and the README carrying a v4
      row — boundary-record / The seat record carries the boundary as
      seat-record/v4.
- [x] 9.8 Carry `boundary` in the seat input of every site with hands
      under every boundary, through the one mark helper `mark_boxed`
      grows into at its four call sites (DD21), keep the `hands: boxed` marker exactly when
      Brokkr builds the box (`namespace`, `seatbelt`, `container`) and
      absent under `harness` and `open`, and add `result_delivery:
      last-message` for a `harness` gate-class site whose adapter declares
      that door (D15, D23) — boundary-record / The seat input and prompt
      name the boundary the seat stands under.
- [x] 9.9 Give `render_prompt` the driver kind — `render_prompt(input,
      kind: AdapterKind)`, one signature, five call sites, no wire byte
      (DD21) — and render the prompt paragraph to follow the two for the
      model kinds and never for `AdapterKind::Exec` (D31)
      (`crates/brokkr-protocol/src/adapters.rs`): under the three boxed
      words exactly today's paragraph naming `mcp__brokkr__workspace` as
      the only writer; under `harness` a paragraph naming the word, the
      harness's own sandbox and no workspace tool, saying with `file` that
      the result path is the one file the sandbox lets it write and with
      `last-message` that the final message must be exactly the result
      object which the harness writes to the path, the result contract's
      line following; under `open` the same with the word `open` and no
      delivery change; a site without hands unchanged; an exec site's
      prompt carrying no hands paragraph under any boundary (D31) —
      boundary-record / The seat input and prompt name the boundary the
      seat stands under.
- [x] 9.10 Input and prompt tests: `namespace` byte-identical to today
      with `hands: boxed` and `boundary: namespace`; a `harness` gate on a
      `file` door with the word, no marker, no `result_delivery` and a
      paragraph that does not name the workspace tool; a `harness` gate on
      a `last-message` door with the field, the final-message contract and
      no file asked for; an `open` site with the word and no marker; the shipped verify seat
      of `bundles/self` rendered under `namespace`, `harness` and `open`
      in turn, its input carrying `boundary` each time and `hands: boxed`
      only under `namespace`, its prompt naming neither the workspace
      tool nor the word in a hands paragraph (D31, DD21); a
      site without hands carrying neither field and no hands paragraph —
      boundary-record / The seat input and prompt name the boundary the
      seat stands under.

## 10. The readouts (ruling 3)

- [x] 10.1 Add `ModelAtBoundary { model: Cell, boundary: Cell }` to
      `brokkr-view` and replace the bare `model` cell on `Participant`,
      `Node`, `CheckpointRow` and `JournalRow` with it as a
      `#[serde(flatten)]` field named `served`, so the wire keeps `model`
      and gains `boundary` as siblings; advance `VIEW_VERSION` to 9
      (DD12) — boundary-readouts / The view derives the boundary beside
      every model cell.
- [x] 10.2 Derive the cells beside the provenance scan: a participant's
      from its site's `effect/started.boundary` entry with the last
      attempt winning; a node's from its participant; a checkpoint row's
      from its attempt, the same word on every row of that attempt; a
      journal row's from the effect and site its model is read beside,
      absent for an event belonging to no effect —
      boundary-readouts / The view derives the boundary beside every model
      cell.
- [x] 10.3 Fall back, for a participant whose attempt journaled no entry,
      to the `boundary` a record of its attempt carries beside a `model`
      — its finishing checkpoint, its successful result or the engine's
      own marker (DD19) — and otherwise render the absent mark with the note
      `no boundary recorded` — every model row of a pre-0046 journal,
      boxed or not, and a running seat whose record has not landed —
      never `not applicable` derived from the manifest and never a
      default (DD13); read one boolean from the `run/started` manifest,
      whether the run declares hands, beside the agents roster the view
      already reads there, for the run-level fact alone; an entry outside
      the vocabulary or lacking a `member` tag is not recorded, never
      boxed or unboxed (DD14) —
      boundary-readouts / The view derives the boundary beside every model
      cell.
- [x] 10.4 Add `RunView.boundary: RunBoundary { word, unboxed, text }`,
      derived once: `unboxed` true exactly when a valid entry has `gate`
      true and a word of `harness` or `open`, absent with the note
      `no boundary recorded` when the manifest declares hands and no valid
      entry exists, and nothing when it declares none; the data carries
      the plain word and only `text` carries the adjective —
      boundary-readouts / A run whose gate stood under harness or open is
      rendered unboxed.
- [x] 10.5 View tests: a boxed seat under `harness` reading `harness`
      beside its model; a pre-0046 journal rendering the absent mark and
      the note everywhere with no surface printing `namespace`; a
      hands-less site reading `not applicable` from its finishing
      checkpoint and the absent mark while it still runs; a boxed seat with three
      turns carrying the cell on its participant, node, three checkpoint
      rows and journal rows with a `run/started` row's cell absent; an old
      journal with no `hands` key at all reading the absent mark
      throughout and rendering no run-level fact; a `chroot` entry and a
      tagless entry read as not recorded; `--json` from `inspect`
      carrying `view_version` 9 with the boundary fields present as
      null-bearing cells and never skipped —
      boundary-readouts / The view derives the boundary beside every model
      cell.
- [x] 10.6 Print the run-level fact from that one derivation on every
      surface that summarises a run — `brokkr inspect`'s header,
      `brokkr watch`'s frame header, the TUI's run header and the web
      console's run header — and never compose it on a page; test
      `harness · unboxed` on all four, `namespace` alone for a boxed run,
      `harness` without the adjective for a run whose only boxed site is
      work-class, and nothing at all for a run that boxes nothing —
      boundary-readouts / A run whose gate stood under harness or open is
      rendered unboxed.
- [x] 10.7 Add the one pair helper — a text face that prints the model
      and boundary pair, a JSON face that emits them as data for
      `compare`'s `resolution` map (10.10) — and use it in
      `crates/brokkr-cli/src/render.rs`: the seats
      table gains a `boundary` column beside `model`, the per-seat lines
      under `inspect`, `inspect --seat` and `watch` print the pair, and a
      decision-trail row printing `· model <x>` prints `· boundary <y>`
      beside it — boundary-readouts / Every readout that names a seat's
      model names its boundary.
- [x] 10.8 Add the thin `brokkr seats --run <id> [--realms] [--db] [--json]` verb
      (DD11) rendering the seats block `inspect` renders from the same
      `RunView`, opening the journal by the same `journal_of` route so
      `--realms` and `--db` mean what they mean for `inspect`, `--json`
      printing the view model verbatim — byte-identical to
      `inspect --json` — and deriving nothing of its own; test both faces —
      boundary-readouts / Every readout that names a seat's model names its
      boundary.
- [x] 10.9 Carry the pair in the TUI (`crates/brokkr-cli/src/tui.rs`) —
      seat table, seat detail, checkpoint rows, journal rows — and in the
      web console (`crates/brokkr-cli/src/ui.html`) — participants table,
      seat detail, checkpoint stream, journal rows — read from the model
      `/api/view/<run>` serves and computed nowhere on the page, the
      page's reads of `item.model`, `part.model` and `e.model` replaced
      by one page-side pair helper — a single script function taking the
      carrier and returning the two cells, the only place in the page
      that names `.model` (DD12, D28); test each, the console's tests
      proving the two cells land in one row — boundary-readouts / Every
      readout that names a seat's model names its boundary.
- [x] 10.10 Reduce `boundary` in the one seat-costs derivation
      (`crates/brokkr-cli/src/compare.rs`) exactly as `model` is reduced —
      the set of words the seat's records that name a model carry (its
      finishing checkpoint, its successful result, a per-turn checkpoint
      that names one), one word or a joined list, `not recorded` when none
      does — print the plain word in `costs`, and report a boundary
      difference in `compare` as a first-class divergence; carry the pair
      per participant in `compare`'s `resolution` map (`resolution_of`)
      through the helper's JSON face, `resolution_divergence` diverging
      on `boundary` as on `model`; test a `harness` run beside a
      `namespace` run — the seat-costs records and the `resolution` map
      both — and a pre-0046 seat reading `not recorded` — boundary-readouts / Every readout that names a
      seat's model names its boundary.
- [x] 10.11 Add the roster-style pin test as
      `crates/brokkr-cli/tests/boundary_readouts.rs` — the readout
      sources it reads (`render.rs`, `tui.rs`, `compare.rs`, `ui.html`)
      are that crate's, and it walks up from `CARGO_MANIFEST_DIR` as
      `crates/brokkr-runtime/tests/roster.rs` does — reading every readout
      source and failing, naming the source, where `served.model` is read
      outside the pair helper's two faces or a seat-costs record's
      `model` key is rendered without the boundary beside it; and that
      scans `ui.html` — as `crates/brokkr-cli/tests/agent_readouts.rs`
      already scans it for a
      composed provenance sentence — failing and naming the line where
      `.model` is read off a carrier outside the page-side pair helper
      (DD12, D28) — boundary-readouts / Every readout that names a seat's
      model names its boundary.
- [x] 10.12 Test `brokkr export` as the record itself: the exported
      `effect/started` events and seat records of a `harness`-judged run
      carry the plain word, `verify-run` accepts the file and no adjective
      appears in it — boundary-readouts / Every readout that names a seat's
      model names its boundary.
- [x] 10.13 Teach `scripts/delivered-by-brokkr.sh` to read the anchored
      run's `effect/started` events after verifying the journal and append
      ` · unboxed` to the tier line and the vouch line when an entry has
      `gate` true with `harness` or `open`; ` · boundary not recorded`
      when the manifest carries `hands` and no `boundary`, and likewise
      for an entry outside the vocabulary or without its tag; nothing for
      a run that boxes nothing; a docs-tier preflight run read the same
      way on its own line; bash 3.2-compatible, no `mapfile` —
      boundary-readouts / The delivery gate's check summary says unboxed.
- [x] 10.14 Gate-script tests in
      `crates/brokkr-cli/tests/delivered_by_brokkr.rs`: a harness-judged
      run ending both lines with `· unboxed`; a `namespace` run carrying
      neither; an old journal and a malformed entry each ending with
      `· boundary not recorded`; and the binding pinned in
      `crates/brokkr-cli/tests/contributing.rs`, which already reads the
      script — boundary-readouts / The delivery gate's check summary says
      unboxed.

## 11. Re-pin, and the shipped bundles under `harness`

- [x] 11.1 Re-pin every moved witness digest in
      `crates/brokkr-runtime/tests/witness_digests.rs` and every moved
      compose pin in `crates/brokkr-runtime/src/bundle/compose_tests.rs`
      from the tests' own left/right pairs, once, at the end —
      boundary-manifest-pin / Every pinned digest that moves is re-pinned
      with 0046 as the reason.
- [x] 11.2 Name decision 0046 in each pin file's doc comment as the reason
      every bundle declaring hands moved once (the manifest's `boundary`
      key) and the reason an inline gate on codex or claude moved (the
      adapter's `hands.harness`) —
      boundary-manifest-pin / Every pinned digest that moves is re-pinned
      with 0046 as the reason.
- [x] 11.3 Check the fixed points: every pinned bundle that declares no
      hands and pins neither changed adapter keeps its digest —
      boundary-manifest-pin / Every pinned digest that moves is re-pinned
      with 0046 as the reason.
- [x] 11.4 Test that every bundle under `recipes/` and `bundles/` without a
      dialect step — eleven of the thirteen — compiles under `harness` in
      a realm declaring the openspec dialect, each hands site's manifest
      `boundary` entry reading `harness`, and that `recipes/triage` and
      `recipes/night-shift` refuse naming a dialect step and decision
      0046 ruling 4 (DD8), against a scratch copy of the shipped adapter
      library with the codex and claude `hands.harness` members planted
      as fragments — the shipped files themselves once the measurement
      lands —
      gate-boundary-policy / Every shipped bundle compiles under harness
      once the fragments are measured.
- [x] 11.5 Test the measured-gap path: with claude's `work` declared
      unsupported, a fixture bundle seating the chief architect as a work
      seat with hands refuses under `harness` naming `claude`,
      `hands.harness.work` and the site, `recipes/triage` and
      `recipes/night-shift` refuse, and every other shipped bundle still
      compiles —
      gate-boundary-policy / Every shipped bundle compiles under harness
      once the fragments are measured.
- [x] 11.6 Report in the completion note, for the operator to rule on,
      ruling 6's promise as unmet while a claude member is undeclared or
      unsupported: name the members, the bundles that refuse under
      `harness` and why, the measurement as the operator's with the
      recipe, the candidates and the version to measure against, that
      landing it is a data change to `adapters/claude.json` moving the
      pins that name its digest, codex's door measurement as pending (task 8.9), and the dialect
      step's refusal under `harness` with the amendment DD8 records as
      the operator's to accept or refuse. Never widen a rule, seat another provider, declare an
      unmeasured fragment, or report the run blocked to make the shipped
      bundles compile (DD20) — gate-boundary-policy / Every shipped bundle
      compiles under harness once the fragments are measured.
- [x] 11.7 In the same test as 11.4, as its second half (DD20, D27):
      with the adapters as they stand in the tree, compiling every bundle
      under `recipes/` and `bundles/` under `harness` in the same realm
      refuses exactly four, each naming the ground the compiler reaches
      first — `bundles/self` at `review` and `recipes/panel-review` at
      `review:correctness` naming `claude`, `hands.harness.gate` and the
      site; `recipes/triage` and `recipes/night-shift` naming the
      `analyze` sequence's `check` step, decision 0046 ruling 4 and
      decision 0042 ruling 4, because phases compile in name order and
      that step comes before any claude link — and compiles every other;
      the pin moves when the operator's measurement lands and, for the
      last two, when a decision admits the dialect step —
      gate-boundary-policy / Every shipped bundle compiles under harness
      once the fragments are measured.

## 12. The guides and the erratum

- [x] 12.1 `docs/guides/provider-adapters.md`: document `hands.harness`
      with `gate`, `work` and `result`, the `{result_path}` token and the
      two workspace tokens refused there, the three-shape convention,
      codex's fragments and its `last-message` door — the door's
      measurement recorded when made and named as pending and the operator's until then (8.9) — and, per claude member, the
      measurement with the claude version and what the mode denies and
      allows, or that it is undeclared pending the operator's
      measurement with the candidates and the recipe; its doctor section names the `boundaries`
      line and what *offered* and *ready* mean —
      boundary-guides / The guides document the boundary and never lose a
      section.
- [x] 12.2 `docs/guides/recipe-authoring.md`: the `hands` row stops saying
      Linux only, says the boundary lives in the realm and that a bind's
      `mask` is declared and not enforced under `harness` and `open` and
      that clearing the environment confines nothing on disk (DD10); the
      `driver.confine` row says the field is refused and points at
      decision 0046 ruling 5 —
      boundary-guides / The guides document the boundary and never lose a
      section.
- [x] 12.3 `docs/guides/quickstart.md`: rewrite the platform paragraph to
      say a realm on macOS or Windows declares `boundary: harness` today,
      what that means (judged under the harness's own sandbox, rendered
      *unboxed*), that `namespace` is the default and needs bubblewrap,
      that `seatbelt` and `container` are named by 0046 and refuse at
      start until slices (ii) and (iii), and which shipped bundles run
      under `harness` today — the nine whose hands sites are their own
      `./` exec gates — and which four refuse by name and on which
      ground (the claude measurement for `bundles/self`,
      `recipes/panel-review`, `recipes/triage` and `recipes/night-shift`;
      the dialect step for the last two), pointing at the pin test of
      11.4 and 11.7 as the record (DD8, DD20); and its `rerun` line says
      the rerun compiles in the discovered realm as `run` does —
      boundary-guides / The guides document the boundary and never lose a
      section.
- [x] 12.4 `docs/guides/journal-and-verification.md`: add the unboxed
      rendering and what `boundary` means on a seat record and on
      `effect/started` —
      boundary-guides / The guides document the boundary and never lose a
      section.
- [x] 12.5 `docs/guides/read-surfaces.md`: refresh the seats-table example
      from the renderer's header line showing the `boundary` column beside
      `model`, and add `brokkr seats` to the verb list beside `inspect`,
      its `--json` named as `inspect`'s own view model —
      boundary-guides / The guides document the boundary and never lose a
      section.
- [x] 12.6 `docs/guides/repository-layout.md`: name `boundary` beside
      `house` and `dialect` in the `realms.json` row and the four new
      contract files in the `contracts/` row —
      boundary-guides / The guides document the boundary and never lose a
      section.
- [x] 12.7 `docs/guides/driver-authoring.md` and `ARCHITECTURE.md`: stop
      describing the `docker run` wrapper as a trust class and point at
      the boundary; driver-authoring's opening paragraph says `brokkr
      hands exec` of `namespace` and adds that under `harness` and `open`
      the same `exec` dispatch runs with no verb of Brokkr's around it, in
      a fixed environment, the network narrowed on Linux where `unshare`
      permits, that the declaring layer is re-walked at every unboxed spawn and a
      pinned byte that moved refuses the gate, the residual being the
      interval between the re-walk and the `exec` (DD9),
      and that the `hands` subcommand gains no verb for it —
      boundary-guides / The guides document the boundary and never lose a
      section.
- [x] 12.8 State in no guide that the network was off under `harness` or
      `open` — the prefix is described as a narrowing the engine attempts
      on Linux (DD15) —
      boundary-guides / The guides document the boundary and never lose a
      section.
- [x] 12.9 `contracts/README.md`: add rows and a paragraph for
      `realms.v4`, `run-manifest.v9`, `seat-record.v4` and
      `effect-boundary.v1` in the style of the rows before them —
      boundary-guides / The guides document the boundary and never lose a
      section.
- [x] 12.10 Add a `## Erratum` heading to
      `docs/decisions/0046-the-boundary-is-named.md` followed by exactly
      one line saying rulings 3 and 6 name `seat-record.v3`, that v3
      already exists (landed by #202 under decision 0034 rulings 6 and 7,
      the dialect state), and that the field therefore lands as
      `seat-record.v4`, additive on v3, nothing else renumbered; the
      `Status:` line and every other line untouched, and
      `docs/decisions/README.md` unchanged —
      boundary-guides / Decision 0046 carries the erratum.
- [x] 12.11 Test that the guides keep every section they had and gained
      the rows above, the two blueprint pages of 12.12 included, and that
      the decisions index test passes without a change to
      `docs/decisions/README.md` —
      boundary-guides / The guides document the boundary and never lose a
      section; Decision 0046 carries the erratum.
- [x] 12.12 `docs/extension-model.md` and `docs/target-architecture.md`:
      the seat-field `trust` row says the wall is the realm's `boundary`
      (decision 0046) and the tier decides what is mounted inside it; the
      runner table's `policy-confined` row points at decision 0046's
      `container` boundary — declared by the realm, refused at start
      until slice (iii) measures it — and its `public-evidence-only` row
      names the same boundary for its container form; no row or section
      removed, each page's status line untouched (D30) —
      boundary-guides / The guides document the boundary and never lose a
      section.

## 13. Gates and the commit

- [x] 13.1 `cargo fmt --all -- --check` clean — every requirement of this
      change (the house rule that gates the work).
- [x] 13.2 `cargo clippy --workspace --all-targets --all-features -- -D
      warnings` clean — every requirement of this change.
- [x] 13.3 `cargo test --workspace --no-fail-fast` green as one
      whole-workspace run — the house gate; a crate-scoped run may
      precede it while iterating and never stands in for it — every
      requirement of this change.
- [x] 13.4 `cargo package --workspace --allow-dirty` verifies — every
      requirement of this change.
- [x] 13.5 `brokkr compile --bundle bundles/self` and `--bundle
      bundles/verify` both compile — realm-boundary / brokkr compile prints
      the boundary under each hands site.
- [x] 13.6 `scripts/coverage-exact.sh` at literal 100% of lines and
      branches, every new branch reached by a test and every repeated mark
      folded into one helper — every requirement of this change.
- [x] 13.7 Fold the change into the living truth with the dialect's
      archive operation, `openspec archive boundary-named-slice-i --yes`,
      so the eight deltas seed `openspec/specs/` — the first capabilities
      written against it (decision 0042's addendum; the dialect's
      `archive`) — and re-run `openspec validate --archived --strict
      --no-interactive` — every requirement of this change.
- [x] 13.8 Commit the work in the repository's message style, and never
      push — the house rule that closes the work.
