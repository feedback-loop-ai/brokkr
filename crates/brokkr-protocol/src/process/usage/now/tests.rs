//! The live read of a tree: this test process's own tree is listed with
//! the memory it holds, and a pid no process holds is not.

use std::time::Duration;

use super::super::tests::{entry, sample};
use super::{from_table, trees_now, TreeNow};

#[test]
fn nested_and_repeated_roots_never_credit_the_same_process_twice() {
    let entries = [
        entry(10, 1, Some(sample(100, 10))),
        entry(11, 10, Some(sample(200, 20))),
        entry(12, 11, Some(sample(300, 30))),
        entry(20, 1, Some(sample(400, 40))),
    ];
    assert_eq!(
        from_table(&entries, &[10, 12, 20, 20]),
        vec![
            None,
            Some(TreeNow {
                memory_bytes: 300,
                cpu: Duration::from_millis(30)
            }),
            None,
            None,
        ]
    );
}

/// The test process holds memory and has spent CPU time; `u32::MAX` is
/// past every pid a kernel hands out, and reads as no tree.
#[test]
fn a_listed_root_reads_its_tree_and_an_unlisted_one_reads_none() {
    let read = trees_now(&[std::process::id(), u32::MAX]).expect("the table reads");
    let [own, none] = read.as_slice() else {
        panic!("one answer per root: {read:?}");
    };
    let own = own.expect("this process is listed");
    assert!(own.memory_bytes > 1 << 20, "{own:?}");
    assert_eq!(*none, None);
}
