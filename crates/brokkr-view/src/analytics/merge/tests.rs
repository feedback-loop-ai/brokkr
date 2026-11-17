//! The merge yields every source's facts in the full key, ties in their
//! sources' order, and pulls no fact a source before the reader needs it.

use std::cell::Cell;

use super::*;

/// Three sources of `(key, source)` facts, each ordered by key, tying
/// with one another on every key they share.
fn sources() -> Vec<Vec<(u32, usize)>> {
    (0..3)
        .map(|source| {
            (0..200)
                .filter(|key| key % (source as u32 + 2) != 1)
                .map(|key| (key, source))
                .collect()
        })
        .collect()
}

/// The merge, its reader pulling one fact at a time and noting, at each,
/// how many facts the sources had handed over beyond those it was given.
#[test]
fn the_merge_is_in_key_order_and_holds_one_head_a_source() {
    let sources = sources();
    let pulled = Cell::new(0usize);
    let mut merged_facts = Vec::new();
    let mut ahead = 0;
    merged(
        &sources,
        &|source: &Vec<(u32, usize)>, read| {
            read(
                &mut source
                    .iter()
                    .copied()
                    .inspect(|_| pulled.set(pulled.get() + 1)),
            );
        },
        &|left: &(u32, usize), right: &(u32, usize)| left.0 > right.0,
        &mut |stream| {
            for fact in stream {
                merged_facts.push(fact);
                ahead = usize::max(ahead, pulled.get() - merged_facts.len());
            }
        },
    );
    let mut expected: Vec<(u32, usize)> = sources.concat();
    // Stable: a tie keeps the sources' order.
    expected.sort_by_key(|fact| fact.0);
    assert_eq!(merged_facts, expected);
    assert!(
        ahead <= sources.len(),
        "the merge held {ahead} facts beyond those it yielded, over {} sources",
        sources.len()
    );
}

/// No source is a stream of nothing; one source is its own stream; a
/// right side that ends first leaves the left's rest to be yielded.
#[test]
fn no_source_and_one_source_are_their_own_streams() {
    let open: Open<'_, Vec<u32>, u32> = &|source, read| read(&mut source.iter().copied());
    let after = &|left: &u32, right: &u32| left > right;
    let merge_of = |sources: &[Vec<u32>]| {
        let mut facts = Vec::new();
        merged(sources, open, after, &mut |stream| facts.extend(stream));
        facts
    };
    assert_eq!(merge_of(&[]), Vec::<u32>::new());
    assert_eq!(
        merge_of(&[vec![3, 1, 2]]),
        [3, 1, 2],
        "one source is yielded as it stands"
    );
    assert_eq!(
        merge_of(&[vec![1, 5, 7], vec![2]]),
        [1, 2, 5, 7],
        "the left's rest after the right ends"
    );
}
