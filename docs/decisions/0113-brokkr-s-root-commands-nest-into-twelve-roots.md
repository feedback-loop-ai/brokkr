# 0113 — Brokkr's root commands nest into twelve roots

Status: accepted — operator ruled on #1169, 2026-11-19: "also I believe we have too many root level commands, we should have more nesting", then on the proposal "muninn under runs, report --> stats, host --> system"
Date: 2026-11-21

Built: unbuilt (#1169) — story unit 1169a builds the tree, its aliases, the binary's own spellings and the CLI reference; the guides' spellings are 1169b's, and the aliases' removal is 0.17's

## Context

Main (5864795d1) parses 43 root commands, two of them already hidden,
and reads a 44th, `dsh-sandbox-runner`, before the parser. An operator
has to scan all of them to find one. The operator ruled the
shape on #1169. The standing ruling of 2026-11-04 ("go with recommended
on anything else that comes up") places the roots that shape does not
name.

## Ruling

1. **Twelve roots.** The six daily verbs stay at the root: `init`, `run`,
   `resume`, `watch`, `tui` and `doctor`. Six families hold the rest, by
   what each verb acts on:

   | Family | Verbs, with the root each was on main |
   |---|---|
   | `runs` | `list` (`runs`), `inspect`, `seats`, `transcript`, `compare`, `replay`, `rerun`, `conclude`, `operator`, `muninn` |
   | `journal` | `export`, `import`, `verify` (`verify-run`), `backup`, `restore`, `anchor`, `keep-refs`, `evidence` |
   | `library` | `recipes`, `agents`, `compile` |
   | `queue` | the queue's own verbs (`queue`), `dispatch`, `sweep` |
   | `stats` | `analytics`, `graph`, `costs`, `ledger` |
   | `system` | `realms`, `secrets`, `probe`, `ui` |

2. **The engine's verbs are hidden, and their spellings do not move.**
   `driver`, `hands` and `bridge` are spawned by the engine and by
   adapters. On the standing ruling, every other verb the engine or an
   adapter spawns is hidden beside them: `broker` (the MCP capability
   broker the harness starts, already hidden), `fake-driver` (the
   scripted proof driver, already hidden) and `dsh-sandbox-runner` (read
   before the parser). Their names stay at the root, as they are.

3. **A bare family keeps its old default.** `brokkr runs` with no verb
   lists runs, as `brokkr runs` did, so no spelling that parsed on main
   changes what it does.

4. **Every old flat spelling is a hidden alias for 0.16.** Each root that
   moved under a family still parses as the same command with the same
   arguments, reaching the same handler. It is hidden from `--help`. It
   prints exactly one line on stderr, before anything else:

   `brokkr: 'inspect' is now 'brokkr runs inspect'; the old spelling goes in 0.17`

   with the old and new names filled in. Nothing more is printed on
   stdout, so `--json` output is byte for byte what the new spelling
   prints. The note is printed only for a moved root. The six daily
   verbs, `runs`, `queue` and the hidden engine verbs print none.

5. **What does not move.** The frozen contracts, every `--json` document,
   every exit code, and every flag's name and meaning stay as they are.
   Only the path to a verb changes.

6. **The aliases are removed in 0.17.** That removal is a follow-up for
   0.17. Until then, every spelling the binary prints is the new one.
