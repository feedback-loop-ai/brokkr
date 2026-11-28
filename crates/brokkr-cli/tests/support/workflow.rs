//! Structural YAML 1.2 reader and closed-form admission for GitHub Actions
//! workflow files (#1396, #1417, operator ruling 2026-11-27).
//!
//! `tests/it.rs` declares this once so every guard test asking a workflow a
//! structural question reads one typed document parsed by `yaml-rust2` rather
//! than scanning lines or indentation (decision 0071 rulings 3 and 5).
//!
//! Covered workflow guards (`ci.yml`, `macos-test-shards.yml`,
//! `macos-tests.yml`, `release.yml`) do not emulate GitHub Actions. They admit
//! each covered file once into a typed `ClosedWorkflow` under the closed form
//! documented in `CONTRIBUTING.md` ("Workflow closed form"):
//! - Total admission: every top-level key, trigger, matrix, condition, and
//!   artifact name/pattern in the file is checked statically up front.
//! - Final refusals: admission and evaluation refusals are carried in opaque
//!   types (`ClosedWorkflowError`, `Verdict`) to keep refusal propagation
//!   explicit on evaluation paths.

use std::collections::BTreeMap;

use yaml_rust2::{Yaml, YamlLoader};

#[path = "workflow/closed_form.rs"]
mod closed_form;
#[path = "workflow/condition.rs"]
mod condition;

pub(crate) use closed_form::{
    is_plain_artifact_literal, ArtifactName, ArtifactPattern, ClosedJob, ClosedStep,
    ClosedWorkflow, EventExpectation,
};
pub(crate) use condition::{
    ClosedCondition, ClosedFormRefusal, ClosedWorkflowError, CondVal, ConditionContext,
    RefusalKind, Verdict,
};

/// Error parsing or structurally validating a workflow document (decision 0071 ruling 8).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum WorkflowError {
    #[error("YAML parse error: {0}")]
    Parse(String),
    #[error("expected 1 YAML document, found {0}")]
    DocumentCount(usize),
    #[error("workflow contains an unreadable YAML value")]
    BadValue,
    #[error("workflow uses a YAML merge key `<<`, which is refused")]
    MergeKey,
    #[error("{0} must be a YAML mapping")]
    NotAMapping(&'static str),
    #[error("{0} must be a YAML sequence")]
    NotASequence(&'static str),
    #[error("{0} has a non-string key")]
    NonStringKey(&'static str),
    #[error("{0} must be a string scalar")]
    NotAString(&'static str),
    #[error("{ctx} duplicates input `{key}` (case-insensitive)")]
    DuplicateInput { ctx: &'static str, key: String },
    #[error("{file} has no {id} job")]
    MissingJob { file: String, id: String },
}

/// Typed YAML scalar that keeps booleans, integers, strings, null, and complex values distinct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Scalar {
    String(String),
    Bool(bool),
    Int(i64),
    Real(String),
    Null,
    Complex,
}

pub(crate) fn continue_on_error_allows_failure(val: Option<&Scalar>) -> bool {
    !matches!(val, None | Some(Scalar::Bool(false)))
}

impl Scalar {
    fn from_yaml(yaml: &Yaml) -> Self {
        match yaml {
            Yaml::String(s) => Self::String(s.clone()),
            Yaml::Boolean(b) => Self::Bool(*b),
            Yaml::Integer(n) => Self::Int(*n),
            Yaml::Real(r) => Self::Real(r.clone()),
            Yaml::Null => Self::Null,
            Yaml::Array(_) | Yaml::Hash(_) | Yaml::Alias(_) | Yaml::BadValue => Self::Complex,
        }
    }

    pub(crate) fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Single-line literal string, rejecting `${{ ... }}` expressions, empty
    /// strings, and embedded newlines.
    pub(crate) fn as_literal(&self) -> Option<&str> {
        self.as_str()
            .filter(|s| !s.is_empty() && !s.contains("${{") && !s.contains(['\n', '\r']))
    }

    /// Strict YAML boolean (`true`/`false`), rejecting quoted strings.
    pub(crate) fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub(crate) fn is_positive_int(&self) -> bool {
        matches!(self, Self::Int(n) if *n > 0)
    }

    pub(crate) fn as_text(&self) -> Option<String> {
        match self {
            Self::String(s) => Some(s.clone()),
            Self::Bool(b) => Some(b.to_string()),
            Self::Int(n) => Some(n.to_string()),
            Self::Real(r) => Some(r.clone()),
            Self::Null | Self::Complex => None,
        }
    }

    pub(crate) fn kind_label(&self) -> &'static str {
        match self {
            Self::String(_) => "string",
            Self::Bool(_) => "boolean",
            Self::Int(_) => "integer",
            Self::Real(_) => "number",
            Self::Null => "null",
            Self::Complex => "non-scalar YAML sequence or mapping",
        }
    }
}

/// Action or reusable-workflow `with:` inputs with ASCII case-insensitive keys.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Inputs {
    entries: BTreeMap<String, Scalar>,
}

impl Inputs {
    pub(crate) fn get(&self, key: &str) -> Option<&Scalar> {
        self.entries.get(&key.to_ascii_lowercase())
    }

    pub(crate) fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }
}

/// Result of comparing a parsed `uses:` reference against `<owner>/<repo>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ActionMatch {
    Exact,
    SubAction(String),
    Other,
}

/// Whether `uses` references `expected_repo@<ref>` or `expected_repo/<subpath>@<ref>`.
pub(crate) fn action_repo_match(uses: &str, expected_repo: &str) -> Option<ActionMatch> {
    if uses.is_empty() || uses.contains("${{") || uses.contains(['\n', '\r']) {
        return None;
    }
    let repo = uses
        .split_once('@')
        .map_or(uses, |(r, _)| r)
        .to_ascii_lowercase();
    let expected = expected_repo.to_ascii_lowercase();
    if repo == expected {
        Some(ActionMatch::Exact)
    } else if repo.starts_with(&format!("{expected}/")) {
        Some(ActionMatch::SubAction(repo))
    } else {
        Some(ActionMatch::Other)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Permissions {
    All(String),
    Scopes(BTreeMap<String, String>),
}

impl Permissions {
    pub(crate) fn grants(&self, scope: &str, level: &str) -> bool {
        match self {
            Self::All(all) => all == "write-all" || (level == "read" && all == "read-all"),
            Self::Scopes(map) => map
                .get(scope)
                .is_some_and(|v| v == level || (level == "read" && v == "write")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Concurrency {
    pub(crate) group: Option<Scalar>,
    pub(crate) cancel_in_progress: Option<Scalar>,
    pub(crate) extra_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowInput {
    pub(crate) input_type: Option<Scalar>,
    pub(crate) default: Option<Scalar>,
    pub(crate) extra_keys: Vec<String>,
}

/// The one home for the closed-form trigger filter vocabulary (decision 0071 ruling 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TriggerFilterKey {
    Branches,
    Tags,
    Types,
}

impl TriggerFilterKey {
    pub(super) const ALL: [Self; 3] = [Self::Branches, Self::Tags, Self::Types];

    pub(super) fn parse(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_str() == key)
    }

    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Branches => "branches",
            Self::Tags => "tags",
            Self::Types => "types",
        }
    }
}

/// Out-of-form trigger shape recorded during non-strict parsing so closed-form
/// admission reports a typed refusal naming the exact place under `on:`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TriggerShapeIssue {
    pub(crate) place: String,
    pub(crate) detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EventTrigger {
    pub(crate) name: String,
    pub(crate) has_types_key: bool,
    pub(crate) has_inputs_key: bool,
    pub(crate) shape_issues: Vec<TriggerShapeIssue>,
    pub(crate) types: Vec<String>,
    pub(crate) filters: BTreeMap<String, Vec<String>>,
    pub(crate) crons: Vec<String>,
    pub(crate) inputs: Vec<(String, WorkflowInput)>,
    pub(crate) extra_keys: Vec<String>,
}

impl EventTrigger {
    /// True when the event carries no path, branch, tag, workflow, or unknown filter.
    pub(crate) fn is_unfiltered(&self) -> bool {
        self.filters.is_empty() && self.extra_keys.is_empty()
    }

    /// True when this is a `schedule` trigger with at least one 5-field cron
    /// entry and no unsupported sibling keys.
    pub(crate) fn has_active_schedule(&self) -> bool {
        self.name == "schedule"
            && self.extra_keys.is_empty()
            && !self.crons.is_empty()
            && self
                .crons
                .iter()
                .all(|cron| cron.split_whitespace().count() == 5)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RunsOn {
    Single(String),
    Labels(Vec<String>),
}

impl RunsOn {
    pub(crate) fn as_str(&self) -> Option<&str> {
        match self {
            Self::Single(s) => Some(s.as_str()),
            Self::Labels(_) => None,
        }
    }

    pub(crate) fn contains(&self, needle: &str) -> bool {
        match self {
            Self::Single(s) => s.contains(needle),
            Self::Labels(labels) => labels.iter().any(|l| l.contains(needle)),
        }
    }
}

/// Presence and row-parse state for `strategy.matrix.include` or `exclude`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum MatrixClauseState {
    #[default]
    Absent,
    Rows,
    PresentUnread,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Matrix {
    pub(crate) axes: BTreeMap<String, Vec<Scalar>>,
    pub(crate) include: Vec<BTreeMap<String, Scalar>>,
    pub(crate) exclude: Vec<BTreeMap<String, Scalar>>,
    pub(crate) include_clause: MatrixClauseState,
    pub(crate) exclude_clause: MatrixClauseState,
    pub(crate) non_mapping: bool,
}

impl Matrix {
    pub(crate) fn axis_strings(&self, axis: &str) -> Option<Vec<&str>> {
        self.axes.get(axis)?.iter().map(Scalar::as_str).collect()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Step {
    pub(crate) id: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) if_cond: Option<String>,
    pub(crate) if_raw: Option<Scalar>,
    pub(crate) uses: Option<String>,
    pub(crate) run: Option<String>,
    pub(crate) with: Option<Inputs>,
    pub(crate) env: Vec<(String, String)>,
    pub(crate) continue_on_error: Option<Scalar>,
}

impl Step {
    pub(crate) fn env_value(&self, key: &str) -> Option<&str> {
        self.env
            .iter()
            .find_map(|(k, v)| (k == key).then_some(v.as_str()))
    }

    pub(crate) fn with_value(&self, key: &str) -> Option<&Scalar> {
        self.with.as_ref()?.get(key)
    }

    pub(crate) fn with_str(&self, key: &str) -> Option<&str> {
        self.with_value(key)?.as_str()
    }

    pub(crate) fn with_literal(&self, key: &str) -> Option<&str> {
        self.with_value(key)?.as_literal()
    }

    pub(crate) fn action_match(&self, expected_repo: &str) -> Option<ActionMatch> {
        action_repo_match(self.uses.as_deref()?, expected_repo)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Job {
    pub(crate) id: String,
    pub(crate) keys: Vec<String>,
    pub(crate) name: Option<String>,
    pub(crate) if_cond: Option<String>,
    pub(crate) if_raw: Option<Scalar>,
    pub(crate) needs: Vec<String>,
    pub(crate) outputs: Vec<String>,
    pub(crate) runs_on: Option<RunsOn>,
    pub(crate) timeout_minutes: Option<Scalar>,
    pub(crate) uses: Option<String>,
    pub(crate) with: Option<Inputs>,
    pub(crate) env: Vec<(String, String)>,
    pub(crate) permissions: Option<Permissions>,
    pub(crate) matrix: Option<Matrix>,
    pub(crate) steps: Vec<Step>,
    pub(crate) continue_on_error: Option<Scalar>,
    pub(crate) all_scalars: Vec<String>,
}

impl Job {
    pub(crate) fn is_reusable_caller(&self) -> bool {
        self.uses.is_some() && self.runs_on.is_none()
    }

    pub(crate) fn has_positive_timeout(&self) -> bool {
        self.timeout_minutes
            .as_ref()
            .is_some_and(Scalar::is_positive_int)
    }

    pub(crate) fn all_conditions(&self) -> Vec<&str> {
        self.if_cond
            .as_deref()
            .into_iter()
            .chain(self.steps.iter().filter_map(|s| s.if_cond.as_deref()))
            .collect()
    }

    pub(crate) fn has_continue_on_error(&self) -> bool {
        self.continue_on_error.is_some() || self.steps.iter().any(|s| s.continue_on_error.is_some())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Workflow {
    pub(crate) top_level_keys: Vec<String>,
    pub(crate) events: Vec<EventTrigger>,
    pub(crate) event_shape_issues: Vec<TriggerShapeIssue>,
    pub(crate) permissions: Option<Permissions>,
    pub(crate) concurrency: Option<Concurrency>,
    pub(crate) jobs: Vec<Job>,
}

impl Workflow {
    /// Parse a workflow YAML document, failing closed on syntax errors,
    /// duplicate keys, merge keys (`<<`), unresolved aliases, or bad shapes.
    pub(crate) fn parse(text: &str) -> Result<Self, WorkflowError> {
        parse_workflow_raw(text, true)
    }

    pub(crate) fn event_names(&self) -> Vec<&str> {
        self.events.iter().map(|e| e.name.as_str()).collect()
    }

    pub(crate) fn event(&self, name: &str) -> Option<&EventTrigger> {
        self.events.iter().find(|e| e.name == name)
    }

    pub(crate) fn unfiltered_event(&self, name: &str) -> Option<&EventTrigger> {
        self.event(name).filter(|e| e.is_unfiltered())
    }

    pub(crate) fn workflow_call_input(&self, name: &str) -> Option<&WorkflowInput> {
        self.event("workflow_call")?
            .inputs
            .iter()
            .find_map(|(k, v)| k.eq_ignore_ascii_case(name).then_some(v))
    }

    pub(crate) fn job(&self, id: &str) -> Option<&Job> {
        self.jobs.iter().find(|job| job.id == id)
    }

    pub(crate) fn require_job(&self, file: &str, id: &str) -> Result<&Job, WorkflowError> {
        self.job(id).ok_or_else(|| WorkflowError::MissingJob {
            file: file.to_string(),
            id: id.to_string(),
        })
    }

    pub(crate) fn grants_permission(&self, scope: &str, level: &str) -> bool {
        self.permissions
            .as_ref()
            .is_some_and(|p| p.grants(scope, level))
            || self.jobs.iter().any(|j| {
                j.permissions
                    .as_ref()
                    .is_some_and(|p| p.grants(scope, level))
            })
    }
}

pub(super) fn parse_workflow_raw(text: &str, strict_if: bool) -> Result<Workflow, WorkflowError> {
    let docs = YamlLoader::load_from_str(text).map_err(|e| WorkflowError::Parse(e.to_string()))?;
    if docs.len() != 1 {
        return Err(WorkflowError::DocumentCount(docs.len()));
    }
    reject_unsafe_yaml(&docs[0])?;
    let root = expect_map(&docs[0], "workflow root")?;
    let (events, event_shape_issues) = match map_get(&root, "on") {
        Some(n) => parse_events(n, strict_if)?,
        None => (Vec::new(), Vec::new()),
    };
    Ok(Workflow {
        top_level_keys: root.iter().map(|(k, _)| (*k).to_string()).collect(),
        events,
        event_shape_issues,
        permissions: map_get(&root, "permissions")
            .map(parse_permissions)
            .transpose()?,
        concurrency: map_get(&root, "concurrency")
            .map(parse_concurrency)
            .transpose()?,
        jobs: map_get(&root, "jobs")
            .map(|n| parse_jobs(n, strict_if))
            .transpose()?
            .unwrap_or_default(),
    })
}

type Entries<'a> = Vec<(&'a str, &'a Yaml)>;

fn reject_unsafe_yaml(node: &Yaml) -> Result<(), WorkflowError> {
    match node {
        Yaml::BadValue | Yaml::Alias(_) => Err(WorkflowError::BadValue),
        Yaml::Array(items) => items.iter().try_for_each(reject_unsafe_yaml),
        Yaml::Hash(map) => map.iter().try_for_each(|(k, v)| {
            if k.as_str() == Some("<<") {
                return Err(WorkflowError::MergeKey);
            }
            reject_unsafe_yaml(k)?;
            reject_unsafe_yaml(v)
        }),
        _ => Ok(()),
    }
}

fn expect_map<'a>(node: &'a Yaml, ctx: &'static str) -> Result<Entries<'a>, WorkflowError> {
    let Yaml::Hash(map) = node else {
        return Err(WorkflowError::NotAMapping(ctx));
    };
    map.iter()
        .map(|(k, v)| Ok((k.as_str().ok_or(WorkflowError::NonStringKey(ctx))?, v)))
        .collect()
}

fn map_get<'a>(entries: &[(&'a str, &'a Yaml)], key: &str) -> Option<&'a Yaml> {
    entries.iter().find_map(|&(k, v)| (k == key).then_some(v))
}

fn expect_string(node: &Yaml, ctx: &'static str) -> Result<String, WorkflowError> {
    node.as_str()
        .map(str::to_string)
        .ok_or(WorkflowError::NotAString(ctx))
}

fn opt_str(
    entries: &[(&str, &Yaml)],
    key: &str,
    ctx: &'static str,
) -> Result<Option<String>, WorkflowError> {
    map_get(entries, key)
        .map(|v| expect_string(v, ctx))
        .transpose()
}

fn parse_if_field(
    entries: &[(&str, &Yaml)],
    ctx: &'static str,
    strict_if: bool,
) -> Result<(Option<String>, Option<Scalar>), WorkflowError> {
    let Some(node) = map_get(entries, "if") else {
        return Ok((None, None));
    };
    let raw = Scalar::from_yaml(node);
    let text = raw.as_text();
    if strict_if && text.is_none() {
        return Err(WorkflowError::NotAString(ctx));
    }
    Ok((text, Some(raw)))
}

fn expect_scalar_map(
    node: &Yaml,
    ctx: &'static str,
) -> Result<BTreeMap<String, Scalar>, WorkflowError> {
    Ok(expect_map(node, ctx)?
        .into_iter()
        .map(|(k, v)| (k.to_string(), Scalar::from_yaml(v)))
        .collect())
}

fn parse_env(node: &Yaml) -> Result<Vec<(String, String)>, WorkflowError> {
    expect_map(node, "env")?
        .into_iter()
        .map(|(k, v)| {
            Ok((
                k.to_string(),
                Scalar::from_yaml(v)
                    .as_text()
                    .ok_or(WorkflowError::NotAString("env value"))?,
            ))
        })
        .collect()
}

fn parse_permissions(node: &Yaml) -> Result<Permissions, WorkflowError> {
    if let Yaml::String(all) = node {
        return Ok(Permissions::All(all.clone()));
    }
    let scopes = expect_map(node, "permissions")?
        .into_iter()
        .map(|(k, v)| Ok((k.to_string(), expect_string(v, "permissions level")?)))
        .collect::<Result<_, WorkflowError>>()?;
    Ok(Permissions::Scopes(scopes))
}

fn parse_concurrency(node: &Yaml) -> Result<Concurrency, WorkflowError> {
    if let Yaml::String(_) = node {
        return Ok(Concurrency {
            group: Some(Scalar::from_yaml(node)),
            cancel_in_progress: None,
            extra_keys: Vec::new(),
        });
    }
    let e = expect_map(node, "concurrency")?;
    let extra_keys = e
        .iter()
        .filter(|&&(k, _)| !matches!(k, "group" | "cancel-in-progress"))
        .map(|&(k, _)| k.to_string())
        .collect();
    Ok(Concurrency {
        group: map_get(&e, "group").map(Scalar::from_yaml),
        cancel_in_progress: map_get(&e, "cancel-in-progress").map(Scalar::from_yaml),
        extra_keys,
    })
}

fn parse_inputs(node: &Yaml, ctx: &'static str) -> Result<Inputs, WorkflowError> {
    let mut entries = BTreeMap::new();
    for (k, v) in expect_map(node, ctx)? {
        let lower = k.to_ascii_lowercase();
        if entries
            .insert(lower.clone(), Scalar::from_yaml(v))
            .is_some()
        {
            return Err(WorkflowError::DuplicateInput { ctx, key: lower });
        }
    }
    Ok(Inputs { entries })
}

fn empty_trigger(name: String) -> EventTrigger {
    EventTrigger {
        name,
        has_types_key: false,
        has_inputs_key: false,
        shape_issues: Vec::new(),
        types: Vec::new(),
        filters: BTreeMap::new(),
        crons: Vec::new(),
        inputs: Vec::new(),
        extra_keys: Vec::new(),
    }
}

fn shape_issue(place: impl Into<String>, detail: impl Into<String>) -> TriggerShapeIssue {
    TriggerShapeIssue {
        place: place.into(),
        detail: detail.into(),
    }
}

fn parse_events(
    node: &Yaml,
    strict: bool,
) -> Result<(Vec<EventTrigger>, Vec<TriggerShapeIssue>), WorkflowError> {
    match node {
        Yaml::String(name) => Ok((vec![empty_trigger(name.clone())], Vec::new())),
        Yaml::Array(items) => parse_event_list(items, strict),
        Yaml::Hash(map) => parse_event_map(map, strict),
        _ if strict => Err(WorkflowError::NotAMapping("on")),
        _ => Ok((
            Vec::new(),
            vec![shape_issue(
                "on",
                format!("{} `on` trigger", Scalar::from_yaml(node).kind_label()),
            )],
        )),
    }
}

fn parse_event_list(
    items: &[Yaml],
    strict: bool,
) -> Result<(Vec<EventTrigger>, Vec<TriggerShapeIssue>), WorkflowError> {
    let mut triggers = Vec::with_capacity(items.len());
    let mut issues = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        if let Some(name) = item.as_str() {
            triggers.push(empty_trigger(name.to_string()));
        } else if strict {
            return Err(WorkflowError::NotAString("on event"));
        } else {
            issues.push(shape_issue(
                format!("on[{idx}]"),
                format!(
                    "{} event entry in `on` list",
                    Scalar::from_yaml(item).kind_label()
                ),
            ));
        }
    }
    Ok((triggers, issues))
}

fn parse_event_map(
    map: &yaml_rust2::yaml::Hash,
    strict: bool,
) -> Result<(Vec<EventTrigger>, Vec<TriggerShapeIssue>), WorkflowError> {
    let mut triggers = Vec::with_capacity(map.len());
    let mut issues = Vec::new();
    for (k, v) in map {
        let Some(name) = k.as_str() else {
            if strict {
                return Err(WorkflowError::NonStringKey("on"));
            }
            issues.push(shape_issue("on", "non-string event key under `on`"));
            continue;
        };
        triggers.push(parse_event_trigger(name, v, strict)?);
    }
    Ok((triggers, issues))
}

fn parse_schedule_entry(
    idx: usize,
    item: &Yaml,
    strict: bool,
    trigger: &mut EventTrigger,
) -> Result<(), WorkflowError> {
    let place = format!("on.schedule[{idx}]");
    let Yaml::Hash(map) = item else {
        if strict {
            return Err(WorkflowError::NotAMapping("on.schedule entry"));
        }
        trigger.shape_issues.push(shape_issue(
            place,
            format!("{} `schedule` entry", Scalar::from_yaml(item).kind_label()),
        ));
        return Ok(());
    };
    let mut cron_val: Option<&Yaml> = None;
    let mut row_extra = false;
    for (k, v) in map {
        let Some(key) = k.as_str() else {
            if strict {
                return Err(WorkflowError::NonStringKey("on.schedule entry"));
            }
            trigger
                .shape_issues
                .push(shape_issue(&place, "non-string key in `schedule` entry"));
            continue;
        };
        if key == "cron" {
            cron_val = Some(v);
        } else {
            row_extra = true;
            trigger.extra_keys.push(key.to_string());
        }
    }
    match cron_val {
        Some(Yaml::String(s)) => trigger.crons.push(s.clone()),
        Some(_) if strict => return Err(WorkflowError::NotAString("on.schedule.cron")),
        Some(other) => trigger.shape_issues.push(shape_issue(
            place,
            format!(
                "{} `cron` value in `schedule` entry",
                Scalar::from_yaml(other).kind_label()
            ),
        )),
        None if !row_extra => {
            if strict {
                return Err(WorkflowError::NotAString("on.schedule.cron"));
            }
            trigger
                .shape_issues
                .push(shape_issue(place, "empty `schedule` entry missing `cron`"));
        }
        None => {}
    }
    Ok(())
}

fn parse_schedule_entries(
    config: &Yaml,
    strict: bool,
    trigger: &mut EventTrigger,
) -> Result<(), WorkflowError> {
    if matches!(config, Yaml::Null) {
        if !strict {
            trigger
                .shape_issues
                .push(shape_issue("on.schedule", "null `schedule` trigger"));
        }
        return Ok(());
    }
    let Yaml::Array(items) = config else {
        if strict {
            return Err(WorkflowError::NotASequence("on.schedule"));
        }
        trigger
            .shape_issues
            .push(shape_issue("on.schedule", "non-list `schedule` trigger"));
        return Ok(());
    };
    for (idx, item) in items.iter().enumerate() {
        parse_schedule_entry(idx, item, strict, trigger)?;
    }
    Ok(())
}

fn parse_workflow_input_def(
    event_name: &str,
    input_name: &str,
    def: &Yaml,
    strict: bool,
    issues: &mut Vec<TriggerShapeIssue>,
) -> Result<Option<WorkflowInput>, WorkflowError> {
    let place = format!("on.{event_name}.inputs.{input_name}");
    let Yaml::Hash(map) = def else {
        if strict {
            return Err(WorkflowError::NotAMapping("workflow input"));
        }
        issues.push(shape_issue(
            place,
            format!(
                "{} input definition for `inputs.{input_name}`",
                Scalar::from_yaml(def).kind_label()
            ),
        ));
        return Ok(None);
    };
    let mut entries = Vec::with_capacity(map.len());
    for (ik, iv) in map {
        let Some(key) = ik.as_str() else {
            if strict {
                return Err(WorkflowError::NonStringKey("workflow input"));
            }
            issues.push(shape_issue(
                &place,
                format!("non-string key in input definition `inputs.{input_name}`"),
            ));
            return Ok(None);
        };
        entries.push((key, iv));
    }
    let extra_keys = entries
        .iter()
        .filter(|&&(ik, _)| {
            !matches!(
                ik,
                "description" | "type" | "required" | "default" | "options"
            )
        })
        .map(|&(ik, _)| ik.to_string())
        .collect();
    Ok(Some(WorkflowInput {
        input_type: map_get(&entries, "type").map(Scalar::from_yaml),
        default: map_get(&entries, "default").map(Scalar::from_yaml),
        extra_keys,
    }))
}

fn parse_workflow_inputs(
    event_name: &str,
    node: &Yaml,
    strict: bool,
    issues: &mut Vec<TriggerShapeIssue>,
) -> Result<Vec<(String, WorkflowInput)>, WorkflowError> {
    let place = format!("on.{event_name}.inputs");
    let Yaml::Hash(map) = node else {
        if strict {
            return Err(WorkflowError::NotAMapping("on.<event>.inputs"));
        }
        issues.push(shape_issue(
            place,
            format!(
                "{} `inputs` value on `{event_name}`",
                Scalar::from_yaml(node).kind_label()
            ),
        ));
        return Ok(Vec::new());
    };
    let mut inputs = Vec::new();
    let mut seen = BTreeMap::new();
    for (k, def) in map {
        let Some(name) = k.as_str() else {
            if strict {
                return Err(WorkflowError::NonStringKey("on.<event>.inputs"));
            }
            issues.push(shape_issue(
                &place,
                format!("non-string key in `inputs` on `{event_name}`"),
            ));
            continue;
        };
        let lower = name.to_ascii_lowercase();
        if seen.insert(lower.clone(), ()).is_some() {
            if strict {
                return Err(WorkflowError::DuplicateInput {
                    ctx: "on.<event>.inputs",
                    key: lower,
                });
            }
            issues.push(shape_issue(
                &place,
                format!("duplicate input `{lower}` on `{event_name}`"),
            ));
            continue;
        }
        if let Some(parsed) = parse_workflow_input_def(event_name, name, def, strict, issues)? {
            inputs.push((name.to_string(), parsed));
        }
    }
    Ok(inputs)
}

fn parse_trigger_filter_values(
    event_name: &str,
    key: &str,
    node: &Yaml,
    strict: bool,
    ctx: &'static str,
    issues: &mut Vec<TriggerShapeIssue>,
) -> Result<Vec<String>, WorkflowError> {
    if strict {
        return parse_string_or_list(node, ctx);
    }
    let place = format!("on.{event_name}.{key}");
    match node {
        Yaml::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                if let Some(s) = item.as_str() {
                    out.push(s.to_string());
                } else {
                    issues.push(shape_issue(
                        &place,
                        format!(
                            "{} item in `{key}` filter on `{event_name}`",
                            Scalar::from_yaml(item).kind_label()
                        ),
                    ));
                }
            }
            Ok(out)
        }
        _ => {
            issues.push(shape_issue(
                place,
                format!(
                    "{} `{key}` filter value on `{event_name}`",
                    Scalar::from_yaml(node).kind_label()
                ),
            ));
            Ok(Vec::new())
        }
    }
}

fn parse_event_trigger(
    name: &str,
    config: &Yaml,
    strict: bool,
) -> Result<EventTrigger, WorkflowError> {
    let mut trigger = empty_trigger(name.to_string());
    if name == "schedule" {
        parse_schedule_entries(config, strict, &mut trigger)?;
        return Ok(trigger);
    }
    if matches!(config, Yaml::Null) {
        return Ok(trigger);
    }
    let Yaml::Hash(map) = config else {
        if strict {
            return Err(WorkflowError::NotAMapping("on.<event>"));
        }
        trigger.shape_issues.push(shape_issue(
            format!("on.{name}"),
            format!(
                "{} configuration on `{name}`",
                Scalar::from_yaml(config).kind_label()
            ),
        ));
        return Ok(trigger);
    };
    for (raw_k, v) in map {
        let Some(k) = raw_k.as_str() else {
            if strict {
                return Err(WorkflowError::NonStringKey("on.<event>"));
            }
            trigger.shape_issues.push(shape_issue(
                format!("on.{name}"),
                format!("non-string key on `{name}`"),
            ));
            continue;
        };
        if k == "inputs" {
            trigger.has_inputs_key = true;
            if strict || name == "workflow_call" {
                trigger.inputs = parse_workflow_inputs(name, v, strict, &mut trigger.shape_issues)?;
            }
            continue;
        }
        match TriggerFilterKey::parse(k) {
            Some(TriggerFilterKey::Types) => {
                trigger.has_types_key = true;
                trigger.types = parse_trigger_filter_values(
                    name,
                    k,
                    v,
                    strict,
                    "on.<event>.types",
                    &mut trigger.shape_issues,
                )?;
            }
            Some(TriggerFilterKey::Branches | TriggerFilterKey::Tags) => {
                let vals = parse_trigger_filter_values(
                    name,
                    k,
                    v,
                    strict,
                    "on.<event> filter",
                    &mut trigger.shape_issues,
                )?;
                trigger.filters.insert(k.to_string(), vals);
            }
            None => trigger.extra_keys.push(k.to_string()),
        }
    }
    Ok(trigger)
}

fn parse_string_or_list(node: &Yaml, ctx: &'static str) -> Result<Vec<String>, WorkflowError> {
    match node {
        Yaml::String(s) => Ok(vec![s.clone()]),
        Yaml::Array(items) => items.iter().map(|i| expect_string(i, ctx)).collect(),
        _ => Err(WorkflowError::NotASequence(ctx)),
    }
}

fn parse_runs_on(node: &Yaml) -> Result<RunsOn, WorkflowError> {
    match node {
        Yaml::String(s) => Ok(RunsOn::Single(s.clone())),
        Yaml::Array(items) => Ok(RunsOn::Labels(
            items
                .iter()
                .map(|i| expect_string(i, "runs-on label"))
                .collect::<Result<_, _>>()?,
        )),
        _ => Err(WorkflowError::NotAString("runs-on")),
    }
}

fn parse_matrix_rows(
    node: &Yaml,
    seq: &'static str,
    row: &'static str,
) -> Result<Vec<BTreeMap<String, Scalar>>, WorkflowError> {
    let Yaml::Array(rows) = node else {
        return Err(WorkflowError::NotASequence(seq));
    };
    rows.iter().map(|r| expect_scalar_map(r, row)).collect()
}

fn parse_matrix_clause(
    node: &Yaml,
    strict: bool,
    seq: &'static str,
    row: &'static str,
) -> Result<(MatrixClauseState, Vec<BTreeMap<String, Scalar>>), WorkflowError> {
    match parse_matrix_rows(node, seq, row) {
        Ok(rows) => Ok((MatrixClauseState::Rows, rows)),
        Err(err) if strict => Err(err),
        Err(_) => Ok((MatrixClauseState::PresentUnread, Vec::new())),
    }
}

fn parse_matrix(node: &Yaml, strict_if: bool) -> Result<Matrix, WorkflowError> {
    if !matches!(node, Yaml::Hash(_)) {
        if strict_if {
            return Err(WorkflowError::NotAMapping("strategy.matrix"));
        }
        return Ok(Matrix {
            non_mapping: true,
            ..Matrix::default()
        });
    }
    let mut matrix = Matrix::default();
    for (k, v) in expect_map(node, "strategy.matrix")? {
        match k {
            "include" => {
                let (state, rows) = parse_matrix_clause(
                    v,
                    strict_if,
                    "strategy.matrix.include",
                    "strategy.matrix.include row",
                )?;
                matrix.include_clause = state;
                matrix.include = rows;
            }
            "exclude" => {
                let (state, rows) = parse_matrix_clause(
                    v,
                    strict_if,
                    "strategy.matrix.exclude",
                    "strategy.matrix.exclude row",
                )?;
                matrix.exclude_clause = state;
                matrix.exclude = rows;
            }
            _ => {
                let Yaml::Array(items) = v else {
                    return Err(WorkflowError::NotASequence("strategy.matrix axis"));
                };
                matrix
                    .axes
                    .insert(k.to_string(), items.iter().map(Scalar::from_yaml).collect());
            }
        }
    }
    Ok(matrix)
}

fn parse_step(node: &Yaml, strict_if: bool) -> Result<Step, WorkflowError> {
    let e = expect_map(node, "step")?;
    let (if_cond, if_raw) = parse_if_field(&e, "step.if", strict_if)?;
    Ok(Step {
        id: opt_str(&e, "id", "step.id")?,
        name: opt_str(&e, "name", "step.name")?,
        if_cond,
        if_raw,
        uses: opt_str(&e, "uses", "step.uses")?,
        run: opt_str(&e, "run", "step.run")?,
        with: map_get(&e, "with")
            .map(|v| parse_inputs(v, "step.with"))
            .transpose()?,
        env: map_get(&e, "env")
            .map(parse_env)
            .transpose()?
            .unwrap_or_default(),
        continue_on_error: map_get(&e, "continue-on-error").map(Scalar::from_yaml),
    })
}

fn parse_jobs(node: &Yaml, strict_if: bool) -> Result<Vec<Job>, WorkflowError> {
    expect_map(node, "jobs")?
        .into_iter()
        .map(|(id, n)| parse_job(id, n, strict_if))
        .collect()
}

fn parse_job_outputs(node: &Yaml) -> Result<Vec<String>, WorkflowError> {
    Ok(expect_map(node, "job.outputs")?
        .into_iter()
        .map(|(k, _)| k.to_string())
        .collect())
}

fn parse_job(id: &str, node: &Yaml, strict_if: bool) -> Result<Job, WorkflowError> {
    let e = expect_map(node, "job")?;
    let matrix = match map_get(&e, "strategy") {
        Some(s) => map_get(&expect_map(s, "job.strategy")?, "matrix")
            .map(|m| parse_matrix(m, strict_if))
            .transpose()?,
        None => None,
    };
    let steps = match map_get(&e, "steps") {
        Some(Yaml::Array(items)) => items
            .iter()
            .map(|s| parse_step(s, strict_if))
            .collect::<Result<_, _>>()?,
        Some(_) => return Err(WorkflowError::NotASequence("steps")),
        None => Vec::new(),
    };
    let (if_cond, if_raw) = parse_if_field(&e, "job.if", strict_if)?;
    let mut all_scalars = vec![id.to_string()];
    collect_scalars(node, &mut all_scalars);
    Ok(Job {
        id: id.to_string(),
        keys: e.iter().map(|(k, _)| (*k).to_string()).collect(),
        name: opt_str(&e, "name", "job.name")?,
        if_cond,
        if_raw,
        needs: map_get(&e, "needs")
            .map(|v| parse_string_or_list(v, "job.needs"))
            .transpose()?
            .unwrap_or_default(),
        outputs: map_get(&e, "outputs")
            .map(parse_job_outputs)
            .transpose()?
            .unwrap_or_default(),
        runs_on: map_get(&e, "runs-on").map(parse_runs_on).transpose()?,
        timeout_minutes: map_get(&e, "timeout-minutes").map(Scalar::from_yaml),
        uses: opt_str(&e, "uses", "job.uses")?,
        with: map_get(&e, "with")
            .map(|v| parse_inputs(v, "job.with"))
            .transpose()?,
        env: map_get(&e, "env")
            .map(parse_env)
            .transpose()?
            .unwrap_or_default(),
        permissions: map_get(&e, "permissions")
            .map(parse_permissions)
            .transpose()?,
        matrix,
        steps,
        continue_on_error: map_get(&e, "continue-on-error").map(Scalar::from_yaml),
        all_scalars,
    })
}

fn collect_scalars(node: &Yaml, out: &mut Vec<String>) {
    match node {
        Yaml::String(s) => out.push(s.clone()),
        Yaml::Array(items) => items.iter().for_each(|i| collect_scalars(i, out)),
        Yaml::Hash(map) => map.iter().for_each(|(k, v)| {
            collect_scalars(k, out);
            collect_scalars(v, out);
        }),
        _ => {}
    }
}
