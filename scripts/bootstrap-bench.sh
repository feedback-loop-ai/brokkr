#!/usr/bin/env bash
# The bootstrap budgets, as a check rather than a promise.
#
# Two paths are timed on a clean tempdir, and this script exits non-zero
# when either blows its budget:
#
#   install-from-tarball   <= 60s    (BUDGET_INSTALL_SECONDS)
#   init-to-first-run      <= 300s   (BUDGET_FIRST_RUN_SECONDS)
#
# ────────────────────────────────────────────────────────────────────
# WHAT IS MOCKED, SAID PLAINLY
#
# Two steps would otherwise make these numbers dishonest, and both are
# stood in for. The script prints this same paragraph when it runs, so a
# green line in a CI log can never be read as more than it is.
#
# 1. THE DOWNLOAD IS NOT A DOWNLOAD. The install path fetches the
#    tarball and its SHA256SUMS over `file://` from a staging directory
#    on this machine — the same `curl -LO`, `sha256sum -c` and `tar xzf`
#    commands the quickstart names, over a local URL instead of GitHub's.
#    So the measured 60s covers unpack, checksum verification and a
#    `brokkr --version` smoke test, and it does NOT cover network
#    transfer. A pass here is NOT evidence that the real GitHub download
#    is fast; it is evidence that everything AROUND the transfer costs
#    almost nothing, which is the part this repository controls.
#
# 2. NO AGENT SESSION IS SPAWNED. `brokkr run` would otherwise start a
#    real, billed Claude Code session per seat — unusable inside a timing
#    gate. The run below is driven by a stub standing in for the `claude`
#    binary, via the adapter's own `FORGE_CLAUDE_BIN` override: it reads
#    the seat prompt, writes the typed result file the engine reads, and
#    exits. Everything else is real — the real bundle `brokkr init`
#    scaffolded, the real compile with its gate-class trust check, the
#    real driver transport, the real journal.
#
#    (The scripted `brokkr fake-driver` would have been the more obvious
#    stand-in, but it cannot seat this scaffold: `fake-driver` is not a
#    `brokkr driver <kind>` dispatch, so no adapter can declare its trust
#    tier, and a gate-class seat without a trusted adapter refuses to
#    compile — decision 0021 ruling 2. Swapping the drivers would have
#    meant demoting the scaffold's three gate seats to `work`, and then
#    the thing being timed would no longer be the thing being shipped.
#    Stubbing the agent binary leaves the bundle byte-identical to what
#    an operator gets.)
#
# So: the timed run used a stubbed `claude` binary, not Claude Code, and
# the timed install used a local file:// URL, not GitHub.
# ────────────────────────────────────────────────────────────────────
#
# Usage:
#   scripts/bootstrap-bench.sh [--binary <path to a built brokkr>]
#
# With no --binary, a release build is made first. Building is NOT
# timed: the budgets are an operator's install experience, and an
# operator installs a built artifact.

set -euo pipefail

BUDGET_INSTALL_SECONDS="${BUDGET_INSTALL_SECONDS:-60}"
BUDGET_FIRST_RUN_SECONDS="${BUDGET_FIRST_RUN_SECONDS:-300}"

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
binary=""

while [ $# -gt 0 ]; do
  case "$1" in
    --binary) binary="$2"; shift 2 ;;
    *) printf 'bootstrap-bench: unknown argument %s\n' "$1" >&2; exit 2 ;;
  esac
done

for tool in curl tar git; do
  command -v "$tool" >/dev/null 2>&1 || {
    printf 'bootstrap-bench refusal: %s is required\n' "$tool" >&2
    exit 1
  }
done

# macOS ships `shasum -a 256`; Linux ships `sha256sum`. The quickstart
# names `sha256sum`, so that is what is timed where it exists.
if command -v sha256sum >/dev/null 2>&1; then
  sha256_write() { sha256sum "$1"; }
  sha256_check() { sha256sum --ignore-missing -c "$1"; }
else
  sha256_write() { shasum -a 256 "$1"; }
  sha256_check() { shasum -a 256 --ignore-missing -c "$1"; }
fi

say() { printf '%s\n' "$*"; }
rule() { say '────────────────────────────────────────────────────────'; }

rule
say 'bootstrap-bench — the 60s / 5min budgets, measured'
rule
say 'MOCKED, so the numbers are not read as more than they are:'
say '  · the download is a file:// read from a local staging dir,'
say '    NOT a GitHub transfer. Network time is not in the 60s.'
say '  · the run is driven by a stub standing in for the `claude`'
say '    binary (FORGE_CLAUDE_BIN). No agent session is spawned and'
say '    nothing is billed. The bundle, compile, gate-class trust'
say '    check, driver transport and journal are all real.'
rule

if [ -z "$binary" ]; then
  say 'building brokkr (release, not timed)…'
  ( cd "$repo_root" && cargo build --release --locked -p brokkr-cli )
  binary="$repo_root/target/release/brokkr"
fi
[ -x "$binary" ] || {
  printf 'bootstrap-bench refusal: %s is not an executable\n' "$binary" >&2
  exit 1
}
binary="$(cd "$(dirname "$binary")" && pwd)/$(basename "$binary")"

workdir="$(mktemp -d "${TMPDIR:-/tmp}/bootstrap-bench.XXXXXX")"
trap 'rm -rf "$workdir"' EXIT

failures=0
report() {
  # name, measured seconds, budget seconds
  if [ "$2" -le "$3" ]; then
    printf 'ok    %-22s %3ss  (budget %ss)\n' "$1" "$2" "$3"
  else
    printf 'BLOWN %-22s %3ss  (budget %ss)\n' "$1" "$2" "$3"
    failures=$((failures + 1))
  fi
}

# ── path 1: install from the attested release tarball ────────────────
# Staged first, outside the clock: making the archive is the release
# workflow's job, not the installer's.
mirror="$workdir/mirror"
mkdir -p "$mirror/stage"
case "$(uname -s)" in
  Darwin) os=macos ;;
  *) os=linux ;;
esac
case "$(uname -m)" in
  arm64|aarch64) arch=aarch64 ;;
  *) arch=x86_64 ;;
esac
archive="brokkr-$os-$arch.tar.gz"
cp "$binary" "$mirror/stage/brokkr"
tar czf "$mirror/$archive" -C "$mirror/stage" brokkr
( cd "$mirror" && sha256_write "$archive" >SHA256SUMS )

install_dir="$workdir/install"
mkdir -p "$install_dir"
say "timing install-from-tarball ($archive)…"
started=$(date +%s)
(
  cd "$install_dir"
  curl -sSfLO "file://$mirror/$archive"
  curl -sSfLO "file://$mirror/SHA256SUMS"
  sha256_check SHA256SUMS
  tar xzf "$archive"
  ./brokkr --version
) >"$workdir/install.log" 2>&1 || {
  printf 'bootstrap-bench refusal: the install path failed:\n' >&2
  cat "$workdir/install.log" >&2
  exit 1
}
install_seconds=$(( $(date +%s) - started ))

# ── path 2: init to the first completed effect ───────────────────────
# The stub the adapter spawns instead of Claude Code. It receives the
# seat prompt on stdin — the same prompt a real session would — and
# writes the typed result to the exact file the prompt names, which is
# the only channel the engine reads.
stub_dir="$workdir/stub"
mkdir -p "$stub_dir"
cat >"$stub_dir/claude" <<'STUB'
#!/usr/bin/env bash
# Stands in for the Claude Code CLI inside the benchmark only. It reads
# the prompt, answers the phase it was seated in, and exits. It writes
# no code and reviews nothing: what is being timed is the ENGINE's cost
# to reach a first completed effect, so the seat's own work is zero on
# purpose and the number is a floor for the machinery, not an estimate
# of a real slice.
set -uo pipefail
prompt="$(cat)"
# The result contract puts the one path the engine reads on its own
# four-space-indented line; the last such line is it, because a role
# charter's indented lines are commands and come first.
result_file="$(printf '%s\n' "$prompt" | sed -n 's/^    \(.*\.json\)$/\1/p' | tail -n 1)"
phase="$(printf '%s\n' "$prompt" | sed -n 's/^Phase: \([a-z-]*\).*/\1/p' | head -n 1)"
case "$phase" in
  intake) result=resolved ;;
  # A scripted `blocked` is a HARD stop at implement, so the run ends
  # one seat past its first completed effect. The window therefore
  # OVER-measures time-to-first-effect and never under-measures it.
  *) result=blocked ;;
esac
if [ -z "$result_file" ]; then
  echo "stub: no result path in the prompt" >&2
  exit 1
fi
mkdir -p "$(dirname "$result_file")"
printf '{"result":"%s","notes":"benchmark stub; no agent ran"}\n' "$result" >"$result_file"
printf '{"type":"result","subtype":"success","num_turns":1,"total_cost_usd":0.0}\n'
STUB
chmod +x "$stub_dir/claude"

trial="$workdir/trial"
mkdir -p "$trial"
git -C "$trial" init -q
git -C "$trial" config user.email bench@example.invalid
git -C "$trial" config user.name 'bootstrap bench'
git -C "$trial" config commit.gpgSign false
printf '{\n  "name": "bench-app",\n  "private": true\n}\n' >"$trial/package.json"
printf '.forge/\n' >"$trial/.gitignore"
git -C "$trial" add -A
git -C "$trial" commit -qm 'bench fixture'

say 'timing init-to-first-run (init . + run to first completed effect)…'
started=$(date +%s)
set +e
(
  cd "$trial"
  export FORGE_CLAUDE_BIN="$stub_dir/claude"
  "$install_dir/brokkr" init .
  "$install_dir/brokkr" run --bundle . --repo . --db .forge/forge.db \
    --feature 'bootstrap benchmark: reach one completed effect'
) >"$workdir/run.log" 2>&1
run_exit=$?
set -e
first_run_seconds=$(( $(date +%s) - started ))

# Exit 3 is a stopped run, which is what the scripted `blocked` at
# implement rules. Anything else means the path did not execute and the
# number above would be timing a failure.
if [ "$run_exit" -ne 3 ]; then
  printf 'bootstrap-bench refusal: the init-to-first-run path exited %s, expected 3 (stopped):\n' "$run_exit" >&2
  cat "$workdir/run.log" >&2
  exit 1
fi
# And the first effect really did complete, ended by the ruling the
# stub scripted: a run that stopped without one would have timed
# nothing worth reporting. (This is the same pair of assertions
# `crates/brokkr-cli/tests/bootstrap_bench.rs` makes, so the mechanism
# is under the workspace suite and not only under this script.)
( cd "$trial" && "$install_dir/brokkr" inspect --run latest --db .forge/forge.db --json ) \
  >"$workdir/inspect.json" 2>&1 || true
for expected in 'effect/succeeded' 'IMPL-BLOCKED'; do
  if ! grep -q "$expected" "$workdir/inspect.json"; then
    printf 'bootstrap-bench refusal: the timed run never reached %s:\n' "$expected" >&2
    cat "$workdir/run.log" >&2
    exit 1
  fi
done

rule
report 'install-from-tarball' "$install_seconds" "$BUDGET_INSTALL_SECONDS"
report 'init-to-first-run' "$first_run_seconds" "$BUDGET_FIRST_RUN_SECONDS"
rule
say 'Read these as: everything the repository controls, minus the'
say 'network transfer and minus the agent. Both stand-ins are named'
say 'at the top of this output and in the script itself.'

if [ "$failures" -ne 0 ]; then
  printf 'bootstrap-bench: %s budget(s) blown\n' "$failures" >&2
  exit 1
fi
