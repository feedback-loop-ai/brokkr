//! A queued launch is admitted in the workspace it was queued in, from
//! whatever directory the dispatcher stands in (decision 0068 ruling 1;
//! #430): its bundle, its realm, that realm's boundary and grants
//! (decision 0065 ruling 3) and the repository the engine operates are
//! the workspace's, never the admitting process's directory's.
//!
//! A binary of its own, because it moves the process's working directory;
//! each test holds the one lock while it stands elsewhere.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use brokkr_core::realms::Boundary;
use brokkr_runtime::launch::{self, BundleSource, LaunchError, LaunchRequest, NewRun, RunMap};
use brokkr_runtime::launch::{Encoding, QueuedLaunch};
use brokkr_runtime::{CompileError, Engine, World};
use brokkr_store::Store;
use serde_json::{json, Value};

static CWD: Mutex<()> = Mutex::new(());

/// The process standing in a directory until dropped, alone.
struct Standing {
    _alone: MutexGuard<'static, ()>,
    was: PathBuf,
}

impl Standing {
    fn in_dir(dir: &Path) -> Standing {
        let alone = CWD.lock().unwrap_or_else(PoisonError::into_inner);
        let was = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir).unwrap();
        Standing { _alone: alone, was }
    }
}

impl Drop for Standing {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.was).unwrap();
    }
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn write(path: &Path, value: &Value) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, value.to_string()).unwrap();
}

fn copy_shipped(root: &Path, relative: &str) {
    let to = root.join(relative);
    std::fs::create_dir_all(to.parent().unwrap()).unwrap();
    std::fs::copy(repository_root().join(relative), to).unwrap();
}

/// A bundle `name` under `dir` whose two seats, `work` then `review`, are
/// each `seat`.
fn bundle(dir: &Path, name: &str, seat: Value) {
    std::fs::create_dir_all(dir.join("roles")).unwrap();
    std::fs::write(dir.join("roles/seat.md"), "# seat\n").unwrap();
    let rule = |from: &str, next: &str| json!({"id": from, "from": from, "result": "complete", "next": next, "reason": from});
    write(
        &dir.join("policy.json"),
        &json!({"initial": "work", "phases": ["work", "review", "done"], "terminal": ["done"],
                "rules": [rule("work", "review"), rule("review", "done")]}),
    );
    write(
        &dir.join("bundle.json"),
        &json!({"name": name, "policy": "policy.json",
                "seats": {"work": seat, "review": seat}}),
    );
}

/// Two workspaces under one v6 map: `a`, a `harness` realm granting
/// nothing, and `b`, an `open` realm granting `web-search` through the
/// shipped Codex dialect. Each holds the shipped adapters, an exec bundle
/// `bundle` named for it, and a bundle `searcher` whose Codex seat
/// requires `web-search`.
struct Fleet {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Fleet {
    fn new() -> Fleet {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        write(
            &root.join("realms.json"),
            &json!({"schema": "forge.realms/v6", "journal": "forge.db", "realms": [
                {"name": "a", "path": "a", "default_branch": "main", "boundary": "harness"},
                {"name": "b", "path": "b", "default_branch": "main", "boundary": "open",
                 "capabilities": {"web-search": {"dialect": "codex-native-search"}}}]}),
        );
        copy_shipped(&root, "capabilities/web-search.json");
        copy_shipped(&root, "dialects/tools/codex-native-search.json");
        let searcher = json!({"role": "roles/seat.md", "results": ["complete"],
            "tools": {"sandbox": "workspace-write"},
            "capabilities": {"web-search": "requires"},
            "driver": {"command": ["{brokkr}", "driver", "codex", "--",
                                   "--model", "gpt-6-astra", "--effort", "high"]}});
        for name in ["a", "b"] {
            let workspace = root.join(name);
            for adapter in ["claude", "codex", "dsh", "exec", "lanetally"] {
                copy_shipped(&workspace, &format!("adapters/{adapter}.json"));
            }
            let exec = json!({"role": "roles/seat.md", "results": ["complete"],
                "driver": {"command": ["{brokkr}", "driver", "exec", "--", "true"]}});
            bundle(&workspace.join("bundle"), &format!("bundle-{name}"), exec);
            bundle(&workspace.join("searcher"), "searcher", searcher.clone());
        }
        Fleet { _dir: dir, root }
    }

    fn workspace(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn journal(&self) -> PathBuf {
        self.root.join("forge.db")
    }

    fn named(&self) -> RunMap {
        RunMap::Named(World::load(&self.root.join("realms.json")).unwrap())
    }

    /// The payload a launch of `bundle` from workspace `name` is queued
    /// as, every path the request named relative: the shape the queue
    /// wrote before a queued launch anchored its paths (7e64c72f).
    fn relative(&self, name: &str, bundle: &str, map: RunMap) -> String {
        let request = LaunchRequest {
            workspace: self.workspace(name),
            bundle: BundleSource::Dir(bundle.into()),
            journal: self.journal(),
            repo: Some(".".into()),
            secrets: Some("secrets.env".into()),
            host_path: Default::default(),
        };
        let run = NewRun {
            feature: "queued".into(),
            map,
            dispatch: None,
        };
        let mut queued = QueuedLaunch::of(&request, &run).unwrap();
        assert_eq!(queued.encoding, Encoding::V1);
        queued.bundle = BundleSource::Dir(bundle.into());
        queued.repo = Some(".".into());
        queued.secrets = Some("secrets.env".into());
        queued.encode().unwrap()
    }

    /// `payload` rebuilt and started where the process stands now.
    fn start(&self, payload: &str) -> Result<Engine, LaunchError> {
        let path = std::env::var_os("PATH").unwrap_or_default();
        let (request, run) = QueuedLaunch::decode(payload)
            .unwrap()
            .rebuild(self.journal(), path)?;
        launch::start(request, run, &mut |_| {})
    }

    fn runs(&self) -> Vec<String> {
        let store = Store::open(&self.journal()).unwrap();
        let runs = store.list_runs().unwrap();
        runs.into_iter().map(|(run, ..)| run).collect()
    }
}

/// The bundle, realm and boundary an engine was admitted with, and the
/// repository it operates.
fn admitted(engine: &Engine) -> (PathBuf, String, Value, Boundary, PathBuf) {
    let bundle = &engine.bundle;
    let realm = bundle.manifest["capabilities"]["realm"].clone();
    let (dir, name) = (bundle.dir.clone(), bundle.name.clone());
    (dir, name, realm, bundle.boundary, engine.repo.clone())
}

/// A payload that holds relative paths, as the queue once wrote them, is
/// rebuilt against its own workspace: started from `b`, it is admitted
/// with `a`'s bundle, in realm `a`, under `a`'s boundary, operating `a`.
#[test]
fn a_relative_payload_started_elsewhere_is_admitted_in_its_own_workspace() {
    let fleet = Fleet::new();
    let payload = fleet.relative("a", "bundle", fleet.named());
    let _standing = Standing::in_dir(&fleet.workspace("b"));
    let engine = fleet.start(&payload).unwrap();
    let a = fleet.workspace("a");
    assert_eq!(
        admitted(&engine),
        (
            a.join("bundle"),
            "bundle-a".to_string(),
            json!("a"),
            Boundary::Harness,
            a.join(".")
        )
    );
    assert_eq!(engine.secrets_file, Some(a.join("secrets.env")));
}

/// An entry queued with no map and no repository named is rebuilt naming
/// its workspace as the repository, so the engine operates that tree and
/// not the directory the dispatcher stands in.
#[test]
fn an_unmapped_entry_queued_without_a_repository_operates_its_workspace() {
    let fleet = Fleet::new();
    let a = fleet.workspace("a");
    let request = LaunchRequest {
        workspace: a.clone(),
        bundle: BundleSource::Dir("bundle".into()),
        journal: fleet.journal(),
        repo: None,
        secrets: None,
        host_path: Default::default(),
    };
    let run = NewRun {
        feature: "queued".into(),
        ..NewRun::default()
    };
    let payload = QueuedLaunch::of(&request, &run).unwrap().encode().unwrap();
    let (rebuilt, _) = QueuedLaunch::decode(&payload)
        .unwrap()
        .rebuild(fleet.journal(), Default::default())
        .unwrap();
    assert_eq!(rebuilt.repo, Some(a.clone()));
    let _standing = Standing::in_dir(&fleet.workspace("b"));
    let engine = fleet.start(&payload).unwrap();
    assert_eq!(
        admitted(&engine),
        (
            a.join("bundle"),
            "bundle-a".to_string(),
            json!("<unmapped>"),
            Boundary::Namespace,
            a
        )
    );
}

/// A seat that requires a capability its queued realm does not grant is
/// refused when the entry is started from a realm that would grant it,
/// before a run row is written. The same launch queued in `b` is admitted,
/// so it is `a`'s refusal, not a grant that could not hold anywhere.
#[test]
fn a_requirement_refused_in_the_queued_realm_stays_refused_from_a_realm_that_grants_it() {
    let fleet = Fleet::new();
    let granted = fleet.relative("b", "searcher", fleet.named());
    let refused = fleet.relative("a", "searcher", fleet.named());
    let _standing = Standing::in_dir(&fleet.workspace("b"));
    let held = fleet.start(&granted).unwrap();
    assert_eq!(held.bundle.manifest["capabilities"]["realm"], "b");
    let before = fleet.runs();
    assert_eq!(before, std::slice::from_ref(&held.run_id));
    let Err(error) = fleet.start(&refused) else {
        panic!("admitted from b: {:?}", fleet.runs());
    };
    let LaunchError::Compile(CompileError::Capability(ref reason)) = error else {
        panic!("not a capability refusal: {error:?}");
    };
    let said = "seat 'review' (office 'review') in realm 'a': requires capability 'web-search' \
                but the realm does not grant it to this office";
    assert_eq!(reason, said);
    assert_eq!(error.to_string(), format!("bundle: {said}"));
    assert_eq!(fleet.runs(), before);
}
