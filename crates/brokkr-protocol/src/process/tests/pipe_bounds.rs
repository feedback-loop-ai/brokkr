//! The pipe-bound tests: a driver that stops reading cannot hold the
//! engine's shutdown write, stdout is never read past its bound, and a
//! flooding driver meets backpressure. Each plays in an engine of its own.

use super::*;

/// A pipe, and how many bytes it holds before a write blocks, handed back
/// empty. Measured on the pipe itself: the kernel sizes each pipe as it
/// is made, smaller once a user's pipes pass their soft limit.
fn measured_pipe() -> (std::io::PipeReader, std::io::PipeWriter, usize) {
    let (mut reader, mut writer) = std::io::pipe().unwrap();
    let written = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let stop = Arc::new(AtomicBool::new(false));
    let (counter, stopped) = (Arc::clone(&written), Arc::clone(&stop));
    let filler = std::thread::spawn(move || {
        while !stopped.load(Ordering::SeqCst) {
            writer.write_all(b"x").unwrap();
            counter.fetch_add(1, Ordering::SeqCst);
        }
        writer
    });
    let mut capacity = usize::MAX;
    while written.load(Ordering::SeqCst) != capacity {
        capacity = written.load(Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(200));
    }
    // One read frees the byte the filler is blocked on; it then stops.
    stop.store(true, Ordering::SeqCst);
    let mut chunk = vec![0; capacity];
    let mut drained = reader.read(&mut chunk).unwrap();
    let writer = filler.join().unwrap();
    while drained < written.load(Ordering::SeqCst) {
        drained += reader.read(&mut chunk).unwrap();
    }
    (reader, writer, capacity)
}

/// #403: a driver that answered without draining its stdin cannot hold
/// the report on the `shutdown` write. The start fills the pipe so the
/// write blocks, and the report still returns within the grace.
#[test]
fn a_driver_that_stops_reading_cannot_hold_the_shutdown_write() {
    own_engine!("pipe_bounds::a_driver_that_stops_reading_cannot_hold_the_shutdown_write");
    let line = |body: Body| wire(body).len() + 1;
    let start = |pad: &str| {
        line(Body::Start {
            probe: None,
            effect_id: "effect".into(),
            attempt_id: "attempt".into(),
            seat: "seat".into(),
            input: json!({"pad": pad}),
        })
    };
    // The driver's stdin is a pipe measured here. The greeting is read
    // off it, and then nothing: the start leaves half a shutdown line of
    // room.
    let (reader, writer, capacity) = measured_pipe();
    let pad = capacity - start("") - line(Body::Shutdown) / 2;
    let dir = tempfile::tempdir().unwrap();
    let greeted = dir.path().join("greeted");
    // The read end closes once the test is done, or after ten seconds, so
    // a write left blocking on it fails the test rather than hanging it.
    let (done, closing) = mpsc::channel::<()>();
    let reading = {
        let greeted = greeted.clone();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(reader);
            reader.read_line(&mut String::new()).unwrap();
            std::fs::write(greeted, "").unwrap();
            let _ = closing.recv_timeout(Duration::from_secs(10));
        })
    };
    let driver = command(&format!(
        "while [ ! -e '{}' ]; do sleep 0.01; done; printf '%s\\n' '{}' '{}' '{}'; sleep 10",
        greeted.display(),
        capabilities(),
        accepted(),
        succeeded()
    ));
    let mut process = spawned(&driver, std::path::Path::new("."), None);
    process.stdin = Box::new(writer);
    process.bounds.grace = Duration::from_millis(500);
    let group = i32::try_from(process.child.id()).unwrap();
    let started = Instant::now();
    let report = process.run_attempt(
        "test",
        "effect",
        "attempt",
        "seat",
        json!({"pad": "x".repeat(pad)}),
        |_| {},
    );
    let elapsed = started.elapsed();
    assert!(
        matches!(report.outcome, AttemptOutcome::Succeeded { .. }),
        "{:?} after {elapsed:?}",
        report.outcome
    );
    assert!(elapsed < Duration::from_secs(3), "took {elapsed:?}");
    assert_eq!(report.cleanup, Cleanup::Settled);
    assert!(gone(group));
    // Closing the read end releases the blocked write's thread.
    drop(done);
    reading.join().unwrap();
}

/// #403: once the stdout bound has passed, nothing more is read, however
/// much a writer outside the tree keeps ready: the next read gives up and
/// the drain reports the pipe held without consuming what is ready.
#[test]
fn stdout_is_not_read_past_its_bound_whatever_is_ready() {
    own_engine!("pipe_bounds::stdout_is_not_read_past_its_bound_whatever_is_ready");
    let mut process = spawned(
        &command("read -r hello"),
        std::path::Path::new("."),
        Some(Duration::from_secs(1)),
    );
    let (lines, stdout) = queue::bounded(8, Limits::DEFAULT.stdout_bytes);
    process.stdout = stdout;
    for _ in 0..2 {
        lines.send(Stdout::Line("x\n".into()), 1).unwrap();
    }
    process.started = Instant::now().checked_sub(Duration::from_secs(2)).unwrap();
    process.bounds.drain = Duration::ZERO;
    assert!(
        process.next_stdout().is_none(),
        "a ready line was read past the bound"
    );
    assert_eq!(
        process.stdout_closed(Instant::now()),
        Err(Unsettled::Stdout)
    );
    assert!(
        matches!(
            process.stdout.recv_timeout(Duration::ZERO),
            Ok(Stdout::Line(_))
        ),
        "the drain read a ready line past its bound"
    );
    // Something outside the tree holding stdout open parks the attempt
    // within the drain bound.
    process.bounds.drain = Duration::from_millis(100);
    let started = Instant::now();
    let report = attempt(process);
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "took {:?}",
        started.elapsed()
    );
    assert_eq!(unresolved(&report), Some(&Unsettled::Stdout));
    drop(lines);
}

/// #403: a flooding driver meets backpressure: its reader holds at most
/// `STDOUT_LINES` unread, and once the engine stops listening, it reads
/// nothing more.
#[test]
fn a_flooding_driver_meets_backpressure() {
    struct Flood(Arc<std::sync::atomic::AtomicUsize>);
    impl Read for Flood {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.0.fetch_add(1, Ordering::SeqCst);
            buf[..2].copy_from_slice(b"x\n");
            Ok(2)
        }
    }
    let reads = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (stdout, _) = read_stdout(Flood(Arc::clone(&reads)), Limits::DEFAULT);
    // The count once it stops moving, or after two seconds of moving.
    let still = || {
        let until = Instant::now() + Duration::from_secs(2);
        let mut last = usize::MAX;
        while reads.load(Ordering::SeqCst) != last && Instant::now() < until {
            last = reads.load(Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(100));
        }
        reads.load(Ordering::SeqCst)
    };
    assert_eq!(still(), STDOUT_LINES + 1);
    drop(stdout);
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(
        still(),
        STDOUT_LINES + 1,
        "the reader went on after the refusal"
    );
}
