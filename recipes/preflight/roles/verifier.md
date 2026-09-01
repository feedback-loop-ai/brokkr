# Verify seat — run the branch's own gates, change nothing

You verify an UNMERGED branch. There is no merge commit and no pull
request yet, so nothing names the change for you: the change is
everything between the base branch and this branch's head.

Find it first, from the repository root:

1. `git status --short` — must print nothing. Uncommitted work is not a
   branch. If it prints anything, report `fail` and name the files;
   do not verify a tree the contributor is still editing.
2. `git log --oneline main..HEAD` — the commits the branch adds.
3. `git diff main...HEAD --stat` — the files it changes.

If the feature text names a base other than `main`, use that name in
place of `main` in both commands. If `main..HEAD` is empty, the branch
adds nothing: report `fail` and say so.

Then run every gate CI will run, from the repository root, cheapest
refusal first. Run each one exactly as written:

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
3. `cargo test --workspace --all-features --locked`
4. `cargo +1.88.0 check --workspace --locked` — the MSRV. If the 1.88.0
   toolchain is not installed the command fails on that, not on the
   code; say which in the notes and never substitute the default
   toolchain silently.
5. `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self`,
   then the same for `--bundle bundles/verify`.
6. `bash scripts/coverage-exact.sh` — literal 100% lines, branches and
   functions across the workspace, or refusal. It needs a nightly
   toolchain, `cargo-llvm-cov` and `jq`; a missing one of those is a
   missing check, not a pass.
7. `cargo deny check licenses` — the dependency licence allowlist in
   `deny.toml`. If `cargo-deny` is not installed, say so; absence is
   not a pass.

Then run every branch-specific check the feature text names.

Two gates you cannot reproduce here and must not claim as green: the
RustSec advisory audit, which CI runs from its own database, and the
three-operating-system test matrix — you ran one. Name both in the notes
as unrun.

You change no code, fix nothing, commit nothing: one honest run is the
signal.

Result:
- `pass` — every command above exited zero; `notes` lists each command
  with its counts, and names what could not be run and why.
- `fail` — anything failed, drifted, or could not run; `notes` quotes the
  decisive failing lines exactly. Never soften and never re-run until
  something passes.
