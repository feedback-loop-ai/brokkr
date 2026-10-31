//! Decision 0065 slice two, U8a2a (CR2, MB3; D6): the public engine
//! resolves its writer lease's coordination root from its HOME alone,
//! through the protected walk that judges every ancestor, as an installed
//! `brokkr` does. These are the production resolution proofs, each run
//! again in a child whose HOME is a fresh owner-only directory of its own
//! ([`crate::own_home`]), so nothing here reads or writes the host's HOME.
//! The lease's lifecycle across real engines — an already-running writer,
//! two runs started together, an engine killed mid-attempt — is proved on
//! an injected root, on any TMPDIR and never skipped, by
//! `engine::broker::admission::tests::processes` (M4 of run four's second
//! review), which these suites held until then. U8a2b's exclusion of every
//! managed writer from an admitted server's program and system sources and
//! from every link on their routes — server first, writer first,
//! simultaneous starts, a dispatched engine killed with its seat or its
//! server box alive, an uncertain settlement, a disjoint worktree, binds
//! the mount table shows — needs a server provisioned under a held lease,
//! which no public engine can do while the compile fence stands, and the
//! library's own seams (an injected root and mount table, seal hooks), so
//! it is proved in `engine::broker::admission::tests::sources` and, through
//! dispatch and provisioning, in
//! `engine::capability_tests::provision_tests::brokered_tests::writers_tests`.
//! So are the operator's 2026-10-19 rulings on its host inputs (D6): a
//! writer at, above or below an operating-system root, and one able to make
//! a dangling launcher or launch candidate live, refused before any survey;
//! and the survey started without the loader's `LD_*`, in
//! `engine::capability_tests::provision_tests::survey_tests`.

use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use brokkr_runtime::{Bundle, Engine};
use brokkr_store::Store;
use serde_json::json;

/// A `work` seat, the protected review, then done.
const POLICY: &str = r#"{"phases": ["work", "review", "done"], "initial": "work",
  "terminal": ["done"],
  "rules": [{"id": "WORK-REVIEW", "from": "work", "result": "complete", "next": "review",
             "reason": "worked"},
            {"id": "REVIEW-DONE", "from": "review", "result": "clean", "next": "done",
             "reason": "clean"}]}"#;

/// Where a HOME whose every ancestor protects resolves it, a dispatch holds
/// its lease in that HOME's own coordination root while its driver runs:
/// the driver finds its run's reservation there. Skipped, with its reason,
/// where this host has no location that protects a HOME: the one test here
/// that needs one.
#[test]
fn an_engine_holds_its_lease_in_the_root_its_home_resolves() {
    let test = "capability_broker_launch::an_engine_holds_its_lease_in_the_root_its_home_resolves";
    if !crate::own_home::in_own_home(test) {
        return;
    }
    let home = home();
    if !protects(&home) {
        let required = brokkr_protocol::hands::boundary_evidence_required();
        let reason = "no protected location for a HOME (TMPDIR; D6)";
        brokkr_protocol::hands::skip_boundary_proof(required, reason);
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let work = std::fs::canonicalize(dir.path()).unwrap();
    let key = brokkr_core::canonical::sha256_bytes(work.as_os_str().as_encoded_bytes());
    let runs = home.join(".local/state/brokkr/capabilities").join(key);
    let seen = work.join("seen");
    let script = format!(
        "[ -e '{}'/*/reservation.json ] && touch '{}'; exit 1",
        runs.display(),
        seen.display()
    );
    let end = driven(&work, &script);
    assert!(
        seen.exists(),
        "its driver ran under a lease in its HOME's root: {end:?}"
    );
}

/// The HOME this process stands in, canonical.
pub(crate) fn home() -> PathBuf {
    std::fs::canonicalize(std::env::var_os("HOME").unwrap()).unwrap()
}

/// The operator's ruling of 2026-10-29 on U9b1's fourth run, which keeps
/// U8a2a's coordination refusals keyed to evidence at stake past the fence
/// U9b1 lifts on Linux as where it stands: a run whose seats hold nothing,
/// two hands-less offices Brokkr cannot box, puts nothing at stake, so it
/// is admitted and completes.
#[test]
fn a_run_with_nothing_at_stake_admits_its_unboxed_writers() {
    let test = "capability_broker_launch::a_run_with_nothing_at_stake_admits_its_unboxed_writers";
    if !crate::own_home::in_own_home(test) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let work = std::fs::canonicalize(dir.path()).unwrap();
    let answering = crate::office::answering;
    let end = run_of(&work, [answering(":", "complete"), answering(":", "clean")]);
    let ended = (end.run.status(), end.phase.as_deref());
    assert_eq!(
        ended,
        (brokkr_core::fold::Status::Completed, Some("done")),
        "{end:?}"
    );
}

/// A run whose `work` seat's driver runs `script` and fails, driven to its
/// park by the public engine on the worktree `work`, journaled there: the
/// state it ends in.
pub(crate) fn driven(work: &Path, script: &str) -> brokkr_core::fold::RunState {
    let script = ["sh", "-c", script].map(String::from).to_vec();
    run_of(work, [script, vec!["false".into()]])
}

/// The run of `work` and `review` seats hiring offices over the `drivers`
/// given, driven to its end by the public engine on the worktree `work`.
fn run_of(work: &Path, drivers: [Vec<String>; 2]) -> brokkr_core::fold::RunState {
    crate::own_home::required();
    let bundle = work.join("bundle");
    std::fs::create_dir_all(bundle.join("roles")).unwrap();
    std::fs::write(bundle.join("policy.json"), POLICY).unwrap();
    std::fs::write(bundle.join("roles/role.md"), "# role\n").unwrap();
    // Each seat hires an office over its own command (U9b1).
    let limits = json!({"max_attempts": 1, "timeout_seconds": 120});
    for (office, driver) in ["worker", "reviewer"].into_iter().zip(&drivers) {
        let driver: Vec<&str> = driver.iter().map(String::as_str).collect();
        crate::office::hire(work, office, &driver, Some(limits.clone()));
    }
    let seat = |result: &str, office: &str| json!({"results": [result], "agent": office});
    let seats = json!({"work": seat("complete", "worker"), "review": seat("clean", "reviewer")});
    let document = json!({"name": "resolved", "policy": "policy.json", "seats": seats});
    std::fs::write(bundle.join("bundle.json"), document.to_string()).unwrap();
    let (agents, adapters) = (work.join("agents"), work.join("adapters"));
    let bundle = Bundle::compile_with(&bundle, &agents, &adapters).unwrap();
    let store = Store::open(&work.join("forge.db")).unwrap();
    let mut engine = Engine::start(store, bundle, "resolved", Some(work.to_path_buf())).unwrap();
    engine.drive().unwrap().state
}

/// Whether `at` and every ancestor of it protect what lies below them:
/// this user's or root's, writable by no one else unless root-owned and
/// sticky, as the broker judges a HOME's ancestors.
fn protects(at: &Path) -> bool {
    let user = brokkr_protocol::hands::ids().0;
    let judged = |at: &Path| {
        std::fs::metadata(at).is_ok_and(|found| {
            let (owner, mode) = (found.uid(), found.mode());
            let sticky = (owner == 0) & (mode & 0o1000 != 0);
            ((owner == user) | (owner == 0)) & ((mode & 0o022 == 0) | sticky)
        })
    };
    at.ancestors().all(judged)
}
