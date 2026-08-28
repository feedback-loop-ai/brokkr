//! Sealed secret bindings (decision 0012): the plaintext trust boundary.
//!
//! Everything that touches a secret VALUE lives in this module — the
//! `Secret` type, the operator-side store, the `{{secret:NAME}}`
//! reference scanner, and the known-plaintext masker. Outside this
//! module plaintext is reachable through exactly one method
//! (`Secret::expose_for_spawn`) with exactly one production call site:
//! the spawn injector in the exec adapter. A CI grep test pins that
//! count. Zero new dependencies: base64/hex/percent encoders and the
//! env-format parser are hand-rolled against fixed vectors — a `regex`
//! or `dotenv` edge fails the decision-0009 posture for no gain.

use std::fmt;
use std::io::Write;
use std::path::Path;

/// Names a seat may never bind: grammar-legal names that would turn the
/// env injector into a code-loading or harness-spoofing primitive.
/// Enforced at bundle compile AND at `forge secrets set`.
pub const DENYLIST: [&str; 4] = ["PATH", "IFS", "LD_PRELOAD", "LD_LIBRARY_PATH"];

/// The harness-owned prefix: `FORGE_*` names configure forge itself
/// (FORGE_CLAUDE_BIN and friends) and are never bindable.
pub const DENYLIST_PREFIX: &str = "FORGE_";

/// Values shorter than this are refused at `set`: masking a 2-byte
/// value turns the journal into `[secret:X]` confetti.
pub const MIN_VALUE_BYTES: usize = 4;

/// Values shorter than this are accepted with a warning.
pub const SHORT_VALUE_WARN_BYTES: usize = 8;

/// A secret value that cannot leak accidentally: no `Display`, no
/// `Clone`, no `Serialize`; `Debug` prints `Secret(REDACTED)`;
/// best-effort zeroization on drop (a volatile overwrite plus a
/// compiler fence — kernel buffers and pre-construction copies are out
/// of scope by decision text, and this claims nothing stronger).
///
/// ```compile_fail
/// // No Display: a Secret can never reach println!/format! directly.
/// let s = forge_protocol::secret::Secret::new(b"abcd".to_vec());
/// println!("{}", s);
/// ```
pub struct Secret {
    bytes: Vec<u8>,
}

impl Secret {
    pub fn new(bytes: Vec<u8>) -> Secret {
        Secret { bytes }
    }

    /// The SOLE plaintext egress. Its one production call site is the
    /// child-environment injection in the exec adapter's spawn path —
    /// never argv, never a template substitution, never a log. A CI
    /// grep test asserts exactly one call site outside this module.
    pub fn expose_for_spawn(&self) -> &[u8] {
        &self.bytes
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(REDACTED)")
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        wipe(&mut self.bytes);
    }
}

/// Best-effort zeroization: volatile writes the optimizer may not
/// elide, fenced so they are not reordered past the deallocation.
fn wipe(bytes: &mut [u8]) {
    for byte in bytes.iter_mut() {
        // SAFETY: `byte` is a valid, aligned, exclusive reference.
        unsafe { std::ptr::write_volatile(byte, 0) };
    }
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
}

/// One declared name bound to its resolved value. The masker reads the
/// value through private field access — it does not use (or count
/// against) `expose_for_spawn`. Debug is safe to derive: the value
/// renders through Secret's redacted Debug.
#[derive(Debug)]
pub struct BoundSecret {
    name: String,
    secret: Secret,
}

impl BoundSecret {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn secret(&self) -> &Secret {
        &self.secret
    }
}

/// The name grammar: `[A-Z][A-Z0-9_]*`.
pub fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_uppercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

pub fn denylisted(name: &str) -> bool {
    DENYLIST.contains(&name) || name.starts_with(DENYLIST_PREFIX)
}

/// Validate a name for binding: grammar plus denylist. One shared
/// refusal for bundle compile and `forge secrets set`.
pub fn validate_name(name: &str) -> Result<(), String> {
    if !valid_name(name) {
        return Err(format!(
            "secret name '{name}' does not match [A-Z][A-Z0-9_]*"
        ));
    }
    if denylisted(name) {
        return Err(format!(
            "secret name '{name}' is denylisted (PATH, IFS, LD_PRELOAD, \
             LD_LIBRARY_PATH, and the FORGE_ prefix are never bindable)"
        ));
    }
    Ok(())
}

/// Value constraints, enforced at `set`. The store format is
/// line-oriented, so a multi-line value would silently truncate into a
/// wrong secret plus garbage lines — refused instead. Returns an
/// optional warning for short-but-accepted values.
pub fn validate_value(value: &str) -> Result<Option<String>, String> {
    if value.is_empty() {
        return Err("secret value is empty".into());
    }
    if value.contains('\n') || value.contains('\r') {
        return Err("secret value must be a single line (the store is line-oriented)".into());
    }
    if value.contains('\0') {
        return Err("secret value must not contain NUL".into());
    }
    if value.len() < MIN_VALUE_BYTES {
        return Err(format!(
            "secret value is shorter than {MIN_VALUE_BYTES} bytes; masking a \
             tiny value shreds the journal's evidence trail"
        ));
    }
    if value.len() < SHORT_VALUE_WARN_BYTES {
        return Ok(Some(format!(
            "warning: secret value is shorter than {SHORT_VALUE_WARN_BYTES} \
             bytes; short values mask aggressively"
        )));
    }
    Ok(None)
}

// ---------------------------------------------------------------------
// Reference scanner: {{secret:NAME}} in exec-driver command templates.
// ---------------------------------------------------------------------

/// Scan one template part for secret references. Returns the
/// well-formed names in order of appearance, or an error describing the
/// first malformed occurrence — lowercase/empty names, interior
/// whitespace (`{{ secret:NAME }}`), and unclosed references all fail
/// closed at compile; a typo never rides into argv as literal text.
pub fn scan_secret_refs(text: &str) -> Result<Vec<String>, String> {
    let bytes = text.as_bytes();
    let mut names = Vec::new();
    let mut search = 0;
    while let Some(found) = text[search..].find("secret:") {
        let at = search + found;
        search = at + "secret:".len();
        // A reference occurrence is "secret:" preceded by "{{", with
        // only whitespace allowed (and rejected) in between.
        let mut open = at;
        while open > 0 && (bytes[open - 1] == b' ' || bytes[open - 1] == b'\t') {
            open -= 1;
        }
        if open < 2 || &bytes[open - 2..open] != b"{{" {
            continue; // not a secret reference (no opening braces)
        }
        if open != at {
            return Err(
                "malformed secret reference near '{{{{ secret:': whitespace \
                 is not allowed inside a reference"
                    .to_string(),
            );
        }
        let name_start = at + "secret:".len();
        let mut name_end = name_start;
        while name_end < bytes.len()
            && (bytes[name_end].is_ascii_uppercase()
                || bytes[name_end].is_ascii_digit()
                || bytes[name_end] == b'_')
        {
            name_end += 1;
        }
        let name = &text[name_start..name_end];
        if name.is_empty() || !valid_name(name) {
            return Err(format!(
                "malformed secret reference '{}': expected {{{{secret:NAME}}}} \
                 with NAME matching [A-Z][A-Z0-9_]*",
                &text[at - 2..(name_end + 2).min(text.len())]
            ));
        }
        if !text[name_end..].starts_with("}}") {
            return Err(format!(
                "malformed secret reference '{{{{secret:{name}': missing the \
                 closing '}}}}' (or the name continues with characters outside \
                 [A-Z0-9_])"
            ));
        }
        names.push(name.to_string());
        search = name_end + 2;
    }
    Ok(names)
}

// ---------------------------------------------------------------------
// Operator-side store: env-format file outside the bundle and outside
// version control. Names ride bundles and digests; values live here.
// ---------------------------------------------------------------------

fn store_io<T>(result: std::io::Result<T>, context: String) -> Result<T, String> {
    match result {
        Ok(value) => Ok(value),
        Err(error) => Err(format!("{context}: {error}")),
    }
}

/// Parse the store. Refuses (naming the path, never the contents) a
/// store whose permissions are broader than 0600 — ssh's posture; a
/// silent read of a world-readable file would make the create-time
/// mode meaningless. Unix-only check; recorded as a portability caveat.
fn read_store(path: &Path) -> Result<Vec<(String, Secret)>, String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let meta = store_io(
            std::fs::metadata(path),
            format!("cannot read secrets store {}", path.display()),
        )?;
        let mode = meta.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            return Err(format!(
                "refusing secrets store {}: permissions {mode:03o} are broader \
                 than 0600",
                path.display()
            ));
        }
    }
    let mut buf = store_io(
        std::fs::read(path),
        format!("cannot read secrets store {}", path.display()),
    )?;
    let mut entries: Vec<(String, Secret)> = Vec::new();
    let mut parse_error = None;
    for (index, line) in buf.split(|b| *b == b'\n').enumerate() {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.is_empty() || line.first() == Some(&b'#') {
            continue;
        }
        let Some(eq) = line.iter().position(|b| *b == b'=') else {
            parse_error = Some(format!(
                "secrets store {} line {} is not NAME=value",
                path.display(),
                index + 1
            ));
            break;
        };
        let Ok(name) = std::str::from_utf8(&line[..eq]) else {
            parse_error = Some(format!(
                "secrets store {} line {} has a non-UTF-8 name",
                path.display(),
                index + 1
            ));
            break;
        };
        if !valid_name(name) {
            parse_error = Some(format!(
                "secrets store {} line {} has an ill-formed name",
                path.display(),
                index + 1
            ));
            break;
        }
        let value = Secret::new(line[eq + 1..].to_vec());
        // Env-format convention: a later assignment overrides.
        if let Some(existing) = entries.iter_mut().find(|(n, _)| n == name) {
            existing.1 = value;
        } else {
            entries.push((name.to_string(), value));
        }
    }
    wipe(&mut buf);
    match parse_error {
        Some(error) => Err(error),
        None => Ok(entries),
    }
}

/// Atomic write: temp file in the same directory, 0600 before content,
/// rename over. An existing file's mode is never widened. No locking:
/// the store is operator-local and single-writer; a lost race is a
/// failed resolve (determinate refusal), never a leak.
#[cfg(unix)]
fn set_store_mode(path: &Path, mode: u32) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
}

fn write_store_entry(file: &mut std::fs::File, name: &str, secret: &Secret) -> std::io::Result<()> {
    file.write_all(name.as_bytes())?;
    file.write_all(b"=")?;
    file.write_all(&secret.bytes)?;
    file.write_all(b"\n")
}

fn write_store_with(
    path: &Path,
    entries: &[(String, Secret)],
    #[cfg(unix)] set_mode: impl Fn(&Path, u32) -> std::io::Result<()>,
    mut write_entry: impl FnMut(&mut std::fs::File, &str, &Secret) -> std::io::Result<()>,
) -> Result<(), String> {
    let parent = store_parent(path);
    store_io(
        std::fs::create_dir_all(&parent),
        format!("cannot create {}", parent.display()),
    )?;

    let staged = tempfile::Builder::new()
        .prefix(".secrets.env.")
        .tempfile_in(&parent);
    let mut tmp = store_io(
        staged,
        format!("cannot stage secrets store {}", path.display()),
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // 0600 on create; when replacing, intersect with the existing
        // mode so a stricter operator choice (e.g. 0400) survives.
        let mode = std::fs::metadata(path)
            .map(|m| m.permissions().mode() & 0o600)
            .unwrap_or(0o600);
        store_io(
            set_mode(tmp.path(), mode),
            "cannot set secrets store mode".into(),
        )?;
    }
    for (name, secret) in entries {
        store_io(
            write_entry(tmp.as_file_mut(), name, secret),
            format!("cannot write secrets store {}", path.display()),
        )?;
    }
    tmp.persist(path)
        .map_err(|e| format!("cannot replace secrets store {}: {e}", path.display()))?;
    Ok(())
}

fn store_parent(path: &Path) -> std::path::PathBuf {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => std::path::PathBuf::from("."),
    }
}

fn write_store(path: &Path, entries: &[(String, Secret)]) -> Result<(), String> {
    write_store_with(
        path,
        entries,
        #[cfg(unix)]
        set_store_mode,
        write_store_entry,
    )
}

/// `forge secrets set`: validate, then read-modify-write atomically.
/// Returns an optional warning (short value) on success.
pub fn store_set(path: &Path, name: &str, value: &str) -> Result<Option<String>, String> {
    validate_name(name)?;
    let warning = validate_value(value)?;
    let mut entries = if path.exists() {
        read_store(path)?
    } else {
        Vec::new()
    };
    let secret = Secret::new(value.as_bytes().to_vec());
    if let Some(existing) = entries.iter_mut().find(|(n, _)| n == name) {
        existing.1 = secret;
    } else {
        entries.push((name.to_string(), secret));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    write_store(path, &entries)?;
    Ok(warning)
}

/// `forge secrets list`: names, never values. There is no value-printing
/// verb anywhere — that is the one thing decision 0012 exists to prevent.
pub fn store_names(path: &Path) -> Result<Vec<String>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    Ok(read_store(path)?
        .into_iter()
        .map(|(name, _)| name)
        .collect())
}

/// `forge secrets remove`: Ok(true) when the name existed.
pub fn store_remove(path: &Path, name: &str) -> Result<bool, String> {
    if !path.exists() {
        return Ok(false);
    }
    let mut entries = read_store(path)?;
    let before = entries.len();
    entries.retain(|(n, _)| n != name);
    if entries.len() == before {
        return Ok(false);
    }
    write_store(path, &entries)?;
    Ok(true)
}

/// Resolve every declared name at spawn time. A missing name (or an
/// unreadable store) refuses the attempt determinately — naming the
/// secret and the store path, never the contents, never an empty-string
/// injection that turns into a downstream 401.
pub fn resolve_bindings(path: &Path, names: &[String]) -> Result<Vec<BoundSecret>, String> {
    if names.is_empty() {
        return Ok(Vec::new());
    }
    let mut entries = read_store(path)?;
    let mut bindings = Vec::with_capacity(names.len());
    for name in names {
        let index = entries.iter().position(|(n, _)| n == name).ok_or_else(|| {
            format!(
                "secret '{name}' is not in the store at {} (forge secrets set {name})",
                path.display()
            )
        })?;
        let (name, secret) = entries.swap_remove(index);
        bindings.push(BoundSecret { name, secret });
    }
    Ok(bindings)
}

// ---------------------------------------------------------------------
// Known-plaintext masking: exact matching against every bound value and
// its listed encodings — no entropy or blocklist guessing.
// ---------------------------------------------------------------------

pub type Encoder = fn(&[u8]) -> Vec<u8>;

/// THE canonical needle list, shared verbatim by the masker and the
/// layer-6 machine proof (which iterates it rather than hand-copying —
/// drift between them would let the proof pass while a listed encoding
/// leaks). Encodings of encodings are explicitly out: single pass,
/// listed shapes only, per 0012's "what this does not promise".
pub const NEEDLE_ENCODINGS: [(&str, Encoder); 9] = [
    ("raw", enc_raw),
    ("base64-std-padded", enc_b64_std),
    ("base64-std-unpadded", enc_b64_std_nopad),
    ("base64-url-padded", enc_b64_url),
    ("base64-url-unpadded", enc_b64_url_nopad),
    ("hex-lower", enc_hex_lower),
    ("hex-upper", enc_hex_upper),
    ("percent-upper", enc_pct_upper),
    ("percent-lower", enc_pct_lower),
];

fn enc_raw(bytes: &[u8]) -> Vec<u8> {
    bytes.to_vec()
}

const B64_STD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const B64_URL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn b64(bytes: &[u8], alphabet: &[u8; 64], pad: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        out.push(alphabet[(n >> 18) as usize & 63]);
        out.push(alphabet[(n >> 12) as usize & 63]);
        if chunk.len() > 1 {
            out.push(alphabet[(n >> 6) as usize & 63]);
        } else if pad {
            out.push(b'=');
        }
        if chunk.len() > 2 {
            out.push(alphabet[n as usize & 63]);
        } else if pad {
            out.push(b'=');
        }
    }
    out
}

fn enc_b64_std(bytes: &[u8]) -> Vec<u8> {
    b64(bytes, B64_STD, true)
}
fn enc_b64_std_nopad(bytes: &[u8]) -> Vec<u8> {
    b64(bytes, B64_STD, false)
}
fn enc_b64_url(bytes: &[u8]) -> Vec<u8> {
    b64(bytes, B64_URL, true)
}
fn enc_b64_url_nopad(bytes: &[u8]) -> Vec<u8> {
    b64(bytes, B64_URL, false)
}

fn hex(bytes: &[u8], digits: &[u8; 16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(digits[(byte >> 4) as usize]);
        out.push(digits[(byte & 0x0f) as usize]);
    }
    out
}

fn enc_hex_lower(bytes: &[u8]) -> Vec<u8> {
    hex(bytes, b"0123456789abcdef")
}
fn enc_hex_upper(bytes: &[u8]) -> Vec<u8> {
    hex(bytes, b"0123456789ABCDEF")
}

fn is_unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
}

fn pct(bytes: &[u8], digits: &[u8; 16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    for byte in bytes {
        if is_unreserved(*byte) {
            out.push(*byte);
        } else {
            out.push(b'%');
            out.push(digits[(byte >> 4) as usize]);
            out.push(digits[(byte & 0x0f) as usize]);
        }
    }
    out
}

fn enc_pct_upper(bytes: &[u8]) -> Vec<u8> {
    pct(bytes, b"0123456789ABCDEF")
}
fn enc_pct_lower(bytes: &[u8]) -> Vec<u8> {
    pct(bytes, b"0123456789abcdef")
}

/// Mask every needle of every bound secret in `bytes`, replacing each
/// match with `[secret:NAME]`. Operates on RAW captured bytes — callers
/// mask first and lossy-convert to string second, never the reverse
/// (UTF-8 replacement characters must not split needles). Replacement
/// is longest-needle-first so overlapping needles from multiple secrets
/// resolve deterministically.
///
/// BUFFERED-ONLY INVARIANT: every masked surface today is a complete
/// captured buffer (`wait_with_output`), so whole-buffer masking is
/// sufficient. Any future STREAMING capture must carry an overlap
/// window >= the longest needle across all bound secrets, or a needle
/// split across two chunks silently reopens the leak. Do not "optimize
/// to streaming" without carrying that window.
pub fn mask_bytes(bytes: &[u8], bindings: &[BoundSecret]) -> Vec<u8> {
    if bindings.is_empty() {
        return bytes.to_vec();
    }
    let mut needles: Vec<(Vec<u8>, &str)> = Vec::new();
    for binding in bindings {
        for (_, encode) in NEEDLE_ENCODINGS {
            let needle = encode(&binding.secret.bytes);
            if !needle.is_empty() && !needles.iter().any(|(n, _)| *n == needle) {
                needles.push((needle, binding.name.as_str()));
            }
        }
    }
    needles.sort_by_key(|needle| std::cmp::Reverse(needle.0.len()));
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    'scan: while i < bytes.len() {
        for (needle, name) in &needles {
            if bytes[i..].starts_with(needle) {
                out.extend_from_slice(format!("[secret:{name}]").as_bytes());
                i += needle.len();
                continue 'scan;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests;
