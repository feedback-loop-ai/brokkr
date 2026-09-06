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
        "efforts": [],
        "effort_flag": "unsupported",
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
        let mut judge = adapter("judge", Some("trusted"), Some(true));
        judge["models"] = json!({"judge": "judge-1"});
        judge["judges"] = json!(["judge"]);
        judge["model_flag"] = json!("--model");
        fixture.write_adapter(judge);
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
            serving["judges"] = json!([*model]);
            serving["model_flag"] = json!("--model");
            self.write_adapter(serving);
        }
        let models: Vec<&str> = links.iter().map(|(model, _, _)| *model).collect();
        self.write_agent_file(name, &models, None, false);
    }

    /// Compile a two-seat bundle whose `work` seat is exactly `work`.
    /// The `review` seat is a plain work-class driver: it declares
    /// nothing, so nothing about it is checked.
    fn compile(&self, work: Value) -> Result<Bundle, CompileError> {
        self.compile_against(work, &self.dir.path().join("adapters"))
    }

    fn compile_against(&self, work: Value, adapters: &Path) -> Result<Bundle, CompileError> {
        self.compile_under(work, None, adapters)
    }

    /// The same bundle with the operator's egress bar written into it
    /// (decision 0036 ruling 4). `None` writes no key at all, which is
    /// what every bundle on disk looks like.
    fn compile_under(
        &self,
        work: Value,
        minimum: Option<Value>,
        adapters: &Path,
    ) -> Result<Bundle, CompileError> {
        let bundle = self.stage(work, minimum);
        Bundle::compile_with(&bundle, &self.agents(), adapters)
    }

    /// Write the two-seat bundle to disk and return its directory.
    fn stage(&self, work: Value, minimum: Option<Value>) -> PathBuf {
        let bundle = self.bundle_dir();
        std::fs::create_dir_all(bundle.join("roles")).unwrap();
        std::fs::write(bundle.join("policy.json"), POLICY).unwrap();
        std::fs::write(bundle.join("roles/role.md"), "# role\n").unwrap();
        let mut config = json!({
            "name": "model-policy",
            "policy": "policy.json",
            "seats": {
                "work": work,
                "review": {
                    "role": "roles/role.md",
                    "results": ["clean"],
                    "driver": {"command": [
                        "{brokkr}", "driver", "judge", "--", "--model", "judge-1", "true"
                    ]},
                },
            }
        });
        if let Some(minimum) = minimum {
            config["egress_minimum"] = minimum;
        }
        std::fs::write(
            bundle.join("bundle.json"),
            serde_json::to_string_pretty(&config).unwrap(),
        )
        .unwrap();
        bundle
    }

    fn agents(&self) -> PathBuf {
        self.dir.path().join("agents")
    }

    fn adapters(&self) -> PathBuf {
        self.dir.path().join("adapters")
    }

    /// The directory every compile here stages the bundle into — the
    /// leaf layer, whose manifest walk pins what lies under it.
    fn bundle_dir(&self) -> PathBuf {
        self.dir.path().join("bundle")
    }

    /// The same two-seat bundle compiled under a stated boundary
    /// (decision 0046 ruling 1), against the fixture's own library roots.
    fn compile_bounded(&self, work: Value, boundary: Boundary) -> Result<Bundle, CompileError> {
        self.compile_roots(work, &self.agents(), &self.adapters(), boundary)
    }

    /// The same bundle under a stated boundary against any library roots
    /// — the fixture's, the shipped tree's, or none at all.
    fn compile_roots(
        &self,
        work: Value,
        agents: &Path,
        adapters: &Path,
        boundary: Boundary,
    ) -> Result<Bundle, CompileError> {
        let bundle = self.stage(work, None);
        Bundle::compile_under(&bundle, agents, adapters, boundary)
    }

    fn refusal(&self, work: Value) -> String {
        match self.compile(work) {
            Ok(_) => panic!("expected a compile refusal"),
            Err(error) => error.to_string(),
        }
    }

    fn refusal_under(&self, work: Value, boundary: Boundary) -> String {
        match self.compile_bounded(work, boundary) {
            Ok(_) => panic!("expected a compile refusal under `{boundary}`"),
            Err(error) => error.to_string(),
        }
    }

    /// A file under the bundle directory, which the manifest walk pins.
    fn script(&self, relative: &str) {
        let path = self.bundle_dir().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "#!/bin/sh\ntrue\n").unwrap();
    }

    /// An agent file alone — no serving adapter written beside it — with
    /// the chain, the efforts a shipped provider takes, and boxed hands
    /// (decision 0043) when asked.
    fn write_agent_file(&self, name: &str, models: &[&str], efforts: Option<Value>, hands: bool) {
        std::fs::write(
            self.agents().join(format!("charters/{name}.md")),
            "# charter\n",
        )
        .unwrap();
        let mut body = json!({
            "description": "a fixture agent",
            "charter": format!("charters/{name}.md"),
            "models": models,
        });
        if let Some(efforts) = efforts {
            body["efforts"] = efforts;
        }
        if hands {
            body["hands"] = json!("workspace");
        }
        std::fs::write(
            self.agents().join(format!("{name}.json")),
            serde_json::to_string(&body).unwrap(),
        )
        .unwrap();
    }

    /// A boxed agent whose chain is the given (model, provider, harness)
    /// links: every serving provider is trusted, judges its model and
    /// declares a `hands.workspace` fragment, so the resolver admits the
    /// hands; `harness` is the provider's `hands.harness` declaration, or
    /// `None` for a provider that declares none (decision 0046 ruling 4).
    fn write_boxed_agent(&self, name: &str, links: &[(&str, &str, Option<Value>)]) {
        for (model, provider, harness) in links {
            let mut serving = adapter(provider, Some("trusted"), Some(true));
            serving["models"] = json!({ *model: format!("{provider}-1") });
            serving["judges"] = json!([*model]);
            serving["model_flag"] = json!("--model");
            let mut hands = json!({"workspace": ["--boxed", "{hands_mcp_json}"]});
            if let Some(harness) = harness {
                hands["harness"] = harness.clone();
            }
            serving["hands"] = hands;
            self.write_adapter(serving);
        }
        let models: Vec<&str> = links.iter().map(|(model, _, _)| *model).collect();
        self.write_agent_file(name, &models, None, true);
    }
}

/// An inline seat driven by `provider`, classed and bound as given.
fn seat(provider: &str, class: Option<&str>, secrets: Option<Value>) -> Value {
    let command = match provider {
        "claude" | "lanetally" => json!([
            "{brokkr}",
            "driver",
            provider,
            "--",
            "--model",
            "claude-fable-5-1",
            "--effort",
            "high",
            "true"
        ]),
        "codex" => json!([
            "{brokkr}",
            "driver",
            provider,
            "--",
            "--model",
            "gpt-5.6-sol",
            "--effort",
            "medium",
            "true"
        ]),
        "dsh" => json!([
            "{brokkr}",
            "driver",
            provider,
            "--",
            "--model",
            "deepseek-v4-flash",
            "--effort",
            "medium",
            "true"
        ]),
        "judge" => json!(["{brokkr}", "driver", provider, "--", "--model", "judge-1", "true"]),
        _ => json!(["{brokkr}", "driver", provider, "--", "true"]),
    };
    let mut value = json!({
        "role": "roles/role.md",
        "results": ["pass", "fail"],
        "driver": {"command": command},
    });
    if let Some(class) = class {
        value["class"] = json!(class);
    }
    if let Some(secrets) = secrets {
        value["secrets"] = secrets;
    }
    value
}

// -------------------------------- decision 0031: every model seat is pinned

#[test]
fn the_model_pin_refusal_names_every_inline_invocation_site_and_the_fix() {
    let seats = json!({
        "implement": {
            "role": "roles/role.md",
            "driver": {"command": ["{brokkr}", "driver", "claude", "--"]},
        },
        "review": {
            "sequence": [
                {"name": "codex-step", "role": "roles/role.md",
                 "driver": {"command": ["{brokkr}", "driver", "codex", "--"]}},
                {"name": "panel", "panel": {
                    "dsh-member": {"role": "roles/role.md", "driver": {"command":
                        ["{brokkr}", "driver", "dsh", "--"]}},
                    "lane-member": {"role": "roles/role.md", "driver": {"command":
                        ["{brokkr}", "driver", "lanetally", "--"]}}
                }},
                {"role": "roles/role.md",
                 "driver": {"command": ["{brokkr}", "driver", "claude", "--"]}}
            ]
        }
    });
    let refusal = enforce_model_pins(seats.as_object().unwrap())
        .unwrap_err()
        .to_string();
    for site in [
        "'implement'",
        "'review:codex-step'",
        "'review:panel:dsh-member'",
        "'review:panel:lane-member'",
        "'review:step-3'",
    ] {
        assert!(refusal.contains(site), "{site}: {refusal}");
    }
    assert!(refusal.contains("--model <concrete-model-id>"), "{refusal}");
    assert!(refusal.contains("0031 ruling 2"), "{refusal}");
    // Decision 0035 ruling 5: the effort refusal stands BESIDE the model
    // one, in the same message, naming the same complete repair set. A
    // seat that pins neither must not have to be compiled twice to learn
    // it was missing two things.
    assert!(refusal.contains("do not pin an effort"), "{refusal}");
    assert!(refusal.contains("--effort <level>"), "{refusal}");
    assert!(refusal.contains("0035 ruling 5"), "{refusal}");
    for site in [
        "'implement'",
        "'review:codex-step'",
        "'review:panel:dsh-member'",
        "'review:panel:lane-member'",
        "'review:step-3'",
    ] {
        assert_eq!(
            refusal.matches(site).count(),
            2,
            "{site} is named once in each half of the one refusal: {refusal}"
        );
    }
}

/// The half of ruling 5 the model rule cannot cover: a seat that names
/// its model concretely and its effort not at all is still half a hire,
/// and the compiler says so on its own.
#[test]
fn a_model_pinned_without_an_effort_is_refused_on_its_own() {
    let seats = json!({
        "implement": {
            "role": "roles/role.md",
            "driver": {"command":
                ["{brokkr}", "driver", "claude", "--", "--model", "claude-opus-5"]},
        },
    });
    let refusal = enforce_model_pins(seats.as_object().unwrap())
        .unwrap_err()
        .to_string();
    assert!(!refusal.contains("do not pin a model"), "{refusal}");
    assert!(
        refusal.contains("seats 'implement' do not pin an effort"),
        "{refusal}"
    );
    assert!(refusal.contains("0035 ruling 5"), "{refusal}");

    // Every way of naming an effort that is not one concrete level,
    // named once — the model rule's own list, one axis over.
    for command in [
        json!(["{brokkr}", "driver", "claude", "--", "--model", "m", "--effort"]),
        json!([
            "{brokkr}",
            "driver",
            "claude",
            "--",
            "--model",
            "m",
            "--effort",
            "--verbose"
        ]),
        json!([
            "{brokkr}",
            "driver",
            "claude",
            "--",
            "--model",
            "m",
            "--effort="
        ]),
        json!([
            "{brokkr}", "driver", "claude", "--", "--model", "m", "--effort", "low", "--effort",
            "high"
        ]),
        json!([
            "{brokkr}",
            "driver",
            "claude",
            "--",
            "--model",
            "m",
            "--effort",
            "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
        ]),
    ] {
        let seats = json!({"work": {"driver": {"command": command}}});
        assert!(
            enforce_model_pins(seats.as_object().unwrap())
                .unwrap_err()
                .to_string()
                .contains("do not pin an effort"),
            "{seats}"
        );
    }

    // And an exec seat needs neither pin: ruling 5's own exemption.
    let exec = json!({"work": {"driver": {"command":
        ["{brokkr}", "driver", "exec", "--", "true"]}}});
    enforce_model_pins(exec.as_object().unwrap()).expect("exec needs no effort");
}

#[test]
fn explicit_split_and_equals_pins_agents_custom_drivers_and_exec_are_accepted() {
    let seats = json!({
        "claude": {"driver": {"command":
            ["{brokkr}", "driver", "claude", "--",
             "--model", "claude-fable-5-1", "--effort", "high"]}},
        "codex": {"driver": {"command":
            ["{brokkr}", "driver", "codex", "--",
             "--model=gpt-5.6-sol", "--effort=medium"]}},
        "exec": {"driver": {"command":
            ["{brokkr}", "driver", "exec", "--", "true"]}},
        "agent": {"agent": "implementer"},
        "custom": {"driver": {"command": ["custom-driver"]}}
    });
    enforce_model_pins(seats.as_object().unwrap()).expect("every model invocation is pinned");

    for command in [
        json!(null),
        json!(["{brokkr}", "driver", "claude", "--", "--model"]),
        json!(["{brokkr}", "driver", "claude", "--", "--model", "--verbose"]),
        json!(["{brokkr}", "driver", "claude", "--", "--model="]),
        json!(["{brokkr}", "driver", "claude", "--", "--model", "one", "--model", "two"]),
        json!([
            "{brokkr}",
            "driver",
            "claude",
            "--",
            "--model",
            "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
        ]),
        json!([
            "{brokkr}",
            "driver",
            "claude",
            "--",
            "--model",
            "one",
            "--model=two"
        ]),
    ] {
        assert!(
            !command_pins_model(&json!({"driver":{"command":command}})),
            "a missing, flag-shaped or ambiguous model is not a concrete pin"
        );
    }
}

/// A neighbouring LONG flag is a different flag, not an illegible
/// spelling of this one — and decision 0040 ruling 2 says so by the
/// flag's SHAPE, so both readers answer it the same way.
#[test]
fn a_longer_flag_of_the_same_family_leaves_both_pins_stated() {
    // A long flag has exactly two spellings, `FLAG VALUE` and
    // `FLAG=VALUE`; any other word beginning with it is the next flag
    // along. `--model-fallback` is not a way of writing `--model`, and
    // `--effort-cap` is not a way of writing `--effort` — for decision
    // 0036's route reader as much as for 0031's pin reader.
    //
    // Before ruling 2 this was keyed to the CALLER rather than to the
    // flag, so the route reader carried a strict reading onto long
    // flags too and refused a `--model` adapter's argv that readably
    // pinned a declared route, in a message naming the wrong problem.
    // The half asserted here is that 0031's question is untouched: a
    // seat that states BOTH pins concretely must not be told to add the
    // pin it already wrote.
    let seats = json!({
        "implement": {"driver": {"command": [
            "{brokkr}", "driver", "claude", "--",
            "--model", "claude-opus-5", "--model-fallback", "claude-sonnet-5",
            "--effort", "high", "--effort-cap", "medium"
        ]}},
        // The attached spelling of the value itself still reads, on
        // both axes: this is about the word AFTER the pin, not the pin.
        "review": {"driver": {"command": [
            "{brokkr}", "driver", "codex", "--",
            "--model=gpt-5.6-sol", "--model-fallback=gpt-5.6-thinking",
            "--effort=medium", "--effort-cap=medium"
        ]}},
    });
    enforce_model_pins(seats.as_object().unwrap())
        .expect("a seat that states both pins has stated them, whatever stands beside them");
    for site in seats.as_object().unwrap().values() {
        assert!(command_pins_model(site));
    }

    // And the rule the strict reading is there for is unchanged: two
    // spellings of the SAME flag are still a pin named twice.
    let twice = json!({"work": {"driver": {"command": [
        "{brokkr}", "driver", "claude", "--",
        "--model", "one", "--model-fallback", "two", "--model", "three",
        "--effort", "high"
    ]}}});
    assert!(
        enforce_model_pins(twice.as_object().unwrap())
            .unwrap_err()
            .to_string()
            .contains("do not pin a model"),
        "a model pinned twice is unreadable however many other flags surround it"
    );
}

/// Decision 0040 ruling 2's premise, asserted on the function that holds
/// it: a spelling is decided by the FLAG's shape. Short is one dash and
/// one character — the getopt shape a value attaches to — and everything
/// else, long flags included, has exactly the two spellings.
#[test]
fn a_flags_shape_is_read_off_the_flag_and_nothing_else() {
    assert!(short_flag("-m"));
    assert!(!short_flag("--model"));
    // Two characters after one dash is not the getopt shape, so it is
    // read as a long flag would be: exactly `FLAG VALUE` and `FLAG=VALUE`.
    assert!(!short_flag("-mo"));
    // Neither is a word that is no flag at all, or a bare dash.
    assert!(!short_flag("model"));
    assert!(!short_flag("-"));
}

/// Ruling 2's other half, on the reader that used to disagree: the 0036
/// route read walks past the same long neighbour, because the shape is
/// the flag's and not the caller's. `many` declares `--model`, so the
/// argv below readably pins a DECLARED route — and the reviewer's
/// finding is that it was refused as "not one readable concrete model
/// id" while naming a route the file rules `local`.
#[test]
fn the_route_reader_walks_past_a_long_neighbour_too() {
    let fixture = Fixture::new();
    let mut ruled = many_routes();
    ruled["egress"] = json!("contracted");
    fixture.write_adapter(ruled);

    fixture
        .compile(unpinned_seat(json!([
            "{brokkr}",
            "driver",
            "many",
            "--",
            "--model",
            "nearby/small-1",
            "--model-fallback",
            "partner/large-1",
            "true"
        ])))
        .expect("a long neighbour is a different flag, and the route pinned is read");

    // The equals spelling of the neighbour is a neighbour too, and the
    // route it names lends the site nothing either way.
    fixture
        .compile(unpinned_seat(json!([
            "{brokkr}",
            "driver",
            "many",
            "--",
            "--model=nearby/small-1",
            "--model-fallback=elsewhere/large-1",
            "true"
        ])))
        .expect("a long neighbour is walked past however it spells its own value");

    // And the flag itself is still read where it is written: the
    // neighbour beside it changes nothing about the route named.
    let refusal = fixture.refusal(unpinned_seat(json!([
        "{brokkr}",
        "driver",
        "many",
        "--",
        "--model",
        "elsewhere/large-1",
        "--model-fallback",
        "nearby/small-1",
        "true"
    ])));
    assert!(refusal.contains("on route 'elsewhere'"), "{refusal}");
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
    let mut judge = adapter("judge", Some("trusted"), Some(true));
    judge["models"] = json!({"judge": "judge-1"});
    judge["judges"] = json!(["judge"]);
    judge["model_flag"] = json!("--model");
    fixture.write_adapter(judge);
    let mut work = seat("judge", Some("gate"), None);
    work["driver"]["command"] =
        json!(["{brokkr}", "driver", "judge", "--", "--model", "judge-1", "true"]);
    fixture
        .compile(work)
        .expect("a promoted driver may hold a gate on a declared judge");
}

#[test]
fn a_trusted_unpinned_gate_is_not_exempt_from_an_empty_judges_list() {
    let fixture = Fixture::new();
    fixture.write_adapter(adapter("empty-judge", Some("trusted"), Some(true)));
    let refusal = fixture.refusal(seat("empty-judge", Some("gate"), None));
    assert!(refusal.contains("seat 'work'"), "{refusal}");
    assert!(refusal.contains("gate link 1"), "{refusal}");
    assert!(refusal.contains("model '<unpinned>'"), "{refusal}");
    assert!(
        refusal.contains("does not declare in 'judges'"),
        "{refusal}"
    );
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
    assert!(
        refusal.contains("on its own declared destination"),
        "{refusal}"
    );
    assert!(
        refusal.contains("whose egress class is uncontracted"),
        "{refusal}"
    );
    assert!(
        refusal.contains("binds no secret below contracted"),
        "{refusal}"
    );
    assert!(refusal.contains("0021 ruling 4"), "{refusal}");
    assert!(refusal.contains("0036"), "{refusal}");
}

/// A driver no adapter declares reaches an endpoint the operator has
/// said nothing about — which IS the definition of uncontracted, and so
/// refuses on the same comparison rather than on a special case.
#[test]
fn a_seat_declaring_bindings_refuses_a_driver_no_adapter_declares() {
    let fixture = Fixture::new();
    let refusal = fixture.refusal(seat("stranger", None, Some(json!(["GH_TOKEN"]))));
    assert!(refusal.contains("driver 'stranger'"), "{refusal}");
    assert!(
        refusal.contains("whose egress class is uncontracted"),
        "{refusal}"
    );

    let mut work = seat("judge", None, Some(json!(["GH_TOKEN"])));
    work["driver"] = json!({"command": ["bash", "-c", "true"]});
    let refusal = fixture.refusal(work);
    assert!(refusal.contains("an unnamed driver"), "{refusal}");
    assert!(
        refusal.contains("whose egress class is uncontracted"),
        "{refusal}"
    );
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
    assert!(
        refusal.contains("an undeclared class is uncontracted"),
        "{refusal}"
    );
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
    let mut ascetic = adapter("ascetic", Some("trusted"), Some(false));
    ascetic["models"] = json!({"ascetic": "ascetic-1"});
    ascetic["judges"] = json!(["ascetic"]);
    ascetic["model_flag"] = json!("--model");
    fixture.write_adapter(ascetic);
    fixture
        .compile(seat("courier", None, Some(json!(["GH_TOKEN"]))))
        .expect("an untrusted driver may still be cleared to receive");
    assert!(fixture
        .refusal(seat("courier", Some("gate"), None))
        .contains("does not hold the trusted tier"));
    let mut ascetic_gate = seat("ascetic", Some("gate"), None);
    ascetic_gate["driver"]["command"] = json!([
        "{brokkr}",
        "driver",
        "ascetic",
        "--",
        "--model",
        "ascetic-1",
        "true"
    ]);
    fixture
        .compile(ascetic_gate)
        .expect("a trusted driver judges");
    assert!(fixture
        .refusal(seat("ascetic", None, Some(json!(["GH_TOKEN"]))))
        .contains("whose egress class is uncontracted"));
}

// ------------------- decision 0036: the class belongs to the route

/// One provider fronting three destinations, which is the shape the
/// whole decision exists for: the operator's own hardware, a third party
/// they have ruled acceptable, and one nobody has ruled anything about —
/// all behind a single CLI, and so a single adapter.
fn many_routes() -> Value {
    json!({
        "provider": "many",
        "efforts": [],
        "effort_flag": "unsupported",
        "binary": "many",
        "driver": ["{brokkr}", "driver", "many", "--"],
        "trust_tier": "untrusted",
        "egress": "uncontracted",
        "routes": {"nearby": "local", "partner": "contracted"},
        "models": {"near": "nearby/small-1", "far": "partner/large-1"},
        "model_flag": "--model",
        "tool_permissions": "unsupported",
        "mcp": "unsupported",
    })
}

/// A seat on the `many` provider, pinning one concrete model id.
fn routed_seat(model: &str, class: Option<&str>, secrets: Option<Value>) -> Value {
    let mut value = json!({
        "role": "roles/role.md",
        "results": ["pass", "fail"],
        "driver": {"command":
            ["{brokkr}", "driver", "many", "--", "--model", model, "true"]},
    });
    if let Some(class) = class {
        value["class"] = json!(class);
    }
    if let Some(secrets) = secrets {
        value["secrets"] = secrets;
    }
    value
}

#[test]
fn one_adapter_binds_on_its_local_route_and_refuses_on_the_cloud_one_beside_it() {
    // The consequence, made mechanical: the operator's own hardware can
    // hold a binding without clearing the cloud route that shares its
    // CLI. Before decision 0036 the declaration site was the binary, so
    // both of these answered to one word.
    let fixture = Fixture::new();
    fixture.write_adapter(many_routes());
    fixture
        .compile(routed_seat("nearby/small-1", None, Some(json!(["TOKEN"]))))
        .expect("a local route may receive a binding");
    fixture
        .compile(routed_seat("partner/large-1", None, Some(json!(["TOKEN"]))))
        .expect("a contracted route meets the default minimum");

    // A route this adapter does not name is uncontracted: silence about
    // a route is not a promotion, and it is not an inheritance either.
    let refusal = fixture.refusal(routed_seat(
        "elsewhere/large-1",
        None,
        Some(json!(["TOKEN"])),
    ));
    assert!(refusal.contains("driver 'many'"), "{refusal}");
    assert!(refusal.contains("on route 'elsewhere'"), "{refusal}");
    assert!(
        refusal.contains("whose egress class is uncontracted"),
        "{refusal}"
    );

    // And an UNPREFIXED id gets the adapter's own class and no better:
    // it reaches whatever default the harness profile resolves, and a
    // `local` route on the same adapter lends it nothing.
    let refusal = fixture.refusal(routed_seat("small-1", None, Some(json!(["TOKEN"]))));
    assert!(
        refusal.contains("on its own declared destination"),
        "{refusal}"
    );
    assert!(
        refusal.contains("whose egress class is uncontracted"),
        "{refusal}"
    );
}

#[test]
fn a_ruling_on_one_destination_clears_no_other_the_same_binary_reaches() {
    // The decision's first rejected alternative, held shut at the
    // compile: "granting `dsh` the binding grant clears the Alibaba and
    // DeepSeek routes at the same stroke" is fail-open on the axis that
    // exists to fail closed, and moving the declaration from the boolean
    // to the routes map only closes it if a class declared for the
    // endpoint the file NAMES stops short of the endpoints it does not.
    //
    // Every other fixture here is uncontracted at the adapter, where the
    // two readings agree; this one is not, so it is the only place the
    // difference is visible.
    let fixture = Fixture::new();
    let mut ruled = many_routes();
    ruled["egress"] = json!("contracted");
    fixture.write_adapter(ruled);

    // The destination the operator ruled acceptable: it binds, on the
    // adapter's own word about its own default.
    fixture
        .compile(routed_seat("small-1", None, Some(json!(["TOKEN"]))))
        .expect("the adapter's own destination is the one it declared for");

    // The destination nobody ruled anything about, behind the very same
    // binary and the very same declaration: refused, at the floor.
    let refusal = fixture.refusal(routed_seat(
        "elsewhere/large-1",
        None,
        Some(json!(["TOKEN"])),
    ));
    assert!(refusal.contains("on route 'elsewhere'"), "{refusal}");
    assert!(
        refusal.contains("whose egress class is uncontracted"),
        "{refusal}"
    );
    assert!(
        refusal.contains("an undeclared class is uncontracted"),
        "{refusal}"
    );

    // And the route the file DOES name still stands on its own word,
    // which is the whole reason the declaration moved to the route.
    fixture
        .compile(routed_seat("nearby/small-1", None, Some(json!(["TOKEN"]))))
        .expect("a declared local route clears a contracted bar");
}

#[test]
fn a_pin_this_compiler_cannot_read_inherits_no_clearance_from_its_adapter() {
    // The gap between "this argv names NO model" and "this argv names a
    // model I cannot read". Ruling 2 gives the first the adapter's own
    // class, because an unprefixed id genuinely arrives at the
    // destination that class is the operator's word about. The second is
    // a site steering the binary somewhere and saying so illegibly — the
    // adapter's word does not reach it, so it falls to the floor with
    // every other route the file never named.
    //
    // `enforce_model_pins` catches this shape first for the four
    // model-bearing built-ins, so it is only observable on a provider
    // the operator ADDED, which is exactly the case that has no other
    // guard. `many` is contracted at the adapter here for the same
    // reason as the test above: it is the only fixture shape where
    // inheriting would be visible rather than a coincidence of the
    // floor.
    let fixture = Fixture::new();
    let mut ruled = many_routes();
    ruled["egress"] = json!("contracted");
    fixture.write_adapter(ruled);

    // The control: no `--model` at all, and the adapter's own class
    // carries the seat, as it does for `exec` on the shipped tree.
    fixture
        .compile(unpinned_seat(json!([
            "{brokkr}", "driver", "many", "--", "true"
        ])))
        .expect("an argv naming no model rides its adapter's own destination");

    // Four ways to write a pin this compiler will not read: outside the
    // id alphabet, over-long, pinned twice, and dangling. Each one has
    // an endpoint behind it that nobody ruled on.
    for command in [
        json!(["{brokkr}", "driver", "many", "--", "--model", "partner/qwen@2024"]),
        json!([
            "{brokkr}", "driver", "many", "--",
            "--model",
            "partner/arn:aws:bedrock:eu-central-1:000000000000:inference-profile/eu.anthropic.claude"
        ]),
        json!([
            "{brokkr}", "driver", "many", "--",
            "--model", "nearby/small-1", "--model", "elsewhere/large-1"
        ]),
        json!(["{brokkr}", "driver", "many", "--", "--model"]),
    ] {
        let refusal = fixture.refusal(unpinned_seat(command.clone()));
        assert!(refusal.contains("driver 'many'"), "{command}: {refusal}");
        assert!(
            refusal.contains("on a destination it does not name"),
            "{command}: {refusal}"
        );
        assert!(
            refusal.contains("whose egress class is uncontracted"),
            "{command}: {refusal}"
        );
    }
}

/// A `many` seat whose argv is written out in full, so a test can say
/// what the `--model` looks like rather than only what it resolves to.
fn unpinned_seat(command: Value) -> Value {
    json!({
        "role": "roles/role.md",
        "results": ["pass", "fail"],
        "secrets": ["TOKEN"],
        "driver": {"command": command},
    })
}

#[test]
fn a_route_named_on_the_adapters_own_flag_is_the_route_that_is_read() {
    // The second door, held shut. `model_flag` is per-adapter data: a
    // provider the operator ADDS may take `-m`, and it is told its
    // model on `-m`, so that is where the route is written. A resolver
    // reading a hardcoded `--model` finds nothing on such an argv,
    // calls the site unpinned, and hands it the adapter's own class —
    // clearing a route nobody ruled on because the pin naming it was
    // read with the wrong key. That is decision 0036's first rejected
    // alternative ("granting the grant clears every route at the same
    // stroke") arriving by a different route than the one ruling 2's
    // asymmetry watches.
    //
    // `many` is contracted at the adapter here for the same reason as
    // the two tests above: it is the only shape where inheriting the
    // adapter's word is visible rather than a coincidence of the floor.
    let fixture = Fixture::new();
    let mut takes_dash_m = many_routes();
    takes_dash_m["egress"] = json!("contracted");
    takes_dash_m["model_flag"] = json!("-m");
    fixture.write_adapter(takes_dash_m);

    // The probe: a route this adapter never named, pinned on the flag
    // this adapter actually takes, under a seat that binds a secret.
    let refusal = fixture.refusal(unpinned_seat(json!([
        "{brokkr}",
        "driver",
        "many",
        "--",
        "-m",
        "elsewhere/large-1",
        "true"
    ])));
    assert!(refusal.contains("driver 'many'"), "{refusal}");
    assert!(refusal.contains("on route 'elsewhere'"), "{refusal}");
    assert!(
        refusal.contains("whose egress class is uncontracted"),
        "{refusal}"
    );

    // The same flag reads the declared routes too — the fix moves which
    // key is read, not which rule is applied. `nearby` is local and
    // clears the bar; the adapter's own contracted word still carries
    // an argv that pins nothing at all.
    fixture
        .compile(unpinned_seat(json!([
            "{brokkr}",
            "driver",
            "many",
            "--",
            "-m",
            "nearby/small-1",
            "true"
        ])))
        .expect("a declared local route, named on the flag the adapter takes");
    fixture
        .compile(unpinned_seat(json!([
            "{brokkr}", "driver", "many", "--", "true"
        ])))
        .expect("an argv naming no model rides its adapter's own destination");

    // And `--model` is read on this adapter too (decision 0040 ruling
    // 1), which is the reviewer's finding closed. A real CLI taking
    // `-m` commonly honours `--model` as well, so a seat writing the
    // flag the ENGINE composes sends its material to `elsewhere` — and
    // the read that stopped at the declaration called the site unpinned
    // and handed it the adapter's own contracted word. Fail-open, on
    // exactly the population decision 0036 was written for.
    let refusal = fixture.refusal(unpinned_seat(json!([
        "{brokkr}",
        "driver",
        "many",
        "--",
        "--model",
        "elsewhere/large-1",
        "true"
    ])));
    assert!(refusal.contains("driver 'many'"), "{refusal}");
    assert!(refusal.contains("on route 'elsewhere'"), "{refusal}");
    assert!(
        refusal.contains("whose egress class is uncontracted"),
        "{refusal}"
    );

    // Both flags name the route on the same terms: a declared local one
    // written on `--model` binds, just as it does on `-m`.
    fixture
        .compile(unpinned_seat(json!([
            "{brokkr}",
            "driver",
            "many",
            "--",
            "--model",
            "nearby/small-1",
            "true"
        ])))
        .expect("a declared local route, named on the flag the engine composes");

    // An unreadable pin names the flag it was read on, not a constant.
    let refusal = fixture.refusal(unpinned_seat(json!([
        "{brokkr}",
        "driver",
        "many",
        "--",
        "-m",
        "partner/qwen@2024",
        "true"
    ])));
    assert!(refusal.contains("its '-m' pin is not one"), "{refusal}");
}

#[test]
fn a_value_attached_to_a_short_flag_is_the_pin_that_flag_carries() {
    // Decision 0040 ruling 2, on the shape that has three spellings.
    // A SHORT flag is one dash and one character, and the getopt
    // convention attaches its value to it bare: `-melsewhere/large-1`
    // is how every CLI this fleet has met is told a model on `-m`. So
    // the remainder IS the pin, and the route it names is the route the
    // material goes to — the reading a walker that stopped at
    // `FLAG VALUE` and `FLAG=VALUE` could not give.
    //
    // Before the ruling this shape was `Unreadable`: fail-closed and
    // honest, but it refused an argv that named a route the file
    // declares, and it named the wrong problem doing so. The operator
    // has now ruled the grammar the compiler declined to invent.
    let fixture = Fixture::new();
    let mut takes_dash_m = many_routes();
    takes_dash_m["egress"] = json!("contracted");
    takes_dash_m["model_flag"] = json!("-m");
    fixture.write_adapter(takes_dash_m);

    // The probe: the attached spelling, naming a route nobody ruled on,
    // under a seat that binds a secret. Refused BY ROUTE, because the
    // compiler now reads what the argv says.
    let refusal = fixture.refusal(unpinned_seat(json!([
        "{brokkr}",
        "driver",
        "many",
        "--",
        "-melsewhere/large-1",
        "true"
    ])));
    assert!(refusal.contains("driver 'many'"), "{refusal}");
    assert!(refusal.contains("on route 'elsewhere'"), "{refusal}");
    assert!(
        refusal.contains("whose egress class is uncontracted"),
        "{refusal}"
    );

    // And the same spelling over a route this adapter declares LOCAL
    // clears the bar, on the word it actually wrote.
    fixture
        .compile(unpinned_seat(json!([
            "{brokkr}",
            "driver",
            "many",
            "--",
            "-mnearby/small-1",
            "true"
        ])))
        .expect("the attached spelling names its route like any other");

    // The cost of reading it, stated rather than hidden: a remainder
    // that is not one concrete model id is illegible, so a word meant
    // for something else entirely costs a secret-binding seat a refusal
    // naming the flag, and nothing else. Ruling 2 names this case.
    let refusal = fixture.refusal(unpinned_seat(json!([
        "{brokkr}",
        "driver",
        "many",
        "--",
        "-march=native",
        "true"
    ])));
    assert!(
        refusal.contains("on a destination it does not name"),
        "{refusal}"
    );
    assert!(refusal.contains("its '-m' pin is not one"), "{refusal}");
    assert!(
        refusal.contains("whose egress class is uncontracted"),
        "{refusal}"
    );

    // A pin named twice is still a pin named twice, in any mixture of
    // the three spellings this flag now has: reading the attached form
    // does not let a second destination in behind the first.
    for command in [
        json!([
            "{brokkr}",
            "driver",
            "many",
            "--",
            "-mnearby/small-1",
            "-m",
            "partner/large-1",
            "true"
        ]),
        json!([
            "{brokkr}",
            "driver",
            "many",
            "--",
            "-mnearby/small-1",
            "-mpartner/large-1",
            "true"
        ]),
        json!([
            "{brokkr}",
            "driver",
            "many",
            "--",
            "-m=nearby/small-1",
            "-mpartner/large-1",
            "true"
        ]),
    ] {
        let refusal = fixture.refusal(unpinned_seat(command.clone()));
        assert!(
            refusal.contains("its '-m' pin is not one"),
            "{command}: {refusal}"
        );
    }

    // The two spellings this compiler does read are untouched, on both
    // sides of the bar: the declared local route still clears it, and
    // the route nobody named is still refused BY NAME, because there it
    // genuinely read one.
    for command in [
        json!([
            "{brokkr}",
            "driver",
            "many",
            "--",
            "-m",
            "nearby/small-1",
            "true"
        ]),
        json!([
            "{brokkr}",
            "driver",
            "many",
            "--",
            "-m=nearby/small-1",
            "true"
        ]),
    ] {
        fixture
            .compile(unpinned_seat(command.clone()))
            .unwrap_or_else(|error| panic!("{command}: {error}"));
    }
    let refusal = fixture.refusal(unpinned_seat(json!([
        "{brokkr}",
        "driver",
        "many",
        "--",
        "-m=elsewhere/large-1",
        "true"
    ])));
    assert!(refusal.contains("on route 'elsewhere'"), "{refusal}");

    // And an argv that never writes the flag at all is still the other
    // silence: unpinned, riding the adapter's own contracted word.
    fixture
        .compile(unpinned_seat(json!([
            "{brokkr}", "driver", "many", "--", "true"
        ])))
        .expect("an argv naming no model rides its adapter's own destination");
}

#[test]
fn an_adapter_that_cannot_be_told_a_model_rides_its_own_class_and_no_better() {
    // `model_flag: "unsupported"` is a measured fact about a CLI: there
    // is no flag this engine composes a pin onto. An argv that names no
    // model at all therefore reaches whatever the provider's own
    // profile resolves, which is the unprefixed case, on the adapter's
    // own class — and NO BETTER, however the models map is written.
    let fixture = Fixture::new();
    let mut untellable = many_routes();
    untellable["model_flag"] = json!("unsupported");
    fixture.write_adapter(untellable);

    let refusal = fixture.refusal(unpinned_seat(json!([
        "{brokkr}", "driver", "many", "--", "true"
    ])));
    assert!(
        refusal.contains("on its own declared destination"),
        "{refusal}"
    );
    assert!(
        refusal.contains("whose egress class is uncontracted"),
        "{refusal}"
    );

    // But `--model` is read here too (decision 0040 ruling 1), and a
    // pin found on it is `Unreadable` — never the adapter's own class.
    // A provider that cannot be told a model has no route to name, so a
    // site writing one is telling a binary something the adapter says
    // it cannot hear: the machine cannot say where that material lands,
    // and a seat binding a secret does not get the benefit of a doubt
    // the machine genuinely has.
    //
    // `many` is contracted at the adapter here for the same reason as
    // every test above: it is the only shape where inheriting the
    // adapter's word is visible rather than a coincidence of the floor.
    let fixture = Fixture::new();
    let mut ruled = many_routes();
    ruled["egress"] = json!("contracted");
    ruled["model_flag"] = json!("unsupported");
    fixture.write_adapter(ruled);
    for command in [
        // A route the file declares LOCAL, which lends this site
        // nothing: the read is refused, not promoted.
        json!([
            "{brokkr}",
            "driver",
            "many",
            "--",
            "--model",
            "nearby/small-1",
            "true"
        ]),
        json!([
            "{brokkr}",
            "driver",
            "many",
            "--",
            "--model",
            "elsewhere/large-1",
            "true"
        ]),
        json!(["{brokkr}", "driver", "many", "--", "--model", "spark/x", "true"]),
    ] {
        let refusal = fixture.refusal(unpinned_seat(command.clone()));
        assert!(
            refusal.contains("on a destination it does not name"),
            "{command}: {refusal}"
        );
        assert!(
            refusal.contains("its '--model' pin is not one"),
            "{command}: {refusal}"
        );
        assert!(
            refusal.contains("whose egress class is uncontracted"),
            "{command}: {refusal}"
        );
    }

    // The refusal names `--model` and only `--model`: there is no
    // declared flag for this read to have used, and no constant stands
    // in for one (decision 0040 ruling 3).
    let refusal = fixture.refusal(unpinned_seat(json!([
        "{brokkr}", "driver", "many", "--", "--model", "spark/x", "true"
    ])));
    assert!(!refusal.contains("unsupported'"), "{refusal}");

    // An argv that writes the flag illegibly is illegible on the same
    // terms, and one that never writes it at all still rides the
    // adapter's own contracted word.
    let refusal = fixture.refusal(unpinned_seat(json!([
        "{brokkr}", "driver", "many", "--", "--model"
    ])));
    assert!(
        refusal.contains("its '--model' pin is not one"),
        "{refusal}"
    );
    fixture
        .compile(unpinned_seat(json!([
            "{brokkr}", "driver", "many", "--", "true"
        ])))
        .expect("an argv naming no model rides its adapter's own destination");
}

/// Decision 0040 ruling 1, on the shape it was written for: an adapter
/// whose declared flag and `--model` are DIFFERENT strings. A concrete
/// pin on either names the route, because the material goes where the
/// argv says on whichever flag the provider honours; two concrete pins
/// naming different ids can only be refused, because the material goes
/// one place and the argv says two.
#[test]
fn two_flags_naming_two_destinations_are_refused_naming_both() {
    let fixture = Fixture::new();
    let mut takes_dash_m = many_routes();
    takes_dash_m["egress"] = json!("contracted");
    takes_dash_m["model_flag"] = json!("-m");
    takes_dash_m["routes"] = json!({"nearby": "local", "partner": "contracted", "spark": "local"});
    fixture.write_adapter(takes_dash_m);

    // Two ids, two flags, one body of material: refused, and the
    // refusal names both flags the read used — not one of them, and not
    // a constant standing in for the other.
    let refusal = fixture.refusal(unpinned_seat(json!([
        "{brokkr}",
        "driver",
        "many",
        "--",
        "-m",
        "nearby/small-1",
        "--model",
        "elsewhere/large-1",
        "true"
    ])));
    assert!(refusal.contains("driver 'many'"), "{refusal}");
    assert!(
        refusal.contains("its '-m' and '--model' pins are not one"),
        "{refusal}"
    );
    assert!(
        refusal.contains("whose egress class is uncontracted"),
        "{refusal}"
    );

    // Even where both routes would clear the bar on their own: the
    // refusal is about what the argv cannot mean, not about where it
    // would have gone.
    let refusal = fixture.refusal(unpinned_seat(json!([
        "{brokkr}",
        "driver",
        "many",
        "--",
        "-m",
        "nearby/small-1",
        "--model",
        "spark/x",
        "true"
    ])));
    assert!(
        refusal.contains("its '-m' and '--model' pins are not one"),
        "{refusal}"
    );

    // The SAME id on both flags is not two destinations, so it reads as
    // the one pin it is.
    fixture
        .compile(unpinned_seat(json!([
            "{brokkr}", "driver", "many", "--", "-m", "spark/x", "--model", "spark/x", "true"
        ])))
        .expect("one id written twice names one route");

    // And a pin on the declared flag ALONE still binds on its route,
    // which is the read #161 established and this ruling does not
    // disturb.
    fixture
        .compile(unpinned_seat(json!([
            "{brokkr}", "driver", "many", "--", "-m", "spark/x", "true"
        ])))
        .expect("a local route named on the flag the adapter declares still binds");

    // An illegible read on either flag is illegible for the site, and
    // names the flag it was read on.
    let refusal = fixture.refusal(unpinned_seat(json!([
        "{brokkr}",
        "driver",
        "many",
        "--",
        "-m",
        "spark/x",
        "--model",
        "partner/qwen@2024",
        "true"
    ])));
    assert!(
        refusal.contains("its '--model' pin is not one"),
        "{refusal}"
    );
    assert!(!refusal.contains("'-m' and"), "{refusal}");
}

#[test]
fn the_operator_rules_the_minimum_into_the_bundle() {
    // Ruling 4's bar is the operator's, not the engine's. Raised, the
    // contracted route that just compiled refuses; lowered, the
    // uncontracted one that just refused compiles. Nothing about the
    // adapter data moved between these two compilations.
    let fixture = Fixture::new();
    fixture.write_adapter(many_routes());
    let adapters = fixture.dir.path().join("adapters");

    fixture
        .compile_under(
            routed_seat("nearby/small-1", None, Some(json!(["TOKEN"]))),
            Some(json!("local")),
            &adapters,
        )
        .expect("the local route meets a local bar");
    let refusal = fixture
        .compile_under(
            routed_seat("partner/large-1", None, Some(json!(["TOKEN"]))),
            Some(json!("local")),
            &adapters,
        )
        .expect_err("a contracted route does not meet a local bar")
        .to_string();
    assert!(
        refusal.contains("whose egress class is contracted"),
        "{refusal}"
    );
    assert!(refusal.contains("binds no secret below local"), "{refusal}");

    fixture
        .compile_under(
            routed_seat("elsewhere/large-1", None, Some(json!(["TOKEN"]))),
            Some(json!("uncontracted")),
            &adapters,
        )
        .expect("an operator may rule the floor as the bar");
}

#[test]
fn a_minimum_outside_the_vocabulary_refuses() {
    // Closed vocabulary, in the manner of every other declaration here:
    // a misspelled bar must never read as the default, or a bundle would
    // bind under a rule nobody wrote.
    let fixture = Fixture::new();
    let adapters = fixture.dir.path().join("adapters");
    for written in [json!("contracted-ish"), json!(2)] {
        let refusal = fixture
            .compile_under(seat("judge", None, None), Some(written), &adapters)
            .expect_err("an unreadable bar is not a bar")
            .to_string();
        assert!(refusal.contains("'egress_minimum' is"), "{refusal}");
        assert!(
            refusal.contains("the egress vocabulary is closed"),
            "{refusal}"
        );
        assert!(refusal.contains("0036 ruling 4"), "{refusal}");
    }
}

#[test]
fn a_local_route_earns_no_gate_seat() {
    // Ruling 3, as law: local is STRUCTURAL — it is a fact about where
    // an endpoint runs, not a track record — so it confers nothing on
    // the judging axis. The gate refusal reads `trust_tier` and nothing
    // else, and a model on the operator's own hardware may be the most
    // private worker in the fleet and remain the least qualified to be
    // the check.
    let fixture = Fixture::new();
    fixture.write_adapter(many_routes());
    let refusal = fixture.refusal(routed_seat("nearby/small-1", Some("gate"), None));
    assert!(refusal.contains("driver 'many'"), "{refusal}");
    assert!(
        refusal.contains("does not hold the trusted tier"),
        "{refusal}"
    );
    assert!(refusal.contains("0021 ruling 2"), "{refusal}");
    // And nothing about egress appears in that refusal: the two axes
    // never braid, in either direction.
    assert!(!refusal.contains("egress"), "{refusal}");
}

#[test]
fn an_agent_chains_route_resolves_through_the_adapter_that_maps_it() {
    // An agent site names an ABSTRACT model; the route is the prefix of
    // the CONCRETE id the adapter maps it to. Every link of the chain is
    // resolved, for the same reason the tier is: a chain that could fall
    // back onto an uncontracted route at run time would have cleared the
    // binding at compile time.
    let fixture = Fixture::new();
    let mut local_only = many_routes();
    local_only["models"] = json!({"near": "nearby/small-1"});
    fixture.write_adapter(local_only);
    std::fs::write(
        fixture.dir.path().join("agents/charters/homely.md"),
        "# charter\n",
    )
    .unwrap();
    let write_agent = |chain: Value| {
        std::fs::write(
            fixture.dir.path().join("agents/homely.json"),
            serde_json::to_string(&json!({
                "description": "a fixture agent",
                "charter": "charters/homely.md",
                "models": chain,
            }))
            .unwrap(),
        )
        .unwrap();
    };
    let bound = json!({
        "results": ["pass", "fail"],
        "secrets": ["TOKEN"],
        "agent": "homely",
    });

    write_agent(json!(["near"]));
    fixture
        .compile_under(
            bound.clone(),
            Some(json!("local")),
            &fixture.dir.path().join("adapters"),
        )
        .expect("the whole chain sits on the local route");

    // A second link on the adapter's own destination — an UNPREFIXED
    // id, which is the case that reads the adapter's own class — and the
    // same seat refuses, naming the link that would have carried the
    // value out.
    let mut serving = adapter("elsewhere", Some("untrusted"), Some(true));
    serving["models"] = json!({"faraway": "large-1"});
    serving["model_flag"] = json!("--model");
    fixture.write_adapter(serving);
    write_agent(json!(["near", "faraway"]));
    let refusal = fixture
        .compile_under(
            bound,
            Some(json!("local")),
            &fixture.dir.path().join("adapters"),
        )
        .expect_err("a fallback onto a contracted route is a compile-time fact")
        .to_string();
    assert!(refusal.contains("driver 'elsewhere'"), "{refusal}");
    assert!(
        refusal.contains("whose egress class is contracted"),
        "{refusal}"
    );
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
fn whole_gate_class_rejects_empty_and_driverless_shapes() {
    assert!(!is_gate_class(&Value::Null));
    assert!(!is_gate_class(&json!({"panel": {}})));
    assert!(!is_gate_class(&json!({"sequence": []})));
}

#[test]
fn a_seat_that_bears_no_driver_may_not_carry_a_class() {
    // `recipes/triage`'s design seat is the reason: a panel of work
    // positions, a work chief and a gate check cannot share one word.
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
            {"name": "first", "results": ["pass", "fail"], "role": "roles/role.md",
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

#[test]
fn a_panel_may_not_mix_judges_and_workers() {
    let fixture = Fixture::new();
    let refusal = fixture.refusal(json!({
        "results": ["pass", "fail"],
        "aggregate": "unanimous-pass",
        "panel": {
            "judge": {"role": "roles/role.md", "class": "gate",
                      "driver": {"command": ["{brokkr}", "driver", "judge", "--", "true"]}},
            "smith": {"role": "roles/role.md", "class": "work",
                      "driver": {"command": ["{brokkr}", "driver", "judge", "--", "true"]}}
        }
    }));
    assert!(
        refusal.contains("seat 'work' is a mixed panel"),
        "{refusal}"
    );
    assert!(refusal.contains("gate members [judge]"), "{refusal}");
    assert!(refusal.contains("work members [smith]"), "{refusal}");
}

// ----------------------------- the vocabulary a declaration is written in

#[test]
fn a_misspelled_class_refuses_rather_than_reading_as_work() {
    // The whole fail-closed reading turns on this: `class` is read by
    // ABSENCE, so a key this compiler cannot see is a declaration that
    // was never made. If `clas` were tolerated, a gate would compile as
    // work and both refusals would be off, silently.
    let fixture = Fixture::new();
    let mut work = seat("newcomer", None, None);
    work["clas"] = json!("gate");
    let refusal = fixture.refusal(work);
    assert!(refusal.contains("unknown key 'clas'"), "{refusal}");
    assert!(refusal.contains("known: results"), "{refusal}");
    assert!(refusal.contains("0021 ruling 1"), "{refusal}");

    // And the same for the other declaration a silence would swallow.
    let mut work = seat("newcomer", None, None);
    work["secret"] = json!(["GH_TOKEN"]);
    assert!(fixture.refusal(work).contains("unknown key 'secret'"));
}

#[test]
fn a_panel_member_and_a_sequence_step_have_their_own_narrower_vocabulary() {
    // A member has no `results`/`limits`/`inputs`/`secrets` of its own —
    // the seat above it does — so writing one there could only be
    // discarded, which is exactly the silence this refusal closes.
    let fixture = Fixture::new();
    let refusal = fixture.refusal(json!({
        "results": ["pass", "fail"],
        "aggregate": "unanimous-pass",
        "panel": {
            "a": {"role": "roles/role.md",
                  "driver": {"command": ["{brokkr}", "driver", "judge", "--", "true"]}},
            "b": {"role": "roles/role.md", "secrets": ["GH_TOKEN"],
                  "driver": {"command": ["{brokkr}", "driver", "judge", "--", "true"]}},
        },
    }));
    assert!(refusal.contains("seat 'work:b'"), "{refusal}");
    assert!(refusal.contains("unknown key 'secrets'"), "{refusal}");

    let refusal = fixture.refusal(json!({
        "results": ["pass", "fail"],
        "sequence": [
            {"name": "first", "results": ["pass", "fail"], "role": "roles/role.md",
             "driver": {"command": ["{brokkr}", "driver", "judge", "--", "true"]}},
            {"name": "second", "clas": "gate", "role": "roles/role.md",
             "driver": {"command": ["{brokkr}", "driver", "judge", "--", "true"]}},
        ],
    }));
    assert!(refusal.contains("seat 'work:second'"), "{refusal}");
    assert!(refusal.contains("unknown key 'clas'"), "{refusal}");
}

#[test]
fn a_seat_that_is_not_an_object_at_all_keeps_its_own_refusal() {
    // Nothing to spell wrong in a string, so the vocabulary has nothing
    // to say: the seat falls through to the refusal it always had.
    let fixture = Fixture::new();
    let refusal = fixture.refusal(json!("judge"));
    assert!(refusal.contains("needs non-empty 'results'"), "{refusal}");
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
                                ["{brokkr}", "driver", "judge", "--model", "judge-1", "--", "true"]}},
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

    // A work member beside a judge is refused before either member can
    // run without the whole-effect gate guard.
    let refusal = fixture.refusal(panel(json!({
        "class": "work",
        "role": "roles/role.md",
        "driver": {"command": ["{brokkr}", "driver", "newcomer", "--", "true"]},
    })));
    assert!(refusal.contains("mixed panel"), "{refusal}");
}

#[test]
fn a_sequence_step_is_classed_and_refused_on_its_own() {
    let fixture = Fixture::new();
    let refusal = fixture.refusal(json!({
        "results": ["pass", "fail"],
        "sequence": [
            {"name": "positions", "results": ["pass", "fail"], "class": "work", "role": "roles/role.md",
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
    assert!(
        refusal.contains("whose egress class is uncontracted"),
        "{refusal}"
    );
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
    //
    // Decision 0036 ruling 4's MIGRATION test, and the reason it sits
    // here rather than in a new file: the clearances are the same five
    // facts, re-read through the class vocabulary. `binding_grant: true`
    // reads as `contracted`, and a `false` or absent grant as
    // `uncontracted`, so every adapter carried its old clearance across
    // the enactment unchanged. What has moved since is one operator
    // RULING, not a migration: `dsh`'s `spark` route is `local` as of
    // 2026-09-03. The adapter-level clearances below are still the five
    // the migration pinned — a route class changes no adapter's own.
    let adapters = Adapters::load(&shipped_adapters()).expect("the shipped adapters load");
    for (provider, tier, egress) in [
        ("claude", TrustTier::Trusted, EgressClass::Contracted),
        // 0021 addendum, operator ruled 2026-09-02: trusted for every
        // seat class; the clearance to RECEIVE stays a separate bar.
        ("codex", TrustTier::Trusted, EgressClass::Uncontracted),
        ("dsh", TrustTier::Untrusted, EgressClass::Uncontracted),
        ("exec", TrustTier::Untrusted, EgressClass::Contracted),
        // Not named by this slice's scope, and so left undeclared:
        // fail-closed defaults are the correct reading of silence.
        ("lanetally", TrustTier::Untrusted, EgressClass::Uncontracted),
    ] {
        let adapter = adapters.adapter(provider).expect("a shipped adapter");
        assert_eq!(adapter.trust_tier, tier, "{provider} tier");
        assert_eq!(adapter.egress, egress, "{provider} egress");
        // Four of the five front one destination each, so they declare
        // one class at the adapter and no routes (ruling 2), and every
        // model they map resolves to exactly where it stood the day
        // before this decision landed. `dsh` is the exception the
        // operator has since ruled on, and it is pinned in full below.
        if provider == "dsh" {
            continue;
        }
        assert!(adapter.routes.is_empty(), "{provider} declares no route");
        for model in adapter.models.values() {
            assert_eq!(
                resolve_route(adapter, model).1,
                egress,
                "{provider} model '{model}' stays where it stood"
            );
        }
    }
    assert!(adapters.adapter("nobody").is_none());

    // The operator ruled on 2026-09-03 that `dsh`'s `spark` route — the
    // DGX Spark in their own building — is `local`. That ruling is the
    // whole of what they classed, so this states all three of `dsh`'s
    // fronts rather than dropping the assertion that used to cover
    // them: `uncontracted` now means two different things behind this
    // one binary, and a test that said only "the floor" would stop
    // telling the adapter's own word apart from nobody's word.
    let dsh = adapters.adapter("dsh").expect("a shipped adapter");
    assert_eq!(dsh.routes.len(), 1, "dsh classes exactly one route");
    assert_eq!(
        dsh.routes.get("spark"),
        Some(&EgressClass::Local),
        "the operator's own hardware, ruled 2026-09-03"
    );
    for (model, expected, ground) in [
        (
            "deepseek-v4-pro",
            EgressClass::Uncontracted,
            "unprefixed: dsh's own adapter class, because the id reaches \
             whatever the harness profile resolves",
        ),
        (
            "dashscope/qwen3.8-max",
            EgressClass::Uncontracted,
            "a route this file does not name: the floor by ruling 1, and \
             no longer by the adapter declaring no routes at all",
        ),
        (
            "spark/qwen3.8-flash",
            EgressClass::Local,
            "the route the operator ruled: local, and the Alibaba front \
             beside it is not carried along",
        ),
    ] {
        assert_eq!(
            resolve_route(dsh, model).1,
            expected,
            "dsh '{model}' — {ground}"
        );
    }
    // And completely, over every model `dsh` maps: local exactly where
    // the id is a `spark/` one, uncontracted everywhere else.
    for model in dsh.models.values() {
        let expected = match model.starts_with("spark/") {
            true => EgressClass::Local,
            false => EgressClass::Uncontracted,
        };
        assert_eq!(resolve_route(dsh, model).1, expected, "dsh '{model}'");
    }
}

#[test]
fn the_shipped_adapters_declare_exactly_the_decision_0041_judges() {
    let adapters = Adapters::load(&shipped_adapters()).expect("the shipped adapters load");
    for (provider, judges) in [
        ("claude", vec!["fable", "opus"]),
        ("codex", vec!["astra", "sol"]),
        ("dsh", vec![]),
        ("exec", vec![]),
        ("lanetally", vec!["fable-tallied", "opus-tallied"]),
    ] {
        assert_eq!(
            adapters.adapter(provider).unwrap().judges,
            judges,
            "{provider}"
        );
    }
}

/// The path to the adapters this repository actually ships.
fn shipped_adapters() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .join("adapters")
}

#[test]
fn the_shipped_codex_adapter_may_now_hold_a_gate() {
    // The 0021 addendum (operator ruled 2026-09-02) made real, at the
    // one place it can be observed: a gate seat whose driver is `codex`
    // compiles against the SHIPPED `adapters/codex.json`, not against a
    // fixture provider. Yesterday this same bundle refused — the tier
    // was `untrusted`, and ruling 2 kept untrusted drivers off gates.
    // The sibling pin above asserts the declaration; this one asserts
    // the compiler reads it. Both must move together or neither does.
    let fixture = Fixture::new();
    fixture
        .compile_against(seat("codex", Some("gate"), None), &shipped_adapters())
        .expect("the promoted incumbent-peer may hold a gate");
}

#[test]
fn the_shipped_codex_adapter_still_binds_no_secrets() {
    // The other half of the addendum, and the half that did NOT move:
    // codex's clearance to receive stays unruled — `binding_grant:
    // false`, which decision 0036 ruling 4 reads as `uncontracted` — so
    // the ruling 4 refusal must still fire for codex on the shipped
    // adapter. A future edit that grants the tier a second axis by
    // accident trips here, naming what it took.
    let fixture = Fixture::new();
    let refusal = match fixture.compile_against(
        seat("codex", None, Some(json!(["GH_TOKEN"]))),
        &shipped_adapters(),
    ) {
        Ok(_) => panic!("a codex seat may not bind secrets"),
        Err(error) => error.to_string(),
    };
    assert!(refusal.contains("seat 'work'"), "{refusal}");
    assert!(refusal.contains("driver 'codex'"), "{refusal}");
    assert!(
        refusal.contains("whose egress class is uncontracted"),
        "{refusal}"
    );
    assert!(refusal.contains("0021 ruling 4"), "{refusal}");
}

#[test]
fn the_shipped_codex_adapter_says_why_it_cannot_restrict_tools() {
    // The 0021 addendum's first piece of enabling engineering, pinned
    // where its siblings are. `codex exec` (codex-cli 0.148.0, read off
    // the installed binary) restricts by SANDBOX CLASS — `-s|--sandbox
    // read-only|workspace-write|danger-full-access` — and offers no
    // per-tool allow-list to map a seat's declared tools onto. So the
    // capability stays absent, exactly as it was, and the fail-closed
    // refusal is unchanged; what the addendum added is the REASON,
    // measured rather than assumed. A future reader asking "was
    // `unsupported` decided or defaulted?" now has an answer in the
    // data, and an edit that deletes the answer trips here.
    let adapters = Adapters::load(&shipped_adapters()).expect("the shipped adapters load");
    let codex = adapters.adapter("codex").expect("a shipped adapter");
    assert!(
        codex.tool_permissions.is_none(),
        "codex still expresses no per-tool restriction"
    );
    let gap = codex
        .tool_permissions_gap
        .as_deref()
        .expect("and now records why, rather than leaving it to be guessed");
    assert!(gap.contains("--sandbox"), "{gap}");
    assert!(gap.contains("read-only"), "{gap}");
    assert!(gap.contains("danger-full-access"), "{gap}");
    // The neighbours are untouched by this slice: their silence stays
    // silence, so the new field cannot be mistaken for a migration
    // somebody forgot to finish.
    for quiet in ["dsh", "exec"] {
        let adapter = adapters.adapter(quiet).expect("a shipped adapter");
        assert!(adapter.tool_permissions.is_none(), "{quiet}");
        assert!(
            adapter.tool_permissions_gap.is_none(),
            "{quiet} was not measured by this slice, and does not pretend it was"
        );
    }
}

#[test]
fn the_shipped_codex_adapter_maps_the_models_its_own_cli_names() {
    // The other adapter-data half: `codex debug models` on the
    // installed codex-cli 0.148.0 listed the three gpt-5.6 slugs, and the
    // 0.153.2 catalog adds `gpt-6-astra` at priority 1 (visibility
    // "list", supported_in_api true; decision 0045), so the mapping is
    // transcribed, not remembered. The abstract names are codex's own family words —
    // NOT claude tiers, so no fallback chain written for one provider
    // can quietly land on the other.
    let adapters = Adapters::load(&shipped_adapters()).expect("the shipped adapters load");
    let codex = adapters.adapter("codex").expect("a shipped adapter");
    assert_eq!(
        codex.models.get("astra").map(String::as_str),
        Some("gpt-6-astra")
    );
    assert_eq!(
        codex.models.get("sol").map(String::as_str),
        Some("gpt-5.6-sol")
    );
    assert_eq!(
        codex.models.get("terra").map(String::as_str),
        Some("gpt-5.6-terra")
    );
    assert_eq!(
        codex.models.get("luna").map(String::as_str),
        Some("gpt-5.6-luna")
    );
    for claude_tier in ["opus", "sonnet", "haiku", "fable"] {
        assert!(
            !codex.models.contains_key(claude_tier),
            "codex must not answer to '{claude_tier}': a chain written for \
             claude landing on codex is the silent substitution 0016 refuses"
        );
    }
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

// ------------------------------------- what authorised it, in the manifest

/// The digest of one adapter file in the fixture's tree, read the way
/// the compiler reads it.
fn adapter_digest(fixture: &Fixture, provider: &str) -> String {
    Adapters::load(&fixture.dir.path().join("adapters"))
        .expect("the fixture adapters load")
        .digest(provider)
        .expect("the fixture declares this provider")
        .to_string()
}

#[test]
fn an_inline_gate_pins_the_adapter_that_authorised_it() {
    // The refusal reads a declaration; the manifest must say WHICH one,
    // or a tier demoted in `adapters/` would change what the compiler
    // allows while leaving the bundle's identity untouched.
    let fixture = Fixture::new();
    let bundle = fixture
        .compile(seat("judge", Some("gate"), None))
        .expect("a trusted driver holds a gate");
    assert_eq!(
        bundle.manifest["drivers"],
        json!({"work": {"judge": adapter_digest(&fixture, "judge")}}),
        "only the gate seat consulted a declaration; the work-class \
         review seat beside it consulted none"
    );
}

#[test]
fn an_inline_binding_pins_the_adapter_that_granted_it() {
    let fixture = Fixture::new();
    let bundle = fixture
        .compile(seat("judge", None, Some(json!(["TOKEN"]))))
        .expect("a granted driver may receive a binding");
    assert_eq!(
        bundle.manifest["drivers"],
        json!({"work": {"judge": adapter_digest(&fixture, "judge")}})
    );
}

#[test]
fn a_bundle_that_consulted_nothing_carries_no_drivers_key() {
    // ABSENT, not an empty object — the `agents` key's own rule, for the
    // same reason: a bundle that asked nobody's permission must have the
    // identity it always had.
    let fixture = Fixture::new();
    let bundle = fixture
        .compile(seat("newcomer", Some("work"), None))
        .expect("a work seat needs no tier");
    assert!(bundle.manifest.get("drivers").is_none());
}

#[test]
fn an_agent_gate_is_witnessed_by_its_resolution_not_a_second_time() {
    // An agent site already pins every adapter its chain consulted, in
    // `agents`. Recording it again under `drivers` would put the same
    // fact in the manifest twice, and the two could then disagree.
    let fixture = Fixture::new();
    fixture.write_agent("steady", &[("first", "promoted", "trusted")]);
    let bundle = fixture
        .compile(json!({
            "results": ["pass", "fail"],
            "class": "gate",
            "agent": "steady",
        }))
        .expect("a trusted chain holds a gate");
    assert!(bundle.manifest["agents"]["work"]["adapter_digest"].is_string());
    assert!(bundle.manifest.get("drivers").is_none());
}

// -------------------------------- decision 0041: gates hire judges only

#[test]
fn a_gate_on_sonnet_is_refused_and_a_gate_on_opus_is_admitted() {
    let fixture = Fixture::new();
    fixture.write_agent("sonnet-seat", &[("sonnet", "claude-sonnet", "trusted")]);
    let mut sonnet = adapter("claude-sonnet", Some("trusted"), Some(true));
    sonnet["models"] = json!({"sonnet": "claude-sonnet-5"});
    sonnet["model_flag"] = json!("--model");
    sonnet["judges"] = json!([]);
    fixture.write_adapter(sonnet);
    let refusal = fixture.refusal(json!({
        "agent": "sonnet-seat", "class": "gate", "results": ["pass", "fail"]
    }));
    assert!(refusal.contains("seat 'work'"), "{refusal}");
    assert!(refusal.contains("link 1"), "{refusal}");
    assert!(refusal.contains("model 'sonnet'"), "{refusal}");

    fixture.write_agent("opus-seat", &[("opus", "claude-opus", "trusted")]);
    fixture
        .compile(json!({
            "agent": "opus-seat", "class": "gate", "results": ["pass", "fail"]
        }))
        .expect("an adapter-declared opus judge may hold a gate");
}

#[test]
fn a_gate_chain_refuses_its_third_non_judge_link_by_site_and_link() {
    let fixture = Fixture::new();
    fixture.write_agent(
        "descending",
        &[
            ("fable", "first", "trusted"),
            ("opus", "second", "trusted"),
            ("sonnet", "third", "trusted"),
        ],
    );
    let mut third = adapter("third", Some("trusted"), Some(true));
    third["models"] = json!({"sonnet": "claude-sonnet-5"});
    third["model_flag"] = json!("--model");
    third["judges"] = json!([]);
    fixture.write_adapter(third);

    let refusal = fixture.refusal(json!({
        "agent": "descending", "class": "gate", "results": ["pass", "fail"]
    }));
    assert!(refusal.contains("seat 'work'"), "{refusal}");
    assert!(refusal.contains("link 3"), "{refusal}");
    assert!(refusal.contains("model 'sonnet'"), "{refusal}");
}

#[test]
fn a_work_seat_on_sonnet_is_untouched_by_the_judges_declaration() {
    let fixture = Fixture::new();
    fixture.write_agent("worker", &[("sonnet", "claude-sonnet", "trusted")]);
    let mut sonnet = adapter("claude-sonnet", Some("trusted"), Some(true));
    sonnet["models"] = json!({"sonnet": "claude-sonnet-5"});
    sonnet["model_flag"] = json!("--model");
    fixture.write_adapter(sonnet); // no `judges`: the fail-closed default
    fixture
        .compile(json!({
            "agent": "worker", "class": "work", "results": ["pass", "fail"]
        }))
        .expect("judges constrains gates, not work seats");
}

// ---------------------------------------------- decision 0043: boxed hands

fn exec_gate(hands: Option<Value>, secrets: Option<Value>) -> Value {
    let mut value = json!({
        "role": "roles/role.md",
        "results": ["pass", "fail"],
        "class": "gate",
        "driver": {"command": ["{brokkr}", "driver", "exec", "--", "bash", "check.sh", "{prompt_file}"]},
    });
    if let Some(hands) = hands {
        value["hands"] = hands;
    }
    if let Some(secrets) = secrets {
        value["secrets"] = secrets;
    }
    value
}

/// Ruling 3: a boxed exec command may hold a gate; an unboxed one stays
/// refused as 0021 reads. Ruling 4: the box is in the manifest, and
/// absent when nothing is boxed.
#[test]
fn a_boxed_exec_command_may_hold_a_gate_and_the_box_is_pinned() {
    let fixture = Fixture::new();
    fixture.write_adapter(adapter("exec", Some("untrusted"), Some(true)));

    let refusal = fixture.refusal(exec_gate(None, None));
    assert!(refusal.contains("gate class"), "{refusal}");
    assert!(
        refusal.contains("does not hold the trusted tier"),
        "{refusal}"
    );

    let bundle = fixture
        .compile(exec_gate(Some(json!("workspace")), None))
        .unwrap();
    let hands = bundle.manifest["hands"]
        .as_object()
        .expect("the box is pinned");
    assert_eq!(hands["work"]["kind"], "workspace");
    assert_eq!(hands["work"]["network"], false);
    assert_eq!(hands["work"]["binds"], json!([]));
    assert_eq!(bundle.hands.len(), 1);

    let plain = fixture.compile(seat("judge", Some("gate"), None)).unwrap();
    assert!(
        plain.manifest.get("hands").is_none(),
        "nothing boxed, nothing pinned"
    );
}

/// Ruling 2's refusals: a malformed spec names the site; hands beside
/// secret bindings are refused together; a seat that names an agent
/// cannot amend the agent's hands.
#[test]
fn hands_are_refused_where_they_cannot_be_honest() {
    let fixture = Fixture::new();
    fixture.write_adapter(adapter("exec", Some("untrusted"), Some(true)));

    let refusal = fixture.refusal(exec_gate(Some(json!({"kind": "mitten"})), None));
    assert!(refusal.contains("seat 'work' hands"), "{refusal}");
    assert!(refusal.contains("kind must be"), "{refusal}");

    let refusal = fixture.refusal(exec_gate(Some(json!("workspace")), Some(json!(["TOKEN"]))));
    assert!(refusal.contains("hands and secret bindings"), "{refusal}");
    assert!(refusal.contains("clears the environment"), "{refusal}");

    fixture.write_agent("boxed", &[("fable", "claude", "trusted")]);
    let refusal = fixture.refusal(json!({
        "agent": "boxed",
        "results": ["pass", "fail"],
        "hands": "workspace",
    }));
    assert!(refusal.contains("hands"), "{refusal}");
    assert!(refusal.contains("agent"), "{refusal}");
}

/// The manifest's `hands` keys are the sites as the engine labels its
/// driver seats — `seat:member` for a panel member, `seat:step` for a
/// sequence step — so the spawn-side lookup finds what compile recorded.
#[test]
fn hands_are_recorded_under_the_sites_the_engine_labels() {
    let fixture = Fixture::new();
    fixture.write_adapter(adapter("exec", Some("untrusted"), Some(true)));
    let exec = |name: &str| {
        json!({
            "name": name,
            "results": ["pass", "fail"],
            "role": "roles/role.md",
            "class": "gate",
            "hands": "workspace",
            "driver": {"command": ["{brokkr}", "driver", "exec", "--", "bash", "check.sh", "{prompt_file}"]},
        })
    };
    let panel = json!({
        "results": ["pass", "fail"],
        "panel": {
            "left": {
                "role": "roles/role.md",
                "class": "gate",
                "hands": {"kind": "workspace", "network": true},
                "driver": {"command": ["{brokkr}", "driver", "exec", "--", "bash", "check.sh", "{prompt_file}"]},
            },
            "right": {
                "role": "roles/role.md",
                "class": "gate",
                "hands": "workspace",
                "driver": {"command": ["{brokkr}", "driver", "exec", "--", "bash", "check.sh", "{prompt_file}"]},
            }
        },
        "aggregate": "unanimous-pass",
    });
    let bundle = fixture.compile(panel).unwrap();
    assert_eq!(
        bundle.hands.keys().collect::<Vec<_>>(),
        ["work:left", "work:right"],
        "boxed members are keyed as the engine labels them"
    );
    assert!(bundle.hands["work:left"].network);
    assert!(!bundle.hands["work:right"].network);

    let mut second = exec("second");
    second.as_object_mut().unwrap().remove("results");
    let sequence = json!({
        "results": ["pass", "fail"],
        "sequence": [exec("first"), second],
    });
    let bundle = fixture.compile(sequence).unwrap();
    assert_eq!(
        bundle.hands.keys().collect::<Vec<_>>(),
        ["work:first", "work:second"]
    );
    assert_eq!(bundle.manifest["hands"]["work:second"]["kind"], "workspace");
}

// ------------------------------------ decision 0046: the boundary's law

/// The workspace root the shipped adapters live under.
fn workspace() -> PathBuf {
    shipped_adapters().parent().unwrap().to_path_buf()
}

/// The `hands.harness` shapes a fixture provider declares (decision 0046
/// ruling 4): both members as fragments with the capture door, the gate
/// alone, and a measured gap on each.
fn both_members() -> Value {
    json!({
        "gate": ["--judge-only", "--door", "{result_path}"],
        "work": ["--writable"],
        "result": "last-message",
    })
}

fn gate_only() -> Value {
    json!({"gate": ["--judge-only", "--door", "{result_path}"]})
}

fn measured_gaps() -> Value {
    json!({
        "gate": {"unsupported": "the read-only mode leaves no door"},
        "work": {"unsupported": "no writable class was found"},
    })
}

/// A seat that hires a boxed agent in the given class.
fn boxed_seat(agent: &str, class: &str) -> Value {
    json!({"results": ["pass", "fail"], "class": class, "agent": agent})
}

/// An exec gate with hands whose raw dispatch is `{brokkr} driver exec --`
/// followed by `tail` — the command the pinned-script grammar reads.
fn exec_dispatch(tail: &[&str]) -> Value {
    let mut command = vec!["{brokkr}", "driver", "exec", "--"];
    command.extend_from_slice(tail);
    let mut site = exec_gate(Some(json!("workspace")), None);
    site["driver"]["command"] = json!(command);
    site
}

/// Decision 0046 ruling 4's grammar and lookup (design DD9), row by row.
/// Under `harness` and `open` an exec site with hands is admitted only
/// for the bundle's own pinned `./` script, judged on the raw command
/// against the declaring layer; every other spelling is refused naming
/// the token, ruling 4 and 0021, and no path is compared to judge it.
/// Under `namespace` every row compiles: a boxed gate is admitted by its
/// walls, an unboxed one by its bytes (decision 0043 ruling 3).
#[test]
fn an_unboxed_exec_site_with_hands_holds_only_for_the_bundles_pinned_script() {
    let fixture = Fixture::new();
    fixture.write_adapter(adapter("exec", Some("untrusted"), Some(true)));
    fixture.script("scripts/x.sh");
    fixture.script("scripts/s.sh");
    fixture.script("dialects/run.sh");
    let layer = fixture.bundle_dir().canonicalize().unwrap();
    let layer = layer.display().to_string();

    for boundary in [Boundary::Harness, Boundary::Open] {
        let bundle = fixture
            .compile_bounded(
                exec_dispatch(&["bash", "./scripts/x.sh", "{prompt_file}"]),
                boundary,
            )
            .unwrap_or_else(|e| panic!("{boundary}: {e}"));
        assert_eq!(bundle.boundary, boundary);
        assert_eq!(
            bundle.manifest["boundary"],
            json!({"work": boundary.word()})
        );
        assert!(
            bundle.manifest["files"].get("scripts/x.sh").is_some(),
            "the walk pins the script the law admitted"
        );
    }

    // (raw tail, what the refusal names, whether it names the layer)
    let rows: [(&[&str], &str, bool); 8] = [
        (
            &["true"],
            "the command names no `./`-relative script after its interpreters (true)",
            false,
        ),
        (
            &["bash", "./../outside.sh"],
            "'./../outside.sh' is not a plain `./`-relative path",
            false,
        ),
        (
            &["bash", "/usr/bin/true"],
            "'/usr/bin/true' is spelled as a path that is not `./`-relative to the bundle",
            false,
        ),
        (
            &["bash", ".\\scripts\\s.sh"],
            "'.\\scripts\\s.sh' is spelled as a path that is not `./`-relative to the bundle",
            false,
        ),
        (
            &["bash", "/private/var/b/scripts/s.sh"],
            "'/private/var/b/scripts/s.sh' is spelled as a path that is not `./`-relative",
            false,
        ),
        (
            &["bash", "./scripts/missing.sh"],
            "'./scripts/missing.sh' names no regular file under the declaring layer",
            true,
        ),
        (
            &["bash", "-c", "./scripts/s.sh"],
            "'-c' is an option token before the script",
            false,
        ),
        (
            &["bash", "./dialects/run.sh"],
            "'./dialects/run.sh' names a path the manifest walk does not pin (dialects is \
             workspace data",
            false,
        ),
    ];
    for (tail, names, names_layer) in rows {
        for boundary in [Boundary::Harness, Boundary::Open] {
            let refusal = fixture.refusal_under(exec_dispatch(tail), boundary);
            assert!(
                refusal.contains(&format!(
                    "seat 'work' declares hands under the `{boundary}` boundary, where no box \
                     stands, so its exec command may run only the bundle's own pinned script: "
                )),
                "{tail:?}: {refusal}"
            );
            assert!(refusal.contains(names), "{tail:?}: {refusal}");
            assert!(
                refusal.contains("(decision 0046 ruling 4; decision 0021)"),
                "{tail:?}: {refusal}"
            );
            // A spelling is refused on the token alone: only the lookup
            // of a `./` token that names nothing says where it looked.
            assert_eq!(
                refusal.contains(&layer),
                names_layer,
                "{tail:?} names the layer: {refusal}"
            );
        }
        fixture
            .compile_bounded(exec_dispatch(tail), Boundary::Namespace)
            .unwrap_or_else(|e| panic!("{tail:?} is admitted by the box's walls: {e}"));
    }
    // The dialects file the walk skips does exist, so the refusal above
    // is the walk's exclusion and not the lookup's.
    assert!(fixture.bundle_dir().join("dialects/run.sh").is_file());
}

/// The shipped exec gates on the pinned-script terms: `recipes/fast`'s
/// verify and ship seats compile under `harness` and `open` — the
/// `{brokkr}` among the ship seat's arguments names no command — and
/// `recipes/wager-harness`, which inherits both from `fast`, is judged
/// against the layer that wrote them. `bundles/self` carries the same
/// two seats but chains claude at its review gate, so under `open` it
/// refuses there, before either exec seat is reached (seats compile in
/// name order).
#[test]
fn the_shipped_exec_gates_stand_unboxed_on_their_own_pinned_scripts() {
    let root = workspace();
    let self_dir = root.join("bundles/self");
    let self_config: Value =
        serde_json::from_slice(&std::fs::read(self_dir.join("bundle.json")).unwrap()).unwrap();
    for boundary in [Boundary::Harness, Boundary::Open] {
        enforce_hands_boundary(
            "verify",
            &self_config["seats"]["verify"],
            &[],
            SiteLaw {
                boundary,
                dir: &self_dir,
                agent_hands: None,
            },
            None,
        )
        .expect("self's verifier is its own pinned script");
        let fast = Bundle::compile_under(
            &root.join("recipes/fast"),
            &root.join("agents"),
            &root.join("adapters"),
            boundary,
        )
        .unwrap_or_else(|e| panic!("recipes/fast under {boundary}: {e}"));
        assert_eq!(
            fast.manifest["boundary"],
            json!({"ship": boundary.word(), "verify": boundary.word()})
        );
        let SeatBody::Single { command, .. } = &fast.seats["ship"].body else {
            panic!("the ship seat is a single exec site")
        };
        // `{brokkr}` expanded twice: once as the dispatch, once as the
        // script's own argument, which nobody judged.
        assert_eq!(command.len(), 8, "{command:?}");
        assert_eq!(command[7], command[0], "{command:?}");
        assert_eq!(command[6], "{prompt_file}");
    }

    let wager = Bundle::compile_under(
        &root.join("recipes/wager-harness"),
        &root.join("agents"),
        &root.join("adapters"),
        Boundary::Harness,
    )
    .expect("the inherited verify seat resolves against fast");
    let fast_dir = root.join("recipes/fast").canonicalize().unwrap();
    assert_eq!(wager.roots[1], fast_dir);
    let SeatBody::Single { command, .. } = &wager.seats["verify"].body else {
        panic!("the inherited verify seat is a single exec site")
    };
    let checked = fast_dir.join("scripts/verify-seat.sh");
    assert!(checked.is_file(), "{}", checked.display());
    assert_eq!(command[5], checked.to_string_lossy());
    assert_eq!(wager.manifest["boundary"]["verify"], "harness");

    let refusal = Bundle::compile_under(
        &root.join("bundles/self"),
        &root.join("agents"),
        &root.join("adapters"),
        Boundary::Open,
    )
    .expect_err("bundles/self seats a model gate with hands")
    .to_string();
    assert!(
        refusal.contains("seat 'review' is a gate with hands under the `open` boundary"),
        "{refusal}"
    );
    assert!(!refusal.contains("seat 'verify'"), "{refusal}");
}

#[test]
fn pinned_script_components_reject_ambiguity_and_directories() {
    let fixture = Fixture::new();
    fixture.script("scripts/check.sh");
    let no_separator = ["engine", "driver", "exec", "./scripts/check.sh"]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    assert!(pinned_script(&fixture.bundle_dir(), &no_separator)
        .unwrap_err()
        .contains("no `--` after `exec`"));
    for script in [
        "./scripts/./check.sh",
        "./scripts/../check.sh",
        "./scripts//check.sh",
        "./scripts",
    ] {
        let command: Vec<String> = ["engine", "driver", "exec", "--", "sh", script]
            .into_iter()
            .map(str::to_string)
            .collect();
        assert!(
            pinned_script(&fixture.bundle_dir(), &command).is_err(),
            "{script}"
        );
    }
}

#[test]
fn hands_on_an_unmapped_agent_chain_report_the_resolver_gap() {
    let fixture = Fixture::new();
    fixture.write_agent_file("lost", &["unmapped"], None, true);
    let error = fixture.refusal_under(
        json!({"agent":"lost", "results":["pass","fail"]}),
        Boundary::Namespace,
    );
    assert!(error.contains("unmapped"), "{error}");
}

/// Proposal D32, design DD22: the class decides only whether a site may
/// hold a gate. A `class: work` exec site with hands naming the bundle's
/// pinned `./scripts/lint.sh` is admitted under `open`; its sibling
/// running `true` is refused naming ruling 4 and 0021, the class changing
/// nothing; and the same bundle — naming no agent, seating no gate and
/// binding no secret — compiles with no `adapters/` directory reachable,
/// admitted and refused the same, because the law reads the raw command
/// and the declaring layer and never an adapter.
#[test]
fn a_work_class_exec_site_with_hands_is_judged_on_the_gates_ground_and_reads_no_adapter() {
    let fixture = Fixture::new();
    fixture.script("scripts/lint.sh");
    let site = |name: &str, class: &str, tail: &[&str]| {
        let mut command = vec!["{brokkr}", "driver", "exec", "--"];
        command.extend_from_slice(tail);
        json!({
            "name": name,
            "results": ["pass"],
            "class": class,
            "hands": "workspace",
            "role": "roles/role.md",
            "driver": {"command": command},
        })
    };
    let lint = |class: &str| {
        let mut alone = site("lint", class, &["bash", "./scripts/lint.sh"]);
        alone.as_object_mut().unwrap().remove("name");
        alone["results"] = json!(["pass", "fail"]);
        alone
    };
    let pair = |class: &str| {
        let mut second = site("plain", class, &["true"]);
        second.as_object_mut().unwrap().remove("results");
        json!({
            "results": ["pass", "fail"],
            "sequence": [site("lint", class, &["bash", "./scripts/lint.sh"]), second],
        })
    };
    let nowhere = PathBuf::from("/nonexistent-adapters");
    for adapters in [fixture.adapters(), nowhere.clone()] {
        let bundle = fixture
            .compile_roots(lint("work"), &fixture.agents(), &adapters, Boundary::Open)
            .unwrap_or_else(|e| panic!("{}: {e}", adapters.display()));
        assert_eq!(bundle.manifest["boundary"], json!({"work": "open"}));
        assert!(
            bundle.manifest.get("drivers").is_none(),
            "no adapter was read"
        );
        let refusal = fixture
            .compile_roots(pair("work"), &fixture.agents(), &adapters, Boundary::Open)
            .expect_err("the sibling runs a bare program")
            .to_string();
        assert!(
            refusal.contains("seat 'work:plain' declares hands under the `open` boundary"),
            "{refusal}"
        );
        assert!(
            refusal.contains(
                "the command names no `./`-relative script after its interpreters (true)"
            ),
            "{refusal}"
        );
        assert!(
            refusal.contains("(decision 0046 ruling 4; decision 0021)"),
            "{refusal}"
        );
        assert!(
            !refusal.contains("work:lint"),
            "the first site was admitted: {refusal}"
        );
    }
    // The gate-class pair reads the same, word for word: the class is
    // not consulted on the exec arm. A gate opens the adapters, so this
    // half runs against the fixture's.
    fixture.write_adapter(adapter("exec", Some("untrusted"), Some(true)));
    fixture
        .compile_bounded(lint("gate"), Boundary::Open)
        .expect("a gate on the pinned script is admitted too");
    let work = fixture.refusal_under(pair("work"), Boundary::Open);
    let gate = fixture.refusal_under(pair("gate"), Boundary::Open);
    assert_eq!(work, gate);
    // And a bundle whose only hands site is a work-class exec seat never
    // opened the adapters: the `expect` behind the class read is not
    // reached, because the law returned before it.
    fixture
        .compile_roots(lint("work"), &fixture.agents(), &nowhere, Boundary::Harness)
        .expect("admitted under harness on the same ground");
}

/// Design DD8: a dialect validate or check step holds its gate boxed
/// (decision 0042 ruling 4), and under `harness` or `open` it is refused
/// at compile naming the step, ruling 4, 0042 ruling 4 and a boxed
/// boundary as the road — before the gate law runs, so the chief seated
/// before it on a provider that declares both `hands.harness` members
/// is not what refuses. Under `namespace` the same bundle compiles as
/// today, its synthetic step boxed.
#[test]
fn a_dialect_step_under_an_unboxed_boundary_is_refused_until_a_decision_admits_it() {
    let fixture = Fixture::new();
    fixture.write_boxed_agent("chief", &[("m-chief", "council", Some(both_members()))]);
    let dialect = Dialect::load(&workspace().join("dialects/openspec.json"))
        .unwrap()
        .0;
    let policy = json!({
        "schema": "forge.phase-machine/v1",
        "phases": ["design", "review", "done"],
        "initial": "design",
        "terminal": ["done"],
        "shippable_from": ["review"],
        "rules": [
            {"id": "D", "from": "design", "result": "drafted", "next": "review", "reason": "drafted"},
            {"id": "DF", "from": "design", "result": "fail", "next": "design", "reason": "retry"},
            {"id": "R", "from": "review", "result": "clean", "next": "done", "reason": "clean"}
        ]
    });
    let bundle = fixture.bundle_dir();
    std::fs::create_dir_all(bundle.join("roles")).unwrap();
    std::fs::write(bundle.join("roles/role.md"), "# role\n").unwrap();
    std::fs::write(bundle.join("policy.json"), policy.to_string()).unwrap();
    std::fs::write(
        bundle.join("bundle.json"),
        json!({
            "name": "artifact",
            "policy": "policy.json",
            "seats": {
                "design": {"results": ["drafted", "fail"], "sequence": [
                    {"name": "chief", "results": ["drafted"], "agent": "chief", "class": "work"},
                    {"name": "validate", "dialect": "validate"}
                ]},
                "review": {
                    "role": "roles/role.md",
                    "results": ["clean"],
                    "driver": {"command": [
                        "{brokkr}", "driver", "judge", "--", "--model", "judge-1", "true"
                    ]},
                },
            }
        })
        .to_string(),
    )
    .unwrap();
    let compile = |boundary: Boundary| {
        Bundle::compile_with_realm(
            &bundle,
            &fixture.agents(),
            &fixture.adapters(),
            None,
            Some(&dialect),
            boundary,
        )
    };
    for boundary in [Boundary::Harness, Boundary::Open] {
        let refusal = compile(boundary)
            .expect_err("the dialect step is not the bundle's pinned script")
            .to_string();
        assert!(
            refusal.contains(
                "dialect step 'design:validate' holds its gate boxed (decision 0042 ruling 4)"
            ),
            "{refusal}"
        );
        assert!(
            refusal.contains(&format!("under the `{boundary}` boundary no box stands")),
            "{refusal}"
        );
        assert!(
            refusal.contains("which is all decision 0046 ruling 4 admits at an unboxed gate"),
            "{refusal}"
        );
        assert!(
            refusal.contains("Run the realm under a boxed boundary (namespace)"),
            "{refusal}"
        );
        assert!(!refusal.contains("design:chief"), "{refusal}");
    }
    let boxed = compile(Boundary::Namespace).expect("the synthetic gate is boxed today");
    assert_eq!(
        boxed.hands.keys().collect::<Vec<_>>(),
        ["design:chief", "design:validate"]
    );
    assert_eq!(
        boxed.manifest["boundary"],
        json!({"design:chief": "namespace", "design:validate": "namespace"})
    );
}

/// The gate law under the boundary axis (decision 0046 ruling 4; design
/// DD7 and DD22), arm by arm on fixture providers: under `harness` a
/// model gate is admitted only when every link declares
/// `hands.harness.gate`, refused otherwise naming the link, the provider
/// and the member — with the measured reason when the operator recorded
/// one; under `open` a model gate is refused whatever the adapter says
/// and a work seat runs at the harness's default; under `seatbelt` and
/// `container` a gate compiles exactly as under `namespace` with the word
/// pinned; and a work seat under `harness` without a `work` fragment is
/// a capability gap.
#[test]
fn the_gate_law_reads_the_boundary_for_sites_that_declare_hands() {
    let fixture = Fixture::new();
    fixture.write_boxed_agent(
        "declared",
        &[("m-declared", "declared-provider", Some(gate_only()))],
    );
    fixture.write_boxed_agent("both", &[("m-both", "both-provider", Some(both_members()))]);
    fixture.write_boxed_agent("silent-box", &[("m-silent", "silent-provider", None)]);
    fixture.write_boxed_agent(
        "falls-back",
        &[
            ("m-first", "first-provider", Some(both_members())),
            ("m-second", "second-provider", None),
        ],
    );
    fixture.write_boxed_agent(
        "measured",
        &[("m-measured", "measured-provider", Some(measured_gaps()))],
    );

    // A harness gate on a provider that declares the fragment.
    for agent in ["declared", "both"] {
        let bundle = fixture
            .compile_bounded(boxed_seat(agent, "gate"), Boundary::Harness)
            .unwrap_or_else(|e| panic!("{agent}: {e}"));
        assert_eq!(bundle.boundary, Boundary::Harness);
        assert_eq!(bundle.manifest["boundary"], json!({"work": "harness"}));
    }
    // One that declares none.
    let refusal = fixture.refusal_under(boxed_seat("silent-box", "gate"), Boundary::Harness);
    assert!(
        refusal.contains(
            "seat 'work' gate link 1 resolves to provider 'silent-provider', which declares no \
             `hands.harness.gate` fragment; under the `harness` boundary a model may judge only \
             under its harness's own read-only sandbox as the adapter addresses it (decision \
             0046 ruling 4)"
        ),
        "{refusal}"
    );
    // A fallback link without the fragment refuses the chain, by link.
    let refusal = fixture.refusal_under(boxed_seat("falls-back", "gate"), Boundary::Harness);
    assert!(
        refusal.contains("seat 'work' gate link 2 resolves to provider 'second-provider'"),
        "{refusal}"
    );
    assert!(!refusal.contains("first-provider"), "{refusal}");
    // A measured gap names its reason (task 8.11's compile half).
    let refusal = fixture.refusal_under(boxed_seat("measured", "gate"), Boundary::Harness);
    assert!(
        refusal.contains(
            "provider 'measured-provider', which declares no `hands.harness.gate` fragment (the \
             read-only mode leaves no door)"
        ),
        "{refusal}"
    );
    // An open model gate is refused whatever the adapter declares.
    for agent in ["declared", "both", "silent-box"] {
        let refusal = fixture.refusal_under(boxed_seat(agent, "gate"), Boundary::Open);
        assert!(
            refusal.contains(
                "seat 'work' is a gate with hands under the `open` boundary, where nothing at \
                 all stands between a model's hands and the machine; `open` never holds a model \
                 gate (decision 0046 ruling 4)"
            ),
            "{agent}: {refusal}"
        );
    }
    // The law's last arm: an open work-class chain site is admitted with
    // no link declaring a fragment.
    let bundle = fixture
        .compile_bounded(boxed_seat("silent-box", "work"), Boundary::Open)
        .expect("a work seat under open runs at the harness's default");
    assert_eq!(bundle.manifest["boundary"], json!({"work": "open"}));
    // A harness work seat takes the work fragment, or is a capability gap.
    fixture
        .compile_bounded(boxed_seat("both", "work"), Boundary::Harness)
        .expect("a work seat on a provider with a work fragment");
    let refusal = fixture.refusal_under(boxed_seat("declared", "work"), Boundary::Harness);
    assert!(
        refusal.contains(
            "seat 'work' link 1 resolves to provider 'declared-provider', which declares no \
             `hands.harness.work` fragment: a capability gap — under the `harness` boundary a \
             work seat with hands writes the tree only under the harness's own writable sandbox \
             as the adapter addresses it (decision 0046 rulings 1 and 4)"
        ),
        "{refusal}"
    );
    let refusal = fixture.refusal_under(boxed_seat("measured", "work"), Boundary::Harness);
    assert!(
        refusal.contains(
            "`hands.harness.work` fragment (no writable class was found): a capability gap"
        ),
        "{refusal}"
    );
    // A seatbelt and a container gate are admitted at compile exactly as
    // under namespace, the word pinned, whatever the machine holds.
    let namespace = fixture
        .compile_bounded(boxed_seat("silent-box", "gate"), Boundary::Namespace)
        .unwrap();
    assert_eq!(namespace.manifest["boundary"], json!({"work": "namespace"}));
    for boundary in [Boundary::Seatbelt, Boundary::Container] {
        let bundle = fixture
            .compile_bounded(boxed_seat("silent-box", "gate"), boundary)
            .unwrap_or_else(|e| panic!("{boundary}: {e}"));
        assert_eq!(
            bundle.manifest["boundary"],
            json!({"work": boundary.word()})
        );
        let mut theirs = bundle.manifest.clone();
        let mut ours = namespace.manifest.clone();
        theirs.as_object_mut().unwrap().remove("boundary");
        ours.as_object_mut().unwrap().remove("boundary");
        assert_eq!(
            theirs, ours,
            "{boundary} differs from namespace only in the word"
        );
        assert_ne!(bundle.manifest_digest(), namespace.manifest_digest());
    }
}

/// A gate-class site without hands has no box whose boundary could be
/// named: an inline trusted model gate with a tool list compiles under
/// `open` and `harness` exactly as under `namespace`, byte for byte. An
/// inline model site WITH hands is refused under both naming the seat
/// and the repair — its argv is the author's and carries the box's own
/// tokens — and compiles under `namespace`; a bare program with hands is
/// refused by the grammar's own arm.
#[test]
fn a_gate_without_hands_is_untouched_and_an_inline_model_site_with_hands_is_refused_unboxed() {
    let fixture = Fixture::new();
    let mut tooled = seat("judge", Some("gate"), None);
    tooled["driver"]["command"]
        .as_array_mut()
        .unwrap()
        .extend([json!("--allowedTools"), json!("Bash(cargo:*)")]);
    let namespace = fixture
        .compile_bounded(tooled.clone(), Boundary::Namespace)
        .unwrap();
    assert!(namespace.manifest.get("boundary").is_none());
    for boundary in [Boundary::Open, Boundary::Harness] {
        let bundle = fixture
            .compile_bounded(tooled.clone(), boundary)
            .unwrap_or_else(|e| panic!("{boundary}: {e}"));
        assert_eq!(bundle.boundary, boundary);
        assert_eq!(bundle.manifest, namespace.manifest);
    }

    let mut boxed = seat("judge", Some("work"), None);
    boxed["hands"] = json!("workspace");
    fixture
        .compile_bounded(boxed.clone(), Boundary::Namespace)
        .expect("a boxed inline model site is decision 0043's own case");
    for boundary in [Boundary::Harness, Boundary::Open] {
        let refusal = fixture.refusal_under(boxed.clone(), boundary);
        assert!(
            refusal.contains(&format!(
                "seat 'work' is an inline `judge` site that declares hands under the \
                 `{boundary}` boundary; its argv is the author's own and carries the box's \
                 tokens, which no harness stands behind unboxed. Seat it through an agent, \
                 whose adapter declares how the harness stands under `harness`, or run the \
                 realm under a boxed boundary (decision 0046 ruling 4)"
            )),
            "{refusal}"
        );
    }

    let mut bare = seat("judge", Some("work"), None);
    bare["driver"]["command"] = json!(["custom-driver", "--flag"]);
    bare["hands"] = json!("workspace");
    let refusal = fixture.refusal_under(bare, Boundary::Harness);
    assert!(
        refusal.contains(
            "seat 'work' declares hands under the `harness` boundary, where no box stands, and \
             its command is a bare program rather than a `{brokkr} driver exec` dispatch of the \
             bundle's own pinned script (decision 0046 ruling 4; decision 0021)"
        ),
        "{refusal}"
    );
}

/// Ruling 4's own binding, pinned against the shipped adapter library
/// (`adapters/` as it stands) and not a fixture shaped like it (design
/// DD20): a fixture gate agent with hands chaining `astra` alone is
/// admitted under `harness`, its manifest pinning the shipped codex
/// adapter's digest.
///
/// D33: mapped hands chains face the boundary law before the resolver's
/// workspace capability gap: namespace refuses the untrusted tier;
/// harness refuses the absent gate fragment, naming the provider and link.
#[test]
fn ruling_4s_own_binding_is_pinned_against_the_shipped_adapters() {
    let fixture = Fixture::new();
    fixture.write_agent_file(
        "astra-judge",
        &["astra"],
        Some(json!({"astra": "high"})),
        true,
    );
    let bundle = fixture
        .compile_roots(
            boxed_seat("astra-judge", "gate"),
            &fixture.agents(),
            &shipped_adapters(),
            Boundary::Harness,
        )
        .expect("a harness gate on the shipped codex adapter is admitted");
    let shipped = Adapters::load(&shipped_adapters()).expect("the shipped adapters load");
    let codex = shipped.digest("codex").expect("the shipped codex adapter");
    assert_eq!(bundle.manifest["agents"]["work"]["provider"], "codex");
    assert_eq!(
        bundle.manifest["agents"]["work"]["adapter_digest"],
        brokkr_core::canonical::sha256_hex(&json!({"codex": codex}))
    );
    assert_eq!(bundle.manifest["boundary"], json!({"work": "harness"}));
    let SeatBody::Single { candidates, .. } = &bundle.seats["work"].body else {
        panic!("an agent seat is a single body")
    };
    // The candidate carries the shipped declaration to the engine, which
    // holds no adapter at spawn.
    assert_eq!(
        candidates[0].harness,
        shipped.adapter("codex").unwrap().harness
    );
    assert_eq!(
        candidates[0].harness.gate,
        Some(
            [
                "--sandbox",
                "read-only",
                "--output-last-message",
                "{result_path}"
            ]
            .map(String::from)
            .to_vec()
        )
    );

    fixture.write_agent_file(
        "flash-judge",
        &["flash"],
        Some(json!({"flash": "medium"})),
        true,
    );
    fixture.write_agent_file(
        "tallied-judge",
        &["fable-tallied"],
        Some(json!({"fable-tallied": "high"})),
        true,
    );
    for (agent, provider) in [("flash-judge", "dsh"), ("tallied-judge", "lanetally")] {
        for boundary in [Boundary::Namespace, Boundary::Harness] {
            let refusal = fixture
                .compile_roots(
                    boxed_seat(agent, "gate"),
                    &fixture.agents(),
                    &shipped_adapters(),
                    boundary,
                )
                .map(|_| ())
                .expect_err("a hands agent on a provider without hands is refused")
                .to_string();
            assert!(refusal.contains("seat 'work'"), "{refusal}");
            assert!(
                refusal.contains(&format!("provider '{provider}'"))
                    || refusal.contains(&format!("driver '{provider}'")),
                "{refusal}"
            );
            match boundary {
                Boundary::Namespace => {
                    assert!(refusal.contains("trusted tier"), "{refusal}");
                    assert!(refusal.contains("decision 0021 ruling 2"), "{refusal}");
                }
                _ => {
                    assert!(refusal.contains("gate link 1"), "{refusal}");
                    assert!(refusal.contains("hands.harness.gate"), "{refusal}");
                    assert!(refusal.contains("decision 0046 ruling 4"), "{refusal}");
                }
            }
        }
        // An open worker needs no workspace fragment from either adapter.
        fixture
            .compile_roots(
                boxed_seat(agent, "work"),
                &fixture.agents(),
                &shipped_adapters(),
                Boundary::Open,
            )
            .unwrap();
    }

    // Without hands the law returns at once and decision 0021 ruling 2's
    // trust-tier refusal is what stands, under both boundaries.
    fixture.write_agent_file(
        "flash-bare",
        &["flash"],
        Some(json!({"flash": "medium"})),
        false,
    );
    fixture.write_agent_file(
        "tallied-bare",
        &["fable-tallied"],
        Some(json!({"fable-tallied": "high"})),
        false,
    );
    for (agent, provider) in [("flash-bare", "dsh"), ("tallied-bare", "lanetally")] {
        for boundary in [Boundary::Namespace, Boundary::Harness] {
            let refusal = fixture
                .compile_roots(
                    boxed_seat(agent, "gate"),
                    &fixture.agents(),
                    &shipped_adapters(),
                    boundary,
                )
                .map(|_| ())
                .expect_err("an untrusted provider may not hold a gate")
                .to_string();
            assert!(
                refusal.contains(&format!(
                    "seat 'work' is gate class but seats driver '{provider}', which does not \
                     hold the trusted tier"
                )),
                "{boundary}: {refusal}"
            );
            assert!(
                refusal.contains("decision 0021 ruling 2"),
                "{boundary}: {refusal}"
            );
        }
    }
}

// ------------------------------- decision 0046: the manifest's boundary map

/// A two-step sequence of exec gates with hands, each on the bundle's own
/// pinned `./scripts/check.sh`, so it compiles under every boundary.
fn two_boxed_sites() -> Value {
    let step = |name: &str| {
        json!({
            "name": name,
            "results": ["pass", "fail"],
            "role": "roles/role.md",
            "class": "gate",
            "hands": "workspace",
            "driver": {"command": ["{brokkr}", "driver", "exec", "--", "bash", "./scripts/check.sh", "{prompt_file}"]},
        })
    };
    let mut second = step("second");
    second.as_object_mut().unwrap().remove("results");
    json!({
        "results": ["pass", "fail"],
        "sequence": [step("first"), second],
    })
}

fn v9() -> jsonschema::Validator {
    let schema: Value = serde_json::from_slice(
        &std::fs::read(workspace().join("contracts/run-manifest.v9.schema.json")).unwrap(),
    )
    .unwrap();
    jsonschema::draft7::new(&schema).unwrap()
}

/// run-manifest/v9 (decision 0046 ruling 1; design DD4): `boundary` is
/// written beside `hands` with the same keys, every value the realm's
/// word, and validates; a bundle that boxes nothing carries neither key
/// and keeps one identity under every boundary; the same boxed bundle
/// under `namespace` and `harness` differs only in `boundary` and the
/// digests differ; a half-pinned manifest fails the contract.
#[test]
fn the_manifest_pins_the_boundary_per_hands_site_as_run_manifest_v9() {
    let fixture = Fixture::new();
    fixture.write_adapter(adapter("exec", Some("untrusted"), Some(true)));
    fixture.script("scripts/check.sh");
    let validator = v9();

    let namespace = fixture
        .compile_bounded(two_boxed_sites(), Boundary::Namespace)
        .unwrap();
    let hands = namespace.manifest["hands"].as_object().unwrap();
    let boundary = namespace.manifest["boundary"].as_object().unwrap();
    assert_eq!(
        hands.keys().collect::<Vec<_>>(),
        ["work:first", "work:second"]
    );
    assert_eq!(
        hands.keys().collect::<Vec<_>>(),
        boundary.keys().collect::<Vec<_>>()
    );
    assert!(boundary.values().all(|word| word == "namespace"));
    assert!(
        validator.is_valid(&namespace.manifest),
        "{}",
        namespace.manifest
    );

    let harness = fixture
        .compile_bounded(two_boxed_sites(), Boundary::Harness)
        .unwrap();
    assert!(validator.is_valid(&harness.manifest));
    assert_eq!(
        harness.manifest["boundary"],
        json!({"work:first": "harness", "work:second": "harness"})
    );
    let mut theirs = harness.manifest.clone();
    let mut ours = namespace.manifest.clone();
    theirs.as_object_mut().unwrap().remove("boundary");
    ours.as_object_mut().unwrap().remove("boundary");
    assert_eq!(theirs, ours, "the two manifests differ only in `boundary`");
    assert_ne!(harness.manifest_digest(), namespace.manifest_digest());

    // A plain bundle: neither key, one identity under every word.
    let plain = fixture
        .compile_bounded(seat("judge", Some("gate"), None), Boundary::Namespace)
        .unwrap();
    assert!(plain.manifest.get("hands").is_none());
    assert!(plain.manifest.get("boundary").is_none());
    assert!(validator.is_valid(&plain.manifest));
    for boundary in [
        Boundary::Seatbelt,
        Boundary::Container,
        Boundary::Harness,
        Boundary::Open,
    ] {
        let again = fixture
            .compile_bounded(seat("judge", Some("gate"), None), boundary)
            .unwrap_or_else(|e| panic!("{boundary}: {e}"));
        assert_eq!(
            again.manifest_digest(),
            plain.manifest_digest(),
            "{boundary}"
        );
    }

    // Half-pinned manifests fail v9: hands without boundary, boundary
    // without hands, and a word outside the enum.
    let mut no_boundary = namespace.manifest.clone();
    no_boundary.as_object_mut().unwrap().remove("boundary");
    assert!(!validator.is_valid(&no_boundary));
    let mut no_hands = namespace.manifest.clone();
    no_hands.as_object_mut().unwrap().remove("hands");
    assert!(!validator.is_valid(&no_hands));
    let mut sixth = namespace.manifest.clone();
    sixth["boundary"]["work:first"] = json!("chroot");
    assert!(!validator.is_valid(&sixth));
}

// --------------------------- decision 0046: the re-walk over a leaf layer

/// The spawn-time re-walk (design DD9) as a pure function over a
/// temporary leaf layer: an untouched layer names no key; an edited
/// script, an edited sibling, a deleted pinned file and an added file
/// each name the first key that differs, with the layer's name. The
/// ancestor arm is pinned in `compose_tests.rs`, which owns a recipe
/// library.
#[test]
fn the_re_walk_of_a_leaf_layer_names_the_first_pinned_key_that_moved() {
    let fixture = Fixture::new();
    fixture.script("scripts/x.sh");
    fixture.script("scripts/lib.sh");
    let bundle = fixture
        .compile_bounded(
            exec_dispatch(&["bash", "./scripts/x.sh", "{prompt_file}"]),
            Boundary::Harness,
        )
        .unwrap();
    let layer = bundle.dir.clone();
    assert_eq!(bundle.roots, vec![layer.clone()]);
    assert_eq!(layer_drift(&bundle, &layer), None);

    let script = layer.join("scripts/x.sh");
    let pinned = std::fs::read(&script).unwrap();
    std::fs::write(&script, "#!/bin/sh\nrm -rf /\n").unwrap();
    assert_eq!(
        layer_drift(&bundle, &layer),
        Some((
            "model-policy".to_string(),
            "changed: scripts/x.sh".to_string()
        ))
    );
    std::fs::write(&script, &pinned).unwrap();
    assert_eq!(
        layer_drift(&bundle, &layer),
        None,
        "restored bytes are the pinned bytes"
    );

    let sibling = layer.join("scripts/lib.sh");
    std::fs::write(&sibling, "#!/bin/sh\nexport PATH=/tmp:$PATH\n").unwrap();
    assert_eq!(
        layer_drift(&bundle, &layer),
        Some((
            "model-policy".to_string(),
            "changed: scripts/lib.sh".to_string()
        ))
    );

    std::fs::remove_file(&sibling).unwrap();
    assert_eq!(
        layer_drift(&bundle, &layer),
        Some((
            "model-policy".to_string(),
            "missing: scripts/lib.sh".to_string()
        ))
    );
    std::fs::write(&sibling, &pinned).unwrap();
    assert_eq!(layer_drift(&bundle, &layer), None);

    std::fs::write(layer.join("scripts/new.sh"), "#!/bin/sh\n").unwrap();
    assert_eq!(
        layer_drift(&bundle, &layer),
        Some((
            "model-policy".to_string(),
            "added: scripts/new.sh".to_string()
        ))
    );
    std::fs::remove_file(layer.join("scripts/new.sh")).unwrap();

    // The walk's own exclusions are not pinned, so they cannot drift: a
    // realm map or a dialect written beside the bundle moves nothing.
    std::fs::write(layer.join("realms.json"), "{}").unwrap();
    std::fs::create_dir_all(layer.join("dialects")).unwrap();
    std::fs::write(layer.join("dialects/x.json"), "{}").unwrap();
    assert_eq!(layer_drift(&bundle, &layer), None);
}

// ------------------- decision 0046 ruling 6: every shipped bundle under harness

/// Every bundle directory under `recipes/` and `bundles/`, in name order
/// — the thirteen the tree ships.
fn shipped_bundles() -> Vec<PathBuf> {
    let root = workspace();
    let mut dirs = Vec::new();
    for parent in ["recipes", "bundles"] {
        let mut children: Vec<PathBuf> = std::fs::read_dir(root.join(parent))
            .unwrap()
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.join("bundle.json").is_file())
            .collect();
        children.sort();
        dirs.append(&mut children);
    }
    dirs
}

/// The path relative to the workspace, as the record names a bundle.
fn relative(dir: &Path) -> String {
    dir.strip_prefix(workspace())
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/")
}

/// A scratch copy of the shipped adapter library with `adapters/claude.json`
/// rewritten by `plant`, so a test can state what the operator's
/// measurement would land without touching the shipped file.
fn scratch_adapters(plant: impl FnOnce(&mut Value)) -> tempfile::TempDir {
    let scratch = tempfile::tempdir().unwrap();
    for entry in std::fs::read_dir(shipped_adapters()).unwrap() {
        let path = entry.unwrap().path();
        std::fs::copy(&path, scratch.path().join(path.file_name().unwrap())).unwrap();
    }
    let claude = scratch.path().join("claude.json");
    let mut value: Value = serde_json::from_slice(&std::fs::read(&claude).unwrap()).unwrap();
    plant(&mut value);
    std::fs::write(&claude, serde_json::to_string_pretty(&value).unwrap()).unwrap();
    scratch
}

/// Compile one shipped bundle under a boundary, in a realm that declares
/// the openspec dialect, against the given adapter library.
fn compile_shipped(
    dir: &Path,
    adapters: &Path,
    dialect: &Dialect,
    boundary: Boundary,
) -> Result<Bundle, CompileError> {
    Bundle::compile_with_realm(
        dir,
        &workspace().join("agents"),
        adapters,
        Some("brokkr"),
        Some(dialect),
        boundary,
    )
}

/// The refusal DD8 pins for the two shipped bundles with a dialect step:
/// the `analyze` sequence's `check` step is the first the compiler
/// reaches (phases compile in name order), and it is refused naming the
/// step, ruling 4, 0042 ruling 4 and a boxed boundary.
fn assert_refused_at_the_dialect_step(relative: &str, refusal: &str) {
    assert!(
        refusal
            .contains("dialect step 'analyze:check' holds its gate boxed (decision 0042 ruling 4)"),
        "{relative}: {refusal}"
    );
    assert!(
        refusal.contains("which is all decision 0046 ruling 4 admits at an unboxed gate"),
        "{relative}: {refusal}"
    );
    assert!(
        refusal.contains("Run the realm under a boxed boundary (namespace)"),
        "{relative}: {refusal}"
    );
    assert!(!refusal.contains("claude"), "{relative}: {refusal}");
}

/// Decision 0046 ruling 6's promise, proved as one test with two halves
/// (design DD20; tasks 11.4 and 11.7).
///
/// First half: in a scratch copy of the shipped adapter library with
/// claude's two `hands.harness` members planted as fragments — the
/// shipped files once the operator's measurement lands — every shipped
/// bundle without a dialect step compiles under `harness` in a realm
/// declaring the openspec dialect, eleven of the thirteen, each hands
/// site's manifest `boundary` entry reading `harness`; `recipes/triage`
/// and `recipes/night-shift` refuse naming their dialect step and ruling 4.
///
/// Second half: against the shipped adapters as they stand, claude
/// declaring no member, exactly four refuse, each naming the ground the
/// compiler reaches first — `bundles/self` at `review` and
/// `recipes/panel-review` at `review:correctness` naming `claude`,
/// `hands.harness.gate` and the site; the two dialect bundles at
/// `analyze:check` — and every other compiles. This half is a pin that
/// moves for a known reason: the measurement landing, or a decision
/// admitting the dialect step.
///
/// And under `namespace` every shipped bundle is exactly today: the
/// manifest `compile_with` produces, every `boundary` entry `namespace`.
#[test]
fn every_shipped_bundle_compiles_under_harness_once_the_fragments_are_measured() {
    let root = workspace();
    let dialect = Dialect::load(&root.join("dialects/openspec.json"))
        .unwrap()
        .0;
    let dirs = shipped_bundles();
    assert_eq!(dirs.len(), 13, "{dirs:?}");
    let dialect_bundles = ["recipes/night-shift", "recipes/triage"];

    // Namespace is exactly today.
    for dir in &dirs {
        let name = relative(dir);
        let today = Bundle::compile_with(dir, &root.join("agents"), &root.join("adapters"))
            .unwrap_or_else(|e| panic!("{name} must compile: {e}"));
        let realm = compile_shipped(dir, &root.join("adapters"), &dialect, Boundary::Namespace)
            .unwrap_or_else(|e| panic!("{name} must compile in the realm: {e}"));
        assert_eq!(realm.boundary, Boundary::Namespace);
        let mut theirs = realm.manifest.clone();
        let mut ours = today.manifest.clone();
        // The realm pin is the one key a realm adds; the boundary map is
        // the same under both because both are `namespace`.
        theirs.as_object_mut().unwrap().remove("realms");
        ours.as_object_mut().unwrap().remove("realms");
        assert_eq!(theirs, ours, "{name} under namespace is today's bundle");
        if let Some(map) = today.manifest.get("boundary") {
            assert!(
                map.as_object().unwrap().values().all(|w| w == "namespace"),
                "{name}"
            );
        }
    }

    // First half: the members planted.
    let planted = scratch_adapters(|claude| {
        claude["hands"]["harness"] = json!({
            "gate": ["--permission-mode", "plan", "--door", "{result_path}"],
            "work": ["--permission-mode", "acceptEdits"],
        });
    });
    let mut compiled = Vec::new();
    for dir in &dirs {
        let name = relative(dir);
        match compile_shipped(dir, planted.path(), &dialect, Boundary::Harness) {
            Ok(bundle) => {
                assert!(
                    !dialect_bundles.contains(&name.as_str()),
                    "{name} has a dialect step and may not compile unboxed"
                );
                assert_eq!(bundle.boundary, Boundary::Harness);
                if !bundle.hands.is_empty() {
                    let map = bundle.manifest["boundary"].as_object().unwrap();
                    assert_eq!(
                        map.keys().collect::<Vec<_>>(),
                        bundle.hands.keys().collect::<Vec<_>>(),
                        "{name}"
                    );
                    assert!(map.values().all(|w| w == "harness"), "{name}: {map:?}");
                }
                compiled.push(name);
            }
            Err(error) => {
                assert!(
                    dialect_bundles.contains(&name.as_str()),
                    "{name} refuses under harness with the members planted: {error}"
                );
                assert_refused_at_the_dialect_step(&name, &error.to_string());
            }
        }
    }
    assert_eq!(compiled.len(), 11, "{compiled:?}");

    // Second half: the adapters as they stand.
    let shipped = Adapters::load(&root.join("adapters")).unwrap();
    let claude = &shipped.adapter("claude").unwrap().harness;
    assert!(
        claude.gate.is_none() && claude.work.is_none(),
        "this pin reads the shipped claude adapter with no member declared; when the \
         measurement lands, re-pin the second half against it"
    );
    let mut refused = Vec::new();
    for dir in &dirs {
        let name = relative(dir);
        match compile_shipped(dir, &root.join("adapters"), &dialect, Boundary::Harness) {
            Ok(_) => {}
            Err(error) => {
                let refusal = error.to_string();
                match name.as_str() {
                    "bundles/self" => {
                        assert!(
                            refusal.contains(
                                "seat 'review' gate link 2 resolves to provider 'claude', which \
                                 declares no `hands.harness.gate` fragment"
                            ),
                            "{name}: {refusal}"
                        );
                    }
                    "recipes/panel-review" => {
                        assert!(
                            refusal.contains(
                                "seat 'review:correctness' gate link 2 resolves to provider \
                                 'claude', which declares no `hands.harness.gate` fragment"
                            ),
                            "{name}: {refusal}"
                        );
                    }
                    "recipes/night-shift" | "recipes/triage" => {
                        assert_refused_at_the_dialect_step(&name, &refusal);
                    }
                    other => panic!("{other} refuses under harness: {refusal}"),
                }
                refused.push(name);
            }
        }
    }
    // Recipes first, then bundles, each in name order — the walk's order.
    assert_eq!(
        refused,
        [
            "recipes/night-shift",
            "recipes/panel-review",
            "recipes/triage",
            "bundles/self",
        ]
    );
}

/// The measured-gap path (task 11.5): with claude's `work` member declared
/// unsupported and its `gate` a fragment, a fixture bundle that seats the
/// shipped chief architect as a work seat with hands refuses under
/// `harness` naming `claude`, `hands.harness.work`, the site and the
/// measured reason; the two shipped bundles that seat the chief refuse;
/// and every other shipped bundle still compiles — the promise is
/// reported unmet, never papered over by widening the rule.
#[test]
fn a_measured_claude_gap_is_reported_not_papered_over() {
    let root = workspace();
    let dialect = Dialect::load(&root.join("dialects/openspec.json"))
        .unwrap()
        .0;
    let measured = scratch_adapters(|claude| {
        claude["hands"]["harness"] = json!({
            "gate": ["--permission-mode", "plan", "--door", "{result_path}"],
            "work": {"unsupported": "claude 2.1.x: acceptEdits prompts on every shell call"},
        });
    });

    let fixture = Fixture::new();
    let refusal = fixture
        .compile_roots(
            boxed_seat("chief-architect", "work"),
            &root.join("agents"),
            measured.path(),
            Boundary::Harness,
        )
        .expect_err("the chief's first link is claude's fable")
        .to_string();
    assert!(
        refusal.contains(
            "seat 'work' link 1 resolves to provider 'claude', which declares no \
             `hands.harness.work` fragment (claude 2.1.x: acceptEdits prompts on every shell \
             call): a capability gap"
        ),
        "{refusal}"
    );
    // The same chief as a gate would be admitted on the planted gate
    // fragment: the gap is the work member's alone.
    fixture
        .compile_roots(
            boxed_seat("chief-architect", "gate"),
            &root.join("agents"),
            measured.path(),
            Boundary::Harness,
        )
        .expect("the gate member is declared");

    let mut refused = Vec::new();
    for dir in shipped_bundles() {
        let name = relative(&dir);
        if let Err(error) = compile_shipped(&dir, measured.path(), &dialect, Boundary::Harness) {
            assert_refused_at_the_dialect_step(&name, &error.to_string());
            refused.push(name);
        }
    }
    assert_eq!(refused, ["recipes/night-shift", "recipes/triage"]);
}
