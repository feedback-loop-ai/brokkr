//! Sealed secret bindings (decision 0012): the plaintext trust boundary.
//!
//! Everything that touches a secret VALUE lives in this module — the
//! `Secret` type, the operator-side store, the `{{secret:NAME}}`
//! reference scanner, the known-plaintext masker, and the broker's final
//! guard, which reads a bound value by what a number is worth. Outside this
//! module plaintext is reachable through exactly one method
//! (`Secret::expose_for_spawn`) with exactly one production call site,
//! this module included: the spawn injector [`bind_environment`] every
//! harness spawn shares. A CI grep test pins that count. Zero new
//! dependencies: base64/hex/percent encoders and the env-format parser
//! are hand-rolled against fixed vectors — a `regex` or `dotenv` edge
//! fails the decision-0009 posture for no gain.

use std::fmt;
use std::io::Write;
use std::iter::Peekable;
use std::path::Path;

/// Names a seat may never bind: grammar-legal names that would turn the
/// env injector into a code-loading or harness-spoofing primitive.
/// Enforced at bundle compile AND at `brokkr secrets set`.
pub const DENYLIST: [&str; 4] = ["PATH", "IFS", "LD_PRELOAD", "LD_LIBRARY_PATH"];

/// The harness-owned prefix: `BROKKR_*` names configure the harness
/// itself (BROKKR_CODEX_BIN and friends) and are never bindable. The
/// pre-rename spelling left the denylist (#355): the harness reads it
/// only to refuse it (`overrides`), so it can stop a seat, never aim one.
pub const DENYLIST_PREFIXES: [&str; 1] = ["BROKKR_"];

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
/// let s = brokkr_protocol::secret::Secret::new(b"abcd".to_vec());
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
    /// child-environment injection in [`bind_environment`] — never argv,
    /// never a template substitution, never a log. A CI grep test asserts
    /// exactly one call site across every production module, this one too.
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

/// A binding the spawn injector cannot hand a child. The text names the
/// binding, never its value.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum BindError {
    #[error("secret '{0}' is not valid UTF-8")]
    NotUtf8(String),
}

/// Injection discipline (decision 0012, layer 3): values reach the child
/// ONLY through its environment, resolved at spawn time — never argv
/// (/proc/*/cmdline is world-readable), never the template. Every harness
/// spawn — claude, lanetally, codex, dsh and exec alike — binds through
/// here, so this holds the sole production call site of
/// expose_for_spawn, CI-grep pinned. A declared name overrides any
/// pre-existing env entry: the declaration is in the reviewed charter, so
/// a collision is visible at review time.
pub(crate) fn bind_environment(
    command: &mut std::process::Command,
    bindings: &[BoundSecret],
) -> Result<(), BindError> {
    for binding in bindings {
        let value = std::str::from_utf8(binding.secret.expose_for_spawn())
            .map_err(|_| BindError::NotUtf8(binding.name.clone()))?;
        command.env(&binding.name, value);
    }
    Ok(())
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
    DENYLIST.contains(&name)
        || DENYLIST_PREFIXES
            .iter()
            .any(|prefix| name.starts_with(prefix))
}

/// Validate a name for binding: grammar plus denylist. One shared
/// refusal for bundle compile and `brokkr secrets set`.
pub fn validate_name(name: &str) -> Result<(), String> {
    checked_name(name).map_err(|refusal| refusal.to_string())
}

/// [`validate_name`]'s checks, refusing in a typed cause.
fn checked_name(name: &str) -> Result<(), NameRefusal<'_>> {
    if !valid_name(name) {
        return Err(NameRefusal::Grammar { name });
    }
    if denylisted(name) {
        return Err(NameRefusal::Denylisted { name });
    }
    Ok(())
}

/// Value constraints, enforced at `set`. The store format is
/// line-oriented, so a multi-line value would silently truncate into a
/// wrong secret plus garbage lines — refused instead. Returns an
/// optional warning for short-but-accepted values.
pub fn validate_value(value: &str) -> Result<Option<String>, String> {
    checked_value(value).map_err(|refusal| refusal.to_string())
}

/// [`validate_value`]'s checks, refusing in a typed cause; the warning
/// stays text on the success path.
fn checked_value(value: &str) -> Result<Option<String>, ValueRefusal> {
    if value.is_empty() {
        return Err(ValueRefusal::Empty);
    }
    if value.contains('\n') || value.contains('\r') {
        return Err(ValueRefusal::MultiLine);
    }
    if value.contains('\0') {
        return Err(ValueRefusal::Nul);
    }
    if value.len() < MIN_VALUE_BYTES {
        return Err(ValueRefusal::TooShort);
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
    checked_refs(text).map_err(|refusal| refusal.to_string())
}

/// [`scan_secret_refs`]'s walk, refusing in a typed cause.
fn checked_refs(text: &str) -> Result<Vec<String>, ReferenceRefusal<'_>> {
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
            return Err(ReferenceRefusal::Whitespace);
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
            return Err(ReferenceRefusal::Name {
                quoted: quoted_reference(text, at, name_end),
            });
        }
        if !text[name_end..].starts_with("}}") {
            return Err(ReferenceRefusal::Unclosed { name });
        }
        names.push(name.to_string());
        search = name_end; // the closing "}}" cannot begin "secret:"
    }
    Ok(names)
}

/// The text a refused reference name quotes: from its `{{` through two
/// bytes past the name, or the end of `text`, rounded up to the next
/// character boundary so a multibyte character there is quoted whole
/// rather than split. `text.len()` is a boundary, so the loop ends.
fn quoted_reference(text: &str, at: usize, name_end: usize) -> &str {
    let mut end = (name_end + 2).min(text.len());
    while !text.is_char_boundary(end) {
        end += 1;
    }
    &text[at - 2..end]
}

// ---------------------------------------------------------------------
// Operator-side store: env-format file outside the bundle and outside
// version control. Names ride bundles and digests; values live here.
// ---------------------------------------------------------------------

/// Parse the store at `path` through the one handle the typed reader
/// checks and reads. The typed cause becomes text only here.
fn read_store(path: &Path) -> Result<Vec<(String, Secret)>, String> {
    open_store(path).map_err(|error| error.to_string())
}

/// The path's mode refuses before any open, as it always has, so a broad
/// FIFO cannot block the open and a broad unreadable file refuses on its
/// mode. The reader then checks the opened handle's own mode again.
fn open_store(path: &Path) -> Result<Vec<(String, Secret)>, store::StoreError> {
    let io = |source| store::StoreError::Io {
        path: path.to_path_buf(),
        source,
    };
    store::check_mode(&std::fs::metadata(path).map_err(io)?, path)?;
    let file = std::fs::File::open(path).map_err(io)?;
    store::read_store_file(file, path)
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
) -> Result<(), store::WriteRefusal> {
    use store::WriteRefusal;
    let parent = store_parent(path);
    let at = || path.to_path_buf();
    let create = |source| WriteRefusal::Create {
        parent: parent.clone(),
        source,
    };
    std::fs::create_dir_all(&parent).map_err(create)?;

    let staged = tempfile::Builder::new()
        .prefix(".secrets.env.")
        .tempfile_in(&parent);
    let mut tmp = staged.map_err(|source| WriteRefusal::Stage { path: at(), source })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // 0600 on create; when replacing, intersect with the existing
        // mode so a stricter operator choice (e.g. 0400) survives.
        let mode = std::fs::metadata(path)
            .map(|m| m.permissions().mode() & 0o600)
            .unwrap_or(0o600);
        set_mode(tmp.path(), mode).map_err(|source| WriteRefusal::Mode { source })?;
    }
    for (name, secret) in entries {
        write_entry(tmp.as_file_mut(), name, secret)
            .map_err(|source| WriteRefusal::Write { path: at(), source })?;
    }
    // Only the I/O error is kept: the staged file drops here, as it did.
    let replace = |e: tempfile::PersistError| WriteRefusal::Replace {
        path: at(),
        source: e.error,
    };
    tmp.persist(path).map_err(replace)?;
    Ok(())
}

fn store_parent(path: &Path) -> std::path::PathBuf {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => std::path::PathBuf::from("."),
    }
}

/// Write `entries` to the store at `path`. The typed cause becomes text
/// only here.
fn write_store(path: &Path, entries: &[(String, Secret)]) -> Result<(), String> {
    write_store_with(
        path,
        entries,
        #[cfg(unix)]
        set_store_mode,
        write_store_entry,
    )
    .map_err(|error| error.to_string())
}

/// `brokkr secrets set`: validate, then read-modify-write atomically.
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

/// `brokkr secrets list`: names, never values. There is no value-printing
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

/// `brokkr secrets remove`: Ok(true) when the name existed.
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
    open_store(path)
        .and_then(|entries| store::bind_names(entries, names, path))
        .map_err(|error| error.to_string())
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
/// (UTF-8 replacement characters must not split needles). Every byte
/// any occurrence of any needle covers is masked, and occurrences that
/// overlap or touch read one marker, named for the binding whose
/// occurrence starts first (the longest needle's where several start at
/// one byte), so overlapping needles resolve deterministically.
///
/// BUFFERED-ONLY INVARIANT: every masked surface today is a complete
/// captured buffer (`wait_with_output`), so whole-buffer masking is
/// sufficient. Any future STREAMING capture must carry an overlap
/// window >= the longest needle across all bound secrets, or a needle
/// split across two chunks silently reopens the leak. Do not "optimize
/// to streaming" without carrying that window.
pub fn mask_bytes(bytes: &[u8], bindings: &[BoundSecret]) -> Vec<u8> {
    mask_needles(bytes, bindings, &NEEDLE_ENCODINGS)
}

/// [`mask_bytes`] for text a transcript reader projected (#380), which
/// also masks each value as it reads inside a JSON string.
///
/// A projector renders a structured sub-value — a tool call's arguments
/// object, a shell action — as serialised JSON, so a value holding a
/// `"`, a `\` or a control character reaches the text re-escaped and
/// never matches the raw needle. The extra needle is that value's own
/// JSON string escaping, the same bytes the projector's serialiser
/// writes. It stays off the shared [`NEEDLE_ENCODINGS`]: the surfaces
/// the driver masks decode JSON before masking instead.
pub fn mask_projected(text: &str, bindings: &[BoundSecret]) -> String {
    let mut encodings = NEEDLE_ENCODINGS.to_vec();
    encodings.push(("json-string", enc_json_string));
    String::from_utf8_lossy(&mask_needles(text.as_bytes(), bindings, &encodings)).into_owned()
}

/// A value as it reads between the quotes of a serialised JSON string.
fn enc_json_string(bytes: &[u8]) -> Vec<u8> {
    let quoted = serde_json::Value::String(String::from_utf8_lossy(bytes).into_owned()).to_string();
    quoted.as_bytes()[1..quoted.len() - 1].to_vec()
}

/// Every needle of every binding in `encodings`, each once, with the name
/// of the first binding it is found for, longest first: what the masker
/// replaces, and what the final guard finds.
fn needles<'b>(
    bindings: &'b [BoundSecret],
    encodings: &[(&str, Encoder)],
) -> Vec<(Vec<u8>, &'b str)> {
    let mut needles: Vec<(Vec<u8>, &str)> = Vec::new();
    for binding in bindings {
        for (_, encode) in encodings {
            let needle = encode(&binding.secret.bytes);
            if !needle.is_empty() && !needles.iter().any(|(n, _)| *n == needle) {
                needles.push((needle, binding.name.as_str()));
            }
        }
    }
    needles.sort_by_key(|needle| std::cmp::Reverse(needle.0.len()));
    needles
}

fn mask_needles(bytes: &[u8], bindings: &[BoundSecret], encodings: &[(&str, Encoder)]) -> Vec<u8> {
    if bindings.is_empty() {
        return bytes.to_vec();
    }
    let needles = needles(bindings, encodings);
    let mut out = Vec::with_capacity(bytes.len());
    let mut clear = 0;
    for (start, end, name) in covered(bytes, &needles) {
        out.extend_from_slice(&bytes[clear..start]);
        out.extend_from_slice(format!("[secret:{name}]").as_bytes());
        clear = end;
    }
    out.extend_from_slice(&bytes[clear..]);
    out
}

/// Every run of `bytes` that occurrences of `needles` cover, in order, as
/// where it begins and ends and the name the run's marker reads (#1213,
/// decision 0012's addendum of 2026-11-21): the union of every occurrence
/// of every needle, so a needle starting inside another's match is masked
/// whole. Occurrences that overlap or touch are one run, named for the
/// binding whose occurrence starts first; of those starting at one byte,
/// the longest needle's, as it always was. Each needle is found in one
/// pass of [`occurrences`], so the cost is linear in the text times the
/// needles.
fn covered<'n>(bytes: &[u8], needles: &[(Vec<u8>, &'n str)]) -> Vec<(usize, usize, &'n str)> {
    // Each needle's next unread occurrence, found as the merge reaches it,
    // so the scratch is the needles', never a slot per byte of the text.
    let mut heads: Vec<_> = needles
        .iter()
        .map(|(needle, _)| Occurrences::new(bytes, needle).peekable())
        .collect();
    let mut runs: Vec<(usize, usize, &str)> = Vec::new();
    // The earliest unread occurrence; of those starting at one byte, the
    // first needle's, so the longest. Each turn reads one start, so the
    // turns are at most the text's bytes, each costing the needles.
    while let Some((start, index)) = earliest(&mut heads) {
        for head in &mut heads {
            head.next_if_eq(&start);
        }
        let (needle, name) = &needles[index];
        let end = start + needle.len();
        match runs.last_mut() {
            Some(run) if start <= run.1 => run.1 = run.1.max(end),
            _ => runs.push((start, end, name)),
        }
    }
    runs
}

/// The least next start among `heads`, and the index of the first head
/// it is next in.
fn earliest(heads: &mut [Peekable<Occurrences<'_>>]) -> Option<(usize, usize)> {
    let next =
        |(index, head): (usize, &mut Peekable<Occurrences<'_>>)| Some((*head.peek()?, index));
    heads.iter_mut().enumerate().filter_map(next).min()
}

/// Where every occurrence of `needle`, which is not empty, starts in
/// `bytes`, overlapping ones included, in order: one Knuth–Morris–Pratt
/// pass, linear in the text and the needle, read as it is consumed.
struct Occurrences<'a> {
    bytes: &'a [u8],
    needle: &'a [u8],
    /// `border[i]`: the longest proper prefix of `needle[..=i]` that is
    /// also its suffix.
    border: Vec<usize>,
    /// How many bytes of `bytes` are read.
    read: usize,
    /// How much of `needle` the bytes read end with.
    matched: usize,
}

impl<'a> Occurrences<'a> {
    fn new(bytes: &'a [u8], needle: &'a [u8]) -> Occurrences<'a> {
        let mut border = vec![0; needle.len()];
        let mut matched = 0;
        for i in 1..needle.len() {
            matched = advance(needle, &border, matched, needle[i]);
            border[i] = matched;
        }
        Occurrences {
            bytes,
            needle,
            border,
            read: 0,
            matched: 0,
        }
    }
}

impl Iterator for Occurrences<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        while let Some(&byte) = self.bytes.get(self.read) {
            self.read += 1;
            self.matched = advance(self.needle, &self.border, self.matched, byte);
            if self.matched == self.needle.len() {
                self.matched = self.border[self.matched - 1];
                return Some(self.read - self.needle.len());
            }
        }
        None
    }
}

/// How much of `needle` is matched after `byte`, when `matched` bytes of
/// it, fewer than all, were matched before it.
fn advance(needle: &[u8], border: &[usize], mut matched: usize, byte: u8) -> usize {
    while matched > 0 && byte != needle[matched] {
        matched = border[matched - 1];
    }
    matched + usize::from(byte == needle[matched])
}

/// [`mask_bytes`] over every string inside a JSON value — object keys
/// included — in place.
///
/// For the surfaces that reach the driver as JSON: a harness's stream,
/// a harness's transcript and a seat's result file. A secret the model
/// echoed arrives there JSON-escaped on the wire (a `"`, a `\`, a tab),
/// so it never matches in the raw bytes; it must be masked on the
/// DECODED strings, and before anything clamps or rewrites them — a
/// prefix cut at a length limit, or whitespace collapsed for a one-line
/// reason, no longer matches the whole needle. Keys are masked too
/// because a result file's keys are the seat's to choose. Numbers are
/// left as numbers: a count must stay a count for the seat record, so a
/// wholly numeric secret echoed as a bare JSON number is not rewritten
/// (the secrets guide says so).
pub fn mask_json(value: &mut serde_json::Value, bindings: &[BoundSecret]) {
    match value {
        serde_json::Value::String(text) => {
            *text = mask_text(text.as_bytes(), bindings);
        }
        serde_json::Value::Array(items) => {
            for item in items {
                mask_json(item, bindings);
            }
        }
        serde_json::Value::Object(map) => {
            let entries = std::mem::take(map);
            for (key, mut item) in entries {
                mask_json(&mut item, bindings);
                map.insert(mask_text(key.as_bytes(), bindings), item);
            }
        }
        _ => {}
    }
}

/// `bytes` masked by [`mask_bytes`] and only then decoded, lossily, as
/// text: as [`mask_json`] writes a string, and as the broker's output edge
/// writes free text and a server's standard error.
pub fn mask_text(bytes: &[u8], bindings: &[BoundSecret]) -> String {
    String::from_utf8_lossy(&mask_bytes(bytes, bindings)).into_owned()
}

pub use guard::{json_numbers, Guard};
pub use handoff::{exec_confined, send_environment};

mod guard;
pub(crate) mod handoff;
mod refusal;
mod store;

use refusal::{NameRefusal, ReferenceRefusal, ValueRefusal};

#[cfg(test)]
mod tests;
