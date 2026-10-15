use super::*;
use serde_json::json;

/// A source agent library, standing, and a workspace's place for one,
/// empty, side by side.
fn libraries() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let source = root.join("source");
    std::fs::create_dir(&source).unwrap();
    (dir, source, root.join("library"))
}

/// `text` at `path`, with the directories it needs.
fn write(path: &Path, text: &str) {
    let parent = path.parent().expect("a file below a directory");
    std::fs::create_dir_all(parent).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A recipe beside `source` whose `bundle.json` holds `seats`.
fn recipe(source: &Path, seats: serde_json::Value) -> PathBuf {
    let recipe = source.with_file_name("recipe");
    write(
        &recipe.join("bundle.json"),
        &json!({"seats": seats}).to_string(),
    );
    recipe
}

/// What [`plan`] makes of a recipe holding `seats` beside `source`.
fn planned(source: &Path, library: &Path, seats: serde_json::Value) -> Result<Vec<Hire>, Hiring> {
    plan(&[&recipe(source, seats)], Some(source), library)
}

/// Every form a site names an agent in is read: a seat's, a panel
/// member's, a sequence step's, and a select's case and default. Each
/// definition the source holds is planned with the charter it names, and
/// a name the source lacks is the compile's. The copies land with the
/// directories they need; planned again, every file stands identical and
/// nothing is planned, and with no source library nothing ever is.
#[test]
fn plan_reads_every_site_form_and_copies_what_the_library_lacks() {
    let (_dir, source, library) = libraries();
    for name in ["member", "step", "case", "fallback"] {
        write(&source.join(format!("{name}.json")), "{}");
    }
    let single = json!({"charter": "charters/single.md"}).to_string();
    write(&source.join("single.json"), &single);
    write(&source.join("charters/single.md"), "# single\n");
    let seats = json!({
        "a": {"agent": "single"},
        "b": {"panel": {"x": {"agent": "member"}, "y": {"role": "r.md"}}},
        "c": {"sequence": [{"agent": "step"}, {"name": "inline"}]},
        "d": {"select": {"cases": {"fast": {"agent": "case"}}, "default": {"agent": "fallback"}}},
        "e": {"agent": "missing"},
    });
    let hires = planned(&source, &library, seats.clone()).unwrap();
    let shown: Vec<(&str, &Path)> = hires.iter().map(|h| (&*h.what, &*h.to)).collect();
    let at = |file: &str| library.join(file);
    let (case, charter, fallback) = (
        at("case.json"),
        at("charters/single.md"),
        at("fallback.json"),
    );
    let (member, one, step) = (at("member.json"), at("single.json"), at("step.json"));
    let expected: [(&str, &Path); 6] = [
        ("agent 'case'", &case),
        ("charter charters/single.md of agent 'single'", &charter),
        ("agent 'fallback'", &fallback),
        ("agent 'member'", &member),
        ("agent 'single'", &one),
        ("agent 'step'", &step),
    ];
    assert_eq!(shown, expected);

    let mut made = Vec::new();
    copy(&hires, &mut made).unwrap();
    let charters = library.join("charters");
    let expected = [
        &library, &case, &charters, &charter, &fallback, &member, &one, &step,
    ];
    assert_eq!(made.iter().collect::<Vec<_>>(), expected);
    let copied = std::fs::read(&charter).unwrap();
    assert_eq!(copied, b"# single\n");
    assert_eq!(planned(&source, &library, seats.clone()), Ok(Vec::new()));
    let unsourced = plan(&[&recipe(&source, seats)], None, &library);
    assert_eq!(unsourced, Ok(Vec::new()));

    let mut again = Vec::new();
    let refused = copy(&hires[..1], &mut again).unwrap_err();
    assert_eq!(
        (refused.kind(), again),
        (std::io::ErrorKind::AlreadyExists, vec![])
    );
}

/// Each refusal by its variant, before anything is written: a bundle or a
/// definition that cannot be read, a name outside the grammar, a charter
/// that is not plain, a link or a file where the walk needs a directory,
/// and files the library holds otherwise, a link to nothing included.
#[test]
fn plan_refuses_by_variant_what_it_cannot_read_or_would_replace() {
    let (_dir, source, library) = libraries();
    let one = json!({"a": {"agent": "a"}});
    let missing = source.with_file_name("bare");
    std::fs::create_dir(&missing).unwrap();
    let unread = |path: &Path, cause: &str| Hiring::Unread {
        path: path.into(),
        cause: cause.into(),
    };
    let bundle = missing.join("bundle.json");
    let absent = "No such file or directory (os error 2)";
    assert_eq!(
        plan(&[&missing], Some(&source), &library),
        Err(unread(&bundle, absent))
    );
    let typed = planned(&source, &library, json!({"a": {"agent": 5}}));
    let at = source.with_file_name("recipe/bundle.json");
    let cause = "invalid type: integer `5`, expected a string at line 1 column 24";
    assert_eq!(typed, Err(unread(&at, cause)));
    let named = planned(&source, &library, json!({"a": {"agent": "Bad_Name"}}));
    let name = "Bad_Name".into();
    assert_eq!(named, Err(Hiring::Unnamed { path: at, name }));

    let definition = source.join("a.json");
    write(&definition, "not json");
    let cause = "expected ident at line 1 column 2";
    assert_eq!(
        planned(&source, &library, one.clone()),
        Err(unread(&definition, cause))
    );
    write(&definition, &json!({"charter": "../a.md"}).to_string());
    let (path, charter) = (definition.clone(), "../a.md".into());
    let refused = Err(Hiring::Charter { path, charter });
    assert_eq!(planned(&source, &library, one.clone()), refused);
    write(
        &definition,
        &json!({"charter": "charters/a.md"}).to_string(),
    );
    write(&source.join("charters"), "a file");
    let unplain = |path: PathBuf| Err(Hiring::Unplain { path });
    let refused = planned(&source, &library, one.clone());
    assert_eq!(refused, unplain(source.join("charters")));

    std::fs::remove_file(source.join("charters")).unwrap();
    write(&source.join("charters/a.md"), "# a\n");
    write(&library.join("a.json"), "{}");
    let dangling = library.join("charters/a.md");
    std::fs::create_dir(library.join("charters")).unwrap();
    std::os::unix::fs::symlink("removed", &dangling).unwrap();
    let paths = vec![library.join("a.json"), dangling];
    let differs = planned(&source, &library, one.clone());
    assert_eq!(differs, Err(Hiring::Differs { paths }));

    let linked = source.with_file_name("linked");
    std::os::unix::fs::symlink(&definition, source.join("b.json")).unwrap();
    std::os::unix::fs::symlink(&source, &linked).unwrap();
    let refused = planned(&source, &library, json!({"b": {"agent": "b"}}));
    assert_eq!(refused, unplain(source.join("b.json")));
    assert_eq!(planned(&linked, &library, one), unplain(linked));
}

/// An entry on the walk, or a definition, this process cannot read refuses
/// by name, never taken as absent (fail closed).
#[test]
fn plan_refuses_what_it_is_denied() {
    use std::os::unix::fs::PermissionsExt;
    if rustix::process::geteuid().is_root() {
        return;
    }
    let (_dir, source, library) = libraries();
    let mode = |path: &Path, mode| {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap()
    };
    let denied = "Permission denied (os error 13)".to_string();
    let definition = source.join("a.json");
    write(&definition, &json!({"charter": "locked/a.md"}).to_string());
    std::fs::create_dir(source.join("locked")).unwrap();
    mode(&source.join("locked"), 0o000);
    let refused = planned(&source, &library, json!({"a": {"agent": "a"}}));
    mode(&source.join("locked"), 0o700);
    let (path, cause) = (source.join("locked/a.md"), denied.clone());
    assert_eq!(refused, Err(Hiring::Unread { path, cause }));
    mode(&definition, 0o000);
    let refused = planned(&source, &library, json!({"a": {"agent": "a"}}));
    let (path, cause) = (definition, denied);
    assert_eq!(refused, Err(Hiring::Unread { path, cause }));
}

/// Each refusal's words, pinned once.
#[test]
fn each_refusal_says_what_was_not_written() {
    let path = PathBuf::from("/s/a.json");
    let texts = [
        (
            Hiring::Unread {
                path: path.clone(),
                cause: "why".into(),
            },
            "recipes add cannot read /s/a.json: why (#696); nothing was written",
        ),
        (
            Hiring::Unnamed {
                path: path.clone(),
                name: "B".into(),
            },
            "/s/a.json names agent 'B', which is not an agent name (^[a-z][a-z0-9-]*$) (#696); \
             nothing was written",
        ),
        (
            Hiring::Charter {
                path: path.clone(),
                charter: "../c.md".into(),
            },
            "/s/a.json names charter '../c.md', which is not a relative path of plain names in \
             its library (#696); nothing was written",
        ),
        (
            Hiring::Unplain { path: path.clone() },
            "/s/a.json is a link, or not the file or directory recipes add copies from the \
             source's agent library (#696); nothing was written",
        ),
        (
            Hiring::Differs {
                paths: vec![path, PathBuf::from("/s/b.json")],
            },
            "recipes add replaces no agent definition or charter, and the source's differ from \
             what stands at /s/a.json, /s/b.json (#696): reconcile or remove each; nothing was \
             written",
        ),
    ];
    for (refusal, text) in texts {
        assert_eq!(refusal.to_string(), text);
    }
}

/// A local source's agent library stands beside the library its bundle is
/// in, a recipe library's or a folder's shelf; any other source has none.
#[test]
fn a_local_sources_agents_stand_beside_its_library() {
    let (_dir, source, _) = libraries();
    let at = |path: &str| {
        let bundle = source.join(path);
        std::fs::create_dir_all(&bundle).unwrap();
        library_of(&bundle).unwrap()
    };
    assert_eq!(at("lib/recipes/x"), Some(source.join("lib/agents")));
    assert_eq!(at(".brokkr/recipes/x"), Some(source.join(".brokkr/agents")));
    assert_eq!(at("elsewhere/x"), None);
}
