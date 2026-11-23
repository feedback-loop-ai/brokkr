//! What each attempt's process tree used, as the engine measures it on
//! its tracker's own tick (decision 0112 rulings 1 and 2): the summed
//! resident memory of the driver's subtree, and its CPU time, read from
//! the rows the tick already reads. It is measurement only. Nothing here
//! reaches `observe`, `record`, `recorded`, `doubted`, a kill or the
//! cleanup proof: the meter only reads rows, and only the attempt's
//! report carries what it found.
//!
//! Both memory figures are upper estimates: a page two processes share is
//! counted once per process. CPU time counts the time of exited children
//! their parent waited for, so the thousands of short-lived `rustc` and
//! build-script processes of a cargo build, each born and gone between two
//! ticks, are not lost.

use std::collections::{BTreeSet, VecDeque};
use std::time::Duration;

use super::table::Entry;
use crate::AttemptUsage;

mod now;
pub use now::{trees_now, TreeNow};

/// The span a peak load is read over: the most cores the tree kept busy
/// across any second of its ticks.
const WINDOW: Duration = Duration::from_secs(1);

/// One process's resident memory and CPU time, as one table row reads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Sample {
    pub(super) rss: u64,
    pub(super) cpu: Duration,
}

/// What a `/proc/<pid>/stat` row's numbers are counted in: the page a
/// resident set is counted in, and the clock ticks a second holds. Read
/// from the host's `sysconf`, never assumed.
#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy)]
pub(super) struct Units {
    pub(super) page: u64,
    pub(super) hz: u64,
}

#[cfg(target_os = "linux")]
impl Units {
    /// This host's page size and clock ticks per second (`sysconf`'s
    /// `_SC_PAGESIZE` and `_SC_CLK_TCK`).
    pub(super) fn host() -> Units {
        Units {
            page: rustix::param::page_size() as u64,
            hz: rustix::param::clock_ticks_per_second(),
        }
    }
}

/// A `/proc/<pid>/stat` row's usage, from its `fields` counted after the
/// command name's closing parenthesis, so field N of proc(5) is index
/// N - 3: `utime` (14), `stime` (15), `cutime` (16) and `cstime` (17), in
/// clock ticks, and `rss` (24), in pages. `cutime` and `cstime` are the
/// time of the children the process waited for, which is how an exited
/// child's CPU stays in the tree. `None` when a field does not read.
#[cfg(target_os = "linux")]
pub(super) fn from_stat(fields: &[&str], units: Units) -> Option<Sample> {
    let number = |index: usize| fields.get(index)?.parse::<u64>().ok();
    let ticks = [11, 12, 13, 14]
        .into_iter()
        .map(number)
        .sum::<Option<u64>>()?;
    let rss = number(21)?;
    Some(Sample {
        rss: rss.saturating_mul(units.page),
        cpu: Duration::from_nanos(scaled(ticks, 1_000_000_000, units.hz)),
    })
}

/// A BSD `ps` row's usage, from its `rss` column, in KiB, and its `time`
/// column, `[dd-][hh:]mm:ss.cc`. The engine runs `ps` with `-S`, which
/// BSD ps(1) defines as summing the time of exited children into their
/// parent, so that time stays in the tree as `cutime` keeps it on Linux.
/// `None` when either does not read.
#[cfg(any(test, not(target_os = "linux")))]
pub(super) fn from_ps(rss: &str, time: &str) -> Option<Sample> {
    let rss = rss.parse::<u64>().ok()?;
    let (days, clock) = time.split_once('-').unwrap_or(("0", time));
    let days = days.parse::<u64>().ok()?;
    let (whole, hundredths) = clock.split_once('.').unwrap_or((clock, "0"));
    let hundredths = hundredths.parse::<u64>().ok()?;
    let parts: Vec<u64> = whole
        .split(':')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    let seconds = parts.iter().fold(0, |sum, part| sum * 60 + part) + days * 86_400;
    (1..=3).contains(&parts.len()).then(|| Sample {
        rss: rss.saturating_mul(1024),
        cpu: Duration::from_secs(seconds) + Duration::from_millis(hundredths * 10),
    })
}

/// `value × by / per`, without overflow, saturating at `u64::MAX`.
fn scaled(value: u64, by: u64, per: u64) -> u64 {
    let scaled = u128::from(value) * u128::from(by) / u128::from(per.max(1));
    u64::try_from(scaled).unwrap_or(u64::MAX)
}

/// What `entries` shows of the tree rooted at `root`: the summed sample of
/// the root and every descendant by parent. A row whose usage did not read
/// adds nothing. `None` when the root is not among them, and the tick is
/// skipped.
pub(super) fn tree(entries: &[Entry], root: i32) -> Option<Sample> {
    let mut pids = BTreeSet::from([root]);
    let mut sum = entries
        .iter()
        .find(|entry| entry.id.pid == root)?
        .usage
        .unwrap_or_default();
    loop {
        let found: Vec<&Entry> = entries
            .iter()
            .filter(|entry| pids.contains(&entry.ppid) && !pids.contains(&entry.id.pid))
            .collect();
        if found.is_empty() {
            return Some(sum);
        }
        for entry in found {
            pids.insert(entry.id.pid);
            let usage = entry.usage.unwrap_or_default();
            sum.rss = sum.rss.saturating_add(usage.rss);
            sum.cpu = sum.cpu.saturating_add(usage.cpu);
        }
    }
}

/// One attempt's measure across its ticks: the peak and the running sum
/// of its tree's memory, and its CPU time against the time since spawn,
/// for the peak load.
#[derive(Debug)]
pub(super) struct Meter {
    ticks: u64,
    rss_sum: u128,
    peak_rss: u64,
    /// The highest CPU time a tick summed. A time that leaves the tree,
    /// a child no parent of it waited for, reads as a dip, so the tree's
    /// CPU time is the highest sum, never the last.
    cpu: Duration,
    /// The ticks a peak window may still start at, from the spawn itself
    /// at no CPU time: the front is the newest at least `WINDOW` before
    /// the latest tick, once one is.
    window: VecDeque<(Duration, Duration)>,
    /// The highest load any window of at least `WINDOW` showed.
    peak: Option<u64>,
    /// The load since spawn, the peak of an attempt whose ticks span less
    /// than `WINDOW`.
    since_spawn: u64,
}

impl Meter {
    pub(super) fn new() -> Meter {
        Meter {
            ticks: 0,
            rss_sum: 0,
            peak_rss: 0,
            cpu: Duration::ZERO,
            window: VecDeque::from([(Duration::ZERO, Duration::ZERO)]),
            peak: None,
            since_spawn: 0,
        }
    }

    /// One tick `at` its time since spawn: the tree under `root` that
    /// `entries` shows, when it shows the root.
    pub(super) fn tick(&mut self, at: Duration, entries: &[Entry], root: i32) {
        if let Some(sample) = tree(entries, root) {
            self.sample(at, sample);
        }
    }

    /// One tick's summed `sample`, `at` its time since spawn.
    pub(super) fn sample(&mut self, at: Duration, sample: Sample) {
        self.ticks += 1;
        self.rss_sum += u128::from(sample.rss);
        self.peak_rss = self.peak_rss.max(sample.rss);
        self.cpu = self.cpu.max(sample.cpu);
        self.window.push_back((at, self.cpu));
        while self
            .window
            .get(1)
            .is_some_and(|(start, _)| at.saturating_sub(*start) >= WINDOW)
        {
            self.window.pop_front();
        }
        let (start, cpu) = self.window[0];
        let load = millicores(self.cpu.saturating_sub(cpu), at.saturating_sub(start));
        if at.saturating_sub(start) >= WINDOW {
            self.peak = Some(self.peak.map_or(load, |peak| peak.max(load)));
        } else {
            self.since_spawn = load;
        }
    }

    /// What the ticks measured, over the `wall` time from spawn to
    /// settlement; `None` when no tick read the tree.
    pub(super) fn usage(&self, wall: Duration) -> Option<AttemptUsage> {
        let mean = self.rss_sum.checked_div(u128::from(self.ticks))?;
        Some(AttemptUsage {
            peak_memory_bytes: self.peak_rss,
            mean_memory_bytes: u64::try_from(mean).unwrap_or(u64::MAX),
            peak_load_millicores: self.peak.unwrap_or(self.since_spawn),
            mean_load_millicores: millicores(self.cpu, wall),
            cpu_ms: millis(self.cpu),
            wall_ms: millis(wall),
        })
    }
}

/// `cpu` over `wall`, in thousandths of a core; none over no time.
fn millicores(cpu: Duration, wall: Duration) -> u64 {
    let wall = u64::try_from(wall.as_nanos()).unwrap_or(u64::MAX);
    let cpu = u64::try_from(cpu.as_nanos()).unwrap_or(u64::MAX);
    if wall == 0 {
        return 0;
    }
    scaled(cpu, 1000, wall)
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests;
