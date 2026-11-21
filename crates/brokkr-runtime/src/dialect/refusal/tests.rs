//! A dialect's checks, one by one: each refusal is its own variant, read
//! exactly, each check function's order holds, and what each admits is
//! what it always admitted (#353).

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{json, Value};

use super::super::{check_artifacts, check_command_tokens, check_version, Dialect, SCHEMAS};
use super::{refused, DialectError, DialectRefusal};

/// The shipped OpenSpec dialect as bytes given a shape, before any check.
fn shipped() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dialects/openspec.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// The shipped dialect with each `(pointer, plant)` written in, unchecked.
fn planted(plants: &[(&str, Value)]) -> Dialect {
    let mut value = shipped();
    for (pointer, plant) in plants {
        *value.pointer_mut(pointer).unwrap() = plant.clone();
    }
    serde_json::from_value(value).unwrap()
}

/// The positions the shipped map assigns, which the order check reads.
fn positions() -> BTreeMap<String, usize> {
    let names = ["proposal", "specs", "design", "tasks"];
    names
        .into_iter()
        .enumerate()
        .map(|(at, name)| (name.to_string(), at))
        .collect()
}

#[test]
fn the_version_is_refused_by_its_name_and_an_absent_or_read_one_passes() {
    let v9 = json!({"schema": "brokkr.dialect/v9"});
    assert_eq!(
        check_version(&v9, &SCHEMAS, "a note"),
        Err(DialectRefusal::Version {
            schema: "brokkr.dialect/v9",
            accepted: &SCHEMAS,
            note: "a note",
        })
    );
    let v3 = json!({"schema": "brokkr.dialect/v3"});
    assert_eq!(check_version(&v3, &SCHEMAS, "a note"), Ok(()));
    assert_eq!(check_version(&json!({}), &SCHEMAS, "a note"), Ok(()));
}

#[test]
fn each_version_shape_refuses_in_its_own_variant_and_older_pins_pass() {
    let v1 = json!("brokkr.dialect/v1");
    let v2 = json!("brokkr.dialect/v2");
    // An install and an instructed archive: the install is judged first.
    let early = planted(&[("/schema", v1.clone())]);
    assert_eq!(early.check_versions(), Err(DialectRefusal::InstallTooEarly));
    let uninstalled = planted(&[("/tool", json!({"binary": "openspec", "version": "1"}))]);
    assert_eq!(
        uninstalled.check_versions(),
        Err(DialectRefusal::Uninstalled {
            schema: "brokkr.dialect/v3"
        })
    );
    let instructed = planted(&[("/schema", v2.clone())]);
    assert_eq!(
        instructed.check_versions(),
        Err(DialectRefusal::InstructionTooEarly {
            schema: "brokkr.dialect/v2"
        })
    );
    let uninstructed = planted(&[("/archive/instructions", Value::Null)]);
    assert_eq!(
        uninstructed.check_versions(),
        Err(DialectRefusal::Uninstructed)
    );
    let pinned_v1 = planted(&[
        ("/schema", v1),
        ("/tool", json!({"binary": "openspec", "version": "1"})),
        ("/archive/instructions", Value::Null),
    ]);
    assert_eq!(pinned_v1.first_refusal(), Ok(()));
    let pinned_v2 = planted(&[("/schema", v2), ("/archive/instructions", Value::Null)]);
    assert_eq!(pinned_v2.first_refusal(), Ok(()));
    assert_eq!(planted(&[]).first_refusal(), Ok(()));
}

#[test]
fn each_identity_field_refuses_in_its_own_variant_in_order() {
    let blank = || json!(" ");
    for (pointer, empty) in [
        ("/name", DialectRefusal::EmptyName),
        ("/tool/binary", DialectRefusal::EmptyBinary),
        ("/tool/version", DialectRefusal::EmptyVersion),
        ("/tool/install/package", DialectRefusal::EmptyPackage),
    ] {
        assert_eq!(planted(&[(pointer, blank())]).check_identity(), Err(empty));
    }
    let sourced = |source: Value| {
        let install = json!({"manager": "npm", "package": "openspec", "source": source});
        planted(&[("/tool/install", install)])
    };
    assert_eq!(
        sourced(blank()).check_identity(),
        Err(DialectRefusal::EmptySource)
    );
    assert_eq!(sourced(json!("git+https://x")).check_identity(), Ok(()));
    let binary = || json!("/bin/echo");
    assert_eq!(
        planted(&[("/tool/binary", binary())]).check_identity(),
        Err(DialectRefusal::NotBare {
            binary: "/bin/echo"
        })
    );
    let both = planted(&[("/name", json!("")), ("/tool/binary", json!(""))]);
    assert_eq!(both.check_identity(), Err(DialectRefusal::EmptyName));
    let versionless = planted(&[("/tool/version", json!("")), ("/tool/binary", binary())]);
    assert_eq!(
        versionless.check_identity(),
        Err(DialectRefusal::EmptyVersion)
    );
    // The shipped install names no source.
    assert_eq!(planted(&[]).check_identity(), Ok(()));
}

#[test]
fn each_artifact_check_refuses_in_its_own_variant_in_order() {
    let (name, artifacts) = (
        "/phases/specify/steps/0/name",
        "/phases/specify/steps/0/artifacts",
    );
    let (unfilled, optional) = ("/phases/design/steps", "/phases/tasks/steps/0/optional");
    for (plants, first) in [
        (
            vec![(unfilled, json!([]))],
            DialectRefusal::Unfilled { phase: "design" },
        ),
        (
            vec![(optional, json!(true))],
            DialectRefusal::NoRequiredStep { phase: "tasks" },
        ),
        (
            vec![(name, json!(" "))],
            DialectRefusal::UnnamedStep { phase: "specify" },
        ),
        (
            vec![(artifacts, json!([]))],
            DialectRefusal::NoArtifacts { phase: "specify" },
        ),
        (
            vec![(artifacts, json!(["proposal", "proposal"]))],
            DialectRefusal::AssignedTwice {
                artifact: "proposal",
            },
        ),
        (
            vec![(name, json!("")), (artifacts, json!([]))],
            DialectRefusal::UnnamedStep { phase: "specify" },
        ),
        (
            vec![(unfilled, json!([])), (optional, json!(true))],
            DialectRefusal::Unfilled { phase: "design" },
        ),
    ] {
        let dialect = planted(&plants);
        assert_eq!(check_artifacts(&dialect.phases), Err(first));
    }
    assert_eq!(check_artifacts(&planted(&[]).phases), Ok(positions()));
}

#[test]
fn each_edge_check_refuses_in_its_own_variant_in_order() {
    let edge = |before: &str, after: &str| {
        planted(&[("/order/0", json!({"before": before, "after": after}))])
    };
    for ((before, after), first) in [
        (
            ("ghost", "specs"),
            DialectRefusal::UnassignedBefore { artifact: "ghost" },
        ),
        (
            ("proposal", "phantom"),
            DialectRefusal::UnassignedAfter {
                artifact: "phantom",
            },
        ),
        (
            ("specs", "proposal"),
            DialectRefusal::Reversed {
                after: "proposal",
                before: "specs",
            },
        ),
        (
            ("ghost", "phantom"),
            DialectRefusal::UnassignedBefore { artifact: "ghost" },
        ),
    ] {
        let dialect = edge(before, after);
        assert_eq!(dialect.check_order(&positions()), Err(first));
    }
    assert_eq!(edge("proposal", "specs").check_order(&positions()), Ok(()));
}

#[test]
fn a_taxonomy_an_instruction_and_a_token_refuse_in_their_own_variants() {
    let taxonomy = planted(&[("/phases/clarify/taxonomy", json!(" "))]);
    assert_eq!(
        taxonomy.check_taxonomies(),
        Err(DialectRefusal::NoTaxonomy { phase: "clarify" })
    );
    let instruction = "/phases/specify/steps/0/instructions";
    for escape in [
        "/absolute.md",
        "\\absolute.md",
        "C:/absolute.md",
        "safe/../outside.md",
    ] {
        let dialect = planted(&[(instruction, json!(escape))]);
        assert_eq!(
            dialect.check_instruction_paths(),
            Err(DialectRefusal::Escapes {
                instruction: escape
            })
        );
    }
    let dotted = planted(&[(instruction, json!("notes..md"))]);
    assert_eq!(dotted.check_instruction_paths(), Ok(()));
    let token = planted(&[("/verify/argv/0", json!("{unknown}"))]);
    assert_eq!(
        check_command_tokens(&token),
        Err(DialectRefusal::UnknownPlaceholder { token: "{unknown}" })
    );
    let changed = planted(&[
        ("/verify/argv/0", json!("{change}")),
        ("/archive/argv/1", json!("changes/{change}.md")),
    ]);
    assert_eq!(check_command_tokens(&changed), Ok(()));
}

/// All 22 variants beside the text main prints for each, for fixed names.
fn texts() -> [(DialectRefusal<'static>, &'static str); 22] {
    let identity = "name, tool binary, measured version and install package must be non-empty";
    let (phase, artifact) = ("design", "specs");
    [
        (
            DialectRefusal::Version {
                schema: "brokkr.dialect/v9",
                accepted: &SCHEMAS,
                note: "a pinned dialect may be any of them",
            },
            "it calls itself 'brokkr.dialect/v9'; this reads brokkr.dialect/v1, \
             brokkr.dialect/v2, brokkr.dialect/v3 (a pinned dialect may be any of them)",
        ),
        (
            DialectRefusal::InstallTooEarly,
            "a 'brokkr.dialect/v1' dialect carries no tool install; the install identity \
             landed in brokkr.dialect/v2",
        ),
        (
            DialectRefusal::Uninstalled {
                schema: "brokkr.dialect/v2",
            },
            "a 'brokkr.dialect/v2' dialect names what installs its tool",
        ),
        (
            DialectRefusal::InstructionTooEarly {
                schema: "brokkr.dialect/v1",
            },
            "a 'brokkr.dialect/v1' archive carries no instruction; the archive instruction \
             landed in brokkr.dialect/v3",
        ),
        (
            DialectRefusal::Uninstructed,
            "a 'brokkr.dialect/v3' archive command names the instruction it carries",
        ),
        (DialectRefusal::EmptyName, identity),
        (DialectRefusal::EmptyBinary, identity),
        (DialectRefusal::EmptyVersion, identity),
        (DialectRefusal::EmptyPackage, identity),
        (DialectRefusal::EmptySource, identity),
        (
            DialectRefusal::NotBare {
                binary: "/bin/echo",
            },
            "tool binary '/bin/echo' must be a bare filename resolved from PATH",
        ),
        (
            DialectRefusal::Unfilled { phase },
            "phase 'design' is unfilled",
        ),
        (
            DialectRefusal::NoRequiredStep { phase },
            "phase 'design' has no required step",
        ),
        (
            DialectRefusal::UnnamedStep { phase },
            "phase 'design' has an empty step or artifact list",
        ),
        (
            DialectRefusal::NoArtifacts { phase },
            "phase 'design' has an empty step or artifact list",
        ),
        (
            DialectRefusal::AssignedTwice { artifact },
            "artifact 'specs' is assigned more than once",
        ),
        (
            DialectRefusal::UnassignedBefore { artifact },
            "order names unassigned artifact 'specs'",
        ),
        (
            DialectRefusal::UnassignedAfter { artifact },
            "order names unassigned artifact 'specs'",
        ),
        (
            DialectRefusal::Reversed {
                after: "proposal",
                before: artifact,
            },
            "artifact 'proposal' is mapped before its dependency 'specs'",
        ),
        (
            DialectRefusal::NoTaxonomy { phase: "clarify" },
            "phase 'clarify' has no taxonomy",
        ),
        (
            DialectRefusal::Escapes {
                instruction: "../outside.md",
            },
            "instruction path '../outside.md' must be relative and remain beside its dialect",
        ),
        (
            DialectRefusal::UnknownPlaceholder { token: "{unknown}" },
            "argv token '{unknown}' uses an unknown placeholder",
        ),
    ]
}

/// Each variant's text, written out apart from its `#[error]`: the words
/// main has always printed, and one refusal as the operator reads it.
#[test]
fn every_dialect_refusal_reads_as_it_always_has() {
    for (refusal, text) in texts() {
        assert_eq!(refusal.to_string(), text);
    }
    let phase = "design";
    match refused("dialect.json", DialectRefusal::Unfilled { phase }) {
        DialectError::Invalid { path, problem } => {
            assert_eq!(path, "dialect.json");
            assert_eq!(problem, "phase 'design' is unfilled");
        }
        other => panic!("expected an Invalid refusal, got {other:?}"),
    }
}
