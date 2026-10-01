//! The console's models and state: the frame's views, the selected
//! transcript subject, what the shell asks the journal for, and the
//! owned selection that survives a refresh (decision 0014).

use super::*;

// ------------------------------------------------------------- the models

/// One frame's worth of derivation, produced by the injected source and
/// dropped after the frame. **No `brokkr-view` model is retained**, which
/// is why "selection survives a refresh" and "selection clears when its
/// subject vanishes" are the absence of code rather than a diff routine.
pub(crate) struct Views {
    /// The one clock read the derivation refuses to make itself.
    pub now: String,
    pub runs: RunsView,
    pub run: Option<RunView>,
    /// The one selected participant's complete bounded local read. `None`
    /// means no participant is open, never "the file is missing" — a
    /// missing, refused or unowned source is the reader's own result and
    /// stays visible rather than collapsing into an absence.
    pub transcript: Option<TranscriptRead>,
    /// What this frame has to SAY about the hearth it was read from, as
    /// opposed to what it failed to do. A realm whose journal is not
    /// there yet is empty rather than unreadable (decision 0026 ruling
    /// 2), so it arrives as an ordinary frame carrying a sentence — the
    /// status line states it and the keys stay live, where an `Err`
    /// would have counted toward the give-up bound and ended the
    /// console over a realm that has simply not run yet.
    pub note: Option<String>,
}

impl Views {
    /// What the first frame draws when the journal is unreadable: an
    /// empty fleet, never an invented one (decision 0001).
    pub(crate) fn empty() -> Views {
        Views {
            now: String::new(),
            runs: RunsView {
                view_version: brokkr_view::VIEW_VERSION,
                runs: Vec::new(),
                count: 0,
            },
            run: None,
            transcript: None,
            note: None,
        }
    }
}

/// What one refresh answers: fresh models, or `None` for "the head has
/// not moved, keep the frame you have".
pub(crate) type Refreshed = Option<Views>;

/// The selected transcript subject: realm/journal identity, the full run
/// id, the participant key and the complete effective reference. The
/// shell re-resolves exactly this participant and no other, so no surface
/// gates the pane on a Claude session id or any other single kind.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Subject {
    /// The active hearth, so a stamp taken in one realm never speaks for
    /// the same run or participant name in another.
    pub tab: usize,
    /// The realm name when the world has tabs; the tab index alone is the
    /// journal identity in a one-hearth world.
    pub realm: Option<String>,
    /// The full run id, not a selector.
    pub run: String,
    /// The exact participant key.
    pub key: String,
    /// The complete recorded common reference, echoed even when it cannot
    /// be validated — a present reference always wins.
    pub reference: Option<brokkr_view::Transcript>,
    /// How an absent common reference may synthesize a legacy Claude one.
    pub provenance: LegacyProvenance,
    /// The compatibility flat id a legacy synthesis may use.
    pub legacy_id: Option<String>,
    /// Whether the participant can still gain prose: the reader watches
    /// the source only while this holds.
    pub working: bool,
}

/// What the shell asks the journal for. Selection reaches a store only
/// through this struct — the TUI itself never holds one.
pub(crate) struct Ask<'a> {
    pub run: Option<&'a str>,
    /// The participant whose transcript this frame is about, or `None`
    /// away from the participant level.
    pub subject: Option<Subject>,
    /// `r`, a level change, or the first frame: rebuild regardless.
    pub force: bool,
    /// The fleet's slower cadence is due.
    pub fleet: bool,
    /// Which hearth is being read (decision 0026 ruling 2). The shell
    /// asks about the ACTIVE tab and no other, so a tab nobody has
    /// visited has had no store opened for it — the laziness is the
    /// absence of a question, not a cache.
    pub tab: usize,
}

// -------------------------------------------------------------- the state

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Level {
    Runs,
    Run,
    Participant,
}

/// One tab's own place in its hearth: what was selected, what was
/// filtered for, and where a paragraph pane was scrolled to. Parked when
/// a tab is left and restored when it is returned to, so a switch
/// neither resets a tab nor bleeds one tab's state into another's
/// (decision 0026 ruling 2).
#[derive(Clone, Default)]
pub(crate) struct TabState {
    pub cursor: Option<String>,
    pub filter: String,
    pub offset: usize,
    /// Whether this hearth's fleet lists its older runs (`a`).
    pub all: bool,
    /// The running runs this hearth's fleet has opened in place (`→`).
    pub expanded: BTreeSet<String>,
}

/// Owned scalars only. Selection is by **stable key** — `RunRow.run_id`,
/// `Phase.name`, `Participant.key`, `JournalRow.seq` — resolved against
/// whatever models the current frame carries.
#[expect(clippy::struct_excessive_bools, reason = "baseline 2026-09, #288")]
pub(crate) struct Tui {
    pub level: Level,
    pub run: Option<String>,
    pub seat: Option<String>,
    pub scope: Option<render::Scope>,
    /// One key per pane slot; cleared when the run changes.
    pub cursor: [Option<String>; 3],
    /// The `Node.key` the graph cursor has walked into, subordinate to
    /// `cursor[0]`. It **scopes**: a node whose key names a participant
    /// makes that seat the scope, one level down from the rail's own
    /// "moving IS scoping" (see [`graph_scope`]); a structural node and
    /// the empty lane both fall back to the rail's phase. `Enter` still
    /// scopes the phase whatever it says. Vanishing is the absence of
    /// code — a stale key matches no drawn node and no participant, so
    /// nothing highlights and nothing stays scoped.
    pub node: Option<String>,
    /// The transcript pane's cursor: the selected turn's INDEX in the
    /// stream, held as a key like every other cursor. Live prose
    /// streaming only APPENDS turns, so an index is as stable a key as
    /// a `seq` — the cursor survives an appending refresh by the same
    /// absence of code as every list.
    pub turn: Option<String>,
    /// Animation is enabled exactly when colour is (§the pulse): no new
    /// flag, no new env var, no new doc surface.
    pub animate: bool,
    /// The viewport of a paragraph pane, which has an offset rather than
    /// a cursor.
    pub offset: usize,
    pub pane: usize,
    pub filter: String,
    pub typing: bool,
    pub help: bool,
    /// A row opened for reading: panes clamp their text to the frame,
    /// so a long one (a feature text, a park reason, an error) would
    /// otherwise be unreadable — truncation with no way through is a
    /// dead end, not evidence.
    pub reading: Option<String>,
    /// True when the open reader is the transcript pane's door, so a
    /// notice-only refresh recomposes it from the shared read instead of
    /// leaving a stale notice set behind.
    pub reading_transcript: bool,
    /// Scroll within the reader, in wrapped lines.
    pub read_offset: usize,
    pub status: Option<String>,
    pub ticks: usize,
    /// Consumed by the shell: `r`, a level change, or the first frame.
    pub force: bool,
    /// The world's hearths, as realm names, when it holds more than one
    /// journal (decision 0026 ruling 2). EMPTY or one-long is a world
    /// that draws no tab bar and behaves exactly as it always did.
    pub tabs: Vec<String>,
    /// Which of them is being read. Always `0` without tabs.
    pub tab: usize,
    /// One parked [`TabState`] per tab, so switching away and back is a
    /// return rather than a reset.
    pub parked: Vec<TabState>,
    /// `a`: the fleet lists its older runs too, where it otherwise folds
    /// them into one count line (#491).
    pub all: bool,
    /// The running runs the fleet has opened in place, by id (#503): `→`,
    /// `l` or Space open the selected one, `←` or `h` fold it.
    pub expanded: BTreeSet<String>,
    /// The width of the frame, measured by the shell before it draws, so
    /// the frame, its footer and the keys pressed against it agree on
    /// whether the fleet's detail pane is on it. Zero until the first
    /// frame.
    pub width: u16,
}

impl Tui {
    /// The console over a world that named no hearths — the shape every
    /// headless test drives, and exactly what [`Tui::over`] reduces to
    /// when the world holds one journal.
    #[cfg(test)]
    pub(crate) fn new(run: Option<String>) -> Tui {
        Tui::over(run, Vec::new(), 0)
    }

    /// The console over a world's hearths, opening on the one that holds
    /// the named run. One hearth (or none named) is the console this file
    /// has always drawn: no bar, no switching, and nothing on the frame
    /// that says a tab exists.
    ///
    /// `--run <id>` opens at the RUN level for that run; `Esc` then walks
    /// the ladder to the full fleet rather than exiting, so the flag
    /// needs no special case anywhere else.
    pub(crate) fn over(run: Option<String>, tabs: Vec<String>, tab: usize) -> Tui {
        let level = match run {
            Some(_) => Level::Run,
            None => Level::Runs,
        };
        let parked = vec![TabState::default(); tabs.len()];
        Tui {
            tab: tab.min(tabs.len().saturating_sub(1)),
            tabs,
            parked,
            level,
            run,
            seat: None,
            scope: None,
            cursor: [None, None, None],
            node: None,
            turn: None,
            animate: false,
            offset: 0,
            pane: 0,
            filter: String::new(),
            typing: false,
            help: false,
            reading: None,
            reading_transcript: false,
            read_offset: 0,
            status: None,
            ticks: 0,
            force: true,
            all: false,
            expanded: BTreeSet::new(),
            width: 0,
        }
    }

    /// The only writer of `run`, and the invariant a run-qualified
    /// selection type would otherwise buy: a new run clears every
    /// selection made under the old one.
    pub(super) fn assign_run(&mut self, id: String) {
        self.run = Some(id);
        self.level = Level::Run;
        self.pane = 0;
        self.scope = None;
        self.seat = None;
        self.filter.clear();
        self.typing = false;
        self.cursor = [None, None, None];
        self.node = None;
        self.turn = None;
        self.offset = 0;
        self.force = true;
    }
}

/// The runs pane has tabs exactly when the world holds more than one
/// journal. One hearth draws no bar and binds no tab key: a world that
/// never drew two journals notices nothing (decision 0026 ruling 2).
pub(super) fn tabbed(tui: &Tui) -> bool {
    tui.tabs.len() > 1
}

/// Move to another hearth. Each tab's selection, filter and scroll are
/// parked on the way out and restored on the way in; everything below
/// the fleet — the open run, the seat, the scope — is NOT carried
/// across, because a run id lives in exactly one journal and the one
/// being left is not the one being entered (ruling 3).
pub(super) fn switch(tui: &mut Tui, index: usize) {
    if !tabbed(tui) || index >= tui.tabs.len() || index == tui.tab {
        return;
    }
    tui.parked[tui.tab] = TabState {
        cursor: tui.cursor[0].clone(),
        filter: tui.filter.clone(),
        offset: tui.offset,
        all: tui.all,
        expanded: std::mem::take(&mut tui.expanded),
    };
    let resumed = tui.parked[index].clone();
    tui.tab = index;
    tui.cursor = [resumed.cursor, None, None];
    tui.filter = resumed.filter;
    tui.offset = resumed.offset;
    tui.all = resumed.all;
    tui.expanded = resumed.expanded;
    tui.level = Level::Runs;
    tui.pane = 0;
    tui.run = None;
    tui.seat = None;
    tui.scope = None;
    tui.node = None;
    tui.turn = None;
    tui.typing = false;
    tui.reading = None;
    tui.reading_transcript = false;
    tui.read_offset = 0;
    // This hearth's journal has not been read yet; the shell asks for it
    // on the next frame, which is when its store is first opened at all.
    tui.force = true;
}

/// Any run in the fleet is running: the gate for the brand mark's
/// pulse, refreshed on the fleet cadence the shell already keeps. A
/// stale run is listed as needing the operator, not as running (#503),
/// so it never makes the mark pulse.
pub(super) fn fleet_live(views: &Views) -> bool {
    views.runs.runs.iter().any(|row| {
        row.status.as_deref() == Some("running") && brokkr_view::need(row, &views.now).is_none()
    })
}

/// The panes `Tab` moves across: the fleet gains its detail pane while
/// one is on the frame.
pub(super) fn panes_at(tui: &Tui, views: &Views) -> usize {
    match tui.level {
        Level::Runs => 1 + usize::from(detail_row(tui, views).is_some()),
        Level::Run => 3,
        Level::Participant => 2,
    }
}

/// The scoped seat, when the scope is a seat at all. `Option` is the
/// exclusivity rule: one scope at a time is one field, not a policy.
pub(super) fn scoped_seat(tui: &Tui) -> Option<&str> {
    match &tui.scope {
        Some(render::Scope::Seat(key)) => Some(key),
        _ => None,
    }
}

pub(super) fn participant<'a>(views: &'a Views, key: &str) -> Option<&'a Participant> {
    views
        .run
        .as_ref()?
        .participants
        .iter()
        .find(|part| part.key == key)
}

/// Resolve the scope against the **fresh** models. `.ok().flatten()` is
/// load-bearing: `lens_for`'s `Err` arm means "this run has no such
/// phase or seat", which for a TUI *is* the vanished-subject case, so
/// one mechanism answers both requirements.
pub(super) fn lens_of(tui: &Tui, views: &Views) -> Option<render::Lens> {
    views
        .run
        .as_ref()
        .and_then(|view| render::lens_for(view, tui.scope.as_ref()).ok().flatten())
}

/// Selection clears itself when its subject disappears — the rule the
/// console already follows. A **filter** never clears a scope: absence
/// from a filtered list is a display fact, and this runs against the
/// unfiltered models.
pub(super) fn settle(tui: &mut Tui, views: &Views) {
    if lens_of(tui, views).is_none() {
        tui.scope = None;
    }
    seed_cursor(tui, views);
    if seat_of(tui, views).is_none() {
        tui.seat = None;
        if tui.level == Level::Participant {
            tui.level = Level::Run;
            tui.pane = 1;
        }
    }
    // A fleet that lists runs and does not list this one has lost it.
    // An empty fleet is the unreadable-journal frame, which is not
    // evidence that the run went away.
    let vanished = match &tui.run {
        Some(run) => {
            !views.runs.runs.is_empty() && !views.runs.runs.iter().any(|r| r.run_id == *run)
        }
        None => false,
    };
    if vanished {
        tui.run = None;
        tui.level = Level::Runs;
        tui.pane = 0;
        tui.scope = None;
        tui.seat = None;
        tui.cursor = [None, None, None];
    }
}

pub(super) fn seat_of<'a>(tui: &Tui, views: &'a Views) -> Option<&'a Participant> {
    tui.seat.as_deref().and_then(|key| participant(views, key))
}

/// Map a participant's provenance to the legacy-synthesis rule. The same
/// rule `transcript_command` applies, kept here so the TUI can build a
/// subject without reaching into the command's private helper: only
/// Claude, LaneTally and an inline seat with no provenance may fall back
/// to a local Claude id.
pub(super) fn legacy_provenance(part: &Participant) -> LegacyProvenance {
    match part
        .provenance
        .as_ref()
        .map(|provenance| provenance.provider.as_str())
    {
        None => LegacyProvenance::Absent,
        Some("claude") => LegacyProvenance::Claude,
        Some("lanetally") => LegacyProvenance::LaneTally,
        Some(_) => LegacyProvenance::Other,
    }
}

/// The subject the shell re-resolves: exactly the selected participant at
/// the participant level, with every fact the shared reader needs. No
/// field asks which kind it is, so a Codex thread or DSH session reaches
/// the same read path as a Claude one.
pub(super) fn subject_of(tui: &Tui, views: &Views) -> Option<Subject> {
    if tui.level != Level::Participant {
        return None;
    }
    let part = seat_of(tui, views)?;
    let run = tui.run.clone()?;
    Some(Subject {
        tab: tui.tab,
        realm: tui.tabs.get(tui.tab).cloned(),
        run,
        key: part.key.clone(),
        reference: part.transcript.clone(),
        provenance: legacy_provenance(part),
        legacy_id: part.session_id.clone(),
        working: part.status == "working",
    })
}

/// Whether a refreshed read replaces the previously displayed turns. A
/// pure append — the new projection begins with every old turn, in order,
/// under the same authority — keeps navigation. Removal, replacement,
/// reorder, shrink, a changed reference or path, and every refusal
/// (ambiguity, ownership loss, format refusal) invalidate the cursor and
/// close an open door before new indices are shown. A notice-only change
/// leaves the turns alone and therefore keeps the cursor.
pub(super) fn transcript_invalidates(
    old: Option<&TranscriptRead>,
    new: Option<&TranscriptRead>,
) -> bool {
    let (Some(old), Some(new)) = (old, new) else {
        // A subject appearing or vanishing is a new list either way.
        return old.is_some() != new.is_some();
    };
    if new.unavailable.is_some() || old.unavailable.is_some() {
        return true;
    }
    // D8's source-identity rule: a same-content replacement (a new inode
    // or changed member provenance) invalidates the cursor and overlay
    // even when every projected field is identical.
    if new.source_identity != old.source_identity {
        return true;
    }
    if new.reference != old.reference || new.path != old.path || new.kind != old.kind {
        return true;
    }
    if new.turns.len() < old.turns.len() {
        return true;
    }
    new.turns[..old.turns.len()] != old.turns
}
