use super::super::{
    admit_program_name, errno_of, resolve_executable_in, CompositeError, Library, LIBRARY,
};
use super::*;
use std::ffi::OsString;
use std::fs;

#[test]
fn the_name_rule_turns_at_glibc_2_43() {
    assert!(!version_bounds_name("2.39"));
    assert!(!version_bounds_name("2.42"));
    assert!(version_bounds_name("2.43"));
    assert!(version_bounds_name("2.44.1"));
    assert!(version_bounds_name("3.0"));
    assert!(version_bounds_name("not a version"));
    // A name at NAME_MAX is never refused by its length.
    assert_eq!(admit_program_name(Library::Glibc, &"n".repeat(255)), Ok(()));
}

/// Under glibc 2.43 an overlong searched name is refused before the
/// search, so every PATH shape answers the same: the native child's
/// ENAMETOOLONG, and the resolver's name refusal. Each shape is the one
/// `child_controls` measures under the 2.42 rule.
#[test]
fn under_glibc_2_43_an_overlong_name_is_refused_before_any_entry() {
    if LIBRARY != Library::Glibc || !bounds_name() {
        eprintln!("not glibc 2.43 or later: the 2.42 rule is measured by child_controls");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let existing = dir.path().join("existing");
    fs::create_dir(&existing).unwrap();
    let nowhere = dir.path().join("nowhere");
    let file = dir.path().join("file");
    fs::write(&file, b"a file, not a directory\n").unwrap();
    let joined = |a: &std::path::Path, b: &std::path::Path| {
        OsString::from(format!("{}:{}", a.display(), b.display()))
    };
    let shapes = [
        ("an existing directory", existing.clone().into_os_string()),
        ("a missing directory", nowhere.clone().into_os_string()),
        ("a file", file.clone().into_os_string()),
        ("missing, then existing", joined(&nowhere, &existing)),
        ("the working directory", OsString::new()),
        (
            "only an oversized component",
            OsString::from("x".repeat(5000)),
        ),
    ];
    for name in ["x".repeat(300), "y".repeat(256)] {
        let refusal = CompositeError::Config(format!(
            "'{name}' {}",
            admit_program_name(Library::Glibc, &name).unwrap_err()
        ))
        .to_string();
        for (shape, path) in &shapes {
            let native = std::process::Command::new(&name)
                .env("PATH", path)
                .current_dir(dir.path())
                .status()
                .unwrap_err();
            assert_eq!(
                errno_of(&native),
                Some(rustix::io::Errno::NAMETOOLONG),
                "{} bytes under {shape}: {native}",
                name.len()
            );
            assert_eq!(
                resolve_executable_in(&name, Some(path.clone()))
                    .unwrap_err()
                    .to_string(),
                refusal,
                "{} bytes under {shape}",
                name.len()
            );
        }
    }
}
