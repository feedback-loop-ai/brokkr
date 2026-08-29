//! T21/T22, AC-5: the adopting recipes resolve to exactly what they used
//! to inline, plus the model they never used to name.
//!
//! Every seat, panel member and sequence step below has its PRE-ADOPTION
//! argv recorded here as it stood in the tree, and the test asserts the
//! resolved argv is that argv with one `--model <concrete id>` pair
//! inserted — nothing added, nothing dropped, `--allowedTools` ordering
//! element for element. The charter is asserted by digest against the
//! role file it was moved from.
//!
//! That the argv changes at all is the point of the feature and is
//! stated as such in the spec: an adopting seat used to pass no `--model`
//! and take whatever the provider CLI defaulted to that day — an
//! unpinned, invisible, undated choice. A pinned id is strictly more
//! honest. Non-adopting recipes' argv does not change at all, which
//! `witness_digests.rs` pins.

use std::collections::BTreeMap;
use std::path::PathBuf;

use forge_runtime::{Bundle, PanelMember, SeatBody, StepBody};

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

const BASE_TOOLS: &str = "Bash(cargo:*),Bash(git:*),Bash(python3:*),\
                          Bash(.venv/bin/pytest:*),Bash(ls:*),Bash(rg:*),Bash(mkdir:*)";

/// The argv every adopting seat inlined before this slice: the claude
/// driver, `acceptEdits`, and one `--allowedTools` list.
fn historic(specify: bool) -> Vec<String> {
    let tools = match specify {
        true => format!("{BASE_TOOLS},Bash(specify:*)"),
        false => BASE_TOOLS.to_string(),
    };
    [
        "driver",
        "claude",
        "--",
        "--permission-mode",
        "acceptEdits",
        "--allowedTools",
        &tools,
    ]
    .iter()
    .map(|part| part.to_string())
    .collect()
}

/// The same argv with the model pinned: `--model <id>` inserted where
/// the adapter composes it, immediately before the tool permissions.
fn expected(specify: bool, model: &str) -> Vec<String> {
    let mut argv = historic(specify);
    let at = argv.len() - 2;
    argv.splice(at..at, ["--model".to_string(), model.to_string()]);
    argv
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

/// site → (specify tools?, concrete model id, the digest of the role file
/// the charter was moved from).
type Roster = [(&'static str, bool, &'static str, &'static str)];

const PANEL_REVIEW: &Roster = &[
    (
        "intake",
        false,
        "claude-sonnet-5",
        "d27fd1983362c158af6b878942b6166482632cff43d23ebff72b55532c31aa9c",
    ),
    (
        "implement",
        false,
        "claude-opus-5",
        "3c0e869efcd3c46c853c13c44fc9c1ff0d0a50df45194ab9e0a3019756443f77",
    ),
    (
        "verify",
        false,
        "claude-sonnet-5",
        "b2c93f743e0811e40cc825c28ea74885b292c5df5371f7604fced3715bc54ded",
    ),
    (
        "ship",
        false,
        "claude-sonnet-5",
        "df94781f03b42a9b2186c914c92e4fef85aa8db65a664afaeadecd9d9211b1b9",
    ),
    (
        "review:correctness",
        false,
        "claude-opus-5",
        "ce423d91104cd3e298c49b22a7ebf96182fd2cbde71bd4abc0f147f568aa3001",
    ),
    (
        "review:security",
        false,
        "claude-opus-5",
        "555a59377d31565a87489664571e23015a958839bea50a226471a99e8b11b869",
    ),
];

const SDD: &Roster = &[
    (
        "intake",
        true,
        "claude-sonnet-5",
        "af6146544f8626e7c21d126088c465e3bf08c5b3c0844da650f692b022ef229c",
    ),
    (
        "implement",
        true,
        "claude-opus-5",
        "3720b487fea0e433e23977c528e00cccc924fa667f1e22fa03176f2f3fb4bccc",
    ),
    (
        "verify",
        true,
        "claude-sonnet-5",
        "b2c93f743e0811e40cc825c28ea74885b292c5df5371f7604fced3715bc54ded",
    ),
    (
        "ship",
        true,
        "claude-sonnet-5",
        "df94781f03b42a9b2186c914c92e4fef85aa8db65a664afaeadecd9d9211b1b9",
    ),
    (
        "review:spec-compliance",
        true,
        "claude-opus-5",
        "416f9e17378ab421318a9deee9ba156ab7b8b2e793b6c56fd77253354fe78f75",
    ),
    (
        "review:security",
        true,
        "claude-opus-5",
        "de00a51d25e0a4fc77b12bdb6c0793ec5e6601ebeb05459c6f44e082705e176a",
    ),
    (
        "design:chief",
        true,
        "claude-fable-5",
        "757657c88e0f0b6f48763b836e1e2648e794d5408452dc030138401a5820d60d",
    ),
    (
        "design:positions:simplicity",
        true,
        "claude-opus-5",
        "d00dfc71d5fbcfd619f72b554747dbc1b2cd318c4b1fee4678dbd6a710a9cddf",
    ),
    (
        "design:positions:robustness",
        true,
        "claude-opus-5",
        "f96e146711c0567ef7c93511a13d5bfbc1414ef7335f3df447ccbc6d83b79927",
    ),
];
const SELF: &Roster = &[
    (
        "intake",
        false,
        "claude-sonnet-5",
        "d27fd1983362c158af6b878942b6166482632cff43d23ebff72b55532c31aa9c",
    ),
    (
        "implement",
        false,
        "claude-opus-5",
        "3c0e869efcd3c46c853c13c44fc9c1ff0d0a50df45194ab9e0a3019756443f77",
    ),
    (
        "verify",
        false,
        "claude-sonnet-5",
        "b2c93f743e0811e40cc825c28ea74885b292c5df5371f7604fced3715bc54ded",
    ),
    (
        "review",
        false,
        "claude-opus-5",
        "6015367df641c90cf74131b37cda475c12899a0cece1d90ad167a47860e12df8",
    ),
    (
        "ship",
        false,
        "claude-sonnet-5",
        "df94781f03b42a9b2186c914c92e4fef85aa8db65a664afaeadecd9d9211b1b9",
    ),
];

fn assert_adopted(relative: &str, roster: &Roster) {
    let bundle = compile(relative);
    let sites = sites(&bundle);
    for (site, specify, model, charter_digest) in roster {
        let (charter, argv) = sites
            .get(*site)
            .unwrap_or_else(|| panic!("{relative} has no site '{site}'"));
        assert_eq!(
            &argv[1..],
            expected(*specify, model).as_slice(),
            "{relative} site '{site}' argv is not the inline argv plus its model"
        );
        let bytes = std::fs::read(charter).unwrap();
        assert_eq!(
            forge_core::canonical::sha256_bytes(&bytes),
            *charter_digest,
            "{relative} site '{site}' charter is not the text it replaced"
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
