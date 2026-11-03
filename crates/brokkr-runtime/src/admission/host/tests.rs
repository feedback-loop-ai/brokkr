//! Where the host configuration is found, what the free-space probe
//! measures, and that the published `forge.host/v1` and `forge.host/v2`
//! contracts and the loader refuse and admit the same files.

use std::path::Path;

use serde_json::{json, Value};

use super::*;

#[test]
fn the_host_file_is_found_under_xdg_config_home_else_under_home() {
    let found = |xdg: Option<&str>, home: Option<&str>| {
        host_file(xdg.map(OsString::from), home.map(OsString::from))
            .map_err(|unplaced| unplaced.to_string())
    };
    let under_home = Ok(PathBuf::from("/h/.config/brokkr/host.json"));
    assert_eq!(
        found(Some("/x"), Some("/h")),
        Ok(PathBuf::from("/x/brokkr/host.json"))
    );
    for xdg in [None, Some(""), Some("relative")] {
        assert_eq!(found(xdg, Some("/h")), under_home, "{xdg:?}");
    }
    let unplaced = Err(
        "no host configuration can be found: neither XDG_CONFIG_HOME nor HOME \
                        names an absolute directory"
            .to_string(),
    );
    assert_eq!(found(None, Some("relative")), unplaced);
    assert_eq!(found(Some("relative"), None), unplaced);
}

#[test]
fn the_probe_measures_free_bytes_and_says_why_it_cannot() {
    let dir = tempfile::tempdir().unwrap();
    assert!(free_bytes(dir.path()).unwrap() > 0);
    // A scratch tree not made yet is measured on the filesystem that will
    // hold it, its deepest directory that is there.
    assert!(free_bytes(&dir.path().join(".forge/scratch")).unwrap() > 0);
    std::fs::write(dir.path().join("file"), "").unwrap();
    let under = free_bytes(&dir.path().join("file/scratch")).unwrap_err();
    assert_eq!(under.kind(), ErrorKind::NotADirectory);
    // A link that dangles or loops is there, and is never measured on its
    // parent's filesystem, whether it is the path or a directory on it.
    let link = |name: &str, to: &str| {
        let link = dir.path().join(name);
        std::os::unix::fs::symlink(to, &link).unwrap();
        link
    };
    let (dangling, looping) = (link("dangling", "gone"), link("loop", "loop"));
    let kind = |path: &Path| free_bytes(path).unwrap_err().kind();
    let refused = [
        kind(&dangling),
        kind(&dangling.join("scratch")),
        kind(&looping),
        kind(&looping.join("scratch")),
    ];
    let loops = std::fs::metadata(&looping).unwrap_err().kind();
    assert_eq!(
        refused,
        [ErrorKind::NotFound, ErrorKind::NotFound, loops, loops]
    );
    // A relative path with nothing at it is measured where it is named.
    let nowhere = Path::new("brokkr-no-such-dir/scratch");
    assert_eq!(kind(nowhere), ErrorKind::NotFound);
}

/// The sample host configuration the operator's guide shows.
fn guide_sample() -> Value {
    let guide = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/guides/read-surfaces.md");
    let text = std::fs::read_to_string(guide).unwrap();
    let (_, after) = text
        .split_once("```json\n{\n  \"schema\": \"forge.host/v1\"")
        .unwrap();
    let (rest, _) = after.split_once("```").unwrap();
    serde_json::from_str(&format!("{{\n  \"schema\": \"forge.host/v1\"{rest}")).unwrap()
}

#[test]
fn the_contract_and_the_loader_admit_and_refuse_the_same_files() {
    let contract =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/host.v1.schema.json");
    let schema: Value = serde_json::from_slice(&std::fs::read(contract).unwrap()).unwrap();
    let validator = jsonschema::draft7::new(&schema).unwrap();
    let minimal = json!({"schema": "forge.host/v1", "providers": {},
                         "floors": [{"place": "temp", "floor_bytes": 1}],
                         "boxed_builds": {"ceiling": 1}});
    let with = |pointer: &str, value: Value| {
        let mut host = minimal.clone();
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        host.pointer_mut(parent).unwrap()[key] = value;
        host
    };
    let floors = |floors: Value| with("/floors", floors);
    let floor = |floor: Value| floors(json!([floor]));
    let route = |route: Value| {
        with(
            "/providers/dsh",
            json!({"ceiling": 1, "routes": {"r": route}}),
        )
    };
    let admitted = [
        minimal.clone(),
        guide_sample(),
        route(json!({"ceiling": 4_294_967_295_u64, "class": "cloud"})),
        floor(json!({"path": "/var/tmp", "floor_bytes": u64::MAX})),
        floor(json!({"place": "seat-scratch", "floor_bytes": 1})),
        floors(json!([{"place": "temp", "floor_bytes": 1},
                      {"place": "seat-scratch", "floor_bytes": 1},
                      {"path": "/var/tmp", "floor_bytes": 1}])),
    ];
    let refused = [
        with("/cooldowns", json!({})),
        with("/containment", json!("best-effort")),
        with("/schema", json!("forge.host/v3")),
        with("/providers/claude", json!({"ceiling": 0})),
        with("/providers/claude", json!({"ceiling": -1})),
        with("/providers/claude", json!({"ceiling": 4_294_967_296_u64})),
        with("/providers/claude", json!({"ceiling": 1.5})),
        with("/providers/claude", json!({"ceiling": 1, "burst": 2})),
        with("/providers/claude", json!({})),
        route(json!({"ceiling": 1, "class": "local"})),
        route(json!({"ceiling": 1})),
        route(json!({"ceiling": 1, "class": "cloud", "timeout": 60})),
        floor(json!({"place": "temp", "floor_bytes": 0})),
        floor(json!({"path": "var/tmp", "floor_bytes": 1})),
        floor(json!({"path": null, "floor_bytes": 1})),
        floor(json!({"place": null, "floor_bytes": 1})),
        floor(json!({"place": "disk", "floor_bytes": 1})),
        floor(json!({"path": "/t", "place": "temp", "floor_bytes": 1})),
        floor(json!({"floor_bytes": 1})),
        floor(json!({"place": "temp"})),
        floor(json!({"place": "temp", "floor_bytes": 1, "size": 1})),
        floors(json!([])),
        floors(json!({"place": "temp", "floor_bytes": 1})),
        with("/scratch", json!({"floor_bytes": 1})),
        with("/boxed_builds/ceiling", json!(0)),
        with("/boxed_builds/cap", json!(1)),
        json!({"schema": "forge.host/v1", "providers": {}, "boxed_builds": {"ceiling": 1}}),
    ];
    let judged = |host: &Value| {
        let loaded = serde_json::from_value::<HostConfig>(host.clone()).is_ok();
        (validator.is_valid(host), loaded)
    };
    assert_eq!(admitted.each_ref().map(judged), [(true, true); 6]);
    assert_eq!(refused.each_ref().map(judged), [(false, false); 27]);

    // JSON Schema cannot say a key is written once, nor that two floors
    // name one place or path, and its validator reads the last copy of a
    // key: there the loader is stricter than the contract can be.
    let twice = written(r#""dsh": {"ceiling": 1}, "dsh": {"ceiling": 2}"#);
    let last: Value = serde_json::from_str(&twice).unwrap();
    let loaded = config(twice.as_bytes()).is_ok();
    assert_eq!((validator.is_valid(&last), loaded), (true, false));
    let repeated = [
        floors(json!([{"place": "temp", "floor_bytes": 1},
                      {"place": "temp", "floor_bytes": 2}])),
        floors(json!([{"path": "/var/tmp", "floor_bytes": 1},
                      {"path": "/var/tmp", "floor_bytes": 2}])),
    ];
    assert_eq!(repeated.each_ref().map(judged), [(true, false); 2]);
}

/// Decision 0071 rulings 8 and 9: the v1 refusal of `containment` is bound
/// to its typed cause before its words. A problem carrying the same text
/// must fail this case, so the agreement table's `(false, false)` is not its
/// proof.
#[test]
fn chief_v1_containment_refusal_keeps_typed_cause() {
    let host = json!({
        "schema": "forge.host/v1",
        "providers": {},
        "floors": [{"place": "temp", "floor_bytes": 1}],
        "boxed_builds": {"ceiling": 1},
        "containment": "best-effort",
    });
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("host.json");
    std::fs::write(&file, host.to_string()).unwrap();
    let refused = read(&file).unwrap_err();
    let HostError::Invalid { path, source } = &refused else {
        panic!("a v1 containment was not an invalid host: {refused:?}");
    };
    assert_eq!(path, &file);
    assert!(
        matches!(source, Problem::NotV2("containment")),
        "{source:?}"
    );
    assert_eq!(
        refused.to_string(),
        format!(
            "the host configuration at {} is not a valid forge.host/v1 file",
            file.display()
        )
    );
    assert_eq!(
        source.to_string(),
        "forge.host/v1 declares containment; only forge.host/v2 does"
    );
}

/// A minimal host configuration's text with `providers` written as given.
fn written(providers: &str) -> String {
    format!(
        r#"{{"schema": "forge.host/v1", "providers": {{{providers}}}, "floors": [{{"place": "temp", "floor_bytes": 1}}], "boxed_builds": {{"ceiling": 1}}}}"#
    )
}

#[test]
fn a_provider_or_a_route_written_twice_is_refused_naming_the_key() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("host.json");
    let refused = |providers: &str| {
        std::fs::write(&file, written(providers)).unwrap();
        match read(&file) {
            Err(HostError::Invalid {
                path,
                source: Problem::Repeated(said),
                ..
            }) if path == file => said,
            other => panic!("not refused as written twice: {other:?}"),
        }
    };
    let routes = r#""routes": {"spark-glm": {"ceiling": 1, "class": "shared-local"}}"#;
    let provider = format!(r#""dsh": {{"ceiling": 1, {routes}}}, "dsh": {{"ceiling": 9}}"#);
    let said = "key 'dsh' is written twice at line 1 column 153";
    assert_eq!(refused(&provider), said);
    let route = r#""cloud": {"ceiling": 1, "class": "cloud"}"#;
    let routed = format!(r#""alpha": {{"ceiling": 1, "routes": {{{route}, {route}}}}}"#);
    let said = "key 'cloud' is written twice at line 1 column 162";
    assert_eq!(refused(&routed), said);
}

/// Decision 0082 ruling 4: the `forge.host/v1` row names this crate's private
/// reader, and the reader admits an instance of the version.
#[test]
fn the_lifecycle_row_names_the_reader_and_a_v1_file_loads() {
    let row = crate::contract_lifecycle::row("contracts/host.v1.schema.json");
    assert_eq!(row.version, "forge.host/v1");
    assert_eq!(row.reader, "load brokkr_runtime::admission::host::read");
    let host = file_loaded(
        r#"{"schema":"forge.host/v1","providers":{},"floors":[{"place":"temp","floor_bytes":1}],"boxed_builds":{"ceiling":1}}"#,
    );
    assert_eq!(host.providers.len(), 0);
}

/// The capacity block every v2 test below reads, with two declared
/// providers and two routes on `dsh`.
const PROVIDERS: &str = r#""dsh":{"ceiling":1,"routes":{"r":{"ceiling":1,"class":"cloud"},"s":{"ceiling":1,"class":"cloud"}}},"codex":{"ceiling":1}"#;

/// A `forge.host/v2` file with the capacity block above and `extra`, the
/// cooldowns or containment block as the test writes it.
fn v2(extra: &str) -> String {
    format!(
        r#"{{"schema":"forge.host/v2","providers":{{{PROVIDERS}}},"floors":[{{"place":"temp","floor_bytes":1}}],"boxed_builds":{{"ceiling":1}}{extra}}}"#
    )
}

/// A v2 file carrying one cooldowns block.
fn cooled(block: &str) -> String {
    v2(&format!(r#","cooldowns":{block}"#))
}

/// A file the loader reads.
fn loaded(text: &str) -> HostConfig {
    config(text.as_bytes()).unwrap_or_else(|problem| panic!("{text} was refused: {problem}"))
}

/// A host file written with `text` and read through the loader's path.
fn file_loaded(text: &str) -> HostConfig {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("host.json");
    std::fs::write(&file, text).unwrap();
    match read(&file).unwrap() {
        Hosting::Declared(host) => host,
        other => panic!("a host file did not load: {other:?}"),
    }
}

/// A file the loader refuses, and why.
fn refused(text: &str) -> Problem {
    match config(text.as_bytes()) {
        Err(problem) => problem,
        Ok(_) => panic!("the loader admitted {text}"),
    }
}

/// A site's inherited bounds, as the loader would hand them on.
fn bounds(host: &HostConfig, provider: &str, route: Option<&str>) -> Option<(u32, u32)> {
    host.cooldown(provider, route)
        .map(|bounds| (bounds.default_seconds.get(), bounds.max_seconds.get()))
}

#[test]
fn a_v1_file_reads_as_no_cool_downs() {
    let text = r#"{"schema":"forge.host/v1","providers":{"dsh":{"ceiling":1,"routes":{"r":{"ceiling":1,"class":"cloud"}}}},"floors":[{"place":"temp","floor_bytes":1}],"boxed_builds":{"ceiling":1}}"#;
    let host = loaded(text);
    assert_eq!(bounds(&host, "dsh", None), None);
    assert_eq!(bounds(&host, "dsh", Some("r")), None);
    assert_eq!(bounds(&host, "codex", None), None);
}

#[test]
fn a_v2_block_reads_and_a_route_and_provider_inherit_what_they_leave_out() {
    let block = r#"{"default_seconds":10,"max_seconds":60,"providers":{"dsh":{"default_seconds":20,"routes":{"r":{"max_seconds":30}}}}}"#;
    let host = loaded(&cooled(block));
    // The provider overrides its default, and takes the block's maximum.
    assert_eq!(bounds(&host, "dsh", None), Some((20, 60)));
    // The route overrides its maximum, and takes its provider's default.
    assert_eq!(bounds(&host, "dsh", Some("r")), Some((20, 30)));
    // A route with nothing of its own takes its provider.
    assert_eq!(bounds(&host, "dsh", Some("s")), Some((20, 60)));
    // A provider with nothing of its own takes the block.
    assert_eq!(bounds(&host, "codex", None), Some((10, 60)));
}

#[test]
fn a_v2_cool_down_outside_its_limits_refuses_naming_the_site_and_field() {
    let cases = [
        (
            cooled(r#"{"default_seconds":300,"max_seconds":7201}"#),
            "the cool-down max_seconds of the cooldowns block is 7201, past the 7200-second maximum",
        ),
        (
            cooled(r#"{"default_seconds":20,"max_seconds":10}"#),
            "the cool-down default_seconds of the cooldowns block is 20, above its max_seconds 10",
        ),
        (
            cooled(r#"{"default_seconds":0,"max_seconds":10}"#),
            "the cool-down default_seconds of the cooldowns block is zero",
        ),
        (
            cooled(r#"{"default_seconds":10,"max_seconds":0}"#),
            "the cool-down max_seconds of the cooldowns block is zero",
        ),
        (
            cooled(
                r#"{"default_seconds":10,"max_seconds":60,"providers":{"claude":{}}}"#,
            ),
            "the cool-down provider 'claude' is not declared under providers",
        ),
        (
            cooled(
                r#"{"default_seconds":10,"max_seconds":60,"providers":{"dsh":{"routes":{"second":{}}}}}"#,
            ),
            "the cool-down route 'second' of provider 'dsh' is not declared under providers",
        ),
        (
            cooled(
                r#"{"default_seconds":10,"max_seconds":60,"providers":{"dsh":{"max_seconds":30,"routes":{"r":{"default_seconds":40}}}}}"#,
            ),
            "the cool-down default_seconds of route 'r' of provider 'dsh' is 40, above its max_seconds 30",
        ),
    ];
    for (text, said) in cases {
        assert_eq!(refused(&text).to_string(), said, "{text}");
    }
}

/// A provider's own numbers are held to the block's limits too, and its
/// refusal names the provider, as the block's and a route's name theirs.
#[test]
fn a_providers_cool_down_outside_its_limits_refuses_naming_the_provider() {
    let cases = [
        (
            cooled(
                r#"{"default_seconds":10,"max_seconds":60,"providers":{"dsh":{"default_seconds":0}}}"#,
            ),
            "the cool-down default_seconds of provider 'dsh' is zero",
        ),
        (
            cooled(
                r#"{"default_seconds":10,"max_seconds":60,"providers":{"dsh":{"max_seconds":7201}}}"#,
            ),
            "the cool-down max_seconds of provider 'dsh' is 7201, past the 7200-second maximum",
        ),
    ];
    for (text, said) in cases {
        assert_eq!(refused(&text).to_string(), said, "{text}");
    }
}

/// A v1 file that declares a cooldowns block is refused by its typed
/// cause: cool-downs are v2's (decision 0103 ruling 3).
#[test]
fn a_v1_file_declaring_cool_downs_is_refused_as_not_v2() {
    let text = format!(
        r#"{{"schema":"forge.host/v1","providers":{{{PROVIDERS}}},"floors":[{{"place":"temp","floor_bytes":1}}],"boxed_builds":{{"ceiling":1}},"cooldowns":{{"default_seconds":10,"max_seconds":60}}}}"#
    );
    let problem = refused(&text);
    assert!(
        matches!(problem, Problem::NotV2("cooldowns")),
        "{problem:?}"
    );
    assert_eq!(
        problem.to_string(),
        "forge.host/v1 declares cooldowns; only forge.host/v2 does"
    );
}

#[test]
fn a_v2_cool_down_block_refuses_an_unknown_key() {
    let text = cooled(r#"{"default_seconds":10,"max_seconds":60,"pause":1}"#);
    let Problem::Shape(error) = refused(&text) else {
        panic!("an unknown key was not read as the contract's shape");
    };
    assert_eq!(
        error.to_string(),
        "unknown field `pause`, expected one of `default_seconds`, `max_seconds`, `providers` at line 1 column 292"
    );
    let mut host: Value = serde_json::from_str(&text).unwrap();
    host["cooldowns"].as_object_mut().unwrap().remove("pause");
    host["cooldowns"]["routes"] = json!({"r": {"wait": 1}});
    assert!(serde_json::from_value::<HostConfig>(host).is_err());
}

#[test]
fn a_v2_containment_is_best_effort_unless_required() {
    assert_eq!(
        loaded(&v2(r#","containment":"best-effort""#)).cooldown("dsh", None),
        None
    );
    assert_eq!(
        refused(&v2(r#","containment":"required""#)).to_string(),
        "containment required is decision 0080's, not built yet (#472)"
    );
    let Problem::Shape(error) = refused(&v2(r#","containment":"strict""#)) else {
        panic!("an unknown containment value was not read as the contract's shape");
    };
    assert!(
        error.to_string().contains("unknown variant `strict`"),
        "{error}"
    );
}

#[test]
fn the_v2_contract_and_the_loader_admit_and_refuse_the_same_shape() {
    let contract =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/host.v2.schema.json");
    let schema: Value = serde_json::from_slice(&std::fs::read(contract).unwrap()).unwrap();
    let validator = jsonschema::draft7::new(&schema).unwrap();
    let minimal = json!({"schema": "forge.host/v2", "providers": {},
                         "floors": [{"place": "temp", "floor_bytes": 1}],
                         "boxed_builds": {"ceiling": 1}});
    assert!(validator.is_valid(&minimal), "a minimal v2 host");
    let full = cooled(
        r#"{"default_seconds":10,"max_seconds":60,"providers":{"dsh":{"routes":{"r":{"max_seconds":30}}}}}"#,
    );
    assert!(validator.is_valid(&serde_json::from_str::<Value>(&full).unwrap()));
    for (pointer, value) in [
        ("/containment", json!("best-effort")),
        // `required` is a legal v2 value; the loader refuses it because
        // decision 0080 is not built, which the contract cannot say.
        ("/containment", json!("required")),
    ] {
        let mut host = minimal.clone();
        host[pointer.trim_start_matches('/')] = value;
        assert!(validator.is_valid(&host), "{pointer}");
    }
    for (pointer, value) in [
        ("/cooldowns", json!({})),
        (
            "/cooldowns",
            json!({"default_seconds": 10, "max_seconds": 7201}),
        ),
        (
            "/cooldowns",
            json!({"default_seconds": 0, "max_seconds": 1}),
        ),
        (
            "/cooldowns",
            json!({"default_seconds": 1, "max_seconds": 1, "pause": 1}),
        ),
        ("/containment", json!("strict")),
    ] {
        let mut host = minimal.clone();
        host[pointer.trim_start_matches('/')] = value;
        assert!(!validator.is_valid(&host), "{pointer}");
    }
}

/// H1: a present `null` is a value written empty, never an absent
/// declaration. Every null site the published contract refuses, and the two
/// v2-only keys a v1 file may not hold, refuses through the loader too; one
/// table drives both, so schema and loader agree by construction.
#[test]
fn chief_null_declarations_are_refused_by_reader_and_schema() {
    let contract = |name: &str| {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../contracts")
            .join(name);
        serde_json::from_slice::<Value>(&std::fs::read(path).unwrap()).unwrap()
    };
    let valid =
        |schema: &Value, host: &Value| jsonschema::draft7::new(schema).unwrap().is_valid(host);
    let base = |schema: &str| {
        json!({
            "schema": schema,
            "providers": {
                "dsh": {"ceiling": 1, "routes": {"r": {"ceiling": 1, "class": "cloud"}}},
                "codex": {"ceiling": 1}
            },
            "floors": [{"place": "temp", "floor_bytes": 1}],
            "boxed_builds": {"ceiling": 1}
        })
    };
    let (v1_host, v2_host) = (base("forge.host/v1"), base("forge.host/v2"));
    let insert = |mut host: Value, key: &str, value: Value| {
        host.as_object_mut().unwrap().insert(key.to_owned(), value);
        host
    };
    let set = |mut host: Value, pointer: &str, value: Value| {
        *host.pointer_mut(pointer).unwrap() = value;
        host
    };
    let block = insert(
        v2_host.clone(),
        "cooldowns",
        json!({"default_seconds": 10, "max_seconds": 60, "providers": {}}),
    );
    let with_providers = |providers: Value| set(block.clone(), "/cooldowns/providers", providers);
    let cases = [
        (
            "v1 cooldowns:null",
            "host.v1.schema.json",
            insert(v1_host.clone(), "cooldowns", Value::Null),
        ),
        (
            "v1 containment:null",
            "host.v1.schema.json",
            insert(v1_host.clone(), "containment", Value::Null),
        ),
        (
            "v2 cooldowns:null",
            "host.v2.schema.json",
            insert(v2_host.clone(), "cooldowns", Value::Null),
        ),
        (
            "v2 containment:null",
            "host.v2.schema.json",
            insert(v2_host.clone(), "containment", Value::Null),
        ),
        (
            "v2 provider default_seconds:null",
            "host.v2.schema.json",
            with_providers(json!({"dsh": {"default_seconds": null}})),
        ),
        (
            "v2 provider max_seconds:null",
            "host.v2.schema.json",
            with_providers(json!({"dsh": {"max_seconds": null}})),
        ),
        (
            "v2 route default_seconds:null",
            "host.v2.schema.json",
            with_providers(json!({"dsh": {"routes": {"r": {"default_seconds": null}}}})),
        ),
        (
            "v2 route max_seconds:null",
            "host.v2.schema.json",
            with_providers(json!({"dsh": {"routes": {"r": {"max_seconds": null}}}})),
        ),
    ];
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("host.json");
    let mut admitted = Vec::new();
    for (name, contract_name, host) in cases {
        assert!(
            !valid(&contract(contract_name), &host),
            "{name}: the schema admitted a present null"
        );
        std::fs::write(&file, host.to_string()).unwrap();
        match read(&file) {
            Err(HostError::Invalid {
                source: Problem::Shape(_),
                ..
            }) => {}
            Err(other) => panic!("{name}: not a shape refusal: {other:?}"),
            Ok(_) => admitted.push(name.to_owned()),
        }
    }
    assert_eq!(
        admitted,
        Vec::<String>::new(),
        "a present null read as absent"
    );
}

/// L3: a refusal names the version the file itself claims. A v2 cool-down
/// past its cap says `forge.host/v2`, not v1, and keeps the source's words.
#[test]
fn chief_v2_refusal_uses_a_correct_version_label() {
    let text = cooled(r#"{"default_seconds":10,"max_seconds":7201}"#);
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("host.json");
    std::fs::write(&file, text).unwrap();
    let error = read(&file).unwrap_err();
    assert_eq!(
        error.to_string(),
        format!(
            "the host configuration at {} is not a valid forge.host/v2 file",
            file.display()
        )
    );
    let HostError::Invalid { source, .. } = error else {
        panic!("a v2 cap refusal was not an invalid host: {error:?}");
    };
    assert!(
        matches!(
            &source,
            Problem::AboveMaximum {
                value: 7201,
                maximum: 7200,
                ..
            }
        ),
        "{source:?}"
    );
    assert_eq!(
        source.to_string(),
        "the cool-down max_seconds of the cooldowns block is 7201, past the 7200-second maximum"
    );
}

/// Every problem the loader raises names the contract that owns it: v2
/// where only v2 can raise it, v1 for a v2-only key in v1, and the
/// version-neutral name where either version's shape may be wrong.
#[test]
fn a_refusals_version_is_its_problems_own() {
    let malformed = serde_json::from_str::<Value>("").unwrap_err();
    let cases = [
        (Problem::Shape(malformed), "host configuration"),
        (Problem::Repeated(String::new()), "host configuration"),
        (Problem::NotV2("cooldowns"), "forge.host/v1"),
        (Problem::ContainmentRequired, "forge.host/v2"),
        (
            Problem::Zero {
                site: String::new(),
                field: "default_seconds",
            },
            "forge.host/v2",
        ),
        (
            Problem::AboveMaximum {
                site: String::new(),
                value: 7201,
                maximum: 7200,
            },
            "forge.host/v2",
        ),
        (
            Problem::DefaultAboveMaximum {
                site: String::new(),
                default: 20,
                max: 10,
            },
            "forge.host/v2",
        ),
        (Problem::ProviderUndeclared(String::new()), "forge.host/v2"),
        (
            Problem::RouteUndeclared {
                provider: String::new(),
                route: String::new(),
            },
            "forge.host/v2",
        ),
    ];
    for (problem, contract) in cases {
        assert_eq!(problem.contract(), contract, "{problem:?}");
    }
}

/// Decision 0103 ruling 3's `forge.host/v2` row names this crate's private
/// reader, and the reader admits an instance of the version.
#[test]
fn the_lifecycle_row_names_the_reader_and_a_v2_file_loads() {
    let row = crate::contract_lifecycle::row("contracts/host.v2.schema.json");
    assert_eq!(row.version, "forge.host/v2");
    assert_eq!(row.reader, "load brokkr_runtime::admission::host::read");
    let host = file_loaded(
        r#"{"schema":"forge.host/v2","providers":{},"floors":[{"place":"temp","floor_bytes":1}],"boxed_builds":{"ceiling":1},"cooldowns":{"default_seconds":10,"max_seconds":60}}"#,
    );
    assert_eq!(
        host.cooldown("dsh", None),
        Some(Bounds {
            default_seconds: NonZeroU32::new(10).unwrap(),
            max_seconds: NonZeroU32::new(60).unwrap(),
        })
    );
}
