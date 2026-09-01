//! `brokkr init` scaffolds a bundle that compiles under the
//! constitutional lint; `brokkr doctor` reports health without executing
//! any agent.

use std::process::Command;

fn brokkr(args: &[&str], cwd: &std::path::Path) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_brokkr"))
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
    let (code, _, stderr) = brokkr(&["init", bundle.to_str().unwrap()], dir.path());
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert!(stderr.contains("digest"), "stderr: {stderr}");

    // The scaffold passes the same compile gate as any bundle.
    let (code, stdout, stderr) = brokkr(
        &["compile", "--bundle", bundle.to_str().unwrap()],
        dir.path(),
    );
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert!(stdout.contains("\"starter\""));

    // The tightened ship taxonomy is present from the start.
    let policy = std::fs::read_to_string(bundle.join("policy.json")).unwrap();
    assert!(policy.contains("SHIP-COMPLETE"));
    assert!(policy.contains("SHIPPED-DIRTY"));

    // Refuses to clobber an existing bundle.
    let (code, _, stderr) = brokkr(&["init", bundle.to_str().unwrap()], dir.path());
    assert_eq!(code, Some(1));
    assert!(stderr.contains("refusing to overwrite"), "stderr: {stderr}");
}

/// Decision 0019: a bundle still written with the old `{forge}` token
/// compiles exactly as the scaffold's `{brokkr}` does, and the process
/// says so ONCE — the scaffold has five seats, so a per-read notice
/// would show up here as five lines — on stderr, never on stdout.
#[test]
fn the_old_token_still_compiles_and_is_noticed_once_on_stderr() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("bundle");
    brokkr(&["init", bundle.to_str().unwrap()], dir.path());

    let manifest = bundle.join("bundle.json");
    let scaffolded = std::fs::read_to_string(&manifest).unwrap();
    assert!(
        scaffolded.matches("{brokkr}").count() >= 2,
        "the scaffold writes the new token: {scaffolded}"
    );
    std::fs::write(&manifest, scaffolded.replace("{brokkr}", "{forge}")).unwrap();

    let (code, stdout, stderr) = brokkr(
        &["compile", "--bundle", bundle.to_str().unwrap()],
        dir.path(),
    );
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert!(!stdout.contains("notice:"), "the notice reached stdout");
    let notices: Vec<&str> = stderr
        .lines()
        .filter(|line| line.contains("one more release"))
        .collect();
    assert_eq!(notices.len(), 1, "stderr: {stderr}");
    assert!(notices[0].contains("{forge}") && notices[0].contains("{brokkr}"));
}

#[test]
fn doctor_reports_health_and_validates_a_bundle() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("bundle");
    brokkr(&["init", bundle.to_str().unwrap()], dir.path());

    let db = dir.path().join("forge.db");
    let (code, stdout, _) = brokkr(
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
    let (code, stdout, _) = brokkr(
        &[
            "doctor",
            "--bundle",
            bundle.to_str().unwrap(),
            "--db",
            db.to_str().unwrap(),
        ],
        dir.path(),
    );
    assert_eq!(code, Some(1), "doctor output: {stdout}");
    assert!(stdout.contains("MISSING  bundle"));
}
