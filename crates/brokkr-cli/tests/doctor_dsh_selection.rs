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
/// empty the native child runs the decoy and doctor refuses at that
/// entry (the fourth hold's rule), executing nothing; without the decoy
/// the child finds nothing and doctor still refuses there. An explicit
/// `PATH` is the independent positive.
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

    // Present-empty PATH: the one empty entry is cwd to the native
    // child, which runs the decoy. Doctor refuses AT that entry — the
    // working directory is never selected and never skipped past
    // (fourth hold) — executes nothing, and does not fall back to the
    // absent-PATH default search. The marker directory is checked
    // before the report's prose, so no later refusal can hide a probe.
    //
    // The DSH override here names `dsh`, with a cwd `dsh` decoy, and
    // the cwd `sh` decoy is removed first: doctor's exec-adapter probe
    // of `sh` is a bare native `Command::new("sh")`, which an empty
    // `PATH` sends to the working directory exactly as it sends the
    // native child — a separate surface this slice's DSH selection does
    // not govern, recorded in the delivery account — and this cell
    // proves the DSH selection alone.
    std::fs::remove_file(cwd.join("sh")).unwrap();
    version_script(cwd, "dsh", "SECURITY_CWD_DECOY_DSH_9f3");
    let native = native_lookup(cwd, "dsh", Some("")).unwrap();
    assert!(String::from_utf8_lossy(&native.stdout).contains("SECURITY_CWD_DECOY_DSH_9f3"));
    let stdout = stdout_of(
        doctor(cwd)
            .env("PATH", "")
            .env("BROKKR_DSH_BIN", "dsh")
            .env("BROKKR_MARKS", &doctor_marks),
    );
    assert_eq!(
        executed(&doctor_marks),
        Vec::<String>::new(),
        "doctor executed a cwd decoy through the empty PATH entry:\n{stdout}"
    );
    assert!(!stdout.contains("SECURITY_CWD_DECOY"), "{stdout}");
    let line = dsh_line(&stdout);
    assert!(
        line.contains(
            "dsh: the platform's search would fall into the working directory: PATH entry 0 is \
             empty"
        ),
        "the explicit empty entry is refused where the native child ran the decoy: {line}"
    );
    assert!(
        line.starts_with("warn     dsh: binary 'dsh' not found: "),
        "{line}"
    );
    // And without the decoy the native child finds nothing, and doctor
    // still refuses at the cwd entry rather than reporting a miss or
    // searching the default path.
    std::fs::remove_file(cwd.join("dsh")).unwrap();
    let native = native_lookup(cwd, "dsh", Some("")).unwrap_err();
    assert_eq!(native.kind(), std::io::ErrorKind::NotFound);
    let stdout = stdout_of(
        doctor(cwd)
            .env("PATH", "")
            .env("BROKKR_DSH_BIN", "dsh")
            .env("BROKKR_MARKS", &doctor_marks),
    );
    assert_eq!(executed(&doctor_marks), Vec::<String>::new());
    let line = dsh_line(&stdout);
    assert!(
        line.contains("the platform's search would fall into the working directory")
            && !line.contains("PATH is absent")
            && !line.contains("is not on PATH"),
        "a missing cwd candidate is no permission to advance, and no fallback to the default \
         search under a present, empty PATH: {line}"
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
            // The native child SUCCEEDED, and what it printed is the
            // canonical identity of the runtime the default search
            // selected — not a version banner two installations can
            // share (review 2026-09-20, F7).
            assert!(
                output.status.success(),
                "the native default-search node exited {:?}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            );
            let ran = String::from_utf8_lossy(&output.stdout).trim().to_string();
            assert!(!ran.is_empty() && !ran.contains("DECOY"), "{ran}");
            // The installed launcher prints the runtime it runs under,
            // so doctor's version field IS the `process.execPath` of
            // the node doctor selected, probed and retained.
            assert!(
                line.starts_with(&format!("ok       dsh: {ran} · serves")),
                "doctor observed the very runtime the native child ran ({ran}): {line}"
            );
            let observed = line
                .trim_start_matches("ok       dsh: ")
                .split(" · ")
                .next()
                .expect("a version field");
            assert_eq!(
                std::fs::canonicalize(observed).unwrap(),
                std::fs::canonicalize(&ran).unwrap(),
                "the observed and native runtimes are one canonical file"
            );
            // The launcher's own output proves only what the launcher
            // ran; the COMPOSITE proves what the observation consumed.
            // A readable composite here means the retained `node` was
            // probed for the `node` line and the whole installation
            // read; the private retained field itself is asserted by
            // the protocol companion
            // `absent_path_node_identity_is_retained_by_the_composite`
            // (run `09ec8d81`, R5).
            assert!(
                line.contains("· composite ") && line.ends_with("(no declared wrapper_digest)"),
                "the observation consumed the retained runtime into a readable composite: {line}"
            );
            assert!(!line.contains("composite unreadable"), "{line}");
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

    // Self-symlink at A. What the SEARCH does with a loop is each C
    // library's own: glibc's `execvp` stops with ELOOP and never
    // reaches B (measured 2026-09-20), Apple's continues to B as it
    // continues past ENOENT. The running platform's own native control
    // is the oracle, never a fixed "Unix" outcome — AS1's corrected
    // cell (2026-09-20 spec defect, F3).
    std::fs::remove_file(a.join("dsh")).unwrap();
    std::os::unix::fs::symlink("dsh", a.join("dsh")).unwrap();
    let eloop = std::fs::metadata(a.join("dsh")).unwrap_err();
    let native = native_dsh(cwd, Some(&path));
    match native {
        Err(native) => {
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
        }
        Ok(native) => {
            let ran = String::from_utf8_lossy(&native.stdout).into_owned();
            assert!(
                ran.contains(B_VERSION),
                "this platform's search continued past the loop to B: {ran}"
            );
            let stdout = stdout_of(doctor(cwd).env("PATH", &path));
            assert!(
                dsh_line(&stdout).starts_with(&format!("ok       dsh: {B_VERSION} · serves")),
                "doctor selects exactly what that native control ran: {stdout}"
            );
        }
    }

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

/// R2 (review 2026-09-20). The Linux kernel hands `env` everything after
/// the interpreter as ONE argument: `#!/usr/bin/env reviewed extra`
/// makes `env` search for a program named `reviewed extra`. Doctor
/// inspected `A/reviewed` — a file the child never looked at — and
/// admitted A's script, while the native child walked `A/reviewed
/// extra`'s missing interpreter to `B/reviewed extra`, past D10's
/// refusal. Doctor now selects the program as the kernel spells it and
/// refuses at A's obstruction before any probe.
#[cfg(target_os = "linux")]
#[test]
fn an_env_argument_is_selected_as_the_kernel_hands_it_to_env() {
    let workspace = shipped_workspace();
    let cwd = workspace.path();
    let a = cwd.join("a");
    let b = cwd.join("b");
    let path = format!("{}:{}", a.display(), b.display());
    let missing = cwd.join("no-such-interpreter");
    stage_executable(&a, "dsh", "#!/usr/bin/env reviewed extra\n");
    stage_executable(&a, "reviewed extra", &format!("#!{}\n", missing.display()));
    version_script(&a, "reviewed", "DSH_A_REVIEWED_DECOY_0.0.1");
    version_script(&b, "reviewed extra", "DSH_B_REVIEWED_EXTRA_0.0.2");
    let doctor_marks = marks(cwd, "doctor");

    // The native fact: the child's env searched for `reviewed extra`,
    // found A's obstructed copy, and walked on to B's.
    let native = native_dsh(cwd, Some(&path)).unwrap();
    let ran = String::from_utf8_lossy(&native.stdout);
    assert!(
        ran.contains("DSH_B_REVIEWED_EXTRA_0.0.2"),
        "the native child ran B's `reviewed extra`: {ran}"
    );
    assert_eq!(
        executed(&marks(cwd, "oracle")),
        vec!["DSH_B_REVIEWED_EXTRA_0.0.2".to_string()]
    );

    // Doctor refuses at A's obstruction, naming the program as the
    // kernel spells it, and probes nothing: not A's script, not the
    // decoy `reviewed`, not B.
    let stdout = stdout_of(
        doctor(cwd)
            .env("PATH", &path)
            .env("BROKKR_MARKS", &doctor_marks),
    );
    assert_eq!(
        executed(&doctor_marks),
        Vec::<String>::new(),
        "doctor probed nothing:\n{stdout}"
    );
    assert!(
        !stdout.contains("DSH_B_REVIEWED_EXTRA") && !stdout.contains("DSH_A_REVIEWED_DECOY"),
        "{stdout}"
    );
    let line = dsh_line(&stdout);
    let enoent = std::fs::metadata(&missing).unwrap_err();
    let expected = format!(
        "{}: its #! interpreter '/usr/bin/env' selects no 'reviewed extra': the DSH layout is \
         unreadable: {}: its #! interpreter '{}' is missing: {enoent}",
        a.join("dsh").display(),
        a.join("reviewed extra").display(),
        missing.display()
    );
    assert!(line.contains(&expected), "{line}");
    assert!(
        line.starts_with("warn     dsh: binary 'dsh' not found: "),
        "{line}"
    );

    // F4 (review 2026-09-20), R3 (run `09ec8d81`) and R1 (review of run
    // `124cca78`), SECURITY. The `env` an interpreter IS, asked of the
    // FILE and never of a name, and the invocation it is established
    // under, which is the name `env` and nothing else. Doctor
    // recognized the measured form by the spelled basename, so
    // `env-alias` carried a launcher whose `node` was missing past
    // D10's refusal and doctor EXECUTED it; recognition by the canonical
    // basename was still recognition by name, and a COPY of `env`
    // hard-linked as `tools/uu_env` — the same bytes, no `env` name
    // anywhere — walked past it too; establishing that hard link from
    // its own prefixed name guessed which utility is installed as `env`
    // (uutils runs env under it, busybox copied to `env` runs nothing).
    // The spellings, each with its own native control and its own
    // marker directory:
    //
    // - `/usr/bin/env`, a symlink NAMED `env` elsewhere, and a
    //   byte-for-byte copy named `env`: the platform's env under the
    //   name `env`, the established invocation — doctor names A's
    //   obstruction and probes nothing;
    // - the `env-alias` and `link_env` symlinks and the `uu_env` and
    //   `myenv` hard links of the copy: the same file under another
    //   NAME, a dispatch doctor cannot establish without executing it —
    //   refused by that cause and probes nothing;
    // - an impostor named `env`: refused as such and probes nothing.
    let tools = cwd.join("tools");
    std::fs::create_dir_all(&tools).unwrap();
    let env_binary = PathBuf::from("/usr/bin/env");
    assert!(env_binary.is_file(), "this host has /usr/bin/env");
    let alias = tools.join("env-alias");
    std::os::unix::fs::symlink(&env_binary, &alias).unwrap();
    let linked = cwd.join("linked");
    std::fs::create_dir_all(&linked).unwrap();
    let linked_env = linked.join("env");
    std::os::unix::fs::symlink(&env_binary, &linked_env).unwrap();
    let copied_env = tools.join("env");
    std::fs::copy(&env_binary, &copied_env).unwrap();
    std::fs::set_permissions(&copied_env, std::fs::Permissions::from_mode(0o755)).unwrap();
    let uu_env = tools.join("uu_env");
    std::fs::hard_link(&copied_env, &uu_env).unwrap();
    {
        use std::os::unix::fs::MetadataExt;
        let (copy, link, system) = (
            std::fs::metadata(&copied_env).unwrap(),
            std::fs::metadata(&uu_env).unwrap(),
            std::fs::metadata(&env_binary).unwrap(),
        );
        assert_eq!((copy.dev(), copy.ino()), (link.dev(), link.ino()));
        assert_ne!((copy.dev(), copy.ino()), (system.dev(), system.ino()));
    }
    let impostor = stage_executable(&cwd.join("impostor"), "env", "#!/bin/sh\nexec \"$@\"\n");
    let nodes_a = cwd.join("node-a");
    let nodes_b = cwd.join("node-b");
    stage_executable(&nodes_a, "node", &format!("#!{}\n", missing.display()));
    version_script(&nodes_b, "node", "DSH_B_NODE_0.0.3");
    let launchers = cwd.join("launchers");
    let node_path = format!(
        "{}:{}:{}",
        launchers.display(),
        nodes_a.display(),
        nodes_b.display()
    );
    let obstruction = |interpreter: &Path| {
        format!(
            "{}: its #! interpreter '{}' selects no 'node': the DSH layout is unreadable: {}: \
             its #! interpreter '{}' is missing: {enoent}",
            launchers.join("dsh").display(),
            interpreter.display(),
            nodes_a.join("node").display(),
            missing.display()
        )
    };
    // Every other-name spelling of the platform's env file meets ONE
    // refusal, whatever `/usr/bin/env` resolves to on this host: the
    // file does not say which utility is installed as `env`, and the
    // utilities disagree on what another name runs — uutils runs env
    // under the hard-linked `uu_env`, busybox copied to `env` answers
    // `applet not found` under the same layout (the protocol companion
    // reproduces both on files it owns). Each spelling's native outcome
    // is recorded beside the refusal, never counted: on this host's
    // uutils `env`, `uu_env` runs B's node and `env-alias` is refused by
    // the utility itself.
    let myenv = tools.join("myenv");
    std::fs::hard_link(&copied_env, &myenv).unwrap();
    let link_env = tools.join("link_env");
    std::os::unix::fs::symlink(&env_binary, &link_env).unwrap();
    let unestablished = |interpreter: &Path| {
        format!(
            "{}: its #! interpreter '{}' is the platform's env utility invoked under the name \
             '{}', a dispatch this resolver does not establish without executing it",
            launchers.join("dsh").display(),
            interpreter.display(),
            interpreter.file_name().unwrap().to_str().unwrap()
        )
    };
    // R1, second sitting: a symlink NAMED `env` to the copy's `uu_env`
    // hard link is spelled `env` and is the platform's env by every
    // byte, and this host's uutils refuses it (`argv[0]` `env` against
    // executable name `uu_env`, exit 1, no stdout) where busybox
    // installed as `env` would run it: the implementations disagree,
    // so doctor refuses it naming the file that runs and probes nothing.
    // The same symlink to the copy NAMED `env` runs `env` everywhere and
    // is established.
    let renamed = cwd.join("renamed");
    std::fs::create_dir_all(&renamed).unwrap();
    let renamed_env = renamed.join("env");
    std::os::unix::fs::symlink(&uu_env, &renamed_env).unwrap();
    let viacopy = cwd.join("viacopy");
    std::fs::create_dir_all(&viacopy).unwrap();
    let viacopy_env = viacopy.join("env");
    std::os::unix::fs::symlink(&copied_env, &viacopy_env).unwrap();
    let runs_as = |interpreter: &Path, runs: &Path| {
        format!(
            "{}: its #! interpreter '{}' is the platform's env utility invoked under the name \
             'env' but running as the file '{}', whose own name is not env, a dispatch this \
             resolver does not establish without executing it",
            launchers.join("dsh").display(),
            interpreter.display(),
            std::fs::canonicalize(runs).unwrap().display()
        )
    };
    eprintln!(
        "R1: this host's env resolves to {}",
        std::fs::canonicalize(&env_binary).unwrap().display()
    );
    let impostor_refusal = format!(
        "{}: its #! interpreter '{}' is named env but is not the platform's env utility \
         '/usr/bin/env'",
        launchers.join("dsh").display(),
        impostor.display()
    );
    // The independent native control for one spelling: reaching a
    // program at all proves that native lookup SELECTED the launcher
    // and the kernel loaded it through this spelling — nothing was
    // walked past at the launcher — and where the platform's own `env`
    // went on to run a program it ran B's node, never A's obstructed
    // copy. An `env` that declines to answer to another name stops
    // there; that is this host's fact, recorded rather than asserted
    // away, and it does not weaken doctor's refusal beside it.
    let native_control = |interpreter: &Path, what: &str| -> String {
        let oracle_marks = marks(
            cwd,
            &format!(
                "oracle-{}-{}",
                interpreter.file_name().unwrap().to_str().unwrap(),
                what.replace(' ', "-")
            ),
        );
        let mut command = Command::new("dsh");
        command
            .arg("--version")
            .current_dir(cwd)
            .env("PATH", &node_path)
            .env("BROKKR_MARKS", &oracle_marks);
        let native = spawn(&mut command).unwrap();
        let ran = String::from_utf8_lossy(&native.stdout).into_owned();
        assert!(
            !ran.contains("DSH_A_") && !executed(&oracle_marks).iter().any(|m| m.contains("_A_")),
            "{what}: the native child ran nothing of A's: {ran}"
        );
        eprintln!(
            "R3 native control under {:?} ({what}): status {:?}, stdout {:?}, markers {:?}",
            interpreter.file_name().unwrap(),
            native.status,
            ran.trim(),
            executed(&oracle_marks)
        );
        ran
    };
    // The chief's copied-and-hard-linked spelling first: it is the one
    // name recognition of either kind admits and the prefixed-name rule
    // established, and the marker assertion that fails under that
    // removal names it.
    for (interpreter, expected) in [
        (&uu_env, unestablished(&uu_env)),
        (&link_env, unestablished(&link_env)),
        (&alias, unestablished(&alias)),
        (&myenv, unestablished(&myenv)),
        (&renamed_env, runs_as(&renamed_env, &uu_env)),
        (&env_binary, obstruction(&env_binary)),
        (&linked_env, obstruction(&linked_env)),
        (&copied_env, obstruction(&copied_env)),
        (&viacopy_env, obstruction(&viacopy_env)),
        (&impostor, impostor_refusal.clone()),
    ] {
        stage_executable(
            &launchers,
            "dsh",
            &format!("#!{} node\n", interpreter.display()),
        );
        native_control(interpreter, "obstructed A node");
        // Doctor refuses BEFORE probing anything, and leaves no
        // execution marker under any spelling: markers first, because
        // execution is the defect.
        let stdout = stdout_of(
            doctor(cwd)
                .env("PATH", &node_path)
                .env("BROKKR_MARKS", &doctor_marks),
        );
        assert_eq!(
            executed(&doctor_marks),
            Vec::<String>::new(),
            "doctor probed nothing under {interpreter:?}:\n{stdout}"
        );
        assert!(!stdout.contains("DSH_B_NODE_0.0.3"), "{stdout}");
        let line = dsh_line(&stdout);
        assert!(
            line.contains(&expected),
            "{}: {line}",
            interpreter.display()
        );
        assert!(
            line.starts_with("warn     dsh: binary 'dsh' not found: "),
            "{line}"
        );
    }
    // The valid-chain positives: with A's `node` gone, every
    // established spelling SELECTS the launcher, the native child runs
    // B's node through it, and doctor reports B's version from exactly
    // one probe. The other-name spellings and the impostor are still
    // refused with zero markers — a whole chain establishes no dispatch,
    // and on this uutils host the native child DOES run B's node through
    // `uu_env`: recorded, never counted, because the same layout runs
    // nothing under busybox and the file does not say which it is.
    std::fs::remove_file(nodes_a.join("node")).unwrap();
    let established = [&env_binary, &linked_env, &copied_env, &viacopy_env];
    let mut refused = vec![
        (&uu_env, unestablished(&uu_env)),
        (&alias, unestablished(&alias)),
        (&myenv, unestablished(&myenv)),
        (&link_env, unestablished(&link_env)),
        (&renamed_env, runs_as(&renamed_env, &uu_env)),
    ];
    for (round, interpreter) in established.into_iter().enumerate() {
        stage_executable(
            &launchers,
            "dsh",
            &format!("#!{} node\n", interpreter.display()),
        );
        let ran = native_control(interpreter, "valid chain");
        assert!(
            ran.contains("DSH_B_NODE_0.0.3"),
            "{}: the native child ran B's node through env: {ran}",
            interpreter.display()
        );
        let chain_marks = marks(cwd, &format!("chain-{round}"));
        let stdout = stdout_of(
            doctor(cwd)
                .env("PATH", &node_path)
                .env("BROKKR_MARKS", &chain_marks),
        );
        let line = dsh_line(&stdout);
        assert!(
            line.starts_with("ok       dsh: DSH_B_NODE_0.0.3 · serves"),
            "{}: the launcher runs through B's node once the chain is whole: {line}",
            interpreter.display()
        );
        assert_eq!(executed(&chain_marks), vec!["DSH_B_NODE_0.0.3".to_string()]);
    }
    refused.push((&impostor, impostor_refusal.clone()));
    for (interpreter, expected) in refused {
        stage_executable(
            &launchers,
            "dsh",
            &format!("#!{} node\n", interpreter.display()),
        );
        native_control(interpreter, "valid chain, unestablished invocation");
        let stdout = stdout_of(
            doctor(cwd)
                .env("PATH", &node_path)
                .env("BROKKR_MARKS", &doctor_marks),
        );
        assert_eq!(
            executed(&doctor_marks),
            Vec::<String>::new(),
            "doctor probed nothing under {interpreter:?}:\n{stdout}"
        );
        assert!(
            dsh_line(&stdout).contains(&expected),
            "{}: {stdout}",
            interpreter.display()
        );
    }
}

/// R1 (run `09ec8d81`, HIGH, security). Six `PATH` component lengths
/// ahead of a runnable B, each its own cell with its own native oracle
/// and its own doctor marker directory. On glibc, 255, 4096 and 5000
/// run B natively and doctor selects that exact B; 256, 300 and 4095
/// stop the native search with errno 36 and doctor leaves NO B marker —
/// where doctor walked past every `ENAMETOOLONG` and executed B before
/// the composite's refusal hid it. Markers are checked before the line,
/// because execution is the defect. Removing only the component is a
/// separately invoked native control and doctor run, both executing the
/// same B. Another libc's boundaries are recorded, not asserted from
/// glibc's numbers. Restoring blanket `ENAMETOOLONG` continuation fails
/// the 256/300/4095 no-marker assertion (recorded in the delivery
/// account).
#[test]
fn terminal_path_lengths_refuse_before_doctor_probe() {
    // The re-entered oracle child: `Command::new("dsh")` with the
    // environment the parent staged, unchanged — production's own form.
    if let Some(name) = std::env::var_os(INHERITED_ORACLE) {
        inherited_oracle_child(name.to_str().unwrap());
        return;
    }
    let workspace = shipped_workspace();
    let cwd = workspace.path();
    let b = cwd.join("b");
    let a = cwd.join("a");
    let a_empty = cwd.join("a-empty");
    const B_VERSION: &str = "DSH_B_LENGTH_SENTINEL_0.0.6";
    const A_VERSION: &str = "DSH_A_EARLIER_SENTINEL_0.0.8";
    const CWD_VERSION: &str = "DSH_CWD_LENGTH_SENTINEL_0.0.7";
    version_script(&b, "dsh", B_VERSION);
    std::fs::create_dir_all(&a_empty).unwrap();
    let glibc = cfg!(all(target_os = "linux", target_env = "gnu"));

    /// What the working directory holds under `dsh` for a cell.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum Cwd {
        Runnable,
        Loop,
        Absent,
    }
    /// The glibc expectation of a cell, for both oracle forms and doctor.
    #[derive(Clone, PartialEq, Eq, Debug)]
    enum Expect {
        /// Native runs B; doctor probes exactly B.
        RunsB,
        /// Native runs A; doctor probes exactly A.
        RunsA,
        /// Native stops with ENAMETOOLONG at the named candidate; doctor
        /// probes nothing and names that cause.
        Terminal36(String),
        /// Native runs the cwd file; doctor probes nothing and refuses
        /// by the cwd reason with this suffix.
        CwdRuns(String),
        /// Native stops with ELOOP on the cwd self-symlink; doctor
        /// probes nothing and preserves that cause.
        CwdLoop,
        /// Native walks past the empty cwd to B; doctor probes nothing —
        /// not B — and refuses by the cwd reason with this suffix.
        CwdAbsentB(String),
        /// Native walks past the empty cwd and finds nothing; doctor
        /// probes nothing and refuses by the cwd reason with this suffix.
        CwdAbsentNotFound(String),
    }
    let skip = |bytes: usize| {
        format!("glibc skips the {bytes}-byte component and its next iteration is the empty entry")
    };
    let entry = |index: usize| format!("PATH entry {index} is empty");
    let cwd_expect = |cwd: Cwd, suffix: String, absent_b: bool| match (cwd, absent_b) {
        (Cwd::Runnable, _) => Expect::CwdRuns(suffix),
        (Cwd::Loop, _) => Expect::CwdLoop,
        (Cwd::Absent, true) => Expect::CwdAbsentB(suffix),
        (Cwd::Absent, false) => Expect::CwdAbsentNotFound(suffix),
    };
    let terminal_at = |component: &str| {
        Expect::Terminal36(Path::new(component).join("dsh").display().to_string())
    };
    // A's spelling padded with `/` to exactly `bytes` bytes.
    let padded = |bytes: usize| {
        let spelled = a.display().to_string();
        format!("{spelled}{}", "/".repeat(bytes - spelled.len()))
    };

    /// One cell: its `PATH`, the working directory's state, whether
    /// `A/dsh` is placed, the expectation, and the same-fixture removal
    /// control's `PATH` and expectation where it has one.
    struct LengthCell {
        label: String,
        path: String,
        cwd: Cwd,
        a_present: bool,
        expect: Expect,
        removal: Option<(String, Expect)>,
    }
    let mut cells: Vec<LengthCell> = Vec::new();
    let b_only = b.display().to_string();
    for bytes in [255usize, 256, 300] {
        let component = "x".repeat(bytes);
        let expect = match bytes {
            255 => Expect::RunsB,
            _ => terminal_at(&component),
        };
        cells.push(LengthCell {
            label: format!("{bytes}-byte component, then B"),
            path: format!("{component}:{b_only}"),
            cwd: Cwd::Absent,
            a_present: false,
            expect,
            removal: Some((b_only.clone(), Expect::RunsB)),
        });
    }
    for cwd_state in [Cwd::Runnable, Cwd::Loop, Cwd::Absent] {
        for bytes in [4095usize, 4096, 5000] {
            let component = "x".repeat(bytes);
            let expect = match bytes {
                4095 => terminal_at(&component),
                _ => cwd_expect(cwd_state, skip(bytes), true),
            };
            cells.push(LengthCell {
                label: format!("{bytes}-byte component, then B; cwd {cwd_state:?}"),
                path: format!("{component}:{b_only}"),
                cwd: cwd_state,
                a_present: false,
                expect,
                removal: Some((b_only.clone(), Expect::RunsB)),
            });
        }
        let a_empty = a_empty.display().to_string();
        for (spelling, path, index, has_b) in [
            ("A::B", format!("{a_empty}::{b_only}"), 1, true),
            ("A:", format!("{a_empty}:"), 1, false),
            (":B", format!(":{b_only}"), 0, true),
            ("PATH=\"\"", String::new(), 0, false),
        ] {
            cells.push(LengthCell {
                label: format!("{spelling}; cwd {cwd_state:?}"),
                path,
                cwd: cwd_state,
                a_present: false,
                expect: cwd_expect(cwd_state, entry(index), has_b),
                removal: None,
            });
        }
    }
    // Earlier success ends the search before the empty entry.
    for (spelling, path) in [
        ("A::B", format!("{}::{b_only}", a.display())),
        ("A:", format!("{}:", a.display())),
    ] {
        cells.push(LengthCell {
            label: format!("{spelling}, A/dsh runnable; cwd Runnable"),
            path,
            cwd: Cwd::Runnable,
            a_present: true,
            expect: Expect::RunsA,
            removal: None,
        });
    }
    // The extra slash: A padded to 4092 bytes is a 4,096-byte candidate
    // with native's own `/`, one over what the kernel takes; one slash
    // fewer fits and selects A when present, B otherwise; one more still
    // refuses.
    for present in [true, false] {
        cells.push(LengthCell {
            label: format!(
                "A padded to 4092 bytes, then B; A/dsh {}",
                if present { "present" } else { "absent" }
            ),
            path: format!("{}:{b_only}", padded(4092)),
            cwd: Cwd::Absent,
            a_present: present,
            expect: Expect::Terminal36(format!("{}/dsh", padded(4092))),
            removal: Some((
                format!("{}:{b_only}", padded(4091)),
                if present {
                    Expect::RunsA
                } else {
                    Expect::RunsB
                },
            )),
        });
    }
    cells.push(LengthCell {
        label: "A padded to 4093 bytes, then B; A/dsh present".to_string(),
        path: format!("{}:{b_only}", padded(4093)),
        cwd: Cwd::Absent,
        a_present: true,
        expect: Expect::Terminal36(format!("{}/dsh", padded(4093))),
        removal: None,
    });

    let mut executed_cells = 0usize;
    for (
        index,
        LengthCell {
            label,
            path,
            cwd: cwd_state,
            a_present,
            expect,
            removal,
        },
    ) in cells.into_iter().enumerate()
    {
        // The fixtures of this cell: the cwd file, and A's.
        let _ = std::fs::remove_file(cwd.join("dsh"));
        match cwd_state {
            Cwd::Runnable => {
                version_script(cwd, "dsh", CWD_VERSION);
            }
            Cwd::Loop => std::os::unix::fs::symlink("dsh", cwd.join("dsh")).unwrap(),
            Cwd::Absent => {}
        }
        let _ = std::fs::remove_file(a.join("dsh"));
        if a_present {
            version_script(&a, "dsh", A_VERSION);
        }
        for (form_label, staged_path, expect) in
            [("cell", path.clone(), expect.clone())].into_iter().chain(
                removal
                    .into_iter()
                    .map(|(path, expect)| ("removed", path, expect)),
            )
        {
            let who = format!("{index}-{form_label}");
            let explicit_marks = marks(cwd, &format!("oracle-explicit-{who}"));
            let inherited_marks = marks(cwd, &format!("oracle-inherited-{who}"));
            let doctor_marks = marks(cwd, &format!("doctor-{who}"));
            let explicit = explicit_oracle(cwd, "dsh", Some(&staged_path), &explicit_marks);
            let inherited = inherited_oracle(cwd, "dsh", Some(&staged_path), &inherited_marks);
            let stdout = stdout_of(
                doctor(cwd)
                    .env("PATH", &staged_path)
                    .env("BROKKR_MARKS", &doctor_marks),
            );
            let line = dsh_line(&stdout);
            let describe = |what: &str| {
                format!(
                    "{label} [{form_label}]: {what}; explicit {explicit:?}, inherited \
                     {inherited:?}, doctor markers {:?}, line {line}",
                    executed(&doctor_marks)
                )
            };
            if !glibc {
                eprintln!(
                    "PENDING on {}: {}",
                    std::env::consts::OS,
                    describe("recorded")
                );
                continue;
            }
            executed_cells += 1;
            // Both forms are asserted on their own, then doctor: its
            // markers first, so a later refusal cannot hide a probe.
            let forms = [
                ("explicit", &explicit, &explicit_marks),
                ("inherited", &inherited, &inherited_marks),
            ];
            match &expect {
                Expect::RunsB | Expect::RunsA => {
                    let version = match expect {
                        Expect::RunsA => A_VERSION,
                        _ => B_VERSION,
                    };
                    for (form, native, oracle_marks) in forms {
                        assert_eq!(
                            native,
                            &Native::Ran(version.to_string()),
                            "{}",
                            describe(&format!("the {form} native child ran {version}"))
                        );
                        assert_eq!(executed(oracle_marks), vec![version.to_string()]);
                    }
                    assert_eq!(
                        executed(&doctor_marks),
                        vec![version.to_string()],
                        "{}",
                        describe("doctor probed exactly the file native ran")
                    );
                    assert!(
                        line.starts_with(&format!("ok       dsh: {version} · serves")),
                        "{}",
                        describe("doctor selects the file the native child ran")
                    );
                }
                refused => {
                    for (form, native, oracle_marks) in forms {
                        match refused {
                            Expect::Terminal36(_) => {
                                assert_eq!(
                                    native,
                                    &Native::Failed(36),
                                    "{}",
                                    describe(&format!(
                                        "the {form} native search stops with ENAMETOOLONG"
                                    ))
                                );
                                assert_eq!(executed(oracle_marks), Vec::<String>::new());
                            }
                            Expect::CwdLoop => {
                                assert_eq!(
                                    native,
                                    &Native::Failed(40),
                                    "{}",
                                    describe(&format!(
                                        "the {form} native search stops with ELOOP at cwd"
                                    ))
                                );
                                assert_eq!(executed(oracle_marks), Vec::<String>::new());
                            }
                            Expect::CwdRuns(_) => {
                                assert_eq!(
                                    native,
                                    &Native::Ran(CWD_VERSION.to_string()),
                                    "{}",
                                    describe(&format!("the {form} native child ran the cwd file"))
                                );
                                assert_eq!(executed(oracle_marks), vec![CWD_VERSION.to_string()]);
                            }
                            Expect::CwdAbsentB(_) => {
                                assert_eq!(
                                    native,
                                    &Native::Ran(B_VERSION.to_string()),
                                    "{}",
                                    describe(&format!(
                                        "the {form} native child walked past the empty cwd to B"
                                    ))
                                );
                                assert_eq!(executed(oracle_marks), vec![B_VERSION.to_string()]);
                            }
                            Expect::CwdAbsentNotFound(_) => {
                                assert_eq!(
                                    native,
                                    &Native::Failed(2),
                                    "{}",
                                    describe(&format!(
                                        "the {form} native child found nothing past the empty cwd"
                                    ))
                                );
                                assert_eq!(executed(oracle_marks), Vec::<String>::new());
                            }
                            Expect::RunsA | Expect::RunsB => unreachable!(),
                        }
                    }
                    // Doctor: NO marker of any kind — not cwd, not B,
                    // not A, not Node — checked before the prose.
                    assert_eq!(
                        executed(&doctor_marks),
                        Vec::<String>::new(),
                        "{}",
                        describe("doctor executed something where it must refuse")
                    );
                    for sentinel in [B_VERSION, A_VERSION, CWD_VERSION] {
                        assert!(
                            !stdout.contains(sentinel),
                            "{}",
                            describe("a sentinel reached the report")
                        );
                    }
                    assert!(
                        line.starts_with("warn     dsh: binary 'dsh' not found: "),
                        "{}",
                        describe("a refusal, with no selected file")
                    );
                    let expected = match refused {
                        Expect::Terminal36(candidate) => format!(
                            "{candidate}: metadata answers File name too long (os error 36), on \
                             which the platform's lookup stops"
                        ),
                        Expect::CwdLoop => {
                            "dsh: a symlink loop stops the lookup: Too many levels of symbolic \
                             links (os error 40)"
                                .to_string()
                        }
                        Expect::CwdRuns(suffix)
                        | Expect::CwdAbsentB(suffix)
                        | Expect::CwdAbsentNotFound(suffix) => format!(
                            "dsh: the platform's search would fall into the working directory: \
                             {suffix}"
                        ),
                        Expect::RunsA | Expect::RunsB => unreachable!(),
                    };
                    assert!(
                        line.contains(&expected),
                        "{}",
                        describe(&format!("doctor names the cause: {expected}"))
                    );
                }
            }
        }
    }
    if glibc {
        // 29 cells — three short lengths, three lengths × three cwd
        // states, four empty spellings × three cwd states, two
        // earlier-success controls, three padded spellings — and the
        // 14 same-fixture removal controls, each in both forms.
        assert_eq!(
            executed_cells, 43,
            "every cell and removal control ran on glibc"
        );
    }
}

/// The variable under which this test binary re-enters as an
/// inherited-form oracle: `Command::new(<name>)` with the environment
/// the parent staged, unchanged — the `posix_spawnp` form production's
/// probe takes.
const INHERITED_ORACLE: &str = "BROKKR_DOCTOR_INHERITED_ORACLE";

/// A completed native outcome of an oracle: the first line the sentinel
/// printed, or the spawn error's errno.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Native {
    Ran(String),
    Failed(i32),
}

/// The re-entered child's half: run the name as production runs it and
/// print one parseable line.
fn inherited_oracle_child(name: &str) {
    match Command::new(name).arg("--version").output() {
        Ok(output) => {
            assert!(
                output.status.success(),
                "the sentinel exited {:?}",
                output.status
            );
            println!(
                "\nORACLE ran {}",
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .next()
                    .unwrap_or_default()
            );
        }
        Err(error) => println!("\nORACLE failed {}", error.raw_os_error().unwrap_or(-1)),
    }
}

/// The explicit form: `PATH` set on the `Command` itself, which Rust
/// turns into `fork` and `execvp`.
fn explicit_oracle(cwd: &Path, name: &str, path: Option<&str>, marks: &Path) -> Native {
    let mut command = Command::new(name);
    command
        .arg("--version")
        .current_dir(cwd)
        .env("BROKKR_MARKS", marks);
    match path {
        Some(path) => command.env("PATH", path),
        None => command.env_remove("PATH"),
    };
    match spawn(&mut command) {
        Ok(output) => {
            assert!(
                output.status.success(),
                "the sentinel exited {:?}",
                output.status
            );
            Native::Ran(
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_string(),
            )
        }
        Err(error) => Native::Failed(error.raw_os_error().unwrap_or(-1)),
    }
}

/// The inherited form: this test binary re-entered as a child whose
/// environment carries the staged `PATH` and marker directory, running
/// `Command::new(name)` with no environment change of its own.
fn inherited_oracle(cwd: &Path, name: &str, path: Option<&str>, marks: &Path) -> Native {
    let mut child = Command::new(std::env::current_exe().unwrap());
    child
        .args([
            "terminal_path_lengths_refuse_before_doctor_probe",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ])
        .current_dir(cwd)
        .env(INHERITED_ORACLE, name)
        .env("BROKKR_MARKS", marks);
    match path {
        Some(path) => child.env("PATH", path),
        None => child.env_remove("PATH"),
    };
    let output = spawn(&mut child).unwrap();
    let said = String::from_utf8_lossy(&output.stdout).into_owned()
        + &String::from_utf8_lossy(&output.stderr);
    assert!(
        said.contains("1 passed") && output.status.success(),
        "the re-entered oracle ran its one case: {said}"
    );
    let line = said
        .lines()
        .find_map(|line| line.strip_prefix("ORACLE "))
        .expect("the oracle printed its outcome");
    match line.split_once(' ') {
        Some(("ran", first)) => Native::Ran(first.to_string()),
        Some(("failed", errno)) => Native::Failed(errno.parse().unwrap()),
        _ => panic!("an unparseable oracle line: {line}"),
    }
}

/// R2 (run `09ec8d81`). Apple's absent-`PATH` search is `_PATH_DEFPATH`,
/// `/usr/bin:/bin`, and NOT the `confstr(_CS_PATH)` answer
/// `/usr/bin:/bin:/usr/sbin:/sbin`: a harmless system executable that
/// lives only in `/usr/sbin` is NotFound to a native child with no
/// `PATH`, and doctor refuses it naming the default search it walked;
/// with the wider value as an explicit `PATH` both find it. A `sh` that
/// both searches hold cannot make this distinction. Native macOS only;
/// on a host without the fixture the cell is recorded pending.
#[cfg(target_os = "macos")]
#[test]
fn apple_default_search_excludes_confstr_only_directories() {
    let workspace = shipped_workspace();
    let cwd = workspace.path();
    const NAME: &str = "sysctl";
    let candidate = Path::new("/usr/sbin").join(NAME);
    if !candidate.is_file() {
        eprintln!(
            "PENDING: no {} on this host; the discriminating cell did not run",
            candidate.display()
        );
        return;
    }
    for default_dir in ["/usr/bin", "/bin"] {
        assert!(
            !Path::new(default_dir).join(NAME).exists(),
            "the fixture lives outside Apple's default search"
        );
    }
    let native = native_lookup(cwd, NAME, None).unwrap_err();
    assert_eq!(native.kind(), std::io::ErrorKind::NotFound, "{native}");
    let stdout = stdout_of(doctor(cwd).env_remove("PATH").env("BROKKR_DSH_BIN", NAME));
    let line = dsh_line(&stdout);
    assert!(
        line.contains(&format!(
            "'{NAME}' is not on the default search path /usr/bin:/bin (PATH is absent)"
        )),
        "doctor refuses by Apple's own default search: {line}"
    );
    let confstr = "/usr/bin:/bin:/usr/sbin:/sbin";
    native_lookup(cwd, NAME, Some(confstr)).expect("the wider search finds the fixture");
    let stdout = stdout_of(doctor(cwd).env("PATH", confstr).env("BROKKR_DSH_BIN", NAME));
    let line = dsh_line(&stdout);
    assert!(
        line.contains(&candidate.display().to_string()) && !line.contains("is not on"),
        "the explicit wider search selects the fixture: {line}"
    );
}

/// R4 (review 2026-09-20). A 40-byte Mach-O whose one load command is
/// `LC_LOAD_DYLINKER` with `cmdsize` 8 made the image reader panic on
/// the offset field it had not bounded, and the built doctor exited 101
/// with it. Doctor now reports the malformed command by name, exits as
/// a report does, and probes nothing.
#[test]
fn a_truncated_macho_on_path_is_refused_by_name_without_a_panic() {
    let workspace = shipped_workspace();
    let cwd = workspace.path();
    let on_path = cwd.join("on-path");
    let mut bytes = vec![0xcf, 0xfa, 0xed, 0xfe];
    bytes.extend_from_slice(&[0; 4]);
    bytes.extend_from_slice(&[0; 4]);
    bytes.extend_from_slice(&2u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&8u32.to_le_bytes());
    bytes.extend_from_slice(&[0; 8]);
    bytes.extend_from_slice(&0xeu32.to_le_bytes());
    bytes.extend_from_slice(&8u32.to_le_bytes());
    assert_eq!(bytes.len(), 40);
    // The CPU type is this target's own Mach-O type, so the reader
    // reaches the load command rather than refusing the CPU type first:
    // on Linux the image is then parsed whole and refused for the
    // command, and the kernel refuses it as no image at all either way.
    let cputype: u32 = if cfg!(target_arch = "aarch64") {
        0x0100_000c
    } else {
        0x0100_0007
    };
    bytes[4..8].copy_from_slice(&cputype.to_le_bytes());
    std::fs::create_dir_all(&on_path).unwrap();
    let staging = on_path.join(".dsh.staging");
    std::fs::write(&staging, &bytes).unwrap();
    std::fs::set_permissions(&staging, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::rename(&staging, on_path.join("dsh")).unwrap();
    let doctor_marks = marks(cwd, "doctor");

    // The native control: the kernel does not run a truncated image.
    // glibc's `execvp` retries an ENOEXEC file through `/bin/sh` when
    // the child's environment was changed, so the native answer is
    // either the exec-format error or a shell that exits nonzero.
    match native_dsh(cwd, on_path.to_str()) {
        Err(error) => assert_eq!(error.raw_os_error(), Some(8), "ENOEXEC: {error}"),
        Ok(output) => assert!(!output.status.success(), "{output:?}"),
    }

    let output = spawn(
        doctor(cwd)
            .env("PATH", &on_path)
            .env("BROKKR_MARKS", &doctor_marks),
    )
    .expect("the built doctor runs");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert_ne!(
        output.status.code(),
        Some(101),
        "doctor panicked:\n{stderr}"
    );
    assert!(!stderr.contains("panicked"), "{stderr}");
    assert_eq!(executed(&doctor_marks), Vec::<String>::new());
    let line = dsh_line(&stdout);
    let expected = format!(
        "{}: is not a loadable native image: a malformed Mach-O dynamic linker command",
        on_path.join("dsh").display()
    );
    assert!(line.contains(&expected), "{line}");
    assert!(
        line.starts_with("warn     dsh: binary 'dsh' not found: "),
        "{line}"
    );
}

/// R5 (review 2026-09-20). The `node` selected beside the executable is
/// the one the composite observes. Under `PATH=A:B` the selection of a
/// `#!/usr/bin/env node` core selects `A/node`; the version probe runs
/// it, and here `A/node` removes its own launcher after answering. The
/// composite then probes the retained `A/node` and refuses by name,
/// where a second lookup found `B/node` and produced a readable composite
/// for a runtime the selection never chose. B is never executed.
#[test]
fn the_composite_probes_the_node_the_selection_retained() {
    let workspace = shipped_workspace();
    let cwd = workspace.path();
    let (bin, home) = install_dsh(&cwd.join("install"));
    let a = cwd.join("a");
    let b = cwd.join("b");
    let doctor_marks = marks(cwd, "doctor");
    let oracle_marks = marks(cwd, "oracle");
    // `/bin/rm` by path: the child's `PATH` is A:B and nothing else.
    let self_removing =
        "#!/bin/sh\nif [ -n \"$BROKKR_MARKS\" ]; then : > \"$BROKKR_MARKS/NODE_A_RAN\"; \
                         fi\necho v22.23.2\n/bin/rm -f \"$0\"\n";
    stage_executable(&a, "node", self_removing);
    version_script(&b, "node", "NODE_B_SENTINEL");
    let path = format!("{}:{}", a.display(), b.display());

    // The native fact first: the version probe doctor makes, run as a
    // plain child, reaches A's node through `env`, and A's node answers
    // and removes itself.
    let native = spawn(
        Command::new(&bin)
            .arg("--version")
            .current_dir(cwd)
            .env("PATH", &path)
            .env("BROKKR_MARKS", &oracle_marks),
    )
    .unwrap();
    assert!(
        native.status.success(),
        "the native probe ran A's node: {}",
        String::from_utf8_lossy(&native.stderr)
    );
    assert!(String::from_utf8_lossy(&native.stdout).contains("v22.23.2"));
    assert!(
        !a.join("node").exists(),
        "A's node removed its own launcher"
    );
    assert_eq!(executed(&oracle_marks), vec!["NODE_A_RAN".to_string()]);
    stage_executable(&a, "node", self_removing);

    let line = dsh_line(&stdout_of(
        doctor(cwd)
            .env("PATH", &path)
            .env("BROKKR_DSH_BIN", &bin)
            .env("DSH_HOME", &home)
            .env("BROKKR_MARKS", &doctor_marks),
    ));
    assert!(
        !a.join("node").exists(),
        "A's runtime removed its own launcher while answering the version probe: {line}"
    );
    assert_eq!(
        executed(&doctor_marks),
        vec!["NODE_A_RAN".to_string()],
        "the version probe ran A's node once, and B's never: {line}"
    );
    assert!(
        line.starts_with("ok       dsh: v22.23.2 · serves"),
        "the version A's node answered: {line}"
    );
    let enoent = std::fs::metadata(a.join("node")).unwrap_err();
    assert!(
        line.contains(&format!(
            "composite unreadable: the DSH layout is unreadable: node --version: {enoent}"
        )),
        "the retained runtime, gone, is a named refusal rather than B's composite: {line}"
    );
    assert!(!line.contains("NODE_B_SENTINEL"), "{line}");
    // A readable composite renders `composite <digest> plugin <digest>`;
    // none was produced.
    assert!(!line.contains(" plugin "), "{line}");
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
    install_dsh_with(root, CORE_LAUNCHER)
}

/// What the installed core launcher is: D6's measured
/// `#!/usr/bin/env node` first line, and a body that prints the runtime
/// it is running under.
///
/// A scripted `node` shim never reads the body and answers its own
/// version; the REAL node answers with its own `process.execPath`, which
/// is what lets the absent-`PATH` positive compare the runtime doctor
/// observed against the one a native child ran, rather than compare
/// version banners two different installations can share (review
/// 2026-09-20, F7).
const CORE_LAUNCHER: &str =
    "#!/usr/bin/env node\nprocess.stdout.write(process.execPath + \"\\n\");\n";

/// `install_dsh` with a chosen launcher body, so a regression can make
/// the selected executable behave as a drifted or self-rewriting one.
fn install_dsh_with(root: &Path, launcher: &str) -> (PathBuf, PathBuf) {
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
    let bin = stage_executable(&pkg.join("lib"), "bin.js", launcher);
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

/// F6 (review 2026-09-20). The observation reads the bytes SELECTION
/// inspected, through the BUILT doctor and its real version probe.
///
/// A shell launcher that answers `v22.23.2` and rewrites itself to the
/// `env node` shebang while doing so was refused by the reading that
/// admitted it and admitted by the reading that followed: composition
/// reopened the launcher AFTER the probe had run it, so an installation
/// whose head the resolver never accepted reported a readable composite.
/// The otherwise identical launcher without the rewrite is the control,
/// and a valid `env node` launcher still reads. Restoring the post-probe
/// reread makes the rewriting launcher readable again, which is the
/// removal control recorded in the delivery account.
#[test]
fn the_composite_reuses_the_launcher_head_doctor_selected() {
    let workspace = shipped_workspace();
    let cwd = workspace.path();
    let shims = cwd.join("shims");
    stage_executable(&shims, "node", "#!/bin/sh\necho v22.23.2\n");
    let not_measured = |bin: &Path| {
        format!(
            "composite unreadable: the DSH layout is unreadable: {}: first line is not the env \
             node shebang",
            bin.canonicalize().unwrap().display()
        )
    };
    let run = |bin: &Path, home: &Path| {
        dsh_line(&stdout_of(
            doctor(cwd)
                .env("PATH", &shims)
                .env("BROKKR_DSH_BIN", bin)
                .env("DSH_HOME", home),
        ))
    };

    // The rewriting launcher. Its new bytes are staged and renamed in,
    // so the shell reading it keeps the file it started on, and the
    // mode is restored so the rewrite does not merely break the file.
    // The two utilities are spelled absolutely: doctor hands the probe
    // the PATH this test gives it, which holds the `node` shim alone.
    let utility = |name: &str| {
        ["/bin", "/usr/bin"]
            .into_iter()
            .map(|dir| PathBuf::from(dir).join(name))
            .find(|candidate| candidate.is_file())
            .unwrap_or_else(|| panic!("this host has {name}"))
            .display()
            .to_string()
    };
    let (chmod, mv) = (utility("chmod"), utility("mv"));
    let rewriting = format!(
        "#!/bin/sh\nprintf '#!/usr/bin/env node\\n' > \"$0.staging\"\n{chmod} 755 \
         \"$0.staging\"\n{mv} \"$0.staging\" \"$0\"\necho v22.23.2\n"
    );
    let (bin, home) = install_dsh_with(&cwd.join("rewriting"), &rewriting);
    let line = run(&bin, &home);
    assert!(
        line.starts_with("ok       dsh: v22.23.2 · serves"),
        "the launcher answered the probe: {line}"
    );
    assert_eq!(
        std::fs::read_to_string(&bin).unwrap(),
        "#!/usr/bin/env node\n",
        "the launcher rewrote itself to the measured shebang while answering the probe"
    );
    assert!(
        line.contains(&not_measured(&bin)),
        "the observation reads the first line selection inspected: {line}"
    );

    // The control: the same launcher without the rewrite. The file on
    // disk and the inspected head agree, and the refusal is the same.
    let (bin, home) = install_dsh_with(&cwd.join("plain"), "#!/bin/sh\necho v22.23.2\n");
    let line = run(&bin, &home);
    assert_eq!(
        std::fs::read_to_string(&bin).unwrap(),
        "#!/bin/sh\necho v22.23.2\n"
    );
    assert!(line.contains(&not_measured(&bin)), "{line}");

    // And the valid `env node` launcher reads, through the retained
    // head and nothing else.
    let (bin, home) = install_dsh(&cwd.join("valid"));
    let line = run(&bin, &home);
    assert!(
        line.starts_with("ok       dsh: v22.23.2 · serves") && line.contains("· composite "),
        "a readable composite: {line}"
    );
}

/// F5 (review 2026-09-20), SECURITY. The pnpm syntax rules through the
/// BUILT doctor over a complete installation: every malformed spelling
/// the review reproduced refuses by the pnpm component and a named
/// cause, and none of them reports the valid control's composite.
///
/// The bytes the reader IGNORES are the hole this closes. A NUL or a BEL
/// in a tarball, a colon-space or a control byte in a checksum, an
/// unterminated `deprecated` quote and an unterminated `engines` flow map
/// all reached the control's digest, because an ignored field was not
/// read and so was never admitted as syntax either.
#[test]
fn ignored_pnpm_values_are_admitted_as_syntax_through_the_built_doctor() {
    let workspace = shipped_workspace();
    let cwd = workspace.path();
    let shims = cwd.join("shims");
    stage_executable(&shims, "node", "#!/bin/sh\necho v22.23.2\n");
    let (bin, home) = install_dsh(&cwd.join("install"));
    let lock = home.join("profiles/headless/pnpm-lock.yaml");
    let run = |body: &str| {
        std::fs::write(&lock, body).unwrap();
        dsh_line(&stdout_of(
            doctor(cwd)
                .env("PATH", &shims)
                .env("BROKKR_DSH_BIN", &bin)
                .env("DSH_HOME", &home),
        ))
    };
    let package = "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n";
    let control = run(&format!(
        "{package}    resolution: {{integrity: sha512-D}}\n"
    ));
    let digest = control
        .split("composite ")
        .nth(1)
        .and_then(|rest| rest.split(' ').next())
        .expect("the control's composite digest")
        .to_string();
    assert_eq!(digest.len(), 64, "{control}");

    for (body, reason) in [
        (
            format!("{package}    resolution: {{integrity: sha512-D, tarball: http://x\u{0}}}\n"),
            "the character U+0000, which YAML's character set excludes",
        ),
        (
            format!("{package}    resolution: {{integrity: sha512-D, tarball: http://x\u{7}}}\n"),
            "the character U+0007, which YAML's character set excludes",
        ),
        (
            format!("{package}    resolution: {{integrity: sha512-D, tarball: 'http://x\u{0}'}}\n"),
            "the character U+0000, which YAML's character set excludes",
        ),
        (
            "lockfileVersion: '9.0'\n\npnpmfileChecksum: sha256-a\u{0}\n\npackages:\n\n  \
             debug@2.6.9:\n    resolution: {integrity: sha512-D}\n"
                .to_string(),
            "the character U+0000, which YAML's character set excludes",
        ),
        (
            "lockfileVersion: '9.0'\n\npnpmfileChecksum: sha256-a: b\n\npackages:\n\n  \
             debug@2.6.9:\n    resolution: {integrity: sha512-D}\n"
                .to_string(),
            "a malformed scalar for top-level key 'pnpmfileChecksum'",
        ),
        (
            format!("{package}    resolution: {{integrity: sha512-D}}\n    deprecated: 'oops\n"),
            "a package child 'deprecated' carrying the malformed quoted scalar ''oops'",
        ),
        (
            format!("{package}    resolution: {{integrity: sha512-D}}\n    engines: {{node: >=1\n"),
            "a package child 'engines' carrying the unterminated flow collection '{node: >=1'",
        ),
        // R4 (run `09ec8d81`): a MISSING member — leading, interior,
        // comma-only — and the bodies of ignored children and sections,
        // which were dropped or skipped before they could be refused.
        (
            format!("{package}    resolution: {{integrity: sha512-D}}\n    cpu: [,x64]\n"),
            "a package child 'cpu' carrying the flow collection '[,x64]' with a missing member \
             at position 1",
        ),
        (
            format!("{package}    resolution: {{integrity: sha512-D}}\n    cpu: [x64,,arm64]\n"),
            "a package child 'cpu' carrying the flow collection '[x64,,arm64]' with a missing \
             member at position 2",
        ),
        (
            format!("{package}    resolution: {{integrity: sha512-D}}\n    engines: {{,node: 22}}\n"),
            "a package child 'engines' carrying the flow collection '{,node: 22}' with a missing \
             member at position 1",
        ),
        (
            format!(
                "{package}    resolution: {{integrity: sha512-D}}\n    engines: {{node: 18,,npm: 9}}\n"
            ),
            "a package child 'engines' carrying the flow collection '{node: 18,,npm: 9}' with a \
             missing member at position 2",
        ),
        (
            format!("{package}    resolution: {{integrity: sha512-D}}\n    engines: {{,}}\n"),
            "a package child 'engines' carrying the flow collection '{,}' with a missing member \
             at position 1",
        ),
        (
            format!(
                "{package}    resolution: {{integrity: sha512-D}}\n    peerDependencies:\n      \
                 '@scope/peer': '>=1\n"
            ),
            "a line under the package child 'peerDependencies' carrying the entry '@scope/peer' \
             carrying the malformed quoted scalar ''>=1'",
        ),
        (
            format!(
                "{package}    resolution: {{integrity: sha512-D}}\n\nsnapshots:\n\n  debug@2.6.9:\n    \
                 dependencies:\n      ms: '2.0.0\n"
            ),
            "a line in section 'snapshots' carrying the entry 'ms' carrying the malformed quoted \
             scalar ''2.0.0'",
        ),
        // R3 (fourth hold): the STRUCTURE of an ignored body — a child
        // below a scalar, mapping and sequence members in one block,
        // a scalar `foo: 1` in `snapshots` with a deeper `bar: 2` —
        // refused through the built doctor by the responsible body and
        // its cause, never read as the control.
        (
            format!(
                "{package}    resolution: {{integrity: sha512-D}}\n    peerDependencies:\n      \
                 react: '>=16.8.0'\n        foo: bar\n"
            ),
            "a line under the package child 'peerDependencies' carrying the entry 'foo' nested \
             below the scalar entry 'react', which opens no block",
        ),
        (
            format!(
                "{package}    resolution: {{integrity: sha512-D}}\n    peerDependencies:\n      \
                 react: '>=16.8.0'\n      - foo\n"
            ),
            "a line under the package child 'peerDependencies' carrying the sequence item 'foo' \
             at 6 spaces beside mapping entries, which mixes mapping entries and sequence items \
             in one block",
        ),
        (
            format!(
                "{package}    resolution: {{integrity: sha512-D}}\n\nsnapshots:\n\n  foo: 1\n    \
                 bar: 2\n"
            ),
            "a line in section 'snapshots' carrying the entry 'bar' nested below the scalar \
             entry 'foo', which opens no block",
        ),
        (
            format!(
                "{package}    resolution: {{integrity: sha512-D}}\n\nsnapshots:\n\n  debug@2.6.9:\n      \
                 deep: 1\n    mid: 2\n"
            ),
            "a line in section 'snapshots' carrying the entry 'mid' at 4 spaces, which dedents \
             to no open block",
        ),
        // R2 (review of run `124cca78`): the four repeated keys the chief
        // ran through the built doctor — `peerDependencies.react`, a
        // flow-map `engines.node`, the `cpu` child and
        // `snapshots….dependencies.ms` — each retained the control's
        // composite before; each is refused by its responsible scope.
        (
            format!(
                "{package}    resolution: {{integrity: sha512-D}}\n    peerDependencies:\n      \
                 react: '>=16'\n      react: '>=17'\n"
            ),
            "a line under the package child 'peerDependencies' carrying the entry 'react' at 6 \
             spaces, which repeats a key of its block",
        ),
        (
            format!(
                "{package}    resolution: {{integrity: sha512-D}}\n    engines: {{node: '>=18', \
                 node: '>=20'}}\n"
            ),
            "a package child 'engines' carrying the flow map '{node: '>=18', node: '>=20'}' with \
             the repeated key 'node'",
        ),
        (
            format!("{package}    resolution: {{integrity: sha512-D}}\n    cpu: [x64]\n    cpu: [arm64]\n"),
            "a repeated package child 'cpu'",
        ),
        (
            format!(
                "{package}    resolution: {{integrity: sha512-D}}\n\nsnapshots:\n\n  debug@2.6.9:\n    \
                 dependencies:\n      ms: 2.0.0\n      ms: 2.0.0\n"
            ),
            "a line in section 'snapshots' carrying the entry 'ms' at 6 spaces, which repeats a \
             key of its block",
        ),
        // R2, second sitting: the chief's four YAML-equal pairs that a
        // text comparison called two keys — a padded `react :`, the
        // integers `11`/`0xB`, the booleans `true`/`True`, the nulls
        // `null`/`~` — each retained the control's composite; and the
        // pair it called one key, plain `true` and quoted `'true'`,
        // which are two. Padding is the separator's; a typed plain key
        // is refused by its cause before any comparison.
        (
            format!(
                "{package}    resolution: {{integrity: sha512-D}}\n    peerDependencies:\n      \
                 react: a\n      react : b\n"
            ),
            "a line under the package child 'peerDependencies' carrying the entry 'react' at 6 \
             spaces, which repeats a key of its block",
        ),
        (
            format!("{package}    resolution: {{integrity: sha512-D}}\n    engines: {{11: 1, 0xB: 2}}\n"),
            "a package child 'engines' carrying the flow map '{11: 1, 0xB: 2}' with the key '11', \
             which is a number and not a string",
        ),
        (
            format!(
                "{package}    resolution: {{integrity: sha512-D}}\n    peerDependencies:\n      \
                 true: a\n      True: b\n"
            ),
            "a line under the package child 'peerDependencies' carrying the key 'true', which is \
             a boolean and not a string",
        ),
        (
            format!("{package}    resolution: {{integrity: sha512-D}}\nsettings:\n  null: a\n  ~: b\n"),
            "a line in section 'settings' carrying the key 'null', which is a null and not a \
             string",
        ),
        (
            format!(
                "{package}    resolution: {{integrity: sha512-D}}\n    engines: {{true: a, 'true': b}}\n"
            ),
            "a package child 'engines' carrying the flow map '{true: a, 'true': b}' with the key \
             'true', which is a boolean and not a string",
        ),
    ] {
        let line = run(&body);
        assert!(
            line.contains(&format!(
                "composite unreadable: pnpm lock is unreadable: {reason}"
            )),
            "{reason}: {line}"
        );
        assert!(
            !line.contains(&digest),
            "the malformed lock never reports the valid control's composite: {line}"
        );
    }

    // The valid ignored spellings a real lock carries are still read,
    // and still ignored: the control's own digest, unchanged.
    for child in [
        "engines: {node: '>=18.12', npm: \"9\"}",
        "cpu: [x64, arm64]",
        "cpu: [x64,]",
        "engines: {}",
        "deprecated: Don't use this, use [debug] instead",
        "hasBin: true",
        "peerDependencies:\n      '@scope/peer': '>=1'\n      react: '>=16.8.0 || ^17'\n    \
         peerDependenciesMeta:\n      react:\n        optional: true",
        // One key in two sibling blocks is two keys (R2's control).
        "peerDependenciesMeta:\n      react:\n        optional: true\n      '@scope/peer':\n        \
         optional: true",
        // The keys YAML keeps apart (R2, second sitting): a quoted key
        // keeps its space, a single padded key is its own key, and the
        // quoted spellings of typed scalars are strings — two of them.
        "peerDependencies:\n      react: a\n      'react ': b",
        "peerDependencies:\n      react : '>=16'",
        "engines: {'true': a, 'True': b}",
    ] {
        let body = format!("{package}    resolution: {{integrity: sha512-D}}\n    {child}\n");
        assert!(
            run(&body).contains(&format!("composite {digest} ")),
            "{child:?}"
        );
    }
    // And the valid nested `snapshots` body a real lock carries — a
    // mapping block, a dedent back to its parent, a sequence block
    // beside it and a sibling record — is still read, and still
    // ignored: the control's own digest.
    let snapshots = format!(
        "{package}    resolution: {{integrity: sha512-D}}\n\nsnapshots:\n\n  debug@2.6.9:\n    \
         dependencies:\n      ms: 2.0.0\n    transitivePeerDependencies:\n      - supports-color\n  \
         ms@2.0.0: {{}}\n"
    );
    assert!(
        run(&snapshots).contains(&format!("composite {digest} ")),
        "{snapshots:?}"
    );
}
