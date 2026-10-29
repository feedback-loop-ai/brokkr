#[cfg(target_os = "linux")]
use super::*;

/// #918: the tracker reads the engine's own subtree on Linux. A
/// descendant that leaves the attempt's group by `setsid`, whose parent
/// (the subshell that started it) exits at once, is reparented to the
/// driver before the next tick, still in the engine's tree; the tracker
/// records it, and the attempt's end ends it although the group signal
/// cannot reach it. The deadline still settles.
#[cfg(target_os = "linux")]
#[test]
fn a_detached_descendant_whose_parent_exits_between_ticks_is_ended_with_its_attempt() {
    let seats = Seats::new();
    let grandchild_file = seats.file("seat", "grandchild");
    let marker = seats.file("seat", "marker");
    let role = std::env::current_exe().unwrap();
    let driver = command(&format!(
        "GRANDCHILD='{grandchild}' MARKER='{marker}' DIR='{dir}'\n\
         read -r hello\n\
         {accepted}\n\
         ( {ROLE}=detached {ROLE_PID}=\"$GRANDCHILD\" {ROLE_MARKER}=\"$MARKER\" \
           {ROLE_DIR}=\"$DIR\" '{role}' --exact process::tests::role --ignored \
           </dev/null >/dev/null 2>&1 & )\n\
         while [ ! -s \"$GRANDCHILD\" ]; do sleep 0.01; done\n\
         read -r never",
        grandchild = grandchild_file.display(),
        marker = marker.display(),
        dir = seats.dir.display(),
        accepted = accepting(),
        role = role.display(),
    ));
    let process = spawned(&driver, &seats.dir, Some(Duration::from_secs(5)));
    let driver_pid = i32::try_from(process.child.id()).unwrap();
    let ended = std::thread::spawn(move || attempt(process));
    let until = Instant::now() + Duration::from_secs(5);
    let grandchild = loop {
        let text = std::fs::read_to_string(&grandchild_file).unwrap_or_default();
        if let Ok(pid) = text.trim().parse::<i32>() {
            break pid;
        }
        assert!(
            Instant::now() < until,
            "the detached descendant never recorded itself"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    let parent = |pid| {
        let text = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        let mut fields = text.rsplit_once(')')?.1.split_whitespace();
        fields.nth(1)?.parse::<i32>().ok()
    };
    while parent(grandchild) != Some(driver_pid) {
        assert!(
            Instant::now() < until,
            "the detached descendant never came to the driver"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let report = ended.join().unwrap();
    deadline_failure(&report, 5);
    assert!(
        gone(grandchild),
        "the detached descendant outlived the attempt"
    );
}
