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

use std::collections::BTreeSet;

use brokkr_runtime::agents::{
    Adapter, ResumeAssessment, ResumeEvidence, ResumeIdentity, ResumeShape, ResumeStatus,
    ToolPermissions,
};
use brokkr_runtime::{Adapters, HarnessHands, TrustTier};

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
        (Some(ToolPermissions { names: grants, .. }), _) => names(grants.keys()),
        (None, Some(gap)) => {
            gaps.push(format!("`{provider}` tool allow-list: {gap}"));
            "no (measured)".to_string()
        }
        (None, None) => "no".to_string(),
    };
    let mcp = match mcp {
        Some(_) => "yes",
        None => "no",
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
    // The assessment's shape names are read off its one serialised form;
    // every fact about a shape is then read typed.
    let value = resume.value();
    let shapes: Vec<String> = value
        .as_object()
        .map(|shapes| shapes.keys().cloned().collect())
        .unwrap_or_default();
    let mut cells = Vec::new();
    for name in shapes {
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
        } = resume.shape(&name).expect("a named shape");
        let version = match identity {
            ResumeIdentity::Measured { version, .. } => version.as_str(),
            ResumeIdentity::Unknown { .. } => "version unknown",
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

/// The matrix and gap blocks as the adapter data renders them.
fn rendered(adapters: &Adapters) -> (String, String) {
    let mut matrix = String::from(
        "| Harness | Trust | Egress | Holds a model gate | Efforts | Tool allow-list | MCP servers | Boxed hands | Own sandbox for | Resume shapes |\n\
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

#[test]
fn the_status_and_security_pages_are_linked_from_the_front() {
    for from in ["README.md", "ARCHITECTURE.md"] {
        let text = read(from);
        for link in ["](docs/status.md)", "](docs/security-model.md)"] {
            assert!(text.contains(link), "{from} does not link {link}");
        }
    }
}
