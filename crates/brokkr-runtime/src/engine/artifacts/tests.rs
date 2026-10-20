//! U8c (CR2, CR5): an attempt's cited responses published as
//! content-addressed artifacts, on a canonical temporary root of each
//! test's own. The attempt's evidence is written as provisioning and its
//! brokers write it: its plans and the inventory pinning them, and each
//! broker's ledger and stages by the protocol's own writer
//! ([`Provisioned::served`]). While the MCP compile fence stands no public
//! engine can provision a retaining plan, so these proofs live here
//! (operator ruling of 2026-10-20). Settlement's own call is proved in
//! `broker::admission::tests::sealing`.

use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::fd::OwnedFd;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use brokkr_protocol::broker::ledger::{Ledger, Outcome};
use brokkr_protocol::broker::{Inventory, Owner, Pin, Plan};
use serde_json::json;

use super::*;

/// One call a broker answers: with the bytes it staged and the digest it
/// cited for them, or with none, and how the call ended.
pub(in crate::engine) type Answer<'a> = (Option<(&'a [u8], &'a str)>, Outcome);

/// An answer, its child's result.
const ANSWER: &[u8] = br#"{"result":{"content":[{"text":"docs","type":"text"}]}}"#;

/// A child's error, which is retained too.
const ERROR: &[u8] = br#"{"error":{"code":-32000,"message":"no such page"}}"#;

/// The sha256 of `bytes`, as the broker cites it.
pub(in crate::engine) fn sha(bytes: &[u8]) -> String {
    canonical::sha256_bytes(bytes)
}

/// `bytes` written as the new file `path`, of `mode`.
fn made(path: &Path, bytes: &[u8], mode: u32) {
    let mut options = OpenOptions::new();
    let mut file = options
        .write(true)
        .create_new(true)
        .mode(mode)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
}

/// `path` made as an owner-only directory, with every one above it.
fn private_dir(path: &Path) {
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
        .unwrap();
}

/// An attempt's protected directory in its run's directory `run`, for the
/// worktree's key, the run and the attempt `owner` names.
pub(in crate::engine) struct Provisioned {
    pub(in crate::engine) run: PathBuf,
    owner: [String; 3],
}

impl Provisioned {
    /// The attempt `owner` names, its directory made in `run`.
    pub(in crate::engine) fn new(run: &Path, owner: [&str; 3]) -> Provisioned {
        let provisioned = Provisioned {
            run: run.to_path_buf(),
            owner: owner.map(str::to_owned),
        };
        private_dir(&provisioned.dir());
        provisioned
    }

    /// The attempt's directory.
    pub(in crate::engine) fn dir(&self) -> PathBuf {
        self.run.join(&self.owner[2])
    }

    /// Its owner, as every plan of it names it.
    fn owner(&self) -> Owner {
        let [repo, run, attempt] = self.owner.clone();
        Owner {
            repo,
            run,
            effect: "effect-1".into(),
            attempt,
            site: "work".into(),
            instance: "work#0".into(),
        }
    }

    /// A plan of this attempt, retaining its responses where `retained`.
    fn plan(&self, retained: bool) -> Plan {
        let digest = "b".repeat(64);
        let plan = json!({
            "owner": self.owner(), "server": "cap-docs", "capability": "docs",
            "dialect": {"name": "docs-mcp", "digest": digest, "definition": digest, "version": "1.0.0"},
            "connection": {"argv": ["/opt/docs"]}, "tools": ["lookup"], "restrictions": {},
            "retained": retained, "secrets": [], "clearance": {"dialect": digest, "policy": digest},
            "box": {
                "reach": {"writable": [], "readable": []}, "executable": "/opt/docs",
                "tree": {"kind": "system"}, "sources": {"entries": 1, "mounts": 1, "digest": digest},
                "writers": {"uids": [1], "privilege": "confined"}, "network": "isolated",
                "environment": [], "bootstrap": {"path": "/", "digest": digest},
                "excluded": {"store": "/s", "control": []}
            }
        });
        serde_json::from_value(plan).unwrap()
    }

    /// A plan for each of `retained`, retaining or not as it says, written
    /// as `plan-<index>.json`, owner-only, and the inventory pinning them
    /// last: each plan's digest.
    pub(in crate::engine) fn plans(&self, retained: &[bool]) -> Vec<String> {
        let dir = self.dir();
        let mut pins = Vec::new();
        for (index, retained) in retained.iter().enumerate() {
            let bytes = serde_json::to_vec(&self.plan(*retained)).unwrap();
            let locator = dir.join(format!("plan-{index}.json"));
            made(&locator, &bytes, 0o600);
            let digest = sha(&bytes);
            pins.push(Pin { locator, digest });
        }
        let digests = pins.iter().map(|pin| pin.digest.clone()).collect();
        let inventory = serde_json::to_vec(&Inventory { plans: pins }).unwrap();
        made(&dir.join("inventory.json"), &inventory, 0o600);
        digests
    }

    /// The broker of plan `index`, sealed at `digest`, answering each of
    /// `answers` in turn through the protocol's own ledger, which stages
    /// each answer beside it where `retains`; then closed.
    pub(in crate::engine) fn served(
        &self,
        (index, digest): (usize, &str),
        retains: bool,
        answers: &[Answer<'_>],
    ) {
        let dir = File::open(self.dir()).unwrap();
        let name = format!("plan-{index}.json");
        let plan = OsStr::new(&name);
        let owner = self.owner();
        let mut ledger = Ledger::open(&dir, plan, (&owner, digest), &[]).unwrap();
        if retains {
            ledger.retain(OwnedFd::from(dir.try_clone().unwrap()), plan, 1);
        }
        for (answer, outcome) in answers {
            let mut call = ledger.room().unwrap().begin("lookup").unwrap();
            ledger.reserve(&mut call).unwrap();
            let ended = match answer {
                Some(answered) => ledger.answered(call, *outcome, *answered),
                None => ledger.end(call, *outcome),
            };
            ended.unwrap();
        }
        ledger.close().unwrap();
    }

    /// The stage of call `call` of plan `index`.
    fn stage(&self, index: usize, call: u64) -> PathBuf {
        self.dir()
            .join(format!("plan-{index}.json.{call}.response"))
    }

    /// Its evidence published below `sealed`, each step handed to `hook`.
    fn published(&self, sealed: Option<&Sealed>, hook: Hook) -> Result<(), Unpublished> {
        let run = OwnedFd::from(File::open(&self.run).unwrap());
        let [repo, run_id, attempt] = &self.owner;
        let evidence = Evidence {
            run: &run,
            at: self.run.clone(),
            owner: [repo, run_id, attempt],
        };
        published_with(&evidence, sealed, hook)
    }
}

/// A canonical temporary root holding one attempt's run directory and an
/// artifact root, made owner-only as admission makes one it protects.
struct Fixture {
    _dir: tempfile::TempDir,
    base: PathBuf,
    attempt: Provisioned,
}

impl Fixture {
    fn new() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let base = fs::canonicalize(dir.path()).unwrap();
        private_dir(&base.join("work/.forge/artifacts"));
        let attempt = Provisioned::new(&base.join("run"), [&"a".repeat(64), "run-1", "attempt-1"]);
        Fixture {
            _dir: dir,
            base,
            attempt,
        }
    }

    /// One retaining plan, whose broker answered `answers`.
    fn retaining(answers: &[Answer<'_>]) -> Fixture {
        let fixture = Fixture::new();
        let digests = fixture.attempt.plans(&[true]);
        fixture.attempt.served((0, &digests[0]), true, answers);
        fixture
    }

    /// The artifact root.
    fn artifacts(&self) -> PathBuf {
        self.base.join("work/.forge/artifacts")
    }

    /// The artifact root as admission sealed it.
    fn sealed(&self) -> Sealed {
        let found = fs::symlink_metadata(self.artifacts()).unwrap();
        (self.artifacts(), (found.dev(), found.ino()))
    }

    /// The digest path of `bytes`.
    fn digest(&self, bytes: &[u8]) -> PathBuf {
        self.artifacts().join(DIGESTS).join(sha(bytes))
    }

    /// The names in the artifact root's `dir`, sorted; none where it is
    /// missing.
    fn names(&self, dir: &str) -> Vec<String> {
        let Ok(entries) = fs::read_dir(self.artifacts().join(dir)) else {
            return Vec::new();
        };
        let names = entries.map(|entry| entry.unwrap().file_name().into_string().unwrap());
        let mut names: Vec<String> = names.collect();
        names.sort();
        names
    }

    /// Its evidence published below its artifact root, with no hook.
    fn published(&self) -> Result<(), Unpublished> {
        self.attempt.published(Some(&self.sealed()), unhooked)
    }
}

/// The bytes and the device and inode of the file at `path`.
fn standing(path: &Path) -> (Vec<u8>, (u64, u64)) {
    let found = fs::symlink_metadata(path).unwrap();
    (fs::read(path).unwrap(), (found.dev(), found.ino()))
}

#[test]
fn every_refusal_reads_as_its_cause() {
    let unpublished = [
        (
            Unpublished::Unbound,
            "broker evidence is not bound to this attempt",
        ),
        (Unpublished::Response, "broker response digest mismatch"),
        (Unpublished::Collision, "retained artifact digest mismatch"),
        (
            Unpublished::Uncontained,
            "retained artifact path is not a regular contained file",
        ),
        (
            Unpublished::Undurable,
            "retained artifact could not be published durably",
        ),
    ];
    for (cause, text) in unpublished {
        assert_eq!(cause.to_string(), text);
    }
    let unread = [
        (Unread::Missing, "retained response artifact is missing"),
        (
            Unread::Corrupt,
            "retained response artifact digest mismatch",
        ),
        (
            Unread::Uncontained,
            "retained artifact path is not a regular contained file",
        ),
    ];
    for (cause, text) in unread {
        assert_eq!(cause.to_string(), text);
    }
}

#[test]
fn each_cited_response_is_published_as_the_very_bytes_staged() {
    // A retained answer and a retained child error are published at their
    // digests, owner-only, read-only and of one link; a failure of the
    // broker's own, failed too, cites nothing, and a stage beside it that
    // no Terminal cites, as one a failed write left, is never published.
    let (answer, error) = (sha(ANSWER), sha(ERROR));
    let fixture = Fixture::retaining(&[
        (Some((ANSWER, &answer)), Outcome::Succeeded),
        (Some((ERROR, &error)), Outcome::Failed),
        (None, Outcome::Failed),
    ]);
    made(
        &fixture.attempt.stage(0, 3),
        b"{\"result\":{\"partial",
        0o600,
    );
    assert_eq!(fixture.published(), Ok(()));
    let mut expected = vec![answer.clone(), error.clone()];
    expected.sort();
    assert_eq!(fixture.names(DIGESTS), expected);
    for bytes in [ANSWER, ERROR] {
        let path = fixture.digest(bytes);
        let found = fs::symlink_metadata(&path).unwrap();
        let shape = (
            fs::read(&path).unwrap(),
            found.mode() & 0o7777,
            found.nlink(),
        );
        assert_eq!(shape, (bytes.to_vec(), 0o400, 1));
    }
    assert_eq!(
        fixture.names(""),
        vec![DIGESTS.to_string()],
        "nothing left beside"
    );
}

#[test]
fn an_artifact_already_standing_is_reused_only_once_its_bytes_are_its_name() {
    // Published twice, the first file stands, the very same inode. A file
    // of other bytes at the digest path refuses with CR2's cause and is
    // left as it stood.
    let answer = sha(ANSWER);
    let fixture = Fixture::retaining(&[(Some((ANSWER, &answer)), Outcome::Succeeded)]);
    assert_eq!(fixture.published(), Ok(()));
    let first = standing(&fixture.digest(ANSWER));
    assert_eq!(fixture.published(), Ok(()));
    assert_eq!(standing(&fixture.digest(ANSWER)), first);
    let fixture = Fixture::retaining(&[(Some((ANSWER, &answer)), Outcome::Succeeded)]);
    private_dir(&fixture.artifacts().join(DIGESTS));
    made(&fixture.digest(ANSWER), b"{\"result\":{}}", 0o400);
    let before = standing(&fixture.digest(ANSWER));
    assert_eq!(fixture.published(), Err(Unpublished::Collision));
    assert_eq!(standing(&fixture.digest(ANSWER)), before);
}

/// What stands at a digest path in place of its artifact, made at `path`
/// beside the artifact's true bytes at `beside`.
type Planted = fn(&Path, &Path);

#[test]
fn a_digest_path_that_is_no_regular_file_of_one_link_of_its_own_is_refused_as_it_stands() {
    // A link to the very bytes, a second name of them, a directory, a FIFO
    // and a file another may write are each refused, and none is replaced.
    let cases: [(&str, Planted); 5] = [
        ("symlink", |path, beside| {
            std::os::unix::fs::symlink(beside, path).unwrap()
        }),
        ("hardlink", |path, beside| {
            fs::hard_link(beside, path).unwrap()
        }),
        ("directory", |path, _| private_dir(path)),
        // By the POSIX utility: rustix offers `mknodat` on Linux only.
        ("fifo", |path, _| {
            let made = std::process::Command::new("mkfifo").arg(path).status();
            assert!(made.unwrap().success(), "mkfifo {}", path.display());
        }),
        ("writable", |path, beside| {
            fs::copy(beside, path).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o622)).unwrap();
        }),
    ];
    let answer = sha(ANSWER);
    for (case, planted) in cases {
        let fixture = Fixture::retaining(&[(Some((ANSWER, &answer)), Outcome::Succeeded)]);
        private_dir(&fixture.artifacts().join(DIGESTS));
        let (path, beside) = (fixture.digest(ANSWER), fixture.base.join("true"));
        made(&beside, ANSWER, 0o400);
        planted(&path, &beside);
        let before = fs::symlink_metadata(&path).unwrap();
        let published = fixture.published();
        let after = fs::symlink_metadata(&path).unwrap();
        let kept = (before.ino(), before.file_type()) == (after.ino(), after.file_type());
        assert_eq!(
            (case, published, kept),
            (case, Err(Unpublished::Uncontained), true)
        );
    }
}

/// A case of a stage altered: its name, what the broker answered, how its
/// first stage was altered since, and the refusal.
type Altered<'a> = (&'a str, &'a [Answer<'a>], fn(&Path), Unpublished);

#[test]
fn a_stage_that_is_not_exactly_the_cited_bytes_is_never_published() {
    // The length the Terminal cites is not its stage's, though its digest
    // is; the digest the broker was handed is not its bytes'; the stage was
    // cut short or grown since; it is a link, has a second name, or is
    // gone. Nothing is published for any.
    let answer = sha(ANSWER);
    let cited = [(Some((ANSWER, answer.as_str())), Outcome::Succeeded)];
    let other = sha(ERROR);
    // A Terminal citing ANSWER's own digest beside a length a byte short
    // of its bytes, or a byte past them, its stage then ANSWER whole.
    let (shorter, longer) = (&ANSWER[..ANSWER.len() - 1], [ANSWER, b" "].concat());
    let restaged = |stage: &Path| fs::write(stage, ANSWER).unwrap();
    let cases: [Altered<'_>; 8] = [
        (
            "length under",
            &[(Some((shorter, answer.as_str())), Outcome::Succeeded)],
            restaged,
            Unpublished::Response,
        ),
        (
            "length over",
            &[(Some((&longer, answer.as_str())), Outcome::Succeeded)],
            restaged,
            Unpublished::Response,
        ),
        (
            "digest",
            &[(Some((ANSWER, other.as_str())), Outcome::Succeeded)],
            |_| {},
            Unpublished::Response,
        ),
        (
            "short",
            &cited,
            |stage| fs::write(stage, &ANSWER[1..]).unwrap(),
            Unpublished::Response,
        ),
        (
            "long",
            &cited,
            |stage| fs::write(stage, [ANSWER, b" "].concat()).unwrap(),
            Unpublished::Response,
        ),
        (
            "symlink",
            &cited,
            |stage| {
                let beside = stage.with_extension("kept");
                fs::rename(stage, &beside).unwrap();
                std::os::unix::fs::symlink(beside, stage).unwrap();
            },
            Unpublished::Unbound,
        ),
        (
            "hardlink",
            &cited,
            |stage| fs::hard_link(stage, stage.with_extension("kept")).unwrap(),
            Unpublished::Unbound,
        ),
        (
            "missing",
            &cited,
            |stage| fs::remove_file(stage).unwrap(),
            Unpublished::Unbound,
        ),
    ];
    for (case, answers, altered, refused) in cases {
        let fixture = Fixture::retaining(answers);
        altered(&fixture.attempt.stage(0, 1));
        let published = fixture.published();
        let found = (case, published, fixture.names(DIGESTS));
        assert_eq!(found, (case, Err(refused), Vec::<String>::new()));
    }
}

#[test]
fn a_citation_no_retaining_plan_or_protected_root_admits_is_unbound() {
    // A plan sealed retaining nothing whose ledger cites a response, and a
    // citation where admission protected no artifact root, publish nothing.
    let answer = sha(ANSWER);
    let answered = [(Some((ANSWER, answer.as_str())), Outcome::Succeeded)];
    let fixture = Fixture::new();
    let digests = fixture.attempt.plans(&[false]);
    fixture.attempt.served((0, &digests[0]), true, &answered);
    assert_eq!(fixture.published(), Err(Unpublished::Unbound));
    let fixture = Fixture::retaining(&answered);
    assert_eq!(
        fixture.attempt.published(None, unhooked),
        Err(Unpublished::Unbound)
    );
    assert_eq!(fixture.names(DIGESTS), Vec::<String>::new());
    // The control: the plan that retains nothing, its broker staging and
    // citing nothing, publishes nothing and is not refused.
    let fixture = Fixture::new();
    let digests = fixture.attempt.plans(&[false]);
    fixture.attempt.served((0, &digests[0]), false, &answered);
    assert_eq!(fixture.published(), Ok(()));
}

#[test]
fn a_plan_not_as_pinned_or_of_another_attempt_is_unbound() {
    // A plan whose bytes changed since they were pinned, one beside this
    // attempt's inventory but sealed for another run, and one its pin
    // locates in another directory, are not this attempt's evidence.
    let answer = sha(ANSWER);
    let answered = [(Some((ANSWER, answer.as_str())), Outcome::Succeeded)];
    let fixture = Fixture::retaining(&answered);
    let plan = fixture.attempt.dir().join("plan-0.json");
    let mut bytes = fs::read(&plan).unwrap();
    bytes.push(b' ');
    fs::write(&plan, bytes).unwrap();
    assert_eq!(fixture.published(), Err(Unpublished::Unbound));
    let fixture = Fixture::new();
    let foreign = [&"a".repeat(64), "run-2", "attempt-1"];
    let foreign = Provisioned::new(&fixture.attempt.run, foreign);
    let digests = foreign.plans(&[true]);
    foreign.served((0, &digests[0]), true, &answered);
    assert_eq!(fixture.published(), Err(Unpublished::Unbound));
    assert_eq!(fixture.names(DIGESTS), Vec::<String>::new());
    let fixture = Fixture::retaining(&answered);
    let inventory = fixture.attempt.dir().join("inventory.json");
    let mut pinned: Inventory = serde_json::from_slice(&fs::read(&inventory).unwrap()).unwrap();
    pinned.plans[0].locator = fixture.base.join("attempt-2/plan-0.json");
    fs::write(&inventory, serde_json::to_vec(&pinned).unwrap()).unwrap();
    assert_eq!(fixture.published(), Err(Unpublished::Unbound));
}

#[test]
fn an_attempt_that_provisioned_nothing_or_whose_broker_never_ran_publishes_nothing() {
    // No attempt directory, no inventory, or a plan whose ledger was never
    // opened: nothing is read past it and nothing published, not even the
    // digests made.
    let fixture = Fixture::new();
    fs::remove_dir(fixture.attempt.dir()).unwrap();
    assert_eq!(fixture.published(), Ok(()));
    let fixture = Fixture::new();
    assert_eq!(fixture.published(), Ok(()));
    fixture.attempt.plans(&[true]);
    assert_eq!(fixture.published(), Ok(()));
    assert_eq!(fixture.names(""), Vec::<String>::new());
}

#[test]
fn an_artifact_root_other_than_the_one_admitted_is_refused() {
    // The root renamed away and another made in its place after
    // admission: nothing is written into either.
    let answer = sha(ANSWER);
    let fixture = Fixture::retaining(&[(Some((ANSWER, &answer)), Outcome::Succeeded)]);
    let sealed = fixture.sealed();
    fs::rename(fixture.artifacts(), fixture.base.join("admitted")).unwrap();
    private_dir(&fixture.artifacts());
    let published = fixture.attempt.published(Some(&sealed), unhooked);
    assert_eq!(published, Err(Unpublished::Uncontained));
    assert_eq!(fixture.names(""), Vec::<String>::new());
    assert_eq!(
        fs::read_dir(fixture.base.join("admitted")).unwrap().count(),
        0
    );
}

/// Fails `step` alone, as a full device or a failed sync would.
fn failing(step: Step, failed: Step) -> std::io::Result<()> {
    match step == failed {
        true => Err(std::io::Error::other("refused")),
        false => Ok(()),
    }
}

/// Fails the write's step, as a full device would.
fn unwritable(step: Step, _: &OwnedFd, _: &str) -> std::io::Result<()> {
    failing(step, Step::Written)
}

/// Fails the directories' sync once the artifact is linked in.
fn unsettled(step: Step, _: &OwnedFd, _: &str) -> std::io::Result<()> {
    failing(step, Step::Linked)
}

/// Fails the syncs that would make an artifact standing durable for reuse.
fn unrenewed(step: Step, _: &OwnedFd, _: &str) -> std::io::Result<()> {
    failing(step, Step::Standing)
}

#[test]
fn a_publication_not_made_durable_is_refused_and_leaves_nothing_beside() {
    // A write that fails leaves no artifact and no temporary file; a sync
    // that fails once linked in refuses, the artifact verified by the next
    // publication as any other. A temporary file an earlier publication
    // cut short left goes first.
    let answer = sha(ANSWER);
    let fixture = Fixture::retaining(&[(Some((ANSWER, &answer)), Outcome::Succeeded)]);
    let left = fixture.artifacts().join(format!(".{answer}{PUBLISHING}"));
    made(&left, &ANSWER[..9], 0o400);
    assert_eq!(fixture.published(), Ok(()));
    assert_eq!(fixture.names(""), vec![DIGESTS.to_string()]);
    let fixture = Fixture::retaining(&[(Some((ANSWER, &answer)), Outcome::Succeeded)]);
    let published = fixture
        .attempt
        .published(Some(&fixture.sealed()), unwritable);
    assert_eq!(published, Err(Unpublished::Undurable));
    assert_eq!(fixture.names(""), vec![DIGESTS.to_string()]);
    assert_eq!(fixture.names(DIGESTS), Vec::<String>::new());
    let published = fixture
        .attempt
        .published(Some(&fixture.sealed()), unsettled);
    assert_eq!(published, Err(Unpublished::Undurable));
    assert_eq!(fixture.names(""), vec![DIGESTS.to_string()]);
    // The artifact it linked in is reused only once it and the directories
    // are synced again: a retry whose syncs fail is refused too.
    let published = fixture
        .attempt
        .published(Some(&fixture.sealed()), unrenewed);
    assert_eq!(published, Err(Unpublished::Undurable));
    assert_eq!(fixture.published(), Ok(()));
    assert_eq!(fs::read(fixture.digest(ANSWER)).unwrap(), ANSWER);
}

/// Takes the digest path first, with the very bytes being written, once
/// the temporary file is written, as a racing publication could.
fn racing(step: Step, root: &OwnedFd, temporary: &str) -> std::io::Result<()> {
    if step != Step::Written {
        return Ok(());
    }
    let read = OFlags::RDONLY | OFlags::CLOEXEC;
    let mut bytes = Vec::new();
    File::from(openat(root, temporary, read, Mode::empty())?).read_to_end(&mut bytes)?;
    let digest = temporary
        .trim_start_matches('.')
        .trim_end_matches(PUBLISHING);
    let path = format!("{DIGESTS}/{digest}");
    let made = OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::CLOEXEC;
    File::from(openat(root, path.as_str(), made, Mode::RUSR)?).write_all(&bytes)
}

/// [`racing`], and the syncs that would make what it linked in durable for
/// reuse failed.
fn raced_unrenewed(step: Step, root: &OwnedFd, temporary: &str) -> std::io::Result<()> {
    racing(step, root, temporary)?;
    failing(step, Step::Standing)
}

/// Shuts the digests to writing once the temporary file is written, so its
/// link is refused for a cause other than a name standing there.
fn unlinkable(step: Step, root: &OwnedFd, _: &str) -> std::io::Result<()> {
    if step != Step::Written {
        return Ok(());
    }
    let shut = Mode::RUSR | Mode::XUSR;
    Ok(rustix::fs::chmodat(root, DIGESTS, shut, AtFlags::empty())?)
}

/// A case of a publication raced or refused at its link: its name, its
/// hook, what publication returns, and whether an artifact then stands.
type Raced = (&'static str, Hook, Result<(), Unpublished>, bool);

#[test]
fn an_artifact_a_racing_writer_linked_in_first_is_reused_only_once_durable() {
    // The digest path taken with the very bytes between the write and the
    // link: the link replaces nothing, and what stands is reused once
    // verified and synced, refused where those syncs fail. A link refused
    // for another cause refuses. No temporary file is left beside.
    let answer = sha(ANSWER);
    let undurable = Err(Unpublished::Undurable);
    let cases: [Raced; 3] = [
        ("raced", racing, Ok(()), true),
        ("unrenewed", raced_unrenewed, undurable, true),
        ("unlinkable", unlinkable, undurable, false),
    ];
    for (case, hook, expected, stands) in cases {
        let names = if stands { vec![answer.clone()] } else { vec![] };
        let fixture = Fixture::retaining(&[(Some((ANSWER, &answer)), Outcome::Succeeded)]);
        let published = fixture.attempt.published(Some(&fixture.sealed()), hook);
        let digests = fixture.artifacts().join(DIGESTS);
        fs::set_permissions(&digests, fs::Permissions::from_mode(0o700)).unwrap();
        let found = (case, published, fixture.names(DIGESTS), fixture.names(""));
        assert_eq!(found, (case, expected, names, vec![DIGESTS.to_string()]));
    }
}

/// Replaces the artifact just linked in with another file of its very
/// bytes, as a writer racing the publication could.
fn replaced(step: Step, digests: &OwnedFd, name: &str) -> std::io::Result<()> {
    if step != Step::Linked {
        return Ok(());
    }
    let opened = openat(
        digests,
        name,
        OFlags::RDONLY | OFlags::CLOEXEC,
        Mode::empty(),
    )?;
    let mut bytes = Vec::new();
    File::from(opened).read_to_end(&mut bytes)?;
    let other = format!("{name}.other");
    let flags = OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::CLOEXEC;
    let made = openat(digests, other.as_str(), flags, Mode::RUSR)?;
    File::from(made).write_all(&bytes)?;
    Ok(rustix::fs::renameat(
        digests,
        other.as_str(),
        digests,
        name,
    )?)
}

#[test]
fn an_artifact_replaced_while_it_is_published_is_refused() {
    // The digest path no longer names the very file written, though it
    // holds the same bytes.
    let answer = sha(ANSWER);
    let fixture = Fixture::retaining(&[(Some((ANSWER, &answer)), Outcome::Succeeded)]);
    let published = fixture.attempt.published(Some(&fixture.sealed()), replaced);
    assert_eq!(published, Err(Unpublished::Uncontained));
}

#[test]
fn a_reader_tells_a_missing_artifact_from_a_corrupt_or_uncontained_one() {
    // CR5: the verified bytes; or missing, corrupt, a link, or a name that
    // is no digest, which is never resolved.
    let fixture = Fixture::new();
    let digests = fixture.artifacts().join(DIGESTS);
    private_dir(&digests);
    made(&digests.join(sha(ANSWER)), ANSWER, 0o400);
    made(&digests.join(sha(ERROR)), ANSWER, 0o400);
    let beside = fixture.base.join("true");
    made(&beside, ANSWER, 0o400);
    let linked = sha(b"linked");
    std::os::unix::fs::symlink(&beside, digests.join(&linked)).unwrap();
    let held = OwnedFd::from(File::open(&digests).unwrap());
    let cases = [
        (sha(ANSWER), Ok(ANSWER.to_vec())),
        (sha(b"absent"), Err(Unread::Missing)),
        (sha(ERROR), Err(Unread::Corrupt)),
        (linked, Err(Unread::Uncontained)),
        ("../true".to_string(), Err(Unread::Uncontained)),
    ];
    for (name, expected) in cases {
        let found = read(&held, &name).map(|(_, bytes)| bytes);
        assert_eq!((name.clone(), found), (name, expected));
    }
}
