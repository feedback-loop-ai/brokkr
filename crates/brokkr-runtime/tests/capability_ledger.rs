//! Decision 0065 slice two, U8a2a (CR2, MB3; D6): where the public
//! engine's HOME cannot be protected, nothing is recorded in it. A
//! production resolution proof, run again in a child whose HOME is a fresh
//! owner-only directory of its own ([`crate::own_home`]), which it then
//! makes writable by its group; it runs on any TMPDIR and is never skipped.
//! The durable reservation's retention through engine death, on an
//! injected root, is
//! `engine::broker::admission::tests::processes::an_engine_killed_mid_attempt_keeps_its_worktree_reserved`
//! (M4 of run four's second review), which this suite held until then; a
//! server's sources' retention through a real dispatched engine's death
//! (U8a2b), its seat or its real server box alive, is
//! `engine::capability_tests::provision_tests::brokered_tests::writers_tests::a_dispatched_engine_killed_while_its_seat_lives_keeps_its_servers_sources`
//! and `…::a_dispatched_engine_killed_while_its_server_box_lives_keeps_its_sources`,
//! and through uncertain settlement
//! `engine::broker::admission::tests::sources::a_free_lock_or_an_uncertain_settlement_releases_no_source`.

use std::os::unix::fs::PermissionsExt;

use crate::capability_broker_launch::{driven, home};

/// A HOME its group may write cannot be protected, and a run that puts
/// nothing at stake runs holding no lease (C1), whichever way the fence
/// stands (the operator's ruling of 2026-10-29 on U9b1's fourth run): its
/// driver runs, and the engine makes nothing under that HOME's state.
#[test]
fn an_engine_whose_home_cannot_be_protected_records_nothing_in_it() {
    let test = "capability_ledger::an_engine_whose_home_cannot_be_protected_records_nothing_in_it";
    if !crate::own_home::in_own_home(test) {
        return;
    }
    let home = home();
    std::fs::set_permissions(&home, std::fs::Permissions::from_mode(0o770)).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let work = std::fs::canonicalize(dir.path()).unwrap();
    let ran = work.join("ran");
    driven(&work, &format!("touch '{}'; exit 1", ran.display()));
    assert!(ran.exists(), "its driver ran");
    assert!(!home.join(".local").exists(), "nothing made in its HOME");
}
