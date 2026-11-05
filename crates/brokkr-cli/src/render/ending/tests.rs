//! Decision 0064 ruling 6's painting of `brokkr runs`: the golden listing
//! of every ending beside a stopping, a quarantined, a parked, a running
//! and a cooling-down row, with colour and without; the header's counts
//! and the closing tally; the filters' groups; and a hostile citation
//! through `Safe`.

use brokkr_core::ending::{Citation, Closing, Ended, Ending, Tone};
use brokkr_core::fold::{Cursor, Run, RunState, Status};
use brokkr_view::{RunEntry, RunRow, RunsView};
use serde_json::json;

use super::{cited, code, group, painted, Group, Shade, Tally};
use crate::fleet::Filter;
use crate::render::{runs, Style};
use crate::tests::run_state::{run_state, standing};

const T0: &str = "2026-01-01T00:00:00Z";
const NOW: &str = "2026-01-01T00:07:03Z";

/// The ruling a policy ending's run was stopped by, from ruling 1's table.
fn ruled(ending: Ending) -> serde_json::Value {
    let (rule, from, result) = match ending {
        Ending::Refused => ("REVIEW-SECURITY-HOLD", "review", "hold"),
        Ending::Failed => ("VERIFY-FAIL", "verify", "fail"),
        _ => ("IMPL-BLOCKED", "implement", "blocked"),
    };
    json!({"rule_id": rule, "from": from, "next": "stop", "result": result})
}

/// A run that ended as `ending`: from the policy table's stop phase for
/// a policy ending, shipped from `done`, else stopped by an operator in
/// `implement`.
fn ended_state(ending: Ending) -> RunState {
    let mut state = run_state("r", 14, standing(ending.status(), ""));
    let (phase, decision) = match ending {
        Ending::Refused | Ending::Failed | Ending::Blocked => ("stop", Some(ruled(ending))),
        Ending::Shipped | Ending::ShippedFlagged => ("done", None),
        _ => ("implement", None),
    };
    state.phase = Some(phase.to_string());
    state.last_decision = decision.map(|it| serde_json::from_value(it).unwrap());
    state
}

/// What `ending` cites: the run that superseded it, in its realm, or the
/// pull request it landed by.
fn citation(ending: Ending) -> Option<Citation> {
    match ending {
        Ending::Superseded => Some(Citation::Run {
            realm: Some("brokkr".to_string()),
            run_id: "run-next".to_string(),
        }),
        Ending::DeliveredElsewhere => Some(Citation::Ref("#1014".to_string())),
        _ => None,
    }
}

/// One listed row of `run_id` folded to `state`, ended as `closing`.
fn row(run_id: &str, state: Option<&RunState>, closing: Option<Closing>) -> RunRow {
    let refusal = brokkr_view::Quarantine::DoesNotFold.in_words("event 3: impossible");
    let entry = RunEntry {
        run_id,
        feature: "f",
        created_at: T0,
        last_recorded_at: None,
        state,
        detail: state.is_none().then_some(refusal),
        residuals: &[],
        closing,
    };
    brokkr_view::run_rows(&[entry]).runs.remove(0)
}

/// A row that ended as `ending`, citing what it cites.
pub(crate) fn ended_row(ending: Ending) -> RunRow {
    let state = ended_state(ending);
    let closing = Closing {
        ended: Ended {
            ending,
            cited: citation(ending),
        },
        stood_over: false,
    };
    row(
        &format!("run-{}", ending.word()),
        Some(&state),
        Some(closing),
    )
}

/// Every ending's row in ruling 2's order, then the live rows ruling 6
/// and the addendum's choice 2 (a) paint: a parked run, a stopping one, an
/// unanswered quarantine, a running run and one cooling down between
/// attempts.
pub(crate) fn every_row() -> Vec<RunRow> {
    let mut rows: Vec<RunRow> = Ending::ALL.into_iter().map(ended_row).collect();
    let parked = run_state("r", 14, standing(Status::AwaitingOperator, "held"));
    let mut stopping = run_state("r", 14, Run::Running(Cursor::Stop));
    stopping.stop_accepted_seq = Some(9);
    let running = run_state("r", 14, standing(Status::Running, ""));
    let cooling = run_state(
        "r",
        14,
        Run::Running(Cursor::ExecuteEffect {
            effect_id: "e".to_string(),
            seat: "smith".to_string(),
            failed_attempts: 1,
        }),
    );
    rows.push(row("run-parked", Some(&parked), None));
    rows.push(row("run-stopping", Some(&stopping), None));
    rows.push(row("run-quarantined", None, None));
    rows.push(row("run-running", Some(&running), None));
    rows.push(row("run-cooling", Some(&cooling), None));
    rows
}

/// [`every_row`] as `brokkr runs` paints it in `style`.
fn listed(style: &Style) -> String {
    runs(&RunsView::of(every_row()), NOW, style)
}

/// Ruling 6's golden listing under `NO_COLOR`: the gutter alone carries
/// what each row waits on, a policy ending names its rule under itself and
/// a cited one its citation.
#[test]
fn every_ending_lists_with_its_gutter_and_detail_under_no_color() {
    assert_eq!(listed(&Style::plain(100)), PLAIN);
}

/// The same listing with colour: each status cell in its ending's tone,
/// a parked row yellow, a stopping row and an unanswered quarantine cyan,
/// a live row bold, and every detail line dim.
#[test]
fn every_ending_lists_in_its_tone_with_colour() {
    let coloured = Style {
        color: true,
        width: 100,
    };
    assert_eq!(listed(&coloured), COLOURED);
}

/// The shade and the mark are the ending's own total functions for an
/// ended row, never a second table: a row's paint moves with
/// `Ending::tone` and `Ending::gutter`.
#[test]
fn an_ended_row_is_painted_by_its_endings_tone_and_gutter() {
    for ending in Ending::ALL {
        let expected = (Shade::Toned(ending.tone()), ending.gutter());
        assert_eq!(painted(&ended_row(ending)), expected, "{}", ending.word());
    }
    let shades: Vec<&str> = [Tone::Green, Tone::Yellow, Tone::Cyan, Tone::Red, Tone::Dim]
        .map(Shade::Toned)
        .into_iter()
        .chain([Shade::Live])
        .map(code)
        .collect();
    let expected = [
        "\x1b[32m", "\x1b[33m", "\x1b[36m", "\x1b[31m", "\x1b[2m", "\x1b[1m",
    ];
    assert_eq!(shades, expected);
}

/// The header's two counts and the closing tally, by exact text: a park
/// and every `refused` and `exhausted` run want a ruling, and a stopping
/// run, an unanswered quarantine, `blocked` and `infrastructure` an action.
#[test]
fn the_header_counts_the_two_wants_and_the_tally_counts_each_ending() {
    let tally = super::super::tests::tally_of(&every_row());
    assert_eq!(
        tally.counts(),
        "17 runs · 3 want a ruling · 4 want an action"
    );
    assert_eq!(
        tally.closing(),
        "endings  1 shipped · 1 shipped-flagged · 1 refused · 1 failed · 1 blocked · \
         1 exhausted · 1 superseded · 1 infrastructure · 1 delivered-elsewhere · 1 withdrawn · \
         1 operator-stopped · 1 unclassified\n"
    );
    // Choice 6 (a): a refused run wants a ruling by its ending alone, its
    // residual findings closed or not.
    let mut closed = ended_row(Ending::Refused);
    closed.verdict.residual = None;
    closed.residuals.clear();
    let mut one = Tally::default();
    one.count(&closed);
    one.count(&ended_row(Ending::Refused));
    assert_eq!(one.counts(), "2 runs · 2 want a ruling · 0 want an action");
    assert_eq!(one.closing(), "endings  2 refused\n");
    let mut single = Tally::default();
    single.count(&ended_row(Ending::Blocked));
    assert_eq!(
        single.counts(),
        "1 run · 0 want a ruling · 1 wants an action"
    );
    assert_eq!(Tally::default().closing(), "", "nothing ended, no tally");
}

/// The console's groups and the filters' reading of them: the `!` and
/// `>` rows want the operator, the bold ones are live, and every other
/// row is settled. No filter named keeps every row.
#[test]
fn each_row_falls_in_one_group_and_the_filters_read_it() {
    let rows = every_row();
    let kept = |filter: Filter| -> Vec<&str> {
        let rows = rows.iter().filter(|row| filter.keeps(row));
        rows.map(|row| row.run_id.as_str()).collect()
    };
    let every: Vec<&str> = rows.iter().map(|row| row.run_id.as_str()).collect();
    assert_eq!(
        kept(Filter::default()),
        every,
        "no listing hides a run unasked"
    );
    let wanting = [
        "run-refused",
        "run-blocked",
        "run-exhausted",
        "run-infrastructure",
        "run-parked",
        "run-stopping",
        "run-quarantined",
    ];
    let needs_operator = Filter {
        needs_operator: true,
        ..Filter::default()
    };
    assert_eq!(kept(needs_operator), wanting);
    let unsettled = Filter {
        hide_settled: true,
        ..Filter::default()
    };
    let mut live = wanting.to_vec();
    live.extend(["run-running", "run-cooling"]);
    let mut shown = kept(unsettled);
    shown.sort_unstable();
    live.sort_unstable();
    assert_eq!(shown, live);
    let refused = Filter {
        ending: Some(Ending::Refused),
        ..Filter::default()
    };
    assert_eq!(kept(refused), ["run-refused"]);
    let groups: Vec<Group> = ["run-shipped", "run-parked", "run-cooling"]
        .map(|id| group(rows.iter().find(|row| row.run_id == id).unwrap()))
        .into();
    assert_eq!(groups, [Group::Settled, Group::WantsYou, Group::Live]);
}

/// An ending's citation is journal text: control bytes and escape
/// sequences in it reach the terminal through `Safe`, inert, on the
/// listing's detail line and in what a citation reads.
#[test]
fn a_hostile_citation_is_painted_through_safe() {
    let mut hostile = ended_row(Ending::Superseded);
    hostile.ending_cited = Some(Citation::Run {
        realm: Some("re\x1b]0;pwned\x07alm".to_string()),
        run_id: "run\r\x1b[2J\u{202E}next".to_string(),
    });
    let out = runs(&RunsView::of(vec![hostile]), NOW, &Style::plain(100));
    assert_eq!(
        out,
        "  run-superseded superseded implement seq 14 7m03s f\n      \
         by re]0;pwnedalm/run[2Jnext\n"
    );
    let workspace = Citation::Run {
        realm: None,
        run_id: "run-next".to_string(),
    };
    assert_eq!(cited(&workspace), "by run-next", "a one-hearth citation");
    let reference = Citation::Ref("#1\x1b[31m".to_string());
    assert_eq!(
        cited(&reference),
        "by #1\x1b[31m",
        "sanitized where painted"
    );
    let delivered = RunRow {
        ending_cited: Some(reference),
        ..ended_row(Ending::DeliveredElsewhere)
    };
    let out = runs(&RunsView::of(vec![delivered]), NOW, &Style::plain(100));
    assert!(out.ends_with("      by #1[31m\n"), "{out:?}");
}

/// [`every_row`] under `NO_COLOR`, 100 columns wide.
const PLAIN: &str = "  run-shipped             shipped             done      seq 14 7m03s f
~ run-shipped-flagged     shipped-flagged     done      seq 14 7m03s f
! run-refused             refused             stop      seq 14 7m03s f
      REVIEW-SECURITY-HOLD
  run-failed              failed              stop      seq 14 7m03s f
      VERIFY-FAIL
> run-blocked             blocked             stop      seq 14 7m03s f
      IMPL-BLOCKED
! run-exhausted           exhausted           implement seq 14 7m03s f
  run-superseded          superseded          implement seq 14 7m03s f
      by brokkr/run-next
> run-infrastructure      infrastructure      implement seq 14 7m03s f
  run-delivered-elsewhere delivered-elsewhere implement seq 14 7m03s f
      by #1014
  run-withdrawn           withdrawn           implement seq 14 7m03s f
  run-operator-stopped    operator-stopped    implement seq 14 7m03s f
? run-unclassified        unclassified        implement seq 14 7m03s f
! run-parked              awaiting_operator   -         seq 14 7m03s f
> run-stopping            stopping            -         seq 14 7m03s f
    stop accepted at seq 9; no ending written. If no driver is running it, `brokkr conclude` closes it.
> run-quarantined         ?                   -         seq -  7m03s f
    fold  event 3: impossible
  run-running             running             -         seq 14 7m03s f
  run-cooling             running             -         seq 14 7m03s f
";

/// [`every_row`] with colour, 100 columns wide.
const COLOURED: &str = "  run-shipped             \x1b[32mshipped            \x1b[0m done      seq 14 7m03s f
~ run-shipped-flagged     \x1b[32mshipped-flagged    \x1b[0m done      seq 14 7m03s f
! run-refused             \x1b[33mrefused            \x1b[0m stop      seq 14 7m03s f
      \x1b[2mREVIEW-SECURITY-HOLD\x1b[0m
  run-failed              \x1b[31mfailed             \x1b[0m stop      seq 14 7m03s f
      \x1b[2mVERIFY-FAIL\x1b[0m
> run-blocked             \x1b[36mblocked            \x1b[0m stop      seq 14 7m03s f
      \x1b[2mIMPL-BLOCKED\x1b[0m
! run-exhausted           \x1b[33mexhausted          \x1b[0m implement seq 14 7m03s f
  run-superseded          \x1b[2msuperseded         \x1b[0m implement seq 14 7m03s f
      \x1b[2mby brokkr/run-next\x1b[0m
> run-infrastructure      \x1b[36minfrastructure     \x1b[0m implement seq 14 7m03s f
  run-delivered-elsewhere \x1b[2mdelivered-elsewhere\x1b[0m implement seq 14 7m03s f
      \x1b[2mby #1014\x1b[0m
  run-withdrawn           \x1b[2mwithdrawn          \x1b[0m implement seq 14 7m03s f
  run-operator-stopped    \x1b[2moperator-stopped   \x1b[0m implement seq 14 7m03s f
? run-unclassified        \x1b[2munclassified       \x1b[0m implement seq 14 7m03s f
! run-parked              \x1b[33mawaiting_operator  \x1b[0m -         seq 14 7m03s f
> run-stopping            \x1b[36mstopping           \x1b[0m -         seq 14 7m03s f
    stop accepted at seq 9; no ending written. If no driver is running it, `brokkr conclude` closes it.
> run-quarantined         \x1b[36m?                  \x1b[0m -         seq -  7m03s f
    fold  event 3: impossible
  run-running             \x1b[1mrunning            \x1b[0m -         seq 14 7m03s f
  run-cooling             \x1b[1mrunning            \x1b[0m -         seq 14 7m03s f
";
