//! `forge init` scaffolds a bundle that compiles under the
//! constitutional lint; `forge doctor` reports health without executing
//! any agent.

use std::process::Command;

fn forge(args: &[&str], cwd: &std::path::Path) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_forge"))
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn init_scaffolds_a_compiling_bundle_and_refuses_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("bundle");
    let (code, _, stderr) = forge(&["init", bundle.to_str().unwrap()], dir.path());
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert!(stderr.contains("digest"), "stderr: {stderr}");

    // The scaffold passes the same compile gate as any bundle.
    let (code, stdout, stderr) = forge(&["compile", "--bundle", bundle.to_str().unwrap()], dir.path());
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert!(stdout.contains("\"starter\""));

    // The tightened ship taxonomy is present from the start.
    let policy = std::fs::read_to_string(bundle.join("policy.json")).unwrap();
    assert!(policy.contains("SHIP-COMPLETE"));
    assert!(policy.contains("SHIPPED-DIRTY"));

    // Refuses to clobber an existing bundle.
    let (code, _, stderr) = forge(&["init", bundle.to_str().unwrap()], dir.path());
    assert_eq!(code, Some(1));
    assert!(stderr.contains("refusing to overwrite"), "stderr: {stderr}");
}

#[test]
fn doctor_reports_health_and_validates_a_bundle() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("bundle");
    forge(&["init", bundle.to_str().unwrap()], dir.path());

    let db = dir.path().join("forge.db");
    let (code, stdout, _) = forge(
        &[
            "doctor",
            "--bundle",
            bundle.to_str().unwrap(),
            "--db",
            db.to_str().unwrap(),
        ],
        dir.path(),
    );
    // git and python3 exist on dev and CI machines; claude may only warn.
    assert_eq!(code, Some(0), "doctor output: {stdout}");
    assert!(stdout.contains("git"));
    assert!(stdout.contains("bundle"));
    // The pinned contract versions are part of the health report.
    assert!(
        stdout.contains("contracts: engine")
            && stdout.contains("event_schema")
            && stdout.contains("database_schema")
            && stdout.contains("driver_protocol"),
        "doctor output: {stdout}"
    );
    assert!(!stdout.contains("MISSING  git"));

    // A broken bundle turns the report unhealthy.
    std::fs::write(bundle.join("policy.json"), "{}").unwrap();
    let (code, stdout, _) = forge(
        &["doctor", "--bundle", bundle.to_str().unwrap(), "--db", db.to_str().unwrap()],
        dir.path(),
    );
    assert_eq!(code, Some(1), "doctor output: {stdout}");
    assert!(stdout.contains("MISSING  bundle"));
}
