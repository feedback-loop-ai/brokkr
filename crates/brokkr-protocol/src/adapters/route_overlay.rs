//! The DSH route overlay (AS3; design D6 mechanism 1): the one authorized
//! `--patch` shape, read and validated by a bounded line reader.
//!
//! This is deliberately not a YAML implementation. dsh parses a patch with
//! a schema that turns a `!!js` tagged scalar into an expression node and
//! evaluates any mapping holding a `__jsExpr` key when the entry activates,
//! so a reader that carried an unrecognized line verbatim at any depth
//! would forward executable syntax through a passing check. The reader
//! recognizes the whole admitted grammar — comments, blank lines, `<key>:
//! <value>`, `<key>:` and the sequence item `- <key>: <value>`, two spaces
//! per depth — and refuses every line it has not recognized, at any depth,
//! naming the depth and never the text.
//!
//! Authorization is separate from content. The engine binds the argv value
//! to the compiled leaf layer and carries the manifest digest; this module
//! reads the file once from the working directory and requires SHA-256
//! equality BEFORE the shape check, then decides what the bound bytes may
//! carry. A binding with no `--patch`, a `--patch` with no binding, a value
//! that disagrees with the argv, a shadow, an ancestor-layer file or a
//! changed byte all refuse before staging; the planner owns the arity check
//! and hands the binding here.

use std::path::{Component, Path};

use serde_json::Value;
use sha2::{Digest, Sha256};

/// A route overlay is a handful of block lines; a larger file is not the
/// shipped shape and is refused before it is parsed.
const MAX_BYTES: usize = 64 * 1024;
const MAX_LINES: usize = 4096;

/// The provider-catalogue row the shipped overlay names.
const ROUTE_ROW: &str = "llm-pi-ai";

/// The closed set of provider fields the shipped route uses.
const PROVIDER_FIELDS: [&str; 6] = [
    "displayName",
    "api",
    "baseURL",
    "compat",
    "models",
    "apiKeyEnv",
];

/// Read the engine's route-overlay binding off the private start context
/// and return its bytes after every check.
///
/// `argv_value` is the seat's single `--patch` value, or `None` when the
/// argv carries none. A `--patch` with no binding, a binding with no
/// `--patch`, or a binding whose value disagrees with the argv is refused
/// here; a seat that carries neither is an ordinary cold seat and returns
/// `None`. Every refusal is a bounded reason naming a field, never a
/// value.
///
/// `model` is the seat's pinned model; AS3 refuses a route beside no model
/// pin because there is no provider or id to compare against.
pub(super) fn claim(
    input: &Value,
    workdir: &str,
    model: Option<&str>,
    argv_value: Option<&str>,
) -> Result<Option<Vec<u8>>, String> {
    let Some(binding) = input.pointer("/resume_context/route_overlay") else {
        if argv_value.is_some() {
            return Err(refusal("a `--patch` with no bound route overlay"));
        }
        return Ok(None);
    };
    let binding = binding
        .as_object()
        .ok_or_else(|| refusal("route_overlay is not an object"))?;
    let value = binding
        .get("value")
        .and_then(Value::as_str)
        .ok_or_else(|| refusal("route_overlay carries no value"))?;
    let digest = binding
        .get("digest")
        .and_then(Value::as_str)
        .ok_or_else(|| refusal("route_overlay carries no digest"))?;
    if argv_value != Some(value) {
        return Err(refusal(
            "the route overlay binding disagrees with the `--patch` value",
        ));
    }
    let Some(model) = model else {
        return Err(refusal(
            "a route overlay needs a pinned `--model` with a provider segment",
        ));
    };
    // Independent path discipline: the engine already resolved the value
    // against the compiled layer, but the adapter never trusts a private
    // field it did not compute. A relative, traversal-free value that
    // canonicalizes inside the working directory to a regular file is the
    // only shape read.
    if value.is_empty() {
        return Err(refusal("route_overlay value is empty"));
    }
    let relative = Path::new(value);
    if relative.is_absolute() {
        return Err(refusal("route_overlay value is an absolute path"));
    }
    if relative
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(refusal("route_overlay value contains a `..` component"));
    }
    let base = std::fs::canonicalize(if workdir.is_empty() { "." } else { workdir })
        .map_err(|_| refusal("route_overlay working directory is unreadable"))?;
    let resolved = std::fs::canonicalize(base.join(relative))
        .map_err(|_| refusal("route_overlay file is unreadable"))?;
    if !resolved.starts_with(&base) {
        return Err(refusal(
            "route_overlay value resolves outside the working directory",
        ));
    }
    let meta =
        std::fs::metadata(&resolved).map_err(|_| refusal("route_overlay file is unreadable"))?;
    if !meta.is_file() {
        return Err(refusal("route_overlay value is not a regular file"));
    }
    if meta.len() as usize > MAX_BYTES {
        return Err(refusal("route_overlay exceeds the reader's byte bound"));
    }
    // Read the bytes exactly once; no later check re-reads the file.
    let bytes =
        std::fs::read(&resolved).map_err(|_| refusal("route_overlay file is unreadable"))?;
    if bytes.len() > MAX_BYTES {
        return Err(refusal("route_overlay exceeds the reader's byte bound"));
    }
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let observed = hex::encode(hasher.finalize());
    if observed != digest {
        return Err(refusal(
            "route_overlay bytes do not hash to the bound manifest digest",
        ));
    }
    validate(&bytes, model)?;
    Ok(Some(bytes))
}

/// A refusal names the field or depth, never a value.
fn refusal(reason: &str) -> String {
    format!("refusing to invoke the dsh driver: {reason}")
}

// ---------------------------------------------------------------------
// The bounded line reader.
// ---------------------------------------------------------------------

/// One recognized block line: its depth (two spaces per depth) and text.
struct Line {
    depth: usize,
    number: usize,
    text: String,
}

/// The admitted block tree. Mappings keep their pair order and duplicates,
/// so a repeated field is refused rather than silently collapsed.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Node {
    Scalar(String),
    Mapping(Vec<(String, Node)>),
    Sequence(Vec<Node>),
}

/// Validate one route overlay's bytes against the closed grammar and the
/// pinned model. `model` is `<id>` or `<provider>/<id>`; the route's single
/// provider key must equal its provider segment and its model item, when
/// named, its id segment.
pub(super) fn validate(bytes: &[u8], model: &str) -> Result<(), String> {
    let text = std::str::from_utf8(bytes).map_err(|_| refusal("route overlay is not UTF-8"))?;
    let lines = lex(text)?;
    let node = parse(&lines)?;
    let entry = match node {
        Node::Sequence(mut items) if items.len() == 1 => items.remove(0),
        Node::Sequence(_) => {
            return Err(refusal(
                "route overlay must hold exactly one top-level entry",
            ))
        }
        _ => return Err(refusal("route overlay must be a sequence of one entry")),
    };
    let entry = mapping(&entry, "the route entry")?;
    expect_only(entry, &["id", "config"], "the route entry")?;
    let id = scalar_field(entry, "id", "the route entry")?;
    if id != ROUTE_ROW {
        return Err(refusal("the route entry id is not the route row"));
    }
    let config = mapping(field(entry, "config", "the route entry")?, "config")?;
    expect_only(config, &["providers"], "config")?;
    let providers = mapping(field(config, "providers", "config")?, "providers")?;
    if providers.len() != 1 {
        return Err(refusal("providers must define exactly one provider"));
    }
    let pinned = super::parse_dsh_model(model)
        .map_err(|_| refusal("the pinned model is not `<id>` or `<provider>/<id>`"))?;
    let (provider_key, provider_node) = &providers[0];
    if provider_key != pinned.provider {
        return Err(refusal("the route names a provider the seat did not pin"));
    }
    let provider = mapping(provider_node, "the provider mapping")?;
    expect_only(provider, &PROVIDER_FIELDS, "the provider mapping")?;
    for (key, _) in provider {
        if count(provider, key) > 1 {
            return Err(refusal("the provider mapping repeats a field"));
        }
    }
    let api_key_env = scalar_field(provider, "apiKeyEnv", "the provider mapping")?;
    if !environment_name(api_key_env) {
        return Err(refusal("apiKeyEnv is not an environment-variable name"));
    }
    if let Some(base_url) = optional_scalar(provider, "baseURL", "the provider mapping")? {
        if !endpoint(base_url) {
            return Err(refusal("baseURL leaves the closed endpoint grammar"));
        }
    }
    // `compat` is a mapping the reader already constrained to `<key>: <value>`
    // pairs; it names no credential position.
    let _ = optional_mapping(provider, "compat", "the provider mapping")?;
    let models = field(provider, "models", "the provider mapping")?;
    let item = match models {
        Node::Sequence(items) if items.len() == 1 => &items[0],
        Node::Sequence(_) => return Err(refusal("models must hold exactly one item")),
        _ => return Err(refusal("models is not a sequence")),
    };
    let item = mapping(item, "the model item")?;
    expect_only(item, &["id", "reasoningEfforts"], "the model item")?;
    let model_id = scalar_field(item, "id", "the model item")?;
    if model_id != pinned.model {
        return Err(refusal("the route names a model the seat did not pin"));
    }
    let efforts = mapping(
        field(item, "reasoningEfforts", "the model item")?,
        "reasoningEfforts",
    )?;
    if efforts.is_empty() {
        return Err(refusal("reasoningEfforts must name at least one level"));
    }
    Ok(())
}

fn lex(text: &str) -> Result<Vec<Line>, String> {
    let bytes = text.as_bytes();
    if bytes.len() > MAX_BYTES {
        return Err(refusal("route overlay exceeds the reader's byte bound"));
    }
    let mut lines = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let number = index + 1;
        if raw.is_empty() || raw.trim_start_matches(' ').starts_with('#') {
            continue;
        }
        if raw == "---" || raw == "..." {
            return Err(refusal(&format!(
                "route overlay line {number} is a document marker"
            )));
        }
        if raw.contains('\t') || raw.chars().any(char::is_control) {
            return Err(refusal(&format!(
                "route overlay line {number} carries a tab or control character"
            )));
        }
        let indent = raw.len() - raw.trim_start_matches(' ').len();
        if indent % 2 != 0 {
            return Err(refusal(&format!(
                "route overlay line {number} has an odd indentation"
            )));
        }
        let body = &raw[indent..];
        if body.is_empty() {
            continue;
        }
        lines.push(Line {
            depth: indent / 2,
            number,
            text: body.to_string(),
        });
        if lines.len() > MAX_LINES {
            return Err(refusal("route overlay has too many lines"));
        }
    }
    if lines.is_empty() {
        return Err(refusal("route overlay is empty"));
    }
    Ok(lines)
}

fn parse(lines: &[Line]) -> Result<Node, String> {
    let (node, end) = parse_block(lines, 0, lines[0].depth)?;
    if end != lines.len() {
        return Err(refusal(&format!(
            "route overlay line {} has an unexpected indentation",
            lines[end].number
        )));
    }
    Ok(node)
}

fn parse_block(lines: &[Line], start: usize, depth: usize) -> Result<(Node, usize), String> {
    if start >= lines.len() || lines[start].depth != depth {
        return Err(refusal("route overlay indentation is inconsistent"));
    }
    if is_item(&lines[start].text) {
        parse_sequence(lines, start, depth)
    } else {
        parse_mapping(lines, start, depth)
    }
}

fn parse_mapping(lines: &[Line], mut index: usize, depth: usize) -> Result<(Node, usize), String> {
    let mut pairs = Vec::new();
    while index < lines.len() && lines[index].depth == depth && !is_item(&lines[index].text) {
        let (key, value, next) = parse_member(&lines[index], lines, index + 1, depth)?;
        pairs.push((key, value));
        index = next;
    }
    if pairs.is_empty() {
        return Err(refusal("route overlay block has no lines"));
    }
    Ok((Node::Mapping(pairs), index))
}

fn parse_sequence(lines: &[Line], mut index: usize, depth: usize) -> Result<(Node, usize), String> {
    let mut items = Vec::new();
    while index < lines.len() && lines[index].depth == depth && is_item(&lines[index].text) {
        let rest = lines[index].text[1..].trim_start().to_string();
        if rest.is_empty() {
            return Err(refusal(&format!(
                "route overlay line {} is a bare sequence item",
                lines[index].number
            )));
        }
        let (key, value, mut next) =
            parse_member_text(&rest, lines[index].number, lines, index + 1, depth)?;
        let mut pairs = vec![(key, value)];
        while next < lines.len() && lines[next].depth == depth + 1 && !is_item(&lines[next].text) {
            let (key, value, after) = parse_member(&lines[next], lines, next + 1, depth + 1)?;
            pairs.push((key, value));
            next = after;
        }
        items.push(Node::Mapping(pairs));
        index = next;
    }
    if items.is_empty() {
        return Err(refusal("route overlay sequence is empty"));
    }
    Ok((Node::Sequence(items), index))
}

fn parse_member(
    line: &Line,
    lines: &[Line],
    next: usize,
    depth: usize,
) -> Result<(String, Node, usize), String> {
    parse_member_text(&line.text, line.number, lines, next, depth)
}

fn parse_member_text(
    text: &str,
    number: usize,
    lines: &[Line],
    next: usize,
    depth: usize,
) -> Result<(String, Node, usize), String> {
    if let Some(colon) = text.find(": ") {
        let key = &text[..colon];
        let value = &text[colon + 2..];
        check_key(key, number)?;
        check_scalar(value, number)?;
        return Ok((key.to_string(), Node::Scalar(value.to_string()), next));
    }
    if let Some(key) = text.strip_suffix(':') {
        check_key(key, number)?;
        if next < lines.len() && lines[next].depth > depth {
            if lines[next].depth != depth + 1 {
                return Err(refusal(&format!(
                    "route overlay line {} jumps more than one depth",
                    lines[next].number
                )));
            }
            let (child, after) = parse_block(lines, next, lines[next].depth)?;
            return Ok((key.to_string(), child, after));
        }
        return Err(refusal(&format!(
            "route overlay line {number} has an empty `{key}:` block"
        )));
    }
    Err(refusal(&format!(
        "route overlay line {number} is not a recognized block line"
    )))
}

fn is_item(text: &str) -> bool {
    text == "-" || text.starts_with("- ")
}

fn check_key(key: &str, number: usize) -> Result<(), String> {
    let mut characters = key.chars();
    match characters.next() {
        Some(first) if first.is_ascii_alphabetic() => {}
        _ => {
            return Err(refusal(&format!(
                "route overlay line {number} carries a key that is not a plain identifier"
            )))
        }
    }
    if !characters.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err(refusal(&format!(
            "route overlay line {number} carries a key that is not a plain identifier"
        )));
    }
    Ok(())
}

fn check_scalar(value: &str, number: usize) -> Result<(), String> {
    if value.is_empty() {
        return Err(refusal(&format!(
            "route overlay line {number} carries an empty value"
        )));
    }
    let first = value.chars().next().expect("non-empty");
    if "!&*{[|>%@`\"'?-:,".contains(first) {
        return Err(refusal(&format!(
            "route overlay line {number} carries a value beginning with a reserved character"
        )));
    }
    if value.contains(" #") || value.contains(": ") {
        return Err(refusal(&format!(
            "route overlay line {number} carries a value with a comment or an extra mapping"
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------------
// Semantic helpers over the parsed tree.
// ---------------------------------------------------------------------

fn mapping<'a>(node: &'a Node, what: &str) -> Result<&'a Vec<(String, Node)>, String> {
    match node {
        Node::Mapping(pairs) => Ok(pairs),
        _ => Err(refusal(&format!("{what} is not a mapping"))),
    }
}

fn field<'a>(pairs: &'a [(String, Node)], key: &str, what: &str) -> Result<&'a Node, String> {
    let mut found = pairs.iter().filter(|(name, _)| name == key);
    let Some((_, value)) = found.next() else {
        return Err(refusal(&format!("{what} is missing `{key}`")));
    };
    if found.next().is_some() {
        return Err(refusal(&format!("{what} repeats `{key}`")));
    }
    Ok(value)
}

fn scalar_field<'a>(pairs: &'a [(String, Node)], key: &str, what: &str) -> Result<&'a str, String> {
    match field(pairs, key, what)? {
        Node::Scalar(value) => Ok(value),
        _ => Err(refusal(&format!("{what} nests a non-scalar `{key}`"))),
    }
}

fn optional_scalar<'a>(
    pairs: &'a [(String, Node)],
    key: &str,
    what: &str,
) -> Result<Option<&'a str>, String> {
    match pairs.iter().find(|(name, _)| name == key) {
        None => Ok(None),
        Some((_, Node::Scalar(value))) => Ok(Some(value)),
        Some(_) => Err(refusal(&format!("{what} nests a non-scalar `{key}`"))),
    }
}

fn optional_mapping<'a>(
    pairs: &'a [(String, Node)],
    key: &str,
    what: &str,
) -> Result<Option<&'a Vec<(String, Node)>>, String> {
    match pairs.iter().find(|(name, _)| name == key) {
        None => Ok(None),
        Some((_, Node::Mapping(inner))) => Ok(Some(inner)),
        Some(_) => Err(refusal(&format!("{what} nests a non-mapping `{key}`"))),
    }
}

fn expect_only(pairs: &[(String, Node)], allowed: &[&str], what: &str) -> Result<(), String> {
    for (key, _) in pairs {
        if !allowed.contains(&key.as_str()) {
            return Err(refusal(&format!(
                "{what} carries a field outside the closed set"
            )));
        }
    }
    Ok(())
}

fn count(pairs: &[(String, Node)], key: &str) -> usize {
    pairs.iter().filter(|(name, _)| name == key).count()
}

/// An environment-variable name: an ASCII letter or `_` followed by ASCII
/// letters, digits or `_`. Brokkr never resolves, forwards or echoes it.
fn environment_name(value: &str) -> bool {
    let mut characters = value.chars();
    match characters.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
        _ => return false,
    }
    characters.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// `https://<host>[:<port>][/<segment>...]`, exactly: lowercase scheme,
/// dotted host labels that do not begin or end with `-`, an optional
/// one-to-five-digit port, and segments of letters, digits, `-`, `.`, `_`
/// or `~`. Nothing else — no userinfo, query, fragment, percent escape,
/// backslash, whitespace, brackets, empty segment, other scheme or
/// schemeless value.
fn endpoint(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("https://") else {
        return false;
    };
    if rest.is_empty() || rest.contains('@') || rest.contains('?') || rest.contains('#') {
        return false;
    }
    let (authority, path) = match rest.split_once('/') {
        Some((authority, path)) => (authority, Some(path)),
        None => (rest, None),
    };
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    };
    if port.is_some_and(|port| {
        port.is_empty() || port.len() > 5 || !port.chars().all(|c| c.is_ascii_digit())
    }) {
        return false;
    }
    if host.is_empty() || !host.split('.').all(host_label) {
        return false;
    }
    match path {
        None => true,
        Some(path) => {
            !path.is_empty()
                && path
                    .split('/')
                    .all(|segment| !segment.is_empty() && segment.chars().all(segment_char))
        }
    }
}

fn host_label(label: &str) -> bool {
    if label.is_empty() {
        return false;
    }
    if label.starts_with('-') || label.ends_with('-') {
        return false;
    }
    label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn segment_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '~')
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The shipped research lane's route, copied from
    /// `recipes/research-dsh/drivers/research-web.yml` without its comments.
    const SHIPPED: &str = "\
- id: llm-pi-ai
  config:
    providers:
      dashscope:
        displayName: Model Studio (Token Plan)
        apiKeyEnv: DASHSCOPE_API_KEY
        api: openai-completions
        baseURL: https://token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1
        compat:
          thinkingFormat: deepseek
        models:
          - id: qwen3.8-max
            reasoningEfforts:
              low: low
              medium: medium
              xhigh: xhigh
";

    const PIN: &str = "dashscope/qwen3.8-max";

    fn refused(body: &str, needle: &str) {
        let error = validate(body.as_bytes(), PIN).unwrap_err();
        assert!(error.contains(needle), "expected {needle:?} in {error:?}");
    }

    #[test]
    fn the_shipped_route_and_its_comments_pass_the_closed_grammar() {
        // Comments and blank lines are the shipped file's own shape.
        let with_comments = format!("# Decision 0044 ruling 5.\n\n{}# trailing note\n", SHIPPED);
        assert!(validate(with_comments.as_bytes(), PIN).is_ok());
    }

    #[test]
    fn the_route_row_and_provider_are_closed() {
        refused(
            "- id: session-persistence-jsonl\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: X\n        models:\n          - id: qwen3.8-max\n",
            "route entry id",
        );
        refused(
            "- id: llm-pi-ai\n  config:\n    providers:\n      openrouter:\n        apiKeyEnv: X\n        models:\n          - id: qwen3.8-max\n",
            "provider the seat did not pin",
        );
        refused(
            "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: X\n        models:\n          - id: other\n",
            "model the seat did not pin",
        );
        // Two providers.
        refused(
            "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: X\n        models:\n          - id: qwen3.8-max\n      other:\n        apiKeyEnv: Y\n        models:\n          - id: qwen3.8-max\n",
            "exactly one provider",
        );
    }

    #[test]
    fn a_credential_value_or_a_field_outside_the_set_is_refused() {
        // An inline apiKey is outside the closed set.
        refused(
            "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKey: sk-live\n        apiKeyEnv: DASHSCOPE_API_KEY\n        models:\n          - id: qwen3.8-max\n",
            "outside the closed set",
        );
        // A literal authorization header, even beside a valid apiKeyEnv.
        refused(
            "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: DASHSCOPE_API_KEY\n        headers:\n          Authorization: Bearer sk-live\n        models:\n          - id: qwen3.8-max\n",
            "outside the closed set",
        );
        // A missing or malformed apiKeyEnv.
        refused(
            "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        api: openai-completions\n        models:\n          - id: qwen3.8-max\n",
            "missing `apiKeyEnv`",
        );
        refused(
            "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: 9LIVE\n        models:\n          - id: qwen3.8-max\n",
            "environment-variable name",
        );
        // modelOverrides is outside the set too.
        refused(
            "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: DASHSCOPE_API_KEY\n        modelOverrides:\n          x: y\n        models:\n          - id: qwen3.8-max\n",
            "outside the closed set",
        );
    }

    #[test]
    fn the_endpoint_grammar_decides_the_positive_and_every_refusal() {
        for (base, part) in [
            ("https://user:pass@host/x", "endpoint"),
            ("https://host/x?api_key=1", "endpoint"),
            ("https://host/x#frag", "endpoint"),
            ("https://host/percent%2fescape", "endpoint"),
            ("https://host\\x", "endpoint"),
            ("https://host/ space", "endpoint"),
            ("https://[::1]/x", "endpoint"),
            ("https://host//x", "endpoint"),
            ("http://host/x", "endpoint"),
            ("HTTPS://host/x", "endpoint"),
            ("host/x", "endpoint"),
            ("https://-host/x", "endpoint"),
            ("https://host:123456/x", "endpoint"),
        ] {
            let body = SHIPPED.replace(
                "https://token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1",
                base,
            );
            let error = validate(body.as_bytes(), PIN).unwrap_err();
            assert!(
                error.contains(part),
                "{base:?}: expected {part:?} in {error:?}"
            );
        }
    }

    #[test]
    fn executable_or_unrecognized_syntax_is_refused_at_any_depth() {
        for (body, needle) in [
            ("- id: llm-pi-ai\n  config: !!js ctx\n", "reserved character"),
            ("- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: X\n        __jsExpr: 1\n        models:\n          - id: qwen3.8-max\n", "plain identifier"),
            ("- id: llm-pi-ai\n  config: {providers: x}\n", "reserved character"),
            ("- id: llm-pi-ai\n  config: &anchor x\n", "reserved character"),
            ("- id: llm-pi-ai\n  config: *alias\n", "reserved character"),
            ("- id: llm-pi-ai\n  config: |\n    x\n", "reserved character"),
            ("- id: 'llm-pi-ai'\n", "reserved character"),
            ("- id: llm-pi-ai\n  config:\n    <<: x\n", "plain identifier"),
        ] {
            let error = validate(body.as_bytes(), PIN).unwrap_err();
            assert!(error.contains(needle), "{body:?}: expected {needle:?} in {error:?}");
        }
    }

    #[test]
    fn a_route_needs_a_model_pin_and_model_item() {
        assert!(validate(SHIPPED.as_bytes(), "deepseek-v4-flash").is_err());
        refused(
            "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: X\n",
            "missing `models`",
        );
    }

    /// A helper: write one file under a temporary working directory and
    /// build the private context that names it.
    fn binding(body: &[u8], value: &str) -> (tempfile::TempDir, Value) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(value);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, body).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(body);
        let digest = hex::encode(hasher.finalize());
        let input = json!({
            "resume_context": { "route_overlay": { "value": value, "digest": digest } }
        });
        (dir, input)
    }

    #[test]
    fn claim_reads_the_bound_file_and_requires_the_digest_before_the_shape() {
        let (dir, input) = binding(SHIPPED.as_bytes(), "route.yml");
        let workdir = dir.path().to_string_lossy().into_owned();
        let bytes = claim(&input, &workdir, Some(PIN), Some("route.yml"))
            .unwrap()
            .expect("a bound route is admitted");
        assert_eq!(bytes, SHIPPED.as_bytes());

        // The binding disagrees with the argv.
        let error = claim(&input, &workdir, Some(PIN), Some("other.yml")).unwrap_err();
        assert!(error.contains("disagrees"), "{error}");

        // A `--patch` with no binding and a binding with no `--patch`.
        let error = claim(&json!({}), &workdir, Some(PIN), Some("route.yml")).unwrap_err();
        assert!(error.contains("no bound route overlay"), "{error}");
        let error = claim(&input, &workdir, Some(PIN), None).unwrap_err();
        assert!(error.contains("disagrees"), "{error}");

        // A digest mismatch refuses before any shape check, even though the
        // bytes are a valid route.
        let (_, wrong) = binding(b"route: changed\n", "route.yml");
        let error = claim(&wrong, &workdir, Some(PIN), Some("route.yml")).unwrap_err();
        assert!(error.contains("do not hash"), "{error}");

        // A bound, digest-matching member whose baseURL leaves the grammar
        // still refuses: binding decides bytes, the grammar decides content.
        let bad = SHIPPED.replace("https://token-plan", "http://token-plan");
        let (bad_dir, bad_input) = binding(bad.as_bytes(), "route.yml");
        let error = claim(
            &bad_input,
            &bad_dir.path().to_string_lossy(),
            Some(PIN),
            Some("route.yml"),
        )
        .unwrap_err();
        assert!(error.contains("endpoint"), "{error}");
    }

    #[test]
    fn claim_refuses_absolute_traversal_and_escaping_values() {
        let (dir, _) = binding(SHIPPED.as_bytes(), "route.yml");
        let workdir = dir.path().to_string_lossy().into_owned();
        for value in ["/etc/passwd", "../escape.yml"] {
            let input = json!({
                "resume_context": { "route_overlay": { "value": value, "digest": "a".repeat(64) } }
            });
            let error = claim(&input, &workdir, Some(PIN), Some(value)).unwrap_err();
            assert!(
                error.contains("absolute") || error.contains("`..`"),
                "{value:?}: {error}"
            );
        }
    }
}
