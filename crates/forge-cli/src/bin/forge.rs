//! `forge` — the old name, kept as a shim for one release (decision 0019
//! ruling 9). Identical behavior; one plain line to stderr first, so a
//! piped stdout and every JSON consumer read exactly what `brokkr`
//! writes.

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("{}", forge_cli::SHIM_NOTICE);
    forge_cli::main()
}
