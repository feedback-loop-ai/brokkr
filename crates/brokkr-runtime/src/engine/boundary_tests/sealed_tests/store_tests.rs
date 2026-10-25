//! The secrets store every workspace box hides (decision 0065 slice two,
//! U8h; SI1, MB4), as the engine hands it over: the engine resolves the
//! store's place once for the run and keeps it, admission seals that place
//! in each boxed writer's reach, the seat's input names it, the engine makes
//! a missing store empty and owner-only in its folder before a boxed launch,
//! as it makes the artifact root, and refuses the spawn where it cannot or
//! where the configured spelling no longer leads there; the box itself
//! never makes one.

use std::io::ErrorKind::{NotADirectory, NotFound, PermissionDenied};
use std::os::unix::fs::PermissionsExt;

use brokkr_protocol::hands::sealed_store;

use super::*;
use crate::engine::boxed::{make_store, Unprepared, Unstored};

/// The permission bits at `path`, where it stands.
fn mode(path: &Path) -> Option<u32> {
    let found = std::fs::metadata(path).ok();
    found.map(|found| found.permissions().mode() & 0o777)
}

/// Assert the engine made the store at `store`: one empty file, its mode
/// owner-only.
fn made_empty(store: &Path) {
    let made = (std::fs::read(store).ok(), mode(store));
    let empty = (Some(Vec::new()), Some(0o600));
    assert_eq!(made, empty, "the engine made the store");
}

/// Assert nothing stands at `path`.
fn unmade(path: &Path) {
    let found = std::fs::symlink_metadata(path).map_err(|error| error.kind());
    assert_eq!(found.err(), Some(NotFound), "nothing was made at {path:?}");
}

/// An exec seat running `script`, boxed whole under `namespace`.
fn exec_seat(script: &str) -> Vec<String> {
    let seat = [
        "/usr/local/bin/brokkr",
        "driver",
        "exec",
        "--",
        "/bin/sh",
        "-c",
    ];
    let mut seat: Vec<String> = seat.map(String::from).to_vec();
    seat.push(script.to_string());
    seat
}

/// An engine over a single exec seat running `true`, boxed under
/// `namespace`, and that seat's command.
fn boxed_engine() -> (tempfile::TempDir, Engine, Vec<String>) {
    let command = exec_seat("true");
    let (dir, mut engine) = crate::engine::tests::engine(single_body(command.clone()));
    engine.boundary = Boundary::Namespace;
    (dir, engine, command)
}

/// Under `base`, the folders `real` and `moved`, and the link `route` to
/// `real`, which `engine`'s store is configured through: their paths.
fn routed(engine: &mut Engine, base: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let (real, moved, route) = (base.join("real"), base.join("moved"), base.join("route"));
    std::fs::create_dir(&real).unwrap();
    std::fs::create_dir(&moved).unwrap();
    std::os::unix::fs::symlink(&real, &route).unwrap();
    engine.secrets_file = Some(route.join("secrets.env"));
    (real, moved, route)
}

/// Point the link `route` at `to`.
fn relinked(route: &Path, to: &Path) {
    std::fs::remove_file(route).unwrap();
    std::os::unix::fs::symlink(to, route).unwrap();
}

/// The refusal of `engine`'s next spawn of `command`, boxed with default
/// hands, or unboxed where `boxed` is false.
fn refusal(engine: &mut Engine, command: &[String], boxed: bool) -> Option<String> {
    let spec = HandsSpec::default();
    let hands = boxed.then_some(&spec);
    let spawn = engine.compose("attempt", false, command.to_vec(), hands, None, "r.json");
    spawn.refusal
}

/// The engine makes a missing store one empty file only its owner may
/// read or write, in the folder that holds it, keeps an existing one as it
/// is, and refuses with the store's cause where it cannot make one: a
/// missing folder is never made (operator ruling 18 of 2026-10-25).
#[test]
fn the_engine_makes_a_missing_store_owner_only_or_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().canonicalize().unwrap();
    std::fs::create_dir(base.join("adopter")).unwrap();
    let store = base.join("adopter/secrets.env");
    assert_eq!(make_store(&store), Ok(()));
    made_empty(&store);
    std::fs::write(&store, "DOCS_TOKEN=fake\n").unwrap();
    std::fs::set_permissions(&store, std::fs::Permissions::from_mode(0o640)).unwrap();
    assert_eq!(make_store(&store), Ok(()));
    let kept = (std::fs::read_to_string(&store).unwrap(), mode(&store));
    assert_eq!(kept, ("DOCS_TOKEN=fake\n".to_string(), Some(0o640)));
    std::fs::write(base.join("file"), "").unwrap();
    let missing = base.join("missing/secrets.env");
    let (blocked, long) = (base.join("file/secrets.env"), base.join("n".repeat(300)));
    for (store, kind) in [
        (&missing, NotFound),
        (&blocked, NotADirectory),
        (&long, std::io::ErrorKind::InvalidFilename),
    ] {
        assert_eq!(make_store(store), Err(Unstored(store.clone(), kind)));
    }
    unmade(&base.join("missing"));
    let text = format!(
        "dispatch refused: secrets store cannot be hidden from workspace hands: the store '{}' \
         cannot be made: entity not found",
        missing.display()
    );
    assert_eq!(Unstored(missing, NotFound).to_string(), text);
}

/// Under `namespace` a seat with hands is composed only once the engine
/// made its store, empty and owner-only, where admission sealed it, in a
/// folder that stands; a store its folder will not take refuses the spawn
/// with the store's cause, and nothing is made.
#[test]
fn a_boxed_spawn_makes_its_store_before_launch_or_refuses() {
    let (_dir, mut engine, command) = boxed_engine();
    let store = engine.secrets_store_path();
    assert!(!store.exists());
    assert_eq!(refusal(&mut engine, &command, true), None);
    made_empty(&store);
    let (dir, mut blocked, command) = boxed_engine();
    let folder = dir.path().canonicalize().unwrap().join("closed");
    std::fs::create_dir(&folder).unwrap();
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o500)).unwrap();
    blocked.secrets_file = Some(folder.join("secrets.env"));
    let place = blocked.secrets_store_path();
    let unstored = Unstored(place.clone(), PermissionDenied);
    assert_eq!(
        refusal(&mut blocked, &command, true),
        Some(unstored.to_string())
    );
    unmade(&place);
}

/// Operator ruling 18 of 2026-10-25 and run 6's L1: a store whose folder
/// cannot be resolved, missing, a file or a folder its user may not search,
/// refuses every spawn, boxed or not, with its own cause, and nothing
/// stands in for its place: the reach seals no place of its own, the box is
/// never built and nothing is made.
#[test]
fn a_store_whose_folder_cannot_be_resolved_refuses_every_spawn() {
    let cases = [
        ("missing", NotFound),
        ("file", NotADirectory),
        ("shut", PermissionDenied),
    ];
    for (folder, kind) in cases {
        let (dir, mut engine, command) = boxed_engine();
        let base = dir.path().canonicalize().unwrap();
        let spelling = base.join(folder).join("secrets.env");
        if folder == "file" {
            std::fs::write(base.join(folder), "").unwrap();
        } else if folder == "shut" {
            std::fs::create_dir(base.join(folder)).unwrap();
            let shut = std::fs::Permissions::from_mode(0o000);
            std::fs::set_permissions(base.join(folder), shut).unwrap();
        }
        engine.secrets_file = Some(spelling.clone());
        assert_eq!(engine.kept_store().place, Err(kind));
        let unresolved = Unprepared::Unresolved(spelling.clone(), kind).to_string();
        assert_eq!(
            refusal(&mut engine, &command, true),
            Some(unresolved.clone())
        );
        assert_eq!(refusal(&mut engine, &command, false), Some(unresolved));
        let sealed = engine
            .reach
            .sealed(&HandsSpec::default())
            .and_then(sealed_store);
        assert_eq!(
            sealed,
            Some(spelling.as_path()),
            "sealed as spelled, never built"
        );
        assert_eq!(engine.secrets_store_path(), spelling);
        if folder == "shut" {
            let open = std::fs::Permissions::from_mode(0o700);
            std::fs::set_permissions(base.join(folder), open).unwrap();
        }
        let held = std::fs::read_dir(base.join(folder)).map(Iterator::count);
        assert_eq!(
            held.ok(),
            (folder == "shut").then_some(0),
            "nothing was made"
        );
    }
    let text = format!(
        "dispatch refused: secrets store cannot be hidden from workspace hands: the configured \
         store '{}' cannot be resolved to its place: not a directory",
        Path::new("/f/secrets.env").display()
    );
    let unresolved = Unprepared::Unresolved(PathBuf::from("/f/secrets.env"), NotADirectory);
    assert_eq!(unresolved.to_string(), text);
}

/// Operator ruling 18 of 2026-10-25: the engine resolves the store's place
/// once for the run, a link on the way resolved, and keeps it. Once the
/// route moves, the seat's input, the path threaded to drivers and the
/// reach admission seals still name the kept place, never where the
/// spelling leads now, and every spawn that would use it, boxed or not,
/// refuses before anything is made at either place; a spelling that no
/// longer resolves refuses by its error. Led back to the kept place, the
/// next box is launched and the store made there.
#[test]
fn the_run_keeps_one_store_place_and_refuses_a_spelling_that_left_it() {
    let (dir, mut engine, command) = boxed_engine();
    engine.bundle.seats.get_mut("work").unwrap().secrets = vec!["TOKEN".into()];
    let base = dir.path().canonicalize().unwrap();
    let (real, moved, route) = routed(&mut engine, &base);
    let relink = |to: &Path| relinked(&route, to);
    let place = real.join("secrets.env");
    let named = |engine: &Engine| seat_input_of(engine, "work")["secrets_file"].clone();
    assert_eq!(named(&engine), json!(place));
    relink(&moved);
    assert_eq!(
        (named(&engine), engine.secrets_store_path()),
        (json!(place), place.clone())
    );
    let strayed = Unprepared::Strayed(route.join("secrets.env"), place.clone()).to_string();
    assert_eq!(refusal(&mut engine, &command, true), Some(strayed.clone()));
    let sealed = engine
        .reach
        .sealed(&HandsSpec::default())
        .and_then(sealed_store);
    assert_eq!(
        sealed,
        Some(place.as_path()),
        "admission sealed the kept place"
    );
    assert_eq!(refusal(&mut engine, &command, false), Some(strayed));
    unmade(&place);
    unmade(&moved.join("secrets.env"));
    std::fs::write(base.join("file"), "").unwrap();
    relink(&base.join("file"));
    let unresolved = Unprepared::Unresolved(route.join("secrets.env"), NotADirectory);
    assert_eq!(
        refusal(&mut engine, &command, true),
        Some(unresolved.to_string())
    );
    relink(&real);
    assert_eq!(refusal(&mut engine, &command, true), None);
    made_empty(&place);
    let text = format!(
        "dispatch refused: secrets store cannot be hidden from workspace hands: the configured \
         store '{}' no longer leads to '{}', where the box's reach sealed it",
        route.join("secrets.env").display(),
        place.display()
    );
    let strayed = Unprepared::Strayed(route.join("secrets.env"), place);
    assert_eq!(strayed.to_string(), text);
}

/// Run 6's L2 (ruling 17): a reach retained from another resolution of the
/// store, as an earlier engine process sealed one before the store's route
/// moved, names another place than the one this run keeps, though the
/// configured spelling leads to the kept place now. The boxed spawn built
/// from that reach refuses before anything is made at either place.
#[test]
fn a_reach_sealed_at_another_place_than_the_kept_one_refuses() {
    let (dir, mut engine, command) = boxed_engine();
    let base = dir.path().canonicalize().unwrap();
    let (real, moved, route) = routed(&mut engine, &base);
    let spec = HandsSpec::default();
    engine.sealed_for(&base.join("home"), &spec);
    relinked(&route, &moved);
    engine.store_place = std::sync::OnceLock::new();
    let (sealed, kept) = (real.join("secrets.env"), moved.join("secrets.env"));
    assert_eq!(engine.secrets_store_path(), kept);
    let spawn = engine.compose_at(None, "attempt", false, command, Some(&spec), None, "r.json");
    let strayed = Unprepared::Strayed(route.join("secrets.env"), sealed.clone());
    assert_eq!(spawn.refusal, Some(strayed.to_string()));
    unmade(&sealed);
    unmade(&kept);
}

/// In a real namespace: an exec seat whose store lies in its worktree, and
/// is missing at launch, is composed with the reach admission sealed for
/// it, which names the store; the engine made the store empty and
/// owner-only, and the box `hands exec` builds from that reach denies
/// reading its path, as `cat` reports an EACCES (operator ruling 10 of
/// 2026-10-24), the host's store unchanged.
#[cfg(target_os = "linux")]
#[test]
fn a_store_the_engine_made_is_hidden_in_the_box_it_composes() {
    let command = exec_seat("exec 2>seen; exec cat .forge/secrets.env");
    let (dir, mut engine) = crate::engine::tests::engine(single_body(command.clone()));
    engine.boundary = Boundary::Namespace;
    let (spec, store) = (HandsSpec::default(), engine.secrets_store_path());
    engine.sealed_for(&dir.path().join("home"), &spec);
    let spawn = engine.compose("attempt", false, command, Some(&spec), None, "r.json");
    assert_eq!(spawn.refusal, None);
    made_empty(&store);
    let argv = spawn.argv;
    let sealed: brokkr_protocol::hands::Sealed = serde_json::from_str(&argv[8]).unwrap();
    let carried = serde_json::to_value(&sealed).unwrap();
    let spelling = engine.secrets_store_spelling();
    assert_eq!(
        carried["store"],
        json!({"place": store, "spelling": spelling})
    );
    std::fs::write(&store, "DOCS_TOKEN=fake-u8h-value\n").unwrap();
    let Some(bwrap) = super::namespace_bwrap() else {
        return;
    };
    // The driver runs what follows its own `--` inside the box.
    let ran = argv.iter().rposition(|part| part == "--").unwrap() + 1;
    let spec = HandsSpec::parse(&serde_json::from_str(&argv[10]).unwrap()).unwrap();
    let (work, artifacts) = (Path::new(&argv[4]), Path::new(&argv[6]));
    let session = dir.path().join("session");
    let boxed = brokkr_protocol::hands::run_boxed_in(
        &bwrap,
        &spec,
        work,
        artifacts,
        &sealed,
        &session,
        &[],
        None,
        &argv[ran..],
    );
    assert_eq!(boxed, Ok(1));
    let seen = std::fs::read_to_string(work.join("seen")).unwrap();
    // GNU coreutils' words, or uutils', which name the error number.
    let words = "cat: .forge/secrets.env: Permission denied";
    let either = [format!("{words}\n"), format!("{words} (os error 13)\n")];
    assert!(either.contains(&seen), "{seen:?}");
    let host = std::fs::read_to_string(&store).unwrap();
    assert_eq!(host, "DOCS_TOKEN=fake-u8h-value\n");
}
