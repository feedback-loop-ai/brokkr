//! Structural YAML 1.2 reader for GitHub Actions workflow files (#1396).
//!
//! `tests/it.rs` declares this once so every guard test asking a workflow a
//! structural question reads one typed document parsed by `yaml-rust2` rather
//! than scanning lines or indentation (decision 0071 rulings 3 and 5).

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

/// Error evaluating a GitHub Actions `if:` condition.
///
/// The evaluator fails closed on syntax errors, unsupported constructs, and
/// unbound context references rather than guessing.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ConditionError {
    #[error("condition `{expr}` is invalid: {reason}")]
    Syntax { expr: String, reason: &'static str },
    #[error("condition `{expr}` uses unsupported construct `{token}`")]
    Unsupported { expr: String, token: String },
    #[error("condition `{expr}` references unbound value `{name}`")]
    Unbound { expr: String, name: String },
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
    pub(crate) types: Vec<String>,
    pub(crate) filters: BTreeMap<String, Vec<String>>,
    pub(crate) crons: Vec<String>,
    pub(crate) inputs: Vec<(String, WorkflowInput)>,
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

    pub(crate) fn runs_under(&self, ctx: &ConditionContext) -> Result<bool, ConditionError> {
        evaluate_condition(self.if_cond.as_deref(), ctx)
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

    pub(crate) fn runs_under(&self, ctx: &ConditionContext) -> Result<bool, ConditionError> {
        evaluate_condition(self.if_cond.as_deref(), ctx)
    }

    /// True only when both the job (under `job_ctx`) and `step` (under `step_ctx`)
    /// evaluate to `true` and neither allows failure via `continue-on-error`.
    pub(crate) fn step_enforces(
        &self,
        step: &Step,
        job_ctx: &ConditionContext,
        step_ctx: &ConditionContext,
    ) -> Result<bool, ConditionError> {
        if self.allows_failure() || step.allows_failure() {
            return Ok(false);
        }
        Ok(self.runs_under(job_ctx)? && step.runs_under(step_ctx)?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Workflow {
    pub(crate) top_level_keys: Vec<String>,
    pub(crate) events: Vec<EventTrigger>,
    pub(crate) env: Vec<(String, String)>,
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
            env: map_get(&root, "env")
                .map(parse_env)
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
        types: Vec::new(),
        filters: BTreeMap::new(),
        crons: Vec::new(),
        inputs: Vec::new(),
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
            "types" => trigger.types = parse_string_or_list(v, "on.<event>.types")?,
            "inputs" => trigger.inputs = parse_workflow_inputs(v)?,
            "secrets" | "outputs" if name == "workflow_call" => {
                let _ = expect_map(v, "on.workflow_call")?;
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

/// Value bound in a `ConditionContext` for evaluating GitHub Actions `if:` expressions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CondVal {
    Bool(bool),
    Str(String),
}

impl CondVal {
    fn is_truthy(&self) -> bool {
        match self {
            Self::Bool(b) => *b,
            Self::Str(s) => !s.is_empty(),
        }
    }
}

/// Bindings available while evaluating a GitHub Actions `if:` expression.
#[derive(Debug, Clone)]
pub(crate) struct ConditionContext {
    prior_succeeded: bool,
    event_name: Option<String>,
    matrix: BTreeMap<String, CondVal>,
    inputs: BTreeMap<String, CondVal>,
    needs_results: BTreeMap<String, String>,
}

impl ConditionContext {
    pub(crate) fn default_ok() -> Self {
        Self {
            prior_succeeded: true,
            event_name: None,
            matrix: BTreeMap::new(),
            inputs: BTreeMap::new(),
            needs_results: BTreeMap::new(),
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
}

/// Evaluate a GitHub Actions `if:` expression under `ctx`.
///
/// Supported grammar (everything else fails closed with `Err(ConditionError)`):
/// - Omitted condition (`None`): implicit `success()` (`ctx.prior_succeeded`).
/// - Optional outer `${{ ... }}` wrapper (no nested or multiple wrappers).
/// - Boolean operators `||`, `&&`, unary `!`, equality `==` / `!=` (same-type;
///   ASCII case-insensitive for strings), and parentheses `(...)`.
/// - Literals: `true`, `false`, single-quoted strings `'...'` (with `''` escape).
/// - Status functions: `always()`, `success()`, `failure()`, `cancelled()`.
/// - Context references: `github.event_name`, `matrix.<key>`, `inputs.<key>`
///   (case-insensitive), and `needs.<job>.result`.
///
/// Both operands of `&&` and `||` are evaluated so unsupported or unbound
/// expressions cannot hide behind a constant operand.
pub(crate) fn evaluate_condition(
    if_cond: Option<&str>,
    ctx: &ConditionContext,
) -> Result<bool, ConditionError> {
    let Some(raw) = if_cond else {
        return Ok(ctx.prior_succeeded);
    };
    let expr = unwrap_condition(raw)?;
    let tokens = tokenize_cond(expr, raw)?;
    let mut parser = CondParser {
        raw,
        tokens: &tokens,
        pos: 0,
        ctx,
    };
    let val = parser.parse_or()?;
    if parser.pos < parser.tokens.len() {
        return Err(ConditionError::Syntax {
            expr: raw.to_string(),
            reason: "unexpected trailing tokens",
        });
    }
    Ok(val.is_truthy())
}

#[derive(Debug, PartialEq, Eq)]
enum CondTok {
    Or,
    And,
    Eq,
    Ne,
    Bang,
    LParen,
    RParen,
    Str(String),
    Ident(String),
}

fn unwrap_condition(raw: &str) -> Result<&str, ConditionError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ConditionError::Syntax {
            expr: raw.to_string(),
            reason: "empty condition",
        });
    }
    if let Some(rest) = trimmed.strip_prefix("${{") {
        let Some(inner) = rest.strip_suffix("}}") else {
            return Err(ConditionError::Syntax {
                expr: raw.to_string(),
                reason: "unclosed `${{` wrapper",
            });
        };
        if inner.contains("${{") || inner.contains("}}") || inner.trim().is_empty() {
            return Err(ConditionError::Syntax {
                expr: raw.to_string(),
                reason: "invalid `${{ ... }}` wrapper",
            });
        }
        return Ok(inner.trim());
    }
    if trimmed.contains("${{") || trimmed.contains("}}") {
        return Err(ConditionError::Syntax {
            expr: raw.to_string(),
            reason: "unexpected `${{` or `}}` delimiter",
        });
    }
    Ok(trimmed)
}

fn lex_quoted_string(bytes: &[u8], i: &mut usize, raw: &str) -> Result<String, ConditionError> {
    *i += 1;
    let mut out = String::new();
    while *i < bytes.len() {
        if bytes[*i] == b'\'' {
            if *i + 1 < bytes.len() && bytes[*i + 1] == b'\'' {
                out.push('\'');
                *i += 2;
                continue;
            }
            *i += 1;
            return Ok(out);
        }
        let ch = std::str::from_utf8(&bytes[*i..])
            .ok()
            .and_then(|s| s.chars().next())
            .ok_or_else(|| ConditionError::Syntax {
                expr: raw.to_string(),
                reason: "invalid utf-8",
            })?;
        out.push(ch);
        *i += ch.len_utf8();
    }
    Err(ConditionError::Syntax {
        expr: raw.to_string(),
        reason: "unclosed string literal",
    })
}

fn tokenize_cond(src: &str, raw: &str) -> Result<Vec<CondTok>, ConditionError> {
    let bytes = src.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\n' | b'\r' => i += 1,
            b'(' => {
                tokens.push(CondTok::LParen);
                i += 1;
            }
            b')' => {
                tokens.push(CondTok::RParen);
                i += 1;
            }
            b'\'' => tokens.push(CondTok::Str(lex_quoted_string(bytes, &mut i, raw)?)),
            b'!' if bytes.get(i + 1) == Some(&b'=') => {
                tokens.push(CondTok::Ne);
                i += 2;
            }
            b'!' => {
                tokens.push(CondTok::Bang);
                i += 1;
            }
            b'=' if bytes.get(i + 1) == Some(&b'=') => {
                tokens.push(CondTok::Eq);
                i += 2;
            }
            b'&' if bytes.get(i + 1) == Some(&b'&') => {
                tokens.push(CondTok::And);
                i += 2;
            }
            b'|' if bytes.get(i + 1) == Some(&b'|') => {
                tokens.push(CondTok::Or);
                i += 2;
            }
            b if b.is_ascii_alphabetic() || b == b'_' => {
                let start = i;
                while i < bytes.len()
                    && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b'_' | b'-' | b'.'))
                {
                    i += 1;
                }
                tokens.push(CondTok::Ident(src[start..i].to_string()));
            }
            _ => {
                return Err(ConditionError::Unsupported {
                    expr: raw.to_string(),
                    token: src[i..].chars().next().unwrap_or('?').to_string(),
                });
            }
        }
    }
    Ok(tokens)
}

struct CondParser<'a> {
    raw: &'a str,
    tokens: &'a [CondTok],
    pos: usize,
    ctx: &'a ConditionContext,
}

impl<'a> CondParser<'a> {
    fn peek(&self) -> Option<&'a CondTok> {
        self.tokens.get(self.pos)
    }

    fn parse_or(&mut self) -> Result<CondVal, ConditionError> {
        let mut lhs = self.parse_and()?;
        while self.peek() == Some(&CondTok::Or) {
            self.pos += 1;
            let rhs = self.parse_and()?;
            lhs = CondVal::Bool(lhs.is_truthy() || rhs.is_truthy());
        }
        Ok(lhs)
    }

    fn parse_and(&mut self) -> Result<CondVal, ConditionError> {
        let mut lhs = self.parse_eq()?;
        while self.peek() == Some(&CondTok::And) {
            self.pos += 1;
            let rhs = self.parse_eq()?;
            lhs = CondVal::Bool(lhs.is_truthy() && rhs.is_truthy());
        }
        Ok(lhs)
    }

    fn parse_eq(&mut self) -> Result<CondVal, ConditionError> {
        let mut lhs = self.parse_unary()?;
        while let Some(op @ (CondTok::Eq | CondTok::Ne)) = self.peek() {
            let negate = matches!(op, CondTok::Ne);
            self.pos += 1;
            let rhs = self.parse_unary()?;
            let eq = match (&lhs, &rhs) {
                (CondVal::Bool(a), CondVal::Bool(b)) => a == b,
                (CondVal::Str(a), CondVal::Str(b)) => a.eq_ignore_ascii_case(b),
                _ => {
                    return Err(ConditionError::Unsupported {
                        expr: self.raw.to_string(),
                        token: "mixed-type equality".to_string(),
                    });
                }
            };
            lhs = CondVal::Bool(if negate { !eq } else { eq });
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<CondVal, ConditionError> {
        if self.peek() == Some(&CondTok::Bang) {
            self.pos += 1;
            return Ok(CondVal::Bool(!self.parse_unary()?.is_truthy()));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<CondVal, ConditionError> {
        match self.peek() {
            Some(CondTok::LParen) => {
                self.pos += 1;
                let val = self.parse_or()?;
                if self.peek() != Some(&CondTok::RParen) {
                    return Err(ConditionError::Syntax {
                        expr: self.raw.to_string(),
                        reason: "expected `)`",
                    });
                }
                self.pos += 1;
                Ok(val)
            }
            Some(CondTok::Str(s)) => {
                self.pos += 1;
                Ok(CondVal::Str(s.clone()))
            }
            Some(CondTok::Ident(name)) => {
                self.pos += 1;
                if self.peek() == Some(&CondTok::LParen) {
                    self.eval_call(name)
                } else {
                    self.resolve_ident(name)
                }
            }
            _ => Err(ConditionError::Syntax {
                expr: self.raw.to_string(),
                reason: "expected expression",
            }),
        }
    }

    fn eval_call(&mut self, name: &str) -> Result<CondVal, ConditionError> {
        self.pos += 1;
        if self.peek() != Some(&CondTok::RParen) {
            return Err(ConditionError::Unsupported {
                expr: self.raw.to_string(),
                token: format!("{name}(...)"),
            });
        }
        self.pos += 1;
        match name.to_ascii_lowercase().as_str() {
            "always" => Ok(CondVal::Bool(true)),
            "success" => Ok(CondVal::Bool(self.ctx.prior_succeeded)),
            "failure" => Ok(CondVal::Bool(!self.ctx.prior_succeeded)),
            "cancelled" => Ok(CondVal::Bool(false)),
            _ => Err(ConditionError::Unsupported {
                expr: self.raw.to_string(),
                token: format!("{name}()"),
            }),
        }
    }

    fn resolve_ident(&self, name: &str) -> Result<CondVal, ConditionError> {
        if name.eq_ignore_ascii_case("true") {
            return Ok(CondVal::Bool(true));
        }
        if name.eq_ignore_ascii_case("false") {
            return Ok(CondVal::Bool(false));
        }
        let unbound = || ConditionError::Unbound {
            expr: self.raw.to_string(),
            name: name.to_string(),
        };
        if name == "github.event_name" {
            return self
                .ctx
                .event_name
                .clone()
                .map(CondVal::Str)
                .ok_or_else(unbound);
        }
        if let Some(key) = name
            .strip_prefix("matrix.")
            .filter(|k| !k.is_empty() && !k.contains('.'))
        {
            return self.ctx.matrix.get(key).cloned().ok_or_else(unbound);
        }
        if let Some(key) = name
            .strip_prefix("inputs.")
            .filter(|k| !k.is_empty() && !k.contains('.'))
        {
            return self
                .ctx
                .inputs
                .get(&key.to_ascii_lowercase())
                .cloned()
                .ok_or_else(unbound);
        }
        if let Some(job) = name
            .strip_prefix("needs.")
            .and_then(|r| r.strip_suffix(".result"))
            .filter(|j| !j.is_empty() && !j.contains('.'))
        {
            return self
                .ctx
                .needs_results
                .get(job)
                .cloned()
                .map(CondVal::Str)
                .ok_or_else(unbound);
        }
        Err(ConditionError::Unsupported {
            expr: self.raw.to_string(),
            token: name.to_string(),
        })
    }
}
