use std::path::PathBuf;

use serde_json::{json, Value};

use crate::agents::{Agent, Library, LibraryError};

/// A throwaway library holding one base office, `base`, with a tool list
/// and limits, and `boxed`, with hands; each test writes the overlay it is
/// about beside them.
struct Fixture {
    _guard: tempfile::TempDir,
    root: PathBuf,
}

impl Fixture {
    fn new() -> Fixture {
        let guard = tempfile::tempdir().unwrap();
        let root = guard.path().canonicalize().unwrap();
        std::fs::create_dir(root.join("charters")).unwrap();
        for charter in ["base.md", "other.md"] {
            std::fs::write(root.join("charters").join(charter), "# office\n").unwrap();
        }
        let fixture = Fixture {
            _guard: guard,
            root,
        };
        fixture.write("base", &base());
        fixture.write("boxed", &boxed());
        fixture
    }

    fn write(&self, name: &str, body: &Value) {
        std::fs::write(
            self.root.join(format!("{name}.json")),
            serde_json::to_vec_pretty(body).unwrap(),
        )
        .unwrap();
    }

    fn library(&self) -> Library {
        Library::load(&self.root).unwrap()
    }

    /// The refusal the loader makes of the library, with the overlay's own
    /// place spelt `<scoped>` so a row reads as the operator's text.
    fn refusal(&self, overlay: &Value) -> String {
        self.write("scoped", overlay);
        let error = Library::load(&self.root).unwrap_err();
        assert!(matches!(error, LibraryError::Invalid(_)), "{error:?}");
        let place = format!(
            "agent 'scoped' ({})",
            self.root.join("scoped.json").display()
        );
        error.to_string().replace(&place, "<scoped>")
    }
}

fn limits(agent: &Agent) -> Option<(u64, u64)> {
    agent
        .limits
        .map(|limits| (limits.max_attempts, limits.timeout_seconds))
}

fn base() -> Value {
    json!({
        "description": "the base office",
        "charter": "charters/base.md",
        "models": ["opus", "sonnet"],
        "efforts": {"opus": "high", "sonnet": "high"},
        "tools": {"allow": ["cargo", "git"], "mcp": []},
        "limits": {"max_attempts": 2, "timeout_seconds": 60},
    })
}

fn boxed() -> Value {
    json!({
        "description": "the boxed office",
        "charter": "charters/base.md",
        "models": ["sol"],
        "efforts": {"sol": "medium"},
        "hands": {"kind": "workspace", "network": false, "binds": []},
    })
}

/// The overlay IS the definition it stands for: its digest and source are
/// those of a standalone file restating the base with the overlay applied,
/// so a scoped office that moves from a copy to an overlay resolves, and
/// pins, exactly as it did.
#[test]
fn an_overlay_is_its_base_with_the_chain_replaced_and_a_named_loss_dropped() {
    let fixture = Fixture::new();
    fixture.write(
        "scoped",
        &json!({
            "extends": "base",
            "models": ["flash"],
            "efforts": {"flash": "high"},
            "replaces": {"tools": "the provider expresses no tool list"},
        }),
    );
    let mut copy = base();
    copy["models"] = json!(["flash"]);
    copy["efforts"] = json!({"flash": "high"});
    copy.as_object_mut().unwrap().remove("tools");
    fixture.write("copy", &copy);
    let library = fixture.library();
    let (scoped, copy) = (
        library.agent("scoped").unwrap(),
        library.agent("copy").unwrap(),
    );
    assert_eq!(scoped.models, ["flash"]);
    assert_eq!(scoped.efforts["flash"], "high");
    assert_eq!(scoped.allow, None);
    assert_eq!(limits(scoped), Some((2, 60)));
    assert_eq!(scoped.charter, fixture.root.join("charters/base.md"));
    assert_eq!(scoped.source, copy.source);
    assert_eq!(scoped.digest, copy.digest);
}

/// What an overlay does not name it inherits — tools, hands, limits and
/// the charter — and efforts go with the chain they were keyed by.
#[test]
fn an_overlay_inherits_the_power_it_does_not_name_and_hires_no_stale_effort() {
    let fixture = Fixture::new();
    fixture.write("scoped", &json!({"extends": "base", "models": ["opus"]}));
    fixture.write("narrow", &json!({"extends": "boxed", "models": ["sol"]}));
    let library = fixture.library();
    let scoped = library.agent("scoped").unwrap();
    assert_eq!(scoped.allow, Some(vec!["cargo".into(), "git".into()]));
    assert_eq!(limits(scoped), Some((2, 60)));
    assert!(scoped.efforts.is_empty());
    let narrow = library.agent("narrow").unwrap();
    assert_eq!(narrow.hands, library.agent("boxed").unwrap().hands);
}

/// A replacement written with its reason stands, a charter override is
/// still a charter of this library, and declared inputs are the office's.
#[test]
fn a_reasoned_replacement_and_a_charter_override_stand() {
    let fixture = Fixture::new();
    fixture.write(
        "scoped",
        &json!({
            "extends": "boxed",
            "models": ["opus"],
            "charter": "charters/other.md",
            "inputs": ["has_security_residual"],
            "tools": {"allow": ["git"]},
            "replaces": {
                "hands": "this seat runs the harness's own tools",
                "tools": "the harness's own tools, held to git",
            },
        }),
    );
    let library = fixture.library();
    let scoped = library.agent("scoped").unwrap();
    assert_eq!(scoped.allow, Some(vec!["git".into()]));
    assert_eq!(scoped.hands, None);
    assert_eq!(scoped.charter, fixture.root.join("charters/other.md"));
    assert_eq!(scoped.inputs, Some(vec!["has_security_residual".into()]));
}

#[test]
fn an_overlay_that_hides_a_change_or_reaches_past_one_level_is_refused() {
    let fixture = Fixture::new();
    fixture.write("deeper", &json!({"extends": "base", "models": ["opus"]}));
    let rows = [
        (
            json!({"extends": "base", "models": ["opus"], "tools": {"allow": ["git"]}}),
            "<scoped> writes 'tools' over 'base' without a reason; an overlay that \
             changes an office's tools names it under 'replaces' with why",
        ),
        (
            json!({"extends": "base", "models": ["opus"], "replaces": {"hands": "why"}}),
            "<scoped> 'replaces' gives a reason for 'hands', which neither it writes nor \
             'base' declares",
        ),
        (
            json!({"extends": "base", "models": ["opus"], "replaces": {"tools": ""}}),
            "<scoped> 'replaces' needs a non-empty string 'tools'",
        ),
        (
            json!({"extends": "base", "models": ["opus"], "replaces": {"limits": "why"}}),
            "<scoped> 'replaces' has unknown key 'limits'; known keys: tools, hands",
        ),
        (
            json!({"extends": "base", "models": ["opus"], "limits": {"max_attempts": 1}}),
            "<scoped> has unknown key 'limits'; known keys: extends, models, efforts, \
             charter, inputs, tools, hands, replaces",
        ),
        (
            json!({"extends": "base"}),
            "<scoped> extends 'base' but writes no 'models'; an overlay states the chain \
             it hires",
        ),
        (
            json!({"extends": "absent", "models": ["opus"]}),
            "<scoped> extends 'absent', which is not an agent in this library",
        ),
        (
            json!({"extends": "deeper", "models": ["opus"]}),
            "<scoped> extends 'deeper', which itself extends another office; an overlay \
             extends a base office, one level",
        ),
        (
            json!({"extends": "../base", "models": ["opus"]}),
            "<scoped> 'extends' names '../base', which does not match ^[a-z][a-z0-9-]*$",
        ),
        (
            json!({"extends": "base", "models": ["opus"], "efforts": {"flash": "high"}}),
            "<scoped> 'efforts' names an effort for 'flash', which is not in its 'models' \
             chain [opus]",
        ),
    ];
    for (overlay, expected) in rows {
        assert_eq!(fixture.refusal(&overlay), expected, "{overlay}");
    }
}
