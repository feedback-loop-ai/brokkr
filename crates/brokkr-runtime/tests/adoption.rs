//! Decision 0041 adoption pins: library-backed sites resolve the roster's
//! current first hire and the charter recorded in the compiled manifest.
//! The fable pins deliberately moved to `claude-fable-5-1` under ruling 1;
//! ruling 2 moved tools and model choices into one office definition.
//! Rulings 4 and 5 move the charter witnesses again: judges no longer fix,
//! implementers answer returned findings, and spec compliance can return a
//! defective specification to design.
//! Ruling 8 moves the affected charter witnesses once more: repository rules
//! now come from the realm's house rather than the portable office.

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

/// site → (concrete model id, current charter digest).
type Roster = [(&'static str, &'static str, &'static str)];

const PANEL_REVIEW: &Roster = &[
    (
        "intake",
        "claude-sonnet-5",
        "fbdb7dba8e34fbc0b02e0f7fd7540fd0ab9313e40cdbcb03c27c22d78c138756",
    ),
    (
        "implement",
        "claude-opus-5",
        "b750b0a401fa7fc1aad5dd929bf136cf961b12d2e11ac9fc67995927ea686ad7",
    ),
    (
        "review:correctness",
        "gpt-5.6-sol",
        "7d11cd3201c6bf9464b7092e456ad0e432772aa7cf0fee28d3b18782733b172b",
    ),
    (
        "review:security",
        "claude-fable-5-1",
        "33d6b92f2a349636e60cb9a4ef6a90fcf6925709742457ef918fbaf80a2f0b89",
    ),
];

const TRIAGE: &Roster = &[
    (
        "implement:design",
        "claude-opus-5",
        "720152f35e0190566eb39716ba554fa3ceec705be29b41479cfcb9e464114949",
    ),
    (
        "review:design:positions:spec-compliance",
        "claude-opus-5",
        "bcfc9eedf910ddae08807b3720558d665a03ca9ddb2211dbfddc5839da946782",
    ),
    (
        "review:design:positions:security",
        "claude-fable-5-1",
        "33d6b92f2a349636e60cb9a4ef6a90fcf6925709742457ef918fbaf80a2f0b89",
    ),
    (
        "design:chief",
        "claude-fable-5-1",
        "c6d224031f2e18010fc5e104cf4692fe7c51713e9c43da9f89f748331f4a69da",
    ),
    (
        "specify:author",
        "claude-fable-5-1",
        "c6d224031f2e18010fc5e104cf4692fe7c51713e9c43da9f89f748331f4a69da",
    ),
    (
        "tasks:author",
        "claude-opus-5",
        "720152f35e0190566eb39716ba554fa3ceec705be29b41479cfcb9e464114949",
    ),
    (
        "clarify:judge",
        "claude-opus-5",
        "d6a29b7aed9bec3460469ad812bf0cfe7f53381b17f7b1c537f04beb91c6515b",
    ),
    (
        "analyze:judge",
        "claude-fable-5-1",
        "cb0bb2ee61d718481d3032025b114e8ea63298026fb4bfad18661c37648b30a2",
    ),
    (
        "design:positions:simplicity",
        "claude-opus-5",
        "81a14dffa301d38f2d523d0a828fd7ce5accf38a0e75dc3995f9682c9cf72b0d",
    ),
    (
        "design:positions:robustness",
        "gpt-5.6-sol",
        "696802b3d981e7d487926500a749c699e1697b05a0d7e5686639f8dd09067533",
    ),
];
const SELF: &Roster = &[
    (
        "intake",
        "claude-sonnet-5",
        "fbdb7dba8e34fbc0b02e0f7fd7540fd0ab9313e40cdbcb03c27c22d78c138756",
    ),
    (
        "implement",
        "claude-opus-5",
        "b750b0a401fa7fc1aad5dd929bf136cf961b12d2e11ac9fc67995927ea686ad7",
    ),
    (
        "review",
        "claude-fable-5-1",
        "4efedc43f0b8ac110000f4ffa3b3205aac3acac0850485b027d298dd2b8aa4e8",
    ),
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
    for (site, model, charter_digest) in roster {
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
