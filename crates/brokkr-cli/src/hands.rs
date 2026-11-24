//! `brokkr hands`: the box's server and its one-shot runner (decision
//! 0043). Its clap definition moved here from `lib.rs`, frozen at its
//! baseline, when both verbs gained the reach sealed at admission
//! (decision 0065 slice two, U8a2c).

use std::fmt;
use std::path::PathBuf;
use std::process::ExitCode;

use brokkr_protocol::broker::Refusal;
use brokkr_protocol::hands::{self, Base64UrlError, HandsSpec, Sealed, Session, SessionError};
use clap::Subcommand;

use crate::Exit;

const REACH_FLAG: &str = "--reach";
const REACH_BASE64URL_FLAG: &str = "--reach-base64url";
const SPEC_FLAG: &str = "--spec";
const SPEC_BASE64URL_FLAG: &str = "--spec-base64url";

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
        /// from (U8a2c). Pass either `--reach` or `--reach-base64url`.
        #[arg(long)]
        reach: Option<String>,
        /// The reach sealed at admission as unpadded base64url of its JSON
        /// bytes. Pass either `--reach` or `--reach-base64url`.
        #[arg(long = "reach-base64url")]
        reach_base64url: Option<String>,
        /// The box spec as JSON: {"kind":"workspace","network":…,"binds":[…]}.
        /// Pass either `--spec` or `--spec-base64url`.
        #[arg(long)]
        spec: Option<String>,
        /// The box spec as unpadded base64url of its JSON bytes.
        /// Pass either `--spec` or `--spec-base64url`.
        #[arg(long = "spec-base64url")]
        spec_base64url: Option<String>,
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

/// Why a `--reach` / `--reach-base64url` or `--spec` / `--spec-base64url`
/// argument was refused. Every refusal names which argument and why, and
/// never echoes an encoded argument's value.
#[derive(Debug)]
pub(crate) enum DocumentError {
    Missing {
        plain: &'static str,
        encoded: &'static str,
    },
    BothSpellings {
        plain: &'static str,
        encoded: &'static str,
    },
    Encoded {
        flag: &'static str,
        cause: Base64UrlError,
    },
    Reach {
        flag: &'static str,
        cause: serde_json::Error,
    },
    SpecJson {
        flag: &'static str,
        cause: serde_json::Error,
    },
    Spec {
        flag: &'static str,
        cause: String,
    },
}

impl fmt::Display for DocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing { plain, encoded } => {
                write!(f, "{plain}: missing; pass either {plain} or {encoded}")
            }
            Self::BothSpellings { plain, encoded } => {
                write!(
                    f,
                    "{plain}: both {plain} and {encoded} were given; pass only one"
                )
            }
            Self::Encoded { flag, cause } => write!(f, "{flag}: {cause}"),
            Self::Reach { flag, cause } => {
                write!(f, "{flag}: {}: {cause}", Refusal::Identity)
            }
            Self::SpecJson { flag, cause } => write!(f, "{flag}: {cause}"),
            Self::Spec { flag, cause } => write!(f, "{flag}: {cause}"),
        }
    }
}

impl std::error::Error for DocumentError {}

pub(crate) fn run(command: HandsCommand) -> anyhow::Result<ExitCode> {
    run_with(command, Session::create)
}

/// Resolve one document carried in either its plain JSON spelling or its
/// unpadded base64url spelling, refusing if both or neither were given.
fn resolve_document(
    plain: (&'static str, Option<String>),
    encoded: (&'static str, Option<String>),
) -> Result<(&'static str, String), DocumentError> {
    match (plain.1, encoded.1) {
        (Some(json), None) => Ok((plain.0, json)),
        (None, Some(raw)) => {
            let json = hands::decode_base64url(&raw).map_err(|cause| DocumentError::Encoded {
                flag: encoded.0,
                cause,
            })?;
            Ok((encoded.0, json))
        }
        (Some(_), Some(_)) => Err(DocumentError::BothSpellings {
            plain: plain.0,
            encoded: encoded.0,
        }),
        (None, None) => Err(DocumentError::Missing {
            plain: plain.0,
            encoded: encoded.0,
        }),
    }
}

/// The reach `--reach` or `--reach-base64url` carries, read as the type it
/// is: one that does not read refuses with MB3's identity cause, and no box
/// is built.
fn parse_reach(flag: &'static str, reach: &str) -> Result<Sealed, DocumentError> {
    serde_json::from_str(reach).map_err(|cause| DocumentError::Reach { flag, cause })
}

/// The box spec `--spec` or `--spec-base64url` carries, parsed through the
/// one `HandsSpec::parse` check.
fn parse_spec(flag: &'static str, spec: &str) -> Result<HandsSpec, DocumentError> {
    let raw: serde_json::Value =
        serde_json::from_str(spec).map_err(|cause| DocumentError::SpecJson { flag, cause })?;
    HandsSpec::parse(&raw).map_err(|cause| DocumentError::Spec { flag, cause })
}

fn resolve_serve_documents(
    reach: Option<String>,
    reach_base64url: Option<String>,
    spec: Option<String>,
    spec_base64url: Option<String>,
) -> Result<(HandsSpec, Sealed), DocumentError> {
    let (spec_flag, spec_json) =
        resolve_document((SPEC_FLAG, spec), (SPEC_BASE64URL_FLAG, spec_base64url))?;
    let spec = parse_spec(spec_flag, &spec_json)?;
    let (reach_flag, reach_json) =
        resolve_document((REACH_FLAG, reach), (REACH_BASE64URL_FLAG, reach_base64url))?;
    let sealed = parse_reach(reach_flag, &reach_json)?;
    Ok((spec, sealed))
}

/// `run`, with the server's session made by `session`: the seam a test
/// fails to pin what a refused session prints.
fn run_with(
    command: HandsCommand,
    session: impl FnOnce(&str) -> Result<Session, SessionError>,
) -> anyhow::Result<ExitCode> {
    match command {
        HandsCommand::Serve {
            workdir,
            artifacts,
            reach,
            reach_base64url,
            spec,
            spec_base64url,
        } => {
            let (spec, sealed) =
                resolve_serve_documents(reach, reach_base64url, spec, spec_base64url)?;
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
            let (spec, sealed) = (
                parse_spec(SPEC_FLAG, &spec)?,
                parse_reach(REACH_FLAG, &reach)?,
            );
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
