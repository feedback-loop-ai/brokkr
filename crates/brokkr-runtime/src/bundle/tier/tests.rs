//! Proposed decision 0075 ruling 5, arm by arm: a provisional model is
//! refused at a gate whatever the list says, refused in an office the
//! operator did not list, admitted in one they did, and judged on every
//! link of a chain. Every provider here is invented; the shipped tree is
//! read only to prove it declares nothing provisional.

use super::*;
use crate::bundle::{Bundle, SeatBody};
use serde_json::json;
use std::path::{Path, PathBuf};

/// A workspace of one adapter, `newcomer`, mapping a promoted `steady`
/// and a provisional `fresh`, plus the agents and bundle a test writes.
struct Workspace {
    dir: tempfile::TempDir,
}

impl Workspace {
    fn new() -> Workspace {
        let workspace = Workspace {
            dir: tempfile::tempdir().unwrap(),
        };
        std::fs::create_dir_all(workspace.path("agents/charters")).unwrap();
        std::fs::create_dir_all(workspace.path("adapters")).unwrap();
        workspace.adapter(json!({"id": "fresh-1", "tier": "provisional"}));
        workspace.agent("reviewer", &["steady"]);
        workspace
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.dir.path().join(relative)
    }

    /// The one adapter, with `fresh` written as given: the provisional
    /// declaration, or the bare id a promotion leaves behind.
    fn adapter(&self, fresh: Value) {
        let adapter = json!({
            "provider": "newcomer", "binary": "newcomer", "trust_tier": "trusted",
            "egress": "contracted", "driver": ["{brokkr}", "driver", "newcomer", "--"],
            "models": {"steady": "steady-1", "fresh": fresh}, "judges": ["steady", "fresh"],
            "model_flag": "--model", "efforts": [], "effort_flag": "unsupported",
            "tool_permissions": "unsupported", "mcp": "unsupported",
        });
        std::fs::write(self.path("adapters/newcomer.json"), adapter.to_string()).unwrap();
    }

    fn agent(&self, name: &str, models: &[&str]) {
        std::fs::write(
            self.path(&format!("agents/charters/{name}.md")),
            "# charter\n",
        )
        .unwrap();
        let body = json!({"description": "a fixture office", "charter": format!("charters/{name}.md"), "models": models});
        std::fs::write(self.path(&format!("agents/{name}.json")), body.to_string()).unwrap();
    }

    /// Compile a bundle whose `work` seat is `work`, under a map listing
    /// `offices`. The review seat hires a promoted model, so the compile
    /// opens the adapters whatever the work seat is.
    fn compile(&self, work: Value, offices: &[&str]) -> Result<Bundle, CompileError> {
        let bundle = self.path("bundle");
        std::fs::create_dir_all(bundle.join("roles")).unwrap();
        std::fs::write(bundle.join("roles/role.md"), "# role\n").unwrap();
        let policy = json!({
            "schema": "forge.phase-machine/v1", "phases": ["work", "review", "done"],
            "initial": "work", "terminal": ["done"], "shippable_from": ["review"],
            "rules": [
                {"id": "W", "from": "work", "result": "pass", "next": "review", "reason": "worked"},
                {"id": "R", "from": "review", "result": "clean", "next": "done", "reason": "read"},
            ],
        });
        std::fs::write(bundle.join("policy.json"), policy.to_string()).unwrap();
        let review = json!({"results": ["clean"], "agent": "reviewer"});
        let config = json!({"name": "tier", "policy": "policy.json", "seats": {"work": work, "review": review}});
        std::fs::write(bundle.join("bundle.json"), config.to_string()).unwrap();
        Bundle::compile_with_realm(
            &bundle,
            &self.path("agents"),
            &self.path("adapters"),
            None,
            None,
            RealmLaw {
                boundary: Boundary::Namespace,
                provisional_offices: offices.iter().map(|office| office.to_string()).collect(),
            },
        )
    }

    fn refusal(&self, work: Value, offices: &[&str]) -> ProvisionalRefusal {
        match self.compile(work, offices) {
            Err(CompileError::Provisional(refusal)) => refusal,
            other => panic!("expected a provisional refusal, got {other:?}"),
        }
    }
}

fn seat(agent: &str, class: &str) -> Value {
    json!({"results": ["pass"], "agent": agent, "class": class})
}

fn inline(class: &str, model: &str) -> Value {
    json!({
        "role": "roles/role.md", "results": ["pass"], "class": class,
        "driver": {"command": ["{brokkr}", "driver", "newcomer", "--", "--model", model]},
    })
}

fn chain(bundle: &Bundle) -> Vec<&str> {
    match &bundle.seats["work"].body {
        SeatBody::Single { candidates, .. } => {
            candidates.iter().map(|c| c.model.as_str()).collect()
        }
        other => panic!("a single seat, got {other:?}"),
    }
}

/// The one place each refusal's words are pinned.
#[test]
fn a_provisional_model_at_a_gate_is_refused_even_where_its_office_is_listed() {
    let workspace = Workspace::new();
    workspace.agent("researcher", &["fresh"]);
    let refusal = workspace.refusal(seat("researcher", "gate"), &["researcher"]);
    assert_eq!(
        refusal,
        ProvisionalRefusal::Gate {
            seat: "work".into(),
            link: 1,
            model: "fresh".into(),
            adapter: "newcomer".into(),
        }
    );
    assert_eq!(
        CompileError::from(refusal).to_string(),
        "bundle: seat 'work' is gate class but link 1 seats model 'fresh', which adapter \
         'newcomer' declares provisional; a provisional model never holds a gate, whatever \
         provisional_offices lists (proposed decision 0075 ruling 5)"
    );
}

#[test]
fn a_provisional_model_compiles_in_a_listed_work_office_and_nowhere_else() {
    let workspace = Workspace::new();
    workspace.agent("researcher", &["fresh"]);
    let listed = workspace
        .compile(seat("researcher", "work"), &["researcher"])
        .unwrap();
    assert_eq!(chain(&listed), ["fresh"]);

    let refusal = workspace.refusal(seat("researcher", "work"), &["review-correctness"]);
    assert_eq!(
        refusal,
        ProvisionalRefusal::Unlisted {
            seat: "work".into(),
            link: 1,
            model: "fresh".into(),
            adapter: "newcomer".into(),
            office: Some("researcher".into()),
        }
    );
    assert_eq!(
        refusal.to_string(),
        "seat 'work' link 1 seats model 'fresh', which adapter 'newcomer' declares \
         provisional, in office 'researcher', which realms.json does not list in \
         provisional_offices; a provisional model holds only the offices the operator lists, \
         and an absent or empty list names none (proposed decision 0075 ruling 5)"
    );
    // Absent or empty, the list names no office at all.
    assert_eq!(workspace.refusal(seat("researcher", "work"), &[]), refusal);
}

/// A chain that reaches the provisional model only by falling back is
/// refused the same way, naming the link a run could have fallen to.
#[test]
fn a_provisional_model_reached_only_by_fallback_is_refused_the_same_way() {
    let workspace = Workspace::new();
    workspace.agent("steady-hand", &["steady", "fresh"]);
    let office = Some("steady-hand".to_string());
    assert_eq!(
        workspace.refusal(seat("steady-hand", "work"), &[]),
        ProvisionalRefusal::Unlisted {
            seat: "work".into(),
            link: 2,
            model: "fresh".into(),
            adapter: "newcomer".into(),
            office,
        }
    );
    assert_eq!(
        workspace.refusal(seat("steady-hand", "gate"), &["steady-hand"]),
        ProvisionalRefusal::Gate {
            seat: "work".into(),
            link: 2,
            model: "fresh".into(),
            adapter: "newcomer".into(),
        }
    );
    let listed = workspace
        .compile(seat("steady-hand", "work"), &["steady-hand"])
        .unwrap();
    assert_eq!(chain(&listed), ["steady", "fresh"]);
}

/// An inline command names no agent, so it holds no office: its pinned
/// provisional model is seatable nowhere, and never at a gate. A pin on
/// a promoted model, no pin, and a command no adapter answers are not
/// the tier's to judge.
#[test]
fn an_inline_command_pinning_a_provisional_model_holds_no_office() {
    let workspace = Workspace::new();
    workspace.agent("researcher", &["steady"]);
    let refusal = workspace.refusal(inline("work", "fresh-1"), &["researcher"]);
    assert_eq!(
        refusal.to_string(),
        "seat 'work' link 1 seats model 'fresh', which adapter 'newcomer' declares \
         provisional, in no office (an inline command names no agent), which realms.json does \
         not list in provisional_offices; a provisional model holds only the offices the \
         operator lists, and an absent or empty list names none (proposed decision 0075 \
         ruling 5)"
    );
    assert_eq!(
        refusal,
        ProvisionalRefusal::Unlisted {
            seat: "work".into(),
            link: 1,
            model: "fresh".into(),
            adapter: "newcomer".into(),
            office: None,
        }
    );
    assert!(matches!(
        workspace.refusal(inline("gate", "fresh-1"), &[]),
        ProvisionalRefusal::Gate { link: 1, .. }
    ));
    for admitted in [
        inline("gate", "steady-1"),
        json!({"role": "roles/role.md", "results": ["pass"], "class": "gate",
               "driver": {"command": ["{brokkr}", "driver", "newcomer", "--"]}}),
        json!({"role": "roles/role.md", "results": ["pass"], "class": "gate",
               "driver": {"command": ["./judge.sh"]}}),
        json!({"role": "roles/role.md", "results": ["pass"], "class": "gate",
               "driver": {"command": ["{brokkr}", "driver", "ghost", "--", "--model", "fresh-1"]}}),
    ] {
        assert!(
            !matches!(
                workspace.compile(admitted.clone(), &[]),
                Err(CompileError::Provisional(_))
            ),
            "{admitted}"
        );
    }
}

/// Promotion is data only: the provisional entry rewritten as its bare
/// id compiles where it was refused, and maps the model to the same id.
#[test]
fn removing_the_tier_promotes_the_model_and_nothing_else_changes() {
    let workspace = Workspace::new();
    workspace.agent("researcher", &["fresh"]);
    let adapters = || Adapters::load(&workspace.path("adapters")).unwrap();
    let before = adapters().adapter("newcomer").unwrap().clone();
    assert!(matches!(
        workspace.refusal(seat("researcher", "gate"), &[]),
        ProvisionalRefusal::Gate { .. }
    ));
    workspace.adapter(json!("fresh-1"));
    let after = adapters().adapter("newcomer").unwrap().clone();
    assert_eq!(before.provisional, ["fresh".to_string()].into());
    assert_eq!(after.provisional, Default::default());
    assert_eq!(before.models, after.models);
    let promoted = workspace.compile(seat("researcher", "gate"), &[]).unwrap();
    assert_eq!(chain(&promoted), ["fresh"]);
}

/// The tier is declared inside a `models` entry, closed, and the only
/// word it takes is `provisional`; every other shape of `models` is
/// refused in the words it always was.
#[test]
fn the_model_tier_is_a_closed_declaration_inside_the_models_map() {
    let workspace = Workspace::new();
    let refused = |fresh: Value| {
        workspace.adapter(fresh);
        Adapters::load(&workspace.path("adapters"))
            .unwrap_err()
            .to_string()
    };
    let file = format!(
        "adapter 'newcomer' ({})",
        workspace
            .path("adapters/newcomer.json")
            .canonicalize()
            .unwrap()
            .display()
    );
    assert_eq!(
        refused(json!({"id": "fresh-1", "tier": "promoted"})),
        format!(
            "{file} 'models.fresh' declares tier 'promoted'; the only tier a model declares is \
             \"provisional\", and a model that declares none is promoted (proposed decision \
             0075 ruling 5)"
        )
    );
    assert_eq!(
        refused(json!({"id": "fresh-1"})),
        format!("{file} 'models.fresh' needs a non-empty string 'tier'")
    );
    assert_eq!(
        refused(json!({"tier": "provisional"})),
        format!("{file} 'models.fresh' needs a non-empty string 'id'")
    );
    assert_eq!(
        refused(json!({"id": "fresh-1", "tier": "provisional", "since": "0075"})),
        format!("{file} 'models.fresh' has unknown key 'since'; known keys: id, tier")
    );
    assert_eq!(
        refused(json!(7)),
        format!("{file} 'models.fresh' must be a non-empty string")
    );
    let mut adapter: Value =
        serde_json::from_slice(&std::fs::read(workspace.path("adapters/newcomer.json")).unwrap())
            .unwrap();
    adapter["models"] = json!(["steady"]);
    std::fs::write(
        workspace.path("adapters/newcomer.json"),
        adapter.to_string(),
    )
    .unwrap();
    assert_eq!(
        Adapters::load(&workspace.path("adapters"))
            .unwrap_err()
            .to_string(),
        format!("{file} needs 'models' as an object of strings")
    );
}

fn shipped() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every model shipped today is promoted, so the check can refuse no
/// shipped recipe: each compiles as it did, with no office listed.
#[test]
fn every_shipped_model_is_promoted_and_every_shipped_recipe_still_compiles() {
    let root = shipped();
    let adapters = Adapters::load(&root.join("adapters")).unwrap();
    for adapter in adapters.providers() {
        assert_eq!(
            adapter.provisional,
            Default::default(),
            "{}",
            adapter.provider
        );
    }
    let compiled = shipped_compiles(&root.join("adapters"));
    assert!(compiled.len() >= 17, "{compiled:?}");
    for (path, result) in compiled {
        assert_eq!(result, Ok(()), "{path}");
    }
}

/// The removal control the test above stands on: mark the model a
/// shipped recipe pins provisional, in a copy of the shipped adapters,
/// and that recipe is refused at its first site.
#[test]
fn a_shipped_recipe_is_refused_once_a_model_it_hires_turns_provisional() {
    let root = shipped();
    let copy = tempfile::tempdir().unwrap();
    for entry in std::fs::read_dir(root.join("adapters")).unwrap() {
        let path = entry.unwrap().path();
        let mut adapter: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        if adapter["provider"] == "claude" {
            let id = adapter["models"]["fable"].clone();
            adapter["models"]["fable"] = json!({"id": id, "tier": "provisional"});
        }
        std::fs::write(
            copy.path().join(path.file_name().unwrap()),
            adapter.to_string(),
        )
        .unwrap();
    }
    let fast = Bundle::compile_under(
        &root.join("recipes/fast"),
        &root.join("agents"),
        copy.path(),
        Boundary::Namespace,
    );
    match fast {
        Err(CompileError::Provisional(refusal)) => assert_eq!(
            refusal,
            ProvisionalRefusal::Unlisted {
                seat: "implement".into(),
                link: 1,
                model: "fable".into(),
                adapter: "claude".into(),
                office: None,
            }
        ),
        other => panic!("expected a provisional refusal, got {other:?}"),
    }
}

/// Each shipped bundle and recipe compiled against `adapters`, in no
/// realm and listing no provisional office.
fn shipped_compiles(adapters: &Path) -> Vec<(String, Result<(), String>)> {
    let root = shipped();
    let listed = |kind: &'static str| {
        let entries = std::fs::read_dir(root.join(kind)).unwrap();
        entries.map(move |entry| (kind, entry.unwrap().path()))
    };
    let mut compiled: Vec<_> = listed("recipes")
        .chain(listed("bundles"))
        .filter(|(_, dir)| dir.join("bundle.json").is_file())
        .map(|(kind, dir)| {
            let name = dir.file_name().unwrap().to_string_lossy().to_string();
            let outcome =
                Bundle::compile_under(&dir, &root.join("agents"), adapters, Boundary::Namespace);
            (
                format!("{kind}/{name}"),
                outcome.map(drop).map_err(|e| e.to_string()),
            )
        })
        .collect();
    compiled.sort();
    compiled
}
