//! A dsh folder says whether dsh reaches a key for its route (#717; the
//! operator's ruling of 2026-10-22, item 2): `brokkr init` ends with the
//! check's line and `brokkr doctor` carries it in its status column,
//! proved against the BUILT binary under a PATH holding only a dsh
//! stand-in, a dsh home of the test's own, and each documented source
//! planted in turn. A planted value is never printed, and a planted `.env`
//! is told present but unread.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use crate::brokkr_folder::{Ran, Repo};

/// What every planted key holds: output carrying it is a leak.
const PLANTED: &str = "planted-value-717";

/// A repository, a PATH holding only an executable dsh stand-in, and an
/// empty dsh home, every one the test's own.
struct DshHost {
    repo: Repo,
    bins: PathBuf,
    home: PathBuf,
    _home: tempfile::TempDir,
}

impl DshHost {
    fn new() -> DshHost {
        let repo = Repo::new();
        let bins = repo.root.with_file_name("bins");
        std::fs::create_dir(&bins).unwrap();
        // Staged beside and renamed in, so no write descriptor is open
        // on the file a forked child executes.
        let staged = bins.join("dsh.staged");
        std::fs::write(&staged, "#!/bin/sh\necho '0.0.0 (stand-in)'\n").unwrap();
        std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::rename(&staged, bins.join("dsh")).unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("profiles/headless")).unwrap();
        DshHost {
            repo,
            bins,
            home: dir.path().canonicalize().unwrap(),
            _home: dir,
        }
    }

    /// `brokkr <args>` at the root, `key` the launching environment's key.
    fn brokkr(&self, args: &[&str], key: Option<&str>) -> Ran {
        let mut command = self.repo.command("");
        command
            .args(args)
            .env("PATH", &self.bins)
            .env("DSH_HOME", &self.home);
        command
            .env_remove("BROKKR_DSH_BIN")
            .env_remove("DEEPSEEK_API_KEY");
        command.envs(key.map(|key| ("DEEPSEEK_API_KEY", key)));
        let ran = Ran::of(command.output().unwrap());
        let printed = format!("{}{}", ran.stdout, ran.stderr);
        assert!(!printed.contains(PLANTED), "{printed}");
        ran
    }

    /// `brokkr init .`, and the last line it printed.
    fn init(&self, key: Option<&str>) -> String {
        let init = self.brokkr(&["init", "."], key);
        assert_eq!(init.code, Some(0), "{}", init.stderr);
        init.stderr.lines().last().unwrap().to_string()
    }

    /// `brokkr doctor`'s dsh key line, and its exit code.
    fn doctor(&self, key: Option<&str>) -> (String, Option<i32>) {
        let doctor = self.brokkr(&["doctor"], key);
        let lines: Vec<&str> = doctor
            .stdout
            .lines()
            .filter(|line| line.contains("dsh key"))
            .collect();
        assert_eq!(lines.len(), 1, "{}", doctor.stdout);
        (lines[0].to_string(), doctor.code)
    }
}

#[test]
fn init_and_doctor_name_an_unreached_key_with_both_remedies() {
    let host = DshHost::new();
    let line = host.init(None);
    let (root, home) = (host.repo.root.display(), host.home.display());
    let detail = format!(
        "dsh reaches no DEEPSEEK_API_KEY: it is not set in the launching environment, no .env \
         file is present at {root}/.env or {home}/.env, and dsh's shipped headless bundle leaves \
         its credentials service disabled; export DEEPSEEK_API_KEY where brokkr runs, or write \
         the row `- id: credentials` with `  disabled: false` in \
         {home}/profiles/headless/cordis.patch.yml and store DEEPSEEK_API_KEY through dsh's \
         credentials service"
    );
    assert_eq!(line, format!("dsh key deepseek-official: {detail}"));
    let missing = (
        format!("MISSING  dsh key deepseek-official: {detail}"),
        Some(1),
    );
    assert_eq!(host.doctor(None), missing);
    // A variable set to nothing is no key.
    assert_eq!(host.doctor(Some("")), missing);
}

#[test]
fn a_key_each_source_names_is_reached_and_its_value_never_printed() {
    let host = DshHost::new();
    let reached = |through: &str| {
        format!("dsh key deepseek-official: dsh reaches DEEPSEEK_API_KEY {through}")
    };
    let environment = reached("through the launching environment");
    assert_eq!(host.init(Some(PLANTED)), environment);
    assert_eq!(
        host.doctor(Some(PLANTED)).0,
        format!("ok       {environment}")
    );
    let patch = host.home.join("profiles/headless/cordis.patch.yml");
    std::fs::write(&patch, "- id: credentials\n  disabled: false\n").unwrap();
    let service = format!(
        "through dsh's credentials service, which {} re-enables; its store is dsh's to answer \
         for and is not read",
        patch.display()
    );
    assert_eq!(
        host.doctor(None).0,
        format!("ok       {}", reached(&service))
    );
    // A home patch composes after the profile's, and disables it again.
    let last = host.home.join("cordis.patch.yml");
    std::fs::write(&last, "- id: credentials\n  disabled: true\n").unwrap();
    let (line, code) = host.doctor(None);
    let disables = format!("and {} disables dsh's credentials service;", last.display());
    assert_eq!(
        (
            line.starts_with("MISSING  "),
            line.contains(&disables),
            code
        ),
        (true, true, Some(1))
    );
    // A `.env` dsh reads is present but never read, so it is told as such.
    for env in [host.home.join(".env"), host.repo.path(".env")] {
        std::fs::write(&env, format!("DEEPSEEK_API_KEY={PLANTED}\n")).unwrap();
        let untold = format!(
            "warn     dsh key deepseek-official: DEEPSEEK_API_KEY is not set in the launching \
             environment, and {} is present but unread: Brokkr does not read it, so it can't \
             confirm DEEPSEEK_API_KEY is in it",
            env.display()
        );
        assert_eq!(host.doctor(None).0, untold);
    }
}
