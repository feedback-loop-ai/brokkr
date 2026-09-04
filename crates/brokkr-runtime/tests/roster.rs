//! Decision 0041 rulings 1–3: the shipped recipes seat the library roster.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use brokkr_protocol::hands::BindMode;
use brokkr_runtime::{Bundle, SeatBody, SeatClass};
use serde_json::Value;

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn json(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn names_word(text: &str, word: &str) -> bool {
    text.split(|character: char| !character.is_ascii_alphanumeric())
        .any(|candidate| candidate == word)
}

fn walk<'a>(
    value: &'a Value,
    path: &mut Vec<String>,
    visit: &mut impl FnMut(&[String], &'a Value),
) {
    visit(path, value);
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                path.push(key.clone());
                walk(child, path, visit);
                path.pop();
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                path.push(index.to_string());
                walk(child, path, visit);
                path.pop();
            }
        }
        _ => {}
    }
}

#[test]
fn shipped_model_sites_name_the_library_outside_the_ruled_exceptions() {
    let root = workspace();
    let recipes = std::fs::read_dir(root.join("recipes")).unwrap();
    for entry in recipes.flatten() {
        let bundle_path = entry.path().join("bundle.json");
        if !bundle_path.is_file() {
            continue;
        }
        let recipe = entry.file_name().to_string_lossy().into_owned();
        let bundle = json(&bundle_path);
        walk(&bundle, &mut Vec::new(), &mut |path, value| {
            let Some(command) = value
                .get("driver")
                .and_then(|driver| driver.get("command"))
                .and_then(Value::as_array)
            else {
                return;
            };
            let model_backed = command.windows(3).any(|tokens| {
                tokens[0] == "{brokkr}"
                    && tokens[1] == "driver"
                    && matches!(
                        tokens[2].as_str(),
                        Some("claude" | "codex" | "dsh" | "lanetally")
                    )
            });
            if !model_backed {
                return;
            }
            // Two ruling-2 readings are deliberately pinned here before
            // ruling 7 exists. The implement office is hired at fable/high
            // only under the future engine class, so crucible alone seats
            // temporary `implementer-engine`. Inline model sites are only
            // the parity wager harnesses, fast's four-seat quickstart, the
            // Node-owned charters, preflight's CI gates, and night-shift's
            // dsh lane; ruling 8 will make the last three portable.
            let allowed = recipe.starts_with("wager-harness")
                || recipe == "fast"
                || recipe == "node"
                || recipe == "preflight"
                || (recipe == "night-shift" && path.iter().any(|part| part == "implement"));
            assert!(
                allowed,
                "{recipe} has inline model site at {}",
                path.join(".")
            );
        });
    }
}

#[test]
fn tool_grants_are_invoked_by_the_library_and_effort_never_rises_on_fallback() {
    let root = workspace();
    let rank = |effort: &str| match effort {
        "none" => 0,
        "minimal" => 1,
        "low" => 2,
        "medium" => 3,
        "high" => 4,
        "xhigh" => 5,
        "max" => 6,
        other => panic!("unknown effort {other}"),
    };
    for entry in std::fs::read_dir(root.join("agents")).unwrap().flatten() {
        if entry.path().extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let agent = json(&entry.path());
        let charter =
            std::fs::read_to_string(root.join("agents").join(agent["charter"].as_str().unwrap()))
                .unwrap();
        // Forced readings from the 0041 review: a charter invokes a tool
        // only when it names that tool as a word in the work it orders; a
        // substring such as "commit" is not an invocation of `git`.
        // Decision 0043 ruling 2 makes `hands` replace the allow-list, so
        // ruling 5's historical grants live in the journal, not a dead
        // `tools` field beside `hands`.
        if agent.get("hands").is_some() {
            assert!(
                agent.get("tools").is_none(),
                "{} declares dead tools beside hands",
                entry.path().display()
            );
        }
        if let Some(allow) = agent.pointer("/tools/allow").and_then(Value::as_array) {
            for tool in allow.iter().filter_map(Value::as_str) {
                assert!(
                    names_word(&charter, tool),
                    "{} grants unused tool {tool}",
                    entry.path().display()
                );
            }
        }
        let models = agent["models"].as_array().unwrap();
        let efforts = agent["efforts"].as_object().unwrap();
        let first = models[0].as_str().unwrap();
        let first_rank = rank(efforts[first].as_str().unwrap());
        // Triage's ruling-6 office is explicitly pinned fable/xhigh then
        // opus/max by the commission, the one deliberate rising fallback.
        if entry.file_name() == "triage.json" {
            continue;
        }
        for later in &models[1..] {
            let later = later.as_str().unwrap();
            assert!(
                first_rank >= rank(efforts[later].as_str().unwrap()),
                "{} hires {first} below fallback {later}",
                entry.path().display()
            );
        }
    }
}

#[test]
fn shipped_claude_implementer_can_commit() {
    let root = workspace();
    let bundle = Bundle::compile_with(
        &root.join("bundles/self"),
        &root.join("agents"),
        &root.join("adapters"),
    )
    .expect("self bundle compiles");
    let SeatBody::Single { command, .. } = &bundle.seats["implement"].body else {
        panic!("the library implementer is a single seat")
    };
    let allowed_tools = command
        .windows(2)
        .find(|pair| pair[0] == "--allowedTools")
        .map(|pair| pair[1].as_str())
        .expect("claude implementer resolves an allow-list");
    assert!(
        allowed_tools.split(',').any(|tool| tool == "Bash(git:*)"),
        "the shipped claude implementer must be able to commit with git"
    );
}

#[test]
fn every_shipped_verify_and_ship_office_is_a_boxed_exec_script() {
    let root = workspace();
    let mut shipped = Vec::new();
    for entry in std::fs::read_dir(root.join("recipes")).unwrap().flatten() {
        let bundle_path = entry.path().join("bundle.json");
        if !bundle_path.is_file() {
            continue;
        }
        shipped.push((
            format!("recipes/{}", entry.file_name().to_string_lossy()),
            entry.path(),
        ));
    }
    for name in ["self", "verify"] {
        shipped.push((format!("bundles/{name}"), root.join("bundles").join(name)));
    }
    for (name, path) in shipped {
        let source: Value = serde_json::from_slice(
            &std::fs::read(path.join("bundle.json"))
                .unwrap_or_else(|error| panic!("{name} source reads: {error}")),
        )
        .unwrap_or_else(|error| panic!("{name} source parses: {error}"));
        let bundle = Bundle::compile_with(&path, &root.join("agents"), &root.join("adapters"))
            .unwrap_or_else(|error| panic!("{name} compiles: {error}"));
        for phase in ["verify", "ship"] {
            let Some(seat) = bundle.seats.get(phase) else {
                continue;
            };
            assert!(seat.has_gate, "{name}:{phase} is gate-class");
            assert!(bundle.hands.contains_key(phase), "{name}:{phase} is boxed");
            let SeatBody::Single {
                command,
                candidates,
                ..
            } = &seat.body
            else {
                panic!("{name}:{phase} is one deterministic script")
            };
            assert!(candidates.is_empty(), "{name}:{phase} seats no model");
            assert_eq!(&command[1..4], ["driver", "exec", "--"], "{name}:{phase}");
            let resolved_script = Path::new(
                command
                    .get(5)
                    .unwrap_or_else(|| panic!("{name}:{phase} has no script argv")),
            );
            assert!(
                resolved_script.is_file(),
                "{name}:{phase} resolved script exists: {}",
                resolved_script.display()
            );
            if let Some(source_script) = source
                .pointer(&format!("/seats/{phase}/driver/command/5"))
                .and_then(Value::as_str)
            {
                assert!(
                    source_script.starts_with("./"),
                    "{name}:{phase} script is bundle-relative: {source_script}"
                );
            }
            let script = bundle
                .roots
                .iter()
                .find_map(|root| resolved_script.strip_prefix(root).ok());
            // Follow the store helper's temp-path rule: compare Paths in the
            // spelling the product promises, without baking in a host separator.
            if phase == "ship" {
                let shipped = Path::new("scripts").join("ship-seat.sh");
                assert_eq!(script, Some(shipped.as_path()), "{name}:{phase}");
                assert!(
                    bundle.hands[phase].binds.is_empty(),
                    "{name}:{phase} needs no toolchain or credential-bearing bind"
                );
            } else {
                let scripts = Path::new("scripts").join("verify-seat.sh");
                let roles = Path::new("roles").join("verify-seat.sh");
                assert!(
                    script == Some(scripts.as_path()) || script == Some(roles.as_path()),
                    "{name}:{phase} names a shipped verifier script: {script:?}"
                );
                let binds = &bundle.hands[phase].binds;
                if name == "recipes/node" {
                    assert_eq!(binds.len(), 1, "{name}:{phase}");
                    assert_eq!(binds[0].path, "~/.npm");
                    assert_eq!(binds[0].mode, BindMode::Overlay);
                }
            }
        }
    }
}

#[test]
fn shipped_sdd_gate_scope_stops_at_the_judging_sites() {
    let root = workspace();
    let bundle = Bundle::compile_with(
        &root.join("recipes/sdd"),
        &root.join("agents"),
        &root.join("adapters"),
    )
    .expect("sdd bundle compiles");

    for phase in ["review", "verify", "ship"] {
        assert!(bundle.seats[phase].has_gate, "{phase} must remain a gate");
    }
    for phase in ["implement", "design"] {
        assert!(
            !bundle.seats[phase].has_gate,
            "{phase} includes work and must not guard the whole effect"
        );
    }

    let SeatBody::Sequence { steps } = &bundle.seats["design"].body else {
        panic!("sdd design remains a sequence")
    };
    assert_eq!(steps[1].name, "chief");
    assert_eq!(steps[1].class, SeatClass::Work);
    assert_eq!(steps.last().unwrap().name, "speckit-check");
    assert_eq!(steps.last().unwrap().class, SeatClass::Gate);
}

#[test]
fn every_shipped_panel_seats_at_least_two_model_families() {
    let root = workspace();
    for entry in std::fs::read_dir(root.join("recipes")).unwrap().flatten() {
        let bundle_path = entry.path().join("bundle.json");
        if !bundle_path.is_file() {
            continue;
        }
        let bundle = json(&bundle_path);
        walk(&bundle, &mut Vec::new(), &mut |path, value| {
            let Some(panel) = value.get("panel").and_then(Value::as_object) else {
                return;
            };
            assert!(
                panel.len() >= 2,
                "panel {} in {} has fewer than two members",
                path.join("."),
                bundle_path.display()
            );
            let mut families: BTreeSet<String> = BTreeSet::new();
            for member in panel.values() {
                if let Some(agent) = member.get("agent").and_then(Value::as_str) {
                    let definition = json(&root.join("agents").join(format!("{agent}.json")));
                    families.extend(
                        definition["models"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string),
                    );
                } else if let Some(command) =
                    member.pointer("/driver/command").and_then(Value::as_array)
                {
                    for token in command.iter().filter_map(Value::as_str) {
                        if token.contains("fable") {
                            families.insert("fable".into());
                        }
                        if token.contains("opus") {
                            families.insert("opus".into());
                        }
                        if token.contains("gpt-") {
                            families.insert("sol".into());
                        }
                    }
                }
            }
            // The accepted table deliberately gives both sdd-paranoid
            // members the reviewer row (fable → opus → sol), while ruling
            // 7's per-site `select` does not exist yet. In this slice a
            // panel therefore seats the full fallback chains, not distinct
            // selected heads; this test requires two members and at least
            // two abstract families across those seated chains. Ruling 7
            // will make distinct head selection expressible.
            assert!(
                families.len() >= 2,
                "panel {} in {} seats fewer than two model families",
                path.join("."),
                bundle_path.display()
            );
        });
    }
}

#[test]
fn night_shift_keeps_one_attempt_on_every_roster_gate() {
    let root = workspace();
    let bundle = Bundle::compile_with(
        &root.join("recipes/night-shift"),
        &root.join("agents"),
        &root.join("adapters"),
    )
    .expect("night-shift compiles");
    for gate in ["verify", "review", "ship"] {
        assert_eq!(
            bundle.seats[gate].limits.max_attempts, 1,
            "night-shift's {gate} gate must park after its first failed attempt"
        );
    }
}

#[test]
fn shipped_recipes_have_no_judges_fix_input_and_triage_would_bound_oversized() {
    let root = workspace();
    for parent in ["recipes", "bundles"] {
        for entry in std::fs::read_dir(root.join(parent)).unwrap().flatten() {
            let bundle_path = entry.path().join("bundle.json");
            if !bundle_path.is_file() {
                continue;
            }
            let bundle = json(&bundle_path);
            let compiled =
                Bundle::compile_with(&entry.path(), &root.join("agents"), &root.join("adapters"))
                    .unwrap_or_else(|error| panic!("{}: {error}", bundle_path.display()));
            if let Some(implement) = compiled.seats.get("implement") {
                assert!(
                    implement.results.iter().any(|result| result == "oversized"),
                    "{} does not give implementer the oversized verdict",
                    bundle_path.display()
                );
            }
            let policy_path = entry
                .path()
                .join(bundle["policy"].as_str().unwrap_or("policy.json"));
            walk(&bundle, &mut Vec::new(), &mut |path, value| {
                assert_ne!(
                    value.as_str(),
                    Some("fixes_applied"),
                    "{} declares fixes_applied at {}",
                    bundle_path.display(),
                    path.join(".")
                );
            });
            if !policy_path.is_file() {
                continue;
            }
            let policy = json(&policy_path);
            walk(&policy, &mut Vec::new(), &mut |path, value| {
                assert_ne!(
                    value.as_str(),
                    Some("fixes_applied"),
                    "{} reads fixes_applied at {}",
                    policy_path.display(),
                    path.join(".")
                );
            });

            let has_triage = policy["phases"]
                .as_array()
                .is_some_and(|phases| phases.iter().any(|phase| phase == "triage"));
            if has_triage {
                let rules = policy["rules"].as_array().unwrap();
                assert!(
                    rules.iter().any(|rule| {
                        rule["from"] == "implement"
                            && rule["result"] == "oversized"
                            && rule["next"] == "triage"
                    }),
                    "{} has triage but no oversized return edge",
                    policy_path.display()
                );
                assert!(
                    rules.iter().any(|rule| {
                        rule["from"] == "implement"
                            && rule["result"] == "oversized"
                            && rule["when"]["visits_triage_gte"] == 2
                            && rule["park"] == true
                    }),
                    "{} has triage but no exhausted oversized park",
                    policy_path.display()
                );
            }
        }
    }
}
