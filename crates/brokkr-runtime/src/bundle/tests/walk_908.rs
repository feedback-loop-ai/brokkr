//! #908: a bundle's manifest walk skips `.git` and the trees git ignores,
//! and a bundle past the walk's bound is refused by name before a run
//! starts.

use super::super::{CompileError, ReadStage, READ_HOOK};
use super::{error, workspace_root, Boundary, Bundle, Fixture};
use brokkr_core::canonical::sha256_bytes;
use serde_json::json;
use std::path::{Path, PathBuf};

// The fixture's git runs with the host's own configuration isolated, so a
// global or system `core.excludesFile` cannot decide what this test pins
// (#908, fix round 908e).
use crate::places::places_tests::{run as git_run, test_repository};

/// The child-process marker carrying the fixture root whose compile runs
/// under the hostile global configuration.
const HOSTILE_GLOBAL: &str = "BROKKR_WALK_908_HOSTILE_GLOBAL";

fn git(dir: &Path, args: &[&str]) {
    let out = git_run(dir, args);
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Compile the fixture with its own table and clear the read hook, whatever
/// the outcome, so a controlled replacement cannot leak to the next test.
fn compile(fixture: &Fixture) -> Result<Bundle, super::CompileError> {
    let result = fixture.compile(&Fixture::config(), &Fixture::policy());
    clear_hook();
    result
}

/// Clear the read hook.
fn clear_hook() {
    READ_HOOK.with(|hook| *hook.borrow_mut() = None);
}

/// Run `act` once, when a read of `watched` reaches `stage`.
fn on_stage(stage: ReadStage, watched: &Path, act: impl FnOnce(&Path) + 'static) {
    let watched = watched.to_path_buf();
    let mut once = Some(act);
    READ_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move |at, path| {
            if at == stage && path == watched.as_path() {
                if let Some(act) = once.take() {
                    act(path);
                }
            }
        }));
    });
}

/// The one interface pin for [`crate::places::SkipError`]'s display: both
/// the pinned-script lookup and the walk's own decision carry it.
fn git_unreadable(bundle: &Path, detail: &str) -> String {
    format!(
        "asking git which trees the bundle at {} ignores failed ({detail}); a bundle's walk \
         skips a git-ignored tree only when git says so, never by guessing",
        bundle.display()
    )
}

/// A fixture that is a git repository root whose `target/` is ignored.
fn ignored_repository(fixture: &Fixture) {
    std::fs::write(fixture.root.join(".gitignore"), "target/\n").unwrap();
    test_repository(&fixture.root);
    std::fs::create_dir(fixture.root.join("target")).unwrap();
}

/// A repository root whose index git cannot read: discovery still succeeds,
/// but check-ignore fails for real, with git's own explanation (#908).
fn unreadable_index(fixture: &Fixture) {
    test_repository(&fixture.root);
    std::fs::write(fixture.root.join("tracked"), "bytes").unwrap();
    git(&fixture.root, &["add", "tracked"]);
    std::fs::write(fixture.root.join(".git/index"), "garbage").unwrap();
}

/// The detail a failed check-ignore refusal carries, kept in one place:
/// git's status and its own stderr, through the house's `git_refusal`.
fn check_ignore_failure(bundle: &Path) -> String {
    git_unreadable(
        bundle,
        "git check-ignore answered exit status: 128: fatal: .git/index: index file smaller \
         than expected",
    )
}

/// Compile `fixture` with `relative` holding `one`, then `two`: the first
/// compile, and the digests the two reads produced.
fn recompiled(fixture: &Fixture, relative: &str) -> (Bundle, String, String) {
    std::fs::write(fixture.root.join(relative), "one").unwrap();
    let compiled = fixture
        .compile(&Fixture::config(), &Fixture::policy())
        .unwrap();
    let digest = compiled.manifest_digest();
    std::fs::write(fixture.root.join(relative), "two").unwrap();
    let again = fixture
        .compile(&Fixture::config(), &Fixture::policy())
        .unwrap();
    (compiled, digest, again.manifest_digest())
}

/// Compile `fixture` with `relative` holding `one`, then `two`: a path the
/// walk skips must leave the digest where it was. Returns the first compile.
fn stable_despite(fixture: &Fixture, relative: &str) -> Bundle {
    let (compiled, digest, again) = recompiled(fixture, relative);
    assert_eq!(again, digest, "{relative} moved the digest");
    compiled
}

/// Compile `fixture` with `relative` holding `one`, then `two`: a path the
/// walk pins must move the digest. Returns the first compile.
fn moved_by(fixture: &Fixture, relative: &str) -> Bundle {
    let (compiled, digest, again) = recompiled(fixture, relative);
    assert_ne!(again, digest, "{relative} did not move the digest");
    compiled
}

/// The refusal a seat's ignored charter gets, naming `relative` as both
/// the reference and the tree the walk does not pin.
fn ignored_charter(bundle: &Path, relative: &str) -> String {
    format!(
        "bundle: {}: seat 'work' names role '{relative}', which stands at '{relative}' — a \
         path the bundle's file walk does not pin, because it holds a tree git ignores in the \
         bundle's repository, whose bytes are not part of the bundle's identity. A charter \
         there could change what the seat is told without moving the bundle's identity, so it \
         is refused; move it to a path the bundle pins, such as 'roles/' (decision 0066 \
         ruling 5)",
        bundle.join("bundle.json").display()
    )
}

#[test]
fn a_repository_root_bundle_pins_neither_git_nor_an_ignored_tree() {
    let fixture = Fixture::new();
    ignored_repository(&fixture);
    let compiled = stable_despite(&fixture, "target/artifact");
    pinned_neither_git_nor_target(&compiled);
}

/// The bundle pinned the charter and skipped both `.git` and the ignored
/// `target/`.
fn pinned_neither_git_nor_target(compiled: &Bundle) {
    let files = compiled.manifest["files"].as_object().unwrap();
    assert!(!files.keys().any(|key| key.starts_with(".git/")));
    assert!(!files.keys().any(|key| key.starts_with("target/")));
    assert!(files.contains_key("roles/role.md"));
}

/// C4: a host's global `core.excludesFile` cannot decide what a fixture's
/// repository-root bundle pins. `test_repository` writes the repository's
/// own `core.excludesFile`, and git's precedence makes it win; this test
/// supplies the hostile global that would ignore `roles/` if that isolation
/// were dropped (#908, fix round 908e; decision 0071 ruling 9). The
/// configuration reaches only the child's compile, so no other test's git
/// can see it.
#[test]
fn a_repository_root_bundle_pins_neither_git_nor_an_ignored_tree_under_a_hostile_global() {
    if std::env::var_os(HOSTILE_GLOBAL).is_some() {
        return pinned_under_hostile_global();
    }
    let fixture = Fixture::new();
    ignored_repository(&fixture);
    std::fs::write(fixture.root.join("target/artifact"), "bytes").unwrap();
    fixture.declare(&Fixture::config(), &Fixture::policy());
    let (global, excludes) = hostile_global(&fixture);
    let this = format!(
        "{}::a_repository_root_bundle_pins_neither_git_nor_an_ignored_tree_under_a_hostile_global",
        module_path!().split_once("::").unwrap().1
    );
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([this.as_str(), "--exact", "--test-threads=1"])
        .env(HOSTILE_GLOBAL, &fixture.root)
        .env("GIT_CONFIG_GLOBAL", &global)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&child.stdout);
    let stderr = String::from_utf8_lossy(&child.stderr);
    assert!(
        child.status.success(),
        "a hostile global changed what the bundle pins: {} (global {global:?}, excludes \
         {excludes:?})\n{stdout}\n{stderr}",
        child.status
    );
    assert!(
        stdout.contains("1 passed"),
        "the child ran no test: {stdout}\n{stderr}"
    );
}

/// The child's side of the hostile-global test: the repository's own
/// `core.excludesFile` wins, so `roles/` is pinned and `target/` and `.git`
/// are still skipped.
fn pinned_under_hostile_global() {
    let root = PathBuf::from(std::env::var_os(HOSTILE_GLOBAL).expect("the child's fixture root"));
    let compiled = Bundle::compile(&root).expect("the repository's excludesFile wins");
    pinned_neither_git_nor_target(&compiled);
}

/// A global git configuration whose `core.excludesFile` ignores `roles/`.
/// Both files sit in the fixture's directory, which is the bundle's root,
/// so the walk pins them as ordinary entries; the tests that use them
/// assert only the charter and the trees the walk skips.
fn hostile_global(fixture: &Fixture) -> (PathBuf, PathBuf) {
    let excludes = fixture.dir.path().join("hostile-excludes");
    std::fs::write(&excludes, "roles/\n").unwrap();
    let global = fixture.dir.path().join("hostile-global");
    std::fs::write(
        &global,
        format!("[core]\n\texcludesFile = {}\n", excludes.display()),
    )
    .unwrap();
    (global, excludes)
}

#[test]
fn a_nested_tree_git_ignores_is_skipped_at_any_depth() {
    let fixture = Fixture::new();
    ignored_repository(&fixture);
    std::fs::create_dir_all(fixture.root.join("nested/target")).unwrap();
    let compiled = stable_despite(&fixture, "nested/target/artifact");
    let files = compiled.manifest["files"].as_object().unwrap();
    assert!(!files.keys().any(|key| key.starts_with("nested/target/")));
    assert!(files.contains_key("roles/role.md"));
}

#[test]
fn a_charter_inside_an_ignored_tree_is_refused_by_name() {
    let fixture = Fixture::new();
    ignored_repository(&fixture);
    std::fs::write(fixture.root.join("target/role.md"), "# role").unwrap();
    let mut config = Fixture::config();
    config["seats"]["work"]["role"] = json!("target/role.md");
    let refused = error(fixture.compile(&config, &Fixture::policy()));
    assert_eq!(refused, ignored_charter(&fixture.root, "target/role.md"));
}

#[test]
fn a_charter_inside_a_nested_ignored_tree_is_refused_by_name() {
    let fixture = Fixture::new();
    ignored_repository(&fixture);
    std::fs::create_dir_all(fixture.root.join("nested/target")).unwrap();
    std::fs::write(fixture.root.join("nested/target/role.md"), "# role").unwrap();
    let mut config = Fixture::config();
    config["seats"]["work"]["role"] = json!("nested/target/role.md");
    let refused = error(fixture.compile(&config, &Fixture::policy()));
    assert_eq!(
        refused,
        ignored_charter(&fixture.root, "nested/target/role.md")
    );
}

#[test]
fn a_second_name_inside_an_ignored_tree_is_refused_by_name() {
    for relative in ["target/role.md", "nested/target/role.md"] {
        let fixture = Fixture::new();
        ignored_repository(&fixture);
        let second = fixture.root.join(relative);
        std::fs::create_dir_all(second.parent().unwrap()).unwrap();
        std::fs::hard_link(fixture.root.join("roles/role.md"), &second).unwrap();
        let refused = error(fixture.compile(&Fixture::config(), &Fixture::policy()));
        assert_eq!(
            refused,
            format!(
                "bundle: bundle file '{relative}' is another name for 'roles/role.md', the file \
                 a bound read consumed: a layer's identity names a consumed file by the entry \
                 it was read by, and a second name for it inside the layer is refused rather \
                 than walked as a file nothing consumed (decision 0065 slice one, design D7)"
            ),
            "{relative}"
        );
    }
}

/// A gate whose unboxed exec command names `script`, compiled under the
/// harness boundary where the pinned-script lookup runs.
fn exec_gate_on(fixture: &Fixture, script: &str) -> Result<Bundle, super::CompileError> {
    let mut config = Fixture::config();
    config["seats"]["work"]["class"] = json!("gate");
    config["seats"]["work"]["hands"] = json!("workspace");
    config["seats"]["work"]["driver"] = json!({"command": [
        "{brokkr}", "driver", "exec", "--", "bash", script
    ]});
    std::fs::write(
        fixture.root.join("policy.json"),
        serde_json::to_vec(&Fixture::policy()).unwrap(),
    )
    .unwrap();
    std::fs::write(
        fixture.root.join("bundle.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    let root = workspace_root();
    Bundle::compile_under(
        &fixture.root,
        &root.join("agents"),
        &root.join("adapters"),
        Boundary::Harness,
    )
}

fn harness_refusal(script: &str) -> String {
    format!(
        "bundle: seat 'work' declares hands under the `harness` boundary, where no box stands, \
         so its exec command must name the bundle's own pinned script: {script} (decision 0046 \
         ruling 4; decision 0021)"
    )
}

#[test]
fn a_pinned_script_lookup_that_cannot_ask_git_refuses_by_name() {
    let holder = tempfile::tempdir().unwrap();
    let not_a_directory = holder.path().join("bundle");
    std::fs::write(&not_a_directory, "not a directory").unwrap();
    let refused = super::super::pinned_key(&not_a_directory, "./x/run.sh", "x/run.sh").unwrap_err();
    assert_eq!(
        refused,
        git_unreadable(&not_a_directory, "Not a directory (os error 20)")
    );
}

#[test]
fn a_pinned_script_inside_an_ignored_tree_is_refused_by_name() {
    let fixture = Fixture::new();
    ignored_repository(&fixture);
    for relative in ["target/run.sh", "nested/target/run.sh"] {
        let script = fixture.root.join(relative);
        std::fs::create_dir_all(script.parent().unwrap()).unwrap();
        std::fs::write(&script, "#!/bin/sh\ntrue\n").unwrap();
        let refused = match exec_gate_on(&fixture, &format!("./{relative}")) {
            Ok(_) => panic!("expected the ignored script {relative} to be refused"),
            Err(refusal) => refusal.to_string(),
        };
        assert_eq!(
            refused,
            harness_refusal(&format!(
                "'./{relative}' names a path the manifest walk does not pin ({relative} is in a \
                 tree git ignores in the bundle's repository, whose bytes are not part of the \
                 bundle's identity)"
            )),
            "{relative}"
        );
    }
}

#[test]
fn a_pinned_script_under_git_metadata_is_refused_by_name() {
    let fixture = Fixture::new();
    let refused =
        super::super::pinned_key(&fixture.root, "./.git/hooks/run.sh", ".git/hooks/run.sh")
            .unwrap_err();
    assert_eq!(
        refused,
        "'./.git/hooks/run.sh' names a path the manifest walk does not pin (.git/hooks/run.sh \
         is git's own metadata, which no bundle pins)"
    );
}

#[test]
fn a_bundle_outside_a_work_tree_pins_as_before_minus_git() {
    let fixture = Fixture::new();
    // No `.git` anywhere above the bundle: git is asked nothing, and the
    // walk skips only `.git` itself, as it did before #908.
    let compiled = fixture
        .compile(&Fixture::config(), &Fixture::policy())
        .unwrap();
    let files = compiled.manifest["files"].as_object().unwrap();
    assert!(files.contains_key("roles/role.md"));
}

#[test]
fn a_git_answer_that_cannot_be_read_refuses_the_bundle_by_name() {
    let fixture = Fixture::new();
    ignored_repository(&fixture);
    let compiled = stable_despite(&fixture, "target/artifact");
    assert!(!compiled.manifest["files"]
        .as_object()
        .unwrap()
        .contains_key("target/artifact"));
    // A repository git can no longer answer for must be refused by name,
    // with git's own stderr, never read as "nothing is ignored".
    std::fs::write(fixture.root.join(".git/config"), "[invalid\n").unwrap();
    let refused = error(fixture.compile(&Fixture::config(), &Fixture::policy()));
    let detail = format!(
        "asking git which trees the bundle at {} ignores failed (git rev-parse answered exit \
         status: 128: fatal: bad config line 1 in file .git/config)",
        fixture.root.display()
    );
    assert_eq!(
        refused,
        format!(
            "bundle: {}: the layer's declaring document, {detail}; a bundle's walk skips a \
             git-ignored tree only when git says so, never by guessing. What a run is composed \
             from must be what its identity names, so it is refused (decision 0065 slice one, \
             design D7)",
            fixture.root.join("bundle.json").display()
        )
    );
}

#[test]
fn git_metadata_under_a_work_tree_is_refused_by_its_own_reason() {
    let fixture = Fixture::new();
    ignored_repository(&fixture);
    std::fs::write(fixture.root.join(".git/role.md"), "# role").unwrap();
    let mut config = Fixture::config();
    config["seats"]["work"]["role"] = json!(".git/role.md");
    let refused = error(fixture.compile(&config, &Fixture::policy()));
    assert_eq!(
        refused,
        format!(
            "bundle: {}: seat 'work' names role '.git/role.md', which stands under '.git' — a \
             top-level name the bundle's file walk does not pin, because it holds git's own \
             metadata, which no bundle pins. A charter there could change what the seat is told \
             without moving the bundle's identity, so it is refused; move it to a path the \
             bundle pins, such as 'roles/' (decision 0066 ruling 5)",
            fixture.root.join("bundle.json").display()
        )
    );
}

#[test]
fn a_bundle_inside_git_metadata_is_no_work_tree() {
    let fixture = Fixture::new();
    ignored_repository(&fixture);
    let inside = fixture.root.join(".git/bundle");
    std::fs::create_dir(&inside).unwrap();
    assert!(crate::places::unpinned_reason(&inside, Path::new("roles"))
        .unwrap()
        .is_none());
}

#[test]
fn a_bundle_that_does_not_stand_answers_no_ignored_tree() {
    let holder = tempfile::tempdir().unwrap();
    let missing = holder.path().join("missing");
    assert!(crate::places::unpinned_reason(&missing, Path::new("roles"))
        .unwrap()
        .is_none());
}

#[test]
fn a_git_that_cannot_be_asked_refuses_by_name() {
    let holder = tempfile::tempdir().unwrap();
    let not_a_directory = holder.path().join("bundle");
    std::fs::write(&not_a_directory, "not a directory").unwrap();
    let refused = crate::places::unpinned_reason(&not_a_directory, Path::new("roles")).unwrap_err();
    assert!(matches!(refused, crate::places::SkipError::Git { .. }));
    assert_eq!(
        refused.to_string(),
        git_unreadable(&not_a_directory, "Not a directory (os error 20)")
    );
}

#[test]
fn a_check_ignore_failure_refuses_by_name_with_gits_own_words() {
    let fixture = Fixture::new();
    unreadable_index(&fixture);
    // A real git failure, not a pathspec: check-ignore answers an error
    // rather than yes or no. The refusal names the declaring document and
    // carries git's own explanation of what it could not read (#908).
    let refused = error(fixture.compile(&Fixture::config(), &Fixture::policy()));
    assert_eq!(
        refused,
        format!(
            "bundle: {}: the layer's declaring document, {}. What a run is composed from \
             must be what its identity names, so it is refused (decision 0065 slice one, \
             design D7)",
            fixture.root.join("bundle.json").display(),
            check_ignore_failure(&fixture.root)
        )
    );
}

#[test]
fn a_pathspec_shaped_name_is_judged_literally() {
    let fixture = Fixture::new();
    ignored_repository(&fixture);
    // `:(exclude)x` used to be pathspec magic to git, which answered an
    // error for a name the walk had. Judged literally, it is an ordinary
    // directory whose file the bundle pins and whose bytes move its
    // identity (#908).
    std::fs::create_dir(fixture.root.join(":(exclude)x")).unwrap();
    let compiled = moved_by(&fixture, ":(exclude)x/plain.txt");
    assert!(compiled.manifest["files"]
        .as_object()
        .unwrap()
        .contains_key(":(exclude)x/plain.txt"));
}

#[test]
fn a_name_beginning_with_a_colon_is_judged_literally() {
    let fixture = Fixture::new();
    test_repository(&fixture.root);
    // `foo` is ignored, but the file the walk names is `:foo`: judged as a
    // pathspec it would be omitted and its bytes would not move the
    // bundle's identity (#908).
    std::fs::write(fixture.root.join(".gitignore"), "foo\n").unwrap();
    let compiled = moved_by(&fixture, ":foo");
    assert!(compiled.manifest["files"]
        .as_object()
        .unwrap()
        .contains_key(":foo"));
}

#[test]
fn a_charter_under_a_colon_pathspec_is_refused_by_name() {
    let fixture = Fixture::new();
    test_repository(&fixture.root);
    // `.gitignore` ignores the literal directory `:roles/`; judged as a
    // pathspec the charter would be pinned instead, so its bytes could
    // change under the bundle's identity (#908).
    std::fs::write(fixture.root.join(".gitignore"), ":roles/\n").unwrap();
    std::fs::create_dir(fixture.root.join(":roles")).unwrap();
    std::fs::write(fixture.root.join(":roles/role.md"), "# role").unwrap();
    let mut config = Fixture::config();
    config["seats"]["work"]["role"] = json!(":roles/role.md");
    let refused = error(fixture.compile(&config, &Fixture::policy()));
    assert_eq!(refused, ignored_charter(&fixture.root, ":roles/role.md"));
}

#[test]
fn a_path_outside_the_layer_names_no_skip() {
    let holder = tempfile::tempdir().unwrap();
    let root = holder.path().join("layer");
    std::fs::create_dir(&root).unwrap();
    let skipped = |path: &Path| super::super::walk::skipped_path(&root, path).unwrap();
    assert_eq!(skipped(&holder.path().join("outside")), None);
    assert_eq!(skipped(&root), None);
    assert_eq!(skipped(&root.join("roles")), None);
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::ffi::OsStrExt;
        let invalid = std::ffi::OsStr::from_bytes(b"\xff");
        assert_eq!(skipped(&root.join(invalid)), None);
    }
}

#[test]
fn a_walked_file_that_vanishes_before_the_bound_is_refused() {
    let holder = tempfile::tempdir().unwrap();
    let walked = vec![(holder.path().join("gone"), "gone".to_string())];
    let refused = super::super::walk::refuse_oversized(holder.path(), &walked).unwrap_err();
    assert_eq!(
        refused.to_string(),
        "bundle io: No such file or directory (os error 2)"
    );
}

#[test]
fn a_bundle_past_the_file_bound_is_refused_before_a_run_starts() {
    let fixture = Fixture::new();
    std::fs::create_dir(fixture.root.join("bulk")).unwrap();
    for index in 0..5_001 {
        std::fs::write(fixture.root.join(format!("bulk/{index}")), "").unwrap();
    }
    let refused = error(fixture.compile(&Fixture::config(), &Fixture::policy()));
    assert_eq!(
        refused,
        format!(
            "bundle: the bundle at {} pins 5004 files, past the bound of 5000 files (#908): the \
             largest top-level trees are bulk/ (5001 files), bundle.json (1 files), policy.json \
             (1 files); it is refused before any run starts, so no run carries a manifest this \
             large (decision 0083 amendment)",
            fixture.root.display()
        )
    );
}

#[test]
fn a_bundle_past_the_byte_bound_is_refused_before_a_run_starts() {
    let fixture = Fixture::new();
    std::fs::create_dir(fixture.root.join("big")).unwrap();
    let blob = std::fs::File::create(fixture.root.join("big/blob")).unwrap();
    blob.set_len(64 * 1024 * 1024 + 1).unwrap();
    let refused = error(fixture.compile(&Fixture::config(), &Fixture::policy()));
    let written = |name: &str| std::fs::metadata(fixture.root.join(name)).unwrap().len();
    let (bundle, policy) = (written("bundle.json"), written("policy.json"));
    assert!(
        bundle > 6 && policy > 6,
        "the fixture files outgrow the role"
    );
    let (second, second_bytes, third, third_bytes) = if bundle >= policy {
        ("bundle.json", bundle, "policy.json", policy)
    } else {
        ("policy.json", policy, "bundle.json", bundle)
    };
    let total = 64 * 1024 * 1024 + 1 + bundle + policy + 6;
    assert_eq!(
        refused,
        format!(
            "bundle: the bundle at {} pins {total} bytes, past the bound of 67108864 bytes (64 \
             MiB) (#908): the largest top-level trees are big/ (67108865 bytes), {second} \
             ({second_bytes} bytes), {third} ({third_bytes} bytes); it is refused before any run \
             starts, so no run carries a manifest this large (decision 0083 amendment)",
            fixture.root.display()
        )
    );
}

/// The chief's regression: a file that grows past the bound after the
/// metadata preflight is refused on the bytes actually read, before any
/// `Bundle` is admitted or hashed (#908, fix round 908d).
#[test]
fn a_file_growing_after_the_pre_hash_bound_is_refused_chief_908() {
    let fixture = Fixture::new();
    let blob = fixture.root.join("blob");
    std::fs::write(&blob, "small").unwrap();
    let watched = blob.clone();
    let mut grown = Some(());
    READ_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move |stage, path| {
            if stage == ReadStage::Walked && path == watched.as_path() && grown.take().is_some() {
                std::fs::OpenOptions::new()
                    .write(true)
                    .open(path)
                    .unwrap()
                    .set_len(64 * 1024 * 1024 + 1)
                    .unwrap();
            }
        }));
    });
    // `error` panics on `Ok`, so this asserts that no `Bundle` is returned.
    let refused = error(fixture.compile(&Fixture::config(), &Fixture::policy()));
    READ_HOOK.with(|hook| *hook.borrow_mut() = None);
    assert_eq!(
        refused,
        format!(
            "bundle: the bundle at {} pins 67108865 bytes, past the bound of 67108864 bytes (64 \
             MiB) (#908): the largest top-level trees are blob (67108865 bytes); it is refused \
             before any run starts, so no run carries a manifest this large (decision 0083 \
             amendment)",
            fixture.root.display()
        )
    );
}

/// The regression's valid counterpart: a file that grows after the preflight
/// but stays within the bound is admitted, and the manifest pins the bytes
/// that were actually read (#908, fix round 908d).
#[test]
fn a_file_growing_within_the_bound_is_pinned_at_the_bytes_read() {
    let fixture = Fixture::new();
    let blob = fixture.root.join("blob");
    std::fs::write(&blob, "small").unwrap();
    let grown = b"grown after the size preflight".to_vec();
    let (watched, bytes) = (blob.clone(), grown.clone());
    let mut once = Some(());
    READ_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move |stage, path| {
            if stage == ReadStage::Walked && path == watched.as_path() && once.take().is_some() {
                std::fs::write(path, &bytes).unwrap();
            }
        }));
    });
    let compiled = fixture
        .compile(&Fixture::config(), &Fixture::policy())
        .unwrap();
    READ_HOOK.with(|hook| *hook.borrow_mut() = None);
    assert_eq!(
        compiled.manifest["files"]["blob"],
        json!(sha256_bytes(&grown))
    );
}

/// The regression's shrinking counterpart: a file rewritten while the walk
/// reads it is pinned at the bytes actually read, not the metadata the
/// preflight saw (#908, fix round 908e).
#[test]
fn a_file_that_shrinks_during_the_walk_is_pinned_at_its_bytes_read() {
    let fixture = Fixture::new();
    let blob = fixture.root.join("blob");
    std::fs::write(&blob, "a file that shrinks").unwrap();
    on_stage(ReadStage::Walked, &blob, |path| {
        std::fs::write(path, "x").unwrap();
    });
    let compiled = compile(&fixture).unwrap();
    assert_eq!(
        compiled.manifest["files"]["blob"],
        json!(sha256_bytes(b"x"))
    );
}

/// A file that vanishes between the preflight and the walk's read is refused
/// as the typed io fault it is, never pinned from the metadata that is now
/// stale (#908, fix round 908e). The module's one display pin is
/// [`a_walked_file_that_vanishes_before_the_bound_is_refused`].
#[test]
fn a_file_that_vanishes_during_the_walk_is_refused() {
    let fixture = Fixture::new();
    let watched = fixture.root.join("blob");
    std::fs::write(&watched, "small").unwrap();
    on_stage(ReadStage::Walked, &watched, |path| {
        std::fs::remove_file(path).unwrap();
    });
    match compile(&fixture) {
        Err(CompileError::Io(error)) => {
            assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
        }
        Err(other) => panic!("expected an io refusal, got {other:?}"),
        Ok(_) => panic!("expected compilation to fail"),
    }
}

/// C1: a consumed file is charged the length of the buffer whose digest it
/// supplied, never the metadata of an entry truncated after the read. The
/// 2 MiB charter is truncated at `Walked(blob)` and the blob grown to 63 MiB
/// there: the walk's charged bytes pass 64 MiB with the charter's pinned
/// length, and no `Bundle` is admitted (#908, fix round 908e).
#[test]
fn a_consumed_file_shrunk_during_the_walk_is_charged_its_pinned_bytes() {
    const MIB: u64 = 1024 * 1024;
    let fixture = Fixture::new();
    let role = fixture.root.join("roles/role.md");
    let blob = fixture.root.join("blob");
    let restore = fixture.root.join("z_restore");
    std::fs::write(&role, vec![b'r'; (2 * MIB) as usize]).unwrap();
    std::fs::write(&blob, "small").unwrap();
    std::fs::write(&restore, "trigger").unwrap();
    let (watched, watched_role, watched_restore) = (blob.clone(), role.clone(), restore.clone());
    let mut grown = Some(());
    let mut restored = Some(std::fs::read(&role).unwrap());
    READ_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move |stage, path| {
            if stage != ReadStage::Walked {
                return;
            }
            if path == watched.as_path() && grown.take().is_some() {
                std::fs::OpenOptions::new()
                    .write(true)
                    .open(path)
                    .unwrap()
                    .set_len(63 * MIB)
                    .unwrap();
                std::fs::write(&watched_role, b"").unwrap();
            }
            if path == watched_restore.as_path() {
                if let Some(bytes) = restored.take() {
                    std::fs::write(&watched_role, bytes).unwrap();
                }
            }
        }));
    });
    let refused = error(compile(&fixture));
    let written = |name: &str| std::fs::metadata(fixture.root.join(name)).unwrap().len();
    let (bundle, policy) = (written("bundle.json"), written("policy.json"));
    let (second, second_bytes) = if bundle >= policy {
        ("bundle.json", bundle)
    } else {
        ("policy.json", policy)
    };
    let total = 2 * MIB + 63 * MIB + bundle + policy;
    assert_eq!(
        refused,
        format!(
            "bundle: the bundle at {} pins {total} bytes, past the bound of 67108864 bytes (64 \
             MiB) (#908): the largest top-level trees are blob (66060288 bytes), roles/ \
             (2097152 bytes), {second} ({second_bytes} bytes); it is refused before any run \
             starts, so no run carries a manifest this large (decision 0083 amendment)",
            fixture.root.display()
        )
    );
}

/// C2: a bound read of an input stops at the layer's bound plus one byte and
/// refuses with the shared `past_bound` text; it never buffers the rest
/// (#908, fix round 908e).
#[test]
fn a_consumed_input_past_the_bound_is_refused_without_reading_it_all() {
    let fixture = Fixture::new();
    let role = fixture.root.join("roles/role.md");
    on_stage(ReadStage::Checked, &role, |path| {
        std::fs::OpenOptions::new()
            .write(true)
            .open(path)
            .unwrap()
            .set_len(64 * 1024 * 1024 + 2)
            .unwrap();
    });
    let refused = match super::super::bound_input(&fixture.root, "roles/role.md") {
        Err(super::super::InputFault::Place(place)) => place.to_string(),
        Err(super::super::InputFault::Missing(error)) => panic!("missing: {error}"),
        Ok(_) => panic!("expected the bound read to refuse"),
    };
    clear_hook();
    assert_eq!(
        refused,
        format!(
            "bundle: the bundle at {} pins 67108865 bytes, past the bound of 67108864 bytes (64 \
             MiB) (#908): the largest top-level trees are roles/ (67108865 bytes); it is refused \
             before any run starts, so no run carries a manifest this large (decision 0083 \
             amendment)",
            fixture.root.display()
        )
    );
}

/// C2: a held file grown past its pinned length is not intact, and the check
/// stops at that length plus one byte: the descriptor's position says how
/// much was read (#908, fix round 908e).
#[test]
fn a_held_file_grown_past_its_pinned_length_is_not_intact_without_reading_it_all() {
    use std::io::Seek;
    let fixture = Fixture::new();
    let bound = match super::super::bound_input(&fixture.root, "roles/role.md") {
        Ok(bound) => bound,
        Err(super::super::InputFault::Missing(error)) => panic!("missing: {error}"),
        Err(super::super::InputFault::Place(place)) => panic!("refused: {place}"),
    };
    let digest = sha256_bytes(&bound.bytes);
    let pinned = bound.bytes.len() as u64;
    std::fs::OpenOptions::new()
        .write(true)
        .open(fixture.root.join("roles/role.md"))
        .unwrap()
        .set_len(64 * 1024 * 1024 + 2)
        .unwrap();
    assert!(!bound.held.intact(&digest, pinned));
    let mut file = bound
        .held
        .handles
        .last()
        .expect("a resolution ends at a handle");
    assert_eq!(file.stream_position().unwrap(), pinned + 1);
}

// Linux filesystems allow arbitrary filename bytes; APFS and HFS+ refuse
// this top-level name at creation, so the branch is unreachable there.
#[cfg(target_os = "linux")]
#[test]
fn a_top_level_name_json_cannot_represent_is_refused_without_lossy_aliasing() {
    use std::os::unix::ffi::OsStringExt;

    let fixture = Fixture::new();
    let invalid = std::ffi::OsString::from_vec(b"\xff.txt".to_vec());
    std::fs::write(fixture.root.join(invalid), "bytes").unwrap();
    let refused = error(fixture.compile(&Fixture::config(), &Fixture::policy()));
    assert_eq!(
        refused,
        format!(
            "bundle: bundle filename '{}/�.txt' is not UTF-8; manifest keys must preserve exact \
             names",
            fixture.root.display()
        )
    );
}
