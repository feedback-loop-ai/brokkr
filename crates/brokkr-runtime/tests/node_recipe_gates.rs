//! Decision 0021 ruling 2, applied to `recipes/node` itself.
//!
//! `witness_digests.rs` proves the recipe compiles against the SHIPPED
//! adapters, which is the weaker claim: it holds only as long as the
//! incumbent keeps its tier. What this file proves is the refusal — that
//! the Node recipe's three judging seats stand on a *declaration*, one
//! gate at a time, and that a driver without the trusted tier cannot sit
//! in any of them.
//!
//! Every provider here is invented — `steward`, `apprentice`. The recipe
//! on disk names some real one; this test rewrites that token and never
//! asserts what it was, because a vendor's name belongs in
//! `adapters/`, never in an engine test's expectation.

use std::path::{Path, PathBuf};

use brokkr_runtime::{Adapters, Bundle};
use serde_json::{json, Value};

/// The seats `recipes/node` declares `class: "gate"`, and the order a
/// manifest's `drivers` object lists them in.
const GATES: [&str; 3] = ["review", "ship", "verify"];

/// The one seat that works rather than judges.
const WORK: &str = "implement";

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

/// An adapter as data, in the shape `Adapters::load` reads: a provider
/// that declares a tier and nothing else it does not need.
fn adapter(name: &str, tier: &str) -> Value {
    json!({
        "provider": name,
        "trust_tier": tier,
        "binding_grant": false,
        "binary": name,
        "driver": ["{brokkr}", "driver", name, "--"],
        "models": {},
        "model_flag": "unsupported",
        "tool_permissions": "unsupported",
        "mcp": "unsupported",
    })
}

/// A copy of `recipes/node` whose seats are driven by fixture providers.
struct Fixture {
    dir: tempfile::TempDir,
}

impl Fixture {
    /// The recipe as it ships, copied whole, with each seat's driver
    /// re-pointed at whichever fixture provider `provider_for` names.
    fn new(provider_for: impl Fn(&str) -> &'static str) -> Fixture {
        let fixture = Fixture {
            dir: tempfile::tempdir().expect("a temp dir"),
        };
        let root = fixture.dir.path();
        std::fs::create_dir_all(root.join("adapters")).unwrap();
        std::fs::create_dir_all(root.join("agents")).unwrap();
        for (name, tier) in [("steward", "trusted"), ("apprentice", "untrusted")] {
            std::fs::write(
                root.join(format!("adapters/{name}.json")),
                serde_json::to_string_pretty(&adapter(name, tier)).unwrap(),
            )
            .unwrap();
        }

        copy_tree(&workspace().join("recipes/node"), &root.join("bundle"));
        let path = root.join("bundle/bundle.json");
        let mut config: Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).expect("the recipe is JSON");
        let mut shipped: Vec<String> = Vec::new();
        for (seat, body) in config["seats"].as_object_mut().expect("seats") {
            let command = body["driver"]["command"]
                .as_array_mut()
                .unwrap_or_else(|| panic!("seat '{seat}' drives inline"));
            let at = command
                .iter()
                .position(|token| token == "driver")
                .expect("the seat dispatches a named driver")
                + 1;
            shipped.push(command[at].as_str().expect("a provider name").to_string());
            command[at] = json!(provider_for(seat));
        }
        // Not WHICH provider ships — that is `adapters/` data and a
        // ruling, not an engine fact — only that the recipe speaks with
        // one voice, so re-pointing it seat by seat is a fair rewrite.
        assert!(
            shipped.windows(2).all(|pair| pair[0] == pair[1]),
            "the recipe seats more than one provider: {shipped:?}"
        );
        std::fs::write(&path, serde_json::to_string_pretty(&config).unwrap()).unwrap();
        fixture
    }

    fn compile(&self) -> Result<Bundle, String> {
        Bundle::compile_with(
            &self.dir.path().join("bundle"),
            &self.dir.path().join("agents"),
            &self.dir.path().join("adapters"),
        )
        .map_err(|error| error.to_string())
    }
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).expect("the recipe is readable") {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

#[test]
fn every_gate_of_the_node_recipe_refuses_an_untrusted_driver() {
    // One gate at a time: the other two hold a trusted driver, so the
    // refusal can only be about the seat under test. If a future edit
    // dropped `class: "gate"` from any of the three, this test is the
    // one that notices — the compile would simply succeed.
    for gate in GATES {
        let fixture = Fixture::new(move |seat| {
            if seat == gate {
                "apprentice"
            } else {
                "steward"
            }
        });
        let refusal = fixture
            .compile()
            .expect_err("a gate seating an untrusted driver is refused");
        assert!(refusal.contains(&format!("seat '{gate}'")), "{refusal}");
        assert!(refusal.contains("driver 'apprentice'"), "{refusal}");
        assert!(
            refusal.contains("does not hold the trusted tier"),
            "{refusal}"
        );
        assert!(refusal.contains("0021 ruling 2"), "{refusal}");
    }
}

#[test]
fn the_node_recipes_gates_compile_on_a_trusted_driver_and_pin_it() {
    let fixture = Fixture::new(|_| "steward");
    let bundle = fixture
        .compile()
        .expect("a trusted driver may hold every gate");
    let steward = Adapters::load(&fixture.dir.path().join("adapters"))
        .expect("the fixture adapters load")
        .digest("steward")
        .expect("the fixture declares this provider")
        .to_string();
    let witnessed = bundle.manifest["drivers"]
        .as_object()
        .expect("the gates witness what authorised them");
    let seats: Vec<&str> = witnessed.keys().map(String::as_str).collect();
    assert_eq!(
        seats, GATES,
        "exactly the gate-class seats consulted a declaration"
    );
    for gate in GATES {
        assert_eq!(witnessed[gate], json!({ "steward": steward }));
    }
}

#[test]
fn the_node_recipes_work_seat_takes_an_untrusted_driver() {
    // Ruling 7's newcomer works freely: the refusal is gate-only, and
    // `implement` — which produces output the three gates then check —
    // is where an unpromoted driver may sit.
    let fixture = Fixture::new(|seat| {
        if seat == WORK {
            "apprentice"
        } else {
            "steward"
        }
    });
    fixture
        .compile()
        .expect("a work seat needs no tier of its driver");
}
