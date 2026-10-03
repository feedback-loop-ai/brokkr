//! A queued launch holds every fact its request is rebuilt from, in one
//! versioned encoding, rebuilds the world it was queued with rather than
//! the one its file holds later, and refuses a payload it cannot read as
//! that.

use std::error::Error as _;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::*;
use crate::launch::LaunchRequest;
use crate::realms::World;

/// A one-realm map at `dir/map.json`, its realm named `realm`, read as a
/// world.
fn world_named(dir: &Path, realm: &str) -> World {
    let map = json!({"schema": "forge.realms/v4", "journal": "forge.db",
        "realms": [{"name": realm, "path": ".", "default_branch": "main"}]});
    std::fs::write(dir.join("map.json"), map.to_string()).unwrap();
    World::load(&dir.join("map.json")).unwrap()
}

fn world(dir: &Path) -> World {
    world_named(dir, "here")
}

fn request(bundle: BundleSource) -> LaunchRequest {
    LaunchRequest {
        workspace: PathBuf::from("/work"),
        bundle,
        journal: PathBuf::from("/work/forge.db"),
        repo: Some(PathBuf::from("/repo")),
        secrets: Some(PathBuf::from("secrets.env")),
        host_path: "/usr/bin".into(),
    }
}

fn recipe() -> BundleSource {
    BundleSource::Recipe {
        name: "story".into(),
        recipes_dir: PathBuf::from("recipes"),
    }
}

/// A recipe's launch under the map at `dir/map.json`, named: the world
/// it was queued with, and the payload.
fn queued_named(dir: &Path) -> (World, String) {
    let run = NewRun {
        feature: "queue it".into(),
        map: RunMap::Named(world(dir)),
        dispatch: None,
    };
    let text = QueuedLaunch::of(&request(recipe()), &run)
        .unwrap()
        .encode()
        .unwrap();
    (world(dir), text)
}

/// A world by what identifies it: its file, digest and content.
type Identity = (PathBuf, String, Value);

fn identity(world: &World) -> Identity {
    (
        world.source.clone(),
        world.sha256.clone(),
        world.content.clone(),
    )
}

/// Everything a launch is asked with, comparable.
type Facts = (
    (
        PathBuf,
        BundleSource,
        PathBuf,
        Option<PathBuf>,
        Option<PathBuf>,
    ),
    (
        OsString,
        String,
        &'static str,
        Option<Identity>,
        Option<PathBuf>,
    ),
);

fn facts(request: &LaunchRequest, run: &NewRun) -> Facts {
    let (kind, world) = match &run.map {
        RunMap::Unmapped => ("unmapped", None),
        RunMap::Ambient(world) => ("ambient", Some(identity(world))),
        RunMap::Named(world) => ("named", Some(identity(world))),
    };
    (
        (
            request.workspace.clone(),
            request.bundle.clone(),
            request.journal.clone(),
            request.repo.clone(),
            request.secrets.clone(),
        ),
        (
            request.host_path.clone(),
            run.feature.clone(),
            kind,
            world,
            run.dispatch.clone(),
        ),
    )
}

#[test]
fn a_queued_launch_keeps_every_fact_its_request_is_rebuilt_from() {
    let dir = tempfile::tempdir().unwrap();
    let dispatch = || Some(PathBuf::from("d.json"));
    let maps = [
        (RunMap::Unmapped, dispatch()),
        (RunMap::Ambient(world(dir.path())), dispatch()),
        (RunMap::Named(world(dir.path())), None),
    ];
    // Each relative path comes back anchored to the workspace, `/work`.
    let bundles = [
        (
            BundleSource::Dir("bundles/self".into()),
            BundleSource::Dir("/work/bundles/self".into()),
        ),
        (
            recipe(),
            BundleSource::Recipe {
                name: "story".into(),
                recipes_dir: PathBuf::from("/work/recipes"),
            },
        ),
    ];
    for ((bundle, anchored), (map, dispatch)) in bundles.into_iter().cycle().zip(maps) {
        let asked = request(bundle);
        let run = NewRun {
            feature: "queue it".into(),
            map,
            dispatch,
        };
        let text = QueuedLaunch::of(&asked, &run).unwrap().encode().unwrap();
        let rebuilt = QueuedLaunch::decode(&text).unwrap();
        let (request, new) = rebuilt
            .rebuild(asked.journal.clone(), asked.host_path.clone())
            .unwrap();
        let mut expected = facts(&asked, &run);
        expected.0 .1 = anchored;
        expected.0 .4 = Some(PathBuf::from("/work/secrets.env"));
        expected.1 .4 = run.dispatch.as_ref().map(|_| PathBuf::from("/work/d.json"));
        assert_eq!(facts(&request, &new), expected);
    }
    // The named map, and the encoding's bytes, pinned.
    let (named, text) = queued_named(dir.path());
    let expected = format!(
        "{{\"encoding\":\"queued-launch/v1\",\"workspace\":\"/work\",\
         \"bundle\":{{\"recipe\":{{\"name\":\"story\",\"recipes_dir\":\"/work/recipes\"}}}},\
         \"repo\":\"/repo\",\"secrets\":\"/work/secrets.env\",\"feature\":\"queue it\",\
         \"map\":{{\"named\":{{\"map\":{{\"journal\":\"forge.db\",\"realms\":[{{\
         \"default_branch\":\"main\",\"name\":\"here\",\"path\":\".\"}}],\
         \"schema\":\"forge.realms/v4\"}},\"sha256\":\"{}\",\"source\":{}}}}},\
         \"dispatch\":null}}",
        named.sha256,
        json!(named.source)
    );
    assert_eq!(text, expected);
}

#[test]
fn a_queued_world_is_the_one_it_was_queued_with_whatever_its_file_holds_later() {
    let dir = tempfile::tempdir().unwrap();
    let queued_with = world(dir.path());
    // Queued from the map's own directory, so the pin names its realm.
    let asked = LaunchRequest {
        workspace: dir.path().to_path_buf(),
        repo: None,
        ..request(recipe())
    };
    for map in [
        RunMap::Ambient(world(dir.path())),
        RunMap::Named(world(dir.path())),
    ] {
        let run = NewRun {
            feature: "queue it".into(),
            map,
            dispatch: None,
        };
        let text = QueuedLaunch::of(&asked, &run).unwrap().encode().unwrap();
        // What the entry holds is the run manifest's pin, realm and all.
        let queued = QueuedLaunch::decode(&text).unwrap();
        let (MapSource::Ambient(held) | MapSource::Named(held)) = &queued.map else {
            panic!("no world held: {:?}", queued.map);
        };
        assert_eq!(held.0, queued_with.pin(Some(dir.path())).unwrap());
        assert_eq!(held.0["realm"], "here");
        let rewritten = world_named(dir.path(), "there");
        assert_ne!(rewritten.sha256, queued_with.sha256);
        let (_, new) = QueuedLaunch::decode(&text)
            .unwrap()
            .rebuild(asked.journal.clone(), asked.host_path.clone())
            .unwrap();
        let rebuilt = new.map.world().unwrap();
        assert_eq!(identity(rebuilt), identity(&queued_with));
        assert_eq!(rebuilt.pin(Some(dir.path())).unwrap(), held.0);
        world(dir.path());
    }
}

#[test]
fn a_held_world_that_does_not_hash_to_its_digest_is_refused_at_rebuild() {
    let dir = tempfile::tempdir().unwrap();
    let (held, text) = queued_named(dir.path());
    let forged = "0".repeat(64);
    let tampered = QueuedLaunch::decode(&text.replace(&held.sha256, &forged)).unwrap();
    let error = tampered
        .rebuild(PathBuf::from("/work/forge.db"), "/usr/bin".into())
        .unwrap_err();
    assert!(matches!(error, LaunchError::World(WorldError::Unpinned(_))));
    assert_eq!(
        error.to_string(),
        format!(
            "this run's pinned realms map is unreadable: the embedded map hashes \
             to {}, not the pinned {forged}",
            held.sha256
        )
    );
}

/// An entry is admitted from wherever the dispatcher stands: here, this
/// test's own directory, never the workspace it was queued in. Every path
/// it named relatively, and the map file its world was read from, still
/// resolve in that workspace, so it is admitted in the realm and with the
/// bundle it was queued for, and the capability grants it is authorised
/// against (decision 0065) are that realm's.
#[test]
fn a_queued_launch_is_admitted_in_its_own_workspace_wherever_it_is_rebuilt() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path().canonicalize().unwrap();
    assert_ne!(std::env::current_dir().unwrap(), workspace);
    let map = json!({"schema": "forge.realms/v4", "journal": "forge.db",
        "realms": [{"name": "here", "path": ".", "default_branch": "main",
                    "boundary": "open"}]});
    std::fs::write(workspace.join("map.json"), map.to_string()).unwrap();
    crate::launch::tests::bundle_at(&workspace, false);
    let asked = LaunchRequest {
        workspace: workspace.clone(),
        bundle: BundleSource::Dir("bundle".into()),
        repo: Some(".".into()),
        ..request(recipe())
    };
    let run = NewRun {
        feature: "queue it".into(),
        map: RunMap::Named(World::load(&workspace.join("map.json")).unwrap()),
        dispatch: None,
    };
    let text = QueuedLaunch::of(&asked, &run).unwrap().encode().unwrap();
    let mut queued = QueuedLaunch::decode(&text).unwrap();
    let MapSource::Named(held) = &mut queued.map else {
        panic!("no named world held: {:?}", queued.map);
    };
    assert_eq!(held.0["realm"], "here");
    // The map as `--realms map.json` names it, in the workspace: the pin
    // holds the file as it was named.
    held.0["source"] = json!("map.json");
    let (request, new) = queued
        .rebuild(asked.journal.clone(), asked.host_path.clone())
        .unwrap();
    assert_eq!(request.repo, Some(workspace.join(".")));
    assert_eq!(request.secrets, Some(workspace.join("secrets.env")));
    let world = new.map.world().unwrap();
    assert_eq!(world.source, workspace.join("map.json"));
    let bundle = crate::launch::admit_new(&request, Some(world)).unwrap();
    assert_eq!(bundle.dir, workspace.join("bundle"));
    assert_eq!(bundle.boundary, brokkr_core::realms::Boundary::Open);
    assert_eq!(bundle.manifest["capabilities"]["realm"], "here");
}

#[test]
fn a_workspace_an_entry_cannot_be_anchored_to_is_refused_queued_and_rebuilt() {
    let said = "a queued launch names its workspace absolutely, and work is relative";
    let relative = LaunchRequest {
        workspace: PathBuf::from("work"),
        ..request(recipe())
    };
    let mut queued = QueuedLaunch::of(&request(recipe()), &NewRun::default()).unwrap();
    queued.workspace = PathBuf::from("work");
    for error in [
        QueuedLaunch::of(&relative, &NewRun::default()).unwrap_err(),
        queued
            .rebuild(PathBuf::from("/work/forge.db"), "/usr/bin".into())
            .unwrap_err(),
    ] {
        assert!(
            matches!(error, LaunchError::QueuedWorkspaceRelative(ref at) if at == Path::new("work"))
        );
        assert_eq!(error.to_string(), said);
    }
}

#[test]
fn a_launch_start_would_refuse_unread_is_not_queued() {
    let dir = tempfile::tempdir().unwrap();
    let beside_a_dispatch = NewRun {
        feature: "queue it".into(),
        map: RunMap::Named(world(dir.path())),
        dispatch: Some(PathBuf::from("d.json")),
    };
    let error = QueuedLaunch::of(&request(recipe()), &beside_a_dispatch).unwrap_err();
    assert!(matches!(error, LaunchError::RealmsWithDispatch));
    assert_eq!(
        error.to_string(),
        "a run with --dispatch cannot pin the map named by --realms: the \
         Looper-bound run-manifest/v2 lineage carries no world, and dropping \
         the map silently would leave the run unable to say which one it \
         believed in. Run without --dispatch, or without --realms, until a \
         jointly agreed v2-lineage manifest version exists"
    );
    // A house the operated realm names, and that is not there to hold.
    let map = json!({"schema": "forge.realms/v4", "journal": "forge.db",
        "realms": [{"name": "here", "path": ".", "default_branch": "main",
                    "house": "HOUSE.md"}]});
    std::fs::write(dir.path().join("map.json"), map.to_string()).unwrap();
    let houseless = NewRun {
        feature: "queue it".into(),
        map: RunMap::Ambient(World::load(&dir.path().join("map.json")).unwrap()),
        dispatch: None,
    };
    let asked = LaunchRequest {
        workspace: dir.path().to_path_buf(),
        repo: None,
        ..request(recipe())
    };
    let error = QueuedLaunch::of(&asked, &houseless).unwrap_err();
    assert!(matches!(
        error,
        LaunchError::World(WorldError::RealmText { .. })
    ));
    assert_eq!(
        error.to_string(),
        format!(
            "realm 'here' names house at {}, but it is not a readable file: \
             No such file or directory (os error 2)",
            dir.path().join("./HOUSE.md").display()
        )
    );
}

#[test]
fn a_payload_this_encoding_cannot_read_is_refused() {
    let v1 = QueuedLaunch::of(&request(recipe()), &NewRun::default()).unwrap();
    let text = v1.encode().unwrap();
    let cases = [
        (
            text.replace("queued-launch/v1", "queued-launch/v2"),
            "unknown variant `queued-launch/v2`, expected `queued-launch/v1` at line 1 column 30",
        ),
        (
            text.replacen('{', "{\"host_path\":\"/usr/bin\",", 1),
            "unknown field `host_path`, expected one of `encoding`, `workspace`, `bundle`, \
             `repo`, `secrets`, `feature`, `map`, `dispatch` at line 1 column 12",
        ),
        (
            text.replace(
                "\"unmapped\"",
                "{\"named\":\"n.json\",\"ambient\":\"m.json\"}",
            ),
            "expected value at line 1 column 199",
        ),
    ];
    for (payload, why) in cases {
        let error = QueuedLaunch::decode(&payload).unwrap_err();
        assert!(matches!(error, LaunchError::DecodeQueued(_)));
        let said = (error.to_string(), error.source().unwrap().to_string());
        assert_eq!(said, ("reading a queued launch".into(), why.into()));
    }
}

#[test]
fn a_path_that_is_not_utf8_cannot_be_queued() {
    use std::os::unix::ffi::OsStrExt;
    let mut queued = QueuedLaunch::of(&request(recipe()), &NewRun::default()).unwrap();
    queued.workspace = PathBuf::from(std::ffi::OsStr::from_bytes(b"/w\xff"));
    let error = queued.encode().unwrap_err();
    assert!(matches!(error, LaunchError::EncodeQueued(_)));
    assert_eq!(
        (error.to_string(), error.source().unwrap().to_string()),
        (
            "writing a queued launch".to_string(),
            "path contains invalid UTF-8 characters".to_string()
        )
    );
}
