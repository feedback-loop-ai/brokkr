//! A present driver refusal that cannot be read, named by the field it
//! broke (decision 0103 ruling 1(a)). The transport's generic unreadable
//! report deliberately withholds the serde text (decision 0012), so a
//! malformed refusal would read as unreadable bytes with no field; this
//! module reads the one field again and hands back the typed error, whose
//! Display names the field and never quotes the driver's content.

use serde_json::Value;

use crate::PreSessionRefusal;

/// Why a `result`'s present refusal object cannot be read, or `None` when
/// the line is not a result or the refusal is readable/absent.
pub(super) fn malformed(line: &str) -> Option<String> {
    let value: Value = serde_json::from_str(line).ok()?;
    if value.get("type").and_then(Value::as_str) != Some("result") {
        return None;
    }
    let refusal = value.get("refusal")?;
    PreSessionRefusal::read(refusal)
        .err()
        .map(|error| format!("driver refusal is unreadable: {error}"))
}
