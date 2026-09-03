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
        let bundle = self.dir.path().join("bundle");
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
                    "driver": {"command": ["{brokkr}", "driver", "judge", "--", "true"]},
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

    // And a `--model` on this adapter is no longer a pin at all: the
    // provider is never told that flag, so the argv reaches whatever
    // the profile resolves — the unprefixed case, on the adapter's own
    // word, which is exactly what such an argv does at run time.
    fixture
        .compile(unpinned_seat(json!([
            "{brokkr}",
            "driver",
            "many",
            "--",
            "--model",
            "elsewhere/large-1",
            "true"
        ])))
        .expect("a flag this provider does not take pins nothing");

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
fn an_adapter_that_cannot_be_told_a_model_rides_its_own_class_and_no_better() {
    // `model_flag: "unsupported"` is a measured fact about a CLI: there
    // is no flag to carry a pin, so no argv on it can name a route.
    // A route-shaped word in such an argv is a word, not a pin — the
    // site reaches whatever the provider's own profile resolves, which
    // is the unprefixed case, on the adapter's own class.
    let fixture = Fixture::new();
    let mut untellable = many_routes();
    untellable["model_flag"] = json!("unsupported");
    fixture.write_adapter(untellable);

    // NO BETTER: the adapter is uncontracted, and a `nearby/` word in
    // the argv — a route this adapter declares LOCAL — lends the site
    // nothing. A resolver that read the word anyway would let an
    // untellable provider pick its own clearance out of the models map.
    let refusal = fixture.refusal(unpinned_seat(json!([
        "{brokkr}",
        "driver",
        "many",
        "--",
        "--model",
        "nearby/small-1",
        "true"
    ])));
    assert!(
        refusal.contains("on its own declared destination"),
        "{refusal}"
    );
    assert!(
        refusal.contains("whose egress class is uncontracted"),
        "{refusal}"
    );

    // And no worse, and no refusal for illegibility either: the same
    // argv on a contracted adapter compiles, because `Absent` is what
    // an unreadable-flag read HAS to be — there is no pin to fail to
    // read, and nothing to disambiguate.
    let fixture = Fixture::new();
    let mut ruled = many_routes();
    ruled["egress"] = json!("contracted");
    ruled["model_flag"] = json!("unsupported");
    fixture.write_adapter(ruled);
    for command in [
        json!([
            "{brokkr}",
            "driver",
            "many",
            "--",
            "--model",
            "elsewhere/large-1",
            "true"
        ]),
        json!([
            "{brokkr}",
            "driver",
            "many",
            "--",
            "--model",
            "partner/qwen@2024",
            "true"
        ]),
        json!(["{brokkr}", "driver", "many", "--", "--model"]),
    ] {
        fixture
            .compile(unpinned_seat(command.clone()))
            .unwrap_or_else(|error| panic!("{command}: {error}"));
    }
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
            {"name": "first", "role": "roles/role.md",
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
    // `uncontracted`, so every adapter as it stands today resolves to
    // exactly the clearance it had the day before this decision landed.
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
        // 0036's ruling assigns NO route to a class, so every model
        // these adapters map resolves to exactly where it stood the day
        // before: an unprefixed id to the adapter's own class, and
        // `dsh`'s prefixed `spark/*` and `dashscope/*` fronts to
        // uncontracted — which for `dsh` IS its own class, the floor it
        // has always been held to. The place to say otherwise now
        // exists; nobody has.
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
    // installed codex-cli 0.148.0 lists these three slugs (visibility
    // "list", supported_in_api true), so the mapping is transcribed,
    // not remembered. The abstract names are codex's own family words —
    // NOT claude tiers, so no fallback chain written for one provider
    // can quietly land on the other.
    let adapters = Adapters::load(&shipped_adapters()).expect("the shipped adapters load");
    let codex = adapters.adapter("codex").expect("a shipped adapter");
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
