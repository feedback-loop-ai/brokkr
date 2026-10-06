//! The probe's one module with effects: the scratch world a CLI runs in,
//! each launch run to an exit or a deadline, and what the launch printed
//! and wrote, masked before anything reads it.
//!
//! A launch sees a scratch HOME and a scratch repository and nothing of
//! the operator's: the environment is cleared, then given `PATH`, `HOME`
//! and the credentials bound by name (decision 0012). Whatever the CLI
//! needs beyond those is what config isolation (#467) measures.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use ruzstd::decoding::{FrameDecoder, StreamingDecoder};
use sha2::{Digest, Sha256};

use super::plan::{Step, Transcripts, UserConfig, PROMPT};
use super::ProbeError;
use crate::hands::{mcp_config, serve_args, HandsSpec};
use crate::process::Launched;
use crate::secret::bind_environment;
use crate::secret::{mask_bytes, BoundSecret};

/// The scratch directory's name prefix. A path a CLI derives from its
/// working directory carries it, which is how `measure` finds the part of
/// a transcript path that names this run's scratch repository.
pub(crate) const SCRATCH_PREFIX: &str = "brokkr-probe-";

const POLL: Duration = Duration::from_millis(20);

/// Captured bytes, masked, as text decoded line by line and strictly: a
/// line whose bytes are not UTF-8 is never repaired. It stays an empty
/// line of `text`, so the lines after it keep their numbers, and is named
/// in `not_utf8`, so no reader reads it as anything (#484).
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Captured {
    pub(crate) text: String,
    /// Each line, counted from 1, whose bytes are not UTF-8.
    pub(crate) not_utf8: Vec<usize>,
}

/// A `.jsonl` file, or a `.jsonl.zstd` one read decompressed, a launch
/// wrote under the scratch HOME, `~`-relative, how it wrote it, and the
/// masked text of what it wrote.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Transcript {
    pub(crate) path: String,
    pub(crate) written: Written,
    pub(crate) text: Captured,
}

/// How a launch wrote a transcript, which says which of its bytes are the
/// launch's: a launch shares the scratch HOME with the launches before it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Written {
    /// The file is new, and all of it is the launch's.
    Created,
    /// The file kept the bytes it held and grew; the launch's are those
    /// after them, read from the start of the line they begin in, which
    /// is this line of the file.
    Appended { from_line: usize },
    /// The file shrank or its earlier bytes changed, so all of it is read.
    Rewritten,
}

/// A transcript's length and digest before a launch, decompressed where
/// it is compressed.
struct Seen {
    len: usize,
    digest: Vec<u8>,
}

impl Seen {
    fn of(bytes: &[u8]) -> Seen {
        Seen {
            len: bytes.len(),
            digest: Sha256::digest(bytes).to_vec(),
        }
    }

    /// What a launch that left the file holding `bytes` wrote to it, and
    /// the bytes that are its, from the start of the physical line its
    /// first byte landed in, so a line it joined to one an earlier launch
    /// left unfinished is read whole (#484); `None` when it left the file
    /// as it was.
    fn added<'a>(&self, bytes: &'a [u8]) -> Option<(Written, &'a [u8])> {
        match bytes.split_at_checked(self.len) {
            Some((kept, grown)) if Seen::of(kept).digest == self.digest => {
                let start = kept
                    .iter()
                    .rposition(|byte| *byte == b'\n')
                    .map_or(0, |newline| newline + 1);
                let from_line = kept[..start].iter().filter(|byte| **byte == b'\n').count() + 1;
                (!grown.is_empty()).then_some((Written::Appended { from_line }, &bytes[start..]))
            }
            _ => Some((Written::Rewritten, bytes)),
        }
    }
}

/// What one launch did: its exit code (`None` for a signal or the
/// deadline), what it printed, and the `.jsonl` files it wrote.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Observation {
    pub(crate) exit: Option<i32>,
    pub(crate) stdout: Captured,
    pub(crate) stderr: Captured,
    pub(crate) transcripts: Vec<Transcript>,
}

/// A planned launch, observed, or the reason it was not tried.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Trial {
    Observed(Observation),
    Untried(String),
}

fn io<T>(result: std::io::Result<T>, what: &str) -> Result<T, ProbeError> {
    result.map_err(|source| ProbeError::Io {
        what: what.to_string(),
        source,
    })
}

/// The scratch world: a HOME and a repository, removed when dropped.
pub(crate) struct Scratch {
    _root: tempfile::TempDir,
    home: PathBuf,
    repo: PathBuf,
    /// The spellings of the root a CLI may print, longest first, so a
    /// canonical `/private/var/…` is replaced before the `/var/…` inside it.
    spellings: Vec<String>,
}

impl Scratch {
    /// A HOME with nothing in it and a repository with one file. The
    /// repository's `.git` is written by hand, which is all a CLI that
    /// insists on a repository checks for, and needs no `git`.
    pub(crate) fn create() -> Result<Scratch, ProbeError> {
        Scratch::create_in(&std::env::temp_dir())
    }

    /// [`Scratch::create`] under `tmp`, which a test makes one that fails.
    pub(crate) fn create_in(tmp: &Path) -> Result<Scratch, ProbeError> {
        let root = io(lay_out(tmp), "could not create the probe's scratch world")?;
        let home = root.path().join("home");
        let repo = root.path().join("repo");
        let mut spellings = vec![
            root.path().to_string_lossy().into_owned(),
            fs::canonicalize(root.path())
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        ];
        spellings.retain(|spelling| !spelling.is_empty());
        spellings.sort_by_key(|spelling| std::cmp::Reverse(spelling.len()));
        Ok(Scratch {
            _root: root,
            home,
            repo,
            spellings,
        })
    }

    /// Write the user-scope MCP configuration a turn may or may not read.
    pub(crate) fn plant(&self, config: &UserConfig) -> Result<(), ProbeError> {
        const WHAT: &str = "could not plant the user-scope configuration";
        if let UserConfig::Planted { path, contents } = config {
            let path = self.home.join(path);
            io(
                fs::create_dir_all(path.parent().unwrap_or(&self.home)),
                WHAT,
            )?;
            io(fs::write(path, contents), WHAT)?;
        }
        Ok(())
    }
}

/// The scratch root under `tmp`, holding an empty `home` and a `repo`
/// with one file, which fails as one step: once the root is made, what
/// is made inside it fails only with the filesystem.
fn lay_out(tmp: &Path) -> std::io::Result<tempfile::TempDir> {
    let root = tempfile::Builder::new()
        .prefix(SCRATCH_PREFIX)
        .tempdir_in(tmp)?;
    let repo = root.path().join("repo");
    for dir in [
        &root.path().join("home"),
        &repo.join(".git/objects"),
        &repo.join(".git/refs/heads"),
    ] {
        fs::create_dir_all(dir)?;
    }
    fs::write(repo.join(".git/HEAD"), "ref: refs/heads/main\n")?;
    fs::write(repo.join("README.md"), "# probe\n")?;
    Ok(root)
}

/// Runs launches in one scratch world.
pub(crate) struct Runner<'a> {
    pub(crate) scratch: &'a Scratch,
    pub(crate) cli: &'a str,
    pub(crate) brokkr: &'a Path,
    pub(crate) bindings: &'a [BoundSecret],
    pub(crate) deadline: Duration,
    pub(crate) transcripts: Transcripts,
}

impl Runner<'_> {
    /// A planned step, launched with the bound credentials, or passed on.
    pub(crate) fn trial(&self, step: &Step) -> Result<Trial, ProbeError> {
        match step {
            Step::Launch(argv) => Ok(Trial::Observed(self.launch(argv, true)?)),
            Step::Untried(why) => Ok(Trial::Untried(why.clone())),
        }
    }

    /// The template's placeholders filled in: the CLI, the scratch
    /// repository, the prompt, and the hands MCP server the adapter's
    /// argv names, which is this binary's own `brokkr hands serve`.
    fn argv(&self, template: &[String]) -> Vec<String> {
        let spec = HandsSpec::default();
        let repo = self.scratch.repo.to_string_lossy();
        let mcp_json = mcp_config(self.brokkr, &self.scratch.repo, &spec).to_string();
        // A JSON array of strings is a TOML inline array of basic strings.
        let args_toml =
            serde_json::to_string(&serve_args(&self.scratch.repo, &spec)).unwrap_or_default();
        let brokkr = self.brokkr.to_string_lossy();
        template
            .iter()
            .map(|part| {
                part.replace("{cli}", self.cli)
                    .replace("{workdir}", &repo)
                    .replace("{prompt}", PROMPT)
                    .replace("{hands_mcp_json}", &mcp_json)
                    .replace("{hands_args_toml}", &args_toml)
                    .replace("{brokkr}", &brokkr)
            })
            .collect()
    }

    /// Run one launch to its exit or the deadline, with or without the
    /// bound credentials, stdin closed, and report what it did.
    pub(crate) fn launch(
        &self,
        template: &[String],
        credentials: bool,
    ) -> Result<Observation, ProbeError> {
        let argv = self.argv(template);
        let bindings: &[BoundSecret] = if credentials { self.bindings } else { &[] };
        let before: BTreeMap<PathBuf, Seen> =
            transcripts_under(&self.scratch.home, &self.transcripts)?
                .into_iter()
                .map(|(path, bytes)| (path, Seen::of(&bytes)))
                .collect();
        let mut command = Command::new(&argv[0]);
        command
            .args(&argv[1..])
            .current_dir(&self.scratch.repo)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("HOME", &self.scratch.home)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // Decision 0012 layer 4: every value leaves through the one
        // injector the adapters spawn with.
        bind_environment(&mut command, bindings)
            .map_err(|refused| ProbeError::Credential(refused.to_string()))?;
        // A session and group of its own, among the engine's attempts, so
        // the end reaches every descendant (#403).
        let what = format!("could not launch {}", self.cli);
        let mut launched = io(Launched::spawn(&mut command), &what)?;
        let stdout = drain(launched.child.stdout.take().expect("piped"));
        let stderr = drain(launched.child.stderr.take().expect("piped"));
        let exit = wait(launched, self.deadline)?;
        Ok(Observation {
            exit,
            stdout: self.clean(&stdout.join().unwrap_or_default()),
            stderr: self.clean(&stderr.join().unwrap_or_default()),
            transcripts: self.written(&before)?,
        })
    }

    /// Every transcript under the scratch HOME whose content the launch
    /// changed, by length and digest rather than by path, so a transcript
    /// an earlier launch created and this one wrote to is this one's too.
    fn written(&self, before: &BTreeMap<PathBuf, Seen>) -> Result<Vec<Transcript>, ProbeError> {
        let mut transcripts = Vec::new();
        for (path, bytes) in transcripts_under(&self.scratch.home, &self.transcripts)? {
            let added = match before.get(&path) {
                Some(seen) => seen.added(&bytes),
                None => Some((Written::Created, bytes.as_slice())),
            };
            if let Some((written, text)) = added {
                let home = path.strip_prefix(&self.scratch.home).unwrap_or(&path);
                transcripts.push(Transcript {
                    path: format!("~/{}", home.display()),
                    written,
                    text: self.clean(text),
                });
            }
        }
        Ok(transcripts)
    }

    /// Captured bytes as text: every bound value masked first (decision
    /// 0012), then each line decoded strictly, and in each line that
    /// decodes the scratch root, which no two runs share, written as
    /// `{scratch}`.
    fn clean(&self, bytes: &[u8]) -> Captured {
        let mut captured = Captured::default();
        let masked = mask_bytes(bytes, self.bindings);
        for (index, line) in masked.split_inclusive(|byte| *byte == b'\n').enumerate() {
            match std::str::from_utf8(line) {
                Ok(line) => captured.text.push_str(&self.unscratched(line)),
                Err(_) => {
                    captured.not_utf8.push(index + 1);
                    captured.text.push('\n');
                }
            }
        }
        captured
    }

    fn unscratched(&self, line: &str) -> String {
        let mut text = line.to_string();
        for spelling in &self.scratch.spellings {
            text = text.replace(spelling.as_str(), "{scratch}");
        }
        text
    }
}

fn drain(mut pipe: impl Read + Send + 'static) -> JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = pipe.read_to_end(&mut bytes);
        bytes
    })
}

/// The child's exit code, or `None` once the deadline passes and the
/// child is killed. A CLI waiting on a prompt the probe never answers
/// must not hold the probe forever, and neither may a descendant holding
/// the output pipes the drains read to their end, so the launch's whole
/// tree is ended when the child exits or the deadline passes. A tree not
/// proven over refuses the probe.
fn wait(launched: Launched, deadline: Duration) -> Result<Option<i32>, ProbeError> {
    let started = Instant::now();
    while !launched.exited() && started.elapsed() < deadline {
        std::thread::sleep(POLL);
    }
    launched.end().map_err(ProbeError::Unended)
}

/// Every transcript under `root` and its bytes, decompressed where it is
/// compressed. One that cannot be read refuses the probe rather than read
/// as nothing written, and so does a compressed one that is malformed or
/// decompresses past [`UNPACKED_BOUND`], one named as a transcript in a
/// packing the probe does not read, and any other file under the
/// harness's transcript directories that is not inert (#484).
fn transcripts_under(
    root: &Path,
    kept: &Transcripts,
) -> Result<BTreeMap<PathBuf, Vec<u8>>, ProbeError> {
    let mut transcripts = BTreeMap::new();
    for path in files_under(root)? {
        let Some(packing) = reading(root, &path, kept) else {
            continue;
        };
        let read = || {
            io(
                fs::read(&path),
                "could not read a transcript under the scratch HOME",
            )
        };
        let named = |doing: &str| {
            let home = path.strip_prefix(root).unwrap_or(&path);
            format!(
                "could not {doing} the transcript ~/{} under the scratch HOME",
                home.display()
            )
        };
        let bytes = match packing {
            Packing::Plain => read()?,
            Packing::Zstd => io(unpacked(&read()?, UNPACKED_BOUND), &named("decompress"))?,
            Packing::Unread => {
                let why = "the probe reads a transcript only as .jsonl or .jsonl.zstd";
                let source = std::io::Error::new(std::io::ErrorKind::Unsupported, why);
                return io(Err(source), &named("read"));
            }
        };
        transcripts.insert(path, bytes);
    }
    Ok(transcripts)
}

/// The most a compressed transcript may decompress to: a log past it is
/// refused, never read to its end, so a huge one is not a hang or an
/// exhausted memory (#484).
const UNPACKED_BOUND: u64 = 64 * 1024 * 1024;

/// How a transcript is stored, by its name: a `.jsonl` file as written,
/// or a `.jsonl.zstd` file, zstd-compressed, as dsh writes its session
/// log, `session.v3.jsonl.zstd` (#484).
///
/// Any other name marked as a transcript is `Unread`, and refuses the
/// probe: the same bytes renamed `.jsonl.zst` must not read as nothing
/// written, so a tool the turn ran is never read as no tool run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Packing {
    Plain,
    Zstd,
    Unread,
}

/// The name segments that mark a file a transcript, in any case and under
/// any further extension: `turn.jsonl.zst`, `turn.ndjson`, `turn.JSONL`.
/// They name a transcript outside the harness's transcript directories,
/// where every file is read or refused whatever its name ([`reading`]).
const TRANSCRIPT_SEGMENTS: [&str; 3] = ["jsonl", "ndjson", "jsonlines"];

/// How the file at `path` under `home` is read, when it is read at all.
/// Under one of the harness's transcript directories a file is a
/// transcript in a packing the probe reads, or one of the inert names a
/// recording showed there, or unread, which refuses the probe: none is
/// skipped by default. Elsewhere a file is a transcript by its name.
pub(super) fn reading(home: &Path, path: &Path, kept: &Transcripts) -> Option<Packing> {
    let under = kept.dirs.iter().any(|dir| path.starts_with(home.join(dir)));
    let inert = path
        .file_name()
        .is_some_and(|name| kept.inert.iter().any(|inert| name == OsStr::new(inert)));
    match packing(path) {
        Some(packing) => Some(packing),
        None => (under && !inert).then_some(Packing::Unread),
    }
}

/// How the file at `path` is stored, when it is a transcript at all.
pub(super) fn packing(path: &Path) -> Option<Packing> {
    let is = |path: &Path, extension: &str| path.extension() == Some(OsStr::new(extension));
    let stem = path.file_stem().map(Path::new);
    let compressed = is(path, "zstd") && stem.is_some_and(|stem| is(stem, "jsonl"));
    match (is(path, "jsonl"), compressed) {
        (true, _) => Some(Packing::Plain),
        (false, true) => Some(Packing::Zstd),
        (false, false) => marked(path).then_some(Packing::Unread),
    }
}

/// Whether a segment of the file's name after its first is one of
/// [`TRANSCRIPT_SEGMENTS`], in any case.
fn marked(path: &Path) -> bool {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    name.split('.').skip(1).any(|segment| {
        TRANSCRIPT_SEGMENTS
            .iter()
            .any(|marker| segment.eq_ignore_ascii_case(marker))
    })
}

/// `packed`, each zstd frame of it decompressed in turn, refused once
/// what it decompresses to passes `bound` bytes. A frame the decoder does
/// not read whole, a skippable frame among them, is an error, never read
/// as nothing, and so is one whose bytes its content checksum does not
/// vouch for.
pub(super) fn unpacked(mut packed: &[u8], bound: u64) -> std::io::Result<Vec<u8>> {
    let mut unpacked = Vec::new();
    while !packed.is_empty() {
        let mut frame = StreamingDecoder::new(&mut packed).map_err(std::io::Error::other)?;
        // Within the bound so far, so one byte past what it leaves.
        let room = bound - unpacked.len() as u64 + 1;
        (&mut frame).take(room).read_to_end(&mut unpacked)?;
        if unpacked.len() as u64 > bound {
            return Err(std::io::Error::new(
                std::io::ErrorKind::FileTooLarge,
                format!("it decompresses past {bound} bytes"),
            ));
        }
        checksummed(&frame.into_frame_decoder())?;
    }
    Ok(unpacked)
}

/// Whether `frame`, read to its end, carries the content checksum of the
/// bytes it decompressed to. The streaming decoder compares nothing, and
/// most single-bit corruption decodes to changed bytes without an error,
/// so a frame is trusted only when its checksum is stated and matches,
/// and one stating none cannot be verified and is refused too (#484, the
/// operator's ruling of 2026-10-04).
fn checksummed(frame: &FrameDecoder) -> std::io::Result<()> {
    let refused = |why: String| Err(std::io::Error::new(std::io::ErrorKind::InvalidData, why));
    match frame.get_checksum_from_data() {
        None => refused("a frame carries no content checksum to verify it by".to_string()),
        Some(stated) if frame.get_calculated_checksum() == Some(stated) => Ok(()),
        Some(stated) => refused(format!(
            "a frame's bytes do not match its content checksum {stated:#010x}"
        )),
    }
}

/// Every file under `root`, symlinks not followed. A directory that
/// cannot be listed refuses the probe, since a transcript in it would
/// otherwise read as nothing written.
fn files_under(root: &Path) -> Result<BTreeSet<PathBuf>, ProbeError> {
    const UNLISTED: &str = "could not list a directory under the scratch HOME";
    let mut found = BTreeSet::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in io(fs::read_dir(&dir), UNLISTED)? {
            let entry = io(entry, UNLISTED)?;
            if io(entry.file_type(), UNLISTED)?.is_dir() {
                pending.push(entry.path());
            } else {
                found.insert(entry.path());
            }
        }
    }
    Ok(found)
}
