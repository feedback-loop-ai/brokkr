//! The `operator/commanded` words a fold or a readout reads as an
//! operator annotation, so a command that merely carries one cannot pass
//! for the annotation: `repair` (decision 0108 ruling 4), `classify`
//! (decision 0064 ruling 5), `supersede` (decision 0047) and `settle`
//! (decision 0108 ruling 3, unit 3b). The bridge's door refuses every one
//! before it journals it; the list has one home here.

/// The annotation words, one list the bridge reads.
pub(crate) const RESERVED_ANNOTATIONS: [&str; 4] = [
    super::REPAIR,
    crate::ending::CLASSIFY,
    crate::residual::SUPERSEDE,
    // 440d builds `operator settle`; a command word cannot stand in for it.
    "settle",
];

/// Is `word` reserved for an operator annotation? The bridge asks before
/// it journals a command, so a remote word can never reach the fold or a
/// readout as an annotation nobody wrote.
pub fn reserved_annotation(word: &str) -> bool {
    RESERVED_ANNOTATIONS.contains(&word)
}
