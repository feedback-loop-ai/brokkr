# shellcheck shell=bash
# The libtest summary grammar, the one home for both readers of a test
# run's output: scripts/run-it-tests.sh refuses a run unless every summary
# shows a test executed (#543), and scripts/test-shards.sh refuses a shard
# unless every summary is readable and, for a filtered shard, some summary
# shows a test executed (#633). Sourced, never run.

# How one line reads as a `test result:` summary: 0 when it shows a test
# passed or failed, 1 when it shows none did, 2 when it cannot be read.
# Ignored and filtered-out tests are not execution.
summary_executed() {
  local summary_re='^test result: (ok|FAILED)\. ([0-9]+) passed; ([0-9]+) failed;'
  [[ "$1" =~ $summary_re ]] || return 2
  ((10#${BASH_REMATCH[2]} + 10#${BASH_REMATCH[3]} > 0))
}

# How a run's whole output, the file $1, reads: every line that starts
# `test result:` is ruled on by summary_executed. Sets summaries to how many
# there are, and each of executed, no_test and unread to 1 when some summary
# shows a test executed, shows none did, or cannot be read. Anything but the
# grammar's two readings is unread, so a reader that could not run refuses
# rather than passes.
# shellcheck disable=SC2034 # the counts are read by the sourcing script
summaries_read() {
  local line verdict
  summaries=0 executed=0 no_test=0 unread=0
  while IFS= read -r line || [ -n "$line" ]; do
    case "$line" in
      'test result:'*)
        summaries=$((summaries + 1))
        verdict=0
        summary_executed "$line" || verdict=$?
        case "$verdict" in
          0) executed=1 ;;
          1) no_test=1 ;;
          *) unread=1 ;;
        esac
        ;;
    esac
  done <"$1"
}
