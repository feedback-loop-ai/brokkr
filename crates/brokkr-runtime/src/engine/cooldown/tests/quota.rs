//! Decision 0103 rulings 1(a) and 3 at the engine edge: a real codex
//! usage-limit rollout planted under `CODEX_HOME` is read by the production
//! adapter — the `task_complete.error` is the typed `rate_limit`, and the
//! exhausted window's `resets_at` is the provider's own retry-after — and
//! the engine's cool-down waits that reset, probes between the attempts and
//! retries once. The record-to-refusal parse is also pinned in
//! `brokkr-protocol`'s adapter suite; this case pins the seam: the parsed
//! `until`, the wait and the attempt count.

use super::*;

/// Set in the environment of this suite's own binary when the engine
/// spawns it as a driver: the case below then serves the production codex
/// adapter on its stdin and stdout instead of running as a test.
#[cfg(unix)]
const SERVE_CODEX: &str = "BROKKR_COOLDOWN_TESTS_SERVE_CODEX";

/// The one line libtest writes to stdout ahead of the served protocol when
/// this suite's binary runs one case quietly.
#[cfg(unix)]
const HARNESS_LINE: &str = "running 1 test";

/// Run the production codex adapter when this binary was spawned as the
/// engine's driver. A binary run as a test returns from here and runs the
/// case.
#[cfg(unix)]
fn serve_codex_when_spawned() {
    if std::env::var_os(SERVE_CODEX).is_none() {
        return;
    }
    let extra = std::env::var(format!("{SERVE_CODEX}_EXTRA")).unwrap_or_default();
    let extra = extra.split_whitespace().map(str::to_string).collect();
    let kind = brokkr_protocol::adapters::AdapterKind::Codex;
    let served = brokkr_protocol::adapters::serve(kind, extra, || Ok(()));
    std::process::exit(if served.is_ok() { 0 } else { 70 });
}

/// A fake codex executable: the first invocation writes the usage-limit
/// rollout and exits 0 with no result; the second answers the cool-down
/// probe; the third writes the result file the retry's prompt names.
#[cfg(unix)]
fn fake_codex(dir: &Path) -> PathBuf {
    const THREAD: &str = "01a0619c-928b-7ad3-8cc9-9eaa94c3aec1";
    let work = dir.join("work");
    let log = dir.join("work.log");
    let count = dir.join("codex.count");
    let header = serde_json::to_string(&json!({
        "timestamp": "9999-12-31T00:00:00Z",
        "type": "session_meta",
        "payload": {"id": THREAD, "timestamp": "9999-12-31T00:00:00Z",
                    "cwd": work.to_string_lossy()}
    }))
    .unwrap();
    let started = serde_json::to_string(&json!({
        "timestamp": "9999-12-31T00:00:01Z", "ordinal": 1,
        "type": "event_msg",
        "payload": {"type": "task_started", "turn_id": "t"}
    }))
    .unwrap();
    let complete = serde_json::to_string(&json!({
        "timestamp": "9999-12-31T00:00:03Z", "ordinal": 3,
        "type": "event_msg",
        "payload": {"type": "task_complete", "turn_id": "t",
                    "last_agent_message": null,
                    "error": {"message": "You have hit your usage limit.",
                              "codex_error_info": "usage_limit_exceeded"}}
    }))
    .unwrap();
    let mut script = String::from(
        r#"#!/bin/sh
n=$(cat '@COUNT@' 2>/dev/null || echo 0)
n=$((n+1))
printf '%s' "$n" > '@COUNT@'
prompt=$(cat)
case "$n" in
  1)
    printf '%s\n' 'attempt:refuse' >> '@LOG@'
    mkdir -p "$CODEX_HOME/sessions/2026/11/08"
    reset=$(($(date +%s) + 3600))
    {
      printf '%s\n' '@HEADER@' '@STARTED@'
      printf '{"timestamp":"9999-12-31T00:00:02Z","ordinal":2,"type":"event_msg","payload":{"type":"token_count","rate_limits":{"primary":{"used_percent":100.0,"resets_at":%s},"secondary":null}}}\n' "$reset"
      printf '%s\n' '@COMPLETE@'
    } > "$CODEX_HOME/sessions/2026/11/08/rollout-@THREAD@.jsonl"
    exit 0
    ;;
  2)
    printf '%s\n' 'probe' >> '@LOG@'
    printf '%s\n' '{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"PROBE-OK"}}'
    exit 0
    ;;
  *)
    printf '%s\n' 'attempt:succeed' >> '@LOG@'
    result=$(printf '%s\n' "$prompt" | grep '/.forge/results/' | head -n 1 | sed 's/^[[:space:]]*//')
    printf '%s\n' '{"result":"complete"}' > "$result"
    exit 0
    ;;
esac
"#,
    );
    for (at, with) in [
        ("@COUNT@", count.display().to_string()),
        ("@LOG@", log.display().to_string()),
        ("@HEADER@", header),
        ("@STARTED@", started),
        ("@COMPLETE@", complete),
        ("@THREAD@", THREAD.to_string()),
    ] {
        script = script.replace(at, &with);
    }
    executable(&dir.join("codex"), &script)
}

/// Write an executable script at `path` and return it.
#[cfg(unix)]
fn executable(path: &Path, body: &str) -> PathBuf {
    std::fs::write(path, body).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path.to_path_buf()
}

/// The driver: this test binary, filtered to this case, serving the
/// production codex adapter on stdin and stdout. Its argv is the driver
/// dispatch `<this> driver codex -- <extras>`, so the engine reads the site
/// as a Codex driver and seals the same extra args; the script passes the
/// extras (after the dispatch's three words) to the served adapter. Its
/// stdout is the engine's protocol; libtest's one announcement line is the
/// only line removed.
#[cfg(unix)]
fn serve_driver(dir: &Path, fake: &Path) -> PathBuf {
    let this = format!(
        "{}::a_codex_usage_limit_record_waits_for_the_probe_then_retries",
        module_path!().split_once("::").unwrap().1
    );
    let body = format!(
        r#"#!/bin/sh
shift 3
{SERVE_CODEX}=1 {SERVE_CODEX}_EXTRA="$*" CODEX_HOME='{home}' \
BROKKR_CODEX_BIN='{fake}' HOME='{root}' '{exe}' '{this}' --exact --nocapture \
--test-threads=1 -q | grep --line-buffered -v -x '{HARNESS_LINE}'
"#,
        home = dir.join("codex-home").display(),
        fake = fake.display(),
        root = dir.display(),
        exe = std::env::current_exe().unwrap().display(),
    );
    executable(&dir.join("serve-codex"), &body)
}

/// The workspace root this suite loads the shipped adapters from.
#[cfg(unix)]
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

/// Decision 0065 at the one site this case serves through the REAL codex
/// adapter: dispatch hands its driver a native plan and the charter text
/// only where the site's capability outcome was computed. Bind an empty
/// charter to the bundle's layer and the outcome the SHIPPED codex
/// declaration resolves to in a realm that grants nothing, exactly as the
/// compile writes them.
#[cfg(unix)]
fn codex_served(bundle: &mut crate::bundle::Bundle, label: &str, link: &Candidate) -> PathBuf {
    use crate::capabilities::{Authority, Serving, SiteAsks, SiteCapabilities};
    const CHARTER: &str = "roles/codex.md";
    let dir = bundle.dir.clone();
    let path = dir.join(CHARTER);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "").unwrap();
    let bound = crate::bundle::owned_input(&dir, CHARTER).ok().unwrap();
    let owner = crate::bundle::CharterOwner::Layer {
        dir: dir.clone(),
        key: CHARTER.into(),
    };
    let asks = SiteAsks::of(label, None, None).unwrap();
    let shipped = crate::Adapters::load(&workspace_root().join("adapters"))
        .expect("the shipped adapters load");
    let codex = shipped.adapter("codex").expect("the shipped codex adapter");
    let serving = Serving {
        provider: "codex",
        harness: "codex",
        model: Some(&link.model),
        native: Some((&codex.native, &codex.digest)),
        ..crate::capabilities::tests::serving(&codex.native)
    };
    let outcome = Authority::nothing(crate::capabilities::UNMAPPED, &dir)
        .resolve(&asks, &serving)
        .expect("the shipped codex declaration resolves in a realm that grants nothing");
    let site = bundle.sites.entry(label.into()).or_default();
    site.charter = Some(crate::bundle::CharterPin::of(
        owner,
        CHARTER,
        path.clone(),
        &bound,
    ));
    site.capabilities = Some(SiteCapabilities::unjudged(asks, vec![outcome]));
    path
}

#[cfg(unix)]
#[test]
fn a_codex_usage_limit_record_waits_for_the_probe_then_retries() {
    serve_codex_when_spawned();

    let dir = tempfile::tempdir().unwrap();
    let fake = fake_codex(dir.path());
    let driver = serve_driver(dir.path(), &fake);
    let argv = vec![
        driver.to_string_lossy().into_owned(),
        "driver".into(),
        "codex".into(),
        "--".into(),
        "--model".into(),
        "second/gpt".into(),
    ];
    let mut limits = work(2);
    limits.timeout_seconds = 20;
    let link = candidate("codex", "second/gpt", argv);
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    let mut compiled = bundle(dir.path(), chain(vec![link.clone()]));
    compiled.seats.get_mut("work").unwrap().limits = limits;
    let charter = codex_served(&mut compiled, "work", &link);
    let SeatBody::Invocation(Invocation::Single {
        command, role_path, ..
    }) = &mut compiled.seats.get_mut("work").unwrap().body
    else {
        panic!("the quota seat is a single invocation");
    };
    // The seat's inline command is not the one that runs (its selected
    // candidate is), but the engine reads it to learn that this site is a
    // Codex driver and must hand the sealed launch its secrets store path
    // (decision 0065 slice two, U9c2). The candidate stays the shell
    // script that serves the production adapter.
    *command = vec![
        "brokkr".into(),
        "driver".into(),
        "codex".into(),
        "--".into(),
    ];
    *role_path = charter;
    let store = brokkr_store::Store::open(&dir.path().join("forge.db")).unwrap();
    let mut engine =
        Engine::start(store, compiled, "Feature", Some(dir.path().join("work"))).unwrap();
    // The reset sits inside the route's declared maximum, so the wait is
    // taken rather than parked: the held-wait past the maximum is
    // decision 0106's, and this unit does not build it.
    engine.cooldown.host = Some(host(dir.path(), 1, 7_200));
    let (mut engine, now) = with_clock(engine);
    let refused_at = *now.lock().unwrap();
    engine.drive().unwrap();

    // The probe ran BETWEEN the refused attempt and the retry, so the
    // attempt count is two and the retry is not spent in seconds.
    assert_eq!(
        logs(dir.path(), "work"),
        ["attempt:refuse", "probe", "attempt:succeed"]
    );
    let started = starts(&engine);
    assert_eq!(started.len(), 2, "{started:?}");
    assert_eq!(started[1]["probes"][0]["provider"], "codex");
    assert_eq!(started[1]["probes"][0]["route"], "second");

    // The terminal event records the record's own reset as `until`: the
    // adapter read `resets_at` from the rollout and the engine waited it.
    let failed = failed(&engine);
    let terminal = failed
        .iter()
        .find(|event| event.payload.get("waits").is_some())
        .expect("the refused attempt records its wait");
    let waits = &terminal.payload["waits"];
    assert!(waits.is_array(), "{}", terminal.payload);
    assert_eq!(waits[0]["provider"], "codex");
    assert_eq!(waits[0]["route"], "second");
    let retry = waits[0]["retry_after_seconds"]
        .as_u64()
        .expect("the adapter parsed the record's reset");
    // The fake states `now + 3600`; the adapter reads its clock a moment
    // later, so the parsed wait is 3600 or a few seconds less. It is well
    // inside the route's 7200-second maximum either way.
    assert!((3_400..=3_600).contains(&retry), "{waits}");
    assert_eq!(waits[0]["seconds"], retry);
    assert_eq!(
        waits[0]["until"],
        (refused_at + time::Duration::seconds(retry as i64))
            .format(&Rfc3339)
            .unwrap()
    );
    // The clock crossed the reset before the retry: 0006 did not spend
    // the second attempt in seconds.
    assert!(*now.lock().unwrap() >= refused_at + time::Duration::seconds(retry as i64));
}
