//! The witness table (#358): every digest this crate's tests pin, held
//! once as data in `witnesses.json` and read by each test that pins one.
//!
//! `tests/witness_digests.rs` measures the whole table, reports every
//! moved witness in one run, and rewrites the file under `BROKKR_BLESS=1`.
//! Why a value moved is said in the commit that moves it; the reviewed
//! diff of the data file is the witness.
//!
//! Each test binary includes this file once through `#[path]`; it is test
//! code by its path, so it stays outside the coverage denominator.

use std::collections::BTreeMap;
use std::path::Path;

/// The table's path under the workspace root.
pub(crate) const TABLE: &str = "crates/brokkr-runtime/tests/witnesses.json";

/// The pinned digests, in the file's own shape.
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Witnesses {
    /// Bundle directory, relative to the workspace → compiled manifest
    /// digest. The keys are the set of witnessed bundles.
    pub(crate) bundles: BTreeMap<String, String>,
    /// File under `agents/charters/` → SHA-256 of its bytes. Every
    /// charter the library ships is pinned.
    pub(crate) charters: BTreeMap<String, String>,
}

impl Witnesses {
    /// Read the table, refusing a file that is not exactly its shape.
    pub(crate) fn load(workspace: &Path) -> Self {
        let path = workspace.join(TABLE);
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|error| panic!("{} must be readable: {error}", path.display()));
        serde_json::from_slice(&bytes)
            .unwrap_or_else(|error| panic!("{} is not a witness table: {error}", path.display()))
    }
}
