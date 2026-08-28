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
