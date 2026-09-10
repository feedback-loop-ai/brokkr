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
        publishes: Vec::new(),
        consumes: Vec::new(),
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
            row("brokkr", ".", "main", "5a4bf4a"),
            row("lanetally", "../lanetally", "trunk", "0f0f0f0"),
        ],
        false,
    );
    assert_eq!(
        out,
        "map      ./realms.json\n\
         journal  .forge/forge.db\n\
         realm    brokkr     .             main   5a4bf4a\n\
         realm    lanetally  ../lanetally  trunk  0f0f0f0\n"
    );
}

/// A realm whose tree has no readable HEAD says so, the way every other
/// readout marks an absent fact.
#[test]
fn a_realm_with_no_readable_head_is_marked_absent() {
    let out = render(
        "m.json",
        "j.db",
        &[row("solo", "/tmp/x", "main", NO_HEAD)],
        false,
    );
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
        false,
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
        per_realm(&world, &rows),
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
        false,
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
        true,
    );
    assert_eq!(
        many,
        "map      m.json\n\
         journal  j.db\n\
         realm    alpha  a  main  aaa  a/forge.db\n\
         realm    beta   b  main  bbb  j.db\n"
    );
}

/// The column is decided against the WORLD's journal, not against the
/// other realms. A map whose realms all name one journal OF THEIR OWN
/// holds no disagreement between realms — and would otherwise print a
/// header naming a journal no realm reads, with no column to correct it.
/// The other side of the same rule: `--db` renames the header for one
/// invocation without changing what the map says, so a v1 world read
/// with `--db` still grows no column.
#[test]
fn the_journal_column_answers_to_the_world_and_not_to_the_other_realms() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("tree")).unwrap();
    let map = dir.path().join("realms.json");
    std::fs::write(
        &map,
        r#"{"schema":"forge.realms/v2",
            "realms":[{"name":"alpha","path":"tree","default_branch":"main",
                       "journal":"shared.db"},
                      {"name":"beta","path":"tree","default_branch":"main",
                       "journal":"shared.db"}],
            "journal":"j.db"}"#,
    )
    .unwrap();
    let world = World::load(&map).unwrap();
    let agreed = rows(&world);
    assert!(
        per_realm(&world, &agreed),
        "the realms agree with each other and not with the world"
    );
    let printed = render("m.json", "j.db", &agreed, per_realm(&world, &agreed));
    assert!(
        printed
            .lines()
            .filter(|line| line.starts_with("realm"))
            .all(|line| line.ends_with("shared.db")),
        "{printed}"
    );

    // A v1 world, read with `--db`: the header is the operator's
    // journal, the map still says one thing, and no column appears.
    let v1 = dir.path().join("v1.json");
    std::fs::write(
        &v1,
        r#"{"schema":"forge.realms/v1",
            "realms":[{"name":"solo","path":"tree","default_branch":"main"}],
            "journal":"j.db"}"#,
    )
    .unwrap();
    let world = World::load(&v1).unwrap();
    let plain = rows(&world);
    assert!(!per_realm(&world, &plain));
    let printed = render("v1.json", "/elsewhere/forge.db", &plain, false);
    assert!(
        printed.ends_with("realm    solo  tree  main  -\n"),
        "{printed}"
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

// -------------------- crossings, read out (decision 0057, slice vi)

/// A row carrying crossings, built the way [`rows`] builds one.
fn crossing_row(
    name: &str,
    path: &str,
    head: &str,
    publishes: Vec<Published>,
    consumes: Vec<Consumed>,
) -> Row {
    Row {
        name: name.to_string(),
        path: path.to_string(),
        branch: "main".to_string(),
        head: head.to_string(),
        journal: "j.db".to_string(),
        publishes,
        consumes,
    }
}

fn published(name: &str, path: &str) -> Published {
    Published {
        name: name.to_string(),
        path: path.to_string(),
    }
}

fn consumed(name: &str, publisher: &str, pin: Pin) -> Consumed {
    Consumed {
        name: name.to_string(),
        publisher: publisher.to_string(),
        pin,
    }
}

/// The three states a consumed pin can be in are three distinct words,
/// and the two that have a why carry it — the refusal's own words, never
/// composed here. A realm that draws no crossing adds nothing at all.
#[test]
fn the_three_pin_states_render_distinctly_under_their_realm() {
    let rows = vec![
        crossing_row(
            "brokkr",
            ".",
            "aaa",
            vec![published("orders.api", "contracts/orders.v1.schema.json")],
            Vec::new(),
        ),
        crossing_row(
            "client",
            "client",
            "bbb",
            Vec::new(),
            vec![
                consumed("orders.api", "brokkr", Pin::Matching),
                consumed(
                    "billing.api",
                    "brokkr",
                    Pin::Moved(
                        "realm 'client' consumes crossing 'billing.api' from realm \
                         'brokkr' pinned at aaa, but x hashes to bbb"
                            .to_string(),
                    ),
                ),
                consumed(
                    "audit.api",
                    "ledger",
                    Pin::Unchecked(
                        "realm 'ledger' publishes it and its file could not be read".to_string(),
                    ),
                ),
            ],
        ),
    ];
    let out = render("m.json", "j.db", &rows, false);
    // Published under the publishing realm, with the path.
    assert!(
        out.contains("  publishes  orders.api   contracts/orders.v1.schema.json\n"),
        "{out}"
    );
    // Three consumed lines, three words, and the publisher spelled.
    assert!(
        out.contains("  consumes   orders.api   brokkr  matching\n"),
        "{out}"
    );
    assert!(
        out.contains("  consumes   billing.api  brokkr  moved · realm 'client'"),
        "{out}"
    );
    assert!(
        out.contains("  consumes   audit.api    ledger  unchecked · realm 'ledger'"),
        "{out}"
    );
    // The two have different publishers, so the word alone is not the
    // whole answer: the publishing realm is attributed per line.
    assert!(out.contains("brokkr  moved"), "{out}");
    assert!(out.contains("ledger  unchecked"), "{out}");

    // The same three states as values a script branches on, with the
    // prose beside only where the word leaves something unsaid.
    let seen = view("m.json", "j.db", &rows);
    assert_eq!(
        seen["realms"][0]["publishes"],
        json!([{"name": "orders.api", "path": "contracts/orders.v1.schema.json"}])
    );
    assert_eq!(
        seen["realms"][1]["consumes"],
        json!([
            {"name": "orders.api", "realm": "brokkr", "pin": "matching", "detail": null},
            {"name": "billing.api", "realm": "brokkr", "pin": "moved",
             "detail": "realm 'client' consumes crossing 'billing.api' from realm \
                        'brokkr' pinned at aaa, but x hashes to bbb"},
            {"name": "audit.api", "realm": "ledger", "pin": "unchecked",
             "detail": "realm 'ledger' publishes it and its file could not be read"},
        ])
    );
    // A realm that draws none does not appear: no key, not an empty one.
    let none = view("m.json", "j.db", &[row("solo", ".", "main", "abc")]);
    assert!(none["realms"][0].get("publishes").is_none(), "{none}");
    assert!(none["realms"][0].get("consumes").is_none(), "{none}");
}

/// A world with no crossing renders exactly the bytes it rendered before
/// this slice: the crossings section is silent, not empty headings.
#[test]
fn a_world_that_draws_no_crossing_reads_out_as_it_always_did() {
    let rows = [row("brokkr", ".", "main", "5a4bf4a")];
    assert_eq!(
        render("m.json", "j.db", &rows, false),
        "map      m.json\n\
         journal  j.db\n\
         realm    brokkr  .  main  5a4bf4a\n"
    );
}

/// The map and the published path both reach a terminal through the same
/// sanitizer every journal string does, so a crossing name cannot forge
/// the line below it.
#[test]
fn a_crossing_cannot_smuggle_a_control_sequence_into_the_frame() {
    let rows = [crossing_row(
        "r\u{1b}[2J",
        ".",
        "abc",
        vec![published("orders\u{202e}.api", "p\u{1b}[31m")],
        vec![consumed(
            "b\u{1b}.api",
            "r\u{202e}",
            Pin::Moved("safe\rforged: every pin matches".to_string()),
        )],
    )];
    let out = render("m.json", "j.db", &rows, false);
    assert!(!out.contains('\u{1b}'), "{out:?}");
    assert!(!out.contains('\r'), "{out:?}");
    assert!(!out.contains('\u{202e}'), "{out:?}");
}

/// [`rows`] reads every pin's state off the ONE report `World::load`
/// already built: matching while the bytes hold, moved once they do not,
/// and unchecked when the publisher's own file cannot be read — never
/// counted as matching. A moved or unchecked crossing is a line, not a
/// refusal, because this is a read surface.
#[test]
fn rows_derive_each_pin_state_from_the_one_loaded_report() {
    let dir = tempfile::tempdir().unwrap();
    let contracts = dir.path().join("contracts");
    std::fs::create_dir_all(&contracts).unwrap();
    std::fs::create_dir_all(dir.path().join("client")).unwrap();
    let file = contracts.join("orders.v1.schema.json");
    let bytes = "{\"title\": \"orders\"}\n";
    std::fs::write(&file, bytes).unwrap();
    let pin = brokkr_core::canonical::sha256_bytes(bytes.as_bytes());
    let map = dir.path().join("realms.json");
    std::fs::write(
        &map,
        json!({
            "schema": "forge.realms/v5",
            "realms": [
                {"name": "brokkr", "path": ".", "default_branch": "main",
                 "publishes": [{"name": "orders.api",
                                "path": "contracts/orders.v1.schema.json"}]},
                {"name": "client", "path": "client", "default_branch": "main",
                 "consumes": [{"name": "orders.api", "realm": "brokkr", "sha256": pin}]},
            ],
            "journal": "j.db",
        })
        .to_string(),
    )
    .unwrap();

    // Sound: the pin is the publisher's bytes.
    let world = World::inspect(dir.path(), Some(&map)).unwrap().unwrap();
    let seen = rows(&world);
    let publisher = seen.iter().find(|row| row.name == "brokkr").unwrap();
    assert_eq!(publisher.publishes.len(), 1);
    assert_eq!(publisher.publishes[0].name, "orders.api");
    assert_eq!(
        publisher.publishes[0].path,
        "contracts/orders.v1.schema.json"
    );
    let consumer = seen.iter().find(|row| row.name == "client").unwrap();
    assert_eq!(consumer.consumes.len(), 1);
    assert_eq!(consumer.consumes[0].publisher, "brokkr");
    assert_eq!(consumer.consumes[0].pin.word(), "matching");
    assert_eq!(consumer.consumes[0].pin.detail(), None);

    // Moved: one byte in the publisher's tree, and the consumer's pin is
    // the refusal `run` would give, read out of the error itself.
    std::fs::write(&file, "{\"title\": \"Orders\"}\n").unwrap();
    let observed = brokkr_core::canonical::sha256_bytes(&std::fs::read(&file).unwrap());
    let world = World::inspect(dir.path(), Some(&map)).unwrap().unwrap();
    let seen = rows(&world);
    let consumer = seen.iter().find(|row| row.name == "client").unwrap();
    assert_eq!(consumer.consumes[0].pin.word(), "moved");
    let detail = consumer.consumes[0].pin.detail().unwrap();
    assert!(
        detail.contains("realm 'client' consumes crossing 'orders.api'"),
        "{detail}"
    );
    assert!(detail.contains(&pin), "the pinned digest: {detail}");
    assert!(detail.contains(&observed), "the observed digest: {detail}");

    // Unchecked: the publisher's file is gone, so nothing was compared.
    // The publisher still declares its crossing; the consumer's pin is
    // never called matching.
    std::fs::remove_file(&file).unwrap();
    let world = World::inspect(dir.path(), Some(&map)).unwrap().unwrap();
    let seen = rows(&world);
    let publisher = seen.iter().find(|row| row.name == "brokkr").unwrap();
    assert_eq!(publisher.publishes.len(), 1);
    let consumer = seen.iter().find(|row| row.name == "client").unwrap();
    assert_eq!(consumer.consumes[0].pin.word(), "unchecked");
    let detail = consumer.consumes[0].pin.detail().unwrap();
    assert!(
        detail.contains("realm 'brokkr' publishes it and its file could not be read"),
        "{detail}"
    );

    // And the readout itself never refuses: the world loads as a line.
    let out = render("realms.json", "j.db", &seen, per_realm(&world, &seen));
    assert!(out.contains("unchecked"), "{out}");
}

/// Phase 2 slice (vi): every crossing readout consumes the ONE report the
/// loader already built and never reopens a crossing. The proof is
/// behavioral rather than a source scan: a world is loaded while the pin
/// matches, the published file is then overwritten, and both readouts
/// still report the pin as it was when the world loaded. A second
/// resolver or a second hash would have seen the new bytes and said
/// `moved`.
#[test]
fn a_loaded_readout_never_reopens_the_crossing_it_was_loaded_from() {
    let dir = tempfile::tempdir().unwrap();
    let contracts = dir.path().join("contracts");
    std::fs::create_dir_all(&contracts).unwrap();
    std::fs::create_dir_all(dir.path().join("client")).unwrap();
    let file = contracts.join("orders.v1.schema.json");
    let bytes = "{\"title\": \"orders\"}\n";
    std::fs::write(&file, bytes).unwrap();
    let pin = brokkr_core::canonical::sha256_bytes(bytes.as_bytes());
    let map = dir.path().join("realms.json");
    std::fs::write(
        &map,
        json!({
            "schema": "forge.realms/v5",
            "realms": [
                {"name": "brokkr", "path": ".", "default_branch": "main",
                 "publishes": [{"name": "orders.api",
                                "path": "contracts/orders.v1.schema.json"}]},
                {"name": "client", "path": "client", "default_branch": "main",
                 "consumes": [{"name": "orders.api", "realm": "brokkr", "sha256": pin}]},
            ],
            "journal": "j.db",
        })
        .to_string(),
    )
    .unwrap();

    let world = World::inspect(dir.path(), Some(&map)).unwrap().unwrap();
    let seen = rows(&world);
    let consumer = seen.iter().find(|row| row.name == "client").unwrap();
    assert_eq!(consumer.consumes[0].pin.word(), "matching");
    assert!(crate::muninn::world_crossings(Some(&world))
        .iter()
        .find(|realm| realm.realm == "client")
        .unwrap()
        .consumes
        .iter()
        .all(|consumed| consumed.pin.word() == "matching"));

    // One byte moves on disk AFTER the world was loaded.
    std::fs::write(&file, "{\"title\": \"Orders\"}\n").unwrap();

    // The already-loaded world still answers from its own report — a
    // readout that re-read the file would now say `moved`.
    let seen = rows(&world);
    let consumer = seen.iter().find(|row| row.name == "client").unwrap();
    assert_eq!(
        consumer.consumes[0].pin.word(),
        "matching",
        "the readout reopened the crossing and disagreed with the loaded world"
    );
    let carried = crate::muninn::world_crossings(Some(&world));
    let consumer = carried
        .iter()
        .find(|realm| realm.realm == "client")
        .unwrap();
    assert_eq!(
        consumer.consumes[0].pin.word(),
        "matching",
        "muninn reopened the crossing and disagreed with the loaded world"
    );

    // A fresh inspection of the same changed disk DOES say moved, which
    // is what makes the assertion above about staleness and not blindness.
    let reloaded = World::inspect(dir.path(), Some(&map)).unwrap().unwrap();
    let seen = rows(&reloaded);
    let consumer = seen.iter().find(|row| row.name == "client").unwrap();
    assert_eq!(consumer.consumes[0].pin.word(), "moved");
}

/// A realm that both publishes and consumes puts publication failures and
/// consumption failures on ONE report. Each consumed pin is still found
/// by its own name and publishing realm: the realm's own unreadable
/// publication is passed over, a moved pin for another crossing is passed
/// over, and a pin whose publisher could not be read is `unchecked`.
#[test]
fn a_realm_that_both_publishes_and_consumes_finds_each_pin_by_name_and_publisher() {
    let dir = tempfile::tempdir().unwrap();
    let contracts = dir.path().join("contracts");
    std::fs::create_dir_all(&contracts).unwrap();
    let beta_bytes = b"{\"title\": \"beta\"}\n";
    let sound_bytes = b"{\"title\": \"sound\"}\n";
    let gamma_bytes = b"{\"title\": \"gamma\"}\n";
    std::fs::write(contracts.join("beta.v1.json"), beta_bytes).unwrap();
    std::fs::write(contracts.join("sound.v1.json"), sound_bytes).unwrap();
    std::fs::write(contracts.join("gamma.v1.json"), gamma_bytes).unwrap();
    let sha = |bytes: &[u8]| brokkr_core::canonical::sha256_bytes(bytes);
    let map = dir.path().join("realms.json");
    std::fs::write(
        &map,
        json!({
            "schema": "forge.realms/v5",
            "realms": [
                {"name": "alpha", "path": ".", "default_branch": "main",
                 "publishes": [{"name": "alpha.doc", "path": "contracts/alpha.v1.md"}],
                 "consumes": [
                    {"name": "beta.api", "realm": "beta", "sha256": "a".repeat(64)},
                    {"name": "sound.api", "realm": "beta", "sha256": sha(sound_bytes)},
                    {"name": "gamma.api", "realm": "gamma", "sha256": "c".repeat(64)},
                    {"name": "missing.api", "realm": "delta", "sha256": "d".repeat(64)},
                 ]},
                {"name": "beta", "path": ".", "default_branch": "main",
                 "publishes": [
                    {"name": "beta.api", "path": "contracts/beta.v1.json"},
                    {"name": "sound.api", "path": "contracts/sound.v1.json"},
                 ]},
                {"name": "gamma", "path": ".", "default_branch": "main",
                 "publishes": [{"name": "gamma.api", "path": "contracts/gamma.v1.json"}]},
                {"name": "delta", "path": ".", "default_branch": "main",
                 "publishes": [{"name": "missing.api", "path": "contracts/missing.v1.json"}]},
            ],
            "journal": "j.db",
        })
        .to_string(),
    )
    .unwrap();

    let world = World::inspect(dir.path(), Some(&map)).unwrap().unwrap();
    let seen = rows(&world);
    let alpha = seen.iter().find(|row| row.name == "alpha").unwrap();
    assert_eq!(
        alpha.publishes.len(),
        1,
        "alpha still declares its own file"
    );
    let states: Vec<(&str, &str)> = alpha
        .consumes
        .iter()
        .map(|consumed| (consumed.name.as_str(), consumed.pin.word()))
        .collect();
    assert_eq!(
        states,
        vec![
            ("beta.api", "moved"),
            ("sound.api", "matching"),
            ("gamma.api", "moved"),
            ("missing.api", "unchecked"),
        ],
        "each pin is read against its own publisher, never a neighbour's"
    );
}
