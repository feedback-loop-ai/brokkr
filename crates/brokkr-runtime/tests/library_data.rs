//! T3/T4: the shipped `agents/` library and `adapters/` data.
//!
//! Every charter is pinned by digest in the witness table
//! (`witnesses.json`, #358). Two agents deliberately share a charter file
//! — identical bytes, differing only in tools — so nothing is copied to
//! make the roster look tidy.
//! Decision 0043 retires verifier and shipper from this model library; the
//! roster test accounts for their boxed exec scripts instead.
//! Decision 0058 seats the `recipes/gpt-flash` forced crew as fifteen scoped
//! `gpt-flash-*` offices: each reuses a library charter, names exactly one
//! model, and carries no fallback chain, which is why the resolution test
//! below exempts those offices by name from the standard chain assertion.
//! Its review chief is the exception since 2026-09-30: every chief falls
//! back to astra last (decision 0045's addendum).
//!
//! The adapter data is proved by properties rather than retyped (#358):
//! a model added to an adapter is checked by what it must satisfy, so the
//! edit that adds it touches the adapter, the witness table and the
//! guide's rows (its catalogue row, and a route ruling row when it
//! classes a new route), and no Rust.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use brokkr_runtime::agents::Adapter;
use brokkr_runtime::{resolve_agent, resolve_route, Adapters, Availability, EgressClass, Library};

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[path = "support/witnesses.rs"]
mod witnesses;

use witnesses::Witnesses;

/// Decision 0041's remaining model library roster after decision 0043.
/// `implementer-engine` temporarily shares the implementer charter until
/// strategy-selected seats land. Decision 0044 ruling 4 seats the
/// researcher: the one office that reads the field and holds the fetch
/// grant, authored here like muninn and triage. #355 retires
/// `intake-sdd`, which no recipe seated after `recipes/sdd` was retired.
const AGENTS: [&str; 34] = [
    "analyst",
    "chief-architect",
    "clarifier",
    "gpt-flash-analyst",
    "gpt-flash-chief-architect",
    "gpt-flash-clarifier",
    "gpt-flash-implementer",
    "gpt-flash-implementer-engine",
    "gpt-flash-implementer-sdd",
    "gpt-flash-position-robustness",
    "gpt-flash-position-simplicity",
    "gpt-flash-review-adversarial",
    "gpt-flash-review-chief",
    "gpt-flash-review-correctness",
    "gpt-flash-review-security",
    "gpt-flash-review-spec-compliance",
    "gpt-flash-task-planner",
    "gpt-flash-triage",
    "implementer",
    "implementer-engine",
    "implementer-sdd",
    "intake",
    "muninn",
    "position-robustness",
    "position-simplicity",
    "release-manager",
    "researcher",
    "review-adversarial",
    "review-chief",
    "review-correctness",
    "review-security",
    "review-spec-compliance",
    "reviewer",
    "triage",
];

/// Decision 0058: the `recipes/gpt-flash` forced crew, seated as scoped
/// offices. Each reuses a standard charter and pins exactly one model, so
/// it has no fallback chain; the exemption is this explicit list, not a
/// name prefix, so a new single-model office must be named by a decision.
const SCOPED_OFFICES: [&str; 15] = [
    "gpt-flash-analyst",
    "gpt-flash-chief-architect",
    "gpt-flash-clarifier",
    "gpt-flash-implementer",
    "gpt-flash-implementer-engine",
    "gpt-flash-implementer-sdd",
    "gpt-flash-position-robustness",
    "gpt-flash-position-simplicity",
    "gpt-flash-review-adversarial",
    "gpt-flash-review-chief",
    "gpt-flash-review-correctness",
    "gpt-flash-review-security",
    "gpt-flash-review-spec-compliance",
    "gpt-flash-task-planner",
    "gpt-flash-triage",
];

fn library() -> Library {
    Library::load(&workspace().join("agents")).expect("the shipped library loads")
}

fn adapters() -> Adapters {
    Adapters::load(&workspace().join("adapters")).expect("the shipped adapters load")
}

#[test]
fn the_charter_bytes_match_their_recorded_identities() {
    let root = workspace().join("agents/charters");
    let pinned = Witnesses::load(&workspace()).charters;
    for (name, digest) in &pinned {
        let bytes = std::fs::read(root.join(name))
            .unwrap_or_else(|e| panic!("charter {name} must exist: {e}"));
        assert_eq!(
            brokkr_core::canonical::sha256_bytes(&bytes),
            *digest,
            "charter {name} is not the text it was moved from"
        );
    }
    let present: BTreeSet<String> = std::fs::read_dir(&root)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        present,
        pinned.into_keys().collect::<BTreeSet<_>>(),
        "no charter is unaccounted for"
    );
}

#[test]
fn the_library_holds_the_decision_0041_roster() {
    let library = library();
    assert_eq!(library.names(), AGENTS.map(str::to_string).to_vec());
    let implementer = library.agent("implementer").unwrap();
    let engine = library.agent("implementer-engine").unwrap();
    assert_eq!(implementer.charter, engine.charter);
    assert_eq!(implementer.charter_digest, engine.charter_digest);
    // 0007 declarations stay at their default: the phases' rule-referenced
    // inputs already name exactly the right set for all of them.
    for name in AGENTS {
        assert!(
            library.agent(name).unwrap().inputs.is_none(),
            "{name} should not declare inputs"
        );
    }
    // Decision 0058: the forced crew's scoped offices are ordinary roster
    // entries, so a name that moves out of the roster fails here rather than
    // quietly widening the resolution test's exemption.
    for scoped in SCOPED_OFFICES {
        assert!(
            library.agent(scoped).is_some(),
            "{scoped} is exempted from the fallback assertion but is not a roster office"
        );
    }
}

#[test]
fn sdd_offices_name_the_closed_return_contracts() {
    let charters = workspace().join("agents/charters");
    let analyst = std::fs::read_to_string(charters.join("analyst.md")).unwrap();
    let clarifier = std::fs::read_to_string(charters.join("clarifier.md")).unwrap();
    let smith = std::fs::read_to_string(charters.join("implementer-sdd.md")).unwrap();

    assert!(analyst.contains("context.prior_results.check"));
    for owner in ["`specify`", "`design`", "`tasks`"] {
        assert!(analyst.contains(owner), "analyst omits drift owner {owner}");
    }
    assert!(clarifier.contains("context.prior_results.check"));
    for result in ["`broken`", "`blocked`", "`oversized`"] {
        assert!(smith.contains(result), "SDD smith omits result {result}");
    }
}

/// Proposed decision 0056 ruling 10: the SDD smith persists task
/// progress before the next group, and reconciles it against the
/// worktree on recovery.
///
/// The rule lives in this ONE charter, inherited by both dialects, and
/// it names the task artifact generically. Decision 0042 ruling 6 is
/// what keeps a framework path out of an office charter: `tasks.md` is
/// OpenSpec's spelling and a phased row is spec-kit's, and a charter
/// that named either would be a realm's dialect written into Brokkr's
/// office.
#[test]
fn the_sdd_smith_persists_progress_before_the_next_group_and_reconciles_on_recovery() {
    let charters = workspace().join("agents/charters");
    // Prose wraps, so the clauses are matched against one collapsed
    // line: a rewrap must not be able to fail this test, and a deleted
    // clause must not be able to pass it.
    let flat = |name: &str| {
        std::fs::read_to_string(charters.join(name))
            .unwrap()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    let smith = flat("implementer-sdd.md");

    // The timing rule, its partial-work arm, and the four facts that may
    // not stand for one another.
    for clause in [
        "not at commit time",
        "record the group as in progress",
        "name the focused acceptance checks",
        "before starting the next group",
        "stays unchecked and carries the next action",
        "four separate facts",
    ] {
        assert!(
            smith.contains(clause),
            "the SDD smith omits the progress clause {clause:?}"
        );
    }
    // The recovery clause, warm or cold, and its two prohibitions.
    for clause in [
        "resumed or started cold",
        "current worktree",
        "Reconcile the ticks",
        "return a task whose work no longer holds to pending",
        "Never erase partial uncommitted edits",
        "never read another session's private transcript",
        "Current evidence outranks memory",
    ] {
        assert!(
            smith.contains(clause),
            "the SDD smith omits the recovery clause {clause:?}"
        );
    }
    // No framework path and no repository command: the dialect owns
    // both, and this charter serves OpenSpec and spec-kit alike
    // (decision 0042 ruling 6).
    for framework in ["tasks.md", "openspec", "spec-kit", "specify", "cargo "] {
        assert!(
            !smith.to_lowercase().contains(framework),
            "the SDD charter names {framework:?}, which belongs to the dialect"
        );
    }

    // The rule is the SDD office's, not every implementer's: the non-SDD
    // charter has no required dialect task artifact to persist into.
    assert!(
        !flat("implementer.md").contains("before starting the next group"),
        "the non-SDD implementer gains no progress-timing rule"
    );
}

/// Every shipped agent resolves against the shipped adapters, with no
/// availability facts — which is exactly what `Bundle::compile` does.
#[test]
fn every_shipped_agent_resolves_at_compile_time() {
    let (library, adapters) = (library(), adapters());
    for name in AGENTS {
        let resolution = resolve_agent(&library, &adapters, &Availability::unspecified(), name)
            .unwrap_or_else(|e| panic!("agent {name} must resolve: {e}"));
        assert_eq!(resolution.record["chosen_index"], 0);
        assert!(
            resolution.notices.is_empty(),
            "{name} ships with no capability gap"
        );
        // Decision 0058: `muninn` is the one standard office that names a
        // single model, and the `recipes/gpt-flash` forced crew is seated as
        // scoped offices that deliberately pin one model each so no fallback
        // can silently reach another vendor or an older Flash. Every other
        // model-backed office keeps a real chain (0041 ruling 2). The one
        // scoped exception is the chief's: decision 0045's addendum of
        // 2026-09-30 gives every chief astra, codex's own, as its last
        // fallback, so no fallback leaves the vendor.
        if name == "gpt-flash-review-chief" {
            let chain: Vec<&str> = resolution
                .candidates
                .iter()
                .map(|candidate| candidate.model.as_str())
                .collect();
            assert_eq!(chain, ["sol", "astra"], "{name} falls back to astra alone");
        } else if name == "muninn" || SCOPED_OFFICES.contains(&name) {
            assert_eq!(
                resolution.candidates.len(),
                1,
                "{name} must pin exactly one model (decision 0058)"
            );
        } else {
            assert!(
                resolution.candidates.len() >= 2,
                "{name} ships with a real fallback chain"
            );
        }
        // The composed argv keeps `{brokkr}` a literal: expansion is the
        // compiler's job, and a machine-local path never reaches a digest.
        assert_eq!(resolution.candidates[0].argv[0], "{brokkr}");
    }
}

/// T4: `exec` is the honest degenerate case. It declares all three
/// capabilities unsupported and maps no model, so nothing can select it
/// by accident. Dialect validators also use exec, but are resolved from the
/// realm's checked dialect instead of pretending to be model-backed agents.
#[test]
fn the_exec_adapter_declares_every_capability_unsupported() {
    let adapters = adapters();
    let providers: Vec<&str> = adapters
        .providers()
        .map(|adapter| adapter.provider.as_str())
        .collect();
    assert_eq!(
        providers,
        vec!["claude", "codex", "dsh", "exec", "lanetally"]
    );
    let exec = adapters
        .providers()
        .find(|adapter| adapter.provider == "exec")
        .unwrap();
    assert!(exec.model_flag.is_none());
    assert!(exec.tool_permissions.is_none());
    assert!(exec.mcp.is_none());
    assert!(exec.models.is_empty());
    // `codex` maps models, and its map is proved where its evidence is
    // (`the_shipped_codex_adapter_maps_the_models_its_own_cli_names`);
    // every alias it maps is held to the properties below.
    let codex = adapters
        .providers()
        .find(|adapter| adapter.provider == "codex")
        .unwrap();
    assert_eq!(codex.model_flag.as_deref(), Some("--model"));
    // Still no tool restriction — but now for a MEASURED reason rather
    // than a bare "unsupported". The capability stays `None`, so the
    // fail-closed refusal is byte-for-byte the same decision it was;
    // what changed is that the adapter can say why.
    assert!(
        codex.tool_permissions.is_none(),
        "codex cannot express a per-tool restriction, and says so"
    );
    let gap = codex
        .tool_permissions_gap
        .as_deref()
        .expect("codex records WHY it cannot, not just that it cannot");
    assert!(
        gap.contains("--sandbox"),
        "the gap names codex's real restriction axis: {gap}"
    );
    // `dsh` maps the lanes the provider-adapters guide gives the evidence
    // for. The flag is the shared `--model` grammar; the driver turns
    // `<route>/<id>` into the overlay dsh's launcher reads. Tools stay
    // unexpressible, and the data says so.
    let dsh = adapters
        .providers()
        .find(|adapter| adapter.provider == "dsh")
        .unwrap();
    assert_eq!(dsh.model_flag.as_deref(), Some("--model"));
    assert!(
        dsh.tool_permissions.is_none(),
        "dsh cannot express a tool restriction, and says so"
    );
}

/// The first `/`-separated segment of every model id an adapter maps: the
/// routes its data can reach.
fn reached_routes(adapter: &Adapter) -> BTreeSet<&str> {
    adapter
        .models
        .values()
        .filter_map(|id| resolve_route(adapter, id).0)
        .collect()
}

/// #358: every model resolves to a route with a declared egress. An
/// unprefixed id takes the adapter's own class; a prefixed one takes the
/// class `routes` declares for its route, or the floor for a route the
/// file does not class (decision 0036 ruling 1). And every route the file
/// names — classed, keyed or effortless — is one some mapped model
/// reaches, so a misspelt prefix cannot leave a ruling on a route nothing
/// uses while the model beside it falls to the floor.
#[test]
fn every_model_resolves_to_a_route_with_a_declared_egress() {
    for adapter in adapters().providers() {
        let provider = &adapter.provider;
        for (alias, id) in &adapter.models {
            let declared = match resolve_route(adapter, id).0 {
                None => adapter.egress,
                Some(route) => adapter
                    .routes
                    .get(route)
                    .copied()
                    .unwrap_or(EgressClass::Uncontracted),
            };
            assert_eq!(
                resolve_route(adapter, id).1,
                declared,
                "{provider} '{alias}' ({id}) resolves to a class its data does not declare"
            );
        }
        let reached = reached_routes(adapter);
        let named = adapter
            .routes
            .keys()
            .chain(adapter.credentials.keys())
            .chain(adapter.effortless_routes.keys());
        for route in named {
            assert!(
                reached.contains(route.as_str()),
                "{provider} names route '{route}', which no model it maps reaches"
            );
        }
    }
}

/// A `YYYY-MM-DD` date anywhere in `text`.
fn carries_a_date(text: &str) -> bool {
    let shape = |window: &[u8]| {
        window.iter().enumerate().all(|(at, byte)| match at {
            4 | 7 => *byte == b'-',
            _ => byte.is_ascii_digit(),
        })
    };
    text.as_bytes().windows(10).any(shape)
}

/// #358: every route with no effort levels carries a dated, measured
/// reason: a `YYYY-MM-DD` date, and the release of the adapter's own
/// binary the refusal was measured on (`<binary> <version>`). A seat on
/// such a route pins no effort (decision 0035 addendum 2026-09-11), so the
/// reason is what a reader has in place of a level table.
#[test]
fn every_effortless_route_carries_a_dated_measured_reason() {
    for adapter in adapters().providers() {
        let measured_on = format!("{} ", adapter.binary);
        for (route, reason) in &adapter.effortless_routes {
            let provider = &adapter.provider;
            assert!(
                carries_a_date(reason),
                "{provider} effortless route '{route}' gives no YYYY-MM-DD date: {reason}"
            );
            let release = reason.match_indices(&measured_on).any(|(at, _)| {
                reason[at + measured_on.len()..].starts_with(|c: char| c.is_ascii_digit())
            });
            assert!(
                release,
                "{provider} effortless route '{route}' names no '{measured_on}<version>' \
                 it was measured on: {reason}"
            );
        }
    }
}

/// One backticked name, alone in its cell.
fn ticked(cell: &str) -> String {
    cell.strip_prefix('`')
        .and_then(|cell| cell.strip_suffix('`'))
        .filter(|name| !name.is_empty() && !name.contains('`'))
        .unwrap_or_else(|| panic!("unreadable guide table cell: {cell}"))
        .to_string()
}

/// The body rows of the provider-adapters guide's table under `### heading`,
/// cell by cell. A missing section, another header, or a row with another
/// number of cells fails the test rather than being skipped.
fn guide_table<const N: usize>(heading: &str, header: [&str; N]) -> Vec<[String; N]> {
    let guide = std::fs::read_to_string(workspace().join("docs/guides/provider-adapters.md"))
        .expect("the provider-adapters guide");
    let (_, section) = guide
        .split_once(&format!("\n### {heading}\n"))
        .unwrap_or_else(|| panic!("the guide declares {heading}"));
    let cells = |line: &str| -> Option<[String; N]> {
        let inner = line.strip_prefix('|')?.strip_suffix('|')?;
        let cells: Vec<String> = inner.split('|').map(|c| c.trim().to_string()).collect();
        cells.try_into().ok()
    };
    let mut rows = section
        .lines()
        .skip_while(|line| !line.starts_with('|'))
        .take_while(|line| line.starts_with('|'));
    assert_eq!(
        (rows.next().and_then(cells), rows.next().and_then(cells)),
        (
            Some(header.map(String::from)),
            Some(std::array::from_fn(|_| "---".to_string()))
        ),
        "{heading}: the table's header"
    );
    rows.map(|line| cells(line).unwrap_or_else(|| panic!("unreadable {heading} row: {line}")))
        .collect()
}

/// The guide's alias catalogue: provider → the aliases it declares no
/// shipped agent hires. A header, row or repeated provider this reader
/// does not recognise fails the test rather than being skipped.
fn catalogue() -> BTreeMap<String, BTreeSet<String>> {
    let mut catalogue = BTreeMap::new();
    let header = ["Adapter", "Aliases no shipped agent hires"];
    for [provider, aliases] in guide_table("The alias catalogue", header) {
        let aliases = aliases.split(", ").map(ticked);
        let previous = catalogue.insert(ticked(&provider), aliases.collect::<BTreeSet<_>>());
        assert!(previous.is_none(), "a second catalogue row for {provider}");
    }
    catalogue
}

/// #358 (decision 0036): which class a route stands in is the operator's
/// ruling, so it is kept literally in one place, the guide's dated route
/// rulings table, and every adapter classes exactly the routes that table
/// lists, as it lists them. A route classed, reclassified or unclassed in
/// an adapter file is that row's edit too.
#[test]
fn every_classed_route_is_a_dated_ruling_in_the_guide() {
    let header = ["Adapter", "Route", "Egress", "Ruled"];
    let mut ruled = BTreeMap::new();
    for [provider, route, egress, date] in guide_table("The route rulings", header) {
        assert!(
            date.len() == 10 && carries_a_date(&date),
            "the ruling on {provider} {route} gives no YYYY-MM-DD date: {date}"
        );
        let class = EgressClass::parse(&ticked(&egress))
            .unwrap_or_else(|| panic!("the ruling on {provider} {route} names no class: {egress}"));
        let previous = ruled.insert((ticked(&provider), ticked(&route)), class);
        assert!(previous.is_none(), "a second ruling on {provider} {route}");
    }
    let declared: BTreeMap<(String, String), EgressClass> = adapters()
        .providers()
        .flat_map(|adapter| {
            let provider = &adapter.provider;
            adapter
                .routes
                .iter()
                .map(move |(route, class)| ((provider.clone(), route.clone()), *class))
        })
        .collect();
    assert_eq!(
        declared, ruled,
        "the adapters must class exactly the routes the guide's route rulings list"
    );
}

/// #358 (decision 0071 ruling 6): every alias an adapter maps is hired by
/// a library agent, or listed in the guide's declared catalogue, and the
/// catalogue lists exactly the aliases nothing hires — so it cannot keep a
/// name the adapter dropped or an agent has since hired.
#[test]
fn every_alias_is_hired_or_catalogued() {
    let library = library();
    let hired: BTreeSet<&str> = library
        .names()
        .iter()
        .flat_map(|name| library.agent(name).unwrap().models.iter())
        .map(String::as_str)
        .collect();
    let unhired: BTreeMap<String, BTreeSet<String>> = adapters()
        .providers()
        .map(|adapter| {
            let aliases = adapter
                .models
                .keys()
                .filter(|alias| !hired.contains(alias.as_str()));
            (
                adapter.provider.clone(),
                aliases.cloned().collect::<BTreeSet<_>>(),
            )
        })
        .filter(|(_, aliases)| !aliases.is_empty())
        .collect();
    assert_eq!(
        catalogue(),
        unhired,
        "the guide's alias catalogue must list exactly the aliases no agent hires"
    );
}

/// No adapter file carries a value — only names, flags and ids
/// (decision 0012 unchanged). A secret-shaped assignment anywhere in the
/// two trees is a compile refusal, and this asserts the shipped data has
/// none to begin with.
#[test]
fn no_shipped_data_file_carries_a_secret_value() {
    for tree in ["agents", "adapters"] {
        let mut stack = vec![workspace().join(tree)];
        while let Some(current) = stack.pop() {
            for entry in std::fs::read_dir(&current).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                assert_ne!(
                    path.file_name().and_then(|n| n.to_str()),
                    Some("secrets.env"),
                    "the {tree} tree must carry names, never values"
                );
            }
        }
    }
}
