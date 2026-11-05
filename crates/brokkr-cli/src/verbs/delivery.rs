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
use brokkr_runtime::{conclude as conclude_run, named_cause, operator_command, operator_stop};
use brokkr_runtime::{Cause, Engine, FencedCommandOutcome, Named, Verb};
use brokkr_store::Store;

use crate::cli_args::{
    ConcludeArgs, DeliveryArgs, JournalArgs, LaunchArgs, OperatorArgs, RerunArgs, ResumeArgs,
    RunArgs,
};
use crate::folder::Layout;
use crate::run_view::{self, Viewer};
use crate::Invocation;
use crate::{drive_to_end, finish, open_journal, recipes, selector, supersede, Access, Exit};

/// `brokkr run`: start a new run and drive it until it parks or finishes,
/// beside its run view on a terminal (#508).
pub(crate) fn run(workspace: &Path, args: RunArgs, viewer: &Viewer) -> Result<ExitCode> {
    if args.launch.dispatch.is_some() && args.entry.enter.is_some() {
        return Err(launch::LaunchError::EntryWithDispatch.into());
    }
    let (request, mut new) = new_run(workspace, args.launch)?;
    if let Some(phase) = args.entry.enter {
        let continues = if args.entry.continues.is_empty() {
            Vec::new()
        } else {
            let store = open_journal(&request.journal, Access::Read)?;
            args.entry
                .continues
                .iter()
                .map(|selector| selector::resolve_run(&store, selector))
                .collect::<Result<Vec<_>>>()?
        };
        new.entry = Some(launch::EntryRequest { phase, continues });
    }
    let journal = request.journal.clone();
    let mut engine = launch::start(request, new, &mut |note| eprintln!("{note}"))?;
    eprintln!("run started: {}", engine.run_id);
    if let Some(entry) = engine.entry() {
        eprintln!(
            "{}",
            crate::render::Safe::new(&brokkr_view::entry_sentence(entry)).as_str()
        );
    }
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
        entry: None,
        feature: args.feature,
        map: run_map(world, named),
        dispatch: args.dispatch.map(|path| layout.typed(path)),
        claim: None,
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
    // The project's shelf, else the personal one (#697); a shelf the flag
    // selects is the one the bundle composes across too (ruling 3).
    let (project, shelf) = layout.selecting(typed(recipes_dir));
    let recipes_dir = recipes::personal::holding(recipe.as_deref(), project)?;
    Ok(LaunchRequest {
        workspace: layout.workspace(),
        bundle: recipes::source(typed(bundle), recipe, recipes_dir, || layout.bundle())?,
        shelf,
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

/// `brokkr conclude`: close a stopped or parked run from its journal alone,
/// naming its cause (decision 0064 ruling 4).
pub(crate) fn conclude(
    workspace: &Path,
    ConcludeArgs {
        run,
        reason,
        cause,
        by_run,
        by_realm,
        by_ref,
        journal,
    }: ConcludeArgs,
) -> Result<ExitCode> {
    let by = (by_run, by_realm, by_ref);
    let cause = typed_cause(Verb::Conclude, cause.as_deref(), &by)?;
    let (mut store, cited, run) = closing(workspace, journal, cause.by_realm, &run)?;
    let operator = operator_name();
    let named = Named {
        cause,
        reason: &reason,
        operator: &operator,
    };
    let state = conclude_run(&mut store, &cited, &run, &named)?;
    let events = store.load(&run)?;
    Ok(finish(&state, &events))
}

/// The cause typed on `verb`, parsed where it was typed and before any
/// journal is opened (decision 0071 ruling 3): a missing `--cause` is the
/// usage refusal that names the six, and `by` are `--by-run`,
/// `--by-realm` and `--by-ref` as typed.
fn typed_cause<'a>(
    verb: Verb,
    cause: Option<&str>,
    (by_run, by_realm, by_ref): &'a (Option<String>, Option<String>, Option<String>),
) -> Result<Cause<'a>> {
    Ok(Cause {
        ending: named_cause(verb, cause)?,
        by_realm: by_realm.as_deref(),
        by_run: by_run.as_deref(),
        by_ref: by_ref.as_deref(),
    })
}

/// The journal `stop` and `conclude` write, opened as they always opened
/// it; the journal a `superseded` citation is read in, read-only — that
/// one, unless `--by-realm` names another hearth ([`citation_journal`]);
/// and the run `--run` names.
fn closing(
    workspace: &Path,
    journal: JournalArgs,
    by_realm: Option<&str>,
    run: &str,
) -> Result<(Store, Store, String)> {
    let (realms, db) = (journal.realms.clone(), journal.db.clone());
    let written = journal.journal(workspace)?;
    let store = open_journal(&written, Access::Append)?;
    let cited = match by_realm {
        None => written,
        Some(_) => citation_journal(workspace, realms, db, by_realm)?.1,
    };
    let cited = Store::open_read_only(&cited)?;
    let run = selector::resolve_run(&store, run)?;
    Ok((store, cited, run))
}

/// `brokkr operator`: record `retry`, `stop` or `supersede`, or
/// acknowledge a quarantined run (#511).
pub(crate) fn operator(
    workspace: &Path,
    OperatorArgs {
        run,
        command,
        reason,
        cause,
        findings,
        by_run,
        by_seq,
        by_realm,
        by_ref,
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
        anyhow::ensure!(
            cause.is_none() && by_ref.is_none(),
            "--cause and --by-ref belong to 'stop' and 'classify'"
        );
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
    // The two operator verbs that name how a run ended (decision 0064
    // rulings 4 and 5) take the cause and its citation, and refuse the
    // pair that is supersede's alone.
    let stopping = command == OperatorCommand::Stop.as_str();
    if stopping || command == brokkr_core::ending::CLASSIFY {
        anyhow::ensure!(
            findings.is_empty() && by_seq.is_none(),
            "--findings and --by-seq belong to 'supersede'; '{command}' takes {}",
            member_retry::accepted_arguments(&command)
        );
        let by = (by_run, by_realm, by_ref);
        if stopping {
            let cause = typed_cause(Verb::Stop, cause.as_deref(), &by)?;
            return stop(workspace, &run, &reason, cause, journal);
        }
        let cause = typed_cause(Verb::Classify, cause.as_deref(), &by)?;
        return classify(workspace, &run, &reason, cause, journal);
    }
    anyhow::ensure!(
        !cited,
        "--findings, --by-run, --by-seq and --by-realm belong to \
         'supersede'; '{command}' takes {}",
        member_retry::accepted_arguments(&command)
    );
    anyhow::ensure!(
        cause.is_none() && by_ref.is_none(),
        "--cause and --by-ref belong to 'stop' and 'classify'; '{command}' takes {}",
        member_retry::accepted_arguments(&command)
    );
    if command == "acknowledge" {
        return super::acknowledge::acknowledge(workspace, &run, &reason, journal);
    }
    let Some(verb) = OperatorCommand::parse(&command) else {
        anyhow::bail!(
            "operator command must be 'retry', 'stop', 'supersede', 'classify' or 'acknowledge'"
        );
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

/// The journal an operator annotation writes to and the journal its
/// citation is read in: `--by-realm` names the world's hearth, and no
/// realm names the workspace journal every one-hearth world has. Both
/// annotation verbs open the cited journal read-only, because a citation
/// is checked against a world the command only looks at (decision 0026
/// ruling 5).
pub(crate) fn citation_journal(
    workspace: &std::path::Path,
    realms: Option<PathBuf>,
    db: Option<PathBuf>,
    by_realm: Option<&str>,
) -> Result<(PathBuf, PathBuf)> {
    let invocation = Invocation::resolve(workspace, realms, db)?.announce();
    let cited = match by_realm {
        None => invocation.journal.clone(),
        Some(realm) => invocation
            .world
            .as_ref()
            .map(World::hearths)
            .unwrap_or_default()
            .into_iter()
            .find(|hearth| hearth.realms.iter().any(|known| known == realm))
            .map(|hearth| hearth.journal)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "--by-realm '{realm}' names no realm in this world; --realms \
                     names the map that would"
                )
            })?,
    };
    Ok((invocation.journal, cited))
}

/// The writable journal, the read-only journal a citation is checked
/// against, and the run an operator annotation names. The cited journal
/// is opened READ-ONLY because a citation is checked against a world the
/// command only looks at (decision 0026 ruling 5); the one journal it
/// writes to is the annotated run's.
pub(crate) fn annotated_run(
    workspace: &std::path::Path,
    realms: Option<PathBuf>,
    db: Option<PathBuf>,
    by_realm: Option<&str>,
    run: &str,
) -> Result<(Store, Store, String)> {
    let (journal, cited_journal) = citation_journal(workspace, realms, db, by_realm)?;
    let store = Store::open(&journal)?;
    let cited = Store::open_read_only(&cited_journal)?;
    let run = selector::resolve_run(&store, run)?;
    Ok((store, cited, run))
}

/// `brokkr operator classify` (decision 0064 ruling 5): one annotation
/// on a run that has already finished, naming how it ended. Every citation
/// is verified before anything is written and a refusal writes nothing,
/// so this reports a refusal as an error rather than as a journaled
/// rejection.
///
/// The world is opened the way the fleet verbs open it — `--realms`, an
/// ambient `./realms.json`, `--db` outranking both — because `--by-run`
/// may name a run in another hearth, and a citation into a journal this
/// process never opened is one nobody can follow back.
fn classify(
    workspace: &Path,
    run: &str,
    reason: &str,
    cause: Cause,
    journal: JournalArgs,
) -> Result<ExitCode> {
    let (realms, db) = (journal.realms, journal.db);
    let (mut store, cited, run) = annotated_run(workspace, realms, db, cause.by_realm, run)?;
    let named = Named {
        cause,
        reason,
        operator: &operator_name(),
    };
    let written = brokkr_runtime::operator_classify(&mut store, &cited, &run, &named)?;
    eprintln!(
        "recorded classify at seq {} on {run}; the run's status is untouched. Read it with: \
         brokkr inspect --run {run}",
        written.seq
    );
    Ok(Exit::Completed.into())
}

/// `brokkr operator stop` (decision 0064 ruling 4): the stop, its cause
/// riding in the command's `args`. A refused cause writes nothing; past
/// it the stop is fenced and disposed as every operator command is.
fn stop(
    workspace: &Path,
    run: &str,
    reason: &str,
    cause: Cause,
    journal: JournalArgs,
) -> Result<ExitCode> {
    let (mut store, cited, run) = closing(workspace, journal, cause.by_realm, run)?;
    let named = Named {
        cause,
        reason,
        operator: &operator_name(),
    };
    // The command is fenced against a concurrently-driving
    // engine, so it can come back refused. Saying "recorded"
    // there would tell the operator the opposite of what the
    // journal says.
    let outcome = operator_stop(&mut store, &cited, &run, &named)?;
    disposition(outcome, OperatorCommand::Stop.as_str(), &run)
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
