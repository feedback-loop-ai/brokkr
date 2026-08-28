use super::*;

#[test]
fn report_and_tool_probe_expose_all_health_states() {
    let mut report = Report {
        healthy: true,
        lines: Vec::new(),
    };
    report.ok("present", "yes".into());
    report.warn("optional", "absent".into());
    report.missing("required", "absent".into());
    assert!(!report.healthy);
    let rendered = report.render();
    assert!(rendered.contains("ok       present: yes"));
    assert!(rendered.contains("warn     optional: absent"));
    assert!(rendered.contains("MISSING  required: absent"));

    assert_eq!(tool_version("forge-certainly-does-not-exist"), None);
    assert_eq!(tool_version("false"), None);
    assert!(tool_version("true").is_some());
}

#[test]
fn doctor_marks_an_unopenable_database_missing() {
    let dir = tempfile::tempdir().unwrap();
    fn always_missing(_: &str) -> Option<String> {
        None
    }
    let report = doctor_with_probe(None, dir.path(), always_missing);
    assert!(!report.healthy);
    assert!(report.render().contains("MISSING  git"));
    assert!(report.render().contains("warn     claude"));
    assert!(report.render().contains("MISSING  database"));
}

#[test]
fn doctor_advises_on_a_missing_lanetally_wrapper_without_failing() {
    let dir = tempfile::tempdir().unwrap();
    fn always_missing(_: &str) -> Option<String> {
        None
    }
    let report = doctor_with_probe(None, dir.path(), always_missing);
    let rendered = report.render();
    // Missing wrapper: a warning naming the expected install path and
    // the env override — never a hard failure.
    assert!(
        !rendered.contains("MISSING  claude-lanetally"),
        "{rendered}"
    );
    assert!(
        rendered.contains(
            "warn     claude-lanetally: not found — seats using the \
             claude-lanetally driver will fail to spawn"
        ),
        "{rendered}"
    );
    assert!(
        rendered.contains("~/.local/bin/claude-lanetally"),
        "{rendered}"
    );
    assert!(rendered.contains("FORGE_LANETALLY_BIN"), "{rendered}");
    // The four pre-existing warning strings stay byte-identical.
    for line in [
        "warn     claude: not found — seats using the claude-code driver will fail to spawn",
        "warn     codex: not found — seats using the codex driver will fail to spawn",
        "warn     dsh: not found — seats using the deepseek-harness driver will fail to spawn",
        "warn     python3: not found — seats using the exec (script templates) driver \
         will fail to spawn",
    ] {
        assert!(rendered.contains(line), "{rendered}");
    }

    fn always_present(_: &str) -> Option<String> {
        Some("1.0.0".into())
    }
    let report = doctor_with_probe(None, dir.path(), always_present);
    assert!(
        report.render().contains("ok       claude-lanetally: 1.0.0"),
        "{}",
        report.render()
    );
}
