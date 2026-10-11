# 0083 — Everything Brokkr writes into an adopter's repository lives in `.brokkr/`

Status: accepted (operator ruled in chat, 2026-10-11, on #634)
Date: 2026-10-11

Built: unbuilt (#634) — the build changes CLI defaults in `cli_args.rs`, so it follows U6c8 (#625)

## Context

`brokkr init .` scatters its scaffold across the root of the adopter's
repository. `docs/guides/quickstart.md`'s Step 2 lists it: `bundle.json`,
`policy.json`, `realms.json`, `adapters/`, `agents/`, `scripts/`, `dialects/`
in a spec-driven repository, and `.forge/.gitignore`. Three of those names,
`scripts/`, `agents/` and `policy.json`, are names a project commonly owns,
and `init` writes `policy.json` without first checking for one
(`crates/brokkr-cli/src/init.rs`, `init`). The runtime state lives in
`.forge/`, a name older than the Brokkr name of decision 0019, so a new user
meets two names for one tool.

The defaults that point at the root today:

- the journal, `DEFAULT_DB` = `.forge/forge.db` (`crates/brokkr-cli/src/lib.rs`),
  and the journal `init` writes into its map, `.forge/forge.db`;
- the secrets store, `DEFAULT_SECRETS` and the `--secrets-file` default
  `.forge/secrets.env` (`lib.rs`, `cli_args.rs`), and the engine's own
  `<workdir>/.forge/secrets.env` (`crates/brokkr-runtime/src/engine.rs`);
- the agent library and the adapters, the `--agents-dir` and
  `--adapters-dir` defaults
  `brokkr_runtime::bundle::DEFAULT_AGENTS_DIR` = `agents` and
  `DEFAULT_ADAPTERS_DIR` = `adapters`, relative to the working directory;
- the realms map, `./realms.json` when one is there (`World::discover`,
  decision 0023 ruling 3), and the bundle, `--bundle .`;
- the run-local paths written into the operated working tree: the engine's
  `.forge/results/` and `.forge/scratch/` (`SEAT_SCRATCH`), the `brokkr-cli`
  verbs' `.forge/ledger/` (`ledger.rs`) and `.forge/muninn.ndjson`
  (`muninn.rs`), and the `.forge/tasks/` that `init`'s intake charter names.
  Three of these are also written as text into the scaffold `init` makes:
  `scripts/ship-seat.sh` sets `ledger=".forge/ledger/$run_id.md"` and reads
  that file back for its close-out, and the implementer's and the intake
  agent's charters name `.forge/tasks/` (`init.rs`).

A map's paths resolve against two bases. The map's `journal`, and each
realm's `path` and `journal`, resolve against the map file's own directory
(`World::resolve`). A realm's `house`, a `dialect` given as a path, and a
published crossing's `path` resolve against the realm's root, its `path`
already resolved (`realm_root` in `crates/brokkr-runtime/src/realms.rs`), and
a `dialect` given as a name resolves to the map directory's
`dialects/<name>.json` (`library_path`). So a map that moves into a folder
restates its map-relative paths and keeps its realm-relative ones. A bundle's
manifest walk covers the bundle
directory and skips three top-level names, `realms.json`, `dialects` and the
capability definitions (`unpinned_top_level`), because a scaffold may also be
the workspace. Every other regular file under the bundle is pinned in the
manifest's `files` map (`walk_files`), whether the compile read it or not, so
in a root scaffold a project's own `scripts/build.sh` is pinned beside the
seat scripts. What the compile read by reference is recorded apart: the agents
its seats hire, with their charters (the manifest's `agents`), the adapter
declarations that answered (`drivers`), and the `./` scripts a stage runs
(`pinned_script`).

The operator ruled option A on 2026-10-11: everything Brokkr writes into an
adopter's repository lives in one hidden folder, `.brokkr/`, as a project's
CI lives in `.github/`. The legacy `.forge/` name is retired for adopters.
Brokkr's own repository is a recipe library (decision 0078), not an adopter
scaffold, and its layout stays.

## Rulings

1. **The layout.** An adopter's repository holds exactly this, and `init`
   writes nothing else:

   ```text
   .brokkr/
     .gitignore         state/
     bundle.json        the seats
     policy.json        forge.phase-machine/v1
     realms.json        the realm map; its realm path is "..", its journal "state/forge.db"
     adapters/          the trust tiers and tool maps
     agents/            the agent definitions, their charters/, and README.md
     scripts/           verify-seat.sh, ship-seat.sh
     dialects/          only in a spec-kit or OpenSpec repository
     state/             ignored: the journal and everything a run writes
       forge.db         with its -wal and -shm while open
       secrets.env
       results/  scratch/  ledger/  tasks/
       muninn.ndjson
   ```

   The tracked scaffold is everything but `state/`, and `.brokkr/.gitignore`,
   whose one line is `state/`, keeps the runtime state out of git. The ship
   gate's clean-tree check therefore holds with no edit to the operator's own
   `.gitignore`. The bundle's manifest walk skips `state` as a fourth
   unpinned top-level name, so a run writing its journal never moves its
   bundle's identity. A bundle's script paths stay relative to the bundle,
   which is `.brokkr/` itself, so `scripts/verify-seat.sh` keeps its spelling.
   Capability definitions an adopter adds stand in the bundle directory, at
   `.brokkr/capabilities/`, by the same walk. The secrets store at
   `.brokkr/state/secrets.env` stands under the bundle directory but under
   the unpinned `state/`, so the walk never digests it; the compile's
   refusal of a `secrets.env` anywhere else under the bundle stands, and its
   example path becomes `.brokkr/state/secrets.env`. The scripts and
   charters `init` writes name the state by its new place:
   `ledger=".brokkr/state/ledger/$run_id.md"` in `scripts/ship-seat.sh`, and
   `.brokkr/state/tasks/` in the implementer's and intake charters.

   **Enforcement binding:** the build adds
   `init_writes_only_under_the_folder` to a new
   `crates/brokkr-cli/tests/brokkr_folder.rs`: `init` in a fresh git
   repository, then every path the tree gained is listed, and the list must
   equal the tracked scaffold above exactly, for a plain repository and for
   an OpenSpec one (which adds `dialects/`); nothing stands at the root, and
   no file `init` wrote contains the text `.forge/`. It
   adds `state_never_moves_the_bundle` there too: a compile's digest before
   and after a file is written under `.brokkr/state/` is equal.

2. **Discovery, and its precedence.** A verb walks up from the working
   directory to the repository root: the first directory that holds a `.git`
   entry, a directory or, for a linked worktree, a file. The walk never
   passes that root, and outside any repository the working directory is
   the root. The folder is the root's own `.brokkr`, and only that one: a
   repository has one folder, at its root, as Asked item 1 has it. A
   `.brokkr` entry the walk meets below the root refuses, naming it and the
   root, before anything is opened:

   ```text
   <path>/.brokkr is not at this repository's root <root> (decision 0083); Brokkr reads only <root>/.brokkr: move or remove it
   ```

   A root `.brokkr` that is not a directory refuses too, before anything is
   opened, with its own text:

   ```text
   <root>/.brokkr is not a directory (decision 0083); Brokkr's folder must be one: move or remove it
   ```

   The root is the workspace: the repository the run operates, the realm it
   is matched against and the base of every default in ruling 3. So `brokkr
   run --feature "…"` from `src/deep/` operates the same repository, journal
   and realm as it does from the root, with no `--bundle`, `--realms` or
   `--db`.

   Each fact resolves on its own, in this order:

   1. its explicit flag, resolved against the working directory as typed;
   2. for the journal only, the `journal` of the map in force, as decision
      0023 ruling 3 has it, with `--db` still first;
   3. its place in the discovered folder (ruling 3);
   4. with no folder discovered, in a recipe library (ruling 4 says how one
      is told), the library layout of ruling 3's last column. In any other
      repository with no folder, the read-only facts (the realms map, the
      agents and the adapters) take the library column, and the state has
      no place: neither the journal, nor the secrets store, nor the
      run-local directory has a default. A verb that would fall to one of
      them refuses before opening or creating anything, with this text:

      ```text
      no .brokkr/ folder from <cwd> up to <root>, and <root> is not a recipe library (decision 0083); nothing was created: run `brokkr init .` from <root>
      ```

      So `brokkr secrets set NAME` in a fresh repository refuses, as a run
      does, and outside a library nothing is written under `.forge/` but
      by the escape below, into an old layout's `.forge/` that already
      stands.
      A flag supplies only its own fact: `--secrets-file` lets the secrets
      verbs work on the file it names, and `--db` the journal, but the
      run-local directory has no flag, so a verb that runs a seat still
      refuses. The one exception is ruling 4's escape: where the root holds
      an old layout and `--db` names its journal, the state facts take the
      library column, which there is that layout's own `.forge/`.

   Ruling 4's old-layout check runs first, before any map or store is
   opened, unless `--db` is given: a journal named on the command line is
   read as named, and ruling 4 says why that is the escape and not a
   half-read.

   A map found in the folder is an ambient map in decision 0023 ruling 3's
   sense: it is adopted, and the notice every untyped map earns is printed,
   naming `.brokkr/realms.json`.

   **Enforcement binding:** the build adds to `brokkr_folder.rs`:
   `discovery_finds_the_folder_from_a_subdirectory` (a run and `brokkr
   doctor` from `src/deep/` name `<root>/.brokkr` and open
   `<root>/.brokkr/state/forge.db`); `discovery_stops_at_the_git_root` (a
   `.brokkr/` above the repository root is not found);
   `a_nested_folder_refuses` (with `<root>/.brokkr/` and `<root>/src/.brokkr/`
   both planted, a run from `src/deep/` prints the nested refusal above
   exactly, naming `<root>/src/.brokkr`, and opens no journal);
   `a_folder_that_is_not_a_directory_refuses` (a regular file at
   `<root>/.brokkr` gives its own text above exactly);
   `no_folder_outside_a_library_names_init` (a repository with neither a
   folder nor a library mark, run with `--bundle` alone, refuses with the
   no-place text exactly and creates neither `.forge/` nor `.brokkr/`);
   `secrets_set_without_a_folder_refuses` (in the same repository, `brokkr
   secrets set NAME` with a value on stdin refuses with the same text and
   creates neither directory, and with `--secrets-file` it writes the named
   file alone); and `an_explicit_flag_outranks_discovery`, one case per
   flag (`--bundle`, `--realms`, `--db`, `--agents-dir`, `--adapters-dir`,
   `--secrets-file`), each proving the flagged path is the one opened while
   a folder stands discovered.

3. **The defaults read the discovered folder.**

   | Fact | Flag | With `.brokkr/` discovered | With none (the library layout) |
   |---|---|---|---|
   | bundle | `--bundle` | `.brokkr/` | none: a run verb refuses, naming where it looked and `brokkr init .` |
   | realms map | `--realms` | `.brokkr/realms.json` when present | `./realms.json` when present |
   | journal | `--db` | the map's `journal`, else `.brokkr/state/forge.db` | the map's `journal`, else `.forge/forge.db` in a library and none elsewhere |
   | agent library | `--agents-dir` | `.brokkr/agents/` | `agents/` |
   | adapters | `--adapters-dir` | `.brokkr/adapters/` | `adapters/` |
   | secrets | `--secrets-file` | `.brokkr/state/secrets.env` | `.forge/secrets.env` in a library and none elsewhere |
   | run-local paths | none | `.brokkr/state/` `results/`, `scratch/`, `ledger/`, `tasks/`, `muninn.ndjson` | `.forge/` as today in a library and none elsewhere |

   The folder's paths are resolved once, by one function in `brokkr-cli`, and
   the engine is handed the state directory as an argument rather than
   joining `.forge/` itself, so the seat's result path, the scratch place and
   the secrets store cannot disagree. The library column is today's
   behaviour, kept for Brokkr's own repository and any recipe library, whose
   runs name their bundle with `--bundle`. The one change in that column is
   the bundle: with no folder there is no default bundle, so a new user who
   has not run `init` is told so instead of being handed a `.forge/` journal.
   In a library, a run verb with no `--bundle` refuses with:

   ```text
   no .brokkr/ folder from <cwd> up to <root> (decision 0083), so there is no default bundle: name one with --bundle, or run `brokkr init .` from <root>
   ```

   Outside a library the column holds no state default at all (ruling 2's
   step 4), and the no-place text answers first.

   **Enforcement binding:** the flag cases of ruling 2, and each discovered
   default is asserted by `discovery_finds_the_folder_from_a_subdirectory`
   (the journal, the agents, the adapters and the secrets path `doctor`
   prints). `run_without_a_folder_names_init` asserts the library column's
   bundle refusal text above exactly, in a library with no folder.

4. **Old layouts are refused, and moved whole.** A recipe library is told
   apart by one mark: its root holds a `recipes/<name>/bundle.json` for some
   name, because a library's bundles live under `recipes/` (decision 0078)
   and `init` never wrote a `recipes/`. Three shapes are detected before any
   map or store is opened, by every verb that discovery serves:

   - **The root layout.** No `.brokkr/` is discovered, and the repository
     root holds a `bundle.json`: `init`'s old signature. No library root
     holds one.
   - **Journal only.** No `.brokkr/` is discovered, the root holds a
     `.forge/forge.db` and no `bundle.json`, and the repository bears no
     library mark: an adopter who named its bundle with `--bundle` every
     time. In a library, `.forge/forge.db` is the library's own journal
     (ruling 3's last column) and is no old layout.
   - **Half moved.** A `.brokkr/` is discovered, and its parent still holds a
     `bundle.json` or a `.forge/forge.db`. A repository is read from one
     layout, never two.

   Each refuses, exits as a refusal and opens nothing. The root layout's
   refusal is exactly this, with one line per move the repository needs and
   no line for a file that is not there:

   ```text
   this repository has Brokkr's old root layout and no .brokkr/ folder (decision 0083); nothing was opened
   run `brokkr init --migrate` from <root>; it makes exactly these moves:
     git mv bundle.json .brokkr/bundle.json
     git mv policy.json .brokkr/policy.json
     git mv realms.json .brokkr/realms.json
     git mv adapters/claude.json .brokkr/adapters/claude.json
     git mv agents/implementer.json .brokkr/agents/implementer.json
     git mv scripts/verify-seat.sh .brokkr/scripts/verify-seat.sh
     git rm .forge/.gitignore
     mv .forge/forge.db .brokkr/state/forge.db
     mv .forge/forge.db-wal .brokkr/state/forge.db-wal
   writes .brokkr/agents/README.md afresh; agents/README.md stays where it is
   and rewrites .brokkr/realms.json's map-relative paths to name the same places from .brokkr/
   and rewrites the state paths in .brokkr/scripts/ship-seat.sh:  .forge/ledger/ -> .brokkr/state/ledger/
   and rewrites the state paths in .brokkr/agents/charters/implementer.md:  .forge/tasks/ -> .brokkr/state/tasks/
   and rewrites the state paths in .brokkr/agents/charters/intake.md:  .forge/tasks/ -> .brokkr/state/tasks/
   ```

   (The listing is abridged here; a refusal lists every move.) The
   journal-only refusal opens `this repository has a Brokkr journal at
   .forge/forge.db and no .brokkr/ folder (decision 0083); nothing was
   opened`, lists the state moves alone in the same form, and closes `then
   brokkr init . writes the scaffold beside the moved state`. The half-moved
   refusal names each file still at the root and says which layout is in
   force: `<root>/.brokkr/ is this repository's folder, but <path> still
   stands at the root (decision 0083); move or remove it`.

   **A named journal is the escape.** The check is skipped when `--db` is
   given (ruling 2). That is not a half-read: the journal read is the file
   the operator named, and with no folder the other facts take the library
   column, which in an old layout are that layout's own places (`agents/`,
   `adapters/`, `./realms.json`, `.forge/secrets.env`), so the verb reads one
   layout whole, the old one. It is how a run that has not ended is ended
   before the move, by `brokkr conclude --run <id> --reason "…" --db
   .forge/forge.db`, or driven to its end by `brokkr resume` with the same
   `--db`. In the half-moved shape the other facts come from the folder,
   and a resumed run's drift fence still holds every pinned digest.

   `brokkr init --migrate` makes the move the refusal printed, and nothing
   else:

   - **Which files move** is what the old compile read by reference, never
     the manifest's `files` map, which pins every file under a root
     scaffold, a project's own `scripts/build.sh` included (Context): the
     bundle; the policy table it names; each agent a seat hires and the
     charter that agent names (the manifest's `agents`); each adapter
     declaration the compile read for a seat's harness and gates (its
     `drivers`); each script a hired agent's command or a stage runs by a
     `./` path under the bundle; the map; and, for each `dialect` the map
     gives as a name, `dialects/<name>.json` and the texts it reads (`init`
     writes a name). A `house`, a `dialect` given as a path and a published
     crossing stay where they are, as their paths do. The build has the compile return that list as typed
     data; `--migrate` does not re-derive it. A file no reference reached
     stays where it is and is named as left behind, whatever directory it
     stands in. `init`'s `agents/README.md` is one such file: its text
     depends on the stack `init` detected, so it is not told from an
     operator's own by its bytes. It stays, and `--migrate` writes
     `.brokkr/agents/README.md` afresh, as `init` would. An old bundle that
     does not compile refuses the migration with its compile error.
   - **Tracked files move by `git mv`**, so the move is staged and reviewable;
     untracked ones are renamed. `--migrate` commits nothing.
   - **The map is rewritten**, not reinterpreted, field by field after the
     two bases of the Context. Each path is first resolved as the old map
     resolved it, lexically, the way `World::resolve` and `realm_root` do,
     so a relative and an absolute spelling of one place are judged alike.
     A map-relative path, the map's `journal` and each realm's `path` and
     `journal`, that resolves to a place under `<root>/.forge/` is restated
     as the same place under `<root>/.brokkr/state/`, in the form it was
     written: a relative one from `.brokkr/` (so `init`'s `.forge/forge.db`
     becomes `state/forge.db`), an absolute one as the new absolute path (so
     `<root>/.forge/forge.db` becomes `<root>/.brokkr/state/forge.db`). Any
     other map-relative path keeps the place it names: a relative one gains
     a leading `..`, so `init`'s `.` becomes `..`, and an absolute one is
     kept. A realm-relative path, a `house`, a `dialect` given as a path and
     a published crossing's `path`, is kept byte for byte, because the realm
     root it resolves against names the same directory after the move; one
     that resolves to a place the migration moves refuses the migration
     before any move, with `realm <name>'s <field> <path> names a place
     --migrate moves (decision 0083); nothing was moved: point it outside
     .forge/ first`. A `dialect`
     given as a name is kept, because the dialect library moves with the map
     into `.brokkr/dialects/`. The map's digest moves, and the output says
     so.
   - **The state paths written into moved files are rewritten**, from one
     closed list: `.forge/` followed by `forge.db`, `results/`, `scratch/`,
     `ledger/`, `tasks/`, `secrets.env` or `muninn.ndjson` becomes the same
     text under `.brokkr/state/`, in every script and charter the migration
     moves. That covers what `init` wrote (the ship script's ledger line,
     which its close-out reads back, and the two charters' `.forge/tasks/`),
     and an operator's own text that names the same places. A moved file
     whose text holds any other `.forge/` refuses the migration before any
     move, with `<file>:<line> names <text>, a .forge/ path --migrate does
     not know (decision 0083); nothing was moved: remove it or point it
     outside .forge/ first`, because a path the migration does not
     recognise is never guessed at. Each rewrite is a line
     of the refusal's listing and of the output.
   - **The journal moves; it is never read in place.** `forge.db` moves with
     its `-wal` and `-shm` files, together, because a write-ahead log can hold
     committed history, and so does every journal the map names that
     resolves under `<root>/.forge/`, spelled relative or absolute, so the
     map's rewritten path and the moved file always agree. The migration
     refuses while another process holds a journal,
     and while a journal holds a run that has not ended, naming each run and
     the exact `brokkr conclude --run <id> --reason "…" --db <journal>` line
     that ends it: such a run pinned the old paths, and its resume would
     meet the drift fence. A journal already at `.brokkr/state/forge.db`
     refuses the migration: journals never merge (decision 0026 ruling 5).
   - **The rest of the state moves** (`results/`, `scratch/`, `ledger/`,
     `tasks/`, `secrets.env`, `muninn.ndjson`), `.forge/.gitignore` is removed
     and `.brokkr/.gitignore` written. `.forge/` is removed once empty; anything
     left in it is named, never deleted.
   - It then compiles the moved bundle and prints the old and the new digest.

   There is no one-release read-in-place of `.forge/`: the refusal fires
   before a store opens, so a fresh empty journal can never appear at the new
   path while the history sits at the old one, and the move is a rename of
   every file the journal is made of. The old journal is read where it
   stands only when `--db` names it, which is said on the command line and
   never a fallback.

   **Enforcement binding:** the build adds to `brokkr_folder.rs`:
   `old_root_layout_refuses_with_the_migration_text` (it plants `init`'s old
   output with a journal and its `-wal`, runs `brokkr run --feature x`, and
   asserts the exact refusal, that no `.brokkr/` was created and that the
   journal's bytes did not move); `journal_only_layout_refuses` (it plants a
   `.forge/forge.db` alone, with no root `bundle.json` and no library mark,
   runs `brokkr run --bundle elsewhere --feature x`, and asserts the exact
   journal-only refusal and unmoved journal bytes; then `brokkr init .`
   there refuses, pointing at `--migrate`); `a_library_keeps_its_journal`
   (a `recipes/x/bundle.json` beside `.forge/forge.db`, run with `--bundle
   recipes/x`, opens `.forge/forge.db` with no refusal);
   `half_moved_layout_refuses`; `an_explicit_db_reaches_the_old_journal` (in
   the planted root layout, `brokkr conclude --run <id> --reason x --db
   .forge/forge.db` ends the unfinished run with no refusal, after which
   `--migrate` succeeds); `migrate_refuses_an_unfinished_run` (its exact text,
   with the conclude line); `migrate_moves_the_journal_whole` (the run
   journaled before the migration is listed by `brokkr status` after it,
   from `.brokkr/state/forge.db`); `migrate_keeps_realm_relative_paths` (a
   map whose realm names a `house` of `docs/HOUSE.md`, a `dialect` given as
   the path `spec/dialect.json` and a published crossing `api/schema.json`
   keeps those three strings byte for byte, and the migrated world loads
   with the same house, dialect and crossing digests it had before);
   `migrate_rewrites_an_absolute_journal` (a map whose `journal` is the
   absolute `<root>/.forge/forge.db` and whose realm's `journal` is the
   absolute `<root>/.forge/app.db`, each with its `-wal`: after `--migrate`
   the map names `<root>/.brokkr/state/forge.db` and
   `<root>/.brokkr/state/app.db`, both journals and their `-wal` files stand
   there, `brokkr status` lists the run journaled before the move, and the
   next run raises no half-moved refusal);
   `migrate_refuses_a_realm_path_into_moved_state` (a `house` of
   `.forge/tasks/house.md`, under a directory the migration moves, refuses with its exact text and nothing moves);
   `migrate_rewrites_embedded_state_paths` (in `init`'s old output the moved
   `scripts/ship-seat.sh` reads `ledger=".brokkr/state/ledger/$run_id.md"`
   and both charters name `.brokkr/state/tasks/`, and a migrated run's ship
   close-out finds the ledger the CLI wrote under `.brokkr/state/ledger/`
   while its intake writes under `.brokkr/state/tasks/` and creates no
   `.forge/`); `migrate_refuses_an_unknown_forge_path` (a hired agent's
   charter naming `.forge/custom/` refuses with its exact text and nothing
   moves); and
   `migrate_moves_only_what_the_compile_read` (a `scripts/build.sh` beside
   the seat scripts and an `agents/notes.json` no seat hires stay at the
   root and are named as left behind, `agents/README.md` stays, and
   `.brokkr/agents/README.md` is written).

5. **`init` writes the folder and says what it wrote; `doctor` reports it.**
   `brokkr init <dir>` writes `<dir>/.brokkr/` and nothing outside it, so the
   second `.gitignore` it writes today, beside the repository it was invoked
   from, is gone. `<dir>` must be a repository's root, because ruling 2
   reads a folder only there: `init` walks up from `<dir>` as discovery
   walks up from a working directory, and when the root it reaches is not
   `<dir>` itself it refuses before writing anything, with

   ```text
   <dir> is inside the repository <root> but is not its root (decision 0083); a folder is read only at the root: run `brokkr init <root>`
   ```

   so `brokkr init .` from `<root>/src/` and `brokkr init src` from `<root>`
   both refuse; it never moves the target up on its own. Outside any
   repository `<dir>` is its own root, as in ruling 2. It also refuses when `<dir>/.brokkr` exists and holds anything
   but the `.gitignore` and `state/` a journal-only migration leaves, naming
   it, in place of today's file-by-file refusals, and when `<dir>` holds the
   root layout or the journal-only one, pointing at `--migrate`. It compiles the folder before it
   prints, as now, and then prints its digest, every path it wrote relative
   to `<dir>`, one per line, and the commit line `git add .brokkr && git commit`.

   `brokkr doctor`'s first line is the folder: `folder: <root>/.brokkr (found
   from <cwd>)`, `folder: none from <cwd> up to <root> — brokkr init . writes
   one`, or ruling 4's refusal as a line, because doctor reports and never
   refuses (decision 0046's Addendum). Its journal, agents, adapters and
   secrets lines read the paths ruling 3 resolved, and a state fact with no
   place (ruling 2's step 4) is reported as `none`.

   **Enforcement binding:** `init_writes_only_under_the_folder` asserts the
   printed list equals the written list; `init_refuses_an_existing_folder`
   (with `init_writes_beside_migrated_state`, its one admitted case),
   `init_from_a_subdirectory_refuses` (both forms above give the exact text,
   and neither `<root>/src/.brokkr` nor `<root>/.brokkr` is written), and
   `doctor_names_the_folder` (each of the three lines, exactly) are added
   to `crates/brokkr-cli/tests/init_doctor.rs`.

6. **The docs and tests follow, in the build's pull request.** The quickstart
   (Step 2's inventory and every `.forge/` it names for an adopter),
   `docs/guides/starters/`, `docs/guides/repository-layout.md` (Brokkr's own
   repository is a library and keeps its layout), the doc-truth paths and
   the init tests (`init_doctor.rs`, `init_stacks.rs` and `init.rs`'s own)
   move with the code. This decision writes no guide.

   **Enforcement binding:** `crates/brokkr-cli/tests/doc_truth.rs` holds the
   living docs to the tree, and the tests of rulings 1 to 5 hold the code.

7. **What of #432 this absorbs.** Issue #432 separates personal from tracked
   configuration. This decision settles where an adopter's tracked
   configuration lives (`.brokkr/`, committed) and where a clone's own state
   lives (`.brokkr/state/`, ignored), and `secrets.env` is that clone's
   state. It does not move the machine's configuration: `forge.host/v1` stays
   at `$XDG_CONFIG_HOME/brokkr/host.json`, else
   `$HOME/.config/brokkr/host.json`, and never in a repository (decision
   0068's addendum of 2026-10-10). A personal overlay of tracked
   configuration, an operator's own choice over a committed recipe, and the
   origin of each composed field stay #432's.

   **Enforcement binding:** judgment guidance; `doctor_names_the_folder`
   proves the one origin this decision adds, which folder was read.

## Asked of the operator

- **Brokkr's own run-local state.** Ruling 3 keeps the library column's
  `.forge/`, so Brokkr's own runs write `.forge/results/` as they do now and
  its tooling is not moved by this build. The engine then knows two names
  for its run-local directory, one per layout. A later slice could move the
  library to `.brokkr/state/` too, with `/.brokkr/` in this repository's
  `.gitignore`, so the engine carries one name. Recommended as that follow-up,
  not in this build.
- **`init <dir>` and the invoking workspace.** Today `init` detects the stack
  of the repository it is invoked from and writes into `<dir>`, which may be
  elsewhere. Ruling 5 keeps that, with the folder at `<dir>/.brokkr/` and the
  map's realm path relative from there; it now requires `<dir>` to be a
  repository's root, so the folder is always one discovery reads. The
  invoking repository then holds no folder, and unless it is a recipe
  library its runs refuse with ruling 2's no-place text rather than write a
  `.forge/`. The simpler rule is one
  path: `<dir>` is the repository, detected and written alike. Recommended;
  the build takes it only on this ruling.
- **`--migrate` rather than bare `git mv` lines.** The issue lists the
  `git mv` commands first. Ruling 4 makes `--migrate` the mechanism and
  prints the moves it makes, because the evidence rules out a list of
  `git mv` lines alone: the map's two paths must be rewritten, the journal
  is untracked, and `scripts/` and `agents/` may hold a project's own files.
- **The `seat-scratch` place.** The frozen `contracts/host.v1.schema.json`
  describes `seat-scratch` as the run's `.forge/scratch`. The word still
  names the run's scratch directory, which for an adopter is now
  `.brokkr/state/scratch/`. The frozen bytes are not edited, and no new host
  version is cut for a description; the living docs name the path.

## Consequences

- A new adopter's repository gains one folder and one name; the operator's
  `README.md`, `.gitignore`, `scripts/`, `agents/` and `policy.json` are never
  touched.
- `--bundle`, `--realms` and `--db` stop being needed inside an adopter's
  repository, from any directory in it.
- An existing adopter is refused once, with the exact moves, and keeps every
  journaled run.
- Brokkr's own repository is unchanged by the build.
