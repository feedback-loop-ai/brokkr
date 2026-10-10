//! The capability section of `run-manifest/v12` (decision 0065 slice two,
//! SC3): projections of the sealed authority and of each site's sealed
//! outcomes, never a second resolution. A held record keeps every v11
//! field and adds what implements the capability and what it retains (CR1).
//! An `mcp` implementation is typed here whole — its server, connection,
//! pinned version and secret NAMES — while the compile fence still refuses
//! every `mcp` grant until U9b; no resolved value, ledger path, process id
//! or call id is a fact of a holding, so none reaches identity.

use brokkr_core::realms::GrantRetention;
use brokkr_protocol::adapters::AdapterKind;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use super::{
    Authority, Connection, DialectKind, Holding, NativePlan, Outcome, Retention, SiteCapabilities,
};

/// What carries a held capability (SC3): the provider's own power, by its
/// adapter key, or a broker-carried MCP server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Implementation {
    Native {
        provider: String,
        adapter_key: String,
    },
    Mcp {
        /// `cap-<capability>`: the name the server is carried under.
        server: String,
        connection: Connection,
        version: String,
        /// Decision 0012 binding names, never values.
        secrets: Vec<String>,
    },
}

impl Implementation {
    /// What `capability` is carried by through a dialect of `kind`. The
    /// reserved `hands` kind is never a grant, so nothing carries it.
    pub(super) fn of(capability: &str, kind: &DialectKind) -> Option<Implementation> {
        match kind {
            DialectKind::Native {
                provider,
                adapter_key,
            } => Some(Implementation::Native {
                provider: provider.clone(),
                adapter_key: adapter_key.clone(),
            }),
            DialectKind::Mcp(server) => Some(Implementation::Mcp {
                server: format!("cap-{capability}"),
                connection: server.connection.clone(),
                version: server.version.clone(),
                secrets: server.secrets.clone(),
            }),
            DialectKind::Hands => None,
        }
    }

    fn value(&self) -> Value {
        match self {
            Implementation::Native {
                provider,
                adapter_key,
            } => json!({"kind": "provider-native", "provider": provider,
                        "adapter_key": adapter_key}),
            Implementation::Mcp {
                server,
                connection,
                version,
                secrets,
            } => {
                let connection = match connection {
                    Connection::Stdio(argv) => json!({"argv": argv}),
                    Connection::Url(url) => json!({"url": url}),
                };
                json!({"kind": "mcp", "server": server, "connection": connection,
                       "version": version, "secrets": secrets})
            }
        }
    }
}

impl Retention {
    /// The declaration, the realm's disposition and the effective result
    /// [`Retention::effective`] rules. A grant whose map version reserves
    /// no `retain` cannot veto, so the dialect's declaration stands.
    fn value(self) -> Value {
        let realm = match self.realm {
            GrantRetention::Unreserved | GrantRetention::Inherit => "inherit",
            GrantRetention::Veto => "veto",
        };
        json!({"declared": self.declared, "realm": realm, "effective": self.effective()})
    }
}

impl Holding {
    fn value(&self) -> Value {
        json!({
            "classes": self.classes,
            "dialect": self.dialect,
            "dialect_sha256": self.dialect_sha256,
            "definition_sha256": self.definition_sha256,
            "tools": self.tools,
            "restrictions": self.restrictions,
            "implementation": self.implementation.value(),
            "retention": self.retention.value(),
        })
    }
}

impl Outcome {
    /// The per-candidate record of `run-manifest/v12`.
    pub fn manifest(&self) -> Value {
        let held: Map<String, Value> = self
            .held
            .iter()
            .map(|(name, holding)| (name.clone(), holding.value()))
            .collect();
        let native = match &self.native {
            // A known power whose OFF nobody measured used to be listed
            // here as `unmeasured`; such a seat is now refused (decision
            // 0066 ruling 1), so a record that exists names every declared
            // power as on or off.
            NativePlan::Known {
                declaration,
                on,
                off,
                ..
            } => json!({"inventory": "known", "declaration": declaration,
                        "on": on, "off": off}),
            NativePlan::Unmeasured {
                declaration,
                reason,
                ..
            } => {
                let mut native = json!({"inventory": "unmeasured", "reason": reason});
                if let Some(declaration) = declaration {
                    native["declaration"] = json!(declaration);
                }
                native
            }
        };
        let mut record = json!({
            "provider": self.provider,
            "held": held,
            "not_held": self.not_held,
            "notices": self.notices.iter().map(|(_, message)| message).collect::<Vec<_>>(),
            "native": native,
        });
        if let Some(model) = &self.model {
            record["model"] = json!(model);
        }
        record
    }
}

/// The concrete model ids one candidate is served on, as the compile read
/// them (`run-manifest/v13`): what admission counts a run against, read
/// from the run's own manifest and never resolved again through anyone's
/// adapters (#430). An agent candidate's is the id its adapter maps its
/// model to; an inline built-in model driver's, its `--model` pin and any
/// `--fallback-model`; a driver that takes no model reads none.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum ModelPins {
    /// The ids, primary first, each with the route its driver serves it on.
    Read(Vec<Served>),
    /// The flags on which the compile could not read one concrete id.
    Unreadable(Vec<String>),
}

/// One concrete id and the route the candidate's driver serves it on,
/// where it has one ([`served_route`]).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Served {
    pub(crate) id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) route: Option<String>,
}

impl Served {
    /// `id` as a driver of `harness` serves it.
    pub(crate) fn on(harness: &str, id: String) -> Served {
        let route = served_route(harness, &id).map(str::to_string);
        Served { id, route }
    }
}

/// A concrete model id's route: its prefix before the first `/`, where it
/// has one (decision 0036 ruling 2).
pub(crate) fn route(model_id: &str) -> Option<&str> {
    brokkr_protocol::adapters::split_route(model_id).0
}

/// The route a driver of `harness` serves `model_id` on: the built-in
/// driver's own rule, its default route included (a bare dsh id is served
/// on dsh's default), and an id's own [`route`] under a command no
/// built-in driver dispatches.
pub(crate) fn served_route<'a>(harness: &str, model_id: &'a str) -> Option<&'a str> {
    match AdapterKind::parse(harness) {
        Some(kind) => kind.served_route(model_id),
        None => route(model_id),
    }
}

impl SiteCapabilities {
    /// The per-site record of `run-manifest/v12`.
    pub fn manifest(&self) -> Value {
        self.record(self.outcomes.iter().map(Outcome::manifest).collect())
    }

    /// The per-site record of `run-manifest/v13`, the one a compile
    /// writes: each candidate's v12 record beside the model `pins` the
    /// compile read for it, in candidate order.
    pub(crate) fn pinned(&self, pins: &[ModelPins]) -> Value {
        let candidates: Vec<Value> = (self.outcomes.iter().zip(pins))
            .map(|(outcome, pins)| {
                let mut record = outcome.manifest();
                record["model_pins"] = json!(pins);
                record
            })
            .collect();
        self.record(candidates)
    }

    /// The per-site record around its `candidates`' records.
    fn record(&self, candidates: Vec<Value>) -> Value {
        let asks: Map<String, Value> = self
            .asks
            .asks
            .iter()
            .map(|(name, strength)| (name.clone(), json!(strength.word())))
            .collect();
        json!({
            "office": self.asks.office,
            "asks": asks,
            "subtracted": self.asks.subtracted,
            "candidates": candidates,
        })
    }
}

impl Authority {
    /// The manifest's realm-wide half: the grants as written, and every
    /// definition and dialect the compile consulted, used or not.
    pub fn manifest(&self, consulted: &[String]) -> Value {
        let grants: Map<String, Value> = self
            .context
            .grants
            .iter()
            .map(|(capability, grant)| (capability.clone(), grant.value()))
            .collect();
        let definitions: Map<String, Value> = self
            .context
            .grants
            .keys()
            .chain(consulted)
            .filter_map(|name| Some((name.clone(), self.definitions.get(name)?.value())))
            .collect();
        let dialects: Map<String, Value> = self
            .dialects
            .values()
            .map(|dialect| (dialect.name.clone(), dialect.value()))
            .collect();
        json!({
            "realm": self.context.realm,
            "grants": grants,
            "definitions": definitions,
            "dialects": dialects,
        })
    }
}
