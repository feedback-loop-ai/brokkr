//! `layers.rs`'s own unit tests: each check of a layer's read and of its
//! `extends` refuses in its own variant, and every variant reads as the text
//! the operator has always seen (#353).

use super::*;

/// A library directory on disk holding recipe directories by name, each
/// with a `bundle.json` of the given text.
struct Shelf {
    dir: tempfile::TempDir,
}

impl Shelf {
    fn new() -> Shelf {
        Shelf {
            dir: tempfile::tempdir().unwrap(),
        }
    }

    /// The library, canonical, as `base_library` hands it back.
    fn library(&self) -> PathBuf {
        self.dir.path().canonicalize().unwrap()
    }

    /// The recipe `name`, its document `text`, as the canonical directory.
    fn recipe(&self, name: &str, text: &str) -> PathBuf {
        let dir = self.library().join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("bundle.json"), text).unwrap();
        dir
    }
}

/// Read the layer at `dir` onto `layers` as a library recipe.
fn read(layers: &mut Vec<Layer>, dir: &Path) -> Result<Option<Value>, CompileError> {
    read_layer(layers, (dir, Origin::Library), &None)
}

/// The text of a compile refusal that must be `Invalid`.
fn invalid_text(refusal: CompileError) -> String {
    match refusal {
        CompileError::Invalid(text) => text,
        other => panic!("not an invalid layer: {other:?}"),
    }
}

/// A document parsed as `read_layer` parses one.
fn document(value: Value) -> Map<String, Value> {
    serde_json::from_value(value).unwrap()
}

#[test]
fn an_extends_is_refused_by_its_own_check() {
    let file = Path::new("/shelf/leaf/bundle.json");
    assert_eq!(
        base_of(file, &serde_json::json!(7)),
        Err(LayerRefusal::NotAString { file })
    );
    for base in ["../x", "a/b", "SDD", ".", ""] {
        assert_eq!(
            base_of(file, &Value::from(base)),
            Err(LayerRefusal::NotARecipeName { file, base }),
            "{base}"
        );
    }
    assert_eq!(base_of(file, &Value::from("base-2")), Ok("base-2"));
}

#[test]
fn a_layer_s_name_is_refused_by_its_own_check() {
    let shelf = Shelf::new();
    let base = shelf.recipe("base", r#"{"name": "base"}"#);
    let mut layers = Vec::new();
    assert_eq!(read(&mut layers, &base).unwrap(), None);
    let file = shelf.library().join("leaf/bundle.json");
    assert_eq!(
        name_of(
            &document(serde_json::json!({"extends": "base"})),
            &file,
            &layers
        ),
        Err(LayerRefusal::Nameless { file: &file })
    );
    assert_eq!(
        name_of(&document(serde_json::json!({"name": 7})), &file, &layers),
        Err(LayerRefusal::Nameless { file: &file })
    );
    let prior = base.join("bundle.json");
    assert_eq!(
        name_of(
            &document(serde_json::json!({"name": "base"})),
            &file,
            &layers
        ),
        Err(LayerRefusal::NameTaken {
            file: &file,
            name: "base",
            prior: &prior,
        })
    );
    let named = document(serde_json::json!({"name": "leaf"}));
    assert_eq!(name_of(&named, &file, &layers), Ok("leaf"));
}

#[test]
fn a_base_is_refused_by_its_own_containment_check() {
    let shelf = Shelf::new();
    let library = shelf.library();
    let file = library.join("leaf/bundle.json");
    let absent = library.join("absent");
    assert!(!absent.is_dir());
    assert_eq!(
        contained((&file, "absent"), &library, None),
        Err(LayerRefusal::NotARecipe {
            file: &file,
            base: "absent",
            library: &library,
        })
    );
    let base = shelf.recipe("base", "{}");
    assert_eq!(
        contained(
            (&file, "base"),
            &library,
            Some(base.canonicalize().unwrap())
        ),
        Ok(base)
    );
}

/// Symlinks are a unix concept here, as in `compose_tests.rs`.
#[cfg(unix)]
#[test]
fn a_base_linked_out_of_its_library_is_refused_as_outside() {
    let shelf = Shelf::new();
    let library = shelf.library();
    let away = Shelf::new();
    let outside = away.recipe("elsewhere", "{}");
    std::os::unix::fs::symlink(&outside, library.join("linked")).unwrap();
    let file = library.join("leaf/bundle.json");
    // Taken from `canonicalize`, never from the planted path: macOS
    // resolves /var to /private/var.
    let resolved = library.join("linked").canonicalize().unwrap();
    assert_eq!(
        contained((&file, "linked"), &library, Some(resolved.clone())),
        Err(LayerRefusal::Outside {
            file: &file,
            base: "linked",
            resolved,
            library: &library,
        })
    );
}

#[test]
fn a_declaring_document_s_place_and_parse_refuse_by_their_own_checks() {
    let shelf = Shelf::new();
    let placed = shelf.library().join("placed");
    std::fs::create_dir_all(placed.join("bundle.json")).unwrap();
    let file = placed.join("bundle.json");
    let place = "which is not a regular file; only a regular file's bytes are read, hashed and \
                 pinned, and a FIFO, device or directory could supply bytes the walk never \
                 hashed";
    assert_eq!(
        invalid_text(read(&mut Vec::new(), &placed).unwrap_err()),
        LayerRefusal::Place {
            file: &file,
            place: place.to_string(),
        }
        .to_string()
    );
    let twice = shelf.recipe("twice", r#"{"name": "a", "name": "b"}"#);
    let file = twice.join("bundle.json");
    assert_eq!(
        invalid_text(read(&mut Vec::new(), &twice).unwrap_err()),
        LayerRefusal::Parse {
            file: &file,
            problem: "key 'name' is written twice at line 1 column 26".to_string(),
        }
        .to_string()
    );
}

#[test]
fn every_layer_refusal_reads_as_it_always_has() {
    let file = Path::new("/shelf/leaf/bundle.json");
    let library = Path::new("/shelf");
    let cases = [
        (
            LayerRefusal::Place {
                file,
                place: "which is a FIFO".to_string(),
            },
            "/shelf/leaf/bundle.json: the layer's declaring document, which is a FIFO. What a \
             run is composed from must be what its identity names, so it is refused (decision \
             0065 slice one, design D7)",
        ),
        (
            LayerRefusal::Parse {
                file,
                problem: "trailing comma at line 2 column 1".to_string(),
            },
            "/shelf/leaf/bundle.json: trailing comma at line 2 column 1",
        ),
        (
            LayerRefusal::Nameless { file },
            "/shelf/leaf/bundle.json missing 'name'",
        ),
        (
            LayerRefusal::NameTaken {
                file,
                name: "base",
                prior: Path::new("/shelf/base/bundle.json"),
            },
            "/shelf/leaf/bundle.json declares name 'base', which its ancestor \
             /shelf/base/bundle.json already declares; a derived recipe states its own name so \
             runs, manifests and dispatch pins never report a base's name under a derived \
             digest",
        ),
        (
            LayerRefusal::NotARecipe {
                file,
                base: "absent",
                library,
            },
            "/shelf/leaf/bundle.json: extends 'absent', which is not a recipe in /shelf",
        ),
        (
            LayerRefusal::Outside {
                file,
                base: "linked",
                resolved: PathBuf::from("/elsewhere/linked"),
                library,
            },
            "/shelf/leaf/bundle.json: extends 'linked', which resolves outside the library to \
             /elsewhere/linked; a base is composed from and mounted into confined seats, so it \
             must be a real directory in /shelf",
        ),
        (
            LayerRefusal::NotAString { file },
            "/shelf/leaf/bundle.json: 'extends' must be the name of a recipe in the library",
        ),
        (
            LayerRefusal::NotARecipeName { file, base: "SDD" },
            "/shelf/leaf/bundle.json: 'extends' value 'SDD' is not a recipe name; names match \
             ^[a-z0-9][a-z0-9-]*$ and are refused before any path is built",
        ),
    ];
    for (refusal, text) in cases {
        assert_eq!(refusal.to_string(), text, "{refusal:?}");
    }
}
