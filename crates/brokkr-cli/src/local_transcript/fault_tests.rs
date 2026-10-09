//! The fault-seam proofs (change `prove-transcript-reader-faults`, D2-D8):
//! the reader's error arms reached only by a filesystem failure or race.

use super::tests::{common, read_common};
use super::*;
use brokkr_view::transcript::{TranscriptKind, ValidReference};
use safe_fs::fault::{ChangeAt, FailAt, Plan};
use serde_json::json;

fn claude_body(text: &str) -> String {
    format!(
        "{{\"type\":\"assistant\",\"message\":{{\"role\":\"assistant\",\"content\":{}}}}}\n",
        json!(text)
    )
}

fn dsh_header_text(depth: u64) -> String {
    format!("{{\"type\":\"session\",\"delegationDepth\":{depth},\"version\":0}}\n")
}

/// A discovery-stage unreadable refusal: no path, no turns, zero counts, no
/// truncation, no notices, and the kind's unresolved full-session hint.
fn assert_discovery_unreadable(
    read: &TranscriptRead,
    kind: TranscriptKind,
    reference: &brokkr_view::Transcript,
) {
    assert_eq!(read.unavailable, Some(Unavailable::Unreadable), "{read:?}");
    assert_eq!(
        read.path, None,
        "a discovery-stage refusal keeps no path: {read:?}"
    );
    assert!(read.turns.is_empty(), "{read:?}");
    assert_eq!(read.skipped_lines, 0, "{read:?}");
    assert_eq!(read.unrecognized_records, 0, "{read:?}");
    assert!(!read.truncated, "{read:?}");
    assert!(read.notices.is_empty(), "{read:?}");
    let valid = ValidReference {
        kind,
        locator: reference.locator.clone(),
        home: reference.home.clone(),
    };
    assert_eq!(
        read.full_session,
        brokkr_view::transcript::full_session(&valid, None),
        "the kind's unresolved hint is kept: {read:?}"
    );
}

/// A body-stage unreadable refusal: the admitted path is kept, no turns and
/// zero counts.
fn assert_body_unreadable(read: &TranscriptRead) {
    assert_eq!(read.unavailable, Some(Unavailable::Unreadable), "{read:?}");
    assert!(
        read.path.is_some(),
        "a body-stage refusal keeps the admitted path: {read:?}"
    );
    assert!(read.turns.is_empty(), "{read:?}");
    assert_eq!(read.skipped_lines, 0, "{read:?}");
    assert_eq!(read.unrecognized_records, 0, "{read:?}");
    assert!(!read.truncated, "{read:?}");
}

pub(crate) fn claude_fixture(text: &str) -> (tempfile::TempDir, brokkr_view::Transcript) {
    let dir = tempfile::tempdir().unwrap();
    let projects = dir.path().join("projects");
    std::fs::create_dir_all(projects.join("one")).unwrap();
    std::fs::write(projects.join("one/abcd-1234.jsonl"), claude_body(text)).unwrap();
    let reference = common("claude-session", "abcd-1234", projects.to_str().unwrap());
    (dir, reference)
}

fn codex_fixture(id: &str) -> (tempfile::TempDir, brokkr_view::Transcript) {
    let dir = tempfile::tempdir().unwrap();
    let sessions = dir.path().join("sessions");
    std::fs::create_dir_all(&sessions).unwrap();
    std::fs::write(
        sessions.join(format!("rollout-{id}.jsonl")),
        "{\"type\":\"turn_context\"}\n",
    )
    .unwrap();
    let reference = common("codex-thread", id, dir.path().to_str().unwrap());
    (dir, reference)
}

fn dsh_fixture(locator: &str) -> (tempfile::TempDir, brokkr_view::Transcript) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("dsh");
    let seat = root.join(locator).join("project").join("seat");
    std::fs::create_dir_all(&seat).unwrap();
    std::fs::write(seat.join("session.jsonl"), dsh_header_text(0)).unwrap();
    let reference = common("dsh-session", locator, root.to_str().unwrap());
    (dir, reference)
}

#[test]
#[should_panic(expected = "fault seam entries never fired")]
fn a_plan_whose_entry_never_fires_fails_its_test() {
    let _guard = Plan::new().fail(FailAt::Entries, 1).install();
}

#[test]
#[should_panic(expected = "fault seam entries never fired: Child occurrence 1")]
fn a_change_entry_that_never_fires_names_itself() {
    let _guard = Plan::new().change(ChangeAt::Child, 1, || {}).install();
}

/// The proof `take_change_action` leans on when it leaves `!fired` out of
/// its guard: every visit to a timed point is a new occurrence, so no
/// `(target, occurrence)` is visited twice and a matching entry is never
/// already fired. A counter that stopped advancing would fire occurrence 1
/// on every visit and leave occurrences 2 and 3 unfired.
#[test]
fn every_visit_to_a_timed_point_is_a_new_occurrence() {
    let order = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let plan = (1..=3).fold(Plan::new(), |plan, occurrence| {
        let order = std::rc::Rc::clone(&order);
        plan.change(ChangeAt::Child, occurrence, move || {
            order.borrow_mut().push(occurrence);
        })
    });
    let guard = plan.install();
    for _ in 0..3 {
        safe_fs::fault::point(ChangeAt::Child);
    }
    drop(guard);
    assert_eq!(*order.borrow(), vec![1, 2, 3]);
}

#[test]
#[should_panic(expected = "a fault plan is already installed on this thread")]
fn a_nested_install_is_refused_and_its_unwinding_clears_the_plan() {
    let _outer = Plan::new().fail(FailAt::Entries, 1).install();
    let _nested = Plan::new().install();
}

#[test]
fn a_plan_is_scoped_to_its_installing_thread_and_ends_with_its_test() {
    let dir = tempfile::tempdir().unwrap();
    let projects = dir.path().join("projects");
    std::fs::create_dir_all(projects.join("one")).unwrap();
    std::fs::write(projects.join("one/abcd-1234.jsonl"), claude_body("one")).unwrap();
    let home = projects.to_str().unwrap().to_string();
    let reference = common("claude-session", "abcd-1234", &home);

    // The first lookup enumerates once for discovery and once for the
    // acquisition re-walk; the separate second lookup is occurrence 3.
    let _guard = Plan::new().fail(FailAt::Entries, 3).install();
    assert!(
        read_common(&reference).is_readable(),
        "occurrence 1 sees the real filesystem"
    );

    let other_home = home.clone();
    let other = std::thread::spawn(move || {
        let reference = common("claude-session", "abcd-1234", &other_home);
        read_common(&reference).is_readable()
    })
    .join()
    .unwrap();
    assert!(other, "the other thread sees only real filesystem results");

    assert_eq!(
        read_common(&reference).unavailable,
        Some(Unavailable::Unreadable)
    );
}

#[test]
fn a_scripted_enumeration_error_is_a_discovery_stage_unreadable() {
    {
        let (_dir, reference) = claude_fixture("one");
        let _guard = Plan::new().fail(FailAt::Entries, 1).install();
        let read = read_common(&reference);
        assert_discovery_unreadable(&read, TranscriptKind::ClaudeSession, &reference);
    }
    {
        let (_dir, reference) = codex_fixture("0199mine");
        let _guard = Plan::new().fail(FailAt::Entries, 1).install();
        let read = read_common(&reference);
        assert_discovery_unreadable(&read, TranscriptKind::CodexThread, &reference);
    }
    {
        let (_dir, reference) = dsh_fixture("sessions/one");
        let _guard = Plan::new().fail(FailAt::Entries, 1).install();
        let read = read_common(&reference);
        assert_discovery_unreadable(&read, TranscriptKind::DshSession, &reference);
    }
    {
        let (_dir, reference) = dsh_fixture("sessions/one");
        let _guard = Plan::new().fail(FailAt::Entries, 2).install();
        let read = read_common(&reference);
        assert_discovery_unreadable(&read, TranscriptKind::DshSession, &reference);
    }
}

#[test]
fn a_scripted_identity_error_prevents_a_unique_answer() {
    {
        let (_dir, reference) = claude_fixture("one");
        let _guard = Plan::new().fail(FailAt::Identity, 1).install();
        let read = read_common(&reference);
        assert_discovery_unreadable(&read, TranscriptKind::ClaudeSession, &reference);
    }
    {
        let dir = tempfile::tempdir().unwrap();
        let projects = dir.path().join("projects");
        for project in ["one", "two"] {
            std::fs::create_dir_all(projects.join(project)).unwrap();
            std::fs::write(
                projects.join(project).join("abcd-1234.jsonl"),
                claude_body(project),
            )
            .unwrap();
        }
        let reference = common("claude-session", "abcd-1234", projects.to_str().unwrap());
        let _guard = Plan::new().fail(FailAt::Identity, 1).install();
        let read = read_common(&reference);
        assert_discovery_unreadable(&read, TranscriptKind::ClaudeSession, &reference);
    }
    {
        let (_dir, reference) = codex_fixture("0199mine");
        let _guard = Plan::new().fail(FailAt::Identity, 1).install();
        let read = read_common(&reference);
        assert_discovery_unreadable(&read, TranscriptKind::CodexThread, &reference);
    }
    {
        let dir = tempfile::tempdir().unwrap();
        let sessions = dir.path().join("sessions");
        std::fs::create_dir_all(&sessions).unwrap();
        for id in ["0199mine", "0199other"] {
            std::fs::write(
                sessions.join(format!("rollout-{id}.jsonl")),
                "{\"type\":\"turn_context\"}\n",
            )
            .unwrap();
        }
        let reference = common("codex-thread", "0199mine", dir.path().to_str().unwrap());
        let _guard = Plan::new().fail(FailAt::Identity, 1).install();
        let read = read_common(&reference);
        assert_discovery_unreadable(&read, TranscriptKind::CodexThread, &reference);
    }
    {
        let (_dir, reference) = dsh_fixture("sessions/one");
        let _guard = Plan::new().fail(FailAt::Identity, 1).install();
        let read = read_common(&reference);
        assert_discovery_unreadable(&read, TranscriptKind::DshSession, &reference);
    }
    {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("dsh");
        let locator = "sessions/one";
        for project in ["project", "second"] {
            let seat = root.join(locator).join(project).join("seat");
            std::fs::create_dir_all(&seat).unwrap();
            std::fs::write(seat.join("session.jsonl"), dsh_header_text(0)).unwrap();
        }
        let reference = common("dsh-session", locator, root.to_str().unwrap());
        let _guard = Plan::new().fail(FailAt::Identity, 1).install();
        let read = read_common(&reference);
        assert_discovery_unreadable(&read, TranscriptKind::DshSession, &reference);
    }
}

#[test]
fn a_scripted_dsh_header_read_error_is_discovery_stage_unreadable() {
    let (_dir, reference) = dsh_fixture("sessions/one");
    let _guard = Plan::new().fail(FailAt::Read, 1).install();
    let read = read_common(&reference);
    assert_discovery_unreadable(&read, TranscriptKind::DshSession, &reference);
    assert_eq!(read.full_session, None, "the DSH hint stays null: {read:?}");
}

#[test]
fn a_failed_retained_handle_identity_refuses_without_prose() {
    {
        let (dir, reference) = claude_fixture("body");
        let file = dir.path().join("projects/one/abcd-1234.jsonl");
        let before = std::fs::read(&file).unwrap();
        let _guard = Plan::new().fail(FailAt::Identity, 3).install();
        let read = read_common(&reference);
        assert_body_unreadable(&read);
        assert_eq!(std::fs::read(&file).unwrap(), before, "bytes unchanged");
    }
    {
        let (dir, reference) = codex_fixture("0199mine");
        let file = dir.path().join("sessions/rollout-0199mine.jsonl");
        let before = std::fs::read(&file).unwrap();
        let _guard = Plan::new().fail(FailAt::Identity, 3).install();
        let read = read_common(&reference);
        assert_body_unreadable(&read);
        assert_eq!(std::fs::read(&file).unwrap(), before, "bytes unchanged");
    }
    {
        let (dir, reference) = dsh_fixture("sessions/one");
        let file = dir
            .path()
            .join("dsh/sessions/one/project/seat/session.jsonl");
        let before = std::fs::read(&file).unwrap();
        let _guard = Plan::new().fail(FailAt::Identity, 3).install();
        let read = read_common(&reference);
        assert_body_unreadable(&read);
        assert_eq!(std::fs::read(&file).unwrap(), before, "bytes unchanged");
    }
}

#[test]
fn a_failed_bounded_source_read_refuses_without_prose() {
    {
        let (dir, reference) = claude_fixture("body");
        let file = dir.path().join("projects/one/abcd-1234.jsonl");
        let before = std::fs::read(&file).unwrap();
        let _guard = Plan::new().fail(FailAt::Read, 1).install();
        let read = read_common(&reference);
        assert_body_unreadable(&read);
        assert_eq!(std::fs::read(&file).unwrap(), before, "bytes unchanged");
    }
    {
        let (dir, reference) = codex_fixture("0199mine");
        let file = dir.path().join("sessions/rollout-0199mine.jsonl");
        let before = std::fs::read(&file).unwrap();
        let _guard = Plan::new().fail(FailAt::Read, 1).install();
        let read = read_common(&reference);
        assert_body_unreadable(&read);
        assert_eq!(std::fs::read(&file).unwrap(), before, "bytes unchanged");
    }
    {
        let (dir, reference) = dsh_fixture("sessions/one");
        let file = dir
            .path()
            .join("dsh/sessions/one/project/seat/session.jsonl");
        let before = std::fs::read(&file).unwrap();
        // Two header reads happen before the source read: discovery and the
        // acquisition re-walk.
        let _guard = Plan::new().fail(FailAt::Read, 3).install();
        let read = read_common(&reference);
        assert_body_unreadable(&read);
        assert_eq!(std::fs::read(&file).unwrap(), before, "bytes unchanged");
    }
}

#[test]
fn a_codex_entry_that_vanishes_before_its_open_is_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let sessions = dir.path().join("sessions");
    std::fs::create_dir_all(&sessions).unwrap();
    let rollout = sessions.join("rollout-0199mine.jsonl");
    std::fs::write(&rollout, "{\"type\":\"turn_context\"}\n").unwrap();
    let reference = common("codex-thread", "0199mine", dir.path().to_str().unwrap());
    let target = rollout.clone();
    // `sessions` itself is `Child` 1; the rollout is `Child` 2.
    let _guard = Plan::new()
        .change(ChangeAt::Child, 2, move || {
            std::fs::remove_file(&target).unwrap();
        })
        .install();
    let read = read_common(&reference);
    assert_eq!(read.unavailable, Some(Unavailable::NotFound), "{read:?}");
    assert_eq!(read.path, None);
    assert!(read.turns.is_empty());
}

/// One read of a Claude fixture whose leaf `change` alters at the first
/// child-file attempt, between the project's open and the leaf's.
fn read_with_the_leaf_changed_at_the_file_attempt(
    change: impl FnOnce(&Path) + 'static,
) -> TranscriptRead {
    let (dir, reference) = claude_fixture("gone");
    let target = dir.path().join("projects/one/abcd-1234.jsonl");
    let _guard = Plan::new()
        .change(ChangeAt::ChildFileAttempt, 1, move || change(&target))
        .install();
    read_common(&reference)
}

#[test]
fn a_claude_entry_that_vanishes_at_the_file_attempt_is_not_found() {
    let read = read_with_the_leaf_changed_at_the_file_attempt(|target| {
        std::fs::remove_file(target).unwrap();
    });
    assert_eq!(read.unavailable, Some(Unavailable::NotFound), "{read:?}");
    assert_eq!(read.path, None);
    assert!(read.turns.is_empty());
}

#[test]
fn a_claude_child_file_attempt_placement_witness_is_unsafe_path() {
    let read = read_with_the_leaf_changed_at_the_file_attempt(|target| {
        std::fs::remove_file(target).unwrap();
        std::fs::create_dir(target).unwrap();
    });
    assert_eq!(read.unavailable, Some(Unavailable::UnsafePath), "{read:?}");
}

#[test]
fn a_rename_before_the_rewalk_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    let projects = dir.path().join("projects");
    std::fs::create_dir_all(projects.join("one")).unwrap();
    std::fs::create_dir_all(projects.join("two")).unwrap();
    let file = projects.join("one/abcd-1234.jsonl");
    std::fs::write(&file, claude_body("original")).unwrap();
    let reference = common("claude-session", "abcd-1234", projects.to_str().unwrap());
    let source = file.clone();
    let moved = projects.join("two/abcd-1234.jsonl");
    let _guard = Plan::new()
        .change(ChangeAt::BeforeRewalk, 1, move || {
            std::fs::rename(&source, &moved).unwrap();
        })
        .install();
    let read = read_common(&reference);
    assert_body_unreadable(&read);
}

#[test]
fn a_replacement_before_the_rewalk_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    let projects = dir.path().join("projects");
    std::fs::create_dir_all(projects.join("one")).unwrap();
    let file = projects.join("one/abcd-1234.jsonl");
    std::fs::write(&file, claude_body("original")).unwrap();
    let reference = common("claude-session", "abcd-1234", projects.to_str().unwrap());
    let source = file.clone();
    let holding = projects.join("one/holding.txt");
    let staged = projects.join("one/staged.jsonl");
    let _guard = Plan::new()
        .change(ChangeAt::BeforeRewalk, 1, move || {
            std::fs::rename(&source, &holding).unwrap();
            std::fs::write(&staged, claude_body("replacement")).unwrap();
            std::fs::rename(&staged, &source).unwrap();
        })
        .install();
    let read = read_common(&reference);
    assert_body_unreadable(&read);
}

#[cfg(unix)]
#[test]
fn the_seam_leaves_the_boundary_checks_in_force() {
    use std::os::unix::fs::symlink;

    extern "C" {
        fn mkfifo(path: *const std::os::raw::c_char, mode: u32) -> std::os::raw::c_int;
    }

    let dir = tempfile::tempdir().unwrap();
    // A symlinked project, a symlinked candidate, a FIFO candidate and a
    // regular candidate, each in its own root so one read is one lookup.
    let linked_project = dir.path().join("linked-project");
    let real = linked_project.join("real");
    std::fs::create_dir_all(&real).unwrap();
    symlink(&real, linked_project.join("one")).unwrap();

    let linked_candidate = dir.path().join("linked-candidate");
    std::fs::create_dir_all(linked_candidate.join("one")).unwrap();
    let target = linked_candidate.join("target");
    std::fs::write(&target, claude_body("target")).unwrap();
    symlink(&target, linked_candidate.join("one/abcd-1234.jsonl")).unwrap();

    let fifo = dir.path().join("fifo");
    std::fs::create_dir_all(fifo.join("one")).unwrap();
    let fifo_path = fifo.join("one/abcd-1234.jsonl");
    let fifo_c = std::ffi::CString::new(fifo_path.to_str().unwrap()).unwrap();
    assert_eq!(unsafe { mkfifo(fifo_c.as_ptr(), 0o644) }, 0);

    let regular = dir.path().join("regular");
    std::fs::create_dir_all(regular.join("one")).unwrap();
    std::fs::write(regular.join("one/abcd-1234.jsonl"), claude_body("regular")).unwrap();

    // One enumeration per unadmitted read: linked project (1), linked
    // candidate (2), FIFO (3). The regular candidate is admitted, so it
    // enumerates once for discovery and once for the re-walk (4 and 5);
    // the separate later lookup is occurrence 6.
    let _guard = Plan::new().fail(FailAt::Entries, 6).install();

    let linked_project_ref = common(
        "claude-session",
        "abcd-1234",
        linked_project.to_str().unwrap(),
    );
    assert_eq!(
        read_common(&linked_project_ref).unavailable,
        Some(Unavailable::UnsafePath),
        "the symlinked project keeps its no-follow check"
    );

    let linked_candidate_ref = common(
        "claude-session",
        "abcd-1234",
        linked_candidate.to_str().unwrap(),
    );
    assert_eq!(
        read_common(&linked_candidate_ref).unavailable,
        Some(Unavailable::UnsafePath),
        "the symlinked candidate keeps its no-follow check"
    );

    let fifo_ref = common("claude-session", "abcd-1234", fifo.to_str().unwrap());
    assert_eq!(
        read_common(&fifo_ref).unavailable,
        Some(Unavailable::UnsafePath),
        "the FIFO candidate keeps its non-blocking regular-file check"
    );

    let regular_ref = common("claude-session", "abcd-1234", regular.to_str().unwrap());
    let read = read_common(&regular_ref);
    assert!(read.is_readable(), "the regular candidate stays readable");

    // A separate, otherwise valid lookup whose enumeration is the entry's
    // occurrence: it fires here, so the unfired-entry check passes unchanged
    // and the lookup returns discovery-stage unreadable.
    let later = read_common(&regular_ref);
    assert_eq!(
        later.unavailable,
        Some(Unavailable::Unreadable),
        "{later:?}"
    );
    assert_eq!(later.path, None);
}

#[test]
fn a_discovery_limit_with_two_candidates_is_ambiguous_not_limited() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("dsh");
    let locator = "sessions/one";
    for project in ["a", "b"] {
        let seat = root.join(locator).join(project).join("seat");
        std::fs::create_dir_all(&seat).unwrap();
        std::fs::write(seat.join("session.jsonl"), dsh_header_text(0)).unwrap();
    }
    // A third owned session whose opening header outgrows the cap sets the
    // limit fact while two candidates already stand: the two-candidate
    // branch wins and the answer is ambiguous, not discovery-limited.
    let oversized = root.join(locator).join("c").join("seat");
    std::fs::create_dir_all(&oversized).unwrap();
    std::fs::write(
        oversized.join("session.jsonl"),
        vec![b'x'; brokkr_view::transcript::DSH_HEADER_CAP + 8],
    )
    .unwrap();
    let reference = common("dsh-session", locator, root.to_str().unwrap());
    let read = read_common(&reference);
    assert_eq!(
        read.unavailable,
        Some(Unavailable::AmbiguousSource),
        "{read:?}"
    );
}

#[test]
fn codex_whole_token_branches_at_the_filename_edges() {
    let dir = tempfile::tempdir().unwrap();
    let sessions = dir.path().join("sessions");
    std::fs::create_dir_all(&sessions).unwrap();
    std::fs::write(
        sessions.join("rollout-0199mine.jsonl"),
        "{\"type\":\"turn_context\"}\n",
    )
    .unwrap();

    // The id at the filename's start: `rollout` matches at index zero.
    let start = common("codex-thread", "rollout", dir.path().to_str().unwrap());
    assert!(read_common(&start).is_readable(), "start token admits");
    // The id at the filename's end: `jsonl` matches at the last token.
    let end = common("codex-thread", "jsonl", dir.path().to_str().unwrap());
    assert!(read_common(&end).is_readable(), "end token admits");
    // A non-token substring is refused: `0199` is followed by `m`.
    let inner = common("codex-thread", "0199", dir.path().to_str().unwrap());
    assert_eq!(
        read_common(&inner).unavailable,
        Some(Unavailable::NotFound),
        "a partial token is not a match"
    );
}

#[test]
fn a_codex_rollout_deeper_than_six_levels_is_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let sessions = dir.path().join("sessions");
    let deep = sessions.join("a/b/c/d/e/f/g");
    std::fs::create_dir_all(&deep).unwrap();
    std::fs::write(
        deep.join("rollout-0199mine.jsonl"),
        "{\"type\":\"turn_context\"}\n",
    )
    .unwrap();
    let reference = common("codex-thread", "0199mine", dir.path().to_str().unwrap());
    assert_eq!(
        read_common(&reference).unavailable,
        Some(Unavailable::NotFound)
    );
}
