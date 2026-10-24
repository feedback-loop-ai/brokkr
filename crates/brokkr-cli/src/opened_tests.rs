//! Decision 0089 ruling 5: every readout of one run opens it, its chain
//! read whole and verified before anything else of it is read, and keeps
//! the store's refusal of a corrupt run in the store's own words, however
//! the fleet's page, which checks a row against its run's head alone,
//! lists the run.

use std::path::Path;

use brokkr_core::envelope::ChainError;
use brokkr_store::test_support::unmigrate;
use brokkr_store::{Store, StoreError};
use brokkr_view::Quarantine;
use clap::Parser;

use crate::tests::running_store;
use crate::tui_read::tui_views;
use crate::{run, tui, Cli};

/// The store's refusal of a run whose first event was edited.
const EDITED: &str = "chain: event 1: event_hash does not match canonical content";

/// Runs `r1` and `r2` in a fresh journal at `db`, then `r1`'s first event
/// edited beneath its head by a hand at the file: its summary row still
/// stands at its head.
fn edited_below_its_head(db: &Path) {
    running_store(db, "r1");
    running_store(db, "r2");
    let journal = rusqlite::Connection::open(db).unwrap();
    journal
        .execute_batch("DROP TRIGGER events_append_only_update")
        .unwrap();
    let edit = r#"UPDATE events SET envelope = replace(envelope, '"manifest":{}',
        '"manifest":{"edited":true}') WHERE run_id = 'r1' AND seq = 1"#;
    assert_eq!(journal.execute(edit, []).unwrap(), 1);
}

/// The console's frame over `db`, the operator on `r1`: whether it drew a
/// run view, and the fleet's row of `r1`.
fn opened_in_the_console(db: &Path) -> (bool, brokkr_view::RunRow) {
    let ask = tui::Ask {
        run: Some("r1"),
        subject: None,
        force: true,
        fleet: true,
        tab: 0,
        window: Default::default(),
    };
    let clock = || "2026-10-20T00:00:00Z".to_string();
    let views = tui_views(db, true, ask, &mut None, &mut None, clock).unwrap();
    let views = views.expect("a forced frame is drawn");
    let row = views.runs.runs.into_iter().find(|row| row.run_id == "r1");
    (views.run.is_some(), row.expect("the opened run is listed"))
}

/// The page `brokkr ui` serves.
const PAGE: &str = include_str!("ui.html");

/// The served page's text between `/* <name>:start */` and its end.
fn page_block(name: &str) -> &'static str {
    let (start, end) = (format!("/* {name}:start */"), format!("/* {name}:end */"));
    let from = PAGE.find(&start).expect("the page marks the block's start") + start.len();
    let to = from + PAGE[from..].find(&end).expect("the page marks its end");
    &PAGE[from..to]
}

/// The DOM, the fetch and the page globals the page's reads of one run
/// meet in a browser, each recording what reaches it: `fetch` answers
/// each route as `brokkr ui` served it (`__served`).
const BROWSER: &str = r#"
var __painted = [];
var __transcript = [];
function el(tag, cls, text) {
  return { tag: tag, cls: cls ?? null, text: text ?? null, children: [],
    appendChild: function (child) { this.children.push(child); } };
}
function badge(text) { return el('span', 'badge', text); }
var __detail = el('div');
__detail.replaceChildren = function () { this.children = []; };
var document = { getElementById: function () { return __detail; } };
function renderDetail(id) { __painted.push('view ' + id); }
var activeParticipant = 'work', transcriptBox = {}, transcriptPart = {}, explicitSelectKey = 'work';
var transcriptController = { clear: function () { __painted.push('cleared'); } };
var __unreachable = 0, __timers = [];
function fetch(url) {
  if (__unreachable > 0) {
    __unreachable -= 1;
    return Promise.reject(new TypeError('Failed to fetch'));
  }
  var served = __served[url];
  return Promise.resolve({ ok: served.status < 300, status: served.status,
    json: function () { return Promise.resolve(JSON.parse(served.body)); } });
}
var __participant = createTranscriptController({
  requestPresentation: function (subject) {
    return fetchJson('/api/presentation/' + subject.runId + '/' + subject.participantKey);
  },
  requestBody: function () { return fetchJson('/api/transcript/r1/work'); },
  openWatch: function () { return {}; }, closeWatch: function () {},
  schedule: function (tick) { __timers.push(tick); return {}; },
  cancel: function () {}, clear: function () {},
  render: function (view) { __transcript.push(view.reason + ': ' + view.explanation); }
});
function __drawn() {
  return JSON.stringify({ painted: __painted, detail: __detail.children,
    transcript: __transcript, active: activeParticipant });
}
"#;

/// The operator drilling run `run`'s participant `work`, in the page.
fn drill(run: &str) -> String {
    format!(
        "__participant.operator_select({{ runId: '{run}', participantKey: 'work', \
         reference: null, sessionId: null, provider: null, working: false }});"
    )
}

/// The operator opening run `run` in the detail pane, in the page.
fn open(run: &str) -> String {
    format!("loadDetail('{run}').catch(function (error) {{ __painted.push('threw ' + error); }});")
}

/// What the page `brokkr ui` serves draws of run `run` over `db`, its own
/// code run in Boa against each route as `handle` serves it: the run
/// opened in the detail pane, and its participant `work` drilled.
fn drawn_in_the_page(db: &Path, run: &str) -> serde_json::Value {
    drawn_after(db, run, &[&drill(run), &open(run)])
}

/// What the page draws of run `run` over `db` after `steps`, each run in
/// turn with its promises settled, against each route as `handle` serves
/// it.
fn drawn_after(db: &Path, run: &str, steps: &[&str]) -> serde_json::Value {
    let routes = [
        format!("/api/view/{run}"),
        format!("/api/presentation/{run}/work"),
    ];
    let served: serde_json::Map<String, serde_json::Value> = routes
        .iter()
        .map(|route| {
            let answer = crate::ui::handle(db, route);
            let status: u16 = answer.status[..3].parse().unwrap();
            let served = serde_json::json!({"status": status, "body": answer.body});
            (route.clone(), served)
        })
        .collect();
    let mut boa = boa_engine::Context::default();
    let served = format!("var __served = {};", serde_json::Value::Object(served));
    let page = [
        served.as_str(),
        page_block("transcript-controller"),
        page_block("opened-run"),
        BROWSER,
    ];
    for source in page.iter().chain(steps) {
        boa.eval(boa_engine::Source::from_bytes(source.as_bytes()))
            .unwrap_or_else(|error| panic!("evaluating the page: {error}"));
        boa.run_jobs().expect("the page's promises settle");
    }
    let drawn = boa
        .eval(boa_engine::Source::from_bytes(b"__drawn()"))
        .unwrap();
    let drawn = drawn.as_string().expect("the page's drawing reads as text");
    serde_json::from_str(&drawn.to_std_string_escaped()).unwrap()
}

/// Is `refusal` the store's of the edited event, its broken hash?
fn is_the_edit(refusal: Option<&StoreError>) -> bool {
    matches!(
        refusal,
        Some(StoreError::Chain(ChainError::BadHash { seq: 1 }))
    )
}

/// `line` run as `brokkr` runs it, refused: the refusal's words, and each
/// cause's beneath them. A readout that reads the run, or refuses it with
/// no store refusal of the edit among its causes, is named.
fn refused(line: &[&str]) -> Vec<String> {
    let cli = Cli::try_parse_from(line).unwrap();
    let Err(refusal) = run(cli) else {
        panic!("{line:?} read the corrupt run");
    };
    let store = refusal.chain().find_map(|cause| cause.downcast_ref());
    assert!(is_the_edit(store), "{line:?}: {store:?}");
    refusal.chain().map(ToString::to_string).collect()
}

/// The opening the readouts move to refuses the run as they do, in the
/// store's variant and words.
fn the_opening_refuses_it(db: &Path) {
    let opened = Store::open_read_only(db).unwrap().open_run("r1");
    assert!(is_the_edit(opened.as_ref().err()), "{opened:?}");
    assert_eq!(opened.unwrap_err().to_string(), EDITED);
}

/// The binding ruling 5 names: a journal with one event below a run's
/// head edited, whose summary row the page's head check passes, is
/// refused by every readout of that run in the store's words, each verb
/// with the store's own variant, the edited event's broken hash, as the
/// opening the readouts move to refuses it — the
/// console's opened run, on a journal the summary pages and on one it does
/// not, shows it quarantined and draws no run; `/api/view`, the
/// participant presentation and the transcript refuse it in the same
/// words, where a run the journal does not hold is not found — and the
/// run beside it reads.
#[test]
fn a_corrupt_run_is_refused_wherever_it_is_opened() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    edited_below_its_head(&db);
    let at = db.to_str().unwrap();
    let out = dir.path().join("out");
    let readouts: [&[&str]; 9] = [
        &["brokkr", "inspect", "--run", "r1", "--db", at],
        &["brokkr", "seats", "--run", "r1", "--db", at],
        &[
            "brokkr",
            "watch",
            "--run",
            "r1",
            "--db",
            at,
            "--once",
            "--no-view",
        ],
        &[
            "brokkr",
            "transcript",
            "--run",
            "r1",
            "--seat",
            "work",
            "--db",
            at,
        ],
        &["brokkr", "ledger", "--run", "r1", "--db", at],
        &["brokkr", "replay", "--run", "r1", "--db", at],
        &["brokkr", "costs", "--run", "r1", "--db", at],
        &[
            "brokkr",
            "export",
            "--run",
            "r1",
            "--db",
            at,
            "--out",
            out.to_str().unwrap(),
        ],
        &["brokkr", "compare", "r2", "r1", "--db", at],
    ];
    let chain = "event 1: event_hash does not match canonical content";
    for line in readouts {
        let words = match line[1] {
            "compare" => vec!["loading run 'r1'", EDITED, chain],
            _ => vec![EDITED, chain],
        };
        assert_eq!(refused(line), words, "{line:?}");
    }
    the_opening_refuses_it(&db);
    let served = serde_json::json!({ "error": EDITED }).to_string();
    for route in [
        "/api/view/r1",
        "/api/presentation/r1/work",
        "/api/transcript/r1/work",
    ] {
        let refused = crate::ui::handle(&db, route);
        assert_eq!(
            (refused.status, refused.body.as_str()),
            ("500 Internal Server Error", served.as_str()),
            "{route}"
        );
    }
    // The browser draws what it was served, by the page's own code: the
    // run quarantined in the store's words and no view, a drilled
    // participant gone with it, and the participant's own read refused in
    // the same words; the run beside it draws its view.
    let quarantined = serde_json::json!({
        "painted": ["cleared"],
        "detail": [
            {"tag": "h1", "cls": null, "text": "r1", "children": []},
            {"tag": "p", "cls": null, "text": null, "children": [
                {"tag": "span", "cls": "badge", "text": "quarantined", "children": []},
            ]},
            {"tag": "p", "cls": "problem", "text": EDITED, "children": []},
        ],
        "transcript": [format!("unreadable: {EDITED}")],
        "active": null,
    });
    assert_eq!(drawn_in_the_page(&db, "r1"), quarantined);
    let beside = drawn_in_the_page(&db, "r2");
    assert_eq!(beside["painted"], serde_json::json!(["view r2"]));
    let missing = crate::ui::handle(&db, "/api/view/r3");
    assert_eq!(missing.body, r#"{"error":"r3 not found"}"#);
    // A run the journal does not hold is drawn in the server's words
    // alone, never quarantined.
    let not_found = serde_json::json!([
        {"tag": "h1", "cls": null, "text": "r3", "children": []},
        {"tag": "p", "cls": "problem", "text": "r3 not found", "children": []},
    ]);
    assert_eq!(drawn_in_the_page(&db, "r3")["detail"], not_found);
    assert!(Store::open_read_only(&db).unwrap().pages().unwrap());
    let quarantined = (false, Some(EDITED), Some(Quarantine::DoesNotLoad));
    let (drawn, row) = opened_in_the_console(&db);
    assert_eq!((drawn, row.detail.as_deref(), row.quarantine), quarantined);
    unmigrate(&rusqlite::Connection::open(&db).unwrap()).unwrap();
    let (drawn, row) = opened_in_the_console(&db);
    assert_eq!((drawn, row.detail.as_deref(), row.quarantine), quarantined);
    let beside = Store::open_read_only(&db).unwrap().load("r2").unwrap();
    assert_eq!(beside.len(), 2, "the run beside it reads");
}

/// Ruling 5 on the participant's read: a drilled participant whose first
/// presentation read never reached the server is drawn unreadable as the
/// browser found it, and when the interval's next read is the store's
/// refusal of its run, the pane is drawn again in the store's words.
#[test]
fn a_participant_refused_after_a_failed_read_is_drawn_in_the_store_words() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    edited_below_its_head(&db);
    let drilled = drill("r1");
    let steps = ["__unreachable = 1;", drilled.as_str(), "__timers[0]();"];
    let drawn = drawn_after(&db, "r1", &steps);
    assert_eq!(
        drawn["transcript"],
        serde_json::json!([
            "unreadable: the browser could not establish the shared presentation",
            format!("unreadable: {EDITED}"),
        ])
    );
}
