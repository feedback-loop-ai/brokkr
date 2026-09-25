# 0071 — The architectural principles: how Brokkr's code is shaped, which gate holds each rule, and which charter judges it

Status: proposed (drafted in the operator's session, 2026-09-25; epic #330, issue #331)
Date: 2026-09-25

## Context

Brokkr writes down its control-plane laws and enforces them: fail closed
(0001), a linear machine (0002), strict evaluation (0004), input
provenance (0007). It has never written down how its code is shaped.

These terms return no hits in `docs/house-rules.md`, `CONTRIBUTING.md`,
`ARCHITECTURE.md`, `docs/target-architecture.md` or
`docs/extension-model.md`: SOLID, DRY, YAGNI, single responsibility,
function size, complexity, CRAP, trait and enum. "Single responsibility"
appears once in the whole decision record, in passing (0041). The house's
most important design choice, closed enums and data instead of traits,
exists only as a measurement in #288: 81 enums against one production
trait. Its acceptance 6 asks for that choice to be "stated once,
deliberately".

So every implement, design and review seat builds and judges against its
model's own taste. The 2026-09-25 sweep (epic #330) measured what that
produced:

- **The architecture is sound.** The core is pure, the crates form a
  one-way graph, the journal replays byte for byte, and extension happens
  through enums and data.
- **The implementation carries concentrated debt:**
  - 64 production functions over 100 lines, including a 908-line command
    dispatcher;
  - about 1,140 `serde_json::Value` references carrying protocol
    vocabulary between crates as strings;
  - policy and agents copied rather than composed (fast's constitution
    in five copies);
  - adding a harness costs about twenty code edits across six crates;
  - 38% of documented claims are not what the code does.

None of this is a defect of one change. Each is what a shared standard
would have prevented, and there was none.

This decision states the principles once, as rules a seat can apply and
a reviewer can cite. Each names the gate that holds it, or the charter
that judges it where no deterministic gate can. A principle with neither
is a wish, and none is written here.

## Why configuration and closed enums, not inheritance

The principles below rest on one choice, so it is argued here once.

- **No fragile base class.** Rust has no implementation inheritance, so a
  subclass can never silently change what a parent does.
- **Closed sets are enums.** Brokkr's closed sets (phases, event types,
  verdicts, results, boundary words, harness kinds) are enums matched
  exhaustively. Adding a variant is a compile error at every site that
  must care, so the compiler writes the to-do list that inheritance hides.
- **Open sets are data.** Operators extend models, recipes, realms,
  agents and adapters, and those must be data. A run must replay byte for
  byte, and data can be hashed into the manifest, validated against a
  schema before anything runs, diffed in review, and pinned. A subclass
  can be none of those things.
- **The plugin boundary is a process, not an ABI.** A harness is reached
  over NDJSON on stdio. That boundary is language-neutral, isolates
  crashes, and can be boxed, whatever the harness is written in.

The choice has a price the sweep also measured. The benefit holds only
if the data is typed where it enters. Configuration that grows
conditional merges and overrides can turn into an interpreter nobody
designed; `merge_layer` at 388 lines and `assemble` at 626 are where
that began. Rulings 3 and 4 pay that price.

## Rulings

1. **Functional core, imperative shell.** `brokkr-core` and
   `brokkr-view` perform no I/O, clock reads, randomness, environment
   reads or process execution. Effects live in the crates above them and
   are journaled. A clock or an environment value reaches a pure
   function as an argument.

   *Violating it looks like:* nothing today. Purity holds, but only by
   review.

   **Enforcement binding:** #336. A per-crate `clippy.toml` with
   `disallowed-methods`/`types`/`macros` for core and view, plus a `git
   grep` backstop and a dev-dependency layering test. `cargo-deny` bans
   with `wrappers` encode the one-way crate graph.

2. **Closed sets are enums matched exhaustively; open sets are data; a
   trait is the exception and names its seam.** A `match` over a closed
   enum has no wildcard arm that decides behaviour. Anything an operator
   extends is schema-validated data, digested into the manifest. A trait
   is introduced only for a seam with more than one real implementation
   (today: `ProducerTransport`), and its doc comment names that seam.
   Dependency inversion otherwise uses the house's closure seams
   (`*_with`, `*_in`), as it already does.

   *Violating it looks like:* `AdapterKind::supports()` answering
   `_ => ["resume"]`, so a new kind advertises resume nobody decided; and
   `is_gate_class` / `collect_unpinned` walking raw JSON beside the
   typed tree, so a new body form escapes decision 0031's refusal (#347,
   #349).

   **Enforcement binding:** judged by the reviewer and chief-architect
   charters (#333). The exhaustive-match half is checked by the compiler
   once wildcards are gone.

3. **Parse, don't validate: a value is typed where it enters.** Data
   from a wire, a file or a harness is parsed once into a type at the
   boundary. `serde_json::Value` is allowed only at that edge, and it is
   never threaded through logic, stored in state, or read back by a
   string key. A doubly-optional field, a string vocabulary or a
   `format!("{:?}")` in a digest is a missing type.

   *Violating it looks like:* `EventEnvelope.payload` read by
   `payload_str(event, "effect_id")`; seat input built with `json!` and
   re-read by key 56 times, with `format!("{aggregate:?}")` inside a
   digest; `Option<Option<String>>` for the gate-head span (#345, #346).

   **Enforcement binding:** #338's `cargo-public-api` snapshot holds the
   count of `serde_json::Value` in `brokkr-core`'s public API as a
   ratchet (11 today, never more). Judged by the reviewer charter.

4. **One responsibility per function, measured.** A function holds one
   level of abstraction. New and changed functions stay within the
   ceilings below. Today's offenders are a committed baseline that may
   only shrink. The **provisional** ceilings, which the operator rules
   final from #335's baseline, are:
   - 100 lines per function, as clippy counts them;
   - nesting depth 5;
   - 7 parameters;
   - 800 lines per production file;
   - cyclomatic complexity per function held by `cargo-crap` as a
     ratchet against the committed baseline, not an absolute number (at
     100% coverage CRAP equals CC).

   Clippy's `cognitive_complexity` is not used; its own documentation
   says it does not measure what its name claims.

   *Violating it looks like:* `run_with` at 908 lines; `assemble` at
   626; `execute_sequence` at 424 (#288).

   **Enforcement binding:** #335 records the baseline. #337 enables the
   clippy lints, with today's offenders as `#[expect(…, reason)]` that
   `unfulfilled_lint_expectations` forces out as they are fixed. #338
   adds the `cargo-crap` and file-size ratchets.

5. **Don't repeat yourself, in code and in data.** A fact has one home.
   In code, a rule written twice (a vocabulary, a fence skeleton, a stderr
   drain) becomes one function or one table. In data, a recipe or agent
   that differs from another by a model, a seat or a table entry is an
   overlay (`extends`), never a copy. A copy that must exist, such as a
   script whose bytes a bundle pins, is held to its source by a test.

   *Violating it looks like:* fast's 21-rule constitution in five copies,
   two of them byte-identical; fifteen `gpt-flash-*` agents restating
   their base offices, five of them silently dropping tools or hands; the
   verify seat script in six copies, all still carrying #287's bug
   (#359, #360).

   **Enforcement binding:** #338's `jscpd` ratchet, run separately over
   production code, tests and data, fails on a new clone. The recipe
   parity test from #359. Judged by the position-simplicity and reviewer
   charters.

6. **You aren't going to need it, and retirement is a promise.** No
   alias, agent, contract version, flag or `pub` item without a
   consumer. A deprecation names the release that removes it and keeps
   to it. Code for a host Brokkr does not support is not written
   (decision 0063).

   *Violating it looks like:* `legacy.rs` promised "one more release" in
   v0.5.0 and was still present in v0.11.0; `intake-sdd` is seated by no
   recipe; 14 of 17 dsh aliases are hired by nothing; Windows code grew
   six hours after 0063 was enacted (#355, #356).

   **Enforcement binding:** #337's `unreachable_pub`; #338's
   `cargo-shear`; a test that every agent is seated or catalogued (#355);
   the PR-delta refusal of new `cfg(windows)` (#356).

7. **One derivation, many surfaces.** A fact shown on two surfaces is
   derived once in `brokkr-view` and only painted by the CLI, the TUI and
   the browser. This restates decision 0013 as a design rule. A
   control-plane admission never depends on a display crate.

   *Violating it looks like:* per-seat cost summed by the CLI and
   last-attempt in the view, until #376 was fixed; scoping implemented
   once in `render.rs` and differently in `ui.html`; supersede admission
   defined in `brokkr-view` and called by the engine (#351).

   **Enforcement binding:** judged by the reviewer charter. The #351
   test that the CLI and `ui.html` carry no predicate the view exports.

8. **Errors are types; text is for people.** Callers and tests branch on
   error variants, never on message substrings. A variant's `Display`
   keeps the operator-facing text, and one exact-text test per module
   pins it.

   *Violating it looks like:* 148 `Result<_, String>` signatures in
   `brokkr-protocol`, and about 700 `.contains("…")` assertions that let a
   mutant firing a different check survive (#353).

   **Enforcement binding:** judged by the reviewer charter; #289's
   mutation testing measures what remains.

9. **A test proves a behaviour, and is shown to bind.** Every new test
   asserts exact values or variants, never `is_err()` alone. Its author
   shows it binds: a compiling mutation that removes the behaviour makes
   it fail, and the failure is recorded. Shared fixtures are built once,
   with builders and one environment guard per test binary, never
   hand-restored process state.

   *Violating it looks like:* one panic poisoning `ADAPTER_ENV` and
   failing about seventy unrelated tests (#357, and red `main` on
   2026-09-25); "removal control" appears 77 times in change documents
   and in no rule.

   **Enforcement binding:** #289's `cargo-mutants` (report-only first,
   then no new survivors); #357's environment guard. The implementer
   charters require the removal control (#333).

10. **Patterns where they earn their place, in the house's forms.** A
    pattern named in a commit is visible in the code. These are the forms
    the house uses:
    - **Strategy over a closed enum:** a harness is one module behind
      `AdapterKind`.
    - **Command:** an enum variant plus a handler function; no
      `Box<dyn Command>`.
    - **Parameter Object:** it replaces a long parameter list or an
      `#[allow(clippy::too_many_arguments)]`.
    - **Builder:** for argv and mount tables whose order matters.
    - **Split Phase:** separates observing the world from deciding on it.

    A trait-object registry for a closed set is rejected, as is a
    pattern introduced for its name.

    **Enforcement binding:** judged by the chief-architect and reviewer
    charters (#333).

11. **Which principle is gated and which is judged.** Rulings 1, 4, 5
    and 6 are held by deterministic gates, with the tools pinned and each
    ratchet a committed baseline that may only shrink (track B of #330).
    Rulings 2, 3, 7, 8 and 10 are judged by charters. A judgment is
    advisory where the gate is authoritative: a finding a gate would have
    caught is the gate's verdict, not the reviewer's.

    **Enforcement binding:** the house rules carry the Rust-specific form
    of every ruling (#332). The office charters carry the portable form
    and one severity table for judges (#333). The recipe roles stop
    restating the house and reach the same text (#334).

## What this does not do

- It moves no code. Each violation it cites is already a story in epic
  #330, and each is judged against these rulings when it lands.
- It relaxes nothing. Decisions 0001 (no model repair of the control
  plane), 0003 (native Rust runtime), 0009 (Rust only) and 0063 (Linux
  and macOS) stand as the floor these principles are built on.
- It does not fix the numbers. Ruling 4's ceilings are provisional until
  the operator rules them from #335's measured baseline.

## Consequences

- A seat can cite a ruling number for a design finding the way it cites
  0001 for a fail-open one, and two runs of the same office judge the
  same change against the same text.
- #288's acceptance 6 ("the enum-over-traits bet is stated once,
  deliberately") is met by ruling 2 and the section before the rulings.
- The epic's enforcement stories (#335–#343) become the bindings of
  rulings 1, 4, 5 and 6, and its refactor stories (#344–#356) are judged
  against rulings 2, 3, 7, 8 and 10.
- A future change that wants to break a ruling does so by a new decision
  that says so, not by a seat's taste.
