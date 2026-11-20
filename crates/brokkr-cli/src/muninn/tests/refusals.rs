//! The text of every refusal of a report, pinned once: the tests in the
//! owning `tests.rs` match the variant. A child of that file, which sits
//! near its line ceiling.

use super::*;

use super::queue::Refusal;

#[test]
fn every_report_refusal_reads_as_it_always_has() {
    let other_realm = |stated: Option<&str>| Unusable::OtherRealm {
        what: "a work-queue entry",
        run_id: "parked-run".to_string(),
        seq: 8,
        claimed: "beta".to_string(),
        stated: stated.map(str::to_string),
    };
    let settled = Refusal::Settled {
        run_id: "landed-run".to_string(),
        ending: Ending::Withdrawn,
    };
    for (refusal, text) in [
        (
            Unusable::Unreached {
                reached: "complete".to_string(),
            },
            "the seat reached 'complete', not 'proposed'",
        ),
        (Unusable::NoInputs, "the report carries no 'inputs' object"),
        (
            Unusable::NoArray { key: "parked_runs" },
            "the report is missing the 'parked_runs' array",
        ),
        (
            Unusable::NoText {
                what: "a work-queue entry",
                key: "finding",
            },
            "a work-queue entry is missing a non-empty 'finding'",
        ),
        (
            Unusable::NoSeq {
                what: "a parked-run proposal",
            },
            "a parked-run proposal is missing a numeric 'seq'",
        ),
        (
            Unusable::Unstated {
                what: "a parked-run proposal",
                run_id: "ghost-run".to_string(),
                seq: 8,
            },
            "a parked-run proposal cites ghost-run seq 8, which the dossier does not state",
        ),
        (
            other_realm(Some("alpha")),
            "a work-queue entry cites parked-run seq 8 in realm 'beta'; the dossier states \
             that fact in realm 'alpha'",
        ),
        (
            other_realm(None),
            "a work-queue entry cites parked-run seq 8 in realm 'beta'; the dossier states \
             that fact in no realm",
        ),
        (
            Unusable::UnstatedCrossing {
                what: "a work-queue entry",
                realm: "beta".to_string(),
                crossing: "ghost.api".to_string(),
            },
            "a work-queue entry cites crossing 'ghost.api' in realm 'beta', which the \
             dossier does not state as a finding",
        ),
        (
            Unusable::NotAdmitted {
                what: "a parked-run proposal",
                command: "ship".to_string(),
                run_id: "parked-run".to_string(),
            },
            "a parked-run proposal suggests 'ship' for parked-run, which is not an operator \
             command the dossier states for it",
        ),
        (
            Unusable::from(settled),
            "a work-queue entry cites landed-run, which ended withdrawn: a settled run is \
             never offered as work",
        ),
    ] {
        assert_eq!(refusal.to_string(), text);
    }
}
