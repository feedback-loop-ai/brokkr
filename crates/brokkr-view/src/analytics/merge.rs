//! The union's k-way merge (decision 0089 ruling 7): every source's pulled
//! stream of one figure, merged in the figure's full key with one head a
//! source, so a union of hearths holds one fact a hearth while a figure
//! folds, never the window's facts.
//!
//! A source's stream is open only while the reader its source handed it
//! to runs, so each source is opened inside the one before it, and each
//! is merged two-way with the merge of the sources after it. A two-way
//! merge holds the next fact of each side; the side that is itself a merge
//! holds no head for the fact it handed over, so the sources' heads are
//! all the merge holds.

/// Whether the first fact stands after the second in a figure's full key.
pub(super) type After<'a, T> = &'a dyn Fn(&T, &T) -> bool;

/// Open one source's stream of a figure and hand it to the reader.
pub(super) type Open<'a, S, T> = &'a dyn Fn(&S, &mut dyn FnMut(&mut dyn Iterator<Item = T>));

/// Hand `read` every source's stream merged by `after`: two facts that tie
/// keep their sources' order, and a source's own facts keep theirs.
pub(super) fn merged<S, T>(
    sources: &[S],
    open: Open<'_, S, T>,
    after: After<'_, T>,
    read: &mut dyn FnMut(&mut dyn Iterator<Item = T>),
) {
    match sources {
        [] => read(&mut std::iter::empty()),
        [only] => open(only, read),
        [first, rest @ ..] => open(first, &mut |left| {
            merged(rest, open, after, &mut |right| {
                read(&mut Merge {
                    left: &mut *left,
                    right,
                    left_head: None,
                    right_head: None,
                    after,
                })
            });
        }),
    }
}

/// Two ordered streams as one: the lesser of the sides' next facts is
/// yielded, the left on a tie. A side is pulled only when the merge is
/// asked for its next fact and holds none of that side, so between pulls
/// the merge holds the head of the side it did not yield.
struct Merge<'a, T> {
    left: &'a mut dyn Iterator<Item = T>,
    right: &'a mut dyn Iterator<Item = T>,
    /// Each side's next fact: not yet pulled, pulled and held, or the
    /// side's end once it has none.
    left_head: Option<Option<T>>,
    right_head: Option<Option<T>>,
    after: After<'a, T>,
}

impl<T> Iterator for Merge<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        let left = self.left_head.get_or_insert_with(|| self.left.next());
        let right = self.right_head.get_or_insert_with(|| self.right.next());
        let take_right = match (&*left, &*right) {
            (Some(left), Some(right)) => (self.after)(left, right),
            (None, _) => true,
            (Some(_), None) => false,
        };
        match take_right {
            // A side's end stays held, so an ended side is never pulled
            // again.
            true => self.right_head.take_if(|head| head.is_some())?,
            false => self.left_head.take()?,
        }
    }
}

#[cfg(test)]
mod tests;
