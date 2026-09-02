//! `brokkr init` scaffolds a bundle that compiles under the
//! constitutional lint; `brokkr doctor` reports health without executing
//! any agent.
//!
//! Every verb below is run FROM INSIDE the scaffold, because that is what
//! the scaffold is: a workspace carrying its own `adapters/`, where the
//! trust tier its gate seats judge on is declared (decision 0021), read
//! from the workspace like every other root (decision 0023).

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
    // The scaffold says where to stand, once, on stderr.
    assert!(
        stderr.contains("run brokkr from inside"),
        "stderr: {stderr}"
    );

    // The scaffold passes the same compile gate as any bundle.
    let (code, stdout, stderr) = brokkr(&["compile", "--bundle", "."], &bundle);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert!(stdout.contains("\"starter\""));

    // The tightened ship taxonomy is present from the start.
    let policy = std::fs::read_to_string(bundle.join("policy.json")).unwrap();
    assert!(policy.contains("SHIP-COMPLETE"));
    assert!(policy.contains("SHIPPED-DIRTY"));

    // Decision 0021 ruling 1's roster is declared, not left to default:
    // the three gate seats say so, and the two work seats say so too.
    let scaffolded = std::fs::read_to_string(bundle.join("bundle.json")).unwrap();
    assert_eq!(scaffolded.matches("\"class\": \"gate\"").count(), 3);
    assert_eq!(scaffolded.matches("\"class\": \"work\"").count(), 2);
    // …and the tier they judge on is a file in the operator's tree,
    // theirs to demote, rather than a constant inside this binary.
    let adapter = std::fs::read_to_string(bundle.join("adapters/claude.json")).unwrap();
    assert!(adapter.contains("\"trust_tier\": \"trusted\""), "{adapter}");
    assert!(adapter.contains("\"binding_grant\": false"), "{adapter}");

    // Refuses to clobber an existing bundle.
    let (code, _, stderr) = brokkr(&["init", bundle.to_str().unwrap()], dir.path());
    assert_eq!(code, Some(1));
    assert!(stderr.contains("refusing to overwrite"), "stderr: {stderr}");
}

/// The other half of that: the scaffold WRITES a trust declaration, and
/// a tier is an operator's ruling (decision 0021 ruling 3). `init` guards
/// its bundle against clobbering; the declaration is workspace data and
/// is guarded on the same terms, so scaffolding into a tree that already
/// declares one cannot silently re-promote what the operator demoted.
#[test]
fn init_refuses_to_overwrite_an_operators_trust_declaration() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("bundle");
    std::fs::create_dir_all(bundle.join("adapters")).unwrap();
    let declaration = bundle.join("adapters/claude.json");
    std::fs::write(&declaration, "{\"trust_tier\": \"untrusted\"}\n").unwrap();

    let (code, _, stderr) = brokkr(&["init", bundle.to_str().unwrap()], dir.path());
    assert_eq!(code, Some(1), "stderr: {stderr}");
    assert!(stderr.contains("refusing to overwrite"), "stderr: {stderr}");
    // The demotion is still the operator's, and nothing else was written.
    let kept = std::fs::read_to_string(&declaration).unwrap();
    assert!(kept.contains("untrusted"), "{kept}");
    assert!(!bundle.join("bundle.json").exists());
}

/// Decision 0021, from the operator's side: the scaffold's gate seats
/// stand on a declaration in the operator's own tree, so demoting the
/// tier there refuses the very next compile — naming the seat and the
/// driver — rather than quietly leaving three judges unbacked.
#[test]
fn demoting_the_scaffolded_tier_refuses_the_scaffolded_gates() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("bundle");
    brokkr(&["init", bundle.to_str().unwrap()], dir.path());

    let adapter = bundle.join("adapters/claude.json");
    let declared = std::fs::read_to_string(&adapter).unwrap();
    std::fs::write(
        &adapter,
        declared.replace(
            "\"trust_tier\": \"trusted\"",
            "\"trust_tier\": \"untrusted\"",
        ),
    )
    .unwrap();

    let (code, _, stderr) = brokkr(&["compile", "--bundle", "."], &bundle);
    assert_eq!(code, Some(1), "stderr: {stderr}");
    assert!(
        stderr.contains("gate class") && stderr.contains("claude"),
        "stderr: {stderr}"
    );
}

/// Decision 0019: a driver still written with the old `{forge}` token
/// compiles exactly as the scaffold's `{brokkr}` does, and the process
/// says so ONCE — the token now lives in the scaffold's
/// `adapters/claude.json` driver prefix, which every agent seat's
/// resolution expands (five seats, each with a two-model chain, so a
/// per-expansion notice would show up here as ten lines) — on stderr,
/// never on stdout.
#[test]
fn the_old_token_still_compiles_and_is_noticed_once_on_stderr() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("bundle");
    brokkr(&["init", bundle.to_str().unwrap()], dir.path());

    let adapter = bundle.join("adapters/claude.json");
    let scaffolded = std::fs::read_to_string(&adapter).unwrap();
    assert!(
        scaffolded.contains("\"{brokkr}\""),
        "the scaffold writes the new token: {scaffolded}"
    );
    std::fs::write(&adapter, scaffolded.replace("{brokkr}", "{forge}")).unwrap();

    let (code, stdout, stderr) = brokkr(&["compile", "--bundle", "."], &bundle);
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
        &["doctor", "--bundle", ".", "--db", db.to_str().unwrap()],
        &bundle,
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
        &["doctor", "--bundle", ".", "--db", db.to_str().unwrap()],
        &bundle,
    );
    assert_eq!(code, Some(1), "doctor output: {stdout}");
    assert!(stdout.contains("MISSING  bundle"));
}

#[test]
fn doctor_names_every_unpinned_model_seat_and_the_single_repair() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("bundle");
    brokkr(&["init", bundle.to_str().unwrap()], dir.path());
    let bundle_file = bundle.join("bundle.json");
    let mut config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&bundle_file).unwrap()).unwrap();
    for (phase, charter, kind) in [
        ("implement", "implementer", "claude"),
        ("verify", "verifier", "codex"),
    ] {
        let seat = &mut config["seats"][phase];
        seat.as_object_mut().unwrap().remove("agent");
        seat["role"] = serde_json::json!(format!("agents/charters/{charter}.md"));
        seat["driver"] = serde_json::json!({
            "command": ["{brokkr}", "driver", kind, "--"]
        });
    }
    std::fs::write(&bundle_file, serde_json::to_string_pretty(&config).unwrap()).unwrap();
    let db = dir.path().join("forge.db");
    let (code, stdout, _) = brokkr(
        &["doctor", "--bundle", ".", "--db", db.to_str().unwrap()],
        &bundle,
    );
    assert_eq!(code, Some(1), "{stdout}");
    assert!(stdout.contains("'implement'"), "{stdout}");
    assert!(stdout.contains("'verify'"), "{stdout}");
    assert!(stdout.contains("--model <concrete-model-id>"), "{stdout}");
}
