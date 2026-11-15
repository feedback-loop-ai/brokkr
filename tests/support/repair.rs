//! The repair payload's args, built once for every test suite that plants
//! or checks a repair record (decision 0108 ruling 4).

use serde_json::{json, Value};

/// The args a repair writes for `terminal`'s late tail, from `first` to
/// `last`, each event's type in order.
pub(crate) fn repair_args(
    terminal: u64,
    first: u64,
    last: u64,
    event_types: &[&str],
    reason: &str,
) -> Value {
    json!({"first_seq": first, "last_seq": last, "terminal_seq": terminal,
           "event_types": event_types, "reason": reason})
}
