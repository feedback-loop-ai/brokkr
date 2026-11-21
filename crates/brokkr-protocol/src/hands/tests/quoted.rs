//! The one shell word a box test's tamper names its target by (#1187),
//! and the controls that prove it on the host's bash. Split from
//! evidence.rs, which sits at the 2000-line test ceiling.

use std::os::unix::ffi::OsStrExt;
use std::path::Path;

/// `path` as one bash word that names it byte for byte, in bash's `$'…'`
/// quoting: a printable ASCII byte other than a quote or a backslash
/// stands as itself, and every other byte is written `\xHH`. So a `$`, a
/// space, a quote, a backslash or a byte that is not UTF-8 in a temporary
/// directory never moves a tamper's target to another path.
pub(super) fn shell_quoted(path: &Path) -> String {
    let mut word = String::from("$'");
    for &byte in path.as_os_str().as_bytes() {
        if (b' '..=b'~').contains(&byte) && byte != b'\'' && byte != b'\\' {
            word.push(char::from(byte));
        } else {
            word.push_str(&format!("\\x{byte:02x}"));
        }
    }
    word.push('\'');
    word
}

/// What bash reads at `path`, quoted: its exit code and its output. Bash
/// parses the word as the box's own shell does, but runs here with a
/// cleared environment and no login profile, so no account's
/// configuration moves the command.
fn read_by_bash(path: &Path) -> (Option<i32>, Vec<u8>) {
    let read = std::process::Command::new("/bin/bash")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .arg("-c")
        .arg(format!("cat {}", shell_quoted(path)))
        .output()
        .unwrap();
    (read.status.code(), read.stdout)
}

/// A path whose name a shell would expand, split or unquote reaches the
/// one file it names once quoted, so a tamper is refused on its own
/// target.
#[test]
fn a_quoted_path_names_its_own_file_to_the_box_shell() {
    let dir = tempfile::tempdir().unwrap();
    let odd = dir.path().join("$BROKKR_1187_UNSET it's \\ here");
    std::fs::create_dir_all(&odd).unwrap();
    let artifact = odd.join("evidence");
    std::fs::write(&artifact, "evidence\n").unwrap();
    assert_eq!(read_by_bash(&artifact), (Some(0), b"evidence\n".to_vec()));
}

/// A path holding bytes that are not UTF-8 is quoted byte for byte, so
/// bash reads the file it names, never the one its lossy rendering names.
/// A tamper aimed through that rendering would meet "No such file or
/// directory" on another path and pass as the absent root's refusal.
#[cfg(target_os = "linux")]
#[test]
fn a_quoted_path_that_is_not_utf8_names_its_own_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let named = dir
        .path()
        .join(std::ffi::OsStr::from_bytes(b"not-utf8-\xff\xfe"));
    let lossy = dir
        .path()
        .join(named.file_name().unwrap().to_string_lossy().as_ref());
    std::fs::write(&named, "its own\n").unwrap();
    std::fs::write(&lossy, "another\n").unwrap();
    assert_ne!(named, lossy);
    assert!(shell_quoted(&named).is_ascii());
    assert_eq!(read_by_bash(&named), (Some(0), b"its own\n".to_vec()));
}
