//! The live read of a tree: this test process's own tree is listed with
//! the memory it holds, and a pid no process holds is not.

use super::trees_now;

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
