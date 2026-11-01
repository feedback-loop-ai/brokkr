//! The engine's MCP server set and the checks that read MCP out of a
//! parsed command (decision 0065 slice two, U1a and U7a): the box's hands
//! as the engine binds them, one broker per sealed holding beside them, the
//! one builder of the document and bindings a harness is handed, an
//! independent reading of what a command carries, and the authored server
//! a recipe may not configure. The set is exactly `brokkr` and the brokers
//! named `cap-<capability>`; this module admits no other server.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Value};

use super::grammar::{self, Command, Effect, ListKind};
use super::State;
use crate::hands::SERVER_NAME;

/// One value as the body of a TOML basic string (rebuild unit 13-fix-c,
/// R2): the ONE encoder every value interpolated into a Codex TOML
/// assignment passes through, for the value emitted and the value expected
/// alike. `\` and `"` are escaped, each control character TOML forbids in
/// a basic string (U+0000 to U+001F and U+007F) is its short escape or
/// `\uXXXX`, and every other scalar value stands as itself, so what a TOML
/// reader decodes is `value` exactly: a literal `/` stays those six
/// characters and never decodes as a slash.
pub(super) fn toml_basic(value: &str) -> String {
    let mut body = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\\' => body.push_str("\\\\"),
            '"' => body.push_str("\\\""),
            '\u{8}' => body.push_str("\\b"),
            '\t' => body.push_str("\\t"),
            '\n' => body.push_str("\\n"),
            '\u{c}' => body.push_str("\\f"),
            '\r' => body.push_str("\\r"),
            c if c <= '\u{1f}' || c == '\u{7f}' => {
                body.push_str(&format!("\\u{:04X}", u32::from(c)))
            }
            c => body.push(c),
        }
    }
    body
}

/// The driver-input key the engine's artifact root rides under, beside the
/// sealed serving inputs and written with them (U8a; CR2): the final check
/// binds the box's hands to it as the engine made it, never derived.
pub const ARTIFACT_ROOT: &str = "artifact_root";

/// What the engine binds the box's hands to at spawn (decision 0043;
/// rebuild unit 13-fix-b, R1): its own executable, which the harness spawns
/// as `brokkr hands serve`, the site's workdir, the run-local artifact root
/// the box keeps read-only (decision 0065 slice two, U8a; CR2), the reach
/// sealed at admission that every box of it is built from (U8a2c; MB3) and
/// its typed hands declaration. The transport's values are produced from
/// these, never read from a record or a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transport<'a> {
    pub brokkr: &'a std::path::Path,
    pub workdir: &'a std::path::Path,
    pub artifacts: &'a std::path::Path,
    pub sealed: &'a crate::hands::Sealed,
    pub spec: &'a crate::hands::HandsSpec,
}

impl Transport<'_> {
    /// The MCP document a harness is handed to reach the box's hands alone,
    /// as the set's one builder writes it ([`Servers::expand`]).
    pub(crate) fn document(&self) -> Value {
        let hands = BTreeMap::from([(SERVER_NAME.to_string(), Owed::Hands(*self))]);
        document(self.brokkr, &hands)
    }

    /// An adapter's measured `hands.workspace` fragment with the box's
    /// hands alone expanded into it ([`Transport::expand_beside`]).
    pub fn expand(&self, fragment: &[String]) -> Option<Vec<String>> {
        self.expand_beside(fragment, &[])
    }

    /// An adapter's measured `hands.workspace` fragment with the whole set
    /// expanded into it, `brokers` beside the box's hands (U7d2; MB1), as
    /// the set's one builder writes it ([`Servers::expand`]); `None` where
    /// it cannot be served as sealed ([`Servers::named`]).
    pub fn expand_beside(
        &self,
        fragment: &[String],
        brokers: &[SealedBroker],
    ) -> Option<Vec<String>> {
        let servers = Servers {
            hands: Some(*self),
            brokers,
        };
        servers.expand(fragment)
    }

    /// The MCP document a Claude-grammar harness is handed to reach the
    /// box's hands and, beside them, `brokers` (U7d; MB1), as the set's one
    /// builder writes it; `None` where the set cannot be served as sealed
    /// ([`Servers::named`]).
    pub(crate) fn beside(&self, brokers: &[SealedBroker]) -> Option<Value> {
        let servers = Servers {
            hands: Some(*self),
            brokers,
        };
        Some(document(self.brokkr, &servers.named()?))
    }
}

impl SealedServers {
    /// The brokers this set seals beside the box's hands, none outside
    /// [`SealedServers::Brokered`].
    pub(crate) fn brokers(&self) -> &[SealedBroker] {
        match self {
            SealedServers::Brokered(brokers) => brokers,
            SealedServers::Empty | SealedServers::Hands | SealedServers::NoModelSurface => &[],
        }
    }

    /// The names a spawn removes from the harness's environment, and so
    /// from every broker it spawns (U7d; MB4): each broker's sealed
    /// removals, each name once, sorted, which is the list the final check
    /// ([`Servers::served`]) is handed as what the spawn removed.
    pub(crate) fn removals(&self) -> Vec<String> {
        let names: BTreeSet<&String> = (self.brokers().iter())
            .flat_map(|broker| &broker.removals)
            .collect();
        names.into_iter().cloned().collect()
    }
}

/// The server set a launch's site intends: the no-broker plan has no other,
/// and exec, which serves no model, intends none. Beside the box's hands, a
/// site holding MCP capabilities intends one broker per sealed holding
/// (decision 0065 slice two, U7a; MB1).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SealedServers {
    #[default]
    Empty,
    Hands,
    Brokered(Vec<SealedBroker>),
    NoModelSurface,
}

/// One held capability's broker as the engine seals it (U7a; MB1, MB4):
/// served as `cap-<capability>` by the engine's own executable running
/// `broker serve` on its own sealed plan, named by the plan's locator and
/// digest alone, beside the dialect's secret names the harness, and so the
/// broker it spawns, is served without.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SealedBroker {
    pub capability: String,
    pub plan: String,
    pub plan_digest: String,
    pub removals: Vec<String>,
}

impl SealedBroker {
    /// The one name a holding's broker is served as, `cap-<capability>`,
    /// `None` where `capability` is not one bare name: the set's builder
    /// and the guidance that names it to a seat (U7d; MB1) read it here.
    pub(crate) fn server(capability: &str) -> Option<String> {
        bare(capability).then(|| format!("cap-{capability}"))
    }

    /// The names Claude's allow list grants `brokers` by (U9b1d; MB1): each
    /// broker's one served name, `mcp__cap-<capability>`, once, in sealed
    /// order, which admits that server's tools; the broker stays the one
    /// gate on which of them run (MB3). The set's builder writes them
    /// ([`Servers::expand`]) and the final check binds a launch's allow
    /// list to them. `None` where a capability is not one bare name.
    pub(crate) fn allowance(brokers: &[SealedBroker]) -> Option<Vec<String>> {
        (brokers.iter())
            .map(|broker| Some(format!("mcp__{}", Self::server(&broker.capability)?)))
            .collect()
    }
}

/// One server of the engine's set, by what it serves.
#[derive(Debug, Clone, Copy)]
enum Owed<'a> {
    Hands(Transport<'a>),
    Broker(&'a SealedBroker),
}

impl Owed<'_> {
    /// The arguments the harness spawns this server with after the
    /// engine's executable: the hands profile's `hands serve`, or `broker
    /// serve` on this broker's own sealed plan alone, never a dialect's
    /// executable or the bootstrap.
    fn args(&self) -> Vec<String> {
        match self {
            Owed::Hands(hands) => {
                crate::hands::serve_args(hands.workdir, hands.artifacts, hands.sealed, hands.spec)
            }
            Owed::Broker(broker) => [
                "broker",
                "serve",
                "--plan",
                &broker.plan,
                "--plan-digest",
                &broker.plan_digest,
            ]
            .map(String::from)
            .to_vec(),
        }
    }

    /// Whether `entry`, as a command was read to carry it, is exactly this
    /// server under the engine's `brokkr`: read field by field against the
    /// sealed values, the sealed reach as the type it is (U8a2c), never
    /// compared with a rebuilt document.
    fn is(&self, brokkr: &str, entry: &Entry) -> bool {
        let args: Vec<&str> = entry.args.iter().map(String::as_str).collect();
        let bound = match (self, &args[..]) {
            (
                Owed::Hands(hands),
                ["hands", "serve", "--workdir", workdir, "--artifacts", artifacts, "--reach", reach, "--spec", spec],
            ) => {
                Some(*workdir) == hands.workdir.to_str()
                    && Some(*artifacts) == hands.artifacts.to_str()
                    && serde_json::from_str(reach)
                        .is_ok_and(|reach: crate::hands::Sealed| reach == *hands.sealed)
                    && serde_json::from_str(spec)
                        .is_ok_and(|spec: Value| spec == hands.spec.to_value())
            }
            (
                Owed::Broker(broker),
                ["broker", "serve", "--plan", plan, "--plan-digest", digest],
            ) => *plan == broker.plan && *digest == broker.plan_digest,
            _ => false,
        };
        bound && entry.command == brokkr
    }
}

/// The one MCP document every Claude-grammar harness is handed: each
/// server of `named` by its exact name, run by `brokkr` with its own
/// arguments.
fn document(brokkr: &std::path::Path, named: &BTreeMap<String, Owed<'_>>) -> Value {
    let servers: serde_json::Map<String, Value> = named
        .iter()
        .map(|(name, owed)| {
            let entry = json!({"command": brokkr.to_string_lossy(), "args": owed.args()});
            (name.clone(), entry)
        })
        .collect();
    json!({ "mcpServers": servers })
}

/// Whether `name` is one bare name, ASCII letters, digits, `_` and `-`,
/// which every harness's server table carries as itself.
fn bare(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// The engine's complete MCP server set at one launch (U7a; MB1, SI2): the
/// box's hands server, `brokkr`, where its hands are carried, and beside it
/// one broker per sealed holding. It is written by one builder
/// ([`Servers::expand`], and [`Transport::document`] for the hands alone)
/// and judged by an independent reading of what a command carries
/// ([`Servers::served`]), never by rebuilding that document.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Servers<'a> {
    pub(super) hands: Option<Transport<'a>>,
    pub(super) brokers: &'a [SealedBroker],
}

impl<'a> super::Serving<'a> {
    /// The complete set this serving binds at `harness`, its brokers beside
    /// the box's hands, or [`unsealed`] where it cannot be served as sealed
    /// ([`Servers::named`]).
    pub(super) fn servers(&self, harness: &str) -> Result<Servers<'a>, super::Refusal> {
        let servers = Servers {
            hands: self.hands,
            brokers: self.brokers,
        };
        servers
            .named()
            .map(|_| servers)
            .ok_or_else(|| unsealed(harness))
    }

    /// The final judgment of the set a whole command's `state` carries at
    /// `harness`, under the boundary its launch `stands` under (U7a; MB1,
    /// SI2): read on its own ([`Servers::served`]), or [`unsealed`].
    pub(super) fn sealed_set(
        &self,
        harness: &str,
        stands: Option<super::SealedBoundary>,
        state: &State,
    ) -> Result<(), super::Refusal> {
        let servers = self.servers(harness)?.under(stands);
        (servers.served(harness, state, self.removed))
            .then_some(())
            .ok_or_else(|| unsealed(harness))
    }
}

/// The final-set refusal at `harness` (U7a; MB1, SI2): the cause the
/// serving edge states too, as the driver's refusal carries it.
pub(super) fn unsealed(harness: &str) -> super::Refusal {
    let set = " final MCP configuration is not the engine's sealed server set";
    let pieces = vec![super::Piece::Provider(harness), super::Piece::Words(set)];
    super::refused(super::Why::Final, pieces)
}

impl<'a> Servers<'a> {
    /// This set as served under the boundary a launch `stands` under: under
    /// `harness` the harness's own class carries the box's hands, not a
    /// server.
    pub(super) fn under(self, stands: Option<super::SealedBoundary>) -> Self {
        let boxed = stands != Some(super::SealedBoundary::Harness);
        Servers {
            hands: self.hands.filter(|_| boxed),
            ..self
        }
    }

    /// Every server by its exact name, `None` where the set cannot be
    /// served as sealed: a broker beside no hands (MB2), or a capability
    /// whose `cap-` name is not one bare name or is another holding's. No
    /// name is shortened, normalized or suffixed into another. The empty
    /// set names nothing.
    fn named(&self) -> Option<BTreeMap<String, Owed<'a>>> {
        let mut named = BTreeMap::new();
        let Some(hands) = self.hands else {
            return self.brokers.is_empty().then_some(named);
        };
        named.insert(SERVER_NAME.to_string(), Owed::Hands(hands));
        for broker in self.brokers {
            let name = SealedBroker::server(&broker.capability)?;
            if named.insert(name, Owed::Broker(broker)).is_some() {
                return None;
            }
        }
        Some(named)
    }

    /// An adapter's measured `hands.workspace` fragment with the engine's
    /// tokens expanded as the spawn expands them: `{hands_mcp_json}` to the
    /// whole set's document, `{hands_args_toml}` to the hands server's
    /// arguments as a TOML array of [`toml_basic`] strings and `{brokkr}`,
    /// which stands in a TOML basic string, to the executable that string
    /// encodes (rebuild unit 13-fix-c, R2). Beside brokers, the value of
    /// the fragment's allow list also names each broker's server
    /// ([`SealedBroker::allowance`]; U9b1d), after what it allowed, so a
    /// headless seat may call what its broker admits; a set without brokers
    /// keeps every byte. A set without the box's hands
    /// serves no hands fragment, so nothing. `None` where the executable,
    /// the workdir, the artifact root or a path of the sealed reach is not
    /// UTF-8, since no provider value represents that path exactly and a
    /// lossy one would bind another, and where the set cannot be served as
    /// sealed.
    pub(super) fn expand(&self, fragment: &[String]) -> Option<Vec<String>> {
        let Some(hands) = self.hands else {
            return Some(Vec::new());
        };
        let brokkr = hands.brokkr.to_str()?;
        hands.workdir.to_str()?;
        hands.artifacts.to_str()?;
        serde_json::to_string(hands.sealed).ok()?;
        let document = document(hands.brokkr, &self.named()?).to_string();
        let allowance = SealedBroker::allowance(self.brokers)?;
        let quoted: Vec<String> = (Owed::Hands(hands).args().iter())
            .map(|arg| format!("\"{}\"", toml_basic(arg)))
            .collect();
        let (arguments, command) = (format!("[{}]", quoted.join(",")), toml_basic(brokkr));
        let expanded = (fragment.iter()).map(|part| {
            part.replace("{hands_mcp_json}", &document)
                .replace("{hands_args_toml}", &arguments)
                .replace("{brokkr}", &command)
        });
        Some(allowing(fragment, expanded.collect(), &allowance))
    }

    /// Whether `controls`, the MCP options of one command at `harness`,
    /// carry exactly this set, read on their own ([`read`]): every name,
    /// and each server's executable and arguments. A harness with no MCP
    /// carrier in its arguments carries none. dsh carries the box's hands,
    /// and nothing beside them, in the one overlay its driver stages from
    /// this transport, whose path the final check binds (#896), so its
    /// arguments carry no MCP option and no broker rides beside them.
    fn carried(&self, harness: &str, controls: &[(&str, &Vec<String>)]) -> bool {
        if harness == "dsh" {
            return controls.is_empty() && self.brokers.is_empty() && self.hands.is_some();
        }
        let (Some(owed), Some(read)) = (self.named(), read(harness, controls)) else {
            return false;
        };
        let brokkr = self.hands.and_then(|hands| hands.brokkr.to_str());
        read.len() == owed.len()
            && owed.iter().all(|(name, owed)| {
                let entry = read.get(name);
                entry.is_some_and(|entry| brokkr.is_some_and(|brokkr| owed.is(brokkr, entry)))
            })
    }

    /// Whether the box's hands' read `state` carries exactly this set at
    /// `harness`, and nothing beside it (rebuild unit 13-fix-b, R1; U7a).
    pub(super) fn carried_by(&self, harness: &str, state: &State) -> bool {
        let controls: Vec<(&str, &Vec<String>)> = state.controls().collect();
        self.carried(harness, &controls)
    }

    /// The final judgment of a whole command (U7a; MB1, SI2): the MCP
    /// options its `state` carries at `harness`, read independently, are
    /// exactly this set, and the names its spawn `removed` from the
    /// harness's environment are exactly the brokers' sealed removals, each
    /// once (MB4). The empty set is carried by no MCP option at all or by
    /// the strict flag beside the empty document SI2's isolation appends.
    /// dsh carries its set in its engine-only home and its hands in its
    /// staged overlay, not in its arguments, as R1 reads ([`Self::carried_by`]).
    pub(super) fn served(&self, harness: &str, state: &State, removed: &[String]) -> bool {
        let owed: BTreeSet<&str> = (self.brokers.iter())
            .flat_map(|broker| &broker.removals)
            .map(String::as_str)
            .collect();
        let removes: BTreeSet<&str> = removed.iter().map(String::as_str).collect();
        let controls: Vec<(&str, &Vec<String>)> = state
            .controls()
            .filter(|control| mcp_option(harness, control))
            .collect();
        let empty = controls.is_empty() && self.hands.is_none() && self.brokers.is_empty();
        let carried = match harness {
            "claude" | "lanetally" | "codex" => empty || self.carried(harness, &controls),
            _ => true,
        };
        carried && removes.len() == removed.len() && removes == owed
    }
}

/// `expanded`, `fragment`'s parts with the engine's tokens expanded, with
/// `allowance` joined after the value of each allow list `fragment` writes
/// as an option and its own value, as Claude's grammar reads one
/// (`--allowedTools`; U9b1d). Brokers ride only Claude's grammar, which
/// LaneTally's wrapper shares; a fragment writing no such list, as Codex's
/// and dsh's write none, keeps its parts, and an empty `allowance` changes
/// no byte.
fn allowing(fragment: &[String], mut expanded: Vec<String>, allowance: &[String]) -> Vec<String> {
    for (at, flag) in fragment.iter().enumerate() {
        let allows = grammar::list_of("claude", flag) == Some(ListKind::Allow);
        if let (true, Some(value)) = (allows, expanded.get_mut(at + 1)) {
            let allowed = (Some(value.as_str())
                .filter(|value| !value.is_empty())
                .into_iter())
            .chain(allowance.iter().map(String::as_str));
            *value = allowed.collect::<Vec<_>>().join(",");
        }
    }
    expanded
}

/// Whether one control of a command at `harness` is an MCP option: strict
/// MCP or a document for Claude's grammar, and an assignment into the
/// `mcp_servers` table for Codex.
fn mcp_option(harness: &str, (name, values): &(&str, &Vec<String>)) -> bool {
    match harness {
        "codex" => {
            *name == "--config"
                && (values.iter())
                    .any(|value| grammar::config_under(&grammar::config_key(value), "mcp_servers"))
        }
        _ => matches!(*name, "--strict-mcp-config" | "--mcp-config"),
    }
}

/// One server entry as a command carries it, read on its own terms: its
/// executable and its arguments, and nothing beside them.
#[derive(Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    command: String,
    args: Vec<String>,
}

/// The servers `controls` carry at `harness`, read independently of the
/// builder: for Claude's grammar the one strict flag beside the one
/// document, a closed object naming each server once; for Codex the three
/// assignments of each server and nothing else. `None` where they are not
/// exactly one such reading.
fn read(harness: &str, controls: &[(&str, &Vec<String>)]) -> Option<BTreeMap<String, Entry>> {
    match harness {
        "claude" | "lanetally" => {
            let (mut strict, mut documents) = (0, Vec::new());
            for (name, values) in controls {
                match (*name, values.as_slice()) {
                    ("--strict-mcp-config", []) => strict += 1,
                    ("--mcp-config", [document]) => documents.push(document),
                    _ => return None,
                }
            }
            let [document] = documents[..] else {
                return None;
            };
            let read = serde_json::from_str::<Document>(document).ok();
            read.filter(|_| strict == 1).map(|read| read.servers.0)
        }
        "codex" => codex_entries(controls),
        _ => None,
    }
}

/// The brokers one Claude-grammar MCP `document` serves, each by the name
/// the allow list grants it, `mcp__cap-<capability>`, read as [`read`]
/// reads a document: nothing where it is not one closed table naming each
/// server once (U9b1d; ruling R4). Only the hands contribution's own
/// document is read so, which the final check binds to the sealed set.
pub(super) fn brokered(document: &str) -> Vec<String> {
    let servers = serde_json::from_str::<Document>(document).map(|read| read.servers.0);
    (servers.into_iter().flat_map(BTreeMap::into_keys))
        .filter(|name| name.strip_prefix("cap-").is_some_and(bare))
        .map(|name| format!("mcp__{name}"))
        .collect()
}

/// An MCP document as a Claude-grammar harness reads it: one closed table.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    #[serde(rename = "mcpServers")]
    servers: Unique<Entry>,
}

/// A JSON object read with each name once: a name written twice is
/// refused, never read last-wins.
struct Unique<T>(BTreeMap<String, T>);

impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for Unique<T> {
    fn deserialize<D: serde::Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
        reader.deserialize_map(Names(std::marker::PhantomData))
    }
}

/// [`Unique`]'s reader of one object's names.
struct Names<T>(std::marker::PhantomData<T>);

impl<'de, T: serde::Deserialize<'de>> serde::de::Visitor<'de> for Names<T> {
    type Value = Unique<T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an object naming each server once")
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Unique<T>, A::Error> {
        let mut read = BTreeMap::new();
        while let Some((name, value)) = map.next_entry::<String, T>()? {
            if read.insert(name, value).is_some() {
                return Err(serde::de::Error::custom("a server is named twice"));
            }
        }
        Ok(Unique(read))
    }
}

/// The servers Codex `--config` assignments bind: each exactly
/// `mcp_servers.<name>.<part>=<value>` through bare key parts, every part
/// once, and each server's command and arguments as TOML basic strings
/// beside an approving tool mode. `None` for anything else.
fn codex_entries(controls: &[(&str, &Vec<String>)]) -> Option<BTreeMap<String, Entry>> {
    let blank = [' ', '\t'];
    let mut parts: BTreeMap<&str, BTreeMap<&str, &str>> = BTreeMap::new();
    for (name, values) in controls {
        let ("--config", [assignment]) = (*name, values.as_slice()) else {
            return None;
        };
        let (key, value) = assignment.split_once('=')?;
        let key: Vec<&str> = key
            .split('.')
            .map(|part| part.trim_matches(blank))
            .collect();
        let ["mcp_servers", server, part] = key[..] else {
            return None;
        };
        let fields = parts.entry(server).or_default();
        if !bare(server) || fields.insert(part, value.trim_matches(blank)).is_some() {
            return None;
        }
    }
    let entry = |fields: &BTreeMap<&str, &str>| {
        let owed = ["args", "command", "default_tools_approval_mode"];
        let [Some(args), Some(command), Some(mode)] = owed.map(|part| fields.get(part)) else {
            return None;
        };
        let approves = fields.len() == owed.len() && toml_string(mode)? == "approve";
        approves.then_some(())?;
        Some(Entry {
            command: toml_string(command)?,
            args: toml_strings(args)?,
        })
    };
    (parts.iter())
        .map(|(server, fields)| Some((server.to_string(), entry(fields)?)))
        .collect()
}

/// One TOML basic string read back to the value it encodes, `None` where
/// `text` is not exactly one: the inverse of [`toml_basic`], written apart
/// from it so that a reading is never the writer's own echo.
fn toml_string(text: &str) -> Option<String> {
    let mut chars = text.strip_prefix('"')?.strip_suffix('"')?.chars();
    let mut value = String::new();
    while let Some(c) = chars.next() {
        value.push(match c {
            '\\' => escaped(&mut chars)?,
            '"' => return None,
            c if (c <= '\u{1f}' && c != '\t') || c == '\u{7f}' => return None,
            c => c,
        });
    }
    Some(value)
}

/// The character the escape after one `\` in `chars` names, `None` for an
/// escape TOML does not define.
fn escaped(chars: &mut std::str::Chars<'_>) -> Option<char> {
    Some(match chars.next()? {
        'b' => '\u{8}',
        't' => '\t',
        'n' => '\n',
        'f' => '\u{c}',
        'r' => '\r',
        '"' => '"',
        '\\' => '\\',
        'u' => scalar(chars, 4)?,
        'U' => scalar(chars, 8)?,
        _ => return None,
    })
}

/// The scalar value an escape names in `digits` hexadecimal digits.
fn scalar(chars: &mut std::str::Chars<'_>, digits: usize) -> Option<char> {
    let hex: String = chars.by_ref().take(digits).collect();
    let whole = hex.len() == digits && hex.chars().all(|c| c.is_ascii_hexdigit());
    char::from_u32(u32::from_str_radix(&hex, 16).ok().filter(|_| whole)?)
}

/// A TOML array of basic strings read back to its values, `None` where
/// `text` is not exactly one.
fn toml_strings(text: &str) -> Option<Vec<String>> {
    let blank = [' ', '\t'];
    let mut rest = text
        .strip_prefix('[')?
        .strip_suffix(']')?
        .trim_matches(blank);
    let mut values = Vec::new();
    while !rest.is_empty() {
        let mut escaped = false;
        let end = (rest.char_indices().skip(1)).find_map(|(at, c)| {
            let closes = !escaped && c == '"';
            escaped = !escaped && c == '\\';
            closes.then_some(at + 1)
        })?;
        values.push(toml_string(&rest[..end])?);
        rest = rest[end..].trim_start_matches(blank);
        match rest.strip_prefix(',') {
            Some(tail) if !tail.trim_start_matches(blank).is_empty() => {
                rest = tail.trim_start_matches(blank);
            }
            Some(_) => return None,
            None if rest.is_empty() => {}
            None => return None,
        }
    }
    Some(values)
}

/// The first part of the box's whole transport that the typed hands
/// `hands` do not carry at `provider` (rebuild unit 13-fix-c, R1): strict
/// MCP and the MCP document for Claude and LaneTally, whose include
/// restriction and workspace allowance the composer writes itself; the
/// sandbox class and the server's three bindings for Codex, the class
/// carried by `--sandbox` or by the engine's permission profile bound to
/// `bound` (U9c1; operator ruling 4 of 2026-10-23).
pub(super) fn untransported(
    provider: &str,
    hands: &Command,
    bound: grammar::Bound<'_>,
) -> Option<&'static str> {
    let named = |name: &str| hands.nodes.iter().any(|node| node.name() == name);
    let profiled = || matches!(grammar::profile(hands, bound), Ok(Some(_)));
    let bound = |part: &str| {
        let key = format!("mcp_servers.{}.{part}", crate::hands::SERVER_NAME);
        hands
            .nodes
            .iter()
            .filter(|node| node.name() == "--config")
            .flat_map(|node| &node.values)
            .any(|value| grammar::config_key(value) == key)
    };
    let owed: Vec<(bool, &'static str)> = match provider {
        "claude" | "lanetally" => vec![
            (
                named("--strict-mcp-config"),
                "strict MCP configuration ('--strict-mcp-config')",
            ),
            (named("--mcp-config"), "MCP document ('--mcp-config')"),
        ],
        "codex" => vec![
            (
                named("--sandbox") || profiled(),
                "sandbox class ('--sandbox' or the engine's permission profile)",
            ),
            (
                bound("command"),
                "server command binding ('mcp_servers.brokkr.command')",
            ),
            (
                bound("args"),
                "server arguments binding ('mcp_servers.brokkr.args')",
            ),
            (
                bound("default_tools_approval_mode"),
                "server approval binding ('mcp_servers.brokkr.default_tools_approval_mode')",
            ),
        ],
        _ => Vec::new(),
    };
    owed.into_iter()
        .find(|(present, _)| !present)
        .map(|(_, what)| what)
}

/// The first AUTHORED effect that configures a capability server, loads a
/// plugin, or admits a server's tools — as the NAME of what was written,
/// never a value (decision 0066 rulings 4 and 6; second council H1 and
/// H2). Independent of any native inventory and of any grant: a recipe's
/// driver command is recipe data, and only the realm grants a capability.
///
/// The judgment is on the parsed STRUCTURE, so it does not enumerate
/// spellings:
///
/// - any assignment into the `mcp_servers` table or under it, in all five
///   Codex config spellings including the attached `-cKEY=VALUE`, however
///   the key is quoted or spaced;
/// - any option whose modelled effect is LOADING — `--mcp-config`,
///   `--settings`, `--plugin-dir`, `--agents`, a Codex `--profile` — because
///   each loads a document that can configure a server, and a document the
///   engine cannot classify is not a channel a recipe may open;
/// - any value of any INCLUDE or ALLOW list that names an `mcp__` tool or
///   carries a wildcard, judged on every value of every occurrence rather
///   than on the first.
///
/// A DENY list is never here: subtraction narrows access and is not an
/// admission (second council M1).
///
/// There is deliberately no exception for a server named `brokkr`: the
/// engine's own hands arrive in the OTHER part, and a name proves nothing.
pub fn authored_server_conflict(command: &Command) -> Option<String> {
    for node in &command.nodes {
        match node.spec.effect {
            Effect::Config => {
                let key = grammar::config_key(node.values.first().map_or("", String::as_str));
                if grammar::config_under(&key, "mcp_servers") {
                    return Some(format!("{} mcp_servers", node.spelling));
                }
            }
            Effect::Load => return Some(node.spelling.clone()),
            Effect::List(ListKind::Include | ListKind::Allow) => {
                for tool in grammar::node_patterns(node)
                    .into_iter()
                    .map(grammar::tool_name)
                {
                    if tool.starts_with("mcp__") {
                        return Some(format!("{} mcp__*", node.name()));
                    }
                    if tool.contains('*') {
                        return Some(format!("{} *", node.name()));
                    }
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests;
