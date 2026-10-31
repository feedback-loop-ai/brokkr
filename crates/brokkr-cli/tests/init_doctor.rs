//! `brokkr init` scaffolds a bundle that compiles under the
//! constitutional lint; `brokkr doctor` reports health without executing
//! any agent.
//!
//! `init` writes the `.brokkr/` folder at a repository's root (decision
//! 0083), so every scaffold here is made in a fresh `git init`: a verb run
//! from the root or from inside the folder finds the folder's
//! `adapters/`, where the trust tier its gate seats judge on is declared
//! (decision 0021), as discovery reads every other default (ruling 2).

use std::process::Command;

use crate::brokkr_folder::{git, repository, Repo, ROOT_LAYOUT, STATE_ONLY};
use crate::init_stacks::brokkr;

#[test]
fn init_scaffolds_a_compiling_bundle_and_refuses_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    let (code, _, stderr) = brokkr(&["init", "."], dir.path());
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert!(stderr.contains("digest"), "stderr: {stderr}");
    // Discovery serves the whole repository, so the scaffold no longer
    // says where to stand: it says, on a line of its own, that the
    // operator's MCP servers are not a seat's (SI2), and last, how to
    // commit the folder.
    assert!(!stderr.contains("run brokkr from inside"), "{stderr}");
    assert!(
        stderr.contains(&format!("\n{AMBIENT_MCP}\n")),
        "stderr: {stderr}"
    );
    assert!(
        stderr.ends_with("\ncommit the folder: git add .brokkr && git commit\n"),
        "stderr: {stderr}"
    );
    let bundle = dir.path().join(".brokkr");
    let notes = std::fs::read_to_string(bundle.join("agents/README.md")).unwrap();
    assert!(
        notes.contains(&format!("\n\n{AMBIENT_MCP}\n\n## Specification dialect\n")),
        "{notes}"
    );

    // The scaffold passes the same compile gate as any bundle.
    let (code, stdout, stderr) = brokkr(&["compile", "--bundle", "."], &bundle);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert!(stdout.contains("\"starter\""));

    // The tightened ship taxonomy is present from the start.
    let policy = std::fs::read_to_string(bundle.join("policy.json")).unwrap();
    assert!(policy.contains("SHIP-COMPLETE"));
    assert!(policy.contains("SHIPPED-DIRTY"));

    // Decision 0021 ruling 1's roster is declared, not left to default:
    // the three gate seats say so, and the two work seats say so too.
    let scaffolded = std::fs::read_to_string(bundle.join("bundle.json")).unwrap();
    assert_eq!(scaffolded.matches("\"class\": \"gate\"").count(), 3);
    assert_eq!(scaffolded.matches("\"class\": \"work\"").count(), 2);
    // …and the tier they judge on is a file in the operator's tree,
    // theirs to demote, rather than a constant inside this binary.
    let adapter = std::fs::read_to_string(bundle.join("adapters/claude.json")).unwrap();
    assert!(adapter.contains("\"trust_tier\": \"trusted\""), "{adapter}");
    assert!(
        adapter.contains("\"egress\": \"uncontracted\""),
        "{adapter}"
    );

    // The review seat is a gate, and a gate that moves HEAD parks the
    // run: its charter is read-only, as the library's reviewer is.
    let reviewer = std::fs::read_to_string(bundle.join("agents/charters/reviewer.md")).unwrap();
    assert!(
        reviewer.contains("strictly read-only: change no files and make no commits"),
        "{reviewer}"
    );
    assert!(
        reviewer.contains("`clean` with `inputs: {\"fixes_applied\": false}`"),
        "{reviewer}"
    );
    assert!(
        reviewer.contains("since its merge-base with the default branch"),
        "{reviewer}"
    );
    assert!(reviewer.contains("`context.entry`"), "{reviewer}");
    assert!(!reviewer.contains("since the run began"), "{reviewer}");
    assert!(!reviewer.contains("commit them"), "{reviewer}");

    // Refuses to write over the folder it wrote, naming what it holds.
    let (code, _, stderr) = brokkr(&["init", "."], dir.path());
    let refused = format!(
        "error: {}/.brokkr already holds adapters, agents, bundle.json, policy.json, realms.json, \
         scripts (decision 0083); init writes a folder only where none stands, or beside moved \
         state (.gitignore and state/ alone): move or remove them\n",
        dir.path().canonicalize().unwrap().display()
    );
    assert_eq!((code, stderr), (Some(1), refused));
}

/// `init` under a PATH holding only the named agent CLIs: the repository,
/// its folder, `init`'s stderr, and the compile run from inside the
/// folder.
fn init_with_only(present: &[&str]) -> (Repo, std::path::PathBuf, String) {
    let repo = Repo::new();
    let ran = repo.init_with_only(present);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let bundle = repo.path(".brokkr");
    let (code, stdout, error) = brokkr(&["compile", "--bundle", "."], &bundle);
    assert_eq!(code, Some(0), "{error}");
    assert!(stdout.contains("\"starter\""), "{stdout}");
    (repo, bundle, ran.stderr)
}

fn json_at(path: &std::path::Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// Every executable site's SI2 record as the compile kept it under the
/// standing fence (U1g1), one line per outcome: each shape it is served as,
/// by bare invocation and hands, with what its adapter measured there.
pub(crate) fn strict_records(compiled: &brokkr_runtime::Bundle) -> Vec<(String, Vec<String>)> {
    use brokkr_runtime::agents::{McpAxis, McpHands, McpInvocation, McpUnmeasured};
    let word = |(shape, ambient): &(brokkr_runtime::agents::McpShape, McpAxis)| {
        let invocation = match &shape.invocation {
            McpInvocation::Cold => "cold",
            McpInvocation::Replacement => "replacement",
            McpInvocation::Resume(name) => name,
        };
        let hands = match shape.hands {
            McpHands::Boxed => "boxed",
            McpHands::Harness => "harness",
            McpHands::NoHands => "none",
        };
        let verdict = match ambient {
            McpAxis::Unmeasured(McpUnmeasured::Absent) => "absent",
            McpAxis::Measured { .. } => "measured",
            McpAxis::Unmeasured(McpUnmeasured::Declared(_)) => "unmeasured",
            McpAxis::Unsupported { .. } => "unsupported",
            other => panic!("a scaffold declares no other fact: {other:?}"),
        };
        format!("{invocation} {hands} {verdict}")
    };
    let mut records = Vec::new();
    for (label, facts) in &compiled.sites {
        let Some(site) = facts.capabilities.as_ref() else {
            continue;
        };
        assert_eq!(site.strict.len(), site.outcomes.len(), "{label}");
        for (outcome, record) in site.outcomes.iter().zip(&site.strict) {
            let shapes: Vec<String> = record.iter().map(word).collect();
            records.push((format!("{label} {}", outcome.provider), shapes));
        }
    }
    records
}

/// What a scaffold's adapters measured on this host: U0 and U0b measured
/// on Linux alone, so on macOS every shape a scaffold declares is absent.
pub(crate) fn on_this_host(linux: &str) -> String {
    match cfg!(target_os = "macos") {
        true => linux
            .replace("unmeasured", "absent")
            .replace("measured", "absent")
            .replace("unsupported", "absent"),
        false => linux.to_string(),
    }
}

/// `rows` as [`strict_records`] reads them on this host.
pub(crate) fn expected_records(rows: &[(&str, &[&str])]) -> Vec<(String, Vec<String>)> {
    let rows = rows.iter();
    rows.map(|(site, shapes)| {
        (
            site.to_string(),
            shapes.iter().map(|s| on_this_host(s)).collect(),
        )
    })
    .collect()
}

/// What a scaffold tells its operator about MCP, on stderr and in its
/// README, pinned here once (SI2): the ruling, and that it is not yet
/// enforced.
const AMBIENT_MCP: &str = "Decision 0065 rules that a seat never inherits your own MCP \
     configuration, but this build does not enforce it yet: a seat may still start the MCP \
     servers your harness configuration names.";

/// The guides that transcribe a scaffold's output: the quickstart's
/// Step 2, which `doc_samples` runs, and the starters point to.
const TRANSCRIPTS: [&str; 1] = ["quickstart.md"];

/// Every guide that transcribes `init`'s output carries the sentence it
/// prints now, and none still says an unboxed seat is decided by the
/// operator's MCP servers.
#[test]
fn the_guides_transcribe_the_scaffolds_mcp_instruction() {
    let guides = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/guides");
    for guide in TRANSCRIPTS {
        let text = std::fs::read_to_string(guides.join(guide)).unwrap();
        assert!(text.contains(&format!("\n{AMBIENT_MCP}\n")), "{guide}");
        assert!(!text.contains("own settings and MCP servers"), "{guide}");
    }
}

/// The shapes each scaffolded model declaration can admit, by invocation
/// and the hands it declares: the scaffold gives claude no hands, codex
/// declares boxed and harness hands and no `resume`, and dsh holds none.
/// So Claude's boxed shapes and Codex's resumed one stay with the shipped
/// adapters, and each replacement U0b recorded rides beside its cold
/// shape (U1d3). Codex's hands-less shapes ride too, for an operator's
/// own inline codex seat (U1d4).
const APPLICABLE: [(&str, &[(&str, &str)]); 3] = [
    ("claude", &[("cold", "none"), ("replacement", "none")]),
    (
        "codex",
        &[
            ("cold", "boxed"),
            ("cold", "harness"),
            ("cold", "none"),
            ("replacement", "boxed"),
            ("replacement", "harness"),
            ("replacement", "none"),
        ],
    ),
    ("dsh", &[("cold", "none"), ("replacement", "none")]),
];

/// SI2's generated parity: a scaffolded model declaration carries its
/// shipped adapter's authentication names, carriage, its applicable shapes
/// and its native assessment word for word; exec carries its shipped names
/// and the declaration that it has no model MCP surface, and no strictness
/// of its own.
pub(crate) fn assert_shipped_mcp_facts(bundle: &std::path::Path, provider: &str) {
    let generated = json_at(&bundle.join(format!("adapters/{provider}.json")));
    let shipped = json_at(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../adapters/{provider}.json")),
    );
    // MB4 (U7d3 L2): every scaffold reads its harness's names as shipped.
    let auth = &shipped["harness_auth"];
    assert_eq!(generated["harness_auth"], *auth, "{provider}");
    assert!(auth["measured"].is_string(), "{provider}: {shipped}");
    let Some((_, applicable)) = APPLICABLE.iter().find(|(name, _)| *name == provider) else {
        assert_eq!(provider, "exec");
        assert_eq!(generated["mcp"], shipped["mcp"]);
        assert!(shipped["mcp"]["inapplicable"].is_string(), "{shipped}");
        return;
    };
    let shapes: Vec<&serde_json::Value> = shipped["mcp"]["shapes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|shape| {
            let key = (shape["invocation"].as_str(), shape["hands"].as_str());
            (applicable.iter()).any(|(invocation, hands)| key == (Some(*invocation), Some(*hands)))
        })
        .collect();
    assert_eq!(shapes.len(), applicable.len(), "{provider}: {shipped}");
    assert_eq!(
        generated["mcp"],
        serde_json::json!({"carriage": shipped["mcp"]["carriage"], "shapes": shapes}),
        "{provider}"
    );
    assert_eq!(
        generated["native_capabilities"], shipped["native_capabilities"],
        "{provider}"
    );
}

/// A hands-less work seat's SI2 record: judged cold and as its replacement,
/// which U0b measured for Claude, and for dsh on the keyed routes with its
/// pass 8 (U1d6); no generated declaration names a hands-less resume
/// shape. A gate, never offered a session, is judged cold alone.
const BARE_WORK: &[&str] = &["cold none measured", "replacement none measured"];
const BARE_GATE: &[&str] = &["cold none measured"];
/// A Codex seat's, boxed: U0b measured U9c2's cold launch, and its pass 7
/// the boxed replacement (U1d5).
const BOXED_WORK: &[&str] = &["cold boxed measured", "replacement boxed measured"];
const BOXED_GATE: &[&str] = &["cold boxed measured"];
/// Exec serves no model and records no shape.
const EXEC: &[&str] = &[];

/// The SI2 records a Claude roster's compile keeps, site by outcome.
pub(crate) const CLAUDE_RECORDS: [(&str, &[&str]); 8] = [
    ("implement claude", BARE_WORK),
    ("implement claude", BARE_WORK),
    ("intake claude", BARE_WORK),
    ("intake claude", BARE_WORK),
    ("review claude", BARE_GATE),
    ("review claude", BARE_GATE),
    ("ship exec", EXEC),
    ("verify exec", EXEC),
];

/// A Codex roster's, its one reviewer link included.
const CODEX_RECORDS: [(&str, &[&str]); 7] = [
    ("implement codex", BOXED_WORK),
    ("implement codex", BOXED_WORK),
    ("intake codex", BOXED_WORK),
    ("intake codex", BOXED_WORK),
    ("review codex", BOXED_GATE),
    ("ship exec", EXEC),
    ("verify exec", EXEC),
];

/// A dsh roster's, its review gate hired from Claude.
const DSH_RECORDS: [(&str, &[&str]); 8] = [
    ("implement dsh", BARE_WORK),
    ("implement dsh", BARE_WORK),
    ("intake dsh", BARE_WORK),
    ("intake dsh", BARE_WORK),
    ("review claude", BARE_GATE),
    ("review claude", BARE_GATE),
    ("ship exec", EXEC),
    ("verify exec", EXEC),
];

/// SI2 before strict admission activates (U1f2): each roster writes the
/// declarations it hires, each carrying its shipped MCP facts, and both
/// init's own compile and the compile from inside the workspace pass on
/// them with nothing granted and every native power still OFF — dsh's
/// review gate on claude included. This certifies no strictness. With the
/// fence standing (U1g1) each compile keeps every site's SI2 record, on
/// Linux each measured since dsh's replacement was (U1d6); exec records no
/// shape.
#[test]
fn each_rosters_declarations_carry_their_shipped_mcp_facts_and_still_compile() {
    for (present, written, reviewer, records) in [
        (
            "claude",
            &["claude", "exec"][..],
            "claude",
            &CLAUDE_RECORDS[..],
        ),
        ("codex", &["codex", "exec"][..], "codex", &CODEX_RECORDS[..]),
        (
            "dsh",
            &["claude", "dsh", "exec"][..],
            "claude",
            &DSH_RECORDS[..],
        ),
    ] {
        let (_dir, bundle, _) = init_with_only(&[present]);
        let mut declared: Vec<String> = std::fs::read_dir(bundle.join("adapters"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        declared.sort();
        let mut expected: Vec<String> = written.iter().map(|name| format!("{name}.json")).collect();
        expected.sort();
        assert_eq!(declared, expected, "{present}");
        for provider in written {
            assert_shipped_mcp_facts(&bundle, provider);
        }
        let claude = bundle.join("adapters/claude.json");
        if claude.exists() {
            assert!(json_at(&claude).get("hands").is_none(), "{present}");
        }
        let codex = bundle.join("adapters/codex.json");
        if codex.exists() {
            assert!(json_at(&codex).get("resume").is_none(), "{present}");
        }
        let compiled = brokkr_runtime::Bundle::compile_with(
            &bundle,
            &bundle.join("agents"),
            &bundle.join("adapters"),
        )
        .unwrap();
        assert_eq!(
            compiled.manifest["capabilities"]["grants"],
            serde_json::json!({}),
            "{present}"
        );
        assert_eq!(
            compiled.manifest["agents"]["review"]["provider"], reviewer,
            "{present}"
        );
        let mut served = std::collections::BTreeSet::new();
        for (label, facts) in &compiled.sites {
            let Some(site) = facts.capabilities.as_ref() else {
                continue;
            };
            for outcome in &site.outcomes {
                assert!(outcome.held.is_empty(), "{present}/{label}");
                served.insert(outcome.provider.clone());
                let off = match outcome.provider.as_str() {
                    "claude" => &outcome.controls()["selection"]["deny"],
                    "codex" => &outcome.controls()["argv"],
                    "dsh" => &outcome.controls()["inventory"],
                    _ => continue,
                };
                let expected = match outcome.provider.as_str() {
                    "claude" => serde_json::json!(["WebFetch", "WebSearch"]),
                    "codex" => serde_json::json!(["-c", "web_search=\"disabled\""]),
                    _ => serde_json::json!("unmeasured"),
                };
                assert_eq!(off, &expected, "{present}/{label}");
            }
        }
        assert_eq!(
            served.into_iter().collect::<Vec<_>>(),
            written,
            "{present}: every declaration written serves a site"
        );
        assert_eq!(
            strict_records(&compiled),
            expected_records(records),
            "{present}"
        );
    }
}

/// The cells a measurement's text cites, each a word one of `marks` begins
/// before a digit, in order of first mention.
fn cells<'a>(text: &'a str, marks: &[char]) -> Vec<&'a str> {
    let mut cited: Vec<&str> = Vec::new();
    let words = text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'));
    for word in words {
        let cell = word.starts_with(marks) && word[1..].starts_with(|c: char| c.is_ascii_digit());
        if cell && !cited.contains(&word) {
            cited.push(word);
        }
    }
    cited
}

/// A shape's four axes, each with the U0b cells its line cites.
type AxisCells = [(&'static str, &'static [&'static str]); 4];

/// Codex's two hands-less shapes (U1d4) and its boxed replacement (U1d5)
/// as the shipped adapter records them, which the scaffold's copy carries
/// by parity: codex-cli 0.164.0 on Linux, every axis measured, each citing
/// U0b's own cells for it, pass 7's for the boxed replacement, so a wrong
/// claim the two copies repeated still fails here.
#[test]
fn codexs_hands_less_shapes_and_boxed_replacement_cite_u0bs_cells_on_every_axis() {
    let shipped = json_at(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/codex.json"),
    );
    let pass_7: &[&str] = &["X34", "X35"];
    let expected: [(&str, &str, AxisCells); 3] = [
        (
            "cold",
            "none",
            [
                ("ambient", &["X14", "X15", "X13", "X21-legacyiso"]),
                ("native_write", &["X22", "X23", "X28"]),
                ("store_read", &["X22", "X23", "X29", "X29b"]),
                ("process_read", &["X22", "X23"]),
            ],
        ),
        (
            "replacement",
            "none",
            [
                (
                    "ambient",
                    &["X17", "X15", "X18", "X14", "X13", "X21-legacyiso"],
                ),
                ("native_write", &["X24", "X23"]),
                ("store_read", &["X24", "X29", "X29b"]),
                ("process_read", &["X24"]),
            ],
        ),
        (
            "replacement",
            "boxed",
            [
                (
                    "ambient",
                    &["X32", "X32-A", "X33", "X33-rejoin", "X13", "X21-legacyiso"],
                ),
                ("native_write", pass_7),
                ("store_read", pass_7),
                ("process_read", pass_7),
            ],
        ),
    ];
    for (invocation, hands, axes) in expected {
        let all = shipped["mcp"]["shapes"].as_array().unwrap().iter();
        let shape: Vec<&serde_json::Value> = all
            .filter(|s| s["invocation"] == invocation && s["hands"] == hands)
            .collect();
        assert_eq!(shape.len(), 1, "{invocation} {hands}");
        let shape = shape[0].as_object().unwrap();
        let keys: Vec<&str> = shape.keys().map(String::as_str).collect();
        let mut expected_keys = vec!["invocation", "hands", "measured_on"];
        expected_keys.extend(axes.iter().map(|(axis, _)| *axis));
        expected_keys.sort_unstable();
        assert_eq!(keys, expected_keys, "{invocation} {hands}");
        assert_eq!(
            shape["measured_on"],
            serde_json::json!({"harness": "codex", "binary": "codex", "version": "0.164.0", "host": "linux"}),
            "{invocation} {hands}"
        );
        for (axis, cited) in axes {
            let fact = shape[axis].as_object().unwrap();
            assert_eq!(
                fact.keys().collect::<Vec<_>>(),
                ["measured"],
                "{invocation} {hands} {axis}"
            );
            assert_eq!(
                cells(fact["measured"].as_str().unwrap(), &['X']),
                cited,
                "{invocation} {hands} {axis}"
            );
        }
    }
}

/// The shipped recipes whose Codex seats are served without hands — a
/// gate cold, a work seat cold, as its replacement and on its work-site
/// resume — and every such seat's SI2 record as each compile keeps it
/// under the standing fence: cold and replacement measured on Linux since
/// U1d4, where U9b1's first run found them absent; on macOS all absent,
/// for U0b measured Linux alone.
#[test]
fn the_shipped_hands_less_codex_seats_read_measured_where_u0b_measured_them() {
    let gate: &[&str] = &["cold none measured"];
    let work: &[&str] = &[
        "cold none measured",
        "replacement none measured",
        "work-site none measured",
    ];
    let positions = [
        ("design:positions:robustness codex", work),
        ("design:positions:simplicity codex", work),
        ("triage codex", gate),
    ];
    for (recipe, rows) in [
        ("recipes/triage", &positions[..]),
        ("recipes/gpt-flash", &[positions[0], positions[2]][..]),
        (
            "recipes/standby",
            &[("implement codex", work), ("review codex", gate)][..],
        ),
        ("recipes/wager-harness", &[("implement codex", work)][..]),
    ] {
        let codex = shipped_records(recipe, "codex", " none ");
        assert_eq!(codex, expected_records(rows), "{recipe}");
    }
}

/// The SI2 records of the shipped `recipe`'s `provider` sites, compiled
/// against the in-tree agents and adapters, that are served a shape
/// `served` matches.
fn shipped_records(recipe: &str, provider: &str, served: &str) -> Vec<(String, Vec<String>)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let compiled = brokkr_runtime::Bundle::compile_with(
        &root.join(recipe),
        &root.join("agents"),
        &root.join("adapters"),
    )
    .unwrap_or_else(|e| panic!("{recipe} must compile: {e}"));
    (strict_records(&compiled).into_iter())
        .filter(|(site, shapes)| {
            site.ends_with(&format!(" {provider}"))
                && shapes.iter().any(|shape| shape.contains(served))
        })
        .collect()
}

/// The shipped recipes whose Codex work seats are served with hands boxed
/// — chief-architect's and release-manager's sol, hired with workspace
/// hands, and triage's and night-shift's other boxed Codex work seats —
/// and each such seat's SI2 record as each compile keeps it under
/// the standing fence: cold and replacement measured on Linux since U1d5,
/// where U9b1's second run found the replacement unmeasured; on macOS all
/// absent, for U0b measured Linux alone.
#[test]
fn the_shipped_boxed_codex_work_seats_read_their_replacement_measured() {
    let chief = ("design:chief codex", BOXED_WORK);
    let author = ("specify:author codex", BOXED_WORK);
    let engine = ("implement:engine codex", BOXED_WORK);
    for (recipe, rows) in [
        (
            "recipes/triage",
            &[chief, chief, engine, author, author][..],
        ),
        ("recipes/night-shift", &[chief, chief, author, author][..]),
        ("recipes/release", &[("implement codex", BOXED_WORK)][..]),
    ] {
        let codex = shipped_records(recipe, "codex", "replacement boxed ");
        assert_eq!(codex, expected_records(rows), "{recipe}");
    }
}

/// dsh's replacement without hands as the shipped adapter records it,
/// which the scaffold's copy carries by parity: dsh 0.2.1-rc.2 on Linux,
/// its ambient axis measured from U0b D06 and pass 8's four keyed B cells,
/// naming spark unqualified as for the cold shape (U1d6), and its other
/// three axes still the D07 lines U1d3 declared unmeasured.
#[test]
fn dshs_replacement_cites_d06_and_pass_8_on_its_ambient_axis_alone() {
    let shipped =
        json_at(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/dsh.json"));
    let all = shipped["mcp"]["shapes"].as_array().unwrap().iter();
    let shape: Vec<&serde_json::Value> = all
        .filter(|s| s["invocation"] == "replacement" && s["hands"] == "none")
        .collect();
    assert_eq!(shape.len(), 1);
    let shape = shape[0];
    assert_eq!(
        shape["measured_on"],
        serde_json::json!({"harness": "dsh", "binary": "dsh", "version": "0.2.1-rc.2", "host": "linux"})
    );
    let verdicts: Vec<(&str, Vec<&str>)> =
        ["ambient", "native_write", "store_read", "process_read"]
            .into_iter()
            .map(|axis| {
                let fact = shape[axis].as_object().unwrap();
                let (verdict, text) = fact.iter().next().unwrap();
                assert_eq!(fact.len(), 1, "{axis}");
                let mut cited = vec![verdict.as_str()];
                cited.extend(cells(text.as_str().unwrap(), &['D', 'P', 'S']));
                (axis, cited)
            })
            .collect();
    let d07 = vec!["unmeasured", "D07"];
    assert_eq!(
        verdicts,
        [
            (
                "ambient",
                vec![
                    "measured",
                    "D06",
                    "P8-deepseek-B",
                    "P8-dashscope-B",
                    "P8-meta-B",
                    "P8-metac-B",
                    "S01",
                    "S03",
                ]
            ),
            ("native_write", d07.clone()),
            ("store_read", vec!["unmeasured", "D07", "D10-p5"]),
            ("process_read", d07),
        ]
    );
}

/// The shipped recipes whose dsh work seats U9b1's third run found refused
/// past a lifted fence, and each dsh site's SI2 record as each compile
/// keeps it: cold and replacement measured on Linux since U1d6; on macOS
/// both absent, for U0, U0b and U0c measured Linux alone. Its declared
/// headless-work resume is `unmeasured`, never performed, so it is no
/// served shape and has no record (U9b1, operator ruling (2) of
/// 2026-10-28).
#[test]
fn the_shipped_dsh_work_seats_read_their_replacement_measured() {
    let work: &[&str] = &["cold none measured", "replacement none measured"];
    let implement = [("implement dsh", work)];
    let gpt_flash = [
        "design:positions:simplicity",
        "implement:chore",
        "implement:design",
        "implement:engine",
        "implement:feature",
        "review:chore:positions:security",
        "review:design:positions:security",
        "review:design:positions:spec-compliance",
        "review:engine:positions:security",
        "review:engine:positions:spec-compliance",
        "review:feature:positions:security",
    ]
    .map(|seat| format!("{seat} dsh"));
    let gpt_flash: Vec<(&str, &[&str])> = gpt_flash.iter().map(|s| (s.as_str(), work)).collect();
    for (recipe, rows) in [
        ("recipes/night-shift", &implement[..]),
        ("recipes/research-dsh", &[("research dsh", work)][..]),
        ("recipes/gpt-flash", &gpt_flash[..]),
        ("recipes/review-first", &implement[..]),
        ("recipes/wager-harness-muse", &implement[..]),
        ("recipes/wager-harness-dsh", &implement[..]),
    ] {
        let dsh = shipped_records(recipe, "dsh", "replacement none ");
        assert_eq!(dsh, expected_records(rows), "{recipe}");
    }
}

/// A claude-less host with codex gets a codex scaffold that compiles: the
/// codex adapter and no claude one, every seat hired from codex with
/// hands and no tool allowance, and a realm under the host's boundary —
/// `namespace` on Linux, where Brokkr's workspace tool serves those hands
/// (#717), and `harness` on macOS, where codex's own sandbox classes hold
/// them.
#[test]
fn a_codex_only_host_gets_a_codex_scaffold_that_compiles() {
    let (_dir, bundle, stderr) = init_with_only(&["codex"]);
    assert!(
        stderr
            .contains("Scaffolded for `codex`, the first of claude, codex and dsh found on PATH."),
        "{stderr}"
    );
    // Under `namespace` every seat with hands, each codex office among
    // them, needs the bubblewrap this PATH lacks, and is named (#717).
    let warned = "hands need bubblewrap: no `bwrap` on PATH, and the boundary is never \
                  simulated; the scaffolded seats [\"implement\", \"intake\", \"review\", \
                  \"ship\", \"verify\"] declare hands";
    assert_eq!(
        stderr.contains(warned),
        cfg!(target_os = "linux"),
        "{stderr}"
    );
    assert!(bundle.join("adapters/codex.json").is_file());
    assert!(!bundle.join("adapters/claude.json").exists());
    let adapter = json_at(&bundle.join("adapters/codex.json"));
    assert_eq!(adapter["trust_tier"], "trusted");
    assert_eq!(adapter["judges"], serde_json::json!(["astra", "sol"]));
    let map = json_at(&bundle.join("realms.json"));
    let declared = match cfg!(target_os = "macos") {
        true => ("forge.realms/v4", Some("harness")),
        false => ("forge.realms/v3", None),
    };
    let boundary = map["realms"][0]
        .get("boundary")
        .and_then(|word| word.as_str());
    assert_eq!(
        (map["schema"].as_str().unwrap(), boundary),
        declared,
        "{map}"
    );
    // Astra holds no scaffold chain, and sol hires one step below the
    // scaffold's `high` (decision 0045's 2026-09-30 addendum).
    let sol_terra = serde_json::json!({"sol": "medium", "terra": "high"});
    for (agent, models, efforts) in [
        ("intake", &["sol", "terra"][..], &sol_terra),
        ("implementer", &["sol", "terra"][..], &sol_terra),
        (
            "reviewer",
            &["sol"][..],
            &serde_json::json!({"sol": "medium"}),
        ),
    ] {
        let definition = json_at(&bundle.join(format!("agents/{agent}.json")));
        assert_eq!(definition["models"], serde_json::json!(models), "{agent}");
        assert_eq!(&definition["efforts"], efforts, "{agent}");
        assert_eq!(definition["hands"]["kind"], "workspace", "{agent}");
        assert!(definition.get("tools").is_none(), "{agent}: {definition}");
    }
    // Its README speaks of the adapters it wrote, and of no tool map.
    let readme = std::fs::read_to_string(bundle.join("agents/README.md")).unwrap();
    assert!(!readme.contains("claude.json"), "{readme}");
    assert!(
        readme.contains("## Tool grants\n\nNone. No seat is hired from claude"),
        "{readme}"
    );
}

/// On macOS a codex scaffold declares `harness`, where verify runs under
/// no box of Brokkr's: the note its script journals on a pass says so, and
/// claims no denied network (#366). On Linux it stands under `namespace`
/// (#717), whose note `init`'s own host-arm table pins.
#[cfg(target_os = "macos")]
#[test]
fn a_harness_scaffolds_verify_note_claims_no_denied_network() {
    let (_dir, bundle, _) = init_with_only(&["codex"]);
    let script = std::fs::read_to_string(bundle.join("scripts/verify-seat.sh")).unwrap();
    let notes: Vec<&str> = script
        .lines()
        .filter(|line| line.contains(" passed "))
        .collect();
    let unboxed = "printf '%s and %s passed unboxed, with no network denial confirmed' \
                   \"$test_command\" \"$lint_command\" > \"$notes\"";
    assert_eq!(notes, [unboxed]);
    assert!(!script.contains("network denied"), "{script}");
}

/// A run writes its journal, results and ledger under `.brokkr/state/`,
/// and the ship gate closes out only on a clean tree: `init` ignores the
/// folder's state from inside, with the one line `state/`, and the commit
/// carries that ignore file to every clone (decision 0083 ruling 1).
#[test]
fn init_ignores_the_folders_own_state() {
    let repo = Repo::new();
    let ran = repo.brokkr("", &["init", "."]);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let ignore = std::fs::read_to_string(repo.path(".brokkr/.gitignore")).unwrap();
    assert_eq!(ignore, "state/\n");
    git(&repo.root, &["add", "-A"]);
    crate::brokkr_folder::commit(&repo.root);
    let tracked = git(
        &repo.root,
        &["ls-files", ".brokkr/.gitignore", ".brokkr/state"],
    );
    assert_eq!(tracked, ".brokkr/.gitignore\n");
    repo.write(".brokkr/state/forge.db", "");
    repo.write(".brokkr/state/results/seat.json", "{}");
    assert_eq!(git(&repo.root, &["status", "--porcelain"]), "");
}

/// A dsh-only host: dsh holds intake and implement, and the review gate —
/// which an untrusted adapter with no judges can never hold — stays on
/// claude, and the output says so.
#[test]
fn a_dsh_only_host_names_the_gate_dsh_cannot_hold() {
    let (_dir, bundle, stderr) = init_with_only(&["dsh"]);
    assert!(
        stderr.contains("The review gate cannot be held by dsh: its adapter is untrusted"),
        "{stderr}"
    );
    let readme = std::fs::read_to_string(bundle.join("agents/README.md")).unwrap();
    assert!(
        readme.contains("The review gate cannot be held by dsh"),
        "{readme}"
    );
    for adapter in ["dsh", "claude", "exec"] {
        assert!(bundle.join(format!("adapters/{adapter}.json")).is_file());
    }
    assert!(!bundle.join("adapters/codex.json").exists());
    assert_eq!(
        json_at(&bundle.join("adapters/dsh.json"))["trust_tier"],
        "untrusted"
    );
    for (agent, models) in [
        ("intake", ["flash", "pro"]),
        ("implementer", ["pro", "flash"]),
        ("reviewer", ["fable", "opus"]),
    ] {
        let definition = json_at(&bundle.join(format!("agents/{agent}.json")));
        assert_eq!(definition["models"], serde_json::json!(models), "{agent}");
        assert!(definition.get("hands").is_none(), "{agent}: {definition}");
    }
}

/// On macOS the scaffold's realm declares `harness`, because `namespace`
/// cannot be built there: nothing asks for bubblewrap, and the output
/// says which boundary was written and why.
#[cfg(target_os = "macos")]
#[test]
fn a_macos_scaffold_declares_harness_and_asks_nothing_of_bubblewrap() {
    let (dir, stderr) = init_on_an_empty_path();
    assert!(!stderr.contains("hands need bubblewrap"), "{stderr}");
    assert!(
        stderr.contains("`realms.json` declares the `harness` boundary"),
        "{stderr}"
    );
    let map = std::fs::read_to_string(dir.path().join(".brokkr/realms.json")).unwrap();
    assert!(map.contains("\"boundary\": \"harness\""), "{map}");
}

/// `init .` in a fresh repository under a PATH that holds nothing: the
/// repository, and what `init` printed.
fn init_on_an_empty_path() -> (tempfile::TempDir, String) {
    let dir = repository();
    let empty_path = dir.path().join("empty-path");
    std::fs::create_dir(&empty_path).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_brokkr"))
        .args(["init", "."])
        .env("PATH", &empty_path)
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert_eq!(out.status.code(), Some(0), "{stderr}");
    (dir, stderr)
}

#[cfg(target_os = "linux")]
#[test]
fn init_names_the_scaffolded_seats_that_need_missing_bubblewrap() {
    let (_dir, stderr) = init_on_an_empty_path();
    assert!(
        stderr.contains(
            "hands need bubblewrap: no `bwrap` on PATH, and the boundary is never simulated"
        ),
        "{stderr}"
    );
    assert!(
        stderr.contains("scaffolded seats [\"ship\", \"verify\"] declare hands"),
        "{stderr}"
    );
    // Decision 0046: the warning speaks the boundary vocabulary instead
    // of saying the shipped gates require Linux.
    assert!(
        stderr.contains("run under the realm's boundary"),
        "{stderr}"
    );
    assert!(
        stderr.contains("`namespace`, the default, needs bubblewrap on PATH"),
        "{stderr}"
    );
    assert!(
        stderr.contains("a realm may declare `harness` instead (decision 0046)"),
        "{stderr}"
    );
    assert!(!stderr.contains("require Linux"), "{stderr}");
}

#[test]
fn doctor_checks_the_map_named_by_realms() {
    let dir = tempfile::tempdir().unwrap();
    let map = dir.path().join("fleet.json");
    std::fs::write(
        &map,
        serde_json::json!({
            "schema": "forge.realms/v3",
            "realms": [{"name": "app", "path": ".", "default_branch": "main",
                        "house": "missing-house.md"}],
            "journal": "forge.db"
        })
        .to_string(),
    )
    .unwrap();
    let (code, stdout, stderr) = brokkr(&["doctor", "--realms", map.to_str().unwrap()], dir.path());
    assert_eq!(code, Some(1), "stderr: {stderr}");
    assert!(stdout.contains("realm 'app' names house"), "{stdout}");
    assert!(stdout.contains("missing-house.md"), "{stdout}");
}

#[test]
fn doctor_prints_scaffolded_dialect_absence_and_missing_tool_warning() {
    let absent = repository();
    let (code, _, stderr) = brokkr(&["init", "."], absent.path());
    assert_eq!(code, Some(0), "{stderr}");
    let (_, stdout, _) = brokkr(&["doctor"], absent.path());
    assert!(
        stdout.contains("ok       dialect starter: none declared"),
        "{stdout}"
    );

    let openspec = repository();
    std::fs::create_dir(openspec.path().join("openspec")).unwrap();
    std::fs::write(
        openspec.path().join("openspec/config.yaml"),
        "schema: spec-driven\n",
    )
    .unwrap();
    let (code, _, stderr) = brokkr(&["init", "."], openspec.path());
    assert_eq!(code, Some(0), "{stderr}");
    let empty_path = openspec.path().join("empty-path");
    std::fs::create_dir(&empty_path).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_brokkr"))
        .arg("doctor")
        .env("PATH", &empty_path)
        .current_dir(openspec.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("warn     dialect starter: openspec"),
        "{stdout}"
    );
    assert!(
        stdout.contains("tool binary 'openspec' not found"),
        "{stdout}"
    );
    assert!(
        stdout.contains("the design route will refuse to run"),
        "{stdout}"
    );
    assert!(
        stdout.contains("dialect starter requires openspec/config.yaml: present"),
        "{stdout}"
    );
}

/// `init` writes a folder only where none stands, or beside moved state
/// (decision 0083 ruling 5), so nothing an operator holds in one is ever
/// written over: a prior scaffold's bundle, a trust tier they demoted
/// (decision 0021 ruling 3), a seat script, a map, dialect data, or a
/// third entry beside moved state. Each is refused naming what the folder
/// holds, kept byte for byte, and nothing else is written (R1).
#[test]
fn init_refuses_an_existing_folder() {
    let cases = [
        ("bundle.json", "{\"seats\": {}}\n", "bundle.json"),
        (
            "adapters/claude.json",
            "{\"trust_tier\": \"untrusted\"}\n",
            "adapters",
        ),
        ("adapters/exec.json", "operator-owned\n", "adapters"),
        ("scripts/verify-seat.sh", "operator-owned\n", "scripts"),
        ("scripts/ship-seat.sh", "operator-owned\n", "scripts"),
        ("realms.json", "operator-owned\n", "realms.json"),
        ("dialects/openspec.json", "operator-owned\n", "dialects"),
        (
            "dialects/openspec/specify.md",
            "operator-owned instruction\n",
            "dialects",
        ),
        ("notes.md", "operator-owned\n", "notes.md"),
    ];
    for (planted, theirs, held) in cases {
        let repo = Repo::new();
        repo.write("openspec/config.yaml", "schema: spec-driven\n");
        repo.write(".brokkr/.gitignore", "state/\n");
        repo.write(".brokkr/state/forge.db", "");
        let path = repo.write(&format!(".brokkr/{planted}"), theirs);
        let before = repo.files();
        let ran = repo.brokkr("", &["init", "."]);
        let refused = format!(
            "error: {}/.brokkr already holds {held} (decision 0083); init writes a folder only \
             where none stands, or beside moved state (.gitignore and state/ alone): move or \
             remove them\n",
            repo.root.display()
        );
        assert_eq!((ran.code, ran.stderr), (Some(1), refused), "{planted}");
        assert_eq!(std::fs::read_to_string(path).unwrap(), theirs, "{planted}");
        assert_eq!(repo.files(), before, "{planted}");
    }
}

/// The one folder `init` writes beside: the moved state the state-only
/// refusal's moves leave, its ignore file kept byte for byte and left off
/// the list of what was written, and its `state/` untouched.
#[test]
fn init_writes_beside_moved_state() {
    let repo = Repo::new();
    let theirs = "state/\n# the operator's\n";
    repo.write(".brokkr/.gitignore", theirs);
    repo.write(".brokkr/state/forge.db", "journal");
    let ran = repo.brokkr("", &["init", "."]);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let read = |path: &str| std::fs::read_to_string(repo.path(path)).unwrap();
    assert_eq!(read(".brokkr/.gitignore"), theirs);
    assert_eq!(read(".brokkr/state/forge.db"), "journal");
    assert!(repo.path(".brokkr/bundle.json").is_file());
    let listed = ran
        .stderr
        .lines()
        .any(|line| line.ends_with(".brokkr/.gitignore"));
    assert!(!listed, "{}", ran.stderr);
}

/// A folder is read only at the root (ruling 2), so `init` of a directory
/// below it refuses, whichever way it is named, and writes no folder in
/// either place: it never moves the target up on its own.
#[test]
fn init_from_a_subdirectory_refuses() {
    let repo = Repo::new();
    let root = repo.root.display();
    let refused = format!(
        "error: {} is inside the repository {root} but is not its root (decision 0083); a folder \
         is read only at the root: run `brokkr init {root}`\n",
        repo.path("src").display()
    );
    for (cwd, dir) in [("src", "."), ("", "src")] {
        let ran = repo.brokkr(cwd, &["init", dir]);
        assert_eq!(
            (ran.code, ran.stderr.as_str()),
            (Some(1), refused.as_str()),
            "{dir}"
        );
    }
    assert!(!repo.path("src/.brokkr").exists() && !repo.path(".brokkr").exists());
}

/// A recipe library keeps its own layout and reads no folder (the
/// operator's ruling 1 of 2026-10-13), so `init` writes nothing there (R2).
#[test]
fn init_refuses_in_a_library() {
    let repo = Repo::new();
    repo.library();
    let before = repo.files();
    let ran = repo.brokkr("", &["init", "."]);
    let refused = format!(
        "error: {} is a recipe library, which keeps its own layout and reads no .brokkr/ folder \
         (decision 0083; the operator's ruling 1 of 2026-10-13): init writes nothing here\n",
        repo.root.display()
    );
    assert_eq!((ran.code, ran.stderr), (Some(1), refused));
    assert_eq!(repo.files(), before);
    assert!(!repo.path(".brokkr").exists());
}

/// One form of the folder line: where doctor runs, what the repository
/// holds, and the line it earns, given the root.
type Form = (&'static str, fn(&Repo), fn(&str) -> String);

/// Doctor's first line is the folder, in its status column (decision 0083
/// ruling 5): found and from where, none, a library's own layout, and an
/// old shape or a discovery refusal in its refusal's first words, each of
/// which leaves the report unhealthy (R4).
#[test]
fn doctor_names_the_folder() {
    let rows: [Form; 7] = [
        (
            "",
            |repo| assert_eq!(repo.brokkr("", &["init", "."]).code, Some(0)),
            |root| format!("ok       folder: {root}/.brokkr (found from {root})"),
        ),
        (
            "",
            |_| {},
            |root| {
                format!("warn     folder: none from {root} up to {root} — brokkr init . writes one")
            },
        ),
        ("", Repo::library, |root| {
            format!(
                "ok       folder: none: {root} is a recipe library, which keeps its own layout \
                     (decision 0083)"
            )
        }),
        (
            "",
            |repo| drop(repo.write("bundle.json", "{}")),
            |_| missing(ROOT_LAYOUT),
        ),
        (
            "",
            |repo| drop(repo.write(".forge/secrets.env", "")),
            |_| missing(STATE_ONLY),
        ),
        (
            "",
            |repo| drop(repo.write(".brokkr", "a file")),
            |root| {
                format!(
                    "MISSING  folder: {root}/.brokkr is not a directory (decision 0083); Brokkr's \
                     folder must be one: move or remove it"
                )
            },
        ),
        (
            "src/deep",
            |repo| std::fs::create_dir_all(repo.path("src/.brokkr")).unwrap(),
            |root| {
                format!(
                    "MISSING  folder: {root}/src/.brokkr is not at this repository's root {root} \
                     (decision 0083); Brokkr reads only {root}/.brokkr: move or remove it"
                )
            },
        ),
    ];
    for (cwd, plant, expected) in rows {
        let repo = Repo::new();
        plant(&repo);
        let doctor = repo.brokkr(cwd, &["doctor"]);
        let expected = expected(&repo.root.display().to_string());
        assert_eq!(doctor.stdout.lines().next(), Some(expected.as_str()));
        if expected.starts_with("MISSING") {
            assert_eq!(doctor.code, Some(1), "{expected}");
        }
    }
}

/// Doctor's folder line for an old shape: its refusal's first line.
fn missing(refusal: &str) -> String {
    format!("MISSING  folder: {}", refusal.trim_start_matches("error: "))
}

/// Doctor checks a hand move (decision 0083 ruling 5): with `init`'s new
/// output and a root `bundle.json` left beside it, the folder line names
/// the entry left; with that gone and a charter left behind at the root,
/// it carries the error `brokkr compile --bundle .brokkr` prints for the
/// folder, and so with the charter back and a folder map that will not
/// read. Each leaves the report unhealthy.
#[test]
fn doctor_checks_a_moved_folder() {
    let repo = Repo::new();
    let ran = repo.brokkr("", &["init", "."]);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    repo.write("bundle.json", "{}");
    let found = format!(
        "{}/.brokkr (found from {})",
        repo.root.display(),
        repo.root.display()
    );
    let left = format!(
        "MISSING  folder: {found}, but {} still stands at the root (decision 0083); move or \
         remove it\n",
        repo.path("bundle.json").display()
    );
    let doctor = repo.brokkr("", &["doctor"]);
    assert_eq!(doctor.code, Some(1));
    assert!(doctor.stdout.starts_with(&left), "{}", doctor.stdout);

    std::fs::remove_file(repo.path("bundle.json")).unwrap();
    let charter = "agents/charters/implementer.md";
    std::fs::create_dir_all(repo.path("agents/charters")).unwrap();
    std::fs::rename(repo.path(&format!(".brokkr/{charter}")), repo.path(charter)).unwrap();
    let refuses_as_compile = || {
        let compile = repo.brokkr("", &["compile", "--bundle", ".brokkr"]);
        assert_eq!(compile.code, Some(1));
        let refused = compile.stderr.strip_prefix("error: ").unwrap();
        let clause =
            format!("MISSING  folder: {found}, but its compile refuses (decision 0083): {refused}");
        let doctor = repo.brokkr("", &["doctor"]);
        assert_eq!(doctor.code, Some(1));
        assert!(
            doctor.stdout.starts_with(&clause),
            "{clause}\n{}",
            doctor.stdout
        );
    };
    refuses_as_compile();

    std::fs::rename(repo.path(charter), repo.path(&format!(".brokkr/{charter}"))).unwrap();
    repo.write(".brokkr/realms.json", "{invalid json");
    let map = format!(
        "{} is not a readable realms map: key must be a string at line 1 column 2",
        repo.path(".brokkr/realms.json").display()
    );
    let compile = repo.brokkr("", &["compile", "--bundle", ".brokkr"]);
    assert_eq!(compile.stderr, format!("error: {map}\n"));
    refuses_as_compile();
}

/// R5 (ruled 2026-10-14): with a folder found and no `--bundle`, the
/// folder is doctor's bundle for the whole report, which is the report
/// `doctor --bundle .brokkr` prints, its folder line and its one bundle
/// line included. A folder whose compile refuses is
/// compiled once, so its refusal is named once, in the folder line, and
/// it is still a bundle that did not compile: a credential set in the
/// environment reads as `doctor --bundle .brokkr` reads it, never as no
/// bundle given (decision 0040 ruling 4).
#[test]
fn doctor_reads_the_found_folder_as_its_bundle() {
    let repo = Repo::new();
    assert_eq!(repo.brokkr("", &["init", "."]).code, Some(0));
    let named = repo.brokkr("", &["doctor", "--bundle", ".brokkr"]);
    let found = repo.brokkr("", &["doctor"]);
    let bundle = "\nok       bundle: 'starter' compiles, digest ";
    assert_eq!(found.stdout.matches(bundle).count(), 1, "{}", found.stdout);
    assert_eq!((found.code, found.stdout), (named.code, named.stdout));

    let dsh = crate::workspace_root::workspace().join("adapters/dsh.json");
    std::fs::copy(dsh, repo.path(".brokkr/adapters/dsh.json")).unwrap();
    std::fs::remove_file(repo.path(".brokkr/policy.json")).unwrap();
    let compile = repo.brokkr("", &["compile", "--bundle", ".brokkr"]);
    let refused = compile.stderr.strip_prefix("error: ").unwrap().trim_end();
    let ambient = |args: &[&str]| {
        let mut doctor = repo.command("");
        doctor.args(args).env("SPARK_API_KEY", "review-placeholder");
        String::from_utf8(doctor.output().unwrap().stdout).unwrap()
    };
    let route = |stdout: &str| {
        let route = stdout
            .lines()
            .find(|line| line.contains("route spark-glm:"));
        route.unwrap_or_else(|| panic!("{stdout}")).to_string()
    };
    let doctor = repo.brokkr("", &["doctor"]);
    let root = repo.root.display();
    let clause = format!(
        "MISSING  folder: {root}/.brokkr (found from {root}), but its compile refuses (decision \
         0083): {refused}\n"
    );
    assert!(doctor.stdout.starts_with(&clause), "{}", doctor.stdout);
    assert_eq!(
        (doctor.code, doctor.stdout.matches(refused).count()),
        (Some(1), 1)
    );
    let did_not_compile = format!(
        "warn     route spark-glm: credential 'SPARK_API_KEY' is satisfied from the process \
         environment — an ambient value is journaled nowhere and moves no digest (decision 0036 \
         ruling 5); the bundle given does not compile, so it declares no seats to ask and this \
         checked membership of the bindings store at {root}/.brokkr/state/secrets.env and not whether \
         any seat binds it (decision 0040 ruling 4)"
    );
    let found = ambient(&["doctor"]);
    assert_eq!(found.matches(refused).count(), 1, "{found}");
    let named = route(&ambient(&["doctor", "--bundle", ".brokkr"]));
    assert_eq!(
        (route(&found), named),
        (did_not_compile.clone(), did_not_compile)
    );
}

/// R5 where the map cannot be read at all: what each realm grants is
/// unknown (design D8), so the folder is compiled under no map in its
/// place neither given nor found, and its compile is a run's, refused by
/// the map. `doctor --bundle .brokkr` names that refusal on its bundle
/// line; with the folder found, the folder line alone names it, and the
/// report is otherwise the same. No compile is reached past the map: a
/// folder that would refuse on its own too reads exactly as before.
#[test]
fn a_folder_under_a_map_that_cannot_be_read_is_compiled_under_no_map() {
    let repo = Repo::new();
    assert_eq!(repo.brokkr("", &["init", "."]).code, Some(0));
    let dsh = crate::workspace_root::workspace().join("adapters/dsh.json");
    std::fs::copy(dsh, repo.path(".brokkr/adapters/dsh.json")).unwrap();
    repo.write(".brokkr/realms.json", "{}");
    let doctor = |args: &[&str]| {
        let mut doctor = repo.command("");
        doctor.args(args).env("SPARK_API_KEY", "review-placeholder");
        let out = doctor.output().unwrap();
        (out.status.code(), String::from_utf8(out.stdout).unwrap())
    };
    let (code, named) = doctor(&["doctor", "--bundle", ".brokkr"]);
    let spelled = format!(
        "\nMISSING  bundle: .brokkr: {}/.brokkr/realms.json is not a readable realms map: \
         missing field `schema`",
        repo.root.display()
    );
    let compiles = "\nok       bundle: 'starter' compiles, digest ";
    let counts = (
        named.matches(&spelled).count(),
        named.matches(compiles).count(),
    );
    assert_eq!(counts, (1, 0), "{named}");
    assert_eq!(doctor(&["doctor"]), (code, named.replace(&spelled, "")));

    std::fs::remove_file(repo.path(".brokkr/policy.json")).unwrap();
    assert_eq!(doctor(&["doctor", "--bundle", ".brokkr"]), (code, named));
}

/// Decision 0021, from the operator's side: the scaffold's gate seats
/// stand on a declaration in the operator's own tree, so demoting the
/// tier there refuses the very next compile — naming the seat and the
/// driver — rather than quietly leaving three judges unbacked.
#[test]
fn demoting_the_scaffolded_tier_refuses_the_scaffolded_gates() {
    let dir = repository();
    let bundle = dir.path().join(".brokkr");
    brokkr(&["init", "."], dir.path());

    let adapter = bundle.join("adapters/claude.json");
    let declared = std::fs::read_to_string(&adapter).unwrap();
    std::fs::write(
        &adapter,
        declared.replace(
            "\"trust_tier\": \"trusted\"",
            "\"trust_tier\": \"untrusted\"",
        ),
    )
    .unwrap();

    let (code, _, stderr) = brokkr(&["compile", "--bundle", "."], &bundle);
    assert_eq!(code, Some(1), "stderr: {stderr}");
    assert!(
        stderr.contains("gate class") && stderr.contains("claude"),
        "stderr: {stderr}"
    );
}

/// Decision 0075 ruling 5, end to end: the scaffold's work
/// offices hire `sonnet`, so marking it provisional refuses the bundle
/// until the workspace's own map lists those offices — the list reaches
/// the compile from `realms.json`, and doctor names what it may hold.
#[test]
fn a_provisional_model_compiles_only_in_the_offices_the_map_lists() {
    let (repo, bundle, _) = init_with_only(&["claude"]);
    mark_sonnet_provisional(&bundle);
    let db = repo.path("forge.db");
    let doctor = || {
        brokkr(
            &["doctor", "--bundle", ".", "--db", db.to_str().unwrap()],
            &bundle,
        )
    };

    let (code, stdout, _) = doctor();
    assert_eq!(code, Some(1), "doctor output: {stdout}");
    assert!(
        stdout.contains(
            "warn     provisional sonnet: adapter 'claude' · may hold no office: realms.json \
             lists no provisional_offices · never a gate"
        ),
        "doctor output: {stdout}"
    );
    let refused = stdout
        .lines()
        .find(|line| line.starts_with("MISSING  bundle"))
        .unwrap();
    assert!(
        refused.contains("seats model 'sonnet', which adapter 'claude' declares provisional"),
        "{refused}"
    );

    list_provisional_offices(&bundle, serde_json::json!(["implementer", "intake"]));
    let (code, stdout, _) = doctor();
    assert_eq!(code, Some(0), "doctor output: {stdout}");
    assert!(
        stdout.contains(
            "ok       provisional sonnet: adapter 'claude' · may hold implementer, intake · \
             never a gate"
        ),
        "doctor output: {stdout}"
    );
    assert!(
        stdout.contains("ok       bundle: '"),
        "doctor output: {stdout}"
    );
}

/// The recipe verbs judge a recipe under the list the run reads: `list`
/// and `show` admit the scaffold's provisional `sonnet` in the offices the
/// workspace map lists, and `show` refuses it once the map lists none —
/// the refusal printed once, on the one error line. `add` of the folder's
/// own bundle refuses, because the shelf it installs onto lies inside it
/// (#696), so the library is planted as it copied one.
#[test]
fn the_recipe_verbs_read_the_provisional_offices_the_map_lists() {
    let (repo, bundle, _) = init_with_only(&["claude"]);
    mark_sonnet_provisional(&bundle);
    list_provisional_offices(&bundle, serde_json::json!(["implementer", "intake"]));
    let library = repo.path("library");
    let verb = |args: &[&str]| brokkr(&[&["recipes"], args].concat(), &bundle);

    let starter = ["add", bundle.to_str().unwrap(), "--name", "starter"];
    let (code, _, stderr) = verb(&starter);
    let within = format!(
        "error: recipes add would copy {} into {}/recipes, which lies inside it (#696): install \
         a bundle directory that does not hold the shelf; nothing was written\n",
        bundle.display(),
        bundle.display()
    );
    assert_eq!((code, stderr), (Some(1), within));
    assert!(!bundle.join("recipes").exists());
    crate::machine_proof::copy_dir(&bundle, &library.join("starter"));
    let library = library.to_str().unwrap();
    let (code, stdout, stderr) = verb(&["list", "--dir", library]);
    assert_eq!(code, Some(0), "{stderr}");
    assert!(
        stdout.lines().any(|line| line.starts_with("starter\t")),
        "{stdout}"
    );
    let (code, _, stderr) = verb(&["show", "starter", "--dir", library]);
    assert_eq!(code, Some(0), "{stderr}");

    list_provisional_offices(&bundle, serde_json::json!([]));
    let (code, _, stderr) = verb(&["show", "starter", "--dir", library]);
    assert_eq!(code, Some(1));
    assert_eq!(
        stderr,
        "error: bundle: seat 'implement' link 2 seats model 'sonnet', which adapter 'claude' \
         declares provisional, in office 'implementer', which realms.json does not list in \
         provisional_offices; a provisional model holds only the offices the operator lists, \
         and an absent or empty list names none (decision 0075 ruling 5)\n"
    );
}

/// Mark the claude scaffold's `sonnet` provisional in its own adapter.
fn mark_sonnet_provisional(bundle: &std::path::Path) {
    let adapter = bundle.join("adapters/claude.json");
    let mut claude = json_at(&adapter);
    let id = claude["models"]["sonnet"].clone();
    claude["models"]["sonnet"] = serde_json::json!({"id": id, "tier": "provisional"});
    std::fs::write(&adapter, claude.to_string()).unwrap();
}

/// Write `offices` as the scaffold map's provisional offices, under the
/// version that admits them.
fn list_provisional_offices(bundle: &std::path::Path, offices: serde_json::Value) {
    let map = bundle.join("realms.json");
    let mut world = json_at(&map);
    world["schema"] = serde_json::json!("forge.realms/v7");
    world["provisional_offices"] = offices;
    std::fs::write(&map, world.to_string()).unwrap();
}

#[test]
fn doctor_reports_health_and_validates_a_bundle() {
    let dir = repository();
    let bundle = dir.path().join(".brokkr");
    brokkr(&["init", "."], dir.path());

    let (code, stdout) = doctor_of(&bundle);
    // git and python3 exist on dev and CI machines; claude may only warn.
    assert_eq!(code, Some(0), "doctor output: {stdout}");
    assert!(stdout.contains("git"));
    assert!(stdout.contains("bundle"));
    // The pinned contract versions are part of the health report.
    assert!(
        stdout.contains("contracts: engine")
            && stdout.contains("event_schema")
            && stdout.contains("database_schema")
            && stdout.contains("driver_protocol"),
        "doctor output: {stdout}"
    );
    assert!(!stdout.contains("MISSING  git"));

    // A broken bundle turns the report unhealthy.
    std::fs::write(bundle.join("policy.json"), "{}").unwrap();
    let (code, stdout) = doctor_of(&bundle);
    assert_eq!(code, Some(1), "doctor output: {stdout}");
    assert!(stdout.contains("MISSING  bundle"));
}

/// `brokkr doctor` of the folder `bundle`, run from inside it, with a
/// journal named beside it: the exit code and the report.
fn doctor_of(bundle: &std::path::Path) -> (Option<i32>, String) {
    let db = bundle.with_file_name("forge.db");
    let doctor = ["doctor", "--bundle", ".", "--db", db.to_str().unwrap()];
    let (code, stdout, _) = brokkr(&doctor, bundle);
    (code, stdout)
}

#[test]
fn doctor_names_every_unpinned_model_seat_and_the_single_repair() {
    let dir = repository();
    let bundle = dir.path().join(".brokkr");
    brokkr(&["init", "."], dir.path());
    let bundle_file = bundle.join("bundle.json");
    let mut config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&bundle_file).unwrap()).unwrap();
    for (phase, charter, kind) in [
        ("implement", "implementer", "claude"),
        ("verify", "reviewer", "codex"),
    ] {
        let seat = &mut config["seats"][phase];
        // The scaffold's verify seat has hands. On macOS, init declares
        // the `harness` boundary, which refuses an inline site with hands
        // before the model pins are read (decision 0046 ruling 4), so the
        // inline sites here hold none.
        let seat_map = seat.as_object_mut().unwrap();
        seat_map.remove("agent");
        seat_map.remove("hands");
        seat["role"] = serde_json::json!(format!("agents/charters/{charter}.md"));
        seat["driver"] = serde_json::json!({
            "command": ["{brokkr}", "driver", kind, "--"]
        });
    }
    std::fs::write(&bundle_file, serde_json::to_string_pretty(&config).unwrap()).unwrap();
    let (code, stdout) = doctor_of(&bundle);
    assert_eq!(code, Some(1), "{stdout}");
    assert!(stdout.contains("'implement'"), "{stdout}");
    assert!(stdout.contains("'verify'"), "{stdout}");
    assert!(stdout.contains("--model <concrete-model-id>"), "{stdout}");
}

/// A directory holding one executable that answers `--version`, put FIRST
/// on the PATH doctor runs under: doctor's "installed" is the provider
/// probe's answer, and the probe asks a binary for its version banner. No
/// model is reached — the stand-in can say nothing else.
#[cfg(unix)]
fn path_with_a_stand_in(root: &std::path::Path, binary: &str) -> std::ffi::OsString {
    use std::os::unix::fs::PermissionsExt;
    let bin = root.join("stand-in-bin");
    std::fs::create_dir_all(&bin).unwrap();
    let program = bin.join(binary);
    std::fs::write(&program, "#!/bin/sh\necho '0.0.0 (stand-in)'\n").unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    let rest = std::env::var_os("PATH").unwrap_or_default();
    std::env::join_paths(std::iter::once(bin).chain(std::env::split_paths(&rest))).unwrap()
}

#[cfg(unix)]
fn doctor_lines(cwd: &std::path::Path, path: &std::ffi::OsStr) -> Vec<String> {
    let output = Command::new(env!("CARGO_BIN_EXE_brokkr"))
        .arg("doctor")
        .env("PATH", path)
        .current_dir(cwd)
        .output()
        .unwrap();
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect()
}

/// The complete native lines doctor owes the scaffolded realm for Claude,
/// built from the evidence the SCAFFOLDED adapter declares: doctor prints
/// adapter data, and these are the data.
#[cfg(unix)]
fn scaffolded_claude_denials(workspace: &std::path::Path) -> Vec<String> {
    let adapter: serde_json::Value =
        serde_json::from_slice(&std::fs::read(workspace.join("adapters/claude.json")).unwrap())
            .unwrap();
    ["web-fetch", "web-search"]
        .iter()
        .map(|key| {
            let evidence = &adapter["native_capabilities"]["known"][key]["evidence"];
            let limitations: Vec<&str> = evidence["limitations"]
                .as_array()
                .unwrap()
                .iter()
                .map(|limitation| limitation.as_str().unwrap())
                .collect();
            format!(
                "warn     capabilities starter native claude '{key}': NOT granted here: the \
                 adapter-level plan above switches it off · evidence: {} · still unmeasured: {}",
                evidence["scope"].as_str().unwrap(),
                limitations.join("; ")
            )
        })
        .collect()
}

/// Decision 0065, task 8.2: what `init` scaffolds is proved through
/// doctor. The scaffolded realm grants nothing and says so; with Claude
/// installed, both of its native tools are named as not granted, each
/// beside its evidence scope and the live checks still owed.
#[cfg(unix)]
#[test]
fn doctor_reads_the_scaffold_as_granting_nothing_and_names_claudes_native_tools() {
    let dir = repository();
    let (code, _, stderr) = brokkr(&["init", "."], dir.path());
    assert_eq!(code, Some(0), "{stderr}");
    let path = path_with_a_stand_in(dir.path(), "claude");
    let lines = doctor_lines(dir.path(), &path);
    let expected = scaffolded_claude_denials(&dir.path().join(".brokkr"));
    assert!(
        expected[1].contains("no live denial or enablement of WebSearch has been measured")
            && expected[1].contains("are both unmeasured live"),
        "the scaffold carries the evidence limits: {}",
        expected[1]
    );
    let capabilities: Vec<&String> = lines
        .iter()
        .filter(|line| line.contains(" capabilities starter"))
        .filter(|line| !line.contains(" native exec:"))
        .collect();
    assert_eq!(
        capabilities,
        [
            "ok       capabilities starter: grants nothing; every native capability is governed \
             by the no-grant default — switched off, or the seat is refused",
            "ok       capabilities starter plan claude: adapter-level scope: the adapter's own \
             template alone, with no seat's arguments, model pins, typed tools or hands \
             assessed; a seat's own plan is judged when its bundle compiles · a seat on claude \
             that holds none of its native capabilities is admitted, each of them switched off \
             by the composed command; that is composition, not a live measurement",
            expected[0].as_str(),
            expected[1].as_str(),
        ],
        "{lines:#?}"
    );
}

/// Scaffold into a fresh canonicalised root carrying one init-stacks
/// fixture's markers (none for `None`), and return the implementer's
/// generated `tools` and doctor's capability lines for the starter realm.
#[cfg(unix)]
fn scaffolded_capabilities(fixture: Option<&str>) -> (serde_json::Value, Vec<String>) {
    let dir = repository();
    let root = std::fs::canonicalize(dir.path()).unwrap();
    if let Some(fixture) = fixture {
        let markers = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/init-stacks")
            .join(fixture);
        for entry in std::fs::read_dir(markers).unwrap() {
            let marker = entry.unwrap().path();
            std::fs::copy(&marker, root.join(marker.file_name().unwrap())).unwrap();
        }
    }
    let (code, _, stderr) = brokkr(&["init", "."], &root);
    assert_eq!(code, Some(0), "{fixture:?}: {stderr}");
    let implementer: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join(".brokkr/agents/implementer.json")).unwrap(),
    )
    .unwrap();
    let path = path_with_a_stand_in(&root, "claude");
    let capabilities = doctor_lines(&root, &path)
        .into_iter()
        .filter(|line| line.contains(" capabilities starter"))
        .filter(|line| !line.contains(" native exec:"))
        .collect();
    (implementer["tools"].clone(), capabilities)
}

/// Decision 0065 rebuild unit 5 (task 5.2): a scaffold whose agents carry
/// generated typed restrictions reads through doctor exactly as the
/// unrestricted scaffold does — the same "grants nothing" line and the same
/// two Claude native lines, each switched off. `npm` and `npx` are the
/// migration names the shipped adapters now map too; the scaffolded work
/// agent lists the one its stack runs and nothing wider.
#[cfg(unix)]
#[test]
fn doctor_reads_a_stacks_typed_restrictions_as_granting_nothing() {
    // The control: no stack, so the base grant alone (#717), and the
    // three lines the test above pins whole.
    let (unrestricted, control) = scaffolded_capabilities(None);
    let mut rows = vec![("unrestricted", unrestricted, control.first().cloned())];
    let mut expected = vec![(
        "unrestricted",
        serde_json::json!({"allow": ["git", "ls", "rg", "mkdir"], "mcp": []}),
        Some(
            "ok       capabilities starter: grants nothing; every native capability is governed \
             by the no-grant default — switched off, or the seat is refused"
                .to_string(),
        ),
    )];
    let mut lines = Vec::new();
    for (fixture, runner) in [("node-npm", "npm"), ("turbo-plain", "npx")] {
        let (tools, capabilities) = scaffolded_capabilities(Some(fixture));
        rows.push((fixture, tools, None));
        expected.push((
            fixture,
            serde_json::json!({"allow": [runner, "git", "ls", "rg", "mkdir"], "mcp": []}),
            None,
        ));
        lines.push((fixture, capabilities));
    }
    assert_eq!(rows, expected);
    assert_eq!(
        lines,
        [("node-npm", control.clone()), ("turbo-plain", control)]
    );
}

/// Design D8: independent results survive an agent library that does not
/// load. The library's failure is its own line, and the native lines —
/// Claude's denials and generic exec's unmeasured reason — still print.
#[cfg(unix)]
#[test]
fn a_broken_agent_library_takes_no_native_capability_line_with_it() {
    let dir = repository();
    let (code, _, stderr) = brokkr(&["init", "."], dir.path());
    assert_eq!(code, Some(0), "{stderr}");
    let folder = dir.path().join(".brokkr");
    std::fs::write(folder.join("agents/implementer.json"), "not json").unwrap();
    let path = path_with_a_stand_in(dir.path(), "claude");
    let lines = doctor_lines(dir.path(), &path);
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("warn     agents:")),
        "{lines:#?}"
    );
    // The scaffolded exec adapter declares no assessment of its own, so
    // doctor reads it as every pre-ruling adapter is read: unmeasured,
    // with the absence as the reason — never as an empty inventory.
    let mut expected = scaffolded_claude_denials(&folder);
    expected.push(
        "warn     capabilities starter native exec: native inventory unmeasured: the adapter \
         declares no native_capabilities assessment. Nothing is granted through it and no \
         native denial is claimed"
            .to_string(),
    );
    for line in &expected {
        assert!(lines.contains(line), "{line}\n---\n{lines:#?}");
    }
}

#[test]
fn doctor_judges_each_boundary_and_compiles_the_discovered_dialect() {
    use serde_json::{json, Value};
    let dir = repository();
    let (code, _, error) = brokkr(&["init", "."], dir.path());
    assert_eq!(code, Some(0), "{error}");
    let empty = dir.path().join("empty-path");
    std::fs::create_dir(&empty).unwrap();
    let map_path = dir.path().join(".brokkr/realms.json");
    let mut map: Value = serde_json::from_slice(&std::fs::read(&map_path).unwrap()).unwrap();
    map["schema"] = json!("forge.realms/v4");
    let doctor = || {
        let output = Command::new(env!("CARGO_BIN_EXE_brokkr"))
            .args(["doctor", "--bundle", ".brokkr"])
            .env("PATH", &empty)
            .current_dir(dir.path())
            .output()
            .unwrap();
        String::from_utf8(output.stdout).unwrap()
    };
    for (word, expected) in [
        ("namespace", "warn     hands:"),
        ("harness", "ok       hands:"),
        ("open", "ok       hands:"),
        ("seatbelt", "slice (ii)"),
        ("container", "slice (iii)"),
    ] {
        map["realms"][0]["boundary"] = json!(word);
        std::fs::write(&map_path, map.to_string()).unwrap();
        let output = doctor();
        assert!(output.contains(expected), "{word}: {output}");
        assert!(
            output.contains("boundaries: harness · open offered"),
            "{output}"
        );
        assert!(output.contains("ok       bundle:"), "{output}");
    }
    // A declared dialect which cannot load must also fail bundle compilation;
    // merely adding a diagnostic beside a compile with default dialect is wrong.
    map["realms"][0]["boundary"] = json!("namespace");
    map["realms"][0]["dialect"] = json!("missing-dialect.json");
    std::fs::write(&map_path, map.to_string()).unwrap();
    let output = doctor();
    assert!(output.contains("MISSING  bundle:"), "{output}");
    assert!(output.contains("missing-dialect.json"), "{output}");
    assert!(!output.contains("ok       bundle:"), "{output}");
}
