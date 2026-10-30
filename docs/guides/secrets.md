# Secrets

Decision 0012: bundles and journals carry secret **names** only. An
exec template writes `{{secret:NAME}}`; the runner — and nothing
upstream of it — resolves that to the environment reference `$NAME`, and
hands the value, read from an operator-side store that lives outside
version control, to the child through its environment alone.

`secrets set` reads the value from STDIN, never argv, and creates the
store 0600:

```
$ brokkr secrets set GITHUB_TOKEN
set GITHUB_TOKEN in /home/you/my-app/.brokkr/state/secrets.env
```

`secrets list` prints the bound names, one per line: there is no
value-printing verb.

```
$ brokkr secrets list
GITHUB_TOKEN
```

`secrets remove` takes the name off the store and says where it
removed it from:

```
$ brokkr secrets remove GITHUB_TOKEN
removed GITHUB_TOKEN from /home/you/my-app/.brokkr/state/secrets.env
```

The store defaults to `.brokkr/state/secrets.env` in the repository's
folder, found from anywhere in it, which `.brokkr/.gitignore` keeps out
of git (decision
[0083](../decisions/0083-an-adopters-brokkr-lives-in-one-folder.md)). A
recipe library, which has no folder, keeps it at `.forge/secrets.env`
in the workspace. A repository with neither has no place for it: the
secrets verbs refuse there, create nothing, and name `brokkr init .`.
`--secrets-file` on the secrets verbs and on `brokkr run` points
elsewhere. A seat declares which names it binds,
and compilation fails on an undeclared one. A declared name reaches the
seat's process — a claude, codex or dsh harness as much as an exec
command — through its environment, never its argv. A model seat may
declare a binding only on a route whose egress class meets the bundle's
minimum (decisions
[0021](../decisions/0021-model-policy.md) ruling 4 and
[0036](../decisions/0036-egress-is-a-property-of-the-route.md) ruling 4),
because a model reads its own environment: binding a secret to a model
seat hands it to that model and to the route's provider, so bind there
only what the route is cleared to receive.

Two refusals bound that. A seat with hands cannot declare a binding at
all: its hands start from a cleared environment, in the box under
`namespace` and rebuilt by the engine for an exec seat under `harness`,
and compilation refuses it
(decision [0043](../decisions/0043-the-hands-are-one-tool.md)). At the
default minimum, `contracted`, the shipped claude and exec adapters may
bind; the codex and LaneTally adapters may not, and neither may any dsh
route but the `local` `spark` and `spark-glm`. Separately, a model seat
inherits the engine's own environment, so a value the launching shell
exports reaches the harness undeclared and is not masked. The
[security model](../security-model.md) states the whole flow.

Whatever the child writes back is masked to `[secret:NAME]` before it
reaches the journal: its stderr on raw bytes before the string ever
exists; a harness's stream and transcript on each decoded event, before
any fold clamps a field to a length or rewrites a refusal into one line;
and the result file after it is parsed, so a value JSON escaped on the
way out is still found. A wholly numeric secret that a structured field
carries as a bare JSON number is not rewritten there — a count stays a
number — so do not bind a secret that is only digits. The `{{secret:NAME}}` spelling
itself is not secret-bearing, which is why it is journalable and the
resolved command line is not.

A harness's own session file is not one of those surfaces. A claude,
codex or dsh harness writes its own transcript, and a value the model
echoed is plaintext there; Brokkr does not rewrite that file, because it
is the harness's resume record. The surfaces that read it instead —
`brokkr transcript`, the TUI's transcript pane and the browser's session
view — mask each block to `[secret:NAME]` before printing it, against
every value held by the store beside the journal they read
(`.brokkr/state/secrets.env` next to `.brokkr/state/forge.db`, or a
library's `.forge/secrets.env` next to its `.forge/forge.db`), and add a notice
naming what was masked. A tool call's arguments or a shell action reach
the page as serialised JSON, so each value is also masked as it reads
inside a JSON string, with its quotes, backslashes and control
characters escaped. That covers only the store's current values: a
value rotated or removed since the run is no longer there and is shown
as written, which the notice says. A store that cannot be read masks
nothing, and the notice says why. A store that is absent or holds no
values masks nothing either, and the notice says so and names the path
it looked at. The surfaces look only beside the journal, so a run whose
store lives elsewhere — `--secrets-file`, or a `--db` or `--repo` that
puts the journal in a different directory from the default store — is
masked only against whatever the store beside
its journal holds.
