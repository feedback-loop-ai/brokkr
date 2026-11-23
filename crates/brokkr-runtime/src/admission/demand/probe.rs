//! What the machine has available now, and what each running run's
//! driver tree uses, measured inside one pass (decision 0125 ruling 1).
//! Linux reads `/proc/meminfo` once and `/proc/stat`'s `cpu` lines twice,
//! [`WINDOW`] apart, with one read of the process table at each end of
//! the window; nothing is carried from an earlier pass. Every other host
//! is unmeasured until 1193c (ruling 2).
//!
//! The parsing is pure, and the reads are this module's alone: the pass
//! takes them through [`super::Machine`]'s seam, which every test injects.

use std::io::ErrorKind;
use std::path::Path;
use std::time::Duration;

/// The span between the two CPU readings of one pass: long enough that a
/// tick count of 100 Hz says a share to a percent, short enough that a
/// listing waits on it unnoticed.
pub const WINDOW: Duration = Duration::from_millis(500);

/// A running run whose driver's tree a pass measures: the journal it is
/// in and its id, which name its run lock (decision 0108).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Driven<'a> {
    pub journal: &'a Path,
    pub run: &'a str,
}

/// What one pass measured: the machine, and each run it was asked about,
/// in the order asked, `None` where its driver's tree cannot be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Measurement {
    Measured {
        machine: Available,
        runs: Vec<Option<Use>>,
    },
    Unmeasured(Unmeasured),
}

/// What the machine has now, and the whole of it: memory in bytes, CPU in
/// thousandths of a core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Available {
    pub memory_bytes: u64,
    pub memory_total: u64,
    pub idle_millicores: u64,
    pub cores_millicores: u64,
}

/// What a driver's tree uses now: its resident memory, and the cores it
/// kept busy over the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Use {
    pub memory_bytes: u64,
    pub load_millicores: u64,
}

/// Why the machine is not measured, so admission keeps its count
/// ceilings (ruling 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Unmeasured {
    /// A host whose measures are not built: macOS, until 1193c.
    #[error("this host's availability is not measured yet (macOS is #1193's next slice)")]
    Platform,
    #[error("{file} cannot be read: {kind}")]
    Unreadable { file: &'static str, kind: ErrorKind },
    #[error("{0} does not say what admission reads of it")]
    Unparsed(&'static str),
}

const MEMINFO: &str = "/proc/meminfo";
const STAT: &str = "/proc/stat";

/// `MemAvailable` and `MemTotal` from a `/proc/meminfo`'s text, in bytes.
pub(super) fn meminfo(text: &str) -> Option<(u64, u64)> {
    let field = |name: &str| {
        let line = text.lines().find(|line| line.starts_with(name))?;
        let mut words = line[name.len()..].split_whitespace();
        let kib = words.next()?.parse::<u64>().ok()?;
        (words.next()? == "kB").then(|| kib.saturating_mul(1024))
    };
    Some((field("MemAvailable:")?, field("MemTotal:")?))
}

/// The aggregate ticks a `/proc/stat` reads: the idle ones (idle and
/// iowait), all of them (user to steal; guest time is inside user's), and
/// how many cores it lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Ticks {
    pub(super) idle: u64,
    pub(super) total: u64,
    pub(super) cores: u64,
}

pub(super) fn stat(text: &str) -> Option<Ticks> {
    let mut lines = text.lines();
    let first = lines.next()?.strip_prefix("cpu ")?;
    let ticks: Vec<u64> = first
        .split_whitespace()
        .take(8)
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    let [_, _, _, idle, iowait, _, _, _] = ticks.as_slice() else {
        return None;
    };
    let core = |line: &&str| {
        let rest = line.strip_prefix("cpu").unwrap_or("x");
        rest.starts_with(|digit: char| digit.is_ascii_digit())
    };
    let cores = lines.filter(core).count() as u64;
    (cores > 0).then(|| Ticks {
        idle: idle.saturating_add(*iowait),
        total: ticks.iter().fold(0, |sum, tick| sum.saturating_add(*tick)),
        cores,
    })
}

/// The idle cores between two readings, and the whole machine's, in
/// thousandths of a core; `None` where no tick passed between them.
pub(super) fn idle(before: Ticks, after: Ticks) -> Option<(u64, u64)> {
    let total = after.total.checked_sub(before.total).filter(|t| *t > 0)?;
    let idle = after.idle.checked_sub(before.idle)?.min(total);
    let cores = after.cores.saturating_mul(1000);
    let share = u128::from(idle) * u128::from(cores) / u128::from(total);
    Some((u64::try_from(share).unwrap_or(cores), cores))
}

/// The cores `cpu` kept busy over `wall`, in thousandths of a core.
pub(super) fn load(cpu: Duration, wall: Duration) -> u64 {
    let wall = wall.as_nanos().max(1);
    u64::try_from(cpu.as_nanos() * 1000 / wall).unwrap_or(u64::MAX)
}

/// Read `file` whole, or say why it cannot be.
fn read(file: &'static str) -> Result<String, Unmeasured> {
    std::fs::read_to_string(file).map_err(|error| Unmeasured::Unreadable {
        file,
        kind: error.kind(),
    })
}

/// Whether this machine can be measured, read once with no window, for
/// `brokkr doctor`.
pub fn measurable() -> Result<(), Unmeasured> {
    match cfg!(target_os = "linux") {
        true => {
            meminfo(&read(MEMINFO)?).ok_or(Unmeasured::Unparsed(MEMINFO))?;
            stat(&read(STAT)?).ok_or(Unmeasured::Unparsed(STAT))?;
            Ok(())
        }
        false => Err(Unmeasured::Platform),
    }
}

/// The production measure: this machine now, over [`WINDOW`], and each
/// of `runs`' drivers' trees, through the pid its run lock records.
pub fn measure_now(runs: &[Driven<'_>]) -> Measurement {
    match measurable() {
        Ok(()) => sampled(runs).unwrap_or_else(Measurement::Unmeasured),
        Err(why) => Measurement::Unmeasured(why),
    }
}

/// The two readings of one window, and what lies between them. A run
/// whose lock records no holder has no tree to read.
fn sampled(runs: &[Driven<'_>]) -> Result<Measurement, Unmeasured> {
    use brokkr_protocol::process::usage::trees_now;
    let pids: Vec<Option<u32>> = (runs.iter())
        .map(|run| crate::run_lock::recorded_at(run.journal, run.run).map(|held| held.pid))
        .collect();
    let roots: Vec<u32> = pids.iter().flatten().copied().collect();
    let started = std::time::Instant::now();
    let (before, trees) = (read(STAT)?, trees_now(&roots));
    std::thread::sleep(WINDOW);
    let (after, again) = (read(STAT)?, trees_now(&roots));
    let wall = started.elapsed();
    let machine = available(&before, &after, &read(MEMINFO)?)?;
    let each = |at: usize| {
        let first = trees.as_ref()?.get(at).copied().flatten()?;
        let last = again.as_ref()?.get(at).copied().flatten()?;
        Some(Use {
            memory_bytes: last.memory_bytes,
            load_millicores: load(last.cpu.saturating_sub(first.cpu), wall),
        })
    };
    let mut measured = (0..roots.len()).map(each);
    let runs = (pids.iter())
        .map(|pid| pid.and_then(|_| measured.next().flatten()))
        .collect();
    Ok(Measurement::Measured { machine, runs })
}

/// The machine as two `/proc/stat` texts and a `/proc/meminfo` text say
/// it is.
pub(super) fn available(
    before: &str,
    after: &str,
    meminfo_text: &str,
) -> Result<Available, Unmeasured> {
    let unparsed = Unmeasured::Unparsed(STAT);
    let ticks = stat(before).zip(stat(after)).ok_or(unparsed)?;
    let (idle_millicores, cores_millicores) = idle(ticks.0, ticks.1).ok_or(unparsed)?;
    let (memory_bytes, memory_total) =
        meminfo(meminfo_text).ok_or(Unmeasured::Unparsed(MEMINFO))?;
    Ok(Available {
        memory_bytes,
        memory_total,
        idle_millicores,
        cores_millicores,
    })
}

#[cfg(test)]
mod tests;
