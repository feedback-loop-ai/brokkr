//! The follow-ups of #1052 part 1 (#1056): the 0.2.2 judge refuses a
//! common hands row left enabled, directly and by its id, and the two
//! version tables name the same measured releases both ways.

use super::super::composition::{judge, rows, Composition};
use super::super::{Release, MEASURED_VERSIONS};
#[cfg(unix)]
use super::measured_dump_0_2_2;

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
