//! The refusal a rate-limited attempt's terminal event records (decision
//! 0103 slice 4c1): the wait it writes, the record the failed event keeps,
//! and the park a refusal that cannot wait ends with. The resume reads the
//! same record through these functions, so its text cannot drift from the
//! terminal's.

use brokkr_core::{provider_refusal, EventEnvelope, ProviderRefusal, ProviderWait, RefusalCause};
use brokkr_protocol::{PreSessionRefusal, RefusalClass};
use serde_json::{json, Value};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use super::sites;
use super::{
    episode, note_error, past_maximum, terminal, CooldownError, Engine, EngineError, Terminal,
};
use crate::admission::Bounds;

/// The park reason a recorded refusal gives (decision 0103 slice 4c1, this
/// fix round): one function the terminal path, the cap and the resume all
/// call, so the record's text cannot drift apart from the live one. Every
/// fact it names is one the failed event carries, so a resume that never saw
/// the first park says exactly what that park said.
pub(super) fn refusal_text(refused: &ProviderRefusal, episode: Option<u64>) -> String {
    let body = refusal_body(refused, episode);
    match refused.site.as_deref() {
        Some(site) => format!("member {site}: {body}"),
        None => body,
    }
}

/// [`refusal_text`]'s facts, without the member the record may name.
fn refusal_body(refused: &ProviderRefusal, episode: Option<u64>) -> String {
    let route = refused.route.as_deref().unwrap_or("an unresolved route");
    let retry = refused
        .retry_after_seconds
        .map_or_else(String::new, |retry| {
            format!(" (provider retry-after {retry}s)")
        });
    match refused.cause {
        RefusalCause::RouteUnresolved => format!(
            "provider {} refused before the first turn with a rate limit, but its route could \
             not be resolved; no cool-down is recorded because a provider-wide hold would hold \
             unrelated routes{retry}: {}",
            refused.provider, refused.words
        ),
        RefusalCause::HostUnreadable => format!(
            "refusing to retry {}/{route} without a cool-down: the host configuration could not \
             be read or is not a valid host file{retry}: {}",
            refused.provider, refused.words
        ),
        RefusalCause::EpisodeUnreadable => format!(
            "refusing to retry {}/{route}: an earlier wait does not read{retry}: {}",
            refused.provider, refused.words
        ),
        RefusalCause::PastMaximum => {
            let (seconds, max) = recorded_length(refused);
            match episode {
                Some(episode) => format!(
                    "cool-down for {}/{route} would wait {seconds}{retry} and the episode \
                     {episode}s is past the declared {max} maximum: {}",
                    refused.provider, refused.words
                ),
                None => format!(
                    "cool-down for {}/{route} would wait {seconds}{retry}, past the declared \
                     {max} maximum: {}",
                    refused.provider, refused.words
                ),
            }
        }
        RefusalCause::NoEnd => {
            let (seconds, max) = recorded_length(refused);
            let given = refused.retry_after_seconds.map_or_else(
                || seconds.clone(),
                |retry| format!("retry-after {retry}s ({seconds})"),
            );
            // The numbers as they are: the end the wire cannot spell, the
            // length computed, and the maximum declared. Only a length the
            // record itself shows past that maximum says it was passed.
            let maximum = match recorded_past(refused) {
                true => format!(", past the declared {max} maximum"),
                false => format!(", and the declared {max} maximum"),
            };
            format!(
                "cool-down for {}/{route}: {given} has no representable end; the computed wait \
                 is {seconds}{maximum}: {}",
                refused.provider, refused.words
            )
        }
    }
}

/// The record's computed length and declared maximum, the one extraction
/// both the no-end comparison and the text read. Only a cause that reached
/// the maximum carries them, and the reader requires both on that cause, so
/// the expects are the record's invariant, as `waiter`'s RFC 3339 expect is
/// the wire's.
fn recorded_numbers(refused: &ProviderRefusal) -> (u64, u64) {
    let seconds = refused
        .seconds
        .expect("a computed refusal records its seconds");
    let max = refused
        .max_seconds
        .expect("a computed refusal records its maximum");
    (seconds, max)
}

/// Whether the record's own computed length is past its declared maximum,
/// as the numbers it carries show it: the no-end arm's one comparison.
fn recorded_past(refused: &ProviderRefusal) -> bool {
    let (seconds, max) = recorded_numbers(refused);
    seconds > max
}

/// A computed length and the declared maximum as the refusal's text spells
/// them, both read by [`recorded_numbers`].
fn recorded_length(refused: &ProviderRefusal) -> (String, String) {
    let (seconds, max) = recorded_numbers(refused);
    (format!("{seconds}s"), format!("{max}s"))
}

/// The refused record a typed rate-limit refusal parks on: the facts the
/// failed event keeps, whatever the park that follows says.
pub(super) fn refusal_record(
    cause: RefusalCause,
    site: Option<&str>,
    refusal: &PreSessionRefusal,
    route: Option<&str>,
    seconds: Option<u64>,
    max: Option<u64>,
) -> ProviderRefusal {
    ProviderRefusal {
        cause,
        site: site.map(str::to_string),
        provider: refusal.provider().to_string(),
        route: route.map(str::to_string),
        words: refusal.words().to_string(),
        retry_after_seconds: refusal.retry_after_seconds(),
        seconds,
        max_seconds: max,
    }
}

/// Write the refusal on the failed payload the terminal event will journal.
pub(super) fn record_refusal(payload: &mut Value, refused: &ProviderRefusal) {
    payload["refused"] = serde_json::to_value(refused).expect("a refusal record serializes");
}

/// The end the journal can spell: RFC 3339 carries a four-digit year, and a
/// `time` build with large dates holds years past it. An end the wire cannot
/// carry is no wait at all, the same as one `checked_add` could not form
/// (step 4's no-`until` case), so the reader's one format decides it.
pub(super) fn representable_end(until: OffsetDateTime) -> Option<OffsetDateTime> {
    until.format(&Rfc3339).ok().map(|_| until)
}

/// A sealed wait, as the terminal event's JSON spells it.
pub(super) fn waiter(wait: &ProviderWait, until: OffsetDateTime) -> Value {
    let mut value = json!({
        "provider": wait.provider,
        "route": wait.route,
        "words": wait.words,
        "seconds": wait.seconds,
        "until": until.format(&Rfc3339).expect("an RFC 3339 time formats as RFC 3339"),
        "rounds": wait.rounds,
    });
    if let Some(site) = &wait.site {
        value["site"] = json!(site);
    }
    if let Some(seconds) = wait.retry_after_seconds {
        value["retry_after_seconds"] = json!(seconds);
    }
    value
}

impl Engine {
    /// The last terminal event's one wait, or why a present record does not
    /// read. A single seat carries exactly one; more refuses by name in
    /// [`Engine::recorded_waits`].
    #[cfg(test)]
    pub(super) fn recorded_wait(
        &self,
        events: &[EventEnvelope],
        effect_id: &str,
    ) -> Result<Option<ProviderWait>, CooldownError> {
        Ok(self
            .recorded_waits(events, effect_id)?
            .and_then(|mut waits| waits.pop()))
    }

    /// The refusal the effect's last terminal event carries, or why a
    /// present record does not read. Absent is the old journal's no-refusal;
    /// a present one is never read as absent (decision 0103 slice 4c1, this
    /// fix round).
    pub(in crate::engine) fn recorded_refusal(
        &self,
        events: &[EventEnvelope],
        effect_id: &str,
    ) -> Result<Option<ProviderRefusal>, CooldownError> {
        let Some(event) = terminal(events, effect_id) else {
            return Ok(None);
        };
        provider_refusal(event).map_err(CooldownError::RefusedUnreadable)
    }

    /// The reason a crash-before-the-park resume gives: the same function
    /// the terminal path used, over the episode the journal still holds.
    pub(in crate::engine) fn refused_park_reason(
        &self,
        events: &[EventEnvelope],
        effect_id: &str,
        refused: &ProviderRefusal,
    ) -> String {
        let episode = episode(events, effect_id).ok().map(|(_, seconds)| seconds);
        refusal_text(refused, episode)
    }

    /// The failed payload, with the wait a typed rate-limit refusal records
    /// or the park that ends the run before any retry. A non-rate-limit
    /// refusal, and one with no declared cool-down, keeps today's 0006
    /// path; the absent declaration is named in the error so the cap's own
    /// park carries it (the recommended option (a)).
    pub(in crate::engine) fn rate_limit_terminal(
        &mut self,
        effect_id: &str,
        mut payload: Value,
        refusal: Option<&PreSessionRefusal>,
    ) -> Result<Terminal, EngineError> {
        let Some(refusal) = refusal.filter(|refusal| refusal.class() == RefusalClass::RateLimit)
        else {
            return Ok(Terminal::Plain(payload));
        };
        let events = self.store.load(&self.run_id)?;
        let (provider, route) = (refusal.provider(), refusal.route());
        let Some(route) = route else {
            let refused = refusal_record(
                RefusalCause::RouteUnresolved,
                None,
                refusal,
                None,
                None,
                None,
            );
            record_refusal(&mut payload, &refused);
            return Ok(self.refusal_terminal(&events, effect_id, payload, refused, None));
        };
        let bounds = match self.cooldown_bounds(provider, route) {
            Ok(Some(bounds)) => bounds,
            Ok(None) => {
                let note = format!(
                    "no cool-down is declared for {provider}/{route} in {}",
                    self.host_spelling()
                );
                note_error(&mut payload, &note);
                return Ok(Terminal::Plain(payload));
            }
            Err(_) => {
                let refused = refusal_record(
                    RefusalCause::HostUnreadable,
                    None,
                    refusal,
                    Some(route),
                    None,
                    None,
                );
                record_refusal(&mut payload, &refused);
                return Ok(self.refusal_terminal(&events, effect_id, payload, refused, None));
            }
        };
        Ok(self.recorded_terminal(&events, effect_id, payload, route, refusal, bounds))
    }

    /// The wait the record supports and the park it may end with, once the
    /// refusal's route and bounds are known. Kept apart from
    /// [`Engine::rate_limit_terminal`] so the reading of the journal and
    /// the park it decides each stay one small function (ruling 4).
    fn recorded_terminal(
        &self,
        events: &[EventEnvelope],
        effect_id: &str,
        mut payload: Value,
        route: &str,
        refusal: &PreSessionRefusal,
        bounds: Bounds,
    ) -> Terminal {
        let provider = refusal.provider();
        let (prior_rounds, prior_seconds) = match episode(events, effect_id) {
            Ok(episode) => episode,
            Err(_) => {
                let refused = refusal_record(
                    RefusalCause::EpisodeUnreadable,
                    None,
                    refusal,
                    Some(route),
                    None,
                    None,
                );
                record_refusal(&mut payload, &refused);
                return self.refusal_terminal(events, effect_id, payload, refused, None);
            }
        };
        let rounds = prior_rounds.saturating_add(1);
        let retry_after = refusal.retry_after_seconds();
        let seconds = sites::wait_seconds(retry_after, bounds.default_seconds.get(), rounds);
        let max = u64::from(bounds.max_seconds.get());
        let total = prior_seconds.saturating_add(seconds);
        // The maximum is judged from the computed length BEFORE any `until`
        // is formed, so no huge retry-after can overflow here (step 4).
        let past = past_maximum(seconds, total, max);
        let until = sites::wait_until(self.now(), seconds);
        let wait = until.map(|until| ProviderWait {
            site: None,
            provider: provider.to_string(),
            route: route.to_string(),
            words: refusal.words().to_string(),
            retry_after_seconds: retry_after,
            seconds,
            until,
            rounds,
        });
        if let Some(wait) = &wait {
            payload["waits"] = json!([waiter(wait, wait.until)]);
        }
        // The refusal that ends in a park is written on the failed event, not
        // only on the `run/parked` after it: a crash between the two appends
        // must not turn the refusal into today's 0006 retry. A wait with no
        // `until` is exactly such a park, so it writes no `waits` entry.
        if past || wait.is_none() {
            let (cause, episode) = match wait.is_some() {
                true => (RefusalCause::PastMaximum, Some(total)),
                false => (RefusalCause::NoEnd, None),
            };
            let refused =
                refusal_record(cause, None, refusal, Some(route), Some(seconds), Some(max));
            record_refusal(&mut payload, &refused);
            return self.refusal_terminal(events, effect_id, payload, refused, episode);
        }
        Terminal::Plain(payload)
    }

    /// The terminal a refusal that would park writes. At the cap the failed
    /// event alone is journaled, so the cursor's 0006 branch writes the one
    /// park with the record's facts beside it (decision 0103 ruling 7(c));
    /// below the cap the record's own text is the park.
    fn refusal_terminal(
        &self,
        events: &[EventEnvelope],
        effect_id: &str,
        payload: Value,
        refused: ProviderRefusal,
        episode: Option<u64>,
    ) -> Terminal {
        if self.at_attempt_cap(events, effect_id) {
            return Terminal::Plain(payload);
        }
        Terminal::Park {
            payload,
            reason: refusal_text(&refused, episode),
        }
    }
}
