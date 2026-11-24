//! A bounded run of a native launcher or of the built doctor, alone in a
//! PID namespace of its own (#1137h, I4): every kill a bounded run
//! delivers, its deadline's or doctor's probe's, is sent inside the
//! namespace and nowhere else. The namespace and its guard are
//! `crate::isolated`'s (#1137i). Linux only, as `crate::pidns` is.

use std::path::Path;

use crate::isolated::{seconds, Isolated, LIMIT};

/// Run `isolated` under a deadline, its stdout in a file so a full pipe
/// cannot be what stops it. `None` is the deadline: the guard killed it
/// there, and the namespace ended with it, so a looping child outlives
/// neither the call nor the suite, and no kill of this test's leaves the
/// namespace. A guard that refused fails the test with its reason; a
/// namespace that never ends fails it too, and `--die-with-parent` ends
/// it as the test's thread does.
pub(super) fn bounded(
    isolated: &mut Isolated,
    cwd: &Path,
    tag: &str,
    limit: std::time::Duration,
) -> Option<(std::process::ExitStatus, String)> {
    let (captured, said) = (
        cwd.join(format!("{tag}.stdout")),
        cwd.join(format!("{tag}.stderr")),
    );
    isolated
        .env(LIMIT, seconds(limit))
        .stdin(std::process::Stdio::null());
    let mut child = (0..50)
        .find_map(|_| {
            isolated.stdout(std::fs::File::create(&captured).unwrap());
            isolated.stderr(std::fs::File::create(&said).unwrap());
            match isolated.spawn() {
                Err(error) if error.raw_os_error() == Some(26) => {
                    std::thread::sleep(std::time::Duration::from_millis(20));
                    None
                }
                spawned => Some(spawned.expect("the bounded child spawns")),
            }
        })
        .expect("the executable stayed busy");
    let deadline = std::time::Instant::now() + limit + std::time::Duration::from_secs(60);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "{tag}: the namespace did not end within a minute of its deadline"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    isolated.judged();
    // The guard answers 137 when the deadline's KILL ended the program;
    // 124 stays a deadline too, as GNU `timeout` answered it before #1332.
    match status.code() {
        Some(124 | 137) => None,
        _ => Some((status, std::fs::read_to_string(&captured).unwrap())),
    }
}
