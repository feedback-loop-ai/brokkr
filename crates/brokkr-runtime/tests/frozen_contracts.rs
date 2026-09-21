//! T11: the frozen v1 contracts, the corpus and the policy table are
//! read-only for this slice. A contract change is a NEW numbered file
//! beside the old one, never an edit — so the bytes of the frozen files
//! are pinned by digest here, and the two new files are asserted to
//! exist beside them.

use std::path::PathBuf;

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn digest(relative: &str) -> String {
    let bytes = std::fs::read(workspace().join(relative))
        .unwrap_or_else(|e| panic!("{relative} must exist: {e}"));
    brokkr_core::canonical::sha256_bytes(&bytes)
}

/// Recorded from this tree before the agent library existed, plus the
/// realms map v1 — pinned when decision 0026 landed `forge.realms/v2`
/// beside it, so "beside, never inside" is machine-checked.
const FROZEN: [(&str, &str); 22] = [
    // Proposed decision 0056 ruling 7 lands seat-record v5 beside v4;
    // v4's bytes are pinned here so that slice can prove it edited none
    // of them, exactly as decision 0046 pinned v3's when v4 landed.
    (
        "contracts/seat-record.v4.schema.json",
        "84d1238783db3b77a862207639fafa5c2edaf401c657dca78443e96ab9ae9222",
    ),
    (
        "contracts/realms.v1.schema.json",
        "4a9d0051823995b090935a2a5b326d12ec7953f62c61161b30ec1dbaf0135fbb",
    ),
    // Decision 0046 lands v4, v9, v4 and the boundary extension BESIDE
    // these four; their bytes are pinned here so the slice can prove it
    // edited none of them.
    (
        "contracts/realms.v3.schema.json",
        "52567711de92ccb11d9d7c44731d8abd88913ae390f5f3de6aaeb1d059b962ab",
    ),
    (
        "contracts/run-manifest.v8.schema.json",
        "45560b74755f1c0528ef06679252fb432b1a8cc0b9263e6100f9642f686b1a5d",
    ),
    (
        "contracts/seat-record.v3.schema.json",
        "10528a9efab019f90305dd4e0738f21aeb900442c77effb082139dbf30ca4c73",
    ),
    (
        "contracts/effect-provenance.v1.schema.json",
        "c57d2c997711779495ae7b951e3d07110bd3ccf40f30758cd837729abd43699c",
    ),
    (
        "contracts/event-envelope.v1.schema.json",
        "8863a07c8f5e879afe472b6a2c3060a40e0aa7a46a4132d4302a488d4e9b4f11",
    ),
    (
        "contracts/driver-protocol.v1.schema.json",
        "3435d43bfdc0731dc3895e85871a84301ce96c296dd5e1aa9fb106ca3d257c32",
    ),
    (
        "contracts/run-manifest.v1.schema.json",
        "d612c595e81b17de778a841fc2cea3e1e0769fa9f10204c515d290cfaff6cc96",
    ),
    (
        "contracts/run-manifest.v2.schema.json",
        "771fa0375cd3065c88ccb81775eb80383754bf646877f622e0480adff3ca7588",
    ),
    (
        "contracts/dispatch-envelope.v2.schema.json",
        "14b6bbe0d306f53d29026e4002d14d5b5fae40a12915d67e186a4c3d08f4475b",
    ),
    (
        "fixtures/evaluator/corpus.ndjson",
        "19ed1b05ca04ac0fd3c511b6c6c1c7412a2400b763c27658d964080692cf9964",
    ),
    (
        "contracts/realms.v2.schema.json",
        "23a5da79f07e9f4569350e7f92ba94d514bf088bce116d9f3afc712f0a04c13c",
    ),
    (
        "contracts/run-manifest.v3.schema.json",
        "79e7e87a8d79ae3c6da3f915d44d1940e842a9a150f938886de8917330b65172",
    ),
    (
        "contracts/run-manifest.v4.schema.json",
        "fbb5b01fd79028ad15b2039b5b69b967ad7676d276af7a360d089fd582c2fd5c",
    ),
    (
        "contracts/run-manifest.v5.schema.json",
        "73a6bfa378b2e44c60608d6791e3c21faff96b72b096594c235a588fa3eff4a0",
    ),
    (
        "contracts/run-manifest.v6.schema.json",
        "7f9b5940c334e5596cca724e41a52e26f08fcde478cbefdba244070326bfe3a1",
    ),
    (
        "contracts/run-manifest.v7.schema.json",
        "96e823572a6d0bee51f1b640eb73da7d3c2e9eafaa225f18546a696d26951e0c",
    ),
    (
        "contracts/seat-record.v1.schema.json",
        "91c51d5bea1c5fbc11bab7bbf57b53e6257a85c9424652f9e9162705840e1483",
    ),
    (
        "contracts/seat-record.v2.schema.json",
        "a35c237e1e351a03fb974e9a13a7fc33b9d1a570413626d70367e97a3f501bce",
    ),
    // Decision 0057 lands `forge.realms/v5` beside v4, which was the new
    // file when 0046 landed and is frozen from here: its bytes are pinned
    // so the crossings slice can prove it edited none of them.
    (
        "contracts/realms.v4.schema.json",
        "7f03c61886e91189ae46388eead49a11e52fe70f17fde8d27cf0a33fc08b9ad5",
    ),
    // Decision 0057's recording half lands `run-manifest.v10` beside v9,
    // which was the new file when 0046 landed and is frozen from here.
    (
        "contracts/run-manifest.v9.schema.json",
        "c046add105f94efccba9a4d688016eadc8f2933bdbfe70a9f625991e46878a71",
    ),
];

#[test]
fn the_frozen_contracts_and_the_corpus_keep_their_exact_bytes() {
    for (relative, pinned) in FROZEN {
        assert_eq!(digest(relative), pinned, "{relative} bytes moved");
    }
    // The production table is read-only too, and it is not a contract
    // file, so it is pinned separately by the same rule.
    assert_eq!(
        digest("policy/phase-machine.json"),
        "e0b3e9338745dd07685ef8a2182345f9f78df2b4e80203a498779c305e1e90ee",
    );
}

/// The new contracts land BESIDE the frozen ones, as new numbered
/// files — the only way a frozen contract ever changes.
#[test]
fn the_new_contracts_exist_beside_the_frozen_ones() {
    for (relative, title) in [
        (
            "contracts/run-manifest.v3.schema.json",
            "Forge run manifest v3",
        ),
        (
            "contracts/effect-provenance.v1.schema.json",
            "Forge effect provenance v1",
        ),
        // Decision 0022's rule-driven park: a new version beside v1, not
        // an edit to it. The v1 table schema was never a file here, so
        // this pins the ONE thing that matters — v2 landed as its own
        // published file rather than as bytes changed under v1's name.
        (
            "contracts/phase-machine.v2.schema.json",
            "Forge phase-machine table v2",
        ),
        // Decision 0023's world: the map's own schema, and the manifest
        // version that pins one. Both land beside the frozen files —
        // v1's and v3's bytes are asserted above and did not move.
        ("contracts/realms.v1.schema.json", "Forge realms map v1"),
        (
            "contracts/run-manifest.v4.schema.json",
            "Forge run manifest v4",
        ),
        // Decision 0021's witness (remedy ii of the reforged run): the
        // authorising adapters pinned as a new version beside v4, whose
        // bytes are asserted above and did not move.
        (
            "contracts/run-manifest.v5.schema.json",
            "Forge run manifest v5",
        ),
        // Decision 0026's many hearths: the per-realm journal arrives as
        // `forge.realms/v2`, a new file beside v1 — whose bytes are
        // pinned below and did not move.
        ("contracts/realms.v2.schema.json", "Forge realms map v2"),
        ("contracts/realms.v3.schema.json", "Forge realms map v3"),
        (
            "contracts/dialect.v1.schema.json",
            "Brokkr specification dialect v1",
        ),
        // Decision 0042's addendum of 2026-09-04: the tool block gains the
        // install identity, as a new file beside v1 whose bytes are pinned
        // above and did not move.
        (
            "contracts/dialect.v2.schema.json",
            "Brokkr specification dialect v2",
        ),
        // Decision 0042's addendum of 2026-09-06: the archive step appends
        // the change that wrote each capability, so the dialect's archive
        // operation carries the instruction that says how. v3 lands beside
        // v1 and v2, whose bytes are pinned above and did not move.
        (
            "contracts/dialect.v3.schema.json",
            "Brokkr specification dialect v3",
        ),
        // Decision 0034 freezes the previously conventional accounting
        // record as its own v1 contract; no older frozen file moves.
        (
            "contracts/seat-record.v1.schema.json",
            "Forge seat record v1",
        ),
        // Decision 0035 ruling 7: the hire's effort and the reasoning it
        // spent arrive as a NEW file beside v1, never as a field added to
        // it. v1's own bytes are pinned by the embedded-copy test in
        // `brokkr-store`, and this pins that v2 landed as its own
        // published contract rather than as bytes changed under v1's name.
        (
            "contracts/seat-record.v2.schema.json",
            "Forge seat record v2",
        ),
        // Decision 0034's second addendum (ruled 2026-09-05): the dialect
        // step's `state` is admitted to the typed report as a NEW file
        // beside v2, whose bytes are pinned above and did not move.
        (
            "contracts/seat-record.v3.schema.json",
            "Forge seat record v3",
        ),
        // Decision 0043's boxed hands: the manifest's `hands` key arrives
        // as v6 beside v5, whose bytes are pinned above and did not move.
        (
            "contracts/run-manifest.v6.schema.json",
            "Forge run manifest v6",
        ),
        (
            "contracts/run-manifest.v7.schema.json",
            "Forge run manifest v7",
        ),
        (
            "contracts/run-manifest.v8.schema.json",
            "Forge run manifest v8",
        ),
        (
            "contracts/phase-entered-case.v1.schema.json",
            "Forge phase-entered selected case v1",
        ),
        // Decision 0046 ruling 1: the boundary is named by the realm map
        // as v4 beside v3, and pinned per hands site by the manifest as
        // v9 beside v8 — the frozen predecessors' bytes are pinned below
        // and did not move.
        ("contracts/realms.v4.schema.json", "Forge realms map v4"),
        (
            "contracts/run-manifest.v9.schema.json",
            "Forge run manifest v9",
        ),
        // Decision 0046 ruling 3, with the commission's erratum: the seat
        // record carries the boundary as v4 beside v3 (v3 already
        // carried the dialect state under decision 0034 rulings 6 and 7),
        // and `effect/started` carries it as a numbered extension schema
        // beside the frozen `effect-provenance.v1`.
        (
            "contracts/seat-record.v4.schema.json",
            "Forge seat record v4",
        ),
        (
            "contracts/effect-boundary.v1.schema.json",
            "Forge effect boundary v1",
        ),
        // Decision 0047 ruling 1: the supersede annotation's `args`
        // arrive as their own published payload schema. Nothing above
        // moves — the envelope's `command` was already an open string
        // and `args` an already-legal open object.
        (
            "contracts/operator-supersede.v1.schema.json",
            "Forge operator supersede args v1",
        ),
        // Decision 0057: the crossing's vocabulary arrives as
        // `forge.realms/v5` beside v4, whose bytes are pinned above and
        // did not move.
        ("contracts/realms.v5.schema.json", "Forge realms map v5"),
        // Decision 0057's recording half: the crossings a run stood on
        // arrive as `run-manifest.v10` beside v9, whose bytes are pinned
        // above and did not move.
        (
            "contracts/run-manifest.v10.schema.json",
            "Forge run manifest v10",
        ),
        // Proposed decision 0056 ruling 7: the confirmed root, the two
        // engine stamps and the five added refusal tokens arrive as v5
        // beside v4, whose bytes are pinned above and did not move.
        (
            "contracts/seat-record.v5.schema.json",
            "Forge seat record v5",
        ),
        // Decision 0065, slice one: the grant arrives as `forge.realms/v6`
        // beside v5, the capability authority a bundle compiled under as
        // `run-manifest.v11` beside v10, and the tool dialect as its own
        // first contract. No older file moves: v5's and v10's bytes are
        // pinned in `the_capability_contracts_land_beside_their_frozen_
        // predecessors` below.
        ("contracts/realms.v6.schema.json", "Forge realms map v6"),
        (
            "contracts/run-manifest.v11.schema.json",
            "Forge run manifest v11",
        ),
        (
            "contracts/tool-dialect.v1.schema.json",
            "Brokkr tool dialect v1",
        ),
    ] {
        let body: serde_json::Value =
            serde_json::from_slice(&std::fs::read(workspace().join(relative)).unwrap()).unwrap();
        assert!(
            body["title"].as_str().unwrap().starts_with(title),
            "{relative} is the published extension schema"
        );
    }
}

#[test]
fn the_realms_v3_contract_refuses_windows_drive_relative_text_paths() {
    let schema: serde_json::Value = serde_json::from_slice(
        &std::fs::read(workspace().join("contracts/realms.v3.schema.json")).unwrap(),
    )
    .unwrap();
    let validator = jsonschema::draft7::new(&schema).unwrap();
    for field in ["house", "dialect"] {
        let mut map = serde_json::json!({
            "schema": "forge.realms/v3",
            "realms": [{"name": "app", "path": ".", "default_branch": "main"}],
            "journal": "forge.db"
        });
        map["realms"][0][field] = serde_json::json!("C:outside.md");
        assert!(
            !validator.is_valid(&map),
            "the v3 schema admitted a drive-relative {field}"
        );
    }
}

#[test]
fn the_v4_realm_schema_accepts_only_its_version_and_five_boundaries() {
    use serde_json::json;
    let schema = serde_json::from_slice::<serde_json::Value>(
        &std::fs::read(workspace().join("contracts/realms.v4.schema.json")).unwrap(),
    )
    .unwrap();
    let validator = jsonschema::draft7::new(&schema).unwrap();
    let mut map = json!({"schema":"forge.realms/v4", "realms":[{"name":"app", "path":".", "default_branch":"main"}], "journal":"forge.db"});
    assert!(validator.is_valid(&map));
    for word in brokkr_core::realms::BOUNDARIES {
        map["realms"][0]["boundary"] = json!(word.word());
        assert!(validator.is_valid(&map));
    }
    for invalid in [
        json!("chroot"),
        json!("not applicable"),
        json!(null),
        json!(7),
    ] {
        map["realms"][0]["boundary"] = invalid;
        assert!(!validator.is_valid(&map));
    }
    map["realms"][0]["boundary"] = json!("harness");
    for version in [
        "forge.realms/v1",
        "forge.realms/v2",
        "forge.realms/v3",
        "forge.realms/v5",
    ] {
        map["schema"] = json!(version);
        assert!(!validator.is_valid(&map));
    }
}

/// Decision 0057's recording half: the published contract for what a run
/// stood on. v10 is v9's vocabulary plus one optional `crossings`
/// property — closed inside, absent for every world that draws no
/// crossing — and a v9 manifest is a v10 manifest, field for field.
#[test]
fn the_v10_manifest_schema_carries_the_crossings_and_closes_their_entries() {
    use serde_json::json;
    let read = |name: &str| {
        serde_json::from_slice::<serde_json::Value>(&std::fs::read(workspace().join(name)).unwrap())
            .unwrap()
    };
    let schema = read("contracts/run-manifest.v10.schema.json");
    let v9 = read("contracts/run-manifest.v9.schema.json");
    // v9 plus one property and nothing else: every other clause — the
    // required keys, the `hands`/`boundary` dependency, the whole
    // `select` machinery — is carried over definition for definition.
    for (name, definition) in v9["properties"].as_object().unwrap() {
        assert_eq!(
            &schema["properties"][name], definition,
            "{name} moved between v9 and v10"
        );
    }
    let added: Vec<&String> = schema["properties"]
        .as_object()
        .unwrap()
        .keys()
        .filter(|key| v9["properties"].get(key).is_none())
        .collect();
    assert_eq!(added, ["crossings"]);
    for key in ["required", "dependencies", "definitions", "type"] {
        assert_eq!(schema[key], v9[key], "{key} moved between v9 and v10");
    }

    let validator = jsonschema::draft7::new(&schema).unwrap();
    let pin = brokkr_core::canonical::sha256_bytes(b"{\"title\": \"orders\"}\n");
    let mut manifest = json!({
        "engine": "0.9.1", "event_schema": 1, "database_schema": 1,
        "driver_protocol": 1, "bundle_name": "self",
        "files": {"policy.json": "a".repeat(64)},
    });
    // A world that drew no crossing is exactly a v9 manifest, and the
    // v9 file still validates it — the shape did not move under it.
    assert!(validator.is_valid(&manifest));
    assert!(jsonschema::draft7::new(&v9).unwrap().is_valid(&manifest));
    manifest["crossings"] = json!({"alpha": {"orders.api": {
        "source": "/w/alpha/contracts/orders.v1.schema.json", "sha256": pin,
    }}});
    assert!(validator.is_valid(&manifest));
    // And the entry is closed at every level, as `realms.v5`'s own are:
    // a media type, a declared-versus-observed marker or a compatibility
    // relation would have to arrive as `run-manifest.v11`.
    for entry in [
        json!({"source": "orders.json"}),
        json!({"sha256": pin}),
        json!({"source": "orders.json", "sha256": "not a digest"}),
        json!({"source": "orders.json", "sha256": pin.to_uppercase()}),
        json!({"source": "orders.json", "sha256": pin, "declared": pin}),
        json!("orders.json"),
    ] {
        manifest["crossings"]["alpha"]["orders.api"] = entry.clone();
        assert!(
            !validator.is_valid(&manifest),
            "the v10 schema admitted {entry}"
        );
    }
}

/// Decision 0057: the published contract for the crossing. v5 is v4's
/// vocabulary plus the two optional lists — a realm may publish named
/// repository-relative files and pin another realm's by digest — and it
/// closes both new entries the way every level before them is closed.
#[test]
fn the_v5_realm_schema_carries_the_crossings_and_closes_their_entries() {
    use serde_json::json;
    let schema = serde_json::from_slice::<serde_json::Value>(
        &std::fs::read(workspace().join("contracts/realms.v5.schema.json")).unwrap(),
    )
    .unwrap();
    let validator = jsonschema::draft7::new(&schema).unwrap();
    // v5 is v4 plus two properties and nothing else: every realm
    // property v4 defines is carried over unchanged, definition for
    // definition, the way v4 carried v3's house and dialect.
    let v4 = serde_json::from_slice::<serde_json::Value>(
        &std::fs::read(workspace().join("contracts/realms.v4.schema.json")).unwrap(),
    )
    .unwrap();
    let properties =
        |map: &serde_json::Value| map["properties"]["realms"]["items"]["properties"].clone();
    let carried = properties(&schema);
    for (name, definition) in properties(&v4).as_object().unwrap() {
        assert_eq!(&carried[name], definition, "{name} moved between v4 and v5");
    }
    let added: Vec<&String> = carried
        .as_object()
        .unwrap()
        .keys()
        .filter(|key| properties(&v4).get(key).is_none())
        .collect();
    assert_eq!(added, ["consumes", "publishes"]);
    let pin = brokkr_core::canonical::sha256_bytes(b"# orders\n");
    let world = |alpha: serde_json::Value, beta: serde_json::Value| {
        let mut alpha_realm = json!({"name":"alpha", "path":"alpha", "default_branch":"main"});
        alpha_realm["publishes"] = alpha;
        let mut beta_realm = json!({"name":"beta", "path":"beta", "default_branch":"main"});
        beta_realm["consumes"] = beta;
        json!({"schema":"forge.realms/v5", "realms":[alpha_realm, beta_realm], "journal":"forge.db"})
    };
    let crossed = world(
        json!([{"name": "orders.api", "path": "contracts/orders.v1.schema.json"}]),
        json!([{"name": "orders.api", "realm": "alpha", "sha256": pin}]),
    );
    assert!(validator.is_valid(&crossed));
    // v4's whole vocabulary still reads, and a v5 map that draws no
    // crossing is a v4 map with a later label.
    let mut plain = json!({"schema":"forge.realms/v5", "realms":[{"name":"app", "path":".", "default_branch":"main", "journal":"app.db", "house":"HOUSE.md", "dialect":"openspec", "boundary":"harness"}], "journal":"forge.db"});
    assert!(validator.is_valid(&plain));
    plain["realms"][0]["house"] = json!("C:outside.md");
    assert!(!validator.is_valid(&plain), "a drive-relative house");

    for published in [
        json!([{"name": "Orders", "path": "orders.json"}]),
        json!([{"name": "orders.api", "path": "/orders.json"}]),
        json!([{"name": "orders.api", "path": "../orders.json"}]),
        json!([{"name": "orders.api"}]),
        json!([{"name": "orders.api", "path": "orders.json", "media_type": "json"}]),
        json!([{"path": "orders.json"}]),
        json!("orders.json"),
    ] {
        assert!(
            !validator.is_valid(&world(
                published.clone(),
                json!([{"name": "orders.api", "realm": "alpha", "sha256": pin}])
            )),
            "the v5 schema admitted {published}"
        );
    }
    for consumed in [
        json!([{"name": "orders.api", "realm": "alpha", "sha256": "not a digest"}]),
        json!([{"name": "orders.api", "realm": "alpha", "sha256": pin.to_uppercase()}]),
        json!([{"name": "orders.api", "realm": "alpha", "sha256": format!("{pin}0")}]),
        json!([{"name": "orders.api", "realm": "Alpha", "sha256": pin}]),
        json!([{"name": "orders.api", "realm": "alpha"}]),
        json!([{"name": "orders.api", "realm": "alpha", "sha256": pin, "since": "v1"}]),
    ] {
        assert!(
            !validator.is_valid(&world(
                json!([{"name": "orders.api", "path": "orders.json"}]),
                consumed.clone()
            )),
            "the v5 schema admitted {consumed}"
        );
    }
    // And the label is the version's own: an older map's crossings are
    // refused here exactly as v4's boundary is refused by v3's file.
    for version in [
        "forge.realms/v1",
        "forge.realms/v2",
        "forge.realms/v3",
        "forge.realms/v4",
        "forge.realms/v6",
    ] {
        let mut older = crossed.clone();
        older["schema"] = json!(version);
        assert!(!validator.is_valid(&older), "{version} under the v5 schema");
    }
}

fn contract(name: &str) -> jsonschema::Validator {
    let schema: serde_json::Value =
        serde_json::from_slice(&std::fs::read(workspace().join(name)).unwrap()).unwrap();
    jsonschema::draft7::new(&schema).unwrap()
}

/// Decision 0065 lands three files and edits none: the two predecessors
/// its new versions stand beside keep their exact bytes.
#[test]
fn the_capability_contracts_land_beside_their_frozen_predecessors() {
    for (relative, pinned) in [
        (
            "contracts/realms.v5.schema.json",
            "e0203be78cc2896ce6983d8c8a26417544ccbef311c1b3c63425bfc55a96c1e5",
        ),
        (
            "contracts/run-manifest.v10.schema.json",
            "1093c99ce1ff7bb33c5994f2fa4bba6315f29b911fa9e2b68216f600e2927962",
        ),
    ] {
        assert_eq!(digest(relative), pinned, "{relative} bytes moved");
    }
}

/// `forge.realms/v6` is v5 plus one optional per-realm `capabilities` map
/// (decision 0065 ruling 3). Omission and `{}` both grant nothing; the
/// optional lists keep absent apart from empty; `null`, a wrong type and a
/// repeated entry are refused; and every older version's own contract
/// refuses the word even written empty.
#[test]
fn the_v6_realm_schema_admits_the_grant_and_no_older_version_does() {
    use serde_json::json;
    let v6 = contract("contracts/realms.v6.schema.json");
    let map = |capabilities: Option<serde_json::Value>| {
        let mut realm = json!({"name": "private", "path": "repo", "default_branch": "main",
                               "boundary": "harness"});
        if let Some(capabilities) = capabilities {
            realm["capabilities"] = capabilities;
        }
        json!({"schema": "forge.realms/v6", "realms": [realm], "journal": "forge.db"})
    };
    for valid in [
        None,
        Some(json!({})),
        Some(json!({"web-search": {"dialect": "codex-native-search"}})),
        Some(
            json!({"web-fetch": {"dialect": "fetch-mcp", "tools": ["fetch"],
                    "allow": {"hosts": ["sourceware.org", "yaml.org"]}},
                    "web-search": {"dialect": "codex-native-search",
                                   "offices": ["review-security", "review-adversarial"]},
                    "library-docs": {"dialect": "context7", "tools": [], "offices": []}}),
        ),
    ] {
        assert!(v6.is_valid(&map(valid.clone())), "{valid:?}");
    }
    for invalid in [
        json!(null),
        json!([]),
        json!({"Web Search": {"dialect": "d"}}),
        json!({"web-search": "codex-native-search"}),
        json!({"web-search": {"tools": ["web_search"]}}),
        json!({"web-search": {"dialect": "../escape"}}),
        json!({"web-search": {"dialect": "d", "tools": null}}),
        json!({"web-search": {"dialect": "d", "tools": ["a", "a"]}}),
        json!({"web-search": {"dialect": "d", "offices": [7]}}),
        json!({"web-search": {"dialect": "d", "offices": [""]}}),
    ] {
        assert!(
            !v6.is_valid(&map(Some(invalid.clone()))),
            "the v6 schema admitted {invalid}"
        );
    }
    // The label is the version's own, in both directions.
    for (version, file) in [
        ("forge.realms/v1", "contracts/realms.v1.schema.json"),
        ("forge.realms/v2", "contracts/realms.v2.schema.json"),
        ("forge.realms/v3", "contracts/realms.v3.schema.json"),
        ("forge.realms/v4", "contracts/realms.v4.schema.json"),
        ("forge.realms/v5", "contracts/realms.v5.schema.json"),
    ] {
        let mut older = json!({"schema": version, "journal": "forge.db", "realms": [
            {"name": "private", "path": "repo", "default_branch": "main"}]});
        let schema = contract(file);
        assert!(
            schema.is_valid(&older),
            "{version} still loads and grants nothing"
        );
        assert!(!v6.is_valid(&older), "{version} under the v6 schema");
        older["realms"][0]["capabilities"] = json!({});
        assert!(
            !schema.is_valid(&older),
            "{version} admitted capabilities, even empty"
        );
    }
}

/// `brokkr.tool-dialect/v1` is a closed discriminated shape: exactly one
/// kind, that kind's binding and no other's. Both MCP connection forms are
/// whole as data; a mixture, a missing pin, a credential by value and a
/// word from the wrong vocabulary are refused.
#[test]
fn the_tool_dialect_schema_binds_one_capability_to_exactly_one_kind() {
    use serde_json::json;
    let v1 = contract("contracts/tool-dialect.v1.schema.json");
    let sends = json!({"description": "a query the model composes", "seat_composed": true});
    let native = json!({
        "schema": "brokkr.tool-dialect/v1", "name": "codex-native-search",
        "serves": "web-search", "kind": "provider-native", "provider": "codex",
        "adapter_key": "web-search", "tools": ["web_search"], "sends": sends,
        "classes": ["reads", "egress"], "egress": "uncontracted",
        "restrictions": {"type": "object", "additionalProperties": false}
    });
    let mcp = |connection: serde_json::Value| {
        json!({
            "schema": "brokkr.tool-dialect/v1", "name": "docs-mcp", "serves": "library-docs",
            "kind": "mcp", "connection": connection, "version": "1.4.2",
            "secrets": ["DOCS_TOKEN"], "tools": ["resolve"], "retained": true, "sends": sends
        })
    };
    let hands = json!({
        "schema": "brokkr.tool-dialect/v1", "name": "hands", "serves": "workspace",
        "kind": "hands", "tools": ["workspace"], "egress": "local",
        "sends": {"description": "nothing leaves the box", "seat_composed": false}
    });
    for valid in [
        native.clone(),
        mcp(json!({"argv": ["docs-mcp", "--stdio"]})),
        mcp(json!({"url": "https://docs.example.org/mcp"})),
        hands.clone(),
    ] {
        assert!(v1.is_valid(&valid), "{valid}");
    }
    // Every shipped dialect is inside the contract it is published under.
    for entry in std::fs::read_dir(workspace().join("dialects/tools")).unwrap() {
        let path = entry.unwrap().path();
        let shipped: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert!(
            v1.is_valid(&shipped),
            "{} is outside tool-dialect/v1",
            path.display()
        );
        assert_eq!(shipped["kind"], "provider-native", "{}", path.display());
    }
    let edit = |base: &serde_json::Value, mutate: &dyn Fn(&mut serde_json::Value)| {
        let mut value = base.clone();
        mutate(&mut value);
        value
    };
    let stdio = mcp(json!({"argv": ["docs-mcp"]}));
    for invalid in [
        edit(&native, &|d| d["kind"] = json!("plugin")),
        edit(&native, &|d| {
            d.as_object_mut().unwrap().remove("serves");
        }),
        edit(&native, &|d| {
            d.as_object_mut().unwrap().remove("adapter_key");
        }),
        edit(&native, &|d| {
            d.as_object_mut().unwrap().remove("sends");
        }),
        edit(&native, &|d| d["tools"] = json!([])),
        edit(&native, &|d| {
            d["connection"] = json!({"url": "https://docs.example.org"})
        }),
        edit(&native, &|d| d["egress"] = json!("reads")),
        edit(&native, &|d| d["classes"] = json!(["contracted"])),
        edit(&native, &|d| d["classes"] = json!([])),
        edit(&stdio, &|d| d["provider"] = json!("codex")),
        edit(&stdio, &|d| {
            d["connection"] = json!({"argv": ["x"], "url": "https://docs.example.org"})
        }),
        edit(&stdio, &|d| d["connection"] = json!({})),
        edit(&stdio, &|d| {
            d["connection"] = json!({"url": "https://user:secret@docs.example.org"})
        }),
        edit(&stdio, &|d| {
            d.as_object_mut().unwrap().remove("version");
        }),
        edit(&stdio, &|d| {
            d.as_object_mut().unwrap().remove("secrets");
        }),
        edit(&stdio, &|d| d["secrets"] = json!(["a literal value"])),
        edit(&stdio, &|d| d["token"] = json!("a literal value")),
        edit(&hands, &|d| d["serves"] = json!("web-search")),
        edit(&hands, &|d| d["tools"] = json!(["workspace", "shell"])),
        edit(&hands, &|d| d["provider"] = json!("claude")),
    ] {
        assert!(
            !v1.is_valid(&invalid),
            "the tool-dialect schema admitted {invalid}"
        );
    }
}

/// `run-manifest/v11` is v10 plus the REQUIRED `capabilities` section
/// (decision 0065 ruling 8): an explicit empty authority validates, a
/// manifest without the section does not, and the records are closed.
#[test]
fn the_v11_manifest_schema_requires_the_capability_section_and_closes_its_records() {
    use serde_json::json;
    let v11 = contract("contracts/run-manifest.v11.schema.json");
    let sha = "a".repeat(64);
    let manifest = |capabilities: Option<serde_json::Value>| {
        let mut manifest = json!({"engine": "0.10.0", "event_schema": 1, "database_schema": 1,
            "driver_protocol": 1, "bundle_name": "fast", "files": {"bundle.json": sha}});
        if let Some(capabilities) = capabilities {
            manifest["capabilities"] = capabilities;
        }
        manifest
    };
    let empty = json!({"realm": "<unmapped>", "grants": {}, "definitions": {}, "dialects": {},
                       "sites": {}});
    let held = json!({
        "realm": "private",
        "grants": {"web-search": {"dialect": "codex-native-search", "offices": ["researcher"],
                                  "allow": {"hosts": ["yaml.org"]}}},
        "definitions": {"web-search": {"source": "capabilities/web-search.json", "sha256": sha,
                                       "classes": ["reads", "egress"]}},
        "dialects": {"codex-native-search": {"source": "dialects/tools/codex-native-search.json",
            "sha256": sha, "kind": "provider-native", "serves": "web-search"}},
        "sites": {"research": {"office": "researcher", "asks": {"web-search": "wants"},
            "subtracted": ["web-fetch"], "candidates": [
                {"provider": "codex", "model": "astra",
                 "held": {"web-search": {"classes": ["reads", "egress"],
                     "dialect": "codex-native-search", "dialect_sha256": sha,
                     "definition_sha256": sha, "tools": ["web_search"],
                     "restrictions": {"allow": {"hosts": ["yaml.org"]}}}},
                 "not_held": {"web-fetch": "this seat subtracted it from its office's asks"},
                 "notices": [],
                 "native": {"inventory": "known", "declaration": sha,
                            "on": ["web-search"], "off": []}},
                {"provider": "dsh", "held": {}, "not_held": {}, "notices": ["a dropped want"],
                 "native": {"inventory": "unmeasured", "reason": "never probed"}}]}}
    });
    assert!(v11.is_valid(&manifest(Some(empty.clone()))));
    assert!(v11.is_valid(&manifest(Some(held.clone()))));
    assert!(!v11.is_valid(&manifest(None)), "the section is required");
    // v10 cannot admit the section, and a v10 manifest is not a v11 one.
    let v10 = contract("contracts/run-manifest.v10.schema.json");
    assert!(v10.is_valid(&manifest(None)));
    assert!(!v10.is_valid(&manifest(Some(empty.clone()))));
    let edit = |mutate: &dyn Fn(&mut serde_json::Value)| {
        let mut value = held.clone();
        mutate(&mut value);
        manifest(Some(value))
    };
    for invalid in [
        edit(&|c| {
            c.as_object_mut().unwrap().remove("sites");
        }),
        edit(&|c| c["holdings"] = json!({})),
        edit(&|c| c["definitions"]["web-search"]["source"] = json!("/etc/web-search.json")),
        edit(&|c| c["definitions"]["web-search"]["source"] = json!("../web-search.json")),
        edit(&|c| c["definitions"]["web-search"]["classes"] = json!(["network"])),
        edit(&|c| c["dialects"]["codex-native-search"]["sha256"] = json!("short")),
        edit(&|c| c["dialects"]["codex-native-search"]["argv"] = json!(["codex"])),
        edit(&|c| c["sites"]["research"]["asks"]["web-search"] = json!("optional")),
        edit(&|c| {
            c["sites"]["research"]["candidates"][0]["held"]["web-search"]["tools"] = json!([])
        }),
        edit(&|c| c["sites"]["research"]["candidates"][0]["secret"] = json!("value")),
        edit(&|c| {
            c["sites"]["research"]["candidates"][0]["native"] = json!({"inventory": "known"})
        }),
        edit(&|c| {
            c["sites"]["research"]["candidates"][1]["native"] = json!({"inventory": "unmeasured"})
        }),
    ] {
        assert!(!v11.is_valid(&invalid), "the v11 schema admitted {invalid}");
    }
}
