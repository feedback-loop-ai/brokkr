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

## The store and the box

Decision 0065's slice two keeps the store out of a boxed seat's reach.
What follows applies to both defaults, `.brokkr/state/secrets.env` and
a library's `.forge/secrets.env`, and to a `--secrets-file`: the
ordinary store path is unsafe wherever hands could reach it.

Every workspace box hides the store at the one canonical place the
engine resolves once per run and keeps, by mounting `/dev/null` over it,
so a read inside the box reports "Permission denied". It pins every
folder between a writable mount and the store, and the artifact root's
folders, so that neither can be moved from inside, and it refuses,
before the command runs, a store it cannot hide that way (U8h,
b8e2caff). Each refusal begins "secrets store cannot be hidden from
workspace hands" and names the case: a store that is not one regular
file of one link its user owns, one that grants its group or others
access, one whose real path is not valid UTF-8, one inside the
artifact root, a symbolic link that comes to lie on the sealed path,
and a configured spelling that no longer leads to its sealed place
("the configured store '…' no longer leads to its sealed place '…'").
A Codex seat of the `workspace-write` class writes each folder
between its workspace root and a store inside that root as its own
pin in its permission profile, and on macOS a sealed `workspace-write`
Codex launch whose store lies inside its root is refused (U8i,
5d706bab).

What that still leaves exposed, as U8h records:

- a store that `brokkr secrets set` or `rm` renames over during one
  running box is visible to that one call, and so are the sibling
  `.secrets.env.*` files `secrets set` stages beside it during the
  update; every later box hides the store again;
- a symbolic link on the configured store spelling that lies in
  writable reach, off the canonical place, is not pinned: a boxed call
  can retarget it, and the next spawn then refuses;
- a later, separate run resolves the configured spelling afresh, and
  adopts a store moved between runs at its new place;
- dsh's own native shell runs outside the box, so it can read a store
  inside its workdir.

**A seat holding an MCP grant needs its store outside the worktree.**
The broker reads the dialect's declared secrets from the store, so a
store the seat's hands can reach is refused before any lookup with "MCP
secret store is reachable by workspace hands", and the default store
inside the worktree is such a store (U9b1b's F2). Keep that run's store
outside the worktree, and name it to both verbs: `brokkr secrets set
--secrets-file ~/.config/brokkr/my-app.secrets.env DOCS_TOKEN`, with
the value on stdin, writes it `0600`, and `brokkr run --secrets-file
~/.config/brokkr/my-app.secrets.env --feature …` reads it.

A dialect's secret names follow the same grammar as a seat's, and the
broker adds its own refusals on top: a name that collides with the
server box's fixed environment (`HOME`, `TMPDIR` and the rest) or with
the harness's own authentication names, and a response it cannot mask
safely, such as a digits-only value left as a bare JSON number. Neither
changes how the surfaces above mask; the [security
model](../security-model.md#mcp-capabilities-through-the-broker) says
which harnesses can hold a keyed MCP grant today (none yet: a Claude
work seat whose dialect binds a secret is refused at compile).
