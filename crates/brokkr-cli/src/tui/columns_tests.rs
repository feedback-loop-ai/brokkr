//! The fleet in three columns (#503, second round), headless: the list
//! filling its height, the run dashboard and the live or findings column
//! beside it, `d`, `f` and `c`, focus and scrolling, and the shell asking
//! for the selected run's view.

use super::fleet_tests::{fleet_with_a_dead_run, under, DEAD, HELD, HELD_FEATURE, OLDER};
use super::tests::{drawn, frame_of, lines_of, script, test_ops, T0, TERMINAL};
use super::*;
use crate::tests::envelope_builder::EnvelopeBuilder;
use brokkr_core::fold::Status;
use brokkr_core::{EventEnvelope, EventType};
use ratatui::backend::TestBackend;
use serde_json::{json, Value};

/// The operator's terminal (#503): a 32" 4K screen.
pub(super) const OPERATOR: (u16, u16) = (375, 88);

/// The running run the fleet fixtures review, its second attempt live.
pub(super) const REVIEWING: &str = "fix-403-on-macos-round-9-3c1f9a02";

/// A shipped run whose review left a low residual.
pub(super) const SHIPPED: &str = "landing-4-of-the-fleet-2a3b4c5d";

/// One journal event: its type, its payload and when it was recorded.
type Step = (EventType, Value, &'static str);

/// The journal of `run`, one event per step, numbered in order.
fn journal_of(run: &str, steps: Vec<Vec<Step>>) -> Vec<EventEnvelope> {
    let steps = steps.into_iter().flatten().zip(1u64..);
    let events = steps.map(|((kind, payload, at), seq)| {
        EnvelopeBuilder::new(kind, payload)
            .run(run)
            .seq(seq)
            .event_id(format!("{run}-{seq}"))
            .at(at)
            .build()
    });
    events.collect()
}

/// A seat hired for one attempt: who, where, what it reported spending
/// and serving on, and when it started and ended.
struct Hired {
    effect: &'static str,
    seat: &'static str,
    phase: &'static str,
    spent: Value,
    span: (&'static str, &'static str),
}

/// `phase` entered at `at`, and `hired` concluding with `result`.
fn visit(phase: &'static str, hired: &Hired, result: Value) -> Vec<Step> {
    let (from, to) = hired.span;
    let effect = hired.effect;
    let mut finished = json!({"step": "claude-session-finished"});
    finished
        .as_object_mut()
        .unwrap()
        .extend(hired.spent.as_object().unwrap().clone());
    vec![
        (EventType::PhaseEntered, json!({"phase": phase}), from),
        (
            EventType::EffectRequested,
            json!({"effect_id": effect, "seat": hired.seat, "phase": hired.phase}),
            from,
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": effect, "attempt_id": "a1"}),
            from,
        ),
        (
            EventType::EffectCheckpointed,
            json!({"effect_id": effect, "attempt_id": "a1", "checkpoint": finished}),
            to,
        ),
        (
            EventType::EffectSucceeded,
            json!({"effect_id": effect, "attempt_id": "a1", "result": result}),
            to,
        ),
    ]
}

/// A ruling from `from` to `next`, its rule and anything else it says.
fn rule(rule: &str, from: &str, next: Option<&str>, more: Value, at: &'static str) -> Vec<Step> {
    let mut ruling = json!({"rule_id": rule, "from": from, "next": next, "severity": "normal"});
    ruling
        .as_object_mut()
        .unwrap()
        .extend(more.as_object().unwrap().clone());
    vec![(EventType::TransitionDecided, ruling, at)]
}

fn started(feature: &str) -> Vec<Step> {
    vec![(EventType::RunStarted, json!({"feature": feature}), T0)]
}

/// The security reviewer's findings on [`HELD`]: long enough to fill a
/// column, as a real review's are.
const SECURITY: &str = "Could not verify the audit the implementer cited. The exemption in \
deny.toml names the crate and the advisory, but the audit link in the commit message points at a \
gist that no longer resolves, and the licence the crate ships under (a custom one) is not on the \
allow list. Until someone reads that licence and records the reading beside the exemption, the \
exemption lets cargo deny pass a crate whose terms nobody here has read.\n\
Residual: high, security. Suggested: attach the audit under docs/audits/, name its reader, and \
scope the exemption to the exact version.";

const CORRECTNESS: &str = "The narrowing itself is right: the exemption now names one crate at \
one version instead of the wildcard it replaced, and the deny job fails on any other. One nit: \
the comment above the exemption still says 'temporary' without naming the issue that removes it.";

/// [`HELD`]'s journal: intake, implement and verify pass, and the review
/// panel parks it on a security residual nobody could verify.
pub(super) fn held_view() -> RunView {
    let seat = |effect, seat, phase, spent, span| Hired {
        effect,
        seat,
        phase,
        spent,
        span,
    };
    let opus = json!({"model": "claude-opus-5-5", "total_cost_usd": 0.42});
    let sol = json!({"model": "gpt-6.1-sol", "input_tokens": 812_000, "output_tokens": 41_000});
    let sonnet = json!({"model": "claude-sonnet-5-5", "total_cost_usd": 0.11});
    let fable = json!({"model": "claude-fable-5-1", "total_cost_usd": 1.25});
    let review = json!({"result": "clean", "notes": {
        "members": {"correctness": CORRECTNESS, "security": SECURITY},
        "verdicts": {"correctness": "clean", "security": "unverified"}}});
    let parked = json!({"inputs": {"max_residual_severity": "high"}, "result": "clean",
        "problem": "the reviewer could not verify the audit the implementer cited"});
    let steps = vec![
        started(HELD_FEATURE),
        visit(
            "intake",
            &seat("e1", "intake", "intake", opus, (T0, "2026-01-01T00:00:40Z")),
            json!({"result": "intook", "notes": "A chore: one exemption to narrow, one audit to cite."}),
        ),
        rule(
            "INTAKE-OK",
            "intake",
            Some("implement"),
            json!({}),
            "2026-01-01T00:00:40Z",
        ),
        visit(
            "implement",
            &seat(
                "e2",
                "implementer",
                "implement",
                sol,
                ("2026-01-01T00:00:40Z", "2026-01-01T00:03:10Z"),
            ),
            json!({"result": "complete", "notes": "Narrowed the exemption to one crate at one version."}),
        ),
        rule(
            "IMPL-OK",
            "implement",
            Some("verify"),
            json!({}),
            "2026-01-01T00:03:10Z",
        ),
        visit(
            "verify",
            &seat(
                "e3",
                "verifier",
                "verify",
                sonnet,
                ("2026-01-01T00:03:10Z", "2026-01-01T00:03:55Z"),
            ),
            json!({"result": "pass", "notes": "cargo deny check passes."}),
        ),
        rule(
            "VERIFY-PASS",
            "verify",
            Some("review"),
            json!({}),
            "2026-01-01T00:03:55Z",
        ),
        visit(
            "review",
            &seat(
                "e4",
                "reviewer",
                "review",
                fable,
                ("2026-01-01T00:03:55Z", "2026-01-01T00:06:30Z"),
            ),
            review,
        ),
        rule(
            "REVIEW-UNVERIFIED-SECURITY",
            "review",
            None,
            parked,
            "2026-01-01T00:06:30Z",
        ),
    ];
    brokkr_view::run_view(&journal_of(HELD, steps), None)
}

/// A seat turn of the live reviewer on [`REVIEWING`].
fn turn(turn: u64, tool: &str, target: &str, at: &'static str) -> Step {
    let checkpoint = json!({"step": "seat-turn", "turn": turn, "tool": tool, "target": target,
                            "model": "claude-fable-5-1"});
    let payload = json!({"effect_id": "e9", "attempt_id": "a9", "checkpoint": checkpoint});
    (EventType::EffectCheckpointed, payload, at)
}

/// [`REVIEWING`]'s journal: implement passed, the review's first attempt
/// timed out, and its second is at work.
pub(super) fn reviewing_steps() -> Vec<Vec<Step>> {
    let implementer = Hired {
        effect: "e1",
        seat: "implementer",
        phase: "implement",
        spent: json!({"model": "gpt-6.1-sol"}),
        span: (T0, "2026-01-01T00:02:00Z"),
    };
    let request = json!({"effect_id": "e9", "seat": "reviewer", "phase": "review"});
    let at = "2026-01-01T00:02:00Z";
    let review = vec![
        (EventType::PhaseEntered, json!({"phase": "review"}), at),
        (EventType::EffectRequested, request, at),
        (
            EventType::EffectStarted,
            json!({"effect_id": "e9", "attempt_id": "a8"}),
            at,
        ),
        (
            EventType::EffectFailed,
            json!({"effect_id": "e9", "attempt_id": "a8", "error": "the attempt timed out"}),
            "2026-01-01T00:03:00Z",
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "e9", "attempt_id": "a9"}),
            "2026-01-01T00:03:00Z",
        ),
        turn(
            1,
            "Read",
            "crates/brokkr-protocol/src/process/tests.rs",
            "2026-01-01T00:03:20Z",
        ),
        turn(
            2,
            "Grep",
            "crates/brokkr-protocol/src",
            "2026-01-01T00:03:40Z",
        ),
        turn(3, "Bash", "", "2026-01-01T00:04:10Z"),
        turn(
            4,
            "Read",
            "crates/brokkr-protocol/src/process.rs",
            "2026-01-01T00:04:30Z",
        ),
    ];
    vec![
        started("#403 macOS fix, round 9"),
        visit("implement", &implementer, json!({"result": "complete"})),
        rule("IMPL-OK", "implement", Some("review"), json!({}), at),
        review,
    ]
}

pub(super) fn reviewing_view() -> RunView {
    brokkr_view::run_view(&journal_of(REVIEWING, reviewing_steps()), None)
}

/// [`SHIPPED`]'s journal: implemented, reviewed with a low residual,
/// and shipped.
pub(super) fn shipped_view() -> RunView {
    let seat = |effect, seat, phase, span| Hired {
        effect,
        seat,
        phase,
        spent: json!({"model": "claude-opus-5-5", "total_cost_usd": 0.5}),
        span,
    };
    let notes =
        format!("Residual: low. {CORRECTNESS}\nNothing else stands in the way of the ship.");
    let low = json!({"inputs": {"max_residual_severity": "low"}, "severity": "flagged"});
    let steps = vec![
        started("Landing 4 of the fleet view"),
        visit(
            "implement",
            &seat(
                "e1",
                "implementer",
                "implement",
                (T0, "2026-01-01T00:01:00Z"),
            ),
            json!({"result": "complete"}),
        ),
        rule(
            "IMPL-OK",
            "implement",
            Some("review"),
            json!({}),
            "2026-01-01T00:01:00Z",
        ),
        visit(
            "review",
            &seat(
                "e2",
                "reviewer",
                "review",
                ("2026-01-01T00:01:00Z", "2026-01-01T00:04:00Z"),
            ),
            json!({"result": "residual", "notes": notes}),
        ),
        rule(
            "REVIEW-RESIDUAL-OK",
            "review",
            Some("ship"),
            low,
            "2026-01-01T00:04:00Z",
        ),
        visit(
            "ship",
            &seat(
                "e3",
                "shipper",
                "ship",
                ("2026-01-01T00:04:00Z", "2026-01-01T00:05:00Z"),
            ),
            json!({"result": "complete", "notes": "Merged."}),
        ),
        rule(
            "SHIP-COMPLETE",
            "ship",
            Some("done"),
            json!({}),
            "2026-01-01T00:05:00Z",
        ),
    ];
    brokkr_view::run_view(&journal_of(SHIPPED, steps), None)
}

/// The fleet the operator found, holding `view` as the shell's read of
/// the selected run.
pub(super) fn fleet_reading(view: RunView) -> Views {
    let mut views = fleet_with_a_dead_run();
    views.run = Some(view);
    views
}

/// The fleet at the operator's terminal with `run` selected, measured as
/// the shell measures it.
pub(super) fn selecting(run: &str) -> Tui {
    let mut tui = Tui::new(None);
    tui.cursor[0] = Some(run.to_string());
    (tui.width, tui.height) = OPERATOR;
    tui
}

/// The dashboard's lines for `run` as the operator's terminal wraps them.
fn dashboard_text(tui: &Tui, views: &Views, run: &str) -> Vec<String> {
    let row = views
        .runs
        .runs
        .iter()
        .find(|row| row.run_id == run)
        .unwrap();
    let width = width_of_column(tui, views, FleetColumn::Dashboard).unwrap();
    let lines = dashboard_lines(tui, views, row, width);
    lines.iter().map(Line::to_string).collect()
}

/// The live or findings column for `run`: its title and its lines.
fn live_text(views: &Views, run: &str, width: usize) -> (String, Vec<String>) {
    let row = views
        .runs
        .runs
        .iter()
        .find(|row| row.run_id == run)
        .unwrap();
    let (title, lines) = live_column(views, row, width);
    (title, lines.iter().map(Line::to_string).collect())
}

/// Every line after `heading`: the commission's, which ends the column.
fn after(lines: &[String], heading: &str) -> Vec<String> {
    let start = lines.iter().position(|line| line == heading).unwrap() + 1;
    lines[start..].to_vec()
}

use FleetColumn::{Dashboard, List, Live};

/// The three columns at the operator's width, about 35/35/30.
const THREE: [(FleetColumn, u16); 3] = [(List, 131), (Dashboard, 131), (Live, 113)];

/// `d` and `f` each show or hide their own column, so either, both or
/// neither stands beside the list, and whatever is shown shares the
/// whole frame: about 35/35/30 for three, 45/55 for two. Each is drawn
/// with its own title, and away from the fleet both are characters
/// nothing binds.
#[test]
fn d_and_f_each_toggle_their_column_and_what_shows_shares_the_whole_frame() {
    let views = fleet_reading(held_view());
    let mut tui = selecting(HELD);
    assert_eq!(fleet_columns(&tui, &views), THREE);
    let footer = footer_for(&tui, &views);
    assert!(
        footer.contains("· d hide dashboard · f hide findings ·"),
        "{footer}"
    );
    let steps = [
        (
            'd',
            vec![(List, 168), (Live, 207)],
            "· d dashboard · f hide findings ·",
        ),
        ('f', vec![(List, 375)], "· d dashboard · f findings ·"),
        (
            'd',
            vec![(List, 168), (Dashboard, 207)],
            "· d hide dashboard · f findings ·",
        ),
        (
            'f',
            THREE.to_vec(),
            "· d hide dashboard · f hide findings ·",
        ),
    ];
    for (key, wanted, said) in steps {
        apply(&mut tui, &views, Key::Char(key));
        assert_eq!(fleet_columns(&tui, &views), wanted, "after {key}");
        let footer = footer_for(&tui, &views);
        assert!(footer.contains(said), "after {key}: {footer}");
    }
    let (width, height) = OPERATOR;
    let frame = frame_of(&tui, &views, width, height);
    let top: Vec<char> = frame.lines().next().unwrap().chars().collect();
    let corners: Vec<usize> = (0..top.len()).filter(|at| top[*at] == '┌').collect();
    assert_eq!(corners, [0, 131, 262], "{frame}");
    let titles = ["┌runs─", "┌dashboard─", "┌findings · reviewer─"];
    for title in titles {
        assert!(frame.lines().next().unwrap().contains(title), "{title}");
    }
    let mut run = Tui::new(Some(HELD.to_string()));
    for key in ['d', 'f', 'c'] {
        apply(&mut run, &views, Key::Char(key));
    }
    assert_eq!((run.toggles, run.commission), (Toggles::default(), false));
}

/// At every width from the TUI's minimum, under every pair of toggles,
/// the columns drawn fill the frame exactly, the list beside another is
/// never narrower than a whole title, and each other column is drawn at
/// its useful width or not at all — the live column giving way first.
#[test]
fn every_width_shares_the_whole_frame_and_holds_each_column_it_draws() {
    assert_eq!((DASHBOARD_FROM, LIVE_FROM, THREE_FROM), (195, 185, 257));
    let pairs = [(true, true), (true, false), (false, true), (false, false)];
    for width in MIN_WIDTH..=600 {
        for (dashboard, live) in pairs {
            let columns = layout(Toggles { dashboard, live }, width);
            let shown: Vec<FleetColumn> = columns.iter().map(|(column, _)| *column).collect();
            let wanted = match (dashboard, live) {
                (true, true) if width >= THREE_FROM => vec![List, Dashboard, Live],
                (true, _) if width >= DASHBOARD_FROM => vec![List, Dashboard],
                (false, true) if width >= LIVE_FROM => vec![List, Live],
                _ => vec![List],
            };
            assert_eq!(shown, wanted, "{width} {dashboard} {live}");
            let total: u16 = columns.iter().map(|(_, width)| width).sum();
            assert_eq!(total, width, "{columns:?}");
            for (column, drawn) in &columns {
                let least = match (column, columns.len()) {
                    (List, 1) => width,
                    (List, _) => LIST_COLUMNS,
                    (Dashboard, _) => DASHBOARD_MIN,
                    (Live, _) => LIVE_MIN,
                };
                assert!(*drawn >= least, "{width}: {columns:?}");
            }
        }
    }
}

/// `Tab` cycles the focus through the columns on the frame, and the list
/// keys scroll the focused one: the dashboard's and the live column's
/// scrolls are their own, and moving the list reads the next run from
/// the top of each.
#[test]
fn tab_cycles_the_shown_columns_and_the_keys_scroll_the_focused_one() {
    let views = fleet_reading(held_view());
    let mut tui = selecting(HELD);
    let place = |tui: &Tui| (focused(tui, &views), tui.offset, tui.live_offset);
    assert_eq!(place(&tui), (List, 0, 0));
    apply(&mut tui, &views, Key::Tab);
    let footer = footer_for(&tui, &views);
    assert!(
        footer.starts_with("↑↓/jk scroll · Enter read notes · Tab findings ·"),
        "{footer}"
    );
    apply(&mut tui, &views, Key::Char('j'));
    apply(&mut tui, &views, Key::Down);
    assert_eq!(place(&tui), (Dashboard, 2, 0));
    apply(&mut tui, &views, Key::Tab);
    assert!(footer_for(&tui, &views).contains("· Tab list ·"));
    apply(&mut tui, &views, Key::PageDown);
    assert_eq!(place(&tui), (Live, 2, 10));
    apply(&mut tui, &views, Key::Char('G'));
    let (_, live) = live_text(&views, HELD, 111);
    assert_eq!(place(&tui), (Live, 2, live.len() - 1));
    apply(&mut tui, &views, Key::Char('g'));
    assert_eq!(place(&tui), (Live, 2, 0));
    let frame = frame_of(&tui, &views, OPERATOR.0, OPERATOR.1);
    // The dashboard keeps its own scroll: two lines down, past the title
    // and the id.
    assert!(frame.contains("││parked · review · 7m03s"), "{frame}");
    assert!(!frame.contains(&format!("││{HELD}  ")), "{frame}");
    apply(&mut tui, &views, Key::Tab);
    assert_eq!(
        tui.cursor[0].as_deref(),
        Some(HELD),
        "the columns moved no row"
    );
    apply(&mut tui, &views, Key::Char('j'));
    assert_eq!(place(&tui), (List, 0, 0));
    assert_eq!(tui.cursor[0].as_deref(), Some(REVIEWING));
}

/// The dashboard folds the commission to its first three lines and says
/// how many more `c` shows; `c` opens it in place and `c` again folds
/// it. A new selection reads its commission folded.
#[test]
fn c_opens_the_commission_in_place_and_folds_it_again() {
    let views = fleet_reading(held_view());
    let mut tui = selecting(HELD);
    let opening = "The review held the change on an unverified security residual: the exemption \
                   lets cargo deny pass a crate whose licence nobody";
    let folded = [
        "#362 cargo exemption",
        "",
        opening,
        "… 6 more lines · c shows them",
    ];
    assert_eq!(
        after(&dashboard_text(&tui, &views, HELD), "COMMISSION"),
        folded
    );
    assert!(footer_for(&tui, &views).contains("· c commission ·"));
    apply(&mut tui, &views, Key::Char('c'));
    let opened = after(&dashboard_text(&tui, &views, HELD), "COMMISSION");
    assert_eq!(opened.len(), 10, "{opened:#?}");
    assert_eq!(opened[..3], folded[..3]);
    assert_eq!(
        opened[7..],
        ["- cite the audit", "- name the follow-up", "c folds it"]
    );
    apply(&mut tui, &views, Key::Char('c'));
    assert_eq!(
        after(&dashboard_text(&tui, &views, HELD), "COMMISSION"),
        folded
    );
    apply(&mut tui, &views, Key::Char('c'));
    apply(&mut tui, &views, Key::Char('j'));
    assert!(
        !tui.commission,
        "a new selection reads its commission folded"
    );
}

/// More older runs than the frame holds: the fleet's hearth after a long
/// month, `count` finished runs beyond the two the fixture folds.
fn many_older(count: usize) -> Views {
    let shipped = super::fleet_tests::ruled(
        Status::Completed,
        "done",
        json!({"rule_id": "SHIP-COMPLETE", "from": "ship", "next": "done"}),
    );
    let ids: Vec<String> = (0..count)
        .map(|index| format!("an-older-run-{index:08}"))
        .collect();
    let entries: Vec<brokkr_view::RunEntry> = ids
        .iter()
        .map(|id| brokkr_view::RunEntry {
            run_id: id,
            feature: "An older run",
            created_at: OLDER,
            last_recorded_at: None,
            state: Some(&shipped),
            detail: None,
            residuals: &[],
        })
        .collect();
    let mut views = fleet_with_a_dead_run();
    views.runs.runs.extend(brokkr_view::run_rows(&entries).runs);
    views
}

/// The list fills its height (#503): below the runs that need the
/// operator, run, or ran today, it draws the older runs until it is
/// full, and folds only the rest into one line. Each one drawn is a row
/// `j` reaches; `a` still lists them all.
#[test]
fn the_list_fills_its_height_with_older_runs_and_folds_only_the_rest() {
    let views = many_older(30);
    let mut tui = Tui::new(None);
    tui.height = 30;
    let lines = lines_of(drawn(&tui, &views, 160, 30).backend().buffer());
    let older: Vec<&String> = lines
        .iter()
        .filter(|line| line.contains("An older run"))
        .collect();
    assert_eq!(older.len(), 9, "{}", lines.join("\n"));
    assert!(
        lines[26].contains("│21 more older runs: press a"),
        "{}",
        lines.join("\n")
    );
    assert_eq!(
        lines[27],
        format!("└{}┘", "─".repeat(158)),
        "the list is full"
    );
    // Eight runs in the sections above, and eleven of the 32 older ones.
    let listed = keys_for(&tui, &views);
    assert_eq!(listed.len(), 8 + 11);
    apply(&mut tui, &views, Key::Char('G'));
    assert_eq!(tui.cursor[0].as_deref(), Some("an-older-run-00000021"));
    tui.height = 88;
    let lines = lines_of(drawn(&tui, &views, 160, 88).backend().buffer());
    assert!(!lines.join("\n").contains("more older runs"), "all 32 fit");
    assert_eq!(keys_for(&tui, &views).len(), 8 + 32);
    tui.height = 30;
    apply(&mut tui, &views, Key::Char('a'));
    assert_eq!(keys_for(&tui, &views).len(), 8 + 32, "`a` lists them all");
}

/// What the dashboard answers for a run parked on a review: how it
/// stands, what it needs, the path it took, its seats and its notes'
/// first lines, each wrapped at the column's own width.
#[test]
fn the_dashboard_answers_how_it_stands_what_it_needs_its_path_and_seats() {
    let views = fleet_reading(held_view());
    let tui = selecting(HELD);
    let lines = dashboard_text(&tui, &views, HELD);
    assert_eq!(
        lines[..3],
        ["#362 cargo exemption", HELD, "parked · review · 7m03s"]
    );
    let needs = [
        "verdict   UNVERIFIED-SE… · residual high",
        "rule      REVIEW-UNVERIFIED-SECURITY · normal · review → —",
        "reason    the reviewer could not verify the audit the implementer cited",
        "seat      reviewer · succeeded · result clean",
        "notes     3 of 9 lines · Enter reads them all",
        "correctness (clean): The narrowing itself is right: the exemption now names one crate at \
         one version instead of the wildcard it",
        "replaced, and the deny job fails on any other. One nit: the comment above the exemption \
         still says 'temporary' without naming the",
        "issue that removes it.",
        "cargo-exemption-hold-5b6c7d8e seq 40 · review · max_residual_severity: high",
    ];
    assert_eq!(under(&lines, "WHAT IT NEEDS"), needs);
    let path = [
        "intake ✓ 40s (claude-opus-5-5) → implement ✓ 2m30s (gpt-6.1-sol) → verify ✓ 45s \
         (claude-sonnet-5-5) → review ● high 2m35s",
        "(claude-fable-5-1)",
    ];
    assert_eq!(under(&lines, "PATH"), path);
    let seats = [
        "intake · claude-opus-5-5 · 1 attempt · $0.4200",
        "implementer · gpt-6.1-sol · 1 attempt · 853k tok",
        "verifier · claude-sonnet-5-5 · 1 attempt · $0.1100",
        "reviewer · claude-fable-5-1 · 1 attempt · $1.2500",
    ];
    assert_eq!(under(&lines, "SEATS"), seats);
    assert_eq!(under(&lines, "WAY OUT"), ["—"]);
}

/// A running run's third column follows the seat the fold names at
/// work, newest first, and a newer checkpoint takes the top as it lands.
/// A view of another run is never painted as this one's, and a run with
/// no seat at work or no checkpoint yet says so.
#[test]
fn the_live_column_follows_the_working_seat_newest_first() {
    let views = fleet_reading(reviewing_view());
    let (title, lines) = live_text(&views, REVIEWING, 111);
    assert_eq!(title, "live · reviewer");
    let wanted = [
        "2026-01-01T00:04:30Z  reviewer · turn 4 · Read · crates/brokkr-protocol/src/process.rs",
        "2026-01-01T00:04:10Z  reviewer · turn 3 · Bash · —",
        "2026-01-01T00:03:40Z  reviewer · turn 2 · Grep · crates/brokkr-protocol/src",
        "2026-01-01T00:03:20Z  reviewer · turn 1 · Read · crates/brokkr-protocol/src/process/tests.rs",
    ];
    assert_eq!(lines, wanted);
    let mut steps = reviewing_steps();
    steps.push(vec![turn(
        5,
        "Edit",
        "crates/brokkr-protocol/src/process.rs",
        "2026-01-01T00:05:00Z",
    )]);
    let fresh = brokkr_view::run_view(&journal_of(REVIEWING, steps), None);
    let (_, lines) = live_text(&fleet_reading(fresh), REVIEWING, 111);
    assert_eq!(
        lines[0],
        "2026-01-01T00:05:00Z  reviewer · turn 5 · Edit · crates/brokkr-protocol/src/process.rs"
    );
    let mut quiet = reviewing_steps();
    quiet[3].truncate(5);
    let quiet = brokkr_view::run_view(&journal_of(REVIEWING, quiet), None);
    let (_, lines) = live_text(&fleet_reading(quiet), REVIEWING, 111);
    assert_eq!(lines, ["— no checkpoint from reviewer yet"]);
    let another = fleet_reading(held_view());
    let (_, lines) = live_text(&another, REVIEWING, 111);
    assert_eq!(lines, ["— the run's journal is not read"]);
    let (title, lines) = live_text(&another, "0065-rebuild-unit-20-1c2d3e4f", 111);
    assert_eq!(
        (title.as_str(), lines),
        (
            "live",
            vec!["— no seat at work: the fold names none".to_string()]
        )
    );
    let dashboard = dashboard_text(&selecting(REVIEWING), &views, REVIEWING);
    let stands = [
        "verdict   — · residual none",
        "rule      IMPL-OK · normal · implement → review",
    ];
    assert_eq!(under(&dashboard, "WHERE IT STANDS")[..2], stands);
    assert!(dashboard.contains(&"at work   seat reviewer · attempt 2".to_string()));
}

/// A finished run's third column: its last review's findings whole —
/// the residuals its ruling recorded, then the reviewer's notes — or
/// else its last seat's notes; and the absence marks where there are none.
#[test]
fn the_findings_column_shows_the_last_review_whole_or_else_the_last_seats_notes() {
    let views = fleet_reading(shipped_view());
    let (title, lines) = live_text(&views, SHIPPED, 111);
    assert_eq!(title, "findings · reviewer");
    assert_eq!(
        lines[..3],
        [
            "reviewer · succeeded · result residual",
            "landing-4-of-the-fleet-2a3b4c5d seq 40 · review · max_residual_severity: low",
            "",
        ]
    );
    assert_eq!(
        lines.last().unwrap(),
        "Nothing else stands in the way of the ship."
    );
    let dashboard = dashboard_text(&selecting(SHIPPED), &views, SHIPPED);
    assert_eq!(
        under(&dashboard, "WHY IT ENDED")[1..4],
        [
            "rule      SHIP-COMPLETE · normal · ship → done",
            "seat      shipper · succeeded · result complete",
            "notes",
        ]
    );
    let implemented =
        |steps| fleet_reading(brokkr_view::run_view(&journal_of(SHIPPED, steps), None));
    let only = Hired {
        effect: "e1",
        seat: "implementer",
        phase: "implement",
        spent: json!({}),
        span: (T0, "2026-01-01T00:01:00Z"),
    };
    let views = implemented(vec![
        started("a"),
        visit("implement", &only, json!({"result": "complete"})),
    ]);
    let (title, lines) = live_text(&views, SHIPPED, 111);
    assert_eq!(
        (title.as_str(), lines[2..].to_vec()),
        (
            "notes · implementer",
            vec!["— no notes recorded".to_string()]
        )
    );
    let dashboard = dashboard_text(&selecting(SHIPPED), &views, SHIPPED);
    assert!(
        dashboard.contains(&"notes     —".to_string()),
        "{dashboard:#?}"
    );
    let mut tui = selecting(SHIPPED);
    tui.pane = 1;
    assert!(
        footer_for(&tui, &views).contains("Enter open run"),
        "no notes to read"
    );
    let views = implemented(vec![started("a")]);
    let (_, lines) = live_text(&views, SHIPPED, 111);
    assert_eq!(lines, ["— no seat has concluded"]);
    assert!(
        footer_for(&tui, &views).contains("Enter open run"),
        "no seat to read"
    );
    let mut views = fleet_reading(shipped_view());
    views.run = None;
    assert_eq!(
        live_text(&views, SHIPPED, 111),
        (
            "findings".to_string(),
            vec!["— the run's journal is not read".to_string()]
        )
    );
}

/// `Enter` on the dashboard reads the last seat's notes whole in the
/// reader; on the list, and on the live column, it opens the run.
#[test]
fn enter_on_the_dashboard_reads_the_whole_notes() {
    let views = fleet_reading(held_view());
    let mut tui = selecting(HELD);
    apply(&mut tui, &views, Key::Tab);
    apply(&mut tui, &views, Key::Enter);
    let reading = tui.reading.clone().unwrap();
    assert!(
        reading.starts_with("reviewer · notes\n\ncorrectness (clean): The narrowing"),
        "{reading}"
    );
    assert!(
        reading.ends_with("scope the exemption to the exact version."),
        "{reading}"
    );
    assert_eq!(tui.level, Level::Runs);
    apply(&mut tui, &views, Key::Escape);
    apply(&mut tui, &views, Key::Tab);
    apply(&mut tui, &views, Key::Enter);
    assert_eq!((tui.level, tui.run.as_deref()), (Level::Run, Some(HELD)));
    let mut empty = Tui::new(None);
    apply(&mut empty, &Views::empty(), Key::Enter);
    assert_eq!(
        (empty.level, empty.run),
        (Level::Runs, None),
        "nothing to open"
    );
}

/// At the fleet the shell asks for the selected run's view while a
/// column beside the list shows it, and for no run while none does —
/// the question the run level asks, through the same source.
#[test]
fn the_shell_reads_the_selected_runs_view_while_a_column_shows_it() {
    let asked_at = |width: u16| {
        let _serialized = TERMINAL.lock().unwrap_or_else(|error| error.into_inner());
        let mut terminal = Terminal::new(TestBackend::new(width, 40)).unwrap();
        // `r` forces the second frame, which reads the seeded selection.
        script(&[Key::Char('r'), Key::Down, Key::Quit]);
        let mut asked: Vec<Option<String>> = Vec::new();
        let mut source = |ask: Ask| {
            asked.push(ask.run.map(str::to_string));
            Ok(Some(fleet_with_a_dead_run()))
        };
        let mut tui = Tui::new(None);
        drive(&mut terminal, &test_ops(), &mut source, &mut tui, 5).unwrap();
        (asked, tui.height)
    };
    let (asked, height) = asked_at(OPERATOR.0);
    let broke = Some("journal-that-broke-7f8e9d0c".to_string());
    assert_eq!(asked, [None, Some(DEAD.to_string()), broke]);
    assert_eq!(height, 40, "measured with the width");
    assert_eq!(asked_at(100).0, [None, None, None]);
}

/// The toggles and the open commission are a hearth's own, parked with
/// its selection and returned by a switch back (decision 0026 ruling 2).
#[test]
fn the_columns_asked_for_are_a_hearths_own() {
    let views = fleet_reading(held_view());
    let mut tui = Tui::over(None, vec!["alpha".to_string(), "beta".to_string()], 0);
    apply(&mut tui, &views, Key::Char('f'));
    apply(&mut tui, &views, Key::Char('c'));
    tui.live_offset = 4;
    switch(&mut tui, 1);
    assert_eq!(
        (tui.toggles, tui.commission, tui.live_offset),
        (Toggles::default(), false, 0)
    );
    switch(&mut tui, 0);
    let asked = Toggles {
        dashboard: true,
        live: false,
    };
    assert_eq!((tui.toggles, tui.commission), (asked, true));
}
