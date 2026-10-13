//! The operator console's two readouts (#568), through the shipped binary:
//! `inspect --json` carries the commands, the disabled actions, the clarify
//! questions and the worktree, and `analytics --json` reads every hearth
//! the map names, a hearth that will not load surviving as quarantined.

use std::path::Path;
use std::process::Command;

use brokkr_core::EventType;
use serde_json::{json, Value};

/// A workspace whose map names three hearths: `alpha` on the world's
/// journal, `beta` on its own, and `ghost` on one nobody has written.
struct Workspace {
    dir: tempfile::TempDir,
}

impl Workspace {
    fn new() -> Workspace {
        let ws = Workspace {
            dir: tempfile::tempdir().unwrap(),
        };
        std::fs::create_dir_all(ws.path().join("state")).unwrap();
        let map = json!({
            "schema": "forge.realms/v2",
            "realms": [
                {"name": "alpha", "path": ".", "default_branch": "main"},
                {"name": "beta", "path": ".", "default_branch": "main",
                 "journal": "state/beta.db"},
                {"name": "ghost", "path": ".", "default_branch": "main",
                 "journal": "state/ghost.db"},
            ],
            "journal": "state/world.db",
        });
        std::fs::write(ws.path().join("realms.json"), map.to_string()).unwrap();
        ws
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    /// `brokkr <args>` run from the workspace: its exit code, its stdout
    /// read as JSON (null when it is not), and its stderr.
    fn brokkr(&self, args: &[&str]) -> (Option<i32>, Value, String) {
        let out = Command::new(env!("CARGO_BIN_EXE_brokkr"))
            .args(args)
            .current_dir(self.path())
            .output()
            .unwrap();
        (
            out.status.code(),
            serde_json::from_slice(&out.stdout).unwrap_or(Value::Null),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }

    /// A run journaled straight into `journal`: started, in `phase`, and
    /// then `rest`.
    fn journal(&self, journal: &str, run_id: &str, phase: &str, rest: &[(EventType, Value)]) {
        let mut store = brokkr_store::Store::open(&self.path().join(journal)).unwrap();
        store
            .create_run(run_id, "feature", "test", &json!({"files": {}}))
            .unwrap();
        let started = [
            (
                EventType::RunStarted,
                json!({"feature": "feature", "manifest": {}}),
            ),
            (EventType::PhaseEntered, json!({"phase": phase})),
        ];
        for (kind, payload) in started.iter().chain(rest) {
            store
                .append_next(run_id, *kind, payload.clone(), None, None)
                .unwrap();
        }
    }

    /// Two runs in `alpha`, one shipped and one parked at review when its
    /// seat's attempts ran out, and one shipped run in `beta`.
    fn fleet(&self) {
        let shipped = [(EventType::RunCompleted, json!({}))];
        self.journal("state/world.db", "alpha-shipped", "done", &shipped);
        let parked = [
            (
                EventType::EffectRequested,
                json!({"effect_id": "e1", "seat": "review", "phase": "review"}),
            ),
            (
                EventType::RunParked,
                json!({"reason": "attempts exhausted"}),
            ),
        ];
        self.journal("state/world.db", "alpha-parked", "review", &parked);
        self.journal("state/beta.db", "beta-shipped", "done", &shipped);
    }
}

#[test]
fn inspect_json_names_the_commands_and_the_disabled_actions() {
    let ws = Workspace::new();
    ws.fleet();
    let (code, view, stderr) = ws.brokkr(&["inspect", "--run", "alpha-parked", "--json"]);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(view["view_version"], brokkr_view::VIEW_VERSION);
    assert_eq!(
        view["commands"],
        json!([
            {"command": "retry", "phase": "review", "admission": "admitted"},
            {"command": "stop", "admission": "admitted"},
            {"command": "resume", "admission": "refused", "refusal": "awaiting_operator",
             "text": "run 'alpha-parked' awaits the operator; resume drives it again only \
                      once a retry is accepted"},
            {"command": "conclude", "admission": "admitted"},
        ])
    );
    assert_eq!(
        view["unavailable"],
        json!([
            {"action": "talk_to_reviewer",
             "reason": "Reviews start fresh each time; your note can't reach the reviewer yet."},
            {"action": "hold",
             "reason": "A parked run already waits for you; it holds nothing else back."},
        ])
    );
    assert_eq!(view["clarify"], Value::Null);
    assert_eq!(view["worktree"], Value::Null, "an unmapped run names none");
}

#[test]
fn analytics_json_reads_every_hearth_and_quarantines_the_one_that_will_not_load() {
    let ws = Workspace::new();
    ws.fleet();
    let (code, view, stderr) = ws.brokkr(&["analytics", "--json", "--range", "24h"]);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(view["view_version"], brokkr_view::VIEW_VERSION);
    assert_eq!(view["range"], "24h");
    assert_eq!(view["headline"]["shipped"]["count"], 2);
    let rows: Vec<(&str, &Value)> = view["realms"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| (row["realm"].as_str().unwrap(), &row["shipped"]))
        .collect();
    assert_eq!(rows, [("alpha", &json!(1)), ("beta", &json!(1))]);
    let quarantined = &view["quarantined"];
    assert_eq!(quarantined.as_array().unwrap().len(), 1, "{quarantined}");
    assert_eq!(quarantined[0]["realm"], "ghost");
    assert!(
        !ws.path().join("state/ghost.db").exists(),
        "a read created a journal"
    );

    let (code, beta, stderr) = ws.brokkr(&["analytics", "--json", "--realm", "beta"]);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(beta["realm"], "beta");
    assert_eq!(beta["headline"]["shipped"]["count"], 1);
    assert_eq!(beta["realms"].as_array().unwrap().len(), 1);
}

#[test]
fn analytics_json_over_one_hearth_reads_that_journal_alone() {
    let ws = Workspace::new();
    ws.fleet();
    let (code, view, stderr) = ws.brokkr(&["analytics", "--json", "--db", "state/beta.db"]);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(view["range"], "7d", "the default range");
    assert_eq!(view["headline"]["shipped"]["count"], 1);
    assert_eq!(view["quarantined"], json!([]));
}

#[test]
fn a_range_outside_the_three_is_refused_in_the_views_words() {
    let ws = Workspace::new();
    let (code, _, stderr) = ws.brokkr(&["analytics", "--range", "2d"]);
    assert_eq!(code, Some(2), "a usage error: {stderr}");
    assert!(
        stderr.contains("unknown range '2d': one of 24h, 7d or 30d"),
        "{stderr}"
    );
}
