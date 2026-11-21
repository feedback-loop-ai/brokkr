//! Realm-owned specification dialects (decision 0042, first enactment slice).
//! The file is data: this module parses its closed vocabulary and proves that
//! its artifact map is complete and respects the framework's dependency graph.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

use crate::phase_name;
use refusal::{refused, DialectRefusal};

/// The five phases a dialect fills, a closed set (#349, ask 5): three
/// draft artifacts and two loop back over them with a taxonomy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DialectPhase {
    Specify,
    Design,
    Tasks,
    Clarify,
    Analyze,
}

impl DialectPhase {
    /// In [`DIALECT_PHASES`]'s order, which is the artifact map's order.
    pub(crate) const ALL: [DialectPhase; 5] = [
        DialectPhase::Specify,
        DialectPhase::Design,
        DialectPhase::Tasks,
        DialectPhase::Clarify,
        DialectPhase::Analyze,
    ];

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            DialectPhase::Specify => "specify",
            DialectPhase::Design => "design",
            DialectPhase::Tasks => "tasks",
            DialectPhase::Clarify => "clarify",
            DialectPhase::Analyze => "analyze",
        }
    }

    pub(crate) fn parse(name: &str) -> Option<DialectPhase> {
        DialectPhase::ALL
            .into_iter()
            .find(|phase| phase.as_str() == name)
    }

    /// A loop phase checks the artifacts against a taxonomy; every other
    /// dialect phase drafts them and is validated.
    pub(crate) const fn is_loop(self) -> bool {
        match self {
            DialectPhase::Specify | DialectPhase::Design | DialectPhase::Tasks => false,
            DialectPhase::Clarify | DialectPhase::Analyze => true,
        }
    }
}

pub const ARTIFACT_PHASES: [&str; 3] = [
    DialectPhase::Specify.as_str(),
    DialectPhase::Design.as_str(),
    DialectPhase::Tasks.as_str(),
];
pub const DIALECT_PHASES: [&str; 5] = [
    DialectPhase::Specify.as_str(),
    DialectPhase::Design.as_str(),
    DialectPhase::Tasks.as_str(),
    DialectPhase::Clarify.as_str(),
    DialectPhase::Analyze.as_str(),
];

/// Every phase whose prompt a dialect renders and a bundle pins: the five
/// it fills, and the two offices that read its prose.
pub(crate) fn rendered_phases() -> impl Iterator<Item = &'static str> {
    DIALECT_PHASES
        .into_iter()
        .chain([phase_name::IMPLEMENT, phase_name::REVIEW])
}

/// Decision 0042's first slice: the closed map, with no install identity
/// and no archive instruction.
pub const SCHEMA_V1: &str = "brokkr.dialect/v1";
/// The addendum of 2026-09-04, landed 2026-09-05: the tool names what
/// installs it.
pub const SCHEMA_V2: &str = "brokkr.dialect/v2";
/// The addendum of 2026-09-06: a dialect that promotes a truth tree names
/// the instruction its archive step carries.
pub const SCHEMA_V3: &str = "brokkr.dialect/v3";

/// Every dialect version a run's PIN may carry. A file on disk is v3 and
/// only v3 (`parse`); a pin is read at the version it was written
/// (`parse_pinned`), because a dialect is not only a file: a run pins its
/// resolved content into the manifest and a resume rehydrates that pin
/// through this module. `realms.json` has declared a dialect since
/// 2026-09-04, so runs have pinned v1 and v2 bodies; a build that read v3
/// everywhere would strand every one of them, and would say so only as a
/// serde variant mismatch. Each version's own shape is enforced below, so
/// an older dialect cannot borrow a younger one's fields.
pub const SCHEMAS: [&str; 3] = [SCHEMA_V1, SCHEMA_V2, SCHEMA_V3];

#[derive(Debug, Error)]
pub enum DialectError {
    #[error("reading dialect {path}: {source}")]
    Unreadable {
        path: String,
        source: std::io::Error,
    },
    #[error("reading dialect instruction {path}: {source}")]
    UnreadableInstruction {
        path: String,
        source: std::io::Error,
    },
    #[error("dialect {path} is malformed: {detail}")]
    Malformed { path: String, detail: String },
    #[error("dialect {path} is not usable: {problem}")]
    Invalid { path: String, problem: String },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tool {
    pub binary: String,
    pub version: String,
    /// Absent in a `brokkr.dialect/v1` file and required from v2 on; the
    /// version gate in `check` holds that boundary, not this default.
    #[serde(default)]
    pub install: Option<Install>,
}

/// What installs the tool, because the binary's name is not its
/// identity (decision 0042's addendum of 2026-09-04).
///
/// Measured the same day: OpenSpec's binary is `openspec`, but the bare
/// npm name `openspec` is a placeholder at version 0.0.0 and the tool is
/// published as `@fission-ai/openspec`. An install by binary name gets
/// the wrong package and says nothing. Spec-kit is not a registry
/// package at all: `specify` is `specify-cli`, installed by uv from a
/// git tag. So a dialect names the manager, the package, and the source
/// where the manager's default registry is not where it comes from.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Install {
    pub manager: Manager,
    pub package: String,
    #[serde(default)]
    pub source: Option<String>,
}

/// The installers a dialect may name. Closed, like every other
/// vocabulary here: an unknown manager is a refusal, never a shell
/// command this engine guesses at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Manager {
    Npm,
    Uv,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unsupported {
    pub unsupported: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum PathOrUnsupported {
    Path(String),
    Unsupported(Unsupported),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub argv: Vec<String>,
    #[serde(default)]
    pub state: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum CommandOrUnsupported {
    Command(Command),
    Unsupported(Unsupported),
}

/// The archive step folds a change into standing truth, so it carries
/// the prose that says how (decision 0042's addendum of 2026-09-06): one
/// `## Provenance` line per capability the change touched, appended and
/// never rewritten. A dialect that promotes no truth tree declares
/// `unsupported` instead and needs no instruction.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveCommand {
    pub argv: Vec<String>,
    #[serde(default)]
    pub state: Option<Vec<String>>,
    /// Absent below `brokkr.dialect/v3` and required from v3 on. A v1 or
    /// v2 dialect folds exactly as it did before the addendum, which is
    /// what a resume of a run pinned under it must get.
    #[serde(default)]
    pub instructions: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ArchiveOrUnsupported {
    Command(ArchiveCommand),
    Unsupported(Unsupported),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Office {
    Chief,
    Council,
    Smith,
    Check,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactStep {
    pub name: String,
    pub artifacts: Vec<String>,
    pub office: Office,
    pub optional: bool,
    pub instructions: String,
    pub return_instructions: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactPhase {
    pub steps: Vec<ArtifactStep>,
    pub validate: CommandOrUnsupported,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoopPhase {
    pub taxonomy: String,
    pub check: CommandOrUnsupported,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Phases {
    pub specify: ArtifactPhase,
    pub design: ArtifactPhase,
    pub tasks: ArtifactPhase,
    pub clarify: LoopPhase,
    pub analyze: LoopPhase,
}

impl ArtifactPhase {
    /// Each step's instruction and return instruction, in step order.
    fn prose(&self) -> impl Iterator<Item = &str> {
        self.steps.iter().flat_map(|step| {
            [
                step.instructions.as_str(),
                step.return_instructions.as_str(),
            ]
        })
    }
}

/// What one dialect phase holds.
enum PhaseFill<'a> {
    Artifact(&'a ArtifactPhase),
    Loop(&'a LoopPhase),
}

impl Phases {
    pub fn artifact(&self, phase: &str) -> Option<&ArtifactPhase> {
        self.artifact_of(DialectPhase::parse(phase)?)
    }

    pub fn loop_phase(&self, phase: &str) -> Option<&LoopPhase> {
        self.loop_of(DialectPhase::parse(phase)?)
    }

    fn fill(&self, phase: DialectPhase) -> PhaseFill<'_> {
        match phase {
            DialectPhase::Specify => PhaseFill::Artifact(&self.specify),
            DialectPhase::Design => PhaseFill::Artifact(&self.design),
            DialectPhase::Tasks => PhaseFill::Artifact(&self.tasks),
            DialectPhase::Clarify => PhaseFill::Loop(&self.clarify),
            DialectPhase::Analyze => PhaseFill::Loop(&self.analyze),
        }
    }

    fn artifact_of(&self, phase: DialectPhase) -> Option<&ArtifactPhase> {
        match self.fill(phase) {
            PhaseFill::Artifact(artifact) => Some(artifact),
            PhaseFill::Loop(_) => None,
        }
    }

    fn loop_of(&self, phase: DialectPhase) -> Option<&LoopPhase> {
        match self.fill(phase) {
            PhaseFill::Artifact(_) => None,
            PhaseFill::Loop(loop_phase) => Some(loop_phase),
        }
    }

    /// The artifact phases with what they hold, in [`DialectPhase::ALL`]'s
    /// order.
    fn artifacts(&self) -> impl Iterator<Item = (DialectPhase, &ArtifactPhase)> {
        DialectPhase::ALL
            .into_iter()
            .filter_map(|phase| Some((phase, self.artifact_of(phase)?)))
    }

    fn loops(&self) -> impl Iterator<Item = (DialectPhase, &LoopPhase)> {
        DialectPhase::ALL
            .into_iter()
            .filter_map(|phase| Some((phase, self.loop_of(phase)?)))
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionLocation {
    pub artifact: String,
    pub section: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub before: String,
    pub after: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dialect {
    pub schema: String,
    pub name: String,
    pub tool: Tool,
    pub requires: Vec<String>,
    pub change: String,
    pub truth: PathOrUnsupported,
    pub phases: Phases,
    pub decisions: BTreeMap<String, DecisionLocation>,
    pub order: Vec<Dependency>,
    pub verify: CommandOrUnsupported,
    pub archive: ArchiveOrUnsupported,
    pub house: PathOrUnsupported,
    #[serde(skip)]
    pub rendered: BTreeMap<String, String>,
}

impl Dialect {
    pub fn load(path: &Path) -> Result<(Self, Value), DialectError> {
        let text = std::fs::read_to_string(path).map_err(|source| DialectError::Unreadable {
            path: path.display().to_string(),
            source,
        })?;
        let (mut dialect, value) = Self::parse(&path.display().to_string(), &text)?;
        // Instruction paths belong to the dialect file, not to whichever
        // library or realm happened to name it. Load pins every instruction
        // now; compilation and resume must never recover missing prose from
        // mutable files on disk.
        dialect.render(path.parent().unwrap_or(Path::new("")))?;
        Ok((dialect, value))
    }

    /// A dialect FILE: the newest version only. A file is written now, so
    /// it is written at the version this build writes, and the archive
    /// instruction decision 0042's addendum requires cannot be escaped by
    /// declaring an older version.
    pub fn parse(path: &str, text: &str) -> Result<(Self, Value), DialectError> {
        Self::read(
            path,
            text,
            &[SCHEMA_V3],
            "an older version is read only from a run's own pin",
        )
    }

    /// A dialect rehydrated from a run's PIN: every version this build has
    /// ever written. The pinned content was written on the day the run
    /// started, and a resume must get back the world it pinned, not the
    /// world this build would write today.
    pub fn parse_pinned(path: &str, text: &str) -> Result<(Self, Value), DialectError> {
        Self::read(path, text, &SCHEMAS, "a pinned dialect may be any of them")
    }

    fn read(
        path: &str,
        text: &str,
        accepted: &[&str],
        note: &str,
    ) -> Result<(Self, Value), DialectError> {
        let value: Value = serde_json::from_str(text).map_err(|error| DialectError::Malformed {
            path: path.to_string(),
            detail: error.to_string(),
        })?;
        // The version is read from the bytes BEFORE they are given a
        // shape, because a version this surface does not read must be
        // refused by its version and not by whichever field the newer or
        // older shape happens to trip over. A v2 archive carries no
        // `instructions`, so typed deserialization alone would tell an
        // operator only that some untagged enum matched no variant.
        check_version(&value, accepted, note).map_err(|refusal| refused(path, refusal))?;
        let dialect: Dialect =
            serde_json::from_value(value.clone()).map_err(|error| DialectError::Malformed {
                path: path.to_string(),
                detail: error.to_string(),
            })?;
        dialect.check(path)?;
        Ok((dialect, value))
    }

    /// Judge a parsed dialect against its read version's shape and every
    /// phase rule, one check after another; the first refusal is the one
    /// reported.
    fn check(&self, path: &str) -> Result<(), DialectError> {
        self.first_refusal()
            .map_err(|refusal| refused(path, refusal))
    }

    /// `check`'s seven checks in their order, the first refusal returned.
    fn first_refusal(&self) -> Result<(), DialectRefusal<'_>> {
        self.check_versions()?;
        self.check_identity()?;
        let artifact_position = check_artifacts(&self.phases)?;
        self.check_order(&artifact_position)?;
        self.check_taxonomies()?;
        self.check_instruction_paths()?;
        check_command_tokens(self)
    }

    /// The fields the read version requires or forbids: the tool install,
    /// and the archive instruction.
    fn check_versions(&self) -> Result<(), DialectRefusal<'_>> {
        // Every read version keeps its own shape: an older file may not
        // borrow a field that landed after it, and a file that claims a
        // version must carry what that version requires. Version-tolerant
        // reading is how a pinned world survives (the realm map reads v1
        // through v4 the same way); version-blind reading is not.
        match (self.schema.as_str(), self.tool.install.is_some()) {
            (SCHEMA_V1, true) => return Err(DialectRefusal::InstallTooEarly),
            (SCHEMA_V2 | SCHEMA_V3, false) => {
                let schema = self.schema.as_str();
                return Err(DialectRefusal::Uninstalled { schema });
            }
            _ => {}
        }
        let instructed = match &self.archive {
            ArchiveOrUnsupported::Command(command) => command.instructions.is_some(),
            ArchiveOrUnsupported::Unsupported(_) => false,
        };
        if instructed && self.schema != SCHEMA_V3 {
            let schema = self.schema.as_str();
            return Err(DialectRefusal::InstructionTooEarly { schema });
        }
        if self.schema == SCHEMA_V3 && !instructed {
            if let ArchiveOrUnsupported::Command(_) = &self.archive {
                // Folding without the provenance rule would silently break
                // the two-way trail, so a promoting v3 dialect must say how
                // it records the change that wrote each capability.
                return Err(DialectRefusal::Uninstructed);
            }
        }
        Ok(())
    }

    /// The dialect's name and its tool's identity: non-empty, and a binary
    /// named bare.
    fn check_identity(&self) -> Result<(), DialectRefusal<'_>> {
        if let Some(empty) = self.empty_identity() {
            return Err(empty);
        }
        let binary = self.tool.binary.as_str();
        if binary.contains(['/', '\\', ':']) {
            return Err(DialectRefusal::NotBare { binary });
        }
        Ok(())
    }

    /// The first identity field that trims to empty: name, binary,
    /// version, then a present install's package and present source.
    fn empty_identity(&self) -> Option<DialectRefusal<'_>> {
        let install = self.tool.install.as_ref();
        [
            (Some(&self.name), DialectRefusal::EmptyName),
            (Some(&self.tool.binary), DialectRefusal::EmptyBinary),
            (Some(&self.tool.version), DialectRefusal::EmptyVersion),
            (install.map(|i| &i.package), DialectRefusal::EmptyPackage),
            (
                install.and_then(|i| i.source.as_ref()),
                DialectRefusal::EmptySource,
            ),
        ]
        .into_iter()
        .find_map(|(field, empty)| field.is_some_and(|f| f.trim().is_empty()).then_some(empty))
    }

    /// Every dependency names assigned artifacts, the one it depends on
    /// mapped first.
    fn check_order(
        &self,
        artifact_position: &BTreeMap<String, usize>,
    ) -> Result<(), DialectRefusal<'_>> {
        for edge in &self.order {
            let (before, after) = (edge.before.as_str(), edge.after.as_str());
            let Some(before_at) = artifact_position.get(before) else {
                return Err(DialectRefusal::UnassignedBefore { artifact: before });
            };
            let Some(after_at) = artifact_position.get(after) else {
                return Err(DialectRefusal::UnassignedAfter { artifact: after });
            };
            if before_at >= after_at {
                return Err(DialectRefusal::Reversed { after, before });
            }
        }
        Ok(())
    }

    /// Every loop phase names its taxonomy.
    fn check_taxonomies(&self) -> Result<(), DialectRefusal<'_>> {
        for (phase, loop_phase) in self.phases.loops() {
            if loop_phase.taxonomy.trim().is_empty() {
                let phase = phase.as_str();
                return Err(DialectRefusal::NoTaxonomy { phase });
            }
        }
        Ok(())
    }

    /// Every instruction path spelled relative, beside its dialect.
    fn check_instruction_paths(&self) -> Result<(), DialectRefusal<'_>> {
        for instruction in self.instruction_paths() {
            // A dialect is data that travels between machines, so this
            // boundary is judged by the SPELLING, never by the host.
            // `Path::is_absolute` answers differently on two platforms —
            // "/absolute.md" is absolute on Linux and relative on Windows,
            // which needs a drive prefix — and a boundary that reads two
            // ways is not one. Every spelling below is refused everywhere.
            let first = instruction.split(['/', '\\']).next().unwrap_or_default();
            let drive_letter = first.len() == 2
                && first.ends_with(':')
                && first.starts_with(|c: char| c.is_ascii_alphabetic());
            if instruction.starts_with('/')
                || instruction.starts_with('\\')
                || drive_letter
                || instruction
                    .split(['/', '\\'])
                    .any(|component| component == "..")
            {
                return Err(DialectRefusal::Escapes { instruction });
            }
        }
        Ok(())
    }

    fn instruction_paths(&self) -> Vec<&str> {
        self.phases
            .artifacts()
            .flat_map(|(_, phase)| phase.prose())
            .chain([
                self.phases.clarify.taxonomy.as_str(),
                self.phases.analyze.taxonomy.as_str(),
            ])
            .chain(match &self.archive {
                ArchiveOrUnsupported::Command(command) => command.instructions.as_deref(),
                ArchiveOrUnsupported::Unsupported(_) => None,
            })
            .collect()
    }

    pub fn validation(&self, phase: &str) -> Option<&Command> {
        let gate = match self.phases.fill(DialectPhase::parse(phase)?) {
            PhaseFill::Artifact(artifact) => &artifact.validate,
            PhaseFill::Loop(loop_phase) => &loop_phase.check,
        };
        match gate {
            CommandOrUnsupported::Command(command) => Some(command),
            CommandOrUnsupported::Unsupported(_) => None,
        }
    }

    /// The instruction paths whose prose a phase's prompt splices, in
    /// order: a dialect phase's own, the implementer's tasks prose, and
    /// every artifact's instruction for review.
    fn prompt_paths(&self, phase: &str) -> Vec<&str> {
        match DialectPhase::parse(phase).map(|known| self.phases.fill(known)) {
            Some(PhaseFill::Artifact(artifact)) => artifact.prose().collect(),
            Some(PhaseFill::Loop(loop_phase)) => vec![loop_phase.taxonomy.as_str()],
            None if phase == phase_name::IMPLEMENT => self.phases.tasks.prose().collect(),
            None if phase == phase_name::REVIEW => self
                .phases
                .artifacts()
                .flat_map(|(_, artifact)| artifact.steps.iter())
                .map(|step| step.instructions.as_str())
                .collect(),
            None => Vec::new(),
        }
    }

    /// Read the dialect-owned prose which is spliced into a model prompt.
    /// Paths are relative to the dialect file's own directory; the caller
    /// supplies that directory so a library name and realm path have exactly
    /// the same pinning semantics.
    pub fn prompt_for(&self, root: &Path, phase: &str) -> Result<String, DialectError> {
        let paths = self.prompt_paths(phase);
        let mut rendered = Vec::new();
        for relative in paths {
            let path = root.join(relative);
            let text = std::fs::read_to_string(&path).map_err(|source| {
                DialectError::UnreadableInstruction {
                    path: path.display().to_string(),
                    source,
                }
            })?;
            if !rendered.iter().any(|known| known == text.trim()) {
                rendered.push(text.trim().to_string());
            }
        }
        if phase == phase_name::IMPLEMENT {
            let (archive, instruction) = match &self.archive {
                ArchiveOrUnsupported::Command(command) => (
                    serde_json::to_string(&command.argv).unwrap_or_default(),
                    command.instructions.as_deref(),
                ),
                ArchiveOrUnsupported::Unsupported(reason) => {
                    (format!("unsupported: {}", reason.unsupported), None)
                }
            };
            rendered.push(format!(
                "Change location: `{}`. Archive operation: {archive}.",
                self.change
            ));
            if let Some(relative) = instruction {
                let path = root.join(relative);
                let text = std::fs::read_to_string(&path).map_err(|source| {
                    DialectError::UnreadableInstruction {
                        path: path.display().to_string(),
                        source,
                    }
                })?;
                if !rendered.iter().any(|known| known == text.trim()) {
                    rendered.push(text.trim().to_string());
                }
            }
        }
        Ok(rendered.join("\n\n"))
    }

    pub fn render(&mut self, root: &Path) -> Result<(), DialectError> {
        let mut rendered = BTreeMap::new();
        for phase in rendered_phases() {
            let prompt = self.prompt_for(root, phase)?;
            rendered.insert(phase.to_string(), prompt);
        }
        self.rendered = rendered;
        Ok(())
    }
}

/// The version the bytes call themselves, read before they are given a
/// shape: absent, or one of `accepted`.
fn check_version<'a>(
    value: &'a Value,
    accepted: &'a [&'a str],
    note: &'a str,
) -> Result<(), DialectRefusal<'a>> {
    if let Some(schema) = value.get("schema").and_then(Value::as_str) {
        if !accepted.contains(&schema) {
            return Err(DialectRefusal::Version {
                schema,
                accepted,
                note,
            });
        }
    }
    Ok(())
}

/// Every artifact phase filled with a required step, and every artifact
/// assigned once; the artifacts' positions in map order are returned. A
/// function of the phases, outside `impl Dialect`, so its loops stay within
/// the nesting ceiling (decision 0071 ruling 4).
fn check_artifacts(phases: &Phases) -> Result<BTreeMap<String, usize>, DialectRefusal<'_>> {
    let mut artifact_position = BTreeMap::new();
    let mut assigned = BTreeSet::new();
    let mut position = 0usize;
    for (known, phase) in phases.artifacts() {
        let phase_name = known.as_str();
        if phase.steps.is_empty() {
            return Err(DialectRefusal::Unfilled { phase: phase_name });
        }
        if !phase.steps.iter().any(|step| !step.optional) {
            return Err(DialectRefusal::NoRequiredStep { phase: phase_name });
        }
        for step in &phase.steps {
            if step.name.trim().is_empty() {
                return Err(DialectRefusal::UnnamedStep { phase: phase_name });
            }
            if step.artifacts.is_empty() {
                return Err(DialectRefusal::NoArtifacts { phase: phase_name });
            }
            for artifact in &step.artifacts {
                if !assigned.insert(artifact.as_str()) {
                    return Err(DialectRefusal::AssignedTwice { artifact });
                }
                artifact_position.insert(artifact.clone(), position);
                position += 1;
            }
        }
    }
    Ok(artifact_position)
}

fn check_command_tokens(dialect: &Dialect) -> Result<(), DialectRefusal<'_>> {
    let mut commands: Vec<(&Vec<String>, Option<&Vec<String>>)> = Vec::new();
    for phase in DIALECT_PHASES {
        if let Some(command) = dialect.validation(phase) {
            commands.push((&command.argv, command.state.as_ref()));
        }
    }
    if let CommandOrUnsupported::Command(command) = &dialect.verify {
        commands.push((&command.argv, command.state.as_ref()));
    }
    if let ArchiveOrUnsupported::Command(command) = &dialect.archive {
        commands.push((&command.argv, command.state.as_ref()));
    }
    for (argv, state) in commands {
        for token in argv.iter().chain(state.into_iter().flatten()) {
            let stripped = token.replace("{change}", "");
            if stripped.contains('{') || stripped.contains('}') {
                return Err(DialectRefusal::UnknownPlaceholder { token });
            }
        }
    }
    Ok(())
}

pub fn library_path(root: &Path, declaration: &str, realm_root: &Path) -> PathBuf {
    if declaration.contains(['/', '\\']) || declaration.ends_with(".json") {
        realm_root.join(declaration)
    } else {
        root.join("dialects").join(format!("{declaration}.json"))
    }
}

mod refusal;
#[cfg(test)]
mod tests;
