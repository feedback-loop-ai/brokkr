use super::*;
use brokkr_core::fold::Cursor;
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
        visits: BTreeMap::new(),
        last_result: None,
        reviewed_heads: None,
        last_decision,
        park_reason: Some("needs a human".to_string()),
        feature: Some("feat".to_string()),
        pending_command: None,
        riding_stop: false,
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
                                  "target": "crates/brokkr-view/src/lib.rs"}}),
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
    // Structural, not conventional: `brokkr-view`'s manifest depends on
    // exactly brokkr-core, serde and serde_json, and these tokens are the
    // ways a derivation quietly acquires a side effect.
    for source in [include_str!("lib.rs"), include_str!("js.rs")] {
        for banned in ["std::fs", "std::env", "IsTerminal", "print!", "println!"] {
            assert!(
                !source.contains(banned),
                "brokkr-view must not reach for {banned}"
            );
        }
    }
    let manifest = include_str!("../Cargo.toml");
    let deps = manifest.split("[dependencies]").nth(1).unwrap();
    for allowed in ["brokkr-core", "serde", "serde_json"] {
        assert!(deps.contains(allowed), "{allowed} is a dependency");
    }
    for forbidden in ["brokkr-store", "time", "rusqlite", "clap"] {
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
        short_target("crates/brokkr-view/src/derivation/participants/activity.rs"),
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
            detail: Some("event 93: OperatorAccepted is impossible at cursor EffectInFlight"),
        },
        RunEntry {
            run_id: "new",
            feature: "the newer feature",
            created_at: T1,
            state: Some(&running),
            detail: None,
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
    // …and it says why, in the fold's own words. A row that reads `?`
    // with no reason is the same blindness one row further in.
    assert_eq!(
        json["runs"][1]["detail"],
        "event 93: OperatorAccepted is impossible at cursor EffectInFlight"
    );
    assert_eq!(json["runs"][0]["detail"], Value::Null);
}

/// A run whose journal does not fold is the loudest thing in a fleet:
/// it travels as a finding, cited by the sequence the fold refused at,
/// so the operator's aide can propose about it instead of losing it.
#[test]
fn a_quarantined_run_becomes_a_finding_that_cites_where_the_fold_stopped() {
    let finding = quarantine_finding(
        "tui-graph-run",
        93,
        "event 93: OperatorAccepted is impossible at cursor EffectInFlight",
    );
    assert_eq!(finding.run_id, "tui-graph-run");
    assert_eq!(finding.seq, 93);
    assert_eq!(finding.input, "journal_folds");
    assert_eq!(finding.value, "false");
    // The evaluator ruled nothing here: the claim is the fleet read's.
    assert_eq!(finding.phase, ABSENT);
    assert_eq!(finding.rule_id, ABSENT);
    assert_eq!(
        finding.line,
        "tui-graph-run seq 93 · journal does not fold · \
         event 93: OperatorAccepted is impossible at cursor EffectInFlight"
    );
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
        "crates/brokkr-view/src/lib.rs"
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

// ------------------------------------------- tokens where dollars aren't

/// A codex seat as `docs/evidence/brokkr-export-gains-redact-a-san-c5d011df.ndjson`
/// journals one: `turn-started`/`turn-completed` where claude journals
/// `seat-turn`, per-turn token counts, and — the whole point — no
/// `total_cost_usd` anywhere, because a subscription harness reports no
/// marginal price and the adapter invents none.
///
/// Each element is one turn's `turn-completed` usage. The finished
/// checkpoint is built the way the adapter builds it: its session meta
/// is an insert, not an accumulate, so it ends up carrying the LAST
/// turn's counts while calling them the session's.
fn codex_journal(turns: Vec<Value>) -> Vec<EventEnvelope> {
    let mut events = vec![
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
    ];
    let mut session = json!({"step": "codex-session-finished", "exit_code": 0,
                             "session_id": "01a05992-e55c-7fc1-852a-0bbe590fcc2e"});
    for (index, usage) in turns.into_iter().enumerate() {
        let turn = index as u64 + 1;
        let seq = events.len() as u64 + 1;
        events.push(ev(
            seq,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "attempt_id": "att1",
                   "checkpoint": {"harness": "codex", "step": "turn-started", "turn": turn}}),
            T0,
        ));
        let mut checkpoint = usage;
        checkpoint["harness"] = json!("codex");
        checkpoint["step"] = json!("turn-completed");
        checkpoint["turn"] = json!(turn);
        for key in ["input_tokens", "cache_read_tokens", "output_tokens"] {
            if let Some(value) = checkpoint.get(key) {
                session[key] = value.clone();
            }
        }
        let seq = events.len() as u64 + 1;
        events.push(ev(
            seq,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "attempt_id": "att1", "checkpoint": checkpoint}),
            T0,
        ));
    }
    let seq = events.len() as u64 + 1;
    events.push(ev(
        seq,
        EventType::EffectCheckpointed,
        json!({"effect_id": "eff1", "attempt_id": "att1", "checkpoint": session}),
        T1,
    ));
    let seq = events.len() as u64 + 1;
    events.push(ev(
        seq,
        EventType::EffectSucceeded,
        json!({"effect_id": "eff1", "attempt_id": "att1", "result": {"result": "complete"}}),
        T2,
    ));
    events
}

/// A two-member panel, each member's telemetry given as the
/// `turn-completed`-or-`seat-turn` checkpoint it journals and the
/// finished checkpoint that closes it.
fn panel_of(members: Vec<(&str, Value, Value)>) -> Vec<EventEnvelope> {
    let mut events = vec![
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
    ];
    for (member, working, finished) in members {
        for mut checkpoint in [working, finished] {
            checkpoint["member"] = json!(member);
            let seq = events.len() as u64 + 1;
            events.push(ev(
                seq,
                EventType::EffectCheckpointed,
                json!({"effect_id": "eff1", "attempt_id": "att1", "checkpoint": checkpoint}),
                T0,
            ));
        }
    }
    let seq = events.len() as u64 + 1;
    events.push(ev(
        seq,
        EventType::EffectSucceeded,
        json!({"effect_id": "eff1", "attempt_id": "att1", "result": {"result": "designed"}}),
        T2,
    ));
    events
}

#[test]
fn a_codex_seat_shows_its_tokens_where_a_price_would_be() {
    // The wager's crew-B numbers exactly: 3,975,322 input tokens, of
    // which 3,830,272 were cache reads, and 14,051 output. The cache
    // reads sit INSIDE the input count — a cache hit is still an input
    // token, billed differently — so the total is input + output only.
    let events = codex_journal(vec![json!({
        "input_tokens": 3_975_322, "cache_read_tokens": 3_830_272, "output_tokens": 14_051
    })]);
    let view = run_view(&events, None);
    let part = &view.participants[0];
    assert_eq!(part.cost, None, "no price was reported, so none is claimed");
    assert_eq!(part.cost_cell.text, "3.99M tok");
    assert!(!part.cost_cell.absent);
    assert_eq!(part.cost_cell.note, None);
    assert!(
        !part.cost_cell.text.contains('$'),
        "a token count is never money"
    );
    // The turns gap: not one `seat-turn` in this journal, and the seat
    // shows the turn it took rather than an absence mark.
    assert_eq!(part.turns, Some(1));
    assert_eq!(part.turns_cell.text, "1");
    assert!(!part.turns_cell.absent);
    assert_eq!(
        part.session_id.as_deref(),
        Some("01a05992-e55c-7fc1-852a-0bbe590fcc2e")
    );
}

#[test]
fn a_multi_turn_codex_seat_sums_its_turns_rather_than_its_last_one() {
    let events = codex_journal(vec![
        json!({"input_tokens": 200_000, "cache_read_tokens": 180_000, "output_tokens": 5_000}),
        json!({"input_tokens": 100_000, "cache_read_tokens": 90_000, "output_tokens": 7_000}),
    ]);
    let view = run_view(&events, None);
    let part = &view.participants[0];
    // 205,000 + 107,000. The finished checkpoint carries only turn two's
    // 107,000: reading the tokens off the session the way the cost is
    // read would print `107k tok` and lose the first turn entirely.
    assert_eq!(part.cost_cell.text, "312k tok");
    assert_eq!(
        part.turns,
        Some(2),
        "the highest turn number, as with seat-turn"
    );
    assert_eq!(part.turns_cell.text, "2");
}

#[test]
fn a_codex_seat_with_no_usage_journaled_keeps_the_absence_mark() {
    // A turn ran and closed and the harness counted nothing with it.
    // There is no price and no token count, so the cell says nothing —
    // which is the honest answer, not a zero.
    let events = codex_journal(vec![json!({})]);
    let view = run_view(&events, None);
    let part = &view.participants[0];
    assert_eq!(part.cost, None);
    assert_eq!(part.cost_cell.text, ABSENT);
    assert!(part.cost_cell.absent);
    assert_eq!(
        part.cost_cell.note.as_deref(),
        Some("no session cost or token usage recorded")
    );
    assert_eq!(
        part.turns_cell.text, "1",
        "a turn that counted no tokens is still a turn"
    );
}

#[test]
fn a_priced_seat_shows_dollars_even_when_tokens_are_journaled_too() {
    let mut events = codex_journal(vec![json!({
        "input_tokens": 300_000, "cache_read_tokens": 250_000, "output_tokens": 12_000
    })]);
    // The same seat on a harness that DOES report a price. The price is
    // the answer; the token count is what fills the cell in its absence,
    // never a second opinion printed beside it.
    let session = events
        .iter_mut()
        .find(|event| event.payload["checkpoint"]["step"] == "codex-session-finished")
        .expect("the helper journals a finished session");
    session.payload["checkpoint"]["total_cost_usd"] = json!(0.03125);
    let view = run_view(&events, None);
    let part = &view.participants[0];
    assert_eq!(part.cost, Some(0.03125));
    assert_eq!(part.cost_cell.text, "$0.0313");
    assert!(
        !part.cost_cell.text.contains("tok"),
        "dollars and tokens never share a cell"
    );
}

#[test]
fn a_panel_of_unpriced_members_sums_their_tokens_with_a_sigma() {
    let events = panel_of(vec![
        (
            "simplicity",
            json!({"step": "turn-completed", "turn": 1,
                   "input_tokens": 200_000, "output_tokens": 12_000}),
            json!({"step": "codex-session-finished", "session_id": "s1"}),
        ),
        (
            "robustness",
            json!({"step": "turn-completed", "turn": 1,
                   "input_tokens": 290_000, "output_tokens": 10_000}),
            json!({"step": "codex-session-finished", "session_id": "s2"}),
        ),
    ]);
    let view = run_view(&events, None);
    let parent = &view.participants[0];
    assert_eq!(parent.cost_cell.text, "Σ 512k tok");
    assert!(
        !parent.cost_aggregated,
        "there was no cost to aggregate — the Σ here is over tokens"
    );
    assert_eq!(parent.cost, None);
    assert_eq!(
        view.participants[1].cost_cell.text, "212k tok",
        "no Σ on a member"
    );
    assert_eq!(view.participants[2].cost_cell.text, "300k tok");
}

#[test]
fn a_parent_with_its_own_tokens_does_not_aggregate_its_members() {
    let mut events = panel_of(vec![
        (
            "simplicity",
            json!({"step": "turn-completed", "turn": 1,
                   "input_tokens": 200_000, "output_tokens": 12_000}),
            json!({"step": "codex-session-finished", "session_id": "s1"}),
        ),
        (
            "robustness",
            json!({"step": "turn-completed", "turn": 1,
                   "input_tokens": 290_000, "output_tokens": 10_000}),
            json!({"step": "codex-session-finished", "session_id": "s2"}),
        ),
    ]);
    // The seat counted tokens of its own. Its own count is the answer —
    // a Σ over its members would be a second, different claim about the
    // same seat, which is what the cost rule already refuses.
    let seq = events.len() as u64 + 1;
    events.push(ev(
        seq,
        EventType::EffectCheckpointed,
        json!({"effect_id": "eff1", "attempt_id": "att1",
               "checkpoint": {"step": "turn-completed", "turn": 1,
                              "input_tokens": 40_000, "output_tokens": 2_000}}),
        T0,
    ));
    let view = run_view(&events, None);
    let parent = &view.participants[0];
    assert_eq!(parent.cost_cell.text, "42k tok");
    assert!(
        !parent.cost_cell.text.starts_with('Σ'),
        "its own telemetry, so nothing was aggregated"
    );
}

#[test]
fn a_sigma_whose_members_disagree_in_kind_stays_in_dollars() {
    let events = panel_of(vec![
        (
            "priced",
            json!({"step": "seat-turn", "turn": 1, "tool": "Write"}),
            json!({"step": "claude-session-finished", "session_id": "s1",
                   "total_cost_usd": 0.25}),
        ),
        (
            "unpriced",
            json!({"step": "turn-completed", "turn": 1,
                   "input_tokens": 290_000, "output_tokens": 10_000}),
            json!({"step": "codex-session-finished", "session_id": "s2"}),
        ),
    ]);
    let view = run_view(&events, None);
    let parent = &view.participants[0];
    // The Σ is a dollar total over the members that reported dollars. It
    // does not grow by a token and it does not print one: a roll-up that
    // mixed the units would be a number nothing in the world matches.
    assert_eq!(parent.cost_cell.text, "Σ $0.2500");
    assert!(parent.cost_aggregated);
    assert!(!parent.cost_cell.text.contains("tok"));
    // The unpriced member's tokens are not lost — they are on its own
    // row, in their own unit, one line below the Σ.
    assert_eq!(view.participants[2].cost_cell.text, "300k tok");
}

#[test]
fn the_token_humanizer_pins_its_three_tiers() {
    assert_eq!(fmt_tokens(0), "0 tok");
    assert_eq!(fmt_tokens(842), "842 tok");
    assert_eq!(fmt_tokens(999), "999 tok");
    // Half-up, the rounding every other humanized number here uses.
    assert_eq!(fmt_tokens(1_499), "1k tok");
    assert_eq!(fmt_tokens(1_500), "2k tok");
    assert_eq!(fmt_tokens(312_400), "312k tok");
    // The rounding picks the tier, so 999,999 reads as a million rather
    // than as the `1000k` a threshold check would print.
    assert_eq!(fmt_tokens(999_999), "1.00M tok");
    assert_eq!(fmt_tokens(3_989_373), "3.99M tok");
    assert_eq!(fmt_tokens(12_000_000), "12.00M tok");
    // Absurd input still reads as tokens and never as money.
    assert!(!fmt_tokens(u64::MAX).contains('$'));
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
                              "target": "crates/brokkr-view/src/derivation/participants/activity.rs"}}),
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
        Some("no session cost or token usage recorded")
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
fn phase_names_are_unique_within_a_run_view() {
    // `visits` is a fold keyed by name, so a revisited phase is ONE
    // segment of the rail. Every surface selects a phase BY NAME —
    // `brokkr inspect --phase`, the console's `selectedPhase`, and now
    // `brokkr tui`'s graph cursor — so two segments sharing a name would
    // scope the wrong one, invisibly. The coupling is named here rather
    // than left silent; it is an invariant of the derivation, and no
    // renderer may assume it without this test standing behind it.
    let events = vec![
        ev(1, EventType::PhaseEntered, json!({"phase": "intake"}), T0),
        ev(2, EventType::PhaseEntered, json!({"phase": "design"}), T0),
        ev(3, EventType::PhaseEntered, json!({"phase": "intake"}), T1),
        ev(4, EventType::PhaseEntered, json!({"phase": "design"}), T2),
    ];
    let view = run_view(&events, None);
    let names: Vec<&str> = view
        .phases
        .iter()
        .map(|phase| phase.name.as_str())
        .collect();
    assert_eq!(
        names,
        ["intake", "design"],
        "a revisited phase is one segment, in first-visit order"
    );
    let mut unique = names.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), names.len(), "no two phases share a name");
    assert_eq!(view.phases[0].visits, 2);
    assert_eq!(view.phases[1].visits, 2);

    // And the same over a journal that carries real structure, so the
    // invariant is not an artefact of a rail with nothing on it.
    let view = run_view(&panel_journal(), None);
    let mut names: Vec<&str> = view
        .phases
        .iter()
        .map(|phase| phase.name.as_str())
        .collect();
    let count = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), count, "and over a structured run too");
}

#[test]
fn the_rail_records_the_road_back_once_per_pair_and_only_where_it_was_taken() {
    // Decision 0022's reforging, as the journal writes it: review rules
    // a security residual and the run goes BACK to implement, twice.
    // The revisit count is not the fact — `visits > 1` is necessary and
    // not sufficient — so the pair comes from the ruling that caused it.
    let back = json!({"from": "review", "next": "implement",
                      "result": "residual", "rule_id": "REVIEW-REFORGE"});
    let events = vec![
        ev(1, EventType::PhaseEntered, json!({"phase": "intake"}), T0),
        ev(
            2,
            EventType::TransitionDecided,
            json!({"from": "intake", "next": "implement", "result": "resolved"}),
            T0,
        ),
        ev(
            3,
            EventType::PhaseEntered,
            json!({"phase": "implement"}),
            T0,
        ),
        ev(4, EventType::PhaseEntered, json!({"phase": "review"}), T0),
        ev(5, EventType::TransitionDecided, back.clone(), T1),
        ev(
            6,
            EventType::PhaseEntered,
            json!({"phase": "implement"}),
            T1,
        ),
        ev(7, EventType::PhaseEntered, json!({"phase": "review"}), T2),
        ev(8, EventType::TransitionDecided, back, T2),
        ev(
            9,
            EventType::PhaseEntered,
            json!({"phase": "implement"}),
            T2,
        ),
    ];
    let view = run_view(&events, None);
    let named: Vec<(&str, u64, Vec<&str>)> = view
        .phases
        .iter()
        .map(|phase| {
            (
                phase.name.as_str(),
                phase.visits,
                phase.returns.iter().map(String::as_str).collect(),
            )
        })
        .collect();
    assert_eq!(
        named,
        [
            ("intake", 1, vec![]),
            // Two reforgings, ONE road: the count is the `×N` marker's
            // to carry, never a second entry here.
            ("implement", 3, vec!["review"]),
            ("review", 2, vec![]),
        ]
    );

    // The forward transition that opened the run recorded nothing: at
    // the moment it was ruled, `implement` had not been entered.
    let forward = vec![
        ev(1, EventType::PhaseEntered, json!({"phase": "intake"}), T0),
        ev(
            2,
            EventType::TransitionDecided,
            json!({"from": "intake", "next": "implement", "result": "resolved"}),
            T0,
        ),
        ev(
            3,
            EventType::PhaseEntered,
            json!({"phase": "implement"}),
            T0,
        ),
    ];
    assert!(run_view(&forward, None)
        .phases
        .iter()
        .all(|phase| phase.returns.is_empty()));

    // A ruling that parks carries no `next`, and one whose `from` the
    // journal lost carries no departure: neither is repaired into a
    // road, and neither loses the events around it.
    let ruined = vec![
        ev(1, EventType::PhaseEntered, json!({"phase": "intake"}), T0),
        ev(
            2,
            EventType::TransitionDecided,
            json!({"from": "review", "next": null, "result": "residual"}),
            T0,
        ),
        ev(
            3,
            EventType::TransitionDecided,
            json!({"next": "intake", "result": "residual"}),
            T0,
        ),
    ];
    let view = run_view(&ruined, None);
    assert_eq!(view.phases.len(), 1);
    assert!(view.phases[0].returns.is_empty());
}

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

// ------------------------------- agent provenance and run-level notices

/// The journal an adopting run leaves: a first attempt that fails to
/// start, a second on the next link of the chain, and a compile-time
/// notice already carried inside `run/started`'s manifest.
fn adopting_journal() -> Vec<EventEnvelope> {
    vec![
        ev(
            1,
            EventType::RunStarted,
            json!({"feature": "build it", "manifest": {"agents": {
                "intake": {"notices": [
                    {"message": "optional capability gap: no MCP server 'github'"},
                ]},
            }}}),
            T0,
        ),
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
            json!({"effect_id": "eff1", "attempt_id": "a1", "driver": "d",
                   "provenance": [{"member": null, "agent": "intake",
                                   "model": "fable", "provider": "claude",
                                   "chain_index": 0}]}),
            T0,
        ),
        ev(
            5,
            EventType::EffectFailed,
            json!({"effect_id": "eff1", "attempt_id": "a1", "error": "no binary",
                   "start_failure": true, "start_failure_sites": [null]}),
            T1,
        ),
        ev(
            6,
            EventType::EffectStarted,
            json!({"effect_id": "eff1", "attempt_id": "a2", "driver": "d",
                   "provenance": [{"member": null, "agent": "intake",
                                   "model": "opus", "provider": "claude",
                                   "chain_index": 1}]}),
            T1,
        ),
        ev(
            7,
            EventType::EffectSucceeded,
            json!({"effect_id": "eff1", "attempt_id": "a2",
                   "result": {"result": "resolved"}}),
            T2,
        ),
    ]
}

/// The single derivation: one place turns a journaled record into the
/// sentence four surfaces print, and a fallback is named as a fallback.
#[test]
fn provenance_is_derived_once_and_names_a_fallback_as_one() {
    let view = run_view(&adopting_journal(), None);
    let seat = view
        .participants
        .iter()
        .find(|part| part.label == "intake")
        .expect("the seat participant");
    let provenance = seat.provenance.as_ref().expect("an adopting seat");
    assert_eq!(provenance.agent, "intake");
    // What ACTUALLY ran: the second attempt, on the second link.
    assert_eq!(provenance.model, "opus");
    assert_eq!(provenance.provider, "claude");
    assert_eq!(provenance.chain_index, 1);
    assert!(provenance.fallback);
    assert!(provenance.line.contains("intake · opus via claude"));
    assert!(provenance.line.contains("not the agent's first choice"));
}

/// A first-choice selection reads as a plain statement — the forge does
/// not decorate what it did not have to fall back from.
#[test]
fn a_first_choice_selection_carries_no_fallback_language() {
    let mut events = adopting_journal();
    events.truncate(5);
    events[4] = ev(
        5,
        EventType::EffectSucceeded,
        json!({"effect_id": "eff1", "attempt_id": "a1", "result": {"result": "resolved"}}),
        T1,
    );
    let view = run_view(&events, None);
    let provenance = view.participants[0].provenance.as_ref().unwrap();
    assert_eq!(provenance.chain_index, 0);
    assert!(!provenance.fallback);
    assert_eq!(provenance.line, "intake · fable via claude");
    assert!(!provenance.line.contains("fallback"));
}

/// A record missing every field still renders, and renders as absence
/// rather than as a confident lie.
#[test]
fn an_unreadable_provenance_record_reads_as_absence() {
    let derived = provenance_of(&json!({}));
    assert_eq!(derived.agent, "?");
    assert_eq!(derived.line, "? · ? via ?");
    assert!(!derived.fallback);
}

/// AC-17: both kinds of run-level fact surface as notices, deduplicated,
/// and an inline run has none at all.
#[test]
fn run_notices_carry_capability_gaps_and_fallbacks_without_duplicates() {
    let view = run_view(&adopting_journal(), None);
    assert_eq!(view.notices.len(), 2);
    assert_eq!(view.notices[0].kind, "capability-gap");
    assert!(view.notices[0]
        .text
        .starts_with("intake: optional capability gap"));
    assert_eq!(view.notices[1].kind, "fallback");
    assert!(view.notices[1].text.starts_with("seat: intake · opus"));

    // A second fallback attempt on the same link says it once.
    let mut repeated = adopting_journal();
    repeated.push(ev(
        8,
        EventType::EffectStarted,
        json!({"effect_id": "eff1", "attempt_id": "a3", "driver": "d",
               "provenance": [{"member": null, "agent": "intake",
                               "model": "opus", "provider": "claude",
                               "chain_index": 1}]}),
        T3,
    ));
    assert_eq!(run_view(&repeated, None).notices.len(), 2);

    // An inline run has neither kind, and its participants carry none.
    let inline = run_view(&seat_journal(), None);
    assert!(inline.notices.is_empty());
    assert!(inline.participants.iter().all(|p| p.provenance.is_none()));
}

/// A named site gets a participant even when it never checkpoints:
/// "which model served this member" is a fact about the attempt, not
/// about whether the member said anything.
#[test]
fn a_member_named_only_in_provenance_still_becomes_a_participant() {
    let events = vec![
        ev(1, EventType::RunStarted, json!({"feature": "f"}), T0),
        ev(2, EventType::PhaseEntered, json!({"phase": "review"}), T0),
        ev(
            3,
            EventType::EffectRequested,
            json!({"effect_id": "eff1", "seat": "review", "phase": "review"}),
            T0,
        ),
        ev(
            4,
            EventType::EffectStarted,
            json!({"effect_id": "eff1", "attempt_id": "a1", "driver": "panel[2]",
            "provenance": [
                {"member": "correctness", "agent": "review-correctness",
                 "model": "opus", "provider": "claude", "chain_index": 0},
                {"member": "security", "agent": "review-security",
                 "model": "sonnet", "provider": "other", "chain_index": 0},
            ]}),
            T0,
        ),
    ];
    let view = run_view(&events, None);
    let members: Vec<&Participant> = view
        .participants
        .iter()
        .filter(|part| part.member.is_some())
        .collect();
    assert_eq!(members.len(), 2);
    assert_eq!(members[0].provenance.as_ref().unwrap().provider, "claude");
    assert_eq!(members[1].provenance.as_ref().unwrap().provider, "other");
}

#[test]
fn a_working_seat_carries_its_session_id_from_the_started_checkpoint() {
    // The id used to arrive only with session-finished — at the END —
    // so a working seat's transcript could not be located, let alone
    // live-streamed. session-started journals it at init; the finished
    // checkpoint later replaces it, bringing the cost.
    let mut events = seat_journal();
    events.truncate(5); // requested, started, one turn — still working
    events.insert(
        4,
        ev(
            9,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "attempt_id": "att1",
                   "checkpoint": {"step": "session-started",
                                  "session_id": "sess-live"}}),
            T0,
        ),
    );
    let view = run_view(&events, Some(&state(Some("intake"), Status::Running, None)));
    let part = &view.participants[0];
    assert_eq!(part.status, "working");
    assert_eq!(
        part.session_id.as_deref(),
        Some("sess-live"),
        "a WORKING seat knows its session"
    );
    assert!(part.cost.is_none(), "no cost until the session finishes");

    // The full journal: finished replaces started and brings the cost.
    let whole = run_view(
        &seat_journal(),
        Some(&state(Some("intake"), Status::Completed, None)),
    );
    let part = &whole.participants[0];
    assert_eq!(part.session_id.as_deref(), Some("sess-1"));
    assert!(part.cost.is_some());

    // A RETRY: attempt two's started arrives after attempt one's
    // finished, and the LIVE session is the one the drill must stream
    // — never the dead attempt's transcript.
    let mut retried = seat_journal();
    retried.truncate(6); // through attempt one's session-finished
    retried.push(ev(
        10,
        EventType::EffectStarted,
        json!({"effect_id": "eff1", "attempt_id": "att2"}),
        T2,
    ));
    retried.push(ev(
        11,
        EventType::EffectCheckpointed,
        json!({"effect_id": "eff1", "attempt_id": "att2",
               "checkpoint": {"step": "session-started",
                              "session_id": "sess-2-live"}}),
        T2,
    ));
    let view = run_view(
        &retried,
        Some(&state(Some("intake"), Status::Running, None)),
    );
    assert_eq!(
        view.participants[0].session_id.as_deref(),
        Some("sess-2-live"),
        "the retry's live session replaces the dead attempt's"
    );
}

#[test]
fn only_a_parked_run_admits_an_operator_command() {
    assert_eq!(
        operator_commands("awaiting_operator"),
        vec!["retry".to_string(), "stop".to_string()]
    );
    for status in ["running", "completed", "stopped"] {
        assert!(
            operator_commands(status).is_empty(),
            "{status} admits no operator command"
        );
    }
}

#[test]
fn residual_findings_come_from_the_structured_rule_inputs_only() {
    let events = vec![
        // Not a ruling at all.
        ev(1, EventType::PhaseEntered, json!({"phase": "verify"}), T0),
        // A ruling from a phase that carries no residuals.
        ev(
            2,
            EventType::TransitionDecided,
            json!({"from": "implement", "rule_id": "IMPL-OK",
                   "inputs": {"max_residual_severity": "high"}}),
            T0,
        ),
        // A ruling with no `from` at all.
        ev(
            3,
            EventType::TransitionDecided,
            json!({"rule_id": "ORPHAN", "inputs": {"has_security_residual": true}}),
            T0,
        ),
        // A verify ruling whose inputs are not an object.
        ev(
            4,
            EventType::TransitionDecided,
            json!({"from": "verify", "rule_id": "VERIFY-PASS", "inputs": "none"}),
            T0,
        ),
        // Every arm of the vocabulary, in one review ruling: a ranked
        // severity above `none`, a true flag, a false flag, a severity
        // of `none`, an unranked severity name, a non-string severity,
        // and a key the vocabulary does not read.
        ev(
            5,
            EventType::TransitionDecided,
            json!({"from": "review", "rule_id": null,
            "inputs": {
                "max_residual_severity": "high",
                "has_security_residual": true,
                "high_risk_uncovered": false,
                "fixes_applied": true,
            }}),
            T0,
        ),
        ev(
            6,
            EventType::TransitionDecided,
            json!({"from": "verify", "rule_id": "VERIFY-PASS",
                   "inputs": {"max_residual_severity": "none"}}),
            T0,
        ),
        ev(
            7,
            EventType::TransitionDecided,
            json!({"from": "verify", "rule_id": "VERIFY-PASS",
                   "inputs": {"max_residual_severity": "enormous"}}),
            T0,
        ),
        ev(
            8,
            EventType::TransitionDecided,
            json!({"from": "verify", "rule_id": "VERIFY-PASS",
                   "inputs": {"max_residual_severity": 3}}),
            T0,
        ),
    ];
    let findings = residual_findings("r1", &events);
    let lines: Vec<&str> = findings.iter().map(|f| f.line.as_str()).collect();
    assert_eq!(
        lines,
        vec![
            "r1 seq 5 · review · ? · has_security_residual: true",
            "r1 seq 5 · review · ? · max_residual_severity: high",
        ],
        "only the ranked severity and the true flag survive"
    );
    assert_eq!(findings[0].run_id, "r1");
    assert_eq!(findings[0].seq, 5);
    assert_eq!(findings[0].phase, "review");
    assert_eq!(findings[1].input, "max_residual_severity");
    assert_eq!(findings[1].value, "high");

    // The other true flag, and a named rule, so the rendered sentence is
    // read with a rule id as well as without one.
    let uncovered = vec![ev(
        1,
        EventType::TransitionDecided,
        json!({"from": "review", "rule_id": "REVIEW-RESIDUAL-OK",
               "inputs": {"high_risk_uncovered": true}}),
        T0,
    )];
    assert_eq!(
        residual_findings("r2", &uncovered)[0].line,
        "r2 seq 1 · review · REVIEW-RESIDUAL-OK · high_risk_uncovered: true"
    );
    assert!(residual_findings("r3", &[]).is_empty());
}
