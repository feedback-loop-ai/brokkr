//! Decision 0065 slice two, U4e (CC1, CC3 and SC4): the engine consumes a
//! driver's private observation at the single-site and panel sinks. A real
//! driver process streams deterministic observations through the engine
//! into the store's v6 append fence: a call the selected candidate holds is
//! journaled with the engine's whole group under an attempt-owned call id,
//! a local call stays ordinary, a known call no holding admits fails the
//! attempt, and neither a primary's nor a sibling's holding attributes
//! anything. Every journal exports and verifies offline.

use super::*;
use brokkr_protocol::adapters::capability_calls::{Format, Observation, Tool, OBSERVATION_KEY};

const DEADLINE: std::time::Duration = std::time::Duration::from_secs(10);

/// The site `label`'s holdings: the Codex primary holds `web-search`
/// through `codex-native-search`, whose one tool is `web_search`; the DSH
/// fallback holds nothing and its inventory knows nothing.
fn searching(label: &str) -> SiteCapabilities {
    holdings(label, json!({"web-search": "wants"}), None)
}

/// The observation of Codex item `call` running `name`.
fn observation(call: Option<&str>, name: &str) -> Value {
    serde_json::to_value(Observation {
        format: Format::Codex,
        call: call.map(str::to_string),
        tool: Tool::Named { name: name.into() },
    })
    .unwrap()
}

/// A Codex `item-completed` row, as the shipped lowering writes it.
fn row(tool: &str) -> Value {
    json!({"step": "item-completed", "turn": 1, "tool": tool, "harness": "codex"})
}

/// `row(tool)` with `fields` beside it.
fn row_with(tool: &str, fields: Value) -> Value {
    let mut row = row(tool);
    let fields = fields.as_object().unwrap().clone();
    row.as_object_mut().unwrap().extend(fields);
    row
}

/// `row(tool)` carrying `observation`, and a complete forged group the
/// driver has no authority to write.
fn observed(tool: &str, observation: Value) -> Value {
    let mut observed = row_with(
        tool,
        json!({"capability": "web-fetch", "dialect": "claude-native-fetch",
               "call_id": "forged", "call_state": "succeeded",
               "response_sha256": "a".repeat(64)}),
    );
    observed[OBSERVATION_KEY] = observation;
    observed
}

/// The site stamps a planned fixture site carries.
const SITE: &str = "5e11000000000000000000000000000000000000000000000000000000000011";
const INSTANCE: &str = "1a22000000000000000000000000000000000000000000000000000000000022";

/// A plan offering nothing, at the site whose engine stamps are `site`
/// and `instance`.
fn planned(site: &str, instance: &str) -> SitePlan {
    let context = resume::SiteContext {
        site_ref: site.into(),
        instance_ref: instance.into(),
        class: SeatClass::Work,
    };
    SitePlan {
        context,
        offer: None,
        assessment: Value::Null,
        originating: None,
        route_overlay: None,
    }
}

/// The native call id the design spells: `n-` and the SHA-256 of the
/// canonical attempt, site, instance, provider and harness call id.
fn native(site: &str, instance: &str, provider: &str, call: &str) -> String {
    let tuple = json!({"attempt": "attempt", "site": site, "instance": instance,
                       "provider": provider, "call": call});
    format!("n-{}", brokkr_core::canonical::sha256_hex(&tuple))
}

/// `row("web_search")` with the engine's group for `call_id`.
fn attributed(call_id: &str) -> Value {
    row_with(
        "web_search",
        json!({"capability": "web-search", "dialect": "codex-native-search",
               "call_id": call_id, "call_state": "observed"}),
    )
}

/// The held search, observed as Codex item `item_1`.
fn search() -> Value {
    observed("web_search", observation(Some("item_1"), "web_search"))
}

/// An engine whose seat `work` has its effect requested and one attempt
/// started, as the loop journals them, so a checkpoint folds in flight.
fn in_flight() -> (tempfile::TempDir, Engine, String) {
    let (dir, mut engine) = canonical_engine(single_body(vec!["driver".into()]));
    engine
        .enter_phase("work", &state(None, Cursor::Start))
        .unwrap();
    engine
        .request_or_finish(&state(Some("work"), Cursor::RequestEffect))
        .unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    let effect = events.last().unwrap().payload["effect_id"].clone();
    let effect = effect.as_str().unwrap().to_string();
    let started = json!({"effect_id": effect, "attempt_id": "attempt", "driver": "d"});
    let attempt = Some("attempt".to_string());
    engine
        .append(EventType::EffectStarted, started, attempt)
        .unwrap();
    (dir, engine, effect)
}

/// A link of `provider`/`model` whose driver streams `rows` and succeeds.
fn streaming(provider: &str, model: &str, effect: &str, rows: &[Value]) -> Candidate {
    let outcome = AttemptOutcome::Succeeded {
        result: json!({"result": "complete", "notes": "ran"}),
    };
    templated(Candidate {
        argv: super::super::tests::checkpointing_command(effect, "attempt", rows, outcome),
        ..link(provider, model)
    })
}

/// The checkpoints `engine` journaled, after its whole journal exported
/// and verified offline.
fn journaled(engine: &Engine) -> Vec<Value> {
    let exported = engine.store.export_ndjson(&engine.run_id).unwrap();
    let events = brokkr_store::verified_events(&exported).unwrap().0;
    events
        .into_iter()
        .filter(|event| event.event_type == EventType::EffectCheckpointed)
        .map(|event| event.payload["checkpoint"].clone())
        .collect()
}

/// One attempt at site `work`, which holds `capabilities`, run by the
/// `provider`/`model` candidate whose driver streams `rows`, its spawn
/// sealed as dispatch seals it and planned with the `SITE` and `INSTANCE`
/// stamps: what it journaled, and its report.
fn served(
    capabilities: SiteCapabilities,
    (provider, model): (&str, &str),
    rows: &[Value],
) -> (Vec<Value>, AttemptReport) {
    served_at(
        capabilities,
        (provider, model),
        rows,
        Some(&planned(SITE, INSTANCE)),
    )
}

/// `served`, at a site planned by `plan`, or by none.
fn served_at(
    capabilities: SiteCapabilities,
    (provider, model): (&str, &str),
    rows: &[Value],
    plan: Option<&SitePlan>,
) -> (Vec<Value>, AttemptReport) {
    let (_dir, mut engine, effect) = in_flight();
    let site = engine.bundle.sites.entry("work".into()).or_default();
    site.capabilities = Some(capabilities);
    let link = streaming(provider, model, &effect, rows);
    let (spawn, input) = marked(&engine, &link, json!({}));
    let run = engine.run_driver(
        &effect, "attempt", "work", &spawn, input, DEADLINE, None, plan,
    );
    let Ok(DriverRun::Ran(report)) = run else {
        panic!("the fixture driver spawns");
    };
    (journaled(&engine), report)
}

/// The selected Codex candidate's held search is journaled with the
/// engine's group, its forged group gone; a local call and a legacy row
/// stay ordinary, a spoofed group on the local call included.
#[test]
fn a_held_call_is_attributed_to_the_selected_holding_and_local_calls_stay_ordinary() {
    let local = observation(Some("item_2"), "command_execution");
    let rows = [
        search(),
        observed("command_execution", local),
        row("web_search"),
    ];
    let (journaled, report) = served(searching("work"), ("codex", "astra"), &rows);
    let local = row("command_execution");
    let call_id = native(SITE, INSTANCE, "codex", "item_1");
    let wanted = [attributed(&call_id), local, row("web_search")];
    assert_eq!(journaled, wanted);
    assert_eq!(report.refused, None);
    // The tuple's canonical bytes, pinned once.
    let pinned = "n-bd4253357a0ad826b23b1b22fb3ab73c28fe5dc7637dab75a4aa23b4e52597b3";
    assert_eq!(call_id, pinned);
}

/// A call id is owned by its structural site: the same harness id at
/// another site, or another instance of this one, is another call; and a
/// harness id of any length yields a call id seat-record v6 carries.
#[test]
fn a_call_id_is_owned_by_its_site_and_bounded_whatever_the_harness_id() {
    let long = "item_".repeat(40);
    let other = "0".repeat(64);
    let sites = [(SITE, INSTANCE, long.as_str()), (SITE, INSTANCE, "item_1")];
    let sites = sites.into_iter().chain([(SITE, other.as_str(), "item_1")]);
    let mut seen = std::collections::BTreeSet::new();
    for (site, instance, call) in sites.chain([(other.as_str(), INSTANCE, "item_1")]) {
        let search = observed("web_search", observation(Some(call), "web_search"));
        let plan = planned(site, instance);
        let at = ("codex", "astra");
        let (journaled, report) = served_at(searching("work"), at, &[search], Some(&plan));
        let call_id = native(site, instance, "codex", call);
        assert_eq!(
            (journaled, report.refused),
            (vec![attributed(&call_id)], None)
        );
        assert_eq!(call_id.len(), 66);
        seen.insert(call_id);
    }
    assert_eq!(seen.len(), 4);
}

/// The DSH fallback serves the attempt: the search its primary holds is no
/// holding of its own, and its inventory knows nothing, so the same call
/// is ordinary.
#[test]
fn a_fallback_never_borrows_its_primarys_holding() {
    let (journaled, report) = served(searching("work"), ("dsh", "flash"), &[search()]);
    assert_eq!(journaled, [row("web_search")]);
    assert_eq!(report.refused, None);
}

/// A known tool no holding admits fails the attempt in CC1's words; so does
/// a held call with no harness id, an empty one, one at a site with no
/// engine stamps to own it, or an observation that cannot be read. Rows
/// before the refused call land; nothing after it.
#[test]
fn an_unheld_or_unattributable_call_fails_its_attempt() {
    let mut unreadable = observation(Some("item_1"), "web_search");
    unreadable["capability"] = json!("web-search");
    let unattributable = "capability telemetry cannot be attributed";
    let plan = planned(SITE, INSTANCE);
    let item = || observation(Some("item_1"), "web_search");
    let cases = [
        (
            two_candidates(),
            item(),
            Some(&plan),
            "observed capability tool 'web_search' is not held by this attempt",
        ),
        (
            searching("work"),
            observation(None, "web_search"),
            Some(&plan),
            unattributable,
        ),
        (
            searching("work"),
            observation(Some(""), "web_search"),
            Some(&plan),
            unattributable,
        ),
        (searching("work"), item(), None, unattributable),
        (searching("work"), unreadable, Some(&plan), unattributable),
    ];
    for (capabilities, observation, plan, cause) in cases {
        let rows = [
            row("first"),
            observed("web_search", observation),
            row("later"),
        ];
        let at = ("codex", "astra");
        let (journaled, report) = served_at(capabilities, at, &rows, plan);
        assert_eq!(journaled, [row("first")], "{cause}");
        let refused = AttemptOutcome::Failed {
            error: cause.into(),
        };
        assert_eq!(report.refused, Some(refused));
    }
}

/// An `mcp` holding attributes no native call (CC1 tells its tools apart
/// by their `cap-` server), so the search the inventory knows is unheld;
/// and an MCP call the harness reports is ordinary, its evidence the
/// broker's ledger (CC2). Either way the observation is taken off.
#[test]
fn an_mcp_holding_or_an_mcp_call_attributes_nothing() {
    let (_dir, mut engine, effect) = in_flight();
    let mut capabilities = searching("work");
    let holding = capabilities.outcomes[0].held.get_mut("web-search").unwrap();
    holding.implementation = crate::capabilities::Implementation::Mcp {
        server: "cap-web-search".into(),
        connection: crate::capabilities::Connection::Stdio(vec!["docs-server".into()]),
        version: "1.0.0".into(),
        secrets: Vec::new(),
    };
    let site = engine.bundle.sites.entry("work".into()).or_default();
    site.capabilities = Some(capabilities);
    let link = streaming("codex", "astra", &effect, &[]);
    let (spawn, _) = marked(&engine, &link, json!({}));
    let plan = planned(SITE, INSTANCE);
    let facts = engine.bundle.sites.get("work");
    let calls =
        super::super::capability_calls::Calls::of(facts, &spawn, "attempt", Some(&plan.context));
    let mcp = serde_json::to_value(Observation {
        format: Format::Codex,
        call: Some("item_3".into()),
        tool: Tool::Mcp {
            server: "cap-web-search".into(),
            tool: "web_search".into(),
        },
    })
    .unwrap();
    let mut called = observed("mcp_tool_call", mcp);
    assert_eq!(calls.consume(&mut called), Ok(None));
    let mut searched = search();
    let unheld = super::super::capability_calls::Refusal::Unheld {
        tool: "web_search".into(),
    };
    assert_eq!(calls.consume(&mut searched), Err(unheld));
    for checkpoint in [called, searched] {
        assert_eq!(checkpoint.get(OBSERVATION_KEY), None);
    }
}

/// Two panel members observe the same search: the Codex member's call is
/// attributed to its own holding under a call id its member owns, and the
/// DSH sibling's is ordinary.
#[test]
fn a_panel_member_is_attributed_by_its_own_holding_never_its_siblings() {
    let (_dir, mut engine, effect) = in_flight();
    for name in ["searcher", "sibling"] {
        let label = format!("work:{name}");
        let site = engine.bundle.sites.entry(label.clone()).or_default();
        site.capabilities = Some(searching(&label));
    }
    let rows = [search()];
    let mut selection = Selection::new();
    selection.insert(
        Some("searcher".into()),
        streaming("codex", "astra", &effect, &rows),
    );
    selection.insert(
        Some("sibling".into()),
        streaming("dsh", "flash", &effect, &rows),
    );
    let sibling_site = "0".repeat(64);
    let plans = [("searcher", SITE), ("sibling", &sibling_site)];
    for (name, site) in plans {
        let plan = planned(site, INSTANCE);
        selection.plans.insert(Some(name.into()), plan);
    }
    let members = [
        member("searcher", Vec::new()),
        member("sibling", Vec::new()),
    ];
    let input = super::super::tests::panel_input(&["searcher", "sibling"]);
    let aggregate = Aggregate::UnanimousPass;
    engine
        .execute_panel(
            &effect, "attempt", "work", &members, aggregate, &input, DEADLINE, &selection, false,
        )
        .unwrap();
    // Members run side by side, so only each member's own rows are ordered.
    let mut live: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for row in journaled(&engine) {
        let member = row["member"].as_str().unwrap().to_string();
        live.entry(member).or_default().push(row);
    }
    let finished = |member: &str| {
        json!({"step": "panel-member-finished", "member": member, "outcome": "succeeded",
               "session_ref": "session", "inner_checkpoints": 1, "model": "not reported",
               "boundary": "not applicable"})
    };
    let mut searcher = attributed(&native(SITE, INSTANCE, "codex", "item_1"));
    searcher["member"] = json!("searcher");
    let mut sibling = row("web_search");
    sibling["member"] = json!("sibling");
    let wanted = BTreeMap::from([
        ("searcher".to_string(), vec![searcher, finished("searcher")]),
        ("sibling".to_string(), vec![sibling, finished("sibling")]),
    ]);
    assert_eq!(live, wanted);
}
