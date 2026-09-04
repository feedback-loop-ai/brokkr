//! Decision 0041 adoption pins: library-backed sites resolve the roster's
//! current first hire and the charter recorded in the compiled manifest.
//! The fable pins deliberately moved to `claude-fable-5-1` under ruling 1;
//! ruling 2 moved tools and model choices into one office definition.

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
        "094297953525b949d5a5f26c16e97a73602320150bd40c3951838c93d8d7e35a",
    ),
    (
        "verify",
        "claude-fable-5-1",
        "4cf73af1d979b54cb4026c301bbc7ffa86a4cf7149d037f8d14d05ac714076d0",
    ),
    (
        "ship",
        "claude-fable-5-1",
        "df94781f03b42a9b2186c914c92e4fef85aa8db65a664afaeadecd9d9211b1b9",
    ),
    (
        "review:correctness",
        "gpt-5.6-sol",
        "ce423d91104cd3e298c49b22a7ebf96182fd2cbde71bd4abc0f147f568aa3001",
    ),
    (
        "review:security",
        "claude-fable-5-1",
        "555a59377d31565a87489664571e23015a958839bea50a226471a99e8b11b869",
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
        "8b4b3b12f0df64a695a412b42288a2c112bbed61e55ae7bdc77c588f080a0564",
    ),
    (
        "verify",
        "claude-fable-5-1",
        "4cf73af1d979b54cb4026c301bbc7ffa86a4cf7149d037f8d14d05ac714076d0",
    ),
    (
        "ship",
        "claude-fable-5-1",
        "df94781f03b42a9b2186c914c92e4fef85aa8db65a664afaeadecd9d9211b1b9",
    ),
    (
        "review:spec-compliance",
        "claude-opus-5",
        "416f9e17378ab421318a9deee9ba156ab7b8b2e793b6c56fd77253354fe78f75",
    ),
    (
        "review:security",
        "claude-fable-5-1",
        "555a59377d31565a87489664571e23015a958839bea50a226471a99e8b11b869",
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
        "094297953525b949d5a5f26c16e97a73602320150bd40c3951838c93d8d7e35a",
    ),
    (
        "verify",
        "claude-fable-5-1",
        "4cf73af1d979b54cb4026c301bbc7ffa86a4cf7149d037f8d14d05ac714076d0",
    ),
    (
        "review",
        "claude-fable-5-1",
        "6015367df641c90cf74131b37cda475c12899a0cece1d90ad167a47860e12df8",
    ),
    (
        "ship",
        "claude-fable-5-1",
        "df94781f03b42a9b2186c914c92e4fef85aa8db65a664afaeadecd9d9211b1b9",
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
        "verify" | "ship" | "review" | "review:security" => "xhigh",
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
            "verify" => Some("Bash(cargo:*)"),
            "intake" | "ship" => Some("Bash(git:*)"),
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

/// `recipes/sdd`'s `speckit-check` step stays INLINE. It is
/// `driver exec -- bash …` — a shell script with no model and no
/// charter-as-prompt semantics — and it is the case that proves the
/// library is an option and not a mandate.
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

/// The 0007 declaration an adopting seat used to write inline is the one
/// the 0007 DEFAULT already computes, so dropping it changed nothing
/// about which facts the seat may supply.
#[test]
fn adoption_did_not_change_any_seats_declared_inputs() {
    for relative in ["bundles/self", "recipes/panel-review", "recipes/sdd"] {
        let bundle = compile(relative);
        assert_eq!(
            bundle.seats["review"].inputs,
            vec![
                "fixes_applied".to_string(),
                "has_security_residual".to_string(),
                "max_residual_severity".to_string(),
            ],
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
