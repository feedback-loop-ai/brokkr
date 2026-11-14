//! `repair`: the operator's record naming the late tail of a run already
//! broken by an event after its terminal one (decision 0108 ruling 4, in
//! the shape its 2026-11-13 erratum gives it after decision 0047). It
//! shares the fence and the read-back with `supersede`, and nothing else:
//! it is no command with a disposition, so no `operator/accepted` ever
//! follows it.

use brokkr_core::envelope::EventType;
use brokkr_core::fold::{fold, terminal_event, FoldError, REPAIR};
use brokkr_core::summary::Account;
use brokkr_core::EventEnvelope;
use brokkr_store::Store;
use serde_json::json;
use uuid::Uuid;

use super::{fenced_append, read_back, EngineError, Head};

/// What `brokkr operator repair` says, as the operator typed it: why the
/// late tail is kept, and who repaired it.
pub struct Repair<'a> {
    pub reason: &'a str,
    pub operator: &'a str,
}

/// Record that a broken run's late tail is kept, shown and never applied
/// (decision 0108 ruling 4). One `operator/commanded` event with
/// `command: "repair"` and the tail it names, and nothing else: no
/// `operator/accepted` follows, because there is nothing to execute and
/// `fold` refuses an acceptance on a terminal run by design.
///
/// The run is read from its EVENTS, never from its fold: a run this verb
/// exists for does not fold at all. The first terminal event is found in
/// the verified journal, and the tail runs from it to the head. Every
/// refusal comes before the first append, so a refused repair leaves the
/// journal byte for byte as it found it.
///
/// The write is FENCED on the head the read found (decision 0029). The
/// event is legal wherever it lands — a terminal status is absorbing, so
/// `fold` admits this annotation at any later seq — but a head that moved
/// is a journal that grew under the read, and re-reading beats naming a
/// tail that is no longer the whole tail.
pub fn operator_repair(
    store: &mut Store,
    run_id: &str,
    ask: &Repair<'_>,
) -> Result<EventEnvelope, EngineError> {
    repair_with(store, run_id, ask, |_| {}, |_| {})
}

/// [`operator_repair`] with `before_write` run after the read and before
/// the fenced append — the instant a peer's append makes the head stale —
/// and `after_write` run after the append and before the read-back, the
/// instant a peer's annotation can make the new journal refuse. Production
/// passes no-ops.
fn repair_with(
    store: &mut Store,
    run_id: &str,
    ask: &Repair<'_>,
    before_write: impl FnOnce(&mut Store),
    after_write: impl FnOnce(&mut Store),
) -> Result<EventEnvelope, EngineError> {
    let events = store.load(run_id)?;
    let refused = EngineError::RepairRefused;
    let Some(at) = events
        .iter()
        .position(|event| terminal_event(event.event_type).is_some())
    else {
        return Err(refused(format!(
            "run '{run_id}' has no terminal event; repair names the late tail of a run that has \
             already ended, and it is never automatic"
        )));
    };
    let terminal = &events[at];
    let tail = &events[at + 1..];
    // The run's own fold decides which refusal this is. A journal that
    // folds already has no tail to name — the head is then either a
    // repair that did its work or an annotation that never broke it. A
    // fold that refuses with anything but the late tail is a corrupt
    // record, and this verb does not add to it.
    match fold(&events) {
        Ok(state) => {
            // The fold's own late tail is the one reading: a repaired run
            // with a later lawful annotation still carries it, and is
            // still repaired rather than read as a tail no record names.
            if let Some(repaired) = state.late_tail {
                return Err(refused(format!(
                    "run '{run_id}' is already repaired at seq {}; it folds as it stands and \
                     repair appends nothing",
                    repaired.repaired_at
                )));
            }
            if tail.is_empty() {
                let status = terminal_event(terminal.event_type).expect("the index found one");
                return Err(refused(format!(
                    "run '{run_id}' is already concluded ({}), with no late tail; it folds as it \
                     stands and repair appends nothing",
                    status.as_str()
                )));
            }
            return Err(refused(format!(
                "run '{run_id}' has no late tail after its terminal event at seq {}; it folds as \
                 it stands and repair appends nothing",
                terminal.seq
            )));
        }
        Err(FoldError::AfterTerminal { .. }) => {}
        Err(other) => {
            return Err(refused(format!(
                "run '{run_id}' does not fold for another reason: {other}; repair names a late \
                 tail, never another fault"
            )))
        }
    }
    let head = Head::tip(&events);
    let payload = json!({
        "command_id": Uuid::new_v4().to_string(),
        "command": REPAIR,
        "operator": ask.operator,
        "args": {
            "first_seq": terminal.seq + 1,
            "last_seq": head.seq,
            "terminal_seq": terminal.seq,
            "event_types": tail.iter().map(|event| event.event_type.as_str()).collect::<Vec<_>>(),
            "reason": ask.reason,
        },
    });
    let cause = &head.event_id;
    // The deciding read's account goes to the store, as every writer's
    // does, so the append writes the summary it seals. The carried
    // account is the broken journal's, and a repair in the sealed event
    // answers `Refold` in the append's transaction: the row is the whole
    // journal's, not a refusal's (decision 0089 ruling 3).
    store.carry(Account::whole(run_id, &events));
    before_write(store);
    let written = fenced_append(
        store,
        run_id,
        &head,
        cause,
        EventType::OperatorCommanded,
        payload,
        None,
    )?;
    after_write(store);
    // The record must read back: the run folds again, as its terminal
    // status, with the tail the operator named. The shared read-back is
    // the load and the fold on one line, as `disposition` keeps them, so
    // the exact-coverage gate reads the arm a peer's append can reach as
    // the arm it exercises (INFO a).
    read_back(store, run_id)?;
    Ok(written)
}

#[cfg(test)]
mod tests;
