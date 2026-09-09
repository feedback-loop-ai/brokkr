# Provider adapters

A provider adapter is **data**, one file per provider in `adapters/`:
the driver invocation, the abstract→concrete model mapping, how tool
permissions and MCP servers are expressed, and — the load-bearing part —
which of those the provider **cannot** express, written as the explicit
string `"unsupported"` rather than left to be inferred from an empty
map. Adding a provider or a model is a file edit, not a release.

`brokkr doctor` reports what each adapter can actually reach on this
machine, and refuses to guess about the rest:

```
$ brokkr doctor
ok       claude: 2.1.251 (Claude Code) · serves fable, haiku, opus, sonnet
ok       codex: codex-cli 0.153.2 · serves astra, luna, sol, terra
ok       dsh: 0.1.2-rc.1 · serves flash, flash-experiment, glm, muse, muse-contributor, pro, qwen-flash, qwen-max, qwen-plus, qwen36-flash, qwen37-max, spark-flash, studio-flash, studio-pro
warn     lanetally: binary 'claude-lanetally' not found — seats resolving to this provider will fail to spawn …
ok       boundaries: namespace (bubblewrap 0.11.0) · harness · open offered; seatbelt built by slice (ii) of decision 0046 ruling 6 (sandbox-exec not on PATH); container built by slice (iii) (docker found)
```

The `boundaries` line (decision
[0046](../decisions/0046-the-boundary-is-named.md) ruling 2) names what
this machine can put between a boxed seat's hands and itself. A boundary
is **offered** when `run`, `resume` and `rerun` would start a bundle under
it here: `namespace` needs bubblewrap on `PATH` (0.10 or newer for an
overlay bind), and `harness` and `open` are offered everywhere because
they need nothing of Brokkr's. A boundary is **ready** only once the tree
has built it, and the line says which slice does: `seatbelt` (slice ii)
and `container` (slice iii) are named by the decision and refuse at
start today whether or not `sandbox-exec`, `docker` or `podman` is
found — the tool's presence is reported so the operator knows what the
slice will find, not as an offer. The realm's word is judged against
this line before any journal row is written, naming the seats that
declare hands and the tool or slice they wait on. `brokkr doctor
--bundle <dir>` compiles in the discovered realm, so its `hands` line
judges the bundle's sites against the realm's boundary, not against a
default.

The built-in adapters are reachable directly as
`brokkr driver <claude|lanetally|codex|dsh|exec> -- <extra args>`, which
is exactly how a bundle names them.

The `dsh` adapter reaches two provider routes through one grammar. A
bare id (`deepseek-v4-flash`, `deepseek-v4-pro`) is DeepSeek's own API,
keyed by `DEEPSEEK_API_KEY` in the engine's launching environment; a
`dashscope/<id>` lane is Model Studio's Token Plan catalogue
(`deepseek-v4-flash-0731`, `qwen3.8-max`, `glm-5.2`, …), keyed by
`DASHSCOPE_API_KEY`, and needs that provider route declared in the dsh
headless profile's own patch layer (`$DSH_HOME/profiles/headless/cordis.patch.yml`).
The driver turns `--model <lane>` into the one-seat overlay dsh's
launcher reads; neither key ever enters argv, the recipe, or the
journal. A custom route's model entry must also state its own
`reasoningEfforts`, because dsh resolves an entry without them against
its installed catalog for the same provider key and ships none for
`dashscope`: left unstated, the model materialises as non-reasoning and
any pinned `--effort` is refused at start. `recipes/research-dsh`
states `low`, `medium` and `xhigh` for `qwen3.8-max`, the levels dsh's
own Qwen catalog lists, and pins `xhigh` (decision 0035, second
addendum).

The `flash-experiment` alias pins `deepseek-v4.1-flash-expires-on-0910`
on DeepSeek's own API. A completion verified this beta id on 2026-09-08;
it was not yet included in the API's model list. Its name indicates a
September 10 expiry; the exact cutoff time is unconfirmed.

The same adapter reaches Muse Spark 1.3 through OpenRouter
(`https://openrouter.ai/api/v1`, OpenAI-compatible, Meta as the sole
upstream at Meta's own prices) as two routes that share one endpoint
and one key name, `OPENROUTER_API_KEY`: `meta/meta/muse-spark-1.3` and
`meta-contributor/meta/muse-spark-1.3-contributor`. They are two
routes because the model id is the terms — the bare id is not used to
improve Meta's products, the contributor id is, at roughly a twelfth
of the price — and egress is a property of the route (decision 0036),
so the two must be tellable apart by prefix. The route is the first
segment only; the second `meta/` is OpenRouter's name for the model.
Neither is ruled `contracted`; both are `uncontracted` by silence.
Muse Spark always reasons, so the adapter's `efforts` carries `xhigh`
for it and the profile row declares no `off` level. A seat's `--effort`
reaches the wire: the driver writes it into a settings document of the
seat's own, which dsh reads over its composition and over the route's
`reasoning:` default (decision 0035 addendum), and the request header
echoes the level back into the record. The reasoning itself comes back
encrypted, so the record carries its signature and no text.

A key taken from the launching environment is the one channel that
moves no digest and reaches no journal row. It is not forbidden, but an
adapter may name it — `"credentials": {"<route>": "<VARIABLE>"}`, a
name only — and then `brokkr doctor` warns, by route, whenever that
variable is satisfied from the process environment rather than bound by
a seat (decision
[0036](../decisions/0036-egress-is-a-property-of-the-route.md) ruling
5). `brokkr doctor --bundle <dir>` asks whether any seat of that bundle
declares the name in its `secrets`, because a name sitting in the
bindings store that no seat binds is handed to no driver and the
launching shell's copy is what the provider reads. Store membership is
still necessary — a name a seat declares and the store cannot answer for
is bound to nothing either, and doctor says which of the two halves is
the one missing. Without a bundle to inspect it falls back to store
membership alone and says so in the line, naming whether you passed no
bundle or one that would not compile (decision
[0040](../decisions/0040-the-flag-is-always-read.md) ruling 4).

The route names in `routes` and `credentials` are whatever a concrete
model id may begin with — ASCII letters of either case, digits, `-`,
`_`, `.` and `:`, never `/` — so `us.east` and `openai_compat` are
routes an operator can rule on (decision 0040 ruling 5). And the route a
seat's argv names is read on the flag the adapter declares in
`model_flag` AND on `--model`: the same string is one read, a concrete
pin on either names the route, and two pins naming different ids are
refused naming both flags (ruling 1). A short declared flag (`-m`)
carries its value attached in the getopt way, so `-mspark/x` is a pin
whose value is `spark/x`; a long one has only `--model x` and
`--model=x`, and `--model-fallback` is a different flag every reader
walks past (ruling 2).

Looper-bound runs start with `brokkr run --dispatch <forge-dispatch-v2.json>`.
The immutable dispatch is sealed into the v2 run manifest and therefore travels
with `brokkr export`. `brokkr bridge --run <id> --looper-url <url>` tails only the
verified public store API and synchronizes ordered evidence plus fenced commands;
it reads its bearer credential from `LOOPER_API_KEY` (or `--token-env`), never
from a command-line value or the journal.

## Hands

`hands` (decision 0043) is the adapter's answer to a site that boxes its
hands: the argv fragment that disables the harness's own tools and reaches
`brokkr hands serve` over MCP. Two tokens are expanded by the engine at
spawn — `{hands_mcp_json}`, a Claude-style MCP config naming this binary,
and `{hands_args_toml}`, the server's arguments as a TOML array for
`codex -c`. The codex fragment also sets
`mcp_servers.brokkr.default_tools_approval_mode="approve"`: since
codex-cli 0.153 an MCP tool call "requires approval" by default and a
non-interactive `codex exec` runs under an approval policy of `never`, so
without that key every workspace call — reads and the result write alike
— is refused with "MCP tool call requires approval, but approval policy
is never". The first two astra-judged gates died on exactly that
(2026-09-05), and `auto` does not lift it; `approve` does, measured
against `brokkr hands serve`. `{"unsupported": "<measured reason>"}` declares that the
harness cannot swap its tool surface, and a site with hands then refuses
to compile against it, exactly as an unexpressible tool list does.

The `workspace` fragment is what a site with hands runs under the
`namespace` boundary, where Brokkr builds the box. Which boundary stands
is the realm's fact, not the bundle's (decision
[0046](../decisions/0046-the-boundary-is-named.md) ruling 1): a
`"boundary"` word beside `house` and `dialect` in `realms.json` under
`forge.realms/v4`, absent reading `namespace`. Under `harness` Brokkr
builds nothing and the harness's own sandbox is the only wall, so the
adapter's second answer is `hands.harness`: how that sandbox is addressed
from argv.

### `hands.harness`

```json
"hands": {
  "workspace": ["--sandbox", "read-only", "-c", "mcp_servers.brokkr.command=\"{brokkr}\"", …],
  "harness": {
    "gate": ["--sandbox", "read-only", "--output-last-message", "{result_path}"],
    "work": ["--sandbox", "workspace-write"],
    "result": "last-message"
  }
}
```

Three members, nothing else admitted:

| Member | Meaning |
|---|---|
| `gate` | The argv fragment that puts a gate-class seat in the harness's read-only class. A model gate is admitted under `harness` only when **every** link of its resolved chain declares one (decision 0046 ruling 4); under `open` a model gate is refused outright. |
| `work` | The fragment for a work-class seat with hands — the class that writes. A work seat under `harness` needs it on every link too; under `open` the same seat runs at the harness's own default, and whether that default writes is the harness's fact. |
| `result` | How the gate's verdict reaches the engine: `file` (default — the seat writes the result file itself) or `last-message` (the harness's own capture writes its final message to the result path). |

`gate` and `work` follow the three-shape convention `tool_permissions`
already uses: an argv fragment declares the member, `{"unsupported":
"<measured reason>"}` declares that the harness has no such class and
says why, and an absent member reads unsupported with no reason —
fail-closed, so a provider that has not been measured admits nothing by
silence. A fragment may carry two tokens the engine expands at spawn,
`{result_path}` and `{brokkr}`. The two workspace tokens,
`{hands_mcp_json}` and `{hands_args_toml}`, are refused in a harness
fragment by name: no workspace tool is served under `harness`, so a
fragment naming one would run with the literal token in its argv. A
`hands` object as a whole may still be `{"unsupported": …}`, and then
neither boundary compiles a site with hands against it.

**codex** declares both members and the `last-message` door, above.
`--sandbox read-only` is ruling 4's own word for codex and
`--sandbox workspace-write` is the tool's documented writable class;
`--output-last-message` is the flag `codex exec --help` documents and
the driver already admits on a resume. One fact is declared from the
tool's record rather than measured: whether the capture lands while the
sandbox class is `read-only`. That measurement is **pending and the
operator's** — one gate seat under the fragment, delivering its result
and nothing else; if the capture does not land, `gate` becomes
`{"unsupported": "<that reason>"}` and nothing false enters the record
either way, because a failed door is a missing result, loud.

**claude** declares **no** `hands.harness` member yet. The measurement
is the operator's, against the installed 2.1.x line (the transcript
above records 2.1.251), because the implementing seat's tool grant is
`cargo` and `git` and `claude` is not a command it may run. Until it is
recorded every shipped bundle whose hands agent's chain reaches claude
— every hands agent chains `opus` — refuses under `harness` naming
`claude`, the member and the site; the record of which bundles those
are is the pin test in
`crates/brokkr-runtime/src/bundle/model_policy_tests.rs`. What is known
going in: claude's `--permission-mode` choices are `acceptEdits`,
`auto`, `bypassPermissions`, `manual`, `dontAsk` and `plan` — there is
no `read-only` value. Candidates for `gate`: `--permission-mode
dontAsk` with `--allowedTools` naming the read tools and one edit rule
scoped to `{result_path}` (`result` `file`); `--permission-mode plan`
if it can still write the result file; and the `--restricted` /
`--permission-prompts none` pair reported on 2.1.263, unconfirmed.
Candidates for `work`: `--permission-mode acceptEdits` with the shell
allowed, or the harness's own sandbox settings with the shell
auto-allowed when sandboxed — a bare `acceptEdits` prompts for every
shell call, and a non-interactive seat answers a prompt with a denial,
which is why the empty fragment is an answer only if the measurement
shows the driver's own mode grants the shell. The recipe: under each
candidate, run one gate seat whose prompt asks it to read a file
outside the worktree, write one inside it and deliver its result — the
mode passes as `gate` when only the result file lands; run one work
seat that must run `cargo test` and commit — the mode passes as `work`
when both succeed with no prompt. When a member is measured, this
paragraph records the claude version it was measured against and what
the mode denies and allows; a member no combination satisfies is
declared `{"unsupported": "<measured reason>"}`, and the bundles that
seat claude keep refusing under `harness` by name.

**dsh** and **lanetally** declare no `hands.harness`, and a `harness`
gate reaching either refuses naming the link, the provider and
`hands.harness.gate` — exactly as a boxed gate on them refuses today.
Of claude, codex and dsh, then, a `harness` gate stands on codex today
and on claude once measured. A dsh work seat with hands may run under
`open` at its harness's default; its gate refuses under `namespace`
for the untrusted tier and under `harness` for the missing gate fragment.

A dsh **work** seat under `harness` is confined by dsh's own sandbox,
which writes only under the session workspace. A linked `git worktree`
keeps its metadata under the shared repository's `.git`, outside that
workspace, so `git add`/`git commit` would fail on `index.lock` (the
defect the essay records). `brokkr driver dsh` resolves the worktree's
`--git-dir` and `--git-common-dir` through Git before the seat starts
and, when the seat's mode is `workspace-write` and the common dir lies
outside the workspace, points dsh's supported sandbox `runnerCommand` at
`brokkr dsh-sandbox-runner` with those paths as trusted argv.

**The shared repository is never writable.** The runner adds exactly two
read-write mounts: the worktree's own administrative directory
(`index`, `HEAD`, its reflog) and a PRIVATE common directory the driver
staged. The seat's git follows that private directory because the one
file git reads to find the shared one — `<git_dir>/commondir` — is masked
with a read-only bind naming it. The private directory is a real common
directory: a copy of the shared `refs` and `packed-refs`, so every
branch, tag and remote-tracking ref reads back exactly as the host has
it, an empty `logs`, and an `objects` whose `info/alternates` names the
host's real object store — so the seat reads every object and writes
none. `<common>/objects`, `<common>/refs`, `<common>/packed-refs`,
`<common>/logs`, `<common>/hooks`, `<common>/config`,
`objects/info/alternates` and every sibling worktree's administrative
directory are read-only under the profile's own `--ro-bind / /`.

**The driver promotes exactly one ref.** After `dsh` exits, outside every
box, it takes the branch the worktree had checked out when the seat
started — git refuses to check one branch out in two worktrees, so that
ref is this seat's and no sibling's, and the driver verifies it by
reading the main checkout's `HEAD` and every sibling's — fetches the
seat's commits into
`refs/brokkr/dsh-promotion-<seat>` through git's own local transport
(a name per seat, so two seats in two worktrees of one parent do not
delete each other's anchor), moves the
branch with a compare-and-swap, and deletes the temporary ref. A
sibling's branch, a tag, a remote-tracking ref or a new branch the seat
wrote stays in the private store and is discarded with it. A worktree
with a detached HEAD owns no ref and is refused before the seat starts,
and so is a repository whose refs live in a backend the private store
cannot reproduce (`extensions.refstorage = reftable`), because copying
`refs` and `packed-refs` would give the seat a store with no branch at
all. The fetch runs with `fetch.fsckObjects` on, given on the command
line: it is the one write path into the shared object store, so every
object in the received pack is validated before it lands.

The same fetch runs with the shared repository's automatic maintenance
turned off — `maintenance.auto=false`, `gc.auto=0`, `gc.autoPackLimit=0`,
all on the command line. A promotion ADDS objects and moves one ref;
`git fetch` otherwise ends by running the receiving repository's own
maintenance, which packs every reachable loose object and unlinks the
loose copy, and does it detached, under a configuration the run did not
choose. Nothing is lost when that happens — git reads those objects out
of the new pack — but the operator's object store is not the driver's to
rewrite, and a repack in the middle of a promotion is indistinguishable,
from the outside, from a seat that reached the shared store. Given as
config rather than as `--no-auto-maintenance`, because a git that does not
know that flag would refuse the whole fetch, while an unknown config key
is ignored.

**The store the box held is reclaimed before the driver reads it.** The
seat owns its private common directory for its whole life, and the
promotion then hands that directory to git as a repository — so the
driver first keeps only what the store holds as a value (`objects`,
`refs`, `packed-refs`) and writes `HEAD`, `config` and
`objects/info/alternates` afresh from the shared repository. Everything
else goes, named or not: `commondir`, `config.worktree`, `shallow`,
`info/grafts`, `hooks`, `logs`. `commondir` is why. Git reads a git
directory's common directory from `<dir>/commondir` however that
directory was named, `--git-dir` included, and takes the repository
config, the ref store and the object store from wherever it lands — so
one file a seat creates inside its own store would otherwise steer the
promotion's `rev-parse` and its `upload-pack` at a repository the seat
built. A symlink at a kept name is removed rather than followed. The
reclaim also takes back the ACCESS the box could drop: a seat owns the
store read-write for its whole life and can `chmod` a directory it
created there, and an unlink writes the directory holding the name, so
without that step one locked directory would end the sweep wherever it
happened to be. Every directory in the store is widened to `u+rwx`
first, through `symlink_metadata` so no mode outside the store is
touched.

The same reclaim runs on the kept-store path, so the directory a failed
promotion names is one the driver authored — and that refusal offers
`git --git-dir=<store> log <ref>` only when the reclaim SUCCEEDED. If it
could not finish, the message still names the path, because the commits
are in it and nowhere else, but points no git at it: a surviving
`commondir` would redirect `--git-dir` at a repository the seat built,
whose configuration names the commands git then runs. It says what the
directory is and hands the reclaim over in words — delete every name
except `objects`, `refs` and `packed-refs`; remove `objects/info` too and
recreate it with only `alternates`, then write the shared
`objects` path into `objects/info/alternates` and `ref: <branch>` into
`HEAD` — and it is a repository again.

The compare-and-swap is against the BASELINE the driver recorded for that
branch before the seat started, not against a value read after the seat
finished: the branch is the seat's for the seat's whole life, so a host
that moved it meanwhile — an operator, a fetch, a push — makes the
promotion refuse rather than overwrite that work. Every failure that
reaches the driver while the private store holds the only copy of the
seat's commits keeps the store and names its path and the branch to read
it at: the promotion's own failures, and anything that goes wrong between
the seat's last command and the promotion. Nothing sweeps a kept store
afterwards — it holds the only copy of those commits — so deleting it is
the operator's once the work is safe, and the refusal says so. A fetch
followed by a failed branch update or anchor deletion can also leave
`refs/brokkr/dsh-promotion-<seat>` in the shared repository. After recovering
its commits, remove that particular anchor with `git update-ref -d <ref>`;
`git branch` and `git tag` do not list these anchors.

A BARE parent repository is served: its `HEAD` is the branch a clone
would follow rather than a checkout, `git worktree add` serves it, and
the driver asks git whether the repository is bare instead of reading
`HEAD` and calling the layout a conflict.

`<git_dir>/HEAD` is mounted READ-ONLY, because it is what says which
branch this worktree owns and therefore which ref the promotion moves:
left writable, a seat could aim the next seat's honest commits at a
branch it chose. A commit does not write `HEAD` — it moves the branch
`HEAD` names — so committing, amending and resetting the own branch all
work, including non-fast-forward updates to that branch; `git checkout <other>`, `git switch` and `git rebase` do not, and
fail on the lock.

Three consequences worth knowing at the bench. A seat's commits reach the
shared repository when the seat ENDS, not when it commits, so a host `git
status` in that worktree mid-seat shows the seat's tree as staged. A seat
cannot switch branches. And `git worktree list` inside the seat reads the
private store, which has no `worktrees` directory, so it reports only
that store. Content added to the index but not committed before the seat
ends is not preserved in the shared object store: the index can then refer
to discarded private objects. Commit staged content within the same seat.
To recover such an index, preserve the working files, reset the index to
`HEAD` with `git reset --mixed HEAD`, and stage those files again; partial
staging selections may need to be recreated. Private stashes are discarded
too, because promotion transfers only the checked-out branch.

Both hook paths git could use are empty tmpfs mounts, and three config
paths are masked with a staged empty read-only file: the per-worktree
`config` and `config.worktree`, and the private store's own
`config.worktree` — that last one because `<store>/config` is a copy of
the shared config and therefore carries `extensions.worktreeConfig` for a
repository that has run `git sparse-checkout`, and because the trusted
driver runs `git --git-dir=<store>` over exactly that config after the
seat exits. The mask source is a real empty file the driver stages, never
`/dev/null`: bubblewrap binds a source with `MS_NODEV`, and a device node
the box cannot open makes git call every command fatal. A mask is only a
mask while the box cannot WRITE its source, and the runner measures that
from the profile it was handed rather than assuming it: if the staged
directory lies under any read-write bind in that profile (`--bind`,
`--bind-try`, `--dev-bind`, `--dev-bind-try`, or either host path of an
`--overlay` — its upper layer and its working directory), or under the
runner's own write set, the command refuses. Stepping over the rest of
the profile needs each option's arity, so the runner carries a complete
bubblewrap 0.11 table and refuses an option outside it rather than
guessing — including `--args`, which hides further options in a descriptor,
and `--bind-fd`, whose arity it knows and whose source
is a file descriptor it cannot turn back into a host path to measure.

**One layout is served, and it is proved.** Git resolves both directories
by following the workspace's own `.git` file — a file the model can write
— so resolving them early is not enough on its own. A scope is served
only when the git directory is not the shared repository, sits where git
itself would have put it (`<common>/worktrees/<name>`, symlinks
resolved), and carries a `gitdir` back-pointer whose worktree ROOT is this
seat's own workspace directory. Everything else refuses at seat start and
again in the runner, each naming its own cause: a git directory that IS
the shared repository (a primary checkout reached through a subdirectory,
a `--separate-git-dir` checkout, a `.git` file redirected at the parent),
a workspace-local git directory naming an unrelated repository as its
common directory, a back-pointer naming a different worktree or missing
entirely, and a seat rooted in a subdirectory of its own worktree.

The back-pointer is compared as a DIRECTORY against the workspace, never
as a path against `<workspace>/.git`, and a symlink at `<workspace>/.git`
is itself a refusal. `<workspace>/.git` is a path the seat owns: point it
at another worktree's `.git` — by symlink or by copying the file — and
Git hands back that worktree's REAL administrative directory, so the
topology check holds and a comparison that resolves symlinks finds both
ends of the pair agreeing about a repository the seat does not own. The
worktree root the back-pointer names is the end no workspace write can
move, and both spellings of that alias are refused under a real `git
rev-parse` in the behavioral proof.

The whole shared `.git`, the parent checkout, every sibling worktree
DIRECTORY, every sibling's per-worktree metadata (`HEAD`, `index`,
`config.worktree`) and every sibling's BRANCH stay unwritable — the
branch at every spelling git uses: a loose `refs/heads/<name>`, a line in
`packed-refs`, the `.lock` a ref update takes first, and a `git pack-refs
--all` that would rewrite the file. Each of those is tried inside the box
in the behavioral proof, and the host's bytes are read back unchanged
after the box AND after the promotion. The checks gate what the runner
MOUNTS for a seat: a seat still owns its own worktree, `.git` file
included, so a host `git` run in a worktree a model has written follows
whatever that file names — true of every work boundary, named in 0054's
consequences.

Brokkr's own `namespace` boundary is wider here and knowingly so: decision
0043 ruling 6 binds a linked worktree's whole shared `.git` read-write,
which leaves the shared ref store, the shared object store and the
per-worktree pointers writable. That is recorded as a known-open path at
`box_argv` and in 0054's consequences; narrowing it is a change under
0043 with its own number.

The driver also passes the absolute `bwrap` it probed and the two staged
directories as trusted argv paths, and the runner execs that binary
rather than searching `PATH` again. It is bubblewrap-only: a non-Linux host, no
`bwrap`, or a `bwrap` that cannot build the empty-root namespace refuses
the seat at start, naming the remedies, rather than burning an
implementation that cannot commit. A primary checkout whose workspace is
the repository root, a `read-only` seat and a `danger-full-access` seat
are left to dsh's own provider. The driver also sets
`commit.gpgsign=false` and the host identity on the child, so seat
commits are unsigned (CONTRIBUTING). The boundary in the record is still
`harness`; this is the harness's own runner, not a Brokkr boundary
(decision 0054, proposed).

The behavioral proof builds a real linked worktree and runs the real
bubblewrap. On a machine that cannot open a namespace it skips, and a
skipped Rust test prints `ok`, so CI's Linux legs set
`BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1`: every boundary proof then fails
rather than skips. Set it locally to check that a proof really ran.

The runner is served on Linux, and its tests are compiled and run on
every operating system CI covers — so what those tests may ASSUME about a
host is only what they asked it. A path is absolute because the operating
system says so, not because it starts with a slash: Windows calls
`/usr/bin/bwrap` relative, so the fixtures spell an absolute path the way
the host spells one. A mount's argv is joined with the host's own
separator, so an expected bind is built by joining, never by gluing a `/`
into the middle of a path. And where a proof genuinely needs a Unix
artifact — the shim that makes one promotion step refuse is a POSIX shell
script — it runs on the platforms that have one, while the arm that needs
no shim (a `git` that cannot be run at all) runs everywhere. What the
other operating systems prove in their own right is the refusal they
actually take: a host with no bubblewrap-compatible sandbox names itself
and the remedies at seat start.
