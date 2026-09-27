use super::*;
use crate::adapters::AdapterKind;

/// An environment holding exactly `set`.
fn environment<'a>(set: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<OsString> + 'a {
    move |name| {
        set.iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| OsString::from(value))
    }
}

#[test]
fn every_override_pairs_its_name_with_the_one_it_retired() {
    let all = [
        (Override::ClaudeBin, "CLAUDE_BIN"),
        (Override::LanetallyBin, "LANETALLY_BIN"),
        (Override::CodexBin, "CODEX_BIN"),
        (Override::DshBin, "DSH_BIN"),
        (Override::ExecName, "EXEC_NAME"),
        (Override::BrowserBin, "BROWSER_BIN"),
    ];
    for (what, stem) in all {
        assert_eq!(what.name(), format!("BROKKR_{stem}"));
        assert_eq!(what.retired(), format!("{}{stem}", concat!("FOR", "GE_")));
    }
    let drivers = [
        AdapterKind::Claude,
        AdapterKind::Lanetally,
        AdapterKind::Codex,
        AdapterKind::Dsh,
        AdapterKind::Exec,
    ]
    .map(Override::of_driver);
    assert_eq!(drivers, all.map(|(what, _)| what)[..5]);
}

#[test]
fn a_retired_spelling_alone_is_refused_by_both_names() {
    let retired = Override::ClaudeBin.retired();
    let refused = read_with(
        Override::ClaudeBin,
        environment(&[(&retired, "/pinned/claude")]),
    );
    assert_eq!(
        refused,
        Err(OverrideError::Retired {
            retired: retired.clone(),
            current: "BROKKR_CLAUDE_BIN",
        })
    );
    assert_eq!(
        refused.unwrap_err().to_string(),
        format!(
            "{retired} is set but is no longer read: it was renamed BROKKR_CLAUDE_BIN; \
             set BROKKR_CLAUDE_BIN instead, or unset {retired}"
        )
    );
    // Set but empty is still set: the operator's intent is a pin.
    assert_eq!(
        read_with(Override::ClaudeBin, environment(&[(&retired, "")])),
        Err(OverrideError::Retired {
            retired,
            current: "BROKKR_CLAUDE_BIN",
        })
    );
}

#[test]
fn the_current_name_answers_and_absence_reads_as_unset() {
    let retired = Override::DshBin.retired();
    assert_eq!(
        read_with(
            Override::DshBin,
            environment(&[("BROKKR_DSH_BIN", "/new/dsh"), (&retired, "/old/dsh")])
        ),
        Ok(Some("/new/dsh".to_string()))
    );
    assert_eq!(read_with(Override::DshBin, environment(&[])), Ok(None));
    // Another override's retired spelling is not this one's.
    let codex = Override::CodexBin.retired();
    assert_eq!(
        read_with(Override::DshBin, environment(&[(&codex, "x")])),
        Ok(None)
    );
}
