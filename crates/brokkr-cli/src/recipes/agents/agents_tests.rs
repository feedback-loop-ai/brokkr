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

/// The workspace's capability definitions, beside its `library`.
fn defined(library: &Path) -> PathBuf {
    library.with_file_name("defined")
}

/// The workspace's root, which holds its `library`.
fn rooted(library: &Path) -> &Path {
    library.parent().expect("a library below its root")
}

/// What [`plan`] makes of a recipe holding `seats` beside `source`.
fn planned(source: &Path, library: &Path, seats: serde_json::Value) -> Result<Vec<Hire>, Hiring> {
    plan(
        &[&recipe(source, seats)],
        Some(source),
        rooted(library),
        library,
        &defined(library),
    )
}

/// Every form a site names an agent in is read: a seat's, a panel
/// member's, a sequence step's, and a select's case and default. Each
/// definition the source holds is planned with the charter it names, and
/// a name the source lacks is the compile's. The copies land with the
/// directories they need; planned again, every file stands identical and
/// nothing is planned, and with no source library, or one that does not
/// stand, nothing ever is.
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
    let absent = source.with_file_name("absent");
    assert_eq!(planned(&absent, &library, seats.clone()), Ok(Vec::new()));
    let root = rooted(&library);
    let unsourced = plan(&[&recipe(&source, seats)], None, root, &library, &library);
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
/// that is not plain or that the source lacks, a link or a file where the
/// walk needs a directory, a link to nothing where a copy is due, and
/// files the library holds otherwise.
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
        plan(
            &[&missing],
            Some(&source),
            rooted(&library),
            &library,
            &library
        ),
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
    let uncharted = Err(Hiring::Uncharted {
        path: definition.clone(),
        charter: "charters/a.md".into(),
        library: source.clone(),
    });
    assert_eq!(planned(&source, &library, one.clone()), uncharted);
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
    let occupied = Err(Hiring::Occupied {
        path: dangling.clone(),
    });
    assert_eq!(planned(&source, &library, one.clone()), occupied);
    assert_eq!(std::fs::read_link(&dangling).unwrap(), Path::new("removed"));
    std::fs::remove_file(&dangling).unwrap();
    write(&dangling, "# other\n");
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

/// Each capability a planned definition requests brings its definition,
/// once, from the `capabilities/` the source's mapless compile reads, beside
/// its library, into the workspace's (#703); a name outside the grammar is
/// never walked, and a definition the workspace holds identical is skipped.
/// One the source lacks refuses, even where the workspace holds one, as do
/// a source map that cannot be read, once a capability is asked, a linked
/// `capabilities/`, and one the workspace holds otherwise.
#[test]
fn plan_copies_the_definitions_its_agents_request() {
    let (_dir, source, library) = libraries();
    let asks = |capabilities: serde_json::Value| json!({"capabilities": capabilities});
    let a = asks(json!({"web-fetch": "wants", "../escape": "wants"}));
    write(&source.join("a.json"), &a.to_string());
    let b = asks(json!({"web-fetch": "requires"}));
    write(&source.join("b.json"), &b.to_string());
    let from = source.with_file_name("capabilities");
    write(&from.join("web-fetch.json"), "{\"name\": \"web-fetch\"}\n");
    write(&from.with_file_name("escape.json"), "outside\n");
    let seats = json!({"a": {"agent": "a"}, "b": {"agent": "b"}});
    let hires = planned(&source, &library, seats.clone()).unwrap();
    let shown: Vec<(&str, &Path)> = hires.iter().map(|h| (&*h.what, &*h.to)).collect();
    let copy_at = defined(&library).join("web-fetch.json");
    let (at_a, at_b) = (library.join("a.json"), library.join("b.json"));
    let expected: [(&str, &Path); 3] = [
        ("capability 'web-fetch' of agent 'a'", &copy_at),
        ("agent 'a'", &at_a),
        ("agent 'b'", &at_b),
    ];
    assert_eq!(shown, expected);
    copy(&hires, &mut Vec::new()).unwrap();
    let copied = std::fs::read(&copy_at).unwrap();
    assert_eq!(copied, b"{\"name\": \"web-fetch\"}\n");
    assert_eq!(planned(&source, &library, seats.clone()), Ok(Vec::new()));

    std::fs::write(&copy_at, "{}").unwrap();
    let paths = vec![copy_at.clone()];
    let differs = planned(&source, &library, seats.clone());
    assert_eq!(differs, Err(Hiring::Differs { paths }));
    std::fs::remove_file(from.join("web-fetch.json")).unwrap();
    let undefined = Err(Hiring::Undefined {
        path: source.join("a.json"),
        capability: "web-fetch".into(),
        definitions: from.clone(),
    });
    assert_eq!(planned(&source, &library, seats.clone()), undefined);
    let root = source.parent().unwrap().to_path_buf();
    let map = root.join("realms.json");
    write(&map, "not json");
    let shown = map.display();
    let cause = format!("{shown} is not a readable realms map: expected ident at line 1 column 2");
    let unread = Err(Hiring::Unread { path: root, cause });
    assert_eq!(planned(&source, &library, seats.clone()), unread);
    write(&source.join("c.json"), "{}");
    let unasked = planned(&source, &library, json!({"c": {"agent": "c"}}));
    assert_eq!(unasked.map(|hires| hires.len()), Ok(1));
    std::fs::remove_file(map).unwrap();
    std::fs::remove_dir(&from).unwrap();
    std::os::unix::fs::symlink(&library, &from).unwrap();
    let unplain = Err(Hiring::Unplain { path: from });
    assert_eq!(planned(&source, &library, seats), unplain);
}

/// A destination is judged as the entry itself before any byte is compared
/// (#703): a link to the source's own bytes where an agent or a capability
/// definition is due, and a directory where a file is due, each refuse by
/// name, the link left as it stands; with nothing there, all are planned.
#[test]
fn plan_refuses_a_destination_that_is_no_plain_file() {
    let (_dir, source, library) = libraries();
    let a = json!({"capabilities": {"web-fetch": "wants"}}).to_string();
    write(&source.join("a.json"), &a);
    let from = source.with_file_name("capabilities");
    write(&from.join("web-fetch.json"), "{}\n");
    let same = source.with_file_name("same");
    write(&same.join("a.json"), &a);
    write(&same.join("web-fetch.json"), "{}\n");
    let definition = defined(&library).join("web-fetch.json");
    let agent = library.join("a.json");
    let seats = json!({"a": {"agent": "a"}});
    let occupied = |path: &Path| Err(Hiring::Occupied { path: path.into() });
    for (at, target) in [
        (&agent, same.join("a.json")),
        (&definition, same.join("web-fetch.json")),
    ] {
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(&target, at).unwrap();
        assert_eq!(planned(&source, &library, seats.clone()), occupied(at));
        assert_eq!(std::fs::read_link(at).unwrap(), target);
        std::fs::remove_file(at).unwrap();
    }
    std::fs::create_dir(&definition).unwrap();
    assert_eq!(
        planned(&source, &library, seats.clone()),
        occupied(&definition)
    );
    std::fs::remove_dir(&definition).unwrap();
    let planned = planned(&source, &library, seats);
    assert_eq!(planned.map(|hires| hires.len()), Ok(2));
}

/// A directory between the workspace's root and a destination that is a
/// link refuses by name before any byte is compared (#735), the link left
/// as it stands: the library linked to a directory inside the root, a
/// charter's directory in it linked outside the root, and the capability
/// definitions linked inside. A file where the library is due refuses by
/// name too, left as it stands (#779).
#[test]
fn plan_refuses_a_destination_beneath_a_linked_directory() {
    let (_dir, source, library) = libraries();
    let root = rooted(&library).to_path_buf();
    let a = json!({"charter": "charters/a.md", "capabilities": {"web-fetch": "wants"}});
    write(&source.join("a.json"), &a.to_string());
    write(&source.join("charters/a.md"), "# a\n");
    let from = source.with_file_name("capabilities");
    write(&from.join("web-fetch.json"), "{}\n");
    let seats = json!({"a": {"agent": "a"}});
    let (inside, outside) = (root.join("inside"), tempfile::tempdir().unwrap());
    std::fs::create_dir(&inside).unwrap();
    let refused = |at: &Path, target: &Path| {
        std::os::unix::fs::symlink(target, at).unwrap();
        let linked = Err(Hiring::Linked { path: at.into() });
        assert_eq!(planned(&source, &library, seats.clone()), linked);
        assert_eq!(std::fs::read_link(at).unwrap(), target);
        std::fs::remove_file(at).unwrap();
    };
    refused(&library, &inside);
    std::fs::create_dir(&library).unwrap();
    refused(&library.join("charters"), outside.path());
    refused(&defined(&library), &inside);
    let entries = |dir: &Path| std::fs::read_dir(dir).unwrap().count();
    assert_eq!((entries(&inside), entries(outside.path())), (0, 0));
    std::fs::remove_dir(&library).unwrap();
    write(&library, "a file");
    let blocked = Err(Hiring::Blocked {
        path: library.clone(),
    });
    assert_eq!(planned(&source, &library, seats), blocked);
    assert_eq!(std::fs::read_to_string(&library).unwrap(), "a file");
}

/// An entry on the walk, or a definition, this process cannot read refuses
/// by name, never taken as absent (fail closed); so does a destination
/// whose entry cannot be inspected, or a directory on the way to it.
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
    let (path, cause) = (definition.clone(), denied.clone());
    assert_eq!(refused, Err(Hiring::Unread { path, cause }));
    mode(&definition, 0o600);
    write(&source.join("locked/a.md"), "# a\n");
    std::fs::create_dir(&library).unwrap();
    mode(&library, 0o000);
    let refused = planned(&source, &library, json!({"a": {"agent": "a"}}));
    mode(&library, 0o700);
    let (path, cause) = (library.join("a.json"), denied.clone());
    assert_eq!(refused, Err(Hiring::Unread { path, cause }));
    let z = json!({"charter": "charters/z.md"}).to_string();
    write(&source.join("z.json"), &z);
    write(&source.join("charters/z.md"), "# z\n");
    mode(&library, 0o000);
    let refused = planned(&source, &library, json!({"z": {"agent": "z"}}));
    mode(&library, 0o700);
    let (path, cause) = (library.join("charters"), denied);
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
            Hiring::Uncharted {
                path: path.clone(),
                charter: "c.md".into(),
                library: PathBuf::from("/s"),
            },
            "/s/a.json names charter 'c.md', which its library /s does not hold, so the copy \
             would bind to whatever this workspace holds there (#696); nothing was written",
        ),
        (
            Hiring::Undefined {
                path: path.clone(),
                capability: "web-fetch".into(),
                definitions: PathBuf::from("/capabilities"),
            },
            "/s/a.json requests capability 'web-fetch', whose definition the source's \
             /capabilities does not hold, so the copy would bind to whatever this workspace \
             defines there (#703); nothing was written",
        ),
        (
            Hiring::Unplain { path: path.clone() },
            "/s/a.json is a link, or not the file or directory recipes add copies from the \
             source's agent library or capability definitions (#696); nothing was written",
        ),
        (
            Hiring::Occupied { path: path.clone() },
            "the workspace's /s/a.json is a link, or not a plain file, where recipes add would \
             copy one; recipes add neither reads through nor replaces it (#703): remove it; \
             nothing was written",
        ),
        (
            Hiring::Linked { path: path.clone() },
            "the workspace's /s/a.json is a link, where recipes add would copy a file beneath it; \
             recipes add neither reads through nor replaces it (#735): remove it; nothing was \
             written",
        ),
        (
            Hiring::Blocked { path: path.clone() },
            "the workspace's /s/a.json is not a directory, where recipes add would copy a file \
             beneath it; recipes add does not replace it (#779): remove it; nothing was written",
        ),
        (
            Hiring::Differs {
                paths: vec![path, PathBuf::from("/s/b.json")],
            },
            "recipes add replaces no agent definition, charter or capability definition, and \
             the source's differ from what stands at /s/a.json, /s/b.json (#696): reconcile or \
             remove each; nothing was written",
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
