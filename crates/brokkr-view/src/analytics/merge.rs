//! The union's k-way merge (decision 0089 ruling 7): every source's pulled
//! stream of one figure, merged in the figure's full key with one head a
//! source, so a union of hearths holds one fact a hearth while a figure
//! folds, never the window's facts.
//!
//! Every source's stream is open at once, each its own iterator, and the
//! heads stand in one heap: a pull pops the head that stands first and
//! pulls that head's source again, so the call depth does not grow with
//! the sources and a pull costs their logarithm. Two heads that tie on the
//! key stand in their sources' order, and a source's own facts keep its
//! order, since a source has one head in the heap at a time.
//!
//! The merge's one precondition is that each source yields its stream in
//! the key's order. The facts a figure skips need not: a figure that folds
//! only some of a stream's facts reads the merge of the facts it folds in
//! order as long as those stand in order in every source. The stopped-by-rule
//! figure relies on it: the store orders a run by the rule it ended on,
//! while the key reads a hard stop's rule alone, so a run with no hard stop
//! keys lowest wherever the store stands it, and the figure skips it.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// How two facts stand in a figure's full key, under the figure's order.
pub(super) type Order<T, C> = fn(&T, &T, C) -> Ordering;

/// Every source's stream merged by `order` under `context`: two facts that
/// tie keep their sources' order, and a source's own facts keep theirs.
pub(super) struct Merged<'a, T, C> {
    streams: Vec<Box<dyn Iterator<Item = T> + 'a>>,
    heads: BinaryHeap<Head<T, C>>,
    /// The sources to pull before the next fact is yielded.
    refill: Refill,
    order: Order<T, C>,
    context: C,
}

/// The sources a merge pulls before it yields its next fact: every one
/// before the first, then the source of the head last yielded, so no
/// source is pulled before the reader asks for a fact it may hold; none
/// once the merge has ended.
enum Refill {
    Every,
    One(usize),
    Ended,
}

impl<'a, T, C: Copy> Merged<'a, T, C> {
    pub(super) fn new(
        streams: Vec<Box<dyn Iterator<Item = T> + 'a>>,
        order: Order<T, C>,
        context: C,
    ) -> Merged<'a, T, C> {
        Merged {
            heads: BinaryHeap::with_capacity(streams.len()),
            streams,
            refill: Refill::Every,
            order,
            context,
        }
    }

    /// The next fact of `source` into the heap, if it has one: an ended
    /// source puts none back, so it is never pulled again.
    fn pull(&mut self, source: usize) {
        if let Some(fact) = self.streams[source].next() {
            self.heads.push(Head {
                fact,
                source,
                order: self.order,
                context: self.context,
            });
        }
    }
}

impl<T, C: Copy> Iterator for Merged<'_, T, C> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        match std::mem::replace(&mut self.refill, Refill::Ended) {
            Refill::Every => (0..self.streams.len()).for_each(|source| self.pull(source)),
            Refill::One(source) => self.pull(source),
            Refill::Ended => {}
        }
        let head = self.heads.pop()?;
        self.refill = Refill::One(head.source);
        Some(head.fact)
    }
}

/// One source's next fact in the heap, with the order it stands by.
struct Head<T, C> {
    fact: T,
    source: usize,
    order: Order<T, C>,
    context: C,
}

impl<T, C: Copy> Ord for Head<T, C> {
    /// The heap yields its greatest head first, so the greatest is the one
    /// that stands first in the key, and on a tie the one of the earlier
    /// source. No two heads share a source, so no two are equal.
    fn cmp(&self, other: &Self) -> Ordering {
        (self.order)(&other.fact, &self.fact, self.context).then(other.source.cmp(&self.source))
    }
}

impl<T, C: Copy> PartialOrd for Head<T, C> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T, C: Copy> PartialEq for Head<T, C> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl<T, C: Copy> Eq for Head<T, C> {}

#[cfg(test)]
mod tests;
