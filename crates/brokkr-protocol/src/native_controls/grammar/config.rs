//! Codex's configuration assignments as the grammar reads them, and the
//! isolation options that keep a Codex seat from inheriting MCP
//! configuration and from reading the secrets store (decision 0065 slice
//! two, U9c1; SI2), as the controller measured them on codex-cli 0.162.1:
//! `--ignore-user-config`, `--disable` of exactly `apps` and `plugins`, and
//! the engine's permission profile, `-c permissions.<name>={filesystem={…}}`
//! beside `-c default_permissions="<name>"`, read as the typed rendering of
//! a sandbox class bound to the seat's workspace root and the store's
//! handed path, which it denies. On 0.162.1 `-s` silently overrides a
//! profile, so a profile beside any other sandbox expression refuses.

use super::{blank, canonical_key, key_parts, Arity, Command, Effect, Node, Power, Spec};
use super::{SandboxIntent, KEY_BYTES_MAX};
use crate::native_controls::Isolation;

/// A feature toggle: one feature's name, split or equals-joined, repeatable.
pub(crate) const fn feature(canonical: &'static str, power: Power) -> Spec {
    Spec {
        canonical,
        aliases: &[],
        arity: Arity::One,
        equals: true,
        attached: false,
        repeat: true,
        effect: Effect::Control(power),
    }
}

/// The dotted key parts and the trimmed value of one `KEY=VALUE`
/// assignment, or the fixed cause it cannot be read for.
pub(super) fn assignment_parts(assignment: &str) -> Result<(Vec<&str>, &str), &'static str> {
    const NOT_ASSIGNMENT: &str = "is not a KEY=VALUE configuration assignment";
    const MALFORMED: &str = "assigns through a key the grammar cannot read: each dotted part is a \
                             bare name or a quoted one without escapes, within 16 parts and 256 \
                             bytes";
    const NO_VALUE: &str = "assigns no value";
    let mut quote = None;
    let split = assignment.char_indices().find(|&(_, c)| {
        match (quote, c) {
            (None, '"' | '\'') => quote = Some(c),
            (Some(open), _) if open == c => quote = None,
            _ => {}
        }
        quote.is_none() && c == '='
    });
    let Some((at, _)) = split else {
        return Err(NOT_ASSIGNMENT);
    };
    let (key, value) = (&assignment[..at], &assignment[at + 1..]);
    if key.len() > KEY_BYTES_MAX || key.chars().any(char::is_control) {
        return Err(MALFORMED);
    }
    let parts = key_parts(key).ok_or(MALFORMED)?;
    let value = value.trim_matches(blank);
    if value.is_empty() {
        return Err(NO_VALUE);
    }
    Ok((parts, value))
}

/// Why a Codex command's isolation options cannot be read as the engine's
/// (U9c1; SI2), each at the argument it stands at. The text names fixed
/// words and canonical options only, never a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum Unisolated {
    #[error(
        "carries '--enable' (argument {0}), which switches a feature on, and no feature is \
         switched on"
    )]
    Enabled(usize),
    #[error(
        "carries '--disable' (argument {0}) of a feature other than 'apps' or 'plugins', the \
         two it was measured to disable"
    )]
    Disabled(usize),
    #[error(
        "carries '--config' (argument {0}) under a permission profile through a dotted key, \
         where the profile is one whole table assigned at once"
    )]
    Dotted(usize),
    #[error(
        "carries '--config' (argument {0}) into the 'permissions' configuration with something \
         other than one named profile of one filesystem table, written canonically"
    )]
    Unprofiled(usize),
    #[error(
        "carries '--config' (argument {0}), a permission profile granting a filesystem access \
         other than 'read', 'write' or 'deny'"
    )]
    Access(usize),
    #[error(
        "carries '--config' (argument {0}), a permission profile, where no secrets store's path \
         is handed beside the workdir to bind it to, so no profile is admitted"
    )]
    Unhanded(usize),
    #[error(
        "carries '--config' (argument {0}), a permission profile that renders no sandbox class: \
         ':root' is read once, no other path is read, and no path is named twice"
    )]
    Classless(usize),
    #[error(
        "carries '--config' (argument {0}), a permission profile that writes a path other than \
         the seat's workspace root"
    )]
    Unrooted(usize),
    #[error(
        "carries '--config' (argument {0}), a permission profile writing the workspace root but \
         not every folder between it and the secrets store inside it, which a command could move"
    )]
    Unpinned(usize),
    #[error(
        "carries '--config' (argument {0}), a permission profile that denies a path other than \
         the workspace's secrets store"
    )]
    Unstored(usize),
    #[error(
        "carries '--config' (argument {0}), a permission profile that does not deny the \
         workspace's secrets store, so it stays readable"
    )]
    Undenied(usize),
    #[error(
        "carries '--config' (argument {0}) into the 'default_permissions' configuration with \
         something other than the quoted name of the one profile defined before it"
    )]
    Selector(usize),
    #[error("carries a second permission profile or selector (argument {0}); a command runs one")]
    Twice(usize),
    #[error("defines a permission profile (argument {0}) that no 'default_permissions' selects")]
    Unselected(usize),
    #[error(
        "carries '--sandbox' or a 'sandbox_mode' assignment (argument {0}) beside a permission \
         profile, which on codex-cli 0.162.1 it silently overrides"
    )]
    Beside(usize),
}

/// Each measured isolation option, the value it takes where it takes one,
/// and the isolation it expresses, in the order the Codex builders emit
/// them: the one home of their vocabulary, which [`isolation`] reads and
/// [`isolation_options`] writes (U9c2).
const ISOLATION: [(&str, Option<&str>, Isolation); 3] = [
    ("--ignore-user-config", None, Isolation::UserConfig),
    ("--disable", Some("apps"), Isolation::Apps),
    ("--disable", Some("plugins"), Isolation::Plugins),
];

/// The isolation options, as a Codex launch carries them.
pub(crate) fn isolation_options() -> Vec<String> {
    let parts = ISOLATION
        .iter()
        .flat_map(|(option, value, _)| [Some(*option), *value]);
    parts.flatten().map(String::from).collect()
}

/// The isolation one node of a Codex command expresses, its fixed cause
/// where it is a toggle the engine does not compose, or `None` for a node
/// that is no isolation option.
pub(crate) fn isolation(node: &Node) -> Option<Result<Isolation, Unisolated>> {
    let at = node.at + 1;
    let spelled = (node.name(), node.values.first().map(String::as_str));
    let measured = ISOLATION
        .iter()
        .find(|(option, value, _)| spelled == (*option, *value));
    if let Some((.., isolation)) = measured {
        return Some(Ok(*isolation));
    }
    match spelled.0 {
        "--disable" => Some(Err(Unisolated::Disabled(at))),
        "--enable" => Some(Err(Unisolated::Enabled(at))),
        _ => None,
    }
}

/// The paths a permission profile is bound to (operator ruling 4 of
/// 2026-10-23): the seat's workspace root, and the secrets store's path the
/// final check is handed beside it. Where no store is handed every profile
/// refuses, and so it does under the default, which names no run's paths.
/// The paths are compared as exact strings, never resolved, but for whether
/// the store lies inside the root (U8i).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Bound<'a> {
    pub workdir: &'a str,
    pub store: Option<&'a str>,
}

/// The name the engine defines and selects its one permission profile by.
const PROFILE: &str = "brokkr";

/// The engine's rendering of the sandbox class the word `class` names as its
/// one permission profile bound to `bound` (U9c2; SI2), the four arguments
/// its Codex builders emit and [`rendering`] reads back as that class:
/// `:root` read, exactly the workspace root written where the class is
/// workspace-write, followed by every folder it [`pinned`] (U8i), exactly
/// the store denied, then the selector. `None` where no store is handed or
/// the word names neither read-only nor workspace-write.
pub(crate) fn rendered(class: &str, bound: Bound<'_>) -> Option<Vec<String>> {
    let store = bound.store?;
    let written = if class == SandboxIntent::WorkspaceWrite.word() {
        let pins = pinned(class, bound).unwrap_or_default();
        let written = std::iter::once(bound.workdir).chain(pins);
        written
            .map(|path| format!(r#","{path}"="write""#))
            .collect()
    } else if class == SandboxIntent::ReadOnly.word() {
        String::new()
    } else {
        return None;
    };
    let filesystem = format!(r#"":root"="read"{written},"{store}"="deny""#);
    Some(vec![
        "-c".into(),
        format!("permissions.{PROFILE}={{filesystem={{{filesystem}}}}}"),
        "-c".into(),
        format!("default_permissions=\"{PROFILE}\""),
    ])
}

/// The folders the profile of the class the word `class` names, bound to
/// `bound`, writes beside its root (U8i; SI2), which only a workspace-write
/// one does, since a read-only class already refuses the move and a pin
/// would grant it writes: Codex builds a fresh sandbox for each command
/// and denies the store by its path, so a written folder holding it could be
/// moved and the store read at its new path, while a folder named as
/// written is its own mount in every sandbox, which the kernel never moves
/// (U0b's X29 and X31). `Some` of every folder strictly between the
/// workspace root and a store lying strictly inside it, from the root
/// outwards, each spelled as a prefix of the store's handed path, so empty
/// for a store directly in the root; `None` for another class, where no
/// store is handed or where it lies outside the root. The store is the
/// engine's canonical place (U8h), so the root is compared both as spelled
/// and as its canonical form, by whole components: a root spelled through a
/// link still holds the store it resolves to.
pub(crate) fn pinned<'a>(class: &str, bound: Bound<'a>) -> Option<Vec<&'a str>> {
    let writes = class == SandboxIntent::WorkspaceWrite.word();
    let store = bound.store.filter(|_| writes)?;
    let canonical = std::fs::canonicalize(bound.workdir).ok();
    let spelled = Some(std::path::Path::new(bound.workdir));
    let mut roots = [spelled, canonical.as_deref()].into_iter().flatten();
    roots.find_map(|root| folders(root, store))
}

/// The folders strictly between an absolute `root` and a `store` lying
/// strictly inside it through plain components only, from the root
/// outwards, as prefixes of `store`; `None` for any other pair, so a
/// relative root, a `..` or a store that is the root itself pins nothing.
fn folders<'s>(root: &std::path::Path, store: &'s str) -> Option<Vec<&'s str>> {
    use std::path::{Component, Path};
    let rest = Path::new(store).strip_prefix(root).ok()?;
    let plain = rest
        .components()
        .all(|part| matches!(part, Component::Normal(_)));
    let depth = rest.components().count();
    if !root.is_absolute() || !plain || depth == 0 {
        return None;
    }
    let mut folders: Vec<&str> = (Path::new(store).ancestors().skip(1).take(depth - 1))
        .filter_map(Path::to_str)
        .collect();
    folders.reverse();
    Some(folders)
}

/// The table one configuration `value` assigns into, `None` where it is no
/// assignment the grammar reads.
pub(super) fn table(value: &str) -> Option<&str> {
    assignment_parts(value).ok().map(|(parts, _)| parts[0])
}

/// What [`profile`] has read of one command so far: the profile defined,
/// its name, class and argument; whether a selector selected it; and where
/// a sandbox expression stands.
#[derive(Default)]
struct Reading<'a> {
    defined: Option<(&'a str, SandboxIntent, usize)>,
    selected: bool,
    sandboxed: Option<usize>,
}

impl<'a> Reading<'a> {
    /// Read one configuration `value` standing at argument `at`. An
    /// assignment the grammar cannot read is left to its reader's own cause.
    fn assign(&mut self, value: &'a str, at: usize, bound: Bound<'_>) -> Result<(), Unisolated> {
        let Ok((parts, body)) = assignment_parts(value) else {
            return Ok(());
        };
        match parts[0] {
            "permissions" if self.defined.is_some() => return Err(Unisolated::Twice(at)),
            "permissions" => {
                let (name, class) = definition(value, &parts, body, at, bound)?;
                self.defined = Some((name, class, at));
            }
            "default_permissions" if self.selected => return Err(Unisolated::Twice(at)),
            "default_permissions" => {
                let named =
                    (self.defined).map(|(name, ..)| format!("default_permissions=\"{name}\""));
                self.selected = named.is_some_and(|named| named == value);
                if !self.selected {
                    return Err(Unisolated::Selector(at));
                }
            }
            "sandbox_mode" => self.sandboxed = Some(at),
            _ => {}
        }
        Ok(())
    }
}

/// The sandbox class a Codex command's one permission profile renders
/// bound to `bound`, `None` where it defines none, or why it is not the
/// engine's: a profile is exactly one canonical `permissions.<name>` table,
/// selected once by `default_permissions="<name>"` after it, and stands
/// beside no `--sandbox` and no `sandbox_mode` assignment.
pub(crate) fn profile(
    command: &Command,
    bound: Bound<'_>,
) -> Result<Option<SandboxIntent>, Unisolated> {
    let mut reading = Reading::default();
    for node in &command.nodes {
        let at = node.at + 1;
        if node.name() == "--sandbox" {
            reading.sandboxed = Some(at);
        }
        let values = node
            .values
            .iter()
            .filter(|_| node.spec.effect == Effect::Config);
        for value in values {
            reading.assign(value, at, bound)?;
        }
    }
    match (reading.defined, reading.sandboxed, reading.selected) {
        (None, _, _) => Ok(None),
        (Some(_), Some(at), _) => Err(Unisolated::Beside(at)),
        (Some((.., at)), None, false) => Err(Unisolated::Unselected(at)),
        (Some((_, class, _)), None, true) => Ok(Some(class)),
    }
}

/// One `permissions.<name>={filesystem={…}}` definition, its key `parts`
/// and its trimmed `body` as [`assignment_parts`] read them, read as the
/// name and the sandbox class it renders bound to `bound` ([`rendering`]).
fn definition<'a>(
    assignment: &'a str,
    parts: &[&'a str],
    body: &'a str,
    at: usize,
    bound: Bound<'_>,
) -> Result<(&'a str, SandboxIntent), Unisolated> {
    if parts.len() > 2 {
        return Err(Unisolated::Dotted(at));
    }
    let (key, raw) = assignment.split_once('=').unwrap_or_default();
    let (&[_, name], true) = (parts, canonical_key(key) && raw == body) else {
        return Err(Unisolated::Unprofiled(at));
    };
    let entries = filesystem(body).ok_or(Unisolated::Unprofiled(at))?;
    if !(entries.iter()).all(|(_, access)| matches!(*access, "read" | "write" | "deny")) {
        return Err(Unisolated::Access(at));
    }
    rendering(&entries, bound, at).map(|class| (name, class))
}

/// The class a profile's filesystem `entries` render, bound to `bound`
/// (operator ruling 4 of 2026-10-23), never read by their shape alone: a
/// store must be handed; `":root"="read"` once; a `"write"` only of exactly
/// the workspace root or a folder it [`pinned`], and of those folders only
/// beside the root's, every one of them (U8i); one `"deny"`, of exactly the
/// store; no other entry, and no path twice. Workspace-write where the root
/// is written, read-only where nothing is.
fn rendering(
    entries: &[(&str, &str)],
    bound: Bound<'_>,
    at: usize,
) -> Result<SandboxIntent, Unisolated> {
    let Some(store) = bound.store else {
        return Err(Unisolated::Unhanded(at));
    };
    let pins = pinned(SandboxIntent::WorkspaceWrite.word(), bound).unwrap_or_default();
    for &(path, access) in entries {
        match access {
            "write" if path != bound.workdir && !pins.contains(&path) => {
                return Err(Unisolated::Unrooted(at))
            }
            "deny" if path != store => return Err(Unisolated::Unstored(at)),
            "read" if path != ":root" => return Err(Unisolated::Classless(at)),
            _ => {}
        }
    }
    let named = |path: &str| entries.iter().filter(|(named, _)| *named == path).count();
    let once = entries.iter().all(|(path, _)| named(path) == 1);
    if !once || !entries.contains(&(":root", "read")) {
        return Err(Unisolated::Classless(at));
    }
    if !entries.contains(&(store, "deny")) {
        return Err(Unisolated::Undenied(at));
    }
    classed(entries, bound.workdir, pins.len(), at)
}

/// The class `entries` render once [`rendering`] has held each write to the
/// root `workdir` or one of its `pins` folders and each path to one entry,
/// so the count of writes says whether every pin stands beside the root's:
/// read-only where nothing is written, workspace-write where the root and
/// every pin are, and a refusal otherwise.
fn classed(
    entries: &[(&str, &str)],
    workdir: &str,
    pins: usize,
    at: usize,
) -> Result<SandboxIntent, Unisolated> {
    let written = entries.iter().filter(|(_, access)| *access == "write");
    match (entries.contains(&(workdir, "write")), written.count()) {
        (false, 0) => Ok(SandboxIntent::ReadOnly),
        (false, _) => Err(Unisolated::Unrooted(at)),
        (true, count) if count == pins + 1 => Ok(SandboxIntent::WorkspaceWrite),
        (true, _) => Err(Unisolated::Unpinned(at)),
    }
}

/// The `"<path>"="<access>"` entries of one canonical
/// `{filesystem={…}}` table, joined by single commas with nothing around
/// them, `None` for any other text.
fn filesystem(body: &str) -> Option<Vec<(&str, &str)>> {
    let mut rest = body.strip_prefix("{filesystem={")?;
    let mut entries = Vec::new();
    loop {
        let (path, tail) = quoted(rest)?;
        let (access, tail) = quoted(tail.strip_prefix('=')?)?;
        entries.push((path, access));
        match tail.strip_prefix(',') {
            Some(next) => rest = next,
            None => return (tail == "}}").then_some(entries),
        }
    }
}

/// One nonempty TOML basic string without an escape or a control
/// character, and the text after it.
fn quoted(text: &str) -> Option<(&str, &str)> {
    let body = text.strip_prefix('"')?;
    let (inner, tail) = body.split_once('"')?;
    carried(inner).then_some((inner, tail))
}

/// Whether `text` is carried whole by a TOML basic string with no escape:
/// nonempty, holding no quote, backslash or control character. A path that
/// is not is refused at a Codex launch, never escape-encoded into its
/// profile (U9c2; operator ruling of 2026-10-23).
pub(crate) fn carried(text: &str) -> bool {
    !text.is_empty() && !text.contains(['"', '\\']) && !text.chars().any(char::is_control)
}
