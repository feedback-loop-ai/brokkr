//! A panel's per-site waits (decision 0103 slice 4c2a, ruling 2(a)): at the
//! terminal event each member that refused with a rate limit records its own
//! wait under its own route's bounds, and the engine reads every entry back,
//! waits the latest, and probes each cooled member before the whole panel
//! retries. The per-site reading lives here so `cooldown.rs` stays within
//! ruling 4's 800-line ceiling and `cursor.rs` keeps its small cap branch.

use std::collections::BTreeMap;

use brokkr_core::envelope::EventType;
use brokkr_core::{provider_waits, EventEnvelope, ProviderRefusal, ProviderWait, RefusalCause};
use brokkr_protocol::{AttemptReport, PreSessionRefusal, RefusalClass};
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::{Duration as TimeDuration, OffsetDateTime};

use super::probe::Probe;
use super::{
    effect_seat, note_error, past_maximum, refusal_record, refusal_text, representable_end,
    terminal, waiter, CooldownError, Engine, EngineError,
};
use crate::bundle::{ExecutableBody, Invocation};
use crate::engine::carry::record::Tag;
use crate::engine::{start_failure_sites, Site};

/// One member that failed to start with a typed rate-limit refusal, tagged
/// as the journal tags it (decision 0103 ruling 1(a)): the site the panel's
/// terminal event and each wait entry name.
#[derive(Clone)]
pub(in crate::engine) struct RefusedMember {
    pub(in crate::engine) site: String,
    pub(in crate::engine) refusal: PreSessionRefusal,
}

/// One site of a failing sequence step that did not start, tagged as the
/// journal tags it, with the typed refusal its own report carried where it
/// has one (decision 0103 slice 4c2b): a spawn failure has no report and so
/// no refusal, and keeps today's start-failure site with no wait.
#[derive(Clone)]
pub(in crate::engine) struct FailedSite {
    pub(in crate::engine) site: Site,
    pub(in crate::engine) refusal: Option<PreSessionRefusal>,
}

impl FailedSite {
    /// The site as the terminal event's `start_failure_sites` spells it.
    pub(in crate::engine) fn site(&self) -> Site {
        self.site.clone()
    }
}

/// Every site of a failing step that did not start, in declared order, with
/// the typed refusal its report carried. The sites themselves come from the
/// one assembly a panel's start-failure fields already use, so the chain
/// advance per site stays unchanged.
pub(in crate::engine) fn failed_sites(
    reports: &[(String, AttemptReport)],
    tag_prefix: &str,
) -> Vec<FailedSite> {
    start_failure_sites(reports, tag_prefix)
        .into_iter()
        .map(|site| FailedSite {
            refusal: refusal_at(reports, tag_prefix, site.as_deref()),
            site,
        })
        .collect()
}

/// The typed refusal the report at `site` carried, where its report has one:
/// the same declared-order walk the site assembly makes.
fn refusal_at(
    reports: &[(String, AttemptReport)],
    tag_prefix: &str,
    site: Option<&str>,
) -> Option<PreSessionRefusal> {
    let site = site?;
    reports.iter().find_map(|(name, report)| {
        (Tag::prefixed(tag_prefix, name).0 == site)
            .then(|| report.refusal.as_deref().cloned())
            .flatten()
    })
}

/// The rate-limited sites among a failing step's, in declared order, as the
/// terminal writer's refused members: a site refused for another reason
/// follows decision 0006 unchanged and gets no wait and no probe.
pub(in crate::engine) fn refused_sites(failed: &[FailedSite]) -> Vec<RefusedMember> {
    failed
        .iter()
        .filter_map(|failed| {
            let refusal = failed.refusal.clone()?;
            let site = failed.site.clone()?;
            (refusal.class() == RefusalClass::RateLimit).then_some(RefusedMember { site, refusal })
        })
        .collect()
}

/// Every member whose report carries a typed rate-limit refusal, in declared
/// member order. A member refused for another reason is not one: it follows
/// decision 0006 unchanged and gets no wait and no probe.
pub(in crate::engine) fn refused_members(
    reports: &[(String, AttemptReport)],
    tag_prefix: &str,
) -> Vec<RefusedMember> {
    reports
        .iter()
        .filter_map(|(name, report)| {
            let refusal = report.refusal.as_deref()?;
            (refusal.class() == RefusalClass::RateLimit).then(|| RefusedMember {
                site: Tag::prefixed(tag_prefix, name).0,
                refusal: refusal.clone(),
            })
        })
        .collect()
}

/// The episode's waits per site and their seconds across every site: one
/// entry per earlier wait, keyed by the site it named (decision 0103 ruling
/// 3: the sum is across sites, the round is per site). An earlier entry that
/// does not read refuses by name, never as nothing.
pub(super) fn episode_by_site(
    events: &[EventEnvelope],
    effect_id: &str,
) -> Result<(BTreeMap<Option<String>, u64>, u64), CooldownError> {
    let mut sites = BTreeMap::new();
    let mut seconds = 0u64;
    for event in events {
        if !matches!(
            event.event_type,
            EventType::EffectFailed | EventType::EffectIndeterminate
        ) || event.payload.get("effect_id").and_then(Value::as_str) != Some(effect_id)
        {
            continue;
        }
        let Some(waits) =
            provider_waits(event).map_err(|source| CooldownError::EarlierWaitUnreadable {
                seq: event.seq,
                source,
            })?
        else {
            continue;
        };
        for wait in waits {
            *sites.entry(wait.site.clone()).or_insert(0u64) += 1;
            seconds = seconds.saturating_add(wait.seconds);
        }
    }
    Ok((sites, seconds))
}

/// The length one wait computes: the provider's own retry-after, else the
/// route's default doubled for each earlier wait of the same site
/// (decision 0103 ruling 3).
pub(super) fn wait_seconds(retry_after: Option<u64>, default: u32, rounds: u64) -> u64 {
    retry_after.unwrap_or_else(|| {
        u64::from(default).saturating_mul(2u64.saturating_pow((rounds - 1).min(32) as u32))
    })
}

/// The end a wait's length forms, or `None` where the journal cannot spell
/// it: a length past a nanosecond `time` or an RFC 3339 year is no wait.
pub(super) fn wait_until(now: OffsetDateTime, seconds: u64) -> Option<OffsetDateTime> {
    i64::try_from(seconds).ok().and_then(|seconds| {
        now.checked_add(TimeDuration::seconds(seconds))
            .and_then(representable_end)
    })
}

impl Engine {
    /// Every wait entry on the effect's last terminal event. An inline
    /// single seat carries exactly one; more refuses by name, as its own
    /// writer would (decision 0103 ruling 2(a)). A panel's entries each name
    /// their site, and the reader refuses two for one site.
    pub(in crate::engine) fn recorded_waits(
        &self,
        events: &[EventEnvelope],
        effect_id: &str,
    ) -> Result<Option<Vec<ProviderWait>>, CooldownError> {
        let Some(event) = super::terminal(events, effect_id) else {
            return Ok(None);
        };
        let Some(waits) = provider_waits(event).map_err(CooldownError::LastWaitUnreadable)? else {
            return Ok(None);
        };
        if waits.len() > 1 && waits.iter().any(|wait| wait.site.is_none()) {
            return Err(CooldownError::TooManyWaits {
                seq: event.seq,
                count: waits.len(),
            });
        }
        Ok(Some(waits))
    }

    /// The cooled site's probe: a member's own, through its panel member's
    /// spawn, or the single seat's when the wait names no site.
    pub(super) fn probe_wait(
        &mut self,
        events: &[EventEnvelope],
        state: &brokkr_core::fold::RunState,
        effect_id: &str,
        seat: &str,
        wait: &ProviderWait,
    ) -> Result<Probe, EngineError> {
        match wait.site.as_deref() {
            Some(member) => self.probe_member(events, state, effect_id, seat, wait, member),
            None => self.probe(events, state, effect_id, seat, wait),
        }
    }

    /// The panel terminal payload: every computable refused member's wait in
    /// declared order, and — where the panel may retry and one member's
    /// refusal parks it — the first such member's `refused` record and the
    /// park reason. An indeterminate panel parks anyway, so it takes the
    /// computable waits as evidence only and no member's refusal decides it.
    pub(in crate::engine) fn panel_rate_limit_terminal(
        &self,
        events: &[EventEnvelope],
        effect_id: &str,
        mut payload: Value,
        refused: &[RefusedMember],
        retry: bool,
    ) -> PanelTerminal {
        if refused.is_empty() {
            return PanelTerminal {
                payload,
                park: None,
            };
        }
        let Ok((site_rounds, prior_seconds)) = episode_by_site(events, effect_id) else {
            return self.panel_episode_unreadable(events, effect_id, &mut payload, refused, retry);
        };
        let mut planned: Vec<Option<Plan>> = Vec::with_capacity(refused.len());
        let mut causes: Vec<Option<Cause>> = Vec::with_capacity(refused.len());
        for member in refused {
            let rounds = site_rounds
                .get(&Some(member.site.clone()))
                .copied()
                .unwrap_or(0)
                .saturating_add(1);
            match self.plan_member(member, rounds) {
                PlannedMember::Wait(plan) if plan.until.is_none() && retry => {
                    planned.push(None);
                    causes.push(Some(plan.no_end()));
                }
                PlannedMember::Wait(plan) => {
                    planned.push(Some(plan));
                    causes.push(None);
                }
                PlannedMember::Cause(cause) => {
                    planned.push(None);
                    causes.push(Some(cause));
                }
                PlannedMember::Undeclared(note) => {
                    // The note is the failed event's error tail. An
                    // indeterminate event publishes no `error`, so the
                    // note stays off it: the contract edge admits only the
                    // terminal's own fields there (0071 ruling 3).
                    if retry {
                        note_error(&mut payload, &note);
                    }
                    planned.push(None);
                    causes.push(None);
                }
            }
        }
        let total = planned.iter().flatten().fold(prior_seconds, |total, plan| {
            total.saturating_add(plan.seconds)
        });
        for (at, plan) in planned.iter().enumerate() {
            if causes[at].is_none() {
                if let Some(cause) = plan.as_ref().and_then(|plan| plan.past_maximum(total)) {
                    causes[at] = Some(cause);
                }
            }
        }
        let waits: Vec<Value> = planned
            .iter()
            .flatten()
            .filter_map(Plan::wait)
            .map(|wait| waiter(&wait, wait.until))
            .collect();
        if !waits.is_empty() {
            payload["waits"] = Value::Array(waits);
        }
        let Some(first) = causes.iter().position(Option::is_some) else {
            return PanelTerminal {
                payload,
                park: None,
            };
        };
        if !retry {
            return PanelTerminal {
                payload,
                park: None,
            };
        }
        let cause = causes[first].take().expect("the first parked member");
        let refused = cause.refused;
        super::record_refusal(&mut payload, &refused);
        let reason = refusal_text(&refused, cause.episode);
        let park = (!self.at_attempt_cap(events, effect_id)).then_some(reason);
        PanelTerminal { payload, park }
    }

    /// A sequence step's terminal payload with the step's own waits, through
    /// the panel writer (decision 0103 slice 4c2b): a sequence attempt ends
    /// at its failing step, so only that step's sites appear. No refused site
    /// is no wait, and an indeterminate step takes the computable waits as
    /// evidence only and parks on its own reason.
    pub(in crate::engine) fn step_terminal(
        &self,
        effect_id: &str,
        payload: Value,
        failed: &[FailedSite],
        retry: bool,
    ) -> Result<PanelTerminal, EngineError> {
        let refused = refused_sites(failed);
        if refused.is_empty() {
            return Ok(PanelTerminal {
                payload,
                park: None,
            });
        }
        let events = self.store.load(&self.run_id)?;
        Ok(self.panel_rate_limit_terminal(&events, effect_id, payload, &refused, retry))
    }

    /// The episode an earlier wait made unreadable: no current wait can be
    /// computed, so a failed panel parks naming the first refused member
    /// (decision 0103 ruling 3), and an indeterminate one follows its own
    /// park unchanged.
    fn panel_episode_unreadable(
        &self,
        events: &[EventEnvelope],
        effect_id: &str,
        payload: &mut Value,
        refused: &[RefusedMember],
        retry: bool,
    ) -> PanelTerminal {
        let member = &refused[0];
        if retry {
            // A schema-valid refusal keeps the route its cause resolved: an
            // episode-unreadable record requires one, and a member whose own
            // route never resolved is its own route-unresolved park even when
            // the episode also does not read (decision 0103 ruling 3).
            let cause = match member.refusal.route() {
                Some(_) => RefusalCause::EpisodeUnreadable,
                None => RefusalCause::RouteUnresolved,
            };
            let refused = refusal_record(
                cause,
                Some(&member.site),
                &member.refusal,
                member.refusal.route(),
                None,
                None,
            );
            super::record_refusal(payload, &refused);
            let reason = refusal_text(&refused, None);
            let park = (!self.at_attempt_cap(events, effect_id)).then_some(reason);
            return PanelTerminal {
                payload: payload.clone(),
                park,
            };
        }
        PanelTerminal {
            payload: payload.clone(),
            park: None,
        }
    }

    /// One refused member's wait, or why its own route cannot produce one.
    /// Bounds are read under THAT member's provider and route (ruling 3);
    /// no declaration is 4c1's option (a): no wait, and a note naming the
    /// absence.
    fn plan_member(&self, member: &RefusedMember, rounds: u64) -> PlannedMember {
        let provider = member.refusal.provider();
        let Some(route) = member.refusal.route() else {
            return PlannedMember::Cause(Cause {
                refused: refusal_record(
                    RefusalCause::RouteUnresolved,
                    Some(&member.site),
                    &member.refusal,
                    None,
                    None,
                    None,
                ),
                episode: None,
            });
        };
        let bounds = match self.cooldown_bounds(provider, route) {
            Ok(Some(bounds)) => bounds,
            Ok(None) => {
                return PlannedMember::Undeclared(format!(
                    "no cool-down is declared for {provider}/{route} in {}",
                    self.host_spelling()
                ))
            }
            Err(_) => {
                return PlannedMember::Cause(Cause {
                    refused: refusal_record(
                        RefusalCause::HostUnreadable,
                        Some(&member.site),
                        &member.refusal,
                        Some(route),
                        None,
                        None,
                    ),
                    episode: None,
                })
            }
        };
        let retry_after = member.refusal.retry_after_seconds();
        let seconds = wait_seconds(retry_after, bounds.default_seconds.get(), rounds);
        let max = u64::from(bounds.max_seconds.get());
        let until = wait_until(self.now(), seconds);
        PlannedMember::Wait(Plan {
            member: member.clone(),
            seconds,
            rounds,
            until,
            max,
        })
    }
}

/// What one refused member's route produced for the terminal.
enum PlannedMember {
    Wait(Plan),
    Cause(Cause),
    Undeclared(String),
}

/// One computed wait, with the `until` the journal can spell (`None` is the
/// no-end cause) and the maximum that judges it.
struct Plan {
    member: RefusedMember,
    seconds: u64,
    rounds: u64,
    until: Option<OffsetDateTime>,
    max: u64,
}

impl Plan {
    fn wait(&self) -> Option<ProviderWait> {
        self.until.map(|until| ProviderWait {
            site: Some(self.member.site.clone()),
            provider: self.member.refusal.provider().to_string(),
            route: self
                .member
                .refusal
                .route()
                .expect("a planned wait resolved its route")
                .to_string(),
            words: self.member.refusal.words().to_string(),
            retry_after_seconds: self.member.refusal.retry_after_seconds(),
            seconds: self.seconds,
            until,
            rounds: self.rounds,
        })
    }

    fn past_maximum(&self, total: u64) -> Option<Cause> {
        past_maximum(self.seconds, total, self.max).then(|| Cause {
            refused: refusal_record(
                RefusalCause::PastMaximum,
                Some(&self.member.site),
                &self.member.refusal,
                self.member.refusal.route(),
                Some(self.seconds),
                Some(self.max),
            ),
            episode: Some(total),
        })
    }

    fn no_end(&self) -> Cause {
        Cause {
            refused: refusal_record(
                RefusalCause::NoEnd,
                Some(&self.member.site),
                &self.member.refusal,
                self.member.refusal.route(),
                Some(self.seconds),
                Some(self.max),
            ),
            episode: None,
        }
    }
}

/// One member's park: the record the failed event keeps and the episode its
/// text names where the cause has one.
struct Cause {
    refused: ProviderRefusal,
    episode: Option<u64>,
}

/// The declared order a journal recorded: the sites its failure named, in
/// the order it named them. This is the whole order for a single seat and
/// for a body without a panel declaration.
fn journaled_sites(events: &[EventEnvelope], effect_id: &str) -> Vec<String> {
    terminal(events, effect_id)
        .and_then(|event| event.payload.get("start_failure_sites"))
        .and_then(Value::as_array)
        .map(|sites| {
            sites
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

impl Engine {
    /// The declared member order the cap's facts follow: the bundle's own
    /// panel declaration under the journal's strategy, because the journal's
    /// `start_failure_sites` omits inline members (decision 0103 ruling 7(c),
    /// this fix round). The body is resolved through [`SeatBody::selected`],
    /// the one selection path the dispatch and the probe take (decision 0071
    /// rulings 2 and 5), so a selector-wrapped panel keeps its declaration. A
    /// body that is not a panel, or a seat the bundle no longer names, falls
    /// back to the sites the terminal event recorded.
    fn declared_sites(
        &self,
        events: &[EventEnvelope],
        effect_id: &str,
        strategy: Option<&str>,
    ) -> Vec<String> {
        let panel = effect_seat(events, effect_id)
            .and_then(|seat| self.bundle.seats.get(seat))
            .and_then(|seat| seat.body.selected(strategy))
            .and_then(|(body, _)| match body {
                ExecutableBody::Invocation(Invocation::Panel { members, .. }) => Some(
                    members
                        .iter()
                        .map(|member| Tag::prefixed("", &member.name).0)
                        .collect::<Vec<_>>(),
                ),
                _ => None,
            });
        panel.unwrap_or_else(|| journaled_sites(events, effect_id))
    }

    /// The cap's facts after 0006's text (decision 0103 ruling 7(c)): each
    /// of the last failure's waits in declared member order, and the
    /// refused record beside them at its own site where no wait carries it.
    /// A present wait or refusal that does not read refuses by name.
    ///
    /// A present `waits` array is never empty: `provider_waits` refuses one
    /// by name, the invariant `the_provider_wait_extension_refuses_by_name`
    /// is the removal control for (decision 0071 ruling 9).
    pub(in crate::engine) fn cap_facts(
        &self,
        events: &[EventEnvelope],
        effect_id: &str,
        waits: &Result<Option<Vec<ProviderWait>>, CooldownError>,
        refused: &Result<Option<ProviderRefusal>, CooldownError>,
        strategy: Option<&str>,
    ) -> String {
        let mut facts = String::new();
        let refused = match refused {
            Ok(value) => value.as_ref(),
            Err(detail) => {
                facts.push_str(&format!("; {detail}"));
                None
            }
        };
        match waits {
            Ok(Some(all)) => {
                self.cap_ordered_facts(events, effect_id, all, refused, &mut facts, strategy)
            }
            Ok(None) => cap_empty_wait(refused, &mut facts),
            Err(detail) => {
                if let Some(record) = refused {
                    facts.push_str(&cap_refusal_facts(record, None));
                }
                facts.push_str(&format!("; {detail}"));
            }
        }
        facts
    }

    /// Every wait's facts in declared order, with the refused record at its
    /// own site: the parking member's record and its wait are one entry, so
    /// a record is never paired with another member's wait.
    fn cap_ordered_facts(
        &self,
        events: &[EventEnvelope],
        effect_id: &str,
        waits: &[ProviderWait],
        refused: Option<&ProviderRefusal>,
        facts: &mut String,
        strategy: Option<&str>,
    ) {
        let declared = self.declared_sites(events, effect_id, strategy);
        let mut listed = vec![false; waits.len()];
        let mut recorded = false;
        for site in &declared {
            let at = waits
                .iter()
                .position(|wait| wait.site.as_deref() == Some(site.as_str()));
            if let Some(at) = at {
                listed[at] = true;
                self.cap_push_wait(facts, events, effect_id, &waits[at], refused);
            } else if let Some(record) =
                refused.filter(|record| record.site.as_deref() == Some(site.as_str()))
            {
                facts.push_str(&cap_refusal_facts(record, None));
                recorded = true;
            }
        }
        for (at, wait) in waits.iter().enumerate() {
            if !listed[at] {
                self.cap_push_wait(facts, events, effect_id, wait, refused);
            }
        }
        if let Some(record) = refused {
            if !recorded && !waits.iter().any(|wait| wait.site == record.site) {
                facts.push_str(&cap_refusal_facts(record, None));
            }
        }
    }

    /// One wait's facts and maximum, pairing a refused record only with the
    /// wait that carries its own site.
    fn cap_push_wait(
        &self,
        facts: &mut String,
        events: &[EventEnvelope],
        effect_id: &str,
        wait: &ProviderWait,
        refused: Option<&ProviderRefusal>,
    ) {
        match refused.filter(|record| record.site == wait.site) {
            Some(record) => facts.push_str(&cap_refusal_facts(record, Some(wait))),
            None => facts.push_str(&cap_wait_facts(wait)),
        }
        facts.push_str(&self.cap_maximum(events, effect_id, wait));
    }

    /// The cap's sentence for the declared maximum: the episode past it, the
    /// declaration when the wait is still within it, or why the episode
    /// cannot be read.
    fn cap_maximum(
        &self,
        events: &[EventEnvelope],
        effect_id: &str,
        wait: &ProviderWait,
    ) -> String {
        match self.wait_maximum(events, effect_id, wait) {
            Ok(Some((total, max, true))) => {
                format!("; the episode {total}s is past the declared {max}s maximum")
            }
            Ok(Some((_, max, false))) => format!("; the declared {max}s maximum"),
            Ok(None) => String::new(),
            Err(detail) => format!("; {detail}"),
        }
    }
}

/// A cap whose last failure carries no wait: the refused record alone, and
/// the missing `until` it names.
fn cap_empty_wait(refused: Option<&ProviderRefusal>, facts: &mut String) {
    if let Some(record) = refused {
        facts.push_str(&cap_refusal_facts(record, None));
        facts.push_str("; no until exists for it");
    }
}

/// The cap's facts about a recorded refusal: provider, route, the `until`
/// the wait entry beside it may carry, the provider's words, its retry-after
/// and the declared maximum (decision 0103 ruling 7(c), this fix round).
fn cap_refusal_facts(refused: &ProviderRefusal, wait: Option<&ProviderWait>) -> String {
    let route = refused.route.as_deref().unwrap_or("an unresolved route");
    let member = refused
        .site
        .as_deref()
        .map_or_else(String::new, |site| format!("; member {site}"));
    let mut facts = match wait {
        Some(wait) => format!(
            "; the last refusal waits for {}/{} until {}: {}",
            refused.provider,
            route,
            wait.until
                .format(&Rfc3339)
                .expect("an RFC 3339 time formats as RFC 3339"),
            refused.words
        ),
        None => format!(
            "{member}; the last refusal names {}/{} with no until: {}",
            refused.provider, route, refused.words
        ),
    };
    if wait.is_some() {
        facts = format!("{member}{facts}");
    }
    if let Some(retry) = refused.retry_after_seconds {
        facts.push_str(&format!("; the provider gave retry-after {retry}s"));
    }
    if let (Some(seconds), Some(max)) = (refused.seconds, refused.max_seconds) {
        facts.push_str(&format!(
            "; the computed wait was {seconds}s; the declared maximum was {max}s"
        ));
    }
    facts
}

/// The cap's facts from the wait entry alone, for a journal whose terminal
/// event carries no `refused` record: its provider, route, `until`, words
/// and the retry-after it recorded.
fn cap_wait_facts(wait: &ProviderWait) -> String {
    let member = wait
        .site
        .as_deref()
        .map_or_else(String::new, |site| format!("; member {site}"));
    let mut facts = format!(
        "{member}; the last refusal waits for {}/{} until {}: {}",
        wait.provider,
        wait.route,
        wait.until
            .format(&Rfc3339)
            .expect("an RFC 3339 time formats as RFC 3339"),
        wait.words
    );
    if let Some(retry) = wait.retry_after_seconds {
        facts.push_str(&format!("; the provider gave retry-after {retry}s"));
    }
    facts
}

/// What a panel's terminal event carries: the payload with its waits and,
/// below the cap, the park reason the run ends with.
pub(in crate::engine) struct PanelTerminal {
    pub(in crate::engine) payload: Value,
    pub(in crate::engine) park: Option<String>,
}
