//! `brokkr hands`: the box's server and its one-shot runner (decision
//! 0043). Its clap definition moved here from `lib.rs`, frozen at its
//! baseline, when both verbs gained the reach sealed at admission
//! (decision 0065 slice two, U8a2c).

use std::fmt;
use std::path::PathBuf;
use std::process::ExitCode;

use brokkr_protocol::broker::Refusal;
use brokkr_protocol::hands::{
    self, Base64UrlError, BindRefusal, HandsSpec, Sealed, Session, SessionError, SpecRefusal,
    REACH_BASE64URL_OPTION, REACH_OPTION, SPEC_BASE64URL_OPTION, SPEC_OPTION,
};
use clap::builder::ArgPredicate;
use clap::Subcommand;

use crate::Exit;

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
        #[arg(long = REACH_OPTION, required_unless_present = "reach_base64url")]
        reach: Option<String>,
        /// The reach sealed at admission as unpadded base64url of its JSON
        /// bytes. Pass either `--reach` or `--reach-base64url`.
        #[arg(long = REACH_BASE64URL_OPTION, allow_hyphen_values = true)]
        reach_base64url: Option<String>,
        /// The box spec as JSON: {"kind":"workspace","network":…,"binds":[…]}.
        /// Pass either `--spec` or `--spec-base64url`; omitting both uses
        /// `"workspace"`.
        #[arg(
            long = SPEC_OPTION,
            default_value = DEFAULT_SPEC,
            default_value_if("spec_base64url", ArgPredicate::IsPresent, None)
        )]
        spec: Option<String>,
        /// The box spec as unpadded base64url of its JSON bytes.
        /// Pass either `--spec` or `--spec-base64url`.
        #[arg(long = SPEC_BASE64URL_OPTION, allow_hyphen_values = true)]
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
        #[arg(long = REACH_OPTION)]
        reach: String,
        /// Strategy root, bound read-only at /runtime/bundle.
        #[arg(long)]
        bundle_root: Option<PathBuf>,
        /// The box spec as JSON, as `serve` takes it.
        #[arg(long = SPEC_OPTION, default_value = DEFAULT_SPEC)]
        spec: String,
        /// The command and its arguments, run inside the box.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
        command: Vec<String>,
    },
}

/// Payload-free variant of [`BindRefusal`]: holds no key or path taken from a
/// decoded `--spec-base64url` document in either `Display` or `Debug` (#1277).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BindRefusalKind {
    NotObject,
    UnknownKey,
    NoPath,
    Mode,
    MaskNotArray,
    MaskName,
}

impl From<BindRefusal> for BindRefusalKind {
    fn from(refusal: BindRefusal) -> Self {
        match refusal {
            BindRefusal::NotObject => Self::NotObject,
            BindRefusal::UnknownKey { .. } => Self::UnknownKey,
            BindRefusal::NoPath => Self::NoPath,
            BindRefusal::Mode { .. } => Self::Mode,
            BindRefusal::MaskNotArray { .. } => Self::MaskNotArray,
            BindRefusal::MaskName { .. } => Self::MaskName,
        }
    }
}

impl fmt::Display for BindRefusalKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotObject => "Bind::NotObject",
            Self::UnknownKey => "Bind::UnknownKey",
            Self::NoPath => "Bind::NoPath",
            Self::Mode => "Bind::Mode",
            Self::MaskNotArray => "Bind::MaskNotArray",
            Self::MaskName => "Bind::MaskName",
        })
    }
}

/// Payload-free variant of [`SpecRefusal`] carried by
/// [`DocumentError::EncodedSpec`]: drops every field name, value and fragment
/// taken from a decoded `--spec-base64url` document so neither `Display` nor
/// `Debug` retains decoded text (#1277).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpecRefusalKind {
    UnknownKind,
    NotKindOrObject,
    Boundary,
    UnknownKey,
    Kind,
    Network,
    BindsNotArray,
    Bind(BindRefusalKind),
}

impl From<SpecRefusal> for SpecRefusalKind {
    fn from(refusal: SpecRefusal) -> Self {
        match refusal {
            SpecRefusal::UnknownKind { .. } => Self::UnknownKind,
            SpecRefusal::NotKindOrObject { .. } => Self::NotKindOrObject,
            SpecRefusal::Boundary => Self::Boundary,
            SpecRefusal::UnknownKey { .. } => Self::UnknownKey,
            SpecRefusal::Kind => Self::Kind,
            SpecRefusal::Network { .. } => Self::Network,
            SpecRefusal::BindsNotArray => Self::BindsNotArray,
            SpecRefusal::Bind(bind) => Self::Bind(bind.into()),
        }
    }
}

impl fmt::Display for SpecRefusalKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownKind => f.write_str("UnknownKind"),
            Self::NotKindOrObject => f.write_str("NotKindOrObject"),
            Self::Boundary => f.write_str("Boundary"),
            Self::UnknownKey => f.write_str("UnknownKey"),
            Self::Kind => f.write_str("Kind"),
            Self::Network => f.write_str("Network"),
            Self::BindsNotArray => f.write_str("BindsNotArray"),
            Self::Bind(bind) => write!(f, "{bind}"),
        }
    }
}

/// Why a `--reach` / `--reach-base64url` or `--spec` / `--spec-base64url`
/// argument was refused. Plain JSON spellings keep main's diagnostics byte
/// for byte; encoded spellings name the argument and failure class and hold
/// neither the raw argument value nor any decoded field, value, or serde
/// fragment in `Display` or `Debug`.
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
    EncodedReach {
        category: serde_json::error::Category,
    },
    EncodedSpecJson {
        category: serde_json::error::Category,
    },
    EncodedSpec {
        cause: SpecRefusalKind,
    },
}

fn json_category_name(category: serde_json::error::Category) -> &'static str {
    match category {
        serde_json::error::Category::Io => "Io",
        serde_json::error::Category::Syntax => "Syntax",
        serde_json::error::Category::Data => "Data",
        serde_json::error::Category::Eof => "Eof",
    }
}

impl fmt::Display for DocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingReach => write!(
                f,
                "--{REACH_OPTION}: missing; pass either --{REACH_OPTION} or --{REACH_BASE64URL_OPTION}"
            ),
            Self::BothSpellings { plain, encoded } => write!(
                f,
                "--{plain}: both --{plain} and --{encoded} were given; pass only one"
            ),
            Self::Encoded { flag, cause } => write!(f, "--{flag}: {cause}"),
            Self::PlainReach { cause } => {
                write!(f, "--{REACH_OPTION}: {}: {cause}", Refusal::Identity)
            }
            Self::PlainSpecJson { cause } => write!(f, "{cause}"),
            Self::PlainSpec { cause } => write!(f, "--{SPEC_OPTION}: {cause}"),
            Self::EncodedReach { category } => write!(
                f,
                "--{REACH_BASE64URL_OPTION}: {}: decoded document is not a valid sealed reach ({})",
                Refusal::Identity,
                json_category_name(*category),
            ),
            Self::EncodedSpecJson { category } => write!(
                f,
                "--{SPEC_BASE64URL_OPTION}: decoded document is not valid JSON ({})",
                json_category_name(*category),
            ),
            Self::EncodedSpec { cause } => write!(
                f,
                "--{SPEC_BASE64URL_OPTION}: decoded document is not a valid hands spec ({cause})"
            ),
        }
    }
}

impl std::error::Error for DocumentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::PlainReach { cause } | Self::PlainSpecJson { cause } => Some(cause),
            Self::PlainSpec { cause } => Some(cause),
            Self::MissingReach
            | Self::BothSpellings { .. }
            | Self::Encoded { .. }
            | Self::EncodedReach { .. }
            | Self::EncodedSpecJson { .. }
            | Self::EncodedSpec { .. } => None,
        }
    }
}

/// Whether `argv` invokes `hands serve` with neither `reach` nor
/// `reach_base64url` supplied, checked from clap's structured `ArgMatches`
/// state rather than rendered diagnostic text (#1277 V3, proposed decision
/// 0125 ruling 2).
fn is_missing_serve_reach(argv: &[std::ffi::OsString], command: &clap::Command) -> bool {
    let matches = command
        .clone()
        .ignore_errors(true)
        .mut_subcommand("hands", |hands| {
            hands
                .ignore_errors(true)
                .mut_subcommand("serve", |serve| serve.ignore_errors(true))
        })
        .try_get_matches_from(argv);
    matches.ok().is_some_and(|matches| {
        matches
            .subcommand_matches("hands")
            .and_then(|hands| hands.subcommand_matches("serve"))
            .is_some_and(|serve| {
                serve.value_source("reach").is_none()
                    && serve.value_source("reach_base64url").is_none()
            })
    })
}

/// When `hands serve` fails clap validation because neither `--reach` nor
/// `--reach-base64url` was given, re-render the error with `--reach` marked
/// unconditionally required so clap emits main's exact usage order
/// (`--reach <REACH>` before `--spec <SPEC>`, #1277 V3, proposed decision
/// 0125 ruling 2).
pub(crate) fn reformat_missing_serve_reach(
    err: clap::Error,
    argv: &[std::ffi::OsString],
    command: impl FnOnce() -> clap::Command,
) -> clap::Error {
    if err.kind() != clap::error::ErrorKind::MissingRequiredArgument {
        return err;
    }
    let cmd = command();
    if !is_missing_serve_reach(argv, &cmd) {
        return err;
    }
    cmd.mut_subcommand("hands", |hands| {
        hands.mut_subcommand("serve", |serve| {
            serve.mut_arg("reach", |reach| {
                reach
                    .required(true)
                    .required_unless_present(clap::builder::Resettable::<clap::Id>::Reset)
            })
        })
    })
    .try_get_matches_from(argv)
    .unwrap_err()
}

pub(crate) fn run(command: HandsCommand) -> anyhow::Result<ExitCode> {
    run_with(command, Session::create)
}

/// Parse a sealed reach JSON document directly into [`Sealed`] with no
/// intermediate `Value` or map: shared by both `--reach` and
/// `--reach-base64url` (#1277 V1).
fn parse_reach_doc(reach: &str) -> Result<Sealed, serde_json::Error> {
    serde_json::from_str(reach)
}

/// The reach `--reach` carries as plain JSON: one that does not read refuses
/// with MB3's identity cause, and no box is built.
fn parse_reach_plain(reach: &str) -> Result<Sealed, DocumentError> {
    parse_reach_doc(reach).map_err(|cause| DocumentError::PlainReach { cause })
}

/// The reach `--reach-base64url` carries after decoding: runs the exact same
/// `serde_json::from_str::<Sealed>` parse as `--reach`, then reduces any error
/// to its [`serde_json::error::Category`] so nothing decoded survives in
/// `Display` or `Debug` (#1277 V1, R3).
fn parse_reach_encoded(reach: &str) -> Result<Sealed, DocumentError> {
    parse_reach_doc(reach).map_err(|cause| DocumentError::EncodedReach {
        category: cause.classify(),
    })
}

enum SpecDocError {
    Json(serde_json::Error),
    Spec(SpecRefusal),
}

/// Parse a box spec JSON document through main's `serde_json::from_str::<Value>`
/// and [`HandsSpec::checked`] path: shared by both `--spec` and
/// `--spec-base64url` (#1277 V1).
fn parse_spec_doc(spec: &str) -> Result<HandsSpec, SpecDocError> {
    let raw: serde_json::Value = serde_json::from_str(spec).map_err(SpecDocError::Json)?;
    HandsSpec::checked(&raw).map_err(SpecDocError::Spec)
}

/// The box spec `--spec` carries as plain JSON, keeping main's diagnostics
/// byte for byte.
fn parse_spec_plain(spec: &str) -> Result<HandsSpec, DocumentError> {
    parse_spec_doc(spec).map_err(|cause| match cause {
        SpecDocError::Json(cause) => DocumentError::PlainSpecJson { cause },
        SpecDocError::Spec(cause) => DocumentError::PlainSpec { cause },
    })
}

/// The box spec `--spec-base64url` carries after decoding: runs the exact same
/// parse as `--spec`, then reduces any error to its serde category or
/// payload-free [`SpecRefusalKind`] so no decoded text survives in `Display`
/// or `Debug` (#1277 V1, R3).
fn parse_spec_encoded(spec: &str) -> Result<HandsSpec, DocumentError> {
    parse_spec_doc(spec).map_err(|cause| match cause {
        SpecDocError::Json(err) => DocumentError::EncodedSpecJson {
            category: err.classify(),
        },
        SpecDocError::Spec(refusal) => DocumentError::EncodedSpec {
            cause: SpecRefusalKind::from(refusal),
        },
    })
}

fn resolve_serve_spec(
    spec: Option<String>,
    spec_base64url: Option<String>,
) -> Result<HandsSpec, DocumentError> {
    match (spec, spec_base64url) {
        (Some(_), Some(_)) => Err(DocumentError::BothSpellings {
            plain: SPEC_OPTION,
            encoded: SPEC_BASE64URL_OPTION,
        }),
        (plain, None) => parse_spec_plain(plain.as_deref().unwrap_or(DEFAULT_SPEC)),
        (None, Some(raw)) => {
            let json = hands::decode_base64url(&raw).map_err(|cause| DocumentError::Encoded {
                flag: SPEC_BASE64URL_OPTION,
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
            plain: REACH_OPTION,
            encoded: REACH_BASE64URL_OPTION,
        }),
        (None, None) => Err(DocumentError::MissingReach),
        (Some(json), None) => parse_reach_plain(&json),
        (None, Some(raw)) => {
            let json = hands::decode_base64url(&raw).map_err(|cause| DocumentError::Encoded {
                flag: REACH_BASE64URL_OPTION,
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
            // `DocumentError::Display` already carries the plain cause to keep
            // main's stderr text byte for byte while `Error::source` exposes
            // it to typed callers; `Error::msg` stops `lib.rs`'s `{error:#}`
            // from printing the cause a second time.
            let (spec, sealed) =
                resolve_serve_documents(reach, reach_base64url, spec, spec_base64url)
                    .map_err(anyhow::Error::msg)?;
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
                parse_spec_plain(&spec).map_err(anyhow::Error::msg)?,
                parse_reach_plain(&reach).map_err(anyhow::Error::msg)?,
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
