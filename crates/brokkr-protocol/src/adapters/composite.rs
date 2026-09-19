//! The DSH composite identity, computed by the one Rust producer design D6
//! places beside the DSH planner (task 8.8(b)).
//!
//! Two values are produced here and nowhere else:
//!
//! - a *plugin component* — the SHA-256 of the `<relative path>\0<file
//!   SHA-256>\n` lines over an installed extension directory, in bytewise
//!   path order; and
//! - a *canonical composite* — the SHA-256 of the fixed `<component>\0<value>\n`
//!   lines that name the core, the Node runtime, every normalized dependency
//!   triple from both lock dialects, the plugin component, the patch and
//!   profile lines and the optional extension component.
//!
//! `dsh_composite` is the only production entry point. Every parser, hasher,
//! serializer and injected helper below it is private: a caller consumes the
//! observation and cannot hand the producer an already-computed component or
//! composite value (design D6 (b)).
//!
//! Within one call each identity-bearing source is read once and every use is
//! derived from that read — the hidden lock's bytes serve both the `core` line
//! and the npm triples, and the plugin's `cordis.patch.yml` digest serves both
//! the plugin component and `plugin-patch`. That is a one-pass observation, not
//! an atomic snapshot: it keeps one returned observation from contradicting
//! itself because the producer reopened the same file.
//!
//! Nothing here reads a credential or settings file, and no provenance note,
//! probe script or evidence file computes either value by hand.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};

use serde_json::Value;
use sha2::{Digest, Sha256};

/// The plugin's committed six-file set, in bytewise path order.
const PLUGIN_FILES: [&str; 6] = [
    "LICENSE",
    "README.md",
    "cordis.patch.yml",
    "lib/index.js",
    "lib/startup.js",
    "package.json",
];

/// The conditional extension's four-file set, in bytewise path order.
const EXTENSION_FILES: [&str; 4] = ["LICENSE", "cordis.patch.yml", "index.js", "package.json"];

/// The core package's name, which is also the hidden lock key of its own
/// record — the one record the dependency lines exclude exactly.
const CORE_NAME: &str = "@deepseek-ai/dsh";
const CORE_KEY: &str = "node_modules/@deepseek-ai/dsh";

/// The bundle name whose installed bytes supply the `plugin` line.
const PLUGIN_BUNDLE: &str = "dsh-plugin-cli-session";

/// The bundle name whose installed bytes supply the conditional
/// `extension` line, when the profile names it.
const EXTENSION_BUNDLE: &str = "brokkr-dsh-resume-policy";

/// The inclusive byte bound on the profile's pnpm lock (design D6 (b)).
/// Four orders of magnitude above the measured 1,982 bytes, and read
/// before any unbounded allocation rather than trusted from metadata.
const PNPM_LIMIT: usize = 8_388_608;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CompositeError {
    #[error("plugin component is unreadable: {0}")]
    Component(String),
    #[error("npm lock is unreadable: {0}")]
    NpmLock(String),
    #[error("npm key is unreadable: {0}")]
    NpmKey(String),
    #[error("pnpm lock is unreadable: {0}")]
    PnpmLock(String),
    /// A patch component that is neither the plugin's nor readable: the
    /// profile's or the home's `cordis.patch.yml`. Named separately so an
    /// unreadable profile patch is never reported as plugin drift.
    #[error("{0} is unreadable: {1}")]
    Patch(&'static str, String),
    /// A source scalar that cannot enter an identity line, named by the
    /// component it would have served.
    #[error("the {component} value {reason}")]
    Value {
        component: &'static str,
        reason: &'static str,
    },
    #[error("the DSH layout is unreadable: {0}")]
    Config(String),
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn sha256_text(text: &str) -> String {
    sha256_hex(text.as_bytes())
}

/// The one scalar rule (design D6 (b)): a value entering an identity line
/// is non-empty and carries no NUL and no whitespace — space, tab, CR and
/// LF included. Nothing is trimmed to fit; a malformed value is refused,
/// never repaired. The reason is returned so each caller can name the
/// component responsible for it.
fn scalar_reason(value: &str) -> Option<&'static str> {
    if value.is_empty() {
        return Some("is empty");
    }
    if value.contains('\0') {
        return Some("carries a NUL");
    }
    if value.chars().any(char::is_whitespace) {
        return Some("carries whitespace");
    }
    None
}

/// `scalar_reason` as the refusal a named composite component raises.
fn scalar(component: &'static str, value: &str) -> Result<(), CompositeError> {
    match scalar_reason(value) {
        Some(reason) => Err(CompositeError::Value { component, reason }),
        None => Ok(()),
    }
}

/// One directory's entries as `std::fs::read_dir` yields them, in a shape a
/// test can substitute a failing iterator for. The real reader stays the
/// production read; only the source of the `ReadDir` is injectable, so the
/// per-entry error arm is reachable without a filesystem that errors mid-walk.
type DirEntries = Box<dyn Iterator<Item = std::io::Result<std::fs::DirEntry>>>;

fn read_dir_entries(path: &Path) -> std::io::Result<DirEntries> {
    Ok(Box::new(std::fs::read_dir(path)?))
}

/// Exactly the `expected` files beneath `dir`, each as its relative path
/// with `/` separators mapped to its SHA-256.
///
/// The walk admits the declared set and nothing else. Its single
/// extra-entry exception is a direct, real, non-symlink `node_modules/`
/// directory beneath the root, granted on its metadata and neither
/// traversed nor hashed, because the dependency lines already carry that
/// subtree's identity. A directory is otherwise admitted only as an
/// ancestor the declared set needs, so an empty directory, a stray one and
/// a deeper `node_modules/` are all drift. Every hashed member is a
/// regular non-symlink file. A name this platform cannot spell as UTF-8 is
/// refused by reason rather than merged into a declared spelling by a
/// lossy conversion.
fn plugin_file_digests(
    dir: &Path,
    expected: &[&str],
) -> Result<BTreeMap<String, String>, CompositeError> {
    let mut found = BTreeMap::new();
    walk(dir, "", expected, &mut found, &read_dir_entries)?;
    for path in expected {
        if !found.contains_key(*path) {
            return Err(CompositeError::Component(format!(
                "missing expected file '{path}'"
            )));
        }
    }
    Ok(found)
}

fn walk(
    dir: &Path,
    relative: &str,
    expected: &[&str],
    found: &mut BTreeMap<String, String>,
    read_dir: &dyn Fn(&Path) -> std::io::Result<DirEntries>,
) -> Result<(), CompositeError> {
    // Locators in a refusal are relative to the component root: doctor
    // renders them, and an absolute path would name the operator's home.
    let here = match relative.is_empty() {
        true => "the component directory".to_string(),
        false => format!("'{relative}'"),
    };
    let entries = read_dir(dir)
        .map_err(|error| CompositeError::Component(format!("{here} cannot be read: {error}")))?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            CompositeError::Component(format!("{here} yielded an unreadable entry: {error}"))
        })?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            return Err(CompositeError::Component(format!(
                "{here} holds an entry whose name is not UTF-8"
            )));
        };
        let path = match relative.is_empty() {
            true => name.to_string(),
            false => format!("{relative}/{name}"),
        };
        let metadata = std::fs::symlink_metadata(entry.path())
            .map_err(|error| CompositeError::Component(format!("'{path}': {error}")))?;
        let file_type = metadata.file_type();
        if file_type.is_symlink() {
            return Err(CompositeError::Component(format!("'{path}' is a symlink")));
        }
        if file_type.is_dir() {
            // The one permitted extra entry, granted on the metadata
            // already read: direct, a real directory and not a symlink.
            if relative.is_empty() && name == "node_modules" {
                continue;
            }
            let ancestor = format!("{path}/");
            if !expected.iter().any(|file| file.starts_with(&ancestor)) {
                return Err(CompositeError::Component(format!(
                    "unexpected directory '{path}'"
                )));
            }
            walk(&entry.path(), &path, expected, found, read_dir)?;
            continue;
        }
        if !file_type.is_file() {
            return Err(CompositeError::Component(format!(
                "'{path}' is neither a file nor a directory"
            )));
        }
        if !expected.contains(&path.as_str()) {
            return Err(CompositeError::Component(format!(
                "unexpected entry '{path}'"
            )));
        }
        let bytes = std::fs::read(entry.path())
            .map_err(|error| CompositeError::Component(format!("'{path}': {error}")))?;
        found.insert(path, sha256_hex(&bytes));
    }
    Ok(())
}

/// The component digest over an already-observed file-digest map: the
/// SHA-256 of the `<relative path>\0<file SHA-256>\n` lines in bytewise
/// path order, which is the order a `BTreeMap` keyed by those paths
/// already yields. The map is the single read of those files, so the
/// component and every line derived from it describe one observation.
fn component_digest(digests: &BTreeMap<String, String>) -> String {
    let mut lines = String::new();
    for (path, digest) in digests {
        lines.push_str(path);
        lines.push('\0');
        lines.push_str(digest);
        lines.push('\n');
    }
    sha256_text(&lines)
}

fn valid_component(component: &str) -> bool {
    !component.is_empty()
        && component != "."
        && component != ".."
        && !component.contains('/')
        && !component.contains('@')
        && !component.contains('\\')
        && !component.contains('\0')
        && !component.chars().any(char::is_whitespace)
}

/// The npm hidden-lock key-to-name rule (design D6, 2026-09-12 finding 1).
///
/// A key is one or more `node_modules/<package>` groups separated by `/`.
/// Each `node_modules/` consumes either one unscoped component or exactly
/// `@<scope>/<name>`, whose embedded `/` belongs to that scoped package.
/// Every group is parsed and validated; only the final group's complete
/// package spelling is the name. An optional `name` field is never read.
fn npm_name(key: &str) -> Result<String, CompositeError> {
    let unreadable = |why: &str| CompositeError::NpmKey(format!("'{key}': {why}"));
    if key.is_empty()
        || key.starts_with('/')
        || key.ends_with('/')
        || key.contains('\\')
        || key.contains('\0')
        || key.chars().any(char::is_whitespace)
    {
        return Err(unreadable(
            "empty, absolute, trailing, or carries a forbidden byte",
        ));
    }
    let mut rest = key;
    loop {
        let after = rest
            .strip_prefix("node_modules/")
            .ok_or_else(|| unreadable("expected a 'node_modules/' group"))?;
        let (package, tail) = if let Some(scoped) = after.strip_prefix('@') {
            let slash = scoped
                .find('/')
                .ok_or_else(|| unreadable("scoped package has no '/'"))?;
            let scope = &scoped[..slash];
            let remainder = &scoped[slash + 1..];
            let name_end = remainder.find('/').unwrap_or(remainder.len());
            let name = &remainder[..name_end];
            if !valid_component(scope) || !valid_component(name) {
                return Err(unreadable("invalid scope or name component"));
            }
            (format!("@{scope}/{name}"), &remainder[name_end..])
        } else {
            let name_end = after.find('/').unwrap_or(after.len());
            let name = &after[..name_end];
            if !valid_component(name) {
                return Err(unreadable("invalid unscoped component"));
            }
            (name.to_string(), &after[name_end..])
        };
        if tail.is_empty() {
            return Ok(package);
        }
        // Both arms above set `tail` to the slice beginning at the `/` they
        // just located, so a nonempty tail always begins with the separator;
        // the old `ok_or_else` refusal could never fire.
        rest = tail
            .strip_prefix('/')
            .expect("a nonempty package tail begins at a group separator");
    }
}

/// Every dependency triple in the core's retained hidden lock, normalized
/// to `name version integrity` from each entry's own key and fields,
/// deduplicated and sorted bytewise.
///
/// Exclusions identify exact RECORDS, never package names: the core's own
/// hidden-lock key, and a local `file:` record for a name whose installed
/// bytes already supply a component line. A same-named registry record is
/// retained, and so is a second record of the same name at a different
/// version or integrity — collapsing by name would erase a real
/// dependency.
fn npm_dependencies(lock: &Value, local: &[&str]) -> Result<Vec<String>, CompositeError> {
    let packages = lock
        .get("packages")
        .and_then(Value::as_object)
        .ok_or_else(|| CompositeError::NpmLock("no 'packages' object".into()))?;
    let mut triples = BTreeSet::new();
    for (key, entry) in packages {
        // Every group of every key is validated, including the excluded
        // records': a lock this reader cannot spell is not a lock it may
        // report a partial answer from.
        let name = npm_name(key)?;
        let entry = entry
            .as_object()
            .ok_or_else(|| CompositeError::NpmLock(format!("'{key}': entry is not an object")))?;
        if key == CORE_KEY {
            continue;
        }
        let resolved = entry.get("resolved").and_then(Value::as_str).unwrap_or("");
        if local.contains(&name.as_str()) && resolved.starts_with("file:") {
            continue;
        }
        let version = entry
            .get("version")
            .and_then(Value::as_str)
            .ok_or_else(|| CompositeError::NpmLock(format!("'{key}': no string 'version'")))?;
        if let Some(reason) = scalar_reason(version) {
            return Err(CompositeError::NpmLock(format!(
                "'{key}': version {reason}"
            )));
        }
        let integrity = entry
            .get("integrity")
            .and_then(Value::as_str)
            .ok_or_else(|| CompositeError::NpmLock(format!("'{key}': no registry 'integrity'")))?;
        if let Some(reason) = scalar_reason(integrity) {
            return Err(CompositeError::NpmLock(format!(
                "'{key}': integrity {reason}"
            )));
        }
        triples.insert(format!("{name} {version} {integrity}"));
    }
    Ok(triples.into_iter().collect())
}

fn pnpm_indent(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

/// The byte offset of the `@` that separates a pnpm package key's name
/// from its version: the first `@` after the key's first CHARACTER, so a
/// scoped `@scope/name@1.0.0` skips its leading one. Walked by character
/// rather than sliced at byte one, because a key whose first character is
/// multibyte would make `key[1..]` a panic inside a driver rather than a
/// refusal. `None` for a key with no room for one.
fn scoped_at(key: &str) -> Option<usize> {
    key.char_indices()
        .skip(1)
        .find_map(|(index, c)| (c == '@').then_some(index))
}

/// Read the profile's pnpm lock through the inclusive `PNPM_LIMIT` bound.
///
/// At most one byte beyond the limit is read, which is exactly enough to
/// tell "the largest admissible lock" from "too large": no metadata size
/// is trusted and no unbounded string is allocated first.
fn read_pnpm(path: &Path) -> Result<String, CompositeError> {
    let unreadable =
        |reason: String| CompositeError::PnpmLock(format!("{}: {reason}", path.display()));
    let file = std::fs::File::open(path).map_err(|error| unreadable(error.to_string()))?;
    let mut bytes = Vec::new();
    file.take(PNPM_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| unreadable(error.to_string()))?;
    if bytes.len() > PNPM_LIMIT {
        return Err(CompositeError::PnpmLock(format!(
            "pnpm lock exceeds {PNPM_LIMIT}-byte limit"
        )));
    }
    String::from_utf8(bytes).map_err(|_| unreadable("not UTF-8".to_string()))
}

/// Every dependency triple in a pnpm lockfile-9.0 `packages:` section, parsed
/// by the bounded, fail-closed line reader design D6 states (no YAML crate).
/// `local` names the packages whose installed bytes already supply a
/// component line; only their local `file:` records are excluded, so a
/// same-named registry record is retained.
fn pnpm_dependencies(lock: &str, local: &[&str]) -> Result<Vec<String>, CompositeError> {
    let bad = |why: &str| CompositeError::PnpmLock(why.to_string());
    if lock.contains('\t') {
        return Err(bad("a tab"));
    }
    let mut lines = lock.lines().enumerate().peekable();
    // The first non-blank line is the lockfile version.
    let mut version_seen = false;
    for (_, line) in lines.by_ref() {
        if line.trim().is_empty() {
            continue;
        }
        let version = line
            .strip_prefix("lockfileVersion:")
            .map(str::trim)
            .ok_or_else(|| bad("no lockfileVersion header"))?;
        let version = version.trim_matches('\'').trim_matches('"');
        if version != "9.0" {
            return Err(bad("lockfileVersion is not 9.0"));
        }
        version_seen = true;
        break;
    }
    if !version_seen {
        return Err(bad("empty document"));
    }

    let mut triples = BTreeSet::new();
    let mut in_packages = false;
    // The current entry: (key, resolution integrity).
    let mut entry: Option<(String, Option<String>)> = None;

    let flush = |entry: Option<(String, Option<String>)>,
                 triples: &mut BTreeSet<String>|
     -> Result<(), CompositeError> {
        let Some((key, integrity)) = entry else {
            return Ok(());
        };
        let integrity = integrity
            .ok_or_else(|| CompositeError::PnpmLock(format!("'{key}': no resolution integrity")))?;
        // Every entry admitted into `flush` was already checked to carry an
        // `@` after its first character, so the lookup always finds one.
        let at = scoped_at(&key).expect("a package key carries an '@' after its first character");
        let name = &key[..at];
        let version = &key[at + 1..];
        // `at` is at least one byte, so the name is never empty; only an
        // empty version is reachable from an admitted key.
        if version.is_empty() {
            return Err(CompositeError::PnpmLock(format!(
                "'{key}': empty name or version"
            )));
        }
        // The local tarball record, excluded by its own `file:` version
        // rather than by its name: a registry record spelled with the
        // same name is a real dependency and stays.
        if local.contains(&name) && version.starts_with("file:") {
            return Ok(());
        }
        if let Some(reason) = scalar_reason(integrity.as_str()) {
            return Err(CompositeError::PnpmLock(format!(
                "'{key}': integrity {reason}"
            )));
        }
        triples.insert(format!("{name} {version} {integrity}"));
        Ok(())
    };

    for (_, line) in lines {
        if line.trim().is_empty() {
            continue;
        }
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') || trimmed.starts_with("---") || trimmed.starts_with("...") {
            return Err(bad("a comment or document marker"));
        }
        let indent = pnpm_indent(line);
        if indent == 0 {
            flush(entry.take(), &mut triples)?;
            in_packages = line.trim_end() == "packages:";
            continue;
        }
        if !in_packages {
            continue;
        }
        if indent == 2 {
            flush(entry.take(), &mut triples)?;
            let body = line.trim();
            let key = body
                .strip_suffix(':')
                .ok_or_else(|| bad("a package key is not colon-terminated"))?;
            let key = key.trim_matches('\'').trim_matches('"').to_string();
            if scoped_at(&key).is_none() {
                return Err(bad("a package key has no '@' after its first character"));
            }
            entry = Some((key, None));
            continue;
        }
        // A child line at four or more spaces. Only the resolution row is read.
        let body = line.trim();
        if let Some(rest) = body.strip_prefix("resolution:") {
            let rest = rest.trim();
            let inner = rest
                .strip_prefix('{')
                .and_then(|text| text.strip_suffix('}'))
                .ok_or_else(|| bad("a block-form or malformed resolution"))?;
            if !inner.contains("integrity:") {
                return Err(bad("a resolution without integrity"));
            }
            let after = inner.split("integrity:").nth(1).unwrap_or("").trim();
            let value = after.split(',').next().unwrap_or("").trim();
            let value = value.trim_matches('\'').trim_matches('"');
            if value.is_empty() {
                return Err(bad("an empty resolution integrity"));
            }
            let Some((key, slot)) = entry.as_mut() else {
                return Err(bad("a resolution outside any entry"));
            };
            if slot.is_some() {
                return Err(bad(&format!("'{key}': repeated resolution")));
            }
            *slot = Some(value.to_string());
        }
    }
    flush(entry.take(), &mut triples)?;
    Ok(triples.into_iter().collect())
}

/// The canonical composite's fixed `<component>\0<value>\n` lines, hashed as
/// 64 lowercase hexadecimal characters. Dependencies from both lock dialects
/// are merged, deduplicated and sorted bytewise; the extension line appears
/// only when the profile names the conditional extension.
///
/// This is the serializer alone. Every scalar reaching it was validated at
/// the source it was read from, so the separators it introduces — the
/// spaces inside a `core` or `dependency` value, the NUL and the newline
/// between them — are the only ones in the stream.
#[allow(clippy::too_many_arguments)]
fn canonical_composite(
    core: &str,
    node: &str,
    npm: &[String],
    pnpm: &[String],
    plugin: &str,
    plugin_patch: &str,
    profile_patch: &str,
    profile_bundles: &[String],
    profile_patch_reload: &str,
    home_patch: &str,
    extension: Option<&str>,
) -> String {
    let mut dependencies: BTreeSet<&str> = BTreeSet::new();
    dependencies.extend(npm.iter().map(String::as_str));
    dependencies.extend(pnpm.iter().map(String::as_str));

    let mut lines = String::new();
    let mut push = |component: &str, value: &str| {
        lines.push_str(component);
        lines.push('\0');
        lines.push_str(value);
        lines.push('\n');
    };
    push("core", core);
    push("node", node);
    for dependency in dependencies {
        push("dependency", dependency);
    }
    push("plugin", plugin);
    push("plugin-patch", plugin_patch);
    push("profile-patch", profile_patch);
    for bundle in profile_bundles {
        push("profile-bundle", bundle);
    }
    push("profile-patch-reload", profile_patch_reload);
    push("home-patch", home_patch);
    if let Some(extension) = extension {
        push("extension", extension);
    }
    sha256_text(&lines)
}

/// The two seams the DSH adapter resolves, exactly as it resolves them:
/// the executable through `BROKKR_DSH_BIN`, then `FORGE_DSH_BIN`, then
/// `dsh` on `PATH`, and the home through `$DSH_HOME` when set and
/// non-empty, otherwise `$HOME/.dsh`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DshSeams {
    pub executable: String,
    pub home: PathBuf,
}

impl DshSeams {
    pub fn resolve() -> Result<DshSeams, CompositeError> {
        DshSeams::selected().1
    }

    /// Both seams from ONE environment resolution: the executable the
    /// adapter selected, and the home beside it.
    ///
    /// The executable is a selection, not a lookup, so it always exists;
    /// only the home can fail. A caller that must keep observing the
    /// chosen installation after a home failure — doctor, whose version
    /// probe is independent of the composite (design D10's doctor state
    /// decision) — reads it here rather than resolving the environment a
    /// second time and possibly selecting a different install.
    pub fn selected() -> (String, Result<DshSeams, CompositeError>) {
        let executable = super::adapter_binary("BROKKR_DSH_BIN", Some("FORGE_DSH_BIN"), "dsh");
        let seams = DshSeams::resolve_with(executable.clone(), crate::transcript::dsh_home());
        (executable, seams)
    }

    /// `resolve` over an injected executable and home, so the missing-home
    /// refusal is a plain test.
    fn resolve_with(executable: String, home: Option<PathBuf>) -> Result<DshSeams, CompositeError> {
        let home =
            home.ok_or_else(|| CompositeError::Config("no dsh home: set DSH_HOME or HOME".into()))?;
        Ok(DshSeams { executable, home })
    }
}

/// The Node runtime the executable's `env node` shebang looks up: the
/// first `node` on the child environment's `PATH`, and the single line
/// `node --version` prints.
#[derive(Debug, Clone, PartialEq, Eq)]
struct NodeRuntime {
    path: PathBuf,
    version: String,
}

/// The canonical composite and the raw component values that produced
/// it, so a caller can report the drifted component rather than only the
/// final digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DshComposite {
    pub canonical: String,
    pub core: String,
    pub node: String,
    pub plugin: String,
    pub dependencies: Vec<String>,
    pub plugin_patch: String,
    pub profile_patch: String,
    pub profile_bundles: Vec<String>,
    pub profile_patch_reload: String,
    pub home_patch: String,
    pub extension: Option<String>,
    pub core_root: PathBuf,
    pub profile: PathBuf,
}

fn read_json(path: &Path) -> Result<Value, CompositeError> {
    let bytes = std::fs::read(path)
        .map_err(|error| CompositeError::Config(format!("{}: {error}", path.display())))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| CompositeError::Config(format!("{}: not JSON: {error}", path.display())))
}

fn canonicalize(path: &Path) -> Result<PathBuf, CompositeError> {
    std::fs::canonicalize(path)
        .map_err(|error| CompositeError::Config(format!("{}: {error}", path.display())))
}

fn required_string<'a>(
    value: &'a Value,
    key: &str,
    where_: &str,
) -> Result<&'a str, CompositeError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| CompositeError::Config(format!("{where_}: missing string '{key}'")))
}

fn first_line(path: &Path) -> Result<String, CompositeError> {
    let bytes = std::fs::read(path)
        .map_err(|error| CompositeError::Config(format!("{}: {error}", path.display())))?;
    let end = bytes
        .iter()
        .position(|byte| *byte == b'\n')
        .unwrap_or(bytes.len());
    let mut line = &bytes[..end];
    if line.ends_with(b"\r") {
        line = &line[..line.len() - 1];
    }
    String::from_utf8(line.to_vec())
        .map_err(|_| CompositeError::Config(format!("{}: first line is not UTF-8", path.display())))
}

/// Resolve `command` to a canonical path: a command carrying a separator
/// is used directly, otherwise the first executable on `PATH` wins.
fn resolve_executable(command: &str) -> Result<PathBuf, CompositeError> {
    resolve_executable_in(command, std::env::var_os("PATH"))
}

/// `resolve_executable` over an injected `PATH`, so an empty entry, a
/// present-but-absent candidate and a miss are plain tests.
fn resolve_executable_in(
    command: &str,
    path: Option<std::ffi::OsString>,
) -> Result<PathBuf, CompositeError> {
    if command.contains('/') || command.contains('\\') {
        return canonicalize(Path::new(command));
    }
    let path = path.unwrap_or_default();
    for dir in std::env::split_paths(&path) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let candidate = dir.join(command);
        if candidate.is_file() {
            return canonicalize(&candidate);
        }
    }
    Err(CompositeError::Config(format!(
        "'{command}' is not on PATH"
    )))
}

struct CorePackage {
    root: PathBuf,
    dir: PathBuf,
    version: String,
    integrity: String,
    /// The hidden lock, parsed once: it supplies both the `core` line's
    /// version and integrity and every npm dependency triple, so nothing
    /// below reopens it.
    lock: Value,
}

/// Find the core package from the canonical executable: the nearest
/// ancestor whose `package.json` names `@deepseek-ai/dsh`, whose
/// `bin.dsh` target is that executable with an `env node` first line and
/// which sits at `<core root>/node_modules/@deepseek-ai/dsh`. The core
/// lock is the hidden `<core root>/node_modules/.package-lock.json`,
/// retained here as the sole read of those bytes.
fn resolve_core(executable: &str) -> Result<CorePackage, CompositeError> {
    resolve_core_reading(executable, &read_json)
}

/// `resolve_core` over an injected JSON reader, so "each identity-bearing
/// source is opened once" is a counted claim rather than an assertion in
/// a comment. Only the source of the parse is injectable; the real reader
/// stays production's.
fn resolve_core_reading(
    executable: &str,
    read_json: &dyn Fn(&Path) -> Result<Value, CompositeError>,
) -> Result<CorePackage, CompositeError> {
    let canonical = resolve_executable(executable)?;
    let mut package = None;
    for ancestor in canonical.ancestors().skip(1) {
        let path = ancestor.join("package.json");
        if !path.is_file() {
            continue;
        }
        if let Ok(value) = read_json(&path) {
            if value.get("name").and_then(Value::as_str) == Some(CORE_NAME) {
                // Retained from discovery: the manifest is read once and
                // every later question is asked of this value.
                package = Some((ancestor.to_path_buf(), value));
                break;
            }
        }
    }
    let (dir, manifest) = package.ok_or_else(|| {
        CompositeError::Config(format!(
            "{}: no ancestor package.json names {CORE_NAME}",
            canonical.display()
        ))
    })?;
    let bin = manifest
        .get("bin")
        .and_then(|bin| bin.get("dsh"))
        .and_then(Value::as_str)
        .ok_or_else(|| CompositeError::Config("core package has no bin.dsh".into()))?;
    let bin_path = canonicalize(&dir.join(bin))?;
    if bin_path != canonical {
        return Err(CompositeError::Config(format!(
            "{} is not the core package's bin.dsh ({})",
            canonical.display(),
            bin_path.display()
        )));
    }
    if first_line(&canonical)? != "#!/usr/bin/env node" {
        return Err(CompositeError::Config(format!(
            "{}: first line is not the env node shebang",
            canonical.display()
        )));
    }
    let scope = dir
        .parent()
        .filter(|parent| {
            parent
                .file_name()
                .is_some_and(|name| name == "@deepseek-ai")
        })
        .ok_or_else(|| {
            CompositeError::Config("core package is not under node_modules/@deepseek-ai".into())
        })?;
    let node_modules = scope
        .parent()
        .filter(|parent| {
            parent
                .file_name()
                .is_some_and(|name| name == "node_modules")
        })
        .ok_or_else(|| CompositeError::Config("core package is not under node_modules".into()))?;
    // The filter above admitted only a path whose file name is
    // `node_modules`, and a path with a file name always has a parent, so
    // the old "core root is missing" refusal could never fire.
    let root = node_modules
        .parent()
        .expect("a path named node_modules has a parent");
    if root.join("node_modules").join("@deepseek-ai").join("dsh") != dir {
        return Err(CompositeError::Config(
            "core package is not at <core root>/node_modules/@deepseek-ai/dsh".into(),
        ));
    }
    let package_version = required_string(&manifest, "version", "core package")?;
    let lock = read_json(&root.join("node_modules").join(".package-lock.json"))?;
    let entry = lock
        .get("packages")
        .and_then(|packages| packages.get(CORE_KEY))
        .ok_or_else(|| CompositeError::Config(format!("core lock has no {CORE_KEY} entry")))?;
    let version = required_string(entry, "version", "core lock")?;
    if version != package_version {
        return Err(CompositeError::Config(format!(
            "core lock version {version} differs from package version {package_version}"
        )));
    }
    let integrity = required_string(entry, "integrity", "core lock")?;
    scalar("core", version)?;
    scalar("core", integrity)?;
    let (version, integrity) = (version.to_string(), integrity.to_string());
    Ok(CorePackage {
        root: root.to_path_buf(),
        dir,
        version,
        integrity,
        lock,
    })
}

struct Profile {
    /// The original lookup anchor: the raw `<home>/profiles/headless`
    /// path. The loader's Node search order is unchanged, so this is what
    /// `resolve_bundle` searches from. It is never used for containment: a
    /// symlinked home ancestor would make raw-prefix comparison refuse a
    /// legitimate install (design D6, council return 2026-09-13).
    dir: PathBuf,
    /// The canonical `<home>/profiles/headless` directory, taken once at
    /// read time. Every containment decision compares canonical
    /// candidates against this boundary, never against the raw anchor.
    canonical: PathBuf,
    bundles: Vec<String>,
    patch_reload: String,
}

/// `<home>/profiles/headless/package.json`, of which only
/// `dsh.profile.bundles` (a non-empty string array) and
/// `dsh.profile.patchReload` (`live` or `startup`) are read. The complete
/// profile directory is canonicalized once here and retained as the
/// containment boundary, separate from the raw lookup anchor.
fn read_profile(home: &Path) -> Result<Profile, CompositeError> {
    let dir = home.join("profiles").join("headless");
    let canonical = canonicalize(&dir)?;
    let manifest = read_json(&dir.join("package.json"))?;
    let profile = manifest
        .get("dsh")
        .and_then(|dsh| dsh.get("profile"))
        .ok_or_else(|| CompositeError::Config("profile manifest has no dsh.profile".into()))?;
    let bundles = profile
        .get("bundles")
        .and_then(Value::as_array)
        .filter(|array| !array.is_empty())
        .ok_or_else(|| {
            CompositeError::Config("dsh.profile.bundles must be a non-empty array".into())
        })?;
    let mut names = Vec::with_capacity(bundles.len());
    for bundle in bundles {
        let name = bundle.as_str().ok_or_else(|| {
            CompositeError::Config("a dsh.profile.bundles entry is not a string".into())
        })?;
        scalar("profile-bundle", name)?;
        names.push(name.to_string());
    }
    let patch_reload = required_string(profile, "patchReload", "dsh.profile")?;
    if patch_reload != "live" && patch_reload != "startup" {
        return Err(CompositeError::Config(format!(
            "dsh.profile.patchReload '{patch_reload}' is neither live nor startup"
        )));
    }
    Ok(Profile {
        dir,
        canonical,
        bundles: names,
        patch_reload: patch_reload.to_string(),
    })
}

/// Node's `NODE_MODULES_PATHS` from a package directory: every ancestor
/// `node_modules/` candidate, deepest first, skipping a component that is
/// itself named `node_modules`.
fn node_modules_paths(start: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for ancestor in start.ancestors() {
        if ancestor
            .file_name()
            .is_some_and(|name| name == "node_modules")
        {
            continue;
        }
        paths.push(ancestor.join("node_modules"));
    }
    paths
}

/// The runtime prefix of a `node` executable sitting in a `bin/`
/// directory; `None` when the layout is not the standard one.
fn node_prefix(node: &Path) -> Option<PathBuf> {
    let bin = node.parent()?;
    if bin.file_name().is_some_and(|name| name == "bin") {
        bin.parent().map(Path::to_path_buf)
    } else {
        None
    }
}

/// Node's global folders, as the child environment names them:
/// `NODE_PATH` entries, `$HOME/.node_modules`, `$HOME/.node_libraries`
/// and `lib/node` under the runtime's prefix.
fn global_folders(node: &NodeRuntime) -> Vec<PathBuf> {
    global_folders_in(
        node,
        std::env::var_os("NODE_PATH"),
        std::env::var_os("HOME"),
    )
}

/// `global_folders` over injected `NODE_PATH` and `HOME`, so an absent or
/// empty variable is a plain test.
fn global_folders_in(
    node: &NodeRuntime,
    node_path: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
) -> Vec<PathBuf> {
    let mut folders = Vec::new();
    if let Some(path) = node_path {
        folders.extend(std::env::split_paths(&path).filter(|entry| !entry.as_os_str().is_empty()));
    }
    if let Some(home) = home {
        let home = PathBuf::from(home);
        folders.push(home.join(".node_modules"));
        folders.push(home.join(".node_libraries"));
    }
    if let Some(prefix) = node_prefix(&node.path) {
        folders.push(prefix.join("lib").join("node"));
    }
    folders
}

/// Emulate `resolveBundleDir`: the core package's Node lookup, the
/// global folders, then the profile's Node lookup. The first candidate
/// holding a `package.json` wins, and its canonical directory must lie
/// inside the canonical core root or the canonical profile boundary. The
/// profile lookup ANCHOR is the raw directory, so an alias in the home's
/// ancestry does not move the search order; containment alone is decided
/// on canonical paths. An outside first hit is a refusal, never skipped
/// for a later inside candidate.
fn resolve_bundle(
    name: &str,
    core_dir: &Path,
    profile_dir: &Path,
    globals: &[PathBuf],
    core_root: &Path,
    profile_boundary: &Path,
) -> Result<PathBuf, CompositeError> {
    let mut candidates = node_modules_paths(core_dir);
    candidates.extend(globals.iter().cloned());
    candidates.extend(node_modules_paths(profile_dir));
    candidates.extend(globals.iter().cloned());
    for candidate in candidates {
        let dir = candidate.join(name);
        if !dir.join("package.json").is_file() {
            continue;
        }
        let canonical = canonicalize(&dir)?;
        if canonical.starts_with(core_root) || canonical.starts_with(profile_boundary) {
            return Ok(canonical);
        }
        return Err(CompositeError::Config(format!(
            "bundle '{name}' resolves outside the core root and the profile ({})",
            canonical.display()
        )));
    }
    Err(CompositeError::Config(format!(
        "bundle '{name}' does not resolve"
    )))
}

/// Spawn the first `node` on `PATH` once for its version line.
fn spawn_node_runtime() -> Result<NodeRuntime, CompositeError> {
    let path = resolve_executable("node")?;
    spawn_node_runtime_at(path)
}

/// `spawn_node_runtime` over an already-resolved executable, so a scripted
/// `node` exercises the success, nonzero and unreadable-version paths.
///
/// The output is read as exactly one record: at most one trailing line
/// terminator is removed, and what remains must satisfy the scalar rule.
/// `trim()` would repair a malformed banner into a plausible version, and
/// a repaired identity is the one thing this producer must never invent.
fn spawn_node_runtime_at(path: PathBuf) -> Result<NodeRuntime, CompositeError> {
    let output = std::process::Command::new(&path)
        .arg("--version")
        .output()
        .map_err(|error| CompositeError::Config(format!("node --version: {error}")))?;
    if !output.status.success() {
        return Err(CompositeError::Config(
            "node --version exited nonzero".into(),
        ));
    }
    let printed = String::from_utf8(output.stdout)
        .map_err(|_| CompositeError::Config("node --version did not print UTF-8".into()))?;
    let version = printed.strip_suffix('\n').unwrap_or(&printed);
    let version = version.strip_suffix('\r').unwrap_or(version);
    scalar("node", version)?;
    Ok(NodeRuntime {
        path,
        version: version.to_string(),
    })
}

/// The SHA-256 of one named patch component's bytes.
fn patch_digest(component: &'static str, path: &Path) -> Result<String, CompositeError> {
    let bytes = std::fs::read(path).map_err(|error| {
        CompositeError::Patch(component, format!("{}: {error}", path.display()))
    })?;
    Ok(sha256_hex(&bytes))
}

/// The home-level `$DSH_HOME/cordis.patch.yml` digest, or the literal
/// `absent`.
///
/// Only true missing-path evidence supplies `absent`. A dangling symlink,
/// a permission failure or any other metadata error is unreadable: a
/// failed observation is not absence, and reporting it as one would
/// silently pin an identity the home does not have.
fn home_patch(home: &Path) -> Result<String, CompositeError> {
    let path = home.join("cordis.patch.yml");
    match std::fs::symlink_metadata(&path) {
        Ok(_) => patch_digest("home-patch", &path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok("absent".to_string()),
        Err(error) => Err(CompositeError::Patch(
            "home-patch",
            format!("{}: {error}", path.display()),
        )),
    }
}

/// Compute the canonical composite over the resolved seams, with the
/// Node runtime and global folders injected so every branch is a plain
/// test. This is the only producer of either the plugin component or the
/// canonical composite.
fn dsh_composite_with(
    seams: &DshSeams,
    node: &NodeRuntime,
    globals: &[PathBuf],
) -> Result<DshComposite, CompositeError> {
    let core = resolve_core(&seams.executable)?;
    let profile = read_profile(&seams.home)?;
    let mut resolved: Vec<(String, PathBuf)> = Vec::new();
    for name in &profile.bundles {
        let dir = resolve_bundle(
            name,
            &core.dir,
            &profile.dir,
            globals,
            &core.root,
            &profile.canonical,
        )?;
        if name == PLUGIN_BUNDLE && !dir.starts_with(&profile.canonical) {
            return Err(CompositeError::Config(
                "the plugin resolves outside the profile".into(),
            ));
        }
        resolved.push((name.clone(), dir));
    }
    let plugin_dir = resolved
        .iter()
        .find(|(name, _)| name == PLUGIN_BUNDLE)
        .map(|(_, dir)| dir.clone())
        .ok_or_else(|| {
            CompositeError::Config(format!("the profile does not list {PLUGIN_BUNDLE}"))
        })?;
    // One read of the plugin's files serves both lines: the component
    // over all six and `plugin-patch` from the retained digest of
    // `cordis.patch.yml`. Reopening the patch could describe a file the
    // component never saw.
    let plugin_files = plugin_file_digests(&plugin_dir, &PLUGIN_FILES)?;
    let plugin_patch = plugin_files
        .get("cordis.patch.yml")
        .expect("the plugin file set is complete when the walk returns")
        .clone();
    let plugin = component_digest(&plugin_files);
    // Asked of `resolved` rather than of `profile.bundles`, because the
    // loop above resolves every listed bundle or returns: a name is in
    // `resolved` exactly when it is listed. Asking the list first and
    // then the resolution second spelled a "was not resolved" refusal
    // that no input could reach, and an unreachable guard is a claim the
    // code cannot keep.
    let extension = match resolved.iter().find(|(name, _)| name == EXTENSION_BUNDLE) {
        Some((_, dir)) => {
            if !dir.starts_with(&profile.canonical) {
                return Err(CompositeError::Config(
                    "the extension resolves outside the profile".into(),
                ));
            }
            let files = plugin_file_digests(dir, &EXTENSION_FILES)?;
            Some(component_digest(&files))
        }
        None => None,
    };
    // The names whose installed bytes already supply a component line, so
    // their LOCAL records — and only those — leave the dependency lines.
    let mut local = vec![PLUGIN_BUNDLE];
    if extension.is_some() {
        local.push(EXTENSION_BUNDLE);
    }
    let npm = npm_dependencies(&core.lock, &local)?;
    let pnpm = pnpm_dependencies(&read_pnpm(&profile.dir.join("pnpm-lock.yaml"))?, &local)?;
    let core_line = format!("{CORE_NAME} {} {}", core.version, core.integrity);
    let profile_patch = patch_digest("profile-patch", &profile.dir.join("cordis.patch.yml"))?;
    let home_patch = home_patch(&seams.home)?;
    let canonical = canonical_composite(
        &core_line,
        &node.version,
        &npm,
        &pnpm,
        &plugin,
        &plugin_patch,
        &profile_patch,
        &profile.bundles,
        &profile.patch_reload,
        &home_patch,
        extension.as_deref(),
    );
    let mut dependencies: BTreeSet<String> = BTreeSet::new();
    dependencies.extend(npm);
    dependencies.extend(pnpm);
    Ok(DshComposite {
        canonical,
        core: core_line,
        node: node.version.clone(),
        plugin,
        dependencies: dependencies.into_iter().collect(),
        plugin_patch,
        profile_patch,
        profile_bundles: profile.bundles,
        profile_patch_reload: profile.patch_reload,
        home_patch,
        extension,
        core_root: core.root,
        profile: profile.dir,
    })
}

/// The one production producer of either value (design D6 (b)): the
/// canonical composite and the component values behind it, over the
/// adapter's own resolved seams and a real `node --version` probe.
pub fn dsh_composite(seams: &DshSeams) -> Result<DshComposite, CompositeError> {
    dsh_composite_resolving(seams, spawn_node_runtime)
}

/// `dsh_composite` over an injected runtime probe, so the composition is a
/// plain test without a host `node`.
fn dsh_composite_resolving(
    seams: &DshSeams,
    spawn: impl FnOnce() -> Result<NodeRuntime, CompositeError>,
) -> Result<DshComposite, CompositeError> {
    let node = spawn()?;
    let globals = global_folders(&node);
    dsh_composite_with(seams, &node, &globals)
}

#[cfg(test)]
mod tests;
