//! The inventory a backup publishes and a restore reads before any other
//! file (decision 0100 ruling 2): closed formats, read from a regular
//! file only and under a byte bound, so a backup directory can neither
//! block a restore in `open(2)` nor grow its memory until the host
//! refuses it.

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

use serde::{Deserialize, Serialize};

use super::BackupError;

/// The inventory format this build writes (decision 0100 ruling 2).
pub const JOURNAL_BACKUP_FORMAT: &str = "brokkr.journal-backup.v2";
pub(super) const JOURNAL_BACKUP_V1: &str = "brokkr.journal-backup.v1";

/// The inventory's one name inside a backup directory: the last file a
/// backup publishes, and the mark that makes a backup complete.
pub(crate) const INVENTORY_FILE: &str = "inventory.json";

/// The most bytes a backup's `inventory.json` may hold. A longer file is
/// refused by name before it is parsed, and the read asks for one byte past
/// this bound so that a longer file is distinguishable from one exactly at
/// it.
///
/// The inventory is not bounded by its field types, so this is a stated
/// constant rather than a figure derived from the leaf types: the
/// `boundary.heads` list grows by one entry for every run in the journal,
/// and `taken_at` is the caller's string. One head is at most 128 bytes of
/// run id (`RUN_ID_MAX`, at most doubled by JSON escaping), a 64-byte chain
/// head and the JSON around them, under 512 bytes; 16 MiB therefore holds
/// more than 32,000 runs at their largest, far beyond the journals this
/// build records, while keeping one restore's inventory to a fixed handful
/// of megabytes.
pub(crate) const INVENTORY_MAX_BYTES: u64 = 16 * 1024 * 1024;

/// The journal's schema versions, as the store records them, plus the
/// projection version the taking build reads.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Schemas {
    pub(crate) database: u32,
    pub(crate) queue: u32,
    pub(crate) projection: u32,
    #[serde(default, skip_serializing_if = "OrdersSchema::is_legacy")]
    pub(super) orders: OrdersSchema,
}

/// V1 omits the field. V2 declares either the recorded version or null.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(untagged)]
pub(super) enum OrdersSchema {
    #[default]
    #[serde(skip)]
    Legacy,
    Recorded(Option<u32>),
}

impl OrdersSchema {
    fn is_legacy(&self) -> bool {
        matches!(self, Self::Legacy)
    }
}

impl Inventory {
    fn check_format(&self) -> Result<(), BackupError> {
        match (self.format.as_str(), &self.schemas.orders) {
            (JOURNAL_BACKUP_V1, OrdersSchema::Legacy)
            | (JOURNAL_BACKUP_FORMAT, OrdersSchema::Recorded(_)) => Ok(()),
            (JOURNAL_BACKUP_V1, OrdersSchema::Recorded(_)) => {
                Err(invalid_inventory("schemas.orders is absent in v1"))
            }
            (JOURNAL_BACKUP_FORMAT, OrdersSchema::Legacy) => {
                Err(invalid_inventory("schemas.orders is required in v2"))
            }
            (found, _) => Err(BackupError::UnknownFormat {
                found: found.to_owned(),
            }),
        }
    }
}

fn invalid_inventory(message: impl std::fmt::Display) -> BackupError {
    BackupError::InvalidInventory {
        source: serde::de::Error::custom(message),
    }
}

/// The snapshot file's size and digest.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Snapshot {
    pub(crate) bytes: u64,
    pub(crate) sha256: String,
}

/// One run's place in the declared boundary: its last event's seq and the
/// chain head that event sealed, or `0` and the zero hash for a run with
/// no event.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Head {
    pub(crate) run_id: String,
    pub(crate) seq: u64,
    pub(crate) chain_head: String,
}

/// The instant a snapshot stands at, as the snapshot itself records it.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Boundary {
    pub(crate) taken_at: String,
    pub(crate) runs: u64,
    pub(crate) events: u64,
    pub(crate) heads: Vec<Head>,
}

/// What a slice-1 backup does not carry, each named so a reader never
/// mistakes the journal for a complete recovery unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Excluded {
    GitObjectsAndAnchors,
    ArtifactsOutsideTheJournal,
    HarnessTranscripts,
    Credentials,
}

pub(crate) const EXCLUDED: [Excluded; 4] = [
    Excluded::GitObjectsAndAnchors,
    Excluded::ArtifactsOutsideTheJournal,
    Excluded::HarnessTranscripts,
    Excluded::Credentials,
];

/// `inventory.json`, exactly as this format states it. `deny_unknown_fields`
/// keeps a field this build does not read from passing as one it does.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Inventory {
    pub(crate) format: String,
    pub(crate) engine: String,
    pub(crate) schemas: Schemas,
    pub(crate) snapshot: Snapshot,
    pub(crate) boundary: Boundary,
    pub(crate) excluded: Vec<Excluded>,
}

/// The inventory a backup directory states, read under
/// [`INVENTORY_MAX_BYTES`] and only from a regular file. A missing one is
/// the interrupted backup's own diagnostic.
pub(crate) fn read_inventory(from: &Path) -> Result<Inventory, BackupError> {
    let path = from.join(INVENTORY_FILE);
    let file = match open_nonblocking(&path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(BackupError::MissingInventory(from.to_path_buf()));
        }
        Err(source) => return Err(BackupError::UnreadableInventory { path, source }),
    };
    // The handle's own metadata is the authority: a symlink to a regular
    // file is followed and reads as it always has, while a FIFO, socket,
    // directory or device is refused before a byte is read. A handle whose
    // metadata cannot be read is not known to be regular, so it takes the
    // same refusal.
    if !file.metadata().is_ok_and(|metadata| metadata.is_file()) {
        return Err(BackupError::InventoryNotRegular(path));
    }
    let mut bytes = Vec::new();
    if let Err(source) = file.take(INVENTORY_MAX_BYTES + 1).read_to_end(&mut bytes) {
        return Err(BackupError::UnreadableInventory { path, source });
    }
    if bytes.len() as u64 > INVENTORY_MAX_BYTES {
        return Err(BackupError::InventoryTooLarge {
            path,
            bound: INVENTORY_MAX_BYTES,
        });
    }
    let inventory: Inventory = serde_json::from_slice(&bytes)
        .map_err(|source| BackupError::InvalidInventory { source })?;
    inventory.check_format()?;
    Ok(inventory)
}

/// Serialize `inventory` for publication, refusing a file past
/// [`INVENTORY_MAX_BYTES`] before anything is published: the bound is a
/// fact of the bytes, so a backup whose inventory would exceed it is
/// refused while its destination is still empty. A backup holds its own
/// inventory to the same bound the read enforces, so an inventory this
/// build writes is one it can read.
pub(crate) fn serialize(inventory: &Inventory, final_path: &Path) -> Result<Vec<u8>, BackupError> {
    let bytes = serde_json::to_vec_pretty(inventory)?;
    if bytes.len() as u64 > INVENTORY_MAX_BYTES {
        return Err(BackupError::InventoryTooLarge {
            path: final_path.to_path_buf(),
            bound: INVENTORY_MAX_BYTES,
        });
    }
    Ok(bytes)
}

/// Write `bytes`, already [`serialize`]'s, to a new file at `temporary`
/// and fsync it. The bytes are this build's own serialization, held to the
/// bound the read enforces before the snapshot was published.
pub(crate) fn write(bytes: &[u8], temporary: &Path) -> Result<(), BackupError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

/// Open `path` read-only without waiting for a writer: a FIFO with no
/// writer must answer the open, not block it.
fn open_nonblocking(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    options.custom_flags(O_NONBLOCK);
    options.open(path)
}

/// `O_NONBLOCK`, as the two Unix hosts spell it. The standard library names
/// no open flag, and the house admits no new dependency for one number
/// (decision 0014), so the value is stated here: Linux `0o4000`, macOS and
/// the BSDs `0x0004` (decision 0063: Linux and macOS are the hosts).
#[cfg(unix)]
const O_NONBLOCK: i32 = if cfg!(target_os = "linux") {
    0o4000
} else {
    0x0004
};
