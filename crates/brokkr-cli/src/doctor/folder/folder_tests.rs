//! Every form doctor's folder line takes (decision 0083 ruling 5), each
//! exactly, in its status column and ahead of every other line.

use std::path::{Path, PathBuf};

use super::{bundle, report_folder, report_folder_with, report_keys, Inspected, Report, Seats};
use crate::folder::DoctorDefaults;
use crate::recipes::personal::ShelfPath;

/// A repository root of its own, its `.git` planted, and what `plant`
/// puts in it; the tempdir comes back with the canonical root.
fn root_with(plant: fn(&Path)) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    std::fs::create_dir(root.join(".git")).unwrap();
    plant(&root);
    (dir, root)
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A report already holding one line, so the folder line is seen first.
fn report() -> Report {
    Report {
        healthy: true,
        lines: vec!["ok       later: line".to_string()],
    }
}

/// The folder line of a doctor run from `cwd`, `compile` its compile, and
/// whether the report stayed healthy.
fn folder_lines(cwd: &Path, compile: fn(&Path) -> anyhow::Result<()>) -> (Vec<String>, bool) {
    folder_lines_with(cwd, None, compile)
}

/// [`folder_lines`], with the personal shelf `personal` (#697).
fn folder_lines_with(
    cwd: &Path,
    personal: Option<&Path>,
    compile: fn(&Path) -> anyhow::Result<()>,
) -> (Vec<String>, bool) {
    let mut report = report();
    report_folder_with(
        &mut report,
        &DoctorDefaults::of(cwd.into(), None).found,
        personal.map(ShelfPath::new).as_ref(),
        compile,
        shelved,
    );
    assert_eq!(report.lines.last().unwrap(), "ok       later: line");
    report.lines.pop();
    (report.lines, report.healthy)
}

fn compiles(_: &Path) -> anyhow::Result<()> {
    Ok(())
}

fn refuses(dir: &Path) -> anyhow::Result<()> {
    Err(anyhow::anyhow!("one\u{1b}[2J\ntwo").context(format!("bundle: {}", dir.display())))
}

/// A shelf recipe's compile: the digest of one named `good`, and
/// [`refuses`]' refusal for any other.
fn shelved(dir: &Path) -> anyhow::Result<String> {
    match dir.ends_with("good") {
        true => Ok("0123456789ab".into()),
        false => refuses(dir).map(|()| String::new()),
    }
}

/// A half-moved folder's line for the entry `left` at the root.
fn left_at(root: &Path, left: &str) -> String {
    let root = root.display();
    format!(
        "MISSING  folder: {root}/.brokkr (found from {root}), but {root}/{left} still stands at the \
         root (decision 0083); move or remove it"
    )
}

/// One form: what the root holds, the line it earns, and whether the
/// report stays healthy.
type Form = (fn(&Path), fn(&Path) -> String, bool);

#[test]
fn each_form_of_the_folder_line_is_exact_and_first() {
    let rows: [Form; 7] = [
        (
            |root| std::fs::create_dir(root.join(".brokkr")).unwrap(),
            |root| {
                format!(
                    "ok       folder: {}/.brokkr (found from {})",
                    root.display(),
                    root.display()
                )
            },
            true,
        ),
        (
            |_| {},
            |root| {
                let root = root.display();
                format!("warn     folder: none from {root} up to {root} — brokkr init . writes one")
            },
            true,
        ),
        (
            |root| write(&root.join("recipes/x/bundle.json"), "{}"),
            |root| {
                format!(
                    "ok       folder: none: {} is a recipe library, which keeps its own layout \
                     (decision 0083)",
                    root.display()
                )
            },
            true,
        ),
        (
            |root| write(&root.join("bundle.json"), "{}"),
            |_| {
                "MISSING  folder: this repository has Brokkr's old root layout and no .brokkr/ \
                 folder (decision 0083); nothing was opened, and Brokkr moves nothing"
                    .to_string()
            },
            false,
        ),
        (
            |root| write(&root.join(".forge/secrets.env"), ""),
            |_| {
                "MISSING  folder: this repository has Brokkr state in .forge/ and no .brokkr/ \
                 folder (decision 0083); nothing was opened, and Brokkr moves nothing"
                    .to_string()
            },
            false,
        ),
        (
            |root| {
                std::fs::create_dir(root.join(".brokkr")).unwrap();
                write(&root.join("bundle.json"), "{}");
                write(&root.join(".forge/forge.db"), "");
            },
            |root| {
                format!(
                    "{}\n{}",
                    left_at(root, "bundle.json"),
                    left_at(root, ".forge")
                )
            },
            false,
        ),
        (
            |root| write(&root.join(".brokkr"), "a file"),
            |root| {
                format!(
                    "MISSING  folder: {}/.brokkr is not a directory (decision 0083); Brokkr's \
                     folder must be one: move or remove it",
                    root.display()
                )
            },
            false,
        ),
    ];
    for (plant, expected, healthy) in rows {
        let (_dir, root) = root_with(plant);
        let (lines, kept) = folder_lines(&root, compiles);
        assert_eq!((lines.join("\n"), kept), (expected(&root), healthy));
    }
}

/// A folder whose compile refuses: the clause carries the error's whole
/// chain, each line rendered safe, as `brokkr compile` prints it.
#[test]
fn a_folder_whose_compile_refuses_says_so() {
    let (_dir, root) = root_with(|root| std::fs::create_dir(root.join(".brokkr")).unwrap());
    let (lines, healthy) = folder_lines(&root, refuses);
    let folder = root.join(".brokkr");
    let expected = format!(
        "MISSING  folder: {} (found from {}), but its compile refuses (decision 0083): bundle: \
         {}: one[2J\ntwo",
        folder.display(),
        root.display(),
        folder.display()
    );
    assert_eq!((lines.join("\n"), healthy), (expected, false));
}

/// The folder's own compile, as a run compiles it: an empty folder holds
/// no bundle, and the line carries the compile's own refusal; a folder
/// map that will not read is that map's refusal, never a compile under no
/// map.
#[test]
fn the_folder_is_compiled_as_a_run_compiles_it() {
    let no_bundle = |root: &Path| {
        let folder = root.join(".brokkr");
        let refused = brokkr_runtime::launch::compile_for(root, &folder, None, root).unwrap_err();
        format!("{:#}", anyhow::Error::from(refused))
    };
    let no_map = |root: &Path| {
        let map = root.join(".brokkr/realms.json");
        let read = "key must be a string at line 1 column 2";
        format!("{} is not a readable realms map: {read}", map.display())
    };
    let rows: [Form; 2] = [
        (
            |root| std::fs::create_dir(root.join(".brokkr")).unwrap(),
            no_bundle,
            false,
        ),
        (
            |root| write(&root.join(".brokkr/realms.json"), "{invalid json"),
            no_map,
            false,
        ),
    ];
    for (plant, refused, healthy) in rows {
        let (_dir, root) = root_with(plant);
        let defaults = DoctorDefaults::of(root.clone(), None);
        let mut report = report();
        report_folder(&mut report, &defaults, None, None);
        let expected = format!(
            "MISSING  folder: {}/.brokkr (found from {}), but its compile refuses (decision \
             0083): {}",
            root.display(),
            root.display(),
            refused(&root)
        );
        let later = "ok       later: line".to_string();
        assert_eq!(
            (report.lines, report.healthy),
            (vec![expected, later], healthy)
        );
    }
}

/// R5: the folder's one compile. A folder found with no old shape and no
/// `--bundle` is the report's bundle, and an empty folder's refusal is
/// held for the folder line alone; the line reads the outcome it is
/// handed, never compiling the folder a second time. A named bundle, an
/// old shape beside the folder, no folder and a discovery refusal each
/// leave the folder line its own compile.
#[test]
fn the_folder_is_the_reports_bundle_compiled_once() {
    let (_dir, root) = root_with(|root| std::fs::create_dir(root.join(".brokkr")).unwrap());
    let defaults = DoctorDefaults::of(root.clone(), None);
    let folder = root.join(".brokkr");
    let no_bundle = brokkr_runtime::launch::compile_for(&root, &folder, None, &root).unwrap_err();
    let no_bundle = format!("{:#}", anyhow::Error::from(no_bundle));
    let Inspected::Folder {
        dir,
        compiled: Err(held),
        map: Ok(()),
    } = bundle(None, &defaults, None, &Ok(None))
    else {
        panic!("the found folder, its compile refused");
    };
    assert_eq!(
        (dir, format!("{held:#}")),
        (folder.clone(), no_bundle.clone())
    );
    let folder_line = |held: anyhow::Result<()>| {
        let mut report = report();
        report_folder(&mut report, &defaults, None, Some(held));
        report.lines.swap_remove(0)
    };
    let found = format!("{}/.brokkr (found from {})", root.display(), root.display());
    assert_eq!(folder_line(Ok(())), format!("ok       folder: {found}"));
    assert_eq!(
        folder_line(Err(anyhow::anyhow!("held"))),
        format!("MISSING  folder: {found}, but its compile refuses (decision 0083): held")
    );

    let Inspected::Given(dir, Err(compiled)) = bundle(Some(&folder), &defaults, None, &Ok(None))
    else {
        panic!("the bundle given, its compile refused");
    };
    assert_eq!((dir, format!("{compiled:#}")), (folder, no_bundle));
    let plants: [fn(&Path); 3] = [
        |root| {
            std::fs::create_dir(root.join(".brokkr")).unwrap();
            write(&root.join("bundle.json"), "{}");
        },
        |_| {},
        |root| std::fs::create_dir_all(root.join("src/.brokkr")).unwrap(),
    ];
    for plant in plants {
        let (_dir, root) = root_with(plant);
        std::fs::create_dir_all(root.join("src")).unwrap();
        let defaults = DoctorDefaults::of(root.join("src"), None);
        let inspected = bundle(None, &defaults, None, &Ok(None));
        assert!(matches!(inspected, Inspected::Neither));
    }
}

/// A folder whose compile refuses is, to the seats judgement, a bundle
/// that did not compile, never no bundle given (decision 0040 ruling 4),
/// and no bundle line: its refusal is the folder line's alone. A bundle
/// given that refuses has its bundle line; with neither, no bundle was
/// given.
#[test]
fn a_folder_whose_compile_refuses_is_a_bundle_that_did_not_compile() {
    let (_dir, root) = root_with(|root| std::fs::create_dir(root.join(".brokkr")).unwrap());
    let defaults = DoctorDefaults::of(root.clone(), None);
    let found = bundle(None, &defaults, None, &Ok(None));
    assert!(matches!(found.seats(None), Seats::BundleDidNotCompile));
    assert_eq!(found.line().map(|(dir, _)| dir.to_path_buf()), None);
    assert!(matches!(found.folder(), Some(Err(_))));

    let folder = root.join(".brokkr");
    let given = bundle(Some(&folder), &defaults, None, &Ok(None));
    assert!(matches!(given.seats(None), Seats::BundleDidNotCompile));
    assert_eq!(
        given.line().map(|(dir, _)| dir.to_path_buf()),
        Some(folder.clone())
    );
    assert!(given.folder().is_none());
    assert!(matches!(
        Inspected::Neither.seats(None),
        Seats::NoBundleGiven
    ));

    // A map a run will not load is the folder line's refusal, so the
    // folder's compile refusing too is named on its bundle line.
    write(&folder.join("realms.json"), "{}");
    let map = brokkr_runtime::realms::World::discover(&root, None).unwrap_err();
    let unmapped = bundle(None, &defaults, None, &Ok(None));
    assert!(matches!(unmapped.seats(None), Seats::BundleDidNotCompile));
    let line = unmapped
        .line()
        .map(|(dir, compiled)| (dir.to_path_buf(), compiled.is_err()));
    assert_eq!(line, Some((folder, true)));
    let held = unmapped
        .folder()
        .map(|held| format!("{:#}", held.unwrap_err()));
    assert_eq!(held, Some(map.to_string()));
}

/// An old shape that cannot be read is its own line, never taken as no
/// shape. A process that reads a mode-000 directory, as root does, skips.
#[test]
fn an_old_shape_that_cannot_be_read_is_missing() {
    let (_dir, root) = root_with(|_| {});
    let defaults = DoctorDefaults::of(root.clone(), None);
    let mode = |bits| std::os::unix::fs::PermissionsExt::from_mode(bits);
    std::fs::set_permissions(&root, mode(0o000)).unwrap();
    let readable = std::fs::read_dir(&root).is_ok();
    let mut report = report();
    report_folder_with(&mut report, &defaults.found, None, compiles, shelved);
    std::fs::set_permissions(&root, mode(0o755)).unwrap();
    if readable {
        eprintln!("skipped: this process reads a mode-000 directory, as root does");
        return;
    }
    let denied = format!(
        "MISSING  folder: reading {}: Permission denied (os error 13)",
        root.join("bundle.json").display()
    );
    assert_eq!((&report.lines[0], report.healthy), (&denied, false));
}

/// The folder's shelf (#696): after the folder line, one line per recipe
/// by name, its digest or its compile's refusal as a warning, which
/// leaves the report healthy; a file on the shelf is no recipe, and a
/// shelf that cannot be listed, a link to nothing included, warns.
#[test]
fn each_shelf_recipe_is_a_line_that_never_fails_the_report() {
    let (_dir, root) = root_with(|root| {
        std::fs::create_dir_all(root.join(".brokkr/recipes/good")).unwrap();
        std::fs::create_dir_all(root.join(".brokkr/recipes/broken")).unwrap();
        write(&root.join(".brokkr/recipes/notes.md"), "not a recipe\n");
    });
    let (folder, shelf) = (root.join(".brokkr"), root.join(".brokkr/recipes"));
    let (shown, at) = (root.display(), shelf.display());
    let expected = [
        format!("ok       folder: {} (found from {shown})", folder.display()),
        format!(
            "warn     recipe broken: {at}/broken does not compile (#696): bundle: {at}/broken: \
             one[2J\ntwo"
        ),
        format!("ok       recipe good: 0123456789ab at {at}/good"),
    ];
    assert_eq!(folder_lines(&root, compiles), (expected.to_vec(), true));
    std::fs::remove_dir_all(&shelf).unwrap();
    write(&shelf, "a file\n");
    let unlisted = format!("warn     shelf: {at}: Not a directory (os error 20)");
    let (lines, healthy) = folder_lines(&root, compiles);
    assert_eq!((&lines[1..], healthy), (&[unlisted][..], true));
    std::fs::remove_file(&shelf).unwrap();
    std::os::unix::fs::symlink("removed-shelf", &shelf).unwrap();
    let dangling = format!("warn     shelf: {at}: No such file or directory (os error 2)");
    let (lines, healthy) = folder_lines(&root, compiles);
    assert_eq!((&lines[1..], healthy), (&[dangling][..], true));
}

/// The personal shelf (#697): after the folder found and its shelf, and
/// after a library's line, one line per personal recipe, its digest or
/// its compile's refusal as a warning, which leaves the report healthy; a
/// personal shelf that cannot be listed warns, an absent one is no line
/// but one below a link to nothing warns, as `--recipe` refuses it, and a
/// bare repository, where no run reads a recipe, lists none.
#[test]
fn each_personal_recipe_is_a_line_that_never_fails_the_report() {
    let config = tempfile::tempdir().unwrap();
    let personal = config.path().canonicalize().unwrap().join("recipes");
    for name in ["good", "broken"] {
        std::fs::create_dir_all(personal.join(name)).unwrap();
    }
    let at = personal.display();
    let lines = [
        format!(
            "warn     personal recipe broken: {at}/broken does not compile (#697): bundle: \
             {at}/broken: one[2J\ntwo"
        ),
        format!("ok       personal recipe good: 0123456789ab at {at}/good"),
    ];
    let planted: [fn(&Path); 2] = [
        |root| std::fs::create_dir_all(root.join(".brokkr/recipes/good")).unwrap(),
        |root| write(&root.join("recipes/x/bundle.json"), "{}"),
    ];
    for (plant, shelf_lines) in planted.into_iter().zip([1, 0]) {
        let (_dir, root) = root_with(plant);
        let (found, healthy) = folder_lines_with(&root, Some(&personal), compiles);
        assert_eq!((&found[1 + shelf_lines..], healthy), (&lines[..], true));
    }
    let (_dir, bare) = root_with(|_| {});
    assert_eq!(
        folder_lines_with(&bare, Some(&personal), compiles).0.len(),
        1
    );
    let (_dir, root) = root_with(|root| std::fs::create_dir(root.join(".brokkr")).unwrap());
    let absent = config.path().join("absent");
    assert_eq!(folder_lines_with(&root, Some(&absent), compiles).0.len(), 1);
    let dangling = config.path().join("dangling");
    std::os::unix::fs::symlink("gone", &dangling).unwrap();
    let below = dangling.join("recipes");
    let unread = format!(
        "warn     personal shelf: {}: No such file or directory (os error 2)",
        below.display()
    );
    let (found, healthy) = folder_lines_with(&root, Some(&below), compiles);
    assert_eq!((&found[1..], healthy), (&[unread][..], true));
    std::fs::remove_dir_all(&personal).unwrap();
    write(&personal, "a file\n");
    let unlisted = format!("warn     personal shelf: {at}: Not a directory (os error 20)");
    let (found, healthy) = folder_lines_with(&root, Some(&personal), compiles);
    assert_eq!((&found[1..], healthy), (&[unlisted][..], true));
}

/// Each dsh key answer (#717) in its status column: ok, a failure of the
/// report, or a warning, after what the report already held.
#[test]
fn each_dsh_key_answer_takes_its_status_column() {
    use crate::dsh_key::{Reach, Source};
    let (route, variable) = (String::from("r"), String::from("V"));
    let answers = [
        Reach::Reached {
            route: route.clone(),
            variable: variable.clone(),
            source: Source::Environment,
        },
        Reach::Undeclared {
            route: route.clone(),
        },
        Reach::Unreached {
            route,
            variable,
            files: vec![PathBuf::from("/repo/.env")],
            patch: Err("none".into()),
        },
    ];
    let mut report = report();
    report_keys(&mut report, &answers[..2]);
    assert!(report.healthy);
    report_keys(&mut report, &answers[2..]);
    let status: Vec<&str> = report.lines.iter().map(|line| &line[..20]).collect();
    assert_eq!(
        (status, report.healthy),
        (
            vec![
                "ok       later: line",
                "ok       dsh key r: ",
                "warn     dsh key r: ",
                "MISSING  dsh key r: "
            ],
            false
        )
    );
}
