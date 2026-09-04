//! Realm-owned specification dialects (decision 0042, first enactment slice).
//! The file is data: this module parses its closed vocabulary and proves that
//! its artifact map is complete and respects the framework's dependency graph.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

pub const ARTIFACT_PHASES: [&str; 3] = ["specify", "design", "tasks"];
pub const DIALECT_PHASES: [&str; 5] = ["specify", "design", "tasks", "clarify", "analyze"];

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

impl Phases {
    pub fn artifact(&self, phase: &str) -> Option<&ArtifactPhase> {
        match phase {
            "specify" => Some(&self.specify),
            "design" => Some(&self.design),
            "tasks" => Some(&self.tasks),
            _ => None,
        }
    }

    pub fn loop_phase(&self, phase: &str) -> Option<&LoopPhase> {
        match phase {
            "clarify" => Some(&self.clarify),
            "analyze" => Some(&self.analyze),
            _ => None,
        }
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
    pub archive: CommandOrUnsupported,
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

    pub fn parse(path: &str, text: &str) -> Result<(Self, Value), DialectError> {
        let value: Value = serde_json::from_str(text).map_err(|error| DialectError::Malformed {
            path: path.to_string(),
            detail: error.to_string(),
        })?;
        let dialect: Dialect =
            serde_json::from_value(value.clone()).map_err(|error| DialectError::Malformed {
                path: path.to_string(),
                detail: error.to_string(),
            })?;
        dialect.check(path)?;
        Ok((dialect, value))
    }

    fn check(&self, path: &str) -> Result<(), DialectError> {
        let invalid = |problem| DialectError::Invalid {
            path: path.to_string(),
            problem,
        };
        if self.schema != "brokkr.dialect/v1" {
            return Err(invalid(format!(
                "it calls itself '{}'; this build reads brokkr.dialect/v1",
                self.schema
            )));
        }
        if self.name.trim().is_empty()
            || self.tool.binary.trim().is_empty()
            || self.tool.version.trim().is_empty()
        {
            return Err(invalid(
                "name, tool binary and measured version must be non-empty".into(),
            ));
        }
        let mut artifact_position = BTreeMap::new();
        let mut assigned = BTreeSet::new();
        let mut position = 0usize;
        for phase_name in ARTIFACT_PHASES {
            let phase = self.phases.artifact(phase_name).expect("closed phase name");
            if phase.steps.is_empty() {
                return Err(invalid(format!("phase '{phase_name}' is unfilled")));
            }
            if !phase.steps.iter().any(|step| !step.optional) {
                return Err(invalid(format!(
                    "phase '{phase_name}' has no required step"
                )));
            }
            for step in &phase.steps {
                if step.name.trim().is_empty() || step.artifacts.is_empty() {
                    return Err(invalid(format!(
                        "phase '{phase_name}' has an empty step or artifact list"
                    )));
                }
                for artifact in &step.artifacts {
                    if !assigned.insert(artifact.clone()) {
                        return Err(invalid(format!(
                            "artifact '{artifact}' is assigned more than once"
                        )));
                    }
                    artifact_position.insert(artifact.clone(), position);
                    position += 1;
                }
            }
        }
        for edge in &self.order {
            let before = artifact_position.get(&edge.before).ok_or_else(|| {
                invalid(format!("order names unassigned artifact '{}'", edge.before))
            })?;
            let after = artifact_position.get(&edge.after).ok_or_else(|| {
                invalid(format!("order names unassigned artifact '{}'", edge.after))
            })?;
            if before >= after {
                return Err(invalid(format!(
                    "artifact '{}' is mapped before its dependency '{}'",
                    edge.after, edge.before
                )));
            }
        }
        for phase in ["clarify", "analyze"] {
            let loop_phase = self.phases.loop_phase(phase).expect("closed phase name");
            if loop_phase.taxonomy.trim().is_empty() {
                return Err(invalid(format!("phase '{phase}' has no taxonomy")));
            }
        }
        for instruction in self.instruction_paths() {
            let candidate = Path::new(instruction);
            if candidate.is_absolute()
                || instruction
                    .split(['/', '\\'])
                    .any(|component| component == "..")
            {
                return Err(invalid(format!(
                    "instruction path '{instruction}' must be relative and remain beside its dialect"
                )));
            }
        }
        check_command_tokens(self, path)?;
        Ok(())
    }

    fn instruction_paths(&self) -> Vec<&str> {
        ARTIFACT_PHASES
            .iter()
            .flat_map(|name| {
                self.phases
                    .artifact(name)
                    .expect("closed artifact phase")
                    .steps
                    .iter()
                    .flat_map(|step| {
                        [
                            step.instructions.as_str(),
                            step.return_instructions.as_str(),
                        ]
                    })
            })
            .chain([
                self.phases.clarify.taxonomy.as_str(),
                self.phases.analyze.taxonomy.as_str(),
            ])
            .collect()
    }

    pub fn validation(&self, phase: &str) -> Option<&Command> {
        match phase {
            "specify" | "design" | "tasks" => match &self.phases.artifact(phase)?.validate {
                CommandOrUnsupported::Command(command) => Some(command),
                CommandOrUnsupported::Unsupported(_) => None,
            },
            "clarify" | "analyze" => match &self.phases.loop_phase(phase)?.check {
                CommandOrUnsupported::Command(command) => Some(command),
                CommandOrUnsupported::Unsupported(_) => None,
            },
            _ => None,
        }
    }

    /// Read the dialect-owned prose which is spliced into a model prompt.
    /// Paths are relative to the dialect file's own directory; the caller
    /// supplies that directory so a library name and realm path have exactly
    /// the same pinning semantics.
    pub fn prompt_for(&self, root: &Path, phase: &str) -> Result<String, DialectError> {
        let paths: Vec<&str> = match phase {
            "specify" | "design" | "tasks" => self
                .phases
                .artifact(phase)
                .expect("closed artifact phase")
                .steps
                .iter()
                .flat_map(|step| {
                    [
                        step.instructions.as_str(),
                        step.return_instructions.as_str(),
                    ]
                })
                .collect(),
            "clarify" | "analyze" => vec![self
                .phases
                .loop_phase(phase)
                .expect("closed loop phase")
                .taxonomy
                .as_str()],
            "implement" => self
                .phases
                .tasks
                .steps
                .iter()
                .flat_map(|step| {
                    [
                        step.instructions.as_str(),
                        step.return_instructions.as_str(),
                    ]
                })
                .collect(),
            "review" => ARTIFACT_PHASES
                .iter()
                .flat_map(|name| {
                    self.phases
                        .artifact(name)
                        .expect("closed artifact phase")
                        .steps
                        .iter()
                        .map(|step| step.instructions.as_str())
                })
                .collect(),
            _ => Vec::new(),
        };
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
        if phase == "implement" {
            let archive = match &self.archive {
                CommandOrUnsupported::Command(command) => {
                    serde_json::to_string(&command.argv).unwrap_or_default()
                }
                CommandOrUnsupported::Unsupported(reason) => {
                    format!("unsupported: {}", reason.unsupported)
                }
            };
            rendered.push(format!(
                "Change location: `{}`. Archive operation: {archive}.",
                self.change
            ));
        }
        Ok(rendered.join("\n\n"))
    }

    pub fn render(&mut self, root: &Path) -> Result<(), DialectError> {
        let mut rendered = BTreeMap::new();
        for phase in DIALECT_PHASES.into_iter().chain(["implement", "review"]) {
            let prompt = self.prompt_for(root, phase)?;
            rendered.insert(phase.to_string(), prompt);
        }
        self.rendered = rendered;
        Ok(())
    }
}

fn check_command_tokens(dialect: &Dialect, path: &str) -> Result<(), DialectError> {
    let commands = ARTIFACT_PHASES
        .iter()
        .filter_map(|phase| dialect.validation(phase))
        .chain(
            ["clarify", "analyze"]
                .iter()
                .filter_map(|phase| dialect.validation(phase)),
        )
        .chain(match &dialect.verify {
            CommandOrUnsupported::Command(c) => Some(c),
            _ => None,
        })
        .chain(match &dialect.archive {
            CommandOrUnsupported::Command(c) => Some(c),
            _ => None,
        });
    for command in commands {
        for token in command.argv.iter().chain(command.state.iter().flatten()) {
            let stripped = token.replace("{change}", "");
            if stripped.contains('{') || stripped.contains('}') {
                return Err(DialectError::Invalid {
                    path: path.to_string(),
                    problem: format!("argv token '{token}' uses an unknown placeholder"),
                });
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

#[cfg(test)]
mod tests;
