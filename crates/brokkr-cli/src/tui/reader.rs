//! The console's reads, off its input loop (#744). The shell asks its
//! source once a tick and draws whatever it gets, so a source that takes
//! seconds — the first fold of a large journal, a peer holding the write
//! lock — used to hold every key for as long. [`behind`] runs the source
//! on a thread of its own and hands the shell one that never waits: an
//! answer that has arrived, or [`Polled::Pending`], and the keys stay
//! live either way, the first load included.
//!
//! **No ask is lost.** While a read is out, the shell's later asks are
//! folded into the one owed next: the newest selection, still owing any
//! forced rebuild or fleet cadence an earlier ask raised. **No frame is
//! stale.** A frame read for a run, participant or hearth the shell has
//! since left is dropped, and the read owed next is forced, so it
//! rebuilds what the dropped frame would have.

use std::panic::AssertUnwindSafe;
use std::sync::mpsc::{channel, Receiver, Sender};

use super::*;

/// Run `body` with `source` read on a thread of its own, and hand `body`
/// a source that never waits for it. A panic in `source` is carried back
/// and resumed on `body`'s thread, so the console unwinds through its
/// terminal guard exactly as when the source ran on the loop. The thread
/// is never joined: when `body` returns, the console closes at once — a
/// run view's Ctrl+C is raised then — and the read in hand, if any, ends
/// on its own, its answer unread, before the thread does. A read writes
/// nothing (decision 0014), so nothing is left half done.
pub(crate) fn behind<T>(
    mut source: impl FnMut(Ask) -> Result<Refreshed> + Send + 'static,
    body: impl FnOnce(&mut dyn FnMut(Ask) -> Polled) -> T,
) -> T {
    let (asks, asked) = channel::<Asked>();
    let (answers, answered) = channel::<Answer>();
    std::thread::spawn(move || {
        // Until the shell's end is dropped, which ends the asks.
        for ask in asked {
            let answer = std::panic::catch_unwind(AssertUnwindSafe(|| source(ask.ask())));
            // A shell that has closed reads no answer.
            let _ = answers.send((ask, answer));
        }
    });
    let mut reader = Reader {
        asks,
        answered,
        reading: false,
        owed: None,
    };
    body(&mut |ask| reader.ask(&ask))
}

/// A read's answer, beside the ask it answers.
type Answer = (Asked, std::thread::Result<Result<Refreshed>>);

/// An [`Ask`] the reader thread can hold: what the shell lends, owned.
struct Asked {
    run: Option<String>,
    subject: Option<Subject>,
    force: bool,
    fleet: bool,
    tab: usize,
}

impl Asked {
    fn of(ask: &Ask) -> Asked {
        Asked {
            run: ask.run.map(str::to_string),
            subject: ask.subject.clone(),
            force: ask.force,
            fleet: ask.fleet,
            tab: ask.tab,
        }
    }

    fn ask(&self) -> Ask<'_> {
        Ask {
            run: self.run.as_deref(),
            subject: self.subject.clone(),
            force: self.force,
            fleet: self.fleet,
            tab: self.tab,
        }
    }

    /// Whether a frame read for `self` is a frame of what `other` asks
    /// about: the same hearth, run and participant.
    fn frames(&self, other: &Asked) -> bool {
        self.tab == other.tab && self.run == other.run && self.subject == other.subject
    }

    /// `self`, still owing what `earlier` asked and no read has answered.
    fn owing(self, earlier: &Asked) -> Asked {
        Asked {
            force: self.force || earlier.force,
            fleet: self.fleet || earlier.fleet,
            ..self
        }
    }
}

/// The shell's end of the reader thread.
struct Reader {
    asks: Sender<Asked>,
    answered: Receiver<Answer>,
    /// Whether a read is out.
    reading: bool,
    /// What the shell has asked since the last read went out.
    owed: Option<Asked>,
}

impl Reader {
    /// The shell's ask: the answer that has arrived for it, or
    /// [`Polled::Pending`]. The read owed goes out as soon as none is.
    fn ask(&mut self, ask: &Ask) -> Polled {
        let latest = Asked::of(ask);
        let mut owed = match self.owed.take() {
            Some(earlier) => latest.owing(&earlier),
            None => latest,
        };
        // The reader thread lives as long as this end does, so nothing
        // arrived is a read still out.
        let arrived = match self.reading {
            true => self.answered.try_recv().ok(),
            false => None,
        };
        // A frame dropped for a selection the shell has left is no answer
        // to what it asks now: still pending, and the next read forced.
        let mut polled = Polled::Pending;
        if let Some((asked, answer)) = arrived {
            self.reading = false;
            let answer = answer.unwrap_or_else(|panic| std::panic::resume_unwind(panic));
            match asked.frames(&owed) {
                true => polled = answer.into(),
                false => owed.force = true,
            }
        }
        match self.reading {
            true => self.owed = Some(owed),
            false => {
                // The reader thread lives as long as this end does.
                let _ = self.asks.send(owed);
                self.reading = true;
            }
        }
        polled
    }
}
