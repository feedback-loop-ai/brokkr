//! Every sealed Codex launch is handed the secrets store its permission
//! profile denies (decision 0065 slice two, U9c2; SI2): the engine hands it
//! to each seat whose body runs Codex, and the driver's door refuses a
//! sealed launch handed none. The launches here are handed it as the engine
//! hands it, so each argv pinned here is the profile form a Codex seat is
//! served, never the pre-U9c2 `--sandbox` one.

use super::*;

/// The store the launches here are handed: like their `/w` workdir, a path
/// that names nothing on the host.
pub(super) const STORE: &str = "/s/secrets.env";

/// The three isolation options codex-cli 0.162.1 was measured under, after
/// the composed arguments.
pub(super) const ISOLATING: [&str; 5] = [
    "--ignore-user-config",
    "--disable",
    "apps",
    "--disable",
    "plugins",
];

/// The model and effort most launches here are pinned to.
pub(super) const ASTRA: (&str, &str) = ("gpt-6-astra", "high");

/// `parts`, owned.
pub(super) fn owned(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| part.to_string()).collect()
}

/// The cold command on `program` of an unboxed Codex seat on `model` at
/// `effort` whose class is `class`: the profile leading, then the pins,
/// `rest` (a gate's capture), the isolation options, and web search
/// switched off.
pub(super) fn profiled_cold(
    program: &str,
    (model, effort): (&str, &str),
    class: &str,
    rest: &[&str],
) -> Vec<String> {
    let effort = format!("model_reasoning_effort=\"{effort}\"");
    let pins = owned(&["-c", &effort, "--model", model]);
    let lead = owned(&[program, "exec", "--json", "-C", "/w"]);
    let rest = [owned(rest), owned(&ISOLATING), owned(&OFF)].concat();
    [lead, profile(class).to_vec(), pins, rest].concat()
}

/// The rejoin of an unboxed `workspace-write` Codex seat on `gpt-6-astra`
/// at `high`, on `program`: the profile re-imposed where `sandbox_mode` was,
/// the pins, the isolation options, web search switched off where `off`,
/// then the offered thread and the stdin positional.
pub(super) fn profiled_rejoin(program: &str, off: bool) -> Vec<String> {
    let mut argv = owned(&[program, "exec", "resume", "--json"]);
    argv.extend(profile("workspace-write"));
    let pins = [
        "-c",
        "model_reasoning_effort=\"high\"",
        "--model",
        "gpt-6-astra",
    ];
    argv.extend(owned(&[&pins[..], &ISOLATING].concat()));
    if off {
        argv.extend(owned(&OFF));
    }
    argv.extend(owned(&[THREAD, "-"]));
    argv
}

/// What an unboxed `workspace-write` Codex seat with harness hands is
/// served when offered [`profiled_rejoin`]'s thread, web search switched
/// off where `off`. On Linux, its cold replacement: U1d3 recorded that
/// replacement as U0b measured it (X17, X24), so the launch is isolated,
/// and the rejoin declines, because U0b measured no rejoin with hands (X20)
/// and no `work-site` shape is declared under them. On macOS no shape is
/// measured, the standing fence serves the launch as before, and the
/// rejoin is the actual `exec resume`.
pub(super) fn harnessed_rejoin(program: &str, off: bool) -> Vec<String> {
    if cfg!(target_os = "macos") {
        return profiled_rejoin(program, off);
    }
    let mut argv = profiled_cold(program, ASTRA, "workspace-write", &[]);
    if !off {
        argv.truncate(argv.len() - OFF.len());
    }
    argv
}

/// `codex_command` over `input` handed [`STORE`], as the engine hands every
/// Codex seat its store.
pub(super) fn codex_served(
    bin: &str,
    extra: &[String],
    session: Option<&str>,
    input: &Value,
) -> Result<Vec<String>, String> {
    let mut input = input.clone();
    input["secrets_file"] = json!(STORE);
    brokkr_protocol::adapters::codex_command(bin, extra, "/w", session, &input)
}

/// The cold command a sealed `spawn` served by `provider` is composed into
/// over `input`, on the provider's own binary name: a Codex seat handed the
/// store ([`codex_served`]), any other as handed.
pub(super) fn cold_served(
    provider: &str,
    spawn: &brokkr_runtime::engine::SiteSpawn,
    input: &Value,
) -> Result<Vec<String>, String> {
    let argv = &spawn.argv;
    let extra = &argv[argv.iter().position(|part| part == "--").unwrap() + 1..];
    match provider {
        "codex" => codex_served("codex", extra, None, input),
        _ => brokkr_protocol::adapters::claude_command("claude", extra, None, input),
    }
}

/// The permission profile of `class`, bound to `/w` and [`STORE`], then its
/// selector: the four arguments a Codex seat's class is rendered as.
pub(super) fn profile(class: &str) -> [String; 4] {
    let written = match class {
        "workspace-write" => r#","/w"="write""#,
        "read-only" => "",
        other => panic!("no profile renders {other}"),
    };
    [
        "-c".into(),
        format!(
            r#"permissions.brokkr={{filesystem={{":root"="read"{written},"{STORE}"="deny"}}}}"#
        ),
        "-c".into(),
        r#"default_permissions="brokkr""#.into(),
    ]
}
