//! Each park class seeks its own band (decision 0089 ruling 7's band, ruling
//! 9; slice 4's unit U5b2a, U5b1's L1): the classes' table pairs class `c`
//! with the start bound `from - 2^c`, so a park of class `c` that starts
//! before it cannot reach the window and is never visited. A pairing that
//! sends class `c` to the next class's start doubles its reach and visits
//! parks the window cannot see.

use super::*;

/// One park of `class` that starts at `at` and ends within its own reach,
/// before any window that starts after it: visited when its class seeks it,
/// rejected by the overlap test when it is.
fn banded(conn: &rusqlite::Connection, seq: i64, class: u8, at: i64) {
    let ended = at + (1i64 << class) - 1;
    plant_park(
        conn,
        "r",
        ParkRow {
            seq,
            phase: "phase",
            at,
            answered: Some(ended),
            ended: Some(ended),
            class: i64::from(class),
        },
    );
}

/// For every closed class the statement can bound (the last saturates at
/// the integers' floor, with no start before it), a park starting just
/// before `from - 2^class` is not visited (it costs at most the one step
/// that ends the neighbouring class's range), and one starting at it is,
/// at 13 steps.
#[test]
fn each_park_class_visits_only_the_parks_within_its_own_reach() {
    let (_dir, store) = store();
    crate::test_support::plant_run(&store.conn, "r", "{}").unwrap();
    let mut seq = 0;
    for class in 1u8..=62 {
        let reach = 1i64 << class;
        let base = store.window_parks_steps(FROM, TO, TO, None).unwrap();
        seq += 1;
        banded(&store.conn, seq, class, FROM - reach - 1);
        let out = store.window_parks_steps(FROM, TO, TO, None).unwrap();
        // The class before it ends its range one row past, on this park:
        // one step to see that it is another class's, and no visit.
        assert!(
            out - base <= 1,
            "class {class}: a park just before its reach is visited ({} steps)",
            out - base
        );
        seq += 1;
        banded(&store.conn, seq, class, FROM - reach);
        let visited = store.window_parks_steps(FROM, TO, TO, None).unwrap() - out;
        assert!(
            visited > 1,
            "class {class}: a park at its reach is visited ({visited} steps)"
        );
    }
}
