//! Deterministic budgets for what every seat is handed and what the build
//! pulls in (#342). A seat's prompt is rendered exactly as its driver
//! renders it, and its bytes are held at or under a committed budget; its
//! o200k tokens are printed as a report, never gated. The package count of
//! `Cargo.lock` is held the same way.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use brokkr_core::realms::Boundary;
use brokkr_protocol::adapters::{render_prompt, AdapterKind};
use brokkr_runtime::bundle::{PanelMember, SeatBody, StepBody};
use brokkr_runtime::dialect::Dialect;
use brokkr_runtime::engine::SiteMarks;
use brokkr_runtime::{Bundle, Candidate, Seat, SeatClass};
use serde_json::{json, Value};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// One model-facing site: its budget key, the label the engine marks it
/// by, the text that stands for its office, the results its contract
/// allows, whether it is a gate, the panel member it is (which decides
/// its dialect prose), and the links that may serve it.
struct Site {
    key: String,
    at: String,
    role_path: PathBuf,
    results: Vec<String>,
    gate: bool,
    member: Option<String>,
    links: Vec<Candidate>,
}

/// Where one body sits: its budget key, its engine label, the results its
/// final site allows and whether it is a gate.
struct At<'a> {
    key: String,
    at: String,
    results: &'a [String],
    gate: bool,
}

impl At<'_> {
    fn site(&self, role_path: &Path, member: Option<&str>, links: &[Candidate]) -> Option<Site> {
        // An exec site's charter compiles to an empty path: a script reads
        // no prose, so it has no prompt to budget (see `render_prompt`).
        (!role_path.as_os_str().is_empty()).then(|| Site {
            key: self.key.clone(),
            at: self.at.clone(),
            role_path: role_path.to_path_buf(),
            results: self.results.to_vec(),
            gate: self.gate,
            member: member.map(str::to_string),
            links: links.to_vec(),
        })
    }

    fn inner<'b>(&self, name: &str, results: &'b [String], gate: bool) -> At<'b> {
        At {
            key: format!("{}:{name}", self.key),
            at: format!("{}:{name}", self.at),
            results,
            gate,
        }
    }
}

fn members(sites: &mut Vec<Site>, at: &At<'_>, members: &[PanelMember]) {
    for member in members {
        let inner = at.inner(&member.name, at.results, at.gate);
        let site = inner.site(&member.role_path, Some(&member.name), &member.candidates);
        sites.extend(site);
    }
}

/// Every model-facing site of one executable body, keyed as the manifest
/// keys hands (`seat`, `seat:member`, `seat:step`, `seat:step:member`,
/// with a strategy case in brackets) and labelled as the engine labels
/// the driver seat, a strategy case after a colon. A sequence's final
/// step answers with the seat's vocabulary, as the engine hands it.
fn walk(sites: &mut Vec<Site>, at: &At<'_>, body: &SeatBody) {
    match body {
        SeatBody::Single {
            role_path,
            candidates,
            ..
        } => sites.extend(at.site(role_path, None, candidates)),
        SeatBody::Panel { members: panel, .. } => members(sites, at, panel),
        SeatBody::Sequence { steps } => {
            for (index, step) in steps.iter().enumerate() {
                let results = if index + 1 == steps.len() {
                    at.results
                } else {
                    &step.results
                };
                let inner = at.inner(&step.name, results, step.class == SeatClass::Gate);
                match &step.body {
                    StepBody::Single {
                        role_path,
                        candidates,
                        ..
                    } => sites.extend(inner.site(role_path, None, candidates)),
                    StepBody::Panel { members: panel, .. } => members(sites, &inner, panel),
                    // A dialect step is a validator the engine runs itself.
                    StepBody::Dialect { .. } => {}
                }
            }
        }
        // `Seat::sites` resolves a selector before it walks.
        SeatBody::Select { .. } => unreachable!("a strategy case never nests a selector"),
    }
}

/// Every model-facing site of one seat, each strategy case walked as the
/// body the engine would select for it.
fn seat_sites(phase: &str, seat: &Seat) -> Vec<Site> {
    let mut sites = Vec::new();
    let at = |key: String, at: String, strategy: Option<&str>| At {
        key,
        at,
        results: &seat.results,
        gate: seat.body.selected_is_gate(strategy, seat.has_gate),
    };
    match &seat.body {
        SeatBody::Select { cases, default, .. } => {
            for (strategy, case) in cases {
                let key = format!("{phase}[{strategy}]");
                let label = format!("{phase}:{strategy}");
                walk(&mut sites, &at(key, label, Some(strategy)), case);
            }
            if let Some(case) = default {
                let key = format!("{phase}[default]");
                walk(&mut sites, &at(key, format!("{phase}:default"), None), case);
            }
        }
        body => walk(&mut sites, &at(phase.into(), phase.into(), None), body),
    }
    sites
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

/// The input the engine composes for one model site served by `link`:
/// the office's text, the realm's house rules, the dialect prose and the
/// hands, door and notice marks, each written by the engine's own
/// [`SiteMarks`]. Implement is composed as it is once a change is named,
/// its larger input. The run context and the run's paths are journal
/// facts no budget can bound, so they stand at fixed placeholders.
fn site_input(
    marks: SiteMarks<'_>,
    phase: &str,
    house: &str,
    site: &Site,
    link: Option<&Candidate>,
) -> Value {
    let mut seat = json!({"phase": phase});
    marks.seat_dialect(phase, true, &mut seat);
    let mut input = json!({
        "feature": "the feature under delivery",
        "phase": phase,
        "seat": site.at,
        "role_path": site.role_path.to_string_lossy(),
        "workdir": "/repo",
        "result_path": "/repo/.forge/results/effect.json",
        "allowed_results": site.results,
        "house_rules": house,
        "context": {},
    });
    match &site.member {
        Some(member) => marks.member_dialect(member, &seat, &mut input),
        None if !seat["spec_dialect"].is_null() => {
            input["spec_dialect"] = seat["spec_dialect"].clone();
        }
        None => {}
    }
    marks.site(&site.at, site.gate, link, &mut input);
    input
}

/// The prompt every model site is handed, rendered by the driver's own
/// function over the input the engine composes (see [`site_input`]): the
/// largest over every link that may serve it, since a chain fallback
/// moves the notice a boxed site hears.
fn prompts() -> BTreeMap<String, String> {
    let house =
        std::fs::read_to_string(root().join("docs/house-rules.md")).expect("the house reads");
    let mut prompts = BTreeMap::new();
    for (name, bundle) in shipped() {
        let marks = SiteMarks {
            bundle: &bundle,
            boundary: bundle.boundary,
        };
        for (phase, seat) in &bundle.seats {
            for site in seat_sites(phase, seat) {
                let links: Vec<Option<&Candidate>> = match site.links.as_slice() {
                    [] => vec![None],
                    links => links.iter().map(Some).collect(),
                };
                let prompt = links
                    .into_iter()
                    .map(|link| {
                        let input = site_input(marks, phase, &house, &site, link);
                        render_prompt(&input, AdapterKind::Claude)
                            .unwrap_or_else(|error| panic!("{name}/{}: {error}", site.key))
                    })
                    .max_by_key(String::len)
                    .expect("every site has a link or none");
                prompts.insert(format!("{name}/{}", site.key), prompt);
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
