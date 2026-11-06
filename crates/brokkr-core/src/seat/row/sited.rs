//! The site rows a run's starts open (decision 0089 ruling 7): each
//! provenance entry naming a provider opens a row of its own, with its
//! start, phase and provider, and a start naming an attempt and member
//! already opened opens another site, as the analytics walk reads it. The
//! key's later records land on the latest site's row, and the site's
//! metering rule, as brokkr-core reads the run's sites, moves it.

use serde_json::Value;

use super::{Member, Provider, Seats, Stamp};
use crate::envelope::EventEnvelope;
use crate::residual::text as field;
use crate::seat::boundary_entries;
use crate::seat::meter::Metering;
use crate::stamp::parse_millis;
use crate::summary::{Bounded, MODEL_BYTES};

impl Seats {
    /// An attempt starts: it is the one in flight, and each provenance
    /// entry naming a provider opens a site row of its own: a start naming
    /// an attempt and member already started opens another site, as the
    /// analytics walk reads it, and the key's later records land on the new
    /// site's row. The attempt's seat row, where its start names no site,
    /// takes this start's seq and the run's phase then, and the provider
    /// and the boundary the start records for each participant.
    pub(super) fn started(&mut self, request: usize, attempt: Option<&str>, event: &EventEnvelope) {
        self.requests[request].open = attempt.map(str::to_string);
        let seq = event.seq;
        let provenance: Vec<&Value> = event
            .payload
            .get("provenance")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .collect();
        // A site is a provenance entry of a start that names an attempt
        // and whose stamp parses, as brokkr-core reads it; a start naming
        // none, or one the reading ignores, opens participant rows alone.
        let read = parse_millis(&event.recorded_at).is_some();
        let sited = attempt.is_some() && read;
        let attempt_id = attempt.unwrap_or_default();
        // The seat's row opens first, as it always has: its own site where
        // the start names one, else the participant's row, re-opened.
        let seat = sited
            .then(|| {
                provenance.iter().position(|entry| {
                    field(entry, "member").is_none() && field(entry, "provider").is_some()
                })
            })
            .flatten();
        match seat {
            Some(seat) => {
                let provider = field(provenance[seat], "provider").expect("a provider");
                self.sited_row(request, attempt_id, Member::Seat, event, provider);
            }
            None => {
                self.place(request, attempt, Member::Seat);
            }
        }
        // A start dates the attempt's participant rows, those opened before
        // it and those of its own members named no provider; a site row is
        // dated by the start that opened it, never by a later re-open.
        let key = (request, attempt.map(str::to_string));
        let rows: Vec<usize> = self
            .at
            .get(&key)
            .into_iter()
            .flat_map(|members| members.values().copied())
            .collect();
        for at in rows {
            let row = &mut self.rows[at];
            if row.site.is_some() {
                continue;
            }
            row.started.get_or_insert_with(|| Stamp::of(event));
            row.since = Some(seq);
            row.phase.clone_from(&self.phase);
        }
        for (at, entry) in provenance.iter().enumerate() {
            if Some(at) == seat {
                continue;
            }
            let member = Member::of(field(entry, "member"));
            match (sited, field(entry, "provider")) {
                (true, Some(provider)) => {
                    self.sited_row(request, attempt_id, member, event, provider);
                }
                // A provenance entry naming no provider states the
                // participant's provider as none, as it always has, but
                // never a site's: the site reading opens no site for it
                // and leaves the site's provider be.
                (true, None) => {
                    let at = self.place(request, attempt, member);
                    let row = &mut self.rows[at];
                    if row.site.is_none() {
                        row.provider = Some((seq, Provider::Unnamed));
                    }
                }
                // A start naming no attempt opens no site; its provider is
                // its participant row's, as it always was. A start whose
                // stamp does not parse is no site at all: it states no
                // provider either, so no row is selected for a site the
                // reading never opened.
                (false, provider) if read => {
                    let row = self.row(request, attempt, member);
                    row.provider = Some((seq, Provider::of(provider)));
                }
                (false, _) => {}
            }
        }
        for entry in boundary_entries(&event.payload) {
            let row = self.row(request, attempt, Member::of(entry.member.as_deref()));
            row.boundary = Some((seq, entry.word));
        }
        self.carried(request, attempt, event);
    }

    /// A site's row, opened by a start's provenance entry for `member`:
    /// never the row its key opened before, so a re-opened attempt keeps
    /// one row per site, and the key points at the new row.
    fn sited_row(
        &mut self,
        request: usize,
        attempt: &str,
        member: Member,
        event: &EventEnvelope,
        provider: &str,
    ) -> usize {
        let site = self.sites;
        self.sites += 1;
        let at = self.fresh(request, attempt, member, site);
        let row = &mut self.rows[at];
        row.started = Some(Stamp::of(event));
        row.since = Some(event.seq);
        row.phase.clone_from(&self.phase);
        row.provider = Some((event.seq, Provider::of(Some(provider))));
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
            let row = &mut self.rows[at];
            row.metering = metering;
            row.site_model = model.map(|model| Bounded::within(model, MODEL_BYTES).cut.to_string());
        }
    }
}
