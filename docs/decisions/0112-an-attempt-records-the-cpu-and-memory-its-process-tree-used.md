# 0112 — An attempt records the CPU and memory its process tree used

Status: accepted — operator ruled in chat 2026-11-20, "Accept as revised" (https://github.com/feedback-loop-ai/brokkr/issues/1193#issuecomment-7040256520)
Date: 2026-11-20

Built: partial (#1193) — slice 1, the measurement, its journal extension and its read, is built; admission's use of it is unbuilt

## Context

Admission decides how many runs one machine drives at once (decision 0068
ruling 3, decision 0110 ruling 7), and today it decides that from a
declared ceiling, never from what a run costs the machine. The operator,
in chat on 2026-11-20 (#1193, epic #1194):

- "I don't believe YOU should be measuring, BROKKR should be measuring";
- "I guess we can average per recipe and base our assumptions there";
- "it would be nice to understand min, max, peak load, per phase";
- "disk growth per turn is not needed, just cpu and memory";
- "what about average / mean cpu / memory?";
- "because at the end of the day, what we want to understand is how much
  of this we can run at the same time...".

What main measures today (f7d231597):

- **The tracker.** One thread per engine walks every live attempt's tree
  every 100 ms (decision 0099, `TRACK` in
  `crates/brokkr-protocol/src/process/attempts.rs`). On Linux the tick
  walks the engine's own subtree through `/proc/<pid>/task/<tid>/children`
  and reads each `/proc/<pid>/stat`, keeping only state, ppid, pgrp and
  start time. On macOS the tick runs `ps -axo pid=,ppid=,pgid=,stat=,lstart=`
  under `LC_ALL=C`. The walk is the tick's own memory and never an
  attribution: nothing it reads reaches `observe`, `record`, `recorded`,
  `doubted` or a kill.
- **The tree.** Each driver is spawned as a session leader and, on Linux,
  its tree's child subreaper, so the whole seat tree (the harness CLI,
  `brokkr hands serve`, the boxes, cargo and rustc) descends from it. A
  panel's members run concurrently, one driver each, inside one attempt.
- **The ending.** Settlement returns an `AttemptReport`, and the engine
  writes `effect/succeeded`, `effect/failed` or `effect/indeterminate`.
  No duration and no usage is recorded. Wall time exists only as the
  envelope's `recorded_at`, which is evidence only.

## Rulings

1. **The driver measures its own tree on the tracker's existing 100 ms tick
   (decision 0099), for measurement only.** It is never an input to a kill,
   to `recorded` or to `doubted`. Linux reads rss and utime+stime from the
   `/proc/<pid>/stat` it already reads. macOS adds rss and cputime columns
   to the `ps` call it already makes.
2. **Per attempt:**
   - **memory:** peak (the highest summed RSS of the tree on any tick) and
     mean (that sum averaged over the attempt's ticks). Both are upper
     estimates, because shared pages count once per process;
   - **load:** peak (the most cores busy over a 1 s window) and mean (CPU
     time / wall time);
   - CPU time and wall time.

   No disk.
3. **Journaled as `effect-usage.v1`:** an optional, absent-by-default
   extension on `effect/succeeded`, `effect/failed` and
   `effect/indeterminate`, at `event_schema: 1`. The fold reads none of it.
   A panel attempt carries one entry per member.
4. **A read surface in the existing `recipes` family (#1169):** per recipe
   and per phase, the minimum, average and maximum of each measure, means
   and peaks alike.
5. **When decision 0080's per-attempt cgroup is built (#472),** Linux reads
   the cgroup's `memory.peak` and `cpu.stat` instead, and the fields keep
   their meaning. Do not build 0080 here, and do not preclude it.
6. **Admission does not read it.**

**The API cap (operator, 2026-11-20, "+15 protocol, +20 view").**
`quality/public-api/brokkr-protocol.txt` may grow by at most 15 lines and
`brokkr-view.txt` by at most 20. No other crate's listing moves.

## How slice 1 builds it

- **The measure.** On each tick the meter sums, over each open attempt's
  subtree under its leader, the resident set (Linux `/proc/<pid>/stat`
  field 24, in pages of `sysconf(_SC_PAGESIZE)`; macOS `rss`, in KiB) and
  the CPU time (Linux fields 14 to 17, `utime`, `stime`, `cutime` and
  `cstime`, in ticks of `sysconf(_SC_CLK_TCK)`; macOS `time` under
  `ps -S`, which sums exited children into their parent). The children's
  time is what keeps the thousands of short-lived `rustc` and build-script
  processes of a cargo build from being lost between ticks. A row whose
  numbers do not read adds nothing, and a tick that does not show the
  leader is skipped; neither fails the attempt. The tree's CPU time is the
  highest sum any tick read, since a time that leaves the tree reads as a
  dip. Peak load is the largest CPU increase over a window of ticks at
  least one second long, divided by that window's wall time, or, for an
  attempt whose ticks span less than a second, its load since spawn. Wall
  time runs on the monotonic clock from spawn to settlement.
- **The record.** `AttemptReport.usage` carries an `AttemptUsage`, `None`
  when no tick read the tree. The engine stamps it on the attempt's ending
  as `usage`: the measures themselves for a single seat, and
  `{"members": {"<member>": …}}` for a panel, one entry per member that a
  tick measured. Every field is a non-negative integer — bytes, thousandths
  of a core and milliseconds — so no seat-controlled text reaches the
  journal through it.
- **The read.** `brokkr recipes usage [NAME]` reads the journal it is
  pointed at and shows, per recipe and per phase, the number of ending
  events, how many of them carried no `usage`, how many drivers were
  measured, and the minimum, average and maximum of each of the six
  measures over those drivers. An attempt without `usage`, every attempt
  journaled before this decision among them, is counted as unmeasured and
  never as zero. `--json` emits the view model `brokkr-view` derives.

## Consequences

- Brokkr, not an operator or a seat, measures what an attempt cost the
  machine, from the rows its tracker already reads, at no extra read.
- A panel's members are measured one driver each; the read counts each
  member as one measured driver of its phase, so a phase's peak is a
  member's, not the panel's sum.
- The measure is an estimate: a process born and gone between two ticks
  counts only once its parent waits for it, a page shared by two
  processes counts twice, and an attempt shorter than one tick records no
  usage. Decision 0080's cgroup, once built, makes the memory exact
  without changing a field's meaning.
- Nothing that settles, kills, retries or admits an attempt reads usage,
  so a measurement that fails can only leave the record without it.
- Sequence steps are not stamped in slice 1; their attempts' endings read
  as unmeasured until a later slice stamps them.
- #1193's later slices give admission a reader of the per-recipe figures.
