# 0085 — The DSH operator console plugin lives under `extensions/dsh/brokkr-console/`, with its own source and tests

Status: proposed
Date: 2026-10-13

Built: unbuilt (#568) — the Brokkr side of the change ships first; the plugin waits on task 0.2's measurement of DSH `0.2.0-rc.2`

## Context

The change `2026-10-13-568-operator-console-slice-one` asks for an operator
console inside a DSH client. Its design (D1–D7) splits the work in two. The
Brokkr side is Rust in this workspace: `brokkr-view` derives the admissible
commands, the unavailable actions, the clarify questions and the analytics,
and `brokkr-cli` emits them through `inspect --json` and `analytics --json`.
The DSH side is a plugin: a host part that runs those readouts and the four
existing verbs, and a client module that paints them.

Decision 0009 keeps production code Rust under `crates/` and puts anything
another program loads beside the workspace, at its extension boundary. The one
precedent is `extensions/dsh/plugin-cli-session/`, a byte-exact vendoring of a
third-party plugin with one changed expression, proven by per-file digests in
`extensions/dsh/PROVENANCE.md`. It has no source of ours, no tests and no
build.

The console plugin is different in kind. It is authored here, so it has
source and tests of its own, and nothing upstream to digest it against. The
workspace has no Node toolchain and no Node job in CI today.

## Decision

1. **Home.** The console plugin lives in `extensions/dsh/brokkr-console/`:
   its `package.json`, the host module, the client module, its tests and a
   provenance note. It is outside every hashed runtime set. No Brokkr crate
   builds, loads, reads or executes it, and no recipe, adapter or agent
   names it, so no bundle digest moves when it changes (decision 0009).
2. **Never published.** Its `package.json` carries `"private": true`. It is
   installed by the operator into a DSH profile, as the guide
   `docs/guides/dsh-console.md` will describe, and never handed to a
   registry.
3. **It paints; it decides nothing.** Every standing, need, admissibility,
   refusal text, reason and metric it shows comes from a Brokkr readout
   (design D1, decision 0013). The host never opens a journal. It fences a
   command by the run head it re-reads before invoking a verb, and dedupes by
   command id (design D4, decision 0029). The verbs gain no `--expect-seq`
   in this slice.
4. **Its tests run on the host under Node's built-in test runner,
   offline.** The plugin
   uses Node's standard library and the DSH host interface it is loaded by,
   and nothing from a registry. Its tests stand a stub in for the `brokkr`
   binary and for the DSH host, so they need no network, no DSH install and
   no journal.
5. **What CI does with them.** CI gains one job, on Linux and macOS
   (decision 0063), that installs the pinned Node major and runs Node's
   test runner in `extensions/dsh/brokkr-console/`. It is a gate beside
   the Rust jobs, not part of them: the workspace's cargo tests never run
   Node. Until that job lands, a change to the plugin carries the test
   runner's output it was proven with, and the job is named as pending.
6. **Provenance.** `extensions/dsh/PROVENANCE.md` gains a section naming the
   directory as authored here, the DSH version it targets (`0.2.0-rc.2`),
   and where task 0.2's measurement of that version is recorded. The
   measurement is recorded only once it is observed.

## Consequences

- The plugin's consumer is the operator's DSH profile, the consumer decision
  0071 ruling 6 asks for. Nothing in the workspace consumes it.
- The readouts it reads are the `--json` wire pinned by `VIEW_VERSION`, so a
  shape change on the Brokkr side moves the version the plugin checks.
- A second authored extension follows this decision rather than the vendoring
  precedent.
