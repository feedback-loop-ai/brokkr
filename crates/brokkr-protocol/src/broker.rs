//! The MCP capability broker's private plan and its refusals (decision
//! 0077; slice two U6c, design D5, MB3, MB4, SC1). The engine seals one
//! plan per selected holding and pins it in the attempt's inventory; `brokkr
//! broker serve` reads both and serves nothing a caller names beside them.
//!
//! Every type here is closed: an unknown, duplicate or missing field
//! refuses as the bytes are parsed, and nothing defaults. Every record is
//! a JSON object, never a positional array, so a field is only ever read
//! by its name. A plan holds binding names, locators and compact identity
//! facts, never a secret value, a live descriptor or a fresh grant. Its
//! bytes are authority only once the protected inventory pins their
//! digest; a matching path and digest alone confer nothing.

use std::path::PathBuf;

use serde::de::{Deserializer, Visitor};
use serde::Deserialize;

/// A record's deserializer that offers its derived visitor a map and
/// nothing else: serde's derived struct visitor also takes a sequence,
/// which would read the fields by position.
struct Object<D>(D);

impl<'de, D: Deserializer<'de>> Deserializer<'de> for Object<D> {
    type Error = D::Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        self.0.deserialize_map(visitor)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf
        option unit unit_struct newtype_struct seq tuple tuple_struct map struct enum
        identifier ignored_any
    }
}

/// Each record's `Deserialize`: its derived (`remote = "Self"`) visitor,
/// read through [`Object`].
macro_rules! objects {
    ($($record:ident),+ $(,)?) => {$(
        impl<'de> Deserialize<'de> for $record {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                $record::deserialize(Object(deserializer))
            }
        }
    )+};
}

objects!(
    Inventory,
    Pin,
    Plan,
    Owner,
    Dialect,
    Connection,
    Restrictions,
    Clearance,
    BoxIntent,
    Reach,
    Tree,
    Sources,
    Writers,
    Bootstrap,
    Excluded,
);

/// The attempt's sealed inventory: the one list of plans the engine wrote
/// for it, each by locator and digest.
#[derive(Debug, Deserialize)]
#[serde(remote = "Self", deny_unknown_fields)]
pub struct Inventory {
    pub plans: Vec<Pin>,
}

/// One plan the inventory pins: where the engine wrote it and the sha256
/// of its bytes.
#[derive(Debug, Deserialize)]
#[serde(remote = "Self", deny_unknown_fields)]
pub struct Pin {
    pub locator: PathBuf,
    pub digest: String,
}

/// The engine's plan for one broker (D5): who owns it, the one capability
/// and dialect it serves, how its server is launched, the exact tools
/// admitted, the empty restrictions, the effective retention, the binding
/// names the server receives, the admitted clearance receipt and the box
/// the server runs in.
#[derive(Debug, Deserialize)]
#[serde(remote = "Self", deny_unknown_fields)]
pub struct Plan {
    pub owner: Owner,
    pub server: String,
    pub capability: String,
    pub dialect: Dialect,
    pub connection: Connection,
    pub tools: Vec<String>,
    /// MCP restrictions are deferred (MB2), so a plan holds none.
    pub restrictions: Restrictions,
    pub retained: bool,
    /// The decision 0012 binding names the dialect declares.
    pub secrets: Vec<String>,
    pub clearance: Clearance,
    #[serde(rename = "box")]
    pub intent: BoxIntent,
}

/// The repository, run, effect, attempt, site and instance the plan was
/// written for, each in the engine's own spelling: `repo` is the sha256 of
/// the canonical operated worktree, the effect and attempt are the
/// engine's UUID strings and the instance its site instance reference.
#[derive(Debug, Deserialize)]
#[serde(remote = "Self", deny_unknown_fields)]
pub struct Owner {
    pub repo: String,
    pub run: String,
    pub effect: String,
    pub attempt: String,
    pub site: String,
    pub instance: String,
}

/// The dialect the holding binds: its name, the sha256 of its file and of
/// the capability definition, and the `serverInfo` version pin.
#[derive(Debug, Deserialize)]
#[serde(remote = "Self", deny_unknown_fields)]
pub struct Dialect {
    pub name: String,
    pub digest: String,
    pub definition: String,
    pub version: String,
}

/// The dialect's stdio connection, run directly and never through a shell.
#[derive(Debug, Deserialize)]
#[serde(remote = "Self", deny_unknown_fields)]
pub struct Connection {
    pub argv: Vec<String>,
}

/// No restriction: `{}` and nothing else parses.
#[derive(Debug, Deserialize)]
#[serde(remote = "Self", deny_unknown_fields)]
pub struct Restrictions {}

/// The sealed receipt of the binding-minimum comparison the engine
/// admitted: the dialect digest and the binding-policy digest it compared.
/// The broker checks the receipt, never runtime's egress ordering again.
#[derive(Debug, Deserialize)]
#[serde(remote = "Self", deny_unknown_fields)]
pub struct Clearance {
    pub dialect: String,
    pub policy: String,
}

/// The box the server runs in (SC1): the seat's reach, the resolved
/// executable and its program tree, the observed sources, the managed
/// writers, the network, the fixed environment names, the bootstrap and
/// the roots the box never holds.
#[derive(Debug, Deserialize)]
#[serde(remote = "Self", deny_unknown_fields)]
pub struct BoxIntent {
    pub reach: Reach,
    pub executable: PathBuf,
    pub tree: Tree,
    pub sources: Sources,
    pub writers: Writers,
    pub network: Network,
    /// The builder's fixed environment names; no value is sealed.
    pub environment: Vec<String>,
    pub bootstrap: Bootstrap,
    pub excluded: Excluded,
}

/// The selected seat's effective hands reach, by canonical root: what the
/// seat may write (workspace, Git, read-write binds) and what it may only
/// read (read-only and overlay binds).
#[derive(Debug, Deserialize)]
#[serde(remote = "Self", deny_unknown_fields)]
pub struct Reach {
    pub writable: Vec<PathBuf>,
    pub readable: Vec<PathBuf>,
}

/// MB3's closed program-tree distinction.
#[derive(Debug, Deserialize)]
#[serde(
    remote = "Self",
    tag = "kind",
    rename_all = "lowercase",
    deny_unknown_fields
)]
pub enum Tree {
    /// The executable sits directly in a shared system bin directory and
    /// is its own tree. A struct variant, so a field beside its kind is
    /// refused rather than ignored.
    System {},
    /// The executable's package, bound read-only at `root`.
    Package { root: PathBuf },
}

/// The bounded source observation the plan seals: how many entries and
/// mount records it held, and the sha256 of the observed set.
#[derive(Debug, Deserialize)]
#[serde(remote = "Self", deny_unknown_fields)]
pub struct Sources {
    pub entries: u64,
    pub mounts: u64,
    pub digest: String,
}

/// Every managed writer's mapped uid, and how their privilege is confined.
#[derive(Debug, Deserialize)]
#[serde(remote = "Self", deny_unknown_fields)]
pub struct Writers {
    pub uids: Vec<u32>,
    pub privilege: Privilege,
}

/// How a managed writer's privilege is confined: no capability, no new
/// privilege, no remount. An unknown confinement does not parse.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Privilege {
    Confined,
}

/// The network the dialect's egress projects to: `local` is isolated,
/// `contracted` and `uncontracted` share the host's.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Isolated,
    Shared,
}

/// The trusted bootstrap: the current Brokkr binary and its sha256.
#[derive(Debug, Deserialize)]
#[serde(remote = "Self", deny_unknown_fields)]
pub struct Bootstrap {
    pub path: PathBuf,
    pub digest: String,
}

/// The roots no box source may hold: the operator's store and the
/// attempt's control roots (plans, ledgers, staging).
#[derive(Debug, Deserialize)]
#[serde(remote = "Self", deny_unknown_fields)]
pub struct Excluded {
    pub store: PathBuf,
    pub control: Vec<PathBuf>,
}

/// Why the broker serves no plan, in MB3 and MB4's exact words. A text
/// names no path, binding value or child output.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    #[error("broker plan is not bound to this attempt")]
    Unbound,
    /// Decision 0012's own cause for a binding name, which names the
    /// binding and never a value.
    #[error("{0}")]
    Name(String),
    #[error("MCP server startup inputs are not protected from seat writes")]
    StartupInputs,
    #[error("MCP server box program tree cannot be resolved")]
    ProgramTree,
    #[error("MCP server launch resolves inside seat-writable reach")]
    LaunchInReach,
    #[error("MCP server box bind overlaps seat reach")]
    BindOverlapsReach,
    #[error("MCP server box filesystem identity is not protected")]
    Identity,
    #[error("MCP secret store is reachable by workspace hands")]
    StoreReachable,
    #[error("MCP secret store would be mounted in the server box")]
    StoreInBox,
    /// SD3's temporary cause: an admitted plan before U6f completes the
    /// serving protections, removed with them.
    #[error("broker serving protections are incomplete")]
    ServingIncomplete,
}
