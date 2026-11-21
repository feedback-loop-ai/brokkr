# 0115 — Brokkr completes its own command line

Status: accepted — operator ruled on #1230, 2026-11-21: "I want auto-complete, both static and dynamic, that will improve our quality of life tremendously!!! Schedule it after the new CLI shape"; the standing ruling of 2026-11-04 ("go with recommended on anything else that comes up") places what that ruling does not name
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
   at all.

2. **Dynamic completion is clap_complete's protocol.** `COMPLETE=bash
   brokkr` prints the line a shell registers, and each shell has its
   own equivalent. `main()` hands the command line to
   `clap_complete::env::CompleteEnv` before anything else: before the
   dsh sandbox runner, before the alias respelling and its stderr
   note, and before any folder or journal is found. A request prints
   only candidates and exits. Without `COMPLETE`, startup reads that
   one variable and nothing more.

3. **Read-only, and nothing left behind.** The journal is opened by
   `Store::open_read_only_untouched`. It is read-only, as
   `Store::open_read_only` is, and it also creates no file. A read-only
   SQLite open of a WAL journal still creates its `-wal` and `-shm`.
   So where no `-wal` stands beside the journal, it is opened
   `immutable`, and where one stands it is opened `readonly_shm`. No
   journal, `.forge/` or `.brokkr/` directory, lock or side file is
   created. Nothing is migrated or backfilled, and no writer lease is
   taken.

4. **One bounded page.** Run ids, for `--run`, `--by-run` and `runs
   compare`'s two runs, are the 50 newest. They are in `latest`'s
   order, read from the `runs (created_at)` index, each with its
   summary's feature line as its description. Queue entries are the 50
   newest waiting entries, queued or held. Phases and seats, for
   `--phase` and `--seat`, are offered only when `--run` is already on
   the line. They are the named run's visited phases and participants,
   from the frame `brokkr runs inspect` paints. No candidate read loads
   a run's whole journal.

5. **Only the journal, the library and the map.** Recipe names come
   from the recipe library `--recipe` resolves in, agent names from the
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

- The Homebrew formula's completion install. The formula lives in the
  tap repository, not here.
- Removing 0.16's aliases, which is 0.17's (decision 0113 ruling 6).
  Until then, completion already offers only the new spellings.
