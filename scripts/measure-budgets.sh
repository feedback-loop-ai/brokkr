#!/usr/bin/env bash
# Re-measure the committed budgets (#342) from the budget tests' own reports
# and rewrite the files under quality/ that hold them:
#
#   quality/prompt-bytes.json  every model site's prompt bytes, exactly
#   quality/crate-count.json   Cargo.lock's [[package]] tables, exactly
#   quality/heap-bytes.json    each transcript kind's peak heap, plus a tenth
#
# The tests print what they measured whether or not it is within budget, so
# a moved tree is measured against itself. Review the diff: a budget that
# rises is a regression someone must name in the pull request.
# quality/binary-size.json is measured from CI's release artifact, never a
# local build, and the CPU budgets have no file: CI compares each pull
# request against its base.
set -euo pipefail
export LC_ALL=C
cd "$(git rev-parse --show-toplevel)"

scratch="$(mktemp -d "${TMPDIR:-/tmp}/forge-budgets.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT

# A test over budget still prints its measurement before it fails, so a
# failing run is read, not refused; a run that reports nothing is refused.
report() {
  cargo test --locked "$@" -- --nocapture --test-threads=1 > "$scratch/out" 2>&1 || true
  cat "$scratch/out"
}

report -p brokkr-runtime --test budgets > "$scratch/runtime"
for kind in claude codex dsh; do
  report -p brokkr-cli --test "heap_$kind"
done > "$scratch/heap"

# The harness writes a test's name before its first printed line, on the
# same line, so each record is matched wherever it starts. A site key holds
# no space; `-o` takes the record alone.
grep -oP '[^\t ]+\t[0-9]+ bytes\t[0-9]+ o200k tokens$' "$scratch/runtime" > "$scratch/prompts" || true
grep -oP '\bpackages\t[0-9]+$' "$scratch/runtime" > "$scratch/packages" || true
grep -oP '\bheap\t[a-z-]+\t[0-9]+$' "$scratch/heap" > "$scratch/peaks" || true

[[ -s "$scratch/prompts" ]] || { printf 'measure-budgets: the prompt test reported no site\n' >&2; exit 1; }
[[ "$(wc -l < "$scratch/packages")" -eq 1 ]] || { printf 'measure-budgets: the crate count was not reported once\n' >&2; exit 1; }
[[ "$(wc -l < "$scratch/peaks")" -eq 3 ]] || { printf 'measure-budgets: not every transcript kind reported its peak\n' >&2; exit 1; }

jq -Rn --arg date "$(date -u +%F)" '
  [inputs | split("\t") | {key: .[0], value: (.[1] | split(" ")[0] | tonumber)}] | from_entries
  | {schema: "brokkr.prompt-budgets/v1",
     note: ("Bytes of the prompt each model site is handed, rendered by the driver'"'"'s render_prompt over the input the engine composes (office text, the self realm'"'"'s house rules, the phase'"'"'s dialect instructions outside review). Measured " + $date + " by scripts/measure-budgets.sh from crates/brokkr-runtime/tests/budgets.rs, which holds every site at or under its budget and refuses a site with no budget or a budget with no site. Raise a budget only in the pull request that grows the prompt, and say why."),
     budgets: .}' < "$scratch/prompts" > quality/prompt-bytes.json

jq -Rn --arg date "$(date -u +%F)" '
  [inputs | split("\t")[1] | tonumber][0]
  | {schema: "brokkr.crate-count/v1",
     note: ("The [[package]] tables Cargo.lock holds, measured " + $date + " by scripts/measure-budgets.sh from crates/brokkr-runtime/tests/budgets.rs, which refuses any count above this one. bincode-next is held at 2.1.0 because 3.x needs rustc 1.90 and the MSRV job checks every target on 1.88. A dependency moves the count in the pull request that adds it."),
     budgets: {packages: .}}' < "$scratch/packages" > quality/crate-count.json

jq -Rn --arg date "$(date -u +%F)" '
  [inputs | split("\t") | {key: .[1], value: (.[2] | tonumber)}] | from_entries
  | {schema: "brokkr.heap-budgets/v1",
     note: ("Peak heap bytes of projecting the largest transcript of each kind the reader admits, measured " + $date + " under dhat by scripts/measure-budgets.sh from crates/brokkr-cli/tests/heap_*.rs. Each budget is its measured peak plus a tenth, rounded up."),
     measured: .,
     budgets: (with_entries(.value = ((.value * 11 + 9) / 10 | floor)))}' < "$scratch/peaks" > quality/heap-bytes.json

printf 'measured %s prompt sites, %s packages, peaks:' "$(wc -l < "$scratch/prompts")" "$(cut -f2 "$scratch/packages")"
cut -f2,3 "$scratch/peaks" | tr '\t\n' '= ' && printf '\n'
