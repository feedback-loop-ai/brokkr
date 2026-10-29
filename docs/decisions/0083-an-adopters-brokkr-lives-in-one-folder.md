# 0083 — Everything Brokkr writes into an adopter's repository lives in `.brokkr/`

Status: accepted (operator ruled in chat, 2026-10-11, on #634)
Date: 2026-10-11

Built: built — slice A: discovery, the defaults that follow from it, and the old-layout refusals; slice B: `init` writes the folder and `doctor` checks it; slice C: the guides of ruling 6, doctor's folder as its bundle, and the Addendum

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
  decision 0023 ruling 3);
- the run-local paths written into the operated working tree: the engine's
  `.forge/results/` and `.forge/scratch/` (`SEAT_SCRATCH`), the `brokkr-cli`
  verbs' `.forge/ledger/` (`ledger.rs`) and `.forge/muninn.ndjson`
  (`muninn.rs`), and the `.forge/tasks/` that `init`'s intake charter names.
  Three of these are also written as text into the scaffold `init` makes:
  `scripts/ship-seat.sh` sets `ledger=".forge/ledger/$run_id.md"` and reads
  that file back for its close-out, and the implementer's and the intake
  agent's charters name `.forge/tasks/` (`init.rs`).

The bundle has no default today. `--bundle` is an `Option` with no default
value (`DeliveryArgs` in `crates/brokkr-cli/src/cli_args.rs`), and `run`,
`resume` and `rerun` each require one of `--bundle` or `--recipe` (`Cmd` in
`lib.rs`), so a run names its bundle every time.

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
seat scripts. What the compile read by reference is recorded apart: the
agents its seats hire with their charters (the manifest's `agents`), each
site's `role` (`parse_role`), the adapter declarations that answered
(`drivers`), the capability definitions and tool dialects it consulted (the
manifest's `capabilities`), and the `./` scripts a stage runs
(`pinned_script`). The definitions and the tool dialects are read from the
operator configuration directory: the map's own directory when a map is in
force (`capability_context` in `crates/brokkr-runtime/src/launch.rs`), else
the agent library's parent (`compile_with_realm` in `bundle.rs`). In a root
scaffold both are the root.

A bundle may also compose a base: its `extends` names a recipe that the
compile reads from the bundle directory's parent, as `<parent>/<name>/`
(`read_layers` in `crates/brokkr-runtime/src/bundle/compose.rs`). For a root
`bundle.json` that parent is the directory above the repository.

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
   bundle's identity. A bundle's script
   paths stay relative to the bundle, which is `.brokkr/` itself, so
   `scripts/verify-seat.sh` keeps its spelling.
   Capability definitions an adopter adds stand in the bundle directory, at
   `.brokkr/capabilities/`, by the same walk, and the tool dialects its
   grants name at `.brokkr/dialects/tools/`, because both are read from the
   operator configuration directory, which is the map's own, `.brokkr/`. A
   bundle in `.brokkr/` composes no base: its `extends` would be read from
   the repository root, `<root>/<name>/`, a directory the project owns. So
   the compile of a bundle whose directory is a `.brokkr` folder, discovered
   or named by `--bundle`, refuses an `extends` before any base is read,
   with
   `<root>/.brokkr/bundle.json extends '<name>' (decision 0083); a bundle in
   .brokkr/ composes no base, because its library would be <root>: remove
   extends`. The secrets store at
   `.brokkr/state/secrets.env` stands under the bundle directory but under
   the unpinned `state/`, so the walk never digests it; the compile's
   refusal of a `secrets.env` anywhere else under the bundle stands, and its
   example path becomes `.brokkr/state/secrets.env`. The scripts and
   charters `init` writes name the state by its new place:
   `ledger=".brokkr/state/ledger/$run_id.md"` in `scripts/ship-seat.sh`, and
   `.brokkr/state/tasks/` in the implementer's and intake charters, and
   the no-stack note `scripts/verify-seat.sh` writes names the file to
   fill in as `.brokkr/scripts/verify-seat.sh`.

   **Enforcement binding:** the build adds
   `init_writes_only_under_the_folder` to a new
   `crates/brokkr-cli/tests/brokkr_folder.rs`: `init` in a fresh git
   repository, then every path the tree gained is listed, and the list must
   equal the tracked scaffold above exactly, for a plain repository and for
   an OpenSpec one (which adds `dialects/`); nothing stands at the root, and
   no file `init` wrote contains the text `.forge/`. It
   adds `state_never_moves_the_bundle` there too: a compile's digest before
   and after a file is written under `.brokkr/state/` is equal; and
   `a_folder_bundle_composes_no_base`: a `.brokkr/bundle.json` given
   `extends: "shared"`, with a recipe planted at `<root>/shared/bundle.json`,
   refuses with the text above exactly, from discovery and with `--bundle
   .brokkr` alike, and reads no base.

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
   flag (`--bundle`, `--recipe`, `--realms`, `--db`, `--agents-dir`, `--adapters-dir`,
   `--secrets-file`), each proving the flagged path is the one opened while
   a folder stands discovered.

3. **The defaults read the discovered folder.**

   | Fact | Flag | With `.brokkr/` discovered | With none (the library layout) |
   |---|---|---|---|
   | bundle | `--bundle`, `--recipe` | `.brokkr/`, a new implicit default | none: a run verb refuses, naming where it looked and `brokkr init .` |
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
   runs name their bundle with `--bundle` or `--recipe`. The bundle is the
   one fact with no default today (Context), and discovery supplies a new,
   implicit one: with a folder discovered, a run verb given neither
   `--bundle` nor `--recipe` delivers under `.brokkr/`, and either flag
   still wins. With no folder there is still no default bundle, and the
   refusal that today's required flag gives becomes one that names `init`,
   so a new user who has not run it is told so. In a library, a run verb
   with neither flag refuses with:

   ```text
   no .brokkr/ folder from <cwd> up to <root> (decision 0083), so there is no default bundle: name one with --bundle or --recipe, or run `brokkr init .` from <root>
   ```

   Outside a library the column holds no state default at all (ruling 2's
   step 4), and the no-place text answers first.

   **Enforcement binding:** the flag cases of ruling 2, and each discovered
   default is asserted by `discovery_finds_the_folder_from_a_subdirectory`
   (the journal, the agents, the adapters and the secrets path `doctor`
   prints). `run_without_a_folder_names_init` asserts the library column's
   bundle refusal text above exactly, in a library with no folder.

4. **Old layouts are refused, and the refusal lists the moves.** A recipe
   library is told apart by one mark: its root holds a
   `recipes/<name>/bundle.json` for some name, because a library's bundles
   live under `recipes/` (decision 0078) and `init` never wrote a
   `recipes/`. Three shapes are detected before any map or store is opened,
   by every verb that discovery serves:

   - **The root layout.** No `.brokkr/` is discovered, and the repository
     root holds a `bundle.json`: `init`'s old signature. No library root
     holds one.
   - **State only.** No `.brokkr/` is discovered, the root holds a `.forge`
     entry, whatever it holds, even nothing, and no `bundle.json`, and the
     repository bears no library mark: an adopter who named its bundle with
     `--bundle` every time, or one whose only Brokkr state is a secret,
     because `brokkr secrets set` creates `.forge/secrets.env` with no
     journal and no scaffold. The predicate is the `.forge` entry itself,
     not any one file in it, so no Brokkr state is left at the retired path
     while the defaults read the folder. In a library, `.forge/` is the
     library's own state (ruling 3's last column) and is no old layout.
   - **Half moved.** A `.brokkr/` is discovered, and its parent still holds a
     `bundle.json` or a `.forge` entry. A repository is read from one
     layout, never two.

   Every entry is read by its own entry, never through a link: `.brokkr`,
   `bundle.json` and `.forge` are each judged by what stands at that name,
   so a symbolic link there is the entry, whatever it names. Each shape
   refuses, exits as a refusal and opens nothing.

   **Brokkr moves nothing.** The refusal lists the moves for the operator to
   make, by hand or with an agent, and says that `brokkr doctor` checks the
   result (ruling 5). Its first line is the shape's exact text; the list
   below it is advice, not a contract. The root layout's refusal, for
   `init`'s old output with a journal, reads:

   ```text
   this repository has Brokkr's old root layout and no .brokkr/ folder (decision 0083); nothing was opened, and Brokkr moves nothing
   make these moves, then run `brokkr doctor` to check the result:
     mkdir -p .brokkr/adapters .brokkr/agents .brokkr/scripts .brokkr/state
     git mv bundle.json .brokkr/bundle.json
     git mv policy.json .brokkr/policy.json
     git mv adapters/claude.json .brokkr/adapters/claude.json
     git mv agents/implementer.json .brokkr/agents/implementer.json
     git mv agents/README.md .brokkr/agents/README.md
     git mv scripts/verify-seat.sh .brokkr/scripts/verify-seat.sh
     git mv realms.json .brokkr/realms.json
     in .brokkr/realms.json, rewrite journal ".forge/forge.db" as "state/forge.db" and realms[0].path "." as ".."
     mv .forge/forge.db .forge/forge.db-wal .brokkr/state/
     git rm .forge/.gitignore && printf 'state/\n' > .brokkr/.gitignore
   Brokkr reads no script or charter: edit any path one names under .forge/, or builds from its own place, to name the same file after the move
   ```

   (Abridged here; the refusal lists every move.) It first creates
   `.brokkr/` and each destination directory, because `git mv` into a
   missing directory fails. An entry git tracks moves by `git mv` and goes
   by `git rm`; an untracked one by `mv` and `rm`. The list holds three
   groups:

   - **The files the compile reads**, each to the same relative place under
     `.brokkr/`: the bundle, the policy table it names, the agents its seats
     hire and their charters, each site's `role`, the adapter declarations
     that answered, the capability definitions and tool dialects it
     consulted, the `./` scripts a stage runs, the map and each dialect the
     map names by name. They are what the old compile read by reference,
     never the manifest's `files` map, which pins every file under a root
     scaffold, a project's own `scripts/build.sh` included (Context). One
     file `init` wrote is read by no compile and joins this group by name:
     its notes, `agents/README.md`, which ruling 1's layout places at
     `.brokkr/agents/README.md`. The
     compile that lists them skips the root's `.forge` as ruling 1's walk
     skips `state`, so a secret at today's default does not stop the
     listing. Where that compile cannot be read, the refusal says so and
     lists what it can: its second line is `its bundle.json does not
     compile, so the files it reads cannot be listed: <compile error>`, and
     the map's and the state's moves follow.
   - **The map's paths to rewrite**, after the two bases of the Context. A
     map-relative path (the map's `journal`, each realm's `path` and
     `journal`) that names a place under `.forge/` names the same place
     under `state/`; any other relative one gains a leading `..`. A
     realm-relative path (a `house`, a `dialect` given as a path, a
     published crossing's `path`) is kept, because the realm root names the
     same directory after the move.
   - **The state**, to `.brokkr/state/`: each journal under `.forge/` with
     its `-wal` and `-shm`, together, because a write-ahead log can hold
     committed history, and every other `.forge/` entry; then
     `.forge/.gitignore` is removed, `.brokkr/.gitignore` is written with
     its one line `state/`, and the emptied `.forge/` is removed, because a
     `.forge` entry beside the folder, even an empty one, is the half-moved
     shape. `git rm` of a tracked `.forge/.gitignore` removes the emptied
     directory itself, so the list adds `rmdir .forge` only where that file
     is untracked and goes by `rm`. A run that has not ended is ended first, by the escape below:
     it pinned the old paths, and its resume would meet the drift fence.

   The state-only refusal opens `this repository has Brokkr state in .forge/
   and no .brokkr/ folder (decision 0083); nothing was opened, and Brokkr
   moves nothing`, lists the state's moves alone, and closes `then brokkr
   init . writes the scaffold beside the moved state`. The half-moved
   refusal names each entry still at the root, a `.forge` entry included:
   `<root>/.brokkr/ is this repository's folder, but <path> still stands at
   the root (decision 0083); move or remove it`. Journals never merge
   (decision 0026 ruling 5), so where a journal already stands at
   `.brokkr/state/forge.db` beside `.forge/forge.db`, the half-moved refusal
   adds `both <root>/.forge/forge.db and <root>/.brokkr/state/forge.db hold
   a journal, and journals never merge (decision 0026 ruling 5): keep one`.

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

   There is no one-release read-in-place of `.forge/`: the refusal fires
   before a store opens, so a fresh empty journal can never appear at the new
   path while the history sits at the old one, and the journal moves as
   every file it is made of. The old journal is read where it stands only
   when `--db` names it, which is said on the command line and never a
   fallback.

   **Enforcement binding:** the build adds to `brokkr_folder.rs`:
   `old_root_layout_refuses_with_the_moves` (it plants `init`'s old output
   with a journal, its `-wal` and a `.forge/secrets.env`, runs `brokkr run
   --feature x`, and asserts the refusal's first line exactly, that its
   list names the journal, its `-wal` and the secrets store, that no
   `.brokkr/` was created and that every planted path and byte is
   unchanged); `old_root_layout_that_does_not_compile_refuses` (the same
   with its `bundle.json` made not to compile: the first two lines
   exactly, with the error the compile verb prints for that bundle, the
   state's moves still listed, and nothing created or moved);
   `state_only_layout_refuses` (a `.forge/forge.db` alone, with no root
   `bundle.json` and no library mark: `brokkr run --bundle elsewhere
   --feature x` and `brokkr init .` each give the state-only first line
   exactly, and the journal's bytes do not move);
   `secrets_only_layout_refuses` (a `.forge/secrets.env` alone, holding one
   name: `brokkr secrets list` gives the state-only first line exactly and
   creates no `.brokkr/`; after the listed moves and `brokkr init .`,
   `brokkr secrets list` lists the name from `.brokkr/state/secrets.env`);
   `a_library_keeps_its_journal` (a `recipes/x/bundle.json` beside
   `.forge/forge.db`, run with `--bundle recipes/x`, opens
   `.forge/forge.db` with no refusal); `half_moved_layout_refuses` (one
   case with a root `bundle.json` beside the folder, one with a
   `.forge/secrets.env` alone beside it, and one with a journal at both
   `.forge/forge.db` and `.brokkr/state/forge.db`, each with its exact
   text); `a_linked_entry_is_the_entry` (a `.forge` that is a symbolic link
   to a directory holding a journal, with no root `bundle.json`, gives the
   state-only text, and a `bundle.json` that is a link gives the
   root-layout text, each opening nothing);
   `an_explicit_db_reaches_the_old_journal` (in the planted root layout,
   `brokkr conclude --run <id> --reason x --db .forge/forge.db` ends the
   unfinished run with no refusal); and `a_hand_moved_layout_runs`
   (`init`'s old output moved by exactly the refusal's listed moves:
   `brokkr doctor`'s folder line is the clean one of ruling 5, and `brokkr
   status` lists the run journaled before the move from
   `.brokkr/state/forge.db`).

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
   but a `.gitignore` and `state/`, which the state-only refusal's moves
   leave, naming it, in place of today's file-by-file refusals, and when
   `<dir>` holds the root layout or the state-only one, with that shape's
   refusal. It compiles the folder before it
   prints, as now, and then prints its digest, every path it wrote relative
   to `<dir>`, one per line, and the commit line `git add .brokkr && git commit`.

   `brokkr doctor`'s first line is the folder: `folder: <root>/.brokkr (found
   from <cwd>)`, `folder: none from <cwd> up to <root> — brokkr init . writes
   one`, or, in the root layout or the state-only shape, the first line of
   ruling 4's refusal, because doctor reports and never refuses (decision
   0046's Addendum). Its journal, agents, adapters and secrets lines read the paths ruling 3 resolved, and a state fact with
   no place (ruling 2's step 4) is reported as `none`.

   Doctor is how a hand move is checked. With a folder discovered, its
   folder line also reports, and each makes the report unhealthy:

   - each Brokkr entry left at the root beside the folder, the half-moved
     shape of ruling 4: `folder: <root>/.brokkr (found from <cwd>), but
     <path> still stands at the root (decision 0083); move or remove it`,
     one such clause per entry;
   - any reference the folder's compile cannot reach: doctor compiles the
     discovered bundle under its map, as a run would, and its refusal names
     the reference, such as a charter, role, policy table or script the move
     left behind: `folder: <root>/.brokkr (found from <cwd>), but its
     compile refuses (decision 0083): <compile error>`.

   **Enforcement binding:** `init_writes_only_under_the_folder` asserts the
   printed list equals the written list; `init_refuses_an_existing_folder`
   (with `init_writes_beside_moved_state`, its one admitted case),
   `init_from_a_subdirectory_refuses` (both forms above give the exact text,
   and neither `<root>/src/.brokkr` nor `<root>/.brokkr` is written),
   `doctor_names_the_folder` (each of the three lines, exactly) and
   `doctor_checks_a_moved_folder` are added to
   `crates/brokkr-cli/tests/init_doctor.rs`. The last plants a half-moved
   repository, `init`'s new output under `.brokkr/` with a root
   `bundle.json` left beside it, runs `brokkr doctor` from the root, and
   asserts the folder line `folder: <root>/.brokkr (found from <root>), but
   <root>/bundle.json still stands at the root (decision 0083); move or
   remove it` exactly and an unhealthy exit; then, with that `bundle.json`
   removed and `.brokkr/agents/charters/implementer.md` left at the root as
   `agents/charters/implementer.md`, it asserts the compile line exactly,
   with the error `brokkr compile --bundle .brokkr` prints for that
   folder, and an unhealthy exit.

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
- **No automatic migration (ruled 2026-10-12).** Earlier drafts of ruling
  4 built `brokkr init --migrate`, a move proved in a scratch folder before
  it was made, and six landing reviews each found a kind of dependency the
  move did not carry. The operator ruled to drop it: "we are the only
  consumers of brokkr so far, so we'll migrate with an agent, no need to
  spend time on a secure deterministic migration". Ruling 4 therefore
  refuses and lists the moves, and `brokkr doctor` checks the result. A
  deterministic migration may return as a later decision if adopters ask
  for one.
- **A composed bundle in an adopter's repository.** Ruling 1 lets no
  bundle in `.brokkr/` compose a base, because a base is read from the
  bundle directory's parent, which for the folder is the repository root.
  An old root bundle that composes one reads its base from outside the
  repository, so after the hand move `brokkr doctor` reports ruling 1's
  refusal. Keeping a composed adopter
  bundle would need a new place for its base, such as a
  `.brokkr/recipes/`, which is a change to composition, not to the layout.
  Recommended as ruled; a later slice may give the folder a base library
  only on its own ruling.
- **The capability broker's secrets place.** The broker's sealed plan
  store (`crates/brokkr-cli/src/broker/bootstrap.rs` and `session.rs`)
  depends on where the secrets store stands, and another session owns
  the broker (U6c8, #625). Before the build moves the secrets default to
  `.brokkr/state/secrets.env`, it pings the session that owns the broker,
  and moves the default only once that session has answered. This
  decision moves no default.
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
- An existing adopter is refused, with the moves to make by hand or with an
  agent, and keeps every journaled run, because the journal moves with its
  `-wal` and `-shm`; `brokkr doctor` checks the moved folder.
- Brokkr's own repository is unchanged by the build.

## Addendum — 2026-10-15, the operator's rulings on the build (#634)

The build landed in three slices: discovery, the defaults and the
old-layout refusals (#672); `init` and `doctor` (#686); and the guides
with this addendum. The operator ruled on #634 as the slices were
reviewed. This addendum records those rulings, each with its date, and
the rulings above keep their own text: where one below narrows or
supersedes a sentence above, this addendum is what binds. Its text is
the operator's to accept in the pull request that lands it.

**2026-10-13, four rulings on the first slice.**

1. **A recipe library keeps its layout.** A repository whose root holds
   a `recipes/<name>/bundle.json` is a library, exempt from ruling 4's
   old-layout refusal, and it keeps its layout whole, a tracked root
   `bundle.json` beside `recipes/` included. This narrows ruling 4's
   "No library root holds one": a library root may, and it is no root
   layout. A library outranks a folder beside it, so its journal never
   moves to a fresh one under the folder.
2. **`init` reads and writes one path.** `brokkr init <dir>` detects the
   stack of `<dir>` and writes `<dir>/.brokkr/`, the folder it creates,
   and nothing else. This is the simpler rule the second item of "Asked
   of the operator" recommended.
3. **Doctor's folder line takes the report's usual status column**:
   `ok`, `warn` or `MISSING`, like every other line.
4. **The compile-derived move list is dropped.** Migration is done by an
   agent, so the old-layout refusal lists the state's moves only. Ruling
   4's first group, the files the compile reads, is not built, and with
   it go the line `its bundle.json does not compile, so the files it
   reads cannot be listed: <compile error>` and the binding
   `old_root_layout_that_does_not_compile_refuses`. The root layout's
   refusal lists the state's moves, then says in one line to move
   `bundle.json` and every file it reads into `.brokkr/` at the same
   relative paths and to name the journal `"state/forge.db"` and the
   realm path `".."` in `.brokkr/realms.json`, and closes with its
   `Brokkr reads no script or charter` line. The refusal's example in
   ruling 4 is superseded where it shows `git mv` lines for
   `bundle.json`, `policy.json`, the adapters, the agents and the
   scripts: those moves are the agent's, and the list holds the state's.

**2026-10-14, links (option 1).** Any symbolic link in or under `.forge`,
or a `.forge` that is itself a link, makes the old-layout refusal list no
moves at all. It refuses by name instead, at any depth:

```text
<path> is a symbolic link, and Brokkr lists no moves through or for one (decision 0083; the operator's ruling of 2026-10-14); nothing was opened: move the state into .brokkr/state yourself, then run again
```

Detection is unchanged: each entry is judged by `lstat`, as ruling 4's
"Every entry is read by its own entry" has it. This narrows ruling 4's
sentence "The refusal lists the moves for the operator to make", which
holds only where no link stands in or under `.forge`.

**2026-10-14, the second slice, R1 to R7, as built.**

- **R1, an existing folder.** `init` refuses a `.brokkr/` that holds
  anything but a regular-file `.gitignore` and a `state/` directory,
  each judged as the entry itself, naming each entry it holds:
  `<root>/.brokkr already holds <entries> (decision 0083); init writes a
  folder only where none stands, or beside moved state (.gitignore and
  state/ alone): move or remove them`.
- **R2, a library.** `init` in a recipe library refuses and writes
  nothing: `<root> is a recipe library, which keeps its own layout and
  reads no .brokkr/ folder (decision 0083; the operator's ruling 1 of
  2026-10-13): init writes nothing here`.
- **R3, init's output.** After compiling the folder, `init` prints
  `initialized <root>/.brokkr (digest <digest>), writing:`, every path it
  wrote relative to `<dir>`, one per line and indented, its notes (the
  agent CLI it scaffolded for, and the MCP notice of decision 0065), and
  last `commit the folder: git add .brokkr && git commit`.
- **R4, doctor's folder line**, the report's first, in its forms:
  `folder: <root>/.brokkr (found from <cwd>)`; `folder: none from <cwd>
  up to <root> — brokkr init . writes one`; `folder: none: <root> is a
  recipe library, which keeps its own layout (decision 0083)`; the
  first line of an old shape's refusal or of a discovery refusal;
  `folder: <root>/.brokkr (found from <cwd>), but <path> still stands at
  the root (decision 0083); move or remove it`, one per entry left; and
  `folder: <root>/.brokkr (found from <cwd>), but its compile refuses
  (decision 0083): <compile error>`. Each but the first three makes the
  report unhealthy.
- **R5, doctor's bundle.** With a folder found, no old shape beside it
  and no `--bundle`, the folder is doctor's bundle for the whole report:
  its agents, provider, capability and bundle lines read it as
  `doctor --bundle .brokkr` reads it. Ruled 2026-10-14 and placed in
  the third slice on 2026-10-15. As built, the folder is compiled once,
  under the map doctor inspected, as `--bundle` compiles it. Where the
  map a run loads loads, it is that map, and the one compile is both the
  folder line's and the report's bundle. Where it will not load, its
  refusal is the folder line's, and the rest of the report still reads
  the folder as `doctor --bundle .brokkr` reads it. A folder whose
  compile refuses is, to the rest of the report, a bundle that did not
  compile, never no bundle given, and its refusal is named once: on the
  folder line, with no bundle line, unless the folder line already names
  the map's refusal, when the bundle line names it. The bundle line names
  the folder by its path, where `--bundle` names it as spelled. R4's
  forms are unchanged.
- **R6, the codex adapter's citation.** The codex adapter's evidence
  citation, `.forge/tasks/controller-codex-web-search-switch-2026-09-21.json`,
  stays word for word, in a codex scaffold's
  `.brokkr/adapters/codex.json` too: it cites a measurement made in
  Brokkr's own repository, a library, and is evidence, never a path the
  scaffold reads. Ruling 1's "no file `init` wrote contains the text
  `.forge/`" holds for every byte but that citation.
- **R7, a missing `<dir>`.** `init` of a `<dir>` that does not exist
  refuses in discovery's words, `reading <dir>: No such file or
  directory (os error 2)`, and creates nothing.

**2026-10-15, `recipes add` in a folder repository (#675, option A).**
`brokkr recipes add` refuses in a folder repository and writes nothing,
because the `recipes/<name>/bundle.json` it would install is a library's
mark, which outranks the folder (the first ruling of 2026-10-13). It is
judged by the union of two readings: the working directory's layout, or
the layout `--dir` lands in, `--dir` judged as spelled and as resolved
and `--name` by its spelling. The accepted texts are:

```text
<root> keeps its Brokkr in <root>/.brokkr (decision 0083): recipes add would install a recipe library there, which outranks the folder and moves every default out of it (#675); a folder repository's bundle is .brokkr/ itself, so nothing was written
--dir <dir> holds a part that is not a plain name below <standing>, the nearest entry on it that stands, so recipes add cannot judge where it would write (#675); nothing was written
--name '<name>' is not one plain path component, so the recipe would not be one directory of <dir> (#675); nothing was written
```

`--dir ""` refuses, and an `extends` spelled `base/` does not compose.
Carried as a LOW residual: a folder repository whose own `recipes/`
links outward. Its fix is a change to ruling 4's layout rule, and it is
not made here.

**What comes next.** Recipes in a folder repository go next to
[#696](https://github.com/feedback-loop-ai/brokkr/issues/696), which
carries its own rulings and its own addendum.

## Addendum — 2026-10-16, the recipe shelf (#696)

Status: proposed; only the operator accepts this addendum.

The operator accepted every recommendation of #696's write-up in chat on
2026-10-15, as the issue's rulings 1 to 9 state them, and amended its
ruling 6 the same day. This addendum records them as built, each with its
text. The rulings above and the addendum of 2026-10-15 keep their own
words; where this addendum narrows or amends one of their sentences, it
binds once accepted.

**1. The place.** A folder repository's named recipes stand on its
shelf, `.brokkr/recipes/<name>/`, tracked beside the folder's own
`bundle.json`, which stays the default recipe. The shelf is no library
mark: a library's mark is a root `recipes/<name>/bundle.json`, so a shelf
recipe never changes what the root holds.

**2. The defaults.** With a folder discovered, the shelf is the default
library of `recipes add`, `recipes list` and `recipes show`, and the
default `--recipes-dir` of `--recipe` on `run`, `resume`, `rerun` and
`queue add`. An explicit `--dir` or `--recipes-dir` still wins. Without a
folder every default stays `recipes`. Ruling 3's table gains a row:

| Fact | Flag | With `.brokkr/` discovered | With none (the library layout) |
|---|---|---|---|
| named recipes | `--dir` (`list`, `show`), `--recipes-dir` | `.brokkr/recipes/` | `recipes/` |

While nothing stands at the default shelf, `recipes list` lists nothing
and exits 0. An explicit `--dir` that cannot be read, and a shelf that is
a file or a link to nothing, still print `warning: recipes dir <dir>:
<error>`.

**3. The default bundle's identity.** For a folder bundle only, the
manifest walk skips `recipes` beside `state`, so installing or editing a
shelf recipe never moves the default bundle's digest. Every other bundle
pins a top-level `recipes/` as before. A default bundle that `extends` a
shelf recipe pins that layer as decision 0017 pins every composed base:
the skip keeps the shelf out of the folder's own files, never out of the
composition.

The skip covers the shelf and nothing else, so it does not cover what
ruling 5 copies: an install that copies an agent definition or a charter
into `.brokkr/agents/` moves the default bundle's digest by exactly the
files it copied, because the folder bundle pins its agents as it always
has, and one that copies none leaves the digest where it was. Pinning less
would let an agent a run hires change unseen, so the build keeps the pin
and carries the exception here; a different identity rule for the folder
bundle would be a decision of its own.

**4. Composition.** This amends ruling 1's sentence "A bundle in
`.brokkr/` composes no base" and retires its refusal text: the folder
bundle's base library is its shelf, `<root>/.brokkr/recipes`, never the
repository root, and every other layer's is its parent, as before. Shelf
recipes may extend each other, and `.brokkr/bundle.json` may extend a
shelf recipe. An `extends` naming anything the shelf does not hold
refuses with the composition's own text, which names the shelf and reads
nothing at the root:

```text
bundle: <root>/.brokkr/bundle.json: extends '<name>', which is not a recipe in <root>/.brokkr/recipes
```

A shelf that is a link fails the composition's real-directory rule: a
base must be `a real directory in <root>/.brokkr/recipes`.

**5. Agents a recipe names.** `recipes add` copies into the workspace's
agent library (`.brokkr/agents/` with a folder, `<root>/agents/`
otherwise), from the source's: each agent definition a site of a copied
recipe names (a seat, a panel member, a sequence step, a select case or
default), read as typed data from each copied recipe's own
`bundle.json`, and the charter that definition names. The source's
library, for a bundle whose parent directory is named `recipes`, local
or in a clone, is that directory's sibling `agents/`, so a cloned
folder's shelf recipe brings its `.brokkr/agents/`; any other git
source's is its clone's `agents/`, and any other local source has none,
and nothing is copied. An agent's own
`extends` base is not copied: it resolves in the workspace's library.
The capability definitions a copied agent requests are copied with it,
as the extension below rules (#703). Every
file is planned before anything is written, and a definition, a charter
or a source entry is walked without following a link. The copies join
the install's kept-or-removed set, so a compile that refuses removes them
with the recipe. A file already present and identical is skipped. The
refusals, each before anything is written:

```text
recipes add replaces no agent definition, charter or capability definition, and the source's differ from what stands at <path>, <path> (#696): reconcile or remove each; nothing was written
recipes add cannot read <path>: <cause> (#696); nothing was written
<bundle.json> names agent '<name>', which is not an agent name (^[a-z][a-z0-9-]*$) (#696); nothing was written
<definition> names charter '<charter>', which is not a relative path of plain names in its library (#696); nothing was written
<definition> names charter '<charter>', which its library <library> does not hold, so the copy would bind to whatever this workspace holds there (#696); nothing was written
<path> is a link, or not the file or directory recipes add copies from the source's agent library or capability definitions (#696); nothing was written
```

`recipes add` prints `added <what> at <path>` after its recipe lines,
one line per file it copied: `agent '<name>'`, or `charter <charter> of
agent '<name>'`.

*Extension, 2026-10-17: capability definitions (#703).* Status:
proposed; the operator ruled its substance on #703, and only the
operator accepts this text. An agent definition `recipes add` copies
brings the abstract definition of each capability it requests,
`capabilities/<name>.json`, from the source's operator configuration
into the workspace's: each is the directory a compile reads definitions
from (decision 0065), by the compile's one rule, beside the map, else at
the root. The workspace's is `.brokkr/capabilities/` in the folder
`init` writes, else its repository's root. The source's is judged at the
root its agent library stands in, by ruling 1's one walk from the
directory the library is in: the repository's root where that directory
is its `.brokkr` folder, else that directory, never a folder by its name
alone (a library checked out as `.brokkr` with its own `.git` is a
repository of its own), as that root's own compile reads them; a walk
that refuses, and a source map that cannot be read, refuse by the
`cannot read` text above, once an agent requests a capability. They
join the one plan with the agents and charters, by its rules: planned
before anything is written, walked without following a link, at the
source and at the workspace: every directory between the workspace's
root and each file, `agents/`, `agents/charters/` and `capabilities/`
among them, is judged as the entry itself and refused by the third text
below where it is a link, to anything or nothing, inside the root or
outside it (#735), and by the fourth where it stands and is not a
directory, before anything is written (#779), while a `.brokkr/` folder
that is a link is refused
before any plan by this decision's folder rule, in its own words; the
workspace's own entry for each file,
an agent's and a charter's too, is opened once without following a link
and judged by that handle's own status, so the entry judged is the file
compared (#735), compared only where it is a plain file, and refused by
the second text below where it is a link, to anything or nothing, or not
a plain file, a socket and a FIFO among them (#779); skipped when present and identical,
refused by the text above when present and different, and removed with
the recipe when its compile refuses. A name outside the capability
grammar is never walked; the compile names it. A definition the source
does not hold refuses, even where the workspace holds one, since the
copy would bind to whatever the workspace defines. Each copy prints
`capability '<name>' of agent '<agent>'`. A definition is no bundle
file, so it never enters a bundle's pinned files; but a compile pins, in
its capability record, every definition its library's agents request, so
the folder bundle's digest also moves by the pin of each definition a
copied agent requests. An install onto the personal shelf copies no
definition, as it copies no agent. Only the agents a recipe names bring
theirs: a library copied by hand brings its own `capabilities/` by hand.
The new refusals:

```text
<definition> requests capability '<name>', whose definition the source's <capabilities> does not hold, so the copy would bind to whatever this workspace defines there (#703); nothing was written
the workspace's <path> is a link, or not a plain file, where recipes add would copy one; recipes add neither reads through nor replaces it (#703): remove it; nothing was written
the workspace's <path> is a link, where recipes add would copy a file beneath it; recipes add neither reads through nor replaces it (#735): remove it; nothing was written
the workspace's <path> is not a directory, where recipes add would copy a file beneath it; recipes add does not replace it (#779): remove it; nothing was written
```

**6. `recipes add` loses `--dir`** (amended 2026-10-15; the first ruling
6, narrowing #675's check to the destination, is superseded). It installs
onto the shelf of the repository it runs in, from any directory of it:
`<root>/.brokkr/recipes/` with a folder, `<root>/recipes/` in a recipe
library or a bare repository. It compiles every copy against that root's
agents and adapters, and reads that root's map. `recipes add … --dir x`
is clap's `error: unexpected argument '--dir' found`. `--dir` stays on
`recipes list` and `recipes show`, and `--recipes-dir` on `--recipe`,
because they only read. The `--dir` reading of the addendum of
2026-10-15 goes with the flag: its `--dir` text, the walk of `--dir` as
spelled and `--dir ""`. Its `--name` text stands; `<dir>` in it is now
the shelf. The judgement of the destination as it resolves stays, because
a shelf may be a link: the walk is taken from the nearest entry standing
at the shelf and from every ancestor of it, as spelled and as resolved,
and an install whose shelf resolves into a folder repository anywhere but
that repository's own `.brokkr/recipes` refuses. That keeps #675's holds
refused, a bare directory's `recipes` linked into a folder repository's
root `recipes/`, a folder's `.brokkr/recipes` linked to `../recipes`, and
a nested repository below a folder, and it no longer refuses a folder
installing onto its own shelf. Its text stands where the earlier
addendum's first text stood:

```text
recipes add would install into <place>, inside the folder repository <root> but not its shelf <root>/.brokkr/recipes/ (decision 0083; #675, #696): a folder repository keeps its named recipes on its shelf, so nothing was written
```

A source that holds the shelf, such as the folder's own bundle, would be
copied into itself, so it refuses before anything is made:

```text
recipes add would copy <source> into <shelf>, which lies inside it (#696): install a bundle directory that does not hold the shelf; nothing was written
<source> is a repository's .brokkr folder, its default bundle, not a recipe: name a recipe on its shelf (<source>/recipes/<name>) or in a library as the source (#696); nothing was written
```

The LOW residual of 2026-10-15 stands: a folder repository whose own
root `recipes/` links outward.

**7. Fetching from a repository's subdirectory** shipped apart, in
#695: `recipes add <git-url> --path <relative path>` names the bundle's
directory inside the clone, a relative path of plain names judged by its
spelling before the clone and walked without following a link after it,
and the bases its recipe extends come from beside it. The operator ruled
on 2026-10-15 that it needs no decision of its own and is recorded here,
with its accepted refusal of a local source:

```text
--path names a directory inside a git source's clone, and <source> is a local directory: name <source>/<path> as the source itself (#695); nothing was written
```

**8. Doctor.** With a folder found and no old shape beside it, doctor
prints one line per shelf recipe after the folder line, sorted by name. A
broken one warns and never makes the report unhealthy, and no shelf is
no line:

```text
ok       recipe <name>: <digest> at <root>/.brokkr/recipes/<name>
warn     recipe <name>: <root>/.brokkr/recipes/<name> does not compile (#696): <refusal>
warn     shelf: <root>/.brokkr/recipes: <error>
```

**9. Vehicle and order.** This addendum, in #696's pull request, built
after #634's third slice; the quickstart gains a step that tries another
recipe on the shelf.

## Addendum — 2026-10-16, the personal shelf (#697)

Status: accepted — operator ruled 2026-10-17 on slice A (PR #722) and
2026-10-21 on slice B (#697).

Ruling 7 left #432 a personal overlay of tracked configuration. The
operator ruled its first slice in chat on 2026-10-15, as #697's rulings
1 to 7 state them. This addendum records what its first pull request
built, ruling by ruling, and names what it leaves to the next. The
rulings above and the addenda of 2026-10-15 and 2026-10-16 (#696) keep
their own words.

**1. The place.** The personal shelf is `recipes` in the machine's
configuration directory, the one decision 0068's addendum gives
`host.json`: `$XDG_CONFIG_HOME/brokkr/recipes/<name>/`, else
`$HOME/.config/brokkr/recipes/<name>/`, on Linux and macOS alike. A
value that is not an absolute directory is passed over. Both files take
that directory from one function, so `host.json` and the shelf cannot
part. The shelf is in no repository, and no library mark is judged
there: an install onto it applies no repository, folder or library
detection to the shelf or to any directory above it, and keeps every
hold that guards the write itself. `XDG_CONFIG_HOME` and `HOME` name the
operator's own configuration, and Brokkr does not judge whether that
directory lies inside a repository. Where neither variable names an
absolute directory, an install
refuses with `host.json`'s own text, as the lookup does:

```text
no host configuration can be found: neither XDG_CONFIG_HOME nor HOME names an absolute directory
```

Where the machine places no shelf, a reader of the shelf has nothing to
read: `--recipe`, `recipes list` and doctor read the project's shelf
alone.

**2. No shadowing.** `--recipe <name>` on `run`, `resume`, `rerun` and
`queue add`, and `recipes show <name>`, read the project's shelf, then
the personal one. A name with any entry standing on both refuses, naming
both paths, and neither shelf wins:

```text
recipe '<name>' stands on both the project shelf, at <project>/<name>, and the personal shelf, at <personal>/<name> (#697): neither shelf wins, so rename or remove one of them
```

An entry at the name that cannot be read refuses as `reading <path>:
<cause>`, never read as absent. The project's shelf is the one #696's
ruling 2 gives, so an explicit `--recipes-dir` or `--dir` names it, and
the personal shelf is still read beside it, under the same refusal.
`--recipe` names a recipe, never a path: a name that is not one plain
recipe name, judged by its spelling, so `x/` and `./x` as much as `a/b`,
`..` or an empty name, refuses on every one of those verbs before
either shelf is read:

```text
--recipe '<name>' is not one plain recipe name, so it would not name one recipe on a shelf (#697): name the recipe as it stands on its shelf, or give --bundle <dir> for a bundle elsewhere; nothing was run
```

A shelf that cannot be read, the project's or the personal one, is
never read as an empty one: it refuses on every one of those verbs
before either recipe is taken. Nothing standing at a shelf is an empty
shelf, but the nearest entry standing at it or above it must resolve to
a directory, so a link to nothing, a looping link, a file, or an entry
whose metadata cannot be read refuses, naming the shelf and the entry
judged. A shelf's path is normalised once, where it enters, so every
spelling of one shelf is judged at the same entry. `recipes add` judges
the shelf it installs onto by the same rule:

```text
the <origin> shelf <shelf> cannot be read (#697): <path>: <cause>; a shelf that cannot be read is never read as an empty one, so mend or remove it; nothing was run or installed
```

A name on neither shelf refuses as before,
naming the project's shelf. A queued launch keeps the shelf it resolved
when it was queued, by its path alone, and is admitted and started only
where that path is a shelf the admitting process places (ruling 3).

**3. Composition runs one way.** Every compile a launch, `compile`,
`recipes` and doctor take (`launch::compile_for`, and a resume's) reads
its layers across the workspace's shelves: the project's, the folder's
own bundle, and the personal shelf this process's environment places
(`places::personal_shelf_here`, the one read the CLI's verbs take too).
The project shelf is the one `--recipes-dir` on `run`, `resume` and
`rerun`, or `--dir` on `recipes show` and `recipes list`, selects, where
one does (`launch::compile_on`), else the workspace's own, so a personal
recipe composes across the shelf its `--recipe` was read beside, and is
listed as it is shown. A queued launch holds no selected shelf, so it
composes across its workspace's own, and `queue add` refuses a
`--recipes-dir` naming any other, compared as spelled, before anything
is queued:

```text
--recipes-dir selects the project shelf <shelf>, and a queued launch composes only across its workspace's own, <own> (#697): queued-launch/v1 holds no selected shelf, so the entry would be admitted and started across the workspace's shelf instead; queue it without --recipes-dir, or start it with `brokkr run`; nothing was queued
```

queued-launch/v1 keeps a recipe's shelf by its path, not its origin, and
the process that admits the entry judges each layer's origin by the
shelves it places. So an entry whose recipe stands on a shelf that is
neither its workspace's project shelf nor the personal shelf the
admitting process places, as one queued under another `XDG_CONFIG_HOME`
or `HOME` holds, refuses where admission measures it, which holds the
entry, and where it is rebuilt to start: one check in
`QueuedLaunch::rebuild`, which both take, comparing the shelves as
spelled. Where the admitting process places no personal shelf, the
project's alone stands. An entry naming its bundle's directory is not
judged here:

```text
queued recipe '<name>' stands on the shelf <shelf>, which is neither its workspace's project shelf, <own>, nor the personal shelf this process places (<personal>) (#697): a queued launch composes only across shelves its admitting process places, so the entry would compose with no origin for that shelf's layers and past the refusal of a name on both shelves; admit it under the XDG_CONFIG_HOME and HOME that queued it, or start it with `brokkr run`
```

`<personal>` reads `it places none` where neither variable places one. A
queued-launch/v2 carrying each layer's origin, which would admit such an
entry anywhere, is left to the operator.

Each layer's origin is judged by where its directory stands: the
folder's bundle by the one judgement of a layout, else `personal` or
`project` where its parent is that shelf, else `library`. The origin
decides where its `extends` is read. A personal recipe reads its base
from its own shelf where the name stands there alone, else from the
current workspace's project shelf (decision 0017; #696's ruling 4), and
reaches the folder's own bundle through the reserved name `.brokkr`,
which no other layer may name. A base with an entry on both shelves
refuses with ruling 2's own text before anything at it is read, so a
personal copy that is not even a recipe still refuses as shadowing:

```text
recipe '<name>' stands on both the project shelf, at <project>/<name>, and the personal shelf, at <personal>/<name> (#697): neither shelf wins, so rename or remove one of them
```

A `.brokkr` from a workspace with no folder refuses:

```text
the personal recipe at <personal>/<name> extends '.brokkr', the folder's own bundle, but this workspace has no .brokkr folder (#697): run it from a folder repository, or extend a recipe on a shelf
```

A project recipe, or the folder's bundle, reads its own shelf alone, so a
name the project shelf holds composes there whatever the personal shelf
holds. A base it names that only the personal shelf holds refuses, each
with its own text, naming both paths:

```text
the project recipe at <root>/.brokkr/recipes/<name> extends '<base>', which stands only on the personal shelf, at <personal>/<base> (#697): a project recipe never extends a personal one, since teammates and CI could not compile it, so put '<base>' on the project shelf or extend a recipe there
the folder's bundle at <root>/.brokkr extends '<base>', which stands only on the personal shelf, at <personal>/<base> (#697): the folder's bundle never extends a personal recipe, since teammates and CI could not compile it, so put '<base>' on the project shelf or extend a recipe there
```

A shelf a compile reads is judged by ruling 2's one rule
(`places::resolving_shelf`) each time it is read: the project shelf for a
personal layer's base, and the personal one for a project layer's base
its own shelf lacks. One that cannot be read refuses in ruling 2's words
under the compiler's `bundle: `, on `compile --bundle` and `run --bundle`
as on every verb that names a recipe, never read as an empty one; a
shelf with nothing standing at it, no entry and no link, is an empty
one. `recipes add --personal` copies a base its plan reaches as before, but
never `.brokkr`, which the compile reaches in the workspace. Nothing
installed or edited on the personal shelf moves the folder bundle's
digest or any library bundle's, since the folder's walk skips its shelf
and no other walk reads the personal one.

**4. Same checks.** `recipes add --personal`, `recipes list`, doctor and
a run compile a personal recipe through the one compile every recipe
takes (`launch::compile_for`), against the workspace's agents and
adapters, so it gets every refusal a tracked recipe gets and has one
digest on every surface. An install onto a shelf outside the repository
copies no agent or charter into the repository's library. A recipe that
hires an agent the workspace lacks refuses to compile, and the install
removes it.

**5. Origins.** A run from a personal recipe pins its composed manifest
as every run does, and a resume reads that pin and refuses drift, so an
edit of the personal recipe, or of the project recipe it extends, after
the run started is refused, the base's edit as its `@compose/` entry:

```text
run '<run>' pins a different bundle: changed: <file>
```

A run's `run/started` records each layer's origin and directory, leaf
first, as `layers` beside its `charters`, where the bundle composed more
than one layer or stands on the personal shelf; a bundle of one layer
elsewhere starts with the payload it always did. `recipes show` and
`compile` print the same list, from the same compile:

```json
"layers": [
  {"origin": "personal", "path": "<personal>/<name>"},
  {"origin": "project", "path": "<root>/.brokkr/recipes/<base>"}
]
```

The origins are evidence, outside the pin: the pin covers every layer's
bytes through the manifest's chain, and no manifest key or witness
digest moves.

**6. The CLI.** `recipes add --personal <source> --name <name>` installs
onto the personal shelf, from any directory of the repository whose
agents and adapters compile it. It is `add`'s only destination choice.
The directories it makes above the shelf are removed again when the
install refuses. `recipes list` prints the project's recipes, then the
personal ones, each line with its origin, `project` or `personal`,
before its path:

```text
<name>	<digest>	<n> phases	<seats>	<cost>	<description>	<origin>	<path>
```

Doctor prints the personal shelf's lines after the shelf's, where a run
reads named recipes: after the folder found with no old shape, and after
a recipe library's line. They follow the shelf lines' rules: a broken
recipe warns and never makes the report unhealthy, and no shelf is no
line.

```text
ok       personal recipe <name>: <digest> at <personal>/<name>
warn     personal recipe <name>: <personal>/<name> does not compile (#697): <refusal>
warn     personal shelf: <personal>: <error>
```

**7. Vehicle and order.** This addendum is in #697's first pull request,
built after #696. Its second pull request built composition across
shelves (ruling 3) and the origins of ruling 5 with their proof on
resume, and stated them here. Personal agents and adapters, personal
realms and #432's stock-map test are later #432 work.

*Extension, 2026-10-17: the shelf follow-ups (#726).* Status: accepted
— operator ruled 2026-10-18 (PR #737). It closes three of the LOWs the
operator accepted on #697's first pull request.

A shelf spelled through `..` beneath a part of it that does not stand is
never read as an empty one, by ruling 2's rule. The kernel cannot
resolve a `..` below a directory that does not exist, so it reported
such a shelf absent, and an absent shelf read as an empty one:
`--recipes-dir .brokkr/recipes/sub/..`, with no `sub`, took the personal
recipe past the refusal of a name on both shelves, and
`XDG_CONFIG_HOME=<root>/nope/../cfg` the project's. The part of a shelf
beneath the nearest entry standing at it or above it, the current
directory for a relative shelf, must hold no `..`. A shelf whose part
does refuses, on either origin, every verb ruling 2 names and `recipes
add --personal`, naming the shelf as spelled and that entry. A project
shelf that climbs or cannot be read refuses with or without a personal
shelf placed. A `..` beneath an entry that stands is the kernel's to
resolve, as before.
`recipes list` and doctor, which never refuse, warn of such a personal
shelf by their `warning: recipes dir` and `warn personal shelf` lines,
rather than showing no shelf.

`recipes add --personal` makes the directories above the shelf one at a
time, from the outermost, held by a guard that stands before the first
is made. A directory that cannot be made, a name too long for the
system included, refuses naming the shelf and that directory, and every
directory made before it is removed.

`recipes add --personal` refuses at install a name the project's shelf
already holds, the shelf `--recipe` reads first, judged as resolution
judges it: a project shelf, or an entry at the name, that cannot be read
refuses by ruling 2's texts. Every name the install would copy is
judged, the recipe's and each base its `extends` chain copies onto the
personal shelf, before any is copied, so an install never makes a
project recipe unavailable by copying its base. Whatever was made for
the personal shelf is removed again. A name that is no plain name
refuses by `add`'s own `--name` text.

```text
the <origin> shelf <shelf> climbs by '..' out of a directory that does not stand beneath <at> (#726): a shelf spelled through what does not exist is never read as an empty one, so spell it without the '..'; nothing was run or installed
the personal shelf <shelf> cannot be made (#726): <directory>: <cause>; every directory made for it was removed again, so nothing was installed
recipe '<name>' stands on the project shelf, at <project>/<name>, so installing it onto the personal shelf, at <personal>/<name>, would stand it on both (#726): neither shelf would win, so rename or remove one of them; nothing was installed
```

## Addendum — 2026-10-29, the walk skips git's ignored trees and a bundle past a bound (#908)

Status: proposed; only the operator accepts this addendum.

The operator ruled on #908 (2026-10-29, "go with recommended on the rulings")
that a bundle compiled from a repository's root must not pin the repository's
build and fetch trees. The story's run started from such a root and its
`run/started` manifest pinned 110,209 files (96,276 under `target/`, 13,248
under `.cargo-home/`, 458 under `.git/`), so its identity moved on every
build and every reader of the run carried the event.

1. **The skip.** The manifest walk skips `.git` at the bundle's top level, as
   git's own metadata, and every path git reports ignored for the bundle's
   repository, at any depth. Git answers by its own rules — `git check-ignore`
   for the repository the bundle stands in, asked of the entry's full relative
   path, so `.gitignore`, `.git/info/exclude` and a global exclude rule alike
   stay git's to interpret, never a copy of them. A tree git reports ignored
   is skipped whole, and everything under it inherits that one answer. A
   bundle outside any git work tree skips only `.git`, as it did before this
   addendum. A `.git` stands above the bundle but git cannot answer for it:
   the bundle is refused by name with git's own stderr, never read as "pin
   everything" or "skip everything" (a failed discovery is not "not
   ignored"). The walk, the consumed inputs, the pinned-script lookup and the
   skip reason ask one full-relative-path function, so none can drift.

   The skipped trees are still searched for the one-entry rule, exactly as
   the other unpinned trees are: a consumed file, or a second name for one,
   inside an ignored tree at any depth is refused by name. "Pinning more never
   lets a file change unseen" holds of every file the compile or a
   pinned-script lookup reads.

2. **The bound.** A bundle whose pinned walk is past **5,000 files** or
   **64 MiB** is refused by name before any run starts, in the compile's own
   words, naming the count or bytes and the largest top-level trees. The
   refusal comes before `run/started` is written, so no run carries a
   manifest this large; it is judged from the walked paths, before any byte
   is hashed. Both numbers are an ask: the controller recommended them, and
   only the operator may move them.

3. **What identity covers.** This addendum amends ruling 1's account of what
   a bundle pins: a repository root's build and fetch output, top-level or
   nested, is no longer part of the bundle's identity. It changes no pinned
   file's bytes and no shipped recipe's digest, because no shipped recipe
   holds a `.git` or a git-ignored tree.
