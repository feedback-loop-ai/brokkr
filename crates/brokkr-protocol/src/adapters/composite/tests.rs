use super::*;
use std::fs;
use std::path::Path;

fn write(dir: &Path, relative: &str, bytes: &[u8]) {
    let path = dir.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn plugin_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../extensions/dsh/plugin-cli-session")
}

fn digest_of(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(bytes))
}

#[test]
fn the_npm_key_rule_takes_only_the_terminal_package_spelling() {
    for (key, expected) in [
        ("node_modules/debug", "debug"),
        ("node_modules/body-parser/node_modules/debug", "debug"),
        ("node_modules/express/node_modules/debug", "debug"),
        (
            "node_modules/@smithy/node-http-handler",
            "@smithy/node-http-handler",
        ),
        (
            "node_modules/@aws-sdk/credential-provider-http/node_modules/@smithy/node-http-handler",
            "@smithy/node-http-handler",
        ),
        ("node_modules/@parent/pkg/node_modules/child", "child"),
        (
            "node_modules/a/node_modules/@parent/b/node_modules/@scope/child",
            "@scope/child",
        ),
    ] {
        assert_eq!(npm_name(key).unwrap(), expected, "{key}");
    }
}

#[test]
fn malformed_npm_keys_are_refused() {
    for key in [
        "",
        "/node_modules/a",
        "node_modules/",
        "node_modules/a/",
        "node_modules//a",
        "node_modules/a/extra/node_modules/@scope/child",
        "node_modules/.",
        "node_modules/..",
        "node_modules/@scope",
        "node_modules/@scope/",
        "node_modules/a/node_modules/",
        "node_modules/a b",
        "node_modules/a\\b",
        "node_modules/a@b",
    ] {
        assert!(npm_name(key).is_err(), "{key} should be refused");
    }
}

#[test]
fn npm_three_group_and_dedup_vectors_retain_distinct_triples() {
    let lock = r#"{"lockfileVersion":3,"packages":{
      "node_modules/@scope/child":{"version":"1.0.0","integrity":"sha512-AAA"},
      "node_modules/a/node_modules/@scope/child":{"version":"1.1.0","integrity":"sha512-BBB"},
      "node_modules/a/node_modules/@parent/b/node_modules/@scope/child":{"version":"2.0.0","integrity":"sha512-CCC"},
      "node_modules/x/node_modules/debug":{"version":"4.4.3","integrity":"sha512-DDD"},
      "node_modules/y/node_modules/debug":{"version":"4.4.3","integrity":"sha512-DDD"},
      "node_modules/body-parser/node_modules/debug":{"version":"4.4.3","integrity":"sha512-DDD"}
    }}"#;
    let deps = npm_dependencies(lock, &[]).unwrap();
    assert_eq!(
        deps,
        vec![
            "@scope/child 1.0.0 sha512-AAA".to_string(),
            "@scope/child 1.1.0 sha512-BBB".to_string(),
            "@scope/child 2.0.0 sha512-CCC".to_string(),
            "debug 4.4.3 sha512-DDD".to_string(),
        ]
    );
}

#[test]
fn the_hidden_npm_lock_is_the_sole_source_and_missing_fields_are_refused() {
    // The core and the local tarball installs are excluded by name; a
    // non-excluded entry with no registry integrity is unreadable.
    let lock = r#"{"lockfileVersion":3,"packages":{
      "node_modules/@deepseek-ai/dsh":{"version":"0.1.5-rc.1","integrity":"sha512-CORE"},
      "node_modules/dsh-plugin-cli-session":{"version":"0.2.0","resolved":"file:plugin.tgz","link":true},
      "node_modules/real":{"version":"1.0.0","integrity":"sha512-REAL"}
    }}"#;
    assert_eq!(
        npm_dependencies(lock, &["@deepseek-ai/dsh", "dsh-plugin-cli-session"]).unwrap(),
        vec!["real 1.0.0 sha512-REAL".to_string()]
    );
    assert!(npm_dependencies(lock, &["@deepseek-ai/dsh"]).is_err());

    // A root package-lock's `""` root entry is refused, so the plain lock
    // can never substitute for the hidden one.
    let root = r#"{"lockfileVersion":3,"packages":{
      "":{"name":"x","version":"1.0.0"},
      "node_modules/a":{"version":"1.0.0","integrity":"sha512-A"}}}"#;
    assert!(npm_dependencies(root, &[]).is_err());
    assert!(npm_dependencies(r#"{"lockfileVersion":3}"#, &[]).is_err());
}

#[test]
fn pnpm_reads_lockfile_9_and_normalizes_the_same_triples() {
    let lock = "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    \
                resolution: {integrity: sha512-X}\n\n  '@scope/child@1.0.0':\n    \
                resolution: {integrity: sha512-Y, tarball: https://example.invalid/x.tgz}\n\n  \
                dsh-plugin-cli-session@file:./plugin.tgz:\n    \
                resolution: {integrity: sha512-Z, tarball: file:./plugin.tgz}\n    version: 0.2.0\n";
    assert_eq!(
        pnpm_dependencies(lock, &["dsh-plugin-cli-session"]).unwrap(),
        vec![
            "@scope/child 1.0.0 sha512-Y".to_string(),
            "debug 2.6.9 sha512-X".to_string(),
        ]
    );
}

#[test]
fn npm_and_pnpm_agree_on_equivalent_entries() {
    let npm = r#"{"lockfileVersion":3,"packages":{"node_modules/debug":{"version":"2.6.9","integrity":"sha512-X"}}}"#;
    let pnpm = "lockfileVersion: '9.0'\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n";
    assert_eq!(
        npm_dependencies(npm, &[]).unwrap(),
        pnpm_dependencies(pnpm, &[]).unwrap()
    );
}

#[test]
fn unrecognized_pnpm_constructs_are_refused() {
    for lock in [
        "\tlockfileVersion: '9.0'\npackages:\n",
        "lockfileVersion: '9.0'\n# comment\npackages:\n",
        "lockfileVersion: '9.0'\npackages:\n\n  debug@2.6.9:\n    resolution:\n",
        "lockfileVersion: '9.0'\npackages:\n\n  debug:\n    resolution: {integrity: sha512-X}\n",
        "lockfileVersion: '9.0'\npackages:\n\n  debug@2.6.9:\n    engines: {node: '>=1'}\n",
        "lockfileVersion: '9.0'\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n    resolution: {integrity: sha512-Y}\n",
        "lockfileVersion: '9.0'\npackages:\n\n  debug@2.6.9:\n    resolution: {tarball: x}\n",
        "lockfileVersion: '9.1'\npackages:\n",
    ] {
        assert!(pnpm_dependencies(lock, &[]).is_err(), "{lock:?}");
    }
}

#[test]
fn the_plugin_component_is_bytewise_path_order_and_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    for file in PLUGIN_FILES {
        write(dir.path(), file, file.as_bytes());
    }
    let component = plugin_component(dir.path(), &PLUGIN_FILES).unwrap();
    let mut lines = String::new();
    for file in PLUGIN_FILES {
        lines.push_str(file);
        lines.push('\0');
        lines.push_str(&digest_of(file.as_bytes()));
        lines.push('\n');
    }
    assert_eq!(component, digest_of(lines.as_bytes()));
    assert_eq!(component.len(), 64);
    assert!(component
        .chars()
        .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)));

    // A nested node_modules/ subtree is excluded; an extra top-level entry
    // or a missing expected file is refused.
    let nested = tempfile::tempdir().unwrap();
    for file in PLUGIN_FILES {
        write(nested.path(), file, file.as_bytes());
    }
    write(nested.path(), "node_modules/dep/index.js", b"x");
    assert_eq!(
        plugin_component(nested.path(), &PLUGIN_FILES).unwrap(),
        component
    );
    write(nested.path(), "extra.txt", b"x");
    assert!(plugin_component(nested.path(), &PLUGIN_FILES).is_err());
    fs::remove_file(nested.path().join("LICENSE")).unwrap();
    assert!(plugin_component(nested.path(), &PLUGIN_FILES).is_err());
}

#[test]
fn the_committed_plugin_set_is_the_six_files_and_the_one_expression_delta() {
    let dir = plugin_dir();
    let found = plugin_file_digests(&dir).unwrap();
    let names: Vec<&str> = found.keys().map(String::as_str).collect();
    assert_eq!(names, PLUGIN_FILES.to_vec());

    let index = fs::read_to_string(dir.join("lib/index.js")).unwrap();
    let adapted = "\tconst events = agent.session.snapshotEvents(firstSeq);";
    assert_eq!(index.matches(adapted).count(), 1);
    let upstream = index.replace(adapted, "\tconst events = agent.session.events;");
    assert_eq!(
        digest_of(upstream.as_bytes()),
        "a40b52b3891485821ad01b00c322006abee8a51a0d4a2ae4ddb8427a0183d99b"
    );
}

#[allow(clippy::too_many_arguments)]
fn composite(
    core: &str,
    node: &str,
    npm: &[String],
    pnpm: &[String],
    plugin: &str,
    plugin_patch: &str,
    profile_patch: &str,
    bundles: &[String],
    reload: &str,
    home_patch: &str,
    extension: Option<&str>,
) -> String {
    canonical_composite(
        core,
        node,
        npm,
        pnpm,
        plugin,
        plugin_patch,
        profile_patch,
        bundles,
        reload,
        home_patch,
        extension,
    )
    .unwrap()
}

#[test]
fn the_canonical_composite_orders_lines_and_moves_with_its_inputs() {
    let npm = vec!["debug 4.4.3 sha512-D".to_string()];
    let pnpm = vec![
        "debug 4.4.3 sha512-D".to_string(),
        "zzz 1.0.0 sha512-Z".to_string(),
    ];
    let base = composite(
        "@deepseek-ai/dsh 0.1.5-rc.1 sha512-C",
        "v22.23.2",
        &npm,
        &pnpm,
        "aaaa",
        "bbbb",
        "cccc",
        &["base".to_string(), "headless".to_string()],
        "startup",
        "absent",
        None,
    );
    assert_eq!(base.len(), 64);

    // The npm/pnpm equal triple appears once: adding a pnpm duplicate of a
    // dependency changes nothing.
    let duplicate = vec![
        "debug 4.4.3 sha512-D".to_string(),
        "debug 4.4.3 sha512-D".to_string(),
        "zzz 1.0.0 sha512-Z".to_string(),
    ];
    assert_eq!(
        base,
        composite(
            "@deepseek-ai/dsh 0.1.5-rc.1 sha512-C",
            "v22.23.2",
            &npm,
            &duplicate,
            "aaaa",
            "bbbb",
            "cccc",
            &["base".to_string(), "headless".to_string()],
            "startup",
            "absent",
            None,
        )
    );

    // Reordered bundles, a switched reload mode, an added home patch and a
    // declared extension each move the composite.
    for moved in [
        composite(
            "@deepseek-ai/dsh 0.1.5-rc.1 sha512-C",
            "v22.23.2",
            &npm,
            &pnpm,
            "aaaa",
            "bbbb",
            "cccc",
            &["headless".to_string(), "base".to_string()],
            "startup",
            "absent",
            None,
        ),
        composite(
            "@deepseek-ai/dsh 0.1.5-rc.1 sha512-C",
            "v22.23.2",
            &npm,
            &pnpm,
            "aaaa",
            "bbbb",
            "cccc",
            &["base".to_string(), "headless".to_string()],
            "live",
            "absent",
            None,
        ),
        composite(
            "@deepseek-ai/dsh 0.1.5-rc.1 sha512-C",
            "v22.23.2",
            &npm,
            &pnpm,
            "aaaa",
            "bbbb",
            "cccc",
            &["base".to_string(), "headless".to_string()],
            "startup",
            "sha256-home",
            None,
        ),
        composite(
            "@deepseek-ai/dsh 0.1.5-rc.1 sha512-C",
            "v22.23.2",
            &npm,
            &pnpm,
            "aaaa",
            "bbbb",
            "cccc",
            &["base".to_string(), "headless".to_string()],
            "startup",
            "absent",
            Some("dddd"),
        ),
    ] {
        assert_ne!(base, moved);
    }

    // A value carrying a NUL or newline is refused rather than silently hashed.
    assert!(canonical_composite(
        "@deepseek-ai/dsh\0x",
        "v22.23.2",
        &npm,
        &pnpm,
        "aaaa",
        "bbbb",
        "cccc",
        &[],
        "startup",
        "absent",
        None,
    )
    .is_err());
}
