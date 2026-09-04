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
/// SDD office charters as dialect-specific prose leaves them.
const CHARTERS: [(&str, &str); 13] = [
    (
        "chief-architect.md",
        "c6d224031f2e18010fc5e104cf4692fe7c51713e9c43da9f89f748331f4a69da",
    ),
    (
        "implementer-sdd.md",
        "720152f35e0190566eb39716ba554fa3ceec705be29b41479cfcb9e464114949",
    ),
    (
        "implementer.md",
        "b750b0a401fa7fc1aad5dd929bf136cf961b12d2e11ac9fc67995927ea686ad7",
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
        "696802b3d981e7d487926500a749c699e1697b05a0d7e5686639f8dd09067533",
    ),
    (
        "position-simplicity.md",
        "81a14dffa301d38f2d523d0a828fd7ce5accf38a0e75dc3995f9682c9cf72b0d",
    ),
    (
        "review-correctness.md",
        "7d11cd3201c6bf9464b7092e456ad0e432772aa7cf0fee28d3b18782733b172b",
    ),
    (
        "review-adversarial.md",
        "b188aed4546a8af672835f7fea2de1ac1c13d1f6ae783baafd8644e440743251",
    ),
    (
        "review-chief.md",
        "72aa9dcee51170ef932661db045a3acc40e7af47fc9f68271c00fa15f943ddec",
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
        "4efedc43f0b8ac110000f4ffa3b3205aac3acac0850485b027d298dd2b8aa4e8",
    ),
];

/// Charters authored here rather than moved: they have no pre-move
/// bytes to be compared against, and are listed so the accounting below
/// stays exact instead of merely permissive.
const AUTHORED_CHARTERS: [&str; 5] = [
    "analyst.md",
    "clarifier.md",
    "muninn.md",
    "researcher.md",
    "triage.md",
];

/// Decision 0041's remaining model library roster after decision 0043.
/// `implementer-engine` temporarily shares the implementer charter until
/// strategy-selected seats land. Decision 0044 ruling 4 seats the
/// researcher: the one office that reads the field and holds the fetch
/// grant, authored here like muninn and triage.
const AGENTS: [&str; 19] = [
    "analyst",
    "chief-architect",
    "clarifier",
    "implementer",
    "implementer-engine",
    "implementer-sdd",
    "intake",
    "intake-sdd",
    "muninn",
    "position-robustness",
    "position-simplicity",
    "researcher",
    "review-adversarial",
    "review-chief",
    "review-correctness",
    "review-security",
    "review-spec-compliance",
    "reviewer",
    "triage",
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
        if name == "muninn" {
            assert_eq!(resolution.candidates.len(), 1);
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
    // models` on the installed codex-cli 0.148.0 names these three
    // slugs with visibility "list" and supported_in_api true. The
    // abstract names are codex's own family words — deliberately NOT
    // claude tiers, for the reason `dsh` below spells out.
    let codex = adapters
        .providers()
        .find(|adapter| adapter.provider == "codex")
        .unwrap();
    for (abstract_name, concrete) in [
        ("sol", "gpt-5.6-sol"),
        ("terra", "gpt-5.6-terra"),
        ("luna", "gpt-5.6-luna"),
    ] {
        assert_eq!(
            codex.models.get(abstract_name).map(String::as_str),
            Some(concrete),
            "codex maps the {abstract_name} lane its own CLI catalog names"
        );
    }
    assert_eq!(
        codex.models.len(),
        3,
        "three catalogued lanes, no invented ones"
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
    // own API serves exactly `deepseek-v4-flash` and `deepseek-v4-pro`
    // (bare ids — the dated spellings live only in LaneTally's price
    // rows), and Model Studio's Token Plan catalogue serves the eight
    // behind `dashscope/`, its own DeepSeek snapshot dated in the id.
    // `spark/` is the operator's DGX Spark: SGLang serving
    // RadixArk/Qwen3.8-Flash-Next-NVFP4 as `qwen3.8-flash` (256k context,
    // qwen3_coder tool parser, radix prefix cache), verified with a
    // headless dsh turn on 2026-09-02; the route lives in the dsh profile
    // and costs electricity, not cents.
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
            ("flash", "deepseek-v4-flash"),
            ("glm", "dashscope/glm-5.2"),
            ("pro", "deepseek-v4-pro"),
            ("qwen-flash", "dashscope/qwen3.8-flash"),
            ("qwen-max", "dashscope/qwen3.8-max"),
            ("qwen-plus", "dashscope/qwen3.7-plus"),
            ("qwen36-flash", "dashscope/qwen3.6-flash"),
            ("qwen37-max", "dashscope/qwen3.7-max"),
            ("spark-flash", "spark/qwen3.8-flash"),
            ("studio-flash", "dashscope/deepseek-v4-flash-0731"),
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
