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

use std::collections::BTreeSet;

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

/// Every named resume shape with its status and measured version; the
/// reason of each shape that is not supported goes to the gaps.
fn resume_shapes(provider: &str, resume: &ResumeAssessment, gaps: &mut Vec<String>) -> String {
    let mut cells = Vec::new();
    for (name, shape) in resume.shapes() {
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
/// allow list it is called.
const TOOL_LIST: [&str; 8] = [
    "tool list",
    "tools.allow",
    "tool grant",
    "--allowedtools",
    "tool restriction",
    "allow list",
    "allow-list",
    "allowlist",
];

/// Wording that says a tool list bounds a seat. `--allowedTools`
/// pre-approves and removes no tool, so an unboxed claude seat is bounded
/// by Claude Code's permission model and the operator's own settings and
/// MCP servers, never by its tool list alone (#467). The resolver's
/// "more power than it declares" is not here: a provider that cannot
/// express a tool list refuses it, and the docs quote that refusal.
const OVERCLAIMS: [&str; 17] = [
    "bounded what the model may run",
    "blast radius",
    "not a command it may run",
    "enforced rather than documented",
    "seats run under",
    "only restriction",
    "bounded only by",
    "bounded by its tool list",
    "restricted to its tool list",
    "restricted by its tool list",
    "limited to its tool list",
    "can only use",
    "may only use",
    "may run only",
    "may run exactly",
    "may run nothing outside",
    "decides what the seats may run",
];

/// Each paragraph of a Markdown text with wording that says a tool list
/// bounds the seat, where it or the paragraph before names the tool list,
/// so a list introduced above its bullets cannot hide one. Paragraphs are
/// lowercased with their code and emphasis marks dropped and their line
/// breaks joined, so neither can hide one either.
fn tool_list_overclaims(text: &str) -> Vec<String> {
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
    let names_a_tool_list =
        |paragraph: &str| TOOL_LIST.iter().any(|anchor| paragraph.contains(anchor));
    paragraphs
        .iter()
        .enumerate()
        .filter(|(_, paragraph)| OVERCLAIMS.iter().any(|claim| paragraph.contains(claim)))
        .filter(|(at, paragraph)| {
            names_a_tool_list(paragraph)
                || at
                    .checked_sub(1)
                    .is_some_and(|before| names_a_tool_list(&paragraphs[before]))
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
const OLD_PAGES: [&str; 7] = [
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
];

/// Excerpts of the module docs this story reworded, as the sources
/// carried them.
const OLD_SOURCES: [&str; 3] = [
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
];

#[test]
fn the_guard_refuses_every_sentence_this_story_reworded() {
    for page in OLD_PAGES {
        assert_eq!(tool_list_overclaims(page).len(), 1, "not refused:\n{page}");
    }
    for source in OLD_SOURCES {
        let docs = doc_text(&format!("{source}\nfn f() {{}}\n"));
        assert_eq!(
            tool_list_overclaims(&docs).len(),
            1,
            "not refused:\n{source}"
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
    let sources = tracked_files::tracked(&root, &["crates/*/src/*.rs"]);
    for source in [
        "crates/brokkr-protocol/src/hands.rs",
        "crates/brokkr-runtime/src/agents.rs",
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
