use super::*;
use brokkr_core::fold::{Cursor, RunState, Status};
use brokkr_core::{EventEnvelope, EventType};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const T0: &str = "2026-01-01T00:00:00Z";
const T1: &str = "2026-01-01T00:00:05Z";
const T2: &str = "2026-01-01T00:02:03Z";
const NOW: &str = "2026-01-01T00:07:03Z";

fn ev(seq: u64, event_type: EventType, payload: Value, at: &str) -> EventEnvelope {
    EventEnvelope {
        run_id: "run-7".to_string(),
        seq,
        event_id: format!("ev{seq}"),
        event_schema_version: 1,
        event_type,
        payload,
        causation_id: None,
        correlation_id: "corr".to_string(),
        attempt_id: None,
        recorded_at: at.to_string(),
        previous_hash: String::new(),
        event_hash: String::new(),
    }
}

fn state(status: Status, park: Option<&str>, decision: Option<Value>) -> RunState {
    RunState {
        run_id: "run-7".to_string(),
        seq: 14,
        last_hash: "hash".to_string(),
        status,
        phase: Some("design".to_string()),
        cursor: Cursor::Idle,
        consecutive_failures: BTreeMap::new(),
        visits: BTreeMap::new(),
        last_result: None,
        reviewed_heads: None,
        last_decision: decision,
        park_reason: park.map(str::to_string),
        feature: Some("one derivation, two surfaces".to_string()),
        pending_command: None,
        riding_stop: false,
    }
}

/// An intake seat that concluded, then a design sequence with a parallel
/// step still running: a fork, a sequential step, a ruling and a live
/// seat, all in one run.
fn journal() -> Vec<EventEnvelope> {
    vec![
        ev(
            1,
            EventType::RunStarted,
            json!({"feature": "one derivation, two surfaces"}),
            T0,
        ),
        ev(2, EventType::PhaseEntered, json!({"phase": "intake"}), T0),
        ev(
            3,
            EventType::EffectRequested,
            json!({"effect_id": "eff-i", "seat": "intake", "phase": "intake"}),
            T0,
        ),
        ev(
            4,
            EventType::EffectStarted,
            json!({"effect_id": "eff-i", "attempt_id": "att1"}),
            T0,
        ),
        ev(
            5,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff-i", "attempt_id": "att1",
                   "checkpoint": {"step": "seat-turn", "turn": 3, "tool": "Read",
                                  "target": "docs/decisions/0013.md"}}),
            T0,
        ),
        ev(
            6,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff-i", "attempt_id": "att1",
                   "checkpoint": {"step": "claude-session-finished",
                                  "session_id": "sess-a", "total_cost_usd": 0.03125}}),
            T1,
        ),
        ev(
            7,
            EventType::EffectSucceeded,
            json!({"effect_id": "eff-i", "attempt_id": "att1",
                   "result": {"result": "intook"}}),
            T2,
        ),
        ev(
            8,
            EventType::TransitionDecided,
            json!({"rule_id": "INTAKE-OK", "severity": "normal", "from": "intake",
                   "next": "design", "result": "intook"}),
            T2,
        ),
        ev(9, EventType::PhaseEntered, json!({"phase": "design"}), T2),
        ev(
            10,
            EventType::EffectRequested,
            json!({"effect_id": "eff-d", "seat": "design", "phase": "design"}),
            T2,
        ),
        ev(
            11,
            EventType::EffectStarted,
            json!({"effect_id": "eff-d", "attempt_id": "att2"}),
            T2,
        ),
        ev(
            12,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff-d", "attempt_id": "att2",
                   "checkpoint": {"member": "positions:simplicity",
                                  "step": "panel-member-finished", "outcome": "succeeded"}}),
            T2,
        ),
        ev(
            13,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff-d", "attempt_id": "att2",
                   "checkpoint": {"member": "positions:robustness",
                                  "step": "panel-member-finished", "outcome": "succeeded"}}),
            T2,
        ),
        ev(
            14,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff-d", "attempt_id": "att2",
                   "checkpoint": {"step": "sequence-step-finished",
                                  "step_name": "positions"}}),
            T2,
        ),
        ev(
            15,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff-d", "attempt_id": "att2",
                   "checkpoint": {"member": "chief", "step": "seat-turn", "turn": 2,
                                  "tool": "Write"}}),
            T2,
        ),
    ]
}

fn view(park: Option<&str>) -> brokkr_view::RunView {
    let events = journal();
    let ruled = state(
        Status::Running,
        park,
        Some(
            json!({"rule_id": "INTAKE-OK", "severity": "normal", "from": "intake",
                    "next": "design", "result": "intook"}),
        ),
    );
    brokkr_view::run_view(&events, Some(&ruled))
}

fn runs_view() -> brokkr_view::RunsView {
    let older = state(Status::Completed, None, None);
    let newer = state(Status::Running, None, None);
    let entries = [
        brokkr_view::RunEntry {
            run_id: "run-old",
            feature: "an older feature whose text runs well past any terminal width \
                      and must be clamped rather than wrapped",
            created_at: T0,
            state: Some(&older),
            detail: None,
        },
        brokkr_view::RunEntry {
            run_id: "run-7",
            feature: "one derivation, two surfaces",
            created_at: T1,
            state: Some(&newer),
            detail: None,
        },
    ];
    brokkr_view::run_rows(&entries)
}

// ------------------------------- many hearths (0026 rulings 3 and 5)

/// A many-hearth world's fleet: one realm with runs, one whose journal
/// would not open. Each section aligns its own columns.
fn fleet_view() -> brokkr_view::FleetView {
    let running = state(Status::Running, None, None);
    let alpha = [brokkr_view::RunEntry {
        run_id: "run-a",
        feature: "alpha's work",
        created_at: T1,
        state: Some(&running),
        detail: None,
    }];
    let beta = [
        brokkr_view::RunEntry {
            run_id: "run-b1",
            feature: "beta's older work",
            created_at: T0,
            state: Some(&running),
            detail: None,
        },
        brokkr_view::RunEntry {
            run_id: "run-b2",
            feature: "beta's newer work",
            created_at: T1,
            state: Some(&running),
            detail: None,
        },
    ];
    brokkr_view::fleet_rows(&[
        brokkr_view::HearthEntries {
            realm: "alpha",
            journal: "a/forge.db",
            entries: &alpha,
            detail: None,
        },
        brokkr_view::HearthEntries {
            realm: "beta",
            journal: "b/forge.db",
            entries: &beta,
            detail: None,
        },
        brokkr_view::HearthEntries {
            realm: "gamma",
            journal: "c/forge.db",
            entries: &[],
            detail: Some("unable to open database file"),
        },
    ])
}

/// Grouped by realm, not interleaved: each hearth under its own heading,
/// naming the journal it was read from and how many runs it holds.
#[test]
fn a_many_hearth_fleet_lists_each_realm_under_its_own_heading() {
    let out = fleet(&fleet_view(), NOW, &Style::plain(80));
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "alpha · 1 run · a/forge.db");
    assert!(lines[1].starts_with("run-a "), "{out}");
    assert_eq!(lines[2], "", "a blank line parts the hearths");
    assert_eq!(lines[3], "beta · 2 runs · b/forge.db");
    // Newest first inside a hearth, and no run crosses into another's
    // section (ruling 5): nothing is folded across journals.
    assert!(lines[4].starts_with("run-b2 "), "{out}");
    assert!(lines[5].starts_with("run-b1 "), "{out}");
    assert_eq!(lines[7], "gamma · 0 runs · c/forge.db");
    assert_eq!(lines[8], "  journal  unable to open database file");
    assert_eq!(lines.len(), 9, "{out}");
}

/// One hearth's section is the SAME text `brokkr runs` prints for that
/// journal on its own — the heading is the only thing grouping adds.
#[test]
fn a_hearths_section_is_the_listing_runs_already_prints() {
    let one = brokkr_view::fleet_rows(&[brokkr_view::HearthEntries {
        realm: "brokkr",
        journal: "j.db",
        entries: &[],
        detail: None,
    }]);
    assert_eq!(
        fleet(&one, NOW, &Style::plain(80)),
        "brokkr · 0 runs · j.db\n"
    );
    let flat = runs(&runs_view(), NOW, &Style::plain(80));
    let grouped = fleet(&fleet_view(), NOW, &Style::plain(80));
    assert!(!flat.is_empty() && !grouped.is_empty());
}

/// A realm name and a journal path are operator-written, and they still
/// cross the one sanitizer every journal string does.
#[test]
fn a_hostile_realm_name_or_journal_path_renders_as_inert_text() {
    let view = brokkr_view::fleet_rows(&[
        brokkr_view::HearthEntries {
            realm: "a\u{1b}[2Jb",
            journal: "j\rforged.db",
            entries: &[],
            detail: Some("boom\u{1b}[2J"),
        },
        brokkr_view::HearthEntries {
            realm: "ok",
            journal: "j.db",
            entries: &[],
            detail: None,
        },
    ]);
    let out = fleet(&view, NOW, &Style::plain(80));
    assert!(!out.contains('\u{1b}'), "{out:?}");
    assert!(!out.contains('\r'), "{out:?}");
    assert!(out.contains("a[2Jb"), "{out}");
    assert!(out.contains("jforged.db"), "{out}");
}

// ------------------------------------------------------------- AC-22

#[test]
fn control_characters_never_reach_the_terminal() {
    // The journal is seat-authored: a \r plus spaces overwrites the line
    // above, and \x1b]0; retitles the operator's terminal. A hostile
    // result token could otherwise forge a ruling line, continuously,
    // under `watch`.
    let hostile = "ok\r\x1b[2Jforged\x1b]0;pwned\x07";
    let safe = Safe::new(hostile);
    assert_eq!(safe.as_str(), "ok[2Jforged]0;pwned");
    assert!(!safe.as_str().contains('\x1b'));
    assert!(!safe.as_str().contains('\r'));
    assert!(!safe.as_str().contains('\x07'));
    // Width arithmetic runs on the SANITIZED text, so an escape
    // sequence cannot smuggle invisible columns.
    assert_eq!(safe.width(), 19);
    assert_eq!(Safe::new("abc").padded(6), "abc   ");
    assert_eq!(Safe::new("abcdef").padded(3), "abcdef", "never truncates");
    // C1 and DEL go too.
    assert_eq!(Safe::new("a\u{7f}b\u{9b}c").as_str(), "abc");
}

/// `char::is_control()` does not cover the bidi and zero-width
/// formatting characters, and those **visually reorder the rest of a
/// rendered line** — a hostile seat label can forge a ruling line
/// without a single escape byte. One sanitizer serves all three
/// surfaces, so the hardening lands here (decision 0014).
#[test]
fn bidi_and_zero_width_formatting_characters_never_reach_the_terminal() {
    let hostile = "seat\u{202E}gnippots\u{202C} \u{200B}\u{FEFF}label";
    let safe = Safe::new(hostile);
    assert_eq!(safe.as_str(), "seatgnippots label", "in source order");
    assert_eq!(safe.width(), 18, "width is computed on the sanitized text");
    // Every enumerated range, at both of its ends and inside.
    for stripped in [
        '\u{200B}', '\u{200D}', '\u{200F}', '\u{202A}', '\u{202C}', '\u{202E}', '\u{2060}',
        '\u{2062}', '\u{2064}', '\u{2066}', '\u{2068}', '\u{2069}', '\u{FEFF}',
    ] {
        assert!(reorders(stripped), "{stripped:?} reorders a line");
        assert_eq!(Safe::new(&format!("a{stripped}b")).as_str(), "ab");
    }
    // And the characters on the other side of every boundary stay: this
    // is an enumerated list, not a blanket sweep of the formatting
    // planes, so ordinary text is untouched.
    for kept in [
        '\u{200A}', '\u{2010}', '\u{2029}', '\u{202F}', '\u{205F}', '\u{2065}', '\u{206A}',
        '\u{2070}', '\u{FEFE}', '\u{FF00}', 'a', 'é', '中',
    ] {
        assert!(!reorders(kept), "{kept:?} is ordinary text");
        assert_eq!(Safe::new(&format!("a{kept}b")).width(), 3);
    }
}

#[test]
fn a_hostile_feature_and_result_token_render_as_inert_text() {
    let hostile = state(Status::Running, None, None);
    let entries = [brokkr_view::RunEntry {
        run_id: "r\x1b[31m1",
        feature: "feature\rforged",
        created_at: T0,
        state: Some(&hostile),
        detail: None,
    }];
    let rows = brokkr_view::run_rows(&entries);
    let out = runs(&rows, NOW, &Style::plain(80));
    assert!(!out.contains('\x1b'), "{out:?}");
    assert!(!out.contains('\r'), "{out:?}");
    assert!(out.contains("r[31m1"));
    assert!(out.contains("featureforged"));
}

// ------------------------------------------------------------- AC-23

#[test]
fn width_and_colour_come_from_the_environment_through_pure_rules() {
    assert_eq!(width_from(None), 80);
    assert_eq!(width_from(Some("")), 80);
    assert_eq!(width_from(Some("abc")), 80);
    assert_eq!(width_from(Some("0")), 20, "clamped up");
    assert_eq!(width_from(Some("100000")), 1000, "clamped down");
    assert_eq!(width_from(Some("120")), 120);

    assert!(color_enabled(true, false, false));
    assert!(!color_enabled(false, false, false), "not a terminal");
    assert!(!color_enabled(true, true, false), "NO_COLOR");
    assert!(!color_enabled(true, false, true), "TERM=dumb");

    assert_eq!(status_code("completed"), GREEN);
    assert_eq!(status_code("succeeded"), GREEN);
    assert_eq!(status_code("stopped"), RED);
    assert_eq!(status_code("failed"), RED);
    assert_eq!(status_code("running"), BOLD);
    assert_eq!(status_code("working"), BOLD);
    assert_eq!(status_code("indeterminate"), DIM);
    // One classification, expressed as ANSI here and as a ratatui style
    // in the TUI: `status_code` is `tone` plus a table (decision 0014).
    assert!(matches!(tone("completed"), Tone::Good));
    assert!(matches!(tone("stopped"), Tone::Bad));
    assert!(matches!(tone("working"), Tone::Live));
    assert!(
        matches!(tone("indeterminate"), Tone::Quiet),
        "an unknown status is never guessed into one of the four"
    );

    // Colour is a post-processing wrap of an already-rendered plain
    // string: the goldens all run plain and this proves the wrapping.
    let coloured = Style {
        color: true,
        width: 100,
    };
    assert_eq!(tint("ok", GREEN, &coloured), "\x1b[32mok\x1b[0m");
    assert_eq!(tint("ok", GREEN, &Style::plain(100)), "ok");
    let painted = runs(&runs_view(), NOW, &coloured);
    assert!(painted.contains("\x1b[1mrunning"), "{painted:?}");
    assert!(painted.contains("\x1b[32mcompleted"), "{painted:?}");
    let inspected = inspect(&view(None), None, true, &coloured);
    assert!(inspected.contains("\x1b[1mrunning"), "{inspected:?}");
    // The tree markers are content, not colour, so they are
    // unconditional: the model already emits Σ, ↓, … and — in pre-baked
    // text, and an ASCII mode would need a second derivation of each.
    assert!(inspected.contains('⑂') && inspected.contains('→'));
    let plain = inspect(&view(None), None, true, &Style::plain(100));
    assert!(plain.contains('⑂') && plain.contains('→'));
    assert!(!plain.contains('\x1b'));
}

/// The phase predicate is one function, called by `graph_block` and by
/// the TUI. Two copies of a scope rule is one copy too many.
#[test]
fn the_phase_scope_predicate_answers_in_both_lens_states() {
    let view = view(None);
    let design = view
        .phases
        .iter()
        .find(|phase| phase.name == "design")
        .unwrap();
    let intake = view
        .phases
        .iter()
        .find(|phase| phase.name == "intake")
        .unwrap();
    assert!(keeps_phase(None, design), "no lens keeps every phase");
    let lens = lens_for(&view, Some(&Scope::Phase("design".into())))
        .unwrap()
        .unwrap();
    assert!(keeps_phase(Some(&lens), design));
    assert!(!keeps_phase(Some(&lens), intake));
}

#[test]
fn a_multibyte_feature_truncates_on_a_char_boundary() {
    let running = state(Status::Running, None, None);
    let entries = [brokkr_view::RunEntry {
        run_id: "r1",
        feature: "ééééééééééééééééééééééééééééééééééééééééééééééééé",
        created_at: T0,
        state: Some(&running),
        detail: None,
    }];
    let out = runs(&brokkr_view::run_rows(&entries), NOW, &Style::plain(40));
    assert!(out.ends_with("…\n"), "{out:?}");
    assert!(out.is_char_boundary(out.len() - 1));
}

// ------------------------------------------------------------- AC-19

#[test]
fn brokkr_runs_is_one_clamped_line_per_run_newest_first() {
    let wide = runs(&runs_view(), NOW, &Style::plain(120));
    assert_eq!(
        wide,
        "run-7   running   design seq 14 6m58s one derivation, two surfaces\n\
         run-old completed design seq 14 7m03s an older feature whose text runs well past any terminal width and must be clamped…\n"
    );
    // At the clamp boundary the feature loses its tail to an ellipsis.
    let narrow = runs(&runs_view(), NOW, &Style::plain(48));
    assert_eq!(
        narrow,
        "run-7   running   design seq 14 6m58s one deriv…\n\
         run-old completed design seq 14 7m03s an older …\n"
    );
    // Below eight remaining columns the feature is omitted rather than
    // mangled into two characters and an ellipsis.
    let cramped = runs(&runs_view(), NOW, &Style::plain(44));
    assert_eq!(
        cramped,
        "run-7   running   design seq 14 6m58s\n\
         run-old completed design seq 14 7m03s\n"
    );
    // No `N runs` trailer: the count survives in --json.
    assert!(!wide.contains("2 runs"));
    assert_eq!(runs_view().count, 2);
}

#[test]
fn a_run_whose_journal_does_not_fold_still_lists() {
    let entries = [brokkr_view::RunEntry {
        run_id: "r1",
        feature: "unfoldable",
        created_at: "not a time",
        state: None,
        detail: None,
    }];
    let out = runs(&brokkr_view::run_rows(&entries), NOW, &Style::plain(80));
    assert_eq!(out, "r1 ? - seq - — unfoldable\n");
}

/// The quarantine the fleet listing gives a poisoned journal: the row
/// keeps its place, reads `?`, and carries the fold's own words under
/// itself the way a park reason does. Sanitized like every other line
/// that comes from outside — a fold error quotes journal payloads.
#[test]
fn a_quarantined_row_prints_the_fold_error_under_itself() {
    let healthy = state(Status::Running, None, None);
    let entries = [
        brokkr_view::RunEntry {
            run_id: "poisoned",
            feature: "the stop that came mid-flight",
            created_at: T0,
            state: None,
            detail: Some(
                "event 93: OperatorAccepted is impossible at cursor \
                 EffectInFlight\r\x1b[2Jforged",
            ),
        },
        brokkr_view::RunEntry {
            run_id: "healthy",
            feature: "still readable",
            created_at: T1,
            state: Some(&healthy),
            detail: None,
        },
    ];
    let out = runs(&brokkr_view::run_rows(&entries), NOW, &Style::plain(120));
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 3, "two rows, one of them explained: {out:?}");
    assert_eq!(
        lines[0],
        "healthy  running design seq 14 6m58s still readable"
    );
    assert!(lines[1].starts_with("poisoned ?"), "{out:?}");
    assert_eq!(
        lines[2],
        "  fold  event 93: OperatorAccepted is impossible at cursor EffectInFlight[2Jforged"
    );
    assert!(!out.contains('\x1b'), "{out:?}");
}

// ------------------------------------------------------------- AC-20

#[test]
fn brokkr_inspect_is_a_human_readout_with_a_terminal_tree() {
    let out = inspect(&view(None), None, true, &Style::plain(100));
    assert_eq!(
        out,
        "\
run  run-7
     running · phase design · seq 14
ruling  INTAKE-OK  intake → design · intook
live  design:positions:simplicity · working
live  design:positions:robustness · working
live  design · working
live  design:chief · turn 2 · Write

seats
  participant                 status    attempts turns cost    model activity
  intake                      succeeded 1        3     $0.0313 —     intook · 2m03s
  design                      working   1        Σ 2   —       —     3 members ↓
  design:positions:simplicity succeeded 1        —     —       —     0s
  design:positions:robustness succeeded 1        —     —       —     0s
  design:chief                working   1        2     —       —     Write

trail
  1 run/started        one derivation, two surfaces…
  2 phase/entered      intake
  7 effect/succeeded   intake · intook
  8 transition/decided INTAKE-OK intake → design · intook
  9 phase/entered      design

graph
  intake ×1
    → intake · finished · model —
  design ×1  ←current
    ⑂ positions
      simplicity · finished · model —
      robustness · finished · model —
    → chief · active · model —
"
    );
}

#[test]
fn a_park_reason_prints_before_everything_the_operator_would_act_on() {
    let out = inspect(
        &view(Some("needs a human")),
        None,
        false,
        &Style::plain(100),
    );
    let head: Vec<&str> = out.lines().take(4).collect();
    assert_eq!(head[2], "park  needs a human");
    assert!(!out.contains("trail"), "a watch frame carries no trail");
}

#[test]
fn scoping_flags_mirror_the_consoles_clicks() {
    let view = view(None);
    let by_phase = lens_for(&view, Some(&Scope::Phase("intake".into())))
        .unwrap()
        .unwrap();
    let scoped = inspect(&view, Some(&by_phase), true, &Style::plain(100));
    assert_eq!(
        scoped,
        "\
run  run-7
     running · phase design · seq 14
ruling  INTAKE-OK  intake → design · intook
live  design:positions:simplicity · working
live  design:positions:robustness · working
live  design · working
live  design:chief · turn 2 · Write

seats
  participant status    attempts turns cost    model activity
  intake      succeeded 1        3     $0.0313 —     intook · 2m03s

trail
  2 phase/entered      intake
  7 effect/succeeded   intake · intook
  8 transition/decided INTAKE-OK intake → design · intook

graph
  intake ×1
    → intake · finished · model —
"
    );

    // --seat matches EVERY occurrence, by label or by exact key.
    let by_label = lens_for(&view, Some(&Scope::Seat("design:chief".into())))
        .unwrap()
        .unwrap();
    let seat = inspect(&view, Some(&by_label), false, &Style::plain(100));
    assert!(seat.contains("design:chief working"), "{seat}");
    assert!(!seat.contains("\n  intake  "), "{seat}");
    let by_key = lens_for(&view, Some(&Scope::Seat("eff-d:chief".into())))
        .unwrap()
        .unwrap();
    assert_eq!(
        inspect(&view, Some(&by_key), false, &Style::plain(100)),
        seat
    );

    // No scope at all is no lens.
    assert!(lens_for(&view, None).unwrap().is_none());

    // A value matching nothing exits nonzero naming the valid ones: an
    // empty table would read as "this phase did nothing".
    let missing = lens_for(&view, Some(&Scope::Phase("nowhere".into())))
        .err()
        .unwrap();
    assert_eq!(
        missing,
        "no phase 'nowhere' in this run; visited phases: intake, design"
    );
    let unseated = lens_for(&view, Some(&Scope::Seat("nobody".into())))
        .err()
        .unwrap();
    assert!(unseated.starts_with("no seat 'nobody' in this run; participants: intake, design"));
}

#[test]
fn scope_miss_errors_are_escape_free() {
    // Error paths reach the operator's tty through anyhow, so they obey
    // the module invariant like every print path: the operator-typed
    // scope AND the journal-derived names it lists are sanitized. A
    // hostile recipe naming a seat "\x1b]0;pwned\x07" must not retitle
    // the terminal of anyone who mistypes a scope.
    let view = view(None);
    for scope in [
        Scope::Phase("no\x1b[2Jpe\r".into()),
        Scope::Seat("no\x1b]0;pwned\x07body".into()),
    ] {
        let message = lens_for(&view, Some(&scope)).err().unwrap();
        assert!(!message.contains('\x1b'), "escape survived: {message:?}");
        assert!(!message.contains('\r'), "carriage return survived");
        assert!(!message.contains('\x07'), "bell survived");
    }
}

#[test]
fn an_empty_run_still_renders_a_header() {
    // A journal that does not fold carries no summary — never a guessed
    // one — and there is nothing else to draw.
    let empty = brokkr_view::run_view(&[], None);
    assert_eq!(
        inspect(&empty, None, true, &Style::plain(80)),
        "run  — this journal does not fold\n"
    );
}

#[test]
fn a_ruling_with_a_problem_prints_it_under_the_ruling_line() {
    let events = journal();
    let stopped = state(
        Status::Stopped,
        None,
        Some(
            json!({"rule_id": "HARD-STOP", "severity": "hard", "from": "design",
                    "problem": "the seat refused"}),
        ),
    );
    let view = brokkr_view::run_view(&events, Some(&stopped));
    let out = inspect(&view, None, false, &Style::plain(80));
    assert!(out.contains("ruling  HARD-STOP  design → ?\n"), "{out}");
    assert!(out.contains("        the seat refused\n"), "{out}");
}

#[test]
fn a_phase_whose_only_traffic_is_its_entry_drops_the_seats_block() {
    // A scope that keeps no seats prints no seats table — but it still
    // prints the trail and the tree, because "this phase did nothing"
    // is a claim about a run, not an empty table.
    let events = vec![
        ev(1, EventType::PhaseEntered, json!({"phase": "intake"}), T0),
        ev(2, EventType::PhaseEntered, json!({"phase": "review"}), T0),
    ];
    let view = brokkr_view::run_view(&events, None);
    let lens = lens_for(&view, Some(&Scope::Phase("review".into())))
        .unwrap()
        .unwrap();
    let out = inspect(&view, Some(&lens), true, &Style::plain(80));
    assert_eq!(
        out,
        "run  — this journal does not fold\n\
         \n\
         trail\n\
         \x20 2 phase/entered review\n\
         \n\
         graph\n\
         \x20 review ×1  ←current\n"
    );
}

#[test]
fn a_panel_without_sequence_steps_forks_under_the_marker_alone() {
    let events = vec![
        ev(1, EventType::PhaseEntered, json!({"phase": "design"}), T0),
        ev(
            2,
            EventType::EffectRequested,
            json!({"effect_id": "eff", "seat": "design", "phase": "design"}),
            T0,
        ),
        ev(
            3,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff", "checkpoint": {"member": "a", "step": "note"}}),
            T0,
        ),
        ev(
            4,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff", "checkpoint": {"member": "b", "step": "note"}}),
            T0,
        ),
        // A seat whose effect names no phase belongs to no phase scope.
        ev(
            5,
            EventType::EffectRequested,
            json!({"effect_id": "loose", "seat": "loose"}),
            T0,
        ),
    ];
    let view = brokkr_view::run_view(&events, None);
    let out = inspect(&view, None, false, &Style::plain(80));
    assert!(
        out.contains("    ⑂\n"),
        "a fork column with no label prints the marker alone: {out}"
    );
    let lens = lens_for(&view, Some(&Scope::Phase("design".into())))
        .unwrap()
        .unwrap();
    let scoped = inspect(&view, Some(&lens), false, &Style::plain(80));
    assert!(
        !scoped.contains("loose"),
        "a phase-less seat is in no phase scope: {scoped}"
    );
}

#[test]
fn a_step_with_a_single_member_still_reads_as_the_step() {
    let events = vec![
        ev(1, EventType::PhaseEntered, json!({"phase": "design"}), T0),
        ev(
            2,
            EventType::EffectRequested,
            json!({"effect_id": "eff", "seat": "design", "phase": "design"}),
            T0,
        ),
        ev(
            3,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff", "checkpoint":
                   {"step": "sequence-step-finished", "step_name": "positions"}}),
            T0,
        ),
        ev(
            4,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff", "checkpoint": {"member": "positions:only", "step": "note"}}),
            T0,
        ),
    ];
    let view = brokkr_view::run_view(&events, None);
    let out = inspect(&view, None, false, &Style::plain(80));
    assert!(
        out.contains("    → positions · finished · model —\n"),
        "the column label wins over the lone node's own: {out}"
    );
}

#[test]
fn a_seat_lens_gathers_every_occurrence_and_survives_a_phase_less_one() {
    // A re-entered phase really did run that seat twice; hiding one
    // would be a false statement about the run.
    let events = vec![
        ev(
            1,
            EventType::PhaseEntered,
            json!({"phase": "implement"}),
            T0,
        ),
        ev(
            2,
            EventType::EffectRequested,
            json!({"effect_id": "one", "seat": "implement", "phase": "implement"}),
            T0,
        ),
        ev(
            1,
            EventType::PhaseEntered,
            json!({"phase": "implement"}),
            T0,
        ),
        ev(
            4,
            EventType::EffectRequested,
            json!({"effect_id": "two", "seat": "implement", "phase": "implement"}),
            T0,
        ),
        // A third occurrence whose effect names no phase at all.
        ev(
            5,
            EventType::EffectRequested,
            json!({"effect_id": "three", "seat": "implement"}),
            T0,
        ),
    ];
    let view = brokkr_view::run_view(&events, None);
    let lens = lens_for(&view, Some(&Scope::Seat("implement".into())))
        .unwrap()
        .unwrap();
    assert_eq!(lens.keys.len(), 3, "every occurrence");
    assert_eq!(lens.phases, vec!["implement".to_string()], "deduped");
    let out = inspect(&view, Some(&lens), false, &Style::plain(80));
    assert_eq!(
        out.lines()
            .filter(|line| line.starts_with("  implement ") && line.contains("working"))
            .count(),
        3,
        "{out}"
    );
}

#[test]
fn a_run_that_has_not_entered_a_phase_yet_says_so() {
    let events = journal();
    let mut early = state(Status::Running, None, None);
    early.phase = None;
    let view = brokkr_view::run_view(&events, Some(&early));
    let out = inspect(&view, None, false, &Style::plain(80));
    assert!(out.contains("running · phase - · seq 14"), "{out}");
}
