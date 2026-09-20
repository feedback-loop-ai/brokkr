//! `brokkr doctor` selects DSH the way a spawning child would, or refuses
//! before it probes anything — proved against the BUILT binary with real
//! executables, real children and a working directory of the test's own.
//!
//! The security hold of 2026-09-20 (run
//! `dsh-composite-identity-issue-226-26def5a5`, review visit 3) found that
//! doctor, run with no `PATH` beside an executable `dsh` in its working
//! directory, executed that file: an absent `PATH` had become one empty
//! search entry, and an empty entry is cwd. The controller reproduced it
//! with a script printing `SECURITY_CWD_SENTINEL_9f3`. Every test here
//! runs the built doctor from a temporary directory holding copies of the
//! shipped `adapters/` and `agents/` — the unmeasured DSH declaration as
//! shipped — with the environment changed only on the child.
//!
//! The second hold (run `dsh-composite-identity-issue-226-069caa79`)
//! found the rule behind the instance: a cwd file literally named
//! `C:\Tools\dsh.exe` executed through doctor, with `PATH` absent and
//! with `PATH` set, because the resolver read a backslash as a path
//! separator where the platform reads an ordinary filename byte. The
//! tests here hold the resolver to the platform's rule against a native
//! `Command::new(name)` child in the same cwd and environment, and to
//! D10's one named exception: a candidate whose interpreter or dynamic
//! loader is missing is refused before any probe, where the native child
//! walks past it to the next entry.
//!
//! Unix only: the fixtures are shell scripts, symlinks, mode bits and
//! `:`-separated `PATH`s. The missing-loader fixtures patch an ELF and
//! run on Linux; elsewhere they are recorded as pending.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// What the cwd decoy prints, and must never reach doctor's output.
const SENTINEL: &str = "SECURITY_CWD_SENTINEL_9f3";

/// What the cwd file literally named `C:\Tools\dsh.exe` prints.
const BACKSLASH_SENTINEL: &str = "SECURITY_BACKSLASH_CWD_SENTINEL_9f3";

/// The literal Unix filename of the second reproduction.
const BACKSLASH_NAME: &str = "C:\\Tools\\dsh.exe";

/// The loader path the patched ELF names, which must not exist.
const MISSING_LOADER: &str = "/no-such-ld-9f3";

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

/// Write an executable STAGED beside its destination and renamed in, so
/// the destination never carries a write descriptor a forked child could
/// inherit (#255).
fn stage_executable(dir: &Path, name: &str, body: &str) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join(name);
    let staging = dir.join(format!(".{name}.staging"));
    std::fs::write(&staging, body).unwrap();
    std::fs::set_permissions(&staging, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::rename(&staging, &path).unwrap();
    path
}

/// A script that answers `--version` with a distinguishable line, and
/// records that it RAN in the marker directory `BROKKR_MARKS` names in
/// its environment — a destination the native oracle and doctor keep
/// separate, so an oracle's execution cannot pollute doctor's no-probe
/// assertion and doctor's output alone is not the only witness.
fn version_script(dir: &Path, name: &str, version: &str) -> PathBuf {
    stage_executable(
        dir,
        name,
        &format!(
            "#!/bin/sh\nif [ -n \"$BROKKR_MARKS\" ]; then : > \"$BROKKR_MARKS/{version}\"; fi\n\
             echo {version}\n"
        ),
    )
}

/// Doctor's own marker directory, created empty; a refusal cell asserts
/// it stays empty.
fn marks(cwd: &Path, who: &str) -> PathBuf {
    let dir = cwd.join(format!("marks-{who}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn executed(marks: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(marks)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// Copy the ELF64 image at `working` to `dest` with only the bytes of
/// its `PT_INTERP` path rewritten to `loader`, NUL-padded to the
/// segment's size: a well-formed image whose dynamic loader does not
/// exist, which the kernel refuses with ENOENT at `execve`.
fn patch_elf_interpreter(working: &Path, dest: &Path, loader: &str) -> PathBuf {
    let mut bytes = std::fs::read(working).unwrap();
    assert_eq!(&bytes[..5], b"\x7fELF\x02", "a 64-bit ELF image");
    let u16_at = |bytes: &[u8], at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]);
    let u32_at = |bytes: &[u8], at: usize| {
        u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
    };
    let u64_at =
        |bytes: &[u8], at: usize| u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap());
    let phoff = u64_at(&bytes, 32) as usize;
    let phentsize = usize::from(u16_at(&bytes, 54));
    let phnum = usize::from(u16_at(&bytes, 56));
    let mut patched = false;
    for index in 0..phnum {
        let at = phoff + index * phentsize;
        if u32_at(&bytes, at) != 3 {
            continue;
        }
        let offset = u64_at(&bytes, at + 8) as usize;
        let filesz = u64_at(&bytes, at + 32) as usize;
        assert!(loader.len() < filesz);
        bytes[offset..offset + filesz].fill(0);
        bytes[offset..offset + loader.len()].copy_from_slice(loader.as_bytes());
        patched = true;
    }
    assert!(patched, "the image names a dynamic loader");
    std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
    let staging = dest.with_file_name(".broken.staging");
    std::fs::write(&staging, &bytes).unwrap();
    std::fs::set_permissions(&staging, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::rename(&staging, dest).unwrap();
    dest.to_path_buf()
}

/// Run a prepared command, retrying only the ETXTBSY a freshly staged
/// executable can answer with (#255).
fn spawn(command: &mut Command) -> std::io::Result<Output> {
    for _ in 0..50 {
        match command.output() {
            Err(error) if error.raw_os_error() == Some(26) => {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            result => return result,
        }
    }
    panic!("the executable stayed busy");
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

fn stdout_of(command: &mut Command) -> String {
    let output = spawn(command).expect("the built doctor runs");
    String::from_utf8_lossy(&output.stdout).into_owned()
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
/// Its markers go to the oracle's own directory.
fn native_lookup(cwd: &Path, name: &str, path: Option<&str>) -> std::io::Result<Output> {
    let mut command = Command::new(name);
    command
        .arg("--version")
        .current_dir(cwd)
        .env("BROKKR_MARKS", marks(cwd, "oracle"));
    match path {
        Some(path) => command.env("PATH", path),
        None => command.env_remove("PATH"),
    };
    spawn(&mut command)
}

fn native_dsh(cwd: &Path, path: Option<&str>) -> std::io::Result<Output> {
    native_lookup(cwd, "dsh", path)
}

/// S1. With no `PATH` and an executable `dsh` in cwd, doctor REFUSES —
/// the sentinel is not executed, the refusal names the unsuccessful
/// native default search with the absent `PATH` as its context — and a
/// native child in the same state finds nothing.
///
/// The order of the assertions is the order of the claims: absence of
/// the sentinel first, because that is the defect; the named reason
/// second, because a refusal asserted as "some error" proves nothing.
/// With absent-PATH-as-empty-entry restored this test fails at the
/// no-sentinel assertion (recorded in the delivery account).
#[test]
fn absent_path_refuses_before_doctor_can_execute_a_cwd_sentinel() {
    let workspace = shipped_workspace();
    let cwd = workspace.path();
    version_script(cwd, "dsh", SENTINEL);
    let doctor_marks = marks(cwd, "doctor");

    // The native control: `Command::new("dsh")` under the same cwd with
    // no `PATH` returns NotFound. What doctor must not do is find more
    // than the child would.
    let native = native_dsh(cwd, None);
    match native {
        Err(error) => assert_eq!(error.kind(), std::io::ErrorKind::NotFound, "{error}"),
        Ok(output) => panic!(
            "a native child found a dsh with no PATH: {}",
            String::from_utf8_lossy(&output.stdout)
        ),
    }

    let stdout = stdout_of(
        doctor(cwd)
            .env_remove("PATH")
            .env("BROKKR_MARKS", &doctor_marks),
    );
    assert!(
        !stdout.contains(SENTINEL),
        "doctor executed the cwd dsh under an absent PATH:\n{stdout}"
    );
    assert_eq!(
        executed(&doctor_marks),
        Vec::<String>::new(),
        "nothing doctor ran left a marker"
    );
    let line = dsh_line(&stdout);
    assert!(
        line.contains("'dsh' is not on the default search path")
            && line.contains("(PATH is absent)"),
        "the refusal names the unsuccessful native default search, PATH absence as context: {line}"
    );
    assert!(
        line.starts_with("warn     dsh: binary 'dsh' not found: "),
        "the spelling that was looked for, and no selected file: {line}"
    );

    // Control: an explicit `/usr/bin:/bin` holding no dsh. The sentinel
    // stays unexecuted and the lookup says what it searched.
    for candidate in ["/usr/bin/dsh", "/bin/dsh"] {
        assert!(
            !Path::new(candidate).exists(),
            "this control needs no dsh at {candidate}"
        );
    }
    let stdout = stdout_of(doctor(cwd).env("PATH", "/usr/bin:/bin"));
    assert!(!stdout.contains(SENTINEL), "{stdout}");
    assert!(
        dsh_line(&stdout).contains("'dsh' is not on PATH"),
        "{stdout}"
    );
    let native = native_dsh(cwd, Some("/usr/bin:/bin")).unwrap_err();
    assert_eq!(native.kind(), std::io::ErrorKind::NotFound);

    // Control: a present PATH naming one empty directory, which is
    // host-independent.
    let empty = tempfile::tempdir().unwrap();
    let stdout = stdout_of(doctor(cwd).env("PATH", empty.path()));
    assert!(!stdout.contains(SENTINEL), "{stdout}");
    assert!(
        dsh_line(&stdout).contains("'dsh' is not on PATH"),
        "{stdout}"
    );

    // Control: the sentinel removed and PATH still absent. The refusal
    // is the default search's, not the missing file's.
    std::fs::remove_file(cwd.join("dsh")).unwrap();
    let stdout = stdout_of(doctor(cwd).env_remove("PATH"));
    let line = dsh_line(&stdout);
    assert!(
        line.contains("'dsh' is not on the default search path")
            && line.contains("(PATH is absent)"),
        "{stdout}"
    );
}

/// S1b. A cwd file literally named `C:\Tools\dsh.exe`, with the primary
/// override spelling exactly that name: native lookup returns NotFound
/// with `PATH` absent and with `PATH` set, and so does doctor, without
/// executing the file — a backslash is an ordinary filename byte on
/// Unix, and only `/` makes a path. A file of that literal name in a
/// PATH directory is what native lookup runs, and doctor selects it; a
/// `/`-containing spelling of the cwd file is the direct-path control.
/// With backslash-as-separator restored this test fails at the
/// no-execution assertion (recorded in the delivery account).
#[test]
fn unix_backslash_names_follow_native_lookup_before_doctor_probe() {
    let workspace = shipped_workspace();
    let cwd = workspace.path();
    version_script(cwd, BACKSLASH_NAME, BACKSLASH_SENTINEL);
    let doctor_marks = marks(cwd, "doctor");
    let elsewhere = tempfile::tempdir().unwrap();
    let elsewhere = elsewhere.path().to_str().unwrap().to_string();

    for path in [None, Some(elsewhere.as_str())] {
        let native = native_lookup(cwd, BACKSLASH_NAME, path);
        match native {
            Err(error) => assert_eq!(error.kind(), std::io::ErrorKind::NotFound, "{error}"),
            Ok(output) => panic!(
                "a native child ran the backslash-named cwd file with PATH {path:?}: {}",
                String::from_utf8_lossy(&output.stdout)
            ),
        }
        let mut command = doctor(cwd);
        command
            .env("BROKKR_DSH_BIN", BACKSLASH_NAME)
            .env("BROKKR_MARKS", &doctor_marks);
        match path {
            Some(path) => command.env("PATH", path),
            None => command.env_remove("PATH"),
        };
        let stdout = stdout_of(&mut command);
        assert!(
            !stdout.contains(BACKSLASH_SENTINEL),
            "doctor executed the cwd C:\\Tools\\dsh.exe with PATH {path:?}:\n{stdout}"
        );
        assert_eq!(executed(&doctor_marks), Vec::<String>::new());
        let line = dsh_line(&stdout);
        assert!(
            line.starts_with("warn     dsh: binary 'C:\\Tools\\dsh.exe' not found: "),
            "{line}"
        );
        match path {
            None => assert!(
                line.contains("'C:\\Tools\\dsh.exe' is not on the default search path")
                    && line.contains("(PATH is absent)"),
                "{line}"
            ),
            Some(_) => assert!(
                line.contains("'C:\\Tools\\dsh.exe' is not on PATH"),
                "the lookup failed without converting the backslashes to separators: {line}"
            ),
        }
    }

    // A distinct file of that literal name in a PATH directory is what
    // the native child runs, and the installation doctor describes.
    let on_path = cwd.join("on-path");
    version_script(&on_path, BACKSLASH_NAME, "DSH_BACKSLASH_ON_PATH_0.0.4");
    let native = native_lookup(cwd, BACKSLASH_NAME, on_path.to_str()).unwrap();
    assert!(String::from_utf8_lossy(&native.stdout).contains("DSH_BACKSLASH_ON_PATH_0.0.4"));
    let stdout = stdout_of(
        doctor(cwd)
            .env("PATH", &on_path)
            .env("BROKKR_DSH_BIN", BACKSLASH_NAME)
            .env("BROKKR_MARKS", &doctor_marks),
    );
    assert!(!stdout.contains(BACKSLASH_SENTINEL), "{stdout}");
    let line = dsh_line(&stdout);
    assert!(
        line.starts_with("ok       dsh: DSH_BACKSLASH_ON_PATH_0.0.4 · serves"),
        "{line}"
    );
    assert_eq!(
        executed(&doctor_marks),
        vec!["DSH_BACKSLASH_ON_PATH_0.0.4".to_string()],
        "doctor probed the PATH file once and the cwd file never"
    );

    // The direct-path control: `./C:\Tools\dsh.exe` contains `/`, so
    // it IS the cwd file, to the native child and to doctor alike.
    let direct = format!("./{BACKSLASH_NAME}");
    let native = native_lookup(cwd, &direct, None).unwrap();
    assert!(String::from_utf8_lossy(&native.stdout).contains(BACKSLASH_SENTINEL));
    let stdout = stdout_of(
        doctor(cwd)
            .env_remove("PATH")
            .env("BROKKR_DSH_BIN", &direct),
    );
    assert!(
        dsh_line(&stdout).starts_with(&format!("ok       dsh: {BACKSLASH_SENTINEL} · serves")),
        "{stdout}"
    );

    // Removing the fixture is a separate negative control: the same
    // refusal, from a lookup that never had the file to find.
    std::fs::remove_file(cwd.join(BACKSLASH_NAME)).unwrap();
    let stdout = stdout_of(
        doctor(cwd)
            .env_remove("PATH")
            .env("BROKKR_DSH_BIN", BACKSLASH_NAME),
    );
    assert!(
        dsh_line(&stdout).contains("'C:\\Tools\\dsh.exe' is not on the default search path"),
        "{stdout}"
    );
}

/// AO. With `PATH` absent, native lookup searches the C library's
/// default path — `sh` is the observed positive — and doctor selects the
/// very file that child ran, through the primary override and then the
/// legacy one, never a same-named cwd decoy. With `PATH` present but
/// empty the decoy is exactly what both select, and without the decoy
/// both find nothing. An explicit `PATH` is the independent positive.
/// Node is asked of the host the same way: a native positive on the
/// default search is compared by identity; a native miss is asserted as
/// the resolver's named refusal, and the positive is recorded as pending
/// on that host. With unconditional absent-PATH refusal restored this
/// test fails at the selected-identity assertion while the native
/// positive still executes (recorded in the delivery account).
#[test]
fn absent_path_default_search_matches_native_dsh_and_node() {
    let workspace = shipped_workspace();
    let cwd = workspace.path();
    version_script(cwd, "sh", "SECURITY_CWD_DECOY_SH_9f3");
    version_script(cwd, "node", "SECURITY_CWD_DECOY_NODE_9f3");
    let doctor_marks = marks(cwd, "doctor");

    // The native positive: `sh` runs with PATH removed, from the default
    // search, and says which file it is (through /proc on Linux).
    let native = spawn(
        Command::new("sh")
            .args(["-c", "readlink /proc/$$/exe 2>/dev/null || echo unknown"])
            .current_dir(cwd)
            .env_remove("PATH"),
    )
    .expect("a native child finds sh with no PATH");
    assert!(native.status.success());
    let ran = String::from_utf8_lossy(&native.stdout).trim().to_string();
    assert!(!ran.contains("DECOY"), "{ran}");
    // What that file answers `--version` with, if anything: doctor's
    // line is either its version or the named silence of THAT file.
    let banner = Command::new(&ran)
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .unwrap_or_default()
                .trim()
                .to_string()
        });

    for (primary, legacy) in [(Some("sh"), None), (None, Some("sh"))] {
        let mut command = doctor(cwd);
        command
            .env_remove("PATH")
            .env("BROKKR_MARKS", &doctor_marks);
        if let Some(primary) = primary {
            command.env("BROKKR_DSH_BIN", primary);
        }
        if let Some(legacy) = legacy {
            command.env("FORGE_DSH_BIN", legacy);
        }
        let stdout = stdout_of(&mut command);
        assert!(
            !stdout.contains("SECURITY_CWD_DECOY"),
            "doctor executed a cwd decoy under an absent PATH:\n{stdout}"
        );
        assert_eq!(executed(&doctor_marks), Vec::<String>::new());
        let line = dsh_line(&stdout);
        if cfg!(target_os = "linux") {
            match &banner {
                Some(banner) => assert!(
                    line.starts_with(&format!("ok       dsh: {banner} · serves")),
                    "the default-search sh's own version: {line}"
                ),
                None => assert!(
                    line.starts_with(&format!("warn     dsh: binary '{ran}' not found — seats")),
                    "the default-search sh, selected and silent: {line}"
                ),
            }
        } else {
            assert!(
                !line.contains("PATH is absent") && !line.contains("is not on PATH"),
                "a default-search positive is a selection, not a refusal: {line}"
            );
        }
    }

    // Present-empty PATH: the one empty entry is cwd, to both.
    let native = native_lookup(cwd, "sh", Some("")).unwrap();
    assert!(String::from_utf8_lossy(&native.stdout).contains("SECURITY_CWD_DECOY_SH_9f3"));
    let stdout = stdout_of(
        doctor(cwd)
            .env("PATH", "")
            .env("BROKKR_DSH_BIN", "sh")
            .env("BROKKR_MARKS", &doctor_marks),
    );
    assert!(
        dsh_line(&stdout).starts_with("ok       dsh: SECURITY_CWD_DECOY_SH_9f3 · serves"),
        "the explicit empty entry selects cwd exactly when the native child does: {stdout}"
    );
    // Both probes ran the cwd file the empty entry names: the selected
    // `sh`, and then the `node` the composite looks up under the same
    // PATH — which is what `Command::new("node")` runs there too.
    assert_eq!(
        executed(&doctor_marks),
        vec![
            "SECURITY_CWD_DECOY_NODE_9f3".to_string(),
            "SECURITY_CWD_DECOY_SH_9f3".to_string()
        ]
    );
    std::fs::remove_file(doctor_marks.join("SECURITY_CWD_DECOY_NODE_9f3")).unwrap();
    std::fs::remove_file(doctor_marks.join("SECURITY_CWD_DECOY_SH_9f3")).unwrap();
    // And without the decoy, an empty PATH finds nothing for either.
    std::fs::remove_file(cwd.join("sh")).unwrap();
    let native = native_lookup(cwd, "sh", Some("")).unwrap_err();
    assert_eq!(native.kind(), std::io::ErrorKind::NotFound);
    let stdout = stdout_of(doctor(cwd).env("PATH", "").env("BROKKR_DSH_BIN", "sh"));
    assert!(
        dsh_line(&stdout).contains("'sh' is not on PATH"),
        "no fallback to the default search under a present, empty PATH: {stdout}"
    );

    // Explicit PATH: the independent positive identity.
    let explicit = cwd.join("explicit");
    version_script(&explicit, "sh", "DSH_EXPLICIT_SH_0.0.5");
    let stdout = stdout_of(
        doctor(cwd)
            .env("PATH", &explicit)
            .env("BROKKR_DSH_BIN", "sh"),
    );
    assert!(
        dsh_line(&stdout).starts_with("ok       dsh: DSH_EXPLICIT_SH_0.0.5 · serves"),
        "{stdout}"
    );

    // Node, with DSH safely selected by an explicit path: the measured
    // `#!/usr/bin/env node` core is admitted only when `node` is on the
    // same native search, and the cwd decoy is never it. A host without
    // node on its default search asserts the named refusal; the
    // positive remains pending there, never a passing skip.
    let (bin, home) = install_dsh(&cwd.join("install"));
    let native = spawn(
        Command::new("node")
            .args(["-p", "process.execPath"])
            .current_dir(cwd)
            .env_remove("PATH"),
    );
    let stdout = stdout_of(
        doctor(cwd)
            .env_remove("PATH")
            .env("BROKKR_DSH_BIN", &bin)
            .env("DSH_HOME", &home)
            .env("BROKKR_MARKS", &doctor_marks),
    );
    assert!(
        !stdout.contains("SECURITY_CWD_DECOY_NODE_9f3"),
        "doctor executed the cwd node decoy:\n{stdout}"
    );
    let line = dsh_line(&stdout);
    match native {
        Err(error) => {
            assert_eq!(error.kind(), std::io::ErrorKind::NotFound, "{error}");
            assert!(
                line.contains(
                    "its #! interpreter '/usr/bin/env' selects no 'node': the DSH layout is \
                     unreadable: 'node' is not on the default search path"
                ) && line.contains("(PATH is absent)"),
                "the DSH probe is refused before it can discover the missing node: {line}"
            );
            eprintln!(
                "PENDING: no node on this host's default search path; the absent-PATH Node \
                 positive was not established here"
            );
        }
        Ok(output) => {
            let ran = String::from_utf8_lossy(&output.stdout).trim().to_string();
            assert!(
                line.starts_with("ok       dsh:"),
                "DSH selected with node {ran} on the default search: {line}"
            );
        }
    }
}

/// Finding 2. `PATH=A:B` with a working `B/dsh`: an `A/dsh` naming a
/// nonexistent interpreter is executed past by a native child, which
/// runs B, and doctor refuses at A by that cause with NEITHER probed.
/// A self-symlink at A stops the native child with ELOOP, and doctor
/// refuses by that cause without executing B. An ordinary
/// non-executable A is the positive control: both walk on to B.
#[test]
fn an_obstructed_path_search_takes_the_explicit_safe_refusal() {
    let workspace = shipped_workspace();
    let cwd = workspace.path();
    let a = cwd.join("a");
    let b = cwd.join("b");
    const B_VERSION: &str = "DSH_B_SENTINEL_0.0.0-b";
    version_script(&b, "dsh", B_VERSION);
    let path = format!("{}:{}", a.display(), b.display());

    // Missing interpreter at A.
    let interpreter = cwd.join("no-such-interpreter");
    stage_executable(&a, "dsh", &format!("#!{}\n", interpreter.display()));
    let native = native_dsh(cwd, Some(&path)).unwrap();
    assert!(
        String::from_utf8_lossy(&native.stdout).contains(B_VERSION),
        "the native child walked past A to B"
    );
    let stdout = stdout_of(doctor(cwd).env("PATH", &path));
    assert!(
        !stdout.contains(B_VERSION),
        "doctor did not probe B in place of the obstructed A:\n{stdout}"
    );
    let line = dsh_line(&stdout);
    let expected = format!(
        "{}: its #! interpreter '{}' is missing:",
        a.join("dsh").display(),
        interpreter.display()
    );
    assert!(line.contains(&expected), "{line}");
    assert!(
        line.starts_with("warn     dsh: binary 'dsh' not found: "),
        "{line}"
    );

    // Self-symlink at A.
    std::fs::remove_file(a.join("dsh")).unwrap();
    std::os::unix::fs::symlink("dsh", a.join("dsh")).unwrap();
    let eloop = std::fs::metadata(a.join("dsh")).unwrap_err();
    let native = native_dsh(cwd, Some(&path)).unwrap_err();
    assert_eq!(
        native.raw_os_error(),
        eloop.raw_os_error(),
        "the native child stopped with ELOOP rather than running B: {native}"
    );
    let stdout = stdout_of(doctor(cwd).env("PATH", &path));
    assert!(!stdout.contains(B_VERSION), "{stdout}");
    let line = dsh_line(&stdout);
    let expected = format!(
        "{}: a symlink loop stops the lookup: {eloop}",
        a.join("dsh").display()
    );
    assert!(line.contains(&expected), "{line}");

    // Ordinary non-executable A: both walk on to B, and doctor's line is
    // B's version. B is a shell script, not a DSH core, so its composite
    // is unreadable by a named reason beside the version — informational,
    // because the shipped shape declares no digest.
    std::fs::remove_file(a.join("dsh")).unwrap();
    std::fs::write(a.join("dsh"), "#!/bin/sh\necho DSH_A_NOT_EXECUTABLE\n").unwrap();
    std::fs::set_permissions(a.join("dsh"), std::fs::Permissions::from_mode(0o644)).unwrap();
    let native = native_dsh(cwd, Some(&path)).unwrap();
    assert!(String::from_utf8_lossy(&native.stdout).contains(B_VERSION));
    let stdout = stdout_of(doctor(cwd).env("PATH", &path));
    let line = dsh_line(&stdout);
    assert!(
        line.starts_with(&format!("ok       dsh: {B_VERSION} · serves")),
        "B's own version, from the file B resolves to: {line}"
    );
    assert!(!line.contains("DSH_A_NOT_EXECUTABLE"), "{line}");
    assert!(
        line.contains("composite unreadable:") && line.contains("(no declared wrapper_digest)"),
        "{line}"
    );

    // Finding 4 of the second hold: a NATIVE image at A whose dynamic
    // loader is missing, and a script whose interpreter is that image.
    // The kernel refuses both at `execve` with ENOENT, the native child
    // walks on to B, and doctor refuses at A naming the loader — without
    // probing A, its interpreter or B. Metadata admitted both before.
    // The fixture patches only the `PT_INTERP` bytes of a copy of the
    // built binary, which is an ELF on Linux; elsewhere the cells are
    // pending, not passed.
    std::fs::remove_file(a.join("dsh")).unwrap();
    if !cfg!(target_os = "linux") {
        eprintln!("PENDING: the missing-loader cells need an ELF fixture; not established here");
        return;
    }
    assert!(!Path::new(MISSING_LOADER).exists());
    let broken = patch_elf_interpreter(
        Path::new(env!("CARGO_BIN_EXE_brokkr")),
        &cwd.join("native").join("broken"),
        MISSING_LOADER,
    );
    let doctor_marks = marks(cwd, "doctor");
    for (what, install) in [
        ("a native image with a missing loader", None),
        (
            "a script whose interpreter has a missing loader",
            Some(format!("#!{}\n", broken.display())),
        ),
    ] {
        let _ = std::fs::remove_file(a.join("dsh"));
        match &install {
            None => std::fs::hard_link(&broken, a.join("dsh")).unwrap(),
            Some(body) => {
                stage_executable(&a, "dsh", body);
            }
        }
        let native = native_dsh(cwd, Some(&path)).unwrap();
        assert!(
            String::from_utf8_lossy(&native.stdout).contains(B_VERSION),
            "{what}: the native child walked past A to B"
        );
        let stdout = stdout_of(
            doctor(cwd)
                .env("PATH", &path)
                .env("BROKKR_MARKS", &doctor_marks),
        );
        assert!(
            !stdout.contains(B_VERSION),
            "{what}: doctor did not probe B in place of the obstructed A:\n{stdout}"
        );
        assert_eq!(
            executed(&doctor_marks),
            Vec::<String>::new(),
            "{what}: doctor probed nothing"
        );
        let line = dsh_line(&stdout);
        let expected = match &install {
            None => format!(
                "{}: needs the ELF loader '{MISSING_LOADER}', which is missing:",
                a.join("dsh").display()
            ),
            Some(_) => format!(
                "{}: its #! interpreter '{}' needs the ELF loader '{MISSING_LOADER}', which is \
                 missing:",
                a.join("dsh").display(),
                broken.display()
            ),
        };
        assert!(line.contains(&expected), "{what}: {line}");
        assert!(
            line.starts_with("warn     dsh: binary 'dsh' not found: "),
            "{what}: {line}"
        );
    }
}

/// The overrides keep their precedence and their no-fallback meaning:
/// primary `BROKKR_DSH_BIN`, then legacy `FORGE_DSH_BIN`, then `PATH`; a
/// failed override never falls back to a PATH decoy; an absolute
/// override needs no `PATH` at all.
#[test]
fn explicit_overrides_keep_their_precedence_and_never_fall_back() {
    let workspace = shipped_workspace();
    let cwd = workspace.path();
    let primary = version_script(&cwd.join("primary"), "dsh", "DSH_PRIMARY_0.0.1");
    let legacy = version_script(&cwd.join("legacy"), "dsh", "DSH_LEGACY_0.0.2");
    let on_path = cwd.join("on-path");
    version_script(&on_path, "dsh", "DSH_PATH_DECOY_0.0.3");

    let line = |command: &mut Command| dsh_line(&stdout_of(command));

    // Both set: primary. Legacy alone: legacy. Neither: PATH.
    assert!(line(
        doctor(cwd)
            .env("PATH", &on_path)
            .env("BROKKR_DSH_BIN", &primary)
            .env("FORGE_DSH_BIN", &legacy)
    )
    .contains("DSH_PRIMARY_0.0.1"));
    assert!(line(
        doctor(cwd)
            .env("PATH", &on_path)
            .env("FORGE_DSH_BIN", &legacy)
    )
    .contains("DSH_LEGACY_0.0.2"));
    assert!(line(doctor(cwd).env("PATH", &on_path)).contains("DSH_PATH_DECOY_0.0.3"));

    // An absolute override with NO PATH is selected: an override is a
    // file, not a search.
    let absolute = line(
        doctor(cwd)
            .env_remove("PATH")
            .env("BROKKR_DSH_BIN", &primary),
    );
    assert!(absolute.contains("DSH_PRIMARY_0.0.1"), "{absolute}");

    // A failed override with a usable decoy on PATH: the decoy is never
    // tried, and the line names the override and the lookup's cause.
    let missing = cwd.join("missing").join("dsh");
    let failed = line(
        doctor(cwd)
            .env("PATH", &on_path)
            .env("BROKKR_DSH_BIN", &missing),
    );
    assert!(!failed.contains("DSH_PATH_DECOY_0.0.3"), "{failed}");
    let enoent = std::fs::metadata(&missing).unwrap_err();
    assert!(
        failed.starts_with(&format!(
            "warn     dsh: binary '{}' not found: the DSH layout is unreadable: {}: {enoent}",
            missing.display(),
            missing.display()
        )),
        "{failed}"
    );
}

/// A NATIVE image on `PATH` — not a script — is admitted as itself, and
/// doctor reports the version the native child prints for it. The
/// built `brokkr` binary is the image: a native child under the same
/// `PATH` runs it, and doctor's line carries that child's own first
/// line. This is the native-image half of the platform-native controls;
/// the script half is `an_obstructed_path_search_takes_the_explicit_safe_refusal`.
#[test]
fn a_native_image_on_path_is_selected_as_the_child_selects_it() {
    let workspace = shipped_workspace();
    let cwd = workspace.path();
    let native = cwd.join("native");
    std::fs::create_dir_all(&native).unwrap();
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_brokkr"), native.join("dsh")).unwrap();
    let child = native_dsh(cwd, native.to_str()).unwrap();
    assert!(child.status.success(), "the native child ran the image");
    let banner = String::from_utf8_lossy(&child.stdout);
    let banner = banner.lines().next().unwrap_or_default().trim().to_string();
    assert!(!banner.is_empty(), "the image answers --version");
    let line = dsh_line(&stdout_of(doctor(cwd).env("PATH", &native)));
    assert!(
        line.starts_with(&format!("ok       dsh: {banner} · serves")),
        "doctor's version is the native child's own first line: {line}"
    );
}

/// S2. A nonexistent override carrying a newline and an ANSI
/// clear-screen sequence reaches stdout ESCAPED: the spelling stays
/// recognizable, no raw control byte is printed, and no injected line
/// appears in the report.
#[test]
fn a_nonexistent_override_cannot_inject_terminal_control_bytes_through_doctor() {
    let workspace = shipped_workspace();
    let cwd = workspace.path();
    let injected = format!("{}/x\n\u{1b}[2Jdsh", cwd.display());
    let stdout = stdout_of(
        doctor(cwd)
            .env_remove("PATH")
            .env("BROKKR_DSH_BIN", &injected),
    );
    assert!(
        !stdout.contains('\u{1b}'),
        "a raw escape byte reached stdout:\n{stdout}"
    );
    assert!(
        !stdout
            .lines()
            .any(|line| line.starts_with("\u{1b}[2Jdsh") || line == "[2Jdsh"),
        "the newline opened an injected line:\n{stdout}"
    );
    let line = dsh_line(&stdout);
    let escaped = format!("{}/x[2Jdsh", cwd.display());
    assert!(
        line.starts_with(&format!(
            "warn     dsh: binary '{escaped}' not found: the DSH layout is unreadable: {escaped}: "
        )),
        "the escaped spelling, recognizable, in both the binary and the cause: {line}"
    );
    assert!(line.contains("No such file or directory"), "{line}");
}

/// A minimal DSH installation at D6's locators, enough for the sole
/// producer to return a real composite: the same fixture the doctor unit
/// suite uses, rebuilt here for the built binary.
fn install_dsh(root: &Path) -> (PathBuf, PathBuf) {
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
        br#"{"name":"@deepseek-ai/dsh","version":"0.1.5-rc.2","bin":{"dsh":"lib/bin.js"}}"#,
    );
    let bin = stage_executable(&pkg.join("lib"), "bin.js", "#!/usr/bin/env node\n");
    put(
        &core,
        "node_modules/.package-lock.json",
        br#"{"lockfileVersion":3,"packages":{"node_modules/@deepseek-ai/dsh":{"version":"0.1.5-rc.2","integrity":"sha512-CORE"}}}"#,
    );
    let home = root.join("home");
    let profile = home.join("profiles/headless");
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
    (bin, home)
}

/// Finding 7. The built doctor over a complete install reports its
/// composite; with only the plugin's `package.json` removed it reports
/// the bundle AND the missing file.
#[test]
fn removing_only_the_plugin_manifest_names_the_drifted_file() {
    let workspace = shipped_workspace();
    let cwd = workspace.path();
    let (bin, home) = install_dsh(&cwd.join("install"));
    // The child's whole PATH: a scripted `node`, so the producer's
    // version probe never depends on this machine's runtime.
    let shims = cwd.join("shims");
    stage_executable(&shims, "node", "#!/bin/sh\necho v22.23.2\n");
    let run = || {
        dsh_line(&stdout_of(
            doctor(cwd)
                .env("PATH", &shims)
                .env("BROKKR_DSH_BIN", &bin)
                .env("DSH_HOME", &home),
        ))
    };
    let complete = run();
    assert!(
        complete.starts_with("ok       dsh: v22.23.2 · serves"),
        "the selected executable's own version: {complete}"
    );
    assert!(
        complete.contains("· composite ") && complete.ends_with("(no declared wrapper_digest)"),
        "a readable composite: {complete}"
    );

    std::fs::remove_file(
        home.join("profiles/headless/node_modules/dsh-plugin-cli-session/package.json"),
    )
    .unwrap();
    let drifted = run();
    assert!(
        drifted.contains(
            "composite unreadable: the DSH layout is unreadable: bundle 'dsh-plugin-cli-session' \
             does not resolve: no package.json found (no declared wrapper_digest)"
        ),
        "the bundle and the file: {drifted}"
    );
}
