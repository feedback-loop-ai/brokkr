//! Proposed decision 0075 ruling 5, arm by arm: a provisional model is
//! refused at a gate whatever the list says, refused in an office the
//! operator did not list, admitted in one they did, and judged on every
//! link of a chain. Every provider here is invented; the shipped tree is
//! read only to prove it declares nothing provisional.

use super::*;
use crate::agents::LibraryError;
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
        self.adapter_as("newcomer", fresh, &[]);
    }

    /// The same adapter under another provider's name, declaring `efforts`
    /// on `--effort` where it names any: a built-in kind is what an inline
    /// seat can spawn, and what the optional inline read loads the adapters
    /// for.
    fn adapter_as(&self, provider: &str, fresh: Value, efforts: &[&str]) {
        let models = json!({"steady": "steady-1", "fresh": fresh});
        self.adapter_with(provider, models, "--model", efforts);
    }

    /// An adapter of `provider` mapping `models`, every one a judge, told
    /// a model on `model_flag`.
    fn adapter_with(&self, provider: &str, models: Value, model_flag: &str, efforts: &[&str]) {
        let effort_flag = if efforts.is_empty() {
            "unsupported"
        } else {
            "--effort"
        };
        let judges: Vec<&String> = models.as_object().unwrap().keys().collect();
        let adapter = json!({
            "provider": provider, "binary": provider, "trust_tier": "trusted",
            "egress": "contracted", "driver": ["{brokkr}", "driver", provider, "--"],
            "models": models, "judges": judges,
            "model_flag": model_flag, "efforts": efforts, "effort_flag": effort_flag,
            "tool_permissions": "unsupported", "mcp": "unsupported",
        });
        let file = self.path(&format!("adapters/{provider}.json"));
        std::fs::write(file, adapter.to_string()).unwrap();
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
    /// `offices`. The review seat hires a promoted model through an agent.
    fn compile(&self, work: Value, offices: &[&str]) -> Result<Bundle, CompileError> {
        let review = json!({"results": ["clean"], "agent": "reviewer"});
        self.compile_beside(work, review, offices)
    }

    /// The same, with the review seat as given.
    fn compile_beside(
        &self,
        work: Value,
        review: Value,
        offices: &[&str],
    ) -> Result<Bundle, CompileError> {
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
        let config = json!({"name": "tier", "policy": "policy.json", "seats": {"work": work, "review": review}});
        std::fs::write(bundle.join("bundle.json"), config.to_string()).unwrap();
        Bundle::compile_under(
            &bundle,
            &self.path("agents"),
            &self.path("adapters"),
            RealmLaw {
                boundary: Boundary::Namespace,
                provisional_offices: offices.iter().map(|office| office.to_string()).collect(),
            },
        )
    }

    fn refusal(&self, work: Value, offices: &[&str]) -> ProvisionalRefusal {
        provisional(self.compile(work, offices))
    }
}

fn provisional(compiled: Result<Bundle, CompileError>) -> ProvisionalRefusal {
    match compiled {
        Err(CompileError::Provisional(refusal)) => refusal,
        other => panic!("expected a provisional refusal, got {other:?}"),
    }
}

fn seat(agent: &str, class: &str) -> Value {
    json!({"results": ["pass"], "agent": agent, "class": class})
}

fn inline(class: &str, model: &str) -> Value {
    inline_on("newcomer", class, &["--model", model])
}

/// An inline seat on `driver`, its tail as given.
fn inline_on(driver: &str, class: &str, tail: &[&str]) -> Value {
    let mut command = vec!["{brokkr}", "driver", driver, "--"];
    command.extend(tail);
    json!({"role": "roles/role.md", "results": ["pass"], "class": class,
           "driver": {"command": command}})
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
    // The refusal is the error's own text, not its source: a report that
    // walks the chain prints it once.
    let error = CompileError::Provisional(refusal);
    assert_eq!(
        std::error::Error::source(&error).map(ToString::to_string),
        None
    );
    assert_eq!(
        error.to_string(),
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
/// a promoted model and a command that dispatches no driver are not the
/// tier's to judge; a driver no adapter declares declares no id, so its
/// pin is refused.
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
    assert_eq!(
        workspace.refusal(inline_on("ghost", "gate", &["--model", "fresh-1"]), &[]),
        ProvisionalRefusal::Undeclared {
            seat: "work".into(),
            adapter: "newcomer".into(),
            flags: vec!["--model".into()],
            value: "fresh-1".into(),
        }
    );
    for admitted in [
        inline("gate", "steady-1"),
        json!({"role": "roles/role.md", "results": ["pass"], "class": "gate",
               "driver": {"command": ["./judge.sh"]}}),
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

/// An inline pin the tier cannot read, here `--model` twice with one pin
/// on the provisional id, may reach the provisional model, so it is
/// refused naming the flag read, at a work seat as at a gate. Under an
/// adapter that declares nothing provisional the tier has nothing to
/// judge, and a gate's own refusal of the unreadable pin stands.
#[test]
fn an_inline_pin_the_tier_cannot_read_is_refused_where_the_adapter_declares_a_provisional_model() {
    let workspace = Workspace::new();
    let twice = |class: &str| {
        let mut site = inline(class, "fresh-1");
        site["driver"]["command"]
            .as_array_mut()
            .unwrap()
            .extend([json!("--model"), json!("steady-1")]);
        site
    };
    let refusal = workspace.refusal(twice("work"), &[]);
    assert_eq!(
        refusal,
        ProvisionalRefusal::Unreadable {
            seat: "work".into(),
            adapter: "newcomer".into(),
            flags: vec!["--model".into()],
        }
    );
    assert_eq!(
        refusal.to_string(),
        "seat 'work' pins its model on '--model', which this compiler cannot read as one \
         concrete id, and adapter 'newcomer' declares a provisional model the pin may reach; a \
         pin the tier cannot read is refused, never read as promoted (proposed decision 0075 \
         ruling 5)"
    );
    assert_eq!(workspace.refusal(twice("gate"), &[]), refusal);

    workspace.adapter(json!("fresh-1"));
    assert_eq!(workspace.compile(twice("work"), &[]).unwrap().name, "tier");
    match workspace.compile(twice("gate"), &[]) {
        Err(CompileError::Invalid(message)) => assert_eq!(
            message,
            "seat 'work' gate link 1 names model '<unmapped>', which driver 'newcomer' does not \
             declare in 'judges' (decision 0041 ruling 3 — an absent declaration is empty)"
        ),
        other => panic!("expected the judges refusal, got {other:?}"),
    }
}

/// An inline command's `--fallback-model` is the model a run falls to, so
/// it is link 2 of the chain and judged as link 1 is: a provisional id
/// there is refused at a work seat and at a gate, a fallback pinned twice
/// is unreadable, and a promoted one is not the tier's.
#[test]
fn an_inline_fallback_model_is_judged_as_a_second_link() {
    let workspace = Workspace::new();
    let falling = |class: &str, tail: &[&str]| {
        let mut site = inline(class, "steady-1");
        let command = site["driver"]["command"].as_array_mut().unwrap();
        command.extend(tail.iter().map(|word| json!(word)));
        site
    };
    let to_fresh = ["--fallback-model", "fresh-1"];
    assert_eq!(
        workspace.refusal(falling("work", &to_fresh), &[]),
        ProvisionalRefusal::Unlisted {
            seat: "work".into(),
            link: 2,
            model: "fresh".into(),
            adapter: "newcomer".into(),
            office: None,
        }
    );
    assert_eq!(
        workspace.refusal(falling("gate", &["--fallback-model=fresh-1"]), &[]),
        ProvisionalRefusal::Gate {
            seat: "work".into(),
            link: 2,
            model: "fresh".into(),
            adapter: "newcomer".into(),
        }
    );
    let twice = [
        "--fallback-model",
        "steady-1",
        "--fallback-model",
        "fresh-1",
    ];
    assert_eq!(
        workspace.refusal(falling("work", &twice), &[]),
        ProvisionalRefusal::Unreadable {
            seat: "work".into(),
            adapter: "newcomer".into(),
            flags: vec!["--fallback-model".into()],
        }
    );
    let steady = falling("work", &["--fallback-model", "steady-1"]);
    assert_eq!(workspace.compile(steady, &[]).unwrap().name, "tier");
}

/// An inline pin that is the id of no model the adapter declares, the
/// alias `fresh` a harness may resolve itself or an id the file never
/// names, may reach the provisional model, so it is refused naming the
/// flags its link is read on, on link 1 and link 2 alike. Promoted, the
/// same pin is not the tier's.
#[test]
fn an_inline_pin_on_no_declared_id_is_refused_where_the_adapter_declares_a_provisional_model() {
    let workspace = Workspace::new();
    let refusal = workspace.refusal(inline("work", "fresh"), &[]);
    assert_eq!(
        refusal,
        ProvisionalRefusal::Undeclared {
            seat: "work".into(),
            adapter: "newcomer".into(),
            flags: vec!["--model".into()],
            value: "fresh".into(),
        }
    );
    assert_eq!(
        refusal.to_string(),
        "seat 'work' pins 'fresh' on '--model', which is the id of no model the seat's own \
         adapter declares, and adapter 'newcomer' declares a provisional model; an alias or an \
         undeclared id may reach that model, so a pin the tier cannot map to a declared id is \
         refused, never read as promoted (proposed decision 0075 ruling 5)"
    );
    assert_eq!(workspace.refusal(inline("gate", "fresh"), &[]), refusal);
    let mut falling = inline("work", "steady-1");
    let command = falling["driver"]["command"].as_array_mut().unwrap();
    command.extend([json!("--fallback-model"), json!("other-9")]);
    assert_eq!(
        workspace.refusal(falling.clone(), &[]),
        ProvisionalRefusal::Undeclared {
            seat: "work".into(),
            adapter: "newcomer".into(),
            flags: vec!["--fallback-model".into()],
            value: "other-9".into(),
        }
    );

    workspace.adapter(json!("fresh-1"));
    assert_eq!(workspace.compile(falling, &[]).unwrap().name, "tier");
}

/// A model pin is the engine's to compose, where the tier judges it, so an
/// adapter whose own argv names a model flag is refused at load: in each
/// hands fragment, in either spelling, and a short `model_flag` with its
/// value attached. A longer flag of the same family is not a pin.
#[test]
fn an_adapter_fragment_that_names_a_model_flag_is_refused_at_load() {
    let workspace = Workspace::new();
    let file = adapter_file(&workspace);
    let path = workspace.path("adapters/newcomer.json");
    let declared: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let load = |edits: Value| {
        let mut adapter = declared.clone();
        adapter
            .as_object_mut()
            .unwrap()
            .extend(edits.as_object().unwrap().clone());
        std::fs::write(&path, adapter.to_string()).unwrap();
        Adapters::load(&workspace.path("adapters")).map(drop)
    };
    let tail = |words: &[&str]| {
        let mut driver = declared["driver"].clone();
        let command = driver.as_array_mut().unwrap();
        command.extend(words.iter().map(|word| json!(word)));
        driver
    };
    let harness = |member: &str, words: &[&str]| json!({"hands": {"workspace": [], "harness": {member: words}}});
    for (edits, at, flag) in [
        (
            json!({"hands": {"workspace": ["--model=fresh-1"]}}),
            "hands.workspace",
            "--model",
        ),
        (
            harness("gate", &["--model", "x"]),
            "hands.harness.gate",
            "--model",
        ),
        (
            harness("work", &["--fallback-model", "fresh-1"]),
            "hands.harness.work",
            "--fallback-model",
        ),
        (
            json!({"driver": tail(&["--fallback-model=fresh-1"])}),
            "driver",
            "--fallback-model",
        ),
        (
            json!({"model_flag": "-m", "driver": tail(&["-mfresh-1"])}),
            "driver",
            "-m",
        ),
    ] {
        match load(edits) {
            Err(LibraryError::Invalid(message)) => assert_eq!(
                message,
                format!(
                    "{file} '{at}' names the model flag '{flag}'; a model pin is the engine's to \
                     compose from a seat's chain, where the provisional tier judges it, and one \
                     an adapter carries would seat a model no tier check reads (proposed \
                     decision 0075 ruling 5)"
                )
            ),
            other => panic!("expected '{at}' refused, got {other:?}"),
        }
    }
    load(json!({"model_flag": "-m"})).unwrap();
    load(json!({"driver": tail(&["--model-x", "fresh-1"])})).unwrap();
}

/// The same refusal on the shipped tree: a driver tail opens an inline
/// template's command, as `recipes/fast`'s review gate composes it from
/// claude's tools.allow lowering, and every agent candidate's command, as
/// `recipes/panel-review`'s positions hire claude. A copy of the shipped
/// adapters with claude's tail carrying `--fallback-model` refuses both.
#[test]
fn a_shipped_recipe_is_refused_once_its_adapter_tail_names_a_model() {
    let root = shipped();
    let copy = shipped_adapters_with(|adapter| {
        if adapter["provider"] == "claude" {
            let driver = adapter["driver"].as_array_mut().unwrap();
            driver.extend([json!("--fallback-model"), json!("claude-opus-5-5")]);
        }
    });
    let claude = copy.path().canonicalize().unwrap().join("claude.json");
    let refusal = format!(
        "adapter 'claude' ({}) 'driver' names the model flag '--fallback-model'",
        claude.display()
    );
    for recipe in ["fast", "panel-review"] {
        let compiled = Bundle::compile_under(
            &root.join("recipes").join(recipe),
            &root.join("agents"),
            copy.path(),
            Boundary::Namespace,
        );
        match compiled {
            Err(CompileError::Invalid(message)) => {
                assert!(message.starts_with(&refusal), "{recipe}: {message}");
            }
            other => panic!("expected {recipe} refused, got {other:?}"),
        }
    }
}

/// An inline command that pins no model on the adapter's flag leaves the
/// model to the harness's default, which may be the provisional one, so
/// it is refused at a work seat as at a gate — whether a later rule would
/// demand a pin or not. Promoted, the same commands are not the tier's.
#[test]
fn an_inline_command_that_pins_no_model_is_refused_where_the_adapter_declares_a_provisional_model()
{
    let workspace = Workspace::new();
    let unpinned = |class: &str, tail: &[&str]| inline_on("newcomer", class, tail);
    let refusal = workspace.refusal(unpinned("work", &[]), &[]);
    assert_eq!(
        refusal,
        ProvisionalRefusal::Unpinned {
            seat: "work".into(),
            adapter: "newcomer".into(),
        }
    );
    assert_eq!(
        refusal.to_string(),
        "seat 'work' pins no model, and adapter 'newcomer' declares a provisional model the \
         harness's own default may be; pin a promoted model's id on the adapter's model flag, \
         because a seat whose model the tier cannot read is refused, never read as promoted \
         (proposed decision 0075 ruling 5)"
    );
    assert_eq!(workspace.refusal(unpinned("gate", &[]), &[]), refusal);
    // A flag the model read walks past pins nothing either.
    assert_eq!(
        workspace.refusal(unpinned("work", &["--model-x", "fresh-1"]), &[]),
        refusal
    );

    workspace.adapter(json!("fresh-1"));
    assert_eq!(
        workspace.compile(unpinned("work", &[]), &[]).unwrap().name,
        "tier"
    );
    let gate = workspace.compile(unpinned("gate", &[]), &[]);
    assert!(
        !matches!(gate, Err(CompileError::Provisional(_))),
        "{gate:?}"
    );
}

/// A bundle that names no agent, seats no gate and binds no secret still
/// reads the adapters for its inline built-in model drivers (decision 0066
/// ruling 1), so a seat that leaves its class undeclared — work — is
/// judged. With no adapter data nothing is declared provisional, and the
/// bundle compiles with none in sight.
#[test]
fn an_inline_seat_is_judged_where_the_bundle_names_no_agent_and_no_gate() {
    let workspace = Workspace::new();
    std::fs::remove_file(workspace.path("adapters/newcomer.json")).unwrap();
    let fresh = json!({"id": "fresh-1", "tier": "provisional"});
    workspace.adapter_as("lanetally", fresh, &["high"]);
    let unclassed = |model: &str| {
        let command = [
            "{brokkr}",
            "driver",
            "lanetally",
            "--",
            "--model",
            model,
            "--effort",
            "high",
        ];
        json!({"role": "roles/role.md", "results": ["pass"], "driver": {"command": command}})
    };
    let mut review = unclassed("steady-1");
    review["results"] = json!(["clean"]);
    let compile = |work: Value| workspace.compile_beside(work, review.clone(), &[]);
    assert_eq!(
        provisional(compile(unclassed("fresh-1"))),
        ProvisionalRefusal::Unlisted {
            seat: "work".into(),
            link: 1,
            model: "fresh".into(),
            adapter: "lanetally".into(),
            office: None,
        }
    );
    assert_eq!(compile(unclassed("steady-1")).unwrap().name, "tier");
    std::fs::remove_dir_all(workspace.path("adapters")).unwrap();
    assert_eq!(compile(unclassed("fresh-1")).unwrap().name, "tier");
}

/// The tier is the id's, not the entry's. `sibling` declares nothing
/// provisional but maps `borrowed` to `fresh-1`, the id `newcomer` declares
/// provisional, so an inline pin on it, a gate's fallback on it, and an
/// agent hiring it are each refused naming `newcomer` and its `fresh`, as
/// a second alias of the id inside `newcomer` itself is.
#[test]
fn a_model_is_judged_by_the_adapter_that_declares_its_id_provisional_whichever_serves_it() {
    let workspace = Workspace::new();
    let sibling = json!({"plain": "steady-1", "borrowed": "fresh-1"});
    workspace.adapter_with("sibling", sibling, "--model", &[]);
    let unlisted = |link: usize, office: Option<&str>| ProvisionalRefusal::Unlisted {
        seat: "work".into(),
        link,
        model: "fresh".into(),
        adapter: "newcomer".into(),
        office: office.map(str::to_string),
    };
    let pinned = inline_on("sibling", "work", &["--model", "fresh-1"]);
    assert_eq!(workspace.refusal(pinned, &[]), unlisted(1, None));
    let falling = ["--model", "steady-1", "--fallback-model", "fresh-1"];
    assert_eq!(
        workspace.refusal(inline_on("sibling", "gate", &falling), &[]),
        ProvisionalRefusal::Gate {
            seat: "work".into(),
            link: 2,
            model: "fresh".into(),
            adapter: "newcomer".into(),
        }
    );
    workspace.agent("borrower", &["plain", "borrowed"]);
    let borrower = seat("borrower", "work");
    assert_eq!(
        workspace.refusal(borrower.clone(), &[]),
        unlisted(2, Some("borrower"))
    );
    let listed = workspace.compile(borrower, &["borrower"]).unwrap();
    assert_eq!(chain(&listed), ["plain", "borrowed"]);

    let aliased = json!({"steady": "steady-1", "fresh": {"id": "fresh-1", "tier": "provisional"},
                         "fresh-b": "fresh-1"});
    workspace.adapter_with("newcomer", aliased, "--model", &[]);
    workspace.agent("aliaser", &["fresh-b"]);
    assert_eq!(
        workspace.refusal(seat("aliaser", "work"), &[]),
        unlisted(1, Some("aliaser"))
    );
}

/// Once any loaded adapter declares a provisional model, an inline seat on
/// an adapter that declares none must pin an id its own adapter declares:
/// the alias `fresh` may resolve to the provisional id, so it is refused
/// naming the adapter that declares one, and no pin is refused as it is on
/// `newcomer`. A driver that declares no model and can be told none,
/// exec's shape, is not the tier's, and with the model promoted neither
/// refusal stands.
#[test]
fn an_inline_pin_its_own_adapter_does_not_declare_is_refused_once_any_adapter_declares_one() {
    let workspace = Workspace::new();
    workspace.adapter_with("sibling", json!({"plain": "steady-1"}), "--model", &[]);
    workspace.adapter_with("shell", json!({}), "unsupported", &[]);
    let alias = inline_on("sibling", "work", &["--model", "fresh"]);
    assert_eq!(
        workspace.refusal(alias.clone(), &[]),
        ProvisionalRefusal::Undeclared {
            seat: "work".into(),
            adapter: "newcomer".into(),
            flags: vec!["--model".into()],
            value: "fresh".into(),
        }
    );
    assert_eq!(
        workspace.refusal(inline_on("sibling", "work", &[]), &[]),
        ProvisionalRefusal::Unpinned {
            seat: "work".into(),
            adapter: "newcomer".into(),
        }
    );
    let declared = inline_on("sibling", "work", &["--model", "steady-1"]);
    assert_eq!(workspace.compile(declared, &[]).unwrap().name, "tier");
    let shell = inline_on("shell", "work", &["bash", "./verify.sh"]);
    assert_eq!(workspace.compile(shell, &[]).unwrap().name, "tier");

    workspace.adapter(json!("fresh-1"));
    assert_eq!(workspace.compile(alias, &[]).unwrap().name, "tier");
}

/// Promotion is data only: the provisional entry with its tier removed,
/// or rewritten as its bare id, compiles where it was refused, and maps
/// the model to the same id.
#[test]
fn removing_the_tier_promotes_the_model_and_nothing_else_changes() {
    let workspace = Workspace::new();
    workspace.agent("researcher", &["fresh"]);
    let adapters = || Adapters::load(&workspace.path("adapters")).unwrap();
    let before = adapters().adapter("newcomer").unwrap().clone();
    assert_eq!(before.provisional, ["fresh".to_string()].into());
    for promoted in [json!({"id": "fresh-1"}), json!("fresh-1")] {
        workspace.adapter(json!({"id": "fresh-1", "tier": "provisional"}));
        assert!(matches!(
            workspace.refusal(seat("researcher", "gate"), &[]),
            ProvisionalRefusal::Gate { .. }
        ));
        workspace.adapter(promoted.clone());
        let after = adapters().adapter("newcomer").unwrap().clone();
        assert_eq!(after.provisional, Default::default(), "{promoted}");
        assert_eq!(before.models, after.models, "{promoted}");
        let compiled = workspace.compile(seat("researcher", "gate"), &[]).unwrap();
        assert_eq!(chain(&compiled), ["fresh"], "{promoted}");
    }
}

/// The tier is declared inside a `models` entry, closed, and the only
/// word it takes is `provisional`; every other shape of `models` is
/// refused in the words it always was.
#[test]
fn the_model_tier_is_a_closed_declaration_inside_the_models_map() {
    let workspace = Workspace::new();
    let refused = |fresh: Value| {
        workspace.adapter(fresh);
        invalid(&workspace)
    };
    let file = adapter_file(&workspace);
    assert_eq!(
        refused(json!({"id": "fresh-1", "tier": "promoted"})),
        format!(
            "{file} 'models.fresh' declares tier 'promoted'; the only tier a model declares is \
             \"provisional\", and a model that declares none is promoted (proposed decision \
             0075 ruling 5)"
        )
    );
    // A tier that is written is read: one that cannot be is refused, not
    // taken for the promotion only an absent key declares.
    assert_eq!(
        refused(json!({"id": "fresh-1", "tier": null})),
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
        invalid(&workspace),
        format!("{file} needs 'models' as an object of strings")
    );
}

/// A provisional entry written twice, the second copy bare, is refused by
/// the strict reader rather than promoted by whichever copy came second,
/// in an adapter that declares no native capabilities as in one that
/// does: every adapter is authority data.
#[test]
fn a_model_written_twice_is_refused_rather_than_promoted_by_its_second_copy() {
    let workspace = Workspace::new();
    workspace.agent("researcher", &["fresh"]);
    let path = workspace.path("adapters/newcomer.json");
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(!text.contains("native_capabilities"), "{text}");
    let declared = r#""fresh":{"id":"fresh-1","tier":"provisional"}"#;
    let bare = r#","fresh":"fresh-1""#;
    let twice = text.replacen(declared, &format!("{declared}{bare}"), 1);
    assert_ne!(twice, text);
    std::fs::write(&path, &twice).unwrap();
    // The parser reports the one-based column of the quote that closed
    // the repeated value.
    let column = twice.find(bare).unwrap() + bare.len();
    let refusal = format!(
        "{}: key 'fresh' is written twice at line 1 column {column}",
        adapter_file(&workspace)
    );
    assert_eq!(invalid(&workspace), refusal);
    match workspace.compile(seat("researcher", "gate"), &["researcher"]) {
        Err(CompileError::Invalid(message)) => assert!(
            message.starts_with(&format!("{refusal}; the adapter data is")),
            "{message}"
        ),
        other => panic!("expected the duplicate refused, got {other:?}"),
    }
}

/// The adapter fixture as every load refusal names it.
fn adapter_file(workspace: &Workspace) -> String {
    let path = workspace.path("adapters/newcomer.json");
    format!(
        "adapter 'newcomer' ({})",
        path.canonicalize().unwrap().display()
    )
}

/// What loading the fixture's adapters refuses, as the variant carries it.
fn invalid(workspace: &Workspace) -> String {
    match Adapters::load(&workspace.path("adapters")) {
        Err(LibraryError::Invalid(message)) => message,
        other => panic!("expected an invalid adapter, got {other:?}"),
    }
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
    assert_every_shipped_compiles(&root.join("adapters"));
}

/// The removal control the test above stands on: mark the model a
/// shipped recipe pins provisional, in a copy of the shipped adapters,
/// and that recipe is refused at its first site.
#[test]
fn a_shipped_recipe_is_refused_once_a_model_it_hires_turns_provisional() {
    let root = shipped();
    let copy = shipped_adapters_with(|adapter| {
        if adapter["provider"] == "claude" {
            let id = adapter["models"]["fable"].clone();
            adapter["models"]["fable"] = json!({"id": id, "tier": "provisional"});
        }
    });
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

/// The #487 landing's C2 on the shipped tree: once a model is declared
/// provisional, every inline pin is judged against its adapter's declared
/// ids, so a recipe pinning an id its adapter does not declare stops
/// compiling. Marking a dsh model no shipped recipe hires provisional
/// leaves every shipped recipe compiling: each inline pin names a
/// declared id, the dsh lanes `spark-glm/GLM-5.3-Flash-EXL3` (#532).
#[test]
fn every_shipped_recipe_compiles_beside_a_provisional_model_it_does_not_hire() {
    let copy = shipped_adapters_with(|adapter| {
        if adapter["provider"] == "dsh" {
            let id = adapter["models"]["studio-pro"].clone();
            adapter["models"]["studio-pro"] = json!({"id": id, "tier": "provisional"});
        }
    });
    assert_every_shipped_compiles(copy.path());
}

/// A copy of the shipped adapters, each edited by `edit`.
fn shipped_adapters_with(edit: impl Fn(&mut Value)) -> tempfile::TempDir {
    let copy = tempfile::tempdir().unwrap();
    for entry in std::fs::read_dir(shipped().join("adapters")).unwrap() {
        let path = entry.unwrap().path();
        let mut adapter: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        edit(&mut adapter);
        std::fs::write(
            copy.path().join(path.file_name().unwrap()),
            adapter.to_string(),
        )
        .unwrap();
    }
    copy
}

/// Every shipped bundle and recipe, at least the seventeen shipped today,
/// compiles against `adapters`.
fn assert_every_shipped_compiles(adapters: &Path) {
    let compiled = shipped_compiles(adapters);
    assert!(compiled.len() >= 17, "{compiled:?}");
    for (path, result) in compiled {
        assert_eq!(result, Ok(()), "{path}");
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
