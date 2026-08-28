use super::{artifact_failures, artifact_problem};

fn classes(workdir: &std::path::Path, entries: &[&str]) -> Vec<(String, &'static str)> {
    let required: Vec<String> = entries.iter().map(|e| e.to_string()).collect();
    artifact_failures(workdir, &required)
}

#[test]
fn present_non_empty_file_passes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("spec.md"), "content").unwrap();
    assert!(classes(dir.path(), &["spec.md"]).is_empty());
}

#[test]
fn nested_path_passes_and_absent_nested_is_missing() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("docs")).unwrap();
    std::fs::write(dir.path().join("docs/plan.md"), "x").unwrap();
    assert!(classes(dir.path(), &["docs/plan.md"]).is_empty());
    assert_eq!(
        classes(dir.path(), &["docs/other.md"]),
        vec![("docs/other.md".to_string(), "missing")]
    );
}

#[test]
fn absent_file_is_missing() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        classes(dir.path(), &["spec.md"]),
        vec![("spec.md".to_string(), "missing")]
    );
}

#[test]
fn zero_byte_file_is_empty() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("spec.md"), "").unwrap();
    assert_eq!(
        classes(dir.path(), &["spec.md"]),
        vec![("spec.md".to_string(), "empty")]
    );
}

#[test]
fn directory_is_not_a_file() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("spec.md")).unwrap();
    assert_eq!(
        classes(dir.path(), &["spec.md"]),
        vec![("spec.md".to_string(), "not-a-file")]
    );
}

#[test]
fn lexical_predicate_fails_closed_as_invalid() {
    let dir = tempfile::tempdir().unwrap();
    // Traversal, absolute, current-dir component, backslash, NUL,
    // empty — none ever reaches the filesystem.
    for entry in [
        "../escape",
        "a/../b",
        "..",
        "/etc/hosts",
        "./spec.md",
        ".",
        "a\\b",
        "a\0b",
        "",
    ] {
        assert_eq!(
            classes(dir.path(), &[entry]),
            vec![(entry.to_string(), "invalid")],
            "entry {entry:?} must be invalid"
        );
    }
}

#[test]
fn reserved_characters_are_fenced() {
    let dir = tempfile::tempdir().unwrap();
    for entry in [
        "{slug}",
        "specs/{slug}/spec.md",
        "$HOME",
        "a<b",
        "a>b",
        "x}y",
    ] {
        assert_eq!(
            classes(dir.path(), &[entry]),
            vec![(entry.to_string(), "invalid")],
            "entry {entry:?} must be invalid"
        );
    }
}

#[cfg(unix)]
#[test]
fn symlinks_are_followed_and_dangling_reads_missing() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("real.md"), "content").unwrap();
    std::os::unix::fs::symlink(dir.path().join("real.md"), dir.path().join("link.md")).unwrap();
    std::os::unix::fs::symlink(dir.path().join("gone.md"), dir.path().join("dangling.md")).unwrap();
    assert!(
        classes(dir.path(), &["link.md"]).is_empty(),
        "content presence, not provenance"
    );
    assert_eq!(
        classes(dir.path(), &["dangling.md"]),
        vec![("dangling.md".to_string(), "missing")]
    );
}

#[test]
fn failures_keep_table_order_across_classes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("empty.md"), "").unwrap();
    std::fs::create_dir(dir.path().join("dir.md")).unwrap();
    std::fs::write(dir.path().join("ok.md"), "x").unwrap();
    assert_eq!(
        classes(
            dir.path(),
            &["gone.md", "empty.md", "{slug}", "ok.md", "dir.md"]
        ),
        vec![
            ("gone.md".to_string(), "missing"),
            ("empty.md".to_string(), "empty"),
            ("{slug}".to_string(), "invalid"),
            ("dir.md".to_string(), "not-a-file"),
        ]
    );
}

#[test]
fn problem_string_is_character_exact() {
    let failures = vec![
        ("spec.md".to_string(), "missing"),
        ("plan.md".to_string(), "empty"),
        ("{slug}".to_string(), "invalid"),
    ];
    assert_eq!(
        artifact_problem("ARCH-OK", &failures),
        "requires_artifacts unmet for rule ARCH-OK: \
             missing: spec.md; empty: plan.md; invalid: {slug}"
    );
    assert_eq!(
        artifact_problem("R1", &[("spec.md".to_string(), "missing")]),
        "requires_artifacts unmet for rule R1: missing: spec.md"
    );
}
