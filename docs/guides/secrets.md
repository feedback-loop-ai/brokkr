# Secrets

Decision 0012: bundles and journals carry secret **names** only. A
driver template writes `{{secret:NAME}}`; the runner — and nothing
upstream of it — resolves that to a value from an operator-side store
that lives outside version control.

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
compilation fails on an undeclared one, and any bound value that appears
in captured stderr is masked to `[secret:NAME]` on raw bytes before the
string ever exists. The `{{secret:NAME}}` spelling itself is not
secret-bearing, which is why it is journalable and the resolved command
line is not.
