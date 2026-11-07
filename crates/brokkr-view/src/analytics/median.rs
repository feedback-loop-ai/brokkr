//! A median selected in passes, never sorted (decision 0089 ruling 7): the
//! first pass reads each target's count, least and greatest duration; each
//! later pass counts the durations inside the interval that holds each
//! middle rank into [`BUCKETS`] buckets and narrows to the bucket that
//! holds it, until each rank is one value. Eleven narrowing passes take the
//! widest `i64` span past one value. The caller hands each pass's stream of
//! `(target, duration)` values through a closure seam (decision 0071 ruling
//! 2), so the whole read and the store's rows feed it one fact at a time.

/// The buckets one pass counts into.
const BUCKETS: usize = 64;

/// The passes a median takes: the first reads the bounds and eleven narrow.
const PASSES: usize = 12;

/// One middle rank's search: the interval that still holds it, how many
/// values are below that interval, and the counts of the last pass.
struct Selection {
    rank: u64,
    lo: i64,
    hi: i64,
    below: u64,
    buckets: [u64; BUCKETS],
    value: Option<i64>,
}

impl Selection {
    fn new(rank: u64, lo: i64, hi: i64) -> Selection {
        Selection {
            rank,
            lo,
            hi,
            below: 0,
            buckets: [0; BUCKETS],
            value: (lo == hi).then_some(lo),
        }
    }

    /// Counts `value` when it is this selection's to count.
    fn count(&mut self, value: i64) {
        if self.value.is_some() || value < self.lo || value > self.hi {
            return;
        }
        let bucket = bucket(value, self.lo, self.hi);
        self.buckets[bucket] += 1;
    }

    /// Narrows to the bucket that holds the rank, or finishes on a value.
    fn narrow(&mut self) {
        if self.value.is_some() {
            return;
        }
        let mut wanted = self.rank - self.below;
        let mut chosen = BUCKETS - 1;
        for (index, count) in self.buckets.iter().enumerate() {
            if wanted < *count {
                chosen = index;
                break;
            }
            wanted -= *count;
            self.below += *count;
        }
        let (lo, hi) = bounds(self.lo, self.hi, chosen);
        self.lo = lo;
        self.hi = hi;
        self.buckets = [0; BUCKETS];
        if lo == hi {
            self.value = Some(lo);
        }
    }
}

/// The target's median, selected in at most [`PASSES`] passes of `pass`,
/// which is handed each pass's stream.
pub(crate) fn medians(
    targets: usize,
    mut pass: impl FnMut(&mut dyn FnMut(usize, i64)),
) -> Vec<Option<i64>> {
    let mut counts = vec![0u64; targets];
    let mut least = vec![i64::MAX; targets];
    let mut greatest = vec![i64::MIN; targets];
    pass(&mut |target, value| {
        counts[target] += 1;
        least[target] = least[target].min(value);
        greatest[target] = greatest[target].max(value);
    });
    let mut selections: Vec<Option<[Selection; 2]>> = (0..targets)
        .map(|target| match counts[target] {
            0 => None,
            count => {
                let (lo, hi) = (least[target], greatest[target]);
                Some([
                    Selection::new((count - 1) / 2, lo, hi),
                    Selection::new(count / 2, lo, hi),
                ])
            }
        })
        .collect();
    for _ in 1..PASSES {
        if selections
            .iter()
            .flatten()
            .all(|pair| pair.iter().all(|selection| selection.value.is_some()))
        {
            break;
        }
        pass(&mut |target, value| {
            let pair = selections[target]
                .as_mut()
                .expect("a counted target has a selection");
            pair[0].count(value);
            pair[1].count(value);
        });
        for pair in selections.iter_mut().flatten() {
            for selection in pair.iter_mut() {
                selection.narrow();
            }
        }
    }
    selections
        .into_iter()
        .map(|pair| {
            let mut pair = pair?;
            for selection in pair.iter_mut() {
                selection.value.get_or_insert(selection.lo);
            }
            Some(average(pair[0].value?, pair[1].value?))
        })
        .collect()
}

/// The mean of the two middle values, in a width that cannot overflow.
fn average(left: i64, right: i64) -> i64 {
    ((left as i128 + right as i128) / 2) as i64
}

/// The bucket `value` falls in, of the buckets that cover `lo..=hi`.
fn bucket(value: i64, lo: i64, hi: i64) -> usize {
    let span = hi as i128 - lo as i128 + 1;
    let offset = value as i128 - lo as i128;
    (offset * BUCKETS as i128 / span) as usize
}

/// The bounds of `index`'s bucket among the buckets that cover `lo..=hi`.
fn bounds(lo: i64, hi: i64, index: usize) -> (i64, i64) {
    let span = hi as i128 - lo as i128 + 1;
    let ceil = |part: i128| (part + BUCKETS as i128 - 1) / BUCKETS as i128;
    let start = lo as i128 + ceil(span * index as i128);
    let end = lo as i128 + ceil(span * (index as i128 + 1)) - 1;
    (start as i64, end as i64)
}
