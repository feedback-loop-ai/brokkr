#!/usr/bin/env bash
# Run one test command and fail when a filtered run executes no test
# (#543). `cargo test` and libtest cannot fail a run that selected
# nothing: a stale filter prints `running 0 tests` and exits 0. This is
# the one checked entry point every one-binary test command in the
# workflows and scripts runs through, so a filter that matches no test
# fails loudly whatever the command line, the environment or the profile
# did to the build. The command is passed whole as the script's
# arguments; its output is passed through unchanged.
#
#   scripts/run-it-tests.sh <the whole cargo test command>
#
# A run that reports `running 0 tests` is refused by that line, named
# here with the command; the command's own non-zero exit still stands.
set -uo pipefail

if [ "$#" -eq 0 ]; then
  printf 'run-it-tests: no command given; pass a whole test command\n' >&2
  exit 2
fi

output="$(mktemp "${TMPDIR:-/tmp}/brokkr-run-it-tests.XXXXXX")"
trap 'rm -f "$output"' EXIT
"$@" >"$output" 2>&1
status=$?
cat "$output"
if grep -q '^running 0 tests$' "$output"; then
  printf 'run-it-tests: the command ran 0 tests, so a filter matched no test: %s\n' "$*" >&2
  exit 1
fi
exit "$status"
