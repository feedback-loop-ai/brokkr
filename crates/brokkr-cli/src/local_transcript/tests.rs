//! The shared local reader's discovery and read-path proofs (D2-D4). The
//! browser's routes over the same reader are `ui`'s tests; the fault-seam
//! proofs are `fault_tests.rs`.

use super::discover::{acquisition_is_current, codex_filename_matches};
use super::*;
use brokkr_view::transcript::LegacyProvenance;

/// The subject of one recorded `reference` under `provenance`, with no
/// legacy id: what a test reads when it holds a reference and no journal.
pub(crate) fn reference_subject(
    reference: Option<&brokkr_view::Transcript>,
    provenance: LegacyProvenance,
) -> Subject {
    Subject {
        tab: 0,
        realm: None,
        run: "r1".to_string(),
        key: "seat".to_string(),
        reference: reference.cloned(),
        provenance,
        legacy_id: None,
        working: false,
    }
}

/// The traversal guard lives with the shared reference selection, not in
/// the HTTP layer: invalid ids refuse before any path is formed.
#[test]
fn the_session_lookup_carries_its_own_id_validation() {
    for bad in ["", "../../etc/passwd", &"a".repeat(65), "/etc/passwd"] {
        let reference = common("claude-session", bad, "/tmp/projects");
        assert!(
            read_common(&reference).unavailable.is_some(),
            "{bad:?} must refuse"
        );
    }
}

/// The liveness rule the watch keeps, now measured through the retained
/// handle rather than a reopened pathname: the first look is never
/// growth, an append is, and a shrunk or vanished source is not.
#[test]
fn source_growth_is_measured_through_the_retained_handle() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("t.jsonl"), "1234567890").unwrap();
    let root = safe_fs::Dir::open_root(dir.path().to_str().unwrap()).unwrap();
    let safe_fs::Child::File(file) = root.child(std::ffi::OsStr::new("t.jsonl")).unwrap() else {
        panic!("a regular file opens as a file");
    };
    assert_eq!(file.len(), 10, "the held handle reports the size");
    let grew = |previous: Option<u64>, current: u64| previous.is_some_and(|p| current > p);
    assert!(!grew(None, file.len()), "the first look is never growth");
    assert!(grew(Some(10), 11), "one more byte is new prose");
    assert!(!grew(Some(10), 10), "an unchanged file is quiet");
    assert!(!grew(Some(10), 3), "a shrink is not an append");
}

// ---------------------------------------------- shared discovery (D3/D4)

pub(super) fn common(kind: &str, locator: &str, home: &str) -> brokkr_view::Transcript {
    brokkr_view::Transcript {
        kind: kind.to_string(),
        locator: locator.to_string(),
        home: home.to_string(),
    }
}

/// Read a present common reference without touching the ambient HOME.
pub(super) fn read_common(reference: &brokkr_view::Transcript) -> TranscriptRead {
    read_local(
        &reference_subject(Some(reference), LegacyProvenance::Absent),
        None,
    )
}

/// Claude: the exact filename in one immediate project directory; a
/// duplicate is ambiguous, a symlink is unsafe, and no content is read.
#[cfg(unix)]
#[test]
fn claude_discovery_is_scoped_and_refuses_symlinks() {
    let dir = tempfile::tempdir().unwrap();
    let projects = dir.path().join("projects");
    std::fs::create_dir_all(projects.join("one")).unwrap();
    let reference = common("claude-session", "abcd-1234", projects.to_str().unwrap());
    let body =
        "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"hello\"}}\n";

    // No file yet.
    assert_eq!(
        read_common(&reference).unavailable,
        Some(Unavailable::NotFound)
    );

    std::fs::write(projects.join("one/abcd-1234.jsonl"), body).unwrap();
    let read = read_common(&reference);
    assert!(read.is_readable(), "{read:?}");
    assert_eq!(read.turns.len(), 1);

    // A symlink candidate is unsafe and supplies no content; the one safe
    // regular file still wins.
    std::fs::create_dir_all(projects.join("two")).unwrap();
    std::os::unix::fs::symlink(
        projects.join("one/abcd-1234.jsonl"),
        projects.join("two/abcd-1234.jsonl"),
    )
    .unwrap();
    let read = read_common(&reference);
    assert!(read.is_readable(), "the safe regular file wins: {read:?}");
    assert_eq!(
        read.path.as_deref().unwrap(),
        projects
            .join("one/abcd-1234.jsonl")
            .canonicalize()
            .unwrap()
            .to_str()
            .unwrap()
    );

    // A symlink-only lookup that also hides the real file is unsafe-path.
    std::fs::remove_file(projects.join("one/abcd-1234.jsonl")).unwrap();
    assert_eq!(
        read_common(&reference).unavailable,
        Some(Unavailable::UnsafePath)
    );

    // A real file under a symlinked project directory is not an unsafe
    // candidate; the real project still supplies content.
    std::fs::write(projects.join("one/abcd-1234.jsonl"), body).unwrap();
    std::os::unix::fs::symlink(projects.join("one"), projects.join("three")).unwrap();
    let read = read_common(&reference);
    assert!(read.is_readable(), "the real project still wins: {read:?}");
}

/// Codex: the whole-filename token predicate, through depth, with no
/// header gate and no id-language beyond the engine's own.
#[test]
fn codex_discovery_matches_the_whole_token() {
    let dir = tempfile::tempdir().unwrap();
    let sessions = dir.path().join("sessions");
    std::fs::create_dir_all(sessions.join("2026/09/10")).unwrap();
    let reference = common("codex-thread", "0199mine", dir.path().to_str().unwrap());

    // `rollout-0199other` only.
    std::fs::write(
        sessions.join("2026/09/10/rollout-0199other.jsonl"),
        "{\"type\":\"turn_context\"}\n",
    )
    .unwrap();
    assert_eq!(
        read_common(&reference).unavailable,
        Some(Unavailable::NotFound)
    );

    // Add the whole-token match, then a superset token that must not match.
    std::fs::write(
        sessions.join("rollout-0199mine.jsonl"),
        "{\"type\":\"turn_context\"}\n",
    )
    .unwrap();
    let read = read_common(&reference);
    assert!(read.is_readable(), "{read:?}");
    assert!(read
        .path
        .as_deref()
        .unwrap()
        .ends_with("rollout-0199mine.jsonl"));

    std::fs::write(
        sessions.join("2026/09/10/rollout-0199mineX.jsonl"),
        "{\"type\":\"turn_context\"}\n",
    )
    .unwrap();
    let read = read_common(&reference);
    assert!(
        read.is_readable(),
        "the superset token is not a match: {read:?}"
    );
}

/// A directory whose own name resembles a rollout is still traversed, so
/// an eligible descendant beneath it is discovered (design D3).
#[test]
fn codex_rollout_shaped_directories_are_still_traversed() {
    let dir = tempfile::tempdir().unwrap();
    let sessions = dir.path().join("sessions");
    let shaped = sessions.join("rollout-0199mine.jsonl");
    std::fs::create_dir_all(&shaped).unwrap();
    std::fs::write(
        shaped.join("rollout-0199mine.jsonl"),
        "{\"type\":\"turn_context\"}\n",
    )
    .unwrap();
    let reference = common("codex-thread", "0199mine", dir.path().to_str().unwrap());
    let read = read_common(&reference);
    assert!(
        read.is_readable(),
        "the rollout-shaped directory is traversed: {read:?}"
    );
    assert!(
        read.path
            .as_deref()
            .unwrap()
            .ends_with("rollout-0199mine.jsonl/rollout-0199mine.jsonl"),
        "the descendant is the confirmed source: {read:?}"
    );
}

/// A home spelled in another platform's absolute syntax is refused as an
/// invalid reference before any filesystem call, and its bytes are echoed.
#[cfg(unix)]
#[test]
fn a_foreign_absolute_home_is_refused_before_native_io() {
    let recorded = r"C:\Users\operator\.claude\projects";
    let reference = common("claude-session", "abcd-1234", recorded);
    let read = read_common(&reference);
    assert_eq!(read.unavailable, Some(Unavailable::InvalidReference));
    assert_eq!(read.path, None, "no path is confirmed for a refused home");
    assert_eq!(
        read.reference.as_ref().unwrap().home,
        recorded,
        "the recorded bytes are echoed unchanged"
    );
}

/// DSH: the recorded root's project/session layout, depth zero only, and
/// version cannot choose between two owned roots.
#[test]
fn dsh_discovery_admits_depth_zero_and_refuses_delegated_siblings() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("dsh");
    let locator = "sessions/brokkr/seat-222";
    let make = |project: &str, session: &str, header: &str| {
        let dir = root.join(locator).join(project).join(session);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("session.jsonl"), format!("{header}\n")).unwrap();
    };
    let reference = common("dsh-session", locator, root.to_str().unwrap());

    // A delegated sibling at depth one is not the depth-zero owned session.
    make(
        "project",
        "delegated",
        "{\"type\":\"session\",\"delegationDepth\":1,\"version\":0}",
    );
    let read = read_common(&reference);
    assert_eq!(read.unavailable, Some(Unavailable::NotFound));
    assert!(read.explanation.as_deref().unwrap().contains("depth-zero"));

    // A string depth is not unsigned zero.
    make(
        "project",
        "stringy",
        "{\"type\":\"session\",\"delegationDepth\":\"zero\",\"version\":0}",
    );
    assert_eq!(
        read_common(&reference).unavailable,
        Some(Unavailable::NotFound)
    );

    // A valid depth-zero root.
    make(
        "root",
        "seat",
        "{\"type\":\"session\",\"delegationDepth\":0,\"version\":0}",
    );
    let read = read_common(&reference);
    assert!(read.is_readable(), "{read:?}");

    // Version never changes ownership: two owned roots are ambiguous.
    make(
        "root",
        "foreign",
        "{\"type\":\"session\",\"delegationDepth\":0,\"version\":1}",
    );
    assert_eq!(
        read_common(&reference).unavailable,
        Some(Unavailable::AmbiguousSource)
    );
}

/// A DSH opening row that is malformed cannot borrow a later header, an
/// oversized first record is a discovery-limit (not a missing session),
/// and invalid opening-header UTF-8 is unreadable.
#[test]
fn dsh_discovery_bounds_the_opening_header() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("dsh");
    let locator = "sessions/one";
    let session = root.join(locator).join("project").join("seat");
    std::fs::create_dir_all(&session).unwrap();
    let reference = common("dsh-session", locator, root.to_str().unwrap());

    std::fs::write(
        session.join("session.jsonl"),
        "not json\n{\"type\":\"session\",\"delegationDepth\":0,\"version\":0}\n",
    )
    .unwrap();
    assert_eq!(
        read_common(&reference).unavailable,
        Some(Unavailable::NotFound)
    );

    // A non-session first record is not an invalid-depth explanation.
    std::fs::write(
        session.join("session.jsonl"),
        "{\"type\":\"entry\",\"version\":0}\n",
    )
    .unwrap();
    let read = read_common(&reference);
    assert_eq!(read.unavailable, Some(Unavailable::NotFound));
    assert!(
        !read
            .explanation
            .as_deref()
            .unwrap_or_default()
            .contains("depth-zero"),
        "a non-session opening row must not borrow the invalid-depth explanation: {read:?}"
    );

    // A null depth is invalid, not legacy zero: it is excluded from
    // ownership and keeps the invalid-depth explanation.
    std::fs::write(
        session.join("session.jsonl"),
        "{\"type\":\"session\",\"delegationDepth\":null,\"version\":0}\n",
    )
    .unwrap();
    let read = read_common(&reference);
    assert_eq!(read.unavailable, Some(Unavailable::NotFound));
    assert!(
        read.explanation
            .as_deref()
            .unwrap_or_default()
            .contains("depth-zero"),
        "a null depth keeps the invalid-depth explanation: {read:?}"
    );
    assert!(read.path.is_none());

    std::fs::write(
        session.join("session.jsonl"),
        format!("{}\n", "x".repeat(70_000)),
    )
    .unwrap();
    assert_eq!(
        read_common(&reference).unavailable,
        Some(Unavailable::DiscoveryLimit)
    );

    std::fs::write(
        session.join("session.jsonl"),
        [
            b"{\"type\":\"session\",\"delegationDepth\":0,\"version\":0,\"pad\":\"".as_slice(),
            &[0xff, 0xfe],
            b"\"}\n",
        ]
        .concat(),
    )
    .unwrap();
    assert_eq!(
        read_common(&reference).unavailable,
        Some(Unavailable::Unreadable)
    );
}

/// A header-only DSH file at EOF without a newline is still complete.
#[test]
fn dsh_discovery_accepts_a_header_at_eof_without_newline() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("dsh");
    let locator = "sessions/one";
    let session = root.join(locator).join("project").join("seat");
    std::fs::create_dir_all(&session).unwrap();
    std::fs::write(
        session.join("session.jsonl"),
        "{\"type\":\"session\",\"delegationDepth\":0,\"version\":0}",
    )
    .unwrap();
    let reference = common("dsh-session", locator, root.to_str().unwrap());
    let read = read_common(&reference);
    assert!(read.is_readable(), "{read:?}");
    assert!(read.turns.is_empty());
}

/// L5: a newline immediately after a header of exactly the bounded
/// header cap is part of the header, not the overflow probe.
#[test]
fn dsh_discovery_admits_a_header_exactly_at_the_cap() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("dsh");
    let locator = "sessions/one";
    let session = root.join(locator).join("project").join("seat");
    std::fs::create_dir_all(&session).unwrap();
    let reference = common("dsh-session", locator, root.to_str().unwrap());

    let base = r#"{"type":"session","delegationDepth":0,"version":0"#;
    let cap = brokkr_view::transcript::DSH_HEADER_CAP;
    let pad = cap - base.len() - 1;
    let header = format!("{base}{}}}", " ".repeat(pad));
    assert_eq!(header.len(), cap);
    std::fs::write(session.join("session.jsonl"), format!("{header}\n")).unwrap();
    let read = read_common(&reference);
    assert!(
        read.is_readable(),
        "a cap-length header before a newline is admitted: {read:?}"
    );
    assert!(read.turns.is_empty());

    // One byte past the cap is still beyond the bounded header.
    let header = format!("{base}{}}}", " ".repeat(pad + 1));
    assert_eq!(header.len(), cap + 1);
    std::fs::write(session.join("session.jsonl"), format!("{header}\n")).unwrap();
    assert_eq!(
        read_common(&reference).unavailable,
        Some(Unavailable::DiscoveryLimit)
    );
}

/// The 65,536-byte budget measures the admitted header slice, not the
/// read's separate overflow probe: a true-EOF header of cap-plus-one
/// bytes with no newline must refuse with `discovery-limit` even though
/// the file holds no byte beyond the read, and the exact cap must be
/// admitted, under both admitted DSH versions.
#[test]
fn dsh_discovery_bounds_the_header_at_true_eof() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("dsh");
    let locator = "sessions/one";
    let session = root.join(locator).join("project").join("seat");
    std::fs::create_dir_all(&session).unwrap();
    let reference = common("dsh-session", locator, root.to_str().unwrap());
    let cap = brokkr_view::transcript::DSH_HEADER_CAP;

    for version in ["0", "3"] {
        let base = format!("{{\"type\":\"session\",\"delegationDepth\":0,\"version\":{version}");
        let pad = cap - base.len() - 1;

        // Exactly the cap at true EOF, with no delimiter: admitted.
        let exact = format!("{base}{}}}", " ".repeat(pad));
        assert_eq!(exact.len(), cap);
        std::fs::write(session.join("session.jsonl"), &exact).unwrap();
        let read = read_common(&reference);
        assert!(
            read.is_readable(),
            "version {version}: a cap-length EOF header is admitted: {read:?}"
        );
        assert!(read.turns.is_empty());

        // One byte past the cap at true EOF, with no delimiter: the
        // length predicate alone must refuse, since there is no overflow
        // byte and the bytes parse as valid JSON.
        let over = format!("{base}{}}}", " ".repeat(pad + 1));
        assert_eq!(over.len(), cap + 1);
        std::fs::write(session.join("session.jsonl"), &over).unwrap();
        let read = read_common(&reference);
        assert_eq!(
            read.unavailable,
            Some(Unavailable::DiscoveryLimit),
            "version {version}: cap-plus-one at EOF refuses on the length: {read:?}"
        );
        assert!(read.path.is_none(), "version {version}: no confirmed path");
        assert!(read.turns.is_empty(), "version {version}: no turns");
        assert_eq!(read.skipped_lines, 0, "version {version}");
        assert_eq!(read.unrecognized_records, 0, "version {version}");
        assert!(!read.truncated, "version {version}: not body truncation");
        assert!(
            read.notices.is_empty(),
            "version {version}: discovery refusal adds no notice"
        );
        assert!(
            read.full_session.is_none(),
            "version {version}: a discovery refusal keeps the null hint"
        );
    }
}

/// Every file below `root`, relative path -> bytes, with symlinks
/// recorded as their link text. Used to prove a read changes no
/// retained byte (7.6).
#[cfg(unix)]
fn snapshot_tree(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    use std::os::unix::ffi::OsStringExt;
    fn walk(base: &Path, dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap())
            .collect();
        entries.sort_by_key(|entry| entry.path());
        for entry in entries {
            let path = entry.path();
            let meta = std::fs::symlink_metadata(&path).unwrap();
            let key = path.strip_prefix(base).unwrap().to_path_buf();
            if meta.file_type().is_symlink() {
                out.push((
                    key,
                    std::fs::read_link(&path)
                        .unwrap()
                        .into_os_string()
                        .into_vec(),
                ));
            } else if meta.is_dir() {
                walk(base, &path, out);
            } else if meta.is_file() {
                out.push((key, std::fs::read(&path).unwrap()));
            } else {
                // A FIFO or device is recorded by type, never opened.
                out.push((key, b"<non-regular>".to_vec()));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out
}

#[cfg(unix)]
fn assert_retained(root: &Path, before: &[(PathBuf, Vec<u8>)]) {
    assert_eq!(
        snapshot_tree(root),
        before,
        "the read must not change retained bytes"
    );
}

/// 7.6 — each kind searches only its own declared scope, and a read
/// leaves the retained tree byte-for-byte unchanged.
#[cfg(unix)]
#[test]
fn kind_scopes_are_closed_and_reads_retain_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir_all(&home).unwrap();

    // Claude: only an immediate project directory, never a nested one.
    let projects = home.join(".claude/projects");
    std::fs::create_dir_all(projects.join("one/nested")).unwrap();
    let claude = common("claude-session", "abcd-1234", projects.to_str().unwrap());
    std::fs::write(
        projects.join("one/nested/abcd-1234.jsonl"),
        "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"deep\"}}\n",
    )
    .unwrap();
    let before = snapshot_tree(&home);
    assert_eq!(
        read_common(&claude).unavailable,
        Some(Unavailable::NotFound),
        "a nested project directory is outside the immediate scope"
    );
    assert_retained(&home, &before);
    std::fs::write(
        projects.join("one/abcd-1234.jsonl"),
        "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"hi\"}}\n",
    )
    .unwrap();
    let before = snapshot_tree(&home);
    assert!(read_common(&claude).is_readable());
    assert_retained(&home, &before);

    // Codex: only below `sessions`, never beside the recorded home.
    let codex = common("codex-thread", "0199mine", home.to_str().unwrap());
    std::fs::write(home.join("rollout-0199mine.jsonl"), "{}\n").unwrap();
    let before = snapshot_tree(&home);
    assert_eq!(
        read_common(&codex).unavailable,
        Some(Unavailable::NotFound),
        "a rollout beside the home is outside `<home>/sessions`"
    );
    assert_retained(&home, &before);
    std::fs::create_dir_all(home.join("sessions")).unwrap();
    std::fs::write(home.join("sessions/rollout-0199mine.jsonl"), "{}\n").unwrap();
    let before = snapshot_tree(&home);
    assert!(read_common(&codex).is_readable());
    assert_retained(&home, &before);

    // DSH: only below `<home>/<locator>`.
    let dsh = common(
        "dsh-session",
        "sessions/brokkr/seat-222",
        home.to_str().unwrap(),
    );
    let sibling = home.join("sessions/brokkr/other/project/seat");
    std::fs::create_dir_all(&sibling).unwrap();
    std::fs::write(
        sibling.join("session.jsonl"),
        "{\"type\":\"session\",\"delegationDepth\":0}\n",
    )
    .unwrap();
    let before = snapshot_tree(&home);
    assert_eq!(
        read_common(&dsh).unavailable,
        Some(Unavailable::NotFound),
        "a sibling locator is outside the recorded seat root"
    );
    assert_retained(&home, &before);
    let owned = home.join("sessions/brokkr/seat-222/project/root");
    std::fs::create_dir_all(&owned).unwrap();
    std::fs::write(
        owned.join("session.jsonl"),
        "{\"type\":\"session\",\"delegationDepth\":0,\"version\":0}\n",
    )
    .unwrap();
    let before = snapshot_tree(&home);
    assert!(read_common(&dsh).is_readable());
    assert_retained(&home, &before);
}

/// 7.6 — the whole-token Codex predicate over the variant filenames and
/// the 80/81-character length boundary, with the recorded home winning
/// over a different ambient one.
#[test]
fn codex_whole_token_variants_and_length_boundary() {
    let dir = tempfile::tempdir().unwrap();
    let recorded = dir.path().join("recorded");
    let ambient = dir.path().join("ambient");
    std::fs::create_dir_all(recorded.join("sessions")).unwrap();
    std::fs::create_dir_all(ambient.join("sessions")).unwrap();
    let reference = common("codex-thread", "0199mine", recorded.to_str().unwrap());

    for name in ["rollout-0199other.jsonl", "rollout-0199mineX.jsonl"] {
        std::fs::write(recorded.join("sessions").join(name), "{}\n").unwrap();
    }
    assert_eq!(
        read_common(&reference).unavailable,
        Some(Unavailable::NotFound),
        "only `rollout-0199mine` is a whole-token match"
    );
    std::fs::write(
        recorded.join("sessions/rollout-0199mine.jsonl"),
        "{\"type\":\"turn_context\"}\n",
    )
    .unwrap();
    let read = read_common(&reference);
    assert!(read.is_readable(), "{read:?}");
    assert!(read
        .path
        .as_deref()
        .unwrap()
        .ends_with("rollout-0199mine.jsonl"));

    // The recorded home, not an ambient one, decides the lookup.
    let ambient_only = common("codex-thread", "0199mine", ambient.to_str().unwrap());
    std::fs::write(
        ambient.join("sessions/rollout-0199mine.jsonl"),
        "{\"type\":\"turn_context\"}\n",
    )
    .unwrap();
    let ambient_read = read_local(
        &reference_subject(Some(&reference), LegacyProvenance::Absent),
        Some(ambient.to_str().unwrap()),
    );
    assert!(ambient_read
        .path
        .as_deref()
        .unwrap()
        .starts_with(recorded.canonicalize().unwrap().to_str().unwrap()));
    assert_eq!(
        read_local(
            &reference_subject(Some(&ambient_only), LegacyProvenance::Absent),
            Some(recorded.to_str().unwrap()),
        )
        .path
        .as_deref()
        .unwrap(),
        format!(
            "{}/sessions/rollout-0199mine.jsonl",
            ambient.canonicalize().unwrap().display()
        )
    );

    // An 80-character id never matches an 81-character filename token.
    let id80 = "a".repeat(80);
    let boundary = common("codex-thread", &id80, recorded.to_str().unwrap());
    std::fs::write(
        recorded
            .join("sessions")
            .join(format!("rollout-{id80}a.jsonl")),
        "{}\n",
    )
    .unwrap();
    assert_eq!(
        read_common(&boundary).unavailable,
        Some(Unavailable::NotFound)
    );
    std::fs::write(
        recorded
            .join("sessions")
            .join(format!("rollout-{id80}.jsonl")),
        "{\"type\":\"turn_context\"}\n",
    )
    .unwrap();
    assert!(read_common(&boundary).is_readable());
}

/// 7.6 — DSH depth ownership beyond version, and ambiguity resolved the
/// same way in either creation order.
#[test]
fn dsh_depth_versions_and_both_enumeration_orders() {
    let build = |order: [(&str, &str); 2]| {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("dsh");
        let locator = "sessions/brokkr/seat-222";
        let reference = common("dsh-session", locator, root.to_str().unwrap());
        for (project, version) in order {
            let session = root.join(locator).join(project).join("seat");
            std::fs::create_dir_all(&session).unwrap();
            std::fs::write(
                session.join("session.jsonl"),
                format!("{{\"type\":\"session\",\"delegationDepth\":0,\"version\":{version}}}\n"),
            )
            .unwrap();
        }
        let read = read_common(&reference);
        assert_eq!(read.unavailable, Some(Unavailable::AmbiguousSource));
        (dir, root)
    };
    let (dir, _) = build([("zero-first", "0"), ("one-second", "1")]);
    drop(dir);
    let (dir, _) = build([("one-first", "1"), ("zero-second", "0")]);
    drop(dir);

    // A version-zero root alone is eligible; a version-one root with a
    // delegated or mistyped-depth sibling does not outrank it.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("dsh");
    let locator = "sessions/brokkr/seat-222";
    let reference = common("dsh-session", locator, root.to_str().unwrap());
    let write = |project: &str, header: &str| {
        let session = root.join(locator).join(project).join("seat");
        std::fs::create_dir_all(&session).unwrap();
        std::fs::write(session.join("session.jsonl"), format!("{header}\n")).unwrap();
    };
    write(
        "root",
        "{\"type\":\"session\",\"delegationDepth\":0,\"version\":0}",
    );
    write(
        "delegated",
        "{\"type\":\"session\",\"delegationDepth\":1,\"version\":1}",
    );
    write(
        "stringy",
        "{\"type\":\"session\",\"delegationDepth\":\"zero\",\"version\":1}",
    );
    let read = read_common(&reference);
    assert!(
        read.is_readable(),
        "the version-zero root remains unique: {read:?}"
    );
    assert!(read.path.as_deref().unwrap().contains("/root/"));
}

/// 7.6 — a spent discovery bound outranks a provisional match, and an
/// oversized DSH opening record is refused without allocating it.
#[test]
fn discovery_limit_outranks_a_provisional_match() {
    let dir = tempfile::tempdir().unwrap();
    let projects = dir.path().join("projects");
    std::fs::create_dir_all(projects.join("winning")).unwrap();
    std::fs::write(
        projects.join("winning/abcd-1234.jsonl"),
        "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"late\"}}\n",
    )
    .unwrap();
    for index in 0..10_001 {
        std::fs::write(projects.join(format!("filler-{index:05}")), b"x").unwrap();
    }
    let reference = common("claude-session", "abcd-1234", projects.to_str().unwrap());
    assert_eq!(
        read_common(&reference).unavailable,
        Some(Unavailable::DiscoveryLimit),
        "the bound outranks the one provisional candidate"
    );
}

/// 7.6 — symlinks and a FIFO supply no content, a non-Unicode path is
/// unreadable rather than lossily spelled, and every refusal retains bytes.
#[cfg(unix)]
#[test]
fn symlinks_fifos_and_non_unicode_paths_are_unavailable() {
    use std::os::unix::ffi::OsStringExt;

    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root");
    std::fs::create_dir_all(root.join("one")).unwrap();
    let reference = common("claude-session", "abcd-1234", root.to_str().unwrap());
    let body =
        "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"real\"}}\n";
    std::fs::write(root.join("one/abcd-1234.jsonl"), body).unwrap();

    // A symlink inside the recognised scope, pointing within the root.
    std::fs::create_dir_all(root.join("two")).unwrap();
    std::os::unix::fs::symlink(
        root.join("one/abcd-1234.jsonl"),
        root.join("two/abcd-1234.jsonl"),
    )
    .unwrap();
    let before = snapshot_tree(&root);
    let read = read_common(&reference);
    assert!(read.is_readable(), "the safe regular file still wins");
    assert!(read.path.as_deref().unwrap().contains("/one/"));
    assert_retained(&root, &before);

    // With only the symlink left, the lookup is unsafe and supplies no
    // content, and a symlink escaping the root is the same refusal.
    let outside = dir.path().join("outside.jsonl");
    std::fs::write(&outside, body).unwrap();
    std::fs::remove_file(root.join("one/abcd-1234.jsonl")).unwrap();
    std::fs::remove_file(root.join("two/abcd-1234.jsonl")).unwrap();
    std::os::unix::fs::symlink(&outside, root.join("one/abcd-1234.jsonl")).unwrap();
    let before = snapshot_tree(&root);
    let read = read_common(&reference);
    assert_eq!(read.unavailable, Some(Unavailable::UnsafePath), "{read:?}");
    assert_retained(&root, &before);

    // A FIFO is not a transcript file and cannot block the read.
    std::fs::remove_file(root.join("one/abcd-1234.jsonl")).unwrap();
    let fifo = std::ffi::CString::new(root.join("one/abcd-1234.jsonl").to_str().unwrap()).unwrap();
    let fifo_rc = unsafe { libc_mkfifo(fifo.as_ptr(), 0o644) };
    assert_eq!(fifo_rc, 0, "the test can create a FIFO");
    let before = snapshot_tree(&root);
    let read = read_common(&reference);
    assert!(
        read.unavailable.is_some() && !read.is_readable(),
        "a FIFO supplies no transcript content: {read:?}"
    );
    assert_retained(&root, &before);

    // A non-Unicode directory name makes uniqueness unknowable.
    std::fs::remove_file(root.join("one/abcd-1234.jsonl")).unwrap();
    let bad = root.join(std::ffi::OsString::from_vec(vec![b'b', 0xff, b'd']));
    if !create_non_unicode_dir(&bad) {
        return;
    }
    std::fs::write(bad.join("abcd-1234.jsonl"), body).unwrap();
    let before = snapshot_tree(&root);
    assert_eq!(
        read_common(&reference).unavailable,
        Some(Unavailable::Unreadable)
    );
    assert_retained(&root, &before);
}

// Read a FIFO creation through libc without adding a crate: `libc` is
// already an indirect dependency, so the symbol is declared here.
#[cfg(unix)]
unsafe extern "C" {
    #[link_name = "mkfifo"]
    fn libc_mkfifo(path: *const std::os::raw::c_char, mode: u32) -> std::os::raw::c_int;
}

/// 7.6 — a held handle keeps the verified bytes when the leaf or an
/// ancestor path is replaced between discovery and read.
#[cfg(unix)]
#[test]
fn held_handles_survive_ancestor_and_leaf_replacement() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root");
    let project = root.join("project");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(project.join("session.jsonl"), "original\n").unwrap();

    let root_handle = safe_fs::Dir::open_root(root.to_str().unwrap()).unwrap();
    let project_handle = match root_handle.child(std::ffi::OsStr::new("project")).unwrap() {
        safe_fs::Child::Dir(dir) => dir,
        _ => panic!("the project opens as a directory"),
    };
    let held = match project_handle
        .child(std::ffi::OsStr::new("session.jsonl"))
        .unwrap()
    {
        safe_fs::Child::File(file) => file,
        _ => panic!("the transcript opens as a regular file"),
    };

    // Replace the leaf path: the held file handle keeps the original
    // inode and bytes.
    std::fs::remove_file(project.join("session.jsonl")).unwrap();
    std::fs::write(project.join("session.jsonl"), "replacement\n").unwrap();
    let (bytes, _, _) = held.read_bounded(1024).unwrap();
    assert_eq!(std::str::from_utf8(&bytes).unwrap(), "original\n");

    // Replace the ancestor path: the held directory handle still reaches
    // the original directory, now renamed, and never the impostor.
    std::fs::rename(&project, root.join("moved")).unwrap();
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(project.join("session.jsonl"), "impostor\n").unwrap();
    match project_handle
        .child(std::ffi::OsStr::new("session.jsonl"))
        .unwrap()
    {
        safe_fs::Child::File(after) => {
            let (bytes, _, _) = after.read_bounded(1024).unwrap();
            assert_eq!(std::str::from_utf8(&bytes).unwrap(), "replacement\n");
        }
        _ => panic!("the held directory still opens the retained file"),
    }
}

/// M7: the read boundary's acquisition recheck is not the held leaf
/// compared against itself. A leaf or ancestor replaced after discovery
/// is a different candidate, so the acquisition is refused and the read
/// fails closed instead of serving the old inode.
#[cfg(unix)]
#[test]
fn a_replaced_leaf_or_ancestor_fails_the_acquisition_recheck() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root");
    let project = root.join("project");
    std::fs::create_dir_all(&project).unwrap();
    let file = project.join("abcd-1234.jsonl");
    let original =
        "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"original\"}}\n";
    std::fs::write(&file, original).unwrap();
    let valid = brokkr_view::transcript::ValidReference {
        kind: brokkr_view::transcript::TranscriptKind::ClaudeSession,
        locator: "abcd-1234".to_string(),
        home: root.to_str().unwrap().to_string(),
    };
    let source = match discover(&valid) {
        Discovery::Admitted(source) => source,
        _ => panic!("the original is discoverable"),
    };
    assert!(
        acquisition_is_current(&valid, &source),
        "the unchanged acquisition is current"
    );

    // Replace the leaf with an identical copy: a new inode is a new
    // acquisition even though every displayed byte matches. The copy is
    // staged outside the root and renamed over the target, so the new
    // inode is allocated while the old one still exists; remove-then-write
    // can reuse the freed inode on filesystems that do so.
    let replacement = dir.path().join("replacement.jsonl");
    std::fs::write(&replacement, original).unwrap();
    std::fs::rename(&replacement, &file).unwrap();
    assert!(
        !acquisition_is_current(&valid, &source),
        "a replaced leaf inode is not the retained acquisition"
    );

    // Replace the ancestor: the held root no longer reaches the candidate
    // the recorded path now names.
    std::fs::rename(&project, root.join("moved")).unwrap();
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(&file, original).unwrap();
    assert!(
        !acquisition_is_current(&valid, &source),
        "a replaced ancestor is not the retained acquisition"
    );
}

/// A symlinked recorded home is canonicalized once and opened as the
/// traversal root; the confirmed path is the canonical spelling.
#[cfg(unix)]
#[test]
fn a_symlinked_home_is_canonicalized_and_readable() {
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("real");
    let projects = real.join("project");
    std::fs::create_dir_all(&projects).unwrap();
    std::fs::write(
        projects.join("abcd-1234.jsonl"),
        "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"hi\"}}\n",
    )
    .unwrap();
    let link = dir.path().join("link");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let reference = common("claude-session", "abcd-1234", link.to_str().unwrap());
    let read = read_common(&reference);
    assert!(
        read.is_readable(),
        "a symlinked home is canonicalized, not refused: {read:?}"
    );
    assert!(
        read.path
            .as_deref()
            .unwrap()
            .starts_with(real.canonicalize().unwrap().to_str().unwrap()),
        "the confirmed path is canonical: {read:?}"
    );
}

/// Each enumerated project costs one discovery entry, not two: 6,000
/// projects with only one matching file stay inside the 10,000-entry
/// bound instead of exhausting it on the file probes.
#[test]
fn claude_discovery_counts_each_project_once() {
    let dir = tempfile::tempdir().unwrap();
    let projects = dir.path().join("projects");
    std::fs::create_dir_all(projects.join("winning")).unwrap();
    std::fs::write(
        projects.join("winning/abcd-1234.jsonl"),
        "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"late\"}}\n",
    )
    .unwrap();
    for index in 0..6_000 {
        std::fs::create_dir_all(projects.join(format!("filler-{index:05}"))).unwrap();
    }
    let reference = common("claude-session", "abcd-1234", projects.to_str().unwrap());
    let read = read_common(&reference);
    assert!(
        read.is_readable(),
        "6,000 projects are 6,000 examined entries, not 12,000: {read:?}"
    );
}

// ------------------------------------- discovery boundaries and refusals

#[test]
fn discovery_helper_predicates_cover_their_boundaries() {
    assert!(!codex_filename_matches("session.jsonl", "abc"));
    assert!(!codex_filename_matches("rollout-abc.txt", "abc"));
    assert!(codex_filename_matches("rollout-abc.jsonl", "abc"));
    assert!(!codex_filename_matches("rollout-.jsonl", ""));
    assert!(!codex_filename_matches("rollout-abc.jsonl", "abcd"));
    assert!(!codex_filename_matches("rolloutxabc.jsonl", "abc"));
    assert!(!codex_filename_matches("rollout-abcX.jsonl", "abc"));
    assert!(codex_filename_matches("rollout-abc-.jsonl", "abc"));
}

#[test]
fn discover_of_the_none_kind_selects_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let valid = brokkr_view::transcript::ValidReference {
        kind: brokkr_view::transcript::TranscriptKind::None,
        locator: "x".to_string(),
        home: dir.path().to_str().unwrap().to_string(),
    };
    assert!(matches!(discover(&valid), Discovery::Refused(_, _)));
}

#[test]
fn a_boundary_refusal_reports_the_recorded_path_and_hint() {
    let selection = brokkr_view::transcript::Selection {
        reference: Some(common("claude-session", "abcd-1234", "/h")),
        legacy: false,
        outcome: Err(brokkr_view::transcript::Unavailable::Unreadable),
    };
    let read = refused_source(
        &selection,
        "/h/one/abcd-1234.jsonl".to_string(),
        Some("hint".to_string()),
    );
    assert_eq!(read.unavailable, Some(Unavailable::Unreadable));
    assert_eq!(read.path.as_deref(), Some("/h/one/abcd-1234.jsonl"));
    assert_eq!(read.full_session.as_deref(), Some("hint"));
}

/// Create a fixture directory whose name is not valid UTF-8, reporting
/// whether the filesystem accepted it. Linux takes any byte sequence but
/// slash and null; macOS requires valid UTF-8 in a filename and refuses this
/// one with `EILSEQ` before the directory can exist. A reader cannot
/// encounter an entry the filesystem cannot represent, so a test that needs
/// one has nothing left to prove there and returns. Every other error is a
/// real failure and still fails.
#[cfg(unix)]
fn create_non_unicode_dir(path: &std::path::Path) -> bool {
    match std::fs::create_dir_all(path) {
        Ok(()) => true,
        Err(error) => {
            assert!(
                cfg!(target_vendor = "apple") && error.raw_os_error() == Some(92),
                "unexpected failure creating the non-Unicode fixture: {error}"
            );
            false
        }
    }
}

#[test]
fn a_non_directory_home_is_unreadable_not_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("not-a-dir");
    std::fs::write(&file, "x").unwrap();
    let reference = common("claude-session", "abcd-1234", file.to_str().unwrap());
    assert_eq!(
        read_common(&reference).unavailable,
        Some(Unavailable::Unreadable)
    );
}

#[cfg(unix)]
#[test]
fn a_non_unicode_canonical_home_is_unreadable() {
    use std::os::unix::ffi::OsStringExt;

    let dir = tempfile::tempdir().unwrap();
    let bad = dir
        .path()
        .join(std::ffi::OsString::from_vec(vec![b'b', 0xff]));
    if !create_non_unicode_dir(&bad) {
        return;
    }
    let link = dir.path().join("home");
    std::os::unix::fs::symlink(&bad, &link).unwrap();
    let reference = common("claude-session", "abcd-1234", link.to_str().unwrap());
    assert_eq!(
        read_common(&reference).unavailable,
        Some(Unavailable::Unreadable)
    );
}

/// The DSH core versions its session file: 0.1.5-rc.1 writes
/// `session.v3.jsonl` where earlier cores wrote `session.jsonl`. A reader
/// that knows only one name reports `not-found` for every session written
/// since the upgrade while the transcript sits beside the reference the
/// journal recorded.
#[test]
fn a_versioned_dsh_session_file_is_discovered() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let session = home.join("sessions/one/project/seat");
    std::fs::create_dir_all(&session).unwrap();
    std::fs::write(
        session.join("session.v3.jsonl"),
        "{\"type\":\"session\",\"delegationDepth\":0,\"version\":0}\n",
    )
    .unwrap();
    let reference = common("dsh-session", "sessions/one", home.to_str().unwrap());
    let read = read_common(&reference);
    assert_eq!(read.unavailable, None, "{:?}", read.explanation);
    assert!(
        read.path.as_deref().unwrap().ends_with("session.v3.jsonl"),
        "{:?}",
        read.path
    );
}

/// A session directory yields at most one candidate, so a directory holding
/// both admitted names admits the newer and never becomes ambiguous with
/// itself.
#[test]
fn a_session_directory_holding_both_names_admits_the_versioned_one() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let session = home.join("sessions/one/project/seat");
    std::fs::create_dir_all(&session).unwrap();
    let header = "{\"type\":\"session\",\"delegationDepth\":0,\"version\":0}\n";
    std::fs::write(session.join("session.jsonl"), header).unwrap();
    std::fs::write(session.join("session.v3.jsonl"), header).unwrap();
    let reference = common("dsh-session", "sessions/one", home.to_str().unwrap());
    let read = read_common(&reference);
    assert_eq!(read.unavailable, None, "{:?}", read.explanation);
    assert!(
        read.path.as_deref().unwrap().ends_with("session.v3.jsonl"),
        "{:?}",
        read.path
    );
}

/// The versioned name beside the plain name is decisive whatever its
/// version: a version-three `session.v3.jsonl` beside a version-zero
/// `session.jsonl` reads the versioned file's message and never the plain
/// name's.
#[test]
fn a_versioned_and_plain_pair_reads_the_versioned_files_message() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let session = home.join("sessions/one/project/seat");
    std::fs::create_dir_all(&session).unwrap();
    std::fs::write(
        session.join("session.v3.jsonl"),
        concat!(
            "{\"type\":\"session\",\"version\":3,\"delegationDepth\":0,\"isSeeded\":false}\n",
            "{\"type\":\"user/message\",\"seq\":1,\"time\":1,\"data\":{\"content\":[{\"type\":\"text\",\"text\":\"newest\"}]}}\n",
        ),
    )
    .unwrap();
    std::fs::write(
        session.join("session.jsonl"),
        concat!(
            "{\"type\":\"session\",\"version\":0,\"delegationDepth\":0}\n",
            "{\"type\":\"user/message\",\"seq\":1,\"time\":1,\"data\":{\"content\":[{\"type\":\"text\",\"text\":\"superseded\"}]}}\n",
        ),
    )
    .unwrap();
    let reference = common("dsh-session", "sessions/one", home.to_str().unwrap());
    let read = read_common(&reference);
    assert!(read.is_readable(), "{read:?}");
    assert!(
        read.path.as_deref().unwrap().ends_with("session.v3.jsonl"),
        "{:?}",
        read.path
    );
    assert_eq!(read.turns.len(), 1);
    assert_eq!(read.turns[0].blocks[0].text, "newest");
}

/// A foreign-versioned name beside a readable plain name refuses with the
/// confirmed versioned path, zero counts and no turns; the plain name is
/// never read once the versioned name carries a valid session header.
#[test]
fn a_foreign_versioned_name_beside_a_readable_plain_name_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let session = home.join("sessions/one/project/seat");
    std::fs::create_dir_all(&session).unwrap();
    std::fs::write(
        session.join("session.v3.jsonl"),
        "{\"type\":\"session\",\"version\":4,\"delegationDepth\":0}\n",
    )
    .unwrap();
    std::fs::write(
        session.join("session.jsonl"),
        concat!(
            "{\"type\":\"session\",\"version\":0,\"delegationDepth\":0}\n",
            "{\"type\":\"user/message\",\"seq\":1,\"time\":1,\"data\":{\"content\":[{\"type\":\"text\",\"text\":\"superseded\"}]}}\n",
        ),
    )
    .unwrap();
    let reference = common("dsh-session", "sessions/one", home.to_str().unwrap());
    let read = read_common(&reference);
    assert_eq!(read.unavailable, Some(Unavailable::UnsupportedFormat));
    assert!(
        read.path.as_deref().unwrap().ends_with("session.v3.jsonl"),
        "{:?}",
        read.path
    );
    assert_eq!(read.unrecognized_records, 0);
    assert_eq!(read.skipped_lines, 0);
    assert!(read.turns.is_empty());
}

/// The filename and the header version are independent facts: neither name
/// infers a version and neither version infers a name.
#[test]
fn the_filename_and_the_header_version_are_independent_facts() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");

    // `session.jsonl` carrying a version-three header reads under the
    // version-three vocabulary.
    let three_plain = home.join("sessions/one/project/seat");
    std::fs::create_dir_all(&three_plain).unwrap();
    std::fs::write(
        three_plain.join("session.jsonl"),
        concat!(
            "{\"type\":\"session\",\"version\":3,\"delegationDepth\":0}\n",
            "{\"type\":\"user/message\",\"seq\":1,\"time\":1,\"data\":{\"content\":[{\"type\":\"text\",\"text\":\"three\"}]}}\n",
        ),
    )
    .unwrap();
    let reference = common("dsh-session", "sessions/one", home.to_str().unwrap());
    let read = read_common(&reference);
    assert!(read.is_readable(), "{read:?}");
    assert_eq!(read.turns.len(), 1);
    assert_eq!(read.turns[0].blocks[0].text, "three");

    // `session.v3.jsonl` carrying a version-zero header reads under the
    // version-zero vocabulary.
    let zero_versioned = home.join("sessions/two/project/seat");
    std::fs::create_dir_all(&zero_versioned).unwrap();
    std::fs::write(
        zero_versioned.join("session.v3.jsonl"),
        concat!(
            "{\"type\":\"session\",\"version\":0,\"delegationDepth\":0}\n",
            "{\"type\":\"user/message\",\"seq\":1,\"time\":1,\"data\":{\"content\":[{\"type\":\"text\",\"text\":\"zero\"}]}}\n",
        ),
    )
    .unwrap();
    let reference = common("dsh-session", "sessions/two", home.to_str().unwrap());
    let read = read_common(&reference);
    assert!(read.is_readable(), "{read:?}");
    assert_eq!(read.turns.len(), 1);
    assert_eq!(read.turns[0].blocks[0].text, "zero");

    // Either name carrying version 4 refuses.
    let foreign = home.join("sessions/three/project/seat");
    std::fs::create_dir_all(&foreign).unwrap();
    std::fs::write(
        foreign.join("session.v3.jsonl"),
        "{\"type\":\"session\",\"version\":4,\"delegationDepth\":0}\n",
    )
    .unwrap();
    let reference = common("dsh-session", "sessions/three", home.to_str().unwrap());
    let read = read_common(&reference);
    assert_eq!(read.unavailable, Some(Unavailable::UnsupportedFormat));
    assert!(read.turns.is_empty());
}

/// A seeded header changes neither ownership nor admission under either
/// admitted version, and the depth rule governs ownership regardless of
/// `isSeeded`.
#[test]
fn a_seeded_header_changes_neither_ownership_nor_admission() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    // The seed spelling reaches the header and the assertion messages only:
    // a raw `"yes"` cannot be a directory component on Windows, so paths and
    // locators use a portable per-case index instead.
    for (case, (version, seed)) in [
        ("0", "true"),
        ("0", "false"),
        ("0", "\"yes\""),
        ("0", "{}"),
        ("0", "null"),
        ("3", "true"),
        ("3", "false"),
        ("3", "\"yes\""),
        ("3", "{}"),
        ("3", "null"),
    ]
    .into_iter()
    .enumerate()
    {
        let session = home.join(format!("sessions/v{version}-case{case}/project/seat"));
        std::fs::create_dir_all(&session).unwrap();
        let header = format!(
            "{{\"type\":\"session\",\"version\":{version},\"delegationDepth\":0,\"isSeeded\":{seed}}}\n"
        );
        let user = "{\"type\":\"user/message\",\"seq\":1,\"time\":1,\"data\":{\"content\":[{\"type\":\"text\",\"text\":\"q\"}]}}\n";
        std::fs::write(session.join("session.jsonl"), format!("{header}{user}")).unwrap();
        let locator = format!("sessions/v{version}-case{case}");
        let reference = common("dsh-session", &locator, home.to_str().unwrap());
        let read = read_common(&reference);
        assert!(read.is_readable(), "v{version} seed {seed}: {read:?}");
        assert_eq!(read.turns.len(), 1, "v{version} seed {seed}");
        assert_eq!(read.turns[0].blocks[0].text, "q");
        assert_eq!(read.unrecognized_records, 0, "v{version} seed {seed}");
        assert_eq!(read.skipped_lines, 0, "v{version} seed {seed}");
    }

    // An absent `isSeeded` under both versions.
    for version in ["0", "3"] {
        let session = home.join(format!("sessions/v{version}-absent/project/seat"));
        std::fs::create_dir_all(&session).unwrap();
        let header =
            format!("{{\"type\":\"session\",\"version\":{version},\"delegationDepth\":0}}\n");
        let user = "{\"type\":\"user/message\",\"seq\":1,\"time\":1,\"data\":{\"content\":[{\"type\":\"text\",\"text\":\"q\"}]}}\n";
        std::fs::write(session.join("session.jsonl"), format!("{header}{user}")).unwrap();
        let locator = format!("sessions/v{version}-absent");
        let reference = common("dsh-session", &locator, home.to_str().unwrap());
        let read = read_common(&reference);
        assert!(read.is_readable(), "v{version} absent: {read:?}");
        assert_eq!(read.turns.len(), 1);
    }

    // Version 3 with `isSeeded: true` and depth 1 is not a candidate.
    let delegated = home.join("sessions/delegated/project/seat");
    std::fs::create_dir_all(&delegated).unwrap();
    std::fs::write(
        delegated.join("session.jsonl"),
        "{\"type\":\"session\",\"version\":3,\"delegationDepth\":1,\"isSeeded\":true}\n",
    )
    .unwrap();
    let reference = common("dsh-session", "sessions/delegated", home.to_str().unwrap());
    let read = read_common(&reference);
    assert_eq!(read.unavailable, Some(Unavailable::NotFound));
    assert_eq!(read.path, None);
    assert_eq!(read.full_session, None);
}

/// The set is closed: a name outside it is never read, however plausible.
#[test]
fn a_session_filename_outside_the_admitted_set_is_not_read() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let session = home.join("sessions/one/project/seat");
    std::fs::create_dir_all(&session).unwrap();
    std::fs::write(
        session.join("session.v4.jsonl"),
        "{\"type\":\"session\",\"delegationDepth\":0,\"version\":0}\n",
    )
    .unwrap();
    let reference = common("dsh-session", "sessions/one", home.to_str().unwrap());
    assert_eq!(
        read_common(&reference).unavailable,
        Some(Unavailable::NotFound)
    );
}

#[test]
fn a_dsh_header_that_is_not_an_object_is_not_a_session() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let session = home.join("sessions/one/project/seat");
    std::fs::create_dir_all(&session).unwrap();
    std::fs::write(session.join("session.jsonl"), "[1]\n").unwrap();
    let reference = common("dsh-session", "sessions/one", home.to_str().unwrap());
    assert_eq!(
        read_common(&reference).unavailable,
        Some(Unavailable::NotFound)
    );
}

#[cfg(unix)]
#[test]
fn a_symlinked_codex_sessions_root_is_unsafe() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    std::os::unix::fs::symlink(dir.path().join("elsewhere"), home.join("sessions")).unwrap();
    let reference = common("codex-thread", "thread-1", home.to_str().unwrap());
    assert_eq!(
        read_common(&reference).unavailable,
        Some(Unavailable::UnsafePath)
    );
}

#[cfg(unix)]
#[test]
fn a_symlinked_dsh_locator_component_is_unsafe() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    std::os::unix::fs::symlink(dir.path().join("elsewhere"), home.join("sessions")).unwrap();
    let reference = common("dsh-session", "sessions/one", home.to_str().unwrap());
    assert_eq!(
        read_common(&reference).unavailable,
        Some(Unavailable::UnsafePath)
    );
}

#[cfg(unix)]
#[test]
fn codex_walk_skips_non_unicode_names_and_symlinks() {
    use std::os::unix::ffi::OsStringExt;

    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let sessions = home.join("sessions");
    std::fs::create_dir_all(&sessions).unwrap();
    if !create_non_unicode_dir(&sessions.join(std::ffi::OsString::from_vec(vec![b'x', 0xff]))) {
        return;
    }
    std::os::unix::fs::symlink(
        dir.path().join("outside.jsonl"),
        sessions.join("rollout-thread-1.jsonl"),
    )
    .unwrap();
    let reference = common("codex-thread", "thread-1", home.to_str().unwrap());
    let read = read_common(&reference);
    assert!(!read.is_readable(), "{read:?}");
}

#[cfg(unix)]
#[test]
fn dsh_walk_skips_non_unicode_projects_sessions_and_symlinked_leaves() {
    use std::os::unix::ffi::OsStringExt;

    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let base = home.join("sessions/one");
    std::fs::create_dir_all(&base).unwrap();
    if !create_non_unicode_dir(&base.join(std::ffi::OsString::from_vec(vec![b'p', 0xff]))) {
        return;
    }
    std::os::unix::fs::symlink(dir.path().join("out"), base.join("plink")).unwrap();

    let project = base.join("project");
    std::fs::create_dir_all(&project).unwrap();
    if !create_non_unicode_dir(&project.join(std::ffi::OsString::from_vec(vec![b's', 0xff]))) {
        return;
    }
    std::os::unix::fs::symlink(dir.path().join("out"), project.join("slink")).unwrap();

    let session = project.join("seat");
    std::fs::create_dir_all(&session).unwrap();
    std::os::unix::fs::symlink(dir.path().join("out"), session.join("session.jsonl")).unwrap();

    let reference = common("dsh-session", "sessions/one", home.to_str().unwrap());
    let read = read_common(&reference);
    assert!(!read.is_readable(), "{read:?}");
}

#[cfg(unix)]
#[test]
fn an_unreadable_claude_project_counts_as_io() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let projects = dir.path().join("projects");
    let locked = projects.join("locked");
    std::fs::create_dir_all(&locked).unwrap();
    std::fs::write(
        locked.join("abcd-1234.jsonl"),
        "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"x\"}}\n",
    )
    .unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    let reference = common("claude-session", "abcd-1234", projects.to_str().unwrap());
    let read = read_common(&reference);
    assert_eq!(read.unavailable, Some(Unavailable::Unreadable));
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[cfg(unix)]
#[test]
fn an_unreadable_codex_sessions_root_counts_as_io() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let sessions = home.join("sessions");
    std::fs::create_dir_all(&sessions).unwrap();
    std::fs::set_permissions(&sessions, std::fs::Permissions::from_mode(0o000)).unwrap();
    let reference = common("codex-thread", "thread-1", home.to_str().unwrap());
    let read = read_common(&reference);
    assert_eq!(read.unavailable, Some(Unavailable::Unreadable));
    std::fs::set_permissions(&sessions, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[cfg(unix)]
#[test]
fn an_unreadable_dsh_locator_component_counts_as_io() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let sessions = home.join("sessions");
    std::fs::create_dir_all(&sessions).unwrap();
    std::fs::set_permissions(&sessions, std::fs::Permissions::from_mode(0o000)).unwrap();
    let reference = common("dsh-session", "sessions/one", home.to_str().unwrap());
    let read = read_common(&reference);
    assert_eq!(read.unavailable, Some(Unavailable::Unreadable));
    std::fs::set_permissions(&sessions, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[cfg(unix)]
#[test]
fn unreadable_dsh_projects_sessions_and_leaves_count_as_io() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let base = home.join("sessions/one");
    std::fs::create_dir_all(&base).unwrap();
    let locked_project = base.join("locked");
    std::fs::create_dir_all(&locked_project).unwrap();
    std::fs::set_permissions(&locked_project, std::fs::Permissions::from_mode(0o000)).unwrap();
    let reference = common("dsh-session", "sessions/one", home.to_str().unwrap());
    assert_eq!(
        read_common(&reference).unavailable,
        Some(Unavailable::Unreadable)
    );
    std::fs::set_permissions(&locked_project, std::fs::Permissions::from_mode(0o755)).unwrap();

    let project = base.join("project");
    let locked_session = project.join("locked-session");
    std::fs::create_dir_all(&locked_session).unwrap();
    std::fs::set_permissions(&locked_session, std::fs::Permissions::from_mode(0o000)).unwrap();
    assert_eq!(
        read_common(&reference).unavailable,
        Some(Unavailable::Unreadable)
    );
    std::fs::set_permissions(&locked_session, std::fs::Permissions::from_mode(0o755)).unwrap();

    let session = project.join("seat");
    std::fs::create_dir_all(&session).unwrap();
    let leaf = session.join("session.jsonl");
    std::fs::write(&leaf, "{\"type\":\"session\",\"version\":0}\n").unwrap();
    std::fs::set_permissions(&leaf, std::fs::Permissions::from_mode(0o000)).unwrap();
    assert_eq!(
        read_common(&reference).unavailable,
        Some(Unavailable::Unreadable)
    );
    std::fs::set_permissions(&leaf, std::fs::Permissions::from_mode(0o644)).unwrap();
}

#[test]
fn dsh_walk_ignores_regular_files_and_directory_leaves() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let base = home.join("sessions/one");
    std::fs::create_dir_all(&base).unwrap();
    std::fs::write(base.join("stray"), "x").unwrap();

    let project = base.join("project");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(project.join("stray-session"), "x").unwrap();
    std::fs::create_dir_all(project.join("dirleaf/session.jsonl")).unwrap();

    let reference = common("dsh-session", "sessions/one", home.to_str().unwrap());
    assert_eq!(
        read_common(&reference).unavailable,
        Some(Unavailable::NotFound)
    );
}

#[cfg(unix)]
#[test]
fn safe_fs_classifies_non_regular_children_as_unsafe() {
    use std::os::unix::ffi::OsStrExt;

    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root");
    std::fs::create_dir_all(&root).unwrap();
    let fifo = root.join("pipe.jsonl");
    let c = std::ffi::CString::new(fifo.to_str().unwrap()).unwrap();
    assert_eq!(unsafe { libc_mkfifo(c.as_ptr(), 0o644) }, 0);
    let handle = safe_fs::Dir::open_root(root.to_str().unwrap()).unwrap();
    let name = std::ffi::OsStr::from_bytes(b"pipe.jsonl");
    let got = handle.child(name).expect("child open");
    assert!(matches!(got, safe_fs::Child::Unsafe), "classification");
}

#[cfg(unix)]
#[test]
fn an_unreadable_claude_candidate_counts_as_io() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let projects = dir.path().join("projects");
    std::fs::create_dir_all(projects.join("one")).unwrap();
    let file = projects.join("one/abcd-1234.jsonl");
    std::fs::write(
        &file,
        "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"x\"}}\n",
    )
    .unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o000)).unwrap();
    let reference = common("claude-session", "abcd-1234", projects.to_str().unwrap());
    assert_eq!(
        read_common(&reference).unavailable,
        Some(Unavailable::Unreadable)
    );
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
}

#[cfg(unix)]
#[test]
fn an_unreadable_codex_rollout_candidate_counts_as_io() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let sessions = home.join("sessions");
    std::fs::create_dir_all(&sessions).unwrap();
    let file = sessions.join("rollout-thread-1.jsonl");
    std::fs::write(&file, "{}\n").unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o000)).unwrap();
    let reference = common("codex-thread", "thread-1", home.to_str().unwrap());
    assert_eq!(
        read_common(&reference).unavailable,
        Some(Unavailable::Unreadable)
    );
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
}
