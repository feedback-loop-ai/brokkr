//! Decision 0041 rulings 1–3: the shipped recipes seat the library roster.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use brokkr_protocol::hands::BindMode;
use brokkr_runtime::{
    resolve_agent, Adapters, Availability, Bundle, Candidate, Library, PanelMember, Presence,
    SeatBody, SeatClass, StepBody,
};
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

fn is_house_tool_grant(agent: &str, tool: &str) -> bool {
    matches!(
        (agent, tool),
        ("chief-architect", "git")
            | (
                "fast-implementer" | "fast-reviewer",
                "cargo" | "git" | "ls" | "rg" | "mkdir"
            )
            | ("implementer-sdd", "cargo" | "git")
            | ("implementer", "cargo" | "git")
            | ("intake", "git")
            | (
                "node-implementer" | "node-reviewer",
                "npm" | "npx" | "node" | "git" | "ls" | "rg" | "mkdir"
            )
            | (
                "verify-reviewer",
                "cargo" | "git" | "ls" | "rg" | "gh-pr-view" | "gh-run-view"
            )
    )
}

/// A charter is an office; the house is the realm's. Library charters
/// therefore cannot smuggle this repository's paths or commands into every
/// adopter's prompt. Recipe-local roles are intentionally outside this walk.
#[test]
fn library_charters_name_no_repository_tokens() {
    let root = workspace().join("agents/charters");
    let forbidden = [
        "cargo",
        "crates/",
        "recipes/self",
        "decision 00",
        "policy/",
        "fixtures/",
        "contracts/",
        "specs/",
        "openspec/",
        ".specify/",
    ];
    for entry in std::fs::read_dir(&root).unwrap().flatten() {
        let text = std::fs::read_to_string(entry.path()).unwrap();
        let lowered = text.to_ascii_lowercase();
        for token in forbidden {
            assert!(
                !lowered.contains(token),
                "{} names repository token {token:?}",
                entry.path().display()
            );
        }
    }
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

/// The root of a shipped recipe's own `bundle.json`, as the roster reads
/// it: the decision that forces its crew, when it declares one, and the
/// seats this layer writes itself.
#[derive(serde::Deserialize)]
struct Root {
    forced_crew: Option<String>,
    #[serde(default)]
    seats: serde_json::Map<String, Value>,
}

/// Every shipped recipe's own layer: its name, its root, and the paths of
/// the inline model-backed sites it writes.
fn shipped_layers() -> Vec<(String, Root, Vec<String>)> {
    let mut layers = Vec::new();
    for entry in std::fs::read_dir(workspace().join("recipes"))
        .unwrap()
        .flatten()
    {
        let bundle_path = entry.path().join("bundle.json");
        if !bundle_path.is_file() {
            continue;
        }
        let bundle = json(&bundle_path);
        let mut inline = Vec::new();
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
            if model_backed {
                inline.push(path.join("."));
            }
        });
        let root: Root = serde_json::from_value(bundle).unwrap();
        let recipe = entry.file_name().to_string_lossy().into_owned();
        layers.push((recipe, root, inline));
    }
    layers
}

/// Decision 0041's addendum of 2026-10-07 (#360): the agent overlay is the
/// one forcing mechanism, and a recipe may force its crew only by declaring
/// why — `"forced_crew": "<the decision that forces it>"` at its root, read
/// here in place of a list of recipe names. An inline model site stands
/// only in a recipe that declares one, and only where the site has no
/// overlay form yet: a codex seat's sandbox, which no agent without hands
/// can carry until decision 0065 slice one's lowering does (the wager's
/// Codex arm, the standby hedge, review-first's Sol judge); research-dsh's
/// page-fetch `--patch`, which no agent writes; and preflight's role, which
/// names repository paths no library charter may.
#[test]
fn an_inline_model_site_stands_only_in_a_recipe_that_declares_its_forced_crew() {
    for (recipe, root, inline) in shipped_layers() {
        if root.forced_crew.is_none() {
            assert_eq!(
                inline,
                Vec::<String>::new(),
                "{recipe} pins a model inline without a declared forced_crew"
            );
        }
    }
}

/// The keys a recipe's seat may write without authoring its body: a seat
/// that writes only these keeps the body it inherits. Any other key — a
/// body form, or a key this list does not know — authors the body.
const INHERITING_KEYS: [&str; 4] = ["inputs", "limits", "results", "secrets"];

/// Every executable site a compiled seat body holds, with the chain its
/// candidates hire: a single seat, each panel member, each sequence step
/// and its members, and each select case and the default. An inline or
/// deterministic site hires an empty chain.
fn hired_chains<'a>(site: String, body: &'a SeatBody, chains: &mut Vec<(String, &'a [Candidate])>) {
    let members = |site: &str, members: &'a [PanelMember], chains: &mut Vec<_>| {
        for member in members {
            chains.push((format!("{site}:{}", member.name), &member.candidates[..]));
        }
    };
    match body {
        SeatBody::Single { candidates, .. } => chains.push((site, candidates)),
        SeatBody::Panel { members: panel, .. } => members(&site, panel, chains),
        SeatBody::Sequence { steps } => {
            for step in steps {
                let site = format!("{site}:{}", step.name);
                match &step.body {
                    StepBody::Single { candidates, .. } => chains.push((site, candidates)),
                    StepBody::Panel { members: panel, .. } => members(&site, panel, chains),
                    StepBody::Dialect { .. } => {}
                }
            }
        }
        SeatBody::Select { cases, default, .. } => {
            let default = default.iter().map(|body| ("default", body.as_ref()));
            let cases = cases.iter().map(|(case, body)| (case.as_str(), body));
            for (case, body) in cases.chain(default) {
                hired_chains(format!("{site}:{case}"), body, chains);
            }
        }
    }
}

/// A forced crew stays forced (the same addendum): a recipe that declares
/// `forced_crew` names the decision, forces at least one site it writes,
/// and every model site its own seats hold — a single seat, a panel
/// member, a sequence step, a select case or default, read from the
/// compiled body — either stands inline or hires an overlay whose chain
/// holds exactly the forced model, with no fallback. A seat it inherits,
/// or overlays only with the keys above, is not its forcing: a wager's arm
/// judges with fast's review seat, which falls back (the operator's ruling
/// of 2026-10-09).
#[test]
fn a_forced_crew_names_its_decision_and_hires_no_fallback() {
    let workspace = workspace();
    for (recipe, root, inline) in shipped_layers() {
        let Some(reason) = root.forced_crew else {
            continue;
        };
        assert!(
            reason.contains("decision 0"),
            "{recipe}'s forced_crew names no decision: {reason}"
        );
        let bundle = compile(&workspace, &recipe);
        let mut forced = inline.len();
        for (phase, seat) in &root.seats {
            let inherits = seat.as_object().is_some_and(|keys| {
                keys.keys()
                    .all(|key| INHERITING_KEYS.contains(&key.as_str()))
            });
            if inherits {
                continue;
            }
            let mut chains = Vec::new();
            hired_chains(phase.clone(), &bundle.seats[phase].body, &mut chains);
            for (site, candidates) in chains {
                if candidates.is_empty() {
                    continue;
                }
                let chain: Vec<&str> = candidates.iter().map(|link| link.model.as_str()).collect();
                assert_eq!(
                    chain.len(),
                    1,
                    "{recipe} forces its crew, but {site} hires the chain {chain:?}"
                );
                forced += 1;
            }
        }
        assert_ne!(
            forced, 0,
            "{recipe} declares forced_crew but forces no seat it writes"
        );
    }
}

/// A shipped recipe, compiled against the shipped library and adapters.
fn compile(root: &Path, recipe: &str) -> Bundle {
    Bundle::compile_with(
        &root.join("recipes").join(recipe),
        &root.join("agents"),
        &root.join("adapters"),
    )
    .unwrap_or_else(|error| panic!("{recipe} compiles: {error}"))
}

/// The fast offices and the chains they hire (#360, the operator's ruling 2
/// of 2026-10-07): Fable at `high` first, the crew fast always seated
/// inline, and a fallback behind it on every model seat.
const FAST_OFFICES: [(&str, &str, &[&str]); 2] = [
    (
        "implement",
        "fast-implementer",
        &["fable", "opus", "sonnet"],
    ),
    ("review", "fast-reviewer", &["fable", "opus"]),
];

/// `node`'s and `verify`'s offices and the chains they hire (the operator's
/// ruling (b) of 2026-10-09 on #360): Fable, then Opus, at `high`, as the
/// fast offices fall back.
const NODE_AND_VERIFY_OFFICES: [(&str, &str, &str, &[&str]); 3] = [
    ("node", "implement", "node-implementer", &["fable", "opus"]),
    ("node", "review", "node-reviewer", &["fable", "opus"]),
    ("verify", "review", "verify-reviewer", &["fable", "opus"]),
];

/// A compiled seat hires `office` on exactly `chain`, every link at `high`.
fn assert_hires(bundle: &Bundle, recipe: &str, phase: &str, office: &str, chain: &[&str]) {
    let SeatBody::Single { candidates, .. } = &bundle.seats[phase].body else {
        panic!("{recipe}:{phase} is one seat");
    };
    let hired: Vec<(&str, &str, Option<&str>)> = candidates
        .iter()
        .map(|link| {
            (
                link.agent.as_str(),
                link.model.as_str(),
                link.effort.as_deref(),
            )
        })
        .collect();
    let expected: Vec<(&str, &str, Option<&str>)> = chain
        .iter()
        .map(|model| (office, *model, Some("high")))
        .collect();
    assert_eq!(hired, expected, "{recipe}:{phase}");
}

/// `fast` and `landing`, which inherits both of fast's model seats, hire
/// the fast offices, so the default recipe falls back instead of parking
/// when its first model is spent.
#[test]
fn fast_and_landing_hire_offices_that_fall_back_from_fable() {
    let root = workspace();
    for recipe in ["fast", "landing"] {
        let bundle = compile(&root, recipe);
        for (phase, office, chain) in FAST_OFFICES {
            assert_hires(&bundle, recipe, phase, office, chain);
        }
    }
}

/// `node` and `verify` hire their offices, so neither parks when Fable's
/// limit is spent (the 2026-10-09 ruling).
#[test]
fn node_and_verify_hire_offices_that_fall_back_from_fable() {
    let root = workspace();
    for (recipe, phase, office, chain) in NODE_AND_VERIFY_OFFICES {
        assert_hires(&compile(&root, recipe), recipe, phase, office, chain);
    }
}

/// The same ruling's proof that the chain is a fallback: with the provider
/// that serves Fable unavailable — the shipped claude adapter, split so
/// Fable stands alone — each fast office skips Fable and selects Opus at
/// `high`, its next link.
#[test]
fn a_fast_office_whose_first_model_is_unavailable_selects_its_fallback() {
    assert_falls_back_from_unavailable_fable(
        &FAST_OFFICES.map(|(_, office, chain)| (office, chain)),
    );
}

/// The 2026-10-09 ruling's proof, as fast's: with Fable unavailable,
/// `node`'s and `verify`'s offices select Opus at `high`.
#[test]
fn a_node_or_verify_office_whose_first_model_is_unavailable_selects_its_fallback() {
    assert_falls_back_from_unavailable_fable(
        &NODE_AND_VERIFY_OFFICES.map(|(_, _, office, chain)| (office, chain)),
    );
}

/// Each office, resolved with the provider that serves Fable unavailable,
/// skips Fable and selects the rest of its chain at `high`.
fn assert_falls_back_from_unavailable_fable(offices: &[(&str, &[&str])]) {
    let root = workspace();
    let adapters = tempfile::tempdir().unwrap();
    let mut claude = json(&root.join("adapters/claude.json"));
    let mut fable = claude.clone();
    fable["provider"] = "claude-fable".into();
    fable["models"] = serde_json::json!({"fable": claude["models"]["fable"]});
    fable["judges"] = serde_json::json!(["fable"]);
    claude["models"].as_object_mut().unwrap().remove("fable");
    claude["judges"] = serde_json::json!(["opus"]);
    for (name, body) in [("claude", claude), ("claude-fable", fable)] {
        let path = adapters.path().join(format!("{name}.json"));
        std::fs::write(path, serde_json::to_vec_pretty(&body).unwrap()).unwrap();
    }
    let adapters = Adapters::load(adapters.path()).expect("the split adapters load");
    let library = Library::load(&root.join("agents")).expect("the shipped library loads");
    let mut availability = Availability::unspecified();
    availability.record("claude-fable", Presence::Unavailable);
    for &(office, chain) in offices {
        let resolution = resolve_agent(&library, &adapters, &availability, office)
            .unwrap_or_else(|error| panic!("{office} resolves: {error}"));
        assert_eq!(resolution.record["chosen_index"], 1, "{office}");
        assert_eq!(
            resolution.record["skipped"],
            serde_json::json!([{"model": "fable", "reason": "unavailable"}]),
            "{office}"
        );
        let selected: Vec<(&str, Option<&str>)> = resolution
            .candidates
            .iter()
            .map(|link| (link.model.as_str(), link.effort.as_deref()))
            .collect();
        let fallbacks: Vec<(&str, Option<&str>)> = chain[1..]
            .iter()
            .map(|model| (*model, Some("high")))
            .collect();
        assert_eq!(selected, fallbacks, "{office}");
    }
}

#[test]
fn tool_grants_keep_house_tools_explicit_and_effort_never_rises_on_fallback() {
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
    // The loaded library, not the files: an overlay (#360) states only its
    // difference, and what is judged here is the office it stands for.
    let library = Library::load(&root.join("agents")).expect("the shipped library loads");
    for agent in library.agents() {
        let name = &agent.name;
        let allow = agent.allow.as_deref().unwrap_or_default();
        assert!(
            !allow.iter().any(|tool| tool == "specify"),
            "{name} still grants the retired specify tool"
        );
        let charter = std::fs::read_to_string(&agent.charter).unwrap();
        // A portable charter names only office tools. House tools are an
        // explicit property of the shipped agent, independent of whichever
        // realm happens to use that agent; a substring such as "commit" is
        // not an invocation of `git`.
        // Decision 0043 ruling 2 makes `hands` replace the allow-list, so
        // ruling 5's historical grants live in the journal, not a dead
        // `tools` field beside `hands`.
        if agent.hands.is_some() {
            assert!(
                agent.allow.is_none() && agent.sandbox.is_none(),
                "{name} declares dead tools beside hands"
            );
        }
        for tool in allow {
            assert!(
                is_house_tool_grant(name, tool) || names_word(&charter, tool),
                "{name} grants an unaccounted tool {tool}"
            );
        }
        // Triage's ruling-6 office is explicitly pinned fable/xhigh then
        // opus/max by the commission, the one deliberate rising fallback. A
        // single link has no fallback to rise over, and an effortless lane
        // (dsh's glm-flash, #360) names no effort to rank.
        if name == "triage" || agent.models.len() == 1 {
            continue;
        }
        // Sol's scale sits one step down (decision 0045's addendum of
        // 2026-09-30): its `high` is the level another model's `xhigh` is.
        let rank_of = |model: &str| rank(&agent.efforts[model]) + i32::from(model == "sol");
        let first = &agent.models[0];
        let first_rank = rank_of(first);
        // Astra at `max` as a chief's last fallback is the other one, by
        // the same addendum; `sol_is_capped_and_astra_is_a_chiefs_last_fallback`
        // pins where it may stand.
        for later in agent.models[1..].iter().filter(|model| *model != "astra") {
            assert!(
                first_rank >= rank_of(later),
                "{name} hires {first} below fallback {later}"
            );
        }
    }
}

/// Issue #307 (operator rulings 2026-09-20 and 2026-09-21): the engine
/// smith hires codex, then fable at high — sol at medium since the
/// 2026-09-30 roster ruling moved astra's seats to Sol 6.1 one step down —
/// and its power is decision
/// 0043's workspace box — no network, the Cargo home as a masked overlay
/// and the toolchain read-only — never a tool list, which hands would
/// leave dead on both providers. The boundary stays the realm's fact and
/// the workdir stays the box's own mount, so neither is declared here.
#[test]
fn the_engine_smith_hires_sol_then_fable_through_workspace_hands() {
    let agent = json(&workspace().join("agents/implementer-engine.json"));
    assert_eq!(
        agent,
        serde_json::json!({
            "description": "Engine-class implementer: builds core, store, contract, and policy work selected by triage.",
            "charter": "charters/implementer.md",
            "models": ["sol", "fable"],
            "efforts": {"sol": "medium", "fable": "high"},
            "hands": {
                "kind": "workspace",
                "network": false,
                "binds": [
                    {
                        "path": "~/.cargo",
                        "mode": "overlay",
                        "mask": ["credentials.toml", "credentials"]
                    },
                    {"path": "~/.rustup", "mode": "ro"}
                ]
            },
            "limits": {"max_attempts": 2, "timeout_seconds": 7200}
        })
    );
}

#[test]
fn shipped_claude_implementer_can_commit() {
    let root = workspace();
    let bundle = Bundle::compile_with(
        &root.join("recipes/self"),
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
#[expect(clippy::too_many_lines, reason = "baseline 2026-09, #288")]
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
            let (command, candidates) = match &seat.body {
                SeatBody::Single {
                    command,
                    candidates,
                    ..
                } => {
                    assert!(bundle.hands.contains_key(phase), "{name}:{phase} is boxed");
                    (command, candidates)
                }
                SeatBody::Sequence { steps } if phase == "verify" => {
                    assert_eq!(steps.len(), 2, "{name}:{phase} checks then dialect verify");
                    assert_eq!(steps[0].name, "checks");
                    assert!(matches!(steps[1].body, StepBody::Dialect { .. }));
                    assert!(bundle.hands.contains_key("verify:checks"));
                    assert!(bundle.hands.contains_key("verify:dialect-verify"));
                    let StepBody::Single {
                        command,
                        candidates,
                        ..
                    } = &steps[0].body
                    else {
                        panic!("{name}:{phase}:checks is one deterministic script")
                    };
                    (command, candidates)
                }
                _ => panic!("{name}:{phase} is a deterministic script or dialect sequence"),
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
                // The recipe-local exception is Node's verifier: its npm
                // sequence is a different office, not a library charter with
                // house prose left in it.
                let roles = Path::new("roles").join("verify-seat.sh");
                assert!(
                    script == Some(scripts.as_path()) || script == Some(roles.as_path()),
                    "{name}:{phase} names a shipped verifier script: {script:?}"
                );
                let hands_name = if matches!(seat.body, SeatBody::Sequence { .. }) {
                    "verify:checks"
                } else {
                    phase
                };
                let binds = &bundle.hands[hands_name].binds;
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
fn shipped_triage_gate_scope_stops_at_the_judging_sites() {
    let root = workspace();
    let bundle = Bundle::compile_with(
        &root.join("recipes/triage"),
        &root.join("agents"),
        &root.join("adapters"),
    )
    .expect("triage bundle compiles");

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
        panic!("triage design remains a sequence")
    };
    assert_eq!(steps[1].name, "chief");
    assert_eq!(steps[1].class, SeatClass::Work);
    assert_eq!(steps.last().unwrap().name, "validate");
    assert_eq!(steps.last().unwrap().class, SeatClass::Gate);
}

/// The adapter that maps an abstract model name; the test's notion of a
/// vendor is the adapter file, never a substring of the name.
fn provider_of(root: &Path, abstract_model: &str) -> String {
    for entry in std::fs::read_dir(root.join("adapters")).unwrap().flatten() {
        let adapter = json(&entry.path());
        if adapter["models"].get(abstract_model).is_some() {
            return adapter["provider"].as_str().unwrap().to_string();
        }
    }
    panic!("no shipped adapter maps {abstract_model}")
}

/// Decision 0045 ruling 4: codex declares no per-tool map, so an office
/// that keeps a tool allow-list can never resolve on `astra` — the
/// resolver refuses. The roster therefore chains a codex lane only into
/// offices whose hands are boxed (decision 0043) or absent, and this pin
/// keeps a later edit from writing a chain the compiler would refuse at
/// the first bundle that hires it.
#[test]
fn a_codex_lane_is_chained_only_into_boxed_or_toolless_offices() {
    let root = workspace();
    let codex = json(&root.join("adapters/codex.json"));
    let lanes: BTreeSet<&str> = codex["models"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert!(lanes.contains("astra"), "the codex adapter maps astra");
    let mut chained = 0;
    // The loaded library, so an overlay's inherited tools are judged (#360).
    let library = Library::load(&root.join("agents")).expect("the shipped library loads");
    for agent in library.agents() {
        if !agent
            .models
            .iter()
            .any(|model| lanes.contains(model.as_str()))
        {
            continue;
        }
        chained += 1;
        assert!(
            agent.allow.is_none() && agent.sandbox.is_none(),
            "{} chains a codex lane beside a tool allow-list codex cannot map",
            agent.name
        );
    }
    assert!(
        chained >= 8,
        "the roster chains codex into {chained} offices"
    );
}

/// The chiefs: the offices that rule on a panel's or a council's work.
const CHIEFS: [&str; 3] = ["chief-architect", "gpt-flash-review-chief", "review-chief"];

/// Sol's cap (decision 0045's addendum, ruling 3): no effort above `high`.
fn within_sol_cap(effort: &str) -> bool {
    matches!(effort, "none" | "minimal" | "low" | "medium" | "high")
}

/// Every inline codex model site in a shipped bundle: where it stands, and
/// the concrete model and the effort its driver command pins. A codex site
/// that pins either one by no flag fails the walk instead of passing unread.
fn inline_codex_pins(root: &Path) -> Vec<(String, String, String)> {
    let mut pins = Vec::new();
    for dir in shipped_bundle_dirs(root) {
        let bundle = json(&dir.join("bundle.json"));
        let name = dir.file_name().unwrap().to_string_lossy().into_owned();
        walk(&bundle, &mut Vec::new(), &mut |path, value| {
            let words: Vec<&str> = value
                .get("driver")
                .and_then(|driver| driver.get("command"))
                .and_then(Value::as_array)
                .map(|command| command.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            if !words.starts_with(&["{brokkr}", "driver", "codex"]) {
                return;
            }
            let site = format!("{name} at {}", path.join("."));
            let pinned = |flag: &str| {
                let at = words.iter().position(|word| *word == flag);
                at.and_then(|at| words.get(at + 1))
                    .unwrap_or_else(|| panic!("{site} pins no {flag}"))
                    .to_string()
            };
            pins.push((site.clone(), pinned("--model"), pinned("--effort")));
        });
    }
    pins
}

/// The operator's roster ruling of 2026-09-30 (decision 0045's addendum):
/// `sol` is Sol 6.1, its effort is capped at `high`, and `astra` stands
/// only as a chief's last fallback, at `max`. The inline codex sites of
/// decision 0041 ruling 7 are held to the same two rules by the concrete
/// model they pin, so a recipe cannot hire past the library's cap.
#[test]
fn sol_is_capped_and_astra_is_a_chiefs_last_fallback() {
    let root = workspace();
    let codex = json(&root.join("adapters/codex.json"));
    assert_eq!(codex["models"]["sol"], "gpt-6.1-sol", "sol is Sol 6.1");
    let inline = inline_codex_pins(&root);
    assert!(
        inline.len() >= 4,
        "{inline:?} are too few inline codex sites"
    );
    for (site, model, effort) in &inline {
        assert_ne!(
            codex["models"]["astra"], *model,
            "{site} pins astra inline; astra is only a chief's last fallback"
        );
        assert!(
            codex["models"]["sol"] != *model || within_sol_cap(effort),
            "{site} pins sol inline at {effort}, above its cap of high"
        );
    }
    let mut astra_chiefs = BTreeSet::new();
    for entry in std::fs::read_dir(root.join("agents")).unwrap().flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let name = path.file_stem().unwrap().to_str().unwrap().to_string();
        let agent = json(&path);
        if let Some(effort) = agent["efforts"]["sol"].as_str() {
            assert!(
                within_sol_cap(effort),
                "{name} hires sol at {effort}, above its cap of high"
            );
        }
        let models: Vec<&str> = agent["models"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        if let Some(link) = models.iter().position(|model| *model == "astra") {
            assert!(
                CHIEFS.contains(&name.as_str()) && link + 1 == models.len(),
                "{name} chains astra at link {link} of {models:?}; astra is only a chief's last fallback"
            );
            assert_eq!(
                agent["efforts"]["astra"], "max",
                "{name} hires astra below max"
            );
            astra_chiefs.insert(name);
        }
    }
    assert_eq!(
        astra_chiefs,
        BTreeSet::from(CHIEFS.map(String::from)),
        "every chief falls back to astra last"
    );
}

#[test]
fn every_shipped_panel_seats_at_least_two_providers() {
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
            // Decision 0045 ruling 3: a panel is diverse when its FIRST
            // hires cross a vendor line, not when a fallback might. Two
            // models of one vendor at one effort still argue with
            // themselves; two vendors do not share a training run.
            let mut providers: BTreeSet<String> = BTreeSet::new();
            for member in panel.values() {
                if let Some(agent) = member.get("agent").and_then(Value::as_str) {
                    let definition = json(&root.join("agents").join(format!("{agent}.json")));
                    let first = definition["models"][0].as_str().unwrap();
                    providers.insert(provider_of(&root, first));
                } else if let Some(command) =
                    member.pointer("/driver/command").and_then(Value::as_array)
                {
                    // An inline site names its driver after `driver`.
                    let position = command
                        .iter()
                        .position(|token| token.as_str() == Some("driver"))
                        .expect("an inline driver names a built-in");
                    providers.insert(command[position + 1].as_str().unwrap().to_string());
                }
            }
            assert!(
                providers.len() >= 2,
                "panel {} in {} seats one vendor at its first hires: {providers:?}",
                path.join("."),
                bundle_path.display()
            );
        });
    }
}

#[test]
fn night_shift_keeps_one_attempt_on_every_phase() {
    let root = workspace();
    let bundle = Bundle::compile_with(
        &root.join("recipes/night-shift"),
        &root.join("agents"),
        &root.join("adapters"),
    )
    .expect("night-shift compiles");
    for phase in [
        "triage",
        "specify",
        "clarify",
        "design",
        "tasks",
        "analyze",
        "implement",
        "verify",
        "review",
        "ship",
    ] {
        assert_eq!(
            bundle.seats[phase].limits.max_attempts, 1,
            "night-shift's {phase} seat must park after its first failed attempt"
        );
    }
}

/// The operator's ruling of 2026-10-04 (#532): the shipped dsh
/// implementation lanes run GLM-5.3 Flash on the `spark-glm` route, the
/// dsh adapter's `glm-flash`. The route is effortless (dsh 0.1.5-rc.1
/// refuses reasoningEffort on it, measured 2026-09-16), so the composed
/// command pins the model and no effort.
#[test]
fn the_dsh_implement_lanes_pin_glm_flash_with_no_effort() {
    let root = workspace();
    let dsh = json(&root.join("adapters/dsh.json"));
    assert_eq!(dsh["models"]["glm-flash"], "spark-glm/GLM-5.3-Flash-EXL3");
    assert!(dsh["effortless_routes"]["spark-glm"].is_string());
    for recipe in ["night-shift", "wager-harness-dsh"] {
        let bundle = compile(&root, recipe);
        let SeatBody::Single { command, .. } = &bundle.seats["implement"].body else {
            panic!("{recipe}'s implement seat is one inline session");
        };
        // `{brokkr}` composes to the running binary, this test's own.
        assert_eq!(
            command[0],
            std::env::current_exe().unwrap().to_string_lossy(),
            "{recipe}"
        );
        assert_eq!(
            command[1..],
            [
                "driver",
                "dsh",
                "--",
                "--model",
                "spark-glm/GLM-5.3-Flash-EXL3",
            ],
            "{recipe}'s implement lane"
        );
    }
}

/// The agents a library holds for a consumer outside every shipped
/// bundle, each with the seat that hires it. `muninn` is composed
/// outside a bundle by `brokkr muninn` (decision 0020).
const UNSEATED_CATALOGUE: [&str; 1] = ["muninn"];

/// Every recipe directory with a `bundle.json`, Brokkr's own included.
fn shipped_bundle_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for recipe in std::fs::read_dir(root.join("recipes")).unwrap().flatten() {
        if recipe.path().join("bundle.json").is_file() {
            dirs.push(recipe.path());
        }
    }
    dirs
}

/// #355 (decision 0071 ruling 6): an agent is seated by a shipped
/// recipe, or the catalogue above names it. The seated set is read from
/// each compiled manifest's `agents` pins — the record of which agent
/// every site hired — and a pin that names no agent fails the test
/// rather than being passed over.
#[test]
fn every_library_agent_is_seated_by_a_shipped_bundle_or_catalogued() {
    let root = workspace();
    let mut seated = BTreeSet::new();
    for dir in shipped_bundle_dirs(&root) {
        let name = dir.display();
        let bundle = Bundle::compile_with(&dir, &root.join("agents"), &root.join("adapters"))
            .unwrap_or_else(|error| panic!("{name} compiles: {error}"));
        let Some(pins) = bundle.manifest.get("agents") else {
            continue;
        };
        let pins = pins
            .as_object()
            .unwrap_or_else(|| panic!("{name}: the agents pin is an object"));
        for (site, pin) in pins {
            let agent = pin
                .get("agent")
                .and_then(Value::as_str)
                .unwrap_or_else(|| panic!("{name}:{site}: the pin names its agent"));
            seated.insert(agent.to_string());
        }
    }
    let library = Library::load(&root.join("agents")).expect("the shipped library loads");
    let unseated: Vec<String> = library
        .names()
        .into_iter()
        .filter(|agent| !seated.contains(agent) && !UNSEATED_CATALOGUE.contains(&agent.as_str()))
        .collect();
    assert_eq!(
        unseated,
        Vec::<String>::new(),
        "no shipped recipe seats these agents; seat them, delete them, or catalogue them"
    );
    for agent in UNSEATED_CATALOGUE {
        assert!(
            library.agent(agent).is_some() && !seated.contains(agent),
            "catalogued agent {agent} must exist and be seated by no bundle"
        );
    }
}

#[test]
fn shipped_recipes_have_no_judges_fix_input_and_triage_would_bound_oversized() {
    let root = workspace();
    for dir in shipped_bundle_dirs(&root) {
        let bundle_path = dir.join("bundle.json");
        let bundle = json(&bundle_path);
        let compiled = Bundle::compile_with(&dir, &root.join("agents"), &root.join("adapters"))
            .unwrap_or_else(|error| panic!("{}: {error}", bundle_path.display()));
        if let Some(implement) = compiled.seats.get("implement") {
            assert!(
                implement.results.iter().any(|result| result == "oversized"),
                "{} does not give implementer the oversized verdict",
                bundle_path.display()
            );
        }
        let policy_path = dir.join(bundle["policy"].as_str().unwrap_or("policy.json"));
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

/// Decision 0044 ruling 5: the fetch tools are an explicit grant held by
/// one office. `WebFetch` and `WebSearch` appear on `researcher` and on no
/// other agent, never beside a bindings block, and never at a gate site
/// in a shipped recipe.
#[test]
fn the_fetch_grant_is_held_by_the_researcher_alone_and_never_by_a_gate() {
    let root = workspace();
    const FETCH: [&str; 2] = ["webfetch", "websearch"];
    for entry in std::fs::read_dir(root.join("agents")).unwrap().flatten() {
        if entry.path().extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let agent = json(&entry.path());
        // Decision 0065 rulings 1 and 4: no office NAMES a provider's
        // tool any more — the concrete aliases authorised nothing a realm
        // had ruled, and are refused as a legacy authority path. What the
        // researcher holds instead is a REQUEST, by abstract name, that
        // only a realm can grant; it is still the one office that asks.
        let names_a_tool = agent
            .pointer("/tools/allow")
            .and_then(Value::as_array)
            .is_some_and(|allow| {
                allow
                    .iter()
                    .any(|t| FETCH.contains(&t.as_str().unwrap_or("")))
            });
        assert!(
            !names_a_tool,
            "{}: an office asks for web-search and web-fetch by capability, never by a \
             provider's tool name (decision 0065 ruling 1)",
            entry.path().display()
        );
        let holds = agent
            .get("capabilities")
            .and_then(Value::as_object)
            .is_some_and(|asks| !asks.is_empty());
        let is_researcher = entry.file_name() == "researcher.json";
        assert_eq!(
            holds,
            is_researcher,
            "{}: the fetch request belongs to researcher.json alone (decision 0044 ruling 5)",
            entry.path().display()
        );
        if holds {
            assert_eq!(
                agent["capabilities"],
                serde_json::json!({"web-fetch": "wants", "web-search": "wants"}),
                "the researcher asks, and a realm that grants neither still seats it"
            );
            assert!(
                agent.get("bindings").is_none(),
                "the researcher may not hold secret bindings beside the fetch grant"
            );
        }
    }
    for dir in shipped_bundle_dirs(&root) {
        let bundle = dir.join("bundle.json");
        let mut path = Vec::new();
        walk(&json(&bundle), &mut path, &mut |site, value| {
            let Some(object) = value.as_object() else {
                return;
            };
            if object.get("class").and_then(Value::as_str) != Some("gate") {
                return;
            }
            let site = format!("{}:{}", bundle.display(), site.join("/"));
            assert_ne!(
                object.get("agent").and_then(Value::as_str),
                Some("researcher"),
                "{site}: a gate site seats the researcher, which holds the fetch grant"
            );
            let inline: Vec<&str> = value
                .pointer("/driver/command")
                .and_then(Value::as_array)
                .map(|c| c.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            assert!(
                !inline
                    .iter()
                    .any(|arg| ["WebFetch", "WebSearch"].iter().any(|f| arg.contains(f))),
                "{site}: a gate site holds a fetch tool inline"
            );
        });
    }
}

/// Decision 0044 ruling 5 and its erratum of 2026-09-04, the dsh shape: the
/// `--patch` overlay on a dsh site is the pinned model's ROUTE overlay, and
/// it appears in `research-dsh` alone; the fetch grant is the composed
/// `headless` profile's own and enters the composite through its
/// `profile-bundle` lines. That recipe's role file is the library charter's
/// bytes, so the configurable prompt stays one text.
#[test]
fn the_dsh_fetch_overlay_is_the_research_lanes_alone_and_its_role_is_the_charter() {
    let root = workspace();
    for dir in shipped_bundle_dirs(&root) {
        let bundle = dir.join("bundle.json");
        let name = dir.file_name().unwrap().to_string_lossy().into_owned();
        walk(&json(&bundle), &mut Vec::new(), &mut |site, value| {
            let Some(command) = value
                .get("driver")
                .and_then(|d| d.get("command"))
                .and_then(Value::as_array)
            else {
                return;
            };
            let patched = command.iter().any(|arg| arg.as_str() == Some("--patch"));
            assert!(
                !patched || name == "research-dsh",
                "{name}:{} carries a dsh overlay; only research-dsh may (decision 0044 ruling 5)",
                site.join("/")
            );
        });
    }
    let charter = std::fs::read(root.join("agents/charters/researcher.md")).unwrap();
    let role = std::fs::read(root.join("recipes/research-dsh/roles/researcher.md")).unwrap();
    assert_eq!(
        charter, role,
        "recipes/research-dsh/roles/researcher.md is a copy of agents/charters/researcher.md"
    );
}

/// Each shipped adapter declares its resume assessment under exactly the
/// shape name its driver's gate and `brokkr doctor` read as a Rust
/// literal (`CODEX_SHAPE`, `CLAUDE_SHAPE`, `DSH_SHAPE`,
/// `LANETALLY_SHAPE`, and doctor's own `DSH_SHAPE`). The literals live in
/// two crates and the names in four JSON files; a spelling that drifted
/// would not fail to load, it would silently read as an absent shape —
/// `unsupported-resume` at the gate and "nothing declared" at the doctor
/// line — so the agreement is pinned here, against the files.
#[test]
fn every_shipped_adapter_declares_the_shape_its_gate_and_doctor_read() {
    let root = workspace();
    for (adapter, shape) in [
        ("claude", "boxed-workspace"),
        ("codex", "work-site"),
        ("dsh", "headless-work"),
        ("lanetally", "wrapper-work-site"),
    ] {
        let declared = json(&root.join(format!("adapters/{adapter}.json")));
        let shapes: Vec<&String> = declared["resume"]
            .as_object()
            .unwrap_or_else(|| panic!("adapters/{adapter}.json declares a resume map"))
            .keys()
            .collect();
        assert_eq!(
            shapes,
            [shape],
            "adapters/{adapter}.json declares exactly the shape its gate reads"
        );
    }
}

/// Recipe-local roles stand outside the portability walk above, but they
/// are what an inline seat reads in place of a charter (issue #334). They
/// defer to the house rules instead of restating them, and they carry the
/// same principle text their charter does, so a recipe's seats and the
/// library's offices cannot drift apart. The two reviewer roles that do
/// not specialise are the reviewer charter's bytes and are held to them;
/// fast's, the third, became the charter itself when fast moved onto the
/// library (#360).
#[test]
fn recipe_roles_defer_to_the_house_and_carry_their_charters_principles() {
    let root = workspace();
    let flatten = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    let implementer = std::fs::read_to_string(root.join("agents/charters/implementer.md")).unwrap();
    let design = implementer
        .split("\n\n")
        .find(|paragraph| paragraph.starts_with("Design: "))
        .expect("the implementer charter states its design paragraph");
    let reviewer =
        flatten(&std::fs::read_to_string(root.join("agents/charters/reviewer.md")).unwrap());
    let principles = reviewer
        .split_once("against the architecture principles")
        .and_then(|(_, tail)| tail.split_once("severity table."))
        .map(|(middle, _)| format!("against the architecture principles{middle}severity table."))
        .expect("the reviewer charter states its principles sentence");
    let defer = "The house rules that follow this role, when the run carries them, state the \
                 repository's conventions, frozen surfaces, gates and architecture. Follow \
                 them; this role does not repeat them.";
    let restated = [
        "Rules of the house",
        "policy/schemas",
        "Never push",
        "cargo test --workspace",
    ];
    let (mut implementers, mut reviewers) = (0, 0);
    for recipe in std::fs::read_dir(root.join("recipes")).unwrap().flatten() {
        for name in ["implementer.md", "reviewer.md"] {
            let path = recipe.path().join("roles").join(name);
            let text = match std::fs::read_to_string(&path) {
                Ok(text) => text,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => panic!("{}: {error}", path.display()),
            };
            for phrase in restated {
                assert!(
                    !text.contains(phrase),
                    "{} restates the house: {phrase:?}",
                    path.display()
                );
            }
            let flat = flatten(&text);
            if name == "implementer.md" {
                implementers += 1;
                assert!(
                    text.contains(design),
                    "{} lost the implementer charter's design paragraph",
                    path.display()
                );
                assert!(
                    flat.contains(defer),
                    "{} does not defer to the house",
                    path.display()
                );
            } else {
                reviewers += 1;
                assert!(
                    flat.contains(&principles),
                    "{} does not judge against the house's principles",
                    path.display()
                );
            }
        }
    }
    assert!(implementers > 0 && reviewers > 0, "the walk found no roles");
    let charter = std::fs::read(root.join("agents/charters/reviewer.md")).unwrap();
    for recipe in ["review-first", "standby"] {
        let role = format!("recipes/{recipe}/roles/reviewer.md");
        assert_eq!(
            charter,
            std::fs::read(root.join(&role)).unwrap(),
            "{role} is a copy of agents/charters/reviewer.md"
        );
    }
}
