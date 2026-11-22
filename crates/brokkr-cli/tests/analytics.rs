//! The operator console's two readouts (#568), through the shipped binary:
//! `inspect --json` carries the commands, the disabled actions, the clarify
//! questions and the worktree, and `analytics --json` reads every hearth
//! the map names, a hearth that will not load surviving as quarantined.

use std::path::Path;
use std::process::Command;

use brokkr_core::EventType;
use serde_json::{json, Value};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

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
    let (code, view, stderr) = ws.brokkr(&["runs", "inspect", "--run", "alpha-parked", "--json"]);
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
    let (code, view, stderr) = ws.brokkr(&["stats", "analytics", "--json", "--range", "24h"]);
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

    let (code, beta, stderr) = ws.brokkr(&["stats", "analytics", "--json", "--realm", "beta"]);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(beta["realm"], "beta");
    assert_eq!(beta["headline"]["shipped"]["count"], 1);
    assert_eq!(beta["realms"].as_array().unwrap().len(), 1);
}

#[test]
fn analytics_json_over_one_hearth_reads_that_journal_alone() {
    let ws = Workspace::new();
    ws.fleet();
    let (code, view, stderr) =
        ws.brokkr(&["stats", "analytics", "--json", "--db", "state/beta.db"]);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(view["range"], "7d", "the default range");
    assert_eq!(view["headline"]["shipped"]["count"], 1);
    assert_eq!(view["quarantined"], json!([]));
}

#[test]
fn a_range_outside_the_three_is_refused_in_the_views_words() {
    let ws = Workspace::new();
    let (code, _, stderr) = ws.brokkr(&["stats", "analytics", "--range", "2d"]);
    assert_eq!(code, Some(2), "a usage error: {stderr}");
    assert!(
        stderr.contains("unknown range '2d': one of 24h, 7d or 30d"),
        "{stderr}"
    );
}

#[test]
fn analytics_without_json_paints_the_headline_and_the_quarantined_hearth() {
    let ws = Workspace::new();
    ws.fleet();
    let out = Command::new(env!("CARGO_BIN_EXE_brokkr"))
        .args(["stats", "analytics", "--range", "24h"])
        .current_dir(ws.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("· all realms\n"), "{stdout}");
    assert!(stdout.contains("shipped        2 ("), "{stdout}");
    assert!(stdout.contains("quarantined    ghost: "), "{stdout}");
    assert!(stdout.contains("not recorded yet: "), "{stdout}");
}

/// One state of [`GRAPH_JSON`], entered once and reporting nothing: its
/// stays took `<millis>`, which reads `<took>`.
const STATE_JSON: &str = r#"    {
      "name": "<name>",
      "place": "row",
      "entered": 1,
      "median": {
        "millis": <millis>,
        "text": "<took>"
      },
      "p90": {
        "millis": <millis>,
        "text": "<took>"
      },
      "cost": {
        "cost": {
          "kind": "not_reported"
        },
        "text": "not reported",
        "reporting": [],
        "silent": [],
        "note": null
      },
      "tokens": {
        "total": {
          "text": "—",
          "absent": true,
          "note": "not reported"
        },
        "input": {
          "text": "—",
          "absent": true,
          "note": "not reported"
        },
        "output": {
          "text": "—",
          "absent": true,
          "note": "not reported"
        },
        "cache_hit": {
          "text": "—",
          "absent": true,
          "note": "no cache reads and input reported to compare"
        }
      },
      "seats": []
    }"#;

/// `brokkr stats graph --json` for one live run of `gone` that stayed `<stay>`
/// milliseconds at the state `<long>` before `LONG-OK` moved it on to
/// `<to>` at `<ruled>`, written out by hand in the view's field order,
/// each state's figures as [`STATE_JSON`] reads, the command's clock read
/// as `<time>` (#1167).
const GRAPH_JSON: &str = r#"{
  "view_version": 2,
  "realm": "alpha",
  "recipe": "gone",
  "range": "7d",
  "from": "<time>",
  "to": "<time>",
  "recipes": [
    "gone"
  ],
  "does_not_compile": [],
  "table": {
    "kind": "not_found",
    "text": "this recipe's table was not found in alpha"
  },
  "runs": 1,
  "states": [
<states>
  ],
  "rules": [
    {
      "id": "LONG-OK",
      "from": "<long>",
      "next": "<to>",
      "result": "resolved",
      "when": [],
      "severity": "normal",
      "count": 1,
      "kind": "forward",
      "text": "LONG-OK · 1",
      "latest": [
        {
          "run_id": "live",
          "label": "feature",
          "at": "<ruled>"
        }
      ]
    }
  ],
  "links": [
    {
      "from": "<long>",
      "next": "<to>",
      "kind": "forward",
      "count": 1,
      "rules": 1,
      "outcomes": [
        {
          "result": "resolved",
          "count": 1,
          "text": "resolved 1"
        }
      ],
      "text": "resolved 1"
    }
  ],
  "never_fired": {
    "count": 0,
    "of": 1,
    "text": "0 of 1"
  },
  "retired": [],
  "drifted_runs": 0,
  "drift_text": null,
  "ends": {
    "done": 0,
    "stop": [],
    "stopped": 0,
    "operator": 0,
    "open": 1,
    "text": "0 done · 0 stop by a rule · 0 stopped by you · 1 open",
    "refused_by": [],
    "ledger": {
      "judged": 0,
      "shipped": 0,
      "rate_percent": null,
      "excluded": {
        "blocked": 0,
        "superseded": 0,
        "infrastructure": 0,
        "delivered-elsewhere": 0,
        "withdrawn": 0,
        "operator-stopped": 0
      },
      "unclassified": 0,
      "text": "0 of 0 shipped (—) · 0 excluded · 0 unclassified"
    }
  },
  "live": [
    {
      "run_id": "live",
      "label": "feature",
      "state": "<to>"
    }
  ]
}
"#;

/// An RFC 3339 reading as a time.
fn read(text: &str) -> OffsetDateTime {
    OffsetDateTime::parse(text, &Rfc3339).unwrap()
}

/// One live run of `gone` journaled into `ws`'s world: it worked at
/// `long`, and the rule `LONG-OK` moved it on to `to`. Returns when the
/// rule fired and how long the run stayed at `long`, in milliseconds, as
/// the journal recorded them.
fn journal_a_long_run(ws: &Workspace, long: &str, to: &str) -> (String, i128) {
    let mut store = brokkr_store::Store::open(&ws.path().join("state/world.db")).unwrap();
    let manifest = json!({"bundle_name": "gone"});
    store
        .create_run("live", "feature", "gone", &manifest)
        .unwrap();
    let started = json!({"feature": "feature", "manifest": manifest});
    let decided = json!({"from": long, "result": "resolved", "rule_id": "LONG-OK",
                         "next": to, "severity": "normal", "inputs": {}, "problem": null});
    let effect = json!({"effect_id": "seat", "attempt_id": "seat"});
    let requested = json!({"effect_id": "seat", "seat": "seat", "phase": long});
    let succeeded = json!({"effect_id": "seat", "attempt_id": "seat",
                           "result": {"result": "resolved"}});
    let recorded: Vec<String> = [
        (EventType::RunStarted, started),
        (EventType::PhaseEntered, json!({"phase": long})),
        (EventType::EffectRequested, requested),
        (EventType::EffectStarted, effect),
        (EventType::EffectSucceeded, succeeded),
        (EventType::TransitionDecided, decided),
        (EventType::PhaseEntered, json!({"phase": to})),
    ]
    .into_iter()
    .map(|(kind, payload)| {
        let event = store.append_next("live", kind, payload, None, None);
        event.unwrap().recorded_at
    })
    .collect();
    let stay = read(&recorded[6]) - read(&recorded[1]);
    (recorded[5].clone(), stay.whole_milliseconds())
}

#[test]
fn graph_prints_its_view_as_json_or_as_tables_the_terminals_columns_fit() {
    let ws = Workspace::new();
    let long = "a-state-named-at-length-for-a-narrow-terminal";
    let to = format!("{long}-too");
    let (ruled_at, stay) = journal_a_long_run(&ws, long, &to);
    let graph = |json: &[&str]| {
        let mut brokkr = Command::new(env!("CARGO_BIN_EXE_brokkr"));
        brokkr.envs([("COLUMNS", "60"), ("NO_COLOR", "1")]);
        brokkr
            .current_dir(ws.path())
            .args(["stats", "graph", "--recipe", "gone"]);
        let out = brokkr.args(json).output().unwrap();
        let (code, stderr) = (out.status.code(), out.stderr);
        assert_eq!(code, Some(0), "{}", String::from_utf8_lossy(&stderr));
        String::from_utf8(out.stdout).unwrap()
    };
    // `--json` is the view model alone, pretty in its own field order,
    // pinned whole: only its two clock readings are the clock's, and those
    // are a week ending while the command ran.
    let before = OffsetDateTime::now_utc();
    let json = graph(&["--json"]);
    let after = OffsetDateTime::now_utc();
    let view: Value = serde_json::from_str(&json).unwrap();
    let [from, to_time] = ["from", "to"].map(|key| view[key].as_str().unwrap().to_string());
    assert!(
        before <= read(&to_time) && read(&to_time) <= after,
        "{to_time}"
    );
    let week_before = read(&to_time) - time::Duration::days(7);
    assert_eq!(read(&from), week_before.replace_nanosecond(0).unwrap());
    let unclocked =
        [("from", &from), ("to", &to_time)]
            .iter()
            .fold(json.clone(), |json, (key, text)| {
                let read = format!("\"{key}\": \"{text}\"");
                json.replacen(&read, &format!("\"{key}\": \"<time>\""), 1)
            });
    let state = |name: &str, millis: &str, took: &str| {
        let state = STATE_JSON.replace("<name>", name);
        state.replace("<millis>", millis).replace("<took>", took)
    };
    let states = [state(long, "<stay>", "0m"), state(&to, "null", "—")];
    let expected = GRAPH_JSON
        .replace("<states>", &states.join(",\n"))
        .replace("<long>", long)
        .replace("<to>", &to)
        .replace("<ruled>", &ruled_at)
        .replace("<stay>", &stay.to_string());
    assert_eq!(unclocked, expected);
    // Without it, tables the terminal's 60 columns fit: one-space gaps and
    // the state's name clamped, every number whole, no escape byte.
    let painted = graph(&[]);
    assert!(!painted.contains('\x1b'), "{painted:?}");
    let tables: Vec<&str> = painted.lines().skip(2).take(6).collect();
    assert_eq!(
        tables,
        [
            "STATE           ENTERED MEDIAN P90         COST TOKENS CACHE",
            "a-state-named-…       1     0m  0m not reported      —     —",
            "a-state-named-…       1      —   — not reported      —     —",
            "",
            "RULE    COUNT FROM               TO                  KIND",
            "LONG-OK     1 a-state-named-at-… a-state-named-at-l… forward",
        ],
        "{painted}"
    );
}
