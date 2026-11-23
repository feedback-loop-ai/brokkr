# 0119 — Brokkr completes its own command line

Status: accepted — operator ruled on #1230, 2026-11-21: "I want auto-complete, both static and dynamic, that will improve our quality of life tremendously!!! Schedule it after the new CLI shape"; the standing ruling of 2026-11-04 ("go with recommended on anything else that comes up") places what that ruling does not name; proposed on #1230 as 0115, renumbered to 0119 because #1262's 0115 holds it
Date: 2026-11-22

Built: built — the verb, the live completer and its read rules (#1230); the Homebrew formula's install is the tap's, out of scope

## Context

Decision 0113 nested brokkr's roots into families, so an operator now
types two words where they typed one. A shell can complete both, and
the values after them, if brokkr tells it how. clap_complete 4.6.9
(pinned exactly, with `unstable-dynamic`) writes the static scripts and
answers the shell's live requests. A live request runs on every Tab, in
whatever directory the operator stands in, beside journals that lanes
are writing. So what it may read has to be ruled.

## Ruling

1. **The verb.** `brokkr completions <bash|zsh|fish|powershell|elvish>`
   prints, on stdout, the script `clap_complete::generate` writes for
   the command tree. It is a root verb beside `doctor`, since an
   operator installs completion before learning the families. It is
   the tree's thirteenth root, and every other root stays as 0113 ruled
   it. The script offers every visible verb, flag and fixed value set.
   It offers no hidden verb (`driver`, `hands`, `bridge`, `broker`,
   `fake-driver`) and none of 0.16's flat aliases. The aliases are
   respellings made before the tree parses, so they are not in the tree
   at all. clap_complete's fish generator writes the flags of verbs one
   and two words deep only, so fish's static script offers no flag of a
   three-word verb such as `library agents show`. That is the
   generator's limit, recorded here and in the quickstart, not worked
   around.

2. **Dynamic completion is clap_complete's protocol.** `COMPLETE=bash
   brokkr` prints the line a shell registers, and each shell has its
   own equivalent. `main()` hands the command line to
   `clap_complete::env::CompleteEnv` before anything else: before the
   dsh sandbox runner, before the alias respelling and its stderr
   note, and before any folder or journal is found. A request prints
   only candidates and exits. Without `COMPLETE`, startup reads that
   one variable and nothing more.

3. **Read-only, and nothing left behind.** The journal is read
   through `Store::read_untouched`. It is read-only, as
   `Store::open_read_only` is, and it also creates no file. A read-only
   SQLite open of a WAL journal still creates its `-wal` and `-shm`.
   The journal is first resolved as SQLite resolves it, every link
   followed, since SQLite names the side files after the file it opens.
   Then:
   - where a `-journal` stands, a rollback-mode writer holds the
     journal or left it hot, and nothing is read (ruling 7);
   - where a `-wal` stands, it is opened `readonly_shm`;
   - where neither stands and the header names a WAL journal, it is
     opened `immutable`. Nothing guards that read against a writer
     starting during it, so once the read is done the side files and
     the main file's size, time and inode are looked at again, and a
     read across any change answers nothing;
   - any other journal, a rollback-mode one among them, is opened
     read-only under SQLite's own locks, which create no file in that
     mode, so a peer's lock refuses it within ruling 8's patience.

   No journal, `.forge/` or `.brokkr/` directory, lock or side file is
   created. Nothing is migrated or backfilled, and no writer lease is
   taken.

4. **One bounded page.** Run ids, for `--run`, `--by-run` and `runs
   compare`'s two runs, are the 50 newest. They are in `latest`'s
   order, read from the `runs (created_at)` index, each with its
   summary's feature line as its description. Queue entries are the 50
   newest waiting entries, queued or held. Phases and seats, for
   `--phase` and `--seat`, are offered only when `--run` is already on
   the line. They are read from the named run's seat rows
   (`Store::run_seats`), the first 500 in the order they opened, one
   range of the `seat_summary (run_id, opened)` key: the phases its
   seats sat in, and each participant by the label `runs inspect`
   scopes by. A `--run` prefix is resolved from at most two of its
   matches (`Store::run_candidates_within`), enough to tell one run
   from several. No event is read and no chain is verified, so the cost
   does not grow with the run. A row a lagging fold has not yet
   written is not offered, and a phase no seat sat in is not offered.
   These are candidates, not a readout. No candidate read loads a
   run's events or the whole journal.

5. **Only the journal, the library and the map.** Recipe names come
   from the shelves `--recipe` resolves in, the project's and then the
   personal one (#697), each name once, agent names from the
   agent library, and realm names from the realms map. No candidate
   comes from the network, a provider or an environment secret.

6. **The command's own journal.** The completer reads the words the
   shell hands it, after clap_complete's `--`, the whole line and not
   only the words before the cursor. `--db`, `--realms`,
   `--recipes-dir`, `--agents-dir` and `--dir` there, spelled `--flag
   value` or `--flag=value`, win. Without them, the journal and map are
   resolved as the completed command resolves them, from the
   discovered folder (decision 0083), without the ambient map's notice.

7. **Silent on every failure.** A missing, locked, damaged or
   foreign-schema journal, an unreadable library or map, or a panic
   gives no candidate. Nothing is printed on stderr, because that is
   the terminal the operator is typing in.

8. **Fast.** A peer's lock is waited on for at most 50 ms, never the
   writer's 30 seconds.

## Out of scope

- The Homebrew formula's completion install. The published formula
  lives in the tap repository. Its template,
  `packaging/homebrew/brokkr.rb`, is in this repository and the release
  workflow renders it into the tap. Adding the install to the template
  and the tap is a follow-up for packaging.
- The nix flake's completion install. The flake fetches the release
  tarball and builds nothing, and the tarball carries no scripts.
  Generating them would mean running the binary inside the derivation
  after `autoPatchelfHook` has patched it, which is a change to the
  packaging, not to the command line. It is left out.
- Removing 0.16's aliases, which is 0.17's (decision 0113 ruling 6).
  Until then, completion already offers only the new spellings.
