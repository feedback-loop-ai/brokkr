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

/// One journal's database bytes and its `-wal` sidecar digest. `None` means
/// the file does not exist. A `-wal` is recorded by digest, not length: a
/// length check over the zero-byte `-wal` the first read leaves proves
/// nothing, while a digest proves a read neither changed nor checkpointed a
/// frame-bearing log (S12).
///
/// SQLite's plain read-only open of a clean WAL journal creates an empty
/// `-wal` (and a `-shm` shared-memory index) on first access. That is
/// SQLite's documented shared-memory behavior, not Brokkr writing
/// evidence: the living `transcript-reading` spec ("Transcript prose stays
/// local and inert") requires the journal to open read-only and append no
/// events or checkpoints and the event count/hash to be unchanged, which
/// means the database bytes never move and a newly appeared WAL carries no
/// frame. Only immutable mode avoids the sidecars, and that is unsafe for a
/// live journal a run may still be writing.
fn journal_and_wal(journal: &Path) -> (Option<[u8; 32]>, Option<[u8; 32]>) {
    let db = std::fs::read(journal)
        .ok()
        .map(|bytes| Sha256::digest(bytes).into());
    (db, wal_digest(journal))
}

/// The `-wal` sidecar digest, or `None` when it does not exist.
fn wal_digest(journal: &Path) -> Option<[u8; 32]> {
    std::fs::read(format!("{}-wal", journal.display()))
        .ok()
        .map(|bytes| Sha256::digest(bytes).into())
}

/// Whether a WAL file holds at least one complete frame. The 32-byte
/// header carries the page size at offset 8 (big-endian); a frame is a
/// 24-byte frame header plus one page. A zero-length or header-only WAL
/// carries no journal content, which is exactly what a read-only open
/// leaves behind.
fn wal_has_frames(journal: &Path) -> bool {
    let Ok(bytes) = std::fs::read(format!("{}-wal", journal.display())) else {
        return false;
    };
    if bytes.len() < 32 {
        return false;
    }
    let page_size = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    if !(512..=65_536).contains(&page_size) {
        return false;
    }
    bytes.len() >= 32 + 24 + page_size
}

/// Assert one read left a journal inert under the living spec: the database
/// digest equals the before-state; a `-wal` that did not exist before the
/// read and appeared during it is empty or frame-free (SQLite's own file,
/// never a journalled page); an existing `-wal` keeps the digest it had
/// immediately before the read. A new `-shm` is the shared-memory index, not
/// evidence, and is not compared. `before_db` is the original database
/// digest, taken once before the first read, so an earlier read cannot
/// launder a change into the per-read baseline.
fn assert_journal_inert(
    journal: &Path,
    before_db: [u8; 32],
    before_wal: Option<[u8; 32]>,
    context: &str,
) {
    let (after_db, after_wal) = journal_and_wal(journal);
    assert_eq!(
        after_db,
        Some(before_db),
        "{context}: the journal bytes or existence changed"
    );
    match before_wal {
        None => {
            if after_wal.is_some() {
                assert!(
                    !wal_has_frames(journal),
                    "{context}: the read journalled a WAL frame"
                );
            }
        }
        Some(before_wal) => assert_eq!(
            after_wal,
            Some(before_wal),
            "{context}: the read changed the existing WAL sidecar"
        ),
    }
}

/// Every string value anywhere inside a JSON document. Used to prove a
/// journal-derived readout carries no transcript prose in any nested
/// telemetry or result field, not only in its top-level text.
fn json_strings<'a>(value: &'a Value, out: &mut Vec<&'a str>) {
    match value {
        Value::String(text) => out.push(text),
        Value::Array(items) => {
            for item in items {
                json_strings(item, out);
            }
        }
        Value::Object(map) => {
            for item in map.values() {
                json_strings(item, out);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
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
    let before_journal_state = journal_and_wal(&journal);
    let before_file = digest(&file);
    let before_config = digest(&config);
    let before_tree = snapshot_tree(&world.home);

    let output = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "eff1", "--json"],
    );
    assert!(output.status.success());
    assert_eq!(digest(&journal), before_journal, "no journal write");
    assert_journal_inert(
        &journal,
        before_journal_state.0.unwrap(),
        before_journal_state.1,
        "a successful read",
    );
    assert_eq!(digest(&file), before_file, "no retained-byte change");
    assert_eq!(digest(&config), before_config, "no provider-config change");
    assert_eq!(
        snapshot_tree(&world.home),
        before_tree,
        "the retained root keeps every byte and its existence"
    );

    // A refusal is equally inert. The successful read left a `-wal` (empty,
    // no frame); that `-wal` is now existing, so the refusal must leave its
    // digest exactly as it found it.
    let refusal_wal = wal_digest(&journal);
    assert!(
        refusal_wal.is_some(),
        "the first read-only open leaves the shared-memory `-wal`"
    );
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
    assert_journal_inert(
        &journal,
        before_journal_state.0.unwrap(),
        refusal_wal,
        "a refusal",
    );
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

/// R25: a hostile agent-controlled path/home is losslessly represented only
/// by the portable display literal, and the read still leaves every retained
/// byte and the journal unchanged.
#[test]
fn hostile_paths_are_portable_and_retained_inert() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let home = dir.path().join(if cfg!(windows) {
        "home $(x) `t` ;a&b%c!d é😀"
    } else {
        "home $(x) `t` ;a&b|c<d>e%f!g \"q\" \\ é😀"
    });
    std::fs::create_dir_all(home.join("sessions")).unwrap();
    let file = home.join("sessions/rollout-0199mine.jsonl");
    std::fs::write(&file, "{\"type\":\"turn_context\"}\n").unwrap();

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
        (
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "attempt_id": "att1",
                   "checkpoint": {"step": "session-finished",
                     "transcript": {"kind": "codex-thread", "locator": "0199mine",
                       "home": home.to_str().unwrap()}}}),
        ),
    ] {
        store
            .append_next("r222", event_type, payload, None, None)
            .unwrap();
    }
    drop(store);

    let before_journal = digest(&db);
    let before_file = digest(&file);
    let before_tree = snapshot_tree(&home);
    let output = Command::new(brokkr_bin())
        .args(["transcript", "--run", "r222", "--seat", "eff1", "--json"])
        .arg("--db")
        .arg(&db)
        .env("HOME", &home)
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    let hint = document["full_session"].as_str().expect("the shared hint");
    let expected = format!(
        "full session: path {}, codex exec resume 0199mine, home {}",
        brokkr_view::transcript::portable_display_literal(&format!(
            "{}/sessions/rollout-0199mine.jsonl",
            home.canonicalize().unwrap().display()
        )),
        brokkr_view::transcript::portable_display_literal(home.to_str().unwrap()),
    );
    assert_eq!(hint, expected);
    for raw in ["$(", "`", ";", "&", "|", "<", ">", "%", "!", "é", "😀"] {
        assert!(!hint.contains(raw), "{raw:?} survived raw in {hint:?}");
    }
    assert!(hint.contains("\\u0020"), "{hint}");
    assert!(hint.contains("\\ud83d\\ude00"), "{hint}");

    assert_eq!(digest(&db), before_journal, "no journal write");
    assert_eq!(digest(&file), before_file, "no retained-byte change");
    assert_eq!(
        snapshot_tree(&home),
        before_tree,
        "the retained root keeps every byte and its existence"
    );
}

/// Every hearth the run resolver consults — the selected one and a
/// sibling the map names — is opened read-only: no journal and no WAL
/// sidecar gains a byte or an existence change.
#[test]
fn every_hearth_and_wal_sidecar_stay_inert() {
    let world = world();
    write_transcript(&world);
    let config = write_config(&world);

    // A second hearth the resolver must list but never select.
    let decoy = world.path().join("decoy.db");
    {
        let mut store = Store::open(&decoy).unwrap();
        store
            .create_run("decoy", "decoy-feature", "self", &json!({"files": {}}))
            .unwrap();
        store
            .append_next(
                "decoy",
                EventType::RunStarted,
                json!({"feature": "decoy-feature", "manifest": {}}),
                None,
                None,
            )
            .unwrap();
    }
    let map = world.path().join("realms.json");
    std::fs::write(
        &map,
        r#"{
  "schema": "forge.realms/v2",
  "realms": [
    {"name": "alpha", "path": ".", "default_branch": "main", "journal": "forge.db"},
    {"name": "beta", "path": ".", "default_branch": "main", "journal": "decoy.db"}
  ],
  "journal": "forge.db"
}"#,
    )
    .unwrap();

    let before_primary = journal_and_wal(&world.db);
    let before_decoy = journal_and_wal(&decoy);
    let before_config = digest(&config);
    let before_tree = snapshot_tree(&world.home);

    let output = Command::new(brokkr_bin())
        .args(["transcript", "--run", "r222", "--seat", "eff1", "--json"])
        .arg("--realms")
        .arg(&map)
        .env("HOME", &world.home)
        .current_dir(world.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["run_id"], "r222");
    // The decoy journal was listed, so the resolver really consulted both.
    assert!(before_decoy.0.is_some(), "the decoy journal exists");

    assert_journal_inert(
        &world.db,
        before_primary.0.unwrap(),
        before_primary.1,
        "the selected hearth",
    );
    assert_journal_inert(
        &decoy,
        before_decoy.0.unwrap(),
        before_decoy.1,
        "the sibling hearth",
    );
    assert_eq!(digest(&config), before_config, "provider config changed");
    assert_eq!(
        snapshot_tree(&world.home),
        before_tree,
        "retained tree changed"
    );
}

/// The fleet dossier and the run's result telemetry keep their
/// path/id-only boundary: a transcript read derives no dossier and every
/// nested string in the journal-derived readouts is prose-free.
#[test]
fn dossier_and_result_telemetry_keep_their_boundary() {
    let world = world();
    write_transcript(&world);
    let record = world.path().join(".forge/muninn.ndjson");

    let transcript = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "eff1", "--json"],
    );
    assert!(transcript.status.success());
    assert!(
        !record.exists(),
        "a transcript read must not derive or write a muninn dossier record"
    );

    // Parsed, not substring-scanned: every nested result/telemetry string
    // in the journal-derived readouts is prose-free.
    for args in [
        vec!["inspect", "--run", "r222", "--json"],
        vec!["seats", "--run", "r222", "--json"],
        vec!["runs", "--json"],
    ] {
        let output = run(&world, &args);
        let text = String::from_utf8_lossy(&output.stdout);
        let document: Value = serde_json::from_str(&text).unwrap_or_else(|error| {
            panic!(
                "{args:?} did not emit one JSON document: {error}; stderr {}",
                String::from_utf8_lossy(&output.stderr)
            )
        });
        let mut strings = Vec::new();
        json_strings(&document, &mut strings);
        for value in strings {
            assert!(
                !value.contains(SENTINEL),
                "{args:?} leaked prose into a nested string: {value:?}"
            );
        }
    }

    // The dossier derivation runs read-only with the sentinel transcript
    // present, stops at the missing seat library and writes no record.
    let agents = world.path().join("no-agents");
    let adapters = world.path().join("no-adapters");
    let derived = run(
        &world,
        &[
            "muninn",
            "run",
            "--record",
            record.to_str().unwrap(),
            "--agents-dir",
            agents.to_str().unwrap(),
            "--adapters-dir",
            adapters.to_str().unwrap(),
        ],
    );
    assert!(
        !derived.status.success(),
        "a missing seat library is a refusal"
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&derived.stdout),
        String::from_utf8_lossy(&derived.stderr)
    );
    assert!(
        !combined.contains(SENTINEL),
        "the dossier derivation leaked prose: {combined}"
    );
    assert!(!record.exists(), "a failed dossier run writes no record");

    // The record reader names no prose either.
    let listed = run(
        &world,
        &[
            "muninn",
            "list",
            "--record",
            record.to_str().unwrap(),
            "--json",
        ],
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&listed.stdout),
        String::from_utf8_lossy(&listed.stderr)
    );
    assert!(!combined.contains(SENTINEL), "{combined}");
}

/// A bounded growth read re-derives the grown source without rewriting
/// it, and leaves the retained tree, provider configuration and journal
/// bytes/existence unchanged. (SQLite's read-only open may create an empty
/// `-wal`; no journal page is written — see `journal_and_wal`.)
#[test]
fn growth_reads_keep_the_tree_config_and_journal_inert() {
    let world = world();
    let file = write_transcript(&world);
    let config = write_config(&world);
    let before_read = journal_and_wal(&world.db);
    let before_db = before_read
        .0
        .expect("the journal exists before the first growth read");
    let before_config = digest(&config);
    let before_paths: Vec<_> = snapshot_tree(&world.home)
        .into_iter()
        .map(|(path, _)| path)
        .collect();

    for step in 0..3 {
        // The writer (never the reader) grows the retained source.
        let mut body = std::fs::read_to_string(&file).unwrap();
        body.push_str(&format!(
            "{{\"type\":\"assistant\",\"message\":{{\"role\":\"assistant\",\"content\":\"{SENTINEL}-growth-{step}\"}}}}\n"
        ));
        std::fs::write(&file, &body).unwrap();
        let grown = std::fs::read(&file).unwrap();

        // The before-state for the `-wal` is taken immediately before this
        // read: the first read begins with none, and each later read begins
        // with the `-wal` the previous read left behind.
        let before_wal = wal_digest(&world.db);
        let output = run(
            &world,
            &["transcript", "--run", "r222", "--seat", "eff1", "--json"],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );

        assert_eq!(
            std::fs::read(&file).unwrap(),
            grown,
            "the growth read must not rewrite the retained source"
        );
        assert_eq!(digest(&world.db), before_db, "no journal write");
        assert_journal_inert(&world.db, before_db, before_wal, "a growth read");
        assert_eq!(digest(&config), before_config, "provider config changed");
        let after_paths: Vec<_> = snapshot_tree(&world.home)
            .into_iter()
            .map(|(path, _)| path)
            .collect();
        assert_eq!(after_paths, before_paths, "the retained tree changed");
    }
}

/// Terminal controls in prompt, tool argument and stamp survive only as
/// JSON data; the text face and every refusal are sanitized, and the
/// journal is untouched.
#[test]
fn control_sequences_are_sanitized_in_text_and_preserved_in_json() {
    let world = world();
    let project = world.projects().join("project");
    std::fs::create_dir_all(&project).unwrap();
    let file = project.join("abcd-1234.jsonl");
    std::fs::write(
        &file,
        format!(
            "{{\"type\":\"assistant\",\"message\":{{\"role\":\"assistant\",\"content\":[{{\"type\":\"text\",\"text\":\"\\u001b[2J{SENTINEL}-output\"}},{{\"type\":\"tool_use\",\"name\":\"Read\",\"input\":{{\"file_path\":\"\\u001b[33m{SENTINEL}-arg\"}}}}]}},\"timestamp\":\"\\u001b[0mT\"}}\n"
        ),
    )
    .unwrap();
    let before_journal = journal_and_wal(&world.db);

    let text = run(&world, &["transcript", "--run", "r222", "--seat", "eff1"]);
    assert!(
        text.status.success(),
        "{}",
        String::from_utf8_lossy(&text.stderr)
    );
    let rendered = String::from_utf8_lossy(&text.stdout);
    assert!(
        !rendered.contains('\u{1b}'),
        "the text face leaked a terminal control: {rendered:?}"
    );
    assert!(
        rendered.contains(SENTINEL),
        "the explicit read still carries its requested prose"
    );

    let json = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "eff1", "--json"],
    );
    assert!(json.status.success());
    let document: Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(
        document["turns"][0]["blocks"][0]["text"],
        format!("\u{1b}[2J{SENTINEL}-output")
    );
    assert!(
        document["turns"][0]["blocks"][1]["text"]
            .as_str()
            .unwrap()
            .contains("\u{1b}[33m"),
        "{}",
        document["turns"][0]["blocks"][1]["text"]
    );
    // The raw document still carries the control as escaped JSON data.
    let raw = String::from_utf8_lossy(&json.stdout);
    assert!(raw.contains("\\u001b[2J"), "{raw}");

    // A refusal stays sanitized on stderr too.
    let refused = run(
        &world,
        &[
            "transcript",
            "--run",
            "r222",
            "--seat",
            "eff1",
            "--turn",
            "99",
        ],
    );
    assert!(!refused.status.success());
    assert!(refused.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&refused.stderr).contains('\u{1b}'));

    assert_journal_inert(
        &world.db,
        before_journal.0.unwrap(),
        before_journal.1,
        "the control read",
    );
}

/// A frame-bearing `-wal`: the writer that appended the transcript reference
/// stays open and idle across the three command reads, so the reference
/// exists only in committed WAL frames. Each read resolves it, proving it
/// read the frames, and leaves the `-wal` digest and the database bytes
/// unchanged (S12-S14).
#[test]
fn a_read_leaves_an_existing_frame_bearing_wal_as_it_found() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let home = dir.path().join("home");
    let projects = home.join(".claude/projects");
    std::fs::create_dir_all(projects.join("project")).unwrap();
    let file = projects.join("project/abcd-1234.jsonl");
    std::fs::write(
        &file,
        format!(
            "{{\"type\":\"assistant\",\"message\":{{\"role\":\"assistant\",\
             \"content\":\"{SENTINEL}-wal\"}}}}\n"
        ),
    )
    .unwrap();

    // The writer stays open and idle across all three reads. Its committed
    // frames, not a checkpointed database, carry the reference.
    let mut writer = Store::open(&db).unwrap();
    writer
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
        writer
            .append_next("r222", event_type, payload, None, None)
            .unwrap();
    }
    writer
        .append_next(
            "r222",
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "attempt_id": "att1",
                   "checkpoint": {"step": "session-finished",
                     "transcript": {"kind": "claude-session", "locator": "abcd-1234",
                       "home": projects.to_str().unwrap()}}}),
            None,
            None,
        )
        .unwrap();

    // Check the frames, do not assume them.
    assert!(
        wal_has_frames(&db),
        "the writer's committed reference must still be in the -wal"
    );
    let before_db = digest(&db);
    let before_wal = wal_digest(&db).expect("the frame-bearing -wal exists");

    let read = |args: &[&str]| {
        Command::new(brokkr_bin())
            .args(args)
            .arg("--db")
            .arg(&db)
            .env("HOME", &home)
            .current_dir(dir.path())
            .output()
            .unwrap()
    };

    // A successful read resolves the reference only by reading the frames.
    let success = read(&["transcript", "--run", "r222", "--seat", "eff1", "--json"]);
    assert!(
        success.status.success(),
        "{}",
        String::from_utf8_lossy(&success.stderr)
    );
    let document: Value = serde_json::from_slice(&success.stdout).unwrap();
    assert_eq!(document["run_id"], "r222");
    assert!(
        document["turns"]
            .as_array()
            .is_some_and(|turns| !turns.is_empty()),
        "the read resolved the reference from the frames: {document}"
    );
    assert!(String::from_utf8_lossy(&success.stdout).contains(SENTINEL));
    assert_eq!(digest(&db), before_db, "no database write");
    assert_eq!(
        wal_digest(&db),
        Some(before_wal),
        "the successful read changed or checkpointed the -wal"
    );

    // A refusal over the same frame-bearing journal is equally inert.
    let refusal = read(&[
        "transcript",
        "--run",
        "r222",
        "--seat",
        "eff1",
        "--json",
        "--turn",
        "99",
    ]);
    assert!(!refusal.status.success());
    let document: Value = serde_json::from_slice(&refusal.stdout).unwrap();
    assert_eq!(document["unavailable"], "turn-not-retained");
    assert_eq!(digest(&db), before_db);
    assert_eq!(wal_digest(&db), Some(before_wal));

    // A growth read after the retained file grew still reads the same frames
    // and leaves the -wal alone.
    let mut body = std::fs::read_to_string(&file).unwrap();
    body.push_str(&format!(
        "{{\"type\":\"assistant\",\"message\":{{\"role\":\"assistant\",\"content\":\"{SENTINEL}-wal-growth\"}}}}\n"
    ));
    std::fs::write(&file, &body).unwrap();
    let growth = read(&["transcript", "--run", "r222", "--seat", "eff1", "--json"]);
    assert!(
        growth.status.success(),
        "{}",
        String::from_utf8_lossy(&growth.stderr)
    );
    assert_eq!(digest(&db), before_db, "no database write");
    assert_eq!(
        wal_digest(&db),
        Some(before_wal),
        "the growth read changed or checkpointed the -wal"
    );

    drop(writer);
}
