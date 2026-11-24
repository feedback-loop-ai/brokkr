//! `brokkr hands`: the box's server and its one-shot runner (decision
//! 0043). Its clap definition moved here from `lib.rs`, frozen at its
//! baseline, when both verbs gained the reach sealed at admission
//! (decision 0065 slice two, U8a2c).

use std::fmt;
use std::path::PathBuf;
use std::process::ExitCode;

use brokkr_protocol::broker::Refusal;
use brokkr_protocol::hands::{
    self, Base64UrlError, HandsSpec, Sealed, Session, SessionError, SpecRefusal,
};
use clap::builder::ArgPredicate;
use clap::Subcommand;

use crate::Exit;

const REACH_FLAG: &str = "--reach";
const REACH_BASE64URL_FLAG: &str = "--reach-base64url";
const SPEC_FLAG: &str = "--spec";
const SPEC_BASE64URL_FLAG: &str = "--spec-base64url";
const DEFAULT_SPEC: &str = "\"workspace\"";

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
        #[arg(long = "reach-base64url", allow_hyphen_values = true)]
        reach_base64url: Option<String>,
        /// The box spec as JSON: {"kind":"workspace","network":…,"binds":[…]}.
        /// Pass either `--spec` or `--spec-base64url`; omitting both uses
        /// `"workspace"`.
        #[arg(
            long,
            default_value = DEFAULT_SPEC,
            default_value_if("spec_base64url", ArgPredicate::IsPresent, None)
        )]
        spec: Option<String>,
        /// The box spec as unpadded base64url of its JSON bytes.
        /// Pass either `--spec` or `--spec-base64url`.
        #[arg(long = "spec-base64url", allow_hyphen_values = true)]
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
        #[arg(long, default_value = DEFAULT_SPEC)]
        spec: String,
        /// The command and its arguments, run inside the box.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
        command: Vec<String>,
    },
}

/// Why a `--reach` / `--reach-base64url` or `--spec` / `--spec-base64url`
/// argument was refused. Plain JSON spellings keep main's diagnostics byte
/// for byte; encoded spellings name the argument and failure class and never
/// echo either the raw argument value or any decoded field, value, or serde
/// fragment.
#[derive(Debug)]
pub(crate) enum DocumentError {
    MissingReach,
    BothSpellings {
        plain: &'static str,
        encoded: &'static str,
    },
    Encoded {
        flag: &'static str,
        cause: Base64UrlError,
    },
    PlainReach {
        cause: serde_json::Error,
    },
    PlainSpecJson {
        cause: serde_json::Error,
    },
    PlainSpec {
        cause: SpecRefusal,
    },
    EncodedNotJson {
        flag: &'static str,
    },
    EncodedReach,
    EncodedSpec {
        cause: SpecRefusal,
    },
}

impl fmt::Display for DocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingReach => write!(
                f,
                "{REACH_FLAG}: missing; pass either {REACH_FLAG} or {REACH_BASE64URL_FLAG}"
            ),
            Self::BothSpellings { plain, encoded } => write!(
                f,
                "{plain}: both {plain} and {encoded} were given; pass only one"
            ),
            Self::Encoded { flag, cause } => write!(f, "{flag}: {cause}"),
            Self::PlainReach { cause } => {
                write!(f, "{REACH_FLAG}: {}: {cause}", Refusal::Identity)
            }
            Self::PlainSpecJson { cause } => write!(f, "{cause}"),
            Self::PlainSpec { cause } => write!(f, "{SPEC_FLAG}: {cause}"),
            Self::EncodedNotJson { flag } => {
                write!(f, "{flag}: decoded document is not valid JSON")
            }
            Self::EncodedReach => write!(
                f,
                "{REACH_BASE64URL_FLAG}: {}: decoded document is not a valid sealed reach",
                Refusal::Identity
            ),
            Self::EncodedSpec { cause } => write!(
                f,
                "{SPEC_BASE64URL_FLAG}: decoded document is not a valid hands spec ({})",
                cause.variant_name()
            ),
        }
    }
}

impl std::error::Error for DocumentError {}

pub(crate) fn run(command: HandsCommand) -> anyhow::Result<ExitCode> {
    run_with(command, Session::create)
}

/// The reach `--reach` carries as plain JSON: one that does not read refuses
/// with MB3's identity cause, and no box is built.
fn parse_reach_plain(reach: &str) -> Result<Sealed, DocumentError> {
    serde_json::from_str(reach).map_err(|cause| DocumentError::PlainReach { cause })
}

/// The reach `--reach-base64url` carries after decoding: distinguishes
/// non-JSON from a non-reach document and discloses nothing decoded.
fn parse_reach_encoded(reach: &str) -> Result<Sealed, DocumentError> {
    serde_json::from_str::<serde::de::IgnoredAny>(reach).map_err(|_| {
        DocumentError::EncodedNotJson {
            flag: REACH_BASE64URL_FLAG,
        }
    })?;
    serde_json::from_str(reach).map_err(|_| DocumentError::EncodedReach)
}

/// The box spec `--spec` carries as plain JSON, parsed through the one
/// `HandsSpec::checked` check with main's diagnostics byte for byte.
fn parse_spec_plain(spec: &str) -> Result<HandsSpec, DocumentError> {
    let raw: serde_json::Value =
        serde_json::from_str(spec).map_err(|cause| DocumentError::PlainSpecJson { cause })?;
    HandsSpec::checked(&raw).map_err(|cause| DocumentError::PlainSpec { cause })
}

/// The box spec `--spec-base64url` carries after decoding, parsed through the
/// same `HandsSpec::checked` check without disclosing decoded text.
fn parse_spec_encoded(spec: &str) -> Result<HandsSpec, DocumentError> {
    let raw: serde_json::Value =
        serde_json::from_str(spec).map_err(|_| DocumentError::EncodedNotJson {
            flag: SPEC_BASE64URL_FLAG,
        })?;
    HandsSpec::checked(&raw).map_err(|cause| DocumentError::EncodedSpec { cause })
}

fn resolve_serve_spec(
    spec: Option<String>,
    spec_base64url: Option<String>,
) -> Result<HandsSpec, DocumentError> {
    match (spec, spec_base64url) {
        (Some(_), Some(_)) => Err(DocumentError::BothSpellings {
            plain: SPEC_FLAG,
            encoded: SPEC_BASE64URL_FLAG,
        }),
        (plain, None) => parse_spec_plain(plain.as_deref().unwrap_or(DEFAULT_SPEC)),
        (None, Some(raw)) => {
            let json = hands::decode_base64url(&raw).map_err(|cause| DocumentError::Encoded {
                flag: SPEC_BASE64URL_FLAG,
                cause,
            })?;
            parse_spec_encoded(&json)
        }
    }
}

fn resolve_serve_reach(
    reach: Option<String>,
    reach_base64url: Option<String>,
) -> Result<Sealed, DocumentError> {
    match (reach, reach_base64url) {
        (Some(_), Some(_)) => Err(DocumentError::BothSpellings {
            plain: REACH_FLAG,
            encoded: REACH_BASE64URL_FLAG,
        }),
        (None, None) => Err(DocumentError::MissingReach),
        (Some(json), None) => parse_reach_plain(&json),
        (None, Some(raw)) => {
            let json = hands::decode_base64url(&raw).map_err(|cause| DocumentError::Encoded {
                flag: REACH_BASE64URL_FLAG,
                cause,
            })?;
            parse_reach_encoded(&json)
        }
    }
}

fn resolve_serve_documents(
    reach: Option<String>,
    reach_base64url: Option<String>,
    spec: Option<String>,
    spec_base64url: Option<String>,
) -> Result<(HandsSpec, Sealed), DocumentError> {
    let spec = resolve_serve_spec(spec, spec_base64url)?;
    let sealed = resolve_serve_reach(reach, reach_base64url)?;
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
            let (spec, sealed) = (parse_spec_plain(&spec)?, parse_reach_plain(&reach)?);
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
