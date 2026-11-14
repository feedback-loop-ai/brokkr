//! The `operator/commanded` words a fold or a readout reads as an
//! operator annotation, so a command that merely carries one cannot pass
//! for the annotation: `repair` (decision 0108 ruling 4), `classify`
//! (decision 0064 ruling 5), `supersede` (decision 0047) and `settle`
//! (decision 0108 ruling 3, unit 3b). The bridge's door refuses every one
//! before it journals it; the list has one home here.

/// The command word an operator's settle record carries: `operator/commanded`
/// with this word and the args of contracts/operator-settle.v1.schema.json
/// (decision 0108 ruling 3, in ruling 4's erratum shape after decision 0047).
pub const SETTLE: &str = "settle";

/// The annotation words, one list the bridge reads.
pub(crate) const RESERVED_ANNOTATIONS: [&str; 4] = [
    super::REPAIR,
    crate::ending::CLASSIFY,
    crate::residual::SUPERSEDE,
    SETTLE,
];

/// Is `word` reserved for an operator annotation? The bridge asks before
/// it journals a command, so a remote word can never reach the fold or a
/// readout as an annotation nobody wrote.
pub fn reserved_annotation(word: &str) -> bool {
    RESERVED_ANNOTATIONS.contains(&word)
}
