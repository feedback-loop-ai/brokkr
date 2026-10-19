//! The console's reads: the one refresh source `brokkr tui` and every run
//! view run on, the only place a store is opened on that path. It stands
//! beside `tui.rs` rather than under it, because the console's own
//! modules name no store at all (the TUI's source-level proofs hold that).
//! #744 moved it here from `lib.rs` and made the fleet a kept read.

use std::path::Path;

use anyhow::Result;
use brokkr_core::fold::{fold, RunState};
use brokkr_core::EventEnvelope;
use brokkr_runtime::realms::Hearth;
use brokkr_store::{Store, StoreError};
use brokkr_view::transcript::{LegacyProvenance, TranscriptRead};
use brokkr_view::{Paged, RunsView, Section};

use crate::fleet::{Journal, Pages, AHEAD_ROWS};
use crate::{local_transcript, tui};

/// The in-memory stamp of the selected transcript source: the subject
/// identity and the bounded result it last produced. It holds no mtime
/// and no length — the same bytes are re-derived and compared, which is
/// what lets a same-length rewrite or an append be told apart. No
/// persistent cache and no fleet-wide body cache exist.
pub(crate) struct SourceStamp {
    tab: usize,
    realm: Option<String>,
    run: String,
    key: String,
    reference: Option<brokkr_view::Transcript>,
    provenance: LegacyProvenance,
    legacy_id: Option<String>,
    read: TranscriptRead,
}

impl SourceStamp {
    pub(crate) fn of(subject: &tui::Subject, read: TranscriptRead) -> SourceStamp {
        SourceStamp {
            tab: subject.tab,
            realm: subject.realm.clone(),
            run: subject.run.clone(),
            key: subject.key.clone(),
            reference: subject.reference.clone(),
            provenance: subject.provenance,
            legacy_id: subject.legacy_id.clone(),
            read,
        }
    }

    /// The identity half the design fixes: realm/journal, run, participant
    /// key and the complete effective reference — including the legacy
    /// synthesis inputs, so a concluded participant whose legacy id or
    /// provenance changes is re-read rather than keeping the old prose.
    pub(crate) fn same_subject(&self, subject: &tui::Subject) -> bool {
        self.tab == subject.tab
            && self.realm == subject.realm
            && self.run == subject.run
            && self.key == subject.key
            && self.reference == subject.reference
            && self.provenance == subject.provenance
            && self.legacy_id == subject.legacy_id
    }
}

/// Re-resolve the selected participant's transcript under the shared
/// bounded reader. The journal is read-only. A subject is re-read at every
/// refresh opportunity while it works and on an explicit refresh, and
/// again whenever the selected reference itself changed — which is also
/// the final read a concluding participant receives. A concluded,
/// unchanged subject is not re-read: automatic growth polling has stopped,
/// and only an explicit refresh re-resolves it.
fn resolve_subject(
    subject: &tui::Subject,
    force: bool,
    seen: &mut Option<SourceStamp>,
    secrets: &std::path::Path,
    projects_home: Option<&str>,
) -> (Option<TranscriptRead>, bool) {
    let identity_changed = seen
        .as_ref()
        .is_none_or(|stamp| !stamp.same_subject(subject));
    let fresh = if force || subject.working || identity_changed {
        Some(local_transcript::mask_secrets(
            local_transcript::read_local(subject, projects_home),
            secrets,
        ))
    } else {
        None
    };
    let changed = match (&fresh, &*seen) {
        (Some(read), Some(stamp)) => !stamp.same_subject(subject) || stamp.read != *read,
        (Some(_), None) => true,
        (None, _) => false,
    };
    if let Some(read) = &fresh {
        *seen = Some(SourceStamp::of(subject, read.clone()));
    }
    // A concluded, unchanged subject keeps the frame it already has; the
    // stamp's own bounded result is that frame's transcript.
    let resolved = fresh.or_else(|| seen.as_ref().map(|stamp| stamp.read.clone()));
    (resolved, changed)
}

fn resolve_transcript(
    ask: &tui::Ask,
    seen: &mut Option<SourceStamp>,
    secrets: &std::path::Path,
    projects_home: Option<&str>,
) -> (Option<TranscriptRead>, bool) {
    let Some(subject) = ask.subject.as_ref() else {
        *seen = None;
        return (None, false);
    };
    resolve_subject(subject, ask.force, seen, secrets, projects_home)
}

/// Rebuild the selected subject from the freshly folded run, so authority
/// that changed inside this frame is read before it is displayed. `None`
/// means the participant is no longer in the fresh view.
fn refreshed_subject(prior: &tui::Subject, view: &brokkr_view::RunView) -> Option<tui::Subject> {
    let part = view
        .participants
        .iter()
        .find(|part| part.key == prior.key)?;
    Some(tui::Subject::of(
        prior.tab,
        prior.realm.clone(),
        prior.run.clone(),
        part,
    ))
}

/// What the console keeps of one hearth between refreshes: the head of
/// the run it last read and that run's journal, and the fleet's kept rows
/// (#744). Nothing here is persisted; it lives as long as the console's
/// source does.
#[derive(Default)]
pub(crate) struct Kept {
    head: Option<(u64, String)>,
    run: Option<Selected>,
    fleet: Pages,
    /// How many of the fleet's rows the last frame read (decision 0089
    /// ruling 6): a list that asks for more is read again.
    rows: usize,
    /// The events the last frame read, the fleet's and the run's: the
    /// work a refresh paid for.
    read: usize,
}

/// One refresh for `brokkr tui`: the only place a store is opened on that
/// path, and the reason `tui.rs` can name none. The head is compared on
/// **both** seq and hash — a rewritten journal at equal seq is the
/// tamper case `anchor` exists for, and a console should redraw rather
/// than sit blind. The fleet is the kept read (#744): a refresh reads
/// only what landed since the last one, from the journal file it was kept
/// from.
fn read_views(
    db: &Path,
    // Whether this world has exactly one hearth. A one-hearth world
    // opens its journal exactly as it always has; a many-hearth world
    // opens each hearth READ-ONLY, because a console that CREATED the
    // journal of a realm the operator merely tabbed past would have
    // written to a world it came only to read (decision 0026 ruling 5).
    sole: bool,
    ask: tui::Ask,
    kept: &mut Kept,
    seen: &mut Option<SourceStamp>,
    clock: fn() -> String,
) -> Result<tui::Refreshed> {
    // A realm the map names before its first run has no journal yet, and
    // that is an ordinary state of the world, not a fault: the hearth is
    // EMPTY, and an empty hearth is a frame that says so with the keys
    // still live (decision 0026 ruling 2) — exactly what `brokkr runs`
    // already prints for the same realm. Refusing here would count
    // against the console's give-up bound and end the session over a
    // realm the operator merely tabbed to. Only in a many-hearth world:
    // a sole journal that is not on disk is refused by `tui::start`
    // before the loop begins, and that refusal is unchanged.
    if !sole && !db.is_file() {
        return Ok(no_journal_yet(db, &ask, kept, seen, clock));
    }
    // Reading is inert: even the one-hearth console, the journal the
    // operator named, opens read-only for this refresh. Transcript
    // reading must not enable WAL, migrate columns or repair guards on
    // the journal it came to read (transcript-reading's inertness rule).
    // The journal file is stated before the store opens it, so a file
    // swapped in before the open is not taken for the one stated.
    let before = Journal::at(db);
    let store = Store::open_read_only(db)?;
    let current = match ask.run {
        Some(run) => Some(store.head_hash(run)?),
        None => None,
    };
    // Unconditional, and before the gate: the resolved read (and the
    // stamp it updates) is what the next tick compares against, whatever
    // the gate rules. A working seat's prose lands between checkpoints,
    // so the read is its own refresh reason.
    // The prose is masked against the secrets store beside this journal
    // before the stamp keeps it, so a refresh compares masked to masked.
    // The projects home is resolved once for both reads this frame makes.
    let secrets = local_transcript::store_beside(db);
    let home = crate::local_projects_home();
    let (transcript, transcript_changed) =
        resolve_transcript(&ask, seen, &secrets, home.as_deref());
    // A journal that is not the file the fleet was kept from has moved,
    // whatever head the open run still holds in it: the selected run is
    // read whole again with the fleet.
    let moved = current != kept.head || kept.fleet.replaced(before);
    let scrolled = ask.rows != kept.rows;
    if !(ask.force || ask.fleet || moved || transcript_changed || scrolled) {
        // Nothing has moved: the console keeps the frame it has, and
        // nothing is re-folded at four polls a second.
        return Ok(None);
    }
    kept.head = current;
    // Quarantined per run: a console would otherwise lose the operator's
    // whole fleet table because one old run is corrupt. The absence mark
    // is `RunRow.status_known`'s job (0001), and the refusal's own words
    // ride along as the row's detail. A page at a time (decision 0089):
    // the rows the list asks for, the selected run's values whole.
    let now = clock();
    let (runs, unread) = fleet_page(&store, kept, &now, (ask.rows, ask.run), before)
        .map_err(|error| anyhow::Error::msg(error.to_string()))?;
    let run = run_of(&store, ask.run, ask.force || moved, kept);
    let transcript = reconciled(
        Reconcile {
            prior: ask.subject.as_ref(),
            view: run.as_ref(),
            secrets: &secrets,
            home: home.as_deref(),
        },
        transcript,
        seen,
    );
    Ok(Some(tui::Views {
        now,
        runs,
        run,
        // The selected participant's own bounded local read, produced by
        // the shared reader the command and the browser also consume.
        transcript,
        // A journal that was read has nothing to say about itself.
        note: None,
        unread,
    }))
}

/// The fleet's rows the list asks for, `rows` of them from the first and
/// the run `selected` with its values whole, and what they leave unread:
/// the older runs past them, counted by the summary's index, and the runs
/// behind their summary past those this refresh reads whole.
fn fleet_page(
    store: &Store,
    kept: &mut Kept,
    now: &str,
    (rows, selected): (usize, Option<&str>),
    before: Option<Journal>,
) -> Result<(RunsView, tui::Unread), StoreError> {
    kept.read = kept.fleet.refresh(store, before, AHEAD_ROWS)?;
    let page = kept
        .fleet
        .page(store, now, None, rows, &|run| Some(run) == selected)?;
    let older = page
        .rows
        .iter()
        .filter(|(section, _)| *section == Section::Older)
        .count();
    let unread = tui::Unread {
        older: kept.fleet.older(store, now)?.saturating_sub(older),
        behind: kept.fleet.unread(),
    };
    let mut view = RunsView::of(page.rows.into_iter().map(|(_, row)| row).collect());
    view.page = Some(Paged { next: page.next });
    kept.rows = rows;
    Ok((view, unread))
}

/// The selected run as the console last read it: its journal and what
/// that folded to.
struct Selected {
    run: String,
    events: Vec<EventEnvelope>,
    state: Option<RunState>,
}

/// Run `run`'s view, derived from the journal kept for it while its head
/// stands, no rebuild is asked and the fleet carried it, so the fleet's
/// cadence reads none of the run; read whole and folded otherwise, as the
/// fleet reads a run whose journal it cannot carry. A run that would not
/// load keeps nothing, so the next frame reads it again.
fn run_of(
    store: &Store,
    run: Option<&str>,
    rebuild: bool,
    kept: &mut Kept,
) -> Option<brokkr_view::RunView> {
    let standing = kept.run.take();
    let run = run?;
    let rebuild = rebuild || kept.fleet.was_read_whole(run);
    let kept_run = standing.filter(|selected| !rebuild && selected.run == run);
    let selected = match kept_run.filter(|selected| still_whole(store, selected)) {
        Some(selected) => selected,
        None => {
            let events = store.load(run).ok()?;
            kept.read += events.len();
            let state = fold(&events).ok();
            Selected {
                run: run.to_string(),
                events,
                state,
            }
        }
    };
    let view = brokkr_view::run_view(&selected.events, selected.state.as_ref());
    kept.run = Some(selected);
    Some(view)
}

/// Whether the journal still holds every row of the run `selected` kept,
/// counted by the key's index and no event read: a row lost below its
/// head, which no head compares, has the run opened again (decision 0089
/// ruling 5), as #744's kept read counts every run it keeps.
fn still_whole(store: &Store, selected: &Selected) -> bool {
    let head = selected.events.last().map_or(0, |event| event.seq);
    let counted = store.rows_through(&[(selected.run.as_str(), head)]);
    counted.is_ok_and(|counted| counted == [head])
}

/// The frame of a hearth whose journal is not there yet: an empty fleet
/// that says so, built only when a frame is due.
fn no_journal_yet(
    db: &Path,
    ask: &tui::Ask,
    kept: &mut Kept,
    seen: &mut Option<SourceStamp>,
    clock: fn() -> String,
) -> tui::Refreshed {
    if !(ask.force || ask.fleet) {
        return None;
    }
    // Nothing was read, so nothing is remembered as read: the tick
    // that finds the journal finally there rebuilds from scratch.
    *kept = Kept::default();
    *seen = None;
    Some(tui::Views {
        now: clock(),
        note: Some(format!(
            "this realm has no journal yet, and a read never creates one: {}",
            db.display()
        )),
        ..tui::Views::empty()
    })
}

/// What a frame's transcript is reconciled against: the subject it was
/// asked about, the run view freshly folded, and where its prose is read.
struct Reconcile<'a> {
    prior: Option<&'a tui::Subject>,
    view: Option<&'a brokkr_view::RunView>,
    secrets: &'a Path,
    home: Option<&'a str>,
}

/// The selected participant's authority can change inside this same
/// frame: folding may reveal a new common reference, a new legacy id or
/// a concluded state. Re-resolve the subject against the fresh view
/// before publishing, so the frame never pairs new authority with the
/// previous reference's prose.
fn reconciled(
    against: Reconcile<'_>,
    transcript: Option<TranscriptRead>,
    seen: &mut Option<SourceStamp>,
) -> Option<TranscriptRead> {
    let (Some(prior), Some(view)) = (against.prior, against.view) else {
        return transcript;
    };
    match refreshed_subject(prior, view) {
        Some(fresh) if &fresh != prior => {
            resolve_subject(&fresh, true, seen, against.secrets, against.home).0
        }
        // An unchanged participant: the resolved read still speaks for
        // it.
        Some(_) => transcript,
        // The participant is gone from a view that still lists
        // participants: its content and any open overlay are cleared
        // in this same frame rather than lingering until a later
        // refresh. A view with no participants at all is the shell's
        // own absent-subject frame, cleared there.
        None if !view.participants.is_empty() => {
            *seen = None;
            None
        }
        None => transcript,
    }
}

/// [`read_views`] with nothing kept from an earlier refresh but the head:
/// every frame reads the fleet whole, as the console read it before
/// #744. The shape the console's frame-by-frame proofs are written in.
#[cfg(test)]
pub(crate) fn tui_views(
    db: &Path,
    sole: bool,
    ask: tui::Ask,
    head: &mut Option<(u64, String)>,
    seen: &mut Option<SourceStamp>,
    clock: fn() -> String,
) -> Result<tui::Refreshed> {
    let mut kept = Kept {
        head: head.take(),
        rows: ask.rows,
        ..Kept::default()
    };
    let views = read_views(db, sole, ask, &mut kept, seen, clock);
    *head = kept.head;
    views
}

/// The refresh source the console runs on, with the workspace clock
/// bound in. Built by a named function so it is reachable from a test
/// as well as from `brokkr tui`. It owns what it keeps, so it can be
/// handed to the thread the console reads on (#744).
///
/// One hearth or many, the ACTIVE tab's journal is the only one this
/// ever opens: a tab nobody has visited has never been asked about, so
/// its store is never opened at all and its journal is never polled
/// (decision 0026 ruling 2). The laziness is the absence of a question,
/// not a cache that could go stale.
pub(crate) fn tui_source(
    hearths: Vec<Hearth>,
) -> impl FnMut(tui::Ask) -> Result<tui::Refreshed> + Send + 'static {
    let sole = hearths.len() < 2;
    let mut kept: Vec<Kept> = hearths.iter().map(|_| Kept::default()).collect();
    let mut seen = None;
    move |ask| {
        let tab = ask.tab.min(hearths.len().saturating_sub(1));
        read_views(
            &hearths[tab].journal,
            sole,
            ask,
            &mut kept[tab],
            &mut seen,
            crate::now_rfc3339,
        )
    }
}

#[cfg(test)]
mod read_tests;
