//! A workflow's jobs and YAML structure, for the test files that hold the
//! repository's workflows to their rulings. `tests/it.rs` declares this once,
//! so the rules that split a workflow into jobs, mappings and steps have one
//! home (decision 0071 ruling 5).

use std::collections::BTreeMap;

/// A parsed YAML mapping block, failing closed on flow maps, merge keys,
/// duplicate keys, or other constructs this test reader does not evaluate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum YamlMap {
    Entries(BTreeMap<String, String>),
    Unreadable(String),
}

/// The `steps:` block of a workflow job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ParsedSteps {
    Items(Vec<ParsedStep>),
    Unreadable(String),
}

/// Structurally parsed fields of a single workflow step.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ParsedStep {
    pub(crate) uses: Option<String>,
    pub(crate) if_cond: Option<String>,
    pub(crate) with: Option<YamlMap>,
    pub(crate) unreadable: Option<String>,
}

/// Structurally parsed fields of a single workflow job.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ParsedJob {
    pub(crate) uses: Option<String>,
    pub(crate) if_cond: Option<String>,
    pub(crate) with: Option<YamlMap>,
    pub(crate) steps: Option<ParsedSteps>,
    pub(crate) unreadable: Option<String>,
}

/// Result of comparing a parsed `uses:` action reference against an expected
/// `<owner>/<repo>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ActionMatch {
    Exact,
    SubAction(String),
    Other,
}

#[derive(Debug, Clone)]
enum LineEntry<'a> {
    Empty,
    Pair(String, &'a str),
    Flow(&'a str),
    Other(&'a str),
}

#[derive(Debug, Clone)]
struct YamlLine<'a> {
    indent: usize,
    content_col: usize,
    is_seq_item: bool,
    entry: LineEntry<'a>,
}

/// The jobs of a workflow's text, keyed by job id, in file order. A job
/// starts at a key indented two spaces under `jobs:`; its body is every
/// line after that key up to the next one, a comment between two jobs
/// included in the first.
pub(crate) fn jobs(workflow: &str) -> Vec<(String, String)> {
    let mut lines = workflow.lines();
    let found_jobs = lines.any(|line| {
        !line.starts_with([' ', '\t', '#'])
            && line
                .split_once('#')
                .map_or(line, |(head, _)| head)
                .trim_end()
                == "jobs:"
    });
    assert!(found_jobs, "a jobs map");
    let mut parsed: Vec<(String, String)> = Vec::new();
    for line in lines {
        if !line.is_empty() && !line.starts_with([' ', '\t', '#']) {
            break;
        }
        let id = line
            .strip_prefix("  ")
            .filter(|rest| !rest.starts_with([' ', '#']))
            .and_then(|rest| {
                rest.split_once('#')
                    .map_or(rest, |(head, _)| head)
                    .trim_end()
                    .strip_suffix(':')
            });
        match (id, parsed.last_mut()) {
            (Some(id), _) => parsed.push((id.to_string(), String::new())),
            (None, Some((_, body))) => {
                body.push_str(line);
                body.push('\n');
            }
            (None, None) => {}
        }
    }
    parsed
}

/// Parse a single-line YAML scalar (allowing `${{ ... }}` expressions),
/// returning `None` for empty values, block scalars, or YAML forms we do not
/// evaluate (anchors, flow collections, double-quoted escapes).
pub(crate) fn parse_yaml_scalar(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let value = if let Some(rest) = raw.strip_prefix('"') {
        parse_double_quoted_yaml(rest)?
    } else if let Some(rest) = raw.strip_prefix('\'') {
        parse_single_quoted_yaml(rest)?
    } else {
        parse_plain_yaml(raw)?
    };
    (!value.is_empty()).then_some(value)
}

/// Parse a single-line YAML scalar into an exact literal string, rejecting
/// GitHub Actions `${{ ... }}` expressions (decision 0071 rulings 5 and 9).
pub(crate) fn parse_yaml_literal(raw: &str) -> Option<String> {
    parse_yaml_scalar(raw).filter(|value| !value.contains("${{"))
}

/// Parse an unquoted YAML boolean (`true`/`false`, case-insensitive, with
/// optional trailing `# comment`), failing closed on quoted strings,
/// expressions, or any other value.
pub(crate) fn parse_yaml_bool(raw: &str) -> Result<bool, String> {
    let trimmed = raw.trim();
    if trimmed.starts_with(['\'', '"']) {
        return Err(format!("`{trimmed}` is quoted, not a YAML boolean"));
    }
    let Some(plain) = parse_plain_yaml(trimmed) else {
        return Err(format!("`{trimmed}` is not a plain YAML boolean"));
    };
    if plain.eq_ignore_ascii_case("true") {
        Ok(true)
    } else if plain.eq_ignore_ascii_case("false") {
        Ok(false)
    } else {
        Err(format!("`{trimmed}` is not a YAML boolean"))
    }
}

/// Whether `raw_uses` references `repo@<ref>` or a sub-action
/// `repo/<subpath>@<ref>` (case-insensitive). Returns `None` if `raw_uses`
/// cannot be parsed as a YAML literal.
pub(crate) fn action_repo_match(raw_uses: &str, expected_repo: &str) -> Option<ActionMatch> {
    let used = parse_yaml_literal(raw_uses)?;
    let repo = used
        .split_once('@')
        .map_or(used.as_str(), |(repo, _)| repo)
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

fn parse_double_quoted_yaml(rest: &str) -> Option<String> {
    let (inner, after) = rest.split_once('"')?;
    if inner.contains('\\') {
        return None;
    }
    let tail = after.trim_start();
    (tail.is_empty() || tail.starts_with('#')).then(|| inner.to_string())
}

fn parse_single_quoted_yaml(rest: &str) -> Option<String> {
    let mut out = String::new();
    let mut chars = rest.char_indices().peekable();
    while let Some((idx, ch)) = chars.next() {
        if ch != '\'' {
            out.push(ch);
            continue;
        }
        if matches!(chars.peek(), Some(&(_, '\''))) {
            chars.next();
            out.push('\'');
            continue;
        }
        let tail = rest[idx + 1..].trim_start();
        return (tail.is_empty() || tail.starts_with('#')).then_some(out);
    }
    None
}

fn parse_plain_yaml(raw: &str) -> Option<String> {
    let comment_at = raw.char_indices().find_map(|(i, ch)| {
        let prev_ws = i == 0
            || raw[..i]
                .chars()
                .next_back()
                .is_some_and(char::is_whitespace);
        (ch == '#' && prev_ws).then_some(i)
    });
    let value = match comment_at {
        Some(idx) => raw[..idx].trim_end(),
        None => raw,
    };
    let first = value.chars().next()?;
    if matches!(
        first,
        '&' | '*' | '!' | '{' | '}' | '[' | ']' | '|' | '>' | '%' | '@' | '`' | ',' | '?' | ':'
    ) {
        return None;
    }
    Some(value.to_string())
}

fn is_empty_yaml_value(val: &str) -> bool {
    let trimmed = val.trim();
    trimmed.is_empty() || trimmed.starts_with('#')
}

fn is_block_header(value: &str) -> bool {
    let header = value.split_once(" #").map_or(value, |(h, _)| h).trim();
    header
        .strip_prefix(['|', '>'])
        .is_some_and(|ind| ind.chars().all(|c| matches!(c, '1'..='9' | '+' | '-')))
}

fn split_quoted_key(rest: &str) -> Option<(String, &str)> {
    let quote = rest.chars().next()?;
    let mut key = String::new();
    let mut chars = rest[1..].char_indices().peekable();
    while let Some((idx, ch)) = chars.next() {
        if ch == '\\' && quote == '"' {
            return None;
        }
        if ch != quote {
            key.push(ch);
            continue;
        }
        if quote == '\'' && matches!(chars.peek(), Some(&(_, '\''))) {
            chars.next();
            key.push('\'');
            continue;
        }
        let after = rest[idx + 2..].trim_start();
        let after_colon = after.strip_prefix(':')?;
        if after_colon.is_empty() || after_colon.starts_with(char::is_whitespace) {
            return Some((key, after_colon.trim()));
        }
        return None;
    }
    None
}

fn parse_line_entry(rest: &str) -> LineEntry<'_> {
    if rest.is_empty() || rest.starts_with('#') {
        return LineEntry::Empty;
    }
    if rest.starts_with(['{', '[']) {
        return LineEntry::Flow(rest);
    }
    if rest.starts_with(['"', '\'']) {
        return match split_quoted_key(rest) {
            Some((k, v)) => LineEntry::Pair(k, v),
            None => LineEntry::Other(rest),
        };
    }
    if rest.starts_with(['?', '*', '&', '!']) {
        return LineEntry::Other(rest);
    }
    let sep = rest.char_indices().find(|&(at, ch)| {
        ch == ':'
            && rest[at + 1..]
                .chars()
                .next()
                .is_none_or(char::is_whitespace)
    });
    match sep {
        Some((at, _)) => {
            let key = rest[..at].trim_end();
            if key.is_empty() || key.contains('#') {
                LineEntry::Other(rest)
            } else {
                LineEntry::Pair(key.to_string(), rest[at + 1..].trim())
            }
        }
        None => LineEntry::Other(rest),
    }
}

fn scan_yaml_lines(text: &str) -> Vec<YamlLine<'_>> {
    let mut out = Vec::new();
    let mut block_col: Option<usize> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        if block_col.is_some_and(|col| trimmed.is_empty() || indent > col) {
            continue;
        }
        block_col = None;
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let (is_seq_item, rest) = match trimmed.strip_prefix('-') {
            Some(after) if after.is_empty() || after.starts_with(char::is_whitespace) => {
                (true, after.trim_start())
            }
            _ => (false, trimmed),
        };
        let content_col = line.len() - rest.len();
        let entry = parse_line_entry(rest);
        if let LineEntry::Pair(_, val) | LineEntry::Other(val) = &entry {
            if is_block_header(val) {
                block_col = Some(content_col);
            }
        }
        out.push(YamlLine {
            indent,
            content_col,
            is_seq_item,
            entry,
        });
    }
    out
}

fn parse_yaml_map(header_val: &str, parent_col: usize, rest: &[YamlLine<'_>]) -> YamlMap {
    if !is_empty_yaml_value(header_val) {
        return YamlMap::Unreadable(header_val.to_string());
    }
    let end = rest
        .iter()
        .position(|line| line.indent <= parent_col)
        .unwrap_or(rest.len());
    let block = &rest[..end];
    let Some(first) = block.first() else {
        return YamlMap::Entries(BTreeMap::new());
    };
    let child_indent = first.indent;
    let mut map = BTreeMap::new();
    for line in block {
        if line.indent > child_indent {
            continue;
        }
        if line.indent < child_indent || line.is_seq_item {
            return YamlMap::Unreadable(
                "unexpected indentation or sequence in mapping".to_string(),
            );
        }
        match &line.entry {
            LineEntry::Pair(key, _) if key == "<<" => {
                return YamlMap::Unreadable("<<".to_string());
            }
            LineEntry::Pair(key, val) => {
                if map.insert(key.clone(), (*val).to_string()).is_some() {
                    return YamlMap::Unreadable(format!("duplicate key `{key}`"));
                }
            }
            LineEntry::Flow(raw) | LineEntry::Other(raw) => {
                return YamlMap::Unreadable((*raw).to_string());
            }
            LineEntry::Empty => return YamlMap::Unreadable("empty mapping line".to_string()),
        }
    }
    YamlMap::Entries(map)
}

/// Look up a nested block mapping at `path` (e.g. `&["concurrency"]` or
/// `&["on", "workflow_call", "inputs", "use-cache"]`).
pub(crate) fn nested_map(text: &str, path: &[&str]) -> Option<YamlMap> {
    if path.is_empty() {
        return None;
    }
    let lines = scan_yaml_lines(text);
    let mut scope: &[YamlLine<'_>] = &lines;
    let mut expected_indent = Some(0);
    for (depth, &target) in path.iter().enumerate() {
        let level_indent = match expected_indent {
            Some(indent) => indent,
            None => scope.first()?.indent,
        };
        let pos = scope.iter().position(|line| {
            line.indent == level_indent
                && !line.is_seq_item
                && matches!(&line.entry, LineEntry::Pair(k, _) if k == target)
        })?;
        let LineEntry::Pair(_, val) = scope[pos].entry else {
            return None;
        };
        let rest = &scope[pos + 1..];
        let end = rest
            .iter()
            .position(|line| line.indent <= level_indent)
            .unwrap_or(rest.len());
        let children = &rest[..end];
        if depth + 1 == path.len() {
            return Some(parse_yaml_map(val, level_indent, children));
        }
        if !is_empty_yaml_value(val) {
            return Some(YamlMap::Unreadable(val.to_string()));
        }
        scope = children;
        expected_indent = None;
    }
    None
}

fn assign_once<T>(slot: &mut Option<T>, value: T, unreadable: &mut Option<String>, key: &str) {
    if slot.is_some() {
        *unreadable = Some(format!("duplicate `{key}` key"));
    } else {
        *slot = Some(value);
    }
}

fn apply_step_entry(
    step: &mut ParsedStep,
    entry: &LineEntry<'_>,
    step_col: usize,
    rest: &[YamlLine<'_>],
) {
    match entry {
        LineEntry::Pair(key, val) => match key.as_str() {
            "uses" => assign_once(
                &mut step.uses,
                (*val).to_string(),
                &mut step.unreadable,
                "uses",
            ),
            "if" => assign_once(
                &mut step.if_cond,
                (*val).to_string(),
                &mut step.unreadable,
                "if",
            ),
            "with" => {
                let map = parse_yaml_map(val, step_col, rest);
                assign_once(&mut step.with, map, &mut step.unreadable, "with");
            }
            _ => {}
        },
        LineEntry::Flow(raw) | LineEntry::Other(raw) => {
            step.unreadable = Some((*raw).to_string());
        }
        LineEntry::Empty => {}
    }
}

fn parse_step(chunk: &[YamlLine<'_>]) -> ParsedStep {
    let mut step = ParsedStep::default();
    let (step_col, start) = match &chunk[0].entry {
        LineEntry::Empty => match chunk.get(1) {
            Some(next) if !next.is_seq_item => (next.indent, 1),
            _ => {
                step.unreadable = Some("empty or nested-sequence step item".to_string());
                return step;
            }
        },
        LineEntry::Pair(..) => (chunk[0].content_col, 0),
        LineEntry::Flow(raw) | LineEntry::Other(raw) => {
            step.unreadable = Some((*raw).to_string());
            return step;
        }
    };
    for idx in start..chunk.len() {
        let line = &chunk[idx];
        let col = if idx == 0 {
            line.content_col
        } else {
            line.indent
        };
        if col > step_col {
            continue;
        }
        if col < step_col || (idx > 0 && line.is_seq_item) {
            step.unreadable = Some("unexpected indentation in step".to_string());
            return step;
        }
        apply_step_entry(&mut step, &line.entry, step_col, &chunk[idx + 1..]);
    }
    step
}

fn parse_steps(header_val: &str, job_indent: usize, rest: &[YamlLine<'_>]) -> ParsedSteps {
    if !is_empty_yaml_value(header_val) {
        return ParsedSteps::Unreadable(header_val.to_string());
    }
    let end = rest
        .iter()
        .position(|line| {
            line.indent < job_indent || (line.indent == job_indent && !line.is_seq_item)
        })
        .unwrap_or(rest.len());
    let block = &rest[..end];
    let Some(first) = block.first() else {
        return ParsedSteps::Items(Vec::new());
    };
    if !first.is_seq_item {
        return ParsedSteps::Unreadable(
            "steps block does not start with a sequence item".to_string(),
        );
    }
    let dash_indent = first.indent;
    let mut starts = Vec::new();
    for (idx, line) in block.iter().enumerate() {
        if line.indent < dash_indent || (line.indent == dash_indent && !line.is_seq_item) {
            return ParsedSteps::Unreadable("unexpected indentation inside steps".to_string());
        }
        if line.indent == dash_indent && line.is_seq_item {
            starts.push(idx);
        }
    }
    let mut items = Vec::with_capacity(starts.len());
    for (pos, &start) in starts.iter().enumerate() {
        let stop = starts.get(pos + 1).copied().unwrap_or(block.len());
        items.push(parse_step(&block[start..stop]));
    }
    ParsedSteps::Items(items)
}

/// Structurally parse a single job body returned by [`jobs`].
pub(crate) fn parse_job(body: &str) -> ParsedJob {
    let lines = scan_yaml_lines(body);
    let mut job = ParsedJob::default();
    let Some(first) = lines.first() else {
        return job;
    };
    let job_indent = first.indent;
    for (idx, line) in lines.iter().enumerate() {
        if line.indent > job_indent || line.is_seq_item {
            continue;
        }
        if line.indent < job_indent {
            job.unreadable = Some("unexpected indentation in job".to_string());
            break;
        }
        match &line.entry {
            LineEntry::Pair(key, val) => match key.as_str() {
                "uses" => {
                    assign_once(
                        &mut job.uses,
                        (*val).to_string(),
                        &mut job.unreadable,
                        "uses",
                    );
                }
                "if" => {
                    assign_once(
                        &mut job.if_cond,
                        (*val).to_string(),
                        &mut job.unreadable,
                        "if",
                    );
                }
                "with" => {
                    let map = parse_yaml_map(val, job_indent, &lines[idx + 1..]);
                    assign_once(&mut job.with, map, &mut job.unreadable, "with");
                }
                "steps" => {
                    let steps = parse_steps(val, job_indent, &lines[idx + 1..]);
                    assign_once(&mut job.steps, steps, &mut job.unreadable, "steps");
                }
                _ => {}
            },
            LineEntry::Flow(raw) | LineEntry::Other(raw) => {
                job.unreadable = Some((*raw).to_string());
            }
            LineEntry::Empty => {}
        }
    }
    job
}
