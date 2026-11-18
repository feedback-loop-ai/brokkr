//! A CLI that keeps its login in a file is given that file alone:
//! linked into the scratch HOME for each credentialed launch and absent
//! from the launch without credentials, so the auth refusal is still
//! measured, and a login the CLI rewrites, in place or by a rename,
//! reaches the operator's file.

use std::fs;
use std::os::unix::fs::PermissionsExt;

use super::*;

const AUTH: &str = ".codex/auth.json";

/// A Codex-like CLI logged in by `$HOME/.codex/auth.json` alone, which
/// its turn rewrites with `save`, as a CLI refreshing its login does.
fn codex_logged_in_by_file(save: &str) -> String {
    CODEX_LIKE.replace(
        r#"[ -n "$FAKE_TOKEN" ] || { echo "Not logged in. Run codex login." >&2; exit 1; }"#,
        &format!(
            r#"[ -f "$HOME/{AUTH}" ] || {{ echo "Not logged in. Run codex login." >&2; exit 1; }}
grep -q '"operator"' "$HOME/{AUTH}" && {{ {save}; }}"#
        ),
    )
}

/// An operator HOME holding a 0600 login file.
fn operator_home() -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    fs::create_dir_all(home.path().join(".codex")).unwrap();
    let file = home.path().join(AUTH);
    fs::write(&file, "{\"login\":\"operator\"}\n").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
    home
}

/// The probe of a Codex-like CLI logged in by the operator's file alone,
/// whose turn runs `save`, given the operator's HOME before and after.
fn probe_in(home: &tempfile::TempDir, files: &[&str], save: &str) -> Result<Report, ProbeError> {
    let world = world();
    let cli = world.fake("codex", &codex_logged_in_by_file(save));
    let files: Vec<CredentialFile> = files
        .iter()
        .map(|file| CredentialFile::bind(home.path(), file).unwrap())
        .collect();
    probe_given(
        AdapterKind::Codex,
        &cli,
        &codex_declared(),
        &[],
        &files,
        DEADLINE,
    )
}

fn probe_logged_in_by_file(save: &str) -> (Value, tempfile::TempDir) {
    let home = operator_home();
    let report = probe_in(&home, &[AUTH], save).unwrap();
    (serde_json::to_value(report).unwrap(), home)
}

fn refused_in(home: &tempfile::TempDir, files: &[&str], save: &str) -> String {
    let error = probe_in(home, files, save).unwrap_err();
    error.to_string()
}

#[test]
fn a_login_file_is_given_to_the_credentialed_launches_and_its_refresh_reaches_the_operator() {
    if !in_its_own_engine(
        "probe::tests::credential_file::a_login_file_is_given_to_the_credentialed_launches_and_its_refresh_reaches_the_operator",
    ) {
        return;
    }
    let save = r#"printf '{"login":"refreshed"}\n' > "$HOME/.codex/auth.json""#;
    let (report, home) = probe_logged_in_by_file(save);
    assert_eq!(
        report["facts"]["headless"]["evidence"],
        launched("~/.codex/auth.json", PROMPT_AS_ARGUMENT, "exit 0")
    );
    assert_eq!(
        report["facts"]["refusals"]["auth"],
        refusal(1, "Not logged in. Run codex login.")
    );
    let file = home.path().join(AUTH);
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        "{\"login\":\"refreshed\"}\n"
    );
    assert_eq!(
        fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn a_login_the_cli_renames_over_its_link_is_written_back_to_the_operator() {
    if !in_its_own_engine(
        "probe::tests::credential_file::a_login_the_cli_renames_over_its_link_is_written_back_to_the_operator",
    ) {
        return;
    }
    let save = r#"printf '{"login":"renamed"}\n' > "$HOME/.codex/auth.new" && mv "$HOME/.codex/auth.new" "$HOME/.codex/auth.json""#;
    let (report, home) = probe_logged_in_by_file(save);
    assert_eq!(report["facts"]["headless"]["value"]["exit"], 0);
    let file = home.path().join(AUTH);
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        "{\"login\":\"renamed\"}\n"
    );
    assert_eq!(
        fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn a_credential_file_must_be_a_private_regular_file_named_by_plain_names_under_home() {
    let home = operator_home();
    let bound = CredentialFile::bind(home.path(), AUTH).unwrap();
    assert_eq!(bound.name(), "~/.codex/auth.json");
    let refused = |relative: &str| {
        let error = CredentialFile::bind(home.path(), relative).unwrap_err();
        assert!(matches!(error, ProbeError::Credential(_)), "{error:?}");
        error.to_string()
    };
    for relative in [
        "",
        "/etc/passwd",
        "../outside",
        ".codex/../.codex/auth.json",
        "./.codex/auth.json",
    ] {
        assert_eq!(
            refused(relative),
            format!("credential file ~/{relative} is not a path of plain names under HOME")
        );
    }
    assert!(refused(".codex/missing.json")
        .starts_with("credential file ~/.codex/missing.json cannot be read: "));
    assert_eq!(
        refused(".codex"),
        "credential file ~/.codex is not a regular file"
    );
    fs::set_permissions(home.path().join(AUTH), fs::Permissions::from_mode(0o640)).unwrap();
    assert_eq!(
        refused(AUTH),
        "credential file ~/.codex/auth.json can be read or written by others than its owner; give it mode 0600"
    );
}

#[test]
fn a_login_file_the_cli_removes_leaves_the_operator_s_untouched() {
    if !in_its_own_engine(
        "probe::tests::credential_file::a_login_file_the_cli_removes_leaves_the_operator_s_untouched",
    ) {
        return;
    }
    let (report, home) = probe_logged_in_by_file(r#"rm "$HOME/.codex/auth.json""#);
    assert_eq!(report["facts"]["headless"]["value"]["exit"], 0);
    assert_eq!(
        fs::read_to_string(home.path().join(AUTH)).unwrap(),
        "{\"login\":\"operator\"}\n"
    );
}

#[test]
fn a_launch_that_leaves_the_link_unreadable_or_a_directory_refuses_the_probe() {
    if !in_its_own_engine(
        "probe::tests::credential_file::a_launch_that_leaves_the_link_unreadable_or_a_directory_refuses_the_probe",
    ) {
        return;
    }
    let home = operator_home();
    assert_eq!(
        refused_in(
            &home,
            &[AUTH],
            r#"rm "$HOME/.codex/auth.json" && mkdir "$HOME/.codex/auth.json""#
        ),
        "the launch left a directory where the credential file ~/.codex/auth.json was linked"
    );
    assert_eq!(
        refused_in(
            &home,
            &[AUTH],
            r#"rm -rf "$HOME/.codex" && touch "$HOME/.codex""#
        ),
        "could not unlink the credential file ~/.codex/auth.json: Not a directory (os error 20)"
    );
}

#[test]
fn a_login_that_cannot_be_written_back_refuses_the_probe() {
    if !in_its_own_engine(
        "probe::tests::credential_file::a_login_that_cannot_be_written_back_refuses_the_probe",
    ) {
        return;
    }
    let home = operator_home();
    let dir = home.path().join(".codex");
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o500)).unwrap();
    let refusal = refused_in(
        &home,
        &[AUTH],
        r#"printf '{}' > "$HOME/.codex/auth.new" && mv "$HOME/.codex/auth.new" "$HOME/.codex/auth.json""#,
    );
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(
        refusal.starts_with(
            "could not write ~/.codex/auth.json back: Permission denied (os error 13)"
        ),
        "{refusal}"
    );
    assert_eq!(
        fs::read_to_string(home.path().join(AUTH)).unwrap(),
        "{\"login\":\"operator\"}\n"
    );
}

#[test]
fn a_link_that_cannot_be_made_unlinks_those_made_before_it() {
    if !in_its_own_engine(
        "probe::tests::credential_file::a_link_that_cannot_be_made_unlinks_those_made_before_it",
    ) {
        return;
    }
    let home = operator_home();
    // The probe plants its own user configuration at this path, so the
    // second link meets a file already there.
    let planted = home.path().join(".codex/config.toml");
    fs::write(&planted, "").unwrap();
    fs::set_permissions(&planted, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(
        refused_in(&home, &[AUTH, ".codex/config.toml"], "true"),
        "could not link a credential file into the scratch HOME: File exists (os error 17)"
    );
    assert_eq!(
        fs::read_to_string(home.path().join(AUTH)).unwrap(),
        "{\"login\":\"operator\"}\n"
    );
}
