use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

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
        (Override::ClaudeBin, "CLAUDE_BIN", "claude"),
        (Override::LanetallyBin, "LANETALLY_BIN", "claude-lanetally"),
        (Override::CodexBin, "CODEX_BIN", "codex"),
        (Override::DshBin, "DSH_BIN", "dsh"),
        (Override::ExecName, "EXEC_NAME", "exec"),
        (Override::BrowserBin, "BROWSER_BIN", "xdg-open"),
    ];
    for (what, stem, fallback) in all {
        assert_eq!(what.name(), format!("BROKKR_{stem}"));
        assert_eq!(
            what.retired(),
            Some(format!("{}{stem}", concat!("FOR", "GE_")))
        );
        assert_eq!(read_with(what, environment(&[])), Ok(fallback.to_string()));
    }
    // The runner was only ever read by its current name: it retired no
    // spelling, so the prefixed one is nobody's and refuses nothing.
    let runner = Override::DshRunner;
    assert_eq!(runner.name(), "BROKKR_DSH_RUNNER");
    assert_eq!(runner.retired(), None);
    let never = format!("{}DSH_RUNNER", concat!("FOR", "GE_"));
    assert_eq!(
        read_set_with(runner, environment(&[(&never, "/old/brokkr")])),
        Ok(None)
    );
    assert_eq!(
        read_with(runner, environment(&[])),
        Ok("brokkr".to_string())
    );
    let drivers = [
        AdapterKind::Claude,
        AdapterKind::Lanetally,
        AdapterKind::Codex,
        AdapterKind::Dsh,
        AdapterKind::Exec,
    ]
    .map(Override::of_driver);
    assert_eq!(drivers, all.map(|(what, _, _)| what)[..5]);
}

#[test]
fn a_retired_spelling_alone_is_refused_by_both_names() {
    let retired = Override::ClaudeBin.retired().unwrap();
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

/// A current value that is not UTF-8 is refused by its name, never read
/// as unset: unset would run the built-in in place of the pin. With the
/// retired spelling also set the refusal is still this one, so it never
/// asks the operator to set a name that is already set.
#[test]
fn a_current_value_that_is_not_unicode_is_refused_by_name() {
    let pinned = OsStr::from_bytes(b"/opt/\xffpinned/claude");
    let retired = Override::ClaudeBin.retired().unwrap();
    for retired_too in [false, true] {
        let refused = read_with(Override::ClaudeBin, |name| {
            if name == "BROKKR_CLAUDE_BIN" {
                Some(pinned.to_os_string())
            } else {
                (retired_too && name == retired).then(|| OsString::from("/old/claude"))
            }
        });
        assert_eq!(
            refused,
            Err(OverrideError::NotUnicode {
                variable: "BROKKR_CLAUDE_BIN"
            }),
            "retired spelling also set: {retired_too}"
        );
        assert_eq!(
            refused.unwrap_err().to_string(),
            "BROKKR_CLAUDE_BIN is set, but its value is not UTF-8, so what it names \
             cannot be read; give BROKKR_CLAUDE_BIN a UTF-8 value, or unset it"
        );
    }
}

#[test]
fn the_current_name_answers_and_absence_reads_as_the_fallback() {
    let retired = Override::DshBin.retired().unwrap();
    assert_eq!(
        read_with(
            Override::DshBin,
            environment(&[("BROKKR_DSH_BIN", "/new/dsh"), (&retired, "/old/dsh")])
        ),
        Ok("/new/dsh".to_string())
    );
    // Another override's retired spelling is not this one's.
    let codex = Override::CodexBin.retired().unwrap();
    assert_eq!(
        read_with(Override::DshBin, environment(&[(&codex, "x")])),
        Ok("dsh".to_string())
    );
}
