use std::path::Path;

use serde::Deserialize;
use serde_json::value::RawValue;

use crate::admission::host::{config, HostError, Schema};

/// The schema a fresh or upgraded host file declares.
pub const CURRENT_HOST_SCHEMA: &str = Schema::V5.contract();

/// Validate with the admission loader, then upgrade only the schema
/// token. Borrowing its span preserves the spelling of every key and
/// value, including legacy ceilings that are optional in v5.
/// `None` means the file already declares v5.
pub fn host_upgrade(path: &Path, text: &str) -> Result<Option<String>, HostError> {
    let loaded = config(text.as_bytes()).map_err(|source| HostError::Invalid {
        path: path.to_path_buf(),
        source,
    })?;
    match loaded.schema {
        Schema::V5 => Ok(None),
        Schema::V1 | Schema::V2 | Schema::V3 | Schema::V4 => Ok(Some(replace_schema(text))),
    }
}

#[derive(Deserialize)]
struct SchemaSlot<'a> {
    #[serde(borrow)]
    schema: &'a RawValue,
}

fn replace_schema(text: &str) -> String {
    let slot: SchemaSlot<'_> =
        serde_json::from_str(text).expect("the host loader has validated the schema field");
    let old = slot.schema.get();
    let start = old.as_ptr().addr() - text.as_ptr().addr();
    let mut upgraded = text.to_owned();
    upgraded.replace_range(
        start..start + old.len(),
        &format!("\"{CURRENT_HOST_SCHEMA}\""),
    );
    upgraded
}
