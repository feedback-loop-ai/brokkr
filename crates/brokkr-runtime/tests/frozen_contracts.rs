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
const FROZEN: [(&str, &str); 20] = [
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
