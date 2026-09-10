//! The experimental startup template as an audited rule ledger, and the pure
//! check that proves a rendered profile carries exactly that authority
//! (decision 0046 slice II, design D3; spec requirement "The experimental
//! startup template is an audited rule ledger").
//!
//! Everything here is host-independent ordinary Rust exercised on Linux. It
//! proves spelling, layout and the disjoint-union equality; it claims no file
//! system fact and no native Seatbelt result. The native observer runs the
//! same check over the concrete profile it is about to hand `sandbox-exec`.

use std::sync::LazyLock;

use super::controls::{
    CREDENTIAL_READ_DENIAL_TARGETS, DATA_VOLUME_CREDENTIAL_READ_DENIAL_TARGET,
    HOST_WRITE_DENIAL_TARGET,
};
use super::StartupNegativeAllowance;
use brokkr_protocol::hands::HOST_TOOLCHAIN_BINDS;

/// The typed placeholder replacing the concrete cell root in normalized units.
pub const PLACEHOLDER_CELL_ROOT: &str = "<cell-root>";
/// The typed placeholder replacing the concrete payload root.
pub const PLACEHOLDER_PAYLOAD_ROOT: &str = "<payload-root>";
/// The typed placeholder replacing the concrete helper path.
pub const PLACEHOLDER_HELPER: &str = "<helper>";

/// The five toolchain binds whose programs a boxed `bash -lc` may execute.
/// Each must be a source in [`HOST_TOOLCHAIN_BINDS`]; the check confirms that,
/// and a `process-exec` toolchain unit on any other bind fails by name.
pub const PROGRAM_BINDS: [&str; 5] = ["/usr/bin", "/usr/libexec", "/usr/local", "/bin", "/sbin"];

/// One `allow` form with one operation and at most one simple filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleUnit {
    pub operation: String,
    pub filter: Option<RuleFilter>,
}

/// One simple filter: a filter name and exactly one argument, a string or the
/// symbol `self`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleFilter {
    pub name: String,
    pub target: String,
}

impl RuleFilter {
    fn is_self(&self) -> bool {
        self.name == "target" && self.target == "self"
    }

    fn is_literal(&self) -> bool {
        self.name == "literal"
    }

    fn is_subpath(&self) -> bool {
        self.name == "subpath"
    }

    /// A diagnosis-admitted filter names a single object.
    fn is_single_object(&self) -> bool {
        self.is_literal()
            || self.is_self()
            || matches!(
                self.name.as_str(),
                "sysctl-name" | "ipc-posix-name" | "global-name"
            )
    }
}

impl RuleUnit {
    /// Render this unit as one normalized or concrete `allow` form.
    pub fn render(&self) -> String {
        match &self.filter {
            None => format!("(allow {})", self.operation),
            Some(filter) if filter.is_self() => {
                format!("(allow {} (target self))", self.operation)
            }
            Some(filter) => {
                format!(
                    "(allow {} ({} \"{}\"))",
                    self.operation, filter.name, filter.target
                )
            }
        }
    }

    fn is_process_unit(&self) -> bool {
        self.operation.starts_with("process")
    }
}

/// The closed set of hands elements a baseline entry may name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandsElement {
    Toolchain,
    SystemLibrary,
    WritableWorktree,
    DeviceSet,
    Shell,
}

/// The recorded justification kind of a baseline entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaselineKind {
    HandsElement(HandsElement),
    ExecutionInput,
    ProbeHarnessNeed,
}

/// A recorded system-library correction: a committed unit replaced because
/// native denial evidence showed its read refused under another resolved
/// spelling. It keeps the same justification and records the evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemLibraryCorrection {
    pub replaces: String,
    pub resolved: String,
    pub evidence: String,
}

/// The baseline half's justification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Baseline {
    pub kind: BaselineKind,
    pub justification: String,
    pub correction: Option<SystemLibraryCorrection>,
}

/// The diagnosis-admitted half's recorded evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosisAdmission {
    pub process: String,
    pub consumer: String,
    pub evidence: String,
    pub removal: String,
}

/// Every unit carries exactly one class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerClass {
    Baseline(Baseline),
    DiagnosisAdmitted(DiagnosisAdmission),
}

/// One ledger entry: a rule unit and its single class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerEntry {
    pub unit: RuleUnit,
    pub class: LedgerClass,
}

/// A typed concrete input the check validates before any normalization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckInputs {
    pub cell_root: String,
    pub payload_root: String,
    pub inputs_dir: String,
    pub helper: String,
}

/// A refused check, naming what was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckRefusal {
    pub reason: String,
}

impl CheckRefusal {
    fn new(reason: impl Into<String>) -> CheckRefusal {
        CheckRefusal {
            reason: reason.into(),
        }
    }
}

impl std::fmt::Display for CheckRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.reason)
    }
}

/// Concrete inputs that passed stage-1 validation. Its fields are private and
/// it is constructed only by [`validate_inputs`], so the normalizer cannot run
/// over unvalidated values.
#[derive(Debug, Clone)]
struct ValidatedInputs {
    cell_root: String,
    payload_root: String,
    helper: String,
}

fn refusal(reason: impl Into<String>) -> CheckRefusal {
    CheckRefusal::new(reason)
}

// ---------------------------------------------------------------------------
// The committed ledger
// ---------------------------------------------------------------------------

fn filter(name: &str, target: &str) -> Option<RuleFilter> {
    Some(RuleFilter {
        name: name.to_string(),
        target: target.to_string(),
    })
}

pub fn unit(operation: &str, name: &str, target: &str) -> RuleUnit {
    RuleUnit {
        operation: operation.to_string(),
        filter: filter(name, target),
    }
}

fn unfiltered(operation: &str) -> RuleUnit {
    RuleUnit {
        operation: operation.to_string(),
        filter: None,
    }
}

pub fn entry(unit: RuleUnit, element: HandsElement, justification: &str) -> LedgerEntry {
    LedgerEntry {
        unit,
        class: LedgerClass::Baseline(Baseline {
            kind: BaselineKind::HandsElement(element),
            justification: justification.to_string(),
            correction: None,
        }),
    }
}

fn execution_input(unit: RuleUnit, justification: &str) -> LedgerEntry {
    LedgerEntry {
        unit,
        class: LedgerClass::Baseline(Baseline {
            kind: BaselineKind::ExecutionInput,
            justification: justification.to_string(),
            correction: None,
        }),
    }
}

fn harness(unit: RuleUnit, justification: &str) -> LedgerEntry {
    LedgerEntry {
        unit,
        class: LedgerClass::Baseline(Baseline {
            kind: BaselineKind::ProbeHarnessNeed,
            justification: justification.to_string(),
            correction: None,
        }),
    }
}

fn diagnosis(
    unit: RuleUnit,
    process: &str,
    consumer: &str,
    evidence: &str,
    removal: &str,
) -> LedgerEntry {
    LedgerEntry {
        unit,
        class: LedgerClass::DiagnosisAdmitted(DiagnosisAdmission {
            process: process.to_string(),
            consumer: consumer.to_string(),
            evidence: evidence.to_string(),
            removal: removal.to_string(),
        }),
    }
}

fn build_ledger() -> Vec<LedgerEntry> {
    let mut ledger =
        vec![
        entry(
            unfiltered("process-fork"),
            HandsElement::Shell,
            "0043 ruling 1 runs each call as `bash -lc`, whose commands fork children. \
             The probe's consumers are the ordinary child and the `setsid` and double-fork \
             descendants.",
        ),
        execution_input(
            unit("process-exec", "literal", PLACEHOLDER_HELPER),
            "`sandbox-exec` executes the exact helper under the profile, at the validated \
             `<helper>` spelling. The helper re-executes itself for its child and descendant \
             roles through that same spelling, which it receives as a launch argument.",
        ),
        entry(
            unit("process-exec", "subpath", "/usr/bin"),
            HandsElement::Toolchain,
            "The `/usr/bin` toolchain bind, whose programs 0043 ruling 1 lets `bash -lc` run.",
        ),
        entry(
            unit("process-exec", "subpath", "/usr/libexec"),
            HandsElement::Toolchain,
            "The `/usr/libexec` toolchain bind, which holds helper programs the toolchain runs.",
        ),
        entry(
            unit("process-exec", "subpath", "/usr/local"),
            HandsElement::Toolchain,
            "The `/usr/local` toolchain bind, where host-installed programs live.",
        ),
        entry(
            unit("process-exec", "subpath", "/bin"),
            HandsElement::Toolchain,
            "The `/bin` toolchain bind. Gate B's escape, guard and peer adversaries run the \
             real `/bin/launchctl`, so their denials measure launchd authority rather than a \
             refused exec.",
        ),
        entry(
            unit("process-exec", "subpath", "/sbin"),
            HandsElement::Toolchain,
            "The `/sbin` toolchain bind.",
        ),
        entry(
            unit("file-read*", "subpath", "/usr/bin"),
            HandsElement::Toolchain,
            "The `/usr/bin` toolchain bind, read-only.",
        ),
        entry(
            unit("file-read*", "subpath", "/usr/lib"),
            HandsElement::Toolchain,
            "The `/usr/lib` toolchain bind, read-only. It holds `dyld`.",
        ),
        entry(
            unit("file-read*", "subpath", "/usr/libexec"),
            HandsElement::Toolchain,
            "The `/usr/libexec` toolchain bind, read-only.",
        ),
        entry(
            unit("file-read*", "subpath", "/usr/share"),
            HandsElement::Toolchain,
            "The `/usr/share` toolchain bind, read-only.",
        ),
        entry(
            unit("file-read*", "subpath", "/usr/local"),
            HandsElement::Toolchain,
            "The `/usr/local` toolchain bind, read-only.",
        ),
        entry(
            unit("file-read*", "subpath", "/bin"),
            HandsElement::Toolchain,
            "The `/bin` toolchain bind.",
        ),
        entry(
            unit("file-read*", "subpath", "/sbin"),
            HandsElement::Toolchain,
            "The `/sbin` toolchain bind.",
        ),
        entry(
            unit("file-read*", "subpath", "/System/Library"),
            HandsElement::SystemLibrary,
            "macOS system libraries and frameworks, the image of the `/lib`, `/lib64` and \
             `/usr/lib` binds.",
        ),
        entry(
            unit("file-read*", "subpath", "/System/Volumes/Preboot/Cryptexes/OS"),
            HandsElement::SystemLibrary,
            "The OS cryptex, where current macOS keeps the dyld shared cache. It is the same \
             element.",
        ),
        execution_input(
            unit("file-read*", "literal", PLACEHOLDER_HELPER),
            "The committed helper bytes, which no other unit covers.",
        ),
        harness(
            unit("file-read*", "subpath", "<cell-root>/inputs"),
            "Immutable launch inputs, such as the guard and peer labels the denial adversaries \
             target.",
        ),
        harness(
            unit("file-read*", "subpath", PLACEHOLDER_PAYLOAD_ROOT),
            "The payload's own stages, ready token, identities and heartbeat.",
        ),
        entry(
            unit("file-write*", "subpath", PLACEHOLDER_PAYLOAD_ROOT),
            HandsElement::WritableWorktree,
            "The writable worktree's image, and the only writable location. It holds the \
             payload state above.",
        ),
        entry(
            unit("file-read*", "literal", "/dev/null"),
            HandsElement::DeviceSet,
            "The box's private device set (`hands.rs` `--dev /dev`). It is also the ordinary \
             child's null stdin.",
        ),
        entry(
            unit("file-read*", "literal", "/dev/urandom"),
            HandsElement::DeviceSet,
            "The same device set.",
        ),
        entry(
            unit("file-read*", "literal", "/dev/random"),
            HandsElement::DeviceSet,
            "The same device set.",
        ),
    ];
    ledger.push(diagnosis(
        unit("file-read*", "literal", "/"),
        "the helper, in `dyld` process initialisation",
        "the filesystem-root inode read the dynamic loader performs",
        "Without it, `9f4c2c9` aborted before any stage. With it, `fa7ece5` reached \
         `executable`.",
        "root-inode-read",
    ));
    ledger
}

/// The one committed, typed startup-rule ledger. It lives beside the removal
/// set and is the only source of the candidate's authority.
pub static STARTUP_RULE_LEDGER: LazyLock<Vec<LedgerEntry>> = LazyLock::new(build_ledger);

// ---------------------------------------------------------------------------
// The fa7 disposition table
// ---------------------------------------------------------------------------

/// How a measured fa7 template unit is disposed of in the candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fa7Disposition {
    /// The unit is carried by the ledger unchanged.
    Carried,
    /// The unit carries no authority in the candidate.
    Withdrawn,
    /// A narrower unit replaces it.
    Narrowed,
}

/// One fa7 unit's recorded disposition and reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fa7Record {
    pub unit: RuleUnit,
    pub disposition: Fa7Disposition,
    pub reason: &'static str,
}

/// The measured `fa7ece5` `sandbox_profile` template, normalized so its
/// controller-generated root, payload root and helper take the typed
/// placeholders. It is the probe's test data, kept verbatim otherwise.
pub const FA7_TEMPLATE: &str = "\
(version 1)\n\
(deny default)\n\
(allow process*)\n\
(allow signal (target self))\n\
(allow file-read* (literal \"/\"))\n\
(allow file-read* (subpath \"/usr\") (subpath \"/bin\") (subpath \"/sbin\") (subpath \"/System\") (subpath \"/Library\") (subpath \"/private/tmp\") (subpath \"/private/var/tmp\") (subpath \"<cell-root>\") (literal \"<helper>\") (literal \"/dev/null\") (literal \"/dev/urandom\") (literal \"/dev/random\") (literal \"/dev/dtracehelper\"))\n\
(allow file-write* (subpath \"<payload-root>\"))\n\
(allow sysctl-read)\n\
(allow ipc-posix-shm)\n";

/// The fa7 unit dispositions, one per normalized unit of [`FA7_TEMPLATE`].
pub static FA7_RECORDS: LazyLock<Vec<Fa7Record>> = LazyLock::new(|| {
    let records = vec![
        Fa7Record {
            unit: unfiltered("process*"),
            disposition: Fa7Disposition::Narrowed,
            reason: "The family also carries process-information and code-signing operations \
                     on other processes. The hands box unshares pid, and no consumer needs \
                     them.",
        },
        Fa7Record {
            unit: unit("signal", "target", "self"),
            disposition: Fa7Disposition::Withdrawn,
            reason: "No consumer is named, and no measurement shows it is needed.",
        },
        Fa7Record {
            unit: unit("file-read*", "literal", "/"),
            disposition: Fa7Disposition::Carried,
            reason: "The root-inode read carries into the ledger as the one diagnosis-admitted \
                     entry.",
        },
        Fa7Record {
            unit: unit("file-read*", "subpath", "/usr"),
            disposition: Fa7Disposition::Narrowed,
            reason: "Narrowed to one subpath per `hands.rs` `/usr` bind: `/usr/bin`, \
                     `/usr/lib`, `/usr/libexec`, `/usr/share` and `/usr/local`.",
        },
        Fa7Record {
            unit: unit("file-read*", "subpath", "/bin"),
            disposition: Fa7Disposition::Carried,
            reason: "Carried as the `/bin` toolchain read.",
        },
        Fa7Record {
            unit: unit("file-read*", "subpath", "/sbin"),
            disposition: Fa7Disposition::Carried,
            reason: "Carried as the `/sbin` toolchain read.",
        },
        Fa7Record {
            unit: unit("file-read*", "subpath", "/System"),
            disposition: Fa7Disposition::Narrowed,
            reason: "Narrowed to `/System/Library` and the OS cryptex. `/System` also holds \
                     `/System/Volumes/Data`, the data volume that also spells `/Users` and \
                     `/private`. No hands element grants it.",
        },
        Fa7Record {
            unit: unit("file-read*", "subpath", "/Library"),
            disposition: Fa7Disposition::Withdrawn,
            reason: "Host-wide preferences and keychain directories are not a hands element, \
                     and the probe names no consumer. Production toolchain paths come from the \
                     production profile's declarations, not from this template.",
        },
        Fa7Record {
            unit: unit("file-read*", "subpath", "/private/tmp"),
            disposition: Fa7Disposition::Withdrawn,
            reason: "Host tmp contradicts the private per-call `/tmp` of 0043 ruling 1, and no \
                     consumer is named.",
        },
        Fa7Record {
            unit: unit("file-read*", "subpath", "/private/var/tmp"),
            disposition: Fa7Disposition::Withdrawn,
            reason: "The same contradiction.",
        },
        Fa7Record {
            unit: unit("file-read*", "subpath", PLACEHOLDER_CELL_ROOT),
            disposition: Fa7Disposition::Narrowed,
            reason: "Narrowed to `<cell-root>/inputs` and `<payload-root>`. Guard and observer \
                     state needs no payload read.",
        },
        Fa7Record {
            unit: unit("file-read*", "literal", PLACEHOLDER_HELPER),
            disposition: Fa7Disposition::Carried,
            reason: "The committed helper literal is justified baseline, as an execution input.",
        },
        Fa7Record {
            unit: unit("file-read*", "literal", "/dev/null"),
            disposition: Fa7Disposition::Carried,
            reason: "Carried as the device set.",
        },
        Fa7Record {
            unit: unit("file-read*", "literal", "/dev/urandom"),
            disposition: Fa7Disposition::Carried,
            reason: "Carried as the device set.",
        },
        Fa7Record {
            unit: unit("file-read*", "literal", "/dev/random"),
            disposition: Fa7Disposition::Carried,
            reason: "`8c53dce`'s `/dev/random` addition is justified baseline, as a hands \
                     element.",
        },
        Fa7Record {
            unit: unit("file-read*", "literal", "/dev/dtracehelper"),
            disposition: Fa7Disposition::Withdrawn,
            reason: "It is not in the box device set, and no measurement shows it is needed. \
                     `8c53dce`'s addition is withdrawn.",
        },
        Fa7Record {
            unit: unit("file-write*", "subpath", PLACEHOLDER_PAYLOAD_ROOT),
            disposition: Fa7Disposition::Carried,
            reason: "Carried as the writable worktree.",
        },
        Fa7Record {
            unit: unfiltered("sysctl-read"),
            disposition: Fa7Disposition::Withdrawn,
            reason: "Unfiltered, it covers the process-table and process-argument sysctls that \
                     describe other processes.",
        },
        Fa7Record {
            unit: unfiltered("ipc-posix-shm"),
            disposition: Fa7Disposition::Withdrawn,
            reason: "Unfiltered, it opens shared memory with any same-user process. The hands \
                     box unshares ipc.",
        },
    ];
    records
});

/// The historical targets of withdrawn or narrowed fa7 units. A system-library
/// correction SHALL NOT equal or contain one of these.
pub const FA7_HISTORICAL_TARGETS: [&str; 6] = [
    "/usr",
    "/System",
    "/Library",
    "/private/tmp",
    "/private/var/tmp",
    PLACEHOLDER_CELL_ROOT,
];

/// Parse the retained fa7 template and require a disposition for every unit,
/// including the four `8c53dce` additions.
pub fn check_fa7_dispositions() -> Result<(), CheckRefusal> {
    let parsed = parse_template(FA7_TEMPLATE)?;
    let records = &*FA7_RECORDS;
    for unit in &parsed.units {
        let found = records.iter().find(|record| &record.unit == unit);
        match found {
            Some(record) if !record.reason.is_empty() => {}
            Some(_) => {
                return Err(refusal(format!(
                    "fa7 unit {} has an empty disposition reason",
                    unit.render()
                )))
            }
            None => {
                return Err(refusal(format!(
                    "fa7 unit {} has no disposition record",
                    unit.render()
                )))
            }
        }
    }
    for record in records {
        if !parsed.units.contains(&record.unit) {
            return Err(refusal(format!(
                "fa7 disposition names a unit the retained template lacks: {}",
                record.unit.render()
            )));
        }
    }
    // The four `8c53dce` additions keep exactly the required disposition.
    let required = [
        (
            unit("file-read*", "subpath", "/private/var/tmp"),
            Fa7Disposition::Withdrawn,
        ),
        (
            unit("file-read*", "literal", PLACEHOLDER_HELPER),
            Fa7Disposition::Carried,
        ),
        (
            unit("file-read*", "literal", "/dev/random"),
            Fa7Disposition::Carried,
        ),
        (
            unit("file-read*", "literal", "/dev/dtracehelper"),
            Fa7Disposition::Withdrawn,
        ),
    ];
    for (unit, disposition) in required {
        match records.iter().find(|record| record.unit == unit) {
            Some(record) if record.disposition == disposition => {}
            _ => {
                return Err(refusal(format!(
                    "the `8c53dce` unit {} does not keep its required disposition",
                    unit.render()
                )))
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// The closed grammar parser
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
enum Sexp {
    Atom(String),
    Str(String),
    List(Vec<Sexp>),
}

impl Sexp {
    fn atom(&self) -> Option<&str> {
        match self {
            Sexp::Atom(value) => Some(value),
            _ => None,
        }
    }
}

fn tokenize(text: &str) -> Result<Vec<Tok>, CheckRefusal> {
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        let character = bytes[index] as char;
        match character {
            ' ' | '\t' | '\n' | '\r' => index += 1,
            '(' => {
                tokens.push(Tok::LParen);
                index += 1;
            }
            ')' => {
                tokens.push(Tok::RParen);
                index += 1;
            }
            '"' => {
                let start = index + 1;
                let mut end = start;
                while end < bytes.len() && bytes[end] as char != '"' {
                    if bytes[end] == b'\\' {
                        return Err(refusal(
                            "the template holds a string that needs an escape sequence",
                        ));
                    }
                    end += 1;
                }
                if end >= bytes.len() {
                    return Err(refusal("the template holds an unbalanced string"));
                }
                tokens.push(Tok::Str(text[start..end].to_string()));
                index = end + 1;
            }
            ';' => {
                return Err(refusal("the template holds a comment"));
            }
            _ if is_symbol_char(character) => {
                let start = index;
                let mut end = index;
                while end < bytes.len() && is_symbol_char(bytes[end] as char) {
                    end += 1;
                }
                tokens.push(Tok::Atom(text[start..end].to_string()));
                index = end;
            }
            _ => {
                return Err(refusal(format!(
                    "the template holds a token outside the closed grammar: '{character}'"
                )))
            }
        }
    }
    Ok(tokens)
}

fn is_symbol_char(character: char) -> bool {
    character.is_ascii_alphanumeric() || "-_*/.<>".contains(character)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    LParen,
    RParen,
    Atom(String),
    Str(String),
}

struct Parser<'a> {
    tokens: &'a [Tok],
    index: usize,
}

impl<'a> Parser<'a> {
    fn sexp(&mut self) -> Result<Sexp, CheckRefusal> {
        let token = self.tokens.get(self.index).cloned();
        match token {
            Some(Tok::LParen) => {
                self.index += 1;
                let mut items = Vec::new();
                loop {
                    match self.tokens.get(self.index) {
                        Some(Tok::RParen) => {
                            self.index += 1;
                            return Ok(Sexp::List(items));
                        }
                        None => return Err(refusal("the template holds unbalanced parentheses")),
                        _ => items.push(self.sexp()?),
                    }
                }
            }
            Some(Tok::RParen) => Err(refusal("the template holds unbalanced parentheses")),
            Some(Tok::Atom(value)) => {
                self.index += 1;
                Ok(Sexp::Atom(value))
            }
            Some(Tok::Str(value)) => {
                self.index += 1;
                Ok(Sexp::Str(value))
            }
            None => Err(refusal("the template ended before a form was complete")),
        }
    }
}

/// The parsed template: the closed grammar's units, in order.
pub struct ParsedTemplate {
    pub units: Vec<RuleUnit>,
}

/// Parse the template's frame and `allow` forms into normalized-shape units.
/// It refuses everything the closed normalization refuses, naming what it
/// found; it never keeps a first operation, skips a form or unescapes a
/// string.
pub fn parse_template(text: &str) -> Result<ParsedTemplate, CheckRefusal> {
    let tokens = tokenize(text)?;
    let mut parser = Parser {
        tokens: &tokens,
        index: 0,
    };
    let mut forms = Vec::new();
    while parser.index < tokens.len() {
        forms.push(parser.sexp()?);
    }
    if forms.len() < 2 {
        return Err(refusal("the template is missing its two-form frame"));
    }
    match &forms[0] {
        Sexp::List(items)
            if items.len() == 2
                && items[0].atom() == Some("version")
                && items[1].atom() == Some("1") => {}
        other => {
            return Err(refusal(format!(
                "the template does not open with exactly (version 1): {other:?}"
            )))
        }
    }
    match &forms[1] {
        Sexp::List(items)
            if items.len() == 2
                && items[0].atom() == Some("deny")
                && items[1].atom() == Some("default") => {}
        other => {
            return Err(refusal(format!(
                "the template does not open with exactly (deny default): {other:?}"
            )))
        }
    }

    let mut units: Vec<RuleUnit> = Vec::new();
    for form in &forms[2..] {
        let items = match form {
            Sexp::List(items) => items,
            Sexp::Atom(atom) => {
                return Err(refusal(format!(
                    "the template holds a bare top-level atom: '{atom}'"
                )))
            }
            Sexp::Str(_) => return Err(refusal("the template holds a top-level string")),
        };
        let head = items
            .first()
            .and_then(Sexp::atom)
            .ok_or_else(|| refusal("the template holds an empty or non-symbol top-level form"))?;
        if head == "version" {
            return Err(refusal("the template holds a second (version ...) form"));
        }
        if head == "deny" {
            return Err(refusal("the template holds a top-level deny form"));
        }
        if head != "allow" {
            return Err(refusal(format!(
                "the template holds a top-level form other than the frame and allow: '{head}'"
            )));
        }
        let tail = &items[1..];
        let mut operations: Vec<&str> = Vec::new();
        let mut filters: Vec<&Sexp> = Vec::new();
        let mut seen_filter = false;
        for item in tail {
            match item {
                Sexp::Atom(operation) => {
                    if seen_filter {
                        return Err(refusal(format!(
                            "the allow form holds a bare atom after a filter: '{operation}'"
                        )));
                    }
                    operations.push(operation);
                }
                Sexp::List(_) => {
                    seen_filter = true;
                    filters.push(item);
                }
                Sexp::Str(_) => {
                    return Err(refusal("the allow form holds a bare string"));
                }
            }
        }
        if operations.is_empty() {
            return Err(refusal("the allow form names no operation"));
        }
        if operations.len() > 1 {
            return Err(refusal(format!(
                "the allow form names more than one operation: {}; after the first: {}",
                operations.join(", "),
                operations[1..].join(", ")
            )));
        }
        let operation = operations[0];
        if operation == "default" {
            return Err(refusal("the template holds an allow default form"));
        }
        let mut parsed_filters = Vec::new();
        for filter in filters {
            let list = match filter {
                Sexp::List(list) => list,
                _ => unreachable!(),
            };
            let name = list
                .first()
                .and_then(Sexp::atom)
                .ok_or_else(|| refusal("an allow filter has no symbol name"))?;
            if name == "with" {
                return Err(refusal(format!(
                    "the template holds an action modifier: (with {})",
                    list.get(1).and_then(Sexp::atom).unwrap_or("...")
                )));
            }
            if matches!(name, "require-any" | "require-all" | "require-not") {
                return Err(refusal(format!(
                    "the template holds a compound filter: {name}"
                )));
            }
            match list.as_slice() {
                [_, Sexp::Str(target)] => parsed_filters.push(RuleFilter {
                    name: name.to_string(),
                    target: target.clone(),
                }),
                [_, Sexp::Atom(target)] if target == "self" => parsed_filters.push(RuleFilter {
                    name: name.to_string(),
                    target: target.clone(),
                }),
                _ => {
                    return Err(refusal(format!(
                        "the filter {name} does not carry exactly one string or self argument"
                    )))
                }
            }
        }
        if parsed_filters.is_empty() {
            units.push(RuleUnit {
                operation: operation.to_string(),
                filter: None,
            });
        } else {
            for filter in parsed_filters {
                units.push(RuleUnit {
                    operation: operation.to_string(),
                    filter: Some(filter),
                });
            }
        }
    }

    // The same unit rendered twice is refused.
    for (index, unit) in units.iter().enumerate() {
        if units[index + 1..].contains(unit) {
            return Err(refusal(format!(
                "the template renders the same unit twice: {}",
                unit.render()
            )));
        }
    }

    Ok(ParsedTemplate { units })
}

// ---------------------------------------------------------------------------
// Input validation and normalization
// ---------------------------------------------------------------------------

fn validate_inputs(inputs: &CheckInputs) -> Result<ValidatedInputs, CheckRefusal> {
    let cell_root = &inputs.cell_root;
    let payload_root = &inputs.payload_root;
    let inputs_dir = &inputs.inputs_dir;
    let helper = &inputs.helper;

    if payload_root != &format!("{cell_root}/payload") {
        return Err(refusal(format!(
            "payload root {payload_root:?} is not the cell root joined with `payload`"
        )));
    }
    if inputs_dir != &format!("{cell_root}/inputs") {
        return Err(refusal(format!(
            "inputs directory {inputs_dir:?} is not the cell root joined with `inputs`"
        )));
    }
    validate_canonical("cell root", cell_root)?;
    validate_canonical("helper", helper)?;

    if cell_root == "/" {
        return Err(refusal("the cell root is `/`"));
    }
    if overlaps(cell_root, "/System/Volumes/Data") {
        return Err(refusal(
            "the cell root is, lies under or contains `/System/Volumes/Data`",
        ));
    }

    // The cell root may not overlap any spelling of a host-toolchain source, a
    // committed system-library target, a device-set literal or a path-valued
    // denial-control target. The refusal names the source as well as the
    // spelling it matched.
    for source in HOST_TOOLCHAIN_BINDS {
        for spelling in spelling_set(source) {
            if overlaps(cell_root, &spelling) {
                return Err(refusal(format!(
                    "the cell root {cell_root:?} overlaps the host-toolchain source \
                     {source:?} at its spelling {spelling:?}"
                )));
            }
        }
    }
    for target in system_library_targets() {
        if overlaps(cell_root, &target) {
            return Err(refusal(format!(
                "the cell root {cell_root:?} overlaps the system-library target {target:?}"
            )));
        }
    }
    for target in device_set_literals() {
        if overlaps(cell_root, &target) {
            return Err(refusal(format!(
                "the cell root {cell_root:?} overlaps the device-set literal {target:?}"
            )));
        }
    }
    for target in path_denial_control_targets() {
        if overlaps(cell_root, &target) {
            return Err(refusal(format!(
                "the cell root {cell_root:?} overlaps the denial-control target {target:?}"
            )));
        }
    }

    if helper == "/" || overlaps(helper, cell_root) {
        return Err(refusal(format!(
            "the helper path {helper:?} is `/` or overlaps the cell root {cell_root:?}"
        )));
    }

    Ok(ValidatedInputs {
        cell_root: cell_root.clone(),
        payload_root: payload_root.clone(),
        helper: helper.clone(),
    })
}

fn validate_canonical(name: &str, value: &str) -> Result<(), CheckRefusal> {
    if !value.starts_with('/') {
        return Err(refusal(format!("the {name} {value:?} is not absolute")));
    }
    if value.len() > 1 && value.ends_with('/') {
        return Err(refusal(format!("the {name} {value:?} has a trailing `/`")));
    }
    if value != "/" {
        for component in value.split('/').skip(1) {
            if component.is_empty() || component == "." || component == ".." {
                return Err(refusal(format!(
                    "the {name} {value:?} has an empty, `.` or `..` component"
                )));
            }
        }
    }
    for top in ["/var", "/tmp", "/etc"] {
        if value == top || value.starts_with(&format!("{top}/")) {
            return Err(refusal(format!(
                "the {name} {value:?} is spelled through the top-level symlink {top:?}"
            )));
        }
    }
    Ok(())
}

fn overlaps(left: &str, right: &str) -> bool {
    left == right || is_under(left, right) || is_under(right, left)
}

fn is_under(path: &str, ancestor: &str) -> bool {
    path.len() > ancestor.len()
        && path.starts_with(ancestor)
        && (ancestor.ends_with('/') || path.as_bytes()[ancestor.len()] == b'/')
}

/// The host-independent map from a host path to its `/private` spelling where
/// macOS resolves one. The first component of `/etc/ssl` is `etc`, so its set
/// also holds `/private/etc/ssl`; `/usr/bin` has only `/usr/bin`.
fn private_spelling(path: &str) -> Option<String> {
    let first = path.trim_start_matches('/').split('/').next().unwrap_or("");
    matches!(first, "etc" | "var" | "tmp").then(|| format!("/private{path}"))
}

fn spelling_set(source: &str) -> Vec<String> {
    let mut set = vec![source.to_string()];
    if let Some(private) = private_spelling(source) {
        set.push(private);
    }
    set
}

/// The non-direct resolved spellings of a host-toolchain source: its
/// `/private` spelling where one exists and its `/System/Volumes/Data`
/// spelling. A denial event on one of these is a respelling, never a grant.
pub fn toolchain_respelled_spellings(source: &str) -> Vec<String> {
    let mut spellings = Vec::new();
    if let Some(private) = private_spelling(source) {
        spellings.push(private);
    }
    spellings.push(format!("/System/Volumes/Data{source}"));
    if source.starts_with("/etc") {
        spellings.push(format!("/System/Volumes/Data/private{source}"));
    }
    spellings
}

fn system_library_targets() -> Vec<String> {
    vec![
        "/System/Library".to_string(),
        "/System/Volumes/Preboot/Cryptexes/OS".to_string(),
    ]
}

fn device_set_literals() -> Vec<String> {
    vec![
        "/dev/null".to_string(),
        "/dev/urandom".to_string(),
        "/dev/random".to_string(),
    ]
}

/// The path-valued denial-control targets, in direct and `/private` spelling
/// where one exists, plus the data-volume control.
pub fn path_denial_control_targets() -> Vec<String> {
    let mut targets = Vec::new();
    for target in CREDENTIAL_READ_DENIAL_TARGETS {
        targets.push(target.to_string());
        if let Some(private) = private_spelling(target) {
            targets.push(private);
        }
    }
    targets.push(HOST_WRITE_DENIAL_TARGET.to_string());
    targets.push(DATA_VOLUME_CREDENTIAL_READ_DENIAL_TARGET.to_string());
    targets
}

/// Normalize a concrete unit: a string equal to a validated path, or beginning
/// with it and then `/`, takes its placeholder, longest path first.
fn normalize_unit(unit: &RuleUnit, validated: &ValidatedInputs) -> RuleUnit {
    let mut filter = unit.filter.clone();
    if let Some(filter) = filter.as_mut() {
        if !filter.is_self() {
            let mut paths = [
                (validated.helper.as_str(), PLACEHOLDER_HELPER),
                (validated.payload_root.as_str(), PLACEHOLDER_PAYLOAD_ROOT),
                (validated.cell_root.as_str(), PLACEHOLDER_CELL_ROOT),
            ];
            paths.sort_by_key(|(path, _)| std::cmp::Reverse(path.len()));
            for (path, placeholder) in paths {
                if filter.target == path {
                    filter.target = placeholder.to_string();
                    break;
                }
                if let Some(rest) = filter.target.strip_prefix(&format!("{path}/")) {
                    filter.target = format!("{placeholder}/{rest}");
                    break;
                }
            }
        }
    }
    RuleUnit {
        operation: unit.operation.clone(),
        filter,
    }
}

/// Put validated concrete values back for a placeholder target, so the cover,
/// data-volume and control-target rules can read a unit's concrete form.
fn concretize_unit(unit: &RuleUnit, validated: &ValidatedInputs) -> RuleUnit {
    let mut filter = unit.filter.clone();
    if let Some(filter) = filter.as_mut() {
        if !filter.is_self() {
            filter.target = filter
                .target
                .replace(PLACEHOLDER_PAYLOAD_ROOT, &validated.payload_root)
                .replace(PLACEHOLDER_CELL_ROOT, &validated.cell_root)
                .replace(PLACEHOLDER_HELPER, &validated.helper);
        }
    }
    RuleUnit {
        operation: unit.operation.clone(),
        filter,
    }
}

// ---------------------------------------------------------------------------
// The check
// ---------------------------------------------------------------------------

/// Validate the concrete inputs and prove the rendered profile carries exactly
/// the ledger's baseline entries plus the removal set, with every anchor and
/// cross-cutting rule holding in normalized and concrete form.
pub fn check_startup_candidate(
    profile: &str,
    ledger: &[LedgerEntry],
    removal_set: &[StartupNegativeAllowance],
    inputs: &CheckInputs,
) -> Result<(), CheckRefusal> {
    // Stage 1: validate the concrete inputs before any string is normalized.
    let validated = validate_inputs(inputs)?;

    // Stage 2: parse the closed grammar and normalize longest-validated-path
    // first.
    let parsed = parse_template(profile)?;
    let parsed_units: Vec<RuleUnit> = parsed
        .units
        .iter()
        .map(|unit| normalize_unit(unit, &validated))
        .collect();

    // Stage 3: judge the ledger and the equality.
    let baseline_units: Vec<RuleUnit> = ledger
        .iter()
        .filter(|entry| matches!(entry.class, LedgerClass::Baseline(_)))
        .map(|entry| entry.unit.clone())
        .collect();
    let diagnosis_units: Vec<RuleUnit> = ledger
        .iter()
        .filter(|entry| matches!(entry.class, LedgerClass::DiagnosisAdmitted(_)))
        .map(|entry| entry.unit.clone())
        .collect();
    let mut removal_units = Vec::new();
    for removal in removal_set {
        let parsed = parse_template(&format!(
            "(version 1)\n(deny default)\n{}\n",
            removal.removed_rule
        ))
        .map_err(|error| {
            refusal(format!(
                "removal entry {} does not name one rule unit: {}",
                removal.name, error.reason
            ))
        })?;
        if parsed.units.len() != 1 {
            return Err(refusal(format!(
                "removal entry {} does not name exactly one rule unit",
                removal.name
            )));
        }
        removal_units.push(parsed.units[0].clone());
    }

    // Every baseline entry carries a typed, non-empty justification of a named
    // kind and passes its anchor.
    for entry in ledger {
        if let LedgerClass::Baseline(baseline) = &entry.class {
            if baseline.justification.trim().is_empty() {
                return Err(refusal(format!(
                    "baseline unit {} carries no justification",
                    entry.unit.render()
                )));
            }
            if let Some(filter) = &entry.unit.filter {
                if !(filter.is_literal() || filter.is_subpath()) {
                    return Err(refusal(format!(
                        "baseline unit {} uses a filter that is not `literal` or `subpath`",
                        entry.unit.render()
                    )));
                }
            }
            check_baseline_anchor(entry)?;
        }
    }

    // Every diagnosis-admitted entry carries complete evidence, names exactly
    // one removal entry, uses a single-object filter, and matches it.
    for entry in ledger {
        if let LedgerClass::DiagnosisAdmitted(admission) = &entry.class {
            if admission.process.trim().is_empty()
                || admission.consumer.trim().is_empty()
                || admission.evidence.trim().is_empty()
                || admission.removal.trim().is_empty()
            {
                return Err(refusal(format!(
                    "diagnosis-admitted unit {} lacks its operation, target, process, consumer, \
                     evidence or removal entry",
                    entry.unit.render()
                )));
            }
            match &entry.unit.filter {
                Some(filter) if filter.is_single_object() => {}
                _ => {
                    return Err(refusal(format!(
                        "diagnosis-admitted unit {} does not use a single-object filter",
                        entry.unit.render()
                    )))
                }
            }
            let matching = removal_set
                .iter()
                .position(|removal| removal.name == admission.removal);
            match matching {
                Some(index) if removal_units[index] == entry.unit => {}
                Some(_) => {
                    return Err(refusal(format!(
                        "diagnosis-admitted unit {} does not equal its removal entry {}",
                        entry.unit.render(),
                        admission.removal
                    )))
                }
                None => {
                    return Err(refusal(format!(
                        "diagnosis-admitted unit {} names no removal entry",
                        entry.unit.render()
                    )))
                }
            }
        }
    }

    // A removal entry whose unit is not diagnosis-admitted fails.
    for (unit, removal) in removal_units.iter().zip(removal_set) {
        if !diagnosis_units.contains(unit) {
            return Err(refusal(format!(
                "removal entry {} is not a diagnosis-admitted unit",
                removal.name
            )));
        }
    }

    // The candidate's units equal the disjoint union of the baseline entries
    // and the removal set.
    for unit in &baseline_units {
        if removal_units.contains(unit) {
            return Err(refusal(format!(
                "unit {} is in both halves of the ledger",
                unit.render()
            )));
        }
    }
    let mut expected = baseline_units.clone();
    expected.extend(removal_units.iter().cloned());
    for unit in &parsed_units {
        if !expected.contains(unit) {
            return Err(refusal(format!(
                "the template carries a unit that is in neither half: {}",
                unit.render()
            )));
        }
    }
    for unit in &expected {
        if !parsed_units.contains(unit) {
            return Err(refusal(format!(
                "the template lacks the ledger unit {}",
                unit.render()
            )));
        }
    }

    // Cross-cutting rules, over every unit, in normalized and concrete form.
    let toolchain_units: Vec<RuleUnit> = ledger
        .iter()
        .filter(|entry| {
            matches!(
                &entry.class,
                LedgerClass::Baseline(Baseline {
                    kind: BaselineKind::HandsElement(HandsElement::Toolchain),
                    ..
                })
            )
        })
        .map(|entry| entry.unit.clone())
        .collect();
    for unit in &parsed_units {
        let normalized = unit.clone();
        let concrete = concretize_unit(unit, &validated);
        let is_toolchain = toolchain_units.contains(&normalized);
        check_process_rule(&normalized, &validated)?;
        check_process_rule(&concrete, &validated)?;
        check_cross_cutting(&normalized, is_toolchain)?;
        check_cross_cutting(&concrete, is_toolchain)?;
    }
    // The ledger itself is judged too, so a varied ledger cannot smuggle a
    // unit the renderer never emits.
    for entry in ledger {
        let normalized = entry.unit.clone();
        let concrete = concretize_unit(&entry.unit, &validated);
        let is_toolchain = toolchain_units.contains(&normalized);
        check_process_rule(&normalized, &validated)?;
        check_process_rule(&concrete, &validated)?;
        check_cross_cutting(&normalized, is_toolchain)?;
        check_cross_cutting(&concrete, is_toolchain)?;
    }

    Ok(())
}

/// The candidate's process authority is exactly the seven named process units.
fn check_process_rule(unit: &RuleUnit, validated: &ValidatedInputs) -> Result<(), CheckRefusal> {
    if !unit.is_process_unit() {
        return Ok(());
    }
    let named = match (&unit.operation[..], &unit.filter) {
        ("process-fork", None) => true,
        ("process-exec", Some(filter))
            if filter.is_literal()
                && (filter.target == PLACEHOLDER_HELPER || filter.target == validated.helper) =>
        {
            true
        }
        ("process-exec", Some(filter))
            if filter.is_subpath() && PROGRAM_BINDS.contains(&filter.target.as_str()) =>
        {
            true
        }
        _ => false,
    };
    if named {
        Ok(())
    } else {
        Err(refusal(format!(
            "the unit {} is a process unit outside the seven named process units",
            unit.render()
        )))
    }
}

/// Three rules hold for every unit of either half, in normalized and concrete
/// form: no non-toolchain unit covers a host-toolchain spelling, no unit
/// touches the data volume, and no unit covers a denial-control target.
fn check_cross_cutting(unit: &RuleUnit, is_toolchain: bool) -> Result<(), CheckRefusal> {
    let target = unit.filter.as_ref().map(|filter| filter.target.as_str());

    // The data-volume rule reads every unit, toolchain included.
    if let Some(target) = target {
        let data_volume = "/System/Volumes/Data";
        if target == data_volume || is_under(target, data_volume) {
            return Err(refusal(format!(
                "the unit {} targets `/System/Volumes/Data` or a path under it",
                unit.render()
            )));
        }
        if unit
            .filter
            .as_ref()
            .is_some_and(|filter| filter.is_subpath() && is_under(data_volume, target))
        {
            return Err(refusal(format!(
                "the subpath unit {} contains `/System/Volumes/Data`",
                unit.render()
            )));
        }
    }

    // A non-toolchain unit may not cover any spelling of a host-toolchain
    // source.
    if !is_toolchain && target.is_some() {
        for source in HOST_TOOLCHAIN_BINDS {
            for spelling in spelling_set(source) {
                if covers(unit, &spelling) {
                    return Err(refusal(format!(
                        "the unit {} covers the host-toolchain source {source:?} at its \
                         spelling {spelling:?}",
                        unit.render()
                    )));
                }
            }
        }
    }

    // No unit covers a path-valued denial-control target.
    for denial in path_denial_control_targets() {
        if covers(unit, &denial) {
            return Err(refusal(format!(
                "the unit {} covers the denial-control target {denial:?}",
                unit.render()
            )));
        }
    }
    Ok(())
}

/// Does `unit`'s target cover `path`? A `literal` equals it; a `subpath` equals
/// it or contains it. `self` and unfiltered units cover no path.
fn covers(unit: &RuleUnit, path: &str) -> bool {
    match &unit.filter {
        Some(filter) if filter.is_literal() => filter.target == path,
        Some(filter) if filter.is_subpath() => {
            filter.target == path || is_under(path, &filter.target)
        }
        _ => false,
    }
}

/// A baseline entry's two-part anchor: its operation is one the element or kind
/// admits, matched by exact name, and its target passes that element's or
/// kind's target test.
fn check_baseline_anchor(entry: &LedgerEntry) -> Result<(), CheckRefusal> {
    let baseline = match &entry.class {
        LedgerClass::Baseline(baseline) => baseline,
        LedgerClass::DiagnosisAdmitted(_) => return Ok(()),
    };
    let unit = &entry.unit;
    let target = unit.filter.as_ref().map(|filter| filter.target.as_str());
    let fail = |element: &str, detail: String| {
        refusal(format!(
            "the baseline unit {} fails its {element} anchor: {detail}",
            unit.render()
        ))
    };

    match &baseline.kind {
        BaselineKind::HandsElement(HandsElement::Shell) => {
            if unit.operation == "process-fork" && unit.filter.is_none() {
                Ok(())
            } else {
                Err(fail(
                    "shell",
                    "the shell element is (allow process-fork) alone".to_string(),
                ))
            }
        }
        BaselineKind::HandsElement(HandsElement::Toolchain) => {
            let Some(filter) = &unit.filter else {
                return Err(fail(
                    "toolchain",
                    "a toolchain unit needs a target".to_string(),
                ));
            };
            if !(filter.is_literal() || filter.is_subpath()) {
                return Err(fail("toolchain", "the target is not a path".to_string()));
            }
            if !HOST_TOOLCHAIN_BINDS.contains(&filter.target.as_str()) {
                return Err(fail(
                    "toolchain",
                    format!(
                        "the named bind {:?} is not in HOST_TOOLCHAIN_BINDS",
                        filter.target
                    ),
                ));
            }
            match unit.operation.as_str() {
                "file-read*" => Ok(()),
                "process-exec" => {
                    if PROGRAM_BINDS.contains(&filter.target.as_str()) {
                        Ok(())
                    } else {
                        Err(fail(
                            "toolchain",
                            format!(
                                "process-exec is only admitted for a program bind; {:?} is not \
                                 one",
                                filter.target
                            ),
                        ))
                    }
                }
                other => Err(fail(
                    "toolchain",
                    format!("the operation {other} is not admitted (file-read* or process-exec)"),
                )),
            }
        }
        BaselineKind::HandsElement(HandsElement::SystemLibrary) => {
            if unit.operation != "file-read*" {
                return Err(fail(
                    "system library",
                    format!("the operation {} is not file-read*", unit.operation),
                ));
            }
            let Some(target) = target else {
                return Err(fail("system library", "no target".to_string()));
            };
            if system_library_targets()
                .iter()
                .any(|committed| committed == target)
            {
                return Ok(());
            }
            match &baseline.correction {
                Some(correction) if correction.resolved == target => {
                    if correction.replaces.trim().is_empty()
                        || correction.evidence.trim().is_empty()
                    {
                        return Err(fail(
                            "system library",
                            "the correction lacks the unit it replaces or its evidence".to_string(),
                        ));
                    }
                    for historical in FA7_HISTORICAL_TARGETS {
                        if overlaps(target, historical) {
                            return Err(fail(
                                "system library",
                                format!(
                                    "the correction equals or contains a withdrawn or narrowed \
                                     fa7 target {historical:?}"
                                ),
                            ));
                        }
                    }
                    Ok(())
                }
                _ => Err(fail(
                    "system library",
                    format!(
                        "the target {target} is neither a committed target nor a recorded \
                         correction"
                    ),
                )),
            }
        }
        BaselineKind::HandsElement(HandsElement::WritableWorktree) => {
            if unit.operation == "file-write*"
                && unit.filter.as_ref().is_some_and(|filter| {
                    filter.is_subpath() && filter.target == PLACEHOLDER_PAYLOAD_ROOT
                })
            {
                Ok(())
            } else {
                Err(fail(
                    "writable worktree",
                    "the only baseline write is file-write* on exactly <payload-root>".to_string(),
                ))
            }
        }
        BaselineKind::HandsElement(HandsElement::DeviceSet) => {
            if unit.operation == "file-read*"
                && unit.filter.as_ref().is_some_and(|filter| {
                    filter.is_literal()
                        && device_set_literals()
                            .iter()
                            .any(|device| device == &filter.target)
                })
            {
                Ok(())
            } else {
                Err(fail(
                    "device set",
                    "a device-set unit is file-read* on exactly one device literal".to_string(),
                ))
            }
        }
        BaselineKind::ExecutionInput => {
            if matches!(unit.operation.as_str(), "file-read*" | "process-exec")
                && unit.filter.as_ref().is_some_and(|filter| {
                    filter.is_literal() && filter.target == PLACEHOLDER_HELPER
                })
            {
                Ok(())
            } else {
                Err(fail(
                    "execution input",
                    "an execution input is file-read* or process-exec on exactly (literal \
                     \"<helper>\")"
                        .to_string(),
                ))
            }
        }
        BaselineKind::ProbeHarnessNeed => {
            if unit.operation == "file-read*"
                && unit.filter.as_ref().is_some_and(|filter| {
                    filter.is_subpath()
                        && (filter.target == "<cell-root>/inputs"
                            || filter.target == PLACEHOLDER_PAYLOAD_ROOT)
                })
            {
                Ok(())
            } else {
                Err(fail(
                    "probe-harness need",
                    "a probe-harness need is file-read* on exactly <cell-root>/inputs or \
                     <payload-root>"
                        .to_string(),
                ))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The candidate renderer
// ---------------------------------------------------------------------------

/// Render the candidate profile: exactly the two-form frame, then one `allow`
/// form per ledger entry in ledger order, with the validated concrete values
/// substituted for the placeholders.
pub fn render_candidate_profile(ledger: &[LedgerEntry], inputs: &CheckInputs) -> String {
    let concrete = CheckInputs {
        cell_root: inputs.cell_root.clone(),
        payload_root: inputs.payload_root.clone(),
        inputs_dir: inputs.inputs_dir.clone(),
        helper: inputs.helper.clone(),
    };
    let mut profile = String::from("(version 1)\n(deny default)\n");
    for entry in ledger {
        profile.push_str(&render_concrete_unit(&entry.unit, &concrete));
        profile.push('\n');
    }
    profile
}

fn render_concrete_unit(unit: &RuleUnit, inputs: &CheckInputs) -> String {
    let mut unit = unit.clone();
    if let Some(filter) = unit.filter.as_mut() {
        if !filter.is_self() {
            filter.target = filter
                .target
                .replace(PLACEHOLDER_PAYLOAD_ROOT, &inputs.payload_root)
                .replace(PLACEHOLDER_CELL_ROOT, &inputs.cell_root)
                .replace(PLACEHOLDER_HELPER, &inputs.helper);
        }
    }
    unit.render()
}
