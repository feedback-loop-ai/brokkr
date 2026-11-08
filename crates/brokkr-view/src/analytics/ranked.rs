//! The at most [`GROUPS`] groups a figure keeps, and the count of the rest
//! (decision 0089 ruling 7): a grouped figure holds the groups its own
//! order ranks first, each by its already-cut key, and prints the rest as
//! one `N more` count (ruling A3's additive `*_more` field).

use std::collections::BTreeMap;

/// The groups one figure keeps.
pub(crate) const GROUPS: usize = 64;

/// A bounded selection: the [`GROUPS`] groups that `rank` ranks first,
/// kept by their cut key, and the count of the groups it did not keep.
/// Groups are offered in key order, so a tie in `rank` keeps the earlier
/// key, exactly as the figures' stable sorts kept it. The tie is decided
/// while a group is evicted, not only when the kept groups are sorted:
/// the kept map itself is the key order.
pub(crate) struct Ranked<K, A, R> {
    kept: BTreeMap<K, (R, A)>,
    more: usize,
}

impl<K: Ord, A, R: Ord> Default for Ranked<K, A, R> {
    fn default() -> Ranked<K, A, R> {
        Ranked::new()
    }
}

impl<K: Ord, A, R: Ord> Ranked<K, A, R> {
    pub(crate) fn new() -> Ranked<K, A, R> {
        Ranked {
            kept: BTreeMap::new(),
            more: 0,
        }
    }

    /// Whether every one of the [`GROUPS`] is already kept.
    fn is_full(&self) -> bool {
        self.kept.len() == GROUPS
    }

    /// Offers one complete group. A group that ranks after the worst kept,
    /// or ties it, is counted in `more` and dropped; one that ranks
    /// strictly before it replaces it.
    pub(crate) fn push(&mut self, key: K, accumulator: A, rank: R)
    where
        K: Clone,
    {
        if !self.is_full() {
            self.kept.insert(key, (rank, accumulator));
            return;
        }
        self.more += 1;
        let worst = self
            .kept
            .iter()
            .max_by(|(left_key, (left_rank, _)), (right_key, (right_rank, _))| {
                left_rank.cmp(right_rank).then(left_key.cmp(right_key))
            })
            .map(|(key, _)| key.clone())
            .expect("64 groups are kept");
        // The groups arrive in key order, so a new group that ties the
        // worst kept rank has the later key and is the one dropped.
        let worst_rank = self
            .kept
            .get(&worst)
            .map(|(rank, _)| rank)
            .expect("the worst kept key is kept");
        let keep = rank < *worst_rank;
        if keep {
            self.kept.remove(&worst);
            self.kept.insert(key, (rank, accumulator));
        }
    }

    /// Visits each kept group in key order, for a decision every group of
    /// the figure takes part in, kept or not.
    pub(crate) fn for_each_mut(&mut self, mut visit: impl FnMut(&K, &mut A)) {
        for (key, (_, accumulator)) in &mut self.kept {
            visit(key, accumulator);
        }
    }

    /// The kept groups in key order, and the count of the rest.
    pub(crate) fn finish(self) -> (Vec<(K, A)>, usize) {
        let kept = self
            .kept
            .into_iter()
            .map(|(key, (_, accumulator))| (key, accumulator))
            .collect();
        (kept, self.more)
    }
}
