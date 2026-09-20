//! `brokkr doctor` selects DSH the way a spawning child would on Windows,
//! or refuses before it probes anything — proved against the BUILT binary
//! with real images, real children and a working directory of the test's
//! own (review 2026-09-20, R8: the Unix suite is no Windows evidence).
//!
//! Windows' rule, as `std::process::Command` applies it: a spelling with
//! a separator is a path; a file name is searched on the child's `PATH`
//! when the child's environment changed, then the application directory,
//! the system and Windows directories and the parent's `PATH`, with
//! `.exe` appended when the name has no extension; the working directory
//! is never searched; the first entry that EXISTS is the selection, and
//! `CreateProcessW` then runs it or fails without trying a later entry.
//! Every test here runs the built doctor from a temporary directory
//! holding copies of the shipped `adapters/` and `agents/`, with the
//! environment changed only on the child, and compares doctor's line with
//! a native `Command::new(name)` child under the same cwd and `PATH`.
#![cfg(windows)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// A working directory carrying the SHIPPED adapters and agents, so the
/// doctor under test reads the shipped DSH declaration — unmeasured, with
/// no `wrapper_digest` — exactly as an operator's would.
fn shipped_workspace() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    copy_tree(&repository().join("adapters"), &dir.path().join("adapters"));
    copy_tree(&repository().join("agents"), &dir.path().join("agents"));
    let shipped = std::fs::read_to_string(dir.path().join("adapters/dsh.json")).unwrap();
    assert!(
        !shipped.contains("wrapper_digest"),
        "the shipped declaration is unmeasured and declares no digest"
    );
    dir
}

/// The built `brokkr.exe`, placed under `name` in `dir`: a working
/// native image whose `--version` line identifies it. Hard-linked where
/// the volume allows, copied otherwise.
fn image(dir: &Path, name: &str) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let at = dir.join(name);
    let built = Path::new(env!("CARGO_BIN_EXE_brokkr"));
    if std::fs::hard_link(built, &at).is_err() {
        std::fs::copy(built, &at).unwrap();
    }
    at
}

/// The built doctor from `cwd`, with both DSH overrides removed. `PATH`
/// is left to each test: removed, or set to exactly what it names.
fn doctor(cwd: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_brokkr"));
    command
        .arg("doctor")
        .current_dir(cwd)
        .env_remove("BROKKR_DSH_BIN")
        .env_remove("FORGE_DSH_BIN");
    command
}

fn run(command: &mut Command) -> Output {
    command.output().expect("the built doctor runs")
}

fn stdout_of(command: &mut Command) -> String {
    String::from_utf8_lossy(&run(command).stdout).into_owned()
}

/// The report's `dsh` line, whichever status it carries.
fn dsh_line(stdout: &str) -> String {
    stdout
        .lines()
        .find(|line| line.starts_with("ok       dsh:") || line.starts_with("warn     dsh:"))
        .unwrap_or_else(|| panic!("a dsh line in the report:\n{stdout}"))
        .to_string()
}

/// A native child's own lookup of `name` under the same cwd and `PATH`
/// doctor was given: the comparison every claim here is made against.
fn native_lookup(cwd: &Path, name: &str, path: Option<&Path>) -> std::io::Result<Output> {
    let mut command = Command::new(name);
    command.arg("--version").current_dir(cwd);
    match path {
        Some(path) => command.env("PATH", path),
        None => command.env_remove("PATH"),
    };
    command.output()
}

/// The first line the built image answers `--version` with.
fn banner_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// A `dsh.exe` in the working directory is never selected: Windows
/// lookup does not search the working directory, with `PATH` absent, with
/// `PATH` naming another directory and with a present, empty `PATH`
/// alike, and doctor refuses where the native child finds nothing.
#[test]
fn a_cwd_image_is_never_selected_by_a_search_that_does_not_name_it() {
    let workspace = shipped_workspace();
    let cwd = workspace.path();
    image(cwd, "dsh.exe");
    let elsewhere = cwd.join("elsewhere");
    std::fs::create_dir_all(&elsewhere).unwrap();
    for path in [None, Some(elsewhere.as_path()), Some(Path::new(""))] {
        let native = native_lookup(cwd, "dsh", path);
        let mut command = doctor(cwd);
        match path {
            Some(path) => command.env("PATH", path),
            None => command.env_remove("PATH"),
        };
        let line = dsh_line(&stdout_of(&mut command));
        match native {
            Err(error) => {
                assert_eq!(error.kind(), std::io::ErrorKind::NotFound, "{error}");
                assert!(
                    line.starts_with(
                        "warn     dsh: binary 'dsh' not found: the DSH layout is unreadable: \
                         'dsh' is not on the Windows search path"
                    ),
                    "PATH {path:?}: doctor refuses where the child found nothing: {line}"
                );
            }
            // A runner whose own `PATH` or application directory holds a
            // `dsh.exe`: the native child ran it, and so must doctor's
            // selection — never the cwd image.
            Ok(output) => {
                let banner = banner_of(&output);
                assert!(
                    line.starts_with(&format!("ok       dsh: {banner} · serves")),
                    "PATH {path:?}: doctor selected the file the child ran: {line}"
                );
            }
        }
    }
}

/// A native image on `PATH` is selected as the child selects it, and
/// doctor's line carries that child's own first line. A bare `dsh` finds
/// `dsh.exe`; a spelled `dsh.exe` finds it too; a name with a space finds
/// `my dsh.exe`.
#[test]
fn a_native_image_on_path_is_selected_as_the_child_selects_it() {
    let workspace = shipped_workspace();
    let cwd = workspace.path();
    let on_path = cwd.join("on-path");
    image(&on_path, "dsh.exe");
    image(&on_path, "my dsh.exe");
    for name in ["dsh", "dsh.exe", "my dsh"] {
        let child = native_lookup(cwd, name, Some(&on_path)).unwrap();
        assert!(
            child.status.success(),
            "{name}: the native child ran the image"
        );
        let banner = banner_of(&child);
        assert!(!banner.is_empty(), "the image answers --version");
        let line = dsh_line(&stdout_of(
            doctor(cwd)
                .env("PATH", &on_path)
                .env("BROKKR_DSH_BIN", name),
        ));
        assert!(
            line.starts_with(&format!("ok       dsh: {banner} · serves")),
            "{name}: doctor's version is the native child's own first line: {line}"
        );
    }
    // An explicit path selects the file it spells, with and without the
    // `.exe` suffix, under no `PATH` at all.
    for spelled in [on_path.join("dsh.exe"), on_path.join("dsh")] {
        let line = dsh_line(&stdout_of(
            doctor(cwd)
                .env_remove("PATH")
                .env("BROKKR_DSH_BIN", &spelled),
        ));
        assert!(line.starts_with("ok       dsh: "), "{spelled:?}: {line}");
    }
}

/// An entry that EXISTS and cannot run is the child's selection and its
/// failure — Windows lookup never walks past it to B — and doctor refuses
/// it by cause, exits as a report does, and probes nothing: a text file
/// under the candidate's name, a directory under it, the 40-byte Mach-O
/// whose truncated dynamic-linker command once made the reader panic
/// (review 2026-09-20, R4), and a batch script whose `cmd.exe` dispatch
/// this resolver does not establish.
#[test]
fn an_existing_entry_that_cannot_run_is_refused_by_cause_without_a_panic() {
    let workspace = shipped_workspace();
    let cwd = workspace.path();
    let a = cwd.join("a");
    let b = cwd.join("b");
    image(&b, "dsh.exe");
    let b_banner = banner_of(&native_lookup(cwd, "dsh", Some(&b)).unwrap());
    let path = std::env::join_paths([&a, &b]).unwrap();
    std::fs::create_dir_all(&a).unwrap();

    // This target's Mach-O CPU type, so the reader reaches the command
    // rather than refusing the CPU type first.
    let cputype: u32 = if cfg!(target_arch = "aarch64") {
        0x0100_000c
    } else {
        0x0100_0007
    };
    let mut macho = vec![0xcf, 0xfa, 0xed, 0xfe];
    macho.extend_from_slice(&cputype.to_le_bytes());
    macho.extend_from_slice(&[0; 4]);
    macho.extend_from_slice(&2u32.to_le_bytes());
    macho.extend_from_slice(&1u32.to_le_bytes());
    macho.extend_from_slice(&8u32.to_le_bytes());
    macho.extend_from_slice(&[0; 8]);
    macho.extend_from_slice(&0xeu32.to_le_bytes());
    macho.extend_from_slice(&8u32.to_le_bytes());
    assert_eq!(macho.len(), 40);

    for (what, body, cause) in [
        (
            "a text file",
            Some(b"not an image\r\n".to_vec()),
            "is not a loadable native image: neither a #! script nor a native image",
        ),
        (
            "a truncated Mach-O",
            Some(macho),
            "is not a loadable native image: a malformed Mach-O dynamic linker command",
        ),
        ("a directory", None, "is not a regular file"),
    ] {
        let at = a.join("dsh.exe");
        let _ = std::fs::remove_file(&at);
        let _ = std::fs::remove_dir_all(&at);
        match &body {
            Some(bytes) => std::fs::write(&at, bytes).unwrap(),
            None => std::fs::create_dir_all(&at).unwrap(),
        }
        // The native child selects A's entry and fails there: it never
        // runs B.
        let native = native_lookup(cwd, "dsh", Some(Path::new(&path)));
        match native {
            Err(error) => assert_ne!(
                error.kind(),
                std::io::ErrorKind::NotFound,
                "{what}: the child selected A's entry and failed on it: {error}"
            ),
            Ok(output) => panic!(
                "{what}: the native child ran something: {}",
                String::from_utf8_lossy(&output.stdout)
            ),
        }
        let output = run(doctor(cwd).env("PATH", &path));
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        assert_ne!(
            output.status.code(),
            Some(101),
            "{what}: doctor panicked:\n{stderr}"
        );
        assert!(!stderr.contains("panicked"), "{what}: {stderr}");
        assert!(
            !stdout.contains(&b_banner),
            "{what}: doctor did not probe B in place of A's entry:\n{stdout}"
        );
        let line = dsh_line(&stdout);
        assert!(
            line.starts_with("warn     dsh: binary 'dsh' not found: "),
            "{what}: {line}"
        );
        assert!(
            line.contains(&format!("{}: {cause}", at.display())),
            "{what}: {line}"
        );
    }
    let _ = std::fs::remove_dir_all(a.join("dsh.exe"));

    // A batch script spelled by name: the child dispatches it through
    // `cmd.exe`; doctor refuses the dispatch it does not establish, and
    // never runs it.
    std::fs::write(
        a.join("dsh.bat"),
        "@echo off\r\necho DSH_BATCH_SENTINEL\r\n",
    )
    .unwrap();
    let line = dsh_line(&stdout_of(
        doctor(cwd)
            .env("PATH", &path)
            .env("BROKKR_DSH_BIN", "dsh.bat"),
    ));
    assert!(!line.contains("DSH_BATCH_SENTINEL"), "{line}");
    assert!(
        line.contains(&format!(
            "{}: is a batch script, whose cmd.exe dispatch this resolver does not establish",
            a.join("dsh.bat").display()
        )),
        "{line}"
    );
}

/// The overrides keep their precedence and their no-fallback meaning on
/// Windows too: primary `BROKKR_DSH_BIN`, then legacy `FORGE_DSH_BIN`,
/// then `PATH`; a failed override never falls back to a PATH decoy.
#[test]
fn explicit_overrides_keep_their_precedence_and_never_fall_back() {
    let workspace = shipped_workspace();
    let cwd = workspace.path();
    let primary = image(&cwd.join("primary"), "dsh.exe");
    let legacy = image(&cwd.join("legacy"), "dsh.exe");
    let on_path = cwd.join("on-path");
    image(&on_path, "dsh.exe");
    let banner = banner_of(&native_lookup(cwd, "dsh", Some(&on_path)).unwrap());
    let line = |command: &mut Command| dsh_line(&stdout_of(command));

    // Every image is the same built binary, so the selection is asserted
    // through the version probe's success and the failed override's
    // refusal rather than through distinguishable banners.
    for (primary, legacy) in [
        (Some(&primary), Some(&legacy)),
        (None, Some(&legacy)),
        (None, None),
    ] {
        let mut command = doctor(cwd);
        command.env("PATH", &on_path);
        if let Some(primary) = primary {
            command.env("BROKKR_DSH_BIN", primary);
        }
        if let Some(legacy) = legacy {
            command.env("FORGE_DSH_BIN", legacy);
        }
        let observed = line(&mut command);
        assert!(
            observed.starts_with(&format!("ok       dsh: {banner} · serves")),
            "{observed}"
        );
    }
    let missing = cwd.join("missing").join("dsh.exe");
    let failed = line(
        doctor(cwd)
            .env("PATH", &on_path)
            .env("BROKKR_DSH_BIN", &missing),
    );
    assert!(
        failed.starts_with(&format!(
            "warn     dsh: binary '{}' not found: the DSH layout is unreadable: {}: cannot be \
             inspected: ",
            missing.display(),
            missing.display()
        )),
        "a failed override never falls back to the PATH decoy: {failed}"
    );
}
