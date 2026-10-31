//! Decision 0082 ruling 4: the `brokkr.tool-dialect/v1` row names this
//! crate's private embedded validator, and the copy is byte-equal to the
//! published file.

use super::TOOL_DIALECT_SCHEMA;

#[test]
fn the_lifecycle_row_names_the_schema_and_the_embedded_copy_is_byte_equal() {
    let row = crate::contract_lifecycle::row("contracts/tool-dialect.v1.schema.json");
    assert_eq!(row.version, "brokkr.tool-dialect/v1");
    assert_eq!(
        row.reader,
        "schema brokkr_runtime::capabilities::dialect::TOOL_DIALECT_SCHEMA"
    );
    let published = std::fs::read(
        crate::contract_lifecycle::workspace().join("contracts/tool-dialect.v1.schema.json"),
    )
    .expect("the published tool dialect");
    assert_eq!(
        TOOL_DIALECT_SCHEMA.as_bytes(),
        published.as_slice(),
        "the embedded copy moved from the published file"
    );
}
