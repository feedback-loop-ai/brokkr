#!/usr/bin/env bash
# Re-measure the committed budgets (#342) from the budget tests' own reports
# and rewrite the files under quality/ that hold them:
#
#   quality/prompt-bytes.json  every model site's prompt bytes, exactly
#   quality/crate-count.json   Cargo.lock's [[package]] tables, exactly
#   quality/heap-bytes.json    each transcript kind's peak heap, plus a tenth,
#                              and the fleet page's and listing's (decision
#                              0089 rulings 6 and 9), SQLite's share never
#                              above 500 KiB, and opening a run's (ruling 9)
#
# It records, and gates nothing on, the fleet's first page's wall time in a
# release build, against ruling 9's 50 ms, and opening the 7,510-event run's
# beside the whole read's: wall clock is the machine's.
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
# Each run names its own `cargo test` on one plain line, so the test-target
# gate reads its filter (#423).
report() {
  "$@" -- --nocapture --test-threads=1 > "$scratch/out" 2>&1 || true
  cat "$scratch/out"
}

{
  report scripts/run-it-tests.sh cargo test --locked -p brokkr-runtime --test it budgets::
} > "$scratch/runtime"
{
  report cargo test --locked -p brokkr-cli --test heap_claude
  report cargo test --locked -p brokkr-cli --test heap_codex
  report cargo test --locked -p brokkr-cli --test heap_dsh
  report cargo test --locked -p brokkr-cli --test heap_fleet
  report cargo test --locked -p brokkr-cli --test heap_open
  report cargo test --locked -p brokkr-cli --test heap_run_view
} > "$scratch/heap"
{
  report cargo test --release --locked -p brokkr-cli --test heap_fleet
  report cargo test --release --locked -p brokkr-cli --test heap_open
  report cargo test --release --locked -p brokkr-cli --test heap_run_view
} > "$scratch/release"

# The harness writes a test's name before its first printed line, on the
# same line, so each record is matched wherever it starts. A site key holds
# no space; `-o` takes the record alone. The patterns are POSIX extended,
# with a literal tab, so the default grep of both supported hosts reads them.
tab=$'\t'
grep -oE "[^${tab} ]+${tab}[0-9]+ bytes${tab}[0-9]+ o200k tokens\$" "$scratch/runtime" > "$scratch/prompts" || true
grep -oE "packages${tab}[0-9]+\$" "$scratch/runtime" > "$scratch/packages" || true
grep -oE "heap${tab}[a-z-]+${tab}[0-9]+\$" "$scratch/heap" > "$scratch/peaks" || true
grep -oE "wall${tab}[a-z0-9-]+${tab}[0-9]+\$" "$scratch/release" > "$scratch/walls" || true

[[ -s "$scratch/prompts" ]] || { printf 'measure-budgets: the prompt test reported no site\n' >&2; exit 1; }
[[ "$(wc -l < "$scratch/packages")" -eq 1 ]] || { printf 'measure-budgets: the crate count was not reported once\n' >&2; exit 1; }
[[ "$(wc -l < "$scratch/peaks")" -eq 15 ]] || { printf 'measure-budgets: not every transcript kind, fleet page, listing, opening and run view reported its peak\n' >&2; exit 1; }
[[ "$(wc -l < "$scratch/walls")" -eq 5 ]] || { printf 'measure-budgets: the fleet page did not report its wall time at both sizes, or the opening, the whole read and the run view theirs\n' >&2; exit 1; }

jq -Rn --arg date "$(date -u +%F)" '
  [inputs | split("\t") | {key: .[0], value: (.[1] | split(" ")[0] | tonumber)}] | from_entries
  | {schema: "brokkr.prompt-budgets/v1",
     note: ("Bytes of the prompt each model site is handed, rendered by the driver'"'"'s render_prompt over the input the engine composes through its own SiteMarks (office text, the self realm'"'"'s house rules, the dialect prose each seat and panel member reads, and the hands, door and notice marks), the largest over every link that may serve the site; the run context and paths stand at fixed placeholders. Measured " + $date + " by scripts/measure-budgets.sh from crates/brokkr-runtime/tests/budgets.rs, which holds every site at or under its budget and refuses a site with no budget or a budget with no site. Raise a budget only in the pull request that grows the prompt, and say why."),
     budgets: .}' < "$scratch/prompts" > quality/prompt-bytes.json

jq -Rn --arg date "$(date -u +%F)" '
  [inputs | split("\t")[1] | tonumber][0]
  | {schema: "brokkr.crate-count/v1",
     note: ("The [[package]] tables Cargo.lock holds, measured " + $date + " by scripts/measure-budgets.sh from crates/brokkr-runtime/tests/budgets.rs, which refuses any count above this one. bincode-next is held at 2.1.0 because 3.x needs rustc 1.90 and the MSRV job checks every target on 1.88. A dependency moves the count in the pull request that adds it."),
     budgets: {packages: .}}' < "$scratch/packages" > quality/crate-count.json

jq -Rn --arg date "$(date -u +%F)" '
  [inputs | split("\t") | {key: .[1], value: (.[2] | tonumber)}] | from_entries
  | {schema: "brokkr.heap-budgets/v1",
     note: ("Peak heap bytes of projecting the largest transcript of each kind the reader admits, measured " + $date + " under dhat by scripts/measure-budgets.sh from crates/brokkr-cli/tests/heap_*.rs. Each budget is its measured peak plus a tenth, rounded up. The fleet'"'"'s (decision 0089 ruling 9, tests/heap_fleet.rs): the console'"'"'s window, the visible page and the one ahead, read and derived, first and scrolled deep (fleet-page, capped at ruling 9'"'"'s 256 KiB), the same for a page whose every row fills every bound (capped at 640 KiB), and SQLite'"'"'s own high-water beside them (fleet-page-sqlite, held to 500 KiB on every host by the operator'"'"'s ruling of 2026-10-21 on #753, the cap itself and not the measured peak plus a tenth, since CI'"'"'s macOS leg measures it higher than Linux: 448,912 against 399,176); and, a budget of its own that ruling 9'"'"'s caps do not hold, the first window painted at 80x24, read, derived and drawn into ratatui'"'"'s buffers (fleet-painted-frame, and at every bound); and brokkr runs --json with no other flag over 2,000 and 20,000 runs behind their summary, the hundred a refresh reads whole and a batch of keys of the rest (fleet-listing-behind, ruling 6, capped at 640 KiB); and opening the 7,510-event run, its chain verified one envelope at a time, in a journal of 10,000 and one of 1,000,000 events (tests/heap_open.rs): Brokkr'"'"'s share (open-run, capped at ruling 9'"'"'s 256 KiB) and SQLite'"'"'s (open-run-sqlite, held to the same 500 KiB on every host as fleet-page-sqlite is, the cap itself and not the measured peak plus a tenth, since CI'"'"'s macOS leg measures it higher than Linux: 444,016 against 387,552), and Brokkr'"'"'s with one envelope a mebibyte of escaped text, which the parser copies through its scratch buffer before the string it keeps (open-run-escaped, ruling 9 as amended 2026-10-24, its growth over open-run held to three envelopes); and the run view'"'"'s first screen, the 7,510-event run opened, its trail, seat rows and first journal page read and its view composed, in the same two journals (tests/heap_run_view.rs): Brokkr'"'"'s share (run-view, held to ruling 9'"'"'s 256 KiB and the run'"'"'s control envelopes), SQLite'"'"'s (run-view-sqlite, held to the same 500 KiB cap), and Brokkr'"'"'s with the run parked at its head and its park'"'"'s reason a mebibyte of plain text (run-view-stretched, ruling 9 as amended by the operator on 2026-10-25, its growth over the run parked unstretched held to the three copies of the reason the screen holds, the trail'"'"'s envelope, its fold and the view'"'"'s summary, and past one; SQLite'"'"'s growth held to the envelope and 64 KiB of slack, its allocator and page cache around the row, which CI'"'"'s macOS leg measures past the envelope)."),
     measured: .,
     budgets: (with_entries(.value = ((.value * 11 + 9) / 10 | floor))
               | .["fleet-page"] |= ([., 262144] | min)
               | .["fleet-page-at-every-bound"] |= ([., 655360] | min)
               | .["fleet-page-sqlite"] = 512000
               | .["open-run-sqlite"] = 512000
               | .["run-view-sqlite"] = 512000
               | .["fleet-listing-behind"] |= ([., 655360] | min)
               | .["open-run"] |= ([., 262144] | min))}' < "$scratch/peaks" > quality/heap-bytes.json

printf 'measured %s prompt sites, %s packages, peaks:' "$(wc -l < "$scratch/prompts")" "$(cut -f2 "$scratch/packages")"
cut -f2,3 "$scratch/peaks" | tr '\t\n' '= ' && printf '\n'
printf 'wall times, release, warm, in microseconds (the fleet first page against ruling 9: 50000; opening the 7,510-event run beside load; the run view'"'"'s first screen against ruling 9: 50000; not gated):'
cut -f2,3 "$scratch/walls" | tr '\t\n' '= ' && printf '\n'
