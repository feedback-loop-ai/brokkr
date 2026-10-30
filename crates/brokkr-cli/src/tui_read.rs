//! The console's reads: the one refresh source `brokkr tui` and every run
//! view run on, the only place a store is opened on that path. It stands
//! beside `tui.rs` rather than under it, because the console's own
//! modules name no store at all (the TUI's source-level proofs hold that).
//! #744 moved it here from `lib.rs` and made the fleet a kept read.

use std::path::Path;

use anyhow::Result;
use brokkr_runtime::realms::Hearth;
use brokkr_store::{Store, StoreError};
use brokkr_view::transcript::{LegacyProvenance, TranscriptRead};
use brokkr_view::{Cursor, Paged, RunsView, Section};

use crate::fleet::{Ask, Journal, Pages, AHEAD_ROWS, PAGE_ROWS};
use crate::run_pages::{self, Console, Shown};
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
/// the run it last read and what it read of that run, and the fleet's kept rows
/// (#744). Nothing here is persisted; it lives as long as the console's
/// source does.
#[derive(Default)]
pub(crate) struct Kept {
    head: Option<(u64, String)>,
    run: Option<Selected>,
    fleet: Pages,
    /// The window of the fleet the last frame read (decision 0089 ruling
    /// 6): a list that scrolled or filtered is read again.
    window: Option<tui::Window>,
    /// How many checkpoint rows the last frame's screen showed, so how
    /// many pages it asked for: a scroll that shows more is read again
    /// (decision 0089 slice 3).
    rows: usize,
    /// The scroll's own row, which the participant level's window starts
    /// at (R2, F2, F3), or the end key's request where it asked for the
    /// stream's absolute end (M3).
    offset: usize,
    /// The stream's rank the end key asked for, resolved into `offset`
    /// once the read reaches the end (M3).
    end: Option<usize>,
    /// Whether the run's page the last frame showed has a next one.
    more: bool,
    /// How many rows of the screen's order stand before the page the run
    /// keeps (decision 0089 ruling 9).
    start: usize,
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
    // opened again with the fleet.
    let replaced = kept.fleet.replaced(before);
    let moved = current != kept.head || replaced;
    let scrolled = kept.window.as_ref() != Some(&ask.window)
        || kept.rows != ask.live_rows
        || kept.offset != ask.live_offset;
    if !(ask.force || ask.fleet || moved || transcript_changed || scrolled) {
        // Nothing has moved: the console keeps the frame it has, and
        // nothing is re-folded at four polls a second.
        return Ok(None);
    }
    kept.head = current;
    kept.rows = ask.live_rows;
    kept.offset = ask.live_offset;
    kept.end = None;
    // Quarantined per run: a console would otherwise lose the operator's
    // whole fleet table because one old run is corrupt. The absence mark
    // is `RunRow.status_known`'s job (0001), and the refusal's own words
    // ride along as the row's detail. A page at a time (decision 0089):
    // the window the list asks for, the selected run's values whole.
    let now = clock();
    let reopen = ask.force || replaced;
    let (listed, run) = listed_and_opened(&store, kept, (&now, before), &ask, reopen)
        .map_err(|error| anyhow::Error::msg(error.to_string()))?;
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
        runs: listed.runs,
        run,
        // The selected participant's own bounded local read, produced by
        // the shared reader the command and the browser also consume.
        transcript,
        // A journal that was read has nothing to say about itself.
        note: None,
        unread: listed.unread,
        window: ask.window,
        ahead: listed.ahead,
        more: kept.more,
        start: kept.start,
        end: kept.end,
    }))
}

/// The fleet's window the list asks for, and the run `ask` opens read as
/// [`run_of`] reads it: its view, or none and the store's refusal of it
/// in the fleet's rows.
fn listed_and_opened(
    store: &Store,
    kept: &mut Kept,
    (now, before): (&str, Option<Journal>),
    ask: &tui::Ask,
    reopen: bool,
) -> Result<(Listed, Option<brokkr_view::RunView>), StoreError> {
    let mut listed = fleet_page(store, kept, now, (&ask.window, ask.run), before)?;
    let asked = match (&ask.subject, ask.run_level) {
        (Some(subject), _) => Console::Participant(subject.key.clone()),
        (None, true) => Console::Run,
        (None, false) => Console::Detail,
    };
    let run = match run_of(store, (ask.run, asked), reopen, kept) {
        Ok(run) => run,
        Err((run, refused)) => {
            let row = crate::fleet::refused_row(store, run, refused)?;
            opened_refused(&mut listed.runs, row);
            None
        }
    };
    Ok((listed, run))
}

/// `refused`, the row of the run the console opened and the store refused
/// whole, in the fleet's rows in place of the row its summary gave, or
/// beside them: the opening's refusal reaches the operator, quarantined in
/// the store's own words, whatever the summary's head check passed
/// (decision 0089 ruling 5).
fn opened_refused(runs: &mut RunsView, refused: brokkr_view::RunRow) {
    match runs
        .runs
        .iter_mut()
        .find(|row| row.run_id == refused.run_id)
    {
        Some(row) => *row = refused,
        None => {
            runs.runs.push(refused);
            runs.count = runs.runs.len();
        }
    }
}

/// What a frame lists of the fleet: its window's rows, what they leave
/// unread, and the key past the window's first page, where the page
/// ahead begins.
pub(crate) struct Listed {
    pub(crate) runs: RunsView,
    pub(crate) unread: tui::Unread,
    pub(crate) ahead: Option<Cursor>,
}

/// The fleet's rows the list asks for, the page of `window` and the one
/// ahead, read forward by keyset past its cursor (decision 0089 ruling 6),
/// the rows its filter keeps and the run `selected` with its values
/// whole, and what they leave unread: the older runs past them, counted
/// by the summary's index, and the runs behind their summary past those
/// this refresh reads whole. However far the list has scrolled, a frame
/// reads and holds two pages.
pub(crate) fn fleet_page(
    store: &Store,
    kept: &mut Kept,
    now: &str,
    (window, selected): (&tui::Window, Option<&str>),
    before: Option<Journal>,
) -> Result<Listed, StoreError> {
    let keep = |row: &brokkr_view::RunRow| tui::keeps(&window.filter, row);
    fleet_page_with(store, kept, (now, before), (window, selected), &keep)
}

/// [`fleet_page`], keeping the rows `keep` keeps. Its refresh, its window
/// and its count of the older runs are one read of the journal, so a
/// writer that moves a run from one section to another between the
/// sections' reads moves none of the window's rows: each run lists once
/// (decision 0089 ruling 6).
fn fleet_page_with(
    store: &Store,
    kept: &mut Kept,
    (now, before): (&str, Option<Journal>),
    (window, selected): (&tui::Window, Option<&str>),
    keep: &dyn Fn(&brokkr_view::RunRow) -> bool,
) -> Result<Listed, StoreError> {
    let ((page, ahead), older) = store.as_one_read(|| {
        kept.read = kept.fleet.refresh(store, before, AHEAD_ROWS)?;
        let ask = Ask {
            now,
            whole: &|run| Some(run) == selected,
            keep,
        };
        let asked = (AHEAD_ROWS, PAGE_ROWS);
        let window = kept
            .fleet
            .window(store, ask, window.after.as_ref(), asked)?;
        Ok::<_, StoreError>((window, kept.fleet.older(store, now)?))
    })?;
    let paged_older = page
        .rows
        .iter()
        .filter(|(section, _)| *section == Section::Older)
        .count();
    let unread = tui::Unread {
        older: older.saturating_sub(paged_older),
        behind: kept.fleet.unread(),
    };
    let mut runs = RunsView::of(page.rows.into_iter().map(|(_, row)| row).collect());
    runs.page = Some(Paged {
        next: page.next,
        behind: unread.behind,
    });
    kept.window = Some(window.clone());
    Ok(Listed {
        runs,
        unread,
        ahead,
    })
}

/// The selected run as the console last read it: what it read of it
/// through the run's pages, or whole (decision 0089 slice 3).
struct Selected {
    run: String,
    shown: Shown,
}

/// Run `run`'s view as the console shows it `asked`, read through the
/// run's pages at its run level and whole at a participant's
/// ([`run_pages::console`]): derived from what an earlier frame read while
/// the run's head stands and that read paints the screen asked, so the
/// fleet's cadence reads none of the run; read past that head where the
/// head moved, the events landed since and what it shows; and opened
/// again where `reopen` asks it, the fleet read the run whole, or a row
/// below the head it was read at is lost. A run that would not open keeps
/// nothing, so the next frame reads it again, and is the run and the
/// store's refusal of it, which the frame shows (decision 0089 ruling 5).
fn run_of<'a>(
    store: &Store,
    (run, asked): (Option<&'a str>, Console),
    reopen: bool,
    kept: &mut Kept,
) -> Result<Option<brokkr_view::RunView>, (&'a str, StoreError)> {
    let standing = kept.run.take();
    kept.more = false;
    kept.start = 0;
    let Some(run) = run else {
        kept.end = None;
        return Ok(None);
    };
    // An id past what any writer admits names no run, and none is read:
    // the fleet lists it refused in the store's words (decision 0089
    // rulings 5 and 6).
    if brokkr_store::keyed(0, run.to_string()).is_err() {
        kept.end = None;
        return Ok(None);
    }
    // The end a participant read answered is that run's fact: another
    // run starts without it.
    if !standing
        .as_ref()
        .is_some_and(|selected| selected.run == run)
    {
        kept.end = None;
    }
    let reopen = reopen || kept.fleet.was_read_whole(run);
    let standing = standing.filter(|selected| !reopen && selected.run == run);
    let standing = standing.filter(|selected| still_whole(store, selected));
    let standing = standing.map(|selected| selected.shown);
    let screen = asked.clone();
    // The viewport is the frame's: the rows asked for above the scroll's
    // first row. The end key's request reads its own window (M3).
    let visible = match kept.offset {
        usize::MAX => 0,
        offset => kept.rows.saturating_sub(offset),
    };
    let reading = run_pages::Reading {
        rows: kept.rows,
        offset: kept.offset,
        visible,
    };
    let (shown, read) =
        run_pages::console_with(store, run, asked, standing, kept.head.as_ref(), reading)
            .map_err(|refused| (run, refused))?;
    kept.more = shown.more(&screen);
    kept.start = shown.start(&screen);
    // The participant level's whole rank is the read's own fact, carried
    // through an ordinary scroll and advanced by the checkpoints that land
    // (R3, F1); every screen answers it where its read reaches the end.
    kept.end = shown.end(&screen);
    kept.read += read;
    let view = shown.view();
    let run = run.to_string();
    kept.run = Some(Selected { run, shown });
    Ok(Some(view))
}

/// Whether the journal still holds every row of the run `selected` read,
/// through the head it was read at, counted by the key's index and no
/// event read: a row lost below its head, which no head compares, has the
/// run opened again (decision 0089 ruling 5), as #744's kept read counts
/// every run it keeps.
fn still_whole(store: &Store, selected: &Selected) -> bool {
    let head = selected.shown.head();
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
        window: Some(ask.window.clone()),
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
