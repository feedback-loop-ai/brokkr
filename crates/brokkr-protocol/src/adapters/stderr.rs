//! One child's stderr, captured without blocking the driver: the drain
//! thread, the bounded settle a stopped harness gets, and the complete
//! finish an ended one gets (#430 H1; decision 0098 ruling 4).

use std::io::Read;
use std::sync::{Arc, Condvar, Mutex};

/// How long a stopped harness's stderr reader is given to reach EOF before
/// the caller takes what arrived. A descendant that inherited the pipe can
/// hold it open indefinitely; the stop removed that hang, so the wait is
/// bounded and the reader is left detached (fix round 430s4a3 H1).
const STDERR_SETTLE: std::time::Duration = std::time::Duration::from_millis(100);

struct StderrState {
    bytes: Vec<u8>,
    finished: bool,
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
    reader: Option<std::thread::JoinHandle<()>>,
}

/// Drain a child's stderr on its own thread, so a chatty session cannot
/// deadlock the stdout stream being folded live. The bytes come back
/// raw: known-plaintext masking (decision 0012, layer 5) runs on them
/// before any string conversion, in [`secret::mask_text`].
pub(super) fn drain_stderr(child: &mut std::process::Child) -> StderrCapture {
    let mut pipe = child.stderr.take().expect("piped");
    let buffer = Arc::new(StderrBuffer {
        state: Mutex::new(StderrState {
            bytes: Vec::new(),
            finished: false,
        }),
        finished: Condvar::new(),
    });
    let reader_buffer = Arc::clone(&buffer);
    let reader = std::thread::spawn(move || {
        let mut chunk = [0u8; 8192];
        loop {
            match pipe.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(read) => reader_buffer
                    .state
                    .lock()
                    .expect("stderr buffer")
                    .bytes
                    .extend_from_slice(&chunk[..read]),
            }
        }
        let mut state = reader_buffer.state.lock().expect("stderr buffer");
        state.finished = true;
        drop(state);
        reader_buffer.finished.notify_all();
    });
    StderrCapture {
        buffer,
        reader: Some(reader),
    }
}

impl StderrCapture {
    /// Wait a bounded moment for the reader to reach EOF, then take what it
    /// has. The reader is detached, not joined: a descendant still holding
    /// the inherited pipe must not hold this driver.
    pub(super) fn settle(mut self) -> Vec<u8> {
        let state = self.buffer.state.lock().expect("stderr buffer");
        let (state, _) = self
            .buffer
            .finished
            .wait_timeout_while(state, STDERR_SETTLE, |state| !state.finished)
            .expect("stderr buffer");
        let bytes = state.bytes.clone();
        drop(state);
        drop(self.reader.take());
        bytes
    }

    /// Wait for the reader to reach EOF and take everything it read. Used
    /// where the harness was not stopped, so its pipe is expected to close.
    pub(super) fn finish(mut self) -> Vec<u8> {
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
        self.buffer
            .state
            .lock()
            .expect("stderr buffer")
            .bytes
            .clone()
    }
}
