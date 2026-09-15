use super::*;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

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

/// `valid_component` is the one gate every npm group component crosses.
/// Its forbidden-byte arms are reachable only when a caller hands it a
/// whole component, so they are exercised directly rather than through a
/// key the earlier guards already reject.
#[test]
fn the_component_gate_refuses_each_forbidden_byte() {
    for bad in ["a/b", "a\\b", "a\0b", "a b"] {
        assert!(!valid_component(bad), "{bad:?} must not be a component");
    }
    // The early key guard refuses a NUL before any group is parsed.
    assert!(npm_name("node_modules/a\0b").is_err());
}

/// A lock entry's own bytes decide its triple: an empty version and a
/// NUL-bearing integrity are refusals, not shorter or truncated values.
#[test]
fn npm_lock_entry_bytes_are_read_rather_than_shortened() {
    assert!(npm_dependencies(
        r#"{"packages":{"node_modules/a":{"version":"","integrity":"x"}}}"#,
        &[]
    )
    .is_err());
    assert!(npm_dependencies(
        "{\"packages\":{\"node_modules/a\":{\"version\":\"1\",\"integrity\":\"a\\u0000b\"}}}",
        &[]
    )
    .is_err());
}

/// A pnpm key shorter than two characters has no room for its `@`, and a
/// document marker is refused wherever it appears after the header.
#[test]
fn pnpm_refuses_a_short_key_and_a_document_marker() {
    let short = pnpm_dependencies(
        "lockfileVersion: '9.0'\npackages:\n  a:\n    resolution: {integrity: x}\n",
        &[],
    )
    .unwrap_err();
    assert!(
        short
            .to_string()
            .contains("no '@' after its first character"),
        "{short}"
    );
    for marker in ["---", "..."] {
        let lock = format!("lockfileVersion: '9.0'\npackages:\n  {marker}\n");
        let error = pnpm_dependencies(&lock, &[]).unwrap_err();
        assert!(
            error.to_string().contains("comment or document marker"),
            "{marker}: {error}"
        );
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

/// A synthetic qualified install: the task-owned core with its hidden
/// lock, the `headless` profile with its manifest, patch and pnpm lock,
/// and the committed plugin set under the profile. Everything is built
/// in a temporary directory; nothing here reads `.forge/`.
struct Synthetic {
    dir: tempfile::TempDir,
    seams: DshSeams,
}

impl Synthetic {
    fn new() -> Synthetic {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let core = root.join("core");
        let pkg = core.join("node_modules").join("@deepseek-ai").join("dsh");
        write(
            &pkg,
            "package.json",
            br#"{"name":"@deepseek-ai/dsh","version":"0.1.5-rc.1","bin":{"dsh":"lib/bin.js"}}"#,
        );
        write(&pkg, "lib/bin.js", b"#!/usr/bin/env node\n");
        write(
            &core,
            "node_modules/.package-lock.json",
            br#"{"lockfileVersion":3,"packages":{
              "node_modules/@deepseek-ai/dsh":{"version":"0.1.5-rc.1","integrity":"sha512-CORE"},
              "node_modules/dsh-plugin-cli-session":{"version":"0.2.0","resolved":"file:plugin.tgz","link":true},
              "node_modules/debug":{"version":"2.6.9","integrity":"sha512-DEBUG"}
            }}"#,
        );
        let home = root.join("home");
        let profile = home.join("profiles").join("headless");
        write(
            &profile,
            "package.json",
            br#"{"dsh":{"profile":{"bundles":["@deepseek-ai/dsh-base","dsh-plugin-cli-session"],"patchReload":"startup"}}}"#,
        );
        write(&profile, "cordis.patch.yml", b"[]\n");
        write(
            &profile,
            "pnpm-lock.yaml",
            b"lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-DEBUG}\n",
        );
        write(
            &profile,
            "node_modules/@deepseek-ai/dsh-base/package.json",
            br#"{"name":"@deepseek-ai/dsh-base","version":"0.1.5-rc.1"}"#,
        );
        for file in PLUGIN_FILES {
            write(
                &profile.join("node_modules").join("dsh-plugin-cli-session"),
                file,
                file.as_bytes(),
            );
        }
        let executable = pkg.join("lib/bin.js").to_string_lossy().into_owned();
        Synthetic {
            seams: DshSeams {
                executable,
                home: home.clone(),
            },
            dir,
        }
    }

    fn profile(&self) -> std::path::PathBuf {
        self.seams.home.join("profiles").join("headless")
    }

    fn node(&self) -> NodeRuntime {
        NodeRuntime {
            path: self.dir.path().join("node/bin/node"),
            version: "v22.23.2".to_string(),
        }
    }

    fn composite(&self) -> DshComposite {
        dsh_composite_with(&self.seams, &self.node(), &[]).unwrap()
    }
}

#[test]
fn npm_versions_with_any_whitespace_are_unreadable() {
    // D6 rejects a version containing NUL or any whitespace, not only an
    // empty or line-bearing one. Space, tab and CR are the three the
    // inherited empty/NUL/LF check left open.
    for version in [" 1.0.0", "1.0.0 ", "1\t0", "1\r0", "1.0.0\n", "\u{0}"] {
        let lock = format!(
            "{{\"lockfileVersion\":3,\"packages\":{{\"node_modules/a\":{{\"version\":{},\"integrity\":\"sha512-A\"}}}}}}",
            serde_json::to_string(version).unwrap()
        );
        assert!(npm_dependencies(&lock, &[]).is_err(), "{version:?}");
    }
    // A clean version still reads.
    let clean = r#"{"lockfileVersion":3,"packages":{"node_modules/a":{"version":"1.0.0","integrity":"sha512-A"}}}"#;
    assert_eq!(
        npm_dependencies(clean, &[]).unwrap(),
        vec!["a 1.0.0 sha512-A".to_string()]
    );
}

#[test]
fn the_dsh_composite_accepts_a_symlinked_home_ancestor() {
    // Council return 2026-09-13: the containment boundary is canonical,
    // so a symlinked ancestor resolving the same contained bundles is not
    // a refusal. The raw lookup anchor stays in place, so the loader's
    // search order is unchanged.
    let install = Synthetic::new();
    let base = install.composite();
    let alias = install.dir.path().join("alias");
    std::os::unix::fs::symlink(&install.seams.home, &alias).unwrap();
    let aliased = DshSeams {
        executable: install.seams.executable.clone(),
        home: alias,
    };
    let same = dsh_composite_with(&aliased, &install.node(), &[]).unwrap();
    assert_eq!(base.plugin, same.plugin);
    assert_eq!(base.canonical, same.canonical);
}

#[test]
fn containment_compares_canonical_components_not_string_prefixes() {
    // A near-prefix sibling (`headless-extra`) is outside the canonical
    // profile boundary even though its path string starts with
    // `.../headless`. A string-prefix check would wrongly admit it.
    let install = Synthetic::new();
    let sibling = install.seams.home.join("profiles").join("headless-extra");
    for file in PLUGIN_FILES {
        write(
            &sibling.join("dsh-plugin-cli-session"),
            file,
            file.as_bytes(),
        );
    }
    // The first bundle (`@deepseek-ai/dsh-base`) has no hit in the
    // sibling; the plugin does. Point the search at the sibling only to
    // reach the plugin candidate.
    write(
        &sibling.join("@deepseek-ai/dsh-base"),
        "package.json",
        br#"{"name":"@deepseek-ai/dsh-base"}"#,
    );
    assert!(dsh_composite_with(&install.seams, &install.node(), &[sibling]).is_err());

    // A profile directory that cannot be canonicalized (a broken
    // symlink) is unreadable rather than compared raw.
    let broken = tempfile::tempdir().unwrap();
    let home = broken.path().join("home");
    fs::create_dir_all(home.join("profiles")).unwrap();
    std::os::unix::fs::symlink(home.join("nowhere"), home.join("profiles/headless")).unwrap();
    let seams = DshSeams {
        executable: install.seams.executable.clone(),
        home,
    };
    assert!(dsh_composite_with(&seams, &install.node(), &[]).is_err());
}

#[test]
fn an_outside_first_bundle_hit_is_not_skipped_for_a_later_inside_one() {
    // `resolveBundleDir` takes the first candidate holding a
    // package.json. An outside first hit is a refusal, never a reason to
    // keep searching for a contained one.
    let install = Synthetic::new();
    let outside = install.dir.path().join("outside");
    for file in PLUGIN_FILES {
        write(
            &outside.join("dsh-plugin-cli-session"),
            file,
            file.as_bytes(),
        );
    }
    // The core lookup misses, the injected global folder hits first, and
    // the profile's own contained copy is later in the order.
    assert!(dsh_composite_with(&install.seams, &install.node(), &[outside]).is_err());
}

#[test]
fn the_dsh_composite_reads_the_qualified_locators_and_moves_with_them() {
    let install = Synthetic::new();
    let base = install.composite();
    assert_eq!(base.canonical.len(), 64);
    assert_eq!(base.core, "@deepseek-ai/dsh 0.1.5-rc.1 sha512-CORE");
    assert_eq!(base.node, "v22.23.2");
    assert_eq!(
        base.profile_bundles,
        vec!["@deepseek-ai/dsh-base", "dsh-plugin-cli-session"]
    );
    assert_eq!(base.profile_patch_reload, "startup");
    assert_eq!(base.home_patch, "absent");
    assert_eq!(
        base.dependencies,
        vec!["debug 2.6.9 sha512-DEBUG".to_string()]
    );
    assert!(base.extension.is_none());

    // A changed plugin byte moves the plugin component and the composite.
    write(
        &install
            .profile()
            .join("node_modules")
            .join("dsh-plugin-cli-session"),
        "lib/index.js",
        b"changed\n",
    );
    let moved = install.composite();
    assert_ne!(base.plugin, moved.plugin);
    assert_ne!(base.canonical, moved.canonical);

    // A home-level patch moves the home-patch line, and the composite.
    write(&install.seams.home, "cordis.patch.yml", b"[]\n");
    let patched = install.composite();
    assert_ne!(patched.home_patch, "absent");
    assert_ne!(moved.canonical, patched.canonical);
}

#[test]
fn the_dsh_composite_refuses_a_layout_outside_the_locators() {
    // A first bundle hit outside the core root and the profile is
    // unreadable rather than silently skipped.
    let install = Synthetic::new();
    let outside = install.dir.path().join("global/@deepseek-ai/dsh-base");
    write(
        &outside,
        "package.json",
        br#"{"name":"@deepseek-ai/dsh-base"}"#,
    );
    assert!(dsh_composite_with(
        &install.seams,
        &install.node(),
        &[install.dir.path().join("global")],
    )
    .is_err());

    // A core package whose shebang is not `env node` is refused.
    let install = Synthetic::new();
    write(
        std::path::Path::new(&install.seams.executable)
            .parent()
            .unwrap(),
        "bin.js",
        b"#!/bin/sh\n",
    );
    assert!(dsh_composite_with(&install.seams, &install.node(), &[]).is_err());

    // A profile that does not list the plugin is refused.
    let install = Synthetic::new();
    write(
        &install.profile(),
        "package.json",
        br#"{"dsh":{"profile":{"bundles":["@deepseek-ai/dsh-base"],"patchReload":"live"}}}"#,
    );
    assert!(dsh_composite_with(&install.seams, &install.node(), &[]).is_err());
}

#[test]
fn the_dsh_composite_composes_the_conditional_extension_only_when_listed() {
    let install = Synthetic::new();
    let base = install.composite();
    assert!(base.extension.is_none());

    // The bundle is named but does not resolve: unreadable, not an
    // absent line.
    write(
        &install.profile(),
        "package.json",
        br#"{"dsh":{"profile":{"bundles":["dsh-plugin-cli-session","brokkr-dsh-resume-policy"],"patchReload":"startup"}}}"#,
    );
    assert!(dsh_composite_with(&install.seams, &install.node(), &[]).is_err());

    // Once it resolves inside the profile it joins the composite.
    for file in EXTENSION_FILES {
        write(
            &install
                .profile()
                .join("node_modules")
                .join("brokkr-dsh-resume-policy"),
            file,
            file.as_bytes(),
        );
    }
    let extended = install.composite();
    assert!(extended.extension.is_some());
    assert_ne!(base.canonical, extended.canonical);
    assert!(extended
        .profile_bundles
        .iter()
        .any(|b| b == "brokkr-dsh-resume-policy"));
}

const CORE_LOCK: &[u8] =
    br#"{"packages":{"node_modules/@deepseek-ai/dsh":{"version":"1.0.0","integrity":"sha512-X"}}}"#;

fn core_package(root: &Path, manifest: &[u8], lock: &[u8]) -> PathBuf {
    let pkg = root.join("node_modules").join("@deepseek-ai").join("dsh");
    write(&pkg, "package.json", manifest);
    write(&pkg, "lib/bin.js", b"#!/usr/bin/env node\n");
    write(root, "node_modules/.package-lock.json", lock);
    pkg.join("lib/bin.js")
}

#[test]
fn executable_resolution_walks_path_entries_and_refuses_a_miss() {
    let dir = tempfile::tempdir().unwrap();
    let tool = dir.path().join("mytool");
    fs::write(&tool, b"x").unwrap();
    let path = Some(dir.path().as_os_str().to_os_string());
    let found = resolve_executable_in("mytool", path.clone()).unwrap();
    assert_eq!(found, dir.path().canonicalize().unwrap().join("mytool"));
    // An empty entry is skipped and a miss walks off the end.
    assert!(resolve_executable_in("mytool", Some(OsString::from(""))).is_err());
    assert!(resolve_executable_in("not-here", path).is_err());
    // A command carrying a separator is canonicalized, not searched.
    assert!(resolve_executable_in("/definitely/not/here", None).is_err());
    assert!(resolve_executable_in("a\\b", None).is_err());
}

#[test]
fn global_folders_reads_node_path_home_and_the_runtime_prefix() {
    let node = NodeRuntime {
        path: PathBuf::from("/opt/node/bin/node"),
        version: "v1".to_string(),
    };
    let folders = global_folders_in(
        &node,
        Some(OsString::from("/a::/b")),
        Some(OsString::from("/home/u")),
    );
    assert_eq!(
        folders,
        vec![
            PathBuf::from("/a"),
            PathBuf::from("/b"),
            PathBuf::from("/home/u/.node_modules"),
            PathBuf::from("/home/u/.node_libraries"),
            PathBuf::from("/opt/node/lib/node"),
        ]
    );
    // No NODE_PATH, no HOME and a runtime outside a `bin/` prefix.
    let bare = NodeRuntime {
        path: PathBuf::from("/opt/node"),
        version: "v1".to_string(),
    };
    assert!(global_folders_in(&bare, None, None).is_empty());
    let root = NodeRuntime {
        path: PathBuf::from("/"),
        version: "v1".to_string(),
    };
    assert!(global_folders_in(&root, None, None).is_empty());
}

#[cfg(unix)]
#[test]
fn spawn_node_runtime_reads_one_version_line_and_refuses_the_rest() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let stage = |name: &str, body: &str| {
        let path = dir.path().join(name);
        fs::write(&path, body).unwrap();
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).unwrap();
        path
    };
    let good = stage("node-good", "#!/bin/sh\nprintf 'v1.2.3\\n'\n");
    assert_eq!(spawn_node_runtime_at(good).unwrap().version, "v1.2.3");
    let failed = stage("node-fail", "#!/bin/sh\nexit 3\n");
    assert!(spawn_node_runtime_at(failed).is_err());
    let empty = stage("node-empty", "#!/bin/sh\ntrue\n");
    assert!(spawn_node_runtime_at(empty).is_err());
    // A version carrying a NUL or a second line is not one readable line.
    let nul = stage("node-nul", "#!/bin/sh\nprintf 'v1\\0x\\n'\n");
    assert!(spawn_node_runtime_at(nul).is_err());
    let multiline = stage("node-multiline", "#!/bin/sh\nprintf 'v1\\nv2\\n'\n");
    assert!(spawn_node_runtime_at(multiline).is_err());
    // A path that cannot be spawned at all.
    assert!(spawn_node_runtime_at(dir.path().join("absent")).is_err());
}

#[test]
fn dsh_seams_resolve_reads_the_home_and_refuses_a_missing_one() {
    assert_eq!(
        DshSeams::resolve().is_ok(),
        crate::transcript::dsh_home().is_some()
    );
    assert!(DshSeams::resolve_with("dsh".to_string(), None).is_err());
    let dir = tempfile::tempdir().unwrap();
    let seams = DshSeams::resolve_with("dsh".to_string(), Some(dir.path().to_path_buf())).unwrap();
    assert_eq!(seams.executable, "dsh");
    assert_eq!(seams.home, dir.path());
}

#[test]
fn dsh_composite_composes_over_the_real_and_injected_runtime_probes() {
    let install = Synthetic::new();
    let injected = dsh_composite_resolving(&install.seams, || Ok(install.node())).unwrap();
    assert_eq!(injected.node, "v22.23.2");
    assert_eq!(injected.canonical.len(), 64);
    // The real probe runs `node --version` from `PATH`; the delegation is
    // exercised even when a host lacks node.
    let _ = dsh_composite(&install.seams);
    let _ = spawn_node_runtime();
}

#[cfg(unix)]
#[test]
fn the_plugin_walk_refuses_symlinks_special_files_and_unreadable_roots() {
    use std::os::unix::net::UnixListener;
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "LICENSE", b"x");
    std::os::unix::fs::symlink(dir.path().join("LICENSE"), dir.path().join("link")).unwrap();
    let error = plugin_file_digests(dir.path()).unwrap_err();
    assert!(error.to_string().contains("symlink"), "{error}");

    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "LICENSE", b"x");
    let _socket = UnixListener::bind(dir.path().join("sock")).unwrap();
    let error = plugin_file_digests(dir.path()).unwrap_err();
    assert!(
        error.to_string().contains("neither a file nor a directory"),
        "{error}"
    );

    let error = plugin_file_digests(Path::new("/definitely/not/a/realdir")).unwrap_err();
    assert!(error.to_string().contains("unreadable"), "{error}");
    let error = sha256_file(Path::new("/definitely/not/a/file")).unwrap_err();
    assert!(error.to_string().contains("unreadable"), "{error}");
}

#[test]
fn npm_keys_with_an_empty_or_dotted_component_are_refused() {
    for key in [
        "node_modules/@./name",
        "node_modules/@scope/.",
        "node_modules/@scope/..",
    ] {
        assert!(npm_name(key).is_err(), "{key}");
    }
}

#[test]
fn npm_locks_reject_unparseable_and_incomplete_entries() {
    assert!(npm_dependencies("not json", &[]).is_err());
    assert!(npm_dependencies(r#"{"packages":{"node_modules/a":5}}"#, &[]).is_err());
    assert!(npm_dependencies(r#"{"packages":{"node_modules/a":{}}}"#, &[]).is_err());
    assert!(npm_dependencies(
        r#"{"packages":{"node_modules/a":{"version":"1.0.0","integrity":""}}}"#,
        &[]
    )
    .is_err());
    assert!(npm_dependencies(
        "{\"packages\":{\"node_modules/a\":{\"version\":\"1.0.0\",\"integrity\":\"a\\nb\"}}}",
        &[]
    )
    .is_err());
}

#[test]
fn pnpm_locks_reject_every_unrecognized_construct() {
    let blank = "\nlockfileVersion: '9.0'\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n";
    assert_eq!(
        pnpm_dependencies(blank, &[]).unwrap(),
        vec!["debug 2.6.9 sha512-X".to_string()]
    );
    // An all-blank document has no version header.
    assert!(pnpm_dependencies("\n\n", &[]).is_err());
    // A first non-blank line that is not the version header.
    assert!(pnpm_dependencies("foo\n", &[]).is_err());
    // A child line before `packages:` is ignored.
    assert!(pnpm_dependencies("lockfileVersion: '9.0'\n  stray: x\npackages:\n", &[]).is_ok());
    // A package key without a trailing colon.
    assert!(pnpm_dependencies("lockfileVersion: '9.0'\npackages:\n  debug@2.6.9\n", &[]).is_err());
    // An empty version after the `@`.
    assert!(pnpm_dependencies(
        "lockfileVersion: '9.0'\npackages:\n  a@:\n    resolution: {integrity: sha512-X}\n",
        &[]
    )
    .is_err());
    // An empty resolution integrity.
    assert!(pnpm_dependencies(
        "lockfileVersion: '9.0'\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: }\n",
        &[]
    )
    .is_err());
    // A resolution outside any entry.
    assert!(pnpm_dependencies(
        "lockfileVersion: '9.0'\npackages:\n    resolution: {integrity: sha512-X}\n",
        &[]
    )
    .is_err());
}

#[test]
fn read_json_read_text_and_first_line_report_io_and_encoding_failures() {
    let absent = Path::new("/definitely/not/a/file");
    assert!(read_json(absent).is_err());
    assert!(read_text(absent).is_err());
    assert!(first_line(absent).is_err());
    let dir = tempfile::tempdir().unwrap();
    let bad = dir.path().join("bad.json");
    fs::write(&bad, b"not json").unwrap();
    assert!(read_json(&bad).is_err());
    let crlf = dir.path().join("crlf");
    fs::write(&crlf, b"#!/usr/bin/env node\r\nrest\n").unwrap();
    assert_eq!(first_line(&crlf).unwrap(), "#!/usr/bin/env node");
    let raw = dir.path().join("raw");
    fs::write(&raw, b"\xff\n").unwrap();
    assert!(first_line(&raw).is_err());
}

#[test]
fn resolve_core_refuses_an_executable_with_no_dsh_ancestor() {
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("tool.js");
    fs::write(&bin, b"#!/usr/bin/env node\n").unwrap();
    assert!(resolve_core(&bin.to_string_lossy()).is_err());
}

#[test]
fn resolve_core_reads_past_a_non_dsh_or_unparseable_ancestor_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("core");
    let shared = root.join("node_modules").join("@deepseek-ai").join("dsh");
    write(
        &shared.join("lib"),
        "package.json",
        br#"{"name":"something-else"}"#,
    );
    let bin = core_package(
        &root,
        br#"{"name":"@deepseek-ai/dsh","version":"1.0.0","bin":{"dsh":"lib/bin.js"}}"#,
        CORE_LOCK,
    );
    assert_eq!(
        resolve_core(&bin.to_string_lossy()).unwrap().version,
        "1.0.0"
    );

    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("core");
    let shared = root.join("node_modules").join("@deepseek-ai").join("dsh");
    write(&shared.join("lib"), "package.json", b"not json");
    let bin = core_package(
        &root,
        br#"{"name":"@deepseek-ai/dsh","version":"1.0.0","bin":{"dsh":"lib/bin.js"}}"#,
        CORE_LOCK,
    );
    assert_eq!(
        resolve_core(&bin.to_string_lossy()).unwrap().version,
        "1.0.0"
    );
}

#[test]
fn resolve_core_refuses_a_manifest_that_does_not_match_its_binary_or_scope() {
    // A binary that is not `bin.dsh`.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("core");
    let bin = core_package(
        &root,
        br#"{"name":"@deepseek-ai/dsh","version":"1.0.0","bin":{"dsh":"lib/bin.js"}}"#,
        CORE_LOCK,
    );
    let other = bin.parent().unwrap().join("other.js");
    fs::write(&other, b"#!/usr/bin/env node\n").unwrap();
    assert!(resolve_core(&other.to_string_lossy()).is_err());

    // A manifest with no `bin.dsh`.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("core");
    let bin = core_package(
        &root,
        br#"{"name":"@deepseek-ai/dsh","version":"1.0.0"}"#,
        CORE_LOCK,
    );
    assert!(resolve_core(&bin.to_string_lossy()).is_err());

    // A manifest with no `version`.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("core");
    let bin = core_package(
        &root,
        br#"{"name":"@deepseek-ai/dsh","bin":{"dsh":"lib/bin.js"}}"#,
        CORE_LOCK,
    );
    assert!(resolve_core(&bin.to_string_lossy()).is_err());

    // A dsh manifest outside `@deepseek-ai`.
    let dir = tempfile::tempdir().unwrap();
    let pkg = dir.path().join("node_modules").join("dsh");
    write(
        &pkg,
        "package.json",
        br#"{"name":"@deepseek-ai/dsh","version":"1.0.0","bin":{"dsh":"lib/bin.js"}}"#,
    );
    write(&pkg, "lib/bin.js", b"#!/usr/bin/env node\n");
    assert!(resolve_core(&pkg.join("lib/bin.js").to_string_lossy()).is_err());

    // A scope whose parent is not `node_modules`.
    let dir = tempfile::tempdir().unwrap();
    let pkg = dir.path().join("x").join("@deepseek-ai").join("dsh");
    write(
        &pkg,
        "package.json",
        br#"{"name":"@deepseek-ai/dsh","version":"1.0.0","bin":{"dsh":"lib/bin.js"}}"#,
    );
    write(&pkg, "lib/bin.js", b"#!/usr/bin/env node\n");
    assert!(resolve_core(&pkg.join("lib/bin.js").to_string_lossy()).is_err());

    // A renamed package directory under `@deepseek-ai`.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("core");
    let pkg = root.join("node_modules").join("@deepseek-ai").join("other");
    write(
        &pkg,
        "package.json",
        br#"{"name":"@deepseek-ai/dsh","version":"1.0.0","bin":{"dsh":"lib/bin.js"}}"#,
    );
    write(&pkg, "lib/bin.js", b"#!/usr/bin/env node\n");
    assert!(resolve_core(&pkg.join("lib/bin.js").to_string_lossy()).is_err());
}

#[test]
fn resolve_core_refuses_a_lock_that_disagrees_with_the_package() {
    // The hidden lock has no dsh entry.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("core");
    let bin = core_package(
        &root,
        br#"{"name":"@deepseek-ai/dsh","version":"1.0.0","bin":{"dsh":"lib/bin.js"}}"#,
        br#"{"packages":{}}"#,
    );
    assert!(resolve_core(&bin.to_string_lossy()).is_err());

    // The lock version differs from the package version.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("core");
    let bin = core_package(
        &root,
        br#"{"name":"@deepseek-ai/dsh","version":"1.0.0","bin":{"dsh":"lib/bin.js"}}"#,
        br#"{"packages":{"node_modules/@deepseek-ai/dsh":{"version":"2.0.0","integrity":"sha512-X"}}}"#,
    );
    assert!(resolve_core(&bin.to_string_lossy()).is_err());

    // The lock integrity carries a newline.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("core");
    let bin = core_package(
        &root,
        br#"{"name":"@deepseek-ai/dsh","version":"1.0.0","bin":{"dsh":"lib/bin.js"}}"#,
        br#"{"packages":{"node_modules/@deepseek-ai/dsh":{"version":"1.0.0","integrity":"a\nb"}}}"#,
    );
    assert!(resolve_core(&bin.to_string_lossy()).is_err());

    // The lock integrity carries a NUL.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("core");
    let bin = core_package(
        &root,
        br#"{"name":"@deepseek-ai/dsh","version":"1.0.0","bin":{"dsh":"lib/bin.js"}}"#,
        br#"{"packages":{"node_modules/@deepseek-ai/dsh":{"version":"1.0.0","integrity":"a\u0000b"}}}"#,
    );
    assert!(resolve_core(&bin.to_string_lossy()).is_err());
}

#[test]
fn read_profile_refuses_a_missing_manifest_bundles_and_reload() {
    // No profile directory.
    let dir = tempfile::tempdir().unwrap();
    assert!(read_profile(dir.path()).is_err());

    // No `dsh.profile`.
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    write(
        &home.join("profiles/headless"),
        "package.json",
        br#"{"dsh":{}}"#,
    );
    assert!(read_profile(&home).is_err());

    // An empty bundle array.
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    write(
        &home.join("profiles/headless"),
        "package.json",
        br#"{"dsh":{"profile":{"bundles":[],"patchReload":"startup"}}}"#,
    );
    assert!(read_profile(&home).is_err());

    // A non-string bundle entry.
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    write(
        &home.join("profiles/headless"),
        "package.json",
        br#"{"dsh":{"profile":{"bundles":[""],"patchReload":"startup"}}}"#,
    );
    assert!(read_profile(&home).is_err());

    // A patchReload outside the closed pair.
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    write(
        &home.join("profiles/headless"),
        "package.json",
        br#"{"dsh":{"profile":{"bundles":["x"],"patchReload":"other"}}}"#,
    );
    assert!(read_profile(&home).is_err());

    // `package.json` is a directory: read_json reports the read error.
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    fs::create_dir_all(home.join("profiles/headless/package.json")).unwrap();
    assert!(read_profile(&home).is_err());
}

#[test]
fn the_plugin_and_extension_must_resolve_inside_the_profile() {
    // A plugin found under the core root is refused, not relocated.
    let install = Synthetic::new();
    let core_plugin = install
        .dir
        .path()
        .join("core/node_modules/dsh-plugin-cli-session");
    for file in PLUGIN_FILES {
        write(&core_plugin, file, file.as_bytes());
    }
    assert!(dsh_composite_with(&install.seams, &install.node(), &[]).is_err());

    // An extension found under the core root is refused.
    let install = Synthetic::new();
    write(
        &install.profile(),
        "package.json",
        br#"{"dsh":{"profile":{"bundles":["dsh-plugin-cli-session","brokkr-dsh-resume-policy"],"patchReload":"startup"}}}"#,
    );
    let core_extension = install
        .dir
        .path()
        .join("core/node_modules/brokkr-dsh-resume-policy");
    for file in EXTENSION_FILES {
        write(&core_extension, file, file.as_bytes());
    }
    assert!(dsh_composite_with(&install.seams, &install.node(), &[]).is_err());
}

#[test]
fn dsh_composite_with_propagates_a_lock_and_a_value_refusal() {
    // A non-excluded lock entry with no integrity.
    let install = Synthetic::new();
    write(
        &install.dir.path().join("core"),
        "node_modules/.package-lock.json",
        br#"{"lockfileVersion":3,"packages":{"node_modules/@deepseek-ai/dsh":{"version":"0.1.5-rc.1","integrity":"sha512-CORE"},"node_modules/debug":{"version":"2.6.9"}}}"#,
    );
    assert!(dsh_composite_with(&install.seams, &install.node(), &[]).is_err());

    // A core version carrying a newline reaches canonical_composite.
    let install = Synthetic::new();
    let pkg = install
        .dir
        .path()
        .join("core/node_modules/@deepseek-ai/dsh");
    write(
        &pkg,
        "package.json",
        br#"{"name":"@deepseek-ai/dsh","version":"1.0\nx","bin":{"dsh":"lib/bin.js"}}"#,
    );
    write(
        &install.dir.path().join("core"),
        "node_modules/.package-lock.json",
        br#"{"lockfileVersion":3,"packages":{"node_modules/@deepseek-ai/dsh":{"version":"1.0\nx","integrity":"sha512-CORE"},"node_modules/debug":{"version":"2.6.9","integrity":"sha512-DEBUG"}}}"#,
    );
    assert!(dsh_composite_with(&install.seams, &install.node(), &[]).is_err());
}

#[test]
fn the_plugin_component_names_a_missing_expected_file() {
    let dir = tempfile::tempdir().unwrap();
    // Five of the six expected files and no extras, so the missing-file
    // arm is the one that refuses.
    for file in PLUGIN_FILES.iter().skip(1) {
        write(dir.path(), file, file.as_bytes());
    }
    let error = plugin_component(dir.path(), &PLUGIN_FILES).unwrap_err();
    assert!(
        error.to_string().contains("missing expected file"),
        "{error}"
    );
}
