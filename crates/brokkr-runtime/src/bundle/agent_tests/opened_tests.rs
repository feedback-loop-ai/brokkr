//! What a compile opens, and when (#349, ask 6): the adapters on the first
//! site that consults them, the library on the first agent reference, each
//! at most once. Every refusal here was measured on the compiler that
//! scanned the raw seats for these needs before parsing any, and is frozen
//! as that compiler wrote it.

use super::*;

/// The adapters' own refusal, after the loader's words.
const NEEDED: &str = "the adapter data is where a driver's model mapping (decision 0016) and its \
                      trust tier and binding grant (decision 0021) are declared, and this bundle \
                      names an agent, seats a gate, declares a secret binding or declares typed \
                      tools";

/// An inline seat on the bare `driver` program, which consults nothing.
fn bare(results: &str) -> Value {
    json!({"results": [results], "role": "roles/work.md", "driver": {"command": ["driver"]}})
}

/// An inline seat on a built-in model driver, pinned on both axes.
fn claude_seat(results: &str) -> Value {
    let command = [
        "{brokkr}",
        "driver",
        "claude",
        "--",
        "--model",
        "claude-opus-5",
        "--effort",
        "high",
    ];
    json!({"results": [results], "role": "roles/work.md", "driver": {"command": command}})
}

/// The same seat on LaneTally, a harness known to carry no native power.
fn lanetally_seat() -> Value {
    let mut seat = claude_seat("complete");
    seat["driver"]["command"][2] = json!("lanetally");
    seat
}

/// The two seats as given, under the fixture's policy.
fn seats(work: Value, review: Value) -> Value {
    json!({"name": "fixture", "policy": "policy.json", "seats": {"work": work, "review": review}})
}

/// A policy whose first phase is one a dialect fills.
fn dialect_policy() -> Value {
    json!({
        "phases": ["clarify", "review", "done"],
        "initial": "clarify",
        "terminal": ["done"],
        "rules": [
            {"id":"C", "from":"clarify", "result":"complete", "next":"review", "reason":"c"},
            {"id":"R", "from":"review", "result":"clean", "next":"done", "reason":"r"},
        ],
    })
}

/// The fixture with its one adapter file malformed: a present root that
/// does not load.
fn unloadable() -> (AgentFixture, String) {
    let fixture = AgentFixture::new();
    std::fs::write(fixture.adapters().join("claude.json"), "{").unwrap();
    let words = format!(
        "{}: EOF while parsing an object at line 1 column 1",
        fixture.adapters().join("claude.json").display()
    );
    (fixture, words)
}

/// Each reason a site consults the adapters, alone in its bundle with an
/// adapters root that does not load, refuses with the loader's words and
/// the same sentence; an inline model driver's optional read reaches
/// decision 0069's refusal of a present root instead; and an agent seat
/// over a library that does not load is the library's refusal.
#[test]
fn every_reason_to_open_refuses_an_unloadable_root_alone_as_it_always_did() {
    let mut rows = Vec::new();
    let needed = |words: &str| format!("bundle: {words}; {NEEDED}");
    let (fixture, words) = unloadable();
    let mut gate = bare("clean");
    gate["class"] = json!("gate");
    let mut secret = bare("complete");
    secret["secrets"] = json!(["TOKEN"]);
    let mut tools = bare("complete");
    tools["tools"] = json!({});
    let mut unbound = bare("complete");
    unbound["secrets"] = json!([]);
    // Two defects, the root before the pin walk's: a gate opens the
    // adapters before its route is read off a pin it does not write.
    let mut unpinned = claude_seat("clean");
    unpinned["class"] = json!("gate");
    unpinned["driver"]["command"] =
        json!(["{brokkr}", "driver", "claude", "--", "--effort", "high"]);
    for (label, config) in [
        ("agent seat", fixture.config()),
        ("gate-class site", seats(bare("complete"), gate)),
        ("secret binding", seats(secret, bare("clean"))),
        ("empty secret binding", seats(unbound, bare("clean"))),
        ("typed tools", seats(tools, bare("clean"))),
        (
            "gate-class site pinning no model",
            seats(bare("complete"), unpinned),
        ),
    ] {
        rows.push((
            label.to_string(),
            outcome(fixture.compile(config)),
            needed(&words),
        ));
    }
    let mut clarify = seats(bare("complete"), bare("clean"));
    clarify["seats"] = json!({"clarify": bare("complete"), "review": bare("clean")});
    rows.push((
        "dialect phase".to_string(),
        outcome(fixture.compile_with_policy(clarify, &dialect_policy())),
        needed(&words),
    ));
    rows.push((
        "inline model driver".to_string(),
        outcome(fixture.compile(seats(claude_seat("complete"), bare("clean")))),
        format!(
            "bundle: seat 'work' (office 'work') in realm '<unmapped>': provider 'claude' is \
             known to carry native capability 'web-search', which this seat does not hold, and \
             no valid control denies it: the adapter data could not be loaded ({words}). A known \
             native power is launched only with a delivered denial, never on what absence \
             implies; repair the adapter data (decision 0066 ruling 1)"
        ),
    ));
    // Decision 0069: a harness known to carry no native power passes the
    // capability pass, and the present root that did not load is refused
    // with the loader's words; an absent one declares nothing.
    rows.push((
        "inline model driver, no native power".to_string(),
        outcome(fixture.compile(seats(lanetally_seat(), bare("clean")))),
        format!("bundle: {words}"),
    ));
    std::fs::remove_dir_all(fixture.adapters()).unwrap();
    let absent = fixture.compile(seats(lanetally_seat(), bare("clean")));
    assert_eq!(absent.unwrap().name, "fixture");
    let library = AgentFixture::new();
    std::fs::write(library.library().join("worker.json"), "{").unwrap();
    rows.push((
        "agent seat, unloadable library".to_string(),
        outcome(library.compile(library.config())),
        format!(
            "bundle: {}: EOF while parsing an object at line 1 column 1",
            library.library().join("worker.json").display()
        ),
    ));
    each_row(rows);
}

/// An inline exec seat, which consults the adapters only for the tier.
fn exec_seat(results: &str) -> Value {
    let command = ["{brokkr}", "driver", "exec", "--", "true"];
    json!({"results": [results], "role": "roles/work.md", "driver": {"command": command}})
}

/// Decision 0075 ruling 5 judges a site that consults nothing else by the
/// adapters the compile opened for another, wherever that other site
/// stands: an exec seat ahead of the agent seat that opens them is refused
/// all the same, and with nothing opening them it is judged by nothing.
#[test]
fn a_site_ahead_of_the_one_that_opens_the_adapters_is_judged_by_them_all_the_same() {
    let fixture = AgentFixture::new();
    let mut adapter = claude();
    adapter["models"]["fresh"] = json!({"id": "fresh-1", "tier": "provisional"});
    fixture.write("adapters/claude.json", adapter);
    let agent = json!({"results": ["complete"], "agent": "worker"});
    assert_eq!(
        outcome(fixture.compile(seats(agent, exec_seat("clean")))),
        "bundle: seat 'review' pins no model, and adapter 'claude' declares a provisional model \
         the harness's own default may be; pin a promoted model's id on the adapter's model \
         flag, because a seat whose model the tier cannot read is refused, never read as \
         promoted (decision 0075 ruling 5)"
    );
    let unopened = fixture.compile(seats(bare("complete"), exec_seat("clean")));
    assert_eq!(unopened.unwrap().name, "fixture");
}

/// A site reached after an inline model driver's optional read failed is
/// judged by nothing, and the failure is decision 0069's to refuse.
#[test]
fn a_site_after_a_failed_optional_read_leaves_the_failure_to_decision_0069() {
    let (fixture, words) = unloadable();
    let mut lane = lanetally_seat();
    lane["results"] = json!(["clean"]);
    let refused = fixture.compile(seats(exec_seat("complete"), lane));
    assert_eq!(outcome(refused), format!("bundle: {words}"));
}

/// The one multi-defect order this slice moves, for the operator's ruling:
/// the adapters used to be loaded before any seat was parsed, so an
/// unloadable root refused first ("bundle: <loader's words>; <NEEDED>",
/// measured on df112346). They now open at the first site that consults
/// them, and a seat parsed before it speaks first. Each defect alone still
/// refuses as it did.
#[test]
fn a_seat_parsed_before_the_first_site_that_opens_the_adapters_refuses_first() {
    let (fixture, _) = unloadable();
    let mut review = bare("clean");
    review["results"] = json!([]);
    let agent = json!({"results": ["complete"], "agent": "worker"});
    assert_eq!(
        outcome(fixture.compile(seats(agent, review))),
        "bundle: seat 'review' needs non-empty 'results'"
    );
}

/// A review panel of two bare members, one NAMED `name`, and a policy that
/// routes both results its aggregate emits.
fn panel_named(name: &str) -> (Value, Value) {
    let member = json!({"role": "roles/work.md", "driver": {"command": ["driver"]}});
    let review = json!({"results": ["pass", "fail"], "aggregate": "unanimous-pass",
                        "panel": {name: member.clone(), "other": member}});
    let rule = |result: &str| {
        json!({"id": result, "from": "review", "result": result,
               "next": "done", "reason": result})
    };
    let mut table = policy();
    table["rules"] = json!([table["rules"][0], rule("pass"), rule("fail")]);
    (seats(bare("complete"), review), table)
}

/// Review return F1: a member NAMED for a root references nothing, yet the
/// compiler that scanned the raw seats read the name as the key it spells
/// and opened that root, so the roots open as written once every seat is
/// parsed. An `agent` name opens the library, whose unseated agents' asks
/// the manifest pins; with no roots, each name refuses as df112346 did, and
/// a name that spells no key compiles.
#[test]
fn a_member_named_for_a_root_opens_it_as_the_written_keys_always_did() {
    let fixture = AgentFixture::declaring();
    define(&fixture, "web-search", json!(["reads", "egress"]));
    fixture.write(
        "agents/researcher.json",
        unseated(json!({"web-search": "wants"})),
    );
    let (config, table) = panel_named("agent");
    let compiled = fixture.compile_with_policy(config, &table).unwrap();
    let definitions = compiled.manifest["capabilities"]["definitions"].as_object();
    assert_eq!(
        definitions.unwrap().keys().collect::<Vec<_>>(),
        ["web-search"]
    );

    std::fs::remove_dir_all(fixture.library()).unwrap();
    std::fs::remove_dir_all(fixture.adapters()).unwrap();
    let said = |name: &str| {
        let (config, table) = panel_named(name);
        outcome(fixture.compile_with_policy(config, &table))
    };
    let absent = |root: &str, path: PathBuf| {
        format!(
            "bundle: {root} {}: No such file or directory (os error 2)",
            path.display()
        )
    };
    let mut rows = vec![(
        "agent".to_string(),
        said("agent"),
        absent("agent library", fixture.library()),
    )];
    for name in ["secrets", "tools", "dialect"] {
        let expected = format!("{}; {NEEDED}", absent("adapters", fixture.adapters()));
        rows.push((name.to_string(), said(name), expected));
    }
    each_row(rows);
    let (config, table) = panel_named("one");
    let unnamed = fixture.compile_with_policy(config, &table);
    assert_eq!(unnamed.unwrap().name, "fixture");
}

/// Each root is loaded at most once per compile, and what its load said is
/// kept: a root that changes after the first site opened it changes nothing
/// a later site reads, an optional read's failure included.
#[test]
fn each_root_is_loaded_once_and_what_its_load_said_is_kept() {
    let fixture = AgentFixture::new();
    let (library, adapters) = (fixture.library(), fixture.adapters());
    let definitions = crate::capabilities::Definitions::default();
    let opened = || {
        Opened::new(
            (&library, &adapters),
            &definitions,
            EgressClass::Contracted,
            Vec::new(),
        )
    };
    let read = |opened: &mut Opened<'_>| match opened.agent_roots() {
        Ok((library, adapters)) => {
            format!("{:?} {:?}", library.names(), adapters.providers().count())
        }
        Err(refusal) => refusal.to_string(),
    };
    let mut once = opened();
    assert_eq!(read(&mut once), "[\"worker\"] 1");
    std::fs::remove_dir_all(&library).unwrap();
    std::fs::remove_dir_all(&adapters).unwrap();
    assert_eq!(read(&mut once), "[\"worker\"] 1");
    assert_eq!(
        once.adapters()
            .map(|adapters| adapters.providers().count())
            .ok(),
        Some(1)
    );

    std::fs::create_dir_all(&adapters).unwrap();
    std::fs::write(adapters.join("claude.json"), "{").unwrap();
    let mut failed = opened();
    failed.read_for_inline_driver();
    fixture.write("adapters/claude.json", claude());
    let words = format!(
        "{}: EOF while parsing an object at line 1 column 1",
        adapters.join("claude.json").display()
    );
    assert_eq!(failed.unloadable(), Some(words.as_str()));
    assert_eq!(
        error(failed.adapters()),
        format!("bundle: {words}; {NEEDED}")
    );
}
