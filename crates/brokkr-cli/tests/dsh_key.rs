//! A dsh folder says, for its dsh seats, which home serves them and
//! whether that home reaches the key their route reads (#717; the
//! operator's ruling of 2026-10-22, item 2, and the served-home model the
//! ruling of 2026-10-24 named): `brokkr init` ends with the check's lines
//! and `brokkr doctor` carries them in its status column, proved against
//! the BUILT binary under a PATH holding only a dsh stand-in, a dsh home of
//! the test's own, and each documented source planted in turn. The
//! scaffold's seats ride a measured route, so where their cold and
//! replacement shapes were measured on the host, as the shipped adapter
//! measures both on Linux (U1d6), one engine-only home serves their cold
//! and resumed launches alike. A seat on
//! a route no engine-only home was measured on is served from the
//! operator's home while the fence stands. A planted value is never
//! printed, and a planted `.env` is told present but unread.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::brokkr_folder::{Ran, Repo};

/// What every planted key holds: output carrying it is a leak.
const PLANTED: &str = "planted-value-717";

/// The scaffold's key line where the launching environment gives it.
const REACHED: &str =
    "dsh key deepseek-official: dsh reaches DEEPSEEK_API_KEY through the launching environment";

/// The scaffold's key line where the launching environment does not.
const UNSET: &str = "dsh key deepseek-official: dsh reaches no DEEPSEEK_API_KEY: Brokkr serves \
                     this route from its own engine-only home (decision 0065), which reads a key \
                     from the launching environment alone, and DEEPSEEK_API_KEY is not set there; \
                     export DEEPSEEK_API_KEY where brokkr runs";

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
            .env_remove("SPARK_API_KEY");
        command.envs(key.map(|key| ("DEEPSEEK_API_KEY", key)));
        let ran = Ran::of(command.output().unwrap());
        let printed = format!("{}{}", ran.stdout, ran.stderr);
        assert!(!printed.contains(PLANTED), "{printed}");
        ran
    }

    /// `brokkr init .`, and the check's lines it printed last; then the
    /// scaffold's dsh adapter's cold and replacement shapes, which the
    /// shipped adapter measured on Linux, measured on this host, so every
    /// later check reads the home a Linux host serves, on Linux and macOS
    /// alike.
    fn init(&self, key: Option<&str>) -> Vec<String> {
        let init = self.brokkr(&["init", "."], key);
        assert_eq!(init.code, Some(0), "{}", init.stderr);
        let adapter = self.repo.path(".brokkr/adapters/dsh.json");
        let scaffold = std::fs::read_to_string(&adapter).unwrap();
        let linux = r#""host": "linux""#;
        let here = format!(r#""host": "{}""#, std::env::consts::OS);
        let moved = scaffold.replace(linux, &here);
        let dsh: serde_json::Value = serde_json::from_str(&moved).unwrap();
        let shapes = dsh["mcp"]["shapes"].as_array().unwrap().iter();
        let read: Vec<_> = shapes
            .map(|shape| (&shape["invocation"], &shape["measured_on"]["host"]))
            .collect();
        let os = std::env::consts::OS.into();
        let expected = [(&"cold".into(), &os), (&"replacement".into(), &os)];
        assert_eq!(read, expected, "{scaffold}");
        std::fs::write(&adapter, moved).unwrap();
        let lines = init
            .stderr
            .lines()
            .skip_while(|line| !line.starts_with("dsh "));
        lines.map(str::to_string).collect()
    }

    /// What `brokkr init` prints of the scaffold's seats on this host, `key`
    /// the launching environment's: the shipped dsh adapter's cold and
    /// replacement shapes were measured on Linux, so there one engine-only
    /// home serves both launches, and on macOS, where they qualify nothing,
    /// the operator's home serves both (#717).
    fn initialised(&self, key: Option<&str>) -> [String; 2] {
        let (route, seats) = ("deepseek-official", "implement, intake");
        let macos = cfg!(target_os = "macos");
        let reach = match (key, macos) {
            (Some(_), _) => REACHED.to_string(),
            (None, false) => UNSET.to_string(),
            (None, true) => format!("dsh key {route}: {}", self.unreached("DEEPSEEK_API_KEY")),
        };
        match macos {
            true => [reach, operator(route, seats)],
            false => [reach, engine(route, seats)],
        }
    }

    /// `brokkr doctor`'s dsh key check lines, and its exit code.
    fn doctor(&self, key: Option<&str>) -> (Vec<String>, Option<i32>) {
        let doctor = self.brokkr(&["doctor"], key);
        let lines = doctor.stdout.lines().filter(|line| {
            ["dsh key ", "dsh home ", "dsh home:", "dsh workdir:"]
                .iter()
                .any(|name| line.contains(name))
        });
        (lines.map(str::to_string).collect(), doctor.code)
    }

    /// The words of a route whose `variable` no source the operator's home
    /// reads gives.
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

    /// Plant `text` at `path` under the home or the repository, readable by
    /// no one, so a check that opened it would fail.
    fn unreadable(&self, path: &PathBuf, text: &str) {
        std::fs::write(path, text).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o000)).unwrap();
    }
}

/// The home line of `seats` on `route` served from an engine-only home.
fn engine(route: &str, seats: &str) -> String {
    format!(
        "dsh home {route}: Brokkr serves seats [{seats}] on this route from its own engine-only \
         home (decision 0065), where dsh reads a key from the launching environment alone, never \
         the operator's .env or patch layers"
    )
}

/// The home line of `seats` on `route` served from the operator's home.
fn operator(route: &str, seats: &str) -> String {
    format!(
        "dsh home {route}: Brokkr serves seats [{seats}] on this route from the operator's own \
         dsh home, which reads every source this check asks"
    )
}

/// The failing home line of `seats` on dsh's default route, whose
/// engine-only launches the `.env` at `in_repo` refuses.
fn refused(seats: &str, in_repo: &Path) -> String {
    format!(
        "MISSING  dsh home deepseek-official: Brokkr serves seats [{seats}] on this route from \
         its own engine-only home (decision 0065), and refuses each launch while {} is present, \
         unread, since dsh would read it for a key; export the key where brokkr runs and remove \
         the file",
        in_repo.display()
    )
}

/// The scaffold's seats, whose cold and resumed launches one engine-only
/// home serves, are answered for that home, which reads a key from the
/// launching environment alone: one only the operator's home gives,
/// through its `.env` or a patch re-enabling dsh's credentials service, is
/// no key there, and fails the report.
#[test]
fn an_engine_only_seat_reads_its_key_from_the_launching_environment_alone() {
    let host = DshHost::new();
    let served = engine("deepseek-official", "implement, intake");
    assert_eq!(host.init(None), host.initialised(None));
    let missing = (
        vec![format!("MISSING  {UNSET}"), format!("ok       {served}")],
        Some(1),
    );
    assert_eq!(host.doctor(None), missing);
    // A variable set to nothing is no key.
    assert_eq!(host.doctor(Some("")), missing);
    assert_eq!(
        host.doctor(Some(PLANTED)).0,
        [format!("ok       {REACHED}"), format!("ok       {served}")]
    );
    let patch = host.home.join("profiles/headless/cordis.patch.yml");
    std::fs::write(&patch, "- id: credentials\n  disabled: false\n").unwrap();
    host.unreadable(
        &host.home.join(".env"),
        &format!("DEEPSEEK_API_KEY={PLANTED}\n"),
    );
    assert_eq!(host.doctor(None), missing);
}

/// A `.env` in the repository the scaffold's seats run in refuses each
/// engine-only launch, whatever the environment gives, and is never read.
#[test]
fn a_workdir_dotenv_refuses_every_engine_only_launch_and_is_never_read() {
    let host = DshHost::new();
    let initialised = host.initialised(Some(PLANTED));
    assert_eq!(host.init(Some(PLANTED)), initialised);
    let in_repo = host.repo.path(".env");
    host.unreadable(&in_repo, &format!("DEEPSEEK_API_KEY={PLANTED}\n"));
    let refused = refused("implement, intake", &in_repo);
    assert_eq!(host.doctor(Some(PLANTED)), (vec![refused], Some(1)));
}

/// An inline dsh seat, whose launch the compile carries no argv for, is
/// named by its site alone, and leaves the agent seat beside it served from
/// its engine-only home: a key only the operator's home gives still fails
/// the report, and a repository `.env` still refuses that seat's launch.
#[test]
fn an_inline_dsh_seat_leaves_the_agent_seats_home_standing() {
    let host = DshHost::new();
    host.init(Some(PLANTED));
    let bundle = host.repo.path(".brokkr/bundle.json");
    let seats = std::fs::read_to_string(&bundle).unwrap();
    let agent = r#""intake": {
      "agent": "intake","#;
    assert!(seats.contains(agent), "{seats}");
    let inline = r#""intake": {
      "role": "intake.md",
      "driver": {"command": ["{brokkr}", "driver", "dsh", "--", "--model", "deepseek-flash", "--effort", "high"]},"#;
    std::fs::write(&bundle, seats.replace(agent, inline)).unwrap();
    std::fs::write(host.repo.path(".brokkr/intake.md"), "Frame the request.\n").unwrap();
    let unmodelled = "warn     dsh home: Brokkr can't yet tell which home serves seats [intake], \
                      since the compile carries no argv for an inline dsh site's launch and no \
                      model for a candidate that pins no --model; each route no other seat rides \
                      is asked of every source the operator's home reads, with its caveat";
    let served = format!("ok       {}", engine("deepseek-official", "implement"));
    let missing = vec![format!("MISSING  {UNSET}"), served, unmodelled.to_string()];
    // The key the operator's home alone gives is no key for `implement`.
    host.unreadable(
        &host.home.join(".env"),
        &format!("DEEPSEEK_API_KEY={PLANTED}\n"),
    );
    assert_eq!(host.doctor(None), (missing, Some(1)));
    let in_repo = host.repo.path(".env");
    host.unreadable(&in_repo, &format!("DEEPSEEK_API_KEY={PLANTED}\n"));
    let beside = vec![refused("implement", &in_repo), unmodelled.to_string()];
    assert_eq!(host.doctor(Some(PLANTED)), (beside, Some(1)));
}

/// A seat whose model rides a route no engine-only home was measured on is
/// served from the operator's home while the fence stands (macOS), so every
/// layer that home reads is asked, in dsh's order, beside the seats an
/// engine-only home serves; past it (Linux) the route refuses its seats.
#[test]
fn a_seat_on_an_unmeasured_route_is_served_from_the_operators_home() {
    let host = DshHost::new();
    host.init(Some(PLANTED));
    let adapter = host.repo.path(".brokkr/adapters/dsh.json");
    let scaffold = std::fs::read_to_string(&adapter).unwrap();
    let pinned = "\"pro\": \"deepseek-v4-pro\"\n  },";
    assert!(scaffold.contains(pinned), "{scaffold}");
    let spark = "\"pro\": \"deepseek-v4-pro\",\n    \"spark\": \"spark/qwen3.8-flash\"\n  },\n  \
                 \"credentials\": {\"spark\": \"SPARK_API_KEY\"},";
    std::fs::write(&adapter, scaffold.replace(pinned, spark)).unwrap();
    let intake = host.repo.path(".brokkr/agents/intake.json");
    let hired = std::fs::read_to_string(&intake).unwrap();
    let (efforts, models) = (
        "\"flash\": \"high\",\n    \"pro\": \"high\"",
        "\"flash\",\n    \"pro\"",
    );
    assert!(hired.contains(efforts) && hired.contains(models), "{hired}");
    let hired = hired.replace(efforts, "\"spark\": \"high\"");
    std::fs::write(&intake, hired.replace(models, "\"spark\"")).unwrap();
    let operator = format!("ok       {}", operator("spark", "intake"));
    let implement = [
        format!("ok       {REACHED}"),
        format!("ok       {}", engine("deepseek-official", "implement")),
    ];
    // Past the fence, lifted on Linux (U9b1), no seat is served from the
    // operator's home: the unqualified route refuses its seats, and no key
    // is asked for them.
    if cfg!(target_os = "linux") {
        let refused = "MISSING  dsh home spark: Brokkr refuses to launch seats [intake] on this \
                       route: provider 'dsh' has no measured strict MCP configuration on route \
                       'spark'";
        let expected = [&implement[..], &[refused.to_string()]].concat();
        assert_eq!(host.doctor(Some(PLANTED)), (expected, Some(1)));
        return;
    }
    let spark = |key: String| [key, operator.clone()];
    let unreached = format!(
        "MISSING  dsh key spark: {}",
        host.unreached("SPARK_API_KEY")
    );
    assert_eq!(
        host.doctor(Some(PLANTED)),
        ([implement.clone(), spark(unreached)].concat(), Some(1))
    );
    let patch = host.home.join("profiles/headless/cordis.patch.yml");
    std::fs::write(&patch, "- id: credentials\n  disabled: false\n").unwrap();
    let service = format!(
        "warn     dsh key spark: dsh reaches SPARK_API_KEY through dsh's credentials service, \
         which {} re-enables; its store is dsh's to answer for and is not read",
        patch.display()
    );
    assert_eq!(
        host.doctor(Some(PLANTED)).0,
        [implement, spark(service)].concat()
    );
    // The repository's `.env` is told present but unread for the
    // operator's home, and refuses the engine-only one's launch.
    let in_repo = host.repo.path(".env");
    host.unreadable(&in_repo, &format!("SPARK_API_KEY={PLANTED}\n"));
    let untold = format!(
        "warn     dsh key spark: SPARK_API_KEY is not set in the launching environment, and {} \
         is present but unread: Brokkr does not read it, so it can't confirm SPARK_API_KEY is in \
         it",
        in_repo.display()
    );
    let refused = refused("implement", &in_repo);
    assert_eq!(
        host.doctor(Some(PLANTED)).0,
        [vec![refused], spark(untold).to_vec()].concat()
    );
}
