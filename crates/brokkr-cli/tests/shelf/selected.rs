//! The project shelf `--recipes-dir` or `--dir` selects, beyond `run` and
//! `recipes show` (#697 ruling 3): `queue add` refuses one other than the
//! workspace's own, since a queued launch holds none, and an entry queued
//! across the workspace's own is held by each refusal of a name on both
//! shelves or a project layer naming a personal one; `recipes list`
//! compiles each recipe across the selected shelf, as `recipes show` does.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::{folder_names_mine, ledger, project_names_mine, shadowed};
use crate::brokkr_folder::{completes, Repo};

/// A folder repository whose default shelf holds nothing, `own/fast`
/// beside it, and the personal `mine` extending `fast`.
fn selected() -> Repo {
    let repo = Repo::new();
    repo.folder();
    repo.bundle("own/fast", "fast", completes());
    let derived = json!({"name": "mine", "extends": "fast"}).to_string();
    repo.write(mine(&repo).join("bundle.json").to_str().unwrap(), &derived);
    repo
}

/// The personal recipe `mine`.
fn mine(repo: &Repo) -> std::path::PathBuf {
    repo.config().join("brokkr/recipes/mine")
}

/// `queue add` of `mine`, with `flags` beside it.
fn queue_mine<'a>(flags: &[&'a str]) -> Vec<&'a str> {
    let add = ["queue", "add", "--recipe", "mine", "--feature", "f"];
    [&add[..], &["--reason", "r"], flags].concat()
}

/// What `queue add` refuses the selected project shelf `shelf` with, the
/// workspace's own being `own` (#697 ruling 3).
fn unqueueable(shelf: &Path, own: &Path) -> String {
    format!(
        "error: --recipes-dir selects the project shelf {}, and a queued launch composes only \
         across its workspace's own, {} (#697): queued-launch/v1 holds no selected shelf, so the \
         entry would be admitted and started across the workspace's shelf instead; queue it \
         without --recipes-dir, or start it with `brokkr run`; nothing was queued\n",
        shelf.display(),
        own.display()
    )
}

/// `queue add` refuses a project shelf `--recipes-dir` selects other than
/// the workspace's own, from the root and from a subdirectory, with `fast`
/// on it and on the personal shelf, where `run` refuses the name on both:
/// queued-launch/v1 holds no selected shelf, so the entry would be admitted
/// across the workspace's own, which holds no `fast`. Nothing is queued
/// and no file changes. The workspace's own, named by the flag, queues.
#[test]
fn queue_add_refuses_a_selected_project_shelf() {
    let repo = selected();
    let personal = repo.config().join("brokkr/recipes/fast");
    repo.bundle(personal.to_str().unwrap(), "pfast", completes());
    let before = repo.files();
    let own = repo.path(".brokkr/recipes");
    for (from, shelf, named) in [("", "own", "own"), ("src", "../own", "src/../own")] {
        let ran = repo.brokkr(from, &queue_mine(&["--recipes-dir", shelf]));
        let refused = unqueueable(&repo.path(named), &own);
        assert_eq!((ran.code, ran.stderr), (Some(1), refused), "{from}");
    }
    let journal = repo.path(".brokkr/state/forge.db");
    assert_eq!((repo.files(), journal.exists()), (before, false));

    let ran = repo.brokkr("", &queue_mine(&["--recipes-dir", ".brokkr/recipes"]));
    assert_eq!((ran.code, &*ran.stderr), (Some(0), "queued entry 1\n"));
    let rows: Value = serde_json::from_str(&ledger(&repo).1).unwrap();
    assert_eq!(rows.as_array().map(Vec::len), Some(1));
}

/// An entry queued across the workspace's own shelf is held by admission
/// with each refusal its compile gives (#697 rulings 2 and 3): a personal
/// recipe's base on both shelves, a project recipe naming a personal one,
/// and the folder's bundle naming one, each in the words `run` refuses it
/// with.
#[test]
fn an_entry_queued_across_the_workspaces_shelf_is_held_by_each_refusal() {
    let repo = selected();
    let config = hosted(&repo, &repo.config());
    repo.bundle(".brokkr/recipes/fast", "fast", completes());
    repo.bundle(
        config.join("recipes/fast").to_str().unwrap(),
        "pfast",
        completes(),
    );
    let extends = json!({"name": "team", "extends": "mine"}).to_string();
    let team = repo.write(".brokkr/recipes/team/bundle.json", &extends);
    let extends = json!({"name": "folder", "extends": "mine"}).to_string();
    repo.write(".brokkr/bundle.json", &extends);

    let (feature, reason) = (["--feature", "f"], ["--reason", "r"]);
    for recipe in [&["--recipe", "mine"][..], &["--recipe", "team"], &[]] {
        let add = [&["queue", "add"][..], recipe, &feature, &reason].concat();
        let ran = repo.brokkr("", &add);
        assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    }
    let both = shadowed(
        "fast",
        &repo.path(".brokkr/recipes/fast"),
        &config.join("recipes/fast"),
    );
    let verdicts = [
        held(&both),
        held(&project_names_mine(team.parent().unwrap(), &mine(&repo))),
        held(&folder_names_mine(&repo.path(".brokkr"), &mine(&repo))),
    ];
    let rows: Value = serde_json::from_str(&ledger(&repo).1).unwrap();
    let rows = rows.as_array().unwrap().iter();
    let judged: Vec<&Value> = rows.map(|row| &row["admission"]).collect();
    assert_eq!(judged, verdicts.iter().collect::<Vec<_>>());
}

/// The host every admission here measures against, written under the
/// configuration directory `config`: its `brokkr` directory.
fn hosted(repo: &Repo, config: &Path) -> PathBuf {
    let host = json!({"schema": "forge.host/v1", "providers": {},
                      "floors": [{"place": "seat-scratch", "floor_bytes": 1}],
                      "boxed_builds": {"ceiling": 1}});
    let config = config.join("brokkr");
    repo.write(
        config.join("host.json").to_str().unwrap(),
        &host.to_string(),
    );
    config
}

/// Admission's hold of an entry whose compile `refused`, as `queue list
/// --json` prints it, in the compile's words.
fn held(refused: &str) -> Value {
    let refused = refused.trim_start_matches("error: bundle: ").trim_end();
    unmeasured(&format!("bundle: {refused}"))
}

/// Admission's hold of an entry it cannot measure, because `why`.
fn unmeasured(why: &str) -> Value {
    let says = format!("what it seats cannot be measured: {why}");
    json!({"standing": "held", "reasons": [{"kind": "unmeasured", "says": says}]})
}

/// An entry queued under one XDG_CONFIG_HOME, its recipe on that
/// environment's personal shelf, is held by admission under another (#697
/// rulings 2 and 5), naming the entry's shelf and the two this process
/// stands: this process places another personal shelf, so the entry's would
/// be read as a library, its layers given no origin and `fast`, on both
/// shelves, read from beside `mine`. Under the environment that queued it,
/// the same entry is held by the name on both shelves.
#[test]
fn an_entry_queued_under_another_environment_is_held_by_its_unplaced_shelf() {
    let repo = selected();
    let queuing = hosted(&repo, &repo.config()).join("recipes");
    let other = repo.config().with_file_name("config-b");
    hosted(&repo, &other);
    repo.bundle(".brokkr/recipes/fast", "fast", completes());
    repo.bundle(queuing.join("fast").to_str().unwrap(), "pfast", completes());
    let ran = repo.brokkr("", &queue_mine(&[]));
    assert_eq!((ran.code, &*ran.stderr), (Some(0), "queued entry 1\n"));

    let own = repo.path(".brokkr/recipes");
    let both = shadowed("fast", &own.join("fast"), &queuing.join("fast"));
    let unplaced = format!(
        "queued recipe 'mine' stands on the shelf {}, which is neither its workspace's project \
         shelf, {}, nor the personal shelf this process places ({}) (#697): a queued launch \
         composes only across shelves its admitting process places, so the entry would compose \
         with no origin for that shelf's layers and past the refusal of a name on both shelves; \
         admit it under the XDG_CONFIG_HOME and HOME that queued it, or start it with `brokkr \
         run`",
        queuing.display(),
        own.display(),
        other.join("brokkr/recipes").display()
    );
    let listed = super::under(
        &repo,
        &[("XDG_CONFIG_HOME", Some(&other))],
        &["queue", "list", "--json"],
    );
    assert_eq!((listed.code, &*listed.stderr), (Some(0), ""));
    for (queue, hold) in [
        (ledger(&repo).1, held(&both)),
        (listed.stdout, unmeasured(&unplaced)),
    ] {
        let rows: Value = serde_json::from_str(&queue).unwrap();
        assert_eq!(rows[0]["admission"], hold);
    }
}

/// `recipes list --dir` compiles each recipe across the shelf it selects,
/// as `recipes show --dir` does (#697 ruling 3), from the root and from a
/// subdirectory: the personal `mine`, over `own/fast`, lists with the
/// digest `show` prints, never as a recipe whose base is missing.
#[test]
fn recipes_list_composes_across_the_shelf_it_selects() {
    let repo = selected();
    for (from, own) in [("", "own"), ("src", "../own")] {
        let show = repo.brokkr(from, &["recipes", "show", "mine", "--dir", own]);
        assert_eq!(show.code, Some(0), "{}", show.stderr);
        let shown: Value = serde_json::from_str(&show.stdout).unwrap();
        let list = repo.brokkr(from, &["recipes", "list", "--dir", own]);
        assert_eq!((list.code, &*list.stderr), (Some(0), ""));
        let line = list.stdout.lines().find(|line| line.contains("mine"));
        let fields: Vec<&str> = line.unwrap_or_default().split('\t').collect();
        let field = |at: usize| fields.get(at).copied();
        let digest = &shown["digest"].as_str().unwrap()[..12];
        let path = mine(&repo).display().to_string();
        let listed = [field(0), field(1), field(6), field(7)];
        let composed = [Some("mine"), Some(digest), Some("personal"), Some(&*path)];
        assert_eq!(listed, composed, "{from}: {}", list.stdout);
    }
}
