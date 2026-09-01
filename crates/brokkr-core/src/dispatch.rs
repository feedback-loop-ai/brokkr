//! Looper-bound dispatch and run-manifest v2 contracts.
//!
//! v1 stays frozen. A Looper-started run pins this complete dispatch envelope
//! inside an immutable v2 run manifest; later bridge invocations recover the
//! same bytes from the store instead of trusting a mutable side file.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::canonical;

pub const DISPATCH_SCHEMA_V2: &str = "forge-dispatch/v2";
pub const RUN_MANIFEST_SCHEMA_V2: &str = "forge-run-manifest/v2";
pub const PRODUCER_EFFECTS: [&str; 5] = [
    "observation",
    "checkpoint",
    "report",
    "intervention_response",
    "terminal_report",
];
pub const REQUIRED_FORBIDDEN_ACTIONS: [&str; 5] = [
    "grant_create",
    "grant_widen",
    "artifact_decide",
    "workflow_advance",
    "release_promote",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LooperBinding {
    pub organization_id: String,
    pub product_id: String,
    pub story_id: String,
    pub delivery_run_id: String,
    pub request_grant_id: String,
    pub feature_path: String,
    pub immutable_inputs_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorBinding {
    pub principal_kind: String,
    pub principal_id: String,
    pub actor_kind: String,
    pub actor_id: String,
    pub accountable_operator_id: String,
    pub authority_source: String,
    pub operating_profile: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryBinding {
    pub owner: String,
    pub name: String,
    pub base_sha: String,
    pub candidate_sha: Option<String>,
    pub workspace_class: String,
    pub target_environment: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeBinding {
    pub name: String,
    pub compiled_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BudgetBinding {
    pub lane_tally_run_id: String,
    pub reservation_id: Option<String>,
    pub cost_state: String,
    pub ceiling_microunits: Option<u64>,
    pub currency: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProducerBinding {
    pub registration_id: String,
    pub token_reference: String,
    pub callback_audience: String,
    pub accepting_service_id: String,
    pub runtime_id: String,
    pub producer_release: String,
    pub protocol_version: u32,
    pub starting_cursor: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DispatchBounds {
    pub max_attempts: u32,
    pub max_parallel_effects: u32,
    pub max_event_bytes: u32,
    pub max_events_per_ten_seconds: u32,
    pub replay_retention_seconds: u64,
    pub safe_stop: String,
    pub cancellation: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DispatchEnvelopeV2 {
    pub schema: String,
    pub envelope_id: String,
    pub forge_run_id: String,
    pub issued_at: String,
    pub expires_at: String,
    pub canonical_digest: String,
    pub looper: LooperBinding,
    pub actor: ActorBinding,
    pub repository: RepositoryBinding,
    pub recipe: RecipeBinding,
    pub budget: BudgetBinding,
    pub producer: ProducerBinding,
    pub allowed_effects: Vec<String>,
    pub forbidden_actions: Vec<String>,
    pub bounds: DispatchBounds,
    pub evidence_requirements: Vec<String>,
    pub attestation_requirement: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunManifestV2 {
    pub schema: String,
    pub engine: String,
    pub event_schema: u32,
    pub database_schema: u32,
    pub driver_protocol: u32,
    pub bundle_name: String,
    pub files: Map<String, Value>,
    pub bundle_sha256: String,
    pub dispatch_sha256: String,
    pub dispatch: DispatchEnvelopeV2,
}

#[derive(Debug, Error, PartialEq)]
pub enum DispatchError {
    #[error("dispatch schema is unsupported")]
    BadSchema,
    #[error("dispatch field '{0}' is empty or malformed")]
    BadField(&'static str),
    #[error("dispatch time window is invalid or expired")]
    InvalidTime,
    #[error("dispatch digest does not match canonical content")]
    BadDigest,
    #[error("dispatch recipe digest does not match the compiled bundle")]
    RecipeMismatch,
    #[error("dispatch producer effects are not the closed least-scope set")]
    EffectScope,
    #[error("dispatch does not forbid every reserved action")]
    ForbiddenScope,
    #[error("dispatch execution bounds are unsafe")]
    UnsafeBounds,
    #[error("dispatch budget/cost envelope is incomplete")]
    Budget,
    #[error("run manifest is not a supported Forge manifest")]
    BadManifest,
    /// Decision 0016's first named limit, refused rather than truncated.
    /// `bundle_manifest_from_run` reconstructs the bundle manifest from
    /// six named keys and drops the rest, and `dispatch_from_run`
    /// re-checks `bundle_sha256` against that reconstruction — so an
    /// `agents` key would be silently dropped on the v2 round-trip and
    /// every adopting Looper-dispatched run would become unresumable
    /// with a diff that blames no file. Widening a contract a
    /// counterpart system reads is not this slice's to do unilaterally.
    #[error(
        "this bundle pins agent resolutions ('agents' in its manifest) and the \
         Looper-bound run-manifest/v2 lineage cannot carry them: the v2 \
         round-trip reconstructs the bundle manifest from six named keys, so \
         the pin would be dropped and the run would become unresumable. Run \
         this bundle without --dispatch until a jointly agreed v2-lineage \
         manifest version exists"
    )]
    AgentsUnsupportedByDispatchLineage,
    /// The same limit, fail-closed over the whole key space. `agents`
    /// was refused by name, and then decision 0021's `drivers` witness
    /// reached the manifest through a different key and the guard did
    /// not fire — the exact silent drop the named refusal existed to
    /// prevent, reachable again (found by this run's own third review;
    /// the operator ruled remedy ii). So the lineage now refuses every
    /// key beyond the six it can round-trip, and the NEXT witness the
    /// local lineage learns is refused loudly on the day it lands.
    #[error(
        "this bundle's manifest carries '{0}' and the Looper-bound \
         run-manifest/v2 lineage cannot: the v2 round-trip reconstructs \
         the bundle manifest from six named keys, so the key would be \
         dropped and the run would become unresumable with a diff that \
         blames no file. Run this bundle without --dispatch until a \
         jointly agreed v2-lineage manifest version carries it"
    )]
    ManifestKeyUnsupportedByDispatchLineage(String),
}

fn is_hex_64(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 512 && !value.chars().any(char::is_control)
}

fn safe_audience(value: &str) -> bool {
    let Ok(url) = url::Url::parse(value) else {
        return false;
    };
    if !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return false;
    }
    url.scheme() == "https"
        || (url.scheme() == "http" && matches!(url.host_str(), Some("127.0.0.1" | "localhost")))
}

impl DispatchEnvelopeV2 {
    pub fn compute_digest(&self) -> String {
        let mut value = serde_json::to_value(self).expect("dispatch serializes");
        value
            .as_object_mut()
            .expect("dispatch is an object")
            .remove("canonical_digest");
        canonical::sha256_hex(&value)
    }

    pub fn sealed(mut self) -> Self {
        self.canonical_digest = self.compute_digest();
        self
    }

    pub fn verify(
        &self,
        now: OffsetDateTime,
        compiled_bundle_sha256: &str,
    ) -> Result<(), DispatchError> {
        if self.schema != DISPATCH_SCHEMA_V2 || self.producer.protocol_version != 1 {
            return Err(DispatchError::BadSchema);
        }
        for (name, value) in [
            ("envelope_id", self.envelope_id.as_str()),
            ("forge_run_id", self.forge_run_id.as_str()),
            ("organization_id", self.looper.organization_id.as_str()),
            ("product_id", self.looper.product_id.as_str()),
            ("story_id", self.looper.story_id.as_str()),
            ("delivery_run_id", self.looper.delivery_run_id.as_str()),
            ("request_grant_id", self.looper.request_grant_id.as_str()),
            ("feature_path", self.looper.feature_path.as_str()),
            ("principal_id", self.actor.principal_id.as_str()),
            ("actor_id", self.actor.actor_id.as_str()),
            (
                "accountable_operator_id",
                self.actor.accountable_operator_id.as_str(),
            ),
            ("repository_owner", self.repository.owner.as_str()),
            ("repository_name", self.repository.name.as_str()),
            ("recipe_name", self.recipe.name.as_str()),
            ("lane_tally_run_id", self.budget.lane_tally_run_id.as_str()),
            ("registration_id", self.producer.registration_id.as_str()),
            ("token_reference", self.producer.token_reference.as_str()),
            ("runtime_id", self.producer.runtime_id.as_str()),
            ("producer_release", self.producer.producer_release.as_str()),
            ("workspace_class", self.repository.workspace_class.as_str()),
            (
                "target_environment",
                self.repository.target_environment.as_str(),
            ),
        ] {
            if !nonempty(value) {
                return Err(DispatchError::BadField(name));
            }
        }
        if !is_hex_64(&self.canonical_digest)
            || !is_hex_64(&self.looper.immutable_inputs_sha256)
            || !is_hex_64(&self.repository.base_sha)
            || self
                .repository
                .candidate_sha
                .as_deref()
                .is_some_and(|sha| !is_hex_64(sha))
            || !is_hex_64(&self.recipe.compiled_sha256)
        {
            return Err(DispatchError::BadField("sha256"));
        }
        let issued = OffsetDateTime::parse(&self.issued_at, &Rfc3339)
            .map_err(|_| DispatchError::InvalidTime)?;
        let expires = OffsetDateTime::parse(&self.expires_at, &Rfc3339)
            .map_err(|_| DispatchError::InvalidTime)?;
        if issued >= expires || now < issued || now >= expires {
            return Err(DispatchError::InvalidTime);
        }
        if self.compute_digest() != self.canonical_digest {
            return Err(DispatchError::BadDigest);
        }
        if self.actor.principal_kind != "api_key"
            || self.actor.principal_id != self.producer.token_reference
            || self.actor.authority_source != "looper-grant"
            || ![
                "accountable_human",
                "ai_agent",
                "service",
                "system_validator",
            ]
            .contains(&self.actor.actor_kind.as_str())
            || self.producer.accepting_service_id != "looper-api"
            || self.producer.starting_cursor != 0
        {
            return Err(DispatchError::BadField("producer_authority"));
        }
        if self.recipe.compiled_sha256 != compiled_bundle_sha256 {
            return Err(DispatchError::RecipeMismatch);
        }
        let effects: BTreeSet<&str> = self.allowed_effects.iter().map(String::as_str).collect();
        if effects.len() != self.allowed_effects.len()
            || effects.is_empty()
            || effects
                .iter()
                .any(|effect| !PRODUCER_EFFECTS.contains(effect))
        {
            return Err(DispatchError::EffectScope);
        }
        let forbidden: BTreeSet<&str> = self.forbidden_actions.iter().map(String::as_str).collect();
        if forbidden.len() != self.forbidden_actions.len()
            || forbidden.len() != REQUIRED_FORBIDDEN_ACTIONS.len()
            || REQUIRED_FORBIDDEN_ACTIONS
                .iter()
                .any(|action| !forbidden.contains(action))
        {
            return Err(DispatchError::ForbiddenScope);
        }
        if self.bounds.max_attempts == 0
            || self.bounds.max_parallel_effects == 0
            || self.bounds.max_event_bytes == 0
            || self.bounds.max_event_bytes > 65_536
            || self.bounds.max_events_per_ten_seconds == 0
            || self.bounds.max_events_per_ten_seconds > 40
            || self.bounds.replay_retention_seconds < 604_800
            || !["boundary", "nearest_phase_boundary"].contains(&self.bounds.safe_stop.as_str())
            || !["fenced", "fenced_operator_command"].contains(&self.bounds.cancellation.as_str())
            || !safe_audience(&self.producer.callback_audience)
        {
            return Err(DispatchError::UnsafeBounds);
        }
        let cost_state = self.budget.cost_state.as_str();
        if ![
            "known",
            "evidenced-zero",
            "unknown",
            "not-applicable",
            "reconciliation-pending",
            "final",
        ]
        .contains(&cost_state)
            || self
                .budget
                .ceiling_microunits
                .is_none_or(|value| value == 0)
            || self.budget.currency.as_deref().is_none_or(|value| {
                value.len() != 3 || !value.bytes().all(|byte| byte.is_ascii_uppercase())
            })
        {
            return Err(DispatchError::Budget);
        }
        let evidence: BTreeSet<&str> = self
            .evidence_requirements
            .iter()
            .map(String::as_str)
            .collect();
        if evidence.len() != self.evidence_requirements.len()
            || !evidence.contains("ordered_hash_chain")
            || evidence.iter().any(|requirement| !nonempty(requirement))
            || self.attestation_requirement != "self_reported"
        {
            return Err(DispatchError::BadField("evidence_requirements"));
        }
        Ok(())
    }
}

pub fn build_run_manifest_v2(
    bundle_manifest: &Value,
    dispatch: DispatchEnvelopeV2,
) -> Result<Value, DispatchError> {
    let object = bundle_manifest
        .as_object()
        .ok_or(DispatchError::BadManifest)?;
    let string = |key: &'static str| {
        object
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or(DispatchError::BadManifest)
    };
    let integer = |key: &'static str| {
        object
            .get(key)
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .ok_or(DispatchError::BadManifest)
    };
    let files = object
        .get("files")
        .and_then(Value::as_object)
        .cloned()
        .ok_or(DispatchError::BadManifest)?;
    if files.is_empty() {
        return Err(DispatchError::BadManifest);
    }
    // A loud refusal beats a quiet substitution: a bundle whose seats
    // reference agents cannot ride this lineage without losing its pin.
    if object.contains_key("agents") {
        return Err(DispatchError::AgentsUnsupportedByDispatchLineage);
    }
    // And fail-closed for every key the round-trip cannot carry — the
    // v5 `drivers` witness proved the named refusal above was a list
    // that could fall behind the manifest it guards.
    const V2_ROUND_TRIP_KEYS: [&str; 6] = [
        "engine",
        "event_schema",
        "database_schema",
        "driver_protocol",
        "bundle_name",
        "files",
    ];
    if let Some(unsupported) = object
        .keys()
        .find(|key| !V2_ROUND_TRIP_KEYS.contains(&key.as_str()))
    {
        return Err(DispatchError::ManifestKeyUnsupportedByDispatchLineage(
            unsupported.clone(),
        ));
    }
    let bundle_sha256 = canonical::sha256_hex(bundle_manifest);
    if dispatch.recipe.compiled_sha256 != bundle_sha256 {
        return Err(DispatchError::RecipeMismatch);
    }
    let engine = string("engine")?;
    let bundle_name = string("bundle_name")?;
    let event_schema = integer("event_schema")?;
    let database_schema = integer("database_schema")?;
    let driver_protocol = integer("driver_protocol")?;
    if !nonempty(&engine)
        || !nonempty(&bundle_name)
        || event_schema != 1
        || database_schema != 1
        || driver_protocol != 1
    {
        return Err(DispatchError::BadManifest);
    }
    serde_json::to_value(RunManifestV2 {
        schema: RUN_MANIFEST_SCHEMA_V2.to_string(),
        engine,
        event_schema,
        database_schema,
        driver_protocol,
        bundle_name,
        files,
        bundle_sha256,
        dispatch_sha256: dispatch.canonical_digest.clone(),
        dispatch,
    })
    .map_err(|_| DispatchError::BadManifest)
}

pub fn bundle_manifest_from_run(manifest: &Value) -> Result<Value, DispatchError> {
    if manifest.get("schema").and_then(Value::as_str) != Some(RUN_MANIFEST_SCHEMA_V2) {
        // The local lineage IS the bundle manifest, minus the one thing
        // that was never part of it: the world the run was invoked into
        // (run-manifest/v4, carried forward by v5). The map is workspace data — a run started
        // with one must still resume against the same bundle, so the pin
        // is dropped here rather than compared against a bundle that
        // never carried it.
        if manifest.get("realms").is_some() {
            let mut fields = manifest.as_object().cloned().unwrap_or_default();
            fields.remove("realms");
            return Ok(Value::Object(fields));
        }
        return Ok(manifest.clone());
    }
    let parsed: RunManifestV2 =
        serde_json::from_value(manifest.clone()).map_err(|_| DispatchError::BadManifest)?;
    Ok(serde_json::json!({
        "engine": parsed.engine,
        "event_schema": parsed.event_schema,
        "database_schema": parsed.database_schema,
        "driver_protocol": parsed.driver_protocol,
        "bundle_name": parsed.bundle_name,
        "files": parsed.files,
    }))
}

pub fn dispatch_from_run(manifest: &Value) -> Result<Option<DispatchEnvelopeV2>, DispatchError> {
    if manifest.get("schema").and_then(Value::as_str) != Some(RUN_MANIFEST_SCHEMA_V2) {
        return Ok(None);
    }
    let parsed: RunManifestV2 =
        serde_json::from_value(manifest.clone()).map_err(|_| DispatchError::BadManifest)?;
    if parsed.dispatch_sha256 != parsed.dispatch.canonical_digest
        || parsed.bundle_sha256 != canonical::sha256_hex(&bundle_manifest_from_run(manifest)?)
    {
        return Err(DispatchError::BadManifest);
    }
    Ok(Some(parsed.dispatch))
}

#[cfg(test)]
mod tests;
