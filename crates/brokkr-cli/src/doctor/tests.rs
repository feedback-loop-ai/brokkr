use super::*;
use brokkr_protocol::adapters::CompositeError;
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

/// Injected box probes (issue #218). Named functions, not closures,
/// because `report_realm_dialects` takes plain `fn` pointers.
fn box_missing(_: &HandsSpec, _: &Path, _: &str) -> Result<Option<String>, String> {
    Ok(None)
}

fn box_present(_: &HandsSpec, _: &Path, _: &str) -> Result<Option<String>, String> {
    Ok(Some("1.0.0".into()))
}

fn box_openspec(_: &HandsSpec, _: &Path, _: &str) -> Result<Option<String>, String> {
    Ok(Some("OpenSpec 1.12.0".into()))
}

fn box_unbuildable(_: &HandsSpec, _: &Path, _: &str) -> Result<Option<String>, String> {
    Err("the box could not be built: no bwrap on PATH".into())
}

/// The probe a REJECTED dialect must never reach: doctor reads the
/// declaration before it runs anything, so an unusable dialect costs no
/// process on either surface.
fn unexpected_box(_: &HandsSpec, _: &Path, program: &str) -> Result<Option<String>, String> {
    panic!("doctor must not build a box for rejected dialect binary {program}")
}

/// The realm-world report, with the box probe no test of a rejected
/// dialect may reach. A test helper, so it lives here and not in the
/// measured file beside the code it exercises.
fn report_realm(
    report: &mut Report,
    workspace: &Path,
    named: Option<&Path>,
    probe: fn(&str) -> Option<String>,
) {
    report_realm_world(
        report,
        // `inspect`, exactly as `doctor()` reads it: a doctor line
        // reports and never refuses, so the loader it stands on is the
        // one that carries a moved crossing as data.
        brokkr_runtime::realms::World::inspect(workspace, named),
        workspace,
        probe,
        unexpected_box,
    );
}

/// One dialect report, both surfaces injected: each test asserts
/// wording, and no test here builds a namespace. The boundary comes from
/// the realm map, exactly as it does in a run.
fn dialects(
    world: &brokkr_runtime::realms::World,
    host: fn(&str) -> Option<String>,
    inside: fn(&HandsSpec, &Path, &str) -> Result<Option<String>, String>,
) -> Report {
    let mut report = Report {
        healthy: true,
        lines: Vec::new(),
    };
    report_realm_dialects(&mut report, world, Path::new("."), host, inside);
    report
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

/// One realm declaration, named after its own directory, optionally
/// carrying the dialect and optionally naming its boundary.
fn realm_json(name: &str, with_dialect: bool, boundary: Option<Boundary>) -> serde_json::Value {
    let mut realm = json!({"name": name, "path": name, "default_branch": "main"});
    if with_dialect {
        realm["dialect"] = json!("openspec");
    }
    if let Some(boundary) = boundary {
        realm["boundary"] = json!(boundary.word());
    }
    realm
}

/// Write a map and load it. A realm may only name its boundary under
/// `forge.realms/v4` (decision 0046 ruling 1), so the schema follows the
/// realms rather than being chosen by hand at each call.
fn world_of(dir: &Path, realms: Vec<serde_json::Value>) -> brokkr_runtime::realms::World {
    let schema = match realms.iter().any(|realm| realm.get("boundary").is_some()) {
        true => "forge.realms/v4",
        false => "forge.realms/v3",
    };
    for realm in &realms {
        std::fs::create_dir_all(dir.join(realm["path"].as_str().unwrap())).unwrap();
    }
    std::fs::write(
        dir.join("realms.json"),
        serde_json::to_vec(&json!({
            "schema": schema,
            "realms": realms,
            "journal": ".forge/forge.db"
        }))
        .unwrap(),
    )
    .unwrap();
    brokkr_runtime::realms::World::load(&dir.join("realms.json")).unwrap()
}

/// The one-realm map most of these tests read: no boundary declared, so
/// the realm stands behind `namespace`, the default every v1..v3 map has
/// always meant.
fn dialect_world(dir: &Path, with_dialect: bool) -> brokkr_runtime::realms::World {
    world_of(dir, vec![realm_json("app", with_dialect, None)])
}

/// The same map with the realm naming its boundary.
fn dialect_world_under(dir: &Path, boundary: Boundary) -> brokkr_runtime::realms::World {
    world_of(dir, vec![realm_json("app", true, Some(boundary))])
}

/// The openspec dialect, its required file in place: the shape every
/// wording test starts from.
fn dialect_realm(dir: &Path, name: &str) {
    install_openspec_dialect(dir);
    std::fs::create_dir_all(dir.join(name).join("openspec")).unwrap();
    std::fs::write(
        dir.join(name).join("openspec/config.yaml"),
        "schema: spec-driven\n",
    )
    .unwrap();
}

/// The same dialect the library serves, installed INSIDE one realm's own
/// tree under a chosen file name — instructions beside it, because
/// `Dialect::load` pins every instruction from the dialect file's own
/// directory. A declaration ending in `.json` resolves against the
/// REALM's root (`library_path`), so this is how two realms can name the
/// same relative path and still get their own file: the decoy a
/// two-repository proof needs, so that a realm secretly resolved to its
/// neighbour's tree would find something usable there and go healthy.
fn realm_local_dialect(dir: &Path, realm: &str, file: &str) {
    std::fs::create_dir_all(dir.join(realm).join("openspec")).unwrap();
    std::fs::copy(
        workspace().join("dialects/openspec.json"),
        dir.join(realm).join(file),
    )
    .unwrap();
    for entry in std::fs::read_dir(workspace().join("dialects/openspec")).unwrap() {
        let source = entry.unwrap().path();
        std::fs::copy(
            &source,
            dir.join(realm)
                .join("openspec")
                .join(source.file_name().unwrap()),
        )
        .unwrap();
    }
}

#[test]
fn doctor_reports_a_realms_dialect_tool_pin_and_required_files() {
    let dir = tempfile::tempdir().unwrap();
    dialect_realm(dir.path(), "app");
    let world = dialect_world(dir.path(), true);
    let report = dialects(&world, openspec_present, box_openspec);
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
    let report = dialects(&world, always_missing, box_missing);
    assert_eq!(report.render(), "ok       dialect app: none declared");
    assert!(report.healthy);
}

/// A world with a crossing drawn across it, and the published file's
/// bytes optionally moved out from under the pin. Two realms, each its
/// own directory and its own house, so a crossing's failure can be told
/// apart from every other line this readout carries.
fn crossing_world(dir: &Path, moved: bool) {
    let pin = brokkr_core::canonical::sha256_bytes(b"{\"title\": \"orders\"}\n");
    for realm in ["alpha", "beta"] {
        std::fs::create_dir_all(dir.join(realm)).unwrap();
        std::fs::write(dir.join(realm).join("HOUSE.md"), "House.\n").unwrap();
    }
    std::fs::create_dir_all(dir.join("alpha/contracts")).unwrap();
    let published = dir.join("alpha/contracts/orders.v1.schema.json");
    let bytes = match moved {
        true => "{\"title\": \"Orders\"}\n",
        false => "{\"title\": \"orders\"}\n",
    };
    std::fs::write(&published, bytes).unwrap();
    std::fs::write(
        dir.join("realms.json"),
        json!({
            "schema": "forge.realms/v5",
            "realms": [
                {"name": "alpha", "path": "alpha", "default_branch": "main",
                 "house": "HOUSE.md",
                 "publishes": [{"name": "orders.api",
                                "path": "contracts/orders.v1.schema.json"}]},
                {"name": "beta", "path": "beta", "default_branch": "main",
                 "house": "HOUSE.md",
                 "consumes": [{"name": "orders.api", "realm": "alpha", "sha256": pin}]},
            ],
            "journal": ".forge/forge.db",
        })
        .to_string(),
    )
    .unwrap();
}

fn realm_report(dir: &Path) -> Report {
    let mut report = Report {
        healthy: true,
        lines: Vec::new(),
    };
    report_realm(&mut report, dir, None, always_missing);
    report
}

/// Decision 0057 through doctor, on decision 0046's Addendum's terms: a
/// crossing that has moved is one realm's LINE, not the end of the
/// readout. The world still exists — its houses, its dialects, its
/// boundaries all still answer — and what a run would have refused is
/// reported here, keyed to the failing realm and the failing crossing,
/// in the run's own words.
#[test]
fn doctor_reports_a_moved_crossing_as_a_line_and_not_a_broken_world() {
    let dir = tempfile::tempdir().unwrap();
    crossing_world(dir.path(), true);
    let report = realm_report(dir.path());
    let rendered = report.render();

    // The publisher is sound and says so; the consumer's pin is not, and
    // its line names the realm, the crossing, the pin and what is there.
    assert!(
        rendered
            .contains("ok       crossings alpha: 1 published file(s) present, 0 pin(s) matching"),
        "{rendered}"
    );
    assert!(
        rendered.contains(
            "MISSING  crossings beta 'orders.api': realm 'beta' consumes crossing \
             'orders.api' from realm 'alpha' pinned at "
        ),
        "{rendered}"
    );
    assert!(
        rendered.contains("orders.v1.schema.json hashes to "),
        "{rendered}"
    );
    assert!(!report.healthy, "a moved crossing is not healthy");

    // And nothing else collapsed: the world came into existence, so the
    // house count and BOTH realms' dialect lines are still there, and
    // the map itself was never reported as broken.
    assert!(
        rendered.contains("ok       house rules: 2 realm declaration(s) readable"),
        "{rendered}"
    );
    assert!(
        rendered.contains("ok       dialect alpha: none declared"),
        "{rendered}"
    );
    assert!(
        rendered.contains("ok       dialect beta: none declared"),
        "{rendered}"
    );
    assert!(!rendered.contains("realms map"), "{rendered}");

    // The same world with its bytes intact: both realms report ok, and
    // the readout is healthy.
    let sound = tempfile::tempdir().unwrap();
    crossing_world(sound.path(), false);
    let report = realm_report(sound.path());
    assert!(
        report
            .render()
            .contains("ok       crossings beta: 0 published file(s) present, 1 pin(s) matching"),
        "{}",
        report.render()
    );
    assert!(report.healthy);
}

/// The other half of the same honesty: when the PUBLISHER's file cannot
/// be read, its consumer's pin was compared to nothing, and doctor says
/// so. It must not print the sound realm's line for beta — "1 pin(s)
/// matching" beside "alpha's file is missing" would claim one contract
/// both verified and unread — and it must not charge beta with a failure
/// either, because the missing file is alpha's to answer for.
#[test]
fn doctor_never_calls_a_pin_matching_when_its_publisher_could_not_be_read() {
    let dir = tempfile::tempdir().unwrap();
    crossing_world(dir.path(), false);
    std::fs::remove_file(dir.path().join("alpha/contracts/orders.v1.schema.json")).unwrap();
    let report = realm_report(dir.path());
    let rendered = report.render();

    assert!(
        rendered.contains("MISSING  crossings alpha 'orders.api': realm 'alpha' publishes"),
        "{rendered}"
    );
    assert!(
        rendered.contains(
            "warn     crossings beta 'orders.api': pin not checked: realm 'alpha' \
             publishes it and its file could not be read"
        ),
        "{rendered}"
    );
    assert!(
        !rendered.contains("crossings beta: "),
        "beta's unchecked pin was counted as verified: {rendered}"
    );
    assert!(!report.healthy, "the publisher's missing file is unhealthy");

    // And the whole world is still reported, as ever: this is a line, not
    // a collapse.
    assert!(
        rendered.contains("ok       house rules: 2 realm declaration(s) readable"),
        "{rendered}"
    );
    assert!(!rendered.contains("realms map"), "{rendered}");
}

/// A world that never drew a crossing gets no crossing line at all —
/// byte for byte the readout it gave before this existed.
#[test]
fn doctor_adds_no_crossing_line_to_a_world_that_draws_none() {
    let dir = tempfile::tempdir().unwrap();
    world_of(dir.path(), vec![realm_json("app", false, None)]);
    let rendered = realm_report(dir.path()).render();
    assert!(!rendered.contains("crossings"), "{rendered}");
    assert_eq!(
        rendered,
        "ok       house rules: 0 realm declaration(s) readable\n\
         ok       dialect app: none declared"
    );
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
    report_realm_dialects(
        &mut report,
        &world,
        Path::new("."),
        unexpected_probe,
        unexpected_box,
    );
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
    dialect_realm(dir.path(), "app");
    let mut world = dialect_world(dir.path(), true);
    world.map.realms[0].path = dir.path().join("app").display().to_string();
    let report = dialects(&world, openspec_present, box_openspec);
    assert!(report
        .render()
        .contains("requires openspec/config.yaml: present"));
}

#[test]
fn doctor_warns_for_a_missing_dialect_tool_but_fails_a_missing_required_file() {
    let dir = tempfile::tempdir().unwrap();
    install_openspec_dialect(dir.path());
    let world = dialect_world(dir.path(), true);
    let report = dialects(&world, always_missing, box_missing);
    let rendered = report.render();
    assert!(rendered.contains("warn     dialect app: openspec · tool binary 'openspec' not found · pinned 1.12.0 — the design route will refuse to run · probed inside the box"), "{rendered}");
    assert!(
        rendered.contains("MISSING  dialect app requires openspec/config.yaml: missing at"),
        "{rendered}"
    );
    assert!(!report.healthy);

    let mismatch = dialects(&world, always_present, box_present);
    assert!(mismatch
        .render()
        .contains("pinned 1.12.0 (version differs)"));
}

/// Issue #218: the host and the box are two surfaces, and doctor must
/// tell them apart. This is the exact failure that lost the run — the
/// tool on the host PATH, the box unable to see it — and the old line
/// called it green.
#[test]
fn doctor_tells_the_host_path_from_the_box() {
    let dir = tempfile::tempdir().unwrap();
    dialect_realm(dir.path(), "app");
    let world = dialect_world(dir.path(), true);
    let report = dialects(&world, openspec_present, box_missing);
    let rendered = report.render();
    assert!(
        rendered.contains(
            "warn     dialect app: openspec · tool 'openspec' present on PATH, not reachable \
             inside the box; install under /usr/local or declare a bind · pinned 1.12.0"
        ),
        "{rendered}"
    );
    // A warning, not a refusal: the machine is otherwise healthy, and the
    // absence will be felt at the boxed gate, not at spawn.
    assert!(report.healthy, "{rendered}");
}

/// The box answer is the one that counts: a tool the box carries is
/// reachable even when the host PATH does not hold it, and the line says
/// which surface answered.
#[test]
fn doctor_believes_the_box_over_the_host() {
    let dir = tempfile::tempdir().unwrap();
    dialect_realm(dir.path(), "app");
    let world = dialect_world(dir.path(), true);
    let report = dialects(&world, always_missing, box_openspec);
    let rendered = report.render();
    assert!(
        rendered.contains(
            "ok       dialect app: openspec · tool 'openspec' OpenSpec 1.12.0 · \
             pinned 1.12.0 · probed inside the box"
        ),
        "{rendered}"
    );
    assert!(report.healthy, "{rendered}");
}

/// `harness` and `open` build no box of Brokkr's, so the host PATH IS the
/// surface — and doctor says so instead of implying a box it never built.
#[test]
fn doctor_says_the_host_path_when_no_box_stands() {
    let dir = tempfile::tempdir().unwrap();
    dialect_realm(dir.path(), "app");
    for boundary in [Boundary::Harness, Boundary::Open] {
        let world = dialect_world_under(dir.path(), boundary);
        let report = dialects(&world, openspec_present, unexpected_box);
        let rendered = report.render();
        assert!(
            rendered.contains(&format!(
                "probed on the host PATH (boundary `{boundary}` builds no box of Brokkr's)"
            )),
            "{boundary}: {rendered}"
        );
        assert!(report.healthy, "{boundary}: {rendered}");
    }
}

/// A boxed boundary whose box cannot be built here (no bubblewrap, an
/// unbuilt slice) is its own fact: the line names the host fallback and
/// the reason, and does not pretend the box answered.
#[test]
fn doctor_says_when_the_box_could_not_be_built() {
    let dir = tempfile::tempdir().unwrap();
    dialect_realm(dir.path(), "app");
    let world = dialect_world(dir.path(), true);
    let report = dialects(&world, openspec_present, box_unbuildable);
    assert!(
        report
            .render()
            .contains("probed on the host PATH (the box could not be built: no bwrap on PATH)"),
        "{}",
        report.render()
    );

    // With no host answer either, the existing refusal stands — and it
    // still names the surface that answered, because "not found" read
    // off a host whose box never stood is not the gate's verdict.
    let absent = dialects(&world, always_missing, box_unbuildable);
    assert!(
        absent.render().contains(
            "tool binary 'openspec' not found · pinned 1.12.0 — the design route will \
             refuse to run · probed on the host PATH (the box could not be built: no \
             bwrap on PATH)"
        ),
        "{}",
        absent.render()
    );
}

/// Decision 0046 ruling 1 makes the boundary the REALM's own, so one map
/// may stand `app` behind a namespace and `docs` in the open. Each
/// dialect line must be answered on the surface ITS realm's gate will run
/// on: doctor judging every realm by the boundary of whichever realm
/// holds the current directory is issue #218's defect moved from the
/// host/box axis to the realm axis — a box built for a realm whose
/// dialect gate is refused at compile, or the host read for a realm that
/// will run boxed.
#[test]
fn each_realm_is_probed_under_its_own_boundary() {
    let dir = tempfile::tempdir().unwrap();
    dialect_realm(dir.path(), "app");
    dialect_realm(dir.path(), "docs");
    let world = world_of(
        dir.path(),
        vec![
            realm_json("app", true, Some(Boundary::Namespace)),
            realm_json("docs", true, Some(Boundary::Open)),
        ],
    );
    let report = dialects(&world, openspec_present, box_openspec);
    let rendered = report.render();
    assert!(
        rendered.contains(
            "ok       dialect app: openspec · tool 'openspec' OpenSpec 1.12.0 · \
             pinned 1.12.0 · probed inside the box"
        ),
        "{rendered}"
    );
    assert!(
        rendered.contains(
            "ok       dialect docs: openspec · tool 'openspec' OpenSpec 1.12.0 · \
             pinned 1.12.0 · probed on the host PATH (boundary `open` builds no box \
             of Brokkr's)"
        ),
        "{rendered}"
    );
    assert!(report.healthy, "{rendered}");
}

/// Phase 2 slice (i), proof 3, dialect half: one realm's BROKEN
/// declaration is named as that realm's failure and leaves its
/// neighbour's line untouched.
///
/// `each_realm_is_probed_under_its_own_boundary` above proves two healthy
/// realms are each answered on their own surface, and
/// `a_declared_broken_dialect_is_refused_only_for_its_realm`
/// (`crates/brokkr-runtime/src/realms/tests.rs`) proves the refusal WORDS
/// name the realm — but on a ONE-realm map, where there is no neighbour
/// to spoil. Neither proves the isolation this asserts: that a second,
/// healthy realm in the SAME world still gets its own `ok` lines, and
/// that the broken realm's name and error appear on no line but its own.
///
/// The broken declaration is the realm's OWN file (`docs/broken.json`),
/// not the shared library — a corrupted `dialects/openspec.json` would
/// break both realms and prove nothing about isolation.
///
/// And it is load-bearing for two TREES, not merely two names: a usable
/// dialect stands at the same relative path in the neighbour's tree
/// (`app/broken.json`), so a `docs` realm that resolved to `app`'s
/// directory would load that one and report `ok`. The control at the
/// foot of this test proves exactly that, so the assertions above cannot
/// pass in a one-tree world.
#[test]
fn a_broken_dialect_in_one_realm_is_named_without_failing_its_neighbour() {
    let dir = tempfile::tempdir().unwrap();
    dialect_realm(dir.path(), "app");
    realm_local_dialect(dir.path(), "app", "broken.json");
    std::fs::create_dir_all(dir.path().join("docs")).unwrap();
    std::fs::write(dir.path().join("docs/broken.json"), "{").unwrap();
    let mut docs = realm_json("docs", false, None);
    docs["dialect"] = json!("broken.json");
    let world = world_of(dir.path(), vec![realm_json("app", true, None), docs]);

    let report = dialects(&world, openspec_present, box_openspec);
    let rendered = report.render();
    assert!(
        !report.healthy,
        "a broken declaration still makes doctor unhealthy: {rendered}"
    );
    // The failing REALM is named, not the world.
    assert!(
        rendered.contains("MISSING  dialect docs: realm 'docs' dialect is unusable:"),
        "{rendered}"
    );
    // And the healthy realm answers exactly as it does alone.
    assert!(
        rendered.contains(
            "ok       dialect app: openspec · tool 'openspec' OpenSpec 1.12.0 · \
             pinned 1.12.0 · probed inside the box"
        ),
        "{rendered}"
    );
    assert!(
        rendered.contains("ok       dialect app requires openspec/config.yaml: present at"),
        "{rendered}"
    );
    // One line per realm's verdict, in map order, and the broken realm's
    // name and words are confined to its own line: a report that blamed
    // `app` for `docs`'s file, or dropped `app` because `docs` failed,
    // would fail here.
    let app_lines: Vec<&str> = rendered
        .lines()
        .filter(|line| line.contains("dialect app"))
        .collect();
    assert_eq!(app_lines.len(), 2, "{rendered}");
    for line in &app_lines {
        assert!(
            !line.contains("docs"),
            "the neighbour's failure leaked: {line}"
        );
        assert!(!line.contains("unusable"), "{line}");
    }
    assert_eq!(
        rendered
            .lines()
            .filter(|line| line.starts_with("MISSING"))
            .count(),
        1,
        "one realm broke, one line: {rendered}"
    );
    // The failure names the file in DOCS's tree. `dialect docs` alone
    // would say as much in a one-tree world; the resolved path is what
    // says which repository was read.
    let path_of = |realm: &str| {
        dir.path()
            .join(realm)
            .join("broken.json")
            .display()
            .to_string()
    };
    assert!(rendered.contains(&path_of("docs")), "{rendered}");
    assert!(
        !rendered.contains(&path_of("app")),
        "docs was answered from its own tree, never its neighbour's: {rendered}"
    );

    // The control: the SAME two declarations against ONE tree. Point
    // `docs` at `app`'s directory and its `broken.json` is app's usable
    // one, so the world goes healthy and every assertion above fails.
    // Two repositories are what this proof rests on, and this is where
    // that is demonstrated rather than assumed.
    let mut one_tree = realm_json("docs", false, None);
    one_tree["path"] = json!("app");
    one_tree["dialect"] = json!("broken.json");
    let world = world_of(dir.path(), vec![realm_json("app", true, None), one_tree]);
    let report = dialects(&world, openspec_present, box_openspec);
    let rendered = report.render();
    assert!(report.healthy, "{rendered}");
    assert!(
        rendered.contains("ok       dialect docs: openspec · tool 'openspec' OpenSpec 1.12.0"),
        "the decoy is a usable dialect, so only path resolution told the \
         two realms apart above: {rendered}"
    );
}

/// Phase 2 slice (i), proof 3, house half — and a LIMITATION recorded
/// rather than papered over.
///
/// The house readout does name a broken realm by name, and it does not
/// blame the healthy one. But it is NOT per-realm the way the dialect
/// readout is: `report_realm_house_for_world` counts every realm's house
/// into ONE aggregate `ok` line, and emits that line only when no realm
/// failed. So in a two-realm world where one house is unreadable, the
/// healthy realm's house gets no line at all — its `ok` is not corrupted,
/// it is simply gone. A reader of `brokkr doctor` cannot tell from the
/// house lines whether the other realm's house was read and fine, or
/// never declared.
///
/// This test asserts what the machine actually does, so the day the
/// readout becomes one line per realm this test fails and is rewritten
/// deliberately. It is the residual this proof reports.
#[test]
fn a_broken_house_names_its_realm_but_the_readout_is_one_world_wide_count() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("app")).unwrap();
    std::fs::write(dir.path().join("app/HOUSE.md"), "One realm rule.\n").unwrap();
    let mut app = realm_json("app", false, None);
    app["house"] = json!("HOUSE.md");

    // Both houses readable: a single count, for two realms, naming
    // neither. Already not per-realm — this is the shape the failure
    // below degrades from.
    std::fs::create_dir_all(dir.path().join("docs")).unwrap();
    std::fs::write(dir.path().join("docs/HOUSE.md"), "Another rule.\n").unwrap();
    let mut docs = realm_json("docs", false, None);
    docs["house"] = json!("HOUSE.md");
    let world = world_of(dir.path(), vec![app.clone(), docs]);
    let mut report = Report {
        healthy: true,
        lines: Vec::new(),
    };
    report_realm_house_for_world(&mut report, &world);
    assert_eq!(
        report.render(),
        "ok       house rules: 2 realm declaration(s) readable"
    );
    assert!(report.healthy);

    // One house unreadable — unreadable in DOCS's tree. A readable file
    // of that very name stands in the neighbour's tree, so a `docs`
    // realm resolved to `app`'s directory would read it and count two;
    // the control at the foot of this test proves it.
    std::fs::write(dir.path().join("app/missing.md"), "The decoy rule.\n").unwrap();
    let mut docs = realm_json("docs", false, None);
    docs["house"] = json!("missing.md");
    let world = world_of(dir.path(), vec![app.clone(), docs]);
    let mut report = Report {
        healthy: true,
        lines: Vec::new(),
    };
    report_realm_house_for_world(&mut report, &world);
    let rendered = report.render();
    assert!(!report.healthy);
    assert!(
        rendered.contains("MISSING  house rules: realm 'docs' names house at"),
        "{rendered}"
    );
    assert!(rendered.contains("missing.md"), "{rendered}");
    assert!(
        !rendered.contains("'app'"),
        "the healthy realm is not blamed for its neighbour: {rendered}"
    );

    // ...and the path it names is the one under DOCS's own tree, not the
    // readable file of the same name next door.
    let path_of = |realm: &str| {
        dir.path()
            .join(realm)
            .join("missing.md")
            .display()
            .to_string()
    };
    assert!(rendered.contains(&path_of("docs")), "{rendered}");
    assert!(
        !rendered.contains(&path_of("app")),
        "docs was answered from its own tree: {rendered}"
    );

    // ...but the healthy realm's own answer is GONE, not merely
    // unnamed: one line, and it is the failure's. This is the residual.
    assert_eq!(rendered.lines().count(), 1, "{rendered}");
    assert!(
        !rendered.contains("declaration(s) readable"),
        "the aggregate count is suppressed by any failure, so a healthy \
         realm beside a broken one states nothing: {rendered}"
    );

    // The control: the same two declarations against ONE tree. `docs` at
    // `app`'s path finds `app/missing.md` readable, so the world counts
    // two and stays healthy — which is to say the failure above is the
    // second REPOSITORY's, and this proof cannot pass in a one-tree
    // world.
    let mut one_tree = realm_json("docs", false, None);
    one_tree["path"] = json!("app");
    one_tree["house"] = json!("missing.md");
    let world = world_of(dir.path(), vec![app, one_tree]);
    let mut report = Report {
        healthy: true,
        lines: Vec::new(),
    };
    report_realm_house_for_world(&mut report, &world);
    assert!(report.healthy, "{}", report.render());
    assert_eq!(
        report.render(),
        "ok       house rules: 2 realm declaration(s) readable"
    );
}

fn executed(
    exit_code: i32,
    stdout: &str,
    stderr: &str,
    timed_out: bool,
) -> brokkr_protocol::hands::Executed {
    brokkr_protocol::hands::Executed {
        stdout: stdout.into(),
        stderr: stderr.into(),
        exit_code,
        timed_out,
    }
}

/// A probe the box ran and a probe the box never ran are different
/// answers. `bash -lc` reports a name it cannot find as 127, and that is
/// the ONLY non-zero exit that says anything about the tool: reading
/// every non-zero exit as "the box does not carry it" tells an operator
/// whose kernel refuses the namespace to install what they already have,
/// which is issue #218's own confusion in a second costume.
#[test]
fn only_a_missing_command_reads_as_a_tool_the_box_cannot_see() {
    assert_eq!(
        box_answer(&executed(0, "OpenSpec 1.12.0\n", "", false)),
        Ok(Some("OpenSpec 1.12.0".into()))
    );
    assert_eq!(
        box_answer(&executed(
            127,
            "",
            "bash: line 1: openspec: command not found",
            false
        )),
        Ok(None)
    );
    // bubblewrap's own refusal: the tool is beside the point, so the
    // reason it gave rides back instead of advice about a binary.
    assert_eq!(
        box_answer(&executed(
            1,
            "",
            "bwrap: setting up uid map: Permission denied\nmore",
            false
        )),
        Err(
            "the probe did not run in the box: exit 1, bwrap: setting up uid map: \
             Permission denied"
                .into()
        )
    );
    assert_eq!(
        box_answer(&executed(1, "", "", false)),
        Err("the probe did not run in the box: exit 1".into())
    );
    assert_eq!(
        box_answer(&executed(124, "", "", true)),
        Err("the probe was still running in the box after 30 seconds".into())
    );
}

/// Can this process build a bubblewrap namespace at all? Nesting is
/// refused by the engine-owned marker, not by a kernel policy (decision
/// 0043).
#[cfg(target_os = "linux")]
fn can_create_namespace() -> bool {
    if std::env::var_os(brokkr_protocol::hands::HANDS_BOX_ENV).is_some() {
        return false;
    }
    Command::new("bwrap")
        .args(["--ro-bind", "/", "/", "--", "true"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Issue #218's regression, measured against a real namespace: a tool
/// that lives only on the host PATH — here a private HOME the box never
/// binds — is reachable on the host and NOT inside the box the dialect's
/// gate builds. This is the test that fails if doctor goes back to
/// answering for the host. Skipped where a namespace cannot be created,
/// including under `BROKKR_HANDS_BOX`, where nesting is refused.
#[cfg(target_os = "linux")]
#[test]
fn a_host_only_dialect_tool_is_unreachable_in_the_gate_box() {
    use std::os::unix::fs::PermissionsExt;
    if !can_create_namespace() {
        // A host that declared it must produce boundary evidence fails
        // here instead of printing `ok` (decision 0054 ruling 9).
        brokkr_protocol::hands::skip_boundary_proof(
            brokkr_protocol::hands::boundary_evidence_required(),
            "no namespace can be built here",
        );
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let tools = dir.path().join("home/.volta/bin");
    std::fs::create_dir_all(&tools).unwrap();
    let tool = tools.join("brokkr-218-fixture");
    std::fs::write(&tool, "#!/bin/sh\necho 'fixture 1.0.0'\n").unwrap();
    std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).unwrap();
    let work = dir.path().join("work");
    std::fs::create_dir_all(&work).unwrap();

    // The host PATH finds the fixture, which is the reading doctor used to
    // take and report as green. PATH is prepended, never replaced, and
    // restored before the box is opened, so no other test's lookup moves.
    let original = std::env::var_os("PATH");
    let mut search = tools.clone().into_os_string();
    if let Some(path) = &original {
        search.push(":");
        search.push(path);
    }
    std::env::set_var("PATH", &search);
    let on_host = tool_version("brokkr-218-fixture");
    match original {
        Some(path) => std::env::set_var("PATH", path),
        None => std::env::remove_var("PATH"),
    }
    assert_eq!(on_host, Some("fixture 1.0.0".into()));

    // ...but the gate's box binds no part of that home, so the same bare
    // name is not on the box's PATH and the version probe cannot answer.
    // This is the regression: before the fix doctor asked the host and
    // called the dialect green.
    let spec = brokkr_runtime::bundle::dialect_gate_hands();
    assert_eq!(probe_in_box(&spec, &work, "brokkr-218-fixture"), Ok(None));
    // The reachable arm, so both halves of the probe are exercised: a
    // binary the box does carry answers inside it.
    assert_eq!(
        probe_in_box(&spec, &work, "sh"),
        Ok(Some("POSIX shell".into()))
    );
}

/// The dialect's binary reaches `bash -lc` as one quoted word, so a name
/// carrying a metacharacter runs the tool and nothing else.
#[test]
fn a_dialect_binary_is_quoted_before_the_box_runs_it() {
    assert_eq!(shell_quote("openspec"), "'openspec'");
    assert_eq!(shell_quote("a; rm -rf /"), "'a; rm -rf /'");
    assert_eq!(shell_quote("it's"), "'it'\\''s'");
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
        safe_line(b"tool 1.0\x1b[31m\nforged line\n"),
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
            no_composite,
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

#[test]
fn the_dsh_composite_detail_reports_each_disposition() {
    let digest = "a".repeat(64);
    let other = "b".repeat(64);

    // No declared digest at all: informational, never a warning.
    let (warning, line) = composite_detail(None, false, Ok((&digest, "plugin")));
    assert!(!warning);
    assert!(line.contains("no declared wrapper_digest"), "{line}");
    assert!(line.contains("plugin"), "{line}");

    // A matching digest is informational even when the shape is supported.
    let (warning, line) = composite_detail(Some(&digest), true, Ok((&digest, "plugin")));
    assert!(!warning);
    assert!(
        line.contains("matches the declared wrapper_digest"),
        "{line}"
    );

    // A differing digest warns only when a supported shape declares one.
    let (warning, line) = composite_detail(Some(&digest), true, Ok((&other, "plugin")));
    assert!(warning);
    assert!(line.contains("differs from the declared"), "{line}");
    assert!(!composite_detail(Some(&digest), false, Ok((&other, "plugin"))).0);

    // An unreadable composite warns only when a supported shape declares
    // a digest; the detail names the unreadable component. It asserts
    // NO equality result — there is none to assert — but it keeps the
    // declaration context, which doctor knows either way.
    let (warning, line) =
        composite_detail(Some(&digest), true, Err("plugin component is unreadable"));
    assert!(warning);
    assert_eq!(
        line,
        format!(
            "composite unreadable: plugin component is unreadable \
             (declared wrapper_digest {digest}; comparison unavailable)"
        )
    );
    assert!(
        !line.contains("matches") && !line.contains("differs"),
        "{line}"
    );
    // The same wording without the warning for a shape that is not
    // `supported`, and the absent-declaration wording with neither.
    let (warning, other_line) =
        composite_detail(Some(&digest), false, Err("plugin component is unreadable"));
    assert!(!warning);
    assert_eq!(other_line, line);
    let (warning, line) = composite_detail(None, true, Err("plugin component is unreadable"));
    assert!(!warning);
    assert_eq!(
        line,
        "composite unreadable: plugin component is unreadable (no declared wrapper_digest)"
    );

    // The reason came from a filesystem, a lock file or a child process,
    // so a control byte in it is escaped rather than written to the
    // operator's terminal.
    let (_, line) = composite_detail(None, false, Err("pnpm lock is unreadable: \u{1b}[2J"));
    assert!(!line.contains('\u{1b}'), "{line}");
}

#[test]
fn doctor_appends_the_dsh_composite_detail_to_the_provider_line() {
    let dir = tempfile::tempdir().unwrap();
    fn fake_composite(adapter: &Adapter, probe: fn(&str) -> Option<String>) -> Observed {
        Observed {
            binary: adapter.binary.clone(),
            version: probe(&adapter.binary),
            warning: false,
            suffix: "composite deadbeef plugin feedface (no declared wrapper_digest)".to_string(),
            cause: None,
        }
    }
    let rendered = doctor_in(
        None,
        dir.path(),
        &workspace().join("agents"),
        &workspace().join("adapters"),
        &dir.path().join("secrets.env"),
        always_present,
        never_ambient,
        Boundary::Namespace,
        None,
        fake_composite,
    )
    .render();
    assert!(
        rendered.contains("ok       dsh:")
            && rendered.contains("composite deadbeef plugin feedface"),
        "{rendered}"
    );
}

/// The injected composite probe reporting a difference is a warning, not
/// just a suffix: `probe_providers` routes it through `report.warn`.
#[test]
fn doctor_warns_when_the_dsh_composite_differs_from_a_declared_digest() {
    let dir = tempfile::tempdir().unwrap();
    fn warning_composite(adapter: &Adapter, probe: fn(&str) -> Option<String>) -> Observed {
        Observed {
            binary: adapter.binary.clone(),
            version: probe(&adapter.binary),
            warning: true,
            suffix:
                "composite deadbeef plugin feedface (differs from the declared wrapper_digest feedface)"
                    .to_string(),
            cause: None,
        }
    }
    let rendered = doctor_in(
        None,
        dir.path(),
        &workspace().join("agents"),
        &workspace().join("adapters"),
        &dir.path().join("secrets.env"),
        always_present,
        never_ambient,
        Boundary::Namespace,
        None,
        warning_composite,
    )
    .render();
    assert!(
        rendered.contains("warn") && rendered.contains("dsh:"),
        "{rendered}"
    );
    assert!(
        rendered.contains("composite deadbeef plugin feedface"),
        "{rendered}"
    );
}

/// `dsh_provider_line` reads the shipped adapter's real seams; whether the
/// host has a DSH install or not, it answers without panicking and names
/// the executable the seam selected.
#[test]
fn dsh_provider_line_reads_the_real_adapter_and_its_seams() {
    let adapters = Adapters::load(&workspace().join("adapters")).unwrap();
    let adapter = adapters
        .providers()
        .find(|adapter| adapter.provider == "dsh")
        .expect("the shipped dsh adapter");
    let observed = dsh_provider_line(adapter, tool_version);
    // A host without DSH reports no version and no composite suffix; a
    // host with one reports both. Either way the two halves agree, and
    // the line names what the seam selected or what it looked for.
    match DshSeams::selected() {
        Ok(selection) => {
            assert_eq!(observed.binary, selection.executable);
            assert_eq!(observed.cause, None);
        }
        Err(unselected) => {
            assert_eq!(observed.binary, unselected.declared);
            assert_eq!(observed.version, None);
            assert_eq!(observed.cause, Some(unselected.cause.to_string()));
        }
    }
    assert_eq!(observed.version.is_some(), !observed.suffix.is_empty());
}

// `composite_identity`'s mapping is asserted in
// `the_dsh_seam_precedence_moves_the_version_and_the_composite_together`,
// against a REAL observation of a fixture installation. The inherited
// unit test here assembled a `DshComposite` field by field from chosen
// strings; the producer's members are private now, precisely so that no
// caller outside it can do that (council return 2026-09-19, F6).

/// The shipped DSH adapter with one named shape's identity replaced, so
/// each declaration disposition is a plain test.
#[cfg(test)]
fn dsh_adapter_declaring(identity: Option<ResumeIdentity>) -> Adapter {
    use brokkr_runtime::agents::{ResumeAssessment, ResumeEvidence, ResumeShape};
    use std::collections::BTreeMap;

    let adapters = Adapters::load(&workspace().join("adapters")).unwrap();
    let mut adapter = adapters
        .providers()
        .find(|adapter| adapter.provider == "dsh")
        .expect("the shipped dsh adapter")
        .clone();
    if let Some(identity) = identity {
        let mut shapes = BTreeMap::new();
        shapes.insert(
            "headless-work".to_string(),
            ResumeShape {
                status: ResumeStatus::Supported,
                identity,
                classes: vec!["work".into()],
                boundaries: vec!["not applicable".into()],
                hands: "none".into(),
                evidence: ResumeEvidence::default(),
                limitations: Vec::new(),
                reason: None,
            },
        );
        adapter.resume = ResumeAssessment::new(shapes);
    }
    adapter
}

/// A resolved home under a temporary root, so an injected `selected`
/// hands the producer a real `DshSeams` without touching the operator's.
#[cfg(test)]
fn seams_at(executable: &str, home: &Path) -> Result<DshSelection, DshUnselected> {
    Ok(DshSelection {
        executable: executable.to_string(),
        seams: Ok(DshSeams {
            executable: executable.to_string(),
            home: home.to_path_buf(),
        }),
    })
}

/// `dsh_provider_line`'s own arms: an Unknown identity declares no digest
/// and a readable composite becomes the detail. Both are driven over the
/// injected producer, because the real one needs a DSH install and a node
/// probe that a unit test must not require.
#[test]
fn dsh_provider_line_reports_an_unknown_identity_and_a_readable_composite() {
    let dir = tempfile::tempdir().unwrap();
    let digest = "a".repeat(64);

    // An Unknown identity carries no declared digest, so the readable
    // composite is informational and reports no comparison to make.
    let unknown = dsh_adapter_declaring(Some(ResumeIdentity::Unknown {
        reason: "nobody has identified this wrapper".into(),
    }));
    let observed = dsh_provider_line_with(
        &unknown,
        |_| Some("0.1.5-rc.2".to_string()),
        || seams_at("dsh", dir.path()),
        |_| Ok((digest.clone(), "plugin".into())),
    );
    assert!(!observed.warning, "{}", observed.suffix);
    assert!(
        observed.suffix.contains("no declared wrapper_digest"),
        "{}",
        observed.suffix
    );
    assert!(
        observed.suffix.contains(&digest) && observed.suffix.contains("plugin"),
        "{}",
        observed.suffix
    );

    // A Measured digest that matches the composite is informational too,
    // and names the match.
    let measured = dsh_adapter_declaring(Some(ResumeIdentity::Measured {
        version: "0.1.5-rc.2".into(),
        applies_to: "0.1.5-rc.2".into(),
        wrapper_digest: Some(digest.clone()),
    }));
    let observed = dsh_provider_line_with(
        &measured,
        |_| Some("0.1.5-rc.2".to_string()),
        || seams_at("dsh", dir.path()),
        |_| Ok((digest.clone(), "plugin".into())),
    );
    assert!(!observed.warning, "{}", observed.suffix);
    assert!(
        observed
            .suffix
            .contains("matches the declared wrapper_digest"),
        "{}",
        observed.suffix
    );

    // The shipped declaration, whatever it currently says, reaches the
    // same classifier: no panic and a composite suffix either way.
    let shipped = dsh_adapter_declaring(None);
    let observed = dsh_provider_line_with(
        &shipped,
        |_| Some("0.1.5-rc.2".to_string()),
        || seams_at("dsh", dir.path()),
        |_| Ok((digest.clone(), "plugin".into())),
    );
    assert!(observed.suffix.contains("composite"), "{}", observed.suffix);
}

/// Task 8.8(c)'s measured defect, closed. Doctor probed the bare declared
/// `adapter.binary` on PATH while the composite followed the adapter's
/// seam, so an override reported a version from one installation beside a
/// digest from another (proved both ways on 2026-09-19). Both halves now
/// come from ONE `DshSeams::selected`: the probe sees the selected
/// executable, and the producer receives that same resolution.
#[test]
fn the_dsh_version_and_composite_come_from_one_resolved_installation() {
    use std::cell::RefCell;

    let dir = tempfile::tempdir().unwrap();
    let adapter = dsh_adapter_declaring(None);
    let selected = dir.path().join("sentinel-dsh").display().to_string();
    assert_ne!(adapter.binary, selected, "the seam selects another install");

    let probed: RefCell<Vec<String>> = RefCell::new(Vec::new());
    let seen: RefCell<Option<PathBuf>> = RefCell::new(None);
    let observed = dsh_provider_line_with(
        &adapter,
        |binary| {
            probed.borrow_mut().push(binary.to_string());
            Some("0.1.5-rc.2 (sentinel)".to_string())
        },
        || seams_at(&selected, dir.path()),
        |seams| {
            *seen.borrow_mut() = Some(seams.home.clone());
            Ok(("canonical".into(), "plugin".into()))
        },
    );
    assert_eq!(
        probed.into_inner(),
        vec![selected.clone()],
        "the version probe reads the SELECTED executable exactly once, never 'dsh' on PATH"
    );
    assert_eq!(
        seen.into_inner().as_deref(),
        Some(dir.path()),
        "the producer receives the same resolution the probe used"
    );
    assert_eq!(observed.binary, selected);
    assert_eq!(observed.version.as_deref(), Some("0.1.5-rc.2 (sentinel)"));
    assert!(observed.suffix.contains("canonical"), "{}", observed.suffix);
}

/// A selected executable that does not answer is the whole of the line:
/// the warning names THAT executable, and no PATH decoy is tried in its
/// place — so the composite producer is never asked either.
#[test]
fn a_failed_selected_executable_is_never_retried_against_a_path_decoy() {
    let dir = tempfile::tempdir().unwrap();
    let adapter = dsh_adapter_declaring(None);
    let selected = dir.path().join("absent-dsh").display().to_string();
    let observed = dsh_provider_line_with(
        &adapter,
        |binary| match binary == adapter.binary {
            // The decoy the bare declared name WOULD have found.
            true => Some("0.0.0 (decoy on PATH)".to_string()),
            false => None,
        },
        || seams_at(&selected, dir.path()),
        |_| panic!("the composite is not computed for an executable that did not answer"),
    );
    assert_eq!(observed.binary, selected);
    assert_eq!(observed.version, None);
    assert!(!observed.warning);
    assert_eq!(observed.suffix, "");
}

/// The guide's composite wording is the classifier's own output, not a
/// transcription of it. A sample nobody checks drifts: the inherited one
/// had already become a digest no installation produced.
#[test]
fn the_guide_documents_the_wording_the_classifier_emits() {
    let guide = std::fs::read_to_string(workspace().join("docs/guides/provider-adapters.md"))
        .expect("the provider-adapters guide");
    let (_, unreadable) = composite_detail(
        Some("<digest>"),
        false,
        Err("pnpm lock is unreadable: pnpm lock exceeds 8388608-byte limit"),
    );
    assert_eq!(
        unreadable,
        "composite unreadable: pnpm lock is unreadable: pnpm lock exceeds 8388608-byte limit \
         (declared wrapper_digest <digest>; comparison unavailable)"
    );
    assert!(
        guide.contains(&unreadable),
        "the guide's unreadable sample is what doctor prints: {unreadable}"
    );
    // And the readable vocabulary the sample line beside it uses.
    let (_, matched) = composite_detail(None, false, Ok(("<digest>", "<plugin>")));
    assert!(
        guide.contains("(no declared wrapper_digest)"),
        "the guide's readable sample: {matched}"
    );
}

/// A minimal DSH installation at D6's locators under `root`, returning
/// the executable path. Enough for the sole producer to return a real
/// canonical digest, which is what makes the precedence control below a
/// comparison between two INSTALLATIONS rather than two strings.
#[cfg(test)]
fn install_dsh(root: &Path, version: &str) -> String {
    let put = |dir: &Path, name: &str, bytes: &[u8]| {
        let path = dir.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    };
    let core = root.join("core");
    let pkg = core.join("node_modules/@deepseek-ai/dsh");
    put(
        &pkg,
        "package.json",
        format!(
            r#"{{"name":"@deepseek-ai/dsh","version":"{version}","bin":{{"dsh":"lib/bin.js"}}}}"#
        )
        .as_bytes(),
    );
    put(&pkg, "lib/bin.js", b"#!/usr/bin/env node\n");
    put(
        &core,
        "node_modules/.package-lock.json",
        format!(
            r#"{{"lockfileVersion":3,"packages":{{"node_modules/@deepseek-ai/dsh":{{"version":"{version}","integrity":"sha512-CORE-{version}"}}}}}}"#
        )
        .as_bytes(),
    );
    let profile = root.join("home/profiles/headless");
    put(
        &profile,
        "package.json",
        br#"{"dsh":{"profile":{"bundles":["dsh-plugin-cli-session"],"patchReload":"startup"}}}"#,
    );
    put(&profile, "cordis.patch.yml", b"[]\n");
    put(
        &profile,
        "pnpm-lock.yaml",
        b"lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-D}\n",
    );
    let plugin = profile.join("node_modules/dsh-plugin-cli-session");
    for file in [
        "LICENSE",
        "README.md",
        "cordis.patch.yml",
        "lib/index.js",
        "lib/startup.js",
        "package.json",
    ] {
        put(&plugin, file, file.as_bytes());
    }
    let bin = pkg.join("lib/bin.js");
    // The PATH case resolves this file through a child's own rules, which
    // admit only an EXECUTABLE regular file.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    bin.canonicalize().unwrap().display().to_string()
}

/// The version a DSH installation records for ITSELF, read from the
/// package.json beside the selected executable.
///
/// This is doctor's probe under test conditions: a real observation of
/// the file the seam selected, not a fabricated constant. A constant
/// cannot tell two installations apart, so it cannot witness which one
/// the report described (council return 2026-09-19, F7).
#[cfg(test)]
fn recorded_version(binary: &str) -> Option<String> {
    let manifest = Path::new(binary).parent()?.parent()?.join("package.json");
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(manifest).ok()?).ok()?;
    Some(value.get("version")?.as_str()?.to_string())
}

/// Task 8.8(c), hermetically: the adapter seam's `BROKKR_DSH_BIN`, then
/// `FORGE_DSH_BIN`, then PATH precedence moves BOTH halves of the DSH
/// line together.
///
/// The environment is process-global, so each case runs in a re-executed
/// copy of this test binary rather than racing every other test here.
/// Each child installs three distinguishable DSH trees and asserts that
/// the version probe and the producer-derived composite both describe the
/// one the seam selected — the pairing the measured 2026-09-19 defect
/// broke.
///
/// Unix only: the fixture builds an executable `node` shim and a symlink
/// on the child's `PATH`, both of which are POSIX spellings. The
/// production seam is platform-correct; only this fixture is not.
#[cfg(unix)]
#[test]
fn the_dsh_seam_precedence_moves_the_version_and_the_composite_together() {
    const CASE: &str = "BROKKR_DOCTOR_SEAM_CASE";
    const CHOSEN: &str = "BROKKR_DOCTOR_SEAM_CHOSEN";

    const REJECTED: &str = "BROKKR_DOCTOR_SEAM_REJECTED";

    if let Ok(case) = std::env::var(CASE) {
        let chosen = std::env::var(CHOSEN).expect("the parent names the expected selection");
        let rejected = std::env::var(REJECTED).expect("the parent names the rejected install");
        let home = PathBuf::from(std::env::var_os("DSH_HOME").expect("the parent names the home"));
        let adapter = dsh_adapter_declaring(None);
        if case == "unreadable" {
            // The REAL entry point over a selected installation whose
            // profile is gone. The version probe still answers for that
            // executable, and the producer's refusal reaches the line
            // through production's OWN `map_err` — the closure an
            // injected failure can never run (council return
            // 2026-09-19, finding 5).
            let real = dsh_provider_line(&adapter, recorded_version);
            assert_eq!(real.binary, chosen, "the seam's choice");
            assert_eq!(
                real.version,
                recorded_version(&chosen),
                "the version survives an unreadable composite"
            );
            let refusal = dsh_composite(&DshSeams {
                executable: chosen.clone(),
                home: home.clone(),
            })
            .map(|_| unreachable!("the fixture install is broken on purpose"))
            .unwrap_err()
            .to_string();
            assert!(
                refusal.contains("profiles/headless"),
                "the refusal names the missing profile: {refusal}"
            );
            assert_eq!(
                real.suffix,
                format!(
                    "composite unreadable: {} (no declared wrapper_digest)",
                    Safe::new(&refusal).as_str()
                ),
                "the line carries the component's own reason and the declaration context"
            );
            assert!(
                !real.warning,
                "no shape declares a digest, so an unreadable composite stays informational"
            );
            return;
        }
        let probed = std::cell::RefCell::new(Vec::new());
        // The REAL seam resolution and the REAL producer. Only the
        // version probe is injected, and it is not a constant: it reads
        // the version out of the selected installation's own manifest,
        // so the reported version can only be the selected install's.
        let observed = dsh_provider_line_with(
            &adapter,
            |binary| {
                probed.borrow_mut().push(binary.to_string());
                recorded_version(binary)
            },
            DshSeams::selected,
            |seams| {
                dsh_composite(seams)
                    .map(composite_identity)
                    .map_err(|error| error.to_string())
            },
        );
        assert_eq!(observed.binary, chosen, "case {case}: the seam's choice");
        assert_eq!(
            probed.into_inner(),
            vec![chosen.clone()],
            "case {case}: the version probe reads the selected executable, once"
        );
        assert_eq!(
            observed.version,
            recorded_version(&chosen),
            "case {case}: the reported version is that executable's own"
        );
        assert_ne!(
            recorded_version(&chosen),
            recorded_version(&rejected),
            "case {case}: the two installs are distinguishable by version"
        );

        // The producer ran over the SAME resolution, and the child's
        // `PATH` holds a scripted `node`, so BOTH digests below are
        // deterministic and neither depends on this machine.
        let digest_of = |executable: &str| {
            let seams = DshSeams {
                executable: executable.to_string(),
                home: home.clone(),
            };
            let composite = dsh_composite(&seams).expect("the fixture install composes");
            // `composite_identity`'s own mapping, over a real
            // observation: the canonical digest first, the plugin
            // component second, and nothing else.
            assert_eq!(
                composite_identity(composite.clone()),
                (
                    composite.canonical().to_string(),
                    composite.plugin().to_string()
                )
            );
            assert_eq!(composite.canonical().len(), 64);
            assert_ne!(composite.canonical(), composite.plugin());
            (
                composite.canonical().to_string(),
                composite.plugin().to_string(),
            )
        };
        let (chosen_digest, chosen_plugin) = digest_of(&chosen);
        let (rejected_digest, _) = digest_of(&rejected);
        assert_ne!(
            chosen_digest, rejected_digest,
            "case {case}: the two installs have different composites"
        );
        let expected = format!(
            "composite {chosen_digest} plugin {chosen_plugin} (no declared wrapper_digest)"
        );
        assert_eq!(
            observed.suffix, expected,
            "case {case}: the reported composite is the SELECTED install's, exactly"
        );

        // And the REAL entry point over the same controlled environment:
        // production's own seam resolver and producer closure, not the
        // injected pair above. Only a fixture that installs a `node` and
        // a `dsh` of its own reaches this deterministically — which is
        // why the whole case runs in a child.
        let real = dsh_provider_line(&adapter, recorded_version);
        assert_eq!(real.binary, chosen, "case {case}: the real line's binary");
        assert_eq!(
            real.version,
            recorded_version(&chosen),
            "case {case}: the real line's version"
        );
        assert_eq!(real.suffix, expected, "case {case}: the real line's suffix");
        assert!(!real.warning, "case {case}: nothing is declared to differ");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let primary = install_dsh(&dir.path().join("primary"), "0.1.5-rc.2");
    let legacy = install_dsh(&dir.path().join("legacy"), "0.1.4");
    let on_path = install_dsh(&dir.path().join("pathwise"), "0.1.3");
    assert_ne!(primary, legacy);
    let home = dir.path().join("primary/home");

    // The child's whole `PATH`: a scripted `node` so the producer never
    // depends on this machine's runtime, and a `dsh` so the third case
    // selects a KNOWN installation instead of whatever the host happens
    // to have. Without this the `neither` case could only be asserted
    // conditionally, which is how a failing child passed for a reason
    // nobody read (council return 2026-09-19, F7).
    let shims = dir.path().join("shims");
    std::fs::create_dir_all(&shims).unwrap();
    std::fs::write(shims.join("node"), b"#!/bin/sh\necho v22.23.2\n").unwrap();
    std::os::unix::fs::symlink(&on_path, shims.join("dsh")).unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(shims.join("node"), std::fs::Permissions::from_mode(0o755))
            .unwrap();
    }

    // A home that exists but holds no `profiles/headless`, so the
    // producer refuses for a reason of its own while the selected
    // executable still answers with a version.
    let broken_home = dir.path().join("broken-home");
    std::fs::create_dir_all(&broken_home).unwrap();

    for (case, set_primary, set_legacy, chosen, rejected, dsh_home) in [
        // Primary wins over legacy.
        ("both", true, true, primary.as_str(), legacy.as_str(), &home),
        // Legacy alone is honoured.
        (
            "legacy-only",
            false,
            true,
            legacy.as_str(),
            primary.as_str(),
            &home,
        ),
        // Neither: the bare name the adapter declares, resolved on the
        // child's PATH — and reported as the FILE it resolved to.
        (
            "neither",
            false,
            false,
            on_path.as_str(),
            primary.as_str(),
            &home,
        ),
        // The same seam, over an installation the producer cannot read.
        (
            "unreadable",
            true,
            false,
            primary.as_str(),
            legacy.as_str(),
            &broken_home,
        ),
    ] {
        let mut child = std::process::Command::new(std::env::current_exe().unwrap());
        child
            .args([
                // The FULL libtest path. A bare function name matches
                // nothing under `--exact`, and a filter that matches
                // nothing exits zero — a child that ran no assertion at
                // all would make every case here pass vacuously.
                "doctor::tests::the_dsh_seam_precedence_moves_the_version_and_the_composite_together",
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(CASE, case)
            .env(CHOSEN, chosen)
            .env(REJECTED, rejected)
            .env("DSH_HOME", dsh_home)
            .env("PATH", &shims)
            .env_remove("BROKKR_DSH_BIN")
            .env_remove("FORGE_DSH_BIN");
        if set_primary {
            child.env("BROKKR_DSH_BIN", &primary);
        }
        if set_legacy {
            child.env("FORGE_DSH_BIN", &legacy);
        }
        // Only ETXTBSY is retried: a test that re-executes its own
        // binary can reach `exec` while another thread of this run still
        // holds a write descriptor to a file it staged, which is a fact
        // about the moment rather than about the seam (#255).
        let output = loop {
            match child.output() {
                Ok(output) => break output,
                Err(error) if error.raw_os_error() == Some(26) => {
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                Err(error) => panic!("case {case}: the child test binary runs: {error}"),
            }
        };
        let said = String::from_utf8_lossy(&output.stdout).into_owned()
            + &String::from_utf8_lossy(&output.stderr);
        // A filter that matches nothing exits ZERO. Without this the
        // whole control would pass while asserting nothing at all.
        assert!(
            said.contains("1 passed") || said.contains("1 failed"),
            "case {case}: the child ran the case, rather than filtering it away: {said}"
        );
        // Every case is now unconditional: the installs, the runtime and
        // the search are all the fixture's.
        assert!(output.status.success(), "case {case}: {said}");
    }
}

/// A failed HOME seam is a named composite failure, not evidence that the
/// binary is missing: the version the selected executable answered with
/// stays visible beside the unreadable reason (design D10's scenario).
#[test]
fn a_failed_home_seam_leaves_the_version_visible_beside_the_reason() {
    let digest = "a".repeat(64);
    let adapter = dsh_adapter_declaring(Some(ResumeIdentity::Measured {
        version: "0.1.5-rc.2".into(),
        applies_to: "0.1.5-rc.2".into(),
        wrapper_digest: Some(digest.clone()),
    }));
    let observed = dsh_provider_line_with(
        &adapter,
        |_| Some("0.1.5-rc.2".to_string()),
        || {
            Ok(DshSelection {
                executable: "/opt/dsh/lib/bin.js".to_string(),
                seams: Err(CompositeError::Config(
                    "no dsh home: set DSH_HOME or HOME".into(),
                )),
            })
        },
        |_| panic!("no seams, no producer call"),
    );
    assert_eq!(observed.version.as_deref(), Some("0.1.5-rc.2"));
    assert_eq!(observed.cause, None, "the executable WAS selected");
    assert!(observed.warning, "a supported declaration warns");
    assert_eq!(
        observed.suffix,
        format!(
            "composite unreadable: the DSH layout is unreadable: \
             no dsh home: set DSH_HOME or HOME \
             (declared wrapper_digest {digest}; comparison unavailable)"
        )
    );
}

/// A FAILED selection is the whole of the line, before anything runs:
/// the probe is never called, so nothing the declared spelling would
/// have found in a working directory can execute, and the cause the
/// lookup gave rides into the report beside that spelling. The
/// inherited seam handed the declared name to the probe after a failed
/// lookup, which under an absent `PATH` executed a cwd `dsh` (security
/// hold 2026-09-20, S1; the real-child regression is
/// `tests/doctor_dsh_selection.rs`).
#[test]
fn a_failed_selection_probes_nothing_and_carries_its_cause() {
    let adapter = dsh_adapter_declaring(None);
    let observed = dsh_provider_line_with(
        &adapter,
        |binary| panic!("a probe of '{binary}' after a failed selection"),
        || {
            Err(DshUnselected {
                declared: "dsh".to_string(),
                cause: CompositeError::Config("'dsh': PATH is absent".into()),
            })
        },
        |_| panic!("no selection, no producer call"),
    );
    assert_eq!(observed.binary, "dsh", "the spelling that was looked for");
    assert_eq!(observed.version, None);
    assert!(!observed.warning);
    assert_eq!(observed.suffix, "");
    assert_eq!(
        observed.cause.as_deref(),
        Some("the DSH layout is unreadable: 'dsh': PATH is absent"),
        "the lookup's own cause, not a generic not-found"
    );
    // The same over a supported, digest-declaring shape: a selection
    // failure is an availability fact and never a composite warning.
    let supported = dsh_adapter_declaring(Some(ResumeIdentity::Measured {
        version: "0.1.5-rc.2".into(),
        applies_to: "0.1.5-rc.2".into(),
        wrapper_digest: Some("a".repeat(64)),
    }));
    let observed = dsh_provider_line_with(
        &supported,
        |binary| panic!("a probe of '{binary}' after a failed selection"),
        || {
            Err(DshUnselected {
                declared: "/override/dsh".to_string(),
                cause: CompositeError::Config("/override/dsh: missing".into()),
            })
        },
        |_| panic!("no selection, no producer call"),
    );
    assert_eq!(observed.binary, "/override/dsh");
    assert!(!observed.warning);
    assert_eq!(
        observed.cause.as_deref(),
        Some("the DSH layout is unreadable: /override/dsh: missing")
    );
}

/// The unavailable line renders the selected spelling and the retained
/// cause through `Safe`, at their only interpolation. A nonexistent
/// override carrying a newline and a clear-screen sequence reached
/// stdout verbatim before (security hold 2026-09-20, S2); the real
/// built-binary assertion is in `tests/doctor_dsh_selection.rs`, and
/// this one drives the renderer's arm directly.
#[test]
fn the_unavailable_line_escapes_the_binary_and_the_selection_cause() {
    let dir = tempfile::tempdir().unwrap();
    fn injected(adapter: &Adapter, _: fn(&str) -> Option<String>) -> Observed {
        assert_eq!(adapter.provider, "dsh");
        Observed {
            binary: "/tmp/x\n\u{1b}[2Jdsh".to_string(),
            version: None,
            warning: false,
            suffix: String::new(),
            cause: Some("/tmp/x\n\u{1b}[2Jdsh: No such file\u{1b}[2J".to_string()),
        }
    }
    let rendered = doctor_in(
        None,
        dir.path(),
        &workspace().join("agents"),
        &workspace().join("adapters"),
        &dir.path().join("secrets.env"),
        always_present,
        never_ambient,
        Boundary::Namespace,
        None,
        injected,
    )
    .render();
    let line = rendered
        .lines()
        .find(|line| line.starts_with("warn     dsh:"))
        .unwrap_or_else(|| panic!("a dsh warning line: {rendered}"));
    assert!(
        line.contains("binary '/tmp/x[2Jdsh' not found: /tmp/x[2Jdsh: No such file[2J — seats"),
        "the escaped spelling and cause, in Safe's convention: {line}"
    );
    assert!(!rendered.contains('\u{1b}'), "{rendered}");
    assert!(
        !rendered.lines().any(|line| line.starts_with("\u{1b}[2J")),
        "no injected line: {rendered}"
    );
    // A provider whose probe failed with an executable in hand renders
    // no cause: the arm is the same, the suffix is not.
    fn selected_but_silent(adapter: &Adapter, _: fn(&str) -> Option<String>) -> Observed {
        Observed {
            binary: adapter.binary.clone(),
            version: None,
            warning: false,
            suffix: String::new(),
            cause: None,
        }
    }
    let rendered = doctor_in(
        None,
        dir.path(),
        &workspace().join("agents"),
        &workspace().join("adapters"),
        &dir.path().join("secrets.env"),
        always_present,
        never_ambient,
        Boundary::Namespace,
        None,
        selected_but_silent,
    )
    .render();
    assert!(
        rendered.contains("warn     dsh: binary 'dsh' not found — seats"),
        "{rendered}"
    );
}
