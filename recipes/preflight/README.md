# preflight — the machine reviews your branch before you propose it

Two seats, both gates: `verify` → `review` → `done`/`stop`. No intake,
no implement, no ship. The table ends after `review` with a terminal
ruling, so there is no phase that changes your branch and no phase that
merges it: findings are the only thing a preflight run produces.

That is a property of the table, not of a sandbox. Both seats run on
your machine with your credentials and are charged to touch nothing; the
table's part is that a seat which reports having applied fixes hard-stops
the run (`REVIEW-CLEAN-FIXED`, `REVIEW-RESIDUAL-FIXED`) instead of being
believed. If you want the assurance rather than the promise, the check is
one line: `git status --short` and `git log --oneline main..HEAD` should
read the same after the run as before it.

```
brokkr run --recipe preflight --repo . --feature "<what the branch does, and its base if not main>"
```

Run it on your own branch before opening a pull request. You get the
same adversarial, typed, journaled findings the machine's own work
faces, from the same two seats, before a human has read a line.

## What each seat does

| Seat | Class | Results | Runs |
|---|---|---|---|
| `verify` | `gate` | `pass`, `fail` | The gates CI will run, locally: format, clippy, the workspace suite, the MSRV check, both bundle compiles, the exact-coverage script, the licence allowlist, the release build. |
| `review` | `gate` | `clean`, `residual`, `security-hold` | Adversarial read of `git diff main...HEAD` across correctness, fit and security. Read-only. |

`verify` gets 5400 seconds because it runs the coverage gate, which
rebuilds the workspace instrumented; `review` gets 3600, as elsewhere.
Both allow two attempts — the bound is on a driver that fails or hangs,
not on retrying until something passes.

## Where it differs from `bundles/verify`

`bundles/verify` has the same two phases and the same result
vocabularies, and this recipe deliberately keeps its rules rule for
rule. What differs is what the seats are pointed at:

| | `bundles/verify` | `recipes/preflight` |
|---|---|---|
| The change is | already delivered, named by merge commit or diff range (`git show <sha>`) | unmerged, found by diffing the branch against its base (`git diff main...HEAD`) |
| Run by | the operator, after a slice lands | a contributor, before a pull request exists |
| Driver tools | includes `gh pr view` / `gh run view` | no `gh` — there is nothing open to read |
| `verify` runs | the suite plus the two bundle compiles | every gate in `.github/workflows/ci.yml` that a laptop can reproduce |

It is a standalone recipe with its own `policy.json`, not an `extends`
of `bundles/verify`: `extends` names a recipe in the library, and the
library is `recipes/`, so the `bundles/` boundary is not crossable that
way.

## Why both seats declare `class: "gate"`

Decision 0021: a judging seat stands on an adapter's declared trust
tier, checked when the bundle compiles, before any prompt exists. Both
seats here judge, so both declare `"class": "gate"` and both name the
`claude` driver, the one shipped adapter declared trusted. The compiler
refuses an untrusted driver at a gate.

## The two checks a preflight cannot give you

The RustSec advisory audit runs in CI against its own database, and the
test matrix runs on three operating systems. A preflight run has one
machine and no advisory database of CI's vintage. The verifier's charter
tells it to name both as unrun rather than imply they passed.

## The terminal shape is a test, not a comment

`crates/brokkr-runtime/tests/preflight_shape.rs` asserts that this
table's phases are exactly `verify`, `review`, `done`, `stop`; that no
rule names `intake`, `implement` or `ship`; and that every rule from
`review` ends in a terminal. Adding a ship phase back here fails a test.

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for where this sits in the
walk from clone to pull request.
