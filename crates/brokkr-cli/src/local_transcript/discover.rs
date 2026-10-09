//! Safe local discovery (proposed decision 0055, design D3): one validated
//! reference's unique source, located by handle-relative traversal under
//! its recorded home and each kind's closed scope, before any body byte is
//! interpreted.

use brokkr_view::transcript::{TranscriptKind, Unavailable, ValidReference};
use serde_json::Value;

use super::safe_fs;

/// A safely opened candidate source: the lossless confirmed path and the
/// verified handle retained for the body read.
pub(crate) struct AdmittedSource {
    pub path: String,
    pub file: safe_fs::OpenedFile,
    /// The leaf's lossless identity, recorded at discovery and rechecked
    /// through the same held handle at the read boundary.
    pub identity: safe_fs::Identity,
}

/// The outcome of safe discovery, before any body byte is interpreted.
pub(crate) enum Discovery {
    Admitted(AdmittedSource),
    Refused(Unavailable, Option<String>),
}

/// One lookup's collected facts, resolved after the bounded scope is
/// exhausted so iterator order never chooses the explanation.
#[derive(Default)]
#[expect(clippy::struct_excessive_bools, reason = "baseline 2026-09, #288")]
struct Lookup {
    candidates: Vec<AdmittedSource>,
    examined: usize,
    unsafe_seen: bool,
    io_seen: bool,
    limit_hit: bool,
    invalid_depth_seen: bool,
}

impl Lookup {
    /// Entries still chargeable to the one 10,000-entry discovery budget.
    fn remaining(&self) -> usize {
        brokkr_view::transcript::DISCOVERY_LIMIT.saturating_sub(self.examined)
    }

    /// Charge one directory entry that was actually examined.
    fn note_entry(&mut self) {
        self.examined += 1;
    }

    fn resolve(mut self, kind: TranscriptKind) -> Discovery {
        if self.limit_hit && self.candidates.len() < 2 {
            return Discovery::Refused(Unavailable::DiscoveryLimit, None);
        }
        if self.candidates.len() > 1 {
            return Discovery::Refused(Unavailable::AmbiguousSource, None);
        }
        // An unreadable sibling prevents proving that a sole candidate is
        // unique, so I/O failure outranks a provisional admission.
        if self.io_seen {
            return Discovery::Refused(Unavailable::Unreadable, None);
        }
        if let Some(candidate) = self.candidates.pop() {
            return Discovery::Admitted(candidate);
        }
        if self.unsafe_seen {
            return Discovery::Refused(Unavailable::UnsafePath, None);
        }
        if kind == TranscriptKind::DshSession && self.invalid_depth_seen {
            return Discovery::Refused(
                Unavailable::NotFound,
                Some("no valid depth-zero DSH session header".to_string()),
            );
        }
        Discovery::Refused(Unavailable::NotFound, None)
    }
}

/// Enumerate one directory under the remaining discovery budget. A
/// listing larger than the budget sets the limit fact without
/// materializing the extra names; an I/O failure is recorded and yields
/// `None`, ending this lookup.
fn bounded_entries(dir: &safe_fs::Dir, lookup: &mut Lookup) -> Option<Vec<std::ffi::OsString>> {
    match dir.entries_bounded(lookup.remaining()) {
        Ok((names, truncated)) => {
            if truncated {
                lookup.limit_hit = true;
            }
            Some(names)
        }
        Err(_) => {
            lookup.io_seen = true;
            None
        }
    }
}

fn root_error(error: &std::io::Error) -> Discovery {
    let reason = if error.kind() == std::io::ErrorKind::NotFound {
        Unavailable::NotFound
    } else {
        Unavailable::Unreadable
    };
    Discovery::Refused(reason, None)
}

/// Whether a recorded home uses the current target's native absolute
/// syntax, so that a foreign-platform spelling is refused before any
/// filesystem call rather than reinterpreted as a host-relative path.
fn native_absolute_home(home: &str) -> bool {
    home.starts_with('/')
}

/// Locate one safely opened local source for a validated reference. No
/// transcript content is read for Claude or Codex; a DSH candidate's
/// bounded opening header is read to prove depth-zero ownership. The
/// recorded home is canonicalized once (it may itself be a symlink) and
/// only that canonical directory is opened as the traversal root.
pub(crate) fn discover(reference: &ValidReference) -> Discovery {
    // Lexical validity is portable; native I/O is not. A recorded home
    // spelled in another platform's absolute syntax is refused here,
    // before canonicalization, and its bytes stay echoed unchanged.
    if !native_absolute_home(&reference.home) {
        return Discovery::Refused(Unavailable::InvalidReference, None);
    }
    let canonical = match std::fs::canonicalize(&reference.home) {
        Ok(path) => path,
        Err(error) => return root_error(&error),
    };
    let Some(home) = canonical.to_str() else {
        // A non-Unicode root has no lossless output spelling.
        return Discovery::Refused(Unavailable::Unreadable, None);
    };
    let root = match safe_fs::Dir::open_root(home) {
        Ok(root) => root,
        Err(error) => return root_error(&error),
    };
    let mut lookup = Lookup::default();
    match reference.kind {
        TranscriptKind::ClaudeSession => discover_claude(&root, home, reference, &mut lookup),
        TranscriptKind::CodexThread => discover_codex(&root, home, reference, &mut lookup),
        TranscriptKind::DshSession => discover_dsh(&root, home, reference, &mut lookup),
        TranscriptKind::None => {}
    }
    lookup.resolve(reference.kind)
}

/// D3: an acquisition is current only when a fresh handle-relative walk
/// finds the same unique safe candidate at the same recorded path and
/// identity. A swapped leaf or ancestor name, a changed DSH opening
/// header or a newly ambiguous root fails closed before any byte of the
/// retained handle is read.
pub(super) fn acquisition_is_current(
    reference: &ValidReference,
    admitted: &AdmittedSource,
) -> bool {
    // The point before the read-boundary acquisition re-walk. A test-owned
    // change is timed here so the real re-walk observes it; the seam never
    // replaces the re-walk's answer.
    #[cfg(test)]
    safe_fs::fault::point(safe_fs::fault::ChangeAt::BeforeRewalk);
    matches!(
        discover(reference),
        Discovery::Admitted(recheck)
            if recheck.identity == admitted.identity && recheck.path == admitted.path
    )
}

fn join_path(base: &str, name: &str) -> String {
    format!("{base}/{name}")
}

fn discover_claude(
    root: &safe_fs::Dir,
    home: &str,
    reference: &ValidReference,
    lookup: &mut Lookup,
) {
    let file_name = format!("{}.jsonl", reference.locator);
    let Some(entries) = bounded_entries(root, lookup) else {
        return;
    };
    for name in entries {
        lookup.note_entry();
        let Some(name) = name.to_str() else {
            lookup.io_seen = true;
            continue;
        };
        let Some(project) = child_dir(root, name, lookup) else {
            continue;
        };
        match project.child(std::ffi::OsStr::new(&file_name)) {
            Ok(safe_fs::Child::File(file)) => match file.identity() {
                Ok(identity) => lookup.candidates.push(AdmittedSource {
                    path: join_path(&join_path(home, name), &file_name),
                    identity,
                    file,
                }),
                Err(_) => lookup.io_seen = true,
            },
            Ok(safe_fs::Child::Unsafe) => lookup.unsafe_seen = true,
            Ok(_) => {}
            Err(_) => lookup.io_seen = true,
        }
    }
}

/// The whole-filename token predicate for a Codex rollout.
pub(super) fn codex_filename_matches(filename: &str, id: &str) -> bool {
    if !filename.starts_with("rollout-") || !filename.ends_with(".jsonl") {
        return false;
    }
    let bytes = filename.as_bytes();
    let id = id.as_bytes();
    if id.is_empty() || id.len() > bytes.len() {
        return false;
    }
    for start in 0..=(bytes.len() - id.len()) {
        if &bytes[start..start + id.len()] != id {
            continue;
        }
        let before_ok = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
        let after = start + id.len();
        let after_ok = after == bytes.len() || !bytes[after].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
    }
    false
}

fn discover_codex(
    root: &safe_fs::Dir,
    home: &str,
    reference: &ValidReference,
    lookup: &mut Lookup,
) {
    lookup.note_entry();
    let sessions = match root.child(std::ffi::OsStr::new("sessions")) {
        Ok(safe_fs::Child::Dir(dir)) => dir,
        Ok(safe_fs::Child::Unsafe) => {
            lookup.unsafe_seen = true;
            return;
        }
        Ok(_) => return,
        Err(_) => {
            lookup.io_seen = true;
            return;
        }
    };
    walk_codex(
        &sessions,
        &join_path(home, "sessions"),
        0,
        reference,
        lookup,
    );
}

fn walk_codex(
    dir: &safe_fs::Dir,
    path: &str,
    depth: usize,
    reference: &ValidReference,
    lookup: &mut Lookup,
) {
    let Some(entries) = bounded_entries(dir, lookup) else {
        return;
    };
    let mut subdirs: Vec<(String, safe_fs::Dir)> = Vec::new();
    for name in entries {
        lookup.note_entry();
        let Some(name_str) = name.to_str() else {
            lookup.io_seen = true;
            continue;
        };
        // Open and classify the child before applying the rollout filename
        // predicate: a regular file may match, while an opened directory
        // remains traversable within depth six even when its own name
        // resembles `rollout-*.jsonl`.
        match dir.child(std::ffi::OsStr::new(name_str)) {
            Ok(safe_fs::Child::File(file)) => {
                if codex_filename_matches(name_str, &reference.locator) {
                    match file.identity() {
                        Ok(identity) => lookup.candidates.push(AdmittedSource {
                            path: join_path(path, name_str),
                            identity,
                            file,
                        }),
                        Err(_) => lookup.io_seen = true,
                    }
                }
            }
            Ok(safe_fs::Child::Dir(sub)) => {
                if depth < 6 {
                    subdirs.push((name_str.to_string(), sub));
                }
            }
            Ok(safe_fs::Child::Unsafe) => lookup.unsafe_seen = true,
            Ok(safe_fs::Child::Absent) => {}
            Err(_) => lookup.io_seen = true,
        }
    }
    for (name, sub) in subdirs {
        walk_codex(&sub, &join_path(path, &name), depth + 1, reference, lookup);
    }
}

/// The classification of a DSH candidate's bounded opening record. The
/// variants keep an invalid-depth session header distinct from an absent
/// or malformed opening row and from a header that outgrew the cap, so
/// discovery can resolve the exact specified outcome and explanation.
enum HeaderCheck {
    Valid,
    InvalidDepth,
    NotSessionHeader,
    TooLarge,
    Io,
}

/// Read at most the bounded first record from a DSH candidate and admit
/// it only as an opening `session` object with absent or unsigned-zero
/// depth. The version field neither qualifies nor vetoes ownership.
fn dsh_header(file: &safe_fs::OpenedFile) -> HeaderCheck {
    // The header budget is DSH_HEADER_CAP bytes; read one further byte so
    // a newline that immediately follows a header of exactly the cap is
    // seen instead of being discarded as the overflow probe.
    let (bytes, overflow, _eof) =
        match file.read_bounded(brokkr_view::transcript::DSH_HEADER_CAP as u64 + 1) {
            Ok(read) => read,
            Err(_) => return HeaderCheck::Io,
        };
    let header = match bytes.iter().position(|byte| *byte == b'\n') {
        Some(end) => &bytes[..end],
        None => {
            // No complete first record inside the cap: either the header
            // itself exceeds the bound, or the whole file was read.
            if overflow {
                return HeaderCheck::TooLarge;
            }
            &bytes[..]
        }
    };
    // The read's own overflow flag cannot bound a true-EOF header: a file
    // of exactly cap-plus-one bytes without a newline reads fully with no
    // probe byte left. The admitted header is the bytes before the first
    // newline, or the whole buffer when there is none, and that slice is
    // what the 65,536-byte budget measures. The delimiter is never an
    // admitted header byte.
    if header.len() > brokkr_view::transcript::DSH_HEADER_CAP {
        return HeaderCheck::TooLarge;
    }
    let Ok(text) = std::str::from_utf8(header) else {
        // Invalid UTF-8 prevents establishing a unique ownership answer.
        return HeaderCheck::Io;
    };
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return HeaderCheck::NotSessionHeader;
    };
    let Some(object) = value.as_object() else {
        return HeaderCheck::NotSessionHeader;
    };
    if object.get("type").and_then(Value::as_str) != Some("session") {
        return HeaderCheck::NotSessionHeader;
    }
    match object.get("delegationDepth") {
        // Omitted depth retains the legacy meaning of zero.
        None => {}
        Some(depth) if depth.as_u64() == Some(0) => {}
        // Null, booleans, strings, negatives and floats are invalid.
        Some(_) => return HeaderCheck::InvalidDepth,
    }
    HeaderCheck::Valid
}

/// Open one locator component from the previous directory, charging it to
/// the discovery budget and resolving the shared child arms. `None` ends
/// the lookup: an unsafe child, a non-directory child or an I/O failure.
fn dsh_step(parent: &safe_fs::Dir, component: &str, lookup: &mut Lookup) -> Option<safe_fs::Dir> {
    lookup.note_entry();
    child_dir(parent, component, lookup)
}

/// The child directory `name` of `parent`, resolving the shared child
/// arms: an unsafe child and an I/O failure are recorded, and `None` also
/// answers for a child that is not a directory.
fn child_dir(parent: &safe_fs::Dir, name: &str, lookup: &mut Lookup) -> Option<safe_fs::Dir> {
    match parent.child(std::ffi::OsStr::new(name)) {
        Ok(safe_fs::Child::Dir(dir)) => Some(dir),
        Ok(safe_fs::Child::Unsafe) => {
            lookup.unsafe_seen = true;
            None
        }
        Ok(_) => None,
        Err(_) => {
            lookup.io_seen = true;
            None
        }
    }
}

fn discover_dsh(root: &safe_fs::Dir, home: &str, reference: &ValidReference, lookup: &mut Lookup) {
    // The recorded locator is a relative component path. `split_once`
    // separates the first component, which opens from the root; each later
    // component opens from the previous step's directory. The base is
    // always the last directory a step opened, so nothing is left unset:
    // `split_once` yields at least one component for any string, and once
    // the loop starts it either returns or stores the next directory. The
    // removed post-walk `else` was unreachable for exactly that reason.
    let mut base_path = home.to_string();
    let (first, rest) = match reference.locator.split_once('/') {
        Some((first, rest)) => (first, Some(rest)),
        None => (reference.locator.as_str(), None),
    };
    let Some(mut base) = dsh_step(root, first, lookup) else {
        return;
    };
    base_path = join_path(&base_path, first);
    if let Some(rest) = rest {
        for component in rest.split('/') {
            let Some(next) = dsh_step(&base, component, lookup) else {
                return;
            };
            base_path = join_path(&base_path, component);
            base = next;
        }
    }
    let Some(projects) = bounded_entries(&base, lookup) else {
        return;
    };
    for project_name in projects {
        lookup.note_entry();
        let Some(project_name) = project_name.to_str() else {
            lookup.io_seen = true;
            continue;
        };
        let Some(project) = child_dir(&base, project_name, lookup) else {
            continue;
        };
        dsh_project(&project, &join_path(&base_path, project_name), lookup);
    }
}

/// The session directories of one DSH project, each searched for its one
/// candidate.
fn dsh_project(project: &safe_fs::Dir, project_path: &str, lookup: &mut Lookup) {
    let Some(sessions) = bounded_entries(project, lookup) else {
        return;
    };
    for session_name in sessions {
        lookup.note_entry();
        let Some(session_name) = session_name.to_str() else {
            lookup.io_seen = true;
            continue;
        };
        let Some(session) = child_dir(project, session_name, lookup) else {
            continue;
        };
        dsh_session(&session, &join_path(project_path, session_name), lookup);
    }
}

/// One DSH session directory's candidate. The admitted names, newest
/// first. A session directory yields at most one candidate: the first
/// admitted name carrying a valid header, so a directory holding both
/// admits the newer and never becomes ambiguous with itself. Only an
/// absent file or a row that is not a session header moves on to the next
/// name; every other outcome is decisive for this directory.
fn dsh_session(session: &safe_fs::Dir, session_path: &str, lookup: &mut Lookup) {
    for session_file in DSH_SESSION_FILES {
        let mut decided = true;
        match session.child(std::ffi::OsStr::new(session_file)) {
            Ok(safe_fs::Child::File(file)) => match dsh_header(&file) {
                HeaderCheck::Valid => match file.identity() {
                    Ok(identity) => lookup.candidates.push(AdmittedSource {
                        path: join_path(session_path, session_file),
                        identity,
                        file,
                    }),
                    Err(_) => lookup.io_seen = true,
                },
                HeaderCheck::InvalidDepth => lookup.invalid_depth_seen = true,
                HeaderCheck::NotSessionHeader => decided = false,
                HeaderCheck::TooLarge => lookup.limit_hit = true,
                HeaderCheck::Io => lookup.io_seen = true,
            },
            Ok(safe_fs::Child::Unsafe) => lookup.unsafe_seen = true,
            Ok(_) => decided = false,
            Err(_) => lookup.io_seen = true,
        }
        if decided {
            break;
        }
    }
}

/// The session filenames DSH discovery admits, newest first. The core
/// versions this file: 0.1.5-rc.1 writes `session.v3.jsonl` where earlier
/// cores wrote `session.jsonl`, and a reader that knows only one name loses
/// exactly the newest evidence. The set is closed rather than a glob so
/// that admitting a name stays a decision with evidence behind it, and
/// ordered so the choice between two present files is stated rather than
/// incidental.
const DSH_SESSION_FILES: [&str; 2] = ["session.v3.jsonl", "session.jsonl"];

/// The current size of the unique safely discovered source for a validated
/// reference, or `None` when discovery admits nothing. The size event
/// keeps its shipped `{"size": n}` shape; the size is measured through the
/// retained handle, never a reopened pathname.
pub(crate) fn source_size(reference: &ValidReference) -> Option<u64> {
    match discover(reference) {
        Discovery::Admitted(source) => Some(source.file.len()),
        Discovery::Refused(..) => None,
    }
}
