//! The engine's wait (decision 0103 slice 4c1, the aggregate and the probe's
//! extension in slice 4d2b): the maximum judged before anything sleeps, the
//! aggregate taken as the wait's end in bounded slices that honour an
//! operator `stop`, the probe of each cooled route, and a rate-limited
//! probe's next round waited and probed again before the retry.

use brokkr_core::fold::{fold, Cursor, Run, RunState};
use brokkr_core::park::{Park, ParkCause, ProviderLimitRefusal};
use brokkr_core::{EventEnvelope, Held, ProbeAnswer, ProviderWait};
use brokkr_store::{CooldownAggregate, CooldownRound, RoundAppend};
use time::{Duration as TimeDuration, OffsetDateTime};

use super::held::{bound, Bound};
use super::probe::{NextRound, Probe};
use super::{
    episode_by_site, maximum_reason, park_of, past_maximum, sites, CooldownError, Engine,
    EngineError, HeldDrive, MaximumFacts, Waited, SLICE,
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
                // A probe's held round is written: the engine does not sleep
                // it, and this drive ends with the run `running` (decision
                // 0106 ruling 2).
                Step::HeldRound => {
                    self.cooldown.held = HeldDrive::Ended;
                    return Ok(Waited::Held);
                }
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
            ProbedRound::HeldRound => return Ok(Step::HeldRound),
            ProbedRound::Again => return Ok(Step::Again),
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
            Err(error) => Ok(Step::Parked(park_of(&error))),
        }
    }

    /// One round of probes and, if a rate-limited refusal extended it, its
    /// written end. Kept apart from the wait loop so neither function grows
    /// past a ceiling (0071 ruling 4). A foreign row recorded while an
    /// earlier member was probed holds the whole round again without being
    /// counted as this episode's, so its remainder is carried in
    /// `unrecorded` by the next wait.
    fn round_probes(
        &mut self,
        round: &mut Round<'_>,
        events: &[EventEnvelope],
        state: &RunState,
        effect_id: &str,
        seat: &str,
    ) -> Result<ProbedRound, EngineError> {
        let waits = round.waits;
        let unrecorded = round.unrecorded;
        match self.probe_each(events, state, effect_id, seat, waits)? {
            ProbeRound::Stopped => Ok(ProbedRound::Stopped),
            ProbeRound::Parked(reason) => Ok(ProbedRound::Parked(reason)),
            ProbeRound::Held => Ok(ProbedRound::Again),
            ProbeRound::Extended(extension) => {
                match self.hold_extension(waits, &round.maxima, &extension, unrecorded)? {
                    Extension::Stopped => Ok(ProbedRound::Stopped),
                    Extension::Parked(reason) => Ok(ProbedRound::Parked(reason)),
                    Extension::HeldRound => Ok(ProbedRound::HeldRound),
                    Extension::Written(until) => {
                        // The written round is this episode's, so its end is
                        // the counted baseline and never a remainder again.
                        round.counted = round.counted.max(until);
                        Ok(ProbedRound::Again)
                    }
                }
            }
            ProbeRound::Answered(answers) => Ok(ProbedRound::Answered(answers)),
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
            Ok(Written { held: true, .. }) => Ok(Extension::HeldRound),
            Ok(Written { until, held: false }) => Ok(Extension::Written(until)),
            Err(error) => Ok(Extension::Parked(park_of(&error))),
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
            Err(error) => return Ok(WaitRound::Parked(park_of(&error))),
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
                Err(error) => return Ok(WaitRound::Parked(park_of(&error))),
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
                Err(error) => return Ok(WaitRound::Parked(park_of(&error))),
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
    /// extension's next round (later members wait for it), a stop, a hold
    /// another run's row put on a later member's route, or the park reason a
    /// probe end gives. The aggregate is re-read before every probe, not only
    /// at the wait's end: a row recorded while an earlier member's probe ran
    /// holds the later member inside the same round (this round's finding 3,
    /// 0103 ruling 2(a)).
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
            match self.cool_down_end(waits) {
                Ok(end) if end <= self.now() => {}
                Ok(_) => return Ok(ProbeRound::Held),
                Err(error) => return Ok(ProbeRound::Parked(park_of(&error))),
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
                    held: None,
                };
                return Err(CooldownError::Maximum(Box::new(MaximumFacts {
                    wait: pending,
                    episode: through,
                    max,
                    passed: None,
                })));
            }
        }
        Ok(WaitBudget { waited, maxima })
    }

    /// Append one probe's next round after judging it against the maximum
    /// with the remainders this wait has already taken: a round past the
    /// maximum writes nothing and parks with the maximum named (choice 3);
    /// a round within it is written at the terminal's own `(run_id,
    /// event_seq)`, and its end is what the caller waits. A round whose
    /// provider-stated reset alone passes its own site's maximum is another
    /// held round (decision 0106 ruling 2 and its amendment of 0103 ruling
    /// 4's probe-refusal loop): its row is written held, and the caller ends
    /// the drive with the run `running`. [`Engine::judge_round`] is the one
    /// judgement, each site against its own route. A rows-bearing store refusal parks
    /// naming the store's words, never silently. The effect's attempts are
    /// untouched.
    pub(super) fn record_extension(
        &mut self,
        waits: &[ProviderWait],
        maxima: &[Option<u64>],
        round: &NextRound,
        waited: u64,
    ) -> Result<Written, CooldownError> {
        let held = self.judge_round(waits, maxima, round, waited)?;
        // A round within every maximum has a length at most a declared
        // maximum, a `u32`, so its end is always representable
        // (`a_round_within_the_maximum_has_a_representable_end`). Only a
        // held round, the provider's word, can be too long for the journal
        // to spell its end, and it parks naming the provider's limit as the
        // terminal's held wait does (`no-end`).
        let Some(until) = sites::wait_until(self.now(), round.seconds) else {
            let held = held.expect("only a held round's end is unrepresentable");
            return Err(CooldownError::RoundNoEnd {
                provider: round.provider.clone(),
                route: round.route.clone(),
                seconds: round.seconds,
                max_seconds: held.max_seconds,
            });
        };
        let row = CooldownRound {
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
        };
        let appended = match held {
            Some(kind) => self.store.append_held_round(row, kind),
            None => self.store.append_cooldown_round(row),
        };
        match appended {
            Ok(RoundAppend::Written) => Ok(Written {
                until,
                held: held.is_some(),
            }),
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

    /// Judge one probe's next round before anything is written: its held
    /// kind, `None` for an ordinary round, or the park. The refused site's
    /// OWN route maximum classifies its reset, through the terminal's one
    /// decision ([`bound`]), before any episode sum, as each panel member's
    /// own plan classifies its own at the terminal: no other site's maximum
    /// classifies it (fix round 0106b1r3). A held round adds nothing to the
    /// episode, so nothing further judges it. An ordinary round's episode
    /// through it is then judged against every other cooled site's maximum,
    /// and passing any writes no row (finding 4, 0103 choice 3): another
    /// site's maximum bounds the shared episode, never the reset's kind.
    /// The maxima are the ones the round's wait read before the probes; a
    /// site the wait found no declaration for judges nothing, so the round
    /// parks rather than writing unjudged (finding 1).
    fn judge_round(
        &self,
        waits: &[ProviderWait],
        maxima: &[Option<u64>],
        round: &NextRound,
        waited: u64,
    ) -> Result<Option<Held>, CooldownError> {
        let unbounded = |wait: &ProviderWait| CooldownError::RoundUnbounded {
            provider: wait.provider.clone(),
            route: wait.route.clone(),
            words: round.words.clone(),
        };
        let mut bounded = Vec::with_capacity(waits.len());
        for (wait, max) in waits.iter().zip(maxima) {
            bounded.push((wait, max.ok_or_else(|| unbounded(wait))?));
        }
        let Some(&(own_wait, own)) = bounded.iter().find(|(wait, _)| wait.site == round.site)
        else {
            return Err(CooldownError::RoundUnbounded {
                provider: round.provider.clone(),
                route: round.route.clone(),
                words: round.words.clone(),
            });
        };
        let before = round.episode.saturating_add(waited);
        let through = match bound(round.retry_after_seconds, round.seconds, before, own) {
            Bound::Held(kind) => return Ok(Some(kind)),
            Bound::Past(through) => return Err(self.round_past(round, own_wait, through, own)),
            Bound::Within(through) => through,
        };
        for (wait, max) in bounded {
            if wait.site != round.site && past_maximum(0, through, max) {
                return Err(self.round_past(round, wait, through, max));
            }
        }
        Ok(None)
    }

    /// The park a probe's round past `wait`'s maximum `max` gives: the
    /// round that would be written, the episode through it, and the cooled
    /// site whose maximum it passed where that is not the round's own.
    fn round_past(
        &self,
        round: &NextRound,
        wait: &ProviderWait,
        through: u64,
        max: u64,
    ) -> CooldownError {
        let pending = ProviderWait {
            site: round.site.clone(),
            provider: round.provider.clone(),
            route: round.route.clone(),
            words: round.words.clone(),
            retry_after_seconds: round.retry_after_seconds,
            seconds: round.seconds,
            until: self.now(),
            rounds: round.round,
            held: None,
        };
        let passed = (wait.site != round.site).then(|| wait.site.clone().unwrap_or_default());
        CooldownError::Maximum(Box::new(MaximumFacts {
            wait: pending,
            episode: through,
            max,
            passed,
        }))
    }

    /// The wait's end: the latest of every recorded wait's own `until` and
    /// the store's aggregate for its provider and route. A store refusal
    /// parks by its own words, never as an absent aggregate (decision 0103
    /// ruling 2(a)); a version 1, 2 or 3 journal answers none, so the
    /// event's own `until` stands.
    pub(super) fn cool_down_end(
        &self,
        waits: &[ProviderWait],
    ) -> Result<OffsetDateTime, CooldownError> {
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
    ) -> Option<Park> {
        let refusing = |reason: String, refusal: ProviderLimitRefusal| {
            Park::new(reason, ParkCause::refusal(&wait.provider, refusal))
        };
        let route = || wait.route.clone();
        match self.cooldown_bounds(&wait.provider, &wait.route) {
            Ok(Some(bounds)) => {
                let max = u64::from(bounds.max_seconds.get());
                let episode = match episode_by_site(&self.store, &self.run_id, events, effect_id) {
                    Ok(episode) => episode,
                    Err(error) => {
                        return Some(refusing(
                            format!(
                                "a resume waits for {}/{} without reading its episode: {error}",
                                wait.provider, wait.route
                            ),
                            ProviderLimitRefusal::EpisodeUnreadable { route: route() },
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
                let own = episode.lengths.get(&wait.site).copied();
                let seconds = own.unwrap_or(wait.seconds);
                // A held wait's own length is the provider's word and is not
                // judged; a later round's length, where one was written, is
                // the engine's and is (decision 0106 ruling 2).
                let judged = match (wait.held, own) {
                    (Some(_), None) => 0,
                    _ => seconds,
                };
                let pending = ProviderWait {
                    site: wait.site.clone(),
                    provider: wait.provider.clone(),
                    route: wait.route.clone(),
                    words: wait.words.clone(),
                    retry_after_seconds: wait.retry_after_seconds,
                    seconds,
                    until: wait.until,
                    rounds,
                    held: wait.held,
                };
                // Judge with the same per-site length the park names, not
                // the terminal wait's own: a written probe round may have
                // superseded it (finding 6).
                past_maximum(judged, episode.seconds, max).then(|| {
                    refusing(
                        maximum_reason(&pending, episode.seconds, max, None),
                        ProviderLimitRefusal::PastMaximum {
                            route: route(),
                            until: pending.until,
                            seconds,
                            max_seconds: max,
                        },
                    )
                })
            }
            Ok(None) => None,
            Err(reason) => Some(refusing(
                format!(
                    "a resume waits for {}/{} without reading its bounds: {reason}",
                    wait.provider, wait.route
                ),
                ProviderLimitRefusal::HostUnreadable { route: route() },
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
    Parked(Park),
}

/// What a probe's rate-limit refusal left: the written round's end, a
/// written held round, a stop that landed while the probe ran, or the park
/// the judge gave.
enum Extension {
    Written(OffsetDateTime),
    HeldRound,
    Stopped,
    Parked(Park),
}

/// A probe's round as [`Engine::record_extension`] wrote it: its end, and
/// whether it is held (decision 0106 ruling 2).
pub(super) struct Written {
    until: OffsetDateTime,
    held: bool,
}

/// What one round of probes left the wait loop: answers to keep, a written
/// extension or a foreign hold that both send the loop back to wait, a
/// written held round that ends the drive, a stop, or a park.
enum ProbedRound {
    Answered(Vec<ProbeAnswer>),
    Again,
    HeldRound,
    Stopped,
    Parked(Park),
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
    HeldRound,
    Stopped,
    Parked(Park),
}

/// What one round of probes left.
enum ProbeRound {
    Answered(Vec<ProbeAnswer>),
    /// A rate-limited refusal's next round, before its row is written.
    Extended(NextRound),
    /// The aggregate still stands on a cooled site: the round waits again
    /// before any probe (finding 3).
    Held,
    Stopped,
    Parked(Park),
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
