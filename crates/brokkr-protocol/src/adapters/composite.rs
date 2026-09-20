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
    /// A file-set component that could not be observed, named by the
    /// component it serves — `plugin` or `extension`. The name is carried
    /// rather than fixed because both walks share one reader, and
    /// reporting extension drift as plugin drift sends an operator to the
    /// wrong directory (council return 2026-09-19, F9).
    #[error("{0} component is unreadable: {1}")]
    Component(&'static str, String),
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
    component: &'static str,
    dir: &Path,
    expected: &[&str],
    read_dir: &dyn Fn(&Path) -> std::io::Result<DirEntries>,
) -> Result<BTreeMap<String, String>, CompositeError> {
    let mut found = BTreeMap::new();
    walk(component, dir, "", expected, &mut found, read_dir)?;
    for path in expected {
        if !found.contains_key(*path) {
            return Err(CompositeError::Component(
                component,
                format!("missing expected file '{path}'"),
            ));
        }
    }
    Ok(found)
}

fn walk(
    component: &'static str,
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
    let entries = read_dir(dir).map_err(|error| {
        CompositeError::Component(component, format!("{here} cannot be read: {error}"))
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            CompositeError::Component(
                component,
                format!("{here} yielded an unreadable entry: {error}"),
            )
        })?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            return Err(CompositeError::Component(
                component,
                format!("{here} holds an entry whose name is not UTF-8"),
            ));
        };
        let path = match relative.is_empty() {
            true => name.to_string(),
            false => format!("{relative}/{name}"),
        };
        let metadata = std::fs::symlink_metadata(entry.path())
            .map_err(|error| CompositeError::Component(component, format!("'{path}': {error}")))?;
        let file_type = metadata.file_type();
        if file_type.is_symlink() {
            return Err(CompositeError::Component(
                component,
                format!("'{path}' is a symlink"),
            ));
        }
        if file_type.is_dir() {
            // The one permitted extra entry, granted on the metadata
            // already read: direct, a real directory and not a symlink.
            if relative.is_empty() && name == "node_modules" {
                continue;
            }
            let ancestor = format!("{path}/");
            if !expected.iter().any(|file| file.starts_with(&ancestor)) {
                return Err(CompositeError::Component(
                    component,
                    format!("unexpected directory '{path}'"),
                ));
            }
            walk(component, &entry.path(), &path, expected, found, read_dir)?;
            continue;
        }
        if !file_type.is_file() {
            return Err(CompositeError::Component(
                component,
                format!("'{path}' is neither a file nor a directory"),
            ));
        }
        if !expected.contains(&path.as_str()) {
            return Err(CompositeError::Component(
                component,
                format!("unexpected entry '{path}'"),
            ));
        }
        let bytes = std::fs::read(entry.path())
            .map_err(|error| CompositeError::Component(component, format!("'{path}': {error}")))?;
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

/// Whether a line is blank in THIS grammar: nothing but ASCII spaces.
/// `str::trim` read a line of U+00A0 as blank and skipped it, so a byte
/// this reader never admitted disappeared before any rule saw it.
fn pnpm_blank(line: &str) -> bool {
    line.trim_matches(' ').is_empty()
}

/// The value half of a `key: value` line, with only the grammar's own
/// ASCII padding consumed: the space YAML requires after the colon, and
/// trailing spaces. What remains is the scalar EXACTLY as spelled — a
/// U+00A0 at either edge stays in it for the scalar rule to refuse, and
/// a value whose colon is followed by anything but a space or the end of
/// the line is not a separated mapping at all.
///
/// `str::trim` here erased Unicode whitespace from both edges before the
/// scalar rule could reject it, and a missing separator was repaired
/// into a separated field: `lockfileVersion:9.0` and
/// `resolution:{integrity: sha512-X}` read as the well-formed control
/// (security hold 2026-09-20, findings 2 and 3).
enum Separated<'a> {
    /// `key:` with nothing after it: the key opens a block, or is null.
    Block,
    /// `key: value`, the value unchanged.
    Inline(&'a str),
}

fn pnpm_separated(rest: &str) -> Result<Separated<'_>, &'static str> {
    let rest = rest.trim_end_matches(' ');
    if rest.is_empty() {
        return Ok(Separated::Block);
    }
    match rest.strip_prefix(' ') {
        Some(value) => Ok(Separated::Inline(value.trim_start_matches(' '))),
        None => Err("lacks ': ' separation"),
    }
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
    let file = std::fs::File::open(path)
        .map_err(|error| CompositeError::PnpmLock(format!("{}: {error}", path.display())))?;
    read_pnpm_from(path, file)
}

/// `read_pnpm` over an already-opened source, so "at most 8,388,609 bytes
/// are consumed" is a number a test's own counting reader reports rather
/// than a sentence in a comment. Only the source is injectable; the bound,
/// the refusal and the UTF-8 conversion are production's. `path` names
/// the source in a refusal.
///
/// The control for the bound must be FINITE: an endless source under a
/// reader that has lost its `take` does not fail an assertion, it consumes
/// the host, and a control whose failure mode is exhaustion is not a
/// control (council return 2026-09-19, finding 4).
fn read_pnpm_from(path: &Path, source: impl Read) -> Result<String, CompositeError> {
    let unreadable =
        |reason: String| CompositeError::PnpmLock(format!("{}: {reason}", path.display()));
    let mut bytes = Vec::new();
    source
        .take(PNPM_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| unreadable(error.to_string()))?;
    if bytes.len() > PNPM_LIMIT {
        return Err(CompositeError::PnpmLock(format!(
            "pnpm lock exceeds {PNPM_LIMIT}-byte limit"
        )));
    }
    String::from_utf8(bytes).map_err(|_| unreadable("not UTF-8".to_string()))
}

/// The lockfile-9 top-level keys this reader recognizes.
///
/// The grammar is CLOSED. A document carrying a key this list does not
/// name is unreadable rather than partially read: a reader that silently
/// skips what it does not understand cannot promise it saw every package,
/// and identity is exactly that promise (council return 2026-09-19).
/// The recognized top-level keys that carry an inline SCALAR instead of
/// opening a block. Every other recognized key opens a block, and each
/// form is required of its own key.
///
/// The inherited reader decided the form from the LINE — a key with an
/// inline value was a value, a key without one opened a section — so
/// `packages: null` and `packages: {}` were read as inline values and
/// skipped, leaving the triples of an earlier `packages:` block standing
/// as the document's answer. A lock whose package set this reader cannot
/// read is unreadable, never a lock it reports an older section's
/// identity for (council return 2026-09-19).
const PNPM_SCALAR_SECTIONS: [&str; 3] = [
    "lockfileVersion",
    "packageExtensionsChecksum",
    "pnpmfileChecksum",
];

const PNPM_SECTIONS: [&str; 13] = [
    "catalogs",
    "ignoredOptionalDependencies",
    "importers",
    "lockfileVersion",
    "neverBuiltDependencies",
    "onlyBuiltDependencies",
    "overrides",
    "packageExtensionsChecksum",
    "packages",
    "patchedDependencies",
    "pnpmfileChecksum",
    "settings",
    "snapshots",
];

/// The child keys a `packages:` entry may carry. Only `resolution` is
/// read; the rest are named so that an unrecognized child is a refusal
/// rather than a silent skip.
const PNPM_PACKAGE_CHILDREN: [&str; 12] = [
    "bundledDependencies",
    "cpu",
    "deprecated",
    "engines",
    "hasBin",
    "libc",
    "name",
    "os",
    "peerDependencies",
    "peerDependenciesMeta",
    "resolution",
    "version",
];

/// The keys a `resolution:` flow map may carry.
///
/// D6 admits an integrity with an optional tarball and NOTHING else. The
/// inherited list also named `commit`, `directory`, `path`, `registry`,
/// `repo` and `type`, which it then ignored: a record resolving from a
/// git commit or a local directory was read as though it had come from
/// the registry, and its non-registry origin left identity unmoved
/// (council return 2026-09-19).
const PNPM_RESOLUTION_KEYS: [&str; 2] = ["integrity", "tarball"];

/// The YAML indicator characters that may not OPEN a plain scalar.
///
/// A plain scalar is the only unquoted form this grammar admits, and a
/// plain scalar cannot begin with an indicator. Admitting one read YAML
/// SYNTAX as though it were the value it denotes: `*undefined` is an
/// alias whose target this reader never follows, `[sha512-X]` is a
/// one-element sequence, `&anchor` names a node for a later alias and
/// `!!str` is a tag. Each of them hashed its own spelling, so the alias
/// target or sequence member could change underneath an identity that
/// never moved (council return 2026-09-19).
const YAML_INDICATORS: [char; 19] = [
    '-', '?', ':', ',', '[', ']', '{', '}', '#', '&', '*', '!', '|', '>', '\'', '"', '%', '@', '`',
];

/// A YAML scalar with its PROVENANCE kept: whether the document quoted
/// it. The bytes alone cannot say what a value is — a plain `null` is
/// the YAML null and a quoted `'null'` is a four-letter string — and a
/// reader that dropped the quote before the field's rule ran hashed the
/// two as one identity (security hold 2026-09-20, finding 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scalar<'a> {
    Plain(&'a str),
    Quoted(&'a str),
}

impl<'a> Scalar<'a> {
    /// The decoded text, with the quote's meaning already applied.
    fn text(self) -> &'a str {
        match self {
            Scalar::Plain(text) | Scalar::Quoted(text) => text,
        }
    }
}

/// Unwrap a YAML scalar EXACTLY, over the three forms this grammar
/// admits and no others.
///
/// - A single-quoted scalar is literal: its partner must close it and an
///   inner `'` — which YAML would read as the `''` escape — refuses.
/// - A double-quoted scalar admits no backslash at all. An escape is a
///   spelling whose VALUE differs from its bytes, so `"sha512-A"`
///   and `"a\tb"` both entered identity as their own syntax, and an
///   escaped space defeated the whitespace rule downstream.
/// - A plain scalar carries no quote, no `#` comment introducer and no
///   opening indicator.
///
/// The inherited `trim_matches` repaired `'sha512-X` into `sha512-X`, so
/// an unterminated scalar entered identity bytes as though it had been
/// written correctly. Nothing is repaired here; `None` is the refusal.
fn pnpm_scalar(text: &str) -> Option<Scalar<'_>> {
    if let Some(rest) = text.strip_prefix('\'') {
        // A lone quote leaves an empty remainder, which strips to
        // `None`: an opening quote with no partner is never closed.
        let inner = rest.strip_suffix('\'')?;
        return (!inner.contains('\'')).then_some(Scalar::Quoted(inner));
    }
    if let Some(rest) = text.strip_prefix('"') {
        let inner = rest.strip_suffix('"')?;
        return (!inner.contains('"') && !inner.contains('\\')).then_some(Scalar::Quoted(inner));
    }
    if text.contains('\'') || text.contains('"') || text.contains('#') {
        return None;
    }
    match text.chars().next() {
        Some(first) if YAML_INDICATORS.contains(&first) => None,
        // An empty plain scalar is the YAML null, which is not a value
        // any caller here may hash; each of them refuses it by name.
        _ => Some(Scalar::Plain(text)),
    }
}

/// The plain spellings the YAML core schema reads as something other
/// than a string: the null, the two booleans and every number, in the
/// case variants and numeric forms the schema admits. A plain scalar
/// that spells one of these is NOT a string, and an identity-bearing
/// string field refuses it by name rather than hashing the spelling
/// (design D10, 2026-09-20). This is a closed lexical rule over plain
/// scalars only; a quoted `'null'` never reaches it.
fn typed_plain_scalar(text: &str) -> Option<&'static str> {
    match text {
        "null" | "Null" | "NULL" | "~" => return Some("a null"),
        "true" | "True" | "TRUE" | "false" | "False" | "FALSE" => return Some("a boolean"),
        _ => {}
    }
    looks_numeric(text).then_some("a number")
}

/// Whether a plain scalar spells a YAML core-schema number: an optional
/// sign, then a base-prefixed integer, a decimal with optional fraction
/// and exponent, or the special `.inf`/`.nan` forms.
fn looks_numeric(text: &str) -> bool {
    let unsigned = text
        .strip_prefix('-')
        .or_else(|| text.strip_prefix('+'))
        .unwrap_or(text);
    if matches!(
        unsigned,
        ".inf" | ".Inf" | ".INF" | ".nan" | ".NaN" | ".NAN"
    ) {
        return true;
    }
    for (prefix, radix) in [("0x", 16), ("0o", 8), ("0b", 2)] {
        if let Some(digits) = unsigned.strip_prefix(prefix) {
            return !digits.is_empty() && digits.chars().all(|c| c.is_digit(radix));
        }
    }
    let (mantissa, exponent) = match unsigned.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, Some(exponent)),
        None => (unsigned, None),
    };
    let (whole, fraction) = match mantissa.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (mantissa, None),
    };
    let digits = |part: &str| part.chars().all(|c| c.is_ascii_digit());
    let whole_ok = digits(whole);
    let fraction_ok = fraction.is_none_or(digits);
    let some_digit = !whole.is_empty() || fraction.is_some_and(|f| !f.is_empty());
    let exponent_ok = exponent.is_none_or(|exponent| {
        let unsigned = exponent
            .strip_prefix('-')
            .or_else(|| exponent.strip_prefix('+'))
            .unwrap_or(exponent);
        !unsigned.is_empty() && digits(unsigned)
    });
    whole_ok && fraction_ok && some_digit && exponent_ok
}

/// Split a flow map's body at the commas that SEPARATE fields, leaving
/// a comma inside a quoted scalar to the scalar. The inherited split
/// broke `'a,b'` in two and refused a legal string for the wrong reason.
fn split_flow_fields(inner: &str) -> Vec<&str> {
    let mut fields = Vec::new();
    let mut quote: Option<char> = None;
    let mut start = 0;
    for (index, c) in inner.char_indices() {
        match quote {
            Some(open) if c == open => quote = None,
            Some(_) => {}
            None if c == '\'' || c == '"' => quote = Some(c),
            None if c == ',' => {
                fields.push(&inner[start..index]);
                start = index + 1;
            }
            None => {}
        }
    }
    fields.push(&inner[start..]);
    fields
}

/// Why a `resolution:` flow map cannot be read, in the reader's own
/// words: the reason names the field and the cause, so a refusal can
/// tell missing separation from unsupported syntax from a scalar of the
/// wrong kind (security hold 2026-09-20, finding 3).
fn pnpm_flow_map(inner: &str) -> Result<Vec<(&str, &str)>, String> {
    if inner.contains('{') || inner.contains('}') {
        return Err("a malformed resolution flow map".to_string());
    }
    let mut fields = Vec::new();
    for field in split_flow_fields(inner) {
        // The key ends at the first `:` and the separator is the `: `
        // YAML requires of a flow mapping. `integrity:sha512-X` has no
        // separator: in YAML it is one plain scalar, and the inherited
        // `split_once(':')` repaired it into a separated field.
        let Some((key, rest)) = field.split_once(':') else {
            return Err("a malformed resolution flow map".to_string());
        };
        // The flow map's own padding — spaces after `{`, around commas
        // and before `}` — is ASCII, and that is all that is consumed.
        let key = key.trim_matches(' ');
        if !PNPM_RESOLUTION_KEYS.contains(&key) {
            return Err("a malformed resolution flow map".to_string());
        }
        let value = match pnpm_separated(rest) {
            Ok(Separated::Inline(value)) => value,
            // `integrity: ` with nothing after the space is an empty
            // scalar, refused below by the malformed-map reason.
            Ok(Separated::Block) => "",
            Err(_) => {
                return Err(format!(
                    "the resolution field '{key}' lacks ': ' separation: '{}'",
                    field.trim_matches(' ')
                ))
            }
        };
        let Some(scalar) = pnpm_scalar(value) else {
            return Err("a malformed resolution flow map".to_string());
        };
        if scalar.text().is_empty() {
            return Err("a malformed resolution flow map".to_string());
        }
        if let Scalar::Plain(text) = scalar {
            // A plain scalar inside a flow collection may not carry the
            // collection's own punctuation, nor the `: ` that would open
            // a mapping inside it: `sha512-X[one]` is not a string with
            // brackets in it and `x: y` is not a string with a colon in
            // it, they are syntax this grammar does not read. Every field
            // meets this rule, the ignored `tarball` included, because
            // an ignored field that is not read is still a field whose
            // syntax the document must have (security hold 2026-09-20,
            // finding 2). A colon without a following space — a URL —
            // remains a plain string.
            if text.contains(['[', ']', '{', '}', ',']) || text.contains(": ") {
                return Err(format!(
                    "the resolution field '{key}' carries unsupported flow syntax: '{text}'"
                ));
            }
            if key == "integrity" {
                if let Some(kind) = typed_plain_scalar(text) {
                    return Err(format!(
                        "the resolution field '{key}' is the plain scalar '{text}', \
                         which is {kind} and not a string"
                    ));
                }
            }
        }
        fields.push((key, scalar.text()));
    }
    Ok(fields)
}

/// A `packages:` key, split into the package name and the version it
/// carries, with BOTH halves crossing the scalar rule and the package-name
/// grammar before any exclusion or serialization.
///
/// `a b@1` and `a@b 1` both serialized to `a b 1 <integrity>`: two
/// different dependencies wearing one identity line, and a name or
/// version carrying NUL or a control byte entered the hashed stream
/// unchecked. The whole key crosses `scalar_reason` first, which is what
/// separates the two spellings above (council return 2026-09-19).
fn pnpm_package(key: &str) -> Result<(&str, &str), String> {
    if let Some(reason) = scalar_reason(key) {
        return Err(format!("'{key}': the key {reason}"));
    }
    let at = scoped_at(key).ok_or_else(|| format!("'{key}': no '@' after the first character"))?;
    let (name, version) = (&key[..at], &key[at + 1..]);
    if version.is_empty() {
        return Err(format!("'{key}': empty version"));
    }
    let named = match name.strip_prefix('@') {
        Some(scoped) => match scoped.split_once('/') {
            Some((scope, rest)) => valid_component(scope) && valid_component(rest),
            None => false,
        },
        None => valid_component(name),
    };
    if !named {
        return Err(format!("'{key}': '{name}' is not a package name"));
    }
    Ok((name, version))
}

/// One `packages:` record, held while its child lines are read.
struct PnpmEntry {
    /// The key exactly as the lock spells it, so a refusal names the
    /// record the operator can find.
    key: String,
    name: String,
    version: String,
    integrity: Option<String>,
}

/// Every dependency triple in a pnpm lockfile-9.0 `packages:` section, parsed
/// by the bounded, fail-closed line reader design D6 states (no YAML crate).
/// `local` names the packages whose installed bytes already supply a
/// component line; only their local `file:` records are excluded, so a
/// same-named registry record is retained.
///
/// The grammar is closed over the document's SHAPE — its top-level keys,
/// its indentation and the children of a `packages:` record — and over
/// every byte identity is taken from. The bodies of the recognized
/// sections that carry no identity (`importers`, `snapshots` and their
/// kin) are skipped by design: nothing under them can reach a triple,
/// because only an indent-0 key opens a section and only `packages`
/// admits a record.
fn pnpm_dependencies(lock: &str, local: &[&str]) -> Result<Vec<String>, CompositeError> {
    let bad = |why: &str| CompositeError::PnpmLock(why.to_string());
    if lock.contains('\t') {
        return Err(bad("a tab"));
    }
    // `str::lines` accepts a CRLF document by dropping the CR, which read
    // a Windows-written lock as though it were a Unix one and hid the
    // difference from identity.
    if lock.contains('\r') {
        return Err(bad("a carriage return"));
    }
    let mut lines = lock.lines();
    // The first non-blank line is the lockfile version.
    let mut version_seen = false;
    for line in lines.by_ref() {
        if pnpm_blank(line) {
            continue;
        }
        let rest = line
            .strip_prefix("lockfileVersion:")
            .ok_or_else(|| bad("no lockfileVersion header"))?;
        // The header is an inline value and needs the admitted space
        // after its colon: `lockfileVersion:9.0` is one plain scalar in
        // YAML, not a header, and the inherited `trim` repaired it into
        // one (security hold 2026-09-20, finding 2).
        let version = match pnpm_separated(rest) {
            Ok(Separated::Inline(version)) => version,
            Ok(Separated::Block) => return Err(bad("a malformed lockfileVersion")),
            Err(why) => return Err(bad(&format!("a lockfileVersion header that {why}"))),
        };
        // The header keeps its own rule: the plain `9.0` and the quoted
        // `'9.0'` are both the admitted version, so the typed-scalar
        // refusal an identity string makes does not apply here.
        let version = pnpm_scalar(version).ok_or_else(|| bad("a malformed lockfileVersion"))?;
        if version.text() != "9.0" {
            return Err(bad("lockfileVersion is not 9.0"));
        }
        version_seen = true;
        break;
    }
    if !version_seen {
        return Err(bad("empty document"));
    }

    let mut triples = BTreeSet::new();
    let mut section: Option<&str> = None;
    // Every top-level key this document has spelled. A YAML mapping key
    // is a SINGLETON, and identity is the promise that one document has
    // one package set; `lockfileVersion` is seeded because the header
    // loop above already consumed the document's one spelling of it.
    let mut seen: BTreeSet<&str> = BTreeSet::from(["lockfileVersion"]);
    // Every decoded `packages:` heading this document has spelled, kept
    // apart from the triple set: one answers "was this key repeated",
    // the other "are these complete values equal", and the two questions
    // have different answers on the same document.
    let mut seen_packages: BTreeSet<String> = BTreeSet::new();
    let mut entry: Option<PnpmEntry> = None;
    // Whether a block-form child is open, whose own lines this reader
    // skips: they belong to `peerDependencies` and its kin, and none of
    // them enters identity. A flag rather than the open child's
    // indentation, because a child opens only at four spaces and only a
    // line at six or more consults this — so a depth comparison would be
    // a guard no input could falsify, which is a claim the code cannot
    // keep.
    let mut block = false;

    let flush =
        |entry: Option<PnpmEntry>, triples: &mut BTreeSet<String>| -> Result<(), CompositeError> {
            let Some(entry) = entry else {
                return Ok(());
            };
            let integrity = entry.integrity.ok_or_else(|| {
                CompositeError::PnpmLock(format!("'{}': no resolution integrity", entry.key))
            })?;
            // The local tarball record, excluded by its own `file:` version
            // rather than by its name: a registry record spelled with the
            // same name is a real dependency and stays. The exclusion is
            // decided AFTER the key's grammar, so an unreadable record is
            // never waved through by looking local.
            if local.contains(&entry.name.as_str()) && entry.version.starts_with("file:") {
                return Ok(());
            }
            if let Some(reason) = scalar_reason(&integrity) {
                return Err(CompositeError::PnpmLock(format!(
                    "'{}': integrity {reason}",
                    entry.key
                )));
            }
            triples.insert(format!("{} {} {}", entry.name, entry.version, integrity));
            Ok(())
        };

    for line in lines {
        if pnpm_blank(line) {
            continue;
        }
        let trimmed = line.trim_start_matches(' ');
        if trimmed.starts_with('#') || trimmed.starts_with("---") || trimmed.starts_with("...") {
            return Err(bad("a comment or document marker"));
        }
        let indent = pnpm_indent(line);
        if !indent.is_multiple_of(2) {
            return Err(bad("an odd indentation"));
        }
        if indent == 0 {
            flush(entry.take(), &mut triples)?;
            block = false;
            let (name, rest) = line
                .split_once(':')
                .ok_or_else(|| bad("a top-level line that is not a mapping key"))?;
            if !PNPM_SECTIONS.contains(&name) {
                return Err(bad(&format!("an unrecognized top-level key '{name}'")));
            }
            if !seen.insert(name) {
                return Err(bad(&format!("a repeated top-level key '{name}'")));
            }
            let value = match pnpm_separated(rest) {
                Ok(value) => value,
                Err(why) => {
                    return Err(bad(&format!("a top-level key '{name}' that {why}")));
                }
            };
            // The form is required of the KEY, not read off the line.
            if PNPM_SCALAR_SECTIONS.contains(&name) {
                // Recognized and not read, but still parsed: a checksum
                // that opens a block, or one whose quote never closes,
                // is a document shape this grammar has not measured.
                let Separated::Inline(value) = value else {
                    return Err(bad(&format!(
                        "a malformed scalar for top-level key '{name}'"
                    )));
                };
                if pnpm_scalar(value).is_none() {
                    return Err(bad(&format!(
                        "a malformed scalar for top-level key '{name}'"
                    )));
                }
                section = None;
                continue;
            }
            if let Separated::Inline(_) = value {
                return Err(bad(&format!(
                    "an inline value on top-level section '{name}'"
                )));
            }
            section = Some(name);
            continue;
        }
        let Some(open_section) = section else {
            return Err(bad("a child line outside every section"));
        };
        if open_section != "packages" {
            continue;
        }
        if indent == 2 {
            flush(entry.take(), &mut triples)?;
            block = false;
            let key = line
                .trim_end_matches(' ')
                .strip_suffix(':')
                .ok_or_else(|| bad("a package key is not colon-terminated"))?;
            let key = pnpm_scalar(key.trim_matches(' '))
                .ok_or_else(|| bad("a malformed package key"))?
                .text();
            // A key that is itself a mapping — `a@1: {}` reaches this
            // reader as `a@1: {`, and `a@1: b:` as a key ending in `b` —
            // is a flow-form or nested record, not a package heading.
            if key.contains(": ") {
                return Err(bad("a package key that is itself a mapping"));
            }
            let (name, version) = pnpm_package(key).map_err(CompositeError::PnpmLock)?;
            // A mapping key is a SINGLETON, checked on the DECODED key
            // before this record is excluded or its triple normalized.
            // The triple set below deduplicates equal complete values
            // from distinct records, which is legitimate; it cannot
            // tell a repeated heading from that, so an identical
            // repeat vanished into it and a conflicting repeat merged
            // into an order-independent digest for a document with no
            // single meaning (security hold 2026-09-20, finding 4).
            if !seen_packages.insert(key.to_string()) {
                return Err(bad(&format!("a repeated package key '{key}'")));
            }
            entry = Some(PnpmEntry {
                key: key.to_string(),
                name: name.to_string(),
                version: version.to_string(),
                integrity: None,
            });
            continue;
        }
        if indent == 4 {
            block = false;
            let Some(open) = entry.as_mut() else {
                return Err(bad("a package child outside any record"));
            };
            let (name, rest) = trimmed
                .split_once(':')
                .ok_or_else(|| bad("a package child that is not a mapping key"))?;
            if !PNPM_PACKAGE_CHILDREN.contains(&name) {
                return Err(bad(&format!("an unrecognized package child '{name}'")));
            }
            // Every child, read or ignored, is a separated mapping:
            // `resolution:{integrity: sha512-X}` is one plain scalar in
            // YAML, and the inherited `trim` repaired it into the outer
            // mapping this reader admits (security hold 2026-09-20,
            // finding 2).
            let value = match pnpm_separated(rest) {
                Ok(value) => value,
                Err(why) => {
                    return Err(bad(&format!("a package child '{name}' that {why}")));
                }
            };
            if name != "resolution" {
                // Recognized and not read. A child with no inline scalar
                // opens a block whose own lines are skipped below.
                if let Separated::Block = value {
                    block = true;
                }
                continue;
            }
            let Separated::Inline(value) = value else {
                return Err(bad("a block-form or malformed resolution"));
            };
            let inner = value
                .strip_prefix('{')
                .and_then(|text| text.strip_suffix('}'))
                .ok_or_else(|| bad("a block-form or malformed resolution"))?;
            let fields = pnpm_flow_map(inner).map_err(|why| bad(&why))?;
            let mut integrity = None;
            let mut seen_fields = BTreeSet::new();
            for (key, value) in fields {
                // EVERY field is a singleton, not only the one read. A
                // repeated `tarball` is two origins for one record, and a
                // reader that keeps the last of them answers for a
                // document with no single meaning.
                if !seen_fields.insert(key) {
                    return Err(bad(&format!("a repeated resolution field '{key}'")));
                }
                if key == "integrity" {
                    integrity = Some(value);
                }
            }
            let integrity = integrity.ok_or_else(|| bad("a resolution without integrity"))?;
            if open.integrity.is_some() {
                return Err(bad(&format!("'{}': repeated resolution", open.key)));
            }
            open.integrity = Some(integrity.to_string());
            continue;
        }
        // Six spaces or deeper: the children of an open block-form child,
        // and nothing else.
        match block {
            true => continue,
            false => return Err(bad("a package line at an unrecognized indentation")),
        }
    }
    flush(entry.take(), &mut triples)?;
    if !seen.contains("packages") {
        return Err(bad("no packages section"));
    }
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

/// A DSH executable the adapter's seam SELECTED: the canonical file the
/// declared name resolved to, and the home result beside it. The home
/// can fail on its own — a caller that must keep observing the chosen
/// installation after a home failure (doctor, whose version probe is
/// independent of the composite) reads the executable here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DshSelection {
    pub executable: String,
    pub seams: Result<DshSeams, CompositeError>,
}

/// A selection that did not happen: the spelling that was looked for and
/// the cause it was not selected by. Both are for diagnosis only. There
/// is no executable here to probe, and the inherited arrangement that
/// handed a caller the declared spelling in its place is the one that
/// executed a cwd `dsh` under an absent `PATH` (security hold 2026-09-20,
/// S1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DshUnselected {
    pub declared: String,
    pub cause: CompositeError,
}

impl DshSeams {
    pub fn resolve() -> Result<DshSeams, CompositeError> {
        DshSeams::resolved(DshSeams::selected())
    }

    /// `resolve` over an injected selection: the selection's seams, or
    /// the cause it did not happen by. The planner has no use for a
    /// declared spelling without a file behind it, so a failed selection
    /// is the layout's refusal and nothing is looked up again.
    fn resolved(
        selection: Result<DshSelection, DshUnselected>,
    ) -> Result<DshSeams, CompositeError> {
        match selection {
            Ok(selection) => selection.seams,
            Err(unselected) => Err(unselected.cause),
        }
    }

    /// Both seams from ONE environment resolution: the executable the
    /// adapter selected, resolved ONCE to the file it names, and the home
    /// beside it.
    ///
    /// The returned executable is the lookup's answer, not the bare name
    /// the seam spells, and it is the same string the seams carry. A
    /// caller that probes it and then asks the producer for a composite
    /// is therefore asking about one file: two lookups of `dsh` can
    /// disagree — a non-executable `A/dsh` ahead of an executable
    /// `B/dsh` is skipped by a spawning child and was taken by this
    /// crate's own resolver — and a version from one install beside a
    /// digest from another is the defect 8.8(c) exists to close (council
    /// return 2026-09-19).
    ///
    /// Selection is FALLIBLE, and a failed selection carries no
    /// executable at all. The inherited tuple kept the declared spelling
    /// beside a discarded lookup error, so a caller probed that spelling
    /// as though it had been selected (design D10, 2026-09-20).
    pub fn selected() -> Result<DshSelection, DshUnselected> {
        DshSeams::selected_from(
            super::adapter_binary("BROKKR_DSH_BIN", Some("FORGE_DSH_BIN"), "dsh"),
            resolve_executable,
            crate::transcript::dsh_home(),
        )
    }

    /// `selected` over an injected resolver and home, so the resolved,
    /// unresolvable and unspellable selections are all plain tests
    /// rather than facts about whichever `dsh` the host happens to have
    /// installed while the suite runs.
    fn selected_from(
        declared: String,
        resolve: impl FnOnce(&str) -> Result<PathBuf, CompositeError>,
        home: Option<PathBuf>,
    ) -> Result<DshSelection, DshUnselected> {
        let path = match resolve(&declared) {
            Ok(path) => path,
            Err(cause) => return Err(DshUnselected { declared, cause }),
        };
        // A path this platform cannot spell as UTF-8 is refused by
        // cause: neither a lossy rendering of it nor a retry of the
        // declaration names the file that was selected.
        let Some(executable) = path.to_str().map(str::to_string) else {
            return Err(DshUnselected {
                declared,
                cause: CompositeError::Config(format!(
                    "{}: the selected path is not UTF-8",
                    path.display()
                )),
            });
        };
        let seams = DshSeams::resolve_with(executable.clone(), home);
        Ok(DshSelection { executable, seams })
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
/// Every member is PRIVATE to this module. A caller outside it consumes
/// an observation and reads what it needs; it cannot assemble one from
/// digests it computed itself, nor edit one this producer returned. That
/// is what "the only producer of either value" means in the type system
/// rather than only in a comment (design D6 (b); council return
/// 2026-09-19). Synthetic construction exists only under `cfg(test)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DshComposite {
    canonical: String,
    core: String,
    node: String,
    plugin: String,
    dependencies: Vec<String>,
    plugin_patch: String,
    profile_patch: String,
    profile_bundles: Vec<String>,
    profile_patch_reload: String,
    home_patch: String,
    extension: Option<String>,
    core_root: PathBuf,
    profile: PathBuf,
}

impl DshComposite {
    /// The canonical composite digest: 64 lowercase hexadecimal
    /// characters over D6's fixed component lines.
    pub fn canonical(&self) -> &str {
        &self.canonical
    }

    /// The plugin component digest, which doctor reports beside the
    /// canonical one so a drifted plugin names itself.
    pub fn plugin(&self) -> &str {
        &self.plugin
    }

    /// A synthetic observation with a chosen canonical digest, for the
    /// planner and doctor suites that must drive a qualifying or
    /// drifting case without an installed pair. Test-only by
    /// construction: production has no way to reach it.
    #[cfg(test)]
    pub(crate) fn synthetic(canonical: &str) -> DshComposite {
        DshComposite {
            canonical: canonical.to_string(),
            core: "core".to_string(),
            node: "v22.23.2".to_string(),
            plugin: "plugin".to_string(),
            dependencies: Vec::new(),
            plugin_patch: "patch".to_string(),
            profile_patch: "profile".to_string(),
            profile_bundles: Vec::new(),
            profile_patch_reload: "startup".to_string(),
            home_patch: "absent".to_string(),
            extension: None,
            core_root: PathBuf::new(),
            profile: PathBuf::new(),
        }
    }
}

/// One JSON document, reported by REASON rather than by a fixed
/// component: the same reader serves the core manifest, the profile
/// manifest and the hidden npm lock, and each caller names the component
/// whose failure it is. Mapping every one of them to "the DSH layout"
/// sent an operator holding a corrupt npm lock to look at a package.json
/// (council return 2026-09-19).
fn read_json(path: &Path) -> Result<Value, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("{}: not JSON: {error}", path.display()))
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

/// The executable's first line, EXACTLY as the file spells it.
///
/// No carriage return is removed. A CRLF-written `#!/usr/bin/env node\r`
/// is a different first line from the one D6 requires — the kernel reads
/// the CR as part of the interpreter name — and repairing it here let a
/// file the loader could not execute pass as the qualified one (council
/// return 2026-09-19).
fn first_line(path: &Path) -> Result<String, CompositeError> {
    let bytes = std::fs::read(path)
        .map_err(|error| CompositeError::Config(format!("{}: {error}", path.display())))?;
    let end = bytes
        .iter()
        .position(|byte| *byte == b'\n')
        .unwrap_or(bytes.len());
    String::from_utf8(bytes[..end].to_vec())
        .map_err(|_| CompositeError::Config(format!("{}: first line is not UTF-8", path.display())))
}

/// Whether THIS process could execute `path`, asked of the kernel with
/// the process's effective identity — `faccessat(AT_EACCESS)`, which is
/// the check `execve` itself makes.
///
/// `mode & 0o111 != 0` asked a different question: whether ANYONE may
/// execute the file. A candidate at mode 0641 carries an execute bit for
/// others and none for its owner, so the inherited test said yes and the
/// child that followed got `EACCES` and walked on to a later entry —
/// which is exactly the two-installs disagreement 8.8(c) exists to close
/// (council return 2026-09-19).
#[cfg(unix)]
fn effective_exec_access(path: &Path) -> bool {
    rustix::fs::accessat(
        rustix::fs::CWD,
        path,
        rustix::fs::Access::EXEC_OK,
        rustix::fs::AtFlags::EACCESS,
    )
    .is_ok()
}

/// What one search entry taught the resolver about its candidate.
///
/// The inherited `is_executable_file` answered a BOOLEAN, and a boolean
/// cannot say why: a symlink loop, a metadata failure of unknown cause
/// and a plain absence all read `false`, and `false` authorized the
/// next entry. The child's own search stops at the loop with `ELOOP`,
/// so a resolver that walked on to `B/dsh` paired B's version with a
/// selection the child never made (security hold 2026-09-20, finding 2).
enum Candidate {
    /// The entry a child would execute, canonical.
    Admitted(PathBuf),
    /// Not this entry, for a reason the child's search also walks past:
    /// absence, an untraversable component, a non-file or a file this
    /// process may not execute. The reason is kept for an explicit
    /// override, which has no next entry to walk to.
    Passed(String),
    /// The search stops here, by cause: a loop, a failure the resolver
    /// cannot prove the child would walk past, an obstruction the child
    /// would only meet at `exec`, or a path it cannot canonicalize.
    Refused(CompositeError),
}

/// Whether a lookup failed with the kernel's `ELOOP`: the error the
/// child's `execve` stops on at a self-referential symlink. Asked by the
/// platform's own number, because `ErrorKind::FilesystemLoop` is not
/// stable on the pinned compiler.
fn is_symlink_loop(error: &std::io::Error) -> bool {
    #[cfg(unix)]
    {
        error.raw_os_error() == Some(rustix::io::Errno::LOOP.raw_os_error())
    }
    #[cfg(not(unix))]
    {
        let _ = error;
        false
    }
}

/// Whether the platform reads `command` as a PATH to a file rather than
/// a NAME to search for. This is the platform's rule and nothing else:
/// `std::process::Command` on Unix treats a program as a path if and
/// only if it contains `/`, and a backslash is an ordinary filename
/// byte; on Windows either separator makes a path. The inherited
/// classifier read a backslash as a separator on Unix, so an override
/// spelled `C:\Tools\dsh.exe` was taken as a path, and a cwd file
/// literally named that executed where `Command::new` of the same name
/// finds nothing (security hold 2026-09-20, S1b; controller
/// reproduction). Resolution is not a property of a string's characters
/// beyond this one predicate.
#[cfg(unix)]
fn is_explicit_path(command: &str) -> bool {
    command.contains('/')
}

#[cfg(windows)]
fn is_explicit_path(command: &str) -> bool {
    command.contains(['/', '\\'])
}

/// The two spellings no lookup may begin on, refused BEFORE any
/// filesystem, home or search work: a name carrying a NUL, which
/// `Command::new` refuses as invalid input without executing anything,
/// and an empty name, which `execvp` refuses with ENOENT.
fn refuse_unspellable(command: &str) -> Result<(), CompositeError> {
    if command.contains('\0') {
        return Err(CompositeError::Config(format!(
            "'{}' carries a NUL",
            command.replace('\0', "\\0")
        )));
    }
    if command.is_empty() {
        return Err(CompositeError::Config("'' names no program".to_string()));
    }
    Ok(())
}

/// The search a child's `execvp` runs, captured ONCE from the environment
/// the child would inherit (design D10).
///
/// A present `PATH` is searched as spelled, ordered empty entries
/// included, because an empty entry names the working directory to the
/// child too. An ABSENT `PATH` is not an empty search and not a refusal:
/// the C library searches its own default path in that case — glibc's
/// `execvp` reads `CS_PATH` when `getenv("PATH")` is null — so a bare name
/// that sits there runs, and one that does not is a native no-match. The
/// inherited reader first turned absence into one empty entry, which
/// selected a cwd `dsh` (S1), and then into an unconditional refusal,
/// which the commission withdrew: neither is what the platform does.
#[cfg(unix)]
struct Search {
    entries: std::ffi::OsString,
    /// Whether `entries` is the C library's default rather than the
    /// environment's own `PATH`, so a no-match names which was searched.
    default: bool,
}

#[cfg(unix)]
impl Search {
    fn capture(path: Option<std::ffi::OsString>) -> Result<Search, CompositeError> {
        match path {
            Some(entries) => Ok(Search {
                entries,
                default: false,
            }),
            None => Ok(Search {
                entries: default_search_path()?,
                default: true,
            }),
        }
    }

    /// Search the entries for `command` as the child would: the first
    /// entry whose candidate is admitted wins, an entry the child walks
    /// past is walked past, and a candidate the resolver cannot prove
    /// stops the search by cause.
    fn find(&self, command: &str, chain: &mut Vec<(u64, u64)>) -> Result<PathBuf, CompositeError> {
        for dir in std::env::split_paths(&self.entries) {
            let candidate = match dir.as_os_str().is_empty() {
                true => PathBuf::from(command),
                false => dir.join(command),
            };
            match classify_in(&candidate, self, chain) {
                Candidate::Admitted(path) => return Ok(path),
                Candidate::Passed(_) => continue,
                Candidate::Refused(error) => return Err(error),
            }
        }
        Err(CompositeError::Config(match self.default {
            true => format!(
                "'{command}' is not on the default search path {} (PATH is absent)",
                self.entries.to_string_lossy()
            ),
            false => format!("'{command}' is not on PATH"),
        }))
    }
}

/// The C library's default search path: what `execvp` consults when the
/// environment has no `PATH`. Read from the library through
/// `confstr(_CS_PATH)` — glibc and the Apple libc both report there the
/// value their `execvp` uses — never hardcoded from one host's answer,
/// and never obtained by launching `getconf`, a shell or `which`.
#[cfg(all(unix, not(target_env = "musl")))]
fn default_search_path() -> Result<std::ffi::OsString, CompositeError> {
    extern "C" {
        fn confstr(name: std::os::raw::c_int, buf: *mut std::os::raw::c_char, len: usize) -> usize;
    }
    // `_CS_PATH` is 0 in glibc's and Android's <unistd.h> and 1 in the
    // BSD-derived ones, Apple's included.
    const CS_PATH: std::os::raw::c_int = if cfg!(any(target_os = "linux", target_os = "android")) {
        0
    } else {
        1
    };
    default_search_path_from(|buf| {
        // SAFETY: `buf` is a live, writable slice of exactly the length
        // passed; `confstr` writes at most that many bytes into it and
        // reads nothing else.
        unsafe { confstr(CS_PATH, buf.as_mut_ptr().cast(), buf.len()) }
    })
}

/// musl's `execvp` does not consult `confstr`: it searches this literal
/// when `PATH` is absent (src/process/execvp.c), and its `confstr`
/// reports a different string, so the library's own rule is the literal.
#[cfg(target_env = "musl")]
fn default_search_path() -> Result<std::ffi::OsString, CompositeError> {
    Ok(std::ffi::OsString::from("/usr/local/bin:/bin:/usr/bin"))
}

/// `default_search_path` over an injected `confstr`, so the two answers
/// the library can give besides a path — no such variable, and a value
/// longer than the buffer — are plain tests. `query` writes into the
/// buffer and answers the length the value needs, NUL included, or zero.
#[cfg(unix)]
fn default_search_path_from(
    query: impl Fn(&mut [u8]) -> usize,
) -> Result<std::ffi::OsString, CompositeError> {
    use std::os::unix::ffi::OsStringExt;

    let mut buf = vec![0u8; 4096];
    let mut needed = query(&mut buf);
    if needed > buf.len() {
        buf = vec![0u8; needed];
        needed = query(&mut buf);
    }
    if needed == 0 || needed > buf.len() {
        return Err(CompositeError::Config(
            "the C library reports no default search path for an absent PATH".to_string(),
        ));
    }
    let end = buf.iter().position(|byte| *byte == 0).unwrap_or(buf.len());
    buf.truncate(end);
    Ok(std::ffi::OsString::from_vec(buf))
}

/// Classify one candidate as the child's search would, stopping where
/// the child stops and refusing where this resolver cannot prove what
/// the child would do. `chain` is the scripts whose interpreters are
/// being followed to reach this candidate, so a loop is a named refusal.
#[cfg(unix)]
fn classify_in(candidate: &Path, search: &Search, chain: &mut Vec<(u64, u64)>) -> Candidate {
    let refuse = |why: String| {
        Candidate::Refused(CompositeError::Config(format!(
            "{}: {why}",
            candidate.display()
        )))
    };
    let metadata = match std::fs::metadata(candidate) {
        Ok(metadata) => metadata,
        // `execvp` records ENOENT and EACCES and tries the next entry;
        // every other failure is one it stops on, and so does this.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Candidate::Passed(error.to_string());
        }
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            return Candidate::Passed(error.to_string());
        }
        Err(error) if is_symlink_loop(&error) => {
            return refuse(format!("a symlink loop stops the lookup: {error}"));
        }
        Err(error) => return refuse(format!("the lookup cannot be proved: {error}")),
    };
    if !metadata.is_file() {
        return Candidate::Passed("is not a regular file".to_string());
    }
    if !effective_exec_access(candidate) {
        return Candidate::Passed("is not executable by this process".to_string());
    }
    if let Err(why) = loading_obstruction(candidate, metadata.len(), search, chain) {
        return refuse(why);
    }
    // Admitted as the file it canonically is; a canonicalization the
    // metadata above did not already rule out is refused by the helper's
    // own reason.
    canonicalize(candidate).map_or_else(Candidate::Refused, Candidate::Admitted)
}

/// The kernel's own bound on a `#!` line (`BINPRM_BUF_SIZE`): a script
/// whose line runs past it without a terminator is one Linux refuses to
/// load, and one this resolver does not guess at.
#[cfg(unix)]
const SHEBANG_BOUND: usize = 256;

/// How many nested `#!` interpreters are followed. The kernel stops a
/// deeper chain with ELOOP; a chain this resolver will not follow is a
/// named refusal, never an admission.
#[cfg(unix)]
const INTERPRETER_DEPTH: usize = 4;

/// What would stop the child at `exec` for this candidate, established
/// from the file's LOADING DECLARATIONS and nothing more.
///
/// A candidate the metadata and access checks admit can still be one the
/// kernel refuses to load: `A/dsh` naming a nonexistent interpreter, or a
/// native image whose dynamic loader is missing, passes both checks, the
/// child's `execve` fails with ENOENT and its search walks on to `B/dsh`.
/// Metadata cannot see that (security hold 2026-09-20, finding 4). So
/// the head is read for a `#!` line and the interpreter it names is
/// inspected in turn; a native image is read for its loader, and the
/// loader is read as an image too. Where a prerequisite is missing,
/// malformed or beyond what the reader establishes, the candidate is
/// REFUSED by that cause, because neither a loader emulation nor a trial
/// execution is on the table (design D10). The refusal is the one
/// exception to native equality, and it is named as such.
#[cfg(unix)]
fn loading_obstruction(
    candidate: &Path,
    len: u64,
    search: &Search,
    chain: &mut Vec<(u64, u64)>,
) -> Result<(), String> {
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::MetadataExt;

    // At most the bound is read for the head, whatever the file's
    // length: a native image is then read at the offsets its header
    // names, and nowhere else.
    let (mut file, head) =
        open_head(candidate).map_err(|error| format!("cannot be read: {error}"))?;
    let read = head.len();
    let Some(line) = head.strip_prefix(b"#!") else {
        return native_obstruction(&mut file, len);
    };
    let line = match line.iter().position(|byte| *byte == b'\n') {
        Some(end) => &line[..end],
        // The whole file is shorter than the bound, so the line is the
        // rest of it; a full buffer with no terminator is the truncated
        // line the kernel refuses.
        None if read < SHEBANG_BOUND => line,
        None => {
            return Err(format!(
                "its #! line is not terminated within {SHEBANG_BOUND} bytes"
            ))
        }
    };
    if line.contains(&0) {
        return Err("its #! line carries a NUL".to_string());
    }
    let line = match line.iter().position(|byte| *byte != b' ' && *byte != b'\t') {
        Some(start) => &line[start..],
        None => &[],
    };
    let end = line
        .iter()
        .position(|byte| *byte == b' ' || *byte == b'\t')
        .unwrap_or(line.len());
    let (interpreter, arguments) = line.split_at(end);
    if interpreter.is_empty() {
        return Err("its #! line names no interpreter".to_string());
    }
    if interpreter[0] != b'/' {
        return Err(format!(
            "its #! interpreter '{}' is not an absolute path",
            String::from_utf8_lossy(interpreter)
        ));
    }
    let interpreter = Path::new(std::ffi::OsStr::from_bytes(interpreter));
    let named = interpreter.display();
    let metadata = match std::fs::metadata(interpreter) {
        Ok(metadata) if metadata.is_file() && effective_exec_access(interpreter) => metadata,
        Ok(_) => {
            return Err(format!(
                "its #! interpreter '{named}' is not an executable file"
            ))
        }
        Err(error) => return Err(format!("its #! interpreter '{named}' is missing: {error}")),
    };
    // The interpreter is a candidate in its own right: a script whose
    // interpreter is a script is followed, a loop is a refusal, and a
    // chain deeper than the kernel follows is not followed either. The
    // identity followed is the file's own — device and inode — so two
    // spellings of one file are one link of the chain.
    let identity = (metadata.dev(), metadata.ino());
    if chain.contains(&identity) {
        return Err(format!("its #! interpreter chain loops at '{named}'"));
    }
    if chain.len() >= INTERPRETER_DEPTH {
        return Err(format!(
            "its #! interpreter chain runs deeper than {INTERPRETER_DEPTH}"
        ));
    }
    chain.push(identity);
    let loaded = loading_obstruction(interpreter, metadata.len(), search, chain)
        .and_then(|()| env_program(interpreter, arguments, search, chain));
    chain.pop();
    loaded.map_err(|why| format!("its #! interpreter '{named}' {why}"))
}

/// Open a candidate and read its bounded head: one fallible operation,
/// so the caller names one cause for a file it may execute but not read.
#[cfg(unix)]
fn open_head(candidate: &Path) -> std::io::Result<(std::fs::File, Vec<u8>)> {
    let mut file = std::fs::File::open(candidate)?;
    let mut head = Vec::with_capacity(SHEBANG_BOUND);
    (&mut file)
        .take(SHEBANG_BOUND as u64)
        .read_to_end(&mut head)?;
    Ok((file, head))
}

/// The measured `#!/usr/bin/env node` form: `env` itself loads, and the
/// program it would search for is selected under the SAME native search,
/// so a missing `node` is a refusal before any probe rather than a
/// version probe that exits 127. One word after `env`, naming a program
/// and not an option, is the form this establishes; an option language
/// (`-S`, `-i`, `NAME=value`) is not added, and refuses by name.
#[cfg(unix)]
fn env_program(
    interpreter: &Path,
    arguments: &[u8],
    search: &Search,
    chain: &mut Vec<(u64, u64)>,
) -> Result<(), String> {
    if interpreter.file_name() != Some(std::ffi::OsStr::new("env")) {
        return Ok(());
    }
    let arguments = match arguments
        .iter()
        .position(|byte| *byte != b' ' && *byte != b'\t')
    {
        Some(start) => &arguments[start..],
        None => return Ok(()),
    };
    let end = arguments
        .iter()
        .position(|byte| *byte == b' ' || *byte == b'\t')
        .unwrap_or(arguments.len());
    let program = &arguments[..end];
    if program[0] == b'-' || program.contains(&b'=') {
        return Err(format!(
            "takes '{}', which is not the measured `env <program>` form",
            String::from_utf8_lossy(arguments)
        ));
    }
    let program = std::str::from_utf8(program)
        .map_err(|_| "names a program that is not UTF-8".to_string())?;
    match lookup_in(program, search, chain) {
        Ok(_) => Ok(()),
        Err(cause) => Err(format!("selects no '{program}': {cause}")),
    }
}

/// The loading prerequisite of a native image, read as the kernel reads
/// it: the image parsed by the bounded reader, its format this target's,
/// and its declared loader an existing executable that parses as a loader
/// of the same format. A static image has no prerequisite.
#[cfg(unix)]
fn native_obstruction(file: &mut std::fs::File, len: u64) -> Result<(), String> {
    use std::os::unix::ffi::OsStrExt;

    let image = image::inspect(file, len)
        .map_err(|why| format!("is not a loadable native image: {why}"))?;
    if image.kind != image::NATIVE {
        return Err(format!(
            "is a {} image, which this target does not load",
            image.kind
        ));
    }
    let Some(loader) = image.loader else {
        return Ok(());
    };
    let kind = image.kind;
    let loader = Path::new(std::ffi::OsStr::from_bytes(&loader));
    let named = loader.display();
    if !loader.is_absolute() {
        return Err(format!(
            "needs the {kind} loader '{named}', which is not an absolute path"
        ));
    }
    let loader_len = match std::fs::metadata(loader) {
        Ok(metadata) if metadata.is_file() && effective_exec_access(loader) => metadata.len(),
        Ok(_) => {
            return Err(format!(
                "needs the {kind} loader '{named}', which is not an executable file"
            ))
        }
        Err(error) => {
            return Err(format!(
                "needs the {kind} loader '{named}', which is missing: {error}"
            ))
        }
    };
    let mut loader_file = std::fs::File::open(loader).map_err(|error| {
        format!("needs the {kind} loader '{named}', which cannot be read: {error}")
    })?;
    let loaded = image::inspect(&mut loader_file, loader_len).map_err(|why| {
        format!("needs the {kind} loader '{named}', which is not a loadable image: {why}")
    })?;
    if !loaded.is_loader_for(kind) {
        return Err(format!(
            "needs the {kind} loader '{named}', which does not load as a {kind} loader"
        ));
    }
    Ok(())
}

/// Resolve `command` to the canonical file the platform's own lookup
/// selects for it, under the environment this process would hand a
/// child: a path is used directly, a name is searched.
#[cfg(unix)]
fn resolve_executable(command: &str) -> Result<PathBuf, CompositeError> {
    resolve_executable_in(command, std::env::var_os("PATH"))
}

/// `resolve_executable` on Windows: the child's environment is the
/// parent's, unchanged, so only the parent's `PATH` is searched after the
/// application, system and Windows directories.
#[cfg(windows)]
fn resolve_executable(command: &str) -> Result<PathBuf, CompositeError> {
    refuse_unspellable(command)?;
    windows_lookup(command, None, std::env::var_os("PATH"))
}

/// `resolve_executable` over an injected child `PATH`, so an absent
/// `PATH`, an empty entry, a non-executable candidate, an obstructed
/// candidate and a miss are plain tests, and so the differential matrix
/// can hand the resolver exactly the environment its native oracle ran.
///
/// The outcome is EQUAL to `std::process::Command::new(command)`'s under
/// the same cwd and environment: the same file, or a refusal exactly
/// where the child gets NotFound — with the one named exception of a
/// candidate whose loading prerequisite the resolver cannot establish
/// without executing it, which is refused by cause rather than guessed
/// (design D10). Nothing native lookup would not execute is selected,
/// and nothing it would execute is silently swapped for another.
#[cfg(unix)]
fn resolve_executable_in(
    command: &str,
    path: Option<std::ffi::OsString>,
) -> Result<PathBuf, CompositeError> {
    refuse_unspellable(command)?;
    let search = Search::capture(path)?;
    lookup_in(command, &search, &mut Vec::new())
}

/// One lookup under a captured search: a path is one candidate under the
/// same checks, with no next entry to walk to — what a search would pass
/// over is, for an explicit path, the refusal itself; a name is searched.
#[cfg(unix)]
fn lookup_in(
    command: &str,
    search: &Search,
    chain: &mut Vec<(u64, u64)>,
) -> Result<PathBuf, CompositeError> {
    if is_explicit_path(command) {
        return match classify_in(Path::new(command), search, chain) {
            Candidate::Admitted(path) => Ok(path),
            Candidate::Passed(why) => Err(CompositeError::Config(format!("{command}: {why}"))),
            Candidate::Refused(error) => Err(error),
        };
    }
    search.find(command, chain)
}

/// `resolve_executable_in` on Windows: `path` is the child's explicit
/// `PATH` — present, or removed from a changed environment — and the
/// parent's `PATH` is the process's own, exactly the two inputs
/// `std::process::Command` resolves a program against there.
#[cfg(windows)]
fn resolve_executable_in(
    command: &str,
    path: Option<std::ffi::OsString>,
) -> Result<PathBuf, CompositeError> {
    refuse_unspellable(command)?;
    windows_lookup(command, Some(path.as_deref()), std::env::var_os("PATH"))
}

/// Rust's Windows program resolution (library/std/src/sys/process/
/// windows.rs, `resolve_exe`), reimplemented rather than borrowed from
/// the Unix loop: a program with a separator is a path, tried with an
/// appended `.exe` before the literal spelling unless it already ends in
/// `.exe`; a file name is searched — child `PATH` if the child's
/// environment was changed, the application directory, the system
/// directory, the Windows directory, then the parent's `PATH`, skipping
/// empty entries — with `.exe` appended when the name has no extension.
/// The first entry that EXISTS is the selection; `CreateProcessW` then
/// either runs it or fails, and never tries a later entry, so an
/// existing candidate that cannot load is a refusal here too.
#[cfg(windows)]
fn windows_lookup(
    command: &str,
    child: Option<Option<&std::ffi::OsStr>>,
    parent: Option<std::ffi::OsString>,
) -> Result<PathBuf, CompositeError> {
    if command.ends_with(['/', '\\']) {
        return Err(CompositeError::Config(format!(
            "'{command}' has no file name"
        )));
    }
    let has_exe_suffix = command
        .as_bytes()
        .get(command.len().wrapping_sub(4)..)
        .is_some_and(|tail| tail.eq_ignore_ascii_case(b".exe"));
    if is_explicit_path(command) {
        let candidate = if has_exe_suffix {
            PathBuf::from(command)
        } else {
            let mut with_suffix = std::ffi::OsString::from(command);
            with_suffix.push(".exe");
            let with_suffix = PathBuf::from(with_suffix);
            match std::fs::symlink_metadata(&with_suffix).is_ok() {
                true => with_suffix,
                false => PathBuf::from(command),
            }
        };
        return admit_windows(&candidate);
    }
    let file: std::ffi::OsString = match command.contains('.') {
        true => command.into(),
        false => format!("{command}.exe").into(),
    };
    let mut directories: Vec<PathBuf> = Vec::new();
    if let Some(Some(child)) = child {
        directories.extend(std::env::split_paths(child).filter(|dir| !dir.as_os_str().is_empty()));
    }
    if let Ok(mut application) = std::env::current_exe() {
        application.pop();
        directories.push(application);
    }
    directories.extend(windows_system_directories());
    if let Some(parent) = parent {
        directories
            .extend(std::env::split_paths(&parent).filter(|dir| !dir.as_os_str().is_empty()));
    }
    for dir in directories {
        let candidate = dir.join(&file);
        if std::fs::symlink_metadata(&candidate).is_ok() {
            return admit_windows(&candidate);
        }
    }
    Err(CompositeError::Config(format!(
        "'{command}' is not on the Windows search path"
    )))
}

/// The system and Windows directories, as `GetSystemDirectoryW` and
/// `GetWindowsDirectoryW` report them: the two fixed entries of the
/// search `CreateProcessW`'s callers in std consult.
#[cfg(windows)]
fn windows_system_directories() -> Vec<PathBuf> {
    use std::os::windows::ffi::OsStringExt;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetSystemDirectoryW(buffer: *mut u16, size: u32) -> u32;
        fn GetWindowsDirectoryW(buffer: *mut u16, size: u32) -> u32;
    }
    let mut directories = Vec::new();
    for query in [GetSystemDirectoryW, GetWindowsDirectoryW] {
        let mut buffer = vec![0u16; 1024];
        // SAFETY: `buffer` is a live, writable region of exactly the
        // length passed, in UTF-16 units; the call writes at most that
        // many units and answers the length written, or the length
        // needed when the buffer is too small, or zero on failure.
        let written = unsafe { query(buffer.as_mut_ptr(), buffer.len() as u32) } as usize;
        if written == 0 || written >= buffer.len() {
            continue;
        }
        directories.push(PathBuf::from(std::ffi::OsString::from_wide(
            &buffer[..written],
        )));
    }
    directories
}

/// Admit a Windows candidate the search selected: a regular file that is
/// not a batch script dispatched through `cmd.exe`, and a PE image of
/// this target's machine and subsystem. A dangling link, a directory, a
/// malformed image or an image of another format refuses by cause.
#[cfg(windows)]
fn admit_windows(candidate: &Path) -> Result<PathBuf, CompositeError> {
    let refuse = |why: String| CompositeError::Config(format!("{}: {why}", candidate.display()));
    let metadata = std::fs::metadata(candidate)
        .map_err(|error| refuse(format!("cannot be inspected: {error}")))?;
    if !metadata.is_file() {
        return Err(refuse("is not a regular file".to_string()));
    }
    let extension = candidate
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .map(str::to_ascii_lowercase);
    if matches!(extension.as_deref(), Some("bat" | "cmd")) {
        return Err(refuse(
            "is a batch script, whose cmd.exe dispatch this resolver does not establish"
                .to_string(),
        ));
    }
    let mut file = std::fs::File::open(candidate)
        .map_err(|error| refuse(format!("cannot be read: {error}")))?;
    let image = image::inspect(&mut file, metadata.len())
        .map_err(|why| refuse(format!("is not a loadable native image: {why}")))?;
    if image.kind != image::NATIVE {
        return Err(refuse(format!(
            "is a {} image, which this target does not load",
            image.kind
        )));
    }
    canonicalize(candidate)
}

/// The selected executable, exactly as the seams carry it: a path the
/// selection already resolved, never a bare spelling to look up again.
///
/// Selection resolves the declared name ONCE, and the seams hand the
/// producer the file it chose. Searching `PATH` a second time here could
/// choose a different file — the environment is the same, but the
/// filesystem under it need not be — so a bare name is a refusal rather
/// than a second lookup, and the path form is decided by the same
/// platform predicate selection used (design D10). The selection's own
/// loader evidence was taken at selection; the producer consumes that
/// established selection, checking only that the file is still there
/// and canonicalizing it once more to the same identity.
fn selected_executable(executable: &str) -> Result<PathBuf, CompositeError> {
    if !is_explicit_path(executable) {
        return Err(CompositeError::Config(format!(
            "'{executable}' is not a path: the selected executable is resolved once, at selection"
        )));
    }
    let path = Path::new(executable);
    let metadata = std::fs::metadata(path)
        .map_err(|error| CompositeError::Config(format!("{executable}: {error}")))?;
    if !metadata.is_file() {
        return Err(CompositeError::Config(format!(
            "{executable}: is not a regular file"
        )));
    }
    #[cfg(unix)]
    if !effective_exec_access(path) {
        return Err(CompositeError::Config(format!(
            "{executable}: is not executable by this process"
        )));
    }
    canonicalize(path)
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
///
/// Production reaches discovery only through `dsh_composite_reading`,
/// which hands it the JSON reader the whole observation is counted
/// against; this real-reader wrapper is the suite's entry to discovery
/// alone.
#[cfg(test)]
fn resolve_core(executable: &str) -> Result<CorePackage, CompositeError> {
    resolve_core_reading(executable, &read_json)
}

/// `resolve_core` over an injected JSON reader, so "each identity-bearing
/// source is opened once" is a counted claim rather than an assertion in
/// a comment. Only the source of the parse is injectable; the real reader
/// stays production's.
fn resolve_core_reading(
    executable: &str,
    read_json: &dyn Fn(&Path) -> Result<Value, String>,
) -> Result<CorePackage, CompositeError> {
    let canonical = selected_executable(executable)?;
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
    // D6's measured locator is `<core>/lib/bin.js`, and that is a
    // SEPARATE requirement from the manifest agreeing with the
    // executable: a core declaring `bin.dsh: lib/other.js` and shipping
    // that file satisfied the equality above while sitting at a layout
    // the qualification never measured.
    let measured_bin = canonicalize(&dir.join("lib").join("bin.js"))?;
    if measured_bin != canonical {
        return Err(CompositeError::Config(format!(
            "{} is not the core package's lib/bin.js ({})",
            canonical.display(),
            measured_bin.display()
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
    // The hidden npm lock's own failures are the NPM LOCK's, not the
    // layout's: an operator told "the DSH layout is unreadable" looks at
    // directories, and the drifted file is a lock.
    let lock = read_json(&root.join("node_modules").join(".package-lock.json"))
        .map_err(CompositeError::NpmLock)?;
    let entry = lock
        .get("packages")
        .and_then(|packages| packages.get(CORE_KEY))
        .ok_or_else(|| CompositeError::NpmLock(format!("no {CORE_KEY} entry")))?;
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
    let manifest = read_json(&dir.join("package.json")).map_err(CompositeError::Config)?;
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
        // `Path::is_file` answers FALSE for a permission failure, a
        // metadata error and a symlink loop alike, and a false answer
        // here authorizes searching a later candidate — so a bundle the
        // loader would have taken could be skipped for one further down
        // the chain. Only true absence continues the search (council
        // return 2026-09-19).
        let manifest = dir.join("package.json");
        match std::fs::metadata(&manifest) {
            Ok(metadata) if metadata.is_file() => {}
            Ok(_) => {
                return Err(CompositeError::Config(format!(
                    "bundle '{name}': {} is not a regular file",
                    manifest.display()
                )))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(CompositeError::Config(format!(
                    "bundle '{name}': {} cannot be read: {error}",
                    manifest.display()
                )))
            }
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
    // The search ran out of candidates, and the file it ran out of is
    // named: an operator holding an otherwise complete install whose
    // plugin `package.json` is gone was told only that the bundle "does
    // not resolve" (security hold 2026-09-20, finding 7).
    Err(CompositeError::Config(format!(
        "bundle '{name}' does not resolve: no package.json found"
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
    dsh_composite_reading(seams, node, globals, &read_dir_entries, &read_json)
}

/// `dsh_composite_with` over an injected directory reader and an injected
/// JSON reader, so the two one-read claims are claims a test can falsify
/// rather than assertions in a comment:
///
/// - "the plugin's `cordis.patch.yml` is observed ONCE, inside the
///   component walk" — a listing that changes the patch after the walk
///   has read it makes a reopen visible, because the retained digest is
///   the first bytes' and a second read would hash the second bytes; and
/// - "the hidden npm lock is opened ONCE for the whole observation" — a
///   counting JSON reader sees every open across discovery AND
///   composition, where a count taken inside `resolve_core` alone could
///   not see a reopen made after it returned.
///
/// (council return 2026-09-19, finding 4.) Only the sources are
/// injectable; the real readers stay production's.
fn dsh_composite_reading(
    seams: &DshSeams,
    node: &NodeRuntime,
    globals: &[PathBuf],
    read_dir: &dyn Fn(&Path) -> std::io::Result<DirEntries>,
    read_json: &dyn Fn(&Path) -> Result<Value, String>,
) -> Result<DshComposite, CompositeError> {
    let core = resolve_core_reading(&seams.executable, read_json)?;
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
    let plugin_files = plugin_file_digests("plugin", &plugin_dir, &PLUGIN_FILES, read_dir)?;
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
            let files = plugin_file_digests("extension", dir, &EXTENSION_FILES, read_dir)?;
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

mod image;

#[cfg(test)]
mod tests;
