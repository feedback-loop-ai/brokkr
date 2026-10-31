//! Decision 0083 (#634): an adopter's Brokkr lives in one `.brokkr/`
//! folder at its repository's root. Discovery from anywhere in the
//! repository, the defaults that follow from it, the old layouts refused
//! with the moves to make, `init` writing the folder and nothing else,
//! and doctor's folder line checking a hand move. Most tests plant the
//! folder by hand; `init_doctor` reaches [`Repo`] for its own.
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
pub(crate) struct Ran {
    pub(crate) code: Option<i32>,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

impl Ran {
    pub(crate) fn first_line(&self) -> &str {
        self.stderr.lines().next().unwrap_or_default()
    }

    pub(crate) fn of(out: std::process::Output) -> Ran {
        let text = |bytes: Vec<u8>| String::from_utf8(bytes).unwrap();
        let code = out.status.code();
        let (stdout, stderr) = (text(out.stdout), text(out.stderr));
        Ran {
            code,
            stdout,
            stderr,
        }
    }
}

/// A fresh repository, and a directory beside it that is in no
/// repository: the fake driver's script and state, and every engine's
/// TMPDIR, so nothing a run writes outside `.brokkr/state/` lands in the
/// repository.
pub(crate) struct Repo {
    _dir: tempfile::TempDir,
    _aside: tempfile::TempDir,
    pub(crate) root: PathBuf,
    aside: PathBuf,
}

impl Repo {
    pub(crate) fn new() -> Repo {
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

    pub(crate) fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    pub(crate) fn write(&self, relative: &str, text: &str) -> PathBuf {
        let path = self.path(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        path
    }

    /// `brokkr` standing in `dir`, relative to the root, created first,
    /// with [`Repo::config`] for its configuration directory and a HOME of
    /// its own beside the repository ([`crate::engine::home_in`]).
    pub(crate) fn command(&self, dir: &str) -> Command {
        let cwd = self.path(dir);
        std::fs::create_dir_all(&cwd).unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_brokkr"));
        command.current_dir(cwd).env("TMPDIR", &self.aside);
        command.env("XDG_CONFIG_HOME", self.config());
        command.env("HOME", crate::engine::home_in(&self.aside));
        command
    }

    /// The `XDG_CONFIG_HOME` every `brokkr` here reads, beside the
    /// repository: no host's own personal shelf (#697) or `host.json` is
    /// read, and a test plants its own there.
    pub(crate) fn config(&self) -> PathBuf {
        self.aside.join("config")
    }

    /// `brokkr <args>` from `dir`, relative to the root, with `stdin`.
    ///
    /// A `brokkr` that refuses before it reads, as `secrets set` does with
    /// no folder, ends without taking its input: when the host runs it to
    /// its end ahead of this write, the pipe is already closed, and the
    /// broken pipe says only that, which the code and stderr of the
    /// [`Ran`] judge. Any other failure to deliver the input is the
    /// test's.
    fn brokkr_with(&self, dir: &str, args: &[&str], stdin: &str) -> Ran {
        let mut child = self
            .command(dir)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let given = child.stdin.take().unwrap().write_all(stdin.as_bytes());
        if let Err(error) = given {
            assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe, "{error}");
        }
        Ran::of(child.wait_with_output().unwrap())
    }

    pub(crate) fn brokkr(&self, dir: &str, args: &[&str]) -> Ran {
        self.brokkr_with(dir, args, "")
    }

    /// `brokkr init .` at the root under a PATH holding only the agent
    /// CLIs `present`, made beside the repository, so it scaffolds for the
    /// first of them whatever this host carries.
    pub(crate) fn init_with_only(&self, present: &[&str]) -> Ran {
        let bins = bins_holding(&self.aside, present);
        let mut init = self.command("");
        init.args(["init", "."]).env("PATH", &bins);
        Ran::of(init.output().unwrap())
    }

    /// A fake-driver bundle named `name` at `relative`, its implementer
    /// answering each attempt with `answers` in turn and its reviewer
    /// clean.
    pub(crate) fn bundle(&self, relative: &str, name: &str, answers: Value) {
        let script = self.aside.join(format!("{name}.script.json"));
        let review = json!([{"behavior": "succeed", "result": {"result": "clean"}}]);
        let seats = json!({"seats": {"implement": answers, "review": review}});
        std::fs::write(&script, seats.to_string()).unwrap();
        let state = self.aside.join(format!("{name}.state"));
        std::fs::create_dir_all(&state).unwrap();
        // Its office lives where a run reads it: the folder's places once
        // there is a folder, else the library's (U9b1; `fake_office`).
        let folder = self.root.join(".brokkr");
        let places = match relative.starts_with(".brokkr") || folder.is_dir() {
            true => folder,
            false => self.root.clone(),
        };
        let office = format!("{name}-office");
        let hired = [office.as_str(), &format!("{name}-fake")];
        crate::fake_office::hire_driver(
            &places,
            hired,
            crate::fake_office::fake_driver(&script, &state),
        );
        self.hiring(relative, name, &office);
    }

    /// A bundle named `name` at `relative` whose seats hire `office`, an
    /// office already in the library.
    pub(crate) fn hiring(&self, relative: &str, name: &str, office: &str) {
        let dir = self.root.join(relative);
        std::fs::create_dir_all(dir.join("roles")).unwrap();
        std::fs::write(dir.join("policy.json"), policy()).unwrap();
        std::fs::write(dir.join("roles/role.md"), "# role\n").unwrap();
        let seat = |results: Value| json!({"results": results, "agent": office});
        let bundle = json!({
            "name": name,
            "policy": "policy.json",
            "seats": {"implement": seat(json!(["complete"])), "review": seat(json!(["clean"]))},
        });
        std::fs::write(dir.join("bundle.json"), bundle.to_string()).unwrap();
    }

    /// The folder planted by hand, as `init` will write it: its bundle and
    /// the ignore file that keeps its state out of git.
    pub(crate) fn folder(&self) {
        self.bundle(".brokkr", "folder", completes());
        self.write(".brokkr/.gitignore", "state/\n");
    }

    /// Whether the fake driver of the bundle named `name` has kept state:
    /// whether a run was delivered under it.
    pub(crate) fn seated(&self, name: &str) -> bool {
        let state = self.aside.join(format!("{name}.state"));
        std::fs::read_dir(state).unwrap().count() > 0
    }

    /// A recipe library's mark: a bundle under `recipes/`.
    pub(crate) fn library(&self) {
        self.bundle("recipes/x", "library", completes());
    }

    /// Every file under the root but `.git`, with its bytes.
    pub(crate) fn files(&self) -> BTreeMap<PathBuf, Vec<u8>> {
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

pub(crate) fn completes() -> Value {
    json!([{"behavior": "succeed", "result": {"result": "complete"}}])
}

pub(crate) fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// A fresh repository in a temporary directory, so a verb run from inside
/// its `.brokkr/` walks up to it rather than taking the folder for a root
/// (ruling 2).
pub(crate) fn repository() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    dir
}

/// `aside/bins`, holding an empty file for each of the agent CLIs
/// `present`: a PATH on which `init` finds those and no others, plus the
/// host's `git` a compile asks about the bundle's ignored trees (#908).
pub(crate) fn bins_holding(aside: &Path, present: &[&str]) -> PathBuf {
    let bins = aside.join("bins");
    std::fs::create_dir_all(&bins).unwrap();
    for binary in present {
        std::fs::write(bins.join(binary), "").unwrap();
    }
    plant_git(&bins);
    bins
}

/// Put the host's `git` in `bins`. The compiler asks git which trees a
/// bundle's repository ignores (#908), so a fixture PATH that hides the
/// host's tools must still let it ask.
pub(crate) fn plant_git(bins: &Path) {
    let at = bins.join("git");
    if at.symlink_metadata().is_ok() {
        return;
    }
    let git = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|dir| dir.join("git"))
        .find(|path| path.is_file())
        .expect("git on the test process's PATH");
    std::os::unix::fs::symlink(git, at).unwrap();
}

/// Commit what `root`'s index holds.
pub(crate) fn commit(root: &Path) {
    let author = ["-c", "user.name=t", "-c", "user.email=t@example.invalid"];
    let commit = ["-c", "commit.gpgsign=false", "commit", "-q", "-m", "init"];
    git(root, &[&author[..], &commit[..]].concat());
}

/// The run id a `brokkr run` said it started.
pub(crate) fn started(ran: &Ran) -> String {
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

pub(crate) const ROOT_LAYOUT: &str = "error: this repository has Brokkr's old root layout and no \
                                      .brokkr/ folder (decision 0083); nothing was opened, and \
                                      Brokkr moves nothing";

pub(crate) const STATE_ONLY: &str = "error: this repository has Brokkr state in .forge/ and no \
                                     .brokkr/ folder (decision 0083); nothing was opened, and \
                                     Brokkr moves nothing";

pub(crate) fn half_moved(root: &Path, left: &str) -> String {
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
    // store, and, the folder being doctor's bundle (R5), against its
    // seats, none of which binds it.
    let mut doctor = repo.command("src/deep");
    doctor.arg("doctor").env("SPARK_API_KEY", "set");
    let printed = String::from_utf8(doctor.output().unwrap().stdout).unwrap();
    let state = repo.path(".brokkr/state");
    let lines = [
        format!(
            "database: {} opens (WAL, append-only triggers)",
            state.join("forge.db").display()
        ),
        // The folder's own office, hired from its agents (U9b1).
        "ok       agent folder-office: would run folder-office via folder-fake here".to_string(),
        "credential 'SPARK_API_KEY' is satisfied from the process environment — an ambient \
         value is journaled nowhere and moves no digest (decision 0036 ruling 5); no seat of \
         the inspected bundle binds it"
            .to_string(),
    ];
    for line in lines {
        assert!(printed.contains(&line), "{line} in {printed}");
    }
    // Its first line names the folder and where it was found from.
    let root = repo.root.display();
    let found = format!("ok       folder: {root}/.brokkr (found from {root}/src/deep)");
    assert_eq!(printed.lines().next(), Some(found.as_str()), "{printed}");
}

/// Ruling 1's tracked scaffold, under the folder.
const SCAFFOLD: [&str; 15] = [
    ".gitignore",
    "adapters/claude.json",
    "adapters/exec.json",
    "agents/README.md",
    "agents/charters/implementer.md",
    "agents/charters/intake.md",
    "agents/charters/reviewer.md",
    "agents/implementer.json",
    "agents/intake.json",
    "agents/reviewer.json",
    "bundle.json",
    "policy.json",
    "realms.json",
    "scripts/ship-seat.sh",
    "scripts/verify-seat.sh",
];

/// `init` in a fresh repository, a plain one and then an OpenSpec one,
/// which adds the dialect's library: the paths the tree gained, the paths
/// `init` printed and ruling 1's tracked scaffold are one list; nothing
/// else moved or appeared, and no file the claude scaffold holds names
/// `.forge/` (the codex declaration's evidence citation is R6's).
#[test]
fn init_writes_only_under_the_folder() {
    let instructions = [
        "analyze", "archive", "clarify", "design", "return", "specify", "tasks",
    ];
    let mut openspec: Vec<String> = instructions
        .map(|name| format!("dialects/openspec/{name}.md"))
        .into();
    openspec.push("dialects/openspec.json".to_string());
    for (spec, added) in [(false, Vec::new()), (true, openspec)] {
        let repo = Repo::new();
        if spec {
            repo.write("openspec/config.yaml", "schema: spec-driven\n");
        }
        let before = repo.files();
        let ran = repo.init_with_only(&["claude"]);
        assert_eq!(ran.code, Some(0), "{}", ran.stderr);
        let after = repo.files();
        let gained: Vec<&PathBuf> = after
            .keys()
            .filter(|path| !before.contains_key(*path))
            .collect();
        let named = SCAFFOLD
            .iter()
            .copied()
            .chain(added.iter().map(String::as_str));
        let mut expected: Vec<PathBuf> =
            named.map(|path| Path::new(".brokkr").join(path)).collect();
        expected.sort();
        assert_eq!(gained, expected.iter().collect::<Vec<_>>());
        let lines = ran.stderr.lines().skip(1);
        let printed: Vec<PathBuf> = lines
            .map_while(|line| line.strip_prefix("  "))
            .map(PathBuf::from)
            .collect();
        assert_eq!(printed, expected);
        assert!(before
            .iter()
            .all(|(path, bytes)| after.get(path) == Some(bytes)));
        assert!(!repo.path(".forge").exists());
        let forged = gained
            .iter()
            .filter(|path| String::from_utf8_lossy(&after[**path]).contains(".forge/"));
        assert_eq!(forged.collect::<Vec<_>>(), Vec::<&&PathBuf>::new());
    }
}

/// `init`'s old output, journaled by its run through the `--db` escape,
/// moved by hand exactly as the refusal says: its listed moves, then its
/// prose. Doctor's folder line is the clean one, and the run journaled
/// before the move is listed from the folder's journal.
#[test]
fn a_hand_moved_layout_runs() {
    let repo = Repo::new();
    repo.bundle("", "old", completes());
    let map = |journal: &str, path: &str| {
        json!({"schema": "forge.realms/v1", "journal": journal,
               "realms": [{"name": "old", "path": path, "default_branch": "main"}]})
        .to_string()
    };
    repo.write("realms.json", &map(".forge/forge.db", "."));
    repo.write(".forge/.gitignore", "*\n!.gitignore\n");
    git(&repo.root, &["add", "."]);
    commit(&repo.root);
    let old = ["run", "--bundle", ".", "--feature", "x", "--no-view"];
    let ran = repo.brokkr("", &[&old[..], &["--db", ".forge/forge.db"]].concat());
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let run = started(&ran);
    let refused = repo.brokkr("", &["runs"]);
    assert_eq!((refused.code, refused.first_line()), (Some(1), ROOT_LAYOUT));
    make(&repo, &listed_moves(&refused));
    std::fs::create_dir_all(repo.path(".brokkr/roles")).unwrap();
    // The office its seats hire moves with them (U9b1).
    for read in [
        "bundle.json",
        "policy.json",
        "roles/role.md",
        "agents",
        "adapters",
    ] {
        std::fs::rename(repo.path(read), repo.path(&format!(".brokkr/{read}"))).unwrap();
    }
    std::fs::remove_file(repo.path("realms.json")).unwrap();
    repo.write(".brokkr/realms.json", &map("state/forge.db", ".."));

    let doctor = repo.brokkr("", &["doctor"]);
    let root = repo.root.display();
    let clean = format!("ok       folder: {root}/.brokkr (found from {root})");
    assert_eq!(
        doctor.stdout.lines().next(),
        Some(clean.as_str()),
        "{}",
        doctor.stdout
    );
    let runs = repo.brokkr("src", &["runs", "--json"]);
    assert_eq!(listed(&runs), [run], "{}", runs.stderr);
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
/// a relative path against. A queued `--recipe` reads the workspace's own
/// shelf, the one shelf a queued launch holds (#697).
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
        &["--recipe", "r"],
        &["--realms", "maps/realms.json"],
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
        [
            &json!(repo.path(".brokkr/recipes")),
            &at("maps/realms.json")
        ]
    );
}

/// `--bundle` and `--recipe` beside a discovered folder: the run is the
/// named bundle's, whose fake driver alone keeps state.
fn a_named_bundle_runs(repo: &Repo) -> (String, String) {
    let seated = |name: &str| repo.seated(name);
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
    // The folder's own library holds the offices its bundles hire (U9b1).
    let offices: Vec<&str> = folder
        .stdout
        .lines()
        .filter_map(|l| l.split('\t').next())
        .collect();
    assert_eq!(
        (&*folder.stderr, offices.contains(&"folder-office")),
        ("", true),
        "{offices:?}"
    );
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

    // The listed moves, made as listed, leave a folder `brokkr init .`
    // writes the scaffold beside, and the store is read from it.
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
    let ran = repo.brokkr("", &["init", "."]);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
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

/// The one link refusal a link at `at` under the root gives: the moves
/// list nothing through or for one (the operator's ruling of 2026-10-14).
fn a_link(repo: &Repo, at: &str) -> String {
    format!(
        "error: {} is a symbolic link, and Brokkr lists no moves through or for one (decision \
         0083; the operator's ruling of 2026-10-14); nothing was opened: move the state into \
         .brokkr/state yourself, then run again\n",
        repo.path(at).display()
    )
}

/// A `.forge` that is a link is an old layout, judged as the entry, and
/// its moves refuse by the one link rule; a `bundle.json` that is a link
/// is the root layout's.
#[test]
fn a_linked_entry_is_the_entry() {
    let repo = Repo::new();
    let elsewhere = repo.aside.join("state");
    std::fs::create_dir_all(&elsewhere).unwrap();
    std::fs::write(elsewhere.join("forge.db"), "journal").unwrap();
    std::os::unix::fs::symlink(&elsewhere, repo.path(".forge")).unwrap();
    let ran = repo.brokkr("", &["runs"]);
    assert_eq!((ran.code, ran.stderr), (Some(1), a_link(&repo, ".forge")));
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
    // Nor does one whose library holds the old layout's empty agent beside
    // the office its seats hire (U9b1).
    std::fs::remove_file(repo.path("agents/implementer.json")).unwrap();
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

/// The implement seat of the bundle at `relative` hires a role under its
/// `state/`. A role is no agent's, so the seat is inline, dispatching the
/// operator adapter `fake_office::inline` writes in the places the
/// review seat's office opens (U9b1).
fn role_under_state(repo: &Repo, relative: &str) {
    repo.write(&format!("{relative}/state/role.md"), "# role\n");
    let path = repo.path(&format!("{relative}/bundle.json"));
    let mut bundle: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let places = match relative.starts_with(".brokkr") {
        true => repo.path(".brokkr"),
        false => repo.root.clone(),
    };
    let (script, state) = (repo.aside.join("inline.json"), repo.aside.join("inline"));
    let driver = crate::fake_office::inline(&places, &script, &state);
    bundle["seats"]["implement"] =
        json!({"results": ["complete"], "role": "state/role.md", "driver": driver});
    std::fs::write(&path, bundle.to_string()).unwrap();
}

/// The digest `brokkr compile --bundle <relative>` prints.
pub(crate) fn digest(repo: &Repo, relative: &str) -> String {
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

/// The library verbs refuse an old layout as every other verb does, never
/// reading the old layout's agents (round 634c's sixth finding), its
/// bundle or its recipes.
#[test]
fn the_library_verbs_refuse_an_old_layout() {
    let repo = Repo::new();
    old_root_layout(&repo);
    let verbs: [&[&str]; 5] = [
        &["agents", "list"],
        &["agents", "show", "implementer"],
        &["compile", "--bundle", "."],
        &["recipes", "list"],
        &["recipes", "show", "x"],
    ];
    for verb in verbs {
        let ran = repo.brokkr("", verb);
        assert_eq!(
            (ran.code, ran.first_line()),
            (Some(1), ROOT_LAYOUT),
            "{verb:?}"
        );
        assert_eq!(ran.stdout, "", "{verb:?}");
    }
}

/// A `.forge/` shape the state moves list or refuse, keyed by the arm it
/// is there to reach: a `Move` of the list builder in `src/folder.rs`, the
/// `Absent` `.forge` it lists no removal for, or the refusal it gives in
/// place of a list. Its entries each hold their own name as their bytes;
/// `committed` are the ones git tracks; `then` changes the repository
/// after; and `refused`, where set, is the whole refusal with no moves.
struct Shape {
    arm: &'static str,
    entries: &'static [&'static str],
    committed: &'static [&'static str],
    then: fn(&Repo),
    refused: Option<fn(&Repo) -> String>,
}

/// A shape whose listed moves complete.
fn listing(
    arm: &'static str,
    entries: &'static [&'static str],
    committed: &'static [&'static str],
) -> Shape {
    Shape {
        arm,
        entries,
        committed,
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
        listing("Mv", journal, &[]),
        listing("Mv", &["forge.db", "j.sqlite", "j.sqlite-wal"], &[]),
        listing("Mv", &["j.sqlite", "j.sqlite-wal", "j.sqlite-wal-wal"], &[]),
        listing("Mv", &["run history.sqlite", "it's", "it's-wal"], &[]),
        listing(
            "GitMv",
            &["forge.db", "notes.md", "results/r.json"],
            &["notes.md", "results/r.json"],
        ),
        listing(
            "GitRm",
            &["forge.db", ".gitignore", ".gitignore-wal"],
            &[".gitignore"],
        ),
        listing("Rm", &["forge.db", ".gitignore"], &[]),
        Shape {
            then: |repo| drop(repo.write("bundle.json", "{}")),
            ..listing("Absent", &[], &[])
        },
    ]
    .into_iter()
    .chain(refusals())
    .chain(link_refusals())
    .collect()
}

/// `target` linked at `at` under the root.
fn link(repo: &Repo, target: &Path, at: &str) {
    std::os::unix::fs::symlink(target, repo.path(at)).unwrap();
}

/// The shapes of [`shapes`] the one link rule refuses (the operator's
/// ruling of 2026-10-14), a link at each depth: `.forge` itself, its
/// ignore file, a journal, and a link two directories down. The linked
/// `.forge` holds entries git tracks, so the rule is decided before any
/// git question.
fn link_refusals() -> Vec<Shape> {
    vec![
        refusing(
            "Link",
            listing("", &["forge.db", "notes.md"], &["notes.md"]),
            |repo| {
                let linked = repo.aside.join("linked");
                std::fs::rename(repo.path(".forge"), &linked).unwrap();
                link(repo, &linked, ".forge");
            },
            |repo| a_link(repo, ".forge"),
        ),
        refusing(
            "Link",
            listing("", &["forge.db"], &[]),
            |repo| link(repo, &repo.aside.join("ignore"), ".forge/.gitignore"),
            |repo| a_link(repo, ".forge/.gitignore"),
        ),
        refusing(
            "Link",
            listing("", &["notes.md"], &[]),
            |repo| link(repo, &repo.aside.join("journal.db"), ".forge/forge.db"),
            |repo| a_link(repo, ".forge/forge.db"),
        ),
        refusing(
            "Link",
            listing("", &["forge.db", "results/run/r.json"], &[]),
            |repo| {
                link(
                    repo,
                    &repo.aside.join("elsewhere"),
                    ".forge/results/run/out",
                )
            },
            |repo| a_link(repo, ".forge/results/run/out"),
        ),
    ]
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

/// The shapes of [`shapes`] no listed move could complete, each refused
/// by name in place of its moves.
fn refusals() -> Vec<Shape> {
    vec![
        refusing(
            "IgnoreNotAFile",
            listing("", &["forge.db", ".gitignore/keep"], &[]),
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
            listing("", &["forge.db", ".gitignore"], &[]),
            |repo| drop(git(&repo.root, &["add", "-f", ".forge/.gitignore"])),
            |repo| uncommitted(repo, "  A  .forge/.gitignore"),
        ),
        refusing(
            "Uncommitted",
            listing("", &["forge.db", ".gitignore"], &[".gitignore"]),
            |repo| drop(repo.write(".forge/.gitignore", "edited")),
            |repo| uncommitted(repo, "   M .forge/.gitignore"),
        ),
        refusing(
            "Uncommitted",
            listing("", &["forge.db", "notes.md"], &["notes.md"]),
            |repo| {
                std::fs::remove_file(repo.path(".forge/notes.md")).unwrap();
                repo.write(".forge/notes.md/keep", "a directory now");
            },
            |repo| uncommitted(repo, "   D .forge/notes.md"),
        ),
        // A `.forge` committed as a link, then made a directory.
        refusing(
            "Uncommitted",
            listing("", &[], &[]),
            |repo| {
                link(repo, &repo.aside.join("linked"), ".forge");
                git(&repo.root, &["add", ".forge"]);
                commit(&repo.root);
                std::fs::remove_file(repo.path(".forge")).unwrap();
                repo.write(".forge/forge.db", "forge.db");
            },
            |repo| uncommitted(repo, "   D .forge"),
        ),
        refusing(
            "Unwatched",
            listing("", &["forge.db", ".gitignore"], &[".gitignore"]),
            sparse,
            sparse_refused,
        ),
        refusing(
            "Sparse",
            listing("", &["forge.db", "notes.md"], &["notes.md"]),
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
            listing("", &["forge.db"], &[]),
            submodule,
            submodule_refused,
        ),
        refusing(
            "Untracked",
            listing("", &["forge.db"], &[]),
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
/// byte, but the old ignore file, which the list removes. A shape no
/// listed move could complete, a link at or under `.forge` among them,
/// refuses by name with no moves, moves nothing and creates nothing
/// (rounds 634j and 634k; the operator's ruling of 2026-10-14). Every arm
/// of the list builder is reached, read off its enum, so an arm no shape
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
            assert!(!repo.path(".brokkr").exists(), "{arm}");
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
        if !repo.path(".forge").exists() {
            arms.push("Absent");
        }
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

/// A folder bundle composes its base from its shelf, `.brokkr/recipes`
/// (#696, amending decision 0083 ruling 1), never the repository root: a
/// base the shelf lacks refuses naming the shelf, and one it holds
/// composes.
#[test]
fn a_folder_bundle_composes_from_its_shelf() {
    let repo = Repo::new();
    repo.folder();
    let path = repo.path(".brokkr/bundle.json");
    let mut bundle: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    bundle["extends"] = json!("shared");
    std::fs::write(&path, bundle.to_string()).unwrap();
    // A base that would refuse in its own words if it were ever read.
    repo.write("shared/bundle.json", "not json");
    let refusal = format!(
        "error: bundle: {}/.brokkr/bundle.json: extends 'shared', which is not a recipe in \
         {}/.brokkr/recipes\n",
        repo.root.display(),
        repo.root.display()
    );
    let ran = repo.brokkr("src", &["run", "--feature", "x"]);
    assert_eq!((ran.code, ran.stderr.as_str()), (Some(1), refusal.as_str()));
    let ran = repo.brokkr("", &["compile", "--bundle", ".brokkr"]);
    assert_eq!((ran.code, ran.stderr.as_str()), (Some(1), refusal.as_str()));
    repo.bundle(".brokkr/recipes/shared", "shared", completes());
    let overlay = json!({"name": "folder", "extends": "shared"});
    std::fs::write(&path, overlay.to_string()).unwrap();
    let ran = repo.brokkr("", &["compile", "--bundle", ".brokkr"]);
    let compiled: Value = serde_json::from_str(&ran.stdout).unwrap();
    let base = &compiled["composed_from"][0]["recipe"];
    let composed = (ran.code, base);
    assert_eq!(composed, (Some(0), &json!("shared")), "{}", ran.stderr);
    let ran = repo.brokkr("src", &["run", "--feature", "x", "--no-view"]);
    assert_eq!(
        (ran.code, repo.seated("shared")),
        (Some(0), true),
        "{}",
        ran.stderr
    );
}
