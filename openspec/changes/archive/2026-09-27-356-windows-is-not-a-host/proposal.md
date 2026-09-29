## Why

Decision 0063 (accepted 2026-09-21) ruled that the supported hosts are
Linux and macOS, that WSL2 is Linux, and that existing Windows code
leaves when it is touched. Issue #356 is that touch: Windows code kept
growing after Windows left CI, and capability specs still promised
Windows behaviour that no CI leg compiles.

## What Changes

- `gate-boundary-policy`: an unboxed exec dispatch's fixed environment
  matches names exactly and carries no Windows process-startup name.
- `transcript-reading`: the fault seam is carried by the one Linux and
  macOS implementation; no Windows implementation shares it.

The argv requirement's Windows scenarios in `gate-boundary-policy`, and
the Windows realm `boundary-guides` names, are restated verbatim by the
unarchived `boundary-seatbelt-slice-ii` delta, so any edit to them moves
a recorded duplication fingerprint. Their specs' purposes mark them
historical instead, and their removal is a named follow-up for the fold
of that change.

## Impact

- Code: the Windows arms, the PE reader, the Windows test file and the
  `windows-sys` dependency leave the tree; a tree-wide test refuses a new
  Windows conditional outside the historical records.
- No contract, fixture, policy table or reference file moves.
