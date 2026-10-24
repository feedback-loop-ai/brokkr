//! Every answer the dsh key check gives (#717), each exactly, against
//! planted files under a home and a repository of the test's own; a
//! planted value never reaches a line, and no `.env` is ever opened.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::json;

use super::patch::{layer, writes, Unread, Write};
use super::*;

/// What a planted key holds: a line carrying it is a leak.
const PLANTED: &str = "planted-value-9f3";

/// A repository, a dsh home beside it and a folder's adapters, each a
/// directory of their own.
struct Planted {
    _dir: tempfile::TempDir,
    repo: PathBuf,
    home: PathBuf,
    adapters: PathBuf,
}

impl Planted {
    /// The three directories, the folder's dsh adapter pinning `models`
    /// with the route `credentials` it declares.
    fn new(models: serde_json::Value, credentials: serde_json::Value) -> Planted {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let (repo, home, adapters) = (root.join("repo"), root.join("home"), root.join("adapters"));
        for path in [&repo, &home.join("profiles/headless"), &adapters] {
            std::fs::create_dir_all(path).unwrap();
        }
        let dsh = json!({
            "provider": "dsh",
            "trust_tier": "untrusted",
            "egress": "uncontracted",
            "binary": "dsh",
            "driver": ["{brokkr}", "driver", "dsh", "--"],
            "models": models,
            "credentials": credentials,
            "judges": [],
            "model_flag": "--model",
            "efforts": [],
            "effort_flag": "unsupported",
            "tool_permissions": "unsupported",
            "mcp": "unsupported",
            "native_capabilities": {"unmeasured": "the test's"},
            "hands": {"unsupported": "the test's"},
        });
        std::fs::write(adapters.join("dsh.json"), dsh.to_string()).unwrap();
        Planted {
            _dir: dir,
            repo,
            home,
            adapters,
        }
    }

    /// The scaffold's: two bare ids, both on dsh's default route.
    fn scaffold() -> Planted {
        let models = json!({"flash": "deepseek-flash", "pro": "deepseek-v4-pro"});
        Planted::new(models, json!({}))
    }

    /// Plant `text` at `path`, in a directory [`Planted::new`] made.
    fn write(&self, path: &Path, text: &str) {
        std::fs::write(path, text).unwrap();
    }

    /// The headless profile's patch, which dsh composes first.
    fn profile(&self) -> PathBuf {
        self.home.join("profiles/headless/cordis.patch.yml")
    }

    /// The home's own patch, which dsh composes after the profile's.
    fn home_patch(&self) -> PathBuf {
        self.home.join("cordis.patch.yml")
    }

    /// The check's answers, `set` the environment's.
    fn reaches(&self, set: fn(&str) -> bool) -> Vec<Reach> {
        answers(&self.adapters, &self.repo, &self.home, set)
    }

    /// [`Planted::reaches`] with no variable set, [`within_bound`].
    fn reaches_within_bound(&self) -> Vec<Reach> {
        let (adapters, repo, home) = (self.adapters.clone(), self.repo.clone(), self.home.clone());
        within_bound(move || answers(&adapters, &repo, &home, unset))
    }

    /// The check's lines with their caveats, `set` the environment's,
    /// [`within_bound`].
    fn lines_within_bound(&self, set: fn(&str) -> bool) -> Vec<Line> {
        let (adapters, repo, home) = (self.adapters.clone(), self.repo.clone(), self.home.clone());
        within_bound(move || with_caveats(answers(&adapters, &repo, &home, set), &repo))
    }
}

/// `check` on a thread of its own, refused past a bound: a check that
/// opened a FIFO planted as a `.env` or a patch would wait on it for a
/// writer that never comes.
fn within_bound<T: Send + 'static>(check: impl FnOnce() -> T + Send + 'static) -> T {
    let (sent, answered) = std::sync::mpsc::channel();
    std::thread::spawn(move || sent.send(check()));
    let answer = answered.recv_timeout(Duration::from_secs(10));
    answer.expect("the check opened a planted FIFO and waited on it")
}

/// The check's answers for a folder's `adapters`, `repo` and dsh `home`.
fn answers(adapters: &Path, repo: &Path, home: &Path, set: fn(&str) -> bool) -> Vec<Reach> {
    let sources = Sources {
        set,
        repo,
        home: Ok(home.to_path_buf()),
    };
    in_folder(adapters, &sources)
}

fn unset(_: &str) -> bool {
    false
}

/// A FIFO at `path`, which an open for reading waits on.
fn fifo(path: &Path) {
    let (fifo, mode) = (
        rustix::fs::FileType::Fifo,
        rustix::fs::Mode::from_raw_mode(0o600),
    );
    rustix::fs::mknodat(rustix::fs::CWD, path, fifo, mode, 0).unwrap();
}

fn reached(source: Source) -> Vec<Reach> {
    vec![Reach::Reached {
        route: "deepseek-official".into(),
        variable: "DEEPSEEK_API_KEY".into(),
        source,
    }]
}

fn untold(why: Untold) -> Vec<Reach> {
    vec![Reach::Untold {
        route: "deepseek-official".into(),
        variable: "DEEPSEEK_API_KEY".into(),
        why,
    }]
}

/// Each line as both surfaces print it, which must never hold the value.
fn lines(reaches: &[Reach]) -> Vec<String> {
    let lines: Vec<String> = reaches
        .iter()
        .map(|reach| format!("{}: {}", reach.what(), reach.detail()))
        .collect();
    assert!(
        lines.iter().all(|line| !line.contains(PLANTED)),
        "{lines:?}"
    );
    lines
}

#[test]
fn a_scaffold_with_no_source_is_unreached_and_names_both_remedies() {
    let planted = Planted::scaffold();
    let (repo, home) = (planted.repo.display(), planted.home.display());
    assert_eq!(
        lines(&planted.reaches(unset)),
        [format!(
            "dsh key deepseek-official: dsh reaches no DEEPSEEK_API_KEY: it is not set in the \
             launching environment, no .env file is present at {repo}/.env or {home}/.env, and \
             dsh's shipped headless bundle leaves its credentials service disabled; export \
             DEEPSEEK_API_KEY where brokkr runs, or write the row `- id: credentials` with \
             `  disabled: false` in {home}/profiles/headless/cordis.patch.yml and store \
             DEEPSEEK_API_KEY through dsh's credentials service"
        )]
    );
    // A patch that leaves the service disabled is no key either, and is
    // named as the layer that disables it.
    planted.write(&planted.profile(), "- id: credentials\n  disabled: true\n");
    let off = Off::Layer(planted.profile());
    assert_eq!(
        planted.reaches(unset),
        [Reach::Unreached {
            route: "deepseek-official".into(),
            variable: "DEEPSEEK_API_KEY".into(),
            files: vec![planted.repo.join(".env"), planted.home.join(".env")],
            off: Ok(off),
        }]
    );
}

#[test]
fn a_key_in_the_launching_environment_is_reached_and_not_echoed() {
    let planted = Planted::scaffold();
    let answers = planted.reaches(|name| name == "DEEPSEEK_API_KEY");
    assert_eq!(answers, reached(Source::Environment));
    assert_eq!(
        lines(&answers),
        [
            "dsh key deepseek-official: dsh reaches DEEPSEEK_API_KEY through the launching \
          environment"
        ]
    );
}

#[test]
fn a_present_dotenv_is_reported_unread_and_never_opened() {
    let planted = Planted::scaffold();
    let at_home = planted.home.join(".env");
    planted.write(&at_home, &format!("DEEPSEEK_API_KEY={PLANTED}\n"));
    let answers = planted.reaches_within_bound();
    assert_eq!(answers, untold(Untold::Dotenv(at_home.clone())));
    assert_eq!(
        lines(&answers),
        [format!(
            "dsh key deepseek-official: DEEPSEEK_API_KEY is not set in the launching environment, \
             and {} is present but unread: Brokkr does not read it, so it can't confirm \
             DEEPSEEK_API_KEY is in it",
            at_home.display()
        )]
    );
    // The repository's own `.env` is asked first, and a FIFO there, which
    // an open would wait on, or one no one may read, is told as one that
    // holds the key: the check never opens it.
    let in_repo = planted.repo.join(".env");
    fifo(&in_repo);
    assert_eq!(
        planted.reaches_within_bound(),
        untold(Untold::Dotenv(in_repo.clone()))
    );
    std::fs::remove_file(&in_repo).unwrap();
    planted.write(&in_repo, &format!("DEEPSEEK_API_KEY={PLANTED}\n"));
    std::fs::set_permissions(&in_repo, std::fs::Permissions::from_mode(0o000)).unwrap();
    assert_eq!(
        planted.reaches_within_bound(),
        untold(Untold::Dotenv(in_repo))
    );
}

#[test]
fn a_profile_that_re_enables_the_service_is_reached_through_it() {
    let planted = Planted::scaffold();
    let patch = planted.profile();
    planted.write(
        &patch,
        "- id: other\n- id: credentials\n  disabled: false\n",
    );
    let answers = planted.reaches(unset);
    assert_eq!(answers, reached(Source::Service(patch.clone())));
    assert_eq!(
        lines(&answers),
        [format!(
            "dsh key deepseek-official: dsh reaches DEEPSEEK_API_KEY through dsh's credentials \
             service, which {} re-enables; its store is dsh's to answer for and is not read",
            patch.display()
        )]
    );
}

#[test]
fn the_home_patch_composes_after_the_profile_patch() {
    let planted = Planted::scaffold();
    let (profile, home) = (planted.profile(), planted.home_patch());
    planted.write(&profile, "- id: credentials\n  disabled: false\n");
    planted.write(&home, "- id: credentials\n  disabled: true\n");
    let (repo_env, home_env) = (planted.repo.join(".env"), planted.home.join(".env"));
    assert_eq!(
        lines(&planted.reaches(unset)),
        [format!(
            "dsh key deepseek-official: dsh reaches no DEEPSEEK_API_KEY: it is not set in the \
             launching environment, no .env file is present at {} or {}, and {} disables dsh's \
             credentials service; export DEEPSEEK_API_KEY where brokkr runs, or write the row \
             `- id: credentials` with `  disabled: false` in {} and store DEEPSEEK_API_KEY \
             through dsh's credentials service",
            repo_env.display(),
            home_env.display(),
            home.display(),
            home.display()
        )]
    );
    planted.write(&profile, "- id: credentials\n  disabled: true\n");
    planted.write(&home, "- id: other\n- id: credentials\n  disabled: false\n");
    assert_eq!(
        planted.reaches(unset),
        reached(Source::Service(home.clone()))
    );
    // A home patch that writes nothing of the service leaves the
    // profile's answer standing.
    planted.write(&home, "- id: other\n  disabled: true\n");
    assert_eq!(
        planted.reaches(unset)[0],
        Reach::Unreached {
            route: "deepseek-official".into(),
            variable: "DEEPSEEK_API_KEY".into(),
            files: vec![repo_env, home_env],
            off: Ok(Off::Layer(profile)),
        }
    );
}

#[test]
fn a_patch_layer_that_cannot_be_read_is_untold_and_never_ok() {
    let planted = Planted::scaffold();
    let (profile, home) = (planted.profile(), planted.home_patch());
    // Invalid YAML after a good row.
    planted.write(
        &profile,
        "- id: credentials\n  disabled: false\nthis: is: not a row\n",
    );
    let answers = planted.reaches(unset);
    assert_eq!(
        answers,
        untold(Untold::Patch(profile.clone(), Unread::Line(3)))
    );
    assert_eq!(
        lines(&answers),
        [format!(
            "dsh key deepseek-official: DEEPSEEK_API_KEY is not set in the launching environment \
             and no .env file dsh reads is present, but whether a patch re-enables dsh's \
             credentials service can't be told: {}: line 3 is not a row Brokkr reads (a \
             top-level `- key: value` and its own `  key: value` members, each value one plain \
             scalar)",
            profile.display()
        )]
    );
    // A `credentials` row nested under a mapping key is not a top-level
    // row, and is not read past.
    planted.write(&profile, "config:\n- id: credentials\n  disabled: false\n");
    assert_eq!(
        planted.reaches(unset),
        untold(Untold::Patch(profile.clone(), Unread::Line(1)))
    );
    // A home patch that cannot be read is untold after an enabling
    // profile, and a FIFO in its place is never opened.
    planted.write(&profile, "- id: credentials\n  disabled: false\n");
    fifo(&home);
    assert_eq!(
        planted.reaches_within_bound(),
        untold(Untold::Patch(home, Unread::NotAFile))
    );
}

#[test]
fn a_patch_layer_is_read_only_from_a_bounded_utf8_regular_file() {
    let planted = Planted::scaffold();
    let patch = planted.profile();
    assert_eq!(layer(&patch), Ok(None));
    assert_eq!(layer(&planted.home), Err(Unread::NotAFile));
    let under_a_file = planted.adapters.join("dsh.json").join("cordis.patch.yml");
    assert_eq!(
        layer(&under_a_file),
        Err(Unread::Io(std::io::ErrorKind::NotADirectory))
    );
    std::fs::write(&patch, [0xff, b'\n']).unwrap();
    assert_eq!(layer(&patch), Err(Unread::NotUtf8));
    let large = "# a comment\n".repeat(64 * 1024 / 12 + 1);
    planted.write(&patch, &large);
    assert_eq!(layer(&patch), Err(Unread::TooLarge));
    planted.write(&patch, "- id: credentials\n  disabled: false\n");
    assert_eq!(layer(&patch), Ok(Some(Write::Enables)));
    std::fs::set_permissions(&patch, std::fs::Permissions::from_mode(0o000)).unwrap();
    if !rustix::process::geteuid().is_root() {
        let denied = Err(Unread::Io(std::io::ErrorKind::PermissionDenied));
        assert_eq!(layer(&patch), denied);
    }
    let words: Vec<String> = [
        Unread::NotAFile,
        Unread::Io(std::io::ErrorKind::NotADirectory),
        Unread::TooLarge,
        Unread::NotUtf8,
        Unread::Repeated(3),
        Unread::NoId(2),
        Unread::Disabled(2),
    ]
    .iter()
    .map(ToString::to_string)
    .collect();
    assert_eq!(
        words,
        [
            "it is not a regular file",
            "it cannot be read (not a directory)",
            "it is over 65536 bytes",
            "it is not UTF-8",
            "line 3 repeats a key of its row",
            "the row on line 2 names no id",
            "line 2 gives `disabled` a value other than true or false",
        ]
    );
}

#[test]
fn only_a_recognised_last_credentials_row_writes_the_service() {
    let (enables, disables) = (Ok(Some(Write::Enables)), Ok(Some(Write::Disables)));
    for (patch, written) in [
        ("- id: credentials\n  disabled: false\n", enables.clone()),
        ("- disabled: false  \n  id: credentials\n", enables.clone()),
        (
            "- id: credentials\n  disabled: false\n- id: other\n  disabled: true\n",
            enables,
        ),
        (
            "# a comment\n- id: credentials\n  disabled: true\n\n- id: credentials\n",
            disables.clone(),
        ),
        (
            "- id: credentials\n  disabled: false\n- id: credentials\n  disabled: true\n",
            disables,
        ),
        ("- id: other\n  disabled: false\n", Ok(None)),
        ("  # only a comment\n", Ok(None)),
        ("", Ok(None)),
        (
            "- id: credentials\n  disabled: no\n",
            Err(Unread::Disabled(2)),
        ),
        (
            "- id: credentials\n  disabled: \"false\"\n",
            Err(Unread::Line(2)),
        ),
        ("- id: 'credentials'\n", Err(Unread::Line(1))),
        (
            "- id: credentials\n  disabled: false # on\n",
            Err(Unread::Line(2)),
        ),
        (
            "- id: other\n  config:\n    disabled: false\n",
            Err(Unread::Line(2)),
        ),
        (
            "- id: other\n  list:\n  - id: credentials\n",
            Err(Unread::Line(2)),
        ),
        (
            "  disabled: false\n- id: credentials\n",
            Err(Unread::Line(1)),
        ),
        (
            "- id: credentials\n   disabled: false\n",
            Err(Unread::Line(2)),
        ),
        ("- id: credentials\r\n", Err(Unread::Line(1))),
        ("\t- id: credentials\n", Err(Unread::Line(1))),
        ("---\n- id: credentials\n", Err(Unread::Line(1))),
        ("-\n", Err(Unread::Line(1))),
        ("- 9id: credentials\n", Err(Unread::Line(1))),
        ("- id: credentials\n  na me: x\n", Err(Unread::Line(2))),
        ("- id: credentials\n  note: a: b\n", Err(Unread::Line(2))),
        (
            "- id: credentials\n  disabled: false\n  disabled: true\n",
            Err(Unread::Repeated(3)),
        ),
        (
            "- id: credentials\n- disabled: false\n",
            Err(Unread::NoId(2)),
        ),
    ] {
        assert_eq!(writes(patch), written, "{patch:?}");
    }
}

#[test]
fn with_no_dsh_home_only_the_repository_is_asked_and_the_cause_is_named() {
    let planted = Planted::scaffold();
    planted.write(&planted.home.join(".env"), "DEEPSEEK_API_KEY=x\n");
    let cause = CompositeError::Config("no dsh home: set DSH_HOME or HOME".to_string());
    let sources = Sources {
        set: unset,
        repo: &planted.repo,
        home: Err(cause),
    };
    let answers = in_folder(&planted.adapters, &sources);
    assert_eq!(
        lines(&answers),
        [format!(
            "dsh key deepseek-official: dsh reaches no DEEPSEEK_API_KEY: it is not set in the \
             launching environment, no .env file is present at {}/.env, and no dsh home was \
             resolved (the DSH layout is unreadable: no dsh home: set DSH_HOME or HOME), so no \
             patch re-enables dsh's credentials service; export DEEPSEEK_API_KEY where brokkr \
             runs, or write the row `- id: credentials` with `  disabled: false` in the headless \
             profile's cordis.patch.yml and store DEEPSEEK_API_KEY through dsh's credentials \
             service",
            planted.repo.display()
        )]
    );
}

#[test]
fn a_seat_served_from_the_operators_home_reads_its_seams_home_or_cause() {
    let (repo, home) = (Path::new("/repo"), PathBuf::from("/home/.dsh"));
    let seams = DshSeams {
        executable: "/bin/dsh".into(),
        home: home.clone(),
        node: None,
        head: Vec::new(),
    };
    let served = Sources::served(repo, Ok(seams));
    assert_eq!(
        (served.repo, served.home, (served.set)("CARGO_MANIFEST_DIR")),
        (repo, Ok(home), true)
    );
    let cause = CompositeError::Config("unresolved".into());
    assert_eq!(Sources::served(repo, Err(cause.clone())).home, Err(cause));
}

#[test]
fn each_route_reads_its_declared_variable_once_and_an_unnamed_one_is_undeclared() {
    let models = json!({
        "a": "deepseek-official/deepseek-flash",
        "b": "deepseek-flash",
        "c": "dashscope/glm-5.2",
        "d": "dashscope/qwen3.8-max",
        "e": "meta/meta/muse-spark-1.3",
    });
    let planted = Planted::new(models, json!({"dashscope": "DASHSCOPE_API_KEY"}));
    let answers = planted.reaches(|name| name != "SPARK_API_KEY");
    let route = |route: &str, variable: &str| Reach::Reached {
        route: route.into(),
        variable: variable.into(),
        source: Source::Environment,
    };
    assert_eq!(
        answers,
        [
            route("dashscope", "DASHSCOPE_API_KEY"),
            route("deepseek-official", "DEEPSEEK_API_KEY"),
            Reach::Undeclared {
                route: "meta".into()
            },
        ]
    );
    assert_eq!(
        lines(&answers[2..]),
        [
            "dsh key meta: the dsh adapter names no credential variable for this route, so \
          whether dsh reaches a key for it is unknown; declare one under the adapter's \
          `credentials`"
        ]
    );
}

/// Each route whose key was asked carries the served-home caveat after its
/// answer, an undeclared one none, and a `.env` in the repository, a FIFO
/// or one no one may read, warns last and is never opened.
#[test]
fn each_asked_route_carries_the_served_home_caveat_and_a_workdir_dotenv_warns() {
    let models = json!({
        "a": "deepseek-flash",
        "b": "dashscope/glm-5.2",
        "c": "meta/meta/muse-spark-1.3",
    });
    let planted = Planted::new(models, json!({"dashscope": "DASHSCOPE_API_KEY"}));
    let key = |route: &str, variable: &str| {
        let source = Source::Environment;
        let (route, variable) = (route.to_string(), variable.to_string());
        let served = Line::Served {
            route: route.clone(),
            variable: variable.clone(),
        };
        [
            Line::Key(Reach::Reached {
                route,
                variable,
                source,
            }),
            served,
        ]
    };
    let undeclared = Line::Key(Reach::Undeclared {
        route: "meta".into(),
    });
    let [dashscope, dashscope_served] = key("dashscope", "DASHSCOPE_API_KEY");
    let [deepseek, deepseek_served] = key("deepseek-official", "DEEPSEEK_API_KEY");
    let mut expected = vec![
        dashscope,
        dashscope_served,
        deepseek,
        deepseek_served,
        undeclared,
    ];
    let set = |_: &str| true;
    assert_eq!(planted.lines_within_bound(set), expected);
    let in_repo = planted.repo.join(".env");
    fifo(&in_repo);
    expected.push(Line::Workdir(in_repo.clone()));
    assert_eq!(planted.lines_within_bound(set), expected);
    std::fs::remove_file(&in_repo).unwrap();
    planted.write(&in_repo, &format!("DEEPSEEK_API_KEY={PLANTED}\n"));
    std::fs::set_permissions(&in_repo, std::fs::Permissions::from_mode(0o000)).unwrap();
    let lines = planted.lines_within_bound(set);
    assert_eq!(lines, expected);
    let printed: Vec<String> = [&lines[3], &lines[5]]
        .iter()
        .map(|line| format!("{}: {}", line.what(), line.detail()))
        .collect();
    assert!(!printed.concat().contains(PLANTED), "{printed:?}");
    assert_eq!(
        printed,
        [
            "dsh home deepseek-official: this check read the operator's dsh home: \
             DEEPSEEK_API_KEY's presence where brokkr runs, the profile and home patches, and \
             each .env by name, unread; Brokkr can't yet tell whether it serves this route from \
             its own engine-only home (decision 0065), which reads neither patch nor the home's \
             .env, so past the environment this check can't confirm the key reaches a seat \
             served there"
                .to_string(),
            format!(
                "dsh workdir: {} is present, unread: if Brokkr serves a dsh seat from its own \
                 engine-only home (decision 0065), it refuses that launch, since dsh would read \
                 the file for a key",
                in_repo.display()
            ),
        ]
    );
}

#[test]
fn a_folder_that_hires_no_dsh_or_whose_adapters_do_not_load_has_no_line() {
    let planted = Planted::scaffold();
    // A workdir `.env` is warned only beside a dsh seat.
    planted.write(&planted.repo.join(".env"), "");
    std::fs::remove_file(planted.adapters.join("dsh.json")).unwrap();
    assert_eq!(planted.reaches(unset), []);
    assert_eq!(of_folder(&planted.adapters, &planted.repo), []);
    assert_eq!(of_folder(&planted.repo.join("absent"), &planted.repo), []);
}

#[test]
fn a_variable_is_asked_whether_it_is_set_by_name_alone() {
    assert!(set_and_not_empty("CARGO_MANIFEST_DIR"));
    assert!(!set_and_not_empty("BROKKR_717_NEVER_SET"));
    // A name no environment can hold is never set.
    assert!(!set_and_not_empty("CARGO_MANIFEST_DIR\0"));
}

/// The default key is the shipped dsh adapter's own for dsh's default
/// route (decision 0071 ruling 5).
#[test]
fn the_default_key_is_the_shipped_adapters_deepseek_official_credential() {
    let adapters = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters");
    let adapters = Adapters::load(&adapters).unwrap();
    let dsh = adapters.adapter("dsh").unwrap();
    assert_eq!(
        dsh.credentials.get("deepseek-official").map(String::as_str),
        Some(DEFAULT_KEY)
    );
}
