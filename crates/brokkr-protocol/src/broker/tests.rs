//! U7c0: every closed broker record writes exactly the wire names its
//! closed `Deserialize` reads. Each expected form below is written out by
//! hand, never produced by the encoder under test, so an encoder and a
//! decoder that share one misspelling still fail here.

use serde_json::{json, Value};

use super::*;

/// One plan whose box holds `tree`, every field a distinct value.
fn plan(tree: Tree) -> Plan {
    Plan {
        owner: Owner {
            repo: "r".repeat(64),
            run: "run-1".into(),
            effect: "effect-1".into(),
            attempt: "attempt-1".into(),
            site: "research".into(),
            instance: "research#0".into(),
        },
        server: "cap-library-docs".into(),
        capability: "library-docs".into(),
        dialect: Dialect {
            name: "docs-mcp".into(),
            digest: "d".repeat(64),
            definition: "f".repeat(64),
            version: "1.0.0".into(),
        },
        connection: Connection {
            argv: vec!["/opt/docs/bin/server".into(), "--stdio".into()],
        },
        tools: vec!["lookup".into()],
        restrictions: Restrictions {},
        retained: true,
        secrets: vec!["DOCS_TOKEN".into()],
        clearance: Clearance {
            dialect: "d".repeat(64),
            policy: "p".repeat(64),
        },
        intent: BoxIntent {
            reach: Reach {
                writable: vec!["/work/repo".into()],
                readable: vec!["/work/ro".into()],
            },
            executable: "/opt/docs/bin/server".into(),
            tree,
            sources: Sources {
                entries: 7,
                mounts: 3,
                digest: "s".repeat(64),
            },
            writers: Writers {
                uids: vec![1000],
                privilege: Privilege::Confined,
            },
            network: Network::Isolated,
            environment: vec!["HOME".into()],
            bootstrap: Bootstrap {
                path: "/usr/bin/brokkr".into(),
                digest: "b".repeat(64),
            },
            excluded: Excluded {
                store: "/home/op/.brokkr/secrets".into(),
                control: vec!["/run/brokkr/attempt".into()],
            },
        },
    }
}

/// [`plan`]'s wire form with its box's program tree written as `tree`.
fn written(tree: Value) -> Value {
    json!({
        "owner": {"repo": "r".repeat(64), "run": "run-1", "effect": "effect-1",
                  "attempt": "attempt-1", "site": "research", "instance": "research#0"},
        "server": "cap-library-docs",
        "capability": "library-docs",
        "dialect": {"name": "docs-mcp", "digest": "d".repeat(64),
                    "definition": "f".repeat(64), "version": "1.0.0"},
        "connection": {"argv": ["/opt/docs/bin/server", "--stdio"]},
        "tools": ["lookup"],
        "restrictions": {},
        "retained": true,
        "secrets": ["DOCS_TOKEN"],
        "clearance": {"dialect": "d".repeat(64), "policy": "p".repeat(64)},
        "box": {
            "reach": {"writable": ["/work/repo"], "readable": ["/work/ro"]},
            "executable": "/opt/docs/bin/server",
            "tree": tree,
            "sources": {"entries": 7, "mounts": 3, "digest": "s".repeat(64)},
            "writers": {"uids": [1000], "privilege": "confined"},
            "network": "isolated",
            "environment": ["HOME"],
            "bootstrap": {"path": "/usr/bin/brokkr", "digest": "b".repeat(64)},
            "excluded": {"store": "/home/op/.brokkr/secrets",
                         "control": ["/run/brokkr/attempt"]}
        }
    })
}

/// A plan in either program tree, and the inventory pinning two plans,
/// write exactly their pinned wire form and read back equal.
#[test]
fn every_closed_record_writes_the_wire_names_its_reader_reads_and_reads_back_equal() {
    let package = || Tree::Package {
        root: "/opt/docs".into(),
    };
    let trees = [
        (Tree::System {}, json!({"kind": "system"})),
        (package(), json!({"kind": "package", "root": "/opt/docs"})),
    ];
    for (tree, wire) in trees {
        let expected = written(wire);
        let plan = plan(tree);
        assert_eq!(serde_json::to_value(&plan).unwrap(), expected);
        assert_eq!(serde_json::from_value::<Plan>(expected).unwrap(), plan);
    }
    let inventory = Inventory {
        plans: vec![
            Pin {
                locator: "/run/brokkr/attempt/plans/0.json".into(),
                digest: "0".repeat(64),
            },
            Pin {
                locator: "/run/brokkr/attempt/plans/1.json".into(),
                digest: "1".repeat(64),
            },
        ],
    };
    let expected = json!({"plans": [
        {"locator": "/run/brokkr/attempt/plans/0.json", "digest": "0".repeat(64)},
        {"locator": "/run/brokkr/attempt/plans/1.json", "digest": "1".repeat(64)}
    ]});
    assert_eq!(serde_json::to_value(&inventory).unwrap(), expected);
    assert_eq!(
        serde_json::from_value::<Inventory>(expected).unwrap(),
        inventory
    );
    assert_eq!(
        serde_json::to_string(&Inventory { plans: Vec::new() }).unwrap(),
        r#"{"plans":[]}"#
    );
}

/// A written plan that loses any one field, or names its box by the
/// field's Rust name rather than its wire name, refuses as it is read.
#[test]
fn a_written_plan_missing_a_field_or_misnaming_one_refuses() {
    let written = serde_json::to_value(plan(Tree::System {})).unwrap();
    let keys: Vec<String> = written.as_object().unwrap().keys().cloned().collect();
    let refused: Vec<String> = keys
        .iter()
        .map(|key| {
            let mut lost = written.clone();
            lost.as_object_mut().unwrap().remove(key);
            serde_json::from_value::<Plan>(lost)
                .unwrap_err()
                .to_string()
        })
        .collect();
    let missing: Vec<String> = keys
        .iter()
        .map(|key| format!("missing field `{key}`"))
        .collect();
    assert_eq!(refused, missing);
    let mut renamed = written;
    let intent = renamed.as_object_mut().unwrap().remove("box").unwrap();
    renamed["intent"] = intent;
    assert_eq!(
        serde_json::from_value::<Plan>(renamed)
            .unwrap_err()
            .to_string(),
        "unknown field `intent`, expected one of `owner`, `server`, `capability`, `dialect`, \
         `connection`, `tools`, `restrictions`, `retained`, `secrets`, `clearance`, `box`"
    );
}
