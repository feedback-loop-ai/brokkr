//! A rule's conditions in the words a reader sees (#568): what the
//! console's graph labels an edge with, written once beside the
//! vocabulary it renders, so no surface spells a condition for itself.

use super::{Condition, Rule, SEVERITY_ORDER};

impl Rule {
    /// Each condition of the rule's `when`, in the table's key order:
    /// `consecutive_failures ≥ 2`, `max_residual_severity above low`,
    /// `not fixes_applied`, `strategy in chore, feature`. Empty for a rule
    /// that rules its pair unconditionally.
    pub fn when_words(&self) -> Vec<String> {
        self.when.iter().map(words).collect()
    }
}

fn words(condition: &Condition) -> String {
    match condition {
        Condition::CounterGte { name, threshold } => format!("{name} ≥ {threshold}"),
        Condition::SeverityAbove {
            name,
            threshold_rank,
        } => format!("{name} above {}", SEVERITY_ORDER[*threshold_rank]),
        Condition::SeverityAtMost {
            name,
            threshold_rank,
        } => format!("{name} at most {}", SEVERITY_ORDER[*threshold_rank]),
        Condition::Flag {
            name,
            expected: true,
        } => name.clone(),
        Condition::Flag {
            name,
            expected: false,
        } => format!("not {name}"),
        Condition::EnumIn { name, allowed } => format!("{name} in {}", allowed.join(", ")),
    }
}
