# Secrets

Decision 0012: bundles and journals carry secret **names** only. An
exec template writes `{{secret:NAME}}`; the runner — and nothing
upstream of it — resolves that to the environment reference `$NAME`, and
hands the value, read from an operator-side store that lives outside
version control, to the child through its environment alone.

```
$ brokkr secrets set GITHUB_TOKEN    # value read from STDIN, never argv; store created 0600
set GITHUB_TOKEN in .forge/secrets.env

$ brokkr secrets list                # names, one per line — there is no value-printing verb
GITHUB_TOKEN

$ brokkr secrets remove GITHUB_TOKEN
removed GITHUB_TOKEN from .forge/secrets.env
```

The store defaults to `.forge/secrets.env` in the workspace; `brokkr run
--secrets-file` points elsewhere. A seat declares which names it binds,
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
