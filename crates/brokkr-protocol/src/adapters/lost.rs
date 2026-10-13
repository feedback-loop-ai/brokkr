//! A start whose executor left no result of its own (decision 0006's
//! 2026-10-11 addendum, ruling 1). The built-in driver sends `failed` only
//! for a failure its command reported, or for a refusal before anything
//! was spawned. When the command was spawned and then left no result, by
//! a signal, by writing no result file, or by an invocation error after
//! the spawn, the driver sends nothing in its place and ends: the
//! transport reads the attempt as lost, and the engine parks it, because
//! the command may already have acted. Why it ended goes to stderr.

use std::cell::Cell;

use super::LaunchTerminal;
use crate::{Body, ResultStatus};

thread_local! {
    /// Has a harness been spawned since this seat's invocation began?
    /// `spawn_harness` is every harness's one spawn door, and it runs on
    /// the thread that serves the start, so the fact needs no change to
    /// the invocation builders' signatures.
    static SPAWNED: Cell<bool> = const { Cell::new(false) };
}

/// The seat's invocation begins: no harness of it is spawned yet.
pub(super) fn invoking() {
    SPAWNED.with(|spawned| spawned.set(false));
}

/// `spawn_harness` spawned a harness.
pub(super) fn spawned() {
    SPAWNED.with(|spawned| spawned.set(true));
}

/// Run `probe`, which observes the harness and is not the seat's spawn: a
/// version probe acts on nothing, so a spawn it made leaves the fact as
/// it found it, and a refusal after it is still one before the session.
pub(super) fn observing<T>(probe: impl FnOnce() -> T) -> T {
    let before = SPAWNED.with(Cell::get);
    let observed = probe();
    SPAWNED.with(|spawned| spawned.set(before));
    observed
}

/// An invocation's error. Before any spawn it is a refusal, sent as
/// `failed`, which a retry may repeat because nothing ran. After one it
/// is no result of the command's, so nothing is sent.
pub(super) fn invocation_failed(
    send: &mut impl FnMut(Body),
    effect_id: String,
    attempt_id: String,
    error: String,
) {
    if SPAWNED.with(Cell::get) {
        eprintln!("{error}; the command was spawned, so no result is sent");
        return;
    }
    send(Body::Result {
        effect_id,
        attempt_id,
        status: ResultStatus::Failed,
        result: None,
        error: Some(error),
    });
}

/// The exit code an invocation reads for a command ended by a signal. It
/// returned no exit status, and no command exits with a negative code.
const SIGNALLED: i32 = -1;

/// The code the command exited with, or [`SIGNALLED`] if a signal ended it.
pub(super) fn exit_code(status: std::process::ExitStatus) -> i32 {
    status.code().unwrap_or(SIGNALLED)
}

/// Did the command exit with a code of its own, other than 0? A signal is
/// no such exit: the command may have acted before it, so it is never a
/// reason to replace the command within the attempt.
pub(super) fn exited_non_zero(exit_code: i32) -> bool {
    exit_code != 0 && exit_code != SIGNALLED
}

/// The provider's refusal before the first turn, if it stands as one
/// (decision 0053): no work began or was delivered, and the command
/// exited with a code of its own from a settled launch. A command a
/// signal ended, or a rejoin nobody confirmed, left no result whatever
/// its stream said, so it ends lost and is never retried as a refusal.
pub(super) fn refusal_to_start(
    refusal: Option<String>,
    worked: bool,
    exit_code: i32,
    launch: LaunchTerminal,
) -> Option<String> {
    refusal.filter(|_| !worked && exit_code != SIGNALLED && !launch.is_unsettled())
}

/// Did the command end by a signal? It then returned no exit status, and
/// so no result of its own.
pub(super) fn by_signal(exit_code: i32) -> bool {
    let signalled = exit_code == SIGNALLED;
    if signalled {
        eprintln!("agent CLI ended by a signal, so no result is sent");
    }
    signalled
}

/// The command exited 0 and wrote no result file.
pub(super) fn no_result_file() {
    eprintln!("seat wrote no result file (the result contract was not met), so no result is sent");
}

/// A rejoin that never confirmed the exact offered root ran its harness
/// on a session nobody knows, so nothing it wrote is its result.
pub(super) fn unsettled(launch: LaunchTerminal) {
    eprintln!("{}, so no result is sent", unsettled_reason(launch));
}

/// Why an unsettled rejoin is refused, in the words it was always refused in.
pub(super) fn unsettled_reason(launch: LaunchTerminal) -> &'static str {
    match launch {
        LaunchTerminal::Mismatch => {
            "provider named a different session than the offered root; \
             refusing to accept the invocation"
        }
        _ => {
            "provider never confirmed the offered session; \
             refusing to accept the invocation"
        }
    }
}

/// `serve` the start through `send`, and whether it sent a result. A start
/// that sent none ends the driver, so the engine reads the result lost at
/// once rather than at the deadline.
pub(super) fn answered(
    send: &mut impl FnMut(Body),
    serve: impl FnOnce(&mut dyn FnMut(Body)),
) -> bool {
    let mut answered = false;
    serve(&mut |body: Body| {
        answered |= matches!(body, Body::Result { .. });
        send(body);
    });
    answered
}
