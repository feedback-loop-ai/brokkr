//! Decision 0087 ruling 8, slice 1 (#717): Brokkr's two portable recipes,
//! `fast` and `node`, install onto the shelf of a fresh folder made with
//! `git init` and `brokkr init .`, under each agent CLI `init` scaffolds
//! for, and the installed recipe compiles, hiring the folder's chains.
//!
//! Every child runs through `init_stacks`'s launcher with `HOME` and
//! `XDG_CONFIG_HOME` beside the repository, never the host's.

use std::path::Path;

use serde_json::Value;

use crate::brokkr_folder::repository;
use crate::init_stacks::brokkr_for;
use crate::workspace_root::workspace;

/// One file `recipes add` copies into `.brokkr/agents/`: a charter, by its
/// name under `charters/` and the agent that names it, or an agent.
type Copy = (Option<&'static str>, &'static str);

/// What `recipes add` writes for each portable recipe into a folder `init`
/// made, in the order it reports them: the recipe and every base it
/// extends onto the shelf, then each charter and each office they name
/// that the folder lacks (decision 0087 ruling 8).
const COPIED: [(&str, &[&str], &[Copy]); 2] = [
    (
        "fast",
        &["fast"],
        &[
            (Some("implementer-intakeless.md"), "fast-implementer"),
            (None, "fast-implementer"),
            (None, "fast-reviewer"),
        ],
    ),
    (
        "node",
        &["node", "fast"],
        &[
            (Some("implementer-intakeless.md"), "fast-implementer"),
            (Some("node-implementer.md"), "node-implementer"),
            (Some("node-reviewer.md"), "node-reviewer"),
            (None, "fast-implementer"),
            (None, "fast-reviewer"),
            (None, "node-implementer"),
            (None, "node-reviewer"),
        ],
    ),
];

/// `add`'s report, line for line, of `recipes` shelved and `copied` into
/// `folder`; each copy is the library's bytes.
fn assert_reported(stderr: &str, folder: &Path, recipes: &[&str], copied: &[Copy]) {
    let mut lines = stderr.lines();
    let shelf = folder.join("recipes");
    for recipe in recipes {
        let line = lines.next().unwrap_or_default();
        let at = format!(") at {}", shelf.join(recipe).display());
        let named = format!("added recipe '{recipe}' (");
        assert!(line.starts_with(&named) && line.ends_with(&at), "{stderr}");
    }
    for (charter, agent) in copied {
        let (named, path) = match charter {
            Some(name) => (
                format!("charter charters/{name} of agent '{agent}'"),
                format!("charters/{name}"),
            ),
            None => (format!("agent '{agent}'"), format!("{agent}.json")),
        };
        let at = folder.join("agents").join(&path);
        let expected = format!("added {named} at {}", at.display());
        assert_eq!(lines.next(), Some(expected.as_str()), "{stderr}");
        assert_eq!(
            std::fs::read(&at).unwrap(),
            std::fs::read(workspace().join("agents").join(&path)).unwrap(),
            "{path}"
        );
    }
    assert_eq!(lines.next(), None, "{stderr}");
}

/// The provider each model seat hires from in a folder scaffolded for
/// `cli`: its own, except that a dsh folder's review gate stays on claude.
fn providers(cli: &str) -> [(&'static str, &str); 2] {
    let review = if cli == "dsh" { "claude" } else { cli };
    [("implement", cli), ("review", review)]
}

fn json_at(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// In a fresh folder scaffolded for `cli`, `recipes add` of each portable
/// recipe exits 0 and reports exactly the files it copies, and the
/// installed recipe compiles to the folder's base chains: its model seats
/// hire from the CLI `init` found.
fn installs_and_compiles(cli: &str) {
    for (recipe, recipes, copied) in COPIED {
        let (repo, aside) = (repository(), tempfile::tempdir().unwrap());
        let brokkr = |args: &[&str]| brokkr_for(cli, args, repo.path(), aside.path());
        let (code, _, stderr) = brokkr(&["init", "."]);
        assert_eq!(code, Some(0), "{cli}: {stderr}");
        let source = workspace().join("recipes").join(recipe);
        let (code, _, stderr) =
            brokkr(&["recipes", "add", source.to_str().unwrap(), "--name", recipe]);
        assert_eq!(code, Some(0), "{cli} {recipe}: {stderr}");
        let folder = repo.path().join(".brokkr");
        assert_reported(&stderr, &folder, recipes, copied);
        let agents = folder.join("agents");
        let installed = format!(".brokkr/recipes/{recipe}");
        let (code, stdout, stderr) = brokkr(&["compile", "--bundle", &installed]);
        assert_eq!(code, Some(0), "{cli} {recipe}: {stderr}");
        let compiled: Value = serde_json::from_str(&stdout).unwrap();
        for (seat, provider) in providers(cli) {
            let base = if seat == "review" {
                "reviewer"
            } else {
                "implementer"
            };
            let record = &compiled["manifest"]["agents"][seat];
            assert_eq!(record["provider"], provider, "{cli} {recipe} {seat}");
            assert_eq!(
                record["chain"],
                json_at(&agents.join(format!("{base}.json")))["models"],
                "{cli} {recipe} {seat}"
            );
        }
    }
}

#[test]
fn fast_and_node_install_and_compile_in_a_claude_folder() {
    installs_and_compiles("claude");
}

#[test]
fn fast_and_node_install_and_compile_in_a_codex_folder() {
    installs_and_compiles("codex");
}

#[test]
fn fast_and_node_install_and_compile_in_a_dsh_folder() {
    installs_and_compiles("dsh");
}
