//! Decision 0082 ruling 4: one lifecycle table in `docs/guides/versioning.md`
//! covers every file under `contracts/` and the one file-less label ruling 5
//! records. The table is written by hand; this module reads it and checks it
//! against the tree and against the loaders the code uses. Ruling 1's minting
//! rule lives in `decisions_index.rs`, beside the ledger it reads.
//!
//! A row whose reader is private to its crate is not driven here: its row is
//! named in [`PRIVATE`] and the owning crate's tests read the same table and
//! drive the loader. No `pub` item is added for that check (decision 0071
//! ruling 6).

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// The guide that holds the table, from the workspace root.
pub(crate) const GUIDE: &str = "docs/guides/versioning.md";

/// The decision number whose ruling 5 records the one file-less label.
pub(crate) const FILELESS: &str = "none (decision 0082 ruling 5)";

/// A reader private to its crate, with the row its file names. The owning
/// crate's tests drive it; this module only admits the name.
pub(crate) const PRIVATE: [(&str, &str); 5] = [
    (
        "contracts/effect-provenance.v1.schema.json",
        "load brokkr_core::seat::row::Seats::note",
    ),
    (
        "contracts/phase-entered-head.v1.schema.json",
        "load brokkr_core::payload::PhaseEntered::read",
    ),
    (
        "contracts/host.v1.schema.json",
        "load brokkr_runtime::admission::host::read",
    ),
    (
        "contracts/tool-dialect.v1.schema.json",
        "schema brokkr_runtime::capabilities::dialect::TOOL_DIALECT_SCHEMA",
    ),
    (
        "contracts/panel-carry.v1.schema.json",
        "load brokkr_runtime::engine::carry::facts::terminal_record",
    ),
];

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

/// The table's nine columns, in order.
const COLUMNS: [&str; 9] = [
    "Version",
    "File",
    "Class",
    "Minted under",
    "Writer",
    "Reader",
    "State",
    "Last written",
    "Reader until",
];

/// One table row, read.
#[derive(Clone, Debug)]
pub(crate) struct Row {
    pub(crate) line: usize,
    pub(crate) version: String,
    pub(crate) file: String,
    pub(crate) class: String,
    pub(crate) minted: String,
    pub(crate) writer: String,
    pub(crate) reader: String,
    pub(crate) state: String,
    pub(crate) last_written: String,
    pub(crate) reader_until: String,
}

impl Row {
    /// The family or lineage the row belongs to: the `Version` cell's stem.
    pub(crate) fn family(&self) -> &str {
        self.version
            .rsplit_once('/')
            .map_or(self.version.as_str(), |(family, _)| family)
    }

    fn number(&self) -> u16 {
        self.version
            .rsplit_once("/v")
            .and_then(|(_, number)| number.parse().ok())
            .expect("a closed version cell carries a number")
    }
}

/// Every row of the table, in the order the guide writes them.
pub(crate) fn rows() -> Vec<Row> {
    let text = std::fs::read_to_string(workspace().join(GUIDE)).expect("the versioning guide");
    let lines: Vec<&str> = text.lines().collect();
    let header = lines
        .iter()
        .position(|line| *line == format!("| {} |", COLUMNS.join(" | ")))
        .unwrap_or_else(|| panic!("{GUIDE} carries the lifecycle table"));
    // The header, the `---` rule, then one row per line until the table ends.
    lines[header + 2..]
        .iter()
        .zip(header + 3..)
        .take_while(|(line, _)| line.starts_with('|'))
        .map(|(line, at)| parse_row(line, at))
        .collect()
}

fn parse_row(line: &str, at: usize) -> Row {
    let cells: Vec<String> = line
        .trim()
        .trim_matches('|')
        .split('|')
        .map(unquote)
        .collect();
    assert_eq!(cells.len(), COLUMNS.len(), "{GUIDE}:{at}: nine cells");
    Row {
        line: at,
        version: cells[0].clone(),
        file: cells[1].clone(),
        class: cells[2].clone(),
        minted: cells[3].clone(),
        writer: cells[4].clone(),
        reader: cells[5].clone(),
        state: cells[6].clone(),
        last_written: cells[7].clone(),
        reader_until: cells[8].clone(),
    }
}

/// A cell's surrounding backticks, if it has them.
fn unquote(cell: &str) -> String {
    let cell = cell.trim();
    cell.strip_prefix('`')
        .and_then(|rest| rest.strip_suffix('`'))
        .unwrap_or(cell)
        .to_string()
}

fn cell<'a>(row: &'a Row, column: &str) -> &'a str {
    match column {
        "Version" => &row.version,
        "File" => &row.file,
        "Class" => &row.class,
        "Minted under" => &row.minted,
        "Writer" => &row.writer,
        "Reader" => &row.reader,
        "State" => &row.state,
        "Last written" => &row.last_written,
        "Reader until" => &row.reader_until,
        _ => panic!("no such column"),
    }
}

/// A `family/vN` or a contract label.
pub(crate) fn version_form(value: &str) -> bool {
    let Some((family, number)) = value.rsplit_once("/v") else {
        return false;
    };
    !family.is_empty()
        && family
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
        && !number.is_empty()
        && number.bytes().all(|b| b.is_ascii_digit())
}

pub(crate) fn file_form(value: &str) -> bool {
    value == FILELESS
        || (value.starts_with("contracts/")
            && value.ends_with(".schema.json")
            && !value["contracts/".len()..].contains('/'))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Class {
    Journal,
    Input,
}

impl Class {
    fn parse(value: &str) -> Option<Class> {
        match value {
            "journal" => Some(Class::Journal),
            "input" => Some(Class::Input),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Writer {
    Engine(String),
    Drivers,
    Operator,
    Looper,
    Shipped(String),
    None,
}

impl Writer {
    pub(crate) fn parse(value: &str) -> Option<Writer> {
        if let Some(path) = value.strip_prefix("engine ") {
            return Some(Writer::Engine(path.to_string()));
        }
        if let Some(path) = value.strip_prefix("shipped ") {
            return Some(Writer::Shipped(path.to_string()));
        }
        match value {
            "drivers" => Some(Writer::Drivers),
            "operator" => Some(Writer::Operator),
            "Looper" => Some(Writer::Looper),
            "none" => Some(Writer::None),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Reader {
    None,
    Set(String),
    Dispatch(String),
    Label(String),
    Load(String),
    Schema(String),
}

impl Reader {
    pub(crate) fn parse(value: &str) -> Option<Reader> {
        if value == "none" {
            return Some(Reader::None);
        }
        let (kind, item) = value.split_once(' ')?;
        let item = item.to_string();
        match kind {
            "set" => Some(Reader::Set(item)),
            "dispatch" => Some(Reader::Dispatch(item)),
            "label" => Some(Reader::Label(item)),
            "load" => Some(Reader::Load(item)),
            "schema" => Some(Reader::Schema(item)),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum State {
    Current,
    Reader,
    Record,
    Retired(String),
}

impl State {
    pub(crate) fn parse(value: &str) -> Option<State> {
        if let Some(line) = value.strip_prefix("retired (") {
            let line = line.strip_suffix(')')?;
            return engine_line(line).then(|| State::Retired(line.to_string()));
        }
        match value {
            "current" => Some(State::Current),
            "reader" => Some(State::Reader),
            "record" => Some(State::Record),
            _ => None,
        }
    }
}

/// `0.N`, the engine line form the guide and ruling 3 use.
pub(crate) fn engine_line(value: &str) -> bool {
    value
        .strip_prefix("0.")
        .is_some_and(|minor| !minor.is_empty() && minor.bytes().all(|b| b.is_ascii_digit()))
}

/// The engine line's minor number, from a `0.N` cell.
fn minor(line: &str) -> u64 {
    line.strip_prefix("0.")
        .and_then(|minor| minor.parse().ok())
        .expect("a closed engine line")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LastWritten {
    Empty,
    Never,
    Line(String),
    /// Slice 1 states the cell; ruling 3's binding measures it in slice 2.
    Unmeasured,
}

impl LastWritten {
    pub(crate) fn parse(value: &str) -> Option<LastWritten> {
        match value {
            "" => Some(LastWritten::Empty),
            "never" => Some(LastWritten::Never),
            "unmeasured" => Some(LastWritten::Unmeasured),
            _ if engine_line(value) => Some(LastWritten::Line(value.to_string())),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ReaderUntil {
    Empty,
    Decision,
    Line(String),
    Extended(String, u16),
    /// Slice 1 states the cell; ruling 3's binding computes it in slice 2.
    Unmeasured,
}

impl ReaderUntil {
    pub(crate) fn parse(value: &str) -> Option<ReaderUntil> {
        match value {
            "" => Some(ReaderUntil::Empty),
            "decision" => Some(ReaderUntil::Decision),
            "unmeasured" => Some(ReaderUntil::Unmeasured),
            _ if engine_line(value) => Some(ReaderUntil::Line(value.to_string())),
            _ => {
                let (line, decision) = value.split_once(" (decision ")?;
                let decision = decision.strip_suffix(')')?;
                let decision = decision_number(decision)?;
                engine_line(line).then(|| ReaderUntil::Extended(line.to_string(), decision))
            }
        }
    }
}

/// A four-digit decision number.
pub(crate) fn decision_number(value: &str) -> Option<u16> {
    (value.len() == 4 && value.bytes().all(|b| b.is_ascii_digit()))
        .then(|| value.parse().ok())
        .flatten()
}

/// What a `Minted under` cell names, syntactically. The minting rule reads
/// the ruling it names; this is the one home of the cell's grammar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Minted {
    /// One decision, optionally with the date of the addendum that minted
    /// the file.
    Decision {
        number: u16,
        addendum: Option<String>,
    },
    /// A file ruling 2's closed list records as standing without an accepted
    /// ruling, with the exact cell the list writes.
    Listed(String),
    /// A listed file whose ruling was withdrawn: the listed cell with the
    /// marker replaced, matched whole.
    Withdrawn(String),
}

pub(crate) fn minted_form(value: &str) -> Option<Minted> {
    if value.ends_with(", enacted, pending acceptance") {
        return Some(Minted::Listed(value.to_string()));
    }
    if value.ends_with(", withdrawn") {
        return Some(Minted::Withdrawn(value.to_string()));
    }
    if let Some((number, rest)) = value.split_once(" (") {
        let number = decision_number(number)?;
        let date = rest.strip_suffix(" addendum)")?;
        return date_form(date).then(|| Minted::Decision {
            number,
            addendum: Some(date.to_string()),
        });
    }
    decision_number(value).map(|number| Minted::Decision {
        number,
        addendum: None,
    })
}

/// `YYYY-MM-DD`, and nothing else.
pub(crate) fn date_form(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(at, b)| matches!(at, 4 | 7) || b.is_ascii_digit())
}

/// The files under `contracts/*.schema.json`, from the tree.
fn tree_files() -> BTreeSet<String> {
    let dir = workspace().join("contracts");
    std::fs::read_dir(&dir)
        .expect("contracts/")
        .filter_map(|entry| {
            let name = entry.ok()?.file_name().into_string().ok()?;
            name.ends_with(".schema.json")
                .then(|| format!("contracts/{name}"))
        })
        .collect()
}

/// The set of `File` cells, both ways against the tree.
#[test]
fn the_table_names_every_contract_file_and_no_other() {
    let rows = rows();
    let mut files: Vec<String> = rows
        .iter()
        .filter(|row| row.file != FILELESS)
        .map(|row| row.file.clone())
        .collect();
    files.sort();
    files.dedup();
    let mut tree: Vec<String> = tree_files().into_iter().collect();
    tree.sort();
    assert_eq!(files, tree, "the lifecycle table and contracts/ disagree");
    let fileless: Vec<&Row> = rows.iter().filter(|row| row.file == FILELESS).collect();
    assert_eq!(fileless.len(), 1, "exactly one file-less row");
    assert_eq!(
        fileless[0].version, "forge.phase-machine/v1",
        "the file-less row is ruling 5's label"
    );
}

#[test]
fn every_cell_is_read_from_its_closed_form() {
    let mut findings = Vec::new();
    for row in rows() {
        for (column, form) in [
            ("Version", version_form(&row.version)),
            ("File", file_form(&row.file)),
            ("Class", Class::parse(&row.class).is_some()),
            ("Minted under", minted_form(&row.minted).is_some()),
            ("Writer", Writer::parse(&row.writer).is_some()),
            ("Reader", Reader::parse(&row.reader).is_some()),
            ("State", State::parse(&row.state).is_some()),
            (
                "Last written",
                LastWritten::parse(&row.last_written).is_some(),
            ),
            (
                "Reader until",
                ReaderUntil::parse(&row.reader_until).is_some(),
            ),
        ] {
            if !form {
                findings.push(format!(
                    "{GUIDE}:{}: {}: the {column} cell '{}' is not a closed form",
                    row.line,
                    row.file,
                    cell(&row, column)
                ));
            }
        }
    }
    assert_eq!(findings, Vec::<String>::new(), "a lifecycle cell is open");
}

#[test]
fn exactly_one_row_per_family_is_current() {
    let rows = rows();
    let mut current: BTreeMap<&str, usize> = BTreeMap::new();
    for row in &rows {
        if State::parse(&row.state) == Some(State::Current) {
            *current.entry(row.family()).or_default() += 1;
        }
    }
    let mut findings = Vec::new();
    for row in &rows {
        let count = current.get(row.family()).copied().unwrap_or(0);
        if count != 1 {
            findings.push(format!(
                "{GUIDE}:{}: {}: its family {} has {count} current rows",
                row.line, row.file, row.version
            ));
        }
    }
    findings.sort();
    findings.dedup();
    assert_eq!(
        findings,
        Vec::<String>::new(),
        "a family has no current row or more than one"
    );
}

/// The Looper's one file, outside the local lineage (ruling 4's constant).
pub(crate) const LOOPER: &str = "contracts/run-manifest.v2.schema.json";

#[test]
fn a_record_row_is_only_an_older_local_run_manifest() {
    let rows = rows();
    let current = rows
        .iter()
        .find(|row| {
            row.family() == "run-manifest" && State::parse(&row.state) == Some(State::Current)
        })
        .expect("the local lineage has a current row")
        .number();
    let mut findings = Vec::new();
    for row in &rows {
        if State::parse(&row.state) != Some(State::Record) {
            continue;
        }
        let local = row
            .file
            .strip_prefix("contracts/run-manifest.v")
            .and_then(|rest| rest.strip_suffix(".schema.json"))
            .and_then(|number| number.parse::<u16>().ok());
        let admitted =
            local.is_some_and(|number| row.file != LOOPER && number < current && number != 2);
        let refused = if !admitted {
            Some(format!(
                "{GUIDE}:{}: {}: a record row is only an older file of the local run-manifest lineage",
                row.line, row.file
            ))
        } else if Class::parse(&row.class) != Some(Class::Journal) {
            Some(format!(
                "{GUIDE}:{}: {}: a record row is a journal row",
                row.line, row.file
            ))
        } else if row.writer != "none" || row.reader != "none" {
            Some(format!(
                "{GUIDE}:{}: {}: a record row has no writer and no reader",
                row.line, row.file
            ))
        } else if !matches!(
            LastWritten::parse(&row.last_written),
            Some(LastWritten::Empty)
        ) || !matches!(
            ReaderUntil::parse(&row.reader_until),
            Some(ReaderUntil::Empty)
        ) {
            Some(format!(
                "{GUIDE}:{}: {}: a record row states no line and no reader window",
                row.line, row.file
            ))
        } else {
            None
        };
        findings.extend(refused);
    }
    assert_eq!(findings, Vec::<String>::new(), "a record row is refused");
}

/// The public array a `set` row names, as data.
fn set_labels(item: &str) -> Vec<&'static str> {
    match item {
        "brokkr_core::realms::SCHEMAS" => brokkr_core::realms::SCHEMAS.to_vec(),
        "brokkr_runtime::dialect::SCHEMAS" => brokkr_runtime::dialect::SCHEMAS.to_vec(),
        other => panic!("{other} is not a public array this check knows"),
    }
}

/// The contract a `dispatch` row names, as data.
fn dispatched_contract(item: &str, number: u16) -> Option<&'static str> {
    assert_eq!(item, "brokkr_store::SeatRecordVersion");
    Some(seat_variant(number)?.contract())
}

/// The variant a row's version number names, as data.
fn seat_variant(number: u16) -> Option<brokkr_store::SeatRecordVersion> {
    Some(match number {
        1 => brokkr_store::SeatRecordVersion::V1,
        2 => brokkr_store::SeatRecordVersion::V2,
        3 => brokkr_store::SeatRecordVersion::V3,
        4 => brokkr_store::SeatRecordVersion::V4,
        5 => brokkr_store::SeatRecordVersion::V5,
        6 => brokkr_store::SeatRecordVersion::V6,
        _ => return None,
    })
}

/// The label a `label` row names, as data.
fn label_value(item: &str) -> &'static str {
    match item {
        "brokkr_protocol::PROTO" => brokkr_protocol::PROTO,
        "brokkr_core::dispatch::DISPATCH_SCHEMA_V2" => brokkr_core::dispatch::DISPATCH_SCHEMA_V2,
        "brokkr_core::dispatch::RUN_MANIFEST_SCHEMA_V2" => {
            brokkr_core::dispatch::RUN_MANIFEST_SCHEMA_V2
        }
        "brokkr_core::policy::TABLE_SCHEMA_V2" => brokkr_core::policy::TABLE_SCHEMA_V2,
        other => panic!("{other} is not a public label this check knows"),
    }
}

fn envelope(
    event_type: brokkr_core::EventType,
    payload: serde_json::Value,
    seq: u64,
) -> brokkr_core::EventEnvelope {
    brokkr_core::EventEnvelope {
        run_id: "run".into(),
        seq,
        event_id: format!("event-{seq}"),
        event_schema_version: brokkr_core::envelope::EVENT_SCHEMA_VERSION,
        event_type,
        payload,
        causation_id: None,
        correlation_id: "run".into(),
        attempt_id: None,
        recorded_at: "2026-01-01T00:00:00Z".into(),
        previous_hash: brokkr_core::canonical::ZERO_HASH.into(),
        event_hash: String::new(),
    }
    .sealed()
}

/// Drive the public loader a `load` row names with an instance of the row's
/// version, and require it to load.
fn drive_load(item: &str) {
    match item {
        "brokkr_core::policy::Machine::from_table" => {
            let bytes = std::fs::read(workspace().join("policy/phase-machine.json")).unwrap();
            let table = serde_json::from_slice(&bytes).unwrap();
            brokkr_core::policy::Machine::from_table(&table).expect("the frozen table loads");
        }
        "brokkr_core::envelope::verify_chain" => {
            let event = envelope(
                brokkr_core::EventType::RunStarted,
                serde_json::json!({"feature": "f", "manifest": {}}),
                1,
            );
            brokkr_core::envelope::verify_chain(&[event]).expect("a v1 envelope loads");
        }
        "brokkr_core::dispatch::bundle_manifest_from_run" => {
            let manifest =
                serde_json::json!({"engine": "0.12.0", "queue_entry": {"entry": 1, "claim": 2}});
            let loaded = brokkr_core::dispatch::bundle_manifest_from_run(&manifest)
                .expect("a v14 manifest loads");
            assert!(
                loaded.get("queue_entry").is_none(),
                "the v14 queue claim is read and dropped"
            );
        }
        "brokkr_core::residual::closings" => {
            let event = envelope(
                brokkr_core::EventType::OperatorCommanded,
                serde_json::json!({
                    "command": brokkr_core::residual::SUPERSEDE,
                    "operator": "operator",
                    "args": {"by": {"run_id": "run", "seq": 1}, "findings": [1]}
                }),
                1,
            );
            let marks = brokkr_core::residual::closings(&[event]);
            assert_eq!(marks.len(), 1, "a v1 supersede annotation loads");
        }
        "brokkr_core::seat::boundary_entries" => {
            let payload = serde_json::json!({
                "boundary": [{"member": "m", "boundary": "namespace", "gate": false}]
            });
            let entries = brokkr_core::seat::boundary_entries(&payload);
            assert_eq!(entries.len(), 1, "a v1 boundary entry loads");
        }
        other => panic!("{other} is not a public loader this check knows"),
    }
}

/// The workspace's engine line minor, from `Cargo.toml`, as data.
fn workspace_minor() -> u64 {
    let text = std::fs::read_to_string(workspace().join("Cargo.toml")).unwrap();
    text.lines()
        .find_map(|line| {
            let rest = line.strip_prefix("version = \"0.")?;
            let minor = rest.split('.').next()?;
            minor.parse().ok()
        })
        .expect("Cargo.toml carries the workspace version")
}

/// A dispatch row's `Last written` line is the line its variant dispatches
/// for, and the next line it does not; v2's `never` is selected by no line.
fn bind_dispatch(row: &Row, number: u16, findings: &mut Vec<String>) {
    let Some(variant) = seat_variant(number) else {
        findings.push(format!(
            "{GUIDE}:{}: {}: no seat-record variant names this row",
            row.line, row.file
        ));
        return;
    };
    let workspace = workspace_minor();
    match LastWritten::parse(&row.last_written) {
        Some(LastWritten::Line(line)) => {
            let written = format!("{line}.0");
            if brokkr_store::SeatRecordVersion::of_engine(&written) != variant {
                findings.push(format!(
                    "{GUIDE}:{}: {}: of_engine dispatches {written} to another version",
                    row.line, row.file
                ));
            }
            let next = minor(&line) + 1;
            if next <= workspace
                && brokkr_store::SeatRecordVersion::of_engine(&format!("0.{next}.0")) == variant
            {
                findings.push(format!(
                    "{GUIDE}:{}: {}: of_engine still dispatches 0.{next} to this version, so its Last written is not its last line",
                    row.line, row.file
                ));
            }
        }
        Some(LastWritten::Never) => {
            for line in 0..=workspace {
                let engine = format!("0.{line}.0");
                if brokkr_store::SeatRecordVersion::of_engine(&engine) == variant {
                    findings.push(format!(
                        "{GUIDE}:{}: {}: of_engine dispatches {engine} to a version its row says was never written",
                        row.line, row.file
                    ));
                }
            }
        }
        _ => findings.push(format!(
            "{GUIDE}:{}: {}: a dispatch row states an engine line or never",
            row.line, row.file
        )),
    }
}

/// The `set` rows of one family equal its public array, both ways.
fn bind_set(rows: &[Row]) -> Vec<String> {
    let mut findings = Vec::new();
    let families: BTreeSet<&str> = rows
        .iter()
        .filter(|row| matches!(Reader::parse(&row.reader), Some(Reader::Set(_))))
        .map(Row::family)
        .collect();
    for family in families {
        let members: Vec<&Row> = rows
            .iter()
            .filter(|row| row.family() == family)
            .filter(|row| {
                matches!(
                    State::parse(&row.state),
                    Some(State::Current | State::Reader)
                )
            })
            .collect();
        let items: BTreeSet<String> = members
            .iter()
            .filter_map(|row| match Reader::parse(&row.reader) {
                Some(Reader::Set(item)) => Some(item),
                _ => None,
            })
            .collect();
        if items.len() != 1 {
            findings.push(format!(
                "{GUIDE}: {family}: its rows name {} set sources",
                items.len()
            ));
            continue;
        }
        let labels = set_labels(&items.into_iter().next().expect("one item"));
        let listed: BTreeSet<&str> = members.iter().map(|row| row.version.as_str()).collect();
        let array: BTreeSet<&str> = labels.iter().copied().collect();
        for missing in listed.difference(&array) {
            let row = members
                .iter()
                .find(|row| row.version == *missing)
                .expect("a listed row");
            findings.push(format!(
                "{GUIDE}:{}: {}: the reader array does not admit '{}'",
                row.line, row.file, missing
            ));
        }
        for extra in array.difference(&listed) {
            findings.push(format!(
                "{GUIDE}: {family}: the reader array admits '{extra}', which has no current or reader row"
            ));
        }
    }
    findings
}

#[test]
fn every_reader_cell_drives_the_source_its_kind_names() {
    let rows = rows();
    let private: BTreeMap<&str, &str> = PRIVATE.into_iter().collect();
    let mut findings = bind_set(&rows);
    for row in &rows {
        let Some(reader) = Reader::parse(&row.reader) else {
            continue;
        };
        match reader {
            Reader::None | Reader::Set(_) => {}
            Reader::Dispatch(item) => {
                let contract = dispatched_contract(&item, row.number());
                if contract != Some(row.file.as_str()) {
                    findings.push(format!(
                        "{GUIDE}:{}: {}: the dispatch source names {contract:?}",
                        row.line, row.file
                    ));
                }
                if matches!(
                    State::parse(&row.state),
                    Some(State::Reader | State::Retired(_))
                ) {
                    bind_dispatch(row, row.number(), &mut findings);
                }
            }
            Reader::Label(item) => {
                if label_value(&item) != row.version {
                    findings.push(format!(
                        "{GUIDE}:{}: {}: the label reads '{}'",
                        row.line,
                        row.file,
                        label_value(&item)
                    ));
                }
            }
            Reader::Load(item) => {
                if let Some(expected) = private.get(row.file.as_str()) {
                    if *expected != format!("load {item}") {
                        findings.push(format!(
                            "{GUIDE}:{}: {}: the private reader is not the one this check names",
                            row.line, row.file
                        ));
                    }
                } else {
                    drive_load(&item);
                }
            }
            Reader::Schema(item) => {
                if private.get(row.file.as_str()) != Some(&format!("schema {item}").as_str()) {
                    findings.push(format!(
                        "{GUIDE}:{}: {}: a schema source must be private and checked in its crate",
                        row.line, row.file
                    ));
                }
            }
        }
    }
    assert_eq!(findings, Vec::<String>::new(), "a reader does not bind");
}

#[test]
fn every_writer_path_exists() {
    let mut findings = Vec::new();
    for row in rows() {
        let path = match Writer::parse(&row.writer) {
            Some(Writer::Engine(path) | Writer::Shipped(path)) => path,
            _ => continue,
        };
        if !workspace().join(&path).exists() {
            findings.push(format!(
                "{GUIDE}:{}: {}: the writer path '{path}' does not exist",
                row.line, row.file
            ));
        }
    }
    assert_eq!(findings, Vec::<String>::new(), "a writer path is missing");
}

#[test]
fn the_file_less_row_is_ruling_five_and_loads_the_frozen_table() {
    let row = rows()
        .into_iter()
        .find(|row| row.file == FILELESS)
        .expect("the file-less row");
    assert_eq!(row.version, brokkr_core::policy::TABLE_SCHEMA_V1);
    assert_eq!(Class::parse(&row.class), Some(Class::Input));
    assert_eq!(Writer::parse(&row.writer), Some(Writer::None));
    assert_eq!(
        Reader::parse(&row.reader),
        Some(Reader::Load(
            "brokkr_core::policy::Machine::from_table".into()
        ))
    );
    assert_eq!(State::parse(&row.state), Some(State::Reader));
    assert_eq!(
        LastWritten::parse(&row.last_written),
        Some(LastWritten::Empty)
    );
    assert_eq!(
        ReaderUntil::parse(&row.reader_until),
        Some(ReaderUntil::Decision)
    );
    drive_load("brokkr_core::policy::Machine::from_table");
}

#[test]
fn the_private_rows_are_exactly_the_ones_their_crates_check() {
    let rows = rows();
    let mut seen = BTreeSet::new();
    for row in &rows {
        if let Some((file, reader)) = PRIVATE.into_iter().find(|(file, _)| *file == row.file) {
            assert_eq!(
                row.reader, reader,
                "{GUIDE}:{}: {}: the private reader moved",
                row.line, row.file
            );
            seen.insert(file);
        }
    }
    assert_eq!(
        seen.len(),
        PRIVATE.len(),
        "a private reader has no row in the table"
    );
}
