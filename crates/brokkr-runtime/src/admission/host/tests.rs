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

/// The published contract `name`, judged by its own JSON Schema.
fn contract_validator(name: &str) -> jsonschema::Validator {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../contracts")
        .join(name);
    let schema: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    jsonschema::draft7::new(&schema).unwrap()
}

#[test]
fn the_contract_and_the_loader_admit_and_refuse_the_same_files() {
    let validator = contract_validator("host.v1.schema.json");
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
        let bytes = serde_json::to_vec(host).unwrap();
        let loaded = serde_json::from_slice::<HostConfig>(&bytes).is_ok();
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
    let (_dir, file, read) = host_read(&host.to_string());
    let refused = read.unwrap_err();
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

/// A minimal host in `schema` whose one floor is the exact token `number`.
fn floor_token(schema: &str, number: &str) -> String {
    format!(
        r#"{{"schema":"{schema}","providers":{{}},"floors":[{{"place":"temp","floor_bytes":{number}}}],"boxed_builds":{{"ceiling":1}}}}"#
    )
}

/// A minimal host in `schema` whose `dsh` ceiling is the exact token
/// `number`.
fn ceiling_token(schema: &str, number: &str) -> String {
    format!(
        r#"{{"schema":"{schema}","providers":{{"dsh":{{"ceiling":{number}}}}},"floors":[{{"place":"temp","floor_bytes":1}}],"boxed_builds":{{"ceiling":1}}}}"#
    )
}

/// A complete capacity block for either schema: two declared providers,
/// two routes on `dsh`, one floor and a boxed-build ceiling.
fn capacity(schema: &str) -> Value {
    json!({
        "schema": schema,
        "providers": {
            "dsh": {"ceiling": 1, "routes": {
                "r": {"ceiling": 1, "class": "cloud"},
                "s": {"ceiling": 1, "class": "cloud"}
            }},
            "codex": {"ceiling": 1}
        },
        "floors": [{"place": "temp", "floor_bytes": 1}],
        "boxed_builds": {"ceiling": 1}
    })
}

/// `host` with the JSON pointer `pointer` set to `value`; a top-level key
/// the host does not hold yet is inserted.
fn set(mut host: Value, pointer: &str, value: Value) -> Value {
    match host.pointer_mut(pointer) {
        Some(slot) => *slot = value,
        None => {
            host.as_object_mut()
                .unwrap()
                .insert(pointer.trim_start_matches('/').to_owned(), value);
        }
    }
    host
}

/// A file the loader reads.
fn loaded(text: &str) -> HostConfig {
    config(text.as_bytes()).unwrap_or_else(|problem| panic!("{text} was refused: {problem}"))
}

/// A host file written with `text` in a directory of its own, read
/// through the loader's path: the directory, the file, and what was read.
fn host_read(text: &str) -> (tempfile::TempDir, PathBuf, Result<Hosting, HostError>) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("host.json");
    std::fs::write(&file, text).unwrap();
    let read = read(&file);
    (dir, file, read)
}

/// A host file written with `text` and read through the loader's path.
fn file_loaded(text: &str) -> HostConfig {
    match host_read(text).2.unwrap() {
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

/// Decision 0071 rulings 8 and 9: a refusal's typed cause and its fields,
/// then its exact words. A problem carrying the same text but another
/// variant fails here.
fn refused_as(text: &str, cause: impl FnOnce(&Problem) -> bool, said: &str) {
    let problem = refused(text);
    assert!(cause(&problem), "{text}: expected {said}, got {problem:?}");
    assert_eq!(problem.to_string(), said, "{text}");
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

/// Decision 0071 ruling 3 and task judgment 1: JSON Schema's `integer`
/// admits a number with no fractional part, so `10.0` and `6e1` pass the v2
/// contract and the loader reads the same bounds instead of refusing the
/// shape. Each of the six new numeric fields is reached.
#[test]
fn schema_integral_notation_reads_as_the_same_bounds() {
    let validator = contract_validator("host.v2.schema.json");
    let cases = [
        (
            "block default_seconds",
            r#"{"default_seconds":10.0,"max_seconds":60}"#,
            None,
            (10, 60),
        ),
        (
            "block max_seconds",
            r#"{"default_seconds":10,"max_seconds":6e1}"#,
            None,
            (10, 60),
        ),
        (
            "provider default_seconds",
            r#"{"default_seconds":10,"max_seconds":60,"providers":{"dsh":{"default_seconds":2e1}}}"#,
            Some("dsh"),
            (20, 60),
        ),
        (
            "provider max_seconds",
            r#"{"default_seconds":10,"max_seconds":60,"providers":{"dsh":{"max_seconds":30.0}}}"#,
            Some("dsh"),
            (10, 30),
        ),
        (
            "route default_seconds",
            r#"{"default_seconds":10,"max_seconds":60,"providers":{"dsh":{"default_seconds":20,"routes":{"r":{"default_seconds":3.0e1}}}}}"#,
            Some("r"),
            (30, 60),
        ),
        (
            "route max_seconds",
            r#"{"default_seconds":10,"max_seconds":60,"providers":{"dsh":{"routes":{"r":{"default_seconds":20,"max_seconds":45.0}}}}}"#,
            Some("r"),
            (20, 45),
        ),
    ];
    for (name, block, route, expected) in cases {
        let text = cooled(block);
        let host: Value = serde_json::from_str(&text).unwrap();
        assert!(
            validator.is_valid(&host),
            "{name}: the schema refused integral notation"
        );
        let host = loaded(&text);
        assert_eq!(bounds(&host, "dsh", route), Some(expected), "{name}");
    }
}

#[test]
fn a_v2_cool_down_outside_its_limits_refuses_naming_the_site_and_field() {
    refused_as(
        &cooled(r#"{"default_seconds":300,"max_seconds":7201}"#),
        |problem| {
            matches!(
                problem,
                Problem::AboveMaximum { site, value, maximum }
                    if site == "the cooldowns block" && *value == 7201 && *maximum == 7200
            )
        },
        "the cool-down max_seconds of the cooldowns block is 7201, past the 7200-second maximum",
    );
    refused_as(
        &cooled(r#"{"default_seconds":20,"max_seconds":10}"#),
        |problem| {
            matches!(
                problem,
                Problem::DefaultAboveMaximum { site, default, max }
                    if site == "the cooldowns block" && *default == 20 && *max == 10
            )
        },
        "the cool-down default_seconds of the cooldowns block is 20, above its max_seconds 10",
    );
    refused_as(
        &cooled(r#"{"default_seconds":0,"max_seconds":10}"#),
        |problem| {
            matches!(
                problem,
                Problem::Zero { site, field }
                    if site == "the cooldowns block" && *field == "default_seconds"
            )
        },
        "the cool-down default_seconds of the cooldowns block is zero",
    );
    refused_as(
        &cooled(r#"{"default_seconds":10,"max_seconds":0}"#),
        |problem| {
            matches!(
                problem,
                Problem::Zero { site, field }
                    if site == "the cooldowns block" && *field == "max_seconds"
            )
        },
        "the cool-down max_seconds of the cooldowns block is zero",
    );
    refused_as(
        &cooled(r#"{"default_seconds":10,"max_seconds":60,"providers":{"claude":{}}}"#),
        |problem| matches!(problem, Problem::ProviderUndeclared(provider) if provider == "claude"),
        "the cool-down provider 'claude' is not declared under providers",
    );
    refused_as(
        &cooled(
            r#"{"default_seconds":10,"max_seconds":60,"providers":{"dsh":{"routes":{"second":{}}}}}"#,
        ),
        |problem| {
            matches!(
                problem,
                Problem::RouteUndeclared { provider, route }
                    if provider == "dsh" && route == "second"
            )
        },
        "the cool-down route 'second' of provider 'dsh' is not declared under providers",
    );
    refused_as(
        &cooled(
            r#"{"default_seconds":10,"max_seconds":60,"providers":{"dsh":{"max_seconds":30,"routes":{"r":{"default_seconds":40}}}}}"#,
        ),
        |problem| {
            matches!(
                problem,
                Problem::DefaultAboveMaximum { site, default, max }
                    if site == "route 'r' of provider 'dsh'"
                        && *default == 40
                        && *max == 30
            )
        },
        "the cool-down default_seconds of route 'r' of provider 'dsh' is 40, above its max_seconds 30",
    );
}

/// A site's inherited default is held to its own maximum, as its own
/// default is held to an inherited maximum.
#[test]
fn an_inherited_cool_down_default_outside_its_limits_refuses_naming_the_site() {
    refused_as(
        &cooled(
            r#"{"default_seconds":40,"max_seconds":60,"providers":{"dsh":{"max_seconds":30}}}"#,
        ),
        |problem| {
            matches!(
                problem,
                Problem::DefaultAboveMaximum { site, default, max }
                    if site == "provider 'dsh'" && *default == 40 && *max == 30
            )
        },
        "the cool-down default_seconds of provider 'dsh' is 40, above its max_seconds 30",
    );
    let text = cooled(
        r#"{"default_seconds":10,"max_seconds":60,"providers":{"dsh":{"default_seconds":50,"routes":{"r":{"max_seconds":40}}}}}"#,
    );
    match refused(&text) {
        Problem::DefaultAboveMaximum { site, default, max } => {
            assert_eq!(site, "route 'r' of provider 'dsh'");
            assert_eq!((default, max), (50, 40));
        }
        other => panic!("a route's inherited default was not held to its maximum: {other:?}"),
    }
    assert_eq!(
        refused(&text).to_string(),
        "the cool-down default_seconds of route 'r' of provider 'dsh' is 50, above its max_seconds 40"
    );
}

/// A provider's own numbers are held to the block's limits too, and its
/// refusal names the provider, as the block's and a route's name theirs.
#[test]
fn a_providers_cool_down_outside_its_limits_refuses_naming_the_provider() {
    refused_as(
        &cooled(
            r#"{"default_seconds":10,"max_seconds":60,"providers":{"dsh":{"default_seconds":0}}}"#,
        ),
        |problem| {
            matches!(
                problem,
                Problem::Zero { site, field }
                    if site == "provider 'dsh'" && *field == "default_seconds"
            )
        },
        "the cool-down default_seconds of provider 'dsh' is zero",
    );
    refused_as(
        &cooled(
            r#"{"default_seconds":10,"max_seconds":60,"providers":{"dsh":{"max_seconds":7201}}}"#,
        ),
        |problem| {
            matches!(
                problem,
                Problem::AboveMaximum { site, value, maximum }
                    if site == "provider 'dsh'" && *value == 7201 && *maximum == 7200
            )
        },
        "the cool-down max_seconds of provider 'dsh' is 7201, past the 7200-second maximum",
    );
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
    let text = cooled(
        r#"{"default_seconds":10,"max_seconds":60,"providers":{"dsh":{"routes":{"r":{"wait":1}}}}}"#,
    );
    let Problem::Shape(error) = refused(&text) else {
        panic!("an unknown route key was not read as the contract's shape");
    };
    assert_eq!(
        error.to_string(),
        "unknown field `wait`, expected `default_seconds` or `max_seconds` at line 1 column 326"
    );
}

#[test]
fn a_v2_containment_is_best_effort_unless_required() {
    assert_eq!(
        loaded(&v2(r#","containment":"best-effort""#)).cooldown("dsh", None),
        None
    );
    refused_as(
        &v2(r#","containment":"required""#),
        |problem| matches!(problem, Problem::ContainmentRequired),
        "containment required is decision 0080's, not built yet (#472)",
    );
    let Problem::Shape(error) = refused(&v2(r#","containment":"strict""#)) else {
        panic!("an unknown containment value was not read as the contract's shape");
    };
    assert_eq!(
        error.to_string(),
        "unknown variant `strict`, expected `best-effort` or `required` at line 1 column 256"
    );
}

#[test]
fn the_v2_contract_and_the_loader_admit_and_refuse_the_same_shape() {
    let validator = contract_validator("host.v2.schema.json");
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
/// declaration. Every optional site the published contract refuses, and the
/// two v2-only keys a v1 file may not hold, refuses through the loader too;
/// one table drives both, so schema and loader agree by construction.
#[test]
fn chief_null_declarations_are_refused_by_reader_and_schema() {
    let valid = |name: &str, host: &Value| contract_validator(name).is_valid(host);
    let (v1_host, v2_host) = (capacity("forge.host/v1"), capacity("forge.host/v2"));
    let insert = |mut host: Value, key: &str, value: Value| {
        host.as_object_mut().unwrap().insert(key.to_owned(), value);
        host
    };
    let block = insert(
        v2_host.clone(),
        "cooldowns",
        json!({"default_seconds": 10, "max_seconds": 60, "providers": {}}),
    );
    let with_providers = |providers: Value| set(block.clone(), "/cooldowns/providers", providers);
    let v2_case = |name: &'static str, host: Value| (name, "host.v2.schema.json", host);
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
        v2_case(
            "v2 cooldowns:null",
            insert(v2_host.clone(), "cooldowns", Value::Null),
        ),
        v2_case(
            "v2 containment:null",
            insert(v2_host.clone(), "containment", Value::Null),
        ),
        v2_case("v2 cooldowns providers:null", with_providers(Value::Null)),
        v2_case(
            "v2 provider routes:null",
            with_providers(json!({"dsh": {"routes": null}})),
        ),
        v2_case(
            "v2 provider default_seconds:null",
            with_providers(json!({"dsh": {"default_seconds": null}})),
        ),
        v2_case(
            "v2 provider max_seconds:null",
            with_providers(json!({"dsh": {"max_seconds": null}})),
        ),
        v2_case(
            "v2 route default_seconds:null",
            with_providers(json!({"dsh": {"routes": {"r": {"default_seconds": null}}}})),
        ),
        v2_case(
            "v2 route max_seconds:null",
            with_providers(json!({"dsh": {"routes": {"r": {"max_seconds": null}}}})),
        ),
    ];
    let mut admitted = Vec::new();
    for (name, contract_name, host) in cases {
        assert!(
            !valid(contract_name, &host),
            "{name}: the schema admitted a present null"
        );
        match host_read(&host.to_string()).2 {
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
    let (_dir, file, read) = host_read(&text);
    let error = read.unwrap_err();
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

/// F1 (decision 0071 ruling 3): a closed host enum is its word. serde's
/// derive also reads the one-key map form (`{"best-effort": null}`), which
/// both published contracts refuse; the loader must refuse it too, as the
/// same shape problem. Each of the four host enums is reached.
#[test]
fn chief_enum_maps_are_refused_by_reader_and_schema() {
    let cases = [
        (
            "schema",
            "host.v2.schema.json",
            set(
                capacity("forge.host/v2"),
                "/schema",
                json!({"forge.host/v2": null}),
            ),
        ),
        (
            "containment",
            "host.v2.schema.json",
            set(
                capacity("forge.host/v2"),
                "/containment",
                json!({"best-effort": null}),
            ),
        ),
        (
            "route class",
            "host.v1.schema.json",
            set(
                capacity("forge.host/v1"),
                "/providers/dsh/routes/r/class",
                json!({"cloud": null}),
            ),
        ),
        (
            "floor place",
            "host.v1.schema.json",
            set(
                capacity("forge.host/v1"),
                "/floors/0/place",
                json!({"temp": null}),
            ),
        ),
    ];
    for (name, contract, host) in cases {
        assert!(
            !contract_validator(contract).is_valid(&host),
            "{name}: the schema admitted a one-key map"
        );
        let Problem::Shape(error) = refused(&host.to_string()) else {
            panic!("{name}: the loader did not refuse it as a shape");
        };
        assert!(
            error.to_string().contains("expected a string"),
            "{name}: {error}"
        );
    }
}

/// One integral-notation case: the field's name, the pointer, the number as
/// written, the capacity it must read as, and where that capacity lands.
type IntegralCase = (
    &'static str,
    &'static str,
    Value,
    u64,
    fn(&HostConfig) -> u64,
);

/// F2 (decision 0071 ruling 3): JSON Schema's `integer` admits a number
/// with no fractional part, so `10.0` and `6e1` are the same capacity as
/// `10` and `60`. Every capacity number reads that way in both versions,
/// not only the cool-down numbers.
#[test]
fn chief_capacity_integral_notation_reads_the_same_capacity() {
    let cases: [IntegralCase; 4] = [
        (
            "provider ceiling",
            "/providers/dsh/ceiling",
            json!(10.0),
            10,
            |host| u64::from(host.providers["dsh"].ceiling.get()),
        ),
        (
            "route ceiling",
            "/providers/dsh/routes/r/ceiling",
            json!(6e1),
            60,
            |host| u64::from(host.providers["dsh"].routes["r"].ceiling.get()),
        ),
        (
            "boxed-build ceiling",
            "/boxed_builds/ceiling",
            json!(1.0),
            1,
            |host| u64::from(host.boxed_builds.ceiling.get()),
        ),
        (
            "floor bytes",
            "/floors/0/floor_bytes",
            json!(1e0),
            1,
            |host| host.floors.0[0].floor_bytes.get(),
        ),
    ];
    for (schema, contract) in [
        ("forge.host/v1", "host.v1.schema.json"),
        ("forge.host/v2", "host.v2.schema.json"),
    ] {
        let validator = contract_validator(contract);
        for (name, pointer, value, expected, read) in &cases {
            let host = set(capacity(schema), pointer, value.clone());
            assert!(
                validator.is_valid(&host),
                "{schema} {name}: the schema refused integral notation"
            );
            let loaded = loaded(&host.to_string());
            assert_eq!(read(&loaded), *expected, "{schema} {name}");
        }
    }
}

/// F1 (decision 0071 ruling 3; the operator's ruling of 2026-11-04,
/// option A): a floor token is read from its exact digits, never through
/// `f64`. `9007199254740993.0` is one past the last odd integer an `f64`
/// can hold, so a reader that rounds reads `...992`; `u64::MAX.0` is
/// likewise refused by a reader that rounds it to `2^64`.
#[test]
fn chief_exact_floor_tokens_read_without_rounding() {
    for schema in ["forge.host/v1", "forge.host/v2"] {
        assert_eq!(
            loaded(&floor_token(schema, "9007199254740993.0")).floors.0[0]
                .floor_bytes
                .get(),
            9_007_199_254_740_993,
            "{schema}"
        );
        assert_eq!(
            loaded(&floor_token(schema, "18446744073709551615.0"))
                .floors
                .0[0]
                .floor_bytes
                .get(),
            u64::MAX,
            "{schema}"
        );
    }
}

/// The fractional cool-down token the chief's case wrote is above the
/// 7200-second maximum only because its fraction is non-zero, so a reader
/// that rounds admits it as 7200 and weakens the cap. It is not an
/// integer, and it is a shape refusal; an all-zero fraction still reads.
#[test]
fn chief_fractional_cool_down_token_is_a_shape_refusal() {
    let text = cooled(r#"{"default_seconds":10,"max_seconds":7200.0000000000001}"#);
    let Problem::Shape(error) = refused(&text) else {
        panic!("a fractional cool-down was not a shape refusal");
    };
    assert!(
        error
            .to_string()
            .contains("a number with a fractional part"),
        "{error}"
    );
    let exact = loaded(&cooled(r#"{"default_seconds":10,"max_seconds":7200.0}"#));
    assert_eq!(bounds(&exact, "dsh", None), Some((10, 7200)));
}

/// Integral notation both versions' schemas admit reads as its exact
/// value: a zero fraction and an exponent that keeps the number whole.
#[test]
fn integral_token_notations_read_as_their_values() {
    let notations = [
        ("10", 10),
        ("10.0", 10),
        ("1e1", 10),
        ("1.0e1", 10),
        ("100e-1", 10),
        ("6E1", 60),
    ];
    for schema in ["forge.host/v1", "forge.host/v2"] {
        for (token, expected) in notations {
            assert_eq!(
                loaded(&floor_token(schema, token)).floors.0[0]
                    .floor_bytes
                    .get(),
                expected,
                "{schema} {token}"
            );
        }
    }
}

/// F1 from the landing review (2026-11-05; decision 0071 ruling 3, the
/// operator's 2026-11-04 option A): a negative exponent is bounded by the
/// mantissa's trailing zeros, not by the positive exponent's limit. One
/// more zero than the limit still reads exactly `1` in both versions; a
/// shift past the zeros remains a shape refusal, and a huge negative
/// exponent is refused by its own value rather than its size.
#[test]
fn chief_negative_exponent_is_bounded_by_the_mantissa() {
    let shifted = format!("1{}e-41", "0".repeat(41));
    for schema in ["forge.host/v1", "forge.host/v2"] {
        assert_eq!(
            file_loaded(&floor_token(schema, &shifted)).floors.0[0]
                .floor_bytes
                .get(),
            1,
            "{schema} {shifted}"
        );
        for token in ["10e-2", "1e-999999999"] {
            let Problem::Shape(error) = refused(&floor_token(schema, token)) else {
                panic!("{schema} {token} was not a shape refusal");
            };
            assert!(
                error
                    .to_string()
                    .contains("a number with a fractional part"),
                "{schema} {token}: {error}"
            );
        }
    }
}

/// A token that is not exactly an integer in range is a shape refusal: a
/// non-zero fraction, a negative exponent that leaves one, one past each
/// field's maximum, an exponent past the reader's bound and a string.
#[test]
fn non_integral_tokens_are_shape_refusals() {
    for schema in ["forge.host/v1", "forge.host/v2"] {
        for token in [
            "1.5",
            "7200.0000000000001",
            "1e-1",
            "-0.5",
            "18446744073709551616.0",
            "\"one\"",
        ] {
            let Problem::Shape(_) = refused(&floor_token(schema, token)) else {
                panic!("{schema} {token} was not a shape refusal");
            };
        }
        let Problem::Shape(error) = refused(&floor_token(schema, "1e999999999")) else {
            panic!("{schema} 1e999999999 was not a shape refusal");
        };
        assert!(
            error.to_string().contains("exponent too large to read"),
            "{schema}: {error}"
        );
        let Problem::Shape(error) = refused(&ceiling_token(schema, "4294967296.0")) else {
            panic!("{schema} a u32 one past its maximum was not a shape refusal");
        };
        assert!(
            error.to_string().contains("expected a nonzero u32"),
            "{schema}: {error}"
        );
    }
}

/// The token reader on its own, at every place its grammar turns (decision
/// 0071 ruling 9): the JSON number forms no planted file above writes, an
/// explicit `+` exponent, a zero mantissa under a negative exponent, a
/// negative zero and the one magnitude whose negative is `i128::MIN`, read
/// exactly; and the text that is no JSON number, which the reader refuses
/// by its own grammar rather than trusting its caller to never pass it.
#[test]
fn the_token_reader_reads_each_number_form_and_refuses_what_is_not_one() {
    use super::shape::{Integral, Refusal};
    let read = |token: &str| token.parse::<Integral<i128>>().map(|Integral(value)| value);
    assert_eq!(read("1e+1"), Ok(10));
    assert_eq!(read("0e-2"), Ok(0));
    assert_eq!(read("-0"), Ok(0));
    assert_eq!(
        read("-170141183460469231731687303715884105728"),
        Ok(i128::MIN)
    );
    assert_eq!(read(""), Err(Refusal::NotANumber));
    for token in ["-", "1.", "1e", "1x", "1e1x"] {
        assert_eq!(read(token), Err(Refusal::NotAnInteger), "{token}");
    }
}

/// F2 from the landing review (decision 0071 ruling 8): the reader's two
/// diagnostics are pinned by their exact words and parser place, not only
/// by their variant. Each is planted in v1 and v2; changing either phrase
/// must fail this case.
#[test]
fn chief_diagnostic_text_is_pinned_for_non_numbers_and_out_of_range() {
    let cases = [
        (
            "\"one\"",
            "invalid value: a value that is not a number, expected an integer at line 1 column 87",
        ),
        (
            "340282366920938463463374607431768211456.0",
            "invalid value: a number out of range, expected an integer at line 1 column 123",
        ),
    ];
    for schema in ["forge.host/v1", "forge.host/v2"] {
        for (token, said) in cases {
            let Problem::Shape(error) = refused(&floor_token(schema, token)) else {
                panic!("{schema} {token} was not a shape refusal");
            };
            assert_eq!(error.to_string(), said, "{schema} {token}");
        }
    }
}

/// F3 (decision 0071 ruling 3): every closed host object is an object.
/// serde's derived struct reader also reads a sequence positionally, which
/// the contracts refuse; the loader must refuse it too. Every closed object
/// in the file, the root and the nested cool-down records included, is
/// reached.
#[test]
fn chief_struct_arrays_are_refused_by_reader_and_schema() {
    let overrides = |providers: Value| {
        set(
            capacity("forge.host/v2"),
            "/cooldowns",
            json!({"default_seconds": 10, "max_seconds": 60, "providers": providers}),
        )
    };
    let root = json!([
        "forge.host/v2",
        {
            "dsh": {"ceiling": 1, "routes": {"r": {"ceiling": 1, "class": "cloud"}}},
            "codex": {"ceiling": 1}
        },
        [{"place": "temp", "floor_bytes": 1}],
        {"ceiling": 1}
    ]);
    let cases = [
        ("root", "host.v2.schema.json", root),
        (
            "cooldowns",
            "host.v2.schema.json",
            set(capacity("forge.host/v2"), "/cooldowns", json!([10, 60])),
        ),
        (
            "boxed builds",
            "host.v2.schema.json",
            set(capacity("forge.host/v2"), "/boxed_builds", json!([1])),
        ),
        (
            "provider",
            "host.v2.schema.json",
            set(capacity("forge.host/v2"), "/providers/dsh", json!([1])),
        ),
        (
            "route",
            "host.v2.schema.json",
            set(
                capacity("forge.host/v2"),
                "/providers/dsh/routes/r",
                json!([1, "cloud"]),
            ),
        ),
        (
            "floor",
            "host.v2.schema.json",
            set(
                capacity("forge.host/v2"),
                "/floors/0",
                json!([1, "/var/tmp"]),
            ),
        ),
        (
            "provider cool-down overrides",
            "host.v2.schema.json",
            set(
                overrides(json!({"dsh": {"default_seconds": 20, "max_seconds": 30}})),
                "/cooldowns/providers/dsh",
                json!([20, 30]),
            ),
        ),
        (
            "route cool-down overrides",
            "host.v2.schema.json",
            set(
                overrides(json!({"dsh": {"routes": {"r": {"max_seconds": 1}}}})),
                "/cooldowns/providers/dsh/routes/r",
                json!([20, 30]),
            ),
        ),
    ];
    for (name, contract, host) in cases {
        assert!(
            !contract_validator(contract).is_valid(&host),
            "{name}: the schema admitted a positional array"
        );
        let Problem::Shape(error) = refused(&host.to_string()) else {
            panic!("{name}: the loader did not refuse it as a shape");
        };
        assert!(
            error.to_string().contains("expected a JSON object"),
            "{name}: {error}"
        );
    }
}
