use super::*;
use crate::cli_args::{JournalArgs, MemberRetryArgs};
use brokkr_runtime::MemberRetryOutcome;

pub(super) fn validate(command: &str, args: &MemberRetryArgs) -> Result<()> {
    anyhow::ensure!(
        args.member.is_empty() || command == "retry",
        "--member belongs to 'retry' alone"
    );
    let typed: Vec<_> = [
        (args.repo.is_some(), "--repo"),
        (args.delivery.bundle.is_some(), "--bundle"),
        (args.delivery.recipe.is_some(), "--recipe"),
        (args.delivery.recipes_dir.is_some(), "--recipes-dir"),
        (args.delivery.secrets_file.is_some(), "--secrets-file"),
    ]
    .into_iter()
    .filter_map(|(present, name)| present.then_some(name))
    .collect();
    anyhow::ensure!(
        !args.member.is_empty() || typed.is_empty(),
        "{} belong to 'retry --member' alone",
        typed.join(", ")
    );
    Ok(())
}

pub(super) fn accepted_arguments(command: &str) -> &'static str {
    match command {
        "retry" => "--run, --reason, --realms, --db and --member, and with --member also --repo, --bundle, --recipe, --recipes-dir and --secrets-file",
        _ => "--run, --reason, --realms and --db",
    }
}

pub(super) fn retry(
    workspace: &Path,
    requested: &str,
    reason: &str,
    journal: JournalArgs,
    args: MemberRetryArgs,
) -> Result<ExitCode> {
    let escape = journal.db.is_some();
    let named = (journal.journal(workspace)?, escape);
    let (run, request) = resolved(workspace, requested, args.delivery, named, args.repo)?;
    match launch::operator_retry_members(request, &run, &args.member, &operator_name(), reason)? {
        MemberRetryOutcome::FoldRefused(refusal) => {
            eprintln!("{}", brokkr_view::member_retry_refusal(refusal.word()));
            Ok(Exit::Failed.into())
        }
        MemberRetryOutcome::CarryRefused(refusal) => {
            eprintln!("{refusal}");
            Ok(Exit::Failed.into())
        }
        MemberRetryOutcome::Disposed(outcome) => disposition(outcome, "retry", &run),
    }
}

#[cfg(test)]
#[path = "member_tests.rs"]
mod tests;
