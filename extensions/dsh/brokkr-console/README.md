# brokkr-console

Brokkr's operator console inside DSH. The guide is
[The DSH operator console](../../../docs/guides/dsh-console.md).

## The brokkr it runs

The console spawns the `brokkr` on the `PATH` DSH starts with, or the
binary `BROKKR_CONSOLE_BROKKR` names: an installed release, not a build
of this tree. So it still spells every verb as 0.15 does, `inspect`,
`transcript`, `analytics`, `graph`, `realms`, `muninn list`, `operator
retry`, `operator stop` and `conclude`, and pastes `inspect` in its hints.
A 0.15 release has no other spelling, and a 0.16 release reaches the same
verbs through the hidden aliases of decision 0113, each saying its new
spelling once on stderr. The console moves to decision 0113's tree in
0.17, when the aliases go.
