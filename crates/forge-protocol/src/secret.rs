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
            return Err(format!(
                "malformed secret reference near '{{{{ secret:': whitespace \
                 is not allowed inside a reference"
            ));
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

/// Parse the store. Refuses (naming the path, never the contents) a
/// store whose permissions are broader than 0600 — ssh's posture; a
/// silent read of a world-readable file would make the create-time
/// mode meaningless. Unix-only check; recorded as a portability caveat.
fn read_store(path: &Path) -> Result<Vec<(String, Secret)>, String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let meta = std::fs::metadata(path)
            .map_err(|e| format!("cannot read secrets store {}: {e}", path.display()))?;
        let mode = meta.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            return Err(format!(
                "refusing secrets store {}: permissions {mode:03o} are broader \
                 than 0600",
                path.display()
            ));
        }
    }
    let mut buf = std::fs::read(path)
        .map_err(|e| format!("cannot read secrets store {}: {e}", path.display()))?;
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
fn write_store(path: &Path, entries: &[(String, Secret)]) -> Result<(), String> {
    let parent = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => std::path::PathBuf::from("."),
    };
    std::fs::create_dir_all(&parent)
        .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
    let mut tmp = tempfile::Builder::new()
        .prefix(".secrets.env.")
        .tempfile_in(&parent)
        .map_err(|e| format!("cannot stage secrets store {}: {e}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // 0600 on create; when replacing, intersect with the existing
        // mode so a stricter operator choice (e.g. 0400) survives.
        let mode = std::fs::metadata(path)
            .map(|m| m.permissions().mode() & 0o600)
            .unwrap_or(0o600);
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(mode))
            .map_err(|e| format!("cannot set secrets store mode: {e}"))?;
    }
    for (name, secret) in entries {
        tmp.as_file_mut()
            .write_all(name.as_bytes())
            .and_then(|_| tmp.as_file_mut().write_all(b"="))
            .and_then(|_| tmp.as_file_mut().write_all(&secret.bytes))
            .and_then(|_| tmp.as_file_mut().write_all(b"\n"))
            .map_err(|e| format!("cannot write secrets store {}: {e}", path.display()))?;
    }
    tmp.persist(path)
        .map_err(|e| format!("cannot replace secrets store {}: {e}", path.display()))?;
    Ok(())
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
    Ok(read_store(path)?.into_iter().map(|(name, _)| name).collect())
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
    needles.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
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
mod tests {
    use super::*;

    fn bound(name: &str, value: &[u8]) -> BoundSecret {
        BoundSecret {
            name: name.to_string(),
            secret: Secret::new(value.to_vec()),
        }
    }

    // -------------------------------------------------- layer 4: type

    #[test]
    fn debug_prints_redacted_never_the_value() {
        let secret = Secret::new(b"hunter22".to_vec());
        assert_eq!(format!("{secret:?}"), "Secret(REDACTED)");
    }

    #[test]
    fn wipe_zeroizes_the_buffer() {
        let mut buf = b"hunter22".to_vec();
        wipe(&mut buf);
        assert!(buf.iter().all(|b| *b == 0));
    }

    // ------------------------------------------- layer 1: name grammar

    #[test]
    fn name_grammar_vectors() {
        for good in ["A", "GH_TOKEN", "X9", "A_B_C", "TOKEN2"] {
            assert!(valid_name(good), "{good} must be valid");
        }
        for bad in ["", "a", "gh_token", "9X", "_A", "A-B", "A B", "TOKEn", "Ä"] {
            assert!(!valid_name(bad), "{bad:?} must be invalid");
        }
    }

    #[test]
    fn denylist_covers_exact_names_and_forge_prefix() {
        for name in ["PATH", "IFS", "LD_PRELOAD", "LD_LIBRARY_PATH", "FORGE_X", "FORGE_"] {
            assert!(denylisted(name), "{name} must be denylisted");
            assert!(validate_name(name).is_err(), "{name} must not validate");
        }
        for name in ["PATHS", "GH_TOKEN", "LD", "FORG_X"] {
            assert!(!denylisted(name), "{name} must not be denylisted");
        }
    }

    // ------------------------------------------------ layer 1: scanner

    #[test]
    fn scanner_finds_well_formed_references() {
        assert_eq!(
            scan_secret_refs("curl -H 'auth: {{secret:GH_TOKEN}}' {{secret:API_KEY}}").unwrap(),
            vec!["GH_TOKEN", "API_KEY"]
        );
        assert_eq!(scan_secret_refs("no references here").unwrap(), Vec::<String>::new());
        // "secret:" without opening braces is not a reference.
        assert_eq!(scan_secret_refs("https://x/secret:thing").unwrap(), Vec::<String>::new());
    }

    #[test]
    fn scanner_rejects_malformed_occurrences() {
        for (text, why) in [
            ("{{secret:gh_token}}", "lowercase"),
            ("{{secret:}}", "empty"),
            ("{{ secret:NAME }}", "interior whitespace"),
            ("{{secret: NAME}}", "space after colon"),
            ("{{secret:NAME", "unclosed"),
            ("{{secret:NAMe}}", "partial lowercase tail"),
            ("{{secret:9NAME}}", "leading digit"),
        ] {
            assert!(scan_secret_refs(text).is_err(), "{why}: {text:?} must be malformed");
        }
    }

    #[test]
    fn scanner_error_never_silently_passes_typos_into_argv() {
        let error = scan_secret_refs("{{secret:GH_TOKEN").unwrap_err();
        assert!(error.contains("GH_TOKEN"), "error names the reference: {error}");
    }

    // ---------------------------------------------- layer 5: encoders

    #[test]
    fn base64_matches_rfc4648_vectors() {
        // RFC 4648 test vectors.
        for (input, padded) in [
            (&b""[..], ""),
            (b"f", "Zg=="),
            (b"fo", "Zm8="),
            (b"foo", "Zm9v"),
            (b"foob", "Zm9vYg=="),
            (b"fooba", "Zm9vYmE="),
            (b"foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(enc_b64_std(input), padded.as_bytes());
            assert_eq!(
                enc_b64_std_nopad(input),
                padded.trim_end_matches('=').as_bytes()
            );
        }
        // URL-safe alphabet swaps +/ for -_.
        assert_eq!(enc_b64_std(&[0xfb, 0xff]), b"+/8=");
        assert_eq!(enc_b64_url(&[0xfb, 0xff]), b"-_8=");
        assert_eq!(enc_b64_url_nopad(&[0xfb, 0xff]), b"-_8");
    }

    #[test]
    fn hex_and_percent_match_fixed_vectors() {
        assert_eq!(enc_hex_lower(b"\x00\xabZ"), b"00ab5a");
        assert_eq!(enc_hex_upper(b"\x00\xabZ"), b"00AB5A");
        assert_eq!(enc_pct_upper(b"a+b !"), b"a%2Bb%20%21");
        assert_eq!(enc_pct_lower(b"a+b !"), b"a%2bb%20%21");
        assert_eq!(enc_pct_upper(b"AZaz09-._~"), b"AZaz09-._~", "unreserved pass through");
    }

    // ------------------------------------------------ layer 5: masker

    #[test]
    fn masker_replaces_every_listed_encoding() {
        let value = b"tok3n+v4lue!";
        let bindings = vec![bound("API_TOKEN", value)];
        for (label, encode) in NEEDLE_ENCODINGS {
            let leak = encode(value);
            let mut text = b"before ".to_vec();
            text.extend_from_slice(&leak);
            text.extend_from_slice(b" after");
            let masked = mask_bytes(&text, &bindings);
            assert_eq!(
                masked, b"before [secret:API_TOKEN] after",
                "{label} must mask"
            );
        }
    }

    #[test]
    fn masker_handles_multiple_secrets_longest_needle_first() {
        // ALPHA's value is a strict prefix of BETA's: the longer needle
        // must win where both match.
        let bindings = vec![bound("ALPHA", b"abcdef"), bound("BETA", b"abcdefgh")];
        let masked = mask_bytes(b"x abcdefgh y abcdef z", &bindings);
        assert_eq!(masked, b"x [secret:BETA] y [secret:ALPHA] z");
    }

    #[test]
    fn masker_passes_needle_free_text_through_byte_identical() {
        let bindings = vec![bound("API_TOKEN", b"tok3n+v4lue!")];
        let text = b"ordinary build output, no secrets \xff\xfe here".to_vec();
        assert_eq!(mask_bytes(&text, &bindings), text);
        assert_eq!(mask_bytes(b"anything", &[]), b"anything");
    }

    #[test]
    fn masker_operates_on_bytes_so_invalid_utf8_neighbors_still_mask() {
        let bindings = vec![bound("API_TOKEN", b"tok3n+v4lue!")];
        let mut text = vec![0xff, 0xfe];
        text.extend_from_slice(b"tok3n+v4lue!");
        text.push(0xff);
        let masked = mask_bytes(&text, &bindings);
        let mut expected = vec![0xff, 0xfe];
        expected.extend_from_slice(b"[secret:API_TOKEN]");
        expected.push(0xff);
        assert_eq!(masked, expected);
    }

    #[test]
    fn masker_masks_adjacent_and_repeated_needles() {
        let bindings = vec![bound("K", b"vvvv")];
        assert_eq!(mask_bytes(b"vvvvvvvv", &bindings), b"[secret:K][secret:K]");
    }

    // ------------------------------------------------- layer 2: store

    #[test]
    fn store_round_trip_set_list_remove() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secrets.env");
        assert_eq!(store_names(&path).unwrap(), Vec::<String>::new());
        store_set(&path, "GH_TOKEN", "tokenvalue1").unwrap();
        store_set(&path, "API_KEY", "keyvalue22").unwrap();
        assert_eq!(store_names(&path).unwrap(), vec!["API_KEY", "GH_TOKEN"]);
        store_set(&path, "GH_TOKEN", "rotated-value").unwrap();
        assert_eq!(store_names(&path).unwrap(), vec!["API_KEY", "GH_TOKEN"]);
        let bindings =
            resolve_bindings(&path, &["GH_TOKEN".to_string()]).unwrap();
        assert_eq!(bindings[0].secret().expose_for_spawn(), b"rotated-value");
        assert!(store_remove(&path, "GH_TOKEN").unwrap());
        assert!(!store_remove(&path, "GH_TOKEN").unwrap(), "second remove finds nothing");
        assert_eq!(store_names(&path).unwrap(), vec!["API_KEY"]);
    }

    #[test]
    fn resolve_missing_name_names_the_secret_and_the_path_only() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secrets.env");
        store_set(&path, "PRESENT", "somevalue").unwrap();
        let error = resolve_bindings(&path, &["ABSENT".to_string()]).unwrap_err();
        assert!(error.contains("ABSENT"), "{error}");
        assert!(error.contains("secrets.env"), "{error}");
        assert!(!error.contains("somevalue"), "never the contents: {error}");
    }

    #[test]
    fn store_set_refuses_every_rejected_value_class() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secrets.env");
        for (value, why) in [
            ("", "empty"),
            ("two\nlines", "multi-line"),
            ("nul\0byte", "NUL"),
            ("abc", "under 4 bytes"),
        ] {
            assert!(store_set(&path, "NAME", value).is_err(), "{why} must refuse");
        }
        assert!(
            store_set(&path, "NAME", "short7c").unwrap().is_some(),
            "under 8 bytes warns"
        );
        assert!(store_set(&path, "NAME", "longenough").unwrap().is_none());
        for name in ["PATH", "FORGE_X", "lower", "9BAD"] {
            assert!(store_set(&path, name, "longenough").is_err(), "{name} must refuse");
        }
    }

    #[cfg(unix)]
    #[test]
    fn store_created_0600_and_replace_never_widens() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secrets.env");
        store_set(&path, "GH_TOKEN", "tokenvalue1").unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "created 0600");
        // Tighten to 0400: a rewrite must not widen it back.
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o400)).unwrap();
        store_set(&path, "API_KEY", "keyvalue22").unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o400, "atomic replace preserves the stricter mode");
    }

    #[cfg(unix)]
    #[test]
    fn broader_than_0600_store_refuses_on_read() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secrets.env");
        store_set(&path, "GH_TOKEN", "tokenvalue1").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        let error = store_names(&path).unwrap_err();
        assert!(error.contains("broader"), "{error}");
        assert!(error.contains("secrets.env"), "names the path: {error}");
        assert!(!error.contains("tokenvalue1"), "never the contents: {error}");
    }

    #[test]
    fn store_parse_skips_comments_and_refuses_garbage() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secrets.env");
        std::fs::write(&path, "# comment\n\nGH_TOKEN=abcd1234\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        assert_eq!(store_names(&path).unwrap(), vec!["GH_TOKEN"]);
        std::fs::write(&path, "no equals sign\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        let error = store_names(&path).unwrap_err();
        assert!(error.contains("line 1"), "{error}");
        assert!(!error.contains("equals"), "never the contents: {error}");
    }
}
