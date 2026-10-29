//! #908: a bundle's manifest walk skips `.git` and the trees git ignores,
//! and a bundle past the walk's bound is refused by name before a run
//! starts.

use super::{error, workspace_root, Boundary, Bundle, Fixture};
use serde_json::json;
use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
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
    git(&fixture.root, &["init", "-q"]);
    std::fs::create_dir(fixture.root.join("target")).unwrap();
}

/// Compile `fixture` with `relative` holding `one`, then `two`: a path the
/// walk skips must leave the digest where it was. Returns the first compile.
fn stable_despite(fixture: &Fixture, relative: &str) -> Bundle {
    std::fs::write(fixture.root.join(relative), "one").unwrap();
    let compiled = fixture
        .compile(&Fixture::config(), &Fixture::policy())
        .unwrap();
    let digest = compiled.manifest_digest();
    std::fs::write(fixture.root.join(relative), "two").unwrap();
    let again = fixture
        .compile(&Fixture::config(), &Fixture::policy())
        .unwrap();
    assert_eq!(
        again.manifest_digest(),
        digest,
        "{relative} moved the digest"
    );
    compiled
}

#[test]
fn a_repository_root_bundle_pins_neither_git_nor_an_ignored_tree() {
    let fixture = Fixture::new();
    ignored_repository(&fixture);
    let compiled = stable_despite(&fixture, "target/artifact");
    let files = compiled.manifest["files"].as_object().unwrap();
    assert!(!files.keys().any(|key| key.starts_with(".git/")));
    assert!(!files.keys().any(|key| key.starts_with("target/")));
    assert!(files.contains_key("roles/role.md"));
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
    assert_eq!(
        refused,
        format!(
            "bundle: {}: seat 'work' names role 'target/role.md', which stands at \
             'target/role.md' — a path the bundle's file walk does not pin, because it holds a \
             tree git ignores in the bundle's repository, whose bytes are not part of the \
             bundle's identity. A charter there could change what the seat is told without \
             moving the bundle's identity, so it is refused; move it to a path the bundle pins, \
             such as 'roles/' (decision 0066 ruling 5)",
            fixture.root.join("bundle.json").display()
        )
    );
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
        format!(
            "bundle: {}: seat 'work' names role 'nested/target/role.md', which stands at \
             'nested/target/role.md' — a path the bundle's file walk does not pin, because it \
             holds a tree git ignores in the bundle's repository, whose bytes are not part of \
             the bundle's identity. A charter there could change what the seat is told without \
             moving the bundle's identity, so it is refused; move it to a path the bundle pins, \
             such as 'roles/' (decision 0066 ruling 5)",
            fixture.root.join("bundle.json").display()
        )
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
fn a_git_that_cannot_answer_the_ignore_question_refuses_by_name() {
    let fixture = Fixture::new();
    git(&fixture.root, &["init", "-q"]);
    // A real name git's pathspec grammar refuses: check-ignore answers an
    // error rather than yes or no. The compile must refuse by name, never
    // read the tree as pinned or as ignored.
    let magic = fixture.root.join(":(exclude)x");
    std::fs::create_dir(&magic).unwrap();
    std::fs::write(magic.join("role.md"), "# role").unwrap();
    let mut config = Fixture::config();
    config["seats"]["work"]["role"] = json!(":(exclude)x/role.md");
    let refused = error(fixture.compile(&config, &Fixture::policy()));
    assert_eq!(
        refused,
        format!(
            "bundle: {}: seat 'work' names role ':(exclude)x/role.md', asking git which trees \
             the bundle at {} ignores failed (git check-ignore answered exit status: 128); a \
             bundle's walk skips a git-ignored tree only when git says so, never by guessing. \
             A charter there could change what the seat is told without moving the bundle's \
             identity, so it is refused; move it to a path the bundle pins, such as 'roles/' \
             (decision 0066 ruling 5)",
            fixture.root.join("bundle.json").display(),
            fixture.root.display()
        )
    );
}

#[test]
fn a_walk_that_cannot_ask_git_refuses_by_name() {
    let fixture = Fixture::new();
    git(&fixture.root, &["init", "-q"]);
    let magic = fixture.root.join(":(exclude)x");
    std::fs::create_dir(&magic).unwrap();
    std::fs::write(magic.join("plain.txt"), "bytes").unwrap();
    let refused = error(fixture.compile(&Fixture::config(), &Fixture::policy()));
    assert_eq!(
        refused,
        format!(
            "bundle: asking git which trees the bundle at {} ignores failed (git check-ignore \
             answered exit status: 128); a bundle's walk skips a git-ignored tree only when git \
             says so, never by guessing",
            fixture.root.display()
        )
    );
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
