//! The serving command shapes, moved out of `adapters.rs` unchanged so it
//! stays inside its measured ceiling as Antigravity's shape joins (#1200).

/// The serving command shapes the built-in drivers spawn, one per builder
/// [`serving_command`] calls and Qoder's and Agy's, each composed whole in
/// its driver's one home (#1178, #1200). LaneTally is served Claude's. `exec`
/// and an opaque driver have none ([`crate::native_controls::grammar::grammar`]
/// models no grammar of theirs).
///
/// [`serving_command`]: super::serving_command
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ServingShape {
    Claude,
    Codex,
    Dsh,
    Qoder,
    Agy,
}

impl ServingShape {
    pub(crate) fn of(harness: &str) -> Option<ServingShape> {
        match harness {
            "claude" | "lanetally" => Some(ServingShape::Claude),
            "codex" => Some(ServingShape::Codex),
            "dsh" => Some(ServingShape::Dsh),
            "qoder" => Some(ServingShape::Qoder),
            "agy" => Some(ServingShape::Agy),
            _ => None,
        }
    }
}
