//! The effort a scaffolded chain link is hired at. It is carried as data
//! beside the link's model in `SEATS`, so no rule in `init.rs` derives an
//! effort from a model's name.

/// `High` is the level the claude harness runs unconfigured, so the
/// starter names what it would have got rather than tuning a stranger's
/// first run. Sol's scale sits one step lower (decision 0045's addendum of
/// 2026-09-30), so a scaffolded sol link is hired at `Medium`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Effort {
    Medium,
    High,
}

impl Effort {
    /// The word an agent file's `efforts` map carries, in the adapters'
    /// own vocabulary.
    pub(super) fn word(self) -> &'static str {
        match self {
            Effort::Medium => "medium",
            Effort::High => "high",
        }
    }
}
