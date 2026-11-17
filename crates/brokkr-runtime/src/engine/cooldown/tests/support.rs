//! The cool-down tests' support for #1110, apart from `tests.rs` so that
//! file stays within ruling 4's 2,000-line test ceiling: the seat limits a
//! fixture gives, and the test of the witness a failed assertion prints.

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
/// deadline the hanging probe's `sleep 30` outlives, but long enough that
/// admission and the probe's spawn cannot plausibly spend it under load
/// before the probe logs (#1110: at 2s the Hang row of
/// `the_unchanged_probe_ends_write_no_row` read `["attempt:refuse"]`).
fn late(attempts: u64) -> Limits {
    Limits {
        max_attempts: attempts,
        timeout_seconds: 10,
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

/// A failed fixture assertion says why (#1110): the shared witness names
/// the journal's events with the failure's words, every shim's log, and
/// the deadline the seat's attempt and probe were given.
#[test]
fn the_witness_names_the_journal_the_shims_and_the_deadlines() {
    let (dir, mut engine, _now) = seated!(&["refuse", "succeed"], Probed::Answer, None, 2);
    engine.drive().unwrap();
    let seen = witness(&engine, dir.path());
    for line in [
        "\n  5 effect/failed\n    error: \"provider refused; stderr tail: \"\n    waits: [{",
        "\n  6 effect/started\n  7 effect/succeeded\n",
        "\nshims:\n  all: [\"work:attempt:refuse\", \"work:probe\", \"work:attempt:succeed\"]\n  \
         work: [\"attempt:refuse\", \"probe\", \"attempt:succeed\"]\n",
        "\n  work: 60s per attempt and probe, 2 attempts",
    ] {
        assert!(seen.contains(line), "{line:?} is not in:{seen}");
    }
}
