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
/// So is the sweep's class, each word held for the sentence it replaced:
/// a gate said to change nothing or never to write, where the engine
/// checks only that HEAD did not move; the box said to plant no hook or
/// to leave the host untouched, where it binds the common git directory
/// read-write; a verifier, a bind mode or a boundary said to hold with
/// no boundary named; a tool list called a restriction or a seat's
/// bound; and a dead hands server's tree said to leak, where the next
/// run's start reaps it.
///
/// So are the recipes' gates said to be boxed, or to run with the network
/// refused, with no boundary named: under `harness` a pinned exec script
/// runs unboxed in a rebuilt environment and no network denial is
/// reported. A recipe whose dialect steps compile only under `namespace`
/// (`triage`, `night-shift`) may say boxed, and says why.
///
/// So are the landing review's last sentences of the class: preflight's
/// review gate called read-only, where it runs unboxed under
/// `acceptEdits`, and a fixes-applied rule no shipped table carries;
/// panel-review said to run unboxed under `harness`, which refuses it;
/// a harness seat told the result file is the one file its sandbox
/// writes, where a codex work seat's `workspace-write` writes the whole
/// workspace; a boxed seat told its worktree is reachable only through
/// the workspace tool, where a boxed codex seat keeps its native
/// read-only shell; a verifier script's header saying the box denies
/// network, and a recipe table saying its gate is boxed, with no
/// boundary named; and seat commits said to land unsigned, where an
/// unboxed seat's `git` reads the host's signing configuration.
///
/// So are the final landing review's: the standby row's gates called
/// boxed with no boundary named; preflight's table said to produce only
/// a ruling, a landing table saying nothing changes, and a review rule's
/// reason saying a gate changes nothing or no code changed, where the
/// engine parks a moved HEAD and only ship checks the tree; GPT agents
/// said to keep workspace restrictions two of them never declare;
/// release's implement office called boxed, and the harness split's
/// stale count; release said not to expose the host's GitHub
/// credentials, which a boxed Codex seat reads; dsh's sandbox said to
/// write only the session workspace, where the runner adds two mounts;
/// and a model gate under `open` said refused outright, where only one
/// with hands is.
///
/// A comparison that only implies a bound, such as one arm called no
/// narrower than another, is outside the guard: its wording names no
/// control a list could hold.
const ANYWHERE: [&str; 128] = [
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
    "read and never write",
    "box bounds only the workspace calls",
    "plants no git hook",
    "the box expresses the restriction",
    "gates change nothing",
    "the verifier is a boxed script",
    "decision 0043: the box;",
    "judges only and changes nothing",
    "controlled agent effects",
    "brokkr-like effect authorization",
    "grant/effect boundary",
    "boxes its verify and ship gates",
    "read-only subset for review",
    "sandbox holds each seat's hands",
    "keeps a verify seat from installing",
    "gate agents (verify, review, ship)",
    "boxed hands declared at the site",
    "no — reports findings for the implementer",
    "nobody pushes, nobody merges",
    "list in each seat's driver",
    "read-only sdd judge",
    "are boxed, inline exec scripts",
    "holding the worktree read-write and the host toolchain",
    "the box clears the environment",
    "and reports, it never writes",
    "and it never writes",
    "box lacks",
    "boxed and offline",
    "nothing in this repository pushes",
    "re-express every restriction",
    "which boundary builds them",
    "the box replaces the tool list",
    "workdir mounted writable, the declared binds",
    "overlay (the host path as a read-only lower layer",
    "the boundary that enforces it",
    "the read-only judge",
    "its read-only judge",
    "nothing here can execute it",
    "leak their scratch trees",
    "scratch tree stays under",
    "own sandbox stands, as its adapter's fragment",
    "the git directory's hooks",
    "for a linked worktree the box binds",
    "every restriction expressible",
    "boxed verify seat and an unboxed review seat under",
    "policy and boxed exec gates",
    "offline and inside the box",
    "deterministic boxed exec gates",
    "lint tool the box cannot reach",
    "offline inside the box",
    "network remains refused",
    "boxed, no model",
    "boxed exec verifier",
    "boxed exec shipper",
    "the gate is a boxed script",
    "boxed, no network",
    "declare boxed hands",
    "verifier is a boxed exec script",
    "network is refused",
    "verify uses boxed exec",
    "cannot reach the network",
    "boxed registry gate",
    "same boxed exec gate",
    "own boxed exec script",
    "boxed ship seat",
    "its boxed gates",
    "| boxed exec scripts |",
    "the review offices' already are",
    "that are boxed stay boxed",
    "fast's boxed verifier",
    "fit and security. read-only.",
    "applied fixes hard-stops",
    "no network denial reported, under harness",
    "one file that sandbox lets",
    "one file the sandbox lets",
    "result file, are reachable only through",
    "verifier seat. the box denies network",
    "no model, boxed like the verifier",
    "one boxed gate proves",
    "seat commits land unsigned",
    "boxed verify and ship gates unchanged",
    "can only produce a ruling",
    "findings only, nothing changes",
    "a gate changes nothing",
    "no code changed",
    "retain their existing workspace restrictions",
    "boxed implement office",
    "the eleven whose hands sites",
    "does not expose the host's github credentials",
    "writes only under the session workspace",
    "a model gate is refused outright",
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

/// A Rust source's string literals as Markdown: each literal's text with
/// its escapes decoded, a line continuation joined, and a paragraph break
/// between literals, so a sentence a generated file carries is read
/// where it is written. Comments are stepped over (`doc_text` reads the
/// doc comments), and so are char literals and lifetimes.
fn literal_text(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut literals = Vec::new();
    let mut at = 0;
    while at < chars.len() {
        let next = |ahead: usize| chars.get(at + ahead).copied();
        at = match (chars[at], next(1)) {
            ('/', Some('/')) => past(&chars, at, "\n"),
            ('/', Some('*')) => past(&chars, at + 2, "*/"),
            ('\'', Some('\\')) => past(&chars, at + 3, "'"),
            ('\'', _) if next(2) == Some('\'') => at + 3,
            ('r', Some('"' | '#')) => raw_literal(&chars, at + 1, &mut literals),
            ('"', _) => literal(&chars, at + 1, &mut literals),
            _ => at + 1,
        };
    }
    literals.join("\n\n")
}

/// The index just past the first `end` at or after `from`, or the end.
fn past(chars: &[char], from: usize, end: &str) -> usize {
    let end: Vec<char> = end.chars().collect();
    (from..chars.len())
        .find(|&at| chars[at..].starts_with(&end))
        .map_or(chars.len(), |at| at + end.len())
}

/// A raw literal whose `#`s start at `from`; not one when no `"` follows
/// them, as in a raw identifier.
fn raw_literal(chars: &[char], from: usize, literals: &mut Vec<String>) -> usize {
    let hashes = chars[from..].iter().take_while(|c| **c == '#').count();
    if chars.get(from + hashes) != Some(&'"') {
        return from;
    }
    let start = from + hashes + 1;
    let close: String = std::iter::once('"')
        .chain("#".repeat(hashes).chars())
        .collect();
    let end = past(chars, start, &close);
    let body = chars[start..end.saturating_sub(close.len()).max(start)].iter();
    literals.push(body.collect());
    end
}

/// A quoted literal whose body starts at `from`, escapes decoded.
fn literal(chars: &[char], from: usize, literals: &mut Vec<String>) -> usize {
    let mut text = String::new();
    let mut at = from;
    while at < chars.len() && chars[at] != '"' {
        if chars[at] != '\\' {
            text.push(chars[at]);
            at += 1;
            continue;
        }
        let escaped = chars.get(at + 1).copied().unwrap_or('\\');
        at += 2;
        match escaped {
            '\n' => at += chars[at..].iter().take_while(|c| c.is_whitespace()).count(),
            'n' => text.push('\n'),
            't' => text.push('\t'),
            'r' => text.push('\r'),
            '0' => text.push('\0'),
            'x' => {
                let hex: String = chars[at..(at + 2).min(chars.len())].iter().collect();
                text.extend(u8::from_str_radix(&hex, 16).ok().map(char::from));
                at += 2;
            }
            'u' => {
                let end = past(chars, at, "}");
                let hex: String = chars[at + 1..end - 1].iter().collect();
                text.extend(u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32));
                at = end;
            }
            other => text.push(other),
        }
    }
    literals.push(text);
    at + 1
}

/// Excerpts of the pages this story reworded, word for word as they
/// stood: each holds one paragraph the guard refuses.
const OLD_PAGES: [&str; 128] = [
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
    // docs/research/0003, 0004, 0005, 0006 and 0007's citations
    "decision 0041: gates read and never write, so it writes no tests",
    "decision 0043: the box bounds only the workspace calls of a gate whose agent declares hands",
    "decision 0043: a boxed workspace call finds the git hooks directory empty and the git config \
     read-only, so it plants no git hook",
    "decision 0043: a seat's hands are one workspace tool, and the box expresses the restriction",
    "decision 0041: gates change nothing and the engine checks; the reviewed head is recorded",
    "decision 0043: the verifier is a boxed script, not the implementer's word",
    "decision 0002: the journal is the state and the table is the stop condition; decision 0043: \
     the box; decision 0021: the checker is a gate seat",
    "decision 0041: a gate hires judges only and changes nothing;",
    // docs/research/github-peers/
    "It is a direct architecture peer for Brokkr's controlled agent effects and inspectable execution",
    "These choices support unattended operation, but they do not enforce Brokkr-like effect \
     authorization boundaries.",
    "Brokkr's grant/effect boundary and dispatcher work should test every externally reachable path",
    // docs/guides/quickstart.md
    "Every shipped bundle boxes its verify and ship gates, and what stands\n\
     between a boxed seat's hands and your machine is the realm's",
    "`Bash(<bin>:*)` entries, and each model agent's `tools.allow` names them —\n\
     the whole set for the work seats and the read-only subset for review.",
    "`adapters/codex.json` in place of `adapters/claude.json`, hires every\n\
     seat from codex, and declares the `harness` boundary in `realms.json`,\n\
     so codex's own sandbox holds each seat's hands.",
    // docs/guides/starters/bun.md, go.md and node.md
    "it is each gate's charter — \"prove it, fix nothing\", with no install\n\
     line — and not the grant that keeps a verify seat from installing.",
    "The work agents (`intake`, `implement`) carry all five names in\n\
     `tools.allow`; the gate agents (`verify`, `review`, `ship`) carry the\n\
     same minus `mkdir`.",
    // docs/guides/starters/rust.md
    "Verify and ship are deterministic scripts with their limits and\n\
     boxed hands declared at the site.",
    // docs/guides/adopting-a-node-repo.md
    "| `review` | gate | reads the diff for correctness, simplicity and security | no — reports \
     findings for the implementer |",
    "Nobody pushes, nobody merges, and nobody publishes — no `npm publish`,\n\
     no `npm version`, no tag. That authority is yours.",
    "swap point for pnpm and yarn: the `--allowedTools` list in each seat's\n\
     driver, and the install/type-check/test commands plus the lockfile name",
    // docs/guides/agent-library.md
    "analyst\tfable → sol → opus\tRead-only SDD judge: finds drift across the artifacts and the \
     realm constitution.",
    "They are boxed, inline `exec` scripts with no model: verification runs a\n\
     recipe's fixed checks",
    "serve`, and every call to it executes inside an empty-root bubblewrap\n\
     namespace holding the worktree read-write and the host toolchain\n\
     read-only.",
    // docs/guides/secrets.md and docs/security-model.md
    "all: the box clears the environment, and compilation refuses it",
    // docs/guides/journal-and-verification.md
    "across a gate's own span: a gate reads and reports, it never writes",
    // CONTRIBUTING.md
    "and CI proves the rest (any non-Rust lint whose tool the seat's box lacks, which the seat \
     names as not run,",
    // recipes/triage/README.md
    "Each loop begins with the dialect's deterministic\n\
     check, whose result is passed to its read-only judge.",
    // recipes/research/README.md
    "It opens no issue and no pull request, and it never writes\n\
     `Status: ruled`.",
    // docs/guides/contributing-by-hand.md
    "landing's verify seat runs the same list with `--seat`, which names a\n  \
     lint whose tool its box lacks as not run and runs the others",
    "`cargo test --workspace` and `cargo run -p brokkr-cli -- compile\n\
     --bundle bundles/self`, boxed and offline, in that order",
    "- **The operator keeps push and merge.** Nothing in this repository\n  \
     pushes on your behalf, and no agent merges anything.",
    // docs/guides/driver-authoring.md
    "- **Re-express every restriction the seat declared.** If your harness's\n  \
     resume path drops a sandbox class, a permission mode or a tool\n  \
     allow-list, put it back explicitly.",
    "A driver that\n\
     wants walls declares `hands` on its site and lets the realm say which\n\
     boundary builds them.",
    // docs/guides/provider-adapters.md
    "(decision [0043](../decisions/0043-the-hands-are-one-tool.md) ruling 2:\n\
     the box replaces the tool list).",
    "What that confinement **is**: the box decision 0043 already builds — an\n\
     empty root, the run's workdir mounted writable, the declared binds in\n\
     their declared modes, and the declared network (here, none).",
    // docs/guides/recipe-authoring.md
    "with mode `ro`, `rw` or `overlay` (the host path as a read-only lower layer, writes kept in \
     a per-seat upper layer that never touches the host — the mode for a toolchain cache).",
    "`hands` is the **policy** — what the seat may reach; the **boundary** that enforces it is \
     the realm's, never the bundle's",
    "their dialect check before the read-only judge, which receives that output as\n\
     `prior_results`.",
    // docs/guides/read-surfaces.md
    "Nothing it proposes is executed, and nothing here can execute it.",
    // docs/status.md and docs/security-model.md
    "- **Dead hands servers leak their scratch trees under `/tmp`**\n  \
     ([#415](https://github.com/feedback-loop-ai/brokkr/issues/415)).",
    "- **Process settlement.** A timed-out attempt's detached descendants can\n  \
     outlive the kill, and a dead hands server's scratch tree stays under\n  \
     `/tmp`.",
    "| `harness` | Nothing of Brokkr's. The harness's own sandbox stands, as its adapter's \
     fragment addresses it. |",
    "`/runtime/bundle`. The git directory's `hooks` sit behind an empty\n  \
     tmpfs and its `config` is read-only.",
    "- **The git common directory is writable.** For a linked worktree the\n  \
     box binds the shared git directory read-write, so a boxed command can\n  \
     move a sibling worktree's branch, rewrite the object store or another\n  \
     worktree's `config.worktree`.",
    // ARCHITECTURE.md
    "resolve --> checks{\"gate site → trusted tier?<br/>secret bindings → route class ≥ \
     minimum?<br/>every restriction expressible?\"}",
    "| Brokkr verification | `bundles/verify` examines a delivered change with a boxed verify \
     seat and an unboxed review seat under the operator's Claude Code permissions",
    "`recipes/release` combines the manager and library reviewer with `fast`'s policy\n\
     and boxed exec gates.",
    // recipes/fast/README.md
    "The default Rust delivery recipe. Its verifier runs, cheapest first,\n\
     every check CI requires that works offline and inside the box (#427):",
    "`bundles/self` compile. Its shipper renders the journal with `brokkr\n\
     ledger`. Both are deterministic boxed exec gates, and every recipe that\n\
     extends `fast` without its own verifier, `landing` among them, runs this\n\
     one.",
    "A lint tool the box cannot reach, because it is not on the box's `PATH`\n\
     or does not report the version CI pins, is named in the verifier's notes\n\
     as not run; the result is `pass` only if every check that did run passed.",
    "Cargo runs offline inside the box from the bound registry cache.",
    "If a\n\
     dependency is not cached, network remains refused, the command fails\n\
     closed, and the verifier's `fail` notes quote Cargo's decisive cache or\n\
     offline error.",
    // recipes/landing/README.md
    "| `classify` | `gate` | `docs`, `code` | `scripts/classify-seat.sh`, boxed, no model: \
     every path the branch changes against the default branch is read against the \
     repository's own docs class",
    "| `verify` | `gate` | `pass`, `fail` | `fast`'s boxed exec verifier, cheapest first: \
     format,",
    "| `ship` | `gate` | `ready`, `shipped` | `fast`'s boxed exec shipper: a clean tree, the \
     head the engine gated on, the ledger, the anchor with the per-file patch map. |",
    // recipes/research/README.md
    "The work seat is the library's `researcher`\n\
     office; the gate is a boxed script.",
    "- **verify** (`roles/verify-seat.sh`, boxed, no network): the tree is\n  \
     clean, nothing outside `docs/research/` changed,",
    // recipes/node/README.md
    "Decision 0043 permits the other two to\n\
     hold that class only because their complete `exec` dispatches declare\n\
     boxed hands; neither script seats a model.",
    "The verifier is a boxed exec script beside this recipe's roles.",
    "Network\n\
     is refused: `npm ci --offline` can use only the bound `~/.npm` cache\n\
     artifacts,",
    // recipes/preflight/README.md
    "Both\n\
     seats here declare `\"class\": \"gate\"`: verify uses boxed `exec`, while\n\
     review names the trusted model driver.",
    "Cargo runs\n\
     offline from the bound registry cache; an uncached dependency cannot\n\
     reach the network, so the gate fails closed",
    // recipes/research-dsh/README.md
    "Everything else is\n\
     inherited: the boxed registry gate, the ten-entry cap,",
    // recipes/panel-review/README.md
    "Its shipper is the same boxed exec gate as `fast`'s.",
    // recipes/release/README.md
    "or extend `release` and explicitly override\n\
     `verify` with the stack's own boxed exec script and required toolchain binds.",
    // recipes/review-first/README.md
    "and a clean\n\
     verified branch ships through the boxed ship seat.",
    // recipes/standby/README.md
    "`fast`'s shape, its contracts and its boxed gates, with both model seats\n\
     on the other vendor.",
    "| verify, ship | boxed exec scripts | unchanged — no model, no vendor |",
    "When an implementer's hands are boxed the way\n\
     the review offices' already are, the allow-list is not consulted at all",
    "The gates that *are* boxed\n  \
     stay boxed.",
    // recipes/wager-harness/README.md and its two arms
    "The harness inherits `fast`'s boxed verifier and shipper by construction.",
    // recipes/preflight/README.md, as the landing review found it
    "| `review` | `gate` | `clean`, `residual`, `security-hold` | Adversarial read of \
     `git diff main...HEAD` across correctness, fit and security. Read-only. |",
    "your machine with your credentials and are charged to touch nothing; the\n\
     table's part is that a seat which reports having applied fixes hard-stops\n\
     the run (`REVIEW-CLEAN-FIXED`, `REVIEW-RESIDUAL-FIXED`) instead of being\n\
     believed.",
    // recipes/panel-review/README.md
    "Both gates are boxed with no\n\
     network under a `namespace` boundary, and unboxed, with no network\n\
     denial reported, under `harness`.",
    // openspec/specs/boundary-record/spec.md
    "and — with `file` — says the result path is the one file that sandbox\n\
     lets it write, or — with `last-message` — says the seat's final message",
    "- **THEN** the input carries `boundary: harness`, no `hands` marker and no \
     `result_delivery`, and the paragraph names `harness`, does not name \
     `mcp__brokkr__workspace`, and says the result path is the one file the sandbox lets \
     the seat write",
    // docs/security-model.md
    "  Seat commits land unsigned in the worktree, and the operator reviews\n  \
     the branch, pushes and merges. That review is the last check.",
    // docs/guides/recipe-authoring.md's standby row, as the final landing review found it
    "| [`standby`](../../recipes/standby/README.md) | delivering while one vendor's account is \
     out of limit | `extends fast`: both model seats pinned inline to codex `sol` \
     (`gpt-6.1-sol`), the boxed verify and ship gates unchanged; a hedge forces its crew, so it \
     takes no fallback |",
    // recipes/landing/README.md
    "| Ends at | `review` — findings only, nothing changes, nothing merges | `ship` — a vouched \
     head, or a stop with the reason |",
    // recipes/gpt-flash/README.md
    "support workspace hands or named tool filtering; the Flash agents therefore\n\
     declare neither. GPT agents retain their existing workspace restrictions. Flash panel \
     positions",
    // docs/guides/quickstart.md
    "Which shipped bundles run under `harness` today is a fact of the tree,\n\
     not a promise: the eleven whose hands sites are their own `./` exec gates",
    "record of that split. The fifth refusal is `recipes/release`: its boxed\n\
     `implement` office reaches claude without a measured `hands.harness.work`\n\
     fragment before compilation reaches the review gate.",
    // recipes/release/README.md
    "private targets need an explicitly configured access mechanism or preparation\n\
     reports the missing access. It does not expose the host's GitHub credentials.",
    // docs/guides/provider-adapters.md
    "A dsh **work** seat under `harness` is confined by dsh's own sandbox,\n\
     which writes only under the session workspace.",
    "| `gate` | The argv fragment that puts a gate-class seat in the harness's read-only class. \
     A model gate is admitted under `harness` only when **every** link of its resolved chain \
     declares one (decision 0046 ruling 4); under `open` a model gate is refused outright. |",
];

/// Excerpts of the shell scripts and recipe data this story reworded, as
/// the guard reads them: a script's comments, a recipe's JSON strings.
const OLD_DATA: [(&str, &str); 6] = [
    // scripts/verify-seat.sh, and its five pinned copies in bundles/ and recipes/
    (
        "verify-seat.sh",
        "#!/usr/bin/env bash\n\
         # Deterministic verifier seat. The box denies network; Cargo is also told\n\
         # explicitly to stay offline so a cache miss is reported as such.\n\
         set -u\n",
    ),
    // recipes/landing/scripts/classify-seat.sh
    (
        "classify-seat.sh",
        "# base matches the class, `code` otherwise. Seconds, no model, boxed like\n\
         # the verifier. Anything it cannot establish is answered `code`: a\n",
    ),
    // recipes/research/policy.json
    (
        "policy.json",
        r#"{"description": "Research intake (decision 0044): one work seat reads the articles a commission names or finds and proposes registry entries; one boxed gate proves the registry still parses"}"#,
    ),
    // recipes/preflight/policy.json
    (
        "policy.json",
        r#"{"description": "The table has no intake, no implement and no ship phase and it ends after review — so a preflight run can only produce a ruling, never a change and never a merge."}"#,
    ),
    // recipes/fast/policy.json and recipes/review-first/policy.json
    (
        "policy.json",
        r#"{"rules": [{"id": "REVIEW-CLEAN", "reason": "Review clean; a gate changes nothing, so verification evidence stands."}]}"#,
    ),
    // recipes/node/policy.json, recipes/panel-review/policy.json and bundles/self/policy.json
    (
        "policy.json",
        r#"{"rules": [{"id": "REVIEW-CLEAN", "reason": "Review clean and no code changed; verification evidence stands."}]}"#,
    ),
];

/// Excerpts of the doc comments this story reworded, as the sources
/// carried them.
const OLD_SOURCES: [&str; 18] = [
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
    // crates/brokkr-protocol/src/adapters.rs
    "/// - `boundary: harness` — the harness's own sandbox stands, no\n\
     ///   workspace tool is served, and the result reaches the engine through\n\
     ///   the door the input names: the one file the sandbox lets the seat\n\
     ///   write, or the seat's final message, which the harness captures;",
];

/// Excerpts of the string literals this story reworded, as the sources
/// carried them: text a generated file or an error message says.
const OLD_LITERALS: [&str; 6] = [
    // crates/brokkr-cli/src/init.rs, the scaffold README's tool grants
    r#""{gate_list}. Verify and ship are boxed scripts with no model grant.\n\n\
                 The grant is per BINARY, not per subcommand""#,
    // crates/brokkr-runtime/src/agents/load.rs
    r#""{what} 'tools.allow' is empty, which is ambiguous between \
                     'no restriction' and 'restrict to nothing'; omit the key to \
                     declare no restriction""#,
    // crates/brokkr-runtime/src/bundle.rs
    r#""seat '{what}' declares hands and secret bindings {secrets:?}; the box \
                     clears the environment, so a boxed seat cannot receive a binding \
                     (decision 0043)""#,
    // crates/brokkr-protocol/src/adapters.rs, the boxed and harness notices
    r#""\n\nYour hands are boxed: the worktree, and this result file, are \
         reachable ONLY through the `{workspace}` tool. Your \
         harness's own shell runs outside the box and cannot write here""#,
    r#""\n\nYour hands stand under the `harness` boundary: no workspace \
         tool of Brokkr's is served, and you run under your harness's own sandbox. The \
         result path above is the one file that sandbox lets you write; write it yourself.""#,
    // crates/brokkr-cli/src/init.rs, the scaffold's phase table
    r##"r#"    {"id": "REVIEW-CLEAN-NO-FIXES", "from": "review", "result": "clean",
     "when": {"fixes_applied": false}, "next": "ship",
     "reason": "Clean with no code changed; verification evidence stands."},"#"##,
];

/// A shell script's comments as Markdown: each `#` line's text, and a
/// paragraph break at every line that is not one, the `#!` line included.
fn comment_text(source: &str) -> String {
    source
        .lines()
        .map(|line| match line.trim_start().strip_prefix('#') {
            Some(text) if !text.starts_with('!') => text,
            Some(_) | None => "",
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A JSON file's strings as Markdown, a paragraph break between strings.
/// A file that does not parse fails the guard rather than reading as
/// nothing.
fn json_text(path: &str, source: &str) -> String {
    fn strings(value: &serde_json::Value, out: &mut Vec<String>) {
        match value {
            serde_json::Value::String(text) => out.push(text.clone()),
            serde_json::Value::Array(items) => items.iter().for_each(|item| strings(item, out)),
            serde_json::Value::Object(map) => map.values().for_each(|item| strings(item, out)),
            serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {
            }
        }
    }
    let value = serde_json::from_str(source).unwrap_or_else(|error| panic!("{path}: {error}"));
    let mut out = Vec::new();
    strings(&value, &mut out);
    out.join("\n\n")
}

/// What the guard reads in one file: a Rust source's doc comments and
/// string literals, a script's comments, a JSON file's strings, and a
/// page as written.
fn guarded_texts(path: &str, source: &str) -> Vec<String> {
    match path.rsplit_once('.').map(|(_, extension)| extension) {
        Some("rs") => vec![doc_text(source), literal_text(source)],
        Some("sh") => vec![comment_text(source)],
        Some("json") => vec![json_text(path, source)],
        _ => vec![source.to_string()],
    }
}

/// Every old excerpt as the guard reads it: a page as written, a source
/// as its doc comments or its string literals, a script or recipe datum
/// as its comments or strings.
fn old_texts() -> Vec<String> {
    let pages = OLD_PAGES.iter().map(|page| page.to_string());
    let sources = OLD_SOURCES
        .iter()
        .map(|source| doc_text(&format!("{source}\nfn f() {{}}\n")));
    let literals = OLD_LITERALS.iter().map(|source| literal_text(source));
    let data = OLD_DATA
        .iter()
        .flat_map(|(path, source)| guarded_texts(path, source));
    pages.chain(sources).chain(literals).chain(data).collect()
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
    // A gate script's comments and a recipe's data say the class too.
    let data: Vec<String> =
        tracked_files::tracked(&root, &["*.sh", "recipes/*.json", "bundles/*.json"])
            .into_iter()
            .filter(|path| !RECORDS.iter().any(|record| path.starts_with(record)))
            .collect();
    for path in ["scripts/verify-seat.sh", "recipes/research/policy.json"] {
        assert!(data.iter().any(|p| p == path), "{path} is not scanned");
    }
    let texts = pages.iter().chain(&sources).chain(&data).flat_map(|path| {
        let text = read(path);
        guarded_texts(path, &text)
            .into_iter()
            .map(move |text| (path, text))
    });
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
