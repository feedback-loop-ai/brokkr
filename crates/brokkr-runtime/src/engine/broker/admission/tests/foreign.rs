//! Another build's reservation (decision 0090; U8a2a; D6, MB3): a run's
//! record this build cannot read whole, written by hand from synthetic
//! paths under each test's own HOME in the older format, whose boxed reach
//! seals a list of `{path, identity}` (U8a2a, before U8a2c sealed it as
//! one record). It is read through the view every build writes: admitted
//! where its writers are kept from what this run protects, refused by its
//! run where one is not, refused by name where even the view cannot read
//! it, and never sealed again. This run's own reservation is read whole.
//!
//! A boxed writer admitted beside evidence passes the mount table's
//! judgement, which only Linux reads (decision 0063), so the tests that
//! admit one are Linux only.

use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;

use serde_json::Value;

use super::*;
use crate::capabilities::McpFence;

/// The run another build left standing on `worktree`: its one attempt
/// `f1` protecting `protects`, its cleanup witnessed as `cleanup`, its one
/// `work` site reaching `reach`, as that build spells each.
fn older(worktree: &Path, run: &str, [protects, cleanup]: [&str; 2], reach: Value) -> Value {
    json!({
        "repo": canonical::sha256_bytes(worktree.as_os_str().as_bytes()),
        "run": run,
        "workdir": worktree,
        "artifacts": worktree.join(".forge/artifacts"),
        "journal": worktree.join("forge.db"),
        "attempts": {
            "f1": {
                "protects": protects,
                "writers": [{"site": "work", "reach": reach}],
                "cleanup": cleanup,
            }
        },
        "settled": [],
    })
}

/// A box writing `roots`, binding them apart, its reach sealed as the
/// older format seals it: a list of each path and its identity.
fn boxed_to(roots: &[&Path]) -> Value {
    let sealed: Vec<Value> = (roots.iter())
        .map(|root| json!({"path": root, "identity": [1, 2]}))
        .collect();
    json!({"boxed": {"writable": roots, "readable": [], "sources": roots, "sealed": sealed}})
}

/// `bytes` planted as the reservation of the run `run` of `worktree` in
/// `host`'s root, its directories and file this user's alone; where.
fn plant(host: &Host, worktree: &Path, run: &str, bytes: &[u8]) -> PathBuf {
    let at = host.reservation(worktree, run);
    let mut dirs = std::fs::DirBuilder::new();
    dirs.recursive(true).mode(0o700);
    dirs.create(at.parent().unwrap()).unwrap();
    let mut file = std::fs::OpenOptions::new();
    let file = file.write(true).create(true).truncate(true).mode(0o600);
    file.open(&at).unwrap().write_all(bytes).unwrap();
    at
}

/// [`plant`] of `record`.
fn planted(host: &Host, worktree: &Path, record: &Value) -> PathBuf {
    let run = record["run"].as_str().unwrap();
    plant(host, worktree, run, &serde_json::to_vec(record).unwrap())
}

/// The attempt `attempt` of `run-a` on `work`, protecting `protects`, its
/// one `work` site boxed to `work`.
fn kept(work: &Path, attempt: &str, protects: Protects) -> Claim {
    claim(
        work,
        ["run-a", attempt],
        protects,
        &[boxed("work", &[work])],
    )
}

/// The older format's reservation, its writer kept apart, is admitted
/// beside every claim: with nothing at stake, where it protects evidence,
/// where the claim does, and with the fence lifted; and its bytes are the
/// ones planted after each.
#[cfg(target_os = "linux")]
#[test]
fn a_foreign_reservation_kept_apart_is_admitted() {
    let host = Host::new();
    let (work, lanes) = (host.worktree("work"), host.worktree("lanes"));
    let at = |protects: &str| {
        let record = older(
            &lanes,
            "order-1",
            [protects, "settling"],
            boxed_to(&[&lanes]),
        );
        planted(&host, &lanes, &record)
    };
    let cases = [
        ("nothing", open(&work, ["run-a", "a1"]), false),
        ("evidence", kept(&work, "a2", Protects::Nothing), false),
        ("nothing", kept(&work, "a3", Protects::Evidence), false),
        ("nothing", kept(&work, "a4", Protects::Nothing), true),
    ];
    for (protects, incoming, lifted) in cases {
        let at = at(protects);
        let planted = std::fs::read(&at).unwrap();
        let _lift = lifted.then(McpFence::lift);
        freed(&host.admit(&incoming).unwrap());
        assert_eq!(
            std::fs::read(&at).unwrap(),
            planted,
            "{protects}: never sealed"
        );
    }
}

/// The same reservation whose writer can reach what this run protects is
/// refused by its run: its writable root over the claim's artifact root,
/// or over the coordination root beside evidence; and one outside any box
/// beside evidence. A `protects` this build does not know is read
/// as evidence, so a claim's writer outside any box is refused beside it.
#[cfg(target_os = "linux")]
#[test]
fn a_foreign_writer_reaching_what_this_run_protects_refuses_by_its_run() {
    let host = Host::new();
    let (work, lanes) = (host.worktree("work"), host.worktree("lanes"));
    let record = |protects: &str, reach| older(&lanes, "order-1", [protects, "settling"], reach);
    let cases = [
        (
            record("nothing", boxed_to(&[&lanes, &work])),
            kept(&work, "a1", Protects::Evidence),
            reaches("work", "order-1", &work.join(".forge/artifacts")),
        ),
        (
            record("nothing", boxed_to(&[&lanes, &host.home])),
            kept(&work, "a2", Protects::Evidence),
            reaches("work", "order-1", &host.root()),
        ),
        (
            record("evidence", json!("unbounded")),
            kept(&work, "a3", Protects::Nothing),
            unboxed("work", "order-1"),
        ),
        (
            record("everything", boxed_to(&[&lanes])),
            open(&work, ["run-a", "a4"]),
            unboxed("work", "run-a"),
        ),
    ];
    for (record, incoming, refused) in cases {
        planted(&host, &lanes, &record);
        assert_eq!(host.admit(&incoming), cr2(refused));
    }
}

/// A reservation even the view cannot read refuses by its name, and
/// nothing of this run is sealed: one that is no JSON, one without its
/// attempts, a writer without its writable roots, one without its
/// worktree's key, a reach of a kind this build does not know; and one in
/// a format it does not read, by that format. One naming the format it
/// reads is admitted.
#[test]
fn an_unreadable_foreign_reservation_refuses_by_name() {
    let host = Host::new();
    let (work, lanes) = (host.worktree("work"), host.worktree("lanes"));
    let record = older(
        &lanes,
        "order-1",
        ["nothing", "settling"],
        boxed_to(&[&lanes]),
    );
    let without = |pointer: &str, key: &str| {
        let mut record = record.clone();
        record
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove(key);
        serde_json::to_vec(&record).unwrap()
    };
    let mut sideways = record.clone();
    sideways["attempts"]["f1"]["writers"][0]["reach"] = json!({"sideways": {"writable": []}});
    let mut later = record.clone();
    later["format"] = json!(2);
    let at = host.reservation(&lanes, "order-1");
    let participant = Unknown::Participant(at.clone());
    let cases = [
        (b"not json".to_vec(), participant.clone()),
        (without("", "attempts"), participant.clone()),
        (
            without("/attempts/f1/writers/0/reach/boxed", "writable"),
            participant.clone(),
        ),
        (without("", "repo"), participant.clone()),
        (serde_json::to_vec(&sideways).unwrap(), participant),
        (
            serde_json::to_vec(&later).unwrap(),
            Unknown::Formatted(at, 2),
        ),
    ];
    for (bytes, refused) in cases {
        plant(&host, &lanes, "order-1", &bytes);
        assert_eq!(host.admit(&open(&work, ["run-a", "a1"])), mb3(refused));
        assert!(!host.reservation(&work, "run-a").exists(), "nothing sealed");
    }
    later["format"] = json!(1);
    planted(&host, &lanes, &later);
    freed(&host.admit(&open(&work, ["run-a", "a1"])).unwrap());
    let named = Unknown::Formatted(PathBuf::from("/r"), 2).to_string();
    let said = "the protected inventory holds a reservation at '/r' in format 2, which this build \
                does not read: conclude its run with the build that wrote it, or use a build that \
                reads it";
    assert_eq!(named, said);
}

/// This run's own reservation is read whole: as this build seals it but
/// naming a format, or in the older format, it refuses by name; the older
/// one as another run's is read through the view, and holds its worktree.
#[test]
fn this_runs_own_reservation_is_read_whole() {
    let host = Host::new();
    let work = host.worktree("work");
    freed(&host.admit(&kept(&work, "a1", Protects::Nothing)).unwrap());
    let own = host.reservation(&work, "run-a");
    let mut named: Value = serde_json::from_slice(&std::fs::read(&own).unwrap()).unwrap();
    named["format"] = json!(1);
    planted(&host, &work, &named);
    let refused = mb3(Unknown::Participant(own.clone()));
    assert_eq!(host.admit(&open(&work, ["run-a", "a2"])), refused);
    let record = |run| older(&work, run, ["nothing", "settling"], boxed_to(&[&work]));
    planted(&host, &work, &record("run-a"));
    assert_eq!(host.admit(&open(&work, ["run-a", "a2"])), refused);
    std::fs::remove_dir_all(own.parent().unwrap()).unwrap();
    planted(&host, &work, &record("run-b"));
    let held = cr2(Conflict::Held("run-b".into()));
    assert_eq!(host.admit(&open(&work, ["run-a", "a2"])), held);
}

/// An attempt of another build's reservation whose durable facts prove it
/// settled — its positive fact, and a certified end in a journal no
/// participant's writer can reach — is released for this admission alone
/// and never sealed again; one whose cleanup is still settling, or names a
/// witness this build does not know, holds its worktree.
#[test]
fn a_foreign_reservation_proven_settled_is_released_but_never_sealed() {
    let host = Host::new();
    let (work, journals) = (host.worktree("work"), host.worktree("journals"));
    let journal = journals.join("forge.db");
    let mut store = brokkr_store::Store::open(&journal).unwrap();
    store.create_run("order-1", "f", "b", &json!({})).unwrap();
    let ended = json!({"effect_id": "e", "attempt_id": "f1", "result": {"result": "x"}});
    let id = Some("f1".to_string());
    (store.append_next("order-1", EventType::EffectSucceeded, ended, None, id)).unwrap();
    let cases = [
        ("settled", None),
        ("settling", Some(Conflict::Held("order-1".into()))),
        ("released", Some(Conflict::Held("order-1".into()))),
    ];
    for (index, (cleanup, refused)) in cases.into_iter().enumerate() {
        let mut record = older(&work, "order-1", ["nothing", cleanup], boxed_to(&[&work]));
        record["journal"] = json!(journal);
        let at = planted(&host, &work, &record);
        let planted = std::fs::read(&at).unwrap();
        let incoming = kept(&work, &format!("a{index}"), Protects::Nothing);
        match refused {
            None => freed(&host.admit(&incoming).unwrap()),
            Some(conflict) => assert_eq!(host.admit(&incoming), cr2(conflict)),
        }
        assert_eq!(
            std::fs::read(&at).unwrap(),
            planted,
            "{cleanup}: never sealed"
        );
    }
}
