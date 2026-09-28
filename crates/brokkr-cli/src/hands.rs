//! `brokkr hands`: the box's server and its one-shot runner (decision
//! 0043).

use std::process::ExitCode;

use brokkr_protocol::hands::{self, Session, SessionError};

use crate::{Exit, HandsCommand};

pub(crate) fn run(command: HandsCommand) -> anyhow::Result<ExitCode> {
    run_with(command, Session::create)
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
        HandsCommand::Serve { workdir, spec } => {
            let spec = parse_spec(&spec)?;
            // The session outlives every call: overlay upper layers live
            // here until the harness closes the server's stdin, or a
            // termination signal ends the server (#415).
            let session = session("serve")?;
            session.remove_on_termination()?;
            let stdin = std::io::stdin();
            let (input, output) = (stdin.lock(), std::io::stdout());
            let path = session.path();
            hands::serve(input, output, &workdir, path, &spec, &hands::execute)?;
            Ok(Exit::Completed.into())
        }
        HandsCommand::Exec {
            workdir,
            bundle_root,
            spec,
            command,
        } => {
            let spec = parse_spec(&spec)?;
            // Only the leading separator is ours; a command may carry
            // its own `--`.
            let command: Vec<String> = match command.first().map(String::as_str) {
                Some("--") => command[1..].to_vec(),
                _ => command,
            };
            let code = hands::run_boxed(&spec, &workdir, bundle_root.as_deref(), &command)
                .map_err(anyhow::Error::msg)?;
            Ok(Exit::Boxed(u8::try_from(code.clamp(0, 255)).unwrap_or(1)).into())
        }
    }
}

#[cfg(test)]
mod tests;
