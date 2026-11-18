//! A hearth's row read is one snapshot (decision 0089 ruling 8, slice 4's
//! unit U5b2a): a writer commits between any two of the hearth's reads, a
//! lane moving either way, a behind run catching up, a new behind run, a
//! stale seat, and the readout is the journal's as it stood when the read
//! began, each run counted once.

use std::path::{Path, PathBuf};

use brokkr_view::FactUnion;
use rusqlite::Connection;

use super::super::rows::{read_rows_with, HearthSource, Rows, Stage};
use super::*;

/// What a writer does to the journal between two of the hearth's reads.
#[derive(Debug, Clone, Copy)]
enum Write {
    /// A current run's summary row falls behind its head.
    Lags,
    /// Every behind run is caught up, as a writer's open repairs it.
    CatchesUp,
    /// A run an older writer leaves behind its head appears.
    NewBehindRun,
    /// A current run's seat head is lost, so its seat rows are stale.
    StalesASeat,
}

impl Write {
    const EVERY: [Write; 4] = [
        Write::Lags,
        Write::CatchesUp,
        Write::NewBehindRun,
        Write::StalesASeat,
    ];

    /// Commit the write to `journal` from a connection of its own.
    fn commit(self, journal: &Path) {
        match self {
            Write::Lags => execute(
                journal,
                "UPDATE run_summary SET folded_seq = folded_seq - 1 WHERE run_id = 'snap-0'",
            ),
            Write::CatchesUp => execute(
                journal,
                "UPDATE run_summary SET folded_seq = folded_seq + 1 WHERE run_id = 'snap-2'",
            ),
            Write::NewBehindRun => {
                let base = parse_millis(GENERATED_NOW).unwrap();
                let behind = sealed("m-new", &behind_run(base, Some("new-realm")));
                let store = brokkr_store::Store::open(journal).unwrap();
                plant_older_writer(&store, "m-new", &behind);
            }
            Write::StalesASeat => execute(journal, "DELETE FROM seat_head WHERE run_id = 'snap-1'"),
        }
    }

    /// Whether the committed write stands in `journal`, so a test that
    /// passes was not passed by a write that never landed.
    fn landed(self, journal: &Path) -> bool {
        let store = ready(journal);
        let lane_of = |run: &str| lane(&store, run);
        match self {
            Write::Lags => lane_of("snap-0").as_deref() == Some("behind"),
            Write::CatchesUp => lane_of("snap-2").as_deref() == Some("finished"),
            Write::NewBehindRun => lane_of("m-new").as_deref() == Some("behind"),
            Write::StalesASeat => {
                let mut stale = Vec::new();
                store
                    .window_fallbacks(0, 0, None, |run| stale.push(run.run_id.to_string()))
                    .unwrap();
                stale == ["snap-1"]
            }
        }
    }
}

/// Run `sql` against `journal` on a connection of its own.
fn execute(journal: &Path, sql: &str) {
    let writer = Connection::open(journal).unwrap();
    assert_eq!(writer.execute(sql, []).unwrap(), 1, "{sql}");
}

/// Two current stopped runs, `snap-0` and `snap-1`, and two runs behind
/// their heads: `snap-2`, whose row a writer lags and a later one catches
/// up, and `m-behind`, which an older writer left.
fn lagging_journal(dir: &Path) -> PathBuf {
    let base = parse_millis(GENERATED_NOW).unwrap();
    let journal = small_journal(dir, "snap", 3);
    let behind = sealed("m-behind", &behind_run(base, Some("behind-realm")));
    let store = brokkr_store::Store::open(&journal).unwrap();
    plant_older_writer(&store, "m-behind", &behind);
    drop(store);
    execute(
        &journal,
        "UPDATE run_summary SET folded_seq = folded_seq - 1 WHERE run_id = 'snap-2'",
    );
    journal
}

/// `journal`'s Day readout from the row path alone, `between` run at each
/// of the hearth's stages.
pub(super) fn read_between(journal: &Path, mut between: impl FnMut(Stage)) -> Analytics {
    let hearths = [hearth(Vec::new(), journal)];
    let (labels, _) = fleet::names(&hearths);
    let store = ready(journal);
    let query = Query {
        hearths: &[],
        range: Range::Day,
        realm: None,
        now: GENERATED_NOW,
    };
    let (window, _) = brokkr_view::windows(&query).unwrap();
    let refused = std::cell::Cell::new(false);
    let rows = Rows {
        store: &store,
        label: &labels[0],
        configured: &[],
        window,
        realm: None,
        range: Range::Day,
        now: GENERATED_NOW,
        refused: &refused,
    };
    let source = read_rows_with(&rows, &mut between).unwrap();
    let mut union = FactUnion::new(vec![Box::new(HearthSource::Rows(source))]);
    let view = brokkr_view::analytics_with(&mut union, &query).unwrap();
    assert!(!refused.get(), "no window statement refused");
    view
}

/// Whatever a writer commits at any stage of the read, the readout is the
/// journal's as it stood before the write, and the journal then holds the
/// write.
#[test]
fn a_writer_between_the_hearths_reads_changes_nothing_the_readout_counts() {
    for write in Write::EVERY {
        for stage in [Stage::Behind, Stage::Fallbacks, Stage::Rows] {
            let dir = tempfile::tempdir().unwrap();
            let journal = lagging_journal(dir.path());
            let expected = assert_rows_are_whole(&journal, "the journal before the write");
            let mut committed = 0;
            let during = read_between(&journal, |at| {
                if at == stage {
                    write.commit(&journal);
                    committed += 1;
                }
            });
            assert_eq!(committed, 1, "{write:?} at {stage:?}: one commit");
            assert!(write.landed(&journal), "{write:?} at {stage:?}: it landed");
            assert_same(&during, &expected, &format!("{write:?} at {stage:?}"));
        }
    }
}
