use super::*;
#[cfg(target_os = "linux")]
use crate::process::table::parse_stat;
use crate::process::table::{parse_ps, Identity};

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

pub(super) fn sample(rss: u64, cpu_ms: u64) -> Sample {
    Sample {
        rss,
        cpu: ms(cpu_ms),
    }
}

/// A row of `pid` under `ppid`, its usage `usage`.
pub(super) fn entry(pid: i32, ppid: i32, usage: Option<Sample>) -> Entry {
    Entry {
        id: Identity {
            pid,
            start: pid.to_string(),
        },
        ppid,
        pgid: pid,
        zombie: false,
        exiting: false,
        usage,
    }
}

/// A real `/proc/<pid>/stat` row's shape, its command name holding a
/// space and a `)`: utime 150, stime 30, cutime 12, cstime 8, rss 16384.
#[cfg(target_os = "linux")]
const STAT: &str = "4242 (cargo b) uild) S 4200 4242 4242 34816 4242 4194304 2000 0 0 0 \
                    150 30 12 8 20 0 3 0 987654 1234567890 16384 18446744073709551615 \
                    1 1 0 0 0 0 0 0 0 0 0 17 3 0 0 0 0 0";

#[cfg(target_os = "linux")]
#[test]
fn a_stat_row_s_usage_counts_its_waited_children_in_its_units() {
    let (_, rest) = STAT.rsplit_once(')').unwrap();
    let fields: Vec<&str> = rest.split_whitespace().collect();
    let units = Units {
        page: 4096,
        hz: 100,
    };
    assert_eq!(
        from_stat(&fields, units),
        Some(Sample {
            rss: 16384 * 4096,
            cpu: ms(2000),
        })
    );
    let mut unread = fields.clone();
    unread[21] = "-";
    assert_eq!(from_stat(&unread, units), None);
    assert_eq!(from_stat(&fields[..21], units), None);
    // The table's own read takes the host's units from `sysconf`.
    let host = Units::host();
    let read = parse_stat(4242, STAT).unwrap();
    assert_eq!(
        read.usage,
        Some(Sample {
            rss: 16384 * host.page,
            cpu: Duration::from_nanos(200 * 1_000_000_000 / host.hz),
        })
    );
}

#[test]
fn a_ps_row_s_usage_reads_kib_and_its_summed_time() {
    assert_eq!(from_ps("65536", "1:02.50"), Some(sample(64 << 20, 62_500)));
    assert_eq!(from_ps("1", "0:00.07"), Some(sample(1024, 70)));
    assert_eq!(
        from_ps("2", "2-03:04:05.06"),
        Some(sample(2048, ((2 * 24 + 3) * 3600 + 4 * 60 + 5) * 1000 + 60))
    );
    assert_eq!(from_ps("2", "5"), Some(sample(2048, 5000)));
    for (rss, time) in [
        ("-", "0:01.00"),
        ("1", "x-0:01.00"),
        ("1", "0:01.x"),
        ("1", "0:x.00"),
        ("1", "1:2:3:4.00"),
    ] {
        assert_eq!(from_ps(rss, time), None, "{rss} {time}");
    }
}

#[test]
fn a_planted_ps_s_row_carries_its_usage_after_the_stamp() {
    let read = parse_ps(" 7  1  7 S  Mon Sep  8 10:00:00 2026  65536   0:01.25").unwrap();
    assert_eq!(read.id.start, "Mon Sep 8 10:00:00 2026");
    assert_eq!(read.usage, Some(sample(64 << 20, 1250)));
    let unread = parse_ps("7 1 7 S Mon Sep 8 10:00:00 2026 - -");
    assert_eq!(unread.unwrap().usage, None);
    let short = parse_ps("7 1 7 S Mon Sep 8 10:00:00 2026 65536");
    assert_eq!(short, None, "a row with half its usage is not read whole");
}

#[test]
fn the_tree_sums_the_root_and_its_descendants_and_nothing_else() {
    let rows = [
        entry(10, 1, Some(sample(100, 10))),
        entry(11, 10, Some(sample(20, 5))),
        entry(12, 11, Some(sample(3, 1))),
        entry(13, 11, None),
        entry(14, 1, Some(sample(7_000, 7_000))),
    ];
    assert_eq!(
        tree(&rows, 10).map(|(usage, _)| usage),
        Some(sample(123, 16))
    );
    assert_eq!(tree(&rows, 11).map(|(usage, _)| usage), Some(sample(23, 6)));
    assert_eq!(tree(&rows, 99), None);
}

#[test]
fn peak_and_mean_memory_are_over_every_tick() {
    let mut meter = Meter::new();
    for (at, rss) in [(100, 10), (200, 40), (300, 10)] {
        meter.sample(ms(at), sample(rss, 0));
    }
    let usage = meter.usage(ms(400)).unwrap();
    assert_eq!((usage.peak_memory_bytes, usage.mean_memory_bytes), (40, 20));
}

/// Half a core for a second, two cores for the next, then idle: the peak
/// is the busiest one-second window, the mean the CPU time over the wall.
#[test]
fn the_peak_load_is_the_busiest_second_of_ticks() {
    let mut meter = Meter::new();
    let mut cpu = 0;
    for tick in 1..=30 {
        cpu += match tick {
            1..=10 => 50,
            11..=20 => 200,
            _ => 0,
        };
        meter.sample(ms(tick * 100), sample(1, cpu));
    }
    assert_eq!(
        meter.usage(ms(3000)),
        Some(AttemptUsage {
            peak_memory_bytes: 1,
            mean_memory_bytes: 1,
            peak_load_millicores: 2000,
            mean_load_millicores: 833,
            cpu_ms: 2500,
            wall_ms: 3000,
        })
    );
}

/// An attempt whose ticks span less than a second has one window, from
/// its spawn; a CPU time that dips is held at its highest.
#[test]
fn a_short_attempt_s_peak_is_its_load_since_spawn() {
    let mut meter = Meter::new();
    meter.sample(ms(100), sample(1, 50));
    meter.sample(ms(200), sample(1, 150));
    meter.sample(ms(300), sample(1, 90));
    let usage = meter.usage(ms(400)).unwrap();
    assert_eq!(
        (
            usage.peak_load_millicores,
            usage.mean_load_millicores,
            usage.cpu_ms
        ),
        (500, 375, 150)
    );
    assert_eq!(meter.usage(Duration::ZERO).unwrap().mean_load_millicores, 0);
}

#[test]
fn no_tick_is_no_usage() {
    let mut meter = Meter::new();
    assert_eq!(meter.usage(ms(400)), None);
    meter.tick(ms(100), &[entry(5, 1, Some(sample(1, 1)))], 6);
    assert_eq!(
        meter.usage(ms(400)),
        None,
        "a tick without the root is skipped"
    );
}

/// A child that ran between two ticks and exited is still in the tree's
/// CPU time: its parent waited for it, and `cutime` carries it.
#[cfg(target_os = "linux")]
#[test]
fn an_exited_child_s_cpu_arrives_through_its_parent_s_cutime() {
    let row = |pid: i32, ppid: i32, utime: u64, cutime: u64| {
        let stat = format!(
            "{pid} (rustc) S {ppid} {pid} {pid} 0 -1 0 0 0 0 0 {utime} 0 {cutime} 0 \
             20 0 1 0 {pid}00 0 10"
        );
        parse_stat(pid, &stat).unwrap()
    };
    let mut meter = Meter::new();
    meter.tick(ms(100), &[row(10, 1, 10, 0), row(11, 10, 50, 0)], 10);
    meter.tick(ms(200), &[row(10, 1, 10, 80)], 10);
    let hz = Units::host().hz;
    assert_eq!(meter.usage(ms(300)).unwrap().cpu_ms, 90 * 1000 / hz);
}

#[cfg(target_os = "linux")]
fn report(script: &str) -> crate::AttemptReport {
    crate::process::DriverProcess::spawn(
        &crate::process::tests::command(script),
        std::path::Path::new("."),
        None,
        &crate::process::SpawnEnv::Inherit,
    )
    .unwrap()
    .run_attempt(
        "test",
        "effect",
        "attempt",
        "seat",
        serde_json::json!({}),
        |_| {},
    )
}

/// A real tree on Linux: the shell holds 64 MiB it read and touched while
/// it sleeps. A lower bound only: a loaded host may tick late, never less.
#[cfg(target_os = "linux")]
#[test]
fn a_real_tree_s_peak_memory_counts_what_its_child_touched() {
    let usage = report("x=$(head -c 67108864 /dev/zero | tr '\\0' a); sleep 1; echo ${#x} >&2")
        .usage
        .unwrap();
    assert!(usage.peak_memory_bytes >= 64 << 20, "{usage:?}");
}

/// A real tree on Linux: two children that hash a gigabyte and exit
/// before the driver does. A lower bound only.
#[cfg(target_os = "linux")]
#[test]
fn a_real_tree_s_cpu_time_counts_its_spinning_children() {
    let usage = report("head -c 1000000000 /dev/zero | md5sum >/dev/null")
        .usage
        .unwrap();
    assert!(usage.cpu_ms >= 500, "{usage:?}");
}
