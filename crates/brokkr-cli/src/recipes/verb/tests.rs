use super::*;

fn row(phase: &str, spread: Option<Spread>) -> PhaseUsage {
    PhaseUsage {
        recipe: "self".into(),
        phase: phase.into(),
        attempts: 3,
        unmeasured: 1,
        measured: usize::from(spread.is_some()) * 2,
        peak_memory_bytes: spread,
        mean_memory_bytes: spread,
        peak_load_millicores: spread,
        mean_load_millicores: spread,
        cpu_ms: spread,
        wall_ms: spread,
    }
}

/// The table paints the view's rows in MiB, cores and seconds, and a
/// phase no driver was measured in reads unmeasured, never zero.
#[test]
fn the_table_paints_each_phase_s_spread_and_its_unmeasured_apart() {
    let spread = Spread {
        min: 1_048_576,
        average: 3_145_728,
        max: 5_242_880,
    };
    assert_eq!(
        table(&[row("implement", Some(spread)), row("review", None)], None),
        "self / implement: 3 attempts, 1 unmeasured, 2 drivers measured\n\
         \x20 peak memory  min 1.0 MiB  average 3.0 MiB  max 5.0 MiB\n\
         \x20 mean memory  min 1.0 MiB  average 3.0 MiB  max 5.0 MiB\n\
         \x20 peak load    min 1048.58 cores  average 3145.73 cores  max 5242.88 cores\n\
         \x20 mean load    min 1048.58 cores  average 3145.73 cores  max 5242.88 cores\n\
         \x20 cpu time     min 1048.6 s  average 3145.7 s  max 5242.9 s\n\
         \x20 wall time    min 1048.6 s  average 3145.7 s  max 5242.9 s\n\
         self / review: 3 attempts, 1 unmeasured, 0 drivers measured\n\
         \x20 peak memory  unmeasured\n\
         \x20 mean memory  unmeasured\n\
         \x20 peak load    unmeasured\n\
         \x20 mean load    unmeasured\n\
         \x20 cpu time     unmeasured\n\
         \x20 wall time    unmeasured\n"
    );
}

#[test]
fn no_row_says_whose_attempts_are_not_journaled() {
    assert_eq!(table(&[], None), "no attempt of any recipe is journaled\n");
    assert_eq!(
        table(&[], Some("self")),
        "no attempt of 'self' is journaled\n"
    );
}
