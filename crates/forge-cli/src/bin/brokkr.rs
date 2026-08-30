//! `brokkr` — the shipped binary (decision 0019 ruling 9). Nothing of
//! its own: the whole CLI is the library's entry point.

use std::process::ExitCode;

fn main() -> ExitCode {
    forge_cli::main()
}
