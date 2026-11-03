//! One child's stderr, captured without blocking the driver: the drain
//! thread, the bounded settle a stopped harness gets, and the complete
//! finish an ended one gets (#430 H1; decision 0098 ruling 4).
//!
//! Only EOF is a complete capture. An interrupted read is retried, and any
//! other read error ends the capture UNREADABLE with the error it met, so
//! no caller can mistake a failed read for the end of the child's report
//! (fix round 430s4a4 H1; decision 0071 rulings 3 and 9).

use std::io::{ErrorKind, Read};
use std::sync::{Arc, Condvar, Mutex};

/// How long a stopped harness's stderr reader is given to reach EOF before
/// the caller takes what arrived. A descendant that inherited the pipe can
/// hold it open indefinitely; the stop removed that hang, so the wait is
/// bounded and the reader is left detached (fix round 430s4a3 H1).
const STDERR_SETTLE: std::time::Duration = std::time::Duration::from_millis(100);

/// A read failure that ended a capture. It keeps the kind and the text the
/// operating system gave, so the refusal that names it carries the error
/// rather than a fresh string made at the call site (#430 H1; decision 0071
/// ruling 8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Unreadable {
    kind: std::io::ErrorKind,
    detail: String,
}

impl Unreadable {
    fn of(error: std::io::Error) -> Self {
        Self {
            kind: error.kind(),
            detail: error.to_string(),
        }
    }
}

impl std::fmt::Display for Unreadable {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.kind, self.detail)
    }
}

/// How one capture ended. Only EOF is complete; a read failure is named so
/// a startup classification never reads it as a clean end (#430 H1).
#[derive(Debug)]
pub(crate) enum StderrEnd {
    /// The pipe reached EOF: everything the child wrote was captured.
    Eof,
    /// The reader failed. The bytes before the failure are kept, but the
    /// capture is not complete.
    Unreadable(Unreadable),
    /// The reader had not ended when the bounded settle elapsed, or it
    /// ended without reporting. The bytes are what arrived; a descendant
    /// holding the inherited pipe is the measured cause (430s4a3 H1).
    Incomplete,
}

/// One capture's bytes and how its reader ended.
#[derive(Debug)]
pub(crate) struct CapturedStderr {
    bytes: Vec<u8>,
    end: StderrEnd,
}

impl CapturedStderr {
    /// The raw bytes the reader kept, before masking or redaction.
    pub(super) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// How the capture ended, matched by every consumer so a new way of
    /// ending needs an explicit disposition (#430 H1; decision 0071 ruling
    /// 2).
    pub(super) fn end(&self) -> &StderrEnd {
        &self.end
    }

    /// The stderr evidence one invocation records: the captured bytes as
    /// the caller masks them, then, when the reader failed, a line naming
    /// that failure. A failed capture never reads as a clean end of the
    /// child's report (#430 H1; decision 0071 rulings 3 and 9).
    pub(super) fn evidence(&self, mask: impl FnOnce(&[u8]) -> String) -> String {
        let mut text = mask(&self.bytes);
        match &self.end {
            StderrEnd::Unreadable(error) => {
                if !text.is_empty() && !text.ends_with('\n') {
                    text.push('\n');
                }
                text.push_str(&format!("stderr capture ended unreadable: {error}\n"));
            }
            // A complete or bounded capture needs no note.
            StderrEnd::Eof | StderrEnd::Incomplete => {}
        }
        text
    }
}

struct StderrState {
    bytes: Vec<u8>,
    finished: bool,
    failure: Option<std::io::Error>,
}

impl StderrState {
    /// The named way this capture ended. `finished` alone is EOF: a reader
    /// that errored records its failure first (#430 H1).
    fn end(&mut self) -> StderrEnd {
        match self.failure.take() {
            Some(error) => StderrEnd::Unreadable(Unreadable::of(error)),
            None if self.finished => StderrEnd::Eof,
            None => StderrEnd::Incomplete,
        }
    }
}

struct StderrBuffer {
    state: Mutex<StderrState>,
    finished: Condvar,
}

/// One child's stderr, drained on its own thread into a buffer the caller
/// can read while the child still runs. A harness stopped at a probe's
/// answer must keep what it wrote — dsh's startup report is classified from
/// these RAW bytes before the gate (decision 0098 ruling 4) — but waiting
/// for EOF would reintroduce the hang the stop removes when a descendant
/// holds the inherited pipe (fix round 430s4a3 H1).
pub(super) struct StderrCapture {
    buffer: Arc<StderrBuffer>,
    reader: std::thread::JoinHandle<()>,
}

/// Drain a child's stderr on its own thread, so a chatty session cannot
/// deadlock the stdout stream being folded live. The bytes come back
/// raw: known-plaintext masking (decision 0012, layer 5) runs on them
/// before any string conversion, in [`secret::mask_text`].
pub(super) fn drain_stderr(child: &mut std::process::Child) -> StderrCapture {
    capture(child.stderr.take().expect("piped"))
}

/// The one capture loop, over any reader. A test drives it through a seam
/// that fails, so the read dispositions below are proven without a real
/// pipe (#430 H1).
fn capture(pipe: impl Read + Send + 'static) -> StderrCapture {
    let buffer = Arc::new(StderrBuffer {
        state: Mutex::new(StderrState {
            bytes: Vec::new(),
            finished: false,
            failure: None,
        }),
        finished: Condvar::new(),
    });
    let reader_buffer = Arc::clone(&buffer);
    let reader = std::thread::spawn(move || drain_reader(pipe, &reader_buffer));
    StderrCapture { buffer, reader }
}

/// A [`Read`] seam around the capture loop, test-only, so a read failure
/// can be met without a pipe that fails.
#[cfg(test)]
pub(super) fn capture_reader(reader: impl Read + Send + 'static) -> StderrCapture {
    capture(reader)
}

/// Read until EOF, retrying an interrupted read as `read_to_end` did and
/// ending the capture UNREADABLE on any other error. Only EOF sets
/// `finished` without a failure (#430 H1).
fn drain_reader(mut pipe: impl Read, buffer: &StderrBuffer) {
    let mut chunk = [0u8; 8192];
    let failure = loop {
        match pipe.read(&mut chunk) {
            Ok(0) => break None,
            Ok(read) => buffer
                .state
                .lock()
                .expect("stderr buffer")
                .bytes
                .extend_from_slice(&chunk[..read]),
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(error) => break Some(error),
        }
    };
    let mut state = buffer.state.lock().expect("stderr buffer");
    state.finished = true;
    state.failure = failure;
    drop(state);
    buffer.finished.notify_all();
}

impl StderrCapture {
    /// Take the stderr one invocation keeps: bounded when the probe stopped
    /// its harness, complete when the harness ended on its own. The one
    /// home of the stopped/ended disposition (#430 L3; decision 0071
    /// ruling 5).
    pub(super) fn take(self, stopped: bool) -> CapturedStderr {
        if stopped {
            self.settle()
        } else {
            self.finish()
        }
    }

    /// Wait a bounded moment for the reader to reach EOF, then take what it
    /// has. The reader is detached, not joined: a descendant still holding
    /// the inherited pipe must not hold this driver. A reader that has not
    /// ended is named Incomplete, never complete.
    fn settle(self) -> CapturedStderr {
        let state = self.buffer.state.lock().expect("stderr buffer");
        let (mut state, _) = self
            .buffer
            .finished
            .wait_timeout_while(state, STDERR_SETTLE, |state| !state.finished)
            .expect("stderr buffer");
        let bytes = state.bytes.clone();
        let end = state.end();
        drop(state);
        drop(self.reader);
        CapturedStderr { bytes, end }
    }

    /// Wait for the reader to end and take everything it read. Used where
    /// the harness was not stopped, so its pipe is expected to close.
    fn finish(self) -> CapturedStderr {
        let _ = self.reader.join();
        let mut state = self.buffer.state.lock().expect("stderr buffer");
        let bytes = state.bytes.clone();
        let end = state.end();
        CapturedStderr { bytes, end }
    }
}
