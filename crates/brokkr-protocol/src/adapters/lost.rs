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

/// Did the command end by a signal? It then returned no exit status, which
/// the invocation reads as -1, and so no result of its own.
pub(super) fn by_signal(exit_code: i32) -> bool {
    let signalled = exit_code == -1;
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
