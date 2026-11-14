//! Member retry admission over the deciding fold (0093 rulings 1 and 2).
use std::path::PathBuf;

use brokkr_core::dispatch::bundle_manifest_from_run;
use brokkr_core::envelope::EventType;

use super::{
    acceptance_refusal, decided, disposed, Commanded, EngineError, EventEnvelope,
    FencedCommandOutcome, Head, OperatorCommand, RunState, Store,
};
use crate::bundle::{charters_as_started, Bundle};
use crate::engine::carry::{
    carry_plan,
    facts::{read_visit, PanelAttempt},
    record::{RetryArgs, Tag},
    CarryRefusal,
};
use crate::engine::replay::Replay;
use crate::engine::Engine;
use crate::engine::{
    charter_moved, manifest_diff, operated_repo, refuse_unbuilt, shell,
    verify_dispatch_bundle_bounds,
};

/// A refusal writes nothing; a disposition was journaled under the operator fence.
#[derive(Debug)]
pub enum MemberRetryOutcome {
    FoldRefused(brokkr_core::fold::Refusal),
    CarryRefused(CarryRefusal),
    Disposed(FencedCommandOutcome),
}

pub(crate) struct RetryRead {
    deciding: (RunState, Head),
    events: Vec<EventEnvelope>,
    attempts: Vec<PanelAttempt>,
}

impl Engine {
    /// [`Engine::resume`]'s checks and facts without its lock. A member
    /// retry reads and commands a run a live driver may hold and drives
    /// nothing, so it is no second driver (decision 0108 ruling 2); every
    /// driving entry goes through [`Engine::resume`].
    pub(crate) fn resumed(
        mut store: Store,
        bundle: Bundle,
        run_id: &str,
        repo: Option<PathBuf>,
    ) -> Result<Engine, EngineError> {
        refuse_unbuilt(&bundle)?;
        let pinned = store.manifest(run_id)?;
        let pinned_bundle = bundle_manifest_from_run(&pinned)?;
        if let Some(dispatch) = brokkr_core::dispatch::dispatch_from_run(&pinned)? {
            dispatch.verify(time::OffsetDateTime::now_utc(), &bundle.manifest_digest())?;
            verify_dispatch_bundle_bounds(&dispatch, &bundle)?;
        }
        // The boundary needs no second fence here: the manifest pins it
        // per hands site, so a bundle compiled under another word is a
        // manifest mismatch whose diff names `boundary` (design DD3).
        if pinned_bundle != bundle.manifest {
            let detail = manifest_diff(&pinned_bundle, &bundle.manifest);
            return Err(EngineError::ManifestMismatch {
                run_id: run_id.to_string(),
                detail,
            });
        }
        // An equal manifest is the pinned identity, not the pinned bytes:
        // each charter it names is read again through its owner, whether the
        // run pinned a world or none (rebuild unit 19), and must be the
        // binding the run STARTED over: the bundle here may be compiled
        // again, and a recompile binds an equal-byte retarget afresh without
        // moving the manifest (review return F1). A pin that does not answer
        // for itself is tampering, and refuses before any charter is read.
        let world = crate::realms::World::from_manifest(&pinned)?;
        let mut replay = Replay::default();
        let feature = replay.caught_up(&mut store, run_id)?.feature;
        let started = replay
            .events
            .first()
            .filter(|event| event.event_type == EventType::RunStarted)
            .and_then(|event| event.payload.get("charters"));
        charters_as_started(&bundle, started).map_err(charter_moved)?;
        let entry = replay
            .events
            .first()
            .map(|event| brokkr_core::entry::RunEntry::of_started(&event.payload))
            .transpose()?
            .flatten();
        if let Some(entry) = &entry {
            crate::bundle::entry::admit(&bundle, &entry.phase)?;
        }
        // Resume takes no map — and needs none: the world it believed in
        // is pinned in the manifest just read, content and all, so it is
        // rehydrated from evidence rather than off a disk that may have
        // moved on.
        let mut engine = shell(
            store,
            bundle,
            run_id.to_string(),
            feature.unwrap_or("unknown".to_string()),
            operated_repo(repo),
        );
        engine.entry = entry;
        engine.world = world;
        engine.replay = replay;
        Ok(engine)
    }

    pub(crate) fn read_member_retry(
        store: &mut Store,
        run: &str,
    ) -> Result<Result<RetryRead, MemberRetryOutcome>, EngineError> {
        let events = store.load(run)?;
        let deciding = decided(store, run, &events)?;
        if let Some(refusal) = acceptance_refusal(&deciding.0, OperatorCommand::Retry) {
            return Ok(Err(MemberRetryOutcome::FoldRefused(refusal)));
        }
        let attempts = match read_visit(&events, &deciding.0, u64::MAX) {
            Ok(attempts) => attempts,
            Err(refusal) => return Ok(Err(MemberRetryOutcome::CarryRefused(refusal))),
        };
        Ok(Ok(RetryRead {
            deciding,
            events,
            attempts,
        }))
    }

    pub(crate) fn retry_members(
        &mut self,
        read: RetryRead,
        names: &[String],
        operator: &str,
        reason: &str,
    ) -> Result<MemberRetryOutcome, EngineError> {
        self.retry_members_racing(read, names, operator, reason, |_| {})
    }

    pub(crate) fn retry_members_racing(
        &mut self,
        read: RetryRead,
        names: &[String],
        operator: &str,
        reason: &str,
        mut before_command: impl FnMut(&mut Store),
    ) -> Result<MemberRetryOutcome, EngineError> {
        let facts = self.carry_facts(&read.events, &read.deciding.0, read.attempts);
        let names: Vec<_> = names.iter().cloned().map(Tag).collect();
        let plan = match carry_plan(&facts, &names) {
            Ok(plan) => plan,
            Err(refusal) => return Ok(MemberRetryOutcome::CarryRefused(refusal)),
        };
        let args = serde_json::to_value(RetryArgs {
            members: Some(plan.fresh()),
        })
        .expect("retry args serialize");
        let request = Commanded {
            command: OperatorCommand::Retry,
            operator,
            reason,
            args,
        };
        before_command(&mut self.store);
        // One line, as `disposed`'s own refusal callsites: the exact-coverage
        // gate reads a multi-line call's `?` edge as its own region.
        let (store, run_id) = (&mut self.store, &self.run_id);
        let disposition = disposed(store, run_id, request, read.deciding, |_| {})?;
        Ok(MemberRetryOutcome::Disposed(disposition.outcome()))
    }
}

#[cfg(test)]
mod tests;
