//! `brokkr hands`: the box's server and its one-shot runner (decision
//! 0043). Its clap definition moved here from `lib.rs`, frozen at its
//! baseline, when both verbs gained the reach sealed at admission
//! (decision 0065 slice two, U8a2c).

use std::path::PathBuf;
use std::process::ExitCode;

use brokkr_protocol::broker::Refusal;
use brokkr_protocol::hands::{self, Sealed, Session, SessionError};
use clap::Subcommand;

use crate::Exit;

#[derive(Subcommand, Debug)]
pub enum HandsCommand {
    /// Serve the one `workspace` tool over MCP (newline-delimited JSON-RPC
    /// on stdio); every call runs `bash -lc <command>` inside the box.
    Serve {
        /// The worktree, bound read-write at its own path.
        #[arg(long)]
        workdir: PathBuf,
        /// The engine's artifact root, which the box never writes (U8a).
        #[arg(long)]
        artifacts: PathBuf,
        /// The reach sealed at admission as JSON, which every box is built
        /// from (U8a2c).
        #[arg(long)]
        reach: String,
        /// The box spec as JSON: {"kind":"workspace","network":…,"binds":[…]}.
        #[arg(long, default_value = "\"workspace\"")]
        spec: String,
    },
    /// Run one command whole inside the box with stdio passed through —
    /// how a deterministic `exec` seat holds a gate. Exits with the
    /// command's own code.
    Exec {
        /// The worktree, bound read-write at its own path.
        #[arg(long)]
        workdir: PathBuf,
        /// The engine's artifact root, as `serve` takes it.
        #[arg(long)]
        artifacts: PathBuf,
        /// The reach sealed at admission, as `serve` takes it.
        #[arg(long)]
        reach: String,
        /// Strategy root, bound read-only at /runtime/bundle.
        #[arg(long)]
        bundle_root: Option<PathBuf>,
        /// The box spec as JSON, as `serve` takes it.
        #[arg(long, default_value = "\"workspace\"")]
        spec: String,
        /// The command and its arguments, run inside the box.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
        command: Vec<String>,
    },
}

pub(crate) fn run(command: HandsCommand) -> anyhow::Result<ExitCode> {
    run_with(command, Session::create)
}

/// The reach `--reach` carries, read as the type it is: one that does not
/// read refuses with MB3's identity cause, and no box is built.
fn parse_reach(reach: &str) -> anyhow::Result<Sealed> {
    let read = serde_json::from_str(reach);
    read.map_err(|problem| anyhow::anyhow!("--reach: {}: {problem}", Refusal::Identity))
}

/// `run`, with the server's session made by `session`: the seam a test
/// fails to pin what a refused session prints.
fn run_with(
    command: HandsCommand,
    session: impl FnOnce(&str) -> Result<Session, SessionError>,
) -> anyhow::Result<ExitCode> {
    let parse_spec = |spec: &str| -> anyhow::Result<hands::HandsSpec> {
        let raw: serde_json::Value = serde_json::from_str(spec)?;
        hands::HandsSpec::parse(&raw).map_err(|problem| anyhow::anyhow!("--spec: {problem}"))
    };
    match command {
        HandsCommand::Serve {
            workdir,
            artifacts,
            reach,
            spec,
        } => {
            let (spec, sealed) = (parse_spec(&spec)?, parse_reach(&reach)?);
            // The session outlives every call: overlay upper layers live
            // here until the harness closes the server's stdin, or a
            // termination signal ends the server (#415).
            let session = session("serve")?;
            session.remove_on_termination(Exit::of_signal)?;
            let stdin = std::io::stdin();
            let (input, output) = (stdin.lock(), std::io::stdout());
            let path = session.path();
            let (work, root) = (&workdir, &artifacts);
            // Every call's box is built from the one reach it was served.
            let run: &hands::Executor = &move |spec, work, root, at, command, timeout| {
                hands::execute(spec, work, root, &sealed, at, command, timeout)
            };
            hands::serve(input, output, work, root, path, &spec, run)?;
            Ok(Exit::Completed.into())
        }
        HandsCommand::Exec {
            workdir,
            artifacts,
            reach,
            bundle_root,
            spec,
            command,
        } => {
            let (spec, sealed) = (parse_spec(&spec)?, parse_reach(&reach)?);
            // Only the leading separator is ours; a command may carry
            // its own `--`.
            let command: Vec<String> = match command.first().map(String::as_str) {
                Some("--") => command[1..].to_vec(),
                _ => command,
            };
            let bundle_root = bundle_root.as_deref();
            let ran = hands::run_boxed(&spec, &workdir, &artifacts, &sealed, bundle_root, &command);
            let code = ran.map_err(anyhow::Error::msg)?;
            Ok(Exit::of_box(code).into())
        }
    }
}

#[cfg(test)]
mod tests;
