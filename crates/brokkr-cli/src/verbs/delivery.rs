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
use brokkr_runtime::FencedCommandOutcome;
use brokkr_runtime::{conclude as conclude_run, operator_command};

use crate::cli_args::{ConcludeArgs, DeliveryArgs, OperatorArgs, RerunArgs, ResumeArgs, RunArgs};
use crate::Invocation;
use crate::{drive_to_end, finish, open_journal, recipes, selector, supersede, Access, Exit};

/// `brokkr run`: start a new run and drive it until it parks or finishes.
pub(crate) fn run(workspace: &Path, args: RunArgs) -> Result<ExitCode> {
    // The map is read BEFORE anything is compiled, opened or spawned: a
    // named map that is missing or malformed ends the invocation here.
    let Invocation {
        world,
        named,
        journal,
        ..
    } = Invocation::resolve(workspace, args.realms, args.db)?.announce();
    let request = request(workspace, args.delivery, journal, args.repo);
    let new = NewRun {
        feature: args.feature,
        map: run_map(world, named),
        dispatch: args.dispatch,
    };
    let mut engine = launch::start(request, new, &mut |note| eprintln!("{note}"))?;
    eprintln!("run started: {}", engine.run_id);
    drive_to_end(&mut engine)
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
/// search path for the boundary's tool.
fn request(
    workspace: &Path,
    DeliveryArgs {
        bundle,
        recipe,
        recipes_dir,
        secrets_file,
    }: DeliveryArgs,
    journal: PathBuf,
    repo: Option<PathBuf>,
) -> LaunchRequest {
    LaunchRequest {
        workspace: workspace.to_path_buf(),
        bundle: recipes::source(bundle, recipe, recipes_dir),
        journal,
        repo,
        secrets: secrets_file,
        host_path: std::env::var_os("PATH").unwrap_or_default(),
    }
}

/// The run `--run` names in `journal`, through decision 0015's selector.
/// It is resolved here, before the launch, which is only ever handed an
/// id (#350).
fn resolve_run(journal: &Path, requested: &str) -> Result<String> {
    selector::resolve_run(&open_journal(journal, Access::Append)?, requested)
}

/// `brokkr resume`: continue a run under its exact pinned bundle.
pub(crate) fn resume(workspace: &Path, args: ResumeArgs) -> Result<ExitCode> {
    let journal = args.journal.journal(workspace)?;
    let run = resolve_run(&journal, &args.run)?;
    let request = request(workspace, args.delivery, journal, args.repo);
    let mut engine = launch::resume(request, &run)?;
    drive_to_end(&mut engine)
}

/// `brokkr rerun`: a past run's feature as a NEW run. It stands where
/// `run` stands (decision 0046 ruling 1; design DD6): the map is read
/// before anything opens, and the journal it names is the one the rerun
/// is written to (#374).
pub(crate) fn rerun(workspace: &Path, args: RerunArgs) -> Result<ExitCode> {
    let Invocation { world, journal, .. } =
        Invocation::resolve(workspace, args.journal.realms, args.journal.db)?.announce();
    let run = resolve_run(&journal, &args.run)?;
    let request = request(workspace, args.delivery, journal, args.repo);
    let mut engine = launch::rerun(request, &run, world)?;
    let name = &engine.bundle.name;
    eprintln!("rerun of {run} as {} under {name}", engine.run_id);
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
            Ok(Exit::Completed.into())
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

#[cfg(test)]
mod tests;
