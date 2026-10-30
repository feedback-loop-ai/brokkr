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

mod refusal;

use super::dsh_hands::{Mode, DISABLED_ROWS, SANDBOX_POLICY};
pub(super) use refusal::Refused;
use refusal::{Field, Overlay, Part};

/// A route overlay is a handful of block lines; a larger file is not the
/// shipped shape and is refused before it is parsed.
const MAX_BYTES: usize = 64 * 1024;
const MAX_LINES: usize = 4096;

/// The provider-catalogue row the shipped overlay names.
const ROUTE_ROW: &str = "llm-pi-ai";

/// The closed set of provider fields the shipped route uses.
const PROVIDER_FIELDS: [Field; 6] = [
    Field::DisplayName,
    Field::Api,
    Field::BaseUrl,
    Field::Compat,
    Field::Models,
    Field::ApiKeyEnv,
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
/// pin, or beside a pin with no provider segment, because there is no
/// pinned provider to compare against.
pub(super) fn claim(
    input: &Value,
    workdir: &str,
    model: Option<&str>,
    argv_value: Option<&str>,
) -> Result<Option<Vec<u8>>, Refused> {
    claim_with(input, workdir, model, argv_value, read_route_file, |path| {
        std::fs::metadata(path)
    })
}

/// The one file reader production uses; named so a fault test can inject a
/// failing metadata read without spelling a reader that never runs.
fn read_route_file(path: &Path) -> std::io::Result<Vec<u8>> {
    std::fs::read(path)
}

/// `claim` over injected readers. The post-read byte bound guards the file
/// against growing between its metadata check and its one read — a TOCTOU
/// window no deterministic fixture can force — so the reader is injectable,
/// the way this module's other syscalls already are. The metadata read is
/// injectable for the same reason: a successful canonicalization is a
/// precondition, and no ordinary path makes the following `stat` fail.
fn claim_with(
    input: &Value,
    workdir: &str,
    model: Option<&str>,
    argv_value: Option<&str>,
    read: impl FnOnce(&Path) -> std::io::Result<Vec<u8>>,
    stat: impl FnOnce(&Path) -> std::io::Result<std::fs::Metadata>,
) -> Result<Option<Vec<u8>>, Refused> {
    let Some(binding) = input.pointer("/resume_context/route_overlay") else {
        if argv_value.is_some() {
            return Err(Overlay::PatchUnbound.into());
        }
        return Ok(None);
    };
    let binding = binding.as_object().ok_or(Overlay::BindingNotAnObject)?;
    let value = binding
        .get("value")
        .and_then(Value::as_str)
        .ok_or(Overlay::NoValue)?;
    let digest = binding
        .get("digest")
        .and_then(Value::as_str)
        .ok_or(Overlay::NoDigest)?;
    if argv_value != Some(value) {
        return Err(Overlay::Disagrees.into());
    }
    // A pin with no `/` is refused with the absent pin: `parse_dsh_model`
    // would supply its default provider, and a route keyed to that default
    // would then pass for a seat that named no provider at all.
    let Some(model) = model.filter(|model| model.contains('/')) else {
        return Err(Overlay::NoProviderPin.into());
    };
    // Independent path discipline: the engine already resolved the value
    // against the compiled layer, but the adapter never trusts a private
    // field it did not compute. A relative, traversal-free value that
    // canonicalizes inside the working directory to a regular file is the
    // only shape read.
    if value.is_empty() {
        return Err(Overlay::EmptyValue.into());
    }
    let relative = Path::new(value);
    if relative.is_absolute() {
        return Err(Overlay::Absolute.into());
    }
    if relative
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(Overlay::ParentComponent.into());
    }
    let base = std::fs::canonicalize(if workdir.is_empty() { "." } else { workdir })
        .map_err(|_| Overlay::WorkdirUnreadable)?;
    let resolved =
        std::fs::canonicalize(base.join(relative)).map_err(|_| Overlay::FileUnreadable)?;
    if !resolved.starts_with(&base) {
        return Err(Overlay::Outside.into());
    }
    let meta = stat(&resolved).map_err(|_| Overlay::FileUnreadable)?;
    if !meta.is_file() {
        return Err(Overlay::NotRegular.into());
    }
    if meta.len() as usize > MAX_BYTES {
        return Err(Overlay::FileOverBound.into());
    }
    // Read the bytes exactly once; no later check re-reads the file.
    let bytes = read(&resolved).map_err(|_| Overlay::FileUnreadable)?;
    if bytes.len() > MAX_BYTES {
        return Err(Overlay::FileOverBound.into());
    }
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let observed = hex::encode(hasher.finalize());
    if observed != digest {
        return Err(Overlay::DigestMismatch.into());
    }
    validate(&bytes, model)?;
    Ok(Some(bytes))
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
pub(super) fn validate(bytes: &[u8], model: &str) -> Result<(), Refused> {
    let text = std::str::from_utf8(bytes).map_err(|_| Overlay::NotUtf8)?;
    let lines = lex(text)?;
    let node = parse(&lines)?;
    let entry = match node {
        Node::Sequence(mut items) if items.len() == 1 => items.remove(0),
        Node::Sequence(_) => return Err(Overlay::EntryCount.into()),
        _ => return Err(Overlay::NotASequence.into()),
    };
    let route = Part::RouteEntry;
    let entry = mapping(&entry, route)?;
    expect_only(entry, &[Field::Id, Field::Config], route)?;
    let id = scalar_field(entry, Field::Id, route)?;
    if id != ROUTE_ROW {
        return Err(Overlay::NotTheRouteRow.into());
    }
    let config = Part::Named(Field::Config);
    let config_pairs = mapping(field(entry, Field::Config, route)?, config)?;
    expect_only(config_pairs, &[Field::Providers], config)?;
    let providers = mapping(
        field(config_pairs, Field::Providers, config)?,
        Part::Named(Field::Providers),
    )?;
    if providers.len() != 1 {
        return Err(Overlay::ProviderCount.into());
    }
    let pinned = super::parse_dsh_model(model).map_err(|_| Overlay::PinGrammar)?;
    let (provider_key, provider_node) = &providers[0];
    if provider_key != pinned.provider {
        return Err(Overlay::UnpinnedProvider.into());
    }
    let mapped = Part::ProviderMapping;
    let provider = mapping(provider_node, mapped)?;
    expect_only(provider, &PROVIDER_FIELDS, mapped)?;
    for (key, _) in provider {
        if count(provider, key) > 1 {
            return Err(Overlay::ProviderFieldRepeated.into());
        }
    }
    let api_key_env = scalar_field(provider, Field::ApiKeyEnv, mapped)?;
    if !environment_name(api_key_env) {
        return Err(Overlay::ApiKeyEnv.into());
    }
    if let Some(base_url) = optional_scalar(provider, Field::BaseUrl, mapped)? {
        if !endpoint(base_url) {
            return Err(Overlay::Endpoint.into());
        }
    }
    // `compat` is a mapping the reader already constrained to `<key>: <value>`
    // pairs; it names no credential position.
    let _ = optional_mapping(provider, Field::Compat, mapped)?;
    let models = field(provider, Field::Models, mapped)?;
    let item = match models {
        Node::Sequence(items) if items.len() == 1 => &items[0],
        Node::Sequence(_) => return Err(Overlay::ModelCount.into()),
        _ => return Err(Overlay::ModelsNotASequence.into()),
    };
    let model_item = Part::ModelItem;
    let item = mapping(item, model_item)?;
    expect_only(item, &[Field::Id, Field::ReasoningEfforts], model_item)?;
    let model_id = scalar_field(item, Field::Id, model_item)?;
    if model_id != pinned.model {
        return Err(Overlay::UnpinnedModel.into());
    }
    // Read for its shape alone. A mapping cannot be empty here: a block
    // with no lines and a bare `key:` are both refused while parsing, so
    // "names at least one level" is already true of anything that
    // reaches this point. The guard that used to restate it could not
    // fire, and a test proves the parser's refusal instead.
    let _ = mapping(
        field(item, Field::ReasoningEfforts, model_item)?,
        Part::Named(Field::ReasoningEfforts),
    )?;
    Ok(())
}

/// Validate a dsh seat's hands rows (#896) against the one shape they may
/// take, read by the same bounded reader: each effectful tool row of the
/// measured release `disabled: true`, once, and the `sandbox-policy` row's
/// `mode`, `read-only` or `workspace-write`, once, and nothing else. A row
/// in any other shape, a row named twice and a measured row left out are
/// refused, each naming the row: these rows are the engine's own, never a
/// recipe's, so naming them hands a seat nothing it did not write.
pub(super) fn hands(bytes: &[u8]) -> Result<(), Refused> {
    let text = std::str::from_utf8(bytes).map_err(|_| Overlay::NotUtf8)?;
    let Node::Sequence(items) = parse(&lex(text)?)? else {
        return Err(Overlay::NotASequence.into());
    };
    let mut seen: Vec<&str> = Vec::with_capacity(items.len());
    for item in &items {
        let row = hands_row(mapping(item, Part::HandsRow)?)?;
        if seen.contains(&row) {
            return Err(Overlay::HandsRowRepeated(row.to_string()).into());
        }
        seen.push(row);
    }
    let mut owed = DISABLED_ROWS.into_iter().chain([SANDBOX_POLICY]);
    match owed.find(|row| !seen.contains(row)) {
        Some(missing) => Err(Overlay::HandsRowMissing(missing).into()),
        None => Ok(()),
    }
}

/// One hands row's id, where the row is in the one shape its id may take.
fn hands_row(pairs: &[(String, Node)]) -> Result<&str, Overlay> {
    let id = scalar_field(pairs, Field::Id, Part::HandsRow)?;
    let rest: Vec<(&str, &Node)> = (pairs.iter())
        .filter(|(key, _)| key != Field::Id.spelling())
        .map(|(key, value)| (key.as_str(), value))
        .collect();
    let shaped = match rest[..] {
        [("disabled", Node::Scalar(flag))] => DISABLED_ROWS.contains(&id) && flag == "true",
        [("config", Node::Mapping(config))] => {
            id == SANDBOX_POLICY
                && matches!(&config[..], [(key, Node::Scalar(mode))]
                    if key == "mode" && Mode::named(mode).is_some())
        }
        _ => false,
    };
    match shaped {
        true => Ok(id),
        false => Err(Overlay::HandsRow(id.to_string())),
    }
}

fn lex(text: &str) -> Result<Vec<Line>, Overlay> {
    let bytes = text.as_bytes();
    if bytes.len() > MAX_BYTES {
        return Err(Overlay::TextOverBound);
    }
    let mut lines = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let number = index + 1;
        // The character check comes BEFORE the comment skip. `lines`
        // splits on LF alone, so a lone CR inside a `# comment` line
        // would otherwise hide a second row — `apiKey:`, `__jsExpr:` —
        // that a CR-tolerant YAML reader downstream would parse as its
        // own line, past this closed grammar (AS3).
        if raw.contains('\t') || raw.chars().any(char::is_control) {
            return Err(Overlay::Control(number));
        }
        if raw.is_empty() || raw.trim_start_matches(' ').starts_with('#') {
            continue;
        }
        if raw == "---" || raw == "..." {
            return Err(Overlay::DocumentMarker(number));
        }
        let indent = raw.len() - raw.trim_start_matches(' ').len();
        if indent % 2 != 0 {
            return Err(Overlay::OddIndentation(number));
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
            return Err(Overlay::TooManyLines);
        }
    }
    if lines.is_empty() {
        return Err(Overlay::Empty);
    }
    Ok(lines)
}

fn parse(lines: &[Line]) -> Result<Node, Overlay> {
    let (node, end) = parse_block(lines, 0, lines[0].depth)?;
    if end != lines.len() {
        return Err(Overlay::UnexpectedIndentation(lines[end].number));
    }
    Ok(node)
}

fn parse_block(lines: &[Line], start: usize, depth: usize) -> Result<(Node, usize), Overlay> {
    if start >= lines.len() || lines[start].depth != depth {
        return Err(Overlay::Inconsistent);
    }
    if is_item(&lines[start].text) {
        parse_sequence(lines, start, depth)
    } else {
        parse_mapping(lines, start, depth)
    }
}

fn parse_mapping(lines: &[Line], mut index: usize, depth: usize) -> Result<(Node, usize), Overlay> {
    let mut pairs = Vec::new();
    while index < lines.len() && lines[index].depth == depth && !is_item(&lines[index].text) {
        let (key, value, next) = parse_member(&lines[index], lines, index + 1, depth)?;
        pairs.push((key, value));
        index = next;
    }
    if pairs.is_empty() {
        return Err(Overlay::EmptyBlock);
    }
    Ok((Node::Mapping(pairs), index))
}

fn parse_sequence(
    lines: &[Line],
    mut index: usize,
    depth: usize,
) -> Result<(Node, usize), Overlay> {
    let mut items = Vec::new();
    while index < lines.len() && lines[index].depth == depth && is_item(&lines[index].text) {
        let rest = lines[index].text[1..].trim_start().to_string();
        if rest.is_empty() {
            return Err(Overlay::BareItem(lines[index].number));
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
        return Err(Overlay::EmptySequence);
    }
    Ok((Node::Sequence(items), index))
}

fn parse_member(
    line: &Line,
    lines: &[Line],
    next: usize,
    depth: usize,
) -> Result<(String, Node, usize), Overlay> {
    parse_member_text(&line.text, line.number, lines, next, depth)
}

fn parse_member_text(
    text: &str,
    number: usize,
    lines: &[Line],
    next: usize,
    depth: usize,
) -> Result<(String, Node, usize), Overlay> {
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
                return Err(Overlay::DepthJump(lines[next].number));
            }
            let (child, after) = parse_block(lines, next, lines[next].depth)?;
            return Ok((key.to_string(), child, after));
        }
        return Err(Overlay::EmptyKeyBlock {
            line: number,
            key: key.to_string(),
        });
    }
    Err(Overlay::Unrecognized(number))
}

fn is_item(text: &str) -> bool {
    text == "-" || text.starts_with("- ")
}

fn check_key(key: &str, number: usize) -> Result<(), Overlay> {
    let mut characters = key.chars();
    match characters.next() {
        Some(first) if first.is_ascii_alphabetic() => {}
        _ => return Err(Overlay::KeyNotIdentifier(number)),
    }
    if !characters.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err(Overlay::KeyNotIdentifier(number));
    }
    Ok(())
}

fn check_scalar(value: &str, number: usize) -> Result<(), Overlay> {
    if value.is_empty() {
        return Err(Overlay::EmptyScalar(number));
    }
    let first = value.chars().next().expect("non-empty");
    if "!&*{[|>%@`\"'?-:,".contains(first) {
        return Err(Overlay::ReservedStart(number));
    }
    if value.contains(" #") || value.contains(": ") {
        return Err(Overlay::CommentOrMapping(number));
    }
    Ok(())
}

// ---------------------------------------------------------------------
// Semantic helpers over the parsed tree.
// ---------------------------------------------------------------------

fn mapping(node: &Node, part: Part) -> Result<&Vec<(String, Node)>, Overlay> {
    match node {
        Node::Mapping(pairs) => Ok(pairs),
        _ => Err(Overlay::NotAMapping(part)),
    }
}

fn field(pairs: &[(String, Node)], key: Field, part: Part) -> Result<&Node, Overlay> {
    let mut found = pairs.iter().filter(|(name, _)| name == key.spelling());
    let Some((_, value)) = found.next() else {
        return Err(Overlay::Missing { part, field: key });
    };
    if found.next().is_some() {
        return Err(Overlay::Repeats { part, field: key });
    }
    Ok(value)
}

fn scalar_field(pairs: &[(String, Node)], key: Field, part: Part) -> Result<&str, Overlay> {
    match field(pairs, key, part)? {
        Node::Scalar(value) => Ok(value),
        _ => Err(Overlay::NonScalar { part, field: key }),
    }
}

fn optional_scalar(
    pairs: &[(String, Node)],
    key: Field,
    part: Part,
) -> Result<Option<&str>, Overlay> {
    match pairs.iter().find(|(name, _)| name == key.spelling()) {
        None => Ok(None),
        Some((_, Node::Scalar(value))) => Ok(Some(value)),
        Some(_) => Err(Overlay::NonScalar { part, field: key }),
    }
}

fn optional_mapping(
    pairs: &[(String, Node)],
    key: Field,
    part: Part,
) -> Result<Option<&Vec<(String, Node)>>, Overlay> {
    match pairs.iter().find(|(name, _)| name == key.spelling()) {
        None => Ok(None),
        Some((_, Node::Mapping(inner))) => Ok(Some(inner)),
        Some(_) => Err(Overlay::NonMapping { part, field: key }),
    }
}

fn expect_only(pairs: &[(String, Node)], allowed: &[Field], part: Part) -> Result<(), Overlay> {
    for (key, _) in pairs {
        if !allowed.iter().any(|field| field.spelling() == key) {
            return Err(Overlay::OutsideClosedSet(part));
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
mod tests;
