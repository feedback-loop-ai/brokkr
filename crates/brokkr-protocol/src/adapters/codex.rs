//! A Codex seat launched under its isolation options (decision 0065 slice
//! two, U9c2; SI2), as the controller measured them on codex-cli 0.162.1:
//! `--ignore-user-config`, `--disable apps` and `--disable plugins` keep it
//! from the user's configuration, a trusted project's, and its apps and
//! plugins; its sandbox class is rendered as the engine's one permission
//! profile, which denies the workspace's secrets store and which a `-s`
//! beside it would silently override; and a host holding codex's system or
//! managed configuration, which loads whatever options are passed, is
//! refused before any provider work, as is a sealed launch handed no store
//! and a path its profile's basic strings cannot carry unescaped (operator
//! ruling of 2026-10-23), and on macOS a sealed workspace-write launch
//! whose store lies inside its root, which no mount pins there (U8i;
//! operator ruling 3 of 2026-10-24). Beside them, the inline launch's final
//! judgment, moved here from `adapters.rs`, which sits at its line
//! baseline, when it learned the profile.

use std::path::Path;

use serde_json::Value;

use super::{
    codex_effort_config, codex_launch_and_cold, effort_split, picked, sandbox_split, LaunchPlan,
    ServingShape,
};
use crate::native_controls::grammar::{self, Bound, Pins};
use crate::native_controls::SERVING_INPUTS;

/// The secrets store's path the input hands a seat, which a Codex seat's
/// permission profile denies: the engine hands it to every Codex seat,
/// whether or not the seat binds a secret. `None` where the input hands
/// none: a launch the engine sealed then refuses ([`Unlaunchable::Storeless`]),
/// and only a driver run by hand, unsealed, is served its class as written.
pub(super) fn store(input: &Value) -> Option<&str> {
    input.get("secrets_file").and_then(Value::as_str)
}

/// The isolation options a launch of `shape` carries behind its composed
/// arguments: Codex's measured three where the input hands the store its
/// profile denies, and none otherwise. The final check rebuilds them as the
/// configuration the launch's isolation built.
pub(super) fn isolating(shape: Option<ServingShape>, input: &Value) -> Vec<String> {
    if shape == Some(ServingShape::Codex) && store(input).is_some() {
        return grammar::isolation_options();
    }
    Vec::new()
}

/// A cold command's class, taken out of its `passthrough` and rendered as
/// the engine's permission profile bound to `bound`, which leads the
/// command: where no store is handed, or no class the profile renders is
/// written, the profile is empty and the passthrough is whole.
pub(super) fn profiled(passthrough: Vec<String>, bound: Bound<'_>) -> (Vec<String>, Vec<String>) {
    let (class, kept) = sandbox_split(&passthrough);
    match class.and_then(|class| grammar::rendered(&class, bound)) {
        Some(profile) => (profile, picked(&passthrough, &kept)),
        None => (Vec::new(), passthrough),
    }
}

/// The class a rejoin re-imposes: the engine's permission profile bound to
/// `bound`, or, where no store is handed or the class is not one the
/// profile renders, the assignment `codex exec resume` reads it from.
pub(super) fn rejoined(class: &str, bound: Bound<'_>) -> Vec<String> {
    grammar::rendered(class, bound)
        .unwrap_or_else(|| vec!["-c".into(), format!("sandbox_mode=\"{class}\"")])
}

/// The configuration codex reads from the host whatever options it is
/// passed (U9c2; SI2): the controller saw MCP servers named in either start
/// under `--ignore-user-config`, so a host holding one refuses every Codex
/// seat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(super) enum HostConfig {
    #[error(
        "refusing to invoke the agent CLI: this host holds codex's system configuration at \
         '/etc/codex/config.toml', which codex loads whatever options it is passed, so an MCP \
         server it names would start beside the engine's set; a Codex seat is refused on such a \
         host before any provider work (decision 0065 slice two, U9c2; SI2)"
    )]
    System,
    #[error(
        "refusing to invoke the agent CLI: this host holds codex's managed configuration at \
         '/etc/codex/managed_config.toml', which codex loads whatever options it is passed, so an \
         MCP server it names would start beside the engine's set; a Codex seat is refused on \
         such a host before any provider work (decision 0065 slice two, U9c2; SI2)"
    )]
    Managed,
}

impl HostConfig {
    /// Where the host holds each.
    fn path(self) -> &'static Path {
        match self {
            HostConfig::System => Path::new("/etc/codex/config.toml"),
            HostConfig::Managed => Path::new("/etc/codex/managed_config.toml"),
        }
    }

    /// The first the host holds, as `exists` observes it.
    pub(super) fn observed(exists: impl Fn(&Path) -> bool) -> Option<HostConfig> {
        [HostConfig::System, HostConfig::Managed]
            .into_iter()
            .find(|config| exists(config.path()))
    }
}

/// Whether the host holds `path`: anything but its observed absence, so a
/// path that cannot be looked at is held, and refuses.
pub(super) fn present(path: &Path) -> bool {
    !matches!(path.try_exists(), Ok(false))
}

/// Why a Codex launch is refused at the driver's door, before any provider
/// work (U9c2; SI2; operator ruling of 2026-10-23).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(super) enum Unlaunchable {
    #[error(transparent)]
    Host(#[from] HostConfig),
    #[error(
        "refusing to invoke the agent CLI: this Codex launch is sealed by the engine but handed \
         no secrets store's path, so its sandbox class cannot be rendered as the permission \
         profile denying the store; a sealed Codex launch is never served under '--sandbox', \
         which leaves the store readable (decision 0065 slice two, U9c2; SI2)"
    )]
    Storeless,
    #[error(
        "refusing to invoke the agent CLI: the seat's workspace root cannot be carried by a TOML \
         basic string without an escape (it is empty or holds a quote, a backslash or a control \
         character), so the permission profile cannot name it; such a path is refused, never \
         escaped (decision 0065 slice two, U9c2; SI2)"
    )]
    UncarriedRoot,
    #[error(
        "refusing to invoke the agent CLI: the secrets store's path cannot be carried by a TOML \
         basic string without an escape (it is empty or holds a quote, a backslash or a control \
         character), so the permission profile cannot deny it; such a path is refused, never \
         escaped (decision 0065 slice two, U9c2; SI2)"
    )]
    UncarriedStore,
    #[error(
        "refusing to invoke the agent CLI: this sealed Codex launch of the workspace-write class \
         keeps the secrets store inside its workspace root, or where its paths cannot place it \
         outside, and on macOS Codex's sandbox matches \
         paths, so no mount pins the store's folder and a command could move it and read the \
         store at its new path; such a launch is refused on macOS, where the move is unmeasured \
         (decision 0065 slice two, U8i; SI2)"
    )]
    Unpinnable,
}

/// How the host's Codex sandbox holds the paths a profile names (U8i): by
/// mounts on Linux, where a folder named as written is pinned against a
/// move, and by matching paths on macOS, where nothing pins one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Sandboxing {
    Mounts,
    Paths,
}

/// This host's [`Sandboxing`].
const HOST: Sandboxing = if cfg!(target_os = "macos") {
    Sandboxing::Paths
} else {
    Sandboxing::Mounts
};

/// Whether a launch over `input` and `extra`, rooted at `workdir`, may
/// proceed under `sandboxing` (U8i; operator ruling 3 of 2026-10-24): one
/// the engine sealed, of the workspace-write class, whose handed store lies
/// inside the workspace root or cannot be placed outside it
/// ([`grammar::pinned`]), refuses where the sandbox matches paths. Every
/// other launch passes.
pub(super) fn pinnable(
    input: &Value,
    extra: &[String],
    workdir: &str,
    sandboxing: Sandboxing,
) -> Result<(), Unlaunchable> {
    let (class, _) = sandbox_split(extra);
    let bound = Bound {
        workdir,
        store: store(input),
    };
    let held = |class: String| match grammar::pinned(&class, bound) {
        Pins::Unneeded => false,
        Pins::Folders(_) | Pins::Unplaceable => true,
    };
    match sandboxing {
        Sandboxing::Mounts => Ok(()),
        Sandboxing::Paths if sealed(input) && class.is_some_and(held) => {
            Err(Unlaunchable::Unpinnable)
        }
        Sandboxing::Paths => Ok(()),
    }
}

/// Whether the engine sealed a launch over `input`: its launch record
/// beside its sealed serving inputs.
fn sealed(input: &Value) -> bool {
    input.get("launch_record").is_some() && input.get(SERVING_INPUTS).is_some()
}

/// Whether a Codex launch over `input`, rooted at `workdir`, may proceed on a
/// host `exists` observes: the host holds neither of codex's configurations
/// ([`HostConfig`]); a launch the engine sealed, its launch record beside its
/// sealed serving inputs, is handed the store, so it is launched under the
/// profile denying it; and where a store is handed, both paths the profile
/// names are carried by its basic strings unescaped. Half that pair, or a
/// capability plan without it, passes to the refusals the composition and
/// the final check already make of it, in their own words.
pub(super) fn admitted(
    input: &Value,
    workdir: &str,
    exists: impl Fn(&Path) -> bool,
) -> Result<(), Unlaunchable> {
    if let Some(held) = HostConfig::observed(exists) {
        return Err(held.into());
    }
    let Some(store) = store(input) else {
        return if sealed(input) {
            Err(Unlaunchable::Storeless)
        } else {
            Ok(())
        };
    };
    if !grammar::carried(workdir) {
        return Err(Unlaunchable::UncarriedRoot);
    }
    if !grammar::carried(store) {
        return Err(Unlaunchable::UncarriedStore);
    }
    Ok(())
}

/// Why the driver's Codex launch was not composed: refused at its door, or
/// refused by the composition, which keeps its own words.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(super) enum Unlaunched {
    #[error(transparent)]
    Door(#[from] Unlaunchable),
    #[error("{0}")]
    Composition(String),
}

/// The driver's text of a refused launch, at the one boundary that returns
/// text, `invoke_with_stager`, the driver's spawn of an agent CLI.
impl From<Unlaunched> for String {
    fn from(unlaunched: Unlaunched) -> String {
        unlaunched.to_string()
    }
}

/// [`launch_on`] under this host's [`Sandboxing`].
pub(super) fn launch_on_host(
    bin: &str,
    extra: &[String],
    workdir: &str,
    session: Option<&str>,
    input: &Value,
    exists: impl Fn(&Path) -> bool,
) -> Result<(LaunchPlan, Vec<String>), Unlaunched> {
    launch_on(bin, extra, workdir, session, input, exists, HOST)
}

/// [`codex_launch_and_cold`] on a host `exists` observes, its sandbox holding
/// paths by `sandboxing`, refused before any provider work, the version probe
/// included, where [`admitted`] or [`pinnable`] refuses. The refusal stays
/// typed here; the driver's spawn turns it into its text.
pub(super) fn launch_on(
    bin: &str,
    extra: &[String],
    workdir: &str,
    session: Option<&str>,
    input: &Value,
    exists: impl Fn(&Path) -> bool,
    sandboxing: Sandboxing,
) -> Result<(LaunchPlan, Vec<String>), Unlaunched> {
    admitted(input, workdir, exists)?;
    pinnable(input, extra, workdir, sandboxing)?;
    codex_launch_and_cold(bin, extra, workdir, session, input).map_err(Unlaunched::Composition)
}

/// Rebuild unit 5d-fix-c2 (chief F2 of runs
/// `0065-rebuild-unit-5d-fix-b-see-t-8067eebc` and
/// `0065-rebuild-unit-5d-fix-c1-see--b800f52a`; operator ruling of
/// 2026-09-25, "narrow"; design D5.3): where the sealed launch record names
/// the engine's own `local` class, which is an inline Codex site's, the
/// FINAL command — after `--effort` was translated, `--json` and `-C`
/// generated and the class moved into its lead — is judged exactly as the
/// harness receives it, through [`grammar::judge_inline_codex_command`] and
/// so the same `judge_inline_codex_launch` admission and the dispatch door
/// call, handed the seat's workdir and the store's path its profile is bound
/// to (U9c2).
///
/// The record's segments must reassemble the arguments composed, so every
/// part the harness receives is judged beside its origin: the translated
/// effort carries the origin of the pin it translated, the `isolating`
/// options the launch's isolation built are the engine's `local`
/// contribution, and the native plan is the engine's `native` one. A gate is
/// the `read-only` class, whose capture must be into exactly the result path
/// the input names. The cause is bounded and value-free, and names no seat.
/// A launch no record was sealed for, and one whose record names no `local`
/// class (an agent's class rides its hands), is not an inline Codex launch
/// and is not judged here.
pub(super) fn inline_final(
    input: &Value,
    composed: &crate::native_controls::Composed,
    isolating: &[String],
    workdir: &str,
    command: &[String],
    rejoin: bool,
) -> Result<(), String> {
    use crate::native_controls::{
        reassemble, HandsIntent, LaunchRecord, Origin, SandboxIntent, Segment,
    };
    let Some(sealed) = input.get("launch_record") else {
        return Ok(());
    };
    let refuse = |cause: &str| {
        format!(
            "refusing to invoke the agent CLI: the final command of this inline Codex launch \
             {cause} (decision 0046 ruling 4; operator ruling of 2026-09-25; rebuild unit \
             5d-fix-c2)"
        )
    };
    // The decoder's and the reassembly's text is not echoed: the final
    // judgment's causes are fixed.
    let record = LaunchRecord::decode(Some(sealed)).map_err(|_| {
        refuse("carries a launch record that cannot be read, so its class cannot be judged")
    })?;
    let class = match record.expected.hands {
        HandsIntent::None => record.expected.local.sandbox,
        HandsIntent::Required => SandboxIntent::Unspecified,
    };
    if class == SandboxIntent::Unspecified {
        return Ok(());
    }
    let extra = &composed.extra;
    reassemble(&record.segments, extra).map_err(|_| {
        refuse(
            "is composed from arguments its sealed record does not reassemble, so no part of it \
             has an origin to be judged by",
        )
    })?;
    let origins: Vec<Origin> = record
        .segments
        .iter()
        .flat_map(|segment| std::iter::repeat_n(segment.origin, segment.argv.len()))
        .collect();
    // The composition's own splits, by position: the effort leaves first,
    // and where the class leads the command, on a rejoin or as the profile
    // a handed store binds, the class leaves what remains.
    let store = store(input);
    let (effort, mut kept) = effort_split(extra);
    if rejoin || store.is_some() {
        let (_, remaining) = sandbox_split(&picked(extra, &kept));
        kept = remaining.iter().map(|&at| kept[at]).collect();
    }
    let mut runs: Vec<(Origin, Vec<String>)> = Vec::new();
    let mut place = |origin: Origin, part: String| match runs.last_mut() {
        Some((last, parts)) if *last == origin => parts.push(part),
        _ => runs.push((origin, vec![part])),
    };
    if let Some((level, at)) = effort {
        place(origins[at], "-c".to_string());
        place(origins[at], codex_effort_config(&level));
    }
    for at in kept {
        place(origins[at], extra[at].clone());
    }
    for part in isolating {
        place(Origin::Local, part.clone());
    }
    for part in &composed.managed {
        place(Origin::Native, part.clone());
    }
    let contributions: Vec<Segment> = runs
        .iter()
        .map(|(origin, parts)| Segment::new(*origin, parts))
        .collect();
    let capture = (class == SandboxIntent::ReadOnly)
        .then(|| input["result_path"].as_str().unwrap_or_default());
    let judged = &command[1..];
    grammar::judge_inline_codex_command(class, judged, &contributions, capture, workdir, store)
        .map_err(|cause| refuse(&cause.to_string()))
}
