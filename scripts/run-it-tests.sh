#!/usr/bin/env bash
# Run one test command and fail when the run's own summary shows it
# executed no test (#543). `cargo test` and libtest cannot fail a run that
# selected nothing: a stale filter prints `running 0 tests` and exits 0.
# This is the one checked entry point every one-binary test command in the
# workflows and scripts runs through, so a filter that matches no test
# fails loudly whatever the command line, the environment or the profile
# did to the build. The command is passed whole as the script's
# arguments; its output is passed through unchanged.
#
#   scripts/run-it-tests.sh <the whole cargo test command>
#
# The decision is the run's own `test result:` summary, not the absence of
# one line: a run must print at least one `test result:` line, every such
# line must be readable, and every one must show a test passed or failed.
# Ignored and filtered-out tests are not execution. A run that prints no
# summary, one the guard cannot read, or one showing no test ran, is
# refused by that summary, named here with the command, and exits 1: that
# refusal replaces the command's own status. A run the guard accepts
# exits with the command's own status, zero or not.
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

summary_re='^test result: (ok|FAILED)\. ([0-9]+) passed; ([0-9]+) failed;'
summaries=0
unread=0
no_test=0
while IFS= read -r line; do
  case "$line" in
    'test result:'*)
      summaries=$((summaries + 1))
      if [[ "$line" =~ $summary_re ]]; then
        if [[ "${BASH_REMATCH[2]}" == "0" && "${BASH_REMATCH[3]}" == "0" ]]; then
          no_test=1
        fi
      else
        unread=1
      fi
      ;;
  esac
done <"$output"

if [ "$unread" -eq 1 ]; then
  printf 'run-it-tests: the command printed a test result: summary the guard cannot read, so no test is shown to have run: %s\n' "$*" >&2
  exit 1
fi

if [ "$summaries" -eq 0 ]; then
  printf 'run-it-tests: the command printed no test result: summary, so no test is shown to have run: %s\n' "$*" >&2
  exit 1
fi

if [ "$no_test" -eq 1 ]; then
  printf 'run-it-tests: the command ran 0 tests, so a filter matched no test: %s\n' "$*" >&2
  exit 1
fi

exit "$status"
