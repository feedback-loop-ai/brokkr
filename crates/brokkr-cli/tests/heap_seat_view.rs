//! The seat view's heap budgets (decision 0089 ruling 9): a page of 50 of a
//! participant's checkpoints, the run opened and the page read at the head
//! the opening verified, each checkpoint made into its row and dropped,
//! holds the same live heap however many events the journal holds beside
//! the run and however many seats the run holds beside the page's, a small
//! part of what the run holds, and grows with one wide checkpoint on the
//! page, or a wide start of its attempt, by the copies of it the read holds
//! at once, with the wide starts of many attempts on the page by one
//! start's copies, never their sum, with no member's checkpoint, which it
//! reads no body of, and with none of its seat's earlier attempts, its
//! statements the same however many there are. A
//! readout's live heap is two shares, Brokkr's under dhat and SQLite's
//! from its own high-water. The budgets live in `quality/heap-bytes.json`;
//! dhat allows one profiler per process, so this is a binary of its own.

#[path = "heap/budgets.rs"]
mod budgets;
#[path = "heap/grown.rs"]
mod grown;
#[path = "heap/measure.rs"]
mod measure;
#[path = "../../../tests/support/seat_read.rs"]
mod seat_read;
#[path = "heap/stretched.rs"]
mod stretched;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_store::test_support::{statements_begun, stream_plan};
use brokkr_store::Store;
use measure::{peak_of, sqlite_memory};
use stretched::{median_micros, Stretch, SQLITE_SLACK, STRETCH};

/// The checkpoints one page holds (ruling 7).
const PAGE: usize = 50;

/// The grown run's longest-running seat: the participant with the most
/// checkpoints, by its effect and its member, the attempt of its newest
/// checkpoint, that checkpoint's seq, which its first page holds, and how
/// many it holds.
struct Longest {
    effect: String,
    member: Option<String>,
    attempt: String,
    newest: usize,
    count: usize,
}

impl Longest {
    fn of(run: &[EventEnvelope]) -> Longest {
        let mut seats: BTreeMap<(String, Option<String>), Longest> = BTreeMap::new();
        let checkpoints = run
            .iter()
            .filter(|event| event.event_type == EventType::EffectCheckpointed);
        for event in checkpoints {
            let text = |value: &serde_json::Value| value.as_str().map(str::to_string);
            let payload = &event.payload;
            let effect = text(&payload["effect_id"]).expect("a checkpoint names its effect");
            let member = text(&payload["checkpoint"]["member"]);
            let attempt = text(&payload["attempt_id"]).expect("a checkpoint names its attempt");
            let seat = seats.entry((effect.clone(), member.clone()));
            let seat = seat.or_insert(Longest {
                effect,
                member,
                attempt: String::new(),
                newest: 0,
                count: 0,
            });
            (seat.attempt, seat.newest) = (attempt, event.seq as usize);
            seat.count += 1;
        }
        let longest = seats.into_values().max_by_key(|seat| seat.count);
        longest.expect("the grown run checkpoints")
    }

    fn seat(&self) -> seat_read::Seat<'_> {
        (&self.effect, self.member.as_deref())
    }
}

/// What a page cost, from its read-only open to its rows: Brokkr's peak
/// heap under dhat, and SQLite's high-water above where it stood before
/// the open.
#[derive(Debug, Clone, Copy)]
struct Cost {
    brokkr: u64,
    sqlite: u64,
}

/// `seat`'s first page read from `store`: the grown run opened, and its
/// rows counted.
fn first_page(store: &Store, seat: seat_read::Seat<'_>) -> usize {
    let opened = store.open_run(grown::RUN).unwrap();
    let read = seat_read::page(store, grown::RUN, &opened, seat, None).unwrap();
    let (rows, next) = read.expect("the grown run is read at its head");
    assert!(next.is_some(), "the longest seat holds more than a page");
    rows.len()
}

/// What `db`'s first page of `seat` cost, holding that it holds a page.
fn measured(db: &Path, seat: seat_read::Seat<'_>) -> Cost {
    let (base, _) = sqlite_memory(true);
    let mut shown = None;
    let brokkr = peak_of(|| {
        let store = Store::open_read_only(db).unwrap();
        shown = Some(first_page(&store, seat));
    });
    let (_, high) = sqlite_memory(false);
    assert_eq!(shown, Some(PAGE), "the first page");
    Cost {
        brokkr,
        sqlite: u64::try_from(high - base).unwrap(),
    }
}

/// Ruling 9 on the seat view: Brokkr's peak for a page of the grown run's
/// longest-running seat is the same to the byte in a journal of 10,000
/// events and one of 1,000,000, within its budget, ruling 9's 256 KiB and
/// the largest checkpoint on the page, and a tenth of the run's envelopes;
/// SQLite's is within the fleet's cap; and the page is read by the stream
/// index in both journals. With its newest checkpoint stretched to a
/// mebibyte, Brokkr's peak grows over the unstretched page's by no more
/// than the copies of it the read holds at once ([`PLAIN`], [`ESCAPED`]),
/// each within its budget, the plain within ruling 9's 256 KiB and the
/// checkpoint, the escaped past the plain by more than the checkpoint, so
/// the parser's scratch copy is the one measured; and SQLite's grows by no
/// more than the checkpoint and [`SQLITE_SLACK`]. The page's wall time is
/// reported against ruling 9's 50 ms, not gated.
#[test]
fn the_seat_view_s_page_holds_its_checkpoints_one_at_a_time() {
    let dir = tempfile::tempdir().unwrap();
    let run = grown::chained(grown::journal());
    let longest = Longest::of(&run);
    let seat = longest.seat();
    let small_db = stretched::journal(dir.path(), "small.db", 10_000, &run);
    let large_db = stretched::journal(dir.path(), "large.db", 1_000_000, &run);
    let wide = [Stretch::Plain, Stretch::Escaped].map(|stretch| {
        let stretched =
            stretched::run_stretched(grown::journal(), longest.newest, "stretch", stretch);
        stretched::journal(dir.path(), &format!("{stretch:?}.db"), 10_000, &stretched)
    });
    // Each journal's projections caught up by a writable open, as a
    // current writer leaves them.
    for db in [&small_db, &large_db].into_iter().chain(&wide) {
        drop(Store::open(db).unwrap());
    }
    // Read once unmeasured, so what a process allocates once and keeps
    // is allocated before any page is measured.
    measured(&small_db, seat);
    let (small, large) = (measured(&small_db, seat), measured(&large_db, seat));
    let [plain, escaped] = wide.map(|db| measured(&db, seat));
    eprintln!("small: {small:?}; large: {large:?}; plain: {plain:?}; escaped: {escaped:?}");
    let bytes = |event: &EventEnvelope| serde_json::to_string(event).unwrap().len() as u64;
    let run_bytes: u64 = run.iter().map(bytes).sum();
    let on_page = run.iter().filter(|event| {
        event.event_type == EventType::EffectCheckpointed
            && event.payload["effect_id"] == longest.effect.as_str()
            && event.payload["checkpoint"]["member"].as_str() == longest.member.as_deref()
    });
    let mut on_page: Vec<u64> = on_page.map(bytes).collect();
    on_page.reverse();
    on_page.truncate(PAGE);
    let largest = on_page.iter().copied().max().unwrap();
    eprintln!("the run's envelopes: {run_bytes} bytes; the page's largest checkpoint {largest}");
    // The records scripts/measure-budgets.sh reads, each a budget.
    println!("heap\tseat-view\t{}", small.brokkr.max(large.brokkr));
    println!("heap\tseat-view-sqlite\t{}", small.sqlite.max(large.sqlite));
    println!("heap\tseat-view-stretched\t{}", plain.brokkr);
    println!("heap\tseat-view-escaped\t{}", escaped.brokkr);
    let store = Store::open_read_only(&small_db).unwrap();
    let page = median_micros(|| assert_eq!(first_page(&store, seat), PAGE));
    println!("wall\tseat-view-page\t{page}");
    let paged = seat_read::pages(&store, grown::RUN, seat).unwrap();
    let paged: Vec<usize> = paged
        .expect("the grown run is read at its head")
        .iter()
        .map(Vec::len)
        .collect();
    assert_eq!(
        paged.len(),
        longest.count.div_ceil(PAGE),
        "every page of the seat"
    );
    assert_eq!(
        paged.iter().sum::<usize>(),
        longest.count,
        "every checkpoint of the seat"
    );
    let stream = ["SEARCH event_index USING INDEX event_index_stream \
         (run_id=? AND attempt_id=? AND member=? AND seq>? AND seq<?)"];
    for db in [&small_db, &large_db] {
        let store = Store::open_read_only(db).unwrap();
        let plan = stream_plan(&store, grown::RUN, &longest.attempt, seat.1).unwrap();
        assert_eq!(plan, stream, "{db:?}");
    }
    assert_eq!(
        large.brokkr, small.brokkr,
        "1,000,000 events against 10,000"
    );
    let (brokkr, sqlite) = (
        budgets::budget_named("seat-view"),
        budgets::budget_named("seat-view-sqlite"),
    );
    assert!(
        brokkr <= run_bytes / 10,
        "the budget {brokkr} is past a tenth of the run's {run_bytes} bytes"
    );
    assert!(
        brokkr <= 256 * 1024 + largest,
        "the budget {brokkr} is past ruling 9's 256 KiB and the page's largest checkpoint, {largest} bytes"
    );
    assert_eq!(
        sqlite,
        measure::SQLITE_CAP,
        "SQLite's budget is the fleet's cap on every host"
    );
    for (name, cost) in [("10,000", small), ("1,000,000", large)] {
        let within = (cost.brokkr <= brokkr, cost.sqlite <= sqlite);
        let budgets = format!("the budgets are {brokkr} and SQLite's {sqlite}");
        assert_eq!(within, (true, true), "{name}: {cost:?}; {budgets}");
    }
    the_stretched_page_holds(small, plain, escaped);
    the_started_page_holds(dir.path(), &longest, small);
    the_page_holds_none_of_its_seat_s_history(dir.path(), &longest);
    the_page_reads_no_member_s_body(dir.path(), &longest);
    the_page_holds_none_of_its_seat_s_attempts(dir.path(), &longest);
    the_spread_page_holds_one_start_at_a_time(dir.path(), &longest);
}

/// Where the grown run's first event of `longest`'s effect of `event_type`
/// stands: its one start, its request and its success.
fn ours(events: &[EventEnvelope], longest: &Longest, event_type: EventType) -> usize {
    let ours = |event: &EventEnvelope| {
        event.event_type == event_type && event.payload["effect_id"] == longest.effect.as_str()
    };
    events.iter().position(ours).unwrap()
}

/// The grown run with its longest seat's attempt split in `parts`: its
/// start names an attempt of its own, which holds the oldest part of the
/// seat's checkpoints, and the checkpoint that begins each later part is
/// made the start of an attempt of its own, the newest the seat's, which
/// holds the newest part and the first page.
fn split_in(longest: &Longest, parts: usize) -> Vec<EventEnvelope> {
    let mut events = grown::journal();
    let start = ours(&events, longest, EventType::EffectStarted);
    let restart = events[start].clone();
    let length = longest.count / parts;
    for part in 0..parts - 1 {
        let from = start + part * length;
        for event in &mut events[from..from + length] {
            event.payload["attempt_id"] = format!("older-{part}").into();
        }
        let next = from + length;
        let mut started = EventEnvelope {
            seq: events[next].seq,
            ..restart.clone()
        };
        if part + 2 < parts {
            started.payload["attempt_id"] = format!("older-{}", part + 1).into();
        }
        events[next] = started;
    }
    events
}

/// [`split_in`] two, the older attempt's start and another seat's
/// stretched to a mebibyte where `stretch` stretches them.
fn split(longest: &Longest, stretch: Stretch) -> Vec<EventEnvelope> {
    let events = split_in(longest, 2);
    let start = ours(&events, longest, EventType::EffectStarted);
    let other = events.iter().position(|event| {
        event.event_type == EventType::EffectStarted
            && event.payload["effect_id"] != longest.effect.as_str()
    });
    let events = stretched::run_stretched(events, start + 1, "stretch", stretch);
    stretched::run_stretched(events, other.unwrap() + 1, "stretch", stretch)
}

/// `events` renumbered from 1 in their order, as a run's seqs are its
/// positions.
fn renumbered(events: Vec<EventEnvelope>) -> Vec<EventEnvelope> {
    let renumber = |(at, event): (usize, EventEnvelope)| EventEnvelope {
        seq: at as u64 + 1,
        ..event
    };
    events.into_iter().enumerate().map(renumber).collect()
}

/// The grown run with `seats` other effects asked before its longest
/// seat's, each started twice, checkpointed under each attempt and
/// succeeded: seats and attempts the page has nothing of.
fn beside_seats(longest: &Longest, seats: usize) -> Vec<EventEnvelope> {
    let mut events = grown::journal();
    let asked = ours(&events, longest, EventType::EffectRequested);
    let start = ours(&events, longest, EventType::EffectStarted);
    let succeeded = ours(&events, longest, EventType::EffectSucceeded);
    let shape = [asked, start, start + 1, start, start + 1, succeeded].map(|at| &events[at]);
    let mut others = Vec::new();
    for seat in 0..seats {
        let effect = format!("beside-{seat}");
        for (at, event) in shape.iter().enumerate() {
            let mut event = (*event).clone();
            event.payload["effect_id"] = effect.clone().into();
            if at > 0 {
                event.payload["attempt_id"] = format!("{effect}-{}", (at + 1) / 3).into();
            }
            others.push(event);
        }
    }
    events.splice(asked..asked, others);
    renumbered(events)
}

/// What a page after its opening cost: Brokkr's peak heap and SQLite's
/// high-water above where it stood once the run was opened; and what it
/// read, its rows and whether a page is left, none where it was kept.
fn after_opening(db: &Path, seat: seat_read::Seat<'_>) -> (Cost, Option<(usize, bool)>) {
    let store = Store::open_read_only(db).unwrap();
    let opened = store.open_run(grown::RUN).unwrap();
    let mut shown = None;
    let (base, _) = sqlite_memory(true);
    let brokkr = peak_of(|| {
        let read = seat_read::page(&store, grown::RUN, &opened, seat, None).unwrap();
        shown = read.map(|(rows, next)| (rows.len(), next.is_some()));
    });
    let (_, high) = sqlite_memory(false);
    let sqlite = u64::try_from(high - base).unwrap();
    (Cost { brokkr, sqlite }, shown)
}

/// Brokkr's peak for `db`'s first page of `seat` after its opening, read
/// once unmeasured, holding that it holds a page and leaves more.
fn page_after_opening(db: &Path, seat: seat_read::Seat<'_>) -> u64 {
    after_opening(db, seat);
    let (cost, shown) = after_opening(db, seat);
    assert_eq!(shown, Some((PAGE, true)), "the first page");
    cost.brokkr
}

/// `run`, chained, planted under `dir` as `name` in a journal of 10,000
/// events and caught up by a writable open.
fn caught_up(dir: &Path, name: &str, run: Vec<EventEnvelope>) -> PathBuf {
    let db = stretched::journal(dir, name, 10_000, &grown::chained(run));
    drop(Store::open(&db).unwrap());
    db
}

/// After its opening, a page reads the start its own attempt last names
/// and no other, and its own effect's seat rows alone: with a mebibyte
/// start of the seat's older attempt, whose checkpoints are not on the
/// page, and of another seat, and beside 300 other seats each started
/// twice, Brokkr's peak is the page's without them to the byte.
fn the_page_holds_none_of_its_seat_s_history(dir: &Path, longest: &Longest) {
    let [unstretched, stretched] = [Stretch::Unstretched, Stretch::Plain].map(|stretch| {
        let run = split(longest, stretch);
        let db = stretched::journal(dir, &format!("split-{stretch:?}.db"), 10_000, &run);
        drop(Store::open(&db).unwrap());
        page_after_opening(&db, longest.seat())
    });
    eprintln!(
        "after its opening, a page: {unstretched} bytes; beside mebibyte starts: {stretched}"
    );
    assert_eq!(
        stretched, unstretched,
        "a page grew with its seat's older attempt's start and another seat's"
    );
    let grown = caught_up(dir, "grown.db", grown::journal());
    let beside = caught_up(dir, "beside.db", beside_seats(longest, 300));
    let [grown, beside] = [grown, beside].map(|db| page_after_opening(&db, longest.seat()));
    eprintln!("after its opening: {grown} bytes; beside 300 seats {beside}");
    assert_eq!(beside, grown, "a page grew with the run's other seats");
}

/// The grown run with `attempts` small attempts of its longest seat before
/// its own, each started as the seat's attempt is and checkpointed once,
/// as its first checkpoint is: attempts none of whose checkpoints are on
/// the page.
fn after_attempts(longest: &Longest, attempts: usize) -> Vec<EventEnvelope> {
    let mut events = grown::journal();
    let start = ours(&events, longest, EventType::EffectStarted);
    let shape = [&events[start], &events[start + 1]];
    let mut prior = Vec::with_capacity(2 * attempts);
    for attempt in 0..attempts {
        for event in shape {
            let mut event = event.clone();
            event.payload["attempt_id"] = format!("prior-{attempt}").into();
            prior.push(event);
        }
    }
    events.splice(start..start, prior);
    renumbered(events)
}

/// What `db`'s first page of `seat` holds and does after its opening:
/// Brokkr's peak, read once unmeasured, and the statements SQLite began,
/// holding that it holds a page and leaves more.
fn page_and_statements(db: &Path, seat: seat_read::Seat<'_>) -> (u64, usize) {
    let brokkr = page_after_opening(db, seat);
    let mut store = Store::open_read_only(db).unwrap();
    let opened = store.open_run(grown::RUN).unwrap();
    let (read, begun) = statements_begun(&mut store, |store| {
        seat_read::page(store, grown::RUN, &opened, seat, None).unwrap()
    });
    let shown = read.map(|(rows, next)| (rows.len(), next.is_some()));
    assert_eq!(shown, Some((PAGE, true)), "the first page");
    (brokkr, begun)
}

/// A page costs the page, never its seat's history: the same newest 50
/// checkpoints read after 10, 1,000 and 5,000 earlier small attempts of
/// their seat hold the same Brokkr peak to the byte, after the run's
/// opening, and begin the same statements.
fn the_page_holds_none_of_its_seat_s_attempts(dir: &Path, longest: &Longest) {
    let read = [10, 1_000, 5_000].map(|attempts| {
        let run = grown::chained(after_attempts(longest, attempts));
        let db = stretched::journal(dir, &format!("after-{attempts}.db"), 20_000, &run);
        drop(Store::open(&db).unwrap());
        page_and_statements(&db, longest.seat())
    });
    eprintln!("after 10, 1,000 and 5,000 attempts, a page's peak and statements: {read:?}");
    assert_eq!(
        read.map(|(brokkr, _)| brokkr),
        [read[0].0; 3],
        "a page's peak grew with its seat's attempts"
    );
    assert_eq!(
        read.map(|(_, begun)| begun),
        [read[0].1; 3],
        "a page's statements grew with its seat's attempts"
    );
}

/// The grown run with its longest seat's newest `attempts` checkpoints
/// each under an attempt of its own, started just before it, the newest
/// the seat's own attempt started again, every such start stretched as
/// `stretch` stretches the seat's first start.
fn spread(longest: &Longest, attempts: usize, stretch: Stretch) -> Vec<EventEnvelope> {
    let mut events = grown::journal();
    let start = ours(&events, longest, EventType::EffectStarted);
    let succeeded = ours(&events, longest, EventType::EffectSucceeded);
    let restart = stretched::run_stretched(vec![events[start].clone()], 1, "stretch", stretch);
    let from = succeeded - attempts;
    let tail: Vec<EventEnvelope> = events.drain(from..succeeded).collect();
    let mut spread = Vec::with_capacity(2 * attempts);
    for (at, mut checkpoint) in tail.into_iter().enumerate() {
        let mut started = restart[0].clone();
        if at + 1 < attempts {
            let attempt = format!("spread-{at}");
            started.payload["attempt_id"] = attempt.clone().into();
            checkpoint.payload["attempt_id"] = attempt.into();
        }
        spread.extend([started, checkpoint]);
    }
    events.splice(from..from, spread);
    renumbered(events)
}

/// A page holds one start at a time: its 50 checkpoints each of an attempt
/// of its own, each attempt's start a mebibyte, grow its peak after the
/// run's opening over the same page with small starts by no more than the
/// copies of one start the read holds at once ([`PLAIN`], [`ESCAPED`]),
/// never their sum; and SQLite's high-water by no more than one start and
/// [`SQLITE_SLACK`].
fn the_spread_page_holds_one_start_at_a_time(dir: &Path, longest: &Longest) {
    let stretch = STRETCH as u64;
    let [small, plain, escaped] =
        [Stretch::Unstretched, Stretch::Plain, Stretch::Escaped].map(|wide| {
            let run = spread(longest, PAGE + 1, wide);
            let db = caught_up(dir, &format!("spread-{wide:?}.db"), run);
            after_opening(&db, longest.seat());
            let (cost, shown) = after_opening(&db, longest.seat());
            assert_eq!(shown, Some((PAGE, true)), "{wide:?}: the first page");
            cost
        });
    eprintln!("a page across 50 attempts: {small:?}; their starts a mebibyte: plain {plain:?}, escaped {escaped:?}");
    for (name, cost, copies) in [("plain", plain, PLAIN), ("escaped", escaped, ESCAPED)] {
        let growth = cost.brokkr - small.brokkr;
        assert!(
            growth <= copies * stretch,
            "50 mebibyte {name} starts grew the page's peak by {growth} bytes, past {copies} starts"
        );
        let held = cost.sqlite - small.sqlite;
        assert!(
            held <= stretch + SQLITE_SLACK,
            "50 mebibyte {name} starts grew SQLite's high-water by {held} bytes, past one and the slack"
        );
    }
}

/// After its opening, a page of the seat's own reads no member's body: a
/// member's mebibyte checkpoint of the seat's attempt within the page's
/// reach makes it read the kept way, and one past its reach leaves the
/// page read, each with SQLite's high-water short of the checkpoint.
fn the_page_reads_no_member_s_body(dir: &Path, longest: &Longest) {
    let membered = |newer: usize| {
        let mut events = grown::journal();
        let at = longest.newest - newer;
        events[at - 1].payload["checkpoint"]["member"] = "m".into();
        stretched::run_stretched(events, at, "stretch", Stretch::Plain)
    };
    let read = [("within", PAGE / 2), ("past", 2 * PAGE)].map(|(name, newer)| {
        let db = stretched::journal(dir, &format!("member-{name}.db"), 10_000, &membered(newer));
        drop(Store::open(&db).unwrap());
        after_opening(&db, longest.seat());
        (name, after_opening(&db, longest.seat()))
    });
    eprintln!("a member's mebibyte checkpoint within the page's reach and past it: {read:?}");
    let shown = read.map(|(_, (_, shown))| shown);
    assert_eq!(shown, [None, Some((PAGE, true))]);
    for (name, (cost, _)) in read {
        assert!(
            cost.sqlite < STRETCH as u64,
            "{name} the page's reach, SQLite's high-water grew by {} bytes: a member's body was read",
            cost.sqlite
        );
    }
}

/// With the page's newest checkpoint a mebibyte, the page's peak grows
/// over the `unstretched` page's by no more than the copies of it the read
/// holds at once, `plain` within ruling 9's 256 KiB and the checkpoint and
/// `escaped` past it by more than the checkpoint, each within its budget,
/// and SQLite's by no more than the checkpoint and [`SQLITE_SLACK`].
fn the_stretched_page_holds(unstretched: Cost, plain: Cost, escaped: Cost) {
    let stretch = STRETCH as u64;
    assert!(
        plain.brokkr <= 256 * 1024 + stretch,
        "{} bytes with a mebibyte checkpoint, past ruling 9's 256 KiB and the checkpoint",
        plain.brokkr
    );
    assert!(
        escaped.brokkr > plain.brokkr + stretch,
        "the escaped stretch's {} bytes are not a checkpoint past the plain stretch's {}: the scratch copy is not the one measured",
        escaped.brokkr,
        plain.brokkr
    );
    for (name, cost, copies, budget) in [
        ("plain", plain, PLAIN, "seat-view-stretched"),
        ("escaped", escaped, ESCAPED, "seat-view-escaped"),
    ] {
        let growth = cost.brokkr - unstretched.brokkr;
        eprintln!("{name}: the page grew by {growth} bytes");
        assert!(
            growth <= copies * stretch,
            "a mebibyte {name} checkpoint grew the page's peak by {growth} bytes, past {copies} checkpoints"
        );
        let budget = budgets::budget_named(budget);
        assert!(
            cost.brokkr <= budget,
            "{name}: {} bytes with a mebibyte checkpoint; the budget is {budget}",
            cost.brokkr
        );
        let held = cost.sqlite - unstretched.sqlite;
        assert!(
            held <= stretch + SQLITE_SLACK,
            "a mebibyte {name} checkpoint grew SQLite's high-water by {held} bytes, past it and the slack"
        );
    }
}

/// With the start its page's attempt last names a mebibyte, the page's
/// peak, its opening included, grows over the `unstretched` page's by no
/// more than the copies of it the read holds at once, [`PLAIN`] and
/// [`ESCAPED`] as a checkpoint's, each within its budget, and SQLite's by
/// no more than the start and [`SQLITE_SLACK`]; the page alone, after its
/// opening, holds the
/// start as it holds a checkpoint, once where its text is plain.
fn the_started_page_holds(dir: &Path, longest: &Longest, unstretched: Cost) {
    let stretch = STRETCH as u64;
    let wide = [Stretch::Plain, Stretch::Escaped].map(|wide| {
        let events = grown::journal();
        let start = ours(&events, longest, EventType::EffectStarted);
        let run = stretched::run_stretched(events, start + 1, "stretch", wide);
        let db = stretched::journal(dir, &format!("started-{wide:?}.db"), 10_000, &run);
        drop(Store::open(&db).unwrap());
        (
            measured(&db, longest.seat()),
            page_after_opening(&db, longest.seat()),
        )
    });
    let [(plain, plain_page), (escaped, escaped_page)] = wide;
    eprintln!("a mebibyte start: plain {plain:?}, {plain_page} after its opening; escaped {escaped:?}, {escaped_page} after its opening");
    println!("heap\tseat-view-started\t{}", plain.brokkr);
    println!("heap\tseat-view-started-escaped\t{}", escaped.brokkr);
    assert!(
        plain_page <= 256 * 1024 + stretch,
        "after its opening, a page beside a mebibyte start holds {plain_page} bytes, past ruling 9's 256 KiB and the start"
    );
    for (name, cost, copies, budget) in [
        ("plain", plain, PLAIN, "seat-view-started"),
        ("escaped", escaped, ESCAPED, "seat-view-started-escaped"),
    ] {
        let growth = cost.brokkr - unstretched.brokkr;
        eprintln!("{name}: a mebibyte start grew the page by {growth} bytes");
        assert!(
            growth <= copies * stretch,
            "a mebibyte {name} start grew the page's peak by {growth} bytes, past {copies} starts"
        );
        let budget = budgets::budget_named(budget);
        assert!(
            cost.brokkr <= budget,
            "{name}: {} bytes with a mebibyte start; the budget is {budget}",
            cost.brokkr
        );
        let held = cost.sqlite - unstretched.sqlite;
        assert!(
            held <= stretch + SQLITE_SLACK,
            "a mebibyte {name} start grew SQLite's high-water by {held} bytes, past it and the slack"
        );
    }
}

/// The copies of a wide checkpoint the page holds at once, plain: the
/// checkpoint parsed, as the opening holds its envelope.
const PLAIN: u64 = 1;

/// The copies of a wide checkpoint the page holds at once when its text
/// carries escapes: the parser's scratch buffer, which grows past the text,
/// and the string it keeps, as the opening's escaped bound was amended to
/// three envelopes on 2026-10-24. The seat view's row of ruling 9 says one
/// checkpoint body; its amendment to three, in the opening's form, is
/// proposed to the operator (#753).
const ESCAPED: u64 = 3;
