//! Decision 0115 through the binary and clap_complete's bash protocol: a
//! completion request reads the completed command's own journal, library
//! or map, one page of it, writes nothing anywhere, prints nothing on
//! stderr whatever fails, and offers no hidden verb and no 0.16 alias.
//! Every journal is a scratch one; no journal of this checkout is opened.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use brokkr_store::{Attribution, NewEntry, QueueCommand, Store};

/// A real exported run, with its manifest beside it, as `import` requires.
const RUN: &str = "tui-graph-the-selection-box-gets-80f98deb";

/// The separator the request is asked to join candidates with.
const IFS: &str = "\n";

fn brokkr(dir: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_brokkr"));
    command.current_dir(dir).env_remove("COMPLETE");
    command
}

/// The request bash's registered function makes for `words`, the cursor
/// on the last of them: the variables its registration sets (bound by
/// [`the_registration_is_clap_completes_bash_protocol`]), then `--` and
/// the words.
fn request(dir: &Path, words: &[&str]) -> Output {
    request_at(dir, words, words.len() - 1)
}

/// [`request`], the cursor on the word at `index`.
fn request_at(dir: &Path, words: &[&str], index: usize) -> Output {
    let mut command = brokkr(dir);
    command
        .env("COMPLETE", "bash")
        .env("_CLAP_IFS", IFS)
        .env("_CLAP_COMPLETE_INDEX", index.to_string())
        .env("_CLAP_COMPLETE_COMP_TYPE", "9")
        .env("_CLAP_COMPLETE_SPACE", "true")
        .arg("--")
        .args(words);
    command.output().expect("brokkr starts")
}

/// The candidates a request answers, after checking it printed nothing on
/// stderr and exited cleanly.
fn offered(dir: &Path, words: &[&str]) -> Vec<String> {
    let answer = request(dir, words);
    assert_eq!(String::from_utf8_lossy(&answer.stderr), "", "{words:?}");
    assert_eq!(answer.status.code(), Some(0), "{words:?}");
    let stdout = String::from_utf8(answer.stdout).unwrap();
    stdout
        .split(IFS)
        .filter(|word| !word.is_empty())
        .map(str::to_string)
        .collect()
}

/// [`offered`] without the flags clap offers beside a positional value.
fn values(dir: &Path, words: &[&str]) -> Vec<String> {
    let offered = offered(dir, words).into_iter();
    offered.filter(|word| !word.starts_with('-')).collect()
}

fn text(path: &Path) -> &str {
    path.to_str().unwrap()
}

/// A journal at `db` holding `runs`, each created at its stamp, planted by
/// SQL as a writer from before the summaries left them; its writer is
/// closed, so only the journal file stands.
fn journal(db: &Path, runs: &[(&str, &str)]) {
    drop(Store::open(db).unwrap());
    let conn = rusqlite::Connection::open(db).unwrap();
    for (run, at) in runs {
        conn.execute(
            "INSERT INTO runs (run_id, feature, bundle_name, manifest, created_at)
             VALUES (?1, 'feat', 'self', '{}', ?2)",
            [run, at],
        )
        .unwrap();
    }
}

/// A workspace whose discovered folder's journal holds three runs.
fn folder() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let runs = [
        ("run-old", "2026-01-01T00:00:00Z"),
        ("run-new", "2026-01-01T00:00:02Z"),
        ("run-mid", "2026-01-01T00:00:01Z"),
    ];
    journal(&dir.path().join(".brokkr/state/forge.db"), &runs);
    dir
}

/// Every file and directory under `root`, with each file's bytes.
fn tree(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    let mut found = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            match path.is_dir() {
                true => {
                    pending.push(path.clone());
                    found.insert(path, None);
                }
                false => {
                    found.insert(path.clone(), Some(std::fs::read(&path).unwrap()));
                }
            }
        }
    }
    found
}

#[test]
fn the_registration_is_clap_completes_bash_protocol() {
    let dir = tempfile::tempdir().unwrap();
    let registered = brokkr(dir.path()).env("COMPLETE", "bash").output().unwrap();
    assert_eq!(String::from_utf8_lossy(&registered.stderr), "");
    let script = String::from_utf8(registered.stdout).unwrap();
    for variable in [
        "_CLAP_IFS=",
        "_CLAP_COMPLETE_INDEX=",
        "_CLAP_COMPLETE_COMP_TYPE=",
        "_CLAP_COMPLETE_SPACE=",
        "COMPLETE=\"bash\"",
        "-- \"${words[@]}\"",
        // Bash keeps the order the request answers in: newest run first.
        "-o nosort",
    ] {
        assert!(script.contains(variable), "{variable} is not in {script}");
    }
}

/// The root offers the daily verbs and the families, and no hidden verb
/// and no 0.16 alias; an alias typed is no verb, and its stderr note is
/// never printed to a prompt.
#[test]
fn the_live_root_offers_no_hidden_verb_and_no_alias() {
    let dir = tempfile::tempdir().unwrap();
    let roots = offered(dir.path(), &["brokkr", ""]);
    for root in ["init", "doctor", "completions", "runs", "queue", "system"] {
        assert!(roots.contains(&root.to_string()), "{root} in {roots:?}");
    }
    for hidden in [
        "driver",
        "hands",
        "bridge",
        "broker",
        "fake-driver",
        "inspect",
        "ledger",
    ] {
        assert!(
            !roots.contains(&hidden.to_string()),
            "{hidden} in {roots:?}"
        );
    }
    let drives = offered(dir.path(), &["brokkr", "queue", "dispatch", ""]);
    assert!(!drives.contains(&"drive".to_string()), "{drives:?}");
    let aliased = offered(dir.path(), &["brokkr", "inspect", "--run", ""]);
    assert!(
        !aliased.iter().any(|word| word.starts_with("run-")),
        "{aliased:?}"
    );
}

/// Run ids come from the discovered folder's journal, newest first, and a
/// typed prefix narrows them.
#[test]
fn run_ids_complete_newest_first_from_the_discovered_folders_journal() {
    let dir = folder();
    assert_eq!(
        offered(dir.path(), &["brokkr", "runs", "inspect", "--run", ""]),
        ["run-new", "run-mid", "run-old"]
    );
    assert_eq!(
        offered(dir.path(), &["brokkr", "stats", "ledger", "--run", "run-m"]),
        ["run-mid"]
    );
    assert_eq!(
        values(dir.path(), &["brokkr", "runs", "compare", "run-new", ""]),
        ["run-new", "run-mid", "run-old"]
    );
}

/// `--db` on the line, before or after the word being completed and in
/// either spelling, outranks the folder's journal, and so does the
/// journal `--realms` names.
#[test]
fn the_journal_the_line_names_outranks_the_folders() {
    let dir = folder();
    let other = dir.path().join("elsewhere/other.db");
    journal(&other, &[("other-run", "2026-01-01T00:00:00Z")]);
    let other = text(&other);
    for words in [
        vec!["brokkr", "runs", "inspect", "--db", other, "--run", ""],
        vec![
            "brokkr",
            "runs",
            "inspect",
            &format!("--db={other}"),
            "--run",
            "",
        ],
    ] {
        assert_eq!(offered(dir.path(), &words), ["other-run"], "{words:?}");
    }
    // The cursor on the empty word, with `--db` after it.
    let after = ["brokkr", "runs", "inspect", "--run", "", "--db", other];
    let after = request_at(dir.path(), &after, 4);
    assert_eq!(String::from_utf8_lossy(&after.stdout), "other-run");
    let map = dir.path().join("world/realms.json");
    std::fs::create_dir_all(map.parent().unwrap()).unwrap();
    let realms = serde_json::json!({
        "schema": "forge.realms/v1",
        "realms": [{"name": "brokkr", "path": ".", "default_branch": "main"}],
        "journal": "../elsewhere/other.db",
    });
    std::fs::write(&map, realms.to_string()).unwrap();
    let words = [
        "brokkr",
        "runs",
        "inspect",
        "--realms",
        text(&map),
        "--run",
        "",
    ];
    assert_eq!(offered(dir.path(), &words), ["other-run"]);
    let words = [
        "brokkr",
        "stats",
        "analytics",
        "--realms",
        text(&map),
        "--realm",
        "",
    ];
    assert_eq!(offered(dir.path(), &words), ["brokkr"]);
}

/// A page is fifty runs, the newest, however many the journal holds.
#[test]
fn run_ids_are_one_page_of_fifty() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let names: Vec<(String, String)> = (0..60)
        .map(|n| (format!("run-{n:02}"), format!("2026-01-01T00:00:{n:02}Z")))
        .collect();
    let runs: Vec<(&str, &str)> = names
        .iter()
        .map(|(r, a)| (r.as_str(), a.as_str()))
        .collect();
    journal(&db, &runs);
    let listed = offered(
        dir.path(),
        &["brokkr", "runs", "seats", "--db", text(&db), "--run", ""],
    );
    let newest: Vec<String> = (10..60).rev().map(|n| format!("run-{n:02}")).collect();
    assert_eq!(listed, newest);
}

/// Phases and seats are the named run's, as `runs inspect` reads them, and
/// none are offered before a `--run` is on the line.
#[test]
fn phases_and_seats_are_the_named_runs() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("journal.db");
    let from = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/journals")
        .join(format!("{RUN}.ndjson"));
    let imported = brokkr(dir.path())
        .args([
            "journal",
            "import",
            "--db",
            text(&db),
            "--from",
            text(&from),
        ])
        .output()
        .unwrap();
    assert_eq!(imported.status.code(), Some(0), "{imported:?}");
    let shown = brokkr(dir.path())
        .args(["runs", "inspect", "--db", text(&db), "--run", RUN, "--json"])
        .output()
        .unwrap();
    let view: serde_json::Value = serde_json::from_slice(&shown.stdout).unwrap();
    let names = |key: &str, field: &str| {
        let mut names: Vec<String> = (view[key].as_array().unwrap().iter())
            .map(|each| each[field].as_str().unwrap().to_string())
            .collect();
        names.sort();
        names.dedup();
        names
    };
    let (phases, seats) = (names("phases", "name"), names("participants", "label"));
    assert!(!phases.is_empty() && !seats.is_empty());
    let line = ["brokkr", "runs", "inspect", "--db", text(&db), "--run", RUN];
    assert_eq!(
        offered(dir.path(), &[&line[..], &["--phase", ""]].concat()),
        phases
    );
    assert_eq!(
        offered(dir.path(), &[&line[..], &["--seat", ""]].concat()),
        seats
    );
    let bare = [
        "brokkr",
        "runs",
        "inspect",
        "--db",
        text(&db),
        "--phase",
        "",
    ];
    assert_eq!(offered(dir.path(), &bare), Vec::<String>::new());
}

/// Recipe names come from the folder's shelf and queue entries from its
/// journal: only those still waiting, newest first.
#[test]
fn recipes_and_waiting_entries_come_from_the_folder() {
    let dir = folder();
    for recipe in ["alpha", "beta"] {
        std::fs::create_dir_all(dir.path().join(".brokkr/recipes").join(recipe)).unwrap();
    }
    assert_eq!(
        offered(dir.path(), &["brokkr", "run", "--recipe", ""]),
        ["alpha", "beta"]
    );
    let by = Attribution {
        operator: "op",
        reason: "test",
    };
    {
        let mut store = Store::open(&dir.path().join(".brokkr/state/forge.db")).unwrap();
        let mut add = |n: u32| {
            let payload = format!("launch {n}");
            let new = NewEntry {
                payload: &payload,
                priority: 0,
                waits: &[],
            };
            store.queue_add(new, by).unwrap()
        };
        let (first, second, third) = (add(1), add(2), add(3));
        store.queue_command(second, QueueCommand::Drop, by).unwrap();
        let _ = (first, third);
    }
    assert_eq!(
        values(dir.path(), &["brokkr", "queue", "hold", ""]),
        ["3", "1"]
    );
}

/// A missing journal, one a peer holds exclusively, a file that is no
/// journal and a folder of an old layout each complete nothing, print
/// nothing on stderr and never wait out the writer's patience.
#[test]
fn every_failure_completes_nothing_and_prints_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing.db");
    let foreign = dir.path().join("foreign.db");
    std::fs::write(&foreign, "not a database, only text long enough to be read").unwrap();
    let held = dir.path().join("held.db");
    journal(&held, &[("run-held", "2026-01-01T00:00:00Z")]);
    let peer = rusqlite::Connection::open(&held).unwrap();
    peer.execute_batch("PRAGMA locking_mode = EXCLUSIVE; BEGIN EXCLUSIVE;")
        .unwrap();
    peer.execute("INSERT INTO meta VALUES ('peer', 'holds')", [])
        .unwrap();
    for db in [&missing, &foreign, &held] {
        let started = Instant::now();
        let line = ["brokkr", "runs", "inspect", "--db", text(db), "--run", ""];
        assert_eq!(offered(dir.path(), &line), Vec::<String>::new(), "{db:?}");
        let line = ["brokkr", "queue", "drop", "--db", text(db), ""];
        assert_eq!(values(dir.path(), &line), Vec::<String>::new(), "{db:?}");
        assert!(started.elapsed() < Duration::from_secs(10), "{db:?} waited");
    }
    assert!(!missing.exists(), "a missing journal is never created");
    let bare = ["brokkr", "runs", "inspect", "--run", ""];
    assert_eq!(offered(dir.path(), &bare), Vec::<String>::new());
}

/// Requests of every kind, failing ones among them, leave the workspace
/// exactly as they found it: no journal, folder, side file or lock.
#[test]
fn a_completion_leaves_every_file_as_it_found_it() {
    let dir = folder();
    std::fs::create_dir_all(dir.path().join(".brokkr/recipes/alpha")).unwrap();
    let other = dir.path().join("other.db");
    journal(&other, &[("other-run", "2026-01-01T00:00:00Z")]);
    let before = tree(dir.path());
    assert!(before.keys().all(|path| !text(path).ends_with("-wal")));
    let other = text(&other);
    let lines: [&[&str]; 7] = [
        &["brokkr", "runs", "inspect", "--run", ""],
        &["brokkr", "runs", "inspect", "--db", other, "--run", ""],
        &[
            "brokkr", "runs", "inspect", "--run", "run-new", "--phase", "",
        ],
        &["brokkr", "queue", "hold", ""],
        &["brokkr", "run", "--recipe", ""],
        &[
            "brokkr",
            "runs",
            "inspect",
            "--db",
            "nowhere/forge.db",
            "--run",
            "",
        ],
        &["brokkr", "system", "realms", "--realms", "nowhere.json", ""],
    ];
    for line in lines {
        let answer = request(dir.path(), line);
        assert_eq!(String::from_utf8_lossy(&answer.stderr), "", "{line:?}");
    }
    assert_eq!(tree(dir.path()), before);
}

/// `COMPLETE` set to `0` asks for nothing, and the command runs as it
/// runs without it.
#[test]
fn a_request_switched_off_runs_the_command_as_before() {
    let dir = tempfile::tempdir().unwrap();
    let plain = brokkr(dir.path())
        .args(["completions", "bash"])
        .output()
        .unwrap();
    let off = (brokkr(dir.path()).env("COMPLETE", "0"))
        .args(["completions", "bash"])
        .output()
        .unwrap();
    assert_eq!(plain.status.code(), Some(0));
    assert_eq!(
        (off.stdout, off.stderr, off.status.code()),
        (plain.stdout, plain.stderr, Some(0))
    );
}
