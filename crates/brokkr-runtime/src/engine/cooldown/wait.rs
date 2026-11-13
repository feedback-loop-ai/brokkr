//! The engine's wait (decision 0103 slice 4c1, the aggregate and the probe's
//! extension in slice 4d2b): the maximum judged before anything sleeps, the
//! aggregate taken as the wait's end in bounded slices that honour an
//! operator `stop`, the probe of each cooled route, and a rate-limited
//! probe's next round waited and probed again before the retry.

use brokkr_core::fold::{fold, Cursor, Run, RunState};
use brokkr_core::{EventEnvelope, ProbeAnswer, ProviderWait};
use brokkr_store::{CooldownAggregate, CooldownRound, RoundAppend};
use time::{Duration as TimeDuration, OffsetDateTime};

use super::probe::{NextRound, Probe};
use super::{
    episode_by_site, maximum_reason, past_maximum, sites, CooldownError, Engine, EngineError,
    Waited, SLICE,
};

impl Engine {
    /// Wait the recorded remainder in bounded slices, honouring an operator
    /// `stop`, then probe the cooled route. The wait's end is the latest of
    /// each recorded wait's own `until` and the store's aggregate for its
    /// provider and route (decision 0103 ruling 2(a)); after the probes
    /// answer, the aggregate is re-read before the retry, so a row recorded
    /// while the probe ran holds the route again. The maximum is judged from
    /// the record before anything sleeps. A stop already journaled is seen
    /// before the first slice and again immediately before every probe.
    pub(in crate::engine) fn take_cooldown(
        &mut self,
        events: &[EventEnvelope],
        state: &RunState,
        effect_id: &str,
        seat: &str,
        waits: &[ProviderWait],
    ) -> Result<Waited, EngineError> {
        for wait in waits {
            if let Some(reason) = self.resume_park_reason(events, effect_id, wait) {
                return Ok(Waited::Parked(reason));
            }
        }
        self.hold_the_aggregate(events, state, effect_id, seat, waits)
    }

    /// The wait's rounds: each round waits the record's own end and the
    /// store's aggregate, probes every cooled site, then re-reads the
    /// aggregate before deciding to retry. A rate-limited refusal's next row
    /// extends the wait and the loop; the episode maximum bounds it, the
    /// remainders waited on another run's rows included (decision 0103
    /// ruling 2(a), choice 3).
    fn hold_the_aggregate(
        &mut self,
        events: &[EventEnvelope],
        state: &RunState,
        effect_id: &str,
        seat: &str,
        waits: &[ProviderWait],
    ) -> Result<Waited, EngineError> {
        // The latest own or probe-round end the episode counts. A written
        // round is counted once by `episode`, never again as a remainder.
        let mut round = Round {
            answers: Vec::with_capacity(waits.len()),
            unrecorded: 0,
            counted: waits
                .iter()
                .map(|wait| wait.until)
                .max()
                .expect("a wait to take"),
            waits,
            maxima: Vec::new(),
        };
        loop {
            match self.step(&mut round, events, state, effect_id, seat)? {
                Step::Proceed => return Ok(Waited::Proceed(round.answers)),
                Step::Again => continue,
                Step::Stopped => return Ok(Waited::Stopped),
                Step::Parked(reason) => return Ok(Waited::Parked(reason)),
            }
        }
    }

    /// One round's wait, probes and re-read: the aggregate still in force
    /// (again), every cooled site answered and clear (proceed), a stop, or
    /// the park a step gave. The round's answers, unrecorded seconds and
    /// counted end live in [`Round`] so this stays within the parameter and
    /// complexity ceilings (0071 ruling 4).
    fn step(
        &mut self,
        round: &mut Round<'_>,
        events: &[EventEnvelope],
        state: &RunState,
        effect_id: &str,
        seat: &str,
    ) -> Result<Step, EngineError> {
        match self.wait_round(
            events,
            effect_id,
            round.waits,
            &mut round.unrecorded,
            round.counted,
            &mut round.maxima,
        )? {
            WaitRound::Stopped => return Ok(Step::Stopped),
            WaitRound::Parked(reason) => return Ok(Step::Parked(reason)),
            WaitRound::Waited => {}
        }
        let answers = match self.round_probes(round, events, state, effect_id, seat)? {
            ProbedRound::Stopped => return Ok(Step::Stopped),
            ProbedRound::Parked(reason) => return Ok(Step::Parked(reason)),
            ProbedRound::Extended(until) => {
                round.counted = round.counted.max(until);
                return Ok(Step::Again);
            }
            ProbedRound::Answered(answers) => answers,
        };
        for answer in answers {
            record_answer(&mut round.answers, answer);
        }
        // Re-read AFTER the probes and BEFORE the retry: another run's row,
        // or one recorded while the probe ran, holds the route again, so
        // the wait and the probes repeat.
        match self.aggregate_in_force(round.waits) {
            Ok(false) => Ok(Step::Proceed),
            Ok(true) => Ok(Step::Again),
            Err(error) => Ok(Step::Parked(error.to_string())),
        }
    }

    /// One round of probes and, if a rate-limited refusal extended it, its
    /// written end. Kept apart from the wait loop so neither function grows
    /// past a ceiling (0071 ruling 4).
    fn round_probes(
        &mut self,
        round: &Round<'_>,
        events: &[EventEnvelope],
        state: &RunState,
        effect_id: &str,
        seat: &str,
    ) -> Result<ProbedRound, EngineError> {
        match self.probe_each(events, state, effect_id, seat, round.waits)? {
            ProbeRound::Stopped => Ok(ProbedRound::Stopped),
            ProbeRound::Parked(reason) => Ok(ProbedRound::Parked(reason)),
            ProbeRound::Extended(extension) => {
                match self.hold_extension(
                    round.waits,
                    &round.maxima,
                    &extension,
                    round.unrecorded,
                )? {
                    Extension::Stopped => Ok(ProbedRound::Stopped),
                    Extension::Parked(reason) => Ok(ProbedRound::Parked(reason)),
                    Extension::Written(until) => Ok(ProbedRound::Extended(until)),
                }
            }
            ProbeRound::Answered(round) => Ok(ProbedRound::Answered(round)),
        }
    }

    /// A probe's rate-limit refusal, once the probe has returned: a stop
    /// journaled while the probe ran ends the run before the append, so no
    /// row is written for that probe; otherwise the round is judged and
    /// appended. Lifted out of the round loop so the loop stays small
    /// (0071 ruling 4).
    fn hold_extension(
        &mut self,
        waits: &[ProviderWait],
        maxima: &[Option<u64>],
        round: &NextRound,
        unrecorded: u64,
    ) -> Result<Extension, EngineError> {
        if self.moved_from_the_effect()? {
            return Ok(Extension::Stopped);
        }
        match self.record_extension(waits, maxima, round, unrecorded) {
            Ok(until) => Ok(Extension::Written(until)),
            Err(error) => Ok(Extension::Parked(error.to_string())),
        }
    }

    /// One round's wait: a stop, the park reason the maximum gives, or the
    /// wait taken. The episode's own stored rows seed the counted baseline,
    /// so a written round's remainder is never counted twice; `unrecorded`
    /// is the seconds waited on rows this episode does not own.
    pub(super) fn wait_round(
        &mut self,
        events: &[EventEnvelope],
        effect_id: &str,
        waits: &[ProviderWait],
        unrecorded: &mut u64,
        counted: OffsetDateTime,
        maxima: &mut Vec<Option<u64>>,
    ) -> Result<WaitRound, EngineError> {
        if self.moved_from_the_effect()? {
            return Ok(WaitRound::Stopped);
        }
        let mut end = match self.cool_down_end(waits) {
            Ok(end) => end,
            Err(error) => return Ok(WaitRound::Parked(error.to_string())),
        };
        loop {
            // The maximum is judged before every wait this round takes,
            // including a re-wait to an end another run moved during the
            // slices.
            let budget = match self.extension_park_reason(
                events,
                effect_id,
                waits,
                end,
                counted,
                *unrecorded,
            ) {
                Ok(budget) => budget,
                Err(error) => return Ok(WaitRound::Parked(error.to_string())),
            };
            *unrecorded = budget.waited;
            *maxima = budget.maxima;
            if self.wait_in_slices(end)? {
                return Ok(WaitRound::Stopped);
            }
            // Re-read the aggregate at the wait's own end, before any
            // probe: a row recorded during the slices can hold every
            // cooled site past the end sampled before them.
            end = match self.cool_down_end(waits) {
                Ok(end) => end,
                Err(error) => return Ok(WaitRound::Parked(error.to_string())),
            };
            if end <= self.now() {
                break;
            }
        }
        Ok(WaitRound::Waited)
    }

    /// Does any cooled site still stand in an aggregate at `now`: the
    /// re-read after the probes and before the retry, and a store refusal
    /// parks by its own words.
    fn aggregate_in_force(&self, waits: &[ProviderWait]) -> Result<bool, CooldownError> {
        Ok(!self.cool_down_aggregates(waits)?.is_empty())
    }

    /// One round's probes in declared order: every answer, the first
    /// extension's new end (later members wait for it), a stop, or the park
    /// reason a probe end gives.
    fn probe_each(
        &mut self,
        events: &[EventEnvelope],
        state: &RunState,
        effect_id: &str,
        seat: &str,
        waits: &[ProviderWait],
    ) -> Result<ProbeRound, EngineError> {
        let mut answers = Vec::new();
        for wait in waits {
            if self.moved_from_the_effect()? {
                return Ok(ProbeRound::Stopped);
            }
            match self.probe_wait(events, state, effect_id, seat, wait)? {
                Probe::Answered(answer) => answers.push(answer),
                Probe::Extended(round) => return Ok(ProbeRound::Extended(round)),
                Probe::Parked(reason) => return Ok(ProbeRound::Parked(reason)),
            }
        }
        Ok(ProbeRound::Answered(answers))
    }

    /// The maximum judged over the episode through `end`, the seconds waited
    /// past the episode already counted included: the waited total, or the
    /// reason to park. The episode's own stored rows seed the baseline, so a
    /// round written before this wait is counted once and a resume over it
    /// does not add its remainder again; a remainder waited on another run's
    /// row is not a row of this episode and is carried in `unrecorded`.
    pub(super) fn extension_park_reason(
        &self,
        events: &[EventEnvelope],
        effect_id: &str,
        waits: &[ProviderWait],
        end: OffsetDateTime,
        counted: OffsetDateTime,
        unrecorded: u64,
    ) -> Result<WaitBudget, CooldownError> {
        let episode =
            episode_by_site(&self.store, &self.run_id, events, effect_id).map_err(|source| {
                let wait = &waits[0];
                CooldownError::WaitEpisodeUnreadable {
                    provider: wait.provider.clone(),
                    route: wait.route.clone(),
                    source: Box::new(source),
                }
            })?;
        let counted = counted.max(episode.latest.unwrap_or(counted));
        let waited = unrecorded.saturating_add(beyond_the_record(counted, end, self.now()));
        let mut maxima = Vec::with_capacity(waits.len());
        for wait in waits {
            let max = match self.cooldown_bounds(&wait.provider, &wait.route) {
                Ok(Some(bounds)) => u64::from(bounds.max_seconds.get()),
                Ok(None) => {
                    maxima.push(None);
                    continue;
                }
                Err(source) => {
                    return Err(CooldownError::WaitBoundsUnreadable {
                        provider: wait.provider.clone(),
                        route: wait.route.clone(),
                        source: Box::new(source),
                    })
                }
            };
            maxima.push(Some(max));
            let through = episode.seconds.saturating_add(waited);
            if past_maximum(waited, through, max) {
                let rounds = episode
                    .rounds
                    .get(&wait.site)
                    .copied()
                    .unwrap_or(0)
                    .max(wait.rounds);
                let pending = ProviderWait {
                    site: wait.site.clone(),
                    provider: wait.provider.clone(),
                    route: wait.route.clone(),
                    words: wait.words.clone(),
                    retry_after_seconds: None,
                    seconds: waited,
                    until: self.now(),
                    rounds,
                };
                return Err(CooldownError::Maximum(maximum_reason(
                    &pending, through, max,
                )));
            }
        }
        Ok(WaitBudget { waited, maxima })
    }

    /// Append one probe's next round after judging it against the maximum
    /// with the remainders this wait has already taken: a round past the
    /// maximum writes nothing and parks with the maximum named (choice 3);
    /// a round within it is written at the terminal's own `(run_id,
    /// event_seq)`, and its end is what the caller waits. A rows-bearing
    /// store refusal parks naming the store's words, never silently. The
    /// effect's attempts are untouched.
    pub(super) fn record_extension(
        &mut self,
        waits: &[ProviderWait],
        maxima: &[Option<u64>],
        round: &NextRound,
        waited: u64,
    ) -> Result<OffsetDateTime, CooldownError> {
        let through = round
            .episode
            .saturating_add(waited)
            .saturating_add(round.seconds);
        // Judge EVERY cooled site's maximum before the append, not only the
        // extending route's: a round that passes any of them writes no row
        // (finding 4, 0103 choice 3). The maxima are the ones the round's
        // wait read before the probes, so the host file is read once a
        // round and the append cannot see a different declaration.
        for (wait, max) in waits.iter().zip(maxima) {
            let Some(max) = max else { continue };
            if past_maximum(round.seconds, through, *max) {
                let pending = ProviderWait {
                    site: wait.site.clone(),
                    provider: wait.provider.clone(),
                    route: wait.route.clone(),
                    words: round.words.clone(),
                    retry_after_seconds: round.retry_after_seconds,
                    seconds: round.seconds,
                    until: self.now(),
                    rounds: round.round,
                };
                return Err(CooldownError::Maximum(maximum_reason(
                    &pending, through, *max,
                )));
            }
        }
        // A round within the maximum has a length at most the declared
        // maximum, a `u32`, so its end is always representable:
        // `a_round_within_the_maximum_has_a_representable_end` holds that
        // bound, which is why this arm has no no-end park to cover.
        let until = sites::wait_until(self.now(), round.seconds)
            .expect("a round within the maximum has a representable end");
        let appended = self.store.append_cooldown_round(CooldownRound {
            run_id: &self.run_id,
            event_seq: round.event_seq,
            site: round.site.as_deref().unwrap_or(""),
            provider: &round.provider,
            route: &round.route,
            until_at: until,
            reason: &round.words,
            retry_after_seconds: round.retry_after_seconds,
            rounds: round.rounds,
            seconds: round.seconds,
        });
        match appended {
            Ok(RoundAppend::Written) => Ok(until),
            Ok(RoundAppend::NoTable) => Err(CooldownError::RoundNoTable {
                provider: round.provider.clone(),
                route: round.route.clone(),
                words: round.words.clone(),
            }),
            Err(source) => Err(CooldownError::RoundUnwritable {
                provider: round.provider.clone(),
                route: round.route.clone(),
                source: Box::new(source),
            }),
        }
    }

    /// The wait's end: the latest of every recorded wait's own `until` and
    /// the store's aggregate for its provider and route. A store refusal
    /// parks by its own words, never as an absent aggregate (decision 0103
    /// ruling 2(a)); a version 1, 2 or 3 journal answers none, so the
    /// event's own `until` stands.
    fn cool_down_end(&self, waits: &[ProviderWait]) -> Result<OffsetDateTime, CooldownError> {
        let mut end = waits
            .iter()
            .map(|wait| wait.until)
            .max()
            .expect("a wait to take");
        for aggregate in self.cool_down_aggregates(waits)? {
            end = end.max(aggregate.until);
        }
        Ok(end)
    }

    /// Every unexpired aggregate the cooled sites stand at, at `now`, or
    /// the park reason a store refusal gives.
    fn cool_down_aggregates(
        &self,
        waits: &[ProviderWait],
    ) -> Result<Vec<CooldownAggregate>, CooldownError> {
        let mut aggregates = Vec::new();
        for wait in waits {
            match self.store.cooldown(&wait.provider, &wait.route, self.now()) {
                Ok(Some(aggregate)) => aggregates.push(aggregate),
                Ok(None) => {}
                Err(source) => {
                    return Err(CooldownError::AggregateUnreadable {
                        provider: wait.provider.clone(),
                        route: wait.route.clone(),
                        source: Box::new(source),
                    })
                }
            }
        }
        Ok(aggregates)
    }

    /// The bounds and episode the resume reads before it sleeps: the reason
    /// to park, or `None` when the wait may proceed. The maximum is judged
    /// from the record before anything sleeps; a bound or episode that does
    /// not read parks by name, never as no declaration (decision 0103
    /// ruling 3).
    fn resume_park_reason(
        &self,
        events: &[EventEnvelope],
        effect_id: &str,
        wait: &ProviderWait,
    ) -> Option<String> {
        match self.cooldown_bounds(&wait.provider, &wait.route) {
            Ok(Some(bounds)) => {
                let max = u64::from(bounds.max_seconds.get());
                let episode = match episode_by_site(&self.store, &self.run_id, events, effect_id) {
                    Ok(episode) => episode,
                    Err(error) => {
                        return Some(format!(
                            "a resume waits for {}/{} without reading its episode: {error}",
                            wait.provider, wait.route
                        ));
                    }
                };
                // The episode's own per-site round and latest length are
                // what a park after a written probe round must name; the
                // terminal wait's first round is stale by then (finding 6).
                let rounds = episode
                    .rounds
                    .get(&wait.site)
                    .copied()
                    .unwrap_or(wait.rounds);
                let seconds = episode
                    .lengths
                    .get(&wait.site)
                    .copied()
                    .unwrap_or(wait.seconds);
                let pending = ProviderWait {
                    site: wait.site.clone(),
                    provider: wait.provider.clone(),
                    route: wait.route.clone(),
                    words: wait.words.clone(),
                    retry_after_seconds: wait.retry_after_seconds,
                    seconds,
                    until: wait.until,
                    rounds,
                };
                past_maximum(wait.seconds, episode.seconds, max)
                    .then(|| maximum_reason(&pending, episode.seconds, max))
            }
            Ok(None) => None,
            Err(reason) => Some(format!(
                "a resume waits for {}/{} without reading its bounds: {reason}",
                wait.provider, wait.route
            )),
        }
    }

    /// Wait the recorded remainder in bounded slices, honouring an operator
    /// `stop` through the journal's high-water mark. `true` is a stop:
    /// nothing may probe or retry after it.
    fn wait_in_slices(&mut self, until: OffsetDateTime) -> Result<bool, EngineError> {
        // Sample BEFORE the first stop check: an event journaled between
        // the caller's fold and this sample is then still a change, and the
        // check below sees it now rather than the sample swallowing it.
        let mut seen = self.store.high_water()?;
        if self.moved_from_the_effect()? {
            return Ok(true);
        }
        loop {
            let remaining = until - self.now();
            if remaining <= TimeDuration::ZERO {
                break;
            }
            (self.cooldown.clock.sleep)(SLICE);
            let high = self.store.high_water()?;
            if high != seen {
                seen = high;
                if self.moved_from_the_effect()? {
                    return Ok(true);
                }
            }
        }
        self.moved_from_the_effect()
    }

    /// Has the run left this effect's executable position: an operator
    /// `stop` during the wait lands as `Cursor::Stop`, and nothing may
    /// probe or retry after it.
    fn moved_from_the_effect(&self) -> Result<bool, EngineError> {
        let events = self.store.load(&self.run_id)?;
        let state = fold(&events)?;
        Ok(!matches!(
            state.run,
            Run::Running(Cursor::ExecuteEffect { .. })
        ))
    }
}

/// What one round's wait left.
pub(super) enum WaitRound {
    Waited,
    Stopped,
    Parked(String),
}

/// What a probe's rate-limit refusal left: the written round's end, a stop
/// that landed while the probe ran, or the park the judge gave.
enum Extension {
    Written(OffsetDateTime),
    Stopped,
    Parked(String),
}

/// What one round of probes left the wait loop: answers to keep, an
/// extension's written end, a stop, or a park.
enum ProbedRound {
    Answered(Vec<ProbeAnswer>),
    Extended(OffsetDateTime),
    Stopped,
    Parked(String),
}

/// The state one effect's wait carries between rounds: the answers taken so
/// far, the seconds waited on rows this episode does not own, the latest end
/// the episode counts, and the cooled sites.
struct Round<'a> {
    answers: Vec<ProbeAnswer>,
    unrecorded: u64,
    counted: OffsetDateTime,
    waits: &'a [ProviderWait],
    maxima: Vec<Option<u64>>,
}

/// One round's waited seconds and each cooled site's declared maximum, as
/// the round's one bounds read left them.
#[derive(Debug)]
pub(super) struct WaitBudget {
    waited: u64,
    maxima: Vec<Option<u64>>,
}

/// What one step left the round loop.
enum Step {
    Proceed,
    Again,
    Stopped,
    Parked(String),
}

/// What one round of probes left.
enum ProbeRound {
    Answered(Vec<ProbeAnswer>),
    /// A rate-limited refusal's next round, before its row is written.
    Extended(NextRound),
    Stopped,
    Parked(String),
}

/// The seconds from `end` that lie past what the episode already counts:
/// `counted` is the latest own or probe-round end it sums, and an instant
/// already reached is no wait at all.
fn beyond_the_record(counted: OffsetDateTime, end: OffsetDateTime, now: OffsetDateTime) -> u64 {
    (end - counted.max(now)).whole_seconds().max(0) as u64
}

/// Keep one answer per cooled site: a later round's answer replaces the
/// earlier one, so the retry's `effect/started` records the final answers
/// (decision 0103 ruling 4(a)).
fn record_answer(answers: &mut Vec<ProbeAnswer>, answer: ProbeAnswer) {
    match answers.iter_mut().find(|seen| seen.site == answer.site) {
        Some(slot) => *slot = answer,
        None => answers.push(answer),
    }
}
