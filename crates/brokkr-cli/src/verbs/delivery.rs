//! The verbs that start, continue or end a run: `run`, `resume`, `rerun`,
//! `conclude` and `operator`. The first three are one admission in the
//! runtime ([`brokkr_runtime::launch`], #350); a handler here only reads
//! its arguments into a request and drives what comes back.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Result;
use brokkr_core::fold::OperatorCommand;
use brokkr_runtime::launch::{self, LaunchRequest, NewRun, RunMap};
use brokkr_runtime::realms::World;
use brokkr_runtime::{conclude as conclude_run, operator_command};
use brokkr_runtime::{Engine, FencedCommandOutcome};

use crate::cli_args::{
    ConcludeArgs, DeliveryArgs, LaunchArgs, OperatorArgs, RerunArgs, ResumeArgs, RunArgs,
};
use crate::folder::Layout;
use crate::run_view::{self, Viewer};
use crate::Invocation;
use crate::{drive_to_end, finish, open_journal, recipes, selector, supersede, Access, Exit};

/// `brokkr run`: start a new run and drive it until it parks or finishes,
/// beside its run view on a terminal (#508).
pub(crate) fn run(workspace: &Path, args: RunArgs, viewer: &Viewer) -> Result<ExitCode> {
    let (request, new) = new_run(workspace, args.launch)?;
    let journal = request.journal.clone();
    let mut engine = launch::start(request, new, &mut |note| eprintln!("{note}"))?;
    eprintln!("run started: {}", engine.run_id);
    run_view::drive(&mut engine, &journal, args.no_view, viewer)
}

/// The launch `brokkr run`'s arguments ask for; `brokkr queue add` queues
/// the same one.
pub(super) fn new_run(workspace: &Path, args: LaunchArgs) -> Result<(LaunchRequest, NewRun)> {
    // The map is read BEFORE anything is compiled, opened or spawned: a
    // named map that is missing or malformed ends the invocation here.
    let escape = args.db.is_some();
    let Invocation {
        world,
        named,
        journal,
        ..
    } = Invocation::resolve(workspace, args.realms, args.db)?.announce();
    let layout = Layout::discover(workspace)?;
    let request = request(&layout, args.delivery, (journal, escape), args.repo)?;
    let new = NewRun {
        feature: args.feature,
        map: run_map(world, named),
        dispatch: args.dispatch.map(|path| layout.typed(path)),
    };
    Ok((request, new))
}

/// Who an operator command is recorded as issued by.
pub(super) fn operator_name() -> String {
    std::env::var("USER").unwrap_or("operator".into())
}

/// The map an invocation read, as the launch weighs it: a map `--realms`
/// named is an instruction, one lying in the workspace is not.
fn run_map(world: Option<World>, named: bool) -> RunMap {
    match (world, named) {
        (Some(world), true) => RunMap::Named(world),
        (Some(world), false) => RunMap::Ambient(world),
        (None, _) => RunMap::Unmapped,
    }
}

/// The request a handler hands the launch: its arguments, and the host's
/// search path for the boundary's tool. The run's results need a place
/// before anything is compiled (decision 0083 ruling 2), and where a
/// folder is discovered its root is the workspace, its bundle the
/// default and its repository the one operated, while a path a flag
/// names is still read where it was typed ([`Layout::typed`]). `journal`
/// comes with whether `--db` named it, the escape an old layout's state
/// reads by.
fn request(
    layout: &Layout,
    DeliveryArgs {
        bundle,
        recipe,
        recipes_dir,
        secrets_file,
    }: DeliveryArgs,
    (journal, escape): (PathBuf, bool),
    repo: Option<PathBuf>,
) -> Result<LaunchRequest> {
    layout.state(escape)?;
    let typed = |path: Option<PathBuf>| path.map(|path| layout.typed(path));
    // The project's shelf, else the personal one (#697).
    let project = layout.recipes(typed(recipes_dir));
    let recipes_dir = recipes::personal::holding(recipe.as_deref(), project)?;
    Ok(LaunchRequest {
        workspace: layout.workspace(),
        bundle: recipes::source(typed(bundle), recipe, recipes_dir, || layout.bundle())?,
        journal,
        repo: typed(repo).or_else(|| layout.operated()),
        secrets: typed(secrets_file),
        host_path: std::env::var_os("PATH").unwrap_or_default(),
    })
}

/// The run `--run` names in `journal`, through decision 0015's selector,
/// and the request that continues it. The run's state needs a place
/// before the journal opens (decision 0083); the run is resolved here,
/// before the launch, which is only ever handed an id (#350), and before
/// its bundle is named, as it always was.
fn resolved(
    workspace: &Path,
    requested: &str,
    delivery: DeliveryArgs,
    (journal, escape): (PathBuf, bool),
    repo: Option<PathBuf>,
) -> Result<(String, LaunchRequest)> {
    let layout = Layout::discover(workspace)?;
    layout.state(escape)?;
    let run = selector::resolve_run(&open_journal(&journal, Access::Append)?, requested)?;
    Ok((run, request(&layout, delivery, (journal, escape), repo)?))
}

/// `brokkr resume`: continue a run under its exact pinned bundle, beside
/// its run view on a terminal (#508).
pub(crate) fn resume(workspace: &Path, args: ResumeArgs, viewer: &Viewer) -> Result<ExitCode> {
    let escape = args.journal.db.is_some();
    let journal = args.journal.journal(workspace)?;
    let named = (journal.clone(), escape);
    let (run, request) = resolved(workspace, &args.run, args.delivery, named, args.repo)?;
    let mut engine = launch::resume(request, &run)?;
    run_view::drive(&mut engine, &journal, args.no_view, viewer)
}

/// `brokkr rerun`: a past run's feature as a NEW run. It stands where
/// `run` stands (decision 0046 ruling 1; design DD6): the map is read
/// before anything opens, and the journal it names is the one the rerun
/// is written to (#374).
pub(crate) fn rerun(workspace: &Path, args: RerunArgs) -> Result<ExitCode> {
    let escape = args.journal.db.is_some();
    let Invocation { world, journal, .. } =
        Invocation::resolve(workspace, args.journal.realms, args.journal.db)?.announce();
    let named = (journal, escape);
    let (run, request) = resolved(workspace, &args.run, args.delivery, named, args.repo)?;
    let mut engine = launch::rerun(request, &run, world)?;
    let name = &engine.bundle.name;
    eprintln!("rerun of {run} as {} under {name}", engine.run_id);
    drive_to_end(&mut engine, Engine::drive)
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
    let state = conclude_run(&mut store, &run, &operator_name(), &reason)?;
    Ok(finish(&state))
}

/// `brokkr operator`: record `retry`, `stop` or `supersede`, or
/// acknowledge a quarantined run (#511).
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
        member,
    }: OperatorArgs,
) -> Result<ExitCode> {
    member_retry::validate(&command, &member)?;
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
    if command == brokkr_core::residual::SUPERSEDE {
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
    anyhow::ensure!(
        !cited,
        "--findings, --by-run, --by-seq and --by-realm belong to \
         'supersede'; '{command}' takes {}",
        member_retry::accepted_arguments(&command)
    );
    if command == "acknowledge" {
        return super::acknowledge::acknowledge(workspace, &run, &reason, journal);
    }
    let Some(verb) = OperatorCommand::parse(&command) else {
        anyhow::bail!("operator command must be 'retry', 'stop', 'supersede' or 'acknowledge'");
    };
    if !member.member.is_empty() {
        return member_retry::retry(workspace, &run, &reason, journal, member);
    }
    // The journal `run` wrote (#374): the map's, unless `--db`
    // outranks it.
    let mut store = open_journal(&journal.journal(workspace)?, Access::Append)?;
    let run = selector::resolve_run(&store, &run)?;
    let operator = operator_name();
    // The command is fenced against a concurrently-driving
    // engine, so it can come back refused. Saying "recorded"
    // there would tell the operator the opposite of what the
    // journal says.
    let outcome = operator_command(&mut store, &run, verb, &operator, &reason)?;
    disposition(outcome, &command, &run)
}

fn disposition(outcome: FencedCommandOutcome, command: &str, run: &str) -> Result<ExitCode> {
    match outcome {
        FencedCommandOutcome::Accepted { .. } => {
            eprintln!("recorded operator {command}; continue with: brokkr resume --run {run}");
            Ok(Exit::Completed.into())
        }
        FencedCommandOutcome::Rejected { reason, .. } => {
            // The reason word carries which condition it was —
            // `lost_fence` for a run that moved under the
            // operator, `after_terminal` or
            // `run_not_awaiting_operator` for a command the run
            // was never in a state to take — so this line states
            // the condition and passes the word through rather
            // than paraphrasing it into one story. The view offers the
            // same words before the command is typed (#568).
            eprintln!("{}", brokkr_view::operator_refusal(command, &reason, run));
            Ok(Exit::Failed.into())
        }
    }
}

#[cfg(test)]
mod tests;

mod member_retry;
