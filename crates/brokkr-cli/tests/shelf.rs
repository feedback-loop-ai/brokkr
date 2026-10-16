//! The folder's shelf of named recipes, `.brokkr/recipes/<name>/` (#696):
//! the default of `recipes list`, `recipes show` and `run --recipe` where
//! a folder is found, with an explicit flag still winning; a shelf that
//! never moves the folder bundle's digest; and doctor's line per shelf
//! recipe. The personal shelf beside `host.json` (#697): `recipes add
//! --personal`, `--recipe` reading the project's shelf and then it, a name
//! on both refused, and the origin `recipes list` and doctor show. Every
//! repository is [`Repo`]'s, its folder planted by hand, and its
//! configuration directory [`Repo::config`].

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::brokkr_folder::{completes, digest, started, Ran, Repo};

/// A folder repository whose shelf holds the fake-driver recipe `x`.
fn shelved() -> Repo {
    let repo = Repo::new();
    repo.folder();
    repo.bundle(".brokkr/recipes/x", "x", completes());
    repo
}

/// With no shelf, `recipes list` lists nothing and says nothing; a shelf
/// that is not a directory, a link to nothing, and an absent `--dir`, warn
/// as before.
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
    std::fs::remove_file(&shelf).unwrap();
    std::os::unix::fs::symlink("removed-shelf", &shelf).unwrap();
    let ran = repo.brokkr("", &["recipes", "list"]);
    let dangling = format!(
        "warning: recipes dir {}: No such file or directory (os error 2)\n",
        shelf.display()
    );
    assert_eq!((ran.code, ran.stdout), (Some(0), dangling));
}

/// `recipes list` and `recipes show` read the shelf with no `--dir`, from
/// the root or a subdirectory, and a `--dir` still names its own library.
/// With neither `XDG_CONFIG_HOME` nor `HOME` set, `recipes list` lists the
/// project's shelf alone (#697).
#[test]
fn list_and_show_read_the_shelf() {
    let repo = shelved();
    let at = repo.path(".brokkr/recipes/x");
    let compiled = digest(&repo, ".brokkr/recipes/x");
    let line = format!(
        "x\t{}\t3 phases\timplement, review\t\t\tproject\t{}\n",
        &compiled[..12],
        at.display()
    );
    // Where this machine places no personal shelf, the project's alone.
    let mut unplaced = repo.command("");
    unplaced.args(["recipes", "list"]);
    unplaced.env_remove("XDG_CONFIG_HOME").env_remove("HOME");
    let ran = Ran::of(unplaced.output().unwrap());
    assert_eq!((ran.code, &ran.stdout, &*ran.stderr), (Some(0), &line, ""));
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
        "y\t{}\t3 phases\timplement, review\t\t\tproject\town/y\n",
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

/// A folder repository made by `brokkr init .`, and a recipe library
/// beside it holding `fast` ([`fast_in`]). The recipe's path is returned.
fn library_of_fast() -> (Repo, PathBuf) {
    let repo = initialised();
    let fast = fast_in(&repo, &repo.root.parent().unwrap().join("library"));
    (repo, fast)
}

/// A folder repository made by `brokkr init .`.
fn initialised() -> Repo {
    let repo = Repo::new();
    let init = repo.init_with_only(&["claude"]);
    assert_eq!(init.code, Some(0), "{}", init.stderr);
    repo
}

/// `fast` at `<library>/recipes/fast`, a recipe library's or a folder's
/// shelf, with its agents in `<library>/agents`: `repo`'s folder bundle, its
/// implementer and reviewer hired as `fast-implementer`, with a charter of
/// its own, and `fast-reviewer`, each extending the agent `init` wrote.
/// The recipe's path is returned.
fn fast_in(repo: &Repo, library: &Path) -> PathBuf {
    let fast = library.join("recipes/fast");
    std::fs::create_dir_all(&fast).unwrap();
    std::fs::copy(repo.path(".brokkr/policy.json"), fast.join("policy.json")).unwrap();
    crate::machine_proof::copy_dir(&repo.path(".brokkr/scripts"), &fast.join("scripts"));
    let mut bundle = json_at(&repo.path(".brokkr/bundle.json"));
    bundle["seats"]["implement"]["agent"] = json!("fast-implementer");
    bundle["seats"]["review"]["agent"] = json!("fast-reviewer");
    std::fs::write(fast.join("bundle.json"), bundle.to_string()).unwrap();
    let charter = "charters/fast-implementer.md";
    let overlay = |base: &str| {
        let written = json_at(&repo.path(&format!(".brokkr/agents/{base}.json")));
        json!({"extends": base, "models": written["models"], "efforts": written["efforts"]})
    };
    let mut implementer = overlay("implementer");
    implementer["charter"] = json!(charter);
    let reviewer = overlay("reviewer");
    let agents = [
        ("fast-implementer.json", implementer.to_string()),
        (charter, "# Fast implementer\n".to_string()),
        ("fast-reviewer.json", reviewer.to_string()),
    ];
    for (file, text) in agents {
        let at = library.join("agents").join(file);
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        std::fs::write(at, text).unwrap();
    }
    fast
}

fn json_at(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// The files of `fast`'s agent library an install copies into a folder
/// `init` made.
const COPIED: [&str; 3] = [
    "charters/fast-implementer.md",
    "fast-implementer.json",
    "fast-reviewer.json",
];

/// The folder bundle's compiled digest and the files its manifest pins.
fn pinned(repo: &Repo) -> (Value, serde_json::Map<String, Value>) {
    let ran = repo.brokkr("", &["compile", "--bundle", ".brokkr"]);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let mut compiled: Value = serde_json::from_str(&ran.stdout).unwrap();
    let files = compiled["manifest"]["files"].as_object().unwrap().clone();
    (compiled["digest"].take(), files)
}

/// Installing `fast` from a library copies the agents it names that the
/// folder lacks, and the charter one names, into `.brokkr/agents/`, and the
/// recipe compiles (#696 ruling 5); each copy is the library's bytes. The
/// folder bundle pins its `agents/`, so the copies move its digest by
/// exactly the files copied, where the shelf does not (ruling 3). A second
/// install finds them present and identical, writes none again, and leaves
/// the digest where it was.
#[test]
fn add_copies_the_agents_a_recipe_names() {
    let (repo, fast) = library_of_fast();
    let add = |name: &str| {
        repo.brokkr(
            "",
            &["recipes", "add", fast.to_str().unwrap(), "--name", name],
        )
    };
    let (before, mut pinned_before) = pinned(&repo);
    let ran = add("fast");
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let agents = repo.path(".brokkr/agents");
    let copied = COPIED;
    for file in copied {
        let source = fast.join("../../agents").join(file);
        let copy = std::fs::read(agents.join(file)).unwrap();
        assert_eq!(copy, std::fs::read(source).unwrap(), "{file}");
    }
    let listed = repo.brokkr("", &["recipes", "list"]);
    let digest = &listed.stdout[5..17];
    let at = agents.display();
    let expected = format!(
        "added recipe 'fast' ({digest}) at {}/fast\nadded charter charters/fast-implementer.md \
         of agent 'fast-implementer' at {at}/charters/fast-implementer.md\nadded agent \
         'fast-implementer' at {at}/fast-implementer.json\nadded agent 'fast-reviewer' at \
         {at}/fast-reviewer.json\n",
        repo.path(".brokkr/recipes").display()
    );
    assert_eq!(ran.stderr, expected);
    let (moved, pinned_after) = pinned(&repo);
    assert_ne!(moved, before);
    for file in copied {
        let pin = pinned_after[&format!("agents/{file}")].clone();
        assert_eq!(pinned_before.insert(format!("agents/{file}"), pin), None);
    }
    assert_eq!(pinned_after, pinned_before);
    let before = repo.files();
    let ran = add("again");
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert!(
        ran.stderr.starts_with("added recipe 'again' ("),
        "{}",
        ran.stderr
    );
    assert_eq!(ran.stderr.lines().count(), 1, "{}", ran.stderr);
    let new: Vec<PathBuf> = repo
        .files()
        .into_keys()
        .filter(|f| !before.contains_key(f))
        .collect();
    assert!(
        new.iter()
            .all(|file| file.starts_with(".brokkr/recipes/again")),
        "{new:?}"
    );
    assert_eq!(pinned(&repo).0, moved);
}

/// One shelf recipe has one identity (#696): under the map `init` writes,
/// `recipes add`, `recipes list` and doctor's shelf line name the digest
/// `recipes show` prints, compiled in the folder's realm as a run is.
#[test]
fn every_recipe_verb_names_the_digest_show_prints() {
    let (repo, fast) = library_of_fast();
    let add = ["recipes", "add", fast.to_str().unwrap(), "--name", "fast"];
    let added = repo.brokkr("", &add);
    assert_eq!(added.code, Some(0), "{}", added.stderr);
    let shown = repo.brokkr("", &["recipes", "show", "fast"]);
    let compiled: Value = serde_json::from_str(&shown.stdout).unwrap();
    let realm = &compiled["manifest"]["capabilities"]["realm"];
    assert_eq!((shown.code, realm), (Some(0), &json!("starter")));
    let digest = &compiled["digest"].as_str().unwrap()[..12];
    let shelf = repo.path(".brokkr/recipes");
    let first = |text: &str| text.lines().next().unwrap().to_string();
    let added_line = format!("added recipe 'fast' ({digest}) at {}/fast", shelf.display());
    assert_eq!(first(&added.stderr), added_line);
    let listed = repo.brokkr("", &["recipes", "list"]);
    assert_eq!(listed.stdout.split('\t').nth(1), Some(digest));
    let doctor = repo.brokkr("", &["doctor"]);
    let line = format!("ok       recipe fast: {digest} at {}/fast", shelf.display());
    assert!(
        doctor.stdout.lines().any(|l| l == line),
        "{}",
        doctor.stdout
    );
}

/// A differing `fast-reviewer` already in the folder refuses the install by
/// name, and nothing is written: the folder is byte for byte as it was. An
/// install whose compile refuses removes the agents and the charter it
/// copied with the recipe.
#[test]
fn add_refuses_a_differing_agent_and_removes_a_refused_compiles_agents() {
    let (repo, fast) = library_of_fast();
    let add = ["recipes", "add", fast.to_str().unwrap(), "--name", "fast"];
    let planted = repo.write(".brokkr/agents/fast-reviewer.json", "{}\n");
    let before = repo.files();
    let ran = repo.brokkr("", &add);
    let differs = format!(
        "error: recipes add replaces no agent definition or charter, and the source's differ from \
         what stands at {} (#696): reconcile or remove each; nothing was written\n",
        planted.display()
    );
    assert_eq!((ran.code, ran.stderr), (Some(1), differs));
    assert_eq!(repo.files(), before);
    assert!(!repo.path(".brokkr/recipes").exists());

    std::fs::remove_file(&planted).unwrap();
    let reviewer = fast.join("../../agents/fast-reviewer.json");
    std::fs::write(reviewer, json!({"extends": "absent"}).to_string()).unwrap();
    let before = repo.files();
    let ran = repo.brokkr("", &add);
    let refused = "error: recipe 'fast' does not compile (removed): ";
    assert_eq!(ran.code, Some(1), "{}", ran.stderr);
    assert!(ran.stderr.starts_with(refused), "{}", ran.stderr);
    assert_eq!(repo.files(), before);
    assert!(!repo.path(".brokkr/recipes").exists());
    assert!(!repo
        .path(".brokkr/agents/charters/fast-implementer.md")
        .exists());
}

/// A recipe on another folder repository's shelf brings the agents its
/// folder's library holds, `.brokkr/agents`, local or cloned, never a
/// decoy `agents/` at the clone's root, and a recipe at a library clone's
/// `--path`, or at a clone's root, brings the clone's `agents/`: each
/// source's library as ruling 5 reads it (#696). Each source's charter
/// differs, so the copy names the library it was read from.
#[test]
fn add_copies_the_agents_of_a_shelf_and_of_a_clone() {
    let repo = initialised();
    let shelved = Repo::new();
    let cloned = repo.root.with_file_name("cloned");
    for home in [shelved.path(".brokkr"), cloned.clone()] {
        let charter = home.join("agents").join(COPIED[0]);
        fast_in(&repo, &home);
        std::fs::write(charter, home.display().to_string()).unwrap();
    }
    let decoy = shelved.path("agents");
    crate::machine_proof::copy_dir(&shelved.path(".brokkr/agents"), &decoy);
    std::fs::write(decoy.join(COPIED[0]), "# ROOT DECOY\n").unwrap();
    let rooted = repo.root.with_file_name("rooted");
    crate::machine_proof::copy_dir(&cloned.join("recipes/fast"), &rooted);
    crate::machine_proof::copy_dir(&cloned.join("agents"), &rooted.join("agents"));
    std::fs::write(rooted.join("agents").join(COPIED[0]), "# CLONE ROOT\n").unwrap();
    let rooted_url = crate::recipes::committed(&rooted);
    let url = crate::recipes::committed(&cloned);
    let shelf_url = crate::recipes::committed(&shelved.root);
    let fast = shelved.path(".brokkr/recipes/fast");
    let at_shelf = ".brokkr/recipes/fast";
    let sources = [
        (vec![fast.to_str().unwrap()], shelved.path(".brokkr/agents")),
        (vec![&*url, "--path", "recipes/fast"], cloned.join("agents")),
        (
            vec![&*shelf_url, "--path", at_shelf],
            shelved.path(".brokkr/agents"),
        ),
        (vec![&*rooted_url], rooted.join("agents")),
    ];
    let agents = repo.path(".brokkr/agents");
    for (source, library) in sources {
        let add = [&["recipes", "add"], &source[..], &["--name", "fast"]].concat();
        let ran = repo.brokkr("", &add);
        assert_eq!(ran.code, Some(0), "{}", ran.stderr);
        for file in COPIED {
            let copy = std::fs::read(agents.join(file)).unwrap();
            assert_eq!(copy, std::fs::read(library.join(file)).unwrap(), "{file}");
            std::fs::remove_file(agents.join(file)).unwrap();
        }
        std::fs::remove_dir_all(repo.path(".brokkr/recipes")).unwrap();
    }
}

/// What `recipes add` prints when it refuses `source`, a repository's
/// folder bundle, as a recipe (#696), its one pin.
fn folder_bundle(source: &Path) -> String {
    let source = source.display();
    format!(
        "error: {source} is a repository's .brokkr folder, its default bundle, not a recipe: name \
         a recipe on its shelf ({source}/recipes/<name>) or in a library as the source (#696); \
         nothing was written\n"
    )
}

/// A folder bundle is no recipe source (#696): one that extends `base`
/// from its shelf compiles where it stands, and `recipes add` of it from a
/// folder `init` made refuses by name, the destination's tree as it was:
/// spelled, with a decoy `base` beside the folder, and through a link.
#[test]
fn add_refuses_a_folder_bundle_as_a_source() {
    let repo = initialised();
    let source = Repo::new();
    source.bundle(".brokkr/recipes/base", "base", completes());
    let custom = json!({"name": "custom", "extends": "base"});
    source.write(".brokkr/bundle.json", &custom.to_string());
    let compiled = source.brokkr("", &["compile", "--bundle", ".brokkr"]);
    assert_eq!(compiled.code, Some(0), "{}", compiled.stderr);
    let folder = source.path(".brokkr");
    let linked = source.root.with_file_name("linked");
    std::os::unix::fs::symlink(&folder, &linked).unwrap();
    let tree = |repo: &Repo| (repo.files(), repo.path(".brokkr/recipes").exists());
    let before = tree(&repo);
    let refused = |from: &Path| {
        let from = from.to_str().unwrap();
        let ran = repo.brokkr("", &["recipes", "add", from, "--name", "custom"]);
        assert_eq!((ran.code, ran.stderr), (Some(1), folder_bundle(&folder)));
        assert_eq!(tree(&repo), before, "{from}");
    };
    refused(&folder);
    source.bundle("base", "decoy", completes());
    refused(&folder);
    refused(&linked);
}

/// A definition naming a charter its library does not hold refuses by
/// name with nothing written, whether the folder holds a file there, the
/// reviewer charter `init` wrote, its bytes kept, or not: the copy would
/// bind to whatever stands there (#696).
#[test]
fn add_refuses_a_charter_its_library_lacks() {
    let (repo, fast) = library_of_fast();
    let agents = fast.join("../../agents").canonicalize().unwrap();
    let definition = agents.join("fast-implementer.json");
    let mut implementer = json_at(&definition);
    implementer["charter"] = json!("charters/reviewer.md");
    std::fs::write(&definition, implementer.to_string()).unwrap();
    let refusal = format!(
        "error: {} names charter 'charters/reviewer.md', which its library {} does not hold, so \
         the copy would bind to whatever this workspace holds there (#696); nothing was written\n",
        definition.display(),
        agents.display()
    );
    let held = repo.path(".brokkr/agents/charters/reviewer.md");
    let add = ["recipes", "add", fast.to_str().unwrap(), "--name", "fast"];
    for holds in [true, false] {
        assert_eq!(held.is_file(), holds);
        let before = repo.files();
        let ran = repo.brokkr("", &add);
        assert_eq!(
            (ran.code, ran.stderr),
            (Some(1), refusal.clone()),
            "{holds}"
        );
        assert_eq!(repo.files(), before);
        assert!(!repo.path(".brokkr/recipes").exists());
        if holds {
            std::fs::remove_file(&held).unwrap();
        }
    }
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

/// `recipes add --personal <source> --name mine` from `repo`'s root, each
/// variable in `env` set to its value, or unset where it has none.
fn add_personal(repo: &Repo, source: &Path, env: &[(&str, Option<&Path>)]) -> Ran {
    let mut add = repo.command("");
    add.args(["recipes", "add", "--personal", source.to_str().unwrap()]);
    add.args(["--name", "mine"]);
    for (variable, value) in env {
        match value {
            Some(value) => add.env(variable, value),
            None => add.env_remove(variable),
        };
    }
    Ran::of(add.output().unwrap())
}

/// `recipes add --personal` installs beside `host.json` (#697 rulings 1
/// and 6): under `$XDG_CONFIG_HOME/brokkr/recipes`, else
/// `$HOME/.config/brokkr/recipes`, and refuses as `host.json`'s lookup does
/// where neither names an absolute directory, making nothing. No file of
/// the repository changes, and the folder bundle's digest stays.
#[test]
fn add_personal_installs_beside_the_host_file() {
    let repo = shelved();
    repo.bundle("../mine-source", "mine", completes());
    let source = repo.root.with_file_name("mine-source");
    let before = (repo.files(), digest(&repo, ".brokkr"));
    let home = repo.config().with_file_name("home");
    let unset = ("XDG_CONFIG_HOME", None);
    let placed = [
        (vec![], repo.config().join("brokkr/recipes/mine")),
        (
            vec![unset, ("HOME", Some(&*home))],
            home.join(".config/brokkr/recipes/mine"),
        ),
    ];
    for (env, installed) in placed {
        let ran = add_personal(&repo, &source, &env);
        let compiled = digest(&repo, installed.to_str().unwrap());
        let added = format!(
            "added recipe 'mine' ({}) at {}\n",
            &compiled[..12],
            installed.display()
        );
        assert_eq!((ran.code, ran.stderr), (Some(0), added));
        assert_eq!((repo.files(), digest(&repo, ".brokkr")), before);
    }
    let relative = [unset, ("HOME", Some(Path::new("relative")))];
    let ran = add_personal(&repo, &source, &relative);
    let unplaced = "error: no host configuration can be found: neither XDG_CONFIG_HOME nor HOME \
                    names an absolute directory\n";
    assert_eq!((ran.code, &*ran.stderr), (Some(1), unplaced));
    assert_eq!(repo.files(), before.0);
}

/// A personal install copies no agent or charter into the repository
/// (#697 ruling 4): `fast`, whose library holds the agents it hires, is
/// compiled against the folder's agents, which lack them, so it refuses,
/// the repository byte for byte as it was, and makes nothing on the
/// shelf, not even the configuration directory it would have stood in.
#[test]
fn a_personal_install_copies_no_agent_into_the_repository() {
    let (repo, fast) = library_of_fast();
    let before = repo.files();
    let ran = add_personal(&repo, &fast, &[]);
    let refused = "error: recipe 'mine' does not compile (removed): bundle: seat 'implement': \
                   agent 'fast-implementer' is not in the library; known agents: implementer, \
                   intake, reviewer\n";
    assert_eq!((ran.code, &*ran.stderr), (Some(1), refused));
    assert!(!repo.config().exists());
    assert_eq!(repo.files(), before);
}

/// `--recipe` reads the project's shelf, then the personal one (#697
/// ruling 6): a run delivers under either, `recipes list` shows both with
/// their origin, and doctor lists the personal recipe. Editing it changes
/// no file of the repository and not the folder bundle's digest, and a
/// resume of the run it delivered refuses the drift. A name on both
/// shelves refuses `run` and `show` alike, naming both paths (ruling 2).
#[test]
fn run_recipe_reads_the_project_shelf_then_the_personal_one() {
    let repo = shelved();
    let personal = repo.config().join("brokkr/recipes");
    let mine = personal.join("mine");
    repo.bundle(mine.to_str().unwrap(), "mine", completes());
    let run = |name: &str| {
        let ran = repo.brokkr(
            "src",
            &["run", "--recipe", name, "--feature", "f", "--no-view"],
        );
        assert_eq!(ran.code, Some(0), "{}", ran.stderr);
        started(&ran)
    };
    let delivered = run("mine");
    run("x");
    assert_eq!((repo.seated("mine"), repo.seated("x")), (true, true));
    let line = |name: &str, origin: &str, at: &Path| {
        let compiled = digest(&repo, at.to_str().unwrap());
        let (digest, at) = (&compiled[..12], at.display());
        format!("{name}\t{digest}\t3 phases\timplement, review\t\t\t{origin}\t{at}\n")
    };
    let project = repo.path(".brokkr/recipes/x");
    let listed = repo.brokkr("", &["recipes", "list"]);
    let both = line("x", "project", &project) + &line("mine", "personal", &mine);
    assert_eq!((listed.code, listed.stdout), (Some(0), both));
    let doctor = repo.brokkr("", &["doctor"]);
    let compiled = digest(&repo, mine.to_str().unwrap());
    let shown = format!(
        "ok       personal recipe mine: {} at {}",
        &compiled[..12],
        mine.display()
    );
    assert!(
        doctor.stdout.lines().any(|l| l == shown),
        "{}",
        doctor.stdout
    );

    let before = (repo.files(), digest(&repo, ".brokkr"));
    std::fs::write(mine.join("roles/role.md"), "# edited\n").unwrap();
    assert_eq!((repo.files(), digest(&repo, ".brokkr")), before);
    let resume = [
        "resume",
        "--run",
        &delivered,
        "--recipe",
        "mine",
        "--no-view",
    ];
    let ran = repo.brokkr("", &resume);
    let drift = format!("error: run '{delivered}' pins a different bundle: changed: roles/role.md");
    assert_eq!((ran.code, ran.first_line()), (Some(1), &*drift));

    repo.bundle(personal.join("x").to_str().unwrap(), "px", completes());
    let shadowed = format!(
        "error: recipe 'x' stands on both the project shelf, at {}, and the personal shelf, at \
         {} (#697): neither shelf wins, so rename or remove one of them\n",
        project.display(),
        personal.join("x").display()
    );
    let ran = repo.brokkr("", &["run", "--recipe", "x", "--feature", "f", "--no-view"]);
    assert_eq!((ran.code, ran.stderr), (Some(1), shadowed.clone()));
    let ran = repo.brokkr("", &["recipes", "show", "x"]);
    assert_eq!((ran.code, ran.stderr), (Some(1), shadowed));
}

/// `--recipe` names a recipe, never a path (#697 ruling 2): a name that is
/// not one plain recipe name, judged by its spelling, refuses on `run`,
/// `resume`, `rerun`, `queue add` and `recipes show` before either shelf is
/// read, with `x` on the project's shelf alone and with `x` on both, so
/// no spelling of `x` seats the project's recipe past the refusal of a
/// name on both shelves.
#[test]
fn a_recipe_name_that_is_a_path_refuses_on_every_verb() {
    let repo = shelved();
    let ran = repo.brokkr("", &["run", "--recipe", "x", "--feature", "f", "--no-view"]);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let delivered = started(&ran);
    let verbs = [
        vec!["run", "--feature", "f", "--no-view", "--recipe"],
        vec!["resume", "--run", &delivered, "--no-view", "--recipe"],
        vec!["rerun", "--run", &delivered, "--recipe"],
        vec![
            "queue",
            "add",
            "--feature",
            "f",
            "--reason",
            "r",
            "--recipe",
        ],
        vec!["recipes", "show"],
    ];
    let names = ["x/", "./x", "x/.", "../x", "a/b", "/abs/x", ""];
    let personal = repo.config().join("brokkr/recipes/x");
    for planted in [false, true] {
        if planted {
            repo.bundle(personal.to_str().unwrap(), "px", completes());
        }
        for (verb, name) in verbs.iter().flat_map(|v| names.map(|n| (v, n))) {
            let ran = repo.brokkr("", &[&verb[..], &[name]].concat());
            let refused = format!(
                "error: --recipe '{name}' is not one plain recipe name, so it would not name one \
                 recipe on a shelf (#697): name the recipe as it stands on its shelf, or give \
                 --bundle <dir> for a bundle elsewhere; nothing was run\n"
            );
            assert_eq!(
                (ran.code, ran.stderr),
                (Some(1), refused),
                "{verb:?} {name}"
            );
        }
    }
    assert!(!repo.seated("px"));
}

/// A shelf that cannot be read is never read as an empty one (#697 ruling
/// 2): the personal shelf or the project's, a link whose target was moved
/// away or a file, with `x` on both shelves until the break and with `x`
/// on the readable shelf alone, refuses `run`, `resume`, `rerun`, `queue
/// add` and `recipes show` of `x`, naming the shelf, before either recipe
/// is taken: no run starts, and neither fake driver keeps state. Nothing
/// at the personal shelf is an empty one, so `x` is the project's.
#[test]
fn a_shelf_that_cannot_be_read_refuses_on_every_verb() {
    let repo = shelved();
    let personal = repo.config().join("brokkr/recipes");
    std::fs::create_dir_all(personal.parent().unwrap()).unwrap();
    let ran = repo.brokkr("", &["run", "--recipe", "x", "--feature", "f", "--no-view"]);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let delivered = started(&ran);
    let shown = repo.brokkr("", &["recipes", "show", "x"]);
    assert_eq!(shown.code, Some(0), "{}", shown.stderr);
    let verbs = [
        vec!["recipes", "show", "x"],
        vec!["run", "--feature", "f", "--no-view", "--recipe", "x"],
        vec!["resume", "--run", &delivered, "--no-view", "--recipe", "x"],
        vec!["rerun", "--run", &delivered, "--recipe", "x"],
        vec![
            "queue",
            "add",
            "--feature",
            "f",
            "--reason",
            "r",
            "--recipe",
            "x",
        ],
    ];
    repo.bundle(personal.join("x").to_str().unwrap(), "px", completes());
    let state = |name: &str| {
        std::fs::read_dir(repo.config().with_file_name(name))
            .unwrap()
            .count()
    };
    let kept = (state("x.state"), state("px.state"));
    let project = repo.path(".brokkr/recipes");
    let dangle: fn(&Path) =
        |shelf| std::os::unix::fs::symlink(shelf.with_extension("target"), shelf).unwrap();
    let file: fn(&Path) = |shelf| std::fs::write(shelf, "").unwrap();
    let breaks = [
        (dangle, "No such file or directory (os error 2)"),
        (file, "not a directory"),
    ];
    for ((origin, shelf), (plant, cause), both) in [("personal", &personal), ("project", &project)]
        .into_iter()
        .flat_map(|side| breaks.map(|broken| (side, broken)))
        .flat_map(|(side, broken)| [true, false].map(|both| (side, broken, both)))
    {
        let gone = shelf.with_extension("gone");
        let x = shelf.with_extension("x");
        if !both {
            std::fs::rename(shelf.join("x"), &x).unwrap();
        }
        std::fs::rename(shelf, &gone).unwrap();
        plant(shelf);
        let at = shelf.display();
        let refused = format!(
            "error: the {origin} shelf {at} cannot be read (#697): {at}: {cause}; a shelf that \
             cannot be read is never read as an empty one, so mend or remove it; nothing was run \
             or installed\n"
        );
        for verb in &verbs {
            let ran = repo.brokkr("", verb);
            let case = format!("{verb:?} {origin} {cause} {both}");
            assert_eq!((ran.code, ran.stderr), (Some(1), refused.clone()), "{case}");
            assert_eq!((state("x.state"), state("px.state")), kept, "{case}");
        }
        std::fs::remove_file(shelf).unwrap();
        std::fs::rename(&gone, shelf).unwrap();
        if !both {
            std::fs::rename(&x, shelf.join("x")).unwrap();
        }
    }
}

/// A personal install judges no repository, folder or library mark at its
/// shelf or above it (#697 ruling 1): a dangling link standing on the
/// personal shelf, which a library walk would read and refuse, leaves the
/// install of `mine` beside it unrefused, and the link as it was.
#[test]
fn a_personal_install_judges_no_library_at_its_shelf() {
    let repo = shelved();
    repo.bundle("../mine-source", "mine", completes());
    let source = repo.root.with_file_name("mine-source");
    let shelf = repo.config().join("brokkr/recipes");
    let link = shelf.join("broken/bundle.json");
    std::fs::create_dir_all(link.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink("absent.json", &link).unwrap();
    let ran = add_personal(&repo, &source, &[]);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let installed = shelf.join("mine");
    let added = format!(
        "added recipe 'mine' ({}) at {}\n",
        &digest(&repo, installed.to_str().unwrap())[..12],
        installed.display()
    );
    assert_eq!((ran.code, ran.stderr), (Some(0), added));
    assert_eq!(std::fs::read_link(&link).unwrap(), Path::new("absent.json"));
}
