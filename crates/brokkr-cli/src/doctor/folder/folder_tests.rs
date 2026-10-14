//! Every form doctor's folder line takes (decision 0083 ruling 5), each
//! exactly, in its status column and ahead of every other line.

use std::path::{Path, PathBuf};

use super::{report_folder, report_folder_with, Report};
use crate::folder::DoctorDefaults;

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
    let mut report = report();
    report_folder_with(
        &mut report,
        &DoctorDefaults::of(cwd.into(), None).found,
        compile,
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
        report_folder(&mut report, &defaults, None);
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
    report_folder_with(&mut report, &defaults.found, compiles);
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
