//! The office's `access` declaration (#896 round 2), decoded once at the
//! loader edge into the shared typed carrier.

use brokkr_protocol::native_controls::Access;
use serde_json::{Map, Value};

use super::LibraryError;

/// The office's `access` declaration (#896 round 2): the closed pair
/// `read-only` or `writes`, decoded once here into the shared typed
/// carrier. Absent is `writes`. That default is a CHANGE for a legacy gate
/// hire that declares none: the old gate-class mapping made it read-only,
/// while all eight shipped read-only offices declare `read-only`
/// explicitly. A present value of any other shape is refused rather than
/// read as absent (decision 0071 rulings 2 and 3).
pub(super) fn parse(map: &Map<String, Value>, what: &str) -> Result<Access, LibraryError> {
    let Some(raw) = map.get("access") else {
        return Ok(Access::Writes);
    };
    serde_json::from_value(raw.clone()).map_err(|_| {
        LibraryError::Invalid(format!(
            "{what} 'access' is {raw}; the vocabulary is read-only or writes (decision 0071 \
             rulings 2 and 3)"
        ))
    })
}
