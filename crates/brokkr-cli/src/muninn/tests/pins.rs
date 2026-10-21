//! The exact bytes of Muninn's documents (#346): the dossier the seat
//! reads and the line the record keeps. A child of the owning
//! `tests.rs`, which sits near its line ceiling.
//!
//! The dossier reaches the seat as a `Value`, so its keys print in byte
//! order whatever builds it; its pins hold what each part writes and
//! where a key is left out. The record line is written from the document
//! itself, so its pins hold the key order too.

use super::*;

/// `text` with each row's wall-clock `created_at`, and the `age` derived
/// from it, written in for their placeholders: the store stamps a run
/// when it is created, so the pins below name the run instead.
fn stamped(derived: &Dossier, text: &str) -> String {
    let mut text = text.to_string();
    for row in derived.value["runs"].as_array().unwrap() {
        let run = row["run_id"].as_str().unwrap();
        for key in ["created_at", "age"] {
            text = text.replace(&format!("\"{run}.{key}\""), &row[key].to_string());
        }
    }
    text
}

/// The dossier exactly as `seat_input` hands it over, pretty-printed.
fn printed(derived: &Dossier) -> String {
    serde_json::to_string_pretty(&derived.value).unwrap()
}

/// The exact bytes `record::append` writes for one line.
fn appended(line: &Value) -> String {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("muninn.ndjson");
    record::append(&path, line).unwrap();
    std::fs::read_to_string(&path).unwrap()
}

/// Two hearths: a parked run in alpha, a completed one in beta.
fn two_hearths() -> (Fleet, Fleet) {
    let alpha = Fleet::new();
    alpha.parked("parked-run");
    let beta = Fleet::new();
    beta.completed("done-run");
    (alpha, beta)
}

#[test]
fn the_one_hearth_dossier_prints_these_bytes() {
    let derived = parked_dossier();
    assert_eq!(printed(&derived), stamped(&derived, ONE_HEARTH));
}

#[test]
fn a_quarantined_row_prints_these_bytes() {
    let fleet = Fleet::new();
    fleet.poisoned("poisoned-run");
    let store = Store::open_read_only(&fleet.db()).unwrap();
    let derived = dossier(&store, NOW).unwrap();
    assert_eq!(printed(&derived), stamped(&derived, QUARANTINED));
}

#[test]
fn the_many_hearth_dossier_prints_these_bytes() {
    let (alpha, beta) = two_hearths();
    beta.poisoned("poisoned-run");
    let derived = many_hearth_dossier(&alpha, &beta);
    assert_eq!(printed(&derived), stamped(&derived, MANY_HEARTHS));
}

#[test]
fn the_crossing_dossier_prints_these_bytes() {
    let derived = crossing_dossier(&three_pin_crossings());
    assert_eq!(printed(&derived), CROSSINGS);
}

#[test]
fn the_one_hearth_record_line_is_these_bytes() {
    let seat = Staged::new(json!({"max_attempts": 1, "timeout_seconds": 900}))
        .seat()
        .unwrap();
    let derived = parked_dossier();
    let report = validate(&derived, &parked_report()).unwrap();
    let spent = usage(&[json!({"step": "claude-code-session-finished",
                               "total_cost_usd": 0.5, "num_turns": 4,
                               "session_id": "sess-1"})]);
    let line = entry(NOW, &seat, &derived, &report, spent);
    assert_eq!(appended(&line), ONE_HEARTH_LINE);
}

#[test]
fn the_many_hearth_record_line_is_these_bytes() {
    let seat = Staged::new(json!({"max_attempts": 1, "timeout_seconds": 900}))
        .seat()
        .unwrap();
    let (alpha, beta) = two_hearths();
    let derived = two_hearth_dossier(&alpha, &beta, &three_pin_crossings());
    let report = validate(
        &derived,
        &reported(json!({
            "fleet_summary": "a parked run in alpha and a moved contract in beta",
            "parked_runs": [{
                "run_id": "parked-run", "seq": 8, "command": "stop",
                "reasoning": "the ruling was hard",
            }],
            "work_queue": [
                {"realm": "beta", "crossing": "billing.api",
                 "finding": "the contract moved", "reasoning": "beta would refuse"},
                {"run_id": "parked-run", "seq": 7, "finding": "severity high",
                 "reasoning": "the highest residual"},
            ],
        })),
    )
    .unwrap();
    let line = entry(NOW, &seat, &derived, &report, usage(&[]));
    assert_eq!(appended(&line), MANY_HEARTHS_LINE);
}

/// [`parked_dossier`]: an awaiting-operator run with a seat, a cost, a
/// park reason and a ruling, a completed run with no cost, two findings,
/// and no `realms` or `crossings` key.
const ONE_HEARTH: &str = concat!(
    r#"{
  "dossier_version": 1,
  "fleet": {
    "awaiting_operator": 1,
    "completed": 1,
    "quarantined": 0,
    "running": 0,
    "runs": 2,
    "stopped": 0,
    "superseded_findings": 0
  },
  "generated_at": "2099-01-01T00:00:00Z",
  "residual_findings": [
    {
      "input": "has_security_residual",
      "line": "parked-run seq 7 · review · REVIEW-RESIDUAL-ABOVE-MEDIUM · has_security_residual: true",
      "phase": "review",
      "rule_id": "REVIEW-RESIDUAL-ABOVE-MEDIUM",
      "run_id": "parked-run",
      "seq": 7,
      "superseded": null,
      "value": "true"
    },
    {
      "input": "max_residual_severity",
      "line": "parked-run seq 7 · review · REVIEW-RESIDUAL-ABOVE-MEDIUM · max_residual_severity: high",
      "phase": "review",
      "rule_id": "REVIEW-RESIDUAL-ABOVE-MEDIUM",
      "run_id": "parked-run",
      "seq": 7,
      "superseded": null,
      "value": "high"
    }
  ],
  "runs": [
"#,
    parked_row_to_realm!(),
    parked_row_from_realm_to_done_realm!(),
    done_row_from_realm!(),
    r#"    }
  ]
}"#
);

// The two run rows [`ONE_HEARTH`] and [`MANY_HEARTHS`] both print, cut
// where a many-hearth dossier writes each row's realm: each byte of them
// is written once, and both pins stay exact literals.

/// The parked run's row, up to where its realm goes.
macro_rules! parked_row_to_realm {
    () => {
        r#"    {
      "age": "parked-run.age",
      "consecutive_failures": {
        "review": 0
      },
      "cost_usd": 1.25,
      "created_at": "parked-run.created_at",
      "feature": "make it shippable",
      "last_ruling": {
        "from": "review",
        "inputs": [
          [
            "has_security_residual",
            "true"
          ],
          [
            "max_residual_severity",
            "\"high\""
          ]
        ],
        "next": "?",
        "problem": "residual severity above medium",
        "result": "residual",
        "rule_id": "REVIEW-RESIDUAL-ABOVE-MEDIUM",
        "severity_class": "",
        "superseded": null
      },
      "operator_commands": [
        "retry",
        "stop"
      ],
      "park_reason": "residual severity above medium; not shippable",
      "phase": "review",
"#
    };
}
use parked_row_to_realm;

/// The rest of the parked run's row, and the completed run's up to where
/// its realm goes.
macro_rules! parked_row_from_realm_to_done_realm {
    () => {
        r#"      "run_id": "parked-run",
      "seats": [
        {
          "attempts": 1,
          "cost_usd": 1.25,
          "phase": "review",
          "seat": "review",
          "status": "succeeded"
        }
      ],
      "seq": 8,
      "status": "awaiting_operator"
    },
    {
      "age": "done-run.age",
      "consecutive_failures": {
        "ship": 0
      },
      "cost_usd": null,
      "created_at": "done-run.created_at",
      "feature": "ship it",
      "last_ruling": {
        "from": "ship",
        "inputs": [],
        "next": "done",
        "problem": null,
        "result": "shipped",
        "rule_id": "SHIP-OK",
        "severity_class": "",
        "superseded": null
      },
      "operator_commands": [],
      "park_reason": null,
      "phase": "done",
"#
    };
}
use parked_row_from_realm_to_done_realm;

/// The rest of the completed run's row, open for what follows it.
macro_rules! done_row_from_realm {
    () => {
        r#"      "run_id": "done-run",
      "seats": [
        {
          "attempts": 1,
          "cost_usd": null,
          "phase": "ship",
          "seat": "ship",
          "status": "succeeded"
        }
      ],
      "seq": 8,
      "status": "completed"
"#
    };
}
use done_row_from_realm;

/// One journal that does not fold: its row with `fold_error` and a null
/// phase, and its finding.
const QUARANTINED: &str = r#"{
  "dossier_version": 1,
  "fleet": {
    "awaiting_operator": 0,
    "completed": 0,
    "quarantined": 1,
    "running": 0,
    "runs": 1,
    "stopped": 0,
    "superseded_findings": 0
  },
  "generated_at": "2099-01-01T00:00:00Z",
  "residual_findings": [
    {
      "input": "journal_folds",
      "line": "poisoned-run seq 6 · journal does not fold · event 6: operator/accepted without a matching command",
      "phase": "—",
      "rule_id": "—",
      "run_id": "poisoned-run",
      "seq": 6,
      "superseded": null,
      "value": "false"
    }
  ],
  "runs": [
    {
      "age": "poisoned-run.age",
      "created_at": "poisoned-run.created_at",
      "feature": "the acceptance that names no command",
      "fold_error": "event 6: operator/accepted without a matching command",
      "operator_commands": [],
      "phase": null,
      "run_id": "poisoned-run",
      "seq": 6,
      "status": "?"
    }
  ]
}"#;

/// Two hearths: `fleet.realms`, and a realm on every row and finding.
const MANY_HEARTHS: &str = concat!(
    r#"{
  "dossier_version": 1,
  "fleet": {
    "awaiting_operator": 1,
    "completed": 1,
    "quarantined": 1,
    "realms": [
      "alpha",
      "beta"
    ],
    "running": 0,
    "runs": 3,
    "stopped": 0,
    "superseded_findings": 0
  },
  "generated_at": "2099-01-01T00:00:00Z",
  "residual_findings": [
    {
      "input": "has_security_residual",
      "line": "parked-run seq 7 · review · REVIEW-RESIDUAL-ABOVE-MEDIUM · has_security_residual: true",
      "phase": "review",
      "realm": "alpha",
      "rule_id": "REVIEW-RESIDUAL-ABOVE-MEDIUM",
      "run_id": "parked-run",
      "seq": 7,
      "superseded": null,
      "value": "true"
    },
    {
      "input": "max_residual_severity",
      "line": "parked-run seq 7 · review · REVIEW-RESIDUAL-ABOVE-MEDIUM · max_residual_severity: high",
      "phase": "review",
      "realm": "alpha",
      "rule_id": "REVIEW-RESIDUAL-ABOVE-MEDIUM",
      "run_id": "parked-run",
      "seq": 7,
      "superseded": null,
      "value": "high"
    },
    {
      "input": "journal_folds",
      "line": "poisoned-run seq 6 · journal does not fold · event 6: operator/accepted without a matching command",
      "phase": "—",
      "realm": "beta",
      "rule_id": "—",
      "run_id": "poisoned-run",
      "seq": 6,
      "superseded": null,
      "value": "false"
    }
  ],
  "runs": [
"#,
    parked_row_to_realm!(),
    r#"      "realm": "alpha",
"#,
    parked_row_from_realm_to_done_realm!(),
    r#"      "realm": "beta",
"#,
    done_row_from_realm!(),
    r#"    },
    {
      "age": "poisoned-run.age",
      "created_at": "poisoned-run.created_at",
      "feature": "the acceptance that names no command",
      "fold_error": "event 6: operator/accepted without a matching command",
      "operator_commands": [],
      "phase": null,
      "realm": "beta",
      "run_id": "poisoned-run",
      "seq": 6,
      "status": "?"
    }
  ]
}"#
);

/// [`three_pin_crossings`] over two empty hearths: a matching pin (detail
/// null), a moved one, an unchecked one, the moved pin's finding, and an
/// empty `consumes` and `publishes` written out.
const CROSSINGS: &str = r#"{
  "crossings": [
    {
      "consumes": [],
      "publishes": [
        {
          "name": "orders.api",
          "path": "contracts/orders.v1.schema.json"
        }
      ],
      "realm": "alpha"
    },
    {
      "consumes": [
        {
          "detail": null,
          "name": "orders.api",
          "pin": "matching",
          "realm": "alpha"
        },
        {
          "detail": "realm 'beta' consumes crossing 'billing.api' from realm 'alpha' pinned at aaa, but contracts/billing.v1.schema.json hashes to bbb",
          "name": "billing.api",
          "pin": "moved",
          "realm": "alpha"
        },
        {
          "detail": "realm 'ledger' publishes it and its file could not be read",
          "name": "audit.api",
          "pin": "unchecked",
          "realm": "ledger"
        }
      ],
      "publishes": [],
      "realm": "beta"
    }
  ],
  "dossier_version": 1,
  "fleet": {
    "awaiting_operator": 0,
    "completed": 0,
    "quarantined": 0,
    "realms": [
      "alpha",
      "beta"
    ],
    "running": 0,
    "runs": 0,
    "stopped": 0,
    "superseded_findings": 0
  },
  "generated_at": "2099-01-01T00:00:00Z",
  "residual_findings": [
    {
      "crossing": "billing.api",
      "input": "crossing_pins",
      "line": "realm 'beta' consumes crossing 'billing.api' from realm 'alpha' pinned at aaa, but contracts/billing.v1.schema.json hashes to bbb",
      "publisher": "alpha",
      "realm": "beta",
      "value": "moved"
    }
  ],
  "runs": []
}"#;

/// One hearth: a parked proposal, a run queue entry, and the session's
/// usage.
const ONE_HEARTH_LINE: &str = concat!(
    r#"{"agent":{"deadline_seconds":900,"model":"opus","name":"muninn","provider":"test"},"#,
    r#""citations":[{"run_id":"parked-run","seq":7},{"run_id":"parked-run","seq":8}],"#,
    r#""dossier":{"dossier_version":1,"#,
    r#""fleet":{"awaiting_operator":1,"completed":1,"quarantined":0,"running":0,"runs":2,"#,
    r#""stopped":0,"superseded_findings":0},"generated_at":"2099-01-01T00:00:00Z"},"#,
    r#""fleet_summary":"one run is parked","#,
    r#""parked_runs":[{"command":"stop","reasoning":"the ruling was hard","#,
    r#""run_id":"parked-run","seq":8}],"#,
    r#""record_version":1,"recorded_at":"2099-01-01T00:00:00Z","#,
    r#""usage":{"cost_usd":0.5,"session_id":"sess-1","turns":4},"#,
    r#""work_queue":[{"finding":"severity high","reasoning":"the highest residual","#,
    r#""run_id":"parked-run","seq":7}]}"#,
    "\n"
);

/// Many hearths: realm citations, a crossing queue entry, its crossing
/// citation, the dossier's crossings, and no usage.
const MANY_HEARTHS_LINE: &str = concat!(
    r#"{"agent":{"deadline_seconds":900,"model":"opus","name":"muninn","provider":"test"},"#,
    r#""citations":[{"realm":"alpha","run_id":"parked-run","seq":7},"#,
    r#"{"realm":"alpha","run_id":"parked-run","seq":8}],"#,
    r#""crossing_citations":[{"crossing":"billing.api","realm":"beta"}],"#,
    r#""dossier":{"crossings":[{"consumes":[],"#,
    r#""publishes":[{"name":"orders.api","path":"contracts/orders.v1.schema.json"}],"#,
    r#""realm":"alpha"},"#,
    r#"{"consumes":[{"detail":null,"name":"orders.api","pin":"matching","realm":"alpha"},"#,
    r#"{"detail":"realm 'beta' consumes crossing 'billing.api' from realm 'alpha' "#,
    r#"pinned at aaa, but contracts/billing.v1.schema.json hashes to bbb","#,
    r#""name":"billing.api","pin":"moved","realm":"alpha"},"#,
    r#"{"detail":"realm 'ledger' publishes it and its file could not be read","#,
    r#""name":"audit.api","pin":"unchecked","realm":"ledger"}],"#,
    r#""publishes":[],"realm":"beta"}],"#,
    r#""dossier_version":1,"#,
    r#""fleet":{"awaiting_operator":1,"completed":1,"quarantined":0,"realms":["alpha","beta"],"#,
    r#""running":0,"runs":2,"stopped":0,"superseded_findings":0},"#,
    r#""generated_at":"2099-01-01T00:00:00Z"},"#,
    r#""fleet_summary":"a parked run in alpha and a moved contract in beta","#,
    r#""parked_runs":[{"command":"stop","realm":"alpha","reasoning":"the ruling was hard","#,
    r#""run_id":"parked-run","seq":8}],"#,
    r#""record_version":1,"recorded_at":"2099-01-01T00:00:00Z","usage":null,"#,
    r#""work_queue":[{"crossing":"billing.api","finding":"the contract moved","realm":"beta","#,
    r#""reasoning":"beta would refuse"},"#,
    r#"{"finding":"severity high","realm":"alpha","reasoning":"the highest residual","#,
    r#""run_id":"parked-run","seq":7}]}"#,
    "\n"
);
