//! Decision 0083, slice A (#634): an adopter's Brokkr lives in one
//! `.brokkr/` folder at its repository's root. Discovery from anywhere in
//! the repository, the defaults that follow from it, and the old layouts
//! refused with the moves to make. `init` writing the folder and doctor's
//! folder line are slice B's; these tests plant the folder by hand.
//!
//! Every repository here is a fresh `git init` in a temporary directory
//! that sits in no checkout, so discovery reads that repository alone.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{json, Value};

/// The table every bundle here runs: implement, the protected review, done.
fn policy() -> String {
    let rule = |id: &str, from: &str, result: &str, next: &str| {
        let reason = format!("{from} ended {result}");
        json!({"id": id, "from": from, "result": result, "next": next, "reason": reason})
    };
    let rules = [
        rule("IMPL-OK", "implement", "complete", "review"),
        rule("REVIEW-CLEAN", "review", "clean", "done"),
    ];
    let phases = ["implement", "review", "done"];
    let table = json!({"schema": "forge.phase-machine/v1", "phases": phases,
                       "initial": "implement", "terminal": ["done"], "rules": rules});
    table.to_string()
}

/// What one invocation left: its exit code, stdout and stderr.
struct Ran {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl Ran {
    fn first_line(&self) -> &str {
        self.stderr.lines().next().unwrap_or_default()
    }
}

/// A fresh repository, and a directory beside it that is in no
/// repository: the fake driver's script and state, and every engine's
/// TMPDIR, so nothing a run writes outside `.brokkr/state/` lands in the
/// repository.
struct Repo {
    _dir: tempfile::TempDir,
    _aside: tempfile::TempDir,
    root: PathBuf,
    aside: PathBuf,
}

impl Repo {
    fn new() -> Repo {
        let dir = tempfile::tempdir().unwrap();
        let aside = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap().join("repo");
        std::fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "-q"]);
        Repo {
            root,
            aside: aside.path().canonicalize().unwrap(),
            _dir: dir,
            _aside: aside,
        }
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    fn write(&self, relative: &str, text: &str) -> PathBuf {
        let path = self.path(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        path
    }

    /// `brokkr` standing in `dir`, relative to the root, created first.
    fn command(&self, dir: &str) -> Command {
        let cwd = self.path(dir);
        std::fs::create_dir_all(&cwd).unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_brokkr"));
        command.current_dir(cwd).env("TMPDIR", &self.aside);
        command
    }

    /// `brokkr <args>` from `dir`, relative to the root, with `stdin`.
    fn brokkr_with(&self, dir: &str, args: &[&str], stdin: &str) -> Ran {
        let mut child = self
            .command(dir)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(stdin.as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        let text = |bytes: Vec<u8>| String::from_utf8(bytes).unwrap();
        let code = out.status.code();
        let (stdout, stderr) = (text(out.stdout), text(out.stderr));
        Ran {
            code,
            stdout,
            stderr,
        }
    }

    fn brokkr(&self, dir: &str, args: &[&str]) -> Ran {
        self.brokkr_with(dir, args, "")
    }

    /// A fake-driver bundle named `name` at `relative`, its implementer
    /// answering each attempt with `answers` in turn and its reviewer
    /// clean.
    fn bundle(&self, relative: &str, name: &str, answers: Value) {
        let script = self.aside.join(format!("{name}.script.json"));
        let review = json!([{"behavior": "succeed", "result": {"result": "clean"}}]);
        let seats = json!({"seats": {"implement": answers, "review": review}});
        std::fs::write(&script, seats.to_string()).unwrap();
        let state = self.aside.join(format!("{name}.state"));
        std::fs::create_dir_all(&state).unwrap();
        let dir = self.root.join(relative);
        std::fs::create_dir_all(dir.join("roles")).unwrap();
        std::fs::write(dir.join("policy.json"), policy()).unwrap();
        std::fs::write(dir.join("roles/role.md"), "# role\n").unwrap();
        let seat = |results: Value| {
            json!({
                "role": "roles/role.md",
                "results": results,
                "driver": {"command": [
                    env!("CARGO_BIN_EXE_brokkr"), "fake-driver",
                    "--script", script, "--state", state,
                ]},
            })
        };
        let bundle = json!({
            "name": name,
            "policy": "policy.json",
            "seats": {"implement": seat(json!(["complete"])), "review": seat(json!(["clean"]))},
        });
        std::fs::write(dir.join("bundle.json"), bundle.to_string()).unwrap();
    }

    /// The folder planted by hand, as `init` will write it: its bundle and
    /// the ignore file that keeps its state out of git.
    fn folder(&self) {
        self.bundle(".brokkr", "folder", completes());
        self.write(".brokkr/.gitignore", "state/\n");
    }

    /// A recipe library's mark: a bundle under `recipes/`.
    fn library(&self) {
        self.bundle("recipes/x", "library", completes());
    }

    /// Every file under the root but `.git`, with its bytes.
    fn files(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        let mut files = BTreeMap::new();
        let mut pending = vec![self.root.clone()];
        while let Some(dir) = pending.pop() {
            for entry in std::fs::read_dir(&dir).unwrap().flatten() {
                let path = entry.path();
                let kind = entry.file_type().unwrap();
                if path.ends_with(".git") {
                    continue;
                }
                match kind.is_dir() {
                    true => pending.push(path),
                    false => {
                        let bytes = std::fs::read(&path).unwrap_or_default();
                        files.insert(path.strip_prefix(&self.root).unwrap().into(), bytes);
                    }
                }
            }
        }
        files
    }
}

fn completes() -> Value {
    json!([{"behavior": "succeed", "result": {"result": "complete"}}])
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Commit what `root`'s index holds.
fn commit(root: &Path) {
    let author = ["-c", "user.name=t", "-c", "user.email=t@example.invalid"];
    let commit = ["-c", "commit.gpgsign=false", "commit", "-q", "-m", "init"];
    git(root, &[&author[..], &commit[..]].concat());
}

/// The run id a `brokkr run` said it started.
fn started(ran: &Ran) -> String {
    ran.stderr
        .lines()
        .find_map(|line| line.strip_prefix("run started: "))
        .unwrap_or_else(|| panic!("a run started: {}", ran.stderr))
        .trim()
        .to_string()
}

/// The ids `brokkr runs --json` lists.
fn listed(ran: &Ran) -> Vec<String> {
    let runs: Value = serde_json::from_str(&ran.stdout).unwrap();
    let rows = runs["runs"]
        .as_array()
        .unwrap_or_else(|| panic!("rows: {runs}"));
    rows.iter()
        .map(|row| row["run_id"].as_str().unwrap().to_string())
        .collect()
}

fn no_place(cwd: &Path, root: &Path) -> String {
    format!(
        "error: no .brokkr/ folder from {} up to {}, and {} is not a recipe library (decision \
         0083); nothing was created: run `brokkr init .` from {}",
        cwd.display(),
        root.display(),
        root.display(),
        root.display()
    )
}

const ROOT_LAYOUT: &str = "error: this repository has Brokkr's old root layout and no .brokkr/ \
                           folder (decision 0083); nothing was opened, and Brokkr moves nothing";

const STATE_ONLY: &str = "error: this repository has Brokkr state in .forge/ and no .brokkr/ \
                          folder (decision 0083); nothing was opened, and Brokkr moves nothing";

fn half_moved(root: &Path, left: &str) -> String {
    format!(
        "{}/.brokkr/ is this repository's folder, but {}/{left} still stands at the root \
         (decision 0083); move or remove it",
        root.display(),
        root.display()
    )
}

#[test]
fn discovery_finds_the_folder_from_a_subdirectory() {
    let repo = Repo::new();
    repo.folder();
    let shipped = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/dsh.json");
    std::fs::create_dir_all(repo.path(".brokkr/adapters")).unwrap();
    std::fs::copy(shipped, repo.path(".brokkr/adapters/dsh.json")).unwrap();

    let ran = repo.brokkr("src/deep", &["run", "--feature", "x", "--no-view"]);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let run = started(&ran);
    assert!(repo.path(".brokkr/state/forge.db").is_file());
    assert!(repo.path(".brokkr/state/results").is_dir());
    assert!(!repo.path(".forge").exists() && !repo.path("src/deep/.forge").exists());
    assert_eq!(git(&repo.root, &["status", "--porcelain"]), "?? .brokkr/\n");
    let runs = repo.brokkr("src", &["runs", "--json"]);
    assert_eq!(listed(&runs), std::slice::from_ref(&run));
    let ledger = repo.brokkr("src", &["ledger", "--run", &run, "--repo", "."]);
    let written = repo.path(&format!(".brokkr/state/ledger/{run}.md"));
    assert_eq!(ledger.stdout, format!("{}\n", written.display()));
    assert!(written.is_file() && !repo.path("src/.forge").exists());
    let record = repo.path(".brokkr/state/muninn.ndjson");
    let ran = repo.brokkr("src/deep", &["muninn", "list"]);
    let none = format!("no proposals recorded in {}\n", record.display());
    assert_eq!((ran.code, ran.stdout), (Some(0), none));

    // Doctor, from the same place, reads the folder's journal, agents,
    // adapters and secrets store: a credential the folder's adapter
    // declares, set in the environment, is checked against the folder's
    // store.
    let mut doctor = repo.command("src/deep");
    doctor.arg("doctor").env("SPARK_API_KEY", "set");
    let printed = String::from_utf8(doctor.output().unwrap().stdout).unwrap();
    let state = repo.path(".brokkr/state");
    let lines = [
        format!(
            "database: {} opens (WAL, append-only triggers)",
            state.join("forge.db").display()
        ),
        format!("agents: {}:", repo.path(".brokkr/agents").display()),
        format!(
            "bindings store at {} and not",
            state.join("secrets.env").display()
        ),
    ];
    for line in lines {
        assert!(printed.contains(&line), "{line} in {printed}");
    }
}

#[test]
fn discovery_stops_at_the_git_root() {
    let repo = Repo::new();
    repo.folder();
    let inner = repo.path("vendor/inner");
    std::fs::create_dir_all(&inner).unwrap();
    git(&inner, &["init", "-q"]);
    let ran = repo.brokkr(
        "vendor/inner",
        &["run", "--bundle", "../../.brokkr", "--feature", "x"],
    );
    assert_eq!(ran.code, Some(1));
    assert_eq!(ran.stderr, format!("{}\n", no_place(&inner, &inner)));
    assert!(!repo.path(".brokkr/state").exists());
}

#[test]
fn a_nested_folder_refuses() {
    let repo = Repo::new();
    repo.folder();
    std::fs::create_dir_all(repo.path("src/.brokkr")).unwrap();
    let ran = repo.brokkr("src/deep", &["run", "--feature", "x"]);
    assert_eq!(ran.code, Some(1));
    assert_eq!(
        ran.stderr,
        format!(
            "error: {}/src/.brokkr is not at this repository's root {} (decision 0083); Brokkr \
             reads only {}/.brokkr: move or remove it\n",
            repo.root.display(),
            repo.root.display(),
            repo.root.display()
        )
    );
    assert!(!repo.path(".brokkr/state").exists());
}

#[test]
fn a_folder_that_is_not_a_directory_refuses() {
    let repo = Repo::new();
    repo.write(".brokkr", "not a folder\n");
    let ran = repo.brokkr("", &["run", "--feature", "x"]);
    assert_eq!(ran.code, Some(1));
    assert_eq!(
        ran.stderr,
        format!(
            "error: {}/.brokkr is not a directory (decision 0083); Brokkr's folder must be one: \
             move or remove it\n",
            repo.root.display()
        )
    );
}

#[test]
fn no_folder_outside_a_library_names_init() {
    let repo = Repo::new();
    repo.bundle("b", "plain", completes());
    let ran = repo.brokkr("", &["run", "--bundle", "b", "--feature", "x"]);
    refused_with_no_place(&repo, &ran);

    // `--db` supplies the journal alone: the ledger's run-local place
    // has none here, and doctor reports the journal's as `none`.
    refused_with_no_place(&repo, &ledger_of_a_library_run(&repo));
    let ran = repo.brokkr("", &["doctor"]);
    let none = no_place(&repo.root, &repo.root).replacen("error: ", "none: ", 1);
    let line = format!("warn     database: {none}");
    assert!(
        ran.stdout.lines().any(|printed| printed == line),
        "{line} in {}",
        ran.stdout
    );
    assert!(!repo.path(".forge").exists() && !repo.path(".brokkr").exists());
}

/// The no-place refusal from the root, with neither `.forge/` nor
/// `.brokkr/` created.
fn refused_with_no_place(repo: &Repo, ran: &Ran) {
    let refusal = no_place(&repo.root, &repo.root);
    assert_eq!(
        (ran.code, ran.stderr.as_str()),
        (Some(1), &*format!("{refusal}\n"))
    );
    assert!(!repo.path(".forge").exists() && !repo.path(".brokkr").exists());
}

#[test]
fn secrets_set_without_a_folder_refuses() {
    let repo = Repo::new();
    let ran = repo.brokkr_with("", &["secrets", "set", "NAME"], "value\n");
    refused_with_no_place(&repo, &ran);

    let flagged = ["secrets", "set", "NAME", "--secrets-file", "named.env"];
    let ran = repo.brokkr_with("", &flagged, "value\n");
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert_eq!(
        repo.files().into_keys().collect::<Vec<_>>(),
        [PathBuf::from("named.env")]
    );
}

#[test]
fn an_explicit_flag_outranks_discovery() {
    let repo = Repo::new();
    repo.folder();
    let (bundled, recipe_run) = a_named_bundle_runs(&repo);

    // --db: the named journal is the one written, the folder's is not.
    let ran = repo.brokkr(
        "",
        &["run", "--db", "named.db", "--feature", "x", "--no-view"],
    );
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let in_named = started(&ran);
    let runs = repo.brokkr("", &["runs", "--json", "--db", "named.db"]);
    assert_eq!(listed(&runs), std::slice::from_ref(&in_named));
    let folder_runs = listed(&repo.brokkr("", &["runs", "--json"]));
    assert!(!folder_runs.contains(&in_named), "{folder_runs:?}");
    assert!(folder_runs.contains(&bundled) && folder_runs.contains(&recipe_run));

    a_named_map_is_read(&repo, &bundled);
    a_named_library_is_read(&repo);
    a_flag_reads_where_it_was_typed(&repo);
}

/// A relative path a flag names in a subdirectory is under that
/// subdirectory (ruling 2's step 1), in the launch a queue entry holds as
/// in a direct run, though the folder's root is the workspace both read
/// a relative path against.
fn a_flag_reads_where_it_was_typed(repo: &Repo) {
    let deep = repo.path("src/deep");
    let map = json!({"schema": "forge.realms/v1", "journal": "deep.db",
                     "realms": [{"name": "here", "path": "../../..", "default_branch": "main"}]});
    repo.write("src/deep/maps/realms.json", &map.to_string());
    let queue = [
        "queue",
        "add",
        "--reason",
        "here",
        "--db",
        "q.db",
        "--feature",
        "x",
    ];
    let flags = [
        "--bundle",
        "local",
        "--repo",
        ".",
        "--secrets-file",
        "local.env",
    ];
    let dispatched = [&queue[..], &flags, &["--dispatch", "d.json"]].concat();
    let mapped = [
        &queue[..],
        &["--recipe", "r", "--realms", "maps/realms.json"],
    ]
    .concat();
    for added in [dispatched, mapped] {
        let ran = repo.brokkr("src/deep", &added);
        assert_eq!(ran.code, Some(0), "{added:?}: {}", ran.stderr);
    }
    let ran = repo.brokkr("src/deep", &["queue", "list", "--json", "--db", "q.db"]);
    let rows: Value = serde_json::from_str(&ran.stdout).unwrap();
    let at = |path: &str| json!(deep.join(path));
    let (first, second) = (&rows[0]["launch"], &rows[1]["launch"]);
    assert_eq!(
        [
            &first["bundle"],
            &first["repo"],
            &first["secrets"],
            &first["dispatch"]
        ],
        [
            &json!({"dir": deep.join("local")}),
            &at("."),
            &at("local.env"),
            &at("d.json")
        ]
    );
    assert_eq!(
        [
            &second["bundle"]["recipe"]["recipes_dir"],
            &second["map"]["named"]["source"]
        ],
        [&at("recipes"), &at("maps/realms.json")]
    );
}

/// `--bundle` and `--recipe` beside a discovered folder: the run is the
/// named bundle's, whose fake driver alone keeps state.
fn a_named_bundle_runs(repo: &Repo) -> (String, String) {
    let seated = |name: &str| {
        let state = repo.aside.join(format!("{name}.state"));
        std::fs::read_dir(state).unwrap().count() > 0
    };
    repo.bundle("elsewhere", "elsewhere", completes());
    let ran = repo.brokkr(
        "",
        &[
            "run",
            "--bundle",
            "elsewhere",
            "--feature",
            "x",
            "--no-view",
        ],
    );
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let bundled = started(&ran);
    assert_eq!((seated("elsewhere"), seated("folder")), (true, false));
    repo.bundle("shelf/named", "named", completes());
    let recipe = [
        "run",
        "--recipe",
        "named",
        "--recipes-dir",
        "shelf",
        "--feature",
        "x",
    ];
    let ran = repo.brokkr("", &[&recipe[..], &["--no-view"]].concat());
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let recipe_run = started(&ran);
    assert_eq!((seated("named"), seated("folder")), (true, false));
    (bundled, recipe_run)
}

/// `--realms` beside the folder's own map: the named map's journal is the
/// one opened, where the folder's map is adopted with its notice.
fn a_named_map_is_read(repo: &Repo, bundled: &String) {
    repo.write(
        ".brokkr/realms.json",
        &json!({"schema": "forge.realms/v1", "journal": "state/forge.db",
                "realms": [{"name": "here", "path": "..", "default_branch": "main"}]})
        .to_string(),
    );
    repo.write(
        "maps/realms.json",
        &json!({"schema": "forge.realms/v1", "journal": "mapped.db",
                "realms": [{"name": "here", "path": "..", "default_branch": "main"}]})
        .to_string(),
    );
    let ambient = repo.brokkr("", &["runs", "--json"]);
    let adopted = format!(
        "note: the map {}/.brokkr/realms.json found in this workspace is adopted (journal \
         unchanged); --realms names it explicitly\n",
        repo.root.display()
    );
    assert_eq!(ambient.stderr, adopted);
    assert!(listed(&ambient).contains(bundled));
    let mapped = ["runs", "--json", "--realms", "maps/realms.json"];
    let ran = repo.brokkr("", &mapped);
    assert_eq!(ran.code, Some(1));
    assert_eq!(
        ran.stderr,
        format!(
            "error: journal does not exist: {}; a read never creates one\n",
            repo.path("maps/mapped.db").display()
        )
    );
}

/// `--agents-dir`, `--adapters-dir` and `--secrets-file` beside the
/// folder's own: the named library, adapters and store are read.
fn a_named_library_is_read(repo: &Repo) {
    let agents = |extra: &[&str]| repo.brokkr("", &[&["agents", "list"][..], extra].concat());
    let (named, folder) = (agents(&["--agents-dir", "own/agents"]), agents(&[]));
    let absent = |library: &str| {
        format!("error: agent library {library}: No such file or directory (os error 2)\n")
    };
    assert_eq!(named.stderr, absent("own/agents"));
    let folder_agents = repo.path(".brokkr/agents");
    assert_eq!(folder.stderr, absent(folder_agents.to_str().unwrap()));
    let probe = [
        "probe",
        "harness",
        "--adapter",
        "nope",
        "--adapters-dir",
        "own/adapters",
    ];
    std::fs::create_dir_all(repo.path("own/adapters")).unwrap();
    let ran = repo.brokkr("", &probe);
    assert_eq!(ran.stderr, "error: no adapter 'nope' under own/adapters\n");

    // --secrets-file: the named store is the one listed.
    let named = repo.write("own/secrets.env", "OWN=1\n");
    std::fs::set_permissions(&named, std::os::unix::fs::PermissionsExt::from_mode(0o600)).unwrap();
    let ran = repo.brokkr_with(".brokkr", &["secrets", "set", "FOLDER"], "value\n");
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let ran = repo.brokkr(
        "",
        &["secrets", "list", "--secrets-file", "own/secrets.env"],
    );
    assert_eq!((ran.code, ran.stdout.as_str()), (Some(0), "OWN\n"));
    let ran = repo.brokkr("", &["secrets", "list"]);
    assert_eq!((ran.code, ran.stdout.as_str()), (Some(0), "FOLDER\n"));
}

#[test]
fn run_without_a_folder_names_init() {
    let repo = Repo::new();
    repo.library();
    let ran = repo.brokkr("recipes", &["run", "--feature", "x"]);
    assert_eq!(ran.code, Some(1));
    assert_eq!(
        ran.stderr,
        format!(
            "error: no .brokkr/ folder from {} up to {} (decision 0083), so there is no default \
             bundle: name one with --bundle or --recipe, or run `brokkr init .` from {}\n",
            repo.path("recipes").display(),
            repo.root.display(),
            repo.root.display()
        )
    );
}

/// `init`'s old output, by hand, with a journal, its write-ahead log and a
/// secrets store, the ignore file committed as `init` told the operator to.
fn old_root_layout(repo: &Repo) {
    repo.bundle("", "old", completes());
    let map = json!({"schema": "forge.realms/v1", "journal": ".forge/forge.db",
                     "realms": [{"name": "old", "path": ".", "default_branch": "main"}]});
    repo.write("realms.json", &map.to_string());
    repo.write("agents/implementer.json", "{}");
    repo.write("scripts/verify-seat.sh", "#!/bin/sh\n");
    repo.write(".forge/.gitignore", "*\n!.gitignore\n");
    repo.write(".forge/forge.db", "journal");
    repo.write(".forge/forge.db-wal", "log");
    repo.write(".forge/secrets.env", "NAME=value\n");
    git(&repo.root, &["add", ".forge/.gitignore"]);
    commit(&repo.root);
}

#[test]
fn old_root_layout_refuses_with_the_moves() {
    let repo = Repo::new();
    old_root_layout(&repo);
    let before = repo.files();
    let ran = repo.brokkr("", &["run", "--feature", "x"]);
    assert_eq!(ran.code, Some(1));
    assert_eq!(ran.first_line(), ROOT_LAYOUT);
    let moves: Vec<&str> = ran.stderr.lines().skip(1).collect();
    assert_eq!(
        moves[..7],
        [
            "make these moves, then run `brokkr doctor` to check the result:",
            "  mkdir -p .brokkr/state",
            "  mv .forge/forge.db .brokkr/state/forge.db",
            "  mv .forge/forge.db-wal .brokkr/state/forge.db-wal",
            "  mv .forge/secrets.env .brokkr/state/secrets.env",
            "  git rm .forge/.gitignore",
            "  printf 'state/\\n' > .brokkr/.gitignore",
        ]
    );
    assert!(!repo.path(".brokkr").exists());
    assert_eq!(repo.files(), before);
}

#[test]
fn state_only_layout_refuses() {
    let repo = Repo::new();
    repo.write(".forge/forge.db", "journal");
    repo.bundle("../elsewhere", "elsewhere", completes());
    let before = repo.files();
    let ran = repo.brokkr("", &["run", "--bundle", "../elsewhere", "--feature", "x"]);
    assert_eq!((ran.code, ran.first_line()), (Some(1), STATE_ONLY));
    let ran = repo.brokkr("", &["init", "."]);
    assert_eq!((ran.code, ran.first_line()), (Some(1), STATE_ONLY));
    assert_eq!(repo.files(), before);
}

/// The moves an old-layout refusal lists, one per indented line.
fn listed_moves(ran: &Ran) -> Vec<&str> {
    let lines = ran.stderr.lines();
    lines.filter_map(|line| line.strip_prefix("  ")).collect()
}

/// Each listed move run by `sh` at the root, as an operator pastes it.
fn make(repo: &Repo, moves: &[&str]) {
    for line in moves {
        let status = Command::new("sh")
            .args(["-c", line])
            .current_dir(&repo.root)
            .status()
            .unwrap();
        assert!(status.success(), "{line}");
    }
}

#[test]
fn secrets_only_layout_refuses() {
    let repo = Repo::new();
    let store = repo.write(".forge/secrets.env", "NAME=value\n");
    std::fs::set_permissions(&store, std::os::unix::fs::PermissionsExt::from_mode(0o600)).unwrap();
    let ran = repo.brokkr("", &["secrets", "list"]);
    assert_eq!((ran.code, ran.first_line()), (Some(1), STATE_ONLY));
    assert!(!repo.path(".brokkr").exists());

    // The listed moves, made as listed, leave a folder the store is read
    // from. (`brokkr init .` writing the scaffold beside it is slice B's.)
    let moves = listed_moves(&ran);
    assert_eq!(
        moves,
        [
            "mkdir -p .brokkr/state",
            "mv .forge/secrets.env .brokkr/state/secrets.env",
            "rmdir .forge",
            "printf 'state/\\n' > .brokkr/.gitignore",
        ]
    );
    make(&repo, &moves);
    let ran = repo.brokkr("", &["secrets", "list"]);
    assert_eq!(
        (ran.code, ran.stdout.as_str()),
        (Some(0), "NAME\n"),
        "{}",
        ran.stderr
    );
}

#[test]
fn a_library_keeps_its_journal() {
    let repo = Repo::new();
    repo.library();
    repo.write(".forge/forge.db", "");
    let run = library_run(&repo);
    let listed_in = listed(&repo.brokkr("", &["runs", "--json", "--db", ".forge/forge.db"]));
    assert_eq!(listed_in, [run]);
    assert!(!repo.path(".brokkr").exists());
}

/// The library's own bundle run with no other flag: the run it started.
fn library_run(repo: &Repo) -> String {
    let run = [
        "run",
        "--bundle",
        "recipes/x",
        "--feature",
        "x",
        "--no-view",
    ];
    let ran = repo.brokkr("", &run);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    started(&ran)
}

/// The operator's ruling of 2026-10-13: a library keeps its layout, a
/// root `bundle.json` beside `recipes/` included.
#[test]
fn a_library_with_a_root_bundle_is_not_refused() {
    let repo = Repo::new();
    repo.library();
    repo.bundle("", "root", completes());
    repo.write(".forge/secrets.env", "");
    let run = library_run(&repo);
    let runs = listed(&repo.brokkr("", &["runs", "--json"]));
    assert_eq!(runs, std::slice::from_ref(&run));
    assert!(repo.path(".forge/forge.db").is_file());

    // A library that also holds a folder still keeps its layout: it is
    // refused as nothing, and its journal never moves to the folder's.
    std::fs::create_dir(repo.path(".brokkr")).unwrap();
    let ran = repo.brokkr("", &["runs", "--json"]);
    assert_eq!(listed(&ran), [run], "{}", ran.stderr);
    assert!(!repo.path(".brokkr/state").exists());
}

/// `brokkr ledger --repo .` in `repo`, of a run a library beside it
/// journaled, named by `--db`: a flag that supplies the journal alone.
fn ledger_of_a_library_run(repo: &Repo) -> Ran {
    let library = Repo::new();
    library.library();
    let run = library_run(&library);
    let db = library.path(".forge/forge.db");
    let ledger = ["ledger", "--run", &run, "--db", db.to_str().unwrap()];
    repo.brokkr("", &[&ledger[..], &["--repo", "."]].concat())
}

#[test]
fn half_moved_layout_refuses() {
    let repo = Repo::new();
    repo.folder();
    repo.write("bundle.json", "{}");
    let ran = repo.brokkr("", &["runs"]);
    assert_eq!(ran.code, Some(1));
    assert_eq!(
        ran.stderr,
        format!("error: {}\n", half_moved(&repo.root, "bundle.json"))
    );

    std::fs::remove_file(repo.path("bundle.json")).unwrap();
    repo.write(".forge/secrets.env", "NAME=value\n");
    let ran = repo.brokkr("", &["runs"]);
    assert_eq!(
        ran.stderr,
        format!("error: {}\n", half_moved(&repo.root, ".forge"))
    );

    repo.write(".forge/forge.db", "");
    repo.write(".brokkr/state/forge.db", "");
    let ran = repo.brokkr("", &["runs"]);
    assert_eq!(
        ran.stderr,
        format!(
            "error: {}\nboth {}/.forge/forge.db and {}/.brokkr/state/forge.db hold a journal, \
             and journals never merge (decision 0026 ruling 5): keep one\n",
            half_moved(&repo.root, ".forge"),
            repo.root.display(),
            repo.root.display()
        )
    );
    // A journal at the retired place alone is no second journal.
    std::fs::remove_file(repo.path(".brokkr/state/forge.db")).unwrap();
    let ran = repo.brokkr("", &["runs"]);
    assert_eq!(
        ran.stderr,
        format!("error: {}\n", half_moved(&repo.root, ".forge"))
    );
}

/// A `.forge` linked to `name` beside the repository, holding a journal,
/// and the state-only refusal `brokkr runs` gives it.
fn linked_forge(repo: &Repo, name: &str) -> (PathBuf, Ran) {
    let elsewhere = repo.aside.join(name);
    std::fs::create_dir_all(&elsewhere).unwrap();
    std::fs::write(elsewhere.join("forge.db"), "journal").unwrap();
    std::os::unix::fs::symlink(&elsewhere, repo.path(".forge")).unwrap();
    let ran = repo.brokkr("", &["runs"]);
    assert_eq!((ran.code, ran.first_line()), (Some(1), STATE_ONLY));
    (elsewhere, ran)
}

#[test]
fn a_linked_entry_is_the_entry() {
    let repo = Repo::new();
    let (elsewhere, _) = linked_forge(&repo, "state");
    assert_eq!(
        std::fs::read(elsewhere.join("forge.db")).unwrap(),
        b"journal"
    );

    std::fs::remove_file(repo.path(".forge")).unwrap();
    std::os::unix::fs::symlink(repo.aside.join("absent.json"), repo.path("bundle.json")).unwrap();
    let ran = repo.brokkr("", &["runs"]);
    assert_eq!((ran.code, ran.first_line()), (Some(1), ROOT_LAYOUT));
    assert!(!repo.path(".forge").exists() && !repo.path(".brokkr").exists());
}

#[test]
fn an_explicit_db_reaches_the_old_journal() {
    let repo = Repo::new();
    old_root_layout(&repo);
    // A journal and nothing else under `.forge/`: a root bundle that holds
    // a secrets store does not compile, before and after decision 0083.
    for planted in ["forge.db", "forge.db-wal", "secrets.env"] {
        std::fs::remove_file(repo.path(".forge").join(planted)).unwrap();
    }
    repo.bundle("", "old", json!([{"behavior": "fail", "error": "boom"}]));
    let db = ["--db", ".forge/forge.db"];
    let old = ["run", "--bundle", ".", "--feature", "x", "--no-view"];
    let ran = repo.brokkr("", &[&old[..], &db].concat());
    let run = started(&ran);
    assert_eq!(
        ran.code,
        Some(2),
        "the run parked unfinished: {}",
        ran.stderr
    );
    let refused = repo.brokkr("", &["conclude", "--run", &run, "--reason", "x"]);
    assert_eq!((refused.code, refused.first_line()), (Some(1), ROOT_LAYOUT));
    let concluded = [&["conclude", "--run", &run, "--reason", "x"][..], &db].concat();
    let ran = repo.brokkr("", &concluded);
    let summary: Value = serde_json::from_str(&ran.stdout).unwrap();
    assert_eq!((ran.code, &summary["status"]), (Some(3), &json!("stopped")));
    // The ledger a ship seat writes before the move, in the layout's own
    // `.forge/`.
    let ledger = [&["ledger", "--run", &run, "--repo", "."][..], &db].concat();
    let written = format!("./.forge/ledger/{run}.md");
    let ran = repo.brokkr("", &ledger);
    assert_eq!((ran.code, ran.stdout), (Some(0), format!("{written}\n")));
    assert!(repo.path(&written).is_file());
    assert!(!repo.path(".brokkr").exists());
}

#[test]
fn state_never_moves_the_bundle() {
    let repo = Repo::new();
    repo.folder();
    let digest = || {
        let ran = repo.brokkr("", &["compile", "--bundle", ".brokkr"]);
        let compiled: Value = serde_json::from_str(&ran.stdout).unwrap();
        compiled["digest"].as_str().unwrap().to_string()
    };
    let before = digest();
    repo.write(".brokkr/state/forge.db", "a journal");
    repo.write(".brokkr/state/results/r.json", "{}");
    assert_eq!(digest(), before);
    repo.write(".brokkr/roles/other.md", "a pinned file\n");
    assert_ne!(digest(), before);
}

/// Both seats of the bundle at `relative` hire a role under its `state/`.
fn role_under_state(repo: &Repo, relative: &str) {
    repo.write(&format!("{relative}/state/role.md"), "# role\n");
    let path = repo.path(&format!("{relative}/bundle.json"));
    let mut bundle: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    for seat in ["implement", "review"] {
        bundle["seats"][seat]["role"] = json!("state/role.md");
    }
    std::fs::write(&path, bundle.to_string()).unwrap();
}

/// The digest `brokkr compile --bundle <relative>` prints.
fn digest(repo: &Repo, relative: &str) -> String {
    let ran = repo.brokkr("", &["compile", "--bundle", relative]);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let compiled: Value = serde_json::from_str(&ran.stdout).unwrap();
    compiled["digest"].as_str().unwrap().to_string()
}

/// Only a folder bundle skips its `state/`: a library's recipe pins a
/// top-level `state/` and reads a role there exactly as before decision
/// 0083 (round 634b's third finding).
#[test]
fn a_library_recipe_pins_its_state() {
    let repo = Repo::new();
    repo.library();
    repo.write("recipes/x/state/data.txt", "one\n");
    let before = digest(&repo, "recipes/x");
    repo.write("recipes/x/state/data.txt", "two\n");
    assert_ne!(digest(&repo, "recipes/x"), before);
    role_under_state(&repo, "recipes/x");
    let ran = repo.brokkr("", &["compile", "--bundle", "recipes/x"]);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
}

/// A library's recipe named `.brokkr` is an ordinary bundle: it pins its
/// `state/` and composes a base, because the folder is the layout's one
/// judgement, never a directory's name (round 634k's second finding).
#[test]
fn a_library_recipe_named_brokkr_is_an_ordinary_bundle() {
    let repo = Repo::new();
    repo.library();
    repo.bundle("recipes/.brokkr", "dot", completes());
    repo.write("recipes/.brokkr/state/data.txt", "one\n");
    let before = digest(&repo, "recipes/.brokkr");
    repo.write("recipes/.brokkr/state/data.txt", "two\n");
    assert_ne!(digest(&repo, "recipes/.brokkr"), before);
    let overlay = json!({"name": "dot", "extends": "x"});
    repo.write("recipes/.brokkr/bundle.json", &overlay.to_string());
    let ran = repo.brokkr("", &["compile", "--bundle", "recipes/.brokkr"]);
    let compiled: Value = serde_json::from_str(&ran.stdout).unwrap();
    let base = &compiled["composed_from"][0]["recipe"];
    assert_eq!(
        (ran.code, base),
        (Some(0), &json!("library")),
        "{}",
        ran.stderr
    );
}

/// A folder bundle's role under `state/` is refused, by the folder's own
/// reason rather than operator configuration's.
#[test]
fn a_folder_bundle_skips_its_state_by_its_own_reason() {
    let repo = Repo::new();
    repo.folder();
    role_under_state(&repo, ".brokkr");
    let ran = repo.brokkr("", &["compile", "--bundle", ".brokkr"]);
    assert_eq!(ran.code, Some(1));
    assert_eq!(
        ran.stderr,
        format!(
            "error: bundle: {}/.brokkr/bundle.json: seat 'implement' names role 'state/role.md', \
             which stands under 'state' — a top-level name the bundle's file walk does not pin, \
             because it holds the folder's runtime state, which a run writes. A charter there \
             could change what the seat is told without moving the bundle's identity, so it is \
             refused; move it to a path the bundle pins, such as 'roles/' (decision 0066 ruling \
             5)\n",
            repo.root.display()
        )
    );
}

/// The default map's place holding no file refuses by name, for a launch
/// and a queued one alike, before any journal is created (round 634b's
/// first finding); a library's `./realms.json` is held to the same rule.
#[test]
fn a_default_map_that_is_no_file_refuses() {
    let repo = Repo::new();
    repo.folder();
    let map = repo.path(".brokkr/realms.json");
    let not_a_file = format!(
        "error: {} is not a file (decision 0083): the default realms map is a file, or a link to \
         one; move or remove it\n",
        map.display()
    );
    let dangling = format!(
        "error: {} is a link to nothing (decision 0083 ruling 4: a linked entry is the entry); \
         mend or remove the link\n",
        map.display()
    );
    let run = ["run", "--feature", "x", "--no-view"];
    let queue = ["queue", "add", "--feature", "x", "--reason", "probe"];
    std::fs::create_dir(&map).unwrap();
    for args in [&run[..], &queue] {
        let ran = repo.brokkr("src", args);
        assert_eq!((ran.code, ran.stderr.as_str()), (Some(1), &*not_a_file));
    }
    std::fs::remove_dir(&map).unwrap();
    std::os::unix::fs::symlink(repo.aside.join("absent.json"), &map).unwrap();
    for args in [&run[..], &queue] {
        let ran = repo.brokkr("src", args);
        assert_eq!((ran.code, ran.stderr.as_str()), (Some(1), &*dangling));
    }
    assert!(!repo.path(".brokkr/state").exists());

    let library = Repo::new();
    library.library();
    let refused = |said: &str| {
        let ran = library.brokkr("", &["run", "--bundle", "recipes/x", "--feature", "x"]);
        assert_eq!((ran.code, ran.stderr.as_str()), (Some(1), said));
        assert!(!library.path(".forge").exists());
    };
    std::fs::create_dir(library.path("realms.json")).unwrap();
    refused(
        "error: ./realms.json is not a file (decision 0083): the default realms map is a file, \
         or a link to one; move or remove it\n",
    );

    // A link to nothing at a library's `./realms.json` refuses by name too.
    std::fs::remove_dir(library.path("realms.json")).unwrap();
    let absent = library.aside.join("absent.json");
    std::os::unix::fs::symlink(absent, library.path("realms.json")).unwrap();
    refused(
        "error: ./realms.json is a link to nothing (decision 0083 ruling 4: a linked entry is the \
         entry); mend or remove the link\n",
    );
}

/// A library mark that cannot be read refuses by name, and nothing is
/// created: the folder beside it is never chosen in its place (round
/// 634c's first finding). A process that reads a mode-000 directory, as
/// root does, skips the case.
#[test]
fn an_unreadable_library_mark_refuses() {
    let repo = Repo::new();
    repo.library();
    std::fs::create_dir(repo.path(".brokkr")).unwrap();
    let recipes = repo.path("recipes");
    let mode = |bits| std::os::unix::fs::PermissionsExt::from_mode(bits);
    std::fs::set_permissions(&recipes, mode(0o000)).unwrap();
    if std::fs::read_dir(&recipes).is_ok() {
        std::fs::set_permissions(&recipes, mode(0o755)).unwrap();
        eprintln!("skipped: this process reads a mode-000 directory, as root does");
        return;
    }
    let (runs, doctor) = (repo.brokkr("", &["runs"]), repo.brokkr("", &["doctor"]));
    std::fs::set_permissions(&recipes, mode(0o755)).unwrap();
    let denied = format!(
        "reading {}: Permission denied (os error 13)",
        recipes.display()
    );
    assert_eq!(
        (runs.code, runs.stderr),
        (Some(1), format!("error: {denied}\n"))
    );
    let database = doctor
        .stdout
        .lines()
        .find(|line| line.contains(" database: "));
    let reported = format!("warn     database: none: {denied}");
    assert_eq!(database, Some(reported.as_str()), "{}", doctor.stdout);
    assert!(!repo.path(".brokkr/state").exists() && !repo.path(".forge").exists());
}

/// The agents verb refuses an old layout as every other verb does, never
/// reading the old layout's agents (round 634c's sixth finding).
#[test]
fn the_agents_verb_refuses_an_old_layout() {
    let repo = Repo::new();
    old_root_layout(&repo);
    for verb in [&["agents", "list"][..], &["agents", "show", "implementer"]] {
        let ran = repo.brokkr("", verb);
        assert_eq!(
            (ran.code, ran.first_line()),
            (Some(1), ROOT_LAYOUT),
            "{verb:?}"
        );
        assert_eq!(ran.stdout, "", "{verb:?}");
    }
}

/// A `.forge` that is a link is emptied through it, then removed as the
/// link it is: its target directory stays, and the moves made as listed
/// leave nothing half-moved (round 634b's second finding).
#[test]
fn a_linked_forge_is_removed_as_a_link() {
    let repo = Repo::new();
    let (elsewhere, ran) = linked_forge(&repo, "ext");
    let moves = listed_moves(&ran);
    assert_eq!(
        moves,
        [
            "mkdir -p .brokkr/state",
            "mv .forge/forge.db .brokkr/state/forge.db",
            "# .forge is a link: rm removes the link, and the directory it names stays",
            "rm .forge",
            "printf 'state/\\n' > .brokkr/.gitignore",
        ]
    );
    make(&repo, &moves);
    assert!(!repo.path(".forge").exists() && elsewhere.is_dir());
    assert_eq!(
        std::fs::read(repo.path(".brokkr/state/forge.db")).unwrap(),
        b"journal"
    );
}

/// A `.forge/` shape the state moves list or refuse, keyed by the arm it
/// is there to reach: a `Move` of the list builder in `src/folder.rs`, the
/// `Absent` `.forge` it lists no removal for, or the refusal it gives in
/// place of a list. Its entries each hold their own name as their bytes;
/// `committed` are the ones git tracks, committed before `.forge` becomes
/// a link to them where `linked`; `then` changes the repository after;
/// and `refused`, where set, is the whole refusal with no moves.
struct Shape {
    arm: &'static str,
    entries: &'static [&'static str],
    committed: &'static [&'static str],
    linked: bool,
    then: fn(&Repo),
    refused: Option<fn(&Repo) -> String>,
}

/// A shape whose listed moves complete.
fn listing(
    arm: &'static str,
    entries: &'static [&'static str],
    committed: &'static [&'static str],
    linked: bool,
) -> Shape {
    Shape {
        arm,
        entries,
        committed,
        linked,
        then: |_| {},
        refused: None,
    }
}

/// A shape refused by `refused`, after `then`.
fn refusing(
    arm: &'static str,
    base: Shape,
    then: fn(&Repo),
    refused: fn(&Repo) -> String,
) -> Shape {
    Shape {
        arm,
        then,
        refused: Some(refused),
        ..base
    }
}

impl Shape {
    /// Plant the shape in a fresh repository.
    fn plant(&self) -> Repo {
        let repo = Repo::new();
        for entry in self.entries {
            repo.write(&format!(".forge/{entry}"), entry);
        }
        for entry in self.committed {
            git(&repo.root, &["add", "-f", &format!(".forge/{entry}")]);
        }
        if !self.committed.is_empty() {
            commit(&repo.root);
        }
        if self.linked {
            let linked = repo.aside.join("linked");
            std::fs::rename(repo.path(".forge"), &linked).unwrap();
            std::os::unix::fs::symlink(&linked, repo.path(".forge")).unwrap();
        }
        (self.then)(&repo);
        repo
    }
}

/// The arm of the list builder that lists `line`. A line no arm lists
/// fails the test, never passes unclassified.
fn arm_of(line: &str) -> &'static str {
    match line {
        "mkdir -p .brokkr/state" => "Mkdir",
        "git rm .forge/.gitignore" => "GitRm",
        "rm .forge/.gitignore" => "Rm",
        "rmdir .forge" => "Rmdir",
        "# .forge is a link: rm removes the link, and the directory it names stays"
        | "rm .forge" => "Unlink",
        "printf 'state/\\n' > .brokkr/.gitignore" => "Printf",
        _ if line.starts_with("git mv ") => "GitMv",
        _ if line.starts_with("mv ") => "Mv",
        _ => panic!("a listed line no arm of the builder lists: {line}"),
    }
}

/// The arms of the list builder, read off `src/folder.rs`'s `enum Move`:
/// one variant a line, whatever it holds. A line there this cannot read
/// fails the test.
fn builder_arms() -> std::collections::BTreeSet<&'static str> {
    let source = include_str!("../src/folder.rs");
    let body = source
        .split_once("\nenum Move {\n")
        .and_then(|(_, rest)| rest.split_once("\n}\n"))
        .expect("src/folder.rs declares enum Move")
        .0;
    body.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("///") && !line.starts_with("#["))
        .map(|line| {
            let name = line.split(['(', ',']).next().unwrap_or_default();
            let plain = !name.is_empty() && name.chars().all(char::is_alphanumeric);
            assert!(plain && name.len() < line.len(), "a variant line: {line}");
            name
        })
        .collect()
}

/// The refusal a shape whose tracked paths git holds a change to gives,
/// `listed` its lines.
fn uncommitted(repo: &Repo, listed: &str) -> String {
    format!(
        "error: {}/.forge holds paths git tracks whose index or work tree differs from the last \
         commit, which `git mv` or `git rm` would refuse or leave half-moved, so no moves are \
         listed (decision 0083); nothing was opened: commit or restore these, then run \
         again\n{listed}\n",
        repo.root.display()
    )
}

/// The refusal git's own words for `git -C <root> ls-files` give.
fn unlisted(repo: &Repo) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(&repo.root)
        .args(["ls-files", "-s", "-v", "-z", "--", ".forge"])
        .output()
        .unwrap();
    assert!(!out.status.success(), "{out:?}");
    format!(
        "error: git ls-files in {} failed, so the moves cannot tell what git tracks (decision \
         0083); nothing was opened: {}\n",
        repo.root.display(),
        String::from_utf8_lossy(&out.stderr).trim_end()
    )
}

/// A submodule committed at `.forge/z-cache`, then an edit to
/// `.gitmodules` left unstaged, which `git mv` refuses to move it past
/// and `git status -- .forge` never shows (round 634k's review).
fn submodule(repo: &Repo) {
    let sub = repo.aside.join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    git(&sub, &["init", "-q"]);
    std::fs::write(sub.join("cache.txt"), "cached").unwrap();
    git(&sub, &["add", "cache.txt"]);
    commit(&sub);
    let add = ["-c", "protocol.file.allow=always", "submodule", "add", "-q"];
    let from = sub.to_str().unwrap();
    git(&repo.root, &[&add[..], &[from, ".forge/z-cache"]].concat());
    commit(&repo.root);
    let modules = std::fs::read_to_string(repo.path(".gitmodules")).unwrap();
    repo.write(".gitmodules", &format!("{modules}# edited\n"));
}

/// The refusal [`submodule`]'s shape gives in place of its moves.
fn submodule_refused(repo: &Repo) -> String {
    format!(
        "error: {} is a submodule git tracks, which `git mv` moves only together with \
         .gitmodules, so no moves are listed (decision 0083); nothing was opened: move it out of \
         .forge and commit, then run again\n",
        repo.path(".forge/z-cache").display()
    )
}

/// Every shape of `.forge/` the state moves list or refuse, one or more
/// for each arm of the list builder (round 634k's fourth finding).
fn shapes() -> Vec<Shape> {
    let journal = &["forge.db", "forge.db-wal", "forge.db-shm", "results/r.json"];
    vec![
        listing("Mv", journal, &[], false),
        listing("Mv", &["forge.db", "j.sqlite", "j.sqlite-wal"], &[], false),
        listing(
            "Mv",
            &["j.sqlite", "j.sqlite-wal", "j.sqlite-wal-wal"],
            &[],
            false,
        ),
        listing(
            "Mv",
            &["run history.sqlite", "it's", "it's-wal"],
            &[],
            false,
        ),
        listing(
            "GitMv",
            &["forge.db", "notes.md", "results/r.json"],
            &["notes.md", "results/r.json"],
            false,
        ),
        listing(
            "GitRm",
            &["forge.db", ".gitignore", ".gitignore-wal"],
            &[".gitignore"],
            false,
        ),
        listing("Rm", &["forge.db", ".gitignore"], &[], false),
        listing("Rm", &["forge.db", ".gitignore"], &[], true),
        listing("Unlink", &["forge.db", "forge.db-wal"], &[], true),
        Shape {
            then: |repo| {
                let far = repo.aside.join("far");
                std::fs::write(&far, "far").unwrap();
                link(repo, Path::new("./forge.db"), ".forge/alias");
                link(repo, Path::new("../forge.db"), ".forge/results/up");
                link(repo, &far, ".forge/far");
                link(repo, &repo.aside.join("gone/far"), ".forge/gone");
            },
            ..listing("Mv", &["forge.db", "results/r.json"], &[], false)
        },
        Shape {
            then: |repo| drop(repo.write("bundle.json", "{}")),
            ..listing("Absent", &[], &[], false)
        },
    ]
    .into_iter()
    .chain(refusals())
    .chain(linked_refusals())
    .chain(stranding_refusals())
    .collect()
}

/// The refusal a link at `link` under the root, naming `target`, gives:
/// once moved into `.brokkr/state` it would name another path or none.
fn escaping(repo: &Repo, link: &str, target: &Path) -> String {
    format!(
        "error: {} is a link to {}, which a listed move into .brokkr/state would leave naming \
         another path or none, so no moves are listed (decision 0083); nothing was opened: put \
         what it names in its place, or remove it, then run again\n",
        repo.path(link).display(),
        target.display()
    )
}

/// `target` linked at `at` under the root.
fn link(repo: &Repo, target: &Path, at: &str) {
    std::os::unix::fs::symlink(target, repo.path(at)).unwrap();
}

/// The shapes of [`shapes`] a listed move would leave a link stranded in
/// (the fifth landing's review): a `.forge` entry or a deeper link whose
/// relative target leads out of `.forge`, to the ignore file the list
/// removes or beneath it, an absolute one into `.forge` by the root's name
/// for it, by its real directory, through a detour's `..` or through a
/// linked ancestor (the sixth landing's review), and a `.forge` linked
/// inside the repository, where git does not look, to a directory whose
/// entries git tracks.
fn stranding_refusals() -> Vec<Shape> {
    vec![
        refusing(
            "Escaping",
            listing("", &["notes.md"], &[], false),
            |repo| {
                repo.write("journal.db", "journal");
                link(repo, Path::new("../journal.db"), ".forge/forge.db");
            },
            |repo| escaping(repo, ".forge/forge.db", Path::new("../journal.db")),
        ),
        refusing(
            "Escaping",
            listing("", &["results/r.json"], &[], false),
            |repo| link(repo, Path::new("../../journal.db"), ".forge/results/back"),
            |repo| escaping(repo, ".forge/results/back", Path::new("../../journal.db")),
        ),
        refusing(
            "Escaping",
            listing("", &["forge.db", ".gitignore"], &[], false),
            |repo| link(repo, Path::new(".gitignore"), ".forge/alias"),
            |repo| escaping(repo, ".forge/alias", Path::new(".gitignore")),
        ),
        refusing(
            "Escaping",
            listing("", &["notes.md"], &[], true),
            |repo| link(repo, &repo.path(".forge/notes.md"), ".forge/alias"),
            |repo| escaping(repo, ".forge/alias", &repo.path(".forge/notes.md")),
        ),
        refusing(
            "Escaping",
            listing("", &["notes.md"], &[], true),
            |repo| link(repo, &repo.aside.join("linked/notes.md"), ".forge/alias"),
            |repo| escaping(repo, ".forge/alias", &repo.aside.join("linked/notes.md")),
        ),
        refusing(
            "Escaping",
            listing("", &["notes.md"], &[], false),
            |repo| {
                std::fs::create_dir(repo.path("d")).unwrap();
                link(repo, &repo.path("d/../.forge/notes.md"), ".forge/alias");
            },
            |repo| escaping(repo, ".forge/alias", &repo.path("d/../.forge/notes.md")),
        ),
        refusing(
            "Escaping",
            listing("", &["notes.md"], &[], false),
            |repo| {
                std::os::unix::fs::symlink(&repo.root, repo.aside.join("r")).unwrap();
                link(repo, &repo.aside.join("r/.forge/notes.md"), ".forge/alias");
            },
            |repo| escaping(repo, ".forge/alias", &repo.aside.join("r/.forge/notes.md")),
        ),
        refusing(
            "Escaping",
            listing("", &["notes.md"], &[], false),
            |repo| {
                std::fs::create_dir(repo.aside.join("outside")).unwrap();
                std::fs::write(repo.aside.join("outside/forge.db"), "journal").unwrap();
                link(repo, &repo.aside.join("outside"), ".forge/.gitignore");
                link(repo, Path::new(".gitignore/forge.db"), ".forge/alias.db");
            },
            |repo| escaping(repo, ".forge/alias.db", Path::new(".gitignore/forge.db")),
        ),
        refusing(
            "InRepository",
            listing("", &[], &[], false),
            |repo| {
                repo.write("state/notes.md", "notes.md");
                git(&repo.root, &["add", "state/notes.md"]);
                commit(&repo.root);
                link(repo, Path::new("state"), ".forge");
            },
            |repo| {
                format!(
                    "error: {} is a link to {}, inside the repository, and git does not look \
                     through a link, so the moves cannot tell which entries git tracks there; no \
                     moves are listed (decision 0083); nothing was opened: move its entries into \
                     .brokkr/state yourself, by `git mv` for each git tracks, then remove the link \
                     and run again\n",
                    repo.path(".forge").display(),
                    repo.path("state").display()
                )
            },
        ),
    ]
}

/// What every link under `dir` reads as, by its path under `dir`, the old
/// ignore file aside, which the list removes: a list that moves a link
/// leaves it reading what it read (the fifth landing's review).
fn links(dir: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    let mut links = BTreeMap::new();
    let mut pending = vec![PathBuf::new()];
    while let Some(inner) = pending.pop() {
        for entry in std::fs::read_dir(dir.join(&inner)).unwrap().flatten() {
            let (kind, inner) = (entry.file_type().unwrap(), inner.join(entry.file_name()));
            if kind.is_symlink() && inner != Path::new(".gitignore") {
                links.insert(inner, std::fs::read(entry.path()).ok());
            } else if kind.is_dir() {
                pending.push(inner);
            }
        }
    }
    links
}

/// A sparse checkout of `src` alone, which leaves every tracked path under
/// `.forge` out of the work tree, so `git status -- .forge` states nothing
/// and `git rm` refuses each (round 634k's chief). A cone that holds
/// `.forge` but not `.brokkr` is the `Sparse` row: its sources are read,
/// and `git mv` refuses their destination (the third landing's review).
fn sparse(repo: &Repo) {
    git(&repo.root, &["sparse-checkout", "set", "--cone", "src"]);
}

/// The refusal [`sparse`]'s shape gives in place of its moves.
fn sparse_refused(repo: &Repo) -> String {
    format!(
        "error: {}/.forge holds paths git tracks but does not read in the work tree, so `git \
         status` cannot say whether `git mv` or `git rm` would complete, and `git rm` refuses a \
         path a sparse checkout leaves out; no moves are listed (decision 0083); nothing was \
         opened: run `git sparse-checkout disable`, or `git update-index --no-skip-worktree \
         --no-assume-unchanged` on each, then run again\n  S .forge/.gitignore\n",
        repo.root.display()
    )
}

/// The refusal a linked `.forge` gives where git tracks it, a link
/// committed as one, or entries under it.
fn linked_tracked(repo: &Repo) -> String {
    format!(
        "error: {}/.forge is a link and git tracks it or entries under it; a listed `rm` would \
         leave git a deletion of a tracked link, and git refuses some paths beyond a symbolic \
         link, so no moves are listed (decision 0083); nothing was opened: put the directory it \
         names in its place, or untrack it and its entries, then run again\n",
        repo.root.display()
    )
}

/// The refusal a `.forge` linked to `target`, the root or a directory
/// holding it, gives: its entries would move into themselves, the link
/// first among them (the fourth landing's review).
fn holds_root(repo: &Repo, target: &Path) -> String {
    format!(
        "error: {}/.forge is a link to {}, which holds the repository's root, so a listed move \
         would move it into itself; no moves are listed (decision 0083); nothing was opened: put \
         the directory it should name in its place, then run again\n",
        repo.root.display(),
        target.display()
    )
}

/// Commit the root's `.forge` link as the link it is.
fn commit_link(repo: &Repo) {
    git(&repo.root, &["add", ".forge"]);
    commit(&repo.root);
}

/// The shapes of [`shapes`] refused where `.forge` is or was a link: one git
/// tracks entries under, one committed as a link, one committed as a link
/// and then made a directory, and links to the root and to its parent,
/// which hold the root (the fourth landing's review).
fn linked_refusals() -> Vec<Shape> {
    vec![
        refusing(
            "LinkedTracked",
            listing("", &["forge.db", "notes.md"], &["notes.md"], true),
            |_| {},
            linked_tracked,
        ),
        refusing(
            "LinkedTracked",
            listing("", &["forge.db"], &[], true),
            commit_link,
            linked_tracked,
        ),
        refusing(
            "Uncommitted",
            listing("", &["forge.db"], &[], true),
            |repo| {
                commit_link(repo);
                std::fs::remove_file(repo.path(".forge")).unwrap();
                repo.write(".forge/forge.db", "forge.db");
            },
            |repo| uncommitted(repo, "   D .forge"),
        ),
        refusing(
            "ForgeHoldsRoot",
            listing("", &[], &[], false),
            |repo| std::os::unix::fs::symlink(".", repo.path(".forge")).unwrap(),
            |repo| holds_root(repo, &repo.root),
        ),
        refusing(
            "ForgeHoldsRoot",
            listing("", &[], &[], false),
            |repo| std::os::unix::fs::symlink("..", repo.path(".forge")).unwrap(),
            |repo| holds_root(repo, repo.root.parent().unwrap()),
        ),
    ]
}

/// The shapes of [`shapes`] no listed move could complete, each refused
/// by name in place of its moves.
fn refusals() -> Vec<Shape> {
    vec![
        refusing(
            "IgnoreNotAFile",
            listing("", &["forge.db", ".gitignore/keep"], &[], false),
            |_| {},
            |repo| {
                format!(
                    "error: {} is neither a file nor a link, so no listed move could remove it \
                     (decision 0083); nothing was opened: move it aside\n",
                    repo.path(".forge/.gitignore").display()
                )
            },
        ),
        refusing(
            "Uncommitted",
            listing("", &["forge.db", ".gitignore"], &[], false),
            |repo| drop(git(&repo.root, &["add", "-f", ".forge/.gitignore"])),
            |repo| uncommitted(repo, "  A  .forge/.gitignore"),
        ),
        refusing(
            "Uncommitted",
            listing("", &["forge.db", ".gitignore"], &[".gitignore"], false),
            |repo| drop(repo.write(".forge/.gitignore", "edited")),
            |repo| uncommitted(repo, "   M .forge/.gitignore"),
        ),
        refusing(
            "Uncommitted",
            listing("", &["forge.db", "notes.md"], &["notes.md"], false),
            |repo| {
                std::fs::remove_file(repo.path(".forge/notes.md")).unwrap();
                repo.write(".forge/notes.md/keep", "a directory now");
            },
            |repo| uncommitted(repo, "   D .forge/notes.md"),
        ),
        refusing(
            "Unwatched",
            listing("", &["forge.db", ".gitignore"], &[".gitignore"], false),
            sparse,
            sparse_refused,
        ),
        refusing(
            "Sparse",
            listing("", &["forge.db", "notes.md"], &["notes.md"], false),
            |repo| {
                drop(git(
                    &repo.root,
                    &["sparse-checkout", "set", "--cone", ".forge"],
                ))
            },
            |repo| {
                format!(
                    "error: {} has a sparse checkout and git tracks entries under .forge, so a \
                     listed `git mv` into .brokkr/state could fall outside its definition and \
                     fail; no moves are listed (decision 0083); nothing was opened: run `git \
                     sparse-checkout disable`, or move the tracked entries yourself, then run \
                     again\n",
                    repo.root.display()
                )
            },
        ),
        refusing(
            "TrackedSubmodule",
            listing("", &["forge.db"], &[], false),
            submodule,
            submodule_refused,
        ),
        refusing(
            "Untracked",
            listing("", &["forge.db"], &[], false),
            |repo| {
                std::fs::remove_dir_all(repo.path(".git")).unwrap();
                repo.write(".git", "not a gitfile\n");
            },
            unlisted,
        ),
    ]
}

/// Every shape of `.forge/` this slice moves, its listed moves made as
/// listed by `sh` (round 634i's third finding): `.forge` no longer
/// stands, git tracks nothing under it and states no tracked path deleted
/// in the work tree, every entry arrived under `.brokkr/state/` byte for
/// byte, but the old ignore file, which the list removes, and every link
/// moved reads what it read (the fifth landing's review). A shape no listed move could complete refuses by name
/// with no moves and moves nothing (rounds 634j and 634k). Every arm of
/// the list builder is reached, read off its enum, so an arm no shape
/// reaches fails here (round 634k's fourth finding).
#[test]
fn the_listed_moves_empty_every_forge() {
    let mut reached = std::collections::BTreeSet::new();
    for shape in shapes() {
        let repo = shape.plant();
        let (arm, entries) = (shape.arm, shape.entries);
        let before = repo.files();
        let ran = repo.brokkr("", &["runs"]);
        if let Some(refused) = shape.refused {
            assert_eq!((ran.code, ran.stderr), (Some(1), refused(&repo)), "{arm}");
            assert_eq!(repo.files(), before, "{arm}");
            reached.insert(arm);
            continue;
        }
        let layout = match repo.path("bundle.json").exists() {
            true => ROOT_LAYOUT,
            false => STATE_ONLY,
        };
        assert_eq!((ran.code, ran.first_line()), (Some(1), layout), "{arm}");
        let moves = listed_moves(&ran);
        let mut arms: Vec<&str> = moves.iter().map(|line| arm_of(line)).collect();
        let carried = match std::fs::symlink_metadata(repo.path(".forge")) {
            Ok(_) => links(&repo.path(".forge")),
            Err(_) => {
                arms.push("Absent");
                BTreeMap::new()
            }
        };
        assert!(arms.contains(&arm), "{arm}: {moves:?}");
        reached.extend(arms);
        make(&repo, &moves);
        let forge = std::fs::symlink_metadata(repo.path(".forge"));
        let gone = forge.map(|_| ()).map_err(|error| error.kind());
        assert_eq!(gone, Err(std::io::ErrorKind::NotFound), "{arm}");
        assert_eq!(git(&repo.root, &["ls-files", "--", ".forge"]), "", "{arm}");
        let status = git(
            &repo.root,
            &["status", "--porcelain=v1", "--untracked-files=no"],
        );
        let deleted = status.lines().filter(|line| line.as_bytes()[1] == b'D');
        assert_eq!(deleted.count(), 0, "{arm}: a tracked deletion: {status}");
        let state = repo.path(".brokkr/state");
        assert_eq!(links(&state), carried, "{arm}");
        for entry in entries.iter().filter(|entry| **entry != ".gitignore") {
            let moved = std::fs::read(state.join(entry)).unwrap();
            assert_eq!(moved, entry.as_bytes(), "{arm}: {entries:?}");
        }
        assert!(!state.join(".gitignore").exists(), "{arm}");
    }
    let unreached: Vec<&str> = builder_arms()
        .into_iter()
        .filter(|arm| !reached.contains(arm))
        .collect();
    assert_eq!(
        unreached,
        Vec::<&str>::new(),
        "arms of the list builder no shape reaches"
    );
}

#[test]
fn a_folder_bundle_composes_no_base() {
    let repo = Repo::new();
    repo.folder();
    let path = repo.path(".brokkr/bundle.json");
    let mut bundle: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    bundle["extends"] = json!("shared");
    std::fs::write(&path, bundle.to_string()).unwrap();
    // A base that would refuse in its own words if it were ever read.
    repo.write("shared/bundle.json", "not json");
    let refusal = format!(
        "error: bundle: {}/.brokkr/bundle.json extends 'shared' (decision 0083); a bundle in \
         .brokkr/ composes no base, because its library would be {}: remove extends\n",
        repo.root.display(),
        repo.root.display()
    );
    let ran = repo.brokkr("src", &["run", "--feature", "x"]);
    assert_eq!((ran.code, ran.stderr.as_str()), (Some(1), refusal.as_str()));
    let ran = repo.brokkr("", &["compile", "--bundle", ".brokkr"]);
    assert_eq!((ran.code, ran.stderr.as_str()), (Some(1), refusal.as_str()));
}
