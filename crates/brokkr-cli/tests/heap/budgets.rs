//! The committed heap budgets (`quality/heap-bytes.json`), read by name by
//! every heap-budget binary.

use std::path::PathBuf;

/// The committed budget named `name`. A name with no whole-number budget
/// refuses.
pub(crate) fn budget_named(name: &str) -> u64 {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../quality/heap-bytes.json");
    let text = std::fs::read_to_string(&path).expect("quality/heap-bytes.json reads");
    let file: serde_json::Value =
        serde_json::from_str(&text).expect("quality/heap-bytes.json is JSON");
    file["budgets"][name]
        .as_u64()
        .unwrap_or_else(|| panic!("quality/heap-bytes.json holds no whole budget for {name}"))
}
