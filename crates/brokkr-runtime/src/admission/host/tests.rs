//! Where the host configuration is found, what the free-space probe
//! measures, and that the published `forge.host/v1` contract and the
//! loader refuse and admit the same files.

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
                         "scratch": {"floor_bytes": 1}, "boxed_builds": {"ceiling": 1}});
    let with = |pointer: &str, value: Value| {
        let mut host = minimal.clone();
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        host.pointer_mut(parent).unwrap()[key] = value;
        host
    };
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
        with("/scratch/path", json!("/var/tmp")),
        with("/scratch/floor_bytes", json!(u64::MAX)),
    ];
    let refused = [
        with("/cooldowns", json!({})),
        with("/schema", json!("forge.host/v2")),
        with("/providers/claude", json!({"ceiling": 0})),
        with("/providers/claude", json!({"ceiling": -1})),
        with("/providers/claude", json!({"ceiling": 4_294_967_296_u64})),
        with("/providers/claude", json!({"ceiling": 1.5})),
        with("/providers/claude", json!({"ceiling": 1, "burst": 2})),
        with("/providers/claude", json!({})),
        route(json!({"ceiling": 1, "class": "local"})),
        route(json!({"ceiling": 1})),
        route(json!({"ceiling": 1, "class": "cloud", "timeout": 60})),
        with("/scratch/floor_bytes", json!(0)),
        with("/scratch/path", json!("var/tmp")),
        with("/scratch/path", Value::Null),
        with("/boxed_builds/ceiling", json!(0)),
        with("/boxed_builds/cap", json!(1)),
        json!({"schema": "forge.host/v1", "providers": {}, "boxed_builds": {"ceiling": 1}}),
    ];
    let judged = |host: &Value| {
        let loaded = serde_json::from_value::<HostConfig>(host.clone()).is_ok();
        (validator.is_valid(host), loaded)
    };
    assert_eq!(admitted.each_ref().map(judged), [(true, true); 5]);
    assert_eq!(refused.each_ref().map(judged), [(false, false); 17]);

    // JSON Schema cannot say a key is written once, and its validator reads
    // the last copy: there the loader is stricter than the contract can be.
    let twice = written(r#""dsh": {"ceiling": 1}, "dsh": {"ceiling": 2}"#);
    let last: Value = serde_json::from_str(&twice).unwrap();
    let loaded = config(twice.as_bytes()).is_ok();
    assert_eq!((validator.is_valid(&last), loaded), (true, false));
}

/// A minimal host configuration's text with `providers` written as given.
fn written(providers: &str) -> String {
    format!(
        r#"{{"schema": "forge.host/v1", "providers": {{{providers}}}, "scratch": {{"floor_bytes": 1}}, "boxed_builds": {{"ceiling": 1}}}}"#
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
