//! Decision 0065 slice two: a checkpoint's capability-call evidence as
//! the read surfaces carry it. Rows are planted through the store's
//! append fence, so each is a record the seat-record contract of its
//! engine admits, and read back through the binary's `inspect --json`,
//! whose checkpoints are the one derivation in `brokkr-view` (U4g); and
//! the retained response a call cites, opened through `inspect
//! --capability-call` from a folder's run-local directory (U8g).

use std::fs;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use brokkr_core::{canonical, EventType};
use brokkr_runtime::engine::artifacts::Unread;
use brokkr_store::Store;
use brokkr_view::capability_calls::{NO_RETAINED, UNRECORDED};
use serde_json::{json, Value};

use crate::boundary_readouts::brokkr;

const DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// A response as the broker stages and the engine publishes it.
const RESPONSE: &[u8] = br#"{"result":{"content":[{"text":"docs","type":"text"}]}}"#;

/// One `research` seat under `engine` whose checkpoints are `checkpoints`.
fn plant(db: &Path, run_id: &str, engine: &str, checkpoints: &[Value]) {
    let manifest = json!({"engine": engine});
    let mut events = vec![
        (
            EventType::RunStarted,
            json!({"feature": "call it", "manifest": manifest}),
        ),
        (EventType::PhaseEntered, json!({"phase": "research"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "e1", "seat": "research", "phase": "research"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "e1", "attempt_id": "a1", "driver": "d"}),
        ),
    ];
    events.extend(checkpoints.iter().map(|checkpoint| {
        let payload = json!({"effect_id": "e1", "attempt_id": "a1", "checkpoint": checkpoint});
        (EventType::EffectCheckpointed, payload)
    }));
    let mut store = Store::open(db).unwrap();
    store
        .create_run(run_id, "call it", "test", &manifest)
        .unwrap();
    for (kind, payload) in events {
        // The append fence judges each row by its engine's seat record.
        store
            .append_next(run_id, kind, payload, None, None)
            .unwrap();
    }
}

/// `inspect --json`'s checkpoint call evidence for `run_id`, in order.
fn evidence(dir: &Path, db: &Path, run_id: &str) -> Vec<Value> {
    let db = db.to_str().unwrap();
    let (out, err, ok) = brokkr(dir, &["inspect", "--run", run_id, "--db", db, "--json"]);
    assert!(ok, "{err}");
    let view: Value = serde_json::from_str(&out).unwrap();
    let rows = view["participants"][0]["checkpoints"].as_array().unwrap();
    rows.iter()
        .map(|row| row["capability_call"].clone())
        .collect()
}

fn docs(tool: &str, call: &str, state: &str) -> Value {
    json!({"step": "seat-turn", "tool": tool, "capability": "library-docs",
           "dialect": "cap-library-docs", "call_id": call, "call_state": state})
}

/// A 0.12 journal's native observation and four settled broker states
/// pass the v6 fence and read back exactly; the refusal is a refusal.
#[test]
fn inspect_carries_every_recorded_call_state_exactly() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let db = root.join("forge.db");
    let mut succeeded = docs("lookup", "a1:b:1", "succeeded");
    succeeded["response_sha256"] = json!(DIGEST);
    plant(
        &db,
        "attributed",
        "0.12.0",
        &[
            json!({"step": "seat-turn", "turn": 1, "tool": "WebSearch",
                   "capability": "web-search", "dialect": "claude-native-search",
                   "call_id": "a1:toolu_1", "call_state": "observed"}),
            succeeded,
            docs("lookup", "a1:b:2", "failed"),
            docs("admin", "a1:b:3", "refused"),
            docs("lookup", "a1:b:4", "interrupted"),
        ],
    );
    let rows = evidence(&root, &db, "attributed");
    let call = |capability: &str, dialect: &str, tool: &str, id: &str, state: &str| {
        json!({"capability": capability, "dialect": dialect, "tool": tool,
               "call_id": id, "state": state, "response_sha256": null})
    };
    let lib = |tool, id, state| call("library-docs", "cap-library-docs", tool, id, state);
    let mut kept = lib("lookup", "a1:b:1", "succeeded");
    kept["response_sha256"] = json!(DIGEST);
    let calls: Vec<_> = rows.iter().map(|row| row["call"].clone()).collect();
    assert_eq!(
        calls,
        [
            call(
                "web-search",
                "claude-native-search",
                "WebSearch",
                "a1:toolu_1",
                "observed"
            ),
            kept,
            lib("lookup", "a1:b:2", "failed"),
            lib("admin", "a1:b:3", "refused"),
            lib("lookup", "a1:b:4", "interrupted"),
        ]
    );
    assert_eq!(
        rows[3]["cell"],
        json!({"text": "library-docs via cap-library-docs · admin refused",
               "absent": false, "note": null})
    );
}

/// A 0.10 journal's `WebSearch` checkpoint, valid under v5, stays
/// unrecorded beside an ordinary tool: absence is never backfilled.
#[test]
fn inspect_leaves_a_historical_native_tool_unrecorded() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let db = root.join("forge.db");
    plant(
        &db,
        "historical",
        "0.10.0",
        &[
            json!({"step": "seat-turn", "turn": 1, "tool": "WebSearch"}),
            json!({"step": "seat-turn", "turn": 2, "tool": "Read", "target": "a.rs"}),
        ],
    );
    let absent = json!({"text": brokkr_view::ABSENT, "absent": true, "note": UNRECORDED});
    let unrecorded = json!({"call": null, "cell": absent, "response": absent});
    assert_eq!(
        evidence(&root, &db, "historical"),
        [unrecorded.clone(), unrecorded]
    );
}

/// U8f: through the append fence and `inspect --json`, a succeeded and a
/// failed call that kept a response cite its digest, and every settled
/// outcome that kept none says only that none is recorded — the refused
/// call included, which stays a refusal and never reads as a grant.
#[test]
fn inspect_tells_a_retained_response_from_one_never_recorded() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let db = root.join("forge.db");
    let mut kept = docs("lookup", "a1:b:1", "succeeded");
    kept["response_sha256"] = json!(DIGEST);
    let mut failed_kept = docs("lookup", "a1:b:2", "failed");
    failed_kept["response_sha256"] = json!(DIGEST);
    plant(
        &db,
        "retained",
        "0.12.0",
        &[
            kept,
            failed_kept,
            docs("lookup", "a1:b:3", "succeeded"),
            docs("lookup", "a1:b:4", "failed"),
            docs("admin", "a1:b:5", "refused"),
            docs("lookup", "a1:b:6", "interrupted"),
        ],
    );
    let rows = evidence(&root, &db, "retained");
    let responses: Vec<_> = rows.iter().map(|row| row["response"].clone()).collect();
    let retained = json!({"text": DIGEST, "absent": false, "note": null});
    let none = json!({"text": brokkr_view::ABSENT, "absent": true, "note": NO_RETAINED});
    assert_eq!(
        responses,
        [
            retained.clone(),
            retained,
            none.clone(),
            none.clone(),
            none.clone(),
            none
        ]
    );
    assert_eq!(rows[4]["call"]["state"], "refused");
    assert_eq!(
        rows[1]["cell"]["text"],
        "library-docs via cap-library-docs · lookup failed · response retained"
    );
}

/// A folder repository on a canonical temporary root: its journal, and the
/// digests below its run-local directory's artifact root, owner-only as
/// the engine makes them.
struct Workspace {
    _dir: tempfile::TempDir,
    root: PathBuf,
    db: PathBuf,
    digests: PathBuf,
}

impl Workspace {
    fn new() -> Workspace {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        // Its own repository root, so no repository above the temporary
        // directory makes its `.brokkr` a nested folder.
        fs::create_dir(root.join(".git")).unwrap();
        let digests = root.join(".brokkr/state/artifacts/sha256");
        let mut made = fs::DirBuilder::new();
        made.recursive(true).mode(0o700).create(&digests).unwrap();
        let db = root.join("forge.db");
        Workspace {
            _dir: dir,
            root,
            db,
            digests,
        }
    }

    /// `bytes` published as `name` in the digests, read-only as the engine
    /// leaves an artifact.
    fn publish(&self, name: &str, bytes: &[u8]) {
        let mut options = fs::OpenOptions::new();
        let opened = options.write(true).create_new(true).mode(0o400);
        let mut file = opened.open(self.digests.join(name)).unwrap();
        std::io::Write::write_all(&mut file, bytes).unwrap();
    }

    /// What `inspect --capability-call call` prints over `run`, as JSON
    /// where `json` asks.
    fn open(&self, run: &str, call: &str, json: bool) -> (String, String, bool) {
        self.open_with(
            run,
            call,
            &json.then_some("--json").into_iter().collect::<Vec<_>>(),
        )
    }

    /// What `inspect --capability-call call` prints over `run`, run in
    /// this workspace with `more` after it.
    fn open_with(&self, run: &str, call: &str, more: &[&str]) -> (String, String, bool) {
        let db = self.db.to_str().unwrap();
        let args = [
            "inspect",
            "--run",
            run,
            "--db",
            db,
            "--capability-call",
            call,
        ];
        brokkr(&self.root, &[&args[..], more].concat())
    }
}

/// A `lookup` call `call` that succeeded, citing `sha256` where given.
fn kept(call: &str, sha256: Option<&str>) -> Value {
    let mut kept = docs("lookup", call, "succeeded");
    if let Some(sha256) = sha256 {
        kept["response_sha256"] = json!(sha256);
    }
    kept
}

/// What the binary prints refusing with `text`.
fn refused(text: &str) -> (String, String, bool) {
    (String::new(), format!("error: {text}\n"), false)
}

/// U8g (CR5): the call the selected run records opens as exactly the bytes
/// its digest names, beside the provenance the view derived, as text and
/// as JSON, whose response is those bytes as they are spelled, never
/// parsed and written again; and nothing is written: the journal and the
/// digests stand as they stood. No server stands anywhere to be asked.
#[test]
fn inspect_opens_exactly_the_response_a_call_cites_and_writes_nothing() {
    // Spelled as no serializer writes it again: `1.50`, and an escaped
    // letter no escape needs.
    let spelled = concat!(r#"{"result":{"n":1.50,"text":"n"#, "\\", r#"u00e9e"}}"#);
    let spelled = spelled.as_bytes();
    let workspace = Workspace::new();
    let sha256 = canonical::sha256_bytes(spelled);
    workspace.publish(&sha256, spelled);
    let calls = [kept("a1:b:1", Some(&sha256))];
    plant(&workspace.db, "retained", "0.12.0", &calls);
    let journal = fs::read(&workspace.db).unwrap();
    let provenance = [
        "capability call a1:b:1",
        "library-docs via cap-library-docs · lookup succeeded · response retained",
        &format!("response sha256 {sha256}"),
        "",
        std::str::from_utf8(spelled).unwrap(),
    ];
    let opened = workspace.open("retained", "a1:b:1", false);
    assert_eq!(opened, (provenance.join("\n"), String::new(), true));
    let (out, err, ok) = workspace.open("retained", "a1:b:1", true);
    let call = json!({"capability": "library-docs", "dialect": "cap-library-docs",
                      "tool": "lookup", "call_id": "a1:b:1", "state": "succeeded",
                      "response_sha256": sha256});
    let (printed, rest) = out.split_once(",\"response\":").unwrap();
    let printed: Value = serde_json::from_str(&format!("{printed}}}")).unwrap();
    let response = format!("{}}}\n", std::str::from_utf8(spelled).unwrap());
    let expected = (json!({ "call": call }), response.as_str());
    assert_eq!(
        (printed, rest, err, ok),
        (expected.0, expected.1, String::new(), true)
    );
    assert_eq!(fs::read(&workspace.db).unwrap(), journal);
    let standing: Vec<_> = fs::read_dir(&workspace.digests).unwrap().collect();
    assert_eq!(standing.len(), 1);
    assert_eq!(fs::read(workspace.digests.join(&sha256)).unwrap(), spelled);
}

/// U8g (CR5): a call another run records, one no run records and one the
/// selected run records twice are refused, though the artifact each would
/// cite stands sound; each run opens a call it records once.
#[test]
fn inspect_opens_only_a_call_the_selected_run_records_once() {
    let workspace = Workspace::new();
    let sha256 = canonical::sha256_bytes(RESPONSE);
    workspace.publish(&sha256, RESPONSE);
    let twice = kept("a1:b:2", Some(&sha256));
    let calls = [kept("a1:b:1", Some(&sha256)), twice.clone(), twice];
    plant(&workspace.db, "selected", "0.12.0", &calls);
    plant(
        &workspace.db,
        "other",
        "0.12.0",
        &[kept("a1:b:9", Some(&sha256))],
    );
    let unrecorded = refused("capability call is not recorded in this run");
    assert_eq!(workspace.open("selected", "a1:b:9", false), unrecorded);
    assert_eq!(workspace.open("selected", "a1:b:7", false), unrecorded);
    assert_eq!(
        workspace.open("selected", "a1:b:2", false),
        refused("capability call is recorded more than once in this run")
    );
    for (run, call) in [("selected", "a1:b:1"), ("other", "a1:b:9")] {
        let (_, err, ok) = workspace.open(run, call, false);
        assert_eq!((run, err, ok), (run, String::new(), true));
    }
    // The selector names one call, never a call within a scope.
    let db = workspace.db.to_str().unwrap();
    for (scope, value) in [("--phase", "research"), ("--seat", "research")] {
        let args = ["inspect", "--run", "selected", "--db", db, scope, value];
        let args = [&args[..], &["--capability-call", "a1:b:1"]].concat();
        let (out, err, ok) = brokkr(&workspace.root, &args);
        let named = format!(
            "error: the argument '{scope} <{}>' cannot be used with '--capability-call <CALL_ID>'",
            scope[2..].to_uppercase()
        );
        let first = err.lines().next().unwrap_or_default();
        assert_eq!((out.as_str(), first, ok), ("", named.as_str(), false));
    }
}

/// U8g (CR5): a call citing no digest, a digest no artifact stands at, an
/// artifact whose bytes are not its name, one that is a link and, as JSON,
/// one that is not JSON are each refused with its own cause, and nothing
/// is fetched in its place; the sound call beside them opens, and the one
/// that is not JSON opens as text.
#[test]
fn inspect_refuses_a_response_it_cannot_verify_with_its_cause() {
    let workspace = Workspace::new();
    let sha = |bytes: &[u8]| canonical::sha256_bytes(bytes);
    let [sound, absent, corrupt, linked, prose] =
        [RESPONSE, b"absent", b"corrupt", b"linked", b"prose"].map(sha);
    workspace.publish(&sound, RESPONSE);
    workspace.publish(&corrupt, RESPONSE);
    workspace.publish(&prose, b"prose");
    let link = workspace.digests.join(&linked);
    std::os::unix::fs::symlink(workspace.digests.join(&sound), link).unwrap();
    let cited = [Some(&sound), None, Some(&absent), Some(&corrupt)];
    let cited = cited.into_iter().chain([Some(&linked), Some(&prose)]);
    let calls: Vec<_> = (cited.enumerate())
        .map(|(at, sha256)| kept(&format!("a1:b:{}", at + 1), sha256.map(String::as_str)))
        .collect();
    plant(&workspace.db, "checked", "0.12.0", &calls);
    let cases = [
        ("a1:b:2", false, NO_RETAINED.to_string()),
        ("a1:b:3", false, Unread::Missing.to_string()),
        ("a1:b:4", false, Unread::Corrupt.to_string()),
        ("a1:b:5", false, Unread::Uncontained.to_string()),
        (
            "a1:b:6",
            true,
            "retained response artifact is not JSON".to_string(),
        ),
    ];
    for (call, json, text) in cases {
        let opened = workspace.open("checked", call, json);
        assert_eq!((call, opened), (call, refused(&text)));
    }
    let (_, err, ok) = workspace.open("checked", "a1:b:1", true);
    assert_eq!((err, ok), (String::new(), true));
    let (out, err, ok) = workspace.open("checked", "a1:b:6", false);
    assert_eq!(
        (out.ends_with("\n\nprose"), err, ok),
        (true, String::new(), true)
    );
}

/// U8g (CR5; operator ruling 2026-10-22): a run whose run-local directory
/// is another repository's opens there through `--repo`, which names the
/// repository, never the artifact: there too, an artifact whose bytes are
/// not its name is corrupt. Without `--repo`, the workspace's own run-local
/// directory is read, and it holds no such artifact. A repository with no
/// place for run-local state is refused in the folder's words, and `--repo`
/// without a call by clap.
#[test]
fn inspect_opens_a_call_in_the_repository_repo_names() {
    let (workspace, operated) = (Workspace::new(), Workspace::new());
    let [sound, corrupt] = [RESPONSE, b"corrupt"].map(canonical::sha256_bytes);
    operated.publish(&sound, RESPONSE);
    operated.publish(&corrupt, RESPONSE);
    let calls = [kept("a1:b:1", Some(&sound)), kept("a1:b:2", Some(&corrupt))];
    plant(&workspace.db, "elsewhere", "0.12.0", &calls);
    let repo = operated.root.to_str().unwrap();
    let provenance = [
        "capability call a1:b:1",
        "library-docs via cap-library-docs · lookup succeeded · response retained",
        &format!("response sha256 {sound}"),
        "",
        std::str::from_utf8(RESPONSE).unwrap(),
    ];
    let opened = workspace.open_with("elsewhere", "a1:b:1", &["--repo", repo]);
    assert_eq!(opened, (provenance.join("\n"), String::new(), true));
    let corrupted = workspace.open_with("elsewhere", "a1:b:2", &["--repo", repo]);
    assert_eq!(corrupted, refused(&Unread::Corrupt.to_string()));
    let unmoved = workspace.open("elsewhere", "a1:b:1", false);
    assert_eq!(unmoved, refused(&Unread::Missing.to_string()));
    let dir = tempfile::tempdir().unwrap();
    let bare = dir.path().canonicalize().unwrap();
    fs::create_dir(bare.join(".git")).unwrap();
    let place = bare.to_str().unwrap();
    let nowhere = format!(
        "no .brokkr/ folder from {place} up to {place}, and {place} is not a recipe library \
         (decision 0083); nothing was created: run `brokkr init .` from {place}"
    );
    let unplaced = workspace.open_with("elsewhere", "a1:b:1", &["--repo", place]);
    assert_eq!(unplaced, refused(&nowhere));
    let db = workspace.db.to_str().unwrap();
    let args = ["inspect", "--run", "elsewhere", "--db", db, "--repo", repo];
    let (out, err, ok) = brokkr(&workspace.root, &args);
    let first = err.lines().take(2).collect::<Vec<_>>();
    let required = [
        "error: the following required arguments were not provided:",
        "  --capability-call <CALL_ID>",
    ];
    assert_eq!((out.as_str(), first, ok), ("", required.to_vec(), false));
}
