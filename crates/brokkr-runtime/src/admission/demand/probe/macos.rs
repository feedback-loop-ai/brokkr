use std::io;

use brokkr_protocol::process::usage::{self, Cpu, Memory, MemoryPressure};

use super::{idle, Available, Ticks, Unmeasured};

const CPU: &str = "HOST_CPU_LOAD_INFO and hw.logicalcpu";
const MEMORY: &str = "HOST_VM_INFO64, hw.memsize and kern.memorystatus_vm_pressure_level";

fn unreadable(file: &'static str, error: io::Error) -> Unmeasured {
    Unmeasured::Unreadable {
        file,
        kind: error.kind(),
    }
}

fn memory() -> Result<Memory, Unmeasured> {
    usage::memory().map_err(|error| unreadable(MEMORY, error))
}

pub(super) fn cpu() -> Result<Cpu, Unmeasured> {
    usage::cpu().map_err(|error| unreadable(CPU, error))
}

pub(super) fn measurable() -> Result<(), Unmeasured> {
    memory()?;
    cpu()?;
    Ok(())
}

pub(super) fn machine(before: Cpu, after: Cpu) -> Result<(Available, MemoryPressure), Unmeasured> {
    let (idle_ticks, total, cores) = after.since(before);
    let zero = Ticks {
        idle: 0,
        total: 0,
        cores,
    };
    let delta = Ticks {
        idle: idle_ticks,
        total,
        cores,
    };
    let (idle_millicores, cores_millicores) = idle(zero, delta).ok_or(Unmeasured::Unparsed(CPU))?;
    let memory = memory()?;
    Ok((
        Available {
            memory_bytes: memory.available_bytes,
            memory_total: memory.total_bytes,
            idle_millicores,
            cores_millicores,
        },
        memory.pressure,
    ))
}
