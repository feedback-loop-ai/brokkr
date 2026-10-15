//! The folder's shelf of named recipes, `.brokkr/recipes/<name>/` (#696):
//! the default of `recipes list`, `recipes show` and `run --recipe` where
//! a folder is found, with an explicit flag still winning; a shelf that
//! never moves the folder bundle's digest; and doctor's line per shelf
//! recipe. Every repository is [`Repo`]'s, its folder planted by hand.

use serde_json::Value;

use crate::brokkr_folder::{completes, digest, started, Repo};

/// A folder repository whose shelf holds the fake-driver recipe `x`.
fn shelved() -> Repo {
    let repo = Repo::new();
    repo.folder();
    repo.bundle(".brokkr/recipes/x", "x", completes());
    repo
}

/// With no shelf, `recipes list` lists nothing and says nothing; a shelf
/// that is not a directory, and an absent `--dir`, warn as before.
#[test]
fn an_absent_shelf_lists_nothing() {
    let repo = Repo::new();
    repo.folder();
    let ran = repo.brokkr("src", &["recipes", "list"]);
    assert_eq!((ran.code, &*ran.stdout, &*ran.stderr), (Some(0), "", ""));
    let ran = repo.brokkr("", &["recipes", "list", "--dir", "missing"]);
    let absent = "warning: recipes dir missing: No such file or directory (os error 2)\n";
    assert_eq!((ran.code, &*ran.stdout), (Some(0), absent));
    let shelf = repo.write(".brokkr/recipes", "a file\n");
    let ran = repo.brokkr("", &["recipes", "list"]);
    let unlisted = format!(
        "warning: recipes dir {}: Not a directory (os error 20)\n",
        shelf.display()
    );
    assert_eq!((ran.code, ran.stdout), (Some(0), unlisted));
}

/// `recipes list` and `recipes show` read the shelf with no `--dir`, from
/// the root or a subdirectory, and a `--dir` still names its own library.
#[test]
fn list_and_show_read_the_shelf() {
    let repo = shelved();
    let at = repo.path(".brokkr/recipes/x");
    let compiled = digest(&repo, ".brokkr/recipes/x");
    let line = format!(
        "x\t{}\t3 phases\timplement, review\t\t\t{}\n",
        &compiled[..12],
        at.display()
    );
    for from in ["", "src/deep"] {
        let ran = repo.brokkr(from, &["recipes", "list"]);
        assert_eq!((ran.code, &ran.stdout), (Some(0), &line), "{}", ran.stderr);
        let ran = repo.brokkr(from, &["recipes", "show", "x"]);
        let shown: Value = serde_json::from_str(&ran.stdout).unwrap();
        assert_eq!(
            (ran.code, &shown["digest"]),
            (Some(0), &Value::from(&*compiled))
        );
    }
    repo.bundle("own/y", "y", completes());
    let ran = repo.brokkr("", &["recipes", "list", "--dir", "own"]);
    let own = format!(
        "y\t{}\t3 phases\timplement, review\t\t\town/y\n",
        &digest(&repo, "own/y")[..12]
    );
    assert_eq!((ran.code, ran.stdout), (Some(0), own));
    let ran = repo.brokkr("", &["recipes", "show", "x", "--dir", "own"]);
    let absent = "error: recipe 'x' not found under own; install it with `brokkr recipes add \
                  <source> --name x`";
    assert_eq!(
        (ran.code, ran.stderr.lines().next()),
        (Some(1), Some(absent))
    );
}

/// `run --recipe x` delivers under the shelf's `x` with no
/// `--recipes-dir`, and an explicit `--recipes-dir` wins.
#[test]
fn run_recipe_reads_the_shelf_and_a_flag_wins() {
    let repo = shelved();
    let run = ["run", "--recipe", "x", "--feature", "f", "--no-view"];
    let ran = repo.brokkr("src", &run);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    started(&ran);
    assert_eq!((repo.seated("x"), repo.seated("folder")), (true, false));
    repo.bundle("own/x", "own", completes());
    let named = [&run[..], &["--recipes-dir", "../own"]].concat();
    let ran = repo.brokkr("src", &named);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert!(repo.seated("own"));
}

/// Installing or editing a shelf recipe never moves the folder bundle's
/// digest (#696), as its `state/` never does (`state_never_moves_the_bundle`).
#[test]
fn the_shelf_never_moves_the_bundle() {
    let repo = Repo::new();
    repo.folder();
    let before = digest(&repo, ".brokkr");
    repo.bundle(".brokkr/recipes/x", "x", completes());
    assert_eq!(digest(&repo, ".brokkr"), before);
    repo.write(".brokkr/recipes/x/roles/role.md", "# edited\n");
    assert_eq!(digest(&repo, ".brokkr"), before);
    repo.write(".brokkr/roles/other.md", "a pinned file\n");
    assert_ne!(digest(&repo, ".brokkr"), before);
}

/// A shelf that is a link fails the base's real-directory rule: a folder
/// bundle composes only from a shelf that is a directory of its own.
#[test]
fn a_linked_shelf_composes_no_base() {
    let repo = Repo::new();
    repo.folder();
    repo.bundle("../outside/shared", "shared", completes());
    let outside = repo.root.parent().unwrap().join("outside");
    std::os::unix::fs::symlink(&outside, repo.path(".brokkr/recipes")).unwrap();
    let path = repo.path(".brokkr/bundle.json");
    let mut bundle: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    bundle["extends"] = serde_json::json!("shared");
    std::fs::write(&path, bundle.to_string()).unwrap();
    let ran = repo.brokkr("", &["compile", "--bundle", ".brokkr"]);
    let (root, shelf) = (repo.root.display(), outside.join("shared"));
    let refusal = format!(
        "error: bundle: {root}/.brokkr/bundle.json: extends 'shared', which resolves outside the \
         library to {}; a base is composed from and mounted into confined seats, so it must be \
         a real directory in {root}/.brokkr/recipes\n",
        shelf.display()
    );
    assert_eq!((ran.code, ran.stderr), (Some(1), refusal));
}

/// Doctor prints a line per shelf recipe after the folder line, and a
/// broken one warns without changing the report's exit (#696).
#[test]
fn doctor_reads_the_shelf() {
    let repo = shelved();
    let doctor = |repo: &Repo| repo.brokkr("", &["doctor"]);
    let healthy = doctor(&repo).code;
    let digest = &digest(&repo, ".brokkr/recipes/x")[..12];
    repo.write(".brokkr/recipes/broken/bundle.json", "not json");
    let ran = doctor(&repo);
    let (shelf, root) = (repo.path(".brokkr/recipes"), repo.root.display());
    let lines: Vec<&str> = ran.stdout.lines().take(3).collect();
    let broken = format!(
        "warn     recipe broken: {at}/broken does not compile (#696): bundle: \
         {at}/broken/bundle.json: expected ident at line 1 column 2",
        at = shelf.display()
    );
    let expected = [
        format!("ok       folder: {root}/.brokkr (found from {root})"),
        broken,
        format!("ok       recipe x: {digest} at {}/x", shelf.display()),
    ];
    assert_eq!(
        (ran.code, lines),
        (healthy, expected.iter().map(String::as_str).collect())
    );
}
