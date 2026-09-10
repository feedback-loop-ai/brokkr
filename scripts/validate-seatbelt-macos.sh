#!/bin/bash
# Native lifetime feasibility only. No model/provider accounts are used.
set -euo pipefail
if [ "${1:-}" != '--lifetime' ] || [ "$#" -ne 1 ]; then
  echo 'Usage: bash scripts/validate-seatbelt-macos.sh --lifetime' >&2
  echo 'Full Seatbelt acceptance is not implemented; this measures the R3 candidate.' >&2
  exit 2
fi
REPO=$(cd "$(dirname "$0")/.." && pwd)
EVIDENCE_ROOT=$(mktemp -d "${TMPDIR:-/tmp}/brokkr-seatbelt-lifetime.XXXXXX")
printf 'Evidence directory: %s\n' "$EVIDENCE_ROOT"
finish() {
  RUN_EXIT=$?
  # Only the completed measurement can confer exit 1. Prerequisites may
  # themselves return 1, which must remain an experiment ERROR (exit 2).
  if [ "$RUN_EXIT" -ne 1 ] || [ "${MEASUREMENT_EXIT:-2}" -ne 1 ] ||
     ! grep -q '"valid_measurement":true' "$EVIDENCE_ROOT/measurement/summary.json" 2>/dev/null; then
    RUN_EXIT=2
  fi
  trap - EXIT
  set +e
  if [ -n "${TEE_PID:-}" ]; then
    exec 1>&3 2>&4
    if ! wait "$TEE_PID"; then
      echo 'ERROR: transcript capture failed.' >&2
      RUN_EXIT=2
    fi
  fi
  rm -f "$EVIDENCE_ROOT/runner.fifo"
  printf '%s\n' "$RUN_EXIT" > "$EVIDENCE_ROOT/exit-code.txt"
  if [ ! -f "$EVIDENCE_ROOT/measurement/summary.json" ]; then
    printf '{"status":"ERROR","r3_closed":false,"seatbelt_activation_authorized":false,"reason":"measurement did not produce a summary; inspect runner.log"}\n' > "$EVIDENCE_ROOT/runner-summary.json"
  fi
  tar -czf "$EVIDENCE_ROOT.tgz" -C "$EVIDENCE_ROOT" .
  ARCHIVE_EXIT=$?
  printf 'Runner exit: %s; evidence: %s.tgz\n' "$RUN_EXIT" "$EVIDENCE_ROOT"
  if [ "$ARCHIVE_EXIT" -ne 0 ]; then
    echo 'Archive creation failed; preserve the evidence directory.' >&2
    exit 2
  fi
  exit "$RUN_EXIT"
}
trap finish EXIT
# Keep original stdout for progress and a complete transcript for errors.
exec 3>&1 4>&2
# A direct background child is waitable on macOS Bash 3.2; process
# substitution's PID is not reliably waitable there.
mkfifo "$EVIDENCE_ROOT/runner.fifo"
tee "$EVIDENCE_ROOT/runner.log" < "$EVIDENCE_ROOT/runner.fifo" &
TEE_PID=$!
exec > "$EVIDENCE_ROOT/runner.fifo" 2>&1
cd "$REPO"
{
  date -u
  uname -s
  uname -m
  git rev-parse HEAD
  git status --porcelain
} > "$EVIDENCE_ROOT/host.txt"
if [ "$(uname -s)" != Darwin ]; then
  echo 'ERROR: native measurement requires macOS. Linux self-tests are separate.'
  exit 2
fi
if [ -n "$(git status --porcelain --untracked-files=normal)" ]; then
  echo 'ERROR: use a clean published candidate checkout; evidence must name its exact source.'
  exit 2
fi
sw_vers >> "$EVIDENCE_ROOT/host.txt"
xcode-select -p >> "$EVIDENCE_ROOT/host.txt"
xcrun clang --version >> "$EVIDENCE_ROOT/host.txt" 2>&1
# Homebrew rustc may precede rustup's proxy in PATH and cannot parse +stable.
RUSTUP_BIN="${CARGO_HOME:-$HOME/.cargo}/bin/rustup"
if [ ! -x "$RUSTUP_BIN" ]; then
  RUSTUP_BIN=$(command -v rustup) || {
    echo 'ERROR: rustup missing; install rustup and its stable toolchain per issue #253.'
    exit 2
  }
fi
"$RUSTUP_BIN" run stable rustc --version --verbose >> "$EVIDENCE_ROOT/host.txt"
if [ ! -x /usr/bin/sandbox-exec ]; then
  echo 'ERROR: /usr/bin/sandbox-exec missing or not executable.'
  exit 2
fi
shasum -a 256 experiments/seatbelt-lifetime/main.rs scripts/validate-seatbelt-macos.sh > "$EVIDENCE_ROOT/source-sha256.txt"
"$RUSTUP_BIN" run stable rustc --edition 2021 -D warnings experiments/seatbelt-lifetime/main.rs -o "$EVIDENCE_ROOT/lifetime"
shasum -a 256 "$EVIDENCE_ROOT/lifetime" > "$EVIDENCE_ROOT/binary-sha256.txt"
echo 'Running 1 negative control and 12 candidate cases; allow roughly two minutes.'
echo 'Exit 1 means valid feasibility measurements with R3 still OPEN; exit 2 means invalid/incomplete evidence.'
echo 'No filesystem/network containment or Brokkr hands integration is claimed.'
set +e
"$EVIDENCE_ROOT/lifetime" --native "$EVIDENCE_ROOT/measurement"
MEASUREMENT_EXIT=$?
set -e
if [ -f "$EVIDENCE_ROOT/measurement/summary.json" ]; then
  cat "$EVIDENCE_ROOT/measurement/summary.json"
fi
# The EXIT trap waits for the transcript before archiving.
printf 'Measurement exit: %s\n' "$MEASUREMENT_EXIT" > "$EVIDENCE_ROOT/measurement-exit.txt"
exit "$MEASUREMENT_EXIT"
