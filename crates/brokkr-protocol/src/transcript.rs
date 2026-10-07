//! The transcript law shared by every built-in driver (decision 0032).
//!
//! A harness-specific arm supplies only its locator. This module owns the
//! closed kind vocabulary, harness-home resolution, the 80-character
//! locator clamp, the `session_meta.transcript` shape, and the checkpoint
//! row that puts that shape in the journal. It never reads transcript
//! content and never removes a transcript or its directory; an engine-only
//! dsh home is removed only when no transcript row can have named it.

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
        let operator_home = std::env::var_os("HOME");
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

    /// A dsh launch's transcript at the one home it is served from.
    pub(crate) fn dsh(home: &DshHome) -> Self {
        Transcript::at(Kind::DshSession, home.path().to_path_buf())
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
    ) -> Value {
        self.locator = locator.chars().take(LOCATOR_LIMIT).collect();
        self.publish(session_meta, emit)
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

    /// Returns the row it published, so a caller that must carry the
    /// address elsewhere takes it from here instead of reading it back
    /// out of `session_meta` through a lookup that cannot miss.
    fn publish(
        &mut self,
        session_meta: &mut Map<String, Value>,
        emit: &mut impl FnMut(&Value),
    ) -> Value {
        let transcript = self.value();
        session_meta.insert("transcript".into(), transcript.clone());
        emit(&json!({"step": "transcript", "transcript": transcript}));
        self.journaled = true;
        transcript
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
    dsh_home_from(std::env::var_os("DSH_HOME"), std::env::var_os("HOME"))
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
    Ok(fresh_under(&home.join("sessions").join("brokkr"), "seat-")?.keep())
}

/// A fresh directory named `<prefix>…` in `base`, created with its parents,
/// removed when its handle is dropped unless the caller keeps it.
fn fresh_under(base: &Path, prefix: &str) -> std::io::Result<tempfile::TempDir> {
    std::fs::create_dir_all(base)?;
    tempfile::Builder::new().prefix(prefix).tempdir_in(base)
}

/// Where engine-only homes are staged, below the operator's `HOME`: beside
/// the engine's protected capability root and outside the operator's dsh
/// home, so no directory dsh's Node lookup walks above a staged profile is
/// the operator's dsh tree, and none is a shared, world-writable one.
const ENGINE_HOMES: [&str; 4] = [".local", "state", "brokkr", "dsh-homes"];

/// The dsh home one launch is served from (decision 0065 slice two, U1c2;
/// requirements SI2 and MB1). A launch without the engine's isolation
/// intent is served from the operator's own home, resolved as the harness
/// resolves it, exactly as before. One with an intent is served from an
/// engine-only home staged for that seat, U0's D03/D04 shape: dsh is
/// pointed at a directory nothing but dsh itself has written, so its
/// profile is the shipped bundles' own scaffold, no home-level or
/// profile-level row is ambient, and the validated route rows and the
/// engine's server row ride the seat's one `--patch` overlay. The launch's
/// transcript, its retained root, the persistence home an offered root is
/// checked against and the composite's declared home all read this value.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DshHome {
    Operator(PathBuf),
    Engine(EngineHome),
}

/// A staged engine-only home, held as text because the child's environment
/// names it. It is removed with its launch unless [`DshHome::serve`] kept
/// it, so a launch refused before it spawns leaves no home behind, while a
/// home a transcript row may name stays for that transcript.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct EngineHome {
    path: String,
    retained: std::cell::Cell<bool>,
}

impl Drop for EngineHome {
    fn drop(&mut self) {
        if !self.retained.get() {
            // Best effort: a home that cannot be removed holds no transcript.
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

/// Why no dsh home is served. The text is the operator's.
#[derive(Debug, thiserror::Error)]
pub(crate) enum DshHomeError {
    #[error("no harness home for transcript kind dsh-session")]
    Unresolved,
    #[error("no HOME to stage the engine-only dsh home under")]
    Homeless,
    /// The secret injector sets a declared name over the launch's own
    /// environment, so the child would run on another home or identity
    /// than every read of the launch follows.
    #[error(
        "refusing to invoke the agent CLI: the seat declares a binding named '{0}', which the \
         engine-only dsh home fixes in the child's environment"
    )]
    FixedBinding(String),
    #[error("could not stage the engine-only dsh home: {0}")]
    Unstaged(#[source] std::io::Error),
    #[error("the engine-only dsh home's path is not UTF-8, so it cannot be named to dsh")]
    NotUtf8,
    /// dsh resolves a credential from `<cwd>/.env` after the process
    /// environment and before either home file (dsh-credentials-local
    /// 0.1.5-rc.2, U0c): the one layer an engine-only home does not close.
    #[error(
        "refusing to invoke the agent CLI: the seat's working directory holds a `.env`, or cannot \
         be read for one: dsh reads it as a credential layer its engine-only home does not close, \
         so a key could reach the provider from somewhere other than the engine's environment"
    )]
    WorkdirCredentials,
}

impl From<DshHomeError> for String {
    fn from(error: DshHomeError) -> String {
        error.to_string()
    }
}

impl DshHome {
    /// The operator's own home, as a launch with no intent always read it.
    pub(crate) fn operator() -> Result<DshHome, DshHomeError> {
        dsh_home()
            .map(DshHome::Operator)
            .ok_or(DshHomeError::Unresolved)
    }

    /// A fresh engine-only home under [`ENGINE_HOMES`], in the `HOME` the
    /// child inherits. Refused first where `workdir`, the child's cwd, holds
    /// a `.env` in any form, since the child would read it.
    pub(crate) fn stage(workdir: &str) -> Result<DshHome, DshHomeError> {
        DshHome::stage_under(std::env::var_os("HOME").map(PathBuf::from), workdir)
    }

    fn stage_under(user: Option<PathBuf>, workdir: &str) -> Result<DshHome, DshHomeError> {
        // An empty workdir is the child's `.`, which a relative `.env` names.
        match std::fs::symlink_metadata(Path::new(workdir).join(".env")) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            _ => return Err(DshHomeError::WorkdirCredentials),
        }
        let base = user.ok_or(DshHomeError::Homeless)?;
        let base = base.join(ENGINE_HOMES.iter().collect::<PathBuf>());
        let staged = fresh_under(&base, "engine-home-").map_err(DshHomeError::Unstaged)?;
        // Dropped unkept on the refusal, so a home dsh cannot be named is gone.
        let path = staged.path().to_str().ok_or(DshHomeError::NotUtf8)?.into();
        let _ = staged.keep();
        Ok(DshHome::Engine(EngineHome {
            path,
            retained: std::cell::Cell::new(false),
        }))
    }

    pub(crate) fn path(&self) -> &Path {
        match self {
            DshHome::Operator(path) => path,
            DshHome::Engine(home) => Path::new(&home.path),
        }
    }

    /// The `DSH_HOME` the child is handed: the staged home, and none for
    /// the operator's, whose resolution the child repeats from the
    /// environment it inherits.
    pub(crate) fn child_home(&self) -> Option<&str> {
        match self {
            DshHome::Operator(_) => None,
            DshHome::Engine(home) => Some(&home.path),
        }
    }

    /// Serve the child on this home with the seat's `declared` bindings over
    /// the environment the launch `fixed`, before any transcript row names
    /// the home. An engine-only home refuses a binding named for any fixed
    /// key, its own `DSH_HOME` among them, whatever the binding's value, and
    /// is otherwise retained past its launch from here on.
    pub(crate) fn serve<'a>(
        &self,
        fixed: &[(&str, &str)],
        mut declared: impl Iterator<Item = &'a str>,
    ) -> Result<(), DshHomeError> {
        let DshHome::Engine(home) = self else {
            return Ok(());
        };
        if let Some(name) = declared.find(|name| fixed.iter().any(|(key, _)| key == name)) {
            return Err(DshHomeError::FixedBinding(name.to_string()));
        }
        home.retained.set(true);
        Ok(())
    }
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
    fn the_transcript_root_is_kept_under_the_harness_home_and_survives_the_seat() {
        let home = tempfile::tempdir().unwrap();
        let root = dsh_transcript_root_under(Some(home.path().to_path_buf())).unwrap();
        assert!(
            root.starts_with(home.path().join("sessions").join("brokkr")),
            "{root:?}"
        );
        assert!(root
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("seat-"));
        // The creating handle is gone; the directory is not.
        assert!(root.is_dir(), "{root:?}");
        let other = dsh_transcript_root_under(Some(home.path().to_path_buf())).unwrap();
        assert_ne!(root, other, "one root per seat");

        let refused = dsh_transcript_root_under(None).unwrap_err();
        assert!(
            refused.to_string().contains("set DSH_HOME or HOME"),
            "{refused}"
        );
        // A file where the base must be a directory is the staging failure.
        let blocked = tempfile::tempdir().unwrap();
        std::fs::write(blocked.path().join("sessions"), b"not a directory").unwrap();
        assert!(dsh_transcript_root_under(Some(blocked.path().to_path_buf())).is_err());
    }

    #[test]
    fn the_dsh_home_is_dsh_home_when_set_else_dot_dsh_under_home() {
        assert_eq!(
            dsh_home_from(
                Some(OsString::from("/opt/dsh")),
                Some(OsString::from("/home/x"))
            ),
            Some(PathBuf::from("/opt/dsh"))
        );
        assert_eq!(
            dsh_home_from(Some(OsString::new()), Some(OsString::from("/home/x"))),
            Some(PathBuf::from("/home/x/.dsh"))
        );
        assert_eq!(
            dsh_home_from(None, Some(OsString::from("/home/x"))),
            Some(PathBuf::from("/home/x/.dsh"))
        );
        assert_eq!(dsh_home_from(None, None), None);
        assert_eq!(
            dsh_home(),
            dsh_home_from(std::env::var_os("DSH_HOME"), std::env::var_os("HOME"))
        );
    }

    /// The names directly inside `dir`, sorted.
    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    /// A canonical temporary root holding a user's `HOME`, not yet created,
    /// and the seat's worktree, from which engine-only homes are staged.
    struct Seat {
        _dir: tempfile::TempDir,
        user: PathBuf,
        work: PathBuf,
    }

    impl Seat {
        fn new() -> Seat {
            let dir = tempfile::tempdir().unwrap();
            let root = std::fs::canonicalize(dir.path()).unwrap();
            let (user, work) = (root.join("user"), root.join("work"));
            std::fs::create_dir_all(&work).unwrap();
            Seat {
                _dir: dir,
                user,
                work,
            }
        }

        fn stage_for(&self, user: PathBuf) -> Result<DshHome, DshHomeError> {
            DshHome::stage_under(Some(user), self.work.to_str().unwrap())
        }

        fn stage(&self) -> Result<DshHome, DshHomeError> {
            self.stage_for(self.user.clone())
        }
    }

    /// An engine-only home is a fresh, empty directory of its own under the
    /// user's `.local/state/brokkr/dsh-homes`, one per seat, named to the
    /// child, and the transcript reads it; the operator's home is named to
    /// no child.
    #[test]
    fn an_engine_only_home_is_staged_fresh_and_empty_for_each_seat() {
        let seat = Seat::new();
        let (first, second) = (seat.stage().unwrap(), seat.stage().unwrap());
        let base = seat.user.join(".local/state/brokkr/dsh-homes");
        let mut staged = Vec::new();
        for home in [&first, &second] {
            let path = home.child_home().unwrap();
            assert_eq!(home.path(), Path::new(path));
            assert_eq!(Transcript::dsh(home).home(), Path::new(path));
            assert_eq!(names(home.path()), Vec::<String>::new(), "nothing staged");
            staged.push(home.path().strip_prefix(&base).unwrap().to_path_buf());
        }
        assert_ne!(staged[0], staged[1], "one home per seat");
        assert!(staged
            .iter()
            .all(|name| name.to_string_lossy().starts_with("engine-home-")));
        assert_eq!(names(&base).len(), 2);
        let operator_home = DshHome::Operator(seat.user.clone());
        assert_eq!(
            (operator_home.child_home(), operator_home.path()),
            (None, seat.user.as_path())
        );
    }

    /// An engine-only home serves no declared binding named for a key the
    /// launch fixes, whatever its value, and is removed with its launch; one
    /// it serves is retained past it. The operator's home, as before, serves
    /// every binding and is never removed.
    #[test]
    fn an_engine_only_home_refuses_a_binding_over_a_fixed_key() {
        let seat = Seat::new();
        let base = seat.user.join(".local/state/brokkr/dsh-homes");
        for (declared, refused) in [
            (&["API_TOKEN"][..], None),
            (&["API_TOKEN", "DSH_HOME"][..], Some("DSH_HOME")),
            (&["GIT_CONFIG_COUNT"][..], Some("GIT_CONFIG_COUNT")),
        ] {
            let home = seat.stage().unwrap();
            let staged = home.path().to_path_buf();
            let fixed = [("DSH_HOME", "/elsewhere"), ("GIT_CONFIG_COUNT", "1")];
            // Any other variant fails with its own text, through no closure left unrun.
            let refusal = match home.serve(&fixed, declared.iter().copied()) {
                Err(DshHomeError::FixedBinding(name)) => Ok(Some(name)),
                other => other.as_ref().map(|()| None).map_err(ToString::to_string),
            };
            assert_eq!(refusal, Ok(refused.map(String::from)), "{declared:?}");
            drop(home);
            assert_eq!(staged.is_dir(), refused.is_none(), "{declared:?}");
            let operator = DshHome::Operator(base.clone());
            assert!(matches!(
                operator.serve(&fixed, declared.iter().copied()),
                Ok(())
            ));
        }
        assert_eq!(names(&base).len(), 1, "the served home alone stays");
    }

    /// A home that cannot be served refuses by cause, and a worktree `.env`
    /// in any form refuses before anything is staged. Each cause's text is
    /// pinned here, once.
    #[test]
    fn a_home_that_cannot_be_served_refuses_by_cause() {
        let seat = Seat::new();
        let workdir = seat.work.to_str().unwrap();
        assert!(matches!(
            DshHome::stage_under(None, workdir),
            Err(DshHomeError::Homeless)
        ));
        let env = seat.work.join(".env");
        std::fs::write(&env, b"").unwrap();
        assert!(matches!(
            seat.stage(),
            Err(DshHomeError::WorkdirCredentials)
        ));
        std::fs::remove_file(&env).unwrap();
        std::os::unix::fs::symlink(seat.work.join("absent"), &env).unwrap();
        assert!(matches!(
            seat.stage(),
            Err(DshHomeError::WorkdirCredentials)
        ));
        assert!(!seat.user.exists(), "nothing is staged beside a refusal");
        std::fs::remove_file(&env).unwrap();
        std::fs::create_dir_all(&seat.user).unwrap();
        std::fs::write(seat.user.join(".local"), b"not a directory").unwrap();
        // `Unstaged` alone carries an I/O source.
        let blocked = seat.stage();
        let cause = (blocked.as_ref().err())
            .and_then(std::error::Error::source)
            .and_then(|source| source.downcast_ref::<std::io::Error>());
        assert_eq!(
            cause.map(std::io::Error::kind),
            Some(std::io::ErrorKind::NotADirectory)
        );
        // A worktree that cannot be read for a `.env` refuses as one holding it.
        let file = seat.work.join("file");
        std::fs::write(&file, b"").unwrap();
        assert!(matches!(
            DshHome::stage_under(Some(seat.user.clone()), file.to_str().unwrap()),
            Err(DshHomeError::WorkdirCredentials)
        ));
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::ffi::OsStrExt;
            let raw = seat.work.join(std::ffi::OsStr::from_bytes(b"user\xff"));
            assert!(matches!(
                seat.stage_for(raw.clone()),
                Err(DshHomeError::NotUtf8)
            ));
            let base = raw.join(".local/state/brokkr/dsh-homes");
            assert_eq!(names(&base), Vec::<String>::new(), "removed on refusal");
        }
        for (error, text) in [
            (
                DshHomeError::Unresolved,
                "no harness home for transcript kind dsh-session",
            ),
            (
                DshHomeError::Homeless,
                "no HOME to stage the engine-only dsh home under",
            ),
            (
                DshHomeError::FixedBinding("DSH_HOME".into()),
                "refusing to invoke the agent CLI: the seat declares a binding named 'DSH_HOME', \
                 which the engine-only dsh home fixes in the child's environment",
            ),
            (
                DshHomeError::NotUtf8,
                "the engine-only dsh home's path is not UTF-8, so it cannot be named to dsh",
            ),
            (
                DshHomeError::WorkdirCredentials,
                "refusing to invoke the agent CLI: the seat's working directory holds a `.env`, or \
                 cannot be read for one: dsh reads it as a credential layer its engine-only home \
                 does not close, so a key could reach the provider from somewhere other than the \
                 engine's environment",
            ),
        ] {
            assert_eq!(String::from(error), text);
        }
        assert_eq!(
            DshHomeError::Unstaged(std::io::Error::other("full")).to_string(),
            "could not stage the engine-only dsh home: full"
        );
    }

    #[test]
    fn the_operators_home_is_read_from_the_environment_the_child_inherits() {
        let mut env = crate::env_guard::EnvGuard::lock();
        env.set("DSH_HOME", "/opt/operator-dsh");
        assert_eq!(
            DshHome::operator().unwrap(),
            DshHome::Operator(PathBuf::from("/opt/operator-dsh"))
        );
        env.remove("DSH_HOME");
        env.remove("HOME");
        assert!(matches!(DshHome::operator(), Err(DshHomeError::Unresolved)));
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
