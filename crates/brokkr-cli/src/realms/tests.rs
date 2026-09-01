use super::*;

fn row(name: &str, path: &str, branch: &str, head: &str) -> Row {
    hearth_row(name, path, branch, head, "j.db")
}

fn hearth_row(name: &str, path: &str, branch: &str, head: &str, journal: &str) -> Row {
    Row {
        name: name.to_string(),
        path: path.to_string(),
        branch: branch.to_string(),
        head: head.to_string(),
        journal: journal.to_string(),
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

/// Decision 0026 ruling 1, read out: a world whose realms sit at
/// different hearths says which is whose; a world whose realms share one
/// says it once, at the top, exactly as it always did.
#[test]
fn a_many_hearth_world_reads_out_each_realms_own_journal() {
    let one = render(
        "m.json",
        "j.db",
        &[
            hearth_row("alpha", "a", "main", "aaa", "j.db"),
            hearth_row("beta", "b", "main", "bbb", "j.db"),
        ],
    );
    assert_eq!(
        one,
        "map      m.json\n\
         journal  j.db\n\
         realm    alpha  a  main  aaa\n\
         realm    beta   b  main  bbb\n"
    );

    let many = render(
        "m.json",
        "j.db",
        &[
            hearth_row("alpha", "a", "main", "aaa", "a/forge.db"),
            hearth_row("beta", "b", "main", "bbb", "j.db"),
        ],
    );
    assert_eq!(
        many,
        "map      m.json\n\
         journal  j.db\n\
         realm    alpha  a  main  aaa  a/forge.db\n\
         realm    beta   b  main  bbb  j.db\n"
    );
}

/// The rows carry each realm's EFFECTIVE journal: its own when the map
/// gives it one, the world's when it does not.
#[test]
fn rows_carry_the_effective_journal_of_every_realm() {
    let dir = tempfile::tempdir().unwrap();
    let map = dir.path().join("realms.json");
    std::fs::write(
        &map,
        r#"{"schema":"forge.realms/v2",
            "realms":[{"name":"alpha","path":"tree","default_branch":"main",
                       "journal":"a/forge.db"},
                      {"name":"beta","path":"tree","default_branch":"main"}],
            "journal":"j.db"}"#,
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("tree")).unwrap();
    let world = World::load(&map).unwrap();
    let rows = rows(&world);
    assert_eq!(
        rows[0].journal,
        dir.path().join("a/forge.db").display().to_string()
    );
    assert_eq!(
        rows[1].journal,
        dir.path().join("j.db").display().to_string()
    );
    // And `--json` carries it too, so a consumer sees the same world.
    let seen = view("m.json", "j.db", &rows);
    assert_eq!(seen["realms"][0]["journal"], json!(rows[0].journal));
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
                "journal": "j.db",
            }],
        })
    );
    // Unescaped: a bidi mark the frame strips survives to a parser,
    // which is reading bytes and not painting them.
    let odd = view("m.json", "j.db", &[row("a\u{202e}b", ".", "main", NO_HEAD)]);
    assert_eq!(odd["realms"][0]["name"], json!("a\u{202e}b"));
}
