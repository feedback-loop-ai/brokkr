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

/// Two facts by their key alone, so ties fall to the merge.
fn by_key(left: &(u32, usize), right: &(u32, usize), (): ()) -> Ordering {
    left.0.cmp(&right.0)
}

/// The merge of `sources`, each a stream of its facts in turn.
fn merge_of<'a, T: Copy + 'a>(sources: &'a [Vec<T>], order: Order<T, ()>) -> Merged<'a, T, ()> {
    let streams = sources
        .iter()
        .map(|source| Box::new(source.iter().copied()) as Box<dyn Iterator<Item = T>>)
        .collect();
    Merged::new(streams, order, ())
}

/// The merge, its reader pulling one fact at a time and noting, at each,
/// how many facts the sources had handed over beyond those it was given.
#[test]
fn the_merge_is_in_key_order_and_holds_one_head_a_source() {
    let sources = sources();
    let pulled = Cell::new(0usize);
    let streams = sources
        .iter()
        .map(|source| {
            Box::new(
                source
                    .iter()
                    .copied()
                    .inspect(|_| pulled.set(pulled.get() + 1)),
            ) as Box<dyn Iterator<Item = (u32, usize)>>
        })
        .collect();
    let mut merged_facts = Vec::new();
    let mut ahead = 0;
    for fact in Merged::new(streams, by_key, ()) {
        merged_facts.push(fact);
        ahead = usize::max(ahead, pulled.get() - merged_facts.len());
    }
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
/// source that ends first leaves the other's rest to be yielded.
#[test]
fn no_source_and_one_source_are_their_own_streams() {
    let by_value = |left: &u32, right: &u32, (): ()| left.cmp(right);
    let merged = |sources: &[Vec<u32>]| merge_of(sources, by_value).collect::<Vec<_>>();
    assert_eq!(merged(&[]), Vec::<u32>::new());
    assert_eq!(
        merged(&[vec![3, 1, 2]]),
        [3, 1, 2],
        "one source is yielded as it stands"
    );
    assert_eq!(
        merged(&[vec![1, 5, 7], vec![2]]),
        [1, 2, 5, 7],
        "the left's rest after the right ends"
    );
}

/// An ended source is never pulled again, even once the merge has ended
/// and is asked again: a stream that would yield after its first `None`
/// yields nothing more through the merge.
#[test]
fn an_ended_source_is_never_pulled_again() {
    let mut calls = 0;
    let unfused = std::iter::from_fn(move || {
        calls += 1;
        (calls != 2).then_some((calls, 1usize))
    });
    let streams: Vec<Box<dyn Iterator<Item = (u32, usize)>>> =
        vec![Box::new([(0, 0)].into_iter()), Box::new(unfused)];
    let mut merged = Merged::new(streams, by_key, ());
    let pulled: Vec<_> = (0..4).map(|_| merged.next()).collect();
    assert_eq!(pulled, [Some((0, 0)), Some((1, 1)), None, None]);
}
