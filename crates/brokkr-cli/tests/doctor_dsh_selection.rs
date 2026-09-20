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
//! Unix only: the fixtures are shell scripts, symlinks, mode bits and
//! `:`-separated `PATH`s.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// What the cwd decoy prints, and must never reach doctor's output.
const SENTINEL: &str = "SECURITY_CWD_SENTINEL_9f3";

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

/// A script that answers `--version` with a distinguishable line.
fn version_script(dir: &Path, name: &str, version: &str) -> PathBuf {
    stage_executable(dir, name, &format!("#!/bin/sh\necho {version}\n"))
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

/// A native child's own lookup of `dsh` under the same cwd and `PATH`
/// doctor was given: the comparison every claim here is made against.
fn native_dsh(cwd: &Path, path: Option<&str>) -> std::io::Result<Output> {
    let mut command = Command::new("dsh");
    command.arg("--version").current_dir(cwd);
    match path {
        Some(path) => command.env("PATH", path),
        None => command.env_remove("PATH"),
    };
    spawn(&mut command)
}

/// S1. With no `PATH` and an executable `dsh` in cwd, doctor REFUSES —
/// the sentinel is not executed, the refusal names the absent `PATH` —
/// and a native child in the same state finds nothing.
///
/// The order of the assertions is the order of the claims: absence of
/// the sentinel first, because that is the defect; the named reason
/// second, because a refusal asserted as "some error" proves nothing.
/// On the adopted pre-fix selection path this test fails at the
/// no-sentinel assertion (recorded in the delivery account).
#[test]
fn absent_path_refuses_before_doctor_can_execute_a_cwd_sentinel() {
    let workspace = shipped_workspace();
    let cwd = workspace.path();
    version_script(cwd, "dsh", SENTINEL);

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

    let stdout = stdout_of(doctor(cwd).env_remove("PATH"));
    assert!(
        !stdout.contains(SENTINEL),
        "doctor executed the cwd dsh under an absent PATH:\n{stdout}"
    );
    let line = dsh_line(&stdout);
    assert!(
        line.contains("PATH is absent"),
        "the refusal names the absent PATH: {line}"
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
    // is the absent PATH's, not the missing file's.
    std::fs::remove_file(cwd.join("dsh")).unwrap();
    let stdout = stdout_of(doctor(cwd).env_remove("PATH"));
    assert!(dsh_line(&stdout).contains("PATH is absent"), "{stdout}");
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
