//! Decision 0104 rulings 1 and 4: doctor's floor lines and its egress
//! mismatch line. Each test plants the host file and the adapters, reads
//! the report, and refuses nothing.

use super::*;

fn workspace() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A temp root whose `adapters/` is the shipped dsh adapter: its routes
/// `spark` and `spark-glm` are both egress `local`, so the host file's
/// class for each is the only thing the mismatch line reads.
fn with_dsh_adapters() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("adapters")).unwrap();
    std::fs::copy(
        workspace().join("adapters/dsh.json"),
        dir.path().join("adapters/dsh.json"),
    )
    .unwrap();
    dir
}

fn host(dir: &std::path::Path, text: &str) -> std::path::PathBuf {
    let file = dir.join("host.json");
    std::fs::write(&file, text).unwrap();
    file
}

fn v3(dir: &std::path::Path) -> std::path::PathBuf {
    host(
        dir,
        r#"{"schema":"forge.host/v3","providers":{"dsh":{"ceiling":1,"routes":{"spark":{"ceiling":1,"class":"cloud"},"spark-glm":{"ceiling":1,"class":"shared-local","timeout_floor_seconds":7200}}}},"floors":[{"place":"temp","floor_bytes":1}],"boxed_builds":{"ceiling":1}}"#,
    )
}

fn reporting() -> Report {
    Report {
        healthy: true,
        lines: Vec::new(),
    }
}

/// The report of the host file `file` over the shipped dsh adapters, its
/// health asserted on the way.
fn rendered(dir: &tempfile::TempDir, file: &std::path::Path) -> String {
    let mut report = reporting();
    report_at(&mut report, &dir.path().join("adapters"), file);
    assert!(report.healthy, "{}", report.render());
    report.render()
}

/// The one mismatch line `spark` earns: dsh declares it local and this
/// host file declares it cloud.
const MISMATCH: &str = "warn     egress: route 'spark' of provider 'dsh' declares egress local, \
                        and the host file does not declare it shared-local: its seats keep their \
                        authored deadlines and count against their provider";

#[test]
fn a_declared_floor_is_one_ok_line_and_a_cloud_class_route_is_one_warning() {
    let dir = with_dsh_adapters();
    let mut report = reporting();
    report_at(&mut report, &dir.path().join("adapters"), &v3(dir.path()));
    assert!(report.healthy, "{}", report.render());
    assert_eq!(
        report.render(),
        format!(
            "ok       floor: dsh/spark-glm 7200 seconds on shared-local hardware — seats on this \
             route run under at least this deadline\n{MISMATCH}"
        )
    );
}

#[test]
fn a_shared_local_route_without_a_floor_gets_no_floor_line_and_no_mismatch() {
    let dir = with_dsh_adapters();
    let file = host(
        dir.path(),
        r#"{"schema":"forge.host/v3","providers":{"dsh":{"ceiling":1,"routes":{"spark":{"ceiling":1,"class":"cloud"},"spark-glm":{"ceiling":1,"class":"shared-local"}}}},"floors":[{"place":"temp","floor_bytes":1}],"boxed_builds":{"ceiling":1}}"#,
    );
    let rendered = rendered(&dir, &file);
    assert!(!rendered.contains("floor:"), "{rendered}");
    assert_eq!(rendered, MISMATCH);
}

#[test]
fn an_absent_host_file_names_every_local_route_and_declares_no_floor() {
    let dir = with_dsh_adapters();
    let rendered = rendered(&dir, &dir.path().join("no-host.json"));
    assert!(!rendered.contains("floor:"), "{rendered}");
    assert_eq!(rendered.lines().count(), 2, "{rendered}");
    for route in ["spark", "spark-glm"] {
        assert!(
            rendered.contains(&format!("route '{route}' of provider 'dsh'")),
            "{rendered}"
        );
    }
}

#[test]
fn a_host_file_that_cannot_be_read_is_one_warning_and_no_mismatch() {
    let dir = with_dsh_adapters();
    let rendered = rendered(&dir, dir.path());
    assert!(
        rendered.starts_with("warn     host: the host configuration at "),
        "{rendered}"
    );
    assert!(!rendered.contains("egress:"), "{rendered}");
}

#[test]
fn an_invalid_host_file_is_one_warning_and_no_mismatch() {
    let dir = with_dsh_adapters();
    let file = host(dir.path(), "not json");
    let rendered = rendered(&dir, &file);
    assert!(
        rendered.contains("is not a valid host configuration file"),
        "{rendered}"
    );
    assert!(!rendered.contains("egress:"), "{rendered}");
}

#[test]
fn an_unplaced_host_file_is_one_warning_and_an_unreadable_adapters_root_is_silent() {
    let dir = with_dsh_adapters();
    let mut unplaced = reporting();
    report_from(&mut unplaced, &dir.path().join("adapters"), None, None);
    assert!(unplaced.healthy);
    assert_eq!(
        unplaced.render(),
        "warn     host: no host configuration can be found: neither XDG_CONFIG_HOME nor HOME \
         names an absolute directory"
    );
    let mut no_adapters = reporting();
    report_at(
        &mut no_adapters,
        &dir.path().join("no-adapters"),
        &v3(dir.path()),
    );
    assert!(no_adapters.healthy);
    assert_eq!(no_adapters.render(), "");
}

#[test]
fn a_route_that_is_not_egress_local_gets_no_mismatch() {
    let dir = with_dsh_adapters();
    let path = dir.path().join("adapters/dsh.json");
    let mut adapter: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    adapter["routes"]["cloud"] = serde_json::json!("contracted");
    std::fs::write(&path, adapter.to_string()).unwrap();
    let rendered = rendered(&dir, &v3(dir.path()));
    assert!(!rendered.contains("route 'cloud'"), "{rendered}");
    assert_eq!(rendered.lines().count(), 2, "{rendered}");
}
