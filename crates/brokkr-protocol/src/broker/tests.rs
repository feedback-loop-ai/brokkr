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

/// One survey request, every field a distinct value, and its wire form.
fn survey() -> (Survey, Value) {
    let survey = Survey {
        launch: "docs-mcp".into(),
        home: "/home/op".into(),
        reach: Reach {
            writable: vec!["/work/repo".into()],
            readable: vec!["/work/ro".into()],
        },
        network: Network::Shared,
        writers: vec![1000, 1001],
    };
    let wire = json!({
        "launch": "docs-mcp",
        "home": "/home/op",
        "reach": {"writable": ["/work/repo"], "readable": ["/work/ro"]},
        "network": "shared",
        "writers": [1000, 1001]
    });
    (survey, wire)
}

/// U7c0b: the survey request and its reply, either way, write exactly
/// their pinned wire form and read back equal.
#[test]
fn the_survey_request_and_reply_write_the_wire_names_their_readers_read() {
    let (request, wire) = survey();
    assert_eq!(serde_json::to_value(&request).unwrap(), wire);
    assert_eq!(serde_json::from_value::<Survey>(wire).unwrap(), request);
    let provisioned = Surveyed::Provisioned(Provisioned {
        executable: "/opt/docs/bin/docs-mcp".into(),
        tree: Tree::Package {
            root: "/opt/docs".into(),
        },
        bootstrap: Bootstrap {
            path: "/usr/bin/brokkr".into(),
            digest: "b".repeat(64),
        },
        sources: Sources {
            entries: 7,
            mounts: 3,
            digest: "s".repeat(64),
        },
    });
    let replies = [
        (
            provisioned,
            json!({"provisioned": {
                "executable": "/opt/docs/bin/docs-mcp",
                "tree": {"kind": "package", "root": "/opt/docs"},
                "bootstrap": {"path": "/usr/bin/brokkr", "digest": "b".repeat(64)},
                "sources": {"entries": 7, "mounts": 3, "digest": "s".repeat(64)}
            }}),
        ),
        (
            Surveyed::Refused(Refusal::Identity),
            json!({"refused": "identity"}),
        ),
        (
            Surveyed::Refused(Refusal::ProgramTree),
            json!({"refused": "program-tree"}),
        ),
        (
            Surveyed::Unread(Unread::Oversized),
            json!({"unread": "oversized"}),
        ),
        (
            Surveyed::Unread(Unread::Truncated),
            json!({"unread": "truncated"}),
        ),
        (Surveyed::Unread(Unread::Input), json!({"unread": "input"})),
        (
            Surveyed::Unread(Unread::Malformed),
            json!({"unread": "malformed"}),
        ),
    ];
    for (reply, wire) in replies {
        assert_eq!(serde_json::to_value(&reply).unwrap(), wire);
        assert_eq!(serde_json::from_value::<Surveyed>(wire).unwrap(), reply);
    }
}

/// U7c0b: why the survey read no request, in its one operator text.
#[test]
fn an_unread_survey_request_says_why_in_its_own_words() {
    let said = [
        Unread::Oversized,
        Unread::Truncated,
        Unread::Input,
        Unread::Malformed,
    ]
    .map(|cause| cause.to_string());
    assert_eq!(
        said,
        [
            "MCP server survey request exceeds its bound",
            "MCP server survey request is truncated",
            "MCP server survey request could not be read",
            "MCP server survey request is not one closed record",
        ]
    );
}

/// A request missing, repeating or adding a field, or written by position,
/// and a reply naming another variant, or one twice, refuses as it is read.
#[test]
fn a_survey_request_or_reply_parses_closed_and_defaults_nothing() {
    let (_, wire) = survey();
    let refused = |wire: Value| {
        serde_json::from_value::<Survey>(wire)
            .unwrap_err()
            .to_string()
    };
    let mut lost = wire.clone();
    lost.as_object_mut().unwrap().remove("writers");
    assert_eq!(refused(lost), "missing field `writers`");
    let mut unknown = wire.clone();
    unknown["plan"] = json!("/p");
    assert_eq!(
        refused(unknown),
        "unknown field `plan`, expected one of `launch`, `home`, `reach`, `network`, `writers`"
    );
    let positional =
        json!(["docs-mcp", "/home/op", {"writable": [], "readable": []}, "shared", [1000]]);
    assert_eq!(
        refused(positional),
        "invalid type: sequence, expected struct Fields"
    );
    let twice = wire.to_string().replacen('{', r#"{"launch":"sh","#, 1);
    assert_eq!(
        serde_json::from_str::<Survey>(&twice)
            .unwrap_err()
            .to_string(),
        "duplicate field `launch` at line 1 column 41"
    );
    let reply = |text: &str| {
        serde_json::from_str::<Surveyed>(text)
            .unwrap_err()
            .to_string()
    };
    assert_eq!(
        reply(r#"{"admitted":{}}"#),
        "unknown variant `admitted`, expected one of `provisioned`, `refused`, `unread` \
         at line 1 column 11"
    );
    assert_eq!(
        reply(r#"{"refused":"identity","refused":"linked"}"#),
        "expected value at line 1 column 21"
    );
}
