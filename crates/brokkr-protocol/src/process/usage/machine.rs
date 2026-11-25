//! Darwin's machine availability for admission (decision 0125, ask A).
//! Keep the kernel calls separate from the arithmetic exercised on Linux.

use std::fmt;

// Inline, not a file of its own: Linux compiles and covers this file, so
// the complexity gate reads these macOS-only functions as the rest of the
// platform-only code in files Linux compiles (1193c, #1369's CI).
#[cfg(target_os = "macos")]
mod macos {
    use std::ffi::CStr;
    use std::io::{self, ErrorKind};
    use std::mem::{offset_of, size_of, MaybeUninit};
    use std::ptr;

    use super::{available_bytes, pressure, ticks, MemoryPressure};

    extern "C" {
        // SAFETY: This is the libSystem declaration from mach/mach_port.h.
        // The task must own a send right named by the port.
        fn mach_port_deallocate(
            task: libc::mach_port_t,
            port: libc::mach_port_t,
        ) -> libc::kern_return_t;
    }

    /// Read using a host send right, returning that right to the kernel even
    /// when the statistics call refuses.
    #[expect(
        deprecated,
        reason = "decision 0125 uses the existing libc bindings without mach2"
    )]
    fn host(read: impl FnOnce(libc::host_t) -> libc::kern_return_t) -> libc::kern_return_t {
        // SAFETY: These calls take no input. mach_host_self gives this task a
        // send right that is valid through read and owned here to deallocate.
        unsafe {
            let port = libc::mach_host_self();
            let status = read(port);
            mach_port_deallocate(libc::mach_task_self(), port);
            status
        }
    }

    fn sysctl<const N: usize>(name: &CStr) -> io::Result<[u8; N]> {
        let mut bytes = [0; N];
        let mut length = N;
        // SAFETY: name is NUL terminated, bytes has length bytes, and the
        // null input pointer and zero input length request a read only.
        let result = unsafe {
            libc::sysctlbyname(
                name.as_ptr(),
                bytes.as_mut_ptr().cast(),
                &mut length,
                ptr::null_mut(),
                0,
            )
        };
        match (result, length) {
            (0, length) if length == N => Ok(bytes),
            (0, _) => Err(ErrorKind::InvalidData.into()),
            _ => Err(io::Error::last_os_error()),
        }
    }

    fn statistics(status: libc::kern_return_t, count: u32, minimum: u32) -> io::Result<()> {
        match status {
            libc::KERN_SUCCESS if count >= minimum => Ok(()),
            libc::KERN_SUCCESS => Err(ErrorKind::InvalidData.into()),
            status => Err(io::Error::other(format!("Mach kernel status {status}"))),
        }
    }

    /// Memory available now, the machine's total memory and the kernel's
    /// pressure level.
    pub struct Memory {
        pub available_bytes: u64,
        pub total_bytes: u64,
        pub pressure: MemoryPressure,
    }

    /// Read the VM statistics and the machine's memory and pressure sysctls.
    pub fn memory() -> io::Result<Memory> {
        let mut vm = MaybeUninit::<libc::vm_statistics64>::zeroed();
        let mut count = libc::HOST_VM_INFO64_COUNT;
        let status = host(|port| {
            // SAFETY: vm holds count Mach integers and count is writable.
            unsafe {
                libc::host_statistics64(
                    port,
                    libc::HOST_VM_INFO64,
                    vm.as_mut_ptr().cast(),
                    &mut count,
                )
            }
        });
        // Older kernels can return a shorter revision than libc declares.
        const PREFIX: usize =
            offset_of!(libc::vm_statistics64, speculative_count) + size_of::<u32>();
        statistics(
            status,
            count,
            (PREFIX / size_of::<libc::integer_t>()) as u32,
        )?;
        // SAFETY: The successful call initialized the prefix through
        // speculative_count. The struct was zeroed and all its fields are integers.
        let vm = unsafe { vm.assume_init() };
        // free_count already holds speculative_count (see `available_bytes`).
        let reclaimable = [vm.free_count, vm.inactive_count, vm.purgeable_count];
        Ok(Memory {
            available_bytes: available_bytes(reclaimable, rustix::param::page_size() as u64),
            total_bytes: u64::from_ne_bytes(sysctl(c"hw.memsize")?),
            pressure: pressure(u32::from_ne_bytes(sysctl(
                c"kern.memorystatus_vm_pressure_level",
            )?))?,
        })
    }

    /// One reading of the aggregate processor ticks and logical CPU count.
    pub struct Cpu {
        ticks: [u32; 4],
        cores: u32,
    }

    impl Cpu {
        /// Idle ticks, all ticks and logical cores over the sampling window.
        pub fn since(self, before: Self) -> (u64, u64, u64) {
            let (idle, total) = ticks(before.ticks, self.ticks, libc::CPU_STATE_IDLE as usize);
            (idle, total, u64::from(self.cores))
        }
    }

    /// Read the aggregate processor counters and `hw.logicalcpu`.
    pub fn cpu() -> io::Result<Cpu> {
        let mut cpu = MaybeUninit::<libc::host_cpu_load_info>::uninit();
        let mut count = libc::HOST_CPU_LOAD_INFO_COUNT;
        let status = host(|port| {
            // SAFETY: cpu holds count Mach integers and count is writable.
            unsafe {
                libc::host_statistics(
                    port,
                    libc::HOST_CPU_LOAD_INFO,
                    cpu.as_mut_ptr().cast(),
                    &mut count,
                )
            }
        });
        statistics(status, count, libc::HOST_CPU_LOAD_INFO_COUNT)?;
        // SAFETY: The kernel initialized all the ticks.
        let cpu = unsafe { cpu.assume_init() };
        Ok(Cpu {
            ticks: cpu.cpu_ticks,
            cores: u32::from_ne_bytes(sysctl(c"hw.logicalcpu")?),
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn kernel_vm_and_cpu_reads_have_the_expected_shape() {
            let memory = memory().unwrap();
            assert!(memory.total_bytes > 0);
            assert!(memory.available_bytes <= memory.total_bytes);
            assert!(matches!(
                memory.pressure,
                MemoryPressure::Normal | MemoryPressure::Warn | MemoryPressure::Critical
            ));
            let cpu = cpu().unwrap();
            assert!(cpu.cores > 0);
        }
    }
}
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

/// Free, inactive and purgeable pages, in bytes. The kernel's free count
/// already holds the speculative pages: `vm_stat` prints "Pages free" as
/// `free_count - speculative_count`. So adding them again would count
/// them twice, in the admitting direction, and ask A's speculative pages
/// are counted once, inside free (1193c's review, finding 7).
#[cfg(any(test, target_os = "macos"))]
fn available_bytes(reclaimable: [u32; 3], page_size: u64) -> u64 {
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
    fn reclaimable_pages_are_free_inactive_and_purgeable_with_speculative_inside_free() {
        assert_eq!(available_bytes([3, 5, 7], 16_384), 245_760);
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
