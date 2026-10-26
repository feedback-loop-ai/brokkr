/// What a boxed seat's hands may do to its worktree, as the office declares
/// it (#896 round 2, decision 0069's carrier). A closed, serialized
/// vocabulary shared by the office loader, the engine that marks a site and
/// the driver that chooses dsh's own sandbox mode: the words are the wire,
/// never a free string read back by key (decision 0071 rulings 2 and 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Access {
    /// The seat reviews and writes no worktree file.
    ReadOnly,
    /// The seat writes the worktree through its one tool.
    Writes,
}
