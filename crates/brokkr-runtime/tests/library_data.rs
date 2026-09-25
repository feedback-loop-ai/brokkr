//! T3/T4: the shipped `agents/` library and `adapters/` data.
//!
//! Each shared charter is pinned by digest. The witnesses were re-recorded
//! when decision 0019's closing sweep changed their living prose. Two agents
//! deliberately share a charter file — identical bytes, differing only in
//! tools — so nothing is copied to make the roster look tidy.
//! Decision 0041 rulings 4 and 5 deliberately move the implement and review
//! pins: implementers learn the bounded-return vocabulary, while every judge
//! becomes read-only and reports the return instead of applying it.
//! Decision 0043 retires verifier and shipper from this model library; the
//! roster test accounts for their boxed exec scripts instead.
//! Decision 0058 seats the `recipes/gpt-flash` forced crew as fifteen scoped
//! `gpt-flash-*` offices: each reuses a library charter, names exactly one
//! model, and carries no fallback chain, which is why the resolution test
//! below exempts those offices by name from the standard chain assertion.

use std::path::PathBuf;

use brokkr_runtime::{resolve_agent, Adapters, Availability, Library};

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

/// The current bytes of every shared charter. Decision 0041 ruling 8 moves
/// the affected witnesses because repository rules left the office text and
/// the three sequence disclaimers disappeared. Decision 0042 moves the four
/// SDD office charters as dialect-specific prose leaves them. Decision 0071
/// (#333) moves the eight pinned here that build, design or judge code, as
/// their charters gained its principles; each value is the test's own
/// reported digest.
const CHARTERS: [(&str, &str); 13] = [
    (
        "chief-architect.md",
        "290cfc2763143a2c2411af161fde01558df9b73783cb16d53d95048ddfb8d783",
    ),
    (
        // Moved by proposed decision 0056 ruling 10: the SDD smith
        // persists task progress before the next group and reconciles it
        // against the worktree on recovery, warm or cold. Moved again by
        // the Opus 5.5 / Fable 5.1 prompt audit (acffed37): the condensed
        // scope, evidence and targeted-edit paragraph.
        // Moved again by decision 0071 (#333): the design paragraph.
        "implementer-sdd.md",
        "7df0a3322a937e979795824b537f73b1762b2250f4810deb54d196dfffb9e448",
    ),
    (
        // Moved by the Opus 5.5 / Fable 5.1 prompt audit (acffed37): the
        // scope and evidence paragraphs and the targeted-edit sentence.
        // Moved again by decision 0071 (#333): the design paragraph.
        "implementer.md",
        "00f320f4ee61808db3f121a143e2b4f30515d1a892540302453beec0ac683beb",
    ),
    (
        "intake-sdd.md",
        "bbd5c49d97796d91df3713344faaa3adb536e9acc36ae7cfc5cb1e2700211e9d",
    ),
    (
        "intake.md",
        "fbdb7dba8e34fbc0b02e0f7fd7540fd0ab9313e40cdbcb03c27c22d78c138756",
    ),
    (
        "position-robustness.md",
        "6d7926ae2f207ca576f65d949dae79501cfeccca5cd0ae436b2e85ce8969241a",
    ),
    (
        "position-simplicity.md",
        "22a5d52aa78e3b1d9ba28afdcdb938b1f2857d7c5a946dbd10a205fc4d1e880f",
    ),
    (
        "review-correctness.md",
        "40fe2d11b6d20ee7964d503aea72548605758ee20f8bae521ad65664028dbea1",
    ),
    (
        "review-adversarial.md",
        "b188aed4546a8af672835f7fea2de1ac1c13d1f6ae783baafd8644e440743251",
    ),
    (
        "review-chief.md",
        "199acde1e2e1cdd41d432f58c2f9a5ea6d74fa80d26c06302ad464125774b7e8",
    ),
    (
        "review-security.md",
        "33d6b92f2a349636e60cb9a4ef6a90fcf6925709742457ef918fbaf80a2f0b89",
    ),
    (
        "review-spec-compliance.md",
        "bcfc9eedf910ddae08807b3720558d665a03ca9ddb2211dbfddc5839da946782",
    ),
    (
        "reviewer.md",
        "311489fc120a0bec72ffd0302bac12e60f184ca0e6d4c9e168f4e88453f6c410",
    ),
];

/// Charters authored here rather than moved: they have no pre-move
/// bytes to be compared against, and are listed so the accounting below
/// stays exact instead of merely permissive.
const AUTHORED_CHARTERS: [&str; 6] = [
    "analyst.md",
    "clarifier.md",
    "muninn.md",
    "release-manager.md",
    "researcher.md",
    "triage.md",
];

/// Decision 0041's remaining model library roster after decision 0043.
/// `implementer-engine` temporarily shares the implementer charter until
/// strategy-selected seats land. Decision 0044 ruling 4 seats the
/// researcher: the one office that reads the field and holds the fetch
/// grant, authored here like muninn and triage.
const AGENTS: [&str; 35] = [
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
    "intake-sdd",
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
    for (name, digest) in CHARTERS {
        let bytes = std::fs::read(root.join(name))
            .unwrap_or_else(|e| panic!("charter {name} must exist: {e}"));
        assert_eq!(
            brokkr_core::canonical::sha256_bytes(&bytes),
            digest,
            "charter {name} is not the text it was moved from"
        );
    }
    let present: Vec<String> = std::fs::read_dir(&root)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    for name in AUTHORED_CHARTERS {
        assert!(
            root.join(name).is_file(),
            "authored charter {name} must exist"
        );
    }
    assert_eq!(
        present.len(),
        CHARTERS.len() + AUTHORED_CHARTERS.len(),
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
        // model-backed office keeps a real chain (0041 ruling 2).
        if name == "muninn" || SCOPED_OFFICES.contains(&name) {
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
    // `codex` DOES map models now, and the reason the old pin's "no
    // established mapping" no longer holds is evidence: `codex debug
    // models` on the installed codex-cli 0.148.0 named the three gpt-5.6
    // slugs with visibility "list" and supported_in_api true, and the
    // 0.153.2 catalog lists `gpt-6-astra` beside them at priority 1
    // (decision 0045). The abstract names are codex's own family words —
    // deliberately NOT claude tiers, for the reason `dsh` below spells
    // out.
    let codex = adapters
        .providers()
        .find(|adapter| adapter.provider == "codex")
        .unwrap();
    for (abstract_name, concrete) in [
        ("astra", "gpt-6-astra"),
        ("sol", "gpt-6-sol"),
        ("terra", "gpt-5.6-terra"),
        ("luna", "gpt-6-luna"),
    ] {
        assert_eq!(
            codex.models.get(abstract_name).map(String::as_str),
            Some(concrete),
            "codex maps the {abstract_name} lane its own CLI catalog names"
        );
    }
    assert_eq!(
        codex.models.len(),
        4,
        "four catalogued lanes, no invented ones"
    );
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
    // `dsh` maps the lanes this tree has evidence for, each verified
    // with a completion against its provider on 2026-09-02: DeepSeek's
    // own API serves `deepseek-v4-pro` (bare id — the dated spellings
    // live only in LaneTally's price rows), and Model Studio's Token Plan
    // catalogue serves the eight behind `dashscope/`, its own DeepSeek
    // snapshot dated in the id. `flash` pins `deepseek-flash`, the name
    // DeepSeek's pricing page gives DeepSeek-V4.1-Flash; the retired
    // `deepseek-v4-flash` and the expired beta
    // `deepseek-v4.1-flash-expires-on-0910` both answer as
    // `deepseek-flash` (completions against DeepSeek's API, 2026-09-25),
    // so neither is pinned.
    // `spark/` is the operator's DGX Spark: SGLang serving
    // RadixArk/Qwen3.8-Flash-Next-NVFP4 as `qwen3.8-flash` (256k context,
    // qwen3_coder tool parser, radix prefix cache), verified with a
    // headless dsh turn on 2026-09-02; the route lives in the dsh profile
    // and costs electricity, not cents. `meta/` and `meta-contributor/`
    // are Muse Spark 1.3 through OpenRouter (openrouter.ai/api/v1,
    // OpenAI-compatible, Meta as sole upstream at Meta's own prices)
    // under two ids that differ only in terms: the bare id is not used
    // to improve Meta's products, the `-contributor` id is, at a
    // fraction of the price. Two routes, one key name, because egress
    // is a property of the route (decision 0036) and the terms are the
    // egress fact. The route is the FIRST segment; OpenRouter's own
    // `meta/` inside the id names the model, not a route, which is why
    // the concrete ids carry `meta/` twice. Switched from Meta's own
    // endpoint on 2026-09-05 when its billing refused the operator's
    // card, and verified the same day with a headless dsh turn on the
    // contributor route: the record names provider and model, reports
    // usage, and carries Meta's reasoning encrypted.
    // Abstract names are NOT claude tiers, so no chain written for one
    // provider silently lands on the other. The flag is the shared
    // `--model` grammar; the driver turns `<provider>/<id>` into the
    // overlay dsh's launcher reads. Tools stay unexpressible, and the
    // data says so.
    let dsh = adapters
        .providers()
        .find(|adapter| adapter.provider == "dsh")
        .unwrap();
    let lanes: Vec<(&str, &str)> = dsh
        .models
        .iter()
        .map(|(name, id)| (name.as_str(), id.as_str()))
        .collect();
    assert_eq!(
        lanes,
        [
            ("flash", "deepseek-flash"),
            ("glm", "dashscope/glm-5.2"),
            ("glm-flash", "spark-glm/GLM-5.3-Flash-EXL3"),
            ("glm53", "dashscope/glm-5.3"),
            ("muse", "meta/meta/muse-spark-1.3"),
            (
                "muse-contributor",
                "meta-contributor/meta/muse-spark-1.3-contributor"
            ),
            ("pro", "deepseek-v4-pro"),
            ("qwen-flash", "dashscope/qwen3.8-flash"),
            ("qwen-max", "dashscope/qwen3.8-max"),
            ("qwen-plus", "dashscope/qwen3.7-plus"),
            ("qwen36-flash", "dashscope/qwen3.6-flash"),
            ("qwen37-max", "dashscope/qwen3.7-max"),
            ("spark-flash", "spark/qwen3.8-flash"),
            ("studio-flash", "dashscope/deepseek-v4-flash-0731"),
            ("studio-flash41", "dashscope/deepseek-v4.1-flash"),
            ("studio-pro", "dashscope/deepseek-v4-pro"),
        ]
    );
    assert_eq!(dsh.model_flag.as_deref(), Some("--model"));
    assert!(
        dsh.tool_permissions.is_none(),
        "dsh cannot express a tool restriction, and says so"
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
