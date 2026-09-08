use super::*;
use serde_json::json;
use std::path::PathBuf;

fn always_missing(_: &str) -> Option<String> {
    None
}

fn always_present(_: &str) -> Option<String> {
    Some("1.0.0".into())
}

fn openspec_present(program: &str) -> Option<String> {
    (program == "openspec").then(|| "OpenSpec 1.12.0".into())
}

fn unexpected_probe(program: &str) -> Option<String> {
    panic!("doctor must not execute rejected dialect binary {program}")
}

fn install_openspec_dialect(dir: &Path) {
    std::fs::create_dir_all(dir.join("dialects/openspec")).unwrap();
    std::fs::copy(
        workspace().join("dialects/openspec.json"),
        dir.join("dialects/openspec.json"),
    )
    .unwrap();
    for entry in std::fs::read_dir(workspace().join("dialects/openspec")).unwrap() {
        let source = entry.unwrap().path();
        std::fs::copy(
            &source,
            dir.join("dialects/openspec")
                .join(source.file_name().unwrap()),
        )
        .unwrap();
    }
}

fn dialect_world(dir: &Path, with_dialect: bool) -> brokkr_runtime::realms::World {
    let realm = if with_dialect {
        json!({"name": "app", "path": "app", "default_branch": "main", "dialect": "openspec"})
    } else {
        json!({"name": "app", "path": "app", "default_branch": "main"})
    };
    std::fs::create_dir_all(dir.join("app")).unwrap();
    std::fs::write(
        dir.join("realms.json"),
        serde_json::to_vec(&json!({
            "schema": "forge.realms/v3",
            "realms": [realm],
            "journal": ".forge/forge.db"
        }))
        .unwrap(),
    )
    .unwrap();
    brokkr_runtime::realms::World::load(&dir.join("realms.json")).unwrap()
}

#[test]
fn doctor_reports_a_realms_dialect_tool_pin_and_required_files() {
    let dir = tempfile::tempdir().unwrap();
    install_openspec_dialect(dir.path());
    std::fs::create_dir_all(dir.path().join("app/openspec")).unwrap();
    std::fs::write(
        dir.path().join("app/openspec/config.yaml"),
        "schema: spec-driven\n",
    )
    .unwrap();
    let world = dialect_world(dir.path(), true);
    let mut report = Report {
        healthy: true,
        lines: Vec::new(),
    };
    report_realm_dialects(&mut report, &world, openspec_present);
    let rendered = report.render();
    assert!(
        rendered.contains(
            "ok       dialect app: openspec · tool 'openspec' OpenSpec 1.12.0 · pinned 1.12.0"
        ),
        "{rendered}"
    );
    assert!(
        rendered.contains("ok       dialect app requires openspec/config.yaml: present at"),
        "{rendered}"
    );
    assert!(report.healthy);
}

#[test]
fn doctor_reports_a_realm_without_a_dialect() {
    let dir = tempfile::tempdir().unwrap();
    let world = dialect_world(dir.path(), false);
    let mut report = Report {
        healthy: true,
        lines: Vec::new(),
    };
    report_realm_dialects(&mut report, &world, always_missing);
    assert_eq!(report.render(), "ok       dialect app: none declared");
    assert!(report.healthy);
}

#[test]
fn doctor_reports_a_broken_realm_map_and_an_unusable_dialect() {
    let broken = tempfile::tempdir().unwrap();
    std::fs::write(broken.path().join("realms.json"), "{").unwrap();
    let mut report = Report {
        healthy: true,
        lines: Vec::new(),
    };
    report_realm(&mut report, broken.path(), None, always_missing);
    assert!(report.render().contains("MISSING  realms map:"));

    let invalid = tempfile::tempdir().unwrap();
    install_openspec_dialect(invalid.path());
    let dialect_path = invalid.path().join("dialects/openspec.json");
    let mut dialect: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&dialect_path).unwrap()).unwrap();
    dialect["tool"]["binary"] = json!("/bin/echo");
    std::fs::write(&dialect_path, serde_json::to_vec(&dialect).unwrap()).unwrap();
    let world = dialect_world(invalid.path(), true);
    let mut report = Report {
        healthy: true,
        lines: Vec::new(),
    };
    report_realm_dialects(&mut report, &world, unexpected_probe);
    assert!(report
        .render()
        .contains("MISSING  dialect app: realm 'app' dialect is unusable: dialect"));
    assert!(report
        .render()
        .contains("tool binary '/bin/echo' must be a bare filename"));
}

#[test]
fn doctor_checks_requires_beneath_an_absolute_realm_path() {
    let dir = tempfile::tempdir().unwrap();
    install_openspec_dialect(dir.path());
    std::fs::create_dir_all(dir.path().join("app/openspec")).unwrap();
    std::fs::write(
        dir.path().join("app/openspec/config.yaml"),
        "schema: spec-driven\n",
    )
    .unwrap();
    let mut world = dialect_world(dir.path(), true);
    world.map.realms[0].path = dir.path().join("app").display().to_string();
    let mut report = Report {
        healthy: true,
        lines: Vec::new(),
    };
    report_realm_dialects(&mut report, &world, openspec_present);
    assert!(report
        .render()
        .contains("requires openspec/config.yaml: present"));
}

#[test]
fn doctor_warns_for_a_missing_dialect_tool_but_fails_a_missing_required_file() {
    let dir = tempfile::tempdir().unwrap();
    install_openspec_dialect(dir.path());
    let world = dialect_world(dir.path(), true);
    let mut report = Report {
        healthy: true,
        lines: Vec::new(),
    };
    report_realm_dialects(&mut report, &world, always_missing);
    let rendered = report.render();
    assert!(rendered.contains("warn     dialect app: openspec · tool binary 'openspec' not found · pinned 1.12.0 — the design route will refuse to run"), "{rendered}");
    assert!(
        rendered.contains("MISSING  dialect app requires openspec/config.yaml: missing at"),
        "{rendered}"
    );
    assert!(!report.healthy);

    let mut mismatch = Report {
        healthy: true,
        lines: Vec::new(),
    };
    report_realm_dialects(&mut mismatch, &world, always_present);
    assert!(mismatch
        .render()
        .contains("pinned 1.12.0 (version differs)"));
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

/// No variable is ever set, so nothing is satisfied ambiently and the
/// ambient report has nothing to say — which keeps every test below
/// about the surface it is actually asserting. The shipped tree does
/// declare a credential now (`dsh`'s `spark` route, since the operator
/// ruled it local on 2026-09-03); that line is pinned on its own, in
/// `doctor_names_the_shipped_spark_route_when_its_key_is_ambient`.
fn never_ambient(_: &str) -> bool {
    false
}

/// The one variable the shipped adapter tree names.
fn spark_key_is_set(name: &str) -> bool {
    name == "SPARK_API_KEY"
}

fn shipped(dir: &Path, probe: fn(&str) -> Option<String>) -> Report {
    doctor_with_probe(
        None,
        dir,
        &workspace().join("agents"),
        &workspace().join("adapters"),
        &dir.join("secrets.env"),
        probe,
        never_ambient,
    )
}

#[test]
fn doctor_reports_a_declared_house_that_cannot_be_read() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("realms.json"),
        serde_json::to_vec(&json!({
            "schema": "forge.realms/v3",
            "realms": [{"name": "app", "path": ".", "default_branch": "main",
                        "house": "missing.md"}],
            "journal": ".forge/forge.db"
        }))
        .unwrap(),
    )
    .unwrap();
    let mut report = Report {
        healthy: true,
        lines: Vec::new(),
    };
    report_realm_house(&mut report, dir.path(), None);
    assert!(!report.healthy);
    assert!(report
        .render()
        .contains("MISSING  house rules: realm 'app' names house"));
}

#[test]
fn doctor_honours_a_named_map_and_reports_an_unreadable_neighbour_house() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("here")).unwrap();
    std::fs::write(dir.path().join("here/HOUSE.md"), "Here.\n").unwrap();
    let named = dir.path().join("fleet.json");
    std::fs::write(
        &named,
        serde_json::to_vec(&json!({
            "schema": "forge.realms/v3",
            "realms": [
                {"name": "here", "path": "here", "default_branch": "main",
                 "house": "HOUSE.md"},
                {"name": "away", "path": "not-checked-out", "default_branch": "main",
                 "house": "HOUSE.md"}
            ],
            "journal": ".forge/forge.db"
        }))
        .unwrap(),
    )
    .unwrap();
    let mut report = Report {
        healthy: true,
        lines: Vec::new(),
    };
    report_realm_house(&mut report, dir.path(), Some(&named));
    let rendered = report.render();
    assert!(!report.healthy);
    assert!(rendered.contains("realm 'away' names house"), "{rendered}");
    assert!(!rendered.contains("MISSING  realms map"), "{rendered}");
}

#[test]
fn doctor_reports_a_broken_map_as_the_map_not_as_missing_house_rules() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("realms.json"), "{not json").unwrap();
    let mut report = Report {
        healthy: true,
        lines: Vec::new(),
    };
    report_realm_house(&mut report, dir.path(), None);
    let rendered = report.render();
    assert!(rendered.contains("MISSING  realms map:"), "{rendered}");
    assert!(!rendered.contains("MISSING  house rules:"), "{rendered}");
}

#[test]
fn doctor_reports_readable_and_absent_house_declarations() {
    let dir = tempfile::tempdir().unwrap();
    let mut report = Report {
        healthy: true,
        lines: Vec::new(),
    };
    report_realm_house(&mut report, dir.path(), None);
    assert_eq!(
        report.render(),
        "ok       house rules: no realms map; none declared"
    );

    std::fs::write(dir.path().join("HOUSE.md"), "A readable house.\n").unwrap();
    std::fs::write(
        dir.path().join("realms.json"),
        serde_json::to_vec(&json!({
            "schema": "forge.realms/v3",
            "realms": [{"name": "app", "path": ".", "default_branch": "main",
                        "house": "HOUSE.md"}],
            "journal": ".forge/forge.db"
        }))
        .unwrap(),
    )
    .unwrap();
    let mut report = Report {
        healthy: true,
        lines: Vec::new(),
    };
    report_realm_house(&mut report, dir.path(), None);
    assert!(report.healthy);
    assert_eq!(
        report.render(),
        "ok       house rules: 1 realm declaration(s) readable"
    );
}

#[test]
fn public_doctor_includes_the_workspace_house_check() {
    let dir = tempfile::tempdir().unwrap();
    let report = doctor(
        None,
        &dir.path().join("forge.db"),
        &dir.path().join("secrets.env"),
        None,
    );
    assert!(report.render().contains("house rules:"));
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
    assert_eq!(
        safe_version(b"tool 1.0\x1b[31m\nforged line\n"),
        "tool 1.0[31m"
    );
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
    assert!(rendered.contains("BROKKR_LANETALLY_BIN"), "{rendered}");
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
    assert!(rendered.contains("chain fable → opus"), "{rendered}");

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
            "efforts": ["low", "medium", "high"],
            "effort_flag": "--effort",
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
        &dir.path().join("secrets.env"),
        always_missing,
        never_ambient,
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
        &dir.path().join("secrets.env"),
        always_missing,
        never_ambient,
    );
    assert!(report.render().contains("warn     adapters:"));
}

/// The same ruling over the tree this repository actually ships, which
/// is where it was earned: the run that parked at seq 14 on
/// `MISSING_CREDENTIAL` for `spark` was reaching for a value the
/// launching shell either had or did not, and neither answer was
/// visible anywhere. Now that `adapters/dsh.json` names the route's
/// variable, doctor says which channel it came from — a data change
/// producing a report line, with no doctor code behind it.
#[test]
fn doctor_names_the_shipped_spark_route_when_its_key_is_ambient() {
    let dir = tempfile::tempdir().unwrap();
    let rendered = doctor_with_probe(
        None,
        dir.path(),
        &workspace().join("agents"),
        &workspace().join("adapters"),
        &dir.path().join("secrets.env"),
        always_missing,
        spark_key_is_set,
    )
    .render();
    assert!(
        rendered.contains(
            "warn     route spark: credential 'SPARK_API_KEY' is satisfied \
             from the process environment"
        ),
        "{rendered}"
    );

    // Bound in the store instead, the channel this line exists to make
    // visible is not in use, and the shipped tree goes quiet again.
    let store = dir.path().join("secrets.env");
    brokkr_protocol::secret::store_set(&store, "SPARK_API_KEY", "long-enough").unwrap();
    let rendered = doctor_with_probe(
        None,
        dir.path(),
        &workspace().join("agents"),
        &workspace().join("adapters"),
        &store,
        always_missing,
        spark_key_is_set,
    )
    .render();
    assert!(!rendered.contains("route spark"), "{rendered}");
}

/// Decision 0036 ruling 5: the ambient channel stops being invisible.
/// A credential the bindings store holds is silent; one the store does
/// not hold and the environment does is a `warn` NAMED BY ROUTE; one
/// nobody has anywhere is not this line's business (a run refuses on it
/// with `MISSING_CREDENTIAL`, which is a different report).
#[test]
fn doctor_names_every_route_taking_its_credential_from_the_ambient_environment() {
    fn set_but_for_one(name: &str) -> bool {
        name != "NOWHERE_API_KEY"
    }
    let dir = tempfile::tempdir().unwrap();
    let adapters = dir.path().join("adapters");
    std::fs::create_dir_all(&adapters).unwrap();
    std::fs::write(
        adapters.join("many.json"),
        serde_json::to_vec_pretty(&json!({
            "provider": "many",
            "efforts": [],
            "effort_flag": "unsupported",
            "binary": "many-cli",
            "driver": ["many-cli"],
            "egress": "uncontracted",
            "routes": {"nearby": "local"},
            "credentials": {
                "nearby": "NEARBY_API_KEY",
                "partner": "PARTNER_API_KEY",
                "nowhere": "NOWHERE_API_KEY",
            },
            "models": {"near": "nearby/small-1"},
            "model_flag": "-m",
            "tool_permissions": "unsupported",
            "mcp": "unsupported",
        }))
        .unwrap(),
    )
    .unwrap();
    let store = dir.path().join("secrets.env");
    brokkr_protocol::secret::store_set(&store, "PARTNER_API_KEY", "long-enough").unwrap();

    let rendered = doctor_with_probe(
        None,
        dir.path(),
        &dir.path().join("no-such-library"),
        &adapters,
        &store,
        always_missing,
        set_but_for_one,
    )
    .render();
    assert!(
        rendered.contains(
            "warn     route nearby: credential 'NEARBY_API_KEY' is satisfied \
             from the process environment"
        ),
        "{rendered}"
    );
    assert!(rendered.contains("0036 ruling 5"), "{rendered}");
    // Bound in the store: the channel this decision exists to make
    // visible is not in use, so there is nothing to say.
    assert!(!rendered.contains("route partner"), "{rendered}");
    // Set nowhere at all: a missing credential is a run's refusal, not a
    // report about an ambient one.
    assert!(!rendered.contains("route nowhere"), "{rendered}");

    // And the store the operator actually names is the one consulted:
    // pointed at a store that holds nothing, the bound route joins the
    // ambient ones.
    let rendered = doctor_with_probe(
        None,
        dir.path(),
        &dir.path().join("no-such-library"),
        &adapters,
        &dir.path().join("no-such-secrets.env"),
        always_missing,
        set_but_for_one,
    )
    .render();
    assert!(rendered.contains("route partner"), "{rendered}");
}

/// An adapters tree with one provider fronting two routes, each with a
/// credential of its own: the shape decision 0040 ruling 4 is about.
fn two_credentialled_routes(adapters: &Path) {
    std::fs::create_dir_all(adapters).unwrap();
    std::fs::write(
        adapters.join("many.json"),
        serde_json::to_vec_pretty(&json!({
            "provider": "many",
            "efforts": [],
            "effort_flag": "unsupported",
            "binary": "many-cli",
            "driver": ["many-cli"],
            "egress": "uncontracted",
            "routes": {"nearby": "local", "partner": "local"},
            "credentials": {
                "nearby": "NEARBY_API_KEY",
                "partner": "PARTNER_API_KEY",
            },
            "models": {"near": "nearby/small-1"},
            "model_flag": "-m",
            "tool_permissions": "unsupported",
            "mcp": "unsupported",
        }))
        .unwrap(),
    )
    .unwrap();
}

/// A one-seat bundle on that provider, binding exactly `secrets`.
fn bundle_binding(dir: &Path, secrets: &[&str]) -> PathBuf {
    let bundle = dir.join("bound");
    std::fs::create_dir_all(&bundle).unwrap();
    std::fs::write(
        bundle.join("policy.json"),
        serde_json::to_vec_pretty(&json!({
            "phases": ["work", "review", "done", "stop"],
            "initial": "work",
            "terminal": ["done", "stop"],
            "shippable_from": ["review"],
            "rules": [
                {"id": "W-PASS", "from": "work", "result": "pass", "next": "review",
                 "reason": "work concluded"},
                {"id": "R-OK", "from": "review", "result": "clean", "next": "done",
                 "reason": "review concluded"},
            ],
        }))
        .unwrap(),
    )
    .unwrap();
    std::fs::write(bundle.join("role.md"), "# role\n").unwrap();
    std::fs::write(
        bundle.join("bundle.json"),
        serde_json::to_vec_pretty(&json!({
            "name": "bound",
            "policy": "policy.json",
            "seats": {
                "work": {
                    "results": ["pass"],
                    "role": "role.md",
                    "secrets": secrets,
                    "driver": {"command": [
                        "{brokkr}", "driver", "many", "--", "-m", "nearby/small-1",
                    ]},
                },
                "review": {
                    "results": ["clean"],
                    "role": "role.md",
                    "driver": {"command": [
                        "{brokkr}", "driver", "many", "--", "-m", "nearby/small-1",
                    ]},
                },
            },
        }))
        .unwrap(),
    )
    .unwrap();
    bundle
}

/// Decision 0040 ruling 4: ambient means UNBOUND BY ANY SEAT, not absent
/// from the store. A name sitting in the bindings store that no seat
/// declares in its `secrets` is never handed to the driver — so if the
/// launching shell exports it the driver still takes it ambiently, and
/// the store-membership reading said nothing at all. A false negative on
/// exactly the channel decision 0036 ruling 5 exists to make visible.
#[test]
fn doctor_reads_ambient_against_the_bundles_own_bindings_not_the_store() {
    fn everything_is_set(_: &str) -> bool {
        true
    }
    let dir = tempfile::tempdir().unwrap();
    let adapters = dir.path().join("adapters");
    two_credentialled_routes(&adapters);
    // Both names sit in the store; only one of them is bound by a seat.
    let store = dir.path().join("secrets.env");
    brokkr_protocol::secret::store_set(&store, "NEARBY_API_KEY", "long-enough").unwrap();
    brokkr_protocol::secret::store_set(&store, "PARTNER_API_KEY", "long-enough").unwrap();
    let bundle = bundle_binding(dir.path(), &["NEARBY_API_KEY"]);

    let rendered = doctor_with_probe(
        Some(&bundle),
        dir.path(),
        &dir.path().join("no-such-library"),
        &adapters,
        &store,
        always_missing,
        everything_is_set,
    )
    .render();
    assert!(
        rendered.contains("ok       bundle: 'bound' compiles"),
        "{rendered}"
    );
    // Held and DECLARED: the seat binds it, so the run hands it over and
    // there is nothing ambient to report.
    assert!(!rendered.contains("route nearby"), "{rendered}");
    // Held and UNDECLARED, with the variable exported: warned by route,
    // though the store holds it — which is the whole ruling.
    assert!(
        rendered.contains(
            "warn     route partner: credential 'PARTNER_API_KEY' is satisfied \
             from the process environment"
        ),
        "{rendered}"
    );
    assert!(
        rendered.contains("no seat of the inspected bundle binds it"),
        "{rendered}"
    );
    assert!(rendered.contains("0040 ruling 4"), "{rendered}");

    // The other way round proves the store is not the test at all: a
    // bundle binding the OTHER name silences the other route and warns
    // on this one.
    let bundle = bundle_binding(dir.path(), &["PARTNER_API_KEY"]);
    let rendered = doctor_with_probe(
        Some(&bundle),
        dir.path(),
        &dir.path().join("no-such-library"),
        &adapters,
        &store,
        always_missing,
        everything_is_set,
    )
    .render();
    assert!(rendered.contains("route nearby"), "{rendered}");
    assert!(!rendered.contains("route partner"), "{rendered}");

    // And a variable nobody exports is nobody's ambient value, bound or
    // not: a missing credential is a run's refusal, not this report.
    let rendered = doctor_with_probe(
        Some(&bundle),
        dir.path(),
        &dir.path().join("no-such-library"),
        &adapters,
        &store,
        always_missing,
        never_ambient,
    )
    .render();
    assert!(!rendered.contains("route nearby"), "{rendered}");
}

/// The other half of ruling 4's same sentence: store membership is
/// NECESSARY for a binding, so a name a seat declares and the store
/// cannot answer for is bound to nothing either, and an exported copy of
/// it is ambient. The declaring seat refuses at spawn — but `declared`
/// is a union over every seat, and the sibling seat on this route
/// declares nothing, spawns, and reads the launching shell's value.
/// Reading declaration alone as coverage silenced exactly that.
#[test]
fn doctor_reads_a_declared_name_the_store_cannot_answer_for_as_ambient() {
    fn everything_is_set(_: &str) -> bool {
        true
    }
    let dir = tempfile::tempdir().unwrap();
    let adapters = dir.path().join("adapters");
    two_credentialled_routes(&adapters);
    // The store holds one of the two names; the bundle declares BOTH.
    let store = dir.path().join("secrets.env");
    brokkr_protocol::secret::store_set(&store, "PARTNER_API_KEY", "long-enough").unwrap();
    let bundle = bundle_binding(dir.path(), &["NEARBY_API_KEY", "PARTNER_API_KEY"]);

    let rendered = doctor_with_probe(
        Some(&bundle),
        dir.path(),
        &dir.path().join("no-such-library"),
        &adapters,
        &store,
        always_missing,
        everything_is_set,
    )
    .render();
    assert!(
        rendered.contains("ok       bundle: 'bound' compiles"),
        "{rendered}"
    );
    // Declared AND held by the store: both halves, so it is bound and
    // this line has nothing to say.
    assert!(!rendered.contains("route partner"), "{rendered}");
    // Declared but ABSENT from the store: the seat names it and nothing
    // can be handed over, so the exported copy is what the driver reads.
    assert!(
        rendered.contains(
            "warn     route nearby: credential 'NEARBY_API_KEY' is satisfied \
             from the process environment"
        ),
        "{rendered}"
    );
    // And the line names which half failed, not the other one's reason.
    assert!(
        rendered.contains("the seat declaring it can be handed nothing the bindings store at"),
        "{rendered}"
    );
    assert!(
        rendered.contains("store membership is necessary for a binding"),
        "{rendered}"
    );
    assert!(
        !rendered.contains("no seat of the inspected bundle binds it"),
        "{rendered}"
    );
}

/// The second half of ruling 4: without a bundle to inspect there are no
/// seats to ask, so doctor answers the weaker question — store
/// membership — and SAYS that is the question it answered. A weaker
/// check honestly named beats a strong one silently missed.
#[test]
fn doctor_without_a_bundle_says_it_checked_the_store_and_not_the_seats() {
    fn everything_is_set(_: &str) -> bool {
        true
    }
    let dir = tempfile::tempdir().unwrap();
    let adapters = dir.path().join("adapters");
    two_credentialled_routes(&adapters);
    let store = dir.path().join("secrets.env");
    brokkr_protocol::secret::store_set(&store, "NEARBY_API_KEY", "long-enough").unwrap();

    let rendered = doctor_with_probe(
        None,
        dir.path(),
        &dir.path().join("no-such-library"),
        &adapters,
        &store,
        always_missing,
        everything_is_set,
    )
    .render();
    // The name the store holds is silent, as it was before the ruling —
    // and the name it does not hold is warned with the caveat attached.
    assert!(!rendered.contains("route nearby"), "{rendered}");
    assert!(rendered.contains("route partner"), "{rendered}");
    assert!(
        rendered.contains(
            "no bundle was given to inspect, so this checked membership of the \
             bindings store at"
        ),
        "{rendered}"
    );
    assert!(
        rendered.contains("not whether any seat binds it"),
        "{rendered}"
    );
    assert!(rendered.contains("0040 ruling 4"), "{rendered}");

    // A bundle that does not COMPILE is no bundle to inspect either: it
    // declares nothing this report can trust, so it falls to the same
    // weaker question rather than reading an empty set as "no seat binds
    // anything". But it says so in its OWN words — an operator who
    // passed `--bundle` and reads `MISSING bundle` two lines away is not
    // told they passed no bundle.
    let rendered = doctor_with_probe(
        Some(&dir.path().join("absent")),
        dir.path(),
        &dir.path().join("no-such-library"),
        &adapters,
        &store,
        always_missing,
        everything_is_set,
    )
    .render();
    assert!(rendered.contains("MISSING  bundle"), "{rendered}");
    assert!(!rendered.contains("route nearby"), "{rendered}");
    assert!(
        rendered.contains(
            "the bundle given does not compile, so it declares no seats to ask \
             and this checked membership of the bindings store at"
        ),
        "{rendered}"
    );
    assert!(
        !rendered.contains("no bundle was given to inspect"),
        "{rendered}"
    );
}

/// The real probe behind that report, asserted where the injected one
/// cannot stand in for it: a BOOLEAN about the process environment,
/// never the value. `PATH` is set for every test process this suite
/// already depends on (`tool_version` spawns by name).
#[test]
fn the_ambient_probe_answers_whether_a_variable_is_set_never_what_it_says() {
    assert!(ambient_variable("PATH"));
    assert!(!ambient_variable("BROKKR_CERTAINLY_UNSET_VARIABLE"));
}

/// A bundle argument still compiles and reports, and a broken one is
/// Decision 0046 ruling 2: doctor's one `boundaries` line names what a
/// run can start under here, and the `hands` line is judged against the
/// realm's boundary rather than against bubblewrap alone.
#[test]
fn doctor_names_the_boundaries_this_machine_offers_and_judges_hands_by_the_realms() {
    fn linux_box(program: &str) -> Option<String> {
        match program {
            "bwrap" => Some("0.11.0".into()),
            "docker" => Some("27.0".into()),
            "sandbox-exec" => None,
            _ => Some("1.0.0".into()),
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let rendered = shipped(dir.path(), linux_box).render();
    assert!(
        rendered.contains(
            "ok       boundaries: namespace (bubblewrap 0.11.0) · harness · open offered; \
             seatbelt built by slice (ii) of decision 0046 ruling 6 (sandbox-exec not on \
             PATH); container built by slice (iii) of decision 0046 ruling 6 (docker 27.0 found)"
        ),
        "{rendered}"
    );
    let rendered = shipped(dir.path(), always_missing).render();
    assert!(
        rendered.contains(
            "ok       boundaries: harness · open offered; namespace needs bwrap on PATH \
             (not found); seatbelt built by slice (ii)"
        ),
        "{rendered}"
    );
    assert!(
        rendered.contains("container built by slice (iii) of decision 0046 ruling 6 (docker or podman not on PATH)"),
        "{rendered}"
    );

    // `--bundle` in a `harness` realm on a machine without bubblewrap
    // stays healthy on the hands account; in a `seatbelt` realm the line
    // warns about the slice with and without the tool.
    let under = |boundary: Boundary, probe: fn(&str) -> Option<String>| {
        doctor_in(
            Some(&workspace().join("recipes/fast")),
            dir.path(),
            &workspace().join("agents"),
            &workspace().join("adapters"),
            &dir.path().join("secrets.env"),
            probe,
            never_ambient,
            boundary,
            None,
        )
        .render()
    };
    let harness = under(Boundary::Harness, always_missing);
    assert!(
        harness.contains(
            "ok       hands: seats [\"ship\", \"verify\"] declare hands and can run under \
             `harness` — no box of Brokkr's is built there"
        ),
        "{harness}"
    );
    assert!(
        harness.contains("ok       bundle: 'fast' compiles"),
        "{harness}"
    );
    for probe in [always_missing as fn(&str) -> Option<String>, always_present] {
        let seatbelt = under(Boundary::Seatbelt, probe);
        assert!(
            seatbelt.contains(
                "warn     hands: seats [\"ship\", \"verify\"] declare hands and will refuse to \
                 spawn: `seatbelt` is built by slice (ii) of decision 0046 ruling 6, not by \
                 this engine"
            ),
            "{seatbelt}"
        );
    }
}

/// still a hard failure.
#[test]
fn doctor_still_compiles_a_named_bundle() {
    let dir = tempfile::tempdir().unwrap();
    let report = doctor_with_probe(
        Some(&workspace().join("recipes/fast")),
        dir.path(),
        &workspace().join("agents"),
        &workspace().join("adapters"),
        &dir.path().join("secrets.env"),
        always_present,
        never_ambient,
    );
    let rendered = report.render();
    assert!(rendered.contains("ok       bundle: 'fast' compiles"));
    assert!(
        rendered.contains("seats [\"ship\", \"verify\"] declare hands and can run"),
        "{rendered}"
    );

    let report = doctor_with_probe(
        Some(&workspace().join("recipes/fast")),
        dir.path(),
        &workspace().join("agents"),
        &workspace().join("adapters"),
        &dir.path().join("secrets.env"),
        always_missing,
        never_ambient,
    );
    let rendered = report.render();
    assert!(
        rendered
            .contains("bubblewrap (bwrap) not found — seats [\"ship\", \"verify\"] declare hands"),
        "{rendered}"
    );

    let report = doctor_with_probe(
        Some(&dir.path().join("absent")),
        dir.path(),
        &workspace().join("agents"),
        &workspace().join("adapters"),
        &dir.path().join("secrets.env"),
        always_present,
        never_ambient,
    );
    assert!(report.render().contains("MISSING  bundle"));
}

/// `brokkr doctor --bundle` exposes the effort-pin refusal, not only the
/// model-pin one (decision 0035 ruling 5). The operator who runs doctor
/// before a run must see the same failure the compile would give them,
/// with the same repair — a bundle that would refuse to compile must not
/// read as healthy here.
#[test]
fn doctor_exposes_the_effort_pin_refusal_with_its_repair() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("halfhire");
    std::fs::create_dir_all(&bundle).unwrap();
    std::fs::write(
        bundle.join("policy.json"),
        serde_json::to_vec_pretty(&json!({
            "phases": ["implement", "done"],
            "initial": "implement",
            "terminal": ["done"],
            "rules": [{"id": "OK", "from": "implement", "result": "complete",
                       "next": "done", "reason": "done."}],
        }))
        .unwrap(),
    )
    .unwrap();
    std::fs::write(bundle.join("role.md"), "# role\n").unwrap();
    // A model named concretely and an effort named not at all: half a
    // hire, and the half it withholds is the half that moves the bill.
    std::fs::write(
        bundle.join("bundle.json"),
        serde_json::to_vec_pretty(&json!({
            "name": "halfhire",
            "policy": "policy.json",
            "seats": {"implement": {
                "results": ["complete"],
                "role": "role.md",
                "driver": {"command": [
                    "brokkr", "driver", "claude", "--", "--model", "claude-opus-5",
                ]},
            }},
        }))
        .unwrap(),
    )
    .unwrap();
    let report = doctor_with_probe(
        Some(&bundle),
        dir.path(),
        &workspace().join("agents"),
        &workspace().join("adapters"),
        &dir.path().join("secrets.env"),
        always_present,
        never_ambient,
    );
    let rendered = report.render();
    assert!(rendered.contains("MISSING  bundle"), "{rendered}");
    assert!(
        rendered.contains("seats 'implement' do not pin an effort"),
        "{rendered}"
    );
    assert!(rendered.contains("--effort <level>"), "{rendered}");
}

#[test]
fn the_posix_shell_probe_executes_a_portable_shell_operation() {
    assert_eq!(tool_version("sh").as_deref(), Some("POSIX shell"));
}
