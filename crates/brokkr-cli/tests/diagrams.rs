//! Decision 0037's standing check: a diagram on a living surface is
//! mermaid text beside its prose, and a diagram of a structure the
//! machine owns says exactly what the data says.
//!
//! Two diagrams in `ARCHITECTURE.md` depict machine-owned structures
//! today — the crate graph and the `fast` recipe's phase graph — and
//! both are parsed here and held against the workspace manifests and
//! `recipes/fast/policy.json`. A diagram of a structure that gains a
//! check later is added to this file.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::Value;

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = workspace().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

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

/// The `[dependencies]` names of one crate — not dev-dependencies,
/// which a diagram of the shipped binary does not draw.
fn dependencies(krate: &str) -> BTreeSet<String> {
    let manifest = read(&format!("crates/{krate}/Cargo.toml"));
    let section = manifest
        .split("\n[dependencies]\n")
        .nth(1)
        .unwrap_or_else(|| panic!("{krate} has a [dependencies] section"));
    section
        .split("\n[")
        .next()
        .expect("the section ends")
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(name, _)| name.trim().to_string())
        .filter(|name| !name.is_empty() && !name.starts_with('#'))
        .collect()
}

/// Ruling 3, the crate graph: every drawn edge between two crates is
/// a declared dependency, and every workspace crate is drawn.
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

    let mut checked = 0;
    for (from, to) in flowchart_edges(shape) {
        let (Some(from), Some(to)) = (crate_of(&from), crate_of(&to)) else {
            continue;
        };
        assert!(
            dependencies(&from).contains(&to),
            "the crate diagram draws {from} → {to}, which Cargo.toml does not declare"
        );
        checked += 1;
    }
    assert!(
        checked >= 6,
        "the crate diagram draws too few edges to be a graph: {checked}"
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
/// pictures with prose around them, and the front page carries one.
#[test]
fn the_architecture_page_is_pictures_first_and_under_budget() {
    let architecture = read("ARCHITECTURE.md");
    let words = architecture.split_whitespace().count();
    assert!(
        words < 2_000,
        "ARCHITECTURE.md grew past its word budget: {words}"
    );
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
