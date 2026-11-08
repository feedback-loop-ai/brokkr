//! The at most [`GROUPS`] groups a figure keeps, and the count of the rest
//! (decision 0089 ruling 7): a grouped figure holds the groups its own
//! order ranks first, each by its already-cut key, and prints the rest as
//! one `N more` count (ruling A3's additive `*_more` field).

/// The groups one figure keeps.
pub(crate) const GROUPS: usize = 64;

/// A bounded selection: the [`GROUPS`] groups that `rank` ranks first, kept
/// in the order they were offered, and the count of the groups it did not
/// keep. Every figure offers its groups in its own order — a figure's key
/// order, or a realm's discovery order — and that is the order `finish`
/// returns. A tie in `rank` keeps the earlier offered group, which for a
/// figure offering its keys in order is the earlier key. A group that ranks
/// strictly before the worst kept replaces it and is appended, so a later
/// offered group stays later.
pub(crate) struct Ranked<K, A, R> {
    kept: Vec<(K, R, A)>,
    more: usize,
}

impl<K, A, R: Ord> Default for Ranked<K, A, R> {
    fn default() -> Ranked<K, A, R> {
        Ranked::new()
    }
}

impl<K, A, R: Ord> Ranked<K, A, R> {
    pub(crate) fn new() -> Ranked<K, A, R> {
        Ranked {
            kept: Vec::new(),
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
    pub(crate) fn push(&mut self, key: K, accumulator: A, rank: R) {
        if !self.is_full() {
            self.kept.push((key, rank, accumulator));
            return;
        }
        self.more += 1;
        // The worst kept is the greatest rank; on a tie, the last offered.
        // The groups arrive in their figure's order, so a new group that
        // ties the worst kept rank is the one dropped.
        let worst = self
            .kept
            .iter()
            .enumerate()
            .max_by(
                |(left_at, (_, left_rank, _)), (right_at, (_, right_rank, _))| {
                    left_rank.cmp(right_rank).then(left_at.cmp(right_at))
                },
            )
            .map(|(at, _)| at)
            .expect("64 groups are kept");
        if rank < self.kept[worst].1 {
            self.kept.remove(worst);
            self.kept.push((key, rank, accumulator));
        }
    }

    /// Visits each kept group in the order it was offered, for a decision
    /// every group of the figure takes part in, kept or not.
    pub(crate) fn for_each_mut(&mut self, mut visit: impl FnMut(&K, &mut A)) {
        for (key, _, accumulator) in &mut self.kept {
            visit(key, accumulator);
        }
    }

    /// The kept groups in the order they were offered, and the count of the
    /// rest.
    pub(crate) fn finish(self) -> (Vec<(K, A)>, usize) {
        let kept = self
            .kept
            .into_iter()
            .map(|(key, _, accumulator)| (key, accumulator))
            .collect();
        (kept, self.more)
    }
}
