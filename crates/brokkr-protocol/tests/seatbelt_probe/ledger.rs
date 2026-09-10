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

/// A refused check, carrying the rule and the input or unit it found. Each
/// variant is a named specification refusal, so a falsification test can match
/// it; [`std::fmt::Display`] renders the human reason. The check collects
/// these in a fixed order instead of stopping at the first one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckRefusal {
    /// Stage 1: a concrete input failed one validation rule before any rewrite.
    Input {
        rule: &'static str,
        input: String,
        value: String,
        detail: String,
    },
    /// Stage 2: the frame is not exactly `(version 1)` then `(deny default)`.
    Frame { detail: String },
    /// Stage 2: a top-level form other than the frame and an `allow` form.
    TopLevelForm { form: String },
    /// Stage 2: an `allow` form names more than one operation.
    MultiOperation { operations: Vec<String> },
    /// Stage 2: an action modifier such as `(with report)`.
    Modifier { modifier: String },
    /// Stage 2: a compound `require-any`/`require-all`/`require-not` filter.
    CompoundFilter { filter: String },
    /// Stage 2: a filter that does not carry exactly one string or `self`.
    FilterArity { filter: String },
    /// Stage 2: an `allow default` form.
    AllowDefault,
    /// Stage 2: text the closed grammar cannot normalize.
    Grammar { detail: String },
    /// Stage 2: the same unit rendered twice.
    DuplicateUnit { unit: String },
    /// A baseline entry without a typed justification or with a wide filter.
    BaselineRecord { unit: String, detail: String },
    /// A baseline entry's operation fails its element's or kind's anchor.
    AnchorOperation {
        kind: &'static str,
        unit: String,
        operation: String,
        detail: String,
    },
    /// A baseline entry's target fails its element's or kind's anchor.
    AnchorTarget {
        kind: &'static str,
        unit: String,
        target: String,
        detail: String,
    },
    /// A `process-*` unit of either half outside the seven named process units.
    ProcessUnit { unit: String },
    /// A listed program bind that is not a `HOST_TOOLCHAIN_BINDS` source.
    ProgramBindNotToolchain { bind: String },
    /// A non-toolchain unit covering a host-toolchain source spelling.
    ToolchainCover {
        unit: String,
        source: String,
        spelling: String,
    },
    /// A unit targeting or containing `/System/Volumes/Data`.
    DataVolume { unit: String, detail: String },
    /// A unit covering a path-valued denial-control target.
    DenialControlCover { unit: String, target: String },
    /// A unit in neither half or in both halves of the ledger.
    Classification { unit: String, detail: String },
    /// A ledger or removal entry the rendered template lacks.
    MissingUnit { unit: String, detail: String },
    /// A diagnosis-admitted entry lacking evidence or naming a wide filter.
    DiagnosisRecord { unit: String, detail: String },
    /// A removal entry whose unit is not diagnosis-admitted.
    RemovalNotDiagnosis { name: String },
    /// A system-library target that is neither committed nor a bounded
    /// correction of a committed one.
    SystemLibrary { unit: String, detail: String },
    /// A baseline unit that is unfiltered and is not `process-fork`.
    UnfilteredUnit { unit: String },
}

impl std::fmt::Display for CheckRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckRefusal::Input {
                rule,
                input,
                value,
                detail,
            } => write!(f, "the {input} {value:?} {detail} [{rule}]"),
            CheckRefusal::Frame { detail } => write!(f, "{detail}"),
            CheckRefusal::TopLevelForm { form } => write!(
                f,
                "the template holds a top-level form other than the frame and allow: '{form}'"
            ),
            CheckRefusal::MultiOperation { operations } => write!(
                f,
                "the allow form names more than one operation: {}; after the first: {}",
                operations.join(", "),
                operations
                    .get(1..)
                    .map(|rest| rest.join(", "))
                    .unwrap_or_default()
            ),
            CheckRefusal::Modifier { modifier } => {
                write!(
                    f,
                    "the template holds an action modifier: (with {modifier})"
                )
            }
            CheckRefusal::CompoundFilter { filter } => {
                write!(f, "the template holds a compound filter: {filter}")
            }
            CheckRefusal::FilterArity { filter } => write!(
                f,
                "the filter {filter} does not carry exactly one string or self argument"
            ),
            CheckRefusal::AllowDefault => write!(f, "the template holds an allow default form"),
            CheckRefusal::Grammar { detail } => write!(f, "{detail}"),
            CheckRefusal::DuplicateUnit { unit } => {
                write!(f, "the template renders the same unit twice: {unit}")
            }
            CheckRefusal::BaselineRecord { unit, detail } => {
                write!(f, "the baseline unit {unit} {detail}")
            }
            CheckRefusal::AnchorOperation {
                kind,
                unit,
                operation,
                detail,
            } => write!(
                f,
                "the unit {unit} fails its {kind} anchor on the operation {operation}: {detail}"
            ),
            CheckRefusal::AnchorTarget {
                kind,
                unit,
                target,
                detail,
            } => write!(
                f,
                "the unit {unit} fails its {kind} anchor on the target {target:?}: {detail}"
            ),
            CheckRefusal::ProcessUnit { unit } => write!(
                f,
                "the unit {unit} is a process unit outside the seven named process units"
            ),
            CheckRefusal::ProgramBindNotToolchain { bind } => write!(
                f,
                "the program bind {bind:?} is not a HOST_TOOLCHAIN_BINDS source"
            ),
            CheckRefusal::ToolchainCover {
                unit,
                source,
                spelling,
            } => write!(
                f,
                "the unit {unit} covers the host-toolchain source {source:?} at its spelling \
                 {spelling:?}"
            ),
            CheckRefusal::DataVolume { unit, detail } => write!(f, "the unit {unit} {detail}"),
            CheckRefusal::DenialControlCover { unit, target } => write!(
                f,
                "the unit {unit} covers the denial-control target {target:?}"
            ),
            CheckRefusal::Classification { unit, detail } => write!(f, "the unit {unit} {detail}"),
            CheckRefusal::MissingUnit { unit, detail } => {
                write!(f, "the entry {unit} {detail}")
            }
            CheckRefusal::DiagnosisRecord { unit, detail } => {
                write!(f, "the diagnosis-admitted unit {unit} {detail}")
            }
            CheckRefusal::RemovalNotDiagnosis { name } => {
                write!(f, "removal entry {name} is not a diagnosis-admitted unit")
            }
            CheckRefusal::SystemLibrary { unit, detail } => {
                write!(f, "the system-library unit {unit} {detail}")
            }
            CheckRefusal::UnfilteredUnit { unit } => write!(
                f,
                "the unit {unit} is unfiltered and is not (allow process-fork)"
            ),
        }
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
    CheckRefusal::Grammar {
        detail: reason.into(),
    }
}

fn input_refusal(
    rule: &'static str,
    input: impl Into<String>,
    value: &str,
    detail: impl Into<String>,
) -> CheckRefusal {
    CheckRefusal::Input {
        rule,
        input: input.into(),
        value: value.to_string(),
        detail: detail.into(),
    }
}

/// Render an ordered refusal list as one human string. The check returns every
/// refusal it finds, so the report and the native cell name all of them.
pub fn render_refusals(refusals: &[CheckRefusal]) -> String {
    refusals
        .iter()
        .map(CheckRefusal::to_string)
        .collect::<Vec<_>>()
        .join("; ")
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

/// One withdrawn or narrowed fa7 unit that a restoration diagnostic restores
/// alone, in its fa7 form, on top of the exact candidate. Restoration is
/// evidence only: these units are exactly the ones the disposition table
/// withdrew or narrowed, so none is a ledger entry and the check refuses any
/// profile that carries one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fa7Restoration {
    pub unit: RuleUnit,
    pub reason: &'static str,
}

/// Every withdrawn or narrowed fa7 unit, in disposition order. The seven
/// one-class differentials are kept separate and are not part of this set.
pub static FA7_RESTORATIONS: LazyLock<Vec<Fa7Restoration>> = LazyLock::new(|| {
    FA7_RECORDS
        .iter()
        .filter(|record| {
            matches!(
                record.disposition,
                Fa7Disposition::Withdrawn | Fa7Disposition::Narrowed
            )
        })
        .map(|record| Fa7Restoration {
            unit: record.unit.clone(),
            reason: record.reason,
        })
        .collect()
});

/// Render a restoration diagnostic: the exact candidate plus the named fa7
/// units in their fa7 form, with the validated concrete cell root, payload root
/// and helper substituted for their placeholders. It is diagnostic text only,
/// never a candidate; the caller records it and the check refuses any profile
/// that carries a restored unit because none is a ledger entry.
pub fn render_restoration_profile(
    candidate: &str,
    restored: &[RuleUnit],
    inputs: &CheckInputs,
) -> String {
    let mut text = String::from(
        ";; RESTORATION DIAGNOSTIC: never a passing candidate and never a ledger entry\n",
    );
    text.push_str(candidate);
    if !text.ends_with('\n') {
        text.push('\n');
    }
    for unit in restored {
        text.push_str(&render_concrete_unit(unit, inputs));
        text.push('\n');
    }
    text
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
        return Err(CheckRefusal::Frame {
            detail: "the template is missing its two-form frame".to_string(),
        });
    }
    match &forms[0] {
        Sexp::List(items)
            if items.len() == 2
                && items[0].atom() == Some("version")
                && items[1].atom() == Some("1") => {}
        other => {
            return Err(CheckRefusal::Frame {
                detail: format!("the template does not open with exactly (version 1): {other:?}"),
            })
        }
    }
    match &forms[1] {
        Sexp::List(items)
            if items.len() == 2
                && items[0].atom() == Some("deny")
                && items[1].atom() == Some("default") => {}
        other => {
            return Err(CheckRefusal::Frame {
                detail: format!(
                    "the template does not open with exactly (deny default): {other:?}"
                ),
            })
        }
    }

    let mut units: Vec<RuleUnit> = Vec::new();
    for form in &forms[2..] {
        let items = match form {
            Sexp::List(items) => items,
            Sexp::Atom(atom) => return Err(CheckRefusal::TopLevelForm { form: atom.clone() }),
            Sexp::Str(_) => {
                return Err(CheckRefusal::TopLevelForm {
                    form: "<string>".to_string(),
                })
            }
        };
        let head = items
            .first()
            .and_then(Sexp::atom)
            .ok_or_else(|| refusal("the template holds an empty or non-symbol top-level form"))?;
        if head == "version" {
            return Err(CheckRefusal::TopLevelForm {
                form: "version".to_string(),
            });
        }
        if head == "deny" {
            return Err(CheckRefusal::TopLevelForm {
                form: "deny".to_string(),
            });
        }
        if head != "allow" {
            return Err(CheckRefusal::TopLevelForm {
                form: head.to_string(),
            });
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
            return Err(CheckRefusal::MultiOperation {
                operations: operations.iter().map(|op| (*op).to_string()).collect(),
            });
        }
        let operation = operations[0];
        if operation == "default" {
            return Err(CheckRefusal::AllowDefault);
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
                return Err(CheckRefusal::Modifier {
                    modifier: list
                        .get(1)
                        .and_then(Sexp::atom)
                        .unwrap_or("...")
                        .to_string(),
                });
            }
            if matches!(name, "require-any" | "require-all" | "require-not") {
                return Err(CheckRefusal::CompoundFilter {
                    filter: name.to_string(),
                });
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
                    return Err(CheckRefusal::FilterArity {
                        filter: name.to_string(),
                    })
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
            return Err(CheckRefusal::DuplicateUnit {
                unit: unit.render(),
            });
        }
    }

    Ok(ParsedTemplate { units })
}

// ---------------------------------------------------------------------------
// Input validation and normalization
// ---------------------------------------------------------------------------

/// Validate every concrete input in the specification's rule order, collecting
/// all refusals before any rewrite. The caller stops stage 1 on a non-empty
/// list, so no unit is ever normalized from an invalid value.
fn validate_inputs(inputs: &CheckInputs) -> Result<ValidatedInputs, Vec<CheckRefusal>> {
    let cell_root = &inputs.cell_root;
    let payload_root = &inputs.payload_root;
    let inputs_dir = &inputs.inputs_dir;
    let helper = &inputs.helper;
    let mut refusals: Vec<CheckRefusal> = Vec::new();

    if payload_root != &format!("{cell_root}/payload") {
        refusals.push(input_refusal(
            "layout",
            "payload root",
            payload_root,
            "is not the cell root joined with `payload`",
        ));
    }
    if inputs_dir != &format!("{cell_root}/inputs") {
        refusals.push(input_refusal(
            "layout",
            "inputs directory",
            inputs_dir,
            "is not the cell root joined with `inputs`",
        ));
    }
    if let Err(refusal) = validate_canonical("cell root", cell_root) {
        refusals.push(refusal);
    }
    if let Err(refusal) = validate_canonical("helper", helper) {
        refusals.push(refusal);
    }

    if cell_root == "/" {
        refusals.push(input_refusal("root", "cell root", cell_root, "is `/`"));
    }
    if overlaps(cell_root, "/System/Volumes/Data") {
        refusals.push(input_refusal(
            "data-volume",
            "cell root",
            cell_root,
            "is, lies under or contains `/System/Volumes/Data`",
        ));
    }

    // The cell root may not overlap any spelling of a host-toolchain source, a
    // committed system-library target, a device-set literal or a path-valued
    // denial-control target. The refusal names the source as well as the
    // spelling it matched.
    for source in HOST_TOOLCHAIN_BINDS {
        for spelling in spelling_set(source) {
            if overlaps(cell_root, &spelling) {
                refusals.push(input_refusal(
                    "toolchain-overlap",
                    "cell root",
                    cell_root,
                    format!(
                        "overlaps the host-toolchain source {source:?} at its spelling \
                         {spelling:?}"
                    ),
                ));
            }
        }
    }
    for target in system_library_targets() {
        if overlaps(cell_root, &target) {
            refusals.push(input_refusal(
                "system-library-overlap",
                "cell root",
                cell_root,
                format!("overlaps the system-library target {target:?}"),
            ));
        }
    }
    for target in device_set_literals() {
        if overlaps(cell_root, &target) {
            refusals.push(input_refusal(
                "device-overlap",
                "cell root",
                cell_root,
                format!("overlaps the device-set literal {target:?}"),
            ));
        }
    }
    for target in path_denial_control_targets() {
        if overlaps(cell_root, &target) {
            refusals.push(input_refusal(
                "control-overlap",
                "cell root",
                cell_root,
                format!("overlaps the denial-control target {target:?}"),
            ));
        }
    }

    if helper == "/" || overlaps(helper, cell_root) {
        refusals.push(input_refusal(
            "helper-disjoint",
            "helper path",
            helper,
            format!("is `/` or overlaps the cell root {cell_root:?}"),
        ));
    }

    if !refusals.is_empty() {
        return Err(refusals);
    }
    Ok(ValidatedInputs {
        cell_root: cell_root.clone(),
        payload_root: payload_root.clone(),
        helper: helper.clone(),
    })
}

fn validate_canonical(name: &str, value: &str) -> Result<(), CheckRefusal> {
    if !value.starts_with('/') {
        return Err(input_refusal(
            "canonical",
            name.to_string(),
            value,
            "is not absolute",
        ));
    }
    if value.len() > 1 && value.ends_with('/') {
        return Err(input_refusal(
            "canonical",
            name.to_string(),
            value,
            "has a trailing `/`",
        ));
    }
    if value != "/" {
        for component in value.split('/').skip(1) {
            if component.is_empty() || component == "." || component == ".." {
                return Err(input_refusal(
                    "canonical",
                    name.to_string(),
                    value,
                    "has an empty, `.` or `..` component",
                ));
            }
        }
    }
    for top in ["/var", "/tmp", "/etc"] {
        if value == top || value.starts_with(&format!("{top}/")) {
            return Err(input_refusal(
                "canonical",
                name.to_string(),
                value,
                format!("is spelled through the top-level symlink {top:?}"),
            ));
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

/// Confirm every listed program bind is a host-toolchain source. The pure
/// sub-function takes both lists as parameters, so a falsification test can
/// exercise the refusal the real five-entry constant can never trigger.
pub fn confirm_program_binds(
    program_binds: &[&str],
    host_toolchain: &[&str],
) -> Result<(), CheckRefusal> {
    for bind in program_binds {
        if !host_toolchain.contains(bind) {
            return Err(CheckRefusal::ProgramBindNotToolchain {
                bind: (*bind).to_string(),
            });
        }
    }
    Ok(())
}

/// Validate the concrete inputs and prove the rendered profile carries exactly
/// the ledger's baseline entries plus the removal set, with every anchor and
/// cross-cutting rule holding in normalized and concrete form. The check is a
/// pure function; it collects every refusal in a fixed order instead of
/// stopping at the first, so a unit that fails both its anchor and the process
/// rule is named under both.
pub fn check_startup_candidate(
    profile: &str,
    ledger: &[LedgerEntry],
    removal_set: &[StartupNegativeAllowance],
    inputs: &CheckInputs,
) -> Result<(), Vec<CheckRefusal>> {
    // Stage 1: validate the concrete inputs before any string is normalized.
    let validated = validate_inputs(inputs)?;

    // Stage 2: parse the closed grammar and normalize longest-validated-path
    // first.
    let parsed = parse_template(profile).map_err(|refusal| vec![refusal])?;
    let parsed_units: Vec<RuleUnit> = parsed
        .units
        .iter()
        .map(|unit| normalize_unit(unit, &validated))
        .collect();

    // Stage 3: judge each unit in a fixed order. Within one unit the baseline
    // record and its two-part anchor come first, then the process rule, then
    // the cross-cutting rules.
    let mut refusals: Vec<CheckRefusal> = Vec::new();

    if let Err(refusal) = confirm_program_binds(&PROGRAM_BINDS, HOST_TOOLCHAIN_BINDS) {
        refusals.push(refusal);
    }

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
        match parse_template(&format!(
            "(version 1)\n(deny default)\n{}\n",
            removal.removed_rule
        )) {
            Ok(parsed) if parsed.units.len() == 1 => removal_units.push(parsed.units[0].clone()),
            Ok(_) => refusals.push(CheckRefusal::MissingUnit {
                unit: removal.removed_rule.to_string(),
                detail: format!(
                    "removal entry {} does not name exactly one rule unit",
                    removal.name
                ),
            }),
            Err(error) => refusals.push(CheckRefusal::MissingUnit {
                unit: removal.removed_rule.to_string(),
                detail: format!(
                    "removal entry {} does not name one rule unit: {error}",
                    removal.name
                ),
            }),
        }
    }

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

    // Units rendered by the template, in template order. Each is judged once.
    for unit in &parsed_units {
        let normalized = unit.clone();
        let concrete = concretize_unit(unit, &validated);
        let is_toolchain = toolchain_units.contains(&normalized);
        if let Some(entry) = ledger.iter().find(|entry| &entry.unit == unit) {
            judge_ledger_entry(entry, removal_set, &removal_units, &mut refusals);
        }
        judge_unit(&normalized, &validated, is_toolchain, &mut refusals);
        // The concrete form is read by the cover, data-volume and
        // control-target rules only.
        check_cross_cutting(&concrete, is_toolchain, &mut refusals);
    }

    // Ledger entries the template does not render are judged too, in ledger
    // order, so a varied ledger cannot smuggle a unit the renderer never emits.
    for entry in ledger {
        if !parsed_units.contains(&entry.unit) {
            judge_ledger_entry(entry, removal_set, &removal_units, &mut refusals);
            let is_toolchain = toolchain_units.contains(&entry.unit);
            judge_unit(&entry.unit, &validated, is_toolchain, &mut refusals);
        }
    }

    // A removal entry whose unit is not diagnosis-admitted fails.
    for (unit, removal) in removal_units.iter().zip(removal_set) {
        if !diagnosis_units.contains(unit) {
            refusals.push(CheckRefusal::RemovalNotDiagnosis {
                name: removal.name.to_string(),
            });
        }
    }

    // The candidate's units equal the disjoint union of the baseline entries
    // and the removal set.
    for unit in &baseline_units {
        if removal_units.contains(unit) {
            refusals.push(CheckRefusal::Classification {
                unit: unit.render(),
                detail: "is in both halves of the ledger".to_string(),
            });
        }
    }
    let mut expected = baseline_units.clone();
    expected.extend(removal_units.iter().cloned());
    for unit in &parsed_units {
        if !expected.contains(unit) {
            refusals.push(CheckRefusal::Classification {
                unit: unit.render(),
                detail: "is in neither half of the ledger".to_string(),
            });
        }
    }
    // Every required unit the template lacks is reported once, whichever half
    // it belongs to.
    let mut required = expected;
    for entry in ledger {
        if !required.contains(&entry.unit) {
            required.push(entry.unit.clone());
        }
    }
    for unit in &required {
        if !parsed_units.contains(unit) {
            refusals.push(CheckRefusal::MissingUnit {
                unit: unit.render(),
                detail: "is a ledger or removal entry the template lacks".to_string(),
            });
        }
    }

    if refusals.is_empty() {
        Ok(())
    } else {
        Err(refusals)
    }
}

/// Judge one ledger entry's record and, for a baseline entry, its two-part
/// anchor. Every refusal is pushed in the specification's order.
fn judge_ledger_entry(
    entry: &LedgerEntry,
    removal_set: &[StartupNegativeAllowance],
    removal_units: &[RuleUnit],
    refusals: &mut Vec<CheckRefusal>,
) {
    match &entry.class {
        LedgerClass::Baseline(baseline) => {
            if baseline.justification.trim().is_empty() {
                refusals.push(CheckRefusal::BaselineRecord {
                    unit: entry.unit.render(),
                    detail: "carries no justification".to_string(),
                });
            }
            if let Some(filter) = &entry.unit.filter {
                if !(filter.is_literal() || filter.is_subpath()) {
                    refusals.push(CheckRefusal::BaselineRecord {
                        unit: entry.unit.render(),
                        detail: "uses a filter that is not `literal` or `subpath`".to_string(),
                    });
                }
            }
            check_baseline_anchor(entry, refusals);
        }
        LedgerClass::DiagnosisAdmitted(admission) => {
            if admission.process.trim().is_empty()
                || admission.consumer.trim().is_empty()
                || admission.evidence.trim().is_empty()
                || admission.removal.trim().is_empty()
            {
                refusals.push(CheckRefusal::DiagnosisRecord {
                    unit: entry.unit.render(),
                    detail: "lacks its operation, target, process, consumer, evidence or removal \
                             entry"
                        .to_string(),
                });
            }
            match &entry.unit.filter {
                Some(filter) if filter.is_single_object() => {}
                _ => refusals.push(CheckRefusal::DiagnosisRecord {
                    unit: entry.unit.render(),
                    detail: "does not use a single-object filter".to_string(),
                }),
            }
            let matching = removal_set
                .iter()
                .position(|removal| removal.name == admission.removal);
            match matching {
                Some(index) if removal_units.get(index) == Some(&entry.unit) => {}
                Some(_) => refusals.push(CheckRefusal::DiagnosisRecord {
                    unit: entry.unit.render(),
                    detail: format!("does not equal its removal entry {}", admission.removal),
                }),
                None => refusals.push(CheckRefusal::DiagnosisRecord {
                    unit: entry.unit.render(),
                    detail: "names no removal entry".to_string(),
                }),
            }
        }
    }
}

/// Judge one normalized unit: the unfiltered rule, the process rule and the
/// normalized cross-cutting rules, in that order.
fn judge_unit(
    unit: &RuleUnit,
    validated: &ValidatedInputs,
    is_toolchain: bool,
    refusals: &mut Vec<CheckRefusal>,
) {
    if unit.filter.is_none() && unit.operation != "process-fork" {
        refusals.push(CheckRefusal::UnfilteredUnit {
            unit: unit.render(),
        });
    }
    check_process_rule(unit, validated, refusals);
    check_cross_cutting(unit, is_toolchain, refusals);
}

/// The candidate's process authority is exactly the seven named process units.
fn check_process_rule(
    unit: &RuleUnit,
    validated: &ValidatedInputs,
    refusals: &mut Vec<CheckRefusal>,
) {
    if !unit.is_process_unit() {
        return;
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
    if !named {
        refusals.push(CheckRefusal::ProcessUnit {
            unit: unit.render(),
        });
    }
}

/// Three rules hold for every unit of either half, in normalized and concrete
/// form: no non-toolchain unit covers a host-toolchain spelling, no unit
/// touches the data volume, and no unit covers a denial-control target.
fn check_cross_cutting(unit: &RuleUnit, is_toolchain: bool, refusals: &mut Vec<CheckRefusal>) {
    let target = unit.filter.as_ref().map(|filter| filter.target.as_str());

    // The data-volume rule reads every unit, toolchain included.
    if let Some(target) = target {
        let data_volume = "/System/Volumes/Data";
        if target == data_volume || is_under(target, data_volume) {
            refusals.push(CheckRefusal::DataVolume {
                unit: unit.render(),
                detail: "targets `/System/Volumes/Data` or a path under it".to_string(),
            });
        }
        if unit
            .filter
            .as_ref()
            .is_some_and(|filter| filter.is_subpath() && is_under(data_volume, target))
        {
            refusals.push(CheckRefusal::DataVolume {
                unit: unit.render(),
                detail: "contains `/System/Volumes/Data`".to_string(),
            });
        }
    }

    // A non-toolchain unit may not cover any spelling of a host-toolchain
    // source.
    if !is_toolchain && target.is_some() {
        for source in HOST_TOOLCHAIN_BINDS {
            for spelling in spelling_set(source) {
                if covers(unit, &spelling) {
                    refusals.push(CheckRefusal::ToolchainCover {
                        unit: unit.render(),
                        source: (*source).to_string(),
                        spelling,
                    });
                }
            }
        }
    }

    // No unit covers a path-valued denial-control target.
    for denial in path_denial_control_targets() {
        if covers(unit, &denial) {
            refusals.push(CheckRefusal::DenialControlCover {
                unit: unit.render(),
                target: denial,
            });
        }
    }
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

fn anchor_operation(
    kind: &'static str,
    unit: &RuleUnit,
    operation: &str,
    detail: &str,
) -> CheckRefusal {
    CheckRefusal::AnchorOperation {
        kind,
        unit: unit.render(),
        operation: operation.to_string(),
        detail: detail.to_string(),
    }
}

fn anchor_target(kind: &'static str, unit: &RuleUnit, target: &str, detail: &str) -> CheckRefusal {
    CheckRefusal::AnchorTarget {
        kind,
        unit: unit.render(),
        target: target.to_string(),
        detail: detail.to_string(),
    }
}

/// A baseline entry's two-part anchor: its operation is one the element or kind
/// admits, matched by exact name, and its target passes that element's or
/// kind's target test.
fn check_baseline_anchor(entry: &LedgerEntry, refusals: &mut Vec<CheckRefusal>) {
    let baseline = match &entry.class {
        LedgerClass::Baseline(baseline) => baseline,
        LedgerClass::DiagnosisAdmitted(_) => return,
    };
    let unit = &entry.unit;
    let target = unit.filter.as_ref().map(|filter| filter.target.as_str());

    match &baseline.kind {
        BaselineKind::HandsElement(HandsElement::Shell) => {
            if !(unit.operation == "process-fork" && unit.filter.is_none()) {
                refusals.push(anchor_operation(
                    "shell",
                    unit,
                    &unit.operation,
                    "the shell element is (allow process-fork) alone",
                ));
            }
        }
        BaselineKind::HandsElement(HandsElement::Toolchain) => {
            let Some(filter) = &unit.filter else {
                refusals.push(anchor_target(
                    "toolchain",
                    unit,
                    "",
                    "a toolchain unit needs a target",
                ));
                return;
            };
            if !(filter.is_literal() || filter.is_subpath()) {
                refusals.push(anchor_target(
                    "toolchain",
                    unit,
                    &filter.target,
                    "the target is not a path",
                ));
                return;
            }
            if !HOST_TOOLCHAIN_BINDS.contains(&filter.target.as_str()) {
                refusals.push(anchor_target(
                    "toolchain",
                    unit,
                    &filter.target,
                    &format!(
                        "the named bind {:?} is not in HOST_TOOLCHAIN_BINDS",
                        filter.target
                    ),
                ));
                return;
            }
            match unit.operation.as_str() {
                "file-read*" => {}
                "process-exec" => {
                    if !PROGRAM_BINDS.contains(&filter.target.as_str()) {
                        refusals.push(anchor_operation(
                            "toolchain",
                            unit,
                            &unit.operation,
                            &format!(
                                "process-exec is only admitted for a program bind; {:?} is not \
                                 one",
                                filter.target
                            ),
                        ));
                    }
                }
                other => refusals.push(anchor_operation(
                    "toolchain",
                    unit,
                    other,
                    "the operation is not admitted (file-read* or process-exec)",
                )),
            }
        }
        BaselineKind::HandsElement(HandsElement::SystemLibrary) => {
            if unit.operation != "file-read*" {
                refusals.push(anchor_operation(
                    "system library",
                    unit,
                    &unit.operation,
                    "the operation is not file-read*",
                ));
                return;
            }
            let Some(target) = target else {
                refusals.push(anchor_target("system library", unit, "", "no target"));
                return;
            };
            if system_library_targets()
                .iter()
                .any(|committed| committed == target)
            {
                return;
            }
            match &baseline.correction {
                Some(correction) if correction.resolved == target => {
                    if !system_library_targets()
                        .iter()
                        .any(|committed| committed == &correction.replaces)
                    {
                        refusals.push(CheckRefusal::SystemLibrary {
                            unit: unit.render(),
                            detail: format!(
                                "records a correction that replaces {:?}, which is not a \
                                 committed system-library target",
                                correction.replaces
                            ),
                        });
                        return;
                    }
                    if correction.replaces.trim().is_empty()
                        || correction.evidence.trim().is_empty()
                    {
                        refusals.push(CheckRefusal::SystemLibrary {
                            unit: unit.render(),
                            detail: "has a correction that lacks the unit it replaces or its \
                                     evidence"
                                .to_string(),
                        });
                        return;
                    }
                    for historical in FA7_HISTORICAL_TARGETS {
                        if overlaps(target, historical) {
                            refusals.push(CheckRefusal::SystemLibrary {
                                unit: unit.render(),
                                detail: format!(
                                    "has a correction that equals or contains a withdrawn or \
                                     narrowed fa7 target {historical:?}"
                                ),
                            });
                            return;
                        }
                    }
                }
                _ => refusals.push(CheckRefusal::SystemLibrary {
                    unit: unit.render(),
                    detail: format!(
                        "targets {target}, which is neither a committed target nor a recorded \
                         correction"
                    ),
                }),
            }
        }
        BaselineKind::HandsElement(HandsElement::WritableWorktree) => {
            let exact = unit.operation == "file-write*"
                && unit.filter.as_ref().is_some_and(|filter| {
                    filter.is_subpath() && filter.target == PLACEHOLDER_PAYLOAD_ROOT
                });
            if !exact {
                refusals.push(anchor_operation(
                    "writable worktree",
                    unit,
                    &unit.operation,
                    "the only baseline write is file-write* on exactly <payload-root>",
                ));
            }
        }
        BaselineKind::HandsElement(HandsElement::DeviceSet) => {
            if unit.operation != "file-read*" {
                refusals.push(anchor_operation(
                    "device set",
                    unit,
                    &unit.operation,
                    "a device-set unit is file-read* on exactly one device literal",
                ));
            } else {
                let exact = unit.filter.as_ref().is_some_and(|filter| {
                    filter.is_literal()
                        && device_set_literals()
                            .iter()
                            .any(|device| device == &filter.target)
                });
                if !exact {
                    refusals.push(anchor_target(
                        "device set",
                        unit,
                        target.unwrap_or(""),
                        "a device-set unit is file-read* on exactly one device literal",
                    ));
                }
            }
        }
        BaselineKind::ExecutionInput => {
            if !matches!(unit.operation.as_str(), "file-read*" | "process-exec") {
                refusals.push(anchor_operation(
                    "execution input",
                    unit,
                    &unit.operation,
                    "an execution input is file-read* or process-exec on exactly (literal \
                     \"<helper>\")",
                ));
                return;
            }
            let exact = unit
                .filter
                .as_ref()
                .is_some_and(|filter| filter.is_literal() && filter.target == PLACEHOLDER_HELPER);
            if !exact {
                refusals.push(anchor_target(
                    "execution input",
                    unit,
                    target.unwrap_or(""),
                    "an execution input is file-read* or process-exec on exactly (literal \
                     \"<helper>\")",
                ));
            }
        }
        BaselineKind::ProbeHarnessNeed => {
            if unit.operation != "file-read*" {
                refusals.push(anchor_operation(
                    "probe-harness need",
                    unit,
                    &unit.operation,
                    "a probe-harness need is file-read* on exactly <cell-root>/inputs or \
                     <payload-root>",
                ));
                return;
            }
            let exact = unit.filter.as_ref().is_some_and(|filter| {
                filter.is_subpath()
                    && (filter.target == "<cell-root>/inputs"
                        || filter.target == PLACEHOLDER_PAYLOAD_ROOT)
            });
            if !exact {
                refusals.push(anchor_target(
                    "probe-harness need",
                    unit,
                    target.unwrap_or(""),
                    "a probe-harness need is file-read* on exactly <cell-root>/inputs or \
                     <payload-root>",
                ));
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

pub fn render_concrete_unit(unit: &RuleUnit, inputs: &CheckInputs) -> String {
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
