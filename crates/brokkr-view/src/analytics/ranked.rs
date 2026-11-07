//! The at most [`GROUPS`] groups a figure keeps, and the count of the rest
//! (decision 0089 ruling 7): a grouped figure holds the groups its own
//! order ranks first, each by its already-cut key, and prints the rest as
//! one `N more` count (ruling A3's additive `*_more` field).

/// The groups one figure keeps.
pub(crate) const GROUPS: usize = 64;

/// The `GROUPS` of `pairs` that `rank` ranks first, and how many of the
/// rest there are. `pairs` are fed in key order, so a tie in `rank` keeps
/// the earlier key, exactly as the figures' stable sorts kept it.
pub(crate) fn ranked<K, A, R>(
    pairs: impl IntoIterator<Item = (K, A)>,
    rank: impl Fn(&A) -> R,
) -> (Vec<(K, A)>, usize)
where
    R: Ord,
{
    let mut kept: Vec<(K, A)> = Vec::with_capacity(GROUPS);
    let mut more = 0;
    for (key, accumulator) in pairs {
        if kept.len() < GROUPS {
            kept.push((key, accumulator));
            continue;
        }
        let worst = kept
            .iter()
            .enumerate()
            .max_by(|(_, (_, left)), (_, (_, right))| rank(left).cmp(&rank(right)))
            .map(|(index, _)| index);
        if let Some(index) = worst {
            if rank(&accumulator) < rank(&kept[index].1) {
                kept[index] = (key, accumulator);
            }
        }
        more += 1;
    }
    (kept, more)
}
