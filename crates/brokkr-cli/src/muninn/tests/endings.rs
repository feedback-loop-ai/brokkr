//! A work-queue entry read by its run's own ending (decision 0064 ruling
//! 6), where the run's id is held by more than one hearth (decision 0026
//! rulings 3 and 5). A child of the owning `tests.rs`, which sits near its
//! line ceiling.

use super::*;

use super::queue::{offer, Offer, Refusal};

/// The citation `validate` resolves for the head of `run_id` in `realm`.
fn head(dossier: &Dossier, realm: &str, run_id: &str) -> Cited {
    let runs = dossier.value["runs"].as_array().unwrap();
    let row = runs
        .iter()
        .find(|row| row["realm"] == realm && row["run_id"] == run_id)
        .unwrap();
    let entry = json!({"realm": realm, "run_id": run_id, "seq": row["seq"]});
    citation("a work-queue entry", dossier, &entry).unwrap()
}

/// What `validate` says of a report queueing `cited` alone.
fn queued(dossier: &Dossier, cited: &Cited) -> Result<Report, String> {
    let (realm, run_id, seq) = cited;
    let entry = json!({"realm": realm, "run_id": run_id, "seq": seq, "finding": "f",
                       "reasoning": "r"});
    validate(
        dossier,
        &json!({"result": "proposed", "inputs": {"fleet_summary": "s",
                "parked_runs": [], "work_queue": [entry]}}),
    )
}

fn settled(run_id: &str, ending: Ending) -> Refusal {
    Refusal::Settled {
        run_id: run_id.to_string(),
        ending,
    }
}

/// The refusal's text, pinned once: the tests above it match the variant.
#[test]
fn a_settled_refusal_says_which_run_and_how_it_ended() {
    assert_eq!(
        settled("landed-run", Ending::DeliveredElsewhere).to_string(),
        "a work-queue entry cites landed-run, which ended delivered-elsewhere: a settled run \
         is never offered as work"
    );
}

/// Alpha's `shared-id` shipped at seq 8; beta's withdrew at seq 5. The beta
/// citation is beta's run, and it is refused as withdrawn, never offered
/// with alpha's `shipped`.
#[test]
fn chief_settled_citation_uses_its_own_hearth_ending() {
    let alpha = Fleet::new();
    alpha.completed("shared-id");
    let beta = Fleet::new();
    beta.stopped_with("shared-id", json!({"cause": "withdrawn"}));
    let dossier = many_hearth_dossier(&alpha, &beta);
    let cited = head(&dossier, "beta", "shared-id");
    assert_eq!(cited, (Some("beta".into()), "shared-id".into(), 5));
    let refusal = settled("shared-id", Ending::Withdrawn);
    assert_eq!(offer(&dossier, &cited), Err(refusal.clone()));
    assert_eq!(queued(&dossier, &cited).err(), Some(refusal.to_string()));
    let shipped = head(&dossier, "alpha", "shared-id");
    assert_eq!(offer(&dossier, &shipped), Ok(None));
}

/// The settled run in the hearth the map names first, a shipped one of
/// the same id in the second: each citation is read as its own, so the
/// settled one is refused and the shipped one is queued.
#[test]
fn a_settled_run_in_the_first_hearth_never_refuses_the_seconds_shipped_run() {
    let alpha = Fleet::new();
    let beta = Fleet::new();
    let settled_by = [
        (
            "superseded-id",
            json!({"cause": "superseded", "by": {"realm": null, "run_id": "x"}}),
        ),
        ("withdrawn-id", json!({"cause": "withdrawn"})),
        (
            "landed-id",
            json!({"cause": "delivered-elsewhere", "ref": "abc123"}),
        ),
    ];
    for (run_id, cause) in &settled_by {
        alpha.stopped_with(run_id, cause.clone());
        beta.completed(run_id);
    }
    let dossier = many_hearth_dossier(&alpha, &beta);
    for (run_id, ending) in [
        ("superseded-id", Ending::Superseded),
        ("withdrawn-id", Ending::Withdrawn),
        ("landed-id", Ending::DeliveredElsewhere),
    ] {
        let first = head(&dossier, "alpha", run_id);
        assert_eq!(offer(&dossier, &first), Err(settled(run_id, ending)));
        let second = head(&dossier, "beta", run_id);
        assert_eq!(offer(&dossier, &second), Ok(None));
        assert_eq!(queued(&dossier, &second).unwrap().work_queue.len(), 1);
    }
}

/// A live run in the first hearth has no ending, and borrows none from the
/// second hearth's run of its id: it is queued as nothing more, while the
/// second's `infrastructure` run is offered as a retry and its `blocked`
/// run as an unblock.
#[test]
fn a_live_run_in_the_first_hearth_takes_no_ending_from_the_second() {
    let alpha = Fleet::new();
    alpha.parked_before_any_phase("retry-id");
    alpha.parked_before_any_phase("unblock-id");
    let beta = Fleet::new();
    beta.stopped_with("retry-id", json!({"cause": "infrastructure"}));
    beta.blocked("unblock-id");
    let dossier = many_hearth_dossier(&alpha, &beta);
    for (run_id, offered) in [("retry-id", Offer::Retry), ("unblock-id", Offer::Unblock)] {
        let live = head(&dossier, "alpha", run_id);
        assert_eq!(offer(&dossier, &live), Ok(None));
        let ended = head(&dossier, "beta", run_id);
        assert_eq!(offer(&dossier, &ended), Ok(Some(offered)));
    }
}

/// A recorded `offered_as` is read back from a file anyone may have edited:
/// its escape, carriage return and reordering override do not survive, and
/// the line keeps its shape.
#[test]
fn e6_chief_recorded_offer_preserves_layout_and_sanitizes_its_field() {
    let hostile = format!("re{}[2Jtry\r{}", '\x1b', char::from_u32(0x202E).unwrap());
    let rendered = render(&json!({"work_queue": [{"run_id": "outage-run", "seq": 5,
        "offered_as": hostile, "finding": "f", "reasoning": "r"}]}));
    assert_eq!(
        rendered,
        "— · 0 proposals for parked runs · 1 findings queued\n  summary: —\n  queue \
         outage-run seq 5 · offered as re[2Jtry · f · r\n  cites: \n"
    );
}
