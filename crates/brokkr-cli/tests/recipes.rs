//! Recipe library proof: listing over valid and broken recipes, add from
//! a local path and from a git URL (a local `file://` clone — no
//! network), cleanup of a non-compiling add, and a full delivery driven
//! through `--recipe`. Bundles reuse the machine-proof pattern: the
//! scripted fake driver over the real protocol.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};

fn brokkr_bin() -> &'static str {
    env!("CARGO_BIN_EXE_brokkr")
}

const POLICY: &str = r#"{
  "schema": "forge.phase-machine/v1",
  "phases": ["intake", "implement", "verify", "review", "ship", "done", "stop"],
  "initial": "intake",
  "terminal": ["done", "stop"],
  "shippable_from": ["review"],
  "rules": [
    {"id": "INTAKE-OK", "from": "intake", "result": "resolved", "next": "implement",
     "reason": "Task framed and recorded."},
    {"id": "IMPL-BROKEN-TWICE", "from": "implement", "result": "broken",
     "when": {"consecutive_failures_gte": 2}, "next": "stop", "severity": "hard",
     "reason": "Two consecutive broken implement runs; stop rather than thrash."},
    {"id": "IMPL-BROKEN-RETRY", "from": "implement", "result": "broken",
     "next": "implement", "reason": "First broken run; one re-run permitted."},
    {"id": "IMPL-BLOCKED", "from": "implement", "result": "blocked", "next": "stop",
     "severity": "hard", "reason": "Implementer blocked; report, never silently continue."},
    {"id": "IMPL-OK", "from": "implement", "result": "complete", "next": "verify",
     "reason": "Implementation complete and committed."},
    {"id": "VERIFY-FAIL", "from": "verify", "result": "fail", "next": "stop",
     "severity": "hard", "reason": "Verification failed; not shippable."},
    {"id": "VERIFY-PASS", "from": "verify", "result": "pass", "next": "review",
     "reason": "Suite green; reviewers read verified code."},
    {"id": "REVIEW-SECURITY-HOLD", "from": "review", "result": "security-hold",
     "next": "stop", "severity": "hard",
     "reason": "Unresolved security findings. NEVER ship."},
    {"id": "REVIEW-RESIDUAL-ABOVE-MEDIUM", "from": "review", "result": "residual",
     "when": {"max_residual_severity_above": "medium"}, "next": "stop",
     "severity": "hard", "reason": "Residual severity above medium; not shippable."},
    {"id": "REVIEW-RESIDUAL-SECURITY", "from": "review", "result": "residual",
     "when": {"has_security_residual": true}, "next": "stop", "severity": "hard",
     "reason": "Security residuals never take the tracked-debt path."},
    {"id": "REVIEW-RESIDUAL-OK", "from": "review", "result": "residual", "next": "ship",
     "severity": "flagged",
     "reason": "Non-security residuals at or below medium proceed as tracked debt."},
    {"id": "REVIEW-CLEAN-NO-FIXES", "from": "review", "result": "clean",
     "when": {"fixes_applied": false}, "next": "ship",
     "reason": "Clean with no fixes reported; a gate that moves HEAD parks and ship stops on a dirty tree."},
    {"id": "REVIEW-CLEAN", "from": "review", "result": "clean", "next": "verify",
     "reason": "Clean but fixes applied; re-verify before shipping."},
    {"id": "SHIP-DRIFT", "from": "ship", "result": "ready",
     "when": {"drift_detected": true}, "next": "review", "severity": "flagged",
     "reason": "HEAD moved after review; re-arm a scoped review."},
    {"id": "SHIP-DIRTY", "from": "ship", "result": "ready",
     "when": {"dirty_worktrees": true}, "next": "stop", "severity": "hard",
     "reason": "Dirty tree at ship time is a defect."},
    {"id": "SHIP-READY", "from": "ship", "result": "ready", "next": "ship",
     "reason": "Gates passed and ledger written; confirm close-out and report shipped."},
    {"id": "SHIPPED-DRIFT", "from": "ship", "result": "shipped",
     "when": {"drift_detected": true}, "next": "review", "severity": "flagged",
     "reason": "HEAD moved between ready and close-out; re-arm a scoped review."},
    {"id": "SHIPPED-DIRTY", "from": "ship", "result": "shipped",
     "when": {"dirty_worktrees": true}, "next": "stop", "severity": "hard",
     "reason": "Dirty tree at close-out is a defect."},
    {"id": "SHIP-COMPLETE", "from": "ship", "result": "shipped", "next": "done",
     "reason": "Close-out confirmed: clean, reviewed, verified; done."}
  ]
}"#;

fn happy_script() -> Value {
    json!({"seats": {
        "intake": [{"behavior": "succeed", "result": {"result": "resolved"}}],
        "implement": [{"behavior": "succeed", "result": {"result": "complete"}}],
        "verify": [{"behavior": "succeed", "result": {"result": "pass"}}],
        "review:correctness": [{"behavior": "succeed",
            "result": {"result": "clean", "inputs": {"fixes_applied": false}}}],
        "review:security": [{"behavior": "succeed",
            "result": {"result": "clean", "inputs": {"fixes_applied": false}}}],
        "ship": [
            {"behavior": "succeed", "result": {"result": "ready"}},
            {"behavior": "succeed", "result": {"result": "shipped"}},
        ],
    }})
}

struct Ws {
    dir: tempfile::TempDir,
    /// The HOME every `brokkr` here is given, apart from the workspace a
    /// run operates (decision 0065 slice two, U8a2a).
    home: tempfile::TempDir,
}

impl Ws {
    fn new() -> Ws {
        let ws = Ws {
            dir: tempfile::tempdir().unwrap(),
            home: tempfile::tempdir().unwrap(),
        };
        std::fs::create_dir_all(ws.path().join("state")).unwrap();
        std::fs::write(
            ws.path().join("script.json"),
            serde_json::to_string(&happy_script()).unwrap(),
        )
        .unwrap();
        ws
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    /// A workspace whose compiling recipe at `src-bundle` `recipes add`
    /// installed as `name`, the source, and what the install printed.
    fn installed(name: &str) -> (Ws, PathBuf, String) {
        let ws = Ws::new();
        let src = ws.path().join("src-bundle");
        ws.write_recipe(&src);
        let add = ["recipes", "add", src.to_str().unwrap(), "--name", name];
        let (code, _, stderr) = ws.brokkr(&add);
        assert_eq!(code, Some(0), "stderr: {stderr}");
        (ws, src, stderr)
    }

    /// A minimal compiling bundle at `at`, review as a two-member panel
    /// so the listing's `review[correctness+security]` rendering is
    /// exercised. Every seat hires the workspace's scripted fake office.
    fn write_recipe(&self, at: &Path) {
        self.write_recipe_hiring(at, self.path());
    }

    /// [`Ws::write_recipe`], its office hired at `places`, where the
    /// workspace that compiles it reads its agents and adapters.
    fn write_recipe_hiring(&self, at: &Path, places: &Path) {
        std::fs::create_dir_all(at.join("roles")).unwrap();
        std::fs::write(at.join("policy.json"), POLICY).unwrap();
        std::fs::write(at.join("roles/role.md"), "# test role\n").unwrap();
        self.hire(places);
        let seat = crate::fake_office::seat;
        let body = crate::fake_office::body;
        let config = json!({
            "name": "recipe-proof",
            "policy": "policy.json",
            "seats": {
                "intake": seat(&["resolved"]),
                "implement": seat(&["complete", "broken", "blocked"]),
                "verify": seat(&["pass", "fail"]),
                "review": {
                    "results": ["clean", "residual", "security-hold"],
                    "aggregate": "review-panel",
                    "panel": {"correctness": body(), "security": body()},
                },
                "ship": seat(&["ready", "shipped"]),
            }
        });
        std::fs::write(
            at.join("bundle.json"),
            serde_json::to_string_pretty(&config).unwrap(),
        )
        .unwrap();
    }

    /// A recipe that extends `base` (a sibling DIRECTORY name) and
    /// states one difference: a different review panel. `marker` is the
    /// resolver-owned `override` block — omit it and the collision is a
    /// compile error, which is the point of decision 0017's merge rules.
    fn write_derived(&self, at: &Path, name: &str, base: &str, marker: bool) {
        std::fs::create_dir_all(at).unwrap();
        self.hire(self.path());
        let body = crate::fake_office::body;
        let mut config = json!({
            "name": name,
            "extends": base,
            "seats": {
                "review": {
                    "results": ["clean", "residual", "security-hold"],
                    "aggregate": "review-panel",
                    "panel": {"adversarial": body(), "security": body()},
                },
            },
        });
        if marker {
            config["override"] = json!({"seats": ["review"]});
        }
        std::fs::write(
            at.join("bundle.json"),
            serde_json::to_string_pretty(&config).unwrap(),
        )
        .unwrap();
    }

    /// The fake office over the workspace's script and state, at `places`
    /// (U9b1).
    fn hire(&self, places: &Path) {
        let (script, state) = (self.path().join("script.json"), self.path().join("state"));
        crate::fake_office::hire(places, &script, &state);
    }

    /// A bundle directory that fails to compile: bundle.json without
    /// 'policy'.
    fn write_broken(&self, at: &Path) {
        std::fs::create_dir_all(at).unwrap();
        std::fs::write(at.join("bundle.json"), r#"{"name": "broken"}"#).unwrap();
    }

    /// `brokkr <args>` in the workspace, whose personal shelf (#697) is one
    /// nothing stands at, so no host's own is listed here.
    fn brokkr(&self, args: &[&str]) -> (Option<i32>, String, String) {
        let mut command = Command::new(brokkr_bin());
        command.args(args).current_dir(self.path());
        command.env("XDG_CONFIG_HOME", self.path().join(".config"));
        command.env("HOME", self.home.path());
        let ran = crate::brokkr_folder::Ran::of(command.output().unwrap());
        (ran.code, ran.stdout, ran.stderr)
    }

    fn recipes_dir(&self) -> PathBuf {
        // Where `recipes add` installs from this workspace, a bare directory
        // and so its own root (#696 ruling 6), as the walk resolves it.
        self.path().canonicalize().unwrap().join("recipes")
    }
}

fn git(cwd: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args([
            "-c",
            "user.email=brokkr@test",
            "-c",
            "user.name=forge",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn list_prints_valid_recipes_and_warns_on_broken_ones() {
    let ws = Ws::new();
    let rdir = ws.recipes_dir();
    ws.write_recipe(&rdir.join("good"));
    ws.write_broken(&rdir.join("broken"));

    // The digest the listing must abbreviate.
    let (code, stdout, stderr) =
        ws.brokkr(&["compile", "--bundle", rdir.join("good").to_str().unwrap()]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let compiled: Value = serde_json::from_str(&stdout).unwrap();
    let digest = compiled["digest"].as_str().unwrap();

    let (code, stdout, stderr) = ws.brokkr(&["recipes", "list", "--dir", rdir.to_str().unwrap()]);
    assert_eq!(code, Some(0), "a broken recipe must not abort: {stderr}");

    let line = stdout
        .lines()
        .find(|l| l.starts_with("good\t"))
        .expect("listing line for the valid recipe");
    let cols: Vec<&str> = line.split('\t').collect();
    assert_eq!(
        cols.len(),
        8,
        "name, digest, phases, seats, cost, description, origin, path: {line}"
    );
    assert_eq!(
        cols[1],
        &digest[..12],
        "short digest is the manifest digest prefix"
    );
    assert!(cols[1].chars().all(|c| c.is_ascii_hexdigit()));
    assert_eq!(cols[2], "7 phases");
    assert!(
        cols[3].contains("review[correctness+security]"),
        "seats: {}",
        cols[3]
    );
    assert!(cols[3].contains("implement"), "seats: {}", cols[3]);
    assert!(cols[4].is_empty(), "legacy fixture has no cost: {line}");
    assert!(
        cols[5].is_empty(),
        "legacy fixture has no description: {line}"
    );
    assert_eq!(cols[6], "project", "the origin (#697): {line}");
    assert!(cols[7].ends_with("good"), "source path: {}", cols[7]);

    let warning = stdout
        .lines()
        .find(|l| l.starts_with("warning:") && l.contains("broken"))
        .expect("warning line for the broken recipe");
    assert!(
        warning.contains("missing 'policy'"),
        "warning names the error: {warning}"
    );

    // Brokkr's own bundles are library recipes now (#359), listed only
    // from the library they are in, never beside another one.
    let names: Vec<&str> = stdout
        .lines()
        .filter_map(|line| {
            line.split(['\t', ' '])
                .nth(usize::from(line.starts_with("warning:")))
        })
        .collect();
    assert_eq!(names, ["broken", "good"], "stdout: {stdout}");
}

#[test]
fn add_from_local_path_installs_and_lists() {
    let (ws, _, stderr) = Ws::installed("mine");
    let rdir = ws.recipes_dir();
    assert!(stderr.contains("added recipe 'mine'"), "stderr: {stderr}");
    assert!(rdir.join("mine/bundle.json").is_file());

    let (code, _, stderr) =
        ws.brokkr(&["compile", "--bundle", rdir.join("mine").to_str().unwrap()]);
    assert_eq!(code, Some(0), "installed copy compiles: {stderr}");

    let (code, stdout, _) = ws.brokkr(&["recipes", "list", "--dir", rdir.to_str().unwrap()]);
    assert_eq!(code, Some(0));
    assert!(
        stdout.lines().any(|l| l.starts_with("mine\t")),
        "stdout: {stdout}"
    );
}

#[test]
fn add_from_git_url_clones_and_installs_without_network() {
    let ws = Ws::new();
    let rdir = ws.recipes_dir();

    // Fixture repo with the bundle in a subdirectory: exercises the
    // clone-root-without-bundle.json resolution. The `file://` scheme is
    // classified as a git source (see recipes::is_git_source).
    let repo = ws.path().join("fixture-repo");
    ws.write_recipe(&repo.join("bundle"));
    git(&repo, &["init"]);
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-m", "fixture bundle"]);
    let url = format!("file://{}", repo.display());

    let (code, _, stderr) = ws.brokkr(&["recipes", "add", &url, "--name", "cloned"]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert!(rdir.join("cloned/bundle.json").is_file());
    assert!(
        !rdir.join("cloned/.git").exists(),
        "recipes are plain files, not repos"
    );

    let (code, _, stderr) =
        ws.brokkr(&["compile", "--bundle", rdir.join("cloned").to_str().unwrap()]);
    assert_eq!(code, Some(0), "cloned copy compiles: {stderr}");
}

/// `repo` made a git repository with every file in it committed, and the
/// `file://` URL a clone reads it by, with no network.
pub(crate) fn committed(repo: &Path) -> String {
    git(repo, &["init"]);
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-m", "fixture recipes"]);
    format!("file://{}", repo.display())
}

/// What `recipes add` prints for the recipes it installed into `rdir`, in
/// the order given, each by the digest its installed copy compiles to.
fn added(ws: &Ws, rdir: &Path, names: &[&str]) -> String {
    let line = |name: &&str| {
        let at = rdir.join(name);
        let (code, stdout, stderr) = ws.brokkr(&["compile", "--bundle", at.to_str().unwrap()]);
        assert_eq!(code, Some(0), "{stderr}");
        let compiled: Value = serde_json::from_str(&stdout).unwrap();
        let digest = &compiled["digest"].as_str().unwrap()[..12];
        format!("added recipe '{name}' ({digest}) at {}\n", at.display())
    };
    names.iter().map(line).collect()
}

/// The names `rdir` holds.
fn library(rdir: &Path) -> std::collections::BTreeSet<String> {
    let entries = std::fs::read_dir(rdir).unwrap();
    entries
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect()
}

/// `--path` installs the bundle at that directory of a clone that keeps its
/// recipes below its root, where discovery finds none (#695).
#[test]
fn add_with_path_installs_the_bundle_it_names_inside_the_clone() {
    let ws = Ws::new();
    let rdir = ws.recipes_dir();
    let repo = ws.path().join("fixture-repo");
    for recipe in ["x", "y"] {
        ws.write_recipe(&repo.join("recipes").join(recipe));
    }
    std::fs::write(repo.join("recipes/x/roles/x.md"), "x\n").unwrap();
    let url = committed(&repo);
    let add = ["recipes", "add", &url, "--name", "x"];

    let (code, _, stderr) = ws.brokkr(&add);
    let discovered = "error: clone has no bundle.json at its root or in any subdirectory\n";
    assert_eq!((code, stderr.as_str()), (Some(1), discovered));
    let (code, _, stderr) = ws.brokkr(&[&add[..], &["--path", "recipes/x"]].concat());
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(stderr, added(&ws, &rdir, &["x"]));
    assert_eq!(library(&rdir), ["x".to_string()].into());
    assert!(rdir.join("x/roles/x.md").is_file(), "x, not y");
}

/// A recipe at `--path` brings each base it extends from its own library,
/// the directory beside it in the clone (decision 0078 ruling 3; #695).
#[test]
fn add_with_path_brings_the_bases_its_recipe_extends_from_beside_it() {
    let ws = Ws::new();
    let rdir = ws.recipes_dir();
    let repo = ws.path().join("fixture-repo");
    ws.write_recipe(&repo.join("recipes/fast"));
    ws.write_derived(&repo.join("recipes/node"), "node", "fast", true);
    let url = committed(&repo);
    let (code, _, stderr) = ws.brokkr(&[
        "recipes",
        "add",
        &url,
        "--name",
        "node",
        "--path",
        "recipes/node",
    ]);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(stderr, added(&ws, &rdir, &["node", "fast"]));
    let both = ["fast".to_string(), "node".to_string()];
    assert_eq!(library(&rdir), both.into());
}

/// A `--path` that is not plain, that passes through a link the clone
/// holds, that leads nowhere or to a directory with no bundle.json, or that
/// comes with a local source refuses by name, and the library is never
/// made (#695). The link leads to a compiling bundle outside the clone.
#[test]
fn add_with_path_refuses_what_it_cannot_reach_and_writes_nothing() {
    let ws = Ws::new();
    let rdir = ws.recipes_dir();
    let repo = ws.path().join("fixture-repo");
    let outside = ws.path().join("outside");
    ws.write_recipe(&repo.join("recipes/x"));
    ws.write_recipe(&outside);
    std::fs::create_dir_all(repo.join("recipes/empty")).unwrap();
    std::fs::write(repo.join("recipes/empty/README.md"), "no bundle\n").unwrap();
    std::os::unix::fs::symlink(&outside, repo.join("recipes/link")).unwrap();
    let url = committed(&repo);
    let local = repo.to_str().unwrap();

    let unplain = |path: &str| {
        format!(
            "error: --path '{path}' is not a relative path of plain names, so recipes add cannot \
             judge where in the clone it leads (#695); nothing was cloned or written\n"
        )
    };
    let mut refusals: Vec<(&str, &str, String)> = ["../x", "/abs", "recipes/./x", "recipes/x/", ""]
        .into_iter()
        .map(|path| (&*url, path, unplain(path)))
        .collect();
    refusals.extend([
        (
            &*url,
            "recipes/link",
            "error: --path recipes/link passes through recipes/link, a symbolic link in the \
             clone, which can name anything outside it (#695); nothing was written\n"
                .to_string(),
        ),
        (
            &*url,
            "recipes/absent/x",
            "error: --path recipes/absent/x: the clone holds no directory at recipes/absent \
             (#695); nothing was written\n"
                .to_string(),
        ),
        (
            &*url,
            "recipes/empty",
            "error: --path recipes/empty: the clone's recipes/empty holds no bundle.json (#695); \
             nothing was written\n"
                .to_string(),
        ),
        (
            local,
            "recipes/x",
            format!(
                "error: --path names a directory inside a git source's clone, and {local} is a \
                 local directory: name {local}/recipes/x as the source itself (#695); nothing \
                 was written\n"
            ),
        ),
    ]);
    for (source, path, refusal) in refusals {
        let (code, _, stderr) =
            ws.brokkr(&["recipes", "add", source, "--name", "x", "--path", path]);
        assert_eq!((code, stderr), (Some(1), refusal), "{path}");
        assert!(!rdir.exists(), "{path}");
    }
}

#[test]
fn add_of_a_non_compiling_bundle_cleans_up_and_fails() {
    let ws = Ws::new();
    let rdir = ws.recipes_dir();
    let src = ws.path().join("bad-bundle");
    ws.write_broken(&src);

    let (code, _, stderr) = ws.brokkr(&["recipes", "add", src.to_str().unwrap(), "--name", "bad"]);
    assert_eq!(code, Some(1), "stderr: {stderr}");
    assert!(
        stderr.contains("missing 'policy'"),
        "prints the compile error: {stderr}"
    );
    assert!(!rdir.join("bad").exists(), "the rejected copy is removed");
}

#[test]
fn add_refuses_the_ext_transport_that_would_execute_a_command() {
    let ws = Ws::new();
    let rdir = ws.recipes_dir();
    // Ends with `.git` so it classifies as a git source; the ext
    // transport would run `sh -c` if git were allowed to use it.
    let marker = ws.path().join("pwned");
    let url = format!("ext::sh -c \"touch {}\" x.git", marker.display());

    let (code, _, _) = ws.brokkr(&["recipes", "add", &url, "--name", "evil"]);
    assert_ne!(code, Some(0), "ext transport must be refused");
    assert!(!marker.exists(), "the ext command must never execute");
    assert!(!rdir.join("evil").exists());
}

#[cfg(unix)]
#[test]
fn add_refuses_symlinks_and_removes_the_partial_copy() {
    let ws = Ws::new();
    let rdir = ws.recipes_dir();
    let src = ws.path().join("sneaky");
    ws.write_recipe(&src);
    let secret = ws.path().join("secret.txt");
    std::fs::write(&secret, "not for the library").unwrap();
    std::os::unix::fs::symlink(&secret, src.join("zz-link")).unwrap();

    let (code, _, stderr) =
        ws.brokkr(&["recipes", "add", src.to_str().unwrap(), "--name", "sneaky"]);
    assert_eq!(code, Some(1), "stderr: {stderr}");
    assert!(stderr.contains("symlink"), "names the refusal: {stderr}");
    assert!(!rdir.join("sneaky").exists(), "the partial copy is removed");
}

/// What `recipes add` prints when it refuses to install into `place`,
/// inside the folder repository at `root` but not its shelf (#675, #696),
/// its one pin.
pub(crate) fn in_folder(root: &Path, place: &Path) -> String {
    format!(
        "error: recipes add would install into {place}, inside the folder repository {root} but \
         not its shelf {root}/.brokkr/recipes/ (decision 0083; #675, #696): a folder repository \
         keeps its named recipes on its shelf, so nothing was written\n",
        root = root.display(),
        place = place.display()
    )
}

/// `recipes add` in a folder repository installs onto its shelf,
/// `.brokkr/recipes/` (#696), from its root and from a subdirectory, and
/// writes no root `recipes/` and no `.forge/`: no library mark appears to
/// outrank the folder, so the next verb still reads the folder's journal,
/// and the folder bundle's digest never moves. `list` and `show` find the
/// recipes with no flag; `add` and `list` print one digest (#704 keeps
/// theirs apart from `compile`'s).
#[test]
fn add_installs_onto_the_shelf_of_a_folder_repository() {
    let (repo, _ws, src) = folder_and_source();
    let before = crate::brokkr_folder::digest(&repo, ".brokkr");
    let shelf = repo.path(".brokkr/recipes");
    let mut added = Vec::new();
    for (from, name) in [("", "x"), ("src", "y")] {
        let ran = repo.brokkr(
            from,
            &["recipes", "add", src.to_str().unwrap(), "--name", name],
        );
        assert_eq!(ran.code, Some(0), "{}", ran.stderr);
        added.push(ran.stderr);
    }
    assert_eq!(crate::brokkr_folder::digest(&repo, ".brokkr"), before);
    let listed = repo.brokkr("src", &["recipes", "list"]);
    let line = |l: &str| {
        let (name, digest) = (&l[..1], &l[2..14]);
        format!(
            "added recipe '{name}' ({digest}) at {}/{name}\n",
            shelf.display()
        )
    };
    let lines: Vec<String> = listed.stdout.lines().map(line).collect();
    assert_eq!((listed.code, lines), (Some(0), added));
    let shown = repo.brokkr("src", &["recipes", "show", "y"]);
    assert_eq!(shown.code, Some(0), "{}", shown.stderr);
    for left in ["recipes", "src/recipes", ".forge"] {
        assert!(!repo.path(left).exists(), "{left}");
    }
    reads_the_folder_journal(&repo);
}

/// A folder repository made by `brokkr init .`, and a workspace beside it
/// holding a compiling recipe source at the path returned.
fn folder_and_source() -> (crate::brokkr_folder::Repo, Ws, PathBuf) {
    let repo = crate::brokkr_folder::Repo::new();
    let init = repo.init_with_only(&["claude"]);
    assert_eq!(init.code, Some(0), "{}", init.stderr);
    let ws = Ws::new();
    let src = ws.path().join("src-bundle");
    ws.write_recipe_hiring(&src, &repo.path(".brokkr"));
    (repo, ws, src)
}

/// `brokkr runs` in `repo` still reads the folder's journal: no library
/// mark has appeared to outrank it. No run has opened the journal yet, so
/// the read names where it looks.
pub(crate) fn reads_the_folder_journal(repo: &crate::brokkr_folder::Repo) {
    let runs = repo.brokkr("", &["runs"]);
    let journal = repo.path(".brokkr/state/forge.db");
    let read = format!(
        "error: journal does not exist: {}; a read never creates one",
        journal.display()
    );
    assert_eq!(
        (runs.code, runs.stderr.lines().last()),
        (Some(1), Some(&*read))
    );
}

/// `recipes add` writes only where it judged (#675): from a library beside
/// a folder repository, a `--name` that climbs into the repository's
/// `recipes/` refuses by name and writes nothing.
#[test]
fn add_refuses_a_name_that_would_write_where_it_did_not_judge() {
    let (repo, ws, src) = folder_and_source();
    let outside = repo.root.parent().unwrap();
    ws.write_recipe(&outside.join("lib/recipes/base"));
    std::fs::create_dir(repo.path("recipes")).unwrap();
    let climb = "../../repo/recipes/x";
    let ran = repo.brokkr(
        "../lib",
        &["recipes", "add", src.to_str().unwrap(), "--name", climb],
    );
    let refusal = format!(
        "error: --name '{climb}' is not one plain path component, so the recipe would not be \
         one directory of {}/lib/recipes (#675); nothing was written\n",
        outside.display()
    );
    assert_eq!((ran.code, ran.stderr), (Some(1), refusal));
    assert_eq!(std::fs::read_dir(repo.path("recipes")).unwrap().count(), 0);
    assert!(!outside.join("lib/recipes/x").exists());
}

/// `recipes add` from a bare directory beside a folder repository, whose
/// `recipes` is a link into the repository's `recipes/` as a plain
/// directory or a nested repository, refuses by name and writes nothing
/// through the link: the library is judged where it resolves (#675, #696),
/// so no library mark appears to outrank the folder.
#[test]
fn add_refuses_a_folder_repository_behind_a_link_or_a_nested_repository() {
    let (repo, ws, src) = folder_and_source();
    let into = repo.path("recipes");
    std::os::unix::fs::symlink(&into, ws.path().join("recipes")).unwrap();
    let plant_directory = |at: &Path| std::fs::create_dir(at).unwrap();
    let plant_repository = |at: &Path| {
        plant_directory(at);
        crate::brokkr_folder::git(at, &["init", "-q"]);
    };
    for plant in [&plant_directory as &dyn Fn(&Path), &plant_repository] {
        plant(&into);
        let before = repo.files();
        let (code, _, stderr) =
            ws.brokkr(&["recipes", "add", src.to_str().unwrap(), "--name", "x"]);
        assert_eq!((code, stderr), (Some(1), in_folder(&repo.root, &into)));
        assert_eq!(repo.files(), before);
        assert!(!into.join("x").exists());
        reads_the_folder_journal(&repo);
        std::fs::remove_dir_all(&into).unwrap();
    }
}

/// `recipes add` from beside a folder repository, in the directory that
/// holds it or in a sibling bare repository, whose `recipes` is a link,
/// absolute or relative, into a nested repository at the folder
/// repository's `recipes/` refuses by name and writes nothing there: the
/// library is judged where it resolves as well as where it is spelled
/// (#675, #696).
#[test]
fn add_refuses_a_folder_repository_a_link_from_outside_resolves_into() {
    let (repo, _ws, src) = folder_and_source();
    let into = repo.path("recipes");
    std::fs::create_dir(&into).unwrap();
    crate::brokkr_folder::git(&into, &["init", "-q"]);
    let outside = repo.root.parent().unwrap();
    std::fs::create_dir(outside.join("sibling")).unwrap();
    crate::brokkr_folder::git(&outside.join("sibling"), &["init", "-q"]);
    for (from, link, target) in [
        ("..", outside.join("recipes"), into.as_path()),
        ("..", outside.join("recipes"), Path::new("repo/recipes")),
        (
            "../sibling",
            outside.join("sibling/recipes"),
            into.as_path(),
        ),
    ] {
        std::os::unix::fs::symlink(target, &link).unwrap();
        let before = repo.files();
        let ran = repo.brokkr(
            from,
            &["recipes", "add", src.to_str().unwrap(), "--name", "x"],
        );
        let refused = (ran.code, ran.stderr);
        assert_eq!(refused, (Some(1), in_folder(&repo.root, &into)), "{from}");
        assert_eq!(repo.files(), before);
        let left: Vec<_> = std::fs::read_dir(&into)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(left, [".git"]);
        reads_the_folder_journal(&repo);
        std::fs::remove_file(&link).unwrap();
    }
}

/// A folder repository whose shelf, `.brokkr/recipes`, links to its root
/// `recipes/` refuses `recipes add` by name and writes nothing: the shelf
/// is judged where it resolves (#696).
#[test]
fn add_refuses_a_shelf_that_links_out_of_the_folder() {
    let (repo, _ws, src) = folder_and_source();
    let into = repo.path("recipes");
    std::fs::create_dir(&into).unwrap();
    std::os::unix::fs::symlink("../recipes", repo.path(".brokkr/recipes")).unwrap();
    let before = repo.files();
    let ran = repo.brokkr(
        "",
        &["recipes", "add", src.to_str().unwrap(), "--name", "x"],
    );
    assert_eq!(
        (ran.code, ran.stderr),
        (Some(1), in_folder(&repo.root, &into))
    );
    assert_eq!(repo.files(), before);
    assert_eq!(std::fs::read_dir(&into).unwrap().count(), 0);
    reads_the_folder_journal(&repo);
}

/// `recipes add` takes no `--dir` (#696 ruling 6), so clap refuses one as
/// an unexpected argument. In a recipe library it installs into the
/// library at the root from any subdirectory, and `list --dir` and a
/// flagless `show` from the root read it.
#[test]
fn add_has_no_dir_and_installs_at_a_librarys_root() {
    let repo = crate::brokkr_folder::Repo::new();
    repo.library();
    let ws = Ws::new();
    let src = ws.path().join("src-bundle");
    ws.write_recipe_hiring(&src, &repo.path(""));
    let add = ["recipes", "add", src.to_str().unwrap(), "--name", "z"];
    let ran = repo.brokkr("sub", &[&add[..], &["--dir", "x"]].concat());
    let unexpected = "error: unexpected argument '--dir' found";
    assert_eq!((ran.code, ran.first_line()), (Some(2), unexpected));
    let ran = repo.brokkr("sub", &add);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert!(repo.path("recipes/z/bundle.json").is_file());
    assert!(!repo.path("sub/recipes").exists());
    // From the root, whose agents its recipes hire (U9b1): `list --dir`
    // compiles each with the working directory's library.
    let listed = repo.brokkr("", &["recipes", "list", "--dir", "recipes"]);
    let names: Vec<&str> = listed.stdout.lines().map(|l| &l[..2]).collect();
    assert_eq!((listed.code, names), (Some(0), vec!["x\t", "z\t"]));
    let shown = repo.brokkr("", &["recipes", "show", "z"]);
    assert_eq!(shown.code, Some(0), "{}", shown.stderr);
}

#[test]
fn run_recipe_completes_a_delivery_and_arg_group_holds() {
    let (ws, src, _) = Ws::installed("good");
    let rdir = ws.recipes_dir();

    let db = ws.path().join("forge.db");
    let (code, stdout, stderr) = ws.brokkr(&[
        "run",
        "--recipe",
        "good",
        "--recipes-dir",
        rdir.to_str().unwrap(),
        "--feature",
        "recipe feature",
        "--db",
        db.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let summary: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(summary["status"], "completed");
    assert_eq!(summary["phase"], "done");

    // Exactly one of --bundle/--recipe: both is a usage error...
    let (code, _, stderr) = ws.brokkr(&[
        "run",
        "--bundle",
        src.to_str().unwrap(),
        "--recipe",
        "good",
        "--feature",
        "f",
        "--db",
        db.to_str().unwrap(),
    ]);
    assert_ne!(code, Some(0));
    assert!(stderr.contains("cannot be used with"), "stderr: {stderr}");

    // ...and neither, in this library with no `.brokkr/` folder to supply
    // the implicit bundle, refuses naming init (decision 0083 ruling 3).
    let (code, _, stderr) = ws.brokkr(&["run", "--feature", "f", "--db", db.to_str().unwrap()]);
    let at = ws.path().canonicalize().unwrap();
    let at = at.display();
    assert_eq!(code, Some(1));
    assert_eq!(
        stderr,
        format!(
            "error: no .brokkr/ folder from {at} up to {at} (decision 0083), so there is no \
             default bundle: name one with --bundle or --recipe, or run `brokkr init .` from \
             {at}\n"
        )
    );
}

#[test]
fn compile_and_show_print_the_resolved_result_and_its_provenance() {
    let ws = Ws::new();
    let rdir = ws.recipes_dir();
    ws.write_recipe(&rdir.join("good"));
    ws.write_derived(&rdir.join("paranoid"), "paranoid", "good", true);

    // A bundle that composed nothing prints exactly what it always did:
    // no `composed_from` member at all.
    let (code, plain, stderr) =
        ws.brokkr(&["compile", "--bundle", rdir.join("good").to_str().unwrap()]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let base: Value = serde_json::from_str(&plain).unwrap();
    assert!(base.get("composed_from").is_none(), "{plain}");
    // Decision 0050's sweep of the table goes to stderr: a v1 table's
    // presence finding is reported and the table is not refused (#429).
    assert_eq!(
        stderr,
        "policy sweep (decision 0050): 32 valuations over 11 groups, 0 \
         unruled\n  REVIEW-RESIDUAL-OK lets the run \
         go on without reading has_security_residual, max_residual_severity, \
         which a hard rule of its group reads\n"
    );

    // A composed one prints the RESOLVED result and its provenance.
    let (code, composed, stderr) = ws.brokkr(&[
        "compile",
        "--bundle",
        rdir.join("paranoid").to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let view: Value = serde_json::from_str(&composed).unwrap();
    assert_eq!(view["bundle"], "paranoid");
    let seats: Vec<&str> = view["seats"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap())
        .collect();
    assert_eq!(
        seats,
        vec!["implement", "intake", "review", "ship", "verify"],
        "the resolved bundle carries every inherited seat"
    );
    let chain = view["composed_from"].as_array().unwrap();
    assert_eq!(chain.len(), 1);
    assert_eq!(chain[0]["recipe"], "recipe-proof");
    // A layer's digest is the base's manifest WITHOUT its `capabilities`
    // section (decision 0065): authority belongs to the bundle compiled in
    // a realm, never to an ancestor, so the standalone base — which says
    // what its realm grants — and the layer differ by exactly that section.
    assert_ne!(chain[0]["digest"], base["digest"]);
    // Nor does a layer carry the leaf's agent resolution: its seats hire
    // the fake office (U9b1), whose placements belong to the composed
    // bundle.
    let mut layer = base["manifest"].clone();
    let sections = layer.as_object_mut().unwrap();
    assert!(sections.remove("capabilities").is_some());
    assert!(sections.remove("agents").is_some());
    let layer_digest = json!(brokkr_core::canonical::sha256_hex(&layer));
    assert_eq!(chain[0]["digest"], layer_digest);
    assert!(chain[0]["dir"].as_str().unwrap().ends_with("good"));
    assert_eq!(
        view["manifest"]["files"]["@compose/0000/recipe-proof@good"], layer_digest,
        "the chain pins the run through the digested manifest, naming BOTH the \
         declared name and the library directory it was extended by — recording \
         only one lets a directory answer to a name it does not declare"
    );

    // `recipes show` is the same renderer, reached by name, for both.
    let (code, shown, stderr) = ws.brokkr(&[
        "recipes",
        "show",
        "paranoid",
        "--dir",
        rdir.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert_eq!(shown, composed);
    let (code, shown, _) = ws.brokkr(&["recipes", "show", "good", "--dir", rdir.to_str().unwrap()]);
    assert_eq!(code, Some(0));
    assert_eq!(shown, plain);

    let (code, _, stderr) =
        ws.brokkr(&["recipes", "show", "absent", "--dir", rdir.to_str().unwrap()]);
    assert_eq!(code, Some(1));
    assert!(stderr.contains("recipe 'absent' not found"), "{stderr}");
}

#[test]
fn a_merge_conflict_names_the_file_and_the_conflicting_key() {
    let ws = Ws::new();
    let rdir = ws.recipes_dir();
    ws.write_recipe(&rdir.join("good"));
    ws.write_derived(&rdir.join("clash"), "clash", "good", false);

    let (code, _, stderr) =
        ws.brokkr(&["compile", "--bundle", rdir.join("clash").to_str().unwrap()]);
    assert_eq!(code, Some(1));
    assert!(stderr.contains("redefines seat 'review'"), "{stderr}");
    // Windows spells the same path with backslashes; the assertion is
    // about which files are named, not about the separator.
    let named = stderr.replace('\\', "/");
    assert!(named.contains("clash/bundle.json"), "{stderr}");
    assert!(named.contains("good/bundle.json"), "{stderr}");
    assert!(stderr.contains("override.seats"), "{stderr}");
}

#[test]
fn list_warns_when_a_derived_recipes_base_is_missing() {
    let ws = Ws::new();
    let rdir = ws.recipes_dir();
    ws.write_recipe(&rdir.join("good"));
    let orphan = rdir.join("orphan");
    std::fs::create_dir_all(&orphan).unwrap();
    std::fs::write(
        orphan.join("bundle.json"),
        json!({"name": "orphan", "extends": "absent"}).to_string(),
    )
    .unwrap();

    let (code, stdout, stderr) = ws.brokkr(&["recipes", "list", "--dir", rdir.to_str().unwrap()]);
    assert_eq!(code, Some(0), "a missing base must not abort: {stderr}");
    assert!(
        stdout.lines().any(|l| l.starts_with("good\t")),
        "everything else still lists: {stdout}"
    );
    let warning = stdout
        .lines()
        .find(|l| l.starts_with("warning:") && l.contains("orphan"))
        .expect("warning line for the orphan");
    assert!(warning.contains("extends 'absent'"), "{warning}");
    assert!(warning.contains("is not a recipe in"), "{warning}");
}

/// Each (map, recipe) pair `compile` and `recipes show` refuse alike,
/// with the line both print: the specifying recipes need a dialect no
/// unmapped repository has, and hold a dialect gate boxed that `harness`
/// cannot box; three more hire a `claude` link `harness` has no fragment
/// for.
fn refused() -> std::collections::BTreeMap<(&'static str, &'static str), String> {
    let specifying = [
        ("triage", "triage -> brokkr-fast -> fast"),
        (
            "night-shift",
            "night-shift -> triage -> brokkr-fast -> fast",
        ),
        ("gpt-flash", "gpt-flash -> triage -> brokkr-fast -> fast"),
    ];
    let undialected = "realm '<unmapped>' declares no dialect, but phase 'specify' needs one";
    let unboxed = "dialect step 'analyze:check' holds its gate boxed (decision 0042 ruling 4), \
        and under the `harness` boundary no box stands: its command is the realm's dialect \
        declaration, not the bundle's own pinned script, which is all decision 0046 ruling 4 \
        admits at an unboxed gate. Run the realm under a boxed boundary (namespace) until a \
        decision admits the dialect step on the pinned-script terms";
    let mut refused = std::collections::BTreeMap::new();
    for (label, why) in [
        ("no map", undialected),
        ("harness elsewhere", undialected),
        ("harness here", unboxed),
    ] {
        for (name, chain) in specifying {
            let line = format!("error: bundle: bundle: {why} (composed: {chain})\n");
            refused.insert((label, name), line);
        }
    }
    // Both compose since #359, so each refusal names its chain.
    let judged = |seat: &str, chain: &str| {
        format!(
            "error: bundle: bundle: seat '{seat}' gate link 2 resolves to provider 'claude', \
             which declares no `hands.harness.gate` fragment; under the `harness` boundary a \
             model may judge only under its harness's own read-only sandbox as the adapter \
             addresses it (decision 0046 ruling 4) (composed: {chain})\n"
        )
    };
    refused.insert(
        ("harness here", "panel-review"),
        judged("review:correctness", "panel-review -> brokkr-fast -> fast"),
    );
    refused.insert(
        ("harness here", "self"),
        judged("review", "self -> panel-review -> brokkr-fast -> fast"),
    );
    let worked = "error: bundle: bundle: seat 'implement' link 1 resolves to provider 'claude', \
        which declares no `hands.harness.work` fragment: a capability gap — under the `harness` \
        boundary a work seat with hands writes the tree only under the harness's own writable \
        sandbox as the adapter addresses it (decision 0046 rulings 1 and 4) (composed: release \
        -> brokkr-fast -> fast)\n";
    refused.insert(("harness here", "release"), worked.to_string());
    refused
}

/// What `recipes show --dir` printed, as `compile --bundle` prints it: the
/// shelf `--dir` selects is the project's (#697 ruling 3), where `compile`
/// reads the same layers as a library's, so each `project` origin in the
/// view's `layers` reads `library`. The origins are evidence outside the
/// pin, and every other byte is the same.
fn as_compiled(
    (code, view, sweep): (Option<i32>, String, String),
) -> (Option<i32>, String, String) {
    let Ok(mut parsed) = serde_json::from_str::<Value>(&view) else {
        return (code, view, sweep);
    };
    let layers = parsed.get_mut("layers").and_then(Value::as_array_mut);
    for layer in layers.into_iter().flatten() {
        if layer["origin"] == "project" {
            layer["origin"] = json!("library");
        }
    }
    let view = format!("{}\n", serde_json::to_string_pretty(&parsed).unwrap());
    (code, view, sweep)
}

/// `recipes show` compiles on the path `compile` and a run compile on
/// (#350): for every recipe, Brokkr's own included, under no map, a
/// realm with a dialect, the same realm under `harness`, and a map that
/// names another tree, the two verbs print the same bytes. Every pair is
/// one of two outcomes and nothing else: both print a view whose digest
/// is a sha256 in lowercase hex, or both refuse with exactly the line
/// [`refused`] expects for that pair.
#[test]
fn show_and_compile_agree_for_every_recipe_under_every_realm_map() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ws = Ws::new();
    for tree in ["agents", "adapters", "dialects"] {
        std::os::unix::fs::symlink(root.join(tree), ws.path().join(tree)).unwrap();
    }
    // The operator's abstract definitions, which the library's agents ask
    // for, are read from the map's directory or the operated repository —
    // here both this workspace — and only through a handle rooted there, so
    // they are copied in, never linked (decision 0065; design D2).
    std::fs::create_dir(ws.path().join("capabilities")).unwrap();
    for entry in std::fs::read_dir(root.join("capabilities")).unwrap() {
        let entry = entry.unwrap();
        let to = ws.path().join("capabilities").join(entry.file_name());
        std::fs::copy(entry.path(), to).unwrap();
    }
    let realm = |path: &str, boundary: &str| {
        json!({"schema": "forge.realms/v4", "journal": "forge.db", "realms": [{
            "name": "here", "path": path, "default_branch": "main",
            "dialect": "openspec", "boundary": boundary}]})
    };
    let maps = [
        ("no map", None),
        ("namespace here", Some(realm(".", "namespace"))),
        ("harness here", Some(realm(".", "harness"))),
        ("harness elsewhere", Some(realm("elsewhere", "harness"))),
    ];
    let mut named = Vec::new();
    for entry in std::fs::read_dir(root.join("recipes")).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        named.push((root.join("recipes"), name));
    }
    let mut digests = std::collections::BTreeSet::new();
    let mut refused = std::collections::BTreeMap::new();
    for (label, map) in &maps {
        let _ = std::fs::remove_file(ws.path().join("realms.json"));
        if let Some(map) = map {
            std::fs::write(ws.path().join("realms.json"), map.to_string()).unwrap();
        }
        for (library, name) in &named {
            let bundle = library.join(name);
            let compiled = ws.brokkr(&["compile", "--bundle", bundle.to_str().unwrap()]);
            let dir = library.to_str().unwrap();
            let shown = ws.brokkr(&["recipes", "show", name, "--dir", dir]);
            assert_eq!(as_compiled(shown), compiled, "{name} under {label}");
            match compiled {
                // A compiled view carries decision 0050's sweep on stderr
                // (#429), the same for both verbs by the assertion above.
                (Some(0), view, _sweep) if !view.is_empty() => {
                    let view: Value = serde_json::from_str(&view).unwrap();
                    let digest = view["digest"].as_str().unwrap().to_string();
                    let hex = |b: u8| b.is_ascii_digit() || (b'a'..=b'f').contains(&b);
                    assert!(digest.len() == 64 && digest.bytes().all(hex), "{digest}");
                    digests.insert((name.clone(), digest));
                }
                (Some(1), view, line) if view.is_empty() => {
                    refused.insert((*label, name.as_str()), line);
                }
                other => panic!("{name} under {label}: neither a view nor a refusal: {other:?}"),
            }
        }
    }
    assert_eq!(refused, self::refused());
    // The maps are not all alike to every recipe: a boxed one compiled
    // under `harness` pins another word, and so another digest.
    assert!(digests.len() > named.len(), "{digests:?}");
}
