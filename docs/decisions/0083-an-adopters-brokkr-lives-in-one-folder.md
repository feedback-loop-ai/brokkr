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
seat scripts. What the compile read by reference is recorded apart: the agents
its seats hire, with their charters (the manifest's `agents`), the `role` an
inline executable site names, a nested panel member, sequence step or select
arm included, read from the bundle's directory and bound to its site
(`parse_role` in `crates/brokkr-runtime/src/bundle/charters.rs`), the adapter
declarations that answered (`drivers`), the capability definitions a seat's
requirement consulted or a grant of the operated realm names, and the tool
dialect each such grant names, `dialects/tools/<name>.json` (`Authority::load`
and `Definitions::load` in `crates/brokkr-runtime/src/capabilities.rs`,
`ToolDialect::load` in `capabilities/dialect.rs`), all pinned by name and
digest (the manifest's `capabilities`), and the `./` scripts a stage runs
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
   bundle's identity. A bundle's script paths stay relative to the bundle,
   which is `.brokkr/` itself, so `scripts/verify-seat.sh` keeps its spelling.
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

4. **Old layouts are refused, and moved whole.** A recipe library is told
   apart by one mark: its root holds a `recipes/<name>/bundle.json` for some
   name, because a library's bundles live under `recipes/` (decision 0078)
   and `init` never wrote a `recipes/`. Three shapes are detected before any
   map or store is opened, by every verb that discovery serves:

   - **The root layout.** No `.brokkr/` is discovered, and the repository
     root holds a `bundle.json`: `init`'s old signature. No library root
     holds one.
   - **State only.** No `.brokkr/` is discovered, the root holds a `.forge`
     entry, whatever it holds, even nothing, and no `bundle.json`, and the
     repository bears no library mark: an adopter
     who named its bundle with `--bundle` every time, or one whose only
     Brokkr state is a secret, because `brokkr secrets set` creates
     `.forge/secrets.env` with no journal and no scaffold, as a run's
     `results/` and `ledger/` can stand with no `forge.db`. The predicate is
     the `.forge` entry itself, not any one file in it, so no Brokkr state
     is left at the retired path while the defaults read the folder. In a
     library, `.forge/` is the library's own state (ruling 3's last column)
     and is no old layout.
   - **Half moved.** A `.brokkr/` is discovered, and its parent still holds a
     `bundle.json` or a `.forge` entry. A repository is read from one
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
   it proves these moves before making any, and it moves scripts but reads none, so it needs --scripts-use-root-paths: scripts/ship-seat.sh, scripts/verify-seat.sh
   ```

   (The listing is abridged here; a refusal lists every move and every
   restatement of a typed field, and its last line names every script the
   moves hold, or drops its second clause when they hold none.) No line
   rewrites a file's text: `--migrate` changes no moved file's bytes but
   the map's paths and the typed fields of the restatement below, so the
   `.forge/` paths `init`'s old scripts and charters name move as written
   (the script rule below, and Asked). The moves are what the old compile read, so when the old
   bundle does not compile they cannot be listed, and the root layout's
   refusal is exactly this instead, still opening nothing:

   ```text
   this repository has Brokkr's old root layout and no .brokkr/ folder (decision 0083); nothing was opened
   its bundle.json does not compile, so the moves `brokkr init --migrate` makes cannot be listed: <compile error>
   make <root>/bundle.json compile, then rerun to see them; until it does, `brokkr init --migrate` refuses with the same compile error
   ```

   An old bundle that compiles by composing a base cannot be moved whole:
   the base is read from the directory above the repository, which
   `--migrate` never moves into, and a moved `.brokkr/bundle.json` composes
   no base (ruling 1). Moving the bundle would leave its `extends` to be
   read from `<root>/<name>/`, so the closing compile would refuse after
   every move, or compose a project's own directory of that name in place
   of the base. So when the old compile composed a base, the root layout's
   refusal is exactly this instead, still opening nothing, and `--migrate`
   refuses with the same text and moves nothing:

   ```text
   this repository has Brokkr's old root layout and no .brokkr/ folder (decision 0083); nothing was opened
   its bundle.json extends '<name>', read from <library>/<name>/, and a bundle in .brokkr/ composes no base, so `brokkr init --migrate` moves nothing
   make <root>/bundle.json compile with no extends, then rerun to see the moves
   ```

   The
   state-only refusal opens `this repository has Brokkr state in .forge/
   and no .brokkr/ folder (decision 0083); nothing was opened`, lists the
   state moves alone in the same form (for a repository holding only a
   secret, the one line `mv .forge/secrets.env .brokkr/state/secrets.env`),
   and closes `then brokkr init . writes the scaffold beside the moved
   state`. The half-moved refusal names each file still at the root, a
   `.forge` entry included, and says which layout is in
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
   else, and only once it has proved that move. One rule closes the
   migration, stated once here, so no kind of dependency needs its own
   arm: **the move list is what the old compile pinned, no move lands on
   a path the migration writes itself, and a proof compile over exactly
   the moved files must pin the same inputs, byte for byte, or nothing
   moves.** A dependency the move list does not carry is
   not reached by the proof, or is reached at a place the move empties,
   and either way it refuses before any move, whatever its kind, a kind
   nobody has listed yet included; a move the list does carry is never
   overwritten after the proof, because nothing the migration writes may
   be its destination. The bullets below say what the list
   holds, which places are the migration's own, how the proof is run, and
   the only changes made to a moved file's bytes: the map's paths, and the
   typed references the compile returns.

   - **Which files move** is what the old compile read by reference, never
     the manifest's `files` map, which pins every file under a root
     scaffold, a project's own `scripts/build.sh` included (Context): the
     bundle; the policy table it names; each agent a seat hires and the
     charter that agent names (the manifest's `agents`); the `role` of each
     inline executable site, a nested panel member, sequence step or select
     arm included (`parse_role`); each adapter
     declaration the compile read for a seat's harness and gates (its
     `drivers`); each capability definition a seat's requirement consulted
     or a grant names, `capabilities/<name>.json`, which moves to
     `.brokkr/capabilities/<name>.json`, and each tool dialect a grant
     names, `dialects/tools/<name>.json`, which moves to
     `.brokkr/dialects/tools/<name>.json` (the manifest's `capabilities`,
     its `definitions` and `dialects`), beside the moved map and agent
     library, where ruling 1 has them and the closing compile looks. The
     compile that lists the moves loads the grants of every realm the map
     holds, as `Authority::load` loads the operated realm's, because the
     world already reads every realm's house and dialect and a grant's
     names resolve against the map's directory whichever realm holds it;
     so a grant in another realm reads, and moves, its definition and its
     tool dialect too, and one that does not load gives the second
     refusal above. Each script a
     hired agent's command or a stage runs by a
     `./` path under the bundle; the map; and, for each `dialect` the map
     gives as a name, `dialects/<name>.json` and the texts it reads (`init`
     writes a name). A `house`, a `dialect` given as a path and a published
     crossing stay where they are, as their paths do. The build has the compile return that list as typed
     data; `--migrate` does not re-derive it. Each entry moves as it
     stands: a symbolic link the compile followed moves as a link, its
     text unchanged, beside the file it reached when that file is on the
     list too. The list grows by no kind of dependency on that kind's
     behalf. The base an overlay agent `extends`, which the library load
     reads but the manifest's `agents` does not pin, is not on it, and
     neither is a file a dialect given as a path reads, nor a link's
     target the compile did not pin; the proof below rules on each. A file no reference reached
     stays where it is and is named as left behind, whatever directory it
     stands in. `init`'s `agents/README.md` is one such file: its text
     depends on the stack `init` detected, so it is not told from an
     operator's own by its bytes. It stays, and `--migrate` writes
     `.brokkr/agents/README.md` afresh, as `init` would; when a reference
     did reach it, as a role or a charter, it is on the list, and its move
     lands on a place the migration writes, which refuses (below). An old bundle that
     does not compile refuses the migration with its compile error, as the
     root layout's second refusal above says, and one that composes a base
     refuses with the third, before any move.
   - **The old compile skips the old state.** In the root layout the bundle
     directory is the root, so today's walk reaches `.forge/` and refuses a
     `.forge/secrets.env` (`bundle contains a secrets store`): an adopter
     whose secret stands at today's default could never see the moves, and
     the second refusal's remedy would not be theirs. So the compile that
     lists the moves, for the refusal and for `--migrate` alike, skips the
     root's `.forge` as an unpinned top-level name, as ruling 1's walk skips
     `state`, and the old digest `--migrate` prints is that compile's. It
     skips nothing else: a `secrets.env` anywhere else under the root still
     refuses, as it does today. What the skip leaves unread is state, never
     configuration. A file under `<root>/.forge/` that the compile reads
     by reference (a role, a charter, a policy or a script) is state to the
     move list, and the proof refuses it, as a place the move empties.
   - **No script is read, so no script moves until the operator says its
     paths survive the move.** The move list stays what the compile read;
     the rule is to refuse, not to move more or to guess. A script moves
     from where it stands under the root to the same place under
     `.brokkr/`, so `scripts/verify-seat.sh` becomes
     `.brokkr/scripts/verify-seat.sh`, while the seat's working directory
     is the root before the move and after it (ruling 1's realm path `..`).
     Two kinds of path a script names change meaning with the move:
     - a path it builds from its own place, in any direction and at any
       depth: `"$(dirname "$0")/verify-lib.sh"` names
       `scripts/verify-lib.sh` before the move and
       `.brokkr/scripts/verify-lib.sh` after it, and
       `"$(dirname "$0")/../Cargo.toml"` names the root's `Cargo.toml`
       before it and `.brokkr/Cargo.toml` after it, whatever the move
       list holds;
     - a path from the root to a place the migration moves or empties:
       `policy.json` or `scripts/verify-seat.sh`, named from the root,
       names nothing after the move, and neither does any path through
       `.forge/`, which the migration removes. `.forge/ledger/x.md` names
       nothing, and `.forge/results/../../Cargo.toml`, which names the
       root's `Cargo.toml` before the move, names nothing after it, because
       a kernel resolves `.forge/results` before it climbs out of it.

     Every other path from the root, such as `Cargo.toml` or
     `scripts/build.sh`, names the same file before the move and after it.
     No move into `.brokkr/` keeps a script's own-place geometry, because
     the folder adds a component above every script it holds, and keeping
     the scripts where they stand breaks ruling 1's layout. A script's text
     cannot be read to a closed answer either (a variable, `eval` or another
     language spells a path any way it likes), so `--migrate` reads no
     script's text, rewrites none, proves none safe, and moves each one it
     moves byte for byte. The proof below pins a script's bytes, never what
     it will open when it runs. The rule is therefore closed over what it
     cannot read: a migration whose move list holds a script refuses before
     any move unless the operator passes `--scripts-use-root-paths`, which
     says, on the command line as `--db` does, that every path each moved
     script builds, a `.forge/` path and a `..` traversal included, names
     the same file after the move as before it, or the place that file
     moves to: none is built from the script's own place, one that names a
     moved file names it under `.brokkr/`, and one that names state names
     it under `.brokkr/state/`. Nothing is exempt, because nothing is
     rewritten. The refusal names every script the move list holds, one per
     line in byte order:

     ```text
     --migrate moves these scripts byte for byte and reads none of them (decision 0083); nothing was moved:
       <script>
     a path a moved script builds from its own place, such as "$(dirname "$0")/../Cargo.toml", or from the root to a file --migrate moves, or through .forge/, such as ".forge/results/../../Cargo.toml", names another file or none after the move: rewrite each as the path from the repository root it must name once the move is made, then rerun with `--scripts-use-root-paths`
     ```

     With the flag, the migration goes on, and every file the move list
     lacks stays where it stands, in whatever directory, and is named as
     left behind; a moved script reaches it by its path from the root,
     which the move keeps. A migration whose move list holds no script, as
     in the state-only shape, needs no flag, and the flag changes nothing
     there. `init`'s own seat scripts build no path from their own place
     (each place they write is the result path's directory, which the
     prompt names), but its old `scripts/ship-seat.sh` names
     `.forge/ledger/$run_id.md`, so the operator rewrites that line as
     `.brokkr/state/ledger/$run_id.md` before giving the flag, and
     `scripts/verify-seat.sh` is the file an operator fills in, so their
     bytes prove nothing, and the flag is asked of every migration that
     moves a script. This is the trade the rule takes: one flag, said once,
     in place of reading a script, and a script whose paths do not survive
     the move is rewritten by the operator before it. A false flag is the
     operator's word against the script, which nothing that reads no
     script can check; a moved script that writes under `.forge/` after
     the move recreates it beside the folder, and every verb then refuses
     the half-moved shape, so that much fails closed. A script that only
     reads does not fail closed: a probe through `.forge/` or from the
     script's own place names nothing, or another file, after the move,
     and the script decides on that as on any absent file, so a gate that
     runs its failing suite only when `.forge/results/../../Cargo.toml` is
     found writes `pass` after a migration given a false flag. That
     fail-open is the flag's to prevent, said on the command line as `--db`
     is, and Asked puts the trade to the operator. A charter's or a
     role's prose is not read for paths either: the proof pins its bytes,
     and a seat that follows a `.forge/` path in it recreates `.forge/` the
     same way. Whether to keep `init`'s old `.forge/` texts working is
     Asked.
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
     that resolves to a place the migration moves or empties, or reads
     a file there, as a dialect given as a path may read an instruction
     text from a hired charter, is the proof's to refuse. A `dialect`
     given as a name is kept, because the dialect library moves with the map
     into `.brokkr/dialects/`, and so is a grant's `dialect`, a tool
     dialect's name, because `dialects/tools/` moves with it. The map's
     digest moves, and the output says so.
   - **A moved file's typed references are restated** as the map's are. The
     compile reads three paths that one moved file holds to another: the
     bundle's `policy`, resolved against the bundle's directory (`own_table`
     in `crates/brokkr-runtime/src/bundle/compose.rs`); a hired agent's
     `charter`, resolved against the agent library (`contained` in
     `crates/brokkr-runtime/src/agents/load.rs`); and an inline executable
     site's `role`, at any depth, resolved against the bundle's directory
     (`parse_role`, which reads it through `owned_input`, whose `queue` in
     `bundle.rs` admits an absolute spelling under that directory). Each
     admits an absolute
     spelling that stands under its base, so the build has the compile
     return each one, with its spelling, beside the move list. A relative
     one is kept byte for byte: the file that holds it and the file it
     names move together, to the same places under `.brokkr/`, so `init`'s
     `policy.json` and `charters/implementer.md` name the same files after
     the move. An absolute one is restated as the new absolute path in the
     same field, so `<root>/policy.json` becomes `<root>/.brokkr/policy.json`
     and `<root>/agents/charters/implementer.md` becomes
     `<root>/.brokkr/agents/charters/implementer.md`, whether a hired
     agent's `charter` or a site's `role` names it. Left as written, the
     first would stand outside the moved bundle's directory and the second
     would name a file that is no longer there, so the closing compile would
     refuse after every move was made. The restated file's digest moves, and
     each restatement is a line of the refusal's listing and of the output,
     as `and rewrites .brokkr/bundle.json's policy:  <old> -> <new>`, a
     site's `role` named by its seat label. The
     compile returns these references as one closed enum, an arm per
     field (`policy`, `charter`, `role`), which `--migrate` matches with no
     wildcard (decision 0071
     ruling 2), so a path field the compile learns later cannot be read
     without its arm saying how it is restated. The enum holds only typed
     fields the compile itself returns. Nothing else a moved file holds is
     restated: not a symbolic link's text, not a charter's, role's or
     script's prose, not a field the compile does not return. A capability
     definition and a tool dialect are named by a requirement or a grant,
     not by a path, so neither needs an arm: each moves to the place the
     closing compile looks for it. A bundle's `extends` is a name too, but
     the place it is read from is never moved into, so it has no arm and
     refuses before any move (the third refusal above). Whatever a
     restatement or the move list misses, the proof refuses.
   - **No move lands on what the migration writes.** Besides the moves,
     `--migrate` writes five places under `.brokkr/` itself:
     `.brokkr/.proof` (the proof's marker below), `.brokkr/.migrating`
     (the moves' marker below), `.brokkr/.gitignore`
     (the state bullet below), `.brokkr/agents/README.md` (above) and the
     `.brokkr/state/` subtree, which only the state moves fill. The build
     holds them in one table, which the writers and this check both read
     (decision 0071 ruling 5), so a place the migration learns to write
     later is reserved by being written. A move whose destination is one of
     them, stands under one or stands above one is refused once the move
     list is known and before the proof writes its first entry, so no
     `.brokkr/` is created and every byte at the root is as it was. The
     comparison folds ASCII case, as macOS's default volume does, so the
     refusal is the same on both hosts (decision 0063). A restatement
     names a moved file's new place, so it lands on a reserved place only
     through a move, and this refuses that move. The refusal is
     exactly, one line per such move in byte order:

     ```text
     --migrate moves nothing (decision 0083): a move would land on a place --migrate writes itself
       <old path> -> <new path>: <reserved place> is written by --migrate
     ```

     Without this rule each collision would pass the proof and then break
     the migrated folder: a seat whose `role` is the root's `.gitignore`
     would have its moved role overwritten by `state/` after every move, so
     the closing compile read other bytes than the proof; a `role` of a
     root `.proof` would land the marker in the migrated folder, which
     every verb then refuses as an unfinished proof whose remedy removes
     the folder that now holds the only journal; and a `role` of
     `agents/README.md` would be overwritten by the fresh `README.md`.
   - **Prove before moving.** Before any move, and once every refusal
     above, the script refusal and the state's below has passed, so none of
     them ever leaves a tree behind, `--migrate` builds the
     proposed folder at `<root>/.brokkr/`, which no migrating shape holds
     (each is defined by its absence), as copies: exactly the files the
     move list names, each at its new place, a link copied as a link with
     its text unchanged, every byte as it stands but the rewritten map and
     the restated typed fields. The root is untouched beside it: no
     original is moved or edited, and `.forge/` stands as it is. That tree
     reaches every place the moved folder will, because it is the moved
     folder's own path, so a relative path climbing out of it, an absolute
     spelling of a restated field and the realm root `..` all resolve as
     they will after the move. It then runs the closing compile there, the
     world load included (the library, the adapters, the map, its realms'
     houses, dialects and their texts, the published crossings, the
     definitions and the tool dialects), and the build has that compile
     return what it read as typed data, as the old one did: for each
     input, the path that named it, every link it followed and the file
     it reached, with its digest. The proof holds only when the two read
     the same inputs, byte for byte: each input of the old compile and its
     world load, its path mapped through the move list (an input the list
     does not move, such as a `house`, keeps its path), is an input of the
     proof's, with
     the bytes the move list says it has (its old bytes, or the map's and
     a restated file's new ones), and the proof's compile read nothing
     else; and no input of the proof's, nor any link it followed, stands
     at a place the move empties, an old place on the move list or
     anything under `<root>/.forge/`, because that place stands now and
     will not after the move. Otherwise `--migrate` removes the tree it
     built, so every path and byte in the repository is as it was, and
     refuses with

     ```text
     --migrate moves nothing (decision 0083): its proof in <root>/.brokkr/ did not read what the old compile read
       <input>: <reason>
     ```

     one line per input in byte order, the reason one of a closed set:
     `the proof's compile refused: <error>`, when the compile or the load
     did not finish, an overlay whose base is not hired or a link outside
     the folder among the ways; `reached through <path>, which --migrate
     moves or empties`; `read with other bytes, <old digest> before and
     <new digest> in the proof`; `read by the old compile from <old path>,
     and not by the proof's`; and `read by the proof's compile, and not by
     the old one`. Only after the proof holds is the tree removed and the
     listed moves made in the repository, in one pass. (The inputs
     compared are those read by reference, the move list's kind: a file a
     seat hires, an overlay `extends`, a seat's driver resolves to, a typed
     field names or the map names. Three directory walks are not inputs.
     One is the manifest's `files` walk, which pins whatever stands under
     a bundle, as the Context says. The others are the walks
     `Library::load` and `Adapters::load` make of `agents/` and
     `adapters/` (`definition_files` in
     `crates/brokkr-runtime/src/agents/load.rs`), which parse every `.json`
     directly there. Neither puts a definition no reference reached into
     anything the compile returns: the manifest's `agents` and `drivers`
     hold only what a seat reached; such a definition could only have
     refused the old compile, which compiled; and its models resolve no
     seat's, because two files declaring one model refuse the load. So it
     stays where it is and is named as left behind, as the move list says,
     and the proof compares nothing about it. A definition a reference
     reached, an overlay's base included, is an input, and the proof rules
     on it.) The closing
     compile then runs again there and must read by reference what the
     proof read; the migration holds every journal's lock, so
     only a writer outside Brokkr can make them differ, and the output
     then says so, naming the input, with the moves made. The tree's first
     entry is `<root>/.brokkr/.proof`, written before any copy and removed
     with the tree, so a crash during the proof leaves a folder every verb
     refuses before reading it, ahead of the half-moved check, with
     `<root>/.brokkr/ is an unfinished --migrate proof (decision 0083);
     nothing at the root was moved: remove <root>/.brokkr/ and rerun
     brokkr init --migrate`, so a stray tree is never read as the folder
     and the originals are never named for removal. In the state-only
     shape the move list holds no file a compile reads, so there is
     nothing to prove, and the state moves alone.
   - **The moves are fenced as the proof is.** The proof's tree goes,
     `.proof` with it, before the first real move. Then `--migrate` writes
     `<root>/.brokkr/.migrating`, in every shape, the state-only one
     included. It lists every move it will make, one `<from> -> <to>` line
     in the order it makes them, the state moves included. It removes the
     marker only after the last move and the closing compile. While the
     marker stands, every verb refuses before it reads the folder or opens
     any store, ahead of the half-moved check, with
     `<root>/.brokkr/ is an unfinished --migrate (decision 0083); the
     moves listed in <root>/.brokkr/.migrating are partly made, and every
     file not yet moved is kept where it stands: rerun brokkr init
     --migrate to finish them`. So a crash mid-move never leaves a layout
     that no refusal matches, no store opens a fresh journal at the new
     default while the history sits at the old one, and the originals
     are never named for removal.
     The rerun reads the marker's list as its only plan, in order:
     - a move whose source stands and whose destination does not is made;
     - one whose destination stands and whose source does not is done;
     - any other shape (both stand, or neither) refuses, naming the move,
       with `<from> -> <to>: --migrate cannot tell whether this move was
       made (decision 0083); nothing was changed`, and changes nothing.
     Each entry is read by its own entry, never through a link. The rerun
     then runs the closing compile and removes the marker. A crash
     between the proof's removal and the marker's writing leaves the old
     layout whole, which its own refusal names.
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
     and `.brokkr/.gitignore` written. That is the closed list of what
     `.forge/` may hold: those entries, the journals above with their `-wal`
     and `-shm`, and `.gitignore`. Any other entry in it, or a `.forge` that
     is not a directory, refuses the migration before any move, with
     `.forge/<entry> is not state --migrate knows (decision 0083); nothing
     was moved: move or remove it first`, where a `.forge` that is not a
     directory is named as `.forge` itself in place of `.forge/<entry>`,
     because a `.forge/` left standing
     beside the new folder is the half-moved shape. Each of these is read
     by its own entry, never through a link: `.forge` itself, every
     journal with its `-wal` and `-shm`, and every entry above. A symbolic
     link among them refuses the migration before any move, in every
     shape, with `.forge/<entry> is a symbolic link, and --migrate moves no
     link as state (decision 0083); nothing was moved: put the file or
     directory it names in its place first`, where `.forge` itself is
     named as `.forge`. A rename carries a link's text, not what it names,
     so a journal reached through a link would no longer resolve after the
     move, and the store would open a fresh one there. `.forge/` is then
     empty and removed.
   - It then compiles the moved bundle and prints the old and the new
     digest. In the state-only shape no bundle moved, so there is none to
     compile, and the closing line is the refusal's `then brokkr init .`.

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
   journal's bytes did not move); `state_only_layout_refuses` (it plants a
   `.forge/forge.db` alone, with no root `bundle.json` and no library mark,
   runs `brokkr run --bundle elsewhere --feature x`, and asserts the exact
   state-only refusal and unmoved journal bytes; then `brokkr init .`
   there refuses, pointing at `--migrate`); `secrets_only_layout_refuses`
   (it plants `.forge/secrets.env` alone, holding one name, with no
   journal, no root `bundle.json` and no library mark; `brokkr run --bundle
   elsewhere --feature x`, `brokkr secrets list` and `brokkr init .` each
   refuse with the exact state-only refusal, whose one move is `mv
   .forge/secrets.env .brokkr/state/secrets.env`, and none creates
   `.brokkr/`; after `brokkr init --migrate`, `.forge/` is gone and `brokkr
   secrets list` lists the name from `.brokkr/state/secrets.env`);
   `a_library_keeps_its_journal`
   (a `recipes/x/bundle.json` beside `.forge/forge.db`, run with `--bundle
   recipes/x`, opens `.forge/forge.db` with no refusal);
   `half_moved_layout_refuses` (one case with a root `bundle.json` beside
   the folder, one with a `.forge/secrets.env` alone beside it, each with
   its exact text); `migrate_refuses_an_unknown_forge_entry` (a
   `.forge/notes.txt` beside the journal refuses with its exact text and
   nothing moves); `an_explicit_db_reaches_the_old_journal` (in
   the planted root layout, `brokkr conclude --run <id> --reason x --db
   .forge/forge.db` ends the unfinished run with no refusal, after which
   `--migrate --scripts-use-root-paths` succeeds); `migrate_refuses_an_unfinished_run` (its exact text,
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
   `migrate_restates_absolute_typed_references` (`init`'s old output with
   `bundle.json`'s `policy` set to `<root>/policy.json` and the implementer
   agent's `charter` set to `<root>/agents/charters/implementer.md`, and
   the inline `verify` seat given a `role` of
   `<root>/agents/charters/verify.md`, a planted charter no agent hires,
   which compiles before the move: the root layout's refusal lists the
   move of `agents/charters/verify.md` and all three restatements exactly;
   after `--migrate --scripts-use-root-paths` the
   moved bundle's `policy` reads `<root>/.brokkr/policy.json`, the moved
   agent's `charter` reads `<root>/.brokkr/agents/charters/implementer.md`,
   the `verify` seat's `role` reads
   `<root>/.brokkr/agents/charters/verify.md`, which stands there and is
   not named as left behind,
   every other agent's relative `charter` and every other field is kept
   byte for byte, and the closing compile succeeds and prints its digest);
   `migrate_moves_consulted_capability_definitions` (`init`'s old output
   with a root `capabilities/probe.json` that a seat's `capabilities`
   requires, and a `capabilities/unused.json` no seat requires, which
   compiles before the move: the root layout's refusal lists `git mv
   capabilities/probe.json .brokkr/capabilities/probe.json` and no line
   for `unused.json`; after `--migrate --scripts-use-root-paths`,
   `probe.json` stands under `.brokkr/capabilities/`, `unused.json` stays
   and is named as left behind, and the closing compile succeeds and pins
   `probe` with the digest the old compile pinned);
   `migrate_moves_granted_capability_files` (`init`'s old output whose
   map's realm grants `web-search` through the tool dialect
   `claude-native-search`, which no seat requires, and whose map holds a
   second realm granting `probe` through the tool dialect `probe-tool`,
   with `capabilities/web-search.json`, `capabilities/probe.json`,
   `dialects/tools/claude-native-search.json`, `dialects/tools/probe-tool.json`
   and a `dialects/tools/unused.json` no grant names, which compiles before
   the move: the root layout's refusal lists the four `git mv` lines into
   `.brokkr/capabilities/` and `.brokkr/dialects/tools/` and no line for
   `unused.json`; after `--migrate --scripts-use-root-paths` the map's
   grants are kept byte for byte, the four files stand under `.brokkr/`
   with their bytes unchanged, `unused.json` stays and is named as left
   behind, and the closing compile succeeds and pins `web-search` and
   `claude-native-search` with the digests the old compile pinned; with
   `dialects/tools/probe-tool.json` removed before the migration, the run
   prints the second root-layout refusal with the tool dialect's error and
   nothing moves);
   `migrate_refuses_a_composed_bundle` (`init`'s old output in
   `<parent>/adopter`, its `bundle.json` given `extends: "shared"`, with a
   recipe at `<parent>/shared/bundle.json`, which compiles before the move:
   `brokkr run --feature x` prints the third root-layout refusal exactly,
   `--migrate --scripts-use-root-paths` refuses with the same text, no
   `.brokkr/` is created and every planted file's path and bytes are
   unchanged; the same holds, byte for byte, with a
   `<parent>/adopter/shared/bundle.json` planted as well, the directory a
   moved bundle would compose in place of the base);
   `a_root_layout_with_a_secret_lists_its_moves` (`init`'s old output with
   a `.forge/secrets.env` holding one name beside the journal: `brokkr run
   --feature x` prints the first root-layout refusal, with the line `mv
   .forge/secrets.env .brokkr/state/secrets.env`, not the compile error;
   after `--migrate --scripts-use-root-paths`, `.forge/` is gone, `brokkr
   secrets list` lists the name from `.brokkr/state/secrets.env` and the
   closing compile succeeds; with a `secrets.env` planted at
   `<root>/config/secrets.env` as well, the run prints the second
   root-layout refusal with the compile's secrets-store error and nothing
   moves; and with the inline `verify` seat's `role` pointed at
   `.forge/tasks/role.md`, `--migrate --scripts-use-root-paths` refuses
   with the proof's text exactly, its one line naming
   `.forge/tasks/role.md` with the error the proof's compile gave, leaves
   no `.brokkr/`, and nothing moves);
   `migrate_refuses_a_realm_path_into_moved_state` (a `house` of
   `.forge/tasks/house.md`, under a directory the migration empties,
   refuses with the proof's text exactly, its one line `reached through
   <root>/.forge/tasks/house.md, which --migrate moves or empties`, and
   nothing moves);
   `migrate_rewrites_no_text` (in `init`'s old output, with the ship
   script's ledger line rewritten by the operator as
   `ledger=".brokkr/state/ledger/$run_id.md"`: after `--migrate
   --scripts-use-root-paths` every moved file but the map is byte for
   byte its old self, `scripts/ship-seat.sh` and both charters included,
   so the charters still name `.forge/tasks/`; a migrated run's ship
   close-out finds the ledger the CLI wrote under `.brokkr/state/ledger/`;
   and once a seat writes `.forge/tasks/x.md`, as the unedited intake
   charter tells it to, the next `brokkr run --feature x` refuses with the
   half-moved text exactly, naming `.forge`);
   `migrate_refuses_an_unhired_overlay_base` (`init`'s old output whose
   implementer seat hires `agents/scoped.json`, an overlay that `extends`
   `base`, with `agents/base.json` and its charter hired by no seat, which
   compiles before the move: the root layout's refusal lists no move for
   `agents/base.json`, and `--migrate --scripts-use-root-paths` refuses
   with the proof's text exactly, its line naming `agents/scoped.json`
   with the library's `extends 'base', which is not an agent in this
   library`, leaves no `.brokkr/`, and every planted path, byte and
   journal is unchanged); `migrate_refuses_a_path_dialect_reading_a_moved_charter`
   (a map whose realm gives its `dialect` as the path `dialect.json`, at
   the root, an instruction of which names `agents/charters/implementer.md`
   beside it, the hired implementer's charter, a legal instruction path
   since it is relative and stays beside its dialect, which compiles and
   loads before the move:
   `--migrate --scripts-use-root-paths` refuses with the proof's text
   exactly, its line naming that instruction `reached through
   <root>/agents/charters/implementer.md, which --migrate moves or
   empties`, and every planted path and byte is unchanged);
   `migrate_refuses_an_absolute_role_link` (the inline `verify` seat's
   `role` `roles/verify.md`, a symbolic link whose text is the absolute
   `<root>/roles/target.md`, which compiles before the move and pins both:
   the root layout's refusal lists both moves, and `--migrate
   --scripts-use-root-paths` refuses with the proof's text exactly, its
   line naming `roles/verify.md` with the error the proof's compile gives
   for a link that leaves the folder, leaves no `.brokkr/`, and the link's
   text and every other planted byte are unchanged);
   `migrate_refuses_a_move_onto_its_own_output` (`init`'s old output with a
   journal, its inline `verify` seat asking nothing so that any text is its
   role, which compiles before the move in each case: with that `role` the
   root's `.gitignore`, a planted root `.proof`, `init`'s own
   `agents/README.md`, a planted `state/role.md`, and a planted
   `agents/readme.md`, which on Linux is a file of its own, `--migrate
   --scripts-use-root-paths` refuses with the reserved-place text exactly,
   its one line `.gitignore -> .brokkr/.gitignore: .brokkr/.gitignore is
   written by --migrate`, `.proof -> .brokkr/.proof: .brokkr/.proof is
   written by --migrate`, `agents/README.md -> .brokkr/agents/README.md:
   .brokkr/agents/README.md is written by --migrate`, `state/role.md ->
   .brokkr/state/role.md: .brokkr/state/ is written by --migrate` and
   `agents/readme.md -> .brokkr/agents/readme.md: .brokkr/agents/README.md
   is written by --migrate` in turn; in each, no `.brokkr/` is created and
   every planted path and byte, the journal and `.forge/` included, is
   unchanged);
   `migrate_refuses_a_linked_state_entry` (in the state-only shape and in
   `init`'s old output, a planted `.forge/forge.db` that is a relative link
   to `../history.sqlite`, and then a `.forge` that is itself a link to a
   directory holding a journal: `--migrate`, with
   `--scripts-use-root-paths` in the root layout, refuses with the link
   text exactly, naming `.forge/forge.db` and then `.forge`; no `.brokkr/`
   is created, and every byte and every link's text is unchanged);
   `an_interrupted_migration_refuses_until_it_is_finished` (`init`'s old
   output with a journal and its `-wal`, migrated by a build whose test
   hook stops the moves after the move of `.forge/forge.db` and before
   that of its `-wal`; then the same in a root layout whose map names a
   journal outside `.forge/` and which holds no `.forge`, stopped after
   `bundle.json` moves and before `realms.json` does. In each, `brokkr
   run --feature x`, `brokkr conclude --run x --reason y` and `brokkr
   doctor`'s folder line give the unfinished-migration text exactly, no
   store is opened or created and no `.brokkr/state/forge.db` appears
   that was not moved there, and every byte is unchanged. Then `brokkr
   init --migrate` finishes the listed moves, the closing compile passes,
   and the marker is gone. A marker whose list names a move whose source
   and destination both stand refuses with that text exactly and changes
   nothing);
   `an_unfinished_proof_refuses` (the root layout with a
   `<root>/.brokkr/.proof` planted beside a partial copy: `brokkr run
   --feature x`, `brokkr doctor`'s folder line and `--migrate` each give
   the unfinished-proof text exactly, and no root file is named for
   removal); `migrate_refuses_to_move_an_unattested_script` (in `init`'s
   old output with a root `Cargo.toml`, and `scripts/verify-seat.sh`
   replaced by a gate that writes `fail` when
   `"$(dirname "$0")/../Cargo.toml"` names a file and `pass` otherwise,
   standing for a verify gate whose failing suite runs only when the
   manifest it probes is found, so that before the migration the gate
   writes `fail`: `--migrate` without the flag refuses with the script
   refusal's exact text, naming `scripts/ship-seat.sh` and
   `scripts/verify-seat.sh`, creates no `.brokkr/`, every planted file's
   path and bytes are unchanged, and the gate still writes `fail`; the
   same refusal, byte for byte, holds with the probe spelled
   `"${BASH_SOURCE%/*}/../Cargo.toml"`, with the probe spelled `Cargo.toml`
   and with `init`'s own unedited scripts, because no script is read; and
   with the probe spelled `.forge/results/../../Cargo.toml`, which names
   the root's `Cargo.toml` while `.forge/results/` stands, the gate writes `fail` before the move, the refusal without the
   flag is the same byte for byte, nothing moves and the gate still
   writes `fail`; with the flag given over that unrewritten probe the
   migration goes on, because the flag is the operator's word that the
   probe survives and nothing reads it to say otherwise, and the binding
   asserts that the moved script is byte for byte the old one, holding no
   `.brokkr/state/` text, so no rewrite turned the probe into
   `.brokkr/state/results/../../Cargo.toml`, that `.forge/` is gone and
   `Cargo.toml` stays at the root, and that the migrated gate then writes
   `pass`, the fail-open the false flag attested, pinned so the trade
   ruling 4 states cannot change unseen; with the probe rewritten to
   `Cargo.toml` and the flag given, the migration goes on, `Cargo.toml`
   stays at the root, and the migrated gate still writes `fail`); the
   root layout's refusal from `brokkr run` names the
   same two scripts on its last line, which
   `old_root_layout_refuses_with_the_migration_text` asserts with the
   rest; `old_root_layout_that_does_not_compile_refuses` (`init`'s old
   output with its `bundle.json` made not to compile: `brokkr run
   --feature x` prints the second root-layout refusal exactly, with the
   compile error the compile verb prints for that bundle, creates no
   `.brokkr/`, and the journal's bytes do not move; `--migrate` then
   refuses with the same compile error and nothing moves); and
   `migrate_moves_only_what_the_compile_read` (with the flag, a
   `scripts/build.sh` beside the seat scripts, a valid `agents/notes.json`
   no seat hires and a valid `adapters/unused.json` declaring a model no
   seat names, both parsed by the old load's directory walks, stay at the
   root and are named as left behind, the proof holds and the closing
   compile pins every agent and driver with the digests the old compile
   pinned, `agents/README.md` stays, and `.brokkr/agents/README.md` is
   written).
   Every other binding that runs `--migrate` over a move list holding a
   script passes `--scripts-use-root-paths`.

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
   but the `.gitignore` and `state/` a state-only migration leaves, naming
   it, in place of today's file-by-file refusals, and when `<dir>` holds the
   root layout or the state-only one, with that shape's refusal, which
   points at `--migrate`. It compiles the folder before it
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
- **`--scripts-use-root-paths` for a moved script.** Ruling 4 reads no
  script's text, rewrites none, and refuses to move one until the
  operator says, by one flag, that every path a moved script builds, a
  `.forge/` path and a `..` traversal included, names after the move the
  file it named before, or the place that file moves to, so every
  migration of `init`'s old output costs that flag once. An earlier
  draft rewrote the `.forge/` state paths in moved scripts and exempted
  them from the flag; a probe of `.forge/results/../../Cargo.toml`
  became `.brokkr/state/results/../../Cargo.toml`, which names
  `.brokkr/Cargo.toml`, so a gate that runs its suite only when the
  manifest is found passed after the move. Nothing is rewritten now, and
  nothing is exempt. The flag also replaces a still earlier draft,
  which refused only the files under a moved script's own directory until
  each was named by `--leave`: a script that climbs out of that directory
  from its own place, as `"$(dirname "$0")/../Cargo.toml"` does, reached a
  file the draft never refused, so a verify gate that runs its suite only
  when the manifest it probes is found would pass after the move. The
  other closed rule the evidence offers is worse: keeping each moved
  script's own-place geometry would keep the scripts out of `.brokkr/`,
  against ruling 1. The trade the flag carries is the operator's to
  take: a false flag over a script that only reads fails open, as
  ruling 4 says and `migrate_refuses_to_move_an_unattested_script`
  pins, because `.forge/results/../../Cargo.toml` names nothing after
  the move and the gate writes `pass`; a script that writes through
  `.forge/` fails closed on the half-moved refusal. The only rules that
  would refuse the read-only case either read a script's text, which
  has no closed answer, or refuse every migration that moves a script,
  which leaves the operator to move each one by hand and puts the same
  trust in a `git mv` they type. Recommended as ruled; the build takes
  another rule only on this ruling.
- **Keeping `init`'s old `.forge/` texts working.** Because `--migrate`
  rewrites no text, `init`'s old `scripts/ship-seat.sh` keeps
  `ledger=".forge/ledger/$run_id.md"` and its old implementer and intake
  charters keep `.forge/tasks/` unless the operator edits them before
  the move. Ruling 4 makes that the operator's edit, and the half-moved
  refusal catches a `.forge/` a moved file recreates. Two ways to spare
  each adopter the edit, neither in this build: (a) a closed table of
  the digests of every script and charter `init` has written, by
  release and stack, against which `--migrate` matches the digest the
  old compile pinned, reading no text, and writes the current `init`'s
  file in place of a match, as it writes `.brokkr/agents/README.md`
  afresh, leaving every other file byte for byte; (b) a compatibility
  path, `.forge` kept as a symbolic link to `.brokkr/state`. The second
  is refused here on the evidence: a kernel follows the link before it
  climbs, so `.forge/results/../../Cargo.toml` would name
  `.brokkr/Cargo.toml` again, the fail-open the rewrite had, and the
  retired name would stand in every adopter's repository. Recommended:
  (a) as a later slice, only on this ruling; until then, the edit.
- **Narrowing what `--migrate` accepts.** The proof of ruling 4 closes
  every dependency kind at once, so migration need not be narrowed to
  the layouts `init` produced. A narrower rule, moving only a scaffold
  whose every pinned digest is one `init` wrote and refusing anything
  else for the operator to move by hand, would be simpler to judge but
  would refuse the operator's own `verify-seat.sh`, the one file `init`
  asks them to fill in, so every real adopter would be refused. Not
  recommended; it is listed because the proof is the larger mechanism.
- **A composed bundle in an adopter's repository.** Ruling 1 lets no
  bundle in `.brokkr/` compose a base, and ruling 4 refuses to move an old
  root bundle that does, because a base is read from the bundle
  directory's parent, which for the folder is the repository root, and
  for a root bundle is outside the repository. Keeping a composed adopter
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
- An existing adopter is refused once, with the exact moves, and keeps every
  journaled run.
- Brokkr's own repository is unchanged by the build.
