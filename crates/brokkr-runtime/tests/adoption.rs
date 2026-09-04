//! Decision 0041 adoption pins: library-backed sites resolve the roster's
//! current first hire and the charter recorded in the compiled manifest.
//! The fable pins deliberately moved to `claude-fable-5-1` under ruling 1;
//! ruling 2 moved tools and model choices into one office definition.
//! Rulings 4 and 5 move the charter witnesses again: judges no longer fix,
//! implementers answer returned findings, and spec compliance can return a
//! defective specification to design.

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
    for (name, seat) in &bundle.seats {
        match &seat.body {
            SeatBody::Single {
                role_path, command, ..
            } => {
                out.insert(name.clone(), (role_path.clone(), command.clone()));
            }
            SeatBody::Panel { members, .. } => member(&mut out, &format!("{name}:"), members),
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
                            member(&mut out, &format!("{name}:{}:", step.name), members)
                        }
                    }
                }
            }
        }
    }
    out
}

/// site → (concrete model id, current charter digest).
type Roster = [(&'static str, &'static str, &'static str)];

const PANEL_REVIEW: &Roster = &[
    (
        "intake",
        "claude-sonnet-5",
        "2ee39f00481d3650d945174fc0aabe11ccd82057352116c5c112ef224b1b4168",
    ),
    (
        "implement",
        "claude-opus-5",
        "f032a871a3bcb4cd2cbd0836098189eca2dbb0c11599a0b237a95169d8a24055",
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

const SDD: &Roster = &[
    (
        "intake",
        "claude-sonnet-5",
        "2fb2a1685da166fc0c4dc519a711913f81fff451a441f6f3572abac73ddf23d1",
    ),
    (
        "implement",
        "claude-opus-5",
        "5853f85c7e8ee053b8085af1610c59dd455d15216b70490e80bae533835039a0",
    ),
    (
        "review:spec-compliance",
        "claude-opus-5",
        "8eef2c37b4cd882ca4af4138372506c6bb15f58a46f9c57a7aea3afd127a9c40",
    ),
    (
        "review:security",
        "claude-fable-5-1",
        "33d6b92f2a349636e60cb9a4ef6a90fcf6925709742457ef918fbaf80a2f0b89",
    ),
    (
        "design:chief",
        "claude-fable-5-1",
        "757657c88e0f0b6f48763b836e1e2648e794d5408452dc030138401a5820d60d",
    ),
    (
        "design:positions:simplicity",
        "claude-opus-5",
        "d00dfc71d5fbcfd619f72b554747dbc1b2cd318c4b1fee4678dbd6a710a9cddf",
    ),
    (
        "design:positions:robustness",
        "gpt-5.6-sol",
        "f96e146711c0567ef7c93511a13d5bfbc1414ef7335f3df447ccbc6d83b79927",
    ),
];
const SELF: &Roster = &[
    (
        "intake",
        "claude-sonnet-5",
        "2ee39f00481d3650d945174fc0aabe11ccd82057352116c5c112ef224b1b4168",
    ),
    (
        "implement",
        "claude-opus-5",
        "f032a871a3bcb4cd2cbd0836098189eca2dbb0c11599a0b237a95169d8a24055",
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
        "design:chief" => "max",
        "review" | "review:security" => "xhigh",
        _ => "high",
    };
    argv.extend(["--model", model, "--effort", effort]);
    if site == "review" || site.starts_with("review:") {
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
            "intake" => Some("Bash(git:*)"),
            "design:chief" => Some("Bash(git:*),Bash(specify:*)"),
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
fn sdd_resolves_to_what_it_used_to_inline() {
    assert_adopted("recipes/sdd", SDD);
}

#[test]
fn self_resolves_to_what_it_used_to_inline() {
    assert_adopted("bundles/self", SELF);
}

/// `recipes/sdd`'s boxed `speckit-check` gate stays INLINE. It is
/// `driver exec -- bash …` — a deterministic shell script with no model
/// and no charter-as-prompt semantics — and it is the case that proves
/// the library is an option and not a mandate.
#[test]
fn the_speckit_check_step_stays_inline() {
    let bundle = compile("recipes/sdd");
    let SeatBody::Sequence { steps } = &bundle.seats["design"].body else {
        panic!("design is a sequence")
    };
    let step = steps
        .iter()
        .find(|step| step.name == "speckit-check")
        .expect("the speckit-check step");
    let StepBody::Single {
        command,
        candidates,
        ..
    } = &step.body
    else {
        panic!("speckit-check is a single step")
    };
    assert!(candidates.is_empty(), "an inline step has no chain");
    assert_eq!(command[1], "driver");
    assert_eq!(command[2], "exec");
    assert!(bundle.manifest["agents"]
        .get("design:speckit-check")
        .is_none());
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
            "recipes/sdd",
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
    for relative in ["bundles/self", "recipes/panel-review", "recipes/sdd"] {
        let bundle = compile(relative);
        for (phase, (attempts, seconds)) in &expected {
            let limits = bundle.seats[*phase].limits;
            assert_eq!(limits.max_attempts, *attempts, "{relative} {phase}");
            assert_eq!(limits.timeout_seconds, *seconds, "{relative} {phase}");
        }
    }
}
