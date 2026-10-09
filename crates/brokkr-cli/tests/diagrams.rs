//! Decision 0037's standing check: a diagram on a living surface is
//! mermaid text beside its prose, and a diagram of a structure the
//! machine owns says exactly what the data says.
//!
//! Five diagrams in `ARCHITECTURE.md` depict machine-owned structures
//! today, and each is parsed here and held against its data: the crate
//! graph against the workspace manifests, the `fast` recipe's phase
//! graph against `recipes/fast/policy.json`, the effect lifecycle
//! against core's `EventType` and the envelope contracts, the driver
//! sequence against the `forge-driver` contracts, and the policy inputs
//! against core's engine-owned inputs. A diagram of a structure that
//! gains a check later is added to this file.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use brokkr_core::policy::{ENGINE_OWNED_INPUTS, VISIT_PREFIX};
use brokkr_core::EventType;
use serde::de::value::StrDeserializer;
use serde::de::{DeserializeOwned, IntoDeserializer};
use serde::Deserialize;
use serde_json::Value;

use crate::layering::shipped_workspace_edges;
use crate::workspace_root::{read, workspace};

/// The mermaid blocks of a Markdown page, in order, fence lines removed.
fn mermaid_blocks(markdown: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Option<String> = None;
    for line in markdown.lines() {
        match (&mut current, line.trim_end()) {
            (None, "```mermaid") => current = Some(String::new()),
            (Some(block), "```") => blocks.push(std::mem::take(block)),
            (Some(block), text) => {
                block.push_str(text);
                block.push('\n');
            }
            (None, _) => {}
        }
        if line.trim_end() == "```" {
            current = None;
        }
    }
    blocks
}

/// The identifier a mermaid node reference starts with: `cli["…"]`,
/// `store[("…")]`, `operator([…])` and bare `core` all name their id
/// before the first shape character.
fn node_id(reference: &str) -> String {
    reference
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect()
}

/// The `(from, to)` pairs a flowchart draws. Labelled forms
/// (`-- "x" -->`, `-. "x" .->`) and fan-outs (`a --> b & c`) are read;
/// everything without an arrow is a node definition or scaffolding.
fn flowchart_edges(block: &str) -> Vec<(String, String)> {
    let mut edges = Vec::new();
    for line in block.lines() {
        let line = line.trim();
        let Some((arrow_at, arrow)) = ["-->", ".->", "==>"]
            .iter()
            .filter_map(|arrow| line.rfind(arrow).map(|at| (at, *arrow)))
            .max_by_key(|(at, _)| *at)
        else {
            continue;
        };
        let right = &line[arrow_at + arrow.len()..];
        let left_end = [" --", " -.", " =="]
            .iter()
            .filter_map(|opener| line.find(opener))
            .min()
            .expect("an arrow line has an opener");
        let left = &line[..left_end];
        for from in left.split('&') {
            for to in right.split('&') {
                edges.push((node_id(from), node_id(to)));
            }
        }
    }
    edges
}

/// The `(from, to)` pairs a state diagram draws, the pseudo-state
/// `[*]` left out, transition labels ignored.
fn state_edges(block: &str) -> BTreeSet<(String, String)> {
    block
        .lines()
        .filter_map(|line| line.trim().split_once(" --> "))
        .map(|(from, to)| {
            let to = to.split(':').next().unwrap_or(to).trim();
            (from.trim().to_string(), to.to_string())
        })
        .filter(|(from, to)| from != "[*]" && to != "[*]")
        .collect()
}

fn workspace_crates() -> BTreeSet<String> {
    std::fs::read_dir(workspace().join("crates"))
        .expect("crates/")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().join("Cargo.toml").is_file())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect()
}

/// Ruling 3, the crate graph: every drawn edge between two crates is
/// a declared dependency, every declared dependency between two
/// workspace crates is drawn (#364), and every workspace crate is drawn.
/// The declared edges are Cargo's own reading of every manifest table,
/// dev edges aside, which a diagram of the shipped binary does not draw.
#[test]
fn the_crate_diagram_draws_real_dependencies_and_every_crate() {
    let architecture = read("ARCHITECTURE.md");
    let blocks = mermaid_blocks(&architecture);
    let shape = blocks
        .iter()
        .find(|block| block.contains("brokkr-cli"))
        .expect("ARCHITECTURE.md draws the crate graph");

    let crates = workspace_crates();
    let crate_of = |id: &str| {
        let name = format!("brokkr-{id}");
        crates.contains(&name).then_some(name)
    };

    let drawn: BTreeSet<String> = shape
        .lines()
        .map(|line| node_id(line.trim()))
        .chain(
            flowchart_edges(shape)
                .into_iter()
                .flat_map(|(from, to)| [from, to]),
        )
        .filter_map(|id| crate_of(&id))
        .collect();
    assert_eq!(drawn, crates, "the crate diagram must draw every crate");

    let declared = shipped_workspace_edges();
    let edges: BTreeSet<(String, String)> = flowchart_edges(shape)
        .into_iter()
        .filter_map(|(from, to)| Some((crate_of(&from)?, crate_of(&to)?)))
        .collect();
    for (from, to) in &edges {
        assert!(
            declared.contains(&(from.clone(), to.clone())),
            "the crate diagram draws {from} → {to}, which Cargo.toml does not declare"
        );
    }
    assert!(
        edges.len() >= 6,
        "the crate diagram draws too few edges to be a graph: {}",
        edges.len()
    );
    for (from, to) in &declared {
        assert!(
            edges.contains(&(from.clone(), to.clone())),
            "Cargo.toml declares {from} → {to}, which the crate diagram does not draw"
        );
    }
}

/// The first mermaid block of `ARCHITECTURE.md` that `is_it` picks out.
fn architecture_block(what: &str, is_it: impl Fn(&str) -> bool) -> String {
    mermaid_blocks(&read("ARCHITECTURE.md"))
        .into_iter()
        .find(|block| is_it(block))
        .unwrap_or_else(|| panic!("ARCHITECTURE.md draws {what}"))
}

/// The lines of a mermaid block that draw something: Mermaid renders no
/// `%%` comment, so a transition or message commented out is undrawn.
fn drawn_lines(block: &str) -> impl Iterator<Item = &str> {
    block
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("%%"))
}

/// Whether `text` is one mermaid participant or state identifier.
fn is_identifier(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The contract files of one family, every version, each read into the
/// projection `T` its check needs: `contracts/<stem>.v<n>.schema.json`.
fn contract_versions<T: DeserializeOwned>(stem: &str) -> Vec<T> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(workspace().join("contracts"))
        .expect("contracts/")
        .map(|entry| entry.expect("entry").path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| name.strip_prefix(stem))
                .and_then(|rest| rest.strip_prefix(".v"))
                .and_then(|rest| rest.strip_suffix(".schema.json"))
                .is_some_and(|version| version.parse::<u32>().is_ok())
        })
        .collect();
    found.sort();
    assert!(!found.is_empty(), "contracts/ holds no {stem} version");
    found
        .iter()
        .map(|path| {
            serde_json::from_str(&std::fs::read_to_string(path).unwrap())
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
        })
        .collect()
}

/// The envelope contract's `properties.type.enum`, the wire names of its
/// event types.
#[derive(Deserialize)]
struct EnvelopeContract {
    properties: EnvelopeProperties,
}

#[derive(Deserialize)]
struct EnvelopeProperties {
    #[serde(rename = "type")]
    event_type: EventTypeNames,
}

#[derive(Deserialize)]
struct EventTypeNames {
    #[serde(rename = "enum")]
    wire: Vec<String>,
}

/// One effect-lifecycle line: a transition `from --> to` with an optional
/// `: label`, or the diagram's own scaffolding. Anything else is a line
/// this check cannot read, and it is refused rather than skipped.
fn lifecycle_label(line: &str) -> Option<&str> {
    if line == "stateDiagram-v2" || line.starts_with("direction ") {
        return None;
    }
    let (from, rest) = line.split_once(" --> ").unwrap_or_else(|| {
        panic!("the effect lifecycle has a line this check cannot read: {line}")
    });
    let (to, label) = rest.split_once(':').unwrap_or((rest, ""));
    let is_state = |state: &str| state == "[*]" || is_identifier(state);
    assert!(
        is_state(from.trim()) && is_state(to.trim()),
        "the effect lifecycle has a transition this check cannot read: {line}"
    );
    Some(label)
}

/// Whether the effect lifecycle draws an event, by its wire name, on a
/// transition. No wildcard arm: a new `EventType` fails to compile here
/// until it is placed, and an `effect/` event placed off the drawing
/// fails the test below.
fn drawn_on_the_effect_lifecycle(event: EventType) -> bool {
    match event {
        EventType::EffectRequested
        | EventType::EffectStarted
        | EventType::EffectCheckpointed
        | EventType::EffectSucceeded
        | EventType::EffectFailed
        | EventType::EffectIndeterminate
        | EventType::RunParked => true,
        // The run's own facts: its start, phases, rulings, the operator's
        // commands and its endings other than a park.
        EventType::RunStarted
        | EventType::PhaseEntered
        | EventType::TransitionDecided
        | EventType::OperatorCommanded
        | EventType::OperatorAccepted
        | EventType::OperatorRejected
        | EventType::RunCompleted
        | EventType::RunStopped => false,
    }
}

/// Ruling 3, the effect lifecycle (#364): the event names its transitions
/// carry are exactly the event types the lifecycle draws, read from every
/// envelope contract version and parsed into core's `EventType`, and every
/// `effect/` event is among them.
#[test]
fn the_effect_lifecycle_draws_every_effect_event() {
    let lifecycle = architecture_block("the effect lifecycle", |block| {
        block.starts_with("stateDiagram") && block.contains("effect/requested")
    });
    let drawn: BTreeSet<String> = drawn_lines(&lifecycle)
        .filter_map(lifecycle_label)
        .flat_map(str::split_whitespace)
        .filter(|word| word.contains('/'))
        .map(str::to_string)
        .collect();

    let mut expected = BTreeSet::new();
    for contract in contract_versions::<EnvelopeContract>("event-envelope") {
        for wire in &contract.properties.event_type.wire {
            let name: StrDeserializer<'_, serde::de::value::Error> =
                wire.as_str().into_deserializer();
            let event = EventType::deserialize(name)
                .unwrap_or_else(|error| panic!("core does not know {wire}: {error}"));
            let on_it = drawn_on_the_effect_lifecycle(event);
            assert!(
                on_it || !wire.starts_with("effect/"),
                "{wire} is an effect event, and the effect lifecycle must draw it"
            );
            if on_it {
                expected.insert(wire.clone());
            }
        }
    }
    assert_eq!(
        drawn, expected,
        "the effect lifecycle in ARCHITECTURE.md drifted from EventType"
    );
}

/// The driver contract's message family (`oneOf`) and the definitions
/// its members point at; a shared definition such as `base` names no type.
#[derive(Deserialize)]
struct DriverContract {
    #[serde(rename = "oneOf")]
    family: Vec<FamilyMember>,
    definitions: BTreeMap<String, Definition>,
}

#[derive(Deserialize)]
struct FamilyMember {
    #[serde(rename = "$ref")]
    reference: String,
}

#[derive(Deserialize)]
struct Definition {
    #[serde(default)]
    properties: DefinitionProperties,
}

#[derive(Deserialize, Default)]
struct DefinitionProperties {
    #[serde(rename = "type")]
    message_type: Option<MessageType>,
}

#[derive(Deserialize)]
struct MessageType {
    #[serde(rename = "const")]
    name: String,
}

/// The message types every `forge-driver` contract version defines.
fn protocol_messages() -> BTreeSet<String> {
    let mut messages = BTreeSet::new();
    for contract in contract_versions::<DriverContract>("driver-protocol") {
        for member in &contract.family {
            let reference = &member.reference;
            let message = reference
                .strip_prefix("#/definitions/")
                .and_then(|name| contract.definitions.get(name))
                .and_then(|definition| definition.properties.message_type.as_ref())
                .unwrap_or_else(|| panic!("{reference} names no message type"));
            messages.insert(message.name.clone());
        }
    }
    messages
}

/// The first word of a message the driver sequence draws between the
/// engine and the driver, `None` for any other recognised line. A message
/// line is `from ->> to: text` or `from -->> to: text`; scaffolding is
/// `sequenceDiagram`, `participant`, `opt`, `loop`, `end` or `Note over`.
/// Anything else, another arrow form included, is refused rather than
/// skipped.
fn driver_message(line: &str) -> Option<String> {
    let scaffolding = ["participant ", "opt ", "loop ", "Note over "];
    if line == "sequenceDiagram"
        || line == "end"
        || scaffolding.iter().any(|keyword| line.starts_with(keyword))
    {
        return None;
    }
    let unreadable = || panic!("the driver sequence has a line this check cannot read: {line}");
    let (arrow, text) = line.split_once(':').unwrap_or_else(unreadable);
    let (from, to) = arrow
        .split_once("-->>")
        .or_else(|| arrow.split_once("->>"))
        .unwrap_or_else(unreadable);
    let ends = [from.trim(), to.trim()];
    assert!(
        ends.iter().all(|end| is_identifier(end)),
        "the driver sequence has a line this check cannot read: {line}"
    );
    let first = text.split_whitespace().next().unwrap_or_default();
    (ends == ["E", "D"] || ends == ["D", "E"]).then(|| first.to_string())
}

/// Ruling 3, the driver sequence (#364): the messages drawn between the
/// engine and the driver are exactly the protocol's message family.
#[test]
fn the_driver_sequence_draws_every_protocol_message() {
    let sequence = architecture_block("the driver sequence", |block| {
        block.starts_with("sequenceDiagram")
    });
    let drawn: BTreeSet<String> = drawn_lines(&sequence).filter_map(driver_message).collect();
    assert_eq!(
        drawn,
        protocol_messages(),
        "the driver sequence in ARCHITECTURE.md drifted from driver-protocol"
    );
}

/// Ruling 3, the policy inputs (#364): the inputs the diagram says the
/// engine owns are exactly core's engine-owned inputs and the visit family.
#[test]
fn the_policy_diagram_names_every_engine_owned_input() {
    let policy = architecture_block("the policy inputs", |block| {
        block.contains("engine-owned inputs")
    });
    let label = policy
        .lines()
        .find_map(|line| line.trim().strip_prefix("engine[\"")?.split_once("\"]"))
        .map(|(label, _)| label)
        .expect("the policy diagram draws the engine's inputs");
    let drawn: BTreeSet<String> = label
        .split("<br/>")
        .skip(1)
        .flat_map(|row| row.split(" · "))
        .map(|input| input.trim().to_string())
        .collect();
    let expected: BTreeSet<String> = ENGINE_OWNED_INPUTS
        .iter()
        .map(|input| input.to_string())
        .chain(std::iter::once(format!("{VISIT_PREFIX}‹phase›")))
        .collect();
    assert_eq!(
        drawn, expected,
        "the policy diagram in ARCHITECTURE.md drifted from ENGINE_OWNED_INPUTS"
    );
}

/// Ruling 3, the phase graph: the state diagram's edges are exactly the
/// `(from → next | park)` pairs of the `fast` recipe's table, and its
/// states are exactly the table's phases plus the park.
#[test]
fn the_phase_diagram_is_the_fast_table() {
    let architecture = read("ARCHITECTURE.md");
    let blocks = mermaid_blocks(&architecture);
    let graph = blocks
        .iter()
        .find(|block| block.starts_with("stateDiagram") && block.contains("[*] --> implement"))
        .expect("ARCHITECTURE.md draws the fast table");
    let drawn = state_edges(graph);

    let policy: Value = serde_json::from_str(&read("recipes/fast/policy.json")).unwrap();
    let expected: BTreeSet<(String, String)> = policy["rules"]
        .as_array()
        .expect("rules")
        .iter()
        .map(|rule| {
            let from = rule["from"].as_str().expect("from").to_string();
            let to = match (rule.get("next"), rule.get("park")) {
                (Some(next), _) => next.as_str().expect("next").to_string(),
                (None, Some(Value::Bool(true))) => "parked".to_string(),
                _ => panic!("rule {} neither advances nor parks", rule["id"]),
            };
            (from, to)
        })
        .collect();
    assert_eq!(
        drawn, expected,
        "the phase diagram in ARCHITECTURE.md drifted from recipes/fast/policy.json"
    );

    let phases: BTreeSet<String> = policy["phases"]
        .as_array()
        .expect("phases")
        .iter()
        .map(|phase| phase.as_str().expect("phase").to_string())
        .chain(std::iter::once("parked".to_string()))
        .collect();
    let states: BTreeSet<String> = drawn
        .iter()
        .flat_map(|(from, to)| [from.clone(), to.clone()])
        .collect();
    assert_eq!(
        states, phases,
        "the phase diagram's states are the table's phases"
    );
}

/// Ruling 2's determinable edge: the architecture page stays a page of
/// pictures with prose around them, and the front page carries one. Its
/// word cap went with its verb list (#364): the verbs live in the
/// generated CLI reference, and the page is the as-built map.
#[test]
fn the_architecture_page_is_pictures_first() {
    let architecture = read("ARCHITECTURE.md");
    assert!(
        mermaid_blocks(&architecture).len() >= 6,
        "ARCHITECTURE.md lost its diagrams"
    );
    assert!(
        !mermaid_blocks(&read("README.md")).is_empty(),
        "the front page lost its bootstrap picture"
    );
}

fn markdown_files(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|error| panic!("{}: {error}", dir.display()))
    {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            markdown_files(&path, found);
        } else if path.extension().is_some_and(|ext| ext == "md") {
            found.push(path);
        }
    }
}

/// Rulings 1 and 4: no repository-hosted image outside `assets/` is
/// embedded on a living surface — a picture's source is mermaid text,
/// and nothing rendered is committed.
#[test]
fn living_surfaces_embed_no_rendered_pictures() {
    let root = workspace();
    let mut pages = vec![
        root.join("README.md"),
        root.join("ARCHITECTURE.md"),
        root.join("CONTRIBUTING.md"),
    ];
    markdown_files(&root.join("docs"), &mut pages);

    let mut offenses = Vec::new();
    for page in pages {
        let contents = std::fs::read_to_string(&page).unwrap();
        for (index, line) in contents.lines().enumerate() {
            let targets =
                line.match_indices("![")
                    .filter_map(|(at, _)| line[at..].split_once("](").map(|(_, rest)| rest))
                    .chain(line.match_indices("<img ").filter_map(|(at, _)| {
                        line[at..].split_once("src=\"").map(|(_, rest)| rest)
                    }))
                    .map(|rest| {
                        rest.split([')', '"', ' '])
                            .next()
                            .unwrap_or_default()
                            .to_string()
                    });
            for target in targets {
                let external = target.starts_with("http://") || target.starts_with("https://");
                let brand = target.trim_start_matches("../").starts_with("assets/");
                if !external && !brand {
                    offenses.push(format!(
                        "{}:{}: {target}",
                        page.strip_prefix(&root).unwrap().display(),
                        index + 1
                    ));
                }
            }
        }
    }
    assert!(
        offenses.is_empty(),
        "rendered pictures embedded on living surfaces (decision 0037):\n{}",
        offenses.join("\n")
    );
}
