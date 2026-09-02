//! The transcript law shared by every built-in driver (decision 0032).
//!
//! A harness-specific arm supplies only its locator. This module owns the
//! closed kind vocabulary, harness-home resolution, the 80-character
//! locator clamp, the `session_meta.transcript` shape, and the checkpoint
//! row that puts that shape in the journal. It never reads transcript
//! content and never removes a transcript or its directory.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

/// The existing checkpoint target bound also governs a transcript locator.
const LOCATOR_LIMIT: usize = 80;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    ClaudeSession,
    CodexThread,
    DshSession,
    None,
}

impl Kind {
    fn label(self) -> &'static str {
        match self {
            Kind::ClaudeSession => "claude-session",
            Kind::CodexThread => "codex-thread",
            Kind::DshSession => "dsh-session",
            Kind::None => "none",
        }
    }
}

/// One invocation's transcript identity. The locator may arrive after the
/// harness starts, so the value is mutable while the home and kind are not.
pub(crate) struct Transcript {
    kind: Kind,
    home: PathBuf,
    locator: String,
    journaled: bool,
}

impl Transcript {
    /// Resolve the home through the same environment the child inherits.
    pub(crate) fn resolve(kind: Kind) -> Result<Self, String> {
        let operator_home = std::env::var_os("HOME").or(std::env::var_os("USERPROFILE"));
        resolved(
            kind,
            match kind {
                Kind::ClaudeSession => claude_home_from(operator_home),
                Kind::CodexThread => codex_home_from(std::env::var_os("CODEX_HOME"), operator_home),
                Kind::DshSession => dsh_home(),
                Kind::None => Some(PathBuf::new()),
            },
        )
    }

    /// The resolved home is also where dsh stages its retained seat root.
    pub(crate) fn home(&self) -> &Path {
        &self.home
    }

    /// Record a harness locator. Repeated reports replace the earlier one:
    /// a retry or a harness final event may reveal a newer authoritative id.
    pub(crate) fn record(
        &mut self,
        locator: &str,
        session_meta: &mut Map<String, Value>,
        emit: &mut impl FnMut(&Value),
    ) {
        self.locator = locator.chars().take(LOCATOR_LIMIT).collect();
        self.publish(session_meta, emit);
    }

    /// Every invocation reports the row, including `none` and a harness
    /// which failed to announce its id. An empty locator is an explicit
    /// absence inside the common shape, never an invented path.
    pub(crate) fn finish(
        &mut self,
        session_meta: &mut Map<String, Value>,
        emit: &mut impl FnMut(&Value),
    ) {
        if !self.journaled {
            self.publish(session_meta, emit);
        }
    }

    /// A path locator is relative to its separately recorded harness home.
    /// This keeps the address complete even when the operator's absolute
    /// home is longer than the locator clamp.
    pub(crate) fn locator_under_home(&self, path: &Path) -> Result<String, String> {
        path.strip_prefix(&self.home)
            .map(|relative| relative.to_string_lossy().replace('\\', "/"))
            .map_err(|_| {
                format!(
                    "transcript path {:?} is not under harness home {:?}",
                    path, self.home
                )
            })
    }

    fn at(kind: Kind, home: PathBuf) -> Self {
        Self {
            kind,
            home,
            locator: String::new(),
            journaled: false,
        }
    }

    fn value(&self) -> Value {
        json!({
            "kind": self.kind.label(),
            "locator": self.locator,
            "home": self.home.to_string_lossy(),
        })
    }

    fn publish(&mut self, session_meta: &mut Map<String, Value>, emit: &mut impl FnMut(&Value)) {
        let transcript = self.value();
        session_meta.insert("transcript".into(), transcript.clone());
        emit(&json!({"step": "transcript", "transcript": transcript}));
        self.journaled = true;
    }
}

fn resolved(kind: Kind, home: Option<PathBuf>) -> Result<Transcript, String> {
    match home {
        Some(home) => Ok(Transcript::at(kind, home)),
        None => Err(format!(
            "no harness home for transcript kind {}",
            kind.label()
        )),
    }
}

fn claude_home_from(home: Option<OsString>) -> Option<PathBuf> {
    home.map(|home| PathBuf::from(home).join(".claude").join("projects"))
}

fn codex_home_from(codex_home: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    match codex_home {
        Some(explicit) if !explicit.is_empty() => Some(explicit.into()),
        _ => home.map(|home| PathBuf::from(home).join(".codex")),
    }
}

/// The operator's dsh home: `$DSH_HOME` when set and non-empty, else
/// `~/.dsh` — the resolution the harness itself uses for its sessions.
pub(crate) fn dsh_home() -> Option<PathBuf> {
    dsh_home_from(
        std::env::var_os("DSH_HOME"),
        // Eager on purpose: a lazy fallback is a function no Unix test
        // can reach, and the gate counts functions.
        std::env::var_os("HOME").or(std::env::var_os("USERPROFILE")),
    )
}

/// `dsh_home` over its two inputs, so every branch is a plain test.
pub(crate) fn dsh_home_from(dsh_home: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    match dsh_home {
        Some(explicit) if !explicit.is_empty() => Some(explicit.into()),
        _ => home.map(|home| PathBuf::from(home).join(".dsh")),
    }
}

/// One seat's own directory under `<harness home>/sessions/brokkr`:
/// unique per invocation, and kept when the handle is released.
pub(crate) fn dsh_transcript_root_under(home: Option<PathBuf>) -> std::io::Result<PathBuf> {
    let home = home.ok_or_else(|| {
        std::io::Error::other("no dsh home to keep the transcript under: set DSH_HOME or HOME")
    })?;
    let base = home.join("sessions").join("brokkr");
    std::fs::create_dir_all(&base)?;
    Ok(tempfile::Builder::new()
        .prefix("seat-")
        .tempdir_in(&base)?
        .keep())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn homes_follow_each_harness_and_empty_overrides_fall_back() {
        assert_eq!(
            claude_home_from(Some(OsString::from("/home/operator"))),
            Some(PathBuf::from("/home/operator/.claude/projects"))
        );
        assert_eq!(claude_home_from(None), None);
        assert_eq!(
            codex_home_from(
                Some(OsString::from("/var/codex")),
                Some(OsString::from("/home/operator"))
            ),
            Some(PathBuf::from("/var/codex"))
        );
        assert_eq!(
            codex_home_from(
                Some(OsString::new()),
                Some(OsString::from("/home/operator"))
            ),
            Some(PathBuf::from("/home/operator/.codex"))
        );
        assert_eq!(codex_home_from(None, None), None);
        assert_eq!(
            resolved(Kind::None, Some(PathBuf::new())).unwrap().home(),
            Path::new("")
        );
        assert!(resolved(Kind::ClaudeSession, None)
            .err()
            .unwrap()
            .contains("no harness home for transcript kind claude-session"));
    }

    #[test]
    fn one_shape_clamps_locators_and_one_row_carries_no_content() {
        let mut transcript = Transcript::at(Kind::ClaudeSession, PathBuf::from("/h/projects"));
        let mut meta = Map::new();
        let mut emitted = Vec::new();
        let mut capture = |row: &Value| emitted.push(row.clone());
        transcript.record(&"x".repeat(81), &mut meta, &mut capture);
        transcript.finish(&mut meta, &mut capture);
        assert_eq!(emitted.len(), 1, "finish does not duplicate a recorded row");
        assert_eq!(emitted[0]["step"], "transcript");
        assert_eq!(meta["transcript"]["kind"], "claude-session");
        assert_eq!(meta["transcript"]["home"], "/h/projects");
        assert_eq!(meta["transcript"]["locator"].as_str().unwrap().len(), 80);
        assert_eq!(emitted[0]["transcript"], meta["transcript"]);
        assert!(emitted[0].get("content").is_none());
    }

    #[test]
    fn path_locators_are_relative_forward_slashed_and_must_stay_under_home() {
        let transcript = Transcript::at(Kind::DshSession, PathBuf::from("/h/.dsh"));
        assert_eq!(
            transcript
                .locator_under_home(Path::new("/h/.dsh/sessions/brokkr/seat-1"))
                .unwrap(),
            "sessions/brokkr/seat-1"
        );
        assert!(transcript
            .locator_under_home(Path::new("/somewhere/else"))
            .unwrap_err()
            .contains("not under harness home"));
    }

    #[test]
    fn none_and_unannounced_sessions_are_still_reported() {
        for (kind, home, label) in [
            (Kind::CodexThread, PathBuf::from("/codex"), "codex-thread"),
            (Kind::DshSession, PathBuf::from("/dsh"), "dsh-session"),
            (Kind::None, PathBuf::new(), "none"),
        ] {
            let mut transcript = Transcript::at(kind, home);
            let mut meta = Map::new();
            let mut row = Value::Null;
            transcript.finish(&mut meta, &mut |value| row = value.clone());
            assert_eq!(row["transcript"]["kind"], label);
            assert_eq!(row["transcript"]["locator"], "");
        }
    }
}
