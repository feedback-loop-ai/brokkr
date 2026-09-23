use super::*;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

/// The differential matrix against `std::process::Command`, in its own
/// file: every name crossed with every layout, each cell its own oracle.
#[cfg(unix)]
mod native_matrix;

fn write(dir: &Path, relative: &str, bytes: &[u8]) {
    let path = dir.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

/// `write` for a file the selection must admit as an EXECUTABLE: the
/// resolver classifies a core `lib/bin.js` as a child's search would,
/// and a child does not execute a mode-0644 script.
fn write_executable(dir: &Path, relative: &str, bytes: &[u8]) {
    write(dir, relative, bytes);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir.join(relative), fs::Permissions::from_mode(0o755)).unwrap();
    }
}

/// Run a prepared command, retrying only the ETXTBSY a freshly linked
/// binary can answer with.
///
/// A test that re-executes its own binary can reach `exec` while another
/// thread of this same run still holds a write descriptor to a file it
/// staged; the kernel then refuses with `Text file busy`. That is a fact
/// about the moment, not about the code under test, and a coverage gate
/// that demands one clean run cannot be left to lose a race (#255).
///
/// This is INHERITED and guards a different subject — `current_exe`, a
/// file no thread of this run writes — so it is not the repair below and
/// proves nothing about it.
#[cfg(unix)]
fn spawn_retrying_etxtbsy(command: &mut std::process::Command) -> std::process::Output {
    for _ in 0..50 {
        match command.output() {
            Ok(output) => return output,
            Err(error) if error.raw_os_error() == Some(26) => {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(error) => panic!("the child test binary runs: {error}"),
        }
    }
    panic!("the child test binary stayed busy");
}

use crate::adapters::tests::staging_name;

/// Install an executable shim the house way (#255): write a TEMPORARY
/// SIBLING beside the destination, close it, then `rename` it into place
/// before anything execs that pathname. No retry loop and no sleep.
///
/// The rename is the installation rule: the destination pathname never
/// names a partially written file, so a concurrent `exec` of it either
/// finds nothing or finds the complete shim. But the rename alone is not
/// the whole of #255, and the reason is measured in
/// `a_renamed_shim_inherits_its_writer_and_a_staged_one_carries_none`
/// below: `execve` refuses with ETXTBSY while the inode's write count is
/// above zero, and `rename` moves the INODE, write count and all — so a
/// destination renamed in from a staging file THIS process wrote
/// inherits exactly that descriptor. A fork inherits every open
/// descriptor and holds it until it execs, and this suite forks
/// constantly.
///
/// So the staging sibling's bytes are written by a CHILD — `/bin/sh`,
/// which every shim here already depends on for its own shebang — and
/// that child is reaped before the rename. The inode that arrives at the
/// destination carries no write descriptor of this process's for any
/// sibling thread's fork to have inherited. Measured on this host with
/// eight forking threads and 4000 rounds: writing in place refused 436
/// times, staging-and-renaming from this process refused 471, and
/// staging from a child refused 0.
#[cfg(unix)]
fn stage_executable(dir: &Path, name: &str, body: &[u8]) -> PathBuf {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join(name);
    let staging = dir.join(staging_name());
    let mut child = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg("cat > \"$0\"")
        .arg(&staging)
        // The staging child's own PATH, never the caller's: the native
        // matrix runs its cells under a PATH it composes itself, and a
        // fixture that cannot be written there would fail as a missing
        // `cat` rather than as the layout under test.
        .env("PATH", "/usr/bin:/bin")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("staging {}: {error}", path.display()));
    child
        .stdin
        .take()
        .expect("piped")
        .write_all(body)
        .unwrap_or_else(|error| panic!("staging {}: {error}", path.display()));
    let status = child.wait().unwrap();
    assert!(
        status.success(),
        "staging {} exited {status}",
        path.display()
    );
    // `chmod` opens nothing, so the mode is the parent's to set: only the
    // WRITE descriptor is what `execve` counts. It is set on the sibling,
    // so the destination is complete and executable the instant it exists.
    fs::set_permissions(&staging, fs::Permissions::from_mode(0o755)).unwrap();
    fs::rename(&staging, &path)
        .unwrap_or_else(|error| panic!("installing {}: {error}", path.display()));
    path
}

fn plugin_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../extensions/dsh/plugin-cli-session")
}

/// A temporary fixture tree and the CANONICAL spelling of its root, kept
/// together because only the second of them may be built on.
///
/// The producer canonicalizes what it reports, and the per-user temporary
/// directory is not reached through a canonical path on every host:
/// macOS's `$TMPDIR` lives under `/var`, which is a symlink to
/// `/private/var`, so `TempDir::path()` spells `/var/folders/…` where the
/// producer says `/private/var/folders/…`. A fixture that glued
/// `TempDir::path()` into an expected refusal therefore wrote a path the
/// producer never says, and seven assertions that hold on Linux failed on
/// macOS for that reason alone (PR #311's macOS leg, 2026-09-21). The
/// root is resolved ONCE here, at creation, and every fixture path is
/// joined onto it, so fixture and producer name the same file by the same
/// spelling. Linux's `/tmp` is already canonical, so no Linux expectation
/// moves. The `TempDir` is retained, and with it the tree's cleanup.
struct FixtureRoot {
    /// Held for its `Drop` alone: the tree lives as long as this value
    /// does, and is removed with it. Nothing reads its raw path.
    _dir: tempfile::TempDir,
    path: PathBuf,
}

impl FixtureRoot {
    fn new() -> FixtureRoot {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().canonicalize().unwrap();
        FixtureRoot { _dir: dir, path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

fn digest_of(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(bytes))
}

/// An npm hidden lock as the producer receives it: already-parsed JSON.
/// Parsing is `read_json`'s job and is proved there, so a reader test
/// starts where the reader starts.
fn lock(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

/// The component digest over a directory, which is how every caller
/// reaches it: one walk, then the serialization of what it observed.
fn component_of(dir: &Path, expected: &[&str]) -> Result<String, CompositeError> {
    Ok(component_digest(&plugin_file_digests(
        "plugin",
        dir,
        expected,
        &read_dir_entries,
    )?))
}

/// The canonical FILE of a retained Node selection: the identity half,
/// which the selection tests written before R2 compare.
fn node_file(node: &Option<DshNode>) -> Option<PathBuf> {
    node.as_ref().map(|node| node.path.clone())
}

/// A Node runtime run under its own path: what a suite retains by hand.
fn node_at(path: impl Into<PathBuf>) -> DshNode {
    let path = path.into();
    DshNode {
        invocation: DshInvocation::of(path.clone()),
        path,
    }
}

/// The refusal from a producer whose success value is a private struct
/// with no `Debug`: a derived one nobody prints is a function the exact
/// coverage gate counts and no test can reach, so the reason is read by
/// matching rather than by `unwrap_err`.
fn refused<T>(result: Result<T, CompositeError>) -> String {
    match result {
        Ok(_) => panic!("expected a refusal"),
        Err(error) => error.to_string(),
    }
}

/// `refused` for a table of vectors: an ACCEPTED vector is named in the
/// panic, so a removal control that lets one through reports which one
/// rather than "expected a refusal" at a line inside a loop.
fn refused_vector<T>(result: Result<T, CompositeError>, vector: impl std::fmt::Debug) -> String {
    match result {
        Ok(_) => panic!("{vector:?} was accepted"),
        Err(error) => error.to_string(),
    }
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
    const FORBIDDEN: &str = "empty, absolute, trailing, or carries a forbidden byte";
    // Each key is asserted by the REASON it is refused for, so a key
    // that started failing the early byte guard instead of the group
    // grammar — or the reverse — is a visible change rather than a
    // still-green `is_err()` (council return 2026-09-19, finding 4).
    for (key, reason) in [
        ("", FORBIDDEN),
        ("/node_modules/a", FORBIDDEN),
        ("node_modules/", FORBIDDEN),
        ("node_modules/a/", FORBIDDEN),
        ("node_modules/a b", FORBIDDEN),
        ("node_modules/a\tb", FORBIDDEN),
        ("node_modules/a\\b", FORBIDDEN),
        // A key whose GROUPS are separated the other platform's way is
        // not a key with one odd byte in a name: it is a spelling this
        // reader never converts, so it is refused whole.
        ("node_modules\\a", FORBIDDEN),
        ("node_modules\\a\\node_modules\\b", FORBIDDEN),
        ("node_modules/a/node_modules/", FORBIDDEN),
        ("node_modules//a", "invalid unscoped component"),
        ("node_modules/.", "invalid unscoped component"),
        ("node_modules/..", "invalid unscoped component"),
        ("node_modules/a@b", "invalid unscoped component"),
        ("node_modules/@scope/", FORBIDDEN),
        ("node_modules/@scope", "scoped package has no '/'"),
        // The scope half and the name half of a scoped package are
        // separately invalid: one passing is not the key passing.
        ("node_modules/@./name", "invalid scope or name component"),
        ("node_modules/@scope/.", "invalid scope or name component"),
        // INCOMPLETE keys: a first group with no separator at all, and a
        // trailing `node_modules` that opens a group it never fills.
        ("node_modules", "expected a 'node_modules/' group"),
        (
            "node_modules/a/node_modules",
            "expected a 'node_modules/' group",
        ),
        // TRAVERSAL, at each of the three places a key offers it: ahead
        // of the first group, between two groups, and as a terminal or
        // scoped component. None of them is normalized away.
        ("../node_modules/a", "expected a 'node_modules/' group"),
        (
            "node_modules/a/../node_modules/b",
            "expected a 'node_modules/' group",
        ),
        (
            "node_modules/a/node_modules/../b",
            "invalid unscoped component",
        ),
        (
            "node_modules/@scope/../name",
            "invalid scope or name component",
        ),
        // A valid terminal package cannot excuse a malformed group ahead
        // of it: every group of every key is parsed and validated.
        (
            "node_modules/a/extra/node_modules/@scope/child",
            "expected a 'node_modules/' group",
        ),
    ] {
        assert_eq!(
            refused(npm_name(key)),
            format!("npm key is unreadable: '{key}': {reason}"),
            "{key:?}"
        );
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
    // The early key guard refuses a NUL BEFORE any group is parsed: the
    // reason separates it from `valid_component`'s own NUL arm above,
    // which a bare `is_err()` could not.
    assert_eq!(
        refused(npm_name("node_modules/a\0b")),
        "npm key is unreadable: 'node_modules/a\0b': empty, absolute, trailing, or carries a forbidden byte"
    );
}

/// The one scalar rule (design D6 (b)): empty, NUL and EVERY whitespace
/// character — space, tab, CR and LF — are refused, and nothing is
/// trimmed to fit. Each reason is distinct, so a refusal says which rule
/// the value broke.
#[test]
fn the_scalar_rule_names_empty_nul_and_every_whitespace() {
    assert_eq!(scalar_reason(""), Some("is empty"));
    assert_eq!(scalar_reason("a\0b"), Some("carries a NUL"));
    for bad in [" x", "x ", "a\tb", "a\rb", "a\nb", "\u{a0}"] {
        assert_eq!(scalar_reason(bad), Some("carries whitespace"), "{bad:?}");
    }
    assert_eq!(scalar_reason("v22.23.2"), None);
    assert_eq!(scalar("node", "v22.23.2"), Ok(()));
    assert_eq!(
        scalar("node", "v22 23"),
        Err(CompositeError::Value {
            component: "node",
            reason: "carries whitespace",
        })
    );
    assert_eq!(
        CompositeError::Value {
            component: "core",
            reason: "is empty",
        }
        .to_string(),
        "the core value is empty"
    );
}

/// A lock entry's own bytes decide its triple: an empty version and a
/// NUL-bearing integrity are refusals, not shorter or truncated values.
#[test]
fn npm_lock_entry_bytes_are_read_rather_than_shortened() {
    let empty = npm_dependencies(
        &lock(r#"{"packages":{"node_modules/a":{"version":"","integrity":"x"}}}"#),
        &[],
    )
    .unwrap_err();
    assert_eq!(
        empty.to_string(),
        "npm lock is unreadable: 'node_modules/a': version is empty"
    );
    let nul = npm_dependencies(
        &lock(
            "{\"packages\":{\"node_modules/a\":{\"version\":\"1\",\"integrity\":\"a\\u0000b\"}}}",
        ),
        &[],
    )
    .unwrap_err();
    assert_eq!(
        nul.to_string(),
        "npm lock is unreadable: 'node_modules/a': integrity carries a NUL"
    );
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
    assert_eq!(
        short.to_string(),
        "pnpm lock is unreadable: 'a': no '@' after the first character"
    );
    // A key whose first character is multibyte is read by character, not
    // sliced at byte one: with an `@` behind it the entry is a triple, and
    // without one it is the same refusal as above rather than a panic.
    assert_eq!(
        pnpm_dependencies(
            "lockfileVersion: '9.0'\npackages:\n  'é@1.0.0':\n    resolution: {integrity: x}\n",
            &[],
        )
        .unwrap(),
        vec!["é 1.0.0 x".to_string()]
    );
    let multibyte = pnpm_dependencies(
        "lockfileVersion: '9.0'\npackages:\n  'éa':\n    resolution: {integrity: x}\n",
        &[],
    )
    .unwrap_err();
    assert_eq!(
        multibyte.to_string(),
        "pnpm lock is unreadable: 'éa': no '@' after the first character"
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
    let text = r#"{"lockfileVersion":3,"packages":{
      "node_modules/@scope/child":{"version":"1.0.0","integrity":"sha512-AAA"},
      "node_modules/a/node_modules/@scope/child":{"version":"1.1.0","integrity":"sha512-BBB"},
      "node_modules/a/node_modules/@parent/b/node_modules/@scope/child":{"version":"2.0.0","integrity":"sha512-CCC"},
      "node_modules/x/node_modules/debug":{"version":"4.4.3","integrity":"sha512-DDD"},
      "node_modules/y/node_modules/debug":{"version":"4.4.3","integrity":"sha512-DDD"},
      "node_modules/body-parser/node_modules/debug":{"version":"4.4.3","integrity":"sha512-DDD"}
    }}"#;
    let deps = npm_dependencies(&lock(text), &[]).unwrap();
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

/// Exclusions identify exact RECORDS. The core's own hidden-lock key
/// leaves the dependencies; a same-named registry record at another
/// version does NOT, because collapsing by name would erase a real
/// dependency the installation resolved.
#[test]
fn npm_exclusions_name_exact_records_and_keep_same_named_registry_ones() {
    let text = r#"{"lockfileVersion":3,"packages":{
      "node_modules/@deepseek-ai/dsh":{"version":"0.1.5-rc.2","integrity":"sha512-CORE"},
      "node_modules/x/node_modules/@deepseek-ai/dsh":{"version":"0.1.4","integrity":"sha512-OLD"},
      "node_modules/dsh-plugin-cli-session":{"version":"0.2.0","resolved":"file:plugin.tgz"},
      "node_modules/y/node_modules/dsh-plugin-cli-session":{"version":"0.2.0","resolved":"https://registry.npmjs.org/x.tgz","integrity":"sha512-REG"}
    }}"#;
    assert_eq!(
        npm_dependencies(&lock(text), &["dsh-plugin-cli-session"]).unwrap(),
        vec![
            "@deepseek-ai/dsh 0.1.4 sha512-OLD".to_string(),
            "dsh-plugin-cli-session 0.2.0 sha512-REG".to_string(),
        ],
        "only the core's own key and the plugin's local file: record leave"
    );
}

#[test]
fn the_hidden_npm_lock_is_the_sole_source_and_missing_fields_are_refused() {
    let text = r#"{"lockfileVersion":3,"packages":{
      "node_modules/@deepseek-ai/dsh":{"version":"0.1.5-rc.2","integrity":"sha512-CORE"},
      "node_modules/dsh-plugin-cli-session":{"version":"0.2.0","resolved":"file:plugin.tgz","link":true},
      "node_modules/real":{"version":"1.0.0","integrity":"sha512-REAL"}
    }}"#;
    assert_eq!(
        npm_dependencies(&lock(text), &["dsh-plugin-cli-session"]).unwrap(),
        vec!["real 1.0.0 sha512-REAL".to_string()]
    );
    // Without the plugin named as a local component source, its tarball
    // record is an ordinary entry — and it carries no registry integrity.
    let error = npm_dependencies(&lock(text), &[]).unwrap_err();
    assert_eq!(
        error.to_string(),
        "npm lock is unreadable: 'node_modules/dsh-plugin-cli-session': no registry 'integrity'"
    );

    // A root package-lock's `""` root entry is refused, so the plain lock
    // can never substitute for the hidden one.
    let root = r#"{"lockfileVersion":3,"packages":{
      "":{"name":"x","version":"1.0.0"},
      "node_modules/a":{"version":"1.0.0","integrity":"sha512-A"}}}"#;
    let error = npm_dependencies(&lock(root), &[]).unwrap_err();
    assert_eq!(
        error.to_string(),
        "npm key is unreadable: '': empty, absolute, trailing, or carries a forbidden byte"
    );
    let error = npm_dependencies(&lock(r#"{"lockfileVersion":3}"#), &[]).unwrap_err();
    assert_eq!(
        error.to_string(),
        "npm lock is unreadable: no 'packages' object"
    );
}

#[test]
fn pnpm_reads_lockfile_9_and_normalizes_the_same_triples() {
    let text = "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    \
                resolution: {integrity: sha512-X}\n\n  '@scope/child@1.0.0':\n    \
                resolution: {integrity: sha512-Y, tarball: https://example.invalid/x.tgz}\n\n  \
                dsh-plugin-cli-session@file:./plugin.tgz:\n    \
                resolution: {integrity: sha512-Z, tarball: file:./plugin.tgz}\n    version: 0.2.0\n\n  \
                dsh-plugin-cli-session@0.2.0:\n    resolution: {integrity: sha512-REG}\n";
    assert_eq!(
        pnpm_dependencies(text, &["dsh-plugin-cli-session"]).unwrap(),
        vec![
            "@scope/child 1.0.0 sha512-Y".to_string(),
            "debug 2.6.9 sha512-X".to_string(),
            // The registry record of the same name is retained; only the
            // `file:` tarball record left.
            "dsh-plugin-cli-session 0.2.0 sha512-REG".to_string(),
        ]
    );
}

/// The pnpm integrity crosses the same scalar rule the npm one does.
#[test]
fn a_pnpm_integrity_with_whitespace_is_unreadable() {
    let error = pnpm_dependencies(
        "lockfileVersion: '9.0'\npackages:\n  a@1.0.0:\n    resolution: {integrity: 'sha512 X'}\n",
        &[],
    )
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        "pnpm lock is unreadable: 'a@1.0.0': integrity carries whitespace"
    );
}

#[test]
fn npm_and_pnpm_agree_on_equivalent_entries() {
    let npm = r#"{"lockfileVersion":3,"packages":{"node_modules/debug":{"version":"2.6.9","integrity":"sha512-X"}}}"#;
    let pnpm = "lockfileVersion: '9.0'\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n";
    assert_eq!(
        npm_dependencies(&lock(npm), &[]).unwrap(),
        pnpm_dependencies(pnpm, &[]).unwrap()
    );
}

/// Every refusal below is asserted by its REASON. A bare `is_err()` is
/// satisfied by a document that failed for an unrelated cause — the
/// construct under test never reached the reader — so it proves nothing
/// about the grammar (council return 2026-09-19, F8).
#[test]
fn unrecognized_pnpm_constructs_are_refused() {
    for (text, reason) in [
        ("\tlockfileVersion: '9.0'\npackages:\n", "a tab"),
        (
            "lockfileVersion: '9.0'\n# comment\npackages:\n",
            "a comment or document marker",
        ),
        (
            "lockfileVersion: '9.0'\npackages:\n\n  debug@2.6.9:\n    resolution:\n",
            "a block-form or malformed resolution",
        ),
        (
            "lockfileVersion: '9.0'\npackages:\n\n  debug:\n    resolution: {integrity: sha512-X}\n",
            "'debug': no '@' after the first character",
        ),
        (
            "lockfileVersion: '9.0'\npackages:\n\n  debug@2.6.9:\n    engines: {node: '>=1'}\n",
            "'debug@2.6.9': no resolution integrity",
        ),
        (
            "lockfileVersion: '9.0'\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n    resolution: {integrity: sha512-Y}\n",
            "'debug@2.6.9': repeated resolution",
        ),
        (
            "lockfileVersion: '9.0'\npackages:\n\n  debug@2.6.9:\n    resolution: {tarball: x}\n",
            "a resolution without integrity",
        ),
        ("lockfileVersion: '9.1'\npackages:\n", "lockfileVersion is not 9.0"),
    ] {
        assert_eq!(
            refused_vector(pnpm_dependencies(text, &[]), text),
            format!("pnpm lock is unreadable: {reason}"),
            "{text:?}"
        );
    }
}

/// The two package keys that separate a validated reader from a
/// plausible one. `a b@1` splits into the name `a b` and the version `1`;
/// `a@b 1` splits into the name `a` and the version `b 1`. Both
/// serialized to the IDENTICAL dependency line `a b 1 sha512-X`, so two
/// different installs shared one identity. The key crosses the scalar
/// rule whole, before the split, which is what tells them apart
/// (council return 2026-09-19, F1).
#[test]
fn two_package_keys_that_serialized_to_one_line_are_both_refused() {
    let lock = |key: &str| {
        format!("lockfileVersion: '9.0'\npackages:\n  '{key}':\n    resolution: {{integrity: sha512-X}}\n")
    };
    for key in ["a b@1", "a@b 1"] {
        assert_eq!(
            refused(pnpm_dependencies(&lock(key), &[])),
            format!("pnpm lock is unreadable: '{key}': the key carries whitespace")
        );
    }
    // The line both of them WOULD have produced, from a key that carries
    // no whitespace at all: the collision was real, not hypothetical.
    assert_eq!(
        pnpm_dependencies(&lock("ab@1"), &[]).unwrap(),
        vec!["ab 1 sha512-X".to_string()]
    );
    // A name that is not a package name, and a scope with no name
    // behind it.
    for (key, reason) in [
        ("@scope@1.0.0", "'@scope' is not a package name"),
        ("a/b@1.0.0", "'a/b' is not a package name"),
        // A well-formed scope whose NAME half is not a component: the
        // scope alone passing is not the key passing. And the mirror of
        // it — an empty scope — which refuses before the name is read.
        ("@scope/na/me@1.0.0", "'@scope/na/me' is not a package name"),
        ("@/x@1.0.0", "'@/x' is not a package name"),
    ] {
        assert_eq!(
            refused(pnpm_dependencies(&lock(key), &[])),
            format!("pnpm lock is unreadable: '{key}': {reason}")
        );
    }
    // A NUL inside a quoted key never reaches the dependency line. It is
    // refused BEFORE the key's grammar now, by YAML's own character set:
    // a stream carrying a NUL is not a YAML document at all, and the
    // reader that admitted one through an ignored field produced the
    // valid control's composite (review 2026-09-20, F5).
    let nul = lock("a\0b@1");
    assert_eq!(
        refused(pnpm_dependencies(&nul, &[])),
        "pnpm lock is unreadable: the character U+0000, which YAML's character set excludes"
    );
    // The key's own scalar rule still answers for the whitespace a
    // printable document can spell.
    assert_eq!(
        refused(pnpm_dependencies(&lock("a\u{a0}b@1"), &[])),
        "pnpm lock is unreadable: 'a\u{a0}b@1': the key carries whitespace"
    );
}

/// The pnpm lock is read through an INCLUSIVE 8,388,608-byte bound, and
/// at most one byte beyond it: exactly the limit reaches grammar
/// validation, and the next byte is the named refusal. The limit is
/// applied to the bytes actually read, never to a metadata size.
#[test]
fn the_pnpm_reader_is_bounded_inclusively_at_the_limit() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pnpm-lock.yaml");

    // Exactly the limit: admitted by the bound, then judged by the
    // grammar — which is what makes the bound inclusive rather than one
    // byte short.
    let header = "lockfileVersion: '9.0'\npackages:\n";
    let mut exact = String::from(header);
    exact.push_str(&"#".repeat(PNPM_LIMIT - header.len()));
    assert_eq!(exact.len(), PNPM_LIMIT);
    fs::write(&path, &exact).unwrap();
    let error = read_pnpm(&path)
        .map(|text| pnpm_dependencies(&text, &[]))
        .unwrap()
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "pnpm lock is unreadable: a comment or document marker",
        "the exact limit reaches grammar validation"
    );

    // One byte more: refused by the bound, before the grammar sees it.
    fs::write(&path, format!("{exact}#")).unwrap();
    let error = read_pnpm(&path).unwrap_err();
    assert_eq!(
        error.to_string(),
        "pnpm lock is unreadable: pnpm lock exceeds 8388608-byte limit"
    );
    assert_eq!(PNPM_LIMIT, 8_388_608);
}

/// A source that counts every byte the reader pulls from it. The count
/// is the test's own observation, taken from outside production, so the
/// claim "at most 8,388,609 bytes are consumed" is a number that can be
/// wrong rather than a sentence in a comment.
struct Counting<'a> {
    inner: std::io::Cursor<&'a [u8]>,
    consumed: &'a std::cell::Cell<usize>,
}

impl std::io::Read for Counting<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let pulled = self.inner.read(buf)?;
        self.consumed.set(self.consumed.get() + pulled);
        Ok(pulled)
    }
}

/// The bound is applied to the bytes CONSUMED, and the consumption is
/// counted: a source holding 4,096 bytes more than the limit gives up
/// exactly 8,388,609 of them before the refusal, and a source of exactly
/// the limit gives up all of it and is asked for nothing more.
///
/// The inherited control here read `/dev/zero`, which is not a control at
/// all: with the `take` removed it does not fail an assertion, it reads
/// until the host is exhausted. Both sources below are FINITE, so
/// removing the bound moves the count from 8,388,609 to 8,392,704 and the
/// assertion fails in bounded time (council return 2026-09-19, finding 4).
#[test]
fn the_pnpm_reader_consumes_at_most_one_byte_past_the_limit() {
    const SURPLUS: usize = 4_096;
    let source = vec![b'#'; PNPM_LIMIT + SURPLUS];
    let path = Path::new("counted-pnpm-lock.yaml");

    let consumed = std::cell::Cell::new(0);
    assert_eq!(
        refused(read_pnpm_from(
            path,
            Counting {
                inner: std::io::Cursor::new(&source),
                consumed: &consumed,
            },
        )),
        "pnpm lock is unreadable: pnpm lock exceeds 8388608-byte limit"
    );
    assert_eq!(
        consumed.get(),
        PNPM_LIMIT + 1,
        "exactly one byte past the limit is consumed, of a source holding {SURPLUS} more"
    );

    // Exactly the limit: every byte is consumed, the bound is not
    // reached, and the reader hands the text on to the grammar.
    let consumed = std::cell::Cell::new(0);
    let text = read_pnpm_from(
        path,
        Counting {
            inner: std::io::Cursor::new(&source[..PNPM_LIMIT]),
            consumed: &consumed,
        },
    )
    .unwrap();
    assert_eq!(text.len(), PNPM_LIMIT);
    assert_eq!(consumed.get(), PNPM_LIMIT, "the whole admissible source");

    // The path names the source in a refusal the source itself raises.
    let failing = std::io::Cursor::new(&b"lockfileVersion: '9.0'\n\xff"[..]);
    assert_eq!(
        refused(read_pnpm_from(path, failing)),
        "pnpm lock is unreadable: counted-pnpm-lock.yaml: not UTF-8"
    );
}

#[test]
fn the_pnpm_reader_refuses_an_absent_unreadable_or_non_utf8_lock() {
    let dir = tempfile::tempdir().unwrap();
    // Each of the three is asserted by the reason the reader gave, and
    // the three reasons differ: the open failure, the read failure and
    // the encoding failure are separate arms, and a bare `is_err()`
    // could not tell which one answered.
    let absent = dir.path().join("absent.yaml");
    assert_eq!(
        refused(read_pnpm(&absent)),
        format!(
            "pnpm lock is unreadable: {}: {}",
            absent.display(),
            std::fs::File::open(&absent).unwrap_err()
        )
    );

    // A directory OPENS as a file and then refuses to be read, so the
    // refusal comes from `read_to_end` rather than from `File::open`.
    let directory = dir.path().join("as-a-directory");
    fs::create_dir_all(&directory).unwrap();
    let read_failure = {
        let mut sink = Vec::new();
        std::fs::File::open(&directory)
            .unwrap()
            .read_to_end(&mut sink)
            .unwrap_err()
    };
    assert_eq!(
        refused(read_pnpm(&directory)),
        format!(
            "pnpm lock is unreadable: {}: {read_failure}",
            directory.display()
        )
    );

    let raw = dir.path().join("raw.yaml");
    fs::write(&raw, b"lockfileVersion: '9.0'\n\xff\n").unwrap();
    assert_eq!(
        refused(read_pnpm(&raw)),
        format!("pnpm lock is unreadable: {}: not UTF-8", raw.display())
    );
}

#[test]
fn the_plugin_component_is_bytewise_path_order_and_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    for file in PLUGIN_FILES {
        write(dir.path(), file, file.as_bytes());
    }
    let component = component_of(dir.path(), &PLUGIN_FILES).unwrap();
    // The expectation is a LITERAL recorded from the sole producer, not a
    // second serialization of the same stream. The inherited assertion
    // assembled `<path>\0<digest>\n` lines and hashed them here, which
    // made this test a competing producer of the component (security
    // hold 2026-09-20, finding 5; `no_test_reassembles_the_component_stream`
    // keeps that block from returning).
    //
    // Provenance: `component_digest` over `plugin_file_digests` at
    // `slice-dsh-composite-b` `ab2d8e1d`, on the synthetic set above —
    // the six `PLUGIN_FILES` paths, each file holding its own relative
    // path's bytes with no trailing newline. The per-file input hashes
    // beside it are independent, and permitted.
    assert_eq!(
        component, "8894f23eef97b42abfda88b6dd42c4b44b17ac4cb7e6df14c6bda687ae534c99",
        "the producer's recorded output over the synthetic six-file set"
    );
    let inputs =
        plugin_file_digests("plugin", dir.path(), &PLUGIN_FILES, &read_dir_entries).unwrap();
    for file in PLUGIN_FILES {
        assert_eq!(inputs[file], digest_of(file.as_bytes()), "{file}");
    }
    // A changed byte in any one input moves the component.
    let changed = tempfile::tempdir().unwrap();
    for file in PLUGIN_FILES {
        write(changed.path(), file, file.as_bytes());
    }
    write(changed.path(), "lib/index.js", b"lib/index.js!");
    assert_ne!(
        component_of(changed.path(), &PLUGIN_FILES).unwrap(),
        component
    );
    assert_eq!(component.len(), 64);
    assert!(component
        .chars()
        .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)));

    // The one permitted extra entry: a DIRECT, real, non-symlink
    // dependency subtree, granted on its metadata and neither traversed
    // nor hashed, so the component is unchanged by what it holds.
    let nested = tempfile::tempdir().unwrap();
    for file in PLUGIN_FILES {
        write(nested.path(), file, file.as_bytes());
    }
    write(nested.path(), "node_modules/dep/index.js", b"x");
    assert_eq!(
        component_of(nested.path(), &PLUGIN_FILES).unwrap(),
        component
    );

    // A DEEPER `node_modules/` is not that exception.
    write(nested.path(), "lib/node_modules/dep/index.js", b"x");
    let error = component_of(nested.path(), &PLUGIN_FILES).unwrap_err();
    assert_eq!(
        error.to_string(),
        "plugin component is unreadable: unexpected directory 'lib/node_modules'"
    );
    fs::remove_dir_all(nested.path().join("lib/node_modules")).unwrap();

    // Any other extra file, and then a missing expected one, each name
    // themselves. The extra file is removed before the required one is
    // deleted, so each arm is reached by its own cause.
    write(nested.path(), "extra.txt", b"x");
    let error = component_of(nested.path(), &PLUGIN_FILES).unwrap_err();
    assert_eq!(
        error.to_string(),
        "plugin component is unreadable: unexpected entry 'extra.txt'"
    );
    fs::remove_file(nested.path().join("extra.txt")).unwrap();
    fs::remove_file(nested.path().join("LICENSE")).unwrap();
    let error = component_of(nested.path(), &PLUGIN_FILES).unwrap_err();
    assert_eq!(
        error.to_string(),
        "plugin component is unreadable: missing expected file 'LICENSE'"
    );
}

/// An empty directory the declared set does not need is drift, not an
/// entry to walk past: `lib/` is admitted only because two declared files
/// live under it, and an empty `lib/` still fails on what is missing.
#[test]
fn only_an_ancestor_of_a_declared_file_is_a_walkable_directory() {
    let dir = tempfile::tempdir().unwrap();
    for file in PLUGIN_FILES {
        write(dir.path(), file, file.as_bytes());
    }
    fs::create_dir_all(dir.path().join("empty")).unwrap();
    let error = component_of(dir.path(), &PLUGIN_FILES).unwrap_err();
    assert_eq!(
        error.to_string(),
        "plugin component is unreadable: unexpected directory 'empty'"
    );
    fs::remove_dir(dir.path().join("empty")).unwrap();

    fs::remove_file(dir.path().join("lib/index.js")).unwrap();
    fs::remove_file(dir.path().join("lib/startup.js")).unwrap();
    let error = component_of(dir.path(), &PLUGIN_FILES).unwrap_err();
    assert_eq!(
        error.to_string(),
        "plugin component is unreadable: missing expected file 'lib/index.js'"
    );
}

#[test]
fn the_committed_plugin_set_is_the_six_files_and_the_one_expression_delta() {
    let dir = plugin_dir();
    let found = plugin_file_digests("plugin", &dir, &PLUGIN_FILES, &read_dir_entries).unwrap();
    let names: Vec<&str> = found.keys().map(String::as_str).collect();
    assert_eq!(names, PLUGIN_FILES.to_vec());
    // The full committed-adapted digest map from PROVENANCE.md, not just
    // the filenames: a coordinated byte edit that kept the names would
    // still fail here. The five upstream-attributed files and the one
    // adapted JavaScript file are pinned together.
    let expected: BTreeMap<String, String> = COMMITTED_PLUGIN_DIGESTS
        .into_iter()
        .map(|(name, digest)| (name.to_string(), digest.to_string()))
        .collect();
    assert_eq!(found, expected);

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
}

#[test]
fn the_canonical_composite_orders_lines_and_moves_with_its_inputs() {
    let npm = vec!["debug 4.4.3 sha512-D".to_string()];
    let pnpm = vec![
        "debug 4.4.3 sha512-D".to_string(),
        "zzz 1.0.0 sha512-Z".to_string(),
    ];
    let base = composite(
        "@deepseek-ai/dsh 0.1.5-rc.2 sha512-C",
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
            "@deepseek-ai/dsh 0.1.5-rc.2 sha512-C",
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
        ),
        "an equal complete triple from both locks is ONE dependency line"
    );

    // EVERY line the serializer emits moves the composite, and each row
    // is named: a serializer that dropped one line fails at the element
    // it dropped rather than at an anonymous inequality. Each row changes
    // exactly one input against the base above.
    const CORE: &str = "@deepseek-ai/dsh 0.1.5-rc.2 sha512-C";
    const NODE: &str = "v22.23.2";
    let bundles = ["base".to_string(), "headless".to_string()];
    let swapped = ["headless".to_string(), "base".to_string()];
    let npm_moved = vec!["debug 4.4.3 sha512-E".to_string()];
    let pnpm_moved = vec![
        "debug 4.4.3 sha512-D".to_string(),
        "zzz 1.0.0 sha512-Y".to_string(),
    ];
    #[allow(clippy::type_complexity)]
    let rows: [(
        &str,
        &str,
        &str,
        &[String],
        &[String],
        &str,
        &str,
        &str,
        &[String],
        &str,
        &str,
        Option<&str>,
    ); 11] = [
        (
            "core",
            "@deepseek-ai/dsh 0.1.5-rc.2 sha512-X",
            NODE,
            &npm,
            &pnpm,
            "aaaa",
            "bbbb",
            "cccc",
            &bundles,
            "startup",
            "absent",
            None,
        ),
        (
            "node", CORE, "v22.23.3", &npm, &pnpm, "aaaa", "bbbb", "cccc", &bundles, "startup",
            "absent", None,
        ),
        (
            "dependency (from npm)",
            CORE,
            NODE,
            &npm_moved,
            &pnpm,
            "aaaa",
            "bbbb",
            "cccc",
            &bundles,
            "startup",
            "absent",
            None,
        ),
        (
            "dependency (from pnpm)",
            CORE,
            NODE,
            &npm,
            &pnpm_moved,
            "aaaa",
            "bbbb",
            "cccc",
            &bundles,
            "startup",
            "absent",
            None,
        ),
        (
            "plugin", CORE, NODE, &npm, &pnpm, "aaab", "bbbb", "cccc", &bundles, "startup",
            "absent", None,
        ),
        (
            "plugin-patch",
            CORE,
            NODE,
            &npm,
            &pnpm,
            "aaaa",
            "bbbc",
            "cccc",
            &bundles,
            "startup",
            "absent",
            None,
        ),
        (
            "profile-patch",
            CORE,
            NODE,
            &npm,
            &pnpm,
            "aaaa",
            "bbbb",
            "cccd",
            &bundles,
            "startup",
            "absent",
            None,
        ),
        (
            "profile-bundle (declared order)",
            CORE,
            NODE,
            &npm,
            &pnpm,
            "aaaa",
            "bbbb",
            "cccc",
            &swapped,
            "startup",
            "absent",
            None,
        ),
        (
            "profile-patch-reload",
            CORE,
            NODE,
            &npm,
            &pnpm,
            "aaaa",
            "bbbb",
            "cccc",
            &bundles,
            "live",
            "absent",
            None,
        ),
        (
            "home-patch",
            CORE,
            NODE,
            &npm,
            &pnpm,
            "aaaa",
            "bbbb",
            "cccc",
            &bundles,
            "startup",
            "sha256-home",
            None,
        ),
        (
            "extension",
            CORE,
            NODE,
            &npm,
            &pnpm,
            "aaaa",
            "bbbb",
            "cccc",
            &bundles,
            "startup",
            "absent",
            Some("dddd"),
        ),
    ];
    for (
        element,
        core,
        node,
        npm,
        pnpm,
        plugin,
        plugin_patch,
        profile_patch,
        bundles,
        reload,
        home,
        extension,
    ) in rows
    {
        let moved = composite(
            core,
            node,
            npm,
            pnpm,
            plugin,
            plugin_patch,
            profile_patch,
            bundles,
            reload,
            home,
            extension,
        );
        assert_ne!(base, moved, "the `{element}` line moves the composite");
    }
}

/// A synthetic qualified install: the task-owned core with its hidden
/// lock, the `headless` profile with its manifest, patch and pnpm lock,
/// and the committed plugin set under the profile. Everything is built
/// in a temporary directory; nothing here reads `.forge/`.
///
/// This fixture is deliberately SYNTHETIC and small: it is where the
/// refusal vectors and the two optional layouts live. The measured rc.2
/// ground truth has its own fixture below.
struct Synthetic {
    dir: FixtureRoot,
    seams: DshSeams,
}

/// The head a selection would retain for `path`: its first bytes,
/// through the same bound production reads, taken BEFORE anything runs
/// the file. A fixture that hands the seams these bytes is a fixture
/// whose observation reads what selection read (review 2026-09-20, F6).
fn selected_head(path: &Path) -> Vec<u8> {
    let bytes = fs::read(path).unwrap();
    bytes[..bytes.len().min(SHEBANG_BOUND)].to_vec()
}

impl Synthetic {
    fn new() -> Synthetic {
        let dir = FixtureRoot::new();
        let root = dir.path().to_path_buf();
        let core = root.join("core");
        let pkg = core.join("node_modules").join("@deepseek-ai").join("dsh");
        write(
            &pkg,
            "package.json",
            br#"{"name":"@deepseek-ai/dsh","version":"0.1.5-rc.2","bin":{"dsh":"lib/bin.js"}}"#,
        );
        write_executable(&pkg, "lib/bin.js", b"#!/usr/bin/env node\n");
        write(
            &core,
            "node_modules/.package-lock.json",
            br#"{"lockfileVersion":3,"packages":{
              "node_modules/@deepseek-ai/dsh":{"version":"0.1.5-rc.2","integrity":"sha512-CORE"},
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
            br#"{"name":"@deepseek-ai/dsh-base","version":"0.1.5-rc.2"}"#,
        );
        for file in PLUGIN_FILES {
            write(
                &profile.join("node_modules").join("dsh-plugin-cli-session"),
                file,
                file.as_bytes(),
            );
        }
        let bin = pkg.join("lib/bin.js");
        let head = selected_head(&bin);
        let executable = bin.to_string_lossy().into_owned();
        Synthetic {
            seams: DshSeams {
                executable,
                home: home.clone(),
                node: None,
                head,
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
    for version in [" 1.0.0", "1.0.0 ", "1\t0", "1\r0", "1.0.0\n"] {
        let text = format!(
            "{{\"lockfileVersion\":3,\"packages\":{{\"node_modules/a\":{{\"version\":{},\"integrity\":\"sha512-A\"}}}}}}",
            serde_json::to_string(version).unwrap()
        );
        let error = npm_dependencies(&lock(&text), &[]).unwrap_err();
        assert_eq!(
            error.to_string(),
            "npm lock is unreadable: 'node_modules/a': version carries whitespace",
            "{version:?}"
        );
    }
    // A clean version still reads.
    let clean = r#"{"lockfileVersion":3,"packages":{"node_modules/a":{"version":"1.0.0","integrity":"sha512-A"}}}"#;
    assert_eq!(
        npm_dependencies(&lock(clean), &[]).unwrap(),
        vec!["a 1.0.0 sha512-A".to_string()]
    );
}

// Unix only: the case is a symlinked ancestor, and Windows has no std::os::unix::fs::symlink.
#[cfg(unix)]
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
        node: None,
        head: install.seams.head.clone(),
    };
    let same = dsh_composite_with(&aliased, &install.node(), &[]).unwrap();
    assert_eq!(base.plugin, same.plugin);
    assert_eq!(base.canonical, same.canonical);

    // The lookup ANCHOR is the raw path and the BOUNDARY is canonical,
    // and the two are observably different things. The loader's search
    // walks the ancestors of the anchor it was given, so a `node_modules`
    // beside the alias's own parent is a candidate when the home is
    // reached through that alias and is not one when the home is reached
    // directly. Planting a bundle there — and removing the profile's own
    // copy so the walk gets that far — makes the raw anchor visible: the
    // first hit is the alias-side directory, outside both boundaries, and
    // the refusal names it. A producer that searched from the CANONICAL
    // directory instead would never reach that candidate and would refuse
    // for a different reason (the bundle "does not resolve"), which is
    // the failure that separates an altered anchor from a kept one.
    let aliases = install.dir.path().join("aliases");
    fs::create_dir_all(&aliases).unwrap();
    let nested_alias = aliases.join("alias");
    std::os::unix::fs::symlink(&install.seams.home, &nested_alias).unwrap();
    let decoy = aliases
        .join("node_modules")
        .join("@deepseek-ai")
        .join("dsh-base");
    write(
        &decoy,
        "package.json",
        br#"{"name":"@deepseek-ai/dsh-base","version":"0.1.5-rc.2"}"#,
    );
    fs::remove_dir_all(
        install
            .profile()
            .join("node_modules")
            .join("@deepseek-ai")
            .join("dsh-base"),
    )
    .unwrap();
    let through_nested_alias = DshSeams {
        executable: install.seams.executable.clone(),
        home: nested_alias,
        node: None,
        head: install.seams.head.clone(),
    };
    assert_eq!(
        refused(dsh_composite_with(
            &through_nested_alias,
            &install.node(),
            &[]
        )),
        format!(
            "the DSH layout is unreadable: bundle '@deepseek-ai/dsh-base' resolves outside \
             the core root and the profile ({})",
            decoy.canonicalize().unwrap().display()
        ),
        "the search walked the RAW anchor's ancestry and stopped at its outside first hit"
    );
    // The same install reached directly never sees that candidate.
    assert_eq!(
        refused(dsh_composite_with(&install.seams, &install.node(), &[])),
        "the DSH layout is unreadable: bundle '@deepseek-ai/dsh-base' does not resolve: \
         no package.json found"
    );
}

// Unix only: a broken symlink is the input under test.
#[cfg(unix)]
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
    // The first bundle (`@deepseek-ai/dsh-base`) has NO hit in the
    // sibling, so it resolves validly inside the profile and the search
    // reaches the plugin candidate. Planting one there too would stop
    // this test at the base bundle and never test what it claims to.
    assert!(!sibling.join("@deepseek-ai/dsh-base").exists());
    // The refusal is the bundle lookup's own — the first hit, outside
    // BOTH boundaries, named by its canonical path — and not the later
    // plugin-inside-the-profile guard, which a string-prefix lookup
    // would have left to catch the sibling instead.
    assert_eq!(
        refused(dsh_composite_with(
            &install.seams,
            &install.node(),
            std::slice::from_ref(&sibling),
        )),
        format!(
            "the DSH layout is unreadable: bundle 'dsh-plugin-cli-session' resolves outside \
             the core root and the profile ({})",
            sibling
                .join("dsh-plugin-cli-session")
                .canonicalize()
                .unwrap()
                .display()
        )
    );

    // A profile directory that cannot be canonicalized (a broken
    // symlink) is unreadable rather than compared raw: the refusal names
    // the boundary and the platform's own reason, before any manifest is
    // read.
    let broken = tempfile::tempdir().unwrap();
    let home = broken.path().join("home");
    fs::create_dir_all(home.join("profiles")).unwrap();
    std::os::unix::fs::symlink(home.join("nowhere"), home.join("profiles/headless")).unwrap();
    let boundary = home.join("profiles").join("headless");
    let seams = DshSeams {
        executable: install.seams.executable.clone(),
        home,
        node: None,
        head: install.seams.head.clone(),
    };
    assert_eq!(
        refused(dsh_composite_with(&seams, &install.node(), &[])),
        format!(
            "the DSH layout is unreadable: {}: {}",
            boundary.display(),
            std::fs::canonicalize(&boundary).unwrap_err()
        )
    );
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
    // the profile's own contained copy is later in the order. The refusal
    // names the OUTSIDE hit, so a producer that had gone on to the inside
    // copy could not have written it.
    assert_eq!(
        refused(dsh_composite_with(
            &install.seams,
            &install.node(),
            std::slice::from_ref(&outside)
        )),
        format!(
            "the DSH layout is unreadable: bundle 'dsh-plugin-cli-session' resolves outside \
             the core root and the profile ({})",
            outside
                .join("dsh-plugin-cli-session")
                .canonicalize()
                .unwrap()
                .display()
        )
    );
}

#[test]
fn the_dsh_composite_reads_the_qualified_locators_and_moves_with_them() {
    let install = Synthetic::new();
    let base = install.composite();
    assert_eq!(base.canonical.len(), 64);
    assert_eq!(base.core, "@deepseek-ai/dsh 0.1.5-rc.2 sha512-CORE");
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
    assert_eq!(
        refused(dsh_composite_with(
            &install.seams,
            &install.node(),
            &[install.dir.path().join("global")],
        )),
        format!(
            "the DSH layout is unreadable: bundle '@deepseek-ai/dsh-base' resolves outside \
             the core root and the profile ({})",
            outside.canonicalize().unwrap().display()
        )
    );

    // A core package whose shebang is not `env node` is refused — as
    // SELECTION read it. The head is retaken here because rewriting the
    // file after a selection moves nothing the observation reads; that
    // is the retained-head requirement, proved on its own below (review
    // 2026-09-20, F6).
    let mut install = Synthetic::new();
    let bin = std::path::PathBuf::from(&install.seams.executable);
    write_executable(bin.parent().unwrap(), "bin.js", b"#!/bin/sh\n");
    install.seams.head = selected_head(&bin);
    assert_eq!(
        refused(dsh_composite_with(&install.seams, &install.node(), &[])),
        format!(
            "the DSH layout is unreadable: {}: first line is not the env node shebang",
            bin.canonicalize().unwrap().display()
        )
    );

    // A profile that does not list the plugin is refused.
    let install = Synthetic::new();
    write(
        &install.profile(),
        "package.json",
        br#"{"dsh":{"profile":{"bundles":["@deepseek-ai/dsh-base"],"patchReload":"live"}}}"#,
    );
    assert_eq!(
        refused(dsh_composite_with(&install.seams, &install.node(), &[])),
        "the DSH layout is unreadable: the profile does not list dsh-plugin-cli-session"
    );
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
    assert_eq!(
        refused(dsh_composite_with(&install.seams, &install.node(), &[])),
        "the DSH layout is unreadable: bundle 'brokkr-dsh-resume-policy' does not resolve: \
         no package.json found"
    );

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

/// A bundle candidate this producer could not INSPECT is unreadable, and
/// the search stops there. `Path::is_file` answered `false` for a
/// permission failure exactly as it answers `false` for absence, so the
/// resolver walked past a candidate the Node loader would have taken and
/// described a DIFFERENT installed copy further down the chain (council
/// return 2026-09-19, F3).
#[cfg(unix)]
#[test]
fn a_bundle_candidate_that_cannot_be_inspected_stops_the_search() {
    use std::os::unix::fs::PermissionsExt;

    let install = Synthetic::new();
    // The profile copy — the one the search would reach next — is valid
    // and complete, so nothing but the sealed candidate can explain the
    // refusal.
    assert!(install.composite().plugin.len() == 64);

    // The FIRST candidate on the chain: `<core>/node_modules/@deepseek-ai
    // /dsh/node_modules/<bundle>`, sealed so its manifest cannot be
    // stat'ed at all.
    let sealed = install
        .seams
        .home
        .parent()
        .unwrap()
        .join("core/node_modules/@deepseek-ai/dsh/node_modules/dsh-plugin-cli-session");
    write(&sealed, "package.json", b"{}");
    fs::set_permissions(&sealed, fs::Permissions::from_mode(0o000)).unwrap();
    let result = dsh_composite_with(&install.seams, &install.node(), &[]);
    fs::set_permissions(&sealed, fs::Permissions::from_mode(0o700)).unwrap();

    let error = refused(result);
    assert!(
        error.starts_with(&format!(
            "the DSH layout is unreadable: bundle 'dsh-plugin-cli-session': {}/package.json cannot be read: ",
            sealed.display()
        )),
        "the refusal names the candidate it could not inspect: {error}"
    );

    // With the sealed candidate GONE the same layout resolves to the
    // profile copy, which is what makes the refusal above the candidate's
    // and not the fixture's. (Unsealing alone is not the control: that
    // candidate then resolves inside the core root, and a plugin outside
    // the profile is its own refusal.)
    fs::remove_dir_all(&sealed).unwrap();
    assert_eq!(install.composite().plugin.len(), 64);

    // A manifest that is present but is not a regular file is the same
    // kind of failure, and is not absence either.
    let shadow = install
        .profile()
        .join("node_modules/brokkr-dsh-resume-policy/package.json");
    fs::create_dir_all(&shadow).unwrap();
    write(
        &install.profile(),
        "package.json",
        br#"{"dsh":{"profile":{"bundles":["dsh-plugin-cli-session","brokkr-dsh-resume-policy"],"patchReload":"startup"}}}"#,
    );
    assert_eq!(
        refused(dsh_composite_with(&install.seams, &install.node(), &[])),
        format!(
            "the DSH layout is unreadable: bundle 'brokkr-dsh-resume-policy': {} is not a regular file",
            shadow.display()
        )
    );
}

/// D6's measured executable is `<core>/lib/bin.js` carrying EXACTLY
/// `#!/usr/bin/env node`. Both are checked, and each on its own: a core
/// whose manifest agrees with the executable it ships still has to be
/// at the measured locator, and a CRLF shebang is not the measured first
/// line (council return 2026-09-19, F5).
#[test]
fn the_core_executable_must_be_lib_bin_js_with_the_exact_shebang() {
    let dir = FixtureRoot::new();
    let root = dir.path();

    // A manifest that names a DIFFERENT file, which exists and carries
    // the right shebang, beside a `lib/bin.js` that also exists. The
    // manifest/executable equality holds; the locator does not.
    let pkg = root.join("node_modules/@deepseek-ai/dsh");
    write(
        &pkg,
        "package.json",
        br#"{"name":"@deepseek-ai/dsh","version":"1.0.0","bin":{"dsh":"lib/other.js"}}"#,
    );
    write_executable(&pkg, "lib/bin.js", b"#!/usr/bin/env node\n");
    write_executable(&pkg, "lib/other.js", b"#!/usr/bin/env node\n");
    write(root, "node_modules/.package-lock.json", CORE_LOCK);
    let other = pkg.join("lib/other.js");
    let error = refused(resolve_core(other.to_str().unwrap()));
    assert_eq!(
        error,
        format!(
            "the DSH layout is unreadable: {} is not the core package's lib/bin.js ({})",
            other.canonicalize().unwrap().display(),
            pkg.join("lib/bin.js").canonicalize().unwrap().display()
        )
    );

    // The same core with the measured locator resolves, so the refusal
    // above is the locator's alone.
    let bin = pkg.join("lib/bin.js");
    write(
        &pkg,
        "package.json",
        br#"{"name":"@deepseek-ai/dsh","version":"1.0.0","bin":{"dsh":"lib/bin.js"}}"#,
    );
    assert_eq!(
        resolve_core(bin.to_str().unwrap()).unwrap().version,
        "1.0.0"
    );

    // A CRLF shebang. The kernel would read `env node\r`, which is not a
    // program; the inherited reader stripped the CR and called it the
    // measured first line.
    write_executable(&pkg, "lib/bin.js", b"#!/usr/bin/env node\r\nrest\n");
    assert_eq!(
        refused(resolve_core(bin.to_str().unwrap())),
        format!(
            "the DSH layout is unreadable: {}: first line is not the env node shebang",
            bin.canonicalize().unwrap().display()
        )
    );

    // And a core whose `lib/bin.js` is missing altogether is named by the
    // locator it could not canonicalize, never by a raw-path fallback.
    // The locator is spelled from the fixture's canonical root, which is
    // the only spelling available for a file that does not exist.
    let bare = FixtureRoot::new();
    let pkg = bare.path().join("node_modules/@deepseek-ai/dsh");
    write(
        &pkg,
        "package.json",
        br#"{"name":"@deepseek-ai/dsh","version":"1.0.0","bin":{"dsh":"lib/other.js"}}"#,
    );
    write_executable(&pkg, "lib/other.js", b"#!/usr/bin/env node\n");
    write(bare.path(), "node_modules/.package-lock.json", CORE_LOCK);
    let error = refused(resolve_core(pkg.join("lib/other.js").to_str().unwrap()));
    assert!(
        error.starts_with(&format!(
            "the DSH layout is unreadable: {}: ",
            pkg.join("lib/bin.js").display()
        )),
        "{error}"
    );
}

/// Each unreadable source names the COMPONENT whose failure it is. The
/// hidden npm lock reported as "the DSH layout" sent an operator holding
/// a corrupt lock to look at directories, and extension drift reported as
/// "plugin component" sent them to the wrong installed package (council
/// return 2026-09-19, F9).
#[test]
fn an_unreadable_npm_lock_and_extension_are_named_by_their_own_component() {
    let install = Synthetic::new();
    let core = install.seams.home.parent().unwrap().join("core");

    // The hidden npm lock: corrupt, then absent.
    let lock = core.join("node_modules/.package-lock.json");
    fs::write(&lock, b"not json").unwrap();
    let error = refused(dsh_composite_with(&install.seams, &install.node(), &[]));
    assert!(
        error.starts_with(&format!(
            "npm lock is unreadable: {}: not JSON: ",
            lock.display()
        )),
        "{error}"
    );
    fs::remove_file(&lock).unwrap();
    let error = refused(dsh_composite_with(&install.seams, &install.node(), &[]));
    assert!(
        error.starts_with(&format!("npm lock is unreadable: {}: ", lock.display())),
        "{error}"
    );
    // A lock that parses but has no core record is the lock's failure too.
    fs::write(&lock, br#"{"packages":{}}"#).unwrap();
    assert_eq!(
        refused(dsh_composite_with(&install.seams, &install.node(), &[])),
        "npm lock is unreadable: no node_modules/@deepseek-ai/dsh entry"
    );
    fs::write(
        &lock,
        br#"{"packages":{
          "node_modules/@deepseek-ai/dsh":{"version":"0.1.5-rc.2","integrity":"sha512-CORE"},
          "node_modules/debug":{"version":"2.6.9","integrity":"sha512-DEBUG"}
        }}"#,
    )
    .unwrap();

    // The conditional extension's own file set, drifted.
    write(
        &install.profile(),
        "package.json",
        br#"{"dsh":{"profile":{"bundles":["dsh-plugin-cli-session","brokkr-dsh-resume-policy"],"patchReload":"startup"}}}"#,
    );
    let extension = install
        .profile()
        .join("node_modules/brokkr-dsh-resume-policy");
    for file in EXTENSION_FILES {
        write(&extension, file, file.as_bytes());
    }
    assert!(install.composite().extension.is_some());
    fs::remove_file(extension.join("index.js")).unwrap();
    assert_eq!(
        refused(dsh_composite_with(&install.seams, &install.node(), &[])),
        "extension component is unreadable: missing expected file 'index.js'",
        "the extension's drift is the EXTENSION's, not the plugin's"
    );
    // The plugin's own drift still answers as the plugin's, so the two
    // components are told apart rather than merely renamed.
    write(&extension, "index.js", b"index.js");
    let plugin = install
        .profile()
        .join("node_modules/dsh-plugin-cli-session");
    fs::remove_file(plugin.join("README.md")).unwrap();
    assert_eq!(
        refused(dsh_composite_with(&install.seams, &install.node(), &[])),
        "plugin component is unreadable: missing expected file 'README.md'"
    );
}

const CORE_LOCK: &[u8] =
    br#"{"packages":{"node_modules/@deepseek-ai/dsh":{"version":"1.0.0","integrity":"sha512-X"}}}"#;

fn core_package(root: &Path, manifest: &[u8], lock: &[u8]) -> PathBuf {
    let pkg = root.join("node_modules").join("@deepseek-ai").join("dsh");
    write(&pkg, "package.json", manifest);
    write_executable(&pkg, "lib/bin.js", b"#!/usr/bin/env node\n");
    write(root, "node_modules/.package-lock.json", lock);
    pkg.join("lib/bin.js")
}

#[cfg(unix)]
#[test]
fn executable_resolution_walks_path_entries_and_refuses_a_miss() {
    let dir = tempfile::tempdir().unwrap();
    let tool = stage_executable(dir.path(), "mytool", b"#!/bin/sh\ntrue\n");
    let path = Some(dir.path().as_os_str().to_os_string());
    let found = resolve_executable_in("mytool", path.clone()).unwrap();
    assert_eq!(found, tool.canonicalize().unwrap());
    // An exhausted search names the last candidate and the cause the
    // platform reports for it — here the child's NotFound.
    let enoent = std::fs::metadata(dir.path().join("not-here")).unwrap_err();
    assert_eq!(
        refused(resolve_executable_in("not-here", path.clone())),
        format!(
            "the DSH layout is unreadable: 'not-here' is not on PATH (the search ended at {}: \
             {enoent})",
            dir.path().join("not-here").display()
        )
    );
    // A command carrying `/` is canonicalized, not searched. The refusal
    // names the path and the platform's own reason for it, never "is
    // not on PATH": that difference is the whole of the arm, and an
    // `is_err()` here would have passed on either.
    let command = "/definitely/not/here";
    let reason = std::fs::canonicalize(command).unwrap_err();
    assert_eq!(
        refused(resolve_executable_in(command, None)),
        format!("the DSH layout is unreadable: {command}: {reason}")
    );
    // A backslash is an ordinary filename byte on Unix, so `a\b` is a
    // NAME, searched on PATH like any other and missing from this one.
    // The inherited classifier read it as a path (security hold
    // 2026-09-20, S1b); the cwd-executing consequence is proved by the
    // differential matrix and the built-doctor suite.
    assert_eq!(
        refused(resolve_executable_in("a\\b", path.clone())),
        format!(
            "the DSH layout is unreadable: 'a\\b' is not on PATH (the search ended at {}: \
             {enoent})",
            dir.path().join("a\\b").display()
        )
    );
    let named = stage_executable(dir.path(), "a\\b", b"#!/bin/sh\ntrue\n");
    assert_eq!(
        resolve_executable_in("a\\b", path).unwrap(),
        named.canonicalize().unwrap(),
        "a file literally named a\\b in a PATH directory is what native lookup runs"
    );
    // The two spellings refused before any lookup: a NUL, and nothing.
    assert_eq!(
        refused(resolve_executable_in("dsh\0x", None)),
        "the DSH layout is unreadable: 'dsh\\0x' carries a NUL"
    );
    assert_eq!(
        refused(resolve_executable_in("", None)),
        "the DSH layout is unreadable: '' names no program"
    );
}

/// The C library's default search path is asked of the library, and the
/// two answers besides a path — none, and one longer than the buffer —
/// are refusals by name rather than an empty search.
#[cfg(unix)]
#[test]
fn the_default_search_path_is_the_c_librarys_own_answer() {
    let default = default_search_path().unwrap();
    let entries: Vec<PathBuf> = std::env::split_paths(&default).collect();
    assert!(
        !entries.is_empty() && entries.iter().all(|entry| entry.is_absolute()),
        "confstr(_CS_PATH) names absolute directories: {default:?}"
    );
    // Injected: a value that fits, one that needs a second, larger
    // buffer, and the two refusals.
    let fits = default_search_path_from(|buf| {
        buf[..9].copy_from_slice(b"/bin:/usr");
        10
    })
    .unwrap();
    assert_eq!(fits, OsString::from("/bin:/usr"));
    let long = "/".repeat(5000);
    let grown = default_search_path_from(|buf| {
        let count = buf.len().min(long.len());
        buf[..count].copy_from_slice(&long.as_bytes()[..count]);
        long.len() + 1
    })
    .unwrap();
    assert_eq!(grown.len(), 5000);
    assert_eq!(
        refused(default_search_path_from(|_| 0)),
        "the DSH layout is unreadable: the C library reports no default search path for an absent PATH"
    );
    assert_eq!(
        refused(default_search_path_from(|buf| buf.len() + 1)),
        "the DSH layout is unreadable: the C library reports no default search path for an absent PATH"
    );
    // A literal answer is the entries themselves, with no query made.
    assert_eq!(
        default_search_path_for(DefaultSearch::Literal("/usr/bin:/bin")).unwrap(),
        OsString::from("/usr/bin:/bin")
    );
}

/// The absent-`PATH` search is each platform's OWN `execvp` rule, per
/// platform, and not one library call standing in for all of them.
///
/// Apple's `execvP` and `posix_spawnp` search `_PATH_DEFPATH`,
/// `/usr/bin:/bin`. Apple's `confstr(_CS_PATH)` answers
/// `/usr/bin:/bin:/usr/sbin:/sbin` — `USER_CS_PATH` — so asking the
/// library there would put two system directories on a search the
/// loader never walks, and the resolver could select or probe a
/// system executable native lookup would not select (review
/// 2026-09-20, F2, security-relevant). An `sh` positive cannot tell the
/// two searches apart: `sh` sits in `/bin` under both.
///
/// Restoring the overbroad Apple search — `DefaultSearch::Library` for
/// Apple, or `_PATH_DEFPATH` widened to the `confstr` value — fails the
/// assertions below, on whichever platform this suite runs.
#[cfg(unix)]
#[test]
fn each_platforms_absent_path_search_is_its_own_loaders_rule() {
    // Apple: the loader's literal, and never the library query.
    for apple in ["macos", "ios", "tvos", "watchos", "visionos"] {
        assert_eq!(
            default_search_of(apple, ""),
            DefaultSearch::Literal("/usr/bin:/bin"),
            "{apple}"
        );
    }
    // The distinction the `sh` positive cannot make: the two system
    // directories Apple's `confstr` adds are not on its loader's search.
    let DefaultSearch::Literal(apple) = default_search_of("macos", "") else {
        panic!("Apple's search is a literal, not a library query");
    };
    let confstr = "/usr/bin:/bin:/usr/sbin:/sbin";
    assert_ne!(apple, confstr);
    for added in ["/usr/sbin", "/sbin"] {
        assert!(
            confstr.contains(added) && !apple.contains(added),
            "{added} is on Apple's confstr answer and not on its execvp search"
        );
    }
    // glibc: its own `execvp` reads `confstr(_CS_PATH)`, so the
    // library's answer IS the child's search.
    assert_eq!(default_search_of("linux", "gnu"), DefaultSearch::Library);
    // musl: a literal its own `confstr` does not report.
    assert_eq!(
        default_search_of("linux", "musl"),
        DefaultSearch::Literal("/usr/local/bin:/bin:/usr/bin")
    );
    // FreeBSD's `_PATH_DEFPATH` is its own (`include/paths.h` 36–40),
    // in its own order, and NOT Apple's: the constant read from one
    // BSD's header was assigned to every BSD (run `09ec8d81`, R2).
    assert_eq!(
        default_search_of("freebsd", ""),
        DefaultSearch::Literal("/sbin:/bin:/usr/sbin:/usr/bin:/usr/local/sbin:/usr/local/bin")
    );
    assert_ne!(
        default_search_of("freebsd", ""),
        default_search_of("macos", "")
    );
    // A target whose source this resolver has not read gets no other
    // target's literal: Android's bionic, the other BSDs, illumos, and
    // a Linux C library that is neither glibc nor musl are each an
    // explicit limitation the refusal names.
    for (os, env) in [
        ("android", ""),
        ("openbsd", ""),
        ("netbsd", ""),
        ("dragonfly", ""),
        ("illumos", ""),
        ("linux", ""),
        ("linux", "uclibc"),
    ] {
        assert_eq!(
            default_search_of(os, env),
            DefaultSearch::Unestablished,
            "{os}/{env}"
        );
    }
    assert_eq!(
        refused(default_search_path_for(DefaultSearch::Unestablished)),
        "the DSH layout is unreadable: PATH is absent, and this target's native default search \
         is not established"
    );
    // And production takes the RUNNING target's row, not a fixed one.
    assert_eq!(
        default_search_path().unwrap(),
        default_search_path_for(default_search_of(std::env::consts::OS, TARGET_ENV)).unwrap()
    );
}

/// The search is the CHILD's. A candidate that is present but not
/// executable is walked past exactly as a spawning child walks past it,
/// so this resolver and `Command` agree on which file they are naming.
///
/// The inherited reader stopped at the first regular FILE, so a
/// `PATH=A:B` holding a non-executable `A/dsh` beside an executable
/// `B/dsh` paired B's version with A's composite (council return
/// 2026-09-19, F4).
#[cfg(unix)]
#[test]
fn path_resolution_walks_past_a_candidate_a_child_could_not_execute() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("first");
    let second = dir.path().join("second");
    fs::create_dir_all(&first).unwrap();
    fs::create_dir_all(&second).unwrap();

    // Present, readable, a regular file — and not executable.
    let decoy = first.join("dsh");
    fs::write(&decoy, b"#!/bin/sh\ntrue\n").unwrap();
    fs::set_permissions(&decoy, fs::Permissions::from_mode(0o644)).unwrap();
    let real = stage_executable(&second, "dsh", b"#!/bin/sh\ntrue\n");

    let path = Some(OsString::from(format!(
        "{}:{}",
        first.display(),
        second.display()
    )));
    assert_eq!(
        resolve_executable_in("dsh", path).unwrap(),
        real.canonicalize().unwrap(),
        "the non-executable candidate is walked past, as a spawning child walks past it"
    );

    // A directory named like the command is not a candidate, and neither
    // is one whose metadata cannot be read at all.
    let shadow = dir.path().join("shadow");
    fs::create_dir_all(shadow.join("dsh")).unwrap();
    let sealed = dir.path().join("sealed");
    fs::create_dir_all(&sealed).unwrap();
    stage_executable(&sealed, "dsh", b"#!/bin/sh\ntrue\n");
    fs::set_permissions(&sealed, fs::Permissions::from_mode(0o000)).unwrap();
    let path = Some(OsString::from(format!(
        "{}:{}:{}",
        shadow.display(),
        sealed.display(),
        second.display()
    )));
    let resolved = resolve_executable_in("dsh", path);
    fs::set_permissions(&sealed, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(resolved.unwrap(), real.canonicalize().unwrap());

    // Nothing executable anywhere on the search is the named refusal —
    // and the name is the child's: a search whose only candidate was
    // denied ends in EACCES, which the refusal carries rather than
    // reporting an absence the child did not see (review 2026-09-20,
    // R9). A search with no candidate at all is the plain no-match.
    assert_eq!(
        refused(resolve_executable_in(
            "dsh",
            Some(OsString::from(first.display().to_string()))
        )),
        format!(
            "the DSH layout is unreadable: 'dsh' is not executable by this process on PATH: {}: \
             is not executable by this process",
            decoy.display()
        )
    );
    let nowhere = dir.path().join("nowhere");
    let enoent = fs::metadata(nowhere.join("dsh")).unwrap_err();
    assert_eq!(
        refused(resolve_executable_in(
            "dsh",
            Some(OsString::from(nowhere.display().to_string()))
        )),
        format!(
            "the DSH layout is unreadable: 'dsh' is not on PATH (the search ended at {}: {enoent})",
            nowhere.join("dsh").display()
        )
    );

    // The PERMISSION CLASS a mode-bit test cannot see. Mode 0641 carries
    // an execute bit for OTHERS and none for its owner, so
    // `mode & 0o111 != 0` answered yes while the owner's own child got
    // `EACCES` and walked on to a later entry — the same two-installs
    // disagreement, reached by permission rather than by absence
    // (council return 2026-09-19, finding 3).
    let other_only = dir.path().join("other-only");
    fs::create_dir_all(&other_only).unwrap();
    let owner_denied = other_only.join("dsh");
    fs::write(&owner_denied, b"#!/bin/sh\ntrue\n").unwrap();
    fs::set_permissions(&owner_denied, fs::Permissions::from_mode(0o641)).unwrap();
    // POSIX grants a PRIVILEGED process `X_OK` on any regular file
    // carrying some execute bit, so a root run and an unprivileged run
    // disagree about this file honestly. The test asserts whichever one
    // it is in rather than skipping; the detector is a mode-0000 file,
    // which only a privileged process can read.
    let sealed_file = dir.path().join("sealed-file");
    fs::write(&sealed_file, b"x").unwrap();
    fs::set_permissions(&sealed_file, fs::Permissions::from_mode(0o000)).unwrap();
    let privileged = fs::read(&sealed_file).is_ok();
    let expected = match privileged {
        true => owner_denied.canonicalize().unwrap(),
        false => real.canonicalize().unwrap(),
    };
    assert_eq!(
        resolve_executable_in(
            "dsh",
            Some(OsString::from(format!(
                "{}:{}",
                other_only.display(),
                second.display()
            )))
        )
        .unwrap(),
        expected,
        "the candidate is taken exactly when THIS process could execute it"
    );
    // The same question asked of the access probe alone, so the
    // resolver's answer above is not its only evidence: a mode-0644
    // regular file is executable to nobody and a mode-0755 one to
    // everybody, on a privileged run and an unprivileged one alike,
    // while mode 0641 is exactly the case the two runs part on.
    // The answer is the kernel's own errno: EACCES, the denial a search
    // remembers, and not a boolean that would fold every other failure
    // into it.
    assert_eq!(
        effective_exec_access(&decoy),
        Err(rustix::io::Errno::ACCESS)
    );
    assert_eq!(effective_exec_access(&real), Ok(()));
    assert_eq!(effective_exec_access(&owner_denied).is_ok(), privileged);
    assert_eq!(
        effective_exec_access(&dir.path().join("absent")),
        Err(rustix::io::Errno::NOENT)
    );
}

/// An EMPTY `PATH` entry names the current directory — POSIX's rule, and
/// the one a spawning child follows. The inherited reader skipped it, so
/// a command the child WOULD have found was reported as missing.
///
/// The working directory is process-wide state, so the positive case runs
/// in a CHILD of this test binary whose working directory is the fixture.
/// Changing it in-process would reach every other test in the run.
/// An empty `PATH` entry IS the working directory to the child, at its
/// position in the search — and the working directory is where this
/// resolver never goes: when the search reaches an empty entry it
/// refuses by name, whether or not a candidate sits there, and it never
/// walks past the entry to a later one (fourth hold, reconciled rule).
/// A `.` entry is a nonempty relative component, which keeps its native
/// meaning: this change adds no containment policy for aliases of cwd.
#[cfg(unix)]
#[test]
fn an_empty_path_entry_is_the_working_directory_and_is_refused() {
    const CASE: &str = "BROKKR_COMPOSITE_EMPTY_PATH_ENTRY";

    if std::env::var_os(CASE).is_some() {
        // In the child: `mytool` exists only in the working directory the
        // parent chose, and only the empty entry can reach it — so the
        // empty entry is refused, by its position.
        let cwd_reason = |index: usize| {
            format!(
                "the DSH layout is unreadable: mytool: the platform's search would fall into the \
                 working directory: PATH entry {index} is empty"
            )
        };
        assert_eq!(
            refused(resolve_executable_in(
                "mytool",
                Some(OsString::from(":/nonexistent"))
            )),
            cwd_reason(0),
            "the leading empty entry is cwd, and cwd is refused"
        );
        let enoent = std::fs::metadata("/nonexistent/mytool").unwrap_err();
        assert_eq!(
            refused(resolve_executable_in(
                "mytool",
                Some(OsString::from("/nonexistent"))
            )),
            format!(
                "the DSH layout is unreadable: 'mytool' is not on PATH (the search ended at \
                 /nonexistent/mytool: {enoent})"
            ),
            "and nothing else on that search could have found it"
        );
        // A PRESENT, EMPTY `PATH` is one empty entry — cwd — which is a
        // different answer from an absent `PATH`, and so are a leading
        // empty entry, a trailing one and one between two directories
        // (security hold 2026-09-20, S1 and finding 2): each is reached
        // at its position and refused there, and a later `/other` is
        // never searched.
        for (path, index) in [("", 0), ("/nonexistent:", 1), ("/nonexistent::/other", 1)] {
            assert_eq!(
                refused(resolve_executable_in("mytool", Some(OsString::from(path)))),
                cwd_reason(index),
                "PATH={path:?} reaches the working directory and is refused there"
            );
        }
        // An explicit `.` is a nonempty component the child searches as
        // spelled, and so does the resolver.
        let cwd = std::env::current_dir().unwrap().canonicalize().unwrap();
        for path in [".", "/nonexistent:."] {
            assert_eq!(
                resolve_executable_in("mytool", Some(OsString::from(path))).unwrap(),
                cwd.join("mytool"),
                "PATH={path:?} names the working directory as a component"
            );
        }
        // An ABSENT `PATH` is the C library's default search, which
        // holds no `mytool` and never reaches the working directory: the
        // search ends at the default path's last entry.
        let default = default_search_path().unwrap();
        let last = std::env::split_paths(&default)
            .last()
            .unwrap()
            .join("mytool");
        assert_eq!(
            refused(resolve_executable_in("mytool", None)),
            format!(
                "the DSH layout is unreadable: 'mytool' is not on the default search path {} \
                 (PATH is absent) (the search ended at {}: {})",
                default.to_string_lossy(),
                last.display(),
                std::fs::metadata(&last).unwrap_err()
            ),
            "an absent PATH never reaches the working directory"
        );
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    stage_executable(dir.path(), "mytool", b"#!/bin/sh\ntrue\n");
    let mut child = std::process::Command::new(std::env::current_exe().unwrap());
    child
        .args([
            "adapters::composite::tests::an_empty_path_entry_is_the_working_directory_and_is_refused",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ])
        .current_dir(dir.path())
        .env(CASE, "1");
    let output = spawn_retrying_etxtbsy(&mut child);
    let said = String::from_utf8_lossy(&output.stdout).into_owned()
        + &String::from_utf8_lossy(&output.stderr);
    // A libtest filter that matches nothing exits ZERO, which would make
    // this whole control pass while asserting nothing at all.
    assert!(
        said.contains("1 passed") || said.contains("1 failed"),
        "the child ran the case rather than filtering it away: {said}"
    );
    assert!(output.status.success(), "{said}");
}

/// The working-directory refusal names the SEARCHED NAME, whichever
/// library's rule the search translates. The candidate a platform builds
/// for its cwd iteration is not one spelling: glibc and musl build the
/// bare name for an empty entry, Apple builds `./<name>` (`p = "."` in
/// `gen/FreeBSD/exec.c`). A refusal that displayed the candidate
/// therefore reported the SAME refusal of the SAME search as `mytool` on
/// Linux and `./mytool` on macOS (PR #311's macOS leg, 2026-09-21). Each
/// library's rule is a plain test on this host because `Search` carries
/// the library it translates; only the running host's own row is native
/// evidence as well.
///
/// The other half of the rule is asserted beside it: where native STOPS
/// at that candidate the answer is the platform's observation of a FILE,
/// so it keeps the candidate in the bytes the platform built.
#[cfg(unix)]
#[test]
fn the_working_directory_refusal_names_the_searched_name_under_every_library() {
    let search = |library: Library, entries: &str| Search {
        entries: OsString::from(entries),
        default: false,
        library,
        operation: Operation::Spawn,
        env_reference: PathBuf::from(ENV_REFERENCE),
    };
    let reason = |how: &str| {
        format!(
            "the DSH layout is unreadable: mytool: the platform's search would fall into the \
             working directory: {how}"
        )
    };
    for library in [Library::Glibc, Library::Musl, Library::Apple] {
        assert_eq!(
            refused(search(library, ":/nonexistent").find("mytool", &mut Vec::new())),
            reason("PATH entry 0 is empty"),
            "{library:?} reaches the working directory at entry 0"
        );
    }
    // glibc's IMPLICIT cwd iteration after an oversized skip is the same
    // answer about the same name, by its own `how`.
    assert_eq!(
        refused(
            search(
                Library::Glibc,
                &format!("{}:/nonexistent", "x".repeat(5000))
            )
            .find("mytool", &mut Vec::new())
        ),
        reason(
            "glibc skips the 5000-byte component and its next iteration is the empty entry it \
             leaves the cursor on (posix/execvpe.c 118–124, 168)"
        )
    );
    // The native stop at the cwd candidate: the cause is that file's, and
    // so is the path it names.
    let dir = FixtureRoot::new();
    let looping = dir.path().join("mytool");
    std::os::unix::fs::symlink("mytool", &looping).unwrap();
    let stopped = search(Library::Glibc, "").refuse_working_directory(
        "mytool",
        &looping,
        "PATH entry 0 is empty".to_string(),
    );
    assert_eq!(
        stopped.to_string(),
        format!(
            "the DSH layout is unreadable: {}: a symlink loop stops the lookup: {}",
            looping.display(),
            fs::metadata(&looping).unwrap_err()
        ),
        "a stop names the candidate the platform built, not the name searched for"
    );
}

// Unix only: the case is built from POSIX literals — a `:`-separated
// NODE_PATH and `/`-joined expectations — and Windows separates PATH with
// `;` and joins with `\`. The production reader is platform-correct; only
// this fixture's spelling is not.
#[cfg(unix)]
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

/// The mechanism behind #255, proved rather than argued, so the staging
/// helper above cannot quietly regress to a form that does not cure it.
///
/// The helper's product — installed by `rename` from a sibling whose
/// bytes a CHILD wrote — execs. That is the assertion both supported
/// hosts carry (decision 0063), and it is what every fixture in this file
/// depends on.
///
/// The Linux half measures WHY the rename alone is not the cure it was
/// taken for: `execve` refuses while the inode's write count is above
/// zero, and `rename` moves the INODE, so a destination renamed in from a
/// staging file this process still holds open inherits exactly that
/// descriptor. Apple's `exec_check_permissions` does not apply the
/// writer-count check at all, so that errno is a Linux mechanism fact and
/// is asserted only where it holds — never as a portable expectation the
/// macOS leg would have to fail.
#[cfg(unix)]
#[test]
fn a_renamed_shim_inherits_its_writer_and_a_staged_one_carries_none() {
    let dir = tempfile::tempdir().unwrap();
    let body = b"#!/bin/sh\nexit 0\n";
    #[cfg(target_os = "linux")]
    {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;
        let staging = dir.path().join(".probe.staging");
        let renamed = dir.path().join("probe-renamed");
        let mut held = fs::File::create(&staging).unwrap();
        held.write_all(body).unwrap();
        held.flush().unwrap();
        fs::set_permissions(&staging, fs::Permissions::from_mode(0o755)).unwrap();
        fs::rename(&staging, &renamed).unwrap();
        // The descriptor is still open on the same inode, and the rename
        // did nothing about it.
        let refused = std::process::Command::new(&renamed).output().unwrap_err();
        assert_eq!(
            refused.raw_os_error(),
            Some(26),
            "a renamed shim whose writer is still open is Text file busy: {refused}"
        );
        // Closing it here proves nothing further: a fork of ANOTHER
        // thread may still be carrying the same descriptor, which is
        // exactly why a shim this process wrote can never be relied on
        // to exec.
        drop(held);
    }
    // The helper's product is installed by rename AND carries no
    // descriptor of this process's at all, so there is none for any fork
    // to have inherited.
    let staged = stage_executable(dir.path(), "probe-staged", body);
    assert!(std::process::Command::new(&staged)
        .output()
        .unwrap()
        .status
        .success());
    let left: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .filter(|name| name.to_string_lossy().starts_with(".stage-"))
        .collect();
    assert!(
        left.is_empty(),
        "the temporary sibling was renamed into place, not left beside it: {left:?}"
    );
}

#[cfg(unix)]
#[test]
fn spawn_node_runtime_reads_one_version_line_and_refuses_the_rest() {
    let dir = tempfile::tempdir().unwrap();
    // Every shim's bytes are written by a CHILD, so no fork of this
    // process can be holding a write descriptor when `exec` counts them
    // (#255; see `stage_executable`).
    let stage = |name: &str, body: &str| stage_executable(dir.path(), name, body.as_bytes());
    // Each refusal is asserted by its REASON, not merely by being an error.
    // A bare `is_err()` is satisfied by a spawn that failed for an unrelated
    // cause — `Text file busy` on a shim written moments ago, say — so the
    // check under test is never reached and the only trace is a branch the
    // coverage gate reports missing. Name the reason and the test says what
    // it means.
    // Each shim is run under its own spelling: an explicit path's
    // invocation (R2).
    let spawn_at =
        |path: std::path::PathBuf| spawn_node_runtime_at(path.clone(), &DshInvocation::of(path));
    let refusal = |path: std::path::PathBuf| match spawn_at(path) {
        Ok(runtime) => panic!("expected a refusal, got {}", runtime.version),
        Err(error) => error.to_string(),
    };
    // One record with its single terminator, and the same record with a
    // CRLF terminator: the terminator is not part of the version.
    let good = stage("node-good", "#!/bin/sh\nprintf 'v1.2.3\\n'\n");
    assert_eq!(spawn_at(good).unwrap().version, "v1.2.3");
    let crlf = stage("node-crlf", "#!/bin/sh\nprintf 'v1.2.3\\r\\n'\n");
    assert_eq!(spawn_at(crlf).unwrap().version, "v1.2.3");
    // No terminator at all is still one record.
    let bare = stage("node-bare", "#!/bin/sh\nprintf 'v1.2.3'\n");
    assert_eq!(spawn_at(bare).unwrap().version, "v1.2.3");

    let failed = stage("node-fail", "#!/bin/sh\nexit 3\n");
    assert_eq!(
        refusal(failed),
        "the DSH layout is unreadable: node --version exited nonzero"
    );
    let empty = stage("node-empty", "#!/bin/sh\ntrue\n");
    assert_eq!(refusal(empty), "the node value is empty");
    // A version carrying a NUL, a second line, or a banner that `trim()`
    // would have repaired into a plausible version.
    let nul = stage("node-nul", "#!/bin/sh\nprintf 'v1\\0x\\n'\n");
    assert_eq!(refusal(nul), "the node value carries a NUL");
    let multiline = stage("node-multiline", "#!/bin/sh\nprintf 'v1\\nv2\\n'\n");
    assert_eq!(refusal(multiline), "the node value carries whitespace");
    let padded = stage("node-padded", "#!/bin/sh\nprintf '  v1.2.3  \\n'\n");
    assert_eq!(refusal(padded), "the node value carries whitespace");
    let raw = stage("node-raw", "#!/bin/sh\nprintf 'v\\377\\n'\n");
    assert_eq!(
        refusal(raw),
        "the DSH layout is unreadable: node --version did not print UTF-8"
    );
    // A path that cannot be spawned at all.
    assert!(refusal(dir.path().join("absent")).contains("node --version:"));
}

/// Executable selection and home availability are INDEPENDENT
/// requirements, and combined resolution needs both. The inherited
/// premise `resolve().is_ok() == dsh_home().is_some()` equated a home
/// with a resolution, which is false wherever no `dsh` is selectable —
/// HOME=/tmp with PATH=/usr/bin:/bin and no override fails it
/// deterministically (security hold 2026-09-20, finding 5). The four
/// combinations are driven below; the real child control reproduces the
/// commissioned environment.
#[test]
fn dsh_seams_resolve_reads_the_home_and_refuses_a_missing_one() {
    // This test READS the process `DSH_HOME` and asserts what it read;
    // the planner suite beside it sets a temporary one. Without the
    // shared adapter-environment lock the two race and this assertion
    // reports another test's home as this host's fact.
    let _guard = crate::adapters::tests::ADAPTER_ENV.lock().unwrap();
    // The selection is the ADAPTER's, resolved once: the file the
    // declared name resolves to, or a failed selection carrying that
    // name and the lookup's cause. Asserting the literal `dsh` here made
    // this test fail under any configured `BROKKR_DSH_BIN` — an
    // environment an operator running the suite may well have, and one
    // this seat reproduced (council return 2026-09-19, F11).
    let declared = super::super::adapter_binary("BROKKR_DSH_BIN", Some("FORGE_DSH_BIN"), "dsh");
    let home = crate::transcript::dsh_home();
    match (DshSeams::selected(), resolve_executable(&declared)) {
        (Ok(selection), Ok(path)) => {
            assert_eq!(Path::new(&selection.executable), path);
            // A selected executable with a home is a resolution; one
            // without a home keeps the executable beside the named
            // home refusal, and `resolve` is that refusal.
            match &home {
                Some(home) => {
                    let seams = DshSeams::resolve().unwrap();
                    assert_eq!(seams.executable, selection.executable);
                    assert_eq!(&seams.home, home);
                    // Whether THIS host's home admits is the host's
                    // fact; an admitted one holds the same seams.
                    if let Ok(prepared) = &selection.admission {
                        assert_eq!(prepared.seams(), &seams);
                    }
                }
                None => {
                    assert_eq!(
                        selection
                            .admission
                            .as_ref()
                            .unwrap_err()
                            .cause()
                            .to_string(),
                        "the DSH layout is unreadable: no dsh home: set DSH_HOME or HOME"
                    );
                    assert_eq!(
                        refused(DshSeams::resolve()),
                        "the DSH layout is unreadable: no dsh home: set DSH_HOME or HOME"
                    );
                }
            }
        }
        // A failed lookup is a failed SELECTION: the declared spelling
        // and the lookup's cause, and no executable at all — whether or
        // not a home exists. A home alone never makes a resolution.
        (Err(unselected), Err(cause)) => {
            assert_eq!(unselected.declared, declared);
            assert_eq!(unselected.cause, cause);
            assert_eq!(DshSeams::resolve(), Err(cause));
        }
        (selection, lookup) => panic!("selection {selection:?} disagrees with lookup {lookup:?}"),
    }

    // The commissioned control, in a real child: HOME=/tmp, the default
    // system directories as PATH and every override unset. The home is
    // present there by construction, and native lookup decides the rest:
    // where the child's `Command::new("dsh")` finds nothing, selection
    // refuses and so does resolution, home or no home; where a host
    // keeps a dsh there, both select that very file.
    #[cfg(unix)]
    {
        const CASE: &str = "BROKKR_COMPOSITE_SEAMS_CONTROL";
        if std::env::var_os(CASE).is_some() {
            assert_eq!(
                crate::transcript::dsh_home(),
                Some(PathBuf::from("/tmp/.dsh")),
                "the home is present in this environment"
            );
            let native = std::process::Command::new("dsh")
                .arg("--version")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            match native {
                Err(error) => {
                    assert_eq!(error.kind(), std::io::ErrorKind::NotFound, "{error}");
                    let unselected = DshSeams::selected().unwrap_err();
                    assert_eq!(unselected.declared, "dsh");
                    // The search ended at the last entry's candidate,
                    // `/bin/dsh`, with the child's own NotFound.
                    let enoent = fs::metadata("/bin/dsh").unwrap_err();
                    let cause = format!(
                        "the DSH layout is unreadable: 'dsh' is not on PATH (the search ended at \
                         /bin/dsh: {enoent})"
                    );
                    assert_eq!(unselected.cause.to_string(), cause);
                    assert_eq!(
                        refused(DshSeams::resolve()),
                        cause,
                        "a present home does not make a resolution"
                    );
                }
                Ok(_) => {
                    let selection = DshSeams::selected().unwrap();
                    assert!(
                        Path::new("/usr/bin/dsh").exists() || Path::new("/bin/dsh").exists(),
                        "the native child found dsh on the child PATH: {}",
                        selection.executable
                    );
                    assert_eq!(
                        DshSeams::resolve().unwrap().executable,
                        selection.executable
                    );
                }
            }
            return;
        }
        let mut child = std::process::Command::new(std::env::current_exe().unwrap());
        child
            .args([
                "adapters::composite::tests::dsh_seams_resolve_reads_the_home_and_refuses_a_missing_one",
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(CASE, "1")
            .env("HOME", "/tmp")
            .env("PATH", "/usr/bin:/bin")
            .env_remove("BROKKR_DSH_BIN")
            .env_remove("FORGE_DSH_BIN")
            .env_remove("DSH_HOME");
        let output = spawn_retrying_etxtbsy(&mut child);
        let said = String::from_utf8_lossy(&output.stdout).into_owned()
            + &String::from_utf8_lossy(&output.stderr);
        assert!(
            said.contains("1 passed") || said.contains("1 failed"),
            "the child ran the control rather than filtering it away: {said}"
        );
        assert!(output.status.success(), "{said}");
    }

    assert_eq!(
        refused(DshSeams::resolve_with(
            "dsh".to_string(),
            None,
            Vec::new(),
            None
        )),
        "the DSH layout is unreadable: no dsh home: set DSH_HOME or HOME"
    );
    let dir = tempfile::tempdir().unwrap();
    let seams = DshSeams::resolve_with(
        "dsh".to_string(),
        None,
        b"#!/usr/bin/env node\n".to_vec(),
        Some(dir.path().to_path_buf()),
    )
    .unwrap();
    assert_eq!(seams.executable, "dsh");
    assert_eq!(seams.home, dir.path());
    assert_eq!(seams.node, None);
    assert_eq!(seams.head, b"#!/usr/bin/env node\n");

    // Each arm of the selection, DRIVEN rather than observed: whichever
    // `dsh` this host has installed decides which arm the real call above
    // takes, and a gate that demands every line cannot rest on that.
    let home = Some(dir.path().to_path_buf());
    let resolved = dir.path().join("resolved-dsh");
    let retained = DshNode {
        path: dir.path().join("retained-node"),
        invocation: DshInvocation {
            program: dir.path().join("bin").join("node"),
            argv0: "node".into(),
        },
    };
    // The invocation is its own fact: the candidate the search found
    // under the searched name, beside the canonical file (R2).
    let invoked = DshInvocation {
        program: dir.path().join("bin").join("dsh"),
        argv0: "dsh".into(),
    };
    let selected = || {
        Ok(Selected {
            path: resolved.clone(),
            invocation: invoked.clone(),
            node: Some(retained.clone()),
            head: b"#!/usr/bin/env node\n".to_vec(),
        })
    };
    let selection =
        DshSeams::located_from("dsh".to_string(), |_| selected(), home.clone()).unwrap();
    assert_eq!(Path::new(&selection.executable), resolved);
    assert_eq!(selection.invocation, invoked);
    // Admitted, the same selection over a home with no profile keeps the
    // invocation beside the cause the home was not located by: the
    // executable's availability does not depend on the home.
    let unlocated = DshSeams::selected_from("dsh".to_string(), |_| selected(), home.clone())
        .unwrap()
        .admission
        .unwrap_err();
    match &unlocated {
        DshUnprepared::Unlocated { invocation, .. } => assert_eq!(invocation, &invoked),
        DshUnprepared::Refused { cause } => panic!("nothing was located to refuse: {cause}"),
    }
    assert!(
        unlocated.cause().to_string().contains("profiles/headless"),
        "{}",
        unlocated.cause()
    );
    // And with no home at all: the same independent fact, by the home's
    // own cause, with the invocation still there for the version.
    assert_eq!(
        DshSeams::selected_from("dsh".to_string(), |_| selected(), None)
            .unwrap()
            .admission,
        Err(DshUnprepared::Unlocated {
            invocation: invoked.clone(),
            cause: CompositeError::Config("no dsh home: set DSH_HOME or HOME".into()),
        })
    );
    let seams = selection.seams.unwrap();
    assert_eq!(seams.executable, selection.executable);
    // The Node selection made beside the executable is the seams' own:
    // what the composite will observe, not a spelling to look up again.
    assert_eq!(seams.node, Some(retained));
    // And so are the bytes the selection inspected: the observation's
    // first-line check reads these and never the file again.
    assert_eq!(seams.head, b"#!/usr/bin/env node\n");
    // A name that resolves to nothing is NOT selected. The inherited
    // arm kept the declared spelling as the executable, and doctor
    // probed that spelling: under an absent `PATH` it executed a `dsh`
    // in its working directory (security hold 2026-09-20, S1).
    let unselected = DshSeams::selected_from(
        "dsh".to_string(),
        |name| Err(CompositeError::Config(format!("'{name}': PATH is absent"))),
        home.clone(),
    )
    .unwrap_err();
    assert_eq!(unselected.declared, "dsh");
    assert_eq!(
        unselected.cause.to_string(),
        "the DSH layout is unreadable: 'dsh': PATH is absent"
    );
    // The home is not consulted for a selection that failed: the
    // producer has no executable to observe.
    let unselected = DshSeams::selected_from(
        "dsh".to_string(),
        |name| Err(CompositeError::Config(format!("'{name}' is not on PATH"))),
        None,
    )
    .unwrap_err();
    assert_eq!(
        unselected.cause.to_string(),
        "the DSH layout is unreadable: 'dsh' is not on PATH"
    );
    // And a resolved path this platform cannot spell as UTF-8 is refused
    // by cause, neither rendered lossily nor replaced by the declaration.
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let raw = PathBuf::from(std::ffi::OsStr::from_bytes(b"/tmp/dsh\xff"));
        let unselected = DshSeams::selected_from(
            "dsh".to_string(),
            |_| {
                Ok(Selected {
                    path: raw.clone(),
                    invocation: DshInvocation::of(raw.clone()),
                    node: None,
                    head: Vec::new(),
                })
            },
            home,
        )
        .unwrap_err();
        assert_eq!(unselected.declared, "dsh");
        assert_eq!(
            unselected.cause.to_string(),
            format!(
                "the DSH layout is unreadable: {}: the selected path is not UTF-8",
                raw.display()
            )
        );
    }
    // `resolve` is the location's seams, or the selection's cause.
    assert_eq!(
        DshSeams::resolve().map_err(|error| error.to_string()),
        DshSeams::located()
            .map_err(|unselected| unselected.cause.to_string())
            .and_then(|located| located.seams.map_err(|error| error.to_string()))
    );
    // Both arms, injected: a failed selection reaching the planner is
    // the lookup's own cause — here the absent `PATH` — and never a
    // spelling to look up again; a selection is its seams, whichever
    // way the home went.
    assert_eq!(
        refused(DshSeams::resolved(Err(DshUnselected {
            declared: "dsh".to_string(),
            cause: CompositeError::Config("'dsh': PATH is absent".into()),
        }))),
        "the DSH layout is unreadable: 'dsh': PATH is absent"
    );
    let seams = DshSeams {
        executable: "/opt/dsh/lib/bin.js".to_string(),
        home: PathBuf::from("/opt/home"),
        node: Some(DshNode {
            path: PathBuf::from("/opt/node/bin/node"),
            invocation: DshInvocation::of("/opt/node/bin/node"),
        }),
        head: b"#!/usr/bin/env node\n".to_vec(),
    };
    assert_eq!(
        DshSeams::resolved(Ok(Located {
            executable: seams.executable.clone(),
            invocation: DshInvocation::of(&seams.executable),
            seams: Ok(seams.clone()),
        }))
        .unwrap(),
        seams
    );
    assert_eq!(
        refused(DshSeams::resolved(Ok(Located {
            executable: seams.executable.clone(),
            invocation: DshInvocation::of(&seams.executable),
            seams: Err(CompositeError::Config(
                "no dsh home: set DSH_HOME or HOME".into()
            )),
        }))),
        "the DSH layout is unreadable: no dsh home: set DSH_HOME or HOME"
    );
}

/// R2 (review of run `124cca78`): a selection is the canonical file AND
/// the invocation native runs it by, and neither stands in for the
/// other.
///
/// `dsh -> /usr/bin/env`, searched and as an absolute alias, is the
/// platform's env utility run under the name `dsh`: natively uutils exits
/// 1 on the name mismatch and prints nothing, while executing the
/// canonical target under its own name reported env's version as DSH's.
/// Both refuse at selection, by the selected invocation and the env
/// dispatch, so there is no probe target. Direct `/usr/bin/env` is the
/// availability control; an admitted path-sensitive launcher proves the
/// invocation, and not the canonical target, is what runs.
#[cfg(unix)]
#[test]
fn the_selected_invocation_is_not_replaced_by_its_canonical_target() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a");
    let lib = dir.path().join("lib");
    fs::create_dir_all(&a).unwrap();
    fs::create_dir_all(&lib).unwrap();
    let path = |dir: &Path| Some(OsString::from(dir.display().to_string()));
    let native = |program: &std::ffi::OsStr, search: &Path| {
        std::process::Command::new(program)
            .arg("--version")
            .env("PATH", search)
            .output()
    };

    let alias = a.join("dsh");
    std::os::unix::fs::symlink(ENV_REFERENCE, &alias).unwrap();
    let unestablished = format!(
        "the DSH layout is unreadable: {}: the selected invocation is the platform's env utility \
         invoked under the name 'dsh', a dispatch this resolver does not establish without \
         executing it",
        alias.display()
    );
    for (form, declared) in [
        ("searched", "dsh".to_string()),
        ("absolute", alias.display().to_string()),
    ] {
        // The native outcome is the HOST's and is recorded, never
        // counted: uutils refuses the name, GNU's env runs under any.
        let ran = native(declared.as_ref(), &a).unwrap();
        eprintln!(
            "native {form} alias: {:?}, {} stdout bytes",
            ran.status,
            ran.stdout.len()
        );
        assert_eq!(
            refused(select_in(&declared, path(&a))),
            unestablished,
            "{form}"
        );
        // And through the seam a caller holds: no selection, so no
        // invocation and nothing to probe.
        let unselected = DshSeams::selected_from(
            declared.clone(),
            |name| select_in(name, path(&a)),
            Some(dir.path().to_path_buf()),
        )
        .unwrap_err();
        assert_eq!(unselected.declared, declared);
        assert_eq!(unselected.cause.to_string(), unestablished, "{form}");
    }

    // Direct env: established under its own name, selected as the file
    // it canonically is and invoked as it was spelled. Its version is
    // the availability control, and the invocation answers as native —
    // whatever native answers. GNU and uutils env print a version; Apple's
    // env has no `--version`, prints its usage and exits 1, and that
    // failed probe is the native result the invocation has to preserve
    // (review of run `e291e076`). The status, stdout and stderr are
    // compared, never assumed.
    let direct = select_in(ENV_REFERENCE, path(&a)).unwrap();
    assert_eq!(
        direct.path,
        Path::new(ENV_REFERENCE).canonicalize().unwrap()
    );
    assert_eq!(direct.invocation, DshInvocation::of(ENV_REFERENCE));
    assert_eq!(direct.node, None);
    let control = native(ENV_REFERENCE.as_ref(), &a).unwrap();
    let invoked = direct
        .invocation
        .command()
        .arg("--version")
        .env("PATH", &a)
        .output()
        .unwrap();
    assert_eq!(invoked.status, control.status, "{control:?}");
    assert_eq!(invoked.stdout, control.stdout);
    assert_eq!(invoked.stderr, control.stderr);

    // An admitted launcher that reads the path it was run by, behind a
    // symlink: identity is the target, the invocation is the candidate
    // the search found under the searched name, and running it prints
    // what native prints — never the canonical target's path.
    let launcher = stage_executable(&lib, "launcher.sh", b"#!/bin/sh\necho \"$0\"\n");
    fs::remove_file(&alias).unwrap();
    std::os::unix::fs::symlink(&launcher, &alias).unwrap();
    let selected = select_in("dsh", path(&a)).unwrap();
    assert_eq!(selected.path, launcher.canonicalize().unwrap());
    assert_eq!(
        selected.invocation,
        DshInvocation {
            program: alias.clone(),
            argv0: "dsh".into(),
        }
    );
    let natively = native("dsh".as_ref(), &a).unwrap();
    let invoked = selected
        .invocation
        .command()
        .arg("--version")
        .output()
        .unwrap();
    assert!(natively.status.success(), "{natively:?}");
    assert_eq!(
        String::from_utf8_lossy(&invoked.stdout),
        format!("{}\n", alias.display()),
        "the selected candidate runs, not its canonical target"
    );
    assert_eq!(invoked.stdout, natively.stdout);
    // An explicit path keeps its own spelling on both halves.
    let explicit = select_in(&alias.display().to_string(), path(&a)).unwrap();
    assert_eq!(explicit.invocation, DshInvocation::of(&alias));
    assert_eq!(explicit.path, selected.path);

    // A platform env that cannot be inspected establishes nothing about
    // the selected file either, and says so — asked of a native image,
    // which has no interpreter to be asked first.
    fs::remove_file(&alias).unwrap();
    fs::copy(std::env::current_exe().unwrap(), a.join(".dsh.staging")).unwrap();
    fs::rename(a.join(".dsh.staging"), &alias).unwrap();
    let absent = dir.path().join("no-env");
    let enoent = fs::metadata(&absent).unwrap_err();
    let search = Search {
        entries: OsString::from(a.display().to_string()),
        default: false,
        library: LIBRARY,
        operation: Operation::Exec,
        env_reference: absent.clone(),
    };
    assert_eq!(
        refused(lookup_in("dsh", &search, &mut Vec::new())),
        format!(
            "the DSH layout is unreadable: {}: the selected invocation cannot be compared with \
             the platform's env '{}', which cannot be inspected: {enoent}",
            alias.display(),
            absent.display()
        )
    );

    // The carried values print, copy and compare as values.
    let node = node_at(&launcher);
    assert_eq!(node.clone(), node);
    assert!(format!("{node:?}").contains("launcher.sh"));
}

/// R1 and R3's wiring (review of run `124cca78`): the home's profile and
/// pnpm lock are ADMITTED before anything is executed, and composed as
/// retained.
///
/// The lock was parsed last — after doctor's DSH probe and the producer's
/// Node probe — so a lock YAML refuses was refused only once both had
/// run. Admission is now the selection's, and the producer's own first
/// step; a located lock that fails it is a refusal with nothing spawned,
/// a home that was never located keeps the invocation for the version
/// alone, and a probe that rewrites the admitted files changes nothing
/// the composite reads.
#[test]
fn the_pnpm_lock_is_admitted_before_any_probe_and_composed_as_retained() {
    let install = Synthetic::new();
    let profile = install.profile();
    let lock = profile.join("pnpm-lock.yaml");
    let admitted_lock = fs::read(&lock).unwrap();
    let control = install.composite();
    let invoked = DshInvocation {
        program: install.dir.path().join("bin").join("dsh"),
        argv0: "dsh".into(),
    };

    let prepared = DshPrepared::admit(invoked.clone(), install.seams.clone()).unwrap();
    assert_eq!(prepared.invocation(), &invoked);
    assert_eq!(prepared.seams(), &install.seams);
    assert_eq!(prepared.clone(), prepared);
    assert!(format!("{prepared:?}").contains("debug 2.6.9 sha512-DEBUG"));

    // The probe REWRITES the lock and the profile manifest it was
    // admitted from. The composite is the admitted one: a reopen would
    // read `other@1.0.0` and a `live` reload and differ.
    let probes = std::cell::Cell::new(0);
    let observed = dsh_composite_observing(prepared.seams(), &prepared.admitted, || {
        probes.set(probes.get() + 1);
        write(
            &profile,
            "pnpm-lock.yaml",
            b"lockfileVersion: '9.0'\n\npackages:\n\n  other@1.0.0:\n    resolution: {integrity: sha512-OTHER}\n",
        );
        write(
            &profile,
            "package.json",
            br#"{"dsh":{"profile":{"bundles":["@deepseek-ai/dsh-base","dsh-plugin-cli-session"],"patchReload":"live"}}}"#,
        );
        Ok(install.node())
    })
    .unwrap();
    assert_eq!(probes.get(), 1);
    assert_eq!(
        observed.canonical, control.canonical,
        "the retained inputs are the ones composed"
    );
    // The rewritten files ARE another identity to a fresh observation,
    // so the equality above is retention and not indifference.
    assert_ne!(install.composite().canonical, control.canonical);
    write(
        &profile,
        "package.json",
        br#"{"dsh":{"profile":{"bundles":["@deepseek-ai/dsh-base","dsh-plugin-cli-session"],"patchReload":"startup"}}}"#,
    );

    // A LOCATED lock that fails admission: refused by its cause, with no
    // invocation to probe, and through the producer with no Node probe.
    for (body, reason) in [
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-DEBUG}\n    engines: {node:  *missing}\n".to_string(),
            "pnpm lock is unreadable: a package child 'engines' carrying the malformed flow \
             member 'node:  *missing', whose value opens with the YAML indicator '*'",
        ),
        (
            format!(
                "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {{integrity: sha512-DEBUG}}\n    peerDependencies:\n      {}: a\n",
                "k".repeat(1025)
            ),
            "pnpm lock is unreadable: a line under the package child 'peerDependencies' \
             carrying an implicit key past YAML's implicit-key lookahead limit of 1,024 \
             characters",
        ),
        (
            format!(
                "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9{}:\n    resolution: {{integrity: sha512-DEBUG}}\n",
                " ".repeat(1014)
            ),
            "pnpm lock is unreadable: a package key that is an implicit key past YAML's \
             implicit-key lookahead limit of 1,024 characters",
        ),
    ] {
        fs::write(&lock, body).unwrap();
        let refusal = DshPrepared::admit(invoked.clone(), install.seams.clone()).unwrap_err();
        assert_eq!(
            refusal,
            DshUnprepared::Refused {
                cause: CompositeError::PnpmLock(
                    reason
                        .strip_prefix("pnpm lock is unreadable: ")
                        .unwrap()
                        .to_string()
                ),
            }
        );
        assert_eq!(refusal.cause().to_string(), reason);
        assert_eq!(refusal.clone(), refusal);
        assert!(format!("{refusal:?}").starts_with("Refused"));
        let probes = std::cell::Cell::new(0);
        assert_eq!(
            refused(dsh_composite_resolving(&install.seams, || {
                probes.set(probes.get() + 1);
                Ok(install.node())
            })),
            reason
        );
        assert_eq!(probes.get(), 0, "no Node probe for a lock that fails admission");
        // The selection a caller holds says the same, and holds nothing
        // to probe.
        let selection = DshSeams::selected_from(
            "dsh".to_string(),
            |_| {
                Ok(Selected {
                    path: PathBuf::from(&install.seams.executable),
                    invocation: invoked.clone(),
                    node: None,
                    head: install.seams.head.clone(),
                })
            },
            Some(install.seams.home.clone()),
        )
        .unwrap();
        assert_eq!(selection.admission, Err(refusal));
    }
    // A dangling lock is LOCATED: the name is there and cannot be read.
    // The link is staged where the platform has one to stage; the
    // admission and retention facts around it hold on every host.
    #[cfg(unix)]
    {
        fs::remove_file(&lock).unwrap();
        std::os::unix::fs::symlink(profile.join("no-such-lock"), &lock).unwrap();
        let dangling = fs::metadata(&lock).unwrap_err();
        assert_eq!(
            DshPrepared::admit(invoked.clone(), install.seams.clone()).unwrap_err(),
            DshUnprepared::Refused {
                cause: CompositeError::PnpmLock(format!("{}: {dangling}", lock.display())),
            }
        );
    }

    // A lock, or a profile, that was never FOUND is the independent
    // fact: the invocation stays beside the cause, for the version alone.
    fs::remove_file(&lock).unwrap();
    let enoent = fs::metadata(&lock).unwrap_err();
    let unlocated = DshPrepared::admit(invoked.clone(), install.seams.clone()).unwrap_err();
    assert_eq!(
        unlocated,
        DshUnprepared::Unlocated {
            invocation: invoked.clone(),
            cause: CompositeError::PnpmLock(format!("{}: {enoent}", lock.display())),
        }
    );
    assert_eq!(unlocated.clone(), unlocated);
    assert!(format!("{unlocated:?}").starts_with("Unlocated"));
    fs::remove_file(profile.join("package.json")).unwrap();
    match DshPrepared::admit(invoked.clone(), install.seams.clone()).unwrap_err() {
        DshUnprepared::Unlocated { invocation, cause } => {
            assert_eq!(invocation, invoked);
            assert!(cause.to_string().contains("package.json"), "{cause}");
        }
        DshUnprepared::Refused { cause } => panic!("no lock was located to refuse: {cause}"),
    }

    // Restored, the same home admits and composes as the control.
    write(
        &profile,
        "package.json",
        br#"{"dsh":{"profile":{"bundles":["@deepseek-ai/dsh-base","dsh-plugin-cli-session"],"patchReload":"startup"}}}"#,
    );
    fs::write(&lock, admitted_lock).unwrap();
    let prepared = DshPrepared::admit(invoked, install.seams.clone()).unwrap();
    let observed =
        dsh_composite_observing(prepared.seams(), &prepared.admitted, || Ok(install.node()));
    assert_eq!(observed.unwrap().canonical, control.canonical);
}

/// The producer receives the SELECTED executable, a path the selection
/// already resolved, and never searches for a bare spelling a second
/// time: a second lookup could choose a different file than the one
/// doctor probed (design D10).
#[test]
fn the_producer_refuses_a_bare_executable_spelling() {
    assert_eq!(
        refused(selected_executable("dsh")),
        "the DSH layout is unreadable: 'dsh' is not a path: the selected executable is \
         resolved once, at selection"
    );
    let install = Synthetic::new();
    assert_eq!(
        selected_executable(&install.seams.executable).unwrap(),
        Path::new(&install.seams.executable).canonicalize().unwrap()
    );
    // A path that is not there, one that is a directory, and — on Unix —
    // one this process may not execute: each refused by its own reason,
    // because the producer consumes an ESTABLISHED selection and says
    // when the file behind it is no longer that.
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing").join("dsh");
    let enoent = fs::metadata(&missing).unwrap_err();
    assert_eq!(
        refused(selected_executable(missing.to_str().unwrap())),
        format!(
            "the DSH layout is unreadable: {}: {enoent}",
            missing.display()
        )
    );
    assert_eq!(
        refused(selected_executable(dir.path().to_str().unwrap())),
        format!(
            "the DSH layout is unreadable: {}: is not a regular file",
            dir.path().display()
        )
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let plain = dir.path().join("plain.js");
        fs::write(&plain, b"#!/usr/bin/env node\n").unwrap();
        fs::set_permissions(&plain, fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(
            refused(selected_executable(plain.to_str().unwrap())),
            format!(
                "the DSH layout is unreadable: {}: is not executable by this process",
                plain.display()
            )
        );
        // A backslash is an ordinary filename byte on Unix: `missing\dsh`
        // is a bare NAME, not a path, and the producer refuses it as one
        // (security hold 2026-09-20, S1b).
        assert_eq!(
            refused(selected_executable("missing\\dsh")),
            "the DSH layout is unreadable: 'missing\\dsh' is not a path: the selected \
             executable is resolved once, at selection"
        );
    }
    assert_eq!(
        refused(resolve_core("dsh")),
        "the DSH layout is unreadable: 'dsh' is not a path: the selected executable is \
         resolved once, at selection"
    );
}

/// An ABSENT `PATH` is the C library's DEFAULT search, never an empty
/// search and never an unconditional refusal. The inherited
/// `unwrap_or_default()` read `None` as one empty entry, and an empty
/// entry is the working directory: with no `PATH` and an executable
/// `dsh` in cwd, the lookup selected that file where `Command::new("dsh")`
/// finds nothing (security hold 2026-09-20, S1; controller reproduction).
/// The first repair refused every absent-PATH lookup, which the
/// commission withdrew: `execvp` searches `confstr(_CS_PATH)` in that
/// case, so a name that sits there — `sh` — runs, and the resolver
/// selects the same file. The real doctor regression, sentinel and all,
/// is `crates/brokkr-cli/tests/doctor_dsh_selection.rs`; this is the
/// resolver's own arm, and the explicit-path arm beside it.
#[cfg(unix)]
#[test]
fn an_absent_path_is_a_named_refusal_and_never_the_working_directory() {
    let last_dir = std::env::split_paths(&default_search_path().unwrap())
        .last()
        .unwrap();
    let default = default_search_path().unwrap();
    let default = default.to_string_lossy().into_owned();
    // The names the default search does not hold refuse by naming what
    // WAS searched, and where the search ended: a `node` looked up for
    // its version meets the same rule as `dsh`. Whether a host keeps
    // either on its default path is asked of the host rather than
    // assumed.
    for name in ["dsh", "node"] {
        let last = last_dir.join(name);
        let native = std::process::Command::new(name)
            .arg("--version")
            .env_remove("PATH")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        match native {
            Err(error) => {
                assert_eq!(
                    error.kind(),
                    std::io::ErrorKind::NotFound,
                    "{name}: {error}"
                );
                assert_eq!(
                    refused(resolve_executable_in(name, None)),
                    format!(
                        "the DSH layout is unreadable: '{name}' is not on the default search \
                         path {default} (PATH is absent) (the search ended at {}: {})",
                        last.display(),
                        fs::metadata(&last).unwrap_err()
                    )
                );
            }
            Ok(_) => {
                let selected = resolve_executable_in(name, None).unwrap();
                assert!(
                    std::env::split_paths(&default_search_path().unwrap())
                        .any(|dir| selected.starts_with(dir.canonicalize().unwrap_or(dir))),
                    "{name} selected from the default search: {}",
                    selected.display()
                );
                // Node reports the runtime it is: the resolver's
                // selection is that very file.
                if name == "node" {
                    let printed = std::process::Command::new("node")
                        .args(["-p", "process.execPath"])
                        .env_remove("PATH")
                        .output()
                        .unwrap();
                    let ran = String::from_utf8_lossy(&printed.stdout).trim().to_string();
                    assert_eq!(
                        selected,
                        Path::new(&ran).canonicalize().unwrap(),
                        "the resolver selected the runtime native default search ran"
                    );
                }
            }
        }
    }
    // The positive: `sh` sits on every Unix default search path, a
    // native child with no `PATH` runs it, and the resolver selects the
    // very file that child ran (identified through /proc on Linux).
    let native = std::process::Command::new("sh")
        .args(["-c", "readlink /proc/$$/exe 2>/dev/null || echo unknown"])
        .env_remove("PATH")
        .output()
        .expect("a native child finds sh with no PATH");
    assert!(native.status.success());
    let ran = String::from_utf8_lossy(&native.stdout).trim().to_string();
    let selected = resolve_executable_in("sh", None).unwrap();
    assert!(
        std::env::split_paths(&default_search_path().unwrap())
            .any(|dir| selected.starts_with(dir.canonicalize().unwrap_or(dir))),
        "sh selected from the default search: {}",
        selected.display()
    );
    if cfg!(target_os = "linux") {
        assert_eq!(
            selected,
            PathBuf::from(&ran),
            "the resolver selected the file the native default search ran"
        );
    }
    // An explicit path needs no `PATH` to be selected: an override is a
    // file, not a search.
    let dir = tempfile::tempdir().unwrap();
    let tool = stage_executable(dir.path(), "dsh", b"#!/bin/sh\ntrue\n");
    assert_eq!(
        resolve_executable_in(tool.to_str().unwrap(), None).unwrap(),
        tool.canonicalize().unwrap()
    );
    // A PRESENT but empty `PATH` is one empty entry, which is cwd — a
    // different answer from absence, asserted in the child case of
    // `an_empty_path_entry_is_the_working_directory_and_is_refused`.
    // With no cwd candidate the child gets NotFound; the resolver still
    // refuses at the cwd entry, never past it, and never falls back to
    // the absent-PATH default search (fourth hold).
    assert_eq!(
        refused(resolve_executable_in(
            "definitely-not-in-cwd",
            Some(OsString::new())
        )),
        "the DSH layout is unreadable: definitely-not-in-cwd: the platform's search would fall \
         into the working directory: PATH entry 0 is empty"
    );
}

/// Where a bare or direct spelling places its candidate in a layout,
/// shared by the Unix matrix (`native_matrix.rs`) and the Windows one.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Slot {
    /// The file the spelling itself denotes: `cwd/<name>` for a bare
    /// name, the spelled file for a direct one.
    Target,
    /// A directory on `PATH`.
    PathDir,
    /// `A` of `A:B`; for a direct spelling this is the spelled file.
    A,
    /// `B` of `A:B`.
    B,
    /// A directory on `PATH` holding no candidate, or a later one.
    Other,
    /// A `PATH` component of exactly this many ASCII `x` bytes, which
    /// no layout places a file in: the commissioned lengths 255, 256,
    /// 300, 4095, 4096 and 5000, each its own cell (run `09ec8d81`, R1).
    Long(usize),
    /// A regular FILE spelled as a `PATH` component: the candidate under
    /// it answers ENOTDIR.
    File,
    /// A component that does not exist: the candidate under it answers
    /// ENOENT.
    Nowhere,
    /// An EMPTY `PATH` entry, spelled as nothing between two separators
    /// (or as the whole variable): the working directory to the child.
    Empty,
    /// `A`'s own directory spelling padded with trailing `/` bytes to
    /// exactly this many bytes, so the candidate native constructs under
    /// it — one more `/` and the name — lands on a byte boundary the
    /// kernel measures (fourth hold, R2).
    PaddedA(usize),
}

/// The overlong `PATH` component's spelling, as the commission first
/// reproduced it; the Windows matrix keeps it as its one long cell.
#[cfg(windows)]
const OVERLONG_COMPONENT: usize = 5000;
/// R5 (run `09ec8d81`). With `PATH` absent and DSH safely selected by an
/// explicit path, the `node` the selection RETAINS is exactly the
/// runtime a native `Command::new("node")` runs from the platform's
/// default search, and the composite consumes THAT field: the `node`
/// line is what the retained file answers `--version`, and a distinct
/// runtime retained in its place moves the composite. The private field
/// is read here through the module's own seams; the built-doctor half is
/// `absent_path_default_search_matches_native_dsh_and_node`. A host with
/// no `node` on its default search supplies only the refusal, and the
/// positive with its two removal controls is recorded PENDING, never a
/// passing skip. Restoring unconditional absent-PATH refusal fails the
/// positive while the native child still succeeds; retaining a distinct
/// wrong Node fails the retained identity (recorded in the delivery
/// account).
#[cfg(unix)]
#[test]
fn absent_path_node_identity_is_retained_by_the_composite() {
    use std::process::Command;

    let install = Synthetic::new();
    let executable = install.seams.executable.clone();
    let default = default_search_path().unwrap();
    let default = default.to_string_lossy().into_owned();
    let native = Command::new("node")
        .args(["-p", "process.execPath"])
        .env_remove("PATH")
        .output();
    match native {
        Err(error) => {
            assert_eq!(error.kind(), std::io::ErrorKind::NotFound, "{error}");
            let last = std::env::split_paths(&default_search_path().unwrap())
                .last()
                .unwrap()
                .join("node");
            assert_eq!(
                refused(select_in(&executable, None)),
                format!(
                    "the DSH layout is unreadable: {executable}: its #! interpreter \
                     '/usr/bin/env' selects no 'node': the DSH layout is unreadable: 'node' is \
                     not on the default search path {default} (PATH is absent) (the search \
                     ended at {}: {})",
                    last.display(),
                    fs::metadata(&last).unwrap_err()
                ),
                "the selection refuses before any probe where native lookup finds no node"
            );
            eprintln!(
                "PENDING: no node on this host's default search path {default}; the absent-PATH \
                 retained-Node positive and its two removal controls were not established here"
            );
        }
        Ok(output) => {
            assert!(
                output.status.success(),
                "the native default-search node exited {:?}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            );
            let ran = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let ran = Path::new(&ran).canonicalize().unwrap();
            let selected = select_in(&executable, None).unwrap();
            assert_eq!(
                node_file(&selected.node),
                Some(ran.clone()),
                "the retained node is the runtime the native default search ran"
            );
            // The composite consumes the retained field and nothing
            // else: its `node` line is what THAT file answers.
            let seams = DshSeams {
                executable: executable.clone(),
                home: install.seams.home.clone(),
                node: selected.node.clone(),
                head: selected.head.clone(),
            };
            let composite = dsh_composite(&seams).unwrap();
            let version = Command::new(&ran).arg("--version").output().unwrap();
            assert_eq!(
                composite.node,
                String::from_utf8_lossy(&version.stdout).trim(),
                "the composite's node line is the retained runtime's own version"
            );
            // A distinct runtime retained in its place is what the
            // composite then observes: the field is consumed, not
            // re-derived by a second search.
            let wrong = stage_executable(
                install.dir.path(),
                "wrong-node",
                b"#!/bin/sh\necho v0.0.0-wrong\n",
            );
            let retained_wrong = DshSeams {
                node: Some(node_at(wrong)),
                ..seams.clone()
            };
            let observed = dsh_composite(&retained_wrong).unwrap();
            assert_eq!(observed.node, "v0.0.0-wrong");
            assert_ne!(observed.canonical, composite.canonical);
            eprintln!(
                "absent-PATH retained-Node positive established: {} ({})",
                ran.display(),
                composite.node
            );
        }
    }
}

/// The Windows sentinel arm: run as a hard-linked copy of this test
/// binary under another name, with the marker variable set, it prints
/// the image it runs as, so a matrix cell can tell WHICH copy the
/// platform ran. Run as an ordinary test it does nothing.
#[cfg(windows)]
#[test]
fn windows_matrix_sentinel_reports_its_own_image() {
    if std::env::var_os(WINDOWS_SENTINEL).is_some() {
        println!(
            "SENTINEL_EXE:{}",
            std::env::current_exe().unwrap().display()
        );
    }
}

#[cfg(windows)]
const WINDOWS_SENTINEL: &str = "BROKKR_COMPOSITE_WINDOWS_SENTINEL";

/// The layouts the Windows matrix crosses: the same cwd, PATH-directory,
/// absent, present-empty and empty-entry layouts as the Unix table, and
/// the three `A;B` cells Windows lookup has — a non-image at A, a
/// directory at A and a working image at A — in place of the Unix loader
/// obstructions, because Windows lookup stops at the first entry that
/// EXISTS and never walks past one it cannot run.
#[cfg(windows)]
const WINDOWS_MATRIX_LAYOUTS: usize = 11;

/// The differential matrix on Windows (design D10, proposal AO; review
/// 2026-09-20, R8): every commissioned spelling crossed with every
/// layout, the resolver compared with a real `Command::new(name)` child
/// under IDENTICAL cwd and environment, and every cell equality — the
/// same canonical file, or a refusal exactly where the child failed. The
/// candidates are hard-linked copies of this test binary answering as
/// the sentinel arm above, so a cell identifies the file the platform
/// ran. The literal `C:\Tools\dsh.exe` row is asserted as the absent
/// path it is on a runner and recorded PENDING where an operator keeps
/// an installation there, which this test never creates or runs.
///
/// Windows' rule, as std applies it: a spelling with a separator is a
/// path, tried with `.exe` appended before the literal unless it already
/// ends in `.exe`; a file name is searched — the child's `PATH` when its
/// environment changed (empty entries skipped), the application
/// directory, the system and Windows directories, the parent's `PATH` —
/// with `.exe` appended when the name has no extension; the working
/// directory is never searched; the first entry that exists is the
/// selection, run or not.
#[cfg(windows)]
#[test]
fn native_executable_resolution_matches_command_matrix_on_windows() {
    const CASE: &str = "BROKKR_COMPOSITE_NATIVE_MATRIX_WINDOWS";
    if std::env::var_os(CASE).is_some() {
        windows_matrix_child();
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let cwd = root.path().join("cwd");
    fs::create_dir_all(&cwd).unwrap();
    let mut child = std::process::Command::new(std::env::current_exe().unwrap());
    child
        .args([
            "adapters::composite::tests::native_executable_resolution_matches_command_matrix_on_windows",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ])
        .current_dir(&cwd)
        .env(CASE, "1");
    let output = child.output().expect("the child test binary runs");
    let said = String::from_utf8_lossy(&output.stdout).into_owned()
        + &String::from_utf8_lossy(&output.stderr);
    assert!(
        said.contains("1 passed") || said.contains("1 failed"),
        "the child ran the matrix rather than filtering it away: {said}"
    );
    assert!(output.status.success(), "{said}");
    assert!(
        said.contains(&format!(
            "matrix: 8 names x {WINDOWS_MATRIX_LAYOUTS} layouts = {} cells",
            8 * WINDOWS_MATRIX_LAYOUTS
        )),
        "{said}"
    );
}

/// What a Windows candidate is made of.
#[cfg(windows)]
enum WindowsBody {
    /// A hard-linked copy of this test binary: a working image.
    Sentinel,
    /// A text file under the candidate's name: exists, and is no image.
    Text,
    /// A directory under the candidate's name: exists, and is no file.
    Directory,
}

#[cfg(windows)]
struct WindowsLayout {
    name: &'static str,
    files: Vec<(Slot, WindowsBody)>,
    path: Option<Vec<Slot>>,
    empty_at: Option<usize>,
}

#[cfg(windows)]
fn windows_matrix_child() {
    use std::process::{Command, Stdio};

    let cwd = std::env::current_dir().unwrap();
    let root = cwd.parent().unwrap().to_path_buf();
    let exe = std::env::current_exe().unwrap();
    fs::create_dir_all(root.join("abs")).unwrap();
    let abs = root.join("abs").join("dsh").display().to_string();
    const OPERATOR_PATH: &str = "C:\\Tools\\dsh.exe";
    let operator_installation = Path::new(OPERATOR_PATH).exists();
    let names: [&str; 8] = [
        "dsh",
        ".\\dsh",
        "..\\dsh",
        &abs,
        OPERATOR_PATH,
        "dsh.exe",
        "my dsh",
        "dsh\0x",
    ];
    let layouts = vec![
        WindowsLayout {
            name: "cwd-only, PATH elsewhere",
            files: vec![(Slot::Target, WindowsBody::Sentinel)],
            path: Some(vec![Slot::Other]),
            empty_at: None,
        },
        WindowsLayout {
            name: "PATH directory plus competing cwd file",
            files: vec![
                (Slot::PathDir, WindowsBody::Sentinel),
                (Slot::Target, WindowsBody::Sentinel),
            ],
            path: Some(vec![Slot::PathDir]),
            empty_at: None,
        },
        WindowsLayout {
            name: "PATH absent, cwd file",
            files: vec![(Slot::Target, WindowsBody::Sentinel)],
            path: None,
            empty_at: None,
        },
        WindowsLayout {
            name: "present-empty PATH, cwd file",
            files: vec![(Slot::Target, WindowsBody::Sentinel)],
            path: Some(vec![]),
            empty_at: Some(0),
        },
        WindowsLayout {
            name: "present-empty PATH, no candidate",
            files: vec![],
            path: Some(vec![]),
            empty_at: Some(0),
        },
        WindowsLayout {
            name: "leading empty entry",
            files: vec![
                (Slot::Target, WindowsBody::Sentinel),
                (Slot::PathDir, WindowsBody::Sentinel),
            ],
            path: Some(vec![Slot::PathDir]),
            empty_at: Some(0),
        },
        WindowsLayout {
            name: "interior empty entry",
            files: vec![
                (Slot::Target, WindowsBody::Sentinel),
                (Slot::Other, WindowsBody::Sentinel),
            ],
            path: Some(vec![Slot::PathDir, Slot::Other]),
            empty_at: Some(1),
        },
        WindowsLayout {
            name: "trailing empty entry",
            files: vec![(Slot::Target, WindowsBody::Sentinel)],
            path: Some(vec![Slot::PathDir]),
            empty_at: Some(1),
        },
        WindowsLayout {
            name: "A;B, A is not an image",
            files: vec![
                (Slot::A, WindowsBody::Text),
                (Slot::B, WindowsBody::Sentinel),
            ],
            path: Some(vec![Slot::A, Slot::B]),
            empty_at: None,
        },
        WindowsLayout {
            name: "A;B, A is a directory of the candidate's name",
            files: vec![
                (Slot::A, WindowsBody::Directory),
                (Slot::B, WindowsBody::Sentinel),
            ],
            path: Some(vec![Slot::A, Slot::B]),
            empty_at: None,
        },
        WindowsLayout {
            name: "A;B, A a working image",
            files: vec![
                (Slot::A, WindowsBody::Sentinel),
                (Slot::B, WindowsBody::Sentinel),
            ],
            path: Some(vec![Slot::A, Slot::B]),
            empty_at: None,
        },
    ];
    assert_eq!(layouts.len(), WINDOWS_MATRIX_LAYOUTS);

    // The file a spelling denotes once std's `.exe` rule is applied: a
    // bare name without an extension gains `.exe`; a path without the
    // `.exe` suffix is tried with it appended first, which is the form
    // every placed candidate takes here.
    let with_exe = |spelled: &str| -> String {
        let has_exe = spelled
            .as_bytes()
            .get(spelled.len().wrapping_sub(4)..)
            .is_some_and(|tail| tail.eq_ignore_ascii_case(b".exe"));
        match has_exe {
            true => spelled.to_string(),
            false => format!("{spelled}.exe"),
        }
    };
    let mut cell_number = 0usize;
    let mut equal = 0usize;
    let mut not_found = 0usize;
    let mut terminal = 0usize;
    let mut nul = 0usize;
    let mut pending = 0usize;
    for name in names {
        let direct = name.contains(['\\', '/']);
        let target: PathBuf = match name {
            ".\\dsh" => cwd.join("dsh.exe"),
            "..\\dsh" => root.join("dsh.exe"),
            OPERATOR_PATH => PathBuf::from(OPERATOR_PATH),
            _ if direct => PathBuf::from(with_exe(name)),
            _ => {
                let file = match name.contains('.') {
                    true => name.to_string(),
                    false => format!("{name}.exe"),
                };
                cwd.join(file)
            }
        };
        for layout in &layouts {
            cell_number += 1;
            if name == OPERATOR_PATH && operator_installation {
                eprintln!(
                    "matrix: PENDING, an operator installation sits at {OPERATOR_PATH}; the \
                     literal drive-path row is not executed on this host ({})",
                    layout.name
                );
                pending += 1;
                continue;
            }
            let cell = root.join(format!("cell-{cell_number}"));
            let dir_of = |slot: Slot| match slot {
                Slot::PathDir => cell.join("path"),
                Slot::A => cell.join("a"),
                Slot::B => cell.join("b"),
                Slot::Other => cell.join("other"),
                // Never created, and never a candidate's home: the
                // component exists in the PATH spelling alone. The
                // Windows table places no candidate under the Unix
                // matrix's length, file or nonexistent components.
                Slot::Long(_) => PathBuf::from("x".repeat(OVERLONG_COMPONENT)),
                Slot::File => cell.join("file-as-dir"),
                Slot::Nowhere => cell.join("nowhere"),
                Slot::Target => cwd.clone(),
                // The Unix matrix's own spellings; no Windows layout
                // places them.
                Slot::Empty => PathBuf::new(),
                Slot::PaddedA(_) => cell.join("a"),
            };
            for slot in [Slot::PathDir, Slot::A, Slot::B, Slot::Other] {
                fs::create_dir_all(dir_of(slot)).unwrap();
            }
            let bare_file = match name.contains('.') {
                true => name.to_string(),
                false => format!("{name}.exe"),
            };
            let place_at = |slot: Slot| -> PathBuf {
                match (slot, direct) {
                    (Slot::Target, _) | (Slot::A, true) => target.clone(),
                    (slot, true) => dir_of(slot).join("dsh.exe"),
                    (slot, false) => dir_of(slot).join(&bare_file),
                }
            };
            let mut sentinels: Vec<PathBuf> = Vec::new();
            let mut placed: Vec<PathBuf> = Vec::new();
            // The literal drive path is never created: its cells assert
            // the absent path a runner has. A NUL name places nothing.
            if !name.contains('\0') && name != OPERATOR_PATH {
                for (slot, body) in &layout.files {
                    let at = place_at(*slot);
                    match body {
                        WindowsBody::Sentinel => {
                            if fs::hard_link(&exe, &at).is_err() {
                                fs::copy(&exe, &at).unwrap();
                            }
                            sentinels.push(at.clone());
                        }
                        WindowsBody::Text => {
                            fs::write(&at, b"not an image\r\n").unwrap();
                        }
                        WindowsBody::Directory => {
                            fs::create_dir_all(&at).unwrap();
                        }
                    }
                    placed.push(at);
                }
            }
            let path: Option<OsString> = layout.path.as_ref().map(|slots| {
                let mut entries: Vec<OsString> = slots
                    .iter()
                    .map(|slot| dir_of(*slot).into_os_string())
                    .collect();
                if let Some(at) = layout.empty_at {
                    entries.insert(at, OsString::new());
                }
                entries.join(std::ffi::OsStr::new(";"))
            });

            // The native oracle: the same name, the same cwd (this
            // process's), the same PATH.
            let mut command = Command::new(name);
            command
                .args([
                    "adapters::composite::tests::windows_matrix_sentinel_reports_its_own_image",
                    "--exact",
                    "--nocapture",
                ])
                .env(WINDOWS_SENTINEL, "1")
                .stdin(Stdio::null());
            match &path {
                Some(path) => command.env("PATH", path),
                None => command.env_remove("PATH"),
            };
            let native = command.output().map(|output| {
                let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
                match stdout
                    .lines()
                    .find_map(|line| line.strip_prefix("SENTINEL_EXE:"))
                {
                    Some(ran) => PathBuf::from(ran),
                    None => panic!(
                        "{name:?} in {}: the child ran an unidentified file: {stdout}",
                        layout.name
                    ),
                }
            });
            let resolved = resolve_executable_in(name, path.clone());
            let describe = |what: &str| {
                format!(
                    "{what}: name {name:?}, layout {:?}, PATH {path:?}, native {native:?}, \
                     resolver {resolved:?}",
                    layout.name
                )
            };
            let refusal = |what: &str| match &resolved {
                Ok(_) => panic!("{}", describe(what)),
                Err(error) => error.to_string(),
            };
            match &native {
                Err(error) if error.kind() == std::io::ErrorKind::InvalidInput => {
                    assert!(name.contains('\0'), "{}", describe("invalid input"));
                    assert_eq!(
                        refused(resolved.clone()),
                        "the DSH layout is unreadable: 'dsh\\0x' carries a NUL",
                        "{}",
                        describe("a NUL is refused up front")
                    );
                    nul += 1;
                }
                Ok(ran) => {
                    let ran = ran.canonicalize().unwrap();
                    assert!(
                        sentinels.iter().any(|at| at.canonicalize().unwrap() == ran),
                        "{}",
                        describe("the child ran a placed sentinel")
                    );
                    assert_eq!(
                        resolved.as_ref().ok(),
                        Some(&ran),
                        "{}",
                        describe("the resolver selects exactly the file the child ran")
                    );
                    equal += 1;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    refusal("the resolver selected a file where the child found nothing");
                    not_found += 1;
                }
                Err(_) => {
                    // An entry that exists and cannot run — no image, a
                    // directory — is the child's selection and its
                    // failure; the resolver refuses it by cause, and B
                    // is never selected in its place.
                    let reason = refusal("the resolver selected a file where the child stopped");
                    let b = place_at(Slot::B).display().to_string();
                    assert!(
                        !reason.contains(&b),
                        "{}",
                        describe("an existing entry that cannot run authorizes no later candidate")
                    );
                    assert!(
                        reason.contains("is not a loadable native image")
                            || reason.contains("is not a regular file"),
                        "{}",
                        describe("the refusal names the cause")
                    );
                    terminal += 1;
                }
            }
            for at in placed {
                let _ = fs::remove_file(&at);
                let _ = fs::remove_dir_all(&at);
            }
            let _ = fs::remove_dir_all(&cell);
        }
    }
    eprintln!(
        "matrix: {} names x {} layouts = {} cells; {equal} equal selections, {not_found} \
         NotFound parities, {terminal} terminal-error parities, {nul} NUL refusals, {pending} \
         PENDING operator-installation cells",
        names.len(),
        layouts.len(),
        cell_number
    );
    assert!(equal > 0 && not_found > 0 && terminal > 0 && nul > 0);
}

/// `Command::output`, retrying only the ETXTBSY a freshly staged
/// executable can answer with (#255); every other error is the oracle's
/// answer and is returned.
#[cfg(unix)]
fn matrix_spawn(command: &mut std::process::Command) -> std::io::Result<std::process::Output> {
    for _ in 0..50 {
        match command.output() {
            Err(error) if error.raw_os_error() == Some(26) => {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            result => return result,
        }
    }
    panic!("the executable stayed busy");
}

/// Copy the ELF64 image at `working` to `dest` with only the bytes of
/// its `PT_INTERP` path rewritten to `loader`, NUL-padded to the
/// segment's size, so the kernel reads a well-formed interpreter path
/// that names a file which does not exist.
#[cfg(unix)]
fn patch_elf_interpreter(working: &Path, dest: &Path, loader: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let mut bytes = fs::read(working).unwrap();
    assert_eq!(&bytes[..5], b"\x7fELF\x02", "a 64-bit ELF test binary");
    let u16_at = |bytes: &[u8], at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]);
    let u32_at = |bytes: &[u8], at: usize| {
        u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
    };
    let u64_at =
        |bytes: &[u8], at: usize| u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap());
    let phoff = u64_at(&bytes, 32) as usize;
    let phentsize = usize::from(u16_at(&bytes, 54));
    let phnum = usize::from(u16_at(&bytes, 56));
    let mut patched = false;
    for index in 0..phnum {
        let at = phoff + index * phentsize;
        if u32_at(&bytes, at) != 3 {
            continue;
        }
        let offset = u64_at(&bytes, at + 8) as usize;
        let filesz = u64_at(&bytes, at + 32) as usize;
        assert!(
            loader.len() < filesz,
            "the replacement fits the interpreter segment"
        );
        bytes[offset..offset + filesz].fill(0);
        bytes[offset..offset + loader.len()].copy_from_slice(loader.as_bytes());
        patched = true;
    }
    assert!(patched, "the test binary names a dynamic loader");
    let staging = dest.with_file_name(".broken.staging");
    fs::write(&staging, &bytes).unwrap();
    fs::set_permissions(&staging, fs::Permissions::from_mode(0o755)).unwrap();
    fs::rename(&staging, dest).unwrap();
    dest.to_path_buf()
}

/// No test in this file reassembles the component or composite stream:
/// the sole producer is the ONLY serializer, and an expectation is a
/// literal recorded from it. A test that rebuilt the NUL-separated
/// lines and hashed their concatenation passed every runtime assertion
/// while being a second producer (security hold 2026-09-20, finding 5).
/// This check reads the test source and fails if that block returns.
/// It detects that known block — a NUL pushed between path and digest,
/// or a hash taken of an assembled `lines` buffer — and nothing wider:
/// hashing one file's bytes on its own is an input hash, and stays
/// permitted.
///
/// A NUL-bearing line that is BUILT rather than recorded is the same
/// second producer under another spelling: Pass D's first profile suite
/// assembled `profile-bundle` / `profile-patch-reload` / `home-patch`
/// rows from the very case inputs its fixture had just written, and the
/// two patterns above did not see it (review 2026-09-23, finding 1). So
/// the NUL separator may appear in this source only in a RECORDED
/// literal or where a producer value is taken apart; a line that puts it
/// into a `format!`, a `write!` or a `push`/`push_str` is building the
/// stream and is refused by name.
#[test]
fn no_test_reassembles_the_component_stream() {
    let source = include_str!("tests.rs");
    // Spelled in pieces so that this test's own text is not the block
    // it forbids.
    let nul_push = format!("push('{}')", "\\0");
    let nul_escape = format!("\\u{}0{}", "{", "}");
    let builders = ["format!", "write!", ".push(", ".push_str("];
    let hashed_stream = [
        format!("digest_of({}", "lines"),
        format!("sha256_text(&{}", "lines"),
        format!("sha256_hex({}", "lines"),
    ];
    let mut built = 0usize;
    for (number, line) in source.lines().enumerate() {
        let line_number = number + 1;
        assert!(
            !line.contains(&nul_push),
            "tests.rs:{line_number} pushes a NUL separator: a test that serializes the \
             component stream is a second producer"
        );
        for pattern in &hashed_stream {
            assert!(
                !line.contains(pattern.as_str()),
                "tests.rs:{line_number} hashes an assembled stream: a test that computes the \
                 component or composite is a second producer"
            );
        }
        if !line.contains(nul_escape.as_str()) {
            continue;
        }
        built += 1;
        for builder in builders {
            assert!(
                !line.contains(builder),
                "tests.rs:{line_number} builds a NUL-separated row with `{builder}`: a stream \
                 assembled from a case's own inputs is a second producer, and an expectation \
                 is a literal recorded from the sole producer"
            );
        }
    }
    // The NUL-bearing lines exist and were examined, so a source that
    // stopped spelling the separator at all could not pass this check by
    // examining nothing.
    assert!(built > 0, "the recorded streams carry the separator");
    // The check reads the file it lives in, which is not empty.
    assert!(source.contains("fn no_test_reassembles_the_component_stream"));
}

/// The pnpm reader through the SOLE PRODUCER over an installed lock:
/// each malformed spelling refuses by the pnpm component and a named
/// cause, and the well-formed spelling beside it reads (security hold
/// 2026-09-20, finding 3).
fn composite_over_pnpm(install: &Synthetic, lock: &str) -> Result<DshComposite, CompositeError> {
    write(&install.profile(), "pnpm-lock.yaml", lock.as_bytes());
    dsh_composite_with(&install.seams, &install.node(), &[])
}

fn pnpm_with_resolution(resolution: &str) -> String {
    format!("lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {resolution}\n")
}

#[test]
fn missing_pnpm_field_separation_and_unsupported_flow_syntax_refuse_by_reason() {
    let install = Synthetic::new();
    // The properly separated control is readable and is the identity
    // the malformed spellings can no longer converge on.
    let control =
        composite_over_pnpm(&install, &pnpm_with_resolution("{integrity: sha512-X}")).unwrap();
    assert!(control
        .dependencies
        .contains(&"debug 2.6.9 sha512-X".to_string()));

    for (resolution, reason) in [
        // No `: ` between key and value: in YAML this is one plain
        // scalar, and the inherited `split_once(':')` repaired it into
        // the separated control above.
        (
            "{integrity:sha512-X}",
            "the resolution field 'integrity' lacks ': ' separation: 'integrity:sha512-X'",
        ),
        (
            "{integrity: sha512-X, tarball:x}",
            "the resolution field 'tarball' lacks ': ' separation: 'tarball:x'",
        ),
        // Collection punctuation inside a plain flow scalar is syntax
        // this grammar does not read, wherever it sits in the value.
        (
            "{integrity: sha512-X[one]}",
            "the resolution field 'integrity' carries unsupported flow syntax: 'sha512-X[one]'",
        ),
        (
            "{integrity: sha512-]X}",
            "the resolution field 'integrity' carries unsupported flow syntax: 'sha512-]X'",
        ),
        (
            "{integrity: sha512-X, tarball: a[b]}",
            "the resolution field 'tarball' carries unsupported flow syntax: 'a[b]'",
        ),
        // A plain `: ` inside a flow field opens a mapping YAML would
        // not read here, in the ignored `tarball` as much as anywhere:
        // the second hold's third vector (finding 2).
        (
            "{integrity: sha512-X, tarball: x: y}",
            "the resolution field 'tarball' carries unsupported flow syntax: 'x: y'",
        ),
        (
            "{integrity: sha512-X: y}",
            "the resolution field 'integrity' carries unsupported flow syntax: 'sha512-X: y'",
        ),
        // A colon ENDING a plain flow scalar is the mapping indicator
        // too, whether the document follows it with padding, the
        // closing brace or a comma: `x: }` and `x:}` read as the
        // control before the trailing padding was consumed ahead of
        // the syntax rule (review 2026-09-20, R3).
        (
            "{integrity: sha512-X, tarball: x: }",
            "the resolution field 'tarball' carries unsupported flow syntax: 'x:'",
        ),
        (
            "{integrity: sha512-X, tarball: x:}",
            "the resolution field 'tarball' carries unsupported flow syntax: 'x:'",
        ),
        (
            "{integrity: sha512-X:}",
            "the resolution field 'integrity' carries unsupported flow syntax: 'sha512-X:'",
        ),
        (
            "{integrity: sha512-X: }",
            "the resolution field 'integrity' carries unsupported flow syntax: 'sha512-X:'",
        ),
        (
            "{integrity: sha512-X:, tarball: y}",
            "the resolution field 'integrity' carries unsupported flow syntax: 'sha512-X:'",
        ),
    ] {
        assert_eq!(
            refused_vector(
                composite_over_pnpm(&install, &pnpm_with_resolution(resolution)),
                resolution
            ),
            format!("pnpm lock is unreadable: {reason}"),
            "{resolution}"
        );
    }

    // The header and the OUTER mapping meet the same separation rule
    // BEFORE anything is trimmed: `lockfileVersion:9.0` and
    // `resolution:{integrity: sha512-X}` are each one plain scalar in
    // YAML, and the inherited `trim` repaired both into the readable
    // control (security hold 2026-09-20, finding 2). Every other child
    // is a separated mapping too, read or not.
    for (lock, reason) in [
        (
            "lockfileVersion:9.0\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n".to_string(),
            "a lockfileVersion header that lacks ': ' separation",
        ),
        (
            "lockfileVersion:'9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n".to_string(),
            "a lockfileVersion header that lacks ': ' separation",
        ),
        (
            "lockfileVersion:\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n".to_string(),
            "a malformed lockfileVersion",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution:{integrity: sha512-X}\n".to_string(),
            "a package child 'resolution' that lacks ': ' separation",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n    version:1.0\n".to_string(),
            "a package child 'version' that lacks ': ' separation",
        ),
        (
            "lockfileVersion: '9.0'\n\nsettings:x\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n".to_string(),
            "a top-level key 'settings' that lacks ': ' separation",
        ),
        (
            "lockfileVersion: '9.0'\n\npnpmfileChecksum:sha256-x\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n".to_string(),
            "a top-level key 'pnpmfileChecksum' that lacks ': ' separation",
        ),
        // A package heading whose key ends in a colon once its own
        // colon and padding are gone — `debug@2.6.9: :` and
        // `debug@2.6.9::` — is a nested mapping, not a heading with a
        // version spelled `2.6.9:` (review 2026-09-20, R3).
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9: :\n    resolution: {integrity: sha512-X}\n".to_string(),
            "a package key that is itself a mapping",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9::\n    resolution: {integrity: sha512-X}\n".to_string(),
            "a package key that is itself a mapping",
        ),
        // The bodies of the sections this reader does not READ are
        // still sections the document must have spelled: an
        // unterminated quote under `snapshots` or `importers` read as
        // the control before (run `09ec8d81`, R4).
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n\nsnapshots:\n\n  debug@2.6.9:\n    dependencies:\n      ms: '2.0.0\n".to_string(),
            "a line in section 'snapshots' carrying the entry 'ms' carrying the malformed quoted \
             scalar ''2.0.0'",
        ),
        (
            "lockfileVersion: '9.0'\n\nimporters:\n\n  .:\n    dependencies:\n      debug:\n        specifier: \"^2\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n".to_string(),
            "a line in section 'importers' carrying the entry 'specifier' carrying the malformed \
             quoted scalar '\"^2'",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n\nsnapshots:\n\n  debug@2.6.9:\n    transitivePeerDependencies:\n      -x\n".to_string(),
            "a line in section 'snapshots' carrying the line '-x', which is neither a mapping \
             entry nor a sequence item",
        ),
        (
            "lockfileVersion: '9.0'\n\nsettings:\n  autoInstallPeers: [true,,false]\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n".to_string(),
            "a line in section 'settings' carrying the entry 'autoInstallPeers' carrying the \
             flow collection '[true,,false]' with a missing member at position 2",
        ),
        // R3 (fourth hold): the STRUCTURE of an ignored body, which a
        // line rule that saw each line without its neighbours could not
        // refuse. A child below a scalar entry, a sequence item beside
        // mapping entries and the reverse, a child below a sequence
        // item, and a dedent to an indentation no open block has — in
        // a package child's block and in an ignored section alike —
        // are each a document YAML refuses, and each read as the
        // control before.
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n    peerDependencies:\n      react: '>=16.8.0'\n        foo: bar\n".to_string(),
            "a line under the package child 'peerDependencies' carrying the entry 'foo' nested \
             below the scalar entry 'react', which opens no block",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n    peerDependencies:\n      react: '>=16.8.0'\n      - foo\n".to_string(),
            "a line under the package child 'peerDependencies' carrying the sequence item 'foo' \
             at 6 spaces beside mapping entries, which mixes mapping entries and sequence items \
             in one block",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n\nsnapshots:\n\n  debug@2.6.9:\n    transitivePeerDependencies:\n      - supports-color\n      ms: 2.0.0\n".to_string(),
            "a line in section 'snapshots' carrying the entry 'ms' at 6 spaces beside sequence \
             items, which mixes mapping entries and sequence items in one block",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n\nsnapshots:\n\n  foo: 1\n    bar: 2\n".to_string(),
            "a line in section 'snapshots' carrying the entry 'bar' nested below the scalar \
             entry 'foo', which opens no block",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n\nsnapshots:\n\n  debug@2.6.9:\n    transitivePeerDependencies:\n      - supports-color\n        bar: 2\n".to_string(),
            "a line in section 'snapshots' carrying the entry 'bar' nested below the sequence \
             item 'supports-color', which opens no block",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n\nsnapshots:\n\n  debug@2.6.9:\n      deep: 1\n    mid: 2\n".to_string(),
            "a line in section 'snapshots' carrying the entry 'mid' at 4 spaces, which dedents \
             to no open block",
        ),
        (
            "lockfileVersion: '9.0'\n\nimporters:\n\n    .:\n      x: 1\n  y: 2\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n".to_string(),
            "a line in section 'importers' carrying the entry 'y' at 2 spaces, which dedents to \
             no open block",
        ),
        // R2 (review of run `124cca78`): a REPEATED KEY in a mapping
        // this grammar admits and does not read — a flow map, a block
        // under a package child, a `snapshots` or `settings` body at
        // either depth, and a package child spelled twice — is a
        // document YAML refuses (§3.2.1.1: a mapping's keys are unique),
        // and each read as the control before. The key is the DECODED
        // one: `'node'` and `node`, `'react'` and `react` are one key.
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n    engines: {node: '>=18', node: '>=20'}\n".to_string(),
            "a package child 'engines' carrying the flow map '{node: '>=18', node: '>=20'}' with \
             the repeated key 'node'",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n    engines: {node: 18, 'node': 20}\n".to_string(),
            "a package child 'engines' carrying the flow map '{node: 18, 'node': 20}' with the \
             repeated key 'node'",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n    peerDependencies:\n      react: '>=16'\n      react: '>=17'\n".to_string(),
            "a line under the package child 'peerDependencies' carrying the entry 'react' at 6 \
             spaces, which repeats a key of its block",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n    peerDependencies:\n      react: '>=16'\n      'react': '>=17'\n".to_string(),
            "a line under the package child 'peerDependencies' carrying the entry 'react' at 6 \
             spaces, which repeats a key of its block",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n\nsnapshots:\n\n  debug@2.6.9:\n    dependencies:\n      ms: 2.0.0\n      ms: 2.0.0\n".to_string(),
            "a line in section 'snapshots' carrying the entry 'ms' at 6 spaces, which repeats a \
             key of its block",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n\nsnapshots:\n\n  debug@2.6.9: {}\n  debug@2.6.9: {}\n".to_string(),
            "a line in section 'snapshots' carrying the entry 'debug@2.6.9' at 2 spaces, which \
             repeats a key of its block",
        ),
        (
            "lockfileVersion: '9.0'\n\nsettings:\n  autoInstallPeers: true\n  autoInstallPeers: false\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n".to_string(),
            "a line in section 'settings' carrying the entry 'autoInstallPeers' at 2 spaces, \
             which repeats a key of its block",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n    cpu: [x64]\n    cpu: [arm64]\n".to_string(),
            "a repeated package child 'cpu'",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n    peerDependencies:\n      a: '1'\n    peerDependencies:\n      b: '2'\n".to_string(),
            "a repeated package child 'peerDependencies'",
        ),
        // R2, second sitting: keys are compared as YAML compares them —
        // by node, not by text. The padding before a plain key's colon
        // is the separator's (`react : b` spells `react`), and a plain
        // key that spells a typed scalar (`11`/`0xB`, `true`/`True`,
        // `null`/`~`) is refused by that cause before any comparison,
        // because this grammar resolves no type and a text set called
        // each pair two keys — and called plain `true` and quoted
        // `'true'`, which ARE two keys, one.
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n    peerDependencies:\n      react: '>=16'\n      react : '>=17'\n".to_string(),
            "a line under the package child 'peerDependencies' carrying the entry 'react' at 6 \
             spaces, which repeats a key of its block",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n    engines: {11: 1, 0xB: 2}\n".to_string(),
            "a package child 'engines' carrying the flow map '{11: 1, 0xB: 2}' with the key \
             '11', which is a number and not a string",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n    peerDependencies:\n      true: a\n      True: b\n".to_string(),
            "a line under the package child 'peerDependencies' carrying the key 'true', which \
             is a boolean and not a string",
        ),
        (
            "lockfileVersion: '9.0'\n\nsettings:\n  null: a\n  ~: b\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n".to_string(),
            "a line in section 'settings' carrying the key 'null', which is a null and not a \
             string",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n    engines: {true: a, 'true': b}\n".to_string(),
            "a package child 'engines' carrying the flow map '{true: a, 'true': b}' with the \
             key 'true', which is a boolean and not a string",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n    peerDependencies:\n      42: a\n".to_string(),
            "a line under the package child 'peerDependencies' carrying the key '42', which is \
             a number and not a string",
        ),
    ] {
        assert_eq!(
            refused_vector(composite_over_pnpm(&install, &lock), &lock),
            format!("pnpm lock is unreadable: {reason}"),
            "{lock:?}"
        );
    }
    // The keys YAML keeps apart stay apart and readable: a plain
    // `react` beside a quoted `'react '` (the quote keeps its space), a
    // single padded key, and the quoted spellings of typed scalars,
    // which are strings — two of them.
    for child in [
        "peerDependencies:\n      react: a\n      'react ': b",
        "peerDependencies:\n      react : '>=16'",
        "engines: {'true': a, 'True': b}",
        "engines: {'11': 1, '0xB': 2}",
    ] {
        let lock = format!(
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {{integrity: sha512-X}}\n    {child}\n"
        );
        assert_eq!(
            composite_over_pnpm(&install, &lock).unwrap().canonical,
            control.canonical,
            "{child:?}"
        );
    }
    let with_child = |child: &str| {
        format!(
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {{integrity: sha512-X}}\n    {child}\n"
        )
    };
    // R1 (review of run `124cca78`): separator padding cannot hide a
    // scalar's opening. An alias, an anchor and a reserved indicator are
    // refused behind one, two and more spaces alike — by pnpm, the
    // `engines` member and the opening indicator — where two spaces read
    // each as the valid control's composite.
    for opening in ["*missing", "&", "%bad"] {
        for padding in [" ", "  ", "   ", "        "] {
            let member = format!("node:{padding}{opening}");
            let lock = with_child(&format!("engines: {{{member}}}"));
            assert_eq!(
                refused_vector(composite_over_pnpm(&install, &lock), &lock),
                format!(
                    "pnpm lock is unreadable: a package child 'engines' carrying the malformed \
                     flow member '{member}', whose value opens with the YAML indicator '{}'",
                    opening.chars().next().unwrap()
                ),
                "{lock:?}"
            );
        }
    }
    // The same padding before a value YAML reads stays readable, and is
    // the control: numeric, plain and quoted, in a map and in a sequence
    // beside it. A quoted value keeps the spaces inside its quotes.
    for child in [
        "engines: {node: 22}",
        "engines: {node:    22}",
        "engines: {node:  '>=18',   npm:     \"9\"}",
        "engines: {node:   '  *kept  '}",
    ] {
        assert_eq!(
            composite_over_pnpm(&install, &with_child(child))
                .unwrap()
                .canonical,
            control.canonical,
            "{child:?}"
        );
    }
    // A tab is not separation here and keeps its own, document-wide
    // refusal: the separator run consumed above is spaces and no more.
    let tabbed = with_child("engines: {node: \t22}");
    assert_eq!(
        refused_vector(composite_over_pnpm(&install, &tabbed), &tabbed),
        "pnpm lock is unreadable: a tab"
    );
    // A quote OPENS a quoted scalar: one that never closes is that
    // scalar's own fault behind any padding, and is not reported as an
    // indicator opening.
    for padding in [" ", "   "] {
        let member = format!("node:{padding}'oops");
        let unclosed = with_child(&format!("engines: {{{member}}}"));
        assert_eq!(
            refused_vector(composite_over_pnpm(&install, &unclosed), &unclosed),
            format!(
                "pnpm lock is unreadable: a package child 'engines' carrying the malformed flow \
                 member '{member}'"
            )
        );
    }

    // R3 (review of run `124cca78`): an implicit block key is found by a
    // lookahead YAML bounds at 1,024 characters, counted over the key AS
    // SPELLED — quotes and pre-colon padding included, indentation, colon
    // and value not. Every 1,025-character span refuses by pnpm, the
    // `peerDependencies` body and the limit; every 1,024-character span
    // is the valid ignored-body control's composite.
    let peer =
        |key: &str, value: &str| with_child(&format!("peerDependencies:\n      {key}: {value}"));
    let body_control = composite_over_pnpm(&install, &peer("react", "a"))
        .unwrap()
        .canonical;
    assert_eq!(body_control, control.canonical);
    let ascii = |count: usize| "k".repeat(count);
    // U+00E9 is two bytes: 1,024 of them are 2,048 bytes and one key.
    let multibyte = |count: usize| "\u{e9}".repeat(count);
    for (key, admitted) in [
        (ascii(1024), true),
        (ascii(1025), false),
        (format!("{} ", ascii(1023)), true),
        (format!("{} ", ascii(1024)), false),
        (format!("{}     ", ascii(1020)), false),
        (format!("'{}'", ascii(1022)), true),
        (format!("'{}'", ascii(1023)), false),
        (format!("\"{}\"", ascii(1022)), true),
        (format!("\"{}\"", ascii(1023)), false),
        (multibyte(1024), true),
        (multibyte(1025), false),
        (format!("{} ", multibyte(1023)), true),
        (format!("{} ", multibyte(1024)), false),
        (format!("'{}'", multibyte(1022)), true),
        (format!("'{}'", multibyte(1023)), false),
    ] {
        let lock = peer(&key, "a");
        let observed = composite_over_pnpm(&install, &lock);
        let spelled = key.chars().count();
        match admitted {
            true => assert_eq!(
                observed.unwrap().canonical,
                body_control,
                "{spelled} characters"
            ),
            false => assert_eq!(
                refused(observed),
                "pnpm lock is unreadable: a line under the package child 'peerDependencies' \
                 carrying an implicit key past YAML's implicit-key lookahead limit of 1,024 \
                 characters",
                "{spelled} characters"
            ),
        }
    }
    // The bound is the KEY's. A long value and a long line are YAML, in
    // the body and beside it.
    let long = "v".repeat(4096);
    for child in [
        format!("peerDependencies:\n      react: {long}"),
        format!("peerDependencies:\n      {}: '{long}'", ascii(1024)),
        format!("deprecated: {long}"),
    ] {
        assert_eq!(
            composite_over_pnpm(&install, &with_child(&child))
                .unwrap()
                .canonical,
            control.canonical,
            "a {}-byte child",
            child.len()
        );
    }
    // The same bound on the OTHER route that admits a key of the
    // document's choosing, the `packages:` heading (review of run
    // `e291e076`, R3). The heading trims its pre-colon padding before it
    // reads the scalar, so `debug@2.6.9` behind 1,014 spaces — a
    // 1,025-character span — read as the valid control's composite. Each
    // padded spelling decodes to the control's own key, so every admitted
    // one IS the control; the unpadded long version is another package
    // and only has to be read.
    let heading = |key: &str| {
        format!(
            "lockfileVersion: '9.0'\n\npackages:\n\n  {key}:\n    resolution: {{integrity: sha512-X}}\n"
        )
    };
    let padded = |key: &str, span: usize| format!("{key}{}", " ".repeat(span - key.len()));
    let versioned = |span: usize| format!("debug@2.6.9-{}", "k".repeat(span - 12));
    for (key, admitted) in [
        (padded("debug@2.6.9", 1024), Some(true)),
        (padded("debug@2.6.9", 1025), None),
        (padded("debug@2.6.9", 4096), None),
        (padded("'debug@2.6.9'", 1024), Some(true)),
        (padded("'debug@2.6.9'", 1025), None),
        (padded("\"debug@2.6.9\"", 1024), Some(true)),
        (padded("\"debug@2.6.9\"", 1025), None),
        (versioned(1024), Some(false)),
        (versioned(1025), None),
        (format!("'{}'", versioned(1022)), Some(false)),
        (format!("'{}'", versioned(1023)), None),
    ] {
        let observed = composite_over_pnpm(&install, &heading(&key));
        let spelled = key.chars().count();
        match admitted {
            Some(control_key) => assert_eq!(
                observed.unwrap().canonical == control.canonical,
                control_key,
                "{spelled} characters"
            ),
            None => assert_eq!(
                refused(observed),
                "pnpm lock is unreadable: a package key that is an implicit key past YAML's \
                 implicit-key lookahead limit of 1,024 characters",
                "{spelled} characters"
            ),
        }
    }
    // Trailing padding AFTER the heading's colon is not the key's span.
    let trailing = format!(
        "lockfileVersion: '9.0'\n\npackages:\n\n  {}:{}\n    resolution: {{integrity: sha512-X}}\n",
        padded("debug@2.6.9", 1024),
        " ".repeat(64)
    );
    assert_eq!(
        composite_over_pnpm(&install, &trailing).unwrap().canonical,
        control.canonical
    );
    // A key repeated in ANOTHER block is another key: two importers each
    // with `dependencies`, two records each with `cpu`, two
    // `peerDependenciesMeta` children each with `optional` — the shape
    // of every real monorepo lock, and the control's own digest.
    let sibling_blocks = "lockfileVersion: '9.0'\n\nimporters:\n\n  .:\n    dependencies:\n      debug:\n        specifier: ^2.6.9\n        version: 2.6.9\n  packages/app:\n    dependencies:\n      debug:\n        specifier: ^2.6.9\n        version: 2.6.9\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n    cpu: [x64]\n    peerDependenciesMeta:\n      react:\n        optional: true\n      '@scope/peer':\n        optional: true\n  ms@2.0.0:\n    resolution: {integrity: sha512-M}\n    cpu: [x64]\n    engines: {node: '>=18', npm: '>=9'}\n";
    let two_records = composite_over_pnpm(&install, sibling_blocks).unwrap();
    assert_ne!(two_records.canonical, control.canonical);
    assert_eq!(
        two_records.canonical,
        composite_over_pnpm(
            &install,
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n  ms@2.0.0:\n    resolution: {integrity: sha512-M}\n"
        )
        .unwrap()
        .canonical,
        "the sibling blocks are admitted and ignored"
    );
    // The bodies a real lock carries — importer specifiers with a
    // `link:` or `npm:` colon, quoted keys, a sequence of transitive
    // peers, a nested `peerDependenciesMeta` — are admitted as syntax
    // and still ignored: the control's own digest.
    let bodies = "lockfileVersion: '9.0'\n\nimporters:\n\n  .:\n    dependencies:\n      debug:\n        specifier: ^2.6.9\n        version: 2.6.9\n      local:\n        specifier: link:../local\n        version: link:../local\n      '@scope/renamed':\n        specifier: npm:debug@^2\n        version: debug@2.6.9\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n    peerDependencies:\n      '@scope/peer': '>=1'\n      react: '>=16.8.0 || ^17'\n    peerDependenciesMeta:\n      react:\n        optional: true\n\nsnapshots:\n\n  debug@2.6.9:\n    dependencies:\n      ms: 2.0.0\n    transitivePeerDependencies:\n      - supports-color\n      - '@scope/peer'\n";
    assert_eq!(
        composite_over_pnpm(&install, bodies).unwrap().canonical,
        control.canonical
    );
    // The structural controls the rule above must keep readable: a
    // block key with no members (YAML's null) followed by its sibling, a
    // dedent back to an open block after a deeper one, a sequence block
    // after a mapping block under one parent, and a body whose first
    // line sits deeper than two spaces.
    let structured = "lockfileVersion: '9.0'\n\nimporters:\n\n    .:\n      dependencies:\n        debug:\n          specifier: ^2.6.9\n          version: 2.6.9\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n    peerDependenciesMeta:\n      react:\n        optional: true\n      '@scope/peer':\n        optional: false\n\nsnapshots:\n\n  debug@2.6.9:\n    optionalDependencies:\n    dependencies:\n      ms: 2.0.0\n      '@scope/peer': 1.0.0\n    transitivePeerDependencies:\n      - supports-color\n  ms@2.0.0: {}\n";
    assert_eq!(
        composite_over_pnpm(&install, structured).unwrap().canonical,
        control.canonical
    );
    // The separated spellings of each, plain and quoted, and a
    // colon-without-space URL value remain readable and reach the
    // control's identity where their identity bytes are the control's.
    for lock in [
        "lockfileVersion: 9.0\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n",
        "lockfileVersion: \"9.0\"\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n",
        "lockfileVersion: '9.0'  \n\npackages:  \n\n  debug@2.6.9:  \n    resolution:   {integrity: sha512-X}  \n    version: 1.0\n    engines: {node: '>=1'}\n",
        "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X, tarball: https://example.invalid/debug-2.6.9.tgz}\n",
        // A recognized top-level scalar, quoted: read, not identity.
        "lockfileVersion: '9.0'\n\npnpmfileChecksum: 'sha256-a'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n",
    ] {
        assert_eq!(
            composite_over_pnpm(&install, lock).unwrap().canonical,
            control.canonical,
            "{lock:?}"
        );
    }

    // Quote-aware splitting: a comma INSIDE a quoted scalar belongs to
    // the scalar, so a legal quoted string is read as itself rather than
    // refused for the wrong reason.
    let quoted = composite_over_pnpm(
        &install,
        &pnpm_with_resolution("{integrity: 'sha512-a,b', tarball: \"x,y\"}"),
    )
    .unwrap();
    assert!(quoted
        .dependencies
        .contains(&"debug 2.6.9 sha512-a,b".to_string()));
    assert_ne!(quoted.canonical, control.canonical);

    // YAML's own CHARACTER SET, asked of the whole document. A NUL or a
    // BEL is not a value with an unusual byte in it — it is a stream
    // YAML cannot carry — and the fields this reader IGNORES carried
    // them straight through to the control's composite (review
    // 2026-09-20, F5, security). The plain and quoted tarball and the
    // two recognized checksums are the reproduced vectors.
    for (lock, control_char) in [
        (
            pnpm_with_resolution("{integrity: sha512-X, tarball: http://x\u{0}}"),
            0u32,
        ),
        (
            pnpm_with_resolution("{integrity: sha512-X, tarball: http://x\u{7}}"),
            7,
        ),
        (
            pnpm_with_resolution("{integrity: sha512-X, tarball: 'http://x\u{0}'}"),
            0,
        ),
        (pnpm_with_resolution("{integrity: 'sha512-X\u{1b}'}"), 0x1b),
        (
            "lockfileVersion: '9.0'\n\npnpmfileChecksum: sha256-x\u{0}\n\npackages:\n\n  \
             debug@2.6.9:\n    resolution: {integrity: sha512-X}\n"
                .to_string(),
            0,
        ),
        (
            "lockfileVersion: '9.0'\n\npackageExtensionsChecksum: sha256-x\u{7f}\n\npackages:\n\n  \
             debug@2.6.9:\n    resolution: {integrity: sha512-X}\n"
                .to_string(),
            0x7f,
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: \
             {integrity: sha512-X}\n    deprecated: use \u{0} instead\n"
                .to_string(),
            0,
        ),
        // The upper edge of the admitted range: the two noncharacters
        // that end the basic plane are outside `c-printable` where
        // U+FFFD, just below them, is inside it.
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: \
             {integrity: sha512-X}\n    deprecated: \u{fffe}\n"
                .to_string(),
            0xfffe,
        ),
    ] {
        assert_eq!(
            refused_vector(composite_over_pnpm(&install, &lock), &lock),
            format!(
                "pnpm lock is unreadable: the character U+{control_char:04X}, which YAML's \
                 character set excludes"
            ),
            "{lock:?}"
        );
    }

    // A recognized top-level scalar is a SCALAR: a checksum spelled
    // `a: b` is the mapping YAML opens there, and the reader that read
    // it as a string left the control's composite standing.
    for lock in [
        "lockfileVersion: '9.0'\n\npnpmfileChecksum: sha256-a: b\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n",
        "lockfileVersion: '9.0'\n\npnpmfileChecksum: sha256-a:\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n",
        "lockfileVersion: '9.0'\n\npackageExtensionsChecksum: 'sha256-a\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n",
    ] {
        let name = match lock.contains("pnpmfileChecksum") {
            true => "pnpmfileChecksum",
            false => "packageExtensionsChecksum",
        };
        assert_eq!(
            refused_vector(composite_over_pnpm(&install, lock), lock),
            format!("pnpm lock is unreadable: a malformed scalar for top-level key '{name}'"),
            "{lock:?}"
        );
    }

    // An IGNORED package child is still a field the document must have
    // spelled. An unterminated `deprecated` quote and an unterminated
    // `engines` flow map were skipped whole, so a file that is not a
    // YAML document at all produced the control's composite.
    for (child, reason) in [
        (
            "deprecated: 'oops",
            "a package child 'deprecated' carrying the malformed quoted scalar ''oops'",
        ),
        (
            "deprecated: \"oops",
            "a package child 'deprecated' carrying the malformed quoted scalar '\"oops'",
        ),
        (
            "deprecated: \"a\\tb\"",
            "a package child 'deprecated' carrying the malformed quoted scalar '\"a\\tb\"'",
        ),
        (
            "engines: {node: >=1",
            "a package child 'engines' carrying the unterminated flow collection '{node: >=1'",
        ),
        (
            "cpu: [x64",
            "a package child 'cpu' carrying the unterminated flow collection '[x64'",
        ),
        (
            "engines: {node: {min: 1}}",
            "a package child 'engines' carrying the nested flow collection '{node: {min: 1}}', \
             which this grammar does not read",
        ),
        (
            "engines: {node:'>=1'}",
            "a package child 'engines' carrying the flow map '{node:'>=1'}' with the member \
             'node:'>=1'', which is not a `key: value` entry",
        ),
        (
            "engines: {node: }",
            "a package child 'engines' carrying the flow map '{node: }' with the member \
             'node:', which is not a `key: value` entry",
        ),
        (
            "engines: {node: a: b}",
            "a package child 'engines' carrying the malformed flow member 'node: a: b'",
        ),
        // The KEY half of a flow member is a scalar in its own right: an
        // unterminated quote swallows the separator, so no entry remains.
        (
            "engines: {'oops: 1}",
            "a package child 'engines' carrying the flow map '{'oops: 1}' with the member \
             ''oops: 1', which is not a `key: value` entry",
        ),
        // A MISSING member is a member the document did not spell:
        // leading, interior, and a comma-only body each refuse by
        // position, where every empty field was dropped before (run
        // `09ec8d81`, R4). `[]`, `{}` and one trailing comma remain the
        // forms YAML admits, asserted among the readable controls.
        (
            "cpu: [,x64]",
            "a package child 'cpu' carrying the flow collection '[,x64]' with a missing member \
             at position 1",
        ),
        (
            "cpu: [x64,,arm64]",
            "a package child 'cpu' carrying the flow collection '[x64,,arm64]' with a missing \
             member at position 2",
        ),
        (
            "cpu: [x64,,]",
            "a package child 'cpu' carrying the flow collection '[x64,,]' with a missing member \
             at position 2",
        ),
        (
            "engines: {,node: 22}",
            "a package child 'engines' carrying the flow collection '{,node: 22}' with a missing \
             member at position 1",
        ),
        (
            "engines: {node: 18,,npm: 9}",
            "a package child 'engines' carrying the flow collection '{node: 18,,npm: 9}' with a \
             missing member at position 2",
        ),
        (
            "engines: {,}",
            "a package child 'engines' carrying the flow collection '{,}' with a missing member \
             at position 1",
        ),
        // A mapping's member is `key: value` and a sequence's is a
        // scalar; the other way round is a structure this grammar does
        // not read.
        (
            "engines: {node}",
            "a package child 'engines' carrying the flow map '{node}' with the member 'node', \
             which is not a `key: value` entry",
        ),
        (
            "cpu: [a: b]",
            "a package child 'cpu' carrying the flow sequence '[a: b]' with the member 'a: b', \
             which is a mapping and not a scalar",
        ),
        // The lines UNDER a block-form child were skipped whole, so an
        // unterminated quote in a `peerDependencies` body read as the
        // control. Each line is a mapping entry or a sequence item, its
        // key a scalar and its value admitted as every ignored value is.
        (
            "peerDependencies:\n      '@scope/peer': '>=1",
            "a line under the package child 'peerDependencies' carrying the entry '@scope/peer' \
             carrying the malformed quoted scalar ''>=1'",
        ),
        (
            "peerDependencies:\n      'oops: '>=1'",
            "a line under the package child 'peerDependencies' carrying the line ''oops: '>=1'', \
             which is not a mapping entry",
        ),
        (
            "peerDependencies:\n      - '@scope/peer",
            "a line under the package child 'peerDependencies' carrying the sequence item \
             carrying the malformed quoted scalar ''@scope/peer'",
        ),
        (
            "peerDependencies:\n      -",
            "a line under the package child 'peerDependencies' carrying the sequence item '-' \
             with no value",
        ),
        (
            "peerDependencies:\n      -   ",
            "a line under the package child 'peerDependencies' carrying the sequence item '-' \
             with no value",
        ),
        (
            "peerDependencies:\n      -x",
            "a line under the package child 'peerDependencies' carrying the line '-x', which is \
             neither a mapping entry nor a sequence item",
        ),
        (
            "peerDependencies:\n      bare",
            "a line under the package child 'peerDependencies' carrying the line 'bare', which \
             is not a mapping entry",
        ),
        (
            "peerDependencies:\n      'a' b: c",
            "a line under the package child 'peerDependencies' carrying the line ''a' b: c', \
             which is not a mapping entry",
        ),
        // `react:16` is one plain scalar in YAML, not an entry; a quoted
        // key ends where its quote ends, so `'react':16` is an entry
        // whose separator is missing.
        (
            "peerDependencies:\n      react:16",
            "a line under the package child 'peerDependencies' carrying the line 'react:16', \
             which is not a mapping entry",
        ),
        (
            "peerDependencies:\n      'react':16",
            "a line under the package child 'peerDependencies' carrying the entry 'react' lacks \
             ': ' separation",
        ),
        (
            "peerDependencies:\n      : x",
            "a line under the package child 'peerDependencies' carrying the malformed key ''",
        ),
        // A quoted key that never closes is read as the plain line it
        // is, and its key half is the unterminated quote.
        (
            "peerDependencies:\n      'oops: 1",
            "a line under the package child 'peerDependencies' carrying the malformed key ''oops'",
        ),
        // A flow map entry whose KEY is malformed after a closed quote.
        (
            "engines: {'a'b: 1}",
            "a package child 'engines' carrying the malformed flow member ''a'b: 1'",
        ),
        (
            "peerDependencies:\n      '@a': *ref",
            "a line under the package child 'peerDependencies' carrying the entry '@a' carrying \
             the value '*ref', which opens YAML syntax this grammar does not read",
        ),
        (
            "peerDependenciesMeta:\n      react:\n        optional: {a: [b]}",
            "a line under the package child 'peerDependenciesMeta' carrying the entry \
             'optional' carrying the nested flow collection '{a: [b]}', which this grammar does \
             not read",
        ),
        (
            "deprecated: *anchor",
            "a package child 'deprecated' carrying the value '*anchor', which opens YAML syntax \
             this grammar does not read",
        ),
        (
            "deprecated: a: b",
            "a package child 'deprecated' carrying the plain scalar 'a: b', which is a mapping \
             or a comment and not a value",
        ),
        (
            "deprecated: text #comment",
            "a package child 'deprecated' carrying the plain scalar 'text #comment', which is a \
             mapping or a comment and not a value",
        ),
    ] {
        let lock = format!(
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: \
             {{integrity: sha512-X}}\n    {child}\n"
        );
        assert_eq!(
            refused_vector(composite_over_pnpm(&install, &lock), &lock),
            format!("pnpm lock is unreadable: {reason}"),
            "{child:?}"
        );
    }

    // The valid ignored spellings a real lock carries are still read,
    // and still ignored: the admitted dialect did not grow. A plain
    // block scalar's free text — apostrophes, commas, brackets — is
    // free text, exactly as YAML has it.
    for child in [
        "engines: {node: '>=18.12', npm: \"9\"}",
        "engines: {}",
        "engines: {'k: x': v}",
        "cpu: [x64, arm64]",
        "cpu: [x64,]",
        "cpu: [x64, ]",
        "os: [darwin]",
        "os: ['a: b']",
        "os: [\"a: b\"]",
        "libc: []",
        "hasBin: true",
        "bundledDependencies: false",
        "name: debug",
        "version: 2.6.9",
        "deprecated: Don't use this, use [debug] instead",
        "deprecated: see https://example.invalid/a#b",
        "deprecated: 'use debug instead'",
        // The printable characters above the ASCII range YAML admits:
        // the private-use area, the replacement character and the
        // supplementary planes are all `c-printable`, and an ignored
        // free-text value may carry them.
        "deprecated: \u{e000} \u{fffd} \u{1f600}",
        "peerDependencies:",
    ] {
        let lock = format!(
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: \
             {{integrity: sha512-X}}\n    {child}\n"
        );
        assert_eq!(
            composite_over_pnpm(&install, &lock).unwrap().canonical,
            control.canonical,
            "{child:?}"
        );
    }
}

/// Identity bytes are never trimmed. A U+00A0 at either edge of a plain
/// integrity was erased by `str::trim` before the scalar rule ran, so the
/// padded lock produced the control's composite, while the quoted
/// spellings correctly refused (security hold 2026-09-20, finding 3).
/// Every spelling now reaches the same named refusal through the sole
/// producer, and a U+00A0 outside a scalar — on a blank line, beside a
/// key, after a section — cannot become admitted padding either.
#[test]
fn pnpm_integrity_preserves_unicode_whitespace_for_refusal() {
    let install = Synthetic::new();
    let control =
        composite_over_pnpm(&install, &pnpm_with_resolution("{integrity: sha512-X}")).unwrap();
    for resolution in [
        "{integrity: \u{a0}sha512-X}",
        "{integrity: sha512-X\u{a0}}",
        "{integrity: '\u{a0}sha512-X'}",
        "{integrity: 'sha512-X\u{a0}'}",
        "{integrity: \"\u{a0}sha512-X\"}",
        "{integrity: \"sha512-X\u{a0}\"}",
        "{integrity: \u{2003}sha512-X}",
    ] {
        assert_eq!(
            refused_vector(
                composite_over_pnpm(&install, &pnpm_with_resolution(resolution)),
                resolution
            ),
            "pnpm lock is unreadable: 'debug@2.6.9': integrity carries whitespace",
            "{resolution:?}"
        );
    }
    for (lock, reason) in [
        (
            "\u{a0}\nlockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n",
            "no lockfileVersion header",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\n\u{a0}\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n",
            "a top-level line that is not a mapping key",
        ),
        (
            "lockfileVersion: '9.0'\u{a0}\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n",
            "a malformed lockfileVersion",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\u{a0}\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n",
            "a top-level key 'packages' that lacks ': ' separation",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages: \u{a0}\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n",
            "an inline value on top-level section 'packages'",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\u{a0}\n    resolution: {integrity: sha512-X}\n",
            "a package key is not colon-terminated",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\u{a0}\n",
            "a block-form or malformed resolution",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {\u{a0}integrity: sha512-X}\n",
            "a malformed resolution flow map",
        ),
    ] {
        assert_eq!(
            refused_vector(composite_over_pnpm(&install, lock), lock),
            format!("pnpm lock is unreadable: {reason}"),
            "{lock:?}"
        );
    }
    // The unpadded control still reads to its own identity, and ASCII
    // padding the grammar owns is still consumed.
    assert_eq!(
        composite_over_pnpm(&install, &pnpm_with_resolution("{ integrity:  sha512-X }"))
            .unwrap()
            .canonical,
        control.canonical
    );
}

#[test]
fn pnpm_identity_strings_preserve_the_distinction_from_typed_scalars() {
    let install = Synthetic::new();
    // The readable controls: the QUOTED spellings are strings, and each
    // is its own identity.
    let quoted_null =
        composite_over_pnpm(&install, &pnpm_with_resolution("{integrity: 'null'}")).unwrap();
    assert!(quoted_null
        .dependencies
        .contains(&"debug 2.6.9 null".to_string()));
    let quoted_number =
        composite_over_pnpm(&install, &pnpm_with_resolution("{integrity: \"42\"}")).unwrap();
    assert!(quoted_number
        .dependencies
        .contains(&"debug 2.6.9 42".to_string()));
    assert_ne!(quoted_null.canonical, quoted_number.canonical);

    // Every plain typed spelling refuses by field, spelling and kind;
    // the plain null therefore has NO identity to share with `'null'`.
    for (value, kind) in [
        ("null", "a null"),
        ("Null", "a null"),
        ("NULL", "a null"),
        ("~", "a null"),
        ("true", "a boolean"),
        ("True", "a boolean"),
        ("TRUE", "a boolean"),
        ("false", "a boolean"),
        ("False", "a boolean"),
        ("FALSE", "a boolean"),
        ("42", "a number"),
        ("+42", "a number"),
        ("4.2", "a number"),
        (".5", "a number"),
        ("5.", "a number"),
        ("1e3", "a number"),
        ("1.5E-3", "a number"),
        ("+2e+2", "a number"),
        ("0x1F", "a number"),
        ("0o17", "a number"),
        ("0b101", "a number"),
        (".inf", "a number"),
        ("+.Inf", "a number"),
        (".INF", "a number"),
        (".nan", "a number"),
        (".NaN", "a number"),
        (".NAN", "a number"),
    ] {
        assert_eq!(
            refused_vector(
                composite_over_pnpm(
                    &install,
                    &pnpm_with_resolution(&format!("{{integrity: {value}}}"))
                ),
                value
            ),
            format!(
                "pnpm lock is unreadable: the resolution field 'integrity' is the plain scalar \
                 '{value}', which is {kind} and not a string"
            ),
            "{value}"
        );
    }
    // A leading `-` is a YAML indicator, refused as malformed syntax
    // before the kind is asked — the same closed grammar, one arm up.
    assert_eq!(
        refused(composite_over_pnpm(
            &install,
            &pnpm_with_resolution("{integrity: -1}")
        )),
        "pnpm lock is unreadable: a malformed resolution flow map"
    );

    // Plain spellings that LOOK numeric to a looser rule and are not
    // numbers remain strings: the rule is the schema's, not "starts
    // with a digit".
    for value in [
        "1abc", "0x", "0xZ", "1e", "1.2.3", "+", "nul", "yes", "no", "on", "off",
    ] {
        let readable = composite_over_pnpm(
            &install,
            &pnpm_with_resolution(&format!("{{integrity: {value}}}")),
        )
        .unwrap_or_else(|error| panic!("{value}: {error}"));
        assert!(
            readable
                .dependencies
                .contains(&format!("debug 2.6.9 {value}")),
            "{value}"
        );
    }
    // The `tarball` field is admitted but never read, so its scalar
    // kind is not asked: a plain number there is not an identity string.
    composite_over_pnpm(
        &install,
        &pnpm_with_resolution("{integrity: sha512-X, tarball: 42}"),
    )
    .unwrap();

    // The header keeps its own rule: plain `9.0` and quoted `'9.0'` are
    // both the admitted version, and neither is a "number" refusal.
    for header in ["9.0", "'9.0'", "\"9.0\""] {
        let lock = format!(
            "lockfileVersion: {header}\n\npackages:\n\n  debug@2.6.9:\n    resolution: {{integrity: sha512-X}}\n"
        );
        composite_over_pnpm(&install, &lock).unwrap_or_else(|error| panic!("{header}: {error}"));
    }
    assert_eq!(
        refused(composite_over_pnpm(
            &install,
            "lockfileVersion: 9\npackages:\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n"
        )),
        "pnpm lock is unreadable: lockfileVersion is not 9.0"
    );
}

/// The typed-scalar rule at its own unit, so every arm of the numeric
/// grammar is a plain assertion rather than a fact about which vectors
/// the producer test happened to list.
#[test]
fn the_plain_scalar_kind_rule_is_the_core_schema_s() {
    for value in [
        "0", "7", "42", "-1", "+1", "1.0", ".5", "5.", "1e3", "1E3", "1e-3", "1.5e+3", "0x0",
        "0xff", "0o7", "0b1", ".inf", "-.inf", "+.INF", ".nan",
    ] {
        assert!(looks_numeric(value), "{value}");
        assert_eq!(typed_plain_scalar(value), Some("a number"), "{value}");
    }
    for value in [
        "",
        "x",
        "1x",
        "1.2.3",
        "1e",
        "e3",
        "1e+",
        "0x",
        "0xg",
        "0o8",
        "0b2",
        ".",
        "+",
        "-",
        "inf",
        "nan",
        ".infinity",
        "sha512-X",
        "1_000",
        "1:20",
    ] {
        assert!(!looks_numeric(value), "{value}");
        assert_eq!(typed_plain_scalar(value), None, "{value}");
    }
    assert_eq!(typed_plain_scalar("~"), Some("a null"));
    assert_eq!(typed_plain_scalar("FALSE"), Some("a boolean"));
    assert_eq!(
        split_flow_fields("a: 'x,y', b: \"p,q\", c: z"),
        vec!["a: 'x,y'", " b: \"p,q\"", " c: z"]
    );
    assert_eq!(split_flow_fields(""), vec![""]);
    assert_eq!(split_flow_fields("a: 'x"), vec!["a: 'x"]);
    // A field with no colon at all, and a key the map does not admit,
    // are the map's own refusals.
    assert_eq!(
        pnpm_flow_map("integrity"),
        Err("a malformed resolution flow map".to_string())
    );
    assert_eq!(
        pnpm_flow_map("rogue: x"),
        Err("a malformed resolution flow map".to_string())
    );
    assert_eq!(
        pnpm_flow_map("integrity: 'x"),
        Err("a malformed resolution flow map".to_string())
    );
    assert_eq!(
        pnpm_flow_map("integrity: ''"),
        Err("a malformed resolution flow map".to_string())
    );
    assert_eq!(pnpm_flow_map("integrity: x"), Ok(vec![("integrity", "x")]));
    assert_eq!(pnpm_scalar("'a'"), Some(Scalar::Quoted("a")));
    assert_eq!(pnpm_scalar("a"), Some(Scalar::Plain("a")));
}

/// A repeated decoded `packages:` heading refuses before exclusion or
/// triple normalization, in every spelling of the repeat; equal
/// complete triples from DISTINCT valid records still deduplicate
/// (security hold 2026-09-20, finding 4).
#[test]
fn duplicate_decoded_pnpm_package_keys_refuse_before_triple_normalization() {
    let install = Synthetic::new();
    let base = install.composite();
    // The positive control this repair must not lose: the npm lock and
    // the pnpm lock each carry `debug 2.6.9 sha512-DEBUG`, two distinct
    // valid records, and the composite carries the triple ONCE.
    assert_eq!(
        base.dependencies
            .iter()
            .filter(|triple| triple.starts_with("debug "))
            .count(),
        1,
        "{:?}",
        base.dependencies
    );
    // Differing versions or integrities of one name remain distinct.
    let two = composite_over_pnpm(
        &install,
        "lockfileVersion: '9.0'\npackages:\n  debug@2.6.9:\n    resolution: {integrity: sha512-DEBUG}\n  debug@2.7.0:\n    resolution: {integrity: sha512-OTHER}\n",
    )
    .unwrap();
    assert!(two
        .dependencies
        .contains(&"debug 2.6.9 sha512-DEBUG".to_string()));
    assert!(two
        .dependencies
        .contains(&"debug 2.7.0 sha512-OTHER".to_string()));

    let record = |key: &str, integrity: &str| {
        format!("  {key}:\n    resolution: {{integrity: {integrity}}}\n")
    };
    for (name, lock) in [
        // Identical repeat: the triple set hid it as a deduplication.
        (
            "identical",
            format!(
                "lockfileVersion: '9.0'\npackages:\n{}{}",
                record("debug@2.6.9", "sha512-DEBUG"),
                record("debug@2.6.9", "sha512-DEBUG")
            ),
        ),
        // Conflicting repeat, in both orders: two integrities for one
        // key merged into an order-independent digest.
        (
            "conflicting",
            format!(
                "lockfileVersion: '9.0'\npackages:\n{}{}",
                record("debug@2.6.9", "sha512-A"),
                record("debug@2.6.9", "sha512-B")
            ),
        ),
        (
            "conflicting-reversed",
            format!(
                "lockfileVersion: '9.0'\npackages:\n{}{}",
                record("debug@2.6.9", "sha512-B"),
                record("debug@2.6.9", "sha512-A")
            ),
        ),
        // Quoted and unquoted spellings of ONE decoded key.
        (
            "quoted",
            format!(
                "lockfileVersion: '9.0'\npackages:\n{}{}",
                record("debug@2.6.9", "sha512-A"),
                record("'debug@2.6.9'", "sha512-B")
            ),
        ),
        (
            "double-quoted",
            format!(
                "lockfileVersion: '9.0'\npackages:\n{}{}",
                record("\"debug@2.6.9\"", "sha512-A"),
                record("debug@2.6.9", "sha512-A")
            ),
        ),
    ] {
        assert_eq!(
            refused_vector(composite_over_pnpm(&install, &lock), name),
            "pnpm lock is unreadable: a repeated package key 'debug@2.6.9'",
            "{name}"
        );
    }
    // A repeated LOCAL record: excluded from identity, and still a
    // repeated mapping key the document cannot mean two things by.
    let local = format!(
        "lockfileVersion: '9.0'\npackages:\n{}{}{}",
        record("debug@2.6.9", "sha512-DEBUG"),
        record("dsh-plugin-cli-session@file:plugin.tgz", "sha512-L"),
        record("dsh-plugin-cli-session@file:plugin.tgz", "sha512-L")
    );
    assert_eq!(
        refused(composite_over_pnpm(&install, &local)),
        "pnpm lock is unreadable: a repeated package key 'dsh-plugin-cli-session@file:plugin.tgz'"
    );
    // The same at the reader's own unit, with the reason and with no
    // partial answer: `Err`, not a vector missing one record.
    assert_eq!(
        refused(pnpm_dependencies(&local, &["dsh-plugin-cli-session"])),
        "pnpm lock is unreadable: a repeated package key 'dsh-plugin-cli-session@file:plugin.tgz'"
    );
}

/// An otherwise complete install whose plugin `package.json` alone is
/// gone: the refusal names the bundle AND the file the search ran out
/// of, through the whole producer (security hold 2026-09-20, finding 7).
/// The real doctor line over the same layout is asserted in
/// `crates/brokkr-cli/tests/doctor_dsh_selection.rs`.
#[test]
fn removing_only_the_plugin_manifest_names_the_drifted_file() {
    let install = Synthetic::new();
    let base = install.composite();
    let plugin = install
        .profile()
        .join("node_modules")
        .join("dsh-plugin-cli-session");
    fs::remove_file(plugin.join("package.json")).unwrap();
    assert_eq!(
        refused(dsh_composite_with(&install.seams, &install.node(), &[])),
        "the DSH layout is unreadable: bundle 'dsh-plugin-cli-session' does not resolve: \
         no package.json found"
    );
    write(&plugin, "package.json", b"package.json");

    // The lookup order is unchanged by the wording: an EARLIER candidate
    // directory with no manifest is true absence, and the search walks
    // on to the profile's legitimate later hit.
    let earlier = install
        .dir
        .path()
        .join("core/node_modules/@deepseek-ai/dsh/node_modules/dsh-plugin-cli-session");
    fs::create_dir_all(&earlier).unwrap();
    assert_eq!(install.composite().canonical, base.canonical);
    // An earlier candidate whose manifest is not a regular file still
    // stops the search: it is not absence, and it never falls through.
    fs::create_dir_all(earlier.join("package.json")).unwrap();
    assert_eq!(
        refused(dsh_composite_with(&install.seams, &install.node(), &[])),
        format!(
            "the DSH layout is unreadable: bundle 'dsh-plugin-cli-session': {} is not a \
             regular file",
            earlier.join("package.json").display()
        )
    );
}

/// The resolver classifies a candidate as the child's search would, and
/// REFUSES where it cannot prove what the child would do (security hold
/// 2026-09-20, finding 2). Each arm is asserted by its reason.
#[cfg(unix)]
#[test]
fn the_candidate_classifier_stops_where_the_child_stops_and_refuses_the_unprovable() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a");
    let b = dir.path().join("b");
    fs::create_dir_all(&a).unwrap();
    fs::create_dir_all(&b).unwrap();
    let real = stage_executable(&b, "dsh", b"#!/bin/sh\ntrue\n");
    let path = |first: &Path| {
        Some(OsString::from(format!(
            "{}:{}",
            first.display(),
            b.display()
        )))
    };

    // A SELF-SYMLINK at A/dsh. What the SEARCH does with it is each C
    // library's own: glibc's `execvp` stops with ELOOP and never reaches
    // B (measured 2026-09-20), and Apple's `execvP` and `posix_spawnp`
    // continue past it as they continue past ENOENT. The running
    // platform's own rule is what is asserted, never a fixed "Unix"
    // outcome — AS1's corrected cell (2026-09-20 spec defect, F3).
    std::os::unix::fs::symlink("dsh", a.join("dsh")).unwrap();
    let eloop = fs::metadata(a.join("dsh")).unwrap_err();
    assert_eq!(errno_of(&eloop), Some(rustix::io::Errno::LOOP), "{eloop}");
    let loop_stops = matches!(LIBRARY, Library::Glibc | Library::Musl);
    match loop_stops {
        true => assert_eq!(
            refused(resolve_executable_in("dsh", path(&a))),
            format!(
                "the DSH layout is unreadable: {}: a symlink loop stops the lookup: {eloop}",
                a.join("dsh").display()
            )
        ),
        false => assert_eq!(
            resolve_executable_in("dsh", path(&a)).unwrap(),
            real.canonicalize().unwrap(),
            "this platform's search continues past the loop to B"
        ),
    }
    // The same file as an explicit override is the SAME refusal on every
    // library's arm, because an override has no next entry to continue
    // to: the loop is named wherever it is met (run `551ef2a7`,
    // `a_direct_names_symlink_loop_is_named_on_every_librarys_arm`). The
    // library-qualified spelling this expectation used to carry was the
    // Apple-arm leftover of the rule that run replaced, and it is what
    // failed on PR #311's third macOS pass.
    assert_eq!(
        refused(resolve_executable_in(a.join("dsh").to_str().unwrap(), None)),
        format!(
            "the DSH layout is unreadable: {}: a symlink loop stops the lookup: {eloop}",
            a.join("dsh").display()
        )
    );
    fs::remove_file(a.join("dsh")).unwrap();

    // A MISSING INTERPRETER at A/dsh: metadata and access both admit
    // it, the child's `execve` would fail at exec and walk on to B, and
    // this resolver refuses at A by that cause rather than guessing.
    let missing = dir.path().join("no-such-interpreter");
    stage_executable(&a, "dsh", format!("#!{}\n", missing.display()).as_bytes());
    let enoent = fs::metadata(&missing).unwrap_err();
    assert_eq!(
        refused(resolve_executable_in("dsh", path(&a))),
        format!(
            "the DSH layout is unreadable: {}: its #! interpreter '{}' is missing: {enoent}",
            a.join("dsh").display(),
            missing.display()
        )
    );

    // Every other obstruction the head can show, each by its reason.
    // Trailing spaces and tabs before the interpreter are skipped as the
    // kernel skips them; a CR is not a terminator.
    let interpreter_is_dir = format!("#!{}\n", dir.path().display());
    let not_executable = dir.path().join("plain-interpreter");
    fs::write(&not_executable, b"#!/bin/sh\n").unwrap();
    fs::set_permissions(&not_executable, fs::Permissions::from_mode(0o644)).unwrap();
    let mut unterminated = b"#!/bin/sh".to_vec();
    unterminated.resize(SHEBANG_BOUND + 10, b' ');
    for (body, reason) in [
        (
            b"#!\n".to_vec(),
            "its #! line names no interpreter".to_string(),
        ),
        (
            b"#! \t \n".to_vec(),
            "its #! line names no interpreter".to_string(),
        ),
        (
            b"#!bin/sh\n".to_vec(),
            "its #! interpreter 'bin/sh' is not an absolute path".to_string(),
        ),
        (
            b"#!/bin/sh\0\n".to_vec(),
            "its #! line carries a NUL".to_string(),
        ),
        (
            unterminated,
            format!("its #! line is not terminated within {SHEBANG_BOUND} bytes"),
        ),
        (
            interpreter_is_dir.into_bytes(),
            format!(
                "its #! interpreter '{}' is not an executable file",
                dir.path().display()
            ),
        ),
        (
            format!("#!{}\n", not_executable.display()).into_bytes(),
            format!(
                "its #! interpreter '{}' is not an executable file",
                not_executable.display()
            ),
        ),
    ] {
        stage_executable(&a, "dsh", &body);
        assert_eq!(
            refused(resolve_executable_in("dsh", path(&a))),
            format!(
                "the DSH layout is unreadable: {}: {reason}",
                a.join("dsh").display()
            ),
            "{body:?}"
        );
    }

    // The forms the head ADMITS: a shebang with arguments and tabs, one
    // whose whole file is shorter than the bound and unterminated, an
    // established `env` with a program that is not `node` (`env sh`, the
    // terminating control beside R4's refusals below), the measured `env
    // node` form with a `node` on the SAME search (staged in B), a script
    // whose interpreter is a script, and a native image that is not a
    // script at all — this test binary, hard-linked in, whose loader the
    // reader opens and checks.
    stage_executable(&b, "node", b"#!/bin/sh\necho v0\n");
    stage_executable(&b, "sh", b"#!/bin/sh\n");
    let interpreter_script = stage_executable(&b, "interp", b"#!/bin/sh\n");
    for (body, node) in [
        (b"#! \t/bin/sh\t-e  extra\n".to_vec(), None),
        (b"#!/bin/sh".to_vec(), None),
        (b"#!/usr/bin/env sh\n".to_vec(), None),
        (
            b"#!/usr/bin/env node\n".to_vec(),
            Some(b.join("node").canonicalize().unwrap()),
        ),
        (
            b"#!/usr/bin/env  \tnode \t\n".to_vec(),
            Some(b.join("node").canonicalize().unwrap()),
        ),
        (
            format!("#!{}\n", interpreter_script.display()).into_bytes(),
            None,
        ),
    ] {
        stage_executable(&a, "dsh", &body);
        let selected = select_in("dsh", path(&a)).unwrap();
        assert_eq!(
            selected.path,
            a.join("dsh").canonicalize().unwrap(),
            "{body:?}"
        );
        // The `node` an `env node` line selected is retained beside the
        // file; every other admitted form establishes none.
        assert_eq!(node_file(&selected.node), node, "{body:?}");
    }
    // R4 (review of run `124cca78`): an established `env` with NO
    // nonblank program. The kernel appends the launcher's path to the
    // line's arguments, so the program `env` runs is the launcher itself,
    // again and without end; this row was ADMITTED as "no node selected",
    // and the doctor that followed never returned. Bare, unterminated,
    // and every blank tail refuse by the launcher, the env interpreter
    // and the missing program — with no selection, so nothing to probe.
    for body in [
        b"#!/usr/bin/env\n".to_vec(),
        b"#!/usr/bin/env".to_vec(),
        b"#!/usr/bin/env \n".to_vec(),
        b"#!/usr/bin/env \t \t\n".to_vec(),
        b"#! \t/usr/bin/env\t\n".to_vec(),
    ] {
        stage_executable(&a, "dsh", &body);
        let missing_program = format!(
            "the DSH layout is unreadable: {}: its #! interpreter '/usr/bin/env' is the \
             platform's env utility given no nonblank program, so the program it would run is \
             the launcher itself",
            a.join("dsh").display()
        );
        assert_eq!(
            refused(select_in("dsh", path(&a))),
            missing_program,
            "{body:?}"
        );
        // Through the seam a caller probes by: no selection, so no
        // invocation — there is nothing a version probe could launch.
        let unselected = DshSeams::selected_from(
            "dsh".to_string(),
            |name| select_in(name, path(&a)),
            Some(dir.path().to_path_buf()),
        )
        .unwrap_err();
        assert_eq!(unselected.declared, "dsh");
        assert_eq!(unselected.cause.to_string(), missing_program, "{body:?}");
    }
    // A script whose INTERPRETER is the `env node` script retains that
    // innermost selection as its own.
    let via = stage_executable(&b, "via-env-node", b"#!/usr/bin/env node\n");
    stage_executable(&a, "dsh", format!("#!{}\n", via.display()).as_bytes());
    assert_eq!(
        node_file(&select_in("dsh", path(&a)).unwrap().node),
        Some(b.join("node").canonicalize().unwrap())
    );
    // An `env` argument with more than one word: the Linux kernel hands
    // it to env as one program name, which no search holds; the other
    // Unix kernels split it, and this resolver refuses the form rather
    // than guessing which half is the program.
    stage_executable(&a, "dsh", b"#!/usr/bin/env node --flag\n");
    let refusal = refused(resolve_executable_in("dsh", path(&a)));
    let enoent_flag = fs::metadata(b.join("node --flag")).unwrap_err();
    match cfg!(any(target_os = "linux", target_os = "android")) {
        true => assert_eq!(
            refusal,
            format!(
                "the DSH layout is unreadable: {}: its #! interpreter '/usr/bin/env' selects no \
                 'node --flag': the DSH layout is unreadable: 'node --flag' is not on PATH (the \
                 search ended at {}: {enoent_flag})",
                a.join("dsh").display(),
                b.join("node --flag").display()
            )
        ),
        false => assert_eq!(
            refusal,
            format!(
                "the DSH layout is unreadable: {}: its #! interpreter '/usr/bin/env' takes \
                 'node --flag', which this platform splits into more than the one `env \
                 <program>` word this establishes",
                a.join("dsh").display()
            )
        ),
    }
    fs::remove_file(a.join("dsh")).unwrap();
    fs::copy(std::env::current_exe().unwrap(), a.join(".dsh.staging")).unwrap();
    fs::rename(a.join(".dsh.staging"), a.join("dsh")).unwrap();
    let selected = select_in("dsh", path(&a)).unwrap();
    assert_eq!(
        selected.path,
        a.join("dsh").canonicalize().unwrap(),
        "a native image whose loader is established is admitted"
    );
    assert_eq!(selected.node, None, "a native image establishes no node");

    // What the head can NOT establish is a refusal by name, never an
    // admission (security hold 2026-09-20, finding 4): a truncated
    // native header, an empty file, another target's image, an `env`
    // form beyond the measured one, an `env node` with no `node` on the
    // search, and an interpreter chain that loops or runs too deep.
    // The foreign images are the formats THIS target does not load:
    // each well-formed, parsed whole, and refused for being another
    // target's rather than for a shape no run of this target reads.
    let foreign: Vec<(Vec<u8>, String)> = [
        (image::tests::synthetic_elf(None), image::Kind::Elf),
        (
            {
                let mut bytes = vec![0xcf, 0xfa, 0xed, 0xfe];
                bytes.extend_from_slice(&image::tests_cputype().to_le_bytes());
                bytes.extend_from_slice(&[0; 4]);
                bytes.extend_from_slice(&2u32.to_le_bytes());
                bytes.extend_from_slice(&[0; 16]);
                bytes
            },
            image::Kind::MachO,
        ),
        (image::tests::synthetic_pe(), image::Kind::Pe),
    ]
    .into_iter()
    .filter(|(_, kind)| *kind != image::NATIVE)
    .map(|(bytes, kind)| {
        (
            bytes,
            format!("is a {kind} image, which this target does not load"),
        )
    })
    .collect();
    assert_eq!(foreign.len(), 2, "two of the three formats are foreign");
    let looping = format!("#!{}\n", a.join("dsh").display()).into_bytes();
    let mut deep = Vec::new();
    for depth in 0..5 {
        let interpreter = match depth {
            0 => "/bin/sh".to_string(),
            _ => b.join(format!("deep{}", depth - 1)).display().to_string(),
        };
        deep.push(stage_executable(
            &b,
            &format!("deep{depth}"),
            format!("#!{interpreter}\n").as_bytes(),
        ));
    }
    let env = Path::new("/usr/bin/env").canonicalize().unwrap();
    assert!(
        env.exists(),
        "the measured interpreter exists on every supported Unix"
    );
    let mut vectors = vec![
        (
            b"\x7fELF\x02\x01\x01\0\0\0\0\0\0\0\0\0\x02\0>\0".to_vec(),
            "is not a loadable native image: an ELF header beyond the end of the file".to_string(),
        ),
        (
            Vec::new(),
            "is not a loadable native image: neither a #! script nor a native image".to_string(),
        ),
        (
            b"not a script and not an image\n".to_vec(),
            "is not a loadable native image: neither a #! script nor a native image".to_string(),
        ),
    ];
    vectors.extend(foreign);
    // `-S node` and `FOO=1 node` are both a MULTIWORD argument tail, and
    // which refusal they meet is the KERNEL's rule, chosen at compile
    // time in `env_program` and therefore here too. The Linux kernel
    // hands the whole tail to `env` as one argument, so the form check
    // reads it and names the form; Apple's kernel separates a `#!`
    // line's arguments on whitespace (XNU `exec_shell_imgact`), so the
    // resolver refuses the split before any form is read. Each is that
    // platform's own exact reason, and the one does not soften the
    // other. The `\xff` vector below carries no blank and reaches the
    // UTF-8 refusal on both.
    let multiword = |tail: &str| match cfg!(any(target_os = "linux", target_os = "android")) {
        true => format!(
            "its #! interpreter '/usr/bin/env' takes '{tail}', which is not the measured \
             `env <program>` form"
        ),
        false => format!(
            "its #! interpreter '/usr/bin/env' takes '{tail}', which this platform splits into \
             more than the one `env <program>` word this establishes"
        ),
    };
    vectors.extend([
        (b"#!/usr/bin/env -S node\n".to_vec(), multiword("-S node")),
        (
            b"#!/usr/bin/env FOO=1 node\n".to_vec(),
            multiword("FOO=1 node"),
        ),
        // The same two forms as ONE word, which no kernel splits: the
        // form check answers them on every platform, so the measured-form
        // refusal has a vector of its own wherever this suite runs.
        (
            b"#!/usr/bin/env -S\n".to_vec(),
            "its #! interpreter '/usr/bin/env' takes '-S', which is not the measured \
             `env <program>` form"
                .to_string(),
        ),
        (
            b"#!/usr/bin/env FOO=1\n".to_vec(),
            "its #! interpreter '/usr/bin/env' takes 'FOO=1', which is not the measured \
             `env <program>` form"
                .to_string(),
        ),
        (
            b"#!/usr/bin/env \xff\n".to_vec(),
            "its #! interpreter '/usr/bin/env' names a program that is not UTF-8".to_string(),
        ),
        (
            b"#!/usr/bin/env definitely-no-such-program\n".to_vec(),
            format!(
                "its #! interpreter '/usr/bin/env' selects no 'definitely-no-such-program': the \
                 DSH layout is unreadable: 'definitely-no-such-program' is not on PATH (the \
                 search ended at {}: {})",
                b.join("definitely-no-such-program").display(),
                fs::metadata(b.join("definitely-no-such-program")).unwrap_err()
            ),
        ),
        // A script naming itself is followed once — the candidate is not
        // yet a link of its own chain — and refused at the second sight.
        (
            looping,
            format!(
                "its #! interpreter '{0}' its #! interpreter chain loops at '{0}'",
                a.join("dsh").display()
            ),
        ),
        (
            format!("#!{}\n", deep[4].display()).into_bytes(),
            format!(
                "its #! interpreter '{}' its #! interpreter '{}' its #! interpreter '{}' its #! \
                 interpreter '{}' its #! interpreter chain runs deeper than 4",
                deep[4].display(),
                deep[3].display(),
                deep[2].display(),
                deep[1].display()
            ),
        ),
    ]);
    for (body, reason) in vectors {
        stage_executable(&a, "dsh", &body);
        assert_eq!(
            refused(resolve_executable_in("dsh", path(&a))),
            format!(
                "the DSH layout is unreadable: {}: {reason}",
                a.join("dsh").display()
            ),
            "{body:?}"
        );
    }
    // A chain of four interpreters is followed to its end and admitted.
    stage_executable(&a, "dsh", format!("#!{}\n", deep[2].display()).as_bytes());
    assert_eq!(
        resolve_executable_in("dsh", path(&a)).unwrap(),
        a.join("dsh").canonicalize().unwrap()
    );
    // The `env node` refusal names `node` under the search that lacks it,
    // which here is A alone: B holds the `node` shim.
    stage_executable(&a, "dsh", b"#!/usr/bin/env node\n");
    assert_eq!(
        refused(resolve_executable_in(
            "dsh",
            Some(OsString::from(a.display().to_string()))
        )),
        format!(
            "the DSH layout is unreadable: {}: its #! interpreter '/usr/bin/env' selects no \
             'node': the DSH layout is unreadable: 'node' is not on PATH (the search ended at \
             {}: {})",
            a.join("dsh").display(),
            a.join("node").display(),
            fs::metadata(a.join("node")).unwrap_err()
        )
    );

    // What the child WALKS PAST, this walks past to B: an ordinary
    // non-executable A/dsh, a directory named dsh, and an A this process
    // may not traverse. An explicit override of each is a refusal that
    // names why, because an override has nowhere to walk to.
    fs::write(a.join("dsh"), b"#!/bin/sh\ntrue\n").unwrap();
    fs::set_permissions(a.join("dsh"), fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(
        resolve_executable_in("dsh", path(&a)).unwrap(),
        real.canonicalize().unwrap()
    );
    assert_eq!(
        refused(resolve_executable_in(a.join("dsh").to_str().unwrap(), None)),
        format!(
            "the DSH layout is unreadable: {}: is not executable by this process",
            a.join("dsh").display()
        )
    );
    fs::remove_file(a.join("dsh")).unwrap();
    fs::create_dir(a.join("dsh")).unwrap();
    assert_eq!(
        resolve_executable_in("dsh", path(&a)).unwrap(),
        real.canonicalize().unwrap()
    );
    assert_eq!(
        refused(resolve_executable_in(a.join("dsh").to_str().unwrap(), None)),
        format!(
            "the DSH layout is unreadable: {}: is not a regular file",
            a.join("dsh").display()
        )
    );
    fs::remove_dir(a.join("dsh")).unwrap();

    // A candidate this process may execute but not READ: the head cannot
    // be inspected, which is a refusal, not a guess. A privileged process
    // reads anything, so the arm is asserted only where it is reachable.
    let sealed_file = dir.path().join("sealed-file");
    fs::write(&sealed_file, b"x").unwrap();
    fs::set_permissions(&sealed_file, fs::Permissions::from_mode(0o000)).unwrap();
    let privileged = fs::read(&sealed_file).is_ok();
    stage_executable(&a, "dsh", b"#!/bin/sh\ntrue\n");
    fs::set_permissions(a.join("dsh"), fs::Permissions::from_mode(0o111)).unwrap();
    let resolved = resolve_executable_in("dsh", path(&a));
    match privileged {
        true => assert_eq!(resolved.unwrap(), a.join("dsh").canonicalize().unwrap()),
        false => {
            let eacces = fs::File::open(a.join("dsh")).unwrap_err();
            assert_eq!(
                refused(resolved),
                format!(
                    "the DSH layout is unreadable: {}: cannot be read: {eacces}",
                    a.join("dsh").display()
                )
            );
        }
    }
    fs::set_permissions(a.join("dsh"), fs::Permissions::from_mode(0o755)).unwrap();

    // A PATH entry that is a regular FILE: the candidate's lookup gives
    // ENOTDIR, which `execvp` records beside ENOENT and walks past, so
    // `PATH=<file>:B` runs `B/dsh` natively — and this resolver stopped
    // on it as an unproved failure (review 2026-09-20, R1). The native
    // child is the oracle; as an explicit path the same cause is the
    // refusal, because an override has nowhere to walk to.
    let notdir = dir.path().join("file-as-dir");
    fs::write(&notdir, b"x").unwrap();
    let enotdir = fs::metadata(notdir.join("dsh")).unwrap_err();
    assert_eq!(
        errno_of(&enotdir),
        Some(rustix::io::Errno::NOTDIR),
        "{enotdir}"
    );
    let native = std::process::Command::new("dsh")
        .env("PATH", path(&notdir).unwrap())
        .output()
        .expect("the native child walks past the file-as-directory entry to B");
    assert!(native.status.success());
    assert_eq!(
        resolve_executable_in("dsh", path(&notdir)).unwrap(),
        real.canonicalize().unwrap(),
        "the resolver walks past ENOTDIR to B as the child did"
    );
    assert_eq!(
        refused(resolve_executable_in(
            notdir.join("dsh").to_str().unwrap(),
            None
        )),
        format!(
            "the DSH layout is unreadable: {}: {enotdir}",
            notdir.join("dsh").display()
        )
    );
    // A `PATH` component too long for any candidate under it. What the
    // search does is the library's own loop, not a metadata errno: glibc
    // skips a component of `path_len` bytes or more BEFORE `execve`
    // (`posix/execvpe.c` 107–119) and — because that `continue` bypasses
    // the colon increment — its NEXT iteration constructs the bare name,
    // the working directory (fourth hold, R1). So 4096 and 5000 reach
    // cwd, where the native child runs whatever sits there or walks on
    // to B, and this resolver refuses by name either way; a shorter
    // component is attempted, the kernel answers ENAMETOOLONG, and that
    // errno is not in the switch that continues (134–158), so 256, 300
    // and 4095 STOP — where the earlier resolver walked past every
    // ENAMETOOLONG and authorized an execution native lookup rejects
    // (run `09ec8d81`, R1, HIGH). The native child is the oracle at
    // every one of the six boundaries; this process's cwd holds no
    // `dsh`, so the native child runs B at 4096/5000, and the
    // cwd-bearing cells are the matrix's and the built-doctor
    // regression `terminal_path_lengths_refuse_before_doctor_probe`.
    assert!(!Path::new("dsh").exists(), "this suite's cwd holds no dsh");
    let b_only = OsString::from(b.display().to_string());
    for bytes in [255usize, 256, 300, 4095, 4096, 5000] {
        let component = "x".repeat(bytes);
        let first = OsString::from(format!("{component}:{}", b.display()));
        let native = std::process::Command::new("dsh")
            .env("PATH", &first)
            .stdout(std::process::Stdio::null())
            .status();
        let resolved = resolve_executable_in("dsh", Some(first));
        // Removing ONLY the component is the positive control, with its
        // own native oracle: the same B, from a search with nothing
        // ahead of it.
        assert!(std::process::Command::new("dsh")
            .env("PATH", &b_only)
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap()
            .success());
        assert_eq!(
            resolve_executable_in("dsh", Some(b_only.clone())).unwrap(),
            real.canonicalize().unwrap(),
            "{bytes}: the removed-component control selects B"
        );
        match LIBRARY {
            Library::Glibc => match bytes {
                255 => {
                    assert!(
                        native.unwrap().success(),
                        "{bytes}: the native child walks on to B"
                    );
                    assert_eq!(
                        resolved.unwrap(),
                        real.canonicalize().unwrap(),
                        "{bytes}: the resolver selects B as the child did"
                    );
                }
                4096 | 5000 => {
                    assert!(
                        native.unwrap().success(),
                        "{bytes}: the native child reaches cwd, finds nothing, and walks on to B"
                    );
                    assert_eq!(
                        refused(resolved),
                        format!(
                            "the DSH layout is unreadable: dsh: the platform's search would fall \
                             into the working directory: glibc skips the {bytes}-byte component \
                             and its next iteration is the empty entry it leaves the cursor on \
                             (posix/execvpe.c 118–124, 168)"
                        ),
                        "{bytes}: the resolver refuses at the cwd iteration and never advances to B"
                    );
                }
                _ => {
                    let error = native.unwrap_err();
                    assert_eq!(
                        errno_of(&error),
                        Some(rustix::io::Errno::NAMETOOLONG),
                        "{bytes}: the native search stops with ENAMETOOLONG: {error}"
                    );
                    let enametoolong = fs::metadata(Path::new(&component).join("dsh")).unwrap_err();
                    assert_eq!(
                        refused(resolved),
                        format!(
                            "the DSH layout is unreadable: {}: metadata answers {enametoolong}, \
                             on which the platform's lookup stops",
                            Path::new(&component).join("dsh").display()
                        ),
                        "{bytes}: the resolver stops by the same cause and never selects B"
                    );
                }
            },
            // Another library's boundaries are its own; the matrix
            // asserts them against that platform's native child, and
            // this table is recorded here rather than asserted from
            // glibc's numbers.
            _ => eprintln!(
                "{bytes}-byte component on {}: native {native:?}, resolver {resolved:?}",
                std::env::consts::OS
            ),
        }
    }
    // A NAME longer than NAME_MAX is not refused by glibc before the
    // search: `__strnlen (file, NAME_MAX)` caps the length it tests, so
    // the name MEETS THE KERNEL under each entry, and the entry decides
    // the answer (fourth hold, R4; the inherited unconditional NAME_MAX
    // refusal was an invention). Under a directory that exists the
    // kernel answers ENAMETOOLONG, on which the switch stops; under a
    // missing directory it answers ENOENT and under a file ENOTDIR, both
    // walked past, so a search of only those entries is exhausted with
    // that final cause and never ENAMETOOLONG. Each prefix has its own
    // native oracle.
    let overlong = "x".repeat(300);
    let spelled = a.join(&overlong);
    let native = std::process::Command::new(&overlong)
        .env("PATH", path(&a).unwrap())
        .status();
    let resolved = resolve_executable_in(&overlong, path(&a));
    let nowhere = dir.path().join("nowhere");
    let native_missing = std::process::Command::new(&overlong)
        .env("PATH", &nowhere)
        .status();
    let resolved_missing = resolve_executable_in(&overlong, Some(nowhere.clone().into_os_string()));
    let native_file = std::process::Command::new(&overlong)
        .env("PATH", &notdir)
        .status();
    let resolved_file = resolve_executable_in(&overlong, Some(notdir.clone().into_os_string()));
    // As an explicit path the kernel's own ENAMETOOLONG is the cause,
    // because an override has nowhere to walk to.
    let native_spelled = std::process::Command::new(&spelled).status();
    let resolved_spelled = resolve_executable_in(spelled.to_str().unwrap(), None);
    match LIBRARY {
        Library::Glibc => {
            assert_eq!(
                errno_of(&native.unwrap_err()),
                Some(rustix::io::Errno::NAMETOOLONG),
                "under an existing directory the kernel answers ENAMETOOLONG"
            );
            assert_eq!(
                refused(resolved),
                format!(
                    "the DSH layout is unreadable: {}: metadata answers {}, on which the \
                     platform's lookup stops",
                    spelled.display(),
                    fs::metadata(&spelled).unwrap_err()
                )
            );
            let missing = native_missing.unwrap_err();
            assert_eq!(
                errno_of(&missing),
                Some(rustix::io::Errno::NOENT),
                "under a missing directory the name is never measured: {missing}"
            );
            assert_eq!(
                refused(resolved_missing),
                format!(
                    "the DSH layout is unreadable: '{overlong}' is not on PATH (the search ended \
                     at {}: {})",
                    nowhere.join(&overlong).display(),
                    fs::metadata(nowhere.join(&overlong)).unwrap_err()
                )
            );
            let file = native_file.unwrap_err();
            assert_eq!(
                errno_of(&file),
                Some(rustix::io::Errno::NOTDIR),
                "under a file spelled as a directory the answer is ENOTDIR: {file}"
            );
            assert_eq!(
                refused(resolved_file),
                format!(
                    "the DSH layout is unreadable: '{overlong}' is not on PATH (the search ended \
                     at {}: {})",
                    notdir.join(&overlong).display(),
                    fs::metadata(notdir.join(&overlong)).unwrap_err()
                )
            );
            assert_eq!(
                errno_of(&native_spelled.unwrap_err()),
                Some(rustix::io::Errno::NAMETOOLONG)
            );
            assert_eq!(
                refused(resolved_spelled),
                format!(
                    "the DSH layout is unreadable: {}: metadata answers {}, on which the \
                     platform's lookup stops",
                    spelled.display(),
                    fs::metadata(&spelled).unwrap_err()
                )
            );
        }
        _ => eprintln!(
            "overlong name on {}: native {native:?} / {native_missing:?} / {native_file:?} / \
             {native_spelled:?}, resolver {resolved:?} / {resolved_missing:?} / \
             {resolved_file:?} / {resolved_spelled:?}",
            std::env::consts::OS
        ),
    }
    // The valid-length control: a 255-byte name that exists runs
    // natively and is selected.
    let valid = "v".repeat(255);
    let planted = stage_executable(&a, &valid, b"#!/bin/sh\ntrue\n");
    assert!(std::process::Command::new(&valid)
        .env("PATH", path(&a).unwrap())
        .status()
        .unwrap()
        .success());
    assert_eq!(
        resolve_executable_in(&valid, path(&a)).unwrap(),
        planted.canonicalize().unwrap()
    );
    fs::remove_file(&planted).unwrap();

    // A lookup failure the resolver cannot prove a child would walk past
    // is still a refusal by cause. A `PATH` entry carrying a NUL is one:
    // the platform answers before it ever asks the kernel, with no error
    // NUMBER to compare against the causes `execvp` steps over, so
    // stepping over it would be a claim this code cannot keep.
    let nul_dir = PathBuf::from(format!("{}\0", a.display()));
    let unprovable = fs::metadata(nul_dir.join("dsh")).unwrap_err();
    assert_eq!(unprovable.raw_os_error(), None, "{unprovable}");
    assert_eq!(
        refused(resolve_executable_in(
            "dsh",
            Some(OsString::from(format!(
                "{}:{}",
                nul_dir.display(),
                b.display()
            )))
        )),
        format!(
            "the DSH layout is unreadable: {}: the lookup cannot be proved: {unprovable}",
            nul_dir.join("dsh").display()
        )
    );

    // The search remembers a DENIAL as the child's `execvp` does: a
    // search that admits nothing answers EACCES when any entry was a
    // candidate this process may not execute, is not a regular file or
    // sits behind a component it may not traverse, and ENOENT only when
    // every entry was absent. The resolver named "not on PATH" for both
    // (review 2026-09-20, R9). A later executable still wins.
    fs::remove_file(&real).unwrap();
    fs::write(a.join("dsh"), b"#!/bin/sh\ntrue\n").unwrap();
    fs::set_permissions(a.join("dsh"), fs::Permissions::from_mode(0o644)).unwrap();
    let native = std::process::Command::new("dsh")
        .env("PATH", path(&a).unwrap())
        .output()
        .unwrap_err();
    assert_eq!(
        native.kind(),
        std::io::ErrorKind::PermissionDenied,
        "{native}"
    );
    assert_eq!(
        refused(resolve_executable_in("dsh", path(&a))),
        format!(
            "the DSH layout is unreadable: 'dsh' is not executable by this process on PATH: {}: \
             is not executable by this process",
            a.join("dsh").display()
        )
    );
    fs::remove_file(a.join("dsh")).unwrap();
    fs::create_dir(a.join("dsh")).unwrap();
    let native = std::process::Command::new("dsh")
        .env("PATH", path(&a).unwrap())
        .output()
        .unwrap_err();
    assert_eq!(
        native.kind(),
        std::io::ErrorKind::PermissionDenied,
        "{native}"
    );
    assert_eq!(
        refused(resolve_executable_in("dsh", path(&a))),
        format!(
            "the DSH layout is unreadable: 'dsh' is not executable by this process on PATH: {}: \
             is not a regular file",
            a.join("dsh").display()
        )
    );
    fs::remove_dir(a.join("dsh")).unwrap();
    // The first denial is the one named when several entries deny.
    fs::write(a.join("dsh"), b"#!/bin/sh\ntrue\n").unwrap();
    fs::set_permissions(a.join("dsh"), fs::Permissions::from_mode(0o644)).unwrap();
    fs::create_dir(b.join("dsh")).unwrap();
    assert_eq!(
        refused(resolve_executable_in("dsh", path(&a))),
        format!(
            "the DSH layout is unreadable: 'dsh' is not executable by this process on PATH: {}: \
             is not executable by this process",
            a.join("dsh").display()
        )
    );
    fs::remove_dir(b.join("dsh")).unwrap();
    // The later executable wins over the earlier denial, as it does for
    // the child.
    let real = stage_executable(&b, "dsh", b"#!/bin/sh\ntrue\n");
    assert!(std::process::Command::new("dsh")
        .env("PATH", path(&a).unwrap())
        .output()
        .unwrap()
        .status
        .success());
    assert_eq!(
        resolve_executable_in("dsh", path(&a)).unwrap(),
        real.canonicalize().unwrap()
    );
    fs::remove_file(a.join("dsh")).unwrap();
}

/// The Linux kernel hands `env` everything after the interpreter as ONE
/// argument, trailing spaces and tabs removed. A resolver that cut the
/// argument at its first space inspected `A/reviewed` where the child's
/// `env` searched for `reviewed extra`: with `A/reviewed extra` obstructed
/// and `B/reviewed extra` runnable, the native child ran B past the
/// obstruction while doctor admitted A's script on a file the child never
/// looked at (review 2026-09-20, R2). The program is selected as the
/// kernel spells it, so the obstruction is a refusal before any probe.
#[cfg(target_os = "linux")]
#[test]
fn an_env_argument_is_selected_as_the_kernel_hands_it_to_env() {
    use std::os::unix::fs::PermissionsExt;
    use std::process::{Command, Stdio};

    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a");
    let b = dir.path().join("b");
    fs::create_dir_all(&a).unwrap();
    fs::create_dir_all(&b).unwrap();
    let path = Some(OsString::from(format!("{}:{}", a.display(), b.display())));
    let missing = dir.path().join("no-such-interpreter");
    let enoent = fs::metadata(&missing).unwrap_err();

    // The native fact first: `#!/usr/bin/env reviewed extra` makes env
    // search for a program named `reviewed extra`, and with A's copy
    // obstructed the child runs B's.
    stage_executable(&a, "dsh", b"#!/usr/bin/env reviewed extra\n");
    stage_executable(
        &a,
        "reviewed extra",
        format!("#!{}\n", missing.display()).as_bytes(),
    );
    stage_executable(&a, "reviewed", b"#!/bin/sh\nprintf 'MARK:a-reviewed\\n'\n");
    stage_executable(
        &b,
        "reviewed extra",
        b"#!/bin/sh\nprintf 'MARK:b-reviewed-extra\\n'\n",
    );
    let native = spawn_retrying_etxtbsy(
        Command::new("dsh")
            .env("PATH", path.as_ref().unwrap())
            .stdin(Stdio::null()),
    );
    assert_eq!(
        String::from_utf8_lossy(&native.stdout).trim(),
        "MARK:b-reviewed-extra",
        "the child's env searched for the whole argument and ran B's copy"
    );
    // The resolver selects the same program under the same search, and
    // refuses at A's obstruction rather than admitting `A/dsh` on the
    // strength of an `A/reviewed` the child never looked at.
    assert_eq!(
        refused(resolve_executable_in("dsh", path.clone())),
        format!(
            "the DSH layout is unreadable: {}: its #! interpreter '/usr/bin/env' selects no \
             'reviewed extra': the DSH layout is unreadable: {}: its #! interpreter '{}' is \
             missing: {enoent}",
            a.join("dsh").display(),
            a.join("reviewed extra").display(),
            missing.display()
        )
    );
    // With A's copy gone, the argument selects B's copy and the script is
    // admitted — a program that is not `node` is selected for admission
    // and not retained as the runtime.
    fs::remove_file(a.join("reviewed extra")).unwrap();
    let selected = select_in("dsh", path.clone()).unwrap();
    assert_eq!(selected.path, a.join("dsh").canonicalize().unwrap());
    assert_eq!(selected.node, None);
    // Trailing spaces and tabs are the kernel's to remove, and a `node`
    // spelled with an option is a program named `node --flag` to env,
    // which no search holds.
    stage_executable(&b, "node", b"#!/bin/sh\necho v0\n");
    stage_executable(&a, "dsh", b"#!/usr/bin/env node \t \n");
    let selected = select_in("dsh", path.clone()).unwrap();
    assert_eq!(
        node_file(&selected.node),
        Some(b.join("node").canonicalize().unwrap())
    );
    stage_executable(&a, "dsh", b"#!/usr/bin/env node --flag\n");
    let native = spawn_retrying_etxtbsy(
        Command::new("dsh")
            .env("PATH", path.as_ref().unwrap())
            .stdin(Stdio::null()),
    );
    assert!(
        !native.status.success(),
        "env finds no program named `node --flag`"
    );
    let enoent_flag = fs::metadata(b.join("node --flag")).unwrap_err();
    assert_eq!(
        refused(resolve_executable_in("dsh", path)),
        format!(
            "the DSH layout is unreadable: {}: its #! interpreter '/usr/bin/env' selects no \
             'node --flag': the DSH layout is unreadable: 'node --flag' is not on PATH (the \
             search ended at {}: {enoent_flag})",
            a.join("dsh").display(),
            b.join("node --flag").display()
        )
    );

    // The `env` an interpreter IS, asked of the FILE and never of a name
    // (review 2026-09-20, F4; run `09ec8d81`, R3, security), and the
    // INVOCATION it is established under, which is the name `env` and
    // nothing else (review of run `124cca78`, R1, security).
    // Recognition by the spelled basename let `env-alias` carry the
    // measured form past D10's refusal; recognition by the canonical
    // basename was still recognition by name, and a COPY of `env`
    // hard-linked as `tools/uu_env` — the same bytes, no `env` name
    // anywhere — walked past it too. Establishing that same hard link
    // from the file and its own prefixed name (fourth hold, R8) was a
    // guess about WHICH utility is installed as `env`: uutils runs
    // `env` under `uu_env`, and busybox copied to `env` answers
    // `applet not found` under the very same layout — the chief's R1
    // counterexample, reproduced below on files this test owns and,
    // where this host has busybox, on the real thing. The spellings,
    // each with its own native control:
    //
    // - `/usr/bin/env` itself, a symlink NAMED `env` elsewhere and a
    //   byte-for-byte copy named `env`: the platform's env, invoked
    //   under the name `env` — the established invocation, followed
    //   into the `node` chain on every host;
    // - the `env-alias` and `link_env` symlinks and the `uu_env` and
    //   `myenv` hard links of the copy: the same file under another
    //   NAME — refused as a dispatch the resolver does not establish
    //   without executing it, whatever this host's `env` would do with
    //   the name (recorded beside the refusal, never counted);
    // - an impostor named `env`: another file under the utility's
    //   name — refused, never admitted.
    let tools = dir.path().join("tools");
    fs::create_dir_all(&tools).unwrap();
    let env_binary = PathBuf::from(ENV_REFERENCE);
    assert!(env_binary.is_file(), "this host has {ENV_REFERENCE}");
    let alias = tools.join("env-alias");
    std::os::unix::fs::symlink(&env_binary, &alias).unwrap();
    let linked = dir.path().join("linked");
    fs::create_dir_all(&linked).unwrap();
    let linked_env = linked.join("env");
    std::os::unix::fs::symlink(&env_binary, &linked_env).unwrap();
    let copied_env = tools.join("env");
    fs::copy(&env_binary, &copied_env).unwrap();
    fs::set_permissions(&copied_env, fs::Permissions::from_mode(0o755)).unwrap();
    let uu_env = tools.join("uu_env");
    fs::hard_link(&copied_env, &uu_env).unwrap();
    {
        use std::os::unix::fs::MetadataExt;
        let (copy, link, system) = (
            fs::metadata(&copied_env).unwrap(),
            fs::metadata(&uu_env).unwrap(),
            fs::metadata(&env_binary).unwrap(),
        );
        assert_eq!(
            (copy.dev(), copy.ino()),
            (link.dev(), link.ino()),
            "the copy and its hard link are one file"
        );
        assert_ne!(
            (copy.dev(), copy.ino()),
            (system.dev(), system.ino()),
            "the copy is not the system file: only its bytes say what it is"
        );
        assert_eq!(copy.len(), system.len());
    }
    let impostor_dir = dir.path().join("impostor");
    fs::create_dir_all(&impostor_dir).unwrap();
    let impostor = stage_executable(&impostor_dir, "env", b"#!/bin/sh\nexec \"$@\"\n");
    stage_executable(&a, "node", format!("#!{}\n", missing.display()).as_bytes());
    stage_executable(&b, "node", b"#!/bin/sh\nprintf 'MARK:b-node\\n'\n");
    let path = Some(OsString::from(format!("{}:{}", a.display(), b.display())));
    let obstruction = |interpreter: &Path| {
        format!(
            "the DSH layout is unreadable: {}: its #! interpreter '{}' selects no 'node': the \
             DSH layout is unreadable: {}: its #! interpreter '{}' is missing: {enoent}",
            a.join("dsh").display(),
            interpreter.display(),
            a.join("node").display(),
            missing.display()
        )
    };
    // Every other-name spelling meets ONE refusal, whatever this host's
    // `env` is installed as: the file does not say which utility it is,
    // and the utilities disagree on what another name runs. Each
    // spelling's native outcome is recorded beside its refusal — on a
    // uutils host `uu_env` runs env and `env-alias` is refused by the
    // utility itself as a name that is not its executable's; on a
    // busybox host `uu_env` runs no applet at all.
    let myenv = tools.join("myenv");
    fs::hard_link(&copied_env, &myenv).unwrap();
    let link_env = tools.join("link_env");
    std::os::unix::fs::symlink(&env_binary, &link_env).unwrap();
    let unestablished = |interpreter: &Path| {
        format!(
            "the DSH layout is unreadable: {}: its #! interpreter '{}' is the platform's env \
             utility invoked under the name '{}', a dispatch this resolver does not establish \
             without executing it",
            a.join("dsh").display(),
            interpreter.display(),
            interpreter.file_name().unwrap().to_str().unwrap()
        )
    };
    // The second sitting's R1 (review of run `124cca78`): the name
    // `env` is asked of the path the kernel invokes AND of the file that
    // runs. A symlink NAMED `env` to the copy's `uu_env` or `ls` hard
    // link is spelled `env`, is the platform's env by every byte, and on
    // this uutils host exits 1 with the utility's own `Security
    // violation` (argv[0] `env` against executable name `uu_env`) while
    // busybox installed as `env` would dispatch on `argv[0]` and run it:
    // the implementations disagree, and the resolver refuses it naming
    // the file that runs. The same symlink to the copy NAMED `env`, and
    // a hard link named `env` of the copy, run `env` everywhere and are
    // established.
    let ls = tools.join("ls");
    fs::hard_link(&copied_env, &ls).unwrap();
    let renamed = dir.path().join("renamed");
    fs::create_dir_all(&renamed).unwrap();
    let renamed_env = renamed.join("env");
    std::os::unix::fs::symlink(&uu_env, &renamed_env).unwrap();
    let renamed_ls = dir.path().join("renamed-ls");
    fs::create_dir_all(&renamed_ls).unwrap();
    let renamed_ls_env = renamed_ls.join("env");
    std::os::unix::fs::symlink(&ls, &renamed_ls_env).unwrap();
    let viacopy = dir.path().join("viacopy");
    fs::create_dir_all(&viacopy).unwrap();
    let viacopy_env = viacopy.join("env");
    std::os::unix::fs::symlink(&copied_env, &viacopy_env).unwrap();
    let hard = dir.path().join("hard");
    fs::create_dir_all(&hard).unwrap();
    let hard_env = hard.join("env");
    fs::hard_link(&copied_env, &hard_env).unwrap();
    let runs_as = |interpreter: &Path, runs: &Path| {
        format!(
            "the DSH layout is unreadable: {}: its #! interpreter '{}' is the platform's env \
             utility invoked under the name 'env' but running as the file '{}', whose own name \
             is not env, a dispatch this resolver does not establish without executing it",
            a.join("dsh").display(),
            interpreter.display(),
            fs::canonicalize(runs).unwrap().display()
        )
    };
    eprintln!(
        "R1: this host's env resolves to {}",
        fs::canonicalize(&env_binary).unwrap().display()
    );
    // The independent native control for every spelling: reaching a
    // program at all is the proof that native lookup SELECTED `A/dsh`
    // and the kernel loaded it through that spelling — nothing was
    // walked past — and where the platform's own `env` went on to run a
    // program it ran B's, never A's obstructed copy. An `env` that
    // declines to answer to another name stops there instead; that is
    // the platform's fact, recorded rather than asserted away, and it
    // does not weaken the refusal beside it.
    let native_control = |interpreter: &Path, what: &str| -> String {
        let native = spawn_retrying_etxtbsy(
            Command::new("dsh")
                .env("PATH", path.as_ref().unwrap())
                .stdin(Stdio::null()),
        );
        let said = String::from_utf8_lossy(&native.stdout).trim().to_string();
        assert!(
            !said.contains("MARK:a"),
            "{what}: the native child ran nothing of A's: {said:?}"
        );
        eprintln!(
            "R3 native control under {:?} ({what}): status {:?}, stdout {said:?}, stderr {:?}",
            interpreter.file_name().unwrap(),
            native.status,
            String::from_utf8_lossy(&native.stderr).trim()
        );
        said
    };
    // The chief's copied-and-hard-linked spelling first: it is the one
    // name recognition of either kind admits and the prefixed-name rule
    // established, and the assertion that fails under each removal
    // names it.
    for (interpreter, expected) in [
        (&uu_env, unestablished(&uu_env)),
        (&link_env, unestablished(&link_env)),
        (&alias, unestablished(&alias)),
        (&myenv, unestablished(&myenv)),
        (&renamed_env, runs_as(&renamed_env, &uu_env)),
        (&renamed_ls_env, runs_as(&renamed_ls_env, &ls)),
        (&env_binary, obstruction(&env_binary)),
        (&linked_env, obstruction(&linked_env)),
        (&copied_env, obstruction(&copied_env)),
        (&viacopy_env, obstruction(&viacopy_env)),
        (&hard_env, obstruction(&hard_env)),
    ] {
        stage_executable(
            &a,
            "dsh",
            format!("#!{} node\n", interpreter.display()).as_bytes(),
        );
        native_control(interpreter, "obstructed A/node");
        // The resolver refuses BEFORE anything is probed, under every
        // spelling: at A's obstructed `node` where the invocation is
        // established, and at the invocation itself where it is not.
        assert_eq!(
            refused(resolve_executable_in("dsh", path.clone())),
            expected,
            "{}: no spelling bypasses D10",
            interpreter.display()
        );
    }
    // The impostor: a file named `env` that is not the platform's env is
    // refused as such, whatever it would go on to do.
    stage_executable(
        &a,
        "dsh",
        format!("#!{} node\n", impostor.display()).as_bytes(),
    );
    native_control(&impostor, "impostor, obstructed A/node");
    let impostor_refusal = format!(
        "the DSH layout is unreadable: {}: its #! interpreter '{}' is named env but is not the \
         platform's env utility '{ENV_REFERENCE}'",
        a.join("dsh").display(),
        impostor.display()
    );
    assert_eq!(
        refused(resolve_executable_in("dsh", path.clone())),
        impostor_refusal
    );
    // The valid-chain positives: with A's `node` gone, every established
    // spelling selects B's `node` and RETAINS it as the runtime, with a
    // fresh native control each. The other-name spellings and the
    // impostor are still refused — the chain being whole does not
    // establish a dispatch — and their native outcome is recorded: on a
    // uutils host the same-file `uu_env` runs B's node natively and the
    // resolver still refuses it, because the same layout runs nothing
    // under busybox (below), and the file does not say which it is.
    fs::remove_file(a.join("node")).unwrap();
    let established = [
        &env_binary,
        &linked_env,
        &copied_env,
        &viacopy_env,
        &hard_env,
    ];
    let other_names = [
        (&uu_env, unestablished(&uu_env)),
        (&alias, unestablished(&alias)),
        (&myenv, unestablished(&myenv)),
        (&link_env, unestablished(&link_env)),
        (&renamed_env, runs_as(&renamed_env, &uu_env)),
        (&renamed_ls_env, runs_as(&renamed_ls_env, &ls)),
    ];
    for interpreter in established {
        stage_executable(
            &a,
            "dsh",
            format!("#!{} node\n", interpreter.display()).as_bytes(),
        );
        let said = native_control(interpreter, "valid chain");
        assert_eq!(said, "MARK:b-node", "{}", interpreter.display());
        let selected = select_in("dsh", path.clone()).unwrap();
        assert_eq!(selected.path, a.join("dsh").canonicalize().unwrap());
        assert_eq!(
            node_file(&selected.node),
            Some(b.join("node").canonicalize().unwrap()),
            "{}",
            interpreter.display()
        );
    }
    for (interpreter, expected) in other_names {
        stage_executable(
            &a,
            "dsh",
            format!("#!{} node\n", interpreter.display()).as_bytes(),
        );
        native_control(interpreter, "valid chain, unestablished invocation");
        assert_eq!(
            refused(resolve_executable_in("dsh", path.clone())),
            expected,
            "{}",
            interpreter.display()
        );
    }
    stage_executable(
        &a,
        "dsh",
        format!("#!{} node\n", impostor.display()).as_bytes(),
    );
    native_control(&impostor, "impostor, valid chain");
    assert_eq!(
        refused(resolve_executable_in("dsh", path.clone())),
        impostor_refusal
    );

    // The chief's R1 counterexample on files this test owns, through an
    // injected reference: a stand-in INSTALLED AS a file named `env`
    // that dispatches on the name it is invoked under, as busybox and a
    // single-binary coreutils do — `env` runs the platform's env, any
    // other name is an applet it does not have. `bb/uu_env` is a hard
    // link of it: the same file (`is_env` is true of both), its own
    // name, a prefixed spelling — exactly the layout the prefixed-name
    // rule established — and the native child under it runs nothing of
    // A's and nothing of B's. The resolver refuses it as the
    // unestablished dispatch it is, and follows the same file under the
    // name `env` to A's obstruction and, with that gone, to B's node.
    let bb = tools.join("bb");
    fs::create_dir_all(&bb).unwrap();
    let bb_env = stage_executable(
        &bb,
        "env",
        format!(
            "#!/bin/sh\ncase \"${{0##*/}}\" in\n  env) exec {ENV_REFERENCE} \"$@\" ;;\n  *) \
             printf 'multicall: applet %s not found\\n' \"${{0##*/}}\" >&2; exit 127 ;;\nesac\n"
        )
        .as_bytes(),
    );
    let bb_uu_env = bb.join("uu_env");
    fs::hard_link(&bb_env, &bb_uu_env).unwrap();
    let injected = |reference: &Path| Search {
        entries: path.clone().unwrap(),
        default: false,
        library: LIBRARY,
        operation: Operation::Exec,
        env_reference: reference.to_path_buf(),
    };
    stage_executable(&a, "node", format!("#!{}\n", missing.display()).as_bytes());
    for (interpreter, expected) in [
        (&bb_env, obstruction(&bb_env)),
        (&bb_uu_env, unestablished(&bb_uu_env)),
    ] {
        stage_executable(
            &a,
            "dsh",
            format!("#!{} node\n", interpreter.display()).as_bytes(),
        );
        native_control(interpreter, "installed-as-env stand-in, obstructed A/node");
        assert_eq!(
            refused(lookup_in("dsh", &injected(&bb_env), &mut Vec::new())),
            expected,
            "{}",
            interpreter.display()
        );
    }
    fs::remove_file(a.join("node")).unwrap();
    stage_executable(
        &a,
        "dsh",
        format!("#!{} node\n", bb_env.display()).as_bytes(),
    );
    assert_eq!(
        native_control(&bb_env, "installed-as-env stand-in, valid chain"),
        "MARK:b-node"
    );
    let selected = lookup_in("dsh", &injected(&bb_env), &mut Vec::new()).unwrap();
    assert_eq!(
        node_file(&selected.node),
        Some(b.join("node").canonicalize().unwrap())
    );
    stage_executable(
        &a,
        "dsh",
        format!("#!{} node\n", bb_uu_env.display()).as_bytes(),
    );
    assert_eq!(
        native_control(
            &bb_uu_env,
            "installed-as-env stand-in under uu_env, valid chain"
        ),
        "",
        "the stand-in installed as env has no applet named uu_env"
    );
    assert_eq!(
        refused(lookup_in("dsh", &injected(&bb_env), &mut Vec::new())),
        unestablished(&bb_uu_env)
    );
    // The second sitting's two cells on the stand-in. A symlink NAMED
    // `env` to `bb/uu_env`: the stand-in dispatches on the name it is
    // invoked under, so natively it runs `env` and reaches B — where
    // uutils under the same layout refused (above). The implementations
    // disagree, so the resolver refuses, naming the file that runs; the
    // native positive is recorded, never counted.
    let bb_link = tools.join("bb-link");
    fs::create_dir_all(&bb_link).unwrap();
    let bb_link_env = bb_link.join("env");
    std::os::unix::fs::symlink(&bb_uu_env, &bb_link_env).unwrap();
    stage_executable(
        &a,
        "dsh",
        format!("#!{} node\n", bb_link_env.display()).as_bytes(),
    );
    assert_eq!(
        native_control(
            &bb_link_env,
            "installed-as-env stand-in through a symlink named env to uu_env, valid chain"
        ),
        "MARK:b-node",
        "the stand-in dispatches on the name it is invoked under"
    );
    assert_eq!(
        refused(lookup_in("dsh", &injected(&bb_env), &mut Vec::new())),
        runs_as(&bb_link_env, &bb_uu_env)
    );
    // And the platform's OWN installed file under an own name that is
    // not `env` — a `multicall` the reference itself resolves to, as
    // `/usr/bin/env -> /bin/busybox` — invoked through a symlink named
    // `env`: the file that runs is the path the reference resolves to,
    // which the platform runs `env` through under this name by its own
    // construction. Established, and natively B.
    let multicall = stage_executable(
        &bb,
        "multicall",
        format!(
            "#!/bin/sh\ncase \"${{0##*/}}\" in\n  env) exec {ENV_REFERENCE} \"$@\" ;;\n  *) \
             printf 'multicall: applet %s not found\\n' \"${{0##*/}}\" >&2; exit 127 ;;\nesac\n"
        )
        .as_bytes(),
    );
    let bbm = dir.path().join("bbm");
    fs::create_dir_all(&bbm).unwrap();
    let bbm_env = bbm.join("env");
    std::os::unix::fs::symlink(&multicall, &bbm_env).unwrap();
    stage_executable(
        &a,
        "dsh",
        format!("#!{} node\n", bbm_env.display()).as_bytes(),
    );
    assert_eq!(
        native_control(
            &bbm_env,
            "installed multicall through a symlink named env, valid chain"
        ),
        "MARK:b-node"
    );
    let selected = lookup_in("dsh", &injected(&multicall), &mut Vec::new()).unwrap();
    assert_eq!(
        node_file(&selected.node),
        Some(b.join("node").canonicalize().unwrap()),
        "the platform's own installed file is established under the name env"
    );

    // The same layout on the real thing, where this host has busybox:
    // copied to `env`, hard-linked as `uu_env`. Under `env` a program
    // runs; under `uu_env` nothing does, exit 127 — the platform's fact
    // the prefixed-name rule contradicted — and the resolver's answer
    // to both is the one above. Absent busybox the cell is recorded as
    // pending, never as a pass.
    let busybox = Path::new("/usr/bin/busybox");
    if busybox.is_file() {
        let real = tools.join("busybox");
        fs::create_dir_all(&real).unwrap();
        let real_env = real.join("env");
        fs::copy(busybox, &real_env).unwrap();
        fs::set_permissions(&real_env, fs::Permissions::from_mode(0o755)).unwrap();
        let real_uu_env = real.join("uu_env");
        fs::hard_link(&real_env, &real_uu_env).unwrap();
        let echo = |interpreter: &Path| {
            spawn_retrying_etxtbsy(
                Command::new(interpreter)
                    .arg("/bin/echo")
                    .arg("ENV_SELECTED")
                    .stdin(Stdio::null()),
            )
        };
        let under_env = echo(&real_env);
        assert_eq!(
            String::from_utf8_lossy(&under_env.stdout).trim(),
            "ENV_SELECTED",
            "busybox copied to env runs env: {under_env:?}"
        );
        let under_uu_env = echo(&real_uu_env);
        assert!(
            !under_uu_env.status.success() && under_uu_env.stdout.is_empty(),
            "busybox hard-linked as uu_env runs no program: {under_uu_env:?}"
        );
        eprintln!(
            "R1 native busybox: env {:?}; uu_env {:?} {:?}",
            under_env.status,
            under_uu_env.status,
            String::from_utf8_lossy(&under_uu_env.stderr).trim()
        );
        stage_executable(
            &a,
            "dsh",
            format!("#!{} node\n", real_env.display()).as_bytes(),
        );
        assert_eq!(
            native_control(&real_env, "busybox env, valid chain"),
            "MARK:b-node"
        );
        let selected = lookup_in("dsh", &injected(&real_env), &mut Vec::new()).unwrap();
        assert_eq!(
            node_file(&selected.node),
            Some(b.join("node").canonicalize().unwrap())
        );
        stage_executable(
            &a,
            "dsh",
            format!("#!{} node\n", real_uu_env.display()).as_bytes(),
        );
        assert_eq!(
            native_control(&real_uu_env, "busybox uu_env, valid chain"),
            "",
            "busybox has no applet named uu_env"
        );
        assert_eq!(
            refused(lookup_in("dsh", &injected(&real_env), &mut Vec::new())),
            unestablished(&real_uu_env)
        );
    } else {
        eprintln!("R1 native busybox: PENDING, no /usr/bin/busybox on this host");
    }
    // And a file that is no `env` under any name, and is not named
    // `env`, is an ordinary interpreter: admitted, establishing no node.
    let other = stage_executable(&tools, "runner", b"#!/bin/sh\nexec \"$@\"\n");
    stage_executable(
        &a,
        "dsh",
        format!("#!{} node\n", other.display()).as_bytes(),
    );
    assert_eq!(select_in("dsh", path).unwrap().node, None);
}

/// The invocation is established by the file that RUNS, and an
/// interpreter whose path no longer resolves to one — its inspected
/// metadata is the reference's, its path is gone — is refused by that
/// cause, never established on the metadata alone (review of run
/// `124cca78`, R1, second sitting).
#[cfg(unix)]
#[test]
fn an_env_invocation_needs_the_file_that_runs() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let reference = dir.path().join("env");
    fs::write(&reference, b"#!/bin/sh\nexec \"$@\"\n").unwrap();
    fs::set_permissions(&reference, fs::Permissions::from_mode(0o755)).unwrap();
    let search = Search {
        entries: OsString::new(),
        default: false,
        library: LIBRARY,
        operation: Operation::Exec,
        env_reference: reference.clone(),
    };
    // The reference's own metadata, presented as a path named `env`
    // that does not exist: `is_env` is true of the device and inode,
    // the name is `env`, and the file that runs cannot be resolved.
    let gone = dir.path().join("gone").join("env");
    let error = fs::canonicalize(&gone).unwrap_err();
    assert_eq!(
        env_program(
            &gone,
            &fs::metadata(&reference).unwrap(),
            b" node",
            &search,
            &mut Vec::new()
        ),
        Err(format!("cannot be resolved to the file that runs: {error}"))
    );
}

/// `env` identity is a fact about FILES: device and inode against the
/// reference, or equal length and equal bytes for a copy, compared on
/// the file that is open and checked to be the file that was inspected.
/// Every arm is a plain test with an injected reference (design D10,
/// third hold, R3).
#[cfg(unix)]
#[test]
fn env_identity_is_the_file_and_never_a_name() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let reference = dir.path().join("reference-env");
    fs::write(&reference, b"#!/bin/sh\necho reference\n").unwrap();
    let inspected = |path: &Path| fs::metadata(path).unwrap();

    // The reference itself, and a symlink to it: one device and inode.
    assert_eq!(
        is_env(&reference, &inspected(&reference), &reference),
        Ok(true)
    );
    let symlink = dir.path().join("env");
    std::os::unix::fs::symlink(&reference, &symlink).unwrap();
    assert_eq!(is_env(&symlink, &inspected(&symlink), &reference), Ok(true));
    // A copy: another inode, the same length, the same bytes.
    let copy = dir.path().join("copy");
    fs::copy(&reference, &copy).unwrap();
    assert_eq!(is_env(&copy, &inspected(&copy), &reference), Ok(true));
    // Another length is another file, with no bytes read.
    let shorter = dir.path().join("shorter");
    fs::write(&shorter, b"#!/bin/sh\n").unwrap();
    assert_eq!(
        is_env(&shorter, &inspected(&shorter), &reference),
        Ok(false)
    );
    // The same length and different bytes is another file.
    let same_length = dir.path().join("same-length");
    fs::write(&same_length, b"#!/bin/sh\necho REFERENCE\n").unwrap();
    assert_eq!(
        is_env(&same_length, &inspected(&same_length), &reference),
        Ok(false)
    );
    // A reference that cannot be inspected establishes nothing.
    let absent = dir.path().join("absent-env");
    let enoent = fs::metadata(&absent).unwrap_err();
    assert_eq!(
        is_env(&copy, &inspected(&copy), &absent),
        Err(format!(
            "cannot be compared with the platform's env '{}', which cannot be inspected: {enoent}",
            absent.display()
        ))
    );
    // A candidate that cannot be opened for comparison is a refusal,
    // not a guess: the metadata is a same-length file's, the path is
    // nothing.
    let gone = dir.path().join("gone");
    let enoent = fs::File::open(&gone).unwrap_err();
    assert_eq!(
        is_env(&gone, &inspected(&copy), &reference),
        Err(format!("cannot be read: {enoent}"))
    );
    // The file that is open must be the file that was inspected.
    assert_eq!(
        is_env(&same_length, &inspected(&copy), &reference),
        Err("changed between its inspection and its reading".to_string())
    );
    // A reference that can be inspected but not read, where this
    // process is not privileged.
    let sealed = dir.path().join("sealed-env");
    fs::copy(&reference, &sealed).unwrap();
    fs::set_permissions(&sealed, fs::Permissions::from_mode(0o000)).unwrap();
    let result = is_env(&copy, &inspected(&copy), &sealed);
    match fs::File::open(&sealed) {
        Ok(_) => assert_eq!(result, Ok(true), "a privileged process reads anything"),
        Err(eacces) => assert_eq!(
            result,
            Err(format!(
                "cannot be compared with the platform's env '{}', which cannot be read: {eacces}",
                sealed.display()
            ))
        ),
    }
    fs::set_permissions(&sealed, fs::Permissions::from_mode(0o644)).unwrap();

    // The comparison itself: equal, unequal, and each side ending early.
    let bytes = vec![7u8; 20_000];
    let mut other = bytes.clone();
    other[19_999] = 8;
    assert_eq!(
        same_bytes(&mut &bytes[..], &mut &bytes[..], 20_000),
        Ok(true)
    );
    assert_eq!(
        same_bytes(&mut &bytes[..], &mut &other[..], 20_000),
        Ok(false)
    );
    assert_eq!(same_bytes(&mut &bytes[..], &mut &bytes[..], 0), Ok(true));
    let short: &[u8] = &bytes[..100];
    assert_eq!(
        same_bytes(&mut &short[..], &mut &bytes[..], 20_000),
        Err("cannot be read whole: failed to fill whole buffer".to_string())
    );
    assert_eq!(
        same_bytes(&mut &bytes[..], &mut &short[..], 20_000),
        Err("cannot be compared whole: failed to fill whole buffer".to_string())
    );

    // Through the search: a reference the search cannot inspect makes
    // every `env` interpreter a refusal by that cause, before any
    // probe, rather than an `Ok(None)` admission.
    let a = dir.path().join("a");
    fs::create_dir_all(&a).unwrap();
    stage_executable(&a, "dsh", b"#!/usr/bin/env node\n");
    let search = Search {
        entries: OsString::from(a.display().to_string()),
        default: false,
        library: LIBRARY,
        operation: Operation::Exec,
        env_reference: absent.clone(),
    };
    let enoent = fs::metadata(&absent).unwrap_err();
    assert_eq!(
        refused(lookup_in("dsh", &search, &mut Vec::new())),
        format!(
            "the DSH layout is unreadable: {}: its #! interpreter '/usr/bin/env' cannot be \
             compared with the platform's env '{}', which cannot be inspected: {enoent}",
            a.join("dsh").display(),
            absent.display()
        )
    );
}

/// The lookup rule is each library's own switch, arm by arm, with its
/// source cited on the arm — and never one library's switch applied to
/// another (run `09ec8d81`, R1/R2). These are table checks of the
/// translated source, supplemental to the native matrix and not native
/// evidence for any platform this suite does not run on.
#[cfg(unix)]
#[test]
fn the_lookup_rule_is_each_librarys_own_switch_arm_by_arm() {
    use rustix::io::Errno;

    // glibc `posix/execvpe.c` 136–158: the literal continue-set, EACCES
    // remembered, and everything else terminal — ENAMETOOLONG, ELOOP,
    // EIO, EINVAL, ENOEXEC, ETXTBSY, EPERM included.
    assert_eq!(
        step(Library::Glibc, Errno::ACCESS),
        Ok(Step::Continue { denied: true })
    );
    for errno in [
        Errno::NOENT,
        Errno::STALE,
        Errno::NOTDIR,
        Errno::NODEV,
        Errno::TIMEDOUT,
    ] {
        assert_eq!(
            step(Library::Glibc, errno),
            Ok(Step::Continue { denied: false }),
            "{errno}"
        );
    }
    for errno in [
        Errno::NAMETOOLONG,
        Errno::LOOP,
        Errno::IO,
        Errno::INVAL,
        Errno::NOEXEC,
        Errno::TXTBSY,
        Errno::PERM,
        Errno::NOMEM,
    ] {
        assert_eq!(step(Library::Glibc, errno), Ok(Step::Stop), "{errno}");
    }
    // musl `src/process/execvp.c`: EACCES remembered, ENOENT and
    // ENOTDIR continue, and nothing else — not even ESTALE.
    assert_eq!(
        step(Library::Musl, Errno::ACCESS),
        Ok(Step::Continue { denied: true })
    );
    for errno in [Errno::NOENT, Errno::NOTDIR] {
        assert_eq!(
            step(Library::Musl, errno),
            Ok(Step::Continue { denied: false }),
            "{errno}"
        );
    }
    for errno in [Errno::STALE, Errno::NAMETOOLONG, Errno::LOOP, Errno::IO] {
        assert_eq!(step(Library::Musl, errno), Ok(Step::Stop), "{errno}");
    }
    // Apple `sys/posix_spawn.c`: ELOOP, ENAMETOOLONG, ENOENT and ENOTDIR
    // continue, EACCES is remembered, and an arm this seat did not pin
    // is a limitation rather than either guess.
    assert_eq!(
        step(Library::Apple, Errno::ACCESS),
        Ok(Step::Continue { denied: true })
    );
    for errno in [Errno::LOOP, Errno::NAMETOOLONG, Errno::NOENT, Errno::NOTDIR] {
        assert_eq!(
            step(Library::Apple, errno),
            Ok(Step::Continue { denied: false }),
            "{errno}"
        );
    }
    for errno in [Errno::IO, Errno::INVAL, Errno::STALE, Errno::NOEXEC] {
        assert_eq!(
            step(Library::Apple, errno),
            Err("that arm of Apple's posix_spawnp switch is not pinned by this resolver"),
            "{errno}"
        );
    }
    assert_eq!(
        step(Library::Unestablished, Errno::NOENT),
        Err("this target's native program lookup rule is not established")
    );

    // The candidate SEQUENCE each walk constructs over the exact PATH
    // bytes, before any attempt — the fourth hold's whole subject: the
    // sequence, not a per-component classification. Every candidate is
    // recorded with its bytes and provenance; a construction stop is
    // recorded as such.
    let sequence = |library: Library, operation: Operation, path: &str, file: &str| {
        let mut seen: Vec<(String, Origin)> = Vec::new();
        let ended: Option<()> = walk_search(
            library,
            operation,
            path.as_bytes(),
            file.as_bytes(),
            |step| {
                match step {
                    Try::Candidate { bytes, origin } => {
                        seen.push((String::from_utf8(bytes).unwrap(), origin));
                    }
                    Try::ConstructionStop { bytes } => seen.push((
                        format!("STOP {}", String::from_utf8(bytes).unwrap()),
                        Origin::Entry,
                    )),
                }
                std::ops::ControlFlow::Continue(())
            },
        );
        assert!(
            ended.is_none(),
            "a walk that is never broken ends on its own"
        );
        seen
    };
    let entry = |bytes: &str| (bytes.to_string(), Origin::Entry);
    let empty = |bytes: &str, index: usize| (bytes.to_string(), Origin::EmptyEntry { index });
    let implicit = |skipped: usize| ("dsh".to_string(), Origin::AfterOversizedSkip { skipped });
    let x = |bytes: usize| "x".repeat(bytes);
    for operation in [Operation::Spawn, Operation::Exec] {
        // glibc (`posix/execvpe.c` 107–126, 160–168): ordinary entries;
        // every empty entry as the bare name at its position; the extra
        // slash after an entry that already ends in one; a component of
        // `path_len` bytes or more skipped with the cursor left on the
        // colon, so the NEXT candidate is the bare name — the implicit
        // working-directory iteration; and a final oversized component
        // breaking with nothing constructed.
        let glibc = |path: &str| sequence(Library::Glibc, operation, path, "dsh");
        assert_eq!(glibc("A:B"), vec![entry("A/dsh"), entry("B/dsh")]);
        assert_eq!(
            glibc("A::B"),
            vec![entry("A/dsh"), empty("dsh", 1), entry("B/dsh")]
        );
        assert_eq!(glibc("A:"), vec![entry("A/dsh"), empty("dsh", 1)]);
        assert_eq!(glibc(":B"), vec![empty("dsh", 0), entry("B/dsh")]);
        assert_eq!(glibc(""), vec![empty("dsh", 0)]);
        assert_eq!(glibc("A/"), vec![entry("A//dsh")]);
        assert_eq!(
            glibc(&format!("{}:B", x(4095))),
            vec![entry(&format!("{}/dsh", x(4095))), entry("B/dsh")]
        );
        assert_eq!(
            glibc(&format!("{}:B", x(4096))),
            vec![implicit(4096), entry("B/dsh")]
        );
        assert_eq!(
            glibc(&format!("{}:B", x(5000))),
            vec![implicit(5000), entry("B/dsh")]
        );
        assert_eq!(glibc(&x(4096)), vec![]);
        assert_eq!(glibc(&format!("A:{}", x(4096))), vec![entry("A/dsh")]);
        assert_eq!(
            glibc(&format!("A:{}:B", x(5000))),
            vec![entry("A/dsh"), implicit(5000), entry("B/dsh")]
        );
        // The implicit candidate and an explicit empty entry after it
        // are two candidates, each with its own provenance.
        assert_eq!(
            glibc(&format!("{}::B", x(4096))),
            vec![implicit(4096), empty("dsh", 1), entry("B/dsh")]
        );
        // A short PATH is never skipped from: `path_len` is the whole
        // variable's length plus one, so no component can reach it.
        assert_eq!(
            glibc(&format!("{}:B", x(100))),
            vec![entry(&format!("{}/dsh", x(100))), entry("B/dsh")]
        );

        // musl (`src/process/execvp.c`): the same empty-entry meaning,
        // and a skipped component whose `continue` has already stepped
        // past the colon — no implicit iteration, straight to B.
        let musl = |path: &str| sequence(Library::Musl, operation, path, "dsh");
        assert_eq!(
            musl("A::B"),
            vec![entry("A/dsh"), empty("dsh", 1), entry("B/dsh")]
        );
        assert_eq!(musl(""), vec![empty("dsh", 0)]);
        assert_eq!(musl(&format!("{}:B", x(4096))), vec![entry("B/dsh")]);
        assert_eq!(musl(&x(4096)), vec![]);
        assert_eq!(musl("A/"), vec![entry("A//dsh")]);
    }
    // Apple: `strsep` tokens, an empty token spelled `.`, the same extra
    // slash — and the one branch the two operations take apart: a
    // candidate longer than the 1,024-byte buffer is skipped by
    // `execvP` and stops `posix_spawnp`.
    let exec = |path: &str, file: &str| sequence(Library::Apple, Operation::Exec, path, file);
    let spawn = |path: &str, file: &str| sequence(Library::Apple, Operation::Spawn, path, file);
    for operation in [Operation::Exec, Operation::Spawn] {
        let apple = |path: &str, file: &str| sequence(Library::Apple, operation, path, file);
        assert_eq!(
            apple("A::B", "dsh"),
            vec![entry("A/dsh"), empty("./dsh", 1), entry("B/dsh")]
        );
        assert_eq!(apple("", "dsh"), vec![empty("./dsh", 0)]);
        assert_eq!(apple(":B", "dsh"), vec![empty("./dsh", 0), entry("B/dsh")]);
        assert_eq!(apple("A:", "dsh"), vec![entry("A/dsh"), empty("./dsh", 1)]);
        assert_eq!(apple("A/", "dsh"), vec![entry("A//dsh")]);
        // `lp + ln + 2 > 1024`: a directory of 1,019 bytes with a 3-byte
        // name fills the buffer exactly and is attempted.
        assert_eq!(
            apple(&x(1019), "dsh"),
            vec![entry(&format!("{}/dsh", x(1019)))]
        );
        assert_eq!(
            apple("", &"n".repeat(1021)),
            vec![empty(&format!("./{}", "n".repeat(1021)), 0)]
        );
    }
    assert_eq!(exec(&format!("{}:B", x(1100)), "dsh"), vec![entry("B/dsh")]);
    assert_eq!(exec(&format!("{}:B", x(1020)), "dsh"), vec![entry("B/dsh")]);
    assert_eq!(exec("", &"n".repeat(1022)), vec![]);
    assert_eq!(
        spawn(&format!("{}:B", x(1100)), "dsh"),
        vec![(format!("STOP {}/dsh", x(1100)), Origin::Entry)]
    );
    assert_eq!(
        spawn("", &"n".repeat(1022)),
        vec![(format!("STOP ./{}", "n".repeat(1022)), Origin::Entry)]
    );
    // An unestablished library constructs nothing.
    assert_eq!(
        sequence(Library::Unestablished, Operation::Spawn, "A:B", "dsh"),
        vec![]
    );

    // The name, before any search: musl refuses more than NAME_MAX
    // with ENAMETOOLONG (`src/process/execvp.c`); glibc does NOT — its
    // `__strnlen (file, NAME_MAX)` caps what its own check measures, so
    // the name meets the kernel under each entry (fourth hold, R4);
    // Apple has no such check; an unestablished library searches for
    // nothing.
    let long = "n".repeat(256);
    let refusal = "is 256 bytes long, more than the 255 bytes NAME_MAX allows a searched name, \
                   which the platform's lookup refuses with ENAMETOOLONG before searching"
        .to_string();
    assert_eq!(admit_program_name(Library::Musl, &long), Err(refusal));
    assert_eq!(admit_program_name(Library::Musl, &"n".repeat(255)), Ok(()));
    assert_eq!(admit_program_name(Library::Glibc, &long), Ok(()));
    assert_eq!(admit_program_name(Library::Apple, &long), Ok(()));
    assert_eq!(
        admit_program_name(Library::Unestablished, "dsh"),
        Err(
            "cannot be searched for: this target's native program lookup rule is not \
             established"
                .to_string()
        )
    );

    // One lookup failure through the switch, by operation: the
    // continuation keeps the operation's own words, the stop names the
    // operation and the cause, and a limitation names itself.
    let candidate = Path::new("/nowhere/dsh");
    let describe = |candidate: Candidate| match candidate {
        Candidate::Admitted(_) => "admitted".to_string(),
        Candidate::Passed { why, denied } => format!("passed ({denied}): {why}"),
        Candidate::Refused(error) => format!("refused: {error}"),
    };
    assert_eq!(
        describe(lookup_failure(
            candidate,
            Library::Glibc,
            "metadata",
            Errno::NOENT,
            Position::Searched
        )),
        "passed (false): No such file or directory (os error 2)"
    );
    assert_eq!(
        describe(lookup_failure(
            candidate,
            Library::Glibc,
            "access",
            Errno::ACCESS,
            Position::Searched
        )),
        "passed (true): is not executable by this process"
    );
    assert_eq!(
        describe(lookup_failure(
            candidate,
            Library::Glibc,
            "metadata",
            Errno::ACCESS,
            Position::Searched
        )),
        "passed (true): Permission denied (os error 13)"
    );
    // ELOOP is the one errno in this table whose NUMBER is the host's
    // rather than the constant's: 40 under Linux, 62 under Darwin, while
    // ENOENT, EACCES, EIO and ENOTDIR agree across both. The switch under
    // test is the LIBRARY's, chosen by the table's own `Library` row, and
    // the rendering is `io::Error`'s, which is the running host's — so
    // the expectation is built from the same errno constant the call is
    // given and the assertion stays the whole reason, not a prefix.
    let rendered = |errno: Errno| std::io::Error::from_raw_os_error(errno.raw_os_error());
    assert_eq!(
        describe(lookup_failure(
            candidate,
            Library::Glibc,
            "metadata",
            Errno::LOOP,
            Position::Searched
        )),
        format!(
            "refused: the DSH layout is unreadable: /nowhere/dsh: a symlink loop stops the \
             lookup: {}",
            rendered(Errno::LOOP)
        )
    );
    assert_eq!(
        describe(lookup_failure(
            candidate,
            Library::Glibc,
            "access",
            Errno::IO,
            Position::Searched
        )),
        "refused: the DSH layout is unreadable: /nowhere/dsh: access answers Input/output error \
         (os error 5), on which the platform's lookup stops"
    );
    assert_eq!(
        describe(lookup_failure(
            candidate,
            Library::Apple,
            "metadata",
            Errno::LOOP,
            Position::Searched
        )),
        format!("passed (false): {}", rendered(Errno::LOOP))
    );
    // The same Apple arm at a DIRECT name: the continuation has no next
    // entry to continue to, so the loop is the stop it is and carries
    // the name every library's terminal loop carries. The libraries
    // whose search already stops on ELOOP answer this identically,
    // which is the point — one rule, one reason, every platform.
    for library in [Library::Apple, Library::Glibc, Library::Musl] {
        assert_eq!(
            describe(lookup_failure(
                candidate,
                library,
                "metadata",
                Errno::LOOP,
                Position::Direct
            )),
            format!(
                "refused: the DSH layout is unreadable: /nowhere/dsh: a symlink loop stops the \
                 lookup: {}",
                rendered(Errno::LOOP)
            ),
            "{library:?} names a direct name's terminal loop"
        );
    }
    // ENAMETOOLONG is the OTHER errno the searching libraries part
    // company on — glibc and musl stop, Apple's switch continues — so it
    // is the other one whose direct-name refusal read one way on Linux
    // and another on macOS until this repair. A direct name has no next
    // entry on any arm, so all three name the stop, in the one wording
    // Linux has always carried.
    for library in [Library::Apple, Library::Glibc, Library::Musl] {
        assert_eq!(
            describe(lookup_failure(
                candidate,
                library,
                "metadata",
                Errno::NAMETOOLONG,
                Position::Direct
            )),
            format!(
                "refused: the DSH layout is unreadable: /nowhere/dsh: metadata answers {}, on \
                 which the platform's lookup stops",
                rendered(Errno::NAMETOOLONG)
            ),
            "{library:?} names a direct name's terminal ENAMETOOLONG"
        );
    }
    // The SEARCHED Apple arm does not move with it: a kernel
    // ENAMETOOLONG under one entry is a continuation there, exactly as
    // `sys/posix_spawn.c`'s switch has it, and only the direct position
    // turns that continuation into the stop it is.
    assert_eq!(
        describe(lookup_failure(
            candidate,
            Library::Apple,
            "metadata",
            Errno::NAMETOOLONG,
            Position::Searched
        )),
        format!("passed (false): {}", rendered(Errno::NAMETOOLONG))
    );
    // And nothing else about a direct name's refusal moves: a continued
    // errno keeps the operation's own answer, and a stop keeps its own
    // words, exactly as at a searched candidate.
    //
    // This is the AUDIT the differential matrix's errno-named branches
    // rest on (PR #311, 2026-09-21): at a direct name no library's
    // switch runs, so every arm must answer alike, and each errno the
    // matrix names is asserted so arm by arm rather than described.
    // ENOENT, ENOTDIR and EACCES already agreed — all three sit in every
    // library's continue-set — and ELOOP and ENAMETOOLONG, the two the
    // searching libraries part company on, were the two that did not.
    for (operation, errno, answer) in [
        (
            "metadata",
            Errno::NOENT,
            "passed (false): No such file or directory (os error 2)",
        ),
        (
            "metadata",
            Errno::NOTDIR,
            "passed (false): Not a directory (os error 20)",
        ),
        (
            "access",
            Errno::ACCESS,
            "passed (true): is not executable by this process",
        ),
        (
            "metadata",
            Errno::ACCESS,
            "passed (true): Permission denied (os error 13)",
        ),
    ] {
        for library in [Library::Apple, Library::Glibc, Library::Musl] {
            assert_eq!(
                describe(lookup_failure(
                    candidate,
                    library,
                    operation,
                    errno,
                    Position::Direct
                )),
                answer,
                "{library:?} answers a direct name's {errno} as every other arm does"
            );
        }
    }
    // The ONE arm that audit found still library-dependent at a direct
    // name, recorded rather than guessed at: an errno OUTSIDE Apple's
    // pinned switch — EIO here — renders as that limitation, where
    // glibc and musl name the stop a direct name always is. No cell of
    // the differential matrix asserts it and no Linux string carries
    // it, so it is left as named pending work (delivery account, 8.8).
    assert_eq!(
        describe(lookup_failure(
            candidate,
            Library::Apple,
            "access",
            Errno::IO,
            Position::Direct
        )),
        "refused: the DSH layout is unreadable: /nowhere/dsh: access answers Input/output error \
         (os error 5), and that arm of Apple's posix_spawnp switch is not pinned by this resolver"
    );
    assert_eq!(
        describe(lookup_failure(
            candidate,
            Library::Glibc,
            "access",
            Errno::ACCESS,
            Position::Direct
        )),
        "passed (true): is not executable by this process"
    );
    assert_eq!(
        describe(lookup_failure(
            candidate,
            Library::Glibc,
            "access",
            Errno::IO,
            Position::Direct
        )),
        "refused: the DSH layout is unreadable: /nowhere/dsh: access answers Input/output error \
         (os error 5), on which the platform's lookup stops"
    );
    assert_eq!(
        describe(lookup_failure(
            candidate,
            Library::Apple,
            "metadata",
            Errno::IO,
            Position::Searched
        )),
        "refused: the DSH layout is unreadable: /nowhere/dsh: metadata answers Input/output \
         error (os error 5), and that arm of Apple's posix_spawnp switch is not pinned by this \
         resolver"
    );
    assert_eq!(
        describe(lookup_failure(
            candidate,
            Library::Unestablished,
            "metadata",
            Errno::NOENT,
            Position::Searched
        )),
        "refused: the DSH layout is unreadable: /nowhere/dsh: metadata answers No such file or \
         directory (os error 2), and this target's native program lookup rule is not established"
    );

    // The whole search under another library's rule, driven on this
    // host: Apple's construction stop under `posix_spawnp` and its
    // skip under `execvP` for the same 1,100-byte component, Apple's
    // continuation past a metadata ENAMETOOLONG the kernel answers for
    // a 600-byte component, musl's NAME_MAX refusal and its straight
    // advance past a skipped component, and the unestablished library's
    // refusal of every search.
    let dir = tempfile::tempdir().unwrap();
    let b = dir.path().join("b");
    fs::create_dir_all(&b).unwrap();
    let real = stage_executable(&b, "dsh", b"#!/bin/sh\ntrue\n");
    let under = |library: Library, operation: Operation, first: &str| Search {
        entries: OsString::from(format!("{first}:{}", b.display())),
        default: false,
        library,
        operation,
        env_reference: PathBuf::from(ENV_REFERENCE),
    };
    let stopped = "x".repeat(1100);
    assert_eq!(
        refused(lookup_in(
            "dsh",
            &under(Library::Apple, Operation::Spawn, &stopped),
            &mut Vec::new()
        )),
        format!(
            "the DSH layout is unreadable: {}: the platform's lookup stops before attempting a \
             candidate longer than the 1024 bytes it builds one in (ENAMETOOLONG)",
            Path::new(&stopped).join("dsh").display()
        )
    );
    assert_eq!(
        lookup_in(
            "dsh",
            &under(Library::Apple, Operation::Exec, &stopped),
            &mut Vec::new()
        )
        .unwrap()
        .path,
        real.canonicalize().unwrap(),
        "execvP warns about the overlong component and continues to B"
    );
    let walked = "x".repeat(600);
    assert_eq!(
        lookup_in(
            "dsh",
            &under(Library::Apple, Operation::Spawn, &walked),
            &mut Vec::new()
        )
        .unwrap()
        .path,
        real.canonicalize().unwrap(),
        "Apple's switch continues past the kernel's ENAMETOOLONG"
    );
    assert_eq!(
        lookup_in(
            "dsh",
            &under(Library::Musl, Operation::Exec, &"x".repeat(4096)),
            &mut Vec::new()
        )
        .unwrap()
        .path,
        real.canonicalize().unwrap(),
        "musl's skip steps past the colon and reaches B with no cwd iteration"
    );
    assert_eq!(
        refused(lookup_in(
            &"m".repeat(300),
            &under(Library::Musl, Operation::Exec, &walked),
            &mut Vec::new()
        )),
        format!(
            "the DSH layout is unreadable: '{}' is 300 bytes long, more than the 255 bytes \
             NAME_MAX allows a searched name, which the platform's lookup refuses with \
             ENAMETOOLONG before searching",
            "m".repeat(300)
        )
    );
    assert_eq!(
        refused(lookup_in(
            "dsh",
            &under(Library::Unestablished, Operation::Spawn, "/nowhere"),
            &mut Vec::new()
        )),
        "the DSH layout is unreadable: 'dsh' cannot be searched for: this target's native \
         program lookup rule is not established"
    );
    // And the running target's own rule is the compiled one, under the
    // operation the caller named; `env`'s nested search is always the
    // exec form.
    let captured = Search::capture(
        Some(OsString::from(b.display().to_string())),
        Operation::Spawn,
    )
    .unwrap();
    assert_eq!(captured.library, LIBRARY);
    assert_eq!(captured.operation, Operation::Spawn);
    assert_eq!(captured.for_env().operation, Operation::Exec);
    assert_eq!(captured.for_env().entries, captured.entries);
}

/// A DIRECT name whose own path is a symlink loop is refused by the
/// NAMED cause on every library's arm, because a direct name has no
/// next candidate for a continuation to reach.
///
/// The platform-qualified ELOOP rule is a rule about a SEARCH: glibc's
/// `posix/execvpe.c` stops on ELOOP, Apple's `sys/posix_spawn.c` breaks
/// to the next entry (D10, controller correction 2026-09-20). Read as a
/// rule about a CANDIDATE instead, it made the Apple arm render a direct
/// `./dsh` that is a self-symlink as the bare errno while glibc and musl
/// named the loop — one refusal reported two ways, and the matrix's
/// terminal-ELOOP naming failed on macOS for that reason alone (PR
/// #311's macOS leg, 2026-09-21, cell n1-l4).
///
/// The Apple arm is INJECTED here, so the branch is proved on this host.
/// That is not native macOS evidence: the matrix's own run on that host
/// supplies it, and this test stands beside the per-library lookup table
/// as a translated-source check.
#[cfg(unix)]
#[test]
fn a_direct_names_symlink_loop_is_named_on_every_librarys_arm() {
    let root = FixtureRoot::new();
    let a = root.path().join("a");
    let b = root.path().join("b");
    fs::create_dir_all(&a).unwrap();
    fs::create_dir_all(&b).unwrap();
    let real = stage_executable(&b, "dsh", b"#!/bin/sh\ntrue\n");
    // `a/dsh -> dsh`: the candidate's last component resolves to itself,
    // so the loop is the kernel's answer to this fixture and not a
    // condition the resolver invented.
    let looped = a.join("dsh");
    std::os::unix::fs::symlink("dsh", &looped).unwrap();
    let native = fs::metadata(&looped).unwrap_err();
    assert_eq!(
        errno_of(&native),
        Some(rustix::io::Errno::LOOP),
        "the fixture is a loop this host's kernel answers ELOOP for: {native}"
    );

    let search = |library: Library, operation: Operation| Search {
        entries: OsString::from(format!("{}:{}", a.display(), b.display())),
        default: false,
        library,
        operation,
        env_reference: PathBuf::from(ENV_REFERENCE),
    };
    // ELOOP's NUMBER is the host's — 40 under Linux, 62 under Darwin —
    // so the expected rendering is built from the same constant the
    // kernel just answered rather than from Darwin's integer spelled out
    // in a test that runs on Linux.
    let named = format!(
        "the DSH layout is unreadable: {}: a symlink loop stops the lookup: {}",
        looped.display(),
        std::io::Error::from_raw_os_error(rustix::io::Errno::LOOP.raw_os_error())
    );
    let direct = looped.display().to_string();
    for library in [Library::Apple, Library::Glibc, Library::Musl, LIBRARY] {
        for operation in [Operation::Exec, Operation::Spawn] {
            assert_eq!(
                refused(lookup_in(
                    &direct,
                    &search(library, operation),
                    &mut Vec::new()
                )),
                named,
                "{library:?} under {operation:?} names a direct name's terminal loop"
            );
        }
    }

    // And the SEARCHED controls do not move: Apple's switch still walks
    // past the loop at A to the runnable B, and glibc's still stops
    // there, by the same named cause the direct name now carries.
    for operation in [Operation::Exec, Operation::Spawn] {
        assert_eq!(
            lookup_in("dsh", &search(Library::Apple, operation), &mut Vec::new())
                .unwrap()
                .path,
            real.canonicalize().unwrap(),
            "Apple's search continues past ELOOP to the next entry"
        );
        assert_eq!(
            refused(lookup_in(
                "dsh",
                &search(Library::Glibc, operation),
                &mut Vec::new()
            )),
            named,
            "glibc's search stops at the loop"
        );
    }
}

/// The native matrix's OVERSIZED-COMPONENT cells, driven on this host
/// under the Apple arm — the shapes whose macOS answer the matrix used
/// to predict from glibc's rule and therefore discovered one cell per
/// 25-minute CI pass (PR #311, passes 1–3).
///
/// The question the third pass asked was which refusal Apple owes cell
/// n0-l10 — `PATH` a single 5,000-byte component, name `dsh`, native
/// answering ENAMETOOLONG. The per-library table answers it: Apple sizes
/// EVERY candidate against a 1,024-byte buffer before it is built
/// (`lp + ln + 2 > sizeof(buf)`, `sys/posix_spawn.c` and
/// `gen/FreeBSD/exec.c`, design D10), so a 5,004-byte candidate is never
/// constructed and never handed to `execve`; `posix_spawnp` answers
/// `err = ENAMETOOLONG` there and `execvP` warns and takes the next
/// token. The kernel cannot be the author of that errno, because the
/// candidate whose name it would have measured does not exist — and the
/// two refusals differ in exactly that: the pre-attempt bound says the
/// lookup stopped BEFORE a candidate, the kernel's stop says a candidate
/// was measured. Apple's arm owes the pre-attempt bound, which is what
/// production already answers; it was the matrix that carried glibc's
/// rule onto the Apple arm.
///
/// glibc's own answers to the same four spellings stand beside them, so
/// the two rules are read together rather than one being assumed to be
/// the other.
#[cfg(unix)]
#[test]
fn the_apple_arm_answers_an_oversized_component_by_its_construction_bound() {
    let root = FixtureRoot::new();
    let b = root.path().join("b");
    fs::create_dir_all(&b).unwrap();
    let real = stage_executable(&b, "dsh", b"#!/bin/sh\ntrue\n");
    let real = real.canonicalize().unwrap();
    let search = |library: Library, operation: Operation, entries: String| Search {
        entries: OsString::from(entries),
        default: false,
        library,
        operation,
        env_reference: PathBuf::from(ENV_REFERENCE),
    };
    let find = |library, operation, entries: String| {
        lookup_in("dsh", &search(library, operation, entries), &mut Vec::new())
    };
    // The exact `PATH` spellings the matrix's oversized layouts stage:
    // the sole 5,000-byte component (cell n0-l10); the 4,095-, 4,096-
    // and 5,000-byte components ahead of a runnable B (n0-l11/47,
    // n0-l12/48 and n0-l13/49); the 4,092 bytes the padded-A spelling
    // reaches (n0-l39–41); and the 4,096-byte component ahead of an
    // EXPLICIT empty entry and B (n0-l18). Every one of them is over
    // Apple's 1,024-byte bound, and the first four are the cells the
    // third macOS pass had still to reveal.
    let x = |bytes: usize| "x".repeat(bytes);
    let alone = x(5000);
    let then_b = |bytes: usize| format!("{}:{}", x(bytes), b.display());
    let then_empty_b = format!("{}::{}", x(4096), b.display());
    // Apple's bound is on the CANDIDATE, so the refusal names the
    // candidate the walk would have built and the buffer it builds one
    // in — never a candidate the kernel measured.
    let bound = |component: &str| {
        format!(
            "the DSH layout is unreadable: {}/dsh: the platform's lookup stops before attempting \
             a candidate longer than the {DARWIN_PATH_MAX} bytes it builds one in (ENAMETOOLONG)",
            component
        )
    };

    // `posix_spawnp` — production's own form, and the matrix's INHERITED
    // cell: the walk stops at the first oversized token, whatever
    // follows it, and B is never reached.
    for component in [5000, 4096, 4095, 4092] {
        assert_eq!(
            refused(find(Library::Apple, Operation::Spawn, then_b(component))),
            bound(&x(component)),
            "Apple's posix_spawnp stops at a {component}-byte token"
        );
    }
    assert_eq!(
        refused(find(Library::Apple, Operation::Spawn, alone.clone())),
        bound(&alone)
    );
    assert_eq!(
        refused(find(Library::Apple, Operation::Spawn, then_empty_b.clone())),
        bound(&x(4096)),
        "the construction stop comes before the explicit empty entry"
    );

    // `execvP` — the matrix's EXPLICIT cell: the same token is a SKIP,
    // so the walk goes on to whatever the next token is. B is selected
    // where B follows; the empty entry that follows is the working
    // directory, and is refused by the reconciled rule under the NAME
    // the caller searched for; a sole oversized token leaves the walk
    // with no candidate at all.
    for component in [5000, 4096, 4095, 4092] {
        assert_eq!(
            find(Library::Apple, Operation::Exec, then_b(component))
                .unwrap()
                .path,
            real,
            "Apple's execvP skips a {component}-byte token and reaches B"
        );
    }
    assert_eq!(
        refused(find(Library::Apple, Operation::Exec, then_empty_b)),
        "the DSH layout is unreadable: dsh: the platform's search would fall into the working \
         directory: PATH entry 1 is empty"
    );
    assert_eq!(
        refused(find(Library::Apple, Operation::Exec, alone.clone())),
        "the DSH layout is unreadable: 'dsh' is not on PATH (the search attempted no candidate: \
         every component was skipped as longer than the buffer the platform builds one in)"
    );

    // glibc, on the same four spellings, under both operations — one
    // loop, so the operation changes nothing — and NOT Apple's answer
    // anywhere: 4,095 bytes fit its 4,096-byte bound and are ATTEMPTED,
    // 4,096 and 5,000 are skipped with the cursor left on the colon so
    // the next candidate is the working directory, and a sole oversized
    // component ends the walk with nothing constructed.
    let attempted = format!(
        "the DSH layout is unreadable: {}/dsh: metadata answers {}, on which the platform's \
         lookup stops",
        x(4095),
        std::io::Error::from_raw_os_error(rustix::io::Errno::NAMETOOLONG.raw_os_error())
    );
    let implicit_cwd = |bytes: usize| {
        format!(
            "the DSH layout is unreadable: dsh: the platform's search would fall into the \
             working directory: glibc skips the {bytes}-byte component and its next iteration \
             is the empty entry it leaves the cursor on (posix/execvpe.c 118–124, 168)"
        )
    };
    for operation in [Operation::Spawn, Operation::Exec] {
        assert_eq!(
            refused(find(Library::Glibc, operation, then_b(4095))),
            attempted,
            "glibc attempts a 4095-byte component and the kernel stops it"
        );
        for bytes in [4096, 5000] {
            assert_eq!(
                refused(find(Library::Glibc, operation, then_b(bytes))),
                implicit_cwd(bytes)
            );
        }
        assert_eq!(
            refused(find(Library::Glibc, operation, alone.clone())),
            "the DSH layout is unreadable: 'dsh' is not on PATH (the search attempted no \
             candidate: every component was skipped as longer than the buffer the platform \
             builds one in)"
        );
    }
}

/// The refusal each pinned errno ENDS IN, read from the lookup itself
/// under every library's arm — the searched name's exhaustion and denial
/// formatting, and the direct name's own, not the switch's intermediate
/// answer.
///
/// The Apple-arm audit of PR #311 cited `step` and `lookup_failure` for
/// EACCES, ENOENT and ENOTDIR (review 2026-09-21, R2). Those two answer
/// the SWITCH's question — continue, stop, or unpinned — and a
/// continuation is not a refusal: what a caller actually reads is
/// `Search::find`'s exhaustion, which reports a remembered denial over
/// any later cause, or `lookup_in`'s direct-name formatting of a
/// `Candidate::Passed` that had no next entry to walk to. Those final
/// strings are what the differential matrix compares on macOS, so they
/// are what the Apple arm owes an assertion, and the injected arm proves
/// them on this host.
///
/// The loading refusal is here for the same reason. ENOEXEC never
/// reaches `step` at all: a candidate whose content is neither a `#!`
/// script nor a loadable image is refused by `native_obstruction` before
/// any execution, on every arm alike, and the audit's `step(Apple,
/// NOEXEC)` row described the switch rather than the path an
/// unrecognized executable actually takes.
#[cfg(unix)]
#[test]
fn each_pinned_errno_ends_in_the_same_refusal_on_every_librarys_arm() {
    use std::os::unix::fs::PermissionsExt;

    let root = FixtureRoot::new();
    let here = root.path();
    // A missing directory (ENOENT), a regular file spelled as one
    // (ENOTDIR), a candidate this process may not execute (EACCES), a
    // runnable B, and an executable file whose content no loader reads.
    let missing = here.join("nowhere");
    let file = here.join("file");
    fs::write(&file, b"a file, not a directory\n").unwrap();
    let denied_in = here.join("denied");
    fs::create_dir_all(&denied_in).unwrap();
    let denied = denied_in.join("dsh");
    fs::write(&denied, b"#!/bin/sh\ntrue\n").unwrap();
    fs::set_permissions(&denied, fs::Permissions::from_mode(0o644)).unwrap();
    let b = here.join("b");
    fs::create_dir_all(&b).unwrap();
    let real = stage_executable(&b, "dsh", b"#!/bin/sh\ntrue\n")
        .canonicalize()
        .unwrap();
    let unloadable_in = here.join("unloadable");
    fs::create_dir_all(&unloadable_in).unwrap();
    let unloadable = stage_executable(&unloadable_in, "dsh", b"\x7fnot a script, not an image\n");
    // A component longer than the kernel takes, under a directory that
    // exists, so the kernel measures it and answers ENAMETOOLONG.
    let overlong = b.join("y".repeat(300));
    assert_eq!(
        errno_of(&fs::metadata(&overlong).unwrap_err()),
        Some(rustix::io::Errno::NAMETOOLONG),
        "the fixture is a name this host's kernel refuses by length"
    );

    let search = |library: Library, operation: Operation, entries: String| Search {
        entries: OsString::from(entries),
        default: false,
        library,
        operation,
        env_reference: PathBuf::from(ENV_REFERENCE),
    };
    let find = |command: &str, library, operation, entries: String| {
        lookup_in(
            command,
            &search(library, operation, entries),
            &mut Vec::new(),
        )
    };
    // The exhaustion `Search::find` answers with, built from the same
    // kernel error the candidate itself gives this host.
    let ended = |candidate: &Path| {
        format!(
            "the DSH layout is unreadable: 'dsh' is not on PATH (the search ended at {}: {})",
            candidate.display(),
            fs::metadata(candidate).unwrap_err()
        )
    };
    let denial = format!(
        "the DSH layout is unreadable: 'dsh' is not executable by this process on PATH: {}: is \
         not executable by this process",
        denied.display()
    );
    let directly = |candidate: &Path, why: String| {
        format!(
            "the DSH layout is unreadable: {}: {why}",
            candidate.display()
        )
    };
    let loading = "is not a loadable native image: neither a #! script nor a native image";

    for library in [Library::Apple, Library::Glibc, Library::Musl, LIBRARY] {
        for operation in [Operation::Spawn, Operation::Exec] {
            let at = |what: &str| format!("{library:?} under {operation:?}: {what}");
            // SEARCHED. EACCES, ENOENT and ENOTDIR sit in every pinned
            // continue-set, so every arm walks the same entries past the
            // same causes and ends in the same words: the remembered
            // denial where one entry denied, the LAST cause otherwise.
            assert_eq!(
                refused(find(
                    "dsh",
                    library,
                    operation,
                    format!("{}:{}", denied_in.display(), missing.display()),
                )),
                denial,
                "{}",
                at("a denied entry is reported over a later absence")
            );
            assert_eq!(
                refused(find(
                    "dsh",
                    library,
                    operation,
                    format!("{}:{}", missing.display(), denied_in.display()),
                )),
                denial,
                "{}",
                at("and over an earlier one")
            );
            assert_eq!(
                refused(find(
                    "dsh",
                    library,
                    operation,
                    missing.display().to_string()
                )),
                ended(&missing.join("dsh")),
                "{}",
                at("an exhausted search keeps ENOENT's own words")
            );
            assert_eq!(
                refused(find("dsh", library, operation, file.display().to_string())),
                ended(&file.join("dsh")),
                "{}",
                at("and ENOTDIR's")
            );
            // All three walked past, and the entry behind them selected:
            // the continuations are continuations on every arm.
            assert_eq!(
                find(
                    "dsh",
                    library,
                    operation,
                    format!(
                        "{}:{}:{}:{}",
                        missing.display(),
                        file.display(),
                        denied_in.display(),
                        b.display()
                    ),
                )
                .unwrap()
                .path,
                real,
                "{}",
                at("the search walks past all three to B")
            );
            // A candidate no loader reads STOPS the search on every arm,
            // ahead of the runnable B behind it: the refusal is the
            // loading one, and no errno switch is consulted for it.
            assert_eq!(
                refused(find(
                    "dsh",
                    library,
                    operation,
                    format!("{}:{}", unloadable_in.display(), b.display()),
                )),
                directly(&unloadable, loading.to_string()),
                "{}",
                at("an unloadable candidate is refused before B is reached")
            );

            // DIRECT. No library's switch runs, so each of these is the
            // one answer `lookup_in` formats, and it is the same answer
            // on every arm.
            let direct = |candidate: &Path| {
                refused(find(
                    candidate.to_str().unwrap(),
                    library,
                    operation,
                    b.display().to_string(),
                ))
            };
            assert_eq!(
                direct(&missing.join("dsh")),
                directly(
                    &missing.join("dsh"),
                    fs::metadata(missing.join("dsh")).unwrap_err().to_string()
                ),
                "{}",
                at("a direct name's ENOENT")
            );
            assert_eq!(
                direct(&file.join("dsh")),
                directly(
                    &file.join("dsh"),
                    fs::metadata(file.join("dsh")).unwrap_err().to_string()
                ),
                "{}",
                at("a direct name's ENOTDIR")
            );
            assert_eq!(
                direct(&denied),
                directly(&denied, "is not executable by this process".to_string()),
                "{}",
                at("a direct name's EACCES")
            );
            assert_eq!(
                direct(&b),
                directly(&b, "is not a regular file".to_string()),
                "{}",
                at("a direct name that is a directory")
            );
            assert_eq!(
                direct(&overlong),
                directly(
                    &overlong,
                    format!(
                        "metadata answers {}, on which the platform's lookup stops",
                        std::io::Error::from_raw_os_error(
                            rustix::io::Errno::NAMETOOLONG.raw_os_error()
                        )
                    )
                ),
                "{}",
                at("a direct name's terminal ENAMETOOLONG")
            );
            assert_eq!(
                direct(&unloadable),
                directly(&unloadable, loading.to_string()),
                "{}",
                at("a direct name no loader reads")
            );
            assert_eq!(
                find(
                    b.join("dsh").to_str().unwrap(),
                    library,
                    operation,
                    String::new(),
                )
                .unwrap()
                .path,
                real,
                "{}",
                at("and the direct name that loads is selected")
            );
        }
    }
}

/// A native image's loader is read as the kernel reads it, from the
/// image's own `PT_INTERP`, and every answer the loader can give is a
/// named refusal or an admission — never a guess. The candidates are
/// synthetic ELF images of this target's machine, planted as explicit
/// paths; nothing here is executed (security hold 2026-09-20, finding 4).
/// ELF targets only: a synthetic ELF is another target's image on macOS,
/// whose loader is the Mach-O `dyld` this test does not construct
/// (review 2026-09-20, R6).
#[cfg(all(unix, not(target_vendor = "apple")))]
#[test]
fn a_native_images_loader_is_read_as_the_kernel_reads_it() {
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let plant = |name: &str, loader: Option<&Path>| -> PathBuf {
        let loader = loader.map(|loader| loader.as_os_str().as_encoded_bytes().to_vec());
        stage_executable(
            dir.path(),
            name,
            &image::tests::synthetic_elf(loader.as_deref()),
        )
    };
    let resolve = |candidate: &Path| resolve_executable_in(candidate.to_str().unwrap(), None);
    let refusal = |candidate: &Path, why: &str| {
        format!(
            "the DSH layout is unreadable: {}: {why}",
            candidate.display()
        )
    };

    // A static image names no loader and is admitted as it is.
    let stat = plant("static", None);
    assert_eq!(resolve(&stat).unwrap(), stat.canonicalize().unwrap());

    // The loader this test binary itself names is a real loader: an
    // image naming it is admitted.
    let mut exe = fs::File::open(std::env::current_exe().unwrap()).unwrap();
    let exe_len = exe.metadata().unwrap().len();
    let real_loader = image::inspect(&mut exe, exe_len)
        .unwrap()
        .loader
        .expect("this test binary is dynamically linked");
    let real_loader = PathBuf::from(std::ffi::OsStr::from_bytes(&real_loader));
    let dynamic = plant("dynamic", Some(&real_loader));
    assert_eq!(resolve(&dynamic).unwrap(), dynamic.canonicalize().unwrap());

    // A loader spelled relative to nothing.
    let relative = plant("relative", Some(Path::new("lib/ld.so")));
    assert_eq!(
        refused(resolve(&relative)),
        refusal(
            &relative,
            "needs the ELF loader 'lib/ld.so', which is not an absolute path"
        )
    );
    // A loader that is a directory, and one that is a file nobody may
    // execute.
    let as_dir = plant("as-dir", Some(dir.path()));
    assert_eq!(
        refused(resolve(&as_dir)),
        refusal(
            &as_dir,
            &format!(
                "needs the ELF loader '{}', which is not an executable file",
                dir.path().display()
            )
        )
    );
    let plain = dir.path().join("plain-loader");
    fs::write(&plain, b"x").unwrap();
    fs::set_permissions(&plain, fs::Permissions::from_mode(0o644)).unwrap();
    let not_executable = plant("not-executable", Some(&plain));
    assert_eq!(
        refused(resolve(&not_executable)),
        refusal(
            &not_executable,
            &format!(
                "needs the ELF loader '{}', which is not an executable file",
                plain.display()
            )
        )
    );
    // A loader that is missing.
    let absent = dir.path().join("absent-loader");
    let enoent = fs::metadata(&absent).unwrap_err();
    let missing = plant("missing", Some(&absent));
    assert_eq!(
        refused(resolve(&missing)),
        refusal(
            &missing,
            &format!(
                "needs the ELF loader '{}', which is missing: {enoent}",
                absent.display()
            )
        )
    );
    // A loader this process may execute but not read: unreadable, which
    // is a refusal, not a guess; a privileged process reads it and finds
    // one byte, which is no image.
    let sealed_file = dir.path().join("sealed-file");
    fs::write(&sealed_file, b"x").unwrap();
    fs::set_permissions(&sealed_file, fs::Permissions::from_mode(0o000)).unwrap();
    let privileged = fs::read(&sealed_file).is_ok();
    let unreadable = dir.path().join("unreadable-loader");
    fs::write(&unreadable, b"x").unwrap();
    fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o111)).unwrap();
    let candidate = plant("unreadable", Some(&unreadable));
    let resolved = resolve(&candidate);
    fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o755)).unwrap();
    match privileged {
        true => assert_eq!(
            refused(resolved),
            refusal(
                &candidate,
                &format!(
                    "needs the ELF loader '{}', which is not a loadable image: neither a #! \
                     script nor a native image",
                    unreadable.display()
                )
            )
        ),
        false => {
            assert!(
                refused(resolved).starts_with(&refusal(
                    &candidate,
                    &format!(
                        "needs the ELF loader '{}', which cannot be read: ",
                        unreadable.display()
                    )
                )),
                "the unreadable loader is named"
            );
        }
    }
    // A loader that is an executable script, and one that is an image of
    // another format: neither loads as an ELF loader.
    let script = stage_executable(dir.path(), "script-loader", b"#!/bin/sh\n");
    let scripted = plant("scripted", Some(&script));
    assert_eq!(
        refused(resolve(&scripted)),
        refusal(
            &scripted,
            &format!(
                "needs the ELF loader '{}', which is not a loadable image: neither a #! \
                 script nor a native image",
                script.display()
            )
        )
    );
    let foreign = stage_executable(dir.path(), "pe-loader", &image::tests::synthetic_pe());
    let foreign_loader = plant("foreign", Some(&foreign));
    assert_eq!(
        refused(resolve(&foreign_loader)),
        refusal(
            &foreign_loader,
            &format!(
                "needs the ELF loader '{}', which does not load as a ELF loader",
                foreign.display()
            )
        )
    );
}

#[test]
fn dsh_composite_composes_over_the_real_and_injected_runtime_probes() {
    let install = Synthetic::new();
    let injected = dsh_composite_resolving(&install.seams, || Ok(install.node())).unwrap();
    assert_eq!(injected.node, "v22.23.2");
    assert_eq!(injected.canonical.len(), 64);
    // The real probe runs `node --version` from `PATH` when the selection
    // retained none; the delegation is exercised even when a host lacks
    // node. A RETAINED runtime is probed as it is: one that is gone since
    // selection is a refusal naming the probe, never a lookup of another.
    let _ = dsh_composite(&install.seams);
    let _ = spawn_node_runtime(None);
    let gone = install.dir.path().join("gone-node");
    let enoent = fs::metadata(&gone).unwrap_err();
    assert_eq!(
        refused(spawn_node_runtime(Some(&node_at(&gone)))),
        format!("the DSH layout is unreadable: node --version: {enoent}")
    );
}

/// The Node selection made when the executable was selected is the one
/// the composite observes. Under `PATH=A:B`, selecting a `#!/usr/bin/env
/// node` executable selects `A/node`; the composite then probes THAT
/// file, so an `A/node` gone since selection is a refusal, where a second
/// lookup would have found `B/node` and produced a readable composite for
/// a runtime the selection never chose (review 2026-09-20, R5). Restoring
/// the second lookup — `node: None` on the seams — makes the composite
/// readable through B, which is the removal control recorded in the
/// delivery account.
#[cfg(unix)]
#[test]
fn the_composite_observes_the_node_the_selection_retained() {
    let install = Synthetic::new();
    let a = install.dir.path().join("a");
    let b = install.dir.path().join("b");
    fs::create_dir_all(&a).unwrap();
    fs::create_dir_all(&b).unwrap();
    stage_executable(&a, "node", b"#!/bin/sh\necho v22.23.2\n");
    stage_executable(&b, "node", b"#!/bin/sh\necho v22.23.2\n");
    let path = Some(OsString::from(format!("{}:{}", a.display(), b.display())));
    let selected = select_in(&install.seams.executable, path).unwrap();
    assert_eq!(
        selected.path,
        Path::new(&install.seams.executable).canonicalize().unwrap()
    );
    assert_eq!(
        node_file(&selected.node),
        Some(a.join("node").canonicalize().unwrap())
    );
    let seams = DshSeams {
        executable: install.seams.executable.clone(),
        home: install.seams.home.clone(),
        node: selected.node.clone(),
        head: selected.head.clone(),
    };
    // With A's runtime in place the composite reads through it.
    assert_eq!(dsh_composite(&seams).unwrap().node, "v22.23.2");
    // A's runtime gone since selection: the probe of the retained file
    // fails by name, and B is never consulted.
    fs::remove_file(a.join("node")).unwrap();
    let enoent = fs::metadata(a.join("node")).unwrap_err();
    assert_eq!(
        refused(dsh_composite(&seams)),
        format!("the DSH layout is unreadable: node --version: {enoent}")
    );
    // The seams without a retained runtime — a selection that established
    // none — look `node` up under this process's own `PATH`, as before.
    let unretained = DshSeams {
        node: None,
        ..seams.clone()
    };
    assert_eq!(
        dsh_composite(&unretained).map(|composite| composite.node),
        spawn_node_runtime(None).map(|node| node.version)
    );
}

/// D10's byte-retention requirement: the identity-bearing bytes
/// SELECTION inspected are the bytes the observation reads.
///
/// Composition asked the launcher for its first line again, AFTER the
/// version probe had run it. A shell launcher that answers `v22.23.2`
/// and rewrites itself to the `env node` shebang while doing so was
/// therefore refused by the reading that admitted it and admitted by the
/// reading that followed: a readable composite for an installation whose
/// head the resolver never accepted, with no concurrent writer needed
/// (review 2026-09-20, F6). The otherwise identical launcher that does
/// not rewrite itself is the control, and a valid `env node` launcher
/// still reads. Restoring the post-probe reread makes the rewriting
/// launcher readable, which is the removal control recorded in the
/// delivery account.
#[cfg(unix)]
#[test]
fn the_composite_reuses_the_launcher_head_selection_inspected() {
    use std::process::Command;

    let install = Synthetic::new();
    let bin = PathBuf::from(&install.seams.executable);
    let lib = bin.parent().unwrap().to_path_buf();
    let shims = install.dir.path().join("shims");
    fs::create_dir_all(&shims).unwrap();
    stage_executable(&shims, "node", b"#!/bin/sh\necho v22.23.2\n");
    let path = Some(OsString::from(shims.display().to_string()));
    let canonical = bin.canonicalize().unwrap();
    let not_measured = format!(
        "the DSH layout is unreadable: {}: first line is not the env node shebang",
        canonical.display()
    );
    let observe = |head: Vec<u8>| {
        let seams = DshSeams {
            executable: install.seams.executable.clone(),
            home: install.seams.home.clone(),
            node: None,
            head,
        };
        dsh_composite_with(&seams, &install.node(), &[])
    };

    // The rewriting launcher. Its new bytes are staged and renamed in,
    // so the shell reading it keeps the file it started on.
    stage_executable(
        &lib,
        "bin.js",
        b"#!/bin/sh\nprintf '#!/usr/bin/env node\\n' > \"$0.staging\"\nchmod 755 \"$0.staging\"\nmv \"$0.staging\" \"$0\"\necho v22.23.2\n",
    );
    let selected = select_in(&install.seams.executable, path.clone()).unwrap();
    assert_eq!(first_line(&bin, &selected.head).unwrap(), "#!/bin/sh");
    // The version probe, as doctor runs it: the selected executable,
    // answering for itself.
    let probe = spawn_retrying_etxtbsy(Command::new(&bin).arg("--version"));
    assert_eq!(String::from_utf8_lossy(&probe.stdout).trim(), "v22.23.2");
    assert_eq!(
        fs::read(&bin).unwrap(),
        b"#!/usr/bin/env node\n",
        "the launcher rewrote itself to the measured shebang while answering the probe"
    );
    assert_eq!(
        refused(observe(selected.head)),
        not_measured,
        "the observation reads the first line selection inspected"
    );

    // The control: the same launcher without the rewrite. The file on
    // disk and the retained head agree, and the refusal is the same.
    stage_executable(&lib, "bin.js", b"#!/bin/sh\necho v22.23.2\n");
    let selected = select_in(&install.seams.executable, path.clone()).unwrap();
    let probe = spawn_retrying_etxtbsy(Command::new(&bin).arg("--version"));
    assert_eq!(String::from_utf8_lossy(&probe.stdout).trim(), "v22.23.2");
    assert_eq!(fs::read(&bin).unwrap(), b"#!/bin/sh\necho v22.23.2\n");
    assert_eq!(refused(observe(selected.head)), not_measured);

    // And a valid `env node` launcher still reads, through the retained
    // head and nothing else.
    stage_executable(&lib, "bin.js", b"#!/usr/bin/env node\n");
    let selected = select_in(&install.seams.executable, path).unwrap();
    assert_eq!(
        first_line(&bin, &selected.head).unwrap(),
        "#!/usr/bin/env node"
    );
    assert_eq!(
        observe(selected.head).unwrap().canonical,
        install.composite().canonical
    );
}

/// A 40-byte Mach-O whose one load command is `LC_LOAD_DYLINKER` with
/// `cmdsize` 8 made the reader panic on the offset field it had not
/// bounded, and doctor with it (review 2026-09-20, R4). Through public
/// selection it is a refusal by name, as an explicit path and on a
/// search alike; the built-doctor regression is in
/// `crates/brokkr-cli/tests/doctor_dsh_selection.rs`.
#[cfg(unix)]
#[test]
fn a_truncated_macho_dylinker_command_is_refused_rather_than_panicking() {
    let dir = tempfile::tempdir().unwrap();
    let mut bytes = vec![0xcf, 0xfa, 0xed, 0xfe];
    bytes.extend_from_slice(&image::tests_cputype().to_le_bytes());
    bytes.extend_from_slice(&[0; 4]);
    bytes.extend_from_slice(&2u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&8u32.to_le_bytes());
    bytes.extend_from_slice(&[0; 8]);
    bytes.extend_from_slice(&0xeu32.to_le_bytes());
    bytes.extend_from_slice(&8u32.to_le_bytes());
    assert_eq!(bytes.len(), 40);
    let planted = stage_executable(dir.path(), "dsh", &bytes);
    let reason = format!(
        "the DSH layout is unreadable: {}: is not a loadable native image: a malformed Mach-O \
         dynamic linker command",
        planted.display()
    );
    assert_eq!(
        refused(resolve_executable_in(planted.to_str().unwrap(), None)),
        reason
    );
    assert_eq!(
        refused(resolve_executable_in(
            "dsh",
            Some(dir.path().as_os_str().to_os_string())
        )),
        reason
    );
    assert_eq!(
        DshSeams::selected_from(
            "dsh".to_string(),
            |name| select_in(name, Some(dir.path().into())),
            None
        )
        .unwrap_err()
        .cause
        .to_string(),
        reason
    );
}

#[cfg(unix)]
#[test]
fn the_plugin_walk_refuses_symlinks_special_files_and_unreadable_roots() {
    use std::os::unix::net::UnixListener;
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "LICENSE", b"x");
    std::os::unix::fs::symlink(dir.path().join("LICENSE"), dir.path().join("link")).unwrap();
    let error =
        plugin_file_digests("plugin", dir.path(), &PLUGIN_FILES, &read_dir_entries).unwrap_err();
    assert_eq!(
        error.to_string(),
        "plugin component is unreadable: 'link' is a symlink"
    );

    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "LICENSE", b"x");
    let _socket = UnixListener::bind(dir.path().join("sock")).unwrap();
    let error =
        plugin_file_digests("plugin", dir.path(), &PLUGIN_FILES, &read_dir_entries).unwrap_err();
    assert_eq!(
        error.to_string(),
        "plugin component is unreadable: 'sock' is neither a file nor a directory"
    );

    let error = plugin_file_digests(
        "plugin",
        Path::new("/definitely/not/a/realdir"),
        &PLUGIN_FILES,
        &read_dir_entries,
    )
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("the component directory cannot be read"),
        "{error}"
    );
}

/// A name the platform holds as bytes but cannot spell as UTF-8 is
/// refused by reason. A lossy conversion would map it onto a spelling
/// that might collide with a declared one, which is the one outcome a
/// membership check must never allow.
///
/// Whether such a name can EXIST is the filesystem's answer, not this
/// suite's. Linux's tmpfs and ext4 take any byte sequence without a NUL
/// or a `/`, so the entry is made and the walker meets it. APFS enforces
/// UTF-8 at creation and answers EILSEQ, so on that host there is no
/// on-disk entry for the walker to meet and none can be made: what is
/// provable there is the filesystem's own refusal, and it is asserted by
/// its exact cause — never unwrapped into a panic, never skipped, and
/// never reduced to `is_err()`. The walker's refusal itself then awaits a
/// host whose filesystem can hold the name; no injected reader can stand
/// in for it, because a `std::fs::DirEntry` is only ever yielded by a
/// real directory.
#[cfg(unix)]
#[test]
fn the_plugin_walk_refuses_a_name_that_is_not_utf8() {
    use std::os::unix::ffi::OsStrExt;
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "LICENSE", b"x");
    let raw = dir.path().join(std::ffi::OsStr::from_bytes(b"LICENSE\xff"));
    match fs::write(&raw, b"x") {
        Ok(()) => {
            let error = plugin_file_digests("plugin", dir.path(), &PLUGIN_FILES, &read_dir_entries)
                .unwrap_err();
            assert_eq!(
                error.to_string(),
                "plugin component is unreadable: the component directory holds an entry whose name is not UTF-8"
            );
        }
        Err(error) => {
            assert_eq!(
                errno_of(&error),
                Some(rustix::io::Errno::ILSEQ),
                "a filesystem that refuses the name refuses it as an encoding error: {error}"
            );
            // And it really refused: the name is ABSENT, by that exact
            // cause. `is_err()` would have been satisfied by any other
            // answer — a denial, an I/O fault — and so would have proved
            // nothing about whether the entry was made (review of run
            // `d462f720`). The directory's exact membership says the
            // same thing from the walk's own side: one entry, the one
            // this test made.
            let left = raw.symlink_metadata().err().unwrap_or_else(|| {
                panic!("the refused name left an entry behind: {}", raw.display())
            });
            assert_eq!(
                left.kind(),
                std::io::ErrorKind::NotFound,
                "the refused name left no entry: {left}"
            );
            let mut names: Vec<std::ffi::OsString> = fs::read_dir(dir.path())
                .unwrap()
                .map(|entry| entry.unwrap().file_name())
                .collect();
            names.sort();
            assert_eq!(
                names,
                vec![std::ffi::OsString::from("LICENSE")],
                "the directory holds the one made name"
            );
            let digests =
                plugin_file_digests("plugin", dir.path(), &["LICENSE"], &read_dir_entries).unwrap();
            assert_eq!(digests.len(), 1, "the walk reads that one name");
            eprintln!(
                "the walker's non-UTF-8 refusal is PENDING on {}: the filesystem answered {error} \
                 to the fixture's creation",
                std::env::consts::OS
            );
        }
    }
}

/// A declared file whose bytes cannot be read after its metadata was
/// read is a component refusal naming that relative path. Running as
/// root defeats the denial, and the assertion then fails rather than
/// passing on a fault that did not happen.
#[cfg(unix)]
#[test]
fn the_plugin_walk_refuses_a_declared_file_it_cannot_read() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    for file in PLUGIN_FILES {
        write(dir.path(), file, file.as_bytes());
    }
    let locked = dir.path().join("LICENSE");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    let result = plugin_file_digests("plugin", dir.path(), &PLUGIN_FILES, &read_dir_entries);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o600)).unwrap();
    let error = result.unwrap_err();
    assert!(
        error
            .to_string()
            .starts_with("plugin component is unreadable: 'LICENSE': Permission denied"),
        "{error}"
    );
}

/// A directory entry the reader itself cannot yield is a component refusal;
/// the injected `read_dir` makes the mid-walk iterator error deterministic
/// where a real filesystem cannot be asked to fail.
#[test]
fn the_plugin_walk_refuses_a_directory_entry_the_reader_cannot_yield() {
    let dir = tempfile::tempdir().unwrap();
    let failing = |_: &Path| -> std::io::Result<DirEntries> {
        Ok(Box::new(std::iter::once(Err(std::io::Error::other(
            "the directory entry is unreadable",
        )))))
    };
    let mut found = BTreeMap::new();
    let error = walk(
        "plugin",
        dir.path(),
        "",
        &PLUGIN_FILES,
        &mut found,
        &failing,
    )
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        "plugin component is unreadable: the component directory yielded an unreadable entry: \
         the directory entry is unreadable"
    );
}

/// A directory can be listed with read permission alone while
/// `symlink_metadata` needs search permission, so clearing search makes the
/// per-entry metadata read fail without hiding the entry. Running as root
/// defeats the denial, and the assertion then fails rather than passing on a
/// fault that did not happen.
#[cfg(unix)]
#[test]
fn the_plugin_walk_refuses_an_entry_metadata_it_cannot_read() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let locked = dir.path().join("locked");
    fs::create_dir_all(&locked).unwrap();
    write(&locked, "LICENSE", b"x");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o400)).unwrap();
    let result = plugin_file_digests("plugin", &locked, &PLUGIN_FILES, &read_dir_entries);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o700)).unwrap();
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Permission denied"), "{error}");
}

#[test]
fn npm_keys_with_an_empty_or_dotted_component_are_refused() {
    for key in [
        "node_modules/@./name",
        "node_modules/@scope/.",
        "node_modules/@scope/..",
    ] {
        assert_eq!(
            refused(npm_name(key)),
            format!("npm key is unreadable: '{key}': invalid scope or name component"),
            "{key:?}"
        );
    }
}

/// Every arm is asserted by the REASON it answered with, and the four
/// reasons differ: a non-object entry, a missing version, a missing
/// integrity and a malformed one are separate refusals, and the entry
/// each one names is the key an operator would go looking for. A bare
/// `is_err()` passed on all four alike (council return 2026-09-19,
/// finding 4).
#[test]
fn npm_locks_reject_unparseable_and_incomplete_entries() {
    for (packages, reason) in [
        (r#"{"node_modules/a":5}"#, "entry is not an object"),
        (r#"{"node_modules/a":{}}"#, "no string 'version'"),
        // A version that is present but is not a STRING is refused as a
        // missing one rather than rendered into `5` or `true`: a JSON
        // number is not a version this reader may spell for the lock.
        (
            r#"{"node_modules/a":{"version":5,"integrity":"sha512-A"}}"#,
            "no string 'version'",
        ),
        (
            r#"{"node_modules/a":{"version":true,"integrity":"sha512-A"}}"#,
            "no string 'version'",
        ),
        (
            r#"{"node_modules/a":{"version":null,"integrity":"sha512-A"}}"#,
            "no string 'version'",
        ),
        (
            r#"{"node_modules/a":{"version":"","integrity":"sha512-A"}}"#,
            "version is empty",
        ),
        (
            r#"{"node_modules/a":{"version":"1.0.0","integrity":5}}"#,
            "no registry 'integrity'",
        ),
        (
            r#"{"node_modules/a":{"version":"1.0.0"}}"#,
            "no registry 'integrity'",
        ),
        (
            r#"{"node_modules/a":{"version":"1.0.0","integrity":""}}"#,
            "integrity is empty",
        ),
        (
            "{\"node_modules/a\":{\"version\":\"1.0.0\",\"integrity\":\"a\\nb\"}}",
            "integrity carries whitespace",
        ),
    ] {
        assert_eq!(
            refused(npm_dependencies(
                &lock(&format!(r#"{{"packages":{packages}}}"#)),
                &[]
            )),
            format!("npm lock is unreadable: 'node_modules/a': {reason}"),
            "{packages}"
        );
    }
}

#[test]
fn pnpm_locks_reject_every_unrecognized_construct() {
    let blank = "\nlockfileVersion: '9.0'\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n";
    assert_eq!(
        pnpm_dependencies(blank, &[]).unwrap(),
        vec!["debug 2.6.9 sha512-X".to_string()]
    );
    // A recognized section that carries no identity keeps its body
    // skipped, and a block-form package child opens a region whose own
    // lines are skipped: both are the positive controls the closed
    // arms below are measured against.
    let full = "lockfileVersion: '9.0'\n\nsettings:\n  autoInstallPeers: false\n\nimporters:\n\n  .:\n    dependencies:\n      x:\n        specifier: file:/tmp/x\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n    engines: {node: '>=1'}\n    peerDependencies:\n      '@scope/peer': '>=1'\n\nsnapshots:\n\n  debug@2.6.9: {}\n";
    assert_eq!(
        pnpm_dependencies(full, &[]).unwrap(),
        vec!["debug 2.6.9 sha512-X".to_string()]
    );
    for (text, reason) in [
        // An all-blank document has no version header.
        ("\n\n", "empty document"),
        // A first non-blank line that is not the version header.
        ("foo\n", "no lockfileVersion header"),
        // A version scalar whose quote is never closed. `trim_matches`
        // repaired this into `9.0`.
        ("lockfileVersion: '9.0\npackages:\n", "a malformed lockfileVersion"),
        // A CRLF document read as a Unix one: `str::lines` drops the CR
        // silently, so the two spellings hashed alike.
        ("lockfileVersion: '9.0'\r\npackages:\r\n", "a carriage return"),
        // A child line before any section opened. The inherited reader
        // IGNORED this, which is the whole of "arbitrary children are
        // skipped".
        (
            "lockfileVersion: '9.0'\n  stray: x\npackages:\n",
            "a child line outside every section",
        ),
        // An unrecognized top-level key, and a top-level line that is no
        // mapping key at all.
        (
            "lockfileVersion: '9.0'\nrogue:\n  a@1.0.0:\npackages:\n",
            "an unrecognized top-level key 'rogue'",
        ),
        ("lockfileVersion: '9.0'\nrogue\n", "a top-level line that is not a mapping key"),
        // A document with no `packages:` section at all yields an empty
        // dependency set, which is a silent answer rather than a read one.
        (
            "lockfileVersion: '9.0'\nsettings:\n  autoInstallPeers: false\n",
            "no packages section",
        ),
        // Odd indentation, and a depth with no open block above it.
        (
            "lockfileVersion: '9.0'\npackages:\n   debug@2.6.9:\n",
            "an odd indentation",
        ),
        (
            "lockfileVersion: '9.0'\npackages:\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n      deeper: x\n",
            "a package line at an unrecognized indentation",
        ),
        // A package key without a trailing colon, and the flow form the
        // grammar does not admit.
        (
            "lockfileVersion: '9.0'\npackages:\n  debug@2.6.9\n",
            "a package key is not colon-terminated",
        ),
        (
            "lockfileVersion: '9.0'\npackages:\n  debug@2.6.9: {}\n",
            "a package key is not colon-terminated",
        ),
        (
            "lockfileVersion: '9.0'\npackages:\n  debug@2.6.9: nested:\n",
            "a package key that is itself a mapping",
        ),
        (
            "lockfileVersion: '9.0'\npackages:\n  'debug@2.6.9:\n",
            "a malformed package key",
        ),
        // An empty version after the `@`.
        (
            "lockfileVersion: '9.0'\npackages:\n  a@:\n    resolution: {integrity: sha512-X}\n",
            "'a@': empty version",
        ),
        // An unrecognized package child.
        (
            "lockfileVersion: '9.0'\npackages:\n  a@1.0.0:\n    rogue: x\n",
            "an unrecognized package child 'rogue'",
        ),
        (
            "lockfileVersion: '9.0'\npackages:\n  a@1.0.0:\n    rogue\n",
            "a package child that is not a mapping key",
        ),
        // An empty resolution integrity, and one whose quote never closes.
        (
            "lockfileVersion: '9.0'\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: }\n",
            "a malformed resolution flow map",
        ),
        (
            "lockfileVersion: '9.0'\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: 'sha512-X}\n",
            "a malformed resolution flow map",
        ),
        // The SUBSTRING match this reader no longer makes: `xintegrity`
        // and `fakeintegrity` answered for `integrity`, so a changed
        // actual integrity left identity unmoved.
        (
            "lockfileVersion: '9.0'\npackages:\n\n  debug@2.6.9:\n    resolution: {xintegrity: sha512-X}\n",
            "a malformed resolution flow map",
        ),
        (
            "lockfileVersion: '9.0'\npackages:\n\n  debug@2.6.9:\n    resolution: {fakeintegrity: sha512-X, tarball: x}\n",
            "a malformed resolution flow map",
        ),
        // A quote INSIDE a closed scalar, and a stray quote inside a bare
        // one: neither is unwrapped into something plausible.
        (
            "lockfileVersion: '9.0'\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: 'sha512'X'}\n",
            "a malformed resolution flow map",
        ),
        (
            "lockfileVersion: '9.0'\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X'}\n",
            "a malformed resolution flow map",
        ),
        // A nested flow map, a CLOSING brace with no opener inside the
        // map, and a field that is no mapping at all.
        (
            "lockfileVersion: '9.0'\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: {a: b}}\n",
            "a malformed resolution flow map",
        ),
        (
            "lockfileVersion: '9.0'\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: a}b}\n",
            "a malformed resolution flow map",
        ),
        (
            "lockfileVersion: '9.0'\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity}\n",
            "a malformed resolution flow map",
        ),
        // Two integrity fields in ONE flow map: neither is the record's.
        // Every field is a singleton, not only the one read — a second
        // `tarball` is two origins for one record.
        (
            "lockfileVersion: '9.0'\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X, integrity: sha512-Y}\n",
            "a repeated resolution field 'integrity'",
        ),
        (
            "lockfileVersion: '9.0'\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X, tarball: a, tarball: b}\n",
            "a repeated resolution field 'tarball'",
        ),
        // A child outside any record.
        (
            "lockfileVersion: '9.0'\npackages:\n    resolution: {integrity: sha512-X}\n",
            "a package child outside any record",
        ),
        // An entry with no resolution row at all.
        (
            "lockfileVersion: '9.0'\npackages:\n  a@1.0.0:\n    engines: {node: '>=1'}\n",
            "'a@1.0.0': no resolution integrity",
        ),
    ] {
        assert_eq!(
            refused_vector(pnpm_dependencies(text, &[]), text),
            format!("pnpm lock is unreadable: {reason}"),
            "{text:?}"
        );
    }
}

/// A YAML mapping key is a SINGLETON, and each recognized top-level key
/// has exactly ONE admissible form.
///
/// The inherited reader tracked no key it had already seen and decided
/// the form from the line: a key carrying an inline value was a value, a
/// key without one opened a section. So after a first `packages:` block
/// had been read, a second `packages: null`, a second `packages: {}`, a
/// second block-form `packages:` and a late `lockfileVersion: '8.0'` were
/// each skipped, and the FIRST block's triples stood as the document's
/// answer — an ambiguous lock reporting an unchanged identity (council
/// return 2026-09-19, finding 1).
#[test]
fn a_pnpm_document_key_is_a_singleton_and_its_form_is_fixed() {
    // The positive control every closed arm below is measured against:
    // both scalar-valued top-level keys carry their scalar, every
    // block-valued key opens a block, and the one `packages:` section
    // yields its triple.
    let read = "lockfileVersion: '9.0'\npackageExtensionsChecksum: sha256-abc\npnpmfileChecksum: sha256-def\nsettings:\n  autoInstallPeers: false\npackages:\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\nsnapshots:\n  debug@2.6.9: {}\n";
    assert_eq!(
        pnpm_dependencies(read, &[]).unwrap(),
        vec!["debug 2.6.9 sha512-X".to_string()]
    );
    // The exact document the defect was measured on: a complete first
    // `packages:` block, then a second spelling of the same key. The
    // inherited reader answered `["a 1.0.0 sha512-X"]` for all four.
    let after = |second: &str| {
        format!("lockfileVersion: '9.0'\npackages:\n  a@1.0.0:\n    resolution: {{integrity: sha512-X}}\n{second}\n")
    };
    for second in [
        "packages: null",
        "packages: {}",
        "packages:\n  b@2.0.0:\n    resolution: {integrity: sha512-Y}",
    ] {
        assert_eq!(
            refused_vector(pnpm_dependencies(&after(second), &[]), second),
            "pnpm lock is unreadable: a repeated top-level key 'packages'",
            "{second:?}"
        );
    }
    assert_eq!(
        refused(pnpm_dependencies(&after("lockfileVersion: '8.0'"), &[])),
        "pnpm lock is unreadable: a repeated top-level key 'lockfileVersion'"
    );
    // A repeat is a repeat even when the second spelling agrees with the
    // first: the refusal is about the document having one meaning, not
    // about the two values differing.
    assert_eq!(
        refused(pnpm_dependencies(&after("lockfileVersion: '9.0'"), &[])),
        "pnpm lock is unreadable: a repeated top-level key 'lockfileVersion'"
    );
    for (text, reason) in [
        // A block-valued key carrying an inline value, with no earlier
        // spelling to be a repeat of: the form alone refuses, so
        // `packages: null` is never a lock with an empty package set.
        (
            "lockfileVersion: '9.0'\npackages: null\n",
            "an inline value on top-level section 'packages'",
        ),
        (
            "lockfileVersion: '9.0'\npackages: {}\n",
            "an inline value on top-level section 'packages'",
        ),
        (
            "lockfileVersion: '9.0'\nimporters: []\npackages:\n",
            "an inline value on top-level section 'importers'",
        ),
        (
            "lockfileVersion: '9.0'\nsettings: on\npackages:\n",
            "an inline value on top-level section 'settings'",
        ),
        // A repeated block-valued key that is not `packages`.
        (
            "lockfileVersion: '9.0'\nsettings:\n  autoInstallPeers: false\nsettings:\n  x: y\npackages:\n",
            "a repeated top-level key 'settings'",
        ),
        // A scalar-valued key opening a block, and one whose scalar this
        // grammar cannot read: an alias and an unterminated quote.
        (
            "lockfileVersion: '9.0'\npnpmfileChecksum:\n  a: b\npackages:\n",
            "a malformed scalar for top-level key 'pnpmfileChecksum'",
        ),
        (
            "lockfileVersion: '9.0'\npackageExtensionsChecksum: 'sha256-x\npackages:\n",
            "a malformed scalar for top-level key 'packageExtensionsChecksum'",
        ),
        (
            "lockfileVersion: '9.0'\npackageExtensionsChecksum: *checksum\npackages:\n",
            "a malformed scalar for top-level key 'packageExtensionsChecksum'",
        ),
    ] {
        assert_eq!(
            refused_vector(pnpm_dependencies(text, &[]), text),
            format!("pnpm lock is unreadable: {reason}"),
            "{text:?}"
        );
    }
}

/// A pnpm scalar is one of exactly three forms, and never the YAML
/// SYNTAX that denotes a value elsewhere in the document.
///
/// The inherited check unwrapped matched quotes and otherwise passed the
/// text through, so `*undefined` (an alias this reader never follows),
/// `[sha512-X]` (a one-element sequence), `&anchor`, `!!str` and a
/// double-quoted `"A"` escape all entered identity as their own
/// spelling. An alias target or a sequence member could then change
/// while the hashed bytes did not, and a `"a\tb"` escape smuggled
/// whitespace past the scalar rule that exists to refuse it (council
/// return 2026-09-19, finding 2).
#[test]
fn a_pnpm_scalar_is_one_of_three_forms_and_never_yaml_syntax() {
    let resolution = |body: &str| {
        format!("lockfileVersion: '9.0'\npackages:\n  debug@2.6.9:\n    resolution: {{{body}}}\n")
    };
    // The three admitted forms, each proved by the VALUE it contributes
    // rather than by being accepted: a plain scalar carrying an inner
    // `:`, a single-quoted one and a double-quoted one all reach the
    // same dependency line with their quotes removed.
    for body in [
        "integrity: sha512-X, tarball: file:../x.tgz",
        "integrity: 'sha512-X'",
        "integrity: \"sha512-X\"",
    ] {
        assert_eq!(
            pnpm_dependencies(&resolution(body), &[]).unwrap(),
            vec!["debug 2.6.9 sha512-X".to_string()],
            "{body:?}"
        );
    }
    // A double-quoted lockfileVersion is the same admitted form at the
    // document header.
    assert_eq!(
        pnpm_dependencies(
            "lockfileVersion: \"9.0\"\npackages:\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n",
            &[]
        )
        .unwrap(),
        vec!["debug 2.6.9 sha512-X".to_string()]
    );
    // Every YAML form that is not a scalar this reader may hash. Each is
    // a distinct piece of syntax the inherited check took literally.
    for value in [
        // An alias, whose target this reader never follows.
        "*undefined",
        // A one-element flow sequence.
        "[sha512-X]",
        // An anchor, a tag, and the two block-scalar headers.
        "&integrity",
        "!!str",
        "|",
        ">-",
        // The reserved indicators and the remaining flow punctuation.
        "%YAML",
        "@reserved",
        "`reserved",
        ",sha512-X",
        "?sha512-X",
        ":sha512-X",
        "]sha512-X",
        "-sha512-X",
        // A comment introducer, which ends a plain scalar in YAML and
        // was hashed as part of it here.
        "sha512-X #note",
        // A double-quoted escape: the bytes and the value differ, and
        // `\\t` is whitespace the scalar rule must never see repaired
        // into an identity line.
        "\"sha512-\\u0041\"",
        "\"sha512-\\tX\"",
        // A double quote inside a closed double-quoted scalar, and one
        // whose partner never closes it.
        "\"sha512\"X\"",
        "\"sha512-X",
        // A stray double quote in a plain scalar.
        "sha512-X\"",
    ] {
        assert_eq!(
            refused_vector(
                pnpm_dependencies(&resolution(&format!("integrity: {value}")), &[]),
                value
            ),
            "pnpm lock is unreadable: a malformed resolution flow map",
            "{value:?}"
        );
    }
    // D6 admits an integrity with an OPTIONAL tarball and nothing else.
    // The inherited vocabulary also named these, then ignored them, so a
    // record resolving from a git commit or a local directory was read
    // as though it had come from the registry.
    for field in [
        "directory: /tmp/x",
        "path: ../x",
        "repo: git@example.invalid",
        "type: git",
        "commit: 0123456789abcdef",
        "registry: https://example.invalid/",
    ] {
        assert_eq!(
            refused_vector(
                pnpm_dependencies(&resolution(&format!("integrity: sha512-X, {field}")), &[]),
                field
            ),
            "pnpm lock is unreadable: a malformed resolution flow map",
            "{field:?}"
        );
    }
}

#[test]
fn read_json_and_the_retained_head_report_io_and_encoding_failures() {
    let absent = Path::new("/definitely/not/a/file");
    // The JSON reader answers with a REASON and no component: the same
    // failure is the layout's for a manifest and the npm lock's for the
    // hidden lock, and only the caller knows which file it asked for
    // (council return 2026-09-19, F9).
    let reason = read_json(absent).unwrap_err();
    assert!(
        reason.starts_with("/definitely/not/a/file: "),
        "the reason names the file it could not read: {reason}"
    );
    let dir = tempfile::tempdir().unwrap();
    let bad = dir.path().join("bad.json");
    fs::write(&bad, b"not json").unwrap();
    let reason = read_json(&bad).unwrap_err();
    assert!(
        reason.starts_with(&format!("{}: not JSON: ", bad.display())),
        "{reason}"
    );
    // A CRLF file's first line CARRIES its carriage return. The kernel
    // reads `#!/usr/bin/env node\r` as an interpreter name that does not
    // exist, so removing the CR here reported a file the loader cannot
    // execute as the qualified one (council return 2026-09-19, F5).
    //
    // The line is read from the head SELECTION retained, never from the
    // file: `first_line` opens nothing, so the path it is handed serves
    // only to name the file in a refusal (review 2026-09-20, F6).
    let crlf = dir.path().join("crlf");
    fs::write(&crlf, b"#!/usr/bin/env node\r\nrest\n").unwrap();
    assert_eq!(
        first_line(&crlf, &selected_head(&crlf)).unwrap(),
        "#!/usr/bin/env node\r"
    );
    let raw = dir.path().join("raw");
    fs::write(&raw, b"\xff\n").unwrap();
    assert_eq!(
        refused(first_line(&raw, &selected_head(&raw))),
        format!(
            "the DSH layout is unreadable: {}: first line is not UTF-8",
            raw.display()
        )
    );
    // A head with no terminator in it is the line: the bound reached, or
    // a file shorter than one line.
    let unterminated = dir.path().join("unterminated");
    fs::write(&unterminated, b"#!/usr/bin/env node").unwrap();
    assert_eq!(
        first_line(&unterminated, &selected_head(&unterminated)).unwrap(),
        "#!/usr/bin/env node"
    );
    // And the file's own bytes are never consulted: a head handed for a
    // path that does not exist is still the answer, which is what makes
    // a post-probe reread impossible rather than merely unlikely.
    assert_eq!(
        first_line(absent, b"#!/usr/bin/env node\nrest\n").unwrap(),
        "#!/usr/bin/env node"
    );
}

/// `home-patch` distinguishes true absence from a failed observation.
/// Only a missing path yields the literal `absent`; a dangling symlink
/// and an unreachable parent are both unreadable, named by their own
/// component (design D10's scenario).
#[test]
fn a_present_unreadable_home_patch_is_not_absence() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    fs::create_dir_all(&home).unwrap();
    assert_eq!(home_patch(&home).unwrap(), "absent");

    write(&home, "cordis.patch.yml", b"[]\n");
    assert_eq!(home_patch(&home).unwrap(), digest_of(b"[]\n"));

    // A home path that is a FILE: the patch lookup cannot even reach a
    // metadata answer, which is unreadable rather than absent.
    let file = dir.path().join("not-a-home");
    fs::write(&file, b"x").unwrap();
    let error = home_patch(&file).unwrap_err();
    assert!(
        error.to_string().starts_with("home-patch is unreadable:"),
        "{error}"
    );
}

// Unix only: a dangling symlink is the input under test.
#[cfg(unix)]
#[test]
fn a_dangling_home_patch_symlink_is_unreadable_rather_than_absent() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    fs::create_dir_all(&home).unwrap();
    std::os::unix::fs::symlink(home.join("nowhere"), home.join("cordis.patch.yml")).unwrap();
    let error = home_patch(&home).unwrap_err();
    assert!(
        error.to_string().starts_with("home-patch is unreadable:"),
        "{error}"
    );
}

/// An unreadable PROFILE patch is named as the profile's, never as
/// plugin drift: the two components fail for different reasons and an
/// operator acts on them differently.
#[test]
fn an_unreadable_profile_patch_is_not_a_plugin_component_failure() {
    let install = Synthetic::new();
    let patch = install.profile().join("cordis.patch.yml");
    fs::remove_file(&patch).unwrap();
    // The refusal names the profile-patch component, the file and the
    // platform's own reason — not "plugin", whose six-file walk reads a
    // `cordis.patch.yml` of its own.
    assert_eq!(
        refused(dsh_composite_with(&install.seams, &install.node(), &[])),
        format!(
            "profile-patch is unreadable: {}: {}",
            patch.display(),
            std::fs::read(&patch).unwrap_err()
        )
    );
}

#[test]
fn resolve_core_refuses_an_executable_with_no_dsh_ancestor() {
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("tool.js");
    write_executable(dir.path(), "tool.js", b"#!/usr/bin/env node\n");
    // The refusal names the CANONICAL executable and the package it
    // looked for, which is the whole of this arm: no ancestor manifest at
    // all, as distinct from one that names something else.
    assert_eq!(
        refused(resolve_core(&bin.to_string_lossy())),
        format!(
            "the DSH layout is unreadable: {}: no ancestor package.json names @deepseek-ai/dsh",
            bin.canonicalize().unwrap().display()
        )
    );
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

/// The core manifest and the hidden lock are each read ONCE and retained:
/// the `core` line's version and integrity and every npm triple come from
/// the same bytes. Removing either source after the read would be the
/// reopening this guards against, so the test deletes both and the same
/// observation still completes.
#[test]
fn the_core_manifest_and_hidden_lock_are_read_once_and_retained() {
    let install = Synthetic::new();
    let core = install.dir.path().join("core");
    let base = install.composite();

    // Both sources are removed AFTER `resolve_core` has read them: a
    // reopening anywhere below it would now fail, and the observation
    // would not complete at all.
    let reading = resolve_core(&install.seams.executable).unwrap();
    fs::remove_file(core.join("node_modules/.package-lock.json")).unwrap();
    fs::remove_file(core.join("node_modules/@deepseek-ai/dsh/package.json")).unwrap();
    assert_eq!(reading.version, "0.1.5-rc.2");
    assert_eq!(reading.integrity, "sha512-CORE");
    assert_eq!(
        npm_dependencies(&reading.lock, &["dsh-plugin-cli-session"]).unwrap(),
        vec!["debug 2.6.9 sha512-DEBUG".to_string()],
        "the triples come from the retained lock, not a second read"
    );
    assert_eq!(base.core, "@deepseek-ai/dsh 0.1.5-rc.2 sha512-CORE");

    // Counted, not asserted in prose. Discovery opens each ancestor
    // `package.json` it must inspect and the hidden lock once; the
    // SELECTED manifest and the lock are then read from the retained
    // values. A reopening of either shows up here as a second count.
    let opened: std::cell::RefCell<Vec<PathBuf>> = std::cell::RefCell::new(Vec::new());
    let counting = |path: &Path| {
        opened.borrow_mut().push(path.to_path_buf());
        read_json(path)
    };
    let install = Synthetic::new();
    let core = install.dir.path().join("core");
    resolve_core_reading(
        &install.seams.executable,
        &install.seams.head.clone(),
        &counting,
    )
    .unwrap();
    let opened = opened.into_inner();
    assert_eq!(
        opened
            .iter()
            .filter(|path| *path == &core.join("node_modules/@deepseek-ai/dsh/package.json"))
            .count(),
        1,
        "the selected core manifest is opened once: {opened:?}"
    );
    assert_eq!(
        opened
            .iter()
            .filter(|path| *path == &core.join("node_modules/.package-lock.json"))
            .count(),
        1,
        "the hidden lock is opened once and serves both the core line and \
         the dependency triples: {opened:?}"
    );

    // The same count over the WHOLE observation, not only over discovery.
    // A reopening of the lock for the dependency triples would happen
    // after `resolve_core` has returned, where the count above cannot see
    // it; this one can, because the reader is injected into the
    // composition itself.
    let opened: std::cell::RefCell<Vec<PathBuf>> = std::cell::RefCell::new(Vec::new());
    let counting = |path: &Path| {
        opened.borrow_mut().push(path.to_path_buf());
        read_json(path)
    };
    let install = Synthetic::new();
    let core = install.dir.path().join("core");
    let observed = dsh_composite_reading(
        &install.seams,
        &install.node(),
        &[],
        &read_dir_entries,
        &counting,
    )
    .unwrap();
    assert_eq!(observed.canonical, base.canonical);
    let opened = opened.into_inner();
    assert_eq!(
        opened
            .iter()
            .filter(|path| *path == &core.join("node_modules/.package-lock.json"))
            .count(),
        1,
        "the whole observation opens the hidden lock once: {opened:?}"
    );
    assert_eq!(
        opened
            .iter()
            .filter(|path| *path == &core.join("node_modules/@deepseek-ai/dsh/package.json"))
            .count(),
        1,
        "the whole observation opens the selected core manifest once: {opened:?}"
    );
}

/// The plugin's `cordis.patch.yml` digest is observed once, inside the
/// component walk, and reused as `plugin-patch`. One file cannot be seen
/// as two different things by one observation.
#[test]
fn the_plugin_patch_is_the_retained_component_digest() {
    let install = Synthetic::new();
    let observed = install.composite();
    let plugin_dir = install
        .profile()
        .join("node_modules")
        .join("dsh-plugin-cli-session");
    let digests =
        plugin_file_digests("plugin", &plugin_dir, &PLUGIN_FILES, &read_dir_entries).unwrap();
    assert_eq!(
        observed.plugin_patch, digests["cordis.patch.yml"],
        "plugin-patch is the retained digest of the file the component hashed"
    );
    assert_eq!(observed.plugin, component_digest(&digests));
}

/// `plugin-patch` is the digest the COMPONENT WALK took, not a second
/// read of the same path — and the difference is made observable rather
/// than asserted.
///
/// The injected listing rewrites `cordis.patch.yml` the moment the walk
/// has consumed and hashed it. A producer that reopened the file for the
/// `plugin-patch` line would hash the SECOND bytes; this one still
/// carries the first, which is what "one observation" means. Comparing
/// an unchanged patch after another read proved nothing, because both
/// readings agree on a file that never moved (council return
/// 2026-09-19, finding 4).
#[test]
fn the_plugin_patch_cannot_be_a_second_read_of_a_changed_file() {
    let install = Synthetic::new();
    let plugin = install
        .profile()
        .join("node_modules")
        .join("dsh-plugin-cli-session");
    let patch = plugin.join("cordis.patch.yml");
    let before = fs::read(&patch).unwrap();
    const AFTER: &[u8] = b"# rewritten the instant the walk moved on\n";
    assert_ne!(before.as_slice(), AFTER);

    // The listing is production's, in a fixed order, with one hook: once
    // `cordis.patch.yml` has been yielded — and therefore read and
    // hashed by the walk — the next pull rewrites it on disk.
    let rewriting = move |dir: &Path| -> std::io::Result<DirEntries> {
        let mut entries = std::fs::read_dir(dir)?.collect::<std::io::Result<Vec<_>>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        let mut yielded_patch = false;
        Ok(Box::new(entries.into_iter().map(move |entry| {
            if yielded_patch {
                fs::write(
                    entry.path().parent().unwrap().join("cordis.patch.yml"),
                    AFTER,
                )
                .unwrap();
            }
            yielded_patch = entry.file_name() == "cordis.patch.yml";
            Ok(entry)
        })))
    };
    let observed =
        dsh_composite_reading(&install.seams, &install.node(), &[], &rewriting, &read_json)
            .unwrap();

    // The rewrite really happened, so the two readings really differ.
    assert_eq!(fs::read(&patch).unwrap(), AFTER);
    assert_eq!(
        observed.plugin_patch,
        digest_of(&before),
        "plugin-patch is the walk's observation, not a later reopen"
    );
    assert_ne!(observed.plugin_patch, digest_of(AFTER));
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
    write_executable(bin.parent().unwrap(), "other.js", b"#!/usr/bin/env node\n");
    // Asserted by its reason and by BOTH paths it names: which file was
    // asked for and which one the manifest declares is the whole of the
    // refusal, and an `is_err()` carried neither.
    assert_eq!(
        refused(resolve_core(&other.to_string_lossy())),
        format!(
            "the DSH layout is unreadable: {} is not the core package's bin.dsh ({})",
            other.canonicalize().unwrap().display(),
            bin.canonicalize().unwrap().display()
        )
    );

    // A manifest with no `bin.dsh`.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("core");
    let bin = core_package(
        &root,
        br#"{"name":"@deepseek-ai/dsh","version":"1.0.0"}"#,
        CORE_LOCK,
    );
    assert_eq!(
        refused(resolve_core(&bin.to_string_lossy())),
        "the DSH layout is unreadable: core package has no bin.dsh"
    );

    // A manifest with no `version`. The refusal names the MANIFEST, not
    // the lock, which is the distinction the two `required_string`
    // callers exist to keep.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("core");
    let bin = core_package(
        &root,
        br#"{"name":"@deepseek-ai/dsh","bin":{"dsh":"lib/bin.js"}}"#,
        CORE_LOCK,
    );
    assert_eq!(
        refused(resolve_core(&bin.to_string_lossy())),
        "the DSH layout is unreadable: core package: missing string 'version'"
    );

    // A dsh manifest outside `@deepseek-ai`.
    let dir = tempfile::tempdir().unwrap();
    let pkg = dir.path().join("node_modules").join("dsh");
    write(
        &pkg,
        "package.json",
        br#"{"name":"@deepseek-ai/dsh","version":"1.0.0","bin":{"dsh":"lib/bin.js"}}"#,
    );
    write_executable(&pkg, "lib/bin.js", b"#!/usr/bin/env node\n");
    assert_eq!(
        refused(resolve_core(&pkg.join("lib/bin.js").to_string_lossy())),
        "the DSH layout is unreadable: core package is not under node_modules/@deepseek-ai"
    );

    // A scope whose parent is not `node_modules`.
    let dir = tempfile::tempdir().unwrap();
    let pkg = dir.path().join("x").join("@deepseek-ai").join("dsh");
    write(
        &pkg,
        "package.json",
        br#"{"name":"@deepseek-ai/dsh","version":"1.0.0","bin":{"dsh":"lib/bin.js"}}"#,
    );
    write_executable(&pkg, "lib/bin.js", b"#!/usr/bin/env node\n");
    assert_eq!(
        refused(resolve_core(&pkg.join("lib/bin.js").to_string_lossy())),
        "the DSH layout is unreadable: core package is not under node_modules"
    );

    // A renamed package directory under `@deepseek-ai`.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("core");
    let pkg = root.join("node_modules").join("@deepseek-ai").join("other");
    write(
        &pkg,
        "package.json",
        br#"{"name":"@deepseek-ai/dsh","version":"1.0.0","bin":{"dsh":"lib/bin.js"}}"#,
    );
    write_executable(&pkg, "lib/bin.js", b"#!/usr/bin/env node\n");
    assert_eq!(
        refused(resolve_core(&pkg.join("lib/bin.js").to_string_lossy())),
        "the DSH layout is unreadable: core package is not at \
         <core root>/node_modules/@deepseek-ai/dsh"
    );
}

#[test]
fn resolve_core_refuses_a_lock_that_disagrees_with_the_package() {
    // The hidden lock has no dsh entry. The refusal is the NPM LOCK's,
    // not the layout's: an operator told "the DSH layout is unreadable"
    // looks at directories, and the drifted file is a lock.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("core");
    let bin = core_package(
        &root,
        br#"{"name":"@deepseek-ai/dsh","version":"1.0.0","bin":{"dsh":"lib/bin.js"}}"#,
        br#"{"packages":{}}"#,
    );
    assert_eq!(
        refused(resolve_core(&bin.to_string_lossy())),
        "npm lock is unreadable: no node_modules/@deepseek-ai/dsh entry"
    );

    // The lock version differs from the package version, and the
    // refusal carries BOTH so the operator can see which moved.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("core");
    let bin = core_package(
        &root,
        br#"{"name":"@deepseek-ai/dsh","version":"1.0.0","bin":{"dsh":"lib/bin.js"}}"#,
        br#"{"packages":{"node_modules/@deepseek-ai/dsh":{"version":"2.0.0","integrity":"sha512-X"}}}"#,
    );
    assert_eq!(
        refused(resolve_core(&bin.to_string_lossy())),
        "the DSH layout is unreadable: core lock version 2.0.0 differs from package version 1.0.0"
    );

    // The lock integrity carries a newline, and then a NUL: the `core`
    // component names itself in each refusal.
    for (integrity, reason) in [
        ("a\\nb", "carries whitespace"),
        ("a\\u0000b", "carries a NUL"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("core");
        let bin = core_package(
            &root,
            br#"{"name":"@deepseek-ai/dsh","version":"1.0.0","bin":{"dsh":"lib/bin.js"}}"#,
            format!(
                r#"{{"packages":{{"node_modules/@deepseek-ai/dsh":{{"version":"1.0.0","integrity":"{integrity}"}}}}}}"#
            )
            .as_bytes(),
        );
        assert_eq!(
            refused(resolve_core(&bin.to_string_lossy())),
            format!("the core value {reason}"),
            "{integrity}"
        );
    }

    // A core VERSION carrying whitespace is the same rule on the other
    // scalar of the same line.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("core");
    let bin = core_package(
        &root,
        br#"{"name":"@deepseek-ai/dsh","version":"1.0 0","bin":{"dsh":"lib/bin.js"}}"#,
        br#"{"packages":{"node_modules/@deepseek-ai/dsh":{"version":"1.0 0","integrity":"sha512-X"}}}"#,
    );
    assert_eq!(
        refused(resolve_core(&bin.to_string_lossy())),
        "the core value carries whitespace"
    );
}

#[test]
fn read_profile_refuses_a_missing_manifest_bundles_and_reload() {
    // No profile directory: the CANONICALIZATION of the boundary fails
    // first, before any manifest is read, which is the order that keeps
    // containment from ever comparing a raw path.
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("profiles").join("headless");
    assert_eq!(
        refused(read_profile(dir.path())),
        format!(
            "the DSH layout is unreadable: {}: {}",
            missing.display(),
            std::fs::canonicalize(&missing).unwrap_err()
        )
    );

    // No `dsh.profile`.
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    write(
        &home.join("profiles/headless"),
        "package.json",
        br#"{"dsh":{}}"#,
    );
    assert_eq!(
        refused(read_profile(&home)),
        "the DSH layout is unreadable: profile manifest has no dsh.profile"
    );

    // An empty bundle array, and an array entry that is not a string:
    // separate refusals, so a manifest that lost its bundles is never
    // reported as one that spelled a bundle wrong.
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    write(
        &home.join("profiles/headless"),
        "package.json",
        br#"{"dsh":{"profile":{"bundles":[],"patchReload":"startup"}}}"#,
    );
    assert_eq!(
        refused(read_profile(&home)),
        "the DSH layout is unreadable: dsh.profile.bundles must be a non-empty array"
    );

    // A bundle entry that is not a string at all.
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    write(
        &home.join("profiles/headless"),
        "package.json",
        br#"{"dsh":{"profile":{"bundles":[5],"patchReload":"startup"}}}"#,
    );
    assert_eq!(
        refused(read_profile(&home)),
        "the DSH layout is unreadable: a dsh.profile.bundles entry is not a string"
    );

    // A bundle name the scalar rule refuses, named by its component AND
    // by the rule it broke: "is empty" and "carries whitespace" are
    // different defects in the profile manifest.
    for (bad, reason) in [("", "is empty"), ("a b", "carries whitespace")] {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        write(
            &home.join("profiles/headless"),
            "package.json",
            format!(r#"{{"dsh":{{"profile":{{"bundles":["{bad}"],"patchReload":"startup"}}}}}}"#)
                .as_bytes(),
        );
        assert_eq!(
            refused(read_profile(&home)),
            format!("the profile-bundle value {reason}"),
            "{bad:?}"
        );
    }

    // A patchReload outside the closed pair.
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    write(
        &home.join("profiles/headless"),
        "package.json",
        br#"{"dsh":{"profile":{"bundles":["x"],"patchReload":"other"}}}"#,
    );
    assert_eq!(
        refused(read_profile(&home)),
        "the DSH layout is unreadable: dsh.profile.patchReload 'other' is neither live nor startup"
    );

    // `package.json` is a directory: read_json reports the READ error,
    // with the path, rather than a parse failure.
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let manifest = home.join("profiles/headless/package.json");
    fs::create_dir_all(&manifest).unwrap();
    assert_eq!(
        refused(read_profile(&home)),
        format!(
            "the DSH layout is unreadable: {}: {}",
            manifest.display(),
            std::fs::read(&manifest).unwrap_err()
        )
    );
}

/// The core lock is the HIDDEN `<core root>/node_modules/.package-lock.json`
/// and nothing else. D6's locator rule admits no root-lock fallback, so a
/// `<core root>/package-lock.json` sitting beside it is never read — not
/// as a source of triples, and not as a substitute when the hidden lock
/// is gone (council return 2026-09-19, finding 4: the missing
/// hidden-lock-versus-root-lock control).
#[test]
fn the_root_package_lock_is_never_read_beside_or_instead_of_the_hidden_one() {
    let install = Synthetic::new();
    let core_root = install.dir.path().join("core");
    // A decoy root lock naming the core at ITS OWN version with a
    // different integrity, and a dependency that exists nowhere else. The
    // version agrees on purpose: a decoy at another version would be
    // refused by the version check rather than read into identity, and
    // then a reader that preferred the root lock would fail this test by
    // refusing instead of by reporting the decoy's integrity. Either
    // reaching identity is visible in the lines below.
    let decoy = br#"{"lockfileVersion":3,"packages":{
      "node_modules/@deepseek-ai/dsh":{"version":"0.1.5-rc.2","integrity":"sha512-DECOY"},
      "node_modules/decoy":{"version":"9.9.9","integrity":"sha512-DECOY"}
    }}"#;
    write(&core_root, "package-lock.json", decoy);
    let observed = install.composite();
    assert_eq!(
        observed.core, "@deepseek-ai/dsh 0.1.5-rc.2 sha512-CORE",
        "the core line comes from the hidden lock, not the root one"
    );
    assert!(
        !observed
            .dependencies
            .iter()
            .any(|line| line.contains("decoy") || line.contains("DECOY")),
        "no root-lock triple enters the dependency lines: {:?}",
        observed.dependencies
    );

    // And with the hidden lock removed, the root lock beside it is not a
    // fallback: the refusal names the hidden locator.
    let hidden = core_root.join("node_modules").join(".package-lock.json");
    fs::remove_file(&hidden).unwrap();
    let absent = format!(
        "npm lock is unreadable: {}: {}",
        hidden.display(),
        // The host's own wording for the absent path: the exact
        // filesystem cause is the platform's to phrase, and only the
        // component and the locator are this producer's (Linux and
        // macOS, decision 0063).
        std::fs::read(&hidden).unwrap_err()
    );
    assert_eq!(
        refused(dsh_composite_with(&install.seams, &install.node(), &[])),
        absent
    );

    // The same absence beside a root lock in the full npm shape, whose
    // `""` root entry the key rule refuses. The distinction the decoy
    // above cannot draw: a reader that FELL BACK to this file would
    // refuse too, but with `'': empty, absolute, trailing…` — a refusal
    // naming the root lock's own first entry. The refusal is still the
    // hidden lock's absence, so the root lock was never opened.
    let rooted = r#"{"name":"dsh-home","version":"0.0.0","lockfileVersion":3,"requires":true,"packages":{
      "":{"name":"dsh-home","version":"0.0.0","dependencies":{"decoy":"9.9.9"}},
      "node_modules/decoy":{"version":"9.9.9","integrity":"sha512-DECOY"}
    }}"#;
    assert_eq!(
        refused(npm_dependencies(&lock(rooted), &[])),
        "npm key is unreadable: '': empty, absolute, trailing, or carries a forbidden byte",
        "read as a lock, this file refuses by its own root entry — which is \
         how a fallback would be visible below"
    );
    write(&core_root, "package-lock.json", rooted.as_bytes());
    assert_eq!(
        refused(dsh_composite_with(&install.seams, &install.node(), &[])),
        absent,
        "the sole npm source is the hidden lock, and its absence is not \
         cured by a root lock in any shape"
    );
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
    let error = dsh_composite_with(&install.seams, &install.node(), &[]).unwrap_err();
    assert_eq!(
        error.to_string(),
        "the DSH layout is unreadable: the plugin resolves outside the profile"
    );

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
    let error = dsh_composite_with(&install.seams, &install.node(), &[]).unwrap_err();
    assert_eq!(
        error.to_string(),
        "the DSH layout is unreadable: the extension resolves outside the profile"
    );
}

#[test]
fn dsh_composite_with_propagates_a_lock_refusal() {
    // A non-excluded lock entry with no integrity.
    let install = Synthetic::new();
    write(
        &install.dir.path().join("core"),
        "node_modules/.package-lock.json",
        br#"{"lockfileVersion":3,"packages":{"node_modules/@deepseek-ai/dsh":{"version":"0.1.5-rc.2","integrity":"sha512-CORE"},"node_modules/debug":{"version":"2.6.9"}}}"#,
    );
    let error = dsh_composite_with(&install.seams, &install.node(), &[]).unwrap_err();
    assert_eq!(
        error.to_string(),
        "npm lock is unreadable: 'node_modules/debug': no registry 'integrity'"
    );

    // An unreadable pnpm lock is the pnpm component, not a generic
    // configuration failure.
    let install = Synthetic::new();
    fs::remove_file(install.profile().join("pnpm-lock.yaml")).unwrap();
    let error = dsh_composite_with(&install.seams, &install.node(), &[]).unwrap_err();
    assert!(
        error.to_string().starts_with("pnpm lock is unreadable:"),
        "{error}"
    );
}

// ---------------------------------------------------------------------------
// The worked lock-dialect vectors (Pass D, D1).
//
// One worked vector per dialect — npm's lockfile-3 hidden lock and pnpm's
// lockfile 9.0 — read through the SOLE PRODUCER at D6's locators, pinning
// the normalized dependency value bytes and the canonical composite's byte
// form.
//
// Both vectors are SYNTHETIC excerpts written in the measured grammar, and
// they are deterministic planner and storage shims: they are not live DSH
// compatibility, qualification or enforcement evidence, and they make no
// claim about the measured rc.2 tree, whose own literal bytes are the
// fixture below. The measured hidden lock reaches two package groups; the
// three-group key here is grammar coverage, not a deeper measured tree.
// ---------------------------------------------------------------------------

/// The worked npm vector: one lockfile-3 hidden lock carrying every key
/// spelling D6's rule admits, both exclusions, and the four normalization
/// outcomes (collapse, two versions, two integrities, a nested entry whose
/// own bytes differ from its shallower namesake's).
///
/// Only ONE entry carries the optional `name` field, and it disagrees with
/// its key: the terminal spelling is the name, and an entry's own
/// `version` and `integrity` are the triple's, never an ancestor's, the
/// optional name's, a manifest's or a URL's.
const WORKED_NPM_LOCK: &str = r#"{"lockfileVersion":3,"packages":{
  "node_modules/@deepseek-ai/dsh":{"version":"0.1.5-rc.2","integrity":"sha512-CORE"},
  "node_modules/dsh-plugin-cli-session":{"version":"0.2.0","resolved":"file:plugin.tgz","link":true},
  "node_modules/debug":{"version":"2.6.9","integrity":"sha512-DEBUG"},
  "node_modules/express/node_modules/debug":{"version":"4.4.3","integrity":"sha512-DEEP"},
  "node_modules/@scope/child":{"version":"1.0.0","integrity":"sha512-ONE"},
  "node_modules/a/node_modules/@scope/child":{"name":"@wrong/spelling","version":"1.1.0","integrity":"sha512-TWO"},
  "node_modules/@parent/b/node_modules/@scope/child":{"version":"1.2.0","integrity":"sha512-THREE"},
  "node_modules/a/node_modules/@parent/b/node_modules/@scope/child":{"version":"2.0.0","integrity":"sha512-FOUR"},
  "node_modules/@parent/b/node_modules/plain":{"version":"3.0.0","integrity":"sha512-PLAIN"},
  "node_modules/x/node_modules/dup":{"version":"1.0.0","integrity":"sha512-DUP"},
  "node_modules/y/node_modules/dup":{"version":"1.0.0","integrity":"sha512-DUP"},
  "node_modules/split":{"version":"1.0.0","integrity":"sha512-SAME"},
  "node_modules/z/node_modules/split":{"version":"1.0.0","integrity":"sha512-OTHER"},
  "node_modules/versions":{"version":"1.0.0","integrity":"sha512-V"},
  "node_modules/w/node_modules/versions":{"version":"2.0.0","integrity":"sha512-V"}
}}"#;

/// The worked pnpm vector: three records equivalent to npm ones — two of
/// them the npm vector's nested entries — one record this dialect alone
/// supplies, and the plugin's local tarball record beside its same-named
/// registry record, so the exclusion is proved to identify a RECORD.
const WORKED_PNPM_LOCK: &str = "lockfileVersion: '9.0'\n\npackages:\n\n  \
    debug@2.6.9:\n    resolution: {integrity: sha512-DEBUG}\n\n  \
    debug@4.4.3:\n    resolution: {integrity: sha512-DEEP, tarball: https://example.invalid/d.tgz}\n\n  \
    '@scope/child@1.0.0':\n    resolution: {integrity: sha512-ONE}\n\n  \
    '@pnpm/only@0.3.1':\n    resolution: {integrity: sha512-PNPM}\n\n  \
    dsh-plugin-cli-session@file:./plugin.tgz:\n    \
    resolution: {integrity: sha512-LOCAL, tarball: file:./plugin.tgz}\n    version: 0.2.0\n\n  \
    dsh-plugin-cli-session@0.2.0:\n    resolution: {integrity: sha512-REG}\n";

/// Every key spelling the npm vector carries, with the terminal package
/// the rule takes from it and that entry's OWN version and integrity.
///
/// Top-level and nested unscoped names; a top-level scoped name; scoped
/// names under an unscoped and under a scoped parent; a scoped parent
/// with an unscoped terminal; and three successive groups whose terminal
/// `@scope/child` carries a version and integrity distinct from both
/// shallower `@scope/child` entries.
const WORKED_NPM_KEY_CASES: [(&str, &str, &str, &str); 11] = [
    ("node_modules/debug", "debug", "2.6.9", "sha512-DEBUG"),
    (
        "node_modules/express/node_modules/debug",
        "debug",
        "4.4.3",
        "sha512-DEEP",
    ),
    (
        "node_modules/@scope/child",
        "@scope/child",
        "1.0.0",
        "sha512-ONE",
    ),
    (
        "node_modules/a/node_modules/@scope/child",
        "@scope/child",
        "1.1.0",
        "sha512-TWO",
    ),
    (
        "node_modules/@parent/b/node_modules/@scope/child",
        "@scope/child",
        "1.2.0",
        "sha512-THREE",
    ),
    (
        "node_modules/a/node_modules/@parent/b/node_modules/@scope/child",
        "@scope/child",
        "2.0.0",
        "sha512-FOUR",
    ),
    (
        "node_modules/@parent/b/node_modules/plain",
        "plain",
        "3.0.0",
        "sha512-PLAIN",
    ),
    (
        "node_modules/x/node_modules/dup",
        "dup",
        "1.0.0",
        "sha512-DUP",
    ),
    (
        "node_modules/y/node_modules/dup",
        "dup",
        "1.0.0",
        "sha512-DUP",
    ),
    ("node_modules/split", "split", "1.0.0", "sha512-SAME"),
    (
        "node_modules/z/node_modules/split",
        "split",
        "1.0.0",
        "sha512-OTHER",
    ),
];

/// The complete ordered dependency values the two worked vectors yield:
/// the merged, deduplicated, bytewise-sorted set the serializer emits one
/// `dependency` line per. Fourteen values over ten names.
const WORKED_DEPENDENCIES: [&str; 14] = [
    "@pnpm/only 0.3.1 sha512-PNPM",
    "@scope/child 1.0.0 sha512-ONE",
    "@scope/child 1.1.0 sha512-TWO",
    "@scope/child 1.2.0 sha512-THREE",
    "@scope/child 2.0.0 sha512-FOUR",
    "debug 2.6.9 sha512-DEBUG",
    "debug 4.4.3 sha512-DEEP",
    "dsh-plugin-cli-session 0.2.0 sha512-REG",
    "dup 1.0.0 sha512-DUP",
    "plain 3.0.0 sha512-PLAIN",
    "split 1.0.0 sha512-OTHER",
    "split 1.0.0 sha512-SAME",
    "versions 1.0.0 sha512-V",
    "versions 2.0.0 sha512-V",
];

/// The synthetic install with both worked vectors written at D6's
/// locators, recomposed in place: ONE home, so a difference between two
/// observations is a difference between two LOCKS and nothing else. The
/// same pair staged at two absolute paths is a separate matrix.
fn recomposed(install: &Synthetic, npm: &str, pnpm: &str) -> DshComposite {
    write(
        &install.dir.path().join("core"),
        "node_modules/.package-lock.json",
        npm.as_bytes(),
    );
    write(&install.profile(), "pnpm-lock.yaml", pnpm.as_bytes());
    install.composite()
}

/// The npm dialect, read through the producer: every key spelling binds
/// to its terminal package, every group of every key is consumed, and the
/// optional `name` field is not the name.
#[test]
fn the_worked_npm_vector_binds_every_key_spelling_to_its_terminal_package() {
    let install = Synthetic::new();
    let observed = recomposed(&install, WORKED_NPM_LOCK, WORKED_PNPM_LOCK);

    for (key, name, version, integrity) in WORKED_NPM_KEY_CASES {
        // The rule's own answer, and the value the composition carried it
        // into: a key that bound correctly but reached no dependency line
        // would pass the first assertion alone.
        assert_eq!(npm_name(key).unwrap(), name, "{key}");
        let value = format!("{name} {version} {integrity}");
        assert!(
            observed.dependencies.contains(&value),
            "{key} yields {value:?}: {:?}",
            observed.dependencies
        );
    }

    // The vector really is what it claims: exactly one entry carries an
    // optional `name`, and that name is NOT its key's terminal spelling.
    // Every other entry has no `name` field at all.
    let parsed = lock(WORKED_NPM_LOCK);
    let named: Vec<(String, String)> = parsed["packages"]
        .as_object()
        .unwrap()
        .iter()
        .filter_map(|(key, entry)| {
            entry
                .get("name")
                .and_then(Value::as_str)
                .map(|name| (key.clone(), name.to_string()))
        })
        .collect();
    assert_eq!(
        named,
        vec![(
            "node_modules/a/node_modules/@scope/child".to_string(),
            "@wrong/spelling".to_string()
        )],
        "one conflicting optional name, and no other entry carries one"
    );
    assert!(
        !observed
            .dependencies
            .iter()
            .any(|value| value.contains("@wrong/spelling")),
        "the optional name reaches no dependency value: {:?}",
        observed.dependencies
    );

    // The four normalization outcomes, counted by name: one collapsed
    // pair, two versions of one name, two integrities of one name and
    // version, and a nested entry retained beside its shallower namesake.
    let values = |name: &str| -> Vec<&String> {
        observed
            .dependencies
            .iter()
            .filter(|value| value.starts_with(&format!("{name} ")))
            .collect()
    };
    assert_eq!(
        values("dup"),
        vec!["dup 1.0.0 sha512-DUP"],
        "two keys with the same complete triple are one value"
    );
    assert_eq!(
        values("versions"),
        vec!["versions 1.0.0 sha512-V", "versions 2.0.0 sha512-V"],
        "one name at two versions is two values"
    );
    assert_eq!(
        values("split"),
        vec!["split 1.0.0 sha512-OTHER", "split 1.0.0 sha512-SAME"],
        "one name and version at two integrities is two values"
    );
    assert_eq!(
        values("debug"),
        vec!["debug 2.6.9 sha512-DEBUG", "debug 4.4.3 sha512-DEEP"],
        "a nested entry's own bytes are retained beside the shallower one's"
    );
    assert_eq!(
        values("@scope/child"),
        vec![
            "@scope/child 1.0.0 sha512-ONE",
            "@scope/child 1.1.0 sha512-TWO",
            "@scope/child 1.2.0 sha512-THREE",
            "@scope/child 2.0.0 sha512-FOUR",
        ],
        "the three-group terminal is distinct from both shallower entries"
    );

    // Both exclusions identify exact RECORDS: the core's own hidden-lock
    // key and the plugin's local `file:` record leave, and the plugin's
    // same-named pnpm registry record stays.
    assert!(
        !observed
            .dependencies
            .iter()
            .any(|value| value.starts_with("@deepseek-ai/dsh ")),
        "the core's own record is not a dependency: {:?}",
        observed.dependencies
    );
    assert_eq!(
        values("dsh-plugin-cli-session"),
        vec!["dsh-plugin-cli-session 0.2.0 sha512-REG"],
        "the local tarball record leaves and the registry record stays"
    );
}

/// The canonical composite's BYTE FORM over the two worked vectors.
///
/// `WORKED_CANONICAL_STREAM` is a frozen literal, not a computation: it
/// is D6's `<component>\0<value>\n` form written out by hand, component
/// name for component name, in the fixed order, with the final newline.
/// It assembles nothing from the producer's output and cannot follow it —
/// reordering the lines, dropping one, dropping the trailing newline or
/// changing a separator in production leaves this literal behind and the
/// digests unequal. That is what makes it an expectation rather than the
/// second serializer `no_test_reassembles_the_component_stream` forbids.
///
/// `WORKED_CANONICAL` is the producer's own pinned output over the same
/// inputs, so the form and the result are pinned independently.
#[test]
fn the_worked_vectors_pin_the_canonical_composite_byte_form() {
    let install = Synthetic::new();
    let observed = recomposed(&install, WORKED_NPM_LOCK, WORKED_PNPM_LOCK);

    // Every component value, as literal bytes. Each is one line of the
    // stream below, so a value that moved is named here before the digest
    // comparison reports only that something did.
    assert_eq!(observed.core, "@deepseek-ai/dsh 0.1.5-rc.2 sha512-CORE");
    assert_eq!(observed.node, "v22.23.2");
    assert_eq!(
        observed.dependencies,
        WORKED_DEPENDENCIES.map(str::to_string).to_vec(),
        "the complete ordered dependency values"
    );
    // A dependency value's shape, read off each value rather than
    // asserted of the list: name, ONE ASCII space, version, ONE ASCII
    // space, integrity — and no other whitespace anywhere, which is what
    // the one scalar rule buys the serializer.
    for value in &observed.dependencies {
        let fields: Vec<&str> = value.split(' ').collect();
        assert_eq!(fields.len(), 3, "{value:?} is three space-separated fields");
        for field in fields {
            assert!(!field.is_empty(), "{value:?} carries an empty field");
            assert!(
                !field.contains(char::is_whitespace),
                "{value:?} carries whitespace inside a field"
            );
        }
    }
    assert_eq!(observed.plugin, WORKED_PLUGIN_COMPONENT);
    assert_eq!(
        observed.plugin_patch,
        digest_of(b"cordis.patch.yml"),
        "the synthetic plugin's patch file carries its own name as bytes"
    );
    assert_eq!(observed.profile_patch, digest_of(b"[]\n"));
    assert_eq!(
        observed.profile_bundles,
        vec!["@deepseek-ai/dsh-base", "dsh-plugin-cli-session"],
        "the declared order"
    );
    assert_eq!(observed.profile_patch_reload, "startup");
    assert_eq!(observed.home_patch, "absent");
    assert_eq!(
        observed.extension, None,
        "this profile lists no extension, so the stream carries no extension line"
    );

    assert_eq!(
        observed.canonical, WORKED_CANONICAL,
        "the producer's pinned canonical composite over the worked vectors"
    );
    assert_eq!(
        digest_of(WORKED_CANONICAL_STREAM.as_bytes()),
        observed.canonical,
        "the canonical composite is the SHA-256 of exactly these component \
         lines, in this order, ending with a newline"
    );
}

/// The producer's pinned canonical composite over the two worked vectors.
const WORKED_CANONICAL: &str = "2b22346b648577b4ad3776fe926935a2506b5f35fe3df9a732a8b4bda7a466c9";

/// The plugin component over the synthetic plugin set, whose six files
/// each carry their own relative name as their bytes.
const WORKED_PLUGIN_COMPONENT: &str =
    "8894f23eef97b42abfda88b6dd42c4b44b17ac4cb7e6df14c6bda687ae534c99";

/// D6's component stream for the worked vectors, written out by hand:
/// `core`, `node`, the bytewise-sorted complete `dependency` values,
/// `plugin`, `plugin-patch`, `profile-patch`, the declared-order
/// `profile-bundle` rows, `profile-patch-reload`, `home-patch`, and no
/// `extension` line, each as `<component>\0<value>\n`.
const WORKED_CANONICAL_STREAM: &str = "core\u{0}@deepseek-ai/dsh 0.1.5-rc.2 sha512-CORE\n\
     node\u{0}v22.23.2\n\
     dependency\u{0}@pnpm/only 0.3.1 sha512-PNPM\n\
     dependency\u{0}@scope/child 1.0.0 sha512-ONE\n\
     dependency\u{0}@scope/child 1.1.0 sha512-TWO\n\
     dependency\u{0}@scope/child 1.2.0 sha512-THREE\n\
     dependency\u{0}@scope/child 2.0.0 sha512-FOUR\n\
     dependency\u{0}debug 2.6.9 sha512-DEBUG\n\
     dependency\u{0}debug 4.4.3 sha512-DEEP\n\
     dependency\u{0}dsh-plugin-cli-session 0.2.0 sha512-REG\n\
     dependency\u{0}dup 1.0.0 sha512-DUP\n\
     dependency\u{0}plain 3.0.0 sha512-PLAIN\n\
     dependency\u{0}split 1.0.0 sha512-OTHER\n\
     dependency\u{0}split 1.0.0 sha512-SAME\n\
     dependency\u{0}versions 1.0.0 sha512-V\n\
     dependency\u{0}versions 2.0.0 sha512-V\n\
     plugin\u{0}8894f23eef97b42abfda88b6dd42c4b44b17ac4cb7e6df14c6bda687ae534c99\n\
     plugin-patch\u{0}66e6d923ac24b898cc4d8b405e107adfca86b017b7b63d31287a71549c1580bd\n\
     profile-patch\u{0}37517e5f3dc66819f61f5a7bb8ace1921282415f10551d2defa5c3eb0985b570\n\
     profile-bundle\u{0}@deepseek-ai/dsh-base\n\
     profile-bundle\u{0}dsh-plugin-cli-session\n\
     profile-patch-reload\u{0}startup\n\
     home-patch\u{0}absent\n";

/// Equivalent entries in the two dialects normalize to the SAME value
/// bytes, and a complete triple present in both is one dependency.
///
/// Proved by identity rather than by comparing two lists: the same home,
/// the same everything else, and the triple supplied by both locks, by
/// npm alone and by pnpm alone. All three compose to one canonical
/// composite. A pnpm record differing only in its integrity is the
/// control that the comparison is capable of moving at all.
#[test]
fn equivalent_npm_and_pnpm_entries_compose_to_one_dependency() {
    const NPM_ONLY: &str = r#"{"lockfileVersion":3,"packages":{
      "node_modules/@deepseek-ai/dsh":{"version":"0.1.5-rc.2","integrity":"sha512-CORE"},
      "node_modules/dsh-plugin-cli-session":{"version":"0.2.0","resolved":"file:plugin.tgz","link":true},
      "node_modules/debug":{"version":"2.6.9","integrity":"sha512-DEBUG"}
    }}"#;
    const NPM_BARE: &str = r#"{"lockfileVersion":3,"packages":{
      "node_modules/@deepseek-ai/dsh":{"version":"0.1.5-rc.2","integrity":"sha512-CORE"},
      "node_modules/dsh-plugin-cli-session":{"version":"0.2.0","resolved":"file:plugin.tgz","link":true}
    }}"#;
    const PNPM_ONLY: &str =
        "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-DEBUG}\n";
    const PNPM_BARE: &str = "lockfileVersion: '9.0'\n\npackages:\n";
    const PNPM_OTHER: &str =
        "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-OTHER}\n";

    let install = Synthetic::new();
    let both = recomposed(&install, NPM_ONLY, PNPM_ONLY);
    assert_eq!(both.dependencies, vec!["debug 2.6.9 sha512-DEBUG"]);
    let npm_only = recomposed(&install, NPM_ONLY, PNPM_BARE);
    let pnpm_only = recomposed(&install, NPM_BARE, PNPM_ONLY);
    assert_eq!(
        both.canonical, npm_only.canonical,
        "the pnpm record equal to the npm one adds no dependency line"
    );
    assert_eq!(
        both.canonical, pnpm_only.canonical,
        "either dialect alone normalizes that entry to the same bytes"
    );

    let differing = recomposed(&install, NPM_ONLY, PNPM_OTHER);
    assert_eq!(
        differing.dependencies,
        vec!["debug 2.6.9 sha512-DEBUG", "debug 2.6.9 sha512-OTHER"],
        "a pnpm record differing only in integrity is a second dependency"
    );
    assert_ne!(both.canonical, differing.canonical);
}

/// A key whose terminal package is perfectly good and whose INTERMEDIATE
/// group is not is refused, through the producer: every group of every
/// key is consumed and validated, so no identity is composed from a lock
/// this reader cannot spell.
#[test]
fn a_malformed_intermediate_group_refuses_the_whole_worked_lock() {
    let install = Synthetic::new();
    let base = recomposed(&install, WORKED_NPM_LOCK, WORKED_PNPM_LOCK);
    assert_eq!(base.canonical, WORKED_CANONICAL);

    let malformed = WORKED_NPM_LOCK.replace(
        "  \"node_modules/debug\":",
        "  \"node_modules/a/extra/node_modules/@scope/child\":{\"version\":\"9.9.9\",\"integrity\":\"sha512-EXTRA\"},\n  \"node_modules/debug\":",
    );
    assert!(
        malformed.contains("node_modules/a/extra/node_modules/@scope/child"),
        "the vector carries the malformed key"
    );
    write(
        &install.dir.path().join("core"),
        "node_modules/.package-lock.json",
        malformed.as_bytes(),
    );
    assert_eq!(
        refused(dsh_composite_with(&install.seams, &install.node(), &[])),
        "npm key is unreadable: 'node_modules/a/extra/node_modules/@scope/child': \
         expected a 'node_modules/' group"
    );
}

/// Each construct the pnpm grammar does not recognize, refused through
/// the PRODUCER: the reader's own vectors prove the reason, and these
/// prove that an unrecognized construct composes no identity at all.
#[test]
fn the_unrecognized_pnpm_constructs_refuse_through_the_producer() {
    let install = Synthetic::new();
    // The legal control: the same document, admitted, with its triple in
    // the composed dependency values. Every refusal below is a single
    // departure from a spelling this producer does read.
    let legal = composite_over_pnpm(
        &install,
        "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n",
    )
    .unwrap();
    assert!(legal
        .dependencies
        .contains(&"debug 2.6.9 sha512-X".to_string()));

    for (lock, reason) in [
        (
            "\tlockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n",
            "a tab",
        ),
        (
            "lockfileVersion: '9.0'\n# a comment\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n",
            "a comment or document marker",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  ---\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n",
            "a comment or document marker",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution:\n      integrity: sha512-X\n",
            "a block-form or malformed resolution",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    engines: {node: '>=1'}\n",
            "'debug@2.6.9': no resolution integrity",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug@2.6.9:\n    resolution: {integrity: sha512-X}\n    resolution: {integrity: sha512-Y}\n",
            "'debug@2.6.9': repeated resolution",
        ),
        (
            "lockfileVersion: '9.0'\n\npackages:\n\n  debug:\n    resolution: {integrity: sha512-X}\n",
            "'debug': no '@' after the first character",
        ),
    ] {
        assert_eq!(
            refused_vector(composite_over_pnpm(&install, lock), lock),
            format!("pnpm lock is unreadable: {reason}"),
            "{lock:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// The plugin component, equal staging, containment and conditional
// extension vectors (Pass D, D2).
//
// The plugin and extension byte sets below are SYNTHETIC. They are written
// in the declared SHAPE of the committed pair and they are not its bytes:
// the measured rc.2 fixture at the end of this file holds those unchanged,
// and `the_committed_plugin_set_is_the_six_files_and_the_one_expression_delta`
// above remains the only test that reads the repository's own adaptation.
// Like D1's lock vectors these are deterministic planner and storage shims,
// not live DSH compatibility, qualification or enforcement evidence. Every
// home is built under a canonicalized temporary root through `Synthetic`
// and `FixtureRoot`; nothing here reads `.forge/`, none of it needs an
// installed provider, and no repository extension is created to exercise a
// case.
// ---------------------------------------------------------------------------

/// The worked plugin vector: the six declared relative paths and the exact
/// bytes staged at each.
///
/// No two members share bytes, none carries its own relative path as its
/// content, and none matches a member of the worked extension set below —
/// so a walk that crossed two paths, reused one member's digest for
/// another or read an extension file where a plugin file belongs cannot
/// reach the pinned component.
const WORKED_PLUGIN_SET: [(&str, &[u8]); 6] = [
    ("LICENSE", b"MIT: the plugin's licence\n"),
    ("README.md", b"# dsh-plugin-cli-session\n"),
    ("cordis.patch.yml", b"- id: cli-session\n  config: {}\n"),
    ("lib/index.js", b"module.exports = { name: 'index' }\n"),
    ("lib/startup.js", b"module.exports = { name: 'startup' }\n"),
    (
        "package.json",
        b"{\"name\":\"dsh-plugin-cli-session\",\"version\":\"0.2.0\"}\n",
    ),
];

/// D6's component stream over the worked plugin set, written out by hand:
/// one `<relative path>\0<file SHA-256>\n` line per declared file, in
/// bytewise path order, ending with a newline.
///
/// It is a frozen literal, not a computation. It assembles nothing from
/// the producer's output and cannot follow it: reordering the walk's
/// lines, dropping a separator or dropping the final newline in production
/// leaves this literal behind. `read_component_stream` PARSES it back
/// rather than building it, which is what keeps it an expectation instead
/// of the second serializer `no_test_reassembles_the_component_stream`
/// forbids.
const WORKED_PLUGIN_STREAM: &str =
    "LICENSE\u{0}79dce08a18044a366563406c6a3c02cbd947989c47fdd64d0fbaf351ed1a83a4\n\
     README.md\u{0}86f31cae9f04723ccacec665210e2fdb6fffe4ae7d19901e0420a81376d654ac\n\
     cordis.patch.yml\u{0}2e5380d538ff7f04b1d0035336fd3937c6762ad7acf8a17e00a0a23bdf2d6bc2\n\
     lib/index.js\u{0}e931e87e321b250f52712f4090c3c7b960a5e225f6a5eaf4b9f177f05faa1b24\n\
     lib/startup.js\u{0}725cce087fb0ffa4df40acf9b5ac782097850db6da91e61ff29d5387266ba32d\n\
     package.json\u{0}935f6e76f24a7b51dde7e944be8dfa5216f5d1058602cfd72a40ff6136cfebf0\n";

/// The producer's pinned component over the worked plugin set.
const WORKED_PLUGIN_VECTOR_COMPONENT: &str =
    "d1f7df60c6c4eeda7797b4d2d54241cc7d1597b808b33422af87ee7ef9e09d7a";

/// Stage one worked byte set beneath `dir`, each member at its declared
/// relative path.
fn stage_set(dir: &Path, set: &[(&str, &[u8])]) {
    for (name, bytes) in set {
        write(dir, name, bytes);
    }
}

/// Read a frozen component-stream literal back as the form D6 declares,
/// and bind it to one observation.
///
/// The literal is PARSED, never assembled: this proves the hand-written
/// expectation really is `<relative path>\0<file SHA-256>\n` per line, in
/// strictly increasing bytewise path order, over exactly the declared set,
/// carrying exactly the digests the producer's walk observed — and it
/// produces no component of its own.
fn read_component_stream(stream: &str, expected: &[&str], observed: &BTreeMap<String, String>) {
    let body = stream
        .strip_suffix('\n')
        .expect("the component stream ends with a newline");
    let lines: Vec<&str> = body.split('\n').collect();
    assert_eq!(
        lines.len(),
        expected.len(),
        "one line per declared file: {lines:?}"
    );
    let mut previous = "";
    for (line, file) in lines.iter().zip(expected) {
        let (path, digest) = match line.split_once('\u{0}') {
            Some(split) => split,
            None => panic!("{line:?} carries no NUL separator"),
        };
        assert_eq!(path, *file, "the declared file at this position");
        assert!(
            previous < path,
            "{previous:?} then {path:?} is not bytewise path order"
        );
        previous = path;
        assert_eq!(
            digest,
            observed
                .get(path)
                .unwrap_or_else(|| panic!("{path} was observed")),
            "{path}: the line carries the observed digest"
        );
    }
}

/// The worked plugin vector: the component's bytewise path order, its
/// exact input bytes and per-file digests, D6's stream form, and the
/// producer's own pinned component over all six.
///
/// The files are staged in REVERSE declared order, so a component that
/// followed the order the fixture wrote them in — or the order the
/// platform's `read_dir` yields — parts from the pinned value.
#[test]
fn the_worked_plugin_vector_pins_the_bytewise_path_order_of_the_component() {
    let dir = FixtureRoot::new();
    for (name, bytes) in WORKED_PLUGIN_SET.iter().rev() {
        write(dir.path(), name, bytes);
    }

    // The declared set is itself in bytewise order, so "declared order"
    // and "bytewise order" below are the same claim about the same six
    // paths rather than two that happen to agree.
    assert!(
        PLUGIN_FILES.windows(2).all(|pair| pair[0] < pair[1]),
        "the declared plugin set is in strictly increasing bytewise order"
    );

    let observed =
        plugin_file_digests("plugin", dir.path(), &PLUGIN_FILES, &read_dir_entries).unwrap();
    assert_eq!(
        observed.keys().map(String::as_str).collect::<Vec<&str>>(),
        PLUGIN_FILES.to_vec(),
        "the walk observed exactly the declared six, in bytewise path order"
    );

    // Each member's EXACT input bytes reached the file, and each observed
    // digest is that file's own. Hashing one file's bytes on its own is an
    // input check, not a second component producer.
    for (name, bytes) in WORKED_PLUGIN_SET {
        assert_eq!(fs::read(dir.path().join(name)).unwrap(), bytes, "{name}");
        assert_eq!(observed[name], digest_of(bytes), "{name}");
    }

    read_component_stream(WORKED_PLUGIN_STREAM, &PLUGIN_FILES, &observed);
    let component = component_digest(&observed);
    assert_eq!(
        component, WORKED_PLUGIN_VECTOR_COMPONENT,
        "the producer's pinned component over the worked plugin set"
    );
    assert_eq!(
        digest_of(WORKED_PLUGIN_STREAM.as_bytes()),
        component,
        "the component is the SHA-256 of exactly these lines, in this \
         order, ending with a newline"
    );
}

/// The hidden lock the worked pair is staged with: the core's own record,
/// the plugin's LOCAL `file:` tarball record, a same-named REGISTRY record
/// of the plugin nested under another package, and one ordinary
/// dependency the synthetic pnpm lock also carries.
///
/// Written in D1's grammar and read through the same producer; D1's own
/// dialect matrix is not reopened here.
const WORKED_PAIR_NPM_LOCK: &str = r#"{"lockfileVersion":3,"packages":{
  "node_modules/@deepseek-ai/dsh":{"version":"0.1.5-rc.2","integrity":"sha512-CORE"},
  "node_modules/dsh-plugin-cli-session":{"version":"0.2.0","resolved":"file:plugin.tgz","link":true},
  "node_modules/nested/node_modules/dsh-plugin-cli-session":{"version":"0.2.0","resolved":"https://registry.npmjs.org/dsh-plugin-cli-session-0.2.0.tgz","integrity":"sha512-NPMREG"},
  "node_modules/debug":{"version":"2.6.9","integrity":"sha512-DEBUG"}
}}"#;

/// The complete ordered dependency values the worked pair composes.
const WORKED_PAIR_DEPENDENCIES: [&str; 2] = [
    "debug 2.6.9 sha512-DEBUG",
    "dsh-plugin-cli-session 0.2.0 sha512-NPMREG",
];

/// Install the worked pair into a synthetic home: the six worked plugin
/// files at the profile's plugin locator, replacing the fixture's own, and
/// the worked hidden lock at the core's. Everything else is
/// `Synthetic::new`'s, so two homes prepared this way differ only in where
/// they sit.
fn worked_pair(install: &Synthetic) {
    stage_set(
        &install.profile().join("node_modules").join(PLUGIN_BUNDLE),
        &WORKED_PLUGIN_SET,
    );
    write(
        &install.dir.path().join("core"),
        "node_modules/.package-lock.json",
        WORKED_PAIR_NPM_LOCK.as_bytes(),
    );
}

/// The plugin's own local tarball record leaves the dependency lines and
/// its same-named registry record stays — through the PRODUCER, whose
/// exclusion list carries the plugin exactly because the component above
/// was measured from its installed bytes.
#[test]
fn the_plugin_s_own_tarball_record_leaves_and_its_registry_namesake_stays() {
    let install = Synthetic::new();
    worked_pair(&install);
    let observed = install.composite();

    assert_eq!(
        observed.plugin, WORKED_PLUGIN_VECTOR_COMPONENT,
        "the installed plugin is the worked set, so the exclusion is that set's"
    );
    assert_eq!(
        observed.dependencies,
        WORKED_PAIR_DEPENDENCIES.map(str::to_string).to_vec(),
        "the local record leaves, the registry namesake stays, and the core's \
         own record is never a dependency"
    );

    // What the exclusion is worth: the same lock read with NO name
    // supplying a component refuses at that exact record, because a local
    // `file:` entry carries no registry integrity. Composition above
    // succeeded only because the record was excluded, and it was excluded
    // as a RECORD — the nested registry entry of the same name survived
    // beside it.
    assert_eq!(
        refused(npm_dependencies(&lock(WORKED_PAIR_NPM_LOCK), &[])),
        "npm lock is unreadable: 'node_modules/dsh-plugin-cli-session': \
         no registry 'integrity'"
    );
}

/// The producer's pinned canonical composite over the worked pair in a
/// synthetic home with no extension listed.
const WORKED_PAIR_CANONICAL: &str =
    "85f6e4b6653ca50a9004f067def20e5971192c8745bd7dce0f97543cd8bec54e";

/// One seat's own overlay, staged the way the launch planner stages it:
/// this seat's transcript root allocated under the home, and the rows
/// that seat pins written to a temporary `--patch` file. The value is
/// RETURNED to the caller because the patch file and its settings
/// document live exactly as long as it does.
///
/// A per-seat overlay is not an identity-bearing patch. The composite's
/// `plugin-patch`, `profile-patch` and `home-patch` lines are the
/// installation's own `cordis.patch.yml` files; this is a file the driver
/// hands the launcher, beside a directory under the home only this seat
/// writes.
fn seat_overlay(
    home: &Path,
    model: Option<&str>,
    effort: Option<&str>,
) -> crate::adapters::DshSeatOverlay {
    let root = crate::transcript::dsh_transcript_root_under(Some(home.to_path_buf()))
        .expect("the seat's transcript root is allocated under the home");
    assert!(
        root.starts_with(home),
        "this seat's transcript root sits under the home: {}",
        root.display()
    );
    crate::adapters::dsh_seat_overlay_with(model, effort, &root, None, None)
        .expect("the planner stages this seat's overlay")
}

/// The same pair staged in two homes at different absolute paths, under
/// two different per-seat overlays, is ONE identity — and so is the first
/// home reached through a symlinked ancestor.
///
/// Unix only: a symlinked ancestor is one of the inputs, and Linux and
/// macOS are the only hosts (decision 0063).
#[cfg(unix)]
#[test]
fn one_pair_in_two_homes_under_two_seat_overlays_is_one_identity() {
    let first = Synthetic::new();
    let second = Synthetic::new();
    assert_ne!(
        first.seams.home, second.seams.home,
        "the two homes are at different absolute paths"
    );
    worked_pair(&first);
    worked_pair(&second);

    // Two seats, each with its own overlay near its own home: different
    // pinned rows, different `--patch` files, and a transcript root under
    // each home that only that seat writes.
    let one_seat = seat_overlay(
        &first.seams.home,
        Some("deepseek/deepseek-chat"),
        Some("high"),
    );
    let two_seat = seat_overlay(&second.seams.home, None, None);
    assert_ne!(
        one_seat.path(),
        two_seat.path(),
        "each seat stages its own overlay file"
    );
    assert_ne!(
        fs::read(one_seat.path()).unwrap(),
        fs::read(two_seat.path()).unwrap(),
        "the two seats pinned different rows"
    );

    let one = first.composite();
    let two = second.composite();
    // The invariance this case owns, asserted before the literals it is
    // pinned to, so a producer that let a staging path or a seat's own
    // file into the identity parts HERE rather than at an expectation
    // that could be restated.
    assert_eq!(
        two.plugin, one.plugin,
        "the same six files at another absolute path are the same component"
    );
    assert_eq!(
        two.canonical, one.canonical,
        "neither the staging path nor the seat beside it enters the identity"
    );
    assert_eq!(
        one.plugin, WORKED_PLUGIN_VECTOR_COMPONENT,
        "the worked plugin component, staged at an absolute path this \
         fixture chose"
    );
    assert_eq!(
        one.canonical, WORKED_PAIR_CANONICAL,
        "the producer's pinned composite over the worked pair"
    );

    // The identity-bearing patches are equal, and the per-seat overlay is
    // not one of them: a seat's own directory under the home leaves
    // `home-patch` at the literal `absent`.
    assert_eq!(
        one.plugin_patch,
        digest_of(b"- id: cli-session\n  config: {}\n"),
        "the worked plugin's own cordis.patch.yml"
    );
    assert_eq!(two.plugin_patch, one.plugin_patch);
    assert_eq!(one.profile_patch, two.profile_patch);
    assert_eq!(
        [one.home_patch.as_str(), two.home_patch.as_str()],
        ["absent", "absent"],
        "a seat's transcript root under the home is not a home patch"
    );

    // The same home, reached through a symlinked ancestor: the same
    // bundles resolve, so the plugin and the composite are the same
    // values. The raw anchor's own discriminator lives in
    // `the_dsh_composite_accepts_a_symlinked_home_ancestor` and is not
    // repeated here.
    let alias = first.dir.path().join("alias");
    std::os::unix::fs::symlink(&first.seams.home, &alias).unwrap();
    let aliased = DshSeams {
        executable: first.seams.executable.clone(),
        home: alias,
        node: None,
        head: first.seams.head.clone(),
    };
    let through_alias = dsh_composite_with(&aliased, &first.node(), &[]).unwrap();
    assert_eq!(through_alias.plugin, WORKED_PLUGIN_VECTOR_COMPONENT);
    assert_eq!(through_alias.canonical, WORKED_PAIR_CANONICAL);

    // The moving control: the comparison above is capable of parting. A
    // home-level `cordis.patch.yml` — an identity-bearing patch, unlike
    // the overlay — moves the second home's composite away from the
    // first's.
    write(&second.seams.home, "cordis.patch.yml", b"[]\n");
    let patched = second.composite();
    assert_eq!(patched.home_patch, digest_of(b"[]\n"));
    assert_ne!(patched.canonical, one.canonical);
}

/// The manifest every general-bundle candidate in the containment cases
/// carries: the same package at the same version, so which candidate the
/// search reached is the only thing that can separate two observations.
const BUNDLE_MANIFEST: &[u8] = br#"{"name":"@deepseek-ai/dsh-base","version":"0.1.5-rc.2"}"#;

/// The refusal a listed bundle earns when the candidate the search
/// reached lands outside both canonical roots.
fn outside_both_roots(name: &str, dir: &Path) -> String {
    format!(
        "the DSH layout is unreadable: bundle '{name}' resolves outside the core root \
         and the profile ({})",
        dir.canonicalize().unwrap().display()
    )
}

/// A bundle reached through a SYMLINK is judged where the link lands.
///
/// The boundary is canonical, so a candidate whose own spelling sits
/// inside the profile and whose target does not earns its target's
/// refusal, named by the canonical path. No fallback and no string-prefix
/// comparison cures it.
///
/// Unix only: the symlink is the input, and Linux and macOS are the only
/// hosts (decision 0063).
#[cfg(unix)]
#[test]
fn a_bundle_directory_that_is_a_symlink_is_judged_where_it_lands() {
    let install = Synthetic::new();
    let installed = install
        .profile()
        .join("node_modules")
        .join("@deepseek-ai/dsh-base");
    fs::remove_dir_all(&installed).unwrap();

    let elsewhere = install.dir.path().join("elsewhere/@deepseek-ai/dsh-base");
    write(&elsewhere, "package.json", BUNDLE_MANIFEST);
    std::os::unix::fs::symlink(&elsewhere, &installed).unwrap();
    assert_eq!(
        refused(dsh_composite_with(&install.seams, &install.node(), &[])),
        outside_both_roots("@deepseek-ai/dsh-base", &elsewhere),
        "the candidate's own spelling is inside the profile; its target is not"
    );

    // The control: the same symlinked candidate whose target lands INSIDE
    // the profile resolves, so the refusal above is the target's and not
    // the link's.
    fs::remove_file(&installed).unwrap();
    let vendored = install.profile().join("vendor/@deepseek-ai/dsh-base");
    write(&vendored, "package.json", BUNDLE_MANIFEST);
    std::os::unix::fs::symlink(&vendored, &installed).unwrap();
    assert_eq!(
        install.composite().profile_bundles,
        vec!["@deepseek-ai/dsh-base", "dsh-plugin-cli-session"]
    );
}

/// The bundle search order D6 preserves, read off the producer: the core
/// package's Node lookup, then the global folders, then the profile's own
/// Node lookup from the RAW anchor upward.
///
/// Each position holds one candidate outside both canonical roots, so the
/// refusal names the candidate the search reached first and removing it
/// hands the next position its turn. Steps one and two are also the two
/// shapes a listed general bundle escapes through — an ancestor
/// `node_modules` and an injected global folder — and the contained copy
/// the profile ships is present throughout, so no refusal here is a
/// bundle the search simply could not find.
#[test]
fn the_bundle_search_order_is_core_ancestors_then_globals_then_the_profile() {
    let install = Synthetic::new();
    let root = install.dir.path().to_path_buf();
    let bundle = "@deepseek-ai/dsh-base";

    // An ancestor `node_modules` of the core package, above the core root
    // `<root>/core` and outside the profile.
    let ancestor = root.join("node_modules").join(bundle);
    write(&ancestor, "package.json", BUNDLE_MANIFEST);
    // An injected global folder, searched after the core's ancestors.
    let global = root.join("global");
    write(&global.join(bundle), "package.json", BUNDLE_MANIFEST);
    // An ancestor `node_modules` of the home, above the profile boundary.
    let above_profile = install.seams.home.join("node_modules").join(bundle);
    write(&above_profile, "package.json", BUNDLE_MANIFEST);
    // The contained copy the fixture ships, which the search prefers only
    // when it is reached.
    let contained = install.profile().join("node_modules").join(bundle);
    assert!(contained.join("package.json").is_file());
    let globals = [global.clone()];

    // 1. The core package's own Node lookup is first.
    assert_eq!(
        refused(dsh_composite_with(
            &install.seams,
            &install.node(),
            &globals
        )),
        outside_both_roots(bundle, &ancestor)
    );

    // 2. The global folders come next.
    fs::remove_dir_all(root.join("node_modules")).unwrap();
    assert_eq!(
        refused(dsh_composite_with(
            &install.seams,
            &install.node(),
            &globals
        )),
        outside_both_roots(bundle, &global.join(bundle))
    );

    // 3. Then the profile's own lookup from the raw anchor, whose first
    //    candidate is the contained copy: the layout composes, and the
    //    candidate above the profile is never reached.
    fs::remove_dir_all(global.join(bundle)).unwrap();
    assert_eq!(
        dsh_composite_with(&install.seams, &install.node(), &globals)
            .unwrap()
            .profile_bundles,
        vec![bundle, PLUGIN_BUNDLE]
    );

    // 4. With the contained copy gone that walk climbs, and the candidate
    //    above the profile is where it lands — outside both roots, and
    //    still a refusal rather than a bundle taken from above.
    fs::remove_dir_all(&contained).unwrap();
    assert_eq!(
        refused(dsh_composite_with(
            &install.seams,
            &install.node(),
            &globals
        )),
        outside_both_roots(bundle, &above_profile)
    );
}

/// The worked conditional-extension vector: the four declared relative
/// paths and the exact bytes staged at each.
///
/// SYNTHETIC throughout. No repository extension exists and none is
/// created here: these bytes are a temporary set staged under a temporary
/// profile, and the Pass D clause comparing a real extension's committed
/// files to its own provenance block applies only if an extension ever
/// becomes required. No member shares bytes with a worked plugin member,
/// so a walk that read one set where the other belongs cannot reach the
/// pinned component.
const WORKED_EXTENSION_SET: [(&str, &[u8]); 4] = [
    ("LICENSE", b"MIT: the extension's licence\n"),
    ("cordis.patch.yml", b"- id: resume-policy\n  config: {}\n"),
    ("index.js", b"module.exports = { name: 'resume-policy' }\n"),
    (
        "package.json",
        b"{\"name\":\"brokkr-dsh-resume-policy\",\"version\":\"0.1.0\"}\n",
    ),
];

/// D6's component stream over the worked extension set, written out by
/// hand in the same frozen form as `WORKED_PLUGIN_STREAM` and parsed back
/// by `read_component_stream`.
const WORKED_EXTENSION_STREAM: &str =
    "LICENSE\u{0}f2b46aa194ff89288e43e52a6114072cd0e534f900b774aaf712c852d4820349\n\
     cordis.patch.yml\u{0}9573302ca2ebd68d6d82706d7719b64e3263ae72ae207974230e72fb1beeec69\n\
     index.js\u{0}875368ee35e942d7403b1dfc80c6cf471ae42c9f2bbe24790f6b9b3a86514733\n\
     package.json\u{0}aeee393cadf790f650bdad1ece41c1dd4520fe78a4aaadad76f4a8643dbcf662\n";

/// The producer's pinned component over the worked extension set.
const WORKED_EXTENSION_COMPONENT: &str =
    "7d95298968e908a8b10a692aff697b41d707f2b64f04b589cb7948c0f39a9cfd";

/// The producer's pinned canonical composite over the worked pair with
/// the worked extension listed and installed beside it.
const WORKED_TRIO_CANONICAL: &str =
    "2a08a17b3526f542138e760fa35c6e245be69d13380b32e2566ae82e13885183";

/// The profile manifest that lists the conditional extension after the
/// pair, so the declared order carries three `profile-bundle` rows.
const TRIO_PROFILE_MANIFEST: &[u8] = br#"{"dsh":{"profile":{"bundles":["@deepseek-ai/dsh-base","dsh-plugin-cli-session","brokkr-dsh-resume-policy"],"patchReload":"startup"}}}"#;

/// D6's component stream for the worked pair with NO extension listed,
/// written out by hand: the same frozen-literal discipline as D1's, and
/// the direct statement that absence emits no `extension` line.
const WORKED_PAIR_STREAM: &str = "core\u{0}@deepseek-ai/dsh 0.1.5-rc.2 sha512-CORE\n\
     node\u{0}v22.23.2\n\
     dependency\u{0}debug 2.6.9 sha512-DEBUG\n\
     dependency\u{0}dsh-plugin-cli-session 0.2.0 sha512-NPMREG\n\
     plugin\u{0}d1f7df60c6c4eeda7797b4d2d54241cc7d1597b808b33422af87ee7ef9e09d7a\n\
     plugin-patch\u{0}2e5380d538ff7f04b1d0035336fd3937c6762ad7acf8a17e00a0a23bdf2d6bc2\n\
     profile-patch\u{0}37517e5f3dc66819f61f5a7bb8ace1921282415f10551d2defa5c3eb0985b570\n\
     profile-bundle\u{0}@deepseek-ai/dsh-base\n\
     profile-bundle\u{0}dsh-plugin-cli-session\n\
     profile-patch-reload\u{0}startup\n\
     home-patch\u{0}absent\n";

/// The same stream once the extension is listed and installed: one more
/// `profile-bundle` row in declared order, and one `extension` line at
/// the end.
const WORKED_TRIO_STREAM: &str = "core\u{0}@deepseek-ai/dsh 0.1.5-rc.2 sha512-CORE\n\
     node\u{0}v22.23.2\n\
     dependency\u{0}debug 2.6.9 sha512-DEBUG\n\
     dependency\u{0}dsh-plugin-cli-session 0.2.0 sha512-NPMREG\n\
     plugin\u{0}d1f7df60c6c4eeda7797b4d2d54241cc7d1597b808b33422af87ee7ef9e09d7a\n\
     plugin-patch\u{0}2e5380d538ff7f04b1d0035336fd3937c6762ad7acf8a17e00a0a23bdf2d6bc2\n\
     profile-patch\u{0}37517e5f3dc66819f61f5a7bb8ace1921282415f10551d2defa5c3eb0985b570\n\
     profile-bundle\u{0}@deepseek-ai/dsh-base\n\
     profile-bundle\u{0}dsh-plugin-cli-session\n\
     profile-bundle\u{0}brokkr-dsh-resume-policy\n\
     profile-patch-reload\u{0}startup\n\
     home-patch\u{0}absent\n\
     extension\u{0}7d95298968e908a8b10a692aff697b41d707f2b64f04b589cb7948c0f39a9cfd\n";

/// Stage the worked extension beneath the profile and list it after the
/// pair. The files are written in REVERSE declared order, so a component
/// that followed the fixture's write order parts from the pinned value.
fn stage_worked_extension(install: &Synthetic) {
    for (name, bytes) in WORKED_EXTENSION_SET.iter().rev() {
        write(
            &install
                .profile()
                .join("node_modules")
                .join(EXTENSION_BUNDLE),
            name,
            bytes,
        );
    }
    write(&install.profile(), "package.json", TRIO_PROFILE_MANIFEST);
}

/// The conditional extension: absent, then composed from its own four
/// files, with both composites pinned twice and the ABSENCE stated as the
/// missing line rather than as a missing value.
#[test]
fn the_conditional_extension_is_absent_or_composed_from_its_own_four_files() {
    let install = Synthetic::new();
    worked_pair(&install);

    // Absent: the profile names no extension, so the stream carries no
    // `extension` line at all.
    let absent = install.composite();
    assert_eq!(absent.extension, None);
    assert_eq!(absent.canonical, WORKED_PAIR_CANONICAL);
    assert!(
        !WORKED_PAIR_STREAM.contains("extension"),
        "absence emits no extension line"
    );
    assert_eq!(
        digest_of(WORKED_PAIR_STREAM.as_bytes()),
        absent.canonical,
        "the composite over the pair is the SHA-256 of exactly these \
         component lines, in this order"
    );

    // Present: the four declared files, in bytewise path order.
    stage_worked_extension(&install);
    assert!(
        EXTENSION_FILES.windows(2).all(|pair| pair[0] < pair[1]),
        "the declared extension set is in strictly increasing bytewise order"
    );
    let installed = install
        .profile()
        .join("node_modules")
        .join(EXTENSION_BUNDLE);
    let observed =
        plugin_file_digests("extension", &installed, &EXTENSION_FILES, &read_dir_entries).unwrap();
    assert_eq!(
        observed.keys().map(String::as_str).collect::<Vec<&str>>(),
        EXTENSION_FILES.to_vec(),
        "the walk observed exactly the declared four, in bytewise path order"
    );
    for (name, bytes) in WORKED_EXTENSION_SET {
        assert_eq!(fs::read(installed.join(name)).unwrap(), bytes, "{name}");
        assert_eq!(observed[name], digest_of(bytes), "{name}");
    }
    read_component_stream(WORKED_EXTENSION_STREAM, &EXTENSION_FILES, &observed);
    assert_eq!(
        digest_of(WORKED_EXTENSION_STREAM.as_bytes()),
        WORKED_EXTENSION_COMPONENT,
        "the extension component is the SHA-256 of exactly these lines"
    );

    let present = install.composite();
    assert_eq!(
        present.extension.as_deref(),
        Some(WORKED_EXTENSION_COMPONENT),
        "the producer's pinned component over the worked extension set"
    );
    assert_eq!(
        present.profile_bundles,
        vec!["@deepseek-ai/dsh-base", PLUGIN_BUNDLE, EXTENSION_BUNDLE],
        "the declared order, with the extension last"
    );
    assert_eq!(present.canonical, WORKED_TRIO_CANONICAL);
    assert_ne!(present.canonical, absent.canonical);
    assert_eq!(
        digest_of(WORKED_TRIO_STREAM.as_bytes()),
        present.canonical,
        "the same stream with one more profile-bundle row and one \
         extension line at the end"
    );
    // Read the frozen literal's last line back: the extension line is the
    // final one, and it carries the observed component.
    let last = WORKED_TRIO_STREAM
        .strip_suffix('\n')
        .expect("the stream ends with a newline")
        .rsplit('\n')
        .next()
        .expect("the stream has a last line");
    let (component, value) = last
        .split_once('\u{0}')
        .expect("the last line carries a NUL separator");
    assert_eq!(component, "extension");
    assert_eq!(value, WORKED_EXTENSION_COMPONENT);

    // One changed byte in one extension file moves the extension
    // component and the composite with it.
    write(
        &installed,
        "index.js",
        b"module.exports = { name: 'other' }\n",
    );
    let changed = install.composite();
    assert_ne!(changed.extension, present.extension);
    assert_ne!(changed.canonical, present.canonical);
}

/// A listed extension whose installed set has drifted is unreadable by
/// the EXTENSION component — never plugin drift, and never absence.
#[cfg(unix)]
#[test]
fn the_extension_walk_refuses_a_missing_extra_or_symlinked_member() {
    let install = Synthetic::new();
    worked_pair(&install);
    stage_worked_extension(&install);
    let installed = install
        .profile()
        .join("node_modules")
        .join(EXTENSION_BUNDLE);
    assert!(install.composite().extension.is_some());

    // An EXTRA file the declared set does not name.
    write(&installed, "extra.txt", b"x");
    assert_eq!(
        refused(dsh_composite_with(&install.seams, &install.node(), &[])),
        "extension component is unreadable: unexpected entry 'extra.txt'"
    );
    fs::remove_file(installed.join("extra.txt")).unwrap();

    // A declared file that is a SYMLINK, even to bytes that would hash to
    // the same value: a hashed member is a regular non-symlink file.
    let target = install.dir.path().join("licence-elsewhere");
    fs::write(&target, b"MIT: the extension's licence\n").unwrap();
    fs::remove_file(installed.join("LICENSE")).unwrap();
    std::os::unix::fs::symlink(&target, installed.join("LICENSE")).unwrap();
    assert_eq!(
        refused(dsh_composite_with(&install.seams, &install.node(), &[])),
        "extension component is unreadable: 'LICENSE' is a symlink"
    );
    fs::remove_file(installed.join("LICENSE")).unwrap();

    // A MISSING declared file — the same LICENSE, now simply gone.
    assert_eq!(
        refused(dsh_composite_with(&install.seams, &install.node(), &[])),
        "extension component is unreadable: missing expected file 'LICENSE'"
    );
}

/// The extension's own local `file:` record leaves the dependency lines
/// only when the extension RESOLVED, because the exclusion list carries a
/// name exactly when that name's installed bytes supplied a component.
#[test]
fn the_extension_s_local_record_leaves_only_when_the_extension_resolves() {
    // The pair's lock with the extension's local tarball record and a
    // same-named registry record beside it.
    const TRIO_NPM_LOCK: &str = r#"{"lockfileVersion":3,"packages":{
      "node_modules/@deepseek-ai/dsh":{"version":"0.1.5-rc.2","integrity":"sha512-CORE"},
      "node_modules/dsh-plugin-cli-session":{"version":"0.2.0","resolved":"file:plugin.tgz","link":true},
      "node_modules/nested/node_modules/dsh-plugin-cli-session":{"version":"0.2.0","resolved":"https://registry.npmjs.org/dsh-plugin-cli-session-0.2.0.tgz","integrity":"sha512-NPMREG"},
      "node_modules/brokkr-dsh-resume-policy":{"version":"0.1.0","resolved":"file:extension.tgz","link":true},
      "node_modules/nested/node_modules/brokkr-dsh-resume-policy":{"version":"0.1.0","resolved":"https://registry.npmjs.org/brokkr-dsh-resume-policy-0.1.0.tgz","integrity":"sha512-EXTREG"},
      "node_modules/debug":{"version":"2.6.9","integrity":"sha512-DEBUG"}
    }}"#;

    let install = Synthetic::new();
    worked_pair(&install);
    stage_worked_extension(&install);
    write(
        &install.dir.path().join("core"),
        "node_modules/.package-lock.json",
        TRIO_NPM_LOCK.as_bytes(),
    );
    let observed = install.composite();
    assert!(observed.extension.is_some());
    assert_eq!(
        observed.dependencies,
        vec![
            "brokkr-dsh-resume-policy 0.1.0 sha512-EXTREG".to_string(),
            "debug 2.6.9 sha512-DEBUG".to_string(),
            "dsh-plugin-cli-session 0.2.0 sha512-NPMREG".to_string(),
        ],
        "both local records leave and both registry namesakes stay"
    );

    // The SAME lock with the extension no longer listed: nothing supplied
    // an extension component, so its local record is an ordinary entry —
    // and an ordinary entry needs a registry integrity.
    write(
        &install.profile(),
        "package.json",
        br#"{"dsh":{"profile":{"bundles":["@deepseek-ai/dsh-base","dsh-plugin-cli-session"],"patchReload":"startup"}}}"#,
    );
    assert_eq!(
        refused(dsh_composite_with(&install.seams, &install.node(), &[])),
        "npm lock is unreadable: 'node_modules/brokkr-dsh-resume-policy': \
         no registry 'integrity'"
    );
}

// ---------------------------------------------------------------------------
// Pass D part three: the profile's own lines, and the generated file that
// is not one of them.
//
// SYNTHETIC deterministic storage shims over the canonical-root synthetic
// home. Nothing here observes a live DSH installation, and no case is
// evidence of upstream compatibility, qualification or enforcement.
// ---------------------------------------------------------------------------

/// The manifest that lists `bundles` in `order` with `reload`.
fn profile_manifest(order: &[&str], reload: &str) -> Vec<u8> {
    let bundles = order
        .iter()
        .map(|name| format!("\"{name}\""))
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"dsh\":{{\"profile\":{{\"bundles\":[{bundles}],\"patchReload\":\"{reload}\"}}}}}}")
        .into_bytes()
}

/// An additional bundle the profile may list, installed beneath the
/// profile so it resolves: the composite gains a `profile-bundle` row and
/// nothing else, because a bundle's own bytes reach the identity only
/// through the plugin, the extension and the locks.
const ADDED_BUNDLE: &str = "@deepseek-ai/dsh-headless";

/// A profile bundle added, dropped or reordered, a changed `patchReload`
/// and an added home-level `cordis.patch.yml` each move the canonical
/// composite, and each moves it to its own RECORDED value.
///
/// Every case asserts three independent things: the component values and
/// their order as the producer reports them, the identity each mutation
/// lands on as a literal digest recorded from that same sole producer,
/// and that no two of the six collide. A serializer that sorted the
/// bundle rows, dropped one, or left `patchReload` out would differ from
/// the recorded digest and fail here, because the moved value is pinned
/// and not merely required to differ from the base.
///
/// The digests below are literals: the first version of this suite
/// rebuilt the `profile-bundle` / `profile-patch-reload` / `home-patch`
/// rows from the same case inputs the fixture had just written and hashed
/// them, which is the second serializer D6 forbids — it agreed with the
/// producer by construction, not by evidence (review 2026-09-23, finding
/// 1; `no_test_reassembles_the_component_stream` now refuses that
/// spelling by name). The tail's BYTE FORM is pinned once, where a frozen
/// recorded stream can pin it: `WORKED_CANONICAL_STREAM` carries the
/// `profile-bundle` rows in declared order, `profile-patch-reload` and
/// `home-patch` verbatim. What these six cases own is the MOVEMENT
/// (task 8.8(d), Pass D; design D6).
#[test]
fn a_profile_bundle_added_dropped_or_reordered_moves_the_composite() {
    let install = Synthetic::new();
    let profile = install.profile();
    // The added bundle resolves inside the profile, so listing it is a
    // profile change and never a containment refusal.
    write(
        &profile.join("node_modules").join(ADDED_BUNDLE),
        "package.json",
        br#"{"name":"@deepseek-ai/dsh-headless","version":"0.1.5-rc.2"}"#,
    );

    const BASE: [&str; 2] = ["@deepseek-ai/dsh-base", "dsh-plugin-cli-session"];
    // Every case keeps `dsh-plugin-cli-session`, which the profile must
    // list: the dropped case drops the OTHER bundle, so a readable
    // removal is what is measured rather than the plugin refusal.
    //
    // The fourth member of each row is the canonical composite this
    // install yields under that profile, recorded from the producer.
    let cases: [(&str, Vec<&str>, &str, &str, &str); 6] = [
        (
            "base",
            BASE.to_vec(),
            "startup",
            "absent",
            "286b90a009cb66135faa2566be7b26305e8470ae7761d1e3e4b6c74e9973c1c4",
        ),
        (
            "added",
            vec![BASE[0], ADDED_BUNDLE, BASE[1]],
            "startup",
            "absent",
            "b24fc83caefcc7cd6791e74959a6faebc5a617aace859fc0268fb59a78ebad44",
        ),
        (
            "dropped",
            vec![BASE[1]],
            "startup",
            "absent",
            "dd75d065a528545bd7b775c679fb9d2d099b960300577e5fc27db1e4b13d1710",
        ),
        (
            "reordered",
            vec![BASE[1], BASE[0]],
            "startup",
            "absent",
            "ae478bfc13e8271728003dc2a2ab8ca0c808ff8a64df3d4004f92887a4081365",
        ),
        (
            "reload",
            BASE.to_vec(),
            "live",
            "absent",
            "46af043de52f509bd66bcef35297d3ae4e2ca884d0b33ef698a13814f466fc16",
        ),
        (
            "home patch",
            BASE.to_vec(),
            "startup",
            // `[]\n` is the profile patch's bytes too, so the home line
            // takes a digest the stream already carries: the case moves
            // because a `home-patch` line changed from `absent`, not
            // because a new digest appeared anywhere.
            "37517e5f3dc66819f61f5a7bb8ace1921282415f10551d2defa5c3eb0985b570",
            "021ee2d3a3edf28ca80b16794a9218bfd46474c486952468594b96f1e5e7262b",
        ),
    ];

    let mut digests: Vec<String> = Vec::new();
    for (case, order, reload, home_patch, canonical) in &cases {
        write(&profile, "package.json", &profile_manifest(order, reload));
        if *home_patch == "absent" {
            let path = install.seams.home.join("cordis.patch.yml");
            if path.exists() {
                fs::remove_file(&path).unwrap();
            }
        } else {
            write(&install.seams.home, "cordis.patch.yml", b"[]\n");
        }
        let observed = install.composite();

        // The component values, before any digest is compared.
        assert_eq!(
            observed.profile_bundles,
            order
                .iter()
                .map(|name| name.to_string())
                .collect::<Vec<_>>(),
            "{case}: the declared bundle order"
        );
        assert_eq!(observed.profile_patch_reload, *reload, "{case}");
        assert_eq!(observed.home_patch, *home_patch, "{case}");
        assert_eq!(
            observed.dependencies,
            vec!["debug 2.6.9 sha512-DEBUG".to_string()],
            "{case}: a profile change moves no dependency line"
        );
        assert_eq!(
            observed.plugin, "8894f23eef97b42abfda88b6dd42c4b44b17ac4cb7e6df14c6bda687ae534c99",
            "{case}: a profile change moves no plugin byte"
        );
        assert_eq!(observed.extension, None, "{case}");

        // The identity: the value recorded from the producer for exactly
        // this profile, not a value recomputed here from these inputs.
        assert_eq!(
            observed.canonical, *canonical,
            "{case}: the recorded canonical composite for this profile"
        );
        digests.push(observed.canonical.clone());
    }

    // Six streams, six identities: each mutation MOVED the composite, and
    // no two of them collided.
    let distinct: BTreeSet<&String> = digests.iter().collect();
    assert_eq!(
        distinct.len(),
        digests.len(),
        "each profile mutation moves the composite to its own value: {digests:?}"
    );
}

/// The generated `cordis.yml` is not a composite input, so rewriting it
/// moves nothing.
///
/// The core rewrites this file from its own constant before every boot
/// and the Loader may write it back between boots, which is exactly why
/// D6 excludes it. The control is the file beside it: the profile's
/// `cordis.patch.yml` IS an input, and one changed byte there moves the
/// identity — so this test can tell "excluded" from "the fixture never
/// changed anything" (task 8.8(d), Pass D; design D6).
#[test]
fn a_rewritten_generated_cordis_yml_leaves_the_composite_untouched() {
    let install = Synthetic::new();
    let profile = install.profile();
    let base = install.composite();

    // The generated file appears where `prepareProfile` writes it.
    write(
        &profile,
        "cordis.yml",
        b"# generated by prepareProfile\nbundles: []\n",
    );
    assert_eq!(
        install.composite(),
        base,
        "a generated cordis.yml joins no component line"
    );

    // And it is rewritten, as every boot rewrites it.
    write(
        &profile,
        "cordis.yml",
        b"# generated by prepareProfile\nbundles: []\nrewritten: true\n",
    );
    assert_eq!(
        install.composite(),
        base,
        "rewriting the generated cordis.yml moves no component line"
    );

    // The control: the patch file beside it is an input.
    write(&profile, "cordis.patch.yml", b"[]\n# one more byte\n");
    let moved = install.composite();
    assert_eq!(
        moved.profile_patch,
        digest_of(b"[]\n# one more byte\n"),
        "the profile patch is the digest of its own bytes"
    );
    assert_ne!(
        base.canonical, moved.canonical,
        "the fixture is capable of moving the composite at all"
    );
}

/// The profile manifest's two required members, by the reason each defect
/// raises. `bundles` and `patchReload` are read under different grammars —
/// a non-empty array of scalars, and a closed pair of strings — so a
/// missing member, a member of the wrong type and a member with an
/// invalid value are separate refusals wherever the grammar separates
/// them, and the same refusal where it does not (task 8.8(d), Pass D).
///
/// Every home is built on a `FixtureRoot`, the canonical spelling of a
/// temporary root: the reader canonicalizes the profile boundary before
/// it reads a manifest, so a home glued from a raw `TempDir` path names a
/// file by a spelling the producer never says on a host whose `$TMPDIR`
/// is reached through a symlink (review 2026-09-23, finding 2).
#[test]
fn the_profile_manifest_names_a_missing_mistyped_and_invalid_member_apart() {
    // `bundles`: the grammar admits a non-empty ARRAY, so absence and
    // every wrong type reach one reason, and a non-string ENTRY and an
    // entry the scalar rule refuses reach two more.
    for (case, bundles) in [
        ("absent", None),
        ("a string", Some("\"@deepseek-ai/dsh-base\"")),
        ("an object", Some("{\"0\":\"@deepseek-ai/dsh-base\"}")),
        ("null", Some("null")),
        ("a number", Some("3")),
    ] {
        let root = FixtureRoot::new();
        let home = root.path().join("home");
        let manifest = match bundles {
            Some(value) => format!(
                "{{\"dsh\":{{\"profile\":{{\"bundles\":{value},\"patchReload\":\"startup\"}}}}}}"
            ),
            None => "{\"dsh\":{\"profile\":{\"patchReload\":\"startup\"}}}".to_string(),
        };
        write(
            &home.join("profiles/headless"),
            "package.json",
            manifest.as_bytes(),
        );
        assert_eq!(
            refused(read_profile(&home)),
            "the DSH layout is unreadable: dsh.profile.bundles must be a non-empty array",
            "{case}"
        );
    }

    // `patchReload`: a missing member and one that is not a non-empty
    // string share `required_string`'s reason, and a well-formed string
    // outside the closed pair is the SEPARATE value refusal that names
    // the value it read.
    for (case, reload) in [
        ("absent", None),
        ("a number", Some("3")),
        ("null", Some("null")),
        ("an empty string", Some("\"\"")),
        ("an array", Some("[\"startup\"]")),
    ] {
        let root = FixtureRoot::new();
        let home = root.path().join("home");
        let manifest = match reload {
            Some(value) => format!(
                "{{\"dsh\":{{\"profile\":{{\"bundles\":[\"x\"],\"patchReload\":{value}}}}}}}"
            ),
            None => "{\"dsh\":{\"profile\":{\"bundles\":[\"x\"]}}}".to_string(),
        };
        write(
            &home.join("profiles/headless"),
            "package.json",
            manifest.as_bytes(),
        );
        assert_eq!(
            refused(read_profile(&home)),
            "the DSH layout is unreadable: dsh.profile: missing string 'patchReload'",
            "{case}"
        );
    }

    // Both admitted values, so the refusals above are the defects' alone.
    for reload in ["live", "startup"] {
        let root = FixtureRoot::new();
        let home = root.path().join("home");
        write(
            &home.join("profiles/headless"),
            "package.json",
            &profile_manifest(&["@deepseek-ai/dsh-base"], reload),
        );
        let profile = read_profile(&home).unwrap();
        assert_eq!(profile.patch_reload, reload);
        assert_eq!(profile.bundles, vec!["@deepseek-ai/dsh-base".to_string()]);
    }
}

// ---------------------------------------------------------------------------
// The measured rc.2 fixture (design D6, AK).
//
// Every input below is the LITERAL byte content measured on the installed
// pair and recorded in the qualification addenda; the expectations are the
// values those bytes yield. The constants were copied into this source
// while authoring: no test reads `.forge/` at build or run time, and no
// substitute was generated for a measured value.
// ---------------------------------------------------------------------------

/// The measured install materialized at D6's locators under a temporary
/// root: the rc.2 core with its complete hidden lock, the `headless`
/// profile with its complete pnpm lock and patch, and the UNCHANGED six
/// committed plugin files copied beneath the profile.
struct Measured {
    dir: tempfile::TempDir,
    seams: DshSeams,
}

impl Measured {
    fn new() -> Measured {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();

        let core = root.join("core");
        let pkg = core.join("node_modules").join("@deepseek-ai").join("dsh");
        write(
            &pkg,
            "package.json",
            br#"{"name":"@deepseek-ai/dsh","version":"0.1.5-rc.2","bin":{"dsh":"lib/bin.js"}}"#,
        );
        // The measured executable path and its exact first line.
        write_executable(&pkg, "lib/bin.js", b"#!/usr/bin/env node\n");
        write(
            &core,
            "node_modules/.package-lock.json",
            MEASURED_NPM_LOCK.as_bytes(),
        );
        // The two built-in bundles resolve under the CORE root.
        for bundle in ["dsh-base", "dsh-headless"] {
            write(
                &core.join("node_modules").join("@deepseek-ai").join(bundle),
                "package.json",
                format!(r#"{{"name":"@deepseek-ai/{bundle}","version":"0.1.5-rc.2"}}"#).as_bytes(),
            );
        }

        let home = root.join("home");
        let profile = home.join("profiles").join("headless");
        write(
            &profile,
            "package.json",
            br#"{"dsh":{"profile":{"bundles":["@deepseek-ai/dsh-base","@deepseek-ai/dsh-headless","dsh-plugin-cli-session"],"patchReload":"startup"}}}"#,
        );
        write(
            &profile,
            "cordis.patch.yml",
            MEASURED_PROFILE_PATCH.as_bytes(),
        );
        write(&profile, "pnpm-lock.yaml", MEASURED_PNPM_LOCK.as_bytes());
        // The plugin resolves under the PROFILE, as a real directory with
        // no nested dependencies, from the unchanged committed bytes.
        let installed = profile.join("node_modules").join("dsh-plugin-cli-session");
        for file in PLUGIN_FILES {
            write(
                &installed,
                file,
                &fs::read(plugin_dir().join(file)).unwrap(),
            );
        }
        // No home-level patch: the measured home has none.
        let bin = pkg.join("lib/bin.js");
        let head = selected_head(&bin);
        let executable = bin.to_string_lossy().into_owned();
        Measured {
            seams: DshSeams {
                executable,
                home,
                node: None,
                head,
            },
            dir,
        }
    }

    fn node(&self) -> NodeRuntime {
        NodeRuntime {
            path: self.dir.path().join("node/bin/node"),
            version: MEASURED_NODE.to_string(),
        }
    }

    fn composite(&self) -> DshComposite {
        dsh_composite_with(&self.seams, &self.node(), &[]).unwrap()
    }
}

/// The literal inputs are the measured bytes, byte for byte. Their
/// lengths and SHA-256s are the qualification record's, so a copy that
/// lost a line ending, a final newline or a character fails here before
/// any composite value is compared.
#[test]
fn the_measured_literals_are_the_recorded_bytes() {
    assert_eq!(MEASURED_NPM_LOCK.len(), 311_184);
    assert_eq!(
        digest_of(MEASURED_NPM_LOCK.as_bytes()),
        "b84bac2d866224a997be29811dc71bde6013dbc6e2adf8c1e77523e6f05a3847"
    );
    assert_eq!(MEASURED_PNPM_LOCK.len(), 1_982);
    assert_eq!(MEASURED_PNPM_LOCK.lines().count(), 57);
    assert_eq!(
        MEASURED_PNPM_LOCK
            .lines()
            .map(str::len)
            .max()
            .expect("the pnpm lock has lines"),
        186
    );
    assert_eq!(
        digest_of(MEASURED_PNPM_LOCK.as_bytes()),
        "4708752f0463211bf25d470fc26befa49748707b9c12fae7b4f2544e02b21055"
    );
    assert_eq!(MEASURED_PROFILE_PATCH.len(), 217);
    assert_eq!(
        digest_of(MEASURED_PROFILE_PATCH.as_bytes()),
        "ef189a8c27db6d63930aa3046a3040482e952eafcb7487c644d508e8d461f027"
    );
    // The hidden lock's own shape: 522 entries, every one carrying
    // integrity, and the core's own record among them.
    let value = lock(MEASURED_NPM_LOCK);
    let packages = value["packages"].as_object().unwrap();
    assert_eq!(packages.len(), 522);
    assert_eq!(
        packages
            .values()
            .filter(|entry| entry.get("integrity").is_some())
            .count(),
        522
    );
    assert_eq!(packages[CORE_KEY]["version"], "0.1.5-rc.2");
    assert_eq!(packages[CORE_KEY]["integrity"], MEASURED_CORE_INTEGRITY);
}

/// The three real npm path-grammar cases the measured lock contains.
/// Each binds a key to its TERMINAL complete spelling and to that same
/// entry's version and integrity: an implementation that took the first
/// group, split on every slash or read an ancestor's fields would
/// disagree with at least one of them.
#[test]
fn the_measured_npm_keys_bind_to_their_terminal_spellings() {
    let value = lock(MEASURED_NPM_LOCK);
    let packages = value["packages"].as_object().unwrap();
    let deps = npm_dependencies(&value, &[PLUGIN_BUNDLE]).unwrap();
    for (key, name, version, integrity) in MEASURED_NPM_PATH_CASES {
        assert_eq!(npm_name(key).unwrap(), name, "{key}");
        assert_eq!(packages[key]["version"], version, "{key}");
        assert_eq!(packages[key]["integrity"], integrity, "{key}");
        assert!(
            deps.contains(&format!("{name} {version} {integrity}")),
            "{key} contributes its own terminal triple"
        );
    }
}

/// The complete measured dependency set, as ordered values rather than a
/// count: 501 npm triples over 489 names — twelve of which legitimately
/// appear at more than one version or integrity and are all retained —
/// the four pnpm triples with their three exact npm overlaps, and the
/// 502 combined values in bytewise order.
#[test]
fn the_measured_locks_yield_the_complete_ordered_dependency_set() {
    let npm = npm_dependencies(&lock(MEASURED_NPM_LOCK), &[PLUGIN_BUNDLE]).unwrap();
    assert_eq!(
        npm.len(),
        501,
        "521 records deduplicate to 501 distinct complete triples"
    );
    let names: BTreeSet<&str> = npm
        .iter()
        .map(|triple| triple.split(' ').next().expect("a triple has a name"))
        .collect();
    assert_eq!(
        names.len(),
        489,
        "collapsing by name would lose twelve dependencies"
    );
    assert_eq!(npm.len() - names.len(), 12);

    let pnpm = pnpm_dependencies(MEASURED_PNPM_LOCK, &[PLUGIN_BUNDLE]).unwrap();
    assert_eq!(
        pnpm,
        MEASURED_PNPM_DEPENDENCIES.map(str::to_string).to_vec()
    );
    assert!(
        !pnpm.iter().any(|triple| triple.starts_with(PLUGIN_BUNDLE)),
        "the local-tarball record leaves the dependency lines: {pnpm:?}"
    );
    let overlaps: Vec<&String> = pnpm.iter().filter(|triple| npm.contains(triple)).collect();
    assert_eq!(overlaps.len(), 3, "{overlaps:?}");

    let composite = Measured::new().composite();
    assert_eq!(composite.dependencies.len(), 502);
    assert_eq!(
        composite.dependencies,
        MEASURED_DEPENDENCIES.map(str::to_string).to_vec()
    );
}

/// The measured observation: every component field the installation
/// yields, both bundle anchors, and the two digests the sole producer
/// computes over them.
#[test]
fn the_measured_install_yields_its_recorded_components_and_pinned_digests() {
    let install = Measured::new();
    let observed = install.composite();

    assert_eq!(
        observed.core,
        format!("@deepseek-ai/dsh 0.1.5-rc.2 {MEASURED_CORE_INTEGRITY}")
    );
    assert_eq!(observed.node, "v22.23.2");
    assert_eq!(
        observed.profile_bundles,
        vec![
            "@deepseek-ai/dsh-base",
            "@deepseek-ai/dsh-headless",
            "dsh-plugin-cli-session"
        ],
        "the declared order, not a sorted one"
    );
    assert_eq!(observed.profile_patch_reload, "startup");
    assert_eq!(
        observed.profile_patch,
        "ef189a8c27db6d63930aa3046a3040482e952eafcb7487c644d508e8d461f027"
    );
    assert_eq!(
        observed.plugin_patch,
        "84745a1bb00d773acf2e5ab5e32dc42825ffe164100ba469375dcabbbd5f9dab"
    );
    assert_eq!(observed.home_patch, "absent");
    assert_eq!(observed.extension, None);

    // The measured plugin directory is the six committed files and no
    // nested dependencies.
    let installed = install
        .seams
        .home
        .join("profiles/headless/node_modules/dsh-plugin-cli-session");
    let digests =
        plugin_file_digests("plugin", &installed, &PLUGIN_FILES, &read_dir_entries).unwrap();
    assert_eq!(
        digests,
        COMMITTED_PLUGIN_DIGESTS
            .into_iter()
            .map(|(name, digest)| (name.to_string(), digest.to_string()))
            .collect::<BTreeMap<String, String>>()
    );
    assert!(!installed.join("node_modules").exists());

    // Both anchors are real: two bundles resolve under the CORE root and
    // the plugin under the PROFILE. One anchor for all three would pass
    // this layout by accident or refuse it wrongly.
    let core_dir = install
        .dir
        .path()
        .join("core/node_modules/@deepseek-ai/dsh");
    let profile_dir = install.seams.home.join("profiles/headless");
    let core_root = install.dir.path().join("core").canonicalize().unwrap();
    let boundary = profile_dir.canonicalize().unwrap();
    for (bundle, under_core) in [
        ("@deepseek-ai/dsh-base", true),
        ("@deepseek-ai/dsh-headless", true),
        ("dsh-plugin-cli-session", false),
    ] {
        let resolved =
            resolve_bundle(bundle, &core_dir, &profile_dir, &[], &core_root, &boundary).unwrap();
        assert_eq!(
            resolved.starts_with(&core_root),
            under_core,
            "{bundle} resolves at the {} anchor",
            if under_core { "core" } else { "profile" }
        );
        assert_eq!(
            resolved.starts_with(&boundary),
            !under_core,
            "{bundle} resolves at the {} anchor",
            if under_core { "core" } else { "profile" }
        );
    }

    // The two digests, pinned from the sole producer's output over these
    // measured inputs. They are this FIXTURE's result: they make no claim
    // about a retained-home recording or task 8.8's acceptance.
    assert_eq!(
        observed.plugin, MEASURED_PLUGIN_COMPONENT,
        "the plugin component over the measured inputs is the producer's pinned output"
    );
    assert_eq!(
        observed.canonical, MEASURED_CANONICAL_COMPOSITE,
        "the canonical composite over the measured inputs is the producer's pinned output"
    );
}

/// The guide's doctor sample shows what this producer actually emits.
/// A sample nobody checks drifts into fiction — the inherited one had
/// already become a digest no installation produced — so the values in
/// it are asserted against the fixture's own output, not transcribed.
#[test]
fn the_guide_sample_shows_the_producer_s_measured_fixture_output() {
    let observed = Measured::new().composite();
    let guide = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/guides/provider-adapters.md"),
    )
    .expect("the provider-adapters guide");
    let line = guide
        .lines()
        .find(|line| line.starts_with("ok       dsh:"))
        .expect("the guide shows a dsh doctor line");
    assert!(
        line.contains(&format!("composite {}", observed.canonical)),
        "the sample's canonical digest is the producer's: {line}"
    );
    assert!(
        line.contains(&format!("plugin {}", observed.plugin)),
        "the sample's plugin component is the producer's: {line}"
    );
    assert!(
        line.contains("(no declared wrapper_digest)"),
        "no declaration is pinned by this slice: {line}"
    );
    // The bound's phrase in the guide's unreadable sample is this
    // producer's, character for character. Doctor's half of that
    // sentence is asserted against `composite_detail` in the CLI suite,
    // where the classifier lives.
    //
    // The refusal is provoked by a FINITE oversized file rather than by
    // `/dev/zero`: that path exists only on Unix, so on Windows the
    // sample was compared against a "no such file" reason instead of the
    // bound's, and the guide assertion failed for a reason the guide has
    // nothing to do with (council return 2026-09-19, F10).
    let dir = tempfile::tempdir().unwrap();
    let oversized = dir.path().join("pnpm-lock.yaml");
    fs::write(&oversized, vec![b'#'; PNPM_LIMIT + 1]).unwrap();
    let bound = refused(read_pnpm(&oversized));
    assert_eq!(
        bound,
        "pnpm lock is unreadable: pnpm lock exceeds 8388608-byte limit"
    );
    assert!(
        guide.contains(&bound),
        "the guide's unreadable sample quotes the producer's own reason: {bound}"
    );
}

/// The measured composite moves with each of its inputs. Every mutation
/// below is a byte the identity is supposed to cover, so a serializer
/// that dropped a line or an order would be caught by the one that no
/// longer moves.
#[test]
fn the_measured_composite_moves_with_every_component_it_names() {
    let install = Measured::new();
    let base = install.composite();
    let profile = install.seams.home.join("profiles/headless");

    // A reordered bundle list: the lines are declared-order, not sorted.
    write(
        &profile,
        "package.json",
        br#"{"dsh":{"profile":{"bundles":["@deepseek-ai/dsh-headless","@deepseek-ai/dsh-base","dsh-plugin-cli-session"],"patchReload":"startup"}}}"#,
    );
    let reordered = install.composite();
    assert_eq!(base.dependencies, reordered.dependencies);
    assert_ne!(
        base.canonical, reordered.canonical,
        "the declared bundle order moves the measured composite"
    );

    // A switched reload mode.
    write(
        &profile,
        "package.json",
        br#"{"dsh":{"profile":{"bundles":["@deepseek-ai/dsh-base","@deepseek-ai/dsh-headless","dsh-plugin-cli-session"],"patchReload":"live"}}}"#,
    );
    let live = install.composite();
    assert_eq!(live.profile_patch_reload, "live");
    assert_ne!(
        base.canonical, live.canonical,
        "the reload mode moves the measured composite"
    );
    write(
        &profile,
        "package.json",
        br#"{"dsh":{"profile":{"bundles":["@deepseek-ai/dsh-base","@deepseek-ai/dsh-headless","dsh-plugin-cli-session"],"patchReload":"startup"}}}"#,
    );

    // One byte of the profile patch.
    let mut patched = MEASURED_PROFILE_PATCH.to_string();
    patched.push('\n');
    write(&profile, "cordis.patch.yml", patched.as_bytes());
    let moved = install.composite();
    assert_ne!(base.profile_patch, moved.profile_patch);
    assert_ne!(
        base.canonical, moved.canonical,
        "the profile patch moves the measured composite"
    );
    write(
        &profile,
        "cordis.patch.yml",
        MEASURED_PROFILE_PATCH.as_bytes(),
    );

    // A home-level patch where the measured home had none.
    write(&install.seams.home, "cordis.patch.yml", b"[]\n");
    let homed = install.composite();
    assert_eq!(homed.home_patch, digest_of(b"[]\n"));
    assert_ne!(
        base.canonical, homed.canonical,
        "the home patch moves the measured composite"
    );
    fs::remove_file(install.seams.home.join("cordis.patch.yml")).unwrap();

    // One byte of one plugin file.
    let installed = profile.join("node_modules/dsh-plugin-cli-session");
    write(&installed, "README.md", b"drifted\n");
    let drifted = install.composite();
    assert_ne!(base.plugin, drifted.plugin);
    assert_ne!(
        base.canonical, drifted.canonical,
        "a plugin byte moves the measured composite"
    );
}

/// The permitted nested-dependency layout and the conditional extension,
/// proved separately and visibly as SYNTHETIC additions to the measured
/// tree: the measured rc.2 plugin has neither.
#[test]
fn a_nested_dependency_subtree_and_a_listed_extension_are_separate_cases() {
    let install = Measured::new();
    let base = install.composite();
    let profile = install.seams.home.join("profiles/headless");
    let installed = profile.join("node_modules/dsh-plugin-cli-session");

    // SYNTHETIC: the measured plugin has no nested node_modules. A
    // direct, real one is the permitted exception and changes nothing.
    write(
        &installed,
        "node_modules/left-pad/index.js",
        b"module.exports=1\n",
    );
    assert_eq!(install.composite().plugin, base.plugin);
    assert_eq!(install.composite().canonical, base.canonical);
    fs::remove_dir_all(installed.join("node_modules")).unwrap();

    // SYNTHETIC: the measured profile names no extension.
    write(
        &profile,
        "package.json",
        br#"{"dsh":{"profile":{"bundles":["@deepseek-ai/dsh-base","@deepseek-ai/dsh-headless","dsh-plugin-cli-session","brokkr-dsh-resume-policy"],"patchReload":"startup"}}}"#,
    );
    for file in EXTENSION_FILES {
        write(
            &profile.join("node_modules/brokkr-dsh-resume-policy"),
            file,
            file.as_bytes(),
        );
    }
    let extended = install.composite();
    assert!(extended.extension.is_some());
    assert_ne!(base.canonical, extended.canonical);
}

// ---------------------------------------------------------------------------
// Measured constants (design D6, AK).
//
// Copied into this source while authoring from the qualification addenda
// recorded on 2026-09-19 for the installed `@deepseek-ai/dsh` 0.1.5-rc.2
// pair. Nothing here is read at build or run time: a measured value that
// could not be copied was never invented, and a derived list is an
// expectation only, never an input to the producer.
// ---------------------------------------------------------------------------

/// The committed six-file digest map from `extensions/dsh/plugin-cli-session`
/// PROVENANCE, which the measured install reproduced file for file.
const COMMITTED_PLUGIN_DIGESTS: [(&str, &str); 6] = [
    (
        "LICENSE",
        "7a9d5a3b08b3802d77eb237283c26ce6ae346af58b86b1b0d655dada45416915",
    ),
    (
        "README.md",
        "c92c60d057e456beea6c0cd27bad26bf60f9a8909a852c96b36eceaf802cfc1d",
    ),
    (
        "cordis.patch.yml",
        "84745a1bb00d773acf2e5ab5e32dc42825ffe164100ba469375dcabbbd5f9dab",
    ),
    (
        "lib/index.js",
        "325eccc0d67de1dcea79a3c2d89eebb139b10970d0b7efd6dd1b7e3e6949fe85",
    ),
    (
        "lib/startup.js",
        "3526be1cd885f99592f1cfb5133f065879411f13bffba2672a74a55a11e66491",
    ),
    (
        "package.json",
        "7e96b1467153b4aed91e65e52ab03ebecf901f52a1f437484da14382e8a4a0fb",
    ),
];

/// The core's measured registry integrity, recomputed from the downloaded
/// tarball and matched against the hidden lock's own record.
const MEASURED_CORE_INTEGRITY: &str =
    "sha512-8Xc8hCQHcIWRmTCVU/xZdp6/qMsWMeAd2ObChKDEsfhUPJFXx6H0lgeb1DxUMD86HZrrVN+1bCvn1ppjZ/fOxw==";

/// The Node runtime the measured executable's `env node` shebang found.
const MEASURED_NODE: &str = "v22.23.2";

/// Three real path-grammar cases present in the measured hidden lock:
/// key, terminal complete spelling, and that entry's own version and
/// integrity. Two are nested scoped packages whose ancestor carries a
/// different spelling; the third is a direct scoped one.
const MEASURED_NPM_PATH_CASES: [(&str, &str, &str, &str); 3] = [
    (
        "node_modules/@aws-sdk/credential-provider-http/node_modules/@smithy/node-http-handler",
        "@smithy/node-http-handler",
        "4.12.1",
        "sha512-ThMkboGeONWXAelq9FvGsuJC4rOi+qyC4/zhUF58xYpxUg5sQKx2VXZYJmtNjr4dSuBJ1HeJXETQILCz3wOHvw==",
    ),
    (
        "node_modules/@aws-sdk/credential-provider-sso/node_modules/@aws-sdk/token-providers",
        "@aws-sdk/token-providers",
        "3.1129.0",
        "sha512-Sbl3rpzQdsG4ZK2zh0JWUYyZPKKorJlVOddA2T0DVbKJFrsW8J6wgnslxxUH04+WaBMr4A1HzJZvZX0xUvkniA==",
    ),
    (
        "node_modules/@anthropic-ai/sdk",
        "@anthropic-ai/sdk",
        "0.123.0",
        "sha512-Y9oX9mPNGZClHQOFqrWRk43Srcu/UHuPq3rfxxOq7JgW0gi+lJA2MAOK4Ul3k/+AUrwRWFJvd0tK3oC0Pw25dw==",
    ),
];

/// The four triples the measured pnpm lock yields once its local-tarball
/// record is excluded.
const MEASURED_PNPM_DEPENDENCIES: [&str; 4] = [
    "@deepseek-ai/cosmokit 1.8.3 sha512-qBo+ronVM6Eu2WNVJXi8JcMiqZ19T9BRIpV+5qJUFPXjGH/Z0QKcQMC/IZJ7L394YTOtJgcovbk9qP0w2GsBXQ==",
    "@deepseek-ai/schemastery 3.18.1 sha512-Qn0FCSwCQnpnj6SB31I6i2sIKgKWnkbJM8O0EU91Gv2UsYVvtZTl6IA0sCwk2e2MZf5S8w5hpq9QkeVvK9qwxg==",
    "@standard-schema/spec 1.1.0 sha512-l2aFy5jALhniG5HgqrD6jXLi/rUWrKvqN/qJx6yoJsgKhblVd+iqqU4RCXavm/jPityDo5TCvKMnpjKnOriy0w==",
    "commander 15.0.0 sha512-z67u4ZhzCL/Tydu1lJARtEZYWbWaN7oYLHbsuzocr6y4N6WZAagG3RQ4FW61V1/0+jImpj293XfrcYnd1qxtPg==",
];

/// The complete measured dependency set in bytewise order: the 501
/// distinct complete npm triples merged with the four pnpm ones, whose
/// three exact overlaps collapse to 502 values. A count alone would not
/// separate a correct reader from one that collapsed by name, so the
/// values themselves are the expectation.
#[rustfmt::skip]
const MEASURED_DEPENDENCIES: [&str; 502] = [
    "@agentclientprotocol/sdk 1.4.0 sha512-/eufudw+aFY1LKLolT6yFE6UMmYRl7fMJ/DEONSIyR6wI3slHWITBsANRGqXEY8FRzqUxwh7QEaGiZHcJPVThg==",
    "@anthropic-ai/sdk 0.123.0 sha512-Y9oX9mPNGZClHQOFqrWRk43Srcu/UHuPq3rfxxOq7JgW0gi+lJA2MAOK4Ul3k/+AUrwRWFJvd0tK3oC0Pw25dw==",
    "@aws-crypto/sha256-browser 5.2.0 sha512-AXfN/lGotSQwu6HNcEsIASo7kWXZ5HYWvfOmSNKDsEqC4OashTp8alTmaz+F7TC2L083SFv5RdB+qU3Vs1kZqw==",
    "@aws-crypto/sha256-js 5.2.0 sha512-FFQQyu7edu4ufvIZ+OadFpHHOt+eSTBaYaki44c+akjg7qZg9oOQeLlk77F6tSYqjDAFClrHJk9tMf0HdVyOvA==",
    "@aws-crypto/supports-web-crypto 5.2.0 sha512-iAvUotm021kM33eCdNfwIN//F77/IADDSs58i+MDaOqFrVjZo9bAal0NK7HurRuWLLpF1iLX7gbWrjHjeo+YFg==",
    "@aws-crypto/util 5.2.0 sha512-4RkU9EsI6ZpBve5fseQlGNUWKMa1RLPQ1dnjnQoe07ldfIzcsGb5hC5W0Dm7u423KWzawlrpbjXBrXCEv9zazQ==",
    "@aws-sdk/client-bedrock-runtime 3.1048.0 sha512-u+NT61JZEkRFtpL0CAw1N1dwxnaLgwVXQl/zjJxTGgLyS/jTIdg2SdoEoCTHxgDyCnqa1HEi9QOoE9/pYRNpOQ==",
    "@aws-sdk/core 3.978.0 sha512-2yX9LUmxPklVjSGTb8dfnWRJSiFQ3TeH2nn7G1mdKHTfnabzF0+gfrS8rYfLWmZrQ8A3mEcxMJjRc51dL5KWaA==",
    "@aws-sdk/credential-provider-env 3.972.71 sha512-JN+JHruYZw3GUZB8YGAlDk4wTDPOEAEEdEzj5nS0xodWR4smzHsN7PnK2j6IeOsDIj2aqua5DSbhXl9Gtf90FQ==",
    "@aws-sdk/credential-provider-http 3.972.73 sha512-uyYYnJOnlis8uQzaYGPd7N1JoioCoNpXgnkXYixsWJXHXgXyYi8WXJSDfofxJeWfQIGWLe2Nwyq60Uc7MZdVOg==",
    "@aws-sdk/credential-provider-ini 3.973.16 sha512-i++ly+0Uxa+u3ebSSyr0S/3CFhFJDxCXT3+Zj+mW2bXenEx5bKGCdTIKFu39SgXBNhWDjex/8cXUx9MUTMCrTw==",
    "@aws-sdk/credential-provider-login 3.972.78 sha512-eUtswnXu0+Ii9ieRK+0L7aPFV3Z/dnW2VntJzjBP9xs8s+8p5nBNuymIXtXwZ+5r5+XJP3e32nMkuZ/r0HozEA==",
    "@aws-sdk/credential-provider-node 3.972.83 sha512-jdso7ejzfRnatxMUZK4S/U6KbaDPCvfIV4XL+IQAPFDBt5rj5Fq595euqlK8Le4lNCMFR9oUpt+1l0aMgaayOQ==",
    "@aws-sdk/credential-provider-process 3.972.71 sha512-lYmXJa4gvq4xN1lrT5NiP5vIYYKcGWAdj8y+8o6dlcateB5eF3Dn8DtmjjHKfMBrTPAMr2pebIiX/UOj8c1/UA==",
    "@aws-sdk/credential-provider-sso 3.973.15 sha512-6Jhcf4v0pSFdjk1EW2kvzuEBKD+UZ2uNcHUIglKKLndD20YhvkL2kdmDOV5/j4mYuWWwe/a1FQ1aomU86/Cg5Q==",
    "@aws-sdk/credential-provider-web-identity 3.972.77 sha512-uylIQSUWpfLuH2LovxEEfwzJGM/SabLOfLMg6YXu/E8jJEKUdpdILCVCQCdFvHyu/7dLJOHPMfrSwduxO56NkQ==",
    "@aws-sdk/eventstream-handler-node 3.972.34 sha512-cTeVzpu1xEAkryTZBYhGwnQ6gOGyp8ZYZvmn0Sg/nI/ABmy/CRHHxPDJDUi9PxwxUtGGaatvfRUB3FCgT/rSWw==",
    "@aws-sdk/middleware-eventstream 3.972.29 sha512-dlRzHCgyB8W6hLuDC5pcT5q+ziPt00n4QGgGBE17ucLVU4zMa6lsbuUdQ2Pm75Z5VA8GF+R/+SgrRcaTdIzSIQ==",
    "@aws-sdk/middleware-websocket 3.972.53 sha512-bIrDaMENQmYRBHntOiOheqkiw5+fhKW4Lqb+mS1uqF0VwvdWI22fW2HFgWrng66CmYd+4k8ePlpj38sEfTuMLQ==",
    "@aws-sdk/nested-clients 3.997.45 sha512-mooq9Q+jLa18VoM7HouczmslZU60iiB0aKc/Ztnq/luIL1ud0z4DnYprLR/ZO1gp331S9tJctM1HZr7u6YKBXQ==",
    "@aws-sdk/signature-v4-multi-region 3.996.46 sha512-L+2xZTye/2T96f3lwCws0Zw6GG2JHZW9e8FpVgGBeeExSKyeoZ6CWRpBml/7DNiK/O26jrgPM9F+Ay8VkgzUWQ==",
    "@aws-sdk/token-providers 3.1048.0 sha512-k0y/GcuesuSfWyUM0WamrGyeZmltRYaPbHO82UDA6mZ/doB+FOHKutikPAtSXMn/hDz970cF+iRuuiYO9VEbAA==",
    "@aws-sdk/token-providers 3.1129.0 sha512-Sbl3rpzQdsG4ZK2zh0JWUYyZPKKorJlVOddA2T0DVbKJFrsW8J6wgnslxxUH04+WaBMr4A1HzJZvZX0xUvkniA==",
    "@aws-sdk/types 3.974.5 sha512-LkwLL2BLbC6wNNm4JaH9mbEqBMdOZCct6VAYqhdN4U1xrWM+fUJQEfbHwQgDypapOWTRtlk25akb5afM0P8CIQ==",
    "@aws-sdk/util-locate-window 3.965.10 sha512-ycwH6Zd2GhuSqdXX9ihbCjeGTB6xOJs+O3+Jb8/zDG9978XU80qs75dfkPJRMNKe5MvBZPuNeFpd4JZKPoUF4g==",
    "@aws-sdk/xml-builder 3.972.40 sha512-wlFmCIGUlwF4zx/kncw+bmxTQh1HeSJq4mYV/V5cZUSJadDP3kXvGW8Rn21cimj/7y9ju+47oYWXi97vF7czaA==",
    "@aws/lambda-invoke-store 0.3.0 sha512-sl4Bm6yiMNYrZKkqqDFWN0UfnWhlS8ivKxrYl+6t0gCLrqr8y3B2IqZZbFRkfaVVp7C/baApyh71P+LeE1A2sQ==",
    "@babel/code-frame 7.29.7 sha512-Aup7aUOfpbAUg2ROOJN6Iw5f9DMBlzu0mIkm/malLQFN/YQgO48wCj0Kxa3sEHJvPVFg7siR+qRInwXd2qhQKw==",
    "@babel/helper-validator-identifier 7.29.7 sha512-qehxGkRj55h/ff8EMaJ+cYhyaKlHIxqYDn682wQD7RNp9UujOQsHog2uS0r2vzr4pW+sXf90NeeayjcNaX3fFg==",
    "@babel/runtime 7.29.7 sha512-Nq8OhGWiZIZGV6hLHoyAKLLcJihP/xFeBMGJoUrxTX2psI8dCifzLhZISFb+VWS3wFMRDmCGw5R+dOySCqPLhw==",
    "@deepseek-ai/cordis 4.0.2 sha512-asOnXP1TzFSFQlHb1iegDZp0z/8WD1c7YNrwJR/Tx2bzNuMXfcekE/I67Iv6SQXeLB4csxqCngzQKANP7gdw0g==",
    "@deepseek-ai/cordis-plugin-group 1.0.2 sha512-OeGiPD793Mhma9+rGIox95neuufwOFTqQ5jooFgrMK/Mn7Fp7Hkv/d7VKMXZz40iks2fZCbiFE6p9WjTVTQRdg==",
    "@deepseek-ai/cordis-plugin-hmr 1.0.17 sha512-o1BooaJpwd+C80Tn5+B48MGqrfNZydlfRVQjmCrrULA4nCmlMqBKjDF2FmcZoNP+L2uX9m/6DdGKVtFpQRQVUQ==",
    "@deepseek-ai/cordis-plugin-include 1.0.7 sha512-bZ4S1YuOmwOeE97HnslQ6ggVQbaWz4GejJ8OcY4P/DIyc1Qhu/3Szt1XSw6c/pd37kRkxxJXGwt4cmCxWSBBuw==",
    "@deepseek-ai/cordis-plugin-loader 1.0.3 sha512-YNcHiH7TRFrgnRbQU7qdS1spABUFFHg04QpB9DdboPEjgKLTFfeEAvr6Y7qll3/nGgqywDQdk1tYshqKO0p1WQ==",
    "@deepseek-ai/cordis-plugin-timer 1.1.4 sha512-GhxOiU+TF2BbNBKlV2N24zvfv2Kh7RxkFW1hjML6s+DOb+y2zomzbeC5zHIIjsc/rpRwbKpDNucPGSwkH0ChXw==",
    "@deepseek-ai/cosmokit 1.8.3 sha512-qBo+ronVM6Eu2WNVJXi8JcMiqZ19T9BRIpV+5qJUFPXjGH/Z0QKcQMC/IZJ7L394YTOtJgcovbk9qP0w2GsBXQ==",
    "@deepseek-ai/dsh-acp 0.1.5-rc.2 sha512-0lmDF2qyy136fTFN7jowBVBRVQ00YASi1umgnFSVpHQ0rDKh32tpl7xWaoOTlh6mmKh04A93qt92QhPLIzkcSg==",
    "@deepseek-ai/dsh-acp-app 0.1.5-rc.2 sha512-90SRAmF6haHARulVllv6uh88n+zubdrgDfxvTzmi9+p3ktt7o6oZ6+yKO7I7e04bSskT+Pz/1J0FLugi9l39dw==",
    "@deepseek-ai/dsh-agent 0.1.5-rc.2 sha512-SlUL1riZmVLwMUR3jo9CP/R1cxov9dHkCJDh6JQW3fSZJVCIdPygBRlAweCUDvHxAEmPpFHxE/U3NmSUbX+vQQ==",
    "@deepseek-ai/dsh-agent-default-model 0.1.5-rc.2 sha512-B2e3e1iT+iSE2Pf7bnrKmsbOPiD+ZXplVhL98K35y0IGX7yUdqATsyKVPEHjgfCGooNKjGlSBB42kUk3s7Iv/A==",
    "@deepseek-ai/dsh-agent-instructions 0.1.5-rc.2 sha512-735wmeuRDr/fB9vCnBALEHE7z04Lmhx6WX3KJLE3DjWMGbmTudU5i0iYSYst2mZh7C/igV5wXBveIGKLPIrL+A==",
    "@deepseek-ai/dsh-agent-loop 0.1.5-rc.2 sha512-24wvqVlqFmdqJ2Bhcku/vKeNy+qWSmeEeN7lvcUB+WkhcF0/Ra2G3WwcVmnahJA4+w0AKdW3W7v+YYcO/jKJpg==",
    "@deepseek-ai/dsh-agent-presets 0.1.5-rc.2 sha512-h0j7IfuHPYh/oJR7SDxh6pzSgUeaHwaaAIg5X0RNL6Z2oQrq0oUmjtArCVVw7nkgt48DjdBwx4EwJ9IdNrHxug==",
    "@deepseek-ai/dsh-agent-tool-presentation 0.1.5-rc.2 sha512-XA4Z3zm73SQw4VmWdo2Z/HdRr3YCWL7SseO70wIuYbcjfar1BdMC0S6/KXTgvshn1lCuCqf7I47B1e5oxoHKhg==",
    "@deepseek-ai/dsh-anonymous-user-id 0.1.5-rc.2 sha512-YIpZoi8mY/d+BjnPOryDYR7lkwr2uWh5KJWjIqm2sGN+C6imc66lDiSi5wy91JMRzefFWJf26fCiYXAF7ErQAw==",
    "@deepseek-ai/dsh-api-gateway 0.1.5-rc.2 sha512-jYDsxVyRnRI++f+hXqBJemOk6ASTvly1sO8ZRuoI77RR22r0nfTFiifxevbmUcavl6j9fgB/LL3X7BpXMy5QQg==",
    "@deepseek-ai/dsh-api-remotes 0.1.5-rc.2 sha512-O8t/aJiHatgzlQIgXx6xdmpy+iAXJ/Zab1xMHIx7Noo6mzSYUIxwfJkoYgPv9G0Yfx06FPoRTUtLiZ8IADqUOA==",
    "@deepseek-ai/dsh-api-session-controller 0.1.5-rc.2 sha512-rwOxS6piZ9EuLgE/pbq4cUlRCGv98IKgdFnK5v8WNwZYKug3rUpvYODDU9vE42ydmClgjFd/KIAaEDFPatcDpA==",
    "@deepseek-ai/dsh-api-settings-controller 0.1.5-rc.2 sha512-HcnffXfBLi8Xaq3i8DZDDMZHkekW3RjRvA1XqnG/1SLR2w9OEt5P3oY2VzO0UuhyBlYCjS61BwGxDa5zWLCPlw==",
    "@deepseek-ai/dsh-api-workspace-controller 0.1.5-rc.2 sha512-95USICv+Ds+BS4Bjf27gVneXc/ZuhlYvTsdfjx/S+MMSutjgV2eZDKkQOxVn8u8KMZw3FG0X5Era47qa3kg01Q==",
    "@deepseek-ai/dsh-api-workspace-files 0.1.5-rc.2 sha512-Kx4ksBWQhN2Sbj48TPwC9cM0cxltIKzjyzanYog0d1WRnzxtqPDHthi6fx1I6RYA+k6257/xLi/Vx/ThYJ1ppQ==",
    "@deepseek-ai/dsh-app-boot 0.1.5-rc.2 sha512-beM+ULhjr2mGoyrWta3F6HOQ9OD+i5tKG8BU0RtgVGmXDjA57cVEAiEkkgPLMTxbbzU+xsFP61I5SApna4Cz5A==",
    "@deepseek-ai/dsh-atomic-write 0.1.5-rc.2 sha512-9bCOLkug83IGuoEBPHUxgfJ/IxsHg/UigGi8Oj0agibcGVq2WdL/fbU8KA3KU5H49KDOxq2/AA3XUI3RiAEtYA==",
    "@deepseek-ai/dsh-attachment 0.1.5-rc.2 sha512-S6b8/WjqzGw+dMDLRXnq+tbijDGkQh38yE+zpQytX2/w/mPR3VzGj5r6McS01WwD76vXR8WFoheSCLyCAro8WQ==",
    "@deepseek-ai/dsh-attachment-local 0.1.5-rc.2 sha512-UN9Zw6oBqHl/SQxFQCGUp8OcARQkPzZo4BGNkSQHqSsEo3d59Mv7H5mSFHmKl4NogSMwksjN3SF0FocN7xyF2A==",
    "@deepseek-ai/dsh-authorization 0.1.5-rc.2 sha512-1AtUlo72eYUwiiYL728E4oLgGfP3Kq1yiftiKv0hZ9AYzwZRMIGxv1659UymViNQ/m3BIEPlfiv+vWIDHkqMgQ==",
    "@deepseek-ai/dsh-base 0.1.5-rc.2 sha512-a4QqqqnN/qmWmIvFdCsXXlPerI3IhyIXh0UdLX1Wo1Q6P+blCIkQDG4LLLNS4RoWjhDURoKm2t51NBjotD6BVg==",
    "@deepseek-ai/dsh-bash-local 0.1.5-rc.2 sha512-a/FTpYUKY0Sorn4GlY2HjuQh7+gjU8XQ2+HRDN0tuqMQyO1RIzJi62FjtZSZ6+qV4EJJ8o8h0+8URHsVrzro+Q==",
    "@deepseek-ai/dsh-bash-sandbox 0.1.5-rc.2 sha512-y8vwK6jf4gPq8mG71zWure803NjfqJ66Nvzr1FTB8An+IF68hT2eU+hHJCByvP0u6FuiLPRptuX/im2CxcNo1g==",
    "@deepseek-ai/dsh-brand 0.1.5-rc.2 sha512-/+3TzQRYT4M8NINZ9OrsL5VWNyQSXof8mFLx3txiosWA7Bt0H6UcxUdKGXVwj6gCXhsQ74mzEHm2ucsflG0mYA==",
    "@deepseek-ai/dsh-chunked-list 0.1.5-rc.2 sha512-XPr311bleMyOh9SzhTMmc1mC6qx2Tty8315MNYYyzIOLAns0h27PReFa9axy8I0soBaE7gfp6eQI1ZriVkI68g==",
    "@deepseek-ai/dsh-client-connection 0.1.5-rc.2 sha512-W0GAZX01hAfrfjoZbtwEJ5ik3dG20Hy/0TFYJ6dnvwxtcacGYNIwi3f2oHpZgLcEw+PWwq1Y9IV1iKG+yDOytg==",
    "@deepseek-ai/dsh-client-file-upload 0.1.5-rc.2 sha512-Y0v4pr1btn1fY8ox1cxIuxDhm2RKHrztfIDMhzj2Z1xRCdSdfbZYtkYvOdWS4uAbpZWQKs5gDMGgASvwTCJ+lg==",
    "@deepseek-ai/dsh-client-hmr 0.1.5-rc.2 sha512-Ra1JziRbpV5CiIaYUT4FbBKko1m+PONCGJjmKM7PyHSAv00v53/xKREHVB1SNqu/QT9cGs688ID2KHuwFxbHSA==",
    "@deepseek-ai/dsh-client-locale 0.1.5-rc.2 sha512-kv56ki/WQWagsHt94wJAPzsiKnxa8KlmM33bZGgl2SgF4atlQ3iRCdkwJ3fQARXF4Ocnfutivoj5fu5usks1Sw==",
    "@deepseek-ai/dsh-client-modules 0.1.5-rc.2 sha512-034DxLlGvX4GgkqqFN2SGcQx8hKdicj5IrgENclBXXbyMDlpF9xADiWi5DxPms+iOBcL/5LLe84q/GCTWOrA2g==",
    "@deepseek-ai/dsh-client-resources 0.1.5-rc.2 sha512-fl0saN8LKcAZ/YUSqxBeAbzVj6ZSBWnWrKGFUqhuq5N0XHWu4CpswFIruKxkvgl7lwAsdTtbL3wMtsKIupSmNQ==",
    "@deepseek-ai/dsh-client-ui-agent-preset 0.1.5-rc.2 sha512-BMhRU54q7/8E/JoksXm9TW1rCBBg2V64XsDwltYWmCLcZuLCVBc3GX7t/11WMERNi8Do+0ZM6khslwrhbT5QLw==",
    "@deepseek-ai/dsh-client-ui-approval 0.1.5-rc.2 sha512-jrlEBEYfYq04HuKYBxD1yI5Z9btJ4fnuWbz3xtK8M/gvsTye8PrhCMxaNARMznGiUFoXmwv9CRhl/KAqbbfE/g==",
    "@deepseek-ai/dsh-client-ui-attachment 0.1.5-rc.2 sha512-+i98BXfTpz9HX0G7m7k9ka1R5Tekd0tBIljFtKMmiXWju5MW1u+dngpmGCwoSHenEuZGZDC8K/qgw7MIOcbRnw==",
    "@deepseek-ai/dsh-client-ui-brand-official 0.1.5-rc.2 sha512-VESOQ7N8MmUNyy5H1A099Zx3NW1DTTcFIdN/zYVTXWKukmSR6xanShJMai6e69ibDmUfh0YmRgwOvHOVd2K0Uw==",
    "@deepseek-ai/dsh-client-ui-chat 0.1.5-rc.2 sha512-ywvtoJUgd/vkfETuFDIgqrvdR5yzfvBDqpuyw0pFDHaHwLzRtk/xPjraI9blNoLWQ8x/Sp42aS4Pr7XQPfjIvQ==",
    "@deepseek-ai/dsh-client-ui-commands 0.1.5-rc.2 sha512-wXSTOM4DHogdXZp8tp5RhEvpRJbNey/lN7yjCNN5Qg+BjSu6BAVwKLRKiriCmKIKJcEEpq4BFQb9Di4c1GMm1w==",
    "@deepseek-ai/dsh-client-ui-conversation 0.1.5-rc.2 sha512-VnZ0VrmI7+1JH/iMYV6+FaxCsZrVk4CZIuG+k1HkeevHgUPHe3ghLo8u1Qt9cx00MEH5i3bLOY5Nka3w4mbA6g==",
    "@deepseek-ai/dsh-client-ui-cordis 0.1.5-rc.2 sha512-wLolxQURdqIXNLcoGmiXb6tE7dPD5v5eBBu7WtUn/w7FNlp4QambxpCfX/ytGy8LtKO42LwhYk9uPPr51COorQ==",
    "@deepseek-ai/dsh-client-ui-deliverables 0.1.5-rc.2 sha512-RO2XqjCOZfCSkMzlZf1rgg1ZYhLO6Uy2FUsTwkPContjKCGnGWUMtcaRS8LTs1lYgH8+hP50LIf67o/6z8FJxA==",
    "@deepseek-ai/dsh-client-ui-directory-picker-browse 0.1.5-rc.2 sha512-DrrcPERnA50Il/1Roc6T3goodqANi0u+nVtQShw2P3FHBwVqdlfUURMQgZgKdHO4JFv2dGnikwf+XhiE41icWw==",
    "@deepseek-ai/dsh-client-ui-directory-picker-native 0.1.5-rc.2 sha512-bH3VBqhlXcmkEcnPrnRLecDW9uOdvsaTzfZ8zTUh5ZtexwNhBs6Wv3lN6gZNw457jbP4BnQoEar5fq+rIC5RTw==",
    "@deepseek-ai/dsh-client-ui-goal 0.1.5-rc.2 sha512-38+k8tQq61nQjHYLwYfrwOoju+Jlg2hB0xWJrMvPfcZlB5Shjtthm50lW362tK6iJ9GMSfOGIPGuC+nooBD/Yw==",
    "@deepseek-ai/dsh-client-ui-input-trigger 0.1.5-rc.2 sha512-Am7qlXQX+Kf3IddzoJDy2/kgtHm3R5hcUFjDQ8tespSWXswPEX+9wsPFugUvjHJkqdpfeE9OTI3jnd8K6sT/BA==",
    "@deepseek-ai/dsh-client-ui-jobs 0.1.5-rc.2 sha512-/ygltqocItoM+Jh+tnyYJwIpAPTSu6bAWfEkyBunHpHllCjONuFmnpPcLPZTOxKwkjTdQT6XTRdMhmxU7Sageg==",
    "@deepseek-ai/dsh-client-ui-layout 0.1.5-rc.2 sha512-N5+kH1W6UjzOuagJEKDenm/Gbs8Y2sz6btdmVKW5dypEEUgVWsQGV0auTC37gsWKWDaWGpfYOpeAa3Ybk6DYbQ==",
    "@deepseek-ai/dsh-client-ui-message-feedback 0.1.5-rc.2 sha512-LSp6c87DLMiO4B0JCpp00QEHI6VEKSLWydAx7DBjWTma397HLGLwKoaOT7kHZ8ZSkei970LmdBCUxcwo2Mc4tQ==",
    "@deepseek-ai/dsh-client-ui-model-selection 0.1.5-rc.2 sha512-AlYjgwb40OOe5InaAFsQEu/KZr5+vYTItXMkFhkP5myzo5fOea0H0xdF29UV12BZ0SaRItacs8GE1kbDB9n5Ew==",
    "@deepseek-ai/dsh-client-ui-open-in-app 0.1.5-rc.2 sha512-UqLrgiNa48kTGJuMHo/f1er8ti3vcktkseUgFVW2WfWiV3ES1CQTyKF3+4gH3ub7RYFN8VteTQ5QKuOZe5WgPg==",
    "@deepseek-ai/dsh-client-ui-permission-presets 0.1.5-rc.2 sha512-2iDWBqr8bfhvzuxNW03JptdPtqQs6kHjh4GdBfgudUhFszaJZOBETx83+PfazYqV4daT4N0cE696l8tqqsS5Ow==",
    "@deepseek-ai/dsh-client-ui-plan 0.1.5-rc.2 sha512-3lbKHZA+9gPSXlMpEFA9xVbn7FOTCgP78Tf3buAjTmOnwSF3PRwBQoMeaANro2SoTn4ZuJZjJkpxqe3YDEOc/w==",
    "@deepseek-ai/dsh-client-ui-reference 0.1.5-rc.2 sha512-9zNSMWIhG0txbpwbUdHfkPTHYR44rBVl2aKPIcqr8JSUhI+8y8+QZwoykYTnbZXCKo2Ircdm5IXYbMXTbrGCGQ==",
    "@deepseek-ai/dsh-client-ui-renderer 0.1.5-rc.2 sha512-otUJ72f1UfL8b/UL+tMdSE+FHqJWErPmHs2AWbad9nHXghNoZjL2bxumOhI3b+Hb1eezhhM+KanlWzIulTIkvw==",
    "@deepseek-ai/dsh-client-ui-schedule 0.1.5-rc.2 sha512-DjBRHytIkcntXs93cpDSiDCiA6eEegc26ht+ZuE9sdCHNh3eu2GxLihMoKBiVFbepwIsdfuWO2mf40iFH/w82A==",
    "@deepseek-ai/dsh-client-ui-session 0.1.5-rc.2 sha512-EGCG4Ik95obEM1MA3ZVSsPuK7nknQyhfV/qgNg035jn6gtZJxAguuck9qBNuSOC0upx1xTbh+gSJ6bAmS4ugcQ==",
    "@deepseek-ai/dsh-client-ui-settings 0.1.5-rc.2 sha512-NsnZLRI2ZDzJJylx9KATuDwrTdJ0FSP93dU5SL5k2G2W4FLUspNvlAmk7bha5Miopfzmfd+h5/NvMdWe8PVVjQ==",
    "@deepseek-ai/dsh-client-ui-settings-general 0.1.5-rc.2 sha512-Y/UNa3iwrqLesUVSPLts1xZotenWyybnpU0/oABUt6rkvhDuxVz2w8HltnrVD3ytKzPBSrmlK2vP2YyUaK4z2g==",
    "@deepseek-ai/dsh-client-ui-settings-models 0.1.5-rc.2 sha512-L1vxfbVZEDmGlVuEaJcQb60E36l7BMZoICLJf+D7r8Gwcy8kFckGWKS0fbxsz+3Jvm2L0+lINJESzSMC9pMGNA==",
    "@deepseek-ai/dsh-client-ui-settings-plugin-inventory 0.1.5-rc.2 sha512-lpa6KZA/wqYRNi0zHQVrdUEbt4UGHKfughUSpew4sWvd4HymGGyuKF1ksVrioRUdRGAM+f8/VYpXd0ySk4wQ5A==",
    "@deepseek-ai/dsh-client-ui-settings-plugins 0.1.5-rc.2 sha512-MRErIAbG1GjrIDN/ULFEMrBA5rHUfg55+uY2P51UVmLdI5UC3B6I5QQYNo9pPcqHSVnT8EZFz7ZWz4Yi5DTAuQ==",
    "@deepseek-ai/dsh-client-ui-sidebar 0.1.5-rc.2 sha512-52PGlC4e9zkD6MQOjDo2TakxsWi4f5QigmdNsHluhDMBUfwMYl8ppWGD3uuHyAkK95yh0D1eKI5UPdQ5kikn+A==",
    "@deepseek-ai/dsh-client-ui-sidebar-documentpreview 0.1.5-rc.2 sha512-HeaqHwOyCg6/3ppFP7///G8C4vmVrhRBP8R0w31PLWIv9dcpnULKuFLFO7UemeBsi8K7uQ5zwN7V25DoT05X7w==",
    "@deepseek-ai/dsh-client-ui-sidebar-files 0.1.5-rc.2 sha512-ZJCpQNruk1Sm29wce30+ATjFkvnuhd55yB3wb+9WpeN+hW6jPmBnAmvUGbS4JsWqpimHLL4y8xYUd5AzAe7+sQ==",
    "@deepseek-ai/dsh-client-ui-sidebar-right 0.1.5-rc.2 sha512-EqKZ5JuyM+yCd6TVe7NwTmZf9y7dyhcLczpnu5rG87RuwVufEz1ARqO9TBXf32ApXCeYOJQGVeJjR/VsYzVMgA==",
    "@deepseek-ai/dsh-client-ui-skill 0.1.5-rc.2 sha512-aVBAA4cp79MRE98yVyFjosz8BAn4W5U2DFGF7BxmDYKNmFEp7Ae0THGNn9VXZg5wfmTH6fTAzQKw1THSv+EVWA==",
    "@deepseek-ai/dsh-client-ui-subagent 0.1.5-rc.2 sha512-/SWymLiSOzYswhikUo+VAuTc4dYkTecw+8vtex0H5IsCsRHlDaTnHgcxXnlvgeqFFifjanr4JzVhwzg9Ope2dA==",
    "@deepseek-ai/dsh-client-ui-theme 0.1.5-rc.2 sha512-qLR/6E/AzHUpTgKn8P0TiHiWg69qXYFfbpfHWM+HpQ66K7lyRQUXC+WKEJf2ZStCF4/enkx/rhMdXzRDlPVO3A==",
    "@deepseek-ai/dsh-client-ui-tool 0.1.5-rc.2 sha512-ncGDBvf7EpuDkq8KteGqQoUCcYkLWlGWNHPZScF8B92zdftYjlnsiQJZCkFbDiyRju3z7PAV3qdY0WO9hOTs3Q==",
    "@deepseek-ai/dsh-client-ui-trajectory 0.1.5-rc.2 sha512-QiCP1h1bgHG05vlJL5hde3hZPfTuSdwRrszgfY7Jy2NfkuIX6wqv8TI5zsZsEkS2/ha3Ebp3nhjDfkXsGZpgbQ==",
    "@deepseek-ai/dsh-client-ui-user-questions 0.1.5-rc.2 sha512-vJzqgAIAlGX7oth3a4qVq3pyyBRc4paL7ENOnt1kfmY+XoPTGF+/CUXwe/Lep5Xjo+95K2HPq7tMxeryKJj72g==",
    "@deepseek-ai/dsh-client-ui-workflow-run 0.1.5-rc.2 sha512-dFpAilctrJhwwZY/XXVvTy73v9UhAPqbuEMIK3sMBFtoAg+S/gCQsiVX/WV5/WCADWdjHLtBoEVAsZgrnImbHQ==",
    "@deepseek-ai/dsh-client-ui-workspace 0.1.5-rc.2 sha512-BRe/RDIJJblCECYLwVroy8h4+cXrf3eXSfLjJ6BdpnW2fI3CBJuEKlFaBPGtKAFTTain0rQ+dP6SscZNrXm5Gw==",
    "@deepseek-ai/dsh-cmdline 0.1.5-rc.2 sha512-sfjqgFojZRprFx0oKCI2J1GX2BZ+mgWq8bYX9CktFt3OGlLgmFE5JJ/nNSLKtCjRoXok7KXhsYFcH0dZGlkUsQ==",
    "@deepseek-ai/dsh-code-runtime 0.1.5-rc.2 sha512-qex3bvQNBkv1eapi8iTyNq+c52q1kxfdv5+8Nkswv4/kwrkvcDa4n4iiCbEgKS+K5U4DiVIprnyIv8Nx7il7Nw==",
    "@deepseek-ai/dsh-code-runtime-worker-thread 0.1.5-rc.2 sha512-eXrkDTWlc/ZH2A6S1bP1An2AgfQjmaMRmCKJU1mUzvENO7sLTo++GFR6L1CRLRkFxB8VGZo1TYFKtBxNKsdbaA==",
    "@deepseek-ai/dsh-command-compact 0.1.5-rc.2 sha512-TBvpsgCLLXvnyuVYV+eiAyL/xkl0dG2j1GUVC2mxwPov1jLiKgLqneZClJpo9gT192HAkGIs8a7wXCoID2ryag==",
    "@deepseek-ai/dsh-command-feedback 0.1.5-rc.2 sha512-7bXJ5IiSzaCbNUffWboAvMXze4N/GTID7O8loSsKBbeAo4CIS6lOWxyx8ZSrcQv/Gnd4onxizN6WqIMxW6i5Qw==",
    "@deepseek-ai/dsh-command-goal 0.1.5-rc.2 sha512-wNxZKEBC6bAHd92fzKJjyPnYsZJ7yq13p+5wLufNPZrty7y1nlJ8lm7SwxuPZyXYJ9+/5njYlMnZNNZFmitJCA==",
    "@deepseek-ai/dsh-commands 0.1.5-rc.2 sha512-ODc9h2Jig+Lo4XLdxqHpHjSXsBFeUCth6Y/rToor4KVRWQMedUARlq5otPyB1lYHyQh2DonNs2uf7g3mvag57A==",
    "@deepseek-ai/dsh-compaction 0.1.5-rc.2 sha512-aYCBBcmodNycKjaX9hWChxi2+YYMKlfiIfrNaL9QwtXs0BLvsv2/ttZqGxCebLGv6XdHWozNadC+opFpUCeZAA==",
    "@deepseek-ai/dsh-compaction-basic 0.1.5-rc.2 sha512-Tz6aEvPunsp+SXQruXTldxnZIMLPwXL3ZkWWiWnCM+83r1PglVBWABrxjaRkMaIEype/R16OjxodzR4k7GNA1A==",
    "@deepseek-ai/dsh-compaction-tool-result-pruner 0.1.5-rc.2 sha512-JsCZrbStt2EehHSLXU4CFKJW8idc0xefqCOh101bW4El26JShEKoJKTUwIMvRPq7AQh9RBWk25Pf9N1yAjKuDw==",
    "@deepseek-ai/dsh-cordis-client-runner 0.1.5-rc.2 sha512-RFzivtNX2Zx5wTR+9XnawzoxDViti44QoGEamhLP5+Jc0X8BMPVemYB9755UL3YRqgASNQ2x0IrJgBoc813jAg==",
    "@deepseek-ai/dsh-cordis-host-runner 0.1.5-rc.2 sha512-VUvOt3L7D7EJEMO2Cetlv0UMY1haIzI401W5kO1AKgOHktGr5SC+wFk83SHX7v2/CS0g1N3PbQ3D+RslutqPjg==",
    "@deepseek-ai/dsh-credentials 0.1.5-rc.2 sha512-TfX5MYLlyw0BFERj3dGxVfP9QGcKZUE5eXDSaY7ZfJbfQgK51VfxV5tP899OxIDRD3Bw94K5Rzx+dbUTSWJBtA==",
    "@deepseek-ai/dsh-credentials-local 0.1.5-rc.2 sha512-Xaz/giXBtfwWP/jd/o1QwKDCurGjZVfVXEfFZvXfzGJ1pdQiMFdt/1nfCnIkktHO2fHhtxEE4MBg10PBLtmp7A==",
    "@deepseek-ai/dsh-deepseek-llm-api-extensions 0.1.5-rc.2 sha512-zue4FWkj7Srg8mAwA6XNqT560NkraILiYtAwIcpT4mmt1Nlim+m55x7ACNkwA5Es0uCkEFAU1H6wHSnR7l2EfQ==",
    "@deepseek-ai/dsh-deque 0.1.5-rc.2 sha512-j1dINwK5XU8f+GPdzx22o2GvsAEhCvnLxqZfEqaLGYImbvdLkToVuU7M2nVcNSU+B7v+kl39lWf3E5u6tzG8RQ==",
    "@deepseek-ai/dsh-file-reference 0.1.5-rc.2 sha512-vJIHFFn3YmipM5X9SgPglnUO4jpvlCh3Yr3JQYITRtpBkL4hBbleayICP8cCmrfN0prstJ8dtIuXkUANu7decw==",
    "@deepseek-ai/dsh-file-reference-local 0.1.5-rc.2 sha512-WYTbwUD5TFWExRttqeNRPBk0EF6xfngevyQyNMRu3fgWYVrHHW9vtR7iGAs/mxAqCgTwZw4vAhCnw1HxaYYLqw==",
    "@deepseek-ai/dsh-fs 0.1.5-rc.2 sha512-6DHTquXPbpYdykGayqYaXSI9t668tDCoswH/bDezV+nwj4LxjrfY/smEtgp7nC4ubyoKf1hmjpNfUonGC9e7aA==",
    "@deepseek-ai/dsh-fs-local 0.1.5-rc.2 sha512-akUTz9D/N0ruSOzytZ8SZ330SdzG75fd7DzHsJ7KJzSif/QM0Y+RpOmGMnjlJzit4HDV2A4Etn80wSzstyp82A==",
    "@deepseek-ai/dsh-fs-observation-policy 0.1.5-rc.2 sha512-AntY5dfkTL8WugNGHkJxCEffaTFHqdXaYm7ZrerexnLiZQscDRRjf9zI00JwOmlFM/IrioCuLf5cERfhZN5GYw==",
    "@deepseek-ai/dsh-fs-sandbox 0.1.5-rc.2 sha512-eUxNsnM+TsjGw5OleOIcAhMnFhmQ4OAZoBYeiRMSeOMuCKWEjhxUGN8S8Hg1HxPaZVeIrUV7qFsNQzhehKj7wg==",
    "@deepseek-ai/dsh-goal 0.1.5-rc.2 sha512-atFJaoijwAz5yZ119f82I7jMx3tGCwXOz6qoY0Likb2c5DpumWZTJgs5L19OhKbhvEW+r2MAC4MYKaUxtrbb0Q==",
    "@deepseek-ai/dsh-goal-round-driver 0.1.5-rc.2 sha512-9uEhBTqIVxJklNkxCvDDD9upyEa6mIYlB5kfan37zixGDfKSgj83VSwXjJhVqN0b/2CLQDuPxtXgFuSdItoR0g==",
    "@deepseek-ai/dsh-headless 0.1.5-rc.2 sha512-EJ0QCWo+0WCTq2SFFRZpbMcGwkSh/uBRNJmE6nx9Eh1Qlt4y3I4G3k+p2mn276ujPbGfEadaI5KLciLOHsZQXQ==",
    "@deepseek-ai/dsh-home-paths 0.1.5-rc.2 sha512-Ek+DH9+MTiulfWDMKo5r335k4I9XIItk+jq8YaPihpK3klSeiKLW/SpKJhFodRnW1CwEyjIlt2XriycRvceIiw==",
    "@deepseek-ai/dsh-hook-protocol 0.1.5-rc.2 sha512-qJ0AlRhLsj31tro0vzubhXarigq8EDi3PwyOxxhoSxZUbGqOBWx+QLNEOC5E5tuob6DYA64Qbmgh7Cbz8vYgiA==",
    "@deepseek-ai/dsh-hooks-claude-code 0.1.5-rc.2 sha512-wdsl+A8tvAk66N5ozqyxmJ5t30AQQ5byBaKYL60Vx1jFoWXaD83piPcD1BAPZ+hVoWI1a9PCUDowFzAzq/PnwQ==",
    "@deepseek-ai/dsh-hooks-codex 0.1.5-rc.2 sha512-WVmZIWJJ04A7rpe80wcd4+O4MfJbBJdtiPwM99B2DASpm/Igyz3t0mNzNfbWe3zmznRKTHFz1imqhaPdceY1Ng==",
    "@deepseek-ai/dsh-host-directory-picker 0.1.5-rc.2 sha512-a0fVQTX0tM7Zl+1WWkCO3Pd+64CW1CwfOJrNxpTuz7EOp6wOtsYDRHlDyJUSKAOMJArS4xN2qkkt8n3eIkNkbQ==",
    "@deepseek-ai/dsh-host-directory-picker-auto 0.1.5-rc.2 sha512-R6nl8L/xAd1kmlCZc8xOl4jxp69HMmZ75vWdRswM3MXrM3GIgNiF7AKwK0JlNIKPc6M4vsC3sGMR4yR0XtdhrA==",
    "@deepseek-ai/dsh-host-directory-picker-browse 0.1.5-rc.2 sha512-8xnMfNt5XcO81qshrC/zzD2NYfLKYEQjVzcu7r+u/ymro7jLypRAp7DQFGuRZiQV4jIF9bdTTT2gcCYaXkWi5A==",
    "@deepseek-ai/dsh-host-directory-picker-native 0.1.5-rc.2 sha512-bBsGBR+tE/w9Dw9rM2Lh/4umjYtGnEmQ9YEss7vrKzo/KFuMfgNPkXHKb6A+AoCfSVd7L7PqkocoA5qVRQND2g==",
    "@deepseek-ai/dsh-host-frontend-static 0.1.5-rc.2 sha512-cTc0vTyejoYBE0XWOXw/nEDTcveAF9ucYEcaUVintvoo4b4WGG8fuTZSJX9EVkvXggAvxvFjSiYAQ8o2zP7qrA==",
    "@deepseek-ai/dsh-host-open-in-app 0.1.5-rc.2 sha512-8CBzeDQYMmz4IG7xp7a8mp0xeQ9v6FS9qcN2pOdfRBTXmMk8zHroBpiuW+26yEQmP4HsAi1BECgcWQ03eNoOCg==",
    "@deepseek-ai/dsh-host-plugin-inventory 0.1.5-rc.2 sha512-U7RTRLRs+O18ru6KzUy7LvMk/IgXnkWSalogg0abfA+QU020XZi+UpwZqM567RzC7MtpxijQk8bulGunu0lifw==",
    "@deepseek-ai/dsh-host-webserver 0.1.5-rc.2 sha512-lFgGm9wDrHiTBANzsdoWzdfPSjWYuDFwCoNQ4Uko57Fo5XASL2unfRHGm1xZ828rwEYuGwRvJHMOuoP/17VmlA==",
    "@deepseek-ai/dsh-http-proxy 0.1.5-rc.2 sha512-dRK8SMoxyY1F1ulzfnslI++9RDvXLGkPxahUwrnSQl8gTL9R69ZEh6P9dtcIVWMbstaYcq4CqWvo2/qUHSYRhA==",
    "@deepseek-ai/dsh-invariants 0.1.5-rc.2 sha512-oUxttB2yjAgkk47AiXOCxk9GwnfGvIEGsQfnMD7fukwIdD3KLJObU7+nz63tbFN80rjwMI8rg+p9QBQ10KU5+g==",
    "@deepseek-ai/dsh-jobs 0.1.5-rc.2 sha512-C3rBEuWhtDBlxMeKykFvSfBwjSPxkLsvKCFq8BrFdjDmZC1lI9GooMjPZkPxXVbogVrcOBaYVtJdOYJ4+rIpQg==",
    "@deepseek-ai/dsh-jobs-local 0.1.5-rc.2 sha512-PCDLSktONJ+If3QCcPlRskVLXIa8hG/l0pBIgKNllz4mb7CmYlJ5w1H9pWyrFZllMO0Wm94ZS9aP3SnUcJ8R1g==",
    "@deepseek-ai/dsh-launch-environment 0.1.5-rc.2 sha512-Cr35kPA3W7skJsPLJhUExLDAjVKCmvJgkog0m8EZ+XdK8RlBDsWiH2peRWFtFOjt8QOOeEkupPs1VDpWORIRyw==",
    "@deepseek-ai/dsh-llm 0.1.5-rc.2 sha512-Z7BVsBkK24SE4EItQeow8PHms/9GP0DSTi337vTAa/RY7tNg2Snz3INcXUj6CPZfvntQr1in9op9wLI+rfNsqA==",
    "@deepseek-ai/dsh-llm-deepseek 0.1.5-rc.2 sha512-qNRbLsE2ro+AfD7NgJwTukQLG82gIkbAISjDHgv0au7TocfPkNkUWSu20mTaBcuLGXzfSgBVn4oYx5N3ws6KVg==",
    "@deepseek-ai/dsh-llm-pi-ai 0.1.5-rc.2 sha512-56/TRc857HqapiAia0OyWsKtWPaPOfivx1OMzWYsexhTsIvxIOetrj5IMF+tW6geeSm38vfOffn18gOb97NgEg==",
    "@deepseek-ai/dsh-llm-retry 0.1.5-rc.2 sha512-6jyxD9tdzePZMdA4BdvrbdnqSkNYv2M1Nac84urkOcc3D8Jc+QYMnXrpeDB4fZRWg/l7FoKXtfUdn4Mj3kY5ow==",
    "@deepseek-ai/dsh-mcp-client 0.1.5-rc.2 sha512-L1uhXptzs63bne3fpe8By30xUu2KzC9bbo8t1RdasnRNxhevNJA/5/Z9ZenuMRh26XZh9THF+DVuudEH8PFIsQ==",
    "@deepseek-ai/dsh-message-feedback 0.1.5-rc.2 sha512-oNLajqBpv5bkebQrzByUpNTlQPcCoLoreMKzYN0s6STwtHpTzOqj9vSzwk964TfFAYutcmfQoCHNY7j9jmw6iw==",
    "@deepseek-ai/dsh-native-command 0.1.5-rc.2 sha512-FFkiS4Izm5VC1l0bHuw3yExNDmqW0/ypSLt+RtH24GG/W5ofpFvwjGclYDUzSDa0MP5uPzhTMN6qgM76BAA40g==",
    "@deepseek-ai/dsh-output-retention 0.1.5-rc.2 sha512-3eOx7EmMkttMcJfF/BT6e8A8b0SzWuLCxU+AdY9jBcU8ffZFspIPtMgJ86yNCMLdoZctbrmeSVwEdweMJWRhLA==",
    "@deepseek-ai/dsh-package-manifest 0.1.5-rc.2 sha512-PTieT+tXdV8TNK718JZCd9dpVZR52UMYu5i1nXPDPBpoq1fTP7lLSlgHvDEs6SpoJtdFPGNTe8+7KjELyUDDvw==",
    "@deepseek-ai/dsh-permission-presets 0.1.5-rc.2 sha512-WeQ3a+mcWfGjv6PLJGvMIgIZNBiR9mCA+wsjFZvxjtuPdu8p8uILxnaodlXhWpSDQ+8oimRO3eBxNVuJdtX1eQ==",
    "@deepseek-ai/dsh-persona 0.1.5-rc.2 sha512-VYAoj8tmSmwBr8SzDfjoG+t0v4Y/clu7BIWjZB5rnyNXLl5RNiY8kVTttnYRBQE+2jLLXlcR7PBJlYkfig3Kiw==",
    "@deepseek-ai/dsh-plan-mode 0.1.5-rc.2 sha512-veJCNJ8qL1vTG5F1GBpBa+lmtBVm+P3kgXpHBCJbYGmZKyrmHj3kbkiVgqxDdxTwnOoQiVco1OcBQUIO/ofGxA==",
    "@deepseek-ai/dsh-plugin-package-inventory-deepseek 0.1.5-rc.2 sha512-X/6mHIXFoFj6KSj0NBA2RC1xQaHtKnj1QS703AypNaq9+rxn3su3oFnZeWulLx0Tex4URfSmIo8SD1pkJkgUeg==",
    "@deepseek-ai/dsh-pwsh-local 0.1.5-rc.2 sha512-TwH/xwDRtgk3LyQraG4VoaIaItFSuorTTqyJ3nzuyLOsXq0I8nNYQr/wuaoRjkiDyzFTDeEVMCKq6zEEoLLz1Q==",
    "@deepseek-ai/dsh-pwsh-sandbox 0.1.5-rc.2 sha512-AYTAiy8wQwVoHO47e+hnG73GvHdmg+bKy3bFnrPDANCqnRZh9TJIbkxHxtJnfpBnNCcs2fhhCo6yHtGFYl7KHg==",
    "@deepseek-ai/dsh-repeat-tool-reminder 0.1.5-rc.2 sha512-ZDK8AJOP/hdpCKLfbKbfwgj/flICKEPdJbtdoYsTR5A+BGfq8RIy0BWux0IczDenzNSNa8FXq3gbnwQGTPAJ1Q==",
    "@deepseek-ai/dsh-sandbox 0.1.5-rc.2 sha512-OTOR6Jj9cey5YkhALG0TBwZ/Z3t986aczH6fLbzoIIegix+gwNaEBOqCWs+exVJ0Z2QuN/ItXzn+xHxW8Y0dcA==",
    "@deepseek-ai/dsh-sandbox-local 0.1.5-rc.2 sha512-qlbg0X7sR9BE90sfHMuOl0t29XG859jJK8xwf5x8yI9Du0zcmviTlQtvlsT7MgDC9OOw8TEDfRTgbm3VDSXCdg==",
    "@deepseek-ai/dsh-sandbox-policy 0.1.5-rc.2 sha512-QyQSCyLFxljkvmsVWJG0xUrYTiXr1DVOxHWURf7EHnbmvYZgg2B+nFcI58+IeUB2qbB35B1ClW5NQsKjOgDm2g==",
    "@deepseek-ai/dsh-sandbox-windows-acl 0.1.5-rc.2 sha512-GPig5OcIBSYtE5YwVzXqV+ICG68FMI1Fo+ZgyH4Jc73Qlze6A4uZqtCyH/xsmiZfzOZiOLyFB9+gc8lu4tZv3g==",
    "@deepseek-ai/dsh-schedule 0.1.5-rc.2 sha512-SjVt6miSlSX1WQdAyvLf0KyQULIds+bLihr3nIvUAhrVSmoRDzbu2jNrNA5jGCo923+dgjGfLn6iUT6aNDJO3w==",
    "@deepseek-ai/dsh-scope 0.1.5-rc.2 sha512-JwItISje52iVIjlGMNayRVMZrvZU/0i3Gr0s+coND5nBp5lS+pOj9Zf1RQzA5cWjHYCWKmDuQc+m6zGlwmtAeQ==",
    "@deepseek-ai/dsh-sdk-app 0.1.5-rc.2 sha512-WamfPr7CctpoICsT32UpnLlAZavaEno2zwa3icqqpGdSHY0H+BL8yx/4mmRROIX1/ovMyxTcC5urjptDKy34KQ==",
    "@deepseek-ai/dsh-sdk-jsonrpc-server 0.1.5-rc.2 sha512-TRVkUngQPKlpHvqS5AZi63zwa+5eG0zp1NwZXzJjdiYhjV2LPICiWVM6EuSbmIRB6tUZMB1MXf4ZSj2OFQWn/Q==",
    "@deepseek-ai/dsh-sdk-minimal 0.1.5-rc.2 sha512-mWdnFnz+MGU1+xgg4GExF+OkkYxEdaNQdAPFq30m0oP67cWQV0Om+pNBHDeGeguf8MVGLNLJB+iRP3/63LQV/Q==",
    "@deepseek-ai/dsh-sdk-protocol 0.1.5-rc.2 sha512-ewWYU+2tkcm5rTQtM5F6pYP5S6ox/7j/Gc/xCpXBtVOfUNchI8IUYbVNuea/lGX6XYGCBxpdG1jxV51W1YyWkg==",
    "@deepseek-ai/dsh-session 0.1.5-rc.2 sha512-y+klWiGAWR4m4cc4ylurA0cW63673B4N8cr2ANMimweDZAfxL4XVBC7WiD/5DT2DtIhYmVZhz/niyS/WbniUTA==",
    "@deepseek-ai/dsh-session-checkpoint-policy 0.1.5-rc.2 sha512-RDaGTq4c49nFfVNq0AiL71z07OhtxoxfXESWppyegxotcdZTpdJTk4wIUto56SuaXi+kNCAP2PEjHC+VM0Z30A==",
    "@deepseek-ai/dsh-session-format 0.1.5-rc.2 sha512-Bih+d4wQ8ea+sZ1+Hws+ZBG3XpExrSQAtNMJRueFVzfTALhbnzpS5uLYw9qDapdfYYyHURlotQPRImBEpIVGFQ==",
    "@deepseek-ai/dsh-session-format-catalog 0.1.5-rc.2 sha512-UKKVXS551VuTA5hH0cz9s9GjBb8HKdQcM+8Xa4uiviavTy4SOkVO3zmo2Wx43jzgdmChtNiUhc+w9RDOZLc7gQ==",
    "@deepseek-ai/dsh-session-format-v0-to-v1 0.1.5-rc.2 sha512-0ff7Rl5JsUHGv223XMRFv0adfy7nV2mUQSRf3N1MZviblnp2c65dArq+ieRGISw6JQL4FAz4LSt6A6zV/D6o0A==",
    "@deepseek-ai/dsh-session-format-v1-to-v2 0.1.5-rc.2 sha512-VI15vcRCFfzmPpUg+BXSqgB9Kd99sVVXbfxGXo/gYRgjCY3JqClRh1CYhj1BDUvL91l7LsLv9Oa63kYXXbCaIw==",
    "@deepseek-ai/dsh-session-format-v2-to-v3 0.1.5-rc.2 sha512-BO9N4O3HhBrUhyO1ucuk1xa5r1+wYCUPnMTryIrasVnOHI3ndb9ITnPhTEc2m6OXJa3c+wuEWrxc7DJHM1Cx6g==",
    "@deepseek-ai/dsh-session-log-deepseek 0.1.5-rc.2 sha512-d/PJ5/vsJcbv6ZOOD3pBXH58rBWhhSmWHlfLqH7wDq2wA25e1va290OVKv1wwR05/mZBBQZZXxc7qwT4xnsiyg==",
    "@deepseek-ai/dsh-session-log-export 0.1.5-rc.2 sha512-eoJWiv/oZQA2S61PpDpkU6kovoN5rQ1ZvOmp3HfKLKqfWzi6La1YP5VyiXi/g0qFjZ3ttZC0N+ckmWjNcGUjzw==",
    "@deepseek-ai/dsh-session-persistence 0.1.5-rc.2 sha512-0nDeM+H+3YR0CH/IVlhjNL9bDx2G4QbliaLekVxY0jdZHM7RqdN/fWPKjeCKbXcRq6qdn9QcmvXU3uWZdErlIw==",
    "@deepseek-ai/dsh-session-persistence-jsonl 0.1.5-rc.2 sha512-nkIXdb5oOV6a05Of+3Wywe3gEB77uQOH1U0M+/SRVziSLX4JkE+us7dbWczOOloZjuUsEV7Il8kZ3R9JLsGqjA==",
    "@deepseek-ai/dsh-session-projection 0.1.5-rc.2 sha512-WMXGBdbxD1FO1Eu7Qs2FKDXq/kl+z+5b7zGLHdra66eEnShSnQmdF9bXSFQu1sJwba2yF7ueMcBJpWy76gtgQQ==",
    "@deepseek-ai/dsh-session-projection-cache 0.1.5-rc.2 sha512-dS4GGlgZBBHv0U7C4+pQssApxao8Jbt3tMSGQeRU3xpmR5wP8PucR20wTiFThNrsUjdIs2Q/2IKw5bcnkaHwCw==",
    "@deepseek-ai/dsh-session-query 0.1.5-rc.2 sha512-Ntv/yYwhRhhXO3UkRKc1RIIk4RMLVR6jR8mdphRILEZoxPGjZxMJR95RuZg8TuLERbmGH8AkPAEKD6HWHqWh1g==",
    "@deepseek-ai/dsh-session-query-sqlite 0.1.5-rc.2 sha512-7xqyRvcaXfPzK6T96QFTdfEVbYIOz69qcghBoDMwSo66MQ+3DSdH3yTPlvrf3a8Nd9Jek375Upz1h12//FB5Ew==",
    "@deepseek-ai/dsh-session-reference 0.1.5-rc.2 sha512-SpkwyQp0o28bxc01xwid+s+nVCo1Xs8OGsOqyuPRky4WhZtbuI3HN0q/M7+8EcH+kN/SbUzrDD45Q4FAiiHzbQ==",
    "@deepseek-ai/dsh-session-stats 0.1.5-rc.2 sha512-p0C+g31Xcq5w4FWxS+V2hMfLtb97sHpaHNTaAT1UeVTqpSVHt21qC0PnqfFfOe/OsVkMkWDKWDbLb1cA9SnQ5Q==",
    "@deepseek-ai/dsh-session-telemetry 0.1.5-rc.2 sha512-HCTlVFiyWHNqewD1wLow3kf6dbDBU8GqjHlLO6Es9viYW4GCAALO8FXxdqbWMSLOkwMNMM5xqWrXedjfSmRehA==",
    "@deepseek-ai/dsh-session-telemetry-otel 0.1.5-rc.2 sha512-T2gH6xJ9NNTnEjQG3od1vVIieT3htMShlqTTfOzxIR8YoGf/92KpCq6xVtYx3URm6aJysRpIbtRmoYzVq8Ln7w==",
    "@deepseek-ai/dsh-session-title 0.1.5-rc.2 sha512-lQ1PzIdySxANbhVM9blxCSlqR0iF2GVCiE9R1Qm0VJJue/G9ZUasz+jGQJeAVZi7PHjVWCALljtqpcnaOZq+6g==",
    "@deepseek-ai/dsh-session-title-first-prompt-llm 0.1.5-rc.2 sha512-D+eMbKUadK/9QTkk00TItqvE0ealZek8Tc17HJgdGP7Ya0pSYL3hI03q0U6m7SXJQAAFjENMof3xHS+HmyPhdQ==",
    "@deepseek-ai/dsh-session-title-llm 0.1.5-rc.2 sha512-IO80OEJ88AUX6S3c5hgjmrrOAgqvTwg9S/4o40H2btAFPCefj+ioHg9qa6KyMZaaZBe8dGjjdngpsaOvC/QPIQ==",
    "@deepseek-ai/dsh-session-turn-outline 0.1.5-rc.2 sha512-z3kVLSv/dqwtOM4ii8FfzmmLuEvp9NJORMvm2MQPkYzBxLrQvfxIRPm5Q6uqmTowFrEcYWu1bQCdg6EdnZJDgg==",
    "@deepseek-ai/dsh-settings 0.1.5-rc.2 sha512-LI2Y6GkEs9ALMW+7S9jHPeEZDXHtG5X6cix1HdJ1rRxTEO5427QlYhMiz45rk7hqZ8ca2O4XFMW8IWXFZQLxmw==",
    "@deepseek-ai/dsh-settings-file 0.1.5-rc.2 sha512-5bdcCyDLMQyo4sm1g8l4mNjIrsyfn1mu3+9tvGYrDWIw0f1n1YEIWuHp2reyoZpRCMAco9S5N4mDoKF2mc9T/Q==",
    "@deepseek-ai/dsh-shell 0.1.5-rc.2 sha512-BfmNN6X0NHN2XleW0fCbtFXedXEppeDQ2oa3WsOTuhvnBftl6QWMWWMM59weE4xXker5fzqOnoJdeBR8D9qR9Q==",
    "@deepseek-ai/dsh-shell-env 0.1.5-rc.2 sha512-fFSrfhxfvVYfDxsOuV0cjAeC/PWweW+86uOJWT+2paHXOSgM6MSDe3eTd5DHRDYnjb5XutTYu32pR49cVBHMug==",
    "@deepseek-ai/dsh-skill 0.1.5-rc.2 sha512-Z1mouW3vTzmYk9OAGUp7S9oOB4k9+qr8BpD7KPAR0nAPX1XOxyKA7zQA/6P0tMS69/aS7UsyLZNz9+RG8ZZ8Wg==",
    "@deepseek-ai/dsh-skill-badge 0.1.5-rc.2 sha512-piOjH/WNjv0If4P3IQLuAsOh0WWbmWBv/06IXd1x7emet/d62nnnAp2gl75pqpYDR6qyavvRLNCqhqBJIudKTw==",
    "@deepseek-ai/dsh-skill-filesystem 0.1.5-rc.2 sha512-lEZ9KojaIc24vKCa5mp8LBHUDoXpNRSR/vlMsWONc9Tc1wwfn9kEotIjj0HD4PjIRcnc+Lma1nk4G/oNED8A9Q==",
    "@deepseek-ai/dsh-spill 0.1.5-rc.2 sha512-djiz5HH1xtuxiBAPT6ODlwEPUQz/SR/Q6SCUiEud7x90nmTsfLLxGUQmNETdiRC3juBLZeoDV1Xgmjf54P4y+A==",
    "@deepseek-ai/dsh-spill-local 0.1.5-rc.2 sha512-HrTEyP/KJh9YudGYmXBZvu54x022JWIlVX96d2UkHKICSB1NJy3AUIO0zyYCOk1PX+4iC77oD6zdOAyyxX21QA==",
    "@deepseek-ai/dsh-spill-policy 0.1.5-rc.2 sha512-XDtIEF7eeErjf0HS6ASkquXZIorv96+A3kuSZs9ZoWqI6eHVxUBrTebHhNc59P9Cp6LeumLUhPIb+SEP3ITgYA==",
    "@deepseek-ai/dsh-storage 0.1.5-rc.2 sha512-mYt6+JSRxA5fSUcYnI3JFvYw9/TKUyykz5JWoZZ3MxWudItfhw+CV6T3kn3/bpTbWIQnGesSeY+0pJJVz0xBAA==",
    "@deepseek-ai/dsh-storage-domain 0.1.5-rc.2 sha512-NZ9U14kqcCnF4F+ynrTlBLb+S9FEOwSLkfdjDgdfrFdHh4c2WsRKWxyeYbL8AME1rB+XxLp4YQBx8LQ0Kiy62A==",
    "@deepseek-ai/dsh-storage-json 0.1.5-rc.2 sha512-vIdKeI6BUQpqneD/+oTZ2YNotNSGXZX0VbFbhdvgMSgamOrExIjl4cRsEJttcNr8zSvB8PjmuX2wPFvkMoeAlQ==",
    "@deepseek-ai/dsh-subagent 0.1.5-rc.2 sha512-f2wbfB1M2P8RpcApp5eDpDQR4V5ElvEkeK6su7tor1U06uJadJ+JuIVXP3MrCnT7yjg/IM2tTacYFMOg62BOfg==",
    "@deepseek-ai/dsh-subagent-fork-in-process 0.1.5-rc.2 sha512-9VHI9pA1FX7mEOUx8Iw7Q6Sqk76rJGsOCQ0jwsaE+7YvQxV8YhOFe9gBzkmk+ChRNag5ToBVLFuUFROpboo+Xw==",
    "@deepseek-ai/dsh-subagent-in-process-driver 0.1.5-rc.2 sha512-IBwRQHVX4hi3YO2vUYmTtDxKM2B0lIwAuiAG3hsvljmaF66z2snyLYezKszLZeTPphr2QHgt2CJNxNP1LlhUJg==",
    "@deepseek-ai/dsh-subagent-spawn-in-process 0.1.5-rc.2 sha512-/FJxY+dBc8AVWFmSYcQPWapuTd3ATGQpAb9Z2cQEZjFRXy8ySuZnh2MrnAa/r00FX6Lz0G8BfvUurr4K+uKHSw==",
    "@deepseek-ai/dsh-subprocess 0.1.5-rc.2 sha512-Bbxg5/dlbK09qNu4/BVuicUAYnC+b/YGhtkUBsxa9/+KRfvloeJcJ9Ug6dKAeJeluzEXnFIh08+MNweO7hn7ig==",
    "@deepseek-ai/dsh-subprocess-local 0.1.5-rc.2 sha512-DmG3lcQlAh8bTKfeyYM44cfRyJktbLL8drXrVfOvGrFk0zSs5eTSLcfhIKLT3jFJ5CKFriHYZPLdRT6Alvyy4w==",
    "@deepseek-ai/dsh-system-prompt 0.1.5-rc.2 sha512-VtmZVKqBMJ7kzskHu0jY+Jth7jSuKMG8QB3MBPzJe9M0LL4YPMWsGKt9gGky9RXolk/ugAy4YZEv1gUqouVofA==",
    "@deepseek-ai/dsh-terminal 0.1.5-rc.2 sha512-ALDjWfRMWvRmk/HfM7z5sXfQ1DSarJcjkzL8bT1KV01WlJB1KHVbXKlLI29tQH1Zu92uXk9g0JaFQKNycqT9yQ==",
    "@deepseek-ai/dsh-terminal-bash 0.1.5-rc.2 sha512-UJVx058VgMNjwptwlGPBsMCmAs/ljgQwW9RNDZCpxvKxtc6D4RqEm675/f4uMPnGEKhPCJFKCGZy3ltqHvUPdg==",
    "@deepseek-ai/dsh-time-context 0.1.5-rc.2 sha512-XClt8a3ijmELHrMuPG8F+P7uLbi+ZBcjulTdeMq7UB4yRSmTHNcQ1rJLyPjPajSDsR6F81x6z6xT842PC4u1kQ==",
    "@deepseek-ai/dsh-timeout 0.1.5-rc.2 sha512-FgfaAw8Zt5X4Y6Ljh833B8Hy7h96FeR+yjNKI3iSyZSqREqOBgvDGXFfQlSu6V3Buvw/cbI/CJUQJvfRBKXOiA==",
    "@deepseek-ai/dsh-tmux-context 0.1.5-rc.2 sha512-gocDAjUDx80DkbS92/WZJqx+NJkTfuGnSBglDXdUZRFj5O38Ggl9ywSKxTISk/Yhj3SbjRab9UfNjBZ1M8Nlxg==",
    "@deepseek-ai/dsh-token-meter 0.1.5-rc.2 sha512-GS13T/USWa8pFBsahLGQCTQ31ktIa5iBqUYyFFrDPKhvUZFk7dTgltqIo9BxpsU5op0Lwj1LFyCnhNnuYbzvaw==",
    "@deepseek-ai/dsh-tool-ask-user 0.1.5-rc.2 sha512-M+ha4uty4SM/nEfkOeIMw7vxj3Ez6lZin6oQqgw5hWXNrmf5C9SfF8VH6KZYuPhsmxPiyTUr5SyULxgn2eTUGA==",
    "@deepseek-ai/dsh-tool-bash 0.1.5-rc.2 sha512-f4LmiZkZSfJfvBcEzV4q5J83VL79l/+ncLkwnJyHzsjvJbW5qFxzCy4p5FXfY/CflG0taG14/p42UJzb9qEhrQ==",
    "@deepseek-ai/dsh-tool-bash-persistent 0.1.5-rc.2 sha512-OCfqQijK/eo+ILLatQntWnKJ7hFwUUOLpxdbkgmN8zjvD3o3LPRnnt9IerCid4YjGwc69jSTBX0Zk2jW2n/rhQ==",
    "@deepseek-ai/dsh-tool-call-timeout-policy 0.1.5-rc.2 sha512-B8lBJ6FHT3eqfQupHANU6JGwSvqp4EGYSGgOiNh1EVwXNn5MeX1AztQO+sNqWl69nof9d7/7vxXmTeS7rmIFXw==",
    "@deepseek-ai/dsh-tool-cordis 0.1.5-rc.2 sha512-bm1p2cRmXvO5b4oj2YHvF6OFh8EcHJQREcD52SEkSWtdkeakHbwMEOeCLfSWSNMUiNeOa7tMutUQrepdCS37EA==",
    "@deepseek-ai/dsh-tool-fs 0.1.5-rc.2 sha512-/3AUx+V1UxVfl24fm10hwRbJnHwpBkRDHniOeocGknMvASheRiKnHpnnj9EszFRLYSpe2PRFSMuyuhXgYyd3MQ==",
    "@deepseek-ai/dsh-tool-fs-search 0.1.5-rc.2 sha512-B4UrzbrigYiD8IKweuVzOHHLrhKfBPWTxscPDy0WmaYJ+aYQ/qaix9RgxnmJRoF8aVo4lw/Fzlz6hq30ahwUIw==",
    "@deepseek-ai/dsh-tool-goal 0.1.5-rc.2 sha512-nZ0NkUxvtAsrPAd9NMXt+4kS7WPn8xIvXCwVunKfBwbrnGnFCNmIsTQoHF44J6q9VbMeSCN2eyj07w6ej3Hf+g==",
    "@deepseek-ai/dsh-tool-jobs 0.1.5-rc.2 sha512-v4y56H3FsVBF2jUJLGLHdPeKaKJxLT4Zx/oii45UmJv5t8Bp3uCTVUoq/mfSFVuFY66q6rHJwgIOKWc4ZO3ySw==",
    "@deepseek-ai/dsh-tool-present 0.1.5-rc.2 sha512-WWxjEgh/nPsATfofNc0Qi+Eshy9LPNAYM+UWWisuKcT6smtv7+pR8Bk79RJu5BiFog8Nq17mwEh5XnFlDIg8FQ==",
    "@deepseek-ai/dsh-tool-pwsh 0.1.5-rc.2 sha512-rnHM3Jqlr7rthwfPYysXPq6zH+K8JTDqbE+xzjkbo1WIBKBZeSGr8kPjqdCZa7eUtWhmV/RFBP1qmBgDjEZaWg==",
    "@deepseek-ai/dsh-tool-pwsh-persistent 0.1.5-rc.2 sha512-NToI2tXkbAZS6Cxjnn7VjgCF7o68vMquvmpVh6RjTXAsawNY0TMmAVo67bREDw07+1j/bTq8EPywrJSmBbS2Fw==",
    "@deepseek-ai/dsh-tool-ralph 0.1.5-rc.2 sha512-qP7KoVUBPLvvPSiJBXJtQt6c5ChnJRs5ZJnW/VU2DbBjYn4lkN6BiSfpRsSfjRGKO9fmldn2BYR0OnSLXi57xw==",
    "@deepseek-ai/dsh-tool-skill 0.1.5-rc.2 sha512-J0AshOq06tVWFbESolUP+9TEYXymASMULsWsqTBAgaIjzbZjhJ3x2CPMP54gM8uROh/y0qGBhVUErJeamP9AhQ==",
    "@deepseek-ai/dsh-tool-str-replace-editor 0.1.5-rc.2 sha512-SqxB7jgmAURwttFMQ9CmM4l1n5pdMA1ofPOd0Fp2OqYRdo8C0NCO0YN06PtX3CvmjI6vjDUT3grXqETzhmN3gg==",
    "@deepseek-ai/dsh-tool-subagent 0.1.5-rc.2 sha512-6H7iL1UMcPrQTZrmp/olatipQzg6z5b3xqqPpW26qhgX9tDttJJGAspObePM8Zj02EO9ibz7QLg/VIehPuu+sg==",
    "@deepseek-ai/dsh-tool-subagent-control 0.1.5-rc.2 sha512-kuSx9UGDBUSrffTPnNetKj7mUmH1+24BQT+589vaN5epO8GKZ1STzIR3IINn+kMP0t9MAGIhEsa39b30OQu9nA==",
    "@deepseek-ai/dsh-tool-todo 0.1.5-rc.2 sha512-TmyoAthal1pOCQ/XwlohcxDXdI5pq5GlZiQpLaI7e+rAruqneBPz/0Zjv6/rlOMrseoENMx+H0sazwoL9LufTQ==",
    "@deepseek-ai/dsh-tool-web 0.1.5-rc.2 sha512-FPHiDBHjyO/2ayVcUL8tmcBqc+Fi8Y0zcQuzsFw6/K8pOJvP/pBSN8mXi4tBVmmqXg25ZXVo1vDDo4T1mBIlFw==",
    "@deepseek-ai/dsh-tool-workflow 0.1.5-rc.2 sha512-co/4mEKOC5MYXM/x2sdTiIwPfUtK3Bhst7HMwmgcz3PNZG4DEkaqLlzsZyisLn4iQvKChtvdyS5fWwNeC8ws1w==",
    "@deepseek-ai/dsh-tools 0.1.5-rc.2 sha512-k2yZuJJtszaU9lzr2aBtdeFMINrkdlk4ellbtrMokA2oySVqJmAM8dv+u9RtzduD3aRRqyr2i2hWrTACz0qOrA==",
    "@deepseek-ai/dsh-typert-loader 0.1.5-rc.2 sha512-5kNw5jLJ1PBGqL9Ma/ym+Uz8pbFhMe2Ci0QypPtE71ViUC8K0usgypqmn3+2vXtBk5E5mI/n3UGu9gyTPdHIYA==",
    "@deepseek-ai/dsh-typert-protocol 0.1.5-rc.2 sha512-zP8J20rBXa1AFjKL2i83JBeQcePZx8yal3pNojj4t7CCokMcoFMUbr50woa6fY1tRJ7qHxgypO3rH8Q6YWgmiA==",
    "@deepseek-ai/dsh-typert-registry 0.1.5-rc.2 sha512-GBHGD8T4goY8kKA0U1jD2YRPAgkESdVuSNXn7KpYeb3SLtVntt2ND+FMSwNqcMN/zuBm9Yvo5x/nuK+yxFqvBw==",
    "@deepseek-ai/dsh-user-approval 0.1.5-rc.2 sha512-8UpMEnEyMo6mYEVELBo0DC2iG7aJJfFMTNkU+DKD3c5Ut7ySRDzt1iRr1qni/CzqlTIpQBAkGaG77qk4q/v1bg==",
    "@deepseek-ai/dsh-user-questions 0.1.5-rc.2 sha512-0cRRE9nc9pMxxyC8G8C3859EtPyZR/27FRgRMRe/wDWL30S6JoWWgaw7IFN9FogvaoEp62XV7wQ6ub/eoTqggg==",
    "@deepseek-ai/dsh-util-crypto 0.1.5-rc.2 sha512-JR0aJEUL35RE8FVT89wMZMbPwMrbkjNcy+gE9pq70d/m2Zxw3a0iNer+BAxPAKYPbmy0xDDXReTwXiecKVy4sQ==",
    "@deepseek-ai/dsh-util-time 0.1.5-rc.2 sha512-nWJrNCaEBANFUX+Rp2cE1+hA2CGCmktXOo2Nsh524aV2VaLumFFS501IsmI9ghUmUE77qZxvayGGO7w3LLIECQ==",
    "@deepseek-ai/dsh-util-values 0.1.5-rc.2 sha512-Cr0TkM6dFAD+Iy4sNXH/afJVMHwJXoOztivj+RflkYoACsOq+Ix0vdHn36hwW7mayU8OK3ZKIZUQ6BPo1fHvUg==",
    "@deepseek-ai/dsh-util-workspace-path 0.1.5-rc.2 sha512-RCBz+6BpdPDNsRk2ukIdIIuLdf6u+cS8sLiViFg/8/x/kxc+LEXRKJMy2xv+YA9hYOGFK109rjUFgx9JVtZf2w==",
    "@deepseek-ai/dsh-web 0.1.5-rc.2 sha512-3qt/Fh+uCghOy2wZPjwQ6xIMU3t1NW4D5yRvTXOyvadwaDwosqXOzCteYUhnuWzWN66adrFLeRrymhSDS4PMeg==",
    "@deepseek-ai/dsh-web-app 0.1.5-rc.2 sha512-Ng7YVDt9txh2BlLmu6B+V677c1bihbh/rq3EOtZK4b0JWtgdNgYb1KPIED8+aK2i+FghrCrIy6X60AyJ45eOvw==",
    "@deepseek-ai/dsh-web-fetch-http 0.1.5-rc.2 sha512-K3us2mU0L1sgnW4FFtDI6UYcL+JZvIbdRLEAO+8oq9lHAuwKuCcb4RPPuyXMI35ue97PkfBgt6A6Sl0utiIF3A==",
    "@deepseek-ai/dsh-web-frontend 0.1.5-rc.2 sha512-o0+dEbtEzchPnNFkKQ4o3/OekEi+PG8CTEwp1eWWg15TRiZbG2OeUKMsQjPpAVCMDnpEwMQ9skdKcwKj4TXAzQ==",
    "@deepseek-ai/dsh-web-search-deepseek 0.1.5-rc.2 sha512-0r9KqkjnOd7VLwWEAYHOEGLctRwhGLOJ9xnnGuL3nd3uFcrA5Wb1MYbJhGKxx+9XRpTJZyi7Aiex9QRAr8c0Ag==",
    "@deepseek-ai/dsh-webhook 0.1.5-rc.2 sha512-Tv4aR5JYxq9ODe9lYoFaQWjFUf644tPaSVSgfBMPrynN9X94rcY7HAkYxt3qLggvK/cGrtNOFwyqyOIDTjqyJQ==",
    "@deepseek-ai/dsh-webhook-github 0.1.5-rc.2 sha512-esocwn22CYbOAo7WzwWl/E3jsHKOAXFS5e1dnW9b5VNPujnbHdvhXyyNQzUW0nNnxt1QTsIARyhqpDKWuanCcA==",
    "@deepseek-ai/dsh-win32-process 0.1.5-rc.2 sha512-KWF9pldnznDE1XiD7vjxg+P2cDQHOCJtz8ZQ/P0bLorDog/peQwqxYNgc/nrHxEZ1vWl/vbdd8Nvoen6FJ/Orw==",
    "@deepseek-ai/dsh-workflow 0.1.5-rc.2 sha512-eGOBiC4RA89zZTTG6hBpTchau55lZ67P/dyqg4kJq8K1FwummTg3xed4Ecy9KQFaSX8rWQP9RMJnaWg1fBgXTA==",
    "@deepseek-ai/dsh-workflow-worker-thread 0.1.5-rc.2 sha512-y8twj+FNYoxaaLWYkr/tQauCHymlY3AUgS5lHEi4tZW1PgYD2mEz7VPHu1nUWX7W6m78gCQvg/6KcDCLrTgvcA==",
    "@deepseek-ai/dsh-workspace 0.1.5-rc.2 sha512-J2RIjHk2RNTtQNZ1Dpy1ItdhOmXDsWXLvMNv/MMdvv3NzCjKMYQ8le18lpzboqAZY3I2cOfX0cxu6wHIC96SIg==",
    "@deepseek-ai/node-addon-system 0.1.2 sha512-EFn8K+mXIXiw6s3rxX+cgp2hZTEAQdVIsURhrXnBxycSXvtQ1EQpsSVzoNKtKAQI0I0VqkEcVEjHzS/PeaGsvw==",
    "@deepseek-ai/node-addon-system-linux-x64 0.1.2 sha512-S2aPVHvYCpNCppCFyNlooMYuTB7ucK5lvD9oXQQ42v5Z2s5AoaiCdjz9r2l+ED2rI5oS1x0cUZmKjJH2dxV0pg==",
    "@deepseek-ai/schemastery 3.18.1 sha512-Qn0FCSwCQnpnj6SB31I6i2sIKgKWnkbJM8O0EU91Gv2UsYVvtZTl6IA0sCwk2e2MZf5S8w5hpq9QkeVvK9qwxg==",
    "@deepseek-ai/schemastery 3.18.2 sha512-njDtZsznjYxok7KLLlHOPyuv2efdWVbSflAHgztSfbMsg+CVraEoRe2DjOCgClYv3ZCSm7WXoaUkbB/+RY7tWQ==",
    "@earendil-works/pi-ai 0.85.1 sha512-+VgVIJDkDO2efYJKEEqvPTH4zmnIaXdAppGbO+vKFA9qy5PdhFiAenuFAkU+oiCSfOC4dMHDyrjdQeL4ZoC5CQ==",
    "@earendil-works/pi-telemetry 0.85.1 sha512-Bg/YN6kA7Swja/NQxka8xFdecb4E/auIEGF2G5A25EaQXhRnPj300/7/KpgsDDMYUzHTDAv4RyUxaQPJKW81Rw==",
    "@emnapi/runtime 1.11.3 sha512-Xz4Tpyki7XyrpbUK1jR1AhdAdaXyhhY4lZ3neLodmhpuWfy2PAQN5B46sAiU4liOXGLkHypn/qU+jvfWSCYYLA==",
    "@google/genai 1.52.0 sha512-gwSvbpiN/17O9TbsqSsE/OzZcpv5Fo4RQjdngGgogtuB9RsyJ8ZHhX5KjHj1bp5N9snN2eK8LDGXSaWW2hof8Q==",
    "@hono/node-server 2.1.1 sha512-ELuehkj5VCBdgEw9zs+ivkKwyzzUCSQuE96YmiPvn1ECBoZCczbFXJLeEGMTYjphP6gydh4pHMqEYPVMYUVgQg==",
    "@img/colour 1.1.0 sha512-Td76q7j57o/tLVdgS746cYARfSyxk8iEfRxewL9h4OMzYhbW4TAcppl0mT4eyqXddh6L/jwoM75mo7ixa/pCeQ==",
    "@img/sharp-libvips-linux-x64 1.3.3 sha512-4vKmvAst9nrowcqquKFAyZJUDolUaIp8uRiN0mWFguJ1IplC9/pitXtlnnlU4aa/eJw3J7i67V+pwUL+wZGdsA==",
    "@img/sharp-linux-x64 0.35.4 sha512-9qvvEAuk8k89TfWUoX2htWjbAMX8p+NxCppjpcg5k6xMsjhBQPTsoIh36h9Qde4WRuGpJeYnOjdosDn/cnv+OA==",
    "@img/sharp-wasm32 0.35.4 sha512-zQnl4Kwp7Q6NHsENtU2T/00Zi+w3AQNwz3+UaTyVBy2FpXrzXzGjndpK61onhZjRtRpQXxCTeqw19bVyXOh7jA==",
    "@joplin/turndown-plugin-gfm 1.0.68 sha512-m8DfAQNC/V7g0j5H6Jv60WhekBBjodD+zTPGrU+g2m+ux/z8ZX07KQIl6kaAmbKwfvsQhC04op52g63I78sVgg==",
    "@koromix/koffi-linux-x64 3.3.1 sha512-uU5cJNe145TvITqrUWvBmZO4WLNObdZ2Gnm8eNk/OdrhRFG9LB44RUv07aQwPbdo1438j+Cr+tFNnpjTBadZFQ==",
    "@mixmark-io/domino 2.2.0 sha512-Y28PR25bHXUg88kCV7nivXrP2Nj2RueZ3/l/jdx6J9f8J4nsEGcgX0Qe6lt7Pa+J79+kPiJU3LguR6O/6zrLOw==",
    "@modelcontextprotocol/sdk 1.30.0 sha512-xKd8OIzlqNzcqcNumGAa6g+PW2kjD5vrpcKOnfldAUPP3j7lnqMPwlTXQm8gF+UwH72z0lqaRbjr9hqGz0eITA==",
    "@octokit/openapi-types 29.0.1 sha512-9qWOMFNxxLokERcms42rU0PTLqQmVs7g5E41TI4mCOxmpFayD1rfC7XxOL55cG9MBZLFlC31BrR37myMKardwg==",
    "@octokit/openapi-webhooks-types 12.1.0 sha512-WiuzhOsiOvb7W3Pvmhf8d2C6qaLHXrWiLBP4nJ/4kydu+wpagV5Fkz9RfQwV2afYzv3PB+3xYgp4mAdNGjDprA==",
    "@octokit/request-error 7.1.2 sha512-XZRuT3xZ84D3gYErI1DZvhJ33dCWVV6uzBtWkaBB4TvA/L6eOeTZodxLFVB44bBEEo3vEx7y00UfX1tBLrtLRg==",
    "@octokit/types 18.0.0 sha512-l6bAF43PNxkJp6g+W4PjoUSSkxHomXw2nOum5CTftJz1NlV3vu93NImgOYtLf6CbBUb5j+fiuzW0PPQ5JTSvZA==",
    "@octokit/webhooks 14.2.0 sha512-da6KbdNCV5sr1/txD896V+6W0iamFWrvVl8cHkBSPT+YlvmT3DwXa4jxZnQc+gnuTEqSWbBeoSZYTayXH9wXcw==",
    "@octokit/webhooks-methods 6.0.0 sha512-MFlzzoDJVw/GcbfzVC1RLR36QqkTLUf79vLVO3D+xn7r0QgxnFoLZgtrzxiQErAjFUOdH6fas2KeQJ1yr/qaXQ==",
    "@opentelemetry/api 1.9.1 sha512-gLyJlPHPZYdAk1JENA9LeHejZe1Ti77/pTeFm/nMXmQH/HFZlcS/O2XJB+L8fkbrNSqhdtlvjBVjxwUYanNH5Q==",
    "@opentelemetry/api-logs 0.220.0 sha512-CmVa4ImJ+ynfrPMNaAXHET6Bhb44SwzmfyVJFq9ni2jgXJR/l7C6gfVFddNmHP+ZOkP9cf4f9DBe68qVLTHc9w==",
    "@opentelemetry/core 2.11.0 sha512-7YP44XH0tV6+Mb54x2YGf84i7yi+31MBZlE8JwvozkxyTvXbSp10X7cI7YE49ChJ3shMJoBmCJF3+1QFBJctGA==",
    "@opentelemetry/core 2.9.0 sha512-m2nckMT80NnmjTYSPjJQObBJ+8dgkoajEOUbznL8AHZ3T3yHRk2P7gI1PhEBc1+lOnrYE9UWrWHqJDsmqjmNbw==",
    "@opentelemetry/exporter-logs-otlp-http 0.220.0 sha512-8186thl+pTw64iz/qEEen5oJZoZ/gO73XruChdaGlYdWOdBIQ42r+vHLf6a7vIDqTD4b8ZOoMlyxptanECaI9A==",
    "@opentelemetry/otlp-exporter-base 0.220.0 sha512-CXYo8UD5Mn9YbgebO2EL4wejtA+gxLmLiu6HCk2KH2BR7XhFN6/6p1UlCb23DYCjeYkndevLHuejCCN1yx4+OQ==",
    "@opentelemetry/otlp-transformer 0.220.0 sha512-lXGrv7KXZ0gNH9SVNUaa6vv6phVYGvJxfXAlMbzbakiXru75f5MZl8Z7oqiMMQD77riVHJCFlQvbZs/VVN2/4A==",
    "@opentelemetry/resources 2.11.0 sha512-Ie7+8q8MDF4FAEQCKVMTx3ReUvxiIAgIiiW3c9JdmP8+HMcDy20puT+AHjexnExgnbvBxjQ9fjkFDWrikJ2jQA==",
    "@opentelemetry/resources 2.9.0 sha512-jyA5MBLQ+Dkl3+JsZkUoUvL7yHvU64kLsvpXKarWm6347Sl1t1bXFTFykUePNpT5WH5pm9a2Qtt03iIYQhZ1Fg==",
    "@opentelemetry/sdk-logs 0.220.0 sha512-WywcTkQtv2iNmt+6y5Kcd4rzvx9bLVsBa2Nwcmg01IUaBTkTow3W4d9KE5vNBpEDtb9tp21WcRBY/lANRrApYA==",
    "@opentelemetry/sdk-metrics 2.9.0 sha512-Xx8RGS4H5XEBl01WuCreMIpiah9cCXMbSkeuIePPdD2cUpq/vUzYmj8E/MK1OsbOc93FuAD4jfn2WOacKwLn7Q==",
    "@opentelemetry/sdk-trace 2.9.0 sha512-sGA19HvtrrSKYsseHphluH6j3p6Xa3fqc7c7y8f/7mYWejc1lyDFcpSdD1kYa50HCLUeEo4zA5bW0pniaPszuw==",
    "@opentelemetry/semantic-conventions 1.43.0 sha512-eSYWTm620tTk45EKSedaUL8MFYI8hW164hIXsgIHyxu3VobUB3fFCu5t0hQby6OoWRPsG1KkKUG2M5UadiLiVg==",
    "@protobufjs/aspromise 1.1.2 sha512-j+gKExEuLmKwvz3OgROXtrJ2UG2x8Ch2YZUxahh+s1F2HZ+wAceUNLkvy6zKCPVRkU++ZWQrdxsUeQXmcg4uoQ==",
    "@protobufjs/base64 1.1.2 sha512-AZkcAA5vnN/v4PDqKyMR5lx7hZttPDgClv83E//FMNhR2TMcLUhfRUBHCmSl0oi9zMgDDqRUJkSxO3wm85+XLg==",
    "@protobufjs/codegen 2.0.5 sha512-zgXFLzW3Ap33e6d0Wlj4MGIm6Ce8O89n/apUaGNB/jx+hw+ruWEp7EwGUshdLKVRCxZW12fp9r40E1mQrf/34g==",
    "@protobufjs/eventemitter 1.1.1 sha512-vW1GmwMZNnL+gMRaovlh9yZX74kc+TTU3FObkkurpMaRtBfLP3ldjS9KQWlwZgraRE0+dheEEoAxdzcJQ8eXZg==",
    "@protobufjs/fetch 1.1.1 sha512-GpptLrs57adMSuHi3VNj0mAF8dwh36LMaYF6XyJ6JMWlVsc+t42tm1HSEDmOs3A8fC9yyeisgLhsTVQokOZ0zw==",
    "@protobufjs/float 1.0.2 sha512-Ddb+kVXlXst9d+R9PfTIxh1EdNkgoRe5tOX6t01f1lYWOvJnSPDBlG241QLzcyPdoNTsblLUdujGSE4RzrTZGQ==",
    "@protobufjs/path 1.1.2 sha512-6JOcJ5Tm08dOHAbdR3GrvP+yUUfkjG5ePsHYczMFLq3ZmMkAD98cDgcT2iA1lJ9NVwFd4tH/iSSoe44YWkltEA==",
    "@protobufjs/pool 1.1.0 sha512-0kELaGSIDBKvcgS4zkjz1PeddatrjYcmMWOlAuAPwAeccUrPHdUqo/J6LiymHHEiJT5NrF1UVwxY14f+fy4WQw==",
    "@protobufjs/utf8 1.1.2 sha512-b1UQwcEZ4yCnMCD8DAL1VlbvBJE9/IX4FTIp7BG1xYpf29SLazLSrqUkj4w7Y5y7cCVP6E5tcqqcI0xemPkHug==",
    "@smithy/core 3.34.1 sha512-dLcOUxz8YCv1RZUMKq6GbyUf95pLbrqh34bPvpCZ1+CByFF31BEAFewZjsGCnVsZTKdThNENfGyAgk2TJqVwSw==",
    "@smithy/credential-provider-imds 4.5.2 sha512-A9uSdn72ozbRUSit0eib0TW7nXuNPlaeM0zcGkJ+nE6tFcSDbnmtwoxbTCFBukVQcszDAyvsd7+rTduPTXpygg==",
    "@smithy/fetch-http-handler 5.8.0 sha512-ycSJu3tFAQ4v04CBB0agqFMVsSQ1iG3yw+SpgxRqKfaURpQD4CZ8Wn0zPMmSnOuTpTh65Vz+EA0rMrw089wvkA==",
    "@smithy/is-array-buffer 2.2.0 sha512-GGP3O9QFD24uGeAXYUjwSTXARoqpZykHadOmA8G5vfJPK0/DC67qa//0qvqrJzL1xc8WQWX7/yc7fwudjPHPhA==",
    "@smithy/node-http-handler 4.12.1 sha512-ThMkboGeONWXAelq9FvGsuJC4rOi+qyC4/zhUF58xYpxUg5sQKx2VXZYJmtNjr4dSuBJ1HeJXETQILCz3wOHvw==",
    "@smithy/node-http-handler 4.7.3 sha512-/jPhevcTFPMVl6KNjbaI47iOg1zxC7IsnX4PQDGVZKMFceOXtB8IEYaB7a9VvkP/3oC60WzTeKocvSI7vLT0vA==",
    "@smithy/signature-v4 5.7.3 sha512-7ImGm+FkHRLcBaRttIAMZ6bzJZWb2cJGoYjq46F2UjycujWzrL9GEN9h4w7eQyXJYnltrUhxbbieBAIRrdqpow==",
    "@smithy/types 4.18.0 sha512-CgB6HHWer/vrKps24ulRIbpcpb7K4xAU7SkZ7YHzBPlwHsvsrCJFEXK421s+cJzX+ZrqtA/TuU5w1HzI7k9N8A==",
    "@smithy/util-buffer-from 2.2.0 sha512-IJdWBbTcMQ6DA0gdNhh/BwrLkDR+ADW5Kr1aZmd4k3DIF6ezMV4R2NIAmT08wQJ3yUK82thHWmC/TnK/wpMMIA==",
    "@smithy/util-utf8 2.3.0 sha512-R8Rdn8Hy72KKcebgLiv8jQcQkXoLMOGGv5uI1/k0l+snqkOzQ1R0ChUBCxWMlBsFMekWjq0wRudIweFs7sKT5A==",
    "@stablelib/base64 1.0.1 sha512-1bnPQqSxSuc3Ii6MhBysoWCg58j97aUjuCSZrGSmDxNqtytIi0k8utUenAwTZN4V5mXXYGsVUI9zeBqy+jBOSQ==",
    "@standard-schema/spec 1.1.0 sha512-l2aFy5jALhniG5HgqrD6jXLi/rUWrKvqN/qJx6yoJsgKhblVd+iqqU4RCXavm/jPityDo5TCvKMnpjKnOriy0w==",
    "@types/node 26.6.2 sha512-X1P21scMv4zGKLYqjdGjaKa7COa0RKVYYZZN/NfvLQ1JegxFhdhpZG/Lyn8AXx6CDUavKAd11v6BvfpkDByK8g==",
    "@types/retry 0.12.0 sha512-wWKOClTTiizcZhXnPY4wikVAwmdYHp8q6DmC+EJUzAMsycb7HB32Kh9RN4+0gExjmPmZSAQjgURXIGATPegAvA==",
    "@vscode/ripgrep 1.18.0 sha512-ns5lWe44tSfbTMbVUsyB+I1819PVSw4AdpgK0RNkzfWfwy6+3IUNSxwSrfTno1/oWaS/hERNz+XLWVyga2aJBQ==",
    "@vscode/ripgrep-linux-x64 1.18.0 sha512-mQ3bVrUpnD2vs7QT0vX90Lt0cnUq467uFtEktIdsJJmW296RoSULRGqWgzG1AKxyBpNDD6l4ZO4qKf6SgyC23Q==",
    "@xterm/headless 6.0.0 sha512-5Yj1QINYCyzrZtf8OFIHi47iQtI+0qYFPHmouEfG8dHNxbZ9Tb9YGSuLcsEwj9Z+OL75GJqPyJbyoFer80a2Hw==",
    "accepts 2.0.0 sha512-5cvg6CtKwfgdmVqY1WIiXKc3Q1bkRqGLi+2W/6ao+6Y7gu/RCwRuAhGEzh5B4KlszSuTLgZYuqFqo5bImjNKng==",
    "agent-base 7.1.4 sha512-MnA+YT8fwfJPgBx3m60MNqakm30XOkyIoH1y6huTQvC0PwZG7ki8NacLBcrPbNoo8vEZy7Jpuk7+jMO+CUovTQ==",
    "ajv 8.20.0 sha512-Thbli+OlOj+iMPYFBVBfJ3OmCAnaSyNn4M1vz9T6Gka5Jt9ba/HIR56joy65tY6kx/FCF5VXNB819Y7/GUrBGA==",
    "ajv-formats 3.0.1 sha512-8iUql50EUR+uUcdRQ3HDqa6EVyo3docL8g5WJ3FNcWmu62IbkGUue/pEyLBW8VGKKucTPgqeks4fIU1DA4yowQ==",
    "argparse 2.0.1 sha512-8+9WqebbFzpX9OR+Wa6O29asIogeRMzcGtAINdpMHHyAg10f05aSFVBbcEqGf/PXw1EjAZ+q2/bEBg3DvurK3Q==",
    "base64-js 1.5.1 sha512-AKpaYlHn8t4SVbOHCy+b5+KKgvR4vrsD8vbvrbiQJps7fKDTkjkDry6ji0rUJjC0kzbNePLwzxq8iypo41qeWA==",
    "bignumber.js 9.3.1 sha512-Ko0uX15oIUS7wJ3Rb30Fs6SkVbLmPBAKdlm7q9+ak9bbIeFf0MwuBsQV6z7+X768/cHsfg+WlysDWJcmthjsjQ==",
    "body-parser 2.3.0 sha512-2cGmJupaNgg+QUwVLAucDuWuoMZ6EX9iHDRswZ5lsNYEmwPaRknMPCLZz07yTzVq/83p4o/wzbDZbBrTvGGTIw==",
    "bowser 2.14.1 sha512-tzPjzCxygAKWFOJP011oxFHs57HzIhOEracIgAePE4pqB3LikALKnSzUyU4MGs9/iCEUuHlAJTjTc5M+u7YEGg==",
    "buffer-equal-constant-time 1.0.1 sha512-zRpUiDwd/xk6ADqPMATG8vc9VPrkck7T07OIx0gnjmJAnHnTVXNQG3vfvWNuiZIkwu9KrKdA1iJKfsfTVxE6NA==",
    "bundle-name 4.1.0 sha512-tjwM5exMg6BGRI+kNmTntNsvdZS1X8BFYS6tnJ2hdH0kVxM6/eVZ2xy+FqStSWvYmtfFMDLIxurorHwDKfDz5Q==",
    "bytes 3.1.2 sha512-/Nf7TyzTx6S3yRJObOAV7956r8cr2+Oj8AC5dt8wSP3BQAoeX58NoHyCU8P8zGkNXStjTSi6fzO6F0pBdcYbEg==",
    "call-bind-apply-helpers 1.0.2 sha512-Sp1ablJ0ivDkSzjcaJdxEunN5/XvksFJ2sMBFfq6x0ryhQV/2b/KwFe21cMpmHtPOSij8K99/wSfoEuTObmuMQ==",
    "call-bound 1.0.4 sha512-+ys997U96po4Kx/ABpBCqhA9EuxJaQWDQg7295H4hBphv3IZg0boBKuwYpt4YXp6MZ5AmZQnU/tyMTlRpaSejg==",
    "chokidar 4.0.3 sha512-Qgzu8kfBvo+cA4962jnP1KkS6Dop5NS6g7R5LFYJr4b8Ub94PPQXUksCw9PvXoeXPRRddRNC5C1JQUR2SMGtnA==",
    "chokidar 5.0.0 sha512-TQMmc3w+5AxjpL8iIiwebF73dRDF4fBIieAqGn9RGCWaEVwQ6Fb2cGe31Yns0RRIzii5goJ1Y7xbMwo1TxMplw==",
    "commander 15.0.0 sha512-z67u4ZhzCL/Tydu1lJARtEZYWbWaN7oYLHbsuzocr6y4N6WZAagG3RQ4FW61V1/0+jImpj293XfrcYnd1qxtPg==",
    "compressible 2.0.18 sha512-AF3r7P5dWxL8MxyITRMlORQNaOA2IkAFaTr4k7BUumjPtRpGDTZpl0Pb1XCO6JeDCBdp126Cgs9sMxqSjgYyRg==",
    "compression 1.8.2 sha512-o8vI5RE5A6EVVOd9o41jKp41aJom+QTEO/Bx8MYNjexMo/Bv2WOjUfZr+aL0WnYSgymUy6zeguqLTsIhV0gMvQ==",
    "content-disposition 1.1.0 sha512-5jRCH9Z/+DRP7rkvY83B+yGIGX96OYdJmzngqnw2SBSxqCFPd0w2km3s5iawpGX8krnwSGmF0FW5Nhr0Hfai3g==",
    "content-type 1.0.5 sha512-nTjqfcBFEipKdXCv4YDQWCfmcLZKm81ldF0pAopTvyrFGVbcR6P/VAAd5G7N+0tTr8QqiU0tFadD6FK4NtJwOA==",
    "content-type 2.1.0 sha512-mj7UPXE0jaqaOsukNZRUEfEi2AcL7C/vwmwcHV0O97eO1E1pxBZuyjlZrx5seTaNBg1U6+o35wpa35Qfcc+7ag==",
    "cookie 0.7.2 sha512-yki5XnKuf750l50uGTllt6kKILY4nQ1eNIQatoXEByZ5dWgnKqbnqmTrBE5B4N7lrMJKQ2ytWMiTO2o0v6Ew/w==",
    "cookie-signature 1.2.2 sha512-D76uU73ulSXrD1UXF4KE2TMxVVwhsnCgfAyTg9k8P6KGZjlXKrOLe4dJQKI3Bxi5wjesZoFXJWElNWBjPZMbhg==",
    "cors 2.8.6 sha512-tJtZBBHA6vjIAaF6EnIaq6laBBP9aq/Y3ouVJjEfoHbRBcHBAHYcMh/w8LDrk2PvIMMq8gmopa5D4V8RmbrxGw==",
    "cross-spawn 7.0.6 sha512-uV2QOWP2nWzsy2aMp8aRibhi9dlzF5Hgh5SHaB9OiTGEyDTiJJyx0uy51QXdyWbtAHNua4XJzUKca3OzKUd3vA==",
    "data-uri-to-buffer 4.0.1 sha512-0R9ikRb668HB7QDxT1vkpuUBtqc53YyAwMwGeUFKRojY/NWKvdZ+9UYtRfGmhqNbRkTSVpMbmyhXipFFv2cb/A==",
    "debug 2.6.9 sha512-bC7ElrdJaJnPbAP+1EotYvqZsb3ecl5wi6Bfi6BJTUcNowp6cvspg0jXznRTKDjm/E7AdgFBVeAPVMNcKGsHMA==",
    "debug 4.4.3 sha512-RGwwWnwQvkVfavKVt22FGLw+xYSdzARwm0ru6DhTVA3umU5hZc28V3kO4stgYryrTlLpuvgI9GiijltAjNbcqA==",
    "default-browser 5.5.1 sha512-m1pAzaJgZ/gssEqlOhJkPJp8Xly7QyW6xcrkUa2KKcDeDSEMP7X8xipU3snUcfisTQx0w1AGae+9UtJSfVnXGw==",
    "default-browser-id 5.0.1 sha512-x1VCxdX4t+8wVfd1so/9w+vQ4vx7lKd2Qp5tDRutErwmR85OgmfX7RlLRMWafRMY7hbEiXIbudNrjOAPa/hL8Q==",
    "define-lazy-prop 3.0.0 sha512-N+MeXYoqr3pOgn8xfyRPREN7gHakLYjhsHhWGT3fWAiL4IkAt0iDw14QiiEm2bE30c5XX5q0FtAA3CK5f9/BUg==",
    "depd 2.0.0 sha512-g7nH6P6dyDioJogAAGprGpCtVImJhpPk/roCzdb3fIh61/s/nPsfR6onyMwkCAR/OlC3yBC0lESvUoQEAssIrw==",
    "destroy 1.2.0 sha512-2sJGJTaXIIaR1w4iJSNoN0hnMY7Gpc/n8D4qSCJw8QqFWXf7cuAgnEHxBpweaVcPevC2l3KpjYCx3NypQQgaJg==",
    "detect-libc 2.1.2 sha512-Btj2BOOO83o3WyH59e8MgXsxEQVcarkUOpEYrubB0urwnN10yQ364rsiByU11nZlqWYZm05i/of7io4mzihBtQ==",
    "diff 9.0.0 sha512-svtcdpS8CgJyqAjEQIXdb3OjhFVVYjzGAPO8WGCmRbrml64SPw/jJD4GoE98aR7r25A0XcgrK3F02yw9R/vhQw==",
    "dunder-proto 1.0.1 sha512-KIN/nDJBQRcXw0MLVhZE9iQHmG68qAVIBg9CqmUYjmQIhgij9U5MFvrqkUL5FbtyyzZuOeOt0zdeRe4UY7ct+A==",
    "ecdsa-sig-formatter 1.0.11 sha512-nagl3RYrbNv6kQkeJIpt6NJZy8twLB/2vtz6yN9Z4vRKHN4/QZJIEbqohALSgwKdnksuY3k5Addp5lg8sVoVcQ==",
    "ee-first 1.1.1 sha512-WMwm9LhRUo+WUaRN+vRuETqG89IgZphVSNkdFgeb6sS/E4OrDIN7t48CAewSHXc6C8lefD8KKfr5vY61brQlow==",
    "encodeurl 2.0.0 sha512-Q0n9HRi4m6JuGIV1eFlmvJB7ZEVxu93IrMyiMsGC0lrMJMWzRgx6WGquyfQgZVb31vhGgXnfmPNNXmxnOkRBrg==",
    "es-define-property 1.0.1 sha512-e3nRfgfUZ4rNGL232gUgX06QNyyez04KdjFrF+LTRoOXmrOgFKDg4BCdsjW8EnT69eqdYGmRpJwiPVYNrCaW3g==",
    "es-errors 1.3.0 sha512-Zf5H2Kxt2xjTvbJvP2ZWLEICxA6j+hAmMzIlypy4xcBg1vKVnx89Wy0GbS+kf5cwCVFFzdCFh2XSCFNULS6csw==",
    "es-object-atoms 1.1.2 sha512-HWcBoN6NileqtSydK2FqHbS/LoDd2pqrnQHLyJzBj4kOp/ky2MWMN694xOfkK8/SnUsW2DH7EfyVlydKCsm1Zw==",
    "escape-html 1.0.3 sha512-NiSupZ4OeuGwr68lGIeym/ksIZMJodUGOSCZ/FSnTxcrekbvqrgdUxlJOMpijaKZVjAJrWrGs/6Jy8OMuyj9ow==",
    "etag 1.8.1 sha512-aIL5Fx7mawVa300al2BnEE4iNvo1qETxLrPI/o05L7z6go7fCw1J6EQmbK4FmJ2AS7kgVF/KEZWufBfdClMcPg==",
    "eventsource 3.0.7 sha512-CRT1WTyuQoD771GW56XEZFQ/ZoSfWid1alKGDYMmkt2yl8UXrVR4pspqWNEcqKvVIzg6PAltWjxcSSPrboA4iA==",
    "eventsource-parser 3.1.1 sha512-EKN1vKAMcZ8MlYMpaNuxN6R9yakzH6uajHcHVTqWJzvu5pWw9DyhbP35HH8MVBQ+dZjAfDxk+A8NiR9KWaXiyQ==",
    "express 5.2.1 sha512-hIS4idWWai69NezIdRt2xFVofaF4j+6INOpJlVOLDO8zXGpUVEVzIYk12UUi2JzjEzWL3IOAxcTubgz9Po0yXw==",
    "express-rate-limit 8.7.0 sha512-hOwV7WOxXfjRpAM1DSJWZDXx3GhplwD8IfwuwvogD8i1Qnkgosw/H45s4ZnFAUHDAhPjlY9hLBvJhKmGMyY26g==",
    "extend 3.0.2 sha512-fjquC59cD7CyW6urNXK0FBufkZcoiGG80wTuPujX590cB5Ttln20E2UB4S/WARVqhXffZl2LNgS+gQdPIIim/g==",
    "fast-deep-equal 3.1.3 sha512-f3qQ9oQy9j2AhBe/H9VC91wLmKBCCU/gDOnKNAYG5hswO7BLKj09Hc5HYNz9cGI++xlpDCIgDaitVs03ATR84Q==",
    "fast-sha256 1.3.0 sha512-n11RGP/lrWEFI/bWdygLxhI+pVeo1ZYIVwvvPkW7azl/rOy+F3HYRZ2K5zeE9mmkhQppyv9sQFx0JM9UabnpPQ==",
    "fast-uri 3.1.8 sha512-GZMtZUTNRpOVIECoXwLNZS5xUGE+mVNbTB8h/7Rwh2TFWcBQiPzTgyZi05BF9UMZKkLJv8XBRJTlU7zg8+ZfMg==",
    "fetch-blob 3.2.0 sha512-7yAQpD2UMJzLi1Dqv7qFYnPbaPx7ZfFK6PiIxQ4PfkGPyNyl2Ugx+a/umUonmKqjhM4DnfbMvdX6otXq83soQQ==",
    "fflate 0.8.3 sha512-tbZNuJrLwGUp3zshBtdy4W+ORxZuIh8a5ilyIEQDC5rY1f3U20JMry0Ll3WBzU58EZKsEuJFXhb5gwv8CsPvgA==",
    "finalhandler 2.1.1 sha512-S8KoZgRZN+a5rNwqTxlZZePjT/4cnm0ROV70LedRHZ0p8u9fRID0hJUZQpkKLzro8LfmC8sx23bY6tVNxv8pQA==",
    "formdata-polyfill 4.0.10 sha512-buewHzMvYL29jdeQTVILecSaZKnt/RJWjoZCF5OW60Z67/GmSLBkOFM7qh1PI3zFNtJbaZL5eQu1vLfazOwj4g==",
    "forwarded 0.2.0 sha512-buRG0fpBtRHSTCOASe6hD258tEubFoRLb4ZNA6NxMVHNw2gOcwHo9wyablzMzOA5z9xA9L1KNjk/Nt6MT9aYow==",
    "fresh 2.0.0 sha512-Rx/WycZ60HOaqLKAi6cHRKKI7zxWbJ31MhntmtwMoaTeF7XFH9hhBp8vITaMidfljRQ6eYWCKkaTK+ykVJHP2A==",
    "function-bind 1.1.2 sha512-7XHNxH7qX9xG5mIwxkhumTox/MIRNcOgDrxWsMt2pAr23WHp6MrRlN7FBSFpCpr+oVO0F744iUgR82nJMfG2SA==",
    "gaxios 7.3.1 sha512-kB3rzJV7d9juLZh8/56QTXCwQfxyhdOMdyYk1HdQKFtF8TJTDTZQJtixWIwXdE9Jji91mC41DUNpjleo4L4eAQ==",
    "gcp-metadata 8.1.2 sha512-zV/5HKTfCeKWnxG0Dmrw51hEWFGfcF2xiXqcA3+J90WDuP0SvoiSO5ORvcBsifmx/FoIjgQN3oNOGaQ5PhLFkg==",
    "get-intrinsic 1.3.0 sha512-9fSjSaos/fRIVIp+xSJlE6lfwhES7LNtKaCBIamHsjr2na1BiABJPo0mOjjz8GJDURarmCPGqaiVg5mfjb98CQ==",
    "get-proto 1.0.1 sha512-sTSfBjoXBp89JvIKIefqw7U2CCebsc74kiY6awiGogKtoSGbgjYE/G/+l9sF3MWFPNc9IcoOC4ODfKHfxFmp0g==",
    "google-auth-library 10.9.1 sha512-i1ydyHrqcIxXkWh/uBmVkzCvIuq5yiK2ATndIe5XxKholrG/MTYP9xGYka4sQhrbIAgGjL2B6NOE7rFaiF3fXw==",
    "google-logging-utils 1.1.3 sha512-eAmLkjDjAFCVXg7A1unxHsLf961m6y17QFqXqAXGj/gVkKFrEICfStRfwUlGNfeCEjNRa32JEWOUTlYXPyyKvA==",
    "gopd 1.2.0 sha512-ZUKRh6/kUFoAiTAtTYPZJ3hw9wNxx+BIBOijnlG9PnrJsCcSjs1wyyD6vJpaYtgnzDrKYRSqf3OO6Rfa93xsRg==",
    "has-symbols 1.1.0 sha512-1cDNdwJ2Jaohmb3sg4OmKaMBwuC48sYni5HUw2DvsC8LjGTLK9h+eb1X6RyuOHe4hT0ULCW68iomhjUoKUqlPQ==",
    "hasown 2.0.4 sha512-T2UbfbBEF32wiepXIsMlTW9+dDYC6wMh/t/vYA4tuOMKqWz/n3vr1NFSxQiyP+zk2mXsoMA/i/7qV6LKut1t1A==",
    "hono 4.13.8 sha512-/Gng7NfoykZl2pjukW5Z6+8Yxm3BPRf86GTbQnt0SbySkvax4fyL4H3HhY1cCpBGmiW9XDRFzRV+CXK2W8QudQ==",
    "http-errors 2.0.1 sha512-4FbRdAX+bSdmo4AUFuS0WNiPz8NgFt+r8ThgNWmlrjQjt1Q7ZR9+zTlce2859x4KSXrwIsaeTqDoKQmtP8pLmQ==",
    "http-proxy-agent 7.0.2 sha512-T1gkAiYYDWYx3V5Bmyu7HcfcvL7mUrTWiM6yOfa3PIphViJ/gFPbvidQ+veqSOHci/PxBcDabeUNCzpOODJZig==",
    "https-proxy-agent 7.0.6 sha512-vK9P5/iUfdl95AI+JVyUuIcVtd4ofvtrOr3HNtM2yxC9bnMbEdp3x01OhQNnjb8IJYi38VlTE3mBXwcfvywuSw==",
    "iconv-lite 0.7.3 sha512-IKXpvIzjnC9XTAUbVBcMfGS0EPaIXtW6v+zr+RRp+hqULEpo0owZax6wyRwPOJbWbzjYspQwusTsfVr0ifh4uQ==",
    "inherits 2.0.4 sha512-k/vGaX4/Yla3WzyMCvTQOXYeIHvqOKtnqBduzTHpzpQZzAskKMhZ2K+EnBiSM9zGSoIFeMpXKxa4dYeZIQqewQ==",
    "ip-address 10.7.2 sha512-7H/2gFSIitxc0hG3nOI1glS8QLo/EHBFFLk8vEUjXY/xu0AdL8jZ9U1IzO2PUm0d2D/ofQcAifb0g6OBkt8U7w==",
    "ipaddr.js 1.9.1 sha512-0KI/607xoxSToH7GjN1FfSbLoU0+btTicjsQSWQlh/hZykN8KpmMf7uYwPW3R+akZ6R/w18ZlXSHBYXiYUPO3g==",
    "ipaddr.js 2.5.0 sha512-aq+t5NAc+cS6rZQQVWC2x98CPqGtKKTMDd4Gaodv0wShnItdKg/51djkGJ1hqH+Oy0ivDftCbSLCQob8zso01w==",
    "is-docker 3.0.0 sha512-eljcgEDlEns/7AXFosB5K/2nCM4P7FQPkGc/DWLy5rmFEWvZayGrik1d9/QIY5nJ4f9YsVvBkA6kJpHn9rISdQ==",
    "is-in-ssh 1.0.0 sha512-jYa6Q9rH90kR1vKB6NM7qqd1mge3Fx4Dhw5TVlK1MUBqhEOuCagrEHMevNuCcbECmXZ0ThXkRm+Ymr51HwEPAw==",
    "is-inside-container 1.0.0 sha512-KIYLCCJghfHZxqjYBE7rEy0OBuTd5xCHS7tHVgvCLkx7StIoaxwNW3hCALgEUjFfeRk+MG/Qxmp/vtETEF3tRA==",
    "is-promise 4.0.0 sha512-hvpoI6korhJMnej285dSg6nu1+e6uxs7zG3BYAm5byqDsgJNWwxzM6z6iZiAgQR4TJ30JmBTOwqZUw3WlyH3AQ==",
    "is-wsl 3.1.1 sha512-e6rvdUCiQCAuumZslxRJWR/Doq4VpPR82kqclvcS0efgt430SlGIk05vdCN58+VrzgtIcfNODjozVielycD4Sw==",
    "isexe 2.0.0 sha512-RHxMLp9lnKHGHRng9QFhRCMbYAcVpn69smSGcq3f36xjgVVWThj4qqLbTLlq7Ssj8B+fIQ1EuCEGI2lKsyQeIw==",
    "jose 6.2.12 sha512-9NiFmJEex0sy2Dk58j2UGBSHgUs2ypF9eZSu4L6vjOX3Dp96Sw1F3uL+H+D1sx02jZZdzUT0HgvCy59CuvXcWw==",
    "js-tokens 4.0.0 sha512-RdJUflcE3cUzKiMqQgsCu06FPu9UdIJO0beYbPhHN4k6apgJtifcoCtT9bcxOpYBtpD2kCM6Sbzg4CausW/PKQ==",
    "js-yaml 4.3.2 sha512-SFNOvSJ+Dgf/9An904Yx+CgSlIPCkIpao4qo51lpee25TIRejdH3rhR4EZMGoNx3/TP3O+wzWuiTFl4sqbltzA==",
    "json-bigint 1.0.0 sha512-SiPv/8VpZuWbvLSMtTDU8hEfrZWg/mH/nV/b4o0CYbSxu1UIQPLdwKOCIyLQX+VIPO5vrLX3i8qtqFyhdPSUSQ==",
    "json-schema-to-ts 3.1.1 sha512-+DWg8jCJG2TEnpy7kOm/7/AxaYoaRbjVB4LFZLySZlWn8exGs3A4OLJR966cVvU26N7X9TWxl+Jsw7dzAqKT6g==",
    "json-schema-traverse 1.0.0 sha512-NM8/P9n3XjXhIZn1lLhkFaACTOURQXjWhV4BA/RnOv8xvgqtqpAX9IO4mRQxSx1Rlo4tqzeqb0sOlruaOy3dug==",
    "json-schema-typed 8.0.2 sha512-fQhoXdcvc3V28x7C7BMs4P5+kNlgUURe2jmUT1T//oBRMDrqy1QPelJimwZGo7Hg9VPV3EQV5Bnq4hbFy2vetA==",
    "jwa 2.0.1 sha512-hRF04fqJIP8Abbkq5NKGN0Bbr3JxlQ+qhZufXVr0DvujKy93ZCbXZMHDL4EOtodSbCWxOqR8MS1tXA5hwqCXDg==",
    "jws 4.0.1 sha512-EKI/M/yqPncGUUh44xz0PxSidXFr/+r0pA70+gIYhjv+et7yxM+s29Y+VGDkovRofQem0fs7Uvf4+YmAdyRduA==",
    "koffi 3.3.1 sha512-FZYfhBfYQr/cmHhZpaQ9rhbKpkjHhxaytn2A0mbPhiD5yV8QzF3SiQwR1w/34rCJ1fw+6qBQSj/ZoXRdC+Xc2Q==",
    "long 5.3.2 sha512-mNAgZ1GmyNhD7AuqnTG3/VQ26o760+ZYBPKjPvugO8+nLbYfX6TVpJPseBvopbdY+qpZ/lKUnmEc1LeZYS3QAA==",
    "math-intrinsics 1.1.0 sha512-/IXtbwEk5HTPyEwyKX6hGkYXxM9nbj64B+ilVJnC/R6B0pH5G4V3b0pVbL7DBj4tkhBAppbQUlf6F6Xl9LHu1g==",
    "media-typer 1.1.1 sha512-yz3xRaG20c6/BOzvYoDaGtPmGscs7YivItZEEqe6GbwNfHuxu9YNmvnEkMzKldAGY4/80pRcQRZSEnhquk9XuQ==",
    "merge-descriptors 2.0.0 sha512-Snk314V5ayFLhp3fkUREub6WtjBfPdCPY1Ln8/8munuLuiYhsABgBVWsozAG+MWMbVEvcdcpbi9R7ww22l9Q3g==",
    "mime-db 1.54.0 sha512-aU5EJuIN2WDemCcAp2vFBfp/m4EAhWJnUNSSw0ixs7/kXbd6Pg64EmwJkNdFhB8aWt1sH2CTXrLxo/iAGV3oPQ==",
    "mime-types 3.0.2 sha512-Lbgzdk0h4juoQ9fCKXW4by0UJqj+nOOrI9MJ1sSj4nI8aI2eo1qmvQEie4VD1glsS250n15LsWsYtCugiStS5A==",
    "ms 2.0.0 sha512-Tpp60P6IUJDTuOq/5Z8cdskzJujfwqfOTkrwIwj7IRISpnkJnT6SyJ4PCPnGMoFjC9ddhal5KVIYtAt97ix05A==",
    "ms 2.1.3 sha512-6FlzubTLZG3J2a/NVCAleEhjzq5oxgHyaCU9yYXvcLsvoVaHJq/s5xXI6/XXP6tz7R9xAOtHnSO/tXtF3WRTlA==",
    "negotiator 0.6.4 sha512-myRT3DiWPHqho5PrJaIRyaMv2kgYf0mUVgBNOYMuCH5Ki1yEiQaf/ZJuQ62nvpc44wL5WDbTX7yGJi1Neevw8w==",
    "negotiator 1.1.0 sha512-NMPBRMJgiQHjbd8phG3Vebdx4kZ1H121rbl5IkMqeOsahptB9BKo/d7oJ3zTXqTgagn2bWlNSXkh0QUGM31RYg==",
    "node-addon-api 7.1.1 sha512-5m3bsyrjFWE1xf7nz7YXdN4udnVtXK6/Yfgn5qnahL6bCkf2yKt4k3nuTKAtT4r3IG8JNR2ncsIMdZuAzJjHQQ==",
    "node-addon-native-custom-loader 0.1.6 sha512-QaW7d8lTcGCXrpNVyFDtID3vrpa7vFdpcY9pql2/9FGFSe7tcSnhQOKZX+1aOWq+pB1d2ZwNnKAdQI5nD9wAXA==",
    "node-addon-require-builtin 0.1.6 sha512-P9ZGMDkloktirLJSggfpxsJ9jog5FItE1Omxpj50UBn3LhD6TS6/yx0jEBXsCGK3P9EtW1EpQVA1QL5gb11GaQ==",
    "node-addon-require-builtin-linux-x64-gnu 0.1.6 sha512-8wCxrFlB2Ld5DhxIm2P+nNFnQ0wjzDE5sXF68EUhLH0X+z313B07TUQFMBKCOkyhZIIBh590dBmhHCuA43No2A==",
    "node-domexception 1.0.0 sha512-/jKZoMpw0F8GRwl4/eLROPA3cfcXtLApP0QzLmUT/HuPCZWyB7IY9ZrMeKw2O/nFIqPQB3PVM9aYm0F312AXDQ==",
    "node-fetch 3.3.2 sha512-dRB78srN/l6gqWulah9SrxeYnxeddIG30+GOqK/9OlLVyLg3HPnr6SqOWTWOXKRwC2eGYCkZ59NNuSgvSrpgOA==",
    "node-pty 1.2.0-beta.15 sha512-vORSzHXi4Ofl7HemVWpuudLqCPdaQb4LfpRCUpE5HPxhp4JYscl8zZwxh11p26v2wvW24WMwnMfLjhRLixrfxA==",
    "object-assign 4.1.1 sha512-rJgTQnkUnH1sFw8yT6VSU3zD3sWmu6sZhIseY8VX+GRu3P6F7Fu+JNDoXfklElbLJSnc3FUQHVe4cU5hj+BcUg==",
    "object-inspect 1.13.4 sha512-W67iLl4J2EXEGTbfeHCffrjDfitvLANg0UlX3wFUUSTx92KXRFegMHUVgSqE+wvhAbi4WqjGg9czysTV2Epbew==",
    "on-finished 2.4.1 sha512-oVlzkg3ENAhCk2zdv7IJwd/QUD4z2RxRwpkcGY8psCVcCYZNq4wYnVWALHM+brtuJjePWiYF/ClmuDr8Ch5+kg==",
    "on-headers 1.1.0 sha512-737ZY3yNnXy37FHkQxPzt4UZ2UWPWiCZWLvFZ4fu5cueciegX0zGPnrlY6bwRg4FdQOe9YU8MkmJwGhoMybl8A==",
    "once 1.4.0 sha512-lNaJgI+2Q5URQBkccEKHTQOPaXdUxnZZElQTZY0MFUAuaEqe1E+Nyvgdz/aIyNi6Z9MzO5dv1H8n58/GELp3+w==",
    "open 11.0.4 sha512-++Zlftm0kVLPmzC06t6epuWmcRMDbI4z5P3NNX979WA/k23+NtSOynEGzsVfZwguKw2mi5umVgnBlJQMwRz4Pg==",
    "openai 6.40.0 sha512-MWtTjd/gQt4jpbji61NTgFWJLoY/PdRJ6wG9/ZDRMYNMlBKrCrSlkLI+KgHP1vR1qT6LKSAyAqIxno6lcK9JiA==",
    "p-retry 4.6.2 sha512-312Id396EbJdvRONlngUx0NydfrIQ5lsYu0znKVUzVvArzEIt08V1qhtyESbGVd1FGX7UKtiFp5uwKZdM8wIuQ==",
    "parseurl 1.3.3 sha512-CiyeOxFT/JZyN5m0z9PfXw4SCBJ6Sygz1Dpl0wqjlhDEGGBP1GnsUVEL0p63hoG1fcj3fHynXi9NYO4nWOL+qQ==",
    "partial-json 0.1.7 sha512-Njv/59hHaokb/hRUjce3Hdv12wd60MtM9Z5Olmn+nehe0QDAsRtRbJPvJ0Z91TusF0SuZRIvnM+S4l6EIP8leA==",
    "path-key 3.1.1 sha512-ojmeN0qd+y0jszEtoY48r0Peq5dwMEkIlCOu6Q5f41lfkswXuKtYrhgoTpLnyIcHm24Uhqx+5Tqm2InSwLhE6Q==",
    "path-to-regexp 8.4.2 sha512-qRcuIdP69NPm4qbACK+aDogI5CBDMi1jKe0ry5rSQJz8JVLsC7jV8XpiJjGRLLol3N+R5ihGYcrPLTno6pAdBA==",
    "picocolors 1.1.1 sha512-xceH2snhtb5M9liqDsmEw56le376mTZkEX/jEb/RxNFyegNul7eNslCXP9FDj/Lcu0X8KEyMceP2ntpaHrDEVA==",
    "picomatch 4.0.7 sha512-qcJu88Q2IWqJsDD529JKMdwGm/dvInW4HvQnRwiH9JtihJvzGOscDtHE3x1pBKeUOTysQ8kVmLnJ2kJu7yhcGA==",
    "pkce-challenge 5.0.1 sha512-wQ0b/W4Fr01qtpHlqSqspcj3EhBvimsdh0KlHhH8HRZnMsEa0ea2fTULOXOS9ccQr3om+GcGRk4e+isrZWV8qQ==",
    "powershell-utils 0.1.0 sha512-dM0jVuXJPsDN6DvRpea484tCUaMiXWjuCn++HGTqUWzGDjv5tZkEZldAJ/UMlqRYGFrD/etByo4/xOuC/snX2A==",
    "powershell-utils 0.2.1 sha512-C+y9x90UElAddDZmV4qOx9W53B61PO7cIqWz2dQsWlwswuq4mr8NEwytdGKboYbQlGZ3awrkTeNvcZiZNHnQ8A==",
    "protobufjs 7.6.6 sha512-dYDWdjSl5RNb7SgPxGQcRU+GtvP7s2fpkrY0r432PcOIaZ0/rBcxEZnQN67iJhFuQiVw754JDoPruPCNdGsbjg==",
    "proxy-addr 2.0.8 sha512-5nnx0yGyVUcY6t9RnWcARWtwT9F1D8O9rt08htPvnd49W1IgZtmLkhu9WfMzQj1cFxjHIO6connUNVW5k7AVyQ==",
    "qs 6.16.0 sha512-h6fhOIaRrID2CbEY2fqs+7t+UXZo+MLAnU5gRIq85uFtdiUPCdsApMlHhXogKVM4HM2DVbIjGNTTYH2OcmP1vA==",
    "range-parser 1.3.0 sha512-hek2mFQpPuI4E1BBKrSto+BU3e3x4xuarsbiwr3+lf7p44juvFMV0XFWQAP3xUyqXA4RrXLIoaSUGbSt056ZMw==",
    "raw-body 3.0.2 sha512-K5zQjDllxWkf7Z5xJdV0/B0WTNqx6vxG70zJE4N0kBs4LovmEYWJzQGxC9bS9RAKu3bgM40lrd5zoLJ12MQ5BA==",
    "readdirp 4.1.2 sha512-GDhwkLfywWL2s6vEjyhri+eXmfH6j1L7JE27WhqLeYzoh/A3DBaYGEj2H/HFZCn/kMfim73FXxEJTw06WtxQwg==",
    "readdirp 5.1.1 sha512-Kko+Y5XQ6fM+Ce3dq3m9YGxnacYZYl9cA1wZjaF3Vbry2L3i1qVg8+CAgNPsXRArPMUMCaOR7oa9Nqntc43JKA==",
    "require-from-string 2.0.2 sha512-Xf0nWe6RseziFMu+Ap9biiUbmplq6S9/p+7w7YXP/JBHhrUDDUhwa+vANyubuqfZWTveU//DYVGsDG7RKL/vEw==",
    "resolve.exports 2.0.3 sha512-OcXjMsGdhL4XnbShKpAcSqPMzQoYkYyhbEaeSko47MjRP9NfEQMhZkXL1DoFlt9LWQn4YttrdnV6X2OiyzBi+A==",
    "retry 0.13.1 sha512-XQBQ3I8W1Cge0Seh+6gjj03LbmRFWuoszgK9ooCpwYIrhhoO80pfq4cUkU5DkknwfOfFteRwlZ56PYOGYyFWdg==",
    "router 2.2.0 sha512-nLTrUKm2UyiL7rlhapu/Zl45FwNgkZGaCpZbIHajDYgwlJCOzLSk+cIPAnsEqV955GjILJnKbdQC1nVPz+gAYQ==",
    "run-applescript 7.1.0 sha512-DPe5pVFaAsinSaV6QjQ6gdiedWDcRCbUuiQfQa2wmWV7+xC9bGulGI8+TdRmoFkAPaBXk8CrAbnlY2ISniJ47Q==",
    "safe-buffer 5.2.1 sha512-rp3So07KcdmmKbGvgaNxQSJr7bGVSVk5S9Eq1F+ppbRo70+YeaDxkw5Dd8NPN+GD6bjnYm2VuPuCXmpuYvmCXQ==",
    "safer-buffer 2.1.2 sha512-YZo3K82SD7Riyi0E1EQPojLz7kpepnSQI9IyPbHHg1XXXevb5dJI7tpyN2ADxGcQbHG7vcyRHk0cbwqcQriUtg==",
    "semver 7.8.5 sha512-Y7/KDsb8LjooZpwaqGyulO6DQlksgCncchHGk+sZIY4SBvUocMBEFH5Ur1fI4dV+Jvl0w6cjvucaIi40puRioA==",
    "send 1.2.1 sha512-1gnZf7DFcoIcajTjTwjwuDjzuz4PPcY2StKPlsGAQ1+YH20IRVrBaXSWmdjowTJ6u8Rc01PoYOGHXfP1mYcZNQ==",
    "serve-static 2.2.1 sha512-xRXBn0pPqQTVQiC8wyQrKs2MOlX24zQ0POGaj0kultvoOCstBQM5yvOhAVSUwOMjQtTvsPWoNCHfPGwaaQJhTw==",
    "setprototypeof 1.2.0 sha512-E5LDX7Wrp85Kil5bhZv46j8jOeboKq5JMmYM3gVGdGH8xFpPWXUMsNrlODCrkoxMEeNi/XZIwuRvY4XNwYMJpw==",
    "sharp 0.35.4 sha512-n++8XWcj+jCOr2IOl7h8LbKnGBDY4aPbmprMONBNFdn0ImXqpGVv5zliDs0V9HbmbCQLpbuo2ej9rAoOQTvMDA==",
    "shebang-command 2.0.0 sha512-kHxr2zZpYtdmrN1qDjrrX/Z1rR1kG8Dx+gkpK1G4eXmvXswmcE1hTWBWYUzlraYw1/yZp6YuDY77YtvbN0dmDA==",
    "shebang-regex 3.0.0 sha512-7++dFhtcx3353uBaq8DDR4NuxBetBzC7ZQOhmTQInHEd6bSrXdiEyzCvG07Z44UYdLShWUyXt5M/yhz8ekcb1A==",
    "side-channel 1.1.1 sha512-6x6dK6zJdpTzF4sQeNYxwtvBzf6Eg4GtlesS94HOvTudUeyK2WXAaIfmDgsyslYrRBeFIlsi54AYsFGUuhmvrQ==",
    "side-channel-list 1.0.1 sha512-mjn/0bi/oUURjc5Xl7IaWi/OJJJumuoJFQJfDDyO46+hBWsfaVM65TBHq2eoZBhzl9EchxOijpkbRC8SVBQU0w==",
    "side-channel-map 1.0.1 sha512-VCjCNfgMsby3tTdo02nbjtM/ewra6jPHmpThenkTYh8pG9ucZ/1P8So4u4FGBek/BjpOVsDCMoLA/iuBKIFXRA==",
    "side-channel-weakmap 1.0.2 sha512-WPS/HvHQTYnHisLo9McqBHOJk2FkHO/tlpvldyrnem4aeQp4hai3gythswg6p01oSoTl58rcpiFAjF2br2Ak2A==",
    "standardwebhooks 1.1.1 sha512-bCbX9ZEyFkWPsRz7Bl3NuQUJohmwGSev/yhr7vhaGPlc4AfIrspIRa6cPTBuI1ItmrTDJ4d/S2hCsfe4+vQGnQ==",
    "statuses 2.0.2 sha512-DvEy55V3DB7uknRo+4iOGT5fP1slR8wQohVdknigZPMpMstaKJQWhwiYBACJE3Ul2pTnATihhBYnRhZQHGBiRw==",
    "toidentifier 1.0.1 sha512-o5sSPKEkg/DIQNmH43V0/uerLrpzVedkUh8tGNvaeXpfpuwjKenlSox/2O/BTlZUtEe+JG7s5YhEz608PlAHRA==",
    "ts-algebra 2.0.0 sha512-FPAhNPFMrkwz76P7cdjdmiShwMynZYN6SgOujD1urY4oNm80Ou9oMdmbR45LotcKOXoy7wSmHkRFE6Mxbrhefw==",
    "tslib 2.8.1 sha512-oJFu94HQb+KVduSUQL7wnpmqnfmLsOA/nAh6b6EH0wCEoK0/mPeXU6c3wKDV83MkOuHPRHtSXKKU99IBazS/2w==",
    "turndown 7.2.4 sha512-I8yFsfRzmzK0WV1pNNOA4A7y4RDfFxPRxb3t+e3ui14qSGOxGtiSP6GjeX+Y6CHb7HYaFj7ECUD7VE5kQMZWGQ==",
    "type-is 2.1.0 sha512-faYHw0anBbc/kWF3zFTEnxSFOAGUX9GFbOBthvDdLsIlEoWOFOtS0zgCiQYwIskL9iGXZL3kAXD8OoZ4GmMATA==",
    "typebox 1.3.7 sha512-meKuifc33Pccx0O6PdIzYMq3Og8zvP4TIi/a+Bw3AEMZMxOD0+RHGQvpglEe6Zdy3wZ8nqn/j95h8LUZLk/6Hg==",
    "undici 8.10.2 sha512-/y4/bH9YNU5hi9NIrpOuvGXFcxrj3CMrV+/AYpowAYTpHn8gX/XPFjNy766FPoYY0miQhdW977JFWKGNhBdwyQ==",
    "undici-types 8.9.0 sha512-KTDyRTYX8sWmKXAikPHHSyc63CRPETMctyjKFupcC6OBLXT3xsN0e9aF7m+mIXutFWpUXuedtowG7iLOzp0kQg==",
    "unpipe 1.0.0 sha512-pjy2bYhSsufwWlKwPc+l3cN7+wuJlK6uz0YdJEOlQDbl6jo/YlPi4mb8agUkVC8BF7V8NuzeyPNqRksA3hztKQ==",
    "vary 1.1.2 sha512-BNGbWLfd0eUPabhkXUVm0j8uuvREyTh5ovRa/dyow/BqAbZJyC+5fU+IzQOzmAKzYqYRAISoRhdQr3eIZ/PXqg==",
    "web-streams-polyfill 3.3.3 sha512-d2JWLCivmZYTSIoge9MsgFCZrt571BikcWGYkjC1khllbTeDlGqZ2D8vD8E/lJa8WGWbb7Plm8/XJYV7IJHZZw==",
    "which 2.0.2 sha512-BLI3Tl1TW3Pvl70l3yq3Y64i+awpwXqsGBYWkkqMtnbXgrMD+yj7rhW0kuEDxzJaYXGjEW5ogapKNMEKNMjibA==",
    "wrappy 1.0.2 sha512-l4Sp/DRseor9wL6EvV2+TuQn63dMkPjZ/sp9XkghTEbV9KlPS1xUsZ3u7/IQO4wxtcFB4bgpQPRcR3QCvezPcQ==",
    "ws 8.21.3 sha512-201TZ/kPWxoPr/OKWjquZR1SWKXcvxdH+e1xrx89b3YbmzLMFCLfnaG1HFIgWzJOEWZ7MvpK++odZufgYR50Rw==",
    "wsl-utils 1.0.0 sha512-Hl0ZOAs672vg+06kfujwRhoS6/jehvULrlFkuF2dRu6pHgA8U06h3xqNIqNNU1LTXPcedxByAR4GS6pwQK0mgA==",
    "yaml 2.9.1 sha512-3NxN8+78OdzbT7C/WjGsyfPAtJaN3FNDsWxv7Y7mcDsT/oOmgW8BpyQQFFBnvZE3j9Y2Sdz1ULFLezL7Eb2yFw==",
    "zod 4.6.5 sha512-v5l/aFXZQeai4awLbOpSoHecE9UiMrnfx75tEXLjNonXVARxQ5mOeipTjROUchszUNCqnE+hqAMujRsRHsut2Q==",
    "zod-to-json-schema 3.25.2 sha512-O/PgfnpT1xKSDeQYSCfRI5Gy3hPf91mKVDuYLUHZJMiDFptvP41MSnWofm8dnCm0256ZNfZIM7DSzuSMAFnjHA==",
];

/// The plugin component the sole Rust producer computes over the six
/// unchanged committed files, and the canonical composite it computes
/// over the measured inputs above. Both are pinned FIXTURE results:
/// they are what these bytes yield, not a claim that a retained-home
/// recording or task 8.8's acceptance is complete.
const MEASURED_PLUGIN_COMPONENT: &str =
    "074d1b111148cd3f1770a5afc23e1589fbef61cc940c49385e97da8117e2eda5";
const MEASURED_CANONICAL_COMPOSITE: &str =
    "a64fcd6d048603ecb1767b229fa0fb6a30d9ae7cda92a47cdc82360d9ee3ddd1";

/// The measured profile patch, byte for byte: 217 bytes, SHA-256
/// `ef189a8c27db6d63930aa3046a3040482e952eafcb7487c644d508e8d461f027`.
const MEASURED_PROFILE_PATCH: &str = r#"# Your patch layer for this dsh profile, applied after every bundle layer:
# a top-level YAML array of loader patch entries (id-targeted config
# overrides, disables, and insert lists; `!!js` expressions allowed).
[]
"#;

/// The measured profile pnpm lock, byte for byte: 1,982 bytes, 57 lines,
/// longest line 186, SHA-256
/// `4708752f0463211bf25d470fc26befa49748707b9c12fae7b4f2544e02b21055`.
/// Its importer's `specifier` records an absolute install path and is
/// never read; the `packages:` section is.
const MEASURED_PNPM_LOCK: &str = r#"lockfileVersion: '9.0'

settings:
  autoInstallPeers: false
  excludeLinksFromLockfile: false

importers:

  .:
    dependencies:
      dsh-plugin-cli-session:
        specifier: file:/home/vyanakiev/brokkr-dsh-qual-0155rc2/pack/dsh-plugin-cli-session-0.2.0.tgz
        version: file:../../../pack/dsh-plugin-cli-session-0.2.0.tgz

packages:

  '@deepseek-ai/cosmokit@1.8.3':
    resolution: {integrity: sha512-qBo+ronVM6Eu2WNVJXi8JcMiqZ19T9BRIpV+5qJUFPXjGH/Z0QKcQMC/IZJ7L394YTOtJgcovbk9qP0w2GsBXQ==}

  '@deepseek-ai/schemastery@3.18.1':
    resolution: {integrity: sha512-Qn0FCSwCQnpnj6SB31I6i2sIKgKWnkbJM8O0EU91Gv2UsYVvtZTl6IA0sCwk2e2MZf5S8w5hpq9QkeVvK9qwxg==}

  '@standard-schema/spec@1.1.0':
    resolution: {integrity: sha512-l2aFy5jALhniG5HgqrD6jXLi/rUWrKvqN/qJx6yoJsgKhblVd+iqqU4RCXavm/jPityDo5TCvKMnpjKnOriy0w==}

  commander@15.0.0:
    resolution: {integrity: sha512-z67u4ZhzCL/Tydu1lJARtEZYWbWaN7oYLHbsuzocr6y4N6WZAagG3RQ4FW61V1/0+jImpj293XfrcYnd1qxtPg==}
    engines: {node: '>=22.12.0'}

  dsh-plugin-cli-session@file:../../../pack/dsh-plugin-cli-session-0.2.0.tgz:
    resolution: {integrity: sha512-VZDwJO6nPlQNvHUIealeWgLIN/6jwyNEG/E2fbztWu7uIPoy3wLQqt36yTN0mAaXM5TBXE+iPN9deKnMgQUViA==, tarball: file:../../../pack/dsh-plugin-cli-session-0.2.0.tgz}
    version: 0.2.0
    engines: {node: ^22.19.0 || >=24.0.0}
    peerDependencies:
      '@deepseek-ai/cordis': '>=4.0.1'
      '@deepseek-ai/dsh-agent': '>=0.1.0-rc.6'
      '@deepseek-ai/dsh-cmdline': '>=0.1.0-rc.6'
      '@deepseek-ai/dsh-llm': '>=0.1.0-rc.6'
      '@deepseek-ai/dsh-session': '>=0.1.0-rc.6'

snapshots:

  '@deepseek-ai/cosmokit@1.8.3': {}

  '@deepseek-ai/schemastery@3.18.1':
    dependencies:
      '@deepseek-ai/cosmokit': 1.8.3
      '@standard-schema/spec': 1.1.0

  '@standard-schema/spec@1.1.0': {}

  commander@15.0.0: {}

  dsh-plugin-cli-session@file:../../../pack/dsh-plugin-cli-session-0.2.0.tgz:
    dependencies:
      '@deepseek-ai/schemastery': 3.18.1
      commander: 15.0.0
"#;

/// The core's complete hidden npm lock, byte for byte: 311,184 bytes,
/// 522 `packages` entries, every one carrying integrity, SHA-256
/// `b84bac2d866224a997be29811dc71bde6013dbc6e2adf8c1e77523e6f05a3847`.
/// It is embedded whole because a three-entry excerpt cannot prove the
/// measured locator set: the counts that separate a correct reader from
/// a plausible one — 521 records, 501 triples, 489 names — only exist in
/// the complete lock.
const MEASURED_NPM_LOCK: &str = r#"{
  "name": "core",
  "version": "1.0.0",
  "lockfileVersion": 3,
  "requires": true,
  "packages": {
    "node_modules/@agentclientprotocol/sdk": {
      "version": "1.4.0",
      "resolved": "https://registry.npmjs.org/@agentclientprotocol/sdk/-/sdk-1.4.0.tgz",
      "integrity": "sha512-/eufudw+aFY1LKLolT6yFE6UMmYRl7fMJ/DEONSIyR6wI3slHWITBsANRGqXEY8FRzqUxwh7QEaGiZHcJPVThg==",
      "license": "Apache-2.0",
      "peerDependencies": {
        "zod": "^3.25.0 || ^4.0.0"
      }
    },
    "node_modules/@anthropic-ai/sdk": {
      "version": "0.123.0",
      "resolved": "https://registry.npmjs.org/@anthropic-ai/sdk/-/sdk-0.123.0.tgz",
      "integrity": "sha512-Y9oX9mPNGZClHQOFqrWRk43Srcu/UHuPq3rfxxOq7JgW0gi+lJA2MAOK4Ul3k/+AUrwRWFJvd0tK3oC0Pw25dw==",
      "license": "MIT",
      "dependencies": {
        "json-schema-to-ts": "^3.1.1",
        "standardwebhooks": "^1.0.0"
      },
      "bin": {
        "anthropic-ai-sdk": "bin/cli"
      },
      "peerDependencies": {
        "zod": "^3.25.0 || ^4.0.0"
      },
      "peerDependenciesMeta": {
        "zod": {
          "optional": true
        }
      }
    },
    "node_modules/@aws-crypto/sha256-browser": {
      "version": "5.2.0",
      "resolved": "https://registry.npmjs.org/@aws-crypto/sha256-browser/-/sha256-browser-5.2.0.tgz",
      "integrity": "sha512-AXfN/lGotSQwu6HNcEsIASo7kWXZ5HYWvfOmSNKDsEqC4OashTp8alTmaz+F7TC2L083SFv5RdB+qU3Vs1kZqw==",
      "license": "Apache-2.0",
      "dependencies": {
        "@aws-crypto/sha256-js": "^5.2.0",
        "@aws-crypto/supports-web-crypto": "^5.2.0",
        "@aws-crypto/util": "^5.2.0",
        "@aws-sdk/types": "^3.222.0",
        "@aws-sdk/util-locate-window": "^3.0.0",
        "@smithy/util-utf8": "^2.0.0",
        "tslib": "^2.6.2"
      }
    },
    "node_modules/@aws-crypto/sha256-js": {
      "version": "5.2.0",
      "resolved": "https://registry.npmjs.org/@aws-crypto/sha256-js/-/sha256-js-5.2.0.tgz",
      "integrity": "sha512-FFQQyu7edu4ufvIZ+OadFpHHOt+eSTBaYaki44c+akjg7qZg9oOQeLlk77F6tSYqjDAFClrHJk9tMf0HdVyOvA==",
      "license": "Apache-2.0",
      "dependencies": {
        "@aws-crypto/util": "^5.2.0",
        "@aws-sdk/types": "^3.222.0",
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=16.0.0"
      }
    },
    "node_modules/@aws-crypto/supports-web-crypto": {
      "version": "5.2.0",
      "resolved": "https://registry.npmjs.org/@aws-crypto/supports-web-crypto/-/supports-web-crypto-5.2.0.tgz",
      "integrity": "sha512-iAvUotm021kM33eCdNfwIN//F77/IADDSs58i+MDaOqFrVjZo9bAal0NK7HurRuWLLpF1iLX7gbWrjHjeo+YFg==",
      "license": "Apache-2.0",
      "dependencies": {
        "tslib": "^2.6.2"
      }
    },
    "node_modules/@aws-crypto/util": {
      "version": "5.2.0",
      "resolved": "https://registry.npmjs.org/@aws-crypto/util/-/util-5.2.0.tgz",
      "integrity": "sha512-4RkU9EsI6ZpBve5fseQlGNUWKMa1RLPQ1dnjnQoe07ldfIzcsGb5hC5W0Dm7u423KWzawlrpbjXBrXCEv9zazQ==",
      "license": "Apache-2.0",
      "dependencies": {
        "@aws-sdk/types": "^3.222.0",
        "@smithy/util-utf8": "^2.0.0",
        "tslib": "^2.6.2"
      }
    },
    "node_modules/@aws-sdk/client-bedrock-runtime": {
      "version": "3.1048.0",
      "resolved": "https://registry.npmjs.org/@aws-sdk/client-bedrock-runtime/-/client-bedrock-runtime-3.1048.0.tgz",
      "integrity": "sha512-u+NT61JZEkRFtpL0CAw1N1dwxnaLgwVXQl/zjJxTGgLyS/jTIdg2SdoEoCTHxgDyCnqa1HEi9QOoE9/pYRNpOQ==",
      "license": "Apache-2.0",
      "dependencies": {
        "@aws-crypto/sha256-browser": "5.2.0",
        "@aws-crypto/sha256-js": "5.2.0",
        "@aws-sdk/core": "^3.974.11",
        "@aws-sdk/credential-provider-node": "^3.972.42",
        "@aws-sdk/eventstream-handler-node": "^3.972.16",
        "@aws-sdk/middleware-eventstream": "^3.972.12",
        "@aws-sdk/middleware-websocket": "^3.972.19",
        "@aws-sdk/token-providers": "3.1048.0",
        "@aws-sdk/types": "^3.973.8",
        "@smithy/core": "^3.24.2",
        "@smithy/fetch-http-handler": "^5.4.2",
        "@smithy/node-http-handler": "^4.7.2",
        "@smithy/types": "^4.14.1",
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=20.0.0"
      }
    },
    "node_modules/@aws-sdk/core": {
      "version": "3.978.0",
      "resolved": "https://registry.npmjs.org/@aws-sdk/core/-/core-3.978.0.tgz",
      "integrity": "sha512-2yX9LUmxPklVjSGTb8dfnWRJSiFQ3TeH2nn7G1mdKHTfnabzF0+gfrS8rYfLWmZrQ8A3mEcxMJjRc51dL5KWaA==",
      "license": "Apache-2.0",
      "dependencies": {
        "@aws-sdk/types": "^3.974.5",
        "@aws-sdk/xml-builder": "^3.972.40",
        "@aws/lambda-invoke-store": "^0.3.0",
        "@smithy/core": "^3.33.3",
        "@smithy/signature-v4": "^5.6.12",
        "@smithy/types": "^4.17.2",
        "bowser": "^2.11.0",
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=20.0.0"
      }
    },
    "node_modules/@aws-sdk/credential-provider-env": {
      "version": "3.972.71",
      "resolved": "https://registry.npmjs.org/@aws-sdk/credential-provider-env/-/credential-provider-env-3.972.71.tgz",
      "integrity": "sha512-JN+JHruYZw3GUZB8YGAlDk4wTDPOEAEEdEzj5nS0xodWR4smzHsN7PnK2j6IeOsDIj2aqua5DSbhXl9Gtf90FQ==",
      "license": "Apache-2.0",
      "dependencies": {
        "@aws-sdk/core": "^3.978.0",
        "@aws-sdk/types": "^3.974.5",
        "@smithy/core": "^3.33.3",
        "@smithy/types": "^4.17.2",
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=20.0.0"
      }
    },
    "node_modules/@aws-sdk/credential-provider-http": {
      "version": "3.972.73",
      "resolved": "https://registry.npmjs.org/@aws-sdk/credential-provider-http/-/credential-provider-http-3.972.73.tgz",
      "integrity": "sha512-uyYYnJOnlis8uQzaYGPd7N1JoioCoNpXgnkXYixsWJXHXgXyYi8WXJSDfofxJeWfQIGWLe2Nwyq60Uc7MZdVOg==",
      "license": "Apache-2.0",
      "dependencies": {
        "@aws-sdk/core": "^3.978.0",
        "@aws-sdk/types": "^3.974.5",
        "@smithy/core": "^3.33.3",
        "@smithy/fetch-http-handler": "^5.7.2",
        "@smithy/node-http-handler": "^4.11.3",
        "@smithy/types": "^4.17.2",
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=20.0.0"
      }
    },
    "node_modules/@aws-sdk/credential-provider-http/node_modules/@smithy/node-http-handler": {
      "version": "4.12.1",
      "resolved": "https://registry.npmjs.org/@smithy/node-http-handler/-/node-http-handler-4.12.1.tgz",
      "integrity": "sha512-ThMkboGeONWXAelq9FvGsuJC4rOi+qyC4/zhUF58xYpxUg5sQKx2VXZYJmtNjr4dSuBJ1HeJXETQILCz3wOHvw==",
      "license": "Apache-2.0",
      "dependencies": {
        "@smithy/core": "^3.33.3",
        "@smithy/types": "^4.18.0",
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=18.0.0"
      }
    },
    "node_modules/@aws-sdk/credential-provider-ini": {
      "version": "3.973.16",
      "resolved": "https://registry.npmjs.org/@aws-sdk/credential-provider-ini/-/credential-provider-ini-3.973.16.tgz",
      "integrity": "sha512-i++ly+0Uxa+u3ebSSyr0S/3CFhFJDxCXT3+Zj+mW2bXenEx5bKGCdTIKFu39SgXBNhWDjex/8cXUx9MUTMCrTw==",
      "license": "Apache-2.0",
      "dependencies": {
        "@aws-sdk/core": "^3.978.0",
        "@aws-sdk/credential-provider-env": "^3.972.71",
        "@aws-sdk/credential-provider-http": "^3.972.73",
        "@aws-sdk/credential-provider-login": "^3.972.78",
        "@aws-sdk/credential-provider-process": "^3.972.71",
        "@aws-sdk/credential-provider-sso": "^3.973.15",
        "@aws-sdk/credential-provider-web-identity": "^3.972.77",
        "@aws-sdk/nested-clients": "^3.997.45",
        "@aws-sdk/types": "^3.974.5",
        "@smithy/core": "^3.33.3",
        "@smithy/credential-provider-imds": "^4.4.16",
        "@smithy/types": "^4.17.2",
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=20.0.0"
      }
    },
    "node_modules/@aws-sdk/credential-provider-login": {
      "version": "3.972.78",
      "resolved": "https://registry.npmjs.org/@aws-sdk/credential-provider-login/-/credential-provider-login-3.972.78.tgz",
      "integrity": "sha512-eUtswnXu0+Ii9ieRK+0L7aPFV3Z/dnW2VntJzjBP9xs8s+8p5nBNuymIXtXwZ+5r5+XJP3e32nMkuZ/r0HozEA==",
      "license": "Apache-2.0",
      "dependencies": {
        "@aws-sdk/core": "^3.978.0",
        "@aws-sdk/nested-clients": "^3.997.45",
        "@aws-sdk/types": "^3.974.5",
        "@smithy/core": "^3.33.3",
        "@smithy/types": "^4.17.2",
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=20.0.0"
      }
    },
    "node_modules/@aws-sdk/credential-provider-node": {
      "version": "3.972.83",
      "resolved": "https://registry.npmjs.org/@aws-sdk/credential-provider-node/-/credential-provider-node-3.972.83.tgz",
      "integrity": "sha512-jdso7ejzfRnatxMUZK4S/U6KbaDPCvfIV4XL+IQAPFDBt5rj5Fq595euqlK8Le4lNCMFR9oUpt+1l0aMgaayOQ==",
      "license": "Apache-2.0",
      "dependencies": {
        "@aws-sdk/credential-provider-env": "^3.972.71",
        "@aws-sdk/credential-provider-http": "^3.972.73",
        "@aws-sdk/credential-provider-ini": "^3.973.16",
        "@aws-sdk/credential-provider-process": "^3.972.71",
        "@aws-sdk/credential-provider-sso": "^3.973.15",
        "@aws-sdk/credential-provider-web-identity": "^3.972.77",
        "@aws-sdk/types": "^3.974.5",
        "@smithy/core": "^3.33.3",
        "@smithy/credential-provider-imds": "^4.4.16",
        "@smithy/types": "^4.17.2",
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=20.0.0"
      }
    },
    "node_modules/@aws-sdk/credential-provider-process": {
      "version": "3.972.71",
      "resolved": "https://registry.npmjs.org/@aws-sdk/credential-provider-process/-/credential-provider-process-3.972.71.tgz",
      "integrity": "sha512-lYmXJa4gvq4xN1lrT5NiP5vIYYKcGWAdj8y+8o6dlcateB5eF3Dn8DtmjjHKfMBrTPAMr2pebIiX/UOj8c1/UA==",
      "license": "Apache-2.0",
      "dependencies": {
        "@aws-sdk/core": "^3.978.0",
        "@aws-sdk/types": "^3.974.5",
        "@smithy/core": "^3.33.3",
        "@smithy/types": "^4.17.2",
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=20.0.0"
      }
    },
    "node_modules/@aws-sdk/credential-provider-sso": {
      "version": "3.973.15",
      "resolved": "https://registry.npmjs.org/@aws-sdk/credential-provider-sso/-/credential-provider-sso-3.973.15.tgz",
      "integrity": "sha512-6Jhcf4v0pSFdjk1EW2kvzuEBKD+UZ2uNcHUIglKKLndD20YhvkL2kdmDOV5/j4mYuWWwe/a1FQ1aomU86/Cg5Q==",
      "license": "Apache-2.0",
      "dependencies": {
        "@aws-sdk/core": "^3.978.0",
        "@aws-sdk/nested-clients": "^3.997.45",
        "@aws-sdk/token-providers": "3.1129.0",
        "@aws-sdk/types": "^3.974.5",
        "@smithy/core": "^3.33.3",
        "@smithy/types": "^4.17.2",
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=20.0.0"
      }
    },
    "node_modules/@aws-sdk/credential-provider-sso/node_modules/@aws-sdk/token-providers": {
      "version": "3.1129.0",
      "resolved": "https://registry.npmjs.org/@aws-sdk/token-providers/-/token-providers-3.1129.0.tgz",
      "integrity": "sha512-Sbl3rpzQdsG4ZK2zh0JWUYyZPKKorJlVOddA2T0DVbKJFrsW8J6wgnslxxUH04+WaBMr4A1HzJZvZX0xUvkniA==",
      "license": "Apache-2.0",
      "dependencies": {
        "@aws-sdk/core": "^3.978.0",
        "@aws-sdk/nested-clients": "^3.997.45",
        "@aws-sdk/types": "^3.974.5",
        "@smithy/core": "^3.33.3",
        "@smithy/types": "^4.17.2",
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=20.0.0"
      }
    },
    "node_modules/@aws-sdk/credential-provider-web-identity": {
      "version": "3.972.77",
      "resolved": "https://registry.npmjs.org/@aws-sdk/credential-provider-web-identity/-/credential-provider-web-identity-3.972.77.tgz",
      "integrity": "sha512-uylIQSUWpfLuH2LovxEEfwzJGM/SabLOfLMg6YXu/E8jJEKUdpdILCVCQCdFvHyu/7dLJOHPMfrSwduxO56NkQ==",
      "license": "Apache-2.0",
      "dependencies": {
        "@aws-sdk/core": "^3.978.0",
        "@aws-sdk/nested-clients": "^3.997.45",
        "@aws-sdk/types": "^3.974.5",
        "@smithy/core": "^3.33.3",
        "@smithy/types": "^4.17.2",
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=20.0.0"
      }
    },
    "node_modules/@aws-sdk/eventstream-handler-node": {
      "version": "3.972.34",
      "resolved": "https://registry.npmjs.org/@aws-sdk/eventstream-handler-node/-/eventstream-handler-node-3.972.34.tgz",
      "integrity": "sha512-cTeVzpu1xEAkryTZBYhGwnQ6gOGyp8ZYZvmn0Sg/nI/ABmy/CRHHxPDJDUi9PxwxUtGGaatvfRUB3FCgT/rSWw==",
      "license": "Apache-2.0",
      "dependencies": {
        "@aws-sdk/types": "^3.974.5",
        "@smithy/core": "^3.33.3",
        "@smithy/types": "^4.17.2",
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=20.0.0"
      }
    },
    "node_modules/@aws-sdk/middleware-eventstream": {
      "version": "3.972.29",
      "resolved": "https://registry.npmjs.org/@aws-sdk/middleware-eventstream/-/middleware-eventstream-3.972.29.tgz",
      "integrity": "sha512-dlRzHCgyB8W6hLuDC5pcT5q+ziPt00n4QGgGBE17ucLVU4zMa6lsbuUdQ2Pm75Z5VA8GF+R/+SgrRcaTdIzSIQ==",
      "license": "Apache-2.0",
      "dependencies": {
        "@aws-sdk/types": "^3.974.5",
        "@smithy/core": "^3.33.3",
        "@smithy/types": "^4.17.2",
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=20.0.0"
      }
    },
    "node_modules/@aws-sdk/middleware-websocket": {
      "version": "3.972.53",
      "resolved": "https://registry.npmjs.org/@aws-sdk/middleware-websocket/-/middleware-websocket-3.972.53.tgz",
      "integrity": "sha512-bIrDaMENQmYRBHntOiOheqkiw5+fhKW4Lqb+mS1uqF0VwvdWI22fW2HFgWrng66CmYd+4k8ePlpj38sEfTuMLQ==",
      "license": "Apache-2.0",
      "dependencies": {
        "@aws-sdk/core": "^3.978.0",
        "@aws-sdk/types": "^3.974.5",
        "@smithy/core": "^3.33.3",
        "@smithy/fetch-http-handler": "^5.7.2",
        "@smithy/signature-v4": "^5.6.12",
        "@smithy/types": "^4.17.2",
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">= 14.0.0"
      }
    },
    "node_modules/@aws-sdk/nested-clients": {
      "version": "3.997.45",
      "resolved": "https://registry.npmjs.org/@aws-sdk/nested-clients/-/nested-clients-3.997.45.tgz",
      "integrity": "sha512-mooq9Q+jLa18VoM7HouczmslZU60iiB0aKc/Ztnq/luIL1ud0z4DnYprLR/ZO1gp331S9tJctM1HZr7u6YKBXQ==",
      "license": "Apache-2.0",
      "dependencies": {
        "@aws-sdk/core": "^3.978.0",
        "@aws-sdk/signature-v4-multi-region": "^3.996.46",
        "@aws-sdk/types": "^3.974.5",
        "@smithy/core": "^3.33.3",
        "@smithy/fetch-http-handler": "^5.7.2",
        "@smithy/node-http-handler": "^4.11.3",
        "@smithy/types": "^4.17.2",
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=20.0.0"
      }
    },
    "node_modules/@aws-sdk/nested-clients/node_modules/@smithy/node-http-handler": {
      "version": "4.12.1",
      "resolved": "https://registry.npmjs.org/@smithy/node-http-handler/-/node-http-handler-4.12.1.tgz",
      "integrity": "sha512-ThMkboGeONWXAelq9FvGsuJC4rOi+qyC4/zhUF58xYpxUg5sQKx2VXZYJmtNjr4dSuBJ1HeJXETQILCz3wOHvw==",
      "license": "Apache-2.0",
      "dependencies": {
        "@smithy/core": "^3.33.3",
        "@smithy/types": "^4.18.0",
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=18.0.0"
      }
    },
    "node_modules/@aws-sdk/signature-v4-multi-region": {
      "version": "3.996.46",
      "resolved": "https://registry.npmjs.org/@aws-sdk/signature-v4-multi-region/-/signature-v4-multi-region-3.996.46.tgz",
      "integrity": "sha512-L+2xZTye/2T96f3lwCws0Zw6GG2JHZW9e8FpVgGBeeExSKyeoZ6CWRpBml/7DNiK/O26jrgPM9F+Ay8VkgzUWQ==",
      "license": "Apache-2.0",
      "dependencies": {
        "@aws-sdk/types": "^3.974.5",
        "@smithy/signature-v4": "^5.6.12",
        "@smithy/types": "^4.17.2",
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=20.0.0"
      }
    },
    "node_modules/@aws-sdk/token-providers": {
      "version": "3.1048.0",
      "resolved": "https://registry.npmjs.org/@aws-sdk/token-providers/-/token-providers-3.1048.0.tgz",
      "integrity": "sha512-k0y/GcuesuSfWyUM0WamrGyeZmltRYaPbHO82UDA6mZ/doB+FOHKutikPAtSXMn/hDz970cF+iRuuiYO9VEbAA==",
      "license": "Apache-2.0",
      "dependencies": {
        "@aws-sdk/core": "^3.974.11",
        "@aws-sdk/nested-clients": "^3.997.9",
        "@aws-sdk/types": "^3.973.8",
        "@smithy/core": "^3.24.2",
        "@smithy/types": "^4.14.1",
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=20.0.0"
      }
    },
    "node_modules/@aws-sdk/types": {
      "version": "3.974.5",
      "resolved": "https://registry.npmjs.org/@aws-sdk/types/-/types-3.974.5.tgz",
      "integrity": "sha512-LkwLL2BLbC6wNNm4JaH9mbEqBMdOZCct6VAYqhdN4U1xrWM+fUJQEfbHwQgDypapOWTRtlk25akb5afM0P8CIQ==",
      "license": "Apache-2.0",
      "dependencies": {
        "@smithy/types": "^4.17.2",
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=20.0.0"
      }
    },
    "node_modules/@aws-sdk/util-locate-window": {
      "version": "3.965.10",
      "resolved": "https://registry.npmjs.org/@aws-sdk/util-locate-window/-/util-locate-window-3.965.10.tgz",
      "integrity": "sha512-ycwH6Zd2GhuSqdXX9ihbCjeGTB6xOJs+O3+Jb8/zDG9978XU80qs75dfkPJRMNKe5MvBZPuNeFpd4JZKPoUF4g==",
      "license": "Apache-2.0",
      "dependencies": {
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=20.0.0"
      }
    },
    "node_modules/@aws-sdk/xml-builder": {
      "version": "3.972.40",
      "resolved": "https://registry.npmjs.org/@aws-sdk/xml-builder/-/xml-builder-3.972.40.tgz",
      "integrity": "sha512-wlFmCIGUlwF4zx/kncw+bmxTQh1HeSJq4mYV/V5cZUSJadDP3kXvGW8Rn21cimj/7y9ju+47oYWXi97vF7czaA==",
      "license": "Apache-2.0",
      "dependencies": {
        "@smithy/types": "^4.17.2",
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=20.0.0"
      }
    },
    "node_modules/@aws/lambda-invoke-store": {
      "version": "0.3.0",
      "resolved": "https://registry.npmjs.org/@aws/lambda-invoke-store/-/lambda-invoke-store-0.3.0.tgz",
      "integrity": "sha512-sl4Bm6yiMNYrZKkqqDFWN0UfnWhlS8ivKxrYl+6t0gCLrqr8y3B2IqZZbFRkfaVVp7C/baApyh71P+LeE1A2sQ==",
      "license": "Apache-2.0",
      "engines": {
        "node": ">=18.0.0"
      }
    },
    "node_modules/@babel/code-frame": {
      "version": "7.29.7",
      "resolved": "https://registry.npmjs.org/@babel/code-frame/-/code-frame-7.29.7.tgz",
      "integrity": "sha512-Aup7aUOfpbAUg2ROOJN6Iw5f9DMBlzu0mIkm/malLQFN/YQgO48wCj0Kxa3sEHJvPVFg7siR+qRInwXd2qhQKw==",
      "license": "MIT",
      "dependencies": {
        "@babel/helper-validator-identifier": "^7.29.7",
        "js-tokens": "^4.0.0",
        "picocolors": "^1.1.1"
      },
      "engines": {
        "node": ">=6.9.0"
      }
    },
    "node_modules/@babel/helper-validator-identifier": {
      "version": "7.29.7",
      "resolved": "https://registry.npmjs.org/@babel/helper-validator-identifier/-/helper-validator-identifier-7.29.7.tgz",
      "integrity": "sha512-qehxGkRj55h/ff8EMaJ+cYhyaKlHIxqYDn682wQD7RNp9UujOQsHog2uS0r2vzr4pW+sXf90NeeayjcNaX3fFg==",
      "license": "MIT",
      "engines": {
        "node": ">=6.9.0"
      }
    },
    "node_modules/@babel/runtime": {
      "version": "7.29.7",
      "resolved": "https://registry.npmjs.org/@babel/runtime/-/runtime-7.29.7.tgz",
      "integrity": "sha512-Nq8OhGWiZIZGV6hLHoyAKLLcJihP/xFeBMGJoUrxTX2psI8dCifzLhZISFb+VWS3wFMRDmCGw5R+dOySCqPLhw==",
      "license": "MIT",
      "engines": {
        "node": ">=6.9.0"
      }
    },
    "node_modules/@deepseek-ai/cordis": {
      "version": "4.0.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/cordis/-/cordis-4.0.2.tgz",
      "integrity": "sha512-asOnXP1TzFSFQlHb1iegDZp0z/8WD1c7YNrwJR/Tx2bzNuMXfcekE/I67Iv6SQXeLB4csxqCngzQKANP7gdw0g==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/cosmokit": "^1.8.3",
        "@standard-schema/spec": "^1.1.0"
      },
      "bin": {
        "cordis": "bin.js"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis-plugin-include": "^1.0.7",
        "@deepseek-ai/cordis-plugin-loader": "^1.0.3"
      },
      "peerDependenciesMeta": {
        "@deepseek-ai/cordis-plugin-include": {
          "optional": true
        },
        "@deepseek-ai/cordis-plugin-loader": {
          "optional": true
        }
      }
    },
    "node_modules/@deepseek-ai/cordis-plugin-group": {
      "version": "1.0.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/cordis-plugin-group/-/cordis-plugin-group-1.0.2.tgz",
      "integrity": "sha512-OeGiPD793Mhma9+rGIox95neuufwOFTqQ5jooFgrMK/Mn7Fp7Hkv/d7VKMXZz40iks2fZCbiFE6p9WjTVTQRdg==",
      "license": "MIT",
      "peer": true,
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/cordis-plugin-loader": "^1.0.3"
      }
    },
    "node_modules/@deepseek-ai/cordis-plugin-hmr": {
      "version": "1.0.17",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/cordis-plugin-hmr/-/cordis-plugin-hmr-1.0.17.tgz",
      "integrity": "sha512-o1BooaJpwd+C80Tn5+B48MGqrfNZydlfRVQjmCrrULA4nCmlMqBKjDF2FmcZoNP+L2uX9m/6DdGKVtFpQRQVUQ==",
      "license": "MIT",
      "dependencies": {
        "@babel/code-frame": "^7.29.0",
        "@deepseek-ai/cosmokit": "^1.8.3",
        "@deepseek-ai/schemastery": "^3.18.2",
        "chokidar": "^4.0.3",
        "picomatch": "^4.0.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/cordis-plugin-timer": "^1.1.4"
      }
    },
    "node_modules/@deepseek-ai/cordis-plugin-include": {
      "version": "1.0.7",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/cordis-plugin-include/-/cordis-plugin-include-1.0.7.tgz",
      "integrity": "sha512-bZ4S1YuOmwOeE97HnslQ6ggVQbaWz4GejJ8OcY4P/DIyc1Qhu/3Szt1XSw6c/pd37kRkxxJXGwt4cmCxWSBBuw==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/cosmokit": "^1.8.3",
        "js-yaml": "^4.1.0"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/cordis-plugin-loader": "^1.0.3"
      }
    },
    "node_modules/@deepseek-ai/cordis-plugin-loader": {
      "version": "1.0.3",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/cordis-plugin-loader/-/cordis-plugin-loader-1.0.3.tgz",
      "integrity": "sha512-YNcHiH7TRFrgnRbQU7qdS1spABUFFHg04QpB9DdboPEjgKLTFfeEAvr6Y7qll3/nGgqywDQdk1tYshqKO0p1WQ==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/cosmokit": "^1.8.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "node-addon-require-builtin": "^0.1.4"
      },
      "peerDependenciesMeta": {
        "node-addon-require-builtin": {
          "optional": true
        }
      }
    },
    "node_modules/@deepseek-ai/cordis-plugin-timer": {
      "version": "1.1.4",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/cordis-plugin-timer/-/cordis-plugin-timer-1.1.4.tgz",
      "integrity": "sha512-GhxOiU+TF2BbNBKlV2N24zvfv2Kh7RxkFW1hjML6s+DOb+y2zomzbeC5zHIIjsc/rpRwbKpDNucPGSwkH0ChXw==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/cosmokit": "^1.8.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/cosmokit": {
      "version": "1.8.3",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/cosmokit/-/cosmokit-1.8.3.tgz",
      "integrity": "sha512-qBo+ronVM6Eu2WNVJXi8JcMiqZ19T9BRIpV+5qJUFPXjGH/Z0QKcQMC/IZJ7L394YTOtJgcovbk9qP0w2GsBXQ==",
      "license": "MIT"
    },
    "node_modules/@deepseek-ai/dsh": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh/-/dsh-0.1.5-rc.2.tgz",
      "integrity": "sha512-8Xc8hCQHcIWRmTCVU/xZdp6/qMsWMeAd2ObChKDEsfhUPJFXx6H0lgeb1DxUMD86HZrrVN+1bCvn1ppjZ/fOxw==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/cordis-plugin-hmr": "^1.0.17",
        "@deepseek-ai/cordis-plugin-include": "^1.0.7",
        "@deepseek-ai/cordis-plugin-loader": "^1.0.3",
        "@deepseek-ai/cordis-plugin-timer": "^1.1.4",
        "@deepseek-ai/dsh-acp-app": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-agent-instructions": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-agent-tool-presentation": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-app-boot": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-base": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-agent-preset": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-cordis": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-cmdline": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-command-compact": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-command-goal": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-compaction-basic": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-compaction-tool-result-pruner": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-cordis-client-runner": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-fs-local": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-goal": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-goal-round-driver": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-headless": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-home-paths": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-hooks-claude-code": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-hooks-codex": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-http-proxy": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-jobs-local": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-launch-environment": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-mcp-client": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-persona": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-plan-mode": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-pwsh-local": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-pwsh-sandbox": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-schedule": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sdk-app": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sdk-minimal": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-reference": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-skill": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-skill-filesystem": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-terminal": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-terminal-bash": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-time-context": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tmux-context": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-token-meter": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-ask-user": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-bash": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-bash-persistent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-cordis": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-fs": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-fs-search": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-goal": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-jobs": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-present": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-pwsh": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-pwsh-persistent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-ralph": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-skill": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-str-replace-editor": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-subagent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-subagent-control": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-todo": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-web": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-workflow": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-web-app": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-webhook": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-webhook-github": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-workflow-worker-thread": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2",
        "commander": "^15.0.0",
        "js-yaml": "^4.2.0",
        "node-addon-require-builtin": "^0.1.4"
      },
      "bin": {
        "dsh": "lib/bin.js"
      }
    },
    "node_modules/@deepseek-ai/dsh-acp": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-acp/-/dsh-acp-0.1.5-rc.2.tgz",
      "integrity": "sha512-0lmDF2qyy136fTFN7jowBVBRVQ00YASi1umgnFSVpHQ0rDKh32tpl7xWaoOTlh6mmKh04A93qt92QhPLIzkcSg==",
      "license": "MIT",
      "dependencies": {
        "@agentclientprotocol/sdk": "1.4.0",
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-attachment": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-mcp-client": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-persistence": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-token-meter": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-user-approval": "^0.1.5-rc.2"
      },
      "peerDependenciesMeta": {
        "@deepseek-ai/dsh-token-meter": {
          "optional": true
        }
      }
    },
    "node_modules/@deepseek-ai/dsh-acp-app": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-acp-app/-/dsh-acp-app-0.1.5-rc.2.tgz",
      "integrity": "sha512-90SRAmF6haHARulVllv6uh88n+zubdrgDfxvTzmi9+p3ktt7o6oZ6+yKO7I7e04bSskT+Pz/1J0FLugi9l39dw==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-acp": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-cmdline": "^0.1.5-rc.2",
        "commander": "^15.0.0"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-agent": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-agent/-/dsh-agent-0.1.5-rc.2.tgz",
      "integrity": "sha512-SlUL1riZmVLwMUR3jo9CP/R1cxov9dHkCJDh6JQW3fSZJVCIdPygBRlAweCUDvHxAEmPpFHxE/U3NmSUbX+vQQ==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-scope": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-system-prompt": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-typert-protocol": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-agent-default-model": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-agent-default-model/-/dsh-agent-default-model-0.1.5-rc.2.tgz",
      "integrity": "sha512-B2e3e1iT+iSE2Pf7bnrKmsbOPiD+ZXplVhL98K35y0IGX7yUdqATsyKVPEHjgfCGooNKjGlSBB42kUk3s7Iv/A==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-settings": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-agent-instructions": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-agent-instructions/-/dsh-agent-instructions-0.1.5-rc.2.tgz",
      "integrity": "sha512-735wmeuRDr/fB9vCnBALEHE7z04Lmhx6WX3KJLE3DjWMGbmTudU5i0iYSYst2mZh7C/igV5wXBveIGKLPIrL+A==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-fs": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-home-paths": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-agent-loop": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-agent-loop/-/dsh-agent-loop-0.1.5-rc.2.tgz",
      "integrity": "sha512-24wvqVlqFmdqJ2Bhcku/vKeNy+qWSmeEeN7lvcUB+WkhcF0/Ra2G3WwcVmnahJA4+w0AKdW3W7v+YYcO/jKJpg==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2",
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-scope": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-persistence": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-settings": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-system-prompt": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-agent-presets": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-agent-presets/-/dsh-agent-presets-0.1.5-rc.2.tgz",
      "integrity": "sha512-h0j7IfuHPYh/oJR7SDxh6pzSgUeaHwaaAIg5X0RNL6Z2oQrq0oUmjtArCVVw7nkgt48DjdBwx4EwJ9IdNrHxug==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2",
        "js-yaml": "^4.1.0",
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/cordis-plugin-include": "^1.0.7",
        "@deepseek-ai/cordis-plugin-loader": "^1.0.3",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-atomic-write": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-home-paths": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-scope": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-settings": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-system-prompt": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-typert-protocol": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-agent-tool-presentation": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-agent-tool-presentation/-/dsh-agent-tool-presentation-0.1.5-rc.2.tgz",
      "integrity": "sha512-XA4Z3zm73SQw4VmWdo2Z/HdRr3YCWL7SseO70wIuYbcjfar1BdMC0S6/KXTgvshn1lCuCqf7I47B1e5oxoHKhg==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-anonymous-user-id": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-anonymous-user-id/-/dsh-anonymous-user-id-0.1.5-rc.2.tgz",
      "integrity": "sha512-YIpZoi8mY/d+BjnPOryDYR7lkwr2uWh5KJWjIqm2sGN+C6imc66lDiSi5wy91JMRzefFWJf26fCiYXAF7ErQAw==",
      "license": "MIT",
      "peer": true,
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-home-paths": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-api-gateway": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-api-gateway/-/dsh-api-gateway-0.1.5-rc.2.tgz",
      "integrity": "sha512-jYDsxVyRnRI++f+hXqBJemOk6ASTvly1sO8ZRuoI77RR22r0nfTFiifxevbmUcavl6j9fgB/LL3X7BpXMy5QQg==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-deque": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-timeout": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-typert-protocol": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2",
        "ws": "^8.21.0"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-api-remotes": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-api-remotes/-/dsh-api-remotes-0.1.5-rc.2.tgz",
      "integrity": "sha512-O8t/aJiHatgzlQIgXx6xdmpy+iAXJ/Zab1xMHIx7Noo6mzSYUIxwfJkoYgPv9G0Yfx06FPoRTUtLiZ8IADqUOA==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-deque": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-scope": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-api-session-controller": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-api-session-controller/-/dsh-api-session-controller-0.1.5-rc.2.tgz",
      "integrity": "sha512-rwOxS6piZ9EuLgE/pbq4cUlRCGv98IKgdFnK5v8WNwZYKug3rUpvYODDU9vE42ydmClgjFd/KIAaEDFPatcDpA==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-deque": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2",
        "mime-types": "^3.0.2",
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-agent-default-model": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-agent-presets": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-api-gateway": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-attachment": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-connection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-file-upload": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-commands": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-file-reference": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-fs": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-jobs": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-native-command": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-scope": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-persistence": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection-cache": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-query": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-title": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-skill": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-subagent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-typert-protocol": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-typert-registry": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-util-time": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-util-workspace-path": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-workspace": "^0.1.5-rc.2"
      },
      "peerDependenciesMeta": {
        "@deepseek-ai/dsh-jobs": {
          "optional": true
        },
        "@deepseek-ai/dsh-session-persistence": {
          "optional": true
        },
        "@deepseek-ai/dsh-session-projection-cache": {
          "optional": true
        }
      }
    },
    "node_modules/@deepseek-ai/dsh-api-settings-controller": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-api-settings-controller/-/dsh-api-settings-controller-0.1.5-rc.2.tgz",
      "integrity": "sha512-HcnffXfBLi8Xaq3i8DZDDMZHkekW3RjRvA1XqnG/1SLR2w9OEt5P3oY2VzO0UuhyBlYCjS61BwGxDa5zWLCPlw==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2",
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent-presets": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-credentials": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-native-command": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-settings": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-typert-protocol": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-api-workspace-controller": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-api-workspace-controller/-/dsh-api-workspace-controller-0.1.5-rc.2.tgz",
      "integrity": "sha512-95USICv+Ds+BS4Bjf27gVneXc/ZuhlYvTsdfjx/S+MMSutjgV2eZDKkQOxVn8u8KMZw3FG0X5Era47qa3kg01Q==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-deque": "^0.1.5-rc.2",
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-api-gateway": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-connection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-host-directory-picker": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-storage-domain": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-typert-protocol": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-workspace": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-api-workspace-files": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-api-workspace-files/-/dsh-api-workspace-files-0.1.5-rc.2.tgz",
      "integrity": "sha512-Kx4ksBWQhN2Sbj48TPwC9cM0cxltIKzjyzanYog0d1WRnzxtqPDHthi6fx1I6RYA+k6257/xLi/Vx/ThYJ1ppQ==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-deque": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-typert-protocol": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2",
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-app-boot": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-app-boot/-/dsh-app-boot-0.1.5-rc.2.tgz",
      "integrity": "sha512-beM+ULhjr2mGoyrWta3F6HOQ9OD+i5tKG8BU0RtgVGmXDjA57cVEAiEkkgPLMTxbbzU+xsFP61I5SApna4Cz5A==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-atomic-write": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-package-manifest": "^0.1.5-rc.2",
        "js-yaml": "^4.2.0",
        "resolve.exports": "^2.0.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/cordis-plugin-group": "^1.0.2",
        "@deepseek-ai/cordis-plugin-hmr": "^1.0.17",
        "@deepseek-ai/cordis-plugin-include": "^1.0.7",
        "@deepseek-ai/cordis-plugin-loader": "^1.0.3",
        "@deepseek-ai/dsh-home-paths": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-launch-environment": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-system-prompt": "^0.1.5-rc.2"
      },
      "peerDependenciesMeta": {
        "@deepseek-ai/cordis-plugin-hmr": {
          "optional": true
        }
      }
    },
    "node_modules/@deepseek-ai/dsh-atomic-write": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-atomic-write/-/dsh-atomic-write-0.1.5-rc.2.tgz",
      "integrity": "sha512-9bCOLkug83IGuoEBPHUxgfJ/IxsHg/UigGi8Oj0agibcGVq2WdL/fbU8KA3KU5H49KDOxq2/AA3XUI3RiAEtYA==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-attachment": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-attachment/-/dsh-attachment-0.1.5-rc.2.tgz",
      "integrity": "sha512-S6b8/WjqzGw+dMDLRXnq+tbijDGkQh38yE+zpQytX2/w/mPR3VzGj5r6McS01WwD76vXR8WFoheSCLyCAro8WQ==",
      "license": "MIT",
      "peer": true,
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-attachment-local": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-attachment-local/-/dsh-attachment-local-0.1.5-rc.2.tgz",
      "integrity": "sha512-UN9Zw6oBqHl/SQxFQCGUp8OcARQkPzZo4BGNkSQHqSsEo3d59Mv7H5mSFHmKl4NogSMwksjN3SF0FocN7xyF2A==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2",
        "sharp": "^0.35.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-attachment": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-home-paths": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-authorization": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-authorization/-/dsh-authorization-0.1.5-rc.2.tgz",
      "integrity": "sha512-1AtUlo72eYUwiiYL728E4oLgGfP3Kq1yiftiKv0hZ9AYzwZRMIGxv1659UymViNQ/m3BIEPlfiv+vWIDHkqMgQ==",
      "license": "MIT",
      "peer": true,
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-credentials": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-base": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-base/-/dsh-base-0.1.5-rc.2.tgz",
      "integrity": "sha512-a4QqqqnN/qmWmIvFdCsXXlPerI3IhyIXh0UdLX1Wo1Q6P+blCIkQDG4LLLNS4RoWjhDURoKm2t51NBjotD6BVg==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/cordis-plugin-hmr": "^1.0.17",
        "@deepseek-ai/cordis-plugin-timer": "^1.1.4",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-agent-default-model": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-agent-instructions": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-agent-loop": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-api-gateway": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-attachment-local": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-bash-sandbox": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-command-compact": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-command-feedback": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-command-goal": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-commands": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-compaction-basic": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-compaction-tool-result-pruner": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-credentials-local": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-deepseek-llm-api-extensions": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-fs-local": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-fs-observation-policy": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-fs-sandbox": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-goal": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-goal-round-driver": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-jobs-local": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm-deepseek": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm-pi-ai": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm-retry": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-permission-presets": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-plan-mode": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-plugin-package-inventory-deepseek": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-pwsh-sandbox": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-repeat-tool-reminder": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sandbox-local": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sandbox-policy": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-checkpoint-policy": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-log-deepseek": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-persistence-jsonl": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection-cache": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-query-sqlite": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-telemetry-otel": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-title": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-title-first-prompt-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-settings-file": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-shell-env": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-skill": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-skill-badge": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-skill-filesystem": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-spill-local": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-spill-policy": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-storage": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-storage-domain": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-storage-json": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-subagent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-subagent-fork-in-process": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-subagent-spawn-in-process": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-subprocess-local": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-system-prompt": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-token-meter": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-bash": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-call-timeout-policy": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-fs": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-fs-search": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-goal": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-jobs": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-present": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-pwsh": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-ralph": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-skill": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-subagent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-subagent-control": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-todo": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-web": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-workflow": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-typert-loader": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-typert-registry": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-user-approval": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-user-questions": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-web": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-web-fetch-http": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-web-search-deepseek": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-workflow-worker-thread": "^0.1.5-rc.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-bash-local": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-bash-local/-/dsh-bash-local-0.1.5-rc.2.tgz",
      "integrity": "sha512-a/FTpYUKY0Sorn4GlY2HjuQh7+gjU8XQ2+HRDN0tuqMQyO1RIzJi62FjtZSZ6+qV4EJJ8o8h0+8URHsVrzro+Q==",
      "license": "MIT",
      "peer": true,
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-settings": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-shell": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-subprocess": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-timeout": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-bash-sandbox": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-bash-sandbox/-/dsh-bash-sandbox-0.1.5-rc.2.tgz",
      "integrity": "sha512-y8vwK6jf4gPq8mG71zWure803NjfqJ66Nvzr1FTB8An+IF68hT2eU+hHJCByvP0u6FuiLPRptuX/im2CxcNo1g==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-bash-local": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sandbox": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sandbox-policy": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-shell": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-brand": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-brand/-/dsh-brand-0.1.5-rc.2.tgz",
      "integrity": "sha512-/+3TzQRYT4M8NINZ9OrsL5VWNyQSXof8mFLx3txiosWA7Bt0H6UcxUdKGXVwj6gCXhsQ74mzEHm2ucsflG0mYA==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-chunked-list": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-chunked-list/-/dsh-chunked-list-0.1.5-rc.2.tgz",
      "integrity": "sha512-XPr311bleMyOh9SzhTMmc1mC6qx2Tty8315MNYYyzIOLAns0h27PReFa9axy8I0soBaE7gfp6eQI1ZriVkI68g==",
      "license": "MIT",
      "dependencies": {
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-connection": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-connection/-/dsh-client-connection-0.1.5-rc.2.tgz",
      "integrity": "sha512-W0GAZX01hAfrfjoZbtwEJ5ik3dG20Hy/0TFYJ6dnvwxtcacGYNIwi3f2oHpZgLcEw+PWwq1Y9IV1iKG+yDOytg==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-credentials": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2",
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-file-upload": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-file-upload/-/dsh-client-file-upload-0.1.5-rc.2.tgz",
      "integrity": "sha512-Y0v4pr1btn1fY8ox1cxIuxDhm2RKHrztfIDMhzj2Z1xRCdSdfbZYtkYvOdWS4uAbpZWQKs5gDMGgASvwTCJ+lg==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-typert-protocol": "^0.1.5-rc.2",
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-scope": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-hmr": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-hmr/-/dsh-client-hmr-0.1.5-rc.2.tgz",
      "integrity": "sha512-Ra1JziRbpV5CiIaYUT4FbBKko1m+PONCGJjmKM7PyHSAv00v53/xKREHVB1SNqu/QT9cGs688ID2KHuwFxbHSA==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-locale": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-locale/-/dsh-client-locale-0.1.5-rc.2.tgz",
      "integrity": "sha512-kv56ki/WQWagsHt94wJAPzsiKnxa8KlmM33bZGgl2SgF4atlQ3iRCdkwJ3fQARXF4Ocnfutivoj5fu5usks1Sw==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-modules": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-modules/-/dsh-client-modules-0.1.5-rc.2.tgz",
      "integrity": "sha512-034DxLlGvX4GgkqqFN2SGcQx8hKdicj5IrgENclBXXbyMDlpF9xADiWi5DxPms+iOBcL/5LLe84q/GCTWOrA2g==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-resources": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-resources/-/dsh-client-resources-0.1.5-rc.2.tgz",
      "integrity": "sha512-fl0saN8LKcAZ/YUSqxBeAbzVj6ZSBWnWrKGFUqhuq5N0XHWu4CpswFIruKxkvgl7lwAsdTtbL3wMtsKIupSmNQ==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-agent-preset": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-agent-preset/-/dsh-client-ui-agent-preset-0.1.5-rc.2.tgz",
      "integrity": "sha512-BMhRU54q7/8E/JoksXm9TW1rCBBg2V64XsDwltYWmCLcZuLCVBc3GX7t/11WMERNi8Do+0ZM6khslwrhbT5QLw==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-approval": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-approval/-/dsh-client-ui-approval-0.1.5-rc.2.tgz",
      "integrity": "sha512-jrlEBEYfYq04HuKYBxD1yI5Z9btJ4fnuWbz3xtK8M/gvsTye8PrhCMxaNARMznGiUFoXmwv9CRhl/KAqbbfE/g==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-attachment": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-attachment/-/dsh-client-ui-attachment-0.1.5-rc.2.tgz",
      "integrity": "sha512-+i98BXfTpz9HX0G7m7k9ka1R5Tekd0tBIljFtKMmiXWju5MW1u+dngpmGCwoSHenEuZGZDC8K/qgw7MIOcbRnw==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-brand-official": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-brand-official/-/dsh-client-ui-brand-official-0.1.5-rc.2.tgz",
      "integrity": "sha512-VESOQ7N8MmUNyy5H1A099Zx3NW1DTTcFIdN/zYVTXWKukmSR6xanShJMai6e69ibDmUfh0YmRgwOvHOVd2K0Uw==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-chat": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-chat/-/dsh-client-ui-chat-0.1.5-rc.2.tgz",
      "integrity": "sha512-ywvtoJUgd/vkfETuFDIgqrvdR5yzfvBDqpuyw0pFDHaHwLzRtk/xPjraI9blNoLWQ8x/Sp42aS4Pr7XQPfjIvQ==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-commands": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-commands/-/dsh-client-ui-commands-0.1.5-rc.2.tgz",
      "integrity": "sha512-wXSTOM4DHogdXZp8tp5RhEvpRJbNey/lN7yjCNN5Qg+BjSu6BAVwKLRKiriCmKIKJcEEpq4BFQb9Di4c1GMm1w==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-conversation": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-conversation/-/dsh-client-ui-conversation-0.1.5-rc.2.tgz",
      "integrity": "sha512-VnZ0VrmI7+1JH/iMYV6+FaxCsZrVk4CZIuG+k1HkeevHgUPHe3ghLo8u1Qt9cx00MEH5i3bLOY5Nka3w4mbA6g==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-cordis": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-cordis/-/dsh-client-ui-cordis-0.1.5-rc.2.tgz",
      "integrity": "sha512-wLolxQURdqIXNLcoGmiXb6tE7dPD5v5eBBu7WtUn/w7FNlp4QambxpCfX/ytGy8LtKO42LwhYk9uPPr51COorQ==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-deliverables": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-deliverables/-/dsh-client-ui-deliverables-0.1.5-rc.2.tgz",
      "integrity": "sha512-RO2XqjCOZfCSkMzlZf1rgg1ZYhLO6Uy2FUsTwkPContjKCGnGWUMtcaRS8LTs1lYgH8+hP50LIf67o/6z8FJxA==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-typert-protocol": "^0.1.5-rc.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-directory-picker-browse": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-directory-picker-browse/-/dsh-client-ui-directory-picker-browse-0.1.5-rc.2.tgz",
      "integrity": "sha512-DrrcPERnA50Il/1Roc6T3goodqANi0u+nVtQShw2P3FHBwVqdlfUURMQgZgKdHO4JFv2dGnikwf+XhiE41icWw==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-directory-picker-native": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-directory-picker-native/-/dsh-client-ui-directory-picker-native-0.1.5-rc.2.tgz",
      "integrity": "sha512-bH3VBqhlXcmkEcnPrnRLecDW9uOdvsaTzfZ8zTUh5ZtexwNhBs6Wv3lN6gZNw457jbP4BnQoEar5fq+rIC5RTw==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-goal": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-goal/-/dsh-client-ui-goal-0.1.5-rc.2.tgz",
      "integrity": "sha512-38+k8tQq61nQjHYLwYfrwOoju+Jlg2hB0xWJrMvPfcZlB5Shjtthm50lW362tK6iJ9GMSfOGIPGuC+nooBD/Yw==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-input-trigger": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-input-trigger/-/dsh-client-ui-input-trigger-0.1.5-rc.2.tgz",
      "integrity": "sha512-Am7qlXQX+Kf3IddzoJDy2/kgtHm3R5hcUFjDQ8tespSWXswPEX+9wsPFugUvjHJkqdpfeE9OTI3jnd8K6sT/BA==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-jobs": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-jobs/-/dsh-client-ui-jobs-0.1.5-rc.2.tgz",
      "integrity": "sha512-/ygltqocItoM+Jh+tnyYJwIpAPTSu6bAWfEkyBunHpHllCjONuFmnpPcLPZTOxKwkjTdQT6XTRdMhmxU7Sageg==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-layout": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-layout/-/dsh-client-ui-layout-0.1.5-rc.2.tgz",
      "integrity": "sha512-N5+kH1W6UjzOuagJEKDenm/Gbs8Y2sz6btdmVKW5dypEEUgVWsQGV0auTC37gsWKWDaWGpfYOpeAa3Ybk6DYbQ==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-message-feedback": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-message-feedback/-/dsh-client-ui-message-feedback-0.1.5-rc.2.tgz",
      "integrity": "sha512-LSp6c87DLMiO4B0JCpp00QEHI6VEKSLWydAx7DBjWTma397HLGLwKoaOT7kHZ8ZSkei970LmdBCUxcwo2Mc4tQ==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-model-selection": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-model-selection/-/dsh-client-ui-model-selection-0.1.5-rc.2.tgz",
      "integrity": "sha512-AlYjgwb40OOe5InaAFsQEu/KZr5+vYTItXMkFhkP5myzo5fOea0H0xdF29UV12BZ0SaRItacs8GE1kbDB9n5Ew==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-open-in-app": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-open-in-app/-/dsh-client-ui-open-in-app-0.1.5-rc.2.tgz",
      "integrity": "sha512-UqLrgiNa48kTGJuMHo/f1er8ti3vcktkseUgFVW2WfWiV3ES1CQTyKF3+4gH3ub7RYFN8VteTQ5QKuOZe5WgPg==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-permission-presets": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-permission-presets/-/dsh-client-ui-permission-presets-0.1.5-rc.2.tgz",
      "integrity": "sha512-2iDWBqr8bfhvzuxNW03JptdPtqQs6kHjh4GdBfgudUhFszaJZOBETx83+PfazYqV4daT4N0cE696l8tqqsS5Ow==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-plan": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-plan/-/dsh-client-ui-plan-0.1.5-rc.2.tgz",
      "integrity": "sha512-3lbKHZA+9gPSXlMpEFA9xVbn7FOTCgP78Tf3buAjTmOnwSF3PRwBQoMeaANro2SoTn4ZuJZjJkpxqe3YDEOc/w==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-reference": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-reference/-/dsh-client-ui-reference-0.1.5-rc.2.tgz",
      "integrity": "sha512-9zNSMWIhG0txbpwbUdHfkPTHYR44rBVl2aKPIcqr8JSUhI+8y8+QZwoykYTnbZXCKo2Ircdm5IXYbMXTbrGCGQ==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-renderer": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-renderer/-/dsh-client-ui-renderer-0.1.5-rc.2.tgz",
      "integrity": "sha512-otUJ72f1UfL8b/UL+tMdSE+FHqJWErPmHs2AWbad9nHXghNoZjL2bxumOhI3b+Hb1eezhhM+KanlWzIulTIkvw==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-schedule": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-schedule/-/dsh-client-ui-schedule-0.1.5-rc.2.tgz",
      "integrity": "sha512-DjBRHytIkcntXs93cpDSiDCiA6eEegc26ht+ZuE9sdCHNh3eu2GxLihMoKBiVFbepwIsdfuWO2mf40iFH/w82A==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-session": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-session/-/dsh-client-ui-session-0.1.5-rc.2.tgz",
      "integrity": "sha512-EGCG4Ik95obEM1MA3ZVSsPuK7nknQyhfV/qgNg035jn6gtZJxAguuck9qBNuSOC0upx1xTbh+gSJ6bAmS4ugcQ==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-settings": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-settings/-/dsh-client-ui-settings-0.1.5-rc.2.tgz",
      "integrity": "sha512-NsnZLRI2ZDzJJylx9KATuDwrTdJ0FSP93dU5SL5k2G2W4FLUspNvlAmk7bha5Miopfzmfd+h5/NvMdWe8PVVjQ==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-settings-general": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-settings-general/-/dsh-client-ui-settings-general-0.1.5-rc.2.tgz",
      "integrity": "sha512-Y/UNa3iwrqLesUVSPLts1xZotenWyybnpU0/oABUt6rkvhDuxVz2w8HltnrVD3ytKzPBSrmlK2vP2YyUaK4z2g==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-settings-models": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-settings-models/-/dsh-client-ui-settings-models-0.1.5-rc.2.tgz",
      "integrity": "sha512-L1vxfbVZEDmGlVuEaJcQb60E36l7BMZoICLJf+D7r8Gwcy8kFckGWKS0fbxsz+3Jvm2L0+lINJESzSMC9pMGNA==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-settings-plugin-inventory": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-settings-plugin-inventory/-/dsh-client-ui-settings-plugin-inventory-0.1.5-rc.2.tgz",
      "integrity": "sha512-lpa6KZA/wqYRNi0zHQVrdUEbt4UGHKfughUSpew4sWvd4HymGGyuKF1ksVrioRUdRGAM+f8/VYpXd0ySk4wQ5A==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-settings-plugins": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-settings-plugins/-/dsh-client-ui-settings-plugins-0.1.5-rc.2.tgz",
      "integrity": "sha512-MRErIAbG1GjrIDN/ULFEMrBA5rHUfg55+uY2P51UVmLdI5UC3B6I5QQYNo9pPcqHSVnT8EZFz7ZWz4Yi5DTAuQ==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-sidebar": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-sidebar/-/dsh-client-ui-sidebar-0.1.5-rc.2.tgz",
      "integrity": "sha512-52PGlC4e9zkD6MQOjDo2TakxsWi4f5QigmdNsHluhDMBUfwMYl8ppWGD3uuHyAkK95yh0D1eKI5UPdQ5kikn+A==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-sidebar-documentpreview": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-sidebar-documentpreview/-/dsh-client-ui-sidebar-documentpreview-0.1.5-rc.2.tgz",
      "integrity": "sha512-HeaqHwOyCg6/3ppFP7///G8C4vmVrhRBP8R0w31PLWIv9dcpnULKuFLFO7UemeBsi8K7uQ5zwN7V25DoT05X7w==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-sidebar-files": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-sidebar-files/-/dsh-client-ui-sidebar-files-0.1.5-rc.2.tgz",
      "integrity": "sha512-ZJCpQNruk1Sm29wce30+ATjFkvnuhd55yB3wb+9WpeN+hW6jPmBnAmvUGbS4JsWqpimHLL4y8xYUd5AzAe7+sQ==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-sidebar-right": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-sidebar-right/-/dsh-client-ui-sidebar-right-0.1.5-rc.2.tgz",
      "integrity": "sha512-EqKZ5JuyM+yCd6TVe7NwTmZf9y7dyhcLczpnu5rG87RuwVufEz1ARqO9TBXf32ApXCeYOJQGVeJjR/VsYzVMgA==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-skill": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-skill/-/dsh-client-ui-skill-0.1.5-rc.2.tgz",
      "integrity": "sha512-aVBAA4cp79MRE98yVyFjosz8BAn4W5U2DFGF7BxmDYKNmFEp7Ae0THGNn9VXZg5wfmTH6fTAzQKw1THSv+EVWA==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-subagent": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-subagent/-/dsh-client-ui-subagent-0.1.5-rc.2.tgz",
      "integrity": "sha512-/SWymLiSOzYswhikUo+VAuTc4dYkTecw+8vtex0H5IsCsRHlDaTnHgcxXnlvgeqFFifjanr4JzVhwzg9Ope2dA==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-theme": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-theme/-/dsh-client-ui-theme-0.1.5-rc.2.tgz",
      "integrity": "sha512-qLR/6E/AzHUpTgKn8P0TiHiWg69qXYFfbpfHWM+HpQ66K7lyRQUXC+WKEJf2ZStCF4/enkx/rhMdXzRDlPVO3A==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-tool": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-tool/-/dsh-client-ui-tool-0.1.5-rc.2.tgz",
      "integrity": "sha512-ncGDBvf7EpuDkq8KteGqQoUCcYkLWlGWNHPZScF8B92zdftYjlnsiQJZCkFbDiyRju3z7PAV3qdY0WO9hOTs3Q==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-trajectory": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-trajectory/-/dsh-client-ui-trajectory-0.1.5-rc.2.tgz",
      "integrity": "sha512-QiCP1h1bgHG05vlJL5hde3hZPfTuSdwRrszgfY7Jy2NfkuIX6wqv8TI5zsZsEkS2/ha3Ebp3nhjDfkXsGZpgbQ==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-user-questions": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-user-questions/-/dsh-client-ui-user-questions-0.1.5-rc.2.tgz",
      "integrity": "sha512-vJzqgAIAlGX7oth3a4qVq3pyyBRc4paL7ENOnt1kfmY+XoPTGF+/CUXwe/Lep5Xjo+95K2HPq7tMxeryKJj72g==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-workflow-run": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-workflow-run/-/dsh-client-ui-workflow-run-0.1.5-rc.2.tgz",
      "integrity": "sha512-dFpAilctrJhwwZY/XXVvTy73v9UhAPqbuEMIK3sMBFtoAg+S/gCQsiVX/WV5/WCADWdjHLtBoEVAsZgrnImbHQ==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-client-ui-workspace": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-client-ui-workspace/-/dsh-client-ui-workspace-0.1.5-rc.2.tgz",
      "integrity": "sha512-BRe/RDIJJblCECYLwVroy8h4+cXrf3eXSfLjJ6BdpnW2fI3CBJuEKlFaBPGtKAFTTain0rQ+dP6SscZNrXm5Gw==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-cmdline": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-cmdline/-/dsh-cmdline-0.1.5-rc.2.tgz",
      "integrity": "sha512-sfjqgFojZRprFx0oKCI2J1GX2BZ+mgWq8bYX9CktFt3OGlLgmFE5JJ/nNSLKtCjRoXok7KXhsYFcH0dZGlkUsQ==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/cordis-plugin-loader": "^1.0.3"
      }
    },
    "node_modules/@deepseek-ai/dsh-code-runtime": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-code-runtime/-/dsh-code-runtime-0.1.5-rc.2.tgz",
      "integrity": "sha512-qex3bvQNBkv1eapi8iTyNq+c52q1kxfdv5+8Nkswv4/kwrkvcDa4n4iiCbEgKS+K5U4DiVIprnyIv8Nx7il7Nw==",
      "license": "MIT",
      "peer": true,
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-code-runtime-worker-thread": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-code-runtime-worker-thread/-/dsh-code-runtime-worker-thread-0.1.5-rc.2.tgz",
      "integrity": "sha512-eXrkDTWlc/ZH2A6S1bP1An2AgfQjmaMRmCKJU1mUzvENO7sLTo++GFR6L1CRLRkFxB8VGZo1TYFKtBxNKsdbaA==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-code-runtime": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-timeout": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-command-compact": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-command-compact/-/dsh-command-compact-0.1.5-rc.2.tgz",
      "integrity": "sha512-TBvpsgCLLXvnyuVYV+eiAyL/xkl0dG2j1GUVC2mxwPov1jLiKgLqneZClJpo9gT192HAkGIs8a7wXCoID2ryag==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-commands": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-compaction": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-command-feedback": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-command-feedback/-/dsh-command-feedback-0.1.5-rc.2.tgz",
      "integrity": "sha512-7bXJ5IiSzaCbNUffWboAvMXze4N/GTID7O8loSsKBbeAo4CIS6lOWxyx8ZSrcQv/Gnd4onxizN6WqIMxW6i5Qw==",
      "license": "MIT",
      "dependencies": {
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-anonymous-user-id": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-commands": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-typert-protocol": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-command-goal": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-command-goal/-/dsh-command-goal-0.1.5-rc.2.tgz",
      "integrity": "sha512-wNxZKEBC6bAHd92fzKJjyPnYsZJ7yq13p+5wLufNPZrty7y1nlJ8lm7SwxuPZyXYJ9+/5njYlMnZNNZFmitJCA==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-commands": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-goal": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-commands": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-commands/-/dsh-commands-0.1.5-rc.2.tgz",
      "integrity": "sha512-ODc9h2Jig+Lo4XLdxqHpHjSXsBFeUCth6Y/rToor4KVRWQMedUARlq5otPyB1lYHyQh2DonNs2uf7g3mvag57A==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-util-crypto": "^0.1.5-rc.2",
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-attachment": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-scope": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-typert-protocol": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-compaction": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-compaction/-/dsh-compaction-0.1.5-rc.2.tgz",
      "integrity": "sha512-aYCBBcmodNycKjaX9hWChxi2+YYMKlfiIfrNaL9QwtXs0BLvsv2/ttZqGxCebLGv6XdHWozNadC+opFpUCeZAA==",
      "license": "MIT",
      "peer": true,
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-commands": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-compaction-basic": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-compaction-basic/-/dsh-compaction-basic-0.1.5-rc.2.tgz",
      "integrity": "sha512-Tz6aEvPunsp+SXQruXTldxnZIMLPwXL3ZkWWiWnCM+83r1PglVBWABrxjaRkMaIEype/R16OjxodzR4k7GNA1A==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-commands": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-compaction": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-compaction-tool-result-pruner": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-token-meter": "^0.1.5-rc.2"
      },
      "peerDependenciesMeta": {
        "@deepseek-ai/dsh-compaction-tool-result-pruner": {
          "optional": true
        }
      }
    },
    "node_modules/@deepseek-ai/dsh-compaction-tool-result-pruner": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-compaction-tool-result-pruner/-/dsh-compaction-tool-result-pruner-0.1.5-rc.2.tgz",
      "integrity": "sha512-JsCZrbStt2EehHSLXU4CFKJW8idc0xefqCOh101bW4El26JShEKoJKTUwIMvRPq7AQh9RBWk25Pf9N1yAjKuDw==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-compaction": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-token-meter": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-cordis-client-runner": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-cordis-client-runner/-/dsh-cordis-client-runner-0.1.5-rc.2.tgz",
      "integrity": "sha512-RFzivtNX2Zx5wTR+9XnawzoxDViti44QoGEamhLP5+Jc0X8BMPVemYB9755UL3YRqgASNQ2x0IrJgBoc813jAg==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-cordis-host-runner": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-cordis-host-runner/-/dsh-cordis-host-runner-0.1.5-rc.2.tgz",
      "integrity": "sha512-VUvOt3L7D7EJEMO2Cetlv0UMY1haIzI401W5kO1AKgOHktGr5SC+wFk83SHX7v2/CS0g1N3PbQ3D+RslutqPjg==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2",
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-scope": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-typert-protocol": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-credentials": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-credentials/-/dsh-credentials-0.1.5-rc.2.tgz",
      "integrity": "sha512-TfX5MYLlyw0BFERj3dGxVfP9QGcKZUE5eXDSaY7ZfJbfQgK51VfxV5tP899OxIDRD3Bw94K5Rzx+dbUTSWJBtA==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-credentials-local": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-credentials-local/-/dsh-credentials-local-0.1.5-rc.2.tgz",
      "integrity": "sha512-Xaz/giXBtfwWP/jd/o1QwKDCurGjZVfVXEfFZvXfzGJ1pdQiMFdt/1nfCnIkktHO2fHhtxEE4MBg10PBLtmp7A==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2",
        "chokidar": "^4.0.3",
        "yaml": "^2.9.0"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-atomic-write": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-credentials": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-home-paths": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-launch-environment": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-deepseek-llm-api-extensions": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-deepseek-llm-api-extensions/-/dsh-deepseek-llm-api-extensions-0.1.5-rc.2.tgz",
      "integrity": "sha512-zue4FWkj7Srg8mAwA6XNqT560NkraILiYtAwIcpT4mmt1Nlim+m55x7ACNkwA5Es0uCkEFAU1H6wHSnR7l2EfQ==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-deque": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-deque/-/dsh-deque-0.1.5-rc.2.tgz",
      "integrity": "sha512-j1dINwK5XU8f+GPdzx22o2GvsAEhCvnLxqZfEqaLGYImbvdLkToVuU7M2nVcNSU+B7v+kl39lWf3E5u6tzG8RQ==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-file-reference": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-file-reference/-/dsh-file-reference-0.1.5-rc.2.tgz",
      "integrity": "sha512-vJIHFFn3YmipM5X9SgPglnUO4jpvlCh3Yr3JQYITRtpBkL4hBbleayICP8cCmrfN0prstJ8dtIuXkUANu7decw==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-file-reference-local": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-file-reference-local/-/dsh-file-reference-local-0.1.5-rc.2.tgz",
      "integrity": "sha512-WYTbwUD5TFWExRttqeNRPBk0EF6xfngevyQyNMRu3fgWYVrHHW9vtR7iGAs/mxAqCgTwZw4vAhCnw1HxaYYLqw==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-file-reference": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-system-prompt": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-fs": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-fs/-/dsh-fs-0.1.5-rc.2.tgz",
      "integrity": "sha512-6DHTquXPbpYdykGayqYaXSI9t668tDCoswH/bDezV+nwj4LxjrfY/smEtgp7nC4ubyoKf1hmjpNfUonGC9e7aA==",
      "license": "MIT",
      "peer": true,
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sandbox": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-fs-local": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-fs-local/-/dsh-fs-local-0.1.5-rc.2.tgz",
      "integrity": "sha512-akUTz9D/N0ruSOzytZ8SZ330SdzG75fd7DzHsJ7KJzSif/QM0Y+RpOmGMnjlJzit4HDV2A4Etn80wSzstyp82A==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2",
        "koffi": "^3.1.0"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-fs": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-fs-observation-policy": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-fs-observation-policy/-/dsh-fs-observation-policy-0.1.5-rc.2.tgz",
      "integrity": "sha512-AntY5dfkTL8WugNGHkJxCEffaTFHqdXaYm7ZrerexnLiZQscDRRjf9zI00JwOmlFM/IrioCuLf5cERfhZN5GYw==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-fs": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-fs-sandbox": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-fs-sandbox/-/dsh-fs-sandbox-0.1.5-rc.2.tgz",
      "integrity": "sha512-eUxNsnM+TsjGw5OleOIcAhMnFhmQ4OAZoBYeiRMSeOMuCKWEjhxUGN8S8Hg1HxPaZVeIrUV7qFsNQzhehKj7wg==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-fs": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-fs-local": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sandbox": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sandbox-policy": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-goal": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-goal/-/dsh-goal-0.1.5-rc.2.tgz",
      "integrity": "sha512-atFJaoijwAz5yZ119f82I7jMx3tGCwXOz6qoY0Likb2c5DpumWZTJgs5L19OhKbhvEW+r2MAC4MYKaUxtrbb0Q==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2",
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-scope": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-typert-protocol": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-goal-round-driver": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-goal-round-driver/-/dsh-goal-round-driver-0.1.5-rc.2.tgz",
      "integrity": "sha512-9uEhBTqIVxJklNkxCvDDD9upyEa6mIYlB5kfan37zixGDfKSgj83VSwXjJhVqN0b/2CLQDuPxtXgFuSdItoR0g==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-goal": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-headless": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-headless/-/dsh-headless-0.1.5-rc.2.tgz",
      "integrity": "sha512-EJ0QCWo+0WCTq2SFFRZpbMcGwkSh/uBRNJmE6nx9Eh1Qlt4y3I4G3k+p2mn276ujPbGfEadaI5KLciLOHsZQXQ==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-cmdline": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-code-runtime-worker-thread": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2",
        "commander": "^15.0.0"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/cordis-plugin-loader": "^1.0.3",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-agent-default-model": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-home-paths": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-home-paths/-/dsh-home-paths-0.1.5-rc.2.tgz",
      "integrity": "sha512-Ek+DH9+MTiulfWDMKo5r335k4I9XIItk+jq8YaPihpK3klSeiKLW/SpKJhFodRnW1CwEyjIlt2XriycRvceIiw==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-hook-protocol": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-hook-protocol/-/dsh-hook-protocol-0.1.5-rc.2.tgz",
      "integrity": "sha512-qJ0AlRhLsj31tro0vzubhXarigq8EDi3PwyOxxhoSxZUbGqOBWx+QLNEOC5E5tuob6DYA64Qbmgh7Cbz8vYgiA==",
      "license": "MIT",
      "peer": true,
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-shell": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-hooks-claude-code": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-hooks-claude-code/-/dsh-hooks-claude-code-0.1.5-rc.2.tgz",
      "integrity": "sha512-wdsl+A8tvAk66N5ozqyxmJ5t30AQQ5byBaKYL60Vx1jFoWXaD83piPcD1BAPZ+hVoWI1a9PCUDowFzAzq/PnwQ==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-hook-protocol": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-subagent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-hooks-codex": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-hooks-codex/-/dsh-hooks-codex-0.1.5-rc.2.tgz",
      "integrity": "sha512-WVmZIWJJ04A7rpe80wcd4+O4MfJbBJdtiPwM99B2DASpm/Igyz3t0mNzNfbWe3zmznRKTHFz1imqhaPdceY1Ng==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-hook-protocol": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-host-directory-picker": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-host-directory-picker/-/dsh-host-directory-picker-0.1.5-rc.2.tgz",
      "integrity": "sha512-a0fVQTX0tM7Zl+1WWkCO3Pd+64CW1CwfOJrNxpTuz7EOp6wOtsYDRHlDyJUSKAOMJArS4xN2qkkt8n3eIkNkbQ==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-host-directory-picker-auto": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-host-directory-picker-auto/-/dsh-host-directory-picker-auto-0.1.5-rc.2.tgz",
      "integrity": "sha512-R6nl8L/xAd1kmlCZc8xOl4jxp69HMmZ75vWdRswM3MXrM3GIgNiF7AKwK0JlNIKPc6M4vsC3sGMR4yR0XtdhrA==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-launch-environment": "^0.1.5-rc.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/cordis-plugin-loader": "^1.0.3",
        "@deepseek-ai/dsh-client-ui-directory-picker-browse": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-directory-picker-native": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-host-directory-picker-browse": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-host-directory-picker-native": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-host-webserver": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-host-directory-picker-browse": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-host-directory-picker-browse/-/dsh-host-directory-picker-browse-0.1.5-rc.2.tgz",
      "integrity": "sha512-8xnMfNt5XcO81qshrC/zzD2NYfLKYEQjVzcu7r+u/ymro7jLypRAp7DQFGuRZiQV4jIF9bdTTT2gcCYaXkWi5A==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-host-directory-picker": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-host-directory-picker-native": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-host-directory-picker-native/-/dsh-host-directory-picker-native-0.1.5-rc.2.tgz",
      "integrity": "sha512-bBsGBR+tE/w9Dw9rM2Lh/4umjYtGnEmQ9YEss7vrKzo/KFuMfgNPkXHKb6A+AoCfSVd7L7PqkocoA5qVRQND2g==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-host-directory-picker": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-native-command": "^0.1.5-rc.2",
        "koffi": "^3.1.0"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-host-frontend-static": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-host-frontend-static/-/dsh-host-frontend-static-0.1.5-rc.2.tgz",
      "integrity": "sha512-cTc0vTyejoYBE0XWOXw/nEDTcveAF9ucYEcaUVintvoo4b4WGG8fuTZSJX9EVkvXggAvxvFjSiYAQ8o2zP7qrA==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-client-connection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-host-webserver": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-host-open-in-app": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-host-open-in-app/-/dsh-host-open-in-app-0.1.5-rc.2.tgz",
      "integrity": "sha512-8CBzeDQYMmz4IG7xp7a8mp0xeQ9v6FS9qcN2pOdfRBTXmMk8zHroBpiuW+26yEQmP4HsAi1BECgcWQ03eNoOCg==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-launch-environment": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-native-command": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-subprocess": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-host-plugin-inventory": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-host-plugin-inventory/-/dsh-host-plugin-inventory-0.1.5-rc.2.tgz",
      "integrity": "sha512-U7RTRLRs+O18ru6KzUy7LvMk/IgXnkWSalogg0abfA+QU020XZi+UpwZqM567RzC7MtpxijQk8bulGunu0lifw==",
      "license": "MIT",
      "dependencies": {
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/cordis-plugin-loader": "^1.0.3",
        "@deepseek-ai/dsh-agent-presets": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-typert-protocol": "^0.1.5-rc.2"
      },
      "peerDependenciesMeta": {
        "@deepseek-ai/dsh-agent-presets": {
          "optional": true
        }
      }
    },
    "node_modules/@deepseek-ai/dsh-host-webserver": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-host-webserver/-/dsh-host-webserver-0.1.5-rc.2.tgz",
      "integrity": "sha512-lFgGm9wDrHiTBANzsdoWzdfPSjWYuDFwCoNQ4Uko57Fo5XASL2unfRHGm1xZ828rwEYuGwRvJHMOuoP/17VmlA==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2",
        "compression": "^1.8.1",
        "negotiator": "^1.0.0"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-http-proxy": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-http-proxy/-/dsh-http-proxy-0.1.5-rc.2.tgz",
      "integrity": "sha512-dRK8SMoxyY1F1ulzfnslI++9RDvXLGkPxahUwrnSQl8gTL9R69ZEh6P9dtcIVWMbstaYcq4CqWvo2/qUHSYRhA==",
      "license": "MIT",
      "dependencies": {
        "undici": "^8.10.0"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-invariants": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-invariants/-/dsh-invariants-0.1.5-rc.2.tgz",
      "integrity": "sha512-oUxttB2yjAgkk47AiXOCxk9GwnfGvIEGsQfnMD7fukwIdD3KLJObU7+nz63tbFN80rjwMI8rg+p9QBQ10KU5+g==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-jobs": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-jobs/-/dsh-jobs-0.1.5-rc.2.tgz",
      "integrity": "sha512-C3rBEuWhtDBlxMeKykFvSfBwjSPxkLsvKCFq8BrFdjDmZC1lI9GooMjPZkPxXVbogVrcOBaYVtJdOYJ4+rIpQg==",
      "license": "MIT",
      "peer": true,
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-jobs-local": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-jobs-local/-/dsh-jobs-local-0.1.5-rc.2.tgz",
      "integrity": "sha512-PCDLSktONJ+If3QCcPlRskVLXIa8hG/l0pBIgKNllz4mb7CmYlJ5w1H9pWyrFZllMO0Wm94ZS9aP3SnUcJ8R1g==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-jobs": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-scope": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-timeout": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-launch-environment": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-launch-environment/-/dsh-launch-environment-0.1.5-rc.2.tgz",
      "integrity": "sha512-Cr35kPA3W7skJsPLJhUExLDAjVKCmvJgkog0m8EZ+XdK8RlBDsWiH2peRWFtFOjt8QOOeEkupPs1VDpWORIRyw==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-llm": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-llm/-/dsh-llm-0.1.5-rc.2.tgz",
      "integrity": "sha512-Z7BVsBkK24SE4EItQeow8PHms/9GP0DSTi337vTAa/RY7tNg2Snz3INcXUj6CPZfvntQr1in9op9wLI+rfNsqA==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-timeout": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-typert-protocol": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-util-crypto": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2",
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-llm-deepseek": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-llm-deepseek/-/dsh-llm-deepseek-0.1.5-rc.2.tgz",
      "integrity": "sha512-qNRbLsE2ro+AfD7NgJwTukQLG82gIkbAISjDHgv0au7TocfPkNkUWSu20mTaBcuLGXzfSgBVn4oYx5N3ws6KVg==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2",
        "eventsource-parser": "^3.1.0"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-anonymous-user-id": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-atomic-write": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-attachment": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-credentials": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-deepseek-llm-api-extensions": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-fs": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-home-paths": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-launch-environment": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-settings": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-timeout": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-llm-pi-ai": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-llm-pi-ai/-/dsh-llm-pi-ai-0.1.5-rc.2.tgz",
      "integrity": "sha512-56/TRc857HqapiAia0OyWsKtWPaPOfivx1OMzWYsexhTsIvxIOetrj5IMF+tW6geeSm38vfOffn18gOb97NgEg==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2",
        "@earendil-works/pi-ai": "^0.85.1"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-attachment": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-authorization": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-credentials": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-fs": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-launch-environment": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-settings": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-timeout": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-llm-retry": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-llm-retry/-/dsh-llm-retry-0.1.5-rc.2.tgz",
      "integrity": "sha512-6jyxD9tdzePZMdA4BdvrbdnqSkNYv2M1Nac84urkOcc3D8Jc+QYMnXrpeDB4fZRWg/l7FoKXtfUdn4Mj3kY5ow==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2",
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-timeout": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-mcp-client": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-mcp-client/-/dsh-mcp-client-0.1.5-rc.2.tgz",
      "integrity": "sha512-L1uhXptzs63bne3fpe8By30xUu2KzC9bbo8t1RdasnRNxhevNJA/5/Z9ZenuMRh26XZh9THF+DVuudEH8PFIsQ==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2",
        "@modelcontextprotocol/sdk": "^1.12.0",
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-attachment": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-scope": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-subprocess": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-timeout": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-message-feedback": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-message-feedback/-/dsh-message-feedback-0.1.5-rc.2.tgz",
      "integrity": "sha512-oNLajqBpv5bkebQrzByUpNTlQPcCoLoreMKzYN0s6STwtHpTzOqj9vSzwk964TfFAYutcmfQoCHNY7j9jmw6iw==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2",
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-command-feedback": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-persistence": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-typert-protocol": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-native-command": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-native-command/-/dsh-native-command-0.1.5-rc.2.tgz",
      "integrity": "sha512-FFkiS4Izm5VC1l0bHuw3yExNDmqW0/ypSLt+RtH24GG/W5ofpFvwjGclYDUzSDa0MP5uPzhTMN6qgM76BAA40g==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-output-retention": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-output-retention/-/dsh-output-retention-0.1.5-rc.2.tgz",
      "integrity": "sha512-3eOx7EmMkttMcJfF/BT6e8A8b0SzWuLCxU+AdY9jBcU8ffZFspIPtMgJ86yNCMLdoZctbrmeSVwEdweMJWRhLA==",
      "license": "MIT",
      "peer": true,
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-package-manifest": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-package-manifest/-/dsh-package-manifest-0.1.5-rc.2.tgz",
      "integrity": "sha512-PTieT+tXdV8TNK718JZCd9dpVZR52UMYu5i1nXPDPBpoq1fTP7lLSlgHvDEs6SpoJtdFPGNTe8+7KjELyUDDvw==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-permission-presets": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-permission-presets/-/dsh-permission-presets-0.1.5-rc.2.tgz",
      "integrity": "sha512-WeQ3a+mcWfGjv6PLJGvMIgIZNBiR9mCA+wsjFZvxjtuPdu8p8uILxnaodlXhWpSDQ+8oimRO3eBxNVuJdtX1eQ==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2",
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-commands": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sandbox": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sandbox-policy": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-settings": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-shell": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-user-approval": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-persona": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-persona/-/dsh-persona-0.1.5-rc.2.tgz",
      "integrity": "sha512-VYAoj8tmSmwBr8SzDfjoG+t0v4Y/clu7BIWjZB5rnyNXLl5RNiY8kVTttnYRBQE+2jLLXlcR7PBJlYkfig3Kiw==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-system-prompt": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-plan-mode": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-plan-mode/-/dsh-plan-mode-0.1.5-rc.2.tgz",
      "integrity": "sha512-veJCNJ8qL1vTG5F1GBpBa+lmtBVm+P3kgXpHBCJbYGmZKyrmHj3kbkiVgqxDdxTwnOoQiVco1OcBQUIO/ofGxA==",
      "license": "MIT",
      "dependencies": {
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-commands": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-system-prompt": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-user-questions": "^0.1.5-rc.2"
      },
      "peerDependenciesMeta": {
        "@deepseek-ai/dsh-commands": {
          "optional": true
        }
      }
    },
    "node_modules/@deepseek-ai/dsh-plugin-package-inventory-deepseek": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-plugin-package-inventory-deepseek/-/dsh-plugin-package-inventory-deepseek-0.1.5-rc.2.tgz",
      "integrity": "sha512-X/6mHIXFoFj6KSj0NBA2RC1xQaHtKnj1QS703AypNaq9+rxn3su3oFnZeWulLx0Tex4URfSmIo8SD1pkJkgUeg==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/cordis-plugin-loader": "^1.0.3",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-agent-presets": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-deepseek-llm-api-extensions": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2"
      },
      "peerDependenciesMeta": {
        "@deepseek-ai/dsh-agent-presets": {
          "optional": true
        }
      }
    },
    "node_modules/@deepseek-ai/dsh-pwsh-local": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-pwsh-local/-/dsh-pwsh-local-0.1.5-rc.2.tgz",
      "integrity": "sha512-TwH/xwDRtgk3LyQraG4VoaIaItFSuorTTqyJ3nzuyLOsXq0I8nNYQr/wuaoRjkiDyzFTDeEVMCKq6zEEoLLz1Q==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-settings": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-shell": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-subprocess": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-timeout": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-pwsh-sandbox": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-pwsh-sandbox/-/dsh-pwsh-sandbox-0.1.5-rc.2.tgz",
      "integrity": "sha512-AYTAiy8wQwVoHO47e+hnG73GvHdmg+bKy3bFnrPDANCqnRZh9TJIbkxHxtJnfpBnNCcs2fhhCo6yHtGFYl7KHg==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-pwsh-local": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sandbox": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sandbox-policy": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-shell": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-repeat-tool-reminder": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-repeat-tool-reminder/-/dsh-repeat-tool-reminder-0.1.5-rc.2.tgz",
      "integrity": "sha512-ZDK8AJOP/hdpCKLfbKbfwgj/flICKEPdJbtdoYsTR5A+BGfq8RIy0BWux0IczDenzNSNa8FXq3gbnwQGTPAJ1Q==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-sandbox": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-sandbox/-/dsh-sandbox-0.1.5-rc.2.tgz",
      "integrity": "sha512-OTOR6Jj9cey5YkhALG0TBwZ/Z3t986aczH6fLbzoIIegix+gwNaEBOqCWs+exVJ0Z2QuN/ItXzn+xHxW8Y0dcA==",
      "license": "MIT",
      "peer": true,
      "dependencies": {
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-sandbox-local": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-sandbox-local/-/dsh-sandbox-local-0.1.5-rc.2.tgz",
      "integrity": "sha512-qlbg0X7sR9BE90sfHMuOl0t29XG859jJK8xwf5x8yI9Du0zcmviTlQtvlsT7MgDC9OOw8TEDfRTgbm3VDSXCdg==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-sandbox-windows-acl": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2",
        "@deepseek-ai/node-addon-system": "^0.1.2",
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sandbox": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-sandbox-policy": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-sandbox-policy/-/dsh-sandbox-policy-0.1.5-rc.2.tgz",
      "integrity": "sha512-QyQSCyLFxljkvmsVWJG0xUrYTiXr1DVOxHWURf7EHnbmvYZgg2B+nFcI58+IeUB2qbB35B1ClW5NQsKjOgDm2g==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2",
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sandbox": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-system-prompt": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-sandbox-windows-acl": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-sandbox-windows-acl/-/dsh-sandbox-windows-acl-0.1.5-rc.2.tgz",
      "integrity": "sha512-GPig5OcIBSYtE5YwVzXqV+ICG68FMI1Fo+ZgyH4Jc73Qlze6A4uZqtCyH/xsmiZfzOZiOLyFB9+gc8lu4tZv3g==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-win32-process": "^0.1.5-rc.2",
        "koffi": "^3.1.0"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-schedule": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-schedule/-/dsh-schedule-0.1.5-rc.2.tgz",
      "integrity": "sha512-SjVt6miSlSX1WQdAyvLf0KyQULIds+bLihr3nIvUAhrVSmoRDzbu2jNrNA5jGCo923+dgjGfLn6iUT6aNDJO3w==",
      "license": "MIT",
      "dependencies": {
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-persistence": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-scope": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-scope/-/dsh-scope-0.1.5-rc.2.tgz",
      "integrity": "sha512-JwItISje52iVIjlGMNayRVMZrvZU/0i3Gr0s+coND5nBp5lS+pOj9Zf1RQzA5cWjHYCWKmDuQc+m6zGlwmtAeQ==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-sdk-app": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-sdk-app/-/dsh-sdk-app-0.1.5-rc.2.tgz",
      "integrity": "sha512-WamfPr7CctpoICsT32UpnLlAZavaEno2zwa3icqqpGdSHY0H+BL8yx/4mmRROIX1/ovMyxTcC5urjptDKy34KQ==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-cmdline": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sdk-jsonrpc-server": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2",
        "commander": "^15.0.0"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-sdk-jsonrpc-server": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-sdk-jsonrpc-server/-/dsh-sdk-jsonrpc-server-0.1.5-rc.2.tgz",
      "integrity": "sha512-TRVkUngQPKlpHvqS5AZi63zwa+5eG0zp1NwZXzJjdiYhjV2LPICiWVM6EuSbmIRB6tUZMB1MXf4ZSj2OFQWn/Q==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-attachment": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm-deepseek": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-scope": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sdk-protocol": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-subagent": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-sdk-minimal": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-sdk-minimal/-/dsh-sdk-minimal-0.1.5-rc.2.tgz",
      "integrity": "sha512-mWdnFnz+MGU1+xgg4GExF+OkkYxEdaNQdAPFq30m0oP67cWQV0Om+pNBHDeGeguf8MVGLNLJB+iRP3/63LQV/Q==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/cordis-plugin-timer": "^1.1.4",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-agent-loop": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-deepseek-llm-api-extensions": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-jobs-local": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm-deepseek": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm-retry": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-plugin-package-inventory-deepseek": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sandbox-local": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sandbox-policy": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-scope": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sdk-app": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sdk-jsonrpc-server": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-log-deepseek": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-persistence-jsonl": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-title": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-subprocess-local": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-system-prompt": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-terminal": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-terminal-bash": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-bash-persistent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-pwsh-persistent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-sdk-protocol": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-sdk-protocol/-/dsh-sdk-protocol-0.1.5-rc.2.tgz",
      "integrity": "sha512-ewWYU+2tkcm5rTQtM5F6pYP5S6ox/7j/Gc/xCpXBtVOfUNchI8IUYbVNuea/lGX6XYGCBxpdG1jxV51W1YyWkg==",
      "license": "MIT",
      "peer": true,
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-subagent": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-session": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-session/-/dsh-session-0.1.5-rc.2.tgz",
      "integrity": "sha512-y+klWiGAWR4m4cc4ylurA0cW63673B4N8cr2ANMimweDZAfxL4XVBC7WiD/5DT2DtIhYmVZhz/niyS/WbniUTA==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-scope": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-session-checkpoint-policy": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-session-checkpoint-policy/-/dsh-session-checkpoint-policy-0.1.5-rc.2.tgz",
      "integrity": "sha512-RDaGTq4c49nFfVNq0AiL71z07OhtxoxfXESWppyegxotcdZTpdJTk4wIUto56SuaXi+kNCAP2PEjHC+VM0Z30A==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-persistence": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-session-format": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-session-format/-/dsh-session-format-0.1.5-rc.2.tgz",
      "integrity": "sha512-Bih+d4wQ8ea+sZ1+Hws+ZBG3XpExrSQAtNMJRueFVzfTALhbnzpS5uLYw9qDapdfYYyHURlotQPRImBEpIVGFQ==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-session-format-catalog": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-session-format-catalog/-/dsh-session-format-catalog-0.1.5-rc.2.tgz",
      "integrity": "sha512-UKKVXS551VuTA5hH0cz9s9GjBb8HKdQcM+8Xa4uiviavTy4SOkVO3zmo2Wx43jzgdmChtNiUhc+w9RDOZLc7gQ==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-session-format": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-format-v0-to-v1": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-format-v1-to-v2": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-format-v2-to-v3": "^0.1.5-rc.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-session-format-v0-to-v1": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-session-format-v0-to-v1/-/dsh-session-format-v0-to-v1-0.1.5-rc.2.tgz",
      "integrity": "sha512-0ff7Rl5JsUHGv223XMRFv0adfy7nV2mUQSRf3N1MZviblnp2c65dArq+ieRGISw6JQL4FAz4LSt6A6zV/D6o0A==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-session-format": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-session-format-v1-to-v2": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-session-format-v1-to-v2/-/dsh-session-format-v1-to-v2-0.1.5-rc.2.tgz",
      "integrity": "sha512-VI15vcRCFfzmPpUg+BXSqgB9Kd99sVVXbfxGXo/gYRgjCY3JqClRh1CYhj1BDUvL91l7LsLv9Oa63kYXXbCaIw==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-format": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-format-v0-to-v1": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-session-format-v2-to-v3": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-session-format-v2-to-v3/-/dsh-session-format-v2-to-v3-0.1.5-rc.2.tgz",
      "integrity": "sha512-BO9N4O3HhBrUhyO1ucuk1xa5r1+wYCUPnMTryIrasVnOHI3ndb9ITnPhTEc2m6OXJa3c+wuEWrxc7DJHM1Cx6g==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-session-format": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-format-v0-to-v1": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-format-v1-to-v2": "^0.1.5-rc.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-session-log-deepseek": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-session-log-deepseek/-/dsh-session-log-deepseek-0.1.5-rc.2.tgz",
      "integrity": "sha512-d/PJ5/vsJcbv6ZOOD3pBXH58rBWhhSmWHlfLqH7wDq2wA25e1va290OVKv1wwR05/mZBBQZZXxc7qwT4xnsiyg==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-deepseek-llm-api-extensions": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-session-log-export": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-session-log-export/-/dsh-session-log-export-0.1.5-rc.2.tgz",
      "integrity": "sha512-eoJWiv/oZQA2S61PpDpkU6kovoN5rQ1ZvOmp3HfKLKqfWzi6La1YP5VyiXi/g0qFjZ3ttZC0N+ckmWjNcGUjzw==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-format": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2",
        "fflate": "^0.8.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-persistence": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-session-persistence": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-session-persistence/-/dsh-session-persistence-0.1.5-rc.2.tgz",
      "integrity": "sha512-0nDeM+H+3YR0CH/IVlhjNL9bDx2G4QbliaLekVxY0jdZHM7RqdN/fWPKjeCKbXcRq6qdn9QcmvXU3uWZdErlIw==",
      "license": "MIT",
      "peer": true,
      "dependencies": {
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-timeout": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-session-persistence-jsonl": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-session-persistence-jsonl/-/dsh-session-persistence-jsonl-0.1.5-rc.2.tgz",
      "integrity": "sha512-nkIXdb5oOV6a05Of+3Wywe3gEB77uQOH1U0M+/SRVziSLX4JkE+us7dbWczOOloZjuUsEV7Il8kZ3R9JLsGqjA==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-format": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-format-catalog": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-format-v2-to-v3": "^0.1.5-rc.2",
        "@deepseek-ai/node-addon-system": "^0.1.2",
        "@deepseek-ai/schemastery": "^3.18.2",
        "koffi": "^3.1.0"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-persistence": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-session-projection": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-session-projection/-/dsh-session-projection-0.1.5-rc.2.tgz",
      "integrity": "sha512-WMXGBdbxD1FO1Eu7Qs2FKDXq/kl+z+5b7zGLHdra66eEnShSnQmdF9bXSFQu1sJwba2yF7ueMcBJpWy76gtgQQ==",
      "license": "MIT",
      "dependencies": {
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-session-projection-cache": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-session-projection-cache/-/dsh-session-projection-cache-0.1.5-rc.2.tgz",
      "integrity": "sha512-dS4GGlgZBBHv0U7C4+pQssApxao8Jbt3tMSGQeRU3xpmR5wP8PucR20wTiFThNrsUjdIs2Q/2IKw5bcnkaHwCw==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2",
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-storage-domain": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-session-query": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-session-query/-/dsh-session-query-0.1.5-rc.2.tgz",
      "integrity": "sha512-Ntv/yYwhRhhXO3UkRKc1RIIk4RMLVR6jR8mdphRILEZoxPGjZxMJR95RuZg8TuLERbmGH8AkPAEKD6HWHqWh1g==",
      "license": "MIT",
      "peer": true,
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-persistence": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection-cache": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-title": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-todo": "^0.1.5-rc.2"
      },
      "peerDependenciesMeta": {
        "@deepseek-ai/dsh-session-persistence": {
          "optional": true
        },
        "@deepseek-ai/dsh-session-projection": {
          "optional": true
        },
        "@deepseek-ai/dsh-session-projection-cache": {
          "optional": true
        }
      }
    },
    "node_modules/@deepseek-ai/dsh-session-query-sqlite": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-session-query-sqlite/-/dsh-session-query-sqlite-0.1.5-rc.2.tgz",
      "integrity": "sha512-7xqyRvcaXfPzK6T96QFTdfEVbYIOz69qcghBoDMwSo66MQ+3DSdH3yTPlvrf3a8Nd9Jek375Upz1h12//FB5Ew==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-persistence": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-query": "^0.1.5-rc.2"
      },
      "peerDependenciesMeta": {
        "@deepseek-ai/dsh-session-persistence": {
          "optional": true
        }
      }
    },
    "node_modules/@deepseek-ai/dsh-session-reference": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-session-reference/-/dsh-session-reference-0.1.5-rc.2.tgz",
      "integrity": "sha512-SpkwyQp0o28bxc01xwid+s+nVCo1Xs8OGsOqyuPRky4WhZtbuI3HN0q/M7+8EcH+kN/SbUzrDD45Q4FAiiHzbQ==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2",
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-compaction": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-output-retention": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection-cache": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-query": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-title": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-spill": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-system-prompt": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-typert-protocol": "^0.1.5-rc.2"
      },
      "peerDependenciesMeta": {
        "@deepseek-ai/dsh-session-projection-cache": {
          "optional": true
        },
        "@deepseek-ai/dsh-spill": {
          "optional": true
        }
      }
    },
    "node_modules/@deepseek-ai/dsh-session-stats": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-session-stats/-/dsh-session-stats-0.1.5-rc.2.tgz",
      "integrity": "sha512-p0C+g31Xcq5w4FWxS+V2hMfLtb97sHpaHNTaAT1UeVTqpSVHt21qC0PnqfFfOe/OsVkMkWDKWDbLb1cA9SnQ5Q==",
      "license": "MIT",
      "dependencies": {
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-session-telemetry": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-session-telemetry/-/dsh-session-telemetry-0.1.5-rc.2.tgz",
      "integrity": "sha512-HCTlVFiyWHNqewD1wLow3kf6dbDBU8GqjHlLO6Es9viYW4GCAALO8FXxdqbWMSLOkwMNMM5xqWrXedjfSmRehA==",
      "license": "MIT",
      "peer": true,
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-session-telemetry-otel": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-session-telemetry-otel/-/dsh-session-telemetry-otel-0.1.5-rc.2.tgz",
      "integrity": "sha512-T2gH6xJ9NNTnEjQG3od1vVIieT3htMShlqTTfOzxIR8YoGf/92KpCq6xVtYx3URm6aJysRpIbtRmoYzVq8Ln7w==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2",
        "@opentelemetry/api": "^1.9.1",
        "@opentelemetry/api-logs": "^0.220.0",
        "@opentelemetry/exporter-logs-otlp-http": "^0.220.0",
        "@opentelemetry/otlp-exporter-base": "^0.220.0",
        "@opentelemetry/resources": "^2.9.0",
        "@opentelemetry/sdk-logs": "^0.220.0"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-anonymous-user-id": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-command-feedback": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-message-feedback": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-telemetry": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-session-title": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-session-title/-/dsh-session-title-0.1.5-rc.2.tgz",
      "integrity": "sha512-lQ1PzIdySxANbhVM9blxCSlqR0iF2GVCiE9R1Qm0VJJue/G9ZUasz+jGQJeAVZi7PHjVWCALljtqpcnaOZq+6g==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2",
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-session-title-first-prompt-llm": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-session-title-first-prompt-llm/-/dsh-session-title-first-prompt-llm-0.1.5-rc.2.tgz",
      "integrity": "sha512-D+eMbKUadK/9QTkk00TItqvE0ealZek8Tc17HJgdGP7Ya0pSYL3hI03q0U6m7SXJQAAFjENMof3xHS+HmyPhdQ==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-title": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-title-llm": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-session-title-llm": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-session-title-llm/-/dsh-session-title-llm-0.1.5-rc.2.tgz",
      "integrity": "sha512-IO80OEJ88AUX6S3c5hgjmrrOAgqvTwg9S/4o40H2btAFPCefj+ioHg9qa6KyMZaaZBe8dGjjdngpsaOvC/QPIQ==",
      "license": "MIT",
      "peer": true,
      "dependencies": {
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-title": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-timeout": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-session-turn-outline": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-session-turn-outline/-/dsh-session-turn-outline-0.1.5-rc.2.tgz",
      "integrity": "sha512-z3kVLSv/dqwtOM4ii8FfzmmLuEvp9NJORMvm2MQPkYzBxLrQvfxIRPm5Q6uqmTowFrEcYWu1bQCdg6EdnZJDgg==",
      "license": "MIT",
      "dependencies": {
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-settings": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-settings/-/dsh-settings-0.1.5-rc.2.tgz",
      "integrity": "sha512-LI2Y6GkEs9ALMW+7S9jHPeEZDXHtG5X6cix1HdJ1rRxTEO5427QlYhMiz45rk7hqZ8ca2O4XFMW8IWXFZQLxmw==",
      "license": "MIT",
      "peer": true,
      "dependencies": {
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-settings-file": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-settings-file/-/dsh-settings-file-0.1.5-rc.2.tgz",
      "integrity": "sha512-5bdcCyDLMQyo4sm1g8l4mNjIrsyfn1mu3+9tvGYrDWIw0f1n1YEIWuHp2reyoZpRCMAco9S5N4mDoKF2mc9T/Q==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2",
        "chokidar": "^4.0.3",
        "yaml": "^2.9.0"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-atomic-write": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-home-paths": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-settings": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-shell": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-shell/-/dsh-shell-0.1.5-rc.2.tgz",
      "integrity": "sha512-BfmNN6X0NHN2XleW0fCbtFXedXEppeDQ2oa3WsOTuhvnBftl6QWMWWMM59weE4xXker5fzqOnoJdeBR8D9qR9Q==",
      "license": "MIT",
      "peer": true,
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-sandbox": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-settings": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-subprocess": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-shell-env": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-shell-env/-/dsh-shell-env-0.1.5-rc.2.tgz",
      "integrity": "sha512-fFSrfhxfvVYfDxsOuV0cjAeC/PWweW+86uOJWT+2paHXOSgM6MSDe3eTd5DHRDYnjb5XutTYu32pR49cVBHMug==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-home-paths": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-shell": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-skill": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-skill/-/dsh-skill-0.1.5-rc.2.tgz",
      "integrity": "sha512-Z1mouW3vTzmYk9OAGUp7S9oOB4k9+qr8BpD7KPAR0nAPX1XOxyKA7zQA/6P0tMS69/aS7UsyLZNz9+RG8ZZ8Wg==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-scope": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-skill-badge": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-skill-badge/-/dsh-skill-badge-0.1.5-rc.2.tgz",
      "integrity": "sha512-piOjH/WNjv0If4P3IQLuAsOh0WWbmWBv/06IXd1x7emet/d62nnnAp2gl75pqpYDR6qyavvRLNCqhqBJIudKTw==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-skill": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-skill-filesystem": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-skill-filesystem/-/dsh-skill-filesystem-0.1.5-rc.2.tgz",
      "integrity": "sha512-lEZ9KojaIc24vKCa5mp8LBHUDoXpNRSR/vlMsWONc9Tc1wwfn9kEotIjj0HD4PjIRcnc+Lma1nk4G/oNED8A9Q==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2",
        "chokidar": "^5.0.0",
        "yaml": "^2.4.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-fs": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-home-paths": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-skill": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-skill-filesystem/node_modules/chokidar": {
      "version": "5.0.0",
      "resolved": "https://registry.npmjs.org/chokidar/-/chokidar-5.0.0.tgz",
      "integrity": "sha512-TQMmc3w+5AxjpL8iIiwebF73dRDF4fBIieAqGn9RGCWaEVwQ6Fb2cGe31Yns0RRIzii5goJ1Y7xbMwo1TxMplw==",
      "license": "MIT",
      "dependencies": {
        "readdirp": "^5.0.0"
      },
      "engines": {
        "node": ">= 20.19.0"
      },
      "funding": {
        "url": "https://paulmillr.com/funding/"
      }
    },
    "node_modules/@deepseek-ai/dsh-skill-filesystem/node_modules/readdirp": {
      "version": "5.1.1",
      "resolved": "https://registry.npmjs.org/readdirp/-/readdirp-5.1.1.tgz",
      "integrity": "sha512-Kko+Y5XQ6fM+Ce3dq3m9YGxnacYZYl9cA1wZjaF3Vbry2L3i1qVg8+CAgNPsXRArPMUMCaOR7oa9Nqntc43JKA==",
      "license": "MIT",
      "engines": {
        "node": ">= 20.19.0"
      },
      "funding": {
        "type": "individual",
        "url": "https://paulmillr.com/funding/"
      }
    },
    "node_modules/@deepseek-ai/dsh-spill": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-spill/-/dsh-spill-0.1.5-rc.2.tgz",
      "integrity": "sha512-djiz5HH1xtuxiBAPT6ODlwEPUQz/SR/Q6SCUiEud7x90nmTsfLLxGUQmNETdiRC3juBLZeoDV1Xgmjf54P4y+A==",
      "license": "MIT",
      "peer": true,
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-spill-local": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-spill-local/-/dsh-spill-local-0.1.5-rc.2.tgz",
      "integrity": "sha512-HrTEyP/KJh9YudGYmXBZvu54x022JWIlVX96d2UkHKICSB1NJy3AUIO0zyYCOk1PX+4iC77oD6zdOAyyxX21QA==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-spill": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-spill-policy": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-spill-policy/-/dsh-spill-policy-0.1.5-rc.2.tgz",
      "integrity": "sha512-XDtIEF7eeErjf0HS6ASkquXZIorv96+A3kuSZs9ZoWqI6eHVxUBrTebHhNc59P9Cp6LeumLUhPIb+SEP3ITgYA==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-output-retention": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-spill": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-storage": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-storage/-/dsh-storage-0.1.5-rc.2.tgz",
      "integrity": "sha512-mYt6+JSRxA5fSUcYnI3JFvYw9/TKUyykz5JWoZZ3MxWudItfhw+CV6T3kn3/bpTbWIQnGesSeY+0pJJVz0xBAA==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-storage-domain": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-storage-domain/-/dsh-storage-domain-0.1.5-rc.2.tgz",
      "integrity": "sha512-NZ9U14kqcCnF4F+ynrTlBLb+S9FEOwSLkfdjDgdfrFdHh4c2WsRKWxyeYbL8AME1rB+XxLp4YQBx8LQ0Kiy62A==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2",
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-storage": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-storage-json": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-storage-json/-/dsh-storage-json-0.1.5-rc.2.tgz",
      "integrity": "sha512-vIdKeI6BUQpqneD/+oTZ2YNotNSGXZX0VbFbhdvgMSgamOrExIjl4cRsEJttcNr8zSvB8PjmuX2wPFvkMoeAlQ==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-storage": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-subagent": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-subagent/-/dsh-subagent-0.1.5-rc.2.tgz",
      "integrity": "sha512-f2wbfB1M2P8RpcApp5eDpDQR4V5ElvEkeK6su7tor1U06uJadJ+JuIVXP3MrCnT7yjg/IM2tTacYFMOg62BOfg==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-chunked-list": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2",
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-agent-presets": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-attachment": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-jobs": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sandbox": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sandbox-policy": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-scope": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-persistence": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection-cache": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-query": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-system-prompt": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-typert-protocol": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-user-approval": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-util-time": "^0.1.5-rc.2"
      },
      "peerDependenciesMeta": {
        "@deepseek-ai/dsh-agent-presets": {
          "optional": true
        },
        "@deepseek-ai/dsh-jobs": {
          "optional": true
        },
        "@deepseek-ai/dsh-sandbox": {
          "optional": true
        },
        "@deepseek-ai/dsh-sandbox-policy": {
          "optional": true
        },
        "@deepseek-ai/dsh-session-persistence": {
          "optional": true
        },
        "@deepseek-ai/dsh-session-projection": {
          "optional": true
        },
        "@deepseek-ai/dsh-session-projection-cache": {
          "optional": true
        },
        "@deepseek-ai/dsh-session-query": {
          "optional": true
        },
        "@deepseek-ai/dsh-user-approval": {
          "optional": true
        }
      }
    },
    "node_modules/@deepseek-ai/dsh-subagent-fork-in-process": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-subagent-fork-in-process/-/dsh-subagent-fork-in-process-0.1.5-rc.2.tgz",
      "integrity": "sha512-9VHI9pA1FX7mEOUx8Iw7Q6Sqk76rJGsOCQ0jwsaE+7YvQxV8YhOFe9gBzkmk+ChRNag5ToBVLFuUFROpboo+Xw==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-subagent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-subagent-in-process-driver": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-subagent-in-process-driver": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-subagent-in-process-driver/-/dsh-subagent-in-process-driver-0.1.5-rc.2.tgz",
      "integrity": "sha512-IBwRQHVX4hi3YO2vUYmTtDxKM2B0lIwAuiAG3hsvljmaF66z2snyLYezKszLZeTPphr2QHgt2CJNxNP1LlhUJg==",
      "license": "MIT",
      "peer": true,
      "dependencies": {
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-subagent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-system-prompt": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-subagent-spawn-in-process": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-subagent-spawn-in-process/-/dsh-subagent-spawn-in-process-0.1.5-rc.2.tgz",
      "integrity": "sha512-/FJxY+dBc8AVWFmSYcQPWapuTd3ATGQpAb9Z2cQEZjFRXy8ySuZnh2MrnAa/r00FX6Lz0G8BfvUurr4K+uKHSw==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-subagent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-subagent-in-process-driver": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-subprocess": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-subprocess/-/dsh-subprocess-0.1.5-rc.2.tgz",
      "integrity": "sha512-Bbxg5/dlbK09qNu4/BVuicUAYnC+b/YGhtkUBsxa9/+KRfvloeJcJ9Ug6dKAeJeluzEXnFIh08+MNweO7hn7ig==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-http-proxy": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-subprocess-local": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-subprocess-local/-/dsh-subprocess-local-0.1.5-rc.2.tgz",
      "integrity": "sha512-DmG3lcQlAh8bTKfeyYM44cfRyJktbLL8drXrVfOvGrFk0zSs5eTSLcfhIKLT3jFJ5CKFriHYZPLdRT6Alvyy4w==",
      "hasInstallScript": true,
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-win32-process": "^0.1.5-rc.2",
        "koffi": "^3.1.0",
        "node-pty": "1.2.0-beta.15"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-subprocess": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-timeout": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-system-prompt": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-system-prompt/-/dsh-system-prompt-0.1.5-rc.2.tgz",
      "integrity": "sha512-VtmZVKqBMJ7kzskHu0jY+Jth7jSuKMG8QB3MBPzJe9M0LL4YPMWsGKt9gGky9RXolk/ugAy4YZEv1gUqouVofA==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-scope": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-terminal": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-terminal/-/dsh-terminal-0.1.5-rc.2.tgz",
      "integrity": "sha512-ALDjWfRMWvRmk/HfM7z5sXfQ1DSarJcjkzL8bT1KV01WlJB1KHVbXKlLI29tQH1Zu92uXk9g0JaFQKNycqT9yQ==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-terminal-bash": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-terminal-bash/-/dsh-terminal-bash-0.1.5-rc.2.tgz",
      "integrity": "sha512-UJVx058VgMNjwptwlGPBsMCmAs/ljgQwW9RNDZCpxvKxtc6D4RqEm675/f4uMPnGEKhPCJFKCGZy3ltqHvUPdg==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-pwsh-local": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2",
        "@xterm/headless": "^6.0.0"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sandbox": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sandbox-policy": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-subprocess": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-terminal": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-time-context": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-time-context/-/dsh-time-context-0.1.5-rc.2.tgz",
      "integrity": "sha512-XClt8a3ijmELHrMuPG8F+P7uLbi+ZBcjulTdeMq7UB4yRSmTHNcQ1rJLyPjPajSDsR6F81x6z6xT842PC4u1kQ==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2",
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-timeout": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-timeout/-/dsh-timeout-0.1.5-rc.2.tgz",
      "integrity": "sha512-FgfaAw8Zt5X4Y6Ljh833B8Hy7h96FeR+yjNKI3iSyZSqREqOBgvDGXFfQlSu6V3Buvw/cbI/CJUQJvfRBKXOiA==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-tmux-context": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-tmux-context/-/dsh-tmux-context-0.1.5-rc.2.tgz",
      "integrity": "sha512-gocDAjUDx80DkbS92/WZJqx+NJkTfuGnSBglDXdUZRFj5O38Ggl9ywSKxTISk/Yhj3SbjRab9UfNjBZ1M8Nlxg==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2",
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-shell": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-token-meter": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-token-meter/-/dsh-token-meter-0.1.5-rc.2.tgz",
      "integrity": "sha512-GS13T/USWa8pFBsahLGQCTQ31ktIa5iBqUYyFFrDPKhvUZFk7dTgltqIo9BxpsU5op0Lwj1LFyCnhNnuYbzvaw==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2",
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-compaction": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm-retry": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-tool-ask-user": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-tool-ask-user/-/dsh-tool-ask-user-0.1.5-rc.2.tgz",
      "integrity": "sha512-M+ha4uty4SM/nEfkOeIMw7vxj3Ez6lZin6oQqgw5hWXNrmf5C9SfF8VH6KZYuPhsmxPiyTUr5SyULxgn2eTUGA==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-user-questions": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-tool-bash": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-tool-bash/-/dsh-tool-bash-0.1.5-rc.2.tgz",
      "integrity": "sha512-f4LmiZkZSfJfvBcEzV4q5J83VL79l/+ncLkwnJyHzsjvJbW5qFxzCy4p5FXfY/CflG0taG14/p42UJzb9qEhrQ==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-jobs": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sandbox": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sandbox-policy": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-shell": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-shell-env": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-system-prompt": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-user-approval": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-tool-bash-persistent": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-tool-bash-persistent/-/dsh-tool-bash-persistent-0.1.5-rc.2.tgz",
      "integrity": "sha512-OCfqQijK/eo+ILLatQntWnKJ7hFwUUOLpxdbkgmN8zjvD3o3LPRnnt9IerCid4YjGwc69jSTBX0Zk2jW2n/rhQ==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-terminal": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-timeout": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-tool-call-timeout-policy": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-tool-call-timeout-policy/-/dsh-tool-call-timeout-policy-0.1.5-rc.2.tgz",
      "integrity": "sha512-B8lBJ6FHT3eqfQupHANU6JGwSvqp4EGYSGgOiNh1EVwXNn5MeX1AztQO+sNqWl69nof9d7/7vxXmTeS7rmIFXw==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-timeout": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-tool-cordis": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-tool-cordis/-/dsh-tool-cordis-0.1.5-rc.2.tgz",
      "integrity": "sha512-bm1p2cRmXvO5b4oj2YHvF6OFh8EcHJQREcD52SEkSWtdkeakHbwMEOeCLfSWSNMUiNeOa7tMutUQrepdCS37EA==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-cordis-host-runner": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-scope": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-system-prompt": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-tool-fs": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-tool-fs/-/dsh-tool-fs-0.1.5-rc.2.tgz",
      "integrity": "sha512-/3AUx+V1UxVfl24fm10hwRbJnHwpBkRDHniOeocGknMvASheRiKnHpnnj9EszFRLYSpe2PRFSMuyuhXgYyd3MQ==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2",
        "diff": "^9.0.0"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-attachment": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-fs": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sandbox": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sandbox-policy": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-system-prompt": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-user-approval": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-tool-fs-search": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-tool-fs-search/-/dsh-tool-fs-search-0.1.5-rc.2.tgz",
      "integrity": "sha512-B4UrzbrigYiD8IKweuVzOHHLrhKfBPWTxscPDy0WmaYJ+aYQ/qaix9RgxnmJRoF8aVo4lw/Fzlz6hq30ahwUIw==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2",
        "@vscode/ripgrep": "^1.18.0"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-output-retention": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-spill": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-subprocess": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-system-prompt": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-timeout": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-tool-goal": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-tool-goal/-/dsh-tool-goal-0.1.5-rc.2.tgz",
      "integrity": "sha512-nZ0NkUxvtAsrPAd9NMXt+4kS7WPn8xIvXCwVunKfBwbrnGnFCNmIsTQoHF44J6q9VbMeSCN2eyj07w6ej3Hf+g==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-goal": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-system-prompt": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-tool-jobs": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-tool-jobs/-/dsh-tool-jobs-0.1.5-rc.2.tgz",
      "integrity": "sha512-v4y56H3FsVBF2jUJLGLHdPeKaKJxLT4Zx/oii45UmJv5t8Bp3uCTVUoq/mfSFVuFY66q6rHJwgIOKWc4ZO3ySw==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-jobs": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-output-retention": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-system-prompt": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-tool-present": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-tool-present/-/dsh-tool-present-0.1.5-rc.2.tgz",
      "integrity": "sha512-WWxjEgh/nPsATfofNc0Qi+Eshy9LPNAYM+UWWisuKcT6smtv7+pR8Bk79RJu5BiFog8Nq17mwEh5XnFlDIg8FQ==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-fs": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-tool-pwsh": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-tool-pwsh/-/dsh-tool-pwsh-0.1.5-rc.2.tgz",
      "integrity": "sha512-rnHM3Jqlr7rthwfPYysXPq6zH+K8JTDqbE+xzjkbo1WIBKBZeSGr8kPjqdCZa7eUtWhmV/RFBP1qmBgDjEZaWg==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-jobs": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sandbox": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sandbox-policy": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-shell": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-shell-env": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-system-prompt": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-user-approval": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-tool-pwsh-persistent": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-tool-pwsh-persistent/-/dsh-tool-pwsh-persistent-0.1.5-rc.2.tgz",
      "integrity": "sha512-NToI2tXkbAZS6Cxjnn7VjgCF7o68vMquvmpVh6RjTXAsawNY0TMmAVo67bREDw07+1j/bTq8EPywrJSmBbS2Fw==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-terminal": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-timeout": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-tool-ralph": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-tool-ralph/-/dsh-tool-ralph-0.1.5-rc.2.tgz",
      "integrity": "sha512-qP7KoVUBPLvvPSiJBXJtQt6c5ChnJRs5ZJnW/VU2DbBjYn4lkN6BiSfpRsSfjRGKO9fmldn2BYR0OnSLXi57xw==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-subagent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-system-prompt": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-workflow": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-tool-skill": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-tool-skill/-/dsh-tool-skill-0.1.5-rc.2.tgz",
      "integrity": "sha512-J0AshOq06tVWFbESolUP+9TEYXymASMULsWsqTBAgaIjzbZjhJ3x2CPMP54gM8uROh/y0qGBhVUErJeamP9AhQ==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-skill": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-tool-str-replace-editor": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-tool-str-replace-editor/-/dsh-tool-str-replace-editor-0.1.5-rc.2.tgz",
      "integrity": "sha512-SqxB7jgmAURwttFMQ9CmM4l1n5pdMA1ofPOd0Fp2OqYRdo8C0NCO0YN06PtX3CvmjI6vjDUT3grXqETzhmN3gg==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-fs": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sandbox": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-sandbox-policy": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-tool-subagent": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-tool-subagent/-/dsh-tool-subagent-0.1.5-rc.2.tgz",
      "integrity": "sha512-6H7iL1UMcPrQTZrmp/olatipQzg6z5b3xqqPpW26qhgX9tDttJJGAspObePM8Zj02EO9ibz7QLg/VIehPuu+sg==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2",
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-jobs": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-scope": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-settings": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-subagent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-system-prompt": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-tool-subagent-control": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-tool-subagent-control/-/dsh-tool-subagent-control-0.1.5-rc.2.tgz",
      "integrity": "sha512-kuSx9UGDBUSrffTPnNetKj7mUmH1+24BQT+589vaN5epO8GKZ1STzIR3IINn+kMP0t9MAGIhEsa39b30OQu9nA==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-subagent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-tool-todo": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-tool-todo/-/dsh-tool-todo-0.1.5-rc.2.tgz",
      "integrity": "sha512-TmyoAthal1pOCQ/XwlohcxDXdI5pq5GlZiQpLaI7e+rAruqneBPz/0Zjv6/rlOMrseoENMx+H0sazwoL9LufTQ==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2",
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-projection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-tool-web": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-tool-web/-/dsh-tool-web-0.1.5-rc.2.tgz",
      "integrity": "sha512-FPHiDBHjyO/2ayVcUL8tmcBqc+Fi8Y0zcQuzsFw6/K8pOJvP/pBSN8mXi4tBVmmqXg25ZXVo1vDDo4T1mBIlFw==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2",
        "@joplin/turndown-plugin-gfm": "^1.0.67",
        "turndown": "^7.2.4"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-system-prompt": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-web": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-tool-workflow": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-tool-workflow/-/dsh-tool-workflow-0.1.5-rc.2.tgz",
      "integrity": "sha512-co/4mEKOC5MYXM/x2sdTiIwPfUtK3Bhst7HMwmgcz3PNZG4DEkaqLlzsZyisLn4iQvKChtvdyS5fWwNeC8ws1w==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-system-prompt": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-workflow": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-tools": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-tools/-/dsh-tools-0.1.5-rc.2.tgz",
      "integrity": "sha512-k2yZuJJtszaU9lzr2aBtdeFMINrkdlk4ellbtrMokA2oySVqJmAM8dv+u9RtzduD3aRRqyr2i2hWrTACz0qOrA==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-code-runtime": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-scope": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-system-prompt": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-user-approval": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-typert-loader": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-typert-loader/-/dsh-typert-loader-0.1.5-rc.2.tgz",
      "integrity": "sha512-5kNw5jLJ1PBGqL9Ma/ym+Uz8pbFhMe2Ci0QypPtE71ViUC8K0usgypqmn3+2vXtBk5E5mI/n3UGu9gyTPdHIYA==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/cordis-plugin-loader": "^1.0.3",
        "@deepseek-ai/dsh-typert-registry": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-typert-protocol": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-typert-protocol/-/dsh-typert-protocol-0.1.5-rc.2.tgz",
      "integrity": "sha512-zP8J20rBXa1AFjKL2i83JBeQcePZx8yal3pNojj4t7CCokMcoFMUbr50woa6fY1tRJ7qHxgypO3rH8Q6YWgmiA==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-typert-registry": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-typert-registry/-/dsh-typert-registry-0.1.5-rc.2.tgz",
      "integrity": "sha512-GBHGD8T4goY8kKA0U1jD2YRPAgkESdVuSNXn7KpYeb3SLtVntt2ND+FMSwNqcMN/zuBm9Yvo5x/nuK+yxFqvBw==",
      "license": "MIT",
      "dependencies": {
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-user-approval": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-user-approval/-/dsh-user-approval-0.1.5-rc.2.tgz",
      "integrity": "sha512-8UpMEnEyMo6mYEVELBo0DC2iG7aJJfFMTNkU+DKD3c5Ut7ySRDzt1iRr1qni/CzqlTIpQBAkGaG77qk4q/v1bg==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-scope": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-system-prompt": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-user-questions": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-user-questions/-/dsh-user-questions-0.1.5-rc.2.tgz",
      "integrity": "sha512-0cRRE9nc9pMxxyC8G8C3859EtPyZR/27FRgRMRe/wDWL30S6JoWWgaw7IFN9FogvaoEp62XV7wQ6ub/eoTqggg==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-scope": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-util-crypto": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-util-crypto/-/dsh-util-crypto-0.1.5-rc.2.tgz",
      "integrity": "sha512-JR0aJEUL35RE8FVT89wMZMbPwMrbkjNcy+gE9pq70d/m2Zxw3a0iNer+BAxPAKYPbmy0xDDXReTwXiecKVy4sQ==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-util-time": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-util-time/-/dsh-util-time-0.1.5-rc.2.tgz",
      "integrity": "sha512-nWJrNCaEBANFUX+Rp2cE1+hA2CGCmktXOo2Nsh524aV2VaLumFFS501IsmI9ghUmUE77qZxvayGGO7w3LLIECQ==",
      "license": "MIT",
      "peer": true,
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-util-values": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-util-values/-/dsh-util-values-0.1.5-rc.2.tgz",
      "integrity": "sha512-Cr0TkM6dFAD+Iy4sNXH/afJVMHwJXoOztivj+RflkYoACsOq+Ix0vdHn36hwW7mayU8OK3ZKIZUQ6BPo1fHvUg==",
      "license": "MIT",
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-util-workspace-path": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-util-workspace-path/-/dsh-util-workspace-path-0.1.5-rc.2.tgz",
      "integrity": "sha512-RCBz+6BpdPDNsRk2ukIdIIuLdf6u+cS8sLiViFg/8/x/kxc+LEXRKJMy2xv+YA9hYOGFK109rjUFgx9JVtZf2w==",
      "license": "MIT",
      "peer": true,
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-web": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-web/-/dsh-web-0.1.5-rc.2.tgz",
      "integrity": "sha512-3qt/Fh+uCghOy2wZPjwQ6xIMU3t1NW4D5yRvTXOyvadwaDwosqXOzCteYUhnuWzWN66adrFLeRrymhSDS4PMeg==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-web-app": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-web-app/-/dsh-web-app-0.1.5-rc.2.tgz",
      "integrity": "sha512-Ng7YVDt9txh2BlLmu6B+V677c1bihbh/rq3EOtZK4b0JWtgdNgYb1KPIED8+aK2i+FghrCrIy6X60AyJ45eOvw==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-agent-presets": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-api-remotes": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-api-session-controller": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-api-settings-controller": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-api-workspace-controller": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-api-workspace-files": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-app-boot": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-connection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-file-upload": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-hmr": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-locale": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-modules": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-resources": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-agent-preset": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-approval": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-attachment": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-brand-official": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-chat": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-commands": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-conversation": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-cordis": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-deliverables": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-directory-picker-browse": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-directory-picker-native": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-goal": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-input-trigger": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-jobs": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-layout": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-message-feedback": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-model-selection": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-open-in-app": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-permission-presets": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-plan": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-reference": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-renderer": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-schedule": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-settings": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-settings-general": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-settings-models": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-settings-plugin-inventory": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-settings-plugins": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-sidebar": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-sidebar-documentpreview": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-sidebar-files": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-sidebar-right": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-skill": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-subagent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-theme": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-tool": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-trajectory": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-user-questions": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-workflow-run": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-client-ui-workspace": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-cmdline": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-code-runtime-worker-thread": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-cordis-client-runner": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-cordis-host-runner": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-file-reference": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-file-reference-local": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-host-directory-picker-auto": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-host-directory-picker-browse": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-host-directory-picker-native": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-host-frontend-static": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-host-open-in-app": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-host-plugin-inventory": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-host-webserver": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-launch-environment": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-message-feedback": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-log-export": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-reference": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-stats": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-turn-outline": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-subprocess": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tool-subagent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-web-frontend": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-workspace": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2",
        "commander": "^15.0.0",
        "open": "^11.0.0"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/cordis-plugin-loader": "^1.0.3",
        "@deepseek-ai/dsh-shell-env": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-system-prompt": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-web-fetch-http": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-web-fetch-http/-/dsh-web-fetch-http-0.1.5-rc.2.tgz",
      "integrity": "sha512-K3us2mU0L1sgnW4FFtDI6UYcL+JZvIbdRLEAO+8oq9lHAuwKuCcb4RPPuyXMI35ue97PkfBgt6A6Sl0utiIF3A==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2",
        "ipaddr.js": "^2.5.0",
        "undici": "^8.10.0"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-http-proxy": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-timeout": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-web": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-web-frontend": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-web-frontend/-/dsh-web-frontend-0.1.5-rc.2.tgz",
      "integrity": "sha512-o0+dEbtEzchPnNFkKQ4o3/OekEi+PG8CTEwp1eWWg15TRiZbG2OeUKMsQjPpAVCMDnpEwMQ9skdKcwKj4TXAzQ==",
      "license": "MIT"
    },
    "node_modules/@deepseek-ai/dsh-web-search-deepseek": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-web-search-deepseek/-/dsh-web-search-deepseek-0.1.5-rc.2.tgz",
      "integrity": "sha512-0r9KqkjnOd7VLwWEAYHOEGLctRwhGLOJ9xnnGuL3nd3uFcrA5Wb1MYbJhGKxx+9XRpTJZyi7Aiex9QRAr8c0Ag==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-credentials": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-launch-environment": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-settings": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-web": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-webhook": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-webhook/-/dsh-webhook-0.1.5-rc.2.tgz",
      "integrity": "sha512-Tv4aR5JYxq9ODe9lYoFaQWjFUf644tPaSVSgfBMPrynN9X94rcY7HAkYxt3qLggvK/cGrtNOFwyqyOIDTjqyJQ==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-agent-default-model": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-agent-presets": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-permission-presets": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-title": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-workspace": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-webhook-github": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-webhook-github/-/dsh-webhook-github-0.1.5-rc.2.tgz",
      "integrity": "sha512-esocwn22CYbOAo7WzwWl/E3jsHKOAXFS5e1dnW9b5VNPujnbHdvhXyyNQzUW0nNnxt1QTsIARyhqpDKWuanCcA==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2",
        "@octokit/webhooks": "^14.2.0"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-credentials": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-host-webserver": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-webhook": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-win32-process": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-win32-process/-/dsh-win32-process-0.1.5-rc.2.tgz",
      "integrity": "sha512-KWF9pldnznDE1XiD7vjxg+P2cDQHOCJtz8ZQ/P0bLorDog/peQwqxYNgc/nrHxEZ1vWl/vbdd8Nvoen6FJ/Orw==",
      "license": "MIT",
      "dependencies": {
        "koffi": "^3.1.0"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-workflow": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-workflow/-/dsh-workflow-0.1.5-rc.2.tgz",
      "integrity": "sha512-eGOBiC4RA89zZTTG6hBpTchau55lZ67P/dyqg4kJq8K1FwummTg3xed4Ecy9KQFaSX8rWQP9RMJnaWg1fBgXTA==",
      "license": "MIT",
      "peer": true,
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-workflow-worker-thread": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-workflow-worker-thread/-/dsh-workflow-worker-thread-0.1.5-rc.2.tgz",
      "integrity": "sha512-y8twj+FNYoxaaLWYkr/tQauCHymlY3AUgS5lHEi4tZW1PgYD2mEz7VPHu1nUWX7W6m78gCQvg/6KcDCLrTgvcA==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-util-values": "^0.1.5-rc.2",
        "@deepseek-ai/schemastery": "^3.18.2"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-agent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-llm": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-subagent": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-tools": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-workflow": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/dsh-workspace": {
      "version": "0.1.5-rc.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/dsh-workspace/-/dsh-workspace-0.1.5-rc.2.tgz",
      "integrity": "sha512-J2RIjHk2RNTtQNZ1Dpy1ItdhOmXDsWXLvMNv/MMdvv3NzCjKMYQ8le18lpzboqAZY3I2cOfX0cxu6wHIC96SIg==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/dsh-brand": "^0.1.5-rc.2",
        "zod": "^4.4.3"
      },
      "peerDependencies": {
        "@deepseek-ai/cordis": "^4.0.2",
        "@deepseek-ai/dsh-invariants": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-session-persistence": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-storage": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-storage-domain": "^0.1.5-rc.2",
        "@deepseek-ai/dsh-typert-protocol": "^0.1.5-rc.2"
      }
    },
    "node_modules/@deepseek-ai/node-addon-system": {
      "version": "0.1.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/node-addon-system/-/node-addon-system-0.1.2.tgz",
      "integrity": "sha512-EFn8K+mXIXiw6s3rxX+cgp2hZTEAQdVIsURhrXnBxycSXvtQ1EQpsSVzoNKtKAQI0I0VqkEcVEjHzS/PeaGsvw==",
      "license": "BSD-3-Clause",
      "engines": {
        "node": ">=20"
      },
      "optionalDependencies": {
        "@deepseek-ai/node-addon-system-darwin-arm64": "0.1.2",
        "@deepseek-ai/node-addon-system-darwin-x64": "0.1.2",
        "@deepseek-ai/node-addon-system-linux-arm64": "0.1.2",
        "@deepseek-ai/node-addon-system-linux-x64": "0.1.2"
      }
    },
    "node_modules/@deepseek-ai/node-addon-system-linux-x64": {
      "version": "0.1.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/node-addon-system-linux-x64/-/node-addon-system-linux-x64-0.1.2.tgz",
      "integrity": "sha512-S2aPVHvYCpNCppCFyNlooMYuTB7ucK5lvD9oXQQ42v5Z2s5AoaiCdjz9r2l+ED2rI5oS1x0cUZmKjJH2dxV0pg==",
      "cpu": [
        "x64"
      ],
      "license": "BSD-3-Clause",
      "optional": true,
      "os": [
        "linux"
      ],
      "engines": {
        "node": ">=20"
      }
    },
    "node_modules/@deepseek-ai/schemastery": {
      "version": "3.18.2",
      "resolved": "https://registry.npmjs.org/@deepseek-ai/schemastery/-/schemastery-3.18.2.tgz",
      "integrity": "sha512-njDtZsznjYxok7KLLlHOPyuv2efdWVbSflAHgztSfbMsg+CVraEoRe2DjOCgClYv3ZCSm7WXoaUkbB/+RY7tWQ==",
      "license": "MIT",
      "dependencies": {
        "@deepseek-ai/cosmokit": "^1.8.3",
        "@standard-schema/spec": "^1.1.0"
      }
    },
    "node_modules/@earendil-works/pi-ai": {
      "version": "0.85.1",
      "resolved": "https://registry.npmjs.org/@earendil-works/pi-ai/-/pi-ai-0.85.1.tgz",
      "integrity": "sha512-+VgVIJDkDO2efYJKEEqvPTH4zmnIaXdAppGbO+vKFA9qy5PdhFiAenuFAkU+oiCSfOC4dMHDyrjdQeL4ZoC5CQ==",
      "license": "MIT",
      "dependencies": {
        "@anthropic-ai/sdk": "0.123.0",
        "@aws-sdk/client-bedrock-runtime": "3.1048.0",
        "@earendil-works/pi-telemetry": "^0.85.1",
        "@google/genai": "1.52.0",
        "@smithy/node-http-handler": "4.7.3",
        "http-proxy-agent": "7.0.2",
        "https-proxy-agent": "7.0.6",
        "openai": "6.40.0",
        "partial-json": "0.1.7",
        "typebox": "1.3.7"
      },
      "bin": {
        "pi-ai": "dist/cli.js"
      },
      "engines": {
        "node": ">=22.19.0"
      }
    },
    "node_modules/@earendil-works/pi-telemetry": {
      "version": "0.85.1",
      "resolved": "https://registry.npmjs.org/@earendil-works/pi-telemetry/-/pi-telemetry-0.85.1.tgz",
      "integrity": "sha512-Bg/YN6kA7Swja/NQxka8xFdecb4E/auIEGF2G5A25EaQXhRnPj300/7/KpgsDDMYUzHTDAv4RyUxaQPJKW81Rw==",
      "license": "MIT",
      "engines": {
        "node": ">=22.19.0"
      }
    },
    "node_modules/@emnapi/runtime": {
      "version": "1.11.3",
      "resolved": "https://registry.npmjs.org/@emnapi/runtime/-/runtime-1.11.3.tgz",
      "integrity": "sha512-Xz4Tpyki7XyrpbUK1jR1AhdAdaXyhhY4lZ3neLodmhpuWfy2PAQN5B46sAiU4liOXGLkHypn/qU+jvfWSCYYLA==",
      "license": "MIT",
      "optional": true,
      "dependencies": {
        "tslib": "^2.4.0"
      }
    },
    "node_modules/@google/genai": {
      "version": "1.52.0",
      "resolved": "https://registry.npmjs.org/@google/genai/-/genai-1.52.0.tgz",
      "integrity": "sha512-gwSvbpiN/17O9TbsqSsE/OzZcpv5Fo4RQjdngGgogtuB9RsyJ8ZHhX5KjHj1bp5N9snN2eK8LDGXSaWW2hof8Q==",
      "hasInstallScript": true,
      "license": "Apache-2.0",
      "dependencies": {
        "google-auth-library": "^10.3.0",
        "p-retry": "^4.6.2",
        "protobufjs": "^7.5.4",
        "ws": "^8.18.0"
      },
      "engines": {
        "node": ">=20.0.0"
      },
      "peerDependencies": {
        "@modelcontextprotocol/sdk": "^1.25.2"
      },
      "peerDependenciesMeta": {
        "@modelcontextprotocol/sdk": {
          "optional": true
        }
      }
    },
    "node_modules/@hono/node-server": {
      "version": "2.1.1",
      "resolved": "https://registry.npmjs.org/@hono/node-server/-/node-server-2.1.1.tgz",
      "integrity": "sha512-ELuehkj5VCBdgEw9zs+ivkKwyzzUCSQuE96YmiPvn1ECBoZCczbFXJLeEGMTYjphP6gydh4pHMqEYPVMYUVgQg==",
      "license": "MIT",
      "engines": {
        "node": ">=20"
      },
      "peerDependencies": {
        "hono": "^4"
      }
    },
    "node_modules/@img/colour": {
      "version": "1.1.0",
      "resolved": "https://registry.npmjs.org/@img/colour/-/colour-1.1.0.tgz",
      "integrity": "sha512-Td76q7j57o/tLVdgS746cYARfSyxk8iEfRxewL9h4OMzYhbW4TAcppl0mT4eyqXddh6L/jwoM75mo7ixa/pCeQ==",
      "license": "MIT",
      "engines": {
        "node": ">=18"
      }
    },
    "node_modules/@img/sharp-libvips-linux-x64": {
      "version": "1.3.3",
      "resolved": "https://registry.npmjs.org/@img/sharp-libvips-linux-x64/-/sharp-libvips-linux-x64-1.3.3.tgz",
      "integrity": "sha512-4vKmvAst9nrowcqquKFAyZJUDolUaIp8uRiN0mWFguJ1IplC9/pitXtlnnlU4aa/eJw3J7i67V+pwUL+wZGdsA==",
      "cpu": [
        "x64"
      ],
      "license": "LGPL-3.0-or-later",
      "optional": true,
      "os": [
        "linux"
      ],
      "funding": {
        "url": "https://opencollective.com/libvips"
      }
    },
    "node_modules/@img/sharp-linux-x64": {
      "version": "0.35.4",
      "resolved": "https://registry.npmjs.org/@img/sharp-linux-x64/-/sharp-linux-x64-0.35.4.tgz",
      "integrity": "sha512-9qvvEAuk8k89TfWUoX2htWjbAMX8p+NxCppjpcg5k6xMsjhBQPTsoIh36h9Qde4WRuGpJeYnOjdosDn/cnv+OA==",
      "cpu": [
        "x64"
      ],
      "license": "Apache-2.0",
      "optional": true,
      "os": [
        "linux"
      ],
      "engines": {
        "node": ">=20.9.0"
      },
      "funding": {
        "url": "https://opencollective.com/libvips"
      },
      "optionalDependencies": {
        "@img/sharp-libvips-linux-x64": "1.3.3"
      }
    },
    "node_modules/@img/sharp-wasm32": {
      "version": "0.35.4",
      "resolved": "https://registry.npmjs.org/@img/sharp-wasm32/-/sharp-wasm32-0.35.4.tgz",
      "integrity": "sha512-zQnl4Kwp7Q6NHsENtU2T/00Zi+w3AQNwz3+UaTyVBy2FpXrzXzGjndpK61onhZjRtRpQXxCTeqw19bVyXOh7jA==",
      "license": "Apache-2.0 AND LGPL-3.0-or-later AND MIT",
      "optional": true,
      "dependencies": {
        "@emnapi/runtime": "^1.11.3"
      },
      "engines": {
        "node": ">=20.9.0"
      },
      "funding": {
        "url": "https://opencollective.com/libvips"
      }
    },
    "node_modules/@joplin/turndown-plugin-gfm": {
      "version": "1.0.68",
      "resolved": "https://registry.npmjs.org/@joplin/turndown-plugin-gfm/-/turndown-plugin-gfm-1.0.68.tgz",
      "integrity": "sha512-m8DfAQNC/V7g0j5H6Jv60WhekBBjodD+zTPGrU+g2m+ux/z8ZX07KQIl6kaAmbKwfvsQhC04op52g63I78sVgg==",
      "license": "MIT"
    },
    "node_modules/@koromix/koffi-linux-x64": {
      "version": "3.3.1",
      "resolved": "https://registry.npmjs.org/@koromix/koffi-linux-x64/-/koffi-linux-x64-3.3.1.tgz",
      "integrity": "sha512-uU5cJNe145TvITqrUWvBmZO4WLNObdZ2Gnm8eNk/OdrhRFG9LB44RUv07aQwPbdo1438j+Cr+tFNnpjTBadZFQ==",
      "cpu": [
        "x64"
      ],
      "license": "MIT",
      "optional": true,
      "os": [
        "linux"
      ],
      "funding": {
        "url": "https://liberapay.com/Koromix"
      }
    },
    "node_modules/@mixmark-io/domino": {
      "version": "2.2.0",
      "resolved": "https://registry.npmjs.org/@mixmark-io/domino/-/domino-2.2.0.tgz",
      "integrity": "sha512-Y28PR25bHXUg88kCV7nivXrP2Nj2RueZ3/l/jdx6J9f8J4nsEGcgX0Qe6lt7Pa+J79+kPiJU3LguR6O/6zrLOw==",
      "license": "BSD-2-Clause"
    },
    "node_modules/@modelcontextprotocol/sdk": {
      "version": "1.30.0",
      "resolved": "https://registry.npmjs.org/@modelcontextprotocol/sdk/-/sdk-1.30.0.tgz",
      "integrity": "sha512-xKd8OIzlqNzcqcNumGAa6g+PW2kjD5vrpcKOnfldAUPP3j7lnqMPwlTXQm8gF+UwH72z0lqaRbjr9hqGz0eITA==",
      "license": "MIT",
      "dependencies": {
        "@hono/node-server": "^1.19.9 || ^2.0.5",
        "ajv": "^8.17.1",
        "ajv-formats": "^3.0.1",
        "content-type": "^1.0.5",
        "cors": "^2.8.5",
        "cross-spawn": "^7.0.5",
        "eventsource": "^3.0.2",
        "eventsource-parser": "^3.0.0",
        "express": "^5.2.1",
        "express-rate-limit": "^8.2.1",
        "hono": "^4.11.4",
        "jose": "^6.1.3",
        "json-schema-typed": "^8.0.2",
        "pkce-challenge": "^5.0.0",
        "raw-body": "^3.0.0",
        "zod": "^3.25 || ^4.0",
        "zod-to-json-schema": "^3.25.1"
      },
      "engines": {
        "node": ">=18"
      },
      "peerDependencies": {
        "@cfworker/json-schema": "^4.1.1",
        "zod": "^3.25 || ^4.0"
      },
      "peerDependenciesMeta": {
        "@cfworker/json-schema": {
          "optional": true
        },
        "zod": {
          "optional": false
        }
      }
    },
    "node_modules/@octokit/openapi-types": {
      "version": "29.0.1",
      "resolved": "https://registry.npmjs.org/@octokit/openapi-types/-/openapi-types-29.0.1.tgz",
      "integrity": "sha512-9qWOMFNxxLokERcms42rU0PTLqQmVs7g5E41TI4mCOxmpFayD1rfC7XxOL55cG9MBZLFlC31BrR37myMKardwg==",
      "license": "MIT"
    },
    "node_modules/@octokit/openapi-webhooks-types": {
      "version": "12.1.0",
      "resolved": "https://registry.npmjs.org/@octokit/openapi-webhooks-types/-/openapi-webhooks-types-12.1.0.tgz",
      "integrity": "sha512-WiuzhOsiOvb7W3Pvmhf8d2C6qaLHXrWiLBP4nJ/4kydu+wpagV5Fkz9RfQwV2afYzv3PB+3xYgp4mAdNGjDprA==",
      "license": "MIT"
    },
    "node_modules/@octokit/request-error": {
      "version": "7.1.2",
      "resolved": "https://registry.npmjs.org/@octokit/request-error/-/request-error-7.1.2.tgz",
      "integrity": "sha512-XZRuT3xZ84D3gYErI1DZvhJ33dCWVV6uzBtWkaBB4TvA/L6eOeTZodxLFVB44bBEEo3vEx7y00UfX1tBLrtLRg==",
      "license": "MIT",
      "dependencies": {
        "@octokit/types": "^18.0.0"
      },
      "engines": {
        "node": ">= 20"
      }
    },
    "node_modules/@octokit/types": {
      "version": "18.0.0",
      "resolved": "https://registry.npmjs.org/@octokit/types/-/types-18.0.0.tgz",
      "integrity": "sha512-l6bAF43PNxkJp6g+W4PjoUSSkxHomXw2nOum5CTftJz1NlV3vu93NImgOYtLf6CbBUb5j+fiuzW0PPQ5JTSvZA==",
      "license": "MIT",
      "dependencies": {
        "@octokit/openapi-types": "^29.0.1"
      }
    },
    "node_modules/@octokit/webhooks": {
      "version": "14.2.0",
      "resolved": "https://registry.npmjs.org/@octokit/webhooks/-/webhooks-14.2.0.tgz",
      "integrity": "sha512-da6KbdNCV5sr1/txD896V+6W0iamFWrvVl8cHkBSPT+YlvmT3DwXa4jxZnQc+gnuTEqSWbBeoSZYTayXH9wXcw==",
      "license": "MIT",
      "dependencies": {
        "@octokit/openapi-webhooks-types": "12.1.0",
        "@octokit/request-error": "^7.0.0",
        "@octokit/webhooks-methods": "^6.0.0"
      },
      "engines": {
        "node": ">= 20"
      }
    },
    "node_modules/@octokit/webhooks-methods": {
      "version": "6.0.0",
      "resolved": "https://registry.npmjs.org/@octokit/webhooks-methods/-/webhooks-methods-6.0.0.tgz",
      "integrity": "sha512-MFlzzoDJVw/GcbfzVC1RLR36QqkTLUf79vLVO3D+xn7r0QgxnFoLZgtrzxiQErAjFUOdH6fas2KeQJ1yr/qaXQ==",
      "license": "MIT",
      "engines": {
        "node": ">= 20"
      }
    },
    "node_modules/@opentelemetry/api": {
      "version": "1.9.1",
      "resolved": "https://registry.npmjs.org/@opentelemetry/api/-/api-1.9.1.tgz",
      "integrity": "sha512-gLyJlPHPZYdAk1JENA9LeHejZe1Ti77/pTeFm/nMXmQH/HFZlcS/O2XJB+L8fkbrNSqhdtlvjBVjxwUYanNH5Q==",
      "license": "Apache-2.0",
      "engines": {
        "node": ">=8.0.0"
      }
    },
    "node_modules/@opentelemetry/api-logs": {
      "version": "0.220.0",
      "resolved": "https://registry.npmjs.org/@opentelemetry/api-logs/-/api-logs-0.220.0.tgz",
      "integrity": "sha512-CmVa4ImJ+ynfrPMNaAXHET6Bhb44SwzmfyVJFq9ni2jgXJR/l7C6gfVFddNmHP+ZOkP9cf4f9DBe68qVLTHc9w==",
      "license": "Apache-2.0",
      "dependencies": {
        "@opentelemetry/api": "^1.3.0"
      },
      "engines": {
        "node": ">=8.0.0"
      }
    },
    "node_modules/@opentelemetry/core": {
      "version": "2.9.0",
      "resolved": "https://registry.npmjs.org/@opentelemetry/core/-/core-2.9.0.tgz",
      "integrity": "sha512-m2nckMT80NnmjTYSPjJQObBJ+8dgkoajEOUbznL8AHZ3T3yHRk2P7gI1PhEBc1+lOnrYE9UWrWHqJDsmqjmNbw==",
      "license": "Apache-2.0",
      "dependencies": {
        "@opentelemetry/semantic-conventions": "^1.29.0"
      },
      "engines": {
        "node": "^18.19.0 || >=20.6.0"
      },
      "peerDependencies": {
        "@opentelemetry/api": ">=1.0.0 <1.10.0"
      }
    },
    "node_modules/@opentelemetry/exporter-logs-otlp-http": {
      "version": "0.220.0",
      "resolved": "https://registry.npmjs.org/@opentelemetry/exporter-logs-otlp-http/-/exporter-logs-otlp-http-0.220.0.tgz",
      "integrity": "sha512-8186thl+pTw64iz/qEEen5oJZoZ/gO73XruChdaGlYdWOdBIQ42r+vHLf6a7vIDqTD4b8ZOoMlyxptanECaI9A==",
      "license": "Apache-2.0",
      "dependencies": {
        "@opentelemetry/api-logs": "0.220.0",
        "@opentelemetry/core": "2.9.0",
        "@opentelemetry/otlp-exporter-base": "0.220.0",
        "@opentelemetry/otlp-transformer": "0.220.0",
        "@opentelemetry/sdk-logs": "0.220.0"
      },
      "engines": {
        "node": "^18.19.0 || >=20.6.0"
      },
      "peerDependencies": {
        "@opentelemetry/api": "^1.3.0"
      }
    },
    "node_modules/@opentelemetry/otlp-exporter-base": {
      "version": "0.220.0",
      "resolved": "https://registry.npmjs.org/@opentelemetry/otlp-exporter-base/-/otlp-exporter-base-0.220.0.tgz",
      "integrity": "sha512-CXYo8UD5Mn9YbgebO2EL4wejtA+gxLmLiu6HCk2KH2BR7XhFN6/6p1UlCb23DYCjeYkndevLHuejCCN1yx4+OQ==",
      "license": "Apache-2.0",
      "dependencies": {
        "@opentelemetry/core": "2.9.0",
        "@opentelemetry/otlp-transformer": "0.220.0"
      },
      "engines": {
        "node": "^18.19.0 || >=20.6.0"
      },
      "peerDependencies": {
        "@opentelemetry/api": "^1.3.0"
      }
    },
    "node_modules/@opentelemetry/otlp-transformer": {
      "version": "0.220.0",
      "resolved": "https://registry.npmjs.org/@opentelemetry/otlp-transformer/-/otlp-transformer-0.220.0.tgz",
      "integrity": "sha512-lXGrv7KXZ0gNH9SVNUaa6vv6phVYGvJxfXAlMbzbakiXru75f5MZl8Z7oqiMMQD77riVHJCFlQvbZs/VVN2/4A==",
      "license": "Apache-2.0",
      "dependencies": {
        "@opentelemetry/api-logs": "0.220.0",
        "@opentelemetry/core": "2.9.0",
        "@opentelemetry/resources": "2.9.0",
        "@opentelemetry/sdk-logs": "0.220.0",
        "@opentelemetry/sdk-metrics": "2.9.0",
        "@opentelemetry/sdk-trace": "2.9.0"
      },
      "engines": {
        "node": "^18.19.0 || >=20.6.0"
      },
      "peerDependencies": {
        "@opentelemetry/api": "^1.3.0"
      }
    },
    "node_modules/@opentelemetry/otlp-transformer/node_modules/@opentelemetry/resources": {
      "version": "2.9.0",
      "resolved": "https://registry.npmjs.org/@opentelemetry/resources/-/resources-2.9.0.tgz",
      "integrity": "sha512-jyA5MBLQ+Dkl3+JsZkUoUvL7yHvU64kLsvpXKarWm6347Sl1t1bXFTFykUePNpT5WH5pm9a2Qtt03iIYQhZ1Fg==",
      "license": "Apache-2.0",
      "dependencies": {
        "@opentelemetry/core": "2.9.0",
        "@opentelemetry/semantic-conventions": "^1.29.0"
      },
      "engines": {
        "node": "^18.19.0 || >=20.6.0"
      },
      "peerDependencies": {
        "@opentelemetry/api": ">=1.3.0 <1.10.0"
      }
    },
    "node_modules/@opentelemetry/resources": {
      "version": "2.11.0",
      "resolved": "https://registry.npmjs.org/@opentelemetry/resources/-/resources-2.11.0.tgz",
      "integrity": "sha512-Ie7+8q8MDF4FAEQCKVMTx3ReUvxiIAgIiiW3c9JdmP8+HMcDy20puT+AHjexnExgnbvBxjQ9fjkFDWrikJ2jQA==",
      "license": "Apache-2.0",
      "dependencies": {
        "@opentelemetry/core": "2.11.0",
        "@opentelemetry/semantic-conventions": "^1.29.0"
      },
      "engines": {
        "node": "^18.19.0 || >=20.6.0"
      },
      "peerDependencies": {
        "@opentelemetry/api": ">=1.3.0 <1.10.0"
      }
    },
    "node_modules/@opentelemetry/resources/node_modules/@opentelemetry/core": {
      "version": "2.11.0",
      "resolved": "https://registry.npmjs.org/@opentelemetry/core/-/core-2.11.0.tgz",
      "integrity": "sha512-7YP44XH0tV6+Mb54x2YGf84i7yi+31MBZlE8JwvozkxyTvXbSp10X7cI7YE49ChJ3shMJoBmCJF3+1QFBJctGA==",
      "license": "Apache-2.0",
      "dependencies": {
        "@opentelemetry/semantic-conventions": "^1.29.0"
      },
      "engines": {
        "node": "^18.19.0 || >=20.6.0"
      },
      "peerDependencies": {
        "@opentelemetry/api": ">=1.0.0 <1.10.0"
      }
    },
    "node_modules/@opentelemetry/sdk-logs": {
      "version": "0.220.0",
      "resolved": "https://registry.npmjs.org/@opentelemetry/sdk-logs/-/sdk-logs-0.220.0.tgz",
      "integrity": "sha512-WywcTkQtv2iNmt+6y5Kcd4rzvx9bLVsBa2Nwcmg01IUaBTkTow3W4d9KE5vNBpEDtb9tp21WcRBY/lANRrApYA==",
      "license": "Apache-2.0",
      "dependencies": {
        "@opentelemetry/api-logs": "0.220.0",
        "@opentelemetry/core": "2.9.0",
        "@opentelemetry/resources": "2.9.0",
        "@opentelemetry/semantic-conventions": "^1.29.0"
      },
      "engines": {
        "node": "^18.19.0 || >=20.6.0"
      },
      "peerDependencies": {
        "@opentelemetry/api": ">=1.4.0 <1.10.0"
      }
    },
    "node_modules/@opentelemetry/sdk-logs/node_modules/@opentelemetry/resources": {
      "version": "2.9.0",
      "resolved": "https://registry.npmjs.org/@opentelemetry/resources/-/resources-2.9.0.tgz",
      "integrity": "sha512-jyA5MBLQ+Dkl3+JsZkUoUvL7yHvU64kLsvpXKarWm6347Sl1t1bXFTFykUePNpT5WH5pm9a2Qtt03iIYQhZ1Fg==",
      "license": "Apache-2.0",
      "dependencies": {
        "@opentelemetry/core": "2.9.0",
        "@opentelemetry/semantic-conventions": "^1.29.0"
      },
      "engines": {
        "node": "^18.19.0 || >=20.6.0"
      },
      "peerDependencies": {
        "@opentelemetry/api": ">=1.3.0 <1.10.0"
      }
    },
    "node_modules/@opentelemetry/sdk-metrics": {
      "version": "2.9.0",
      "resolved": "https://registry.npmjs.org/@opentelemetry/sdk-metrics/-/sdk-metrics-2.9.0.tgz",
      "integrity": "sha512-Xx8RGS4H5XEBl01WuCreMIpiah9cCXMbSkeuIePPdD2cUpq/vUzYmj8E/MK1OsbOc93FuAD4jfn2WOacKwLn7Q==",
      "license": "Apache-2.0",
      "dependencies": {
        "@opentelemetry/core": "2.9.0",
        "@opentelemetry/resources": "2.9.0"
      },
      "engines": {
        "node": "^18.19.0 || >=20.6.0"
      },
      "peerDependencies": {
        "@opentelemetry/api": ">=1.9.0 <1.10.0"
      }
    },
    "node_modules/@opentelemetry/sdk-metrics/node_modules/@opentelemetry/resources": {
      "version": "2.9.0",
      "resolved": "https://registry.npmjs.org/@opentelemetry/resources/-/resources-2.9.0.tgz",
      "integrity": "sha512-jyA5MBLQ+Dkl3+JsZkUoUvL7yHvU64kLsvpXKarWm6347Sl1t1bXFTFykUePNpT5WH5pm9a2Qtt03iIYQhZ1Fg==",
      "license": "Apache-2.0",
      "dependencies": {
        "@opentelemetry/core": "2.9.0",
        "@opentelemetry/semantic-conventions": "^1.29.0"
      },
      "engines": {
        "node": "^18.19.0 || >=20.6.0"
      },
      "peerDependencies": {
        "@opentelemetry/api": ">=1.3.0 <1.10.0"
      }
    },
    "node_modules/@opentelemetry/sdk-trace": {
      "version": "2.9.0",
      "resolved": "https://registry.npmjs.org/@opentelemetry/sdk-trace/-/sdk-trace-2.9.0.tgz",
      "integrity": "sha512-sGA19HvtrrSKYsseHphluH6j3p6Xa3fqc7c7y8f/7mYWejc1lyDFcpSdD1kYa50HCLUeEo4zA5bW0pniaPszuw==",
      "license": "Apache-2.0",
      "dependencies": {
        "@opentelemetry/core": "2.9.0",
        "@opentelemetry/resources": "2.9.0",
        "@opentelemetry/semantic-conventions": "^1.29.0"
      },
      "engines": {
        "node": "^18.19.0 || >=20.6.0"
      },
      "peerDependencies": {
        "@opentelemetry/api": ">=1.3.0 <1.10.0"
      }
    },
    "node_modules/@opentelemetry/sdk-trace/node_modules/@opentelemetry/resources": {
      "version": "2.9.0",
      "resolved": "https://registry.npmjs.org/@opentelemetry/resources/-/resources-2.9.0.tgz",
      "integrity": "sha512-jyA5MBLQ+Dkl3+JsZkUoUvL7yHvU64kLsvpXKarWm6347Sl1t1bXFTFykUePNpT5WH5pm9a2Qtt03iIYQhZ1Fg==",
      "license": "Apache-2.0",
      "dependencies": {
        "@opentelemetry/core": "2.9.0",
        "@opentelemetry/semantic-conventions": "^1.29.0"
      },
      "engines": {
        "node": "^18.19.0 || >=20.6.0"
      },
      "peerDependencies": {
        "@opentelemetry/api": ">=1.3.0 <1.10.0"
      }
    },
    "node_modules/@opentelemetry/semantic-conventions": {
      "version": "1.43.0",
      "resolved": "https://registry.npmjs.org/@opentelemetry/semantic-conventions/-/semantic-conventions-1.43.0.tgz",
      "integrity": "sha512-eSYWTm620tTk45EKSedaUL8MFYI8hW164hIXsgIHyxu3VobUB3fFCu5t0hQby6OoWRPsG1KkKUG2M5UadiLiVg==",
      "license": "Apache-2.0",
      "engines": {
        "node": ">=14"
      }
    },
    "node_modules/@protobufjs/aspromise": {
      "version": "1.1.2",
      "resolved": "https://registry.npmjs.org/@protobufjs/aspromise/-/aspromise-1.1.2.tgz",
      "integrity": "sha512-j+gKExEuLmKwvz3OgROXtrJ2UG2x8Ch2YZUxahh+s1F2HZ+wAceUNLkvy6zKCPVRkU++ZWQrdxsUeQXmcg4uoQ==",
      "license": "BSD-3-Clause"
    },
    "node_modules/@protobufjs/base64": {
      "version": "1.1.2",
      "resolved": "https://registry.npmjs.org/@protobufjs/base64/-/base64-1.1.2.tgz",
      "integrity": "sha512-AZkcAA5vnN/v4PDqKyMR5lx7hZttPDgClv83E//FMNhR2TMcLUhfRUBHCmSl0oi9zMgDDqRUJkSxO3wm85+XLg==",
      "license": "BSD-3-Clause"
    },
    "node_modules/@protobufjs/codegen": {
      "version": "2.0.5",
      "resolved": "https://registry.npmjs.org/@protobufjs/codegen/-/codegen-2.0.5.tgz",
      "integrity": "sha512-zgXFLzW3Ap33e6d0Wlj4MGIm6Ce8O89n/apUaGNB/jx+hw+ruWEp7EwGUshdLKVRCxZW12fp9r40E1mQrf/34g==",
      "license": "BSD-3-Clause"
    },
    "node_modules/@protobufjs/eventemitter": {
      "version": "1.1.1",
      "resolved": "https://registry.npmjs.org/@protobufjs/eventemitter/-/eventemitter-1.1.1.tgz",
      "integrity": "sha512-vW1GmwMZNnL+gMRaovlh9yZX74kc+TTU3FObkkurpMaRtBfLP3ldjS9KQWlwZgraRE0+dheEEoAxdzcJQ8eXZg==",
      "license": "BSD-3-Clause"
    },
    "node_modules/@protobufjs/fetch": {
      "version": "1.1.1",
      "resolved": "https://registry.npmjs.org/@protobufjs/fetch/-/fetch-1.1.1.tgz",
      "integrity": "sha512-GpptLrs57adMSuHi3VNj0mAF8dwh36LMaYF6XyJ6JMWlVsc+t42tm1HSEDmOs3A8fC9yyeisgLhsTVQokOZ0zw==",
      "license": "BSD-3-Clause",
      "dependencies": {
        "@protobufjs/aspromise": "^1.1.1"
      }
    },
    "node_modules/@protobufjs/float": {
      "version": "1.0.2",
      "resolved": "https://registry.npmjs.org/@protobufjs/float/-/float-1.0.2.tgz",
      "integrity": "sha512-Ddb+kVXlXst9d+R9PfTIxh1EdNkgoRe5tOX6t01f1lYWOvJnSPDBlG241QLzcyPdoNTsblLUdujGSE4RzrTZGQ==",
      "license": "BSD-3-Clause"
    },
    "node_modules/@protobufjs/path": {
      "version": "1.1.2",
      "resolved": "https://registry.npmjs.org/@protobufjs/path/-/path-1.1.2.tgz",
      "integrity": "sha512-6JOcJ5Tm08dOHAbdR3GrvP+yUUfkjG5ePsHYczMFLq3ZmMkAD98cDgcT2iA1lJ9NVwFd4tH/iSSoe44YWkltEA==",
      "license": "BSD-3-Clause"
    },
    "node_modules/@protobufjs/pool": {
      "version": "1.1.0",
      "resolved": "https://registry.npmjs.org/@protobufjs/pool/-/pool-1.1.0.tgz",
      "integrity": "sha512-0kELaGSIDBKvcgS4zkjz1PeddatrjYcmMWOlAuAPwAeccUrPHdUqo/J6LiymHHEiJT5NrF1UVwxY14f+fy4WQw==",
      "license": "BSD-3-Clause"
    },
    "node_modules/@protobufjs/utf8": {
      "version": "1.1.2",
      "resolved": "https://registry.npmjs.org/@protobufjs/utf8/-/utf8-1.1.2.tgz",
      "integrity": "sha512-b1UQwcEZ4yCnMCD8DAL1VlbvBJE9/IX4FTIp7BG1xYpf29SLazLSrqUkj4w7Y5y7cCVP6E5tcqqcI0xemPkHug==",
      "license": "BSD-3-Clause"
    },
    "node_modules/@smithy/core": {
      "version": "3.34.1",
      "resolved": "https://registry.npmjs.org/@smithy/core/-/core-3.34.1.tgz",
      "integrity": "sha512-dLcOUxz8YCv1RZUMKq6GbyUf95pLbrqh34bPvpCZ1+CByFF31BEAFewZjsGCnVsZTKdThNENfGyAgk2TJqVwSw==",
      "license": "Apache-2.0",
      "dependencies": {
        "@smithy/types": "^4.18.0",
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=18.0.0"
      }
    },
    "node_modules/@smithy/credential-provider-imds": {
      "version": "4.5.2",
      "resolved": "https://registry.npmjs.org/@smithy/credential-provider-imds/-/credential-provider-imds-4.5.2.tgz",
      "integrity": "sha512-A9uSdn72ozbRUSit0eib0TW7nXuNPlaeM0zcGkJ+nE6tFcSDbnmtwoxbTCFBukVQcszDAyvsd7+rTduPTXpygg==",
      "license": "Apache-2.0",
      "dependencies": {
        "@smithy/core": "^3.33.2",
        "@smithy/types": "^4.17.2",
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=18.0.0"
      }
    },
    "node_modules/@smithy/fetch-http-handler": {
      "version": "5.8.0",
      "resolved": "https://registry.npmjs.org/@smithy/fetch-http-handler/-/fetch-http-handler-5.8.0.tgz",
      "integrity": "sha512-ycSJu3tFAQ4v04CBB0agqFMVsSQ1iG3yw+SpgxRqKfaURpQD4CZ8Wn0zPMmSnOuTpTh65Vz+EA0rMrw089wvkA==",
      "license": "Apache-2.0",
      "dependencies": {
        "@smithy/core": "^3.33.3",
        "@smithy/types": "^4.18.0",
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=18.0.0"
      }
    },
    "node_modules/@smithy/is-array-buffer": {
      "version": "2.2.0",
      "resolved": "https://registry.npmjs.org/@smithy/is-array-buffer/-/is-array-buffer-2.2.0.tgz",
      "integrity": "sha512-GGP3O9QFD24uGeAXYUjwSTXARoqpZykHadOmA8G5vfJPK0/DC67qa//0qvqrJzL1xc8WQWX7/yc7fwudjPHPhA==",
      "license": "Apache-2.0",
      "dependencies": {
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=14.0.0"
      }
    },
    "node_modules/@smithy/node-http-handler": {
      "version": "4.7.3",
      "resolved": "https://registry.npmjs.org/@smithy/node-http-handler/-/node-http-handler-4.7.3.tgz",
      "integrity": "sha512-/jPhevcTFPMVl6KNjbaI47iOg1zxC7IsnX4PQDGVZKMFceOXtB8IEYaB7a9VvkP/3oC60WzTeKocvSI7vLT0vA==",
      "license": "Apache-2.0",
      "dependencies": {
        "@smithy/core": "^3.24.3",
        "@smithy/types": "^4.14.2",
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=18.0.0"
      }
    },
    "node_modules/@smithy/signature-v4": {
      "version": "5.7.3",
      "resolved": "https://registry.npmjs.org/@smithy/signature-v4/-/signature-v4-5.7.3.tgz",
      "integrity": "sha512-7ImGm+FkHRLcBaRttIAMZ6bzJZWb2cJGoYjq46F2UjycujWzrL9GEN9h4w7eQyXJYnltrUhxbbieBAIRrdqpow==",
      "license": "Apache-2.0",
      "dependencies": {
        "@smithy/core": "^3.33.3",
        "@smithy/types": "^4.17.2",
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=18.0.0"
      }
    },
    "node_modules/@smithy/types": {
      "version": "4.18.0",
      "resolved": "https://registry.npmjs.org/@smithy/types/-/types-4.18.0.tgz",
      "integrity": "sha512-CgB6HHWer/vrKps24ulRIbpcpb7K4xAU7SkZ7YHzBPlwHsvsrCJFEXK421s+cJzX+ZrqtA/TuU5w1HzI7k9N8A==",
      "license": "Apache-2.0",
      "dependencies": {
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=18.0.0"
      }
    },
    "node_modules/@smithy/util-buffer-from": {
      "version": "2.2.0",
      "resolved": "https://registry.npmjs.org/@smithy/util-buffer-from/-/util-buffer-from-2.2.0.tgz",
      "integrity": "sha512-IJdWBbTcMQ6DA0gdNhh/BwrLkDR+ADW5Kr1aZmd4k3DIF6ezMV4R2NIAmT08wQJ3yUK82thHWmC/TnK/wpMMIA==",
      "license": "Apache-2.0",
      "dependencies": {
        "@smithy/is-array-buffer": "^2.2.0",
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=14.0.0"
      }
    },
    "node_modules/@smithy/util-utf8": {
      "version": "2.3.0",
      "resolved": "https://registry.npmjs.org/@smithy/util-utf8/-/util-utf8-2.3.0.tgz",
      "integrity": "sha512-R8Rdn8Hy72KKcebgLiv8jQcQkXoLMOGGv5uI1/k0l+snqkOzQ1R0ChUBCxWMlBsFMekWjq0wRudIweFs7sKT5A==",
      "license": "Apache-2.0",
      "dependencies": {
        "@smithy/util-buffer-from": "^2.2.0",
        "tslib": "^2.6.2"
      },
      "engines": {
        "node": ">=14.0.0"
      }
    },
    "node_modules/@stablelib/base64": {
      "version": "1.0.1",
      "resolved": "https://registry.npmjs.org/@stablelib/base64/-/base64-1.0.1.tgz",
      "integrity": "sha512-1bnPQqSxSuc3Ii6MhBysoWCg58j97aUjuCSZrGSmDxNqtytIi0k8utUenAwTZN4V5mXXYGsVUI9zeBqy+jBOSQ==",
      "license": "MIT"
    },
    "node_modules/@standard-schema/spec": {
      "version": "1.1.0",
      "resolved": "https://registry.npmjs.org/@standard-schema/spec/-/spec-1.1.0.tgz",
      "integrity": "sha512-l2aFy5jALhniG5HgqrD6jXLi/rUWrKvqN/qJx6yoJsgKhblVd+iqqU4RCXavm/jPityDo5TCvKMnpjKnOriy0w==",
      "license": "MIT"
    },
    "node_modules/@types/node": {
      "version": "26.6.2",
      "resolved": "https://registry.npmjs.org/@types/node/-/node-26.6.2.tgz",
      "integrity": "sha512-X1P21scMv4zGKLYqjdGjaKa7COa0RKVYYZZN/NfvLQ1JegxFhdhpZG/Lyn8AXx6CDUavKAd11v6BvfpkDByK8g==",
      "license": "MIT",
      "dependencies": {
        "undici-types": "~8.9.0"
      }
    },
    "node_modules/@types/retry": {
      "version": "0.12.0",
      "resolved": "https://registry.npmjs.org/@types/retry/-/retry-0.12.0.tgz",
      "integrity": "sha512-wWKOClTTiizcZhXnPY4wikVAwmdYHp8q6DmC+EJUzAMsycb7HB32Kh9RN4+0gExjmPmZSAQjgURXIGATPegAvA==",
      "license": "MIT"
    },
    "node_modules/@vscode/ripgrep": {
      "version": "1.18.0",
      "resolved": "https://registry.npmjs.org/@vscode/ripgrep/-/ripgrep-1.18.0.tgz",
      "integrity": "sha512-ns5lWe44tSfbTMbVUsyB+I1819PVSw4AdpgK0RNkzfWfwy6+3IUNSxwSrfTno1/oWaS/hERNz+XLWVyga2aJBQ==",
      "license": "MIT",
      "optionalDependencies": {
        "@vscode/ripgrep-darwin-arm64": "1.18.0",
        "@vscode/ripgrep-darwin-x64": "1.18.0",
        "@vscode/ripgrep-linux-arm": "1.18.0",
        "@vscode/ripgrep-linux-arm64": "1.18.0",
        "@vscode/ripgrep-linux-ia32": "1.18.0",
        "@vscode/ripgrep-linux-ppc64": "1.18.0",
        "@vscode/ripgrep-linux-riscv64": "1.18.0",
        "@vscode/ripgrep-linux-s390x": "1.18.0",
        "@vscode/ripgrep-linux-x64": "1.18.0",
        "@vscode/ripgrep-win32-arm64": "1.18.0",
        "@vscode/ripgrep-win32-ia32": "1.18.0",
        "@vscode/ripgrep-win32-x64": "1.18.0"
      }
    },
    "node_modules/@vscode/ripgrep-linux-x64": {
      "version": "1.18.0",
      "resolved": "https://registry.npmjs.org/@vscode/ripgrep-linux-x64/-/ripgrep-linux-x64-1.18.0.tgz",
      "integrity": "sha512-mQ3bVrUpnD2vs7QT0vX90Lt0cnUq467uFtEktIdsJJmW296RoSULRGqWgzG1AKxyBpNDD6l4ZO4qKf6SgyC23Q==",
      "cpu": [
        "x64"
      ],
      "license": "MIT",
      "optional": true,
      "os": [
        "linux"
      ]
    },
    "node_modules/@xterm/headless": {
      "version": "6.0.0",
      "resolved": "https://registry.npmjs.org/@xterm/headless/-/headless-6.0.0.tgz",
      "integrity": "sha512-5Yj1QINYCyzrZtf8OFIHi47iQtI+0qYFPHmouEfG8dHNxbZ9Tb9YGSuLcsEwj9Z+OL75GJqPyJbyoFer80a2Hw==",
      "license": "MIT",
      "workspaces": [
        "addons/*"
      ]
    },
    "node_modules/accepts": {
      "version": "2.0.0",
      "resolved": "https://registry.npmjs.org/accepts/-/accepts-2.0.0.tgz",
      "integrity": "sha512-5cvg6CtKwfgdmVqY1WIiXKc3Q1bkRqGLi+2W/6ao+6Y7gu/RCwRuAhGEzh5B4KlszSuTLgZYuqFqo5bImjNKng==",
      "license": "MIT",
      "dependencies": {
        "mime-types": "^3.0.0",
        "negotiator": "^1.0.0"
      },
      "engines": {
        "node": ">= 0.6"
      }
    },
    "node_modules/agent-base": {
      "version": "7.1.4",
      "resolved": "https://registry.npmjs.org/agent-base/-/agent-base-7.1.4.tgz",
      "integrity": "sha512-MnA+YT8fwfJPgBx3m60MNqakm30XOkyIoH1y6huTQvC0PwZG7ki8NacLBcrPbNoo8vEZy7Jpuk7+jMO+CUovTQ==",
      "license": "MIT",
      "engines": {
        "node": ">= 14"
      }
    },
    "node_modules/ajv": {
      "version": "8.20.0",
      "resolved": "https://registry.npmjs.org/ajv/-/ajv-8.20.0.tgz",
      "integrity": "sha512-Thbli+OlOj+iMPYFBVBfJ3OmCAnaSyNn4M1vz9T6Gka5Jt9ba/HIR56joy65tY6kx/FCF5VXNB819Y7/GUrBGA==",
      "license": "MIT",
      "dependencies": {
        "fast-deep-equal": "^3.1.3",
        "fast-uri": "^3.0.1",
        "json-schema-traverse": "^1.0.0",
        "require-from-string": "^2.0.2"
      },
      "funding": {
        "type": "github",
        "url": "https://github.com/sponsors/epoberezkin"
      }
    },
    "node_modules/ajv-formats": {
      "version": "3.0.1",
      "resolved": "https://registry.npmjs.org/ajv-formats/-/ajv-formats-3.0.1.tgz",
      "integrity": "sha512-8iUql50EUR+uUcdRQ3HDqa6EVyo3docL8g5WJ3FNcWmu62IbkGUue/pEyLBW8VGKKucTPgqeks4fIU1DA4yowQ==",
      "license": "MIT",
      "dependencies": {
        "ajv": "^8.0.0"
      },
      "peerDependencies": {
        "ajv": "^8.0.0"
      },
      "peerDependenciesMeta": {
        "ajv": {
          "optional": true
        }
      }
    },
    "node_modules/argparse": {
      "version": "2.0.1",
      "resolved": "https://registry.npmjs.org/argparse/-/argparse-2.0.1.tgz",
      "integrity": "sha512-8+9WqebbFzpX9OR+Wa6O29asIogeRMzcGtAINdpMHHyAg10f05aSFVBbcEqGf/PXw1EjAZ+q2/bEBg3DvurK3Q==",
      "license": "Python-2.0"
    },
    "node_modules/base64-js": {
      "version": "1.5.1",
      "resolved": "https://registry.npmjs.org/base64-js/-/base64-js-1.5.1.tgz",
      "integrity": "sha512-AKpaYlHn8t4SVbOHCy+b5+KKgvR4vrsD8vbvrbiQJps7fKDTkjkDry6ji0rUJjC0kzbNePLwzxq8iypo41qeWA==",
      "funding": [
        {
          "type": "github",
          "url": "https://github.com/sponsors/feross"
        },
        {
          "type": "patreon",
          "url": "https://www.patreon.com/feross"
        },
        {
          "type": "consulting",
          "url": "https://feross.org/support"
        }
      ],
      "license": "MIT"
    },
    "node_modules/bignumber.js": {
      "version": "9.3.1",
      "resolved": "https://registry.npmjs.org/bignumber.js/-/bignumber.js-9.3.1.tgz",
      "integrity": "sha512-Ko0uX15oIUS7wJ3Rb30Fs6SkVbLmPBAKdlm7q9+ak9bbIeFf0MwuBsQV6z7+X768/cHsfg+WlysDWJcmthjsjQ==",
      "license": "MIT",
      "engines": {
        "node": "*"
      }
    },
    "node_modules/body-parser": {
      "version": "2.3.0",
      "resolved": "https://registry.npmjs.org/body-parser/-/body-parser-2.3.0.tgz",
      "integrity": "sha512-2cGmJupaNgg+QUwVLAucDuWuoMZ6EX9iHDRswZ5lsNYEmwPaRknMPCLZz07yTzVq/83p4o/wzbDZbBrTvGGTIw==",
      "license": "MIT",
      "dependencies": {
        "bytes": "^3.1.2",
        "content-type": "^2.0.0",
        "debug": "^4.4.3",
        "http-errors": "^2.0.1",
        "iconv-lite": "^0.7.2",
        "on-finished": "^2.4.1",
        "qs": "^6.15.2",
        "raw-body": "^3.0.2",
        "type-is": "^2.1.0"
      },
      "engines": {
        "node": ">=18"
      },
      "funding": {
        "type": "opencollective",
        "url": "https://opencollective.com/express"
      }
    },
    "node_modules/body-parser/node_modules/content-type": {
      "version": "2.1.0",
      "resolved": "https://registry.npmjs.org/content-type/-/content-type-2.1.0.tgz",
      "integrity": "sha512-mj7UPXE0jaqaOsukNZRUEfEi2AcL7C/vwmwcHV0O97eO1E1pxBZuyjlZrx5seTaNBg1U6+o35wpa35Qfcc+7ag==",
      "license": "MIT",
      "engines": {
        "node": ">=18"
      },
      "funding": {
        "type": "opencollective",
        "url": "https://opencollective.com/express"
      }
    },
    "node_modules/body-parser/node_modules/debug": {
      "version": "4.4.3",
      "resolved": "https://registry.npmjs.org/debug/-/debug-4.4.3.tgz",
      "integrity": "sha512-RGwwWnwQvkVfavKVt22FGLw+xYSdzARwm0ru6DhTVA3umU5hZc28V3kO4stgYryrTlLpuvgI9GiijltAjNbcqA==",
      "license": "MIT",
      "dependencies": {
        "ms": "^2.1.3"
      },
      "engines": {
        "node": ">=6.0"
      },
      "peerDependenciesMeta": {
        "supports-color": {
          "optional": true
        }
      }
    },
    "node_modules/body-parser/node_modules/ms": {
      "version": "2.1.3",
      "resolved": "https://registry.npmjs.org/ms/-/ms-2.1.3.tgz",
      "integrity": "sha512-6FlzubTLZG3J2a/NVCAleEhjzq5oxgHyaCU9yYXvcLsvoVaHJq/s5xXI6/XXP6tz7R9xAOtHnSO/tXtF3WRTlA==",
      "license": "MIT"
    },
    "node_modules/bowser": {
      "version": "2.14.1",
      "resolved": "https://registry.npmjs.org/bowser/-/bowser-2.14.1.tgz",
      "integrity": "sha512-tzPjzCxygAKWFOJP011oxFHs57HzIhOEracIgAePE4pqB3LikALKnSzUyU4MGs9/iCEUuHlAJTjTc5M+u7YEGg==",
      "license": "MIT"
    },
    "node_modules/buffer-equal-constant-time": {
      "version": "1.0.1",
      "resolved": "https://registry.npmjs.org/buffer-equal-constant-time/-/buffer-equal-constant-time-1.0.1.tgz",
      "integrity": "sha512-zRpUiDwd/xk6ADqPMATG8vc9VPrkck7T07OIx0gnjmJAnHnTVXNQG3vfvWNuiZIkwu9KrKdA1iJKfsfTVxE6NA==",
      "license": "BSD-3-Clause"
    },
    "node_modules/bundle-name": {
      "version": "4.1.0",
      "resolved": "https://registry.npmjs.org/bundle-name/-/bundle-name-4.1.0.tgz",
      "integrity": "sha512-tjwM5exMg6BGRI+kNmTntNsvdZS1X8BFYS6tnJ2hdH0kVxM6/eVZ2xy+FqStSWvYmtfFMDLIxurorHwDKfDz5Q==",
      "license": "MIT",
      "dependencies": {
        "run-applescript": "^7.0.0"
      },
      "engines": {
        "node": ">=18"
      },
      "funding": {
        "url": "https://github.com/sponsors/sindresorhus"
      }
    },
    "node_modules/bytes": {
      "version": "3.1.2",
      "resolved": "https://registry.npmjs.org/bytes/-/bytes-3.1.2.tgz",
      "integrity": "sha512-/Nf7TyzTx6S3yRJObOAV7956r8cr2+Oj8AC5dt8wSP3BQAoeX58NoHyCU8P8zGkNXStjTSi6fzO6F0pBdcYbEg==",
      "license": "MIT",
      "engines": {
        "node": ">= 0.8"
      }
    },
    "node_modules/call-bind-apply-helpers": {
      "version": "1.0.2",
      "resolved": "https://registry.npmjs.org/call-bind-apply-helpers/-/call-bind-apply-helpers-1.0.2.tgz",
      "integrity": "sha512-Sp1ablJ0ivDkSzjcaJdxEunN5/XvksFJ2sMBFfq6x0ryhQV/2b/KwFe21cMpmHtPOSij8K99/wSfoEuTObmuMQ==",
      "license": "MIT",
      "dependencies": {
        "es-errors": "^1.3.0",
        "function-bind": "^1.1.2"
      },
      "engines": {
        "node": ">= 0.4"
      }
    },
    "node_modules/call-bound": {
      "version": "1.0.4",
      "resolved": "https://registry.npmjs.org/call-bound/-/call-bound-1.0.4.tgz",
      "integrity": "sha512-+ys997U96po4Kx/ABpBCqhA9EuxJaQWDQg7295H4hBphv3IZg0boBKuwYpt4YXp6MZ5AmZQnU/tyMTlRpaSejg==",
      "license": "MIT",
      "dependencies": {
        "call-bind-apply-helpers": "^1.0.2",
        "get-intrinsic": "^1.3.0"
      },
      "engines": {
        "node": ">= 0.4"
      },
      "funding": {
        "url": "https://github.com/sponsors/ljharb"
      }
    },
    "node_modules/chokidar": {
      "version": "4.0.3",
      "resolved": "https://registry.npmjs.org/chokidar/-/chokidar-4.0.3.tgz",
      "integrity": "sha512-Qgzu8kfBvo+cA4962jnP1KkS6Dop5NS6g7R5LFYJr4b8Ub94PPQXUksCw9PvXoeXPRRddRNC5C1JQUR2SMGtnA==",
      "license": "MIT",
      "dependencies": {
        "readdirp": "^4.0.1"
      },
      "engines": {
        "node": ">= 14.16.0"
      },
      "funding": {
        "url": "https://paulmillr.com/funding/"
      }
    },
    "node_modules/commander": {
      "version": "15.0.0",
      "resolved": "https://registry.npmjs.org/commander/-/commander-15.0.0.tgz",
      "integrity": "sha512-z67u4ZhzCL/Tydu1lJARtEZYWbWaN7oYLHbsuzocr6y4N6WZAagG3RQ4FW61V1/0+jImpj293XfrcYnd1qxtPg==",
      "license": "MIT",
      "engines": {
        "node": ">=22.12.0"
      }
    },
    "node_modules/compressible": {
      "version": "2.0.18",
      "resolved": "https://registry.npmjs.org/compressible/-/compressible-2.0.18.tgz",
      "integrity": "sha512-AF3r7P5dWxL8MxyITRMlORQNaOA2IkAFaTr4k7BUumjPtRpGDTZpl0Pb1XCO6JeDCBdp126Cgs9sMxqSjgYyRg==",
      "license": "MIT",
      "dependencies": {
        "mime-db": ">= 1.43.0 < 2"
      },
      "engines": {
        "node": ">= 0.6"
      }
    },
    "node_modules/compression": {
      "version": "1.8.2",
      "resolved": "https://registry.npmjs.org/compression/-/compression-1.8.2.tgz",
      "integrity": "sha512-o8vI5RE5A6EVVOd9o41jKp41aJom+QTEO/Bx8MYNjexMo/Bv2WOjUfZr+aL0WnYSgymUy6zeguqLTsIhV0gMvQ==",
      "license": "MIT",
      "dependencies": {
        "bytes": "3.1.2",
        "compressible": "~2.0.18",
        "debug": "2.6.9",
        "destroy": "1.2.0",
        "negotiator": "~0.6.4",
        "on-headers": "~1.1.0",
        "safe-buffer": "5.2.1",
        "vary": "~1.1.2"
      },
      "engines": {
        "node": ">= 0.8.0"
      },
      "funding": {
        "type": "opencollective",
        "url": "https://opencollective.com/express"
      }
    },
    "node_modules/compression/node_modules/negotiator": {
      "version": "0.6.4",
      "resolved": "https://registry.npmjs.org/negotiator/-/negotiator-0.6.4.tgz",
      "integrity": "sha512-myRT3DiWPHqho5PrJaIRyaMv2kgYf0mUVgBNOYMuCH5Ki1yEiQaf/ZJuQ62nvpc44wL5WDbTX7yGJi1Neevw8w==",
      "license": "MIT",
      "engines": {
        "node": ">= 0.6"
      }
    },
    "node_modules/content-disposition": {
      "version": "1.1.0",
      "resolved": "https://registry.npmjs.org/content-disposition/-/content-disposition-1.1.0.tgz",
      "integrity": "sha512-5jRCH9Z/+DRP7rkvY83B+yGIGX96OYdJmzngqnw2SBSxqCFPd0w2km3s5iawpGX8krnwSGmF0FW5Nhr0Hfai3g==",
      "license": "MIT",
      "engines": {
        "node": ">=18"
      },
      "funding": {
        "type": "opencollective",
        "url": "https://opencollective.com/express"
      }
    },
    "node_modules/content-type": {
      "version": "1.0.5",
      "resolved": "https://registry.npmjs.org/content-type/-/content-type-1.0.5.tgz",
      "integrity": "sha512-nTjqfcBFEipKdXCv4YDQWCfmcLZKm81ldF0pAopTvyrFGVbcR6P/VAAd5G7N+0tTr8QqiU0tFadD6FK4NtJwOA==",
      "license": "MIT",
      "engines": {
        "node": ">= 0.6"
      }
    },
    "node_modules/cookie": {
      "version": "0.7.2",
      "resolved": "https://registry.npmjs.org/cookie/-/cookie-0.7.2.tgz",
      "integrity": "sha512-yki5XnKuf750l50uGTllt6kKILY4nQ1eNIQatoXEByZ5dWgnKqbnqmTrBE5B4N7lrMJKQ2ytWMiTO2o0v6Ew/w==",
      "license": "MIT",
      "engines": {
        "node": ">= 0.6"
      }
    },
    "node_modules/cookie-signature": {
      "version": "1.2.2",
      "resolved": "https://registry.npmjs.org/cookie-signature/-/cookie-signature-1.2.2.tgz",
      "integrity": "sha512-D76uU73ulSXrD1UXF4KE2TMxVVwhsnCgfAyTg9k8P6KGZjlXKrOLe4dJQKI3Bxi5wjesZoFXJWElNWBjPZMbhg==",
      "license": "MIT",
      "engines": {
        "node": ">=6.6.0"
      }
    },
    "node_modules/cors": {
      "version": "2.8.6",
      "resolved": "https://registry.npmjs.org/cors/-/cors-2.8.6.tgz",
      "integrity": "sha512-tJtZBBHA6vjIAaF6EnIaq6laBBP9aq/Y3ouVJjEfoHbRBcHBAHYcMh/w8LDrk2PvIMMq8gmopa5D4V8RmbrxGw==",
      "license": "MIT",
      "dependencies": {
        "object-assign": "^4",
        "vary": "^1"
      },
      "engines": {
        "node": ">= 0.10"
      },
      "funding": {
        "type": "opencollective",
        "url": "https://opencollective.com/express"
      }
    },
    "node_modules/cross-spawn": {
      "version": "7.0.6",
      "resolved": "https://registry.npmjs.org/cross-spawn/-/cross-spawn-7.0.6.tgz",
      "integrity": "sha512-uV2QOWP2nWzsy2aMp8aRibhi9dlzF5Hgh5SHaB9OiTGEyDTiJJyx0uy51QXdyWbtAHNua4XJzUKca3OzKUd3vA==",
      "license": "MIT",
      "dependencies": {
        "path-key": "^3.1.0",
        "shebang-command": "^2.0.0",
        "which": "^2.0.1"
      },
      "engines": {
        "node": ">= 8"
      }
    },
    "node_modules/data-uri-to-buffer": {
      "version": "4.0.1",
      "resolved": "https://registry.npmjs.org/data-uri-to-buffer/-/data-uri-to-buffer-4.0.1.tgz",
      "integrity": "sha512-0R9ikRb668HB7QDxT1vkpuUBtqc53YyAwMwGeUFKRojY/NWKvdZ+9UYtRfGmhqNbRkTSVpMbmyhXipFFv2cb/A==",
      "license": "MIT",
      "engines": {
        "node": ">= 12"
      }
    },
    "node_modules/debug": {
      "version": "2.6.9",
      "resolved": "https://registry.npmjs.org/debug/-/debug-2.6.9.tgz",
      "integrity": "sha512-bC7ElrdJaJnPbAP+1EotYvqZsb3ecl5wi6Bfi6BJTUcNowp6cvspg0jXznRTKDjm/E7AdgFBVeAPVMNcKGsHMA==",
      "license": "MIT",
      "dependencies": {
        "ms": "2.0.0"
      }
    },
    "node_modules/default-browser": {
      "version": "5.5.1",
      "resolved": "https://registry.npmjs.org/default-browser/-/default-browser-5.5.1.tgz",
      "integrity": "sha512-m1pAzaJgZ/gssEqlOhJkPJp8Xly7QyW6xcrkUa2KKcDeDSEMP7X8xipU3snUcfisTQx0w1AGae+9UtJSfVnXGw==",
      "license": "MIT",
      "dependencies": {
        "bundle-name": "^4.1.0",
        "default-browser-id": "^5.0.0"
      },
      "engines": {
        "node": ">=18"
      },
      "funding": {
        "url": "https://github.com/sponsors/sindresorhus"
      }
    },
    "node_modules/default-browser-id": {
      "version": "5.0.1",
      "resolved": "https://registry.npmjs.org/default-browser-id/-/default-browser-id-5.0.1.tgz",
      "integrity": "sha512-x1VCxdX4t+8wVfd1so/9w+vQ4vx7lKd2Qp5tDRutErwmR85OgmfX7RlLRMWafRMY7hbEiXIbudNrjOAPa/hL8Q==",
      "license": "MIT",
      "engines": {
        "node": ">=18"
      },
      "funding": {
        "url": "https://github.com/sponsors/sindresorhus"
      }
    },
    "node_modules/define-lazy-prop": {
      "version": "3.0.0",
      "resolved": "https://registry.npmjs.org/define-lazy-prop/-/define-lazy-prop-3.0.0.tgz",
      "integrity": "sha512-N+MeXYoqr3pOgn8xfyRPREN7gHakLYjhsHhWGT3fWAiL4IkAt0iDw14QiiEm2bE30c5XX5q0FtAA3CK5f9/BUg==",
      "license": "MIT",
      "engines": {
        "node": ">=12"
      },
      "funding": {
        "url": "https://github.com/sponsors/sindresorhus"
      }
    },
    "node_modules/depd": {
      "version": "2.0.0",
      "resolved": "https://registry.npmjs.org/depd/-/depd-2.0.0.tgz",
      "integrity": "sha512-g7nH6P6dyDioJogAAGprGpCtVImJhpPk/roCzdb3fIh61/s/nPsfR6onyMwkCAR/OlC3yBC0lESvUoQEAssIrw==",
      "license": "MIT",
      "engines": {
        "node": ">= 0.8"
      }
    },
    "node_modules/destroy": {
      "version": "1.2.0",
      "resolved": "https://registry.npmjs.org/destroy/-/destroy-1.2.0.tgz",
      "integrity": "sha512-2sJGJTaXIIaR1w4iJSNoN0hnMY7Gpc/n8D4qSCJw8QqFWXf7cuAgnEHxBpweaVcPevC2l3KpjYCx3NypQQgaJg==",
      "license": "MIT",
      "engines": {
        "node": ">= 0.8",
        "npm": "1.2.8000 || >= 1.4.16"
      }
    },
    "node_modules/detect-libc": {
      "version": "2.1.2",
      "resolved": "https://registry.npmjs.org/detect-libc/-/detect-libc-2.1.2.tgz",
      "integrity": "sha512-Btj2BOOO83o3WyH59e8MgXsxEQVcarkUOpEYrubB0urwnN10yQ364rsiByU11nZlqWYZm05i/of7io4mzihBtQ==",
      "license": "Apache-2.0",
      "engines": {
        "node": ">=8"
      }
    },
    "node_modules/diff": {
      "version": "9.0.0",
      "resolved": "https://registry.npmjs.org/diff/-/diff-9.0.0.tgz",
      "integrity": "sha512-svtcdpS8CgJyqAjEQIXdb3OjhFVVYjzGAPO8WGCmRbrml64SPw/jJD4GoE98aR7r25A0XcgrK3F02yw9R/vhQw==",
      "license": "BSD-3-Clause",
      "engines": {
        "node": ">=0.3.1"
      }
    },
    "node_modules/dunder-proto": {
      "version": "1.0.1",
      "resolved": "https://registry.npmjs.org/dunder-proto/-/dunder-proto-1.0.1.tgz",
      "integrity": "sha512-KIN/nDJBQRcXw0MLVhZE9iQHmG68qAVIBg9CqmUYjmQIhgij9U5MFvrqkUL5FbtyyzZuOeOt0zdeRe4UY7ct+A==",
      "license": "MIT",
      "dependencies": {
        "call-bind-apply-helpers": "^1.0.1",
        "es-errors": "^1.3.0",
        "gopd": "^1.2.0"
      },
      "engines": {
        "node": ">= 0.4"
      }
    },
    "node_modules/ecdsa-sig-formatter": {
      "version": "1.0.11",
      "resolved": "https://registry.npmjs.org/ecdsa-sig-formatter/-/ecdsa-sig-formatter-1.0.11.tgz",
      "integrity": "sha512-nagl3RYrbNv6kQkeJIpt6NJZy8twLB/2vtz6yN9Z4vRKHN4/QZJIEbqohALSgwKdnksuY3k5Addp5lg8sVoVcQ==",
      "license": "Apache-2.0",
      "dependencies": {
        "safe-buffer": "^5.0.1"
      }
    },
    "node_modules/ee-first": {
      "version": "1.1.1",
      "resolved": "https://registry.npmjs.org/ee-first/-/ee-first-1.1.1.tgz",
      "integrity": "sha512-WMwm9LhRUo+WUaRN+vRuETqG89IgZphVSNkdFgeb6sS/E4OrDIN7t48CAewSHXc6C8lefD8KKfr5vY61brQlow==",
      "license": "MIT"
    },
    "node_modules/encodeurl": {
      "version": "2.0.0",
      "resolved": "https://registry.npmjs.org/encodeurl/-/encodeurl-2.0.0.tgz",
      "integrity": "sha512-Q0n9HRi4m6JuGIV1eFlmvJB7ZEVxu93IrMyiMsGC0lrMJMWzRgx6WGquyfQgZVb31vhGgXnfmPNNXmxnOkRBrg==",
      "license": "MIT",
      "engines": {
        "node": ">= 0.8"
      }
    },
    "node_modules/es-define-property": {
      "version": "1.0.1",
      "resolved": "https://registry.npmjs.org/es-define-property/-/es-define-property-1.0.1.tgz",
      "integrity": "sha512-e3nRfgfUZ4rNGL232gUgX06QNyyez04KdjFrF+LTRoOXmrOgFKDg4BCdsjW8EnT69eqdYGmRpJwiPVYNrCaW3g==",
      "license": "MIT",
      "engines": {
        "node": ">= 0.4"
      }
    },
    "node_modules/es-errors": {
      "version": "1.3.0",
      "resolved": "https://registry.npmjs.org/es-errors/-/es-errors-1.3.0.tgz",
      "integrity": "sha512-Zf5H2Kxt2xjTvbJvP2ZWLEICxA6j+hAmMzIlypy4xcBg1vKVnx89Wy0GbS+kf5cwCVFFzdCFh2XSCFNULS6csw==",
      "license": "MIT",
      "engines": {
        "node": ">= 0.4"
      }
    },
    "node_modules/es-object-atoms": {
      "version": "1.1.2",
      "resolved": "https://registry.npmjs.org/es-object-atoms/-/es-object-atoms-1.1.2.tgz",
      "integrity": "sha512-HWcBoN6NileqtSydK2FqHbS/LoDd2pqrnQHLyJzBj4kOp/ky2MWMN694xOfkK8/SnUsW2DH7EfyVlydKCsm1Zw==",
      "license": "MIT",
      "dependencies": {
        "es-errors": "^1.3.0"
      },
      "engines": {
        "node": ">= 0.4"
      }
    },
    "node_modules/escape-html": {
      "version": "1.0.3",
      "resolved": "https://registry.npmjs.org/escape-html/-/escape-html-1.0.3.tgz",
      "integrity": "sha512-NiSupZ4OeuGwr68lGIeym/ksIZMJodUGOSCZ/FSnTxcrekbvqrgdUxlJOMpijaKZVjAJrWrGs/6Jy8OMuyj9ow==",
      "license": "MIT"
    },
    "node_modules/etag": {
      "version": "1.8.1",
      "resolved": "https://registry.npmjs.org/etag/-/etag-1.8.1.tgz",
      "integrity": "sha512-aIL5Fx7mawVa300al2BnEE4iNvo1qETxLrPI/o05L7z6go7fCw1J6EQmbK4FmJ2AS7kgVF/KEZWufBfdClMcPg==",
      "license": "MIT",
      "engines": {
        "node": ">= 0.6"
      }
    },
    "node_modules/eventsource": {
      "version": "3.0.7",
      "resolved": "https://registry.npmjs.org/eventsource/-/eventsource-3.0.7.tgz",
      "integrity": "sha512-CRT1WTyuQoD771GW56XEZFQ/ZoSfWid1alKGDYMmkt2yl8UXrVR4pspqWNEcqKvVIzg6PAltWjxcSSPrboA4iA==",
      "license": "MIT",
      "dependencies": {
        "eventsource-parser": "^3.0.1"
      },
      "engines": {
        "node": ">=18.0.0"
      }
    },
    "node_modules/eventsource-parser": {
      "version": "3.1.1",
      "resolved": "https://registry.npmjs.org/eventsource-parser/-/eventsource-parser-3.1.1.tgz",
      "integrity": "sha512-EKN1vKAMcZ8MlYMpaNuxN6R9yakzH6uajHcHVTqWJzvu5pWw9DyhbP35HH8MVBQ+dZjAfDxk+A8NiR9KWaXiyQ==",
      "license": "MIT",
      "engines": {
        "node": ">=18.0.0"
      }
    },
    "node_modules/express": {
      "version": "5.2.1",
      "resolved": "https://registry.npmjs.org/express/-/express-5.2.1.tgz",
      "integrity": "sha512-hIS4idWWai69NezIdRt2xFVofaF4j+6INOpJlVOLDO8zXGpUVEVzIYk12UUi2JzjEzWL3IOAxcTubgz9Po0yXw==",
      "license": "MIT",
      "dependencies": {
        "accepts": "^2.0.0",
        "body-parser": "^2.2.1",
        "content-disposition": "^1.0.0",
        "content-type": "^1.0.5",
        "cookie": "^0.7.1",
        "cookie-signature": "^1.2.1",
        "debug": "^4.4.0",
        "depd": "^2.0.0",
        "encodeurl": "^2.0.0",
        "escape-html": "^1.0.3",
        "etag": "^1.8.1",
        "finalhandler": "^2.1.0",
        "fresh": "^2.0.0",
        "http-errors": "^2.0.0",
        "merge-descriptors": "^2.0.0",
        "mime-types": "^3.0.0",
        "on-finished": "^2.4.1",
        "once": "^1.4.0",
        "parseurl": "^1.3.3",
        "proxy-addr": "^2.0.7",
        "qs": "^6.14.0",
        "range-parser": "^1.2.1",
        "router": "^2.2.0",
        "send": "^1.1.0",
        "serve-static": "^2.2.0",
        "statuses": "^2.0.1",
        "type-is": "^2.0.1",
        "vary": "^1.1.2"
      },
      "engines": {
        "node": ">= 18"
      },
      "funding": {
        "type": "opencollective",
        "url": "https://opencollective.com/express"
      }
    },
    "node_modules/express-rate-limit": {
      "version": "8.7.0",
      "resolved": "https://registry.npmjs.org/express-rate-limit/-/express-rate-limit-8.7.0.tgz",
      "integrity": "sha512-hOwV7WOxXfjRpAM1DSJWZDXx3GhplwD8IfwuwvogD8i1Qnkgosw/H45s4ZnFAUHDAhPjlY9hLBvJhKmGMyY26g==",
      "license": "MIT",
      "dependencies": {
        "debug": "^4.4.3",
        "ip-address": "^10.2.0"
      },
      "engines": {
        "node": ">= 16"
      },
      "funding": {
        "url": "https://github.com/sponsors/express-rate-limit"
      },
      "peerDependencies": {
        "express": ">= 4.11"
      }
    },
    "node_modules/express-rate-limit/node_modules/debug": {
      "version": "4.4.3",
      "resolved": "https://registry.npmjs.org/debug/-/debug-4.4.3.tgz",
      "integrity": "sha512-RGwwWnwQvkVfavKVt22FGLw+xYSdzARwm0ru6DhTVA3umU5hZc28V3kO4stgYryrTlLpuvgI9GiijltAjNbcqA==",
      "license": "MIT",
      "dependencies": {
        "ms": "^2.1.3"
      },
      "engines": {
        "node": ">=6.0"
      },
      "peerDependenciesMeta": {
        "supports-color": {
          "optional": true
        }
      }
    },
    "node_modules/express-rate-limit/node_modules/ms": {
      "version": "2.1.3",
      "resolved": "https://registry.npmjs.org/ms/-/ms-2.1.3.tgz",
      "integrity": "sha512-6FlzubTLZG3J2a/NVCAleEhjzq5oxgHyaCU9yYXvcLsvoVaHJq/s5xXI6/XXP6tz7R9xAOtHnSO/tXtF3WRTlA==",
      "license": "MIT"
    },
    "node_modules/express/node_modules/debug": {
      "version": "4.4.3",
      "resolved": "https://registry.npmjs.org/debug/-/debug-4.4.3.tgz",
      "integrity": "sha512-RGwwWnwQvkVfavKVt22FGLw+xYSdzARwm0ru6DhTVA3umU5hZc28V3kO4stgYryrTlLpuvgI9GiijltAjNbcqA==",
      "license": "MIT",
      "dependencies": {
        "ms": "^2.1.3"
      },
      "engines": {
        "node": ">=6.0"
      },
      "peerDependenciesMeta": {
        "supports-color": {
          "optional": true
        }
      }
    },
    "node_modules/express/node_modules/ms": {
      "version": "2.1.3",
      "resolved": "https://registry.npmjs.org/ms/-/ms-2.1.3.tgz",
      "integrity": "sha512-6FlzubTLZG3J2a/NVCAleEhjzq5oxgHyaCU9yYXvcLsvoVaHJq/s5xXI6/XXP6tz7R9xAOtHnSO/tXtF3WRTlA==",
      "license": "MIT"
    },
    "node_modules/extend": {
      "version": "3.0.2",
      "resolved": "https://registry.npmjs.org/extend/-/extend-3.0.2.tgz",
      "integrity": "sha512-fjquC59cD7CyW6urNXK0FBufkZcoiGG80wTuPujX590cB5Ttln20E2UB4S/WARVqhXffZl2LNgS+gQdPIIim/g==",
      "license": "MIT"
    },
    "node_modules/fast-deep-equal": {
      "version": "3.1.3",
      "resolved": "https://registry.npmjs.org/fast-deep-equal/-/fast-deep-equal-3.1.3.tgz",
      "integrity": "sha512-f3qQ9oQy9j2AhBe/H9VC91wLmKBCCU/gDOnKNAYG5hswO7BLKj09Hc5HYNz9cGI++xlpDCIgDaitVs03ATR84Q==",
      "license": "MIT"
    },
    "node_modules/fast-sha256": {
      "version": "1.3.0",
      "resolved": "https://registry.npmjs.org/fast-sha256/-/fast-sha256-1.3.0.tgz",
      "integrity": "sha512-n11RGP/lrWEFI/bWdygLxhI+pVeo1ZYIVwvvPkW7azl/rOy+F3HYRZ2K5zeE9mmkhQppyv9sQFx0JM9UabnpPQ==",
      "license": "Unlicense"
    },
    "node_modules/fast-uri": {
      "version": "3.1.8",
      "resolved": "https://registry.npmjs.org/fast-uri/-/fast-uri-3.1.8.tgz",
      "integrity": "sha512-GZMtZUTNRpOVIECoXwLNZS5xUGE+mVNbTB8h/7Rwh2TFWcBQiPzTgyZi05BF9UMZKkLJv8XBRJTlU7zg8+ZfMg==",
      "funding": [
        {
          "type": "github",
          "url": "https://github.com/sponsors/fastify"
        },
        {
          "type": "opencollective",
          "url": "https://opencollective.com/fastify"
        }
      ],
      "license": "BSD-3-Clause"
    },
    "node_modules/fetch-blob": {
      "version": "3.2.0",
      "resolved": "https://registry.npmjs.org/fetch-blob/-/fetch-blob-3.2.0.tgz",
      "integrity": "sha512-7yAQpD2UMJzLi1Dqv7qFYnPbaPx7ZfFK6PiIxQ4PfkGPyNyl2Ugx+a/umUonmKqjhM4DnfbMvdX6otXq83soQQ==",
      "funding": [
        {
          "type": "github",
          "url": "https://github.com/sponsors/jimmywarting"
        },
        {
          "type": "paypal",
          "url": "https://paypal.me/jimmywarting"
        }
      ],
      "license": "MIT",
      "dependencies": {
        "node-domexception": "^1.0.0",
        "web-streams-polyfill": "^3.0.3"
      },
      "engines": {
        "node": "^12.20 || >= 14.13"
      }
    },
    "node_modules/fflate": {
      "version": "0.8.3",
      "resolved": "https://registry.npmjs.org/fflate/-/fflate-0.8.3.tgz",
      "integrity": "sha512-tbZNuJrLwGUp3zshBtdy4W+ORxZuIh8a5ilyIEQDC5rY1f3U20JMry0Ll3WBzU58EZKsEuJFXhb5gwv8CsPvgA==",
      "license": "MIT"
    },
    "node_modules/finalhandler": {
      "version": "2.1.1",
      "resolved": "https://registry.npmjs.org/finalhandler/-/finalhandler-2.1.1.tgz",
      "integrity": "sha512-S8KoZgRZN+a5rNwqTxlZZePjT/4cnm0ROV70LedRHZ0p8u9fRID0hJUZQpkKLzro8LfmC8sx23bY6tVNxv8pQA==",
      "license": "MIT",
      "dependencies": {
        "debug": "^4.4.0",
        "encodeurl": "^2.0.0",
        "escape-html": "^1.0.3",
        "on-finished": "^2.4.1",
        "parseurl": "^1.3.3",
        "statuses": "^2.0.1"
      },
      "engines": {
        "node": ">= 18.0.0"
      },
      "funding": {
        "type": "opencollective",
        "url": "https://opencollective.com/express"
      }
    },
    "node_modules/finalhandler/node_modules/debug": {
      "version": "4.4.3",
      "resolved": "https://registry.npmjs.org/debug/-/debug-4.4.3.tgz",
      "integrity": "sha512-RGwwWnwQvkVfavKVt22FGLw+xYSdzARwm0ru6DhTVA3umU5hZc28V3kO4stgYryrTlLpuvgI9GiijltAjNbcqA==",
      "license": "MIT",
      "dependencies": {
        "ms": "^2.1.3"
      },
      "engines": {
        "node": ">=6.0"
      },
      "peerDependenciesMeta": {
        "supports-color": {
          "optional": true
        }
      }
    },
    "node_modules/finalhandler/node_modules/ms": {
      "version": "2.1.3",
      "resolved": "https://registry.npmjs.org/ms/-/ms-2.1.3.tgz",
      "integrity": "sha512-6FlzubTLZG3J2a/NVCAleEhjzq5oxgHyaCU9yYXvcLsvoVaHJq/s5xXI6/XXP6tz7R9xAOtHnSO/tXtF3WRTlA==",
      "license": "MIT"
    },
    "node_modules/formdata-polyfill": {
      "version": "4.0.10",
      "resolved": "https://registry.npmjs.org/formdata-polyfill/-/formdata-polyfill-4.0.10.tgz",
      "integrity": "sha512-buewHzMvYL29jdeQTVILecSaZKnt/RJWjoZCF5OW60Z67/GmSLBkOFM7qh1PI3zFNtJbaZL5eQu1vLfazOwj4g==",
      "license": "MIT",
      "dependencies": {
        "fetch-blob": "^3.1.2"
      },
      "engines": {
        "node": ">=12.20.0"
      }
    },
    "node_modules/forwarded": {
      "version": "0.2.0",
      "resolved": "https://registry.npmjs.org/forwarded/-/forwarded-0.2.0.tgz",
      "integrity": "sha512-buRG0fpBtRHSTCOASe6hD258tEubFoRLb4ZNA6NxMVHNw2gOcwHo9wyablzMzOA5z9xA9L1KNjk/Nt6MT9aYow==",
      "license": "MIT",
      "engines": {
        "node": ">= 0.6"
      }
    },
    "node_modules/fresh": {
      "version": "2.0.0",
      "resolved": "https://registry.npmjs.org/fresh/-/fresh-2.0.0.tgz",
      "integrity": "sha512-Rx/WycZ60HOaqLKAi6cHRKKI7zxWbJ31MhntmtwMoaTeF7XFH9hhBp8vITaMidfljRQ6eYWCKkaTK+ykVJHP2A==",
      "license": "MIT",
      "engines": {
        "node": ">= 0.8"
      }
    },
    "node_modules/function-bind": {
      "version": "1.1.2",
      "resolved": "https://registry.npmjs.org/function-bind/-/function-bind-1.1.2.tgz",
      "integrity": "sha512-7XHNxH7qX9xG5mIwxkhumTox/MIRNcOgDrxWsMt2pAr23WHp6MrRlN7FBSFpCpr+oVO0F744iUgR82nJMfG2SA==",
      "license": "MIT",
      "funding": {
        "url": "https://github.com/sponsors/ljharb"
      }
    },
    "node_modules/gaxios": {
      "version": "7.3.1",
      "resolved": "https://registry.npmjs.org/gaxios/-/gaxios-7.3.1.tgz",
      "integrity": "sha512-kB3rzJV7d9juLZh8/56QTXCwQfxyhdOMdyYk1HdQKFtF8TJTDTZQJtixWIwXdE9Jji91mC41DUNpjleo4L4eAQ==",
      "license": "Apache-2.0",
      "dependencies": {
        "extend": "^3.0.2",
        "https-proxy-agent": "^7.0.1",
        "node-fetch": "^3.3.2"
      },
      "engines": {
        "node": ">=18"
      }
    },
    "node_modules/gcp-metadata": {
      "version": "8.1.2",
      "resolved": "https://registry.npmjs.org/gcp-metadata/-/gcp-metadata-8.1.2.tgz",
      "integrity": "sha512-zV/5HKTfCeKWnxG0Dmrw51hEWFGfcF2xiXqcA3+J90WDuP0SvoiSO5ORvcBsifmx/FoIjgQN3oNOGaQ5PhLFkg==",
      "license": "Apache-2.0",
      "dependencies": {
        "gaxios": "^7.0.0",
        "google-logging-utils": "^1.0.0",
        "json-bigint": "^1.0.0"
      },
      "engines": {
        "node": ">=18"
      }
    },
    "node_modules/get-intrinsic": {
      "version": "1.3.0",
      "resolved": "https://registry.npmjs.org/get-intrinsic/-/get-intrinsic-1.3.0.tgz",
      "integrity": "sha512-9fSjSaos/fRIVIp+xSJlE6lfwhES7LNtKaCBIamHsjr2na1BiABJPo0mOjjz8GJDURarmCPGqaiVg5mfjb98CQ==",
      "license": "MIT",
      "dependencies": {
        "call-bind-apply-helpers": "^1.0.2",
        "es-define-property": "^1.0.1",
        "es-errors": "^1.3.0",
        "es-object-atoms": "^1.1.1",
        "function-bind": "^1.1.2",
        "get-proto": "^1.0.1",
        "gopd": "^1.2.0",
        "has-symbols": "^1.1.0",
        "hasown": "^2.0.2",
        "math-intrinsics": "^1.1.0"
      },
      "engines": {
        "node": ">= 0.4"
      },
      "funding": {
        "url": "https://github.com/sponsors/ljharb"
      }
    },
    "node_modules/get-proto": {
      "version": "1.0.1",
      "resolved": "https://registry.npmjs.org/get-proto/-/get-proto-1.0.1.tgz",
      "integrity": "sha512-sTSfBjoXBp89JvIKIefqw7U2CCebsc74kiY6awiGogKtoSGbgjYE/G/+l9sF3MWFPNc9IcoOC4ODfKHfxFmp0g==",
      "license": "MIT",
      "dependencies": {
        "dunder-proto": "^1.0.1",
        "es-object-atoms": "^1.0.0"
      },
      "engines": {
        "node": ">= 0.4"
      }
    },
    "node_modules/google-auth-library": {
      "version": "10.9.1",
      "resolved": "https://registry.npmjs.org/google-auth-library/-/google-auth-library-10.9.1.tgz",
      "integrity": "sha512-i1ydyHrqcIxXkWh/uBmVkzCvIuq5yiK2ATndIe5XxKholrG/MTYP9xGYka4sQhrbIAgGjL2B6NOE7rFaiF3fXw==",
      "license": "Apache-2.0",
      "dependencies": {
        "base64-js": "^1.3.0",
        "ecdsa-sig-formatter": "^1.0.11",
        "gaxios": "^7.1.4",
        "gcp-metadata": "8.1.2",
        "google-logging-utils": "1.1.3",
        "jws": "^4.0.0"
      },
      "engines": {
        "node": ">=18"
      }
    },
    "node_modules/google-logging-utils": {
      "version": "1.1.3",
      "resolved": "https://registry.npmjs.org/google-logging-utils/-/google-logging-utils-1.1.3.tgz",
      "integrity": "sha512-eAmLkjDjAFCVXg7A1unxHsLf961m6y17QFqXqAXGj/gVkKFrEICfStRfwUlGNfeCEjNRa32JEWOUTlYXPyyKvA==",
      "license": "Apache-2.0",
      "engines": {
        "node": ">=14"
      }
    },
    "node_modules/gopd": {
      "version": "1.2.0",
      "resolved": "https://registry.npmjs.org/gopd/-/gopd-1.2.0.tgz",
      "integrity": "sha512-ZUKRh6/kUFoAiTAtTYPZJ3hw9wNxx+BIBOijnlG9PnrJsCcSjs1wyyD6vJpaYtgnzDrKYRSqf3OO6Rfa93xsRg==",
      "license": "MIT",
      "engines": {
        "node": ">= 0.4"
      },
      "funding": {
        "url": "https://github.com/sponsors/ljharb"
      }
    },
    "node_modules/has-symbols": {
      "version": "1.1.0",
      "resolved": "https://registry.npmjs.org/has-symbols/-/has-symbols-1.1.0.tgz",
      "integrity": "sha512-1cDNdwJ2Jaohmb3sg4OmKaMBwuC48sYni5HUw2DvsC8LjGTLK9h+eb1X6RyuOHe4hT0ULCW68iomhjUoKUqlPQ==",
      "license": "MIT",
      "engines": {
        "node": ">= 0.4"
      },
      "funding": {
        "url": "https://github.com/sponsors/ljharb"
      }
    },
    "node_modules/hasown": {
      "version": "2.0.4",
      "resolved": "https://registry.npmjs.org/hasown/-/hasown-2.0.4.tgz",
      "integrity": "sha512-T2UbfbBEF32wiepXIsMlTW9+dDYC6wMh/t/vYA4tuOMKqWz/n3vr1NFSxQiyP+zk2mXsoMA/i/7qV6LKut1t1A==",
      "license": "MIT",
      "dependencies": {
        "function-bind": "^1.1.2"
      },
      "engines": {
        "node": ">= 0.4"
      }
    },
    "node_modules/hono": {
      "version": "4.13.8",
      "resolved": "https://registry.npmjs.org/hono/-/hono-4.13.8.tgz",
      "integrity": "sha512-/Gng7NfoykZl2pjukW5Z6+8Yxm3BPRf86GTbQnt0SbySkvax4fyL4H3HhY1cCpBGmiW9XDRFzRV+CXK2W8QudQ==",
      "license": "MIT",
      "engines": {
        "node": ">=16.9.0"
      }
    },
    "node_modules/http-errors": {
      "version": "2.0.1",
      "resolved": "https://registry.npmjs.org/http-errors/-/http-errors-2.0.1.tgz",
      "integrity": "sha512-4FbRdAX+bSdmo4AUFuS0WNiPz8NgFt+r8ThgNWmlrjQjt1Q7ZR9+zTlce2859x4KSXrwIsaeTqDoKQmtP8pLmQ==",
      "license": "MIT",
      "dependencies": {
        "depd": "~2.0.0",
        "inherits": "~2.0.4",
        "setprototypeof": "~1.2.0",
        "statuses": "~2.0.2",
        "toidentifier": "~1.0.1"
      },
      "engines": {
        "node": ">= 0.8"
      },
      "funding": {
        "type": "opencollective",
        "url": "https://opencollective.com/express"
      }
    },
    "node_modules/http-proxy-agent": {
      "version": "7.0.2",
      "resolved": "https://registry.npmjs.org/http-proxy-agent/-/http-proxy-agent-7.0.2.tgz",
      "integrity": "sha512-T1gkAiYYDWYx3V5Bmyu7HcfcvL7mUrTWiM6yOfa3PIphViJ/gFPbvidQ+veqSOHci/PxBcDabeUNCzpOODJZig==",
      "license": "MIT",
      "dependencies": {
        "agent-base": "^7.1.0",
        "debug": "^4.3.4"
      },
      "engines": {
        "node": ">= 14"
      }
    },
    "node_modules/http-proxy-agent/node_modules/debug": {
      "version": "4.4.3",
      "resolved": "https://registry.npmjs.org/debug/-/debug-4.4.3.tgz",
      "integrity": "sha512-RGwwWnwQvkVfavKVt22FGLw+xYSdzARwm0ru6DhTVA3umU5hZc28V3kO4stgYryrTlLpuvgI9GiijltAjNbcqA==",
      "license": "MIT",
      "dependencies": {
        "ms": "^2.1.3"
      },
      "engines": {
        "node": ">=6.0"
      },
      "peerDependenciesMeta": {
        "supports-color": {
          "optional": true
        }
      }
    },
    "node_modules/http-proxy-agent/node_modules/ms": {
      "version": "2.1.3",
      "resolved": "https://registry.npmjs.org/ms/-/ms-2.1.3.tgz",
      "integrity": "sha512-6FlzubTLZG3J2a/NVCAleEhjzq5oxgHyaCU9yYXvcLsvoVaHJq/s5xXI6/XXP6tz7R9xAOtHnSO/tXtF3WRTlA==",
      "license": "MIT"
    },
    "node_modules/https-proxy-agent": {
      "version": "7.0.6",
      "resolved": "https://registry.npmjs.org/https-proxy-agent/-/https-proxy-agent-7.0.6.tgz",
      "integrity": "sha512-vK9P5/iUfdl95AI+JVyUuIcVtd4ofvtrOr3HNtM2yxC9bnMbEdp3x01OhQNnjb8IJYi38VlTE3mBXwcfvywuSw==",
      "license": "MIT",
      "dependencies": {
        "agent-base": "^7.1.2",
        "debug": "4"
      },
      "engines": {
        "node": ">= 14"
      }
    },
    "node_modules/https-proxy-agent/node_modules/debug": {
      "version": "4.4.3",
      "resolved": "https://registry.npmjs.org/debug/-/debug-4.4.3.tgz",
      "integrity": "sha512-RGwwWnwQvkVfavKVt22FGLw+xYSdzARwm0ru6DhTVA3umU5hZc28V3kO4stgYryrTlLpuvgI9GiijltAjNbcqA==",
      "license": "MIT",
      "dependencies": {
        "ms": "^2.1.3"
      },
      "engines": {
        "node": ">=6.0"
      },
      "peerDependenciesMeta": {
        "supports-color": {
          "optional": true
        }
      }
    },
    "node_modules/https-proxy-agent/node_modules/ms": {
      "version": "2.1.3",
      "resolved": "https://registry.npmjs.org/ms/-/ms-2.1.3.tgz",
      "integrity": "sha512-6FlzubTLZG3J2a/NVCAleEhjzq5oxgHyaCU9yYXvcLsvoVaHJq/s5xXI6/XXP6tz7R9xAOtHnSO/tXtF3WRTlA==",
      "license": "MIT"
    },
    "node_modules/iconv-lite": {
      "version": "0.7.3",
      "resolved": "https://registry.npmjs.org/iconv-lite/-/iconv-lite-0.7.3.tgz",
      "integrity": "sha512-IKXpvIzjnC9XTAUbVBcMfGS0EPaIXtW6v+zr+RRp+hqULEpo0owZax6wyRwPOJbWbzjYspQwusTsfVr0ifh4uQ==",
      "license": "MIT",
      "dependencies": {
        "safer-buffer": ">= 2.1.2 < 3.0.0"
      },
      "engines": {
        "node": ">=0.10.0"
      },
      "funding": {
        "type": "opencollective",
        "url": "https://opencollective.com/express"
      }
    },
    "node_modules/inherits": {
      "version": "2.0.4",
      "resolved": "https://registry.npmjs.org/inherits/-/inherits-2.0.4.tgz",
      "integrity": "sha512-k/vGaX4/Yla3WzyMCvTQOXYeIHvqOKtnqBduzTHpzpQZzAskKMhZ2K+EnBiSM9zGSoIFeMpXKxa4dYeZIQqewQ==",
      "license": "ISC"
    },
    "node_modules/ip-address": {
      "version": "10.7.2",
      "resolved": "https://registry.npmjs.org/ip-address/-/ip-address-10.7.2.tgz",
      "integrity": "sha512-7H/2gFSIitxc0hG3nOI1glS8QLo/EHBFFLk8vEUjXY/xu0AdL8jZ9U1IzO2PUm0d2D/ofQcAifb0g6OBkt8U7w==",
      "license": "MIT",
      "engines": {
        "node": ">= 12"
      }
    },
    "node_modules/ipaddr.js": {
      "version": "2.5.0",
      "resolved": "https://registry.npmjs.org/ipaddr.js/-/ipaddr.js-2.5.0.tgz",
      "integrity": "sha512-aq+t5NAc+cS6rZQQVWC2x98CPqGtKKTMDd4Gaodv0wShnItdKg/51djkGJ1hqH+Oy0ivDftCbSLCQob8zso01w==",
      "license": "MIT",
      "engines": {
        "node": ">= 10"
      }
    },
    "node_modules/is-docker": {
      "version": "3.0.0",
      "resolved": "https://registry.npmjs.org/is-docker/-/is-docker-3.0.0.tgz",
      "integrity": "sha512-eljcgEDlEns/7AXFosB5K/2nCM4P7FQPkGc/DWLy5rmFEWvZayGrik1d9/QIY5nJ4f9YsVvBkA6kJpHn9rISdQ==",
      "license": "MIT",
      "bin": {
        "is-docker": "cli.js"
      },
      "engines": {
        "node": "^12.20.0 || ^14.13.1 || >=16.0.0"
      },
      "funding": {
        "url": "https://github.com/sponsors/sindresorhus"
      }
    },
    "node_modules/is-in-ssh": {
      "version": "1.0.0",
      "resolved": "https://registry.npmjs.org/is-in-ssh/-/is-in-ssh-1.0.0.tgz",
      "integrity": "sha512-jYa6Q9rH90kR1vKB6NM7qqd1mge3Fx4Dhw5TVlK1MUBqhEOuCagrEHMevNuCcbECmXZ0ThXkRm+Ymr51HwEPAw==",
      "license": "MIT",
      "engines": {
        "node": ">=20"
      },
      "funding": {
        "url": "https://github.com/sponsors/sindresorhus"
      }
    },
    "node_modules/is-inside-container": {
      "version": "1.0.0",
      "resolved": "https://registry.npmjs.org/is-inside-container/-/is-inside-container-1.0.0.tgz",
      "integrity": "sha512-KIYLCCJghfHZxqjYBE7rEy0OBuTd5xCHS7tHVgvCLkx7StIoaxwNW3hCALgEUjFfeRk+MG/Qxmp/vtETEF3tRA==",
      "license": "MIT",
      "dependencies": {
        "is-docker": "^3.0.0"
      },
      "bin": {
        "is-inside-container": "cli.js"
      },
      "engines": {
        "node": ">=14.16"
      },
      "funding": {
        "url": "https://github.com/sponsors/sindresorhus"
      }
    },
    "node_modules/is-promise": {
      "version": "4.0.0",
      "resolved": "https://registry.npmjs.org/is-promise/-/is-promise-4.0.0.tgz",
      "integrity": "sha512-hvpoI6korhJMnej285dSg6nu1+e6uxs7zG3BYAm5byqDsgJNWwxzM6z6iZiAgQR4TJ30JmBTOwqZUw3WlyH3AQ==",
      "license": "MIT"
    },
    "node_modules/is-wsl": {
      "version": "3.1.1",
      "resolved": "https://registry.npmjs.org/is-wsl/-/is-wsl-3.1.1.tgz",
      "integrity": "sha512-e6rvdUCiQCAuumZslxRJWR/Doq4VpPR82kqclvcS0efgt430SlGIk05vdCN58+VrzgtIcfNODjozVielycD4Sw==",
      "license": "MIT",
      "dependencies": {
        "is-inside-container": "^1.0.0"
      },
      "engines": {
        "node": ">=16"
      },
      "funding": {
        "url": "https://github.com/sponsors/sindresorhus"
      }
    },
    "node_modules/isexe": {
      "version": "2.0.0",
      "resolved": "https://registry.npmjs.org/isexe/-/isexe-2.0.0.tgz",
      "integrity": "sha512-RHxMLp9lnKHGHRng9QFhRCMbYAcVpn69smSGcq3f36xjgVVWThj4qqLbTLlq7Ssj8B+fIQ1EuCEGI2lKsyQeIw==",
      "license": "ISC"
    },
    "node_modules/jose": {
      "version": "6.2.12",
      "resolved": "https://registry.npmjs.org/jose/-/jose-6.2.12.tgz",
      "integrity": "sha512-9NiFmJEex0sy2Dk58j2UGBSHgUs2ypF9eZSu4L6vjOX3Dp96Sw1F3uL+H+D1sx02jZZdzUT0HgvCy59CuvXcWw==",
      "license": "MIT",
      "funding": {
        "url": "https://github.com/sponsors/panva"
      }
    },
    "node_modules/js-tokens": {
      "version": "4.0.0",
      "resolved": "https://registry.npmjs.org/js-tokens/-/js-tokens-4.0.0.tgz",
      "integrity": "sha512-RdJUflcE3cUzKiMqQgsCu06FPu9UdIJO0beYbPhHN4k6apgJtifcoCtT9bcxOpYBtpD2kCM6Sbzg4CausW/PKQ==",
      "license": "MIT"
    },
    "node_modules/js-yaml": {
      "version": "4.3.2",
      "resolved": "https://registry.npmjs.org/js-yaml/-/js-yaml-4.3.2.tgz",
      "integrity": "sha512-SFNOvSJ+Dgf/9An904Yx+CgSlIPCkIpao4qo51lpee25TIRejdH3rhR4EZMGoNx3/TP3O+wzWuiTFl4sqbltzA==",
      "funding": [
        {
          "type": "github",
          "url": "https://github.com/sponsors/puzrin"
        },
        {
          "type": "github",
          "url": "https://github.com/sponsors/nodeca"
        }
      ],
      "license": "MIT",
      "dependencies": {
        "argparse": "^2.0.1"
      },
      "bin": {
        "js-yaml": "bin/js-yaml.js"
      }
    },
    "node_modules/json-bigint": {
      "version": "1.0.0",
      "resolved": "https://registry.npmjs.org/json-bigint/-/json-bigint-1.0.0.tgz",
      "integrity": "sha512-SiPv/8VpZuWbvLSMtTDU8hEfrZWg/mH/nV/b4o0CYbSxu1UIQPLdwKOCIyLQX+VIPO5vrLX3i8qtqFyhdPSUSQ==",
      "license": "MIT",
      "dependencies": {
        "bignumber.js": "^9.0.0"
      }
    },
    "node_modules/json-schema-to-ts": {
      "version": "3.1.1",
      "resolved": "https://registry.npmjs.org/json-schema-to-ts/-/json-schema-to-ts-3.1.1.tgz",
      "integrity": "sha512-+DWg8jCJG2TEnpy7kOm/7/AxaYoaRbjVB4LFZLySZlWn8exGs3A4OLJR966cVvU26N7X9TWxl+Jsw7dzAqKT6g==",
      "license": "MIT",
      "dependencies": {
        "@babel/runtime": "^7.18.3",
        "ts-algebra": "^2.0.0"
      },
      "engines": {
        "node": ">=16"
      }
    },
    "node_modules/json-schema-traverse": {
      "version": "1.0.0",
      "resolved": "https://registry.npmjs.org/json-schema-traverse/-/json-schema-traverse-1.0.0.tgz",
      "integrity": "sha512-NM8/P9n3XjXhIZn1lLhkFaACTOURQXjWhV4BA/RnOv8xvgqtqpAX9IO4mRQxSx1Rlo4tqzeqb0sOlruaOy3dug==",
      "license": "MIT"
    },
    "node_modules/json-schema-typed": {
      "version": "8.0.2",
      "resolved": "https://registry.npmjs.org/json-schema-typed/-/json-schema-typed-8.0.2.tgz",
      "integrity": "sha512-fQhoXdcvc3V28x7C7BMs4P5+kNlgUURe2jmUT1T//oBRMDrqy1QPelJimwZGo7Hg9VPV3EQV5Bnq4hbFy2vetA==",
      "license": "BSD-2-Clause"
    },
    "node_modules/jwa": {
      "version": "2.0.1",
      "resolved": "https://registry.npmjs.org/jwa/-/jwa-2.0.1.tgz",
      "integrity": "sha512-hRF04fqJIP8Abbkq5NKGN0Bbr3JxlQ+qhZufXVr0DvujKy93ZCbXZMHDL4EOtodSbCWxOqR8MS1tXA5hwqCXDg==",
      "license": "MIT",
      "dependencies": {
        "buffer-equal-constant-time": "^1.0.1",
        "ecdsa-sig-formatter": "1.0.11",
        "safe-buffer": "^5.0.1"
      }
    },
    "node_modules/jws": {
      "version": "4.0.1",
      "resolved": "https://registry.npmjs.org/jws/-/jws-4.0.1.tgz",
      "integrity": "sha512-EKI/M/yqPncGUUh44xz0PxSidXFr/+r0pA70+gIYhjv+et7yxM+s29Y+VGDkovRofQem0fs7Uvf4+YmAdyRduA==",
      "license": "MIT",
      "dependencies": {
        "jwa": "^2.0.1",
        "safe-buffer": "^5.0.1"
      }
    },
    "node_modules/koffi": {
      "version": "3.3.1",
      "resolved": "https://registry.npmjs.org/koffi/-/koffi-3.3.1.tgz",
      "integrity": "sha512-FZYfhBfYQr/cmHhZpaQ9rhbKpkjHhxaytn2A0mbPhiD5yV8QzF3SiQwR1w/34rCJ1fw+6qBQSj/ZoXRdC+Xc2Q==",
      "hasInstallScript": true,
      "license": "MIT",
      "funding": {
        "url": "https://liberapay.com/Koromix"
      },
      "optionalDependencies": {
        "@koromix/koffi-android-arm64": "3.3.1",
        "@koromix/koffi-android-x64": "3.3.1",
        "@koromix/koffi-darwin-arm64": "3.3.1",
        "@koromix/koffi-darwin-x64": "3.3.1",
        "@koromix/koffi-freebsd-arm64": "3.3.1",
        "@koromix/koffi-freebsd-ia32": "3.3.1",
        "@koromix/koffi-freebsd-x64": "3.3.1",
        "@koromix/koffi-linux-arm": "3.3.1",
        "@koromix/koffi-linux-arm64": "3.3.1",
        "@koromix/koffi-linux-ia32": "3.3.1",
        "@koromix/koffi-linux-loong64": "3.3.1",
        "@koromix/koffi-linux-ppc64": "3.3.1",
        "@koromix/koffi-linux-riscv64": "3.3.1",
        "@koromix/koffi-linux-x64": "3.3.1",
        "@koromix/koffi-openbsd-arm64": "3.3.1",
        "@koromix/koffi-openbsd-ia32": "3.3.1",
        "@koromix/koffi-openbsd-x64": "3.3.1",
        "@koromix/koffi-win32-arm64": "3.3.1",
        "@koromix/koffi-win32-ia32": "3.3.1",
        "@koromix/koffi-win32-x64": "3.3.1"
      }
    },
    "node_modules/long": {
      "version": "5.3.2",
      "resolved": "https://registry.npmjs.org/long/-/long-5.3.2.tgz",
      "integrity": "sha512-mNAgZ1GmyNhD7AuqnTG3/VQ26o760+ZYBPKjPvugO8+nLbYfX6TVpJPseBvopbdY+qpZ/lKUnmEc1LeZYS3QAA==",
      "license": "Apache-2.0"
    },
    "node_modules/math-intrinsics": {
      "version": "1.1.0",
      "resolved": "https://registry.npmjs.org/math-intrinsics/-/math-intrinsics-1.1.0.tgz",
      "integrity": "sha512-/IXtbwEk5HTPyEwyKX6hGkYXxM9nbj64B+ilVJnC/R6B0pH5G4V3b0pVbL7DBj4tkhBAppbQUlf6F6Xl9LHu1g==",
      "license": "MIT",
      "engines": {
        "node": ">= 0.4"
      }
    },
    "node_modules/media-typer": {
      "version": "1.1.1",
      "resolved": "https://registry.npmjs.org/media-typer/-/media-typer-1.1.1.tgz",
      "integrity": "sha512-yz3xRaG20c6/BOzvYoDaGtPmGscs7YivItZEEqe6GbwNfHuxu9YNmvnEkMzKldAGY4/80pRcQRZSEnhquk9XuQ==",
      "license": "MIT",
      "engines": {
        "node": ">= 0.8"
      },
      "funding": {
        "type": "opencollective",
        "url": "https://opencollective.com/express"
      }
    },
    "node_modules/merge-descriptors": {
      "version": "2.0.0",
      "resolved": "https://registry.npmjs.org/merge-descriptors/-/merge-descriptors-2.0.0.tgz",
      "integrity": "sha512-Snk314V5ayFLhp3fkUREub6WtjBfPdCPY1Ln8/8munuLuiYhsABgBVWsozAG+MWMbVEvcdcpbi9R7ww22l9Q3g==",
      "license": "MIT",
      "engines": {
        "node": ">=18"
      },
      "funding": {
        "url": "https://github.com/sponsors/sindresorhus"
      }
    },
    "node_modules/mime-db": {
      "version": "1.54.0",
      "resolved": "https://registry.npmjs.org/mime-db/-/mime-db-1.54.0.tgz",
      "integrity": "sha512-aU5EJuIN2WDemCcAp2vFBfp/m4EAhWJnUNSSw0ixs7/kXbd6Pg64EmwJkNdFhB8aWt1sH2CTXrLxo/iAGV3oPQ==",
      "license": "MIT",
      "engines": {
        "node": ">= 0.6"
      }
    },
    "node_modules/mime-types": {
      "version": "3.0.2",
      "resolved": "https://registry.npmjs.org/mime-types/-/mime-types-3.0.2.tgz",
      "integrity": "sha512-Lbgzdk0h4juoQ9fCKXW4by0UJqj+nOOrI9MJ1sSj4nI8aI2eo1qmvQEie4VD1glsS250n15LsWsYtCugiStS5A==",
      "license": "MIT",
      "dependencies": {
        "mime-db": "^1.54.0"
      },
      "engines": {
        "node": ">=18"
      },
      "funding": {
        "type": "opencollective",
        "url": "https://opencollective.com/express"
      }
    },
    "node_modules/ms": {
      "version": "2.0.0",
      "resolved": "https://registry.npmjs.org/ms/-/ms-2.0.0.tgz",
      "integrity": "sha512-Tpp60P6IUJDTuOq/5Z8cdskzJujfwqfOTkrwIwj7IRISpnkJnT6SyJ4PCPnGMoFjC9ddhal5KVIYtAt97ix05A==",
      "license": "MIT"
    },
    "node_modules/negotiator": {
      "version": "1.1.0",
      "resolved": "https://registry.npmjs.org/negotiator/-/negotiator-1.1.0.tgz",
      "integrity": "sha512-NMPBRMJgiQHjbd8phG3Vebdx4kZ1H121rbl5IkMqeOsahptB9BKo/d7oJ3zTXqTgagn2bWlNSXkh0QUGM31RYg==",
      "license": "MIT",
      "dependencies": {
        "content-type": "^2.1.0"
      },
      "engines": {
        "node": ">=18"
      },
      "funding": {
        "type": "opencollective",
        "url": "https://opencollective.com/express"
      }
    },
    "node_modules/negotiator/node_modules/content-type": {
      "version": "2.1.0",
      "resolved": "https://registry.npmjs.org/content-type/-/content-type-2.1.0.tgz",
      "integrity": "sha512-mj7UPXE0jaqaOsukNZRUEfEi2AcL7C/vwmwcHV0O97eO1E1pxBZuyjlZrx5seTaNBg1U6+o35wpa35Qfcc+7ag==",
      "license": "MIT",
      "engines": {
        "node": ">=18"
      },
      "funding": {
        "type": "opencollective",
        "url": "https://opencollective.com/express"
      }
    },
    "node_modules/node-addon-api": {
      "version": "7.1.1",
      "resolved": "https://registry.npmjs.org/node-addon-api/-/node-addon-api-7.1.1.tgz",
      "integrity": "sha512-5m3bsyrjFWE1xf7nz7YXdN4udnVtXK6/Yfgn5qnahL6bCkf2yKt4k3nuTKAtT4r3IG8JNR2ncsIMdZuAzJjHQQ==",
      "license": "MIT"
    },
    "node_modules/node-addon-native-custom-loader": {
      "version": "0.1.6",
      "resolved": "https://registry.npmjs.org/node-addon-native-custom-loader/-/node-addon-native-custom-loader-0.1.6.tgz",
      "integrity": "sha512-QaW7d8lTcGCXrpNVyFDtID3vrpa7vFdpcY9pql2/9FGFSe7tcSnhQOKZX+1aOWq+pB1d2ZwNnKAdQI5nD9wAXA==",
      "license": "MIT",
      "engines": {
        "node": ">=20"
      }
    },
    "node_modules/node-addon-require-builtin": {
      "version": "0.1.6",
      "resolved": "https://registry.npmjs.org/node-addon-require-builtin/-/node-addon-require-builtin-0.1.6.tgz",
      "integrity": "sha512-P9ZGMDkloktirLJSggfpxsJ9jog5FItE1Omxpj50UBn3LhD6TS6/yx0jEBXsCGK3P9EtW1EpQVA1QL5gb11GaQ==",
      "license": "MIT",
      "dependencies": {
        "node-addon-native-custom-loader": "0.1.6"
      },
      "engines": {
        "node": ">=20"
      },
      "optionalDependencies": {
        "node-addon-require-builtin-darwin-arm64": "0.1.6",
        "node-addon-require-builtin-darwin-x64": "0.1.6",
        "node-addon-require-builtin-linux-arm64-gnu": "0.1.6",
        "node-addon-require-builtin-linux-x64-gnu": "0.1.6",
        "node-addon-require-builtin-win32-arm64-msvc": "0.1.6",
        "node-addon-require-builtin-win32-ia32-msvc": "0.1.6",
        "node-addon-require-builtin-win32-x64-msvc": "0.1.6"
      }
    },
    "node_modules/node-addon-require-builtin-linux-x64-gnu": {
      "version": "0.1.6",
      "resolved": "https://registry.npmjs.org/node-addon-require-builtin-linux-x64-gnu/-/node-addon-require-builtin-linux-x64-gnu-0.1.6.tgz",
      "integrity": "sha512-8wCxrFlB2Ld5DhxIm2P+nNFnQ0wjzDE5sXF68EUhLH0X+z313B07TUQFMBKCOkyhZIIBh590dBmhHCuA43No2A==",
      "cpu": [
        "x64"
      ],
      "license": "MIT",
      "optional": true,
      "os": [
        "linux"
      ],
      "dependencies": {
        "node-addon-native-custom-loader": "0.1.6"
      },
      "engines": {
        "node": ">=20"
      }
    },
    "node_modules/node-domexception": {
      "version": "1.0.0",
      "resolved": "https://registry.npmjs.org/node-domexception/-/node-domexception-1.0.0.tgz",
      "integrity": "sha512-/jKZoMpw0F8GRwl4/eLROPA3cfcXtLApP0QzLmUT/HuPCZWyB7IY9ZrMeKw2O/nFIqPQB3PVM9aYm0F312AXDQ==",
      "deprecated": "Use your platform's native DOMException instead",
      "funding": [
        {
          "type": "github",
          "url": "https://github.com/sponsors/jimmywarting"
        },
        {
          "type": "github",
          "url": "https://paypal.me/jimmywarting"
        }
      ],
      "license": "MIT",
      "engines": {
        "node": ">=10.5.0"
      }
    },
    "node_modules/node-fetch": {
      "version": "3.3.2",
      "resolved": "https://registry.npmjs.org/node-fetch/-/node-fetch-3.3.2.tgz",
      "integrity": "sha512-dRB78srN/l6gqWulah9SrxeYnxeddIG30+GOqK/9OlLVyLg3HPnr6SqOWTWOXKRwC2eGYCkZ59NNuSgvSrpgOA==",
      "license": "MIT",
      "dependencies": {
        "data-uri-to-buffer": "^4.0.0",
        "fetch-blob": "^3.1.4",
        "formdata-polyfill": "^4.0.10"
      },
      "engines": {
        "node": "^12.20.0 || ^14.13.1 || >=16.0.0"
      },
      "funding": {
        "type": "opencollective",
        "url": "https://opencollective.com/node-fetch"
      }
    },
    "node_modules/node-pty": {
      "version": "1.2.0-beta.15",
      "resolved": "https://registry.npmjs.org/node-pty/-/node-pty-1.2.0-beta.15.tgz",
      "integrity": "sha512-vORSzHXi4Ofl7HemVWpuudLqCPdaQb4LfpRCUpE5HPxhp4JYscl8zZwxh11p26v2wvW24WMwnMfLjhRLixrfxA==",
      "hasInstallScript": true,
      "license": "MIT",
      "dependencies": {
        "node-addon-api": "^7.1.0"
      }
    },
    "node_modules/object-assign": {
      "version": "4.1.1",
      "resolved": "https://registry.npmjs.org/object-assign/-/object-assign-4.1.1.tgz",
      "integrity": "sha512-rJgTQnkUnH1sFw8yT6VSU3zD3sWmu6sZhIseY8VX+GRu3P6F7Fu+JNDoXfklElbLJSnc3FUQHVe4cU5hj+BcUg==",
      "license": "MIT",
      "engines": {
        "node": ">=0.10.0"
      }
    },
    "node_modules/object-inspect": {
      "version": "1.13.4",
      "resolved": "https://registry.npmjs.org/object-inspect/-/object-inspect-1.13.4.tgz",
      "integrity": "sha512-W67iLl4J2EXEGTbfeHCffrjDfitvLANg0UlX3wFUUSTx92KXRFegMHUVgSqE+wvhAbi4WqjGg9czysTV2Epbew==",
      "license": "MIT",
      "engines": {
        "node": ">= 0.4"
      },
      "funding": {
        "url": "https://github.com/sponsors/ljharb"
      }
    },
    "node_modules/on-finished": {
      "version": "2.4.1",
      "resolved": "https://registry.npmjs.org/on-finished/-/on-finished-2.4.1.tgz",
      "integrity": "sha512-oVlzkg3ENAhCk2zdv7IJwd/QUD4z2RxRwpkcGY8psCVcCYZNq4wYnVWALHM+brtuJjePWiYF/ClmuDr8Ch5+kg==",
      "license": "MIT",
      "dependencies": {
        "ee-first": "1.1.1"
      },
      "engines": {
        "node": ">= 0.8"
      }
    },
    "node_modules/on-headers": {
      "version": "1.1.0",
      "resolved": "https://registry.npmjs.org/on-headers/-/on-headers-1.1.0.tgz",
      "integrity": "sha512-737ZY3yNnXy37FHkQxPzt4UZ2UWPWiCZWLvFZ4fu5cueciegX0zGPnrlY6bwRg4FdQOe9YU8MkmJwGhoMybl8A==",
      "license": "MIT",
      "engines": {
        "node": ">= 0.8"
      }
    },
    "node_modules/once": {
      "version": "1.4.0",
      "resolved": "https://registry.npmjs.org/once/-/once-1.4.0.tgz",
      "integrity": "sha512-lNaJgI+2Q5URQBkccEKHTQOPaXdUxnZZElQTZY0MFUAuaEqe1E+Nyvgdz/aIyNi6Z9MzO5dv1H8n58/GELp3+w==",
      "license": "ISC",
      "dependencies": {
        "wrappy": "1"
      }
    },
    "node_modules/open": {
      "version": "11.0.4",
      "resolved": "https://registry.npmjs.org/open/-/open-11.0.4.tgz",
      "integrity": "sha512-++Zlftm0kVLPmzC06t6epuWmcRMDbI4z5P3NNX979WA/k23+NtSOynEGzsVfZwguKw2mi5umVgnBlJQMwRz4Pg==",
      "license": "MIT",
      "dependencies": {
        "default-browser": "^5.5.1",
        "define-lazy-prop": "^3.0.0",
        "is-in-ssh": "^1.0.0",
        "is-inside-container": "^1.0.0",
        "powershell-utils": "^0.2.1",
        "wsl-utils": "^1.0.0"
      },
      "engines": {
        "node": ">=20"
      },
      "funding": {
        "url": "https://github.com/sponsors/sindresorhus"
      }
    },
    "node_modules/openai": {
      "version": "6.40.0",
      "resolved": "https://registry.npmjs.org/openai/-/openai-6.40.0.tgz",
      "integrity": "sha512-MWtTjd/gQt4jpbji61NTgFWJLoY/PdRJ6wG9/ZDRMYNMlBKrCrSlkLI+KgHP1vR1qT6LKSAyAqIxno6lcK9JiA==",
      "license": "Apache-2.0",
      "peerDependencies": {
        "ws": "^8.18.0",
        "zod": "^3.25 || ^4.0"
      },
      "peerDependenciesMeta": {
        "ws": {
          "optional": true
        },
        "zod": {
          "optional": true
        }
      }
    },
    "node_modules/p-retry": {
      "version": "4.6.2",
      "resolved": "https://registry.npmjs.org/p-retry/-/p-retry-4.6.2.tgz",
      "integrity": "sha512-312Id396EbJdvRONlngUx0NydfrIQ5lsYu0znKVUzVvArzEIt08V1qhtyESbGVd1FGX7UKtiFp5uwKZdM8wIuQ==",
      "license": "MIT",
      "dependencies": {
        "@types/retry": "0.12.0",
        "retry": "^0.13.1"
      },
      "engines": {
        "node": ">=8"
      }
    },
    "node_modules/parseurl": {
      "version": "1.3.3",
      "resolved": "https://registry.npmjs.org/parseurl/-/parseurl-1.3.3.tgz",
      "integrity": "sha512-CiyeOxFT/JZyN5m0z9PfXw4SCBJ6Sygz1Dpl0wqjlhDEGGBP1GnsUVEL0p63hoG1fcj3fHynXi9NYO4nWOL+qQ==",
      "license": "MIT",
      "engines": {
        "node": ">= 0.8"
      }
    },
    "node_modules/partial-json": {
      "version": "0.1.7",
      "resolved": "https://registry.npmjs.org/partial-json/-/partial-json-0.1.7.tgz",
      "integrity": "sha512-Njv/59hHaokb/hRUjce3Hdv12wd60MtM9Z5Olmn+nehe0QDAsRtRbJPvJ0Z91TusF0SuZRIvnM+S4l6EIP8leA==",
      "license": "MIT"
    },
    "node_modules/path-key": {
      "version": "3.1.1",
      "resolved": "https://registry.npmjs.org/path-key/-/path-key-3.1.1.tgz",
      "integrity": "sha512-ojmeN0qd+y0jszEtoY48r0Peq5dwMEkIlCOu6Q5f41lfkswXuKtYrhgoTpLnyIcHm24Uhqx+5Tqm2InSwLhE6Q==",
      "license": "MIT",
      "engines": {
        "node": ">=8"
      }
    },
    "node_modules/path-to-regexp": {
      "version": "8.4.2",
      "resolved": "https://registry.npmjs.org/path-to-regexp/-/path-to-regexp-8.4.2.tgz",
      "integrity": "sha512-qRcuIdP69NPm4qbACK+aDogI5CBDMi1jKe0ry5rSQJz8JVLsC7jV8XpiJjGRLLol3N+R5ihGYcrPLTno6pAdBA==",
      "license": "MIT",
      "funding": {
        "type": "opencollective",
        "url": "https://opencollective.com/express"
      }
    },
    "node_modules/picocolors": {
      "version": "1.1.1",
      "resolved": "https://registry.npmjs.org/picocolors/-/picocolors-1.1.1.tgz",
      "integrity": "sha512-xceH2snhtb5M9liqDsmEw56le376mTZkEX/jEb/RxNFyegNul7eNslCXP9FDj/Lcu0X8KEyMceP2ntpaHrDEVA==",
      "license": "ISC"
    },
    "node_modules/picomatch": {
      "version": "4.0.7",
      "resolved": "https://registry.npmjs.org/picomatch/-/picomatch-4.0.7.tgz",
      "integrity": "sha512-qcJu88Q2IWqJsDD529JKMdwGm/dvInW4HvQnRwiH9JtihJvzGOscDtHE3x1pBKeUOTysQ8kVmLnJ2kJu7yhcGA==",
      "license": "MIT",
      "engines": {
        "node": ">=12"
      },
      "funding": {
        "url": "https://github.com/sponsors/jonschlinkert"
      }
    },
    "node_modules/pkce-challenge": {
      "version": "5.0.1",
      "resolved": "https://registry.npmjs.org/pkce-challenge/-/pkce-challenge-5.0.1.tgz",
      "integrity": "sha512-wQ0b/W4Fr01qtpHlqSqspcj3EhBvimsdh0KlHhH8HRZnMsEa0ea2fTULOXOS9ccQr3om+GcGRk4e+isrZWV8qQ==",
      "license": "MIT",
      "engines": {
        "node": ">=16.20.0"
      }
    },
    "node_modules/powershell-utils": {
      "version": "0.2.1",
      "resolved": "https://registry.npmjs.org/powershell-utils/-/powershell-utils-0.2.1.tgz",
      "integrity": "sha512-C+y9x90UElAddDZmV4qOx9W53B61PO7cIqWz2dQsWlwswuq4mr8NEwytdGKboYbQlGZ3awrkTeNvcZiZNHnQ8A==",
      "license": "MIT",
      "engines": {
        "node": ">=20"
      },
      "funding": {
        "url": "https://github.com/sponsors/sindresorhus"
      }
    },
    "node_modules/protobufjs": {
      "version": "7.6.6",
      "resolved": "https://registry.npmjs.org/protobufjs/-/protobufjs-7.6.6.tgz",
      "integrity": "sha512-dYDWdjSl5RNb7SgPxGQcRU+GtvP7s2fpkrY0r432PcOIaZ0/rBcxEZnQN67iJhFuQiVw754JDoPruPCNdGsbjg==",
      "hasInstallScript": true,
      "license": "BSD-3-Clause",
      "dependencies": {
        "@protobufjs/aspromise": "^1.1.2",
        "@protobufjs/base64": "^1.1.2",
        "@protobufjs/codegen": "^2.0.5",
        "@protobufjs/eventemitter": "^1.1.1",
        "@protobufjs/fetch": "^1.1.1",
        "@protobufjs/float": "^1.0.2",
        "@protobufjs/path": "^1.1.2",
        "@protobufjs/pool": "^1.1.0",
        "@protobufjs/utf8": "^1.1.1",
        "@types/node": ">=13.7.0",
        "long": "^5.3.2"
      },
      "engines": {
        "node": ">=12.0.0"
      }
    },
    "node_modules/proxy-addr": {
      "version": "2.0.8",
      "resolved": "https://registry.npmjs.org/proxy-addr/-/proxy-addr-2.0.8.tgz",
      "integrity": "sha512-5nnx0yGyVUcY6t9RnWcARWtwT9F1D8O9rt08htPvnd49W1IgZtmLkhu9WfMzQj1cFxjHIO6connUNVW5k7AVyQ==",
      "license": "MIT",
      "dependencies": {
        "forwarded": "0.2.0",
        "ipaddr.js": "1.9.1"
      },
      "engines": {
        "node": ">= 0.10"
      },
      "funding": {
        "type": "opencollective",
        "url": "https://opencollective.com/express"
      }
    },
    "node_modules/proxy-addr/node_modules/ipaddr.js": {
      "version": "1.9.1",
      "resolved": "https://registry.npmjs.org/ipaddr.js/-/ipaddr.js-1.9.1.tgz",
      "integrity": "sha512-0KI/607xoxSToH7GjN1FfSbLoU0+btTicjsQSWQlh/hZykN8KpmMf7uYwPW3R+akZ6R/w18ZlXSHBYXiYUPO3g==",
      "license": "MIT",
      "engines": {
        "node": ">= 0.10"
      }
    },
    "node_modules/qs": {
      "version": "6.16.0",
      "resolved": "https://registry.npmjs.org/qs/-/qs-6.16.0.tgz",
      "integrity": "sha512-h6fhOIaRrID2CbEY2fqs+7t+UXZo+MLAnU5gRIq85uFtdiUPCdsApMlHhXogKVM4HM2DVbIjGNTTYH2OcmP1vA==",
      "license": "BSD-3-Clause",
      "dependencies": {
        "es-define-property": "^1.0.1",
        "side-channel": "^1.1.1"
      },
      "engines": {
        "node": ">=0.6"
      },
      "funding": {
        "url": "https://github.com/sponsors/ljharb"
      }
    },
    "node_modules/range-parser": {
      "version": "1.3.0",
      "resolved": "https://registry.npmjs.org/range-parser/-/range-parser-1.3.0.tgz",
      "integrity": "sha512-hek2mFQpPuI4E1BBKrSto+BU3e3x4xuarsbiwr3+lf7p44juvFMV0XFWQAP3xUyqXA4RrXLIoaSUGbSt056ZMw==",
      "license": "MIT",
      "engines": {
        "node": ">= 0.6"
      },
      "funding": {
        "type": "opencollective",
        "url": "https://opencollective.com/express"
      }
    },
    "node_modules/raw-body": {
      "version": "3.0.2",
      "resolved": "https://registry.npmjs.org/raw-body/-/raw-body-3.0.2.tgz",
      "integrity": "sha512-K5zQjDllxWkf7Z5xJdV0/B0WTNqx6vxG70zJE4N0kBs4LovmEYWJzQGxC9bS9RAKu3bgM40lrd5zoLJ12MQ5BA==",
      "license": "MIT",
      "dependencies": {
        "bytes": "~3.1.2",
        "http-errors": "~2.0.1",
        "iconv-lite": "~0.7.0",
        "unpipe": "~1.0.0"
      },
      "engines": {
        "node": ">= 0.10"
      }
    },
    "node_modules/readdirp": {
      "version": "4.1.2",
      "resolved": "https://registry.npmjs.org/readdirp/-/readdirp-4.1.2.tgz",
      "integrity": "sha512-GDhwkLfywWL2s6vEjyhri+eXmfH6j1L7JE27WhqLeYzoh/A3DBaYGEj2H/HFZCn/kMfim73FXxEJTw06WtxQwg==",
      "license": "MIT",
      "engines": {
        "node": ">= 14.18.0"
      },
      "funding": {
        "type": "individual",
        "url": "https://paulmillr.com/funding/"
      }
    },
    "node_modules/require-from-string": {
      "version": "2.0.2",
      "resolved": "https://registry.npmjs.org/require-from-string/-/require-from-string-2.0.2.tgz",
      "integrity": "sha512-Xf0nWe6RseziFMu+Ap9biiUbmplq6S9/p+7w7YXP/JBHhrUDDUhwa+vANyubuqfZWTveU//DYVGsDG7RKL/vEw==",
      "license": "MIT",
      "engines": {
        "node": ">=0.10.0"
      }
    },
    "node_modules/resolve.exports": {
      "version": "2.0.3",
      "resolved": "https://registry.npmjs.org/resolve.exports/-/resolve.exports-2.0.3.tgz",
      "integrity": "sha512-OcXjMsGdhL4XnbShKpAcSqPMzQoYkYyhbEaeSko47MjRP9NfEQMhZkXL1DoFlt9LWQn4YttrdnV6X2OiyzBi+A==",
      "license": "MIT",
      "engines": {
        "node": ">=10"
      }
    },
    "node_modules/retry": {
      "version": "0.13.1",
      "resolved": "https://registry.npmjs.org/retry/-/retry-0.13.1.tgz",
      "integrity": "sha512-XQBQ3I8W1Cge0Seh+6gjj03LbmRFWuoszgK9ooCpwYIrhhoO80pfq4cUkU5DkknwfOfFteRwlZ56PYOGYyFWdg==",
      "license": "MIT",
      "engines": {
        "node": ">= 4"
      }
    },
    "node_modules/router": {
      "version": "2.2.0",
      "resolved": "https://registry.npmjs.org/router/-/router-2.2.0.tgz",
      "integrity": "sha512-nLTrUKm2UyiL7rlhapu/Zl45FwNgkZGaCpZbIHajDYgwlJCOzLSk+cIPAnsEqV955GjILJnKbdQC1nVPz+gAYQ==",
      "license": "MIT",
      "dependencies": {
        "debug": "^4.4.0",
        "depd": "^2.0.0",
        "is-promise": "^4.0.0",
        "parseurl": "^1.3.3",
        "path-to-regexp": "^8.0.0"
      },
      "engines": {
        "node": ">= 18"
      }
    },
    "node_modules/router/node_modules/debug": {
      "version": "4.4.3",
      "resolved": "https://registry.npmjs.org/debug/-/debug-4.4.3.tgz",
      "integrity": "sha512-RGwwWnwQvkVfavKVt22FGLw+xYSdzARwm0ru6DhTVA3umU5hZc28V3kO4stgYryrTlLpuvgI9GiijltAjNbcqA==",
      "license": "MIT",
      "dependencies": {
        "ms": "^2.1.3"
      },
      "engines": {
        "node": ">=6.0"
      },
      "peerDependenciesMeta": {
        "supports-color": {
          "optional": true
        }
      }
    },
    "node_modules/router/node_modules/ms": {
      "version": "2.1.3",
      "resolved": "https://registry.npmjs.org/ms/-/ms-2.1.3.tgz",
      "integrity": "sha512-6FlzubTLZG3J2a/NVCAleEhjzq5oxgHyaCU9yYXvcLsvoVaHJq/s5xXI6/XXP6tz7R9xAOtHnSO/tXtF3WRTlA==",
      "license": "MIT"
    },
    "node_modules/run-applescript": {
      "version": "7.1.0",
      "resolved": "https://registry.npmjs.org/run-applescript/-/run-applescript-7.1.0.tgz",
      "integrity": "sha512-DPe5pVFaAsinSaV6QjQ6gdiedWDcRCbUuiQfQa2wmWV7+xC9bGulGI8+TdRmoFkAPaBXk8CrAbnlY2ISniJ47Q==",
      "license": "MIT",
      "engines": {
        "node": ">=18"
      },
      "funding": {
        "url": "https://github.com/sponsors/sindresorhus"
      }
    },
    "node_modules/safe-buffer": {
      "version": "5.2.1",
      "resolved": "https://registry.npmjs.org/safe-buffer/-/safe-buffer-5.2.1.tgz",
      "integrity": "sha512-rp3So07KcdmmKbGvgaNxQSJr7bGVSVk5S9Eq1F+ppbRo70+YeaDxkw5Dd8NPN+GD6bjnYm2VuPuCXmpuYvmCXQ==",
      "funding": [
        {
          "type": "github",
          "url": "https://github.com/sponsors/feross"
        },
        {
          "type": "patreon",
          "url": "https://www.patreon.com/feross"
        },
        {
          "type": "consulting",
          "url": "https://feross.org/support"
        }
      ],
      "license": "MIT"
    },
    "node_modules/safer-buffer": {
      "version": "2.1.2",
      "resolved": "https://registry.npmjs.org/safer-buffer/-/safer-buffer-2.1.2.tgz",
      "integrity": "sha512-YZo3K82SD7Riyi0E1EQPojLz7kpepnSQI9IyPbHHg1XXXevb5dJI7tpyN2ADxGcQbHG7vcyRHk0cbwqcQriUtg==",
      "license": "MIT"
    },
    "node_modules/semver": {
      "version": "7.8.5",
      "resolved": "https://registry.npmjs.org/semver/-/semver-7.8.5.tgz",
      "integrity": "sha512-Y7/KDsb8LjooZpwaqGyulO6DQlksgCncchHGk+sZIY4SBvUocMBEFH5Ur1fI4dV+Jvl0w6cjvucaIi40puRioA==",
      "license": "ISC",
      "bin": {
        "semver": "bin/semver.js"
      },
      "engines": {
        "node": ">=10"
      }
    },
    "node_modules/send": {
      "version": "1.2.1",
      "resolved": "https://registry.npmjs.org/send/-/send-1.2.1.tgz",
      "integrity": "sha512-1gnZf7DFcoIcajTjTwjwuDjzuz4PPcY2StKPlsGAQ1+YH20IRVrBaXSWmdjowTJ6u8Rc01PoYOGHXfP1mYcZNQ==",
      "license": "MIT",
      "dependencies": {
        "debug": "^4.4.3",
        "encodeurl": "^2.0.0",
        "escape-html": "^1.0.3",
        "etag": "^1.8.1",
        "fresh": "^2.0.0",
        "http-errors": "^2.0.1",
        "mime-types": "^3.0.2",
        "ms": "^2.1.3",
        "on-finished": "^2.4.1",
        "range-parser": "^1.2.1",
        "statuses": "^2.0.2"
      },
      "engines": {
        "node": ">= 18"
      },
      "funding": {
        "type": "opencollective",
        "url": "https://opencollective.com/express"
      }
    },
    "node_modules/send/node_modules/debug": {
      "version": "4.4.3",
      "resolved": "https://registry.npmjs.org/debug/-/debug-4.4.3.tgz",
      "integrity": "sha512-RGwwWnwQvkVfavKVt22FGLw+xYSdzARwm0ru6DhTVA3umU5hZc28V3kO4stgYryrTlLpuvgI9GiijltAjNbcqA==",
      "license": "MIT",
      "dependencies": {
        "ms": "^2.1.3"
      },
      "engines": {
        "node": ">=6.0"
      },
      "peerDependenciesMeta": {
        "supports-color": {
          "optional": true
        }
      }
    },
    "node_modules/send/node_modules/ms": {
      "version": "2.1.3",
      "resolved": "https://registry.npmjs.org/ms/-/ms-2.1.3.tgz",
      "integrity": "sha512-6FlzubTLZG3J2a/NVCAleEhjzq5oxgHyaCU9yYXvcLsvoVaHJq/s5xXI6/XXP6tz7R9xAOtHnSO/tXtF3WRTlA==",
      "license": "MIT"
    },
    "node_modules/serve-static": {
      "version": "2.2.1",
      "resolved": "https://registry.npmjs.org/serve-static/-/serve-static-2.2.1.tgz",
      "integrity": "sha512-xRXBn0pPqQTVQiC8wyQrKs2MOlX24zQ0POGaj0kultvoOCstBQM5yvOhAVSUwOMjQtTvsPWoNCHfPGwaaQJhTw==",
      "license": "MIT",
      "dependencies": {
        "encodeurl": "^2.0.0",
        "escape-html": "^1.0.3",
        "parseurl": "^1.3.3",
        "send": "^1.2.0"
      },
      "engines": {
        "node": ">= 18"
      },
      "funding": {
        "type": "opencollective",
        "url": "https://opencollective.com/express"
      }
    },
    "node_modules/setprototypeof": {
      "version": "1.2.0",
      "resolved": "https://registry.npmjs.org/setprototypeof/-/setprototypeof-1.2.0.tgz",
      "integrity": "sha512-E5LDX7Wrp85Kil5bhZv46j8jOeboKq5JMmYM3gVGdGH8xFpPWXUMsNrlODCrkoxMEeNi/XZIwuRvY4XNwYMJpw==",
      "license": "ISC"
    },
    "node_modules/sharp": {
      "version": "0.35.4",
      "resolved": "https://registry.npmjs.org/sharp/-/sharp-0.35.4.tgz",
      "integrity": "sha512-n++8XWcj+jCOr2IOl7h8LbKnGBDY4aPbmprMONBNFdn0ImXqpGVv5zliDs0V9HbmbCQLpbuo2ej9rAoOQTvMDA==",
      "license": "Apache-2.0",
      "dependencies": {
        "@img/colour": "^1.1.0",
        "detect-libc": "^2.1.2",
        "semver": "^7.8.5"
      },
      "engines": {
        "node": ">=20.9.0"
      },
      "funding": {
        "url": "https://opencollective.com/libvips"
      },
      "optionalDependencies": {
        "@img/sharp-darwin-arm64": "0.35.4",
        "@img/sharp-darwin-x64": "0.35.4",
        "@img/sharp-freebsd-wasm32": "0.35.4",
        "@img/sharp-libvips-darwin-arm64": "1.3.3",
        "@img/sharp-libvips-darwin-x64": "1.3.3",
        "@img/sharp-libvips-linux-arm": "1.3.3",
        "@img/sharp-libvips-linux-arm64": "1.3.3",
        "@img/sharp-libvips-linux-ppc64": "1.3.3",
        "@img/sharp-libvips-linux-riscv64": "1.3.3",
        "@img/sharp-libvips-linux-s390x": "1.3.3",
        "@img/sharp-libvips-linux-x64": "1.3.3",
        "@img/sharp-libvips-linuxmusl-arm64": "1.3.3",
        "@img/sharp-libvips-linuxmusl-x64": "1.3.3",
        "@img/sharp-linux-arm": "0.35.4",
        "@img/sharp-linux-arm64": "0.35.4",
        "@img/sharp-linux-ppc64": "0.35.4",
        "@img/sharp-linux-riscv64": "0.35.4",
        "@img/sharp-linux-s390x": "0.35.4",
        "@img/sharp-linux-x64": "0.35.4",
        "@img/sharp-linuxmusl-arm64": "0.35.4",
        "@img/sharp-linuxmusl-x64": "0.35.4",
        "@img/sharp-webcontainers-wasm32": "0.35.4",
        "@img/sharp-win32-arm64": "0.35.4",
        "@img/sharp-win32-ia32": "0.35.4",
        "@img/sharp-win32-x64": "0.35.4"
      },
      "peerDependenciesMeta": {
        "@types/node": {
          "optional": true
        }
      }
    },
    "node_modules/shebang-command": {
      "version": "2.0.0",
      "resolved": "https://registry.npmjs.org/shebang-command/-/shebang-command-2.0.0.tgz",
      "integrity": "sha512-kHxr2zZpYtdmrN1qDjrrX/Z1rR1kG8Dx+gkpK1G4eXmvXswmcE1hTWBWYUzlraYw1/yZp6YuDY77YtvbN0dmDA==",
      "license": "MIT",
      "dependencies": {
        "shebang-regex": "^3.0.0"
      },
      "engines": {
        "node": ">=8"
      }
    },
    "node_modules/shebang-regex": {
      "version": "3.0.0",
      "resolved": "https://registry.npmjs.org/shebang-regex/-/shebang-regex-3.0.0.tgz",
      "integrity": "sha512-7++dFhtcx3353uBaq8DDR4NuxBetBzC7ZQOhmTQInHEd6bSrXdiEyzCvG07Z44UYdLShWUyXt5M/yhz8ekcb1A==",
      "license": "MIT",
      "engines": {
        "node": ">=8"
      }
    },
    "node_modules/side-channel": {
      "version": "1.1.1",
      "resolved": "https://registry.npmjs.org/side-channel/-/side-channel-1.1.1.tgz",
      "integrity": "sha512-6x6dK6zJdpTzF4sQeNYxwtvBzf6Eg4GtlesS94HOvTudUeyK2WXAaIfmDgsyslYrRBeFIlsi54AYsFGUuhmvrQ==",
      "license": "MIT",
      "dependencies": {
        "es-errors": "^1.3.0",
        "object-inspect": "^1.13.4",
        "side-channel-list": "^1.0.1",
        "side-channel-map": "^1.0.1",
        "side-channel-weakmap": "^1.0.2"
      },
      "engines": {
        "node": ">= 0.4"
      },
      "funding": {
        "url": "https://github.com/sponsors/ljharb"
      }
    },
    "node_modules/side-channel-list": {
      "version": "1.0.1",
      "resolved": "https://registry.npmjs.org/side-channel-list/-/side-channel-list-1.0.1.tgz",
      "integrity": "sha512-mjn/0bi/oUURjc5Xl7IaWi/OJJJumuoJFQJfDDyO46+hBWsfaVM65TBHq2eoZBhzl9EchxOijpkbRC8SVBQU0w==",
      "license": "MIT",
      "dependencies": {
        "es-errors": "^1.3.0",
        "object-inspect": "^1.13.4"
      },
      "engines": {
        "node": ">= 0.4"
      },
      "funding": {
        "url": "https://github.com/sponsors/ljharb"
      }
    },
    "node_modules/side-channel-map": {
      "version": "1.0.1",
      "resolved": "https://registry.npmjs.org/side-channel-map/-/side-channel-map-1.0.1.tgz",
      "integrity": "sha512-VCjCNfgMsby3tTdo02nbjtM/ewra6jPHmpThenkTYh8pG9ucZ/1P8So4u4FGBek/BjpOVsDCMoLA/iuBKIFXRA==",
      "license": "MIT",
      "dependencies": {
        "call-bound": "^1.0.2",
        "es-errors": "^1.3.0",
        "get-intrinsic": "^1.2.5",
        "object-inspect": "^1.13.3"
      },
      "engines": {
        "node": ">= 0.4"
      },
      "funding": {
        "url": "https://github.com/sponsors/ljharb"
      }
    },
    "node_modules/side-channel-weakmap": {
      "version": "1.0.2",
      "resolved": "https://registry.npmjs.org/side-channel-weakmap/-/side-channel-weakmap-1.0.2.tgz",
      "integrity": "sha512-WPS/HvHQTYnHisLo9McqBHOJk2FkHO/tlpvldyrnem4aeQp4hai3gythswg6p01oSoTl58rcpiFAjF2br2Ak2A==",
      "license": "MIT",
      "dependencies": {
        "call-bound": "^1.0.2",
        "es-errors": "^1.3.0",
        "get-intrinsic": "^1.2.5",
        "object-inspect": "^1.13.3",
        "side-channel-map": "^1.0.1"
      },
      "engines": {
        "node": ">= 0.4"
      },
      "funding": {
        "url": "https://github.com/sponsors/ljharb"
      }
    },
    "node_modules/standardwebhooks": {
      "version": "1.1.1",
      "resolved": "https://registry.npmjs.org/standardwebhooks/-/standardwebhooks-1.1.1.tgz",
      "integrity": "sha512-bCbX9ZEyFkWPsRz7Bl3NuQUJohmwGSev/yhr7vhaGPlc4AfIrspIRa6cPTBuI1ItmrTDJ4d/S2hCsfe4+vQGnQ==",
      "license": "MIT",
      "dependencies": {
        "@stablelib/base64": "^1.0.0",
        "fast-sha256": "^1.3.0"
      }
    },
    "node_modules/statuses": {
      "version": "2.0.2",
      "resolved": "https://registry.npmjs.org/statuses/-/statuses-2.0.2.tgz",
      "integrity": "sha512-DvEy55V3DB7uknRo+4iOGT5fP1slR8wQohVdknigZPMpMstaKJQWhwiYBACJE3Ul2pTnATihhBYnRhZQHGBiRw==",
      "license": "MIT",
      "engines": {
        "node": ">= 0.8"
      }
    },
    "node_modules/toidentifier": {
      "version": "1.0.1",
      "resolved": "https://registry.npmjs.org/toidentifier/-/toidentifier-1.0.1.tgz",
      "integrity": "sha512-o5sSPKEkg/DIQNmH43V0/uerLrpzVedkUh8tGNvaeXpfpuwjKenlSox/2O/BTlZUtEe+JG7s5YhEz608PlAHRA==",
      "license": "MIT",
      "engines": {
        "node": ">=0.6"
      }
    },
    "node_modules/ts-algebra": {
      "version": "2.0.0",
      "resolved": "https://registry.npmjs.org/ts-algebra/-/ts-algebra-2.0.0.tgz",
      "integrity": "sha512-FPAhNPFMrkwz76P7cdjdmiShwMynZYN6SgOujD1urY4oNm80Ou9oMdmbR45LotcKOXoy7wSmHkRFE6Mxbrhefw==",
      "license": "MIT"
    },
    "node_modules/tslib": {
      "version": "2.8.1",
      "resolved": "https://registry.npmjs.org/tslib/-/tslib-2.8.1.tgz",
      "integrity": "sha512-oJFu94HQb+KVduSUQL7wnpmqnfmLsOA/nAh6b6EH0wCEoK0/mPeXU6c3wKDV83MkOuHPRHtSXKKU99IBazS/2w==",
      "license": "0BSD"
    },
    "node_modules/turndown": {
      "version": "7.2.4",
      "resolved": "https://registry.npmjs.org/turndown/-/turndown-7.2.4.tgz",
      "integrity": "sha512-I8yFsfRzmzK0WV1pNNOA4A7y4RDfFxPRxb3t+e3ui14qSGOxGtiSP6GjeX+Y6CHb7HYaFj7ECUD7VE5kQMZWGQ==",
      "license": "MIT",
      "dependencies": {
        "@mixmark-io/domino": "^2.2.0"
      },
      "engines": {
        "node": ">=18",
        "npm": ">=9"
      }
    },
    "node_modules/type-is": {
      "version": "2.1.0",
      "resolved": "https://registry.npmjs.org/type-is/-/type-is-2.1.0.tgz",
      "integrity": "sha512-faYHw0anBbc/kWF3zFTEnxSFOAGUX9GFbOBthvDdLsIlEoWOFOtS0zgCiQYwIskL9iGXZL3kAXD8OoZ4GmMATA==",
      "license": "MIT",
      "dependencies": {
        "content-type": "^2.0.0",
        "media-typer": "^1.1.0",
        "mime-types": "^3.0.0"
      },
      "engines": {
        "node": ">= 18"
      },
      "funding": {
        "type": "opencollective",
        "url": "https://opencollective.com/express"
      }
    },
    "node_modules/type-is/node_modules/content-type": {
      "version": "2.1.0",
      "resolved": "https://registry.npmjs.org/content-type/-/content-type-2.1.0.tgz",
      "integrity": "sha512-mj7UPXE0jaqaOsukNZRUEfEi2AcL7C/vwmwcHV0O97eO1E1pxBZuyjlZrx5seTaNBg1U6+o35wpa35Qfcc+7ag==",
      "license": "MIT",
      "engines": {
        "node": ">=18"
      },
      "funding": {
        "type": "opencollective",
        "url": "https://opencollective.com/express"
      }
    },
    "node_modules/typebox": {
      "version": "1.3.7",
      "resolved": "https://registry.npmjs.org/typebox/-/typebox-1.3.7.tgz",
      "integrity": "sha512-meKuifc33Pccx0O6PdIzYMq3Og8zvP4TIi/a+Bw3AEMZMxOD0+RHGQvpglEe6Zdy3wZ8nqn/j95h8LUZLk/6Hg==",
      "license": "MIT"
    },
    "node_modules/undici": {
      "version": "8.10.2",
      "resolved": "https://registry.npmjs.org/undici/-/undici-8.10.2.tgz",
      "integrity": "sha512-/y4/bH9YNU5hi9NIrpOuvGXFcxrj3CMrV+/AYpowAYTpHn8gX/XPFjNy766FPoYY0miQhdW977JFWKGNhBdwyQ==",
      "license": "MIT",
      "engines": {
        "node": ">=22.19.0"
      }
    },
    "node_modules/undici-types": {
      "version": "8.9.0",
      "resolved": "https://registry.npmjs.org/undici-types/-/undici-types-8.9.0.tgz",
      "integrity": "sha512-KTDyRTYX8sWmKXAikPHHSyc63CRPETMctyjKFupcC6OBLXT3xsN0e9aF7m+mIXutFWpUXuedtowG7iLOzp0kQg==",
      "license": "MIT"
    },
    "node_modules/unpipe": {
      "version": "1.0.0",
      "resolved": "https://registry.npmjs.org/unpipe/-/unpipe-1.0.0.tgz",
      "integrity": "sha512-pjy2bYhSsufwWlKwPc+l3cN7+wuJlK6uz0YdJEOlQDbl6jo/YlPi4mb8agUkVC8BF7V8NuzeyPNqRksA3hztKQ==",
      "license": "MIT",
      "engines": {
        "node": ">= 0.8"
      }
    },
    "node_modules/vary": {
      "version": "1.1.2",
      "resolved": "https://registry.npmjs.org/vary/-/vary-1.1.2.tgz",
      "integrity": "sha512-BNGbWLfd0eUPabhkXUVm0j8uuvREyTh5ovRa/dyow/BqAbZJyC+5fU+IzQOzmAKzYqYRAISoRhdQr3eIZ/PXqg==",
      "license": "MIT",
      "engines": {
        "node": ">= 0.8"
      }
    },
    "node_modules/web-streams-polyfill": {
      "version": "3.3.3",
      "resolved": "https://registry.npmjs.org/web-streams-polyfill/-/web-streams-polyfill-3.3.3.tgz",
      "integrity": "sha512-d2JWLCivmZYTSIoge9MsgFCZrt571BikcWGYkjC1khllbTeDlGqZ2D8vD8E/lJa8WGWbb7Plm8/XJYV7IJHZZw==",
      "license": "MIT",
      "engines": {
        "node": ">= 8"
      }
    },
    "node_modules/which": {
      "version": "2.0.2",
      "resolved": "https://registry.npmjs.org/which/-/which-2.0.2.tgz",
      "integrity": "sha512-BLI3Tl1TW3Pvl70l3yq3Y64i+awpwXqsGBYWkkqMtnbXgrMD+yj7rhW0kuEDxzJaYXGjEW5ogapKNMEKNMjibA==",
      "license": "ISC",
      "dependencies": {
        "isexe": "^2.0.0"
      },
      "bin": {
        "node-which": "bin/node-which"
      },
      "engines": {
        "node": ">= 8"
      }
    },
    "node_modules/wrappy": {
      "version": "1.0.2",
      "resolved": "https://registry.npmjs.org/wrappy/-/wrappy-1.0.2.tgz",
      "integrity": "sha512-l4Sp/DRseor9wL6EvV2+TuQn63dMkPjZ/sp9XkghTEbV9KlPS1xUsZ3u7/IQO4wxtcFB4bgpQPRcR3QCvezPcQ==",
      "license": "ISC"
    },
    "node_modules/ws": {
      "version": "8.21.3",
      "resolved": "https://registry.npmjs.org/ws/-/ws-8.21.3.tgz",
      "integrity": "sha512-201TZ/kPWxoPr/OKWjquZR1SWKXcvxdH+e1xrx89b3YbmzLMFCLfnaG1HFIgWzJOEWZ7MvpK++odZufgYR50Rw==",
      "license": "MIT",
      "engines": {
        "node": ">=10.0.0"
      },
      "peerDependencies": {
        "bufferutil": "^4.0.1",
        "utf-8-validate": ">=5.0.2"
      },
      "peerDependenciesMeta": {
        "bufferutil": {
          "optional": true
        },
        "utf-8-validate": {
          "optional": true
        }
      }
    },
    "node_modules/wsl-utils": {
      "version": "1.0.0",
      "resolved": "https://registry.npmjs.org/wsl-utils/-/wsl-utils-1.0.0.tgz",
      "integrity": "sha512-Hl0ZOAs672vg+06kfujwRhoS6/jehvULrlFkuF2dRu6pHgA8U06h3xqNIqNNU1LTXPcedxByAR4GS6pwQK0mgA==",
      "license": "MIT",
      "dependencies": {
        "is-wsl": "^3.1.0",
        "powershell-utils": "^0.1.0"
      },
      "engines": {
        "node": ">=20"
      },
      "funding": {
        "url": "https://github.com/sponsors/sindresorhus"
      }
    },
    "node_modules/wsl-utils/node_modules/powershell-utils": {
      "version": "0.1.0",
      "resolved": "https://registry.npmjs.org/powershell-utils/-/powershell-utils-0.1.0.tgz",
      "integrity": "sha512-dM0jVuXJPsDN6DvRpea484tCUaMiXWjuCn++HGTqUWzGDjv5tZkEZldAJ/UMlqRYGFrD/etByo4/xOuC/snX2A==",
      "license": "MIT",
      "engines": {
        "node": ">=20"
      },
      "funding": {
        "url": "https://github.com/sponsors/sindresorhus"
      }
    },
    "node_modules/yaml": {
      "version": "2.9.1",
      "resolved": "https://registry.npmjs.org/yaml/-/yaml-2.9.1.tgz",
      "integrity": "sha512-3NxN8+78OdzbT7C/WjGsyfPAtJaN3FNDsWxv7Y7mcDsT/oOmgW8BpyQQFFBnvZE3j9Y2Sdz1ULFLezL7Eb2yFw==",
      "license": "ISC",
      "bin": {
        "yaml": "bin.mjs"
      },
      "engines": {
        "node": ">= 14.6"
      },
      "funding": {
        "url": "https://github.com/sponsors/eemeli"
      }
    },
    "node_modules/zod": {
      "version": "4.6.5",
      "resolved": "https://registry.npmjs.org/zod/-/zod-4.6.5.tgz",
      "integrity": "sha512-v5l/aFXZQeai4awLbOpSoHecE9UiMrnfx75tEXLjNonXVARxQ5mOeipTjROUchszUNCqnE+hqAMujRsRHsut2Q==",
      "license": "MIT",
      "funding": {
        "url": "https://github.com/sponsors/colinhacks"
      }
    },
    "node_modules/zod-to-json-schema": {
      "version": "3.25.2",
      "resolved": "https://registry.npmjs.org/zod-to-json-schema/-/zod-to-json-schema-3.25.2.tgz",
      "integrity": "sha512-O/PgfnpT1xKSDeQYSCfRI5Gy3hPf91mKVDuYLUHZJMiDFptvP41MSnWofm8dnCm0256ZNfZIM7DSzuSMAFnjHA==",
      "license": "ISC",
      "peerDependencies": {
        "zod": "^3.25.28 || ^4"
      }
    }
  }
}
"#;
