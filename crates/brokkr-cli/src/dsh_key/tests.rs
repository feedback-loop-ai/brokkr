//! Every answer the dsh key check gives (#717), each exactly, against
//! planted files under a home and a repository of the test's own; a
//! planted value never reaches a line, and no `.env` is ever opened.

use std::os::fd::AsFd;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use brokkr_protocol::native_controls::{SealedAssessment, SealedFence, SealedIsolation};
use serde_json::json;

use super::home::Seat;
use super::patch::{layer, opened, writes, Kind, Layer, Unread, Write};
use super::*;

/// What a planted key holds: a line carrying it is a leak.
const PLANTED: &str = "planted-value-9f3";

/// A repository, a dsh home beside it and a folder's adapters, each a
/// directory of their own.
struct Planted {
    _dir: tempfile::TempDir,
    repo: PathBuf,
    home: PathBuf,
    adapters: PathBuf,
}

impl Planted {
    /// The three directories, the folder's dsh adapter pinning `models`
    /// with the route `credentials` it declares.
    fn new(models: serde_json::Value, credentials: serde_json::Value) -> Planted {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let (repo, home, adapters) = (root.join("repo"), root.join("home"), root.join("adapters"));
        for path in [&repo, &home.join("profiles/headless"), &adapters] {
            std::fs::create_dir_all(path).unwrap();
        }
        let dsh = json!({
            "provider": "dsh",
            "trust_tier": "untrusted",
            "egress": "uncontracted",
            "binary": "dsh",
            "driver": ["{brokkr}", "driver", "dsh", "--"],
            "models": models,
            "credentials": credentials,
            "judges": [],
            "model_flag": "--model",
            "efforts": [],
            "effort_flag": "unsupported",
            "tool_permissions": "unsupported",
            "mcp": "unsupported",
            "native_capabilities": {"unmeasured": "the test's"},
            "hands": {"unsupported": "the test's"},
        });
        std::fs::write(adapters.join("dsh.json"), dsh.to_string()).unwrap();
        Planted {
            _dir: dir,
            repo,
            home,
            adapters,
        }
    }

    /// The scaffold's: two bare ids, both on dsh's default route.
    fn scaffold() -> Planted {
        let models = json!({"flash": "deepseek-flash", "pro": "deepseek-v4-pro"});
        Planted::new(models, json!({}))
    }

    /// Plant `text` at `path`, in a directory [`Planted::new`] made.
    fn write(&self, path: &Path, text: &str) {
        std::fs::write(path, text).unwrap();
    }

    /// The headless profile's patch, which dsh composes first.
    fn profile(&self) -> PathBuf {
        self.home.join("profiles/headless/cordis.patch.yml")
    }

    /// The home's own patch, which dsh composes after the profile's.
    fn home_patch(&self) -> PathBuf {
        self.home.join("cordis.patch.yml")
    }

    /// The check's answers, `set` the environment's.
    fn reaches(&self, set: fn(&str) -> bool) -> Vec<Reach> {
        answers(&self.adapters, &self.repo, &self.home, set)
    }

    /// [`Planted::reaches`] with no variable set, [`within_bound`].
    fn reaches_within_bound(&self) -> Vec<Reach> {
        let (adapters, repo, home) = (self.adapters.clone(), self.repo.clone(), self.home.clone());
        within_bound(move || answers(&adapters, &repo, &home, unset))
    }

    /// The check's lines with their caveats, `set` the environment's,
    /// [`within_bound`].
    fn lines_within_bound(&self, set: fn(&str) -> bool) -> Vec<Line> {
        let (adapters, repo, home) = (self.adapters.clone(), self.repo.clone(), self.home.clone());
        within_bound(move || with_caveats(asked(&adapters, &repo, Ok(home), set), &repo))
    }

    /// The lines of a compile's `seats`, `set` the environment's,
    /// [`within_bound`].
    fn modelled_within_bound(&self, seats: home::Seats, set: fn(&str) -> bool) -> Vec<Line> {
        let (adapters, repo, home) = (self.adapters.clone(), self.repo.clone(), self.home.clone());
        within_bound(move || {
            let sources = Sources {
                set,
                repo: &repo,
                home: Ok(home),
            };
            with_dsh(&adapters, |dsh| home::modelled(dsh, &seats, &sources))
        })
    }
}

/// `check` on a thread of its own, refused past a bound: a check that
/// opened a FIFO planted as a `.env` or a patch would wait on it for a
/// writer that never comes.
fn within_bound<T: Send + 'static>(check: impl FnOnce() -> T + Send + 'static) -> T {
    let (sent, answered) = std::sync::mpsc::channel();
    std::thread::spawn(move || sent.send(check()));
    let answer = answered.recv_timeout(Duration::from_secs(10));
    answer.expect("the check opened a planted FIFO and waited on it")
}

/// The check's answers for a folder's `adapters`, `repo` and dsh `home`.
fn answers(adapters: &Path, repo: &Path, home: &Path, set: fn(&str) -> bool) -> Vec<Reach> {
    let answers = asked(adapters, repo, Ok(home.to_path_buf()), set);
    answers.into_iter().map(|(reach, _)| reach).collect()
}

/// [`answers`] with what each route's check asked, `home` dsh's home or
/// why none was resolved.
fn asked(
    adapters: &Path,
    repo: &Path,
    home: Result<PathBuf, CompositeError>,
    set: fn(&str) -> bool,
) -> Vec<(Reach, Option<Asked>)> {
    in_folder(adapters, &Sources { set, repo, home })
}

fn unset(_: &str) -> bool {
    false
}

/// A FIFO at `path`, which an open for reading waits on, made by
/// `mkfifo(1)` because rustix offers `mknodat` on Linux alone.
fn fifo(path: &Path) {
    let made = std::process::Command::new("mkfifo")
        .args(["-m", "600"])
        .arg(path)
        .status()
        .unwrap();
    assert_eq!(made.code(), Some(0), "mkfifo {}", path.display());
}

fn reached(source: Source) -> Vec<Reach> {
    vec![Reach::Reached {
        route: "deepseek-official".into(),
        variable: "DEEPSEEK_API_KEY".into(),
        source,
    }]
}

fn untold(why: Untold) -> Vec<Reach> {
    vec![Reach::Untold {
        route: "deepseek-official".into(),
        variable: "DEEPSEEK_API_KEY".into(),
        why,
    }]
}

/// Each line as both surfaces print it, which must never hold the value.
fn lines(reaches: &[Reach]) -> Vec<String> {
    let lines: Vec<String> = reaches
        .iter()
        .map(|reach| format!("{}: {}", reach.what(), reach.detail()))
        .collect();
    assert!(
        lines.iter().all(|line| !line.contains(PLANTED)),
        "{lines:?}"
    );
    lines
}

#[test]
fn a_scaffold_with_no_source_is_unreached_and_names_both_remedies() {
    let planted = Planted::scaffold();
    let (repo, home) = (planted.repo.display(), planted.home.display());
    assert_eq!(
        lines(&planted.reaches(unset)),
        [format!(
            "dsh key deepseek-official: dsh reaches no DEEPSEEK_API_KEY: it is not set in the \
             launching environment, no .env file is present at {repo}/.env or {home}/.env, and \
             dsh's shipped headless bundle leaves its credentials service disabled; export \
             DEEPSEEK_API_KEY where brokkr runs, or write the row `- id: credentials` with \
             `  disabled: false` in {home}/profiles/headless/cordis.patch.yml and store \
             DEEPSEEK_API_KEY through dsh's credentials service"
        )]
    );
    // A patch that leaves the service disabled is no key either, and is
    // named as the layer that disables it.
    planted.write(&planted.profile(), "- id: credentials\n  disabled: true\n");
    let off = Off::Layer(planted.profile());
    assert_eq!(
        planted.reaches(unset),
        [Reach::Unreached {
            route: "deepseek-official".into(),
            variable: "DEEPSEEK_API_KEY".into(),
            files: vec![planted.repo.join(".env"), planted.home.join(".env")],
            off: Ok(off),
        }]
    );
}

#[test]
fn a_key_in_the_launching_environment_is_reached_and_not_echoed() {
    let planted = Planted::scaffold();
    let answers = planted.reaches(|name| name == "DEEPSEEK_API_KEY");
    assert_eq!(answers, reached(Source::Environment));
    assert_eq!(
        lines(&answers),
        [
            "dsh key deepseek-official: dsh reaches DEEPSEEK_API_KEY through the launching \
          environment"
        ]
    );
}

#[test]
fn a_present_dotenv_is_reported_unread_and_never_opened() {
    let planted = Planted::scaffold();
    let at_home = planted.home.join(".env");
    planted.write(&at_home, &format!("DEEPSEEK_API_KEY={PLANTED}\n"));
    let answers = planted.reaches_within_bound();
    assert_eq!(answers, untold(Untold::Dotenv(at_home.clone())));
    assert_eq!(
        lines(&answers),
        [format!(
            "dsh key deepseek-official: DEEPSEEK_API_KEY is not set in the launching environment, \
             and {} is present but unread: Brokkr does not read it, so it can't confirm \
             DEEPSEEK_API_KEY is in it",
            at_home.display()
        )]
    );
    // The repository's own `.env` is asked first, and a FIFO there, which
    // an open would wait on, or one no one may read, is told as one that
    // holds the key: the check never opens it.
    let in_repo = planted.repo.join(".env");
    fifo(&in_repo);
    assert_eq!(
        planted.reaches_within_bound(),
        untold(Untold::Dotenv(in_repo.clone()))
    );
    std::fs::remove_file(&in_repo).unwrap();
    planted.write(&in_repo, &format!("DEEPSEEK_API_KEY={PLANTED}\n"));
    std::fs::set_permissions(&in_repo, std::fs::Permissions::from_mode(0o000)).unwrap();
    assert_eq!(
        planted.reaches_within_bound(),
        untold(Untold::Dotenv(in_repo.clone()))
    );
    // A `.env` whose directory can't be searched can't be looked up, and
    // counts as present. Root searches any directory, so the case is
    // asked only of a test that runs as another user.
    if !rustix::process::geteuid().is_root() {
        let mode = |mode| std::fs::Permissions::from_mode(mode);
        std::fs::set_permissions(&planted.repo, mode(0o000)).unwrap();
        let answers = planted.reaches_within_bound();
        std::fs::set_permissions(&planted.repo, mode(0o700)).unwrap();
        assert_eq!(answers, untold(Untold::Dotenv(in_repo)));
    }
}

#[test]
fn a_profile_that_re_enables_the_service_is_reached_through_it() {
    let planted = Planted::scaffold();
    let patch = planted.profile();
    planted.write(
        &patch,
        "- id: other\n- id: credentials\n  disabled: false\n",
    );
    let answers = planted.reaches(unset);
    assert_eq!(answers, reached(Source::Service(patch.clone())));
    assert_eq!(
        lines(&answers),
        [format!(
            "dsh key deepseek-official: dsh reaches DEEPSEEK_API_KEY through dsh's credentials \
             service, which {} re-enables; its store is dsh's to answer for and is not read",
            patch.display()
        )]
    );
}

#[test]
fn the_home_patch_composes_after_the_profile_patch() {
    let planted = Planted::scaffold();
    let (profile, home) = (planted.profile(), planted.home_patch());
    planted.write(&profile, "- id: credentials\n  disabled: false\n");
    planted.write(&home, "- id: credentials\n  disabled: true\n");
    let (repo_env, home_env) = (planted.repo.join(".env"), planted.home.join(".env"));
    assert_eq!(
        lines(&planted.reaches(unset)),
        [format!(
            "dsh key deepseek-official: dsh reaches no DEEPSEEK_API_KEY: it is not set in the \
             launching environment, no .env file is present at {} or {}, and {} disables dsh's \
             credentials service; export DEEPSEEK_API_KEY where brokkr runs, or write the row \
             `- id: credentials` with `  disabled: false` in {} and store DEEPSEEK_API_KEY \
             through dsh's credentials service",
            repo_env.display(),
            home_env.display(),
            home.display(),
            home.display()
        )]
    );
    planted.write(&profile, "- id: credentials\n  disabled: true\n");
    planted.write(&home, "- id: other\n- id: credentials\n  disabled: false\n");
    assert_eq!(
        planted.reaches(unset),
        reached(Source::Service(home.clone()))
    );
    // A home patch that writes nothing of the service leaves the
    // profile's answer standing.
    planted.write(&home, "- id: other\n  disabled: true\n");
    assert_eq!(
        planted.reaches(unset)[0],
        Reach::Unreached {
            route: "deepseek-official".into(),
            variable: "DEEPSEEK_API_KEY".into(),
            files: vec![repo_env, home_env],
            off: Ok(Off::Layer(profile)),
        }
    );
}

#[test]
fn a_patch_layer_that_cannot_be_read_is_untold_and_never_ok() {
    let planted = Planted::scaffold();
    let (profile, home) = (planted.profile(), planted.home_patch());
    // Invalid YAML after a good row.
    planted.write(
        &profile,
        "- id: credentials\n  disabled: false\nthis: is: not a row\n",
    );
    let answers = planted.reaches(unset);
    assert_eq!(
        answers,
        untold(Untold::Patch(profile.clone(), Unread::Line(3)))
    );
    assert_eq!(
        lines(&answers),
        [format!(
            "dsh key deepseek-official: DEEPSEEK_API_KEY is not set in the launching environment \
             and no .env file dsh reads is present, but whether a patch re-enables dsh's \
             credentials service can't be told: {}: line 3 is not a row Brokkr reads (a \
             top-level `- key: value` and its own `  key: value` members, each value one plain \
             scalar)",
            profile.display()
        )]
    );
    // A `credentials` row nested under a mapping key is not a top-level
    // row, and is not read past.
    planted.write(&profile, "config:\n- id: credentials\n  disabled: false\n");
    assert_eq!(
        planted.reaches(unset),
        untold(Untold::Patch(profile.clone(), Unread::Line(1)))
    );
    // A home patch that cannot be read is untold after an enabling
    // profile, and a FIFO in its place is never opened.
    planted.write(&profile, "- id: credentials\n  disabled: false\n");
    fifo(&home);
    assert_eq!(
        planted.reaches_within_bound(),
        untold(Untold::Patch(home, Unread::NotAFile))
    );
}

/// The directories a walk takes from a dsh home to its headless profile.
const PROFILE: &[&str] = &["profiles", "headless"];

/// `path`, a directory, opened as a walk holds it.
fn directory(path: &Path) -> std::os::fd::OwnedFd {
    let flags = rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::DIRECTORY;
    rustix::fs::open(path, flags, rustix::fs::Mode::empty()).unwrap()
}

#[test]
fn a_patch_layer_is_read_only_from_a_bounded_utf8_regular_file() {
    let planted = Planted::scaffold();
    let (home, patch) = (&planted.home, planted.profile());
    let profile = || layer(home, PROFILE, PATCH);
    assert_eq!(profile(), Ok(Layer::Absent));
    // A home, or a directory on the way, that is not there holds no layer.
    assert_eq!(layer(&home.join("absent"), &[], PATCH), Ok(Layer::Absent));
    assert_eq!(layer(home, &["absent"], PATCH), Ok(Layer::Absent));
    assert_eq!(layer(home, &[], "profiles"), Err(Unread::NotAFile));
    let not_a_directory = Err(Unread::Io(std::io::ErrorKind::NotADirectory));
    let dsh = planted.adapters.join("dsh.json");
    assert_eq!(
        layer(&planted.adapters, &["dsh.json"], PATCH),
        not_a_directory
    );
    assert_eq!(layer(&dsh, &[], PATCH), not_a_directory);
    // A file swapped in for a directory after its lookup fails to open.
    let held = directory(&planted.adapters);
    let swapped = opened(held.as_fd(), "dsh.json", Kind::Directory);
    assert_eq!(swapped.err(), not_a_directory.err());
    std::fs::write(&patch, [0xff, b'\n']).unwrap();
    assert_eq!(profile(), Err(Unread::NotUtf8));
    let large = "# a comment\n".repeat(64 * 1024 / 12 + 1);
    planted.write(&patch, &large);
    assert_eq!(profile(), Err(Unread::TooLarge));
    planted.write(&patch, "- id: credentials\n  disabled: false\n");
    assert_eq!(profile(), Ok(Layer::Read(Some(Write::Enables))));
    std::fs::set_permissions(&patch, std::fs::Permissions::from_mode(0o000)).unwrap();
    if !rustix::process::geteuid().is_root() {
        let denied = Err(Unread::Io(std::io::ErrorKind::PermissionDenied));
        assert_eq!(profile(), denied);
    }
    // A regular file whose read fails after its open is refused unread:
    // Linux's `/proc/self/mem` opens, and answers its first read with EIO.
    if cfg!(target_os = "linux") {
        let eio = Unread::Io(std::io::Error::from(rustix::io::Errno::IO).kind());
        assert_eq!(layer(Path::new("/proc/self"), &[], "mem"), Err(eio));
    }
    let words: Vec<String> = [
        Unread::Link,
        Unread::NotAFile,
        Unread::Io(std::io::ErrorKind::NotADirectory),
        Unread::TooLarge,
        Unread::NotUtf8,
        Unread::Repeated(3),
        Unread::NoId(2),
        Unread::Disabled(2),
    ]
    .iter()
    .map(ToString::to_string)
    .collect();
    assert_eq!(
        words,
        [
            "it is a symbolic link, which Brokkr never follows",
            "it is not a regular file",
            "it cannot be read (not a directory)",
            "it is over 65536 bytes",
            "it is not UTF-8",
            "line 3 repeats a key of its row",
            "the row on line 2 names no id",
            "line 2 gives `disabled` a value other than true or false",
        ]
    );
}

/// `planted`'s check, which tells `patch` a link and names it so, and
/// `link`, a symlink where a `kind` is walked, failing to open as one
/// swapped in after its lookup would.
fn refused_as_a_link(planted: &Planted, patch: &Path, link: &Path, kind: Kind) {
    let answers = planted.reaches_within_bound();
    let refused = Untold::Patch(patch.to_path_buf(), Unread::Link);
    assert_eq!(answers, untold(refused));
    assert_eq!(
        lines(&answers),
        [format!(
            "dsh key deepseek-official: DEEPSEEK_API_KEY is not set in the launching environment \
             and no .env file dsh reads is present, but whether a patch re-enables dsh's \
             credentials service can't be told: {}: it is a symbolic link, which Brokkr never \
             follows",
            patch.display()
        )]
    );
    let held = directory(link.parent().unwrap());
    let name = link.file_name().unwrap().to_str().unwrap();
    assert_eq!(opened(held.as_fd(), name, kind).err(), Some(Unread::Link));
}

/// A patch layer symlinked to a credentials file is untold at either
/// layer and never followed: the lookup refuses the link, and one swapped
/// in after the lookup fails to open. A hard link is a regular file, read
/// and refused at its first line, which is never echoed.
#[test]
fn a_patch_layer_symlinked_to_a_dotenv_is_untold_and_never_opened() {
    let planted = Planted::scaffold();
    let credentials = planted.repo.with_file_name("credentials");
    std::fs::create_dir(&credentials).unwrap();
    let dotenv = credentials.join(".env");
    planted.write(&dotenv, &format!("DEEPSEEK_API_KEY={PLANTED}\n"));
    for patch in [planted.profile(), planted.home_patch()] {
        std::os::unix::fs::symlink(&dotenv, &patch).unwrap();
        refused_as_a_link(&planted, &patch, &patch, Kind::File);
        std::fs::remove_file(&patch).unwrap();
    }
    std::fs::hard_link(&dotenv, planted.home_patch()).unwrap();
    let answers = planted.reaches_within_bound();
    let refused = Untold::Patch(planted.home_patch(), Unread::Line(1));
    assert_eq!(lines(&answers).len(), 1);
    assert_eq!(answers, untold(refused));
}

/// A patch layer symlinked to a FIFO returns at once, and a FIFO swapped
/// in after the lookup opens without waiting and is refused unread.
#[test]
fn a_patch_layer_symlinked_to_or_swapped_for_a_fifo_returns_at_once() {
    let planted = Planted::scaffold();
    let pipe = planted.repo.with_file_name("pipe");
    fifo(&pipe);
    std::os::unix::fs::symlink(&pipe, planted.home_patch()).unwrap();
    assert_eq!(
        planted.reaches_within_bound(),
        untold(Untold::Patch(planted.home_patch(), Unread::Link))
    );
    let held = directory(planted.repo.parent().unwrap());
    let swapped = within_bound(move || opened(held.as_fd(), "pipe", Kind::File).err());
    assert_eq!(swapped, Some(Unread::NotAFile));
}

/// A link at any directory below the dsh home is never followed: a
/// symlinked profile directory, or `profiles` itself, holding a planted
/// patch is refused as a link unread, and a directory swapped for a link
/// after its lookup fails to open. The home itself is the operator's root,
/// reached by the name dsh resolves, and a link there is followed.
#[test]
fn a_link_at_any_directory_below_the_dsh_home_is_never_followed() {
    let planted = Planted::scaffold();
    let credentials = planted.repo.with_file_name("credentials");
    std::fs::create_dir_all(credentials.join("headless")).unwrap();
    for dir in [credentials.clone(), credentials.join("headless")] {
        planted.write(&dir.join(PATCH), &format!("DEEPSEEK_API_KEY={PLANTED}\n"));
    }
    let profiles = planted.home.join("profiles");
    std::fs::remove_dir(profiles.join("headless")).unwrap();
    for (link, target) in [
        (profiles.join("headless"), credentials.clone()),
        (profiles.clone(), credentials.clone()),
    ] {
        if link == profiles {
            std::fs::remove_dir(&profiles).unwrap();
        }
        std::os::unix::fs::symlink(&target, &link).unwrap();
        refused_as_a_link(&planted, &planted.profile(), &link, Kind::Directory);
        std::fs::remove_file(&link).unwrap();
    }
    // An ordinary layout still reads, and a home reached through a link of
    // the operator's is walked from where it leads.
    std::fs::create_dir_all(profiles.join("headless")).unwrap();
    planted.write(&planted.profile(), "- id: credentials\n  disabled: false\n");
    assert_eq!(
        planted.reaches(unset),
        reached(Source::Service(planted.profile()))
    );
    let linked = planted.home.with_file_name("linked-home");
    std::os::unix::fs::symlink(&planted.home, &linked).unwrap();
    let through = linked.join("profiles/headless").join(PATCH);
    assert_eq!(
        answers(&planted.adapters, &planted.repo, &linked, unset),
        reached(Source::Service(through))
    );
}

/// Each route's caveat names only what its check asked: every source past
/// the environment where none answers, the `.env` names up to the first
/// present one and no patch, or the patches up to one that cannot be read,
/// each layer as absent, read, or refused and why.
#[test]
fn each_caveat_says_only_what_its_route_asked() {
    let planted = Planted::scaffold();
    let (repo_env, home_env) = (planted.repo.join(".env"), planted.home.join(".env"));
    let (profile, home_patch) = (planted.profile(), planted.home_patch());
    let caveat = |planted: &Planted| {
        let lines = planted.lines_within_bound(unset);
        let detail = lines[1].detail();
        let Line::Served {
            asked: Some(asked), ..
        } = lines.into_iter().nth(1).unwrap()
        else {
            panic!("no caveat with what was asked")
        };
        ((asked.files, asked.patches), detail)
    };
    let unknown = "Brokkr can't yet tell whether it serves this route from its own engine-only \
                   home (decision 0065), which reads neither patch nor the home's .env, so past \
                   the environment this check can't confirm the key reaches a seat served there";
    let looked_up = format!(
        "this check asked whether DEEPSEEK_API_KEY is set where brokkr runs, then looked up {} \
         and {} by name, unread",
        repo_env.display(),
        home_env.display()
    );
    let both = vec![repo_env.clone(), home_env.clone()];
    assert_eq!(
        caveat(&planted),
        (
            (
                both.clone(),
                vec![
                    AtLayer::Absent(profile.clone()),
                    AtLayer::Absent(home_patch.clone())
                ]
            ),
            format!(
                "{looked_up}, then found no {}, then found no {}; {unknown}",
                profile.display(),
                home_patch.display()
            )
        )
    );
    planted.write(&profile, "- id: other\n");
    assert_eq!(
        caveat(&planted).1,
        format!(
            "{looked_up}, then read {} for dsh's credentials row, then found no {}; {unknown}",
            profile.display(),
            home_patch.display()
        )
    );
    planted.write(&repo_env, &format!("DEEPSEEK_API_KEY={PLANTED}\n"));
    assert_eq!(
        caveat(&planted),
        (
            (vec![repo_env.clone()], Vec::new()),
            format!(
                "this check asked whether DEEPSEEK_API_KEY is set where brokkr runs, then looked \
                 up {} by name, unread; {unknown}",
                repo_env.display()
            )
        )
    );
    std::fs::remove_file(&repo_env).unwrap();
    planted.write(&profile, "not: a: row\n");
    assert_eq!(
        caveat(&planted).0,
        (
            both.clone(),
            vec![AtLayer::Refused(profile.clone(), Unread::Line(1))]
        )
    );
    std::fs::remove_file(&profile).unwrap();
    fifo(&home_patch);
    assert_eq!(
        caveat(&planted).1,
        format!(
            "{looked_up}, then found no {}, then refused {}: it is not a regular file; {unknown}",
            profile.display(),
            home_patch.display()
        )
    );
}

#[test]
fn only_a_recognised_last_credentials_row_writes_the_service() {
    let (enables, disables) = (Ok(Some(Write::Enables)), Ok(Some(Write::Disables)));
    for (patch, written) in [
        ("- id: credentials\n  disabled: false\n", enables.clone()),
        ("- disabled: false  \n  id: credentials\n", enables.clone()),
        (
            "- id: credentials\n  disabled: false\n- id: other\n  disabled: true\n",
            enables,
        ),
        (
            "# a comment\n- id: credentials\n  disabled: true\n\n- id: credentials\n",
            disables.clone(),
        ),
        (
            "- id: credentials\n  disabled: false\n- id: credentials\n  disabled: true\n",
            disables,
        ),
        ("- id: other\n  disabled: false\n", Ok(None)),
        ("  # only a comment\n", Ok(None)),
        ("", Ok(None)),
        (
            "- id: credentials\n  disabled: no\n",
            Err(Unread::Disabled(2)),
        ),
        (
            "- id: credentials\n  disabled: \"false\"\n",
            Err(Unread::Line(2)),
        ),
        ("- id: 'credentials'\n", Err(Unread::Line(1))),
        (
            "- id: credentials\n  disabled: false # on\n",
            Err(Unread::Line(2)),
        ),
        (
            "- id: other\n  config:\n    disabled: false\n",
            Err(Unread::Line(2)),
        ),
        (
            "- id: other\n  list:\n  - id: credentials\n",
            Err(Unread::Line(2)),
        ),
        (
            "  disabled: false\n- id: credentials\n",
            Err(Unread::Line(1)),
        ),
        (
            "- id: credentials\n   disabled: false\n",
            Err(Unread::Line(2)),
        ),
        ("- id: credentials\r\n", Err(Unread::Line(1))),
        ("\t- id: credentials\n", Err(Unread::Line(1))),
        ("---\n- id: credentials\n", Err(Unread::Line(1))),
        ("-\n", Err(Unread::Line(1))),
        ("- 9id: credentials\n", Err(Unread::Line(1))),
        ("- -id: credentials\n", Err(Unread::Line(1))),
        ("- : credentials\n", Err(Unread::Line(1))),
        ("- id: credentials\n  1st: x\n", Err(Unread::Line(2))),
        ("- id: credentials\n  : x\n", Err(Unread::Line(2))),
        ("- id: credentials\n---\n", Err(Unread::Line(2))),
        ("...\n- id: credentials\n", Err(Unread::Line(1))),
        ("- id: credentials\n...\n", Err(Unread::Line(2))),
        // A member this check does not read is admitted and passed over.
        (
            "- id: credentials\n  config_dir: x\n  api-key: y\n  disabled: false\n",
            Ok(Some(Write::Enables)),
        ),
        ("- id: credentials\n  na me: x\n", Err(Unread::Line(2))),
        ("- id: credentials\n  note: a: b\n", Err(Unread::Line(2))),
        (
            "- id: credentials\n  disabled: false\n  disabled: true\n",
            Err(Unread::Repeated(3)),
        ),
        (
            "- id: credentials\n- disabled: false\n",
            Err(Unread::NoId(2)),
        ),
    ] {
        assert_eq!(writes(patch), written, "{patch:?}");
    }
}

#[test]
fn with_no_dsh_home_only_the_repository_is_asked_and_the_cause_is_named() {
    let planted = Planted::scaffold();
    planted.write(&planted.home.join(".env"), "DEEPSEEK_API_KEY=x\n");
    let cause = CompositeError::Config("no dsh home: set DSH_HOME or HOME".to_string());
    let answers = asked(&planted.adapters, &planted.repo, Err(cause), unset);
    // Only the repository's `.env` is looked up, and no patch is read.
    let (reach, asked) = answers.into_iter().next().unwrap();
    assert_eq!(
        asked,
        Some(Asked {
            variable: "DEEPSEEK_API_KEY".into(),
            files: vec![planted.repo.join(".env")],
            patches: Vec::new(),
        })
    );
    let answers = [reach];
    assert_eq!(
        lines(&answers),
        [format!(
            "dsh key deepseek-official: dsh reaches no DEEPSEEK_API_KEY: it is not set in the \
             launching environment, no .env file is present at {}/.env, and no dsh home was \
             resolved (the DSH layout is unreadable: no dsh home: set DSH_HOME or HOME), so no \
             patch re-enables dsh's credentials service; export DEEPSEEK_API_KEY where brokkr \
             runs, or write the row `- id: credentials` with `  disabled: false` in the headless \
             profile's cordis.patch.yml and store DEEPSEEK_API_KEY through dsh's credentials \
             service",
            planted.repo.display()
        )]
    );
}

#[test]
fn a_seat_served_from_the_operators_home_reads_its_seams_home_or_cause() {
    // The reader asks the environment raw, so no test may write it meanwhile.
    let _env = crate::tests::env_guard::EnvGuard::lock();
    let (repo, home) = (Path::new("/repo"), PathBuf::from("/home/.dsh"));
    let seams = DshSeams {
        executable: "/bin/dsh".into(),
        home: home.clone(),
        node: None,
        head: Vec::new(),
    };
    let served = Sources::served(repo, Ok(seams));
    assert_eq!(
        (served.repo, served.home, (served.set)("CARGO_MANIFEST_DIR")),
        (repo, Ok(home), true)
    );
    let cause = CompositeError::Config("unresolved".into());
    assert_eq!(Sources::served(repo, Err(cause.clone())).home, Err(cause));
}

#[test]
fn each_route_reads_its_declared_variable_once_and_an_unnamed_one_is_undeclared() {
    let models = json!({
        "a": "deepseek-official/deepseek-flash",
        "b": "deepseek-flash",
        "c": "dashscope/glm-5.2",
        "d": "dashscope/qwen3.8-max",
        "e": "meta/meta/muse-spark-1.3",
    });
    let planted = Planted::new(models, json!({"dashscope": "DASHSCOPE_API_KEY"}));
    let answers = planted.reaches(|name| name != "SPARK_API_KEY");
    let route = |route: &str, variable: &str| Reach::Reached {
        route: route.into(),
        variable: variable.into(),
        source: Source::Environment,
    };
    assert_eq!(
        answers,
        [
            route("dashscope", "DASHSCOPE_API_KEY"),
            route("deepseek-official", "DEEPSEEK_API_KEY"),
            Reach::Undeclared {
                route: "meta".into()
            },
        ]
    );
    assert_eq!(
        lines(&answers[2..]),
        [
            "dsh key meta: the dsh adapter names no credential variable for this route, so \
          whether dsh reaches a key for it is unknown; declare one under the adapter's \
          `credentials`"
        ]
    );
}

/// Each route carries the served-home caveat after its answer, an
/// undeclared one saying nothing of the home was asked, and a `.env` in the
/// repository, a FIFO or one no one may read, warns last and is never
/// opened.
#[test]
fn each_route_carries_the_served_home_caveat_and_a_workdir_dotenv_warns() {
    let models = json!({
        "a": "deepseek-flash",
        "b": "dashscope/glm-5.2",
        "c": "meta/meta/muse-spark-1.3",
    });
    let planted = Planted::new(models, json!({"dashscope": "DASHSCOPE_API_KEY"}));
    let key = |route: &str, variable: &str| {
        let source = Source::Environment;
        let (route, variable) = (route.to_string(), variable.to_string());
        // A key the environment gives stops the check there.
        let asked = Asked {
            variable: variable.clone(),
            files: Vec::new(),
            patches: Vec::new(),
        };
        let served = Line::Served {
            route: route.clone(),
            asked: Some(asked),
        };
        [
            Line::Key(Reach::Reached {
                route,
                variable,
                source,
            }),
            served,
        ]
    };
    let undeclared = Line::Key(Reach::Undeclared {
        route: "meta".into(),
    });
    let [dashscope, dashscope_served] = key("dashscope", "DASHSCOPE_API_KEY");
    let [deepseek, deepseek_served] = key("deepseek-official", "DEEPSEEK_API_KEY");
    let undeclared_served = Line::Served {
        route: "meta".into(),
        asked: None,
    };
    let mut expected = vec![
        dashscope,
        dashscope_served,
        deepseek,
        deepseek_served,
        undeclared,
        undeclared_served,
    ];
    let set = |_: &str| true;
    assert_eq!(planted.lines_within_bound(set), expected);
    let in_repo = planted.repo.join(".env");
    fifo(&in_repo);
    expected.push(Line::Workdir(in_repo.clone()));
    assert_eq!(planted.lines_within_bound(set), expected);
    std::fs::remove_file(&in_repo).unwrap();
    planted.write(&in_repo, &format!("DEEPSEEK_API_KEY={PLANTED}\n"));
    std::fs::set_permissions(&in_repo, std::fs::Permissions::from_mode(0o000)).unwrap();
    let lines = planted.lines_within_bound(set);
    assert_eq!(lines, expected);
    let printed: Vec<String> = [&lines[3], &lines[5], &lines[6]]
        .iter()
        .map(|line| format!("{}: {}", line.what(), line.detail()))
        .collect();
    assert!(!printed.concat().contains(PLANTED), "{printed:?}");
    assert_eq!(
        printed,
        [
            "dsh home deepseek-official: this check asked only whether DEEPSEEK_API_KEY is set \
             where brokkr runs, and nothing of the operator's dsh home; Brokkr can't yet tell \
             whether it serves this route from its own engine-only home (decision 0065), which \
             reads neither patch nor the home's .env"
                .to_string(),
            "dsh home meta: this check asked nothing of the operator's dsh home, since no \
             variable is named for this route; Brokkr can't yet tell whether it serves this \
             route from its own engine-only home (decision 0065), which reads neither patch nor \
             the home's .env"
                .to_string(),
            format!(
                "dsh workdir: {} is present, unread: if Brokkr serves a dsh seat from its own \
                 engine-only home (decision 0065), it refuses that launch, since dsh would read \
                 the file for a key",
                in_repo.display()
            ),
        ]
    );
}

#[test]
fn a_folder_that_hires_no_dsh_or_whose_adapters_do_not_load_has_no_line() {
    let planted = Planted::scaffold();
    // A workdir `.env` is warned only beside a dsh seat.
    planted.write(&planted.repo.join(".env"), "");
    std::fs::remove_file(planted.adapters.join("dsh.json")).unwrap();
    assert_eq!(planted.reaches(unset), []);
    assert_eq!(of_folder(&planted.adapters, &planted.repo, None), []);
    assert_eq!(
        of_folder(&planted.repo.join("absent"), &planted.repo, None),
        []
    );
}

#[test]
fn a_variable_is_asked_whether_it_is_set_by_name_alone() {
    let _env = crate::tests::env_guard::EnvGuard::lock();
    assert!(set_and_not_empty("CARGO_MANIFEST_DIR"));
    assert!(!set_and_not_empty("BROKKR_717_NEVER_SET"));
    // A name no environment can hold is never set.
    assert!(!set_and_not_empty("CARGO_MANIFEST_DIR\0"));
}

/// The default key is the shipped dsh adapter's own for dsh's default
/// route (decision 0071 ruling 5).
#[test]
fn the_default_key_is_the_shipped_adapters_deepseek_official_credential() {
    let adapters = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters");
    let adapters = Adapters::load(&adapters).unwrap();
    let dsh = adapters.adapter("dsh").unwrap();
    assert_eq!(
        dsh.credentials.get("deepseek-official").map(String::as_str),
        Some(DEFAULT_KEY)
    );
}

/// The name of the host this test runs on, or of the other supported one
/// (decision 0063).
fn host_name(here: bool) -> &'static str {
    match cfg!(target_os = "macos") == here {
        true => "macos",
        false => "linux",
    }
}

/// A dsh scaffold written by `brokkr init --cli dsh` under a directory of
/// its own, recompiled with its dsh adapter's cold and replacement shapes
/// measured on the host this test runs on, and that compile: the compile a
/// Linux host makes of the shipped adapter, whose two shapes were measured
/// there (U0, U0c, and U0b's D06 and pass 8, U1d6), on Linux and macOS
/// alike.
fn scaffolded() -> (tempfile::TempDir, Bundle) {
    scaffolded_on(host_name(true))
}

/// [`scaffolded`] with its dsh adapter's two shapes measured on the host
/// this test does not run on: the compile a macOS host makes of the
/// shipped adapter, since a result from one host qualifies nothing on the
/// other.
fn scaffolded_elsewhere() -> (tempfile::TempDir, Bundle) {
    scaffolded_on(host_name(false))
}

/// A dsh scaffold whose adapter's cold and replacement shapes, which the
/// shipped adapter measured on Linux, are measured on `on`, and its
/// compile.
fn scaffolded_on(on: &str) -> (tempfile::TempDir, Bundle) {
    let (dir, compiled) = scaffold_compiled_on(on);
    (dir, compiled.unwrap())
}

/// [`scaffolded_on`], its compile as it ended.
fn scaffold_compiled_on(
    on: &str,
) -> (
    tempfile::TempDir,
    Result<Bundle, brokkr_runtime::bundle::CompileError>,
) {
    let dir = tempfile::tempdir().unwrap();
    let (root, path) = (dir.path(), std::ffi::OsStr::new(""));
    let dsh = Some(crate::init::Cli::Dsh);
    crate::init::init(root, path, "linux", root, dsh).unwrap();
    let folder = dir.path().join(".brokkr");
    let path = folder.join("adapters/dsh.json");
    let mut dsh: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    for shape in dsh["mcp"]["shapes"].as_array_mut().unwrap() {
        let host = &mut shape["measured_on"]["host"];
        assert_eq!(*host, json!("linux"));
        *host = json!(on);
    }
    std::fs::write(&path, dsh.to_string()).unwrap();
    let (agents, adapters) = (folder.join("agents"), folder.join("adapters"));
    let boundary = brokkr_core::realms::Boundary::Namespace;
    let compiled = Bundle::compile_under(&folder, &agents, &adapters, boundary);
    (dir, compiled)
}

/// The fence this host's compile seals: lifted on Linux (U9b1).
fn sealed_fence() -> SealedFence {
    match cfg!(target_os = "linux") {
        true => SealedFence::Lifted,
        false => SealedFence::Standing,
    }
}

/// The intent of an empty set under `fence` whose cold and replacement
/// shapes are both `assessed`, so each seat's cold and resumed launches
/// are served from one home, and every resume shape unmeasured.
fn intent(fence: SealedFence, assessed: SealedAssessment) -> SealedIsolation {
    SealedIsolation {
        cold: assessed.clone(),
        replacement: assessed,
        fence,
        ..SealedIsolation::default()
    }
}

/// A seat at `label` pinning `model`, its launch carrying an intent under
/// `fence` whose cold and replacement shapes are measured.
fn seat(label: &str, model: &str, routed: bool, fence: Option<SealedFence>) -> Seat {
    let isolation = fence.map(|fence| intent(fence, SealedAssessment::Measured));
    Seat {
        label: label.into(),
        model: model.into(),
        routed,
        isolation,
    }
}

/// Each dsh seat of a compile is read from its candidate's argv as the
/// launch reads it, with the fence the predicate dispatch seals by gives;
/// a dsh seat whose launch can't be read so, an inline dsh site or a
/// candidate pinning no `--model`, is named by its site, and every other
/// seat is still read.
#[test]
fn each_dsh_seat_of_a_compile_is_read_from_its_candidates_argv() {
    let (_dir, mut compiled) = scaffolded();
    let sealed = Some(sealed_fence());
    let (flash, pro) = ("deepseek-flash", "deepseek-v4-pro");
    let scaffold = vec![
        seat("implement", pro, false, sealed),
        seat("implement", flash, false, sealed),
        seat("intake", flash, false, sealed),
        seat("intake", pro, false, sealed),
    ];
    let read = |read: &[Seat], unread: &[&str]| home::Seats {
        read: read.to_vec(),
        unread: unread.iter().map(|site| site.to_string()).collect(),
    };
    assert_eq!(home::seats(&compiled), read(&scaffold, &[]));
    // A `--patch` is a route overlay, and a site whose plan holds no
    // outcome launches with no intent.
    let implement = compiled.sites.get_mut("implement").unwrap();
    implement.chain[0]
        .argv
        .extend(["--patch", "route.yml"].map(String::from));
    implement.capabilities = None;
    // An inline site that dsh does not serve, or that holds no plan, is
    // no dsh seat.
    let mut inline = compiled.sites["review"].clone();
    inline.chain.clear();
    compiled.sites.insert("inline".into(), inline);
    compiled.sites.insert("planless".into(), Default::default());
    let rerouted = [
        seat("implement", pro, true, None),
        seat("implement", flash, false, None),
    ];
    let mut seats = [&rerouted[..], &scaffold[2..]].concat();
    assert_eq!(home::seats(&compiled), read(&seats, &[]));
    let mut inline = compiled.sites["intake"].clone();
    inline.chain.clear();
    compiled.sites.insert("inline".into(), inline);
    assert_eq!(home::seats(&compiled), read(&seats, &["inline"]));
    let intake = compiled.sites.get_mut("intake").unwrap();
    intake.chain[1].argv.retain(|part| part != "--model");
    seats.pop();
    assert_eq!(home::seats(&compiled), read(&seats, &["inline", "intake"]));
}

/// Six seats, one for each way the served-home decision rules, and the
/// folder they are answered in: `a` and `b` on dsh's default route, which
/// an engine-only home was measured on with no overlay, under the standing
/// fence; `c` with no intent on a measured route; `d` past the lifted fence
/// on a measured route with its overlay, its route naming no variable; `e`
/// past the lifted fence on a route no home was measured on; and `f` with
/// no intent on an unmeasured route that names no variable.
struct Served {
    planted: Planted,
    seats: [Seat; 6],
}

impl Served {
    fn new() -> Served {
        let (standing, lifted) = (Some(SealedFence::Standing), Some(SealedFence::Lifted));
        Served {
            planted: Planted::new(json!({}), json!({"dashscope": "DASHSCOPE_API_KEY"})),
            seats: [
                seat("b", "deepseek-v4-pro", false, standing),
                seat("a", "deepseek-flash", false, standing),
                seat("c", "dashscope/glm-5.2", false, None),
                seat("d", "meta/meta/muse-spark-1.3", true, lifted),
                seat("e", "openai/gpt-6", false, lifted),
                seat("f", "spark/qwen3.8-flash", true, None),
            ],
        }
    }

    /// The seats' lines, `set` the environment's, [`within_bound`].
    fn homed(&self, set: fn(&str) -> bool) -> Vec<Line> {
        let planted = &self.planted;
        let (dir, repo) = (planted.adapters.clone(), planted.repo.clone());
        let (dsh_home, seats) = (planted.home.clone(), self.seats.to_vec());
        within_bound(move || {
            let adapters = Adapters::load(&dir).unwrap();
            let sources = Sources {
                set,
                repo: &repo,
                home: Ok(dsh_home),
            };
            home::homed(adapters.adapter("dsh").unwrap(), &seats, &sources)
        })
    }
}

/// The home line of `seats` on `route`, both of whose launches `home`
/// serves.
fn group(route: &str, seats: &[&str], home: Home) -> Line {
    split(route, seats, home.clone(), home)
}

/// The home line of `seats` on `route`, whose cold launch `cold` serves and
/// whose resumed one `resumed` does.
fn split(route: &str, seats: &[&str], cold: Home, resumed: Home) -> Line {
    Line::Home {
        route: route.into(),
        seats: seats.iter().map(|seat| seat.to_string()).collect(),
        cold,
        resumed,
    }
}

fn undeclared(route: &str) -> Line {
    Line::Key(Reach::Undeclared {
        route: route.into(),
    })
}

/// `e`'s refusal, in the driver's words.
const OPENAI: &str = "provider 'dsh' has no measured strict MCP configuration on route 'openai'";

/// Each seat is answered for the home brokkr-protocol's decision serves
/// it from, grouped by route and home: the operator's home asked of every
/// source, an engine-only home of the launching environment alone, and a
/// refused launch named with its refusal and no key.
#[test]
fn each_seat_is_answered_for_the_home_that_serves_it() {
    let served = Served::new();
    let planted = &served.planted;
    let deepseek = || (String::from("deepseek-official"), "DEEPSEEK_API_KEY".into());
    let (route, variable) = deepseek();
    let unreached = Reach::Unreached {
        route: "dashscope".into(),
        variable: "DASHSCOPE_API_KEY".into(),
        files: vec![planted.repo.join(".env"), planted.home.join(".env")],
        off: Ok(Off::Shipped(planted.profile())),
    };
    let mut expected = vec![
        Line::Key(unreached),
        group("dashscope", &["c"], Home::Operator),
        Line::Key(Reach::Reached {
            route,
            variable,
            source: Source::Environment,
        }),
        group("deepseek-official", &["a", "b"], Home::Engine),
        undeclared("meta"),
        group("meta", &["d"], Home::Engine),
        group("openai", &["e"], Home::Refused(OPENAI.into())),
        undeclared("spark"),
        group("spark", &["f"], Home::Operator),
    ];
    assert_eq!(served.homed(|name| name == "DEEPSEEK_API_KEY"), expected);
    let (route, variable) = deepseek();
    expected[2] = Line::Key(Reach::Unset { route, variable });
    let keyless = served.homed(unset);
    assert_eq!(keyless, expected);
    let printed = [&keyless[1], &keyless[2], &keyless[3], &keyless[6]]
        .map(|line| format!("{}: {}", line.what(), line.detail()));
    assert_eq!(
        printed,
        [
            "dsh home dashscope: Brokkr serves seats [c] on this route from the operator's own dsh \
             home, which reads every source this check asks",
            "dsh key deepseek-official: dsh reaches no DEEPSEEK_API_KEY: Brokkr serves this route \
             from its own engine-only home (decision 0065), which reads a key from the launching \
             environment alone, and DEEPSEEK_API_KEY is not set there; export DEEPSEEK_API_KEY \
             where brokkr runs",
            "dsh home deepseek-official: Brokkr serves seats [a, b] on this route from its own \
             engine-only home (decision 0065), where dsh reads a key from the launching \
             environment alone, never the operator's .env or patch layers",
            "dsh home openai: Brokkr refuses to launch seats [e] on this route: provider 'dsh' has \
             no measured strict MCP configuration on route 'openai'",
        ]
    );
}

/// Beside a repository `.env`, which is never opened, every engine-only
/// launch is refused and keyless, while the operator's home tells the file
/// present but unread.
#[test]
fn an_engine_only_launch_beside_a_repository_dotenv_is_refused_keyless() {
    let served = Served::new();
    let in_repo = served.planted.repo.join(".env");
    fifo(&in_repo);
    let workdir = Home::Workdir(in_repo.clone());
    let beside = vec![
        Line::Key(Reach::Untold {
            route: "dashscope".into(),
            variable: "DASHSCOPE_API_KEY".into(),
            why: Untold::Dotenv(in_repo.clone()),
        }),
        group("dashscope", &["c"], Home::Operator),
        group("deepseek-official", &["a", "b"], workdir.clone()),
        group("meta", &["d"], workdir),
        group("openai", &["e"], Home::Refused(OPENAI.into())),
        undeclared("spark"),
        group("spark", &["f"], Home::Operator),
    ];
    let lines = served.homed(unset);
    assert_eq!(lines, beside);
    assert_eq!(
        [format!("{}: {}", lines[2].what(), lines[2].detail())],
        [format!(
            "dsh home deepseek-official: Brokkr serves seats [a, b] on this route from its own \
                 engine-only home (decision 0065), and refuses each launch while {} is present, \
                 unread, since dsh would read it for a key; export the key where brokkr runs and \
                 remove the file",
            in_repo.display()
        ),]
    );
}

/// The folder's check models a compiled folder's seats at both their
/// launches, whatever the environment running the test gives: the
/// scaffold's ride dsh's default route with their cold and replacement
/// shapes both measured on this host (U1d6), so one engine-only home serves
/// a cold launch and a resumed one alike.
#[test]
fn a_compiled_folder_is_answered_by_the_homes_of_its_seats() {
    let (dir, compiled) = scaffolded();
    let adapters = dir.path().join(".brokkr/adapters");
    let mut lines = of_folder(&adapters, dir.path(), Some(&compiled));
    assert_eq!(lines.len(), 2, "{lines:?}");
    let line = lines.remove(1);
    let (route, seats) = ("deepseek-official", &["implement", "intake"][..]);
    assert_eq!(line, group(route, seats, Home::Engine));
    assert_eq!(
        format!("{}: {}", line.what(), line.detail()),
        "dsh home deepseek-official: Brokkr serves seats [implement, intake] on this route from \
         its own engine-only home (decision 0065), where dsh reads a key from the launching \
         environment alone, never the operator's .env or patch layers"
    );
}

/// A seat whose cold shape the driver admits and whose replacement shape
/// it does not is served from two homes (#717), and its key line stays the
/// engine-only home's, fail-closed: under the standing fence a resumed
/// launch is served from the operator's home, and past the lifted fence it
/// is refused in the admission's words. A seat admitted the other way
/// round is answered for its resumed launch's engine-only home.
#[test]
fn a_resumed_launch_the_admission_does_not_admit_is_named_beside_the_cold_one() {
    let planted = Planted::scaffold();
    let at = |label: &str, fence, cold, replacement| Seat {
        isolation: Some(SealedIsolation {
            cold,
            replacement,
            fence,
            ..SealedIsolation::default()
        }),
        ..seat(label, "deepseek-flash", false, None)
    };
    let (standing, lifted) = (SealedFence::Standing, SealedFence::Lifted);
    let (measured, unmeasured) = (
        || SealedAssessment::Measured,
        || SealedAssessment::Unmeasured,
    );
    let seats = home::Seats {
        read: vec![
            at("s", standing, measured(), unmeasured()),
            at("l", lifted, measured(), unmeasured()),
            at("r", standing, unmeasured(), measured()),
        ],
        unread: Default::default(),
    };
    let unset_key = || {
        Line::Key(Reach::Unset {
            route: "deepseek-official".into(),
            variable: "DEEPSEEK_API_KEY".into(),
        })
    };
    let refused = "provider 'dsh' has no measured strict MCP configuration for 'replacement'";
    let expected = vec![
        unset_key(),
        split("deepseek-official", &["r"], Home::Operator, Home::Engine),
        unset_key(),
        split("deepseek-official", &["s"], Home::Engine, Home::Operator),
        unset_key(),
        split(
            "deepseek-official",
            &["l"],
            Home::Engine,
            Home::Refused(refused.into()),
        ),
    ];
    let lines = planted.modelled_within_bound(seats, unset);
    assert_eq!(lines, expected);
    assert_eq!(
        format!("{}: {}", lines[5].what(), lines[5].detail()),
        "dsh home deepseek-official: Brokkr serves a cold launch of seats [l] on this route from \
         its own engine-only home (decision 0065), where dsh reads a key from the launching \
         environment alone, never the operator's .env or patch layers, and refuses a resumed \
         launch (provider 'dsh' has no measured strict MCP configuration for 'replacement')"
    );
}

/// The driver's admission rules each seat's home (#717): under the
/// standing fence a seat whose cold shape is unsupported or unmeasured is
/// served from the operator's home, as its launch is, and asked of every
/// source that home reads, beside a measured seat an engine-only home
/// serves; past the lifted fence the same shape is refused in the
/// admission's words.
#[test]
fn a_shape_the_admission_does_not_admit_is_served_as_its_launch_is() {
    let planted = Planted::scaffold();
    let (standing, lifted) = (SealedFence::Standing, SealedFence::Lifted);
    let unsupported = SealedAssessment::Unsupported("the test's".into());
    let at = |label: &str, isolation| Seat {
        isolation: Some(isolation),
        ..seat(label, "deepseek-flash", false, None)
    };
    let seats = home::Seats {
        read: vec![
            at("m", intent(standing, SealedAssessment::Measured)),
            at("s", intent(standing, unsupported.clone())),
            at("u", intent(standing, SealedAssessment::Unmeasured)),
            at("l", intent(lifted, unsupported)),
        ],
        unread: Default::default(),
    };
    let (route, variable) = (
        || "deepseek-official".to_string(),
        || "DEEPSEEK_API_KEY".to_string(),
    );
    let refused = "provider 'dsh' cannot exclude ambient MCP configuration (the test's)";
    let expected = vec![
        Line::Key(Reach::Unreached {
            route: route(),
            variable: variable(),
            files: vec![planted.repo.join(".env"), planted.home.join(".env")],
            off: Ok(Off::Shipped(planted.profile())),
        }),
        group("deepseek-official", &["s", "u"], Home::Operator),
        Line::Key(Reach::Unset {
            route: route(),
            variable: variable(),
        }),
        group("deepseek-official", &["m"], Home::Engine),
        group("deepseek-official", &["l"], Home::Refused(refused.into())),
    ];
    assert_eq!(planted.modelled_within_bound(seats, unset), expected);
}

/// The macOS case (#717): the shipped dsh adapter's cold and replacement
/// shapes were measured on Linux and qualify nothing on macOS, so each seat
/// of a scaffold compiled there carries both unmeasured and is served from
/// the operator's home, as its launch is; its key line asks every layer
/// that home reads, and none reads as an engine-only home's. On Linux,
/// past the fence U9b1 lifts, the same scaffold with its shapes measured
/// only on the other host refuses its first dsh site instead.
#[test]
fn a_scaffold_compiled_on_macos_is_served_from_the_operators_home() {
    if cfg!(target_os = "linux") {
        let (_dir, compiled) = scaffold_compiled_on(host_name(false));
        let refused = "seat 'implement' (office 'implementer') in realm '<unmapped>': provider \
                       'dsh' has no measured strict MCP configuration for 'cold'";
        let Err(brokkr_runtime::bundle::CompileError::Capability(cause)) = compiled else {
            panic!("SI2 refuses the scaffold: {:?}", compiled.map(drop));
        };
        return assert_eq!(cause, refused);
    }
    let (_dir, compiled) = scaffolded_elsewhere();
    let seats = home::seats(&compiled);
    let shapes: Vec<_> = (seats.read.iter())
        .map(|seat| {
            let isolation = seat.isolation.as_ref().unwrap();
            (&isolation.cold, &isolation.replacement)
        })
        .collect();
    let unmeasured = &SealedAssessment::Unmeasured;
    assert_eq!(shapes, [(unmeasured, unmeasured); 4]);
    let planted = Planted::scaffold();
    let lines = planted.modelled_within_bound(seats, unset);
    let printed = lines
        .iter()
        .map(|line| format!("{}: {}", line.what(), line.detail()));
    let (repo, home) = (planted.repo.join(".env"), planted.home.join(".env"));
    assert_eq!(
        printed.collect::<Vec<_>>(),
        [
            format!(
                "dsh key deepseek-official: dsh reaches no DEEPSEEK_API_KEY: it is not set in the \
                 launching environment, no .env file is present at {} or {}, and dsh's shipped \
                 headless bundle leaves its credentials service disabled; export DEEPSEEK_API_KEY \
                 where brokkr runs, or write the row `- id: credentials` with `  disabled: false` \
                 in {} and store DEEPSEEK_API_KEY through dsh's credentials service",
                repo.display(),
                home.display(),
                planted.profile().display()
            ),
            "dsh home deepseek-official: Brokkr serves seats [implement, intake] on this route \
             from the operator's own dsh home, which reads every source this check asks"
                .to_string(),
        ]
    );
}

/// A seat whose launch can't be read is named by its site and leaves every
/// read seat's home standing: an engine-only seat whose key the launching
/// environment does not give still reads MISSING, and beside a repository
/// `.env` still reads as its launch's refusal, while only the routes no
/// read seat rides are checked as an unmodelled folder's, with the caveat.
#[test]
fn an_unread_seat_leaves_every_read_seats_home_standing() {
    let models = json!({
        "a": "deepseek-flash",
        "b": "dashscope/glm-5.2",
        "c": "meta/meta/muse-spark-1.3",
    });
    let planted = Planted::new(models, json!({"dashscope": "DASHSCOPE_API_KEY"}));
    let seats = || home::Seats {
        read: vec![seat(
            "s",
            "deepseek-flash",
            false,
            Some(SealedFence::Standing),
        )],
        unread: ["inline".to_string()].into(),
    };
    let dashscope = |name: &str| name == "DASHSCOPE_API_KEY";
    let unread = || Line::Unmodelled(vec!["inline".into()]);
    let unmodelled = || {
        let variable = String::from("DASHSCOPE_API_KEY");
        vec![
            unread(),
            Line::Key(Reach::Reached {
                route: "dashscope".into(),
                variable: variable.clone(),
                source: Source::Environment,
            }),
            Line::Served {
                route: "dashscope".into(),
                asked: Some(Asked {
                    variable,
                    files: Vec::new(),
                    patches: Vec::new(),
                }),
            },
            undeclared("meta"),
            Line::Served {
                route: "meta".into(),
                asked: None,
            },
        ]
    };
    let unset_key = Line::Key(Reach::Unset {
        route: "deepseek-official".into(),
        variable: "DEEPSEEK_API_KEY".into(),
    });
    let engine = group("deepseek-official", &["s"], Home::Engine);
    let lines = planted.modelled_within_bound(seats(), dashscope);
    let expected: Vec<Line> = [unset_key, engine]
        .into_iter()
        .chain(unmodelled())
        .collect();
    assert_eq!(lines, expected);
    assert_eq!(
        format!("{}: {}", lines[2].what(), lines[2].detail()),
        "dsh home: Brokkr can't yet tell which home serves seats [inline], since the compile \
         carries no argv for an inline dsh site's launch and no model for a candidate that pins \
         no --model; each route no other seat rides is asked of every source the operator's home \
         reads, with its caveat"
    );
    let in_repo = planted.repo.join(".env");
    fifo(&in_repo);
    let refused = group("deepseek-official", &["s"], Home::Workdir(in_repo.clone()));
    let beside = [vec![refused], unmodelled(), vec![Line::Workdir(in_repo)]];
    let beside: Vec<Line> = beside.into_iter().flatten().collect();
    assert_eq!(planted.modelled_within_bound(seats(), dashscope), beside);
    // Where every route the adapter pins is ridden by a read seat, the
    // unread seats' line stands alone.
    let scaffold = Planted::scaffold();
    let operator = group("deepseek-official", &["s"], Home::Operator);
    let mut alone = seats();
    alone.read[0].isolation = None;
    let lines = scaffold.modelled_within_bound(alone, |_| true);
    assert_eq!(&lines[1..], [operator, unread()]);
}
