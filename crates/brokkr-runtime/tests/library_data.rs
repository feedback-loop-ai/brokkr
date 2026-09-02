//! T3/T4: the shipped `agents/` library and `adapters/` data.
//!
//! Each shared charter is pinned by digest. The witnesses were re-recorded
//! when decision 0019's closing sweep changed their living prose. Two agents
//! deliberately share a charter file — identical bytes, differing only in
//! tools — so nothing is copied to make the roster look tidy.

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

/// The current bytes of every shared charter.
const CHARTERS: [(&str, &str); 14] = [
    (
        "chief-architect.md",
        "757657c88e0f0b6f48763b836e1e2648e794d5408452dc030138401a5820d60d",
    ),
    (
        "implementer-speckit.md",
        "87425568ebed546e09592abe2238cc163fe693843697f95c8689bf5a850e15c6",
    ),
    (
        "implementer.md",
        "c18d17c1e6630a99aaae4c66787e8bb3f7bdeb86840123b516f42cac9455a27f",
    ),
    (
        "intake-speckit.md",
        "1df8977a6972c28ca0ba9766c0bd50567f8880951ee3d13799207295faaa687c",
    ),
    (
        "intake.md",
        "c1025fb03a97615c5af3bd58a9bf8da231b15071523acd03d3ff057cd8779387",
    ),
    (
        "position-robustness.md",
        "f96e146711c0567ef7c93511a13d5bfbc1414ef7335f3df447ccbc6d83b79927",
    ),
    (
        "position-simplicity.md",
        "d00dfc71d5fbcfd619f72b554747dbc1b2cd318c4b1fee4678dbd6a710a9cddf",
    ),
    (
        "review-correctness.md",
        "ce423d91104cd3e298c49b22a7ebf96182fd2cbde71bd4abc0f147f568aa3001",
    ),
    (
        "review-security-speckit.md",
        "de00a51d25e0a4fc77b12bdb6c0793ec5e6601ebeb05459c6f44e082705e176a",
    ),
    (
        "review-security.md",
        "555a59377d31565a87489664571e23015a958839bea50a226471a99e8b11b869",
    ),
    (
        "review-spec-compliance.md",
        "416f9e17378ab421318a9deee9ba156ab7b8b2e793b6c56fd77253354fe78f75",
    ),
    (
        "reviewer.md",
        "6015367df641c90cf74131b37cda475c12899a0cece1d90ad167a47860e12df8",
    ),
    (
        "shipper.md",
        "df94781f03b42a9b2186c914c92e4fef85aa8db65a664afaeadecd9d9211b1b9",
    ),
    (
        "verifier.md",
        "4cf73af1d979b54cb4026c301bbc7ffa86a4cf7149d037f8d14d05ac714076d0",
    ),
];

/// Charters authored here rather than moved: they have no pre-move
/// bytes to be compared against, and are listed so the accounting below
/// stays exact instead of merely permissive.
const AUTHORED_CHARTERS: [&str; 1] = ["muninn.md"];

/// The seventeen definitions over fifteen charters. The `-speckit` suffix
/// is ugly on purpose: it names WHY the variant exists (it carries the
/// spec-kit CLI permission) and puts the drift on the surface where
/// `brokkr agents list` shows it every time.
const AGENTS: [&str; 17] = [
    "chief-architect",
    "implementer",
    "implementer-speckit",
    "intake",
    "intake-speckit",
    "muninn",
    "position-robustness",
    "position-simplicity",
    "review-correctness",
    "review-security",
    "review-security-speckit",
    "review-spec-compliance",
    "reviewer",
    "shipper",
    "shipper-speckit",
    "verifier",
    "verifier-speckit",
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
fn the_library_holds_seventeen_agents_over_fifteen_charters() {
    let library = library();
    assert_eq!(library.names(), AGENTS.map(str::to_string).to_vec());
    // Two pairs share a charter file: identical bytes, differing only in
    // tools. Nothing is copied to make the roster look tidy.
    for (a, b) in [
        ("verifier", "verifier-speckit"),
        ("shipper", "shipper-speckit"),
    ] {
        let left = library.agent(a).unwrap();
        let right = library.agent(b).unwrap();
        assert_eq!(left.charter, right.charter);
        assert_eq!(left.charter_digest, right.charter_digest);
        assert_ne!(left.allow, right.allow, "the tools are what differ");
    }
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
        assert!(
            resolution.candidates.len() >= 2,
            "{name} ships with a real fallback chain"
        );
        // The composed argv keeps `{brokkr}` a literal: expansion is the
        // compiler's job, and a machine-local path never reaches a digest.
        assert_eq!(resolution.candidates[0].argv[0], "{brokkr}");
    }
}

/// T4: `exec` is the honest degenerate case. It declares all three
/// capabilities unsupported and maps no model, so nothing can select it
/// by accident — which is also why `recipes/sdd`'s `speckit-check` step
/// stays inline and proves the library is an option, not a mandate.
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
