#![cfg(unix)]

//! A compiled route overlay, end to end: the adapter's route declares it,
//! the ENGINE binds it into the private start context, and the REAL
//! `{brokkr} driver dsh` reads that context and refuses a changed file
//! (its one read and digest equality) or an absent binding
//! (`PatchUnbound`). `route_overlay::claim` is private to
//! `brokkr-protocol`, so the honest cross-crate form is the driver the
//! engine actually spawns.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use brokkr_core::canonical::sha256_bytes;
use brokkr_core::realms::Boundary;
use brokkr_runtime::agents::{Candidate, Lowering};
use brokkr_runtime::bundle::SeatBody;
use brokkr_runtime::{Bundle, Engine, Invocation};
use brokkr_store::Store;
use serde_json::{json, Value};

/// The shipped route overlay the driver's own tests use, byte for byte.
const OVERLAY: &[u8] =
    include_bytes!("../../brokkr-protocol/src/adapters/route_overlay/spark_glm_route.yml");

const CONCRETE: &str = "spark-glm/GLM-5.3-Flash-EXL3";

fn brokkr_bin() -> &'static str {
    env!("CARGO_BIN_EXE_brokkr")
}

fn write(path: &Path, body: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

/// A composed driver that logs the one `start` it is sent and answers it,
/// written with a shebang so the composed argv's first token can be this
/// file while the adapter's template still dispatches the `dsh` kind.
fn shim(root: &Path) -> PathBuf {
    let log = root.join("start.log");
    let path = root.join("shim");
    let body = format!(
        "#!/bin/sh\n\
         log={log}\n\
         read -r hello\n\
         printf '%s\\n' '{{\"proto\":\"forge-driver/v1\",\"msg_id\":\"cap\",\"type\":\"capabilities\",\"driver\":\"test\",\"version\":\"1\",\"supports\":[]}}'\n\
         while read -r line; do\n\
         printf '%s\\n' \"$line\" > \"$log\"\n\
         case \"$line\" in *start*) break ;; esac\n\
         done\n\
         eid=$(printf '%s' \"$line\" | sed 's/.*\"effect_id\":\"\\([^\"]*\\)\".*/\\1/')\n\
         aid=$(printf '%s' \"$line\" | sed 's/.*\"attempt_id\":\"\\([^\"]*\\)\".*/\\1/')\n\
         printf '{{\"proto\":\"forge-driver/v1\",\"msg_id\":\"a\",\"type\":\"accepted\",\"effect_id\":\"%s\",\"attempt_id\":\"%s\"}}\\n' \"$eid\" \"$aid\"\n\
         printf '{{\"proto\":\"forge-driver/v1\",\"msg_id\":\"c\",\"type\":\"checkpoint\",\"effect_id\":\"%s\",\"attempt_id\":\"%s\",\"data\":{{\"step\":\"session-started\",\"session_id\":\"s1\"}}}}\\n' \"$eid\" \"$aid\"\n\
         printf '{{\"proto\":\"forge-driver/v1\",\"msg_id\":\"r\",\"type\":\"result\",\"effect_id\":\"%s\",\"attempt_id\":\"%s\",\"status\":\"succeeded\",\"result\":{{\"result\":\"complete\"}}}}\\n' \"$eid\" \"$aid\"\n\
         read -r done\n",
        log = shell_quote(&log),
    );
    std::fs::write(&path, body).unwrap();
    make_executable(&path);
    path
}

fn make_executable(path: &Path) {
    let mut permissions = std::fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).unwrap();
}

fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "'\\''"))
}

/// The shipped adapter's measured MCP carriage is what admits a strict
/// gate seat; only its provider row and invocation move for this fixture.
fn adapter() -> Value {
    let shipped = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/dsh.json"),
    )
    .unwrap();
    let mut adapter: Value = serde_json::from_str(&shipped).unwrap();
    adapter["models"] = json!({"spark": CONCRETE});
    adapter["routes"]["spark-glm"] = json!({
        "egress": "local",
        "overlay": {
            "file": "overlays/spark_glm_route.yml",
            "sha256": sha256_bytes(OVERLAY),
        },
    });
    adapter
}

/// The adapter declaring the overlay, an agent pinned through the route,
/// and a one-step recipe.
fn tree(root: &Path) -> PathBuf {
    write(&root.join("adapters/overlays/spark_glm_route.yml"), OVERLAY);
    write(&root.join("agents/charters/tester.md"), b"# tester\n");
    write(
        &root.join("agents/tester.json"),
        &serde_json::to_vec(&json!({
            "description": "the route's tester",
            "charter": "charters/tester.md",
            "models": ["spark"],
            "efforts": {},
            "limits": {"max_attempts": 1, "timeout_seconds": 5},
        }))
        .unwrap(),
    );
    write(
        &root.join("adapters/dsh.json"),
        &serde_json::to_vec(&adapter()).unwrap(),
    );
    let recipe = root.join("recipe");
    write(
        &recipe.join("policy.json"),
        &serde_json::to_vec(&json!({
            "phases": ["work", "review", "done"],
            "initial": "work",
            "terminal": ["done"],
            "rules": [
                {"id": "advance", "from": "work", "result": "complete", "next": "review", "reason": "w"},
                {"id": "close", "from": "review", "result": "clean", "next": "done", "reason": "r"},
            ],
        }))
        .unwrap(),
    );
    write(
        &recipe.join("bundle.json"),
        &serde_json::to_vec(&json!({
            "name": "route",
            "policy": "policy.json",
            "seats": {
                "work": {
                    "results": ["complete"],
                    "agent": "tester",
                    "limits": {"max_attempts": 1, "timeout_seconds": 5},
                },
                "review": {
                    "results": ["clean"],
                    "agent": "tester",
                    "limits": {"max_attempts": 1, "timeout_seconds": 5},
                },
            },
        }))
        .unwrap(),
    );
    recipe
}

fn patch_words(words: &mut [String], from: &str, to: &str) {
    for part in words.iter_mut() {
        if part == from {
            *part = to.to_string();
        }
    }
}

fn patch_candidate(candidate: &mut Candidate, from: &str, to: &str) {
    patch_words(&mut candidate.argv, from, to);
    if let Lowering::Composed(composition) = &mut candidate.lowering {
        for segment in composition.segments.iter_mut() {
            patch_words(&mut segment.argv, from, to);
        }
    }
}

/// Point every composed `{brokkr}` at `to`: the compiler expanded it to
/// this test's executable, and the run needs either the driver shim or
/// the built binary.
fn patch(bundle: &mut Bundle, to: &str) {
    let from = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    for seat in bundle.seats.values_mut() {
        match &mut seat.body {
            SeatBody::Invocation(Invocation::Single {
                command,
                candidates,
                ..
            }) => {
                patch_words(command, &from, to);
                for candidate in candidates.iter_mut() {
                    patch_candidate(candidate, &from, to);
                }
            }
            SeatBody::Invocation(Invocation::Panel { members, .. }) => {
                for member in members.iter_mut() {
                    patch_words(&mut member.command, &from, to);
                    for candidate in member.candidates.iter_mut() {
                        patch_candidate(candidate, &from, to);
                    }
                }
            }
            SeatBody::Sequence { .. } | SeatBody::Select { .. } => {}
        }
    }
}

fn compile(root: &Path, recipe: &Path, to: &str) -> Bundle {
    let mut bundle = Bundle::compile_with_realm(
        recipe,
        &root.join("agents"),
        &root.join("adapters"),
        None,
        None,
        Boundary::Namespace,
    )
    .expect("the declaring recipe compiles");
    patch(&mut bundle, to);
    bundle
}

/// The overlay fixture driven by the logging shim, which reports the
/// private context the engine built.
fn shim_fixture() -> (tempfile::TempDir, PathBuf, Bundle) {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().canonicalize().unwrap();
    let shim = shim(&root);
    let recipe = tree(&root);
    let bundle = compile(&root, &recipe, shim.to_str().unwrap());
    (home, root, bundle)
}

/// The same fixture driven by the built binary, so the real dsh driver
/// claims the overlay the engine bound.
fn real_fixture() -> (tempfile::TempDir, PathBuf, Bundle) {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().canonicalize().unwrap();
    let recipe = tree(&root);
    let bundle = compile(&root, &recipe, brokkr_bin());
    (home, root, bundle)
}

/// Run `bundle` from `run` and return its journal.
fn run(run: &Path, bundle: Bundle) -> Vec<brokkr_core::envelope::EventEnvelope> {
    std::fs::create_dir_all(run.join("work")).unwrap();
    let store = Store::open(&run.join("forge.db")).unwrap();
    let mut engine = Engine::start(store, bundle, "route", Some(run.to_path_buf())).unwrap();
    let _ = engine.drive();
    engine.store.load(&engine.run_id).unwrap()
}

/// The private start context the logging shim received.
fn start_context(root: &Path) -> Value {
    let logged = std::fs::read_to_string(root.join("start.log")).expect("the shim logs one start");
    let start: Value = serde_json::from_str(logged.trim()).expect("the logged line is a message");
    start["input"]["resume_context"].clone()
}

/// The failure text the run journaled, if any.
fn failure(events: &[brokkr_core::envelope::EventEnvelope]) -> String {
    events
        .iter()
        .rev()
        .find_map(|event| event.payload.get("reason").and_then(Value::as_str))
        .unwrap_or_default()
        .to_string()
}

#[test]
fn a_compiled_declaring_adapter_binds_its_route_overlay_at_the_private_context() {
    let (_home, root, bundle) = shim_fixture();
    let _ = run(&root, bundle);
    assert_eq!(
        start_context(&root)["route_overlay"],
        json!({
            "value": "adapters/overlays/spark_glm_route.yml",
            "digest": sha256_bytes(OVERLAY),
        })
    );
}

/// The engine never re-hashes the declared file: a byte changed after the
/// compile still travels with the adapter's declared digest, which is the
/// digest the driver's one read must reproduce.
#[test]
fn a_declared_overlay_changed_after_the_compile_still_carries_the_declared_digest() {
    let (_home, root, bundle) = shim_fixture();
    let declared = sha256_bytes(OVERLAY);
    let changed = b"- id: llm-pi-ai\n  config:\n    providers: {}\n";
    std::fs::write(root.join("adapters/overlays/spark_glm_route.yml"), changed).unwrap();
    assert_ne!(
        sha256_bytes(changed),
        declared,
        "the changed bytes are not the declared digest, so the driver's equality refuses"
    );
    let _ = run(&root, bundle);
    assert_eq!(start_context(&root)["route_overlay"]["digest"], declared);
}

#[test]
fn a_changed_declared_overlay_is_refused_by_the_drivers_digest_check() {
    let (_home, root, bundle) = real_fixture();
    std::fs::write(
        root.join("adapters/overlays/spark_glm_route.yml"),
        b"- id: llm-pi-ai\n  config:\n    providers: {}\n",
    )
    .unwrap();
    let said = failure(&run(&root, bundle));
    assert!(
        said.contains("do not hash to the bound manifest digest"),
        "the driver's digest refusal is the run's outcome: {said}"
    );
}

#[test]
fn a_library_outside_the_run_tree_refuses_the_start_with_patch_unbound() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().canonicalize().unwrap();
    let (run_root, library) = (root.join("run"), root.join("library"));
    std::fs::create_dir_all(&run_root).unwrap();
    let recipe = tree(&library);
    let bundle = compile(&library, &recipe, brokkr_bin());
    let said = failure(&run(&run_root, bundle));
    assert!(
        said.contains("a `--patch` with no bound route overlay"),
        "the driver's PatchUnbound refusal is the run's outcome: {said}"
    );
    assert!(!said.contains("do not hash"), "{said}");
}
