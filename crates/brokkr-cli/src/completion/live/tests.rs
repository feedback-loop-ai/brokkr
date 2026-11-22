use std::ffi::OsString;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::PathBuf;

use clap_complete::engine::ArgValueCompleter;

use super::{candidates, completing, Line, Values};

fn line(words: &[&str]) -> Line {
    Line::read(words.iter().map(OsString::from))
}

/// The flags are read from the words after clap_complete's `--`, in
/// either spelling, the last one written winning; a flag with no value
/// yet names nothing, and a word before the `--` is the shell's own.
#[test]
fn the_line_reads_its_flags_after_the_protocols_separator() {
    assert_eq!(
        line(&[
            "brokkr",
            "--agents-dir",
            "ignored",
            "--",
            "brokkr",
            "runs",
            "inspect",
            "--db",
            "first.db",
            "--realms=map.json",
            "--db=second.db",
            "--run",
            "run-a",
            "--phase",
            "",
        ]),
        Line {
            db: Some(PathBuf::from("second.db")),
            realms: Some(PathBuf::from("map.json")),
            run: Some("run-a".to_string()),
            ..Line::default()
        }
    );
    assert_eq!(
        line(&[
            "brokkr",
            "--",
            "brokkr",
            "queue",
            "add",
            "--recipes-dir",
            "shelf",
            "--recipe",
            ""
        ]),
        Line {
            recipes_dir: Some(PathBuf::from("shelf")),
            ..Line::default()
        }
    );
    assert_eq!(
        line(&["brokkr", "--", "brokkr", "runs", "inspect", "--db", ""]),
        Line::default()
    );
    let words = [
        "brokkr",
        "--",
        "brokkr",
        "--agents-dir=team",
        "--dir",
        "lib",
    ];
    assert_eq!(
        line(&words),
        Line {
            agents_dir: Some(PathBuf::from("team")),
            dir: Some(PathBuf::from("lib")),
            ..Line::default()
        }
    );
}

/// A word that is not text names nothing, and a flag the line ends on has
/// no value yet; a current word that is not text completes nothing.
#[test]
fn words_that_are_not_text_name_nothing() {
    let words = [
        OsString::from("brokkr"),
        OsString::from("--"),
        OsString::from("brokkr"),
        OsString::from_vec(b"--db=\xff".to_vec()),
        OsString::from("--run"),
        OsString::from_vec(b"run-\xff".to_vec()),
        OsString::from("--realms"),
    ];
    assert_eq!(Line::read(words), Line::default());
    let current = std::ffi::OsStr::from_bytes(b"run-\xff");
    for values in [Values::Runs, Values::Phases, Values::Recipes] {
        assert!(candidates(values, current).is_empty(), "{values:?}");
    }
}

/// Each visible `verb-path argument` the completer gives values to, and
/// each one that names a run by its id.
fn completed(command: &clap::Command, path: &str, found: &mut Vec<String>, runs: &mut Vec<String>) {
    for arg in command.get_arguments() {
        let (id, named) = (arg.get_id().as_str(), format!("{path} {}", arg.get_id()));
        if id == "run" || id.starts_with("run_") || id.ends_with("_run") {
            runs.push(named.clone());
        }
        if arg.get::<ArgValueCompleter>().is_some() {
            found.push(named);
        }
    }
    for sub in command.get_subcommands().filter(|sub| !sub.is_hide_set()) {
        completed(sub, &format!("{path} {}", sub.get_name()), found, runs);
    }
}

/// Every run argument of every visible verb takes run ids, and each other
/// value its own verbs; the names a secret or a source takes take none.
#[test]
fn every_run_argument_and_each_named_value_is_completed() {
    let (mut found, mut runs) = (Vec::new(), Vec::new());
    completed(&completing(), "brokkr", &mut found, &mut runs);
    assert!(runs.len() > 20, "{runs:?}");
    for run in &runs {
        assert!(found.contains(run), "{run} takes no run ids");
    }
    for named in [
        "brokkr runs inspect phase",
        "brokkr runs inspect seat",
        "brokkr runs transcript seat",
        "brokkr runs compare run_a",
        "brokkr runs compare run_b",
        "brokkr queue add recipe",
        "brokkr queue hold entry",
        "brokkr queue drop entry",
        "brokkr library recipes show name",
        "brokkr library agents show name",
        "brokkr stats analytics realm",
        "brokkr runs conclude by_realm",
    ] {
        assert!(
            found.contains(&named.to_string()),
            "{named} is not completed"
        );
    }
    for unnamed in [
        "brokkr system secrets set name",
        "brokkr library recipes add name",
        "brokkr runs inspect capability_call",
    ] {
        assert!(
            !found.contains(&unnamed.to_string()),
            "{unnamed} is completed"
        );
    }
}
