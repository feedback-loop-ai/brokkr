//! Locality and inertness for the local transcript reader (#222, proposed
//! decision 0055): prose stays out of every journal-derived surface, the
//! retained evidence keeps its bytes, and no provider process is started.

use std::path::{Path, PathBuf};
use std::process::Command;

use brokkr_core::envelope::EventType;
use brokkr_store::Store;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const SENTINEL: &str = "SENTINEL-TRANSCRIPT-PROSE-222";

fn brokkr_bin() -> &'static str {
    env!("CARGO_BIN_EXE_brokkr")
}

struct World {
    dir: tempfile::TempDir,
    db: PathBuf,
    home: PathBuf,
}

impl World {
    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn projects(&self) -> PathBuf {
        self.home.join(".claude/projects")
    }
}

fn world() -> World {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let home = dir.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let mut store = Store::open(&db).unwrap();
    store
        .create_run("r222", "feat", "self", &json!({"files": {}}))
        .unwrap();
    for (event_type, payload) in [
        (
            EventType::RunStarted,
            json!({"feature": "feat", "manifest": {}}),
        ),
        (EventType::PhaseEntered, json!({"phase": "intake"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "eff1", "seat": "review", "phase": "intake"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "eff1", "attempt_id": "att1"}),
        ),
    ] {
        store
            .append_next("r222", event_type, payload, None, None)
            .unwrap();
    }
    let world = World { dir, db, home };
    let mut store = Store::open(&world.db).unwrap();
    store
        .append_next(
            "r222",
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "attempt_id": "att1",
                   "checkpoint": {"step": "session-finished",
                     "transcript": {"kind": "claude-session", "locator": "abcd-1234",
                       "home": world.projects().to_str().unwrap()}}}),
            None,
            None,
        )
        .unwrap();
    world
}

fn write_transcript(world: &World) -> PathBuf {
    let project = world.projects().join("project");
    std::fs::create_dir_all(&project).unwrap();
    let file = project.join("abcd-1234.jsonl");
    let body = format!(
        "{{\"type\":\"assistant\",\"message\":{{\"role\":\"assistant\",\
         \"content\":[{{\"type\":\"text\",\"text\":\"{SENTINEL}-output\"}},\
         {{\"type\":\"thinking\",\"text\":\"{SENTINEL}-reasoning\"}},\
         {{\"type\":\"tool_use\",\"name\":\"Read\",\"input\":{{\"file_path\":\"{SENTINEL}-arg\"}}}}]}},\
         \"timestamp\":\"T\"}}\n"
    );
    std::fs::write(&file, body).unwrap();
    file
}

fn write_config(world: &World) -> PathBuf {
    let claude = world.home.join(".claude");
    std::fs::create_dir_all(&claude).unwrap();
    let config = claude.join("config.json");
    std::fs::write(&config, "{\"provider\":\"claude\",\"retained\":true}\n").unwrap();
    config
}

/// Every regular file below `root`, relative path -> bytes, sorted. Used
/// to prove the retained root and provider configuration keep their bytes
/// and existence across a read or a growth watch.
fn snapshot_tree(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap())
            .collect();
        entries.sort_by_key(|entry| entry.path());
        for entry in entries {
            let path = entry.path();
            let meta = std::fs::symlink_metadata(&path).unwrap();
            if meta.is_dir() {
                walk(base, &path, out);
            } else {
                let bytes = std::fs::read(&path).unwrap_or_default();
                out.push((path.strip_prefix(base).unwrap().to_path_buf(), bytes));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out
}

fn run(world: &World, args: &[&str]) -> std::process::Output {
    Command::new(brokkr_bin())
        .args(args)
        .arg("--db")
        .arg(&world.db)
        .env("HOME", &world.home)
        .current_dir(world.path())
        .output()
        .unwrap()
}

fn digest(path: &Path) -> [u8; 32] {
    let bytes = std::fs::read(path).unwrap();
    Sha256::digest(bytes).into()
}

/// The journal gains nothing and the retained file keeps its bytes across
/// a successful read, a refusal and a re-read.
#[test]
fn reading_leaves_the_journal_and_the_retained_file_unchanged() {
    let world = world();
    let file = write_transcript(&world);
    let config = write_config(&world);
    let journal = world.db.clone();
    let before_journal = digest(&journal);
    let before_file = digest(&file);
    let before_config = digest(&config);
    let before_tree = snapshot_tree(&world.home);

    let output = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "eff1", "--json"],
    );
    assert!(output.status.success());
    assert_eq!(digest(&journal), before_journal, "no journal write");
    assert_eq!(digest(&file), before_file, "no retained-byte change");
    assert_eq!(digest(&config), before_config, "no provider-config change");
    assert_eq!(
        snapshot_tree(&world.home),
        before_tree,
        "the retained root keeps every byte and its existence"
    );

    // A refusal is equally inert.
    let refused = run(
        &world,
        &[
            "transcript",
            "--run",
            "r222",
            "--seat",
            "eff1",
            "--json",
            "--turn",
            "99",
        ],
    );
    assert!(!refused.status.success());
    let document: Value = serde_json::from_slice(&refused.stdout).unwrap();
    assert_eq!(document["unavailable"], "turn-not-retained");
    assert_eq!(digest(&journal), before_journal);
    assert_eq!(digest(&file), before_file);
    assert_eq!(digest(&config), before_config);
    assert_eq!(snapshot_tree(&world.home), before_tree);
}

/// The sentinel prose reaches only the explicit transcript read; every
/// journal-derived surface keeps its path/id-only boundary.
#[test]
fn sentinel_prose_never_enters_a_journal_derived_surface() {
    let world = world();
    write_transcript(&world);

    let transcript = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "eff1", "--json"],
    );
    assert!(String::from_utf8_lossy(&transcript.stdout).contains(SENTINEL));

    for args in [
        vec!["inspect", "--run", "r222", "--json"],
        vec!["seats", "--run", "r222", "--json"],
        vec!["runs", "--json"],
        vec!["replay", "--run", "r222"],
        vec!["watch", "--run", "r222", "--once"],
    ] {
        let output = run(&world, &args);
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !combined.contains(SENTINEL),
            "{args:?} leaked transcript prose: {combined}"
        );
    }

    // An export writes journal events only; the retained file is untouched.
    let out = world.path().join("export");
    std::fs::create_dir_all(&out).unwrap();
    let export = run(
        &world,
        &["export", "--run", "r222", "--out", out.to_str().unwrap()],
    );
    assert!(
        export.status.success(),
        "{}",
        String::from_utf8_lossy(&export.stderr)
    );
    for entry in std::fs::read_dir(&out).unwrap() {
        let text = std::fs::read_to_string(entry.unwrap().path()).unwrap_or_default();
        assert!(!text.contains(SENTINEL), "export leaked transcript prose");
    }
}

/// The reader spawns no provider: a PATH that would materialize a marker
/// if any provider binary ran proves the reader did not run one.
#[cfg(unix)]
#[test]
fn the_reader_starts_no_provider_process() {
    use std::os::unix::fs::PermissionsExt;
    let world = world();
    write_transcript(&world);
    let bin = world.path().join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let marker = world.path().join("provider-ran");
    for name in ["claude", "codex", "dsh", "lanetally", "claude-code"] {
        let script = bin.join(name);
        std::fs::write(&script, format!("#!/bin/sh\ntouch {}\n", marker.display())).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let output = Command::new(brokkr_bin())
        .args(["transcript", "--run", "r222", "--seat", "eff1", "--json"])
        .arg("--db")
        .arg(&world.db)
        .env("HOME", &world.home)
        .env("PATH", path)
        .current_dir(world.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(!marker.exists(), "a provider process was started");
}
