//! Deterministic budgets for what every seat is handed and what the build
//! pulls in (#342). A seat's prompt is rendered exactly as its driver
//! renders it, and its bytes are held at or under a committed budget; its
//! o200k tokens are printed as a report, never gated. The package count of
//! `Cargo.lock` is held the same way.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use brokkr_core::realms::Boundary;
use brokkr_protocol::adapters::{render_prompt, AdapterKind};
use brokkr_runtime::bundle::{SeatBody, StepBody};
use brokkr_runtime::dialect::Dialect;
use brokkr_runtime::Bundle;
use serde_json::{json, Value};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// One model-facing site: its label, the text that stands for its office,
/// and the results its contract allows.
struct Site {
    label: String,
    role_path: PathBuf,
    results: Vec<String>,
}

fn site(sites: &mut Vec<Site>, label: String, role_path: &Path, results: &[String]) {
    // An exec site's charter compiles to an empty path: a script reads no
    // prose, so it has no prompt to budget (see `render_prompt`).
    if !role_path.as_os_str().is_empty() {
        sites.push(Site {
            label,
            role_path: role_path.to_path_buf(),
            results: results.to_vec(),
        });
    }
}

/// Every model-facing site of one seat body, labelled as the manifest keys
/// hands: `seat`, `seat:member`, `seat:step` and `seat:step:member`, with
/// a strategy case in brackets.
fn walk(sites: &mut Vec<Site>, label: &str, body: &SeatBody, results: &[String]) {
    match body {
        SeatBody::Single { role_path, .. } => site(sites, label.to_string(), role_path, results),
        SeatBody::Panel { members, .. } => {
            for member in members {
                let at = format!("{label}:{}", member.name);
                site(sites, at, &member.role_path, results);
            }
        }
        SeatBody::Sequence { steps } => {
            for step in steps {
                let at = format!("{label}:{}", step.name);
                match &step.body {
                    StepBody::Single { role_path, .. } => site(sites, at, role_path, &step.results),
                    StepBody::Panel { members, .. } => {
                        for member in members {
                            let at = format!("{at}:{}", member.name);
                            site(sites, at, &member.role_path, &step.results);
                        }
                    }
                    // A dialect step is a validator the engine runs itself.
                    StepBody::Dialect { .. } => {}
                }
            }
        }
        SeatBody::Select { cases, default, .. } => {
            for (strategy, case) in cases {
                walk(sites, &format!("{label}[{strategy}]"), case, results);
            }
            if let Some(case) = default {
                walk(sites, &format!("{label}[default]"), case, results);
            }
        }
    }
}

/// Every shipped recipe and bundle, compiled in the self realm as
/// `realms.json` declares it: the house the realm reads and the openspec
/// dialect.
fn shipped() -> Vec<(String, Bundle)> {
    let root = root();
    let dialect = Dialect::load(&root.join("dialects/openspec.json"))
        .expect("the openspec dialect loads")
        .0;
    let mut bundles = Vec::new();
    for library in ["recipes", "bundles"] {
        let mut dirs: Vec<PathBuf> = std::fs::read_dir(root.join(library))
            .expect("the library directory reads")
            .map(|entry| entry.expect("a library entry reads").path())
            .filter(|dir| dir.join("bundle.json").is_file())
            .collect();
        dirs.sort();
        for dir in dirs {
            let name = format!("{library}/{}", dir.file_name().unwrap().to_string_lossy());
            let bundle = Bundle::compile_with_realm(
                &dir,
                &root.join("agents"),
                &root.join("adapters"),
                Some("brokkr"),
                Some(&dialect),
                Boundary::Namespace,
            )
            .unwrap_or_else(|error| panic!("{name} compiles in the self realm: {error}"));
            bundles.push((name, bundle));
        }
    }
    bundles
}

/// The input the engine composes for one model site: the office's text,
/// the realm's house rules and, outside review, the dialect's instructions
/// for the phase, the largest input the engine composes for that site.
fn seat_input(site: &Site, phase: &str, house: &str, dialect: Option<&String>) -> Value {
    let mut input = json!({
        "role_path": site.role_path.to_string_lossy(),
        "feature": "the feature under delivery",
        "phase": phase,
        "workdir": "/repo",
        "allowed_results": site.results,
        "context": {},
        "result_path": "/repo/.forge/results/effect.json",
        "house_rules": house,
    });
    if let Some(dialect) = dialect.filter(|_| phase != "review") {
        input["spec_dialect"] = json!(dialect);
    }
    input
}

/// The prompt every model site is handed, rendered by the driver's own
/// function over the input the engine composes (see [`seat_input`]).
fn prompts() -> BTreeMap<String, String> {
    let house =
        std::fs::read_to_string(root().join("docs/house-rules.md")).expect("the house reads");
    let mut prompts = BTreeMap::new();
    for (name, bundle) in shipped() {
        for (phase, seat) in &bundle.seats {
            let mut sites = Vec::new();
            walk(&mut sites, phase, &seat.body, &seat.results);
            for site in sites {
                let input = seat_input(&site, phase, &house, bundle.dialect_prompts.get(phase));
                let prompt = render_prompt(&input, AdapterKind::Claude)
                    .unwrap_or_else(|error| panic!("{name}/{}: {error}", site.label));
                prompts.insert(format!("{name}/{}", site.label), prompt);
            }
        }
    }
    prompts
}

fn committed(file: &str) -> Value {
    let path = root().join("quality").join(file);
    let text = std::fs::read_to_string(&path).expect("the committed budget reads");
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// Refusals of a measured map against its committed budgets, one line
/// each: a site over its budget, a site with no budget, and a budget whose
/// site is gone. A budget is edited only by the pull request that moves it.
fn refusals(
    measured: &BTreeMap<String, u64>,
    budgets: &serde_json::Map<String, Value>,
) -> Vec<String> {
    let mut refusals = Vec::new();
    for (key, bytes) in measured {
        match budgets.get(key).map(Value::as_u64) {
            Some(Some(budget)) if *bytes <= budget => {}
            Some(Some(budget)) => refusals.push(format!(
                "{key}: {bytes} bytes exceeds its budget of {budget}"
            )),
            Some(None) => {
                refusals.push(format!("{key}: the budget is not a whole number of bytes"))
            }
            None => refusals.push(format!("{key}: {bytes} bytes has no committed budget")),
        }
    }
    for key in budgets.keys().filter(|key| !measured.contains_key(*key)) {
        refusals.push(format!("{key}: a budget with no site"));
    }
    refusals
}

#[test]
fn every_seat_prompt_stays_within_its_committed_byte_budget() {
    let prompts = prompts();
    assert!(
        !prompts.is_empty(),
        "the shipped libraries hold model sites"
    );
    let tokenizer = tiktoken_rs::o200k_base().expect("o200k_base loads");
    let mut measured = BTreeMap::new();
    for (key, prompt) in &prompts {
        let tokens = tokenizer.encode_with_special_tokens(prompt).len();
        println!("{key}\t{} bytes\t{tokens} o200k tokens", prompt.len());
        measured.insert(key.clone(), prompt.len() as u64);
    }
    let file = committed("prompt-bytes.json");
    let budgets = file["budgets"]
        .as_object()
        .expect("prompt-bytes.json holds a budgets object");
    let refusals = refusals(&measured, budgets);
    assert!(
        refusals.is_empty(),
        "prompt byte budgets:\n{}",
        refusals.join("\n")
    );
}

/// The package count `Cargo.lock` pins, read as the lockfile's own grammar:
/// one `[[package]]` table per package. Any other table header is refused
/// rather than skipped, so a format this reader does not know cannot be
/// counted as if it did.
fn packages(lock: &str) -> Result<u64, String> {
    let mut packages = 0;
    for line in lock.lines().filter(|line| line.starts_with('[')) {
        match line {
            "[[package]]" => packages += 1,
            other => {
                return Err(format!(
                    "Cargo.lock holds a table this count does not know: {other}"
                ))
            }
        }
    }
    Ok(packages)
}

#[test]
fn the_lockfile_holds_no_more_packages_than_its_committed_count() {
    let lock = std::fs::read_to_string(root().join("Cargo.lock")).expect("Cargo.lock reads");
    let count = packages(&lock).unwrap_or_else(|refusal| panic!("{refusal}"));
    println!("packages\t{count}");
    let file = committed("crate-count.json");
    let budget = file["budgets"]["packages"]
        .as_u64()
        .expect("crate-count.json holds a budgets.packages number");
    assert!(
        count <= budget,
        "Cargo.lock holds {count} packages; the committed count is {budget}. A new \
         dependency moves the count by the pull request that adds it."
    );
}

#[test]
fn the_package_count_refuses_a_table_it_does_not_know() {
    assert_eq!(
        packages("version = 4\n\n[[package]]\nname = \"a\"\n\n[[package]]\n"),
        Ok(2)
    );
    assert_eq!(
        packages("[[package]]\n[metadata]\n"),
        Err("Cargo.lock holds a table this count does not know: [metadata]".to_string())
    );
}

#[test]
fn a_budget_refuses_growth_a_missing_budget_and_a_stale_one() {
    let measured = BTreeMap::from([("a".to_string(), 10), ("b".to_string(), 5)]);
    let budgets = json!({"a": 9, "c": 1});
    assert_eq!(
        refusals(&measured, budgets.as_object().unwrap()),
        vec![
            "a: 10 bytes exceeds its budget of 9",
            "b: 5 bytes has no committed budget",
            "c: a budget with no site",
        ]
    );
    let exact = json!({"a": 10, "b": "5"});
    assert_eq!(
        refusals(&measured, exact.as_object().unwrap()),
        vec!["b: the budget is not a whole number of bytes"]
    );
}
