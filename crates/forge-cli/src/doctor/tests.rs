use super::*;
use serde_json::json;
use std::path::PathBuf;

fn always_missing(_: &str) -> Option<String> {
    None
}

fn always_present(_: &str) -> Option<String> {
    Some("1.0.0".into())
}

/// The workspace's own `agents/` and `adapters/` trees: doctor's default
/// roots, and the ones that must show up in its report.
fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn shipped(dir: &Path, probe: fn(&str) -> Option<String>) -> Report {
    doctor_with_probe(
        None,
        dir,
        &workspace().join("agents"),
        &workspace().join("adapters"),
        probe,
    )
}

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
    let report = shipped(dir.path(), always_missing);
    assert!(!report.healthy);
    assert!(report.render().contains("MISSING  git"));
    assert!(report.render().contains("warn     claude"));
    assert!(report.render().contains("MISSING  database"));
}

/// T20/AC-10: every provider line comes from an adapter FILE — its
/// binary, its probe result and the abstract models it declares — and
/// the operator's advice comes from the file too, so correcting it is an
/// edit rather than a release.
#[test]
fn doctor_reports_providers_and_models_read_from_the_adapter_files() {
    let dir = tempfile::tempdir().unwrap();
    let rendered = shipped(dir.path(), always_missing).render();
    // Missing provider: a warning, never a hard failure — the fleet must
    // work on machines without every tool.
    assert!(!rendered.contains("MISSING  lanetally"), "{rendered}");
    assert!(
        rendered.contains("warn     lanetally: binary 'claude-lanetally' not found"),
        "{rendered}"
    );
    assert!(
        rendered.contains("~/.local/bin/claude-lanetally"),
        "{rendered}"
    );
    assert!(rendered.contains("FORGE_LANETALLY_BIN"), "{rendered}");
    // The declared models are read from the file, not from a list here.
    assert!(
        rendered.contains("serves fable, haiku, opus, sonnet"),
        "{rendered}"
    );
    assert!(
        rendered.contains("warn     exec: binary 'sh' not found"),
        "{rendered}"
    );
    assert!(
        rendered.contains(
            "exec: binary 'sh' not found — seats resolving to this \
                           provider will fail to spawn · serves no abstract model yet"
        ),
        "{rendered}"
    );
    assert!(
        rendered.contains("warn     python3: not found — seats using the exec driver"),
        "{rendered}"
    );

    let rendered = shipped(dir.path(), always_present).render();
    assert!(
        rendered.contains("ok       lanetally: 1.0.0 · serves"),
        "{rendered}"
    );
}

/// AC-10's second half: per agent, which model would be chosen HERE —
/// computed by the same pure resolver the compiler calls, with this
/// machine's probed facts. That is the real consumer of availability's
/// non-`unknown` arms.
#[test]
fn doctor_says_which_model_each_agent_would_run_here() {
    let dir = tempfile::tempdir().unwrap();
    let rendered = shipped(dir.path(), always_present).render();
    assert!(
        rendered.contains("ok       agent chief-architect: would run fable via claude here"),
        "{rendered}"
    );
    assert!(
        rendered.contains("chain fable → opus → sonnet"),
        "{rendered}"
    );

    // Nothing installed: every chain entry is unavailable, so doctor says
    // so per agent rather than pretending a run would work.
    let rendered = shipped(dir.path(), always_missing).render();
    assert!(
        rendered.contains(
            "warn     agent chief-architect: agent 'chief-architect' has no \
                           available candidate"
        ),
        "{rendered}"
    );
}

/// A brand-new provider shows up in doctor with no rebuild, and a tree
/// with no library at all is a normal state rather than a failure.
#[test]
fn a_sixth_provider_appears_without_a_rebuild_and_an_absent_library_is_not_a_failure() {
    let dir = tempfile::tempdir().unwrap();
    let adapters = dir.path().join("adapters");
    std::fs::create_dir_all(&adapters).unwrap();
    std::fs::write(
        adapters.join("invented.json"),
        serde_json::to_vec_pretty(&json!({
            "provider": "invented",
            "binary": "invented-cli",
            "driver": ["invented-cli"],
            "models": {"newmodel": "invented/new-1"},
            "model_flag": "-m",
            "tool_permissions": "unsupported",
            "mcp": "unsupported",
        }))
        .unwrap(),
    )
    .unwrap();
    let report = doctor_with_probe(
        None,
        dir.path(),
        &dir.path().join("no-such-library"),
        &adapters,
        always_missing,
    );
    let rendered = report.render();
    assert!(
        rendered.contains("warn     invented: binary 'invented-cli' not found"),
        "{rendered}"
    );
    assert!(rendered.contains("serves newmodel"), "{rendered}");
    // An absent library is information, not a failure: a tree whose
    // bundles all inline needs none.
    assert!(rendered.contains("warn     agents:"), "{rendered}");
    assert!(report.render().contains("MISSING  database"));

    // An unreadable adapters tree is a warning too, and doctor keeps
    // reporting everything else.
    let report = doctor_with_probe(
        None,
        dir.path(),
        &dir.path().join("no-such-library"),
        &dir.path().join("no-such-adapters"),
        always_missing,
    );
    assert!(report.render().contains("warn     adapters:"));
}

/// A bundle argument still compiles and reports, and a broken one is
/// still a hard failure.
#[test]
fn doctor_still_compiles_a_named_bundle() {
    let dir = tempfile::tempdir().unwrap();
    let report = doctor_with_probe(
        Some(&workspace().join("recipes/fast")),
        dir.path(),
        &workspace().join("agents"),
        &workspace().join("adapters"),
        always_present,
    );
    assert!(report.render().contains("ok       bundle: 'fast' compiles"));

    let report = doctor_with_probe(
        Some(&dir.path().join("absent")),
        dir.path(),
        &workspace().join("agents"),
        &workspace().join("adapters"),
        always_present,
    );
    assert!(report.render().contains("MISSING  bundle"));
}
