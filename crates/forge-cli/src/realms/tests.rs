use super::*;

fn row(name: &str, path: &str, branch: &str, head: &str) -> Row {
    Row {
        name: name.to_string(),
        path: path.to_string(),
        branch: branch.to_string(),
        head: head.to_string(),
    }
}

/// The world, plainly: the map, the journal, one line per realm with its
/// columns aligned. No lore in the machine's mouth (0019 law 4).
#[test]
fn the_world_reads_out_as_the_map_the_journal_and_the_realms() {
    let out = render(
        "./realms.json",
        ".forge/forge.db",
        &[
            row("the-forge", ".", "main", "5a4bf4a"),
            row("lanetally", "../lanetally", "trunk", "0f0f0f0"),
        ],
    );
    assert_eq!(
        out,
        "map      ./realms.json\n\
         journal  .forge/forge.db\n\
         realm    the-forge  .             main   5a4bf4a\n\
         realm    lanetally  ../lanetally  trunk  0f0f0f0\n"
    );
}

/// A realm whose tree has no readable HEAD says so, the way every other
/// readout marks an absent fact.
#[test]
fn a_realm_with_no_readable_head_is_marked_absent() {
    let out = render("m.json", "j.db", &[row("solo", "/tmp/x", "main", NO_HEAD)]);
    assert!(out.ends_with("realm    solo  /tmp/x  main  -\n"), "{out}");
}

/// The map is operator-written, but it still reaches a terminal through
/// the same sanitizer every journal string does.
#[test]
fn a_map_cannot_smuggle_an_escape_sequence_into_the_frame() {
    let out = render(
        "m\u{1b}[2J.json",
        "j.db",
        &[row("a\u{202e}b", ".", "main", "abc")],
    );
    assert!(!out.contains('\u{1b}'), "{out:?}");
    assert!(!out.contains('\u{202e}'), "{out:?}");
    assert!(out.contains("m[2J.json"), "{out}");
}

/// The rows come from the map, and the HEAD from the tree the map names.
/// This workspace is not a git repository, so the honest answer is "-".
#[test]
fn rows_are_the_map_read_against_the_trees_it_names() {
    let dir = tempfile::tempdir().unwrap();
    let map = dir.path().join("realms.json");
    std::fs::write(
        &map,
        r#"{"schema":"forge.realms/v1",
            "realms":[{"name":"solo","path":"tree","default_branch":"main"}],
            "journal":"j.db"}"#,
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("tree")).unwrap();
    let world = World::load(&map).unwrap();
    let rows = rows(&world);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "solo");
    assert_eq!(rows[0].branch, "main");
    assert_eq!(rows[0].head, NO_HEAD);

    // The readout states the journal the OTHER read surfaces would open,
    // which is why the caller resolves it and hands it in.
    let printed = render(
        &world.source.display().to_string(),
        "/elsewhere/forge.db",
        &rows,
    );
    assert!(
        printed.contains("journal  /elsewhere/forge.db"),
        "{printed}"
    );
    assert!(
        printed.contains("realm    solo  tree  main  -"),
        "{printed}"
    );
}

/// `--json` is the same world, unspelled: one derivation of the rows,
/// two renderings, and the data reaches a parser as it is — the frame's
/// escaping is for terminals, not for consumers.
#[test]
fn the_json_view_is_the_same_world_as_the_frame() {
    let rows = [row("solo", "/tmp/x", "main", "abc1234")];
    let seen = view("m.json", "j.db", &rows);
    assert_eq!(
        seen,
        json!({
            "map": "m.json",
            "journal": "j.db",
            "realms": [{
                "name": "solo",
                "path": "/tmp/x",
                "default_branch": "main",
                "head": "abc1234",
            }],
        })
    );
    // Unescaped: a bidi mark the frame strips survives to a parser,
    // which is reading bytes and not painting them.
    let odd = view("m.json", "j.db", &[row("a\u{202e}b", ".", "main", NO_HEAD)]);
    assert_eq!(odd["realms"][0]["name"], json!("a\u{202e}b"));
}
