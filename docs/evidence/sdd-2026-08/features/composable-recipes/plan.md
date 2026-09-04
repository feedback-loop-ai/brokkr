# Implementation Plan: Composable recipes

**Feature slug**: `composable-recipes`
**Spec**: [spec.md](spec.md) · **Tasks**: [tasks.md](tasks.md)

## Approach

Composition is a **pre-pass**, not a rewrite of compilation. One new
module owns every line that knows the word `extends`; `Bundle::compile`
changes its opening to consume that module's output and then proceeds
verbatim. The rebase over the parallel 0016 agent slice is therefore
mechanical: the agent slice edits seat *parsing*, this slice edits what
gets handed *to* seat parsing, and the two touch disjoint code.

```rust
// crates/brokkr-runtime/src/bundle/compose.rs
pub struct Ancestor { pub name: String, pub dir: PathBuf, pub digest: String }

pub struct Resolved {
    /// Flat bundle.json document. Seat values are byte-identical to the
    /// layer that supplied them.
    pub document: Value,
    /// Flat policy table.
    pub table: Value,
    /// Name -> layer index. Name-level only; never derived from a value.
    pub seat_origin: BTreeMap<String, usize>,
    /// Leaf first, then each ancestor. Empty for a non-composed recipe.
    pub chain: Vec<Ancestor>,
    /// Every layer's directory, leaf first, deduplicated.
    pub roots: Vec<PathBuf>,
}

pub fn resolve(leaf: &Path) -> Result<Resolved, CompileError>;
```

`resolve` reads named files in a name-determined order — never
`read_dir` order, the clock, or the environment — and returns before any
seat is interpreted. That placement is what makes AC-1 (purity) a
one-line test: `parse_command` expands `{brokkr}` to
`std::env::current_exe()`, so any resolver returning parsed seats could
never be byte-stable across machines.

### Order of work

The tasks are sequenced so the **hard regression lands first**. Task
group 1 introduces `resolve` on the no-`extends` path only, and pins the
five existing digests as goldens, before any merge logic exists. Every
later group runs against those pins.

### The seven deliverables, mapped to the change

| Deliverable | Where it lands |
|---|---|
| 1 `extends`, chains, cycle naming the loop | `compose.rs`: name grammar, parent-dir lookup, visited list, depth cap 8 |
| 2 Merge rules with explicit override / removal | `compose.rs`: kind-keyed `override` / `remove`, wholesale replacement |
| 3 Policy composition, lint on resolved table | `compose.rs` prepend + remove-then-prepend; `bundle.rs` lint call site **unchanged** |
| 4 Flat at run time | `Bundle::compile` consumes `Resolved`; nothing below the resolver knows `extends` |
| 5 Pinning + chain | `manifest_for` gains `@compose/` entries and one refusal |
| 6 Purity + goldens | `bundle/compose_tests.rs` |
| 7 Operator surfaces | extracted printer in `main.rs`, `recipes::show` |
| 8 Proof recipe | `recipes/sdd-paranoid/` |

## Files this touches

| Path | Change |
|---|---|
| `crates/brokkr-runtime/src/bundle/compose.rs` | **New.** The resolver: name grammar, library lookup, cycle/depth detection, merge, markers, chain, origins, roots. Every composition error message lives here. |
| `crates/brokkr-runtime/src/bundle.rs` | `Bundle::compile` calls `resolve` and parses the returned documents; per-seat `parse_role`/`parse_command` take the origin layer's dir; `Bundle` gains `roots`; `manifest_for` takes the chain and gains the `@compose/` refusal; one `map_err` wrapping downstream errors with the chain. |
| `crates/brokkr-runtime/src/bundle/compose_tests.rs` | **New.** AC-1..AC-18, AC-21..AC-25. |
| `crates/brokkr-runtime/src/bundle/tests.rs` | Regression goldens for the five existing bundles; `roots == [dir]` for non-composed. |
| `crates/brokkr-runtime/src/engine.rs` | `confined_command` takes `&[PathBuf]` and mounts every root; three call sites (`609`, `843`, `1031`) pass `&self.bundle.roots`. |
| `crates/brokkr-runtime/src/engine/tests.rs` | AC-20, AC-26, AC-27. |
| `crates/brokkr-cli/src/main.rs` | Extract the `Compile` arm's printer; add `RecipesCmd::Show`; both emit `composed_from` when non-empty. |
| `crates/brokkr-cli/src/recipes.rs` | `show(name, dir)`; `list` unchanged (its warning path already covers a missing base). |
| `crates/brokkr-cli/src/tests.rs`, `crates/brokkr-cli/tests/recipes.rs` | AC-28..AC-31. |
| `recipes/sdd-paranoid/` | `bundle.json` with `extends` + one override, two role files, `README.md`. |
| `README.md`, `ARCHITECTURE.md` | The strategy-loop paragraph gains composition. |

**Untouched, by constraint**: `contracts/` (no new version — see below),
`policy/phase-machine.json`, `reference/`,
`fixtures/evaluator/corpus.ndjson`, `Cargo.toml` dependency lists.

## Risks and mitigations

| # | Risk | Mitigation |
|---|---|---|
| **R1** | `files` widens in meaning from "path → digest of file bytes" to "identity ingredient → digest". The v1 description sentence no longer covers every entry. | It validates against **both** frozen schemas unmodified (`files` constrains values, not keys) and is the only shape that survives the v2 round-trip in `dispatch.rs:422/445`. `manifest_for` refuses any real file under `@compose/` so the namespace cannot be forged from disk (AC-25). Nothing about this blocks a future cosmetic v3. |
| **R2** | A derived recipe does not compile outside its library — moved away from its siblings it fails. | True by construction of "state only your differences". The error names the leaf file, the base name, and the directory searched (AC-5). |
| **R3** | Confine mounts widen to ancestor directories. | The operator explicitly chose to derive from that bundle. The alternative is a confined inherited seat whose role file is not mounted — a silent run-time break hours into a run (AC-20). |
| **R4** | A case-insensitive filesystem could match `extends: "sdd"` against a directory named `SDD`, recording a chain name that does not exist on Linux. | The lowercase-only name grammar (AC-6) closes the half that is testable. The directory-entry check is **declined**: its error arm is unreachable on the Linux CI that enforces literal 100% coverage, and an unreachable arm fails the gate. Residual accepted and recorded here rather than hidden. |
| **R5** | An inherited non-`./` argv entry (`recipes/sdd`'s `bash recipes/sdd/drivers/speckit_check.sh`) still resolves against the run's workdir. | Not a regression — equally true of `sdd` today — and rewriting it would require reading inside a seat. Stated in `spec.md` as inherited behaviour so it is a documented property, not a surprise. |
| **R6** | `parse_role` accepts `dir.join("../../x")` if the file exists; with several roots a seat could reach into another layer's tree or outside every mount. | Pre-existing for single-root bundles, and the framing binds this slice to the composition layer. Declined here; flagged as a candidate follow-up. All five shipped recipes use plain `roles/*.md`. |
| **R7** | The derived recipe's `README.md` is inside its digest, so writing it after pinning the golden breaks the golden. | Task ordering: `recipes/sdd-paranoid/` including its README lands **before** the composed golden is pinned (T13 before T14). |
| **R8** | Adding a recipe under `recipes/` changes `brokkr recipes list` output and may break CLI assertions that pin it. | T13 updates those assertions in the same commit; AC-31 covers the missing-base warning path. |
| **R9** | An overridden seat with a malformed body fails with an error naming the phase, not the file. | One `map_err` on the composed path appends the chain (AC-18) — one arm, not per-lint plumbing. |
| **R10** | The parallel 0016 slice also edits `bundle.rs`. | The resolver is a separate file; `bundle.rs`'s edits are the opening of `compile`, three `dir` → origin-dir argument swaps, and `manifest_for`. Seat *parsing* — what 0016 changes — is untouched. |

## Invariants to hold, each with a test

- **I1 Opacity.** The resolver never reads inside a seat value (AC-11).
- **I2 Origin is name-level.** Provenance is recorded at the level the
  merge already operates on (AC-19).
- **I3 No-`extends` recipes take no new code path and no new I/O**
  (AC-21).
- **I4 Purity.** `BTreeMap` throughout; canonicalized absolute paths
  never enter the manifest (AC-1).
- **I5 Fail closed.** Every ambiguity is a refusal naming the evidence;
  there is no "derived wins by default" anywhere (AC-8, AC-9, AC-10).
- **I6 The digest covers every byte of every layer.** No exclusions.
- **I7 Composition is invisible below the resolver.** `compose.rs` is
  the only module that knows the word `extends` (AC-4 by construction).
