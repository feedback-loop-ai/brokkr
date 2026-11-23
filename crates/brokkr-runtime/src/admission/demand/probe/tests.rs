//! Decision 0125 ruling 1's Linux measures, read from planted text: never
//! the machine's own numbers.

use std::time::Duration;

use super::{available, idle, load, meminfo, stat, Available, Ticks, Unmeasured};

const MEMINFO: &str = "MemTotal:       16384000 kB\nMemFree:         1000 kB\n\
                       MemAvailable:    8192000 kB\nBuffers:           10 kB\n";

/// Four cores; the aggregate line's idle is 4000 and its iowait 100.
fn stat_text(user: u64, idle: u64) -> String {
    format!(
        "cpu  {user} 0 100 {idle} 100 0 0 0 7 7\ncpu0 1 0 0 1 0 0 0 0 0 0\n\
         cpu1 1 0 0 1 0 0 0 0 0 0\ncpu2 1 0 0 1 0 0 0 0 0 0\ncpu3 1 0 0 1 0 0 0 0 0 0\n\
         intr 1 2 3\nctxt 9\n"
    )
}

#[test]
fn meminfo_reads_available_and_total_in_bytes_and_refuses_a_missing_field() {
    assert_eq!(
        meminfo(MEMINFO),
        Some((8_192_000 * 1024, 16_384_000 * 1024))
    );
    assert_eq!(meminfo("MemTotal: 1 kB\n"), None);
    assert_eq!(meminfo("MemTotal: 1 kB\nMemAvailable: 2 MB\n"), None);
}

/// Idle is idle plus iowait, the total stops at steal so guest time is
/// not counted twice, and the cores are the `cpuN` lines.
#[test]
fn stat_reads_idle_total_and_cores_from_the_aggregate_line() {
    assert_eq!(
        stat(&stat_text(900, 4000)),
        Some(Ticks {
            idle: 4100,
            total: 900 + 100 + 4000 + 100,
            cores: 4
        })
    );
    assert_eq!(stat("cpu0 1 2 3 4 5 6 7 8\n"), None);
    assert_eq!(stat("cpu  1 2 3\ncpu0 1 1 1 1 1 1 1 1\n"), None);
    assert_eq!(stat("cpu  1 2 3 4 5 6 7 8\n"), None);
}

/// Between the readings 1000 ticks passed and 750 of them were idle: three
/// of the four cores.
#[test]
fn idle_cores_are_the_idle_share_of_the_ticks_between_two_readings() {
    let before = stat(&stat_text(1000, 4000)).unwrap();
    let after = stat(&stat_text(1250, 4750)).unwrap();
    assert_eq!(idle(before, after), Some((3000, 4000)));
    assert_eq!(idle(before, before), None);
}

#[test]
fn the_machine_is_measured_only_when_both_files_say_what_is_read() {
    let (before, after) = (stat_text(1000, 4000), stat_text(1250, 4750));
    assert_eq!(
        available(&before, &after, MEMINFO),
        Ok(Available {
            memory_bytes: 8_192_000 * 1024,
            memory_total: 16_384_000 * 1024,
            idle_millicores: 3000,
            cores_millicores: 4000
        })
    );
    assert_eq!(
        available(&before, &before, MEMINFO),
        Err(Unmeasured::Unparsed("/proc/stat"))
    );
    assert_eq!(
        available(&before, &after, "MemTotal: 1 kB\n"),
        Err(Unmeasured::Unparsed("/proc/meminfo"))
    );
}

#[test]
fn a_trees_load_is_its_cpu_time_over_the_window() {
    let window = Duration::from_millis(500);
    assert_eq!(load(Duration::from_millis(750), window), 1500);
    assert_eq!(load(Duration::ZERO, window), 0);
}
