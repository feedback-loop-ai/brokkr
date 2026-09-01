//! Decision 0021's two compile-time refusals, arm by arm.
//!
//! Every bundle here is synthetic and written per test (the pattern
//! `secret_binding_tests.rs` established), and every provider name is
//! invented — `judge`, `newcomer`, `silent`. That is the load-bearing
//! part: the engine matches on a DECLARATION, so a fixture provider no
//! vendor answers to must be able to hold a gate, and the incumbent must
//! be refusable by editing one JSON file. If any test here needed a real
//! vendor's name, a vendor's name would have got into the engine.

use super::*;
use serde_json::json;

/// Two work results and a review that concludes: enough table for a
/// single seat, a two-member panel, or a two-step sequence to sit in the
/// `work` phase without any of them touching the protected gate.
const POLICY: &str = r#"{
      "schema": "forge.phase-machine/v1",
      "phases": ["work", "review", "done", "stop"],
      "initial": "work",
      "terminal": ["done", "stop"],
      "shippable_from": ["review"],
      "rules": [
        {"id": "W-PASS", "from": "work", "result": "pass", "next": "review",
         "reason": "work concluded"},
        {"id": "W-FAIL", "from": "work", "result": "fail", "next": "stop",
         "reason": "work failed"},
        {"id": "R-OK", "from": "review", "result": "clean", "next": "done",
         "reason": "review concluded"}
      ]
    }"#;

/// One adapter as data: a provider that declares a tier, a grant, both
/// or neither. `None` is the ABSENT declaration every fail-closed
/// assertion below turns on — the key is not written at all.
fn adapter(name: &str, tier: Option<&str>, grant: Option<bool>) -> Value {
    let mut value = json!({
        "provider": name,
        "binary": name,
        "driver": ["{brokkr}", "driver", name, "--"],
        "models": {},
        "model_flag": "unsupported",
        "tool_permissions": "unsupported",
        "mcp": "unsupported",
    });
    if let Some(tier) = tier {
        value["trust_tier"] = json!(tier);
    }
    if let Some(grant) = grant {
        value["binding_grant"] = json!(grant);
    }
    value
}

struct Fixture {
    dir: tempfile::TempDir,
}

impl Fixture {
    /// The three providers every test below draws from: one the operator
    /// promoted, one newcomer that declares its position honestly, and
    /// one that declares nothing at all.
    fn new() -> Fixture {
        let fixture = Fixture {
            dir: tempfile::tempdir().unwrap(),
        };
        std::fs::create_dir_all(fixture.dir.path().join("adapters")).unwrap();
        std::fs::create_dir_all(fixture.dir.path().join("agents/charters")).unwrap();
        fixture.write_adapter(adapter("judge", Some("trusted"), Some(true)));
        fixture.write_adapter(adapter("newcomer", Some("untrusted"), Some(false)));
        fixture.write_adapter(adapter("silent", None, None));
        fixture
    }

    fn write_adapter(&self, value: Value) {
        let name = value["provider"].as_str().unwrap().to_string();
        std::fs::write(
            self.dir.path().join(format!("adapters/{name}.json")),
            serde_json::to_string_pretty(&value).unwrap(),
        )
        .unwrap();
    }

    /// An agent whose chain is the given (model, provider, tier) links.
    /// Each link gets its own model-serving adapter, because one model
    /// name maps to exactly one provider.
    fn write_agent(&self, name: &str, links: &[(&str, &str, &str)]) {
        for (model, provider, tier) in links {
            let mut serving = adapter(provider, Some(tier), Some(true));
            serving["models"] = json!({ *model: format!("{provider}-1") });
            serving["model_flag"] = json!("--model");
            self.write_adapter(serving);
        }
        std::fs::write(
            self.dir.path().join(format!("agents/charters/{name}.md")),
            "# charter\n",
        )
        .unwrap();
        std::fs::write(
            self.dir.path().join(format!("agents/{name}.json")),
            serde_json::to_string(&json!({
                "description": "a fixture agent",
                "charter": format!("charters/{name}.md"),
                "models": links.iter().map(|(model, _, _)| *model).collect::<Vec<_>>(),
            }))
            .unwrap(),
        )
        .unwrap();
    }

    /// Compile a two-seat bundle whose `work` seat is exactly `work`.
    /// The `review` seat is a plain work-class driver: it declares
    /// nothing, so nothing about it is checked.
    fn compile(&self, work: Value) -> Result<Bundle, CompileError> {
        self.compile_against(work, &self.dir.path().join("adapters"))
    }

    fn compile_against(&self, work: Value, adapters: &Path) -> Result<Bundle, CompileError> {
        let bundle = self.dir.path().join("bundle");
        std::fs::create_dir_all(bundle.join("roles")).unwrap();
        std::fs::write(bundle.join("policy.json"), POLICY).unwrap();
        std::fs::write(bundle.join("roles/role.md"), "# role\n").unwrap();
        let config = json!({
            "name": "model-policy",
            "policy": "policy.json",
            "seats": {
                "work": work,
                "review": {
                    "role": "roles/role.md",
                    "results": ["clean"],
                    "driver": {"command": ["{brokkr}", "driver", "judge", "--", "true"]},
                },
            }
        });
        std::fs::write(
            bundle.join("bundle.json"),
            serde_json::to_string_pretty(&config).unwrap(),
        )
        .unwrap();
        Bundle::compile_with(&bundle, &self.dir.path().join("agents"), adapters)
    }

    fn refusal(&self, work: Value) -> String {
        match self.compile(work) {
            Ok(_) => panic!("expected a compile refusal"),
            Err(error) => error.to_string(),
        }
    }
}

/// An inline seat driven by `provider`, classed and bound as given.
fn seat(provider: &str, class: Option<&str>, secrets: Option<Value>) -> Value {
    let mut value = json!({
        "role": "roles/role.md",
        "results": ["pass", "fail"],
        "driver": {"command": ["{brokkr}", "driver", provider, "--", "true"]},
    });
    if let Some(class) = class {
        value["class"] = json!(class);
    }
    if let Some(secrets) = secrets {
        value["secrets"] = secrets;
    }
    value
}

// ------------------------------------------------- ruling 2: the gate

#[test]
fn a_gate_seating_an_untrusted_driver_refuses_and_names_both() {
    let fixture = Fixture::new();
    let refusal = fixture.refusal(seat("newcomer", Some("gate"), None));
    assert!(refusal.contains("seat 'work'"), "{refusal}");
    assert!(refusal.contains("driver 'newcomer'"), "{refusal}");
    assert!(
        refusal.contains("does not hold the trusted tier"),
        "{refusal}"
    );
    assert!(refusal.contains("0021 ruling 2"), "{refusal}");
}

#[test]
fn a_gate_seating_a_trusted_driver_compiles() {
    let fixture = Fixture::new();
    fixture
        .compile(seat("judge", Some("gate"), None))
        .expect("a promoted driver may hold a gate");
}

#[test]
fn a_gate_whose_driver_declares_no_tier_at_all_refuses() {
    // Fail-closed on ABSENCE — the `at_most` lesson, as law: a provider
    // file that simply never mentions trust is untrusted, not exempt.
    let fixture = Fixture::new();
    let refusal = fixture.refusal(seat("silent", Some("gate"), None));
    assert!(refusal.contains("driver 'silent'"), "{refusal}");
    assert!(
        refusal.contains("undeclared tier is untrusted"),
        "{refusal}"
    );
}

#[test]
fn a_gate_whose_driver_no_adapter_declares_refuses() {
    let fixture = Fixture::new();
    let refusal = fixture.refusal(seat("stranger", Some("gate"), None));
    assert!(refusal.contains("driver 'stranger'"), "{refusal}");
    assert!(
        refusal.contains("does not hold the trusted tier"),
        "{refusal}"
    );
}

#[test]
fn a_gate_whose_command_is_no_dispatch_at_all_refuses() {
    let fixture = Fixture::new();
    let mut work = seat("judge", Some("gate"), None);
    work["driver"] = json!({"command": ["bash", "-c", "true"]});
    let refusal = fixture.refusal(work);
    assert!(refusal.contains("an unnamed driver"), "{refusal}");

    // And a command too short to carry a dispatch at all reads the same
    // way: no driver named, so nothing to trust.
    let mut work = seat("judge", Some("gate"), None);
    work["driver"] = json!({"command": ["true"]});
    assert!(fixture.refusal(work).contains("an unnamed driver"));
}

#[test]
fn a_work_seat_seating_an_untrusted_driver_compiles() {
    // The refusal is gate-only: work seats produce output the machine
    // checks, which is exactly what ruling 7's newcomers may do.
    let fixture = Fixture::new();
    fixture
        .compile(seat("newcomer", Some("work"), None))
        .expect("a newcomer works freely");
    fixture
        .compile(seat("newcomer", None, None))
        .expect("an undeclared site is work");
}

// --------------------------------------------- ruling 4: the bindings

#[test]
fn a_seat_declaring_bindings_refuses_an_ungranted_driver() {
    let fixture = Fixture::new();
    let refusal = fixture.refusal(seat("newcomer", None, Some(json!(["GH_TOKEN"]))));
    assert!(refusal.contains("seat 'work'"), "{refusal}");
    assert!(refusal.contains("driver 'newcomer'"), "{refusal}");
    assert!(refusal.contains("holds no binding grant"), "{refusal}");
    assert!(refusal.contains("0021 ruling 4"), "{refusal}");
}

#[test]
fn a_seat_declaring_bindings_accepts_a_granted_driver() {
    let fixture = Fixture::new();
    let bundle = fixture
        .compile(seat("judge", None, Some(json!(["GH_TOKEN"]))))
        .expect("a granted driver may receive bindings");
    assert_eq!(bundle.seats["work"].secrets, vec!["GH_TOKEN".to_string()]);
}

#[test]
fn a_seat_declaring_bindings_refuses_a_driver_with_no_grant_key() {
    let fixture = Fixture::new();
    let refusal = fixture.refusal(seat("silent", None, Some(json!(["GH_TOKEN"]))));
    assert!(refusal.contains("driver 'silent'"), "{refusal}");
    assert!(refusal.contains("undeclared grant is none"), "{refusal}");
}

#[test]
fn a_seat_declaring_no_bindings_accepts_an_ungranted_driver() {
    // The grant is read only when bindings are actually declared; an
    // empty declaration is no declaration.
    let fixture = Fixture::new();
    fixture
        .compile(seat("newcomer", None, None))
        .expect("no bindings, nothing to clear");
    fixture
        .compile(seat("newcomer", None, Some(json!([]))))
        .expect("an empty binding list binds nothing");
}

#[test]
fn the_two_axes_are_independent() {
    // Ruling 4's whole point: trust to judge and clearance to receive
    // are different grants. A driver may hold either alone.
    let fixture = Fixture::new();
    fixture.write_adapter(adapter("courier", Some("untrusted"), Some(true)));
    fixture.write_adapter(adapter("ascetic", Some("trusted"), Some(false)));
    fixture
        .compile(seat("courier", None, Some(json!(["GH_TOKEN"]))))
        .expect("an untrusted driver may still be cleared to receive");
    assert!(fixture
        .refusal(seat("courier", Some("gate"), None))
        .contains("does not hold the trusted tier"));
    fixture
        .compile(seat("ascetic", Some("gate"), None))
        .expect("a trusted driver judges");
    assert!(fixture
        .refusal(seat("ascetic", None, Some(json!(["GH_TOKEN"]))))
        .contains("holds no binding grant"));
}

// ------------------------------------------------- the class itself

#[test]
fn a_class_outside_the_vocabulary_refuses() {
    let fixture = Fixture::new();
    let refusal = fixture.refusal(seat("judge", Some("judge-ish"), None));
    assert!(refusal.contains("unknown class"), "{refusal}");
    assert!(refusal.contains("known: work, gate"), "{refusal}");

    let mut work = seat("judge", None, None);
    work["class"] = json!(2);
    assert!(fixture.refusal(work).contains("unknown class"));
}

#[test]
fn a_seat_that_bears_no_driver_may_not_carry_a_class() {
    // `recipes/sdd`'s design seat is the reason: a panel of work
    // positions, a gate chief and a work check cannot share one word.
    let fixture = Fixture::new();
    let refusal = fixture.refusal(json!({
        "results": ["pass", "fail"],
        "class": "gate",
        "aggregate": "unanimous-pass",
        "panel": {
            "a": {"role": "roles/role.md",
                  "driver": {"command": ["{brokkr}", "driver", "judge", "--", "true"]}},
            "b": {"role": "roles/role.md",
                  "driver": {"command": ["{brokkr}", "driver", "judge", "--", "true"]}},
        },
    }));
    assert!(refusal.contains("bears no driver of its own"), "{refusal}");

    let refusal = fixture.refusal(json!({
        "results": ["pass", "fail"],
        "sequence": [
            {"name": "first", "role": "roles/role.md",
             "driver": {"command": ["{brokkr}", "driver", "judge", "--", "true"]}},
            {"name": "second", "class": "gate", "aggregate": "unanimous-pass",
             "panel": {
                "a": {"role": "roles/role.md",
                      "driver": {"command": ["{brokkr}", "driver", "judge", "--", "true"]}},
                "b": {"role": "roles/role.md",
                      "driver": {"command": ["{brokkr}", "driver", "judge", "--", "true"]}},
             }},
        ],
    }));
    assert!(refusal.contains("seat 'work:second'"), "{refusal}");
    assert!(refusal.contains("bears no driver of its own"), "{refusal}");
}

// --------------------------------- every driver-bearing site, not just seats

#[test]
fn a_panel_member_is_classed_and_refused_on_its_own() {
    let fixture = Fixture::new();
    let panel = |member: Value| {
        json!({
            "results": ["pass", "fail"],
            "aggregate": "unanimous-pass",
            "panel": {
                "trusted": {"class": "gate", "role": "roles/role.md",
                            "driver": {"command":
                                ["{brokkr}", "driver", "judge", "--", "true"]}},
                "other": member,
            },
        })
    };
    let refusal = fixture.refusal(panel(json!({
        "class": "gate",
        "role": "roles/role.md",
        "driver": {"command": ["{brokkr}", "driver", "newcomer", "--", "true"]},
    })));
    assert!(refusal.contains("seat 'work:other'"), "{refusal}");
    assert!(refusal.contains("driver 'newcomer'"), "{refusal}");

    // The same member, classed work, is exactly ruling 7's newcomer
    // sitting beside a judge — and compiles.
    fixture
        .compile(panel(json!({
            "class": "work",
            "role": "roles/role.md",
            "driver": {"command": ["{brokkr}", "driver", "newcomer", "--", "true"]},
        })))
        .expect("a work member beside a gate member");
}

#[test]
fn a_sequence_step_is_classed_and_refused_on_its_own() {
    let fixture = Fixture::new();
    let refusal = fixture.refusal(json!({
        "results": ["pass", "fail"],
        "sequence": [
            {"name": "positions", "class": "work", "role": "roles/role.md",
             "driver": {"command": ["{brokkr}", "driver", "newcomer", "--", "true"]}},
            {"name": "chief", "class": "gate", "role": "roles/role.md",
             "driver": {"command": ["{brokkr}", "driver", "newcomer", "--", "true"]}},
        ],
    }));
    assert!(refusal.contains("seat 'work:chief'"), "{refusal}");
    assert!(refusal.contains("gate class"), "{refusal}");
}

#[test]
fn a_seats_bindings_reach_every_site_beneath_it() {
    // The declaration is the seat's, so the grant is required of every
    // driver the seat can put a value in front of.
    let fixture = Fixture::new();
    let refusal = fixture.refusal(json!({
        "results": ["pass", "fail"],
        "secrets": ["GH_TOKEN"],
        "aggregate": "unanimous-pass",
        "panel": {
            "granted": {"role": "roles/role.md",
                        "driver": {"command":
                            ["{brokkr}", "driver", "judge", "--", "true"]}},
            "ungranted": {"role": "roles/role.md",
                          "driver": {"command":
                              ["{brokkr}", "driver", "newcomer", "--", "true"]}},
        },
    }));
    assert!(refusal.contains("seat 'work:ungranted'"), "{refusal}");
    assert!(refusal.contains("holds no binding grant"), "{refusal}");
}

// ------------------------------------------------ agents and their chains

#[test]
fn a_gate_agent_is_refused_when_any_link_of_its_chain_is_untrusted() {
    // Ruling 5 parks an unavailable driver rather than substituting one,
    // but a chain that could fall back to an untrusted judge at run time
    // would have defeated the gate at compile time. Every link.
    let fixture = Fixture::new();
    fixture.write_agent(
        "mixed",
        &[
            ("first", "promoted", "trusted"),
            ("second", "raw", "untrusted"),
        ],
    );
    let refusal = fixture.refusal(json!({
        "results": ["pass", "fail"],
        "class": "gate",
        "agent": "mixed",
    }));
    assert!(refusal.contains("driver 'raw'"), "{refusal}");
    assert!(
        refusal.contains("does not hold the trusted tier"),
        "{refusal}"
    );
}

#[test]
fn a_gate_agent_whose_whole_chain_is_trusted_compiles() {
    let fixture = Fixture::new();
    fixture.write_agent(
        "steady",
        &[
            ("first", "promoted", "trusted"),
            ("second", "deputy", "trusted"),
        ],
    );
    let bundle = fixture
        .compile(json!({
            "results": ["pass", "fail"],
            "class": "gate",
            "agent": "steady",
        }))
        .expect("a chain of trusted links holds a gate");
    let SeatBody::Single { candidates, .. } = &bundle.seats["work"].body else {
        unreachable!("an agent seat is a single body")
    };
    assert_eq!(candidates.len(), 2);
}

// ------------------------------------------------- the adapter data itself

#[test]
fn a_tier_outside_the_vocabulary_refuses_at_load() {
    // Closed vocabulary, load time, decision 0004's style: a misspelled
    // tier must never read as untrusted by accident, or a promotion
    // would silently not have happened.
    let fixture = Fixture::new();
    fixture.write_adapter(adapter("typo", Some("trusetd"), None));
    let refusal = fixture.refusal(seat("judge", Some("gate"), None));
    assert!(refusal.contains("'trust_tier' is"), "{refusal}");
    assert!(refusal.contains("the vocabulary is closed"), "{refusal}");
}

#[test]
fn a_grant_that_is_not_a_boolean_refuses_at_load() {
    let fixture = Fixture::new();
    let mut broken = adapter("wordy", None, None);
    broken["binding_grant"] = json!("yes");
    fixture.write_adapter(broken);
    let refusal = fixture.refusal(seat("judge", Some("gate"), None));
    assert!(refusal.contains("'binding_grant' is"), "{refusal}");
    assert!(refusal.contains("the grant is a boolean"), "{refusal}");
}

#[test]
fn the_shipped_adapters_declare_what_decision_0021_ruled() {
    // The honest declarations item 3 of this slice recorded, asserted
    // where a future edit will trip over them: the incumbent's tier
    // rests on this shop's journaled runs (ruling 8's disclosure), the
    // newcomers start symmetric (ruling 7), and `exec` is untrusted but
    // cleared — it is the driver decision 0012 put secret resolution in.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .join("adapters");
    let adapters = Adapters::load(&root).expect("the shipped adapters load");
    for (provider, tier, grant) in [
        ("claude", TrustTier::Trusted, true),
        ("codex", TrustTier::Untrusted, false),
        ("dsh", TrustTier::Untrusted, false),
        ("exec", TrustTier::Untrusted, true),
        // Not named by this slice's scope, and so left undeclared:
        // fail-closed defaults are the correct reading of silence.
        ("lanetally", TrustTier::Untrusted, false),
    ] {
        let adapter = adapters.adapter(provider).expect("a shipped adapter");
        assert_eq!(adapter.trust_tier, tier, "{provider} tier");
        assert_eq!(adapter.binding_grant, grant, "{provider} grant");
    }
    assert!(adapters.adapter("nobody").is_none());
}

// ------------------------------------------------- what stays a non-event

#[test]
fn a_bundle_that_declares_neither_never_opens_the_adapters() {
    // The property decision 0016 established and this slice keeps: a
    // bundle with no agent, no gate and no binding compiles with no
    // adapter tree in sight.
    let fixture = Fixture::new();
    fixture
        .compile_against(
            seat("newcomer", Some("work"), None),
            Path::new("/nonexistent-adapters"),
        )
        .expect("nothing to check, nothing to open");
}

#[test]
fn a_gate_with_no_adapter_tree_to_read_refuses() {
    let fixture = Fixture::new();
    let error = fixture
        .compile_against(
            seat("judge", Some("gate"), None),
            Path::new("/nonexistent-adapters"),
        )
        .expect_err("a gate that cannot be checked is not a gate");
    let error = error.to_string();
    assert!(error.contains("trust tier and binding grant"), "{error}");
}
