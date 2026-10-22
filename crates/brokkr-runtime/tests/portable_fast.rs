//! Decision 0091 ruling 3 (#717, slice 3c): `fast`'s verify and ship are
//! the folder's own, and Brokkr's verifier is `brokkr-fast`'s. Where no
//! folder stands, `fast`'s own verify fails closed having run no check,
//! and every Brokkr-only recipe compiles to the verify and ship seats it
//! compiled to before the verifier moved.

use std::path::{Path, PathBuf};

use brokkr_runtime::{Bundle, Invocation, SeatBody, StepBody};
use serde_json::Value;

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

/// `recipes/<name>` compiled in Brokkr's own library, where no folder
/// stands.
fn compile(name: &str) -> Bundle {
    let root = workspace();
    let dir = root.join("recipes").join(name);
    Bundle::compile_with(&dir, &root.join("agents"), &root.join("adapters"))
        .unwrap_or_else(|error| panic!("recipes/{name} compiles: {error}"))
}

/// The box of a verify that binds the Rust toolchain, and of a seat that
/// binds nothing, as the manifest records each.
const RUST_HANDS: &str = r#"{"binds":[{"mask":["credentials.toml","credentials"],"mode":"overlay","path":"~/.cargo"},{"mask":[],"mode":"ro","path":"~/.rustup"}],"kind":"workspace","network":false}"#;
const NO_HANDS: &str = r#"{"binds":[],"kind":"workspace","network":false}"#;

/// What fast's own verify writes, in every repository with no folder.
const FAIL_CLOSED: &str = "{\"result\": \"fail\", \"notes\": \"fast's verify is the folder's own \
    (decision 0091), and no folder stands here, so no check ran: run brokkr init in this \
    repository, or run a recipe that declares its own verify seat\"}\n";

/// Ruling 3's binding: with no folder, `fast`'s verify is its own pinned
/// script, which is not Brokkr's verifier and, run, writes `fail` with the
/// fill-in notes and nothing else.
#[test]
fn with_no_folder_fasts_verify_fails_closed_having_run_no_check() {
    let root = workspace();
    let bundle = compile("fast");
    assert_eq!(bundle.folder, None);
    let script = root.join("recipes/fast/scripts/verify-seat.sh");
    let bytes = std::fs::read(&script).unwrap();
    let verifier = std::fs::read(root.join("recipes/brokkr-fast/scripts/verify-seat.sh")).unwrap();
    assert_ne!(bytes, verifier, "fast's verify is not Brokkr's verifier");
    let digest = brokkr_core::canonical::sha256_bytes(&bytes);
    assert_eq!(
        bundle.manifest["files"]["scripts/verify-seat.sh"],
        Value::from(digest.clone())
    );
    let argv =
        "{brokkr} driver exec -- bash <root>/recipes/fast/scripts/verify-seat.sh {prompt_file}";
    let seat =
        format!("fast | verify: {argv} | hands {RUST_HANDS} | script {digest} | limits 2/3600");
    assert_eq!(exec_seat(&bundle, "verify"), seat);

    let dir = tempfile::tempdir().unwrap();
    let result = dir.path().join("results/verify.json");
    let prompt = dir.path().join("prompt.md");
    std::fs::write(
        &prompt,
        format!("Write your result to\n  {}\n", result.display()),
    )
    .unwrap();
    let ran = std::process::Command::new("bash")
        .arg(&script)
        .arg(&prompt)
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(
        (ran.status.code(), &ran.stdout[..], &ran.stderr[..]),
        (Some(0), &b""[..], &b""[..])
    );
    let written = std::fs::read_to_string(&result).unwrap();
    assert_eq!(written, FAIL_CLOSED);
    let record: Value = serde_json::from_str(&written).expect("a result record");
    assert_eq!(record["result"], "fail");
    let mut left: Vec<_> = std::fs::read_dir(dir.path().join("results"))
        .unwrap()
        .collect();
    assert_eq!(left.len(), 1, "{left:?}");
    assert_eq!(left.pop().unwrap().unwrap().path(), result);
}

/// Each site of a seat's `body`, keyed as the manifest's `hands` keys it,
/// with the argv it runs: a single, a sequence's steps, a selected case.
fn sites(key: &str, body: &SeatBody, out: &mut Vec<(String, Vec<String>)>) {
    match body {
        SeatBody::Invocation(Invocation::Single { command, .. }) => {
            out.push((key.to_owned(), command.clone()));
        }
        SeatBody::Sequence { steps } => {
            for step in steps {
                let argv = match &step.body {
                    StepBody::Invocation(Invocation::Single { command, .. }) => command,
                    StepBody::Dialect { execution } => &execution.argv,
                    StepBody::Invocation(panel) => panic!("{key}: a panel step {panel:?}"),
                };
                out.push((format!("{key}:{}", step.name), argv.clone()));
            }
        }
        other => panic!("{key} is a single or a sequence: {other:?}"),
    }
}

/// One exec seat as it compiles: each site's argv with the engine written
/// as `{brokkr}` and the library as `<root>`, with its hands and the
/// digest of the script it runs, then the seat's limits.
fn exec_seat(bundle: &Bundle, seat: &str) -> String {
    let (root, engine) = (
        workspace().display().to_string(),
        std::env::current_exe().unwrap(),
    );
    let mut found = Vec::new();
    sites(seat, &bundle.seats[seat].body, &mut found);
    let mut lines = vec![bundle.name.clone()];
    for (key, mut command) in found {
        for token in command
            .iter_mut()
            .filter(|token| Path::new(token.as_str()) == engine)
        {
            *token = "{brokkr}".into();
        }
        let digest = match command.iter().find(|token| token.ends_with(".sh")) {
            Some(script) => brokkr_core::canonical::sha256_bytes(&std::fs::read(script).unwrap()),
            None => "none".into(),
        };
        let argv = command.join(" ").replace(&root, "<root>");
        let hands = &bundle.manifest["hands"][key.as_str()];
        lines.push(format!("{key}: {argv} | hands {hands} | script {digest}"));
    }
    let limits = &bundle.seats[seat].limits;
    lines.push(format!(
        "limits {}/{}",
        limits.max_attempts, limits.timeout_seconds
    ));
    lines.join(" | ")
}

/// Every recipe in the library but the two portable ones, `fast` and
/// `node`, sorted.
fn brokkr_only() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(workspace().join("recipes"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name != "fast" && name != "node")
        .collect();
    names.sort();
    names
}

/// The digests of Brokkr's verifier, moved byte for byte from
/// `recipes/fast/scripts/` to `recipes/brokkr-fast/scripts/`, of fast's
/// shipper, and of preflight's and research's verifiers.
const VERIFIER: &str = "0ce1a4c2f557d8cc001f2b2625991cb728ed2666ad456bda7f35a58e42449313";
const SHIPPER: &str = "a78376906a41acf713cd2e310b90e57609dc3473a1f80920168588bc678b6941";
const PREFLIGHT: &str = "0973cd6ecf2316a042f71279e38f5de9b38e19f41f39e0297e362d763eb05899";
const RESEARCH: &str = "49a06816e803e6f9bcef7073ec2563ba00ca0cc4fae054cfe83631f44b68d8c0";

/// The layer Brokkr's verifier is pinned under, which before the move was
/// `fast`.
const VERIFIER_LAYER: &str = "brokkr-fast";

/// Each Brokkr-only recipe's verify and ship as `exec_seat` writes them.
fn brokkr_only_seats() -> Vec<String> {
    let (rust, none) = (RUST_HANDS, NO_HANDS);
    let exec = |key: &str, script: &str, digest: &str| {
        let argv =
            format!("{{brokkr}} driver exec -- bash <root>/recipes/{script} {{prompt_file}}");
        format!("{key}: {argv} | hands {rust} | script {digest}")
    };
    let verifier = |key: &str| {
        exec(
            key,
            &format!("{VERIFIER_LAYER}/scripts/verify-seat.sh"),
            VERIFIER,
        )
    };
    let dialect = "openspec validate --archived --strict --no-interactive";
    let checks = format!(
        "{} | verify:dialect-verify: {dialect} | hands {none} | script none",
        verifier("verify:checks")
    );
    // A recipe composed from fast: a verify, then fast's ship, each with
    // `attempts` attempts.
    let fast = |name: &str, verify: &str, attempts: u8| {
        let ship = "{brokkr} driver exec -- bash <root>/recipes/fast/scripts/ship-seat.sh \
                    {prompt_file} {brokkr}";
        vec![
            format!("{name} | {verify} | limits {attempts}/3600"),
            format!(
                "{name} | ship: {ship} | hands {none} | script {SHIPPER} | limits {attempts}/1800"
            ),
        ]
    };
    // A recipe that composes no fast: its own verify, and no ship.
    let own = |name: &str, verify: String, limits: &str| {
        vec![format!("{name} | {verify} | limits {limits}")]
    };
    let (single, research) = (
        verifier("verify"),
        exec("verify", "research/roles/verify-seat.sh", RESEARCH),
    );
    [
        fast("brokkr-fast", &single, 2),
        fast("gpt-flash", &checks, 2),
        fast("landing", &single, 2),
        fast("night-shift", &checks, 1),
        fast("panel-review", &single, 2),
        own(
            "preflight",
            exec("verify", "preflight/roles/verify-seat.sh", PREFLIGHT),
            "2/5400",
        ),
        fast("release", &single, 2),
        own("research", research.clone(), "1/3600"),
        own("research-dsh", research, "1/3600"),
        fast("review-first", &single, 2),
        fast("self", &single, 2),
        fast("standby", &single, 2),
        fast("triage", &checks, 2),
        own(
            "verify",
            exec("verify", "verify/scripts/verify-seat.sh", VERIFIER),
            "2/3600",
        ),
        fast("wager-harness", &single, 2),
        fast("wager-harness-dsh", &single, 2),
        fast("wager-harness-muse", &single, 2),
    ]
    .concat()
}

/// Ruling 3's comparison: each Brokkr-only recipe compiles to the verify
/// and ship it compiled to before the verifier moved: the same argv, box,
/// limits and script bytes. Measured on the base with `VERIFIER_LAYER`
/// set to `fast`, the one difference the move makes.
#[test]
fn every_brokkr_only_recipe_compiles_the_verify_and_ship_it_did() {
    let mut seats = Vec::new();
    for name in brokkr_only() {
        let bundle = compile(&name);
        for seat in ["verify", "ship"] {
            if bundle.seats.contains_key(seat) {
                seats.push(exec_seat(&bundle, seat));
            }
        }
    }
    assert_eq!(seats, brokkr_only_seats());
}
