//! Issue #366: `docs/status.md` says what works today per harness, and
//! `docs/security-model.md` says what the box does and does not do.
//!
//! The status page's matrix is the adapter data rendered. This test loads
//! `adapters/*.json` through the engine's own loader, which refuses a key
//! it does not know, renders every `Adapter` field it reads, and holds the
//! page's block to that rendering byte for byte. Each adapter struct is
//! destructured without `..`, so a field the loader grows fails this
//! file's compile until the matrix rules on it: a field an adapter grows
//! cannot go unlisted, and a rendered value that changes shows on the page.
//!
//! No living doc may say a tool list bounds a seat: every tracked Markdown
//! file outside the dated records, and every doc comment in the crates'
//! sources, is read for that wording, and each sentence this story
//! reworded is held refused.

use std::collections::{BTreeMap, BTreeSet};

use brokkr_runtime::agents::{
    Adapter, Agent, McpSupport, ResumeAssessment, ResumeEvidence, ResumeIdentity, ResumeShape,
    ResumeStatus, ToolPermissions,
};
use brokkr_runtime::{Adapters, HarnessHands, Library, TrustTier};

#[path = "support/tracked.rs"]
mod tracked_files;
#[path = "support/workspace.rs"]
mod workspace;

use workspace::{read, workspace};

/// Where a known limitation's owner lives: the open issue, or the
/// unmerged pull request that delivers the fix.
const OWNERS: [&str; 2] = [
    "](https://github.com/feedback-loop-ai/brokkr/issues/",
    "](https://github.com/feedback-loop-ai/brokkr/pull/",
];

/// The text between a page's `<!-- {name}:start -->` and
/// `<!-- {name}:end -->` markers; a page without both panics.
fn block<'a>(page: &'a str, name: &str) -> &'a str {
    page.split_once(&format!("<!-- {name}:start -->\n"))
        .unwrap_or_else(|| panic!("no {name} start marker"))
        .1
        .split_once(&format!("<!-- {name}:end -->"))
        .unwrap_or_else(|| panic!("no {name} end marker"))
        .0
}

/// What an empty list renders as: a dash, because `none` is one of
/// codex's effort levels.
const EMPTY: &str = "—";

/// Backticked names joined by commas, or [`EMPTY`].
fn names<'a>(items: impl IntoIterator<Item = &'a String>) -> String {
    let joined: Vec<String> = items.into_iter().map(|item| format!("`{item}`")).collect();
    words(&joined)
}

/// Words joined by commas, or [`EMPTY`].
fn words(items: &[String]) -> String {
    match items.is_empty() {
        true => EMPTY.to_string(),
        false => items.join(", "),
    }
}

/// One adapter's matrix row, and the measured gaps it declares as list
/// items for the section below the table.
fn render(adapter: &Adapter) -> (String, Vec<String>) {
    let Adapter {
        provider,
        trust_tier,
        egress,
        routes,
        effortless_routes,
        judges,
        efforts,
        tool_permissions,
        tool_permissions_gap,
        mcp,
        hands,
        hands_gap,
        harness,
        resume,
        // How the engine reaches the harness, not what it can do: the
        // binary doctor probes, its install hint, the driver prefix, the
        // model map and pin flags, route credential names, the boxed
        // seat's tool notice, and the file's own digest.
        binary: _,
        hint: _,
        driver: _,
        models: _,
        model_flag: _,
        effort_flag: _,
        credentials: _,
        hands_notice: _,
        digest: _,
    } = adapter;
    let mut gaps = Vec::new();
    let trust = match trust_tier {
        TrustTier::Trusted => "trusted",
        TrustTier::Untrusted => "untrusted",
    };
    let mut egress = egress.name().to_string();
    for (route, class) in routes {
        egress.push_str(&format!("; `{route}`: {}", class.name()));
    }
    let gate = match (trust_tier, judges.is_empty()) {
        (TrustTier::Trusted, false) => format!("yes: {}", names(judges)),
        (TrustTier::Trusted, true) => "no: no judges".to_string(),
        (TrustTier::Untrusted, _) => "no: untrusted".to_string(),
    };
    let mut effort = words(efforts);
    if !effortless_routes.is_empty() {
        effort.push_str(&format!("; none on {}", names(effortless_routes.keys())));
    }
    for (route, measurement) in effortless_routes {
        gaps.push(format!("`{provider}` effort on `{route}`: {measurement}"));
    }
    let tools = match (tool_permissions, tool_permissions_gap) {
        // The flag is named in the legend, and the separator is argv
        // syntax, not a capability.
        (
            Some(ToolPermissions {
                names: grants,
                flag: _,
                separator: _,
            }),
            _,
        ) => names(grants.keys()),
        (None, Some(gap)) => {
            gaps.push(format!("`{provider}` tool allow-list: {gap}"));
            "no (measured)".to_string()
        }
        (None, None) => "no".to_string(),
    };
    // The flag Brokkr can pass a seat's declared servers through, not
    // whether the seat reaches MCP: the operator's own configuration
    // reaches it either way. The server map is what an agent may name.
    let mcp = match mcp {
        Some(McpSupport { flag, servers: _ }) => format!("`{flag}`"),
        None => "none".to_string(),
    };
    let boxed = match (hands, hands_gap) {
        (Some(_), _) => "yes".to_string(),
        (None, Some(gap)) => {
            gaps.push(format!("`{provider}` boxed hands: {gap}"));
            "no (measured)".to_string()
        }
        (None, None) => "no".to_string(),
    };
    let sandbox = harness_sandbox(provider, harness, &mut gaps);
    let resume = resume_shapes(provider, resume, &mut gaps);
    let row = format!(
        "| `{provider}` | {trust} | {egress} | {gate} | {effort} | {tools} | {mcp} | {boxed} | {sandbox} | {resume} |"
    );
    (row, gaps)
}

/// Which seat classes the harness's own sandbox stands in for the box
/// under the `harness` boundary (decision 0046 ruling 4).
fn harness_sandbox(provider: &str, harness: &HarnessHands, gaps: &mut Vec<String>) -> String {
    let HarnessHands {
        gate,
        gate_gap,
        work,
        work_gap,
        result: _,
    } = harness;
    let mut classes = Vec::new();
    for (class, fragment, gap) in [("gate", gate, gate_gap), ("work", work, work_gap)] {
        if fragment.is_some() {
            classes.push(class.to_string());
        }
        if let Some(gap) = gap {
            gaps.push(format!("`{provider}` harness {class}: {gap}"));
        }
    }
    words(&classes)
}

/// The resume shapes an adapter file names, by name alone: the engine's
/// loader has already typed, and refused, what each one holds.
#[derive(serde::Deserialize)]
struct DeclaredShapes {
    #[serde(default)]
    resume: BTreeMap<String, serde::de::IgnoredAny>,
}

/// Every named resume shape with its status and measured version; the
/// reason of each shape that is not supported goes to the gaps. The
/// names come from the adapter's file, and a name the loaded assessment
/// does not hold panics.
fn resume_shapes(provider: &str, resume: &ResumeAssessment, gaps: &mut Vec<String>) -> String {
    let declared: DeclaredShapes =
        serde_json::from_str(&read(&format!("adapters/{provider}.json")))
            .unwrap_or_else(|error| panic!("adapters/{provider}.json: {error}"));
    assert_eq!(
        declared.resume.is_empty(),
        resume.is_empty(),
        "`{provider}`'s file and loaded assessment disagree on having shapes"
    );
    let mut cells = Vec::new();
    for name in declared.resume.keys() {
        let shape = resume
            .shape(name)
            .unwrap_or_else(|| panic!("`{provider}` names resume `{name}`, which did not load"));
        let ResumeShape {
            status,
            identity,
            reason,
            classes,
            boundaries,
            hands,
            evidence,
            // The adapter's dated prose notes on the shape; the page
            // points to the file for them.
            limitations: _,
        } = shape;
        // The installed version a measurement is claimed for, the
        // composite runner digest and an unknown identity's reason are
        // the adapter's record of the proof, read in the file.
        let version = match identity {
            ResumeIdentity::Measured {
                version,
                applies_to: _,
                wrapper_digest: _,
            } => version.as_str(),
            ResumeIdentity::Unknown { reason: _ } => "version unknown",
        };
        cells.push(format!(
            "`{name}`: {} ({version}); classes {}; boundaries {}; hands `{hands}`; evidence {}",
            status.word(),
            names(classes),
            names(boundaries),
            evidence_axes(evidence),
        ));
        match (status, reason) {
            (ResumeStatus::Supported, _) | (_, None) => {}
            (ResumeStatus::Unmeasured | ResumeStatus::Unsupported, Some(reason)) => {
                gaps.push(format!("`{provider}` resume `{name}`: {reason}"));
            }
        }
    }
    words(&cells)
}

/// The evidence axes a resume shape records, by name, or [`EMPTY`].
fn evidence_axes(evidence: &ResumeEvidence) -> String {
    let ResumeEvidence {
        interface,
        restrictions,
        root,
        accounting,
    } = evidence;
    let axes: Vec<String> = [
        ("interface", interface),
        ("restrictions", restrictions),
        ("root", root),
        ("accounting", accounting),
    ]
    .into_iter()
    .filter(|(_, recorded)| recorded.is_some())
    .map(|(axis, _)| axis.to_string())
    .collect();
    names(&axes)
}

/// The MCP column's name: the flag Brokkr can pass, never whether a seat
/// reaches MCP servers, which the operator's own configuration decides.
const MCP_COLUMN: &str = "MCP flag Brokkr passes";

/// The MCP column's legend, which the page carries word for word.
const MCP_LEGEND: &str = "the flag through which a seat's declared `mcp` servers are \
    passed, not whether the seat reaches MCP servers. The operator's own configuration \
    still reaches every Codex seat, boxed or not, through `~/.codex/config.toml`, and \
    every unboxed claude seat, through their Claude Code configuration. Only the boxed \
    claude fragment passes `--strict-mcp-config`, which shuts those out.";

/// The matrix and gap blocks as the adapter data renders them.
fn rendered(adapters: &Adapters) -> (String, String) {
    let mut matrix = format!(
        "| Harness | Trust | Egress | Holds a model gate | Efforts | Tool allow-list | {MCP_COLUMN} | Boxed hands | Own sandbox for | Resume shapes |\n\
         |---|---|---|---|---|---|---|---|---|---|\n",
    );
    let mut gaps = String::new();
    for adapter in adapters.providers() {
        let (row, declared) = render(adapter);
        matrix.push_str(&row);
        matrix.push('\n');
        for gap in declared {
            gaps.push_str(&format!("- {gap}\n"));
        }
    }
    (matrix, gaps)
}

#[test]
fn the_status_matrix_is_the_adapter_data() {
    let adapters =
        Adapters::load(&workspace().join("adapters")).expect("the shipped adapters load");
    let page = read("docs/status.md");
    let (matrix, gaps) = rendered(&adapters);
    assert_eq!(
        block(&page, "adapter-matrix"),
        matrix,
        "docs/status.md's matrix drifted from adapters/*.json; the rendering is:\n{matrix}"
    );
    assert_eq!(
        block(&page, "adapter-gaps"),
        gaps,
        "docs/status.md's measured gaps drifted from adapters/*.json; the rendering is:\n{gaps}"
    );
    let legend = format!("- **{MCP_COLUMN}**: {MCP_LEGEND}");
    assert!(
        page.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .contains(&legend),
        "docs/status.md's legend does not say what the MCP column is:\n{legend}"
    );

    // The run-behaviour table below the matrix is written by hand, from
    // the drivers' code, and has exactly one row per adapter.
    let providers: BTreeSet<String> = adapters.providers().map(|a| a.provider.clone()).collect();
    let rows: Vec<String> = block(&page, "harness-behaviour")
        .lines()
        .filter(|line| line.starts_with("| `"))
        .map(|line| line.split('`').nth(1).expect("a named harness").to_string())
        .collect();
    assert_eq!(
        rows.iter().cloned().collect::<BTreeSet<_>>(),
        providers,
        "docs/status.md's behaviour table lists every adapter"
    );
    assert_eq!(rows.len(), providers.len(), "one behaviour row per adapter");
}

/// The claude row's web-search cell as the agent library renders it. For
/// an agent a claude seat can hire, a tool list becomes `--allowedTools`
/// and hands become the box's `--tools ""`; an agent with neither runs
/// unboxed with no tool flag, so the harness and the operator's own
/// settings decide, never a Brokkr control (#467).
fn claude_web_cell(library: &Library, claude: &Adapter) -> String {
    let hireable: Vec<&Agent> = library
        .agents()
        .filter(|agent| agent.models.iter().any(|m| claude.models.contains_key(m)))
        .collect();
    let tool_less = hireable
        .iter()
        .filter(|agent| agent.allow.is_none() && agent.hands.is_none())
        .map(|agent| &agent.name);
    let web = hireable
        .iter()
        .filter(|agent| {
            agent.allow.as_ref().is_some_and(|allow| {
                allow
                    .iter()
                    .any(|tool| tool == "websearch" || tool == "webfetch")
            })
        })
        .map(|agent| &agent.name);
    format!(
        "Decided by the harness's permission model and the operator's own Claude Code settings, \
         which reach every unboxed seat. An agent that lists tools passes them as `--allowedTools`; \
         `websearch` or `webfetch` is listed by {}. An agent that lists no tools and declares no \
         hands, as {} do, runs unboxed with no tool flag, so Claude Code's default tools, \
         `WebSearch` and `WebFetch` among them, and the operator's MCP servers reach it \
         ([#467](https://github.com/feedback-loop-ai/brokkr/issues/467)). \
         A boxed seat runs with `--tools \"\"`",
        names(web),
        names(tool_less),
    )
}

#[test]
fn the_claude_web_cell_names_what_decides() {
    let root = workspace();
    let adapters = Adapters::load(&root.join("adapters")).expect("the shipped adapters load");
    let library = Library::load(&root.join("agents")).expect("the shipped agents load");
    let claude = adapters.adapter("claude").expect("a claude adapter");
    let page = read("docs/status.md");
    let row = block(&page, "harness-behaviour")
        .lines()
        .find(|line| line.starts_with("| `claude` |"))
        .expect("a claude behaviour row");
    let (_, cell) = row
        .trim_end_matches(" |")
        .rsplit_once(" | ")
        .expect("a web-search cell");
    let rendered = claude_web_cell(&library, claude);
    assert_eq!(
        cell, rendered,
        "docs/status.md's claude web-search cell drifted from agents/*.json; the rendering is:\n{rendered}"
    );
}

/// Every item of a page's `## Known limitations` list links the open
/// issue or pull request that owns it; an item without one is refused.
fn unlinked_limitations(page: &str) -> Vec<String> {
    let section = page
        .split_once("\n## Known limitations\n")
        .expect("a Known limitations section")
        .1;
    let section = section
        .split_once("\n## ")
        .map_or(section, |(body, _)| body);
    let mut items: Vec<String> = Vec::new();
    for line in section.lines() {
        match (line.strip_prefix("- "), items.last_mut()) {
            (Some(item), _) => items.push(item.to_string()),
            (None, Some(open)) if line.starts_with("  ") => open.push_str(line),
            _ => {}
        }
    }
    assert!(!items.is_empty(), "the Known limitations list is empty");
    items
        .into_iter()
        .filter(|item| !OWNERS.iter().any(|owner| item.contains(owner)))
        .collect()
}

#[test]
fn every_known_limitation_links_its_issue() {
    for page in ["docs/status.md", "docs/security-model.md"] {
        assert_eq!(
            unlinked_limitations(&read(page)),
            Vec::<String>::new(),
            "{page} states a limitation with no issue link"
        );
    }
}

/// Records whose words are fixed when they are written, and so are not
/// living docs.
const RECORDS: [&str; 6] = [
    "docs/decisions/",
    "docs/releases/",
    "docs/lore/",
    "docs/essays/",
    "docs/evidence/",
    "openspec/changes/archive/",
];

/// What names a claude seat's tool list: the list itself, the agent
/// field it comes from, the flag it is passed as, or the restriction or
/// grant it is called.
const TOOL_LIST: [&str; 6] = [
    "tool list",
    "tools.allow",
    "tool grant",
    "the grant",
    "--allowedtools",
    "tool restriction",
];

/// Wording that says a tool list bounds a seat. `--allowedTools`
/// pre-approves and removes no tool, so an unboxed claude seat is bounded
/// by Claude Code's permission model and the operator's own settings and
/// MCP servers, never by its tool list alone (#467). The resolver's
/// "more power than it declares" is not here: a provider that cannot
/// express a tool list refuses it, and the docs quote that refusal.
const OVERCLAIMS: [&str; 8] = [
    "not a command it may run",
    "enforced rather than documented",
    "seats run under",
    "only restriction",
    "bounded only by",
    "may run exactly",
    "may run nothing outside",
    "decides what the seats may run",
];

/// Wording refused wherever it stands: a tool list called a restriction
/// or a narrowing or said to bound a stack's seats, the box said to bound
/// more than a `workspace` call or an exec command, the harness said to
/// be spawned behind it, a boundary said to stand around a seat without
/// hands, or a hands fragment said to take the harness's own tools away.
/// A boxed Codex seat keeps its native read-only shell outside the box,
/// a boxed claude seat's harness still loads the operator's own
/// configuration on the host, and under `open` nothing is added.
///
/// The scaffold's verify and ship scripts said to be boxed, or to run
/// with no network, on every host are refused too: `init` declares
/// `harness` for a codex scaffold and on macOS, where they run under no
/// box of Brokkr's and no network denial is reported. So is a gate said
/// never to write or to be boxed whatever it declares: the default
/// delivery's review gate runs unboxed under `acceptEdits`.
///
/// A comparison that only implies a bound, such as one arm called no
/// narrower than another, is outside the guard: its wording names no
/// control a list could hold.
const ANYWHERE: [&str; 37] = [
    "blast radius",
    "no tool restriction",
    "tools restriction",
    "no restriction",
    "permission narrowing",
    "what the model asks to run goes through one",
    "the one tool the model sees",
    "own tools are replaced by the one boxed tool",
    "replaces the harness's own tools",
    "bounds what running anything can touch",
    "only through its workspace tool",
    "seat's commands run inside",
    "harness and open no box stands",
    "spawn, behind the realm's boundary",
    "around a seat is the realm's boundary",
    "a model gate is judged under",
    "disables the harness's own tools",
    "swap its tool surface",
    "prefixes the former inline seat named",
    "stack's seats may run",
    "on a restriction",
    "replace the harness's own tools",
    "removes the harness's own tools",
    "boxes what a gate can reach",
    "gates never write",
    "boxed exec verify",
    "two boxed exec gates",
    "carry a boxed exec script",
    "name boxed exec scripts",
    "deterministic boxed verify and",
    "deterministic boxed verifier",
    "deterministic boxed driver",
    "deterministic boxed exec scripts",
    "boxed without network",
    "runs without network",
    "with network denied",
    "are boxed scripts",
];

/// The wording the guard refuses, as lists a test can take one word out of.
struct Vocabulary<'a> {
    tool_list: &'a [&'a str],
    overclaims: &'a [&'a str],
    anywhere: &'a [&'a str],
}

const VOCABULARY: Vocabulary<'static> = Vocabulary {
    tool_list: &TOOL_LIST,
    overclaims: &OVERCLAIMS,
    anywhere: &ANYWHERE,
};

/// Each paragraph of a Markdown text that the full [`VOCABULARY`] refuses.
fn tool_list_overclaims(text: &str) -> Vec<String> {
    overclaims_in(text, &VOCABULARY)
}

/// Each paragraph of a Markdown text with wording refused anywhere, or
/// wording that says a tool list bounds the seat where it or the
/// paragraph before names the tool list, so a list introduced above its
/// bullets cannot hide one. Paragraphs are lowercased with their code and
/// emphasis marks dropped and their line breaks joined, so neither can
/// hide one either.
fn overclaims_in(text: &str, vocabulary: &Vocabulary) -> Vec<String> {
    let paragraphs: Vec<String> = text
        .split("\n\n")
        .map(|paragraph| {
            paragraph
                .replace(['`', '*'], "")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase()
        })
        .filter(|paragraph| !paragraph.is_empty())
        .collect();
    let says = |paragraph: &str, words: &[&str]| words.iter().any(|word| paragraph.contains(word));
    paragraphs
        .iter()
        .enumerate()
        .filter(|(at, paragraph)| {
            says(paragraph, vocabulary.anywhere)
                || says(paragraph, vocabulary.overclaims)
                    && (says(paragraph, vocabulary.tool_list)
                        || at
                            .checked_sub(1)
                            .is_some_and(|before| says(&paragraphs[before], vocabulary.tool_list)))
        })
        .map(|(_, paragraph)| paragraph.clone())
        .collect()
}

/// A Rust source's doc comments as Markdown: each `//!` or `///` line's
/// text, and a paragraph break at every line that is not one.
fn doc_text(source: &str) -> String {
    source
        .lines()
        .map(|line| {
            let line = line.trim_start();
            line.strip_prefix("//!")
                .or_else(|| line.strip_prefix("///"))
                .unwrap_or("")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Excerpts of the pages this story reworded, word for word as they
/// stood: each holds one paragraph the guard refuses.
const OLD_PAGES: [&str; 43] = [
    // docs/guides/agent-library.md
    "**The honesty rules are the point, and they are enforced rather than\n\
     documented.** A tool restriction the provider cannot express fails\n\
     compilation naming the agent, the provider and the capability — the\n\
     agent would run with MORE power than it declares, so `optional` is\n\
     structurally unrepresentable there.",
    // `brokkr init`'s line, as the quickstart and starters captured it
    "run brokkr from inside my-bundle — its adapters/ and agents/ declare the \
     trust tier and the tool grants its seats run under",
    // docs/guides/quickstart.md
    "**The seats are granted the tools their charters name.** The same\n\
     detection below decides what the seats may *run*: the binary each\n\
     command invokes (`cargo`, `bun`, `pnpm`, …) plus `git`, `ls`, `rg` and\n\
     `mkdir` go into the adapter's `tool_permissions.names` as\n\
     `Bash(<bin>:*)` entries, and each model agent's `tools.allow` names them —",
    "command it did not need. The same stack decides what the seats may RUN:\n\
     the binaries its commands invoke are written into the scaffold's\n\
     `adapters/claude.json` `tool_permissions.names` as `Bash(<bin>:*)`\n\
     entries and granted in the agents' `tools.allow` — the whole set to the\n\
     work seats,",
    // docs/guides/starters/rust.md: the list is named above the bullet
    "The same names, in the same order, are each agent's `tools.allow` —\n\
     sized by the class of the seat the agent backs:\n\n\
     - **work seats (`intake`, `implement`)** — the whole set:\n  \
     `[\"cargo\", \"git\", \"ls\", \"rg\", \"mkdir\"]`. A work seat may run exactly\n  \
     the commands its charter names, and nothing broader.",
    // docs/guides/provider-adapters.md
    "above records 2.1.251), because the implementing seat's tool grant is\n\
     `cargo` and `git` and `claude` is not a command it may run. Until it is",
    // recipes/wager-harness/README.md
    "   implement seat runs `--permission-mode acceptEdits` with\n   \
     `--allowedTools` naming seven `Bash` prefixes, so it may edit freely\n   \
     but may run nothing outside that list — no network command, for\n   \
     instance.",
    // docs/security-model.md, as this story first wrote it
    "  the engine's environment, under claude's `--permission-mode\n  \
     acceptEdits`, and the tool list is their only restriction. The",
    "credential and its connection to the provider. A boxed claude seat\n\
     reaches the host only through its `workspace` tool. A boxed codex seat",
    // ARCHITECTURE.md, as this story first wrote it
    "| Brokkr verification | `bundles/verify` examines a delivered change with a boxed verify \
     seat and an unboxed review seat bounded only by its tool list ([security \
     model](docs/security-model.md)).",
    // docs/guides/agent-library.md's hands
    "list (decision 0043). The harness keeps its credential and its network;\n\
     what the model asks to run goes through one MCP tool, `workspace`, served",
    "With hands, the adapter's per-tool map is not consulted; what the adapter\n\
     must express is how its harness's own tools are replaced by the one boxed\n\
     tool (`hands` in the adapter file, or `\"unsupported\"` with the reason).",
    // docs/guides/starters/rust.md's fallback
    "(`\"names\": {}`), no `tools` restriction on any agent, and a README that",
    // docs/guides/recipe-authoring.md's `hands` row
    "a site or in this object is refused naming the realm as its home. Under \
     `namespace` the seat's commands run inside an empty-root box holding the \
     worktree; an exec seat also gets its bundle root read-only at `/runtime/bundle`",
    "Under `harness` and `open` no box stands: a model seat runs under its \
     harness's own sandbox as the adapter's `hands.harness` fragment addresses it",
    // ARCHITECTURE.md's driver diagram and the paragraph below it
    "  D->>H: spawn, behind the realm's boundary",
    "language-neutral for third-party drivers. What stands around a seat is\n\
     the realm's **boundary** (decision 0046): `namespace`, `seatbelt`,",
    // docs/guides/driver-authoring.md
    "`container` boundary, slice (iii) and decision 0046 ruling 5: what\n\
     stands around a seat is the realm's `boundary`, declared in",
    // docs/guides/quickstart.md
    "`forge.realms/v4`, and then Brokkr builds no box at all: a model gate\n\
     is judged under the harness's own sandbox as the adapter's\n\
     `hands.harness` fragment addresses it, an exec gate runs the bundle's",
    // docs/guides/provider-adapters.md's hands
    "hands: the argv fragment that disables the harness's own tools and reaches\n\
     `brokkr hands serve` over MCP.",
    "`{\"unsupported\": \"<measured reason>\"}` declares that the\n\
     harness cannot swap its tool surface, and a site with hands then refuses",
    // recipes/night-shift/README.md
    "harness permits, not the seven `Bash` prefixes the former inline seat named.",
    // docs/research/0004-context-privilege-escalation.md's citations
    "`agents/charters/review-chief.md` treats peer prose as untrusted input; decision 0043 \
     boxes what a gate can reach; commit messages are still read by the reviewer as instructions",
    "decision 0043: the box cannot plant a hook and gates never write; decision 0034: the seat \
     record admits no prompt or response text",
    "decision 0043: hands replace the harness's own tools with one boxed workspace tool, so a \
     skill's shell block runs inside the box",
    // docs/research/0005-model-based-agentic-software-engineering.md's
    "decision 0021: work and gate seats; decision 0041: gates never write; decision 0033: the \
     operator merges",
    // docs/guides/agent-library.md
    "([security model](../security-model.md)). Only declared hands put a\n\
     seat in the box, where `--tools \"\"` removes the harness's own tools.",
    // the scaffold's `agents/README.md`, as `brokkr init` wrote it on every host
    "- `bundle.json` — three model offices plus boxed exec verify and\n  \
     ship gates, with each seat's results and limits.",
    "- `scripts/*.sh` — deterministic verify and ship offices; verify\n  \
     names this repository's own commands and runs without network.",
    // the note the scaffold's verify script journaled on every host
    "printf '%s and %s passed with network denied' \"$test_command\" \"$lint_command\" > \"$notes\"",
    // docs/guides/quickstart.md
    "seat per phase. Model offices carry an agent definition with their charter,\n\
     model chain and tool grant; deterministic offices carry a boxed exec script.",
    "./adapters/exec.json   # the deterministic boxed driver",
    "or gate (decision 0021 ruling 1) — and its result vocabulary. Intake,\n\
     implement and review name agents; verify and ship name boxed exec scripts.",
    "./bundle.json          # five seats: three model offices and two boxed exec gates",
    "./scripts/verify-seat.sh # detected test and lint commands, boxed without network",
    "gate constitutionally protected, three model offices, and boxed exec verify\n\
     and ship gates. The agent files in the scaffold's own `agents/` carry each",
    // docs/guides/starters/rust.md, go.md, node.md, bun.md and python.md
    "The same scaffold shape as every stack: three model offices and two boxed\n\
     exec gates, with model and exec adapters, verify/ship scripts and\n\
     `realms.json`. The",
    "The deterministic boxed verifier contains these detected command pins:",
    "It runs both with network denied, using the bound Cargo registry cache,\n\
     types `pass` only when both exit zero, and quotes decisive output on\n\
     `fail`.",
    "It runs both from the repository root with network denied, types `pass`\n\
     only when both exit zero, and quotes decisive output on `fail`.",
    "It runs both from the repository root with network denied, writes `pass`\n\
     only when both exit zero, and writes `fail` with decisive output otherwise.",
    "The deterministic boxed verifier pins `bun run test` and\n\
     `bun run typecheck`. It runs both with network denied, types `pass` only\n\
     when both exit zero, and quotes decisive output on `fail`.",
    "The deterministic boxed verifier pins `uv run pytest` and\n\
     `uv run ruff check .`. It runs both with network denied, types `pass` only\n\
     when both exit zero, and quotes decisive output on `fail`.",
];

/// Excerpts of the doc comments this story reworded, as the sources
/// carried them.
const OLD_SOURCES: [&str; 17] = [
    // crates/brokkr-protocol/src/hands.rs
    "//! `/tmp`, no host home, no host credential, no other process, and no\n\
     //! network unless the spec grants it. A tool allow-list bounded what the\n\
     //! model may run; the box bounds what running anything can touch.",
    // crates/brokkr-runtime/src/agents.rs
    "//! - A **restriction** the resolved provider cannot express (a tool\n\
     //!   permission narrowing) is always a hard failure: the agent would run\n\
     //!   with MORE power than it declares. `optional` is structurally\n\
     //!   unrepresentable on a restriction — `tools.allow` is a plain array,\n\
     //!   there is no key to set.\n\
     //! - Both checks run over **every** entry in the chain, not just the\n\
     //!   chosen one: a chain whose second link cannot express the agent's\n\
     //!   restrictions would silently widen its blast radius the moment it\n\
     //!   fell back.",
    // crates/brokkr-cli/src/init.rs
    "//! name, in the scaffolded model agents' `tools.allow` lists: an allowance the\n\
     //! adapter cannot express is a compile refusal, so the two files are ONE\n\
     //! grant, not two. The split is decision 0021 ruling 1's: the WORK-class\n\
     //! seats (intake, implement) may run the full set — the stack's runners\n\
     //! plus `git`, `ls`, `rg` and `mkdir` — so a seat may run exactly the\n\
     //! commands its charter names and nothing broader;",
    "/// - `work` — the whole set: every runner above plus `git`, `ls`, `rg`\n\
     ///   and `mkdir`, so a work seat may run exactly the commands its\n\
     ///   charter names and nothing broader;\n\
     /// - `gate` — the read-only subset: the test command's tools (which, for\n\
     ///   every row in the tables today, are the same binary the build and\n\
     ///   lint lines also lead with — the grant is per binary, and the README",
    "//! EMPTY, no agent declares a `tools` restriction, and the scaffold's",
    "/// grant: the loader rejects an empty `allow` as ambiguous between \"no\n\
     /// restriction\" and \"restrict to nothing\", and the README says which of",
    // crates/brokkr-cli/tests/init_stacks.rs
    "//! The same table decides what the seats may RUN: the binary each command\n\
     //! invokes is written into the scaffold's adapter as a tool permission and\n\
     //! granted to the scaffolded agents — the whole set to the work seats, the\n\
     //! read-only subset to the gates — and what is asserted is the resolved\n\
     //! argv the compiler composes for the implement seat, because a grant that\n\
     //! never reached `--allowedTools` is no grant.",
    // crates/brokkr-protocol/src/hands.rs
    "/// The one tool the model sees. Claude Code names it `mcp__brokkr__workspace`.",
    // crates/brokkr-runtime/src/agents.rs and its tests
    "    /// `None` declares NO tool restriction; `Some` is ordered, and that",
    "    /// anything can touch — and the adapter must say how it replaces the\n\
     /// harness's own tools with that one.",
    "/// How a provider expresses a tool-permission narrowing on its command",
    "/// would widen the agent's blast radius on fallback is a design-time",
    "/// what makes \"optional on a restriction\" unrepresentable rather than\n\
     /// merely forbidden.",
    // crates/brokkr-cli/src/init.rs
    "/// The tools one detected stack's seats may run, split by decision 0021",
    "//! model-backed work and review offices, deterministic boxed verify and\n\
     //! ship offices, and the bundled headless Claude Code and exec drivers.",
    "/// The scaffold follows the shipped roster: work and review are model\n\
     /// offices, while verify and ship are deterministic boxed exec scripts.",
    "//! Verify and ship are boxed scripts and carry no model grants.",
];

/// Every old excerpt as the guard reads it: a page as written, a source
/// as its doc comments.
fn old_texts() -> Vec<String> {
    let pages = OLD_PAGES.iter().map(|page| page.to_string());
    let sources = OLD_SOURCES
        .iter()
        .map(|source| doc_text(&format!("{source}\nfn f() {{}}\n")));
    pages.chain(sources).collect()
}

#[test]
fn the_guard_refuses_every_sentence_this_story_reworded() {
    for text in old_texts() {
        assert_eq!(tool_list_overclaims(&text).len(), 1, "not refused:\n{text}");
    }
}

/// Each word the guard holds is the only word that refuses some old
/// excerpt, so none can be dropped with this file green and none is held
/// for a sentence no one wrote.
#[test]
fn every_word_the_guard_holds_alone_refuses_an_old_sentence() {
    let texts = old_texts();
    for word in TOOL_LIST.iter().chain(&OVERCLAIMS).chain(&ANYWHERE) {
        let keep = |list: &[&'static str]| -> Vec<&'static str> {
            list.iter().copied().filter(|kept| kept != word).collect()
        };
        let (tool_list, overclaims, anywhere) =
            (keep(&TOOL_LIST), keep(&OVERCLAIMS), keep(&ANYWHERE));
        let without = Vocabulary {
            tool_list: &tool_list,
            overclaims: &overclaims,
            anywhere: &anywhere,
        };
        assert!(
            texts
                .iter()
                .any(|text| overclaims_in(text, &without).is_empty()),
            "{word:?} alone refuses no old excerpt: drop it, or hold the sentence it is for"
        );
    }
}

#[test]
fn no_living_doc_says_a_tool_list_bounds_an_unboxed_seat() {
    let root = workspace();
    let pages: Vec<String> = tracked_files::tracked(&root, &["*.md"])
        .into_iter()
        .filter(|page| !RECORDS.iter().any(|record| page.starts_with(record)))
        .collect();
    for page in ["ARCHITECTURE.md", "README.md", "docs/security-model.md"] {
        assert!(pages.iter().any(|p| p == page), "{page} is not scanned");
    }
    // This file holds the refused sentences as its fixtures.
    let sources: Vec<String> =
        tracked_files::tracked(&root, &["crates/*/src/*.rs", "crates/*/tests/*.rs"])
            .into_iter()
            .filter(|source| source != "crates/brokkr-cli/tests/status_pages.rs")
            .collect();
    for source in [
        "crates/brokkr-protocol/src/hands.rs",
        "crates/brokkr-runtime/src/agents.rs",
        "crates/brokkr-runtime/src/agents/tests.rs",
        "crates/brokkr-cli/tests/init_stacks.rs",
    ] {
        assert!(
            sources.iter().any(|s| s == source),
            "{source} is not scanned"
        );
    }
    let texts = pages.iter().map(|page| (page, read(page))).chain(
        sources
            .iter()
            .map(|source| (source, doc_text(&read(source)))),
    );
    let offenses: Vec<String> = texts
        .flat_map(|(path, text)| {
            tool_list_overclaims(&text)
                .into_iter()
                .map(move |paragraph| format!("{path}: {paragraph}"))
        })
        .collect();
    assert_eq!(
        offenses,
        Vec::<String>::new(),
        "a living doc says a tool list bounds a seat; say what decides instead"
    );
}

#[test]
fn the_status_and_security_pages_are_linked_from_the_front() {
    for from in ["README.md", "ARCHITECTURE.md"] {
        let text = read(from);
        for link in ["](docs/status.md)", "](docs/security-model.md)"] {
            assert!(text.contains(link), "{from} does not link {link}");
        }
    }
}
