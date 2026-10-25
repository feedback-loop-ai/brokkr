# 0091 — A portable recipe's verify and ship are the folder's own seats, pinned from the folder at compile

Status: accepted — operator ruled 2026-10-20 (PR #793), every choice asked taken as recommended
Date: 2026-10-20

Built: built — slice 3a (#796) the portable overlays' run-local wording; slice 3b ruling 1; slice 3c rulings 3 and 4 as data; slice 3d's fixes, the acceptance's gaps 1 to 5 (#852, #859, #872) and `brokkr init --cli` for a CLI PATH's order cannot reach (#882); the acceptance passed on main 59a8bf9d on 2026-10-25: six fresh folders under claude, codex and dsh, each a Cargo.toml or package.json, reached verify and passed it, and one with no marker failed closed with the fill-in text (#717)
Amends: 0048, 0087

## Context

This is the verifier half of #717's slice 3. Decision 0087 (accepted by
the operator on 2026-10-18, PR #740) took its
"Fast's verifier and shipper" choice as recommended: slice 1 leaves both,
says so in fast's README, and slice 3 moves Brokkr's verifier to
`brokkr-fast` and gives `fast` a portable one. Its record on #717 left one
question open, decision 0048's: how a shelf recipe's pinned script
reaches the folder's own `scripts/verify-seat.sh` under the spawn pin.
This decision proposes the answer and the build. It builds nothing.

**The integrity property this is held to.** Decision 0048 ruling 2: a
spawn runs only bytes the compile pinned. An exec seat names exactly one
`./` script (`pinned_script`, `crates/brokkr-runtime/src/bundle.rs:3135`),
joined to the declaring layer's directory (`expand_command`,
`crates/brokkr-runtime/src/bundle/expand.rs:21`). Every component is a
plain name, so `..` is refused (`bundle.rs:3202`). Before every unboxed
spawn the directory holding the script is walked again and compared with
the layer's compiled file map (`script_directory`,
`crates/brokkr-runtime/src/engine.rs:2835`; `layer_drift`,
`bundle.rs:4386`). An ancestor's map is kept beside its digest for that
walk (`Ancestor::files`, `crates/brokkr-runtime/src/bundle/compose.rs:56-66`).
A command run as text is refused because "`bash -c` would run text, not
pinned bytes" (`bundle.rs:3148`).

**What a folder holds today, measured at ebe818f7.** The CLI was built
in this worktree, and `brokkr init <folder>` was run in fresh `git init`
folders under `.forge/measure/`, with a search path holding only a stub
for one agent CLI (`cargo run --locked -p brokkr-cli --config
'env.PATH.value="<stub dir>:/usr/bin:/bin"' --config env.PATH.force=true
-- init <folder>`). It wrote these folders:

| CLI | Stack marker | `init` digest |
|---|---|---|
| claude | none | `77653b7fd6c2…`, as 0087 measured at ae90406a |
| claude | `Cargo.toml` | `3323dd6c3ee7…` |
| claude | `package.json` | `14423b7e106f…` |
| codex | `Cargo.toml` | `489f9777ca86…` |
| dsh | `Cargo.toml` | `ba67812ddc16…` |

Every folder holds its own verify and ship seats in `.brokkr/bundle.json`
and its own scripts beside them in `.brokkr/scripts/`. Both seats are exec
gates written by `bundle_json` (`crates/brokkr-cli/src/init.rs:218-248`).
The verify seat has results `pass` and `fail` and limits of 2 attempts
and 3600 seconds, and it dispatches `./scripts/verify-seat.sh`. Its hands
are `workspace` with no network and the stack's binds (`stack_binds`,
`init.rs:205-214`). Those are `~/.cargo` (an overlay masking
`credentials.toml` and `credentials`) and `~/.rustup` read-only in the
`Cargo.toml` folder, and none in the `package.json` folder or the
folder with no marker. The ship seat has results `ready` and `shipped`
and limits of 2 attempts and 1800 seconds, dispatches
`./scripts/ship-seat.sh`, and binds nothing. `bundle_json` takes no
agent CLI, and `cmp` found the three `Cargo.toml` folders'
`bundle.json` and `ship-seat.sh` identical under claude, codex and dsh.
Their `verify-seat.sh` differ in one line, the pass note's boundary
claim. Under codex it reads "passed unboxed, with no network denial
confirmed", because `init` wrote `"boundary": "harness"` into that
folder's `realms.json`. The claude and dsh folders declare no boundary.

The folder's verifier runs the two commands `init` read from its stack
table (`verify_script`, `init.rs:1345`; the command lines are
`init.rs:1376-1377`):

| Stack | `test_command` | `lint_command` | Measured |
|---|---|---|---|
| `rust` (`init.rs:891`) | `cargo test --workspace` | `cargo clippy --workspace --all-targets -- -D warnings` | Both were run unboxed and offline in the scratch `Cargo.toml` folder. The first printed `test result: ok. 0 passed; 0 failed`, the second `Finished`. |
| `node/npm` (`init.rs:934`) | `npm test` | `npm run lint` | Not run: this seat may not invoke `npm`. |
| none | empty | empty | Read, not run. The script writes `fail` with `no stack was recognized; fill in .brokkr/scripts/verify-seat.sh before running this gate` (`init.rs:1379`). |

Then `brokkr recipes add <checkout>/recipes/fast --name fast` was run in
the three claude folders. It installed and compiled, as `fast` digests
`5708720ca534` (no marker), `87c29d107eb0` (`Cargo.toml`) and
`b91fe3594efd` (`package.json`). The shelf copy at
`.brokkr/recipes/fast/` holds `README.md`, `bundle.json`, `policy.json`,
`shipper.md`, `scripts/verify-seat.sh` and `scripts/ship-seat.sh`.
Fast's verify seat dispatches `./scripts/verify-seat.sh`
(`recipes/fast/bundle.json:32-42`). On the shelf that path resolves
inside the copy, to Brokkr's own verifier, under the Rust binds
(`bundle.json:47-48`). That verifier runs `bash scripts/lint-non-rust.sh
--seat` (`recipes/fast/scripts/verify-seat.sh:80`), and no folder has
that file: `ls` of `scripts/lint-non-rust.sh` in the scratch
`Cargo.toml` folder printed `No such file or directory`. Its last check
is `cargo run -p brokkr-cli -- compile --bundle recipes/self` (line 92),
and a folder has no `recipes/`. The shipper names
`.forge/ledger/$run_id.md` (`recipes/fast/scripts/ship-seat.sh:61`). In
a folder the `ledger` verb writes `.brokkr/state/ledger/<run>.md`
(`run_local`, `crates/brokkr-cli/src/folder.rs:525-528`;
`crates/brokkr-cli/src/ledger.rs:163-177`) and prints the path it wrote
(`crates/brokkr-cli/src/verbs/readouts.rs:44`). The folder's own shipper
names that path (`init.rs:275`). So the folder already holds the
verifier and shipper a portable `fast` needs. The shelf copy cannot name
them: its declaring layer is `.brokkr/recipes/fast/`, and `..` is
refused.

**What inherits fast's verify and ship.** `node` declares its own verify
seat and inherits fast's ship seat with its limits overridden
(`recipes/node/bundle.json:6-10`). `brokkr-fast` overrides `implement`
and `review` only. `landing` and the three `wager-harness` recipes
extend `brokkr-fast`. `panel-review`, `triage`, `release`,
`review-first` and `standby` extend `fast` directly and override no
verify seat. `self` reaches fast through `panel-review`, and `gpt-flash`
and `night-shift` through `triage`. Every one of these runs Brokkr's
verifier today. `preflight`, `research`, `research-dsh` and `verify`
compose no `fast`.

**The forms weighed**, each against 0048's property:

- **(A) The folder's script, named through a placeholder.** The recipe
  writes a new token, such as `{folder}/scripts/verify-seat.sh`, and the
  compile pins the folder's `scripts/`. Integrity holds once the compile
  keeps that map and the spawn walks it. But the script is half the seat.
  The box is the other half, and the recipe would keep its own. Fast's
  verify binds the Rust toolchain, so an npm folder's verifier would run
  in a box holding `~/.cargo`, and a go or uv folder's would lack its
  caches. Binding every stack's caches widens every box. A second
  spelling of the script token also breaks the rule `pinned_script`
  states, that a token spelled any other way than `./` is refused.
- **(B) A portable verifier inside the recipe.** This needs no engine
  change and is fully pinned. It finds the stack by its markers and runs
  that stack's checks. But it restates `init`'s stack table (`STACKS`,
  `init.rs:889`, nine rows plus the orchestrators) in shell. That is a
  second home for one fact, which decision 0071 ruling 5 refuses unless a
  test holds the copy. Its binds are static, as in (A). And a folder
  would then hold two verifiers, the one its starter bundle runs and the
  one `fast` runs, and an edit to its own would not reach `fast`. Run in
  the scratch folders, it would run the same commands as the folder's
  verifier, because it copies the same table.
- **(C) A verify command in the folder's data, run by a pinned generic
  script.** The command would not be bundle bytes. `realms.json` is a
  top-level name the walk skips (`UNPINNED_TOP_LEVEL`,
  `crates/brokkr-runtime/src/places.rs:135-139`). Even pinned as a recipe
  input, the command is text that a generic script runs, which is the
  `bash -c` route `bundle.rs:3148` refuses. This weakens a refusal, so it
  is not taken. `init`'s own verifier is different: it runs
  `bash -lc "$command"` (`init.rs:1369`), but the command is a literal in
  the pinned script itself (`init.rs:1376-1377`).
- **(D) The folder's whole seat (recommended).** The recipe says that its
  verify and ship seats are the folder's. The compile takes the folder
  bundle's own declaration of each: the dispatch, the script and the box.
  It pins the folder's layer exactly as it pins an ancestor's, so the
  spawn walk works unchanged. One verifier per folder serves both the
  starter bundle and every portable shelf recipe. The script and the box
  come from the stack table, which stays in one place, and the folder
  widens its verifier by editing its own file.

## Rulings

1. **A recipe's exec seat may be the folder's own.** A seat may declare
   `"from": "folder"`, a closed vocabulary of one word (decision 0071
   ruling 2). The rules:

   - **What the folder supplies.** A recipe compiled where a folder
     stands takes that seat's `driver` and `hands` from the folder
     bundle's own declaration of the same seat name in
     `<root>/.brokkr/bundle.json`. That declaration is read as written,
     never composed. The `class`, `results` and `limits` stay the
     recipe's, so the recipe's table rules the seat as before. This
     covers a shelf recipe and any recipe composed from one.
   - **What is pinned.** The folder's script is admitted by 0048's
     grammar against the folder layer, `<root>/.brokkr/`. That layer's
     file map is the folder bundle's own walk, which already skips
     `state/` and `recipes/` (`places.rs:156-159`). It is kept in the
     compiled bundle as an ancestor's is (`compose.rs:56-66`), with its
     digest in the manifest's `files` under a `@folder/<name>` key. So the
     folder's root joins the roots `script_directory` reads, and
     `layer_drift` walks `.brokkr/scripts/` against that map at every
     spawn. Run-manifest v13's `files` admits any key with a hex digest,
     so the prediction is that no manifest field or contract version is
     added. If the build finds otherwise, it stops and asks
     (decision 0082).
   - **What a shelf recipe carries.** Its own complete declaration of the
     seat, as today. That declaration is what stands where no folder does:
     in a recipe library, a bare repository and the witness table. In a
     folder, its script is pinned with its layer and never spawned for
     that seat.
   - **What refuses at compile.** Each refusal reads as below
     (predicted wording) and writes nothing:

     ```text
     bundle: seat '<seat>' is the folder's own (decision 0091), and <root>/.brokkr/bundle.json declares no seat '<seat>'
     bundle: seat '<seat>' is the folder's own (decision 0091), and the folder's is <class> with results <results> where the recipe's is <class> with results <results>
     bundle: seat '<seat>' is the folder's own (decision 0091), and the folder's is not a `{brokkr} driver exec` dispatch of its own pinned script
     bundle: seat '<seat>' says "from": "folder" and declares no driver of its own, which a compile with no folder runs (decision 0091)
     bundle: <root>/.brokkr/bundle.json: seat '<seat>' is the folder's own by its base '<base>' (decision 0091), and a folder declares its own: declare seat '<seat>' in <root>/.brokkr/bundle.json
     ```

     The last refusal covers the folder bundle that extends a shelf
     recipe. It never takes a seat from itself through its base.
   - **What refuses mid-run.** A script in `.brokkr/scripts/` that is
     edited, removed or added after the compile is refused at spawn, by
     0048 ruling 2's existing refusal, naming the folder layer and the
     file key. An edit to the folder's `bundle.json` or `scripts/` moves
     the recipe's digest, so resume refuses it as drift (decision 0017).
     This is the protection the starter bundle's own verify has today,
     and it is no weaker.

   This amends 0048 ruling 2: a folder layer's map is kept beside the
   ancestors' maps and walked at spawn on the same terms.

   **Enforcement binding:** the seat's parse in
   `crates/brokkr-runtime/src/bundle.rs`, where `from` is read once into
   a closed enum and any other word is refused. The folder layer is resolved and kept beside the
   ancestors in `bundle/compose.rs`, and `layer_drift` reads it.
   Runtime tests name each refusal by its variant and pin its text once.
   A spawn test edits `.brokkr/scripts/verify-seat.sh` after the compile
   and is refused with the folder layer named. A CLI test in
   `crates/brokkr-cli/tests/` runs `init .`, then `recipes add` of a
   fixture recipe, then a run whose verify is the folder's.

2. **The folder's verifier is what `init` writes, and the folder widens
   it.** By default per stack, `fast`'s verify in a folder runs the
   commands of the table above, in the box the folder declares. In a
   `Cargo.toml` folder it runs `cargo test --workspace` then
   `cargo clippy --workspace --all-targets -- -D warnings` under
   `~/.cargo` and `~/.rustup`. In a `package.json` folder it runs
   `npm test` then `npm run lint` with no binds. In a folder with no
   recognised stack it fails closed with `init`'s fill-in text, and it
   never passes having run no check of the project. A folder widens its
   verifier by editing its own `.brokkr/scripts/verify-seat.sh` and its
   verify seat's `hands` in `.brokkr/bundle.json`. Those are the
   folder's tracked files. Its starter bundle runs the same seat, so one
   edit reaches every recipe that takes the seat. `node` keeps its own
   verify seat (`npm ci --offline`, `npx tsc --noEmit`, `npm test`, and
   `npm run lint` where `package.json` declares it, in
   `recipes/node/roles/verify-seat.sh:28-36`, under an `~/.npm` overlay,
   `recipes/node/bundle.json:49`). It is already portable,
   and it takes only its ship seat from the folder. `init`'s stack rows
   do not change.

   **Enforcement binding:** judgment-guidance for the defaults, which are
   `init`'s table and its existing tests (`crates/brokkr-cli/tests/init_stacks.rs`).
   The no-stack refusal is held by a CLI test that runs `fast`'s verify
   in a folder with no marker and asserts the result `fail` and the
   fill-in text.

3. **Brokkr's verifier moves to `brokkr-fast`.** These changes, and
   nothing else:

   - `recipes/fast/scripts/verify-seat.sh` moves byte for byte to
     `recipes/brokkr-fast/scripts/verify-seat.sh`.
   - `brokkr-fast` overrides `verify` with fast's declaration of today
     (`recipes/fast/bundle.json:22-51`, the Rust binds included) and no
     `from`.
   - Fast's verify and ship seats gain `"from": "folder"`.
   - Fast's own verify script becomes a short pinned script. It writes
     `fail` with notes saying that fast's verify is the folder's own
     (decision 0091), that no folder stands, and that the remedy is
     `brokkr init` or a recipe that declares its own verify seat.
   - `panel-review`, `triage`, `release`, `review-first` and `standby`
     extend `brokkr-fast` in place of `fast`. Each inherits fast's verify
     today, and each overrides `implement` and `review` already, so only
     its verify's source changes. This amends 0087 ruling 4's re-point
     list. `self`, `gpt-flash` and `night-shift` follow through their
     bases.
   - The overlays of decision 0087 ruling 4 do not change, and no seat
     changes its agent.

   Inside Brokkr's repository, which has no folder, every Brokkr-only
   recipe compiles to the verify and ship seats it compiles to today:
   the same argv, script bytes, hands and limits, now declared by
   `brokkr-fast` and `fast`. What moves is the layer the verify script
   is pinned under, and the bundle files edited.

   **Digest forecast, predicted from the mechanics above and not
   measured:**

   - **Bundle digests that move.** `fast` (its `bundle.json`, its verify
     script and `README.md`). `brokkr-fast` (a new seat, a new
     `scripts/`, and fast's map). `node` (fast's map). `landing`,
     `wager-harness`, `wager-harness-dsh` and `wager-harness-muse`
     (brokkr-fast's map). `panel-review`, `triage`, `release`,
     `review-first` and `standby` (their `extends` and their bases).
     `self`, `gpt-flash` and `night-shift` (their bases' maps). That is
     fifteen of the nineteen bundle rows of
     `crates/brokkr-runtime/tests/witnesses.json`.
   - **Bundle digests that stay.** `preflight`, `research`,
     `research-dsh` and `verify`. They compose no `fast`.
   - **Tables and charters.** No `policy.json` and no charter changes, so
     the policy-table rows of `witnesses.json` and the charter witnesses
     stay. No `init` output changes, so a scaffold's digest stays. The
     five measured above are pinned by no test.

   The build re-pins from `BROKKR_BLESS=1 cargo test -p brokkr-runtime
   --test it witness_digests::`, never by hand, and names each moved value in its
   pull request. Before and after the change, it compares the compiled
   verify and ship seats of every Brokkr-only recipe (argv, hands,
   limits, script bytes) and records the comparison there.

   **Enforcement binding:** the witness table, and the CI compile of
   `recipes/self` and `recipes/verify` (decision 0078). One runtime test
   compiles `fast` with no folder and asserts that its verify is the
   fail-closed script.

4. **The shipper.** In a folder, `fast` runs the folder's own shipper,
   which names `.brokkr/state/ledger/<run>.md` (`init.rs:275`), the path
   the `ledger` verb writes there. With no folder, fast's own shipper
   runs. Its `.forge/ledger/` (`ship-seat.sh:61`) is that layout's
   run-local place (`folder.rs:513-528`), so its bytes do not change. A
   folder's run cannot reach fast's own shipper: ruling 1 takes the
   folder's seat or refuses.

   **Enforcement binding:** ruling 1's compile refusals. Ruling 5's
   acceptance runs each read the ledger path from the ship result's
   notes.

5. **The build, in slices, against #717's acceptance.** Slice 3a (the
   overlay charters' run-local wording, queued apart) is independent of
   these.

   - **Slice 3b: ruling 1 alone.** The `from` key, the folder layer's
     pin, the five refusals and the spawn refusal, each tested, with
     ruling 1's CLI test on a fixture recipe. No recipe changes, so
     the forecast is that no witness moves.
   - **Slice 3c: rulings 3 and 4 as data.** Fast's two seats, its
     fail-closed script, `brokkr-fast`'s verify and the moved verifier,
     and the five re-points. The READMEs change too: fast's
     `recipes/fast/README.md:28-35` paragraph says that its verify and
     ship are the folder's, and `brokkr-fast`'s README says that it
     carries Brokkr's verifier. `README.md:66` ("From a clone of this
     repository skip `init`: `brokkr run --recipe fast --repo .` …") and
     `docs/guides/how-brokkr-works.md:9` re-point to `brokkr-fast`,
     because `fast` with no folder fails verify closed once this slice
     lands. The slice also re-pins the witnesses and
     records the comparison of ruling 3.
   - **Slice 3d: the acceptance.** It runs in scratch folders under
     claude, codex and dsh. Each folder is made with `git init`, then a
     stack marker (a `Cargo.toml`, and a `package.json` declaring `test`
     and `lint`), then `brokkr init .` and `brokkr recipes add
     <checkout>/recipes/fast --name fast`. Each folder then runs `brokkr
     run --recipe fast`, which reaches verify and passes it: six runs,
     with each journal's verify row recorded in the pull request. One
     more folder with no marker shows verify reached and failing closed
     with the fill-in text. A guide that then claims a run shows it in a
     block sampled by #365's census
     (`crates/brokkr-cli/tests/doc_samples.rs`).

## Asked of the operator

Each choice is listed with its recommendation first.

- **The form.** (D), the folder's whole seat, as ruling 1 lays out.
  Recommended because it keeps one verifier per folder, takes the box
  with the script, and reuses the ancestor pin that 0048's walk already
  reads. The alternatives are (B), with no engine change and a held
  copy of `init`'s stack table and static binds, and (A), which takes
  the script without its box. (C) is not offered, because it would
  weaken 0048's refusal of text.
- **The key.** `"from": "folder"` on the seat. The alternative is a seat
  that names no driver at all and is only the folder's. That loses the
  declaration a compile with no folder runs, so `fast` would refuse in
  the witness table.
- **What the folder supplies.** `driver` and `hands`, with `class`,
  `results` and `limits` staying the recipe's. The alternative takes the
  limits too. Then `node`'s override of ship's limits would stop
  mattering in a folder.
- **A folder with no recognised stack.** Verify fails closed with
  `init`'s fill-in text, and the acceptance's fresh folders carry a stack
  marker. The alternative passes on a floor of checks every repository
  can run (`git diff --check` of the run's commits). That makes
  `git init` then `brokkr init .` pass verify with no project check run,
  a gate that fails open.
- **Fast with no folder.** A fail-closed script, so Brokkr's verifier
  lives only in `brokkr-fast`, as 0087's accepted choice says. The
  alternative keeps Brokkr's verifier as fast's own declaration. That
  saves ruling 3's five re-points and every move but `fast`'s own, but
  ships Brokkr's checks in every shelf copy of `fast` and leaves the
  verifier where 0087 ruled it would not stay. The recommendation's
  consequence: in Brokkr's own clone, `brokkr run --recipe fast --repo .`
  stops passing verify, so slice 3c re-points the two lines that say to
  run it (`README.md:66` and `docs/guides/how-brokkr-works.md:9`) to
  `brokkr-fast`.
- **The re-points.** The five recipes of ruling 3 extend `brokkr-fast`.
  The alternative has each declare Brokkr's verify seat itself, five
  copies of one seat, which decision 0071 ruling 5 refuses.
- **`node`'s verify.** Its own, unchanged. The alternative takes the
  folder's, which would drop its `tsc` check and its `~/.npm` bind.
- **The shipper's bytes.** Unchanged, as ruling 4 says. The alternative
  has fast's shipper read the path the `ledger` verb prints at its first
  entry. Its confirming entry would still need the path again.

## Consequences

- Once slices 3b to 3d land, `git init`, a stack marker, `brokkr init .`,
  `brokkr recipes add <checkout>/recipes/fast --name fast` and `brokkr
  run --recipe fast` pass verify under claude, codex and dsh. The same
  holds for `node`, whose verify is its own.
- A folder's verify is the same seat whichever recipe runs it. An edit to
  `.brokkr/scripts/verify-seat.sh` or to the folder's verify seat moves
  the digest of the starter bundle and of every shelf recipe that takes
  the seat.
- A shelf copy of `fast` still carries its own verify and ship scripts.
  In a folder they are pinned and never spawned for those seats.
- In Brokkr's own clone, which has no folder, `brokkr run --recipe fast
  --repo .` fails verify closed once slice 3c lands. Brokkr's own runs
  name `brokkr-fast`, and slice 3c re-points the README and the tour that
  say to run `fast` there.
- A run that started before slice 3c lands, on a recipe whose digest
  moves, is refused on resume as drift (decision 0017). The slice lands
  when no such run is parked, or the run resumes from its pinned bytes.
- `init`'s verifier runs each command through `bash -lc`
  (`init.rs:1369`), and a login shell reads the profile files of
  whatever home the seat has. Under `harness` and `open` that is the
  host's. This is decision 0049 ruling 3's unpinned interpreter, which
  existed before this decision. Whether the verifier drops `-l` is a
  follow-up, not this decision's.
- Decision 0083 stands. The shelf, its composition and `recipes add`'s
  copies do not change. A recipe now reads one declared seat of the folder
  bundle at compile, and nothing else of it.
- Only the operator accepts this proposal. Filing it builds nothing,
  moves no digest, and certifies no host run, remote CI or publication.
