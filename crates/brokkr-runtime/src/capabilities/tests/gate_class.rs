//! GP1 (decision 0065 slice two, U3a): what a gate may hold, proved at the
//! pure check and through resolution. The site here is `review`, of office
//! `reviewer`; only its class moves between rows.

use super::*;
use crate::bundle::SeatClass;

/// GP1's two causes, pinned once for this module.
const WRITES: &str = "gate offices cannot hold a capability with class 'writes'";
const EGRESS: &str =
    "a gate's egress capability requires the realm grant to name office 'reviewer' explicitly";

/// A world whose `web-search` abstraction has exactly `classes`, served by
/// `test-native` through `search-native`, which claims no classes itself.
fn classed(classes: &[&str]) -> TempDir {
    let root = TempDir::new_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    define(root.path(), "web-search", classes);
    let search = native_dialect("search-native", "web-search", &["lookup", "search"]);
    dialect(root.path(), &search);
    root
}

/// The realm's `web-search` grant, its offices list written only where
/// one is given.
fn granted(offices: Option<&[&str]>) -> Value {
    let mut grant = json!({"dialect": "search-native"});
    if let Some(offices) = offices {
        grant["offices"] = json!(offices);
    }
    json!({"web-search": grant})
}

impl SiteAsks {
    /// A WORK site's asks: the suites' shorthand for [`SiteAsks::at`].
    /// Production names every site's class, so this lives with the tests.
    pub(crate) fn of(
        label: &str,
        agent: Option<(&str, &Requests)>,
        site: Option<&Value>,
    ) -> Result<SiteAsks, String> {
        SiteAsks::at(SeatClass::Work, label, agent, site)
    }
}

/// Site `review` of office `reviewer` at `class`, its office asking
/// `requests` and the seat subtracting nothing.
fn review(class: SeatClass, requests: &Value) -> SiteAsks {
    let office = parse_requests("agent 'reviewer'", requests).unwrap();
    SiteAsks::at(class, "review", Some(("reviewer", &office)), None).unwrap()
}

fn requires() -> Value {
    json!({"web-search": "requires"})
}

fn wants() -> Value {
    json!({"web-search": "wants"})
}

/// GP1's complete required diagnostic at `review`.
fn required(cause: &str) -> String {
    format!(
        "seat 'review' (office 'reviewer') in realm 'private': requires capability 'web-search' \
         through dialect 'search-native', but {cause}"
    )
}

/// GP1's optional notice at `review`, with the native OFF it stands on.
fn dropped(cause: &str) -> String {
    format!(
        "seat 'review' (office 'reviewer') in realm 'private': dropped wanted capability \
         'web-search' through dialect 'search-native' because {cause}; native capability \
         remains OFF"
    )
}

/// The gate `review` under `realm`: requiring the ask refuses with
/// `cause`; wanting it drops the whole holding with that cause and serves
/// the switchable provider's OFF.
fn gate_loses(realm: &Authority, cause: &str, case: &str) {
    let native = switchable();
    let gate = |asks: &Value| review(SeatClass::Gate, asks);
    let refusal = realm.resolve(&gate(&requires()), &serving(&native));
    assert_eq!(refusal.unwrap_err(), required(cause), "{case}");
    let lost = realm.resolve(&gate(&wants()), &serving(&native)).unwrap();
    assert!(lost.held.is_empty(), "{case}");
    assert_eq!(lost.notices, [("web-search".to_string(), dropped(cause))]);
    let off = format!("{cause}; native capability remains OFF");
    assert_eq!(lost.not_held["web-search"], off, "{case}");
    assert_eq!(argv_of(&lost), ["--search-off"], "{case}");
}

/// The check reads the site's own class, the abstraction's classes and
/// whether the grant names the office — never the dialect's kind. An MCP
/// grant is judged by it exactly as a native one, and every MCP grant is
/// still refused realm-wide before any seat (the fence U9b alone lifts).
#[test]
fn the_gate_check_reads_the_sites_class_the_abstract_classes_and_the_named_office() {
    let root = classed(&["reads"]);
    let realm = |offices: &Value| {
        let mut grant = json!({"dialect": "tracker-mcp"});
        if !offices.is_null() {
            grant["offices"] = offices.clone();
        }
        context(root.path(), json!({"issue-tracker": grant}))
    };
    let tracker = |classes: &[&str]| Definition {
        name: "issue-tracker".into(),
        classes: classes.iter().map(ToString::to_string).collect(),
        source: Definition::source_of("issue-tracker"),
        sha256: String::new(),
    };
    let writes = Err(gates::GateRefusal::Writes);
    let egress = Err(gates::GateRefusal::Egress {
        office: "reviewer".into(),
    });
    let all: &[&str] = &["reads", "writes", "egress"];
    let rows = [
        (SeatClass::Work, all, Value::Null, Ok(())),
        (SeatClass::Gate, &["reads"][..], Value::Null, Ok(())),
        (
            SeatClass::Gate,
            &["reads", "writes"],
            json!(["reviewer"]),
            writes.clone(),
        ),
        (SeatClass::Gate, all, Value::Null, writes.clone()),
        (SeatClass::Gate, &["writes"], json!(["reviewer"]), writes),
        (
            SeatClass::Gate,
            &["reads", "egress"],
            Value::Null,
            egress.clone(),
        ),
        (SeatClass::Gate, &["egress"], json!(["review"]), egress),
        (
            SeatClass::Gate,
            &["reads", "egress"],
            json!(["reviewer"]),
            Ok(()),
        ),
    ];
    for (class, classes, offices, expected) in rows {
        let grants = realm(&offices).grants;
        assert_eq!(
            gates::check(
                &review(class, &json!({})),
                &tracker(classes),
                &grants["issue-tracker"]
            ),
            expected,
            "{class:?} {classes:?} {offices}"
        );
    }
    assert_eq!(gates::GateRefusal::Writes.to_string(), WRITES);
    let office = "reviewer".to_string();
    assert_eq!(gates::GateRefusal::Egress { office }.to_string(), EGRESS);
    define(root.path(), "issue-tracker", &["reads", "writes"]);
    let mut mcp = mcp_dialect("tracker-mcp", json!({"argv": ["/nonexistent/tracker-mcp"]}));
    mcp["serves"] = json!("issue-tracker");
    dialect(root.path(), &mcp);
    for offices in [Value::Null, json!([]), json!(["reviewer"])] {
        assert_eq!(
            Authority::load(realm(&offices)).unwrap_err(),
            "realm 'private' grants capability 'issue-tracker' through dialect 'tracker-mcp' of \
             kind 'mcp', whose broker support is not implemented until decision 0065 slice two"
        );
    }
}

/// Writes is never a gate's, whoever the realm names and whatever other
/// class the abstraction carries; it is judged before egress.
#[test]
fn a_gate_never_holds_writes_whoever_the_realm_names() {
    let classes: [&[&str]; 3] = [
        &["reads", "writes"],
        &["reads", "writes", "egress"],
        &["writes"],
    ];
    for classes in classes {
        let root = classed(classes);
        for offices in [None, Some(&["reviewer"][..])] {
            let realm = authority(root.path(), granted(offices));
            gate_loses(&realm, WRITES, &format!("{classes:?} {offices:?}"));
        }
    }
}

/// Egress needs the grant to name the gate's office: an absent list
/// reaches it and names nobody, an empty one keeps D4's earlier cause, and
/// the execution label is not the office. A reads abstraction needs no
/// naming at all.
#[test]
fn a_gates_egress_needs_the_grant_to_name_its_office() {
    let root = cq1_root();
    let native = switchable();
    gate_loses(&authority(root.path(), granted(None)), EGRESS, "unnamed");
    let named = authority(root.path(), granted(Some(&["reviewer"])));
    let held = named
        .resolve(&review(SeatClass::Gate, &requires()), &serving(&native))
        .unwrap();
    assert_eq!(held.held["web-search"].classes, ["reads", "egress"]);
    assert_eq!(argv_of(&held), ["--search-on"]);
    for (offices, but) in [
        (&[][..], "the realm grants it to no office".to_string()),
        (
            &["review"][..],
            "the realm grants it only to offices [review], not to this office".to_string(),
        ),
    ] {
        let scoped = authority(root.path(), granted(Some(offices)));
        assert_eq!(
            scoped
                .resolve(&review(SeatClass::Gate, &requires()), &serving(&native))
                .unwrap_err(),
            format!(
                "seat 'review' (office 'reviewer') in realm 'private': requires capability \
                 'web-search' but {but}"
            )
        );
    }
    let reads = classed(&["reads"]);
    let read_only = authority(reads.path(), granted(None))
        .resolve(&review(SeatClass::Gate, &requires()), &serving(&native))
        .unwrap();
    assert_eq!(read_only.held["web-search"].classes, ["reads"]);
    assert_eq!(argv_of(&read_only), ["--search-on"]);
}

/// The controls: a work site keeps slice one's holding, and a gate that
/// subtracts the ask or asks nothing leaves the grant pinned and unheld,
/// without a GP1 cause.
#[test]
fn a_work_site_a_subtraction_and_an_unused_grant_meet_no_gate_cause() {
    let root = classed(&["reads", "writes", "egress"]);
    let native = switchable();
    let realm = authority(root.path(), granted(None));
    let work = realm
        .resolve(&review(SeatClass::Work, &requires()), &serving(&native))
        .unwrap();
    assert_eq!(
        work.held["web-search"].classes,
        ["reads", "writes", "egress"]
    );
    assert_eq!(argv_of(&work), ["--search-on"]);
    let office = parse_requests("agent 'reviewer'", &requires()).unwrap();
    let (gate, nothing) = (SeatClass::Gate, json!({}));
    let subtracting = SiteAsks::at(gate, "review", Some(("reviewer", &office)), Some(&nothing));
    for site in [subtracting, SiteAsks::at(gate, "review", None, None)] {
        let site = site.unwrap();
        let outcome = realm.resolve(&site, &serving(&native)).unwrap();
        assert!(outcome.held.is_empty() && outcome.notices.is_empty());
        assert_eq!(argv_of(&outcome), ["--search-off"]);
    }
    assert_eq!(
        realm.manifest(&[])["grants"],
        json!({"web-search": {"dialect": "search-native"}})
    );
}

/// A gate's dropped want still stands on its provider's OFF: one that
/// cannot be switched off, or whose OFF nobody measured, refuses the seat
/// as it would any unheld power, and a default OFF is recorded off.
#[test]
fn a_dropped_gate_want_still_needs_its_native_power_proved_off() {
    let root = cq1_root();
    let realm = authority(root.path(), granted(None));
    let gate = review(SeatClass::Gate, &wants());
    let declared =
        |off: Value| test_native(json!({"default": "on"}), off, json!({"unsupported": "x"}));
    let stuck = declared(json!({"unsupported": "no switch reaches it"}));
    assert_eq!(
        realm.resolve(&gate, &serving(&stuck)).unwrap_err(),
        "seat 'review' (office 'reviewer') in realm 'private': provider 'test-native' cannot \
         switch off its native capability 'web-search', which this seat does not hold (no \
         switch reaches it; evidence: a test, scope: a test); an ungranted native capability \
         that cannot be disabled cannot be seated in this realm (decision 0065 ruling 4)"
    );
    let unknown = declared(json!({"unmeasured": "nobody tried"}));
    assert_eq!(
        realm.resolve(&gate, &serving(&unknown)).unwrap_err(),
        "seat 'review' (office 'reviewer') in realm 'private': provider 'test-native' is known \
         to carry native capability 'web-search', which this seat does not hold, and no valid \
         control denies it: its OFF control is unmeasured (nobody tried). A known native power \
         is launched only with a delivered denial, never on what absence implies; repair the \
         adapter data (decision 0066 ruling 1)"
    );
    let quiet = declared(json!({"default": "off unless asked"}));
    let outcome = realm.resolve(&gate, &serving(&quiet)).unwrap();
    assert_eq!(
        outcome.notices,
        [("web-search".to_string(), dropped(EGRESS))]
    );
    assert_eq!(outcome.manifest()["native"]["off"], json!(["web-search"]));
}
