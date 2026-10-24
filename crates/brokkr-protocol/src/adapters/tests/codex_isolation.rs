//! A Codex seat launched under its isolation options (decision 0065 slice
//! two, U9c2; SI2; tasks 45.5 and 45.6): the exact cold, replacement and
//! rejoin argv of each class, the store's deny in each rendered profile, the
//! final check bound to the store the serving hands, the inline judgment's
//! bound reading, and the host refusal. A new child of the parent suite,
//! which sits at its line baseline; the effort pin's test moved here from
//! it when the cold builder it calls learned the store.

use super::*;
use crate::native_controls::grammar::{self, Unisolated};
use crate::native_controls::{HandsIntent, Origin, SandboxIntent, Segment};

/// The secrets store's path the fixtures' engine hands a Codex seat.
pub(super) const STORE: &str = "/s/secrets.env";

/// The isolation options, as an independent literal.
pub(super) const ISOLATION: [&str; 5] = [
    "--ignore-user-config",
    "--disable",
    "apps",
    "--disable",
    "plugins",
];

/// Each class's profile over the workdir `/w` and [`STORE`], and its
/// selector, as independent literals.
pub(super) const READ_ONLY: &str =
    r#"permissions.brokkr={filesystem={":root"="read","/s/secrets.env"="deny"}}"#;
pub(super) const WORKSPACE_WRITE: &str =
    r#"permissions.brokkr={filesystem={":root"="read","/w"="write","/s/secrets.env"="deny"}}"#;
pub(super) const SELECTOR: &str = r#"default_permissions="brokkr""#;

/// What never stands beside the profile: each spelling that sets, lifts or
/// widens the sandbox (U9c1's carried L1, which U10a closes).
const BESIDE: [&str; 7] = [
    "-s",
    "--sandbox",
    "--full-auto",
    "--dangerously-bypass-approvals-and-sandbox",
    "--ask-for-approval",
    "-a",
    "--add-dir",
];

pub(super) fn strings(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| part.to_string()).collect()
}

/// An inline site of `class` on its `local` fragment, as the engine writes
/// its input over `base`, handed the store.
fn stored_site(mut base: Value, class: SandboxIntent, local: &[&str]) -> (Vec<String>, Value) {
    base["secrets_file"] = json!(STORE);
    let pins = ["--model", "gpt-6-astra", "--effort", "high"];
    inline_codex_input(base, &pins, local, codex_denied(), class, HandsIntent::None)
}

/// A recording codex shim in `dir`, and the input of an inline site of
/// `class` on `local` whose resume shape is enabled over `dir`.
fn shimmed(dir: &Path, class: SandboxIntent, local: &[&str]) -> (String, Vec<String>, Value) {
    let shim = codex_shim(dir, "codex", &dir.join("argv"));
    let base = enabled_input(CODEX_SHAPE, CODEX_VERSION, dir);
    let (extra, input) = stored_site(base, class, local);
    (shim.to_str().unwrap().to_string(), extra, input)
}

/// The two inline classes: the work seat's, and the gate's with its capture.
fn classes() -> [(SandboxIntent, Vec<&'static str>, &'static str); 2] {
    [
        (
            SandboxIntent::WorkspaceWrite,
            vec!["--sandbox", "workspace-write"],
            WORKSPACE_WRITE,
        ),
        (
            SandboxIntent::ReadOnly,
            vec![
                "--sandbox",
                "read-only",
                "--output-last-message",
                "/w/result.json",
            ],
            READ_ONLY,
        ),
    ]
}

/// The command `lead` over the profile, the translated effort, the seat's
/// model and `capture`, the isolation options and the native OFF.
pub(super) fn isolated(lead: &[&str], profile: &str, capture: &[&str]) -> Vec<String> {
    let effort = [
        "-c",
        "model_reasoning_effort=\"high\"",
        "--model",
        "gpt-6-astra",
    ];
    [
        strings(lead),
        strings(&["-c", profile, "-c", SELECTOR]),
        strings(&effort),
        strings(capture),
        strings(&ISOLATION),
        strings(&["-c", "web_search=\"disabled\""]),
    ]
    .concat()
}

/// The cold command of an inline site on its `local` fragment behind `pins`:
/// its class's profile leading, the pins, the rest of the fragment (a gate's
/// capture), the isolation options, then `managed`.
pub(super) fn profiled_cold(pins: &[&str], local: &[&str], managed: &[&str]) -> Vec<String> {
    let profile = match local[1] {
        "read-only" => READ_ONLY,
        _ => WORKSPACE_WRITE,
    };
    let lead = [
        "codex", "exec", "--json", "-C", "/w", "-c", profile, "-c", SELECTOR,
    ];
    strings(&[&lead[..], pins, &local[2..], &ISOLATION, managed].concat())
}

/// 45.5: for each class, the cold command, the cold replacement of an offered
/// rejoin and the rejoin itself carry the three isolation options and the
/// class as the one profile denying the store, never a `--sandbox`, a
/// `sandbox_mode` or anything that widens the sandbox; each passed the
/// inline judgment and the whole final check, so the rejoin check accepts the
/// profile its own builder emits.
#[test]
fn every_codex_shape_is_launched_isolated_under_the_store_denying_profile() {
    let _env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    for (class, local, profile) in classes() {
        let (bin, extra, input) = shimmed(dir.path(), class, &local);
        let capture = &local[2..];
        let cold = isolated(&[&bin, "exec", "--json", "-C", "/w"], profile, capture);
        let mut rejoin = isolated(&[&bin, "exec", "resume", "--json"], profile, capture);
        rejoin.extend(strings(&[THREAD, "-"]));
        let (plan, _) = codex_launch_and_cold(&bin, &extra, "/w", None, &input).unwrap();
        assert_eq!(plan.command, cold, "{class:?}: cold");
        let (offered, replacement) =
            codex_launch_and_cold(&bin, &extra, "/w", Some(THREAD), &input).unwrap();
        assert_eq!(offered.rejoining.as_deref(), Some(THREAD), "{class:?}");
        assert_eq!(offered.command, rejoin, "{class:?}: rejoin");
        assert_eq!(replacement, cold, "{class:?}: replacement");
        for command in [&plan.command, &offered.command, &replacement] {
            let beside = command
                .iter()
                .find(|part| BESIDE.contains(&part.as_str()) || part.starts_with("sandbox_mode"));
            assert_eq!(beside, None, "{class:?}");
        }
    }
}

/// 45.5: Codex's resume verdict is read as every adapter's is: past the
/// lifted fence, an offer whose `work-site` shape is measured rejoins, and
/// one whose shape is unmeasured is declined `restrictions-unavailable` and
/// served as its isolated cold replacement.
#[test]
fn a_codex_offer_rejoins_only_where_its_resume_shape_is_measured() {
    let _env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    let local = ["--sandbox", "workspace-write"];
    let (bin, extra, mut input) = shimmed(dir.path(), SandboxIntent::WorkspaceWrite, &local);
    let cold = isolated(&[&bin, "exec", "--json", "-C", "/w"], WORKSPACE_WRITE, &[]);
    for (resume, rejoining, refusal) in [
        ("measured", Some(THREAD), None),
        ("unmeasured", None, Some("restrictions-unavailable")),
    ] {
        input[crate::native_controls::SERVING_INPUTS]["isolation"] = json!({
            "servers": "empty", "cold": "measured", "replacement": "measured",
            "resume": {"work-site": resume}, "fence": "lifted"});
        let (plan, _) = codex_launch_and_cold(&bin, &extra, "/w", Some(THREAD), &input).unwrap();
        assert_eq!(
            (plan.rejoining.as_deref(), plan.refusal),
            (rejoining, refusal)
        );
        if refusal.is_some() {
            assert_eq!(plan.command, cold);
        }
    }
}

/// 45.5: the final check is bound to the store and workdir the serving
/// hands, never to the profile's shape: the launch's own cold command checks
/// with its store, and refuses with no store, another store, or, for the
/// workspace-write profile, another workdir, each with its exact cause.
#[test]
fn the_final_check_binds_the_profile_to_the_store_the_serving_hands() {
    let base = json!({"workdir": "/w"});
    for (class, local, _) in classes() {
        let (extra, input) = stored_site(base.clone(), class, &local);
        let (plan, _) = codex_launch_and_cold("codex", &extra, "/w", None, &input).unwrap();
        // The edge the launch serves through, its isolation built.
        let (edge, composed) = (
            mcp::Edge::new(&input),
            composed_launch("codex", &extra, &input),
        );
        mcp::isolated("codex", &edge, &composed.unwrap().extra, false).unwrap();
        let check = |workdir: &'static str, store: Option<&'static str>| {
            let chosen = crate::native_controls::Serving {
                program: "codex",
                workdir,
                store,
                ..Default::default()
            };
            served("codex", plan.command.clone(), &extra, &edge, chosen)
        };
        let unread = |fault: Unisolated| {
            let problem = format!("cannot be read: it {fault}");
            Err(checked_refusal("codex", &problem))
        };
        assert_eq!(check("/w", Some(STORE)), Ok(plan.command.clone()));
        assert_eq!(check("/w", None), unread(Unisolated::Unhanded(5)));
        assert_eq!(
            check("/w", Some("/s/other.env")),
            unread(Unisolated::Unstored(5))
        );
        if class == SandboxIntent::WorkspaceWrite {
            assert_eq!(check("/x", Some(STORE)), unread(Unisolated::Unrooted(5)));
        }
    }
}

/// The inline judgment's segments: the template's `--json`, then the
/// engine's `local` contribution, the profile of `profile` beside the
/// isolation options, then the authored model.
fn judged_inline(profile: &str, local: &[&str], authored: &[&str]) -> Vec<Segment> {
    use crate::native_controls::Origin;
    let engine = [strings(&["-c", profile, "-c", SELECTOR]), strings(local)].concat();
    vec![
        Segment::new(Origin::Template, &strings(&["--json"])),
        Segment::new(Origin::Local, &engine),
        Segment::new(Origin::Authored, &strings(authored)),
    ]
}

/// 45.5: the inline launch judgment reads the engine's profile as U9c1's
/// final reading does, bound to the workdir and the store it is handed: the
/// site's class with the isolation options in the engine's contribution is
/// admitted; another store, none, another class's profile, an option in the
/// recipe's words and a profile beside a `--sandbox` each refuse.
#[test]
fn the_inline_judgment_reads_the_profile_bound_to_the_paths_it_is_handed() {
    use SandboxIntent::{ReadOnly, WorkspaceWrite};
    let judge = |class, segments: &[Segment], store| {
        let paths = Some(grammar::Bound {
            workdir: "/w",
            store,
        });
        grammar::judge_inline_codex_launch(class, segments, None, paths)
            .map_err(|cause| cause.to_string())
    };
    let work = judged_inline(WORKSPACE_WRITE, &ISOLATION, &["--model", "m"]);
    assert_eq!(judge(WorkspaceWrite, &work, Some(STORE)), Ok(()));
    let read = judged_inline(READ_ONLY, &[], &[]);
    assert_eq!(judge(ReadOnly, &read, Some(STORE)), Ok(()));
    let fault = |fault: Unisolated| Err(fault.to_string());
    assert_eq!(
        judge(WorkspaceWrite, &work, None),
        fault(Unisolated::Unhanded(2))
    );
    // Compile has no run, so it hands no run's paths, and every profile
    // refuses there (operator ruling of 2026-10-23).
    let compiled = grammar::judge_inline_codex_launch(WorkspaceWrite, &work, None, None);
    assert_eq!(
        compiled.map_err(|cause| cause.to_string()),
        fault(Unisolated::Unhanded(2))
    );
    let moved = Some("/s/other.env");
    assert_eq!(
        judge(WorkspaceWrite, &work, moved),
        fault(Unisolated::Unstored(2))
    );
    assert_eq!(
        judge(WorkspaceWrite, &read, Some(STORE)),
        Err(
            "carries a permission profile rendering the 'read-only' class where the site's is \
             'workspace-write', and one launch runs one class"
                .to_string()
        )
    );
    let authored = judged_inline(WORKSPACE_WRITE, &[], &ISOLATION[..1]);
    assert_eq!(
        judge(WorkspaceWrite, &authored, Some(STORE)),
        Err(
            "carries '--ignore-user-config' (argument 6) in its `authored` contribution, a \
             capability-bearing control outside the closed set an inline Codex launch admits; \
             the launch admits only the engine's one sandbox fragment of the site's class, at a \
             gate the engine's one capture into the result path it owns, and configuration on a \
             closed allowlist, so every other effect is refused rather than reconciled or \
             ordered, whoever composed it"
                .to_string()
        )
    );
    let beside = judged_inline(WORKSPACE_WRITE, &["--sandbox", "workspace-write"], &[]);
    assert_eq!(
        judge(WorkspaceWrite, &beside, Some(STORE)),
        fault(Unisolated::Beside(6))
    );
}

/// 45.5: a host holding codex's system or managed configuration refuses a
/// Codex seat with its own cause before any provider work — an offered
/// rejoin's version probe included — and a host holding neither launches it.
/// The observation is injected; nothing under the real `/etc` is read.
#[test]
fn a_host_holding_codex_configuration_refuses_the_seat_before_any_provider_work() {
    let _env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    let ran = dir.path().join("ran");
    let shim = executable(
        dir.path(),
        "codex",
        &format!("#!/bin/sh\ntouch {}\nexit 1\n", ran.display()),
    );
    let bin = shim.to_str().unwrap();
    let base = enabled_input(CODEX_SHAPE, CODEX_VERSION, dir.path());
    let local = ["--sandbox", "workspace-write"];
    let (extra, input) = stored_site(base, SandboxIntent::WorkspaceWrite, &local);
    let held = |held: &'static str| move |path: &Path| path == Path::new(held);
    for (path, config) in [
        ("/etc/codex/config.toml", codex::HostConfig::System),
        ("/etc/codex/managed_config.toml", codex::HostConfig::Managed),
    ] {
        let refusal = codex::Unlaunchable::Host(config);
        assert_eq!(codex::admitted(&input, "/w", held(path)), Err(refusal));
        let launched = codex::launch_on_host(bin, &extra, "/w", Some(THREAD), &input, held(path));
        assert_eq!(
            launched.err(),
            Some(codex::Unlaunched::Door(refusal)),
            "{path}"
        );
        assert!(!ran.exists(), "{path}: no provider work");
    }
    let other = held("/etc/codex/other.toml");
    assert_eq!(codex::admitted(&input, "/w", other), Ok(()));
    let launched = codex::launch_on_host(bin, &extra, "/w", Some(THREAD), &input, other);
    assert_eq!(launched.err(), None);
    assert!(ran.exists(), "the admitted launch probed its version");
    // The host is observed as held wherever it is not observed absent.
    let file = dir.path().join("file");
    std::fs::write(&file, "").unwrap();
    let observed = [&file, &dir.path().join("missing"), &file.join("below")];
    let observed = observed.map(|path| codex::present(path));
    assert_eq!(observed, [true, false, true]);
}

/// 45.5 (operator ruling of 2026-10-23): a launch the engine sealed, its
/// launch record beside its sealed serving inputs, and handed no store
/// refuses at the driver's door rather than keep the `--sandbox` form. Half
/// the pair, or the plan alone, passes the door to the refusals the existing
/// checks make of it (`the_engines_plan_with_neither_sealed_input_is_refused_at_every_seam`
/// and the unpaired record's), and a driver run by hand launches as written.
#[test]
fn a_sealed_codex_launch_handed_no_store_refuses_at_the_door() {
    let local = ["--sandbox", "workspace-write"];
    let base = json!({"workdir": "/w"});
    let (extra, mut sealed) = stored_site(base.clone(), SandboxIntent::WorkspaceWrite, &local);
    sealed.as_object_mut().unwrap().remove("secrets_file");
    let absent = |_: &Path| false;
    let storeless = codex::Unlaunchable::Storeless;
    assert_eq!(codex::admitted(&sealed, "/w", absent), Err(storeless));
    let launched = codex::launch_on_host("codex", &extra, "/w", None, &sealed, absent);
    assert_eq!(launched.err(), Some(codex::Unlaunched::Door(storeless)));
    for key in [
        "launch_record",
        crate::native_controls::SERVING_INPUTS,
        "native_controls",
    ] {
        let alone = json!({"workdir": "/w", key: sealed[key].clone()});
        assert_eq!(codex::admitted(&alone, "/w", absent), Ok(()), "{key}");
    }
    assert_eq!(codex::admitted(&base, "/w", absent), Ok(()));
}

/// 45.5: the compile-time reading, [`codex_command`], is the composition
/// alone, beneath the door. A sealed inline site handed no store keeps its
/// class where the seat's `local` fragment wrote it, so the inline judgment
/// reads that fragment as the store-less form it is, and the reading returns
/// it whole. The door, not the reading, refuses to spawn it.
#[test]
fn the_compile_reading_of_a_site_handed_no_store_keeps_its_class_where_it_was_written() {
    let effort = [
        "-c",
        "model_reasoning_effort=\"high\"",
        "--model",
        "gpt-6-astra",
    ];
    let lead = ["codex", "exec", "--json", "-C", "/w"];
    let off = ["-c", "web_search=\"disabled\""];
    let absent = |_: &Path| false;
    for (class, local, _) in classes() {
        let base = json!({"workdir": "/w", "result_path": "/w/result.json"});
        let (extra, mut sealed) = stored_site(base, class, &local);
        sealed.as_object_mut().unwrap().remove("secrets_file");
        let read = codex_command("codex", &extra, "/w", None, &sealed);
        let unprofiled = strings(&[&lead[..], &effort, &local, &off].concat());
        assert_eq!(read, Ok(unprofiled), "{class:?}");
        let launched = codex::launch_on_host("codex", &extra, "/w", None, &sealed, absent);
        let storeless = codex::Unlaunched::Door(codex::Unlaunchable::Storeless);
        assert_eq!(launched.err(), Some(storeless), "{class:?}");
    }
}

/// 45.5: the same reading of an offered rejoin, handed no store, re-imposes
/// the class as the `sandbox_mode` its own fragment names, and the door
/// refuses to spawn it.
#[test]
fn the_rejoin_reading_of_a_site_handed_no_store_reimposes_its_class_as_written() {
    let _env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    let absent = |_: &Path| false;
    for (class, local, _) in classes() {
        let (bin, extra, mut input) = shimmed(dir.path(), class, &local);
        input.as_object_mut().unwrap().remove("secrets_file");
        let (offered, _) = codex_launch_and_cold(&bin, &extra, "/w", Some(THREAD), &input).unwrap();
        let mode = format!("sandbox_mode=\"{}\"", local[1]);
        let effort = [
            "-c",
            "model_reasoning_effort=\"high\"",
            "--model",
            "gpt-6-astra",
        ];
        let lead = [&bin, "exec", "resume", "--json", "-c", &mode];
        let off = ["-c", "web_search=\"disabled\"", THREAD, "-"];
        let unprofiled = strings(&[&lead[..], &effort, &local[2..], &off].concat());
        assert_eq!(offered.command, unprofiled, "{class:?}");
        let launched = codex::launch_on_host(&bin, &extra, "/w", Some(THREAD), &input, absent);
        let storeless = codex::Unlaunched::Door(codex::Unlaunchable::Storeless);
        assert_eq!(launched.err(), Some(storeless), "{class:?}");
    }
}

/// 45.5 (operator ruling of 2026-10-23, the row's recorded limitation): a
/// workspace root or store path that a TOML basic string cannot carry
/// without an escape refuses the launch, naming which path, never escaped.
#[test]
fn a_path_no_basic_string_carries_refuses_the_launch_naming_it() {
    use codex::Unlaunchable::{UncarriedRoot, UncarriedStore};
    let local = ["--sandbox", "workspace-write"];
    let base = json!({"workdir": "/w"});
    let (_, input) = stored_site(base, SandboxIntent::WorkspaceWrite, &local);
    let absent = |_: &Path| false;
    for root in ["/w\"x", "/w\\x", "/w\nx", ""] {
        assert_eq!(
            codex::admitted(&input, root, absent),
            Err(UncarriedRoot),
            "{root:?}"
        );
    }
    for store in ["/s/\"x", "/s/\\x", "/s/\tx", ""] {
        let mut moved = input.clone();
        moved["secrets_file"] = json!(store);
        let refused = codex::admitted(&moved, "/w", absent);
        assert_eq!(refused, Err(UncarriedStore), "{store:?}");
    }
    assert_eq!(codex::admitted(&input, "/w x", absent), Ok(()));
}

/// Each refusal at the driver's Codex door, its whole text pinned once, and
/// the launch seam's, which keeps the door's or the composition's words.
#[test]
fn each_codex_door_refusal_keeps_its_text() {
    use codex::{HostConfig, Unlaunchable};
    let host = |text: &str| {
        format!(
            "refusing to invoke the agent CLI: this host holds codex's {text}, which codex loads \
             whatever options it is passed, so an MCP server it names would start beside the \
             engine's set; a Codex seat is refused on such a host before any provider work \
             (decision 0065 slice two, U9c2; SI2)"
        )
    };
    let uncarried = |path: &str, verb: &str| {
        format!(
            "refusing to invoke the agent CLI: the {path} cannot be carried by a TOML basic \
             string without an escape (it is empty or holds a quote, a backslash or a control \
             character), so the permission profile cannot {verb} it; such a path is refused, \
             never escaped (decision 0065 slice two, U9c2; SI2)"
        )
    };
    let texts = [
        Unlaunchable::Host(HostConfig::System),
        Unlaunchable::Host(HostConfig::Managed),
        Unlaunchable::Storeless,
        Unlaunchable::UncarriedRoot,
        Unlaunchable::UncarriedStore,
    ]
    .map(|refusal| refusal.to_string());
    assert_eq!(
        texts,
        [
            host("system configuration at '/etc/codex/config.toml'"),
            host("managed configuration at '/etc/codex/managed_config.toml'"),
            "refusing to invoke the agent CLI: this Codex launch is sealed by the engine but \
             handed no secrets store's path, so its sandbox class cannot be rendered as the \
             permission profile denying the store; a sealed Codex launch is never served under \
             '--sandbox', which leaves the store readable (decision 0065 slice two, U9c2; SI2)"
                .to_string(),
            uncarried("seat's workspace root", "name"),
            uncarried("secrets store's path", "deny"),
        ]
    );
    // The launch seam's refusal is the door's text, or the composition's.
    let door = codex::Unlaunched::Door(Unlaunchable::Storeless).to_string();
    let composed = codex::Unlaunched::Composition("composed".into()).to_string();
    assert_eq!([door, composed], [texts[2].clone(), "composed".into()]);
}

/// 45.5: an agent's class rides its hands, composed from the shipped
/// adapter's `hands.harness.work` and `hands.harness.gate` fragments, read
/// from the committed file, and is launched as the same profile denying the
/// store: the exact cold, rejoin and cold replacement argv of each.
#[test]
fn an_agents_harness_fragment_is_launched_as_the_store_denying_profile() {
    let _env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    let shim = codex_shim(dir.path(), "codex", &dir.path().join("argv"));
    let bin = shim.to_str().unwrap();
    let shipped = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/codex.json");
    let adapter: Value = serde_json::from_slice(&std::fs::read(shipped).unwrap()).unwrap();
    let harness = &adapter["hands"]["harness"];
    for (class, fragment, profile) in [
        (
            SandboxIntent::WorkspaceWrite,
            &harness["work"],
            WORKSPACE_WRITE,
        ),
        (SandboxIntent::ReadOnly, &harness["gate"], READ_ONLY),
    ] {
        let declared: Vec<String> = serde_json::from_value(fragment.clone()).unwrap();
        let hands = declared
            .iter()
            .map(|part| part.replace("{result_path}", "/w/result.json"));
        let hands: Vec<String> = hands.collect();
        let local: Vec<&str> = hands.iter().map(String::as_str).collect();
        // Sealed as dispatch seals an agent unboxed under `harness`: its
        // fragment the engine's hands, the boundary the adapter's fragment.
        let mut seal = Seal::authored(4);
        seal.runs.push((Origin::Hands, hands.len()));
        (seal.dialect.boundary, seal.spec) = (declared, Some(crate::hands::HandsSpec::default()));
        seal.dialect.stands = Some(crate::native_controls::SealedBoundary::Harness);
        let extra = [
            strings(&["--model", "gpt-6-astra", "--effort", "high"]),
            hands.clone(),
        ];
        let extra = extra.concat();
        let plan = codex_denied();
        let mut base = enabled_input(CODEX_SHAPE, CODEX_VERSION, dir.path());
        (base["secrets_file"], base["result_path"]) = (json!(STORE), json!("/w/result.json"));
        if class == SandboxIntent::ReadOnly {
            base["result_delivery"] = json!("last-message");
        }
        let input = engine_input(base, plan, &extra, hands.len());
        let input = sealed_pair(input, &extra, seal);
        let cold = isolated(&[bin, "exec", "--json", "-C", "/w"], profile, &local[2..]);
        let mut rejoin = isolated(&[bin, "exec", "resume", "--json"], profile, &local[2..]);
        rejoin.extend(strings(&[THREAD, "-"]));
        let (plan, _) = codex_launch_and_cold(bin, &extra, "/w", None, &input).unwrap();
        assert_eq!(plan.command, cold, "{class:?}: cold");
        let (offered, replacement) =
            codex_launch_and_cold(bin, &extra, "/w", Some(THREAD), &input).unwrap();
        assert_eq!(offered.rejoining.as_deref(), Some(THREAD), "{class:?}");
        assert_eq!(offered.command, rejoin, "{class:?}: rejoin");
        assert_eq!(replacement, cold, "{class:?}: replacement");
    }
}

/// `codex exec` takes no effort FLAG — the level is the
/// `model_reasoning_effort` config key — so the shared `--effort` pin
/// becomes the `-c key=value` override codex actually reads, cold and on
/// a resume alike. A resume that dropped it would rejoin the thread at
/// the provider's default, which is the silent-substitution case the
/// whole decision refuses.
#[test]
fn the_codex_arms_turn_the_effort_pin_into_the_config_override_it_reads() {
    assert_eq!(
        codex_effort_config("xhigh"),
        "model_reasoning_effort=\"xhigh\""
    );

    let s = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let cold = codex_cold("codex", &s(&["--effort", "high", "--x"]), "/w", None, &[]);
    assert_eq!(
        cold,
        s(&[
            "codex",
            "exec",
            "--json",
            "-C",
            "/w",
            "-c",
            "model_reasoning_effort=\"high\"",
            "--x"
        ])
    );
    // No pin, no override: an unpinned seat's argv is what it always was.
    assert_eq!(
        codex_cold("codex", &s(&["--x"]), "/w", None, &[]),
        s(&["codex", "exec", "--json", "-C", "/w", "--x"])
    );

    // The resume re-expresses the pin, exactly as it re-expresses the
    // sandbox class, because `codex exec resume` inherits neither.
    let launch = codex_launch(
        "codex",
        &s(&["--effort", "high", "-s", "workspace-write"]),
        "/w",
        Some("0199aaaa-bbbb-cccc-dddd-eeeeeeeeeeee"),
        &enabled_assessment(CODEX_SHAPE, "0.153.4", "namespace", "boxed"),
    )
    .unwrap();
    assert!(
        launch
            .command
            .windows(2)
            .any(|pair| pair[0] == "-c" && pair[1] == "model_reasoning_effort=\"high\""),
        "{:?}",
        launch.command
    );
    // And the pin does not read as an argv a resume cannot carry: a
    // dropped session over its own effort pin would be a regression.
    // (The refusal here is the version probe's — this test does not run
    // a codex — so what is asserted is only that the pin is NOT the
    // reason: an incompatible-argv refusal would be.)
    assert_ne!(
        launch.refusal,
        Some("incompatible-argv"),
        "{:?}",
        launch.command
    );
}
