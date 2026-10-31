//! `Bridge::sync_once`'s one pass, split at the seams it already has:
//! the sealed dispatch, the registration, event submission, and the command
//! round with its receipts. The split is mechanical: every check keeps its
//! order, every transport call keeps its place, and every error keeps its
//! variant.

use brokkr_core::canonical;
use brokkr_core::dispatch::{bundle_manifest_from_run, dispatch_from_run, DispatchEnvelopeV2};
use brokkr_runtime::{apply_fenced_operator_command, FencedCommandOutcome};
use brokkr_store::Store;
use serde_json::Value;
use time::OffsetDateTime;

use super::{
    audience_matches, normalize_event, Bridge, BridgeError, CommandReceipt, EventEnvelope,
    ProducerCommand, ProducerTransport, RegistrationState, Submission, SyncReport,
};

impl<T: ProducerTransport> Bridge<T> {
    /// Read the run's manifest, verify the dispatch it sealed, and refuse a
    /// transport bound to any other audience before it is asked to register.
    pub(super) fn sealed_dispatch(
        &self,
        store: &mut Store,
        run_id: &str,
        now: OffsetDateTime,
    ) -> Result<(Value, DispatchEnvelopeV2), BridgeError> {
        let manifest = store.manifest(run_id)?;
        let dispatch = dispatch_from_run(&manifest)?.ok_or(BridgeError::UnboundRun)?;
        let bundle_manifest = bundle_manifest_from_run(&manifest)?;
        let bundle_sha = canonical::sha256_hex(&bundle_manifest);
        dispatch.verify(now, &bundle_sha)?;
        if dispatch.forge_run_id != run_id {
            return Err(BridgeError::RegistrationMismatch);
        }
        audience_matches(self.transport.origin(), &dispatch)?;
        Ok((manifest, dispatch))
    }

    /// Register with the transport and hold the returned state to the
    /// dispatch and to the journal it claims to have seen.
    pub(super) fn bind_registration(
        &mut self,
        store: &mut Store,
        run_id: &str,
        dispatch: &DispatchEnvelopeV2,
        manifest: &Value,
    ) -> Result<(RegistrationState, Vec<EventEnvelope>), BridgeError> {
        let registration = self.transport.register(dispatch, manifest)?;
        if registration.registration_id != dispatch.producer.registration_id {
            return Err(BridgeError::RegistrationMismatch);
        }
        let events = store.load(run_id)?;
        let recorded = registration.last_forge_sequence;
        if recorded > events.len() as u64
            || (recorded > 0
                && events[(recorded - 1) as usize].event_hash != registration.last_event_hash)
        {
            return Err(BridgeError::RegistrationMismatch);
        }
        Ok((registration, events))
    }

    /// Submit every event past `after`, in journal order, counting what the
    /// transport reports as replayed and recording how far submission got.
    pub(super) fn submit_pending(
        &mut self,
        dispatch: &DispatchEnvelopeV2,
        events: &[EventEnvelope],
        after: u64,
        submission: &mut Submission,
    ) -> Result<(), BridgeError> {
        for event in events.iter().filter(|event| event.seq > after) {
            self.await_event_slot(dispatch.bounds.max_events_per_ten_seconds);
            if self.transport.submit(&normalize_event(dispatch, event)?)? {
                submission.replayed += 1;
            } else {
                submission.submitted += 1;
            }
            submission.through = event.seq;
        }
        Ok(())
    }

    /// Apply each command in cursor order, submit the evidence it produced,
    /// and acknowledge it with the receipt for its disposition.
    pub(super) fn apply_commands(
        &mut self,
        store: &mut Store,
        run_id: &str,
        dispatch: &DispatchEnvelopeV2,
        command_after: u64,
        submission: &mut Submission,
    ) -> Result<(usize, u64), BridgeError> {
        let commands = self
            .transport
            .commands(&dispatch.producer.registration_id, command_after)?;
        let mut last_command_cursor = command_after;
        for command in &commands {
            if command.cursor <= last_command_cursor {
                return Err(BridgeError::RegistrationMismatch);
            }
            last_command_cursor = command.cursor;
            let result = apply_fenced_operator_command(store, run_id, &command.fenced())?;
            // A one-shot bridge invocation is still a complete round trip: send
            // the durable command/disposition evidence before acknowledging it.
            let command_start = submission.through;
            let events = store.load(run_id)?;
            self.submit_pending(dispatch, &events, command_start, submission)?;
            let receipt = receipt(command, result);
            self.transport
                .acknowledge_command(&dispatch.producer.registration_id, &receipt)?;
        }
        Ok((commands.len(), last_command_cursor))
    }
}

/// A terminal registration is a clean idempotent boundary when its journal is
/// the prefix it recorded; nothing is submitted and no command is read.
pub(super) fn terminal_report(
    registration: &RegistrationState,
    events: &[EventEnvelope],
    command_after: u64,
) -> Result<Option<SyncReport>, BridgeError> {
    if registration.status != "terminal" {
        return Ok(None);
    }
    if registration.last_forge_sequence != events.len() as u64 {
        return Err(BridgeError::RegistrationMismatch);
    }
    Ok(Some(SyncReport {
        registered: true,
        submitted: 0,
        replayed: 0,
        commands: 0,
        last_command_cursor: command_after,
        last_forge_sequence: registration.last_forge_sequence,
    }))
}

/// The receipt for one fenced command's disposition, as the transport reads
/// it back.
fn receipt(command: &ProducerCommand, result: FencedCommandOutcome) -> CommandReceipt {
    match result {
        FencedCommandOutcome::Accepted {
            head_seq,
            head_hash,
        } => CommandReceipt {
            command_id: command.id.clone(),
            outcome: "accepted".into(),
            reason: None,
            forge_sequence: head_seq,
            event_hash: head_hash,
        },
        FencedCommandOutcome::Rejected {
            reason,
            head_seq,
            head_hash,
        } => CommandReceipt {
            command_id: command.id.clone(),
            outcome: "rejected".into(),
            reason: Some(reason),
            forge_sequence: head_seq,
            event_hash: head_hash,
        },
    }
}
