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
fn arrived(
    source: &mut dyn FnMut(Ask) -> Result<Refreshed>,
    ask: &dyn Fn() -> Ask<'static>,
) -> Views {
    for _ in 0..1000 {
        if let Some(views) = source(ask()).unwrap() {
            return views;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("no frame arrived in ten seconds");
}

/// What a source was asked, read by read: the run, and whether the read
/// was forced and the fleet's cadence due.
type Asked = Arc<Mutex<Vec<(Option<String>, bool, bool)>>>;

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
    let slow = {
        let answered = Arc::clone(&answered);
        move |_: Ask| {
            let _ = gate.recv_timeout(Duration::from_secs(5));
            answered.store(true, Ordering::SeqCst);
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
    assert!(answered.load(Ordering::SeqCst), "the read finished");
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
        assert!(source(ask(false, false)).unwrap().is_none(), "sent");
        // Out: each of these is owed, and none goes to the source yet.
        for (force, fleet) in [(false, false), (false, false), (true, true), (false, false)] {
            assert!(source(ask(force, fleet)).unwrap().is_none());
        }
        open.send(()).unwrap();
        let first = arrived(source, &|| ask(false, false));
        assert_eq!(first.note.as_deref(), Some("a"));
        open.send(()).unwrap();
    });
    assert_eq!(
        *asked.lock().unwrap(),
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
            assert!(source(before()).unwrap().is_none(), "sent");
            arrived(source, &after)
        });
        let after = after();
        assert_eq!(shown.note.as_deref(), after.run, "only the frame asked for");
        let forced: Vec<bool> = asked.lock().unwrap().iter().map(|ask| ask.1).collect();
        // The read for `before`; the forced one that replaced its frame;
        // and the next, sent as that frame was shown.
        assert_eq!(forced, vec![false, true, false]);
    }
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
