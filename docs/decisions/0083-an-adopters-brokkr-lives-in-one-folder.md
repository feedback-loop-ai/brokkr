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
