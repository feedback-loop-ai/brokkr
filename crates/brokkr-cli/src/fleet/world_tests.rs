//! `brokkr runs` over a many-hearth world: each hearth's runs under its
//! realm, in the JSON and the table it has always printed, a hearth whose
//! journal will not open, or one of whose pages the store will not read,
//! listed with that refusal beside the others.

use std::path::{Path, PathBuf};

use brokkr_runtime::realms::Hearth;
use brokkr_store::Store;
use brokkr_view::{FleetView, RealmRuns, RunRow, VIEW_VERSION};

use super::kept_tests::{unlisted, UNLISTED};
use super::listing::Lister;
use super::page::tests::{
    arranged, behind_in_every_section, folded, now, plant, stamp, whole_rows, Shape, DAY, HOUR,
    MINUTE, NOW, STYLE,
};
use super::page::TAIL_KEYS;
use super::world::{list_world, list_world_in};
use super::{list_hearth, read_journal, Journal, PAGE_ROWS};

fn hearth(realm: &str, journal: PathBuf) -> Hearth {
    Hearth {
        realms: vec![realm.to_string()],
        journal,
    }
}

/// Three hearths under `dir`: alpha, a run in each of four sections, one
/// of them quarantined; beta, whose journal is not there; and gamma, a
/// run ruled to hold days ago.
fn world(dir: &Path) -> Vec<Hearth> {
    let alpha = dir.join("alpha.db");
    folded(&alpha, |conn| {
        let ago = |minutes: i64| stamp(now() - minutes * MINUTE);
        plant(conn, "done-a", Shape::Finished, &ago(5));
        plant(conn, "fresh-a", Shape::Running, &ago(10));
        plant(conn, "refused-a", Shape::Refused, &ago(30));
        plant(conn, "park-a", Shape::Parked, &ago(HOUR / MINUTE));
    });
    let gamma = dir.join("gamma.db");
    folded(&gamma, |conn| {
        plant(conn, "held-g", Shape::Held, &stamp(now() - 2 * DAY));
    });
    vec![
        hearth("alpha", alpha),
        hearth("beta", dir.join("beta.db")),
        hearth("gamma", gamma),
    ]
}

/// What `brokkr runs` prints of `hearths` at [`NOW`], `dir` written as
/// `<dir>`.
fn listed(hearths: &[Hearth], json: bool, dir: &Path) -> String {
    let mut out = Vec::new();
    list_world(hearths, json, (NOW, &STYLE), &mut out).unwrap();
    let out = String::from_utf8(out).unwrap();
    out.replace(&dir.display().to_string(), "<dir>")
}

/// The exact bytes `brokkr runs --json` and `brokkr runs` print of a
/// world of three hearths, one whose journal is not there and one holding
/// a quarantined run.
#[test]
fn a_many_hearth_world_prints_these_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let hearths = world(dir.path());
    assert_eq!(listed(&hearths, true, dir.path()), WORLD_JSON);
    assert_eq!(listed(&hearths, false, dir.path()), WORLD_TEXT);
}

/// A hearth whose journal is not there lists its refusal in the store's
/// own words, and opens nothing into being; the hearths beside it list
/// every run.
#[test]
fn a_hearth_whose_journal_is_missing_lists_its_refusal_beside_the_others() {
    let dir = tempfile::tempdir().unwrap();
    let hearths = world(dir.path());
    let view: serde_json::Value =
        serde_json::from_str(&listed(&hearths, true, dir.path())).unwrap();
    let realms = view["realms"].as_array().unwrap();
    let counted: Vec<(&str, u64, &serde_json::Value)> = realms
        .iter()
        .map(|realm| {
            let name = realm["realm"].as_str().unwrap();
            (name, realm["count"].as_u64().unwrap(), &realm["detail"])
        })
        .collect();
    let refusal = serde_json::json!("sqlite: unable to open database file: <dir>/beta.db");
    let whole = read_journal(&dir.path().join("beta.db")).detail.unwrap();
    assert_eq!(
        whole.replace(&dir.path().display().to_string(), "<dir>"),
        refusal
    );
    let none = serde_json::Value::Null;
    let expected = [
        ("alpha", 4, &none),
        ("beta", 0, &refusal),
        ("gamma", 1, &none),
    ];
    assert_eq!(counted, expected);
    assert_eq!(view["count"], 5);
    assert!(!dir.path().join("beta.db").exists());
}

/// Ruling 6 across hearths: a hearth with more runs behind their summary
/// than a batch of keys holds lists every run once, each at its place in
/// the section its fold places it in, as the whole read of that hearth
/// arranges them, beside a hearth whose runs the store cannot list; in the
/// JSON and the table alike, and at a batch smaller than any section's
/// runs behind as at the listing's own.
#[test]
fn every_hearth_lists_every_run_once_however_many_are_behind() {
    let dir = tempfile::tempdir().unwrap();
    let behind = dir.path().join("behind.db");
    behind_in_every_section(&behind);
    let unlisted = unlisted(dir.path());
    let hearths = [
        hearth("behind", behind.clone()),
        hearth("unlisted", unlisted.clone()),
    ];
    let runs = arranged(&whole_rows(&behind));
    let count = runs.len();
    let realm = |name: &str, journal: &Path, runs: Vec<RunRow>, detail: Option<&str>| RealmRuns {
        realm: name.to_string(),
        journal: journal.display().to_string(),
        count: runs.len(),
        runs,
        detail: detail.map(str::to_string),
    };
    let view = FleetView {
        view_version: VIEW_VERSION,
        realms: vec![
            realm("behind", &behind, runs, None),
            realm("unlisted", &unlisted, Vec::new(), Some(UNLISTED)),
        ],
        count,
    };
    let json = format!("{}\n", serde_json::to_string_pretty(&view).unwrap());
    let text = crate::render::tests::fleet(&view, NOW, &STYLE);
    for keys in [3, TAIL_KEYS] {
        let printed = |json| {
            let mut out = Vec::new();
            list_world_in(&hearths, (json, keys), (NOW, &STYLE), &mut out).unwrap();
            String::from_utf8(out).unwrap()
        };
        assert_eq!(printed(true), json, "{keys} keys a batch");
        assert_eq!(printed(false), text, "{keys} keys a batch");
    }
}

/// A journal at `db` of `count` finished runs, `run-000` an hour before
/// [`NOW`] and each after it an hour earlier.
fn finished(db: &Path, count: usize) {
    folded(db, |conn| {
        for (hours, n) in (1..).zip(0..count) {
            let at = stamp(now() - hours * HOUR);
            plant(conn, &format!("run-{n:03}"), Shape::Finished, &at);
        }
    });
}

/// What the store says of a page holding a run whose summary names its
/// feature in bytes, not text.
const UNREAD: &str = "sqlite: Invalid column type Blob at index: 0, name: feature";

/// A journal at `db` whose pages list, a page of runs and two older, the
/// oldest's summary naming its feature in bytes: its first page reads,
/// and the store refuses its second.
fn unread_past_a_page(db: &Path) {
    finished(db, PAGE_ROWS + 2);
    let oldest = format!("run-{:03}", PAGE_ROWS + 1);
    let cast = "UPDATE run_summary SET feature = CAST(feature AS BLOB) WHERE run_id = ?1";
    let conn = rusqlite::Connection::open(db).unwrap();
    assert_eq!(conn.execute(cast, [&oldest]).unwrap(), 1);
}

/// A hearth whose pages list but one of whose pages the store will not
/// read lists the runs of the pages before it, counts those, and states
/// the store's refusal where its runs stop; the hearth after it lists
/// every run. In the JSON and the table alike, each as the same world
/// prints a hearth of just the runs that read.
#[test]
fn a_hearth_a_page_of_which_will_not_read_lists_what_read_and_its_refusal_beside_the_others() {
    let dir = tempfile::tempdir().unwrap();
    let (unread, read) = (dir.path().join("unread.db"), dir.path().join("read.db"));
    unread_past_a_page(&unread);
    finished(&read, PAGE_ROWS);
    let gamma = dir.path().join("gamma.db");
    folded(&gamma, |conn| {
        plant(conn, "held-g", Shape::Held, &stamp(now() - 2 * DAY));
    });
    let printed = |first: &Path, json| {
        let hearths = [
            hearth("first", first.to_path_buf()),
            hearth("gamma", gamma.clone()),
        ];
        listed(&hearths, json, dir.path())
    };
    let json = printed(&unread, true);
    let view: serde_json::Value = serde_json::from_str(&json).unwrap();
    let counts = [&view["realms"][0]["count"], &view["realms"][1]["count"]];
    assert_eq!(
        (counts, &view["count"]),
        ([&50.into(), &1.into()], &51.into())
    );
    let stated = format!("\n      \"detail\": \"{UNREAD}\"\n");
    let expected = printed(&read, true)
        .replace("<dir>/read.db", "<dir>/unread.db")
        .replacen("\n      \"detail\": null\n", &stated, 1);
    assert_eq!(json, expected);
    let head = |db: &str| format!("first · {PAGE_ROWS} runs · <dir>/{db}\n");
    let stated = format!("{}  journal  {UNREAD}\n", head("unread.db"));
    let expected = printed(&read, false).replacen(&head("read.db"), &stated, 1);
    assert_eq!(printed(&unread, false), expected);
}

/// The one-hearth listing of that journal is refused whole, in the
/// store's words, as it always was: its JSON, its table and the
/// console's rows alike.
#[test]
fn a_one_hearth_listing_a_page_of_which_will_not_read_is_refused_in_the_stores_words() {
    let dir = tempfile::tempdir().unwrap();
    let unread = dir.path().join("unread.db");
    unread_past_a_page(&unread);
    let refused = |list: &dyn Fn(&Lister<'_>) -> anyhow::Result<()>| {
        let store = Store::open_read_only(&unread).unwrap();
        let journal = (Journal::at(&unread), NOW);
        let listed = list_hearth(&store, journal, true, |lister| list(lister));
        listed.unwrap_err().to_string()
    };
    let json = refused(&|lister| lister.every_json(&mut std::io::sink()));
    let text = refused(&|lister| lister.every_text(&mut std::io::sink(), &STYLE));
    let rows = refused(&|lister| lister.rows().map(drop));
    assert_eq!([json, text, rows], [UNREAD; 3]);
}

/// [`world`] as `brokkr runs --json` prints it.
const WORLD_JSON: &str = r#"{
  "view_version": 28,
  "realms": [
    {
      "realm": "alpha",
      "journal": "<dir>/alpha.db",
      "runs": [
        {
          "run_id": "refused-a",
          "status": null,
          "status_known": false,
          "phase": null,
          "seq": null,
          "created_at": "2026-01-01T00:00:00Z",
          "last_recorded_at": "2026-10-19T23:30:00Z",
          "feature": "feat",
          "detail": "event 2: operator/accepted without a matching command",
          "quarantine": "does_not_fold",
          "acknowledgement": null,
          "residuals": [],
          "title": "feat",
          "verdict": {
            "standing": "quarantined",
            "rule": null,
            "residual": null,
            "reason": null,
            "text": "does not fold",
            "detail": null
          },
          "hire": null
        },
        {
          "run_id": "park-a",
          "status": "awaiting_operator",
          "status_known": true,
          "phase": "review",
          "seq": 7,
          "created_at": "2026-01-01T00:00:00Z",
          "last_recorded_at": "2026-10-19T23:00:00Z",
          "feature": "feat",
          "detail": null,
          "quarantine": null,
          "acknowledgement": null,
          "residuals": [
            {
              "run_id": "park-a",
              "seq": 6,
              "phase": "review",
              "rule_id": "REVIEW-HOLD",
              "input": "max_residual_severity",
              "value": "high",
              "line": "park-a seq 6 · review · REVIEW-HOLD · max_residual_severity: high",
              "superseded": null
            }
          ],
          "title": "feat",
          "verdict": {
            "standing": "parked",
            "rule": "REVIEW-HOLD",
            "residual": "high",
            "reason": "REVIEW-HOLD for (review, hold): held",
            "text": "HOLD",
            "detail": null
          },
          "hire": null
        },
        {
          "run_id": "fresh-a",
          "status": "running",
          "status_known": true,
          "phase": null,
          "seq": 1,
          "created_at": "2026-01-01T00:00:00Z",
          "last_recorded_at": "2026-10-19T23:50:00Z",
          "feature": "feat",
          "detail": null,
          "quarantine": null,
          "acknowledgement": null,
          "residuals": [],
          "title": "feat",
          "verdict": {
            "standing": "running",
            "rule": null,
            "residual": null,
            "reason": null,
            "text": "",
            "detail": null
          },
          "hire": null
        },
        {
          "run_id": "done-a",
          "status": "completed",
          "status_known": true,
          "phase": null,
          "seq": 2,
          "created_at": "2026-01-01T00:00:00Z",
          "last_recorded_at": "2026-10-19T23:55:00Z",
          "feature": "feat",
          "detail": null,
          "quarantine": null,
          "acknowledgement": null,
          "residuals": [],
          "title": "feat",
          "verdict": {
            "standing": "shipped",
            "rule": null,
            "residual": null,
            "reason": null,
            "text": "—",
            "detail": null
          },
          "hire": null
        }
      ],
      "count": 4,
      "detail": null
    },
    {
      "realm": "beta",
      "journal": "<dir>/beta.db",
      "runs": [],
      "count": 0,
      "detail": "sqlite: unable to open database file: <dir>/beta.db"
    },
    {
      "realm": "gamma",
      "journal": "<dir>/gamma.db",
      "runs": [
        {
          "run_id": "held-g",
          "status": "running",
          "status_known": true,
          "phase": "review",
          "seq": 6,
          "created_at": "2026-01-01T00:00:00Z",
          "last_recorded_at": "2026-10-18T00:00:00Z",
          "feature": "feat",
          "detail": null,
          "quarantine": null,
          "acknowledgement": null,
          "residuals": [
            {
              "run_id": "held-g",
              "seq": 6,
              "phase": "review",
              "rule_id": "REVIEW-HOLD",
              "input": "max_residual_severity",
              "value": "high",
              "line": "held-g seq 6 · review · REVIEW-HOLD · max_residual_severity: high",
              "superseded": null
            }
          ],
          "title": "feat",
          "verdict": {
            "standing": "running",
            "rule": null,
            "residual": "high",
            "reason": null,
            "text": "",
            "detail": null
          },
          "hire": null
        }
      ],
      "count": 1,
      "detail": null
    }
  ],
  "count": 5
}
"#;

/// [`world`] as `brokkr runs` prints it, 120 columns and no colour.
const WORLD_TEXT: &str = r#"alpha · 4 runs · <dir>/alpha.db
refused-a ?                 -      seq - 7008h00m feat
  fold  event 2: operator/accepted without a matching command
park-a    awaiting_operator review seq 7 7008h00m feat
fresh-a   running           -      seq 1 7008h00m feat
done-a    completed         -      seq 2 7008h00m feat

beta · 0 runs · <dir>/beta.db
  journal  sqlite: unable to open database file: <dir>/beta.db

gamma · 1 run · <dir>/gamma.db
held-g running review seq 6 7008h00m feat
"#;
