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

use crate::boundary_readouts::{brokkr, brokkr_in};

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
    home: PathBuf,
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
        let home = root.join("home");
        made.recursive(true).mode(0o700).create(&home).unwrap();
        let db = root.join("forge.db");
        Workspace {
            _dir: dir,
            root,
            home,
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
        brokkr_in(&self.root, Some(&self.home), &[&args[..], more].concat())
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

/// U8g: a readout its standard output refuses fails with the write's own
/// cause on standard error, never as a success that printed nothing:
/// `/dev/full` refuses every write with ENOSPC, the provenance's first
/// line's among them. Linux's alone.
#[cfg(target_os = "linux")]
#[test]
fn inspect_fails_a_readout_its_output_refuses_with_the_cause() {
    let workspace = Workspace::new();
    let sha256 = canonical::sha256_bytes(RESPONSE);
    workspace.publish(&sha256, RESPONSE);
    plant(
        &workspace.db,
        "retained",
        "0.12.0",
        &[kept("a1:b:1", Some(&sha256))],
    );
    let device = fs::File::options().write(true).open("/dev/full").unwrap();
    let db = workspace.db.to_str().unwrap();
    let args = [
        "inspect",
        "--run",
        "retained",
        "--db",
        db,
        "--capability-call",
        "a1:b:1",
    ];
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_brokkr"))
        .args(args)
        .current_dir(&workspace.root)
        .env("HOME", &workspace.home)
        .env("XDG_STATE_HOME", workspace.home.join(".local/state"))
        .stdout(device)
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        (err.as_ref(), out.status.code()),
        ("error: No space left on device (os error 28)\n", Some(1))
    );
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

/// Decision 0105: a historical evidence root retired by `brokkr evidence`,
/// and the readouts after it. Each host gives itself its own HOME, so no
/// test reads or writes the host's coordination root.
struct EvidenceHost {
    _dir: tempfile::TempDir,
    home: PathBuf,
    work: PathBuf,
    artifacts: PathBuf,
    digests: PathBuf,
    db: PathBuf,
}

impl EvidenceHost {
    fn new() -> EvidenceHost {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let home = root.join("home");
        let work = root.join("work");
        fs::create_dir_all(work.join(".git")).unwrap();
        let artifacts = work.join(".brokkr/state/artifacts");
        let digests = artifacts.join("sha256");
        let mut made = fs::DirBuilder::new();
        made.recursive(true).mode(0o700).create(&digests).unwrap();
        let capabilities = home.join(".local/state/brokkr/capabilities");
        made.recursive(true)
            .mode(0o700)
            .create(&capabilities)
            .unwrap();
        let mut options = fs::OpenOptions::new();
        let opened = options.write(true).create_new(true).mode(0o600);
        std::io::Write::write_all(
            &mut opened.open(capabilities.join("admission.lock")).unwrap(),
            b"",
        )
        .unwrap();
        let db = work.join("forge.db");
        EvidenceHost {
            _dir: dir,
            home,
            work,
            artifacts,
            digests,
            db,
        }
    }

    /// `bytes` published as the digest it names, read-only as the engine
    /// leaves an artifact.
    fn publish(&self, bytes: &[u8]) -> String {
        let sha256 = canonical::sha256_bytes(bytes);
        let mut options = fs::OpenOptions::new();
        let opened = options.write(true).create_new(true).mode(0o400);
        let mut file = opened.open(self.digests.join(&sha256)).unwrap();
        std::io::Write::write_all(&mut file, bytes).unwrap();
        sha256
    }

    /// The historical record the engine would have sealed for `work`.
    fn record(&self) {
        self.record_for(&self.work);
    }

    /// The historical record the engine would have sealed for `work`, any
    /// worktree of this host.
    fn record_for(&self, work: &Path) {
        let artifacts = work.join(".brokkr/state/artifacts");
        let repo = canonical::sha256_bytes(work.as_os_str().as_encoded_bytes());
        let key = canonical::sha256_bytes(artifacts.as_os_str().as_encoded_bytes());
        let at = self
            .home
            .join(".local/state/brokkr/capabilities")
            .join(repo);
        let mut made = fs::DirBuilder::new();
        made.recursive(true).mode(0o700).create(&at).unwrap();
        let mut options = fs::OpenOptions::new();
        let opened = options.write(true).create_new(true).mode(0o600);
        let record = json!({"artifacts": artifacts}).to_string();
        std::io::Write::write_all(
            &mut opened
                .open(at.join(format!("historical-{key}.json")))
                .unwrap(),
            record.as_bytes(),
        )
        .unwrap();
    }

    /// The retirement's record in the coordination root.
    fn retirement(&self) -> PathBuf {
        let repo = canonical::sha256_bytes(self.work.as_os_str().as_encoded_bytes());
        let key = canonical::sha256_bytes(self.artifacts.as_os_str().as_encoded_bytes());
        self.home
            .join(".local/state/brokkr/capabilities")
            .join(repo)
            .join(format!("retired-{key}.json"))
    }

    /// The admission lock's path in the coordination root.
    fn lock(&self) -> PathBuf {
        self.home
            .join(".local/state/brokkr/capabilities/admission.lock")
    }

    /// A superseded retirement of this host's root, as a later admission
    /// would have sealed it.
    fn supersede(&self, run: &str) {
        let repo = canonical::sha256_bytes(self.work.as_os_str().as_encoded_bytes());
        let key = canonical::sha256_bytes(self.artifacts.as_os_str().as_encoded_bytes());
        let at = self
            .home
            .join(".local/state/brokkr/capabilities")
            .join(repo);
        let record = json!({
            "retirement": {
                "artifacts": self.artifacts,
                "cause": "discarded",
                "operator": "operator",
                "reason": "given up",
                "retired_at": "2026-11-08T00:00:00Z",
                "engine": "0.13.1",
            },
            "superseded_at": "2026-11-09T00:00:00Z",
            "run": run,
        });
        let mut options = fs::OpenOptions::new();
        let opened = options.write(true).create_new(true).mode(0o600);
        std::io::Write::write_all(
            &mut opened
                .open(at.join(format!("superseded-{key}-1.json")))
                .unwrap(),
            record.to_string().as_bytes(),
        )
        .unwrap();
    }

    /// The binary run in `work` with this host's own HOME.
    fn run(&self, args: &[&str]) -> (String, String, bool) {
        brokkr_in(&self.work, Some(&self.home), args)
    }

    /// The `--root` argument: the artifact root as `evidence list` prints it.
    fn root(&self) -> &str {
        self.artifacts.to_str().unwrap()
    }

    /// `brokkr evidence list`, as its JSON array.
    fn listed(&self) -> Vec<Value> {
        let (out, err, ok) = self.run(&["evidence", "list", "--json"]);
        assert!(ok, "{err}");
        serde_json::from_str(&out).unwrap()
    }

    /// Retire the host's root into the archive `to` with `reason`.
    fn retire_archived(&self, to: &Path, reason: &str) -> (String, String, bool) {
        self.run(&[
            "evidence",
            "retire",
            "--root",
            self.root(),
            "--archived-to",
            to.to_str().unwrap(),
            "--reason",
            reason,
        ])
    }

    /// `inspect --capability-call` over `run` in this host.
    fn inspect_call(&self, run: &str, call: &str) -> (String, String, bool) {
        self.run(&[
            "inspect",
            "--run",
            run,
            "--db",
            self.db.to_str().unwrap(),
            "--capability-call",
            call,
        ])
    }
}

/// U8g after decision 0105: list prints every root by path, existence and
/// retirement, `--json` agrees, retiring archives the evidence and writes
/// the inventory last, and a second retirement refuses.
#[test]
fn evidence_lists_archives_and_retires_a_root() {
    let host = EvidenceHost::new();
    let sha256 = host.publish(RESPONSE);
    host.record();
    let (out, err, ok) = host.run(&["evidence", "list"]);
    assert!(ok, "{err}");
    assert!(out.contains(host.root()), "{out}");
    assert!(
        out.contains("not retired") && out.contains("exists"),
        "{out}"
    );
    let listed = host.listed();
    assert_eq!(listed[0]["artifacts"], json!(host.artifacts));
    assert_eq!(listed[0]["exists"], json!(true));
    assert_eq!(listed[0]["digests"], json!(1));
    assert_eq!(listed[0]["bytes"], json!(RESPONSE.len()));

    let to = host.home.join("archive");
    let (out, err, ok) = host.run(&[
        "evidence",
        "retire",
        "--root",
        host.root(),
        "--archived-to",
        to.to_str().unwrap(),
        "--reason",
        "the evidence is archived",
        "--json",
    ]);
    assert!(ok, "{err}");
    let retired: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(retired["cause"], json!("archived"));
    assert_eq!(retired["archive"], json!(to));
    assert_eq!(retired["reason"], json!("the evidence is archived"));
    let inventory = fs::read(to.join("inventory.json")).unwrap();
    assert_eq!(
        retired["inventory_sha256"],
        json!(canonical::sha256_bytes(&inventory))
    );
    let inventory: Value = serde_json::from_slice(&inventory).unwrap();
    assert_eq!(inventory["format"], json!("brokkr.evidence-archive.v1"));
    assert_eq!(inventory["root"], json!(host.artifacts));
    assert_eq!(inventory["artifacts"][0]["sha256"], json!(sha256));
    assert_eq!(inventory["artifacts"][0]["bytes"], json!(RESPONSE.len()));
    assert_eq!(inventory["total_bytes"], json!(RESPONSE.len()));

    let (out, _, ok) = host.run(&["evidence", "list"]);
    assert!(ok);
    assert!(out.contains("archived at"), "{out}");
    let (_, err, ok) = host.run(&[
        "evidence",
        "retire",
        "--root",
        host.root(),
        "--discarded",
        "--reason",
        "again",
    ]);
    assert!(!ok);
    assert!(err.contains("is already retired"), "{err}");
}

/// U8g after decision 0105: a discarded root is removed and its response is
/// named "discarded", never corrupt, in text and JSON.
#[test]
fn evidence_discards_a_root_and_inspect_names_it_never_corrupt() {
    let host = EvidenceHost::new();
    let sha256 = host.publish(RESPONSE);
    host.record();
    plant(
        &host.db,
        "discarded",
        "0.12.0",
        &[kept("a1:b:1", Some(&sha256))],
    );
    let (out, err, ok) = host.run(&[
        "evidence",
        "retire",
        "--root",
        host.root(),
        "--discarded",
        "--reason",
        "the evidence is given up",
    ]);
    assert!(ok, "{err}");
    assert!(out.contains("discarded"), "{out}");
    assert!(!host.artifacts.exists(), "the root is removed");
    assert_eq!(host.listed()[0]["exists"], json!(false));
    let (out, err, ok) = host.run(&[
        "inspect",
        "--run",
        "discarded",
        "--db",
        host.db.to_str().unwrap(),
        "--capability-call",
        "a1:b:1",
    ]);
    assert!(ok, "{err}");
    assert!(out.contains("response discarded"), "{out}");
    assert!(!out.to_lowercase().contains("corrupt"), "{out}");
    let (out, err, ok) = host.run(&[
        "inspect",
        "--run",
        "discarded",
        "--db",
        host.db.to_str().unwrap(),
        "--capability-call",
        "a1:b:1",
        "--json",
    ]);
    assert!(ok, "{err}");
    let printed: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(printed["retirement"]["cause"], json!("discarded"));
    assert_eq!(printed["call"]["call_id"], json!("a1:b:1"));
}

/// U8g after decision 0105: an archived root's response is named with its
/// archive, and the evidence stands in both places.
#[test]
fn evidence_archives_a_root_and_inspect_names_where_it_went() {
    let host = EvidenceHost::new();
    let sha256 = host.publish(RESPONSE);
    host.record();
    plant(
        &host.db,
        "archived",
        "0.12.0",
        &[kept("a1:b:1", Some(&sha256))],
    );
    let to = host.home.join("archive");
    let (_, err, ok) = host.retire_archived(&to, "the evidence is archived");
    assert!(ok, "{err}");
    let (out, err, ok) = host.inspect_call("archived", "a1:b:1");
    assert!(ok, "{err}");
    assert!(
        out.contains(&format!("response archived at {}", to.display())),
        "{out}"
    );
    assert!(!out.to_lowercase().contains("corrupt"), "{out}");
    assert!(to.join("inventory.json").exists());
    assert!(to.join("sha256").join(&sha256).exists());
    assert!(host.artifacts.join("sha256").join(&sha256).exists());
}

/// Decision 0105 ruling 2: a destination that holds a file refuses before
/// anything is written, and a root with no historical record refuses by
/// name.
#[test]
fn evidence_refuses_a_full_destination_and_an_unrecorded_root() {
    let host = EvidenceHost::new();
    host.record();
    let to = host.home.join("full");
    fs::create_dir(&to).unwrap();
    fs::write(to.join("file"), b"x").unwrap();
    let (_, err, ok) = host.retire_archived(&to, "archived");
    assert!(!ok, "{err}");
    assert!(err.contains("already holds a file"), "{err}");
    let (_, err, ok) = host.run(&[
        "evidence",
        "retire",
        "--root",
        "/no/such/evidence/root",
        "--discarded",
        "--reason",
        "discarded",
    ]);
    assert!(!ok);
    assert!(
        err.contains("no historical evidence root is recorded"),
        "{err}"
    );
    assert_eq!(host.listed()[0]["retirement"], Value::Null);
}

/// M4: one bad root hides no other; every row is printed, the bad one
/// carries its problem, and the verb exits non-zero after printing.
#[test]
fn evidence_list_prints_every_root_and_exits_nonzero_after_a_bad_one() {
    let host = EvidenceHost::new();
    host.publish(RESPONSE);
    host.record();
    let second = host._dir.path().join("second");
    fs::create_dir_all(second.join(".brokkr/state/artifacts/sha256")).unwrap();
    let unfinished = second.join(".brokkr/state/artifacts/sha256/unfinished");
    fs::write(&unfinished, b"x").unwrap();
    host.record_for(&second);
    let third = host._dir.path().join("third");
    fs::create_dir_all(&third).unwrap();
    host.record_for(&third);

    let (out, err, ok) = host.run(&["evidence", "list"]);
    assert!(!ok, "a bad root makes the verb exit non-zero: {err}");
    for at in [
        host.root(),
        second.to_str().unwrap(),
        third.to_str().unwrap(),
    ] {
        assert!(out.contains(at), "every root is printed: {at}\n{out}");
    }
    assert!(out.contains("problem:"), "{out}");
    assert!(out.contains(&unfinished.display().to_string()), "{out}");

    let (out, _err, ok) = host.run(&["evidence", "list", "--json"]);
    assert!(!ok);
    let rows: Vec<Value> = serde_json::from_str(&out).unwrap();
    assert_eq!(rows.len(), 3, "every root is in the JSON array");
    let bad = rows
        .iter()
        .find(|row| row["artifacts"] == json!(second.join(".brokkr/state/artifacts")))
        .unwrap();
    assert!(bad["problem"]
        .as_str()
        .is_some_and(|p| p.contains("unfinished")));
}

/// M3: a relative `--archived-to` is made absolute before the checks and
/// the sealed record, as `--root` is.
#[test]
fn a_relative_archive_destination_is_recorded_absolute() {
    let host = EvidenceHost::new();
    host.publish(RESPONSE);
    host.record();
    let (out, err, ok) = host.run(&[
        "evidence",
        "retire",
        "--root",
        host.root(),
        "--archived-to",
        "relative-archive",
        "--reason",
        "archived",
        "--json",
    ]);
    assert!(ok, "{err}");
    let retired: Value = serde_json::from_str(&out).unwrap();
    let expected = host.work.join("relative-archive");
    assert_eq!(retired["archive"], json!(expected));
    assert!(expected.join("inventory.json").exists());
}

/// L1: a readout on a coordination root with no lock file creates none,
/// and reads nothing from it.
#[test]
fn a_readout_with_no_lock_creates_none() {
    let host = EvidenceHost::new();
    let sha256 = host.publish(RESPONSE);
    host.record();
    fs::remove_file(host.lock()).unwrap();
    let (out, err, ok) = host.run(&["evidence", "list", "--json"]);
    assert!(ok, "{err}");
    assert_eq!(out.trim(), "[]", "nothing has been written under no lock");
    assert!(!host.lock().exists(), "the readout created no lock");

    plant(
        &host.db,
        "unlocked",
        "0.12.0",
        &[kept("a1:b:1", Some(&sha256))],
    );
    let (out, err, ok) = host.run(&[
        "inspect",
        "--run",
        "unlocked",
        "--db",
        host.db.to_str().unwrap(),
        "--capability-call",
        "a1:b:1",
    ]);
    assert!(ok, "{err}");
    assert!(
        out.contains(String::from_utf8_lossy(RESPONSE).as_ref()),
        "{out}"
    );
    assert!(!host.lock().exists(), "inspect created no lock either");
}

/// H1: a response retained after the last retirement reads as live
/// evidence, chosen by the response's own recorded time against
/// `retired_at`.
#[test]
fn inspect_names_a_response_retained_after_a_retirement_as_live() {
    let host = EvidenceHost::new();
    let sha256 = host.publish(RESPONSE);
    host.record();
    let to = host.home.join("archive");
    let (_, err, ok) = host.retire_archived(&to, "archived");
    assert!(ok, "{err}");
    // The response is recorded after a retirement sealed in the past.
    let at = host.retirement();
    let mut sealed: Value = serde_json::from_slice(&fs::read(&at).unwrap()).unwrap();
    sealed["retired_at"] = json!("2020-01-01T00:00:00Z");
    fs::write(&at, sealed.to_string()).unwrap();
    plant(
        &host.db,
        "after",
        "0.12.0",
        &[kept("a1:b:1", Some(&sha256))],
    );
    let (out, err, ok) = host.inspect_call("after", "a1:b:1");
    assert!(ok, "{err}");
    assert!(!out.contains("response archived"), "{out}");
    assert!(
        out.contains(String::from_utf8_lossy(RESPONSE).as_ref()),
        "the live bytes are read: {out}"
    );
}

/// H1: `evidence list` prints a superseded retirement as the root's
/// history, in text and JSON.
#[test]
fn evidence_list_prints_a_superseded_retirement_as_history() {
    let host = EvidenceHost::new();
    host.record();
    host.supersede("run-a");
    let (out, err, ok) = host.run(&["evidence", "list"]);
    assert!(ok, "{err}");
    assert!(
        out.contains("superseded on 2026-11-09T00:00:00Z by run 'run-a'"),
        "{out}"
    );
    assert!(out.contains("discarded"), "{out}");
    let listed = host.listed();
    let history = listed[0]["history"].as_array().unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0]["run"], json!("run-a"));
    assert_eq!(
        history[0]["retirement"]["retired_at"],
        json!("2026-11-08T00:00:00Z")
    );
}
