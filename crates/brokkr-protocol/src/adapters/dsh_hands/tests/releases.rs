//! The follow-ups of #1052 part 1 (#1056): the 0.2.2 judge refuses a
//! common hands row left enabled, directly and by its id, and the two
//! version tables name the same measured releases both ways. And
//! 0.2.2-rc.5 is a measured release with its own composed set and overlay.

use super::super::composition::{
    judge, rows, Composition, ADDED_0_2_2, ADDED_0_2_2_RC5, MEASURED_COMPOSITION, REMOVED_0_2_2,
    REMOVED_0_2_2_RC5,
};
use super::super::{
    hands_rows, measured, route_overlay, Mode, Refusal, Release, MEASURED_VERSIONS,
};
#[cfg(unix)]
use super::{dump_of, measured_dump_0_2_2};
use super::{s, server_row, MEASURED_HEAD};

/// 0.2.2-rc.5's measured 102 rows as a seat's `--dump-config` writes them:
/// 0.2.2-rc.1's set with rc.5's own delta, every pinned row disabled.
#[cfg(unix)]
fn measured_dump_0_2_2_rc5() -> String {
    let mut rows: Vec<(&str, &str)> = MEASURED_COMPOSITION
        .into_iter()
        .filter(|row| !REMOVED_0_2_2.contains(row) && !REMOVED_0_2_2_RC5.contains(row))
        .collect();
    rows.extend(ADDED_0_2_2);
    rows.extend(ADDED_0_2_2_RC5);
    dump_of(Release::V0_2_2Rc5, &rows)
}

/// 0.2.2-rc.5's composed tree is admitted: 0.2.2-rc.1's set plus the two
/// team rows, `deepseek-account` under its new package, and Brokkr's
/// hands row; the team host row stays enabled and its tool row is pinned.
#[cfg(unix)]
#[test]
fn a_0_2_2_rc5_tree_is_admitted() {
    assert_eq!(
        ADDED_0_2_2_RC5,
        [
            ("agent-team", "@deepseek-ai/dsh-agent-team"),
            ("tool-agent-team", "@deepseek-ai/dsh-tool-agent-team"),
            ("deepseek-account", "@deepseek-ai/dsh-deepseek-account"),
        ]
    );
    assert_eq!(
        REMOVED_0_2_2_RC5,
        [(
            "deepseek-account",
            "@deepseek-ai/dsh-deepseek-account-platform"
        )]
    );
    let parsed = rows(measured_dump_0_2_2_rc5().as_bytes()).unwrap();
    assert_eq!(parsed.len(), 102);
    let state = |id: &str| {
        parsed
            .iter()
            .find(|row| row.id == id)
            .map(|row| row.disabled)
    };
    assert_eq!(state("agent-team"), Some(false));
    assert_eq!(state("tool-agent-team"), Some(true));
    assert_eq!(state("tool-executor-ptc"), Some(true));
    assert_eq!(judge(Release::V0_2_2Rc5, &parsed), Ok(()));
}

/// Each release admits only its own set: a 0.2.2-rc.1 tree carrying an
/// rc.5 row is refused naming 0.2.2-rc.1, and an rc.5 tree carrying the
/// old `deepseek-account` package is refused naming 0.2.2-rc.5.
#[cfg(unix)]
#[test]
fn a_0_2_2_rc5_row_is_refused_under_another_release() {
    for (id, name) in ADDED_0_2_2_RC5 {
        let mut dump = measured_dump_0_2_2();
        dump.push_str(&format!("- id: {id}\n  name: '{name}'\n  disabled: true\n"));
        let dump = dump.replace(
            "- id: deepseek-account\n  name: '@deepseek-ai/dsh-deepseek-account-platform'\n",
            "",
        );
        assert_eq!(
            judge(Release::V0_2_2, &rows(dump.as_bytes()).unwrap()).err(),
            Some(Composition::Unmeasured {
                id: id.into(),
                name: name.into(),
                release: "0.2.2-rc.1",
            }),
            "{id}"
        );
    }
    let old = "- id: deepseek-account\n  name: '@deepseek-ai/dsh-deepseek-account-platform'\n";
    let dump = measured_dump_0_2_2_rc5().replace(
        "- id: deepseek-account\n  name: '@deepseek-ai/dsh-deepseek-account'\n",
        old,
    );
    assert_eq!(
        judge(Release::V0_2_2Rc5, &rows(dump.as_bytes()).unwrap()).err(),
        Some(Composition::Unmeasured {
            id: "deepseek-account".into(),
            name: "@deepseek-ai/dsh-deepseek-account-platform".into(),
            release: "0.2.2-rc.5",
        })
    );
}

/// An rc.5 tree whose agent-team tool row is switched on is refused by
/// its id: the overlay pins it, so a profile enabling it cannot survive.
#[cfg(unix)]
#[test]
fn a_0_2_2_rc5_tree_with_the_team_tool_enabled_is_refused() {
    let mut parsed = rows(measured_dump_0_2_2_rc5().as_bytes()).unwrap();
    let team = parsed
        .iter_mut()
        .find(|row| row.id == "tool-agent-team")
        .expect("the measured rc.5 tree names the team tool row");
    team.disabled = false;
    assert_eq!(
        judge(Release::V0_2_2Rc5, &parsed).err(),
        Some(Composition::Enabled {
            id: "tool-agent-team".into(),
        })
    );
}

/// 0.2.2-rc.5 is named whole; its neighbours and the bare release are
/// not, and their refusal names every measured release.
#[test]
fn only_0_2_2_rc5_whole_is_the_new_release() {
    assert_eq!(Release::named("0.2.2-rc.5"), Some(Release::V0_2_2Rc5));
    for version in [
        "0.2.2-rc.2",
        "0.2.2-rc.3",
        "0.2.2-rc.4",
        "0.2.2",
        "0.2.2-rc.50",
    ] {
        assert_eq!(Release::named(version), None, "{version}");
        assert_eq!(
            measured(Some(version.into())),
            Err(Refusal::Unmeasured {
                observed: format!("dsh '{version}'")
            })
        );
    }
    assert_eq!(
        measured(Some("0.2.2-rc.4".into())).unwrap_err().to_string(),
        "refusing to invoke the dsh driver: dsh '0.2.2-rc.4' is not a dsh release its hands rows \
         were measured on (0.2.1-rc.2, 0.2.1-rc.3, 0.2.2-rc.1, 0.2.2-rc.5), and a tool row nobody \
         measured could stay enabled beside the box, so the seat is not served its native tools \
         (#896)"
    );
}

/// The rc.5 hands overlay is pinned byte for byte: the ten common rows,
/// the sandbox, the programmatic executor, then the team tool row and
/// Brokkr's server. The bounded reader requires the team pin on rc.5 and
/// refuses it on rc.1.
#[test]
fn the_0_2_2_rc5_overlay_is_pinned() {
    let args = s(&["hands"]);
    let pins =
        "- id: tool-executor-ptc\n  disabled: true\n- id: tool-agent-team\n  disabled: true\n";
    assert_eq!(
        hands_rows(Release::V0_2_2Rc5, Mode::ReadOnly, "/bin/brokkr", &args).unwrap(),
        format!(
            "{MEASURED_HEAD}read-only\n{pins}{}",
            server_row("/bin/brokkr", &args)
        )
    );
    let rc1 = format!("{MEASURED_HEAD}read-only\n- id: tool-executor-ptc\n  disabled: true\n");
    assert_eq!(
        route_overlay::hands(rc1.as_bytes(), Release::V0_2_2Rc5)
            .unwrap_err()
            .to_string(),
        "refusing to invoke the dsh driver: the hands rows carry no row 'tool-agent-team', which \
         the measured hands set names"
    );
    let rc5 = format!("{MEASURED_HEAD}read-only\n{pins}");
    assert_eq!(
        route_overlay::hands(rc5.as_bytes(), Release::V0_2_2)
            .unwrap_err()
            .to_string(),
        "refusing to invoke the dsh driver: the hands rows carry row 'tool-agent-team' in a shape \
         the measured hands set does not name"
    );
}

/// A 0.2.2-rc.1 tree with one of the ten common hands rows switched on is
/// refused by the judge itself, naming that row: the ten are the rows the
/// release's own overlay pins disabled, not merely a side effect of the
/// fixture's generator.
#[cfg(unix)]
#[test]
fn a_0_2_2_tree_with_a_common_row_enabled_is_refused() {
    for id in ["tool-bash", "tool-fs"] {
        let mut parsed = rows(measured_dump_0_2_2().as_bytes()).unwrap();
        let row = parsed
            .iter_mut()
            .find(|row| row.id == id)
            .expect("the measured 0.2.2 tree names the common row");
        row.disabled = false;
        assert_eq!(
            judge(Release::V0_2_2, &parsed).err(),
            Some(Composition::Enabled { id: id.into() }),
            "{id}"
        );
    }
}

/// The measured-version table and the release variants' own version lists
/// name the same releases both ways: a version added to either table alone
/// is caught here, by name.
#[test]
fn the_version_tables_name_the_same_measured_releases_both_ways() {
    let listed: Vec<&str> = Release::ALL
        .into_iter()
        .flat_map(|release| release.versions().iter().copied())
        .collect();
    for version in &listed {
        assert!(
            MEASURED_VERSIONS.contains(version),
            "{version} is listed by a release but missing from MEASURED_VERSIONS"
        );
    }
    for version in MEASURED_VERSIONS {
        let named = Release::ALL
            .into_iter()
            .filter(|release| release.versions().contains(&version))
            .count();
        assert_eq!(
            named, 1,
            "{version} must be named by exactly one release, not {named}"
        );
    }
    assert_eq!(
        listed.len(),
        MEASURED_VERSIONS.len(),
        "both tables name the same number of versions: {listed:?}"
    );
}
