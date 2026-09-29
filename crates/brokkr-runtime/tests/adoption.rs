//! Decision 0041 adoption pins: library-backed sites resolve the roster's
//! current first hire and the charter recorded in the compiled manifest,
//! whose digest is the witness table's pin (`witnesses.json`, #358).
//! Ruling 2 moved tools and model choices into one office definition.
//! Decision 0045 moves the self bundle's review site across the vendor
//! line: the last judge before ship is hired on codex's `astra`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use brokkr_runtime::{Bundle, PanelMember, SeatBody, StepBody};

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn compile(relative: &str) -> Bundle {
    let root = workspace();
    Bundle::compile_with(
        &root.join(relative),
        &root.join("agents"),
        &root.join("adapters"),
    )
    .unwrap_or_else(|e| panic!("{relative} must compile: {e}"))
}

#[path = "support/witnesses.rs"]
mod witnesses;

use witnesses::Witnesses;

/// Every invocation site of a bundle: label → (charter path, argv).
fn sites(bundle: &Bundle) -> BTreeMap<String, (PathBuf, Vec<String>)> {
    let mut out = BTreeMap::new();
    let member = |out: &mut BTreeMap<String, (PathBuf, Vec<String>)>,
                  prefix: &str,
                  members: &[PanelMember]| {
        for member in members {
            out.insert(
                format!("{prefix}{}", member.name),
                (member.role_path.clone(), member.command.clone()),
            );
        }
    };
    fn add_body(
        out: &mut BTreeMap<String, (PathBuf, Vec<String>)>,
        name: &str,
        body: &SeatBody,
        member: &impl Fn(&mut BTreeMap<String, (PathBuf, Vec<String>)>, &str, &[PanelMember]),
    ) {
        match body {
            SeatBody::Single {
                role_path, command, ..
            } => {
                out.insert(name.to_string(), (role_path.clone(), command.clone()));
            }
            SeatBody::Panel { members, .. } => member(out, &format!("{name}:"), members),
            SeatBody::Sequence { steps } => {
                for step in steps {
                    match &step.body {
                        StepBody::Single {
                            role_path, command, ..
                        } => {
                            out.insert(
                                format!("{name}:{}", step.name),
                                (role_path.clone(), command.clone()),
                            );
                        }
                        StepBody::Panel { members, .. } => {
                            member(out, &format!("{name}:{}:", step.name), members)
                        }
                        StepBody::Dialect { .. } => {}
                    }
                }
            }
            SeatBody::Select { cases, default, .. } => {
                for (case, body) in cases {
                    add_body(out, &format!("{name}:{case}"), body, member);
                }
                if let Some(body) = default {
                    add_body(out, &format!("{name}:default"), body, member);
                }
            }
        }
    }
    for (name, seat) in &bundle.seats {
        add_body(&mut out, name, &seat.body, &member);
    }
    out
}

/// site → (concrete model id, the charter under `agents/charters/` it
/// resolves). The charter's digest is the witness table's pin (#358).
type Roster = [(&'static str, &'static str, &'static str)];

const PANEL_REVIEW: &Roster = &[
    ("intake", "claude-sonnet-5-5", "intake.md"),
    ("implement", "claude-opus-5-5", "implementer.md"),
    ("review:correctness", "gpt-6-sol", "review-correctness.md"),
    ("review:security", "claude-fable-5-1", "review-security.md"),
];

const TRIAGE: &Roster = &[
    ("implement:design", "claude-opus-5-5", "implementer-sdd.md"),
    (
        "review:design:positions:spec-compliance",
        "claude-opus-5-5",
        "review-spec-compliance.md",
    ),
    (
        "review:design:positions:security",
        "claude-fable-5-1",
        "review-security.md",
    ),
    ("design:chief", "claude-fable-5-1", "chief-architect.md"),
    ("specify:author", "claude-fable-5-1", "chief-architect.md"),
    ("tasks:author", "claude-opus-5-5", "implementer-sdd.md"),
    ("clarify:judge", "claude-opus-5-5", "clarifier.md"),
    ("analyze:judge", "claude-fable-5-1", "analyst.md"),
    (
        "design:positions:simplicity",
        "claude-opus-5-5",
        "position-simplicity.md",
    ),
    (
        "design:positions:robustness",
        "gpt-6-sol",
        "position-robustness.md",
    ),
];
const SELF: &Roster = &[
    ("intake", "claude-sonnet-5-5", "intake.md"),
    ("implement", "claude-opus-5-5", "implementer.md"),
    ("review", "gpt-6-astra", "reviewer.md"),
];

fn expected_argv(site: &str, model: &str) -> Vec<String> {
    let (provider, mut argv) = if model.starts_with("gpt-") {
        ("codex", vec!["{brokkr}", "driver", "codex", "--"])
    } else {
        (
            "claude",
            vec![
                "{brokkr}",
                "driver",
                "claude",
                "--",
                "--permission-mode",
                "acceptEdits",
            ],
        )
    };
    let effort = match site {
        "design:chief" | "specify:author" => "max",
        "clarify:judge" | "analyze:judge" => "xhigh",
        "review" => "xhigh",
        site if site.ends_with(":security") || site.ends_with(":chief") => "xhigh",
        _ => "high",
    };
    argv.extend(["--model", model, "--effort", effort]);
    if site == "review"
        || site.starts_with("review:")
        || matches!(
            site,
            "design:chief" | "specify:author" | "clarify:judge" | "analyze:judge"
        )
    {
        let hands: &[&str] = match provider {
            "codex" => &[
                "--sandbox",
                "read-only",
                "-c",
                "mcp_servers.brokkr.command=\"{brokkr}\"",
                "-c",
                "mcp_servers.brokkr.args={hands_args_toml}",
                "-c",
                "mcp_servers.brokkr.default_tools_approval_mode=\"approve\"",
            ],
            _ => &[
                "--tools",
                "",
                "--strict-mcp-config",
                "--mcp-config",
                "{hands_mcp_json}",
                "--allowedTools",
                "mcp__brokkr__workspace",
            ],
        };
        argv.extend(hands);
    } else {
        let tools = match site {
            "implement" => Some("Bash(cargo:*),Bash(git:*)"),
            site if site.starts_with("implement:") => Some("Bash(cargo:*),Bash(git:*)"),
            "tasks:author" => Some("Bash(cargo:*),Bash(git:*)"),
            "intake" => Some("Bash(git:*)"),
            _ => None,
        };
        if let Some(tools) = tools {
            argv.extend(["--allowedTools", tools]);
        }
    }
    argv.into_iter().map(str::to_string).collect()
}

fn assert_adopted(relative: &str, roster: &Roster) {
    let bundle = compile(relative);
    let sites = sites(&bundle);
    let charters = Witnesses::load(&workspace()).charters;
    for (site, model, charter_name) in roster {
        let charter_digest = &charters[*charter_name];
        let (charter, argv) = sites
            .get(*site)
            .unwrap_or_else(|| panic!("{relative} has no site '{site}'"));
        // Decision 0041 moves the roster deliberately: pin the selected
        // concrete generation, while each agent's own definition now owns
        // the full argv, effort and tool grant. Keep this element-for-element:
        // an `iter().any` check let a required grant disappear from the rest
        // of the resolved Claude allow-list, which is the defect this pin guards.
        let mut normalized_argv = argv.clone();
        normalized_argv[0] = "{brokkr}".to_string();
        assert_eq!(
            normalized_argv,
            expected_argv(site, model),
            "{relative} site '{site}' resolved a different argv"
        );
        let bytes = std::fs::read(charter).unwrap();
        assert_eq!(
            brokkr_core::canonical::sha256_bytes(&bytes),
            *charter_digest,
            "{relative} site '{site}' charter moved without a witness update"
        );
        // The pin that replaces the `manifest.files` entry the charter
        // lost by moving out of the recipe directory.
        assert_eq!(
            bundle.manifest["agents"][*site]["charter_digest"],
            *charter_digest
        );
    }
}

#[test]
fn panel_review_resolves_to_what_it_used_to_inline() {
    assert_adopted("recipes/panel-review", PANEL_REVIEW);
}

#[test]
fn triage_cases_resolve_to_the_roster() {
    assert_adopted("recipes/triage", TRIAGE);
}

#[test]
fn self_resolves_to_what_it_used_to_inline() {
    assert_adopted("bundles/self", SELF);
}

/// Triage's boxed validator now comes from the realm dialect. It remains a
/// deterministic exec with no model or charter-as-prompt semantics.
#[test]
fn the_design_validator_is_supplied_by_the_dialect() {
    let bundle = compile("recipes/triage");
    let SeatBody::Sequence { steps } = &bundle.seats["design"].body else {
        panic!("design is a sequence")
    };
    let step = steps
        .iter()
        .find(|step| step.name == "validate")
        .expect("the validate step");
    let StepBody::Dialect { execution } = &step.body else {
        panic!("validate is a dialect step")
    };
    assert_eq!(execution.argv[0], "openspec");
    assert_eq!(execution.argv[1], "validate");
    assert!(bundle.manifest["agents"].get("design:validate").is_none());
}

/// Rulings 4 and 5 deliberately change the 0007 declarations: judge-fix
/// state is gone everywhere, and only the design-bearing table admits the
/// specification-defect finding that its panel can return.
#[test]
fn adopting_review_seats_declare_only_the_findings_their_tables_can_read() {
    for (relative, expected) in [
        ("bundles/self", vec!["max_residual_severity"]),
        (
            "recipes/panel-review",
            vec!["has_security_residual", "max_residual_severity"],
        ),
        (
            "recipes/triage",
            vec![
                "spec_defect",
                "has_security_residual",
                "max_residual_severity",
            ],
        ),
    ] {
        let bundle = compile(relative);
        assert_eq!(
            bundle.seats["review"].inputs,
            expected.into_iter().map(str::to_string).collect::<Vec<_>>(),
            "{relative}"
        );
    }
}

/// The 0006 bounds an adopting seat used to write inline now come from
/// its agent, unchanged.
#[test]
fn adoption_did_not_change_any_seats_limits() {
    let expected: BTreeMap<&str, (u64, u64)> = [
        ("intake", (2, 1800)),
        ("implement", (2, 5400)),
        ("verify", (2, 3600)),
        ("review", (2, 3600)),
        ("ship", (2, 1800)),
    ]
    .into_iter()
    .collect();
    for relative in ["bundles/self", "recipes/panel-review"] {
        let bundle = compile(relative);
        for (phase, (attempts, seconds)) in &expected {
            let limits = bundle.seats[*phase].limits;
            assert_eq!(limits.max_attempts, *attempts, "{relative} {phase}");
            assert_eq!(limits.timeout_seconds, *seconds, "{relative} {phase}");
        }
    }
}
