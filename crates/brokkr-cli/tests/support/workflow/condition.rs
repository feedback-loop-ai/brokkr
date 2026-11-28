//! Closed-form `if:` condition parser, refusal types, and opaque `Verdict`
//! evaluator (#1417, decision 0071 rulings 2, 3 and 8).

use std::collections::BTreeMap;

use super::{Scalar, WorkflowError};

/// Machine-readable category for a closed-form refusal (decision 0071 rulings 2 and 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RefusalKind {
    WorkflowEnv,
    TopLevelKey,
    EventFilter,
    ConditionSyntax,
    ConditionLiteral,
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

pub(super) const ALLOWED_ENV: &str = "job-level or step-level env only (no workflow-level env)";
pub(super) const ALLOWED_TOP_LEVEL: &str = "name, on, permissions, concurrency, or jobs";
pub(super) const ALLOWED_EVENT_FILTERS: &str = "unfiltered event, or branches, tags, types, schedule cron (with no other keys), or workflow_call inputs";
pub(super) const ALLOWED_CONDITIONS: &str = "omitted, always(), comparisons joined by && (optionally prefixed by always() &&), comparisons joined by ||, or one bare boolean input or string matrix axis, without ${{ }}";
pub(super) const ALLOWED_LITERALS: &str =
    "non-empty ASCII string of letters, digits, `.`, `_`, `/` and `-` that is not a number";
pub(super) const ALLOWED_BARE_PATHS: &str = "bare inputs.<name> declared type: boolean, or bare matrix.<axis> with at least one non-empty string row and every other row a non-empty string or omitted";
pub(super) const ALLOWED_PATHS: &str = "github.event_name, runner.environment, matrix.<axis> (non-empty string in every row), inputs.<name> (type: string or choice), needs.<job>.result, needs.<job>.outputs.<name> (declared on a needed job), or steps.<id>.outputs.<name> (from an earlier step on a step if)";
pub(super) const ALLOWED_COMPARISONS: &str = "a context path statically known to be a string compared with == or != against a non-empty non-numeric ASCII literal";
pub(super) const ALLOWED_ARTIFACT_NAMES: &str = "a plain [A-Za-z0-9._-]+ literal with no blanks, ${{ matrix.<axis> }} over plain literals, <literal>${{ strategy.job-index }}, or for pattern a plain literal or <literal>*";
pub(super) const ALLOWED_MATRIX: &str = "non-empty matrix axes with no exclude, and include only on ci.yml pr-caches or release.yml build";

impl ClosedFormRefusal {
    pub(super) fn new(
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

/// Final evaluation outcome of a closed-form condition.
///
/// Intentionally opaque and intentionally does NOT implement `PartialEq`, `Eq`,
/// `Copy`, or `Default`, and offers no boolean/unwrap getter: the only ways to
/// observe a `Verdict` combine verdicts without dropping refusals or convert
/// `Refused(r)` unconditionally into `P: From<ClosedFormRefusal>`.
#[must_use = "a closed-form evaluation refusal is final and must be propagated"]
#[derive(Debug)]
pub(crate) struct Verdict(VerdictInner);

#[derive(Debug)]
enum VerdictInner {
    Allowed,
    Skipped,
    Refused(ClosedFormRefusal),
}

impl Verdict {
    pub(crate) fn from_bool(allowed: bool) -> Self {
        if allowed {
            Self(VerdictInner::Allowed)
        } else {
            Self(VerdictInner::Skipped)
        }
    }

    pub(super) fn refused(refusal: ClosedFormRefusal) -> Self {
        Self(VerdictInner::Refused(refusal))
    }

    /// Conjunction that evaluates both operands and preserves any refusal.
    pub(crate) fn and(self, other: Self) -> Self {
        match (self.0, other.0) {
            (VerdictInner::Refused(r), _) | (_, VerdictInner::Refused(r)) => {
                Self(VerdictInner::Refused(r))
            }
            (VerdictInner::Allowed, VerdictInner::Allowed) => Self(VerdictInner::Allowed),
            _ => Self(VerdictInner::Skipped),
        }
    }

    /// Negation that preserves any refusal.
    pub(crate) fn not(self) -> Self {
        match self.0 {
            VerdictInner::Allowed => Self(VerdictInner::Skipped),
            VerdictInner::Skipped => Self(VerdictInner::Allowed),
            VerdictInner::Refused(r) => Self(VerdictInner::Refused(r)),
        }
    }

    fn fold_iter(
        iter: impl IntoIterator<Item = Self>,
        init: bool,
        combine: fn(bool, bool) -> bool,
    ) -> Self {
        let mut acc = init;
        let mut first_refusal = None;
        for item in iter {
            match item.0 {
                VerdictInner::Allowed => acc = combine(acc, true),
                VerdictInner::Skipped => acc = combine(acc, false),
                VerdictInner::Refused(r) => {
                    if first_refusal.is_none() {
                        first_refusal = Some(r);
                    }
                }
            }
        }
        match first_refusal {
            Some(r) => Self(VerdictInner::Refused(r)),
            None => Self::from_bool(acc),
        }
    }

    /// Evaluate every verdict in `iter`; any refusal wins over `Allowed`/`Skipped`.
    pub(crate) fn all(iter: impl IntoIterator<Item = Self>) -> Self {
        Self::fold_iter(iter, true, |a, b| a && b)
    }

    /// Evaluate every verdict in `iter`; any refusal wins over `Allowed`/`Skipped`.
    pub(crate) fn any(iter: impl IntoIterator<Item = Self>) -> Self {
        Self::fold_iter(iter, false, |a, b| a || b)
    }

    /// Require `Allowed`: `Skipped` produces `on_false()`, and `Refused(r)`
    /// always propagates `P::from(r)`.
    pub(crate) fn expect_true<P: From<ClosedFormRefusal>>(
        self,
        on_false: impl FnOnce() -> P,
    ) -> Option<P> {
        match self.0 {
            VerdictInner::Allowed => None,
            VerdictInner::Skipped => Some(on_false()),
            VerdictInner::Refused(r) => Some(P::from(r)),
        }
    }

    /// Require `Skipped`: `Allowed` produces `on_true()`, and `Refused(r)`
    /// always propagates `P::from(r)`.
    pub(crate) fn expect_false<P: From<ClosedFormRefusal>>(
        self,
        on_true: impl FnOnce() -> P,
    ) -> Option<P> {
        match self.0 {
            VerdictInner::Skipped => None,
            VerdictInner::Allowed => Some(on_true()),
            VerdictInner::Refused(r) => Some(P::from(r)),
        }
    }

    /// Run `on_allowed` iff `Allowed`; `Skipped` does nothing; `Refused(r)`
    /// always propagates `P::from(r)`.
    pub(crate) fn visit_when_allowed<P: From<ClosedFormRefusal>>(
        self,
        on_allowed: impl FnOnce(),
    ) -> Option<P> {
        match self.0 {
            VerdictInner::Allowed => {
                on_allowed();
                None
            }
            VerdictInner::Skipped => None,
            VerdictInner::Refused(r) => Some(P::from(r)),
        }
    }
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
    pub(super) fn display_name(&self) -> String {
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

/// Typed representation of an admitted closed-form GitHub Actions `if:` condition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClosedCondition {
    file: String,
    place: String,
    raw: Option<String>,
    pub(crate) always: bool,
    pub(crate) body: ConditionBody,
}

impl ClosedCondition {
    pub(crate) fn raw(&self) -> Option<&str> {
        self.raw.as_deref()
    }

    /// Parse a closed-form GitHub Actions `if:` condition from its raw YAML scalar.
    pub(super) fn parse_raw(
        raw_scalar: Option<&Scalar>,
        file: &str,
        place: &str,
    ) -> Result<Self, ClosedFormRefusal> {
        let Some(scalar) = raw_scalar else {
            return Ok(Self {
                file: file.to_string(),
                place: place.to_string(),
                raw: None,
                always: false,
                body: ConditionBody::Unconditional,
            });
        };
        let Some(raw) = scalar.as_str() else {
            return Err(ClosedFormRefusal::new(
                file,
                place,
                RefusalKind::ConditionSyntax,
                format!("`if:` value of kind {}", scalar.kind_label()),
                ALLOWED_CONDITIONS,
            ));
        };
        Self::parse_str(raw, file, place)
    }

    pub(crate) fn parse_str(raw: &str, file: &str, place: &str) -> Result<Self, ClosedFormRefusal> {
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
                    file: file.to_string(),
                    place: place.to_string(),
                    raw: Some(raw.to_string()),
                    always,
                    body: ConditionBody::All(parse_all(rest)?),
                })
            }
            Some(LogicOp::Or) => Ok(Self {
                file: file.to_string(),
                place: place.to_string(),
                raw: Some(raw.to_string()),
                always: false,
                body: ConditionBody::Any(parse_all(&clauses)?),
            }),
        }
    }

    /// Single evaluator for closed-form conditions: applies GitHub's implicit
    /// `success() && (...)` whenever leading `always()` is absent, and never
    /// drops or short-circuits past an evaluation refusal.
    pub(crate) fn evaluate(&self, ctx: &ConditionContext) -> Verdict {
        let raw = self.raw.as_deref().unwrap_or("");
        let body_verdict = match &self.body {
            ConditionBody::Unconditional => Verdict::from_bool(true),
            ConditionBody::All(cmps) => Verdict::all(
                cmps.iter()
                    .map(|c| eval_comparison(c, ctx, &self.file, &self.place, raw)),
            ),
            ConditionBody::Any(cmps) => Verdict::any(
                cmps.iter()
                    .map(|c| eval_comparison(c, ctx, &self.file, &self.place, raw)),
            ),
            ConditionBody::Bare(path) => eval_bare(path, ctx, &self.file, &self.place, raw),
        };
        if self.always {
            body_verdict
        } else {
            Verdict::from_bool(ctx.prior_succeeded).and(body_verdict)
        }
    }
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

pub(super) fn is_ident_segment(seg: &str) -> bool {
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

/// Non-empty ASCII string of letters, digits, `.`, `_`, `/` and `-` that is not
/// a number (decimal, scientific, signed, `NaN`, `inf`/`infinity`, or `0x`/`0b`/`0o`).
fn is_closed_comparison_literal(s: &str) -> bool {
    if s.is_empty()
        || !s
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'/' | b'-'))
    {
        return false;
    }
    if s.parse::<f64>().is_ok() {
        return false;
    }
    let unsigned = s.strip_prefix('-').unwrap_or(s);
    let is_radix = |p1: &str, p2: &str, pred: fn(&u8) -> bool| {
        unsigned
            .strip_prefix(p1)
            .or_else(|| unsigned.strip_prefix(p2))
            .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| pred(&b)))
    };
    !(is_radix("0x", "0X", u8::is_ascii_hexdigit)
        || is_radix("0b", "0B", |b| matches!(b, b'0' | b'1'))
        || is_radix("0o", "0O", |b| matches!(b, b'0'..=b'7')))
}

pub(super) fn read_single_quoted_literal(src: &str, start: usize) -> Option<(String, usize)> {
    let bytes = src.as_bytes();
    if bytes.get(start) != Some(&b'\'') {
        return None;
    }
    let mut out = String::new();
    let mut i = start + 1;
    while i < bytes.len() {
        if bytes[i] == b'\'' {
            if bytes.get(i + 1) == Some(&b'\'') {
                out.push('\'');
                i += 2;
                continue;
            }
            return Some((out, i + 1));
        }
        let ch = src[i..].chars().next()?;
        out.push(ch);
        i += ch.len_utf8();
    }
    None
}

fn parse_single_quoted_literal(raw: &str) -> Option<String> {
    let (lit, end) = read_single_quoted_literal(raw, 0)?;
    (end == raw.len()).then_some(lit)
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
    if !is_closed_comparison_literal(&literal) {
        return Err(ClosedFormRefusal::new(
            file,
            place,
            RefusalKind::ConditionLiteral,
            format!("comparison literal `{literal}` in `{raw}`"),
            ALLOWED_LITERALS,
        ));
    }
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
            file: file.to_string(),
            place: place.to_string(),
            raw: Some(raw.to_string()),
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
            file: file.to_string(),
            place: place.to_string(),
            raw: Some(raw.to_string()),
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
        file: file.to_string(),
        place: place.to_string(),
        raw: Some(raw.to_string()),
        always: false,
        body: ConditionBody::Bare(path),
    })
}

fn eval_comparison(
    cmp: &Comparison,
    ctx: &ConditionContext,
    file: &str,
    place: &str,
    raw: &str,
) -> Verdict {
    let Some(val) = ctx.lookup(&cmp.path) else {
        return Verdict::refused(ClosedFormRefusal::new(
            file,
            place,
            RefusalKind::ConditionUnbound,
            format!("unbound context `{}` in `{raw}`", cmp.path.display_name()),
            ALLOWED_PATHS,
        ));
    };
    let eq = match val {
        CondVal::Str(s) => s.eq_ignore_ascii_case(&cmp.literal),
        CondVal::Absent => false,
        CondVal::Bool(_) => {
            return Verdict::refused(ClosedFormRefusal::new(
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
    Verdict::from_bool(match cmp.op {
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
) -> Verdict {
    let Some(val) = ctx.lookup(path) else {
        return Verdict::refused(ClosedFormRefusal::new(
            file,
            place,
            RefusalKind::ConditionUnbound,
            format!("unbound bare context `{}` in `{raw}`", path.display_name()),
            ALLOWED_PATHS,
        ));
    };
    match (path, val) {
        (ContextPath::Input(_), CondVal::Bool(b)) => Verdict::from_bool(b),
        (ContextPath::Matrix(_), CondVal::Str(s)) if !s.is_empty() => Verdict::from_bool(true),
        (ContextPath::Matrix(_), CondVal::Absent) => Verdict::from_bool(false),
        _ => Verdict::refused(ClosedFormRefusal::new(
            file,
            place,
            RefusalKind::ConditionBarePath,
            format!("bare condition `{raw}` has non-admitted value"),
            ALLOWED_BARE_PATHS,
        )),
    }
}
