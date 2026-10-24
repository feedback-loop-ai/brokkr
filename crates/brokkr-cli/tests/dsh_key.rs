//! A dsh folder says whether dsh reaches a key for its route (#717; the
//! operator's ruling of 2026-10-22, item 2): `brokkr init` ends with the
//! check's lines and `brokkr doctor` carries them in its status column,
//! proved against the BUILT binary under a PATH holding only a dsh
//! stand-in, a dsh home of the test's own, and each documented source
//! planted in turn. Each route carries the served-home caveat (the
//! operator's ruling of 2026-10-24 on fix round 717d2's block). A planted
//! value is never printed, and a planted `.env` is told present but unread.
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
            .env_remove("DEEPSEEK_API_KEY")
            .env_remove("DASHSCOPE_API_KEY");
        command.envs(key.map(|key| ("DEEPSEEK_API_KEY", key)));
        let ran = Ran::of(command.output().unwrap());
        let printed = format!("{}{}", ran.stdout, ran.stderr);
        assert!(!printed.contains(PLANTED), "{printed}");
        ran
    }

    /// `brokkr init .`, and the check's lines it printed last.
    fn init(&self, key: Option<&str>) -> Vec<String> {
        let init = self.brokkr(&["init", "."], key);
        assert_eq!(init.code, Some(0), "{}", init.stderr);
        let lines = init
            .stderr
            .lines()
            .skip_while(|line| !line.starts_with("dsh "));
        lines.map(str::to_string).collect()
    }

    /// `brokkr doctor`'s dsh key check lines, and its exit code.
    fn doctor(&self, key: Option<&str>) -> (Vec<String>, Option<i32>) {
        let doctor = self.brokkr(&["doctor"], key);
        let lines = doctor.stdout.lines().filter(|line| {
            ["dsh key ", "dsh home ", "dsh workdir:"]
                .iter()
                .any(|name| line.contains(name))
        });
        (lines.map(str::to_string).collect(), doctor.code)
    }

    /// The words of a route whose `variable` no source gives.
    fn unreached(&self, variable: &str) -> String {
        let (root, home) = (self.repo.root.display(), self.home.display());
        format!(
            "dsh reaches no {variable}: it is not set in the launching environment, no .env file \
             is present at {root}/.env or {home}/.env, and dsh's shipped headless bundle leaves \
             its credentials service disabled; export {variable} where brokkr runs, or write the \
             row `- id: credentials` with `  disabled: false` in \
             {home}/profiles/headless/cordis.patch.yml and store {variable} through dsh's \
             credentials service"
        )
    }

    /// What a check asks past the environment where no source answers.
    fn everything(&self) -> String {
        let (root, home) = (self.repo.root.display(), self.home.display());
        format!(
            "then looked up {root}/.env and {home}/.env by name, unread, then read \
             {home}/profiles/headless/cordis.patch.yml then {home}/cordis.patch.yml for dsh's \
             credentials row"
        )
    }
}

/// The served-home caveat of a `route` reading `variable`, as printed:
/// `past` what its check asked after the environment, none where the
/// environment gave the key.
fn caveat(route: &str, variable: &str, past: Option<&str>) -> String {
    let unknown = "Brokkr can't yet tell whether it serves this route from its own engine-only \
                   home (decision 0065), which reads neither patch nor the home's .env";
    match past {
        None => format!(
            "dsh home {route}: this check asked only whether {variable} is set where brokkr runs, \
             and nothing of the operator's dsh home; {unknown}"
        ),
        Some(past) => format!(
            "dsh home {route}: this check asked whether {variable} is set where brokkr runs, \
             {past}; {unknown}, so past the environment this check can't confirm the key reaches \
             a seat served there"
        ),
    }
}

#[test]
fn init_and_doctor_name_an_unreached_key_with_both_remedies() {
    let host = DshHost::new();
    let detail = host.unreached("DEEPSEEK_API_KEY");
    let everything = host.everything();
    let served = caveat("deepseek-official", "DEEPSEEK_API_KEY", Some(&everything));
    assert_eq!(
        host.init(None),
        [
            format!("dsh key deepseek-official: {detail}"),
            served.clone()
        ]
    );
    let missing = (
        vec![
            format!("MISSING  dsh key deepseek-official: {detail}"),
            format!("warn     {served}"),
        ],
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
    let served = |past: Option<&str>| caveat("deepseek-official", "DEEPSEEK_API_KEY", past);
    let environment = reached("through the launching environment");
    assert_eq!(
        host.init(Some(PLANTED)),
        [environment.clone(), served(None)]
    );
    assert_eq!(
        host.doctor(Some(PLANTED)).0,
        [
            format!("ok       {environment}"),
            format!("warn     {}", served(None))
        ]
    );
    let warned = format!("warn     {}", served(Some(&host.everything())));
    // A key only the operator's home gives depends on the home that serves
    // the seat, so it warns and is never ok.
    let patch = host.home.join("profiles/headless/cordis.patch.yml");
    std::fs::write(&patch, "- id: credentials\n  disabled: false\n").unwrap();
    let service = format!(
        "through dsh's credentials service, which {} re-enables; its store is dsh's to answer \
         for and is not read",
        patch.display()
    );
    assert_eq!(
        host.doctor(None).0,
        [format!("warn     {}", reached(&service)), warned.clone()]
    );
    // A home patch composes after the profile's, and disables it again.
    let last = host.home.join("cordis.patch.yml");
    std::fs::write(&last, "- id: credentials\n  disabled: true\n").unwrap();
    let (lines, code) = host.doctor(None);
    let disables = format!("and {} disables dsh's credentials service;", last.display());
    assert_eq!(
        (
            lines[0].starts_with("MISSING  "),
            lines[0].contains(&disables),
            &lines[1],
            code
        ),
        (true, true, &warned, Some(1))
    );
    // A `.env` dsh reads is present but never read, so it is told as such,
    // and one in the repository a dsh seat runs in warns of the refusal an
    // engine-only home gives it, even where no one may read it.
    let in_repo = host.repo.path(".env");
    for env in [host.home.join(".env"), in_repo.clone()] {
        std::fs::write(&env, format!("DEEPSEEK_API_KEY={PLANTED}\n")).unwrap();
        std::fs::set_permissions(&env, std::fs::Permissions::from_mode(0o000)).unwrap();
        let mut untold = vec![
            format!(
                "warn     dsh key deepseek-official: DEEPSEEK_API_KEY is not set in the \
                 launching environment, and {} is present but unread: Brokkr does not read it, so \
                 it can't confirm DEEPSEEK_API_KEY is in it",
                env.display()
            ),
            // The check stops at the first `.env` present, before any patch.
            format!(
                "warn     {}",
                served(Some(&match env == in_repo {
                    true => format!("then looked up {} by name, unread", env.display()),
                    false => format!(
                        "then looked up {} and {} by name, unread",
                        in_repo.display(),
                        env.display()
                    ),
                }))
            ),
        ];
        untold.extend((env == in_repo).then(|| {
            format!(
                "warn     dsh workdir: {} is present, unread: if Brokkr serves a dsh seat from its \
                 own engine-only home (decision 0065), it refuses that launch, since dsh would \
                 read the file for a key",
                in_repo.display()
            )
        }));
        assert_eq!(host.doctor(None).0, untold);
    }
}

/// Every route carries the caveat, one that names no variable too.
#[test]
fn every_route_carries_the_served_home_caveat() {
    let host = DshHost::new();
    host.init(Some(PLANTED));
    let adapter = host.repo.path(".brokkr/adapters/dsh.json");
    let scaffold = std::fs::read_to_string(&adapter).unwrap();
    let pinned = "\"pro\": \"deepseek-v4-pro\"\n  },";
    assert!(scaffold.contains(pinned), "{scaffold}");
    let three = "\"pro\": \"deepseek-v4-pro\",\n    \"glm\": \"dashscope/glm-5.2\",\n    \
                 \"spark\": \"meta/meta/muse-spark-1.3\"\n  },\n  \
                 \"credentials\": {\"dashscope\": \"DASHSCOPE_API_KEY\"},";
    std::fs::write(&adapter, scaffold.replace(pinned, three)).unwrap();
    let detail = host.unreached("DASHSCOPE_API_KEY");
    assert_eq!(
        host.doctor(Some(PLANTED)),
        (
            vec![
                format!("MISSING  dsh key dashscope: {detail}"),
                format!(
                    "warn     {}",
                    caveat("dashscope", "DASHSCOPE_API_KEY", Some(&host.everything()))
                ),
                "ok       dsh key deepseek-official: dsh reaches DEEPSEEK_API_KEY through the \
                 launching environment"
                    .to_string(),
                format!(
                    "warn     {}",
                    caveat("deepseek-official", "DEEPSEEK_API_KEY", None)
                ),
                "warn     dsh key meta: the dsh adapter names no credential variable for this \
                 route, so whether dsh reaches a key for it is unknown; declare one under the \
                 adapter's `credentials`"
                    .to_string(),
                "warn     dsh home meta: this check asked nothing of the operator's dsh home, \
                 since no variable is named for this route; Brokkr can't yet tell whether it \
                 serves this route from its own engine-only home (decision 0065), which reads \
                 neither patch nor the home's .env"
                    .to_string(),
            ],
            Some(1)
        )
    );
}
