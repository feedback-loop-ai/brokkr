//! The cool-down tests' support for #1110, apart from `tests.rs` so that
//! file stays within ruling 4's 2,000-line test ceiling: the seat limits a
//! fixture gives, and the witness a failed assertion prints.

use std::path::Path;

use super::*;

/// The seat's limits where its deadline is not what the test proves: far
/// past any load, so only the clock the test drives decides the rounds
/// (#1110, as #1099's item 4 did for the sealing tests).
pub(super) fn work(attempts: u64) -> Limits {
    Limits {
        max_attempts: attempts,
        timeout_seconds: 60,
    }
}

/// The seat's limits where a probe's lateness is what the test proves: a
/// short deadline, which the hanging probe's `sleep 30` outlives.
fn late(attempts: u64) -> Limits {
    Limits {
        max_attempts: attempts,
        timeout_seconds: 2,
    }
}

/// The limits a seat whose probes do `probed` is given: the short one only
/// where a probe hangs to be ended by its deadline.
pub(super) fn limits_for(probed: Probed, attempts: u64) -> Limits {
    match probed {
        Probed::Hang => late(attempts),
        Probed::Answer
        | Probed::Refuse
        | Probed::Error
        | Probed::Unadvertised
        | Probed::Barrier
        | Probed::HangThenAnswer
        | Probed::RefuseThenAnswer
        | Probed::OtherRefusal
        | Probed::MalformedRefusal
        | Probed::BarrierRefuse => work(attempts),
    }
}

/// What a failed fixture assertion prints, so a failure says why (#1110):
/// the journal's event types in order with each failure's words and
/// refusals, each panel member's outcome and any cleanup left unproven,
/// every member shim's own log and the shared one, and the deadline each
/// seat's attempt and probe were given.
pub(super) fn witness(engine: &Engine, dir: &Path, members: &[&str]) -> String {
    let mut seen = String::from("\njournal:");
    for event in engine.store.load(&engine.run_id).unwrap() {
        seen.push_str(&format!("\n  {} {}", event.seq, event.event_type.as_str()));
        for field in [
            "checkpoint",
            "error",
            "reason",
            "refused",
            "cleanup",
            "unresolved_members",
            "waits",
        ] {
            if let Some(value) = event.payload.get(field) {
                seen.push_str(&format!("\n    {field}: {value}"));
            }
        }
    }
    seen.push_str("\nshims:");
    for tag in members.iter().copied().chain(["all"]) {
        seen.push_str(&format!("\n  {tag}: {:?}", logs(dir, tag)));
    }
    seen.push_str(&format!("\ndeadlines (host {:?}):", engine.cooldown.host));
    for (name, seat) in &engine.bundle.seats {
        let limits = &seat.limits;
        seen.push_str(&format!(
            "\n  {name}: {}s per attempt and probe, {} attempts",
            limits.timeout_seconds, limits.max_attempts
        ));
    }
    seen
}

/// A failed fixture assertion says why (#1110): the witness names the
/// journal's events with the failure's words, each shim's log, and the
/// deadline the seat's attempt and probe were given.
#[test]
fn the_witness_names_the_journal_the_shims_and_the_deadlines() {
    let (dir, mut engine, _now) = seated!(&["refuse", "succeed"], Probed::Answer, None, 2);
    engine.drive().unwrap();
    let seen = witness(&engine, dir.path(), &["work"]);
    for line in [
        "\n  5 effect/failed\n    error: \"provider refused; stderr tail: \"\n    waits: [{",
        "\n  6 effect/started\n  7 effect/succeeded\n",
        "\nshims:\n  work: [\"attempt:refuse\", \"probe\", \"attempt:succeed\"]\n  all: [",
        "\n  work: 60s per attempt and probe, 2 attempts",
    ] {
        assert!(seen.contains(line), "{line:?} is not in:{seen}");
    }
}
