//! Typed carrier for an adapter's declared `record_kinds.transcript` kind
//! across the engine -> driver `resume_context` boundary (#1227 W6).

use serde_json::{Map, Value};

use crate::transcript::Kind;

/// The JSON key inside `resume_context` where [`DeclaredTranscriptKind`] is
/// written by the engine (`brokkr-runtime`) and read by the driver
/// (`brokkr-protocol`). One home so both sides cannot disagree (#1227 W6).
const DECLARED_TRANSCRIPT_KIND_KEY: &str = "declared_transcript_kind";

/// The JSON key inside `resume_context` where an adapter's declared
/// `record_kinds.root_session` kind is carried for work seats (#1227).
const DECLARED_ROOT_SESSION_KIND_KEY: &str = "declared_root_session_kind";

/// Why `resume_context.declared_transcript_kind` could not be read as a
/// [`DeclaredTranscriptKind`]. Names the member, never any value (#1227 W6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("resume_context.declared_transcript_kind is not a valid declared transcript kind")]
pub struct InvalidDeclaredTranscriptKind;

/// Why `resume_context.declared_root_session_kind` could not be read as a
/// declared root session kind. Names the member, never any value (#1227).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("resume_context.declared_root_session_kind is not a valid declared root session kind")]
pub(crate) struct InvalidDeclaredRootSessionKind;

/// The transcript kind a seat's loaded adapter declared in `record_kinds`,
/// carried from the engine to the driver inside `resume_context` (#1227 W6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclaredTranscriptKind {
    AgyConversation,
    Other(String),
}

impl DeclaredTranscriptKind {
    /// Parse a declared `record_kinds` kind word (the one shared rule in
    /// `brokkr_core::seat::is_declared_record_kind`: `^[a-z][a-z0-9]*(-[a-z0-9]+)*$`,
    /// at most 40 bytes, never `"none"`, #1227 Y2) into a [`DeclaredTranscriptKind`].
    pub fn parse(word: &str) -> Result<Self, InvalidDeclaredTranscriptKind> {
        if !brokkr_core::seat::is_declared_record_kind(word) {
            return Err(InvalidDeclaredTranscriptKind);
        }
        if word == Kind::AgyConversation.label() {
            Ok(Self::AgyConversation)
        } else {
            Ok(Self::Other(word.to_string()))
        }
    }

    /// The wire word for this declared kind.
    fn as_str(&self) -> &str {
        match self {
            Self::AgyConversation => Kind::AgyConversation.label(),
            Self::Other(word) => word.as_str(),
        }
    }

    /// Whether this declaration is `agy-conversation`.
    pub(crate) fn is_agy_conversation(&self) -> bool {
        matches!(self, Self::AgyConversation)
    }

    /// Write this declaration into the engine's `resume_context` object.
    pub fn write_into(&self, resume_context: &mut Map<String, Value>) {
        resume_context.insert(
            DECLARED_TRANSCRIPT_KIND_KEY.into(),
            Value::String(self.as_str().to_string()),
        );
    }

    /// Write a declared `record_kinds.root_session` kind into the engine's
    /// `resume_context` object for a work seat (#1227).
    pub fn write_root_session_into(&self, resume_context: &mut Map<String, Value>) {
        resume_context.insert(
            DECLARED_ROOT_SESSION_KIND_KEY.into(),
            Value::String(self.as_str().to_string()),
        );
    }

    /// Read the engine's declaration from a driver start `input`.
    ///
    /// Returns `Ok(None)` when `resume_context` is absent/null or does not
    /// carry the member (an adapter without `record_kinds` or a by-hand
    /// driver launch). Returns `Err(InvalidDeclaredTranscriptKind)` whenever
    /// `resume_context` is non-object or the member is present with an
    /// unreadable value, so the driver refuses to start rather than silently
    /// recording `none`.
    pub fn read_from(input: &Value) -> Result<Option<Self>, InvalidDeclaredTranscriptKind> {
        Self::read_key_from(input, DECLARED_TRANSCRIPT_KIND_KEY)
            .map_err(|()| InvalidDeclaredTranscriptKind)
    }

    /// Read the engine's declared `record_kinds.root_session` kind from a
    /// driver start `input` (#1227).
    pub(crate) fn read_root_session_from(
        input: &Value,
    ) -> Result<Option<Self>, InvalidDeclaredRootSessionKind> {
        Self::read_key_from(input, DECLARED_ROOT_SESSION_KIND_KEY)
            .map_err(|()| InvalidDeclaredRootSessionKind)
    }

    fn read_key_from(input: &Value, key: &str) -> Result<Option<Self>, ()> {
        let Some(context) = input.get("resume_context") else {
            return Ok(None);
        };
        if context.is_null() {
            return Ok(None);
        }
        let Some(map) = context.as_object() else {
            return Err(());
        };
        let Some(value) = map.get(key) else {
            return Ok(None);
        };
        let Some(word) = value.as_str() else {
            return Err(());
        };
        Self::parse(word).map(Some).map_err(|_| ())
    }
}
