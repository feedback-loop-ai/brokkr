//! Total closed-form admission and typed covered-workflow representation
//! (#1417, decision 0071 rulings 2, 3 and 8).
//!
//! Every covered workflow is admitted once into `ClosedWorkflow`. Admission
//! checks every top-level key, trigger, matrix, condition, and artifact
//! name/pattern statically and retains the parsed typed values so guards never
//! re-parse raw YAML strings.

use super::condition::{
    is_ident_segment, Comparison, ConditionBody, ContextPath, ALLOWED_ARTIFACT_NAMES,
    ALLOWED_BARE_PATHS, ALLOWED_COMPARISONS, ALLOWED_ENV, ALLOWED_EVENT_FILTERS, ALLOWED_MATRIX,
    ALLOWED_PATHS, ALLOWED_TOP_LEVEL,
};
use super::{
    action_repo_match, continue_on_error_allows_failure, parse_workflow_raw, ActionMatch,
    ClosedCondition, ClosedFormRefusal, ClosedWorkflowError, Concurrency, ConditionContext,
    EventTrigger, Inputs, Job, Matrix, Permissions, RefusalKind, RunsOn, Scalar, Step,
    TriggerFilterKey, Verdict, Workflow, WorkflowError, WorkflowInput,
};

/// Covered-workflow jobs whose `strategy.matrix.include` rows are read and
/// accounted for by a guard (documented in `CONTRIBUTING.md`).
const ALLOWED_MATRIX_INCLUDE_JOBS: [(&str, &str); 2] =
    [("ci.yml", "pr-caches"), ("release.yml", "build")];

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

/// Closed-form download artifact `pattern:` value (plain literal or `<literal>*`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ArtifactPattern {
    Literal(String),
    PrefixWildcard(String),
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

/// Typed, closed-form-admitted workflow step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClosedStep {
    raw: Step,
    pub(crate) condition: ClosedCondition,
    pub(crate) artifact_name: Option<ArtifactName>,
    pub(crate) artifact_pattern: Option<ArtifactPattern>,
}

impl ClosedStep {
    pub(crate) fn uses(&self) -> Option<&str> {
        self.raw.uses.as_deref()
    }

    pub(crate) fn run(&self) -> Option<&str> {
        self.raw.run.as_deref()
    }

    pub(crate) fn with(&self) -> Option<&Inputs> {
        self.raw.with.as_ref()
    }

    pub(crate) fn env(&self) -> &[(String, String)] {
        &self.raw.env
    }

    pub(crate) fn env_value(&self, key: &str) -> Option<&str> {
        self.raw.env_value(key)
    }

    pub(crate) fn with_value(&self, key: &str) -> Option<&Scalar> {
        self.raw.with_value(key)
    }

    pub(crate) fn with_literal(&self, key: &str) -> Option<&str> {
        self.raw.with_literal(key)
    }

    pub(crate) fn action_match(&self, expected_repo: &str) -> Option<ActionMatch> {
        self.raw.action_match(expected_repo)
    }

    pub(crate) fn allows_failure(&self) -> bool {
        continue_on_error_allows_failure(self.raw.continue_on_error.as_ref())
    }

    pub(crate) fn runs_under(&self, ctx: &ConditionContext) -> Verdict {
        self.condition.evaluate(ctx)
    }
}

/// Typed, closed-form-admitted workflow job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClosedJob {
    raw: Job,
    pub(crate) condition: ClosedCondition,
    pub(crate) steps: Vec<ClosedStep>,
}

impl ClosedJob {
    pub(crate) fn id(&self) -> &str {
        &self.raw.id
    }

    pub(crate) fn name(&self) -> Option<&str> {
        self.raw.name.as_deref()
    }

    pub(crate) fn needs(&self) -> &[String] {
        &self.raw.needs
    }

    pub(crate) fn runs_on(&self) -> Option<&RunsOn> {
        self.raw.runs_on.as_ref()
    }

    pub(crate) fn uses(&self) -> Option<&str> {
        self.raw.uses.as_deref()
    }

    pub(crate) fn with_value(&self, key: &str) -> Option<&Scalar> {
        self.raw.with.as_ref()?.get(key)
    }

    pub(crate) fn env(&self) -> &[(String, String)] {
        &self.raw.env
    }

    pub(crate) fn permissions(&self) -> Option<&Permissions> {
        self.raw.permissions.as_ref()
    }

    pub(crate) fn matrix(&self) -> Option<&Matrix> {
        self.raw.matrix.as_ref()
    }

    pub(crate) fn all_scalars(&self) -> &[String] {
        &self.raw.all_scalars
    }

    /// Evaluates job and step conditions without masking refusals behind
    /// `continue-on-error`.
    pub(crate) fn step_enforces(
        &self,
        step: &ClosedStep,
        job_ctx: &ConditionContext,
        step_ctx: &ConditionContext,
    ) -> Verdict {
        let enforced = Verdict::from_bool(!self.allows_failure() && !step.allows_failure());
        enforced
            .and(self.runs_under(job_ctx))
            .and(step.runs_under(step_ctx))
    }

    pub(crate) fn runs_under(&self, ctx: &ConditionContext) -> Verdict {
        self.condition.evaluate(ctx)
    }

    pub(crate) fn allows_failure(&self) -> bool {
        continue_on_error_allows_failure(self.raw.continue_on_error.as_ref())
    }
}

/// Typed, closed-form-admitted covered workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClosedWorkflow {
    raw: Workflow,
    pub(crate) jobs: Vec<ClosedJob>,
}

impl ClosedWorkflow {
    /// Parse a covered workflow and enforce total closed-form admission once.
    pub(crate) fn parse(file: &str, text: &str) -> Result<Self, ClosedWorkflowError> {
        let raw = parse_workflow_raw(text, false).map_err(|err| ClosedWorkflowError::Parse {
            file: file.to_string(),
            err,
        })?;
        admit_workflow(file, raw).map_err(ClosedWorkflowError::Refusal)
    }

    pub(crate) fn events(&self) -> &[EventTrigger] {
        &self.raw.events
    }

    pub(crate) fn concurrency(&self) -> Option<&Concurrency> {
        self.raw.concurrency.as_ref()
    }

    pub(crate) fn event_names(&self) -> Vec<&str> {
        self.raw.event_names()
    }

    pub(crate) fn event(&self, name: &str) -> Option<&EventTrigger> {
        self.raw.event(name)
    }

    pub(crate) fn unfiltered_event(&self, name: &str) -> Option<&EventTrigger> {
        self.raw.unfiltered_event(name)
    }

    pub(crate) fn workflow_call_input(&self, name: &str) -> Option<&WorkflowInput> {
        self.raw.workflow_call_input(name)
    }

    pub(crate) fn grants_permission(&self, scope: &str, level: &str) -> bool {
        self.raw.grants_permission(scope, level)
    }

    pub(crate) fn require_job(&self, file: &str, id: &str) -> Result<&ClosedJob, WorkflowError> {
        self.jobs
            .iter()
            .find(|job| job.id() == id)
            .ok_or_else(|| WorkflowError::MissingJob {
                file: file.to_string(),
                id: id.to_string(),
            })
    }

    /// Evaluates whether `on:` delivers `spec` AND `job` runs under `ctx` with
    /// `github.event_name` bound to `spec.event_name()`, preserving any refusal.
    pub(crate) fn event_reaches_job(
        &self,
        job: &ClosedJob,
        spec: &EventExpectation<'_>,
        ctx: &ConditionContext,
    ) -> Verdict {
        let trigger_matches = self
            .event(spec.event_name())
            .is_some_and(|t| spec.matches_trigger(t));
        let ev_ctx = ctx.clone().with_event(spec.event_name());
        Verdict::from_bool(trigger_matches).and(job.runs_under(&ev_ctx))
    }
}

fn admit_workflow(file: &str, raw: Workflow) -> Result<ClosedWorkflow, ClosedFormRefusal> {
    validate_top_level(&raw, file)?;
    if let Some(issue) = raw.event_shape_issues.first() {
        return Err(ClosedFormRefusal::new(
            file,
            &issue.place,
            RefusalKind::EventFilter,
            &issue.detail,
            ALLOWED_EVENT_FILTERS,
        ));
    }
    for ev in &raw.events {
        validate_event_closed_form(ev, file)?;
    }
    let mut closed_jobs = Vec::with_capacity(raw.jobs.len());
    for job in &raw.jobs {
        closed_jobs.push(admit_job(file, &raw, job)?);
    }
    Ok(ClosedWorkflow {
        raw,
        jobs: closed_jobs,
    })
}

fn validate_top_level(wf: &Workflow, file: &str) -> Result<(), ClosedFormRefusal> {
    for key in &wf.top_level_keys {
        if key == "env" {
            return Err(ClosedFormRefusal::new(
                file,
                "top-level env",
                RefusalKind::WorkflowEnv,
                "workflow-level `env`",
                ALLOWED_ENV,
            ));
        }
        if !matches!(
            key.as_str(),
            "name" | "on" | "permissions" | "concurrency" | "jobs"
        ) {
            return Err(ClosedFormRefusal::new(
                file,
                format!("top-level {key}"),
                RefusalKind::TopLevelKey,
                format!("top-level key `{key}`"),
                ALLOWED_TOP_LEVEL,
            ));
        }
    }
    if let Some(extra) = wf.concurrency.as_ref().and_then(|c| c.extra_keys.first()) {
        return Err(ClosedFormRefusal::new(
            file,
            "concurrency",
            RefusalKind::TopLevelKey,
            format!("concurrency key `{extra}`"),
            "concurrency group and cancel-in-progress only",
        ));
    }
    Ok(())
}

fn validate_schedule_trigger(
    ev: &EventTrigger,
    file: &str,
    place: &str,
) -> Result<(), ClosedFormRefusal> {
    if ev.crons.is_empty() {
        return Err(ClosedFormRefusal::new(
            file,
            place,
            RefusalKind::EventFilter,
            "empty `schedule` trigger",
            ALLOWED_EVENT_FILTERS,
        ));
    }
    Ok(())
}

fn validate_workflow_call_trigger(
    ev: &EventTrigger,
    file: &str,
    place: &str,
) -> Result<(), ClosedFormRefusal> {
    if ev.has_types_key {
        return Err(ClosedFormRefusal::new(
            file,
            place,
            RefusalKind::EventFilter,
            format!(
                "event filter `{}` on `workflow_call`",
                TriggerFilterKey::Types.as_str()
            ),
            ALLOWED_EVENT_FILTERS,
        ));
    }
    if let Some(key) = ev.filters.keys().next() {
        return Err(ClosedFormRefusal::new(
            file,
            place,
            RefusalKind::EventFilter,
            format!("event filter `{key}` on `workflow_call`"),
            ALLOWED_EVENT_FILTERS,
        ));
    }
    Ok(())
}

fn validate_event_closed_form(ev: &EventTrigger, file: &str) -> Result<(), ClosedFormRefusal> {
    let place = format!("on.{}", ev.name);
    if let Some(issue) = ev.shape_issues.first() {
        return Err(ClosedFormRefusal::new(
            file,
            &issue.place,
            RefusalKind::EventFilter,
            &issue.detail,
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
    if ev.name == "schedule" {
        validate_schedule_trigger(ev, file, &place)?;
    }
    if ev.name == "workflow_call" {
        validate_workflow_call_trigger(ev, file, &place)?;
    } else if ev.has_inputs_key || !ev.inputs.is_empty() {
        return Err(ClosedFormRefusal::new(
            file,
            &place,
            RefusalKind::EventFilter,
            format!("event filter `inputs` on `{}`", ev.name),
            ALLOWED_EVENT_FILTERS,
        ));
    }
    for (input_name, input) in &ev.inputs {
        if let Some(extra) = input.extra_keys.first() {
            return Err(ClosedFormRefusal::new(
                file,
                format!("{place}.inputs.{input_name}"),
                RefusalKind::EventFilter,
                format!("input key `{extra}` on `inputs.{input_name}`"),
                ALLOWED_EVENT_FILTERS,
            ));
        }
    }
    for key in ev.filters.keys() {
        if !matches!(
            TriggerFilterKey::parse(key),
            Some(TriggerFilterKey::Branches | TriggerFilterKey::Tags)
        ) {
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

fn admit_job(file: &str, wf: &Workflow, job: &Job) -> Result<ClosedJob, ClosedFormRefusal> {
    if let Some(matrix) = &job.matrix {
        validate_matrix_closed_form(file, &job.id, matrix)?;
    }
    let job_place = format!("job {} if", job.id);
    let job_cond = ClosedCondition::parse_raw(job.if_raw.as_ref(), file, &job_place)?;
    validate_condition_static(wf, job, None, &job_cond, file, &job_place)?;

    let mut closed_steps = Vec::with_capacity(job.steps.len());
    for (idx, step) in job.steps.iter().enumerate() {
        closed_steps.push(admit_step(file, wf, job, idx, step)?);
    }
    Ok(ClosedJob {
        raw: job.clone(),
        condition: job_cond,
        steps: closed_steps,
    })
}

fn admit_step(
    file: &str,
    wf: &Workflow,
    job: &Job,
    idx: usize,
    step: &Step,
) -> Result<ClosedStep, ClosedFormRefusal> {
    let step_place = format!("job {} step {} if", job.id, idx + 1);
    let condition = ClosedCondition::parse_raw(step.if_raw.as_ref(), file, &step_place)?;
    validate_condition_static(wf, job, Some(idx), &condition, file, &step_place)?;
    let (artifact_name, artifact_pattern) = parse_step_artifacts(job, step, idx, file)?;
    Ok(ClosedStep {
        raw: step.clone(),
        condition,
        artifact_name,
        artifact_pattern,
    })
}

fn file_basename(file: &str) -> &str {
    file.rsplit('/').next().unwrap_or(file)
}

fn validate_matrix_closed_form(
    file: &str,
    job_id: &str,
    matrix: &Matrix,
) -> Result<(), ClosedFormRefusal> {
    let place = format!("job {job_id} strategy.matrix");
    if matrix.non_mapping {
        return Err(ClosedFormRefusal::new(
            file,
            place,
            RefusalKind::MatrixClause,
            "non-mapping `strategy.matrix`",
            ALLOWED_MATRIX,
        ));
    }
    if matrix.has_exclude_key || !matrix.exclude.is_empty() {
        return Err(ClosedFormRefusal::new(
            file,
            place,
            RefusalKind::MatrixClause,
            "matrix `exclude`",
            ALLOWED_MATRIX,
        ));
    }
    if matrix.has_include_key || !matrix.include.is_empty() {
        let base = file_basename(file);
        let allowed = ALLOWED_MATRIX_INCLUDE_JOBS
            .iter()
            .any(|&(f, j)| f == base && j == job_id);
        if !allowed || !matrix.axes.is_empty() {
            return Err(ClosedFormRefusal::new(
                file,
                place,
                RefusalKind::MatrixClause,
                "matrix `include`",
                ALLOWED_MATRIX,
            ));
        }
    }
    if (matrix.axes.is_empty() && matrix.include.is_empty())
        || matrix.axes.values().any(Vec::is_empty)
    {
        return Err(ClosedFormRefusal::new(
            file,
            place,
            RefusalKind::MatrixClause,
            "empty matrix or empty matrix axis",
            ALLOWED_MATRIX,
        ));
    }
    Ok(())
}

fn matrix_axis_all_match(matrix: &Matrix, axis: &str, pred: fn(&Scalar) -> bool) -> bool {
    if matrix.axes.is_empty() {
        return !matrix.include.is_empty()
            && matrix
                .include
                .iter()
                .all(|row| row.get(axis).is_some_and(pred));
    }
    matrix
        .axes
        .get(axis)
        .is_some_and(|vals| !vals.is_empty() && vals.iter().all(pred))
}

fn matrix_axis_all_non_empty_strings(matrix: &Matrix, axis: &str) -> bool {
    matrix_axis_all_match(matrix, axis, |v| v.as_literal().is_some())
}

fn matrix_axis_all_plain_literals(matrix: &Matrix, axis: &str) -> bool {
    matrix_axis_all_match(matrix, axis, |v| {
        v.as_str().is_some_and(is_plain_artifact_literal)
    })
}

fn matrix_axis_bare_truthy_valid(matrix: &Matrix, axis: &str) -> bool {
    if matrix.axes.is_empty() {
        return !matrix.include.is_empty()
            && matrix
                .include
                .iter()
                .all(|row| row.get(axis).is_none_or(|v| v.as_literal().is_some()))
            && matrix
                .include
                .iter()
                .any(|row| row.get(axis).is_some_and(|v| v.as_literal().is_some()));
    }
    matrix_axis_all_non_empty_strings(matrix, axis)
}

fn validate_condition_static(
    wf: &Workflow,
    job: &Job,
    step_idx: Option<usize>,
    cond: &ClosedCondition,
    file: &str,
    place: &str,
) -> Result<(), ClosedFormRefusal> {
    match &cond.body {
        ConditionBody::Unconditional => Ok(()),
        ConditionBody::Bare(path) => validate_bare_path(wf, job, path, file, place),
        ConditionBody::All(cmps) | ConditionBody::Any(cmps) => cmps
            .iter()
            .try_for_each(|cmp| validate_comparison_static(wf, job, step_idx, cmp, file, place)),
    }
}

fn validate_bare_path(
    wf: &Workflow,
    job: &Job,
    path: &ContextPath,
    file: &str,
    place: &str,
) -> Result<(), ClosedFormRefusal> {
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
                    "bare matrix axis `matrix.{axis}` does not name a non-empty string row with all other rows non-empty strings or omitted"
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

fn validate_comparison_static(
    wf: &Workflow,
    job: &Job,
    step_idx: Option<usize>,
    cmp: &Comparison,
    file: &str,
    place: &str,
) -> Result<(), ClosedFormRefusal> {
    match &cmp.path {
        ContextPath::GithubEventName | ContextPath::RunnerEnvironment => Ok(()),
        ContextPath::Matrix(axis) => {
            if job
                .matrix
                .as_ref()
                .is_some_and(|m| matrix_axis_all_non_empty_strings(m, axis))
            {
                return Ok(());
            }
            Err(ClosedFormRefusal::new(
                file,
                place,
                RefusalKind::ConditionTypeMismatch,
                format!(
                    "comparison on `matrix.{axis}` which is not a non-empty string in every row"
                ),
                ALLOWED_COMPARISONS,
            ))
        }
        ContextPath::Input(name) => {
            let ty = wf
                .workflow_call_input(name)
                .and_then(|i| i.input_type.as_ref())
                .and_then(Scalar::as_literal);
            if matches!(ty, Some("string" | "choice")) {
                return Ok(());
            }
            Err(ClosedFormRefusal::new(
                file,
                place,
                RefusalKind::ConditionTypeMismatch,
                format!(
                    "comparison on `inputs.{name}` which is not declared `type: string` or `choice`"
                ),
                ALLOWED_COMPARISONS,
            ))
        }
        ContextPath::NeedsResult(dep) => {
            if job.needs.iter().any(|n| n == dep) && wf.job(dep).is_some() {
                return Ok(());
            }
            Err(ClosedFormRefusal::new(
                file,
                place,
                RefusalKind::ConditionUnbound,
                format!(
                    "comparison on `needs.{dep}.result` where `{dep}` is not in this job's `needs`"
                ),
                ALLOWED_PATHS,
            ))
        }
        ContextPath::NeedsOutput { job: dep, output } => {
            let ok = job.needs.iter().any(|n| n == dep)
                && wf
                    .job(dep)
                    .is_some_and(|j| j.outputs.iter().any(|o| o == output));
            if ok {
                return Ok(());
            }
            Err(ClosedFormRefusal::new(
                file,
                place,
                RefusalKind::ConditionUnbound,
                format!(
                    "comparison on `needs.{dep}.outputs.{output}` not declared on a needed job"
                ),
                ALLOWED_PATHS,
            ))
        }
        ContextPath::StepOutput { step: step_id, .. } => {
            let ok = step_idx.is_some_and(|idx| {
                job.steps[..idx]
                    .iter()
                    .any(|prev| prev.id.as_deref() == Some(step_id.as_str()))
            });
            if ok {
                return Ok(());
            }
            Err(ClosedFormRefusal::new(
                file,
                place,
                RefusalKind::ConditionUnbound,
                format!(
                    "comparison on `steps.{step_id}.outputs.*` without an earlier step id `{step_id}`"
                ),
                ALLOWED_PATHS,
            ))
        }
    }
}

fn parse_artifact_name(
    raw: &Scalar,
    matrix: Option<&Matrix>,
    file: &str,
    place: &str,
) -> Result<ArtifactName, ClosedFormRefusal> {
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
        return Ok(ArtifactName::Literal(s.to_string()));
    }
    if let Some(inner) = s.strip_prefix("${{").and_then(|r| r.strip_suffix("}}")) {
        if let Some(axis) = inner
            .trim()
            .strip_prefix("matrix.")
            .filter(|a| is_ident_segment(a))
        {
            if matrix.is_some_and(|m| matrix_axis_all_plain_literals(m, axis)) {
                return Ok(ArtifactName::MatrixAxis(axis.to_string()));
            }
        }
    }
    if let Some(prefix) = s.strip_suffix("${{ strategy.job-index }}") {
        if is_plain_artifact_literal(prefix) {
            return Ok(ArtifactName::JobIndex {
                prefix: prefix.to_string(),
            });
        }
    }
    Err(refuse(s.to_string()))
}

fn parse_artifact_pattern(
    raw: &Scalar,
    file: &str,
    place: &str,
) -> Result<ArtifactPattern, ClosedFormRefusal> {
    let Some(s) = raw.as_str() else {
        return Err(ClosedFormRefusal::new(
            file,
            place,
            RefusalKind::ArtifactName,
            format!("artifact pattern `{raw:?}`"),
            ALLOWED_ARTIFACT_NAMES,
        ));
    };
    if is_plain_artifact_literal(s) {
        return Ok(ArtifactPattern::Literal(s.to_string()));
    }
    if let Some(prefix) = s.strip_suffix('*') {
        if is_plain_artifact_literal(prefix) {
            return Ok(ArtifactPattern::PrefixWildcard(prefix.to_string()));
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

fn parse_step_artifacts(
    job: &Job,
    step: &Step,
    step_idx: usize,
    file: &str,
) -> Result<(Option<ArtifactName>, Option<ArtifactPattern>), ClosedFormRefusal> {
    let Some(uses) = step.uses.as_deref() else {
        return Ok((None, None));
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
        return Ok((None, None));
    }
    let prefix = format!("job {} step {}", job.id, step_idx + 1);
    let name = step
        .with_value("name")
        .map(|raw| {
            parse_artifact_name(
                raw,
                job.matrix.as_ref(),
                file,
                &format!("{prefix} with.name"),
            )
        })
        .transpose()?;
    let pattern = if is_download {
        step.with_value("pattern")
            .map(|raw| parse_artifact_pattern(raw, file, &format!("{prefix} with.pattern")))
            .transpose()?
    } else {
        None
    };
    Ok((name, pattern))
}
