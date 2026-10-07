//! dsh served from an engine-only `DSH_HOME` (decision 0065 slice two, U1c2;
//! requirements SI2 and MB1): only on the routes U0 and U0c measured, cold,
//! as an offer's cold replacement and never as a measured resume, with the
//! transcript, the retained root, the persistence check and the composite's
//! declared home all reading the one home the child is served from.

use super::super::super::composite::{DshComposite, DshSeams};
use super::super::super::tests::{
    binding, dsh_enabled_input, dsh_version_shim, executable, plant_dsh_session,
    synthetic_dsh_composite,
};
use super::super::super::{
    dsh_argv, dsh_launch_with, dsh_model_row, dsh_transcript_row, invoke_dsh_launch, DshLaunch,
};
use super::*;
use crate::env_guard::EnvGuard;
use crate::transcript::{DshHome, DshHomeError};
use sha2::Digest;
use std::path::PathBuf;

/// The six routes U0 and U0c qualified, each pinned as the roster pins it,
/// and whether its validated provider entry rides the seat's overlay.
const ROUTES: [(&str, &str, bool); 6] = [
    ("spark", "spark/qwen3.8-flash", true),
    ("spark-glm", "spark-glm/GLM-5.3-Flash-EXL3", true),
    ("deepseek-official", "deepseek-flash", false),
    ("dashscope", "dashscope/qwen3.8-flash", true),
    ("meta", "meta/meta/muse-spark-1.3", true),
    (
        "meta-contributor",
        "meta-contributor/meta/muse-spark-1.3-contributor",
        true,
    ),
];

const OFFERED: Option<&str> = Some("019c4b7e-0000-7000-8000-000000000001");

/// A canonical temporary root holding the operator's `HOME` and dsh home,
/// the seat's worktree and the shims, with `HOME` and `DSH_HOME` pointed at
/// the operator's.
struct Homes {
    env: EnvGuard,
    operator: PathBuf,
    user: PathBuf,
    work: PathBuf,
    bin: PathBuf,
    _dir: tempfile::TempDir,
}

impl Homes {
    fn new() -> Homes {
        let mut env = EnvGuard::lock();
        let dir = tempfile::tempdir().unwrap();
        let root = std::fs::canonicalize(dir.path()).unwrap();
        let [operator, user, work, bin] =
            ["operator", "user", "work", "bin"].map(|name| root.join(name));
        for path in [&operator, &user, &work, &bin] {
            std::fs::create_dir_all(path).unwrap();
        }
        env.set("DSH_HOME", &operator);
        env.set("HOME", &user);
        Homes {
            env,
            operator,
            user,
            work,
            bin,
            _dir: dir,
        }
    }

    /// Where the engine-only homes are staged.
    fn staged(&self) -> PathBuf {
        self.user.join(".local/state/brokkr/dsh-homes")
    }

    fn workdir(&self) -> &str {
        self.work.to_str().unwrap()
    }

    /// The seat's argv for `pinned`, and its input carrying `isolation`
    /// over `base`, bound to a route overlay written into the worktree
    /// where `routed`; and the route's bytes, empty where it has none.
    fn seat(&self, pinned: &str, routed: bool, mut base: Value) -> (Vec<String>, Value, String) {
        let mut extra = s(&["--model", pinned]);
        if !routed {
            return (extra, base, String::new());
        }
        let (provider, model) = pinned.split_once('/').unwrap();
        let route = [
            "- id: llm-pi-ai".to_string(),
            "  config:".into(),
            "    providers:".into(),
            format!("      {provider}:"),
            "        apiKeyEnv: ROUTE_KEY".into(),
            "        models:".into(),
            format!("          - id: {model}"),
            "            reasoningEfforts:".into(),
            "              high: high\n".into(),
        ]
        .join("\n");
        std::fs::write(self.work.join("route.yml"), &route).unwrap();
        let digest = hex::encode(sha2::Sha256::digest(route.as_bytes()));
        base["resume_context"]["route_overlay"] = json!({"value": "route.yml", "digest": digest});
        extra.extend(s(&["--patch", "route.yml"]));
        (extra, base, route)
    }

    /// The engine-only home `launch` was planned on: fresh under the user's
    /// state directory, outside the operator's dsh home, holding nothing but
    /// the seat's root.
    fn engine_home(&self, launch: &DshLaunch) -> PathBuf {
        let Some(home) = launch.home.child_home() else {
            panic!("an engine-only home: {:?}", launch.home)
        };
        let home = PathBuf::from(home);
        assert_eq!(home.parent(), Some(self.staged().as_path()));
        assert!(home
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("engine-home-")));
        let seat = launch.root.strip_prefix(&home).unwrap();
        assert_eq!(seat.parent(), Some(Path::new("sessions/brokkr")));
        let expected = ["sessions", "sessions/brokkr", &launch.locator].map(str::to_string);
        assert_eq!(tree(&home), expected);
        assert_eq!(launch.locator, seat.to_str().unwrap());
        home
    }
}

/// Every path under `dir`, relative and sorted.
fn tree(dir: &Path) -> Vec<String> {
    let mut paths = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in std::fs::read_dir(&next).unwrap() {
            let path = entry.unwrap().path();
            paths.push(
                path.strip_prefix(dir)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
            );
            if path.is_dir() {
                pending.push(path);
            }
        }
    }
    paths.sort();
    paths
}

/// The intent's empty set with cold and replacement measured and `resume`.
fn empty_set(base: Value, resume: &str) -> Value {
    intended(
        base,
        "empty",
        [json!("measured"), json!("measured"), json!(resume)],
    )
}

fn unreachable_composite(_: &Path) -> Result<DshComposite, String> {
    unreachable!("a closed gate never recomputes the composite")
}

/// Run `launch` on its shipped cold route through a shim that records the
/// `DSH_HOME` it was handed and the overlay it was named; report both and
/// the transcript rows the invocation published.
fn spawned(homes: &Homes, launch: DshLaunch) -> (String, String, Vec<Value>) {
    let (seen_home, seen_overlay) = (homes.bin.join("seen-home"), homes.bin.join("seen-overlay"));
    let (invoked, emitted) = invoked(homes, launch, &[]);
    invoked.unwrap();
    let rows = emitted
        .into_iter()
        .filter(|row| row["step"] == "transcript")
        .map(|row| row["transcript"].clone())
        .collect();
    let read = |path: &Path| std::fs::read_to_string(path).unwrap();
    (read(&seen_home), read(&seen_overlay), rows)
}

/// Invoke `launch` with `bindings` through production's spawn path, and
/// report the result beside every row it published.
fn invoked(
    homes: &Homes,
    launch: DshLaunch,
    bindings: &[crate::secret::BoundSecret],
) -> (Result<super::super::super::Invocation, String>, Vec<Value>) {
    let mut emitted = Vec::new();
    let invoked = invoke_dsh_launch(
        launch,
        "the prompt",
        homes.workdir(),
        bindings,
        &mut |row: &Value| emitted.push(row.clone()),
        |child| {
            child
                .try_wait()
                .map(|status| status.map(|status| status.code().unwrap_or(-1)))
        },
    );
    (invoked, emitted)
}

/// The recording shim, named under the shims' own directory.
fn recording_shim(homes: &Homes, name: &str) -> String {
    let body = format!(
        "#!/bin/sh\nprintf '%s' \"$DSH_HOME\" > '{}'\ncp \"$4\" '{}'\nexit 0\n",
        homes.bin.join("seen-home").display(),
        homes.bin.join("seen-overlay").display()
    );
    let shim = executable(&homes.bin, name, &body);
    shim.to_str().unwrap().to_string()
}

/// Each route U0 or U0c qualified is served, under the engine's intent,
/// from a fresh engine-only home that holds nothing but the seat's retained
/// root: the child is handed exactly that `DSH_HOME` and one overlay that
/// carries the route's validated provider entry where it has one, the
/// model and transcript rows, and no server row (the empty set). Its
/// transcript is published under that home. Without an intent the same
/// seat is served from the operator's home and inherits its `DSH_HOME`.
#[test]
fn each_qualified_route_is_served_cold_from_a_fresh_engine_only_home() {
    let homes = Homes::new();
    for (route, pinned, routed) in ROUTES {
        let base = json!({"workdir": homes.workdir()});
        let (extra, input, route_text) = homes.seat(pinned, routed, empty_set(base, "unmeasured"));
        let shim = recording_shim(&homes, &format!("dsh-{route}"));
        let launch = dsh_launch_with(
            &shim,
            &extra,
            homes.workdir(),
            None,
            &input,
            unreachable_composite,
        )
        .unwrap();
        let home = homes.engine_home(&launch);
        let overlay = std::fs::read_to_string(launch.overlay.path()).unwrap();
        let rows = [dsh_model_row(pinned), dsh_transcript_row(&launch.root)].map(Result::unwrap);
        assert_eq!(
            overlay,
            format!("{route_text}{}{}", rows[0], rows[1]),
            "{route}"
        );
        let named = launch.overlay.path().to_str().unwrap().to_string();
        assert_eq!(
            (&launch.command, launch.rejoining.as_deref(), launch.refusal),
            (
                &s(&[&shim, "--profile", "headless", "--patch", &named]),
                None,
                None
            ),
            "{route}"
        );
        let locator = launch.locator.clone();
        let (seen_home, seen_overlay, transcripts) = spawned(&homes, launch);
        assert_eq!(
            (seen_home.as_str(), seen_overlay),
            (home.to_str().unwrap(), overlay)
        );
        assert_eq!(
            transcripts,
            [json!({"kind": "dsh-session", "locator": locator, "home": home})],
            "{route}"
        );
    }
    let (extra, input, _) =
        homes.seat("deepseek-flash", false, json!({"workdir": homes.workdir()}));
    let shim = recording_shim(&homes, "dsh-operator");
    let launch = dsh_launch_with(
        &shim,
        &extra,
        homes.workdir(),
        None,
        &input,
        unreachable_composite,
    )
    .unwrap();
    assert_eq!(launch.home, DshHome::Operator(homes.operator.clone()));
    assert_eq!(
        launch.root.parent(),
        Some(homes.operator.join("sessions/brokkr").as_path())
    );
    let (seen_home, _, transcripts) = spawned(&homes, launch);
    assert_eq!(seen_home, homes.operator.to_str().unwrap());
    assert_eq!(transcripts[0]["home"], json!(homes.operator));
}

/// An offer is admitted only as its cold replacement on every qualified
/// route: a closed gate names its own reason, and an open gate whose rejoin
/// the intent does not admit is declined as `restrictions-unavailable`
/// before any probe or composite, never recorded as a measured resume. The
/// same open gate with no intent probes, as before.
#[test]
fn an_offer_on_an_engine_only_home_is_its_cold_replacement() {
    let homes = Homes::new();
    for (route, pinned, routed) in ROUTES {
        let base = json!({"workdir": homes.workdir()});
        let (extra, input, _) = homes.seat(pinned, routed, empty_set(base, "unmeasured"));
        let launch = dsh_launch_with(
            "dsh-does-not-run",
            &extra,
            homes.workdir(),
            OFFERED,
            &input,
            unreachable_composite,
        )
        .unwrap();
        homes.engine_home(&launch);
        assert_eq!(
            (
                launch.rejoining,
                launch.refusal,
                launch.stream_json,
                launch.command.len()
            ),
            (None, Some("unsupported-resume"), false, 5),
            "{route}"
        );
    }
    let digest = "b".repeat(64);
    let shim = dsh_version_shim(&homes.bin, "dsh-open", "0.1.5-rc.1");
    let shim = shim.to_str().unwrap();
    let open = dsh_enabled_input("0.1.5-rc.1", &digest, &homes.work);
    let extra = s(&["--model", "deepseek-flash"]);
    let declined = dsh_launch_with(
        shim,
        &extra,
        homes.workdir(),
        OFFERED,
        &empty_set(open.clone(), "unmeasured"),
        |_| unreachable!("an offer the intent cannot rejoin never recomputes"),
    )
    .unwrap();
    homes.engine_home(&declined);
    assert_eq!(
        (
            declined.rejoining,
            declined.refusal,
            declined.stream_json,
            declined.observed
        ),
        (None, Some("restrictions-unavailable"), false, None)
    );
    let as_before = dsh_launch_with(shim, &extra, homes.workdir(), OFFERED, &open, |_| {
        Ok(synthetic_dsh_composite(&digest))
    })
    .unwrap();
    assert_eq!(
        (
            as_before.refusal,
            as_before.observed.as_deref(),
            &as_before.home
        ),
        (
            Some("unverified-harness"),
            Some("0.1.5-rc.1"),
            &DshHome::Operator(homes.operator.clone())
        )
    );
}

/// The composite's declared home and the persistence home an offered root
/// is checked against are the one served home. On the operator's home the
/// seam reads that home and the recorded root rejoins, as before; under the
/// intent the seam is asked of the staged home, and the operator's recorded
/// home is a different persistence home, refused as `instance-changed`.
#[test]
fn the_composite_and_the_persistence_check_read_the_served_home() {
    let homes = Homes::new();
    let digest = "b".repeat(64);
    let shim = dsh_version_shim(&homes.bin, "dsh-warm", "0.1.5-rc.1");
    plant_dsh_session(
        &homes.operator,
        "sessions/brokkr/seat-1",
        "--w--",
        "session-1",
        4,
    );
    let mut open = dsh_enabled_input("0.1.5-rc.1", &digest, &homes.work);
    open["resume_context"]["originating_harness_version"] = json!("0.1.5-rc.1");
    open["resume_context"]["originating_wrapper_digest"] = json!(digest);
    open["resume_context"]["owned_target"] = json!({
        "provider_id": "session-1",
        "persistence_locator": "sessions/brokkr/seat-1",
        "persistence_home": homes.operator,
    });
    let extra = s(&["--model", "deepseek-flash"]);
    let asked = std::cell::RefCell::new(Vec::new());
    let launch = |input: &Value| {
        let shim = shim.to_str().unwrap();
        dsh_launch_with(
            shim,
            &extra,
            homes.workdir(),
            Some("session-1"),
            input,
            |home| {
                asked.borrow_mut().push(home.to_path_buf());
                Ok(synthetic_dsh_composite(&digest))
            },
        )
        .unwrap()
    };
    let warm = launch(&open);
    assert_eq!(
        (
            warm.rejoining.as_deref(),
            warm.refusal,
            warm.stream_json,
            &warm.home
        ),
        (
            Some("session-1"),
            None,
            true,
            &DshHome::Operator(homes.operator.clone())
        )
    );
    let isolated = launch(&empty_set(open, "measured"));
    let home = homes.engine_home(&isolated);
    assert_eq!(*asked.borrow(), [homes.operator.clone(), home]);
    assert_eq!(
        (isolated.rejoining, isolated.refusal, isolated.stream_json),
        (None, Some("instance-changed"), false)
    );
}

/// Missing or changed isolation evidence, a server set dsh cannot serve, a
/// route nobody measured and a credential file in the worktree each refuse
/// before anything is staged; without an intent the operator's home is
/// served as before.
#[test]
fn missing_evidence_and_a_worktree_credential_file_refuse_before_staging() {
    let homes = Homes::new();
    let base = json!({"workdir": homes.workdir()});
    let launch = |pinned: &str, input: &Value, offered: Option<&str>| {
        let extra = s(&["--model", pinned]);
        dsh_launch_with(
            "dsh-does-not-run",
            &extra,
            homes.workdir(),
            offered,
            input,
            |_| unreachable!("a refusal never recomputes"),
        )
    };
    let refused = |pinned: &str, input: &Value, offered| launch(pinned, input, offered).map(drop);
    let unmeasured = |shape: &str| McpRefusal::Unmeasured {
        provider: "dsh",
        shape: shape.into(),
    };
    let mut changed = empty_set(base.clone(), "unmeasured");
    changed[MCP_ISOLATION]["cold"] = json!("inherited");
    let cold = [json!("unmeasured"), json!("measured"), json!("measured")];
    let replacement = [json!("measured"), json!("unmeasured"), json!("measured")];
    for (pinned, input, offered, refusal) in [
        (
            "deepseek-flash",
            intended(base.clone(), "empty", cold),
            None,
            unmeasured("cold"),
        ),
        (
            "deepseek-flash",
            intended(base.clone(), "empty", replacement),
            OFFERED,
            unmeasured("replacement"),
        ),
        ("deepseek-flash", changed, None, McpRefusal::Unreadable),
        (
            "deepseek-flash",
            intended(base.clone(), "hands", cold_only()),
            None,
            McpRefusal::NotSealed { provider: "dsh" },
        ),
        (
            "openai/gpt-6",
            empty_set(base.clone(), "unmeasured"),
            None,
            McpRefusal::UnmeasuredRoute {
                route: "openai".into(),
            },
        ),
    ] {
        assert_eq!(
            refused(pinned, &input, offered),
            Err(refusal.at_launch()),
            "{pinned}"
        );
    }
    // Sealed hands do not make a hands set dsh can serve.
    let mut boxed = intended(base.clone(), "hands", cold_only());
    boxed[SERVING_INPUTS] = SealedServing {
        spec: Some(crate::hands::HandsSpec::default()),
        ..Default::default()
    }
    .value();
    assert_eq!(
        isolated("dsh", &Edge::new(&boxed), &[], false).map(|isolated| isolated.argv),
        Err(unmeasured("hands"))
    );
    std::fs::write(homes.work.join(".env"), "").unwrap();
    assert_eq!(
        refused(
            "deepseek-flash",
            &empty_set(base.clone(), "unmeasured"),
            None
        ),
        Err(crate::transcript::DshHomeError::WorkdirCredentials.to_string())
    );
    assert_eq!(
        (tree(&homes.operator), tree(&homes.user)),
        (Vec::<String>::new(), Vec::<String>::new()),
        "nothing staged"
    );
    let as_before = launch("deepseek-flash", &base, None).unwrap();
    assert_eq!(as_before.home, DshHome::Operator(homes.operator.clone()));
}

/// The routes are judged where the seat's argv is: each qualified route on
/// its measured row, and no other, while the shapes are judged first.
#[test]
fn dsh_is_admitted_only_on_a_route_u0_or_u0c_measured() {
    let served = |extra: &[&str], input: &Value| {
        dsh_argv(&s(extra), input, false).map(|argv| argv.isolation)
    };
    let intent = empty_set(json!({"workdir": "/w"}), "unmeasured");
    for (route, pinned, routed) in ROUTES {
        let patched = ["--model", pinned, "--patch", "route.yml"];
        let (with, without) = match routed {
            true => (&patched[..], &patched[..2]),
            false => (&patched[..2], &patched[..]),
        };
        assert_eq!(
            served(with, &intent),
            Ok(DshIsolation::Engine { rejoins: false }),
            "{route}"
        );
        let row = match routed {
            true => RouteRow::Overlay,
            false => RouteRow::Shipped,
        };
        let entry = McpRefusal::RouteEntry {
            route: route.into(),
            row,
        };
        assert_eq!(served(without, &intent), Err(entry.at_launch()), "{route}");
        assert_eq!(
            served(with, &json!({"workdir": "/w"})),
            Ok(DshIsolation::Operator),
            "{route}"
        );
    }
    let resumable = empty_set(json!({"workdir": "/w"}), "measured");
    assert_eq!(
        served(&["--model", "deepseek-flash"], &resumable),
        Ok(DshIsolation::Engine { rejoins: true })
    );
    for (extra, refusal) in [
        (
            &["--model", "deepseek/deepseek-v4-flash"][..],
            McpRefusal::UnmeasuredRoute {
                route: "deepseek".into(),
            },
        ),
        (&[][..], McpRefusal::Unpinned),
    ] {
        assert_eq!(
            served(extra, &intent),
            Err(refusal.at_launch()),
            "{extra:?}"
        );
    }
    let unmeasured = intended(
        json!({"workdir": "/w"}),
        "empty",
        [json!("unmeasured"), json!("measured"), json!("measured")],
    );
    assert_eq!(
        served(&["--model", "openai/gpt-6"], &unmeasured),
        Err(McpRefusal::Unmeasured {
            provider: "dsh",
            shape: "cold".into(),
        }
        .at_launch())
    );
}

/// The gate an engine-only home leaves: an open gate closes as
/// `restrictions-unavailable` only where the rejoin is not admitted, and
/// every closed gate keeps its own reason.
#[test]
fn an_engine_home_closes_only_a_gate_its_rejoin_is_not_admitted_through() {
    let open = || ResumeGate::Enabled {
        applies_to: "0.1.5-rc.1".into(),
    };
    let reason = |gate: ResumeGate| match gate {
        ResumeGate::Disabled(reason) => Some(reason),
        ResumeGate::Enabled { applies_to } => {
            assert_eq!(applies_to, "0.1.5-rc.1");
            None
        }
    };
    for (isolation, gate, expected) in [
        (
            DshIsolation::Engine { rejoins: false },
            open(),
            Some("restrictions-unavailable"),
        ),
        (DshIsolation::Engine { rejoins: true }, open(), None),
        (DshIsolation::Operator, open(), None),
        (
            DshIsolation::Engine { rejoins: false },
            ResumeGate::Disabled("unsupported-resume"),
            Some("unsupported-resume"),
        ),
    ] {
        assert_eq!(reason(isolation.gate(gate)), expected, "{isolation:?}");
    }
}

/// A declared binding named for any key the engine-only launch fixes in the
/// child's environment (its `DSH_HOME`, the unsigned-commit keys and the
/// host identity) refuses before any row is published or the child starts,
/// whatever its value, and the unserved home goes with its launch, as it
/// does for a launch dropped unspawned. A binding over no fixed key is
/// served: the child sees exactly the staged home, the fixed keys and the
/// binding, and the home is retained for its transcript.
#[test]
fn a_binding_over_the_engine_homes_fixed_environment_refuses_before_any_start() {
    let homes = Homes::new();
    let started = homes.bin.join("started");
    let body = format!(
        "#!/bin/sh\nprintf '%s|%s|%s|%s|%s|%s' \"$DSH_HOME\" \"$GIT_CONFIG_COUNT\" \
         \"$GIT_CONFIG_KEY_0\" \"$GIT_CONFIG_VALUE_0\" \"$GIT_AUTHOR_NAME\" \"$API_TOKEN\" > '{}'\n",
        started.display()
    );
    let shim = executable(&homes.bin, "dsh-env", &body);
    let plan = || {
        let base = json!({"workdir": homes.workdir()});
        let (extra, input, _) = homes.seat("deepseek-flash", false, empty_set(base, "unmeasured"));
        let shim = shim.to_str().unwrap();
        let mut launch = dsh_launch_with(
            shim,
            &extra,
            homes.workdir(),
            None,
            &input,
            unreachable_composite,
        )
        .unwrap();
        launch.facts.identity = vec![("GIT_AUTHOR_NAME".into(), "Seat Host".into())];
        let home = homes.engine_home(&launch);
        (launch, home)
    };
    let (unspawned, home) = plan();
    drop(unspawned);
    assert!(!home.exists(), "a launch dropped unspawned takes its home");
    let fixed = [
        "DSH_HOME",
        "GIT_CONFIG_COUNT",
        "GIT_CONFIG_KEY_0",
        "GIT_CONFIG_VALUE_0",
        "GIT_AUTHOR_NAME",
    ];
    for key in fixed {
        let (launch, home) = plan();
        // `DSH_HOME` is bound to the very home staged: the name refuses.
        let value = match key {
            "DSH_HOME" => home.to_str().unwrap(),
            _ => "tok-fixed",
        };
        let (invoked, emitted) = invoked(&homes, launch, &[binding(key, value)]);
        assert_eq!(
            invoked.map(drop),
            Err(DshHomeError::FixedBinding(key.into()).to_string()),
            "{key}"
        );
        assert_eq!(
            (emitted, started.exists(), home.exists()),
            (Vec::<Value>::new(), false, false),
            "{key}"
        );
    }
    let (launch, home) = plan();
    let (invoked, _) = invoked(&homes, launch, &[binding("API_TOKEN", "tok-3xample")]);
    assert_eq!(invoked.unwrap().exit_code, 0);
    assert_eq!(
        std::fs::read_to_string(&started).unwrap(),
        format!(
            "{}|1|commit.gpgsign|false|Seat Host|tok-3xample",
            home.display()
        )
    );
    assert_eq!(
        tree(&homes.staged()).first(),
        Some(&home.file_name().unwrap().to_string_lossy().into_owned()),
        "the served home alone is retained"
    );
}

/// The composite's declared-home seam reads the home it is handed: the
/// operator's yields exactly the seams the environment's own resolution
/// does, so the operator's composite identity is unchanged, and a staged
/// home changes the home alone.
#[test]
fn the_declared_home_seam_reads_the_home_it_is_handed() {
    let mut homes = Homes::new();
    let bin = executable(&homes.bin, "dsh", "#!/bin/sh\nexit 0\n");
    homes.env.set("BROKKR_DSH_BIN", &bin);
    let bin = bin.to_str().unwrap();
    let operator = DshSeams::resolve_declared(bin, &homes.operator).unwrap();
    assert_eq!(DshSeams::resolve(), Ok(operator.clone()));
    let staged = homes.staged().join("engine-home-1");
    assert_eq!(
        DshSeams::resolve_declared(bin, &staged),
        Ok(DshSeams {
            home: staged.clone(),
            ..operator
        })
    );
}
