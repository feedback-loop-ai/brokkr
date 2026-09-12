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
//! Nothing here reads a credential or settings file, and no provenance note,
//! probe script or evidence file computes either value by hand.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::Value;
use sha2::{Digest, Sha256};

/// The plugin's committed six-file set, in bytewise path order.
pub const PLUGIN_FILES: [&str; 6] = [
    "LICENSE",
    "README.md",
    "cordis.patch.yml",
    "lib/index.js",
    "lib/startup.js",
    "package.json",
];

/// The conditional extension's four-file set, in bytewise path order.
pub const EXTENSION_FILES: [&str; 4] = ["LICENSE", "cordis.patch.yml", "index.js", "package.json"];

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
    #[error("a composite value contains a NUL or newline")]
    Value,
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn sha256_file(path: &Path) -> Result<String, CompositeError> {
    let bytes = std::fs::read(path)
        .map_err(|error| CompositeError::Component(format!("{}: {error}", path.display())))?;
    Ok(sha256_hex(&bytes))
}

fn sha256_text(text: &str) -> String {
    sha256_hex(text.as_bytes())
}

/// Every regular file beneath `dir`, as a relative path with `/` separators
/// mapped to its SHA-256. A nested `node_modules/` subtree is excluded
/// because the dependency identity already covers it. A symlink, an
/// unreadable entry or an unreadable file makes the component unreadable and
/// names the offending path.
pub fn plugin_file_digests(dir: &Path) -> Result<BTreeMap<String, String>, CompositeError> {
    let mut found = BTreeMap::new();
    walk(dir, dir, &mut found)?;
    Ok(found)
}

fn walk(
    root: &Path,
    current: &Path,
    found: &mut BTreeMap<String, String>,
) -> Result<(), CompositeError> {
    let entries = std::fs::read_dir(current)
        .map_err(|error| CompositeError::Component(format!("{}: {error}", current.display())))?;
    for entry in entries {
        let entry = entry
            .map_err(|error| CompositeError::Component(format!("{}: {error}", root.display())))?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == "node_modules" {
            continue;
        }
        let metadata = std::fs::symlink_metadata(&path)
            .map_err(|error| CompositeError::Component(format!("{}: {error}", path.display())))?;
        let file_type = metadata.file_type();
        if file_type.is_symlink() {
            return Err(CompositeError::Component(format!(
                "{} is a symlink",
                path.display()
            )));
        }
        if file_type.is_dir() {
            walk(root, &path, found)?;
        } else if file_type.is_file() {
            let relative = path
                .strip_prefix(root)
                .expect("a walked path is beneath its root")
                .to_string_lossy()
                .replace('\\', "/");
            found.insert(relative, sha256_file(&path)?);
        } else {
            return Err(CompositeError::Component(format!(
                "{} is neither a file nor a directory",
                path.display()
            )));
        }
    }
    Ok(())
}

/// The plugin component over `dir`: the SHA-256 of the `<relative
/// path>\0<file SHA-256>\n` lines for exactly the `expected` set, in bytewise
/// path order. A missing expected file, or any found file outside the
/// expected set, is unreadable and names the drifted path.
pub fn plugin_component(dir: &Path, expected: &[&str]) -> Result<String, CompositeError> {
    let found = plugin_file_digests(dir)?;
    let expected_set: BTreeSet<&str> = expected.iter().copied().collect();
    if let Some(extra) = found
        .keys()
        .find(|path| !expected_set.contains(path.as_str()))
    {
        return Err(CompositeError::Component(format!(
            "unexpected entry '{extra}'"
        )));
    }
    let mut ordered: Vec<&str> = expected.to_vec();
    ordered.sort_unstable();
    let mut lines = String::new();
    for path in ordered {
        let digest = found
            .get(path)
            .ok_or_else(|| CompositeError::Component(format!("missing expected file '{path}'")))?;
        lines.push_str(path);
        lines.push('\0');
        lines.push_str(digest);
        lines.push('\n');
    }
    Ok(sha256_text(&lines))
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
pub fn npm_name(key: &str) -> Result<String, CompositeError> {
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
        rest = tail
            .strip_prefix('/')
            .ok_or_else(|| unreadable("expected a group separator"))?;
    }
}

/// Every dependency triple in an npm lockfile-3 hidden lock, normalized to
/// `name version integrity` from each entry's own key and fields, deduplicated
/// and sorted bytewise. `excluded` names the provider's own entries (the core
/// and the local tarball installs), which are not registry dependencies.
pub fn npm_dependencies(lock: &str, excluded: &[&str]) -> Result<Vec<String>, CompositeError> {
    let value: Value = serde_json::from_str(lock)
        .map_err(|error| CompositeError::NpmLock(format!("not JSON: {error}")))?;
    let packages = value
        .get("packages")
        .and_then(Value::as_object)
        .ok_or_else(|| CompositeError::NpmLock("no 'packages' object".into()))?;
    let mut triples = BTreeSet::new();
    for (key, entry) in packages {
        let name = npm_name(key)?;
        if excluded.contains(&name.as_str()) {
            continue;
        }
        let entry = entry
            .as_object()
            .ok_or_else(|| CompositeError::NpmLock(format!("'{key}': entry is not an object")))?;
        let version = entry
            .get("version")
            .and_then(Value::as_str)
            .ok_or_else(|| CompositeError::NpmLock(format!("'{key}': no string 'version'")))?;
        if version.is_empty() || version.contains('\0') || version.contains('\n') {
            return Err(CompositeError::NpmLock(format!("'{key}': invalid version")));
        }
        let integrity = entry
            .get("integrity")
            .and_then(Value::as_str)
            .ok_or_else(|| CompositeError::NpmLock(format!("'{key}': no registry 'integrity'")))?;
        if integrity.is_empty() || integrity.contains('\0') || integrity.contains('\n') {
            return Err(CompositeError::NpmLock(format!(
                "'{key}': invalid integrity"
            )));
        }
        triples.insert(format!("{name} {version} {integrity}"));
    }
    Ok(triples.into_iter().collect())
}

fn pnpm_indent(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

/// Every dependency triple in a pnpm lockfile-9.0 `packages:` section, parsed
/// by the bounded, fail-closed line reader design D6 states (no YAML crate).
/// `excluded` names the local-tarball entries, which are not registry
/// identity.
pub fn pnpm_dependencies(lock: &str, excluded: &[&str]) -> Result<Vec<String>, CompositeError> {
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
        let at = key[1..]
            .find('@')
            .map(|index| index + 1)
            .ok_or_else(|| CompositeError::PnpmLock(format!("'{key}': key has no '@'")))?;
        let name = &key[..at];
        let version = &key[at + 1..];
        if name.is_empty() || version.is_empty() {
            return Err(CompositeError::PnpmLock(format!(
                "'{key}': empty name or version"
            )));
        }
        if !excluded.contains(&name) {
            triples.insert(format!("{name} {version} {integrity}"));
        }
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
            if key.len() < 2 || !key[1..].contains('@') {
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
#[allow(clippy::too_many_arguments)]
pub fn canonical_composite(
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
) -> Result<String, CompositeError> {
    let values = [
        core,
        node,
        plugin,
        plugin_patch,
        profile_patch,
        profile_patch_reload,
        home_patch,
    ];
    if values
        .into_iter()
        .chain(profile_bundles.iter().map(String::as_str))
        .chain(extension)
        .any(|value| value.contains('\0') || value.contains('\n'))
    {
        return Err(CompositeError::Value);
    }
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
    Ok(sha256_text(&lines))
}

#[cfg(test)]
mod tests;
