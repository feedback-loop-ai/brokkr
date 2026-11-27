//! Structural YAML 1.2 reader and closed-form validator for GitHub Actions
//! workflow files (#1396, #1417, operator ruling 2026-11-27).
//!
//! `tests/it.rs` declares this once so every guard test asking a workflow a
//! structural question reads one typed document parsed by `yaml-rust2` rather
//! than scanning lines or indentation (decision 0071 rulings 3 and 5).
//!
//! Covered workflow guards (`ci.yml`, `macos-test-shards.yml`,
//! `macos-tests.yml`, `release.yml`) do not emulate GitHub Actions. They parse
//! and evaluate a small closed form documented once in `CONTRIBUTING.md`
//! ("Workflow closed form"):
//! - `if:` omitted, `always()`, comparisons joined by `&&` (optionally led by
//!   `always() &&`), comparisons joined by `||`, or one bare boolean input or
//!   non-empty string matrix axis, without `${{ }}`; implicit `success()`
//!   applies whenever leading `always()` is absent.
//! - Upload/download `name:` is a plain `[A-Za-z0-9._-]+` literal (no blanks),
//!   `${{ matrix.<axis> }}` over plain literals, or `<literal>${{ strategy.job-index }}`;
//!   download `pattern:` is `<literal>*`.
//! - No workflow-level `env:`; `on:` uses only unfiltered events or `branches`,
//!   `tags`, `types`, `schedule` `cron`, or `workflow_call` `inputs`; unread
//!   matrix `include`/`exclude` clauses are refused.
//! - Anything outside the form fails closed with one sentence naming the file,
//!   the place, `"not a form this guard reads"`, and the allowed forms.

use std::collections::BTreeMap;

use yaml_rust2::{Yaml, YamlLoader};

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

/// Typed YAML scalar that keeps booleans, integers and strings distinct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Scalar {
    String(String),
    Bool(bool),
    Int(i64),
    Real(String),
    Null,
    Complex,
}

fn continue_on_error_allows_failure(val: Option<&Scalar>) -> bool {
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
}

/// Action or reusable-workflow `with:` inputs with ASCII case-insensitive keys.
///
/// GitHub Actions treats action and workflow input names case-insensitively, so
/// every lookup goes through this type and duplicate keys differing only in
/// case fail closed at parse time.
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowInput {
    pub(crate) input_type: Option<Scalar>,
    pub(crate) default: Option<Scalar>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EventTrigger {
    pub(crate) name: String,
    pub(crate) has_types_key: bool,
    pub(crate) types: Vec<String>,
    pub(crate) filters: BTreeMap<String, Vec<String>>,
    pub(crate) crons: Vec<String>,
    pub(crate) inputs: Vec<(String, WorkflowInput)>,
    pub(crate) extra_keys: Vec<String>,
}

impl EventTrigger {
    /// True when the event carries no path, branch, tag, or workflow filter.
    pub(crate) fn is_unfiltered(&self) -> bool {
        self.filters.is_empty()
    }

    /// True when this is a `schedule` trigger with at least one 5-field cron entry.
    pub(crate) fn has_active_schedule(&self) -> bool {
        self.name == "schedule"
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Matrix {
    pub(crate) axes: BTreeMap<String, Vec<Scalar>>,
    pub(crate) include: Vec<BTreeMap<String, Scalar>>,
    pub(crate) exclude: Vec<BTreeMap<String, Scalar>>,
}

impl Matrix {
    pub(crate) fn axis_strings(&self, axis: &str) -> Option<Vec<&str>> {
        self.axes.get(axis)?.iter().map(Scalar::as_str).collect()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Step {
    pub(crate) name: Option<String>,
    pub(crate) if_cond: Option<String>,
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

    pub(crate) fn runs_under(
        &self,
        file: &str,
        job_id: &str,
        step_idx: usize,
        ctx: &ConditionContext,
    ) -> Result<bool, ClosedFormRefusal> {
        let place = format!("job {job_id} step {} if", step_idx + 1);
        ClosedCondition::parse(self.if_cond.as_deref(), file, &place)?.evaluate(ctx, file, &place)
    }

    /// True if this step's `continue-on-error` is anything other than literal `false`.
    pub(crate) fn allows_failure(&self) -> bool {
        continue_on_error_allows_failure(self.continue_on_error.as_ref())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Job {
    pub(crate) id: String,
    pub(crate) keys: Vec<String>,
    pub(crate) name: Option<String>,
    pub(crate) if_cond: Option<String>,
    pub(crate) needs: Vec<String>,
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

    pub(crate) fn with_value(&self, key: &str) -> Option<&Scalar> {
        self.with.as_ref()?.get(key)
    }

    pub(crate) fn has_continue_on_error(&self) -> bool {
        self.continue_on_error.is_some() || self.steps.iter().any(|s| s.continue_on_error.is_some())
    }

    /// True if this job's `continue-on-error` is anything other than literal `false`.
    pub(crate) fn allows_failure(&self) -> bool {
        continue_on_error_allows_failure(self.continue_on_error.as_ref())
    }

    pub(crate) fn runs_under(
        &self,
        file: &str,
        ctx: &ConditionContext,
    ) -> Result<bool, ClosedFormRefusal> {
        let place = format!("job {} if", self.id);
        ClosedCondition::parse(self.if_cond.as_deref(), file, &place)?.evaluate(ctx, file, &place)
    }

    /// True only when both the job (under `job_ctx`) and `step` (under `step_ctx`)
    /// evaluate to `true` and neither allows failure via `continue-on-error`.
    pub(crate) fn step_enforces(
        &self,
        file: &str,
        step: &Step,
        step_idx: usize,
        job_ctx: &ConditionContext,
        step_ctx: &ConditionContext,
    ) -> Result<bool, ClosedFormRefusal> {
        if self.allows_failure() || step.allows_failure() {
            return Ok(false);
        }
        let job_ok = self.runs_under(file, job_ctx)?;
        let step_ok = step.runs_under(file, &self.id, step_idx, step_ctx)?;
        Ok(job_ok && step_ok)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Workflow {
    pub(crate) top_level_keys: Vec<String>,
    pub(crate) events: Vec<EventTrigger>,
    pub(crate) permissions: Option<Permissions>,
    pub(crate) concurrency: Option<Concurrency>,
    pub(crate) jobs: Vec<Job>,
}

impl Workflow {
    /// Parse a workflow YAML document, failing closed on syntax errors,
    /// duplicate keys, merge keys (`<<`), unresolved aliases, or bad shapes.
    pub(crate) fn parse(text: &str) -> Result<Self, WorkflowError> {
        let docs =
            YamlLoader::load_from_str(text).map_err(|e| WorkflowError::Parse(e.to_string()))?;
        if docs.len() != 1 {
            return Err(WorkflowError::DocumentCount(docs.len()));
        }
        reject_unsafe_yaml(&docs[0])?;
        let root = expect_map(&docs[0], "workflow root")?;
        Ok(Self {
            top_level_keys: root.iter().map(|(k, _)| (*k).to_string()).collect(),
            events: map_get(&root, "on")
                .map(parse_events)
                .transpose()?
                .unwrap_or_default(),

            permissions: map_get(&root, "permissions")
                .map(parse_permissions)
                .transpose()?,
            concurrency: map_get(&root, "concurrency")
                .map(parse_concurrency)
                .transpose()?,
            jobs: map_get(&root, "jobs")
                .map(parse_jobs)
                .transpose()?
                .unwrap_or_default(),
        })
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

fn opt_text(
    entries: &[(&str, &Yaml)],
    key: &str,
    ctx: &'static str,
) -> Result<Option<String>, WorkflowError> {
    map_get(entries, key)
        .map(|v| {
            Scalar::from_yaml(v)
                .as_text()
                .ok_or(WorkflowError::NotAString(ctx))
        })
        .transpose()
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
        });
    }
    let e = expect_map(node, "concurrency")?;
    Ok(Concurrency {
        group: map_get(&e, "group").map(Scalar::from_yaml),
        cancel_in_progress: map_get(&e, "cancel-in-progress").map(Scalar::from_yaml),
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
        types: Vec::new(),
        filters: BTreeMap::new(),
        crons: Vec::new(),
        inputs: Vec::new(),
        extra_keys: Vec::new(),
    }
}

fn parse_events(node: &Yaml) -> Result<Vec<EventTrigger>, WorkflowError> {
    match node {
        Yaml::String(name) => Ok(vec![empty_trigger(name.clone())]),
        Yaml::Array(items) => items
            .iter()
            .map(|i| Ok(empty_trigger(expect_string(i, "on event")?)))
            .collect(),
        Yaml::Hash(_) => expect_map(node, "on")?
            .into_iter()
            .map(|(n, c)| parse_event_trigger(n, c))
            .collect(),
        _ => Err(WorkflowError::NotAMapping("on")),
    }
}

fn parse_schedule_crons(config: &Yaml) -> Result<Vec<String>, WorkflowError> {
    if matches!(config, Yaml::Null) {
        return Ok(Vec::new());
    }
    let Yaml::Array(items) = config else {
        return Err(WorkflowError::NotASequence("on.schedule"));
    };
    items
        .iter()
        .map(|item| {
            let e = expect_map(item, "on.schedule entry")?;
            let cron = map_get(&e, "cron").ok_or(WorkflowError::NotAString("on.schedule.cron"))?;
            expect_string(cron, "on.schedule.cron")
        })
        .collect()
}

fn parse_workflow_inputs(node: &Yaml) -> Result<Vec<(String, WorkflowInput)>, WorkflowError> {
    let mut inputs = Vec::new();
    let mut seen = BTreeMap::new();
    for (k, def) in expect_map(node, "on.<event>.inputs")? {
        let lower = k.to_ascii_lowercase();
        if seen.insert(lower.clone(), ()).is_some() {
            return Err(WorkflowError::DuplicateInput {
                ctx: "on.<event>.inputs",
                key: lower,
            });
        }
        let d = expect_map(def, "workflow input")?;
        inputs.push((
            k.to_string(),
            WorkflowInput {
                input_type: map_get(&d, "type").map(Scalar::from_yaml),
                default: map_get(&d, "default").map(Scalar::from_yaml),
            },
        ));
    }
    Ok(inputs)
}

fn parse_event_trigger(name: &str, config: &Yaml) -> Result<EventTrigger, WorkflowError> {
    let mut trigger = empty_trigger(name.to_string());
    if name == "schedule" {
        trigger.crons = parse_schedule_crons(config)?;
        return Ok(trigger);
    }
    if matches!(config, Yaml::Null) {
        return Ok(trigger);
    }
    for (k, v) in expect_map(config, "on.<event>")? {
        match k {
            "types" => {
                trigger.has_types_key = true;
                trigger.types = parse_string_or_list(v, "on.<event>.types")?;
            }
            "inputs" => trigger.inputs = parse_workflow_inputs(v)?,
            "secrets" | "outputs" if name == "workflow_call" => {
                let _ = expect_map(v, "on.workflow_call")?;
                trigger.extra_keys.push(k.to_string());
            }
            _ => {
                trigger
                    .filters
                    .insert(k.to_string(), parse_string_or_list(v, "on.<event> filter")?);
            }
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

fn parse_matrix(node: &Yaml) -> Result<Matrix, WorkflowError> {
    let mut matrix = Matrix::default();
    for (k, v) in expect_map(node, "strategy.matrix")? {
        match k {
            "include" => {
                matrix.include =
                    parse_matrix_rows(v, "strategy.matrix.include", "strategy.matrix.include row")?
            }
            "exclude" => {
                matrix.exclude =
                    parse_matrix_rows(v, "strategy.matrix.exclude", "strategy.matrix.exclude row")?
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

fn parse_step(node: &Yaml) -> Result<Step, WorkflowError> {
    let e = expect_map(node, "step")?;
    Ok(Step {
        name: opt_str(&e, "name", "step.name")?,
        if_cond: opt_text(&e, "if", "step.if")?,
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

fn parse_jobs(node: &Yaml) -> Result<Vec<Job>, WorkflowError> {
    expect_map(node, "jobs")?
        .into_iter()
        .map(|(id, n)| parse_job(id, n))
        .collect()
}

fn parse_job(id: &str, node: &Yaml) -> Result<Job, WorkflowError> {
    let e = expect_map(node, "job")?;
    let matrix = match map_get(&e, "strategy") {
        Some(s) => map_get(&expect_map(s, "job.strategy")?, "matrix")
            .map(parse_matrix)
            .transpose()?,
        None => None,
    };
    let steps = match map_get(&e, "steps") {
        Some(Yaml::Array(items)) => items.iter().map(parse_step).collect::<Result<_, _>>()?,
        Some(_) => return Err(WorkflowError::NotASequence("steps")),
        None => Vec::new(),
    };
    let mut all_scalars = vec![id.to_string()];
    collect_scalars(node, &mut all_scalars);
    Ok(Job {
        id: id.to_string(),
        keys: e.iter().map(|(k, _)| (*k).to_string()).collect(),
        name: opt_str(&e, "name", "job.name")?,
        if_cond: opt_text(&e, "if", "job.if")?,
        needs: map_get(&e, "needs")
            .map(|v| parse_string_or_list(v, "job.needs"))
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

/// Machine-readable category for a closed-form refusal (decision 0071 rulings 2 and 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RefusalKind {
    WorkflowEnv,
    EventFilter,
    ConditionSyntax,
    ConditionBarePath,
    ConditionUnbound,
    ConditionTypeMismatch,
    ArtifactName,
    MatrixClause,
}

/// One-sentence refusal for anything outside the closed form documented in `CONTRIBUTING.md`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{file} ({place}): {detail} is not a form this guard reads; allowed forms: {allowed}.")]
pub(crate) struct ClosedFormRefusal {
    pub(crate) file: String,
    pub(crate) place: String,
    pub(crate) kind: RefusalKind,
    pub(crate) detail: String,
    pub(crate) allowed: &'static str,
}

const ALLOWED_ENV: &str = "job-level or step-level env only (no workflow-level env)";
const ALLOWED_EVENT_FILTERS: &str =
    "unfiltered event, or branches, tags, types, schedule cron, or workflow_call inputs";
const ALLOWED_CONDITIONS: &str = "omitted, always(), comparisons joined by && (optionally prefixed by always() &&), comparisons joined by ||, or one bare boolean input or string matrix axis, without ${{ }}";
const ALLOWED_BARE_PATHS: &str = "bare inputs.<name> declared type: boolean, or bare matrix.<axis> whose rows give a non-empty string or omit it";
const ALLOWED_PATHS: &str = "github.event_name, runner.environment, matrix.<axis>, inputs.<name>, needs.<job>.result, needs.<job>.outputs.<name>, or steps.<id>.outputs.<name> bound in this context";
const ALLOWED_COMPARISONS: &str =
    "a context path known to be a string or absent compared with == or != against a single-quoted literal";
const ALLOWED_ARTIFACT_NAMES: &str = "a plain [A-Za-z0-9._-]+ literal with no blanks, ${{ matrix.<axis> }} over plain literals, <literal>${{ strategy.job-index }}, or for pattern <literal>*";
const ALLOWED_MATRIX: &str = "matrix axes read by this guard, with no unread include or exclude";

impl ClosedFormRefusal {
    fn new(
        file: &str,
        place: impl Into<String>,
        kind: RefusalKind,
        detail: impl Into<String>,
        allowed: &'static str,
    ) -> Self {
        Self {
            file: file.to_string(),
            place: place.into(),
            kind,
            detail: detail.into(),
            allowed,
        }
    }
}

/// Error reading a covered workflow in the closed form.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ClosedWorkflowError {
    #[error("{file} is unreadable: {err}")]
    Parse { file: String, err: WorkflowError },
    #[error("{0}")]
    Refusal(#[from] ClosedFormRefusal),
}

/// Closed set of context paths the closed form allows in `if:` conditions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ContextPath {
    GithubEventName,
    RunnerEnvironment,
    Matrix(String),
    Input(String),
    NeedsResult(String),
    NeedsOutput { job: String, output: String },
    StepOutput { step: String, output: String },
}

impl ContextPath {
    fn display_name(&self) -> String {
        match self {
            Self::GithubEventName => "github.event_name".to_string(),
            Self::RunnerEnvironment => "runner.environment".to_string(),
            Self::Matrix(axis) => format!("matrix.{axis}"),
            Self::Input(name) => format!("inputs.{name}"),
            Self::NeedsResult(job) => format!("needs.{job}.result"),
            Self::NeedsOutput { job, output } => format!("needs.{job}.outputs.{output}"),
            Self::StepOutput { step, output } => format!("steps.{step}.outputs.{output}"),
        }
    }
}

/// Closed set of comparison operators in `if:` conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompareOp {
    Eq,
    Ne,
}

/// One closed-form comparison: `<context path> == '<literal>'` or `!= '<literal>'`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Comparison {
    pub(crate) path: ContextPath,
    pub(crate) op: CompareOp,
    pub(crate) literal: String,
}

/// Body of a closed-form condition after any leading `always()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConditionBody {
    Unconditional,
    All(Vec<Comparison>),
    Any(Vec<Comparison>),
    Bare(ContextPath),
}

/// Typed representation of a closed-form GitHub Actions `if:` condition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClosedCondition {
    raw: String,
    pub(crate) always: bool,
    pub(crate) body: ConditionBody,
}

/// Typed value bound in a `ConditionContext`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CondVal {
    Bool(bool),
    Str(String),
    Absent,
}

/// Bindings available while evaluating a closed-form `if:` condition.
#[derive(Debug, Clone)]
pub(crate) struct ConditionContext {
    prior_succeeded: bool,
    event_name: Option<String>,
    runner_environment: Option<String>,
    matrix: BTreeMap<String, CondVal>,
    inputs: BTreeMap<String, CondVal>,
    needs_results: BTreeMap<String, String>,
    needs_outputs: BTreeMap<(String, String), CondVal>,
    step_outputs: BTreeMap<(String, String), CondVal>,
}

impl ConditionContext {
    pub(crate) fn default_ok() -> Self {
        Self {
            prior_succeeded: true,
            event_name: None,
            runner_environment: None,
            matrix: BTreeMap::new(),
            inputs: BTreeMap::new(),
            needs_results: BTreeMap::new(),
            needs_outputs: BTreeMap::new(),
            step_outputs: BTreeMap::new(),
        }
    }

    pub(crate) fn with_prior_succeeded(mut self, succeeded: bool) -> Self {
        self.prior_succeeded = succeeded;
        self
    }

    pub(crate) fn with_event(mut self, event: &str) -> Self {
        self.event_name = Some(event.to_string());
        self
    }

    pub(crate) fn with_runner_environment(mut self, env: &str) -> Self {
        self.runner_environment = Some(env.to_string());
        self
    }

    pub(crate) fn with_matrix(mut self, key: &str, value: CondVal) -> Self {
        self.matrix.insert(key.to_string(), value);
        self
    }

    pub(crate) fn with_input(mut self, key: &str, value: CondVal) -> Self {
        self.inputs.insert(key.to_ascii_lowercase(), value);
        self
    }

    pub(crate) fn with_needs_result(mut self, job: &str, result: &str) -> Self {
        self.needs_results
            .insert(job.to_string(), result.to_string());
        self
    }

    pub(crate) fn with_needs_output(mut self, job: &str, output: &str, value: CondVal) -> Self {
        self.needs_outputs
            .insert((job.to_string(), output.to_string()), value);
        self
    }

    pub(crate) fn with_step_output(mut self, step: &str, output: &str, value: CondVal) -> Self {
        self.step_outputs
            .insert((step.to_string(), output.to_string()), value);
        self
    }

    fn lookup(&self, path: &ContextPath) -> Option<CondVal> {
        match path {
            ContextPath::GithubEventName => self.event_name.clone().map(CondVal::Str),
            ContextPath::RunnerEnvironment => self.runner_environment.clone().map(CondVal::Str),
            ContextPath::Matrix(axis) => self.matrix.get(axis).cloned(),
            ContextPath::Input(name) => self.inputs.get(&name.to_ascii_lowercase()).cloned(),
            ContextPath::NeedsResult(job) => self.needs_results.get(job).cloned().map(CondVal::Str),
            ContextPath::NeedsOutput { job, output } => self
                .needs_outputs
                .get(&(job.clone(), output.clone()))
                .cloned(),
            ContextPath::StepOutput { step, output } => self
                .step_outputs
                .get(&(step.clone(), output.clone()))
                .cloned(),
        }
    }
}

fn is_ident_segment(seg: &str) -> bool {
    let mut bytes = seg.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
}

fn parse_context_path(raw: &str) -> Option<ContextPath> {
    let parts: Vec<&str> = raw.split('.').collect();
    match parts.as_slice() {
        ["github", "event_name"] => Some(ContextPath::GithubEventName),
        ["runner", "environment"] => Some(ContextPath::RunnerEnvironment),
        ["matrix", axis] if is_ident_segment(axis) => Some(ContextPath::Matrix((*axis).into())),
        ["inputs", name] if is_ident_segment(name) => Some(ContextPath::Input((*name).into())),
        ["needs", job, "result"] if is_ident_segment(job) => {
            Some(ContextPath::NeedsResult((*job).into()))
        }
        ["needs", job, "outputs", output] if is_ident_segment(job) && is_ident_segment(output) => {
            Some(ContextPath::NeedsOutput {
                job: (*job).into(),
                output: (*output).into(),
            })
        }
        ["steps", step, "outputs", output]
            if is_ident_segment(step) && is_ident_segment(output) =>
        {
            Some(ContextPath::StepOutput {
                step: (*step).into(),
                output: (*output).into(),
            })
        }
        _ => None,
    }
}

fn parse_single_quoted_literal(raw: &str) -> Option<String> {
    let inner = raw.strip_prefix('\'')?.strip_suffix('\'')?;
    let bytes = inner.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\'' {
            if bytes.get(i + 1) == Some(&b'\'') {
                out.push('\'');
                i += 2;
                continue;
            }
            return None;
        }
        let ch = inner[i..].chars().next()?;
        out.push(ch);
        i += ch.len_utf8();
    }
    Some(out)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogicOp {
    And,
    Or,
}

fn step_single_quote(bytes: &[u8], i: &mut usize, in_quote: &mut bool) -> bool {
    if bytes[*i] != b'\'' {
        return false;
    }
    if *in_quote && bytes.get(*i + 1) == Some(&b'\'') {
        *i += 2;
    } else {
        *in_quote = !*in_quote;
        *i += 1;
    }
    true
}

fn split_logic_clauses<'a>(
    src: &'a str,
    file: &str,
    place: &str,
    raw: &str,
) -> Result<(Option<LogicOp>, Vec<&'a str>), ClosedFormRefusal> {
    let bytes = src.as_bytes();
    let (mut in_quote, mut op, mut start, mut i) = (false, None, 0usize, 0usize);
    let mut clauses = Vec::new();
    while i < bytes.len() {
        if step_single_quote(bytes, &mut i, &mut in_quote) {
            continue;
        }
        if !in_quote && i + 1 < bytes.len() {
            let found = match (bytes[i], bytes[i + 1]) {
                (b'&', b'&') => Some(LogicOp::And),
                (b'|', b'|') => Some(LogicOp::Or),
                _ => None,
            };
            if let Some(next_op) = found {
                if op.is_some_and(|prev| prev != next_op) {
                    return Err(ClosedFormRefusal::new(
                        file,
                        place,
                        RefusalKind::ConditionSyntax,
                        format!("condition `{raw}` mixes && and ||"),
                        ALLOWED_CONDITIONS,
                    ));
                }
                op = Some(next_op);
                clauses.push(src[start..i].trim());
                i += 2;
                start = i;
                continue;
            }
        }
        i += 1;
    }
    if in_quote {
        return Err(ClosedFormRefusal::new(
            file,
            place,
            RefusalKind::ConditionSyntax,
            format!("condition `{raw}` has an unclosed string literal"),
            ALLOWED_CONDITIONS,
        ));
    }
    clauses.push(src[start..].trim());
    if clauses.iter().any(|c| c.is_empty()) {
        return Err(ClosedFormRefusal::new(
            file,
            place,
            RefusalKind::ConditionSyntax,
            format!("condition `{raw}` has an empty clause"),
            ALLOWED_CONDITIONS,
        ));
    }
    Ok((op, clauses))
}

fn find_comparison_op(clause: &str) -> Result<Option<(usize, CompareOp)>, &'static str> {
    let bytes = clause.as_bytes();
    let (mut in_quote, mut found, mut i) = (false, None, 0usize);
    while i < bytes.len() {
        if step_single_quote(bytes, &mut i, &mut in_quote) {
            continue;
        }
        if !in_quote {
            if matches!(bytes[i], b'(' | b')') {
                return Err("uses parentheses or a function call");
            }
            let op = match (bytes[i], bytes.get(i + 1)) {
                (b'=', Some(b'=')) => Some(CompareOp::Eq),
                (b'!', Some(b'=')) => Some(CompareOp::Ne),
                (b'!', _) => return Err("uses `!` negation"),
                _ => None,
            };
            if let Some(op) = op {
                if found.is_some() {
                    return Err("chains more than one comparison operator");
                }
                found = Some((i, op));
                i += 2;
                continue;
            }
        }
        i += 1;
    }
    Ok(found)
}

fn parse_comparison(
    clause: &str,
    file: &str,
    place: &str,
    raw: &str,
) -> Result<Comparison, ClosedFormRefusal> {
    let bad = |reason: &str| {
        ClosedFormRefusal::new(
            file,
            place,
            RefusalKind::ConditionSyntax,
            format!("condition `{raw}` ({reason})"),
            ALLOWED_CONDITIONS,
        )
    };
    let Some((idx, op)) = find_comparison_op(clause).map_err(bad)? else {
        return Err(bad("clause is not a `<path> == '<literal>'` comparison"));
    };
    let lhs = clause[..idx].trim();
    let rhs = clause[idx + 2..].trim();
    let path = parse_context_path(lhs).ok_or_else(|| bad("unsupported left-hand context path"))?;
    let literal = parse_single_quoted_literal(rhs)
        .ok_or_else(|| bad("right-hand side is not a single-quoted string literal"))?;
    Ok(Comparison { path, op, literal })
}

fn parse_single_clause(
    clause: &str,
    file: &str,
    place: &str,
    raw: &str,
) -> Result<ClosedCondition, ClosedFormRefusal> {
    if clause == "always()" {
        return Ok(ClosedCondition {
            raw: raw.to_string(),
            always: true,
            body: ConditionBody::Unconditional,
        });
    }
    let op = find_comparison_op(clause).map_err(|reason| {
        ClosedFormRefusal::new(
            file,
            place,
            RefusalKind::ConditionSyntax,
            format!("condition `{raw}` ({reason})"),
            ALLOWED_CONDITIONS,
        )
    })?;
    if op.is_some() {
        let cmp = parse_comparison(clause, file, place, raw)?;
        return Ok(ClosedCondition {
            raw: raw.to_string(),
            always: false,
            body: ConditionBody::All(vec![cmp]),
        });
    }
    let Some(path) = parse_context_path(clause) else {
        return Err(ClosedFormRefusal::new(
            file,
            place,
            RefusalKind::ConditionSyntax,
            format!("condition `{raw}`"),
            ALLOWED_CONDITIONS,
        ));
    };
    if !matches!(path, ContextPath::Input(_) | ContextPath::Matrix(_)) {
        return Err(ClosedFormRefusal::new(
            file,
            place,
            RefusalKind::ConditionBarePath,
            format!("bare condition `{raw}`"),
            ALLOWED_BARE_PATHS,
        ));
    }
    Ok(ClosedCondition {
        raw: raw.to_string(),
        always: false,
        body: ConditionBody::Bare(path),
    })
}

impl ClosedCondition {
    /// Parse a closed-form GitHub Actions `if:` condition or refuse it.
    pub(crate) fn parse(
        if_cond: Option<&str>,
        file: &str,
        place: &str,
    ) -> Result<Self, ClosedFormRefusal> {
        let Some(raw) = if_cond else {
            return Ok(Self {
                raw: String::new(),
                always: false,
                body: ConditionBody::Unconditional,
            });
        };
        if raw.contains("${{") || raw.contains("}}") {
            return Err(ClosedFormRefusal::new(
                file,
                place,
                RefusalKind::ConditionSyntax,
                format!("condition `{raw}` uses `${{{{ }}}}` wrapper"),
                ALLOWED_CONDITIONS,
            ));
        }
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(ClosedFormRefusal::new(
                file,
                place,
                RefusalKind::ConditionSyntax,
                "empty condition",
                ALLOWED_CONDITIONS,
            ));
        }
        let (op, clauses) = split_logic_clauses(trimmed, file, place, raw)?;
        let parse_all = |slice: &[&str]| {
            slice
                .iter()
                .map(|c| parse_comparison(c, file, place, raw))
                .collect::<Result<Vec<_>, _>>()
        };
        match op {
            None => parse_single_clause(clauses[0], file, place, raw),
            Some(LogicOp::And) => {
                let (always, rest) = match clauses.first().copied() {
                    Some("always()") => (true, &clauses[1..]),
                    _ => (false, clauses.as_slice()),
                };
                Ok(Self {
                    raw: raw.to_string(),
                    always,
                    body: ConditionBody::All(parse_all(rest)?),
                })
            }
            Some(LogicOp::Or) => Ok(Self {
                raw: raw.to_string(),
                always: false,
                body: ConditionBody::Any(parse_all(&clauses)?),
            }),
        }
    }

    /// Single evaluator for closed-form conditions: applies GitHub's implicit
    /// `success() && (...)` whenever leading `always()` is absent.
    pub(crate) fn evaluate(
        &self,
        ctx: &ConditionContext,
        file: &str,
        place: &str,
    ) -> Result<bool, ClosedFormRefusal> {
        let body_ok = match &self.body {
            ConditionBody::Unconditional => true,
            ConditionBody::All(cmps) => {
                let mut acc = true;
                for cmp in cmps {
                    acc &= eval_comparison(cmp, ctx, file, place, &self.raw)?;
                }
                acc
            }
            ConditionBody::Any(cmps) => {
                let mut acc = false;
                for cmp in cmps {
                    acc |= eval_comparison(cmp, ctx, file, place, &self.raw)?;
                }
                acc
            }
            ConditionBody::Bare(path) => eval_bare(path, ctx, file, place, &self.raw)?,
        };
        Ok(if self.always {
            body_ok
        } else {
            ctx.prior_succeeded && body_ok
        })
    }
}

fn is_numeric_or_empty_literal(s: &str) -> bool {
    let t = s.trim();
    t.is_empty() || t.parse::<f64>().is_ok() || t.starts_with("0x") || t.starts_with("0X")
}

fn eval_comparison(
    cmp: &Comparison,
    ctx: &ConditionContext,
    file: &str,
    place: &str,
    raw: &str,
) -> Result<bool, ClosedFormRefusal> {
    let val = ctx.lookup(&cmp.path).ok_or_else(|| {
        ClosedFormRefusal::new(
            file,
            place,
            RefusalKind::ConditionUnbound,
            format!("unbound context `{}` in `{raw}`", cmp.path.display_name()),
            ALLOWED_PATHS,
        )
    })?;
    let eq = match val {
        CondVal::Str(s) => s.eq_ignore_ascii_case(&cmp.literal),
        CondVal::Absent => {
            if is_numeric_or_empty_literal(&cmp.literal) {
                return Err(ClosedFormRefusal::new(
                    file,
                    place,
                    RefusalKind::ConditionTypeMismatch,
                    format!(
                        "absent `{}` compared with numeric/empty literal `{}` in `{raw}`",
                        cmp.path.display_name(),
                        cmp.literal
                    ),
                    ALLOWED_COMPARISONS,
                ));
            }
            false
        }
        CondVal::Bool(_) => {
            return Err(ClosedFormRefusal::new(
                file,
                place,
                RefusalKind::ConditionTypeMismatch,
                format!(
                    "boolean `{}` compared against string in `{raw}`",
                    cmp.path.display_name()
                ),
                ALLOWED_COMPARISONS,
            ));
        }
    };
    Ok(match cmp.op {
        CompareOp::Eq => eq,
        CompareOp::Ne => !eq,
    })
}

fn eval_bare(
    path: &ContextPath,
    ctx: &ConditionContext,
    file: &str,
    place: &str,
    raw: &str,
) -> Result<bool, ClosedFormRefusal> {
    let val = ctx.lookup(path).ok_or_else(|| {
        ClosedFormRefusal::new(
            file,
            place,
            RefusalKind::ConditionUnbound,
            format!("unbound bare context `{}` in `{raw}`", path.display_name()),
            ALLOWED_PATHS,
        )
    })?;
    match (path, val) {
        (ContextPath::Input(_), CondVal::Bool(b)) => Ok(b),
        (ContextPath::Matrix(_), CondVal::Str(s)) if !s.is_empty() => Ok(true),
        (ContextPath::Matrix(_), CondVal::Absent) => Ok(false),
        _ => Err(ClosedFormRefusal::new(
            file,
            place,
            RefusalKind::ConditionBarePath,
            format!("bare condition `{raw}` has non-admitted value"),
            ALLOWED_BARE_PATHS,
        )),
    }
}

/// True when `s` is a non-empty plain artifact literal (`[A-Za-z0-9._-]+`, no blanks).
pub(crate) fn is_plain_artifact_literal(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// Closed-form upload/download artifact `name:` value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ArtifactName {
    Literal(String),
    MatrixAxis(String),
    JobIndex { prefix: String },
}

/// Closed-form download artifact `pattern:` value (`<plain-literal>*`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArtifactPattern {
    pub(crate) prefix: String,
}

fn matrix_axis_all_plain_literals(matrix: &Matrix, axis: &str) -> bool {
    if matrix.axes.is_empty() {
        return !matrix.include.is_empty()
            && matrix.include.iter().all(|row| {
                row.get(axis)
                    .and_then(Scalar::as_str)
                    .is_some_and(is_plain_artifact_literal)
            });
    }
    let Some(vals) = matrix.axes.get(axis) else {
        return false;
    };
    !vals.is_empty()
        && vals
            .iter()
            .all(|v| v.as_str().is_some_and(is_plain_artifact_literal))
        && matrix.include.iter().all(|row| {
            row.get(axis)
                .and_then(Scalar::as_str)
                .is_some_and(is_plain_artifact_literal)
        })
}

fn matrix_axis_bare_truthy_valid(matrix: &Matrix, axis: &str) -> bool {
    if matrix.axes.is_empty() {
        return !matrix.include.is_empty()
            && matrix
                .include
                .iter()
                .all(|row| row.get(axis).is_none_or(|v| v.as_literal().is_some()));
    }
    let axes_ok = matrix
        .axes
        .get(axis)
        .is_none_or(|vals| vals.iter().all(|v| v.as_literal().is_some()));
    let include_ok = matrix
        .include
        .iter()
        .all(|row| row.get(axis).is_none_or(|v| v.as_literal().is_some()));
    axes_ok && include_ok
}

impl ArtifactName {
    pub(crate) fn parse(
        raw: &Scalar,
        matrix: Option<&Matrix>,
        file: &str,
        place: &str,
    ) -> Result<Self, ClosedFormRefusal> {
        let refuse = |shown: String| {
            ClosedFormRefusal::new(
                file,
                place,
                RefusalKind::ArtifactName,
                format!("artifact name `{shown}`"),
                ALLOWED_ARTIFACT_NAMES,
            )
        };
        let Some(s) = raw.as_str() else {
            return Err(refuse(format!("{raw:?}")));
        };
        if is_plain_artifact_literal(s) {
            return Ok(Self::Literal(s.to_string()));
        }
        if let Some(inner) = s.strip_prefix("${{").and_then(|r| r.strip_suffix("}}")) {
            let trimmed = inner.trim();
            if let Some(axis) = trimmed
                .strip_prefix("matrix.")
                .filter(|a| is_ident_segment(a))
            {
                if matrix.is_some_and(|m| matrix_axis_all_plain_literals(m, axis)) {
                    return Ok(Self::MatrixAxis(axis.to_string()));
                }
            }
        }
        if let Some(prefix) = s.strip_suffix("${{ strategy.job-index }}") {
            if is_plain_artifact_literal(prefix) {
                return Ok(Self::JobIndex {
                    prefix: prefix.to_string(),
                });
            }
        }
        Err(refuse(s.to_string()))
    }
}

impl ArtifactPattern {
    pub(crate) fn parse(raw: &Scalar, file: &str, place: &str) -> Result<Self, ClosedFormRefusal> {
        let Some(s) = raw.as_str() else {
            return Err(ClosedFormRefusal::new(
                file,
                place,
                RefusalKind::ArtifactName,
                format!("artifact pattern `{raw:?}`"),
                ALLOWED_ARTIFACT_NAMES,
            ));
        };
        if let Some(prefix) = s.strip_suffix('*') {
            if is_plain_artifact_literal(prefix) {
                return Ok(Self {
                    prefix: prefix.to_string(),
                });
            }
        }
        Err(ClosedFormRefusal::new(
            file,
            place,
            RefusalKind::ArtifactName,
            format!("artifact pattern `{s}`"),
            ALLOWED_ARTIFACT_NAMES,
        ))
    }
}

impl Matrix {
    pub(crate) fn require_no_include(
        &self,
        file: &str,
        job_id: &str,
    ) -> Result<(), ClosedFormRefusal> {
        if self.include.is_empty() {
            return Ok(());
        }
        Err(ClosedFormRefusal::new(
            file,
            format!("job {job_id} strategy.matrix"),
            RefusalKind::MatrixClause,
            "matrix `include`",
            ALLOWED_MATRIX,
        ))
    }

    pub(crate) fn require_no_exclude(
        &self,
        file: &str,
        job_id: &str,
    ) -> Result<(), ClosedFormRefusal> {
        if self.exclude.is_empty() {
            return Ok(());
        }
        Err(ClosedFormRefusal::new(
            file,
            format!("job {job_id} strategy.matrix"),
            RefusalKind::MatrixClause,
            "matrix `exclude`",
            ALLOWED_MATRIX,
        ))
    }
}

/// Requirement on a workflow's `on:` trigger paired with job reachability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EventExpectation<'a> {
    Plain(&'a str),
    WithTypes { name: &'a str, types: &'a [&'a str] },
    ActiveSchedule,
    PushTags(&'a [&'a str]),
}

impl<'a> EventExpectation<'a> {
    pub(crate) fn event_name(&self) -> &'a str {
        match self {
            Self::Plain(n) | Self::WithTypes { name: n, .. } => n,
            Self::ActiveSchedule => "schedule",
            Self::PushTags(_) => "push",
        }
    }

    pub(crate) fn matches_trigger(&self, trigger: &EventTrigger) -> bool {
        if trigger.name != self.event_name() || !trigger.extra_keys.is_empty() {
            return false;
        }
        match self {
            Self::Plain(_) => {
                !trigger.has_types_key && trigger.filters.is_empty() && trigger.inputs.is_empty()
            }
            Self::WithTypes { types, .. } => {
                trigger.has_types_key
                    && trigger.filters.is_empty()
                    && trigger.inputs.is_empty()
                    && trigger.types.len() == types.len()
                    && types
                        .iter()
                        .all(|expected| trigger.types.iter().any(|t| t == expected))
            }
            Self::ActiveSchedule => {
                !trigger.has_types_key
                    && trigger.filters.is_empty()
                    && trigger.inputs.is_empty()
                    && trigger.has_active_schedule()
            }
            Self::PushTags(tags) => {
                !trigger.has_types_key
                    && trigger.inputs.is_empty()
                    && trigger.filters.len() == 1
                    && trigger.filters.get("tags").is_some_and(|v| {
                        v.len() == tags.len() && v.iter().zip(*tags).all(|(a, b)| a == b)
                    })
            }
        }
    }
}

fn validate_event_closed_form(ev: &EventTrigger, file: &str) -> Result<(), ClosedFormRefusal> {
    let place = format!("on.{}", ev.name);
    if !ev.inputs.is_empty() && ev.name != "workflow_call" {
        return Err(ClosedFormRefusal::new(
            file,
            &place,
            RefusalKind::EventFilter,
            format!("event filter `inputs` on `{}`", ev.name),
            ALLOWED_EVENT_FILTERS,
        ));
    }
    if let Some(extra) = ev.extra_keys.first() {
        return Err(ClosedFormRefusal::new(
            file,
            &place,
            RefusalKind::EventFilter,
            format!("event key `{extra}` on `{}`", ev.name),
            ALLOWED_EVENT_FILTERS,
        ));
    }
    for key in ev.filters.keys() {
        if !matches!(key.as_str(), "branches" | "tags") {
            return Err(ClosedFormRefusal::new(
                file,
                &place,
                RefusalKind::EventFilter,
                format!("event filter `{key}` on `{}`", ev.name),
                ALLOWED_EVENT_FILTERS,
            ));
        }
    }
    Ok(())
}

fn validate_bare_condition(
    wf: &Workflow,
    job: &Job,
    cond: &ClosedCondition,
    file: &str,
    place: &str,
) -> Result<(), ClosedFormRefusal> {
    let ConditionBody::Bare(path) = &cond.body else {
        return Ok(());
    };
    match path {
        ContextPath::Input(name) => {
            let is_bool = wf
                .workflow_call_input(name)
                .and_then(|i| i.input_type.as_ref())
                .and_then(Scalar::as_literal)
                == Some("boolean");
            if is_bool {
                return Ok(());
            }
            Err(ClosedFormRefusal::new(
                file,
                place,
                RefusalKind::ConditionBarePath,
                format!("bare input `inputs.{name}` is not declared `type: boolean`"),
                ALLOWED_BARE_PATHS,
            ))
        }
        ContextPath::Matrix(axis) => {
            if job
                .matrix
                .as_ref()
                .is_some_and(|m| matrix_axis_bare_truthy_valid(m, axis))
            {
                return Ok(());
            }
            Err(ClosedFormRefusal::new(
                file,
                place,
                RefusalKind::ConditionBarePath,
                format!(
                    "bare matrix axis `matrix.{axis}` is not a non-empty string or absent in every row"
                ),
                ALLOWED_BARE_PATHS,
            ))
        }
        _ => Err(ClosedFormRefusal::new(
            file,
            place,
            RefusalKind::ConditionBarePath,
            format!("bare condition `{}`", path.display_name()),
            ALLOWED_BARE_PATHS,
        )),
    }
}

fn validate_step_artifacts(
    job: &Job,
    step: &Step,
    step_idx: usize,
    file: &str,
) -> Result<(), ClosedFormRefusal> {
    let Some(uses) = step.uses.as_deref() else {
        return Ok(());
    };
    let is_upload = matches!(
        action_repo_match(uses, "actions/upload-artifact"),
        Some(ActionMatch::Exact | ActionMatch::SubAction(_))
    ) || matches!(
        action_repo_match(uses, "actions/upload-pages-artifact"),
        Some(ActionMatch::Exact | ActionMatch::SubAction(_))
    );
    let is_download = matches!(
        action_repo_match(uses, "actions/download-artifact"),
        Some(ActionMatch::Exact | ActionMatch::SubAction(_))
    );
    if !is_upload && !is_download {
        return Ok(());
    }
    let prefix = format!("job {} step {}", job.id, step_idx + 1);
    if let Some(raw) = step.with_value("name") {
        ArtifactName::parse(
            raw,
            job.matrix.as_ref(),
            file,
            &format!("{prefix} with.name"),
        )?;
    }
    if is_download {
        if let Some(raw) = step.with_value("pattern") {
            ArtifactPattern::parse(raw, file, &format!("{prefix} with.pattern"))?;
        }
    }
    Ok(())
}

impl Workflow {
    /// Parse a covered workflow and enforce the closed form up front.
    pub(crate) fn parse_closed_form(file: &str, text: &str) -> Result<Self, ClosedWorkflowError> {
        let wf = Self::parse(text).map_err(|err| ClosedWorkflowError::Parse {
            file: file.to_string(),
            err,
        })?;
        wf.validate_closed_form(file)?;
        Ok(wf)
    }

    /// Verify every closed-form invariant across the workflow document.
    pub(crate) fn validate_closed_form(&self, file: &str) -> Result<(), ClosedFormRefusal> {
        if self.top_level_keys.iter().any(|k| k == "env") {
            return Err(ClosedFormRefusal::new(
                file,
                "top-level env",
                RefusalKind::WorkflowEnv,
                "workflow-level `env`",
                ALLOWED_ENV,
            ));
        }
        for ev in &self.events {
            validate_event_closed_form(ev, file)?;
        }
        for job in &self.jobs {
            let job_place = format!("job {} if", job.id);
            let job_cond = ClosedCondition::parse(job.if_cond.as_deref(), file, &job_place)?;
            validate_bare_condition(self, job, &job_cond, file, &job_place)?;
            for (idx, step) in job.steps.iter().enumerate() {
                let step_place = format!("job {} step {} if", job.id, idx + 1);
                let step_cond = ClosedCondition::parse(step.if_cond.as_deref(), file, &step_place)?;
                validate_bare_condition(self, job, &step_cond, file, &step_place)?;
                validate_step_artifacts(job, step, idx, file)?;
            }
        }
        Ok(())
    }

    /// True only when both `on:` delivers `spec` (including its `types` and
    /// filters) and `job` runs under `ctx` with `github.event_name` bound to
    /// `spec.event_name()`.
    pub(crate) fn event_reaches_job(
        &self,
        file: &str,
        job: &Job,
        spec: &EventExpectation<'_>,
        ctx: &ConditionContext,
    ) -> Result<bool, ClosedFormRefusal> {
        let Some(trigger) = self.event(spec.event_name()) else {
            return Ok(false);
        };
        if !spec.matches_trigger(trigger) {
            return Ok(false);
        }
        let ev_ctx = ctx.clone().with_event(spec.event_name());
        job.runs_under(file, &ev_ctx)
    }
}
