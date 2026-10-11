# shellcheck shell=bash
# The libtest summary grammar, the one home for both readers of a test
# run's output: scripts/run-it-tests.sh refuses a run unless every summary
# shows a test executed (#543), and scripts/test-shards.sh refuses a
# filtered shard unless some summary does (#633). Sourced, never run.

# How one line reads as a `test result:` summary: 0 when it shows a test
# passed or failed, 1 when it shows none did, 2 when it cannot be read.
# Ignored and filtered-out tests are not execution.
summary_executed() {
  local summary_re='^test result: (ok|FAILED)\. ([0-9]+) passed; ([0-9]+) failed;'
  [[ "$1" =~ $summary_re ]] || return 2
  ((10#${BASH_REMATCH[2]} + 10#${BASH_REMATCH[3]} > 0))
}
