//! The console's reads off its input loop (#744): a key redraws while a
//! read is out, no ask is lost while one is, no frame read for what the
//! shell has left is shown, and a read that panics unwinds the shell.

use super::tests::{lines_of, script, test_ops, views, SCRIPT, TERMINAL};
use super::*;
use ratatui::backend::TestBackend;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};

/// An ask about `run` on hearth `tab`, open on participant `key`.
fn asking<'a>(
    tab: usize,
    run: Option<&'a str>,
    key: Option<&str>,
    force: bool,
    fleet: bool,
) -> Ask<'a> {
    Ask {
        run,
        subject: key.map(|key| Subject {
            tab,
            realm: None,
            run: "run-7".to_string(),
            key: key.to_string(),
            reference: None,
            provenance: brokkr_view::transcript::LegacyProvenance::Absent,
            legacy_id: None,
            working: false,
        }),
        force,
        fleet,
        tab,
    }
}

/// The first frame that arrives for `ask`, asking again until one does.
fn arrived(source: &mut dyn FnMut(Ask) -> Polled, ask: &dyn Fn() -> Ask<'static>) -> Views {
    for _ in 0..1000 {
        if let Polled::Answered(answer) = source(ask()) {
            if let Some(views) = (*answer).unwrap() {
                return views;
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("no frame arrived in ten seconds");
}

/// Whether the shell's ask found its read still out.
fn pending(polled: Polled) -> bool {
    matches!(polled, Polled::Pending)
}

/// What a source was asked, read by read: the run, and whether the read
/// was forced and the fleet's cadence due.
type Asked = Arc<Mutex<Vec<(Option<String>, bool, bool)>>>;

/// What the source was asked once it has been asked `reads` times: the
/// reader thread outlives the console, so its last ask may land after.
fn asked_by(asked: &Asked, reads: usize) -> Vec<(Option<String>, bool, bool)> {
    for _ in 0..500 {
        if asked.lock().unwrap().len() >= reads {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    asked.lock().unwrap().clone()
}

/// An ask the shell makes, by name.
type Asking = fn() -> Ask<'static>;

/// A source that answers each read once `gate` lets it, or once the gate
/// is gone, recording what it was asked, and names its frame by the run
/// it read.
fn gated(gate: Receiver<()>, asked: &Asked) -> impl FnMut(Ask) -> Result<Refreshed> + Send {
    let asked = Arc::clone(asked);
    move |ask: Ask| {
        let run = ask.run.map(str::to_string);
        asked
            .lock()
            .unwrap()
            .push((run.clone(), ask.force, ask.fleet));
        let _ = gate.recv();
        Ok(Some(Views {
            note: run,
            ..Views::empty()
        }))
    }
}

/// The acceptance (#744): a source that has not answered its first read
/// — the first fold of a large journal — does not hold the key behind
/// it. The shell drew `?` with the read still out and the fleet still
/// empty. Run on the loop instead (`drive` over the slow source itself),
/// the first ask waits out the read and the source has answered before
/// the key is drawn.
#[test]
fn a_key_redraws_while_the_first_read_is_still_out() {
    let _serialized = TERMINAL.lock().unwrap_or_else(|error| error.into_inner());
    let answered = Arc::new(AtomicBool::new(false));
    let (open, gate) = channel::<()>();
    let (finished, read) = channel::<()>();
    let slow = {
        let answered = Arc::clone(&answered);
        move |_: Ask| {
            let _ = gate.recv_timeout(Duration::from_secs(5));
            answered.store(true, Ordering::SeqCst);
            let _ = finished.send(());
            Ok(Some(views()))
        }
    };
    script(&[Key::Char('?')]);
    behind(slow, |source| {
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut tui = Tui::new(None);
        let code = drive(&mut terminal, &test_ops(), source, &mut tui, 2).unwrap();
        assert!(
            !answered.load(Ordering::SeqCst),
            "the key waited for the read"
        );
        assert_eq!(code, Closed::Quit);
        assert!(tui.help, "the key was applied");
        assert!(SCRIPT.lock().unwrap().is_empty(), "and read");
        let frame = lines_of(terminal.backend().buffer()).join("\n");
        assert!(frame.contains("read-only"), "and drawn: {frame}");
        assert!(!frame.contains("run-7"), "over the empty fleet: {frame}");
        open.send(()).unwrap();
    });
    assert_eq!(
        read.recv_timeout(Duration::from_secs(5)),
        Ok(()),
        "the read finished"
    );
}

/// While a read is out the shell keeps asking, and what those asks owe —
/// a forced rebuild, the fleet's cadence — is carried into the next read
/// whichever ask raised it, never dropped for a later ask that did not.
#[test]
fn asks_made_while_a_read_is_out_are_owed_by_the_next_read() {
    let asked = Arc::new(Mutex::new(Vec::new()));
    let (open, gate) = channel::<()>();
    behind(gated(gate, &asked), |source| {
        let ask = |force, fleet| asking(0, Some("a"), None, force, fleet);
        assert!(pending(source(ask(false, false))), "sent");
        // Out: each of these is owed, and none goes to the source yet.
        for (force, fleet) in [(false, false), (false, false), (true, true), (false, false)] {
            assert!(pending(source(ask(force, fleet))));
        }
        open.send(()).unwrap();
        let first = arrived(source, &|| ask(false, false));
        assert_eq!(first.note.as_deref(), Some("a"));
        open.send(()).unwrap();
    });
    assert_eq!(
        asked_by(&asked, 2),
        vec![
            (Some("a".to_string()), false, false),
            (Some("a".to_string()), true, true)
        ],
        "one read for the first ask, one for all the asks owed after it"
    );
}

/// A frame read for a hearth, run or participant the shell has since
/// left is not shown, and the read that replaces it is forced, so it
/// rebuilds what the dropped frame would have.
#[test]
fn a_frame_read_for_what_the_shell_has_left_is_dropped_and_the_next_read_forced() {
    let left: [(Asking, Asking); 3] = [
        (
            || asking(0, Some("a"), None, false, false),
            || asking(1, Some("a"), None, false, false),
        ),
        (
            || asking(0, Some("a"), None, false, false),
            || asking(0, Some("b"), None, false, false),
        ),
        (
            || asking(0, Some("a"), Some("eff-i"), false, false),
            || asking(0, Some("a"), Some("eff-d"), false, false),
        ),
    ];
    for (before, after) in left {
        let asked = Arc::new(Mutex::new(Vec::new()));
        // No gate: every read answers at once, so the frame for `before`
        // is dropped whenever it lands, after the shell has moved.
        let (_, gate) = channel::<()>();
        let shown = behind(gated(gate, &asked), |source| {
            assert!(pending(source(before())), "sent");
            arrived(source, &after)
        });
        let after = after();
        assert_eq!(shown.note.as_deref(), after.run, "only the frame asked for");
        let forced: Vec<bool> = asked_by(&asked, 3).iter().map(|ask| ask.1).collect();
        // The read for `before`; the forced one that replaced its frame;
        // and the next, sent as that frame was shown.
        assert_eq!(forced, vec![false, true, false]);
    }
}

/// A journal that stays unreadable ends the console after the give-up
/// bound's reads, however many polls each read spans: a read still out
/// counts for nothing, where it used to read as a clean one and reset the
/// count, so a failure slower than a poll never gave up. Counting a
/// pending poll as a clean read (resetting the count when no answer has
/// arrived) runs the loop out instead.
#[test]
fn a_journal_that_stays_unreadable_gives_up_however_slow_each_read_is() {
    let _serialized = TERMINAL.lock().unwrap_or_else(|error| error.into_inner());
    let slow = |_: Ask| -> Result<Refreshed> {
        std::thread::sleep(Duration::from_millis(20));
        anyhow::bail!("database is locked")
    };
    script(&[]);
    let gave_up = behind(slow, |source| {
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        drive(
            &mut terminal,
            &test_ops(),
            source,
            &mut Tui::new(None),
            2_000,
        )
    });
    assert_eq!(
        gave_up.map_err(|error| error.to_string()),
        Err("giving up after 5 unreadable polls: database is locked".to_string())
    );
}

/// The polls a test's shell has made, and whether the first hearth's
/// read has answered.
static POLLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static FIRST_READ: AtomicBool = AtomicBool::new(false);

/// The shell's poll with the first hearth's frame drawn before any key:
/// the first poll waits for that read to answer and reads no key, the
/// second holds the script's key once the frame it answered is drawn,
/// and the rest read none.
fn first_frame_then_key(_: Duration) -> std::io::Result<bool> {
    match POLLS.fetch_add(1, Ordering::SeqCst) {
        0 => {
            for _ in 0..500 {
                if FIRST_READ.load(Ordering::SeqCst) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            // The answer is on its way to the shell once the source has
            // returned it.
            std::thread::sleep(Duration::from_millis(100));
            Ok(false)
        }
        _ => Ok(!SCRIPT.lock().unwrap().is_empty()),
    }
}

/// A frame is never drawn under another hearth's tab: the shell draws
/// the first hearth's fleet, moves to the second, whose read is still
/// out, and draws the empty frame the first load is drawn over rather
/// than the first hearth's runs. Without the clearing, `run-7` of the
/// hearth left stays on the screen under the hearth entered.
#[test]
fn a_hearth_entered_draws_none_of_the_hearth_left_while_its_read_is_out() {
    let _serialized = TERMINAL.lock().unwrap_or_else(|error| error.into_inner());
    POLLS.store(0, Ordering::SeqCst);
    FIRST_READ.store(false, Ordering::SeqCst);
    let (open, gate) = channel::<()>();
    let hearths = move |ask: Ask| -> Result<Refreshed> {
        match ask.tab {
            0 => FIRST_READ.store(true, Ordering::SeqCst),
            _ => drop(gate.recv()),
        }
        Ok(Some(views()))
    };
    script(&[Key::Char('2')]);
    let ops = TerminalOps {
        poll: first_frame_then_key,
        ..test_ops()
    };
    let (drawn, tab) = behind(hearths, |source| {
        let mut terminal = Terminal::new(TestBackend::new(160, 48)).unwrap();
        let tabs = vec!["alpha".to_string(), "beta".to_string()];
        let mut tui = Tui::over(None, tabs, 0);
        drive(&mut terminal, &ops, source, &mut tui, 3).unwrap();
        (lines_of(terminal.backend().buffer()).join("\n"), tui.tab)
    });
    drop(open);
    assert_eq!(POLLS.load(Ordering::SeqCst), 3);
    assert!(SCRIPT.lock().unwrap().is_empty(), "the key was read");
    assert_eq!(tab, 1);
    assert!(drawn.contains("beta"), "under the second tab: {drawn}");
    assert!(!drawn.contains("run-7"), "none of the first: {drawn}");
}

/// The frame the shell draws once `keys` are pressed over seat `eff-i`
/// of `run-7`, entered from its seats pane and its prose drawn, with
/// every read after that seat's own still out; and where the keys left
/// the shell. A source still reading is a source that has not answered,
/// so the slow source is one that stops answering.
fn drawn_while_out(keys: &[Key]) -> (String, Tui) {
    let mut out = false;
    let mut slow = |ask: Ask| -> Polled {
        if out {
            return Polled::Pending;
        }
        let key = ask.subject.map(|subject| subject.key);
        out = key.as_deref() == Some("eff-i");
        let transcript = key.map(|_| super::tests::read_of(super::tests::turns_of(2), false));
        Ok(Some(Views {
            transcript,
            ..views()
        }))
        .into()
    };
    let mut tui = super::tests::at_seats("eff-i");
    script(&[&[Key::Enter, Key::Enter][..], keys].concat());
    let mut terminal = Terminal::new(TestBackend::new(160, 48)).unwrap();
    let iterations = keys.len() + 3;
    drive(&mut terminal, &test_ops(), &mut slow, &mut tui, iterations).unwrap();
    assert!(out, "the seat's own read answered");
    assert!(SCRIPT.lock().unwrap().is_empty(), "every key was read");
    (lines_of(terminal.backend().buffer()).join("\n"), tui)
}

/// While a read is out, the frame drawn is the one the selection was
/// read for or none of it: seat `eff-d` entered draws none of `eff-i`'s
/// prose, and `run-old` entered none of `run-7`'s view — its trail's
/// rows, which no fleet row names — until their own reads answer. Without holding the frame to what it was read for, the seat
/// is drawn over the other's prose and the run under the other's view.
#[test]
fn a_selection_entered_draws_none_of_the_one_left_while_its_read_is_out() {
    let _serialized = TERMINAL.lock().unwrap_or_else(|error| error.into_inner());
    let mut reading = super::tests::at_seats("eff-i");
    for key in [Key::Enter, Key::Enter] {
        apply(&mut reading, &views(), key);
    }
    let prose = Views {
        transcript: Some(super::tests::read_of(super::tests::turns_of(2), false)),
        ..views()
    };
    let shown = super::tests::frame_of(&reading, &prose, 160, 48);
    assert!(shown.contains("prose of turn"), "what eff-i draws: {shown}");
    let shown = super::tests::frame_of(&super::tests::at_run(), &views(), 160, 48);
    assert!(shown.contains("phase/entered"), "what run-7 draws: {shown}");

    // Up to the run, the seat scope cleared, down to the next seat, and in.
    let into = [Key::Escape, Key::Escape, Key::Down, Key::Enter, Key::Enter];
    let (drawn, tui) = drawn_while_out(&into);
    assert_eq!(
        (tui.level, tui.seat.as_deref()),
        (Level::Participant, Some("eff-d"))
    );
    assert!(
        !drawn.contains("prose of turn"),
        "no prose of eff-i: {drawn}"
    );

    // Up to the fleet, down to the next run, in, and onto its seats.
    let left = [Key::Escape, Key::Escape, Key::Escape, Key::Down, Key::Enter];
    let (drawn, tui) = drawn_while_out(&[&left[..], &[Key::Tab, Key::Enter]].concat());
    assert_eq!(
        (tui.level, tui.run.as_deref()),
        (Level::Run, Some("run-old"))
    );
    assert!(
        !drawn.contains("phase/entered"),
        "no view of run-7: {drawn}"
    );
}

/// The console closes the moment its shell does, with a read still out:
/// a run view's Ctrl+C is raised once the console has returned, and a
/// peer holding the write lock must not hold the operator's stop for the
/// store's patience. Joining the reader thread as the console closes
/// waits out the read instead.
#[test]
fn the_console_closes_without_waiting_for_the_read_in_hand() {
    let (open, gate) = channel::<()>();
    let (closed, shut) = channel::<()>();
    let held = move |_: Ask| -> Result<Refreshed> {
        let _ = gate.recv();
        Ok(None)
    };
    let shell = std::thread::spawn(move || {
        behind(held, |source| {
            assert!(pending(source(asking(0, None, None, true, false))));
        });
        closed.send(()).unwrap();
    });
    let waited = shut.recv_timeout(Duration::from_secs(5));
    drop(open);
    shell.join().unwrap();
    assert_eq!(waited, Ok(()), "closed with the read still out");
}

/// A read that panics is not a reader that quietly stopped: the panic
/// reaches the shell's thread with its own payload, where the console's
/// terminal guard unwinds as it would have with the source on the loop.
#[test]
fn a_read_that_panics_unwinds_the_shell_with_its_own_words() {
    let _serialized = TERMINAL.lock().unwrap_or_else(|error| error.into_inner());
    let saved = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let broken = |_: Ask| -> Result<Refreshed> { panic!("the journal reader broke") };
    let unwound = std::panic::catch_unwind(AssertUnwindSafe(|| {
        behind(broken, |source| {
            arrived(source, &|| asking(0, None, None, true, false));
        });
    }));
    std::panic::set_hook(saved);
    let payload = unwound.expect_err("the shell unwound");
    assert_eq!(
        payload.downcast_ref::<&str>(),
        Some(&"the journal reader broke")
    );
}
