//! The verbs that start, continue or end a run: `run`, `resume`, `rerun`,
//! `conclude` and `operator`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use brokkr_core::fold::OperatorCommand;
use brokkr_runtime::realms::World;
use brokkr_runtime::{conclude as conclude_run, operator_command};
use brokkr_runtime::{Bundle, Engine, FencedCommandOutcome};
use brokkr_store::Store;
use serde_json::Value;

use crate::cli_args::{ConcludeArgs, OperatorArgs, RerunArgs, ResumeArgs, RunArgs};
use crate::{compile_from_manifest, compile_in_realm, drive_to_end, finish, open_journal};
use crate::{recipes, refuse_unboxable, selector, supersede, Access, Exit, Invocation};

/// `brokkr run`: start a new run and drive it until it parks or finishes.
pub(crate) fn run(
    workspace: &Path,
    RunArgs {
        bundle,
        recipe,
        recipes_dir,
        feature,
        realms,
        db,
        repo,
        dispatch,
        secrets_file,
    }: RunArgs,
) -> Result<ExitCode> {
    // The map is read BEFORE anything is compiled, opened or
    // spawned: a named map that is missing or malformed ends the
    // invocation here, with no journal touched and no seat run.
    let Invocation {
        world,
        named,
        journal: db,
        ..
    } = Invocation::resolve(workspace, realms, db)?.announce();
    // A Looper-bound run pins a run-manifest/v2, whose bytes a
    // counterpart system reads and whose round-trip reconstructs
    // the manifest from six named keys. A world cannot be pinned
    // there, so a map the operator NAMED is refused rather than
    // half-honoured — and refused HERE, in the same breath as a
    // missing or malformed map, before a bundle is compiled or a
    // journal is created.
    anyhow::ensure!(
        !(named && dispatch.is_some()),
        "a run with --dispatch cannot pin the map named by --realms: the \
         Looper-bound run-manifest/v2 lineage carries no world, and dropping \
         the map silently would leave the run unable to say which one it \
         believed in. Run without --dispatch, or without --realms, until a \
         jointly agreed v2-lineage manifest version exists"
    );
    let bundle = compile_to_start(
        workspace,
        &recipes::resolve(bundle, recipe, &recipes_dir)?,
        world.as_ref(),
        repo.as_deref(),
    )?;
    let store = open_journal(&db, Access::Append)?;
    let mut engine = if let Some(path) = dispatch {
        start_dispatched(store, bundle, &feature, repo, world.as_ref(), &path)?
    } else {
        Engine::start_in_world(store, bundle, &feature, repo, world)?
    };
    engine.secrets_file = secrets_file;
    eprintln!("run started: {}", engine.run_id);
    drive_to_end(&mut engine)
}

/// The bundle a NEW run starts under: compiled against the operated
/// repository's realm, and refused when this host cannot box its seats.
/// `run` and `rerun` both start one, so both compile it here.
fn compile_to_start(
    workspace: &Path,
    dir: &Path,
    world: Option<&World>,
    repo: Option<&Path>,
) -> Result<Bundle> {
    let operated_repo = repo.unwrap_or(workspace);
    let bundle = compile_in_realm(workspace, dir, world, operated_repo)?;
    refuse_unboxable(&bundle, &std::env::var_os("PATH").unwrap_or_default())?;
    Ok(bundle)
}

/// A Looper-bound run: the dispatch envelope at `path` is read, parsed
/// and verified against the compiled bundle before the engine starts it.
fn start_dispatched(
    store: Store,
    bundle: Bundle,
    feature: &str,
    repo: Option<PathBuf>,
    world: Option<&World>,
    path: &Path,
) -> Result<Engine> {
    // A map merely lying in the workspace is a different
    // matter: it still names the journal this world's fleet
    // writes, so the run goes there — but it is not pinned,
    // and a dropped pin is said out loud rather than left to
    // be discovered in the manifest.
    if let Some(world) = world {
        eprintln!(
            "note: {} is not pinned into this run: --dispatch writes a \
             run-manifest/v2, which carries no world",
            world.source.display()
        );
    }
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading dispatch {}", path.display()))?;
    let envelope: brokkr_core::dispatch::DispatchEnvelopeV2 =
        serde_json::from_str(&raw).context("parsing forge-dispatch/v2")?;
    envelope.verify(time::OffsetDateTime::now_utc(), &bundle.manifest_digest())?;
    Ok(Engine::start_with_dispatch(
        store, bundle, feature, repo, envelope,
    )?)
}

/// `brokkr resume`: continue a run under its exact pinned bundle.
pub(crate) fn resume(
    workspace: &Path,
    ResumeArgs {
        bundle,
        recipe,
        recipes_dir,
        run,
        journal,
        repo,
        secrets_file,
    }: ResumeArgs,
) -> Result<ExitCode> {
    let store = open_journal(&journal.journal(workspace)?, Access::Append)?;
    let run = selector::resolve_run(&store, &run)?;
    let manifest = store.manifest(&run)?;
    let bundle = compile_from_manifest(
        workspace,
        &recipes::resolve(bundle, recipe, &recipes_dir)?,
        &manifest,
    )?;
    refuse_unboxable(&bundle, &std::env::var_os("PATH").unwrap_or_default())?;
    let mut engine = Engine::resume(store, bundle, &run, repo)?;
    // Decision 0057, on decision 0046's Addendum's terms: a
    // resumed run is fenced where `run` and `rerun` are fenced,
    // before `drive()` and so before any seat spawns. Here, and
    // not inside `Engine::resume`, for two reasons that are one
    // reason: the check needs a workspace to resolve a crossing's
    // path against and the engine is given none, and this handler
    // is already where a whole-invocation refusal stands — one line
    // above, `refuse_unboxable` reads the host's PATH the same
    // way, right after the compile and right before the drive.
    //
    // The world a resumed run holds comes from its own manifest
    // and has met no disk (`World::from_manifest` resolves no
    // crossing, deliberately), so without this a run would carry
    // on over bytes its journal never saw. A Looper-bound run
    // carries no world at all and has nothing to fence.
    if let Some(world) = &engine.world {
        world.verify_crossings(workspace)?;
    }
    engine.secrets_file = secrets_file;
    drive_to_end(&mut engine)
}

/// `brokkr rerun`: a past run's feature as a NEW run.
pub(crate) fn rerun(
    workspace: &Path,
    RerunArgs {
        run,
        bundle,
        recipe,
        recipes_dir,
        journal,
        repo,
        secrets_file,
    }: RerunArgs,
) -> Result<ExitCode> {
    // A rerun is a NEW run, and it stands where `run` stands
    // (decision 0046 ruling 1; design DD6): the map is read
    // before anything opens, the world it names is pinned, and
    // the journal it names is the one the rerun is written to
    // (#374) — never the default while the world is the map's.
    let Invocation {
        world, journal: db, ..
    } = Invocation::resolve(workspace, journal.realms, journal.db)?.announce();
    let store = open_journal(&db, Access::Append)?;
    let run = selector::resolve_run(&store, &run)?;
    let events = store
        .load(&run)
        .with_context(|| format!("loading source run '{run}'"))?;
    let feature = events
        .first()
        .filter(|e| e.event_type == brokkr_core::EventType::RunStarted)
        .and_then(|e| e.payload.get("feature"))
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("source run '{run}' has no run/started feature to re-run"))?
        .to_string();
    // The bundle compiles against the operated repository's
    // realm — its boundary and its dialect — and the world is
    // pinned into the new run's manifest. A rerun that ignored
    // the realm's word would refuse on a `harness` Mac for want
    // of bubblewrap, and run boxed on a `harness` Linux while
    // the realm said otherwise.
    let bundle = compile_to_start(
        workspace,
        &recipes::resolve(bundle, recipe, &recipes_dir)?,
        world.as_ref(),
        repo.as_deref(),
    )?;
    let mut engine = Engine::start_in_world(store, bundle, &feature, repo, world)?;
    engine.secrets_file = secrets_file;
    eprintln!(
        "rerun of {run} as {} under {}",
        engine.run_id, engine.bundle.name
    );
    drive_to_end(&mut engine)
}

/// `brokkr conclude`: close a stopped or parked run from its journal alone.
pub(crate) fn conclude(
    workspace: &Path,
    ConcludeArgs {
        run,
        reason,
        journal,
    }: ConcludeArgs,
) -> Result<ExitCode> {
    let mut store = open_journal(&journal.journal(workspace)?, Access::Append)?;
    let run = selector::resolve_run(&store, &run)?;
    let operator = std::env::var("USER").unwrap_or("operator".into());
    let state = conclude_run(&mut store, &run, &operator, &reason)?;
    Ok(finish(&state))
}

/// `brokkr operator`: record `retry`, `stop` or `supersede`.
pub(crate) fn operator(
    workspace: &Path,
    OperatorArgs {
        run,
        command,
        reason,
        findings,
        by_run,
        by_seq,
        by_realm,
        journal,
    }: OperatorArgs,
) -> Result<ExitCode> {
    // Whether any argument that belongs to `supersede` alone
    // was typed. Asked HERE, before the branch below takes
    // those arguments, because on a `retry` or a `stop` one of
    // them is a refusal rather than something quietly ignored.
    let cited = [
        !findings.is_empty(),
        by_run.is_some(),
        by_seq.is_some(),
        by_realm.is_some(),
    ]
    .contains(&true);
    if command == brokkr_view::SUPERSEDE {
        return supersede(
            workspace,
            &run,
            &reason,
            &findings,
            (by_run, by_seq, by_realm),
            journal.realms,
            journal.db,
        );
    }
    let Some(verb) = OperatorCommand::parse(&command) else {
        anyhow::bail!("operator command must be 'retry', 'stop' or 'supersede'");
    };
    anyhow::ensure!(
        !cited,
        "--findings, --by-run, --by-seq and --by-realm belong to \
         'supersede'; '{command}' takes --run, --reason, --realms and --db"
    );
    // The journal `run` wrote (#374): the map's, unless `--db`
    // outranks it.
    let mut store = open_journal(&journal.journal(workspace)?, Access::Append)?;
    let run = selector::resolve_run(&store, &run)?;
    let operator = std::env::var("USER").unwrap_or("operator".into());
    // The command is fenced against a concurrently-driving
    // engine, so it can come back refused. Saying "recorded"
    // there would tell the operator the opposite of what the
    // journal says.
    match operator_command(&mut store, &run, verb, &operator, &reason)? {
        FencedCommandOutcome::Accepted { .. } => {
            eprintln!("recorded operator {command}; continue with: brokkr resume --run {run}");
            Ok(ExitCode::SUCCESS)
        }
        FencedCommandOutcome::Rejected { reason, .. } => {
            // The reason word carries which condition it was —
            // `lost_fence` for a run that moved under the
            // operator, `after_terminal` or
            // `run_not_awaiting_operator` for a command the run
            // was never in a state to take — so this line states
            // the condition and passes the word through rather
            // than paraphrasing it into one story.
            eprintln!(
                "refused operator {command} ({reason}): the run is not in a state \
                 this command can apply to; the refusal is journaled. Read it with: \
                 brokkr inspect --run {run}"
            );
            Ok(Exit::Failed.into())
        }
    }
}
