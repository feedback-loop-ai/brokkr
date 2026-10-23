//! Every answer the dsh key check gives (#717), each exactly, against
//! planted files under a home and a repository of the test's own; and a
//! planted value never reaches a line.

use std::path::{Path, PathBuf};

use serde_json::json;

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

    fn patch(&self) -> PathBuf {
        self.home.join("profiles/headless/cordis.patch.yml")
    }

    /// The check's answers, `set` the environment's.
    fn reaches(&self, set: fn(&str) -> bool) -> Vec<Reach> {
        let home = Ok(self.home.clone());
        in_folder(
            &self.adapters,
            &Sources {
                set,
                repo: &self.repo,
                home,
            },
        )
    }
}

fn unset(_: &str) -> bool {
    false
}

fn reached(source: Source) -> Vec<Reach> {
    vec![Reach::Reached {
        route: "deepseek-official".into(),
        variable: "DEEPSEEK_API_KEY".into(),
        source,
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
    let answers = planted.reaches(unset);
    let (repo, home) = (planted.repo.display(), planted.home.display());
    assert_eq!(
        lines(&answers),
        [format!(
            "dsh key deepseek-official: dsh reaches no DEEPSEEK_API_KEY: it is not set in the \
             launching environment, no .env file at {repo}/.env or {home}/.env names it, and \
             the headless profile leaves dsh's credentials service disabled; export \
             DEEPSEEK_API_KEY where brokkr runs, or add the row `- id: credentials` with \
             `  disabled: false` to {home}/profiles/headless/cordis.patch.yml and store \
             DEEPSEEK_API_KEY through dsh's credentials service"
        )]
    );
    // A patch that leaves the service disabled is no key either.
    planted.write(&planted.patch(), "- id: credentials\n  disabled: true\n");
    assert_eq!(planted.reaches(unset), answers);
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
fn a_key_a_dotenv_names_is_reached_through_that_file_and_not_echoed() {
    let planted = Planted::scaffold();
    let at_home = planted.home.join(".env");
    planted.write(&at_home, &format!("DEEPSEEK_API_KEY={PLANTED}\n"));
    let answers = planted.reaches(unset);
    assert_eq!(answers, reached(Source::File(at_home.clone())));
    assert_eq!(
        lines(&answers),
        [format!(
            "dsh key deepseek-official: dsh reaches DEEPSEEK_API_KEY through {}, which names it",
            at_home.display()
        )]
    );
    // The repository's own `.env` is asked first.
    let in_repo = planted.repo.join(".env");
    planted.write(
        &in_repo,
        &format!("export DEEPSEEK_API_KEY = '{PLANTED}'\n"),
    );
    assert_eq!(planted.reaches(unset), reached(Source::File(in_repo)));
}

#[test]
fn a_profile_that_re_enables_the_service_is_reached_through_it() {
    let planted = Planted::scaffold();
    let patch = planted.patch();
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
fn with_no_dsh_home_only_the_repository_is_asked_and_the_cause_is_named() {
    let planted = Planted::scaffold();
    planted.write(&planted.home.join(".env"), "DEEPSEEK_API_KEY=x\n");
    let home = Err("no dsh home: set DSH_HOME or HOME".to_string());
    let sources = Sources {
        set: unset,
        repo: &planted.repo,
        home,
    };
    let answers = in_folder(&planted.adapters, &sources);
    assert_eq!(
        lines(&answers),
        [format!(
            "dsh key deepseek-official: dsh reaches no DEEPSEEK_API_KEY: it is not set in the \
             launching environment, no .env file at {}/.env names it, and the headless \
             profile leaves dsh's credentials service disabled; export DEEPSEEK_API_KEY where \
             brokkr runs, or add the row `- id: credentials` with `  disabled: false` to the \
             headless profile's cordis.patch.yml (no dsh home was resolved: no dsh home: set \
             DSH_HOME or HOME) and store DEEPSEEK_API_KEY through dsh's credentials service",
            planted.repo.display()
        )]
    );
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

#[test]
fn a_folder_that_hires_no_dsh_or_whose_adapters_do_not_load_has_no_line() {
    let planted = Planted::scaffold();
    std::fs::remove_file(planted.adapters.join("dsh.json")).unwrap();
    assert_eq!(planted.reaches(unset), []);
    assert_eq!(of_folder(&planted.adapters, &planted.repo), []);
    assert_eq!(of_folder(&planted.repo.join("absent"), &planted.repo), []);
}

#[test]
fn a_variable_is_asked_whether_it_is_set_by_name_alone() {
    assert!(set_and_not_empty("CARGO_MANIFEST_DIR"));
    assert!(!set_and_not_empty("BROKKR_717_NEVER_SET"));
}

#[test]
fn a_dotenv_names_the_variable_only_on_its_own_line_with_a_value() {
    let planted = Planted::scaffold();
    let file = planted.repo.join(".env");
    let long = "x".repeat(64);
    for (text, named) in [
        ("DEEPSEEK_API_KEY=x", true),
        ("  export DEEPSEEK_API_KEY=\t\"x\"\n", true),
        (&format!("{long}\nDEEPSEEK_API_KEY=x\n"), true),
        (&format!("{long}=\nDEEPSEEK_API_KEY=x\n"), true),
        ("DEEPSEEK_API_KEY=\nOTHER=x\n", false),
        ("DEEPSEEK_API_KEY=\"\"\r\n", false),
        ("DEEPSEEK_API_KEY_2=x\n", false),
        ("# DEEPSEEK_API_KEY=x\n", false),
        ("OTHER=DEEPSEEK_API_KEY=x\n", false),
        (&format!("{long}DEEPSEEK_API_KEY=x\n"), false),
        ("", false),
    ] {
        planted.write(&file, text);
        assert_eq!(names(&file, "DEEPSEEK_API_KEY"), named, "{text:?}");
    }
    // A file that is not there, and one that cannot be read, name nothing.
    assert!(!names(&planted.repo.join("absent"), "DEEPSEEK_API_KEY"));
    assert!(!names(&planted.home, "DEEPSEEK_API_KEY"));
}

#[test]
fn only_a_last_top_level_credentials_row_writing_false_re_enables_the_service() {
    for (patch, enabled) in [
        ("- id: credentials\n  disabled: false\n", true),
        ("- id: 'credentials'\n  disabled: \"false\" # on\n", true),
        ("- disabled: false\n  id: credentials\n", true),
        (
            "- id: credentials\n  disabled: false\n- id: other\n  disabled: true\n",
            true,
        ),
        (
            "# a comment\n- id: credentials\n  disabled: true\n- id: credentials\n",
            false,
        ),
        (
            "- id: credentials\n  disabled: false\n- id: credentials\n  disabled: true\n",
            false,
        ),
        ("- id: credentials\n  config:\n    disabled: false\n", false),
        ("- id: credentials\n  disabled: no\n", false),
        ("- id: other\n  disabled: false\n", false),
        ("  disabled: false\n", false),
        ("", false),
    ] {
        assert_eq!(re_enables(patch), enabled, "{patch:?}");
    }
}
