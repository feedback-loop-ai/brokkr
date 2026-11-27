//! Post-turn transcript and root-session publication plus launch-row masking
//! preflight for Antigravity CLI work seats (#1227, decision 0075 level 2).

use std::path::Path;

use brokkr_core::seat::AGY_TRANSCRIPT_KIND;
use serde_json::{Map, Value};

use super::super::fold::Conversation;
use super::super::keep;
use super::super::rejoin::{self, RejoinCaps, RejoinNotice, TurnKeep};
use super::{rejoin_plan, replace_rejoin};
use crate::adapters::{Emitted, Invocation, LaunchHold, LaunchTerminal, RootSession};
use crate::secret::BoundSecret;

/// Publish the kept transcript and, for a declaring work turn (or a declined
/// offer), the kept rejoin state and `harness-started` checkpoint before the
/// engine HOME is removed and while any conversation lock is still held (#1227).
pub(in crate::adapters::agy) fn publish_turn(
    turn: TurnKeep<'_>,
    hold: &mut LaunchHold,
    outcome: &mut Result<Invocation, super::super::Ended>,
    emit: &mut impl FnMut(&Emitted),
) {
    let terminal = hold.terminal();
    // Unsettled rejoin (mismatch or unconfirmed): keep nothing and leave the
    // offered conversation's kept state untouched.
    if terminal.is_unsettled() {
        return;
    }
    let resumed = terminal == LaunchTerminal::Resumed;
    let mut fallback_meta = Map::new();
    let target_meta = match outcome {
        Ok(invocation) => &mut invocation.session_meta,
        Err(_) => &mut fallback_meta,
    };
    let ctx = keep::KeepContext {
        declared: turn.declared.transcript,
        probe: turn.probe,
        refused_to_start: turn.refused_to_start,
        conversation: turn.conversation,
        engine_home: turn.engine_home,
        operator_home: turn.operator_home,
        bindings: turn.bindings,
    };
    let transcript_outcome = if resumed && !turn.refused_to_start {
        keep::publish_transcript_mode(ctx, true, target_meta, emit)
    } else {
        keep::publish_transcript(ctx, target_meta, emit)
    };
    let kept_transcript = match &transcript_outcome {
        keep::TranscriptOutcome::Kept(kept) => Some(kept),
        keep::TranscriptOutcome::NotKept(notice) => {
            if let Some(notice) = notice {
                keep::report_notice(outcome, notice);
            }
            None
        }
    };
    if let Some(notice) = publish_root_session(&turn, hold, resumed, kept_transcript, emit) {
        keep::report_notice(outcome, &notice);
    }
}

fn keep_rejoin_state(turn: &TurnKeep<'_>, id: &str, resumed: bool) -> (bool, Option<RejoinNotice>) {
    let keep_result = match resumed {
        true => replace_rejoin(
            id,
            turn.engine_home,
            turn.operator_home,
            turn.bindings,
            RejoinCaps::DEFAULT,
        ),
        false => rejoin::keep_rejoin(
            id,
            turn.engine_home,
            turn.operator_home,
            turn.bindings,
            true,
            RejoinCaps::DEFAULT,
        ),
    };
    match keep_result {
        Ok(()) => (true, None),
        Err(notice) => (false, Some(notice)),
    }
}

fn publish_root_session(
    turn: &TurnKeep<'_>,
    hold: &mut LaunchHold,
    resumed: bool,
    kept_transcript: Option<&keep::Kept>,
    emit: &mut impl FnMut(&Emitted),
) -> Option<RejoinNotice> {
    if turn.probe || turn.refused_to_start {
        return None;
    }
    let eligible_root = turn.declared.root && (turn.pin_confirmed || resumed);
    // Confirmed rejoins always carry a named UUID, observed version, and
    // declared root (`r4_masked_rejoin_declines_cold_before_lock`).
    let (Conversation::Named(id), Some(version), true) =
        (turn.conversation, hold.harness_version(), eligible_root)
    else {
        if hold.has_refusal() {
            hold.publish_settled(None, None, emit);
        }
        return None;
    };
    let address = turn
        .pin_confirmed
        .then_some(kept_transcript)
        .flatten()
        .map(keep::Kept::transcript_value);
    // Confirmed rejoins were already declined pre-lock if bindings would
    // rewrite either resumed form (`r4_masked_rejoin_declines_cold_before_lock`).
    if !resumed && row_would_mask_launch(hold, id, version, address.as_ref(), turn.bindings) {
        if hold.has_refusal() {
            hold.publish_settled(None, None, emit);
        }
        return Some(RejoinNotice::ReferenceMasked);
    }
    let (persistent, notice) = match (turn.pin_confirmed, address.is_some()) {
        (false, _) => (false, None),
        (true, false) => (false, Some(RejoinNotice::AddressUnavailable)),
        (true, true) => keep_rejoin_state(turn, id, resumed),
    };
    let root = RootSession::new(
        AGY_TRANSCRIPT_KIND,
        id.clone(),
        version.to_string(),
        persistent,
    );
    hold.publish_settled(Some(root), persistent.then_some(address).flatten(), emit);
    notice
}

pub(super) fn resumed_rows_would_mask(
    offered: &str,
    version: &str,
    parent: &Path,
    bindings: &[BoundSecret],
) -> bool {
    if bindings.is_empty() {
        return false;
    }
    let hold = LaunchHold::new(
        "agy",
        rejoin_plan(Vec::new(), offered.to_string(), version.to_string()),
    );
    let address = crate::transcript::Transcript::agy(parent.to_path_buf(), offered).value();
    let persistent = RootSession::new(
        AGY_TRANSCRIPT_KIND,
        offered.to_string(),
        version.to_string(),
        true,
    );
    let non_persistent = RootSession::new(
        AGY_TRANSCRIPT_KIND,
        offered.to_string(),
        version.to_string(),
        false,
    );
    let forms = [
        hold.build_launch_row(true, Some(&non_persistent), None),
        hold.build_launch_row(true, Some(&persistent), Some(&address)),
    ];
    forms.iter().any(|row| keep::row_would_mask(row, bindings))
}

fn row_would_mask_launch(
    hold: &LaunchHold,
    id: &str,
    version: &str,
    address: Option<&Value>,
    bindings: &[BoundSecret],
) -> bool {
    let root = RootSession::new(
        AGY_TRANSCRIPT_KIND,
        id.to_string(),
        version.to_string(),
        address.is_some(),
    );
    let emitted = hold.build_launch_row(false, Some(&root), address);
    keep::row_would_mask(&emitted, bindings)
}
