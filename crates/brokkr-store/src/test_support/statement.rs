//! A deterministic seam between two statements of one connection (fix
//! round 0106b1r5, H1): SQLite's trace runs a closure as a named statement
//! begins, before it reads anything, so a test commits through another
//! connection exactly between the statements before it and that one.

use rusqlite::trace::{TraceEvent, TraceEventCodes};
use std::cell::RefCell;

use crate::Store;

/// The closure due, and the text of the statement it is due before.
type Between = (&'static str, Box<dyn FnOnce()>);

thread_local! {
    // The trace runs on the thread stepping the statement, so one slot per
    // test thread keeps parallel tests apart.
    static BETWEEN: RefCell<Option<Between>> = const { RefCell::new(None) };
}

/// Run `read` on `store`, and run `between` once, on this thread, as the
/// first statement whose text holds `needle` begins on `store`'s
/// connection: whatever `between` commits through another connection lands
/// between the statements before it and that one. The cool-down snapshot
/// test commits a refusal through it. Returns `read`'s value and whether
/// `between` ran.
pub fn before_statement<T>(
    store: &mut Store,
    needle: &'static str,
    between: impl FnOnce() + 'static,
    read: impl FnOnce(&Store) -> T,
) -> (T, bool) {
    BETWEEN.with(|slot| *slot.borrow_mut() = Some((needle, Box::new(between))));
    let guard = Cleared;
    store
        .conn
        .trace_v2(
            TraceEventCodes::SQLITE_TRACE_STMT | TraceEventCodes::SQLITE_TRACE_PROFILE,
            Some(begun),
        )
        .expect("a statement trace installs");
    let read = read(store);
    let ran = BETWEEN.with(|slot| slot.borrow().is_none());
    drop(guard);
    store
        .conn
        .trace_v2(TraceEventCodes::empty(), None::<fn(TraceEvent<'_>)>)
        .expect("a statement trace clears");
    (read, ran)
}

/// The trace: the due closure runs, once, as its statement begins.
fn begun(event: TraceEvent<'_>) {
    let TraceEvent::Stmt(_, sql) = event else {
        return;
    };
    let due = BETWEEN.with(|slot| {
        let mut slot = slot.borrow_mut();
        match slot.as_ref() {
            Some((needle, _)) if sql.contains(needle) => slot.take(),
            _ => None,
        }
    });
    if let Some((_, between)) = due {
        between();
    }
}

/// The slot is cleared however `read` leaves, so a panic leaves no pending
/// closure for the thread's next test.
struct Cleared;

impl Drop for Cleared {
    fn drop(&mut self) {
        BETWEEN.with(|slot| slot.borrow_mut().take());
    }
}
