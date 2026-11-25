//! The walk's refusals, one variant per check (#353): each is built by the
//! function that runs its check, read from in-memory paths and maps, and
//! every variant's text is pinned once, as literals.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::{
    refuse_secrets_store, sorted_keys, unlisted_consumed, walk_keys, CompileError, Supplied, Walk,
    WalkRefusal,
};

/// The in-memory layer every direct test's paths stand under.
const LAYER: &str = "/walked/layer";

/// `rels`, each under [`LAYER`].
fn under_layer(rels: &[&str]) -> Vec<PathBuf> {
    rels.iter().map(|rel| Path::new(LAYER).join(rel)).collect()
}

/// What a bound read supplied for a consumed key, read for `consumer`.
fn supplied_for(consumer: &str) -> Supplied {
    Supplied {
        digest: "0".repeat(64),
        bytes: 1,
        id: (7, 11),
        target: String::from("unused"),
        consumer: consumer.to_string(),
    }
}

/// The consumed map for each `(key, consumer)`.
fn consumed_of(rows: &[(&str, &str)]) -> BTreeMap<String, Supplied> {
    rows.iter()
        .map(|(key, consumer)| (key.to_string(), supplied_for(consumer)))
        .collect()
}

/// `rels` as the walk lists them, each key its own name.
fn walked_of(rels: &[&str]) -> Vec<(PathBuf, String)> {
    under_layer(rels)
        .into_iter()
        .zip(rels.iter().map(|rel| rel.to_string()))
        .collect()
}

/// Unlisted's words for `consumer`, with the key shown as `shown`.
fn unlisted_words(consumer: &str, shown: &str) -> String {
    format!(
        "{consumer}, which the walk that pins the layer lists under no entry of the name \
         {shown} it was read by: the name reached the file through a case or normalization \
         alias the filesystem accepted, or the entry was removed after the read. A layer's \
         identity names a consumed file by the entry its directory lists, so it is refused; \
         write the reference as its directory lists it (decision 0065 slice one, design D7)"
    )
}

#[test]
fn an_entry_named_secrets_env_is_refused_as_the_store_it_is() {
    let path = Path::new(LAYER).join("x/secrets.env");
    assert_eq!(
        refuse_secrets_store(&path),
        Err(WalkRefusal::SecretsStore { path: &path })
    );
}

#[test]
fn only_the_exact_store_name_is_refused() {
    // `Secrets.env` is admitted: the name test is exact and case-sensitive,
    // kept so (#353's ask), though on a case-insensitive filesystem it can
    // be the very file the store loader opens as `secrets.env`.
    for rel in [
        "secrets.env.example",
        "my-secrets.env",
        "secrets.env/y",
        "Secrets.env",
    ] {
        assert_eq!(
            refuse_secrets_store(&Path::new(LAYER).join(rel)),
            Ok(()),
            "{rel}"
        );
    }
}

#[cfg(unix)]
#[test]
fn a_name_that_is_not_utf8_is_refused_naming_the_whole_path() {
    use std::os::unix::ffi::OsStringExt;
    let bad = std::ffi::OsString::from_vec(b"\xff.sh".to_vec());
    let mut paths = vec![Path::new(LAYER).join("tools").join(&bad)];
    let whole = paths[0].clone();
    assert_eq!(
        sorted_keys(Path::new(LAYER), &mut paths),
        Err(WalkRefusal::NotUtf8 { path: &whole })
    );
}

#[cfg(unix)]
#[test]
fn the_first_bad_path_in_sorted_order_is_the_one_refused() {
    use std::os::unix::ffi::OsStringExt;
    let bad = std::ffi::OsString::from_vec(b"\xff.sh".to_vec());
    // `0` sorts before `@`, so the non-UTF-8 path, listed last, is first.
    let mut paths = under_layer(&["@folder/forged"]);
    paths.push(Path::new(LAYER).join("0").join(&bad));
    let first = Path::new(LAYER).join("0").join(&bad);
    assert_eq!(
        sorted_keys(Path::new(LAYER), &mut paths),
        Err(WalkRefusal::NotUtf8 { path: &first })
    );
}

#[test]
fn a_file_under_a_computed_namespace_is_refused_naming_its_prefix() {
    for (rel, prefix) in [("@folder/x", "@folder/"), ("@compose/x", "@compose/")] {
        let mut paths = under_layer(&["plain.md", rel]);
        assert_eq!(
            sorted_keys(Path::new(LAYER), &mut paths),
            Err(WalkRefusal::Reserved {
                rel: rel.to_string(),
                prefix,
            })
        );
    }
}

#[test]
fn admitted_paths_are_keyed_in_sorted_order_by_their_components() {
    let paths = under_layer(&["z.md", "scripts\\gate.sh", "a/b/c.md"]);
    let walked = walk_keys(Path::new(LAYER), paths).unwrap();
    assert_eq!(
        walked,
        vec![
            (Path::new(LAYER).join("a/b/c.md"), String::from("a/b/c.md")),
            (
                Path::new(LAYER).join("scripts\\gate.sh"),
                String::from("scripts\\gate.sh"),
            ),
            (Path::new(LAYER).join("z.md"), String::from("z.md")),
        ]
    );
}

#[test]
fn the_first_unlisted_consumed_key_is_refused_naming_its_consumer() {
    let consumed = consumed_of(&[
        ("listed.md", "recipe.json's seat 'listed'"),
        ("m-unlisted.md", "recipe.json's seat 'second'"),
        ("k-unlisted.md", "recipe.json's seat 'first'"),
    ]);
    assert_eq!(
        unlisted_consumed(&consumed, &walked_of(&["listed.md", "other.md"])),
        Err(WalkRefusal::Unlisted {
            consumer: "recipe.json's seat 'first'",
            key: "k-unlisted.md",
        })
    );
}

#[test]
fn a_listed_consumed_key_is_admitted() {
    let consumed = consumed_of(&[("roles/r.md", "layer.json's role 'r'")]);
    assert_eq!(
        unlisted_consumed(&consumed, &walked_of(&["a.md", "roles/r.md"])),
        Ok(())
    );
}

#[test]
fn an_unlisted_key_too_long_to_echo_is_named_by_its_bounded_form() {
    let key = "q".repeat(200);
    let consumed = consumed_of(&[(&key, "the reader")]);
    let refusal = unlisted_consumed(&consumed, &[]).unwrap_err();
    let shown = format!("'{}…' (200 bytes, not echoed in full)", "q".repeat(64));
    assert_eq!(refusal.to_string(), unlisted_words("the reader", &shown));
}

#[test]
fn a_secrets_store_in_a_real_layer_is_refused_by_the_walk() {
    let layer = tempfile::tempdir().unwrap();
    let dir = layer.path().canonicalize().unwrap();
    std::fs::create_dir(dir.join("conf")).unwrap();
    std::fs::write(dir.join("conf/secrets.env"), "TOKEN=x\n").unwrap();
    std::fs::write(dir.join("conf/plain.md"), "plain\n").unwrap();
    let refusal = match Walk::list(&dir, &dir, &BTreeMap::new()) {
        Err(CompileError::Invalid(text)) => text,
        other => panic!("not refused as invalid: {other:?}"),
    };
    let store = dir.join("conf/secrets.env");
    assert_eq!(
        refusal,
        WalkRefusal::SecretsStore { path: &store }.to_string()
    );
}

#[test]
fn every_walk_refusal_reads_as_it_always_has() {
    let rows = [
        (
            WalkRefusal::SecretsStore {
                path: Path::new("/l/x/secrets.env"),
            },
            "bundle contains a secrets store '/l/x/secrets.env'; the store must live outside \
             the bundle dir (e.g. .forge/secrets.env) so digests carry names only",
        ),
        (
            WalkRefusal::NotUtf8 {
                path: Path::new("/l/odd.sh"),
            },
            "bundle filename '/l/odd.sh' is not UTF-8; manifest keys must preserve exact names",
        ),
        (
            WalkRefusal::Reserved {
                rel: String::from("@compose/x"),
                prefix: "@compose/",
            },
            "bundle file '@compose/x' uses the reserved '@compose/' namespace; composition \
             provenance is computed by the resolver, never supplied by a bundle",
        ),
    ];
    for (refusal, text) in rows {
        assert_eq!(refusal.to_string(), text);
    }
    let unlisted = WalkRefusal::Unlisted {
        consumer: "c.json's seat 's'",
        key: "r.md",
    };
    assert_eq!(
        unlisted.to_string(),
        unlisted_words("c.json's seat 's'", "'r.md'")
    );
}
