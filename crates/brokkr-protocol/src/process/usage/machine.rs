//! Darwin's machine availability for admission (decision 0125, ask A).
//! Keep the kernel calls separate from the arithmetic exercised on Linux.

use std::fmt;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::{cpu, memory, Cpu, Memory};

/// The kernel's current memory-pressure level. Linux's availability
/// probe has no pressure veto and reports `Normal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPressure {
    Normal,
    Warn,
    Critical,
}

impl fmt::Display for MemoryPressure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Normal => "normal",
            Self::Warn => "warn",
            Self::Critical => "critical",
        })
    }
}

#[cfg(any(test, target_os = "macos"))]
fn pressure(level: u32) -> std::io::Result<MemoryPressure> {
    match level {
        1 => Ok(MemoryPressure::Normal),
        2 => Ok(MemoryPressure::Warn),
        4 => Ok(MemoryPressure::Critical),
        _ => Err(std::io::ErrorKind::InvalidData.into()),
    }
}

#[cfg(any(test, target_os = "macos"))]
fn available_bytes(reclaimable: [u32; 4], page_size: u64) -> u64 {
    reclaimable.into_iter().map(u64::from).sum::<u64>() * page_size
}

/// Darwin's counters are 32-bit and can wrap independently. Return idle
/// and total ticks over the window, in that order.
#[cfg(any(test, target_os = "macos"))]
fn ticks(before: [u32; 4], after: [u32; 4], idle: usize) -> (u64, u64) {
    let deltas = std::array::from_fn::<_, 4, _>(|i| u64::from(after[i].wrapping_sub(before[i])));
    (deltas[idle], deltas.into_iter().sum())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reclaimable_pages_include_all_four_kernel_counts() {
        assert_eq!(available_bytes([3, 5, 7, 11], 16_384), 425_984);
    }

    #[test]
    fn idle_and_total_ticks_count_independent_wraps() {
        assert_eq!(ticks([2, 8, u32::MAX - 2, 5], [4, 9, 5, 6], 2), (8, 12));
    }

    #[test]
    fn kernel_pressure_levels_have_stable_names_and_unknown_is_invalid() {
        for (raw, level, word) in [
            (1, MemoryPressure::Normal, "normal"),
            (2, MemoryPressure::Warn, "warn"),
            (4, MemoryPressure::Critical, "critical"),
        ] {
            assert_eq!(pressure(raw).unwrap(), level);
            assert_eq!(level.to_string(), word);
        }
        assert_eq!(
            pressure(3).unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );
    }
}
