//! Every workspace box built from the reach sealed at admission (decision
//! 0065 slice two, U8a2c; CR2, MB3): the engine composes each box of an
//! attempt from the reach its admission sealed for those hands, carries it
//! to `hands serve` and `hands exec` as a typed fact, writes it beside the
//! artifact root for the driver's final check, and refuses a box whose
//! hands were sealed no reach.

use std::sync::LazyLock;

use brokkr_protocol::hands::Sealed;

use super::*;
pub(in crate::engine) use crate::argv::tests::ENGINE;
use crate::argv::Executable;

/// The reach the argv fixtures' boxes are composed with: the one that
/// seals nothing, which hands builds no box from, so only the argv tests
/// read it.
static REACH: LazyLock<Sealed> = LazyLock::new(Sealed::default);

/// [`REACH`] as the exec argv and the hands server carry it.
pub(in crate::engine) const REACH_ARG: &str =
    "{\"home\":\"\",\"worktree\":{\"path\":\"\",\"identity\":null},\"git\":null,\"binds\":[],\"store\":null}";

/// What a box is handed in the argv fixtures: the test binary's own
/// [`ENGINE`], `artifacts` and [`REACH`].
pub(in crate::engine) fn boxed(
    artifacts: &Path,
) -> (&'static Executable, &Path, Option<&'static Sealed>) {
    (&ENGINE, artifacts, Some(&REACH))
}

/// [`boxed`] at the fixtures' artifact root, `/a`.
pub(in crate::engine) fn boxed_a() -> (&'static Executable, &'static Path, Option<&'static Sealed>)
{
    boxed(Path::new("/a"))
}

/// Whether this test already runs in a box, whose namespace a spawn proof
/// cannot nest: it is skipped then, and a host that declared it must
/// produce boundary evidence fails instead of printing `ok` (decision 0054
/// ruling 9).
pub(in crate::engine) fn already_boxed() -> bool {
    let boxed = std::env::var_os(brokkr_protocol::hands::HANDS_BOX_ENV).is_some();
    if boxed {
        brokkr_protocol::hands::skip_boundary_proof(
            brokkr_protocol::hands::boundary_evidence_required(),
            "this environment is already a box",
        );
    }
    boxed
}

/// The bwrap this host builds a real namespace with; none where it cannot,
/// or where this test already runs in a box, the proof's skip logged, or
/// failed on a host that declared it must produce boundary evidence.
#[cfg(target_os = "linux")]
pub(in crate::engine) fn namespace_bwrap() -> Option<PathBuf> {
    let probe = brokkr_protocol::hands::require_bwrap()
        .ok()
        .filter(|bwrap| {
            Command::new(bwrap)
                .args(["--ro-bind", "/", "/", "--", "true"])
                .stderr(std::process::Stdio::null())
                .status()
                .is_ok_and(|status| status.success())
        });
    let found = probe.filter(|_| std::env::var_os(brokkr_protocol::hands::HANDS_BOX_ENV).is_none());
    if found.is_none() {
        brokkr_protocol::hands::skip_boundary_proof(
            brokkr_protocol::hands::boundary_evidence_required(),
            "this environment cannot create a bubblewrap namespace",
        );
    }
    found
}

/// The engine executable every box, harness fragment and dialect step is
/// composed with (#349, ask 8).
mod executable_tests;

/// The secrets store the engine makes before a boxed launch and its box
/// hides (U8h).
mod store_tests;

/// A seat taken from the folder, walked at its spawn (decision 0091).
mod folder_tests;

/// A seat, member or step without hands is sealed no reach and needs none:
/// it composes its own argv untouched under every boundary. Moved here
/// from the parent suite, which sits at its line baseline, by U8a2c.
#[test]
fn retiring_confine_leaves_plain_seat_member_and_step_argv_untouched() {
    let command: Vec<String> = [
        "driver",
        "--model",
        "m",
        "--effort",
        "high",
        "{prompt_file}",
    ]
    .into_iter()
    .map(str::to_string)
    .collect();
    let (_dir, mut engine) = crate::engine::tests::engine(single_body(command.clone()));
    for boundary in brokkr_core::realms::BOUNDARIES {
        engine.boundary = boundary;
        let single = engine.compose("attempt", false, command.clone(), None, None, "result.json");
        assert_eq!(
            single,
            SiteSpawn {
                class: Some(SeatClass::Work),
                ..SiteSpawn::inherit(command.clone())
            }
        );
        // The member composer is shared by a panel and a panel step.
        for (site, prefix) in [("work", ""), ("work:check", "check:")] {
            let runs = engine.member_runs(
                "attempt",
                site,
                &[member("judge", command.clone())],
                &json!({"judge":{"role_path":"role.md","result_path":"result.json"}}),
                &json!({}),
                &json!({}),
                &Selection::new(),
                prefix,
                false,
            );
            assert_eq!(runs[0].spawn, single);
        }
    }
}
