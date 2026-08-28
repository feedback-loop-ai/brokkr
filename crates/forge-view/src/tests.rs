use super::*;
use forge_core::fold::Cursor;
use serde_json::json;

const T0: &str = "2026-01-01T00:00:00Z";
const T1: &str = "2026-01-01T00:00:05Z";
const T2: &str = "2026-01-01T00:02:03Z";
const T3: &str = "2026-01-01T01:05:00Z";

fn ev(seq: u64, event_type: EventType, payload: Value, at: &str) -> EventEnvelope {
    EventEnvelope {
        run_id: "r1".to_string(),
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

fn caused(mut event: EventEnvelope, cause: &str) -> EventEnvelope {
    event.causation_id = Some(cause.to_string());
    event
}

fn state(phase: Option<&str>, status: Status, last_decision: Option<Value>) -> RunState {
    RunState {
        run_id: "r1".to_string(),
        seq: 7,
        last_hash: "hash".to_string(),
        status,
        phase: phase.map(str::to_string),
        cursor: Cursor::Idle,
        consecutive_failures: BTreeMap::new(),
        reviewed_heads: None,
        last_decision,
        park_reason: Some("needs a human".to_string()),
        feature: Some("feat".to_string()),
        pending_command: None,
    }
}

/// A single seat that starts, turns, finishes its session and succeeds.
fn seat_journal() -> Vec<EventEnvelope> {
    vec![
        ev(1, EventType::RunStarted, json!({"feature": "build it"}), T0),
        ev(2, EventType::PhaseEntered, json!({"phase": "intake"}), T0),
        ev(
            3,
            EventType::EffectRequested,
            json!({"effect_id": "eff1", "seat": "intake", "phase": "intake"}),
            T0,
        ),
        ev(
            4,
            EventType::EffectStarted,
            json!({"effect_id": "eff1", "attempt_id": "att1"}),
            T0,
        ),
        ev(
            5,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "attempt_id": "att1",
                   "checkpoint": {"step": "seat-turn", "turn": 1, "tool": "Read",
                                  "target": "crates/forge-view/src/lib.rs"}}),
            T0,
        ),
        ev(
            6,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "attempt_id": "att1",
                   "checkpoint": {"step": "claude-session-finished",
                                  "session_id": "sess-1", "total_cost_usd": 0.03125}}),
            T1,
        ),
        ev(
            7,
            EventType::EffectSucceeded,
            json!({"effect_id": "eff1", "attempt_id": "att1",
                   "result": {"result": "intook"}}),
            T2,
        ),
    ]
}

// ------------------------------------------------------------- AC-1

#[test]
fn the_crate_carries_no_io_no_clock_and_no_terminal_concept() {
    // Structural, not conventional: `forge-view`'s manifest depends on
    // exactly forge-core, serde and serde_json, and these tokens are the
    // ways a derivation quietly acquires a side effect.
    for source in [include_str!("lib.rs"), include_str!("js.rs")] {
        for banned in ["std::fs", "std::env", "IsTerminal", "print!", "println!"] {
            assert!(
                !source.contains(banned),
                "forge-view must not reach for {banned}"
            );
        }
    }
    let manifest = include_str!("../Cargo.toml");
    let deps = manifest.split("[dependencies]").nth(1).unwrap();
    for allowed in ["forge-core", "serde", "serde_json"] {
        assert!(deps.contains(allowed), "{allowed} is a dependency");
    }
    for forbidden in ["forge-store", "time", "rusqlite", "clap"] {
        assert!(!deps.contains(forbidden), "{forbidden} must stay out");
    }
}

// ------------------------------------------------------ small helpers

#[test]
fn js_truthiness_and_the_fixed_tables_are_ported_whole() {
    assert!(!truthy(None));
    assert!(!truthy(Some(&Value::Null)));
    assert!(!truthy(Some(&json!(false))));
    assert!(truthy(Some(&json!(true))));
    assert!(!truthy(Some(&json!(0))));
    assert!(truthy(Some(&json!(2))));
    assert!(!truthy(Some(&json!(""))));
    assert!(truthy(Some(&json!("x"))));
    assert!(truthy(Some(&json!({"a": 1}))));

    assert_eq!(status_str(&Status::Running), "running");
    assert_eq!(status_str(&Status::AwaitingOperator), "awaiting_operator");
    assert_eq!(status_str(&Status::Completed), "completed");
    assert_eq!(status_str(&Status::Stopped), "stopped");

    for (event_type, name) in [
        (EventType::RunStarted, "run/started"),
        (EventType::PhaseEntered, "phase/entered"),
        (EventType::EffectRequested, "effect/requested"),
        (EventType::EffectStarted, "effect/started"),
        (EventType::EffectCheckpointed, "effect/checkpointed"),
        (EventType::EffectSucceeded, "effect/succeeded"),
        (EventType::EffectFailed, "effect/failed"),
        (EventType::EffectIndeterminate, "effect/indeterminate"),
        (EventType::TransitionDecided, "transition/decided"),
        (EventType::OperatorCommanded, "operator/commanded"),
        (EventType::OperatorAccepted, "operator/accepted"),
        (EventType::OperatorRejected, "operator/rejected"),
        (EventType::RunParked, "run/parked"),
        (EventType::RunCompleted, "run/completed"),
        (EventType::RunStopped, "run/stopped"),
    ] {
        assert_eq!(type_str(event_type), name);
    }
    assert_eq!(type_tail("effect/succeeded"), "succeeded");
    assert_eq!(type_tail("bare"), "");

    assert_eq!(
        terminal_status(EventType::EffectSucceeded),
        Some(WORKING).map(|_| ("succeeded", "completed"))
    );
    assert_eq!(
        terminal_status(EventType::EffectFailed),
        Some(("failed", "stopped"))
    );
    assert_eq!(
        terminal_status(EventType::EffectIndeterminate),
        Some(("indeterminate", "awaiting_operator"))
    );
    assert_eq!(terminal_status(EventType::RunStarted), None);

    // `working` really is in the member allowlist: a member can be
    // pinned to working by its own outcome after the effect concluded.
    assert_eq!(outcome_status("working"), Some(("working", "running")));
    assert_eq!(
        outcome_status("succeeded"),
        Some(("succeeded", "completed"))
    );
    assert_eq!(outcome_status("failed"), Some(("failed", "stopped")));
    assert_eq!(
        outcome_status("indeterminate"),
        Some(("indeterminate", "awaiting_operator"))
    );
    assert_eq!(outcome_status("elsewhere"), None);

    assert_eq!(
        outcome_state("succeeded"),
        Some(("finished", "on-phosphor"))
    );
    assert_eq!(outcome_state("failed"), Some(("failed", "on-halt")));
    assert_eq!(
        outcome_state("indeterminate"),
        Some(("indeterminate", "on-park"))
    );
    assert_eq!(outcome_state("working"), None);

    assert_eq!(
        severity_class(&json!({"severity": "flagged"})),
        "awaiting_operator"
    );
    assert_eq!(severity_class(&json!({"severity": "hard"})), "stopped");
    assert_eq!(severity_class(&json!({"severity": "normal"})), "");
    assert_eq!(
        severity_class(&json!({})),
        "",
        "an unlisted severity is no class"
    );

    assert_eq!(
        result_token(&json!({"result": {"result": "ok"}})),
        Some("ok")
    );
    assert_eq!(result_token(&json!({"result": "ok"})), Some("ok"));
    assert_eq!(result_token(&json!({"result": 3})), None);
    assert_eq!(result_token(&json!({})), None);

    assert_eq!(display_or_mark(None), "?");
    assert_eq!(display_or_mark(Some(&Value::Null)), "?");
    assert_eq!(display_or_mark(Some(&json!("design"))), "design");
}

// ------------------------------------------------------------- AC-5/6

#[test]
fn fmt_dur_and_age_hit_every_boundary_the_console_has() {
    assert_eq!(fmt_dur(T0, "2026-01-01T00:00:59Z").as_deref(), Some("59s"));
    // Math.round is half-up: 59500ms is a minute, not 59 seconds.
    assert_eq!(
        fmt_dur(T0, "2026-01-01T00:00:59.500Z").as_deref(),
        Some("1m00s"),
        "seconds are zero-padded below an hour"
    );
    assert_eq!(fmt_dur(T0, T2).as_deref(), Some("2m03s"));
    // At an hour and above, seconds are dropped entirely.
    assert_eq!(fmt_dur(T0, T3).as_deref(), Some("1h05m"));
    assert_eq!(fmt_dur(T2, T0), None, "a negative delta is no duration");
    assert_eq!(fmt_dur(T0, "not a time"), None);
    assert_eq!(fmt_dur("not a time", T0), None);
    assert_eq!(age(T0, T2).as_deref(), Some("2m03s"));
}

#[test]
fn short_target_keeps_the_informative_tail() {
    let short = "src/lib.rs";
    assert_eq!(
        short_target(short),
        short,
        "44 code units or fewer, verbatim"
    );
    // Accumulation stops when one more segment would pass 40.
    assert_eq!(
        short_target("crates/forge-view/src/derivation/participants/activity.rs"),
        "…/src/derivation/participants/activity.rs"
    );
    // Accumulation can instead run out of segments: everything but the
    // head still fits, so the loop ends on its own condition.
    assert_eq!(
        short_target("aaaaaaaaaa/bb/cc/dddddddddddddddddddddddddddddddddd"),
        "…/bb/cc/dddddddddddddddddddddddddddddddddd"
    );
    // A single segment has nothing to accumulate.
    assert_eq!(
        short_target("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        "…/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    );
    // A é-bearing target counts UTF-16 code units, not bytes.
    let accented = "répertoire/répertoire/répertoire/répertoire/fichier.rs";
    assert_eq!(
        short_target(accented),
        "…/répertoire/répertoire/fichier.rs",
        "code units, not bytes: a byte count would stop a segment early"
    );
}

#[test]
fn clamp_truncates_on_char_boundaries() {
    assert_eq!(clamp("abc", 0), "");
    assert_eq!(clamp("abc", 3), "abc");
    assert_eq!(clamp("abcdef", 4), "abc…");
    assert_eq!(clamp("ééééé", 3), "éé…", "multi-byte chars never split");
}

// ------------------------------------------------------------ AC-15

#[test]
fn run_rows_are_newest_first_and_carry_the_whole_feature() {
    let running = state(Some("design"), Status::Running, None);
    let entries = [
        RunEntry {
            run_id: "old",
            feature: "the older feature",
            created_at: T0,
            state: None,
        },
        RunEntry {
            run_id: "new",
            feature: "the newer feature",
            created_at: T1,
            state: Some(&running),
        },
    ];
    let view = run_rows(&entries);
    assert_eq!(view.count, 2);
    assert_eq!(view.view_version, VIEW_VERSION);
    let json = serde_json::to_value(&view).unwrap();
    assert_eq!(json["runs"][0]["run_id"], "new");
    assert_eq!(json["runs"][0]["status"], "running");
    assert_eq!(json["runs"][0]["status_known"], true);
    assert_eq!(json["runs"][0]["phase"], "design");
    assert_eq!(json["runs"][0]["seq"], 7);
    assert_eq!(json["runs"][0]["feature"], "the newer feature");
    // A journal that does not fold carries no status — not a guessed one.
    assert_eq!(json["runs"][1]["run_id"], "old");
    assert_eq!(json["runs"][1]["status"], Value::Null);
    assert_eq!(json["runs"][1]["status_known"], false);
    assert_eq!(json["runs"][1]["seq"], Value::Null);
}

// ------------------------------------------------------------ AC-2/4

#[test]
fn a_seat_participant_carries_its_attempts_turns_cost_and_activity() {
    let events = seat_journal();
    let view = run_view(&events, None);
    let json = serde_json::to_value(&view).unwrap();
    let part = &json["participants"][0];
    assert_eq!(part["key"], "eff1");
    assert_eq!(part["label"], "intake");
    assert_eq!(part["member"], Value::Null);
    assert_eq!(part["phase"], "intake");
    assert_eq!(part["status"], "succeeded");
    assert_eq!(part["status_class"], "completed");
    assert_eq!(part["attempts"], 1);
    assert_eq!(part["turns"], 1);
    assert_eq!(part["turns_cell"]["text"], "1");
    assert_eq!(part["turns_cell"]["absent"], false);
    // The console's toFixed rounding, not Rust's.
    assert_eq!(part["cost_cell"]["text"], "$0.0313");
    assert_eq!(part["cost_aggregated"], false);
    assert_eq!(part["session_id"], "sess-1");
    // Concluded: the result token and how long the work took.
    assert_eq!(part["activity"]["text"], "intook · 2m03s");
    assert_eq!(part["activity"]["absent"], false);
    assert_eq!(
        part["terminal_line"]["text"],
        format!("effect/succeeded · {T2}")
    );
    assert_eq!(part["checkpoints"][0]["step"], "Read");
    assert_eq!(part["checkpoints"][0]["turn"]["text"], "1");
    assert_eq!(
        part["checkpoints"][0]["target"]["text"],
        "crates/forge-view/src/lib.rs"
    );
    assert_eq!(part["checkpoints"][0]["target_full"], Value::Null);
    assert_eq!(part["checkpoints"][1]["step"], "claude-session-finished");
    assert_eq!(part["checkpoints"][1]["turn"]["text"], ABSENT);
    assert_eq!(
        part["checkpoints"][1]["turn"]["note"],
        "not a numbered turn"
    );
    assert_eq!(part["checkpoints"][1]["target"]["absent"], true);
    assert_eq!(view.event_count, 7);
}

#[test]
fn a_live_seat_shows_the_tool_and_a_shortened_target() {
    let mut events = seat_journal();
    events.truncate(5);
    events.push(ev(
        6,
        EventType::EffectCheckpointed,
        json!({"effect_id": "eff1", "attempt_id": "att1",
               "checkpoint": {"step": "seat-turn", "turn": 4, "tool": "Edit",
                              "target": "crates/forge-view/src/derivation/participants/activity.rs"}}),
        T1,
    ));
    let view = run_view(&events, None);
    let part = &view.participants[0];
    assert_eq!(part.status, "working");
    assert_eq!(part.status_class, "running");
    assert_eq!(
        part.turns,
        Some(4),
        "turns is a max of the seat's own turns"
    );
    assert_eq!(part.activity.tool.as_deref(), Some("Edit"));
    assert_eq!(
        part.activity.target_short.as_deref(),
        Some("…/src/derivation/participants/activity.rs")
    );
    assert!(
        part.activity.target_full.is_some(),
        "shortened, so the full path rides along"
    );
    assert_eq!(
        part.activity.text,
        "Edit · …/src/derivation/participants/activity.rs"
    );
    assert_eq!(part.terminal_line.text, "no terminal event yet");
    assert!(part.terminal_line.absent);
    assert_eq!(part.cost_cell.text, ABSENT);
    assert_eq!(
        part.cost_cell.note.as_deref(),
        Some("no session cost recorded")
    );
    // The live block is a second scan and it is populated here.
    assert_eq!(view.live.len(), 1);
    assert_eq!(view.live[0].label, "intake");
    assert_eq!(view.live[0].text, "intake · turn 4 · Edit");
}

#[test]
fn a_live_seat_turn_without_a_tool_prints_the_js_undefined() {
    let events = vec![
        ev(
            1,
            EventType::EffectRequested,
            json!({"effect_id": "eff1", "seat": "intake", "phase": "intake"}),
            T0,
        ),
        ev(
            2,
            EventType::EffectStarted,
            json!({"effect_id": "eff1", "attempt_id": "att1"}),
            T0,
        ),
        ev(
            3,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "attempt_id": "att1",
                   "checkpoint": {"step": "seat-turn", "target": ""}}),
            T0,
        ),
    ];
    let view = run_view(&events, None);
    // JS string concatenation of an absent field yields the literal
    // `undefined`; the console prints exactly this today.
    assert_eq!(view.live[0].text, "intake · turn undefined · undefined");
    assert_eq!(view.participants[0].activity.text, "undefined");
    assert_eq!(
        view.participants[0].activity.tool.as_deref(),
        Some("undefined")
    );
    assert_eq!(view.participants[0].turns, None);
    assert_eq!(view.participants[0].turns_cell.text, ABSENT);
}

#[test]
fn a_terminal_for_a_stale_attempt_is_ignored_entirely() {
    let events = vec![
        ev(
            1,
            EventType::EffectRequested,
            json!({"effect_id": "eff1", "seat": "implement", "phase": "implement"}),
            T0,
        ),
        ev(
            2,
            EventType::EffectStarted,
            json!({"effect_id": "eff1", "attempt_id": "att1"}),
            T0,
        ),
        ev(
            3,
            EventType::EffectStarted,
            json!({"effect_id": "eff1", "attempt_id": "att2"}),
            T1,
        ),
        // A late terminal for the FIRST attempt: it must not conclude the
        // retry, must not set the status, and must contribute nothing —
        // no duration and no result token. A port matching on effect_id
        // alone would mark a retrying seat concluded.
        ev(
            4,
            EventType::EffectSucceeded,
            json!({"effect_id": "eff1", "attempt_id": "att1", "result": "stale"}),
            T2,
        ),
    ];
    let view = run_view(&events, None);
    assert_eq!(view.participants[0].status, "working");
    assert_eq!(view.participants[0].attempts, 2);
    assert_eq!(view.participants[0].activity.text, ABSENT);
    assert_eq!(
        view.participants[0].activity.note.as_deref(),
        Some("no activity recorded")
    );
    // The live block still shows the open second attempt.
    assert_eq!(view.live[0].text, "implement · working");
}

#[test]
fn unknown_and_unkeyed_effect_events_are_skipped_not_repaired() {
    let events = vec![
        // No effect_id: not a participant, and not an error either.
        ev(1, EventType::EffectRequested, json!({"seat": "ghost"}), T0),
        // A known type naming an effect nobody requested.
        ev(
            2,
            EventType::EffectStarted,
            json!({"effect_id": "nobody", "attempt_id": "a"}),
            T0,
        ),
        ev(
            3,
            EventType::EffectRequested,
            json!({"effect_id": "eff1", "phase": "intake"}),
            T0,
        ),
        // An event that names the effect but is not part of its lifecycle.
        ev(
            4,
            EventType::TransitionDecided,
            json!({"effect_id": "eff1", "rule_id": "R", "from": "intake", "next": "design"}),
            T0,
        ),
    ];
    let view = run_view(&events, None);
    assert_eq!(view.participants.len(), 1);
    assert_eq!(view.participants[0].label, "?", "an unnamed seat is '?'");
    assert_eq!(view.participants[0].attempts, 0);
    // The live block is a different scan: it tracks the last start
    // globally, so an unrequested effect still opens it.
    assert_eq!(view.live[0].text, "? · working");
}

// -------------------------------------------------------------- AC-3

/// A panel: the parent seat carries no telemetry, its two members do.
fn panel_journal() -> Vec<EventEnvelope> {
    vec![
        ev(1, EventType::PhaseEntered, json!({"phase": "design"}), T0),
        ev(
            2,
            EventType::EffectRequested,
            json!({"effect_id": "eff1", "seat": "design", "phase": "design"}),
            T0,
        ),
        ev(
            3,
            EventType::EffectStarted,
            json!({"effect_id": "eff1", "attempt_id": "att1"}),
            T0,
        ),
        ev(
            4,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "attempt_id": "att1",
                   "checkpoint": {"member": "simplicity", "step": "seat-turn", "turn": 2,
                                  "tool": "Write"}}),
            T0,
        ),
        ev(
            5,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "attempt_id": "att1",
                   "checkpoint": {"member": "robustness", "step": "seat-turn", "turn": 3,
                                  "tool": "Write"}}),
            T0,
        ),
        ev(
            6,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "attempt_id": "att1",
                   "checkpoint": {"member": "simplicity", "step": "claude-session-finished",
                                  "session_id": "s1", "total_cost_usd": 0.03125}}),
            T1,
        ),
        ev(
            7,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "attempt_id": "att1",
                   "checkpoint": {"member": "robustness", "step": "claude-session-finished",
                                  "session_id": "s2", "total_cost_usd": 0.25}}),
            T1,
        ),
        ev(
            8,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "attempt_id": "att1",
                   "checkpoint": {"member": "simplicity", "step": "panel-member-finished",
                                  "outcome": "succeeded"}}),
            T1,
        ),
        ev(
            9,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "attempt_id": "att1",
                   "checkpoint": {"member": "robustness", "step": "panel-member-finished",
                                  "outcome": "failed"}}),
            T1,
        ),
        ev(
            10,
            EventType::EffectSucceeded,
            json!({"effect_id": "eff1", "attempt_id": "att1", "result": {"result": "designed"}}),
            T2,
        ),
    ]
}

#[test]
fn a_panel_parent_aggregates_its_members_with_a_sigma() {
    let events = panel_journal();
    let view = run_view(&events, None);
    let parent = &view.participants[0];
    assert_eq!(parent.label, "design");
    assert_eq!(parent.member_count, 2);
    assert!(parent.cost_aggregated && parent.turns_aggregated);
    assert_eq!(parent.turns, Some(5), "Σ turns is a SUM across members");
    assert_eq!(parent.turns_cell.text, "Σ 5");
    assert_eq!(parent.cost_cell.text, "Σ $0.2813");
    assert_eq!(parent.activity.text, "designed · 2m03s");

    let simplicity = &view.participants[1];
    assert_eq!(simplicity.key, "eff1:simplicity");
    assert_eq!(simplicity.label, "design:simplicity");
    assert_eq!(simplicity.member.as_deref(), Some("simplicity"));
    assert_eq!(simplicity.status, "succeeded");
    assert!(!simplicity.cost_aggregated, "a member never aggregates");
    assert_eq!(simplicity.cost_cell.text, "$0.0313");
    assert_eq!(simplicity.turns_cell.text, "2", "no Σ on a member");
    assert_eq!(
        simplicity.terminal_line.text,
        format!("panel-member-finished · succeeded · {T1}")
    );

    // A member rules on its own outcome, not the effect's.
    let robustness = &view.participants[2];
    assert_eq!(robustness.status, "failed");
    assert_eq!(robustness.status_class, "stopped");
    assert_eq!(
        robustness.activity.text, "5s",
        "a member has no result token"
    );
}

#[test]
fn a_parent_with_its_own_telemetry_does_not_aggregate() {
    let mut events = panel_journal();
    events.push(ev(
        11,
        EventType::EffectCheckpointed,
        json!({"effect_id": "eff1", "attempt_id": "att1",
               "checkpoint": {"step": "claude-session-finished", "total_cost_usd": 1.5}}),
        T2,
    ));
    events.push(ev(
        12,
        EventType::EffectCheckpointed,
        json!({"effect_id": "eff1", "attempt_id": "att1",
               "checkpoint": {"step": "seat-turn", "turn": 9, "tool": "Bash"}}),
        T2,
    ));
    let view = run_view(&events, None);
    let parent = &view.participants[0];
    assert!(!parent.cost_aggregated && !parent.turns_aggregated);
    assert_eq!(parent.cost_cell.text, "$1.5000");
    assert_eq!(parent.turns_cell.text, "9");
}

#[test]
fn a_parent_whose_members_report_nothing_says_so_rather_than_reading_empty() {
    let events = vec![
        ev(
            1,
            EventType::EffectRequested,
            json!({"effect_id": "eff1", "seat": "design", "phase": "design"}),
            T0,
        ),
        ev(
            2,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "checkpoint": {"member": "a", "step": "note"}}),
            T0,
        ),
    ];
    let view = run_view(&events, None);
    assert_eq!(view.participants[0].member_count, 1);
    assert_eq!(view.participants[0].activity.text, "1 members ↓");
    assert!(view.participants[0].activity.absent);
    assert_eq!(view.participants[0].cost_cell.text, ABSENT);
    // The member's own row: no panel-member-finished and no effect
    // terminal, so no duration either.
    assert_eq!(view.participants[1].activity.text, ABSENT);
}

#[test]
fn a_member_outcome_of_working_pins_the_member_after_the_effect_concluded() {
    let mut events = panel_journal();
    events.truncate(8);
    events.push(ev(
        9,
        EventType::EffectCheckpointed,
        json!({"effect_id": "eff1", "attempt_id": "att1",
               "checkpoint": {"member": "robustness", "step": "panel-member-finished",
                              "outcome": "working"}}),
        T1,
    ));
    events.push(ev(
        10,
        EventType::EffectSucceeded,
        json!({"effect_id": "eff1", "attempt_id": "att1", "result": "designed"}),
        T2,
    ));
    let view = run_view(&events, None);
    let robustness = view
        .participants
        .iter()
        .find(|part| part.key == "eff1:robustness")
        .unwrap();
    assert_eq!(
        robustness.status, "working",
        "`working` is in the allowlist, so the console shows it here"
    );
    // Working with a last turn: the activity is the live tool again.
    assert_eq!(robustness.activity.text, "Write");
    assert_eq!(robustness.activity.target_short, None);
}

#[test]
fn a_null_member_outcome_falls_through_to_the_effects_own_terminal() {
    let events = vec![
        ev(
            1,
            EventType::EffectRequested,
            json!({"effect_id": "eff1", "seat": "design", "phase": "design"}),
            T0,
        ),
        ev(
            2,
            EventType::EffectStarted,
            json!({"effect_id": "eff1", "attempt_id": "att1"}),
            T0,
        ),
        ev(
            3,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "attempt_id": "att1",
                   "checkpoint": {"member": "a", "step": "panel-member-finished",
                                  "outcome": null}}),
            T0,
        ),
        ev(
            4,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "attempt_id": "att1",
                   "checkpoint": {"member": "b", "step": "panel-member-finished"}}),
            T0,
        ),
        // A panel-member-finished with no member is not a member outcome.
        ev(
            5,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "attempt_id": "att1",
                   "checkpoint": {"step": "panel-member-finished", "outcome": "succeeded"}}),
            T0,
        ),
        ev(
            6,
            EventType::EffectIndeterminate,
            json!({"effect_id": "eff1", "attempt_id": "att1", "reason": "vanished"}),
            T1,
        ),
    ];
    let view = run_view(&events, None);
    let member_a = &view.participants[1];
    assert_eq!(member_a.status, "indeterminate");
    assert_eq!(member_a.status_class, "awaiting_operator");
    assert_eq!(
        member_a.terminal_line.text,
        format!("effect/indeterminate · {T1}"),
        "a null outcome is not a conclusion"
    );
    // An ABSENT outcome is a conclusion, and prints the JS undefined.
    let member_b = &view.participants[2];
    assert_eq!(
        member_b.terminal_line.text,
        format!("panel-member-finished · undefined · {T0}")
    );
    assert_eq!(view.participants[0].status, "indeterminate");
}

// -------------------------------------------------------------- AC-14

#[test]
fn the_live_block_synthesizes_a_bare_seat_row_when_the_scan_found_nothing() {
    let events = vec![
        ev(
            1,
            EventType::EffectRequested,
            json!({"effect_id": "eff1", "seat": "", "phase": "p"}),
            T0,
        ),
        ev(
            2,
            EventType::EffectStarted,
            json!({"effect_id": "eff1", "attempt_id": "att1"}),
            T0,
        ),
        // A checkpoint for a DIFFERENT attempt contributes no live row.
        ev(
            3,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "attempt_id": "other",
                   "checkpoint": {"step": "seat-turn", "turn": 1}}),
            T0,
        ),
    ];
    let view = run_view(&events, None);
    // `p.seat` truthiness: an empty seat name is not a name.
    assert_eq!(view.live.len(), 1);
    assert_eq!(view.live[0].label, "?");
    assert_eq!(view.live[0].text, "? · working");
}

#[test]
fn the_live_block_keeps_a_turn_over_a_later_bare_checkpoint() {
    let events = vec![
        ev(
            1,
            EventType::EffectRequested,
            json!({"effect_id": "eff1", "seat": "design", "phase": "design"}),
            T0,
        ),
        ev(
            2,
            EventType::EffectStarted,
            json!({"effect_id": "eff1", "attempt_id": "att1"}),
            T0,
        ),
        ev(
            3,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "attempt_id": "att1",
                   "checkpoint": {"member": "one", "step": "note"}}),
            T0,
        ),
        // A second bare checkpoint for a label already standing at
        // "working" leaves it where it is.
        ev(
            4,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "attempt_id": "att1",
                   "checkpoint": {"member": "one", "step": "note"}}),
            T0,
        ),
        ev(
            5,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "attempt_id": "att1",
                   "checkpoint": {"member": "one", "step": "seat-turn", "turn": 2, "tool": "Read"}}),
            T0,
        ),
        // A bare checkpoint after the turn must not erase it.
        ev(
            6,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "attempt_id": "att1",
                   "checkpoint": {"member": "one", "step": "note"}}),
            T0,
        ),
        ev(
            7,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "attempt_id": "att1"}),
            T0,
        ),
        // A checkpoint belonging to another effect entirely.
        ev(
            8,
            EventType::EffectCheckpointed,
            json!({"effect_id": "elsewhere", "attempt_id": "att1",
                   "checkpoint": {"step": "seat-turn", "turn": 9}}),
            T0,
        ),
    ];
    let view = run_view(&events, None);
    assert_eq!(view.live.len(), 2);
    assert_eq!(view.live[0].text, "design:one · turn 2 · Read");
    assert_eq!(view.live[1].text, "design · working");
}

#[test]
fn a_matching_terminal_closes_the_live_block() {
    let view = run_view(&seat_journal(), None);
    assert!(view.live.is_empty());
}

// --------------------------------------------------------- AC-8, AC-9

#[test]
fn a_phase_with_no_observed_effect_draws_one_plain_node() {
    let events = vec![
        ev(1, EventType::PhaseEntered, json!({"phase": "intake"}), T0),
        ev(2, EventType::PhaseEntered, json!({"phase": 7}), T0),
        ev(3, EventType::PhaseEntered, json!({"phase": "intake"}), T0),
    ];
    let view = run_view(&events, None);
    assert_eq!(view.phases.len(), 1, "a non-string phase name is skipped");
    assert_eq!(view.phases[0].visits, 2);
    assert!(view.phases[0].plain);
    assert!(view.phases[0].columns.is_empty());
    assert!(
        view.phases[0].current,
        "the last visited phase, with no summary"
    );
}

#[test]
fn the_current_phase_comes_from_the_summary_when_it_was_visited() {
    let events = vec![
        ev(1, EventType::PhaseEntered, json!({"phase": "intake"}), T0),
        ev(2, EventType::PhaseEntered, json!({"phase": "design"}), T0),
    ];
    let visited = state(Some("intake"), Status::Running, None);
    let view = run_view(&events, Some(&visited));
    assert!(view.phases[0].current && !view.phases[1].current);
    // A phase the run never entered falls back to the last visited one.
    let elsewhere = state(Some("nowhere"), Status::Running, None);
    let view = run_view(&events, Some(&elsewhere));
    assert!(!view.phases[0].current && view.phases[1].current);
    // No phases at all: an empty rail, not an invented one.
    assert!(run_view(&[], None).phases.is_empty());
}

#[test]
fn a_panel_phase_renders_its_members_as_one_parallel_column() {
    let view = run_view(&panel_journal(), None);
    let phase = &view.phases[0];
    assert!(!phase.plain);
    assert_eq!(phase.columns.len(), 1);
    assert_eq!(phase.columns[0].label, None);
    assert_eq!(phase.columns[0].nodes.len(), 2);
    assert_eq!(phase.columns[0].nodes[0].label, "simplicity");
    assert_eq!(phase.columns[0].nodes[0].key, "eff1:simplicity");
    assert_eq!(phase.columns[0].nodes[0].state, "finished");
    assert_eq!(phase.columns[0].nodes[0].state_class, "on-phosphor");
    assert_eq!(phase.columns[0].nodes[1].state, "failed");
    assert_eq!(phase.columns[0].nodes[1].state_class, "on-halt");
}

#[test]
fn a_sequence_phase_renders_its_steps_in_finished_order_with_member_forks() {
    let events = vec![
        ev(1, EventType::PhaseEntered, json!({"phase": "design"}), T0),
        // An earlier effect for the same phase: the NEWEST one wins.
        ev(
            2,
            EventType::EffectRequested,
            json!({"effect_id": "old", "seat": "design", "phase": "design"}),
            T0,
        ),
        ev(
            3,
            EventType::EffectRequested,
            json!({"effect_id": "eff1", "seat": "design", "phase": "design"}),
            T0,
        ),
        ev(
            4,
            EventType::EffectStarted,
            json!({"effect_id": "eff1", "attempt_id": "att1"}),
            T0,
        ),
        ev(
            5,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "checkpoint":
                   {"step": "sequence-step-finished", "step_name": "positions"}}),
            T0,
        ),
        // Repeats are deduped in first-observed order.
        ev(
            6,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "checkpoint":
                   {"step": "sequence-step-finished", "step_name": "positions"}}),
            T0,
        ),
        ev(
            7,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "checkpoint":
                   {"step": "sequence-step-finished"}}),
            T0,
        ),
        ev(
            8,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "checkpoint":
                   {"member": "positions:simplicity", "step": "panel-member-finished",
                    "outcome": "succeeded"}}),
            T0,
        ),
        ev(
            9,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "checkpoint":
                   {"member": "positions:robustness", "step": "seat-turn", "turn": 1}}),
            T0,
        ),
        // A member tag naming a step the sequence has not finished yet.
        ev(
            10,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "checkpoint": {"member": "chief", "step": "note"}}),
            T0,
        ),
        // A finished step nobody tagged: a single node whose key names
        // no participant at all.
        ev(
            11,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "checkpoint":
                   {"step": "sequence-step-finished", "step_name": "review"}}),
            T0,
        ),
        // A checkpoint with no checkpoint object at all.
        ev(
            12,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1"}),
            T0,
        ),
    ];
    let view = run_view(&events, None);
    let phase = &view.phases[0];
    assert!(!phase.plain);
    assert_eq!(phase.columns.len(), 3);
    // The finished step forks into its two members.
    assert_eq!(phase.columns[0].label.as_deref(), Some("positions"));
    assert_eq!(phase.columns[0].nodes[0].label, "simplicity");
    assert_eq!(phase.columns[0].nodes[0].state, "finished");
    assert_eq!(phase.columns[0].nodes[1].label, "robustness");
    assert_eq!(
        phase.columns[0].nodes[1].state, "finished",
        "a finished step is phosphor even without its own outcome"
    );
    // A finished step nobody tagged is one node, phosphor, naming no
    // participant of its own.
    assert_eq!(phase.columns[1].label, None);
    assert_eq!(phase.columns[1].nodes[0].label, "review");
    assert_eq!(phase.columns[1].nodes[0].key, "eff1:review");
    assert_eq!(phase.columns[1].nodes[0].state, "finished");
    // The unfinished step is a single node keyed on its tag.
    assert_eq!(phase.columns[2].label, None);
    assert_eq!(phase.columns[2].nodes[0].label, "chief");
    assert_eq!(phase.columns[2].nodes[0].key, "eff1:chief");
    assert_eq!(phase.columns[2].nodes[0].state, "active");
    assert_eq!(phase.columns[2].nodes[0].state_class, "in-active");
}

#[test]
fn a_structureless_phase_draws_one_node_labelled_by_its_seat() {
    let view = run_view(&seat_journal(), None);
    let phase = &view.phases[0];
    assert!(phase.plain);
    assert_eq!(phase.columns[0].nodes[0].label, "intake");
    assert_eq!(phase.columns[0].nodes[0].key, "eff1");
    assert_eq!(phase.columns[0].nodes[0].state, "finished");

    // An effect requested with no seat name draws the same `?` the
    // participant scan gives it.
    let events = vec![
        ev(1, EventType::PhaseEntered, json!({"phase": "solo"}), T0),
        ev(
            2,
            EventType::EffectRequested,
            json!({"effect_id": "eff1", "phase": "solo"}),
            T0,
        ),
        ev(
            3,
            EventType::EffectRequested,
            json!({"effect_id": "eff1", "phase": "solo"}),
            T0,
        ),
        ev(
            4,
            EventType::EffectRequested,
            json!({"effect_id": "eff2"}),
            T0,
        ),
    ];
    let view = run_view(&events, None);
    assert_eq!(view.phases[0].columns[0].nodes[0].label, "?");
    // The repeat request in the same phase is one membership, not two.
    let row = view.journal.iter().find(|row| row.seq == 3).unwrap();
    assert_eq!(row.phases, vec!["solo".to_string()]);
}

#[test]
fn an_effect_failure_colours_its_node_by_the_effects_own_status() {
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
            json!({"effect_id": "eff1", "seat": "implement", "phase": "implement"}),
            T0,
        ),
        ev(
            3,
            EventType::EffectStarted,
            json!({"effect_id": "eff1", "attempt_id": "att1"}),
            T0,
        ),
        ev(
            4,
            EventType::EffectFailed,
            json!({"effect_id": "eff1", "attempt_id": "att1", "error": ""}),
            T1,
        ),
    ];
    let view = run_view(&events, None);
    assert_eq!(view.phases[0].columns[0].nodes[0].state, "failed");
    assert_eq!(view.participants[0].activity.text, "5s");
    let row = view.journal.iter().find(|row| row.seq == 4).unwrap();
    assert_eq!(row.what.text, "implement");
    assert_eq!(row.what.problem, None, "an empty error is no problem line");
}

// ------------------------------------------------------ AC-10, AC-13

#[test]
fn the_summary_carries_the_nine_keys_and_the_ruling_reads_the_last_decision() {
    let decision = json!({
        "rule_id": "DESIGN-OK", "severity": "flagged", "from": "design",
        "next": "implement", "result": "designed",
        "inputs": {"positions": 2}, "problem": "none really",
    });
    let parked = state(Some("design"), Status::AwaitingOperator, Some(decision));
    let view = run_view(&seat_journal(), Some(&parked));
    let json = serde_json::to_value(&view).unwrap();
    let summary = &json["summary"];
    for key in [
        "consecutive_failures",
        "cursor",
        "feature",
        "last_decision",
        "park_reason",
        "phase",
        "run_id",
        "seq",
        "status",
    ] {
        assert!(summary.get(key).is_some(), "summarize() key {key}");
    }
    assert_eq!(summary["cursor"], "Idle");
    assert_eq!(summary["status"], "awaiting_operator");
    let ruling = &json["ruling"];
    assert_eq!(ruling["rule_id"], "DESIGN-OK");
    assert_eq!(ruling["severity_class"], "awaiting_operator");
    assert_eq!(ruling["from"], "design");
    assert_eq!(ruling["next"], "implement");
    assert_eq!(ruling["result"], "designed");
    assert_eq!(ruling["inputs"][0][0], "positions");
    assert_eq!(ruling["inputs"][0][1], "2");
    assert_eq!(ruling["problem"], "none really");
}

#[test]
fn a_decision_without_a_rule_id_is_not_a_ruling() {
    assert!(ruling_of(None).is_none());
    assert!(ruling_of(Some(&json!("a string"))).is_none());
    assert!(ruling_of(Some(&json!({"from": "a"}))).is_none());
    let bare = ruling_of(Some(
        &json!({"rule_id": null, "result": "", "problem": null}),
    ))
    .unwrap();
    assert_eq!(bare.rule_id, "?");
    assert_eq!(bare.from, "?", "an absent from renders the mark");
    assert_eq!(bare.next, "?");
    assert_eq!(bare.result, None, "an empty result is no result");
    assert_eq!(bare.problem, None);
    assert!(bare.inputs.is_empty());
    assert_eq!(bare.severity_class, "");
}

#[test]
fn scope_tags_are_precomputed_so_no_surface_implements_the_predicate() {
    let events = vec![
        ev(1, EventType::PhaseEntered, json!({"phase": "design"}), T0),
        ev(
            2,
            EventType::EffectRequested,
            json!({"effect_id": "eff1", "seat": "design", "phase": "design"}),
            T0,
        ),
        ev(
            3,
            EventType::EffectStarted,
            json!({"effect_id": "eff1", "attempt_id": "att1"}),
            T0,
        ),
        // A decision LEAVING design belongs to design; `next` is
        // deliberately not a match.
        ev(
            4,
            EventType::TransitionDecided,
            json!({"rule_id": "R", "from": "design", "next": "implement"}),
            T0,
        ),
        // phase and from naming the same phase is one membership.
        ev(
            5,
            EventType::RunParked,
            json!({"phase": "design", "from": "design", "reason": "stuck"}),
            T0,
        ),
    ];
    let view = run_view(&events, None);
    assert_eq!(view.participants[0].phase.as_deref(), Some("design"));
    let by_seq = |seq: u64| {
        &view
            .journal
            .iter()
            .find(|row| row.seq == seq)
            .unwrap()
            .phases
    };
    assert_eq!(by_seq(1), &vec!["design".to_string()]);
    assert_eq!(by_seq(2), &vec!["design".to_string()]);
    assert_eq!(
        by_seq(3),
        &vec!["design".to_string()],
        "an effect rides its phase"
    );
    assert_eq!(by_seq(4), &vec!["design".to_string()]);
    assert_eq!(by_seq(5), &vec!["design".to_string()]);
}

// ----------------------------------------------------- AC-11, AC-12

#[test]
fn the_trail_classifies_every_event_type_it_knows() {
    let events = vec![
        ev(1, EventType::RunStarted, json!({"feature": "hello"}), T0),
        ev(2, EventType::RunStarted, json!({}), T0),
        ev(3, EventType::PhaseEntered, json!({"phase": "design"}), T0),
        ev(4, EventType::PhaseEntered, json!({}), T0),
        caused(
            ev(
                5,
                EventType::TransitionDecided,
                json!({"rule_id": "DESIGN-OK", "severity": "hard", "from": "design",
                       "next": "implement", "result": "designed", "problem": "a problem"}),
                T0,
            ),
            "ev3",
        ),
        ev(6, EventType::TransitionDecided, json!({}), T0),
        ev(
            7,
            EventType::EffectRequested,
            json!({"effect_id": "eff1", "seat": "design", "phase": "design"}),
            T0,
        ),
        ev(
            8,
            EventType::EffectStarted,
            json!({"effect_id": "eff1", "attempt_id": "att1"}),
            T0,
        ),
        ev(
            9,
            EventType::EffectFailed,
            json!({"effect_id": "eff1", "attempt_id": "att1",
                   "error": "a very long error that the trail clamps"}),
            T0,
        ),
        // A terminal for an effect nobody requested falls back to the id.
        ev(
            10,
            EventType::EffectSucceeded,
            json!({"effect_id": "ghost", "result": {"result": "done"}}),
            T0,
        ),
        ev(11, EventType::EffectIndeterminate, json!({}), T0),
        ev(
            12,
            EventType::RunParked,
            json!({"reason": "operator needed"}),
            T0,
        ),
        ev(
            13,
            EventType::OperatorCommanded,
            json!({"command": "retry"}),
            T0,
        ),
        // A crafted causation id must not resolve through a prototype.
        caused(ev(14, EventType::RunCompleted, json!({}), T0), "__proto__"),
        ev(
            15,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1",
                   "checkpoint": {"step": "seat-turn", "tool": "Read", "member": "one"}}),
            T0,
        ),
        ev(
            16,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "checkpoint": {"step": "seat-turn"}}),
            T0,
        ),
        ev(
            17,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "checkpoint": {}}),
            T0,
        ),
    ];
    let view = run_view(&events, None);
    let row = |seq: u64| view.journal.iter().find(|row| row.seq == seq).unwrap();

    // The 110-character clamp's ellipsis is UNCONDITIONAL.
    assert_eq!(row(1).what.text, "hello…");
    assert_eq!(row(2).what.text, "run started");
    assert_eq!(row(3).what.badge.as_deref(), Some("design"));
    assert_eq!(row(4).what.text, "?");
    assert_eq!(row(5).what.text, "DESIGN-OK design → implement · designed");
    assert_eq!(row(5).what.badge_class.as_deref(), Some("stopped"));
    assert_eq!(row(5).what.problem.as_deref(), Some("a problem"));
    assert_eq!(row(5).causation_seq, Some(3));
    assert_eq!(row(6).what.text, "? ? → ?");
    assert_eq!(row(6).what.problem, None);
    assert_eq!(row(9).what.text, "design");
    assert!(row(9).what.problem.is_some());
    assert_eq!(row(10).what.text, "ghost · done");
    assert_eq!(row(11).what.text, "?", "no effect id and no participant");
    assert_eq!(row(12).what.text, "operator needed");
    assert_eq!(
        row(13).what.text,
        "commanded",
        "the type's own second segment"
    );
    assert_eq!(
        row(14).causation_seq,
        None,
        "an unknown causation is no arrow"
    );

    // The default trail hides checkpoints and effect plumbing.
    assert!(!row(7).in_trail && !row(8).in_trail && !row(15).in_trail);
    assert!(row(1).in_trail && row(5).in_trail && row(9).in_trail);
    assert_eq!(view.event_count, 17, "the count is unfiltered");

    // Full-journal labels, including the checkpoint override.
    assert_eq!(row(5).label.text, "DESIGN-OK");
    assert_eq!(row(3).label.text, "design");
    assert_eq!(row(10).label.text, "done", "the result token");
    assert_eq!(row(13).label.text, "retry");
    assert_eq!(row(12).label.text, "operator needed");
    assert_eq!(row(15).label.text, "one · Read");
    assert_eq!(row(16).label.text, "seat-turn", "a turn with no tool");
    assert_eq!(row(17).label.text, ABSENT);
    assert!(row(17).label.absent);
    assert_eq!(row(14).label.text, ABSENT);
    assert_eq!(row(9).label.text, ABSENT, "an error is not a label");
}

#[test]
fn payload_json_is_derived_once_so_both_surfaces_agree() {
    let mut events = vec![ev(1, EventType::RunStarted, json!({"turns": 1.0}), T0)];
    events.push(ev(2, EventType::RunParked, Value::String("odd".into()), T0));
    let view = run_view(&events, None);
    // Named delta versus today's console: serde_json renders 1.0 where
    // JSON.stringify renders 1. Key order is unaffected.
    assert_eq!(view.journal[0].payload_json, r#"{"turns":1.0}"#);
    // A non-object payload is the console's `e.payload || {}`.
    assert_eq!(view.journal[1].payload_json, "{}");
    assert_eq!(view.journal[1].what.text, "parked");
}

#[test]
fn the_odd_shapes_a_journal_can_carry_are_read_not_repaired() {
    let events = vec![
        ev(1, EventType::PhaseEntered, json!({"phase": "design"}), T2),
        ev(
            2,
            EventType::EffectRequested,
            json!({"effect_id": "eff1", "seat": "design", "phase": "design"}),
            T2,
        ),
        ev(
            3,
            EventType::EffectStarted,
            json!({"effect_id": "eff1", "attempt_id": "att1"}),
            T2,
        ),
        // An empty target string is not a target.
        ev(
            4,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "attempt_id": "att1",
                   "checkpoint": {"step": "seat-turn", "turn": 1, "tool": "Bash",
                                  "target": ""}}),
            T2,
        ),
        // A member outcome outside the allowlist does not override.
        ev(
            5,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "attempt_id": "att1",
                   "checkpoint": {"member": "one", "step": "panel-member-finished",
                                  "outcome": "elsewhere"}}),
            T2,
        ),
        // A checkpoint that names no effect at all.
        ev(
            6,
            EventType::EffectCheckpointed,
            json!({"checkpoint": {"step": "seat-turn"}}),
            T2,
        ),
        // A terminal for a DIFFERENT effect leaves the open one open.
        ev(
            7,
            EventType::EffectSucceeded,
            json!({"effect_id": "elsewhere", "attempt_id": "att1"}),
            T2,
        ),
        // A ruling whose problem is present but empty is no problem.
        ev(
            8,
            EventType::TransitionDecided,
            json!({"rule_id": "R", "from": "design", "next": "ship", "problem": ""}),
            T2,
        ),
        // The effect concludes BEFORE it started: a negative delta is no
        // duration, and the activity falls back to the result token.
        ev(
            9,
            EventType::EffectSucceeded,
            json!({"effect_id": "eff1", "attempt_id": "att1", "result": "designed"}),
            T0,
        ),
    ];
    let view = run_view(&events, None);
    let parent = &view.participants[0];
    assert_eq!(
        parent.activity.text, "designed",
        "no duration, just the token"
    );
    assert_eq!(parent.checkpoints[0].target.text, ABSENT);
    assert_eq!(
        view.participants[1].status, "succeeded",
        "an unlisted member outcome does not override the effect"
    );
    let row = view.journal.iter().find(|row| row.seq == 8).unwrap();
    assert_eq!(row.what.problem, None);
    // The live block closed on the matching terminal, not the foreign one.
    assert!(view.live.is_empty());
    assert_eq!(view.phases[0].columns[0].nodes[0].state, "finished");
}

#[test]
fn an_effect_with_no_ids_at_all_still_reads_as_the_console_reads_it() {
    // JavaScript compares `undefined === undefined` and finds them
    // equal, so an id-less attempt still matches its own checkpoints.
    let events = vec![
        // A named effect that is NOT the open one: its seat must not
        // leak into the live line.
        ev(
            1,
            EventType::EffectRequested,
            json!({"effect_id": "other", "seat": "elsewhere", "phase": "p"}),
            T0,
        ),
        ev(2, EventType::EffectRequested, json!({"seat": "ghost"}), T0),
        ev(3, EventType::EffectStarted, json!({}), T0),
        ev(
            4,
            EventType::EffectCheckpointed,
            json!({"checkpoint": {"step": "seat-turn", "turn": 1, "tool": "Grep"}}),
            T0,
        ),
    ];
    let view = run_view(&events, None);
    assert_eq!(view.participants.len(), 1, "no effect id, no participant");
    assert_eq!(view.participants[0].label, "elsewhere");
    assert_eq!(view.live[0].text, "ghost · turn 1 · Grep");
}
