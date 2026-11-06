//! The site rows a run's starts open (decision 0089 ruling 7): each
//! provenance entry naming a provider opens a row of its own, with its
//! start, phase and provider, and a start naming an attempt and member
//! already opened opens another site, as the analytics walk reads it. The
//! key's later records land on the latest site's row, and the site's
//! metering rule, as brokkr-core reads the run's sites, moves it.

use serde_json::Value;

use super::{Carry, Member, Provider, Seats, Stamp};
use crate::envelope::EventEnvelope;
use crate::residual::text as field;
use crate::seat::boundary_entries;
use crate::seat::meter::Metering;
use crate::stamp::parse_millis;
use crate::summary::{Bounded, MODEL_BYTES, NAME_BYTES};

/// A start's typed facts, as its rows and sites read them: its seq and
/// stamp, the attempt it names, the phase the run was in when it landed,
/// whether its stamp parses, and the provenance, boundary and carried
/// entries it states. Read once, so a start held before its effect's
/// request keeps no payload (decision 0071 ruling 3).
#[derive(Debug, Clone)]
pub(super) struct Start {
    pub(super) seq: u64,
    pub(super) at: String,
    pub(super) attempt: Option<String>,
    pub(super) phase: Option<String>,
    pub(super) read: bool,
    pub(super) provenance: Vec<Entry>,
    pub(super) boundary: Vec<(Option<String>, String)>,
    pub(super) carried: Vec<(String, String)>,
}

/// One provenance entry a start states: the member it names, where it
/// names one, and its provider, where it names one.
#[derive(Debug, Clone)]
pub(super) struct Entry {
    pub(super) member: Option<String>,
    pub(super) provider: Option<String>,
}

impl Start {
    /// `event`'s start, with `phase` the run's when it landed.
    pub(super) fn of(event: &EventEnvelope, phase: Option<String>) -> Start {
        let payload = &event.payload;
        let entries = payload.get("provenance").and_then(Value::as_array);
        let provenance = entries
            .into_iter()
            .flatten()
            .map(|entry| Entry {
                member: field(entry, "member").map(str::to_string),
                provider: field(entry, "provider").map(str::to_string),
            })
            .collect();
        let boundary = boundary_entries(payload)
            .into_iter()
            .map(|entry| (entry.member, entry.word))
            .collect();
        let entries = payload.get("carried").and_then(Value::as_array);
        let carried = entries
            .into_iter()
            .flatten()
            .filter_map(|entry| {
                let member = field(entry, "member")?;
                let attempt = field(entry, "attempt_id")?;
                Some((member.to_string(), attempt.to_string()))
            })
            .collect();
        Start {
            seq: event.seq,
            at: event.recorded_at.clone(),
            attempt: field(payload, "attempt_id").map(str::to_string),
            phase,
            read: parse_millis(&event.recorded_at).is_some(),
            provenance,
            boundary,
            carried,
        }
    }

    /// Whether a provenance entry names a provider: the start opens a site,
    /// as brokkr-core reads one.
    pub(super) fn provider_bearing(&self) -> bool {
        self.attempt.is_some()
            && self.read
            && self.provenance.iter().any(|entry| entry.provider.is_some())
    }
}

impl Seats {
    /// An attempt starts: it is the one in flight, and each provenance
    /// entry naming a provider opens a site row of its own: a start naming
    /// an attempt and member already started opens another site, as the
    /// analytics walk reads it, and the key's later records land on the new
    /// site's row. Every start also states the participant's current facts,
    /// provider-bearing or not: its rows take this start's seq, and its
    /// provenance entries its provider, as main reads them. A site row's own
    /// dated facts, its start, phase and provider key, stay as the start
    /// that opened it set them. `start.phase` is the run's phase when the
    /// start landed, which an early start read at its request keeps.
    pub(super) fn started(&mut self, request: usize, start: &Start) {
        self.requests[request].open = start.attempt.clone();
        let seq = start.seq;
        let provenance = &start.provenance;
        // A site is a provenance entry of a start that names an attempt
        // and whose stamp parses, as brokkr-core reads it; a start naming
        // none, or one the reading ignores, opens participant rows alone.
        let sited = start.attempt.is_some() && start.read;
        let attempt_id = start.attempt.as_deref().unwrap_or_default();
        // The seat's row opens first, as it always has: its own site where
        // the start names one, else the participant's row, re-opened.
        let seat = sited
            .then(|| {
                provenance
                    .iter()
                    .position(|entry| entry.member.is_none() && entry.provider.is_some())
            })
            .flatten();
        match seat {
            Some(seat) => {
                let provider = provenance[seat].provider.as_deref().expect("a provider");
                self.sited_row(request, attempt_id, Member::Seat, start, provider);
            }
            None => {
                self.place(request, start.attempt.as_deref(), Member::Seat);
            }
        }
        // Every start moves its participant rows' current facts: the seq
        // the attempt last started at, and, for a row no site opened, the
        // start's own stamp. A site row is dated by the start that opened
        // it, never by a later re-open, even when that start states the
        // participant's provider.
        let key = (request, start.attempt.clone());
        let rows: Vec<usize> = self
            .at
            .get(&key)
            .into_iter()
            .flat_map(|members| members.values().copied())
            .collect();
        for at in rows {
            let row = &mut self.rows[at];
            row.since = Some(seq);
            if row.site.is_some() {
                continue;
            }
            row.started.get_or_insert_with(|| Stamp {
                seq,
                at: start.at.clone(),
            });
            row.phase = start.phase.clone();
        }
        for (at, entry) in provenance.iter().enumerate() {
            if Some(at) == seat {
                continue;
            }
            let member = Member::of(entry.member.as_deref());
            match (sited, entry.provider.as_deref()) {
                (true, Some(provider)) => {
                    self.sited_row(request, attempt_id, member, start, provider);
                }
                // A provenance entry naming no provider states the
                // participant's provider as none, as main does, and opens
                // no site: an earlier site's row keeps its provider key
                // while the participant's current provider moves.
                (true, None) => {
                    let at = self.place(request, start.attempt.as_deref(), member);
                    self.rows[at].provider = Some((seq, Provider::Unnamed));
                }
                // A start naming no attempt opens no site; its provider is
                // its participant row's, as it always was. A start whose
                // stamp does not parse is no site at all: it states no
                // provider either, so no row is selected for a site the
                // reading never opened.
                (false, provider) if start.read => {
                    let row = self.row(request, start.attempt.as_deref(), member);
                    row.provider = Some((seq, Provider::of(provider)));
                }
                (false, _) => {}
            }
        }
        for (member, word) in &start.boundary {
            let row = self.row(
                request,
                start.attempt.as_deref(),
                Member::of(member.as_deref()),
            );
            row.boundary = Some((seq, word.clone()));
        }
        self.carried(request, start.attempt.as_deref(), seq, &start.carried);
    }

    /// A start's carried entries: each entry naming a member and an
    /// attempt opens its member's row and records that attempt, unsourced
    /// until [`Seats::sourced`] names its source; an entry naming either not
    /// is passed over, as the view passes it over.
    fn carried(
        &mut self,
        request: usize,
        attempt: Option<&str>,
        seq: u64,
        carried: &[(String, String)],
    ) {
        for (member, source) in carried {
            let row = self.row(request, attempt, Member::of(Some(member)));
            row.carried = Some((seq, Carry::Unsourced(source.clone())));
        }
    }

    /// A site's row, opened by a start's provenance entry for `member`:
    /// never the row its key opened before, so a re-opened attempt keeps
    /// one row per site, and the key points at the new row.
    fn sited_row(
        &mut self,
        request: usize,
        attempt: &str,
        member: Member,
        start: &Start,
        provider: &str,
    ) -> usize {
        let site = self.sites;
        self.sites += 1;
        let at = self.fresh(request, attempt, member.clone(), site);
        let row = &mut self.rows[at];
        row.started = Some(Stamp {
            seq: start.seq,
            at: start.at.clone(),
        });
        row.since = Some(start.seq);
        row.phase = start.phase.clone();
        row.provider = Some((start.seq, Provider::of(Some(provider))));
        row.site_provider = Some(Bounded::within(provider, NAME_BYTES).cut.to_string());
        self.site_starts.insert((start.seq, member.clone()), at);
        at
    }

    /// A site's row, opened by a start's provenance entry: never the row
    /// its key opened before, so a re-opened attempt keeps one row per
    /// site, and the key points at the new row.
    fn fresh(&mut self, request: usize, attempt: &str, member: Member, site: u64) -> usize {
        let key = (request, Some(attempt.to_string()));
        let mut row = self.aside.lit(self.opened(&key, &member));
        row.site = Some(site);
        self.rows.push(row);
        let at = self.rows.len() - 1;
        self.at.entry(key).or_default().insert(member.clone(), at);
        let key = (attempt.to_string(), member.tag().map(str::to_string));
        self.sited.insert(key, at);
        at
    }

    /// What the site of `attempt`'s `member` meters by and the last model
    /// it names, as brokkr-core reads the run's sites: the row follows the
    /// reading, which its participant's turns and session record do not
    /// (decision 0089 ruling 7).
    pub(crate) fn resited(
        &mut self,
        attempt: &str,
        member: Option<&str>,
        metering: Metering,
        model: Option<&str>,
    ) {
        let key = (attempt.to_string(), member.map(str::to_string));
        if let Some(&at) = self.sited.get(&key) {
            self.resite(at, metering, model);
        }
    }

    /// Move the site row opened by the start at `start` for `member` to
    /// `metering` and `model`: a later session record of its participant
    /// moved the flag off it, so its rule is no longer the one it was
    /// written with (decision 0089 ruling 7).
    pub(crate) fn resited_start(
        &mut self,
        start: u64,
        member: Option<&str>,
        metering: Metering,
        model: Option<&str>,
    ) {
        let key = (start, Member::of(member));
        if let Some(&at) = self.site_starts.get(&key) {
            self.resite(at, metering, model);
        }
    }

    /// Move the site row at `at` to `metering` and `model`.
    fn resite(&mut self, at: usize, metering: Metering, model: Option<&str>) {
        let row = &mut self.rows[at];
        row.metering = metering;
        row.site_model = model.map(|model| Bounded::within(model, MODEL_BYTES).cut.to_string());
    }
}
